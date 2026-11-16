//! Messages between the AI and the interface (F7, FOUNDATION §5.6): views of a running call, a proposal in review,
//! a call's log, generated as TypeScript like every contract.

use std::path::Path;

use serde::{Deserialize, Serialize};
use stemwise_analysis::evaluate::Evaluation;
use stemwise_core::model::{AiCallKind, AiCallRecord, Project, ReturnFx};
use ts_rs::{Config, TS};

use crate::apply::{DiffRow, TrackDiff};
use crate::budget::CostEstimate;
use crate::log::CostEntry;
use crate::mixing::{AiPhase, AiProgress, IterationSummary, MixProposal, skipped_elements};
use crate::openrouter::AiConfig;
use crate::structure::{NewKind, Skipped};

/// An evaluation as the review screen shows it (AI-2): distance to the reference and the technical constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct EvalView {
    pub distance_db: Option<f64>,
    pub lufs: Option<f64>,
    pub true_peak_db: Option<f64>,
    pub true_peak_ok: bool,
    pub low_corr_ok: bool,
    pub dc_ok: bool,
}

impl From<&Evaluation> for EvalView {
    fn from(e: &Evaluation) -> Self {
        EvalView {
            distance_db: e.spectral_distance_db,
            lufs: e.lufs,
            true_peak_db: e.true_peak_db,
            true_peak_ok: e.constraints.true_peak_ok,
            low_corr_ok: e.constraints.low_corr_ok,
            dc_ok: e.constraints.dc_ok,
        }
    }
}

/// A return or a group a proposal creates (design 4a "New in this proposal", AI-20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct NewElementView {
    /// Project id it will have; ticked or not like a track's in "apply selected".
    pub id: String,
    pub kind: NewKind,
    pub name: String,
    pub rationale: String,
    /// The processor of a new return as the proposal sets it.
    pub ret: Option<ReturnFx>,
    /// Members of a new group.
    pub members: Vec<String>,
    /// The tracks it exists for (senders of a return, members of a group) and how many of them must be applied
    /// for it to be created: the interface shows "not created" below that, the core decides the same way.
    pub depends_on: Vec<String>,
    pub needs: u32,
    /// False when the project changed during the review and the element can no longer be created as planned (a
    /// member removed, locked or routed elsewhere; no room for another return): the proposal says so (AI-20).
    pub fits: bool,
    /// The creative idea it belongs to (AI-26, AI-27): created only with the idea ticked, shown on the idea's card.
    pub idea: Option<String>,
}

/// A creative idea of a proposal (AI-27, design 4c): ticked by the author, off by default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct IdeaView {
    pub id: String,
    pub name: String,
    pub rationale: String,
    /// Names of the sections it plays in (empty: the whole song).
    pub sections: Vec<String>,
    /// What it adds, one line each ("Rhodes · Modulation (new)").
    pub elements: Vec<String>,
    /// Tracks it touches (project ids), for "apply selected".
    pub tracks: Vec<String>,
    pub on: bool,
}

/// A proposal in review (design 4a): nothing is applied until the author says so.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ProposalView {
    pub call_id: String,
    pub kind: AiCallKind,
    pub summary: String,
    pub diff: Vec<TrackDiff>,
    /// Returns and groups the plan creates, in the order they are created (AI-17).
    pub new_elements: Vec<NewElementView>,
    pub before: Option<EvalView>,
    pub after: Option<EvalView>,
    pub iterations: Vec<IterationSummary>,
    pub warnings: Vec<String>,
    pub record: AiCallRecord,
    /// AI-7: the proposal sets input trims towards −23 LUFS.
    pub trims: bool,
    /// Tracks the call could not touch (locked or outside the request's scope), as context.
    pub untouched: Vec<String>,
    pub request: Option<String>,
    /// AI-23: received earlier and opened again. The diff is against the mix as it is now; the evaluations are
    /// those measured when it was received (`record.started_ms`).
    pub reopened: bool,
    /// AI-27: the creative ideas, in order.
    pub ideas: Vec<IdeaView>,
}

impl ProposalView {
    /// `project` is the project as it is now, which may have changed since the proposal was made.
    pub fn of(
        p: &MixProposal,
        project: &Project,
        untouched: Vec<String>,
        request: Option<String>,
        reopened: bool,
        ideas_on: &std::collections::HashSet<String>,
    ) -> Self {
        let unfit: Vec<String> = skipped_elements(project, p, None)
            .into_iter()
            .filter(|(_, why)| *why == Skipped::NoLongerFits)
            .map(|(id, _)| id)
            .collect();
        ProposalView {
            call_id: p.call_id.clone(),
            kind: p.kind,
            summary: p.summary.clone(),
            diff: p.diff.clone(),
            new_elements: p
                .new_elements
                .iter()
                .map(|e| {
                    let (deps, needs) = e.depends_on();
                    NewElementView {
                        id: e.track.id.clone(),
                        kind: e.kind,
                        name: e.track.name.clone(),
                        rationale: e.rationale.clone(),
                        ret: p.preview.track(&e.track.id).and_then(|t| t.ret.clone()).or_else(|| e.track.ret.clone()),
                        members: e.members.clone(),
                        depends_on: deps.to_vec(),
                        needs: needs as u32,
                        fits: !unfit.contains(&e.track.id),
                        idea: e.idea.clone(),
                    }
                })
                .collect(),
            before: p.before.as_ref().map(EvalView::from),
            after: p.after.as_ref().map(EvalView::from),
            iterations: p.iterations.clone(),
            warnings: p.warnings.clone(),
            record: p.record.clone(),
            trims: !p.trims.is_empty(),
            untouched,
            request,
            reopened,
            ideas: idea_views(p, project, ideas_on),
        }
    }
}

/// The ideas of a proposal as the review shows them (AI-27).
pub fn idea_views(p: &MixProposal, project: &Project, on: &std::collections::HashSet<String>) -> Vec<IdeaView> {
    let Some(ideas) = &p.plan.ideas else { return Vec::new() };
    let name_of = |n: &str| -> String {
        let id = p.ids.to_project.get(n).cloned().unwrap_or_else(|| n.to_string());
        project
            .track(&id)
            .map(|t| t.name.clone())
            .or_else(|| project.sections.iter().find(|s| s.id == id).map(|s| s.name.clone()))
            .or_else(|| p.new_elements.iter().find(|e| e.key == n).map(|e| e.track.name.clone()))
            .unwrap_or_else(|| n.to_string())
    };
    ideas
        .iter()
        .map(|(k, idea)| {
            let mut elements = crate::ideas::elements(&p.plan, k, &name_of);
            elements.extend(
                p.new_elements
                    .iter()
                    .filter(|e| e.idea.as_deref() == Some(k))
                    .map(|e| format!("New return · {}", e.track.name)),
            );
            let tracks: Vec<String> = p
                .plan
                .tracks
                .iter()
                .filter(|(_, t)| {
                    t.blocks.iter().any(|b| b.idea.as_deref() == Some(k))
                        || t.sections.values().any(|s| s.idea.as_deref() == Some(k))
                        || t.idea_sends_db.contains_key(k)
                })
                .map(|(n, _)| n)
                .chain(
                    p.plan
                        .groups
                        .iter()
                        .filter(|(_, g)| {
                            g.blocks.iter().any(|b| b.idea.as_deref() == Some(k)) || g.idea_sends_db.contains_key(k)
                        })
                        .map(|(n, _)| n),
                )
                .filter_map(|n| p.ids.to_project.get(n).cloned())
                .collect();
            IdeaView {
                id: k.clone(),
                name: idea.name.clone(),
                rationale: idea.rationale.clone(),
                sections: idea.sections.iter().map(|s| name_of(s)).collect(),
                elements,
                tracks,
                on: on.contains(k),
            }
        })
        .collect()
}

/// A call in progress (design 5a).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AiRun {
    pub kind: AiCallKind,
    pub started_ms: u64,
    pub progress: Option<AiProgress>,
    pub request: Option<String>,
    pub estimate_usd: f64,
}

/// Everything the assistant column needs (AI-11, AI-12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AiStatus {
    pub running: Option<AiRun>,
    pub proposal: Option<ProposalView>,
    /// What the A/B plays: "current", "proposal" or "snapshot:<id>" (AI-2, HIS-4).
    pub listening: String,
    pub has_key: bool,
    /// Where this build keeps the key: the system keychain, or a private file of the app (A-072).
    pub key_place: crate::keys::KeyPlace,
    pub config: AiConfig,
    /// What an auto mix and a revision cost on a typical song at the configured prices (design 14d).
    pub typical_mix_usd: f64,
    pub typical_revision_usd: f64,
    /// Spent on every project, from the app's register (AI-14).
    pub total_usd: f64,
    pub total_calls: u32,
    pub total_projects: u32,
}

/// The log of one call (design 12b, 12c): the exact request and response of each attempt, the plan and the
/// analysis document that were sent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct CallDetail {
    pub record: AiCallRecord,
    /// File name → content, in name order (`analysis.json`, `request-0-0.json`, `response-0-0.json`, `plan-0.json`…).
    pub files: Vec<(String, String)>,
}

/// Reads the log folder of a call (AI-10).
pub fn call_detail(package: &Path, record: &AiCallRecord) -> CallDetail {
    let dir = package.join(&record.path);
    let mut files: Vec<(String, String)> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().into_string().ok()?;
                    let text = std::fs::read_to_string(e.path()).ok()?;
                    Some((name, text))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    CallDetail { record: record.clone(), files }
}

/// Writes the TypeScript bindings of the AI's messages.
pub fn write_bindings(dir: &Path) -> Result<(), ts_rs::ExportError> {
    let cfg = Config::new().with_out_dir(dir).with_large_int("number");
    EvalView::export_all(&cfg)?;
    ProposalView::export_all(&cfg)?;
    IdeaView::export_all(&cfg)?;
    AiRun::export_all(&cfg)?;
    AiStatus::export_all(&cfg)?;
    CallDetail::export_all(&cfg)?;
    CostEstimate::export_all(&cfg)?;
    CostEntry::export_all(&cfg)?;
    AiPhase::export_all(&cfg)?;
    DiffRow::export_all(&cfg)?;
    crate::balance::MixLevels::export_all(&cfg)?;
    Ok(())
}
