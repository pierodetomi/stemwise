//! Creative ideas and blocks in the mix AI (F17 gate): the new measures in the analysis, blocks in the plan, ideas
//! decided once and kept out of the evaluation, applied only when ticked and undone in one step with the rest.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use serde_json::Value;
use stemwise_ai::decider::{DecideError, Decider, DecisionInput, FakeDecider, Message, RawDecision, Retry, Usage};
use stemwise_ai::dto::ProposalView;
use stemwise_ai::mixing::{MixOptions, MixProposal, apply_commands, run_mix};
use stemwise_core::command::Command;
use stemwise_core::edit;
use stemwise_core::fx::{Fx, FxKind};
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, add_reference, import};
use stemwise_core::model::{DelayMode, ReturnFx};
use stemwise_core::{package, role::Role, routing, wav};

const RATE: u32 = 48000;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/// A song with drums, keys, strings and a sibilant vocal; four sections and a reference.
fn song(t: &Path) -> (PathBuf, Document) {
    let src = t.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (dir, p) = package::create(t, "Ideas").unwrap();
    let mut d = Document::new(p);
    let n = 8 * RATE as usize;
    let tau = std::f64::consts::TAU;
    let synth = |f: &dyn Fn(f64) -> f64| -> Vec<f32> { (0..n).map(|i| f(i as f64 / f64::from(RATE)) as f32).collect() };
    let noise = stemwise_dsp::testsignal::xorshift32(31, n);
    let stems: Vec<(&str, Role, Vec<f32>)> = vec![
        ("Kick", Role::Kick, synth(&|t| 0.8 * (-(t % 0.5) * 20.0).exp() * (tau * 55.0 * t).sin())),
        ("Bass", Role::Bass, synth(&|t| 0.4 * (tau * 55.0 * t).sin())),
        ("Keys", Role::Keys, synth(&|t| 0.2 * ((tau * 261.6 * t).sin() + (tau * 329.6 * t).sin()))),
        ("Strings", Role::Strings, synth(&|t| 0.2 * (tau * 392.0 * t).sin())),
        (
            "Vox",
            Role::LeadVocal,
            // A voice at 220 Hz with a hard "s" at 7 kHz every 0.5 s (ANA-19).
            (0..n)
                .map(|i| {
                    let t = i as f64 / f64::from(RATE);
                    let ess = if t % 0.5 < 0.06 { 0.5 * (tau * 7000.0 * t).sin() + 0.1 * noise[i] } else { 0.0 };
                    (0.25 * (tau * 220.0 * t).sin() + ess) as f32
                })
                .collect(),
        ),
    ];
    let mut choices = Vec::new();
    for (name, role, x) in &stems {
        let path = src.join(format!("{name}.wav"));
        wav::write_f32(&path, 1, RATE, x).unwrap();
        choices.push(ImportChoice {
            path: path.display().to_string(),
            role: *role,
            name: name.to_string(),
            convert: false,
        });
    }
    let cmds = import(&dir, &mut d.project, &choices).unwrap();
    d.transact(Phase::Mix, "import", cmds, None).unwrap();
    for (name, a, b) in [("Verse", 0, 2), ("Chorus", 2, 4), ("Bridge", 4, 6), ("Chorus", 6, 8)] {
        let cmds = edit::add_section(&mut d.project, name, a * i64::from(RATE), b * i64::from(RATE)).unwrap();
        d.transact(Phase::Mix, "s", cmds, None).unwrap();
    }
    let rpath = src.join("reference.wav");
    let refx: Vec<f32> = stemwise_dsp::testsignal::xorshift32(5, 2 * n).iter().map(|v| (v * 0.2) as f32).collect();
    wav::write_f32(&rpath, 2, RATE, &refx).unwrap();
    let c = add_reference(&dir, &d.project, &rpath).unwrap();
    d.transact(Phase::Mix, "ref", vec![c], None).unwrap();
    (dir, d)
}

fn id(d: &Document, name: &str) -> String {
    d.project.tracks.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("no track {name}")).id.clone()
}

/// What the fake decider was asked, per call.
struct Seen {
    iteration: u32,
    allow_ideas: bool,
    schema: Value,
    analysis: Value,
    system: String,
    plan: Value,
}

struct Spy {
    seen: Mutex<Vec<Seen>>,
}

impl Decider for Spy {
    fn name(&self) -> &str {
        "spy"
    }
    fn model(&self) -> String {
        "test/model".into()
    }
    fn effort(&self) -> String {
        "low".into()
    }
    fn decide(
        &self,
        input: &DecisionInput,
        m: &[Message],
        schema: &Value,
        _r: Option<&Retry>,
        _c: &AtomicBool,
    ) -> Result<RawDecision, DecideError> {
        let plan = serde_json::to_value(FakeDecider.mix_plan(input)).unwrap();
        self.seen.lock().unwrap().push(Seen {
            iteration: input.iteration,
            allow_ideas: input.allow_ideas,
            schema: schema.clone(),
            analysis: input.analysis.clone(),
            system: m[0].content.clone(),
            plan: plan.clone(),
        });
        Ok(RawDecision {
            content: plan.to_string(),
            usage: Usage { tokens_in: 1000, tokens_out: 200, cost_usd: None },
            model: "test/model".into(),
            raw: "{}".into(),
            request: "{}".into(),
        })
    }
}

fn mix(dir: &Path, d: &Document, decider: &dyn Decider, n: u32) -> MixProposal {
    let opts = MixOptions { max_iterations: n, ..Default::default() };
    run_mix(dir, &d.project, decider, &|_, _| 0.0, &opts, &|_| {}, &AtomicBool::new(false)).unwrap()
}

fn ticked(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

fn applied(d: &Document, p: &MixProposal, ideas: &[&str]) -> Document {
    let mut doc = Document::new(d.project.clone());
    let cmds = apply_commands(&d.project, p, None, &ticked(ideas));
    doc.transact(Phase::Mix, "Apply auto mix", cmds, Some(p.call_id.clone())).unwrap();
    routing::validate(&doc.project).unwrap();
    doc
}

/// req: ANA-19, AI-25, AI-26, AI-27, UI-13, HIS-3, AI-3
#[test]
fn ideas_are_decided_once_stay_out_of_the_evaluation_and_apply_only_when_ticked() {
    let t = tmp();
    let (dir, d) = song(t.path());
    let spy = Spy { seen: Mutex::new(Vec::new()) };
    let p = mix(&dir, &d, &spy, 3);
    let seen = spy.seen.lock().unwrap();

    // ANA-19: the new measures reach the model (rounded), sibilance only for the voice, the floor only for drums.
    let a = &seen[0].analysis;
    assert_eq!(a["version"], 4);
    let by_name = |name: &str| {
        let n = p.ids.n(&id(&d, name));
        a["tracks"][n.as_str()].clone()
    };
    let (vox, keys, kick) = (by_name("Vox"), by_name("Keys"), by_name("Kick"));
    assert!(vox["sibilance_db"].as_f64().unwrap() > -3.0, "{vox}");
    assert!(vox["sibilance_hz"].as_f64().is_some_and(|f| (6000.0..=8000.0).contains(&f)), "{vox}");
    assert!(keys.get("sibilance_db").is_none_or(Value::is_null) && kick["floor_db"].is_number());
    assert!(keys["crest_db"].is_number() && kick["transient_db"].is_number());
    // Only blocks that are not classic are listed: none yet (AI-25; checked again after the application).
    assert!(vox.get("blocks").is_none());

    // AI-26: ideas in the schema and the prompt of the first iteration only; the plan is v4.
    assert!(seen[0].allow_ideas && seen[0].schema["properties"]["ideas"].is_object());
    assert!(seen[0].system.contains("## Blocks") && seen[0].system.contains("idea"));
    assert_eq!(seen[0].plan["version"], 4);
    assert!(seen[0].plan["ideas"]["i1"].is_object() && seen[0].plan["ideas"]["i2"].is_object());
    for s in &seen[1..] {
        assert!(s.iteration > 0 && !s.allow_ideas);
        assert!(s.schema["properties"]["ideas"].is_null(), "no ideas after the first iteration");
    }
    // AI-26: the return an idea creates stays fixed: the later iterations cannot rewrite it or send to it, because
    // it is not in their schema nor in their document.
    let idea_returns: Vec<String> = seen[0].plan["structure"]["returns"]
        .as_object()
        .map(|m| m.iter().filter(|(_, r)| r["idea"].is_string()).map(|(k, _)| k.clone()).collect())
        .unwrap_or_default();
    assert!(!idea_returns.is_empty() && seen.len() > 1, "the fake decider has an idea with a return, in a loop");
    for s in &seen[1..] {
        let text = s.schema.to_string();
        for k in &idea_returns {
            assert!(s.schema["properties"]["returns"]["properties"][k.as_str()].is_null(), "{k} is in the schema");
            assert!(
                !text.contains(&format!("\"{k}\"")),
                "{k} is still named in the schema of iteration {}",
                s.iteration
            );
            let listed = s.analysis["returns"].as_array().is_some_and(|r| r.iter().any(|x| x["id"] == k.as_str()));
            assert!(!listed, "{k} is in the document of iteration {}", s.iteration);
        }
    }

    // The ideas are in the proposal, unchanged, and none of them is in what the loop evaluated and previews.
    let ideas = p.plan.ideas.as_ref().expect("ideas in the proposal");
    assert!((1..=3).contains(&ideas.len()));
    let keys_id = id(&d, "Keys");
    let preview_keys = p.preview.track(&keys_id).unwrap();
    assert!(preview_keys.chain.first(FxKind::Modulation).is_none(), "the evaluated mix has no idea in it");

    // AI-25: the de-esser is a block of the plan itself, shown as a new row in the vocal's diff.
    let vox_id = id(&d, "Vox");
    let vd = p.diff.iter().find(|x| x.track == vox_id).expect("the vocal changes");
    assert!(vd.rows.iter().any(|r| r.label.contains("De-esser") && r.text.is_some()), "{:?}", vd.rows);

    // AI-27: the proposal view lists the ideas, off by default, with sections and elements.
    let view = ProposalView::of(&p, &d.project, Vec::new(), None, false, &HashSet::new());
    assert_eq!(view.ideas.len(), ideas.len());
    assert!(view.ideas.iter().all(|i| !i.on && !i.elements.is_empty() && !i.tracks.is_empty()));
    let i1 = view.ideas.iter().find(|i| i.id == "i1").unwrap();
    assert_eq!(i1.sections, ["Chorus", "Chorus"]);
    assert!(i1.elements.iter().any(|e| e.starts_with("Keys · Modulation")), "{:?}", i1.elements);

    // Nothing ticked: the plan alone, the same as what was evaluated.
    let none = applied(&d, &p, &[]);
    assert_eq!(none.project.tracks, p.preview.tracks, "what was listened to is what is applied");
    assert!(!none.project.tracks.iter().any(|t| t.name == "Ping-pong"));

    // AI-23, AI-25: the same proposal applied again on the mix it produced adds nothing: no second de-esser.
    let mut twice = Document::new(none.project.clone());
    let cmds = apply_commands(&none.project, &p, None, &ticked(&[]));
    twice.transact(Phase::Mix, "Apply auto mix", cmds, None).unwrap();
    assert_eq!(twice.project.tracks, none.project.tracks, "a second application changes nothing");

    // AI-25: on the applied mix, the next request sees the de-esser with a neutral id.
    let again = Spy { seen: Mutex::new(Vec::new()) };
    mix(&dir, &none, &again, 1);
    let later = &again.seen.lock().unwrap()[0].analysis["tracks"][p.ids.n(&vox_id).as_str()]["blocks"];
    assert!(
        later.as_array().is_some_and(|b| b
            .iter()
            .any(|x| x["type"] == "deesser" && x["id"].as_str().is_some_and(|i| i.starts_with('k')))),
        "{later}"
    );

    // One idea ticked: only its elements are added, the rest of the plan is identical.
    let one = applied(&d, &p, &["i1"]);
    for (a, b) in one.project.tracks.iter().zip(&none.project.tracks) {
        if a.id != keys_id {
            assert_eq!(a, b, "{} changes only with its own idea", a.name);
        }
    }
    let k = one.project.track(&keys_id).unwrap();
    let ch = k.chain.blocks.iter().find(|b| matches!(b.fx, Fx::Modulation(_))).expect("the chorus block");
    // On only in the choruses: a stepped bypass lane, off (1) in the verse and the bridge.
    let lane = k.lane(&format!("bypass.{}", ch.id)).expect("a bypass lane for the chorus");
    let at = |s: f64| stemwise_core::automation::value_at(&lane.points, (s * f64::from(RATE)) as i64, true).unwrap();
    assert_eq!((at(1.0), at(3.0), at(5.0), at(7.0)), (1.0, 0.0, 1.0, 0.0));
    // UI-13: the keys' note names the idea.
    let note = one.project.ai.notes.iter().find(|n| n.track.as_deref() == Some(keys_id.as_str())).unwrap();
    assert!(note.text.contains(&format!("Idea “{}”", ideas["i1"].name)), "{}", note.text);

    // Everything ticked: the filter build has its own resonant low-pass moving in the section before the last
    // chorus; the throws have a new ping-pong return fed by the vocal.
    let mut all = applied(&d, &p, &["i1", "i2", "i3"]);
    let s = all.project.track(&id(&d, "Strings")).unwrap();
    let build = s
        .chain
        .blocks
        .iter()
        .find(|b| matches!(&b.fx, Fx::Lpf(f) if !f.bypass && f.resonance > 1.5))
        .expect("the idea's low-pass");
    let lane = s.lane(&format!("{}.freq", build.id)).expect("the filter moves");
    let at = |s: f64| stemwise_core::automation::value_at(&lane.points, (s * f64::from(RATE)) as i64, false).unwrap();
    assert!(at(5.0) < at(1.0) / 4.0 && (at(7.0) - at(1.0)).abs() < 1.0, "{} {} {}", at(1.0), at(5.0), at(7.0));
    let pp = all.project.tracks.iter().find(|t| t.name == "Ping-pong").expect("the idea's return");
    assert!(matches!(&pp.ret, Some(ReturnFx::Delay { mode: DelayMode::PingPong, .. })));
    assert!(all.project.track(&vox_id).unwrap().send_db(&pp.id).is_some());

    // The same with every idea ticked: their blocks are not doubled either.
    let mut twice = Document::new(all.project.clone());
    let cmds = apply_commands(&all.project, &p, None, &ticked(&["i1", "i2", "i3"]));
    twice.transact(Phase::Mix, "Apply auto mix", cmds, None).unwrap();
    for (a, b) in twice.project.tracks.iter().zip(&all.project.tracks) {
        assert_eq!(a.chain.blocks.len(), b.chain.blocks.len(), "{}: blocks doubled", a.name);
    }

    // AI-3: a revision starts from the mix as it is, the idea's moving filter included (F17 review, #1).
    let opts = MixOptions { max_iterations: 1, request: Some("vocal louder".into()), scope: None };
    let rev = run_mix(&dir, &all.project, &FakeDecider, &|_, _| 0.0, &opts, &|_| {}, &AtomicBool::new(false)).unwrap();
    let mut revised = Document::new(all.project.clone());
    revised.transact(Phase::Mix, "Apply revision", rev.commands.clone(), None).unwrap();
    let s = revised.project.track(&id(&d, "Strings")).unwrap();
    assert!(s.lane(&format!("{}.freq", build.id)).is_some(), "the filter still moves after a revision");

    // HIS-3: one undo removes the ideas with the rest.
    all.undo(Phase::Mix).unwrap();
    assert_eq!(all.project, d.project);
}

/// req: AI-27
#[test]
fn with_ideas_switched_off_the_schema_and_the_prompt_have_none() {
    let t = tmp();
    let (dir, mut d) = song(t.path());
    d.apply_view(vec![Command::SetAllowIdeas { on: false }]).unwrap();
    let spy = Spy { seen: Mutex::new(Vec::new()) };
    let p = mix(&dir, &d, &spy, 1);
    let seen = spy.seen.lock().unwrap();
    assert!(!seen[0].allow_ideas && seen[0].schema["properties"]["ideas"].is_null());
    assert!(seen[0].plan.get("ideas").is_none_or(Value::is_null));
    assert!(p.plan.ideas.is_none());
    // The blocks of the plan are not ideas: they stay (AI-25).
    assert!(p.preview.track(&id(&d, "Vox")).unwrap().chain.first(FxKind::Deesser).is_some());
}
