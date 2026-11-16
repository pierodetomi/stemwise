//! The blocks of a node's chain (FX-10 → FX-23) as the graph runs them.
//!
//! - Each block owns a range of the node's parameter slots (its keys, see `stemwise_core::fx::BlockKey`) and the
//!   slot of its bypass. Its parameters are evaluated at the timeline time of the audio they touch: the node's input
//!   latency plus the latency of the blocks before it.
//! - **Bypass fade** (FX-10): the bypass value glides linearly over [`FADE_S`] per sample, whatever sets it (the
//!   author, an automation step). A block fully off is skipped, as in schema 3; a block fully on runs without a blend:
//!   with a static bypass the result is the one of schema 3 bit for bit.
//! - **Latency** (ENG-5): a block's latency is constant (oversampling, the pitch shifter's window). A block with
//!   latency blends its wet output with its own delayed input; the saturation keeps its own dry path, as in schema 3.

use stemwise_core::fx::{BlockKey, Fx, FxBlock, FxKind};
use stemwise_core::model::EqKind;
use stemwise_dsp::SUBBLOCK;
use stemwise_dsp::biquad::{Biquad, Coeffs, Kind};
use stemwise_dsp::clipper::{Clipper, ClipperParams};
use stemwise_dsp::color::{BassEnhancer, BassParams, Exciter, ExciterParams};
use stemwise_dsp::compressor::{Compressor, CompressorParams};
use stemwise_dsp::deesser::{Deesser, DeesserMode, DeesserParams};
use stemwise_dsp::dyneq::{DeqBandParams, DeqShape, DynamicEq, DynamicEqParams};
use stemwise_dsp::gate::{Gate, GateParams};
use stemwise_dsp::modulation::{LfoShape, ModMode, ModParams, Modulation, Tremolo, TremoloMode, TremoloParams};
use stemwise_dsp::multiband::{BandParams, MAX_BANDS, Multiband, MultibandParams};
use stemwise_dsp::pitch::{Doubler, DoublerParams, HarmVoiceParams, Harmonizer, HarmonizerParams};
use stemwise_dsp::saturation::{Character, Saturation, SaturationParams};
use stemwise_dsp::transient::{Transient, TransientParams};
use stemwise_dsp::width::{StereoWidth, WidthParams};

const N: usize = SUBBLOCK;
/// Length of the bypass crossfade (FX-10: about 10 ms).
pub const FADE_S: f64 = 0.010;

/// The catalog of block kinds (`fx_catalog.json`), with the latency each one adds (at 48 kHz).
pub fn catalog() -> Vec<stemwise_core::fx::FxInfo> {
    let mut c = stemwise_core::fx::catalog();
    for i in &mut c {
        i.latency = Blk::latency_of(i.kind, 48000.0);
    }
    c
}

/// Per-sample linear glide of a bypass value (0 = on, 1 = bypassed).
#[derive(Clone)]
pub(crate) struct Fade {
    pub cur: f64,
    pub target: f64,
    step: f64,
    pub buf: [f64; N],
}

impl Fade {
    pub fn new(v: f64, rate: f64) -> Self {
        Fade { cur: v, target: v, step: 1.0 / (FADE_S * rate), buf: [v; N] }
    }

    pub fn snap(&mut self, v: f64) {
        self.cur = v;
        self.target = v;
    }

    /// Fills `buf[..len]` with the value of each sample; returns (all bypassed, all on).
    pub fn fill(&mut self, len: usize) -> (bool, bool) {
        if self.cur == self.target {
            let v = self.cur;
            self.buf[..len].fill(v);
            return (v >= 1.0, v <= 0.0);
        }
        for i in 0..len {
            let d = self.target - self.cur;
            self.cur = if d.abs() <= self.step { self.target } else { self.cur + self.step * d.signum() };
            self.buf[i] = self.cur;
        }
        (false, false)
    }
}

/// Stereo delay line (latency compensation and the dry path of blocks with latency).
pub(crate) struct Line {
    buf: [Vec<f32>; 2],
    pos: usize,
}

impl Line {
    pub fn new(len: usize) -> Option<Line> {
        (len > 0).then(|| Line { buf: [vec![0.0; len], vec![0.0; len]], pos: 0 })
    }

    pub fn clear(&mut self) {
        self.buf[0].fill(0.0);
        self.buf[1].fill(0.0);
        self.pos = 0;
    }

    /// Pushes `x[i]` and returns the sample `len` frames old, for channel `c` (call `advance` once per piece).
    #[inline]
    pub fn tap(&mut self, c: usize, i: usize, x: f32) -> f32 {
        let l = self.buf[c].len();
        let p = self.pos + i;
        let p = if p >= l { p - l } else { p };
        std::mem::replace(&mut self.buf[c][p], x)
    }

    pub fn advance(&mut self, len: usize) {
        self.pos = (self.pos + len) % self.buf[0].len();
    }

    /// Delays `ch` channels of `w[..]` in place.
    pub fn run(&mut self, w: &mut [Vec<f32>; 2], r: std::ops::Range<usize>, ch: usize) {
        let len = r.len();
        for c in 0..ch {
            for (i, v) in w[c][r.clone()].iter_mut().enumerate() {
                *v = self.tap(c, i, *v);
            }
        }
        self.advance(len);
    }
}

// Built once per graph, off the audio thread: the size of a variant costs nothing at run time.
#[allow(clippy::large_enum_variant)]
/// Runs `f` on the first `ch` channels of `w[..][r]` as one slice of channels (stereo-linked processors).
fn with_chans(w: &mut [Vec<f32>; 2], r: std::ops::Range<usize>, ch: usize, f: impl FnOnce(&mut [&mut [f32]])) {
    let (w0, w1) = w.split_at_mut(1);
    if ch == 2 {
        let mut c: [&mut [f32]; 2] = [&mut w0[0][r.clone()], &mut w1[0][r]];
        f(&mut c);
    } else {
        let mut c: [&mut [f32]; 1] = [&mut w0[0][r]];
        f(&mut c);
    }
}

pub(crate) enum Proc {
    Hpf(Biquad, usize, usize),
    Lpf(Biquad, usize, usize),
    Eq {
        b: [Biquad; 4],
        on: [bool; 4],
        kind: [Kind; 4],
        s: [[usize; 3]; 4],
    },
    Comp(Box<Compressor>, [usize; 8]),
    Sat(Box<Saturation>, [usize; 2], Character),
    Gate(Box<Gate>, [usize; 8]),
    /// Slots: frequency, threshold, reduction, and the engine-only `listen` (FX-14: never in the document).
    Deess(Box<Deesser>, [usize; 4], DeesserMode),
    Deq(Box<DynamicEq>, [Option<[usize; 6]>; 2], [(bool, DeqShape); 2]),
    Trans(Box<Transient>, [usize; 4]),
    Clip(Box<Clipper>, [usize; 2]),
    Mb(Box<Multiband>, usize, [[usize; 5]; MAX_BANDS], [usize; MAX_BANDS - 1], [bool; MAX_BANDS]),
    Exc(Box<Exciter>, [usize; 3]),
    Bass(Box<BassEnhancer>, [usize; 3]),
    Width(Box<StereoWidth>, [usize; 2]),
    Mod(Box<Modulation>, [usize; 5], ModMode, u8, u8),
    Trem(Box<Tremolo>, [usize; 3], TremoloMode, LfoShape),
    Harm(Box<Harmonizer>, [Option<[usize; 4]>; 2], usize, [bool; 2]),
    Dbl(Box<Doubler>, [usize; 4], u8),
}

/// A block of a node's chain.
impl Blk {
    /// Gives a de-esser its engine-only "listen" slot (FX-14).
    pub fn set_listen_slot(&mut self, slot: usize) {
        if let Proc::Deess(_, s, _) = &mut self.proc {
            s[3] = slot;
        }
    }
}

pub(crate) struct Blk {
    #[allow(dead_code)]
    pub id: String,
    pub kind: FxKind,
    pub proc: Proc,
    /// Slot of the bypass parameter.
    pub by: usize,
    pub fade: Fade,
    /// Latency of the blocks before this one, frames (parameters are evaluated that much later).
    pub rel: i64,
    pub lat: usize,
    /// The chain is mono up to here and this block makes it stereo (FX-10).
    pub promote: bool,
    /// Channels this block processes.
    pub ch: usize,
    /// Key inputs of the node this block listens to (compressor, gate; dynamic EQ bands).
    pub keys: [Option<usize>; 2],
    /// Dry path of a block with latency, for the bypass blend.
    pub dry_line: Option<Line>,
    /// The second channel of a mono chain for the blocks that always process two (the multiband).
    scratch: [f32; N],
    cache: [f64; 14],
    /// The processor sleeps for this cell: the block was fully off when the cell started.
    skip: bool,
}

fn eq_kind(k: EqKind) -> Kind {
    match k {
        EqKind::Peak => Kind::Peak,
        EqKind::LowShelf => Kind::LowShelf,
        EqKind::HighShelf => Kind::HighShelf,
    }
}

/// The parameter keys of a block as the engine lays them out: those it has, plus the compressor's sidechain
/// low cut even without a sidechain (its default, as in schema 3).
pub(crate) fn keys_of(fx: &Fx) -> Vec<BlockKey> {
    let mut k = BlockKey::all(fx);
    if matches!(fx, Fx::Compressor(_)) && !k.iter().any(|x| x.key == "sc_hpf") {
        k.push(BlockKey { index: None, key: "sc_hpf".into() });
    }
    k
}

impl Blk {
    /// Latency of a kind of block at a rate, frames (a multiple of the 32-frame grid): the oversampled blocks and
    /// the harmonizer's pitch shifter (FX-10, ENG-5).
    pub fn latency_of(kind: FxKind, rate: f64) -> usize {
        match kind {
            FxKind::Saturation => Saturation::LATENCY,
            FxKind::Clipper => Clipper::LATENCY,
            FxKind::Exciter => Exciter::LATENCY,
            FxKind::BassEnhancer => BassEnhancer::LATENCY,
            FxKind::Harmonizer => Harmonizer::latency(rate),
            _ => 0,
        }
    }

    /// `slot(key)` gives the node slot of one of this block's keys.
    pub fn new(b: &FxBlock, rate: f64, by: usize, slot: &dyn Fn(&str) -> usize) -> Blk {
        let kind = b.fx.kind();
        let ks = |keys: &[&str]| -> Vec<usize> { keys.iter().map(|k| slot(k)).collect() };
        let arr = |v: Vec<usize>| -> [usize; 8] { std::array::from_fn(|i| v.get(i).copied().unwrap_or(usize::MAX)) };
        let shape = |k: EqKind| match k {
            EqKind::Peak => DeqShape::Peak,
            EqKind::LowShelf => DeqShape::LowShelf,
            EqKind::HighShelf => DeqShape::HighShelf,
        };
        let proc = match &b.fx {
            Fx::Hpf(_) => Proc::Hpf(Biquad::default(), slot("freq"), slot("res")),
            Fx::Lpf(_) => Proc::Lpf(Biquad::default(), slot("freq"), slot("res")),
            Fx::Eq(e) => Proc::Eq {
                b: std::array::from_fn(|_| Biquad::default()),
                on: std::array::from_fn(|i| e.bands.get(i).is_some_and(|b| b.enabled)),
                kind: std::array::from_fn(|i| e.bands.get(i).map_or(Kind::Peak, |b| eq_kind(b.kind))),
                s: std::array::from_fn(|i| {
                    if i < e.bands.len() {
                        let n = i + 1;
                        [slot(&format!("{n}.freq")), slot(&format!("{n}.gain")), slot(&format!("{n}.q"))]
                    } else {
                        [usize::MAX; 3]
                    }
                }),
            },
            Fx::Compressor(_) => Proc::Comp(
                Box::new(Compressor::new(rate)),
                ["threshold", "ratio", "attack", "release", "knee", "makeup", "mix", "sc_hpf"].map(slot),
            ),
            Fx::Saturation(x) => Proc::Sat(
                Box::new(Saturation::new(rate)),
                ["drive", "mix"].map(slot),
                match x.character {
                    stemwise_core::fx::SatCharacter::Soft => Character::Soft,
                    stemwise_core::fx::SatCharacter::Tape => Character::Tape,
                    stemwise_core::fx::SatCharacter::Tube => Character::Tube,
                    stemwise_core::fx::SatCharacter::Transistor => Character::Transistor,
                },
            ),
            Fx::Gate(_) => Proc::Gate(
                Box::new(Gate::new(rate)),
                arr(ks(&["threshold", "range", "ratio", "attack", "hold", "release", "det_hpf", "det_lpf"])),
            ),
            Fx::Deesser(d) => Proc::Deess(
                Box::new(Deesser::new(rate)),
                ["freq", "threshold", "reduction"]
                    .map(slot)
                    .into_iter()
                    .chain([usize::MAX])
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("four"),
                match d.mode {
                    stemwise_core::fx::DeesserMode::Split => DeesserMode::Split,
                    stemwise_core::fx::DeesserMode::Wide => DeesserMode::Wide,
                },
            ),
            Fx::DynamicEq(d) => Proc::Deq(
                Box::new(DynamicEq::new(rate)),
                std::array::from_fn(|i| {
                    d.bands.get(i).map(|_| {
                        let n = i + 1;
                        ["freq", "q", "threshold", "range", "attack", "release"].map(|k| slot(&format!("{n}.{k}")))
                    })
                }),
                std::array::from_fn(|i| d.bands.get(i).map_or((false, DeqShape::Peak), |b| (b.enabled, shape(b.kind)))),
            ),
            Fx::Transient(_) => {
                Proc::Trans(Box::new(Transient::new(rate)), ["attack", "sustain", "speed", "mix"].map(slot))
            }
            Fx::Clipper(_) => Proc::Clip(Box::new(Clipper::new(rate)), ["threshold", "softness"].map(slot)),
            Fx::Multiband(m) => {
                let n = m.bands.len().clamp(2, MAX_BANDS);
                Proc::Mb(
                    Box::new(Multiband::new(rate)),
                    n,
                    std::array::from_fn(|i| {
                        if i < n {
                            let b = i + 1;
                            ["threshold", "ratio", "attack", "release", "gain"].map(|k| slot(&format!("{b}.{k}")))
                        } else {
                            [usize::MAX; 5]
                        }
                    }),
                    std::array::from_fn(|i| if i + 1 < n { slot(&format!("xover.{}", i + 1)) } else { usize::MAX }),
                    std::array::from_fn(|i| m.bands.get(i).is_some_and(|b| !b.bypass)),
                )
            }
            Fx::Exciter(_) => Proc::Exc(Box::new(Exciter::new(rate)), ["freq", "drive", "amount"].map(slot)),
            Fx::BassEnhancer(_) => {
                Proc::Bass(Box::new(BassEnhancer::new(rate)), ["freq", "harmonics", "mix"].map(slot))
            }
            Fx::Width(_) => Proc::Width(Box::new(StereoWidth::new(rate)), ["width", "mono"].map(slot)),
            Fx::Modulation(m) => Proc::Mod(
                Box::new(Modulation::new(rate)),
                ["rate", "depth", "feedback", "spread", "mix"].map(slot),
                match m.mode {
                    stemwise_core::fx::ModMode::Chorus => ModMode::Chorus,
                    stemwise_core::fx::ModMode::Flanger => ModMode::Flanger,
                    stemwise_core::fx::ModMode::Phaser => ModMode::Phaser,
                },
                m.voices,
                m.stages,
            ),
            Fx::Tremolo(m) => Proc::Trem(
                Box::new(Tremolo::new(rate)),
                ["rate", "depth", "phase"].map(slot),
                match m.mode {
                    stemwise_core::fx::TremoloMode::Tremolo => TremoloMode::Tremolo,
                    stemwise_core::fx::TremoloMode::Autopan => TremoloMode::Autopan,
                },
                match m.shape {
                    stemwise_core::fx::LfoShape::Sine => LfoShape::Sine,
                    stemwise_core::fx::LfoShape::Triangle => LfoShape::Triangle,
                    stemwise_core::fx::LfoShape::Square => LfoShape::Square,
                },
            ),
            Fx::Harmonizer(h) => Proc::Harm(
                Box::new(Harmonizer::new(rate)),
                std::array::from_fn(|i| {
                    h.voices.get(i).map(|_| {
                        let n = i + 1;
                        ["semis", "cents", "level", "pan"].map(|k| slot(&format!("{n}.{k}")))
                    })
                }),
                slot("mix"),
                std::array::from_fn(|i| h.voices.get(i).is_some_and(|v| v.enabled)),
            ),
            Fx::Doubler(d) => {
                Proc::Dbl(Box::new(Doubler::new(rate)), ["detune", "delay", "spread", "mix"].map(slot), d.voices)
            }
        };
        let lat = Self::latency_of(kind, rate);
        Blk {
            id: b.id.clone(),
            kind,
            proc,
            by,
            fade: Fade::new(if b.fx.bypass() { 1.0 } else { 0.0 }, rate),
            rel: 0,
            lat,
            promote: false,
            ch: 1,
            keys: [None; 2],
            dry_line: if kind == FxKind::Saturation { None } else { Line::new(lat) },
            scratch: [0.0; N],
            cache: [f64::NAN; 14],
            skip: false,
        }
    }

    pub fn reset(&mut self) {
        self.reset_proc();
        if let Some(l) = &mut self.dry_line {
            l.clear();
        }
    }

    /// Clears the processor's own state, not the dry path: what a block that was off starts from when it comes back.
    fn reset_proc(&mut self) {
        self.cache = [f64::NAN; 14];
        match &mut self.proc {
            Proc::Hpf(b, ..) | Proc::Lpf(b, ..) => b.reset(),
            Proc::Eq { b, .. } => b.iter_mut().for_each(Biquad::reset),
            Proc::Comp(c, _) => c.reset(),
            Proc::Sat(s, ..) => s.reset(),
            Proc::Gate(x, _) => x.reset(),
            Proc::Deess(x, ..) => x.reset(),
            Proc::Deq(x, ..) => x.reset(),
            Proc::Trans(x, _) => x.reset(),
            Proc::Clip(x, _) => x.reset(),
            Proc::Mb(x, ..) => x.reset(),
            Proc::Exc(x, _) => x.reset(),
            Proc::Bass(x, _) => x.reset(),
            Proc::Width(x, _) => x.reset(),
            Proc::Mod(x, ..) => x.reset(),
            Proc::Trem(x, ..) => x.reset(),
            Proc::Harm(x, ..) => x.reset(),
            Proc::Dbl(x, ..) => x.reset(),
        }
    }

    /// New values for a cell (`v(slot)`: the parameter at the cell start). Filters get coefficients only when a
    /// value changed (AUT-6).
    /// `cycles(slot)`: the timeline frame of the cell start for that parameter and the cycles an oscillator at that parameter's frequency has run since the song's start, read
    /// when a block starts (a seek, a block that wakes up) so its LFO phase depends on the position alone (FX-20).
    pub fn update(
        &mut self,
        v: &dyn Fn(usize) -> f64,
        cycles: &dyn Fn(usize) -> (f64, i64),
        by_end: f64,
        rate: f64,
        snap: bool,
    ) {
        if snap {
            self.fade.snap(by_end);
        } else {
            self.fade.target = by_end;
        }
        // Whether the processor sleeps is decided once per cell, from the fade at the cell's start: the pieces a
        // block boundary cuts the cell into cannot change what the processor has heard (ENG-4). A block that wakes
        // up starts from a clean state, not from the audio it held when it went off.
        let skip = self.fade.cur >= 1.0 && self.fade.target >= 1.0;
        let woke = self.skip && !skip && !snap;
        if woke {
            self.reset_proc();
        }
        self.skip = skip;
        if snap || woke {
            match &mut self.proc {
                Proc::Mod(m, s, ..) => {
                    let (c, at) = cycles(s[0]);
                    m.set_phase(c, at);
                }
                Proc::Trem(t, s, ..) => {
                    let (c, at) = cycles(s[0]);
                    t.set_phase(c, at);
                }
                _ => {}
            }
        }
        match &mut self.proc {
            Proc::Hpf(bq, f, q) | Proc::Lpf(bq, f, q) => {
                let (f, q) = (v(*f), v(*q));
                if f != self.cache[0] || q != self.cache[1] {
                    let k = if self.kind == FxKind::Hpf { Kind::HighPass } else { Kind::LowPass };
                    bq.c = Coeffs::design(k, f, 0.0, q, rate);
                    self.cache[0] = f;
                    self.cache[1] = q;
                }
            }
            Proc::Eq { b, kind, s, .. } => {
                for i in 0..4 {
                    if s[i][0] == usize::MAX {
                        continue;
                    }
                    let w = [v(s[i][0]), v(s[i][1]), v(s[i][2])];
                    let o = 2 + 3 * i;
                    if w != self.cache[o..o + 3] {
                        b[i].c = Coeffs::design(kind[i], w[0], w[1], w[2], rate);
                        self.cache[o..o + 3].copy_from_slice(&w);
                    }
                }
            }
            Proc::Comp(c, s) => {
                c.set_params(&CompressorParams {
                    threshold_db: v(s[0]),
                    ratio: v(s[1]),
                    attack_ms: v(s[2]),
                    release_ms: v(s[3]),
                    knee_db: v(s[4]),
                    makeup_db: v(s[5]),
                    sidechain_hpf_hz: v(s[7]),
                    detector_hpf_hz: 0.0,
                    mix: v(s[6]),
                });
                if snap {
                    c.settle();
                }
            }
            Proc::Sat(sat, s, character) => {
                // Always runs, so the chain latency is constant (ENG-5); off is mix 0, gliding with the fade.
                let by = if snap { by_end } else { self.fade.cur };
                sat.set_params(&SaturationParams {
                    drive_db: v(s[0]),
                    mix: v(s[1]) * (1.0 - by),
                    character: *character,
                });
            }
            Proc::Gate(g, s) => g.set_params(&GateParams {
                threshold_db: v(s[0]),
                range_db: v(s[1]),
                ratio: v(s[2]),
                attack_ms: v(s[3]),
                hold_ms: v(s[4]),
                release_ms: v(s[5]),
                detector_hpf_hz: v(s[6]),
                detector_lpf_hz: v(s[7]),
            }),
            Proc::Deess(d, s, mode) => d.set_params(&DeesserParams {
                freq_hz: v(s[0]),
                threshold_db: v(s[1]),
                max_reduction_db: v(s[2]),
                mode: *mode,
                listen: s[3] != usize::MAX && v(s[3]) >= 0.5,
            }),
            Proc::Deq(d, s, on) => {
                let band = |i: usize| match s[i] {
                    Some(k) => DeqBandParams {
                        on: on[i].0,
                        shape: on[i].1,
                        freq_hz: v(k[0]),
                        q: v(k[1]),
                        threshold_db: v(k[2]),
                        range_db: v(k[3]),
                        attack_ms: v(k[4]),
                        release_ms: v(k[5]),
                    },
                    None => DeqBandParams::default(),
                };
                d.set_params(&DynamicEqParams { bands: [band(0), band(1)] });
            }
            Proc::Trans(t, s) => {
                t.set_params(&TransientParams { attack_db: v(s[0]), sustain_db: v(s[1]), speed: v(s[2]), mix: v(s[3]) })
            }
            Proc::Clip(c, s) => {
                c.set_params(&ClipperParams { threshold: stemwise_dsp::db_to_gain(v(s[0])), softness: v(s[1]) })
            }
            Proc::Mb(m, n, bands, xo, on) => {
                let mut p = MultibandParams { bands: *n, ..MultibandParams::default() };
                for i in 0..*n - 1 {
                    p.xo_hz[i] = v(xo[i]);
                }
                for i in 0..*n {
                    let k = bands[i];
                    p.band[i] = BandParams {
                        on: on[i],
                        solo: false,
                        comp: CompressorParams {
                            threshold_db: v(k[0]),
                            ratio: v(k[1]),
                            attack_ms: v(k[2]),
                            release_ms: v(k[3]),
                            knee_db: 6.0,
                            makeup_db: v(k[4]),
                            sidechain_hpf_hz: 100.0,
                            detector_hpf_hz: 0.0,
                            mix: 1.0,
                        },
                    };
                }
                m.set_params(&p);
            }
            Proc::Exc(e, s) => e.set_params(&ExciterParams { freq_hz: v(s[0]), drive_db: v(s[1]), amount: v(s[2]) }),
            Proc::Bass(e, s) => e.set_params(&BassParams { freq_hz: v(s[0]), harmonics: v(s[1]), mix: v(s[2]) }),
            Proc::Width(w, s) => w.set_params(&WidthParams { width: v(s[0]), mono_below_hz: v(s[1]) }),
            Proc::Mod(m, s, mode, voices, stages) => m.set_params(&ModParams {
                mode: *mode,
                rate_hz: v(s[0]),
                depth: v(s[1]),
                feedback: v(s[2]),
                voices: *voices,
                stages: *stages,
                spread_deg: v(s[3]),
                mix: v(s[4]),
            }),
            Proc::Trem(t, s, mode, shape) => t.set_params(&TremoloParams {
                mode: *mode,
                rate_hz: v(s[0]),
                depth: v(s[1]),
                shape: *shape,
                phase_deg: v(s[2]),
            }),
            Proc::Harm(h, s, mix, on) => {
                let voice = |i: usize| match s[i] {
                    Some(k) => HarmVoiceParams {
                        on: on[i],
                        semitones: v(k[0]),
                        cents: v(k[1]),
                        level_db: v(k[2]),
                        pan: v(k[3]),
                    },
                    None => HarmVoiceParams { on: false, semitones: 0.0, cents: 0.0, level_db: -144.0, pan: 0.0 },
                };
                h.set_params(&HarmonizerParams { voices: [voice(0), voice(1)], mix: v(*mix) });
            }
            Proc::Dbl(d, s, voices) => d.set_params(&DoublerParams {
                voices: *voices,
                detune_cents: v(s[0]),
                delay_ms: v(s[1]),
                spread: v(s[2]),
                mix: v(s[3]),
            }),
        }
    }

    /// Processes `w[..][r]` in place. `dry` is scratch; `keys` the key inputs of the node for this piece.
    pub fn process(
        &mut self,
        w: &mut [Vec<f32>; 2],
        dry: &mut [Vec<f32>; 2],
        r: std::ops::Range<usize>,
        keys: &[[&[f32]; 2]],
        gr: &mut f64,
        pos: i64,
    ) {
        let len = r.len();
        let ch = self.ch;
        if self.promote {
            let (a, b) = w.split_at_mut(1);
            b[0][r.clone()].copy_from_slice(&a[0][r.clone()]);
        }
        if let Proc::Sat(sat, ..) = &mut self.proc {
            // Advance the fade as the other blocks do (the saturation reads it per cell).
            self.fade.fill(len);
            for c in 0..ch {
                sat.process(c, &mut w[c][r.clone()]);
            }
            return;
        }
        let (_, on) = self.fade.fill(len);
        let off = self.skip;
        if off && self.lat == 0 {
            return;
        }
        if matches!(self.proc, Proc::Eq { on, .. } if !on.iter().any(|e| *e)) {
            return;
        }
        for c in 0..ch {
            dry[c][r.clone()].copy_from_slice(&w[c][r.clone()]);
        }
        if !off {
            match &mut self.proc {
                Proc::Hpf(b, ..) | Proc::Lpf(b, ..) => {
                    for c in 0..ch {
                        b.process(c, &mut w[c][r.clone()]);
                    }
                }
                Proc::Eq { b, on, .. } => {
                    for c in 0..ch {
                        for k in 0..4 {
                            if on[k] {
                                b[k].process(c, &mut w[c][r.clone()]);
                            }
                        }
                    }
                }
                Proc::Comp(comp, _) => {
                    let key: Option<[&[f32]; 2]> = self.keys[0].map(|k| [&keys[k][0][..len], &keys[k][1][..len]]);
                    let (w0, w1) = w.split_at_mut(1);
                    if ch == 2 {
                        let mut chans: [&mut [f32]; 2] = [&mut w0[0][r.clone()], &mut w1[0][r.clone()]];
                        comp.process(&mut chans, key.as_ref().map(|k| &k[..]));
                    } else {
                        let mut chans: [&mut [f32]; 1] = [&mut w0[0][r.clone()]];
                        comp.process(&mut chans, key.as_ref().map(|k| &k[..]));
                    }
                    *gr = gr.min(comp.take_gr_db());
                }
                Proc::Sat(..) => {}
                Proc::Gate(g, _) => {
                    let key: Option<[&[f32]; 2]> = self.keys[0].map(|k| [&keys[k][0][..len], &keys[k][1][..len]]);
                    with_chans(w, r.clone(), ch, |c| g.process(c, key.as_ref().map(|k| &k[..])));
                    *gr = gr.min(g.take_gr_db());
                }
                Proc::Deess(d, ..) => {
                    with_chans(w, r.clone(), ch, |c| d.process(c));
                    *gr = gr.min(d.take_gr_db());
                }
                Proc::Deq(d, ..) => {
                    let k0: Option<[&[f32]; 2]> = self.keys[0].map(|k| [&keys[k][0][..len], &keys[k][1][..len]]);
                    let k1: Option<[&[f32]; 2]> = self.keys[1].map(|k| [&keys[k][0][..len], &keys[k][1][..len]]);
                    with_chans(w, r.clone(), ch, |c| {
                        d.process(c, [k0.as_ref().map(|k| &k[..]), k1.as_ref().map(|k| &k[..])])
                    });
                    let g = d.take_gain_db();
                    *gr = gr.min(g[0].min(g[1]).min(0.0));
                }
                Proc::Trans(t, _) => with_chans(w, r.clone(), ch, |c| t.process(c)),
                Proc::Clip(c, _) => {
                    for k in 0..ch {
                        c.process(k, &mut w[k][r.clone()]);
                    }
                    *gr = gr.min(c.take_gr_db());
                }
                Proc::Mb(m, ..) => {
                    let (w0, w1) = w.split_at_mut(1);
                    if ch == 2 {
                        m.process(&mut w0[0][r.clone()], &mut w1[0][r.clone()]);
                    } else {
                        let sc = &mut self.scratch[..len];
                        sc.copy_from_slice(&w0[0][r.clone()]);
                        m.process(&mut w0[0][r.clone()], sc);
                    }
                    let g = m.take_gr_db();
                    *gr = gr.min(g.iter().copied().fold(0.0, f64::min));
                }
                Proc::Exc(e, _) => {
                    for k in 0..ch {
                        e.process(k, &mut w[k][r.clone()]);
                    }
                }
                Proc::Bass(e, _) => {
                    for k in 0..ch {
                        e.process(k, &mut w[k][r.clone()]);
                    }
                }
                Proc::Width(x, _) => {
                    let (w0, w1) = w.split_at_mut(1);
                    x.process(&mut w0[0][r.clone()], &mut w1[0][r.clone()]);
                }
                Proc::Mod(x, ..) => {
                    let (w0, w1) = w.split_at_mut(1);
                    x.process(&mut w0[0][r.clone()], &mut w1[0][r.clone()], pos);
                }
                Proc::Trem(x, ..) => {
                    let (w0, w1) = w.split_at_mut(1);
                    x.process(&mut w0[0][r.clone()], &mut w1[0][r.clone()], pos);
                }
                Proc::Harm(x, ..) => {
                    let (w0, w1) = w.split_at_mut(1);
                    x.process(&mut w0[0][r.clone()], &mut w1[0][r.clone()]);
                }
                Proc::Dbl(x, ..) => {
                    let (w0, w1) = w.split_at_mut(1);
                    x.process(&mut w0[0][r.clone()], &mut w1[0][r.clone()], pos);
                }
            }
        }
        // The dry path of a block with latency is its input that much later.
        if let Some(line) = &mut self.dry_line {
            for c in 0..ch {
                for (i, d) in dry[c][r.clone()].iter_mut().enumerate() {
                    *d = line.tap(c, i, *d);
                }
            }
            line.advance(len);
            if off {
                for c in 0..ch {
                    w[c][r.clone()].copy_from_slice(&dry[c][r.clone()]);
                }
                return;
            }
        }
        if !on {
            for c in 0..ch {
                for (i, (v, d)) in w[c][r.clone()].iter_mut().zip(&dry[c][r.clone()]).enumerate() {
                    // Exact at both ends, so a piece that starts after the glide equals one that contains its end.
                    let by = self.fade.buf[i];
                    if by >= 1.0 {
                        *v = *d;
                    } else if by > 0.0 {
                        let d = f64::from(*d);
                        *v = (d + (1.0 - by) * (f64::from(*v) - d)) as f32;
                    }
                }
            }
        }
    }
}
