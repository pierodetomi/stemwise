//! Measures for the blocks of P-018 (ANA-19): sibilance of voices, short crest and transients of every track, the
//! floor between the hits of drums. Computed on the content after clip editing and before the chain (ANA-14), like
//! the other measures, and cached with them.
//!
//! - **Sibilance** (voices): the signal is summed to mono and filtered to 5–10 kHz (two 2nd-order Butterworth
//!   filters); in frames of 5 ms the band level is compared with the level of the body of the voice (below 5 kHz)
//!   over the surrounding 300 ms, in active frames only (voice above −50 dBFS). `sibilance_db` is the 98th
//!   percentile of that difference: how far the short peaks of the band stand above the body (around −20 dB for a
//!   soft voice, near 0 dB and above for a sibilant one). `sibilance_hz` is where they fall: the loudest of five
//!   one-kHz bands (5.5 … 9.5 kHz) over the frames above the 95th percentile.
//! - **Short crest**: the median, over active windows of 400 ms, of peak minus RMS (dB).
//! - **Transients**: for each onset (librosa-compatible, ANA-4) the peak around it (−15 … +20 ms, onsets fall on
//!   frames of about 10 ms) against the RMS from 40 to 140 ms after it; the median over onsets (dB). How much the attacks stand above what follows.
//! - **Floor** (drums): the RMS in the 50 ms ending 20 ms before each onset (after the previous hit has had 100 ms to
//!   decay) against
//!   the peak of the hits, median over onsets (dB, negative): how much bleed sits between the hits.

use serde::{Deserialize, Serialize};
use stemwise_dsp::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};

/// ANA-19 for one track (full precision; `None` where the track has no such thing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Character {
    pub sibilance_db: Option<f64>,
    pub sibilance_hz: Option<f64>,
    pub crest_db: Option<f64>,
    pub transient_db: Option<f64>,
    pub floor_db: Option<f64>,
}

fn mono(x: &[f32], ch: usize) -> Vec<f64> {
    if ch == 1 {
        return x.iter().map(|v| f64::from(*v)).collect();
    }
    x.chunks_exact(ch).map(|f| f.iter().map(|v| f64::from(*v)).sum::<f64>() / ch as f64).collect()
}

fn db(p: f64) -> f64 {
    10.0 * p.max(1e-20).log10()
}

fn percentile(v: &mut [f64], q: f64) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let i = ((v.len() - 1) as f64 * q).round() as usize;
    Some(v[i])
}

fn filtered(x: &[f64], stages: &[(Kind, f64)], rate: f64) -> Vec<f64> {
    let mut y = x.to_vec();
    for &(k, f) in stages {
        let mut b = Biquad::new(Coeffs::design(k, f, 0.0, BUTTERWORTH_Q, rate));
        b.filter_f64(0, &mut y);
    }
    y
}

/// Mean power of `x[a..b]`.
fn power(x: &[f64], a: usize, b: usize) -> f64 {
    let b = b.min(x.len());
    if b <= a {
        return 0.0;
    }
    x[a..b].iter().map(|v| v * v).sum::<f64>() / (b - a) as f64
}

fn peak(x: &[f64], a: usize, b: usize) -> f64 {
    x[a.min(x.len())..b.min(x.len())].iter().fold(0.0f64, |m, v| m.max(v.abs()))
}

/// Sibilance of a voice: (dB the 5–10 kHz short peaks stand above the body, where they fall).
pub fn sibilance(x: &[f32], ch: usize, rate: u32) -> (Option<f64>, Option<f64>) {
    let fs = f64::from(rate);
    let m = mono(x, ch);
    let band = filtered(&m, &[(Kind::HighPass, 5000.0), (Kind::LowPass, 10000.0)], fs);
    let low = filtered(&m, &[(Kind::LowPass, 5000.0)], fs);
    let f = (0.005 * fs) as usize;
    let body = (0.300 * fs) as usize;
    let mut diffs = Vec::new();
    let mut frames = Vec::new();
    let mut i = 0;
    while i + f <= m.len() {
        let c = i + f / 2;
        let a = c.saturating_sub(body / 2);
        if db(power(&m, a, a + body)) > -50.0 {
            let d = db(power(&band, i, i + f)) - db(power(&low, a, a + body));
            diffs.push(d);
            frames.push((i, d));
        }
        i += f;
    }
    let mut v = diffs.clone();
    let Some(p98) = percentile(&mut v, 0.98) else { return (None, None) };
    let p95 = percentile(&mut v, 0.95).unwrap_or(p98);
    let centres = [5500.0, 6500.0, 7500.0, 8500.0, 9500.0];
    let subs: Vec<Vec<f64>> =
        centres.iter().map(|&c| filtered(&m, &[(Kind::HighPass, c - 500.0), (Kind::LowPass, c + 500.0)], fs)).collect();
    let mut e = [0.0f64; 5];
    for &(i, d) in &frames {
        if d >= p95 {
            for (k, s) in subs.iter().enumerate() {
                e[k] += power(s, i, i + f);
            }
        }
    }
    let k = (0..5).max_by(|a, b| e[*a].total_cmp(&e[*b])).unwrap_or(0);
    (Some(p98), (e[k] > 0.0).then_some(centres[k]))
}

/// Median of peak minus RMS over active 400 ms windows (dB).
pub fn short_crest(x: &[f32], ch: usize, rate: u32) -> Option<f64> {
    let m = mono(x, ch);
    let w = (0.4 * f64::from(rate)) as usize;
    let mut v = Vec::new();
    let mut i = 0;
    while i + w <= m.len() {
        let p = power(&m, i, i + w);
        if db(p) > -60.0 {
            v.push(20.0 * peak(&m, i, i + w).log10() - db(p));
        }
        i += w;
    }
    percentile(&mut v, 0.5)
}

/// (transients, floor) from the onsets: the attacks against what follows them; the level between hits against the hits.
pub fn hits(x: &[f32], ch: usize, rate: u32, floor: bool) -> (Option<f64>, Option<f64>) {
    let fs = f64::from(rate);
    let m = mono(x, ch);
    let ons = crate::onsets::onset_samples(x, ch, rate);
    let ms = |v: f64| (v * fs / 1000.0) as usize;
    let mut tr = Vec::new();
    let mut fl = Vec::new();
    let mut prev: Option<usize> = None;
    for &o in &ons {
        // Onsets fall on analysis frames of about 10 ms: the hit is looked for around them.
        let pk = peak(&m, o.saturating_sub(ms(15.0)), o + ms(20.0));
        if pk <= 1e-6 {
            continue;
        }
        let after = power(&m, o + ms(40.0), o + ms(140.0));
        if after > 0.0 {
            tr.push(20.0 * pk.log10() - db(after));
        }
        if floor && let Some(p) = prev {
            let end = o.saturating_sub(ms(20.0));
            let a = (p + ms(100.0)).max(end.saturating_sub(ms(50.0)));
            if a + ms(10.0) < end {
                fl.push(db(power(&m, a, end)) - 20.0 * pk.log10());
            }
        }
        prev = Some(o);
    }
    (percentile(&mut tr, 0.5), if floor { percentile(&mut fl, 0.5) } else { None })
}

/// ANA-19 for one track: sibilance only for voices, the floor only for drums.
pub fn measure(x: &[f32], ch: usize, rate: u32, voice: bool, drums: bool) -> Character {
    let (sibilance_db, sibilance_hz) = if voice { sibilance(x, ch, rate) } else { (None, None) };
    let (transient_db, floor_db) = hits(x, ch, rate, drums);
    Character { sibilance_db, sibilance_hz, crest_db: short_crest(x, ch, rate), transient_db, floor_db }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: u32 = 48000;

    fn tone(n: usize, f: f64, a: f64) -> Vec<f64> {
        (0..n).map(|i| a * (2.0 * std::f64::consts::PI * f * i as f64 / f64::from(R)).sin()).collect()
    }

    /// req: ANA-19
    #[test]
    fn sibilance_rises_with_the_esses_and_finds_their_frequency() {
        let n = 2 * R as usize;
        // A "voice" at 200 Hz; every 0.25 s an 80 ms burst at 7.5 kHz.
        let body = tone(n, 200.0, 0.3);
        let ess = tone(n, 7500.0, 1.0);
        let mk = |k: f64| -> Vec<f32> {
            (0..n).map(|i| (body[i] + if (i % 12000) < 3840 { k * ess[i] } else { 0.0 }) as f32).collect()
        };
        let (soft, _) = sibilance(&mk(0.02), 1, R);
        let (hard, hz) = sibilance(&mk(0.3), 1, R);
        // 0.3 vs 0.02 is +23.5 dB of ess against a body at 0.3: about −23.5 and 0 dB.
        assert!(hard.unwrap() - soft.unwrap() > 15.0, "{soft:?} {hard:?}");
        assert!((-4.0..4.0).contains(&hard.unwrap()) && soft.unwrap() < -15.0, "{soft:?} {hard:?}");
        assert_eq!(hz, Some(7500.0));
    }

    /// req: ANA-19
    #[test]
    fn transients_and_floor_of_a_drum() {
        let n = 3 * R as usize;
        // Hits every 0.5 s decaying in 20 ms, over a bleed floor at −40 dB of the hit.
        let mk = |bleed: f64| -> Vec<f32> {
            let noise = stemwise_dsp::testsignal::xorshift32(3, n);
            (0..n)
                .map(|i| {
                    let t = (i % 24000) as f64 / f64::from(R);
                    // The bleed is a steady low tone: it never makes onsets of its own.
                    let hum = (2.0 * std::f64::consts::PI * 100.0 * i as f64 / f64::from(R)).sin();
                    (noise[i] * (-t / 0.02).exp() + bleed * hum) as f32
                })
                .collect()
        };
        let (tr, fl) = hits(&mk(0.01), 1, R, true);
        assert!(tr.unwrap() > 10.0, "a sharp attack stands well above its tail: {tr:?}");
        let (_, fl_more) = hits(&mk(0.1), 1, R, true);
        // More bleed → a higher floor (by about 20 dB; ±6 for the onset positions and the noise peaks).
        let d = fl_more.unwrap() - fl.unwrap();
        assert!((d - 20.0).abs() < 6.0, "{fl:?} {fl_more:?}");
        assert!(short_crest(&mk(0.01), 1, R).unwrap() > 6.0);
    }
}
