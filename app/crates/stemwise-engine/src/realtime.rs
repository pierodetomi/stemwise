//! Real-time playback (ENG-1 … ENG-3, ENG-6, TRN-1): reader thread, lock-free rings, the audio callback and the
//! control-side handle.
//!
//! - The **audio callback** ([`AudioProcessor::callback`]) runs the same `Graph::process` as the offline render.
//!   It never allocates, frees, locks or does I/O: messages arrive on an `rtrb` queue (parameters, solo, bus fader,
//!   transport, a new graph); the old graph goes back on another queue to be dropped by the control thread. At most
//!   one graph swap is in flight, so that queue can never be full (no memory is ever freed on the audio thread).
//! - The **reader thread** assembles each audio track's input with the same `TrackStream` as the render and
//!   pushes it into one ring per track, ahead of the playhead.
//! - **Seek protocol** (no lock, no timeout): the audio thread publishes a generation and a position; the reader
//!   stops and acknowledges; the audio thread drains the stale data and confirms; the reader refills from the new
//!   position. Until the rings are primed the callback outputs silence. The output fades out over [`FADE`] frames
//!   before a pause, seek or swap and fades in when playback resumes (no clicks).
//! - **Loop**: both sides wrap to the loop start only when the position reaches the loop end exactly, so they
//!   always agree; a playhead after the loop end plays on.
//! - **Positions** exposed to the control side are timeline positions (the graph latency is internal).
//! - **Meters and position** go to the control side at 50 Hz on a ring of fixed-size frames (ENG-6).

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rtrb::{Consumer, Producer, RingBuffer};
use stemwise_core::model::Project;

use crate::clips::TrackStream;
use crate::graph::{BusMeter, Feed, Graph, NodeMeter, SoloMask, solo_mask};

/// Most nodes whose meters are reported.
pub const MAX_METERS: usize = 192;
/// Read-ahead per track, in seconds.
const RING_SECONDS: f64 = 1.0;
const CHUNK: usize = 2048;
/// Fade length on transport changes, in frames.
pub const FADE: usize = 128;

/// Shared atomics between the control side, the reader and the audio thread.
#[derive(Default)]
pub struct Shared {
    req_gen: AtomicU64,
    req_pos: AtomicI64,
    ack_gen: AtomicU64,
    drained_gen: AtomicU64,
    loop_on: AtomicBool,
    loop_start: AtomicI64,
    loop_end: AtomicI64,
    pub playing: AtomicBool,
    /// Graph position (timeline frame of the audio entering the graph).
    pub position: AtomicI64,
    pub latency: AtomicI64,
    /// Callback load in 1/1000 of the period (NFR-1).
    pub load_permille: AtomicU32,
    pub xruns: AtomicU64,
    /// Graph swaps the audio thread has taken (the control side sends the next one only after the previous).
    pub swaps_taken: AtomicU64,
    pub running: AtomicBool,
    stop: AtomicBool,
}

/// A graph ready for the audio thread, with the consumers of its input rings.
pub struct Live {
    pub graph: Graph,
    pub consumers: Vec<Consumer<f32>>,
    pub nodes: usize,
    pub id: u64,
    /// Node whose output feeds the spectrum ring (FX-7), if any.
    pub watch: Option<usize>,
    /// Sequence number of the master parameters the graph carries (see [`Engine::set_master`]).
    pub master_seq: u64,
}

/// Commands to the audio thread (no heap work on the audio side).
pub enum Msg {
    Swap(Box<Live>),
    /// A parameter of the graph `graph` (its id): the layout of nodes and slots changes with every graph, so a
    /// change made while the previous one fades out goes to the graph it was resolved for (F15 review, #7).
    Param {
        graph: u64,
        track: u32,
        slot: ParamTarget,
        value: f64,
    },
    Solo(SoloMask),
    BusFader(f64),
    Play,
    Pause,
    Locate(i64),
    Loop(Option<(i64, i64)>),
    /// The node whose output feeds the spectrum (FX-7), by index in the current graph; [`WATCH_BUS`] for the bus.
    Watch(Option<u32>),
}

/// Watch index meaning "the bus output" (the master phase's spectrum, MAS-4).
pub const WATCH_BUS: u32 = u32::MAX;
/// Track id that watches the bus.
pub const BUS_ID: &str = "__bus__";

/// Samples kept for the spectrum (one FFT frame).
pub const SPECTRUM_FFT: usize = 4096;
/// Log-spaced bands of the spectrum sent to the UI, 20 Hz … 20 kHz.
pub const SPECTRUM_BANDS: usize = 96;

/// A resolved parameter address (node index and slot), computed on the control side.
#[derive(Clone, Copy, Debug)]
pub enum ParamTarget {
    Slot(u32),
    Send(u32),
}

/// Meter frame sent to the control side.
#[derive(Clone, Copy)]
pub struct MeterFrame {
    /// Timeline position of what is heard.
    pub position: i64,
    pub playing: bool,
    pub graph: u64,
    pub nodes: usize,
    pub node: [[f32; 5]; MAX_METERS],
    /// [peak L, peak R, rms L, rms R, true peak, momentary, short-term, correlation, deepest GR of the master's
    /// blocks, limiter GR].
    pub bus: [f32; 10],
    /// Gain reduction of the master chain: its free blocks in chain order, then the bands of each multiband.
    pub master: [f32; MASTER_GR],
}

/// Values of [`MeterFrame::master`].
pub const MASTER_GR: usize = crate::master::MAX_FREE + crate::master::PER_KIND * stemwise_dsp::multiband::MAX_BANDS;

impl Default for MeterFrame {
    fn default() -> Self {
        MeterFrame {
            position: 0,
            playing: false,
            graph: 0,
            nodes: 0,
            node: [[-200.0; 5]; MAX_METERS],
            bus: [-200.0; 10],
            master: [0.0; MASTER_GR],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Running,
    Waiting(u64),
    Priming,
}

/// What a fade-out leads to once it is complete: a pause, a seek, or both (they accumulate).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Pending {
    pause: bool,
    seek: Option<i64>,
}

impl Pending {
    const NONE: Pending = Pending { pause: false, seek: None };
}

/// The state that lives on the audio thread.
pub struct AudioProcessor {
    shared: Arc<Shared>,
    live: Option<Box<Live>>,
    spare: Option<Box<Live>>,
    rx: Consumer<Msg>,
    trash: Producer<Box<Live>>,
    meters: Producer<MeterFrame>,
    /// Mono samples of the watched node for the spectrum, read on the control side.
    spectrum: Producer<f32>,
    /// New master parameters (master phase, MAS-1), on their own small queue: only the latest applies.
    master_rx: Consumer<(u64, crate::master::MasterParams)>,
    /// The newest master parameters received, applied to a graph that arrives with older ones.
    last_master: Option<(u64, crate::master::MasterParams)>,
    state: State,
    generation: u64,
    pos: i64,
    loop_region: Option<(i64, i64)>,
    l: Vec<f32>,
    r: Vec<f32>,
    meter_tmp: Vec<NodeMeter>,
    bus_tmp: BusMeter,
    since_meter: usize,
    meter_every: usize,
    rate: f64,
    /// Frames left of a fade-in (counting down) and the transition a fade-out leads to.
    fade_in: usize,
    pending: Pending,
}

struct RingFeed<'a> {
    consumers: &'a mut [Consumer<f32>],
    short: bool,
}

impl Feed for RingFeed<'_> {
    fn fill(&mut self, stream: usize, out: &mut [f32]) {
        let c = &mut self.consumers[stream];
        let n = out.len().min(c.slots());
        if let Ok(chunk) = c.read_chunk(n) {
            let (a, b) = chunk.as_slices();
            out[..a.len()].copy_from_slice(a);
            out[a.len()..a.len() + b.len()].copy_from_slice(b);
            chunk.commit_all();
        }
        if n < out.len() {
            out[n..].fill(0.0);
            self.short = true;
        }
    }
}

impl AudioProcessor {
    fn seek(&mut self, pos: i64) {
        self.pos = pos;
        self.generation += 1;
        self.shared.req_pos.store(pos, Ordering::Release);
        self.shared.req_gen.store(self.generation, Ordering::Release);
        self.state = State::Waiting(self.generation);
        self.shared.position.store(pos, Ordering::Relaxed);
    }

    fn drain(&mut self) {
        if let Some(live) = &mut self.live {
            for c in &mut live.consumers {
                let n = c.slots();
                if let Ok(ch) = c.read_chunk(n) {
                    ch.commit_all();
                }
            }
        }
    }

    fn running(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed) && self.state == State::Running && self.live.is_some()
    }

    /// Requests a transition that interrupts the audio: fades out first when audio is running.
    fn interrupt(&mut self, pause: bool, seek: Option<i64>) {
        self.pending.pause |= pause;
        if seek.is_some() {
            self.pending.seek = seek;
        }
        if !self.running() {
            self.finish();
        }
    }

    fn finish(&mut self) {
        let then = std::mem::replace(&mut self.pending, Pending::NONE);
        if then.pause {
            self.shared.playing.store(false, Ordering::Relaxed);
        }
        if let Some(p) = then.seek {
            self.seek(p);
        }
    }

    fn handle_messages(&mut self) {
        let mut latest = None;
        while let Ok(p) = self.master_rx.pop() {
            latest = Some(p);
        }
        if let Some((seq, p)) = latest {
            self.last_master = Some((seq, p));
            for l in [&mut self.live, &mut self.spare].into_iter().flatten() {
                if l.master_seq < seq {
                    l.graph.set_master_params(p);
                    l.master_seq = seq;
                }
            }
        }
        while let Ok(m) = self.rx.pop() {
            match m {
                Msg::Swap(mut new) => {
                    // Parameters newer than the ones the graph was built with (S5 of the F8 review).
                    if let Some((seq, p)) = self.last_master
                        && seq > new.master_seq
                    {
                        new.graph.set_master_params(p);
                        new.master_seq = seq;
                    }
                    // Keep the new graph aside until the current one has faded out.
                    self.spare = Some(new);
                    let p = self.pos;
                    self.interrupt(false, Some(p));
                }
                Msg::Param { graph, track, slot, value } => {
                    for l in [self.live.as_deref_mut(), self.spare.as_deref_mut()].into_iter().flatten() {
                        if l.id == graph {
                            l.graph.set_param_index(track as usize, slot, value);
                        }
                    }
                }
                Msg::Solo(mask) => {
                    if let Some(l) = &mut self.live {
                        l.graph.set_solo(&mask);
                    }
                }
                Msg::BusFader(v) => {
                    if let Some(l) = &mut self.live {
                        l.graph.set_bus_fader(v);
                    }
                }
                Msg::Play => {
                    if !self.shared.playing.load(Ordering::Relaxed) || self.state != State::Running {
                        self.shared.playing.store(true, Ordering::Relaxed);
                        let p = self.pos;
                        self.seek(p);
                    }
                }
                Msg::Pause => self.interrupt(true, None),
                Msg::Locate(p) => {
                    // A locate also cancels a pending pause's seek target but not the pause itself.
                    self.interrupt(false, Some(p.max(0)));
                }
                Msg::Watch(w) => {
                    if let Some(l) = &mut self.live {
                        l.watch = w.map(|x| x as usize);
                    }
                    if let Some(l) = &mut self.spare {
                        l.watch = w.map(|x| x as usize);
                    }
                }
                Msg::Loop(r) => {
                    self.loop_region = r.filter(|(a, b)| b > a);
                    if let Some((a, b)) = self.loop_region {
                        self.shared.loop_start.store(a, Ordering::Release);
                        self.shared.loop_end.store(b, Ordering::Release);
                    }
                    self.shared.loop_on.store(self.loop_region.is_some(), Ordering::Release);
                    let p = self.pending.seek.unwrap_or(self.pos);
                    self.interrupt(false, Some(p));
                }
            }
        }
    }

    fn install_spare(&mut self) {
        if let Some(new) = self.spare.take() {
            let lat = new.graph.latency();
            if let Some(old) = self.live.replace(new)
                && let Err(rtrb::PushError::Full(old)) = self.trash.push(old)
            {
                // Unreachable by construction (one swap in flight); never free on this thread.
                self.spare = Some(old);
            }
            self.shared.latency.store(lat as i64, Ordering::Relaxed);
            self.shared.swaps_taken.fetch_add(1, Ordering::Release);
        }
    }

    /// Fills an interleaved device buffer of `channels` channels at the project rate.
    pub fn callback(&mut self, out: &mut [f32], channels: usize) {
        let t0 = Instant::now();
        self.handle_messages();
        let ch = channels.max(1);
        let frames = out.len() / ch;
        out.fill(0.0);
        // A new graph is installed while silent (now, or after the fade-out of this callback).
        if self.spare.is_some() && !self.running() {
            self.install_spare();
        }
        match self.state {
            State::Waiting(g) => {
                self.drain();
                if self.shared.ack_gen.load(Ordering::Acquire) == g {
                    self.drain();
                    self.shared.drained_gen.store(g, Ordering::Release);
                    self.state = State::Priming;
                }
            }
            State::Priming => {
                let need = frames.min(self.l.len()) + 64;
                let ready = self
                    .live
                    .as_ref()
                    .is_none_or(|l| l.consumers.iter().zip(&l.graph.streams).all(|(c, (_, k))| c.slots() >= need * k));
                if ready {
                    let mut lat = 0;
                    if let Some(l) = &mut self.live {
                        l.graph.reset(self.pos);
                        lat = l.graph.latency();
                    }
                    self.state = State::Running;
                    // The first `latency` frames out of a reset graph are silence: the fade-in starts after them,
                    // so the audio never enters half way up the ramp (FX-10: no click on a structural change).
                    self.fade_in = FADE + lat;
                }
            }
            State::Running => {}
        }
        self.shared.running.store(self.running(), Ordering::Relaxed);
        if self.running() {
            let fading_out = self.pending != Pending::NONE;
            let mut done = 0;
            let mut short = false;
            while done < frames {
                let mut n = (frames - done).min(self.l.len());
                if let Some((_, b)) = self.loop_region
                    && self.pos < b
                    && self.pos + n as i64 > b
                {
                    n = (b - self.pos) as usize;
                }
                let live = self.live.as_mut().expect("running");
                let mut feed = RingFeed { consumers: &mut live.consumers, short: false };
                live.graph.process(self.pos, &mut self.l[..n], &mut self.r[..n], &mut feed);
                short |= feed.short;
                if live.watch == Some(WATCH_BUS as usize) {
                    for (x, y) in self.l[..n].iter().zip(&self.r[..n]) {
                        let _ = self.spectrum.push(0.5 * (x + y));
                    }
                } else if let Some((a, b)) = live.watch.and_then(|w| live.graph.watched_output(w, n)) {
                    // A full ring drops samples: the spectrum only needs the latest frame.
                    for (x, y) in a.iter().zip(b) {
                        let _ = self.spectrum.push(0.5 * (x + y));
                    }
                }
                for i in 0..n {
                    let k = done + i;
                    let mut g = 1.0f32;
                    if self.fade_in > 0 {
                        g = (1.0 - self.fade_in as f32 / FADE as f32).max(0.0);
                        self.fade_in -= 1;
                    }
                    if fading_out {
                        g *= if k < FADE { 1.0 - (k + 1) as f32 / FADE as f32 } else { 0.0 };
                    }
                    let o = k * ch;
                    if ch == 1 {
                        out[o] = 0.5 * (self.l[i] + self.r[i]) * g;
                    } else {
                        out[o] = self.l[i] * g;
                        out[o + 1] = self.r[i] * g;
                    }
                }
                done += n;
                self.pos += n as i64;
                if let Some((a, b)) = self.loop_region
                    && self.pos == b
                {
                    // Same rule as the reader: wrap exactly at the loop end.
                    self.pos = a;
                }
            }
            self.shared.position.store(self.pos, Ordering::Relaxed);
            if fading_out {
                self.finish();
                if self.spare.is_some() {
                    self.install_spare();
                }
            } else if short {
                self.shared.xruns.fetch_add(1, Ordering::Relaxed);
                let p = self.pos;
                self.seek(p);
            }
        }
        self.since_meter += frames;
        if self.since_meter >= self.meter_every {
            self.since_meter = 0;
            self.push_meters();
        }
        let period = frames as f64 / self.rate;
        let load = t0.elapsed().as_secs_f64() / period.max(1e-9);
        self.shared.load_permille.store((load * 1000.0).min(4_000_000.0) as u32, Ordering::Relaxed);
    }

    /// True while audio is being produced (tests).
    pub fn is_running(&self) -> bool {
        self.running()
    }

    fn push_meters(&mut self) {
        let lat = self.live.as_ref().map_or(0, |l| l.graph.latency() as i64);
        let mut f = MeterFrame {
            position: (self.pos - lat).max(0),
            playing: self.shared.playing.load(Ordering::Relaxed),
            ..MeterFrame::default()
        };
        let finite = |v: f64| if v.is_finite() { v.max(-200.0) as f32 } else { -200.0 };
        if let Some(l) = &mut self.live {
            f.graph = l.id;
            let n = l.nodes.min(MAX_METERS).min(self.meter_tmp.len());
            l.graph.take_meters(&mut self.meter_tmp[..n], &mut self.bus_tmp);
            f.nodes = n;
            for (i, m) in self.meter_tmp[..n].iter().enumerate() {
                f.node[i] = [
                    finite(m.peak_db[0]),
                    finite(m.peak_db[1]),
                    finite(m.rms_db[0]),
                    finite(m.rms_db[1]),
                    finite(m.gr_db),
                ];
            }
            let b = &self.bus_tmp;
            f.bus = [
                finite(b.peak_db[0]),
                finite(b.peak_db[1]),
                finite(b.rms_db[0]),
                finite(b.rms_db[1]),
                finite(b.true_peak_db),
                finite(b.momentary_lufs),
                finite(b.short_term_lufs),
                b.correlation as f32,
                b.master_gr.blocks.iter().fold(0.0f64, |m, v| m.min(*v)) as f32,
                b.master_gr.limiter as f32,
            ];
            let g = &b.master_gr;
            for (o, v) in f.master.iter_mut().zip(g.blocks.iter().chain(g.bands.iter().flatten())) {
                *o = *v as f32;
            }
        }
        let _ = self.meters.push(f);
    }
}

/// Reduces FFT bins to log-spaced bands (the loudest bin of each band).
pub fn log_bands(bins: &[f64], rate: f64, fft: usize) -> Vec<f32> {
    let hz = rate / fft as f64;
    (0..SPECTRUM_BANDS)
        .map(|k| {
            let lo = 20.0 * 1000f64.powf(k as f64 / SPECTRUM_BANDS as f64);
            let hi = 20.0 * 1000f64.powf((k + 1) as f64 / SPECTRUM_BANDS as f64);
            let (a, b) = ((lo / hz).floor() as usize, ((hi / hz).ceil() as usize).max((lo / hz).floor() as usize + 1));
            bins[a.min(bins.len() - 1)..b.min(bins.len())].iter().copied().fold(-120.0, f64::max) as f32
        })
        .collect()
}

/// What the reader thread installs on a seek: the track streams and the ring producers of a graph.
pub struct Layout {
    streams: Vec<TrackStream>,
    producers: Vec<Producer<f32>>,
}

fn reader_loop(shared: Arc<Shared>, pending: Arc<Mutex<Option<Layout>>>) {
    let mut layout: Option<Layout> = None;
    let mut my_gen = 0u64;
    let mut pos = 0i64;
    let mut scratch: Vec<f32> = Vec::new();
    while !shared.stop.load(Ordering::Relaxed) {
        let g = shared.req_gen.load(Ordering::Acquire);
        if g != my_gen {
            my_gen = g;
            shared.ack_gen.store(g, Ordering::Release);
            // Wait (without a timeout) until the audio thread has drained what we pushed before, or a newer
            // request arrives, or the engine stops.
            while shared.drained_gen.load(Ordering::Acquire) != g
                && shared.req_gen.load(Ordering::Acquire) == g
                && !shared.stop.load(Ordering::Relaxed)
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            if shared.req_gen.load(Ordering::Acquire) != g || shared.stop.load(Ordering::Relaxed) {
                continue;
            }
            if let Some(l) = pending.lock().ok().and_then(|mut p| p.take()) {
                layout = Some(l);
            }
            pos = shared.req_pos.load(Ordering::Acquire);
            if let Some(l) = &mut layout {
                for s in &mut l.streams {
                    s.seek(pos);
                }
            }
        }
        let Some(l) = &mut layout else {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        };
        // Fill all rings by whole chunks, keeping the tracks in step.
        let room = l.producers.iter().zip(&l.streams).map(|(p, s)| p.slots() / s.channels()).min().unwrap_or(0);
        if room < CHUNK || l.streams.is_empty() {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }
        let mut n = CHUNK as i64;
        let looping = shared.loop_on.load(Ordering::Acquire);
        let (ls, le) = (shared.loop_start.load(Ordering::Acquire), shared.loop_end.load(Ordering::Acquire));
        if looping && pos < le && pos + n > le {
            n = le - pos;
        }
        for (s, p) in l.streams.iter_mut().zip(&mut l.producers) {
            let ch = s.channels();
            scratch.resize(n as usize * ch, 0.0);
            if s.read(&mut scratch).is_err() {
                scratch.fill(0.0);
            }
            if let Ok(mut chunk) = p.write_chunk(scratch.len()) {
                let (a, b) = chunk.as_mut_slices();
                let k = a.len();
                a.copy_from_slice(&scratch[..k]);
                b.copy_from_slice(&scratch[k..]);
                chunk.commit_all();
            }
        }
        pos += n;
        if looping && pos == le {
            // Same rule as the audio thread: wrap exactly at the loop end.
            pos = ls;
            for s in &mut l.streams {
                s.seek(pos);
            }
        }
    }
}

/// Control-side handle: owns the reader thread and talks to the audio processor.
pub struct Engine {
    pub shared: Arc<Shared>,
    tx: Producer<Msg>,
    trash: Consumer<Box<Live>>,
    meters: Consumer<MeterFrame>,
    spectrum_rx: Consumer<f32>,
    master_tx: Producer<(u64, crate::master::MasterParams)>,
    /// Sequence of [`Self::set_master`] calls: a graph remembers the one it was built with.
    master_seq: u64,
    spectrum_buf: std::collections::VecDeque<f32>,
    analyser: stemwise_dsp::meters::Spectrum,
    watched: Option<String>,
    pending: Arc<Mutex<Option<Layout>>>,
    reader: Option<JoinHandle<()>>,
    pub rate: f64,
    /// Node ids of each graph sent, by graph id (meters are matched to the graph that produced them).
    node_ids: Vec<(u64, Vec<String>)>,
    /// Parameter slots of the newest graph's nodes (real-time changes are resolved against them).
    slots: Vec<std::collections::HashMap<String, usize>>,
    next_graph: u64,
    swaps_sent: u64,
    /// A swap waiting for the previous one to be taken.
    queued: Option<(Box<Live>, Layout)>,
    last: MeterFrame,
    /// Explicit locate not yet followed by play; where play started (stop returns there, TRN-1).
    cursor: Option<i64>,
    play_start: i64,
    solo: Option<SoloMask>,
    /// Helper threads shared by every graph of this engine (A-024).
    pool: Option<Arc<crate::pool::Pool>>,
}

impl Engine {
    /// Creates the engine and the audio processor to hand to the output (cpal or a test driver).
    pub fn new(rate: f64, max_block: usize) -> (Engine, AudioProcessor) {
        let shared = Arc::new(Shared::default());
        let (tx, rx) = RingBuffer::<Msg>::new(4096);
        let (trash_tx, trash) = RingBuffer::<Box<Live>>::new(4);
        let (meter_tx, meters) = RingBuffer::<MeterFrame>::new(8);
        let (spectrum_tx, spectrum_rx) = RingBuffer::<f32>::new(SPECTRUM_FFT * 4);
        let (master_tx, master_rx) = RingBuffer::<(u64, crate::master::MasterParams)>::new(16);
        let pending = Arc::new(Mutex::new(None));
        let reader = {
            let (s, p) = (shared.clone(), pending.clone());
            std::thread::Builder::new().name("stemwise-reader".into()).spawn(move || reader_loop(s, p)).ok()
        };
        let ap = AudioProcessor {
            shared: shared.clone(),
            live: None,
            spare: None,
            rx,
            trash: trash_tx,
            meters: meter_tx,
            spectrum: spectrum_tx,
            master_rx,
            last_master: None,
            state: State::Running,
            generation: 0,
            pos: 0,
            loop_region: None,
            l: vec![0.0; max_block],
            r: vec![0.0; max_block],
            meter_tmp: vec![NodeMeter::default(); MAX_METERS],
            bus_tmp: BusMeter::default(),
            since_meter: 0,
            meter_every: (rate / 50.0) as usize,
            rate,
            fade_in: 0,
            pending: Pending::NONE,
        };
        let e = Engine {
            shared,
            tx,
            trash,
            meters,
            spectrum_rx,
            master_tx,
            master_seq: 0,
            spectrum_buf: std::collections::VecDeque::with_capacity(SPECTRUM_FFT),
            analyser: stemwise_dsp::meters::Spectrum::new(SPECTRUM_FFT),
            watched: None,
            pending,
            reader,
            rate,
            node_ids: Vec::new(),
            slots: Vec::new(),
            next_graph: 1,
            swaps_sent: 0,
            queued: None,
            last: MeterFrame::default(),
            cursor: None,
            play_start: 0,
            solo: None,
            pool: match crate::pool::Pool::default_helpers() {
                0 => None,
                k => Some(crate::pool::Pool::new(k)),
            },
        };
        (e, ap)
    }

    fn send(&mut self, m: Msg) {
        // A large queue drained every callback; if the audio side is stalled a message is dropped rather than
        // blocking the interface (the document keeps the value; the next change resends it).
        let _ = self.tx.push(m);
    }

    /// Sends the queued swap when the previous one has been taken.
    fn flush_swap(&mut self) {
        while let Ok(old) = self.trash.pop() {
            drop(old);
        }
        if self.shared.swaps_taken.load(Ordering::Acquire) < self.swaps_sent {
            return;
        }
        if let Some((live, layout)) = self.queued.take() {
            if let Ok(mut p) = self.pending.lock() {
                *p = Some(layout);
            }
            match self.tx.push(Msg::Swap(live)) {
                Ok(()) => self.swaps_sent += 1,
                Err(rtrb::PushError::Full(Msg::Swap(live))) => {
                    let layout = self.pending.lock().ok().and_then(|mut p| p.take());
                    if let Some(layout) = layout {
                        self.queued = Some((live, layout));
                    }
                }
                Err(_) => {}
            }
        }
    }

    /// Builds a graph for the project and hands it to the audio thread (structural change, FOUNDATION §5.3).
    pub fn load(&mut self, package: &Path, project: &Project) -> Result<(), crate::offline::RenderError> {
        self.load_with(package, project, None)
    }

    /// As [`Self::load`], with a master chain on the bus (the master phase plays the mixdown through it).
    pub fn load_with(
        &mut self,
        package: &Path,
        project: &Project,
        master: Option<crate::master::MasterParams>,
    ) -> Result<(), crate::offline::RenderError> {
        let mut graph = Graph::build(project, self.rate)?;
        graph.set_master_chain(master.map(|p| crate::master::MasterProc::new(self.rate, p)));
        graph.pool = self.pool.clone();
        let mut streams = Vec::new();
        let mut producers = Vec::new();
        let mut consumers = Vec::new();
        for (id, (_, ch)) in graph.stream_tracks.iter().zip(&graph.streams) {
            let t = project.track(id).expect("built from this project");
            streams.push(TrackStream::open(package, project, t)?);
            let (p, c) = RingBuffer::<f32>::new((self.rate * RING_SECONDS) as usize * ch);
            producers.push(p);
            consumers.push(c);
        }
        let ids: Vec<String> = graph.meter_slots().into_iter().map(|m| m.id).collect();
        let mask = solo_mask(project, &ids);
        graph.set_solo(&mask);
        self.solo = Some(mask);
        let id = self.next_graph;
        self.next_graph += 1;
        let nodes = ids.len();
        self.slots = graph.param_slots();
        self.node_ids.push((id, ids));
        if self.node_ids.len() > 3 {
            self.node_ids.remove(0);
        }
        let watch = match self.watched.as_deref() {
            Some(BUS_ID) => Some(WATCH_BUS as usize),
            Some(w) => self.current_ids().iter().position(|n| n == w),
            None => None,
        };
        // A newer graph replaces one still waiting to be sent.
        self.queued = Some((
            Box::new(Live { graph, consumers, nodes, id, watch, master_seq: self.master_seq }),
            Layout { streams, producers },
        ));
        self.flush_swap();
        Ok(())
    }

    /// Chooses the track whose output feeds the spectrum (FX-7); `None` stops it.
    pub fn watch(&mut self, track: Option<&str>) {
        self.watched = track.map(str::to_string);
        let idx = match track {
            Some(BUS_ID) => Some(WATCH_BUS as usize),
            Some(t) => self.current_ids().iter().position(|n| n == t),
            None => None,
        };
        if let Some((live, _)) = &mut self.queued {
            live.watch = idx;
        }
        self.spectrum_buf.clear();
        self.send(Msg::Watch(idx.map(|i| i as u32)));
    }

    /// New master parameters, glided by the master processor (no rebuild). Numbered, so a graph built or swapped
    /// in meanwhile ends up with the newest ones whatever the order of arrival.
    pub fn set_master(&mut self, p: crate::master::MasterParams) {
        self.master_seq += 1;
        if let Some((live, _)) = &mut self.queued {
            live.graph.set_master_params(p);
            live.master_seq = self.master_seq;
        }
        // A full queue (audio stalled) drops the value; the next change resends the whole state.
        let _ = self.master_tx.push((self.master_seq, p));
    }

    /// The watched track's spectrum in [`SPECTRUM_BANDS`] log-spaced bands from 20 Hz to 20 kHz, dBFS (smoothed).
    pub fn spectrum(&mut self) -> Option<Vec<f32>> {
        while let Ok(v) = self.spectrum_rx.pop() {
            if self.spectrum_buf.len() == SPECTRUM_FFT {
                self.spectrum_buf.pop_front();
            }
            self.spectrum_buf.push_back(v);
        }
        self.watched.as_ref()?;
        let samples: Vec<f32> = self.spectrum_buf.iter().copied().collect();
        let bins = self.analyser.analyse(&samples, 0.6);
        Some(log_bands(bins, self.rate, SPECTRUM_FFT))
    }

    fn current_ids(&self) -> &[String] {
        self.node_ids.last().map_or(&[], |(_, v)| v.as_slice())
    }

    /// A parameter change (ENG-3): reaches the graph within one buffer, smoothed there.
    pub fn set_param(&mut self, track: &str, param: &str, value: f64) {
        let ids = self.current_ids();
        let Some(node) = ids.iter().position(|n| n == track) else { return };
        let target = if let Some(ret) = param.strip_prefix("send.") {
            match ids.iter().position(|n| n == ret) {
                Some(r) => ParamTarget::Send(r as u32),
                None => return,
            }
        } else {
            match self.slots.get(node).and_then(|m| m.get(param)) {
                Some(&s) => ParamTarget::Slot(s as u32),
                None => return,
            }
        };
        self.send(Msg::Param { graph: self.next_graph - 1, track: node as u32, slot: target, value });
    }

    /// Solo changes (TRK-2): recomputed from the project, applied without rebuilding the graph.
    pub fn set_solo(&mut self, project: &Project) {
        let mask = solo_mask(project, self.current_ids());
        if self.solo != Some(mask) {
            self.solo = Some(mask);
            self.send(Msg::Solo(mask));
        }
    }

    pub fn set_bus_fader(&mut self, db: f64) {
        self.send(Msg::BusFader(db));
    }

    pub fn play(&mut self) {
        if self.playing() {
            return;
        }
        let start = self.cursor.take().unwrap_or_else(|| self.position());
        self.play_start = start;
        self.send(Msg::Locate(start));
        self.send(Msg::Play);
    }

    pub fn pause(&mut self) {
        self.cursor = Some(self.position());
        self.send(Msg::Pause);
    }

    /// Stops and returns to where playback started (TRN-1).
    pub fn stop(&mut self) {
        self.send(Msg::Pause);
        let s = self.play_start;
        self.locate(s);
    }

    pub fn locate(&mut self, frame: i64) {
        let f = frame.max(0);
        self.cursor = Some(f);
        self.send(Msg::Locate(f));
    }

    pub fn set_loop(&mut self, region: Option<(i64, i64)>) {
        self.send(Msg::Loop(region));
    }

    pub fn playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    /// Timeline position of what is heard (the graph latency is removed).
    pub fn position(&self) -> i64 {
        if !self.playing()
            && let Some(c) = self.cursor
        {
            return c;
        }
        (self.shared.position.load(Ordering::Relaxed) - self.shared.latency.load(Ordering::Relaxed)).max(0)
    }

    pub fn load_ratio(&self) -> f64 {
        f64::from(self.shared.load_permille.load(Ordering::Relaxed)) / 1000.0
    }

    /// Latest meter frame and the node ids of the graph that produced it; also sends a queued swap and frees
    /// graphs the audio thread gave back.
    pub fn poll(&mut self) -> (MeterFrame, &[String]) {
        while let Ok(f) = self.meters.pop() {
            self.last = f;
        }
        self.flush_swap();
        let ids = self.node_ids.iter().find(|(g, _)| *g == self.last.graph).map_or(&[][..], |(_, v)| v.as_slice());
        (self.last, ids)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
    }
}
