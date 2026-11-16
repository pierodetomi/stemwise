//! The structural mix AI (F12 gate): true doubles, new returns and groups created by a plan, a structure that
//! stays put through the loop, selective application, and the techniques of the fake decider.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};
use stemwise_ai::decider::{DecideError, Decider, DecisionInput, FakeDecider, Message, RawDecision, Retry, Usage};
use stemwise_ai::mixing::{MixOptions, MixProposal, apply_commands, run_mix, skipped_elements};
use stemwise_ai::structure::{NewKind, Skipped};
use stemwise_core::command::Command;
use stemwise_core::edit;
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, add_reference, import};
use stemwise_core::model::{Output, ReturnFx, ReverbAlgorithm};
use stemwise_core::{package, role::Role, routing, wav};

const RATE: u32 = 48000;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/// One guitar part as a performance: every strum has its own strength, timing and brightness.
fn guitar(seed: u32, detune: f64, n: usize) -> Vec<f32> {
    let tau = std::f64::consts::TAU;
    let pick = stemwise_dsp::testsignal::xorshift32(seed, n);
    let strum = RATE as usize / 4;
    let human = stemwise_dsp::testsignal::xorshift32(seed + 100, 3 * (n / strum + 2));
    (0..n)
        .map(|i| {
            let k = i / strum;
            let late = (0.012 * (1.0 + human[3 * k]) * f64::from(RATE)) as usize;
            let Some(since) = (i % strum).checked_sub(late) else { return 0.0 };
            let (t, p) = (i as f64 / f64::from(RATE), since as f64 / f64::from(RATE));
            let (strength, bright) = (0.7 + 0.3 * human[3 * k + 1], 0.5 + 0.4 * human[3 * k + 2]);
            let chord: f64 = [196.0, 246.9, 293.7, 392.0]
                .iter()
                .enumerate()
                .map(|(h, f)| (1.0 - bright * 0.2 * h as f64) * (tau * f * detune * t).sin())
                .sum();
            (0.1 * strength * (-p * 7.0).exp() * (chord + (0.3 + bright) * pick[i])) as f32
        })
        .collect()
}

/// A song with a kit on two tracks, a bass, one guitar part played twice, a vocal, two sections and a reference.
fn song(t: &Path) -> (PathBuf, Document) {
    let src = t.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (dir, p) = package::create(t, "Doubles").unwrap();
    let mut d = Document::new(p);
    let n = 8 * RATE as usize;
    let tau = std::f64::consts::TAU;
    let synth = |f: &dyn Fn(f64) -> f64| -> Vec<f32> { (0..n).map(|i| f(i as f64 / f64::from(RATE)) as f32).collect() };
    let hits = stemwise_dsp::testsignal::xorshift32(12, n);
    let stems: Vec<(&str, Role, Vec<f32>)> = vec![
        ("Kick", Role::Kick, synth(&|t| 0.8 * (-(t % 0.5) * 20.0).exp() * (tau * 55.0 * t).sin())),
        (
            "Snare",
            Role::Snare,
            (0..n)
                .map(|i| (0.5 * (-(((i as f64 / f64::from(RATE)) + 0.5) % 1.0) * 22.0).exp() * hits[i]) as f32)
                .collect(),
        ),
        ("Bass", Role::Bass, synth(&|t| 0.4 * (tau * 55.0 * t).sin())),
        ("Gtr A", Role::ElectricGuitar, guitar(21, 1.0, n)),
        ("Gtr B", Role::ElectricGuitar, guitar(22, 1.004, n)),
        ("Vox", Role::LeadVocal, synth(&|t| 0.25 * (tau * 440.0 * t).sin() * if t % 1.0 < 0.7 { 1.0 } else { 0.0 })),
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
    for (name, a, b) in [("Verse", 0, 4), ("Chorus", 4, 8)] {
        let cmds = edit::add_section(&mut d.project, name, a * i64::from(RATE), b * i64::from(RATE)).unwrap();
        d.transact(Phase::Mix, "s", cmds, None).unwrap();
    }
    let rpath = src.join("reference.wav");
    let refx: Vec<f32> = stemwise_dsp::testsignal::xorshift32(5, 2 * n).iter().map(|v| (v * 0.2) as f32).collect();
    wav::write_f32(&rpath, 2, RATE, &refx).unwrap();
    let c = add_reference(&dir, &d.project, &rpath).unwrap();
    d.transact(Phase::Mix, "ref", vec![c], None).unwrap();
    // These tests are about the structure (AI-17 … AI-20): creative ideas, which may add a return of their own, are
    // tested in `ideas_flow`.
    d.apply_view(vec![stemwise_core::command::Command::SetAllowIdeas { on: false }]).unwrap();
    (dir, d)
}

fn id(d: &Document, name: &str) -> String {
    d.project.tracks.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("no track {name}")).id.clone()
}

/// What a decider was asked and what it answered, per call.
struct Seen {
    iteration: u32,
    allow_structure: bool,
    schema: Value,
    analysis: Value,
    system: String,
    plan: Value,
}

/// An edit of the fake decider's plan before it is returned: the input, the plan, the number of the call.
type PlanEdit = dyn Fn(&DecisionInput, &mut Value, usize) + Send + Sync;

/// The fake decider, with every call recorded and the plan optionally edited before it is returned.
struct Spy {
    seen: Mutex<Vec<Seen>>,
    edit: Box<PlanEdit>,
}

impl Spy {
    fn new() -> Self {
        Spy::editing(|_, _, _| {})
    }
    fn editing(f: impl Fn(&DecisionInput, &mut Value, usize) + Send + Sync + 'static) -> Self {
        Spy { seen: Mutex::new(Vec::new()), edit: Box::new(f) }
    }
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
        let mut plan = serde_json::to_value(FakeDecider.mix_plan(input)).unwrap();
        let call = self.seen.lock().unwrap().len();
        (self.edit)(input, &mut plan, call);
        self.seen.lock().unwrap().push(Seen {
            iteration: input.iteration,
            allow_structure: input.allow_structure,
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

fn mix(dir: &Path, d: &Document, decider: &dyn Decider, opts: &MixOptions) -> MixProposal {
    run_mix(dir, &d.project, decider, &|_, _| 0.0, opts, &|_| {}, &AtomicBool::new(false)).unwrap()
}

fn opts(n: u32) -> MixOptions {
    MixOptions { max_iterations: n, ..Default::default() }
}

fn element<'a>(p: &'a MixProposal, name: &str) -> &'a stemwise_ai::structure::NewElement {
    p.new_elements.iter().find(|e| e.track.name == name).unwrap_or_else(|| panic!("no new element {name}"))
}

/// req: AI-17, AI-19, AI-20, AI-21, ANA-18, HIS-3
#[test]
fn the_auto_mix_creates_returns_and_a_group_once_and_applies_them_in_one_step() {
    let t = tmp();
    let (dir, mut d) = song(t.path());
    let before = d.project.clone();
    let spy = Spy::new();
    let p = mix(&dir, &d, &spy, &opts(3));
    let seen = spy.seen.lock().unwrap();

    // ANA-18: the two guitars are a double, two performances and not one signal; nothing else is.
    let doubles = seen[0].analysis["doubles"].as_array().unwrap();
    assert_eq!(doubles.len(), 1, "{doubles:?}");
    assert_eq!(doubles[0]["identical"], json!(false));
    let names: Vec<String> = ["a", "b"]
        .iter()
        .map(|k| p.ids.to_project[doubles[0][*k].as_str().unwrap()].clone())
        .map(|id| d.project.track(&id).unwrap().name.clone())
        .collect();
    assert_eq!(names, ["Gtr A", "Gtr B"]);

    // AI-17: two returns and a group, each with a name and a reason; nothing is applied yet.
    assert_eq!(p.new_elements.len(), 3);
    let (plate, slap, drums) = (element(&p, "Vocal plate"), element(&p, "Slap"), element(&p, "Drums"));
    assert!(plate.kind == NewKind::Return && slap.kind == NewKind::Return && drums.kind == NewKind::Group);
    assert!(p.new_elements.iter().all(|e| !e.rationale.is_empty()));
    assert_eq!(plate.users, [id(&d, "Vox")]);
    assert_eq!(drums.members, [id(&d, "Kick"), id(&d, "Snare")]);
    assert_eq!(d.project, before);

    // AI-19: the first iteration may create, the later ones get what was created as ordinary returns and groups.
    assert!(seen.len() >= 2, "the loop runs more than one iteration against this reference");
    assert!(seen[0].allow_structure && seen[0].schema["properties"]["structure"].is_object());
    assert!(seen[0].system.contains("## New returns and groups") && seen[0].system.contains("## Techniques"));
    assert!(seen[0].plan["structure"]["returns"]["n1"].is_object());
    for s in &seen[1..] {
        assert!(s.iteration > 0 && !s.allow_structure);
        assert!(s.schema["properties"]["structure"].is_null(), "no structure after the first iteration");
        assert!(!s.system.contains("## New returns and groups"));
        // The placeholders are now a return of the project and a group of the project, required like the others.
        assert_eq!(
            s.schema["properties"]["returns"]["properties"]["n1"]["properties"]["type"]["enum"],
            json!(["reverb"])
        );
        assert!(s.schema["properties"]["groups"]["required"].as_array().unwrap().contains(&json!("n4")));
        assert!(s.analysis["returns"].as_array().unwrap().iter().any(|r| r["id"] == "n1" && r["algorithm"] == "plate"));
        assert!(s.plan.get("structure").is_none() && s.plan["groups"]["n4"].is_object());
    }
    // The plan that became the proposal carries no structure of its own: it is in the working project.
    assert!(p.plan.structure.is_none());

    // HIS-3: one transaction applies everything, creations included; the routing stays valid.
    d.transact(Phase::Mix, "Apply auto mix", p.commands.clone(), Some(p.call_id.clone())).unwrap();
    routing::validate(&d.project).unwrap();
    assert_eq!(d.project.tracks.len(), before.tracks.len() + 3);
    assert_eq!(d.project.tracks, p.preview.tracks, "what was listened to is what is applied");

    // AI-21, distinct spaces: the vocal has its own plate and leaves the shared reverb; a slapback beside it.
    let vox = d.project.track(&id(&d, "Vox")).unwrap();
    let shared = d.project.tracks.iter().find(|t| t.name == "Reverb").unwrap();
    let plate_t = d.project.track(&plate.track.id).unwrap();
    assert!(
        matches!(&plate_t.ret, Some(ReturnFx::Reverb { algorithm: ReverbAlgorithm::Plate, decay_s, .. }) if *decay_s >= 0.8)
    );
    assert!(vox.send_db(&plate.track.id).is_some() && vox.send_db(&shared.id).is_none());
    let slap_t = d.project.track(&slap.track.id).unwrap();
    assert!(
        matches!(&slap_t.ret, Some(ReturnFx::Delay { time_ms, feedback, .. }) if (80.0..=140.0).contains(time_ms) && *feedback == 0.0)
    );
    assert!(vox.send_db(&slap.track.id).is_some());
    // Delay throw: the send to the delay is higher in the chorus, through a lane the assistant wrote.
    let delay = d.project.tracks.iter().find(|t| t.name == "Delay").unwrap();
    let lane = vox.lane(&format!("send.{}", delay.id)).expect("a lane on the send to the delay");
    let at = |f: i64| stemwise_core::automation::value_at(&lane.points, f, false).unwrap();
    assert!((at(6 * i64::from(RATE)) - at(2 * i64::from(RATE)) - 6.0).abs() < 1e-9);
    // Parallel compression: the kit goes through a new group whose compressor is blended.
    let group = d.project.track(&drums.track.id).unwrap();
    assert!(group.is_group() && group.output == Output::MixBus);
    assert!(
        !group.chain.comp().unwrap().bypass
            && group.chain.comp().unwrap().mix < 0.5
            && group.chain.comp().unwrap().ratio >= 4.0
    );
    for m in ["Kick", "Snare"] {
        assert_eq!(d.project.track(&id(&d, m)).unwrap().output, Output::Group(group.id.clone()));
    }
    assert!(d.project.track_index(&group.id).unwrap() < d.project.track_index(&id(&d, "Kick")).unwrap());
    // True doubles: opposite, symmetric pans at one level.
    let (a, b) = (d.project.track(&id(&d, "Gtr A")).unwrap(), d.project.track(&id(&d, "Gtr B")).unwrap());
    assert!(a.chain.pan == -b.chain.pan && a.chain.pan.abs() >= 0.6 && a.chain.fader_db == b.chain.fader_db);
    // Every technique is explained where it is used.
    let note = |tr: &str| d.project.ai.notes.iter().find(|n| n.track.as_deref() == Some(tr)).unwrap().text.clone();
    assert!(note(&a.id).contains("double") && note(&vox.id).contains("plate") && note(&group.id).contains("Parallel"));

    // One undo removes the new tracks with everything else.
    d.undo(Phase::Mix).unwrap();
    assert_eq!(d.project, before);

    // A second auto mix on the applied result does not create them again: it finds them by what they are.
    d.redo(Phase::Mix).unwrap();
    let again = mix(&dir, &d, &FakeDecider, &opts(1));
    assert!(
        again.new_elements.is_empty(),
        "{:?}",
        again.new_elements.iter().map(|e| &e.track.name).collect::<Vec<_>>()
    );
    let cmds = apply_commands(&d.project, &again, None, &std::collections::HashSet::new());
    d.transact(Phase::Mix, "Apply again", cmds, None).unwrap();
    assert!(d.project.track(&id(&d, "Vox")).unwrap().send_db(&plate.track.id).is_some());
}

/// req: AI-20
#[test]
fn applying_some_tracks_creates_only_what_they_need_and_what_is_ticked() {
    let t = tmp();
    let (dir, d) = song(t.path());
    let p = mix(&dir, &d, &FakeDecider, &opts(1));
    let (plate, slap, drums) = (element(&p, "Vocal plate"), element(&p, "Slap"), element(&p, "Drums"));
    let (vox, kick, snare) = (id(&d, "Vox"), id(&d, "Kick"), id(&d, "Snare"));
    let apply = |only: &[&String]| {
        let set: HashSet<String> = only.iter().map(|s| (*s).clone()).collect();
        let mut doc = Document::new(d.project.clone());
        let cmds = apply_commands(&d.project, &p, Some(&set), &std::collections::HashSet::new());
        doc.transact(Phase::Mix, "Apply selected", cmds, None).unwrap();
        routing::validate(&doc.project).unwrap();
        (doc.project, skipped_elements(&d.project, &p, Some(&set)))
    };

    // The vocal with its plate, the slapback unticked: the plate is created, the slapback is not and the vocal's
    // send to it falls, the rest of the vocal's changes stay; the group has no member applied.
    let (after, skipped) = apply(&[&vox, &plate.track.id]);
    assert!(after.track(&plate.track.id).is_some() && after.track(&slap.track.id).is_none());
    let v = after.track(&vox).unwrap();
    assert!(v.send_db(&plate.track.id).is_some() && v.send_db(&slap.track.id).is_none());
    assert!(v.sends.iter().all(|s| after.track(&s.target).is_some()), "no send points at a missing return");
    assert!(!v.chain.comp().unwrap().bypass, "the other changes of the vocal are applied");
    assert_eq!(after.track(&kick).unwrap(), d.project.track(&kick).unwrap(), "an unticked track is untouched");
    assert_eq!(skipped, [(slap.track.id.clone(), Skipped::LeftOut), (drums.track.id.clone(), Skipped::LeftOut)]);

    // Everything ticked but the vocal: its returns are ticked too, yet nobody applied sends to them.
    let all_but_vox: Vec<&String> = d
        .project
        .tracks
        .iter()
        .map(|t| &t.id)
        .filter(|i| **i != vox)
        .chain(p.new_elements.iter().map(|e| &e.track.id))
        .collect();
    let (after, skipped) = apply(&all_but_vox);
    assert!(after.track(&drums.track.id).is_some() && after.track(&plate.track.id).is_none());
    assert_eq!(skipped, [(plate.track.id.clone(), Skipped::NotNeeded), (slap.track.id.clone(), Skipped::NotNeeded)]);

    // The group ticked with one of its two members: not created, and the member stays where it was.
    let (after, skipped) = apply(&[&kick, &drums.track.id]);
    assert!(after.track(&drums.track.id).is_none());
    assert_eq!(after.track(&kick).unwrap().output, Output::MixBus);
    assert!(skipped.contains(&(drums.track.id.clone(), Skipped::NotNeeded)));

    // Both members ticked: the group is created around them.
    let (after, _) = apply(&[&kick, &snare, &drums.track.id]);
    assert_eq!(after.track(&snare).unwrap().output, Output::Group(drums.track.id.clone()));

    // The project changed during the review: the snare went into a group of the author's. The planned group no
    // longer fits and is left out; everything else is applied.
    let mut changed = Document::new(d.project.clone());
    let cmds = edit::add_group(&mut changed.project, "Mine", std::slice::from_ref(&snare)).unwrap();
    changed.transact(Phase::Mix, "group", cmds, None).unwrap();
    assert_eq!(skipped_elements(&changed.project, &p, None), [(drums.track.id.clone(), Skipped::NoLongerFits)]);
    let cmds = apply_commands(&changed.project, &p, None, &std::collections::HashSet::new());
    changed.transact(Phase::Mix, "Apply", cmds, None).unwrap();
    routing::validate(&changed.project).unwrap();
    assert!(changed.project.track(&drums.track.id).is_none() && changed.project.track(&plate.track.id).is_some());
    assert!(matches!(&changed.project.track(&snare).unwrap().output, Output::Group(g) if *g != drums.track.id));
}

/// req: AI-18
#[test]
fn structure_is_off_with_the_option_with_a_scope_and_for_locked_tracks() {
    let t = tmp();
    let (dir, mut d) = song(t.path());
    // The option off: no `structure` in the schema or in the prompt, and the vocal stays in the shared reverb.
    d.apply_view(vec![Command::SetAllowStructure { on: false }]).unwrap();
    let spy = Spy::new();
    let p = mix(&dir, &d, &spy, &opts(1));
    {
        let seen = spy.seen.lock().unwrap();
        assert!(!seen[0].allow_structure && seen[0].schema["properties"]["structure"].is_null());
        assert!(!seen[0].system.contains("## New returns and groups"));
        assert!(seen[0].system.contains("You never create, delete or reroute"));
    }
    assert!(p.new_elements.is_empty());
    let shared = id(&d, "Reverb");
    let vox_after = p.preview.track(&id(&d, "Vox")).unwrap();
    assert!(vox_after.send_db(&shared).is_some());
    // The doubles are still opened: that is not structure.
    assert!(p.preview.track(&id(&d, "Gtr A")).unwrap().chain.pan < 0.0);

    // A plan that brings structure anyway is not refused: what it asked for is dropped with a warning.
    let pushy = Spy::editing(|_, plan, _| {
        plan["structure"] =
            json!({"returns": {"n1": {"type": "reverb", "name": "Sneaky", "rationale": "x"}}, "groups": {}});
        plan["tracks"]["t1"]["sends_db"]["n1"] = json!(-20.0);
    });
    let p = mix(&dir, &d, &pushy, &opts(1));
    assert!(p.new_elements.is_empty());
    assert!(p.warnings.iter().any(|w| w.contains("does not allow")), "{:?}", p.warnings);
    assert!(p.preview.tracks.iter().all(|t| t.name != "Sneaky"));

    // The option on, a request limited to some tracks: no structure either.
    d.apply_view(vec![Command::SetAllowStructure { on: true }]).unwrap();
    let scoped = MixOptions { scope: Some([id(&d, "Vox")].into()), ..opts(1) };
    let p = mix(&dir, &d, &FakeDecider, &scoped);
    assert!(p.new_elements.is_empty());

    // A locked track does not go into a new group: with the snare locked the kit has one free track left.
    let snare = id(&d, "Snare");
    let cmds = edit::run(&mut d.project, &edit::Edit::SetLocked { track: snare.clone(), locked: true }).unwrap();
    d.transact(Phase::Mix, "lock", cmds, None).unwrap();
    let p = mix(&dir, &d, &FakeDecider, &opts(1));
    assert!(p.new_elements.iter().all(|e| e.kind == NewKind::Return), "returns only");
    assert_eq!(p.preview.track(&snare).unwrap(), d.project.track(&snare).unwrap());
}

/// req: AI-19, AI-17, AI-8
#[test]
fn what_cannot_be_created_is_dropped_with_a_warning_and_a_broken_plan_is_retried() {
    let t = tmp();
    let (dir, mut d) = song(t.path());
    let snare = id(&d, "Snare");
    let cmds = edit::run(&mut d.project, &edit::Edit::SetLocked { track: snare, locked: true }).unwrap();
    d.transact(Phase::Mix, "lock", cmds, None).unwrap();
    let spy = Spy::editing(|inp, plan, call| {
        // A return nobody sends to, a group with a locked member and a return as members, a name far too long.
        let n = |name: &str| inp.ids.n(&inp.project.tracks.iter().find(|t| t.name == name).unwrap().id);
        plan["structure"]["returns"]["n3"] = json!({"type": "reverb", "name": "Unused hall", "rationale": "x"});
        plan["returns"]["n3"] = plan["returns"]["n1"].clone();
        plan["structure"]["groups"] = json!({
            "n4": {"name": "Kit", "members": [n("Kick"), n("Snare"), n("Reverb")], "rationale": "x"},
            "n5": {"name": "A very long name for a guitars group indeed", "members": [n("Gtr A"), n("Gtr B")], "rationale": "x"},
        });
        plan["groups"]["n4"] =
            json!({"gain_db": 0.0, "eq": [], "compressor": null, "saturation": null, "sends_db": {}, "rationale": "x"});
        // The first answer forgets the chain of the group it creates: that is an error, sent back.
        if call > 0 {
            plan["groups"]["n5"] = plan["groups"]["n4"].clone();
        }
    });
    let p = mix(&dir, &d, &spy, &opts(1));
    assert_eq!(spy.seen.lock().unwrap().len(), 2, "one retry");
    let names: Vec<&str> = p.new_elements.iter().map(|e| e.track.name.as_str()).collect();
    assert_eq!(names, ["Vocal plate", "Slap", "A very long name for a g"], "the name is cut at 24 characters");
    let w = p.warnings.join("\n");
    for needle in [
        "Unused hall) has no track sending to it",
        "is locked or outside the request",
        "returns do not go into groups",
        "fewer than two valid members",
    ] {
        assert!(w.contains(needle), "{needle} in:\n{w}");
    }
    // The dropped group left no trace in the plan; the created one holds the two guitars.
    assert!(!p.plan.groups.contains_key("n4") && !p.plan.returns.contains_key("n3"));
    let g = element(&p, "A very long name for a g");
    assert_eq!(g.members, [id(&d, "Gtr A"), id(&d, "Gtr B")]);

    // A revision may bring structure in its single iteration (AI-19).
    let reviser = Spy::editing(|inp, plan, _| {
        let n = |name: &str| inp.ids.n(&inp.project.tracks.iter().find(|t| t.name == name).unwrap().id);
        plan["structure"] =
            json!({"returns": {"n1": {"type": "delay", "name": "Throw", "rationale": "x"}}, "groups": {}});
        plan["returns"]["n1"] = json!({"type": "delay", "time_ms": 500.0, "feedback": 0.3, "feedback_hpf_hz": 200.0, "feedback_lpf_hz": 5000.0, "return_gain_db": -10.0});
        plan["tracks"][n("Vox")]["sends_db"]["n1"] = json!(-20.0);
    });
    let rev = MixOptions { request: Some("a longer echo on the vocal".into()), ..opts(1) };
    let p = mix(&dir, &d, &reviser, &rev);
    assert_eq!(p.new_elements.len(), 1);
    assert!(
        matches!(&p.preview.track(&element(&p, "Throw").track.id).unwrap().ret, Some(ReturnFx::Delay { time_ms, .. }) if *time_ms == 500.0)
    );
}

/// req: AI-20, AI-19, AI-17, HIS-3
#[test]
fn new_elements_that_depend_on_each_other_are_created_together_or_fall_cleanly() {
    let t = tmp();
    let (dir, mut d) = song(t.path());
    // The kit gets its bus (the fake decider's own rule), a room that only that bus sends to, and the bass is keyed
    // by the bus: a new return used by a new group alone, and a sidechain from a new group.
    let spy = Spy::editing(|inp, plan, _| {
        let n = |name: &str| inp.ids.n(&inp.project.tracks.iter().find(|t| t.name == name).unwrap().id);
        if inp.iteration == 0 {
            plan["structure"]["returns"]["n3"] = json!({"type": "reverb", "name": "Drum room", "rationale": "x"});
        }
        plan["returns"]["n3"] = json!({"type": "reverb", "algorithm": "room", "decay_s": 0.6, "size": 0.4, "damping": 0.5, "modulation": 0.1, "predelay_ms": 5.0, "return_hpf_hz": 200.0, "return_lpf_hz": 9000.0, "return_gain_db": -10.0});
        plan["groups"]["n4"]["sends_db"]["n3"] = json!(-18.0);
        plan["tracks"][n("Bass")]["compressor"]["sidechain"] = json!("n4");
    });
    let p = mix(&dir, &d, &spy, &opts(1));
    let (room, drums) = (element(&p, "Drum room"), element(&p, "Drums"));
    assert_eq!(room.users, std::slice::from_ref(&drums.track.id), "the room exists for the new group");
    let bass = id(&d, "Bass");

    // Apply all, as one transaction: both are created, the group sends to the room, the bass listens to the group.
    let mut all = Document::new(d.project.clone());
    all.transact(Phase::Mix, "Apply", p.commands.clone(), None).unwrap();
    routing::validate(&all.project).unwrap();
    assert_eq!(all.project.tracks, p.preview.tracks);
    assert!(all.project.track(&drums.track.id).unwrap().send_db(&room.track.id).is_some());
    assert_eq!(
        all.project.track(&bass).unwrap().chain.comp().unwrap().sidechain.as_ref().map(|s| s.source.clone()),
        Some(drums.track.id.clone())
    );
    assert!(skipped_elements(&d.project, &p, None).is_empty());
    // The same through the path the app uses.
    let mut app = Document::new(d.project.clone());
    app.transact(Phase::Mix, "Apply", apply_commands(&d.project, &p, None, &std::collections::HashSet::new()), None)
        .unwrap();
    assert_eq!(app.project.tracks, all.project.tracks);

    // The group unticked: the room has no sender left and is not created either; the bass keeps its other changes
    // and listens to itself; the application still goes through as one step.
    let only: HashSet<String> = d
        .project
        .tracks
        .iter()
        .map(|t| t.id.clone())
        .chain(p.new_elements.iter().map(|e| e.track.id.clone()))
        .filter(|i| *i != drums.track.id)
        .collect();
    let mut part = Document::new(d.project.clone());
    part.transact(
        Phase::Mix,
        "Apply selected",
        apply_commands(&d.project, &p, Some(&only), &std::collections::HashSet::new()),
        None,
    )
    .unwrap();
    routing::validate(&part.project).unwrap();
    assert!(part.project.track(&drums.track.id).is_none() && part.project.track(&room.track.id).is_none());
    let b = part.project.track(&bass).unwrap();
    assert!(
        !b.chain.comp().unwrap().bypass && b.chain.comp().unwrap().sidechain.is_none(),
        "{:?}",
        b.chain.comp().unwrap()
    );
    assert_eq!(
        skipped_elements(&d.project, &p, Some(&only)),
        [(room.track.id.clone(), Skipped::NotNeeded), (drums.track.id.clone(), Skipped::LeftOut)]
    );
    // What the proposal tells the interface: fine as planned, and no longer fitting once the kick is locked.
    let view =
        stemwise_ai::dto::ProposalView::of(&p, &d.project, Vec::new(), None, false, &std::collections::HashSet::new());
    assert!(view.new_elements.iter().all(|e| e.fits));
    let mut locked = d.project.clone();
    locked.track_mut(&id(&d, "Kick")).unwrap().locked = true;
    let view =
        stemwise_ai::dto::ProposalView::of(&p, &locked, Vec::new(), None, false, &std::collections::HashSet::new());
    assert!(!view.new_elements.iter().find(|e| e.name == "Drums").unwrap().fits);

    // A new return whose only sender is a locked track: the sender's plan is ignored, so nobody sends to it and
    // it is not part of the proposal (AI-19); the commands still apply.
    let gtr = id(&d, "Gtr A");
    let cmds = edit::run(&mut d.project, &edit::Edit::SetLocked { track: gtr, locked: true }).unwrap();
    d.transact(Phase::Mix, "lock", cmds, None).unwrap();
    let ghost = Spy::editing(|inp, plan, _| {
        let n = |name: &str| inp.ids.n(&inp.project.tracks.iter().find(|t| t.name == name).unwrap().id);
        plan["structure"]["returns"]["n3"] = json!({"type": "delay", "name": "Ghost", "rationale": "x"});
        plan["returns"]["n3"] = json!({"type": "delay", "time_ms": 300.0, "feedback": 0.2, "feedback_hpf_hz": 200.0, "feedback_lpf_hz": 5000.0, "return_gain_db": -10.0});
        let mut locked_plan = plan["tracks"][n("Gtr B")].clone();
        locked_plan["sends_db"]["n3"] = json!(-20.0);
        plan["tracks"][n("Gtr A")] = locked_plan;
    });
    let p = mix(&dir, &d, &ghost, &opts(1));
    assert!(p.new_elements.iter().all(|e| e.track.name != "Ghost"));
    assert!(p.warnings.iter().any(|w| w.contains("(Ghost) has no track sending to it")), "{:?}", p.warnings);
    assert!(!p.plan.returns.contains_key("n3") && p.preview.tracks.iter().all(|t| t.name != "Ghost"));
    let mut doc = Document::new(d.project.clone());
    doc.transact(Phase::Mix, "Apply", p.commands.clone(), None).unwrap();
    routing::validate(&doc.project).unwrap();
}
