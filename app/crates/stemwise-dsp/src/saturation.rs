//! Soft saturation with 4x oversampling (FX-4).
//!
//! **Algorithm.** Per sample: input gain `g = 10^(drive_db/20)`, shaper `y = shape(g * x) * c(g)` with
//! `shape(u) = 2 tanh(u / 2)` (unity small-signal slope; ceiling 2 keeps the knee soft enough for alias rejection) evaluated at 4x
//! through [`Oversampler4`] (257-tap Kaiser linear-phase FIR, ~90 dB stopband, see that module), then
//! `out = dry_delayed * (1 - mix) + y * mix`.
//!
//! **Level compensation.** `c(g) = A / shape(g * A)` with reference peak `A = 10^(-18/20)` (the level of a
//! -18 dBFS sine), computed analytically per sample from the smoothed gain: the shaped level at the reference
//! amplitude equals the unshaped one. At drive 0 dB `c` is about 1.0, so low-level signals pass transparently.
//!
//! **Smoothing.** `drive` (linear gain) and `mix` are smoothed per sample with a 5 ms one-pole towards the
//! targets set by `set_params`, independently per channel; output is therefore independent of block size.
//!
//! **Latency.** [`Saturation::LATENCY`] = [`Oversampler4::LATENCY`] = 64 samples, integer; the dry path goes
//! through a 64-sample delay so that `mix` blends phase-aligned signals. Engine compensates it (ENG-5).
//!
//! **Character (FX-18).** [`Character::Soft`] is the algorithm above, bit for bit (projects saved before the
//! character existed keep their render hash). The others reshape the same 4x path and use the same reference-level
//! compensation, computed from their own static curve:
//! - `Tape`: the soft curve followed, in the wet path only, by a one-pole low-pass at 12 kHz (at the 4x rate), the
//!   gentle fall of the highs of a tape machine; peaks are as rounded as in `Soft`.
//! - `Tube`: asymmetric: a bias of 0.3 (in units of the ceiling) before the soft curve, the bias response removed
//!   and the slope renormalised to 1, then a one-pole high-pass at 10 Hz (4x rate) that takes out the DC the
//!   asymmetry creates. Even harmonics dominate the added colour.
//! - `Transistor`: `u / (1 + u^4)^(1/4)`, ceiling 1: a hard knee, odd harmonics, brighter than `Soft`.
//!
//! Switching character on the fly is not crossfaded (the choice is a discrete setting, not an automation target).
//!
//! **Properties verified (tests).** Exact latency (peak and mix 0), transparency at -40 dBFS (+-0.1 dB), level
//! within +-1 dB for drive 0..18 dB at -18 dBFS, 3rd harmonic present at drive 18 dB, aliasing of the 3rd harmonic
//! of 15 kHz at 48 kHz below -60 dB, no NaN/inf at drive 24 dB on full-scale noise, block-size invariance
//! (<= 1e-6), independent channels. Process does not allocate, lock or perform I/O.

use crate::oversample::Oversampler4;
use crate::util::{db_to_gain, flush, one_pole};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SaturationParams {
    /// Input drive in dB, 0..24.
    pub drive_db: f64,
    /// Dry/wet, 0..1.
    pub mix: f64,
    /// Colour of the shaper (FX-18); `Soft` is the original curve.
    pub character: Character,
}

/// Colour of the saturation (FX-18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Character {
    /// `2 tanh(u / 2)`: the original, transparent curve.
    #[default]
    Soft,
    /// Rounded peaks and a gentle fall of the highs.
    Tape,
    /// Asymmetric: even harmonics.
    Tube,
    /// Harder clipping: odd harmonics.
    Transistor,
}

const REF_PEAK: f64 = 0.125_892_541_179_416_72; // 10^(-18/20)
/// Saturation ceiling of the shaper (linear units). `SAT * tanh(u / SAT)` has unity small-signal slope and
/// a knee soft enough that harmonics folding back at 4x stay below -60 dB for 15 kHz at -6 dBFS, drive 18 dB
/// (plain `tanh`, SAT = 1, aliased at only -46 dB).
const SAT: f64 = 2.0;

/// tanh by Lambert's continued fraction (7th order rational), within 1e-4 of `tanh` for |x| < 4.97 (1.5e-5 at 4, 9.6e-5 at 4.97; evaluated in f32) and clamped
/// to ±1 beyond, where it reaches 1 − 1e-7: smooth, monotonic and an order of magnitude cheaper (A-022).
#[inline]
fn fast_tanh(x: f64) -> f64 {
    let x = x as f32;
    if x.abs() >= 4.97 {
        return f64::from(x.signum());
    }
    let x2 = x * x;
    let n = x * (135_135.0 + x2 * (17_325.0 + x2 * (378.0 + x2)));
    let d = 135_135.0 + x2 * (62_370.0 + x2 * (3_150.0 + x2 * 28.0));
    f64::from((n / d).clamp(-1.0, 1.0))
}

#[inline]
fn shape(u: f64) -> f64 {
    SAT * fast_tanh(u / SAT)
}

/// The shaper in f32 on the four oversampled points of a sample (vectorisable).
#[inline]
fn shape4(v: &mut [f32; 4], g: f32, comp: f32) {
    let s = SAT as f32;
    for x in v.iter_mut() {
        let u = (g * *x / s).clamp(-4.97, 4.97);
        let u2 = u * u;
        let n = u * (135_135.0 + u2 * (17_325.0 + u2 * (378.0 + u2)));
        let d = 135_135.0 + u2 * (62_370.0 + u2 * (3_150.0 + u2 * 28.0));
        *x = s * (n / d).clamp(-1.0, 1.0) * comp;
    }
}

/// Tube bias before the soft curve, in units of the curve ceiling.
const TUBE_BIAS: f32 = 0.3;
/// Tape low-pass and tube DC high-pass corners (Hz, at the 4x rate).
const TAPE_LP_HZ: f64 = 12_000.0;
const TUBE_HP_HZ: f64 = 10.0;

/// The soft curve's rational tanh on an already clamped argument (f32, same arithmetic as `shape4`).
#[inline]
fn tanh_r(u: f32) -> f32 {
    let u = u.clamp(-4.97, 4.97);
    let u2 = u * u;
    let n = u * (135_135.0 + u2 * (17_325.0 + u2 * (378.0 + u2)));
    let d = 135_135.0 + u2 * (62_370.0 + u2 * (3_150.0 + u2 * 28.0));
    (n / d).clamp(-1.0, 1.0)
}

/// Static curve (f64) of each character, for the level compensation only.
fn static_shape(character: Character, u: f64) -> f64 {
    match character {
        Character::Soft | Character::Tape => shape(u),
        Character::Tube => {
            let b = f64::from(TUBE_BIAS);
            let slope = 1.0 - fast_tanh(b).powi(2);
            SAT * (fast_tanh(u / SAT + b) - fast_tanh(b)) / slope
        }
        Character::Transistor => {
            let u = u.clamp(-8.0, 8.0);
            u / (1.0 + u.powi(4)).sqrt().sqrt()
        }
    }
}

/// Transistor curve on the four oversampled points.
#[inline]
fn transistor4(v: &mut [f32; 4], g: f32, comp: f32) {
    for x in v.iter_mut() {
        let u = (g * *x).clamp(-8.0, 8.0);
        *x = u / (1.0 + u * u * u * u).sqrt().sqrt() * comp;
    }
}

/// Tube curve on the four oversampled points (before the DC high-pass).
#[inline]
fn tube4(v: &mut [f32; 4], g: f32, comp: f32) {
    let s = SAT as f32;
    let tb = tanh_r(TUBE_BIAS);
    let inv_slope = 1.0 / (1.0 - tb * tb);
    for x in v.iter_mut() {
        *x = s * (tanh_r(g * *x / s + TUBE_BIAS) - tb) * inv_slope * comp;
    }
}

const SMOOTH_MS: f64 = 5.0;

#[derive(Debug, Clone)]
struct Chan {
    os: Oversampler4,
    dry: [f64; Oversampler4::LATENCY],
    dry_pos: usize,
    gain: f64,
    mix: f64,
    /// Level compensation cached for `comp_gain`.
    comp: f64,
    comp_gain: f64,
    /// The oversampled path is idle while the block is fully bypassed (mix 0): it is cleared when it resumes, and
    /// the 5 ms mix ramp fades it in.
    idle: bool,
    /// Character the cached `comp` was computed for.
    comp_char: Character,
    /// Tape low-pass state and tube DC high-pass state (previous input, previous output).
    lp: f32,
    hp_x: f32,
    hp_y: f32,
}

impl Chan {
    fn new() -> Self {
        Self {
            os: Oversampler4::new(),
            dry: [0.0; Oversampler4::LATENCY],
            dry_pos: 0,
            gain: 1.0,
            mix: 1.0,
            comp: 1.0,
            comp_gain: 1.0,
            idle: false,
            comp_char: Character::Soft,
            lp: 0.0,
            hp_x: 0.0,
            hp_y: 0.0,
        }
    }

    /// One wet sample through the oversampled path with the given character.
    #[inline]
    fn wet(&mut self, character: Character, x: f32, g: f32, comp: f32, k: &Coefs) -> f32 {
        let Chan { os, lp, hp_x, hp_y, .. } = self;
        match character {
            Character::Soft => os.process4(x, |v| shape4(v, g, comp)),
            Character::Tape => os.process4(x, |v| {
                shape4(v, g, comp);
                for p in v.iter_mut() {
                    *lp += k.tape * (*p - *lp);
                    *p = *lp;
                }
            }),
            Character::Tube => os.process4(x, |v| {
                tube4(v, g, comp);
                for p in v.iter_mut() {
                    let y = *p - *hp_x + k.tube_r * *hp_y;
                    *hp_x = *p;
                    *hp_y = if y.abs() < 1e-30 { 0.0 } else { y };
                    *p = *hp_y;
                }
            }),
            Character::Transistor => os.process4(x, |v| transistor4(v, g, comp)),
        }
    }
}

/// Filter coefficients of the characters (f32, at the 4x rate).
#[derive(Debug, Clone, Copy)]
struct Coefs {
    tape: f32,
    tube_r: f32,
}

#[derive(Debug, Clone)]
pub struct Saturation {
    ch: [Chan; 2],
    target_gain: f64,
    target_mix: f64,
    coef: f64,
    character: Character,
    k: Coefs,
}

impl Saturation {
    pub const LATENCY: usize = Oversampler4::LATENCY;

    pub fn new(rate: f64) -> Self {
        let r4 = 4.0 * rate;
        let k = Coefs {
            tape: (1.0 - (-std::f64::consts::TAU * TAPE_LP_HZ / r4).exp()) as f32,
            tube_r: (-std::f64::consts::TAU * TUBE_HP_HZ / r4).exp() as f32,
        };
        Self {
            ch: [Chan::new(), Chan::new()],
            target_gain: 1.0,
            target_mix: 1.0,
            coef: one_pole(SMOOTH_MS, rate),
            character: Character::Soft,
            k,
        }
    }

    pub fn set_params(&mut self, p: &SaturationParams) {
        self.target_gain = db_to_gain(p.drive_db.clamp(0.0, 24.0));
        self.target_mix = p.mix.clamp(0.0, 1.0);
        self.character = p.character;
    }

    pub fn reset(&mut self) {
        for c in &mut self.ch {
            c.os.reset();
            c.dry = [0.0; Oversampler4::LATENCY];
            c.dry_pos = 0;
            c.lp = 0.0;
            c.hp_x = 0.0;
            c.hp_y = 0.0;
            c.gain = self.target_gain;
            c.mix = self.target_mix;
            c.idle = false;
        }
    }

    /// In place, channel `ch` (0 or 1). Output is delayed by [`Self::LATENCY`] samples.
    pub fn process(&mut self, ch: usize, buf: &mut [f32]) {
        let c = &mut self.ch[ch];
        let (tg, tm, k) = (self.target_gain, self.target_mix, self.coef);
        let (character, kc) = (self.character, self.k);
        for s in buf.iter_mut() {
            c.gain += k * (tg - c.gain);
            c.mix += k * (tm - c.mix);
            if (tg - c.gain).abs() < 1e-9 {
                c.gain = tg;
            }
            if (tm - c.mix).abs() < 1e-9 {
                c.mix = tm;
            }
            if tm == 0.0 && c.mix < 1e-7 {
                c.mix = 0.0;
                c.idle = true;
            } else if c.idle {
                // Warm the oversampler with the last LATENCY inputs (kept by the dry delay), so the wet path is
                // full again when the 5 ms mix ramp starts (review F3, N2).
                c.os.reset();
                let g0 = c.gain;
                let comp0 = REF_PEAK / static_shape(character, g0 * REF_PEAK);
                c.lp = 0.0;
                c.hp_x = 0.0;
                c.hp_y = 0.0;
                for k in 0..Oversampler4::LATENCY {
                    let x = c.dry[(c.dry_pos + k) % Oversampler4::LATENCY];
                    c.wet(character, x as f32, g0 as f32, comp0 as f32, &kc);
                }
                c.idle = false;
            }
            let g = c.gain;
            if g != c.comp_gain || character != c.comp_char {
                c.comp = REF_PEAK / static_shape(character, g * REF_PEAK);
                c.comp_gain = g;
                c.comp_char = character;
            }
            let comp = c.comp;
            let x = f64::from(*s);
            let wet = if c.idle { 0.0 } else { f64::from(c.wet(character, x as f32, g as f32, comp as f32, &kc)) };
            let dry = c.dry[c.dry_pos];
            c.dry[c.dry_pos] = x;
            c.dry_pos += 1;
            if c.dry_pos == Oversampler4::LATENCY {
                c.dry_pos = 0;
            }
            *s = flush(dry * (1.0 - c.mix) + wet * c.mix) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: f64 = 48000.0;
    const TAU: f64 = 2.0 * std::f64::consts::PI;

    fn sine(freq: f64, db: f64, n: usize) -> Vec<f32> {
        let a = db_to_gain(db);
        (0..n).map(|t| (a * (TAU * freq * t as f64 / FS).sin()) as f32).collect()
    }

    fn make(drive: f64, mix: f64) -> Saturation {
        let mut s = Saturation::new(FS);
        s.set_params(&SaturationParams { drive_db: drive, mix, character: Character::Soft });
        // Settle the smoothers (5 ms) on silence.
        let mut z = vec![0.0f32; 20000];
        s.process(0, &mut z);
        s.process(1, &mut z);
        s.reset();
        s
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
    }

    fn bin(x: &[f32], freq: f64) -> f64 {
        let (mut re, mut im) = (0.0, 0.0);
        for (t, v) in x.iter().enumerate() {
            let ph = TAU * freq * t as f64 / FS;
            re += f64::from(*v) * ph.cos();
            im += f64::from(*v) * ph.sin();
        }
        2.0 * (re * re + im * im).sqrt() / x.len() as f64
    }

    /// req: FX-4, ENG-5
    #[test]
    fn latency_exact() {
        let mut s = make(0.0, 1.0);
        let mut x = vec![0.0f32; 300];
        x[0] = 0.01;
        s.process(0, &mut x);
        let peak = x.iter().enumerate().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs())).unwrap().0;
        assert_eq!(peak, Saturation::LATENCY);

        let mut s = make(0.0, 0.0);
        let inp = sine(1000.0, -6.0, 1000);
        let mut y = inp.clone();
        s.process(0, &mut y);
        for t in 0..y.len() {
            let want = if t >= Saturation::LATENCY { inp[t - Saturation::LATENCY] } else { 0.0 };
            assert!((y[t] - want).abs() <= 1e-6, "t={t}");
        }
    }

    /// req: FX-4
    #[test]
    fn transparent_at_low_level() {
        let mut s = make(0.0, 1.0);
        let inp = sine(1000.0, -40.0, 48000);
        let mut y = inp.clone();
        s.process(0, &mut y);
        let d = 20.0 * (rms(&y[2000..]) / rms(&inp[2000..])).log10();
        assert!(d.abs() < 0.1, "{d} dB");
    }

    /// req: FX-4
    #[test]
    fn level_compensated() {
        for drive in [0.0, 6.0, 12.0, 18.0] {
            let mut s = make(drive, 1.0);
            let inp = sine(1000.0, -18.0, 48000);
            let mut y = inp.clone();
            s.process(0, &mut y);
            let d = 20.0 * (rms(&y[2000..]) / rms(&inp[2000..])).log10();
            assert!(d.abs() < 1.0, "drive {drive}: {d} dB");
        }
    }

    /// req: FX-4
    #[test]
    fn third_harmonic_at_high_drive() {
        let mut s = make(18.0, 1.0);
        let mut y = sine(1000.0, -18.0, 48000 + 2000);
        s.process(0, &mut y);
        let y = &y[2000..];
        let rel = 20.0 * (bin(y, 3000.0) / bin(y, 1000.0)).log10();
        assert!(rel > -40.0, "3rd harmonic {rel} dB");
    }

    /// req: FX-4
    #[test]
    fn aliasing_low() {
        let mut s = make(18.0, 1.0);
        let mut y = sine(15000.0, -6.0, 48000 + 2000);
        s.process(0, &mut y);
        let y = &y[2000..];
        // 3rd harmonic 45 kHz folds to 3 kHz at 48 kHz.
        let rel = 20.0 * (bin(y, 3000.0) / bin(y, 15000.0)).log10();
        assert!(rel < -60.0, "alias {rel} dB");
    }

    /// req: FX-4
    #[test]
    fn no_nan_on_noise() {
        let mut s = make(24.0, 1.0);
        let mut x: Vec<f32> = crate::testsignal::xorshift32(7, 20000).into_iter().map(|v| v as f32).collect();
        s.process(0, &mut x);
        assert!(x.iter().all(|v| v.is_finite() && v.abs() < 4.0));
    }

    /// req: FX-4
    #[test]
    fn block_size_invariant() {
        let inp = sine(440.0, -10.0, 16384);
        let run = |block: usize| {
            let mut s = Saturation::new(FS);
            let mut out = inp.clone();
            for (i, chunk) in out.chunks_mut(block).enumerate() {
                let pos = i * block;
                if pos.is_multiple_of(4096) {
                    let k = (pos / 4096) as f64;
                    s.set_params(&SaturationParams {
                        drive_db: 6.0 * k,
                        mix: 1.0 - 0.2 * k,
                        character: Character::Soft,
                    });
                }
                s.process(0, chunk);
            }
            out
        };
        let (a, b) = (run(64), run(4096));
        let worst = a.iter().zip(&b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert!(worst <= 1e-6, "{worst}");
    }

    /// req: FX-4
    #[test]
    fn channels_independent() {
        let mut s = make(12.0, 1.0);
        let mut l = sine(1000.0, -10.0, 4000);
        let mut r = vec![0.0f32; 4000];
        s.process(0, &mut l);
        s.process(1, &mut r);
        assert!(r.iter().all(|v| *v == 0.0));
        let mut s2 = make(12.0, 1.0);
        let mut l2 = sine(1000.0, -10.0, 4000);
        s2.process(0, &mut l2);
        assert_eq!(l, l2);
    }

    fn golden_signal() -> Vec<f32> {
        let noise = crate::testsignal::xorshift32(11, 20000);
        sine(1000.0, -8.0, 20000).iter().zip(&noise).map(|(a, n)| a + (0.1 * n) as f32).collect()
    }

    fn fnv(x: &[f32]) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for v in x {
            for b in v.to_bits().to_le_bytes() {
                h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
            }
        }
        h
    }

    /// req: FX-18, FX-24
    ///
    /// `Soft` is the algorithm of before the character existed. The checksum was recorded from that code (drive 9 dB,
    /// mix 0.8, a 1 kHz sine plus seeded noise, one call) before this change: any difference changes a render hash.
    #[test]
    fn soft_is_bit_exact() {
        let mut s = Saturation::new(FS);
        s.set_params(&SaturationParams { drive_db: 9.0, mix: 0.8, character: Character::Soft });
        let mut y = golden_signal();
        s.process(0, &mut y);
        assert_eq!(fnv(&y), 0xe836_c7d4_17a4_bd89);
    }

    fn make_c(drive: f64, mix: f64, character: Character) -> Saturation {
        let mut s = Saturation::new(FS);
        s.set_params(&SaturationParams { drive_db: drive, mix, character });
        let mut z = vec![0.0f32; 20000];
        s.process(0, &mut z);
        s.process(1, &mut z);
        s.reset();
        s
    }

    fn run_c(character: Character, drive: f64, freq: f64, db: f64) -> Vec<f32> {
        let mut s = make_c(drive, 1.0, character);
        let mut y = sine(freq, db, 48000 + 2000);
        s.process(0, &mut y);
        y[2000..].to_vec()
    }

    const CHARS: [Character; 4] = [Character::Soft, Character::Tape, Character::Tube, Character::Transistor];

    /// req: FX-18
    ///
    /// Tube: the asymmetry makes a 2nd harmonic where the odd `Soft` curve makes none (only numerical residue):
    /// at least 10 dB more than Soft, and clearly present in absolute terms (> -50 dB re the fundamental).
    #[test]
    fn tube_has_even_harmonics() {
        let soft = run_c(Character::Soft, 12.0, 1000.0, -18.0);
        let tube = run_c(Character::Tube, 12.0, 1000.0, -18.0);
        let h = |y: &[f32]| 20.0 * (bin(y, 2000.0) / bin(y, 1000.0)).log10();
        assert!(h(&tube) > h(&soft) + 10.0, "tube {} soft {}", h(&tube), h(&soft));
        assert!(h(&tube) > -50.0, "{}", h(&tube));
    }

    /// req: FX-18
    ///
    /// Transistor: a harder knee puts more 3rd harmonic than Soft at the same drive (18 dB, -18 dBFS).
    #[test]
    fn transistor_has_more_third_harmonic() {
        let soft = run_c(Character::Soft, 18.0, 1000.0, -18.0);
        let tr = run_c(Character::Transistor, 18.0, 1000.0, -18.0);
        let h = |y: &[f32]| 20.0 * (bin(y, 3000.0) / bin(y, 1000.0)).log10();
        assert!(h(&tr) > h(&soft) + 3.0, "transistor {} soft {}", h(&tr), h(&soft));
    }

    /// req: FX-18
    ///
    /// Tape: the one-pole at 12 kHz lowers 10 kHz by about 2.3 dB; require at least 1.5 dB under Soft, while 1 kHz
    /// stays within 0.3 dB of Soft.
    #[test]
    fn tape_rolls_off_highs() {
        let d = |f: f64| {
            let soft = run_c(Character::Soft, 6.0, f, -18.0);
            let tape = run_c(Character::Tape, 6.0, f, -18.0);
            20.0 * (bin(&tape, f) / bin(&soft, f)).log10()
        };
        assert!(d(10000.0) < -1.5, "{}", d(10000.0));
        assert!(d(1000.0).abs() < 0.3, "{}", d(1000.0));
    }

    /// req: FX-18, FX-4
    ///
    /// Level compensation holds for every character: within 1.5 dB of the input at -18 dBFS, drive 0..18.
    #[test]
    fn characters_level_compensated() {
        for ch in CHARS {
            for drive in [0.0, 6.0, 12.0, 18.0] {
                let y = run_c(ch, drive, 1000.0, -18.0);
                let inp = sine(1000.0, -18.0, 48000);
                let d = 20.0 * (rms(&y) / rms(&inp)).log10();
                assert!(d.abs() < 1.5, "{ch:?} drive {drive}: {d} dB");
            }
        }
    }

    /// req: FX-18, FX-4
    ///
    /// Aliasing below -50 dB for every character (15 kHz at -6 dBFS, drive 18: the 3rd harmonic folds to 3 kHz).
    #[test]
    fn characters_alias_low() {
        for ch in CHARS {
            let y = run_c(ch, 18.0, 15000.0, -6.0);
            let rel = 20.0 * (bin(&y, 3000.0) / bin(&y, 15000.0)).log10();
            assert!(rel < -50.0, "{ch:?} alias {rel} dB");
        }
    }

    /// req: FX-18, FX-24
    ///
    /// Mix 0 is the dry signal delayed by LATENCY, whatever the character.
    #[test]
    fn characters_mix_zero_is_dry() {
        for ch in CHARS {
            let mut s = make_c(18.0, 0.0, ch);
            let inp = sine(1000.0, -6.0, 1000);
            let mut y = inp.clone();
            s.process(0, &mut y);
            for t in 0..y.len() {
                let want = if t >= Saturation::LATENCY { inp[t - Saturation::LATENCY] } else { 0.0 };
                assert!((y[t] - want).abs() <= 1e-6, "{ch:?} t={t}");
            }
        }
    }

    /// req: FX-18
    ///
    /// Bounded on full-scale noise at drive 24: Soft/Tape ceiling 2, Tube about 2 * (1 + tanh 0.3) / 0.92,
    /// Transistor 1; the 4x filters overshoot slightly. Bound 4 covers all with margin. Tube leaves no DC.
    #[test]
    fn characters_bounded_and_tube_dc_free() {
        for ch in CHARS {
            let mut s = make_c(24.0, 1.0, ch);
            let mut x: Vec<f32> = crate::testsignal::xorshift32(7, 40000).into_iter().map(|v| v as f32).collect();
            s.process(0, &mut x);
            assert!(x.iter().all(|v| v.is_finite() && v.abs() < 4.0), "{ch:?}");
        }
        let y = run_c(Character::Tube, 18.0, 1000.0, -18.0);
        let dc = y.iter().map(|v| f64::from(*v)).sum::<f64>() / y.len() as f64;
        assert!(dc.abs() < 2e-3, "dc {dc}");
    }

    /// req: FX-18, ENG-4
    ///
    /// Block-size invariance (7, 1000, 32) with fixed parameters, and stability with parameters changing every 32
    /// samples, for every character.
    #[test]
    fn characters_block_invariant() {
        let noise: Vec<f32> = crate::testsignal::xorshift32(3, 16384).into_iter().map(|v| (0.5 * v) as f32).collect();
        for ch in CHARS {
            // Parameters changing every 32 samples stay finite and bounded.
            let mut s = Saturation::new(FS);
            let mut out = noise.clone();
            for (i, chunk) in out.chunks_mut(32).enumerate() {
                let k = i as f64;
                s.set_params(&SaturationParams {
                    drive_db: 12.0 + 12.0 * (k * 0.3).sin(),
                    mix: 0.6 + 0.4 * (k * 0.2).cos(),
                    character: ch,
                });
                s.process(0, chunk);
            }
            assert!(out.iter().all(|v| v.is_finite() && v.abs() < 4.0), "{ch:?}");
            let fixed = |block: usize| {
                let mut s = Saturation::new(FS);
                s.set_params(&SaturationParams { drive_db: 15.0, mix: 0.7, character: ch });
                let mut out = noise.clone();
                for chunk in out.chunks_mut(block) {
                    s.process(0, chunk);
                }
                out
            };
            let (a, b, c) = (fixed(7), fixed(1000), fixed(32));
            let w = |p: &[f32], q: &[f32]| p.iter().zip(q).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
            assert!(w(&a, &b) <= 1e-6 && w(&a, &c) <= 1e-6, "{ch:?}");
        }
    }
}
