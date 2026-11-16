//! Small shared helpers.

/// Linear gain of a level in dB; levels at or below −144 dB are silence.
#[inline]
pub fn db_to_gain(db: f64) -> f64 {
    if db <= -144.0 { 0.0 } else { 10f64.powf(db / 20.0) }
}

/// Level in dB of a linear amplitude (−∞ for 0).
#[inline]
pub fn gain_to_db(g: f64) -> f64 {
    20.0 * g.abs().log10()
}

/// Level in dB of a power (−∞ for 0).
#[inline]
pub fn power_to_db(p: f64) -> f64 {
    10.0 * p.log10()
}

/// Coefficient of a one-pole smoother reaching 1 − 1/e of a step in `ms` milliseconds.
#[inline]
pub fn one_pole(ms: f64, rate: f64) -> f64 {
    if ms <= 0.0 { 1.0 } else { 1.0 - (-1.0 / (ms * 0.001 * rate)).exp() }
}

/// Flushes values too small to matter to zero, so feedback paths never run into denormals.
#[inline]
pub fn flush(x: f64) -> f64 {
    if x.abs() < 1e-30 { 0.0 } else { x }
}

/// Number of frames of the parameter grid: automation and smoothing update at most every this many samples
/// (AUT-6), on a grid anchored at timeline frame 0 so results do not depend on the block size (ENG-4).
pub const SUBBLOCK: usize = 32;
