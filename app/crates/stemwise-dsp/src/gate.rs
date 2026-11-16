//! Gate / downward expander with a stereo-linked detector (FX-13, FX-24).
//!
//! - Detector: the larger absolute value over the channels of the key (the processed signal itself, or a
//!   sidechain key when one is given), through a 2nd-order Butterworth high-pass and low-pass (own state per
//!   channel; 20 Hz and 20 kHz mean "off"). A peak follower (instant attack, 10 ms decay) turns it into a level.
//! - Gain computer (dB): at or above the threshold 0 dB; below it `−(ratio − 1)·(threshold − level)`, limited to
//!   `−range`. At ratio 20 that reaches the range for any level more than `range/19` dB under the threshold.
//! - Hold: after the level falls under the threshold the gain stays at 0 dB for `hold_ms`.
//! - Ballistics: the gain in dB moves to its target with a one-pole: `attack_ms` when opening, `release_ms` when
//!   closing, per sample, so openings and closings never click. Latency 0.
//! - Meter: the deepest gain since the last `take_gr_db`.
//!
//! Properties verified by the tests: depth at steady state, transparency above the threshold, neutral range,
//! sidechain key, hold, block-size invariance, bound and stability.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};
use crate::util::{flush, one_pole};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateParams {
    pub threshold_db: f64,
    /// Maximum attenuation in dB, 0..80. 0 makes the gate neutral.
    pub range_db: f64,
    /// Expansion ratio, 1..20; 20 behaves as a gate.
    pub ratio: f64,
    pub attack_ms: f64,
    pub hold_ms: f64,
    pub release_ms: f64,
    /// Detector high-pass, 20..2000 Hz (20 = off).
    pub detector_hpf_hz: f64,
    /// Detector low-pass, 1000..20000 Hz (20000 = off).
    pub detector_lpf_hz: f64,
}

impl Default for GateParams {
    fn default() -> Self {
        GateParams {
            threshold_db: -50.0,
            range_db: 60.0,
            ratio: 20.0,
            attack_ms: 0.5,
            hold_ms: 50.0,
            release_ms: 150.0,
            detector_hpf_hz: 20.0,
            detector_lpf_hz: 20000.0,
        }
    }
}

/// Decay of the detector's peak follower.
const DETECTOR_DECAY_MS: f64 = 10.0;

pub struct Gate {
    rate: f64,
    p: GateParams,
    att: f64,
    rel: f64,
    hold_samples: u64,
    dec: f64,
    hpf: Biquad,
    lpf: Biquad,
    env: f64,
    hold_left: u64,
    gain_db: f64,
    gr_min: f64,
}

impl Gate {
    pub fn new(rate: f64) -> Self {
        let mut g = Gate {
            rate,
            p: GateParams::default(),
            att: 0.0,
            rel: 0.0,
            hold_samples: 0,
            dec: 1.0 - one_pole(DETECTOR_DECAY_MS, rate),
            hpf: Biquad::default(),
            lpf: Biquad::default(),
            env: 0.0,
            hold_left: 0,
            gain_db: 0.0,
            gr_min: 0.0,
        };
        g.set_params(&GateParams::default());
        g
    }

    /// Cheap: a few `exp` and, only when a detector frequency changed, two biquad designs.
    pub fn set_params(&mut self, p: &GateParams) {
        let old = self.p;
        let mut p = *p;
        p.range_db = p.range_db.clamp(0.0, 80.0);
        p.ratio = p.ratio.clamp(1.0, 20.0);
        p.attack_ms = p.attack_ms.clamp(0.05, 50.0);
        p.hold_ms = p.hold_ms.clamp(0.0, 500.0);
        p.release_ms = p.release_ms.clamp(5.0, 2000.0);
        p.detector_hpf_hz = p.detector_hpf_hz.clamp(20.0, 2000.0);
        p.detector_lpf_hz = p.detector_lpf_hz.clamp(1000.0, 20000.0);
        self.p = p;
        self.att = one_pole(p.attack_ms, self.rate);
        self.rel = one_pole(p.release_ms, self.rate);
        self.hold_samples = (p.hold_ms * 0.001 * self.rate).round() as u64;
        if p.detector_hpf_hz != old.detector_hpf_hz || self.hpf.c == Coeffs::IDENTITY && p.detector_hpf_hz > 20.0 {
            self.hpf.c = if p.detector_hpf_hz <= 20.0 {
                Coeffs::IDENTITY
            } else {
                Coeffs::design(Kind::HighPass, p.detector_hpf_hz, 0.0, BUTTERWORTH_Q, self.rate)
            };
        }
        if p.detector_lpf_hz != old.detector_lpf_hz || self.lpf.c == Coeffs::IDENTITY && p.detector_lpf_hz < 20000.0 {
            self.lpf.c = if p.detector_lpf_hz >= 20000.0 {
                Coeffs::IDENTITY
            } else {
                Coeffs::design(Kind::LowPass, p.detector_lpf_hz, 0.0, BUTTERWORTH_Q, self.rate)
            };
        }
    }

    pub fn reset(&mut self) {
        self.hpf.reset();
        self.lpf.reset();
        self.env = 0.0;
        self.hold_left = 0;
        self.gain_db = 0.0;
        self.gr_min = 0.0;
    }

    /// Deepest gain (dB, ≤ 0) since the last call.
    pub fn take_gr_db(&mut self) -> f64 {
        std::mem::replace(&mut self.gr_min, self.gain_db.min(0.0))
    }

    /// Processes 1 or 2 channels in place. `key` is an optional sidechain (same length as the channels).
    pub fn process(&mut self, chans: &mut [&mut [f32]], key: Option<&[&[f32]]>) {
        let nch = chans.len().min(2);
        if nch == 0 {
            return;
        }
        let n = chans[0].len();
        let thr = self.p.threshold_db;
        let range = self.p.range_db;
        let slope = self.p.ratio - 1.0;
        for i in 0..n {
            let mut lvl = 0.0f64;
            match key {
                Some(k) if !k.is_empty() => {
                    // Every channel of the key, also when the gated signal is mono: a source panned to one side
                    // still opens it.
                    for c in 0..k.len().min(2) {
                        let x = f64::from(k[c][i]);
                        let y = self.lpf.tick(c, self.hpf.tick(c, x));
                        lvl = lvl.max(y.abs());
                    }
                }
                _ => {
                    for c in 0..nch {
                        let x = f64::from(chans[c][i]);
                        let y = self.lpf.tick(c, self.hpf.tick(c, x));
                        lvl = lvl.max(y.abs());
                    }
                }
            }
            self.env = flush(lvl.max(self.env * self.dec));
            let level_db = 20.0 * self.env.max(1e-12).log10();
            let target = if level_db >= thr {
                self.hold_left = self.hold_samples;
                0.0
            } else if self.hold_left > 0 {
                self.hold_left -= 1;
                0.0
            } else {
                -(slope * (thr - level_db)).min(range)
            };
            let coef = if target > self.gain_db { self.att } else { self.rel };
            self.gain_db = flush(self.gain_db + coef * (target - self.gain_db)).min(0.0);
            if self.gain_db < self.gr_min {
                self.gr_min = self.gain_db;
            }
            let g = (self.gain_db * std::f64::consts::LN_10 / 20.0).exp();
            for ch in chans.iter_mut().take(nch) {
                ch[i] = (f64::from(ch[i]) * g) as f32;
            }
        }
        self.hpf.flush_state();
        self.lpf.flush_state();
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

    fn rms_db(x: &[f32]) -> f64 {
        let p = x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64;
        10.0 * p.max(1e-30).log10()
    }

    fn run(p: &GateParams, x: &[f32], key: Option<&[f32]>) -> Vec<f32> {
        let mut g = Gate::new(FS);
        g.set_params(p);
        let mut y = x.to_vec();
        match key {
            Some(k) => g.process(&mut [&mut y[..]], Some(&[k])),
            None => g.process(&mut [&mut y[..]], None),
        }
        y
    }

    /// req: FX-13, FX-24
    #[test]
    fn steady_state_below_threshold_is_range() {
        // 1 kHz at −60 dBFS peak, threshold −30, ratio 20, range 40: the target is clamped to −40 dB, so the
        // ripple of the detector does not matter. The one-pole settles in 150 ms ≫ 1 s of margin; ±0.5 dB.
        let p = GateParams { threshold_db: -30.0, range_db: 40.0, ratio: 20.0, release_ms: 20.0, ..Default::default() };
        let x = sine(48000, -60.0, 1000.0);
        let y = run(&p, &x, None);
        let d = rms_db(&y[24000..]) - rms_db(&x[24000..]);
        assert!((d + 40.0).abs() <= 0.5, "depth {d}");
    }

    /// req: FX-13
    #[test]
    fn above_threshold_is_unchanged() {
        // −10 dBFS against a −30 dB threshold: after the 1 ms opening the gain is 0 dB; 1e-4 allows the residue.
        let x = sine(24000, -10.0, 1000.0);
        let y = run(&GateParams { threshold_db: -30.0, ..Default::default() }, &x, None);
        let m = x.iter().zip(&y).skip(4800).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
        assert!(m <= 1e-4, "diff {m}");
    }

    /// req: FX-13, FX-24
    #[test]
    fn neutral_range_is_identity() {
        let x: Vec<f32> = xorshift32(5, 20000).iter().map(|v| (v * 0.3) as f32).collect();
        let y = run(&GateParams { range_db: 0.0, threshold_db: 0.0, ..Default::default() }, &x, None);
        let m = x.iter().zip(&y).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
        assert!(m <= 1e-6, "diff {m}");
    }

    /// req: FX-13
    #[test]
    fn sidechain_key_opens_the_gate() {
        let p = GateParams { threshold_db: -30.0, range_db: 40.0, release_ms: 20.0, ..Default::default() };
        let x = sine(48000, -60.0, 440.0);
        let k = sine(48000, -10.0, 1000.0);
        let closed = run(&p, &x, None);
        let open = run(&p, &x, Some(&k));
        let dc = rms_db(&closed[24000..]) - rms_db(&x[24000..]);
        let dopen = rms_db(&open[24000..]) - rms_db(&x[24000..]);
        assert!(dc < -39.0, "closed {dc}");
        assert!(dopen.abs() <= 0.1, "open {dopen}");
    }

    /// A mono signal keyed by a stereo source that plays on the right only: the gate still opens.
    /// req: FX-13
    #[test]
    fn a_key_on_the_right_channel_opens_a_mono_gate() {
        let p = GateParams { threshold_db: -30.0, range_db: 40.0, release_ms: 20.0, ..Default::default() };
        let x = sine(48000, -60.0, 440.0);
        let k = sine(48000, -10.0, 1000.0);
        let silent = vec![0.0f32; 48000];
        let mut g = Gate::new(FS);
        g.set_params(&p);
        let mut y = x.clone();
        g.process(&mut [&mut y[..]], Some(&[&silent[..], &k[..]]));
        let d = rms_db(&y[24000..]) - rms_db(&x[24000..]);
        assert!(d.abs() <= 0.1, "open {d}");
    }

    /// req: FX-13
    #[test]
    fn hold_keeps_it_open_then_releases() {
        // 0.3 s at −10 dBFS then −60 dBFS; threshold −30, hold 200 ms, release 20 ms, range 40.
        let p =
            GateParams { threshold_db: -30.0, range_db: 40.0, hold_ms: 200.0, release_ms: 20.0, ..Default::default() };
        let mut x = sine(14400, -10.0, 1000.0);
        x.extend(sine(48000, -60.0, 1000.0));
        let y = run(&p, &x, None);
        let at = |a: usize, b: usize| rms_db(&y[a..b]) - rms_db(&x[a..b]);
        // 14400 + 0.05 s … +0.15 s: still held open.
        assert!(at(16800, 21600).abs() <= 0.5, "hold {}", at(16800, 21600));
        // 0.3 s after the end of the hold (15 release time constants): closed to −range.
        assert!((at(14400 + 24000 + 14400, 62400) + 40.0).abs() <= 0.5);
    }

    /// req: FX-13
    #[test]
    fn meter_reports_deepest_gain() {
        let p = GateParams { threshold_db: -30.0, range_db: 40.0, ..Default::default() };
        let mut g = Gate::new(FS);
        g.set_params(&p);
        let mut y = sine(48000, -60.0, 1000.0);
        g.process(&mut [&mut y[..]], None);
        let gr = g.take_gr_db();
        assert!((gr + 40.0).abs() <= 0.5, "gr {gr}");
        assert!(g.take_gr_db() >= -40.0);
    }

    /// req: FX-13, FX-24
    #[test]
    fn block_size_does_not_matter() {
        let x: Vec<f32> = xorshift32(7, 30000)
            .iter()
            .enumerate()
            .map(|(i, v)| (v * 0.5 * (1.0 + (i as f64 / 3000.0).sin())) as f32)
            .collect();
        let p = GateParams { threshold_db: -12.0, range_db: 30.0, hold_ms: 5.0, ..Default::default() };
        let whole = |chunk: usize| {
            let mut g = Gate::new(FS);
            g.set_params(&p);
            let mut y = x.clone();
            for c in y.chunks_mut(chunk) {
                g.process(&mut [c], None);
            }
            y
        };
        let a = whole(32);
        for chunk in [1000, 7] {
            let b = whole(chunk);
            let m = a.iter().zip(&b).map(|(u, v)| (u - v).abs()).fold(0.0f32, f32::max);
            assert!(m <= 1e-6, "chunk {chunk}: {m}");
        }
    }

    /// req: FX-13, FX-24
    #[test]
    fn extreme_settings_and_parameter_changes_stay_bounded() {
        // The gain never exceeds 0 dB, so |out| ≤ |in| ≤ 1 (plus rounding).
        let n = 48000 * 3;
        let l: Vec<f32> = xorshift32(1, n).iter().map(|v| *v as f32).collect();
        let r: Vec<f32> = xorshift32(2, n).iter().map(|v| *v as f32).collect();
        let (mut l, mut r) = (l, r);
        let mut g = Gate::new(FS);
        for (k, (cl, cr)) in l.chunks_mut(32).zip(r.chunks_mut(32)).enumerate() {
            let t = (k % 7) as f64 / 6.0;
            g.set_params(&GateParams {
                threshold_db: -80.0 + 80.0 * t,
                range_db: 80.0 * (1.0 - t),
                ratio: 1.0 + 19.0 * t,
                attack_ms: 0.05 + 50.0 * (1.0 - t),
                hold_ms: 500.0 * t,
                release_ms: 5.0 + 2000.0 * t,
                detector_hpf_hz: 20.0 + 1980.0 * t,
                detector_lpf_hz: 1000.0 + 19000.0 * (1.0 - t),
            });
            g.process(&mut [cl, cr], None);
        }
        for v in l.iter().chain(r.iter()) {
            assert!(v.is_finite() && v.abs() <= 1.0 + 1e-6);
        }
    }
}
