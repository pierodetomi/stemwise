//! Measures over time (ANA-15): loudness and active time of each track per section, band profile of the mix per
//! section. Sections are `[start, end)` frame ranges on the signals' own frame axis.

use stemwise_dsp::loudness;

use crate::spectral::{self, N_BANDS, Spectrum};

/// Frames of the analysis STFT whose centre falls inside `[start, end)`.
fn frames_in(s: &Spectrum, start: i64, end: i64) -> impl Iterator<Item = usize> + '_ {
    (0..s.n_frames()).filter(move |&t| {
        let centre = (t * s.hop + s.n_fft / 2) as i64;
        centre >= start && centre < end
    })
}

/// Integrated loudness of the samples inside each section (`None` when shorter than 400 ms; −∞ when silent).
pub fn section_lufs(samples: &[f32], channels: usize, rate: u32, sections: &[(i64, i64)]) -> Vec<Option<f64>> {
    let n = (samples.len() / channels) as i64;
    sections
        .iter()
        .map(|&(a, b)| {
            let (a, b) = (a.clamp(0, n) as usize, b.clamp(0, n) as usize);
            if b <= a {
                return None;
            }
            loudness::integrated(&samples[a * channels..b * channels], channels, rate)
        })
        .collect()
}

/// Percentage of active analysis frames inside each section.
pub fn section_active_pct(s: &Spectrum, sections: &[(i64, i64)]) -> Vec<f64> {
    sections
        .iter()
        .map(|&(a, b)| {
            let (mut n, mut act) = (0usize, 0usize);
            for t in frames_in(s, a, b) {
                n += 1;
                act += usize::from(s.active[t]);
            }
            if n == 0 { 0.0 } else { 100.0 * act as f64 / n as f64 }
        })
        .collect()
}

/// Band levels (dB relative to their sum, active frames only) inside each section.
pub fn section_bands(s: &Spectrum, sections: &[(i64, i64)]) -> Vec<Option<[f64; N_BANDS]>> {
    sections
        .iter()
        .map(|&(a, b)| {
            let mut e = [0.0; N_BANDS];
            let mut any = false;
            for t in frames_in(s, a, b) {
                if s.active[t] {
                    any = true;
                    for k in 0..N_BANDS {
                        e[k] += s.band_power[t][k];
                    }
                }
            }
            let total: f64 = e.iter().sum();
            (any && total > 0.0).then(|| e.map(|v| 10.0 * (v / total).log10()))
        })
        .collect()
}

/// Sum of several signals (interleaved, possibly mono and stereo) on a stereo bus with the PoC's centre law
/// (mono at √½ per channel): the family signal of ANA-16.
pub fn sum_to_stereo(signals: &[(&[f32], usize)]) -> Vec<f32> {
    let n = signals.iter().map(|(s, c)| s.len() / c).max().unwrap_or(0);
    let mut acc = vec![0f64; n * 2];
    let k = std::f64::consts::FRAC_1_SQRT_2;
    for (s, c) in signals {
        for i in 0..s.len() / c {
            if *c == 1 {
                let v = f64::from(s[i]) * k;
                acc[2 * i] += v;
                acc[2 * i + 1] += v;
            } else {
                acc[2 * i] += f64::from(s[2 * i]);
                acc[2 * i + 1] += f64::from(s[2 * i + 1]);
            }
        }
    }
    acc.into_iter().map(|v| v as f32).collect()
}

pub use spectral::analyze;

#[cfg(test)]
mod tests {
    use super::*;

    /// req: ANA-15, SEC-2
    #[test]
    fn per_section_loudness_activity_and_bands() {
        let rate = 48000u32;
        // 4 s of a 100 Hz tone, then 4 s of a 3 kHz tone 10 dB lower, then 4 s of silence.
        let mut x = Vec::new();
        for i in 0..4 * rate as usize {
            x.push((0.5 * (2.0 * std::f64::consts::PI * 100.0 * i as f64 / 48000.0).sin()) as f32);
        }
        for i in 0..4 * rate as usize {
            x.push((0.158 * (2.0 * std::f64::consts::PI * 3000.0 * i as f64 / 48000.0).sin()) as f32);
        }
        x.extend(std::iter::repeat_n(0.0, 4 * rate as usize));
        let secs = [(0, 4 * 48000), (4 * 48000, 8 * 48000), (8 * 48000, 12 * 48000)];
        let l = section_lufs(&x, 1, rate, &secs);
        assert!(l[0].unwrap() > l[1].unwrap() + 5.0, "{l:?}");
        assert_eq!(l[2], Some(f64::NEG_INFINITY));
        let s = spectral::analyze(&x, 1, rate);
        let a = section_active_pct(&s, &secs);
        assert!(a[0] > 95.0 && a[1] > 95.0 && a[2] < 10.0, "{a:?}");
        let b = section_bands(&s, &secs);
        assert!(b[0].unwrap()[1] > -1.0, "100 Hz sits in 60–120");
        assert!(b[1].unwrap()[5] > -1.0, "3 kHz sits in 2k–4k");
        // The first frames of the silent section still hold the end of the tone (170 ms window): if any band
        // profile is reported there, it is the 3 kHz tone's.
        assert!(b[2].is_none_or(|p| p[5] > -1.0));
    }

    /// req: ANA-16
    #[test]
    fn families_sum_mono_and_stereo_on_the_bus() {
        let a = [1.0f32, 0.0];
        let b = [0.5f32, -0.5, 0.25, 0.25];
        let s = sum_to_stereo(&[(&a, 1), (&b, 2)]);
        assert!((s[0] - (std::f32::consts::FRAC_1_SQRT_2 + 0.5)).abs() < 1e-6);
        assert!((s[1] - (std::f32::consts::FRAC_1_SQRT_2 - 0.5)).abs() < 1e-6);
        assert_eq!(s.len(), 4);
    }
}
