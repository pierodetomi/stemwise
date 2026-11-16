//! Movement: chorus, flanger, phaser and tremolo / autopan (FX-20, FX-24).
//!
//! **Phase rule.** Every LFO phase depends only on the position in the song: each `process` gets `pos`, the
//! timeline frame of its first sample. The processor keeps `next_pos`; when `pos != next_pos` (first call after
//! `reset`, seek, loop) the phase is set to `frac(rate_hz * pos / rate)` and the parameter smoothers snap to
//! their targets; otherwise the phase accumulates per sample. A render starting at 30 s therefore gives the same
//! samples as the full render from 30 s on, at constant settings (delay lines and feedback state aside).
//!
//! **Modulation** ([`Modulation`]). Per channel an LFO `sin(2 pi p)`; the right channel's phase is offset by
//! `spread_deg`. Delay reads are linearly interpolated (no zipper). Output `dry * (1 - mix) + wet * mix`.
//! - Chorus: 1..3 voices, bases 12, 16 and 20 ms, voice phases spaced by `1/voices` of a cycle, delay
//!   `base + depth * 3 ms * lfo`; wet is the mean of the voices.
//! - Flanger: one delay `0.5 ms + depth * 7.5 ms * (0.5 + 0.5 lfo)`; the delayed signal feeds back
//!   (`feedback`, -0.9..0.9) into the delay line.
//! - Phaser: `stages` (4, 6, 8, 12) first-order all-passes `H = (a + z^-1) / (1 + a z^-1)`,
//!   `a = (t - 1) / (t + 1)`, `t = tan(pi f / rate)`, `f = 200 Hz * 20^(depth * (0.5 + 0.5 lfo))` (200 Hz to 4 kHz,
//!   exponential sweep); the chain output feeds back one sample late.
//!
//! Neutral (FX-24): `mix` 0 (the chorus at depth 0 is still a static delay, so the neutral position of the
//! module is `mix` 0). Latency 0. `depth`, `mix`, `feedback` and the stereo offset are smoothed per sample (20 ms).
//! Bound: with `|feedback| <= 0.9` the wet path has gain at most `1 / (1 - 0.9) = 10`.
//!
//! **Tremolo / autopan** ([`Tremolo`]). Unipolar LFO `u = 0.5 + 0.5 lfo` with `lfo` in -1..1 (sine, triangle, or a
//! square made of `tanh(5 sin) / tanh(5)` for smoothed edges), all starting at 0 for phase 0.
//! - Tremolo: `gain = 1 - depth * (1 - u)` (between `1 - depth` and 1), right channel phase offset `phase_deg`.
//! - Autopan: position `p = depth * lfo` (-1 left, +1 right), angle `theta = (p + 1) pi / 4`,
//!   `L' = sqrt2 cos(theta) L`, `R' = sqrt2 sin(theta) R`: equal power (`gL^2 + gR^2 = 2`), so the total power of
//!   a centred signal is constant (exactly), both gains are 1 at the centre.
//!
//! Neutral: tremolo `depth` 0 and autopan `depth` 0 are the identity. Latency 0.
//!
//! **Properties verified (tests).** Phase rule (0..40 s against 30..40 s, <= 1e-5), block-size invariance
//! (<= 1e-6), identity at the neutral positions (<= 1e-6), bounded output on noise at extreme settings,
//! finite output with parameters changing every 32 samples, the phaser notch moving, the tremolo envelope and the
//! alternating energy of the autopan.

use crate::util::{flush, one_pole};
use std::f64::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModMode {
    Chorus,
    Flanger,
    Phaser,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModParams {
    pub mode: ModMode,
    /// LFO rate in Hz, 0.02..10.
    pub rate_hz: f64,
    /// Modulation depth, 0..1.
    pub depth: f64,
    /// Feedback, -0.9..0.9 (flanger and phaser).
    pub feedback: f64,
    /// Chorus voices, 1..3.
    pub voices: u8,
    /// Phaser stages: 4, 6, 8 or 12.
    pub stages: u8,
    /// Phase offset of the right channel's LFO in degrees, 0..180.
    pub spread_deg: f64,
    /// Dry/wet, 0..1.
    pub mix: f64,
}

const MAX_STAGES: usize = 12;
const CHORUS_BASE_MS: [f64; 3] = [12.0, 16.0, 20.0];
const CHORUS_DEPTH_MS: f64 = 3.0;
const FLANGER_MIN_MS: f64 = 0.5;
const FLANGER_SPAN_MS: f64 = 7.5;
const PHASER_FMIN: f64 = 200.0;
const PHASER_RATIO: f64 = 20.0;
const SMOOTH_MS: f64 = 20.0;

#[inline]
fn frac(x: f64) -> f64 {
    x - x.floor()
}

/// Chorus, flanger and phaser (FX-20).
pub struct Modulation {
    rate: f64,
    p: ModParams,
    buf: [Vec<f64>; 2],
    mask: usize,
    w: usize,
    phase: f64,
    /// Phase handed in by the host for the next start (see `set_phase`).
    given: Option<(f64, i64)>,
    next_pos: i64,
    snap: bool,
    depth: f64,
    mix: f64,
    fb: f64,
    spread: f64,
    k: f64,
    ap_z: [[f64; MAX_STAGES]; 2],
    fb_state: [f64; 2],
}

impl Modulation {
    pub fn new(rate: f64) -> Self {
        let size = ((rate * 0.05) as usize + 8).next_power_of_two();
        Modulation {
            rate,
            p: ModParams {
                mode: ModMode::Chorus,
                rate_hz: 0.5,
                depth: 0.5,
                feedback: 0.0,
                voices: 2,
                stages: 4,
                spread_deg: 90.0,
                mix: 0.0,
            },
            buf: [vec![0.0; size], vec![0.0; size]],
            mask: size - 1,
            w: 0,
            phase: 0.0,
            given: None,
            next_pos: 0,
            snap: true,
            depth: 0.0,
            mix: 0.0,
            fb: 0.0,
            spread: 0.0,
            k: one_pole(SMOOTH_MS, rate),
            ap_z: [[0.0; MAX_STAGES]; 2],
            fb_state: [0.0; 2],
        }
    }

    pub fn set_params(&mut self, p: &ModParams) {
        self.p = ModParams {
            mode: p.mode,
            rate_hz: p.rate_hz.clamp(0.02, 10.0),
            depth: p.depth.clamp(0.0, 1.0),
            feedback: p.feedback.clamp(-0.9, 0.9),
            voices: p.voices.clamp(1, 3),
            stages: match p.stages {
                0..=4 => 4,
                5..=6 => 6,
                7..=8 => 8,
                _ => 12,
            },
            spread_deg: p.spread_deg.clamp(0.0, 180.0),
            mix: p.mix.clamp(0.0, 1.0),
        };
    }

    /// The LFO phase (in cycles) at the timeline frame `at`, for the next `process` after a reset or a jump (which
    /// may start a few frames later, at the current rate). A host whose rate changes along the song (automation)
    /// gives the cycles run since the song's start, so the phase stays a function of the position alone (FX-20);
    /// without it the phase is `rate x position`, right for a fixed rate.
    pub fn set_phase(&mut self, cycles: f64, at: i64) {
        self.given = Some((frac(cycles), at));
        self.snap = true;
    }

    /// Clears the delay lines and the filter states; the next `process` re-derives the phase from its position.
    pub fn reset(&mut self) {
        for b in &mut self.buf {
            b.fill(0.0);
        }
        self.ap_z = [[0.0; MAX_STAGES]; 2];
        self.fb_state = [0.0; 2];
        self.w = 0;
        self.snap = true;
    }

    /// Linearly interpolated read `d` samples (>= 1) behind the write position.
    #[inline]
    fn read(&self, ch: usize, d: f64) -> f64 {
        let i = d as usize;
        let fr = d - i as f64;
        let a = self.buf[ch][self.w.wrapping_sub(i) & self.mask];
        let b = self.buf[ch][self.w.wrapping_sub(i + 1) & self.mask];
        a + (b - a) * fr
    }

    /// Processes a block in place; `pos` is the timeline frame of its first sample.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], pos: i64) {
        let n = l.len().min(r.len());
        let inc = self.p.rate_hz / self.rate;
        if self.snap || pos != self.next_pos {
            self.phase = match self.given.take() {
                Some((cycles, at)) => frac(cycles + self.p.rate_hz * (pos - at) as f64 / self.rate),
                None => frac(self.p.rate_hz * pos as f64 / self.rate),
            };
            self.depth = self.p.depth;
            self.mix = self.p.mix;
            self.fb = self.p.feedback;
            self.spread = self.p.spread_deg / 360.0;
            self.snap = false;
        }
        let ms = self.rate * 0.001;
        let voices = usize::from(self.p.voices);
        let stages = usize::from(self.p.stages);
        for i in 0..n {
            self.depth += (self.p.depth - self.depth) * self.k;
            self.mix += (self.p.mix - self.mix) * self.k;
            self.fb += (self.p.feedback - self.fb) * self.k;
            self.spread += (self.p.spread_deg / 360.0 - self.spread) * self.k;
            let mut io = [f64::from(l[i]), f64::from(r[i])];
            for ch in 0..2 {
                let x = io[ch];
                let pc = self.phase + if ch == 1 { self.spread } else { 0.0 };
                let wet = match self.p.mode {
                    ModMode::Chorus => {
                        let mut acc = 0.0;
                        for v in 0..voices {
                            let lfo = (TAU * (pc + v as f64 / voices as f64)).sin();
                            let d = (CHORUS_BASE_MS[v] + self.depth * CHORUS_DEPTH_MS * lfo) * ms;
                            acc += self.read(ch, d);
                        }
                        self.buf[ch][self.w & self.mask] = x;
                        acc / voices as f64
                    }
                    ModMode::Flanger => {
                        let lfo = (TAU * pc).sin();
                        let d = (FLANGER_MIN_MS + self.depth * FLANGER_SPAN_MS * (0.5 + 0.5 * lfo)) * ms;
                        let wet = self.read(ch, d);
                        self.buf[ch][self.w & self.mask] = flush(x + self.fb * wet);
                        wet
                    }
                    ModMode::Phaser => {
                        let lfo = (TAU * pc).sin();
                        let f = PHASER_FMIN * PHASER_RATIO.powf(self.depth * (0.5 + 0.5 * lfo));
                        let t = (PI * f / self.rate).tan();
                        let a = (t - 1.0) / (t + 1.0);
                        let mut y = x + self.fb * self.fb_state[ch];
                        for z in self.ap_z[ch].iter_mut().take(stages) {
                            let o = a * y + *z;
                            *z = flush(y - a * o);
                            y = o;
                        }
                        self.fb_state[ch] = flush(y);
                        y
                    }
                };
                io[ch] = x * (1.0 - self.mix) + wet * self.mix;
            }
            self.w = self.w.wrapping_add(1);
            self.phase += inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            l[i] = io[0] as f32;
            r[i] = io[1] as f32;
        }
        self.next_pos = pos + n as i64;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TremoloMode {
    Tremolo,
    Autopan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LfoShape {
    Sine,
    Triangle,
    /// Square with smoothed edges.
    Square,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TremoloParams {
    pub mode: TremoloMode,
    /// LFO rate in Hz, 0.1..20.
    pub rate_hz: f64,
    /// Depth, 0..1.
    pub depth: f64,
    pub shape: LfoShape,
    /// Right channel offset in degrees (tremolo mode), 0..180.
    pub phase_deg: f64,
}

/// LFO in -1..1 for a phase in cycles, 0 at phase 0 and rising.
#[inline]
fn lfo(shape: LfoShape, p: f64) -> f64 {
    let s = (TAU * p).sin();
    match shape {
        LfoShape::Sine => s,
        LfoShape::Triangle => 2.0 * (2.0 * frac(p - 0.25) - 1.0).abs() - 1.0,
        LfoShape::Square => (5.0 * s).tanh() / 5f64.tanh(),
    }
}

/// Tremolo and autopan (FX-20).
pub struct Tremolo {
    rate: f64,
    p: TremoloParams,
    phase: f64,
    /// Phase handed in by the host for the next start (see `set_phase`).
    given: Option<(f64, i64)>,
    next_pos: i64,
    snap: bool,
    depth: f64,
    offset: f64,
    k: f64,
}

impl Tremolo {
    pub fn new(rate: f64) -> Self {
        Tremolo {
            rate,
            p: TremoloParams {
                mode: TremoloMode::Tremolo,
                rate_hz: 4.0,
                depth: 0.0,
                shape: LfoShape::Sine,
                phase_deg: 0.0,
            },
            phase: 0.0,
            given: None,
            next_pos: 0,
            snap: true,
            depth: 0.0,
            offset: 0.0,
            k: one_pole(SMOOTH_MS, rate),
        }
    }

    pub fn set_params(&mut self, p: &TremoloParams) {
        self.p = TremoloParams {
            rate_hz: p.rate_hz.clamp(0.1, 20.0),
            depth: p.depth.clamp(0.0, 1.0),
            phase_deg: p.phase_deg.clamp(0.0, 180.0),
            ..*p
        };
    }

    /// As [`Modulation::set_phase`].
    pub fn set_phase(&mut self, cycles: f64, at: i64) {
        self.given = Some((frac(cycles), at));
        self.snap = true;
    }

    pub fn reset(&mut self) {
        self.snap = true;
    }

    /// Processes a block in place; `pos` is the timeline frame of its first sample.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], pos: i64) {
        let n = l.len().min(r.len());
        let inc = self.p.rate_hz / self.rate;
        if self.snap || pos != self.next_pos {
            self.phase = match self.given.take() {
                Some((cycles, at)) => frac(cycles + self.p.rate_hz * (pos - at) as f64 / self.rate),
                None => frac(self.p.rate_hz * pos as f64 / self.rate),
            };
            self.depth = self.p.depth;
            self.offset = self.p.phase_deg / 360.0;
            self.snap = false;
        }
        for i in 0..n {
            self.depth += (self.p.depth - self.depth) * self.k;
            self.offset += (self.p.phase_deg / 360.0 - self.offset) * self.k;
            match self.p.mode {
                TremoloMode::Tremolo => {
                    let gl = 1.0 - self.depth * (0.5 - 0.5 * lfo(self.p.shape, self.phase));
                    let gr = 1.0 - self.depth * (0.5 - 0.5 * lfo(self.p.shape, self.phase + self.offset));
                    l[i] = (f64::from(l[i]) * gl) as f32;
                    r[i] = (f64::from(r[i]) * gr) as f32;
                }
                TremoloMode::Autopan => {
                    let pan = self.depth * lfo(self.p.shape, self.phase);
                    let theta = (pan + 1.0) * PI / 4.0;
                    l[i] = (f64::from(l[i]) * std::f64::consts::SQRT_2 * theta.cos()) as f32;
                    r[i] = (f64::from(r[i]) * std::f64::consts::SQRT_2 * theta.sin()) as f32;
                }
            }
            self.phase += inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
        self.next_pos = pos + n as i64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::xorshift32;

    const FS: f64 = 48000.0;

    fn noise(seed: u32, n: usize, amp: f64) -> Vec<f32> {
        xorshift32(seed, n).iter().map(|v| (v * amp) as f32).collect()
    }

    fn sine(freq: f64, amp: f64, n: usize) -> Vec<f32> {
        (0..n).map(|t| (amp * (TAU * freq * t as f64 / FS).sin()) as f32).collect()
    }

    fn mp(mode: ModMode) -> ModParams {
        ModParams { mode, rate_hz: 0.7, depth: 0.8, feedback: 0.5, voices: 3, stages: 8, spread_deg: 90.0, mix: 0.5 }
    }

    fn run_mod(p: &ModParams, l: &[f32], r: &[f32], chunk: usize, start: i64) -> (Vec<f32>, Vec<f32>) {
        let mut m = Modulation::new(FS);
        m.set_params(p);
        let (mut ol, mut or) = (l.to_vec(), r.to_vec());
        let mut i = 0;
        while i < ol.len() {
            let e = (i + chunk).min(ol.len());
            m.process(&mut ol[i..e], &mut or[i..e], start + i as i64);
            i = e;
        }
        (ol, or)
    }

    fn maxdiff(a: &[f32], b: &[f32]) -> f64 {
        a.iter().zip(b).map(|(x, y)| f64::from(x - y).abs()).fold(0.0, f64::max)
    }

    /// req: FX-20, FX-24 — the LFO phase follows the song position: a render from 30 s equals the full render
    /// (compared from 30.5 s, after the delay lines and feedback have filled; 1e-5 covers the f64 phase drift
    /// accumulated over 1.4 million samples and the f32 rounding).
    #[test]
    fn phase_follows_position() {
        let n = 40 * 48000;
        let (l, r) = (noise(1, n, 0.5), noise(2, n, 0.5));
        for mode in [ModMode::Chorus, ModMode::Flanger, ModMode::Phaser] {
            let p = mp(mode);
            let (fl, fr) = run_mod(&p, &l, &r, 32, 0);
            let s = 30 * 48000;
            let (sl, sr) = run_mod(&p, &l[s..], &r[s..], 32, s as i64);
            let c = 24000;
            assert!(maxdiff(&fl[s + c..], &sl[c..]) <= 1e-5, "{mode:?} left");
            assert!(maxdiff(&fr[s + c..], &sr[c..]) <= 1e-5, "{mode:?} right");
        }
    }

    /// req: FX-20, FX-24 — mix 0 is the identity for every mode (<= 1e-6, f32 rounding).
    #[test]
    fn mix_zero_is_identity() {
        let (l, r) = (noise(3, 20000, 0.8), noise(4, 20000, 0.8));
        for mode in [ModMode::Chorus, ModMode::Flanger, ModMode::Phaser] {
            let mut p = mp(mode);
            p.mix = 0.0;
            let (ol, or) = run_mod(&p, &l, &r, 32, 0);
            assert!(maxdiff(&ol, &l) <= 1e-6 && maxdiff(&or, &r) <= 1e-6, "{mode:?}");
        }
    }

    /// req: FX-20, FX-24 — the output does not depend on the block size (<= 1e-6).
    #[test]
    fn block_size_invariance() {
        let (l, r) = (noise(5, 30000, 0.5), noise(6, 30000, 0.5));
        for mode in [ModMode::Chorus, ModMode::Flanger, ModMode::Phaser] {
            let p = mp(mode);
            let (a, _) = run_mod(&p, &l, &r, 32, 100);
            for chunk in [1000, 7] {
                let (b, _) = run_mod(&p, &l, &r, chunk, 100);
                assert!(maxdiff(&a, &b) <= 1e-6, "{mode:?} chunk {chunk}");
            }
        }
    }

    /// req: FX-20, FX-24 — full-scale noise at extreme settings stays finite and within 10.5: the wet path has
    /// gain <= 1 / (1 - 0.9) = 10 (linear interpolation and all-passes do not add gain beyond that).
    #[test]
    fn bounded_on_noise() {
        let (l, r) = (noise(7, 96000, 1.0), noise(8, 96000, 1.0));
        for mode in [ModMode::Chorus, ModMode::Flanger, ModMode::Phaser] {
            for fb in [0.9, -0.9] {
                let p = ModParams {
                    mode,
                    rate_hz: 10.0,
                    depth: 1.0,
                    feedback: fb,
                    voices: 3,
                    stages: 12,
                    spread_deg: 180.0,
                    mix: 1.0,
                };
                let (ol, or) = run_mod(&p, &l, &r, 32, 0);
                for v in ol.iter().chain(or.iter()) {
                    assert!(v.is_finite() && v.abs() <= 10.5, "{mode:?} {v}");
                }
            }
        }
    }

    /// req: FX-20, FX-24 — parameters changing every 32 samples keep the output finite and bounded (<= 10.5).
    #[test]
    fn stable_under_parameter_changes() {
        let (mut l, mut r) = (noise(9, 5 * 48000, 1.0), noise(10, 5 * 48000, 1.0));
        let mut m = Modulation::new(FS);
        let mut s = xorshift32(11, 5 * 48000 / 32 + 1);
        let modes = [ModMode::Chorus, ModMode::Flanger, ModMode::Phaser];
        for (b, (cl, cr)) in l.chunks_mut(32).zip(r.chunks_mut(32)).enumerate() {
            let u = s[b] * 0.5 + 0.5;
            let p = ModParams {
                mode: modes[(u * 3.0) as usize % 3],
                rate_hz: 0.02 + 9.98 * u,
                depth: u,
                feedback: 1.8 * u - 0.9,
                voices: 1 + (u * 3.0) as u8 % 3,
                stages: [4, 6, 8, 12][(u * 4.0) as usize % 4],
                spread_deg: 180.0 * u,
                mix: u,
            };
            m.set_params(&p);
            m.process(cl, cr, (b * 32) as i64);
        }
        s.clear();
        for v in l.iter().chain(r.iter()) {
            assert!(v.is_finite() && v.abs() <= 10.5);
        }
    }

    /// req: FX-20 — flanger with feedback 0.9 on a sine stays bounded by the geometric series (<= 10 plus margin).
    #[test]
    fn flanger_feedback_bounded() {
        let x = sine(300.0, 1.0, 96000);
        let p = ModParams {
            mode: ModMode::Flanger,
            rate_hz: 0.5,
            depth: 1.0,
            feedback: 0.9,
            voices: 1,
            stages: 4,
            spread_deg: 0.0,
            mix: 1.0,
        };
        let (ol, _) = run_mod(&p, &x, &x, 32, 0);
        assert!(ol.iter().all(|v| v.is_finite() && v.abs() <= 10.5));
    }

    /// req: FX-20 — the phaser notch moves: the level of a fixed 800 Hz sine, measured over 50 ms windows, swings by
    /// more than 6 dB over one LFO cycle, and not at all with depth 0 (<= 0.1 dB).
    #[test]
    fn phaser_notch_moves() {
        let x = sine(800.0, 0.5, 2 * 48000);
        let level_swing = |depth: f64| {
            let p = ModParams {
                mode: ModMode::Phaser,
                rate_hz: 1.0,
                depth,
                feedback: 0.7,
                voices: 1,
                stages: 4,
                spread_deg: 0.0,
                mix: 0.5,
            };
            let (ol, _) = run_mod(&p, &x, &x, 32, 0);
            let w = 2400;
            let lv: Vec<f64> = ol[48000..]
                .chunks(w)
                .filter(|c| c.len() == w)
                .map(|c| 10.0 * (c.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / w as f64).log10())
                .collect();
            lv.iter().cloned().fold(f64::MIN, f64::max) - lv.iter().cloned().fold(f64::MAX, f64::min)
        };
        let sw = level_swing(1.0);
        assert!(sw > 6.0, "{sw}");
        assert!(level_swing(0.0) < 0.1);
    }

    /// req: FX-20 — a chorus of a sine adds a delayed, modulated copy: output differs from the dry at mix 0.5 and
    /// its power stays within -6..+3 dB of the dry (a comb of two equal copies may cancel partly; sanity bound).
    #[test]
    fn chorus_changes_signal() {
        let x = sine(1000.0, 0.5, 48000);
        let p = mp(ModMode::Chorus);
        let (ol, _) = run_mod(&p, &x, &x, 32, 0);
        assert!(maxdiff(&ol[24000..], &x[24000..]) > 0.05);
        let pw = |v: &[f32]| v.iter().map(|a| f64::from(*a).powi(2)).sum::<f64>() / v.len() as f64;
        let db = 10.0 * (pw(&ol[24000..]) / pw(&x[24000..])).log10();
        assert!(db > -6.0 && db < 3.0, "{db}");
    }

    fn tp(mode: TremoloMode, shape: LfoShape, depth: f64, rate_hz: f64) -> TremoloParams {
        TremoloParams { mode, rate_hz, depth, shape, phase_deg: 0.0 }
    }

    fn run_trem(p: &TremoloParams, l: &[f32], r: &[f32], chunk: usize, start: i64) -> (Vec<f32>, Vec<f32>) {
        let mut t = Tremolo::new(FS);
        t.set_params(p);
        let (mut ol, mut or) = (l.to_vec(), r.to_vec());
        let mut i = 0;
        while i < ol.len() {
            let e = (i + chunk).min(ol.len());
            t.process(&mut ol[i..e], &mut or[i..e], start + i as i64);
            i = e;
        }
        (ol, or)
    }

    /// req: FX-20 — tremolo depth 1 with a 4 Hz sine LFO on a constant 1.0 gives the envelope
    /// `0.5 + 0.5 sin(2 pi 4 t)` (<= 1e-6, f32 rounding).
    #[test]
    fn tremolo_envelope() {
        let one = vec![1.0f32; 48000];
        let (ol, or) = run_trem(&tp(TremoloMode::Tremolo, LfoShape::Sine, 1.0, 4.0), &one, &one, 32, 0);
        for t in 0..48000 {
            let e = 0.5 + 0.5 * (TAU * 4.0 * t as f64 / FS).sin();
            assert!((f64::from(ol[t]) - e).abs() <= 1e-6 && (f64::from(or[t]) - e).abs() <= 1e-6, "t {t}");
        }
    }

    /// req: FX-20 — the right channel of the tremolo is offset by `phase_deg`: at 180 degrees the two gains sum to 1.
    #[test]
    fn tremolo_phase_offset() {
        let one = vec![1.0f32; 24000];
        let mut p = tp(TremoloMode::Tremolo, LfoShape::Sine, 1.0, 4.0);
        p.phase_deg = 180.0;
        let (ol, or) = run_trem(&p, &one, &one, 32, 0);
        for t in 0..24000 {
            assert!((ol[t] + or[t] - 1.0).abs() <= 1e-6);
        }
    }

    /// req: FX-20, FX-24 — depth 0 is the identity for tremolo and autopan; the shapes keep the gain in 1-depth..1.
    #[test]
    fn tremolo_neutral_and_shapes() {
        let (l, r) = (noise(12, 20000, 0.7), noise(13, 20000, 0.7));
        for mode in [TremoloMode::Tremolo, TremoloMode::Autopan] {
            let (ol, or) = run_trem(&tp(mode, LfoShape::Sine, 0.0, 5.0), &l, &r, 32, 0);
            assert!(maxdiff(&ol, &l) <= 1e-6 && maxdiff(&or, &r) <= 1e-6);
        }
        let one = vec![1.0f32; 48000];
        for shape in [LfoShape::Sine, LfoShape::Triangle, LfoShape::Square] {
            let (ol, _) = run_trem(&tp(TremoloMode::Tremolo, shape, 0.6, 3.0), &one, &one, 32, 0);
            let (mn, mx) = ol.iter().fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
            assert!(mn >= 0.4 - 1e-4 && mx <= 1.0 + 1e-6 && mn < 0.45 && mx > 0.99, "{shape:?} {mn} {mx}");
        }
    }

    /// req: FX-20 — autopan alternates the L/R energy at the LFO rate and keeps the total level of a centred signal
    /// constant (power gL^2 + gR^2 = 2 exactly; tolerance 0.5 dB asked, 1e-5 achieved).
    #[test]
    fn autopan_alternates_and_keeps_level() {
        let one = vec![1.0f32; 48000];
        let (ol, or) = run_trem(&tp(TremoloMode::Autopan, LfoShape::Sine, 1.0, 2.0), &one, &one, 32, 0);
        for t in 0..48000 {
            let p = f64::from(ol[t]).powi(2) + f64::from(or[t]).powi(2);
            assert!((p - 2.0).abs() <= 1e-4, "t {t} {p}");
        }
        // 2 Hz: quarter cycle (t = 6000) pans right, three quarters (t = 18000) pans left.
        assert!(or[6000] > 1.3 && ol[6000] < 0.1);
        assert!(ol[18000] > 1.3 && or[18000] < 0.1);
    }

    /// req: FX-20, FX-24 — tremolo phase follows the position (render from 10 s equals the full render, <= 1e-5) and the
    /// output does not depend on the block size (<= 1e-6).
    #[test]
    fn tremolo_phase_and_blocks() {
        let n = 20 * 48000;
        let (l, r) = (noise(14, n, 0.5), noise(15, n, 0.5));
        for mode in [TremoloMode::Tremolo, TremoloMode::Autopan] {
            let p = tp(mode, LfoShape::Triangle, 0.9, 5.3);
            let (fl, _) = run_trem(&p, &l, &r, 32, 0);
            let s = 10 * 48000;
            let (sl, _) = run_trem(&p, &l[s..], &r[s..], 32, s as i64);
            assert!(maxdiff(&fl[s..], &sl) <= 1e-5);
            let (a, _) = run_trem(&p, &l[..30000], &r[..30000], 7, 0);
            let (b, _) = run_trem(&p, &l[..30000], &r[..30000], 1000, 0);
            assert!(maxdiff(&a, &b) <= 1e-6);
        }
    }

    /// req: FX-20, FX-24 — noise at extreme settings and changing parameters stays finite and bounded by sqrt2 gain.
    #[test]
    fn tremolo_bounded() {
        let (mut l, mut r) = (noise(16, 96000, 1.0), noise(17, 96000, 1.0));
        let mut t = Tremolo::new(FS);
        let mut s = xorshift32(18, 96000 / 32 + 1);
        for (b, (cl, cr)) in l.chunks_mut(32).zip(r.chunks_mut(32)).enumerate() {
            let u = s[b] * 0.5 + 0.5;
            t.set_params(&TremoloParams {
                mode: if u > 0.5 { TremoloMode::Autopan } else { TremoloMode::Tremolo },
                rate_hz: 0.1 + 19.9 * u,
                depth: u,
                shape: [LfoShape::Sine, LfoShape::Triangle, LfoShape::Square][(u * 3.0) as usize % 3],
                phase_deg: 180.0 * u,
            });
            t.process(cl, cr, (b * 32) as i64);
        }
        s.clear();
        assert!(l.iter().chain(r.iter()).all(|v| v.is_finite() && v.abs() <= 1.4143));
    }
}
