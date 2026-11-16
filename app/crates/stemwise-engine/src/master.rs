//! The master chain (MAS-1 … MAS-3, MAS-5, MAS-8, MAS-11 … MAS-15): input gain → the free blocks in the order the
//! project gives them (EQ, match EQ, compressor, multiband, width, saturation, clipper; some of them twice) →
//! loudness gain → true-peak limiter. The same processor plays in the master phase (no allocation once built:
//! parameters arrive as a `Copy` value) and renders offline for the export.
//!
//! Loudness: the chain is normalised to the destination (MAS-2) by a gain before the limiter, found offline by
//! rendering and measuring (deterministic: same mixdown and same plan ⇒ same gain, MAS-8). A clipper takes a number
//! of dB off the tallest peak that reaches it (MAS-11): that peak is measured offline too, before the search.
//! A/B at equal loudness (MAS-5): the unprocessed mixdown (or a reference) plays with the gain that brings it to the
//! master's loudness.
//!
//! Real time: every block the chain may ever hold exists from the start ([`PER_KIND`] of each kind). A change of
//! parameters glides; a change of structure (blocks added, removed or moved) dips the output to silence for a few
//! milliseconds, swaps and comes back, so nothing clicks. The latency is the same for every chain: what the blocks
//! in use do not take, a delay line does.

use stemwise_core::model::{Block, BlockKind, EqKind, MasterState, Section, master_limits};
use stemwise_dsp::biquad::{Biquad, Coeffs, Kind};
use stemwise_dsp::clipper::{Clipper, ClipperParams};
use stemwise_dsp::compressor::{Compressor, CompressorParams};
use stemwise_dsp::limiter::Limiter;
use stemwise_dsp::loudness;
use stemwise_dsp::master::{MatchEq, Width, match_gains};
use stemwise_dsp::multiband::{BandParams, MAX_BANDS, Multiband, MultibandParams};
use stemwise_dsp::saturation::{Saturation, SaturationParams};
use stemwise_dsp::util::db_to_gain;

/// Number of parametric bands of a master EQ (MAS-1).
pub const MASTER_BANDS: usize = 6;
/// Free blocks a chain may hold (MAS-15).
pub const MAX_FREE: usize = master_limits::FREE_BLOCKS;
/// Blocks of one kind a chain may hold.
pub const PER_KIND: usize = master_limits::PER_KIND;
/// Sections with offsets the processor follows (MAS-14); further ones are ignored.
pub const MAX_SECTIONS: usize = 32;

/// (enabled, kind, frequency, gain, Q) per band.
pub type Bands = [(bool, Kind, f64, f64, f64); MASTER_BANDS];

/// The parameters of one free block, as a plain value. The variants differ a lot in size, and stay inline: a boxed
/// one would allocate, and these travel to the audio thread by value.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum BlockParams {
    None,
    Eq { on: bool, bands: Bands },
    Match { on: bool, gains: [f64; 8] },
    Comp { on: bool, p: CompressorParams },
    Multiband { on: bool, p: MultibandParams },
    Width { on: bool, side_gain_db: f64, mono_below_hz: f64 },
    Sat { on: bool, p: SaturationParams },
    Clip { on: bool, take_off_db: f64, softness: f64 },
}

impl BlockParams {
    /// A small number per kind (0 = none), for telling two structures apart.
    fn tag(&self) -> u8 {
        match self {
            BlockParams::None => 0,
            BlockParams::Eq { .. } => 1,
            BlockParams::Match { .. } => 2,
            BlockParams::Comp { .. } => 3,
            BlockParams::Multiband { .. } => 4,
            BlockParams::Width { .. } => 5,
            BlockParams::Sat { .. } => 6,
            BlockParams::Clip { .. } => 7,
        }
    }
}

/// A place in the chain: which of the processors of its kind runs there, and with what.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub inst: u8,
    pub block: BlockParams,
}

const EMPTY: Slot = Slot { inst: 0, block: BlockParams::None };

/// Offsets of a section (MAS-14), in frames of the timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionParams {
    pub start: i64,
    pub end: i64,
    pub level_db: f64,
    pub side_gain_db: f64,
    pub release_ms: f64,
    /// Gain offsets of the bands of each EQ processor.
    pub eq: [[f64; MASTER_BANDS]; PER_KIND],
}

const NO_SECTION: SectionParams = SectionParams {
    start: 0,
    end: 0,
    level_db: 0.0,
    side_gain_db: 0.0,
    release_ms: 0.0,
    eq: [[0.0; MASTER_BANDS]; PER_KIND],
};

/// Everything the processor needs, as a plain value: sent to the audio thread without allocating.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MasterParams {
    pub input_gain_db: f64,
    /// The free blocks in processing order; the first `None` ends the chain.
    pub blocks: [Slot; MAX_FREE],
    /// Linear peak at the input of each clipper processor over the whole song ([`clip_peaks`]); 0 = not measured
    /// yet: that clipper lets everything through.
    pub clip_peak: [f64; PER_KIND],
    pub sections: [SectionParams; MAX_SECTIONS],
    pub n_sections: usize,
    /// Normalisation towards the destination (found offline, [`loudness_gain`]).
    pub loud_gain_db: f64,
    pub limiter_on: bool,
    pub ceiling_db: f64,
    pub release_ms: f64,
    /// A/B (MAS-5): play the input unprocessed, with `bypass_gain_db` to match the master's loudness.
    pub bypass: bool,
    pub bypass_gain_db: f64,
    /// Listening level after everything (the A/B trim that keeps equal loudness under the ceiling, MAS-5); 0 in
    /// renders and exports.
    pub out_gain_db: f64,
}

fn kind(k: EqKind) -> Kind {
    match k {
        EqKind::Peak => Kind::Peak,
        EqKind::LowShelf => Kind::LowShelf,
        EqKind::HighShelf => Kind::HighShelf,
    }
}

/// Which processor of its kind each block of a chain uses: blocks of one kind in the order of their ids, so that
/// moving a block does not hand its state to another.
pub fn instances(m: &MasterState) -> Vec<(String, u8)> {
    let mut out = Vec::new();
    for k in BlockKind::ALL {
        let mut ids: Vec<&str> = m.chain.blocks.iter().filter(|b| b.block.kind() == k).map(|b| b.id.as_str()).collect();
        ids.sort_unstable();
        out.extend(ids.into_iter().enumerate().map(|(i, id)| (id.to_string(), i as u8)));
    }
    out
}

impl MasterParams {
    /// The parameters of a master state. `mix_bands` and `target_bands` (8-band profiles of the mixdown and of the
    /// references) drive the match EQ; without both it does nothing. Section offsets and clipper peaks are added
    /// with [`Self::with_sections`] and `clip_peak`.
    pub fn of(
        m: &MasterState,
        mix_bands: Option<&[f64; 8]>,
        target_bands: Option<&[f64; 8]>,
        loud_gain_db: f64,
    ) -> Self {
        let c = &m.chain;
        let inst = instances(m);
        let mut blocks = [EMPTY; MAX_FREE];
        for (slot, b) in blocks.iter_mut().zip(&c.blocks) {
            let on = !b.block.bypass();
            let params = match &b.block {
                Block::Eq(eq) => {
                    let mut bands = [(false, Kind::Peak, 1000.0, 0.0, 1.0); MASTER_BANDS];
                    for (s, band) in bands.iter_mut().zip(&eq.bands) {
                        *s = (band.enabled, kind(band.kind), band.freq_hz, band.gain_db, band.q);
                    }
                    BlockParams::Eq { on, bands }
                }
                Block::MatchEq(x) => {
                    let gains = match (mix_bands, target_bands) {
                        (Some(mb), Some(tb)) if on => match_gains(mb, tb, x.amount),
                        _ => [0.0; 8],
                    };
                    BlockParams::Match { on: on && gains.iter().any(|g| *g != 0.0), gains }
                }
                Block::Compressor(k) => BlockParams::Comp {
                    on,
                    p: CompressorParams {
                        threshold_db: k.threshold_db,
                        ratio: k.ratio,
                        attack_ms: k.attack_ms,
                        release_ms: k.release_ms,
                        knee_db: k.knee_db,
                        makeup_db: k.makeup_db,
                        sidechain_hpf_hz: 100.0,
                        detector_hpf_hz: k.listen_above_hz,
                        mix: k.mix,
                    },
                },
                Block::Multiband(x) => {
                    let mut p =
                        MultibandParams { bands: x.bands.len().clamp(2, MAX_BANDS), ..MultibandParams::default() };
                    for (s, f) in p.xo_hz.iter_mut().zip(&x.crossovers_hz) {
                        *s = *f;
                    }
                    for (s, band) in p.band.iter_mut().zip(&x.bands) {
                        *s = BandParams {
                            on: !band.bypass,
                            solo: false,
                            comp: CompressorParams {
                                threshold_db: band.threshold_db,
                                ratio: band.ratio,
                                attack_ms: band.attack_ms,
                                release_ms: band.release_ms,
                                knee_db: 6.0,
                                makeup_db: band.gain_db,
                                sidechain_hpf_hz: 100.0,
                                detector_hpf_hz: 0.0,
                                mix: 1.0,
                            },
                        };
                    }
                    BlockParams::Multiband { on, p }
                }
                Block::Width(w) => {
                    BlockParams::Width { on, side_gain_db: w.side_gain_db, mono_below_hz: w.mono_below_hz }
                }
                Block::Saturation(x) => BlockParams::Sat {
                    on,
                    p: SaturationParams { drive_db: x.drive_db, mix: x.mix, character: Default::default() },
                },
                Block::Clipper(k) => BlockParams::Clip { on, take_off_db: k.take_off_db, softness: k.softness },
            };
            let i = inst.iter().find(|(id, _)| *id == b.id).map_or(0, |(_, i)| *i);
            *slot = Slot { inst: i.min(PER_KIND as u8 - 1), block: params };
        }
        MasterParams {
            input_gain_db: c.input_gain_db,
            blocks,
            clip_peak: [0.0; PER_KIND],
            sections: [NO_SECTION; MAX_SECTIONS],
            n_sections: 0,
            loud_gain_db,
            // The limiter always runs in the export (MAS-3); its bypass only affects listening.
            limiter_on: !c.limiter.bypass,
            ceiling_db: m.target.true_peak_db,
            release_ms: c.limiter.release_ms,
            bypass: false,
            bypass_gain_db: 0.0,
            out_gain_db: 0.0,
        }
    }

    /// Adds the offsets of the sections (MAS-14): `sections` are the project's, which say where each one lies.
    pub fn with_sections(mut self, m: &MasterState, sections: &[Section]) -> Self {
        let inst = instances(m);
        self.n_sections = 0;
        for o in &m.sections {
            let Some(s) = sections.iter().find(|s| s.id == o.section) else { continue };
            if o.is_empty() || self.n_sections == MAX_SECTIONS {
                continue;
            }
            let mut eq = [[0.0; MASTER_BANDS]; PER_KIND];
            for e in &o.eq {
                if let Some((_, i)) = inst.iter().find(|(id, _)| *id == e.block) {
                    for (g, v) in eq[usize::from(*i).min(PER_KIND - 1)].iter_mut().zip(&e.gains_db) {
                        *g = *v;
                    }
                }
            }
            self.sections[self.n_sections] = SectionParams {
                start: s.start,
                end: s.end,
                level_db: o.level_db,
                side_gain_db: o.side_gain_db,
                release_ms: o.limiter_release_ms,
                eq,
            };
            self.n_sections += 1;
        }
        self
    }

    /// Solo of a multiband's bands (listening only): `mask` has a bit per band.
    pub fn with_solo(mut self, inst: u8, mask: u8) -> Self {
        for s in &mut self.blocks {
            if let (BlockParams::Multiband { p, .. }, true) = (&mut s.block, s.inst == inst) {
                for (i, b) in p.band.iter_mut().enumerate() {
                    b.solo = mask & (1 << i) != 0;
                }
            }
        }
        self
    }

    /// What tells two chains with different structure apart: kinds and processors, in order.
    fn structure(&self) -> [(u8, u8); MAX_FREE] {
        let mut s = [(0, 0); MAX_FREE];
        for (o, b) in s.iter_mut().zip(&self.blocks) {
            *o = (b.block.tag(), b.inst);
        }
        s
    }

    /// The offsets at a position of the timeline.
    fn section_at(&self, pos: i64) -> &SectionParams {
        self.sections[..self.n_sections].iter().find(|s| s.start <= pos && pos < s.end).unwrap_or(&NO_SECTION)
    }

    /// Latency taken by the oversampled blocks of this chain.
    fn used_latency(&self) -> usize {
        self.blocks
            .iter()
            .map(|s| match s.block {
                BlockParams::Sat { .. } => Saturation::LATENCY,
                BlockParams::Clip { .. } => Clipper::LATENCY,
                _ => 0,
            })
            .sum()
    }
}

/// Latency reserved for the oversampled blocks: the most a chain can hold.
const FREE_LATENCY: usize = PER_KIND * (Saturation::LATENCY + Clipper::LATENCY);

/// The master processor. Built once (allocates), then `set` and `process` never allocate.
///
/// Changes never step inside the signal (ENG-3, "no clicks"): continuous values (gains, EQ bands, match EQ, width)
/// glide towards their targets every [`SUB`] samples with a 10 ms time constant; switching a block on or off, and
/// the A/B, cross-fade between its input and its output over the same time; section offsets glide more slowly (about
/// a tenth of a second). A switched-off filter glides to the identity, then stops running; saturation, clipper and
/// limiter keep running when off, and the delay lines that stand in for their latency always run, so a cross-fade
/// never plays stale or missing audio. With constant parameters (the offline render) the glide is already at its
/// target. A new structure waits while the output dips to silence, then takes over with clean state.
pub struct MasterProc {
    rate: f64,
    p: MasterParams,
    /// A chain with another structure, waiting for the dip.
    pending: Option<MasterParams>,
    /// Output gain of the dip (1 = none), and the sub-blocks it still stays at silence after the swap.
    duck: f64,
    hold: usize,
    /// One-pole coefficients per sub-block: values, and section offsets.
    glide: f64,
    glide_slow: f64,
    /// Position on the timeline of the next sample.
    pos: i64,
    cur: Cur,
    eq: [[Biquad; MASTER_BANDS]; PER_KIND],
    match_eq: MatchEq,
    comp: [Compressor; PER_KIND],
    mb: [Multiband; PER_KIND],
    width: Width,
    sat: [Saturation; PER_KIND],
    clip: [Clipper; PER_KIND],
    limiter: Limiter,
    /// Delay of the bypassed path, so A/B switches keep time with the processed path.
    dry: Delay,
    /// Dry paths of the oversampled blocks and of the limiter: same latency whether they are on or off.
    sat_delay: [Delay; PER_KIND],
    clip_delay: [Delay; PER_KIND],
    lim_delay: Delay,
    /// Stands in for the oversampled blocks the chain does not hold.
    pad: Delay,
    /// Highest level seen at the input of each clipper since [`Self::take_clip_in`].
    clip_in: [f32; PER_KIND],
}

/// Samples between two updates of the glides (as the automation grid of the mix, ENG-3).
const SUB: usize = 32;
const GLIDE_S: f64 = 0.010;
const GLIDE_SLOW_S: f64 = 0.025;
/// Sub-blocks a dip takes to reach silence, and to come back.
const DUCK_STEPS: f64 = 8.0;

/// Where the glides are now.
#[derive(Clone, Copy)]
struct Cur {
    input_db: f64,
    loud_db: f64,
    bypass_db: f64,
    out_db: f64,
    /// (kind, frequency, gain, Q) per band; a disabled band glides to 0 dB, a kind change goes through 0 dB.
    eq: [[(Kind, f64, f64, f64); MASTER_BANDS]; PER_KIND],
    eq_live: [bool; PER_KIND],
    match_gains: [f64; 8],
    match_live: bool,
    side_db: f64,
    mono_hz: f64,
    width_on: bool,
    width_live: bool,
    /// Cross-fades: 0 = block off (input passes), 1 = block on.
    comp: [f64; PER_KIND],
    mb: [f64; PER_KIND],
    sat: [f64; PER_KIND],
    clip: [f64; PER_KIND],
    lim: f64,
    /// 1 = A/B on the unprocessed input.
    bypass: f64,
    /// Section offsets.
    off_level: f64,
    off_side: f64,
    off_release: f64,
    off_eq: [[f64; MASTER_BANDS]; PER_KIND],
}

fn approach(cur: f64, target: f64, a: f64, eps: f64) -> f64 {
    let v = target + (cur - target) * a;
    if (v - target).abs() < eps { target } else { v }
}

fn approach_log(cur: f64, target: f64, a: f64) -> f64 {
    if cur <= 0.0 || target <= 0.0 {
        return target;
    }
    let v = (target.ln() + (cur.ln() - target.ln()) * a).exp();
    if (v / target - 1.0).abs() < 1e-4 { target } else { v }
}

/// Per-sample ramp of a gain over a sub-block: from `a` (start) to `b` (end).
fn ramp(buf: &mut [f32], a: f64, b: f64) {
    if a == b {
        if a != 1.0 {
            let g = a as f32;
            for v in buf.iter_mut() {
                *v *= g;
            }
        }
        return;
    }
    let n = buf.len() as f64;
    for (i, v) in buf.iter_mut().enumerate() {
        *v *= (a + (b - a) * (i as f64 + 1.0) / n) as f32;
    }
}

/// `wet` ← `dry`·(1 − m) + `wet`·m, with m ramping from `a` to `b`.
fn mix(wet: &mut [f32], dry: &[f32], a: f64, b: f64) {
    let n = wet.len() as f64;
    for (i, (w, d)) in wet.iter_mut().zip(dry).enumerate() {
        let m = (a + (b - a) * (i as f64 + 1.0) / n) as f32;
        *w = *d + (*w - *d) * m;
    }
}

impl Cur {
    fn target(p: &MasterParams, pos: i64) -> Self {
        let on = |b: bool| if b { 1.0 } else { 0.0 };
        let s = p.section_at(pos);
        let mut c = Cur {
            input_db: p.input_gain_db,
            loud_db: p.loud_gain_db,
            bypass_db: p.bypass_gain_db,
            out_db: p.out_gain_db,
            eq: [[(Kind::Peak, 1000.0, 0.0, 1.0); MASTER_BANDS]; PER_KIND],
            eq_live: [false; PER_KIND],
            match_gains: [0.0; 8],
            match_live: false,
            side_db: 0.0,
            mono_hz: 0.0,
            width_on: false,
            width_live: false,
            comp: [0.0; PER_KIND],
            mb: [0.0; PER_KIND],
            sat: [0.0; PER_KIND],
            clip: [0.0; PER_KIND],
            lim: on(p.limiter_on),
            bypass: on(p.bypass),
            off_level: s.level_db,
            off_side: 0.0,
            off_release: s.release_ms,
            off_eq: [[0.0; MASTER_BANDS]; PER_KIND],
        };
        for slot in &p.blocks {
            let i = usize::from(slot.inst);
            match slot.block {
                BlockParams::None => break,
                BlockParams::Eq { on: block_on, bands } => {
                    for (b, (enabled, k, f, g, q)) in bands.into_iter().enumerate() {
                        let live = enabled && block_on;
                        c.eq[i][b] = (k, f, if live { g } else { 0.0 }, q);
                        // An offset moves a band that is in use; it does not switch one on.
                        c.off_eq[i][b] = if live { s.eq[i][b] } else { 0.0 };
                    }
                }
                BlockParams::Match { on: block_on, gains } => {
                    if block_on {
                        c.match_gains = gains;
                    }
                }
                BlockParams::Comp { on: block_on, .. } => c.comp[i] = on(block_on),
                BlockParams::Multiband { on: block_on, .. } => c.mb[i] = on(block_on),
                BlockParams::Width { on: block_on, side_gain_db, mono_below_hz } => {
                    if block_on {
                        c.side_db = side_gain_db;
                        c.mono_hz = mono_below_hz;
                        c.width_on = true;
                        c.off_side = s.side_gain_db;
                    }
                }
                BlockParams::Sat { on: block_on, .. } => c.sat[i] = on(block_on),
                BlockParams::Clip { on: block_on, .. } => c.clip[i] = on(block_on && p.clip_peak[i] > 0.0),
            }
        }
        c.live();
        c
    }

    /// Which filters have something to do where the glides are now.
    fn live(&mut self) {
        for i in 0..PER_KIND {
            self.eq_live[i] = (0..MASTER_BANDS).any(|b| self.eq[i][b].2 != 0.0 || self.off_eq[i][b] != 0.0);
        }
        self.match_live = self.match_gains.iter().any(|g| *g != 0.0);
        self.width_live = self.side_db != 0.0 || self.off_side != 0.0 || self.mono_hz >= 1.0;
    }
}

impl MasterProc {
    pub fn new(rate: f64, p: MasterParams) -> Self {
        let limiter = Limiter::new(rate);
        let lat = FREE_LATENCY + limiter.latency();
        let mut m = MasterProc {
            rate,
            p,
            pending: None,
            duck: 1.0,
            hold: 0,
            glide: (-(SUB as f64) / (GLIDE_S * rate)).exp(),
            glide_slow: (-(SUB as f64) / (GLIDE_SLOW_S * rate)).exp(),
            pos: 0,
            cur: Cur::target(&p, 0),
            eq: std::array::from_fn(|_| std::array::from_fn(|_| Biquad::default())),
            match_eq: MatchEq::new(rate),
            comp: std::array::from_fn(|_| Compressor::new(rate)),
            mb: std::array::from_fn(|_| Multiband::new(rate)),
            width: Width::new(rate),
            sat: std::array::from_fn(|_| Saturation::new(rate)),
            clip: std::array::from_fn(|_| Clipper::new(rate)),
            dry: Delay::new(lat, lat),
            sat_delay: std::array::from_fn(|_| Delay::new(Saturation::LATENCY, Saturation::LATENCY)),
            clip_delay: std::array::from_fn(|_| Delay::new(Clipper::LATENCY, Clipper::LATENCY)),
            lim_delay: Delay::new(limiter.latency(), limiter.latency()),
            pad: Delay::new(FREE_LATENCY, FREE_LATENCY - p.used_latency()),
            limiter,
            clip_in: [0.0; PER_KIND],
        };
        m.apply(p);
        m.reset(0);
        m
    }

    /// Samples by which the output lags the input: the same for every chain.
    pub fn latency(&self) -> usize {
        FREE_LATENCY + self.limiter.latency()
    }

    pub fn params(&self) -> &MasterParams {
        self.pending.as_ref().unwrap_or(&self.p)
    }

    /// New parameters, without allocating (audio thread): the glides start towards them. A chain with another
    /// structure takes over after a short dip to silence.
    pub fn set(&mut self, p: MasterParams) {
        if p.structure() == self.p.structure() {
            self.pending = None;
            self.apply(p);
        } else {
            self.pending = Some(p);
        }
    }

    /// Hands the parameters to the processors that take them whole.
    fn apply(&mut self, p: MasterParams) {
        for slot in &p.blocks {
            let i = usize::from(slot.inst);
            match &slot.block {
                BlockParams::Comp { p: c, .. } => self.comp[i].set_params(c),
                BlockParams::Multiband { p: m, .. } => self.mb[i].set_params(m),
                BlockParams::Sat { p: s, .. } => self.sat[i].set_params(s),
                BlockParams::Clip { take_off_db, softness, .. } => {
                    let peak = p.clip_peak[i];
                    // Not measured yet: a threshold nothing reaches.
                    let threshold = if peak > 0.0 { peak * db_to_gain(-take_off_db) } else { 1.0e6 };
                    self.clip[i].set_params(&ClipperParams { threshold, softness: *softness });
                }
                _ => {}
            }
        }
        self.p = p;
        self.limiter.set_params(p.ceiling_db, (p.release_ms + self.cur.off_release).max(1.0));
    }

    /// The filters for where the glides are now.
    fn design(&mut self) {
        let c = &self.cur;
        for i in 0..PER_KIND {
            for (b, f) in self.eq[i].iter_mut().enumerate() {
                let (k, freq, g, q) = c.eq[i][b];
                let g = g + c.off_eq[i][b];
                f.c = if g != 0.0 {
                    Coeffs::design(k, freq.min(self.rate * 0.45), g, q, self.rate)
                } else {
                    Coeffs::IDENTITY
                };
            }
        }
        self.match_eq.set_gains(&c.match_gains);
        self.width.set_params(c.side_db + c.off_side, if c.mono_hz >= 1.0 { c.mono_hz } else { 0.0 });
    }

    /// One glide step towards the parameters (every [`SUB`] samples).
    fn step(&mut self) {
        // A new structure: down to silence, swap with clean state, back up.
        if self.pending.is_some() {
            self.duck = (self.duck - 1.0 / DUCK_STEPS).max(0.0);
            if self.duck == 0.0 {
                let p = self.pending.take().expect("checked");
                self.pad.set_len(FREE_LATENCY - p.used_latency());
                self.apply(p);
                self.cur = Cur::target(&self.p, self.pos);
                self.design();
                self.reset_blocks();
                // Silence until what the old chain left in the delay lines and in the limiter has gone by.
                self.hold = self.latency().div_ceil(SUB) + 1;
            }
        } else if self.hold > 0 {
            self.hold -= 1;
        } else if self.duck < 1.0 {
            self.duck = (self.duck + 1.0 / DUCK_STEPS).min(1.0);
        }
        let t = Cur::target(&self.p, self.pos);
        let (a, slow) = (self.glide, self.glide_slow);
        let c = &mut self.cur;
        let before = (c.eq, c.off_eq, c.match_gains, c.side_db, c.off_side, c.mono_hz);
        let release_before = c.off_release;
        c.input_db = approach(c.input_db, t.input_db, a, 1e-4);
        c.loud_db = approach(c.loud_db, t.loud_db, a, 1e-4);
        c.bypass_db = approach(c.bypass_db, t.bypass_db, a, 1e-4);
        c.out_db = approach(c.out_db, t.out_db, a, 1e-4);
        c.off_level = approach(c.off_level, t.off_level, slow, 1e-4);
        c.off_side = approach(c.off_side, t.off_side, slow, 1e-3);
        c.off_release = approach(c.off_release, t.off_release, slow, 0.05);
        for i in 0..PER_KIND {
            for b in 0..MASTER_BANDS {
                let (band, tb) = (&mut c.eq[i][b], t.eq[i][b]);
                // A kind change: down to 0 dB with the old kind, then up with the new one.
                let same = band.0 == tb.0;
                band.2 = approach(band.2, if same { tb.2 } else { 0.0 }, a, 1e-3);
                c.off_eq[i][b] = approach(c.off_eq[i][b], if same { t.off_eq[i][b] } else { 0.0 }, slow, 1e-3);
                if !same && band.2 == 0.0 && c.off_eq[i][b] == 0.0 {
                    band.0 = tb.0;
                }
                band.1 = approach_log(band.1, tb.1, a);
                band.3 = approach_log(band.3, tb.3, a);
            }
            c.comp[i] = approach(c.comp[i], t.comp[i], a, 1e-3);
            c.mb[i] = approach(c.mb[i], t.mb[i], a, 1e-3);
            c.sat[i] = approach(c.sat[i], t.sat[i], a, 1e-3);
            c.clip[i] = approach(c.clip[i], t.clip[i], a, 1e-3);
        }
        for (g, tg) in c.match_gains.iter_mut().zip(t.match_gains) {
            *g = approach(*g, tg, a, 1e-3);
        }
        c.side_db = approach(c.side_db, t.side_db, a, 1e-3);
        c.mono_hz = approach(c.mono_hz, t.mono_hz, a, 0.05);
        c.width_on = t.width_on;
        c.lim = approach(c.lim, t.lim, a, 1e-3);
        c.bypass = approach(c.bypass, t.bypass, a, 1e-3);
        // A block runs while it is on or still gliding; when it reaches the identity it stops, with clean state.
        let (eq_was, match_was, width_was) = (c.eq_live, c.match_live, c.width_live);
        c.live();
        let (eq_now, match_now, width_now) = (c.eq_live, c.match_live, c.width_live);
        let changed = before != (c.eq, c.off_eq, c.match_gains, c.side_db, c.off_side, c.mono_hz);
        let release = (c.off_release != release_before).then_some(c.off_release);
        if changed {
            self.design();
        }
        if let Some(off) = release {
            self.limiter.set_params(self.p.ceiling_db, (self.p.release_ms + off).max(1.0));
        }
        for i in 0..PER_KIND {
            if eq_was[i] && !eq_now[i] {
                self.eq[i].iter_mut().for_each(Biquad::reset);
            }
        }
        if match_was && !match_now {
            self.match_eq.reset();
        }
        if width_was && !width_now {
            self.width.reset();
        }
    }

    /// Clears the state of the free blocks and of their delay lines.
    fn reset_blocks(&mut self) {
        for i in 0..PER_KIND {
            self.eq[i].iter_mut().for_each(Biquad::reset);
            self.comp[i].reset();
            self.mb[i].reset();
            self.sat[i].reset();
            self.clip[i].reset();
            self.sat_delay[i].reset();
            self.clip_delay[i].reset();
        }
        self.match_eq.reset();
        self.width.reset();
        self.pad.reset();
    }

    /// Clears all state and snaps every glide to its target at `pos` (start, seek, offline render).
    pub fn reset(&mut self, pos: i64) {
        if let Some(p) = self.pending.take() {
            self.pad.set_len(FREE_LATENCY - p.used_latency());
            self.apply(p);
        }
        self.duck = 1.0;
        self.hold = 0;
        self.pos = pos;
        self.cur = Cur::target(&self.p, pos);
        self.apply(self.p);
        self.design();
        self.reset_blocks();
        self.limiter.reset();
        self.dry.reset();
        self.lim_delay.reset();
        self.clip_in = [0.0; PER_KIND];
    }

    /// Gain reduction since the last call (meters): per place in the chain, per band of each multiband, and the
    /// limiter's. Gains in dB (≤ 0).
    pub fn take_gr(&mut self) -> Gr {
        let mut g = Gr::default();
        for (n, slot) in self.p.blocks.iter().enumerate() {
            let i = usize::from(slot.inst);
            // Blocks that are off still run (warm state) but do not show their reduction.
            match slot.block {
                BlockParams::None => break,
                BlockParams::Comp { .. } => {
                    let v = self.comp[i].take_gr_db();
                    g.blocks[n] = if self.cur.comp[i] > 0.0 { v } else { 0.0 };
                }
                BlockParams::Multiband { .. } => {
                    let v = self.mb[i].take_gr_db();
                    if self.cur.mb[i] > 0.0 {
                        g.bands[i] = v;
                        g.blocks[n] = v.iter().fold(0.0, |m, x| m.min(*x));
                    }
                }
                BlockParams::Clip { .. } => {
                    let v = self.clip[i].take_gr_db();
                    g.blocks[n] = if self.cur.clip[i] > 0.0 { v } else { 0.0 };
                }
                _ => {}
            }
        }
        let l = self.limiter.take_gr_db();
        g.limiter = if self.cur.lim > 0.0 { l } else { 0.0 };
        g
    }

    /// Highest level that reached each clipper since the last call.
    pub fn take_clip_in(&mut self) -> [f64; PER_KIND] {
        let v = self.clip_in.map(f64::from);
        self.clip_in = [0.0; PER_KIND];
        v
    }

    /// Processes a stereo block in place; the timeline goes on from where the last block (or `reset`) left it. The
    /// output lags by [`Self::latency`] in both modes.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let mut i = 0;
        while i < n {
            let e = (i + SUB).min(n);
            self.process_sub(&mut l[i..e], &mut r[i..e]);
            i = e;
        }
    }

    /// As [`Self::process`], for a block that starts at `pos` of the timeline (the sections follow it).
    pub fn process_at(&mut self, pos: i64, l: &mut [f32], r: &mut [f32]) {
        self.pos = pos;
        self.process(l, r);
    }

    fn process_sub(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len();
        let c0 = self.cur;
        let d0 = self.duck;
        self.step();
        self.pos += n as i64;
        let c1 = self.cur;
        let d1 = self.duck;
        let g = |db: f64| db_to_gain(db);
        // The unprocessed input for the A/B, delayed like the chain.
        let mut dl = [0.0f32; SUB];
        let mut dr = [0.0f32; SUB];
        let (dl, dr) = (&mut dl[..n], &mut dr[..n]);
        dl.copy_from_slice(l);
        dr.copy_from_slice(r);
        self.dry.process(dl, dr);
        ramp(dl, g(c0.bypass_db), g(c1.bypass_db));
        ramp(dr, g(c0.bypass_db), g(c1.bypass_db));

        ramp(l, g(c0.input_db), g(c1.input_db));
        ramp(r, g(c0.input_db), g(c1.input_db));
        let mut xl = [0.0f32; SUB];
        let mut xr = [0.0f32; SUB];
        let (xl, xr) = (&mut xl[..n], &mut xr[..n]);
        let blocks = self.p.blocks;
        for slot in &blocks {
            let i = usize::from(slot.inst);
            match slot.block {
                BlockParams::None => break,
                BlockParams::Eq { .. } => {
                    if c1.eq_live[i] || c0.eq_live[i] {
                        for f in &mut self.eq[i] {
                            if f.c != Coeffs::IDENTITY {
                                f.process(0, l);
                                f.process(1, r);
                            }
                        }
                    }
                }
                BlockParams::Match { .. } => {
                    if c1.match_live || c0.match_live {
                        self.match_eq.process(l, r);
                    }
                }
                // Compressors (no latency): cross-fade with their input.
                BlockParams::Comp { .. } => {
                    let (a, b) = (c0.comp[i], c1.comp[i]);
                    if a > 0.0 || b > 0.0 {
                        if a == 0.0 {
                            self.comp[i].reset();
                        }
                        let fade = a < 1.0 || b < 1.0;
                        if fade {
                            xl.copy_from_slice(l);
                            xr.copy_from_slice(r);
                        }
                        self.comp[i].process(&mut [&mut *l, &mut *r], None);
                        if fade {
                            mix(l, xl, a, b);
                            mix(r, xr, a, b);
                        }
                    }
                }
                BlockParams::Multiband { .. } => {
                    let (a, b) = (c0.mb[i], c1.mb[i]);
                    if a > 0.0 || b > 0.0 {
                        if a == 0.0 {
                            self.mb[i].reset();
                        }
                        let fade = a < 1.0 || b < 1.0;
                        if fade {
                            xl.copy_from_slice(l);
                            xr.copy_from_slice(r);
                        }
                        self.mb[i].process(l, r);
                        if fade {
                            mix(l, xl, a, b);
                            mix(r, xr, a, b);
                        }
                    }
                }
                BlockParams::Width { .. } => {
                    if c1.width_live || c0.width_live {
                        self.width.process(l, r);
                    }
                }
                // Oversampled blocks (latency): their dry path is a delay line that always runs, and they keep
                // running when off, so switching them on fades in a warm block, not an empty oversampler.
                BlockParams::Sat { .. } => {
                    let (a, b) = (c0.sat[i], c1.sat[i]);
                    xl.copy_from_slice(l);
                    xr.copy_from_slice(r);
                    self.sat_delay[i].process(xl, xr);
                    self.sat[i].process(0, l);
                    self.sat[i].process(1, r);
                    if a == 0.0 && b == 0.0 {
                        l.copy_from_slice(xl);
                        r.copy_from_slice(xr);
                    } else if a < 1.0 || b < 1.0 {
                        mix(l, xl, a, b);
                        mix(r, xr, a, b);
                    }
                }
                BlockParams::Clip { .. } => {
                    let (a, b) = (c0.clip[i], c1.clip[i]);
                    let peak = l.iter().chain(r.iter()).fold(0.0f32, |m, v| m.max(v.abs()));
                    self.clip_in[i] = self.clip_in[i].max(peak);
                    xl.copy_from_slice(l);
                    xr.copy_from_slice(r);
                    self.clip_delay[i].process(xl, xr);
                    self.clip[i].process(0, l);
                    self.clip[i].process(1, r);
                    if a == 0.0 && b == 0.0 {
                        l.copy_from_slice(xl);
                        r.copy_from_slice(xr);
                    } else if a < 1.0 || b < 1.0 {
                        mix(l, xl, a, b);
                        mix(r, xr, a, b);
                    }
                }
            }
        }
        self.pad.process(l, r);
        let (lv0, lv1) = (g(c0.loud_db + c0.off_level), g(c1.loud_db + c1.off_level));
        ramp(l, lv0, lv1);
        ramp(r, lv0, lv1);
        // Limiter (look-ahead): same scheme, also always running.
        xl.copy_from_slice(l);
        xr.copy_from_slice(r);
        self.lim_delay.process(xl, xr);
        self.limiter.process(l, r);
        if c0.lim == 0.0 && c1.lim == 0.0 {
            l.copy_from_slice(xl);
            r.copy_from_slice(xr);
        } else if c0.lim < 1.0 || c1.lim < 1.0 {
            mix(l, xl, c0.lim, c1.lim);
            mix(r, xr, c0.lim, c1.lim);
        }
        // The dip of a structure change silences the chain, not the unprocessed input of the A/B.
        ramp(l, d0, d1);
        ramp(r, d0, d1);
        // A/B: cross-fade to the unprocessed input at the master's loudness (MAS-5).
        if c0.bypass > 0.0 || c1.bypass > 0.0 {
            if c0.bypass == 1.0 && c1.bypass == 1.0 {
                l.copy_from_slice(dl);
                r.copy_from_slice(dr);
            } else {
                // mix() fades from its `dry` argument to the buffer: here from the chain to the input.
                xl.copy_from_slice(l);
                xr.copy_from_slice(r);
                l.copy_from_slice(dl);
                r.copy_from_slice(dr);
                mix(l, xl, c0.bypass, c1.bypass);
                mix(r, xr, c0.bypass, c1.bypass);
            }
        }
        ramp(l, g(c0.out_db), g(c1.out_db));
        ramp(r, g(c0.out_db), g(c1.out_db));
    }
}

/// Gain reduction of the chain at one moment (gains in dB, ≤ 0).
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize)]
pub struct Gr {
    /// Per place in the chain (compressors, multibands and clippers; 0 elsewhere).
    pub blocks: [f64; MAX_FREE],
    /// Per band of each multiband processor.
    pub bands: [[f64; MAX_BANDS]; PER_KIND],
    pub limiter: f64,
}

/// A stereo delay line (allocated once for its longest delay; the delay in use can be shorter).
struct Delay {
    buf: [Vec<f32>; 2],
    pos: usize,
    len: usize,
}

impl Delay {
    fn new(max: usize, len: usize) -> Self {
        Delay { buf: [vec![0.0; max.max(1)], vec![0.0; max.max(1)]], pos: 0, len: len.min(max) }
    }

    fn reset(&mut self) {
        for b in &mut self.buf {
            b.fill(0.0);
        }
        self.pos = 0;
    }

    /// A new delay (no more than the longest); the line starts empty.
    fn set_len(&mut self, len: usize) {
        self.len = len.min(self.buf[0].len());
        self.reset();
    }

    fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = self.len;
        if n == 0 {
            return;
        }
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let (x, y) = (*a, *b);
            *a = self.buf[0][self.pos];
            *b = self.buf[1][self.pos];
            self.buf[0][self.pos] = x;
            self.buf[1][self.pos] = y;
            self.pos = (self.pos + 1) % n;
        }
    }
}

/// Renders a stereo signal through the chain offline, aligned with the input (latency removed), in blocks of 4096.
/// The first sample is frame 0 of the timeline.
pub fn render(l: &[f32], r: &[f32], rate: f64, p: MasterParams) -> (Vec<f32>, Vec<f32>) {
    render_blocks(l, r, rate, p, 4096, &mut |_| {})
}

fn render_blocks(
    l: &[f32],
    r: &[f32],
    rate: f64,
    p: MasterParams,
    block: usize,
    meter: &mut dyn FnMut(&mut MasterProc),
) -> (Vec<f32>, Vec<f32>) {
    let mut m = MasterProc::new(rate, p);
    let lat = m.latency();
    let n = l.len();
    let mut ol = Vec::with_capacity(n + lat);
    let mut or = Vec::with_capacity(n + lat);
    ol.extend_from_slice(l);
    or.extend_from_slice(r);
    ol.resize(n + lat, 0.0);
    or.resize(n + lat, 0.0);
    for (a, b) in ol.chunks_mut(block).zip(or.chunks_mut(block)) {
        m.process(a, b);
        meter(&mut m);
    }
    (ol.split_off(lat), or.split_off(lat))
}

/// How much a dynamics block reduced the gain over a whole render, from its deepest reduction in each window of
/// [`GR_WINDOW_S`] (reductions are positive dB).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GrStats {
    /// Deepest reduction.
    pub max_db: f64,
    /// Mean over all the windows (silent ones included).
    pub mean_db: f64,
    /// Share of the windows (0..1) with any reduction (more than 0.1 dB), and with more than 1, 3 and 6 dB.
    pub active: f64,
    pub over_1: f64,
    pub over_3: f64,
    pub over_6: f64,
}

/// Window of the gain-reduction statistics.
pub const GR_WINDOW_S: f64 = 0.01;

#[derive(Default, Clone, Copy)]
struct GrAcc {
    n: u64,
    sum: f64,
    max: f64,
    over: [u64; 4],
}

impl GrAcc {
    fn push(&mut self, db: f64) {
        self.n += 1;
        self.sum += db;
        self.max = self.max.max(db);
        for (count, limit) in self.over.iter_mut().zip([0.1, 1.0, 3.0, 6.0]) {
            *count += u64::from(db > limit);
        }
    }

    fn stats(&self) -> GrStats {
        let n = self.n.max(1) as f64;
        GrStats {
            max_db: self.max,
            mean_db: self.sum / n,
            active: self.over[0] as f64 / n,
            over_1: self.over[1] as f64 / n,
            over_3: self.over[2] as f64 / n,
            over_6: self.over[3] as f64 / n,
        }
    }
}

/// Gain reduction over a whole render: per place in the chain (compressors, multibands, clippers) and the limiter's.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GrReport {
    pub blocks: [GrStats; MAX_FREE],
    pub limiter: GrStats,
}

/// As [`render`], with the gain reduction of every dynamics block. The audio is the same as [`render`]'s: the chain
/// does not depend on the block size (MAS-8).
pub fn render_metered(l: &[f32], r: &[f32], rate: f64, p: MasterParams) -> (Rendered, GrReport) {
    let block = ((rate * GR_WINDOW_S).round() as usize).max(1);
    let mut acc = [GrAcc::default(); MAX_FREE + 1];
    let out = render_blocks(l, r, rate, p, block, &mut |m| {
        // The meters report gains (≤ 0 dB).
        let g = m.take_gr();
        for (a, v) in acc.iter_mut().zip(g.blocks) {
            a.push(-v);
        }
        acc[MAX_FREE].push(-g.limiter);
    });
    let mut rep = GrReport { limiter: acc[MAX_FREE].stats(), ..GrReport::default() };
    for (s, a) in rep.blocks.iter_mut().zip(&acc) {
        *s = a.stats();
    }
    (out, rep)
}

/// The peak that reaches each clipper of the chain over the whole signal (MAS-11): what "take off N dB" is counted
/// from. A clipper downstream of another is measured with the first one working, so one pass per clipper that is on;
/// none when the chain has no clipper on. Deterministic.
pub fn clip_peaks(l: &[f32], r: &[f32], rate: u32, p: MasterParams) -> [f64; PER_KIND] {
    let mut q = p;
    q.clip_peak = [0.0; PER_KIND];
    q.limiter_on = false;
    q.bypass = false;
    for slot in p.blocks {
        let BlockParams::Clip { on: true, .. } = slot.block else { continue };
        let i = usize::from(slot.inst);
        let mut seen = 0.0f64;
        render_blocks(l, r, f64::from(rate), q, 4096, &mut |m| seen = seen.max(m.take_clip_in()[i]));
        // A silent input: any positive value switches the clipper on without touching anything.
        q.clip_peak[i] = seen.max(1.0e-9);
    }
    q.clip_peak
}

fn interleave(l: &[f32], r: &[f32]) -> Vec<f32> {
    l.iter().zip(r).flat_map(|(a, b)| [*a, *b]).collect()
}

/// Integrated loudness (LUFS) of a stereo pair.
pub fn lufs(l: &[f32], r: &[f32], rate: u32) -> Option<f64> {
    loudness::integrated(&interleave(l, r), 2, rate)
}

/// Passes a loudness search may render at most (each renders the whole master).
pub const SEARCH_PASSES: usize = 6;

/// How close to the destination a search stops (LU).
pub const ON_TARGET_LU: f64 = 0.1;

/// The gain before the limiter that brings the master to `target_lufs` (MAS-2): one pass without the limiter to
/// measure, then corrective passes with it until the loudness is within [`ON_TARGET_LU`]. Under heavy limiting a
/// dB of gain adds well under a dB of loudness, so the corrections follow the slope measured between passes.
/// Deterministic (MAS-8).
pub fn loudness_gain(l: &[f32], r: &[f32], rate: u32, p: MasterParams, target_lufs: f64) -> f64 {
    loudness_search(l, r, rate, p, target_lufs, None, &mut |_| {}).gain_db
}

/// The parameters with what only a render can tell: the clipper peaks and the loudness gain for `target_lufs`.
pub fn settled(l: &[f32], r: &[f32], rate: u32, p: MasterParams, target_lufs: f64) -> MasterParams {
    let s = loudness_search(l, r, rate, p, target_lufs, None, &mut |_| {});
    MasterParams { loud_gain_db: s.gain_db, clip_peak: s.clip_peak, ..p }
}

/// The output of a search at the gain it found.
pub type Rendered = (Vec<f32>, Vec<f32>);

/// What a loudness search found.
pub struct Search {
    pub gain_db: f64,
    /// The peak at each clipper ([`clip_peaks`]), measured before the search.
    pub clip_peak: [f64; PER_KIND],
    /// The master rendered at that gain, when the search's last pass already rendered it.
    pub rendered: Option<Rendered>,
}

/// As [`loudness_gain`], with the clipper peaks it measured first and the master rendered at the gain found when
/// the search's last pass already rendered it (the caller need not render it again). With `start`, the gain found
/// for a nearby state, the measuring pass without the limiter is skipped: a small change usually needs one pass
/// instead of three. Gains are rounded to 0.01 dB before each pass, so the gain returned is exactly the one a
/// returned render used.
pub fn loudness_search(
    l: &[f32],
    r: &[f32],
    rate: u32,
    p: MasterParams,
    target_lufs: f64,
    start: Option<f64>,
    progress: &mut dyn FnMut(f64),
) -> Search {
    let clip_peak = clip_peaks(l, r, rate, p);
    let p = MasterParams { clip_peak, ..p };
    let mut passes = 0.0;
    let mut pass_done = |progress: &mut dyn FnMut(f64)| {
        passes += 1.0;
        progress((passes / SEARCH_PASSES as f64).min(1.0));
    };
    let round = |g: f64| (g.clamp(-24.0, 24.0) * 100.0).round() / 100.0;
    let mut g = match start {
        Some(s) if s.is_finite() => round(s),
        _ => {
            let mut q = p;
            q.loud_gain_db = 0.0;
            q.limiter_on = false;
            q.bypass = false;
            let (a, b) = render(l, r, f64::from(rate), q);
            pass_done(progress);
            let Some(l0) = lufs(&a, &b, rate) else { return Search { gain_db: 0.0, clip_peak, rendered: None } };
            round(target_lufs - l0)
        }
    };
    let mut q = p;
    q.limiter_on = true;
    q.bypass = false;
    let mut last: Option<(f64, Rendered)> = None;
    // The previous pass with the limiter (gain, loudness): the slope between two passes guides the next gain.
    let mut prev: Option<(f64, f64)> = None;
    // The measuring pass without the limiter counts towards the passes of a cold search.
    for _ in 0..if start.is_some() { SEARCH_PASSES } else { SEARCH_PASSES - 1 } {
        q.loud_gain_db = g;
        let (a, b) = render(l, r, f64::from(rate), q);
        pass_done(progress);
        let Some(l1) = lufs(&a, &b, rate) else { break };
        last = Some((g, (a, b)));
        let err = target_lufs - l1;
        if err.abs() < ON_TARGET_LU {
            break;
        }
        // Loudness per dB of gain: 1 without limiting, less the harder the limiter works. Bounded so that a noisy
        // slope never sends the gain far away (at most four times the error).
        let slope = match prev {
            Some((g0, l0)) if (g - g0).abs() > 1e-6 => ((l1 - l0) / (g - g0)).clamp(0.25, 1.0),
            _ => 1.0,
        };
        prev = Some((g, l1));
        let next = round(g + err / slope);
        if next == g {
            break;
        }
        g = next;
    }
    match last {
        Some((lg, out)) if lg == g => Search { gain_db: g, clip_peak, rendered: Some(out) },
        _ => Search { gain_db: g, clip_peak, rendered: None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stemwise_dsp::testsignal::xorshift32;
    use stemwise_dsp::truepeak::true_peak_db;

    const R: u32 = 48000;

    /// A dynamic "song": noise bursts and sines at varying levels, louder in the middle.
    fn song(seconds: usize) -> (Vec<f32>, Vec<f32>) {
        let n = seconds * R as usize;
        let noise = xorshift32(11, 2 * n);
        let mut l = Vec::with_capacity(n);
        let mut r = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / f64::from(R);
            let env = 0.15 + 0.35 * (0.5 - 0.5 * (std::f64::consts::TAU * t / seconds as f64).cos());
            let beat = if (t * 2.0).fract() < 0.1 { 1.0 } else { 0.3 };
            let s = (std::f64::consts::TAU * 110.0 * t).sin() * 0.5 + (std::f64::consts::TAU * 2500.0 * t).sin() * 0.1;
            l.push((env * (s + 0.4 * beat * noise[2 * i])) as f32);
            r.push((env * (s * 0.9 + 0.4 * beat * noise[2 * i + 1])) as f32);
        }
        (l, r)
    }

    fn state(lufs: f64, tp: f64) -> MasterState {
        let mut m = MasterState::default();
        m.target.lufs = lufs;
        m.target.true_peak_db = tp;
        m
    }

    /// req: MAS-6, MAS-8
    #[test]
    fn the_metered_render_is_the_same_audio_and_reports_how_hard_the_limiter_works() {
        let (l, r) = song(6);
        let mut seen = Vec::new();
        for lufs_t in [-16.0, -8.0] {
            let m = state(lufs_t, -1.0);
            let p = MasterParams::of(&m, None, None, 0.0);
            let p = MasterParams { loud_gain_db: loudness_gain(&l, &r, R, p, lufs_t), ..p };
            let plain = render(&l, &r, f64::from(R), p);
            let (out, rep) = render_metered(&l, &r, f64::from(R), p);
            let lim = rep.limiter;
            assert!(out == plain, "metering must not change the audio");
            // Every free block is off by default: none reports anything.
            assert!(rep.blocks.iter().all(|b| *b == GrStats::default()));
            assert!(lim.over_1 >= lim.over_3 && lim.over_3 >= lim.over_6);
            assert!(lim.mean_db <= lim.max_db);
            seen.push(lim);
        }
        // A louder destination asks more of the limiter, in depth and in time.
        assert!(seen[1].max_db > seen[0].max_db + 3.0, "{seen:?}");
        assert!(seen[1].over_1 > seen[0].over_1, "{seen:?}");
    }

    /// req: MAS-1, MAS-2, MAS-3
    #[test]
    fn the_master_reaches_the_destination_and_never_exceeds_its_ceiling() {
        let (l, r) = song(12);
        // Loud destinations need heavy limiting, where a dB of gain adds much less than a dB of loudness.
        for (lufs_t, tp_t) in [(-14.0, -1.0), (-9.0, -1.0), (-10.0, -0.3), (-8.0, -1.0), (-7.0, -0.3)] {
            let m = state(lufs_t, tp_t);
            let p = MasterParams::of(&m, None, None, 0.0);
            let g = loudness_gain(&l, &r, R, p, lufs_t);
            let (a, b) = render(&l, &r, f64::from(R), MasterParams { loud_gain_db: g, ..p });
            let out: Vec<f32> = a.iter().zip(&b).flat_map(|(x, y)| [*x, *y]).collect();
            let tp = true_peak_db(&out, 2);
            let lu = lufs(&a, &b, R).unwrap();
            // MAS-3: the ceiling holds on the 4x-oversampled true peak; MAS-2: the loudness lands on the destination.
            assert!(tp <= tp_t + 1e-6, "{lufs_t}/{tp_t}: true peak {tp:.3}");
            assert!((lu - lufs_t).abs() < ON_TARGET_LU, "{lufs_t}/{tp_t}: loudness {lu:.2} with gain {g}");
            assert_eq!(a.len(), l.len());
        }
    }

    /// req: MAS-2, MAS-8
    #[test]
    fn a_search_from_a_nearby_gain_lands_on_the_target_and_hands_back_its_render() {
        let (l, r) = song(12);
        let m = state(-14.0, -1.0);
        let p = MasterParams::of(&m, None, None, 0.0);
        let Search { gain_db: cold, rendered: out, .. } = loudness_search(&l, &r, R, p, -14.0, None, &mut |_| {});
        // The render handed back is exactly the master at the gain returned.
        if let Some((a, b)) = &out {
            let (x, y) = render(&l, &r, f64::from(R), MasterParams { loud_gain_db: cold, limiter_on: true, ..p });
            assert!(a == &x && b == &y);
        }
        // From the gain of a nearby state (here: the same state, then 1 dB off) the result is on target too.
        for start in [cold, cold - 1.0] {
            let s = loudness_search(&l, &r, R, p, -14.0, Some(start), &mut |_| {});
            let g = s.gain_db;
            let (a, b) = s.rendered.unwrap_or_else(|| {
                render(&l, &r, f64::from(R), MasterParams { loud_gain_db: g, limiter_on: true, ..p })
            });
            let lu = lufs(&a, &b, R).unwrap();
            assert!((lu + 14.0).abs() < ON_TARGET_LU, "start {start}: loudness {lu:.2} with gain {g}");
        }
        // Deterministic: the same search gives the same gain (MAS-8).
        assert_eq!(loudness_search(&l, &r, R, p, -14.0, None, &mut |_| {}).gain_db, cold);
    }

    /// req: MAS-5
    #[test]
    fn the_bypassed_mixdown_plays_aligned_and_at_the_masters_loudness() {
        let (l, r) = song(8);
        let m = state(-14.0, -1.0);
        let p = MasterParams::of(&m, None, None, 0.0);
        let mix_lufs = lufs(&l, &r, R).unwrap();
        let ab = MasterParams { bypass: true, bypass_gain_db: -14.0 - mix_lufs, ..p };
        let (a, b) = render(&l, &r, f64::from(R), ab);
        assert!((lufs(&a, &b, R).unwrap() + 14.0).abs() < 0.05);
        // Same timing as the processed path: the bypass is the input scaled, sample for sample.
        let k = 10f64.powf((-14.0 - mix_lufs) / 20.0) as f32;
        assert!(a.iter().zip(&l).all(|(x, y)| (x - y * k).abs() < 1e-6));
    }

    /// The first block of a kind, to set it up.
    fn blk(m: &mut MasterState, k: BlockKind) -> &mut Block {
        &mut m.chain.blocks.iter_mut().find(|b| b.block.kind() == k).expect("a default chain has every kind").block
    }

    /// A state with EQ, compressor, multiband, width, saturation and clipper all at work.
    fn busy() -> MasterState {
        let mut m = state(-12.0, -1.0);
        for b in &mut m.chain.blocks {
            b.block.set_bypass(false);
            match &mut b.block {
                Block::Eq(eq) => {
                    eq.bands[1].enabled = true;
                    eq.bands[1].gain_db = 3.0;
                }
                Block::Compressor(c) => {
                    c.threshold_db = -24.0;
                    c.ratio = 3.0;
                    c.mix = 0.6;
                    c.listen_above_hz = 120.0;
                }
                Block::Multiband(x) => x.bands[0].threshold_db = -30.0,
                Block::Clipper(c) => c.take_off_db = 3.0,
                _ => {}
            }
        }
        m
    }

    fn rms_db(x: &[f32]) -> f64 {
        10.0 * (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).log10()
    }

    /// req: MAS-8, MAS-1
    #[test]
    fn the_master_is_deterministic_at_any_block_size_and_its_parameters_follow_the_state() {
        let (l, r) = song(4);
        let m = busy();
        let p = MasterParams::of(&m, Some(&[0.0; 8]), Some(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0]), 2.0);
        let tags: Vec<u8> = p.blocks.iter().map(|s| s.block.tag()).collect();
        assert_eq!(tags, [1, 2, 3, 4, 5, 6, 7, 0, 0], "the blocks of the state, in its order");
        assert!(matches!(p.blocks[0].block, BlockParams::Eq { on: true, bands } if bands[1].0 && bands[1].3 == 3.0));
        assert!(matches!(p.blocks[1].block, BlockParams::Match { on: true, .. }));
        assert!(
            matches!(p.blocks[2].block, BlockParams::Comp { on: true, p } if p.mix == 0.6 && p.detector_hpf_hz == 120.0)
        );
        let p = settled(&l, &r, R, p, -12.0);
        assert!(p.clip_peak[0] > 0.0, "the clipper's peak is measured");
        let x = render(&l, &r, f64::from(R), p);
        assert_eq!(x, render(&l, &r, f64::from(R), p));
        // The same audio whatever the block size (MAS-8): the offline render and the playback agree.
        for block in [64, 1000] {
            let y = render_blocks(&l, &r, f64::from(R), p, block, &mut |_| {});
            let d = x.0.iter().zip(&y.0).chain(x.1.iter().zip(&y.1)).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
            assert!(d <= 1e-5, "block {block}: differs by {d}");
        }
    }

    /// req: MAS-15
    #[test]
    fn the_order_of_the_blocks_is_the_order_of_processing_and_a_kind_can_be_there_twice() {
        let (l, r) = song(4);
        let mut m = state(-14.0, -1.0);
        m.chain.limiter.bypass = true;
        if let Block::Eq(eq) = blk(&mut m, BlockKind::Eq) {
            eq.bands[1].enabled = true;
            eq.bands[1].freq_hz = 110.0;
            eq.bands[1].gain_db = 9.0;
        }
        if let Block::Compressor(c) = blk(&mut m, BlockKind::Compressor) {
            c.bypass = false;
            c.threshold_db = -24.0;
            c.ratio = 4.0;
        }
        let run = |m: &MasterState| render(&l, &r, f64::from(R), MasterParams::of(m, None, None, 0.0)).0;
        // EQ then compressor: the boost drives the compressor. Compressor then EQ: the boost comes after it.
        let eq_first = run(&m);
        let mut swapped = m.clone();
        let comp = swapped.chain.blocks.remove(2);
        swapped.chain.blocks.insert(0, comp);
        let comp_first = run(&swapped);
        assert!(
            rms_db(&comp_first) > rms_db(&eq_first) + 1.0,
            "{:.2} vs {:.2}",
            rms_db(&comp_first),
            rms_db(&eq_first)
        );
        // The same latency whatever the chain: both renders are aligned with the input.
        let no_comp = {
            let mut x = m.clone();
            blk(&mut x, BlockKind::Compressor).set_bypass(true);
            run(&x)
        };
        let mut bare = m.clone();
        bare.chain.blocks.retain(|b| b.block.kind() == BlockKind::Eq);
        let same = run(&bare);
        let d = no_comp.iter().zip(&same).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
        assert!(d < 1e-6, "removing the blocks that are off changes nothing, not even the timing: {d}");
        // A second EQ adds to the first.
        let mut two = bare.clone();
        let id = two.chain.add(BlockKind::Eq, 9).unwrap();
        if let Block::Eq(eq) = &mut two.chain.block_mut(&id).unwrap().block {
            eq.bypass = false;
            eq.bands[1].enabled = true;
            eq.bands[1].freq_hz = 110.0;
            eq.bands[1].gain_db = -9.0;
        }
        let p = MasterParams::of(&two, None, None, 0.0);
        assert_eq!((p.blocks[0].inst, p.blocks[1].inst), (0, 1));
        let flat = render(&l, &r, f64::from(R), p).0;
        let d = flat.iter().zip(&l).skip(4800).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
        assert!(d < 2e-3, "+9 dB and −9 dB at the same band cancel: {d}");
    }

    /// req: MAS-11
    #[test]
    fn the_clipper_takes_its_decibels_off_the_tallest_peak_and_spares_the_limiter() {
        // A quiet body with a tall click twice a second.
        let n = 6 * R as usize;
        let l: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f64 / f64::from(R);
                let since = (t * 2.0).fract() / 2.0;
                (0.05 * (std::f64::consts::TAU * 220.0 * t).sin()
                    + 0.6 * (-since * 400.0).exp() * (std::f64::consts::TAU * 180.0 * since).cos())
                    as f32
            })
            .collect();
        let peak = |x: &[f32]| 20.0 * f64::from(x.iter().fold(0.0f32, |m, v| m.max(v.abs()))).log10();
        let mut m = state(-9.0, -1.0);
        let base = MasterParams::of(&m, None, None, 0.0);
        if let Block::Clipper(c) = blk(&mut m, BlockKind::Clipper) {
            c.bypass = false;
            c.take_off_db = 4.0;
            c.softness = 0.0;
        }
        let p = MasterParams::of(&m, None, None, 0.0);
        // Before the search the peak is unknown and the clipper lets everything through.
        let untouched = render(&l, &l, f64::from(R), MasterParams { limiter_on: false, ..p }).0;
        assert!((peak(&untouched) - peak(&l)).abs() < 0.05);
        let peaks = clip_peaks(&l, &l, R, p);
        assert!((20.0 * peaks[0].log10() - peak(&l)).abs() < 0.05, "the peak that reaches the clipper");
        let clipped = render(&l, &l, f64::from(R), MasterParams { limiter_on: false, clip_peak: peaks, ..p }).0;
        // 4 dB off the tallest peak (the decimation filter may give a little back).
        assert!((peak(&l) - peak(&clipped) - 4.0).abs() < 0.6, "{:.2} → {:.2}", peak(&l), peak(&clipped));
        // The body between two clicks is the same.
        let body = |x: &[f32]| rms_db(&x[R as usize / 4..R as usize / 2 - 2400]);
        assert!((body(&clipped) - body(&l)).abs() < 0.05);
        // At the same destination the limiter has less to do, and the clipper says how long it worked.
        let lim = |p: MasterParams| render_metered(&l, &l, f64::from(R), settled(&l, &l, R, p, -9.0)).1;
        let (without, with) = (lim(base), lim(p));
        assert!(with.limiter.max_db < without.limiter.max_db - 2.0, "{:?} vs {:?}", with.limiter, without.limiter);
        let clip_slot = p.blocks.iter().position(|s| s.block.tag() == 7).unwrap();
        assert!(
            with.blocks[clip_slot].max_db > 3.0 && with.blocks[clip_slot].active < 0.2,
            "{:?}",
            with.blocks[clip_slot]
        );
    }

    /// req: MAS-12
    #[test]
    fn a_multiband_holds_one_band_and_leaves_the_others_alone() {
        let n = 2 * R as usize;
        let w = |f: f64, a: f64, i: usize| (a * (std::f64::consts::TAU * f * i as f64 / f64::from(R)).sin()) as f32;
        let x: Vec<f32> = (0..n).map(|i| w(60.0, 0.4, i) + w(3000.0, 0.05, i)).collect();
        let mut m = state(-14.0, -1.0);
        m.chain.limiter.bypass = true;
        if let Block::Multiband(mb) = blk(&mut m, BlockKind::Multiband) {
            mb.bypass = false;
            assert!(mb.split(1, 800.0), "four bands");
            mb.bands[0].threshold_db = -30.0;
            mb.bands[0].ratio = 4.0;
            for b in &mut mb.bands[1..] {
                b.ratio = 1.0;
            }
        }
        let p = MasterParams::of(&m, None, None, 0.0);
        assert!(
            matches!(p.blocks[3].block, BlockParams::Multiband { on: true, p } if p.bands == 4 && p.xo_hz[1] == 800.0)
        );
        let ((a, _), rep) = render_metered(&x, &x, f64::from(R), p);
        assert!(rep.blocks[3].max_db > 6.0, "the low band is compressed: {:?}", rep.blocks[3]);
        // Level of each sine after the block (a correlation with the sine itself, on the second second).
        let level = |y: &[f32], f: f64| {
            let (mut s, mut c) = (0.0f64, 0.0f64);
            for (i, v) in y.iter().enumerate().skip(R as usize) {
                let ph = std::f64::consts::TAU * f * i as f64 / f64::from(R);
                s += f64::from(*v) * ph.sin();
                c += f64::from(*v) * ph.cos();
            }
            20.0 * (2.0 * s.hypot(c) / (y.len() - R as usize) as f64).log10()
        };
        assert!(level(&a, 60.0) < level(&x, 60.0) - 6.0);
        assert!((level(&a, 3000.0) - level(&x, 3000.0)).abs() < 0.2, "the band above is untouched");
        // Solo of the top band (listening): the low sine is gone.
        let solo = render(&x, &x, f64::from(R), p.with_solo(0, 0b1000)).0;
        assert!(level(&solo, 60.0) < level(&x, 60.0) - 40.0);
    }

    /// req: MAS-14
    #[test]
    fn section_offsets_act_only_inside_their_section_and_glide_at_its_edges() {
        use stemwise_core::model::{EqOffsets, SectionOffsets};
        let n = 3 * R as usize;
        let x: Vec<f32> =
            (0..n).map(|i| 0.1 * (std::f64::consts::TAU * 200.0 * i as f64 / f64::from(R)).sin() as f32).collect();
        let mut m = state(-14.0, -1.0);
        m.chain.limiter.bypass = true;
        let eq_id = m.chain.first(BlockKind::Eq).unwrap().id.clone();
        if let Block::Eq(eq) = blk(&mut m, BlockKind::Eq) {
            eq.bands[1].enabled = true;
            eq.bands[1].freq_hz = 200.0;
            eq.bands[1].gain_db = 1.0;
        }
        let second = |a: f64, b: f64| ((a * f64::from(R)) as usize, (b * f64::from(R)) as usize);
        let sections =
            [Section { id: "s1".into(), name: "Chorus".into(), color: 1, start: R as i64, end: 2 * R as i64 }];
        m.sections = vec![SectionOffsets {
            level_db: 2.0,
            eq: vec![EqOffsets { block: eq_id, gains_db: vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0] }],
            ..SectionOffsets::new(&"s1".to_string())
        }];
        assert!(m.check().is_ok());
        let p = MasterParams::of(&m, None, None, 0.0).with_sections(&m, &sections);
        assert_eq!(p.n_sections, 1);
        let (a, b) = render(&x, &x, f64::from(R), p);
        let level = |r: (usize, usize)| rms_db(&a[r.0..r.1]) - rms_db(&x[r.0..r.1]);
        // Outside: the song-wide +1 dB of the EQ. Inside: +2 dB of level and +1 dB more on the band.
        assert!((level(second(0.3, 0.9)) - 1.0).abs() < 0.1, "{:.2}", level(second(0.3, 0.9)));
        assert!((level(second(1.3, 1.9)) - 4.0).abs() < 0.1, "{:.2}", level(second(1.3, 1.9)));
        assert!((level(second(2.3, 2.9)) - 1.0).abs() < 0.1, "{:.2}", level(second(2.3, 2.9)));
        // No step at the edges: the sine's own largest step is 0.1·2π·200/48000 ≈ 0.0026, here up to +4 dB.
        assert!(max_step(&a, &b) < 0.0027 * 1.6 * 1.5, "{}", max_step(&a, &b));
        // A section the project no longer has is ignored.
        assert_eq!(MasterParams::of(&m, None, None, 0.0).with_sections(&m, &[]).n_sections, 0);
    }

    /// req: MAS-1
    #[test]
    fn mono_below_removes_the_side_of_the_low_end_only() {
        // Side = a 60 Hz sine, mid = a 2 kHz sine.
        let n = 2 * R as usize;
        let w = |f: f64, i: usize| (std::f64::consts::TAU * f * i as f64 / f64::from(R)).sin() as f32 * 0.25;
        let l: Vec<f32> = (0..n).map(|i| w(2000.0, i) + w(60.0, i)).collect();
        let r: Vec<f32> = (0..n).map(|i| w(2000.0, i) - w(60.0, i)).collect();
        let mut m = state(-14.0, -1.0);
        m.chain.limiter.bypass = true;
        if let Block::Width(w) = blk(&mut m, BlockKind::Width) {
            w.bypass = false;
            w.mono_below_hz = 120.0;
        }
        let (a, b) = render(&l, &r, f64::from(R), MasterParams::of(&m, None, None, 0.0));
        let rms = |x: &mut dyn Iterator<Item = f64>| {
            let v: Vec<f64> = x.collect();
            (v.iter().map(|s| s * s).sum::<f64>() / v.len() as f64).sqrt()
        };
        let half = R as usize / 2;
        let side_in = rms(&mut l.iter().zip(&r).skip(half).map(|(x, y)| f64::from(x - y) / 2.0));
        let side_out = rms(&mut a.iter().zip(&b).skip(half).map(|(x, y)| f64::from(x - y) / 2.0));
        let mid_in = rms(&mut l.iter().zip(&r).skip(half).map(|(x, y)| f64::from(x + y) / 2.0));
        let mid_out = rms(&mut a.iter().zip(&b).skip(half).map(|(x, y)| f64::from(x + y) / 2.0));
        // LR4 at 120 Hz: an octave below it, about −24 dB; the mid is untouched.
        assert!(side_out < side_in * 0.1, "side {side_out:.4} of {side_in:.4}");
        assert!((mid_out / mid_in - 1.0).abs() < 0.01);
    }

    /// Largest step between consecutive samples of either channel.
    fn max_step(l: &[f32], r: &[f32]) -> f32 {
        l.windows(2).chain(r.windows(2)).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max)
    }

    /// req: ENG-3, MAS-5, MAS-15
    #[test]
    fn master_changes_glide_instead_of_stepping() {
        // A 200 Hz sine at −20 dBFS: its largest natural step is 0.1·2π·200/48000 ≈ 0.0026.
        let amp = 0.1f32;
        let n = R as usize;
        let sine: Vec<f32> =
            (0..n).map(|i| amp * (std::f64::consts::TAU * 200.0 * i as f64 / f64::from(R)).sin() as f32).collect();
        let mut m = state(-14.0, -1.0);
        m.chain.limiter.bypass = true;
        let base = MasterParams::of(&m, None, None, 0.0);
        let with = |f: &dyn Fn(&mut MasterState)| {
            let mut x = m.clone();
            f(&mut x);
            MasterParams::of(&x, None, None, 0.0)
        };
        // Each change happens at the start of a block, in the middle of a cycle.
        let changes: [MasterParams; 9] = [
            MasterParams { input_gain_db: 12.0, ..base },
            MasterParams { loud_gain_db: -12.0, ..base },
            with(&|x| {
                if let Block::Eq(eq) = blk(x, BlockKind::Eq) {
                    eq.bands[2] = stemwise_core::model::EqBand {
                        enabled: true,
                        kind: EqKind::Peak,
                        freq_hz: 200.0,
                        gain_db: 12.0,
                        q: 1.0,
                    };
                }
            }),
            with(&|x| {
                if let Block::Width(w) = blk(x, BlockKind::Width) {
                    w.bypass = false;
                    w.mono_below_hz = 120.0;
                }
            }),
            MasterParams { bypass: true, bypass_gain_db: 9.0, ..base },
            MasterParams { limiter_on: true, ..base },
            with(&|x| blk(x, BlockKind::Multiband).set_bypass(false)),
            // Structure changes: a block moved, and blocks removed.
            with(&|x| x.chain.blocks.swap(0, 5)),
            with(&|x| x.chain.blocks.truncate(2)),
        ];
        for (k, change) in changes.iter().enumerate() {
            let mut proc = MasterProc::new(f64::from(R), base);
            let (mut l, mut r) = (sine.clone(), sine.clone());
            let block = 300;
            for (bi, (a, b)) in l.chunks_mut(block).zip(r.chunks_mut(block)).enumerate() {
                if bi == 40 {
                    proc.set(*change);
                }
                proc.process(a, b);
            }
            // After the change the level may be up to 4× (+12 dB): a glide keeps the step close to that sine's
            // own; a jump would be of the order of the amplitude itself (0.1 … 0.4).
            let step = max_step(&l[1000..], &r[1000..]);
            assert!(step < 4.0 * 0.0027 * 1.5, "change {k}: step {step:.4}");
            // And the chain is back: the last tenth of a second is not silence.
            assert!(rms_db(&l[n - 4800..]) > -40.0, "change {k}");
        }
    }
}
