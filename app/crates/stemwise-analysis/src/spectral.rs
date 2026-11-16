//! The analysis STFT (poc/docs/analysis.md, "STFT di analisi e parti attive"): one transform per signal, reduced on
//! the fly to frame powers, band powers, the active-frame spectrum and stereo sums. Port of the PoC's
//! `stemwise.analysis.spectral` (`spectral v1`, `bands v1`, `centroid v1`).

use realfft::RealFftPlanner;

pub const ACTIVITY_THRESHOLD_POWER: f64 = 1e-5; // −50 dBFS
pub const BANDS_HZ: [(f64, f64); 8] = [
    (20.0, 60.0),
    (60.0, 120.0),
    (120.0, 250.0),
    (250.0, 500.0),
    (500.0, 2000.0),
    (2000.0, 4000.0),
    (4000.0, 8000.0),
    (8000.0, 20000.0),
];
pub const BAND_LABELS: [&str; 8] = ["20-60", "60-120", "120-250", "250-500", "500-2k", "2k-4k", "4k-8k", "8k-20k"];
pub const N_BANDS: usize = 8;

/// Sums over active frames and a frequency range of L, R, mid, side bin powers and Re(L·R*).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StereoSums {
    pub ll: f64,
    pub rr: f64,
    pub lr: f64,
    pub mm: f64,
    pub ss: f64,
}

/// Reduction of one signal's analysis STFT.
#[derive(Debug, Clone)]
pub struct Spectrum {
    pub sample_rate: u32,
    pub n_samples: usize,
    pub channels: usize,
    pub n_fft: usize,
    pub hop: usize,
    /// Mean over channels of the power in 20 Hz–20 kHz, per frame.
    pub frame_power: Vec<f64>,
    /// Mean over channels of the power per band, per frame.
    pub band_power: Vec<[f64; N_BANDS]>,
    pub active: Vec<bool>,
    /// Mean bin power over the active frames.
    pub active_spectrum: Vec<f64>,
    pub stereo_full: Option<StereoSums>,
    pub stereo_low: Option<StereoSums>,
}

pub fn fft_size(rate: u32) -> usize {
    let e = (f64::from(rate) / 48000.0).log2().round_ties_even() as i32;
    (8192.0 * 2f64.powi(e)) as usize
}

pub fn hop_size(rate: u32) -> usize {
    fft_size(rate) / 4
}

pub fn frame_count(n: usize, n_fft: usize, hop: usize) -> usize {
    if n == 0 { 0 } else { 1 + n.saturating_sub(n_fft).div_ceil(hop) }
}

pub fn bin_freq(k: usize, n_fft: usize, rate: u32) -> f64 {
    k as f64 * f64::from(rate) / n_fft as f64
}

/// Band of each bin (−1 outside 20 Hz–20 kHz).
pub fn band_index(n_fft: usize, rate: u32) -> Vec<i32> {
    let top = BANDS_HZ[7].1.min(f64::from(rate) / 2.0);
    (0..=n_fft / 2)
        .map(|k| {
            let f = bin_freq(k, n_fft, rate);
            for (b, &(lo, hi)) in BANDS_HZ.iter().enumerate() {
                let hi = hi.min(top);
                let inside = f >= lo && if b == N_BANDS - 1 { f <= hi } else { f < hi };
                if inside {
                    return b as i32;
                }
            }
            -1
        })
        .collect()
}

impl Spectrum {
    pub fn n_frames(&self) -> usize {
        self.frame_power.len()
    }

    pub fn n_active(&self) -> usize {
        self.active.iter().filter(|a| **a).count()
    }

    pub fn duration_s(&self) -> f64 {
        self.n_samples as f64 / f64::from(self.sample_rate)
    }

    /// Band power the signal brings to the stereo bus (mono x², stereo L² + R²).
    pub fn bus_band_power(&self, t: usize, b: usize) -> f64 {
        self.band_power[t][b] * self.channels as f64
    }
}

/// Computes the analysis STFT of interleaved audio.
pub fn analyze(samples: &[f32], channels: usize, rate: u32) -> Spectrum {
    let n = samples.len() / channels;
    let n_fft = fft_size(rate);
    let hop = hop_size(rate);
    let frames = frame_count(n, n_fft, hop);
    let bins = n_fft / 2 + 1;
    let bands = band_index(n_fft, rate);
    let full: Vec<bool> = (0..bins).map(|k| (20.0..=20000.0).contains(&bin_freq(k, n_fft, rate))).collect();
    let low: Vec<bool> = (0..bins).map(|k| (20.0..120.0).contains(&bin_freq(k, n_fft, rate))).collect();
    let window: Vec<f64> =
        (0..n_fft).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n_fft as f64).cos()).collect();
    let wsum2: f64 = window.iter().map(|w| w * w).sum();
    let scale: Vec<f64> =
        (0..bins).map(|k| if k == 0 || k == n_fft / 2 { 1.0 } else { 2.0 } / (n_fft as f64 * wsum2)).collect();

    let mut planner = RealFftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(n_fft);
    let mut input = fft.make_input_vec();
    let mut scratch = fft.make_scratch_vec();
    let mut spectra: Vec<Vec<realfft::num_complex::Complex<f64>>> =
        (0..channels).map(|_| fft.make_output_vec()).collect();

    let mut frame_power = vec![0.0; frames];
    let mut band_power = vec![[0.0; N_BANDS]; frames];
    let mut spectrum_sum = vec![0.0; bins];
    let mut active = vec![false; frames];
    let stereo = channels == 2;
    let (mut sf, mut sl) = (StereoSums::default(), StereoSums::default());
    let mut mean = vec![0.0; bins];
    for t in 0..frames {
        for (c, out) in spectra.iter_mut().enumerate() {
            for i in 0..n_fft {
                let idx = t * hop + i;
                input[i] = if idx < n { f64::from(samples[idx * channels + c]) * window[i] } else { 0.0 };
            }
            fft.process_with_scratch(&mut input, out, &mut scratch).expect("fft sizes");
        }
        mean.fill(0.0);
        for out in &spectra {
            for k in 0..bins {
                mean[k] += out[k].norm_sqr() * scale[k];
            }
        }
        for v in &mut mean {
            *v /= channels as f64;
        }
        let mut fp = 0.0;
        for k in 0..bins {
            if full[k] {
                fp += mean[k];
            }
            if bands[k] >= 0 {
                band_power[t][bands[k] as usize] += mean[k];
            }
        }
        frame_power[t] = fp;
        if fp > ACTIVITY_THRESHOLD_POWER {
            active[t] = true;
            for k in 0..bins {
                spectrum_sum[k] += mean[k];
            }
            if stereo {
                let (l, r) = (&spectra[0], &spectra[1]);
                for k in 0..bins {
                    let (a, b) = (l[k], r[k]);
                    let m = (a + b) * 0.5;
                    let s = (a - b) * 0.5;
                    let terms = [a.norm_sqr(), b.norm_sqr(), (a * b.conj()).re, m.norm_sqr(), s.norm_sqr()];
                    for (on, acc) in [(full[k], &mut sf), (low[k], &mut sl)] {
                        if on {
                            acc.ll += terms[0] * scale[k];
                            acc.rr += terms[1] * scale[k];
                            acc.lr += terms[2] * scale[k];
                            acc.mm += terms[3] * scale[k];
                            acc.ss += terms[4] * scale[k];
                        }
                    }
                }
            }
        }
    }
    let n_active = active.iter().filter(|a| **a).count();
    if n_active > 0 {
        for v in &mut spectrum_sum {
            *v /= n_active as f64;
        }
    }
    Spectrum {
        sample_rate: rate,
        n_samples: n,
        channels,
        n_fft,
        hop,
        frame_power,
        band_power,
        active,
        active_spectrum: spectrum_sum,
        stereo_full: stereo.then_some(sf),
        stereo_low: stereo.then_some(sl),
    }
}

/// ANA-4: percentage of active frames.
pub fn active_pct(s: &Spectrum) -> f64 {
    if s.n_frames() == 0 { 0.0 } else { 100.0 * s.n_active() as f64 / s.n_frames() as f64 }
}

pub fn active_seconds(s: &Spectrum) -> f64 {
    active_pct(s) / 100.0 * s.duration_s()
}

/// ANA-3: energy of the 8 bands over the active frames, in dB relative to their sum.
pub fn band_levels_db(s: &Spectrum) -> Option<[f64; N_BANDS]> {
    if s.n_active() == 0 {
        return None;
    }
    let mut e = [0.0; N_BANDS];
    for (t, bp) in s.band_power.iter().enumerate() {
        if s.active[t] {
            for b in 0..N_BANDS {
                e[b] += bp[b];
            }
        }
    }
    let total: f64 = e.iter().sum();
    if total <= 0.0 {
        return None;
    }
    Some(e.map(|v| 10.0 * (v / total).log10()))
}

/// ANA-4: power-weighted mean frequency of the active-frame spectrum, 20 Hz–20 kHz.
pub fn centroid_hz(s: &Spectrum) -> Option<f64> {
    if s.n_active() == 0 {
        return None;
    }
    let (mut num, mut den) = (0.0, 0.0);
    for (k, p) in s.active_spectrum.iter().enumerate() {
        let f = bin_freq(k, s.n_fft, s.sample_rate);
        if (20.0..=20000.0).contains(&f) {
            num += f * p;
            den += p;
        }
    }
    (den > 0.0).then(|| num / den)
}
