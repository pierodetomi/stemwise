//! End-to-end flows through the `stemwise-cli` binary (ENG-7).

use std::path::Path;
use std::process::Command;

fn cli(args: &[&str]) -> String {
    cli_in(None, args)
}

/// Runs the CLI with its data folder (the cross-project cost register) inside the test's temporary folder.
fn cli_in(data: Option<&Path>, args: &[&str]) -> String {
    let mut c = Command::new(env!("CARGO_BIN_EXE_stemwise-cli"));
    if let Some(d) = data {
        c.env("STEMWISE_DATA", d);
    }
    let out = c.args(args).output().expect("run cli");
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/// req: ENG-7, ENG-4
#[test]
fn demo_project_renders_with_a_stable_hash() {
    let t = tmp();
    let dir = cli(&["demo", t.path().to_str().unwrap(), "--name", "Cli", "--seconds", "4"]);
    let dir = dir.trim();
    assert!(dir.ends_with("Cli.stemwise"));
    let info = cli(&["info", dir]);
    assert!(info.contains("Lead Vox") && info.contains("Reverb"), "{info}");
    let a = cli(&["render", dir]);
    let b = cli(&["render", dir]);
    assert_eq!(a, b, "same project, same render hash");
    assert!(a.contains("hash "));
}

/// req: ENG-7, AI-1, AI-3, AI-14, AI-17, AI-21, ANA-18, MAS-6, AI-25, AI-26, AI-27, UI-13
#[test]
fn offline_ai_flow_from_the_cli() {
    let t = tmp();
    let dir = cli(&["demo", t.path().to_str().unwrap(), "--name", "Ai", "--seconds", "6"]);
    let dir = dir.trim();
    let doc = cli(&["analyze", dir]);
    assert!(doc.contains("\"version\": 4") && doc.contains("masking_top"));
    // ANA-18: the guitar part played twice is found as a double, two performances and not one signal.
    let json = &doc[doc.find('{').unwrap()..];
    let analysis: serde_json::Value = serde_json::from_str(json).unwrap();
    let doubles = analysis["doubles"].as_array().unwrap();
    assert!(doubles.len() == 1 && doubles[0]["identical"] == false, "{doubles:?}");
    let data = t.path().join("data");
    let data = Some(data.as_path());
    let out = cli_in(data, &["automix", dir, "--iterations", "2", "--apply"]);
    assert!(out.contains("summary:") && out.contains("applied"), "{out}");
    // AI-17, AI-21: the fake decider opens the doubled guitar, gives the vocal a plate and a slapback and the
    // kit a bus; they are in the project after the application.
    for line in ["new return: Vocal plate", "new return: Slap", "new group: Drums", "Send · Vocal plate"] {
        assert!(out.contains(line), "{line} in:\n{out}");
    }
    let info = cli(&["info", dir]);
    assert!(info.contains("Vocal plate") && info.contains("Slap") && info.contains("Group    Drums"), "{info}");
    // Every technique of AI-21, read back from the saved project.
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(Path::new(dir).join("project.json")).unwrap()).unwrap();
    let tracks = saved["tracks"].as_array().unwrap();
    let by_name = |n: &str| tracks.iter().find(|t| t["name"] == n).unwrap_or_else(|| panic!("no track {n}"));
    let (left, right) = (by_name("Gtr Left"), by_name("Gtr Right"));
    // True doubles: opposite pans, one level, the same processing.
    assert_eq!(left["chain"]["pan"].as_f64().unwrap(), -right["chain"]["pan"].as_f64().unwrap());
    assert!(left["chain"]["pan"].as_f64().unwrap().abs() >= 0.6);
    assert_eq!(left["chain"]["fader_db"], right["chain"]["fader_db"]);
    assert_eq!(left["chain"]["blocks"], right["chain"]["blocks"]);
    // Parallel compression on the new drum bus, with the kit routed into it.
    let drums = by_name("Drums");
    let block = |t: &serde_json::Value, id: &str| {
        t["chain"]["blocks"].as_array().unwrap().iter().find(|b| b["id"] == id).unwrap()["fx"].clone()
    };
    let comp = block(drums, "comp");
    assert!(comp["bypass"] == false && comp["mix"].as_f64().unwrap() < 0.5);
    assert_eq!(by_name("Kick")["output"]["id"], drums["id"]);
    assert_eq!(by_name("Snare")["output"]["id"], drums["id"]);
    // Distinct spaces and slapback: the vocal sends to its plate and to a short delay without repeats.
    let vox = by_name("Lead Vox");
    let sends: Vec<&str> = vox["sends"].as_array().unwrap().iter().map(|s| s["target"].as_str().unwrap()).collect();
    let (plate, slap, delay) = (by_name("Vocal plate"), by_name("Slap"), by_name("Delay"));
    assert!(sends.contains(&plate["id"].as_str().unwrap()) && sends.contains(&slap["id"].as_str().unwrap()));
    assert!(!sends.contains(&by_name("Reverb")["id"].as_str().unwrap()), "the vocal left the shared reverb");
    assert_eq!(plate["ret"]["algorithm"], "plate");
    assert!(slap["ret"]["feedback"] == 0.0 && slap["ret"]["time_ms"].as_f64().unwrap() <= 140.0);
    // Delay throw: a lane the assistant wrote on the vocal's send to the delay.
    let lane = format!("send.{}", delay["id"].as_str().unwrap());
    assert!(vox["automation"].as_array().unwrap().iter().any(|l| l["param"] == lane.as_str() && l["author"] == "ai"));
    // AI-25 → AI-27: a second auto mix proposes ideas, off by default; `--ideas` ticks them, and they are applied
    // in the same step as the plan; one undo would take them back with it (HIS-3, tested in `ideas_flow`).
    let out = cli_in(data, &["automix", dir, "--iterations", "1"]);
    assert!(out.contains("idea i1 [ ]") && out.contains("Modulation (new)"), "{out}");
    let out = cli_in(data, &["automix", dir, "--iterations", "1", "--apply", "--ideas", "all"]);
    assert!(out.contains("idea i1 [x]") && out.contains("applied with"), "{out}");
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(Path::new(dir).join("project.json")).unwrap()).unwrap();
    let pad = saved["tracks"].as_array().unwrap().iter().find(|t| t["name"] == "Pad").unwrap().clone();
    assert!(pad["chain"]["blocks"].as_array().unwrap().iter().any(|b| b["fx"]["type"] == "modulation"), "{pad}");
    let notes = saved["ai"]["notes"].as_array().unwrap();
    assert!(
        notes.iter().any(|n| n["track"] == pad["id"] && n["text"].as_str().unwrap().contains("Idea “")),
        "{notes:?}"
    );
    let out = cli_in(data, &["revise", dir, "vocal louder", "--apply"]);
    assert!(out.contains("Revision") && out.contains("Lead Vox"), "{out}");
    let out = cli_in(data, &["master-ai", dir, "--apply"]);
    assert!(out.contains("Master") && out.contains("applied"), "{out}");
    let usage = cli(&["ai-usage", dir]);
    assert!(usage.starts_with("5 calls"), "{usage}");
    let csv = cli(&["ai-usage", dir, "--csv"]);
    assert_eq!(csv.lines().count(), 6);
    assert!(csv.contains("offline-demo") && csv.contains("applied"));
    // The register of all projects has the five calls (AI-14).
    assert_eq!(stemwise_ai::log::total_cost(&t.path().join("data")).1, 5);
}
