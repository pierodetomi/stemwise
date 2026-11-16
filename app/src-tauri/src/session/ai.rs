//! The assistant inside the app (F7): calls on their own thread with progress and cancel (AI-12), the proposal in
//! review with A/B against the current mix (AI-2), applying all or some tracks as one step (HIS-3), the call log and
//! the cost register (AI-10, AI-14, AI-15), the estimate and the spending cap (AI-16), snapshots with A/B (HIS-4),
//! brief and references (PRJ-7), the key in the keychain (AI-11).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use stemwise_ai::budget::{CostEstimate, estimate, typical_costs};
use stemwise_ai::connect::{Connect, ConnectError};
use stemwise_ai::decider::{Decider, FakeDecider};
use stemwise_ai::dto::{AiRun, AiStatus, CallDetail, ProposalView, call_detail};
use stemwise_ai::keys::{KeyPlace, KeyStore, MemoryKeyStore, app_key_store};
use stemwise_ai::log::{CostEntry, calls_csv, register_cost, total_cost};
use stemwise_ai::mixing::{AiError, AiProgress, MixOptions, MixProposal, apply_commands, restore_proposal};
use stemwise_ai::openrouter::{AiConfig, OpenRouter};
use stemwise_core::command::Command;
use stemwise_core::history::Phase;
use stemwise_core::model::{AiCallKind, AiCallRecord, AiOutcome, Project};
use stemwise_core::package;

use super::{Res, Session, err};

/// A call in progress: its cancel flag and what the UI shows.
pub struct Running {
    pub cancel: Arc<AtomicBool>,
    pub view: AiRun,
}

pub struct Pending {
    pub proposal: MixProposal,
    pub untouched: Vec<String>,
    pub request: Option<String>,
    /// A proposal received earlier and opened again for review (AI-23): closing it leaves its record as it is.
    pub reopened: bool,
    /// AI-27: the ideas the author ticked (none by default).
    pub ideas: HashSet<String>,
}

pub struct AiSide {
    pub config: AiConfig,
    pub keys: Box<dyn KeyStore>,
    pub key_place: KeyPlace,
    /// The cancel flag of a connection waiting for the browser (AI-24): at most one at a time.
    pub connecting: Option<Arc<AtomicBool>>,
    /// The page of that connection, to open it again (design 14b).
    pub connect_url: Option<String>,
    pub running: Option<Running>,
    pub pending: Option<Pending>,
    /// "current", "proposal" or "snapshot:<id>".
    pub listening: String,
}

impl AiSide {
    pub fn new(app_data: &Path) -> Self {
        let selftest = std::env::var_os("STEMWISE_SELFTEST_PROJECT").is_some();
        let mut config: AiConfig = std::fs::read_to_string(app_data.join("ai-config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        // The self-test never touches the keychain or the network: offline demo, in-memory key.
        let (keys, key_place): (Box<dyn KeyStore>, KeyPlace) = if selftest {
            config.offline_demo = true;
            (Box::<MemoryKeyStore>::default(), KeyPlace::File)
        } else {
            // The keychain where it can be read without a password prompt, a private file otherwise (A-072).
            // Neither is read here: only when a call needs the key (A-052).
            app_key_store(app_data, stemwise_ai::signing::keychain_is_silent())
        };
        AiSide {
            config,
            keys,
            key_place,
            connecting: None,
            connect_url: None,
            running: None,
            pending: None,
            listening: "current".into(),
        }
    }
}

/// What a call needs off the lock.
pub struct Job {
    pub package: PathBuf,
    pub project: Project,
    pub decider: Box<dyn Decider>,
    pub config: AiConfig,
    pub opts: MixOptions,
    pub cancel: Arc<AtomicBool>,
}

/// Rough size of the first messages of a call, for the estimate shown before sending (A-032).
fn message_chars(p: &Project) -> usize {
    let tracks = p.tracks.iter().filter(|t| !t.is_return()).count();
    9000 + 1400 * tracks + 300 * p.sections.len() + 250 * tracks * p.sections.len()
}

impl Session {
    pub(super) fn untouched(&self, scope: Option<&HashSet<String>>) -> Vec<String> {
        self.doc
            .as_ref()
            .map(|d| {
                d.project
                    .tracks
                    .iter()
                    .filter(|t| !t.is_return() && (t.locked || scope.is_some_and(|s| !s.contains(&t.id))))
                    .map(|t| t.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A proposal is in review: what it shows depends on the project as it is now (AI-20).
    pub fn reviewing(&self) -> bool {
        self.ai.pending.is_some()
    }

    pub fn ai_status(&self) -> AiStatus {
        let (total_usd, total_calls, total_projects) = total_cost(&self.app_data);
        let typical = typical_costs(&self.ai.config);
        AiStatus {
            running: self.ai.running.as_ref().map(|r| r.view.clone()),
            proposal: self.ai.pending.as_ref().zip(self.project().ok()).map(|(p, project)| {
                ProposalView::of(&p.proposal, project, p.untouched.clone(), p.request.clone(), p.reopened, &p.ideas)
            }),
            listening: self.ai.listening.clone(),
            has_key: self.ai.keys.has(),
            key_place: self.ai.key_place,
            config: self.ai.config.clone(),
            typical_mix_usd: typical.0,
            typical_revision_usd: typical.1,
            total_usd,
            total_calls: total_calls as u32,
            total_projects: total_projects as u32,
        }
    }

    /// AI-16: tokens and cost before sending, against the project's cap.
    pub fn ai_estimate(&self, revision: bool) -> Res<CostEstimate> {
        let p = self.project()?;
        Ok(estimate(p, message_chars(p), if revision { 1 } else { 3 }, false, &self.ai.config))
    }

    /// Prepares a call: checks that nothing else runs and that a decider is available (AI-11).
    pub fn ai_prepare(&mut self, request: Option<String>, scope: Option<Vec<String>>) -> Res<Job> {
        if self.ai.running.is_some() {
            return Err("the assistant is already working".into());
        }
        if self.ai.pending.is_some() {
            return Err("a proposal is in review: apply or discard it first".into());
        }
        let package = self.dir()?.to_path_buf();
        let project = self.project()?.clone();
        if !project.tracks.iter().any(|t| t.is_audio() && !t.clips.is_empty()) {
            return Err("add tracks first: the assistant works from measurements of your tracks".into());
        }
        // UI-6: a missing file would reach the model as a silent track.
        let missing = stemwise_core::package::missing_media(&package, &project).len();
        if missing > 0 {
            return Err(format!(
                "{missing} audio file{} missing from the project folder: locate {} before asking the assistant",
                if missing == 1 { " is" } else { "s are" },
                if missing == 1 { "it" } else { "them" }
            ));
        }
        let config = self.ai.config.clone();
        // The tour never calls a model and never needs a key: the offline demo answers (PRJ-9).
        let decider: Box<dyn Decider> = if config.offline_demo || self.tour.active {
            Box::new(FakeDecider)
        } else {
            match self.ai.keys.get() {
                Some(k) => Box::new(OpenRouter::new(config.clone(), k)),
                None => return Err("no-key".into()),
            }
        };
        let revision = request.is_some();
        let scope: Option<HashSet<String>> = scope.filter(|s| !s.is_empty()).map(|s| s.into_iter().collect());
        let est = self.ai_estimate(revision)?.cost_usd;
        let cancel = Arc::new(AtomicBool::new(false));
        self.ai.running = Some(Running {
            cancel: cancel.clone(),
            view: AiRun {
                kind: if revision { AiCallKind::Revision } else { AiCallKind::AutoMix },
                started_ms: package::now_ms(),
                progress: None,
                request: request.clone(),
                estimate_usd: est,
            },
        });
        let opts = MixOptions { max_iterations: if revision { 1 } else { 3 }, scope, request };
        Ok(Job { package, project, decider, config, opts, cancel })
    }

    pub fn ai_progress(&mut self, p: AiProgress) {
        if let Some(r) = &mut self.ai.running {
            r.view.progress = Some(p);
        }
    }

    /// The end of a call: a proposal to review, or a failed or cancelled call logged with what it spent.
    pub fn ai_finished(
        &mut self,
        job_scope: Option<HashSet<String>>,
        request: Option<String>,
        result: Result<MixProposal, AiError>,
    ) -> Result<(), String> {
        self.ai.running = None;
        match result {
            Ok(p) => {
                let untouched = self.untouched(job_scope.as_ref());
                self.ai.pending =
                    Some(Pending { proposal: p, untouched, request, reopened: false, ideas: HashSet::new() });
                Ok(())
            }
            Err(e) => {
                let rec = e.record().clone();
                let msg = e.to_string();
                self.log_call(rec);
                Err(msg)
            }
        }
    }

    pub fn ai_cancel(&mut self) {
        if self.tour_stop_waiting() {
            return;
        }
        if let Some(r) = &self.ai.running {
            r.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Records a call in the project (view state: not an undo step) and in the register of all projects.
    pub(crate) fn log_call(&mut self, record: AiCallRecord) {
        let demo = self.tour.active;
        let Some(d) = self.doc.as_mut() else { return };
        // The tour's calls stay out of the register of all projects (PRJ-9): they cost nothing and belong to no project.
        if !demo {
            register_cost(
                &self.app_data,
                &CostEntry {
                    project: d.project.name.clone(),
                    call: record.id.clone(),
                    at_ms: record.started_ms,
                    model: record.model.clone(),
                    cost_usd: record.cost_usd,
                    estimated: record.cost_estimated,
                },
            );
        }
        let mut calls = d.project.ai.calls.clone();
        calls.retain(|c| c.id != record.id);
        calls.push(record);
        let _ = d.apply_view(vec![Command::SetAiCalls { calls }]);
        self.autosave(true);
    }

    /// AI-2: apply the whole proposal or the chosen tracks (project ids), as one step with the call's id (HIS-3).
    /// The commands are computed on the project as it is now, so edits made while listening are kept.
    pub fn ai_apply(&mut self, only: Option<Vec<String>>, trims: bool) -> Res<String> {
        let mut pending = self.ai.pending.take().ok_or("no proposal to apply")?;
        if !trims {
            pending.proposal.trims.clear();
        }
        let only: Option<HashSet<String>> = only.map(|v| v.into_iter().collect());
        let project = self.project()?.clone();
        let cmds = apply_commands(&project, &pending.proposal, only.as_ref(), &pending.ideas);
        let changed = cmds.iter().filter(|c| matches!(c, Command::SetTrack { .. })).count();
        let mut label = match (&pending.request, pending.proposal.kind) {
            (Some(r), _) => format!("Revision applied: “{}”", r.chars().take(48).collect::<String>()),
            _ => format!("Auto mix applied · {changed} {}", if changed == 1 { "track" } else { "tracks" }),
        };
        // AI-27: the ideas applied are named in the step.
        let names: Vec<String> = pending
            .proposal
            .plan
            .ideas
            .iter()
            .flatten()
            .filter(|(k, _)| pending.ideas.contains(*k))
            .map(|(_, i)| format!("“{}”", i.name))
            .collect();
        if !names.is_empty() {
            label = format!("{label} · {} {}", if names.len() == 1 { "idea" } else { "ideas" }, names.join(", "));
        }
        let id = pending.proposal.call_id.clone();
        let change = match self.doc_mut()?.transact(Phase::Mix, &label, cmds, Some(id)) {
            Ok(c) => c,
            Err(e) => {
                self.ai.pending = Some(pending);
                return Err(e.to_string());
            }
        };
        self.ai.listening = "current".into();
        self.apply(&change);
        let mut rec = pending.proposal.record.clone();
        rec.outcome = if only.is_some() { AiOutcome::AppliedPartly } else { AiOutcome::Applied };
        rec.history_label = Some(label.clone());
        self.log_call(rec);
        Ok(label)
    }

    /// AI-27: the ideas ticked in the review; listening to the proposal follows them.
    pub fn ai_set_ideas(&mut self, ideas: Vec<String>) -> Res<()> {
        let p = self.ai.pending.as_mut().ok_or("no proposal in review")?;
        p.ideas = ideas.into_iter().collect();
        if self.ai.listening == "proposal" {
            self.ai_listen("proposal")?;
        }
        Ok(())
    }

    pub fn ai_discard(&mut self) -> Res<()> {
        let pending = self.ai.pending.take().ok_or("no proposal to discard")?;
        // A proposal opened again keeps what its record says happened to it (AI-23).
        if !pending.reopened {
            let mut rec = pending.proposal.record.clone();
            rec.outcome = AiOutcome::Discarded;
            self.log_call(rec);
        }
        self.ai_listen("current")
    }

    /// AI-23: opens a proposal received earlier for review, on the mix as it is now. Nothing changes until it is
    /// applied, which is one history step like the first time (AI-2, HIS-3).
    pub fn ai_reopen(&mut self, id: &str) -> Res<()> {
        if self.ai.running.is_some() {
            return Err("the assistant is working: wait for it to finish".into());
        }
        if self.ai.pending.is_some() {
            return Err("a proposal is in review: apply or discard it first".into());
        }
        let project = self.project()?;
        let rec = project
            .ai
            .calls
            .iter()
            .find(|c| c.id == id && matches!(c.kind, AiCallKind::AutoMix | AiCallKind::Revision))
            .ok_or("no such proposal")?
            .clone();
        let proposal = restore_proposal(self.dir()?, project, &rec)?;
        let untouched = self.untouched(proposal.scope.as_ref());
        self.ai.pending =
            Some(Pending { proposal, untouched, request: rec.request, reopened: true, ideas: HashSet::new() });
        Ok(())
    }

    /// A/B without stopping playback and without a history step (AI-2, HIS-4): the engine plays another version of
    /// the mix while the document stays as it is.
    pub fn ai_listen(&mut self, what: &str) -> Res<()> {
        let dir = self.dir()?.to_path_buf();
        let current = self.project()?.clone();
        let target: Project = if what == "current" {
            current
        } else if what == "proposal" || what.starts_with("idea:") {
            // The proposal with the ideas ticked, or with one idea only (AI-27, design 4d).
            let p = self.ai.pending.as_ref().ok_or("no proposal to listen to")?;
            let ideas: HashSet<String> = match what.strip_prefix("idea:") {
                Some(i) => [i.to_string()].into(),
                None => p.ideas.clone(),
            };
            let mut q = current.clone();
            for c in apply_commands(&current, &p.proposal, None, &ideas) {
                let _ = c.apply(&mut q);
            }
            q
        } else if let Some(id) = what.strip_prefix("snapshot:") {
            let info = current.snapshots.iter().find(|s| s.id == id).ok_or("no such snapshot")?;
            let state = package::load_snapshot(&dir, info).map_err(err)?;
            let mut q = current.clone();
            for c in package::restore_commands(&current, &state) {
                let _ = c.apply(&mut q);
            }
            q
        } else {
            return Err(format!("nothing to listen to: {what}"));
        };
        if let Some(e) = &mut self.engine {
            e.load(&dir, &target).map_err(err)?;
        }
        self.ai.listening = what.to_string();
        Ok(())
    }

    pub fn ai_detail(&self, id: &str) -> Res<CallDetail> {
        let p = self.project()?;
        let rec = p.ai.calls.iter().find(|c| c.id == id).ok_or("no such call")?;
        Ok(call_detail(self.dir()?, rec))
    }

    /// AI-14: the call history as CSV or JSON.
    pub fn ai_export(&self, path: &Path, json: bool) -> Res<()> {
        let calls = &self.project()?.ai.calls;
        let text = if json { serde_json::to_string_pretty(calls).map_err(err)? } else { calls_csv(calls) };
        std::fs::write(path, text).map_err(err)
    }

    pub fn ai_set_config(&mut self, cfg: AiConfig) -> Res<()> {
        std::fs::create_dir_all(&self.app_data).map_err(err)?;
        std::fs::write(self.app_data.join("ai-config.json"), serde_json::to_string_pretty(&cfg).map_err(err)?)
            .map_err(err)?;
        self.ai.config = cfg;
        Ok(())
    }

    pub fn ai_set_key(&mut self, key: Option<String>) -> Res<()> {
        match key {
            Some(k) if !k.trim().is_empty() => self.ai.keys.set(&k),
            _ => self.ai.keys.delete(),
        }
    }

    /// AI-24: starts connecting an OpenRouter account. Returns the listener to wait on and its cancel flag; a
    /// connection already waiting is cancelled first.
    pub fn ai_connect_begin(&mut self) -> Result<(Connect, Arc<AtomicBool>), ConnectError> {
        self.ai_connect_cancel();
        let connect = Connect::begin(&self.ai.config.base_url)?;
        let cancel = Arc::new(AtomicBool::new(false));
        self.ai.connecting = Some(cancel.clone());
        self.ai.connect_url = Some(connect.auth_url.clone());
        Ok((connect, cancel))
    }

    pub fn ai_connect_cancel(&mut self) {
        if let Some(c) = self.ai.connecting.take() {
            c.store(true, Ordering::Relaxed);
        }
        self.ai.connect_url = None;
    }

    /// The end of a connection: the key goes to the store and the assistant leaves the offline demo.
    pub fn ai_connect_finished(&mut self, cancel: &Arc<AtomicBool>, key: Result<String, ConnectError>) -> Res<()> {
        if self.ai.connecting.as_ref().is_some_and(|c| Arc::ptr_eq(c, cancel)) {
            self.ai.connecting = None;
            self.ai.connect_url = None;
        }
        let key = key.map_err(|e| e.code().to_string())?;
        self.ai.keys.set(&key)?;
        let mut cfg = self.ai.config.clone();
        cfg.offline_demo = false;
        self.ai_set_config(cfg)
    }

    pub fn set_spend_cap(&mut self, usd: Option<f64>) -> Res<()> {
        self.view(vec![Command::SetSpendCap { usd }])
    }

    /// AI-18: whether the assistant may propose new returns and groups; kept in the project, not a history step.
    pub fn set_allow_structure(&mut self, on: bool) -> Res<()> {
        self.view(vec![Command::SetAllowStructure { on }])
    }

    /// AI-27: whether the assistant may propose creative ideas; kept in the project, not a history step.
    pub fn set_allow_ideas(&mut self, on: bool) -> Res<()> {
        self.view(vec![Command::SetAllowIdeas { on }])
    }

    pub fn set_normalize_trim(&mut self, on: bool) -> Res<()> {
        self.transact(
            Phase::Mix,
            if on { "Level tracks to −23 LUFS first" } else { "Keep the recorded levels" },
            vec![Command::SetNormalizeTrim { on }],
        )
    }

    // Snapshots (HIS-4) and references (PRJ-7)

    pub fn snapshot_save(&mut self, name: &str) -> Res<()> {
        let dir = self.dir()?.to_path_buf();
        let d = self.doc_mut()?;
        let c = package::save_snapshot(&dir, &mut d.project, name).map_err(err)?;
        // The list of snapshots is not a mix change: no undo step for saving one.
        self.view(vec![c])
    }

    pub fn snapshot_restore(&mut self, id: &str) -> Res<()> {
        let dir = self.dir()?.to_path_buf();
        let p = self.project()?.clone();
        let info = p.snapshots.iter().find(|s| s.id == id).ok_or("no such snapshot")?.clone();
        let state = package::load_snapshot(&dir, &info).map_err(err)?;
        let cmds = package::restore_commands(&p, &state);
        self.ai.listening = "current".into();
        self.transact(Phase::Mix, &format!("Restore snapshot “{}”", info.name), cmds)
    }

    pub fn snapshot_delete(&mut self, id: &str) -> Res<()> {
        let p = self.project()?;
        let snapshots = p.snapshots.iter().filter(|s| s.id != id).cloned().collect();
        self.view(vec![Command::SetSnapshots { snapshots }])
    }

    pub fn add_reference(&mut self, path: &Path) -> Res<()> {
        let dir = self.dir()?.to_path_buf();
        let c = stemwise_core::import::add_reference(&dir, self.project()?, path).map_err(err)?;
        self.transact(Phase::Mix, "Add reference", vec![c])
    }

    pub fn remove_reference(&mut self, id: &str) -> Res<()> {
        let c = stemwise_core::import::remove_reference(self.project()?, id);
        self.transact(Phase::Mix, "Remove reference", vec![c])
    }
}
