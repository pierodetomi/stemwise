//! Evaluation of a mix against the target profile (EVA-1 … EVA-4 of the PoC, poc/docs/analysis.md "Valutazione"):
//! spectral distance after level alignment, width and dynamics deltas, technical constraints.

use serde::{Deserialize, Serialize};

use crate::measures::{MixMeasures, Target};
use crate::spectral::N_BANDS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraints {
    pub true_peak_ok: bool,
    pub low_corr_ok: bool,
    pub dc_ok: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    pub spectral_distance_db: Option<f64>,
    pub band_delta_db: Option<[f64; N_BANDS]>,
    pub width_delta_db: Option<f64>,
    pub dyn_delta_lu: Option<f64>,
    pub constraints: Constraints,
    /// Integrated loudness and true peak of the evaluated mix, for the proposal's before/after (AI-2).
    #[serde(default)]
    pub lufs: Option<f64>,
    #[serde(default)]
    pub true_peak_db: Option<f64>,
}

impl Evaluation {
    pub fn valid(&self) -> bool {
        self.constraints.true_peak_ok && self.constraints.low_corr_ok && self.constraints.dc_ok
    }
}

fn lim(v: f64) -> f64 {
    v.clamp(-120.0, 120.0)
}

/// EVA-2: mean absolute band difference after aligning the mean levels; `band_delta` positive = the mix lacks
/// energy in that band.
pub fn spectral_distance(mix: &[f64; N_BANDS], target: &[f64; N_BANDS]) -> (f64, [f64; N_BANDS]) {
    let m: [f64; N_BANDS] = mix.map(lim);
    let t: [f64; N_BANDS] = target.map(lim);
    let offset = t.iter().sum::<f64>() / N_BANDS as f64 - m.iter().sum::<f64>() / N_BANDS as f64;
    let delta: [f64; N_BANDS] = std::array::from_fn(|i| t[i] - (m[i] + offset));
    (delta.iter().map(|d| d.abs()).sum::<f64>() / N_BANDS as f64, delta)
}

/// Evaluates a mix (`dc_offset_pct` from the mix samples) against an optional target and a true-peak ceiling.
pub fn evaluate(mix: &MixMeasures, dc_offset_pct: f64, target: Option<&Target>, ceiling_db: f64) -> Evaluation {
    let (dist, delta) = match (mix.bands_db.as_ref(), target.and_then(|t| t.bands_db.as_ref())) {
        (Some(m), Some(t)) => {
            let (d, b) = spectral_distance(m, t);
            (Some(d), Some(b))
        }
        _ => (None, None),
    };
    let width = match (target.and_then(|t| t.side_mid_db), mix.stereo.and_then(|s| s.side_mid_db)) {
        (Some(t), Some(m)) => Some(lim(t) - lim(m)),
        _ => None,
    };
    let dynd = match (target.and_then(|t| t.dyn_range_lu), mix.dyn_range_lu) {
        (Some(t), Some(m)) => Some(t - m),
        _ => None,
    };
    Evaluation {
        spectral_distance_db: dist,
        band_delta_db: delta,
        width_delta_db: width,
        dyn_delta_lu: dynd,
        constraints: Constraints {
            true_peak_ok: mix.true_peak_db <= ceiling_db,
            low_corr_ok: mix.low_corr.is_none_or(|c| c > 0.0),
            dc_ok: dc_offset_pct <= 0.1,
        },
        lufs: mix.lufs.filter(|v| v.is_finite()),
        true_peak_db: mix.true_peak_db.is_finite().then_some(mix.true_peak_db),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: AI-1
    #[test]
    fn distance_aligns_levels_and_reports_missing_energy() {
        let t = [-12.5, -9.0, -9.6, -10.8, -7.5, -11.0, -14.2, -19.0];
        // The same shape 6 dB lower everywhere: distance 0.
        let (d, delta) = spectral_distance(&t.map(|v| v - 6.0), &t);
        assert!(d.abs() < 1e-12 && delta.iter().all(|x| x.abs() < 1e-12));
        // The PoC example (PRD §6.2): mix vs target.
        let m = [-14.0, -9.5, -8.8, -9.9, -7.1, -12.4, -16.0, -21.5];
        let (d, delta) = spectral_distance(&m, &t);
        assert!(d > 0.5 && d < 2.0, "{d}");
        assert!(delta[7] > 0.0, "the mix lacks highs");
        assert!((delta.iter().sum::<f64>()).abs() < 1e-9);
    }
}
