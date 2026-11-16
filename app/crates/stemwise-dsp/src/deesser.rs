//! De-esser (FX-14, FX-24).
//!
//! - Band split: `high = x − LP(x)`, with LP a 2nd-order Butterworth low-pass at `freq_hz`. The frequency is
//!   smoothed (one-pole, 20 ms) on a grid of 32 samples counted from `reset`, so automation does not click and the
//!   result does not depend on the block size.
//! - Detector: `high` through a 2nd-order Butterworth high-pass at the same frequency (the complement alone falls
//!   by 6 dB per octave under the corner: a loud vowel at 2 kHz would reach a 6 kHz detector only 6 dB down and pull
//!   the highs down with no sibilance; with the high-pass it is 25 dB down). Its power, larger of the linked channels, followed with a branching one-pole (attack
//!   1 ms, release 60 ms). The level is `10·log10(power) + 3.01` dB, i.e. the peak level of a sine in the band.
//! - Gain computer: ratio 10 above the threshold (reduction `0.9·(level − threshold)`), capped at
//!   `max_reduction_db`; the reduction is smoothed (2 ms) so changes of the parameters do not click.
//! - `Split`: `y = x − (1 − g)·high`, so only the part above the frequency is lowered and below it the signal does
//!   not change in level (the complement of a Butterworth low-pass is a first-order high-pass in quadrature with the
//!   signal at low frequencies: at 500 Hz with a 6 kHz corner `high` is 19 dB down and the level error is below
//!   0.05 dB even at 12 dB of reduction). `Wide`: `y = g·x`. `listen`: the output is `high` (for auditioning; never stored).
//! - Latency 0. Meter: deepest reduction since the last `take_gr_db`.
//!
//! Properties verified by the tests: low frequencies untouched in split mode, wide mode, neutral at 0 dB maximum,
//! the cap, listen, block-size invariance, bound and stability.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};
use crate::util::{SUBBLOCK, flush, one_pole};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeesserMode {
    Split,
    Wide,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeesserParams {
    /// Corner of the band that is detected and reduced, 3000..12000 Hz.
    pub freq_hz: f64,
    pub threshold_db: f64,
    /// Largest reduction in dB, 0..12. 0 makes the processor neutral.
    pub max_reduction_db: f64,
    pub mode: DeesserMode,
    /// Output only the detected band.
    pub listen: bool,
}

impl Default for DeesserParams {
    fn default() -> Self {
        DeesserParams {
            freq_hz: 6500.0,
            threshold_db: -30.0,
            max_reduction_db: 6.0,
            mode: DeesserMode::Split,
            listen: false,
        }
    }
}

/// Ratio of the gain computer.
const RATIO: f64 = 10.0;
/// Peak level of a sine against the mean power of its square.
const SINE_POWER_TO_PEAK_DB: f64 = 3.010_299_956_639_812;

pub struct Deesser {
    rate: f64,
    p: DeesserParams,
    freq: f64,
    freq_smooth: f64,
    fresh: bool,
    frame: usize,
    lp: Biquad,
    /// High-pass of the detector only.
    det: Biquad,
    att: f64,
    rel: f64,
    gr_smooth: f64,
    env: f64,
    red_db: f64,
    gr_min: f64,
}

impl Deesser {
    pub fn new(rate: f64) -> Self {
        let p = DeesserParams::default();
        let mut d = Deesser {
            rate,
            p,
            freq: p.freq_hz,
            freq_smooth: 1.0 - (-(SUBBLOCK as f64) / (0.02 * rate)).exp(),
            fresh: true,
            frame: 0,
            lp: Biquad::default(),
            det: Biquad::default(),
            att: one_pole(1.0, rate),
            rel: one_pole(60.0, rate),
            gr_smooth: one_pole(2.0, rate),
            env: 0.0,
            red_db: 0.0,
            gr_min: 0.0,
        };
        d.design();
        d
    }

    fn design(&mut self) {
        self.lp.c = Coeffs::design(Kind::LowPass, self.freq, 0.0, BUTTERWORTH_Q, self.rate);
        self.det.c = Coeffs::design(Kind::HighPass, self.freq, 0.0, BUTTERWORTH_Q, self.rate);
    }

    pub fn set_params(&mut self, p: &DeesserParams) {
        let mut p = *p;
        p.freq_hz = p.freq_hz.clamp(3000.0, 12000.0);
        p.max_reduction_db = p.max_reduction_db.clamp(0.0, 12.0);
        self.p = p;
        if self.fresh {
            self.freq = p.freq_hz;
            self.design();
        }
    }

    pub fn reset(&mut self) {
        self.lp.reset();
        self.det.reset();
        self.env = 0.0;
        self.red_db = 0.0;
        self.gr_min = 0.0;
        self.frame = 0;
        self.fresh = true;
        self.freq = self.p.freq_hz;
        self.design();
    }

    /// Deepest reduction (dB, ≤ 0) since the last call.
    pub fn take_gr_db(&mut self) -> f64 {
        std::mem::replace(&mut self.gr_min, -self.red_db)
    }

    pub fn process(&mut self, chans: &mut [&mut [f32]]) {
        let nch = chans.len().min(2);
        if nch == 0 {
            return;
        }
        let n = chans[0].len();
        let thr = self.p.threshold_db;
        let max_red = self.p.max_reduction_db;
        let split = self.p.mode == DeesserMode::Split;
        for i in 0..n {
            if self.frame.is_multiple_of(SUBBLOCK) {
                if self.fresh {
                    self.fresh = false;
                } else if (self.freq - self.p.freq_hz).abs() > 1e-9 {
                    self.freq += self.freq_smooth * (self.p.freq_hz - self.freq);
                    self.design();
                }
            }
            self.frame += 1;
            let mut high = [0.0f64; 2];
            let mut pow = 0.0f64;
            for c in 0..nch {
                let x = f64::from(chans[c][i]);
                high[c] = x - self.lp.tick(c, x);
                let d = self.det.tick(c, high[c]);
                pow = pow.max(d * d);
            }
            let coef = if pow > self.env { self.att } else { self.rel };
            self.env = flush(self.env + coef * (pow - self.env));
            let level_db = 10.0 * self.env.max(1e-24).log10() + SINE_POWER_TO_PEAK_DB;
            let over = level_db - thr;
            let target = if over > 0.0 { (over * (1.0 - 1.0 / RATIO)).min(max_red) } else { 0.0 };
            self.red_db = flush(self.red_db + self.gr_smooth * (target - self.red_db)).clamp(0.0, max_red);
            if -self.red_db < self.gr_min {
                self.gr_min = -self.red_db;
            }
            let g = (-self.red_db * std::f64::consts::LN_10 / 20.0).exp();
            for c in 0..nch {
                let x = f64::from(chans[c][i]);
                let y = if self.p.listen {
                    high[c]
                } else if split {
                    x - (1.0 - g) * high[c]
                } else {
                    x * g
                };
                chans[c][i] = y as f32;
            }
        }
        self.lp.flush_state();
        self.det.flush_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::xorshift32;
    use std::f64::consts::PI;

    const FS: f64 = 48000.0;

    fn sine(n: usize, amp_db: f64, freq: f64) -> Vec<f32> {
        let a = 10f64.powf(amp_db / 20.0);
        (0..n).map(|i| (a * (2.0 * PI * freq * i as f64 / FS).sin()) as f32).collect()
    }

    /// Level of one frequency in a signal (single-bin DFT over whole periods), in dB.
    fn tone_db(x: &[f32], freq: f64) -> f64 {
        let (mut re, mut im) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            let ph = 2.0 * PI * freq * i as f64 / FS;
            re += f64::from(*v) * ph.cos();
            im += f64::from(*v) * ph.sin();
        }
        20.0 * (2.0 * (re * re + im * im).sqrt() / x.len() as f64).max(1e-12).log10()
    }

    fn mixed(n: usize) -> Vec<f32> {
        // 500 Hz at −20 dBFS plus 8 kHz at −6 dBFS: far above a −30 dB threshold.
        let a = sine(n, -20.0, 500.0);
        let b = sine(n, -6.0, 8000.0);
        a.iter().zip(&b).map(|(u, v)| u + v).collect()
    }

    fn run(p: &DeesserParams, x: &[f32]) -> (Vec<f32>, f64) {
        let mut d = Deesser::new(FS);
        d.set_params(p);
        let mut y = x.to_vec();
        d.process(&mut [&mut y[..]]);
        (y, d.take_gr_db())
    }

    // Whole periods of both tones: 0.5 s window 24000 samples is 250 periods of 500 Hz and 1000 of 8 kHz.
    const SETTLED: usize = 24000;

    /// req: FX-14, FX-24
    #[test]
    fn split_keeps_the_low_tone_and_lowers_the_high_one() {
        let x = mixed(48000);
        let p = DeesserParams { freq_hz: 6000.0, threshold_db: -30.0, max_reduction_db: 12.0, ..Default::default() };
        let (y, gr) = run(&p, &x);
        assert!(gr <= -4.0, "gr {gr}");
        // At 500 Hz `high` is quadrature and ~19 dB down, so the level moves by ≈ 0.5·(0.75·0.118)² in power: ±0.1 dB.
        let d_low = tone_db(&y[SETTLED..], 500.0) - tone_db(&x[SETTLED..], 500.0);
        let d_high = tone_db(&y[SETTLED..], 8000.0) - tone_db(&x[SETTLED..], 8000.0);
        assert!(d_low.abs() <= 0.1, "low {d_low}");
        assert!(d_high <= -4.0, "high {d_high}");
    }

    /// A loud vowel under the corner does not trigger the de-esser: 2 kHz at −12 dBFS against a 6 kHz corner and a
    /// −30 dB threshold reaches the detector 25 dB down (−37 dB), so nothing is reduced; 8 kHz at the same level is.
    /// req: FX-14
    #[test]
    fn a_loud_tone_under_the_corner_does_not_trigger_it() {
        let p = DeesserParams { freq_hz: 6000.0, threshold_db: -30.0, max_reduction_db: 12.0, ..Default::default() };
        let run = |freq: f64| {
            let mut d = Deesser::new(FS);
            d.set_params(&p);
            let mut y = sine(24000, -12.0, freq);
            d.process(&mut [&mut y[..]]);
            d.take_gr_db()
        };
        assert!(run(2000.0) > -0.1, "vowel: {} dB", run(2000.0));
        assert!(run(8000.0) < -6.0, "sibilant: {} dB", run(8000.0));
    }

    /// req: FX-14
    #[test]
    fn wide_lowers_both() {
        let x = mixed(48000);
        let p = DeesserParams {
            freq_hz: 6000.0,
            threshold_db: -30.0,
            max_reduction_db: 12.0,
            mode: DeesserMode::Wide,
            ..Default::default()
        };
        let (y, _) = run(&p, &x);
        let d_low = tone_db(&y[SETTLED..], 500.0) - tone_db(&x[SETTLED..], 500.0);
        let d_high = tone_db(&y[SETTLED..], 8000.0) - tone_db(&x[SETTLED..], 8000.0);
        assert!(d_low <= -4.0 && (d_low - d_high).abs() <= 0.1, "low {d_low} high {d_high}");
    }

    /// req: FX-14, FX-24
    #[test]
    fn zero_maximum_is_identity() {
        let x: Vec<f32> = xorshift32(3, 20000).iter().map(|v| (v * 0.5) as f32).collect();
        for mode in [DeesserMode::Split, DeesserMode::Wide] {
            let (y, gr) =
                run(&DeesserParams { max_reduction_db: 0.0, mode, threshold_db: -80.0, ..Default::default() }, &x);
            let m = x.iter().zip(&y).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
            assert!(m <= 1e-6 && gr == 0.0, "{mode:?}: {m} {gr}");
        }
    }

    /// req: FX-14
    #[test]
    fn reduction_never_exceeds_the_maximum() {
        let x = mixed(48000);
        let (_, gr) = run(&DeesserParams { max_reduction_db: 3.0, threshold_db: -80.0, ..Default::default() }, &x);
        assert!((-3.0 - 1e-9..=-2.9).contains(&gr), "gr {gr}");
    }

    /// req: FX-14
    #[test]
    fn listen_outputs_only_the_band() {
        let x = mixed(48000);
        let (y, _) = run(&DeesserParams { freq_hz: 6000.0, listen: true, ..Default::default() }, &x);
        let d_low = tone_db(&y[SETTLED..], 500.0) - tone_db(&x[SETTLED..], 500.0);
        let d_high = tone_db(&y[SETTLED..], 8000.0) - tone_db(&x[SETTLED..], 8000.0);
        // `x − LP(x)` is a 6 dB/oct high-pass at low frequencies: 500 Hz against a 6 kHz corner is ≈ −19 dB.
        assert!(d_low <= -15.0, "low {d_low}");
        assert!(d_high.abs() <= 3.0, "high {d_high}");
    }

    /// req: FX-14, FX-24
    #[test]
    fn block_size_does_not_matter_with_moving_frequency() {
        let x = mixed(30000);
        let run_chunks = |chunk: usize| {
            let mut d = Deesser::new(FS);
            let mut y = x.clone();
            let mut pos = 0;
            for c in y.chunks_mut(chunk) {
                // Parameters change once per 32 frames, as in the engine, whatever the chunk size.
                for sub in c.chunks_mut(32) {
                    d.set_params(&DeesserParams {
                        freq_hz: 4000.0 + (pos / 32 % 50) as f64 * 100.0,
                        threshold_db: -40.0,
                        max_reduction_db: 9.0,
                        ..Default::default()
                    });
                    pos += sub.len();
                    d.process(&mut [sub]);
                }
            }
            y
        };
        let a = run_chunks(32);
        for chunk in [1024, 64] {
            let b = run_chunks(chunk);
            let m = a.iter().zip(&b).map(|(u, v)| (u - v).abs()).fold(0.0f32, f32::max);
            assert!(m <= 1e-6, "chunk {chunk}: {m}");
        }
    }

    /// req: FX-14, FX-24
    #[test]
    fn extreme_settings_stay_bounded() {
        // Split: |y| ≤ |x| + |1 − g|·|high| ≤ |x| + 1·(|x| + |LP x|); a 2nd-order Butterworth low-pass has a peak
        // gain ≤ 1.3 on white noise-like inputs, so 4.0 is a generous bound; wide: ≤ |x|.
        let n = 48000 * 3;
        let mut l: Vec<f32> = xorshift32(1, n).iter().map(|v| *v as f32).collect();
        let mut r: Vec<f32> = xorshift32(2, n).iter().map(|v| *v as f32).collect();
        let mut d = Deesser::new(FS);
        for (k, (cl, cr)) in l.chunks_mut(32).zip(r.chunks_mut(32)).enumerate() {
            let k = k as u32 + 1;
            let t = f64::from((k - 1) % 11) / 10.0;
            d.set_params(&DeesserParams {
                freq_hz: 3000.0 + 9000.0 * t,
                threshold_db: -80.0 * (1.0 - t),
                max_reduction_db: 12.0 * (1.0 - t * 0.5),
                mode: if k.is_multiple_of(3) { DeesserMode::Wide } else { DeesserMode::Split },
                listen: k.is_multiple_of(17),
            });
            d.process(&mut [cl, cr]);
        }
        for v in l.iter().chain(r.iter()) {
            assert!(v.is_finite() && v.abs() <= 4.0);
        }
    }
}
