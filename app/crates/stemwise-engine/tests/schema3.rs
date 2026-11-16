//! Projects of schema 3 (`app/fixtures/schema3`, written by the schema-3 build with their render hashes) render
//! with the same hash once migrated to the chain of blocks (FX-10). The signals are rebuilt here; the project files
//! keep the edits of that build.

use std::path::{Path, PathBuf};

use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::package;
use stemwise_core::role::Role;
use stemwise_core::wav;
use stemwise_engine::offline::render_range;

const RATE: u32 = 48000;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

fn sine(n: usize, freq: f64, amp: f64) -> Vec<f32> {
    (0..n).map(|i| (amp * (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(RATE)).sin()) as f32).collect()
}

fn noise(n: usize, seed: u32, amp: f64) -> Vec<f32> {
    stemwise_dsp::testsignal::xorshift32(seed, n).iter().map(|v| (v * amp) as f32).collect()
}

fn signals() -> Vec<(&'static str, u16, Vec<f32>)> {
    let n = 2 * RATE as usize;
    let st: Vec<f32> = noise(n, 7, 0.2).iter().zip(noise(n, 8, 0.2)).flat_map(|(a, b)| [*a, b]).collect();
    // Bursts, so compressors and envelopes move.
    let kick: Vec<f32> =
        noise(n, 1, 0.6).iter().enumerate().map(|(i, v)| v * (-((i % 12000) as f32) / 1500.0).exp()).collect();
    vec![("Kick", 1, kick), ("Bass", 1, sine(n, 55.0, 0.4)), ("Pad", 2, st), ("Vox", 1, sine(n, 440.0, 0.3))]
}

fn imported(dir_parent: &Path) -> (PathBuf, Document) {
    let src = dir_parent.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (dir, p) = package::create(dir_parent, "Schema3").unwrap();
    let mut doc = Document::new(p);
    let mut choices = Vec::new();
    for (name, ch, s) in signals() {
        let path = src.join(format!("{name}.wav"));
        wav::write_f32(&path, ch, RATE, &s).unwrap();
        choices.push(ImportChoice {
            path: path.display().to_string(),
            role: Role::Other,
            name: name.into(),
            convert: false,
        });
    }
    let cmds = import(&dir, &mut doc.project, &choices).unwrap();
    doc.transact(Phase::Mix, "import", cmds, None).unwrap();
    (dir, doc)
}

/// req: FX-10, PRJ-8, ENG-4
#[test]
fn schema3_projects_render_with_the_same_hash() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/schema3");
    let hashes: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("hashes.json")).unwrap()).unwrap();
    for which in ["rich", "nested"] {
        let t = tmp();
        let (pkg, _) = imported(t.path());
        let text = std::fs::read_to_string(dir.join(format!("{which}.json"))).unwrap();
        let project = package::parse(&text).unwrap();
        assert_eq!(project.schema_version, stemwise_core::model::SCHEMA_VERSION);
        let end = 2 * i64::from(RATE) + 24_000;
        for block in [4096, 64, 777] {
            let r = render_range(&pkg, &project, 0, end, block, &mut |_| true).unwrap();
            assert_eq!(r.hash(), hashes[which].as_str().unwrap(), "{which}, blocks of {block}");
        }
    }
}
