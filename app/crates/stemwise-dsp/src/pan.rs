//! Pan laws (TRK-2, REN-3 of the PoC): constant power for mono, balance for stereo.

use std::f64::consts::FRAC_PI_4;

/// `(L, R)` gains of a mono source: θ = (p + 1)·π/4, L = cos θ, R = sin θ; exact at −1, 0, +1.
pub fn mono_gains(pan: f64) -> (f64, f64) {
    if pan <= -1.0 {
        return (1.0, 0.0);
    }
    if pan >= 1.0 {
        return (0.0, 1.0);
    }
    if pan == 0.0 {
        return (std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2);
    }
    let t = (pan + 1.0) * FRAC_PI_4;
    (t.cos(), t.sin())
}

/// `(L, R)` gains of a stereo source: L·min(1, 1 − p), R·min(1, 1 + p).
pub fn balance_gains(pan: f64) -> (f64, f64) {
    let p = pan.clamp(-1.0, 1.0);
    ((1.0 - p).min(1.0), (1.0 + p).min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::fixture;

    /// req: TRK-2
    #[test]
    fn pan_laws_match_the_poc_oracle() {
        // Tolerance ±0.01 dB (FOUNDATION §5.7); we check the linear gains to 1e-12.
        for c in fixture("pan.json")["cases"].as_array().unwrap() {
            let p = c["pan"].as_f64().unwrap();
            let m = mono_gains(p);
            let b = balance_gains(p);
            let mg: Vec<f64> = c["mono_gains"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            let bg: Vec<f64> = c["balance_gains"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            assert!((m.0 - mg[0]).abs() < 1e-12 && (m.1 - mg[1]).abs() < 1e-12, "{p}");
            assert!((b.0 - bg[0]).abs() < 1e-12 && (b.1 - bg[1]).abs() < 1e-12, "{p}");
            // Constant power for mono.
            assert!((m.0 * m.0 + m.1 * m.1 - 1.0).abs() < 1e-12);
        }
    }
}
