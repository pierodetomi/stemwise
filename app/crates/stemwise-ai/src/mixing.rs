//! The AI mix (AI-1 … AI-8, AI-10, AI-12): auto mix with the verification loop and revision by request.
//!
//! Auto mix: analysis (cached) → 2 or 3 iterations of decision (≤ 2 retries with the validation errors), offline
//! render of the plan applied to a working copy, evaluation against the target and balance of families and returns
//! (AI-22), stop rules (never at the first iteration; then below 1.5 dB, less than 0.2 dB of improvement, maximum
//! iterations; without a target a valid plan) and the plan kept (the latest valid one, unless more than 1 dB
//! further from the target than an earlier one). Revision: one iteration from the current state with the
//! request as top priority and no rollback. The result is a [`MixProposal`]: it changes nothing until applied
//! (AI-2), and applying it is one history step (HIS-3).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use stemwise_analysis::evaluate::{Evaluation, evaluate};
use stemwise_analysis::measures;
use stemwise_core::command::Command;
use stemwise_core::edit;
use stemwise_core::model::{AiCallKind, AiCallRecord, AiOutcome, Project, ReturnFx};
use stemwise_engine::offline::render_range;

use crate::analysis_doc::{Ids, ProjectAnalysis, analyze_project, model_document};
use crate::apply::{ApplyOptions, TrackDiff, apply_mix_plan, diff, plan_from_project};
use crate::balance::{balance_json, measure_balance};
use crate::decider::{DecideError, Decider, DecisionInput, Kind, Retry, prompt_estimate};
use crate::log::{self, Meter};
use crate::plan::{Editable, MixPlan, mix_plan_schema_for};
use crate::prompt::{DISTANCE_THRESHOLD_DB, MIN_IMPROVEMENT_DB, SPECTRAL_TOLERANCE_DB, build_messages};
use crate::structure::{self, Allow, NewElement, Skipped};
use crate::validate::{limit_changes, validate_mix};

/// The mix bus ceiling used by the evaluation, as for the mixdown (MDN-1).
pub const MIX_CEILING_DB: f64 = -3.0;
pub const MAX_RETRIES: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum AiPhase {
    Analysis,
    Decision,
    Render,
    Evaluation,
}

/// Progress of a call, per phase and iteration (AI-12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct AiProgress {
    pub phase: AiPhase,
    pub iteration: u32,
    pub of: u32,
    pub fraction: f64,
}

#[derive(Debug, Clone, Default)]
pub struct MixOptions {
    pub max_iterations: u32,
    /// AI-5: the tracks the request may change (project ids); `None` = all unlocked.
    pub scope: Option<HashSet<String>>,
    /// AI-3: a revision request.
    pub request: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct IterationSummary {
    pub iteration: u32,
    pub distance_db: Option<f64>,
    pub valid: bool,
    pub best: bool,
}

/// A proposal in review (AI-2): nothing is applied yet.
#[derive(Debug, Clone)]
pub struct MixProposal {
    pub call_id: String,
    pub kind: AiCallKind,
    pub plan: MixPlan,
    pub ids: Ids,
    /// All commands of "apply all" (returns created if missing, chains, lanes, trims, notes).
    pub commands: Vec<Command>,
    /// Commands that must precede any partial application (returns the plan relies on).
    pub prelude: Vec<Command>,
    /// Returns and groups the plan creates (AI-17), already part of `commands` and of `preview`.
    pub new_elements: Vec<NewElement>,
    pub trims: HashMap<String, f64>,
    /// The scope of the request (AI-5), kept so a later application leaves the same tracks alone.
    pub scope: Option<HashSet<String>>,
    pub diff: Vec<TrackDiff>,
    pub summary: String,
    pub before: Option<Evaluation>,
    pub after: Option<Evaluation>,
    pub iterations: Vec<IterationSummary>,
    pub warnings: Vec<String>,
    pub record: AiCallRecord,
    /// The project with the proposal applied (for A/B listening).
    pub preview: Project,
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("{message}")]
    Failed { message: String, record: Box<AiCallRecord> },
}

impl AiError {
    pub fn record(&self) -> &AiCallRecord {
        match self {
            AiError::Failed { record, .. } => record,
        }
    }
}

/// Renders the project offline and evaluates its mix, trimmed to the ceiling like a mixdown.
pub fn evaluate_project(
    package: &Path,
    p: &Project,
    target: Option<&measures::Target>,
    cancel: &AtomicBool,
) -> Option<Evaluation> {
    let end = p.content_end() + i64::from(p.sample_rate?) * 2;
    let r = render_range(package, p, 0, end, 4096, &mut |_| !cancel.load(Ordering::Relaxed)).ok()?;
    let x = r.interleaved();
    let tp = stemwise_dsp::truepeak::true_peak_db(&x, 2);
    let g = 10f64.powf(((MIX_CEILING_DB - 0.01) - tp).min(0.0) / 20.0);
    let x: Vec<f32> = x.iter().map(|v| (f64::from(*v) * g) as f32).collect();
    let (m, _) = measures::measure_mix(&x, 2, r.rate);
    Some(evaluate(&m, measures::dc_offset_pct(&x, 2), target, MIX_CEILING_DB))
}

fn eval_json(e: &Evaluation, it: u32, best: bool) -> Value {
    let r1 = crate::analysis_doc::r1;
    json!({
        "iteration": it,
        "spectral_distance_db": e.spectral_distance_db.map(r1),
        "band_delta_db": e.band_delta_db.map(|b| b.map(r1)),
        "width_delta_db": e.width_delta_db.map(r1),
        "dyn_delta_lu": e.dyn_delta_lu.map(r1),
        "constraints": {"true_peak_ok": e.constraints.true_peak_ok, "low_corr_ok": e.constraints.low_corr_ok, "dc_ok": e.constraints.dc_ok},
        "is_best": best,
    })
}

/// Returns the plan relies on: a reverb and a delay are created if missing (§5.5).
/// The analysis says `locked` for everything the request may not touch, as the track list does.
fn mark_locked(doc: &mut Value, editable: &[String]) {
    for section in ["tracks", "groups"] {
        if let Some(Value::Object(o)) = doc.get_mut(section) {
            for (n, v) in o.iter_mut() {
                v["locked"] = json!(!editable.contains(n));
            }
        }
    }
    if let Some(Value::Array(a)) = doc.get_mut("returns") {
        for v in a.iter_mut() {
            let n = v["id"].as_str().unwrap_or_default().to_string();
            v["locked"] = json!(!editable.contains(&n));
        }
    }
}

pub fn ensure_returns(p: &mut Project) -> Vec<Command> {
    let mut cmds = Vec::new();
    let has =
        |p: &Project, reverb: bool| p.tracks.iter().any(|t| t.ret.as_ref().is_some_and(|r| r.is_reverb() == reverb));
    for (reverb, fx) in [(true, ReturnFx::default_reverb()), (false, ReturnFx::default_delay())] {
        if !has(p, reverb)
            && let Ok(c) = edit::add_return(p, fx)
        {
            for cmd in c {
                if cmd.clone().apply(p).is_ok() {
                    cmds.push(cmd);
                }
            }
        }
    }
    cmds
}

fn tenth(d: Option<f64>) -> Option<i64> {
    d.map(|x| (x * 10.0).round() as i64)
}

/// Whether an iteration's plan replaces the one kept so far (EVA-6 of the PoC, changed by A-063): only plans that
/// respect the constraints. A later plan was decided knowing the balance of the earlier one, which the spectral
/// distance does not see, so it wins unless it moves away from the target by more than [`SPECTRAL_TOLERANCE_DB`]
/// (in tenths of dB as written). Without a target the later valid plan wins.
pub fn is_better(cand: &Evaluation, best: Option<&Evaluation>) -> bool {
    cand.valid()
        && match best {
            None => true,
            Some(b) => match (tenth(cand.spectral_distance_db), tenth(b.spectral_distance_db)) {
                (Some(x), Some(y)) => x <= y + (SPECTRAL_TOLERANCE_DB * 10.0).round() as i64,
                (None, None) => true,
                _ => false,
            },
        }
}

/// Stop rules of the loop (EVA-5 of the PoC, changed by A-063), on distances in tenths of dB as written: a revision
/// runs once; an auto mix never stops at its first iteration (`done` = iterations completed), because only the
/// second one has seen the balance of a rendered plan; then without a target a valid plan ends it; below 1.5 dB
/// with a valid plan; less than 0.2 dB of improvement over the previous iteration.
pub fn should_stop(revision: bool, done: u32, valid: bool, distance: Option<i64>, last: Option<i64>) -> bool {
    if revision {
        return true;
    }
    if done < 2 {
        return false;
    }
    let Some(d) = distance else { return valid };
    if valid && d < (DISTANCE_THRESHOLD_DB * 10.0) as i64 {
        return true;
    }
    matches!(last, Some(prev) if prev - d < (MIN_IMPROVEMENT_DB * 10.0).round() as i64)
}

/// Auto mix (AI-1) or revision (AI-3 when `opts.request` is set).
pub fn run_mix(
    package: &Path,
    project: &Project,
    decider: &dyn Decider,
    estimate: &(dyn Fn(u64, u64) -> f64 + Sync),
    opts: &MixOptions,
    progress: &(dyn Fn(AiProgress) + Sync),
    cancel: &AtomicBool,
) -> Result<MixProposal, AiError> {
    let revision = opts.request.is_some();
    let kind = if revision { AiCallKind::Revision } else { AiCallKind::AutoMix };
    let id = log::new_id(kind);
    let started = stemwise_core::package::now_ms();
    let max_it = if revision { 1 } else { opts.max_iterations.clamp(1, 3) };
    let mut meter = Meter::default();
    let mut iterations_done = 0u32;
    let fail = |message: String, outcome: AiOutcome, meter: &Meter, its: u32| {
        let record = log::record(
            &id,
            kind,
            started,
            &decider.model(),
            &decider.effort(),
            decider.name(),
            meter,
            its,
            outcome,
            opts.request.clone(),
            Some(message.clone()),
        );
        // A failed or cancelled call keeps its log too (AI-10, AI-14).
        log::write(package, &id, "record.json", &serde_json::to_string_pretty(&record).unwrap_or_default());
        AiError::Failed { record: Box::new(record), message }
    };
    let cancelled = || cancel.load(Ordering::Relaxed);
    // Working copy with the returns the plan relies on.
    let mut work = project.clone();
    let prelude = if revision { Vec::new() } else { ensure_returns(&mut work) };
    progress(AiProgress { phase: AiPhase::Analysis, iteration: 0, of: max_it, fraction: 0.0 });
    let analysis: ProjectAnalysis = analyze_project(package, &work, &|f| {
        progress(AiProgress { phase: AiPhase::Analysis, iteration: 0, of: max_it, fraction: f });
        !cancelled()
    })
    .map_err(|e| fail(e.to_string(), if cancelled() { AiOutcome::Cancelled } else { AiOutcome::Failed }, &meter, 0))?;
    let mut ids = Ids::of(&work);
    let mut doc = model_document(&analysis, &work, &ids);
    log::write(package, &id, "analysis.json", &doc.to_string());
    let mut editable_ids: HashSet<String> = work
        .tracks
        .iter()
        .filter(|t| !t.is_return() && !t.locked && opts.scope.as_ref().is_none_or(|s| s.contains(&t.id)))
        .map(|t| t.id.clone())
        .collect();
    let mut editable: Vec<String> = editable_ids.iter().map(|t| ids.n(t)).collect();
    // Returns are shared: a request limited to some tracks leaves them alone, like a locked return (A-034).
    let mut ed = Editable {
        tracks: work
            .tracks
            .iter()
            .filter(|t| t.is_audio() && editable_ids.contains(&t.id))
            .map(|t| ids.n(&t.id))
            .collect(),
        groups: work
            .tracks
            .iter()
            .filter(|t| t.is_group() && editable_ids.contains(&t.id))
            .map(|t| ids.n(&t.id))
            .collect(),
        returns: work
            .tracks
            .iter()
            .filter(|t| t.is_return() && !t.locked && opts.scope.is_none())
            .map(|t| ids.n(&t.id))
            .collect(),
    };
    editable.extend(ed.returns.iter().cloned());
    mark_locked(&mut doc, &editable);
    // AI-18: new returns and groups only if the project allows them and the request is not limited to some tracks
    // (returns are shared and a group changes the routing of its members: both reach outside a scope).
    let allow_structure = work.ai.allow_structure && opts.scope.is_none();
    // AI-27: creative ideas, decided at the first iteration only; AI-25: the chains of returns are shared, they change
    // only when structural changes are allowed.
    let allow_ideas = work.ai.allow_ideas;
    let allow_return_blocks = allow_structure;
    let mut ideas_part: Option<MixPlan> = None;
    // AI-26: a return an idea created stays out of the later iterations (schema, document, validation): the model
    // cannot rewrite it or send to it, and the idea keeps the return it decided.
    let mut view: Option<(Project, Editable)> = None;
    let mut elements: Vec<NewElement> = Vec::new();
    // AI-7: input trims towards −23 LUFS for the auto mix (revisions keep the current trims).
    let trims: HashMap<String, f64> = if work.ai.normalize_trim && !revision {
        analysis
            .tracks
            .iter()
            .filter(|t| editable_ids.contains(&t.id))
            .map(|t| (t.id.clone(), (t.pre_gain_db * 10.0).round() / 10.0))
            .collect()
    } else {
        HashMap::new()
    };
    let target = analysis.target.as_ref();
    progress(AiProgress { phase: AiPhase::Render, iteration: 0, of: max_it, fraction: 0.0 });
    let before = evaluate_project(package, &work, target, cancel);
    // AI-22: where the balance starts from. A revision starts from the mix as it is; an auto mix from every editable
    // fader at 0 dB on the normalised trims, which is what `gain_db` 0 means in its plan.
    let start_balance = if revision {
        measure_balance(package, &work, cancel)
    } else {
        let mut unity = work.clone();
        for t in unity.tracks.iter_mut().filter(|t| t.is_audio() && editable_ids.contains(&t.id)) {
            t.chain.fader_db = 0.0;
            if let Some(trim) = trims.get(&t.id) {
                t.chain.trim_db = *trim;
            }
        }
        measure_balance(package, &unity, cancel)
    };
    if cancelled() {
        return Err(fail("cancelled".into(), AiOutcome::Cancelled, &meter, 0));
    }
    if !revision && let Some(b) = &start_balance {
        doc["balance_at_unity"] = balance_json(b, &ids);
        log::write(package, &id, "analysis.json", &doc.to_string());
    }
    let mut schema = mix_plan_schema_for(&work, &ids, &ed, allow_structure, allow_ideas, allow_return_blocks);
    let start_plan = plan_from_project(&work, &ids);
    let mut history: Vec<(Value, Value)> = Vec::new();
    if revision && let Some(b) = &before {
        // The starting plan is already in the message: only its evaluation goes here.
        let mut ej = eval_json(b, 0, true);
        if let Some(bal) = &start_balance {
            ej["balance"] = balance_json(bal, &ids);
        }
        history.push((json!("the starting plan above"), ej));
    }
    let mut previous: Option<MixPlan> = revision.then(|| start_plan.clone());
    let mut best: Option<(MixPlan, Evaluation, u32)> = None;
    let mut summaries: Vec<IterationSummary> = Vec::new();
    let mut warnings = Vec::new();
    let mut last_distance: Option<i64> = None;
    for it in 0..max_it {
        if cancelled() {
            return Err(fail("cancelled".into(), AiOutcome::Cancelled, &meter, iterations_done));
        }
        progress(AiProgress { phase: AiPhase::Decision, iteration: it + 1, of: max_it, fraction: 0.0 });
        let input = DecisionInput {
            kind: Kind::Mix,
            iteration: it,
            brief: work.brief.clone(),
            analysis: doc.clone(),
            project: view.as_ref().map_or_else(|| work.clone(), |(p, _)| p.clone()),
            ids: ids.clone(),
            history: history.iter().rev().take(2).rev().cloned().collect(),
            request: opts.request.clone(),
            start: previous.as_ref().map(|p| serde_json::to_value(p).unwrap_or_default()),
            editable: editable.clone(),
            allow_structure: allow_structure && it == 0,
            allow_ideas: allow_ideas && it == 0,
            allow_return_blocks,
        };
        let messages = build_messages(&input);
        let mut retry: Option<Retry> = None;
        let mut plan: Option<MixPlan> = None;
        let mut stopped: Option<String> = None;
        for attempt in 0..=MAX_RETRIES {
            let raw = match decider.decide(&input, &messages, &schema, retry.as_ref(), cancel) {
                Ok(r) => r,
                Err(e) => {
                    // What the call may have spent counts (AI-15): the provider's usage, or the prompt's estimate for
                    // a request that got no answer (A-031).
                    if let Some(usage) = e.usage() {
                        meter.add(usage, estimate);
                    } else if e.maybe_billed() {
                        meter.add(&prompt_estimate(&messages), estimate);
                    }
                    log::write(package, &id, &format!("request-{it}-{attempt}.json"), &e.log_entry(&messages));
                    if let Some(body) = e.last_response() {
                        log::write(package, &id, &format!("response-{it}-{attempt}.json"), body);
                    }
                    if matches!(e, DecideError::Cancelled | DecideError::Abandoned) || cancelled() {
                        return Err(fail("cancelled".into(), AiOutcome::Cancelled, &meter, iterations_done));
                    }
                    let message = e.to_string();
                    if best.is_some() && !matches!(e, DecideError::NoKey) {
                        // A later iteration failed: the plans already evaluated are kept (D-055 of the PoC).
                        stopped = Some(message);
                        break;
                    }
                    return Err(fail(message, AiOutcome::Failed, &meter, iterations_done));
                }
            };
            meter.add(&raw.usage, estimate);
            log::write(package, &id, &format!("request-{it}-{attempt}.json"), &raw.request);
            log::write(package, &id, &format!("response-{it}-{attempt}.json"), &raw.raw);
            let errors = match serde_json::from_str::<MixPlan>(&raw.content) {
                Err(e) => {
                    format!("The response is not a valid plan JSON ({e}). Return the complete plan as one JSON object.")
                }
                Ok(mut p) => {
                    // Ideas only at the first iteration and when allowed (AI-26, AI-27); return chains only with
                    // structural changes allowed (AI-25).
                    if !(allow_ideas && it == 0) {
                        let had = p.ideas.is_some();
                        if let Some(st) = &mut p.structure {
                            st.returns.retain(|_, r| r.idea.is_none());
                        }
                        let _ = crate::ideas::take(&mut p, &[]);
                        if had {
                            warnings.push(
                                "the plan proposed creative ideas, which this request does not allow: ignored".into(),
                            );
                        }
                    }
                    if !allow_return_blocks {
                        let mut had = false;
                        for r in p.returns.values_mut() {
                            had |= !r.blocks().is_empty();
                            r.blocks_mut().clear();
                        }
                        if had {
                            warnings.push(
                                "the plan changed the chain of a return, which this request does not allow: ignored"
                                    .into(),
                            );
                        }
                    }
                    // What the plan creates becomes real in a trial copy first, so the rest of the plan is validated
                    // against a project that has it (AI-17); the copy is kept only if the plan is valid.
                    let (mut trial, mut trial_ids, mut trial_ed) = match &view {
                        Some((p, e)) => (p.clone(), ids.clone(), e.clone()),
                        None => (work.clone(), ids.clone(), ed.clone()),
                    };
                    let allow =
                        Allow { structure: allow_structure && it == 0, editable: &editable_ids, stamp: started };
                    let (created, notes) =
                        structure::materialise(&mut trial, &mut trial_ids, &mut trial_ed, &mut p, &allow);
                    let report = validate_mix(&mut p, &trial, &trial_ids, &trial_ed, previous.as_ref());
                    warnings.extend(report.warnings.iter().cloned());
                    if report.ok() {
                        warnings.extend(notes);
                        if !created.is_empty() {
                            (work, ids, ed) = (trial, trial_ids, trial_ed);
                            for e in &created {
                                editable_ids.insert(e.track.id.clone());
                                editable.push(e.key.clone());
                            }
                            elements = created;
                        }
                        if it == 0 {
                            // The ideas stay fixed through the loop, which renders and evaluates the plan without them.
                            let idea_returns: Vec<(String, String)> =
                                elements.iter().filter_map(|e| e.idea.clone().map(|i| (e.key.clone(), i))).collect();
                            let part = crate::ideas::take(&mut p, &idea_returns);
                            if part.ideas.as_ref().is_some_and(|m| !m.is_empty()) {
                                ideas_part = Some(part);
                            }
                            let hidden: Vec<&NewElement> = elements.iter().filter(|e| e.idea.is_some()).collect();
                            if !hidden.is_empty() {
                                let mut p = work.clone();
                                p.tracks.retain(|t| !hidden.iter().any(|e| e.track.id == t.id));
                                let mut e = ed.clone();
                                e.returns.retain(|r| !hidden.iter().any(|h| &h.key == r));
                                editable.retain(|r| !hidden.iter().any(|h| &h.key == r));
                                view = Some((p, e));
                            }
                        }
                        plan = Some(p);
                        break;
                    }
                    report.feedback()
                }
            };
            retry = Some(Retry { previous: raw.content.clone(), errors });
        }
        if let Some(m) = stopped {
            warnings.push(format!("iteration {} failed ({m}); keeping the best plan", it + 1));
            break;
        }
        let feedback = retry.map(|r| r.errors);
        let Some(mut plan) = plan else {
            if best.is_some() {
                warnings.push(format!(
                    "iteration {} failed validation after {MAX_RETRIES} retries; keeping the best plan",
                    it + 1
                ));
                break;
            }
            return Err(fail(
                format!(
                    "the model did not return a valid plan after {MAX_RETRIES} retries: {}",
                    feedback.unwrap_or_default()
                ),
                AiOutcome::Failed,
                &meter,
                iterations_done,
            ));
        };
        // DEC-5 keeps the loop from swinging between iterations. A revision is one step the author asked for and
        // judges by ear: "as loud as the lead" must be reachable in one request (A-063).
        if !revision && let Some(prev) = &previous {
            warnings.extend(limit_changes(&mut plan, prev));
        }
        if it == 0 && (allow_structure || allow_ideas) {
            // AI-19: the structure is decided. Later iterations see what was created as ordinary returns and groups
            // and their schema has no `structure` (and no `ideas`, AI-26).
            let (seen, seen_ed) = view.as_ref().map_or((&work, &ed), |(p, e)| (p, e));
            schema = mix_plan_schema_for(seen, &ids, seen_ed, false, false, allow_return_blocks);
            doc = model_document(&analysis, seen, &ids);
            mark_locked(&mut doc, &editable);
            if !revision && let Some(b) = &start_balance {
                doc["balance_at_unity"] = balance_json(b, &ids);
            }
            // §5.5: the ideas already decided reach the later iterations as context.
            if let Some(part) = &ideas_part {
                doc["decided_ideas"] = crate::ideas::context(part);
            }
        }
        log::write(package, &id, &format!("plan-{it}.json"), &serde_json::to_string_pretty(&plan).unwrap_or_default());
        // Render the plan applied to the working copy and evaluate it.
        progress(AiProgress { phase: AiPhase::Render, iteration: it + 1, of: max_it, fraction: 0.0 });
        let mut trial = work.clone();
        for c in apply_mix_plan(&work, &plan, &ids, &ApplyOptions { trims: trims.clone(), ..Default::default() }) {
            let _ = c.apply(&mut trial);
        }
        progress(AiProgress { phase: AiPhase::Evaluation, iteration: it + 1, of: max_it, fraction: 0.0 });
        let Some(eval) = evaluate_project(package, &trial, target, cancel) else {
            if cancelled() {
                return Err(fail("cancelled".into(), AiOutcome::Cancelled, &meter, iterations_done));
            }
            return Err(fail("the plan could not be rendered".into(), AiOutcome::Failed, &meter, iterations_done));
        };
        iterations_done = it + 1;
        let d = tenth(eval.spectral_distance_db);
        let is_best = is_better(&eval, best.as_ref().map(|(_, b, _)| b));
        if is_best || (revision && eval.valid()) {
            best = Some((plan.clone(), eval.clone(), it));
        }
        summaries.push(IterationSummary {
            iteration: it + 1,
            distance_db: eval.spectral_distance_db,
            valid: eval.valid(),
            best: is_best,
        });
        let mut ej = eval_json(&eval, it, is_best);
        // AI-22: who is in front in this render, for the next iteration and for the log.
        if let Some(b) = measure_balance(package, &trial, cancel) {
            ej["balance"] = balance_json(&b, &ids);
        } else if cancelled() {
            return Err(fail("cancelled".into(), AiOutcome::Cancelled, &meter, iterations_done));
        }
        log::write(package, &id, &format!("evaluation-{it}.json"), &ej.to_string());
        history.push((serde_json::to_value(&plan).unwrap_or_default(), ej));
        let stop = should_stop(revision, iterations_done, eval.valid(), d, last_distance);
        last_distance = d;
        previous = Some(plan);
        if stop {
            break;
        }
    }
    let Some((plan, after, _)) = best else {
        return Err(fail(
            "no plan respected the technical constraints".into(),
            AiOutcome::Failed,
            &meter,
            iterations_done,
        ));
    };
    // AI-26: the ideas of the first iteration go back into the plan kept.
    let mut plan = plan;
    if let Some(part) = &ideas_part {
        warnings.extend(crate::ideas::merge(&mut plan, part));
        warnings.extend(crate::ideas::fit(&mut plan, &work, &ids));
    }
    // AI-19: a new return the chosen plan no longer sends to is not part of the proposal.
    warnings.extend(structure::prune(&mut work, &mut ids, &mut plan, &mut elements));
    // The commands are built on the project itself, creations first: exactly what an application does later.
    let mut commands = prelude.clone();
    let mut base = project.clone();
    for c in &prelude {
        let _ = c.clone().apply(&mut base);
    }
    // AI-27: ideas are ticked by the author, none by default: the proposal as received has none of them.
    let plain: Vec<NewElement> = elements.iter().filter(|e| e.idea.is_none()).cloned().collect();
    for c in structure::commands(&base, &plain, None).0 {
        if c.clone().apply(&mut base).is_ok() {
            commands.push(c);
        }
    }
    let idea_returns: Vec<(String, String)> =
        elements.iter().filter_map(|e| e.idea.clone().map(|i| (e.key.clone(), i))).collect();
    commands.extend(apply_mix_plan(
        &base,
        &crate::ideas::strip(&plan, &HashSet::new(), &idea_returns),
        &ids,
        &ApplyOptions { trims: trims.clone(), scope: opts.scope.clone(), only: None },
    ));
    let mut preview = project.clone();
    for c in &commands {
        let _ = c.clone().apply(&mut preview);
    }
    let record = log::record(
        &id,
        kind,
        started,
        &decider.model(),
        &decider.effort(),
        decider.name(),
        &meter,
        iterations_done,
        AiOutcome::Pending,
        opts.request.clone(),
        None,
    );
    // AI-23: the proposal is kept with the call, so it can be reviewed and applied again later.
    let mut record = record;
    let stored = StoredProposal {
        plan,
        ids,
        prelude,
        new_elements: elements,
        trims,
        scope: opts.scope.clone(),
        before,
        after: Some(after),
        iterations: summaries,
        warnings,
    };
    record.stored =
        serde_json::to_string(&stored).is_ok_and(|text| log::write_checked(package, &id, PROPOSAL_FILE, &text));
    log::write(package, &id, "record.json", &serde_json::to_string_pretty(&record).unwrap_or_default());
    Ok(stored.proposal(record, diff(project, &preview), commands, preview))
}

const PROPOSAL_FILE: &str = "proposal.json";

/// What a proposal is made of, apart from the project it is applied to: kept as `proposal.json` in the call's
/// folder (AI-23). Commands, diff and preview are not kept: they are computed on the project as it is when the
/// proposal is reviewed, like every application (AI-2).
#[derive(Serialize, Deserialize)]
struct StoredProposal {
    plan: MixPlan,
    ids: Ids,
    prelude: Vec<Command>,
    new_elements: Vec<NewElement>,
    trims: HashMap<String, f64>,
    scope: Option<HashSet<String>>,
    before: Option<Evaluation>,
    after: Option<Evaluation>,
    iterations: Vec<IterationSummary>,
    warnings: Vec<String>,
}

impl StoredProposal {
    fn proposal(
        self,
        record: AiCallRecord,
        diff: Vec<TrackDiff>,
        commands: Vec<Command>,
        preview: Project,
    ) -> MixProposal {
        MixProposal {
            call_id: record.id.clone(),
            kind: record.kind,
            summary: self.plan.summary.clone(),
            diff,
            plan: self.plan,
            ids: self.ids,
            commands,
            prelude: self.prelude,
            new_elements: self.new_elements,
            trims: self.trims,
            scope: self.scope,
            before: self.before,
            after: self.after,
            iterations: self.iterations,
            warnings: self.warnings,
            record,
            preview,
        }
    }
}

/// AI-23: a proposal received earlier, ready for review on the project as it is now. Its changes are computed
/// again: tracks that are gone or locked are left out, returns and groups that exist are not created twice, and
/// the diff shows what applying it would change today. The evaluations stay those measured when it was received.
pub fn restore_proposal(package: &Path, project: &Project, record: &AiCallRecord) -> Result<MixProposal, String> {
    let gone = || "this proposal was not kept with the project: it cannot be reviewed again".to_string();
    let text = std::fs::read_to_string(package.join(&record.path).join(PROPOSAL_FILE)).map_err(|_| gone())?;
    let stored: StoredProposal = serde_json::from_str(&text).map_err(|_| gone())?;
    // Tracks the plan set that are no longer in the project: said in the review, not silently dropped.
    let missing = stored
        .plan
        .tracks
        .keys()
        .chain(stored.plan.groups.keys())
        .filter(|n| stored.ids.to_project.get(*n).is_none_or(|id| project.track(id).is_none()))
        .filter(|n| !stored.new_elements.iter().any(|e| stored.ids.to_neutral.get(&e.track.id) == Some(n)))
        .count();
    let mut p = stored.proposal(record.clone(), Vec::new(), Vec::new(), project.clone());
    if missing > 0 {
        p.warnings.push(format!(
            "{missing} {} this proposal changed {} no longer in the project: {} left out.",
            if missing == 1 { "track" } else { "tracks" },
            if missing == 1 { "is" } else { "are" },
            if missing == 1 { "it is" } else { "they are" },
        ));
    }
    p.commands = apply_commands(project, &p, None, &HashSet::new());
    for c in &p.commands {
        let _ = c.clone().apply(&mut p.preview);
    }
    p.diff = diff(project, &p.preview);
    Ok(p)
}

/// The commands of a proposal computed on the project as it is now (AI-2): edits made while listening (clips,
/// automation, other tracks) are kept. `only` limits them to some tracks ("apply selected"); returns the plan
/// relies on are created first. Returns that already exist are not created twice.
pub fn apply_commands(
    project: &Project,
    proposal: &MixProposal,
    only: Option<&HashSet<String>>,
    ideas: &HashSet<String>,
) -> Vec<Command> {
    let mut work = project.clone();
    let mut cmds = Vec::new();
    for c in &proposal.prelude {
        if let Command::InsertTrack { track, .. } = c
            && work.track(&track.id).is_some()
        {
            continue;
        }
        if c.clone().apply(&mut work).is_ok() {
            cmds.push(c.clone());
        }
    }
    // What the plan creates (AI-20): all of it, or what is ticked and still needed by the ticked tracks; a return of
    // an idea only with its idea ticked (AI-27).
    let elements: Vec<NewElement> =
        proposal.new_elements.iter().filter(|e| e.idea.as_ref().is_none_or(|i| ideas.contains(i))).cloned().collect();
    for c in structure::commands(&work, &elements, only).0 {
        if c.clone().apply(&mut work).is_ok() {
            cmds.push(c);
        }
    }
    let opt = ApplyOptions { trims: proposal.trims.clone(), scope: proposal.scope.clone(), only: only.cloned() };
    let plan = crate::ideas::strip(&proposal.plan, ideas, &proposal.idea_returns());
    let mut out = apply_mix_plan(&work, &plan, &proposal.ids, &opt);
    // UI-13: a track an applied idea touches says so in its note (the motivation and the tooltip of its number).
    let touched = crate::ideas::touched(&proposal.plan, ideas);
    if !touched.is_empty() {
        for c in &mut out {
            if let Command::SetAiNotes { notes, .. } = c {
                for n in notes.iter_mut() {
                    let Some(id) = &n.track else { continue };
                    let Some(names) = touched.get(&proposal.ids.n(id)) else { continue };
                    for (name, why) in names {
                        n.text = if n.text.is_empty() {
                            format!("Idea “{name}”: {why}")
                        } else {
                            format!("{} Idea “{name}”: {why}", n.text)
                        };
                    }
                }
            }
        }
    }
    cmds.extend(out);
    cmds
}

impl MixProposal {
    /// The new returns that belong to an idea: (placeholder id, idea).
    pub fn idea_returns(&self) -> Vec<(String, String)> {
        self.new_elements.iter().filter_map(|e| e.idea.clone().map(|i| (e.key.clone(), i))).collect()
    }
}

/// The new elements an application would leave out, with the reason (AI-20).
pub fn skipped_elements(
    project: &Project,
    proposal: &MixProposal,
    only: Option<&HashSet<String>>,
) -> Vec<(String, Skipped)> {
    let mut work = project.clone();
    for c in &proposal.prelude {
        let _ = c.clone().apply(&mut work);
    }
    structure::commands(&work, &proposal.new_elements, only).1
}

/// "Apply selected" (AI-2): the commands for the chosen tracks (project ids) only.
pub fn partial_commands(project: &Project, proposal: &MixProposal, only: &HashSet<String>) -> Vec<Command> {
    apply_commands(project, proposal, Some(only), &HashSet::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stemwise_analysis::evaluate::Constraints;

    fn ev(d: Option<f64>, valid: bool) -> Evaluation {
        Evaluation {
            spectral_distance_db: d,
            band_delta_db: None,
            width_delta_db: None,
            dyn_delta_lu: None,
            constraints: Constraints { true_peak_ok: valid, low_corr_ok: true, dc_ok: true },
            lufs: None,
            true_peak_db: None,
        }
    }

    /// req: AI-1 (EVA-5 and EVA-6 of the PoC, `loop.py`)
    #[test]
    fn stop_rules_and_best_plan_follow_the_poc() {
        // Below 1.5 dB (in tenths as written) with a valid plan.
        assert!(should_stop(false, 2, true, Some(14), None));
        assert!(!should_stop(false, 2, true, Some(15), None));
        assert!(!should_stop(false, 2, false, Some(10), None), "an invalid plan below the threshold does not stop");
        // Less than 0.2 dB of improvement.
        assert!(should_stop(false, 2, true, Some(47), Some(48)));
        assert!(!should_stop(false, 2, true, Some(46), Some(48)));
        assert!(should_stop(false, 2, true, Some(50), Some(48)), "worse: stop");
        // Without a target: a valid plan; a revision runs once.
        assert!(should_stop(false, 2, true, None, None));
        assert!(!should_stop(false, 2, false, None, None));
        assert!(should_stop(true, 1, false, Some(90), None));
        // A-063: the first iteration of an auto mix never ends the loop, however close to the target: only the
        // second one has seen the balance of a rendered plan.
        assert!(!should_stop(false, 1, true, Some(9), None));
        assert!(!should_stop(false, 1, true, None, None));
        // The plan kept: valid only; a later one wins unless it is more than 1 dB further from the target.
        assert!(is_better(&ev(Some(4.0), true), None));
        assert!(!is_better(&ev(Some(1.0), false), None));
        assert!(is_better(&ev(Some(3.4), true), Some(&ev(Some(4.8), true))));
        assert!(is_better(&ev(Some(2.0), true), Some(&ev(Some(1.1), true))), "0.9 dB further: the later plan");
        assert!(is_better(&ev(Some(2.1), true), Some(&ev(Some(1.1), true))), "1.0 dB as written is still within");
        assert!(!is_better(&ev(Some(2.2), true), Some(&ev(Some(1.1), true))), "1.1 dB further: the earlier stays");
        assert!(is_better(&ev(None, true), Some(&ev(None, true))), "no target: the later valid plan");
        assert!(!is_better(&ev(None, false), Some(&ev(None, true))));
    }
}
