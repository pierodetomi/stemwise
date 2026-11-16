//! Per-signal measures (ANA-1 … ANA-12): port of the PoC's `stemwise.analysis.{stem, mix, masking, reference}`,
//! formulas in poc/docs/analysis.md. Values are full precision; rounding happens only when writing JSON (ANA-13).

use serde::{Deserialize, Serialize};
use stemwise_dsp::{loudness, truepeak};

use crate::spectral::{self, N_BANDS, Spectrum, StereoSums};

pub const CLIP_LEVEL: f32 = 1.0 - 1.0 / 32768.0;
pub const NOISE_FLOOR_MIN_INACTIVE_S: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StereoMeasures {
    pub correlation: Option<f64>,
    pub side_mid_db: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hygiene {
    pub dc_offset_pct: f64,
    pub clipped_samples: usize,
    pub noise_floor_db: Option<f64>,
}

/// ANA-1 … ANA-6 for one stem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemMeasures {
    pub channels: usize,
    pub lufs: Option<f64>,
    pub peak_db: f64,
    pub true_peak_db: f64,
    pub rms_db: Option<f64>,
    pub crest_db: Option<f64>,
    pub dyn_range_lu: Option<f64>,
    pub active_pct: f64,
    pub bands_db: Option<[f64; N_BANDS]>,
    pub centroid_hz: Option<f64>,
    pub onsets_per_s: f64,
    pub stereo: Option<StereoMeasures>,
    pub hygiene: Hygiene,
}

/// ANA-7 and ANA-10 on a mix (also references, ANA-11).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MixMeasures {
    pub lufs: Option<f64>,
    pub peak_db: f64,
    pub true_peak_db: f64,
    pub rms_db: Option<f64>,
    pub crest_db: Option<f64>,
    pub dyn_range_lu: Option<f64>,
    pub bands_db: Option<[f64; N_BANDS]>,
    pub stereo: Option<StereoMeasures>,
    pub low_corr: Option<f64>,
}

fn db(lin: f64) -> f64 {
    20.0 * lin.log10()
}

fn db_power(p: f64) -> f64 {
    if p > 0.0 { 10.0 * p.log10() } else { f64::NEG_INFINITY }
}

pub fn sample_peak_db(samples: &[f32]) -> f64 {
    db(samples.iter().fold(0.0f64, |m, v| m.max(f64::from(v.abs()))))
}

pub fn rms_db(s: &Spectrum) -> Option<f64> {
    let n = s.n_active();
    if n == 0 {
        return None;
    }
    let sum: f64 = s.frame_power.iter().zip(&s.active).filter(|(_, a)| **a).map(|(p, _)| p).sum();
    Some(10.0 * (sum / n as f64).log10())
}

pub fn dc_offset_pct(samples: &[f32], channels: usize) -> f64 {
    let n = samples.len() / channels;
    if n == 0 {
        return 0.0;
    }
    (0..channels)
        .map(|c| (samples.iter().skip(c).step_by(channels).map(|&v| f64::from(v)).sum::<f64>() / n as f64).abs())
        .fold(0.0, f64::max)
        * 100.0
}

pub fn clipped_samples(samples: &[f32]) -> usize {
    samples.iter().filter(|v| v.abs() >= CLIP_LEVEL).count()
}

/// numpy median.
fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { 0.5 * (v[n / 2 - 1] + v[n / 2]) }
}

pub fn noise_floor_db(s: &Spectrum) -> Option<f64> {
    let inactive: Vec<f64> = s.frame_power.iter().zip(&s.active).filter(|(_, a)| !**a).map(|(p, _)| *p).collect();
    let n = inactive.len();
    if n == 0 || n as f64 / s.n_frames() as f64 * s.duration_s() < NOISE_FLOOR_MIN_INACTIVE_S {
        return None;
    }
    Some(db_power(median(inactive)))
}

pub fn correlation(s: &StereoSums) -> Option<f64> {
    if s.ll <= 0.0 && s.rr <= 0.0 {
        return None;
    }
    if s.ll <= 0.0 || s.rr <= 0.0 {
        return Some(0.0);
    }
    Some((s.lr / (s.ll * s.rr).sqrt()).clamp(-1.0, 1.0))
}

pub fn side_mid_db(s: &StereoSums) -> Option<f64> {
    if s.mm <= 0.0 && s.ss <= 0.0 {
        return None;
    }
    if s.mm <= 0.0 {
        return Some(f64::INFINITY);
    }
    Some(db_power(s.ss / s.mm))
}

pub fn stereo_measures(s: &Spectrum) -> Option<StereoMeasures> {
    s.stereo_full.map(|f| StereoMeasures { correlation: correlation(&f), side_mid_db: side_mid_db(&f) })
}

/// ANA-10: L/R correlation over 20–120 Hz and the active frames (1 for mono).
pub fn low_correlation(s: &Spectrum) -> Option<f64> {
    match &s.stereo_low {
        None => (s.n_active() > 0).then_some(1.0),
        Some(l) => correlation(l),
    }
}

/// All per-stem measures of interleaved audio.
pub fn measure_stem(samples: &[f32], channels: usize, rate: u32) -> (StemMeasures, Spectrum) {
    let spec = spectral::analyze(samples, channels, rate);
    let m = measure_stem_with(samples, channels, rate, &spec);
    (m, spec)
}

pub fn measure_stem_with(samples: &[f32], channels: usize, rate: u32, spec: &Spectrum) -> StemMeasures {
    let peak = sample_peak_db(samples);
    let rms = rms_db(spec);
    StemMeasures {
        channels,
        lufs: loudness::integrated(samples, channels, rate),
        peak_db: peak,
        true_peak_db: truepeak::true_peak_db(samples, channels),
        rms_db: rms,
        crest_db: rms.map(|r| peak - r),
        dyn_range_lu: loudness::range(samples, channels, rate),
        active_pct: spectral::active_pct(spec),
        bands_db: spectral::band_levels_db(spec),
        centroid_hz: spectral::centroid_hz(spec),
        onsets_per_s: crate::onsets::onsets_per_s(samples, channels, rate, spec),
        stereo: stereo_measures(spec),
        hygiene: Hygiene {
            dc_offset_pct: dc_offset_pct(samples, channels),
            clipped_samples: clipped_samples(samples),
            noise_floor_db: noise_floor_db(spec),
        },
    }
}

/// The measures of a mix, with its spectrum. The independent parts (spectrum, true peak of each channel, loudness)
/// run on separate threads: the same values as one after the other, in about the time of the longest.
pub fn measure_mix(samples: &[f32], channels: usize, rate: u32) -> (MixMeasures, Spectrum) {
    use rayon::prelude::*;
    let ((spec, tp), (lufs, lra)) = rayon::join(
        || {
            rayon::join(
                || spectral::analyze(samples, channels, rate),
                || {
                    let peak = (0..channels)
                        .into_par_iter()
                        .map(|c| truepeak::channel_peak(samples, channels, c))
                        .reduce(|| 0.0, f64::max);
                    20.0 * peak.log10()
                },
            )
        },
        || rayon::join(|| loudness::integrated(samples, channels, rate), || loudness::range(samples, channels, rate)),
    );
    (measures_from(samples, &spec, tp, lufs, lra), spec)
}

fn measures_from(samples: &[f32], spec: &Spectrum, tp: f64, lufs: Option<f64>, lra: Option<f64>) -> MixMeasures {
    let peak = sample_peak_db(samples);
    let rms = rms_db(spec);
    MixMeasures {
        lufs,
        peak_db: peak,
        true_peak_db: tp,
        rms_db: rms,
        crest_db: rms.map(|r| peak - r),
        dyn_range_lu: lra,
        bands_db: spectral::band_levels_db(spec),
        stereo: stereo_measures(spec),
        low_corr: low_correlation(spec),
    }
}

pub fn measure_mix_with(samples: &[f32], channels: usize, rate: u32, spec: &Spectrum) -> MixMeasures {
    let peak = sample_peak_db(samples);
    let rms = rms_db(spec);
    MixMeasures {
        lufs: loudness::integrated(samples, channels, rate),
        peak_db: peak,
        true_peak_db: truepeak::true_peak_db(samples, channels),
        rms_db: rms,
        crest_db: rms.map(|r| peak - r),
        dyn_range_lu: loudness::range(samples, channels, rate),
        bands_db: spectral::band_levels_db(spec),
        stereo: stereo_measures(spec),
        low_corr: low_correlation(spec),
    }
}

// ---------------------------------------------------------------------------------------------
// Masking and band shares (ANA-8, ANA-9)
// ---------------------------------------------------------------------------------------------

/// Masking score of two signals in each band (`masking v2`). Both must share the frame grid.
pub fn pair_scores(a: &Spectrum, b: &Spectrum) -> [f64; N_BANDS] {
    let n = a.n_frames().min(b.n_frames());
    let mut out = [0.0; N_BANDS];
    for band in 0..N_BANDS {
        let (mut lo, mut up) = (0.0, 0.0);
        for t in 0..n {
            let aa = a.band_power[t][band] > spectral::ACTIVITY_THRESHOLD_POWER;
            let ab = b.band_power[t][band] > spectral::ACTIVITY_THRESHOLD_POWER;
            let (ea, eb) = (a.bus_band_power(t, band), b.bus_band_power(t, band));
            if aa && ab {
                lo += ea.min(eb);
            }
            if aa || ab {
                up += ea.max(eb);
            }
        }
        out[band] = if up > 0.0 { lo / up } else { 0.0 };
    }
    out
}

// ---------------------------------------------------------------------------------------------
// True doubles (ANA-18)
// ---------------------------------------------------------------------------------------------

/// How much two signals look like two performances of the same part (ANA-18).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Likeness {
    /// Frames where both play over frames where either does.
    pub overlap: f64,
    /// Correlation of the frame levels (dB) where both play.
    pub envelope_corr: f64,
    /// Mean absolute difference of the band profiles (dB relative to each signal's total) where both play.
    pub profile_diff_db: f64,
    /// Mean absolute difference of the band shape frame by frame (dB): two takes differ by several dB, two copies
    /// of one signal (also at different levels, or one a few milliseconds late) by a fraction of one.
    pub frame_diff_db: f64,
}

/// A double needs both to play together, with levels that move together and a similar colour.
pub const DOUBLE_MIN_OVERLAP: f64 = 0.7;
pub const DOUBLE_MIN_ENVELOPE_CORR: f64 = 0.6;
pub const DOUBLE_MAX_PROFILE_DIFF_DB: f64 = 3.0;
/// Below this frame-by-frame difference of the band shape the two are the same signal. The levels are not asked to
/// move as one: a copy a few milliseconds late has the same colour in every frame but a shifted envelope.
pub const IDENTICAL_MAX_FRAME_DIFF_DB: f64 = 0.5;

impl Likeness {
    /// Whether the pair counts as a double, and its score (0–1).
    pub fn double(&self) -> Option<f64> {
        (self.overlap >= DOUBLE_MIN_OVERLAP
            && self.envelope_corr >= DOUBLE_MIN_ENVELOPE_CORR
            && self.profile_diff_db <= DOUBLE_MAX_PROFILE_DIFF_DB)
            .then(|| (self.overlap + self.envelope_corr) / 2.0)
    }

    /// Two copies of the same signal rather than two performances: opening them to the sides does nothing.
    pub fn identical(&self) -> bool {
        self.frame_diff_db <= IDENTICAL_MAX_FRAME_DIFF_DB
    }
}

/// Compares two signals on the shared frame grid; `None` when they never play together for at least 2 s.
pub fn likeness(a: &Spectrum, b: &Spectrum) -> Option<Likeness> {
    let n = a.band_power.len().min(b.band_power.len());
    let total = |s: &Spectrum, t: usize| s.band_power[t].iter().sum::<f64>();
    let (mut both, mut either) = (Vec::new(), 0usize);
    for t in 0..n {
        let (x, y) =
            (total(a, t) > spectral::ACTIVITY_THRESHOLD_POWER, total(b, t) > spectral::ACTIVITY_THRESHOLD_POWER);
        if x || y {
            either += 1;
        }
        if x && y {
            both.push(t);
        }
    }
    let min_frames = (2.0 * f64::from(a.sample_rate) / a.hop.max(1) as f64) as usize;
    if both.len() < min_frames.max(8) {
        return None;
    }
    let m = both.len() as f64;
    let la: Vec<f64> = both.iter().map(|&t| db_power(total(a, t))).collect();
    let lb: Vec<f64> = both.iter().map(|&t| db_power(total(b, t))).collect();
    let (ma, mb) = (la.iter().sum::<f64>() / m, lb.iter().sum::<f64>() / m);
    let (mut saa, mut sbb, mut sab) = (0.0, 0.0, 0.0);
    for (x, y) in la.iter().zip(&lb) {
        saa += (x - ma) * (x - ma);
        sbb += (y - mb) * (y - mb);
        sab += (x - ma) * (y - mb);
    }
    // Two steady signals have no envelope to compare: they move together trivially.
    let envelope_corr = if saa < 1e-9 || sbb < 1e-9 { 1.0 } else { sab / (saa * sbb).sqrt() };
    // Band profile over the shared frames, and the band shape frame by frame; only bands that carry something.
    let (mut pa, mut pb) = ([0.0f64; N_BANDS], [0.0f64; N_BANDS]);
    let (mut frame_diff, mut frame_n) = (0.0, 0usize);
    for &t in &both {
        let (ta, tb) = (total(a, t), total(b, t));
        for k in 0..N_BANDS {
            pa[k] += a.band_power[t][k];
            pb[k] += b.band_power[t][k];
            let (ra, rb) = (a.band_power[t][k] / ta, b.band_power[t][k] / tb);
            if ra > 0.01 || rb > 0.01 {
                frame_diff += (db_power(ra.max(1e-4)) - db_power(rb.max(1e-4))).abs();
                frame_n += 1;
            }
        }
    }
    let (sa, sb) = (pa.iter().sum::<f64>(), pb.iter().sum::<f64>());
    let (mut profile, mut bands) = (0.0, 0usize);
    for k in 0..N_BANDS {
        let (ra, rb) = (pa[k] / sa, pb[k] / sb);
        if ra > 0.01 || rb > 0.01 {
            profile += (db_power(ra.max(1e-4)) - db_power(rb.max(1e-4))).abs();
            bands += 1;
        }
    }
    Some(Likeness {
        overlap: both.len() as f64 / either.max(1) as f64,
        envelope_corr,
        profile_diff_db: profile / bands.max(1) as f64,
        frame_diff_db: frame_diff / frame_n.max(1) as f64,
    })
}

/// A pair of signals that are a double (indices into the input), strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Double {
    pub a: usize,
    pub b: usize,
    pub score: f64,
    pub identical: bool,
}

/// The doubles among signals: `candidate(i, j)` says which pairs to compare (same role). Every signal is in at
/// most one pair, the strongest it has.
pub fn doubles(specs: &[&Spectrum], candidate: &dyn Fn(usize, usize) -> bool) -> Vec<Double> {
    let mut all = Vec::new();
    for i in 0..specs.len() {
        for j in i + 1..specs.len() {
            if candidate(i, j)
                && let Some(l) = likeness(specs[i], specs[j])
                && let Some(score) = l.double()
            {
                all.push(Double { a: i, b: j, score, identical: l.identical() });
            }
        }
    }
    all.sort_by(|x, y| y.score.total_cmp(&x.score));
    let mut used = vec![false; specs.len()];
    all.retain(|d| {
        let free = !used[d.a] && !used[d.b];
        if free {
            used[d.a] = true;
            used[d.b] = true;
        }
        free
    });
    all
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collision {
    pub a: usize,
    pub b: usize,
    pub band: usize,
    pub score: f64,
}

/// All collisions above 0 between signals (pairs in order), strongest first, ties stable.
pub fn collisions(specs: &[&Spectrum], skip_pair: &dyn Fn(usize, usize) -> bool) -> Vec<Collision> {
    let mut out = Vec::new();
    for i in 0..specs.len() {
        for j in i + 1..specs.len() {
            if skip_pair(i, j) {
                continue;
            }
            for (band, s) in pair_scores(specs[i], specs[j]).into_iter().enumerate() {
                if s > 0.0 {
                    out.push(Collision { a: i, b: j, band, score: s });
                }
            }
        }
    }
    out.sort_by(|x, y| y.score.total_cmp(&x.score));
    out
}

/// ANA-8: each signal's share of the summed bus energy per band, in %.
pub fn band_share_pct(specs: &[&Spectrum]) -> Vec<[f64; N_BANDS]> {
    let energy: Vec<[f64; N_BANDS]> = specs
        .iter()
        .map(|s| {
            let mut e = [0.0; N_BANDS];
            for t in 0..s.n_frames() {
                for b in 0..N_BANDS {
                    e[b] += s.bus_band_power(t, b);
                }
            }
            e
        })
        .collect();
    let mut total = [0.0; N_BANDS];
    for e in &energy {
        for b in 0..N_BANDS {
            total[b] += e[b];
        }
    }
    energy.iter().map(|e| std::array::from_fn(|b| if total[b] > 0.0 { 100.0 * e[b] / total[b] } else { 0.0 })).collect()
}

// ---------------------------------------------------------------------------------------------
// References and target (ANA-11, ANA-12)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub n_references: usize,
    pub bands_db: Option<[f64; N_BANDS]>,
    pub side_mid_db: Option<f64>,
    pub dyn_range_lu: Option<f64>,
}

/// Normalises a reference to −23 LUFS (`reference v1`); `None` when silent or shorter than 400 ms.
pub fn normalize_reference(samples: &[f32], channels: usize, rate: u32) -> Option<Vec<f32>> {
    let l = loudness::integrated(samples, channels, rate)?;
    if !l.is_finite() {
        return None;
    }
    let g = 10f64.powf((-23.0 - l) / 20.0);
    Some(samples.iter().map(|&v| (f64::from(v) * g) as f32).collect())
}

pub fn measure_reference(samples: &[f32], channels: usize, rate: u32) -> Option<MixMeasures> {
    let n = normalize_reference(samples, channels, rate)?;
    Some(measure_mix(&n, channels, rate).0)
}

fn mean(v: &[f64]) -> Option<f64> {
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

/// ANA-12: field-wise mean of the references (only finite values count).
pub fn target_profile(refs: &[MixMeasures]) -> Option<Target> {
    if refs.is_empty() {
        return None;
    }
    let with_bands: Vec<&[f64; N_BANDS]> = refs.iter().filter_map(|r| r.bands_db.as_ref()).collect();
    let bands = (!with_bands.is_empty())
        .then(|| std::array::from_fn(|b| with_bands.iter().map(|x| x[b]).sum::<f64>() / with_bands.len() as f64));
    let sm: Vec<f64> =
        refs.iter().filter_map(|r| r.stereo.and_then(|s| s.side_mid_db)).filter(|v| v.is_finite()).collect();
    let dr: Vec<f64> = refs.iter().filter_map(|r| r.dyn_range_lu).filter(|v| v.is_finite()).collect();
    Some(Target { n_references: refs.len(), bands_db: bands, side_mid_db: mean(&sm), dyn_range_lu: mean(&dr) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use stemwise_dsp::testsignal::{fixture, num, render};

    fn close(got: Option<f64>, want: &Value, tol: f64) -> bool {
        match (got, num(want)) {
            (None, None) => true,
            (Some(g), Some(w)) if g.is_infinite() || w.is_infinite() => {
                g == w || (g.abs() > 119.0 && w.abs() > 119.0 && g.signum() == w.signum())
            }
            (Some(g), Some(w)) => (g - w).abs() <= tol,
            _ => false,
        }
    }

    fn check_bands(id: &Value, got: Option<[f64; 8]>, want: &Value) {
        match (got, want.as_array()) {
            (None, None) => {}
            (Some(g), Some(w)) => {
                for b in 0..8 {
                    let wv = num(&w[b]).unwrap();
                    // ±0.2 dB on band energies (FOUNDATION §5.7); −∞ bands are compared as such.
                    if wv.is_infinite() || g[b] < -200.0 {
                        assert!(g[b] < -100.0 && wv < -100.0, "{id} band {b}: {} vs {wv}", g[b]);
                    } else {
                        assert!((g[b] - wv).abs() <= 0.2, "{id} band {b}: {} vs {wv}", g[b]);
                    }
                }
            }
            _ => panic!("{id}: bands {got:?} vs {want}"),
        }
    }

    /// req: ANA-1, ANA-2, ANA-3, ANA-4, ANA-5, ANA-6
    #[test]
    fn stem_measures_match_the_poc_oracle() {
        // Tolerances of FOUNDATION §5.7: levels ±0.1 dB / LU, bands ±0.2 dB, active share ±1 point, centroid ±2%,
        // correlations ±0.02. Onsets are checked in `onsets.rs`.
        for c in fixture("stem_measures.json")["cases"].as_array().unwrap() {
            let id = &c["id"];
            let fs = c["fs"].as_u64().unwrap() as u32;
            let (x, ch) = render(&c["signal"], fs);
            let (m, _) = measure_stem(&x, ch, fs);
            let e = &c["expected"];
            assert_eq!(m.channels as u64, e["channels"].as_u64().unwrap());
            assert!(close(m.lufs, &e["lufs"], 0.1), "{id} lufs {:?} vs {}", m.lufs, e["lufs"]);
            assert!(close(Some(m.peak_db), &e["peak_db"], 0.1), "{id} peak");
            assert!(close(m.rms_db, &e["rms_db"], 0.1), "{id} rms {:?} vs {}", m.rms_db, e["rms_db"]);
            assert!(close(m.crest_db, &e["crest_db"], 0.1), "{id} crest");
            assert!(
                close(Some(m.active_pct), &e["active_pct"], 1.0),
                "{id} active {} vs {}",
                m.active_pct,
                e["active_pct"]
            );
            check_bands(id, m.bands_db, &e["bands_db"]);
            match (m.centroid_hz, num(&e["centroid_hz"])) {
                (Some(g), Some(w)) => assert!((g - w).abs() <= 0.02 * w, "{id} centroid {g} vs {w}"),
                (g, w) => assert_eq!(g.is_some(), w.is_some(), "{id} centroid"),
            }
            match (&m.stereo, e["stereo"].as_object()) {
                (Some(s), Some(w)) => {
                    assert!(
                        close(s.correlation, &w["correlation"], 0.02),
                        "{id} corr {:?} vs {}",
                        s.correlation,
                        w["correlation"]
                    );
                    assert!(
                        close(s.side_mid_db, &w["side_mid_db"], 0.1),
                        "{id} side/mid {:?} vs {}",
                        s.side_mid_db,
                        w["side_mid_db"]
                    );
                }
                (None, None) => {}
                _ => panic!("{id} stereo"),
            }
            assert!(close(Some(m.hygiene.dc_offset_pct), &e["dc_offset_pct"], 0.001), "{id} dc");
            assert_eq!(m.hygiene.clipped_samples as u64, e["clipped_samples"].as_u64().unwrap(), "{id} clipped");
            assert!(
                close(m.hygiene.noise_floor_db, &e["noise_floor_db"], 0.1),
                "{id} noise {:?} vs {}",
                m.hygiene.noise_floor_db,
                e["noise_floor_db"]
            );
        }
    }

    /// req: ANA-7, ANA-10
    #[test]
    fn mix_measures_match_the_poc_oracle() {
        for c in fixture("mix_measures.json")["cases"].as_array().unwrap() {
            let id = &c["id"];
            let fs = c["fs"].as_u64().unwrap() as u32;
            let (x, ch) = render(&c["signal"], fs);
            let (m, _) = measure_mix(&x, ch, fs);
            let e = &c["expected"];
            assert!(close(m.lufs, &e["lufs"], 0.1), "{id} lufs");
            assert!(close(Some(m.true_peak_db), &e["true_peak_db"], 0.1), "{id} tp");
            assert!(close(m.dyn_range_lu, &e["dyn_range_lu"], 0.1), "{id} lra");
            check_bands(id, m.bands_db, &e["bands_db"]);
            assert!(close(m.low_corr, &e["low_corr"], 0.02), "{id} low corr {:?} vs {}", m.low_corr, e["low_corr"]);
        }
    }

    /// req: ANA-8, ANA-9
    #[test]
    fn masking_and_band_shares_match_the_poc_oracle() {
        for c in fixture("masking.json")["cases"].as_array().unwrap() {
            let id = &c["id"];
            let fs = c["fs"].as_u64().unwrap() as u32;
            let (xa, ca) = render(&c["signals"]["a"], fs);
            let (xb, cb) = render(&c["signals"]["b"], fs);
            let (sa, sb) = (spectral::analyze(&xa, ca, fs), spectral::analyze(&xb, cb, fs));
            let scores = pair_scores(&sa, &sb);
            for (b, w) in c["expected"]["pair_scores"].as_array().unwrap().iter().enumerate() {
                assert!((scores[b] - w.as_f64().unwrap()).abs() <= 0.02, "{id} band {b}: {} vs {w}", scores[b]);
            }
            let shares = band_share_pct(&[&sa, &sb]);
            for (k, key) in ["a", "b"].iter().enumerate() {
                for (b, w) in c["expected"]["band_share_pct"][key].as_array().unwrap().iter().enumerate() {
                    assert!((shares[k][b] - w.as_f64().unwrap()).abs() <= 1.0, "{id} share {key} {b}");
                }
            }
        }
    }

    /// req: ANA-11, ANA-12
    #[test]
    fn references_and_target_match_the_poc_oracle() {
        let c = &fixture("reference.json")["cases"][0];
        let fs = c["fs"].as_u64().unwrap() as u32;
        let refs: Vec<MixMeasures> = c["signals"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                let (x, ch) = render(s, fs);
                measure_reference(&x, ch, fs).unwrap()
            })
            .collect();
        for (r, w) in refs.iter().zip(c["expected"]["references"].as_array().unwrap()) {
            assert!(close(r.lufs, &w["lufs"], 0.1));
            check_bands(&c["id"], r.bands_db, &w["bands_db"]);
        }
        let t = target_profile(&refs).unwrap();
        let w = &c["expected"]["target"];
        assert_eq!(t.n_references as u64, w["n_references"].as_u64().unwrap());
        check_bands(&c["id"], t.bands_db, &w["bands_db"]);
        assert!(close(t.side_mid_db, &w["side_mid_db"], 0.1), "{:?} vs {}", t.side_mid_db, w["side_mid_db"]);
        assert!(close(t.dyn_range_lu, &w["dyn_range_lu"], 0.1));
    }

    /// A strummed part: bursts of noise through a resonance, on a rhythm pattern.
    fn take(seed: u32, pattern: u32, res_hz: f64, amp: f64, secs: f64) -> Spectrum {
        spectral::analyze(&take_samples(seed, pattern, res_hz, amp, secs), 1, 48000)
    }

    fn take_samples(seed: u32, pattern: u32, res_hz: f64, amp: f64, secs: f64) -> Vec<f32> {
        let rate = 48000u32;
        let n = (secs * f64::from(rate)) as usize;
        let noise = stemwise_dsp::testsignal::xorshift32(seed, n);
        let mut bp = stemwise_dsp::biquad::Biquad::new(stemwise_dsp::biquad::Coeffs::design(
            stemwise_dsp::biquad::Kind::Peak,
            res_hz,
            18.0,
            1.5,
            f64::from(rate),
        ));
        let step = rate as usize / 4;
        let x: Vec<f32> = (0..n)
            .map(|i| {
                // 16 steps of a quarter second; a hit decays over its step.
                let on = (pattern >> ((i / step) % 16)) & 1 == 1;
                let env = if on { (-(4.0 * (i % step) as f64 / step as f64)).exp() } else { 0.0 };
                (bp.tick(0, noise[i]) * 0.05 * amp * env) as f32
            })
            .collect();
        x
    }

    /// req: ANA-18
    #[test]
    fn two_takes_of_one_part_are_a_double_and_two_copies_are_identical() {
        // Thresholds: the same rhythm played twice shares its active frames (overlap ≥ 0.7) and its level moves
        // together (correlation ≥ 0.6); a different instrument colour or a different rhythm does not qualify.
        let rhythm = 0b1011_0110_1101_1011;
        let (a, b) = (take(1, rhythm, 800.0, 1.0, 12.0), take(2, rhythm, 900.0, 0.8, 12.0));
        let l = likeness(&a, &b).unwrap();
        assert!(l.double().is_some(), "{l:?}");
        // These two synthetic takes share their envelope exactly and differ only in the noise and in the resonance:
        // 0.8 dB frame by frame, against 0.13 dB for a copy 12 ms late (below). Real takes differ by more.
        assert!(!l.identical(), "two takes differ frame by frame: {l:?}");
        // The same take at another level is the same signal.
        let quiet = take(1, rhythm, 800.0, 0.5, 12.0);
        let l = likeness(&a, &quiet).unwrap();
        assert!(l.double().is_some() && l.identical(), "{l:?}");
        // The same take 12 ms late and quieter (a copy nudged to fake a double) is still the same signal.
        let mut late = vec![0f32; 576];
        late.extend(take_samples(1, rhythm, 800.0, 0.7, 12.0));
        let l = likeness(&a, &spectral::analyze(&late, 1, 48000)).unwrap();
        assert!(l.double().is_some() && l.identical(), "{l:?}");
        // Another rhythm: they do not play together enough.
        let other = take(3, 0b0100_1001_0010_0100, 800.0, 1.0, 12.0);
        let l = likeness(&a, &other).unwrap_or(Likeness {
            overlap: 0.0,
            envelope_corr: 0.0,
            profile_diff_db: 99.0,
            frame_diff_db: 99.0,
        });
        assert!(l.double().is_none(), "{l:?}");
        // Same rhythm, another colour (a bright part against a dark one).
        let bright = take(4, rhythm, 6000.0, 1.0, 12.0);
        let l = likeness(&a, &bright).unwrap();
        assert!(l.double().is_none(), "{l:?}");
        // Pairing: every signal in one pair at most, only among candidates.
        let specs = [&a, &b, &quiet, &other];
        let d = doubles(&specs, &|_, _| true);
        // `a` could pair with `b` or with its copy: it gets one of them, the stronger, and the other stays alone.
        assert!(d.len() == 1 && d[0].a == 0, "{d:?}");
        let d = doubles(&specs, &|i, j| (i, j) == (0, 1));
        assert!(d.len() == 1 && (d[0].a, d[0].b) == (0, 1) && !d[0].identical);
    }
}
