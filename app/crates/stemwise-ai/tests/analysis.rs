//! Project analysis (ANA-13 … ANA-17) on synthetic packages.

use std::path::{Path, PathBuf};

use stemwise_ai::analysis_doc::{Ids, analyze_project, estimated_tokens, model_document, track_key};
use stemwise_core::command::Command;
use stemwise_core::edit;
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::{package, role::Role, wav};

const RATE: u32 = 48000;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

fn project(t: &Path, names: &[(&str, Role)], secs: f64) -> (PathBuf, Document) {
    let src = t.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (dir, p) = package::create(t, "Secret Song Title").unwrap();
    let mut d = Document::new(p);
    let n = (secs * f64::from(RATE)) as usize;
    let mut choices = Vec::new();
    for (k, (name, role)) in names.iter().enumerate() {
        let f = 60.0 * (k + 1) as f64;
        let x: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f64 / f64::from(RATE);
                (0.3 * (std::f64::consts::TAU * f * t).sin() * if (t * 2.0) % 1.0 < 0.7 { 1.0 } else { 0.2 }) as f32
            })
            .collect();
        let path = src.join(format!("private-file-{k}.wav"));
        wav::write_f32(&path, 1, RATE, &x).unwrap();
        choices.push(ImportChoice {
            path: path.display().to_string(),
            role: *role,
            name: name.to_string(),
            convert: false,
        });
    }
    let cmds = import(&dir, &mut d.project, &choices).unwrap();
    d.transact(Phase::Mix, "import", cmds, None).unwrap();
    (dir, d)
}

/// req: AI-6, ANA-14
#[test]
fn sections_past_two_minutes_keep_their_real_times() {
    // A-063: seconds went through the rounding of levels, which stops at 120: every section of a longer song past
    // that point reached the model with zero length.
    let t = tmp();
    let (dir, mut d) = project(t.path(), &[("Vox", Role::LeadVocal)], 130.0);
    for (name, a, b) in [("Verse", 0, 121), ("Solo", 121, 130)] {
        let cmds = edit::add_section(&mut d.project, name, a * i64::from(RATE), b * i64::from(RATE)).unwrap();
        d.transact(Phase::Mix, "s", cmds, None).unwrap();
    }
    let a = analyze_project(&dir, &d.project, &|_| true).unwrap();
    let doc = model_document(&a, &d.project, &Ids::of(&d.project));
    assert_eq!(doc["project"]["duration_s"], 130.0);
    assert_eq!(doc["sections"][1]["start_s"], 121.0);
    assert_eq!(doc["sections"][1]["end_s"], 130.0);
    // Levels keep their limit.
    assert_eq!(stemwise_ai::analysis_doc::r1(-300.0), -120.0);
}

/// req: ANA-13, ANA-15, ANA-16, AI-10
#[test]
fn document_stays_under_the_token_budget_and_carries_no_private_strings() {
    let t = tmp();
    let roles = [
        Role::Kick,
        Role::Snare,
        Role::Toms,
        Role::Overheads,
        Role::Room,
        Role::Bass,
        Role::ElectricGuitar,
        Role::AcousticGuitar,
    ];
    let names: Vec<(String, Role)> =
        (0..24).map(|k| (format!("Track {k:02} with a long descriptive name"), roles[k % roles.len()])).collect();
    let refs: Vec<(&str, Role)> = names.iter().map(|(n, r)| (n.as_str(), *r)).collect();
    let (dir, mut d) = project(t.path(), &refs, 8.0);
    for k in 0..8 {
        let cmds =
            edit::add_section(&mut d.project, &format!("Section {k}"), k * RATE as i64, (k + 1) * RATE as i64).unwrap();
        d.transact(Phase::Mix, "s", cmds, None).unwrap();
    }
    let ids: Vec<String> = d.project.tracks.iter().filter(|t| t.is_audio()).map(|t| t.id.clone()).take(4).collect();
    // A group named after the song: its family name must not carry the title to the model (AI-10, A-036).
    let cmds = edit::add_group(&mut d.project, "Secret Song Title drum bus", &ids).unwrap();
    d.transact(Phase::Mix, "g", cmds, None).unwrap();
    let a = analyze_project(&dir, &d.project, &|_| true).unwrap();
    let doc = model_document(&a, &d.project, &Ids::of(&d.project));
    let text = doc.to_string();
    let tokens = estimated_tokens(&doc);
    assert!(tokens < 12_000, "{tokens} tokens");
    for private in ["private-file", "Secret Song Title", dir.to_str().unwrap(), ".wav"] {
        assert!(!text.contains(private), "the document leaks {private}");
    }
    assert_eq!(doc["sections"].as_array().unwrap().len(), 8);
    assert_eq!(doc["tracks"]["t1"]["section_lufs"].as_array().unwrap().len(), 8);
    assert_eq!(doc["mix"]["section_bands_db"].as_array().unwrap().len(), 8);
    let fams = doc["families"].as_array().unwrap();
    assert!(fams.iter().any(|f| f["name"] == "drums"));
    let bus = stemwise_ai::analysis_doc::model_name("Secret Song Title drum bus", &d.project.name);
    assert!(!bus.contains("Secret") && bus.contains("drum bus"), "{bus}");
    assert!(fams.iter().any(|f| f["name"] == bus.as_str()));
    // Masking inside a family is not among the main collisions (ANA-16).
    let drum_ids: Vec<String> = fams.iter().find(|f| f["name"] == "drums").unwrap()["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    for c in doc["masking_top"].as_array().unwrap() {
        let (x, y) = (c["a"].as_str().unwrap(), c["b"].as_str().unwrap());
        assert!(!(drum_ids.iter().any(|m| m == x) && drum_ids.iter().any(|m| m == y)), "{c}");
    }
}

/// req: ANA-14, ANA-17
#[test]
fn cache_follows_clip_edits_not_parameters_and_progress_is_reported() {
    let t = tmp();
    let (dir, mut d) = project(t.path(), &[("Kick", Role::Kick), ("Bass", Role::Bass), ("Vox", Role::LeadVocal)], 4.0);
    let calls = std::sync::Mutex::new(Vec::new());
    let a1 = analyze_project(&dir, &d.project, &|f| {
        calls.lock().unwrap().push(f);
        true
    })
    .unwrap();
    let c = calls.into_inner().unwrap();
    assert!(c.len() >= 4 && c.windows(2).all(|w| w[1] >= w[0]) && *c.last().unwrap() == 1.0, "{c:?}");
    let vox = d.project.tracks.iter().find(|t| t.name == "Vox").unwrap().id.clone();
    let key = |d: &Document| track_key(&d.project, d.project.track(&vox).unwrap());
    let k0 = key(&d);
    assert!(dir.join("cache/analysis").join(format!("{k0}.json")).exists());
    // A parameter change keeps the cache key (measures are taken before the chain).
    d.transact(
        Phase::Mix,
        "f",
        vec![Command::SetParam { track: vox.clone(), param: "fader".into(), value: -12.0 }],
        None,
    )
    .unwrap();
    assert_eq!(key(&d), k0);
    // A clip edit invalidates it, and the measures change accordingly.
    let clip = d.project.track(&vox).unwrap().clips[0].id.clone();
    let cmds = edit::trim_clip(&d.project, &(vox.clone(), clip), false, i64::from(RATE)).unwrap();
    d.transact(Phase::Mix, "trim", cmds, None).unwrap();
    assert_ne!(key(&d), k0);
    let a2 = analyze_project(&dir, &d.project, &|_| true).unwrap();
    let act = |a: &stemwise_ai::analysis_doc::ProjectAnalysis| {
        a.tracks.iter().find(|x| x.id == vox).unwrap().measures.active_pct
    };
    assert!(act(&a2) < act(&a1) - 30.0, "{} vs {}", act(&a2), act(&a1));
    // Cancelling stops the analysis.
    assert!(analyze_project(&dir, &d.project, &|_| false).is_err());
}

/// req: UI-6, ANA-14
#[test]
fn a_missing_file_is_never_measured_as_silence() {
    let t = tmp();
    let (dir, d) = project(t.path(), &[("Kick", Role::Kick), ("Bass", Role::Bass)], 3.0);
    let m = d.project.media[0].clone();
    let stored = dir.join(&m.path);
    let mut perm = std::fs::metadata(&stored).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(false);
    std::fs::set_permissions(&stored, perm).unwrap();
    std::fs::remove_file(&stored).unwrap();
    let e = analyze_project(&dir, &d.project, &|_| true).unwrap_err();
    assert!(matches!(e, stemwise_ai::analysis_doc::AnalysisError::Missing(1)), "{e}");
    // Nothing cached for a later run to reuse.
    let cached = std::fs::read_dir(dir.join("cache").join("analysis")).map(|d| d.count()).unwrap_or(0);
    assert_eq!(cached, 0);
}
