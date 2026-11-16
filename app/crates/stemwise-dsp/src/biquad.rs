//! RBJ biquads (FX-2): the same formulas as the PoC (poc/docs/analysis.md, "Filtri ed EQ").
//!
//! Coefficients are computed and the filter runs in f64, transposed direct form II, one state per channel.

use std::f64::consts::PI;

pub const BUTTERWORTH_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    HighPass,
    LowPass,
    Peak,
    LowShelf,
    HighShelf,
}

/// Normalised coefficients (`a0` = 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coeffs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Coeffs {
    pub const IDENTITY: Coeffs = Coeffs { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 };

    /// RBJ cookbook design. `freq` is clamped below Nyquist, `q` above zero.
    pub fn design(kind: Kind, freq: f64, gain_db: f64, q: f64, rate: f64) -> Coeffs {
        let f0 = freq.clamp(1.0, rate * 0.4999);
        let q = q.max(1e-3);
        let w0 = 2.0 * PI * f0 / rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a = 10f64.powf(gain_db / 40.0);
        let (b0, b1, b2, a0, a1, a2) = match kind {
            Kind::HighPass => {
                ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha)
            }
            Kind::LowPass => ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            Kind::Peak => (1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a),
            Kind::LowShelf => {
                let k = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos + k),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                    a * ((a + 1.0) - (a - 1.0) * cos - k),
                    (a + 1.0) + (a - 1.0) * cos + k,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                    (a + 1.0) + (a - 1.0) * cos - k,
                )
            }
            Kind::HighShelf => {
                let k = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos + k),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                    a * ((a + 1.0) + (a - 1.0) * cos - k),
                    (a + 1.0) - (a - 1.0) * cos + k,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos),
                    (a + 1.0) - (a - 1.0) * cos - k,
                )
            }
        };
        Coeffs { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0 }
    }

    /// Magnitude response in dB at `freq`.
    pub fn response_db(&self, freq: f64, rate: f64) -> f64 {
        let w = 2.0 * PI * freq / rate;
        let (s1, c1) = w.sin_cos();
        let (s2, c2) = (2.0 * w).sin_cos();
        let nr = self.b0 + self.b1 * c1 + self.b2 * c2;
        let ni = -(self.b1 * s1 + self.b2 * s2);
        let dr = 1.0 + self.a1 * c1 + self.a2 * c2;
        let di = -(self.a1 * s1 + self.a2 * s2);
        10.0 * ((nr * nr + ni * ni) / (dr * dr + di * di)).log10()
    }
}

/// A biquad with state for up to two channels.
#[derive(Debug, Clone)]
pub struct Biquad {
    pub c: Coeffs,
    z: [[f64; 2]; 2],
}

impl Default for Biquad {
    fn default() -> Self {
        Biquad { c: Coeffs::IDENTITY, z: [[0.0; 2]; 2] }
    }
}

impl Biquad {
    pub fn new(c: Coeffs) -> Self {
        Biquad { c, z: [[0.0; 2]; 2] }
    }

    pub fn reset(&mut self) {
        self.z = [[0.0; 2]; 2];
    }

    #[inline]
    pub fn tick(&mut self, ch: usize, x: f64) -> f64 {
        let c = &self.c;
        let z = &mut self.z[ch];
        let y = c.b0 * x + z[0];
        z[0] = c.b1 * x - c.a1 * y + z[1];
        z[1] = c.b2 * x - c.a2 * y;
        y
    }

    /// Keeps denormals out of the state after long silences (once per block, not per sample).
    #[inline]
    pub fn flush_state(&mut self) {
        for z in &mut self.z {
            for v in z.iter_mut() {
                if v.abs() < 1e-30 {
                    *v = 0.0;
                }
            }
        }
    }

    pub fn process(&mut self, ch: usize, buf: &mut [f32]) {
        let c = self.c;
        let [mut z0, mut z1] = self.z[ch];
        for s in buf.iter_mut() {
            let x = f64::from(*s);
            let y = c.b0 * x + z0;
            z0 = c.b1 * x - c.a1 * y + z1;
            z1 = c.b2 * x - c.a2 * y;
            *s = y as f32;
        }
        self.z[ch] = [z0, z1];
        self.flush_state();
    }

    /// Filters a whole f64 signal from a zero state (offline measures).
    pub fn filter_f64(&mut self, ch: usize, buf: &mut [f64]) {
        for s in buf {
            *s = self.tick(ch, *s);
        }
        self.flush_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::fixture;

    fn kind(s: &str) -> Kind {
        match s {
            "hpf" => Kind::HighPass,
            "lpf" => Kind::LowPass,
            "peak" => Kind::Peak,
            "low_shelf" => Kind::LowShelf,
            "high_shelf" => Kind::HighShelf,
            _ => panic!("{s}"),
        }
    }

    /// req: FX-2
    #[test]
    fn coefficients_and_responses_match_the_poc_oracle() {
        // Tolerance ±0.01 dB (FOUNDATION §5.7); coefficients agree to 1e-12 (same formulas, f64).
        let fx = fixture("biquad.json");
        let cases = fx["cases"].as_array().unwrap();
        assert!(cases.len() >= 30);
        for c in cases {
            let rate = c["fs"].as_f64().unwrap();
            let k = Coeffs::design(
                kind(c["kind"].as_str().unwrap()),
                c["freq_hz"].as_f64().unwrap(),
                c["gain_db"].as_f64().unwrap(),
                c["q"].as_f64().unwrap(),
                rate,
            );
            let sos: Vec<f64> = c["coefficients"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            for (mine, theirs) in [k.b0, k.b1, k.b2, 1.0, k.a1, k.a2].iter().zip(&sos) {
                assert!((mine - theirs).abs() < 1e-12, "{}: {mine} vs {theirs}", c["id"]);
            }
            for p in c["response"].as_array().unwrap() {
                let f = p["freq_hz"].as_f64().unwrap();
                let want = p["db"].as_f64().unwrap();
                let got = k.response_db(f, rate);
                assert!((got - want).abs() < 0.01, "{} at {f} Hz: {got} vs {want}", c["id"]);
            }
        }
    }

    /// req: FX-2
    #[test]
    fn filtering_a_sine_gives_the_designed_gain() {
        let rate = 48000.0;
        let mut b = Biquad::new(Coeffs::design(Kind::Peak, 1000.0, 6.0, 1.0, rate));
        let mut x: Vec<f32> = (0..48000).map(|n| (0.25 * (2.0 * PI * 1000.0 * n as f64 / rate).sin()) as f32).collect();
        b.process(0, &mut x);
        let peak = x[24000..].iter().fold(0f32, |a, v| a.max(v.abs()));
        assert!((20.0 * (f64::from(peak) / 0.25).log10() - 6.0).abs() < 0.02);
    }
}
