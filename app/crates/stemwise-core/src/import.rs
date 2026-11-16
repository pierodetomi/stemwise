//! Importing stems and reference tracks into a project (PRJ-3 … PRJ-7).

use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::command::Command;
use crate::media::{self, MediaError};
use crate::model::{AudioFormat, Clip, Fade, Project, ReferenceTrack, Track};
use crate::role::Role;

/// One file as the import screen lists it (design 2c).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportCandidate {
    pub path: String,
    pub name: String,
    pub format: AudioFormat,
    pub channels: u16,
    pub sample_rate: u32,
    pub frames: u64,
    pub encoding: String,
    pub role: Role,
    /// The file's rate differs from the project's (PRJ-5): the user must choose to convert it or leave it out.
    pub rate_mismatch: bool,
    pub error: Option<String>,
}

/// A file the user confirmed, with the role they chose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ImportChoice {
    pub path: String,
    pub role: Role,
    pub name: String,
    /// Accept conversion to the project rate when `rate_mismatch` is set (PRJ-5).
    pub convert: bool,
}

const STEM_FORMATS: [AudioFormat; 3] = [AudioFormat::Wav, AudioFormat::Flac, AudioFormat::Aiff];

fn display_name(path: &Path) -> String {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or("Audio").replace('_', " ")
}

/// Probes dropped files. The project rate is the first import's (PRJ-5): before anything is imported, the first
/// readable file of the batch sets it.
pub fn candidates(project: &Project, paths: &[PathBuf]) -> Vec<ImportCandidate> {
    candidates_at(project.sample_rate, paths)
}

/// As [`candidates`], against a rate known by the caller: a project still being defined has none yet, or the rate
/// of the first file already listed for it (the new-project screen, before the project exists).
pub fn candidates_at(rate: Option<u32>, paths: &[PathBuf]) -> Vec<ImportCandidate> {
    let mut rate = rate;
    paths
        .iter()
        .map(|p| {
            let name = display_name(p);
            let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            match media::probe(p) {
                Ok(info) if STEM_FORMATS.contains(&info.format) => {
                    let r = *rate.get_or_insert(info.sample_rate);
                    ImportCandidate {
                        path: p.display().to_string(),
                        name,
                        format: info.format,
                        channels: info.channels,
                        sample_rate: info.sample_rate,
                        frames: info.frames,
                        encoding: info.encoding,
                        role: Role::guess(&file_name),
                        rate_mismatch: info.sample_rate != r,
                        error: None,
                    }
                }
                Ok(info) => failed(p, name, info.format, "MP3 and AAC are accepted as references only"),
                Err(e) => failed(p, name, AudioFormat::Wav, &e.to_string()),
            }
        })
        .collect()
}

fn failed(p: &Path, name: String, format: AudioFormat, msg: &str) -> ImportCandidate {
    ImportCandidate {
        path: p.display().to_string(),
        name,
        format,
        channels: 0,
        sample_rate: 0,
        frames: 0,
        encoding: String::new(),
        role: Role::Other,
        rate_mismatch: false,
        error: Some(msg.to_string()),
    }
}

/// Copies the chosen files into the package and returns the commands of one history step: the project rate (on
/// the first import), the media entries, one audio track per file with a clip starting at zero (PRJ-4).
/// Files with a different rate are converted when `convert` is set and skipped otherwise.
pub fn import(package: &Path, project: &mut Project, choices: &[ImportChoice]) -> Result<Vec<Command>, MediaError> {
    let mut cmds = Vec::new();
    let mut rate = project.sample_rate;
    let mut insert_at = project.tracks.iter().position(|t| t.is_return()).unwrap_or(project.tracks.len());
    for choice in choices {
        let path = Path::new(&choice.path);
        let info = media::probe(path)?;
        if !STEM_FORMATS.contains(&info.format) {
            return Err(MediaError::Unsupported("MP3 and AAC are accepted as references only".into()));
        }
        let project_rate = *rate.get_or_insert(info.sample_rate);
        if info.sample_rate != project_rate && !choice.convert {
            continue;
        }
        let convert = (info.sample_rate != project_rate).then_some(project_rate);
        let m = media::import_into(package, path, "audio", convert)?;
        let track_id = project.new_id("t");
        let clip_id = project.new_id("c");
        let mut t = Track::new_audio(track_id, &choice.name, Some(choice.role), m.channels, choice.role.color());
        t.clips.push(Clip {
            id: clip_id,
            media: m.id.clone(),
            start: 0,
            offset: 0,
            length: m.frames,
            gain_db: 0.0,
            fade_in: Fade::default(),
            fade_out: Fade::default(),
            mute: false,
        });
        cmds.push(Command::AddMedia { media: Box::new(m) });
        cmds.push(Command::InsertTrack { index: insert_at, track: Box::new(t) });
        insert_at += 1;
    }
    if project.sample_rate.is_none() && rate.is_some() && !cmds.is_empty() {
        cmds.insert(0, Command::SetSampleRate { sample_rate: rate });
    }
    Ok(cmds)
}

/// Adds a reference track (PRJ-7): WAV, FLAC, AIFF, MP3 or AAC, copied as is (used for analysis and comparison).
pub fn add_reference(package: &Path, project: &Project, path: &Path) -> Result<Command, MediaError> {
    if project.references.len() >= 3 {
        return Err(MediaError::Unsupported("at most 3 reference tracks".into()));
    }
    let m = media::import_into(package, path, "references", None)?;
    let mut refs = project.references.clone();
    let n = (1..=3).find(|n| !refs.iter().any(|r| r.id == format!("ref{n}"))).unwrap_or(1);
    refs.push(ReferenceTrack { id: format!("ref{n}"), media: m });
    Ok(Command::SetReferences { references: refs })
}

pub fn remove_reference(project: &Project, id: &str) -> Command {
    Command::SetReferences { references: project.references.iter().filter(|r| r.id != id).cloned().collect() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{Document, Phase};
    use crate::media::{self, PcmSource, Peaks};
    use crate::package;
    use crate::wav;
    use std::fs;

    fn tmp() -> tempfile::TempDir {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
        fs::create_dir_all(&root).unwrap();
        tempfile::tempdir_in(root).unwrap()
    }

    fn sine(n: usize, ch: usize, freq: f32, rate: f32) -> Vec<f32> {
        (0..n * ch).map(|i| 0.5 * (2.0 * std::f32::consts::PI * freq * (i / ch) as f32 / rate).sin()).collect()
    }

    /// req: PRJ-3, PRJ-5
    #[test]
    fn files_are_listed_before_the_project_exists_against_the_first_files_rate() {
        let t = tmp();
        let s = sine(4800, 1, 440.0, 48000.0);
        wav::write_pcm(&t.path().join("Kick.wav"), 1, 48000, 24, &s, false, false).unwrap();
        wav::write_pcm(&t.path().join("Snare.wav"), 1, 48000, 24, &s, false, false).unwrap();
        wav::write_pcm(&t.path().join("Vox 44.wav"), 1, 44100, 24, &sine(4410, 1, 440.0, 44100.0), false, false)
            .unwrap();
        // No project and nothing listed yet: the first file sets the rate.
        let first = candidates_at(None, &[t.path().join("Kick.wav")]);
        assert!(first[0].error.is_none() && !first[0].rate_mismatch);
        // A second batch is compared with the rate of the first file already listed.
        let more =
            candidates_at(Some(first[0].sample_rate), &[t.path().join("Snare.wav"), t.path().join("Vox 44.wav")]);
        assert!(!more[0].rate_mismatch && more[1].rate_mismatch, "{more:?}");
    }

    /// req: UI-6
    #[test]
    fn missing_files_are_found_and_relinked() {
        let t = tmp();
        let src = t.path().join("src");
        fs::create_dir_all(&src).unwrap();
        let s = sine(4800, 1, 440.0, 48000.0);
        wav::write_pcm(&src.join("Kick.wav"), 1, 48000, 24, &s, false, false).unwrap();
        let (dir, p) = package::create(t.path(), "Relink").unwrap();
        let mut d = Document::new(p);
        let choice = ImportChoice {
            path: src.join("Kick.wav").display().to_string(),
            role: Role::Kick,
            name: "Kick".into(),
            convert: false,
        };
        let cmds = import(&dir, &mut d.project, &[choice]).unwrap();
        d.transact(Phase::Mix, "Import", cmds, None).unwrap();
        let m = d.project.media[0].clone();
        assert!(package::missing_media(&dir, &d.project).is_empty());
        // The file disappears from the package (deleted in the Finder).
        let stored = dir.join(&m.path);
        let mut perm = fs::metadata(&stored).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perm.set_readonly(false);
        fs::set_permissions(&stored, perm).unwrap();
        fs::remove_file(&stored).unwrap();
        assert_eq!(package::missing_media(&dir, &d.project), vec![m.id.clone()]);
        // A file of another length or channel count is refused and leaves nothing behind.
        wav::write_pcm(&src.join("Short.wav"), 1, 48000, 24, &s[..2400], false, false).unwrap();
        wav::write_pcm(&src.join("Stereo.wav"), 2, 48000, 24, &sine(4800, 2, 440.0, 48000.0), false, false).unwrap();
        assert!(media::relink(&dir, &d.project, &m, &src.join("Short.wav")).is_err());
        assert!(media::relink(&dir, &d.project, &m, &src.join("Stereo.wav")).is_err());
        // Another rate is not converted in silence (PRJ-5): the original was not a converted file.
        wav::write_pcm(&src.join("Kick 44.wav"), 1, 44100, 24, &sine(4410, 1, 440.0, 44100.0), false, false).unwrap();
        let e = media::relink(&dir, &d.project, &m, &src.join("Kick 44.wav")).unwrap_err();
        assert!(e.to_string().contains("44100 Hz"), "{e}");
        assert_eq!(fs::read_dir(dir.join("audio")).unwrap().count(), 0);
        let peaks = dir.join("cache").join("peaks");
        let caches = || fs::read_dir(&peaks).map(|d| d.count()).unwrap_or(0);
        assert!(caches() <= 1, "only the original's overview may remain");
        // The same file comes back in place: nothing changes in the document.
        assert_eq!(media::relink(&dir, &d.project, &m, &src.join("Kick.wav")).unwrap(), None);
        assert!(package::missing_media(&dir, &d.project).is_empty());
        // A different take of the same length: a new media for the clips, one undoable step.
        fs::remove_file(dir.join(&m.path)).ok();
        wav::write_pcm(&src.join("Kick 2.wav"), 1, 48000, 24, &sine(4800, 1, 220.0, 48000.0), false, false).unwrap();
        let new = media::relink(&dir, &d.project, &m, &src.join("Kick 2.wav")).unwrap().unwrap();
        d.transact(
            Phase::Mix,
            "Relink",
            vec![
                crate::Command::AddMedia { media: Box::new(new.clone()) },
                crate::Command::RelinkMedia { from: m.id.clone(), to: new.id.clone() },
            ],
            None,
        )
        .unwrap();
        assert_eq!(d.project.tracks.iter().find(|t| t.is_audio()).unwrap().clips[0].media, new.id);
        assert!(package::missing_media(&dir, &d.project).is_empty());
        d.undo(Phase::Mix).unwrap();
        assert_eq!(d.project.tracks.iter().find(|t| t.is_audio()).unwrap().clips[0].media, m.id);
        assert_eq!(d.project.media, vec![m.clone()]);
        // Redo, then a snapshot of the old state comes back: its clips name the old media, still listed, so they
        // are reported missing again, never silently orphaned.
        d.redo(Phase::Mix).unwrap();
        assert!(d.project.media.iter().any(|x| x.id == m.id) && d.project.media.iter().any(|x| x.id == new.id));
        let t = d.project.tracks.iter().position(|t| t.is_audio()).unwrap();
        d.project.tracks[t].clips[0].media = m.id.clone();
        assert_eq!(package::missing_media(&dir, &d.project), vec![m.id.clone()]);
        // A clip naming a media the project does not list is reported too.
        d.project.tracks[t].clips[0].media = "gone".into();
        assert_eq!(package::missing_media(&dir, &d.project), vec!["gone".to_string()]);
    }

    /// req: PRJ-3, PRJ-4, PRJ-6
    #[test]
    fn imports_wav_variants_as_tracks_without_touching_the_originals() {
        let t = tmp();
        let src = t.path().join("src");
        fs::create_dir_all(&src).unwrap();
        let s = sine(4800, 1, 440.0, 48000.0);
        let st = sine(4800, 2, 440.0, 48000.0);
        wav::write_pcm(&src.join("Kick In.wav"), 1, 48000, 16, &s, false, false).unwrap();
        wav::write_pcm(&src.join("Lead Vox.wav"), 1, 48000, 24, &s, false, true).unwrap();
        wav::write_pcm(&src.join("OH.wav"), 2, 48000, 24, &st, true, false).unwrap();
        wav::write_f32(&src.join("Bass DI.wav"), 1, 48000, &s).unwrap();
        let originals: Vec<_> = fs::read_dir(&src).unwrap().map(|e| e.unwrap().path()).collect();
        let before: Vec<Vec<u8>> = originals.iter().map(|p| fs::read(p).unwrap()).collect();

        let (dir, p) = package::create(t.path(), "Imp").unwrap();
        let mut d = Document::new(p);
        let mut paths = originals.clone();
        paths.sort();
        let cands = candidates(&d.project, &paths);
        assert!(cands.iter().all(|c| c.error.is_none() && !c.rate_mismatch), "{cands:?}");
        let roles: Vec<_> = cands.iter().map(|c| (c.name.clone(), c.role)).collect();
        assert!(roles.contains(&("Kick In".into(), Role::Kick)));
        assert!(roles.contains(&("Lead Vox".into(), Role::LeadVocal)));
        let choices: Vec<_> = cands
            .iter()
            .map(|c| ImportChoice { path: c.path.clone(), role: c.role, name: c.name.clone(), convert: false })
            .collect();
        let cmds = import(&dir, &mut d.project, &choices).unwrap();
        d.transact(Phase::Mix, "Import 4 files", cmds, None).unwrap();

        assert_eq!(d.project.sample_rate, Some(48000));
        let audio: Vec<_> = d.project.tracks.iter().filter(|t| t.is_audio()).collect();
        assert_eq!(audio.len(), 4);
        assert!(audio.iter().all(|t| t.clips.len() == 1 && t.clips[0].start == 0 && t.clips[0].length == 4800));
        // Returns stay at the bottom.
        assert!(d.project.tracks.last().unwrap().is_return());
        for m in &d.project.media {
            let stored = dir.join(&m.path);
            assert!(stored.starts_with(dir.join("audio")));
            assert!(fs::metadata(&stored).unwrap().permissions().readonly());
            let mut src = PcmSource::open(&dir, m).unwrap();
            let mut buf = vec![0f32; 4800 * usize::from(m.channels)];
            src.read(0, &mut buf).unwrap();
            // 16-bit quantisation is the coarsest: within 1 LSB.
            assert!((buf[usize::from(m.channels) * 10] - s[10]).abs() < 1.0 / 32768.0 + 1e-6, "{}", m.encoding);
            assert!(Peaks::load(&dir, &m.id).is_some());
        }
        // The originals are never modified (PRJ-3).
        let after: Vec<Vec<u8>> = originals.iter().map(|p| fs::read(p).unwrap()).collect();
        assert_eq!(before, after);
    }

    fn write_aiff(path: &Path, samples: &[f32], channels: u16, rate: u32) {
        // 80-bit IEEE extended sample rate.
        let mut ext = [0u8; 10];
        let exp = 31 - rate.leading_zeros();
        ext[0..2].copy_from_slice(&((16383 + exp) as u16).to_be_bytes());
        ext[2..10].copy_from_slice(&((u64::from(rate)) << (63 - exp)).to_be_bytes());
        let frames = (samples.len() / usize::from(channels)) as u32;
        let data: Vec<u8> = samples.iter().flat_map(|s| (((s * 32767.0).round()) as i16).to_be_bytes()).collect();
        let mut b = Vec::new();
        b.extend_from_slice(b"FORM");
        b.extend_from_slice(&((4 + 26 + 16 + data.len()) as u32).to_be_bytes());
        b.extend_from_slice(b"AIFFCOMM");
        b.extend_from_slice(&18u32.to_be_bytes());
        b.extend_from_slice(&channels.to_be_bytes());
        b.extend_from_slice(&frames.to_be_bytes());
        b.extend_from_slice(&16u16.to_be_bytes());
        b.extend_from_slice(&ext);
        b.extend_from_slice(b"SSND");
        b.extend_from_slice(&((8 + data.len()) as u32).to_be_bytes());
        b.extend_from_slice(&[0u8; 8]);
        b.extend_from_slice(&data);
        fs::write(path, b).unwrap();
    }

    /// req: PRJ-4
    #[test]
    fn imports_aiff_through_the_decoded_cache() {
        let t = tmp();
        let src = t.path().join("src");
        fs::create_dir_all(&src).unwrap();
        let st = sine(96000, 2, 330.0, 48000.0);
        write_aiff(&src.join("Strings.aiff"), &st, 2, 48000);
        let (dir, mut p) = package::create(t.path(), "Fmt").unwrap();
        let paths = vec![src.join("Strings.aiff")];
        let c = candidates(&p, &paths);
        assert!(c.iter().all(|c| c.error.is_none() && c.channels == 2 && c.frames == 96000), "{c:?}");
        assert_eq!(c[0].role, Role::Strings);
        let choices: Vec<_> = c
            .iter()
            .map(|c| ImportChoice { path: c.path.clone(), role: c.role, name: c.name.clone(), convert: false })
            .collect();
        let cmds = import(&dir, &mut p, &choices).unwrap();
        let mut d = Document::new(p);
        d.transact(Phase::Mix, "import", cmds, None).unwrap();
        for m in &d.project.media {
            assert!(media::pcm_cache_path(&dir, &m.id).exists());
            let mut s = PcmSource::open(&dir, m).unwrap();
            let mut buf = vec![0f32; 96000 * 2];
            s.read(0, &mut buf).unwrap();
            let err = buf.iter().zip(&st).fold(0f32, |a, (x, y)| a.max((x - y).abs()));
            assert!(err < 2.0 / 32768.0, "{:?}: {err}", m.format);
        }
        // The caches are rebuildable: delete them and read again.
        fs::remove_dir_all(dir.join("cache")).unwrap();
        let m = &d.project.media[0];
        let mut s = PcmSource::open(&dir, m).unwrap();
        assert_eq!(s.frames(), 96000);
        let mut one = [0f32; 2];
        s.read(100, &mut one).unwrap();
        // An MP3 is refused as a stem.
        fs::write(src.join("x.mp3"), [0u8; 10]).unwrap();
        assert!(candidates(&d.project, &[src.join("x.mp3")])[0].error.is_some());
    }

    /// req: PRJ-5
    #[test]
    fn a_different_rate_is_converted_only_when_the_user_accepts() {
        let t = tmp();
        let src = t.path().join("src");
        fs::create_dir_all(&src).unwrap();
        wav::write_f32(&src.join("a.wav"), 1, 48000, &sine(48000, 1, 1000.0, 48000.0)).unwrap();
        wav::write_f32(&src.join("b.wav"), 1, 44100, &sine(44100, 1, 1000.0, 44100.0)).unwrap();
        let (dir, mut p) = package::create(t.path(), "Rates").unwrap();
        let paths = vec![src.join("a.wav"), src.join("b.wav")];
        let c = candidates(&p, &paths);
        assert!(!c[0].rate_mismatch && c[1].rate_mismatch);
        let choice = |path: &PathBuf, convert| ImportChoice {
            path: path.display().to_string(),
            role: Role::Other,
            name: "x".into(),
            convert,
        };
        let cmds = import(&dir, &mut p, &[choice(&paths[0], false), choice(&paths[1], false)]).unwrap();
        assert_eq!(cmds.iter().filter(|c| matches!(c, Command::InsertTrack { .. })).count(), 1);
        let cmds = import(&dir, &mut p, &[choice(&paths[0], false), choice(&paths[1], true)]).unwrap();
        let mut d = Document::new(p);
        d.transact(Phase::Mix, "import", cmds, None).unwrap();
        let conv = d.project.media.iter().find(|m| m.converted).expect("converted copy");
        assert_eq!((conv.sample_rate, conv.frames), (48000, 48000));
        // The converted sine keeps its level: amplitude 0.5 within 0.05 dB in the steady part.
        let mut src = PcmSource::open(&dir, conv).unwrap();
        let mut buf = vec![0f32; 48000];
        src.read(0, &mut buf).unwrap();
        let peak = buf[4800..43200].iter().fold(0f32, |a, &b| a.max(b.abs()));
        assert!((20.0 * (peak / 0.5).log10()).abs() < 0.05, "{peak}");
    }

    /// req: TRN-3
    #[test]
    fn peak_pyramid_queries_match_the_samples() {
        let n = 100_000;
        let s: Vec<f32> = (0..n).map(|i| if i == 54_321 { 0.9 } else { 0.1 * ((i as f32) * 0.01).sin() }).collect();
        let p = Peaks::build(&s, 1);
        assert!(p.levels.len() > 3);
        let back = Peaks::from_bytes(&p.to_bytes()).unwrap();
        assert_eq!(back, p);
        for cols in [10, 100, 1000] {
            let q = p.query(0, n as u64, cols);
            let max = q[0].iter().fold(0f32, |a, c| a.max(c[1]));
            assert!((max - 0.9).abs() < 1e-3, "{cols}: {max}");
        }
        let zoomed = p.query(54_000, 55_000, 4);
        assert!(zoomed[0].iter().any(|c| c[1] > 0.89));
    }

    /// req: PRJ-7
    #[test]
    fn up_to_three_references_are_copied_into_the_package() {
        let t = tmp();
        let src = t.path().join("r.wav");
        wav::write_f32(&src, 2, 44100, &sine(44100, 2, 300.0, 44100.0)).unwrap();
        let (dir, mut p) = package::create(t.path(), "Refs").unwrap();
        for _ in 0..3 {
            let c = add_reference(&dir, &p, &src).unwrap();
            c.apply(&mut p).unwrap();
        }
        assert_eq!(p.references.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["ref1", "ref2", "ref3"]);
        assert!(add_reference(&dir, &p, &src).is_err());
        assert!(dir.join(&p.references[0].media.path).starts_with(dir.join("references")));
    }
}
