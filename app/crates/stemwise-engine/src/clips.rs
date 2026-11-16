//! Clip assembly (CLP-1, CLP-2): the input of an audio track, read sequentially from its clips with clip gain,
//! fades and mute applied. The same code feeds the real-time reader thread and the offline render, so both hear
//! exactly the same samples (ENG-4, AUT-4).

use std::path::Path;

use stemwise_core::media::{MediaError, PcmSource};
use stemwise_core::model::{Clip, FadeShape, Project, Track};

struct ClipSrc {
    clip: Clip,
    src: PcmSource,
    gain: f64,
}

/// Sequential reader of one audio track's input.
pub struct TrackStream {
    channels: usize,
    clips: Vec<ClipSrc>,
    scratch: Vec<f32>,
    pub pos: i64,
}

/// Envelope of a clip at `t` frames from its start (fades and gain).
fn envelope(c: &Clip, gain: f64, t: u64) -> f64 {
    let mut g = gain;
    let fi = c.fade_in.length;
    if fi > 0 && t < fi {
        let x = (t as f64 + 0.5) / fi as f64;
        g *= match c.fade_in.shape {
            FadeShape::Linear => x,
            FadeShape::EqualPower => (x * std::f64::consts::FRAC_PI_2).sin(),
        };
    }
    let fo = c.fade_out.length;
    if fo > 0 && t + fo >= c.length {
        let x = ((c.length - t) as f64 - 0.5) / fo as f64;
        g *= match c.fade_out.shape {
            FadeShape::Linear => x,
            FadeShape::EqualPower => (x * std::f64::consts::FRAC_PI_2).sin(),
        };
    }
    g
}

impl TrackStream {
    pub fn open(package: &Path, project: &Project, track: &Track) -> Result<Self, MediaError> {
        let mut clips = Vec::new();
        for c in track.clips.iter().filter(|c| !c.mute && c.length > 0) {
            let Some(m) = project.media(&c.media) else { continue };
            // A file missing from the package plays silent until it is located again (UI-6, design 13d).
            if !package.join(&m.path).is_file() {
                continue;
            }
            let src = PcmSource::open(package, m)?;
            clips.push(ClipSrc { clip: c.clone(), src, gain: stemwise_dsp::db_to_gain(c.gain_db) });
        }
        clips.sort_by_key(|c| c.clip.start);
        Ok(TrackStream { channels: usize::from(track.channels.max(1)), clips, scratch: Vec::new(), pos: 0 })
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn seek(&mut self, frame: i64) {
        self.pos = frame;
    }

    /// Fills `out` (interleaved, `channels` per frame) with the next frames and advances.
    pub fn read(&mut self, out: &mut [f32]) -> std::io::Result<()> {
        let ch = self.channels;
        let frames = out.len() / ch;
        out.fill(0.0);
        let (a, b) = (self.pos, self.pos + frames as i64);
        for cs in &mut self.clips {
            let (s, e) = (cs.clip.start, cs.clip.end());
            if e <= a || s >= b {
                continue;
            }
            let from = a.max(s);
            let to = b.min(e);
            let n = (to - from) as usize;
            let src_ch = usize::from(cs.src.channels());
            self.scratch.resize(n * src_ch, 0.0);
            let in_clip = (from - s) as u64;
            cs.src.read(cs.clip.offset + in_clip, &mut self.scratch)?;
            let base = (from - a) as usize;
            for i in 0..n {
                let g = envelope(&cs.clip, cs.gain, in_clip + i as u64);
                for c in 0..ch {
                    // Channel adaptation if a clip does not match its track (never after CLP-1, but safe).
                    let v = if src_ch == ch {
                        self.scratch[i * src_ch + c]
                    } else if src_ch == 1 {
                        self.scratch[i]
                    } else {
                        0.5 * (self.scratch[i * src_ch] + self.scratch[i * src_ch + 1])
                    };
                    out[(base + i) * ch + c] += (f64::from(v) * g) as f32;
                }
            }
        }
        self.pos = b;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stemwise_core::model::Fade;

    fn clip(fi: u64, fo: u64, shape: FadeShape) -> Clip {
        Clip {
            id: "c".into(),
            media: "m".into(),
            start: 0,
            offset: 0,
            length: 100,
            gain_db: 0.0,
            fade_in: Fade { length: fi, shape },
            fade_out: Fade { length: fo, shape },
            mute: false,
        }
    }

    /// req: CLP-2
    #[test]
    fn fades_are_linear_or_equal_power_and_symmetric() {
        let c = clip(10, 10, FadeShape::Linear);
        assert!((envelope(&c, 1.0, 0) - 0.05).abs() < 1e-12);
        assert!((envelope(&c, 1.0, 9) - 0.95).abs() < 1e-12);
        assert_eq!(envelope(&c, 1.0, 50), 1.0);
        assert!((envelope(&c, 1.0, 99) - 0.05).abs() < 1e-12);
        let p = clip(11, 11, FadeShape::EqualPower);
        // Equal power: at the middle of the fade (sample 5 of 11, evaluated at its centre) the gain is −3.01 dB.
        let mid = envelope(&p, 1.0, 5);
        assert!((20.0 * mid.log10() + 3.0103).abs() < 1e-3, "{mid}");
        assert!((envelope(&p, 2.0, 50) - 2.0).abs() < 1e-12);
    }
}
