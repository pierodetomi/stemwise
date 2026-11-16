//! Full AI mix flows with the fake decider (F5 gate) and a scripted decider for retries and failures.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use serde_json::Value;
use stemwise_ai::decider::{DecideError, Decider, DecisionInput, FakeDecider, Message, RawDecision, Retry, Usage};
use stemwise_ai::mixing::{AiError, MixOptions, partial_commands, run_mix};
use stemwise_core::command::Command;
use stemwise_core::edit;
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, add_reference, import};
use stemwise_core::model::{AiOutcome, Author, Brief};
use stemwise_core::{package, role::Role, wav};

const RATE: u32 = 48000;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/// A short song: kick, bass, guitar, vocal, two sections, one reference, an Italian brief.
fn song(t: &Path, italian: bool) -> (PathBuf, Document) {
    let src = t.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (dir, p) = package::create(t, "Titolo Segreto").unwrap();
    let mut d = Document::new(p);
    let n = 6 * RATE as usize;
    let tau = std::f64::consts::TAU;
    let synth = |f: &dyn Fn(f64) -> f64| -> Vec<f32> { (0..n).map(|i| f(i as f64 / f64::from(RATE)) as f32).collect() };
    let stems: Vec<(&str, Role, Vec<f32>)> = vec![
        ("Kick", Role::Kick, synth(&|t| 0.8 * (-(t % 0.5) * 20.0).exp() * (tau * 55.0 * t).sin())),
        ("Bass", Role::Bass, synth(&|t| 0.4 * (tau * 55.0 * t).sin())),
        ("Gtr", Role::ElectricGuitar, synth(&|t| 0.2 * ((tau * 330.0 * t).sin() + (tau * 2500.0 * t).sin()))),
        ("Vox", Role::LeadVocal, synth(&|t| 0.25 * (tau * 440.0 * t).sin() * if t % 1.0 < 0.7 { 1.0 } else { 0.0 })),
    ];
    let mut choices = Vec::new();
    for (name, role, x) in &stems {
        let path = src.join(format!("secret-file-{name}.wav"));
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
    for (name, a, b) in [("Verse", 0, 3), ("Chorus", 3, 6)] {
        let cmds = edit::add_section(&mut d.project, name, a * i64::from(RATE), b * i64::from(RATE)).unwrap();
        d.transact(Phase::Mix, "s", cmds, None).unwrap();
    }
    let rpath = src.join("reference.wav");
    let refx: Vec<f32> = stemwise_dsp::testsignal::xorshift32(5, 2 * n).iter().map(|v| (v * 0.2) as f32).collect();
    wav::write_f32(&rpath, 2, RATE, &refx).unwrap();
    let c = add_reference(&dir, &d.project, &rpath).unwrap();
    d.transact(Phase::Mix, "ref", vec![c], None).unwrap();
    let brief = if italian {
        Brief {
            genre: "rock".into(),
            mood: "caldo e intimo".into(),
            notes: "La voce deve stare davanti, con la batteria asciutta.".into(),
        }
    } else {
        Brief { genre: "rock".into(), mood: "warm".into(), notes: "Vocal in front, dry drums.".into() }
    };
    d.transact(Phase::Mix, "brief", vec![Command::SetBrief { brief }], None).unwrap();
    (dir, d)
}

fn opts(n: u32) -> MixOptions {
    MixOptions { max_iterations: n, ..Default::default() }
}

fn id(d: &Document, name: &str) -> String {
    d.project.tracks.iter().find(|t| t.name == name).unwrap().id.clone()
}

/// req: AI-1, AI-2, AI-6, AI-7, AI-9, AI-10, AI-13, HIS-3, AUT-5
#[test]
fn auto_mix_with_the_fake_decider_end_to_end() {
    let t = tmp();
    let (dir, mut d) = song(t.path(), true);
    let before_project = d.project.clone();
    let phases = Mutex::new(Vec::new());
    let p = run_mix(
        &dir,
        &d.project,
        &FakeDecider,
        &|_, _| 0.0,
        &opts(3),
        &|g| phases.lock().unwrap().push(g.phase),
        &AtomicBool::new(false),
    )
    .unwrap();
    // Progress through every phase (AI-12).
    let ph = phases.into_inner().unwrap();
    for want in [
        stemwise_ai::mixing::AiPhase::Analysis,
        stemwise_ai::mixing::AiPhase::Decision,
        stemwise_ai::mixing::AiPhase::Render,
        stemwise_ai::mixing::AiPhase::Evaluation,
    ] {
        assert!(ph.contains(&want), "{want:?}");
    }
    // A proposal with summary (Italian, AI-13), evaluations before/after, a diff and rationales; nothing applied yet.
    assert!(p.summary.contains("voce") || p.summary.contains("Bilanciamento"), "{}", p.summary);
    assert!(p.before.is_some() && p.after.as_ref().unwrap().valid());
    assert!(!p.iterations.is_empty() && p.iterations.len() <= 3);
    assert!(p.diff.iter().any(|t| t.name == "Vox" && !t.rows.is_empty() && !t.rationale.is_empty()));
    assert_eq!(d.project, before_project);
    assert_eq!(p.record.outcome, AiOutcome::Pending);
    assert_eq!(p.record.decider, "offline-demo");
    // One transaction applies everything (HIS-3), with the call id.
    d.transact(Phase::Mix, "Apply auto mix", p.commands.clone(), Some(p.call_id.clone())).unwrap();
    assert_eq!(d.history(Phase::Mix).entries().last().unwrap().ai_call.as_deref(), Some(p.call_id.as_str()));
    let vox = d.project.track(&id(&d, "Vox")).unwrap();
    // AI-7: trims towards −23 LUFS; AI-6/AUT-5: the vocal is louder in the chorus through an assistant lane.
    assert!(vox.chain.trim_db != 0.0);
    let lane = vox.lane("fader").expect("section lane");
    assert_eq!(lane.author, Author::Ai);
    let at = |f: i64| stemwise_core::automation::value_at(&lane.points, f, false).unwrap();
    assert!((at(4 * i64::from(RATE)) - at(i64::from(RATE)) - 1.5).abs() < 1e-9);
    // Numbered notes, one per track with a rationale.
    assert!(d.project.ai.notes.iter().any(|n| n.number == 1));
    // The bass is keyed by the kick (sidechain) and the delay follows no tempo: 375 ms.
    let bass = d.project.track(&id(&d, "Bass")).unwrap();
    assert_eq!(bass.chain.comp().unwrap().sidechain.as_ref().map(|s| s.source.clone()), Some(id(&d, "Kick")));
    // Undo restores everything in one step.
    d.undo(Phase::Mix).unwrap();
    assert_eq!(d.project, before_project);
    // AI-10: every call leaves its log; nothing private reaches the model.
    let folder = dir.join(&p.record.path);
    let names: Vec<String> =
        std::fs::read_dir(&folder).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    for f in
        ["analysis.json", "request-0-0.json", "response-0-0.json", "plan-0.json", "evaluation-0.json", "record.json"]
    {
        assert!(names.contains(&f.to_string()), "{f} in {names:?}");
    }
    let req = std::fs::read_to_string(folder.join("request-0-0.json")).unwrap();
    for private in ["secret-file", "Titolo Segreto", ".wav", dir.to_str().unwrap()] {
        assert!(!req.contains(private), "{private} leaked into the request");
    }
}

/// req: AI-3, AI-4, AI-5, AI-13
#[test]
fn revision_starts_from_manual_changes_and_respects_locks_and_scope() {
    let t = tmp();
    let (dir, mut d) = song(t.path(), false);
    let p = run_mix(&dir, &d.project, &FakeDecider, &|_, _| 0.0, &opts(1), &|_| {}, &AtomicBool::new(false)).unwrap();
    d.transact(Phase::Mix, "Apply", p.commands, None).unwrap();
    // The author pulls the vocal down by hand and locks the bass.
    let (vox, bass) = (id(&d, "Vox"), id(&d, "Bass"));
    d.transact(
        Phase::Mix,
        "f",
        vec![Command::SetParam { track: vox.clone(), param: "fader".into(), value: -5.0 }],
        None,
    )
    .unwrap();
    let mut b = d.project.track(&bass).unwrap().clone();
    b.locked = true;
    d.transact(Phase::Mix, "lock", vec![Command::SetTrack { track: Box::new(b) }], None).unwrap();
    let rev = MixOptions {
        max_iterations: 1,
        request: Some("Vocal louder and bass louder, more reverb on the guitars.".into()),
        scope: None,
    };
    let r = run_mix(&dir, &d.project, &FakeDecider, &|_, _| 0.0, &rev, &|_| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(r.iterations.len(), 1);
    assert!(r.summary.starts_with("Revision"), "English brief, English summary: {}", r.summary);
    let mut q = d.project.clone();
    for c in &r.commands {
        c.clone().apply(&mut q).unwrap();
    }
    // From the manual −5 dB, +2 dB (AI-3); the locked bass does not move (AI-4).
    assert_eq!(q.track(&vox).unwrap().chain.fader_db, -3.0);
    // The chorus lift of the auto mix survives the revision (B1 of the F5 review): still 1.5 dB over the verse.
    let lane = q.track(&vox).unwrap().lane("fader").expect("the section lane is kept");
    let at = |f: i64| stemwise_core::automation::value_at(&lane.points, f, false).unwrap();
    assert!((at(4 * i64::from(RATE)) - at(i64::from(RATE)) - 1.5).abs() < 1e-6);
    assert_eq!(q.track(&bass).unwrap(), d.project.track(&bass).unwrap());
    // Scope (AI-5): only the guitar may change.
    let gtr = id(&d, "Gtr");
    let scoped = MixOptions {
        max_iterations: 1,
        request: Some("vocal louder, guitars louder".into()),
        scope: Some([gtr.clone()].into()),
    };
    let r = run_mix(&dir, &d.project, &FakeDecider, &|_, _| 0.0, &scoped, &|_| {}, &AtomicBool::new(false)).unwrap();
    let touched: HashSet<String> = r
        .commands
        .iter()
        .filter_map(|c| if let Command::SetTrack { track } = c { Some(track.id.clone()) } else { None })
        .collect();
    assert!(touched.contains(&gtr) && !touched.contains(&vox), "{touched:?}");
    // Apply selected (AI-2).
    let only: HashSet<String> = [gtr.clone()].into();
    let part = partial_commands(&d.project, &r, &only);
    assert!(part.iter().all(|c| !matches!(c, Command::SetTrack { track } if track.id != gtr)));
}

/// A decider answering from a script: each call takes the next answer.
struct Scripted {
    answers: Mutex<Vec<Result<String, DecideError>>>,
    seen_feedback: Mutex<Vec<Option<String>>>,
}

impl Decider for Scripted {
    fn name(&self) -> &str {
        "scripted"
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
        _m: &[Message],
        _s: &Value,
        retry: Option<&Retry>,
        _c: &AtomicBool,
    ) -> Result<RawDecision, DecideError> {
        self.seen_feedback.lock().unwrap().push(retry.map(|r| r.errors.clone()));
        let next = self.answers.lock().unwrap().remove(0);
        let content = match next? {
            s if s == "FAKE" => serde_json::to_string(&FakeDecider.mix_plan(input)).unwrap(),
            s => s,
        };
        Ok(RawDecision {
            content,
            usage: Usage { tokens_in: 1000, tokens_out: 200, cost_usd: None },
            model: "test/model".into(),
            raw: "{}".into(),
            request: "{}".into(),
        })
    }
}

/// req: AI-8, AI-15
#[test]
fn invalid_plans_are_retried_with_the_errors_then_fail_with_their_cost() {
    let t = tmp();
    let (dir, d) = song(t.path(), false);
    let s = Scripted {
        answers: Mutex::new(vec![Ok("not json".into()), Ok("{\"version\": 2}".into()), Ok("FAKE".into())]),
        seen_feedback: Mutex::new(vec![]),
    };
    let p = run_mix(&dir, &d.project, &s, &|i, o| (i + o) as f64 * 1e-6, &opts(1), &|_| {}, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(p.record.attempts, 3);
    assert!(p.record.cost_estimated && (p.record.cost_usd - 0.0036).abs() < 1e-12, "{}", p.record.cost_usd);
    let fb = s.seen_feedback.lock().unwrap();
    assert!(fb[0].is_none() && fb[1].as_ref().unwrap().contains("not a valid plan") && fb[2].is_some());
    // Three invalid answers: the call fails, and what it spent is recorded.
    let s = Scripted {
        answers: Mutex::new(vec![Ok("x".into()), Ok("y".into()), Ok("z".into())]),
        seen_feedback: Mutex::new(vec![]),
    };
    let e = run_mix(&dir, &d.project, &s, &|_, _| 0.001, &opts(1), &|_| {}, &AtomicBool::new(false)).unwrap_err();
    let AiError::Failed { record, .. } = &e;
    assert_eq!((record.outcome, record.attempts), (AiOutcome::Failed, 3));
    assert!((record.cost_usd - 0.003).abs() < 1e-12);
}

/// req: AI-15, AI-12
#[test]
fn an_abandoned_request_counts_at_its_estimated_cost_and_a_late_failure_keeps_the_best_plan() {
    let t = tmp();
    let (dir, d) = song(t.path(), false);
    // B3 of the F5 review: cancelled while the provider was answering: the prompt's tokens are counted.
    let s = Scripted { answers: Mutex::new(vec![Err(DecideError::Abandoned)]), seen_feedback: Mutex::new(vec![]) };
    let e = run_mix(&dir, &d.project, &s, &|i, o| (i + o) as f64 * 1e-6, &opts(1), &|_| {}, &AtomicBool::new(false))
        .unwrap_err();
    let r = e.record();
    assert_eq!(r.outcome, AiOutcome::Cancelled);
    assert!(r.cost_estimated && r.cost_usd > 0.0 && r.tokens_in > 0, "{r:?}");
    assert!(dir.join(&r.path).join("record.json").exists(), "a cancelled call keeps its log");
    // A network error at a later iteration keeps the plan already evaluated (D-055 of the PoC).
    let s = Scripted {
        answers: Mutex::new(vec![Ok("FAKE".into()), Err(DecideError::Network("offline".into()))]),
        seen_feedback: Mutex::new(vec![]),
    };
    match run_mix(&dir, &d.project, &s, &|_, _| 0.0, &opts(3), &|_| {}, &AtomicBool::new(false)) {
        Ok(p) => {
            assert!(s.answers.lock().unwrap().is_empty(), "the second iteration was attempted");
            assert!(p.warnings.iter().any(|w| w.contains("keeping the best plan")), "{:?}", p.warnings);
            assert_eq!(p.iterations.len(), 1);
        }
        Err(e) => panic!("the evaluated plan was lost: {e}"),
    }
}

/// req: AI-10, AI-15
#[test]
fn a_cut_off_answer_fails_with_what_was_sent_and_received_in_the_log() {
    use stemwise_ai::decider::Exchange;
    let t = tmp();
    let (dir, d) = song(t.path(), false);
    let response = r#"{"choices":[{"finish_reason":"length","message":{"content":"{\"ver"}}]}"#;
    let cut = DecideError::Truncated {
        usage: Usage { tokens_in: 900, tokens_out: 4000, cost_usd: Some(0.05) },
        cap: Some(32_000),
        exchanges: vec![Exchange {
            request: r#"{"model":"test/model","max_tokens":32000}"#.into(),
            status: 200,
            response: response.into(),
        }],
    };
    let s = Scripted { answers: Mutex::new(vec![Err(cut)]), seen_feedback: Mutex::new(vec![]) };
    let e = run_mix(&dir, &d.project, &s, &|_, _| 0.0, &opts(1), &|_| {}, &AtomicBool::new(false)).unwrap_err();
    let r = e.record();
    assert_eq!(r.outcome, AiOutcome::Failed);
    assert_eq!((r.tokens_in, r.tokens_out, r.cost_usd), (900, 4000, 0.05), "what the provider reported is counted");
    let error = r.error.clone().unwrap();
    assert!(error.contains("cut off") && error.contains("4000") && !error.contains("refused"), "{error}");
    let folder = dir.join(&r.path);
    let logged: Value =
        serde_json::from_str(&std::fs::read_to_string(folder.join("request-0-0.json")).unwrap()).unwrap();
    assert!(logged["messages"].as_array().is_some_and(|m| m.len() == 2));
    assert_eq!(logged["exchanges"][0]["request"]["max_tokens"], 32_000, "the request as it was sent");
    assert_eq!(logged["exchanges"][0]["response"]["choices"][0]["finish_reason"], "length");
    assert_eq!(std::fs::read_to_string(folder.join("response-0-0.json")).unwrap(), response, "the raw answer");
}

/// A decider that answers like the fake one, then sets one track's fader: the author's request taken literally.
struct Fader {
    track: String,
    gain_db: f64,
    seen: Mutex<Vec<String>>,
}

impl Decider for Fader {
    fn name(&self) -> &str {
        "fader"
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
        _s: &Value,
        _r: Option<&Retry>,
        _c: &AtomicBool,
    ) -> Result<RawDecision, DecideError> {
        self.seen.lock().unwrap().push(m.iter().map(|x| x.content.clone()).collect::<Vec<_>>().join("\n"));
        let mut plan = FakeDecider.mix_plan(input);
        plan.tracks.get_mut(&input.ids.n(&self.track)).unwrap().gain_db = self.gain_db;
        Ok(RawDecision {
            content: serde_json::to_string(&plan).unwrap(),
            usage: Usage::default(),
            model: "test/model".into(),
            raw: "{}".into(),
            request: "{}".into(),
        })
    }
}

/// req: ANA-20
#[test]
fn the_levels_for_the_explanation_carry_the_mix_and_its_sections() {
    use stemwise_ai::balance::{measure_balance, measure_levels};
    let t = tmp();
    let (dir, d) = song(t.path(), false);
    let never = AtomicBool::new(false);
    let l = measure_levels(&dir, &d.project, &never).unwrap();
    // The mix and each of its sections have a loudness; sections come in time order, under their project ids.
    assert!(l.mix_lufs.is_some_and(|v| v > -70.0 && v < 0.0), "{:?}", l.mix_lufs);
    let mut secs: Vec<_> = d.project.sections.iter().collect();
    secs.sort_by_key(|s| s.start);
    assert_eq!(
        l.sections.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        secs.iter().map(|s| s.id.as_str()).collect::<Vec<_>>()
    );
    assert!(!l.sections.is_empty() && l.sections.iter().all(|s| s.lufs.is_some_and(|v| v > -70.0 && v < 0.0)));
    // Families and returns are the balance the assistant reads (AI-22), each family with one level per section.
    let b = measure_balance(&dir, &d.project, &never).unwrap();
    assert_eq!((&b.families, &b.returns), (&l.families, &l.returns));
    assert!(l.families.iter().all(|f| f.section_lu.len() == l.sections.len()));
    // Same project, same levels; a cancelled measure gives nothing instead of a wrong number.
    assert_eq!(measure_levels(&dir, &d.project, &never).as_ref(), Some(&l));
    assert!(measure_levels(&dir, &d.project, &AtomicBool::new(true)).is_none());
}

/// req: AI-22, AI-1, AI-3
#[test]
fn the_loop_reports_who_is_in_front_and_a_requested_revision_is_not_step_limited() {
    use stemwise_ai::balance::measure_balance;
    let t = tmp();
    let (dir, mut d) = song(t.path(), false);
    let never = AtomicBool::new(false);
    // The balance of the project as it is: one level per family that has tracks, per section too, relative to the
    // mix. 6 dB more on the guitar's fader is 6 LU more against the vocal (tolerance: the loudness meter's 0.1 LU
    // steps on a 6 s signal).
    let b = measure_balance(&dir, &d.project, &never).unwrap();
    let names: Vec<&str> = b.families.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["lead_vocal", "drums", "bass", "guitars"]);
    assert!(b.families.iter().all(|f| f.level_lu.is_some() && f.section_lu.len() == 2 && f.tracks.len() == 1));
    assert!(b.returns.iter().all(|r| r.level_lu.is_none()), "nothing sends to a return yet: silent, not a number");
    let lu =
        |b: &stemwise_ai::balance::Balance, n: &str| b.families.iter().find(|f| f.name == n).unwrap().level_lu.unwrap();
    let mut louder = d.project.clone();
    let gtr = id(&d, "Gtr");
    louder.tracks.iter_mut().find(|t| t.id == gtr).unwrap().chain.fader_db = 6.0;
    let b2 = measure_balance(&dir, &louder, &never).unwrap();
    let moved = (lu(&b2, "guitars") - lu(&b2, "lead_vocal")) - (lu(&b, "guitars") - lu(&b, "lead_vocal"));
    assert!((moved - 6.0).abs() < 0.2, "{moved}");
    // A cancelled measure gives nothing instead of a wrong number.
    assert!(measure_balance(&dir, &d.project, &AtomicBool::new(true)).is_none());

    // Auto mix: the model starts from the balance at unity and each evaluation carries the balance of its render,
    // returns included; the loop never ends at the first iteration, which has not seen one.
    let p = run_mix(&dir, &d.project, &FakeDecider, &|_, _| 0.0, &opts(3), &|_| {}, &never).unwrap();
    assert!(p.iterations.len() >= 2, "{:?}", p.iterations);
    let folder = dir.join(&p.record.path);
    let read = |n: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(folder.join(n)).unwrap()).unwrap() };
    let unity = &read("analysis.json")["balance_at_unity"];
    assert!(
        unity["families"]["drums"]["level_lu"].is_number() && unity["families"]["lead_vocal"]["tracks"][0].is_string()
    );
    let e0 = read("evaluation-0.json");
    assert!(e0["balance"]["families"]["guitars"]["section_lu"].as_array().is_some_and(|a| a.len() == 2));
    assert!(e0["balance"]["returns_lu"].as_object().is_some_and(|r| !r.is_empty()), "{e0}");
    let text = std::fs::read_to_string(folder.join("analysis.json")).unwrap() + &e0.to_string();
    assert!(!text.contains(&gtr) || gtr.starts_with('t'), "neutral ids only");
    d.transact(Phase::Mix, "Apply", p.commands, None).unwrap();

    // Revision: "the vocal as loud as it takes" is one step, not ±4 dB at a time, and the model sees the balance of
    // the mix it starts from.
    let vox = id(&d, "Vox");
    let from = d.project.track(&vox).unwrap().chain.fader_db;
    assert!(from < 5.0, "the test needs room above the limit: {from}");
    let ask = Fader { track: vox.clone(), gain_db: from + 7.0, seen: Mutex::new(vec![]) };
    let rev = MixOptions { max_iterations: 1, request: Some("vocal much louder".into()), scope: None };
    let r = run_mix(&dir, &d.project, &ask, &|_, _| 0.0, &rev, &|_| {}, &never).unwrap();
    assert!(r.warnings.iter().all(|w| !w.contains("clipped")), "{:?}", r.warnings);
    let mut q = d.project.clone();
    for c in &r.commands {
        c.clone().apply(&mut q).unwrap();
    }
    assert_eq!(q.track(&vox).unwrap().chain.fader_db, from + 7.0);
    let seen = ask.seen.lock().unwrap();
    assert!(seen[0].contains("\"balance\"") && seen[0].contains("level_lu"), "the starting balance is in the message");
    assert!(seen[0].contains("do not apply here"), "the prompt says the step limit is off for a revision");
    // Between the iterations of an auto mix the limit still holds (DEC-5).
    let swing = Fader { track: vox, gain_db: 11.0, seen: Mutex::new(vec![]) };
    let p = run_mix(&dir, &d.project, &swing, &|_, _| 0.0, &opts(2), &|_| {}, &never).unwrap();
    assert!(p.iterations.len() == 2 && swing.seen.lock().unwrap()[1].contains("\"balance\""));
}

/// req: AI-12, AI-11
#[test]
fn cancel_and_missing_key_end_the_call_cleanly() {
    let t = tmp();
    let (dir, d) = song(t.path(), false);
    let e =
        run_mix(&dir, &d.project, &FakeDecider, &|_, _| 0.0, &opts(3), &|_| {}, &AtomicBool::new(true)).unwrap_err();
    assert_eq!(e.record().outcome, AiOutcome::Cancelled);
    let s = Scripted { answers: Mutex::new(vec![Err(DecideError::NoKey)]), seen_feedback: Mutex::new(vec![]) };
    let e = run_mix(&dir, &d.project, &s, &|_, _| 0.0, &opts(1), &|_| {}, &AtomicBool::new(false)).unwrap_err();
    assert!(e.to_string().contains("API key"), "{e}");
}

fn master_input<'a>(
    samples: &'a [f32],
    measures: &'a stemwise_analysis::measures::MixMeasures,
    target: Option<&'a stemwise_analysis::measures::Target>,
) -> stemwise_ai::mastering::MasterInput<'a> {
    stemwise_ai::mastering::MasterInput {
        samples,
        rate: RATE,
        measures,
        target,
        references: vec![(Some(-9.5), -0.8)],
        start_gain_db: None,
    }
}

/// A quiet body with a short, tall hit twice a second: peaks far above the loudness, as an unmastered rock mix has.
fn peaky_mixdown(seconds: usize) -> Vec<f32> {
    let n = seconds * RATE as usize;
    let noise = stemwise_dsp::testsignal::xorshift32(5, 2 * n);
    let mut x = Vec::with_capacity(2 * n);
    for i in 0..n {
        let t = i as f64 / f64::from(RATE);
        let body = 0.05 * (std::f64::consts::TAU * 220.0 * t).sin() + 0.02 * noise[2 * i];
        let since = (t * 2.0).fract() / 2.0;
        let hit = 0.65 * (-since * 400.0).exp() * (std::f64::consts::TAU * 180.0 * since).cos();
        x.push((body + hit) as f32);
        x.push((body * 0.9 + hit + 0.02 * noise[2 * i + 1]) as f32);
    }
    x
}

/// req: MAS-6, AI-10
#[test]
fn the_master_loop_measures_each_plan_and_the_next_decision_corrects_it() {
    use stemwise_ai::mastering::{MasterEvaluation, master_document, measure_mixdown, run_master};
    let t = tmp();
    let (dir, mut d) = song(t.path(), false);
    let mut m = d.project.master.clone();
    m.target = stemwise_core::model::LoudnessTarget { preset: "custom".into(), lufs: -9.0, true_peak_db: -1.0 };
    d.transact(Phase::Master, "target", vec![Command::SetMaster { master: Box::new(m) }], None).unwrap();
    let x = peaky_mixdown(8);
    let mixdown = measure_mixdown(&x, RATE);
    let input = master_input(&x, &mixdown, None);
    // What the model reads: the room between peaks and body, how much of it the destination takes away, and the
    // current master as measured. The loudness range is named for what it is.
    let mix = input.dynamics();
    let current = stemwise_ai::mastering::evaluate_master(&input, &mix, &d.project, &d.project.master, None).unwrap();
    let doc = master_document(&input, &mix, Some(&current), &d.project);
    let plr = doc["mixdown"]["peak_to_loudness_db"].as_f64().unwrap();
    assert!(plr > 14.0, "the hits stand far above the body: {plr}");
    assert_eq!(doc["destination"]["peak_to_loudness_db"], 8.0);
    assert!((doc["destination"]["peak_reduction_needed_db"].as_f64().unwrap() - (plr - 8.0)).abs() < 0.11);
    assert!(doc["mixdown"]["peak_share_pct"][2].as_f64().unwrap() < 2.0, "few isolated peaks");
    assert!(doc["mixdown"].get("dyn_range_lu").is_none() && doc["mixdown"].get("loudness_range_lu").is_some());
    assert_eq!(doc["references"][0]["peak_to_loudness_db"], 8.7);
    assert!(doc["current_master"]["limiter"]["time_over_3_db_pct"].as_f64().unwrap() > 5.0);
    assert!(current.on_target, "{current:?}");

    let p = run_master(&dir, &d.project, &input, &FakeDecider, &|_, _| 0.0, None, &|_| {}, &AtomicBool::new(false))
        .unwrap();
    let call = dir.join(&p.record.path);
    let eval = |i: usize| -> MasterEvaluation {
        serde_json::from_str(&std::fs::read_to_string(call.join(format!("evaluation-{i}.json"))).unwrap()).unwrap()
    };
    let (first, second) = (eval(0), eval(1));
    // The second decision saw the limiter's work and added a stage before it: the limiter now works less.
    use stemwise_ai::plan::MasterBlockPlan as B;
    assert!(p.plan.chain.iter().any(|b| matches!(b, B::Clipper { .. })), "{:?}", p.plan);
    // The clipper is the block the master already had, switched on, and the evaluation says what it does.
    let clip = d.project.master.chain.first(stemwise_core::model::BlockKind::Clipper).unwrap().id.clone();
    assert!(!p.preview.chain.block(&clip).unwrap().block.bypass());
    let seen = second.blocks.iter().find(|b| b.id == clip).expect("the clipper is measured");
    assert!(seen.kind == "clipper" && seen.reduction.max_db > 1.0 && seen.reduction.time_active_pct < 20.0, "{seen:?}");
    assert!(p.changes.iter().any(|c| c.key == format!("block:{clip}") && c.state == "on" && !c.rationale.is_empty()));
    assert!(second.limiter.max_db < first.limiter.max_db - 1.0, "{first:?}\n{second:?}");
    assert!(second.limiter.time_over_3_db_pct < first.limiter.time_over_3_db_pct);
    assert!(second.on_target);
    // The third decision confirmed the plan: nothing more was rendered, and that plan is the proposal.
    assert_eq!(p.record.iterations, 3);
    assert!(!call.join("evaluation-2.json").exists());
    assert_eq!(p.evaluation.as_ref(), Some(&second));
    let kept: stemwise_ai::plan::MasterPlan =
        serde_json::from_str(&std::fs::read_to_string(call.join("plan.json")).unwrap()).unwrap();
    assert_eq!(kept, p.plan);
    // The second request carried the first plan with its evaluation.
    let req = std::fs::read_to_string(call.join("request-1-0.json")).unwrap();
    assert!(req.contains("time_over_3_db_pct") && req.contains("History"), "{req}");
    // The destination stays the author's (A-035).
    assert_eq!(p.preview.target.lufs, -9.0);
}

/// req: MAS-6
#[test]
fn a_request_on_an_untouched_master_asks_for_a_whole_master_and_on_a_worked_one_for_a_change() {
    use stemwise_ai::mastering::{measure_mixdown, run_master};
    let t = tmp();
    let (dir, mut d) = song(t.path(), false);
    let x = peaky_mixdown(4);
    let mixdown = measure_mixdown(&x, RATE);
    let input = master_input(&x, &mixdown, None);
    let ask = |d: &Document| {
        let p = run_master(
            &dir,
            &d.project,
            &input,
            &FakeDecider,
            &|_, _| 0.0,
            Some("more punch".into()),
            &|_| {},
            &AtomicBool::new(false),
        )
        .unwrap();
        (std::fs::read_to_string(dir.join(&p.record.path).join("request-0-0.json")).unwrap(), p)
    };
    let (first, p) = ask(&d);
    assert!(!first.contains("Starting plan") && first.contains("following the author's request"), "{first}");
    assert!(first.contains("more punch"));
    d.transact(Phase::Master, "Apply master", p.commands.clone(), Some(p.call_id.clone())).unwrap();
    let mut m = d.project.master.clone();
    m.chain.blocks[2].block.set_bypass(false);
    d.transact(Phase::Master, "comp", vec![Command::SetMaster { master: Box::new(m) }], None).unwrap();
    let (second, _) = ask(&d);
    assert!(second.contains("Starting plan (what the author listened to)"), "{second}");
}

/// req: MAS-6, MAS-12, MAS-14, MAS-15, AI-2, AI-8
#[test]
fn a_master_plan_orders_adds_and_removes_blocks_and_each_change_applies_on_its_own() {
    use stemwise_ai::analysis_doc::Ids;
    use stemwise_ai::mastering::{master_changes, master_from_plan, master_partial, plan_from_master};
    use stemwise_ai::plan::{MasterBlockPlan as B, MasterPlan, MasterSectionPlan, MbBandPlan, SectionEqPlan};
    use stemwise_ai::validate::validate_master;
    use stemwise_core::model::{Block, BlockKind, Section};
    let t = tmp();
    let (_, d) = song(t.path(), false);
    let mut project = d.project.clone();
    project.sections = vec![Section { id: "sec-a".into(), name: "Chorus".into(), color: 1, start: 0, end: 48000 }];
    let ids = Ids::of(&project);
    let old = project.master.clone();
    let id = |k: BlockKind| old.chain.first(k).unwrap().id.clone();
    // A new master is the limiter alone: its plan lists no block.
    assert!(plan_from_master(&old, &ids).chain.is_empty());
    let band = |ratio: f64| MbBandPlan { threshold_db: -14.0, ratio, attack_ms: 20.0, release_ms: 160.0, gain_db: 0.0 };
    let eq = |id: &str, hz: f64| B::Eq {
        id: id.into(),
        bands: vec![stemwise_ai::plan::EqBand {
            kind: stemwise_ai::plan::EqType::HighShelf,
            freq_hz: hz,
            gain_db: 1.0,
            q: 0.7,
        }],
        why: "tone".into(),
    };
    let plan: MasterPlan = MasterPlan {
        version: 2,
        summary: "x".into(),
        target: stemwise_ai::plan::MasterTarget { lufs: -14.0, true_peak_db: -1.0 },
        input_gain_db: 0.0,
        chain: vec![
            B::Compressor {
                id: id(BlockKind::Compressor),
                threshold_db: -9.0,
                ratio: 3.0,
                attack_ms: 3.0,
                release_ms: 90.0,
                knee_db: 4.0,
                makeup_db: 0.0,
                mix: 0.6,
                listen_above_hz: 120.0,
                why: "peaks".into(),
            },
            eq(&id(BlockKind::Eq), 9000.0),
            B::Multiband {
                id: id(BlockKind::Multiband),
                crossovers_hz: vec![100.0, 800.0, 5000.0],
                bands: vec![band(2.5), band(1.0), band(1.0), band(1.0)],
                why: "low end".into(),
            },
            eq("second", 12000.0),
            B::Clipper { id: id(BlockKind::Clipper), take_off_db: 4.0, softness: 0.4, why: "few peaks".into() },
        ],
        limiter: stemwise_ai::plan::LimiterPlan { release_ms: 120.0 },
        sections: vec![MasterSectionPlan {
            section: "s1".into(),
            level_db: 0.5,
            side_gain_db: 0.0,
            limiter_release_ms: 0.0,
            eq: vec![SectionEqPlan { block: "second".into(), gains_db: vec![0.6] }],
        }],
        rationale: [("order".to_string(), "compressor first".to_string())].into(),
    };
    assert!(
        validate_master(&plan, RATE, &["s1".to_string()]).ok(),
        "{:?}",
        validate_master(&plan, RATE, &["s1".into()])
    );
    let new = master_from_plan(&old, &plan, &ids);
    assert!(new.check().is_ok(), "{:?}", new.check());
    // The plan's blocks first, in its order and on; the rest of the master after them, off.
    let kinds: Vec<(BlockKind, bool)> = new.chain.blocks.iter().map(|b| (b.block.kind(), !b.block.bypass())).collect();
    use BlockKind as K;
    assert_eq!(
        kinds,
        [
            (K::Compressor, true),
            (K::Eq, true),
            (K::Multiband, true),
            (K::Eq, true),
            (K::Clipper, true),
            (K::MatchEq, false),
            (K::Width, false),
            (K::Saturation, false)
        ]
    );
    assert_eq!(new.chain.blocks[0].id, id(K::Compressor), "a block the master had keeps its id");
    let added = new.chain.blocks[3].id.clone();
    assert!(old.chain.block(&added).is_none() && new.chain.name(&added).as_deref() == Some("EQ 2"));
    match &new.chain.blocks[2].block {
        Block::Multiband(m) => assert_eq!((m.bands.len(), m.crossovers_hz[1]), (4, 800.0)),
        other => panic!("{other:?}"),
    }
    // The section offsets follow the new block's real id, and the section's real id.
    assert_eq!((new.sections[0].section.as_str(), new.sections[0].eq[0].block.as_str()), ("sec-a", added.as_str()));
    assert_eq!(new.sections[0].eq[0].gains_db, [0.6, 0.0, 0.0, 0.0, 0.0, 0.0]);
    // Back to a plan and to a state again: the same master.
    let again = master_from_plan(&new, &plan_from_master(&new, &ids), &ids);
    assert_eq!(again, new);
    // The changes, entry by entry.
    let changes = master_changes(&old, &new, &plan);
    let state = |key: &str| changes.iter().find(|c| c.key == key).map(|c| c.state.as_str());
    assert_eq!(state(&format!("block:{added}")), Some("new"));
    assert_eq!(state(&format!("block:{}", id(K::Compressor))), Some("on"));
    assert_eq!(state("sections"), Some("changed"));
    assert_eq!(state("limiter"), Some("changed"));
    assert!(changes.iter().find(|c| c.key == "order").is_none(), "nothing was on before: no order to change");
    // Only the compressor and the new EQ: the rest of the master stays as it was.
    let part = master_partial(&old, &new, &[format!("block:{}", id(K::Compressor)), format!("block:{added}")]);
    assert!(part.check().is_ok());
    assert!(!part.chain.block(&id(K::Compressor)).unwrap().block.bypass());
    assert!(part.chain.block(&added).is_some() && part.chain.count(K::Eq) == 2);
    assert!(part.chain.block(&id(K::Clipper)).unwrap().block.bypass() && part.sections.is_empty());
    assert_eq!(part.chain.limiter.release_ms, 80.0);
    // The order alone, on a master that has those blocks on.
    let reordered = master_partial(
        &new,
        &{
            let mut m = new.clone();
            m.chain.blocks.swap(0, 1);
            m
        },
        &["order".into()],
    );
    assert_eq!(reordered.chain.blocks[0].block.kind(), K::Eq);
    let moved = master_changes(&new, &reordered, &plan);
    assert!(moved.iter().any(|c| c.key == "order"));
    // What a plan may not hold.
    let bad = |f: &dyn Fn(&mut MasterPlan)| {
        let mut p = plan.clone();
        f(&mut p);
        validate_master(&p, RATE, &["s1".to_string()]).feedback()
    };
    assert!(bad(&|p| p.chain.push(eq("third", 5000.0))).contains("EQ blocks: 2 at most"));
    assert!(
        bad(&|p| p.chain[4] = B::Clipper { id: "c".into(), take_off_db: 9.0, softness: 0.4, why: String::new() })
            .contains("clipper.take_off_db")
    );
    assert!(
        bad(&|p| {
            if let B::Multiband { crossovers_hz, .. } = &mut p.chain[2] {
                crossovers_hz[1] = 110.0;
            }
        })
        .contains("third of an octave")
    );
    assert!(bad(&|p| p.sections[0].section = "s9".into()).contains("not a section"));
    assert!(bad(&|p| p.sections[0].eq[0].block = id(K::Clipper)).contains("not an EQ block"));
    assert!(bad(&|p| p.chain[3] = eq(&id(K::Eq), 5000.0)).contains("share the id"));
}

/// req: AI-23, MAS-6
#[test]
fn a_master_plan_of_the_first_contract_is_still_read() {
    use stemwise_ai::mastering::restore_master;
    use stemwise_core::model::{AiCallKind, BlockKind};
    let t = tmp();
    let (dir, d) = song(t.path(), false);
    let call = dir.join("ai/old-master");
    std::fs::create_dir_all(&call).unwrap();
    std::fs::write(
        call.join("plan-0.json"),
        r#"{"version": 1, "summary": "Old plan.", "target": {"lufs": -14.0, "true_peak_db": -1.0}, "input_gain_db": 0.0,
          "eq": [{"type": "high_shelf", "freq_hz": 10000, "gain_db": 1.0, "q": 0.7}], "match": null,
          "compressor": {"threshold_db": -14.0, "ratio": 1.8, "attack_ms": 30.0, "release_ms": 200.0, "knee_db": 6.0, "makeup_db": 0.0},
          "width": null, "saturation": {"drive_db": 3.0, "mix": 1.0}, "limiter": {"release_ms": 100.0},
          "rationale": {"eq": "air", "limiter": "smooth"}}"#,
    )
    .unwrap();
    let mut record = stemwise_ai::log::record(
        "old-master",
        AiCallKind::Master,
        0,
        "m",
        "e",
        "p",
        &Default::default(),
        1,
        stemwise_core::model::AiOutcome::Discarded,
        None,
        None,
    );
    record.path = "ai/old-master".into();
    let p = restore_master(&dir, &d.project, &record).unwrap();
    let on: Vec<BlockKind> =
        p.preview.chain.blocks.iter().filter(|b| !b.block.bypass()).map(|b| b.block.kind()).collect();
    assert_eq!(on, [BlockKind::Eq, BlockKind::Compressor, BlockKind::Saturation]);
    assert_eq!(p.preview.chain.limiter.release_ms, 100.0);
    assert!(p.changes.iter().any(|c| c.rationale == "air"));
}

/// req: MAS-6, AI-2, AI-8, AI-13
#[test]
fn master_ai_proposes_a_valid_plan_and_revises_it_by_request() {
    use stemwise_ai::mastering::{measure_mixdown, run_master};
    let t = tmp();
    let (dir, mut d) = song(t.path(), true);
    let r = stemwise_engine::offline::render_song(&dir, &d.project, 1.0).unwrap();
    let x = r.interleaved();
    let mixdown = measure_mixdown(&x, r.rate);
    let a = stemwise_ai::analysis_doc::analyze_project(&dir, &d.project, &|_| true).unwrap();
    let input = master_input(&x, &mixdown, a.target.as_ref());
    let p = run_master(&dir, &d.project, &input, &FakeDecider, &|_, _| 0.0, None, &|_| {}, &AtomicBool::new(false))
        .unwrap();
    assert!(stemwise_ai::validate::validate_master(&p.plan, RATE, &[]).ok());
    assert!(p.summary.starts_with("Master verso"), "Italian brief: {}", p.summary);
    assert_eq!(p.preview.target.lufs, -14.0);
    use stemwise_ai::plan::MasterBlockPlan as B;
    assert!(p.plan.chain.iter().any(|b| matches!(b, B::MatchEq { .. })), "a reference exists: match EQ is proposed");
    d.transact(Phase::Master, "Apply master", p.commands.clone(), Some(p.call_id.clone())).unwrap();
    assert_eq!(d.project.master, p.preview);
    // A revision by request (D-077 of the PoC): brighter and louder.
    let r2 = run_master(
        &dir,
        &d.project,
        &input,
        &FakeDecider,
        &|_, _| 0.0,
        Some("più brillante e più forte".into()),
        &|_| {},
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(r2.preview.target.lufs, -13.0);
    assert!(r2.diff.rows.iter().any(|row| row.label == "Target"));
    assert!(r2.preview.chain.blocks.iter().any(|b| matches!(&b.block, stemwise_core::model::Block::Eq(eq)
        if !eq.bypass && eq.bands.iter().any(|x| x.enabled && x.kind == stemwise_core::model::EqKind::HighShelf))));
    // Master history is separate from the mix (HIS-5).
    assert_eq!(d.history(Phase::Master).position(), 1);
}

/// req: AI-23, AI-2, HIS-3
#[test]
fn a_proposal_received_earlier_is_reviewed_and_applied_again_on_the_mix_as_it_is_now() {
    use stemwise_ai::mixing::restore_proposal;
    let t = tmp();
    let (dir, mut d) = song(t.path(), false);
    let run = |d: &Document| {
        run_mix(&dir, &d.project, &FakeDecider, &|_, _| 0.0, &opts(2), &|_| {}, &AtomicBool::new(false)).unwrap()
    };
    let p = run(&d);
    assert!(p.record.stored, "the proposal is kept with its call");
    assert!(dir.join(&p.record.path).join("proposal.json").is_file());
    // Discarded at the time: nothing applied. Later, from the record alone, the same proposal comes back.
    let fresh = d.project.clone();
    let again = restore_proposal(&dir, &d.project, &p.record).unwrap();
    assert_eq!(again.plan, p.plan);
    assert_eq!(again.commands, p.commands, "on an unchanged project it is the proposal as received");
    assert_eq!(again.diff, p.diff);
    assert_eq!(again.preview, p.preview);
    assert_eq!(again.summary, p.summary);
    // The evaluations are those measured at the time, read back from JSON: equal within the last digit of a
    // decimal round trip (1e-9 dB), far below what the review shows (0.1 dB).
    for (a, b) in [(&again.before, &p.before), (&again.after, &p.after)] {
        let (a, b) = (a.as_ref().unwrap(), b.as_ref().unwrap());
        assert!((a.spectral_distance_db.unwrap() - b.spectral_distance_db.unwrap()).abs() < 1e-9);
        assert!((a.lufs.unwrap() - b.lufs.unwrap()).abs() < 1e-9);
        assert_eq!(a.constraints, b.constraints);
    }
    // Applied, then the author changes a fader and removes a track: the proposal is still there for the mix as
    // it is now.
    d.transact(Phase::Mix, "Apply auto mix", p.commands.clone(), Some(p.call_id.clone())).unwrap();
    let applied = d.project.clone();
    let vox = id(&d, "Vox");
    let mut louder = d.project.track(&vox).unwrap().clone();
    louder.chain.fader_db += 4.0;
    d.transact(Phase::Mix, "fader", vec![Command::SetTrack { track: Box::new(louder) }], None).unwrap();
    let cmds = edit::delete_track(&d.project, &id(&d, "Gtr")).unwrap();
    d.transact(Phase::Mix, "delete", cmds, None).unwrap();
    let again = restore_proposal(&dir, &d.project, &p.record).unwrap();
    // The diff is against today's mix: the fader moved by hand goes back, the track that is gone is left out
    // and said so; the returns the first application created are not created twice.
    let row = again.diff.iter().find(|x| x.name == "Vox").expect("the vocal changes back");
    assert!(
        row.rows.iter().any(|r| r.label.to_lowercase().contains("fader") || r.label.to_lowercase().contains("gain")),
        "{row:?}"
    );
    assert!(!again.diff.iter().any(|x| x.name == "Gtr"));
    assert!(
        again.warnings.iter().any(|w| w.contains("1 track") && w.contains("no longer in the project")),
        "{:?}",
        again.warnings
    );
    assert!(!again.commands.iter().any(|c| matches!(c, Command::InsertTrack { .. })));
    let before = d.project.clone();
    d.transact(Phase::Mix, "Apply auto mix", again.commands.clone(), Some(p.call_id.clone())).unwrap();
    assert_eq!(d.project.track(&vox).unwrap().chain, applied.track(&vox).unwrap().chain, "the vocal is as proposed");
    assert_eq!(d.project.tracks.len(), before.tracks.len());
    // One step, like the first time.
    d.undo(Phase::Mix).unwrap();
    assert_eq!(d.project, before);
    // A call from before proposals were kept: said plainly, nothing reopened.
    std::fs::remove_file(dir.join(&p.record.path).join("proposal.json")).unwrap();
    assert!(restore_proposal(&dir, &fresh, &p.record).unwrap_err().contains("was not kept"));
}

/// req: AI-23, MAS-6
#[test]
fn a_master_proposal_received_earlier_comes_back_on_the_master_as_it_is_now() {
    use stemwise_ai::mastering::{measure_mixdown, restore_master, run_master};
    let t = tmp();
    let (dir, mut d) = song(t.path(), false);
    let r = stemwise_engine::offline::render_song(&dir, &d.project, 1.0).unwrap();
    let x = r.interleaved();
    let mixdown = measure_mixdown(&x, r.rate);
    let input = master_input(&x, &mixdown, None);
    let p = run_master(&dir, &d.project, &input, &FakeDecider, &|_, _| 0.0, None, &|_| {}, &AtomicBool::new(false))
        .unwrap();
    assert!(p.record.stored);
    let again = restore_master(&dir, &d.project, &p.record).unwrap();
    assert_eq!((&again.plan, &again.preview, &again.diff), (&p.plan, &p.preview, &p.diff));
    // Applied, then changed by hand: the same proposal brings the chain back, with a diff against today.
    d.transact(Phase::Master, "Apply master", p.commands.clone(), Some(p.call_id.clone())).unwrap();
    let mut m = d.project.master.clone();
    m.chain.input_gain_db += 3.0;
    d.transact(Phase::Master, "input", vec![Command::SetMaster { master: Box::new(m) }], None).unwrap();
    let again = restore_master(&dir, &d.project, &p.record).unwrap();
    assert_eq!(again.preview.chain, p.preview.chain);
    assert!(!again.diff.rows.is_empty(), "today's input gain differs from the proposal's");
    assert_eq!(again.record.id, p.record.id);
}
