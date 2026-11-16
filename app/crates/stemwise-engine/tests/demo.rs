//! The demo project of the tour (PRJ-9) renders the same, sample for sample, every time it is built.

use std::path::Path;

use stemwise_core::demo;
use stemwise_engine::offline::render_song;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/// req: PRJ-9, ENG-4
#[test]
fn two_builds_of_the_demo_render_with_the_same_hash() {
    let (a, b) = (tmp(), tmp());
    let (da, pa) = demo::build(a.path()).unwrap();
    let (db, pb) = demo::build(b.path()).unwrap();
    let (ra, rb) = (render_song(&da, &pa, 1.0).unwrap(), render_song(&db, &pb, 1.0).unwrap());
    assert_eq!(ra.hash(), rb.hash());
    // It is a song, not silence, and it does not clip.
    let peak = ra.left.iter().chain(&ra.right).fold(0f32, |m, v| m.max(v.abs()));
    assert!(peak > 0.1 && peak < 1.0, "peak {peak}");
}
