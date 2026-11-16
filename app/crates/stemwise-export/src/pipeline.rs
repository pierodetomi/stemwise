//! The export of a master (MAS-2, MAS-3, MAS-7 … MAS-10): the mixdown converted to the export rate, rendered through
//! the master chain normalised to the destination, checked against the true-peak ceiling, quantised (TPDF dither
//! at 16 bit), written in every chosen format with its tags, then measured back from each file for the report.
//!
//! The ceiling is guaranteed on the file (MAS-3): the limiter always runs in the export; each written file is decoded
//! and its 4x true peak measured; a file over the ceiling (quantisation, dither, lossy coding) is trimmed by the
//! excess and written again, and an export that cannot get under it fails rather than deliver a file over it
//! (MAS-10). The reports are measured on the decoded files. WAV and FLAC are deterministic (MAS-8).

use std::path::{Path, PathBuf};

use stemwise_analysis::evaluate::spectral_distance;
use stemwise_analysis::measures::measure_mix;
use stemwise_core::media;
use stemwise_core::model::{MasterState, Metadata, Section};
use stemwise_dsp::dither::{Dither, quantise_plain};
use stemwise_dsp::{loudness, truepeak};
use stemwise_engine::master::{MasterParams, loudness_search, render};

use crate::tags::{Container, Cover, write_tags};
use crate::wav::{Bext, WavSamples, write_wav};
use crate::{ExportError, Result, aac, flac};

/// What to write.
pub struct Settings<'a> {
    /// WAV bit depth (16, 24, or 32 for float), if WAV is wanted.
    pub wav: Option<u32>,
    /// FLAC bit depth (16 or 24), if FLAC is wanted.
    pub flac: Option<u32>,
    /// AAC bit rate in kbit/s, if AAC is wanted.
    pub aac: Option<u32>,
    pub rate: u32,
    /// TPDF dither at 16 bit (MAS-1); on unless the master's dither block is bypassed.
    pub dither: bool,
    pub folder: &'a Path,
    pub basename: &'a str,
    pub meta: &'a Metadata,
    pub cover: Option<Cover<'a>>,
    /// Unix seconds written as the origination date of the `bext` chunk (the caller passes the export time, so the
    /// same master and the same moment give the same bytes, MAS-8).
    pub when_secs: u64,
    /// The sections of the project, for the master's offsets per section (MAS-14).
    pub sections: &'a [Section],
}

/// One written file and its measures (design 10c).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct FileReport {
    pub path: String,
    pub format: String,
    pub bytes: u64,
    pub lufs: Option<f64>,
    pub true_peak_db: f64,
    pub lra: Option<f64>,
    /// Spectral distance to the references' profile (dB), when there is one.
    pub distance_db: Option<f64>,
}

/// Seed of the 16-bit dither: fixed, so the same master gives the same file (MAS-8).
const DITHER_SEED: u32 = 0x5745_4D53;
/// Extra margin when a lossy file comes back over the ceiling (MAS-10).
const LOSSY_MARGIN_DB: f64 = 0.1;

fn split(x: &[f32]) -> (Vec<f32>, Vec<f32>) {
    (x.iter().step_by(2).copied().collect(), x.iter().skip(1).step_by(2).copied().collect())
}

fn measure(x: &[f32], rate: u32, target_bands: Option<&[f64; 8]>) -> (Option<f64>, f64, Option<f64>, Option<f64>) {
    let tp = truepeak::true_peak_db(x, 2);
    let distance = target_bands.and_then(|t| measure_mix(x, 2, rate).0.bands_db.map(|b| spectral_distance(&b, t).0));
    (loudness::integrated(x, 2, rate), if tp.is_finite() { tp } else { -200.0 }, loudness::range(x, 2, rate), distance)
}

fn quantise(x: &[f32], bits: u32, dither: bool) -> Vec<i32> {
    if bits == 16 && dither {
        let mut d = Dither::new(DITHER_SEED);
        x.iter().map(|v| d.quantise(f64::from(*v), 16)).collect()
    } else {
        x.iter().map(|v| quantise_plain(f64::from(*v), bits)).collect()
    }
}

/// The master as floats at the export rate (interleaved stereo), at the destination's loudness and under its ceiling.
/// `sections` are the project's (frames at `mix_rate`), for the offsets per section.
pub fn render_master(
    mixdown: &[f32],
    mix_rate: u32,
    master: &MasterState,
    sections: &[Section],
    mix_bands: Option<&[f64; 8]>,
    target_bands: Option<&[f64; 8]>,
    rate: u32,
) -> Result<Vec<f32>> {
    Ok(render_master_from(mixdown, mix_rate, master, sections, mix_bands, target_bands, rate, None, &mut |_| {})?
        .samples)
}

/// A master rendered, with what the render found.
pub struct Rendered {
    /// Interleaved stereo at the rate asked for.
    pub samples: Vec<f32>,
    /// The loudness gain (MAS-2).
    pub gain_db: f64,
    /// The peak at the input of each clipper (MAS-11).
    pub clip_peak: [f64; stemwise_engine::master::PER_KIND],
}

/// As [`render_master`], with the loudness gain and the clipper peaks it found: one search, its last pass reused as
/// the render. With `start` (the gain of a nearby state) the search is shorter: only for what the workspace shows
/// while the master changes; the export always searches from scratch, so that the same project always gives the
/// same file (MAS-8). `progress` gets the share of the search done.
#[allow(clippy::too_many_arguments)]
pub fn render_master_from(
    mixdown: &[f32],
    mix_rate: u32,
    master: &MasterState,
    sections: &[Section],
    mix_bands: Option<&[f64; 8]>,
    target_bands: Option<&[f64; 8]>,
    rate: u32,
    start: Option<f64>,
    progress: &mut dyn FnMut(f64),
) -> Result<Rendered> {
    let resampled;
    let scaled: Vec<Section>;
    let (x, sections): (&[f32], &[Section]) = if mix_rate == rate {
        (mixdown, sections)
    } else {
        resampled = media::resample(mixdown, 2, mix_rate, rate).map_err(|e| ExportError::Encoder(e.to_string()))?;
        // The sections lie on the timeline of the mixdown: at another rate their frames scale with it.
        let k = f64::from(rate) / f64::from(mix_rate);
        let at = |f: i64| (f as f64 * k).round() as i64;
        scaled = sections.iter().map(|s| Section { start: at(s.start), end: at(s.end), ..s.clone() }).collect();
        (&resampled, &scaled)
    };
    let (l, r) = split(x);
    let mut p = MasterParams::of(master, mix_bands, target_bands, 0.0).with_sections(master, sections);
    // The export always limits, whatever the listening bypass (MAS-3).
    p.limiter_on = true;
    let found = loudness_search(&l, &r, rate, p, master.target.lufs, start, progress);
    p.loud_gain_db = found.gain_db;
    p.clip_peak = found.clip_peak;
    let (a, b) = found.rendered.unwrap_or_else(|| render(&l, &r, f64::from(rate), p));
    let mut out: Vec<f32> = a.iter().zip(&b).flat_map(|(x, y)| [*x, *y]).collect();
    // Belt and braces: a linear trim keeps any residual overshoot under the ceiling.
    let tp = truepeak::true_peak_db(&out, 2);
    let ceiling = master.target.true_peak_db;
    if tp.is_finite() && tp > ceiling {
        scale(&mut out, ceiling - tp - TRIM_MARGIN_DB);
    }
    Ok(Rendered { samples: out, gain_db: p.loud_gain_db, clip_peak: p.clip_peak })
}

/// Margin of the linear trims under the ceiling.
const TRIM_MARGIN_DB: f64 = 0.02;
/// Attempts at bringing a file under the ceiling before the export gives up with an error (MAS-3).
const ATTEMPTS: usize = 6;

fn scale(x: &mut [f32], db: f64) {
    let g = 10f64.powf(db / 20.0) as f32;
    for v in x {
        *v *= g;
    }
}

/// The highest rate the AAC encoder takes (AudioToolbox: 48 kHz).
pub const AAC_MAX_RATE: u32 = 48_000;

/// Why a set of settings cannot be exported, before anything is written.
pub fn check(s: &Settings<'_>) -> Result<()> {
    if s.aac.is_some() {
        if !aac::aac_available() {
            return Err(ExportError::Unsupported("AAC is not available on this system".into()));
        }
        if s.rate > AAC_MAX_RATE {
            return Err(ExportError::Unsupported(format!(
                "AAC goes up to {} kHz: choose 44.1 or 48 kHz, or leave AAC out",
                AAC_MAX_RATE / 1000
            )));
        }
    }
    if s.wav.is_none() && s.flac.is_none() && s.aac.is_none() {
        return Err(ExportError::Unsupported("no format chosen".into()));
    }
    Ok(())
}

/// Writes every chosen format and returns their reports. `progress` gets the fraction done (0–1) and answers whether
/// to go on: `false` cancels, and the files this export already wrote are removed.
pub fn export(
    mixdown: &[f32],
    mix_rate: u32,
    master: &MasterState,
    mix_bands: Option<&[f64; 8]>,
    target_bands: Option<&[f64; 8]>,
    s: &Settings<'_>,
    progress: &mut dyn FnMut(f64) -> bool,
) -> Result<Vec<FileReport>> {
    check(s)?;
    let mut written: Vec<PathBuf> = Vec::new();
    let r = export_inner(mixdown, mix_rate, master, mix_bands, target_bands, s, progress, &mut written);
    if r.is_err() {
        for p in &written {
            let _ = std::fs::remove_file(p);
        }
    }
    r
}

#[allow(clippy::too_many_arguments)]
fn export_inner(
    mixdown: &[f32],
    mix_rate: u32,
    master: &MasterState,
    mix_bands: Option<&[f64; 8]>,
    target_bands: Option<&[f64; 8]>,
    s: &Settings<'_>,
    progress: &mut dyn FnMut(f64) -> bool,
    written: &mut Vec<PathBuf>,
) -> Result<Vec<FileReport>> {
    let go =
        |progress: &mut dyn FnMut(f64) -> bool, f: f64| if progress(f) { Ok(()) } else { Err(ExportError::Cancelled) };
    go(progress, 0.0)?;
    let x = render_master(mixdown, mix_rate, master, s.sections, mix_bands, target_bands, s.rate)?;
    go(progress, 0.5)?;
    std::fs::create_dir_all(s.folder)?;
    let ceiling = master.target.true_peak_db;
    let mut reports = Vec::new();
    let now = s.when_secs;
    let bext = Bext {
        description: [s.meta.artist.as_str(), s.meta.title.as_str()]
            .iter()
            .filter(|v| !v.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" — "),
        date: civil_date(now),
        time: format!("{:02}:{:02}:{:02}", (now / 3600) % 24, (now / 60) % 60, now % 60),
    };
    let cover = || s.cover.as_ref().map(|c| Cover { data: c.data, mime: c.mime });
    let file = |ext: &str| -> PathBuf { s.folder.join(format!("{}.{ext}", s.basename)) };
    let total = [s.wav.is_some(), s.flac.is_some(), s.aac.is_some()].iter().filter(|b| **b).count().max(1) as f64;
    let done = std::cell::Cell::new(0.0);
    let frac = || 0.5 + 0.5 * done.get() / total;
    let step = |progress: &mut dyn FnMut(f64) -> bool| {
        done.set(done.get() + 1.0);
        go(progress, frac())
    };
    // MAS-3 on the file: write, decode what was written, measure; over the ceiling (quantisation and dither can add
    // a fraction of a dB), trim the floats by the excess and write again. Deterministic: same input, same steps.
    let mut lossless = |ext: &str, bits: u32, write: &dyn Fn(&Path, &[f32]) -> Result<()>| -> Result<Vec<f32>> {
        let path = file(ext);
        let mut y = x.clone();
        for attempt in 0..ATTEMPTS {
            write(&path, &y)?;
            if !written.contains(&path) {
                written.push(path.clone());
            }
            let back = media::decode_all(&path).map_err(|e| ExportError::Encoder(e.to_string()))?.1;
            let tp = truepeak::true_peak_db(&back, 2);
            if !tp.is_finite() || tp <= ceiling {
                return Ok(back);
            }
            if attempt + 1 == ATTEMPTS {
                break;
            }
            // One quantisation step of margin on top of the excess.
            let lsb_db = if bits >= 32 { 0.0 } else { 20.0 * (1.0 + 2.0f64.powi(-(bits as i32) + 2)).log10() };
            scale(&mut y, ceiling - tp - TRIM_MARGIN_DB - lsb_db);
        }
        Err(ExportError::Encoder(format!("the {ext} file stays over the {ceiling:.1} dBTP ceiling")))
    };
    if let Some(bits) = s.wav {
        let back = lossless("wav", bits, &|path, y| {
            let ints = if bits == 32 { Vec::new() } else { quantise(y, bits, s.dither) };
            let samples = if bits == 32 { WavSamples::Float(y) } else { WavSamples::Int(&ints, bits) };
            write_wav(path, samples, 2, s.rate, Some(&bext))?;
            write_tags(path, Container::Wav, s.meta, cover())
        })?;
        let label = format!("WAV {}", if bits == 32 { "32-bit float".into() } else { format!("{bits}-bit") });
        reports.push(report(&file("wav"), &label, &back, s.rate, target_bands));
        step(progress)?;
    }
    if let Some(bits) = s.flac {
        let back = lossless("flac", bits, &|path, y| {
            flac::encode_flac(path, &quantise(y, bits, s.dither), 2, s.rate, bits)?;
            write_tags(path, Container::Flac, s.meta, cover())
        })?;
        reports.push(report(&file("flac"), &format!("FLAC {bits}-bit"), &back, s.rate, target_bands));
        step(progress)?;
    }
    if let Some(kbps) = s.aac {
        let path = file("m4a");
        let mut gain_db = 0.0;
        let mut back = Vec::new();
        let mut under = false;
        let mut used = kbps;
        for _ in 0..ATTEMPTS {
            let mut y = x.clone();
            scale(&mut y, gain_db);
            used = aac::encode_aac(&path, &y, 2, s.rate, kbps)?;
            if !written.contains(&path) {
                written.push(path.clone());
            }
            back = media::decode_all(&path).map_err(|e| ExportError::Encoder(e.to_string()))?.1;
            let tp = truepeak::true_peak_db(&back, 2);
            // MAS-10: measured on the decoded file; over the ceiling, encode again with the margin it needs.
            if !tp.is_finite() || tp <= ceiling {
                under = true;
                break;
            }
            gain_db -= tp - ceiling + LOSSY_MARGIN_DB;
            go(progress, frac())?;
        }
        if !under {
            return Err(ExportError::Encoder(format!("the AAC file stays over the {ceiling:.1} dBTP ceiling")));
        }
        write_tags(&path, Container::M4a, s.meta, cover())?;
        reports.push(report(&path, &format!("AAC {used} kbit/s"), &back, s.rate, target_bands));
        step(progress)?;
    }
    Ok(reports)
}

fn report(path: &Path, format: &str, samples: &[f32], rate: u32, target: Option<&[f64; 8]>) -> FileReport {
    let (lufs, tp, lra, distance_db) = measure(samples, rate, target);
    FileReport {
        path: path.display().to_string(),
        format: format.to_string(),
        bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        lufs,
        true_peak_db: tp,
        lra,
        distance_db,
    }
}

/// "YYYY-MM-DD" (UTC) from Unix seconds, for the `bext` chunk.
fn civil_date(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    // Howard Hinnant's days-from-civil, inverted.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: MAS-7
    #[test]
    fn civil_dates() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(1_794_355_200), "2026-11-11");
    }
}
