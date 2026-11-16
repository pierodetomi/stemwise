//! ANA-17: analysis of 16 stems of 4 minutes in less than 30 s (release, Apple Silicon). Exits non-zero on failure.

use std::path::Path;
use std::time::Instant;

use stemwise_ai::analysis_doc::analyze_project;
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::{package, role::Role, wav};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    let tmp = tempfile::tempdir_in(&root).unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let rate = 48000u32;
    let n = 240 * rate as usize;
    let (dir, p) = package::create(tmp.path(), "Bench").unwrap();
    let mut d = Document::new(p);
    let mut choices = Vec::new();
    for k in 0..16u32 {
        let x: Vec<f32> = stemwise_dsp::testsignal::xorshift32(k + 1, n)
            .iter()
            .enumerate()
            .map(|(i, v)| (v * 0.2 * if (i / 24000) % 3 == 0 { 0.1 } else { 1.0 }) as f32)
            .collect();
        let path = src.join(format!("s{k}.wav"));
        wav::write_f32(&path, 1, rate, &x).unwrap();
        choices.push(ImportChoice {
            path: path.display().to_string(),
            role: Role::Other,
            name: format!("S{k}"),
            convert: false,
        });
    }
    let cmds = import(&dir, &mut d.project, &choices).unwrap();
    d.transact(Phase::Mix, "import", cmds, None).unwrap();
    std::fs::remove_dir_all(&src).unwrap();
    let t = Instant::now();
    analyze_project(&dir, &d.project, &|_| true).unwrap();
    let secs = t.elapsed().as_secs_f64();
    println!("ANA-17: 16 stems × 4 min analysed in {secs:.1} s (limit 30 s)");
    if secs > 30.0 {
        println!("ANA-17: FAILED");
        std::process::exit(1);
    }
}
