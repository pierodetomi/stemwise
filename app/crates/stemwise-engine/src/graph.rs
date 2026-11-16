//! The compiled mix graph (FOUNDATION §5.4) and its `process` function, shared by real-time playback and the
//! offline render (ENG-4).
//!
//! - Built off the audio thread from a project snapshot: nodes in processing order (core `routing`), every
//!   buffer and delay line preallocated. `process` never allocates (NFR-2).
//! - Time is split on a grid of [`SUBBLOCK`] frames anchored at timeline frame 0 (A-020). At each grid point all
//!   parameters are evaluated (automation or smoothed static value) for the cell start and end; gains ramp per
//!   sample across the cell, filters get new coefficients per cell (AUT-4, AUT-6). Callbacks are cut at grid
//!   points, so the output does not depend on the block size.
//! - Latency compensation (ENG-5): the saturation's oversampling delays every track and group chain by
//!   [`Saturation::LATENCY`]; each edge (output, send) gets a delay line so all inputs of a sum arrive aligned.
//!   [`Graph::latency`] is the total, removed by the offline render and by the playhead.
//! - Nodes are stored by dependency level; each node processes the whole block (cell by cell). Nodes of one level
//!   are independent: the offline render runs them on several threads (NFR-4). Their contributions are then added
//!   to the destinations serially, in a fixed order, so the result is the same with or without threads.

use std::collections::HashMap;

use rayon::prelude::*;

use stemwise_core::automation::value_at;
use stemwise_core::model::{AutomationPoint, Output, Project, ReturnFx, Track, TrackKind};
use stemwise_core::params::{self, FX, ParamId};
use stemwise_core::routing;
use stemwise_dsp::delay::{Delay, DelayParams};
use stemwise_dsp::loudness::LoudnessMeter;
use stemwise_dsp::meters::{Correlation, PeakRms};
use stemwise_dsp::pan::{balance_gains, mono_gains};
use stemwise_dsp::reverb::{Algorithm, Reverb, ReverbParams};
use stemwise_dsp::truepeak::TruePeakStream;
use stemwise_dsp::{SUBBLOCK, db_to_gain};

use crate::chain::{Blk, Fade, Line, keys_of};

const N: usize = SUBBLOCK;
/// Largest block processed in one pass; longer calls are split.
pub const MAX_BLOCK: usize = 4096;

/// Fixed parameter slots of a node; the blocks' parameters follow.
const TRIM: usize = 0;
const FADER: usize = 1;
const PAN: usize = 2;
const MUTE: usize = 3;
/// Bypass of a return's processor (`bypass.fx`).
const FXBY: usize = 4;
/// The return processor's parameters: reverb decay, size, damping, modulation, pre-delay, low cut, high cut,
/// shimmer; delay time, feedback, feedback low cut, feedback high cut, wow, drive.
const RET: usize = 5;
const RET_N: usize = 8;
const FIXED: usize = RET + RET_N;

fn ret_params(fx: &Option<ReturnFx>) -> Vec<ParamId> {
    use ParamId::*;
    match fx {
        Some(ReturnFx::Reverb { .. }) => {
            vec![RevDecay, RevSize, RevDamping, RevModulation, RevPredelay, RevHpf, RevLpf, RevShimmer]
        }
        Some(ReturnFx::Delay { .. }) => vec![DlyTime, DlyFeedback, DlyFbHpf, DlyFbLpf, DlyWow, DlyDrive],
        None => Vec::new(),
    }
}

/// A parameter: static target (smoothed) or automation lane; value at cell start `a` and end `b`.
#[derive(Clone)]
struct Param {
    /// Frames after the node's input latency at which this parameter is evaluated (the latency of the blocks
    /// before it; the node's chain latency for fader, pan, mute and sends).
    rel: i64,
    target: f64,
    cur: f64,
    lane: Option<Vec<AutomationPoint>>,
    stepped: bool,
    smooth: bool,
    a: f64,
    b: f64,
}

impl Param {
    fn new(v: f64, stepped: bool, smooth: bool) -> Self {
        Param { rel: 0, target: v, cur: v, lane: None, stepped, smooth, a: v, b: v }
    }

    fn eval(&mut self, cell: i64, k: f64) {
        match &self.lane {
            Some(l) => {
                self.a = value_at(l, cell, self.stepped).unwrap_or(self.target);
                self.b = value_at(l, cell + N as i64, self.stepped).unwrap_or(self.target);
                if self.stepped {
                    // Steps change at the cell start and ramp over the cell (no click).
                    self.b = value_at(l, cell, true).unwrap_or(self.target);
                    self.a = self.cur;
                }
                self.cur = self.b;
            }
            None => {
                self.a = self.cur;
                if self.smooth && !self.stepped {
                    self.cur += (self.target - self.cur) * k;
                    if (self.target - self.cur).abs() < 1e-9 {
                        self.cur = self.target;
                    }
                } else {
                    self.cur = self.target;
                }
                self.b = self.cur;
            }
        }
    }

    /// Cycles an oscillator at this parameter's frequency (Hz) has run from frame 0 to the cell start `at`, as a
    /// processor that reads the value once per cell accumulates them: the value of each cell held over its frames.
    /// Closed form per lane segment (the value is linear between two points), so a seek costs the number of points.
    fn cycles(&self, at: i64, rate: f64) -> f64 {
        let n = N as i64;
        let Some(l) = self.lane.as_ref().filter(|l| !l.is_empty() && at > 0) else {
            let v = self.lane.as_ref().and_then(|l| value_at(l, at, false)).unwrap_or(self.target);
            return v * at as f64 / rate;
        };
        let val = |g: i64| value_at(l, g, false).unwrap_or(self.target);
        // The cells start at the frames congruent to `at`; the one that contains frame 0 counts for its part.
        let lead = at.rem_euclid(n);
        let mut sum = if lead > 0 { val(lead - n) * lead as f64 } else { 0.0 };
        let mut lo = lead;
        for hi in l.iter().map(|p| p.frame).chain([i64::MAX]) {
            let hi = hi.min(at);
            if hi > lo {
                // Cell starts in [lo, hi): lo is one of them.
                let count = (hi - lo + n - 1) / n;
                let last = lo + (count - 1) * n;
                sum += (val(lo) + val(last)) * 0.5 * (count * n) as f64;
                lo = last + n;
            }
            if lo >= at {
                break;
            }
        }
        sum / rate
    }

    fn snap(&mut self, cell: i64) {
        let v = match &self.lane {
            Some(l) => value_at(l, cell, self.stepped).unwrap_or(self.target),
            None => self.target,
        };
        self.cur = v;
        self.a = v;
        self.b = v;
    }
}

struct Edge {
    to: usize,
    delay: Option<Line>,
    /// Send slot (index into `Node::sends`), or `None` for the main output.
    send: Option<usize>,
    /// This block's contribution to the destination.
    out: [Vec<f32>; 2],
    /// A send to a soloed return keeps feeding it even when the source is silenced by solo (TRK-2).
    solo_pass: bool,
}

enum Fx {
    None,
    Reverb(Box<Reverb>),
    Delay(Box<Delay>),
}

/// A sidechain key of a node: the post-fader output of `src`, delayed to meet the audio at the block's input.
struct KeyIn {
    src: usize,
    delay: Option<Line>,
    buf: [Vec<f32>; 2],
}

struct Node {
    id: String,
    /// Timeline offset of the audio entering the node (its input latency). Parameters are evaluated at the time
    /// of the audio they touch: `shift_pre` plus their `rel`. Multiples of the grid.
    shift_pre: i64,
    kind: TrackKind,
    /// Channels entering the node.
    ch: usize,
    /// Channels after the chain (a mono chain turns stereo at its first stereo block, FX-10).
    ch_out: usize,
    stream: Option<usize>,
    p: Vec<Param>,
    sends: Vec<Param>,
    blocks: Vec<Blk>,
    keys: Vec<KeyIn>,
    /// Delay bringing the chain to the constant latency of its class (FX-10, ENG-5).
    pad: Option<Line>,
    fx: Fx,
    fx_fade: Fade,
    /// Settings of the return's processor that are not parameters: shimmer interval, delay mode and character.
    ret_cfg: (f64, stemwise_dsp::delay::DelayMode, stemwise_dsp::delay::DelayCharacter),
    edges: Vec<Edge>,
    /// Solo gate (1 audible, 0 silenced by someone else's solo), ramped over a cell like mute (TRK-2).
    solo: Param,
    /// Post-fader output before the solo gate: the sidechain key of other nodes and sends to soloed returns.
    pre_solo: [Vec<f32>; 2],
    input: [Vec<f32>; 2],
    work: [Vec<f32>; 2],
    dry: [Vec<f32>; 2],
    out: [Vec<f32>; 2],
    meter: [PeakRms; 2],
    gr_db: f64,
    cell: i64,
    rate: f64,
    smooth_k: f64,
    /// Parameter id → slot, for the real-time queue (resolved on the control side).
    slots: HashMap<String, usize>,
}

/// Per-node meter values for the UI (ENG-6).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct NodeMeter {
    pub id: String,
    pub peak_db: [f64; 2],
    pub rms_db: [f64; 2],
    pub gr_db: f64,
}

/// Mix bus meters (TRK-2): sample peak, true peak, momentary and short-term loudness, correlation.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct BusMeter {
    pub peak_db: [f64; 2],
    pub rms_db: [f64; 2],
    pub true_peak_db: f64,
    pub momentary_lufs: f64,
    pub short_term_lufs: f64,
    pub correlation: f64,
    /// Gain reduction of the master chain (master phase): its blocks, the bands of its multibands, the limiter.
    pub master_gr: crate::master::Gr,
}

/// Source of audio-track input for the graph (offline: [`crate::offline`]; real time: the reader rings).
pub trait Feed {
    /// Fills `out` (interleaved, the track's channels) with the input of stream `stream` for the next frames.
    fn fill(&mut self, stream: usize, out: &mut [f32]);
}

pub struct Graph {
    pub rate: f64,
    nodes: Vec<Node>,
    /// `(start, len)` of each level in `nodes`.
    levels: Vec<(usize, usize)>,
    bus_in: [Vec<f32>; 2],
    bus_fader: Param,
    bus_cell: i64,
    smooth_k: f64,
    latency: usize,
    /// Streams in order: (node index, channels).
    pub streams: Vec<(usize, usize)>,
    /// Track ids of the streams, for the reader.
    pub stream_tracks: Vec<String>,
    feed_buf: Vec<f32>,
    bus_meter: [PeakRms; 2],
    bus_tp: [TruePeakStream; 2],
    bus_tp_max: f64,
    bus_lufs: LoudnessMeter,
    bus_corr: Correlation,
    index: HashMap<String, usize>,
    /// Run the nodes of a level on several threads with rayon (offline render only).
    pub parallel: bool,
    /// Real-time helper threads (A-024): nodes of a level are shared with them without allocating or locking.
    pub pool: Option<std::sync::Arc<crate::pool::Pool>>,
    /// The master chain on the bus (master phase, MAS-1); `None` in the mix phase.
    master: Option<Box<crate::master::MasterProc>>,
}

/// One level of nodes as a job for the helper pool.
struct NodesJob {
    cur: *mut Node,
    before: *const Node,
    before_len: usize,
    frame: i64,
    n: usize,
}

// SAFETY: each index is claimed by exactly one thread (see `pool`), nodes of a level are disjoint, and the earlier
// levels are only read.
unsafe impl Sync for NodesJob {}

impl crate::pool::LevelJob for NodesJob {
    fn run(&self, i: usize) {
        // SAFETY: as above; the pointers come from slices that outlive `Pool::run_level`.
        unsafe {
            let nd = &mut *self.cur.add(i);
            let before = std::slice::from_raw_parts(self.before, self.before_len);
            nd.run(self.frame, self.n, before);
        }
    }
}

const BUS: usize = usize::MAX;

fn get(t: &Track, id: &ParamId) -> f64 {
    params::get(t, id).unwrap_or_else(|| id.spec().default)
}

fn block2() -> [Vec<f32>; 2] {
    [vec![0.0; MAX_BLOCK], vec![0.0; MAX_BLOCK]]
}

impl Graph {
    /// Compiles a project (the routing must be valid: TRK-3).
    pub fn build(p: &Project, rate: f64) -> Result<Graph, routing::RoutingError> {
        let topo = routing::processing_order(p)?;
        // Dependency levels: a node comes after its inputs, the tracks sending to it and its sidechain sources.
        let pos_of: HashMap<&str, usize> =
            topo.iter().enumerate().map(|(k, &ti)| (p.tracks[ti].id.as_str(), k)).collect();
        let mut level = vec![0usize; topo.len()];
        for (k, &ti) in topo.iter().enumerate() {
            let t = &p.tracks[ti];
            for (_, src) in t.chain.sidechains() {
                if let Some(&s) = pos_of.get(src.as_str()) {
                    level[k] = level[k].max(level[s] + 1);
                }
            }
            let mut targets: Vec<usize> =
                t.sends.iter().filter_map(|s| pos_of.get(s.target.as_str()).copied()).collect();
            if let Output::Group(g) = &t.output
                && let Some(&d) = pos_of.get(g.as_str())
            {
                targets.push(d);
            }
            for d in targets {
                level[d] = level[d].max(level[k] + 1);
            }
        }
        let mut order: Vec<usize> = (0..topo.len()).collect();
        order.sort_by_key(|&k| (level[k], k));
        let order: Vec<usize> = order.into_iter().map(|k| topo[k]).collect();
        let lvl: Vec<usize> = {
            let mut v: Vec<usize> = (0..topo.len()).collect();
            v.sort_by_key(|&k| (level[k], k));
            v.into_iter().map(|k| level[k]).collect()
        };
        let mut levels: Vec<(usize, usize)> = Vec::new();
        for (i, &l) in lvl.iter().enumerate() {
            match levels.last_mut() {
                Some((start, len)) if lvl[*start] == l => *len += 1,
                _ => levels.push((i, 1)),
            }
        }

        // Constant latency (FX-10): every track and group chain takes the latency of the slowest of them, every
        // return the latency of the slowest return. Adding a block never moves a track against the others.
        let chain_lat = |t: &Track| t.chain.blocks.iter().map(|b| Blk::latency_of(b.fx.kind(), rate)).sum::<usize>();
        let lat_tracks = p.tracks.iter().filter(|t| !t.is_return()).map(chain_lat).max().unwrap_or(0);
        let lat_returns = p.tracks.iter().filter(|t| t.is_return()).map(chain_lat).max().unwrap_or(0);
        let lat_of = |k: TrackKind| if k == TrackKind::Return { lat_returns } else { lat_tracks };

        let node_ids: Vec<String> = order.iter().map(|&ti| p.tracks[ti].id.clone()).collect();
        let mask = solo_mask(p, &node_ids);
        let smooth_k = 1.0 - (-(N as f64) / (0.02 * rate)).exp();
        let mut nodes = Vec::with_capacity(order.len());
        let mut index = HashMap::new();
        let mut streams = Vec::new();
        let mut stream_tracks = Vec::new();
        for &ti in &order {
            let t = &p.tracks[ti];
            let ch = if t.is_audio() { usize::from(t.channels.clamp(1, 2)) } else { 2 };
            let lane = |id: &str| t.lane(id).filter(|l| !l.points.is_empty()).map(|l| l.points.clone());
            let mut slots: HashMap<String, usize> = HashMap::new();
            let mut pv: Vec<Param> = (0..FIXED).map(|_| Param::new(0.0, false, true)).collect();
            let mut fixed = |slot: usize, id: ParamId, pv: &mut Vec<Param>, rel: i64| {
                let mut prm = Param::new(get(t, &id), id.is_stepped(), true);
                prm.lane = lane(&id.to_string());
                prm.rel = rel;
                pv[slot] = prm;
                slots.insert(id.to_string(), slot);
            };
            let post = lat_of(t.kind) as i64;
            fixed(FADER, ParamId::Fader, &mut pv, post);
            fixed(PAN, ParamId::Pan, &mut pv, post);
            fixed(MUTE, ParamId::Mute, &mut pv, post);
            if t.is_return() {
                fixed(FXBY, ParamId::Bypass(FX.into()), &mut pv, 0);
                for (k, id) in ret_params(&t.ret).into_iter().enumerate() {
                    fixed(RET + k, id, &mut pv, 0);
                }
            } else {
                fixed(TRIM, ParamId::Trim, &mut pv, 0);
                pv[FXBY] = Param::new(1.0, true, false);
            }
            // The blocks, in chain order, each with its range of slots.
            let mut blocks = Vec::with_capacity(t.chain.blocks.len());
            let mut cum = 0usize;
            let mut cur_ch = ch;
            for b in &t.chain.blocks {
                let base = pv.len();
                let keys = keys_of(&b.fx);
                for k in &keys {
                    let id = ParamId::Block(b.id.clone(), k.clone());
                    let v = params::get(t, &id).unwrap_or_else(|| id.spec().default);
                    let mut prm = Param::new(v, false, true);
                    prm.lane = lane(&id.to_string());
                    prm.rel = cum as i64;
                    slots.insert(id.to_string(), pv.len());
                    pv.push(prm);
                }
                let by_id = ParamId::Bypass(b.id.clone());
                let mut byp = Param::new(get(t, &by_id), true, false);
                byp.lane = lane(&by_id.to_string());
                byp.rel = cum as i64;
                let by = pv.len();
                slots.insert(by_id.to_string(), by);
                pv.push(byp);
                let names: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
                let slot = |key: &str| base + names.iter().position(|n| n == key).expect("the block has the key");
                let mut blk = Blk::new(b, rate, by, &slot);
                if blk.kind == stemwise_core::fx::FxKind::Deesser {
                    // Listening to what the de-esser takes (FX-14): a slot of the engine only, never in the document,
                    // so the offline render and the export never listen.
                    slots.insert(format!("{}.listen", b.id), pv.len());
                    blk.set_listen_slot(pv.len());
                    pv.push(Param::new(0.0, true, false));
                }
                blk.rel = cum as i64;
                blk.promote = cur_ch == 1 && b.fx.stereo();
                if blk.promote {
                    cur_ch = 2;
                }
                blk.ch = cur_ch;
                cum += blk.lat;
                blocks.push(blk);
            }
            let sends = t
                .sends
                .iter()
                .map(|s| {
                    let mut prm = Param::new(s.level_db, false, true);
                    prm.lane = lane(&format!("send.{}", s.target));
                    prm.rel = post;
                    prm
                })
                .collect();
            let stream = if t.is_audio() {
                streams.push((nodes.len(), ch));
                stream_tracks.push(t.id.clone());
                Some(streams.len() - 1)
            } else {
                None
            };
            let fx = match &t.ret {
                Some(ReturnFx::Reverb { algorithm, .. }) => {
                    use stemwise_core::model::ReverbAlgorithm as A;
                    let a = match algorithm {
                        A::Plate => Algorithm::Plate,
                        A::Room => Algorithm::Room,
                        A::Hall => Algorithm::Hall,
                        A::Shimmer => Algorithm::Shimmer,
                    };
                    Fx::Reverb(Box::new(Reverb::new(rate, a)))
                }
                Some(ReturnFx::Delay { .. }) => Fx::Delay(Box::new(Delay::new(rate))),
                None => Fx::None,
            };
            let fx_by = pv[FXBY].target;
            let ret_cfg = match &t.ret {
                Some(ReturnFx::Reverb { shimmer_semitones, .. }) => {
                    (*shimmer_semitones, Default::default(), Default::default())
                }
                Some(ReturnFx::Delay { mode, character, .. }) => (
                    12.0,
                    match mode {
                        stemwise_core::model::DelayMode::Stereo => stemwise_dsp::delay::DelayMode::Stereo,
                        stemwise_core::model::DelayMode::PingPong => stemwise_dsp::delay::DelayMode::PingPong,
                    },
                    match character {
                        stemwise_core::model::DelayCharacter::Clean => stemwise_dsp::delay::DelayCharacter::Clean,
                        stemwise_core::model::DelayCharacter::Tape => stemwise_dsp::delay::DelayCharacter::Tape,
                    },
                ),
                None => (12.0, Default::default(), Default::default()),
            };
            index.insert(t.id.clone(), nodes.len());
            nodes.push(Node {
                id: t.id.clone(),
                shift_pre: 0,
                kind: t.kind,
                ch,
                ch_out: cur_ch,
                stream,
                p: pv,
                sends,
                blocks,
                keys: Vec::new(),
                pad: Line::new(lat_of(t.kind) - cum),
                fx,
                fx_fade: Fade::new(fx_by, rate),
                ret_cfg,
                edges: Vec::new(),
                solo: Param::new(if mask.audible(nodes.len()) { 1.0 } else { 0.0 }, true, false),
                pre_solo: block2(),
                input: block2(),
                work: block2(),
                dry: block2(),
                out: block2(),
                meter: [PeakRms::default(); 2],
                gr_db: 0.0,
                cell: i64::MIN,
                rate,
                smooth_k,
                slots,
            });
        }
        // Edges and latency compensation.
        let mut l_in = vec![0usize; nodes.len()];
        let mut l_out = vec![0usize; nodes.len()];
        let mut bus_l_in = 0usize;
        let dest = |t: &Track| -> usize {
            match &t.output {
                Output::MixBus => BUS,
                Output::Group(g) => index[g],
            }
        };
        // Key inputs of a node: (block position, band, source node, latency of the blocks before the block).
        let keys_of_track = |t: &Track, ni: usize, nodes: &[Node]| -> Vec<(usize, usize, usize, usize)> {
            let mut out = Vec::new();
            for (bi, b) in t.chain.blocks.iter().enumerate() {
                let rel = nodes[ni].blocks[bi].rel as usize;
                let srcs: Vec<(usize, &String)> = match &b.fx {
                    stemwise_core::fx::Fx::DynamicEq(d) => d
                        .bands
                        .iter()
                        .enumerate()
                        .filter(|(_, x)| x.enabled)
                        .filter_map(|(i, x)| x.sidechain.as_ref().map(|s| (i, s)))
                        .collect(),
                    fx => fx.sidechains().into_iter().map(|s| (0, s)).collect(),
                };
                for (band, src) in srcs {
                    if let Some(&k) = index.get(src) {
                        out.push((bi, band.min(1), k, rel));
                    }
                }
            }
            out
        };
        for (ni, &ti) in order.iter().enumerate() {
            let t = &p.tracks[ti];
            // A key is an input too: its source must not arrive later than the audio at the block (ENG-5).
            for (_, _, k, rel) in keys_of_track(t, ni, &nodes) {
                l_in[ni] = l_in[ni].max(l_out[k].saturating_sub(rel));
            }
            l_out[ni] = l_in[ni] + lat_of(nodes[ni].kind);
            let mut targets = vec![dest(t)];
            targets.extend(t.sends.iter().map(|s| index[&s.target]));
            for d in targets {
                if d == BUS {
                    bus_l_in = bus_l_in.max(l_out[ni]);
                } else {
                    l_in[d] = l_in[d].max(l_out[ni]);
                }
            }
        }
        for (ni, &ti) in order.iter().enumerate() {
            let t = &p.tracks[ti];
            let mk = |d: usize, send: Option<usize>| {
                let want = if d == BUS { bus_l_in } else { l_in[d] };
                Edge {
                    to: d,
                    delay: Line::new(want - l_out[ni]),
                    send,
                    out: block2(),
                    solo_pass: send.is_some() && d != BUS && mask.soloed_return(d),
                }
            };
            let mut edges = vec![mk(dest(t), None)];
            for (k, s) in t.sends.iter().enumerate() {
                edges.push(mk(index[&s.target], Some(k)));
            }
            nodes[ni].edges = edges;
            nodes[ni].shift_pre = l_in[ni] as i64;
            let mut keys: Vec<KeyIn> = Vec::new();
            let mut key_rel: Vec<(usize, usize)> = Vec::new();
            for (bi, band, src, rel) in keys_of_track(t, ni, &nodes) {
                let at = match key_rel.iter().position(|&x| x == (src, rel)) {
                    Some(i) => i,
                    None => {
                        key_rel.push((src, rel));
                        keys.push(KeyIn { src, delay: Line::new(l_in[ni] + rel - l_out[src]), buf: block2() });
                        keys.len() - 1
                    }
                };
                nodes[ni].blocks[bi].keys[band] = Some(at);
            }
            nodes[ni].keys = keys;
        }
        Ok(Graph {
            rate,
            nodes,
            levels,
            bus_in: block2(),
            bus_fader: Param::new(p.mix_bus.fader_db, false, true),
            bus_cell: i64::MIN,
            smooth_k,
            latency: bus_l_in,
            streams,
            stream_tracks,
            feed_buf: vec![0.0; MAX_BLOCK * 2],
            bus_meter: [PeakRms::default(); 2],
            bus_tp: [TruePeakStream::default(), TruePeakStream::default()],
            bus_tp_max: 0.0,
            bus_lufs: LoudnessMeter::new(rate),
            bus_corr: Correlation::default(),
            index,
            parallel: false,
            pool: None,
            master: None,
        })
    }

    /// Parameter id → slot of every node, in node order (the control side resolves real-time changes with it).
    pub fn param_slots(&self) -> Vec<HashMap<String, usize>> {
        self.nodes.iter().map(|n| n.slots.clone()).collect()
    }

    /// Frames by which the bus output lags the timeline (ENG-5), master chain included.
    pub fn latency(&self) -> usize {
        self.latency + self.master.as_ref().map_or(0, |m| m.latency())
    }

    /// Puts a master chain on the bus (master phase). Allocates: control side only.
    pub fn set_master_chain(&mut self, m: Option<crate::master::MasterProc>) {
        self.master = m.map(Box::new);
    }

    /// New master parameters (audio thread, no allocation).
    pub fn set_master_params(&mut self, p: crate::master::MasterParams) {
        if let Some(m) = &mut self.master {
            m.set(p);
        }
    }

    /// Gain reduction of the master chain since the last call.
    pub fn take_master_gr(&mut self) -> crate::master::Gr {
        self.master.as_mut().map_or_else(crate::master::Gr::default, |m| m.take_gr())
    }

    /// Sets a static parameter value (a user change: smoothed, ENG-3). Unknown targets are ignored.
    pub fn set_param(&mut self, track: &str, param: &str, value: f64) {
        let Some(&ni) = self.index.get(track) else { return };
        if let Some(&sl) = self.nodes[ni].slots.get(param) {
            self.nodes[ni].p[sl].target = value;
        }
    }

    /// Like `set_param` but for sends, identified by the return's id.
    pub fn set_send(&mut self, track: &str, ret: &str, value: f64) {
        let (Some(&ni), Some(&ri)) = (self.index.get(track), self.index.get(ret)) else { return };
        self.set_param_index(ni, crate::realtime::ParamTarget::Send(ri as u32), value);
    }

    /// Node index of a track id (for the real-time parameter queue).
    pub fn node_of(&self, track: &str) -> Option<usize> {
        self.index.get(track).copied()
    }

    /// Real-time path: a parameter addressed by node index and slot (resolved on the control side).
    pub fn set_param_index(&mut self, node: usize, target: crate::realtime::ParamTarget, value: f64) {
        let Some(n) = self.nodes.get_mut(node) else { return };
        match target {
            crate::realtime::ParamTarget::Slot(s) => {
                if let Some(p) = n.p.get_mut(s as usize) {
                    p.target = value;
                }
            }
            crate::realtime::ParamTarget::Send(ret) => {
                if let Some(k) = n.edges.iter().find(|e| e.to == ret as usize).and_then(|e| e.send) {
                    n.sends[k].target = value;
                }
            }
        }
    }

    /// Applies a solo mask computed on the control side ([`solo_mask`]): no rebuild, ramped gates (TRK-2).
    pub fn set_solo(&mut self, mask: &SoloMask) {
        for (i, n) in self.nodes.iter_mut().enumerate() {
            n.solo.target = if mask.audible(i) { 1.0 } else { 0.0 };
            for e in &mut n.edges {
                e.solo_pass = e.send.is_some() && e.to != BUS && mask.soloed_return(e.to);
            }
        }
    }

    pub fn set_bus_fader(&mut self, db: f64) {
        self.bus_fader.target = db;
    }

    /// Clears all processor state and snaps parameters to `frame` (start, seek, offline render).
    pub fn reset(&mut self, frame: i64) {
        if let Some(m) = &mut self.master {
            m.reset(frame);
        }
        let cell = frame.div_euclid(N as i64) * N as i64;
        self.bus_fader.snap(cell);
        self.bus_cell = cell;
        for n in &mut self.nodes {
            // Parameters first, then the processors' state (the order of schema 3).
            n.update(cell, true);
            n.cell = cell;
            for b in &mut n.blocks {
                b.reset();
            }
            match &mut n.fx {
                Fx::Reverb(r) => r.reset(),
                Fx::Delay(d) => d.reset(),
                Fx::None => {}
            }
            for d in n
                .edges
                .iter_mut()
                .filter_map(|e| e.delay.as_mut())
                .chain(n.keys.iter_mut().filter_map(|k| k.delay.as_mut()))
                .chain(n.pad.as_mut())
            {
                d.clear();
            }
        }
    }

    /// Renders `out_l.len()` frames starting at timeline `frame` (which must follow the previous call, or come
    /// after `reset`). Output is the mix bus, `latency()` frames late.
    pub fn process(&mut self, frame: i64, out_l: &mut [f32], out_r: &mut [f32], feed: &mut dyn Feed) {
        let total = out_l.len();
        let mut done = 0;
        while done < total {
            let n = (total - done).min(MAX_BLOCK);
            self.block(frame + done as i64, n, feed);
            out_l[done..done + n].copy_from_slice(&self.bus_in[0][..n]);
            out_r[done..done + n].copy_from_slice(&self.bus_in[1][..n]);
            done += n;
        }
    }

    fn block(&mut self, frame: i64, n: usize, feed: &mut dyn Feed) {
        // Inputs: audio tracks from the feed, groups and returns from their sources (accumulated below).
        for node in &mut self.nodes {
            match node.stream {
                Some(s) => {
                    let ch = node.ch;
                    let buf = &mut self.feed_buf[..n * ch];
                    feed.fill(s, buf);
                    for c in 0..ch {
                        for i in 0..n {
                            node.work[c][i] = buf[i * ch + c];
                        }
                    }
                }
                None => {
                    node.input[0][..n].fill(0.0);
                    node.input[1][..n].fill(0.0);
                }
            }
        }
        self.bus_in[0][..n].fill(0.0);
        self.bus_in[1][..n].fill(0.0);
        for li in 0..self.levels.len() {
            let (start, len) = self.levels[li];
            let (before, rest) = self.nodes.split_at_mut(start);
            let (cur, after) = rest.split_at_mut(len);
            let before: &[Node] = before;
            if let Some(pool) = self.pool.as_ref().filter(|_| len > 1) {
                let job =
                    NodesJob { cur: cur.as_mut_ptr(), before: before.as_ptr(), before_len: before.len(), frame, n };
                pool.run_level(&job, len);
            } else if self.parallel && len > 1 && n >= 256 {
                cur.par_iter_mut().for_each(|nd| nd.run(frame, n, before));
            } else {
                for nd in cur.iter_mut() {
                    nd.run(frame, n, before);
                }
            }
            // Contributions to later nodes and to the bus, serially and in a fixed order.
            let end = start + len;
            for nd in cur.iter() {
                for e in &nd.edges {
                    let dst = if e.to == BUS { &mut self.bus_in } else { &mut after[e.to - end].input };
                    for c in 0..2 {
                        for (d, v) in dst[c][..n].iter_mut().zip(&e.out[c][..n]) {
                            *d += *v;
                        }
                    }
                }
            }
        }
        // Mix bus: fader (per-sample ramp on the grid) and meters.
        let mut done = 0;
        while done < n {
            let f = frame + done as i64;
            let cell = f.div_euclid(N as i64) * N as i64;
            if cell != self.bus_cell {
                self.bus_fader.eval(cell - self.latency as i64, self.smooth_k);
                self.bus_cell = cell;
            }
            let off = (f - cell) as usize;
            let len = (N - off).min(n - done);
            let (a, b) = (db_to_gain(self.bus_fader.a), db_to_gain(self.bus_fader.b));
            for c in 0..2 {
                for i in 0..len {
                    let g = a + (b - a) * ((off + i) as f64 / N as f64);
                    let v = &mut self.bus_in[c][done + i];
                    *v = (f64::from(*v) * g) as f32;
                }
            }
            done += len;
        }
        if let Some(m) = &mut self.master {
            let (l, r) = self.bus_in.split_at_mut(1);
            m.process_at(frame, &mut l[0][..n], &mut r[0][..n]);
        }
        let (l, r) = self.bus_in.split_at(1);
        let (l, r) = (&l[0][..n], &r[0][..n]);
        self.bus_meter[0].feed(l);
        self.bus_meter[1].feed(r);
        self.bus_lufs.process(l, r);
        self.bus_corr.feed(l, r);
        for (c, x) in [l, r].into_iter().enumerate() {
            for &v in x {
                let t = self.bus_tp[c].push(f64::from(v));
                if t > self.bus_tp_max {
                    self.bus_tp_max = t;
                }
            }
        }
    }

    /// Meter values since the last call (allocation-free into the caller's buffers).
    pub fn take_meters(&mut self, nodes: &mut [NodeMeter], bus: &mut BusMeter) {
        for (m, n) in nodes.iter_mut().zip(self.nodes.iter_mut()) {
            let (p0, r0) = n.meter[0].take();
            let (p1, r1) = n.meter[1].take();
            m.peak_db = [p0, p1];
            m.rms_db = [r0, r1];
            m.gr_db = std::mem::replace(&mut n.gr_db, 0.0);
        }
        let (p0, r0) = self.bus_meter[0].take();
        let (p1, r1) = self.bus_meter[1].take();
        bus.peak_db = [p0, p1];
        bus.rms_db = [r0, r1];
        bus.true_peak_db = 20.0 * std::mem::replace(&mut self.bus_tp_max, 0.0).log10();
        bus.momentary_lufs = self.bus_lufs.momentary;
        bus.short_term_lufs = self.bus_lufs.short_term;
        bus.correlation = self.bus_corr.take();
        bus.master_gr = self.take_master_gr();
    }

    /// Post-fader output of a node over the last `n` frames processed (the live spectrum of the EQ editor, FX-7).
    pub fn watched_output(&self, node: usize, n: usize) -> Option<(&[f32], &[f32])> {
        let nd = self.nodes.get(node)?;
        let n = n.min(nd.pre_solo[0].len());
        Some((&nd.pre_solo[0][..n], &nd.pre_solo[1][..n]))
    }

    /// Meter slots matching the node order (allocate once, outside the audio thread).
    pub fn meter_slots(&self) -> Vec<NodeMeter> {
        self.nodes.iter().map(|n| NodeMeter { id: n.id.clone(), ..Default::default() }).collect()
    }
}

impl Node {
    fn update(&mut self, cell: i64, snap: bool) {
        let k = self.smooth_k;
        let rate = self.rate;
        for p in self.p.iter_mut().chain(self.sends.iter_mut()) {
            let at = cell - self.shift_pre - p.rel;
            if snap {
                p.snap(at);
            } else {
                p.eval(at, k);
            }
        }
        let post = self.p[FADER].rel;
        let at = cell - self.shift_pre - post;
        if snap {
            self.solo.snap(at);
        } else {
            self.solo.eval(at, k);
        }
        let pv = &self.p;
        let v = |s: usize| pv[s].a;
        let shift = self.shift_pre;
        let cycles = |s: usize| {
            let at = cell - shift - pv[s].rel;
            (pv[s].cycles(at, rate), at)
        };
        for b in &mut self.blocks {
            b.update(&v, &cycles, pv[b.by].b, rate, snap);
        }
        if snap {
            self.fx_fade.snap(self.p[FXBY].b);
        } else {
            self.fx_fade.target = self.p[FXBY].b;
        }
        let r = |i: usize| self.p[RET + i].a;
        match &mut self.fx {
            Fx::Reverb(rv) => rv.set_params(&ReverbParams {
                decay_s: r(0),
                size: r(1),
                damping: r(2),
                modulation: r(3),
                predelay_ms: r(4),
                hpf_hz: r(5),
                lpf_hz: r(6),
                shimmer_semitones: self.ret_cfg.0,
                shimmer_amount: r(7),
            }),
            Fx::Delay(d) => d.set_params(&DelayParams {
                time_ms: r(0),
                feedback: r(1),
                feedback_hpf_hz: r(2),
                feedback_lpf_hz: r(3),
                mode: self.ret_cfg.1,
                character: self.ret_cfg.2,
                wow: r(4),
                drive_db: r(5),
            }),
            Fx::None => {}
        }
    }

    /// Processes `n` frames from `frame`: input already in `work` (audio) or `input` (groups, returns). `before`:
    /// the nodes of earlier levels (sidechain sources).
    fn run(&mut self, frame: i64, n: usize, before: &[Node]) {
        if self.stream.is_none() {
            for c in 0..2 {
                let (src, dst) = (&self.input[c], &mut self.work[c]);
                dst[..n].copy_from_slice(&src[..n]);
            }
        }
        let mut done = 0;
        while done < n {
            let f = frame + done as i64;
            let cell = f.div_euclid(N as i64) * N as i64;
            if cell != self.cell {
                self.update(cell, false);
                self.cell = cell;
            }
            let off = (f - cell) as usize;
            let len = (N - off).min(n - done);
            self.piece(done, off, len, before);
            done += len;
        }
        self.meter[0].feed(&self.out[0][..n]);
        self.meter[1].feed(&self.out[1][..n]);
    }

    fn piece(&mut self, at: usize, off: usize, len: usize, before: &[Node]) {
        let ramp = |a: f64, b: f64, i: usize| a + (b - a) * ((off + i) as f64 / N as f64);
        let ab = |p: &[Param], s: usize| (p[s].a, p[s].b);
        let r = at..at + len;
        let ch = self.ch;
        // Sidechain keys of this piece, through their alignment delays.
        for k in &mut self.keys {
            let src = &before[k.src];
            for c in 0..2 {
                for i in 0..len {
                    let v = src.pre_solo[c][at + i];
                    k.buf[c][at + i] = match &mut k.delay {
                        Some(d) => d.tap(c, i, v),
                        None => v,
                    };
                }
            }
            if let Some(d) = &mut k.delay {
                d.advance(len);
            }
        }
        if self.kind != TrackKind::Return {
            // Trim.
            let (ta, tb) = ab(&self.p, TRIM);
            if ta != 0.0 || tb != 0.0 {
                let (ga, gb) = (db_to_gain(ta), db_to_gain(tb));
                for c in 0..ch {
                    for (i, v) in self.work[c][r.clone()].iter_mut().enumerate() {
                        *v = (f64::from(*v) * ramp(ga, gb, i)) as f32;
                    }
                }
            }
        } else {
            // Return: reverb or delay, 100% wet; bypass silences it (with the fade of FX-10).
            let (w0, w1) = self.work.split_at(1);
            let (d0, d1) = self.dry.split_at_mut(1);
            match &mut self.fx {
                Fx::Reverb(rv) => {
                    rv.process(&w0[0][r.clone()], &w1[0][r.clone()], &mut d0[0][r.clone()], &mut d1[0][r.clone()])
                }
                Fx::Delay(dl) => dl.process_at(
                    &w0[0][r.clone()],
                    &w1[0][r.clone()],
                    &mut d0[0][r.clone()],
                    &mut d1[0][r.clone()],
                    self.cell + off as i64 - self.shift_pre,
                ),
                Fx::None => {
                    d0[0][r.clone()].fill(0.0);
                    d1[0][r.clone()].fill(0.0);
                }
            }
            self.fx_fade.fill(len);
            for c in 0..2 {
                for (i, (v, d)) in self.work[c][r.clone()].iter_mut().zip(&self.dry[c][r.clone()]).enumerate() {
                    *v = (f64::from(*d) * (1.0 - self.fx_fade.buf[i])) as f32;
                }
            }
        }
        // The blocks, in chain order.
        if !self.blocks.is_empty() {
            let keys: [[&[f32]; 2]; 8] = std::array::from_fn(|i| match self.keys.get(i) {
                Some(k) => [&k.buf[0][r.clone()], &k.buf[1][r.clone()]],
                None => [&[][..], &[][..]],
            });
            // The timeline frame of the audio this piece carries at the chain's input (song position: FX-20).
            let at_in = self.cell + off as i64 - self.shift_pre;
            for b in &mut self.blocks {
                b.process(&mut self.work, &mut self.dry, r.clone(), &keys, &mut self.gr_db, at_in - b.rel);
            }
        }
        let ch = self.ch_out;
        if let Some(pad) = &mut self.pad {
            pad.run(&mut self.work, r.clone(), ch);
        }
        // Fader, pan, mute (and solo). A chain that was mono keeps the mono pan law on its two channels.
        let (fa, fb) = ab(&self.p, FADER);
        let (pa, pb) = ab(&self.p, PAN);
        let (ma, mb) = ab(&self.p, MUTE);
        let (sa, sb) = (self.solo.a, self.solo.b);
        let mono_law = self.ch == 1;
        let (la, ra) = if mono_law { mono_gains(pa) } else { balance_gains(pa) };
        let (lb, rb) = if mono_law { mono_gains(pb) } else { balance_gains(pb) };
        let (ga, gb) = (db_to_gain(fa) * (1.0 - ma), db_to_gain(fb) * (1.0 - mb));
        for i in 0..len {
            let g = ramp(ga, gb, i);
            let sg = ramp(sa, sb, i);
            let (l, rr) = (ramp(la, lb, i), ramp(ra, rb, i));
            let x0 = f64::from(self.work[0][at + i]);
            let x1 = if ch == 1 { x0 } else { f64::from(self.work[1][at + i]) };
            let (y0, y1) = (x0 * g * l, x1 * g * rr);
            self.pre_solo[0][at + i] = y0 as f32;
            self.pre_solo[1][at + i] = y1 as f32;
            self.out[0][at + i] = (y0 * sg) as f32;
            self.out[1][at + i] = (y1 * sg) as f32;
        }
        // Edges: main output and post-fader sends, through their compensation delays.
        for e in &mut self.edges {
            let gains = e.send.map(|k| (db_to_gain(self.sends[k].a), db_to_gain(self.sends[k].b)));
            for c in 0..2 {
                for i in 0..len {
                    let mut v = if e.solo_pass { self.pre_solo[c][at + i] } else { self.out[c][at + i] };
                    if let Some((a, b)) = gains {
                        v = (f64::from(v) * ramp(a, b, i)) as f32;
                    }
                    e.out[c][at + i] = match &mut e.delay {
                        Some(d) => d.tap(c, i, v),
                        None => v,
                    };
                }
            }
            if let Some(d) = &mut e.delay {
                d.advance(len);
            }
        }
    }
}

/// Solo state of the nodes of a graph, as fixed-size bitsets (sent to the audio thread without allocating).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoloMask {
    silenced: [u64; 4],
    soloed_returns: [u64; 4],
}

impl SoloMask {
    pub fn audible(&self, node: usize) -> bool {
        node >= 256 || self.silenced[node / 64] & (1 << (node % 64)) == 0
    }

    pub fn soloed_return(&self, node: usize) -> bool {
        node < 256 && self.soloed_returns[node / 64] & (1 << (node % 64)) != 0
    }
}

/// Solo-in-place (TRK-2), computed on the control side for nodes in graph order. When something is soloed, audible
/// are: the soloed tracks, the groups they feed up to the bus, the members of a soloed group, every return ("returns
/// follow") and the groups returns feed. A soloed return keeps receiving the sends of silenced tracks. Sidechain keys
/// are taken before the solo gate, so a soloed track sounds as it does in the mix.
pub fn solo_mask(p: &Project, node_ids: &[String]) -> SoloMask {
    let mut m = SoloMask { silenced: [0; 4], soloed_returns: [0; 4] };
    let ancestors = |t: &Track| {
        let mut out = Vec::new();
        let mut cur = t;
        while let Output::Group(g) = &cur.output {
            if out.len() > p.tracks.len() {
                break;
            }
            out.push(g.clone());
            match p.track(g) {
                Some(n) => cur = n,
                None => break,
            }
        }
        out
    };
    let mut audible = std::collections::HashSet::new();
    let any = p.tracks.iter().any(|t| t.solo);
    for t in &p.tracks {
        let up = ancestors(t);
        if t.solo || t.is_return() || up.iter().any(|g| p.track(g).is_some_and(|x| x.solo)) {
            audible.insert(t.id.clone());
        }
        if t.solo || t.is_return() {
            audible.extend(up);
        }
    }
    for (i, id) in node_ids.iter().enumerate().take(256) {
        let t = p.track(id);
        if any && !audible.contains(id) {
            m.silenced[i / 64] |= 1 << (i % 64);
        }
        if t.is_some_and(|t| t.is_return() && t.solo) {
            m.soloed_returns[i / 64] |= 1 << (i % 64);
        }
    }
    m
}
