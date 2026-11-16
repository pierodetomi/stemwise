//! Bounce to master (MDN-1, MDN-4): the whole mix rendered offline to `mixdowns/mixdown-<n>.wav` (32-bit float),
//! trimmed only downwards to stay at or below −3 dBTP, with its measures and the hash of the mix state it comes from.

use std::path::Path;

use stemwise_core::model::{Mixdown, Project};
use stemwise_core::{package, wav};
use stemwise_dsp::{loudness, truepeak};

use crate::offline::{RenderError, render_range};

/// The mixdown ceiling (MDN-1).
pub const MIXDOWN_CEILING_DB: f64 = -3.0;
/// Reverb and delay tails rendered after the last clip.
pub const TAIL_SECONDS: f64 = 2.0;

/// Renders the mix and writes the next mixdown file. `progress` gets the fraction done and may return `false` to
/// cancel (design 8b). Returns the record to store in the project (MDN-4) and the stereo samples.
pub fn bounce(
    package: &Path,
    project: &Project,
    progress: &mut dyn FnMut(f64) -> bool,
) -> Result<(Mixdown, Vec<f32>), RenderError> {
    let rate = project.sample_rate.ok_or(RenderError::NoRate)?;
    let end = project.content_end() + (TAIL_SECONDS * f64::from(rate)) as i64;
    let r = render_range(package, project, 0, end, 4096, &mut |f| progress(f * 0.95))?;
    let mut x = r.interleaved();
    let tp = truepeak::true_peak_db(&x, 2);
    // Attenuation only: a quieter mix is left as it is (MDN-1).
    let trim_db = if tp.is_finite() && tp > MIXDOWN_CEILING_DB { MIXDOWN_CEILING_DB - tp - 0.01 } else { 0.0 };
    if trim_db < 0.0 {
        let g = 10f64.powf(trim_db / 20.0) as f32;
        for v in &mut x {
            *v *= g;
        }
    }
    let number = project.master.mixdown.as_ref().map_or(1, |m| m.number + 1);
    let rel = format!("mixdowns/mixdown-{number}.wav");
    std::fs::create_dir_all(package.join("mixdowns"))?;
    wav::write_f32(&package.join(&rel), 2, rate, &x)?;
    let after_tp = truepeak::true_peak_db(&x, 2);
    let rec = Mixdown {
        number,
        path: rel,
        mix_hash: package::mix_hash(project),
        trim_db: (trim_db * 100.0).round() / 100.0,
        frames: (x.len() / 2) as u64,
        created_ms: package::now_ms(),
        lufs: loudness::integrated(&x, 2, rate),
        true_peak_db: if after_tp.is_finite() { after_tp } else { -200.0 },
        lra: loudness::range(&x, 2, rate),
    };
    progress(1.0);
    Ok((rec, x))
}

/// Whether the mix changed since the mixdown (MDN-2).
pub fn is_stale(project: &Project) -> bool {
    project.master.mixdown.as_ref().is_some_and(|m| m.mix_hash != package::mix_hash(project))
}

/// A project that plays one stereo file (the mixdown, or a reference for the A/B) through a neutral strip at
/// `gain_db`: what the engine loads in the master phase (FOUNDATION §5.4: a second, simpler graph).
pub fn listen_project(project: &Project, path: &str, frames: u64, rate: u32, gain_db: f64) -> Project {
    use stemwise_core::model::{AudioFormat, Clip, Fade, MediaFile, Track};
    let mut p = project.clone();
    let media = MediaFile {
        id: "listen".into(),
        path: path.to_string(),
        original_name: path.to_string(),
        format: AudioFormat::Wav,
        channels: 2,
        sample_rate: rate,
        frames,
        encoding: "float 32".into(),
        converted: false,
    };
    let mut t = Track::new_audio("listen".into(), "Listen", None, 2, 1);
    t.chain.fader_db = gain_db;
    t.clips = vec![Clip {
        id: "c-listen".into(),
        media: "listen".into(),
        start: 0,
        offset: 0,
        length: frames,
        gain_db: 0.0,
        fade_in: Fade::default(),
        fade_out: Fade::default(),
        mute: false,
    }];
    p.media = vec![media];
    p.tracks = vec![t];
    p.mix_bus.fader_db = 0.0;
    p
}
