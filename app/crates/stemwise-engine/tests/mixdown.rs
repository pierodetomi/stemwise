//! Bounce to master (MDN-1, MDN-4): the mixdown file, its attenuation-only trim to −3 dBTP, its measures and the mix
//! hash that tells when it is out of date (MDN-2).

use std::path::Path;

use stemwise_core::command::Command;
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::{package, role::Role, wav};
use stemwise_engine::mixdown::{MIXDOWN_CEILING_DB, bounce, is_stale, listen_project};

const RATE: u32 = 48000;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

fn project(t: &Path, amp: f32) -> (std::path::PathBuf, Document) {
    let src = t.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (dir, p) = package::create(t, "Bounce").unwrap();
    let mut d = Document::new(p);
    let n = 3 * RATE as usize;
    // Stereo: a balance at the centre does not attenuate (a mono track would lose 3 dB in the pan law).
    let x: Vec<f32> = (0..n).flat_map(|i| [amp * (i as f32 * 0.05).sin(), amp * (i as f32 * 0.05).sin()]).collect();
    let path = src.join("a.wav");
    wav::write_f32(&path, 2, RATE, &x).unwrap();
    let choice = ImportChoice { path: path.display().to_string(), role: Role::Other, name: "a".into(), convert: false };
    let cmds = import(&dir, &mut d.project, &[choice]).unwrap();
    d.transact(Phase::Mix, "import", cmds, None).unwrap();
    (dir, d)
}

/// req: MDN-1, MDN-2, MDN-4
#[test]
fn a_bounce_records_its_mix_and_trims_only_downwards() {
    let t = tmp();
    // A loud mix: trimmed down to the ceiling.
    let (dir, mut d) = project(t.path(), 0.99);
    let (md, x) = bounce(&dir, &d.project, &mut |_| true).unwrap();
    assert_eq!(md.number, 1);
    assert!(dir.join(&md.path).exists());
    assert!(md.trim_db < 0.0 && md.true_peak_db <= MIXDOWN_CEILING_DB + 1e-6, "{md:?}");
    assert_eq!(md.mix_hash, package::mix_hash(&d.project));
    assert_eq!(x.len() as u64, 2 * md.frames);
    // Recorded in the project, it is up to date; a mix change makes it stale (MDN-2).
    d.apply_view(vec![Command::SetMixdown { mixdown: Some(Box::new(md.clone())) }]).unwrap();
    assert!(!is_stale(&d.project));
    // What does not change the sound (a name, a tint, a role) leaves it up to date.
    for t in &mut d.project.tracks {
        t.name = format!("{} renamed", t.name);
        t.color = 7;
        t.role = Some(Role::Keys);
    }
    assert!(!is_stale(&d.project));
    let id = d.project.tracks.iter().find(|t| t.is_audio()).unwrap().id.clone();
    d.transact(Phase::Mix, "f", vec![Command::SetParam { track: id, param: "fader".into(), value: -6.0 }], None)
        .unwrap();
    assert!(is_stale(&d.project));
    // The next bounce gets the next number; the listening project plays the file.
    let (md2, _) = bounce(&dir, &d.project, &mut |_| true).unwrap();
    assert_eq!(md2.number, 2);
    let lp = listen_project(&d.project, &md2.path, md2.frames, RATE, 0.0);
    assert_eq!((lp.tracks.len(), lp.media[0].path.as_str()), (1, md2.path.as_str()));
    // A quiet mix is not raised.
    let t2 = tmp();
    let (dir, d) = project(t2.path(), 0.1);
    let (md, _) = bounce(&dir, &d.project, &mut |_| true).unwrap();
    assert_eq!(md.trim_db, 0.0);
    // Cancelled: an error, no record.
    assert!(bounce(&dir, &d.project, &mut |_| false).is_err());
}
