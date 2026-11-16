//! Master-only blocks (MAS-1): match EQ towards a reference profile and stereo width.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};

/// The 8 analysis bands (ANA-3), in Hz.
pub const BANDS: [(f64, f64); 8] = [
    (20.0, 60.0),
    (60.0, 120.0),
    (120.0, 250.0),
    (250.0, 500.0),
    (500.0, 2000.0),
    (2000.0, 4000.0),
    (4000.0, 8000.0),
    (8000.0, 20000.0),
];

/// Largest correction of one band, in dB.
pub const MATCH_LIMIT_DB: f64 = 6.0;

/// Gains per band moving the mixdown's band profile towards the reference's (MAS-1 "match EQ"): the
/// difference after aligning the mean level (EVA-2 of the PoC), times `amount` (0–1), limited to ±6 dB.
pub fn match_gains(mix_bands_db: &[f64; 8], target_bands_db: &[f64; 8], amount: f64) -> [f64; 8] {
    let offset = target_bands_db.iter().sum::<f64>() / 8.0 - mix_bands_db.iter().sum::<f64>() / 8.0;
    let mut g = [0.0; 8];
    for i in 0..8 {
        let delta = target_bands_db[i] - (mix_bands_db[i] + offset);
        g[i] = (delta * amount.clamp(0.0, 1.0)).clamp(-MATCH_LIMIT_DB, MATCH_LIMIT_DB);
    }
    g
}

/// The filter bank of the match EQ: a low shelf for the first band, peaks at the geometric centres of the six
/// middle bands (Q from their width), a high shelf for the last band.
pub fn match_filters(gains: &[f64; 8], rate: f64) -> [Coeffs; 8] {
    let mut out = [Coeffs::IDENTITY; 8];
    for (i, (lo, hi)) in BANDS.iter().enumerate() {
        let g = gains[i];
        out[i] = match i {
            0 => Coeffs::design(Kind::LowShelf, *hi, g, 0.7, rate),
            7 => Coeffs::design(Kind::HighShelf, *lo, g, 0.7, rate),
            _ => {
                let f0 = (lo * hi).sqrt();
                let octaves = (hi / lo).log2();
                let q = 2f64.powf(octaves / 2.0) / (2f64.powf(octaves) - 1.0);
                Coeffs::design(Kind::Peak, f0, g, q, rate)
            }
        };
    }
    out
}

pub struct MatchEq {
    rate: f64,
    filters: [Biquad; 8],
}

impl MatchEq {
    pub fn new(rate: f64) -> Self {
        MatchEq { rate, filters: std::array::from_fn(|_| Biquad::default()) }
    }

    pub fn set_gains(&mut self, gains: &[f64; 8]) {
        let c = match_filters(gains, self.rate);
        for (f, c) in self.filters.iter_mut().zip(c) {
            f.c = c;
        }
    }

    pub fn reset(&mut self) {
        for f in &mut self.filters {
            f.reset();
        }
    }

    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        for f in &mut self.filters {
            f.process(0, l);
            f.process(1, r);
        }
    }
}

/// Stereo width (MAS-1): gain on the side signal and mono below a frequency (the side is high-passed with a
/// 4th-order Linkwitz–Riley, two Butterworth sections).
pub struct Width {
    rate: f64,
    side_gain: f64,
    mono_below: f64,
    hp: [Biquad; 2],
}

impl Width {
    pub fn new(rate: f64) -> Self {
        Width { rate, side_gain: 1.0, mono_below: 0.0, hp: [Biquad::default(), Biquad::default()] }
    }

    pub fn set_params(&mut self, side_gain_db: f64, mono_below_hz: f64) {
        self.side_gain = crate::util::db_to_gain(side_gain_db);
        if mono_below_hz != self.mono_below {
            self.mono_below = mono_below_hz;
            let c = if mono_below_hz > 0.0 {
                Coeffs::design(Kind::HighPass, mono_below_hz, 0.0, BUTTERWORTH_Q, self.rate)
            } else {
                Coeffs::IDENTITY
            };
            self.hp[0].c = c;
            self.hp[1].c = c;
        }
    }

    pub fn reset(&mut self) {
        self.hp[0].reset();
        self.hp[1].reset();
    }

    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        for (a, b) in l.iter_mut().zip(r.iter_mut()) {
            let m = 0.5 * (f64::from(*a) + f64::from(*b));
            let mut s = 0.5 * (f64::from(*a) - f64::from(*b));
            if self.mono_below > 0.0 {
                let t = self.hp[0].tick(0, s);
                s = self.hp[1].tick(0, t);
            }
            s *= self.side_gain;
            *a = (m + s) as f32;
            *b = (m - s) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(n: usize, freq: f64, phase: f64, rate: f64) -> Vec<f32> {
        (0..n).map(|i| (0.5 * (2.0 * std::f64::consts::PI * freq * i as f64 / rate + phase).sin()) as f32).collect()
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
    }

    /// req: MAS-1
    #[test]
    fn match_gains_follow_the_reference_shape_scaled_by_amount() {
        let mix = [-14.0, -9.5, -8.8, -9.9, -7.1, -12.4, -16.0, -21.5];
        let target = [-12.5, -9.0, -9.6, -10.8, -7.5, -11.0, -14.2, -19.0];
        let full = match_gains(&mix, &target, 1.0);
        let half = match_gains(&mix, &target, 0.5);
        for i in 0..8 {
            assert!((half[i] - full[i] / 2.0).abs() < 1e-12);
        }
        // Mean-aligned: the gains sum to ≈ 0, so the match EQ does not change the overall level much.
        assert!(full.iter().sum::<f64>().abs() < 1e-9);
        assert_eq!(match_gains(&mix, &target, 0.0), [0.0; 8]);
        let extreme = match_gains(&[0.0; 8], &[-30.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 30.0], 1.0);
        assert!(extreme.iter().all(|g| g.abs() <= MATCH_LIMIT_DB));
        // The bank boosts the bands it is asked to: +4 dB at 3 kHz shows at 3 kHz.
        let mut g = [0.0; 8];
        g[5] = 4.0;
        let c = match_filters(&g, 48000.0);
        let at = |f: f64| c.iter().map(|k| k.response_db(f, 48000.0)).sum::<f64>();
        assert!((at(2828.0) - 4.0).abs() < 0.3);
        assert!(at(200.0).abs() < 0.3);
    }

    /// req: MAS-1
    #[test]
    fn width_scales_the_side_and_folds_lows_to_mono() {
        let rate = 48000.0;
        // Anti-phase 1 kHz (pure side): +6 dB side gain doubles it, −∞ removes it.
        let mut l = sine(48000, 1000.0, 0.0, rate);
        let mut r: Vec<f32> = l.iter().map(|v| -v).collect();
        let mut w = Width::new(rate);
        w.set_params(6.0, 0.0);
        w.process(&mut l, &mut r);
        assert!((20.0 * (rms(&l[24000..]) / (0.5 / 2f64.sqrt())).log10() - 6.0).abs() < 0.01);
        // Pure side at 50 Hz with mono below 120 Hz: mostly removed; at 1 kHz: kept.
        let mut l = sine(96000, 50.0, 0.0, rate);
        let mut r: Vec<f32> = l.iter().map(|v| -v).collect();
        let mut w = Width::new(rate);
        w.set_params(0.0, 120.0);
        w.process(&mut l, &mut r);
        assert!(20.0 * (rms(&l[48000..]) / (0.5 / 2f64.sqrt())).log10() < -12.0);
        // Mono content is never touched.
        w.reset();
        let mut l = sine(4800, 50.0, 0.0, rate);
        let mut r = l.clone();
        let orig = l.clone();
        w.process(&mut l, &mut r);
        assert!(l.iter().zip(&orig).all(|(a, b)| (a - b).abs() < 1e-6));
    }
}
