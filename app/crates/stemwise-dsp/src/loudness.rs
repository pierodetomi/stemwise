//! Loudness (ITU-R BS.1770-4, EBU Tech 3342): offline measures identical to the PoC (`pyloudnorm` 0.2,
//! poc/docs/analysis.md "Loudness") and a real-time meter for the master (MAS-4).

use crate::biquad::{Biquad, Coeffs, Kind};

/// K-weighting as in `pyloudnorm`: RBJ high shelf +4 dB, Q 1/√2, 1500 Hz, then high-pass Q 0.5, 38 Hz.
pub fn k_weighting(rate: f64) -> [Coeffs; 2] {
    [
        Coeffs::design(Kind::HighShelf, 1500.0, 4.0, std::f64::consts::FRAC_1_SQRT_2, rate),
        Coeffs::design(Kind::HighPass, 38.0, 0.0, 0.5, rate),
    ]
}

/// The blocks of `pyloudnorm.Meter`: `[l, u)` sample ranges of `block` seconds every `block · (1 − overlap)`.
fn block_bounds(n: usize, rate: f64, block: f64, overlap: f64) -> Vec<(usize, usize)> {
    let step = 1.0 - overlap;
    let t = n as f64 / rate;
    let blocks = (((t - block) / (block * step)).round_ties_even() as i64 + 1).max(0) as usize;
    (0..blocks)
        .map(|j| {
            let l = (block * (j as f64 * step) * rate) as usize;
            let u = ((block * (j as f64 * step + 1.0) * rate) as usize).min(n);
            (l, u)
        })
        .collect()
}

/// Mean square of the K-weighted signal in each block, per channel, without copying the signal (NFR-5): the
/// filters run sample by sample over the interleaved input followed by `pad` frames of silence, the squares are
/// summed per segment between consecutive block edges, and each block adds up its segments. Same blocks and values
/// as `pyloudnorm` (summed in another grouping: equal to ~1e-12 relative).
fn block_means(
    samples: &[f32],
    channels: usize,
    pad: usize,
    rate: f64,
    block: f64,
    overlap: f64,
) -> (Vec<f64>, Vec<Vec<f64>>) {
    let frames = samples.len() / channels;
    let n = frames + pad;
    let bounds = block_bounds(n, rate, block, overlap);
    let mut edges: Vec<usize> = bounds.iter().flat_map(|&(l, u)| [l, u]).chain([0, n]).collect();
    edges.sort_unstable();
    edges.dedup();
    let [shelf, hp] = k_weighting(rate);
    let mut z = vec![vec![0.0; bounds.len()]; channels];
    for (c, zc) in z.iter_mut().enumerate() {
        let (mut f1, mut f2) = (Biquad::new(shelf), Biquad::new(hp));
        let mut seg = vec![0.0f64; edges.len().saturating_sub(1)];
        let mut k = 0;
        for i in 0..n {
            while k + 1 < edges.len() - 1 && i >= edges[k + 1] {
                k += 1;
            }
            let x = if i < frames { f64::from(samples[i * channels + c]) } else { 0.0 };
            let y = f2.tick(0, f1.tick(0, x));
            seg[k] += y * y;
        }
        for (zj, &(l, u)) in zc.iter_mut().zip(&bounds) {
            let (a, b) = (edges.partition_point(|&e| e < l), edges.partition_point(|&e| e < u));
            let sum: f64 = if l < u { seg[a..b].iter().sum() } else { 0.0 };
            *zj = (1.0 / (block * rate)) * sum;
        }
    }
    let l = (0..bounds.len()).map(|j| -0.691 + 10.0 * (0..channels).map(|i| z[i][j]).sum::<f64>().log10()).collect();
    (l, z)
}

/// Integrated loudness in LUFS (gates −70 LUFS and −10 LU). `None` when shorter than one 400 ms block;
/// −∞ when everything is gated out.
pub fn integrated(samples: &[f32], channels: usize, rate: u32) -> Option<f64> {
    let rate = f64::from(rate);
    let frames = samples.len() / channels;
    if (frames as f64) < 0.4 * rate {
        return None;
    }
    let (l, z) = block_means(samples, channels, 0, rate, 0.4, 0.75);
    Some(gate(&l, &z))
}

fn gate(l: &[f64], z: &[Vec<f64>]) -> f64 {
    let mean = |idx: &[usize], zi: &Vec<f64>| -> f64 {
        if idx.is_empty() { f64::NAN } else { idx.iter().map(|&j| zi[j]).sum::<f64>() / idx.len() as f64 }
    };
    let abs: Vec<usize> = (0..l.len()).filter(|&j| l[j] >= -70.0).collect();
    let gamma_r = -0.691 + 10.0 * z.iter().map(|zi| mean(&abs, zi)).sum::<f64>().log10() - 10.0;
    let rel: Vec<usize> = (0..l.len()).filter(|&j| l[j] > gamma_r && l[j] > -70.0).collect();
    let total: f64 = z.iter().map(|zi| mean(&rel, zi)).map(|v| if v.is_nan() { 0.0 } else { v }).sum();
    -0.691 + 10.0 * total.log10()
}

/// numpy's default percentile (linear interpolation).
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    let pos = (sorted.len() - 1) as f64 * p / 100.0;
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let frac = pos - lo as f64;
    sorted[lo] + (sorted[hi] - sorted[lo]) * frac
}

/// Loudness range in LU (EBU Tech 3342 as in `pyloudnorm`: 3 s windows, 97% overlap, 1.5 s of silence appended,
/// gates −70 LUFS and −20 LU, P95 − P10). `None` when too short or entirely gated out.
pub fn range(samples: &[f32], channels: usize, rate: u32) -> Option<f64> {
    let ratef = f64::from(rate);
    let pad = (1.5 * ratef) as usize;
    let frames = samples.len() / channels + pad;
    if (frames as f64) < 3.0 * ratef {
        return None;
    }
    // The silence is appended before filtering, as in pyloudnorm: the filters run on over the zeros.
    let (l, _) = block_means(samples, channels, pad, ratef, 3.0, 0.97);
    let abs: Vec<f64> = l.iter().copied().filter(|&x| x >= -70.0).collect();
    if abs.is_empty() {
        return None;
    }
    let power = abs.iter().map(|x| 10f64.powf(x / 10.0)).sum::<f64>() / abs.len() as f64;
    let int = 10.0 * power.log10();
    let mut rel: Vec<f64> = abs.into_iter().filter(|&x| x >= int - 20.0).collect();
    if rel.is_empty() {
        return None;
    }
    rel.sort_by(f64::total_cmp);
    let v = percentile(&rel, 95.0) - percentile(&rel, 10.0);
    v.is_finite().then_some(v)
}

// ---------------------------------------------------------------------------------------------
// Real-time meter (MAS-4): momentary, short-term, integrated and LRA, allocation-free after `new`.
// ---------------------------------------------------------------------------------------------

const HIST_MIN: f64 = -70.0;
const HIST_STEP: f64 = 0.1;
const HIST_BINS: usize = 800; // −70 … +10 LUFS

/// Streaming BS.1770 meter with 100 ms steps; gating with energy histograms (no allocation in `process`).
pub struct LoudnessMeter {
    rate: f64,
    filters: [[Biquad; 2]; 2],
    step_len: usize,
    acc: f64,
    acc_n: usize,
    /// Mean square of the last 30 steps of 100 ms (ring).
    steps: [f64; 30],
    steps_filled: usize,
    head: usize,
    int_hist: [(u32, f64); HIST_BINS],
    st_hist: [u32; HIST_BINS],
    pub momentary: f64,
    pub short_term: f64,
}

impl LoudnessMeter {
    pub fn new(rate: f64) -> Self {
        let [a, b] = k_weighting(rate);
        LoudnessMeter {
            rate,
            filters: [[Biquad::new(a), Biquad::new(b)], [Biquad::new(a), Biquad::new(b)]],
            step_len: (rate * 0.1).round() as usize,
            acc: 0.0,
            acc_n: 0,
            steps: [0.0; 30],
            steps_filled: 0,
            head: 0,
            int_hist: [(0, 0.0); HIST_BINS],
            st_hist: [0; HIST_BINS],
            momentary: f64::NEG_INFINITY,
            short_term: f64::NEG_INFINITY,
        }
    }

    pub fn reset(&mut self) {
        *self = LoudnessMeter::new(self.rate);
    }

    fn bin(l: f64) -> Option<usize> {
        if l < HIST_MIN || !l.is_finite() {
            return None;
        }
        Some((((l - HIST_MIN) / HIST_STEP) as usize).min(HIST_BINS - 1))
    }

    fn window(&self, n: usize) -> Option<f64> {
        if self.steps_filled < n {
            return None;
        }
        let mut s = 0.0;
        for k in 0..n {
            s += self.steps[(self.head + 30 - 1 - k) % 30];
        }
        Some(s / n as f64)
    }

    /// Feeds stereo samples.
    pub fn process(&mut self, l: &[f32], r: &[f32]) {
        for (a, b) in l.iter().zip(r) {
            let mut e = 0.0;
            for (c, x) in [*a, *b].into_iter().enumerate() {
                let y = self.filters[c][0].tick(0, f64::from(x));
                let y = self.filters[c][1].tick(0, y);
                e += y * y;
            }
            self.acc += e;
            self.acc_n += 1;
            if self.acc_n == self.step_len {
                self.steps[self.head] = self.acc / self.acc_n as f64;
                self.head = (self.head + 1) % 30;
                self.steps_filled = (self.steps_filled + 1).min(30);
                self.acc = 0.0;
                self.acc_n = 0;
                if let Some(m) = self.window(4) {
                    self.momentary = -0.691 + 10.0 * m.log10();
                    if let Some(b) = Self::bin(self.momentary) {
                        self.int_hist[b].0 += 1;
                        self.int_hist[b].1 += m;
                    }
                }
                if let Some(s) = self.window(30) {
                    self.short_term = -0.691 + 10.0 * s.log10();
                    if let Some(b) = Self::bin(self.short_term) {
                        self.st_hist[b] += 1;
                    }
                }
            }
        }
    }

    /// Integrated loudness so far (gated), −∞ before the first block.
    pub fn integrated(&self) -> f64 {
        let (mut n, mut e) = (0u32, 0.0);
        for &(c, s) in &self.int_hist {
            n += c;
            e += s;
        }
        if n == 0 {
            return f64::NEG_INFINITY;
        }
        let rel = -0.691 + 10.0 * (e / f64::from(n)).log10() - 10.0;
        let (mut n2, mut e2) = (0u32, 0.0);
        for (i, &(c, s)) in self.int_hist.iter().enumerate() {
            if HIST_MIN + (i as f64 + 0.5) * HIST_STEP > rel {
                n2 += c;
                e2 += s;
            }
        }
        if n2 == 0 { f64::NEG_INFINITY } else { -0.691 + 10.0 * (e2 / f64::from(n2)).log10() }
    }

    /// Loudness range so far, from the short-term histogram (0.1 LU resolution).
    pub fn range(&self) -> f64 {
        let (mut n, mut p) = (0u32, 0.0);
        for (i, &c) in self.st_hist.iter().enumerate() {
            n += c;
            p += f64::from(c) * 10f64.powf((HIST_MIN + (i as f64 + 0.5) * HIST_STEP) / 10.0);
        }
        if n == 0 {
            return 0.0;
        }
        let gate = 10.0 * (p / f64::from(n)).log10() - 20.0;
        let first = Self::bin(gate.max(HIST_MIN)).unwrap_or(0);
        let total: u32 = self.st_hist[first..].iter().sum();
        if total == 0 {
            return 0.0;
        }
        let at = |q: f64| {
            let target = (f64::from(total) * q).ceil().max(1.0) as u32;
            let mut seen = 0;
            for (i, &c) in self.st_hist.iter().enumerate().skip(first) {
                seen += c;
                if seen >= target {
                    return HIST_MIN + (i as f64 + 0.5) * HIST_STEP;
                }
            }
            HIST_MIN + HIST_BINS as f64 * HIST_STEP
        };
        at(0.95) - at(0.10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The direct computation the PoC's `pyloudnorm` does (whole-signal copies), kept to check the streaming one.
    fn reference_blocks(samples: &[f32], channels: usize, pad: usize, rate: f64, block: f64, overlap: f64) -> Vec<f64> {
        let frames = samples.len() / channels;
        let n = frames + pad;
        let [shelf, hp] = k_weighting(rate);
        let ch: Vec<Vec<f64>> = (0..channels)
            .map(|c| {
                let mut x: Vec<f64> =
                    (0..n).map(|i| if i < frames { f64::from(samples[i * channels + c]) } else { 0.0 }).collect();
                Biquad::new(shelf).filter_f64(0, &mut x);
                Biquad::new(hp).filter_f64(0, &mut x);
                x
            })
            .collect();
        block_bounds(n, rate, block, overlap)
            .iter()
            .map(|&(l, u)| {
                let z: f64 = ch
                    .iter()
                    .map(|x| if l < u { x[l..u].iter().map(|v| v * v).sum::<f64>() } else { 0.0 } / (block * rate))
                    .sum();
                -0.691 + 10.0 * z.log10()
            })
            .collect()
    }

    /// req: ANA-1
    #[test]
    fn streaming_blocks_equal_the_direct_computation() {
        let x: Vec<f32> = crate::testsignal::xorshift32(5, 2 * 48000 * 7)
            .iter()
            .enumerate()
            .map(|(i, v)| (v * if (i / 30000) % 3 == 0 { 0.01 } else { 0.4 }) as f32)
            .collect();
        for (pad, block, overlap) in [(0, 0.4, 0.75), (72000, 3.0, 0.97)] {
            let (got, _) = block_means(&x, 2, pad, 48000.0, block, overlap);
            let want = reference_blocks(&x, 2, pad, 48000.0, block, overlap);
            assert_eq!(got.len(), want.len());
            for (g, w) in got.iter().zip(&want) {
                assert!((g - w).abs() < 1e-9, "{g} vs {w}");
            }
        }
    }
    use crate::testsignal::{fixture, num, render};

    fn close(a: Option<f64>, b: Option<f64>, tol: f64) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(x), Some(y)) if x.is_infinite() || y.is_infinite() => x == y,
            (Some(x), Some(y)) => (x - y).abs() <= tol,
            _ => false,
        }
    }

    /// req: ANA-1, ANA-2, MAS-4
    #[test]
    fn integrated_loudness_and_range_match_the_poc_oracle() {
        // Tolerance ±0.1 LU (FOUNDATION §5.7). Same filters, blocks and gates as pyloudnorm.
        let fx = fixture("stem_measures.json");
        for c in fx["cases"].as_array().unwrap() {
            let fs = c["fs"].as_u64().unwrap() as u32;
            let (x, ch) = render(&c["signal"], fs);
            let e = &c["expected"];
            let lufs = integrated(&x, ch, fs);
            assert!(close(lufs, num(&e["lufs"]), 0.1), "{} lufs {lufs:?} vs {}", c["id"], e["lufs"]);
            let lra = range(&x, ch, fs);
            assert!(close(lra, num(&e["dyn_range_lu"]), 0.1), "{} lra {lra:?} vs {}", c["id"], e["dyn_range_lu"]);
        }
    }

    /// req: MAS-4
    #[test]
    fn realtime_meter_agrees_with_the_offline_measure() {
        let fs = 48000u32;
        let spec = serde_json::json!({"kind": "stereo",
            "left": {"kind": "pink", "seed": 3, "amp": 0.5, "secs": 12.0},
            "right": {"kind": "pink", "seed": 4, "amp": 0.5, "secs": 12.0}});
        let (x, _) = render(&spec, fs);
        let l: Vec<f32> = x.iter().step_by(2).copied().collect();
        let r: Vec<f32> = x.iter().skip(1).step_by(2).copied().collect();
        let mut m = LoudnessMeter::new(f64::from(fs));
        for (a, b) in l.chunks(512).zip(r.chunks(512)) {
            m.process(a, b);
        }
        let offline = integrated(&x, 2, fs).unwrap();
        // Histogram gating with 0.1 LU bins and 100 ms steps: within 0.2 LU of the offline measure.
        assert!((m.integrated() - offline).abs() < 0.2, "{} vs {offline}", m.integrated());
        assert!((m.short_term - offline).abs() < 0.5);
        assert!(m.range() >= 0.0 && m.range() < 3.0);
    }
}
