//! TPDF dither for 16-bit export (MAS-1, MAS-7): the sum of two independent uniform values of ±½ LSB, from a
//! seeded xorshift32 generator, so the same master gives the same file (MAS-8).

pub struct Dither {
    state: u32,
}

impl Dither {
    pub fn new(seed: u32) -> Self {
        Dither { state: if seed == 0 { 0x2545_F491 } else { seed } }
    }

    #[inline]
    fn uniform(&mut self) -> f64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        f64::from(x) / 4_294_967_296.0 - 0.5
    }

    /// Quantises a sample to `bits` with TPDF dither; returns the integer code.
    #[inline]
    pub fn quantise(&mut self, x: f64, bits: u32) -> i32 {
        let scale = f64::from(1u32 << (bits - 1));
        let d = self.uniform() + self.uniform();
        (x * scale + d).round().clamp(-scale, scale - 1.0) as i32
    }
}

/// Plain rounding without dither (24 bit and above).
#[inline]
pub fn quantise_plain(x: f64, bits: u32) -> i32 {
    let scale = f64::from(1u32 << (bits - 1));
    (x * scale).round().clamp(-scale, scale - 1.0) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: MAS-1, MAS-8
    #[test]
    fn tpdf_dither_is_deterministic_unbiased_and_one_lsb_wide() {
        let mut a = Dither::new(42);
        let mut b = Dither::new(42);
        let xs: Vec<i32> = (0..10_000).map(|_| a.quantise(0.0, 16)).collect();
        let ys: Vec<i32> = (0..10_000).map(|_| b.quantise(0.0, 16)).collect();
        assert_eq!(xs, ys);
        // Silence becomes −1, 0, +1 LSB with mean ≈ 0 and variance ≈ 1/6 + rounding (≈ 0.25 for TPDF ±1 LSB).
        assert!(xs.iter().all(|v| v.abs() <= 1));
        let mean = xs.iter().map(|&v| f64::from(v)).sum::<f64>() / xs.len() as f64;
        let var = xs.iter().map(|&v| (f64::from(v) - mean).powi(2)).sum::<f64>() / xs.len() as f64;
        assert!(mean.abs() < 0.02, "{mean}");
        assert!((var - 0.25).abs() < 0.03, "{var}");
        // A level between two codes is reproduced on average (no truncation distortion).
        let mut d = Dither::new(7);
        let x = 0.3 / 32768.0;
        let avg = (0..100_000).map(|_| f64::from(d.quantise(x, 16))).sum::<f64>() / 100_000.0;
        assert!((avg - 0.3).abs() < 0.02, "{avg}");
        assert_eq!(quantise_plain(1.0, 24), (1 << 23) - 1);
    }
}
