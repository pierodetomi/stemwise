//! Measures of a reverb's impulse response, used by the tests of FX-9: decay time, echo density, decorrelation.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};

/// Band-limits a signal with a Butterworth high-pass and low-pass (12 dB/octave each).
pub fn band(x: &[f32], lo: f64, hi: f64, rate: f64) -> Vec<f64> {
    let mut y: Vec<f64> = x.iter().map(|v| f64::from(*v)).collect();
    let mut hp = Biquad::new(Coeffs::design(Kind::HighPass, lo, 0.0, BUTTERWORTH_Q, rate));
    let mut lp = Biquad::new(Coeffs::design(Kind::LowPass, hi, 0.0, BUTTERWORTH_Q, rate));
    hp.filter_f64(0, &mut y);
    lp.filter_f64(0, &mut y);
    y
}

/// RT60 in seconds from the Schroeder energy decay curve of an impulse response already band-limited: the time
/// between −5 and −35 dB, doubled (T30). `None` if the response does not fall 35 dB within its length.
pub fn rt60(ir: &[f64], rate: f64) -> Option<f64> {
    let mut edc = vec![0.0f64; ir.len()];
    let mut acc = 0.0;
    for i in (0..ir.len()).rev() {
        acc += ir[i] * ir[i];
        edc[i] = acc;
    }
    let total = edc[0];
    if total <= 0.0 {
        return None;
    }
    let cross = |db: f64| edc.iter().position(|e| *e <= total * 10f64.powf(db / 10.0));
    let (a, b) = (cross(-5.0)?, cross(-35.0)?);
    (b + 1 < ir.len()).then(|| 2.0 * (b - a) as f64 / rate)
}

/// Periodicity of the tail of an impulse response, from its normalized autocorrelation `r` over lags from
/// `lag_lo` to `lag_hi` seconds. Returns `(peak, repeat)`: the largest `|r(lag)|`, and the largest
/// `min(|r(lag)|, |r(2·lag)|)`, which is high only when the same period comes back again and again (a comb: a
/// repeating echo, or a pitched, metallic resonance). The tail (`from`..`from + len` seconds) is flattened with
/// its own running level first; a dense, noise-like tail gives a few hundredths.
pub fn periodicity(ir: &[f32], rate: f64, from: f64, len: f64, lag_lo: f64, lag_hi: f64) -> (f64, f64) {
    let a = (from * rate) as usize;
    let n = (len * rate) as usize;
    let hi = (lag_hi * rate) as usize;
    // Flatten with the running RMS over 40 ms, so the measure does not depend on the decay being a pure
    // exponential.
    let raw: Vec<f64> = (0..n + 2 * hi).map(|i| ir.get(a + i).map_or(0.0, |v| f64::from(*v))).collect();
    let half = (0.02 * rate) as usize;
    let mut cum = vec![0.0f64; raw.len() + 1];
    for (i, v) in raw.iter().enumerate() {
        cum[i + 1] = cum[i] + v * v;
    }
    let x: Vec<f64> = (0..raw.len())
        .map(|i| {
            let (lo, hi) = (i.saturating_sub(half), (i + half).min(raw.len()));
            let e = ((cum[hi] - cum[lo]) / (hi - lo) as f64).sqrt();
            if e > 0.0 { raw[i] / e } else { 0.0 }
        })
        .collect();
    let energy: f64 = x[..n].iter().map(|v| v * v).sum();
    let r: Vec<f64> = (0..=2 * hi)
        .map(|lag| {
            let e2: f64 = x[lag..lag + n].iter().map(|v| v * v).sum();
            let c: f64 = x[..n].iter().zip(&x[lag..lag + n]).map(|(p, q)| p * q).sum();
            (c / (energy * e2).sqrt()).abs()
        })
        .collect();
    let lo = (lag_lo * rate) as usize;
    let peak = r[lo..=hi].iter().cloned().fold(0.0, f64::max);
    let repeat = (lo..=hi).map(|lag| r[lag].min(r[2 * lag])).fold(0.0, f64::max);
    (peak, repeat)
}

/// Correlation of two signals over a range of seconds.
pub fn correlation(l: &[f32], r: &[f32], rate: f64, from: f64, to: f64) -> f64 {
    let (a, b) = ((from * rate) as usize, ((to * rate) as usize).min(l.len()));
    let (mut ll, mut rr, mut lr) = (0.0f64, 0.0f64, 0.0f64);
    for i in a..b {
        let (x, y) = (f64::from(l[i]), f64::from(r[i]));
        ll += x * x;
        rr += y * y;
        lr += x * y;
    }
    lr / (ll * rr).sqrt()
}
