//! Feed-forward compressor with a stereo-linked detector (FX-3, MAS-1; closes D-034 of the PoC).
//!
//! - Detector: per sample, the larger absolute value over the channels of the key signal (the processed signal
//!   itself, or a sidechain source through a Butterworth high-pass). One detector for both channels: stereo is
//!   compressed in a linked way.
//! - Gain computer (in dB) with a soft knee of width `W` (Giannoulis, Massberg & Reiss 2012):
//!   below `T − W/2` no change; above `T + W/2`, `T + (x − T)/R`; in between the quadratic
//!   `x + (1/R − 1)(x − T + W/2)² / (2W)`.
//! - Detector high-pass (MAS-13): without a key, `detector_hpf_hz > 0` filters the detector input (2nd-order
//!   Butterworth, own state per channel); the audio path is untouched. With a key, `sidechain_hpf_hz` applies.
//! - Ballistics: smoothed gain reduction in dB, branching one-pole: attack coefficient `exp(−1/(τa·fs))` when the
//!   reduction grows, release `exp(−1/(τr·fs))` when it shrinks.
//! - Makeup: a gain after the compressor, smoothed per sample (20 ms) so changes do not click.
//! - Meter: the deepest reduction since the last `take_gr_db`.
//!
//! Properties verified by the tests: static curve and knee, attack and release times, linking, sidechain with
//! its high-pass, makeup, stability and block-size invariance.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};
use crate::util::{db_to_gain, one_pole};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressorParams {
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    pub knee_db: f64,
    pub makeup_db: f64,
    /// High-pass on the detector input when a sidechain key is used.
    pub sidechain_hpf_hz: f64,
    /// High-pass (2nd-order Butterworth) on the detector input when NO sidechain key is used; 0 = off. The audio
    /// path is untouched: it only keeps low-frequency energy from driving the gain (MAS-13).
    pub detector_hpf_hz: f64,
    /// Dry/wet, 0..1: 1 is the compressed signal alone, lower values blend the uncompressed signal back in,
    /// sample-aligned (parallel compression).
    pub mix: f64,
}

impl Default for CompressorParams {
    fn default() -> Self {
        CompressorParams {
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 120.0,
            knee_db: 6.0,
            makeup_db: 0.0,
            sidechain_hpf_hz: 100.0,
            detector_hpf_hz: 0.0,
            mix: 1.0,
        }
    }
}

pub struct Compressor {
    rate: f64,
    p: CompressorParams,
    att: f64,
    rel: f64,
    env_db: f64,
    makeup: f64,
    makeup_target: f64,
    makeup_k: f64,
    mix: f64,
    mix_target: f64,
    key_hpf: Biquad,
    key_hpf_freq: f64,
    det_hpf: Biquad,
    det_hpf_freq: f64,
    gr_meter: f64,
    /// Linear level under which the gain computer returns no reduction (threshold − knee/2).
    knee_floor: f64,
}

/// Static gain reduction in dB (≤ 0) for a detector level `x` in dB.
pub fn static_gr(x: f64, p: &CompressorParams) -> f64 {
    let (t, r, w) = (p.threshold_db, p.ratio.max(1.0), p.knee_db.max(0.0));
    let d = x - t;
    let y = if 2.0 * d < -w {
        x
    } else if w > 0.0 && 2.0 * d.abs() <= w {
        x + (1.0 / r - 1.0) * (d + w / 2.0).powi(2) / (2.0 * w)
    } else {
        t + d / r
    };
    (y - x).min(0.0)
}

impl Compressor {
    pub fn new(rate: f64) -> Self {
        let mut c = Compressor {
            rate,
            p: CompressorParams::default(),
            att: 0.0,
            rel: 0.0,
            env_db: 0.0,
            makeup: 1.0,
            makeup_target: 1.0,
            makeup_k: one_pole(20.0, rate),
            mix: 1.0,
            mix_target: 1.0,
            key_hpf: Biquad::default(),
            key_hpf_freq: 0.0,
            det_hpf: Biquad::default(),
            det_hpf_freq: 0.0,
            gr_meter: 0.0,
            knee_floor: 0.0,
        };
        let p = c.p;
        c.set_params(&p);
        c.makeup = c.makeup_target;
        c.mix = c.mix_target;
        c
    }

    pub fn set_params(&mut self, p: &CompressorParams) {
        self.p = *p;
        self.att = (-1.0 / (p.attack_ms.max(0.01) * 0.001 * self.rate)).exp();
        self.rel = (-1.0 / (p.release_ms.max(1.0) * 0.001 * self.rate)).exp();
        self.makeup_target = db_to_gain(p.makeup_db);
        self.mix_target = p.mix.clamp(0.0, 1.0);
        self.knee_floor = 10f64.powf((p.threshold_db - p.knee_db.max(0.0) / 2.0) / 20.0) - 1e-12;
        if p.sidechain_hpf_hz != self.key_hpf_freq {
            self.key_hpf.c = Coeffs::design(Kind::HighPass, p.sidechain_hpf_hz, 0.0, BUTTERWORTH_Q, self.rate);
            self.key_hpf_freq = p.sidechain_hpf_hz;
        }
        if p.detector_hpf_hz != self.det_hpf_freq {
            if p.detector_hpf_hz > 0.0 {
                self.det_hpf.c = Coeffs::design(Kind::HighPass, p.detector_hpf_hz, 0.0, BUTTERWORTH_Q, self.rate);
            }
            self.det_hpf_freq = p.detector_hpf_hz;
        }
    }

    /// Jumps the makeup and the mix to their targets (start of an offline render).
    pub fn settle(&mut self) {
        self.makeup = self.makeup_target;
        self.mix = self.mix_target;
    }

    pub fn reset(&mut self) {
        self.env_db = 0.0;
        self.key_hpf.reset();
        self.det_hpf.reset();
        self.gr_meter = 0.0;
        self.makeup = self.makeup_target;
        self.mix = self.mix_target;
    }

    /// Deepest gain reduction (dB, ≤ 0) since the last call, for the meter.
    pub fn take_gr_db(&mut self) -> f64 {
        std::mem::replace(&mut self.gr_meter, 0.0)
    }

    #[inline]
    fn gain_for(&mut self, level: f64) -> f64 {
        // Fast path: below the knee with no reduction in progress the gain is the makeup alone (no log/exp).
        if self.env_db == 0.0 && level < self.knee_floor && self.makeup == self.makeup_target {
            return self.makeup;
        }
        let x = 20.0 * (level + 1e-12).log10();
        let target = static_gr(x, &self.p);
        let k = if target < self.env_db { self.att } else { self.rel };
        self.env_db = target + k * (self.env_db - target);
        if self.env_db.abs() < 1e-12 {
            self.env_db = 0.0;
        }
        self.gr_meter = self.gr_meter.min(self.env_db);
        self.makeup += self.makeup_k * (self.makeup_target - self.makeup);
        if (self.makeup - self.makeup_target).abs() < 1e-12 {
            self.makeup = self.makeup_target;
        }
        if self.env_db == 0.0 { self.makeup } else { db_to_gain(self.env_db) * self.makeup }
    }

    /// Processes one or two channels in place. `key`, if given, is the sidechain source (one or two channels,
    /// post-fader) and goes through the detector high-pass.
    pub fn process(&mut self, chans: &mut [&mut [f32]], key: Option<&[&[f32]]>) {
        let n = chans.first().map_or(0, |c| c.len());
        for i in 0..n {
            let level = match key {
                Some(k) => {
                    let mut m = 0.0f64;
                    for (c, ch) in k.iter().enumerate() {
                        m = m.max(self.key_hpf.tick(c.min(1), f64::from(ch[i])).abs());
                    }
                    m
                }
                None if self.det_hpf_freq > 0.0 => {
                    let mut m = 0.0f64;
                    for (c, ch) in chans.iter().enumerate() {
                        m = m.max(self.det_hpf.tick(c.min(1), f64::from(ch[i])).abs());
                    }
                    m
                }
                None => chans.iter().fold(0.0f64, |m, c| m.max(f64::from(c[i]).abs())),
            };
            let g = self.gain_for(level);
            // Dry/wet on the gain: x·(1 − mix) + x·g·mix. Both paths are the same sample, so nothing combs.
            if self.mix != self.mix_target {
                self.mix += self.makeup_k * (self.mix_target - self.mix);
                if (self.mix - self.mix_target).abs() < 1e-9 {
                    self.mix = self.mix_target;
                }
            }
            let g = if self.mix == 1.0 { g } else { 1.0 + self.mix * (g - 1.0) };
            for c in chans.iter_mut() {
                c[i] = (f64::from(c[i]) * g) as f32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(n: usize, amp: f64, freq: f64, rate: f64) -> Vec<f32> {
        (0..n).map(|i| (amp * (2.0 * std::f64::consts::PI * freq * i as f64 / rate).sin()) as f32).collect()
    }

    fn peak(x: &[f32]) -> f64 {
        x.iter().fold(0.0f64, |a, v| a.max(f64::from(v.abs())))
    }

    /// req: FX-3
    #[test]
    fn static_curve_and_knee() {
        let p = CompressorParams { threshold_db: -20.0, ratio: 4.0, knee_db: 0.0, ..Default::default() };
        assert_eq!(static_gr(-30.0, &p), 0.0);
        assert!((static_gr(-8.0, &p) - (-9.0)).abs() < 1e-12);
        let k = CompressorParams { knee_db: 6.0, ..p };
        // At the threshold the soft knee already reduces by (1/R − 1)·W/8.
        assert!((static_gr(-20.0, &k) - (0.25 - 1.0) * 6.0 / 8.0).abs() < 1e-12);
        assert_eq!(static_gr(-23.0, &k), 0.0);
        assert!((static_gr(-17.0, &k) - (-17.0_f64 + 20.0) * (0.25 - 1.0)).abs() < 1e-9);
    }

    /// req: FX-3
    #[test]
    fn steady_sine_is_reduced_by_the_static_amount() {
        // A long 1 kHz sine at −10 dBFS peak, threshold −30, ratio 4, no knee: the peak-detector reduction
        // settles at 15 dB; between peaks the release lets it rise a little (PoC D-034 measured 14.2 dB with
        // 10/120 ms). With a 1000 ms release the ripple is below 0.3 dB.
        let rate = 48000.0;
        let mut c = Compressor::new(rate);
        c.set_params(&CompressorParams {
            threshold_db: -30.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 1000.0,
            knee_db: 0.0,
            ..Default::default()
        });
        let mut x = sine(96000, 10f64.powf(-10.0 / 20.0), 1000.0, rate);
        c.process(&mut [&mut x], None);
        let out_db = 20.0 * peak(&x[48000..]).log10();
        assert!((out_db - (-25.0)).abs() < 0.3, "{out_db}");
        assert!((c.take_gr_db() + 15.0).abs() < 0.5);
    }

    /// req: FX-3
    #[test]
    fn attack_and_release_time_constants() {
        let rate = 48000.0;
        let p = CompressorParams {
            threshold_db: -40.0,
            ratio: 20.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let mut c = Compressor::new(rate);
        c.set_params(&p);
        // DC step from silence to 0 dBFS: target reduction ≈ −38 dB; after one attack time 63% of it.
        let mut x = vec![1.0f32; 4800];
        c.process(&mut [&mut x], None);
        let gr_at = |i: usize| 20.0 * f64::from(x[i]).log10();
        let target = static_gr(0.0, &p);
        let at_tau = gr_at(480 - 1) / target;
        assert!((at_tau - 0.632).abs() < 0.02, "{at_tau}");
        // Release: from 30 dB of reduction to a tiny signal; after one release time 36.8% of it is left.
        let mut c2 = Compressor::new(rate);
        c2.set_params(&p);
        c2.env_db = -30.0;
        let mut z = vec![1e-6f32; 4800];
        c2.process(&mut [&mut z], None);
        let after = 20.0 * (f64::from(z[4799]) / 1e-6).log10();
        assert!((after / -30.0 - 0.368).abs() < 0.02, "{after}");
    }

    /// req: FX-3
    #[test]
    fn stereo_is_linked() {
        // Loud left, quiet right: both channels get the same gain (D-034 of the PoC closed).
        let rate = 48000.0;
        let mut c = Compressor::new(rate);
        c.set_params(&CompressorParams {
            threshold_db: -30.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 500.0,
            knee_db: 0.0,
            ..Default::default()
        });
        let mut l = sine(48000, 0.3, 1000.0, rate);
        let mut r = sine(48000, 0.003, 1000.0, rate);
        let (l0, r0) = (l.clone(), r.clone());
        c.process(&mut [&mut l, &mut r], None);
        let gl = peak(&l[24000..]) / peak(&l0[24000..]);
        let gr = peak(&r[24000..]) / peak(&r0[24000..]);
        assert!((20.0 * (gl / gr).log10()).abs() < 0.05, "{gl} {gr}");
        assert!(20.0 * gr.log10() < -10.0);
    }

    /// req: FX-3
    #[test]
    fn sidechain_key_drives_the_gain_through_its_high_pass() {
        let rate = 48000.0;
        let p = CompressorParams {
            threshold_db: -30.0,
            ratio: 8.0,
            attack_ms: 1.0,
            release_ms: 200.0,
            knee_db: 0.0,
            sidechain_hpf_hz: 500.0,
            ..Default::default()
        };
        // Bass at −20 dBFS, quiet itself: no reduction without a key.
        let bass = sine(24000, 0.1, 80.0, rate);
        let mut a = bass.clone();
        let mut c = Compressor::new(rate);
        c.set_params(&CompressorParams { threshold_db: -10.0, ..p });
        c.process(&mut [&mut a], None);
        assert!((peak(&a[12000..]) / 0.1 - 1.0).abs() < 1e-6);
        // A loud 2 kHz key ducks it.
        let key_hi = sine(24000, 0.5, 2000.0, rate);
        let mut b = bass.clone();
        let mut c = Compressor::new(rate);
        c.set_params(&p);
        c.process(&mut [&mut b], Some(&[&key_hi]));
        assert!(20.0 * (peak(&b[12000..]) / 0.1).log10() < -10.0);
        // The same key at 60 Hz is removed by the detector high-pass at 500 Hz: much less ducking.
        let key_lo = sine(24000, 0.5, 60.0, rate);
        let mut d = bass.clone();
        let mut c = Compressor::new(rate);
        c.set_params(&p);
        c.process(&mut [&mut d], Some(&[&key_lo]));
        assert!(peak(&d[12000..]) > 3.0 * peak(&b[12000..]));
    }

    /// req: FX-3
    #[test]
    fn makeup_is_a_smooth_gain_and_processing_is_block_invariant() {
        let rate = 48000.0;
        let p = CompressorParams { threshold_db: 0.0, ratio: 1.0, makeup_db: 6.0, ..Default::default() };
        let mut c = Compressor::new(rate);
        c.set_params(&p);
        c.settle();
        let mut x = vec![0.1f32; 100];
        c.process(&mut [&mut x], None);
        assert!((f64::from(x[50]) / 0.1 - db_to_gain(6.0)).abs() < 1e-6);
        // Same output in one call or in blocks of 64.
        let p = CompressorParams { threshold_db: -24.0, ratio: 5.0, ..Default::default() };
        let noise: Vec<f32> = crate::testsignal::xorshift32(9, 48000).iter().map(|v| (*v * 0.8) as f32).collect();
        let mut a = noise.clone();
        let mut b = noise;
        let mut c1 = Compressor::new(rate);
        c1.set_params(&p);
        c1.process(&mut [&mut a], None);
        let mut c2 = Compressor::new(rate);
        c2.set_params(&p);
        for chunk in b.chunks_mut(64) {
            c2.process(&mut [chunk], None);
        }
        assert_eq!(a, b);
        assert!(a.iter().all(|v| v.is_finite()));
    }

    /// req: FX-3
    #[test]
    fn mix_blends_the_uncompressed_signal_back_in() {
        let rate = 48000.0;
        // A steady sine 20 dB over the threshold at ratio 4 settles 15 dB down; makeup 6 dB: wet gain −9 dB.
        let run = |mix: f64| {
            let mut c = Compressor::new(rate);
            c.set_params(&CompressorParams {
                threshold_db: -26.0,
                ratio: 4.0,
                knee_db: 0.0,
                makeup_db: 6.0,
                mix,
                ..Default::default()
            });
            c.settle();
            let mut x = sine(48000, 0.5, 1000.0, rate);
            c.process(&mut [&mut x], None);
            x
        };
        let peak = |x: &[f32]| x[40000..].iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let dry = sine(48000, 0.5, 1000.0, rate);
        let (wet, half, none) = (run(1.0), run(0.5), run(0.0));
        // Mix 0 is the input, sample for sample.
        assert!(none.iter().zip(&dry).all(|(a, b)| (a - b).abs() < 1e-7));
        // Mix 0.5 is the mean of dry and wet at every sample (same detector, same envelope): tolerance for f32.
        assert!(half.iter().zip(wet.iter().zip(&dry)).all(|(h, (w, d))| (h - (w + d) / 2.0).abs() < 1e-6));
        let wet_db = 20.0 * f64::from(peak(&wet) / peak(&dry)).log10();
        // Peak detector with a 120 ms release on a 1 kHz sine: the gain recovers a little between peaks, so the
        // reduction sits about 1 dB short of the static curve; ±1.5 dB.
        assert!((wet_db + 9.0).abs() < 1.5, "{wet_db}");
        assert!(peak(&half) > peak(&wet) && peak(&half) < peak(&dry));
    }

    /// req: FX-3, AUT-6
    #[test]
    fn mix_changes_are_smoothed() {
        let rate = 48000.0;
        let mut c = Compressor::new(rate);
        let p = CompressorParams { threshold_db: -40.0, ratio: 8.0, knee_db: 0.0, ..Default::default() };
        c.set_params(&p);
        c.settle();
        let mut x = vec![0.5f32; 9600];
        c.process(&mut [&mut x[..4800]], None);
        c.set_params(&CompressorParams { mix: 0.0, ..p });
        c.process(&mut [&mut x[4800..]], None);
        // A jump from fully wet to fully dry on a constant input: no step larger than the 20 ms smoother allows
        // (the whole change is under 0.5, the first step is 1/960 of it; 0.002 leaves room).
        let step = x[4700..].windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        assert!(step < 0.002, "{step}");
        // 100 ms later, five time constants: within 1% of fully dry.
        assert!((x[9599] - 0.5).abs() < 5e-3);
    }

    /// req: MAS-13
    #[test]
    fn detector_high_pass_listens_less_to_the_low_end() {
        let rate = 48000.0;
        let p = CompressorParams {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 1000.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let gr = |freq: f64, hpf: f64| {
            let mut c = Compressor::new(rate);
            c.set_params(&CompressorParams { detector_hpf_hz: hpf, ..p });
            let mut x = sine(96000, 0.5, freq, rate);
            let x0 = x.clone();
            c.process(&mut [&mut x], None);
            // Steady-state reduction from the peaks of the last half second.
            20.0 * (peak(&x[48000..]) / peak(&x0[48000..])).log10()
        };
        // -6 dBFS at 50 Hz: 14 dB over the threshold, about 10 dB of reduction without the filter; the 200 Hz
        // Butterworth is 24 dB down at 50 Hz, so the detector sees it below the threshold. Required: >= 6 dB less.
        let (off, on) = (gr(50.0, 0.0), gr(50.0, 200.0));
        assert!(off < -5.0, "{off}");
        assert!(on - off >= 6.0, "{off} {on}");
        // At 2 kHz the filter is within 0.04 dB of flat: same reduction within 0.2 dB.
        let (a, b) = (gr(2000.0, 0.0), gr(2000.0, 200.0));
        assert!(a < -5.0);
        assert!((a - b).abs() < 0.2, "{a} {b}");
    }
}
