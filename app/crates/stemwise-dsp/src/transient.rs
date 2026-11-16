//! Transient shaper (FX-16, FX-24).
//!
//! No threshold: everything works on ratios between envelopes, so the result does not depend on the input level.
//!
//! - Base envelope `e`: peak follower of the larger absolute value over the linked channels (instant attack,
//!   10 ms decay), which removes most of the ripple of a rectified waveform.
//! - Two one-pole followers of `e`: fast (`τf = 4 ms · 0.15^speed`) and slow (`τs = 40 ms · 0.3^speed`).
//! - Attack part: `d_att = 20·log10(fast/slow)`, positive on onsets. Sustain part: `d_sus = 20·log10(slow/fast)`,
//!   positive while the signal decays. Each is turned into a 0..1 weight with a dead zone and a ramp
//!   (attack: 1.5 dB dead zone, full at 10 dB; sustain: 0.5 dB, full at 6 dB), so a steady tone has weight 0.
//! - Gain in dB: `attack_db · w_att + sustain_db · w_sus`, smoothed (0.25 ms one-pole), then
//!   `y = x · (1 − mix + mix · g)`. Latency 0.
//!
//! Properties verified by the tests: steady tone unchanged at ±12 dB, louder onset, level independence,
//! neutral settings, block-size invariance, bound and stability.

use crate::util::{flush, one_pole};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransientParams {
    /// Gain applied to onsets at full effect, ±12 dB.
    pub attack_db: f64,
    /// Gain applied to decays, ±12 dB.
    pub sustain_db: f64,
    /// 0..1: how short the attack part is (higher is faster).
    pub speed: f64,
    /// Dry/wet, 0..1.
    pub mix: f64,
}

impl Default for TransientParams {
    fn default() -> Self {
        TransientParams { attack_db: 0.0, sustain_db: 0.0, speed: 0.5, mix: 1.0 }
    }
}

const DECAY_MS: f64 = 10.0;
const GAIN_SMOOTH_MS: f64 = 0.25;
const ATT_DEAD_DB: f64 = 1.5;
const ATT_FULL_DB: f64 = 10.0;
const SUS_DEAD_DB: f64 = 0.5;
const SUS_FULL_DB: f64 = 6.0;
/// Floor of the envelopes, far under any audible level; only keeps the logarithms finite.
const FLOOR: f64 = 1e-9;

pub struct Transient {
    rate: f64,
    p: TransientParams,
    dec: f64,
    kf: f64,
    ks: f64,
    kg: f64,
    env: f64,
    fast: f64,
    slow: f64,
    gain_db: f64,
}

impl Transient {
    pub fn new(rate: f64) -> Self {
        let mut t = Transient {
            rate,
            p: TransientParams::default(),
            dec: 1.0 - one_pole(DECAY_MS, rate),
            kf: 0.0,
            ks: 0.0,
            kg: one_pole(GAIN_SMOOTH_MS, rate),
            env: 0.0,
            fast: 0.0,
            slow: 0.0,
            gain_db: 0.0,
        };
        t.set_params(&TransientParams::default());
        t
    }

    pub fn set_params(&mut self, p: &TransientParams) {
        let mut p = *p;
        p.attack_db = p.attack_db.clamp(-12.0, 12.0);
        p.sustain_db = p.sustain_db.clamp(-12.0, 12.0);
        p.speed = p.speed.clamp(0.0, 1.0);
        p.mix = p.mix.clamp(0.0, 1.0);
        self.p = p;
        self.kf = one_pole(4.0 * 0.15f64.powf(p.speed), self.rate);
        self.ks = one_pole(40.0 * 0.3f64.powf(p.speed), self.rate);
    }

    pub fn reset(&mut self) {
        self.env = 0.0;
        self.fast = 0.0;
        self.slow = 0.0;
        self.gain_db = 0.0;
    }

    pub fn process(&mut self, chans: &mut [&mut [f32]]) {
        let nch = chans.len().min(2);
        if nch == 0 {
            return;
        }
        let n = chans[0].len();
        let (att_db, sus_db, mix) = (self.p.attack_db, self.p.sustain_db, self.p.mix);
        for i in 0..n {
            let mut lvl = 0.0f64;
            for ch in chans.iter().take(nch) {
                lvl = lvl.max(f64::from(ch[i]).abs());
            }
            self.env = flush(lvl.max(self.env * self.dec));
            self.fast = flush(self.fast + self.kf * (self.env - self.fast));
            self.slow = flush(self.slow + self.ks * (self.env - self.slow));
            let target = if self.fast < FLOOR && self.slow < FLOOR {
                0.0
            } else {
                let d = 20.0 * (self.fast.max(FLOOR) / self.slow.max(FLOOR)).log10();
                let w_att = ((d - ATT_DEAD_DB) / (ATT_FULL_DB - ATT_DEAD_DB)).clamp(0.0, 1.0);
                let w_sus = ((-d - SUS_DEAD_DB) / (SUS_FULL_DB - SUS_DEAD_DB)).clamp(0.0, 1.0);
                att_db * w_att + sus_db * w_sus
            };
            self.gain_db = flush(self.gain_db + self.kg * (target - self.gain_db));
            let g = (self.gain_db * std::f64::consts::LN_10 / 20.0).exp();
            let k = 1.0 - mix + mix * g;
            for ch in chans.iter_mut().take(nch) {
                ch[i] = (f64::from(ch[i]) * k) as f32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::xorshift32;
    use std::f64::consts::PI;

    const FS: f64 = 48000.0;

    fn rms_db(x: &[f32]) -> f64 {
        let p = x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len().max(1) as f64;
        10.0 * p.max(1e-30).log10()
    }

    fn run(p: &TransientParams, x: &[f32]) -> Vec<f32> {
        let mut t = Transient::new(FS);
        t.set_params(p);
        let mut y = x.to_vec();
        t.process(&mut [&mut y[..]]);
        y
    }

    /// Silence, then a 1 kHz tone with an exponential decay of 60 ms time constant, peak `amp_db`.
    fn burst(amp_db: f64) -> Vec<f32> {
        let a = 10f64.powf(amp_db / 20.0);
        (0..48000)
            .map(|i| {
                let t = i as f64 / FS - 0.2;
                if t < 0.0 { 0.0 } else { (a * (-t / 0.06).exp() * (2.0 * PI * 1000.0 * t).sin()) as f32 }
            })
            .collect()
    }

    /// req: FX-16, FX-24
    #[test]
    fn steady_tone_keeps_its_level() {
        // A 1 kHz and a 200 Hz tone, at the extreme settings: ±0.1 dB after 0.5 s.
        for freq in [1000.0, 200.0] {
            let x: Vec<f32> = (0..48000).map(|i| (0.3 * (2.0 * PI * freq * i as f64 / FS).sin()) as f32).collect();
            for (a, s) in [(12.0, 12.0), (-12.0, -12.0), (12.0, -12.0), (-12.0, 12.0)] {
                for speed in [0.0, 1.0] {
                    let y = run(&TransientParams { attack_db: a, sustain_db: s, speed, mix: 1.0 }, &x);
                    let d = rms_db(&y[24000..]) - rms_db(&x[24000..]);
                    assert!(d.abs() <= 0.1, "{freq} Hz a {a} s {s} speed {speed}: {d}");
                }
            }
        }
    }

    /// req: FX-16
    #[test]
    fn attack_gain_makes_the_onset_louder() {
        // The first 15 ms of the burst: +6 dB of attack lifts the peak by at least 3 dB, and the same
        // setting at −6 dB lowers it by at least 3 dB.
        let x = burst(-12.0);
        let seg = |y: &[f32]| y[9600..9600 + 720].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let base = seg(&x);
        let up = seg(&run(&TransientParams { attack_db: 6.0, ..Default::default() }, &x));
        let down = seg(&run(&TransientParams { attack_db: -6.0, ..Default::default() }, &x));
        let (du, dd) = (20.0 * (f64::from(up / base)).log10(), 20.0 * (f64::from(down / base)).log10());
        assert!(du >= 3.0, "up {du}");
        assert!(dd <= -3.0, "down {dd}");
    }

    /// req: FX-16, FX-24
    #[test]
    fn result_does_not_depend_on_the_input_level() {
        // The gain curve (output over input, in dB) at −30 and −6 dBFS agrees within 0.5 dB at every
        // sample above −90 dBFS of input (f32 rounding aside; the envelope floor is at −180 dB).
        let p = TransientParams { attack_db: 8.0, sustain_db: -6.0, speed: 0.4, mix: 1.0 };
        let (lo, hi) = (burst(-30.0), burst(-6.0));
        let (ylo, yhi) = (run(&p, &lo), run(&p, &hi));
        let mut worst = 0.0f64;
        for i in 0..lo.len() {
            if f64::from(lo[i].abs()) > 3e-3 && f64::from(hi[i].abs()) > 3e-3 {
                let glo = 20.0 * f64::from(ylo[i] / lo[i]).abs().log10();
                let ghi = 20.0 * f64::from(yhi[i] / hi[i]).abs().log10();
                worst = worst.max((glo - ghi).abs());
            }
        }
        assert!(worst <= 0.5, "worst {worst}");
    }

    /// req: FX-16, FX-24
    #[test]
    fn neutral_settings_are_identity() {
        let x: Vec<f32> = xorshift32(9, 20000).iter().map(|v| (v * 0.5) as f32).collect();
        for p in [
            TransientParams { attack_db: 0.0, sustain_db: 0.0, speed: 0.7, mix: 1.0 },
            TransientParams { attack_db: 12.0, sustain_db: -12.0, speed: 0.7, mix: 0.0 },
        ] {
            let y = run(&p, &x);
            let m = x.iter().zip(&y).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
            assert!(m <= 1e-6, "{p:?}: {m}");
        }
    }

    /// req: FX-16, FX-24
    #[test]
    fn block_size_does_not_matter() {
        let x = burst(-10.0);
        let p = TransientParams { attack_db: 9.0, sustain_db: 5.0, speed: 0.3, mix: 0.8 };
        let go = |chunk: usize| {
            let mut t = Transient::new(FS);
            t.set_params(&p);
            let mut y = x.clone();
            for c in y.chunks_mut(chunk) {
                t.process(&mut [c]);
            }
            y
        };
        let a = go(32);
        for chunk in [1000, 7] {
            let b = go(chunk);
            let m = a.iter().zip(&b).map(|(u, v)| (u - v).abs()).fold(0.0f32, f32::max);
            assert!(m <= 1e-6, "chunk {chunk}: {m}");
        }
    }

    /// req: FX-16, FX-24
    #[test]
    fn extreme_settings_stay_bounded() {
        // The gain is at most +24 dB (attack and sustain weights cannot both be 1 for long, but the sum is the
        // worst case): |y| ≤ 16·|x| ≤ 16, with white noise at full scale.
        let n = 48000 * 3;
        let mut l: Vec<f32> = xorshift32(1, n).iter().map(|v| *v as f32).collect();
        let mut r: Vec<f32> = xorshift32(2, n).iter().map(|v| *v as f32).collect();
        let mut t = Transient::new(FS);
        for (k, (cl, cr)) in l.chunks_mut(32).zip(r.chunks_mut(32)).enumerate() {
            let u = (k % 9) as f64 / 8.0;
            t.set_params(&TransientParams {
                attack_db: 12.0 - 24.0 * u,
                sustain_db: -12.0 + 24.0 * u,
                speed: u,
                mix: 1.0 - 0.5 * u,
            });
            t.process(&mut [cl, cr]);
        }
        for v in l.iter().chain(r.iter()) {
            assert!(v.is_finite() && v.abs() <= 16.0);
        }
    }
}
