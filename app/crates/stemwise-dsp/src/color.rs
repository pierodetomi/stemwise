//! Colour processors (FX-18): exciter and bass enhancer.
//!
//! Both add a generated part to the dry signal. The dry path goes through a 64-sample delay, so
//! `out = dry_delayed + amount * generated`, and neutral settings (amount 0, mix 0, harmonics 0) give exactly the
//! delayed dry. [`Exciter::LATENCY`] = [`BassEnhancer::LATENCY`] = 64 samples (the 4x oversampler's), compensated by
//! the engine (ENG-5).
//!
//! **Exciter.** `HP(freq)` (2nd-order Butterworth) -> soft saturation `2 tanh(g u / 2)` at 4x with the reference-level
//! compensation of the saturation (a -18 dBFS sine keeps its level) -> `HP(freq)` again (nothing generated below the
//! corner) -> `* amount`, added to the delayed dry. Below `freq` only the skirt of the filters enters, so content
//! well under the corner is untouched; above it the band is reinforced by up to `amount` (+6 dB at 1) plus the
//! generated harmonics. The two high-passes turn the band by 180 degrees at the corner (it would come back as
//! `-0.5 x` and dig a hole of up to 6 dB there): the generated part therefore carries the band-pass of the delayed
//! dry at the corner, `BP = 1 - LP - HP` (same frequency, Q = 1), also scaled by `amount`. With it the real part of
//! what is added is never negative (for a small-signal gain of the shaper up to 1.31, its value at drive 24), so the
//! response never goes under the dry; the price is a reinforcement that starts about an octave under the corner.
//!
//! **Bass enhancer.** The input (taken from the delay line, so wet and dry are aligned) goes through `LP(freq)`
//! (2nd order). A peak follower (instant attack, 200 ms release) normalises it to `u` in [-1, 1]; the harmonics are
//! `env * (0.5 T2(u) + 0.35 T3(u) + 0.15 T4(u))` with `T_n` the Chebyshev polynomials: for a sine the 2nd, 3rd and
//! 4th harmonic exactly (and no fundamental), level following the input. The result goes through a 4th-order
//! `HP(freq)` (two biquads), so nothing at or under the corner is added, no octave below (the T2 DC is removed
//! too), and is scaled by `harmonics * mix`. No oversampling: the harmonics stay under 4 x `freq` <= 640 Hz.
//!
//! **Smoothing.** Drive, amount, harmonics and mix are smoothed per sample (5 ms one-pole). `freq` is smoothed and
//! its filters redesigned once every [`SUBBLOCK`] samples counted since `reset`, so the output does not depend on
//! how the stream is split into calls.
//!
//! **Properties verified (tests).** Exact latency and neutral = delayed dry (<= 1e-6), energy above the corner and
//! none under it, harmonics of a 60 Hz sine and nothing under 40 Hz, boundedness on full-scale noise, block-size
//! invariance (<= 1e-6), stability with parameters changing every 32 samples. `process` does not allocate, lock or
//! perform I/O.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};
use crate::oversample::Oversampler4;
use crate::util::{SUBBLOCK, db_to_gain, flush, one_pole};

const LAT: usize = Oversampler4::LATENCY;
const SMOOTH_MS: f64 = 5.0;
const REF_PEAK: f64 = 0.125_892_541_179_416_72; // 10^(-18/20)
const SAT: f32 = 2.0;

/// Smoothing coefficient of a one-pole applied once every `SUBBLOCK` samples.
fn sub_coef(coef: f64) -> f64 {
    1.0 - (1.0 - coef).powi(SUBBLOCK as i32)
}

/// Rational tanh of the saturation (f32).
#[inline]
fn tanh_r(u: f32) -> f32 {
    let u = u.clamp(-4.97, 4.97);
    let u2 = u * u;
    let n = u * (135_135.0 + u2 * (17_325.0 + u2 * (378.0 + u2)));
    let d = 135_135.0 + u2 * (62_370.0 + u2 * (3_150.0 + u2 * 28.0));
    (n / d).clamp(-1.0, 1.0)
}

#[inline]
fn shape4(v: &mut [f32; 4], g: f32, comp: f32) {
    for x in v.iter_mut() {
        *x = SAT * tanh_r(g * *x / SAT) * comp;
    }
}

#[inline]
fn comp_for(g: f64) -> f64 {
    let s = f64::from(SAT);
    REF_PEAK / (s * f64::from(tanh_r((g * REF_PEAK / s) as f32)))
}

/// Moves `v` towards `target` by `k`, snapping when closer than `eps`.
#[inline]
fn approach(v: &mut f64, target: f64, k: f64, eps: f64) {
    *v += k * (target - *v);
    if (target - *v).abs() < eps {
        *v = target;
    }
}

fn hp(freq: f64, q: f64, rate: f64) -> Coeffs {
    Coeffs::design(Kind::HighPass, freq, 0.0, q, rate)
}

// ---------------------------------------------------------------------------------------------------------------
// Exciter
// ---------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExciterParams {
    /// Corner of the excited band in Hz, 1000..12000.
    pub freq_hz: f64,
    /// Drive of the saturation in dB, 0..24.
    pub drive_db: f64,
    /// Amount of the generated part added to the dry, 0..1.
    pub amount: f64,
}

/// Q of the band-pass that fills the corner: 1 keeps the real part of what is added positive up to the shaper's
/// largest small-signal gain and adds 0.02 dB four octaves under the corner.
const BP_Q: f64 = 1.0;

#[derive(Debug, Clone)]
struct ExChan {
    os: Oversampler4,
    dry: [f32; LAT],
    pos: usize,
    hp_in: Biquad,
    hp_out: Biquad,
    /// Low-pass and high-pass of the delayed dry: `dry - lp - hp` is its band-pass at the corner.
    bp_lp: Biquad,
    bp_hp: Biquad,
    n: u64,
    freq: f64,
    coef_freq: f64,
    gain: f64,
    amount: f64,
    comp: f64,
    comp_gain: f64,
}

impl ExChan {
    fn new() -> Self {
        Self {
            os: Oversampler4::new(),
            dry: [0.0; LAT],
            pos: 0,
            hp_in: Biquad::default(),
            hp_out: Biquad::default(),
            bp_lp: Biquad::default(),
            bp_hp: Biquad::default(),
            n: 0,
            freq: 5000.0,
            coef_freq: 0.0,
            gain: 1.0,
            amount: 0.0,
            comp: 1.0,
            comp_gain: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Exciter {
    ch: [ExChan; 2],
    rate: f64,
    coef: f64,
    coef_sub: f64,
    t_freq: f64,
    t_gain: f64,
    t_amount: f64,
}

impl Exciter {
    pub const LATENCY: usize = LAT;

    pub fn new(rate: f64) -> Self {
        let coef = one_pole(SMOOTH_MS, rate);
        let mut e = Self {
            ch: [ExChan::new(), ExChan::new()],
            rate,
            coef,
            coef_sub: sub_coef(coef),
            t_freq: 5000.0,
            t_gain: 1.0,
            t_amount: 0.0,
        };
        e.reset();
        e
    }

    pub fn set_params(&mut self, p: &ExciterParams) {
        self.t_freq = p.freq_hz.clamp(1000.0, 12000.0);
        self.t_gain = db_to_gain(p.drive_db.clamp(0.0, 24.0));
        self.t_amount = p.amount.clamp(0.0, 1.0);
    }

    /// Clears the state; smoothed values jump to their targets.
    pub fn reset(&mut self) {
        for c in &mut self.ch {
            c.os.reset();
            c.dry = [0.0; LAT];
            c.pos = 0;
            c.n = 0;
            c.freq = self.t_freq;
            c.gain = self.t_gain;
            c.amount = self.t_amount;
            c.coef_freq = c.freq;
            let co = hp(c.freq, BUTTERWORTH_Q, self.rate);
            c.hp_in = Biquad::new(co);
            c.hp_out = Biquad::new(co);
            c.bp_hp = Biquad::new(hp(c.freq, BP_Q, self.rate));
            c.bp_lp = Biquad::new(Coeffs::design(Kind::LowPass, c.freq, 0.0, BP_Q, self.rate));
            c.comp = comp_for(c.gain);
            c.comp_gain = c.gain;
        }
    }

    /// In place, channel `ch` (0 or 1). Output is delayed by [`Self::LATENCY`] samples.
    pub fn process(&mut self, ch: usize, buf: &mut [f32]) {
        let c = &mut self.ch[ch];
        let (k, ks) = (self.coef, self.coef_sub);
        for s in buf.iter_mut() {
            if c.n.is_multiple_of(SUBBLOCK as u64) {
                approach(&mut c.freq, self.t_freq, ks, 1e-6 * self.t_freq);
                if c.freq != c.coef_freq {
                    let co = hp(c.freq, BUTTERWORTH_Q, self.rate);
                    c.hp_in.c = co;
                    c.hp_out.c = co;
                    c.bp_hp.c = hp(c.freq, BP_Q, self.rate);
                    c.bp_lp.c = Coeffs::design(Kind::LowPass, c.freq, 0.0, BP_Q, self.rate);
                    c.coef_freq = c.freq;
                }
            }
            c.n = c.n.wrapping_add(1);
            approach(&mut c.gain, self.t_gain, k, 1e-9);
            approach(&mut c.amount, self.t_amount, k, 1e-9);
            if c.gain != c.comp_gain {
                c.comp = comp_for(c.gain);
                c.comp_gain = c.gain;
            }
            let x = *s;
            let band = c.hp_in.tick(0, f64::from(x));
            let (g, comp) = (c.gain as f32, c.comp as f32);
            let wet = f64::from(c.os.process4(band as f32, |v| shape4(v, g, comp)));
            let gen_ = c.hp_out.tick(0, wet);
            let dry = c.dry[c.pos];
            c.dry[c.pos] = x;
            c.pos += 1;
            if c.pos == LAT {
                c.pos = 0;
            }
            let d = f64::from(dry);
            let bp = d - c.bp_lp.tick(0, d) - c.bp_hp.tick(0, d);
            *s = flush(d + c.amount * (gen_ + bp)) as f32;
        }
        c.hp_in.flush_state();
        c.hp_out.flush_state();
        c.bp_lp.flush_state();
        c.bp_hp.flush_state();
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Bass enhancer
// ---------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BassParams {
    /// Corner of the bass band in Hz, 40..160.
    pub freq_hz: f64,
    /// Strength of the generated harmonics, 0..1.
    pub harmonics: f64,
    /// Amount of the generated part added to the dry, 0..1.
    pub mix: f64,
}

/// Weights of the 2nd, 3rd and 4th harmonic (sum 1: the generated peak never exceeds the band peak).
const W2: f64 = 0.5;
const W3: f64 = 0.35;
const W4: f64 = 0.15;
const ENV_RELEASE_MS: f64 = 200.0;

#[derive(Debug, Clone)]
struct BaChan {
    dry: [f32; LAT],
    pos: usize,
    lp: Biquad,
    hp_a: Biquad,
    hp_b: Biquad,
    env: f64,
    n: u64,
    freq: f64,
    coef_freq: f64,
    harm: f64,
    mix: f64,
}

impl BaChan {
    fn new() -> Self {
        Self {
            dry: [0.0; LAT],
            pos: 0,
            lp: Biquad::default(),
            hp_a: Biquad::default(),
            hp_b: Biquad::default(),
            env: 0.0,
            n: 0,
            freq: 80.0,
            coef_freq: 0.0,
            harm: 0.0,
            mix: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BassEnhancer {
    ch: [BaChan; 2],
    rate: f64,
    coef: f64,
    coef_sub: f64,
    release: f64,
    t_freq: f64,
    t_harm: f64,
    t_mix: f64,
}

/// Q of the two sections of a 4th-order Butterworth.
const Q4A: f64 = 0.541_196_100_146_197;
const Q4B: f64 = 1.306_562_964_876_376_6;

impl BassEnhancer {
    pub const LATENCY: usize = LAT;

    pub fn new(rate: f64) -> Self {
        let coef = one_pole(SMOOTH_MS, rate);
        let mut e = Self {
            ch: [BaChan::new(), BaChan::new()],
            rate,
            coef,
            coef_sub: sub_coef(coef),
            release: (-1.0 / (ENV_RELEASE_MS * 0.001 * rate)).exp(),
            t_freq: 80.0,
            t_harm: 0.0,
            t_mix: 0.0,
        };
        e.reset();
        e
    }

    pub fn set_params(&mut self, p: &BassParams) {
        self.t_freq = p.freq_hz.clamp(40.0, 160.0);
        self.t_harm = p.harmonics.clamp(0.0, 1.0);
        self.t_mix = p.mix.clamp(0.0, 1.0);
    }

    /// Clears the state; smoothed values jump to their targets.
    pub fn reset(&mut self) {
        for c in &mut self.ch {
            c.dry = [0.0; LAT];
            c.pos = 0;
            c.n = 0;
            c.env = 0.0;
            c.freq = self.t_freq;
            c.harm = self.t_harm;
            c.mix = self.t_mix;
            c.coef_freq = c.freq;
            Self::design(c, self.rate);
            c.lp.reset();
            c.hp_a.reset();
            c.hp_b.reset();
        }
    }

    fn design(c: &mut BaChan, rate: f64) {
        c.lp.c = Coeffs::design(Kind::LowPass, c.freq, 0.0, BUTTERWORTH_Q, rate);
        c.hp_a.c = hp(c.freq, Q4A, rate);
        c.hp_b.c = hp(c.freq, Q4B, rate);
    }

    /// In place, channel `ch` (0 or 1). Output is delayed by [`Self::LATENCY`] samples.
    pub fn process(&mut self, ch: usize, buf: &mut [f32]) {
        let c = &mut self.ch[ch];
        let (k, ks, rel) = (self.coef, self.coef_sub, self.release);
        for s in buf.iter_mut() {
            if c.n.is_multiple_of(SUBBLOCK as u64) {
                approach(&mut c.freq, self.t_freq, ks, 1e-6 * self.t_freq);
                if c.freq != c.coef_freq {
                    Self::design(c, self.rate);
                    c.coef_freq = c.freq;
                }
            }
            c.n = c.n.wrapping_add(1);
            approach(&mut c.harm, self.t_harm, k, 1e-9);
            approach(&mut c.mix, self.t_mix, k, 1e-9);
            let x = *s;
            let dry = c.dry[c.pos];
            c.dry[c.pos] = x;
            c.pos += 1;
            if c.pos == LAT {
                c.pos = 0;
            }
            // Band of the delayed input, so wet and dry stay aligned.
            let band = c.lp.tick(0, f64::from(dry));
            c.env = (c.env * rel).max(band.abs());
            let u = if c.env > 1e-9 { (band / c.env).clamp(-1.0, 1.0) } else { 0.0 };
            let u2 = u * u;
            let t2 = 2.0 * u2 - 1.0;
            let t3 = u * (4.0 * u2 - 3.0);
            let t4 = 8.0 * u2 * (u2 - 1.0) + 1.0;
            let gen_raw = c.env * (W2 * t2 + W3 * t3 + W4 * t4);
            let gen_ = c.hp_b.tick(0, c.hp_a.tick(0, gen_raw));
            *s = flush(f64::from(dry) + c.mix * c.harm * gen_) as f32;
        }
        c.lp.flush_state();
        c.hp_a.flush_state();
        c.hp_b.flush_state();
        if c.env < 1e-30 {
            c.env = 0.0;
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

    fn bin(x: &[f32], freq: f64) -> f64 {
        let (mut re, mut im) = (0.0, 0.0);
        for (t, v) in x.iter().enumerate() {
            let ph = TAU * freq * t as f64 / FS;
            re += f64::from(*v) * ph.cos();
            im += f64::from(*v) * ph.sin();
        }
        2.0 * (re * re + im * im).sqrt() / x.len() as f64
    }

    fn db(a: f64, b: f64) -> f64 {
        20.0 * (a / b).log10()
    }

    fn noise(seed: u32, n: usize) -> Vec<f32> {
        crate::testsignal::xorshift32(seed, n).into_iter().map(|v| v as f32).collect()
    }

    fn worst(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max)
    }

    fn ex(p: ExciterParams) -> Exciter {
        let mut e = Exciter::new(FS);
        e.set_params(&p);
        e.reset();
        e
    }

    fn ba(p: BassParams) -> BassEnhancer {
        let mut e = BassEnhancer::new(FS);
        e.set_params(&p);
        e.reset();
        e
    }

    const EX: ExciterParams = ExciterParams { freq_hz: 3000.0, drive_db: 12.0, amount: 1.0 };
    const BA: BassParams = BassParams { freq_hz: 100.0, harmonics: 1.0, mix: 1.0 };

    /// Output of `process` over `x` after dropping the first `skip` samples.
    fn run_ex(p: ExciterParams, x: &[f32], skip: usize) -> Vec<f32> {
        let mut e = ex(p);
        let mut y = x.to_vec();
        e.process(0, &mut y);
        y[skip..].to_vec()
    }

    fn run_ba(p: BassParams, x: &[f32], skip: usize) -> Vec<f32> {
        let mut e = ba(p);
        let mut y = x.to_vec();
        e.process(0, &mut y);
        y[skip..].to_vec()
    }

    /// req: FX-18, FX-24, ENG-5
    #[test]
    fn neutral_is_delayed_dry() {
        let inp = noise(5, 4000);
        let want = |t: usize| if t >= LAT { inp[t - LAT] } else { 0.0 };
        let neutrals_ex =
            [ExciterParams { amount: 0.0, ..EX }, ExciterParams { amount: 0.0, drive_db: 24.0, freq_hz: 1000.0 }];
        for p in neutrals_ex {
            let y = run_ex(p, &inp, 0);
            assert!((0..y.len()).all(|t| (y[t] - want(t)).abs() <= 1e-6), "{p:?}");
        }
        let neutrals_ba = [BassParams { mix: 0.0, ..BA }, BassParams { harmonics: 0.0, ..BA }];
        for p in neutrals_ba {
            let y = run_ba(p, &inp, 0);
            assert!((0..y.len()).all(|t| (y[t] - want(t)).abs() <= 1e-6), "{p:?}");
        }
        assert_eq!(Exciter::LATENCY, 64);
        assert_eq!(BassEnhancer::LATENCY, 64);
    }

    /// req: FX-18, ENG-5
    ///
    /// An impulse at full level comes out dry-delayed exactly LATENCY samples later (the generated part is zero
    /// for amount/mix 0, so this is the dry path).
    #[test]
    fn latency_exact() {
        let mut x = vec![0.0f32; 300];
        x[3] = 0.5;
        let y = run_ex(ExciterParams { amount: 0.0, ..EX }, &x, 0);
        assert_eq!(y.iter().position(|v| *v != 0.0), Some(3 + Exciter::LATENCY));
        let y = run_ba(BassParams { mix: 0.0, ..BA }, &x, 0);
        assert_eq!(y.iter().position(|v| *v != 0.0), Some(3 + BassEnhancer::LATENCY));
    }

    /// req: FX-18
    ///
    /// A 200 Hz sine under a 3 kHz corner gains less than 0.1 dB (the 4th-order skirt leaves about -90 dB of it),
    /// even at drive 24. A 6 kHz sine above the corner is reinforced by more than 3 dB (the theoretical gain is
    /// +6 dB at amount 1), and a 4 kHz sine gains a 3rd harmonic at 12 kHz that the dry signal does not have.
    #[test]
    fn exciter_adds_only_above_corner() {
        let p = ExciterParams { drive_db: 24.0, ..EX };
        let low = sine(200.0, -6.0, 48000 + 4000);
        let y = run_ex(p, &low, 4000);
        let d = db(bin(&y, 200.0), bin(&low[..48000], 200.0));
        assert!(d.abs() < 0.1, "{d} dB");

        let hi = sine(6000.0, -20.0, 48000 + 4000);
        let y = run_ex(EX, &hi, 4000);
        let d = db(bin(&y, 6000.0), bin(&hi[..48000], 6000.0));
        assert!(d > 3.0, "{d} dB");

        let mid = sine(4000.0, -10.0, 48000 + 4000);
        let y = run_ex(EX, &mid, 4000);
        let h3 = db(bin(&y, 12000.0), bin(&y, 4000.0));
        assert!(h3 > -50.0, "3rd harmonic {h3} dB");
        // And the dry has none.
        assert!(db(bin(&mid[..48000], 12000.0), bin(&mid[..48000], 4000.0)) < -90.0);
    }

    /// req: FX-18
    ///
    /// No hole at the corner: with the amount at 1 a sine from two octaves under the corner to two above never
    /// comes out lower than it went in (0.05 dB of measurement margin), at no drive and at full drive. Without the
    /// band-pass term the corner itself came out 6 dB down at drive 0.
    #[test]
    fn exciter_never_digs_under_the_dry() {
        for drive_db in [0.0, 24.0] {
            let p = ExciterParams { drive_db, amount: 1.0, ..EX };
            for k in -8..=8 {
                let f = p.freq_hz * 2f64.powf(f64::from(k) / 4.0);
                let x = sine(f, -18.0, 48000 + 4000);
                let y = run_ex(p, &x, 4000);
                let d = db(bin(&y, f), bin(&x[..48000], f));
                assert!(d > -0.05, "drive {drive_db}: {f:.0} Hz comes out {d:.2} dB");
            }
        }
    }

    /// req: FX-18
    ///
    /// Bass enhancer on a 60 Hz sine (corner 100 Hz): the 120 and 180 Hz harmonics appear (> -35 dB re the
    /// fundamental; the dry has none), and nothing under 40 Hz: DC, 20 and 30 Hz are below -60 dB re the
    /// fundamental (no octave below).
    #[test]
    fn bass_adds_harmonics_not_sub() {
        let x = sine(60.0, -10.0, 96000 + 24000);
        let y = run_ba(BA, &x, 24000);
        let f0 = bin(&y, 60.0);
        for h in [120.0, 180.0] {
            let rel = db(bin(&y, h), f0);
            assert!(rel > -35.0, "{h} Hz {rel} dB");
        }
        for sub in [0.0, 20.0, 30.0, 40.0] {
            let rel = db(bin(&y, sub), f0);
            assert!(rel < -60.0, "{sub} Hz {rel} dB");
        }
    }

    /// req: FX-18
    ///
    /// The generated harmonics scale with `harmonics`: half the strength is within 1 dB of -6 dB (the follower and
    /// filters are linear in the gain).
    #[test]
    fn bass_harmonics_scale() {
        let x = sine(60.0, -10.0, 48000 + 24000);
        let full = run_ba(BA, &x, 24000);
        let half = run_ba(BassParams { harmonics: 0.5, ..BA }, &x, 24000);
        let dry = &x[24000 - LAT..48000 + 24000 - LAT];
        let h = |y: &[f32]| {
            let g: Vec<f32> = y.iter().zip(dry).map(|(a, b)| a - b).collect();
            bin(&g, 120.0)
        };
        let d = db(h(&half), h(&full));
        assert!((d + 6.02).abs() < 1.0, "{d} dB");
    }

    /// req: FX-18, FX-24
    ///
    /// Full-scale white noise at extreme settings is finite. Bounds: exciter, dry (1) + amount x (saturator ceiling
    /// 2 x compensation up to ~1.3 at drive 24, filter overshoot) stays under 5; bass, dry (1) + the generated peak,
    /// which cannot exceed the band peak (< 1.3 for the Butterworth), under 3.
    #[test]
    fn bounded_on_noise() {
        let x = noise(9, 60000);
        for p in [
            ExciterParams { freq_hz: 1000.0, drive_db: 24.0, amount: 1.0 },
            ExciterParams { freq_hz: 12000.0, drive_db: 24.0, amount: 1.0 },
        ] {
            let y = run_ex(p, &x, 0);
            assert!(y.iter().all(|v| v.is_finite() && v.abs() < 5.0), "{p:?}");
        }
        for p in [BassParams { freq_hz: 40.0, harmonics: 1.0, mix: 1.0 }, BassParams { freq_hz: 160.0, ..BA }] {
            let y = run_ba(p, &x, 0);
            assert!(y.iter().all(|v| v.is_finite() && v.abs() < 3.0), "{p:?}");
        }
    }

    /// req: FX-18, ENG-4
    ///
    /// Calls of 7, 32 and 1000 frames give the same stream (<= 1e-6), with fixed parameters and with a corner
    /// moving at a block-aligned rate.
    #[test]
    fn block_size_invariant() {
        let x: Vec<f32> = noise(2, 16384).iter().map(|v| 0.4 * v).collect();
        let w = |a: &[f32], b: &[f32]| worst(a, b);
        let ex_run = |block: usize| {
            let mut e = ex(EX);
            let mut y = x.clone();
            for c in y.chunks_mut(block) {
                e.process(0, c);
            }
            y
        };
        let (a, b, c) = (ex_run(7), ex_run(1000), ex_run(32));
        assert!(w(&a, &b) <= 1e-6 && w(&a, &c) <= 1e-6);
        let ba_run = |block: usize| {
            let mut e = ba(BA);
            let mut y = x.clone();
            for c in y.chunks_mut(block) {
                e.process(0, c);
            }
            y
        };
        let (a, b, c) = (ba_run(7), ba_run(1000), ba_run(32));
        assert!(w(&a, &b) <= 1e-6 && w(&a, &c) <= 1e-6);

        // The corner changes at sample 4096 only; every split sees the same targets from there.
        let moving = |block: usize| {
            let mut e = ex(EX);
            let mut y = x.clone();
            let mut pos = 0;
            for c in y.chunks_mut(block) {
                if pos >= 4096 && pos - c.len().min(pos) < 4096 {
                    e.set_params(&ExciterParams { freq_hz: 8000.0, ..EX });
                }
                e.process(0, c);
                pos += c.len();
            }
            y
        };
        // Blocks of 32 and 64 both put a boundary exactly on 4096.
        assert!(w(&moving(32), &moving(64)) <= 1e-6);
    }

    /// req: FX-18, AUT-6
    ///
    /// Parameters changing every 32 samples on noise for 4 s: finite and bounded.
    #[test]
    fn stable_with_changing_params() {
        let x = noise(4, 192_000);
        let mut e = ex(EX);
        let mut b = ba(BA);
        let (mut y, mut z) = (x.clone(), x.clone());
        for (i, (cy, cz)) in y.chunks_mut(32).zip(z.chunks_mut(32)).enumerate() {
            let k = i as f64;
            e.set_params(&ExciterParams {
                freq_hz: 6000.0 + 5000.0 * (k * 0.07).sin(),
                drive_db: 12.0 + 12.0 * (k * 0.11).sin(),
                amount: 0.5 + 0.5 * (k * 0.05).cos(),
            });
            b.set_params(&BassParams {
                freq_hz: 100.0 + 60.0 * (k * 0.07).sin(),
                harmonics: 0.5 + 0.5 * (k * 0.11).sin(),
                mix: 0.5 + 0.5 * (k * 0.05).cos(),
            });
            e.process(0, cy);
            b.process(0, cz);
        }
        assert!(y.iter().all(|v| v.is_finite() && v.abs() < 5.0));
        assert!(z.iter().all(|v| v.is_finite() && v.abs() < 3.0));
    }

    /// req: FX-18
    #[test]
    fn channels_independent() {
        let mut e = ex(EX);
        let mut l = sine(5000.0, -10.0, 4000);
        let mut r = vec![0.0f32; 4000];
        e.process(0, &mut l);
        e.process(1, &mut r);
        assert!(r.iter().all(|v| *v == 0.0));
        let mut b = ba(BA);
        let mut l = sine(60.0, -10.0, 4000);
        let mut r = vec![0.0f32; 4000];
        b.process(0, &mut l);
        b.process(1, &mut r);
        assert!(r.iter().all(|v| *v == 0.0));
    }
}
