//! The guided tour inside the session (P-020): the demo project as an ephemeral open project in the app's cache
//! (PRJ-9), the "shown" mark in the app's data folder (UI-17), and the proposal of the offline demo prepared in the
//! background, shown when the tour reaches it (UI-18). The tour never calls a model, never spends and never writes
//! outside its own folder; ending it, in any way, deletes the demo (UI-19).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use stemwise_ai::decider::FakeDecider;
use stemwise_ai::dto::AiRun;
use stemwise_ai::mixing::{AiError, AiProgress, MixOptions, MixProposal, run_mix};
use stemwise_core::model::{AiCallKind, Project};
use stemwise_core::package;
use stemwise_engine::dto::TourInfo;

use super::ai::{Pending, Running};
use super::{Res, Session};

/// The mark that the tour was shown once, in the app's data folder.
const SHOWN: &str = "tour.json";

#[derive(Default)]
pub struct TourSide {
    /// Counts the ends of tours: a demo built for a tour that ended meanwhile is thrown away.
    pub epoch: u64,
    /// The open project is the demo of the tour.
    pub active: bool,
    /// The proposal of the offline demo, once ready; kept so the tour can show it again (back from step 9).
    proposal: Option<MixProposal>,
    failed: Option<String>,
    /// The tour reached the proposal before it was ready: it goes in review as soon as it is.
    want: bool,
    cancel: Option<Arc<AtomicBool>>,
    progress: Option<AiProgress>,
}

/// The background work that prepares the tour's proposal.
pub struct TourJob {
    pub package: PathBuf,
    pub project: Project,
    pub cancel: Arc<AtomicBool>,
    pub owner: u64,
}

/// Where the demo is built: a folder of the app's cache, emptied first (a crashed tour may have left one).
pub fn root(cache: &Path) -> PathBuf {
    cache.join("tour")
}

/// Builds the demo project (PRJ-9). Slow work: called without the session's lock.
pub fn build(cache: &Path) -> Res<(PathBuf, Project)> {
    let root = root(cache);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    stemwise_core::demo::build(&root).map_err(|e| {
        let _ = std::fs::remove_dir_all(&root);
        e.to_string()
    })
}

/// The proposal of the offline demo (AI-9): the fake decider whatever the settings, no key, no network, no cost.
pub fn propose(job: &TourJob, progress: &(dyn Fn(AiProgress) + Sync)) -> Result<MixProposal, AiError> {
    let opts = MixOptions { max_iterations: 3, scope: None, request: None };
    run_mix(&job.package, &job.project, &FakeDecider, &|_, _| 0.0, &opts, progress, &job.cancel)
}

impl Session {
    pub(super) fn not_demo(&self) -> Res<()> {
        if self.tour.active {
            return Err("the demo song of the tour is not saved and cannot be exported".into());
        }
        Ok(())
    }

    pub fn tour_info(&self) -> TourInfo {
        let shown = self.app_data.join(SHOWN).exists();
        let selftest = std::env::var_os("STEMWISE_SELFTEST_PROJECT").is_some();
        TourInfo { shown, auto: !shown && !selftest }
    }

    /// UI-17: written when the first step shows, so a tour cut short never starts by itself again.
    pub fn tour_mark_shown(&self) -> Res<()> {
        std::fs::create_dir_all(&self.app_data).map_err(|e| e.to_string())?;
        std::fs::write(self.app_data.join(SHOWN), "{\"shown\":true}\n").map_err(|e| e.to_string())
    }

    pub(super) fn tour_clear_residue(&self) {
        let _ = std::fs::remove_dir_all(root(&self.cache));
    }

    /// The demo is closed: its folder goes, and so does everything prepared for it.
    pub(super) fn tour_forget(&mut self) {
        if let Some(c) = &self.tour.cancel {
            c.store(true, Ordering::Relaxed);
        }
        self.tour = TourSide { epoch: self.tour.epoch, ..TourSide::default() };
        self.tour_clear_residue();
    }

    /// Opens the demo built by [`build`] as the open project and returns the job that prepares its proposal.
    pub fn tour_install(&mut self, dir: PathBuf, project: Project, epoch: u64) -> Res<TourJob> {
        if epoch != self.tour.epoch {
            // The tour ended while its demo was being built.
            self.tour_clear_residue();
            return Err("cancelled".into());
        }
        self.install_as(dir.clone(), project.clone(), true)?;
        let cancel = Arc::new(AtomicBool::new(false));
        self.tour.cancel = Some(cancel.clone());
        Ok(TourJob { package: dir, project, cancel, owner: self.open_id })
    }

    /// Progress of the tour's proposal; `true` when the assistant shows it (the tour is waiting for it, 18p).
    pub fn tour_progress(&mut self, owner: u64, p: AiProgress) -> bool {
        if !self.owns(owner) || !self.tour.active {
            return false;
        }
        self.tour.progress = Some(p.clone());
        match &mut self.ai.running {
            Some(r) if self.tour.want => {
                r.view.progress = Some(p);
                true
            }
            _ => false,
        }
    }

    /// The tour's proposal is ready (or failed); `true` when the assistant's status changed.
    pub fn tour_proposal_done(&mut self, owner: u64, r: Result<MixProposal, AiError>) -> bool {
        if !self.owns(owner) || !self.tour.active {
            return false;
        }
        self.tour.cancel = None;
        match r {
            Ok(p) => self.tour.proposal = Some(p),
            Err(e) => self.tour.failed = Some(e.to_string()),
        }
        if !self.tour.want {
            return false;
        }
        self.tour.want = false;
        self.ai.running = None;
        let _ = self.tour_show_proposal();
        true
    }

    /// UI-18 step 8: the proposal in review. `false` while it is still being made: the assistant shows the work and
    /// the proposal goes in review by itself when it is ready.
    pub fn tour_show_proposal(&mut self) -> Res<bool> {
        if !self.tour.active {
            return Err("no tour is running".into());
        }
        if self.ai.pending.is_some() {
            return Ok(true);
        }
        if let Some(p) = &self.tour.proposal {
            let untouched = self.untouched(None);
            self.ai.pending =
                Some(Pending { proposal: p.clone(), untouched, request: None, reopened: false, ideas: HashSet::new() });
            self.ai.listening = "current".into();
            return Ok(true);
        }
        if let Some(e) = &self.tour.failed {
            return Err(e.clone());
        }
        self.tour.want = true;
        if self.ai.running.is_none() {
            // Its own flag, never the job's: Cancel in the assistant stops the waiting, not the tour's proposal.
            self.ai.running = Some(Running {
                cancel: Arc::default(),
                view: AiRun {
                    kind: AiCallKind::AutoMix,
                    started_ms: package::now_ms(),
                    progress: self.tour.progress.clone(),
                    request: None,
                    estimate_usd: 0.0,
                },
            });
        }
        Ok(false)
    }

    /// Cancel in the assistant while the tour waits for its proposal: the assistant stops showing the work, the
    /// proposal keeps being prepared and a later step that needs it asks again. `true` when it was the tour's wait.
    pub(super) fn tour_stop_waiting(&mut self) -> bool {
        if !self.tour.active || !self.tour.want {
            return false;
        }
        self.tour.want = false;
        self.ai.running = None;
        true
    }

    /// UI-19: the end of the tour, however it ends: playback stops, the demo closes and is deleted.
    pub fn tour_end(&mut self) {
        self.tour.epoch += 1;
        if self.tour.active {
            self.stop();
            self.close();
        }
        self.tour_clear_residue();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stemwise_ai::keys::{KeyStore, MemoryKeyStore};

    fn tmp() -> tempfile::TempDir {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/tmp");
        std::fs::create_dir_all(&root).unwrap();
        tempfile::tempdir_in(root).unwrap()
    }

    fn session(t: &tempfile::TempDir) -> Session {
        Session::headless(t.path().join("data"), t.path().join("cache"))
    }

    fn files_under(p: &Path) -> usize {
        std::fs::read_dir(p)
            .map(|d| d.flatten().map(|e| if e.path().is_dir() { files_under(&e.path()) } else { 1 }).sum())
            .unwrap_or(0)
    }

    /// req: PRJ-9, UI-19
    #[test]
    fn the_demo_lives_in_the_cache_is_never_saved_and_leaves_nothing() {
        let t = tmp();
        let mut s = session(&t);
        let (dir, project) = build(&s.cache).unwrap();
        assert!(dir.starts_with(t.path().join("cache")));
        s.tour_install(dir.clone(), project, s.tour.epoch).unwrap();
        assert!(s.tour.active);
        assert_eq!(s.project().unwrap().name, stemwise_core::demo::NAME);
        // Not a project of the user: no recent entry, no crash recovery, no save, no autosave, no export.
        assert!(package::recent_projects(&s.app_data).is_empty());
        assert!(s.save().is_err());
        assert!(s.save_as(t.path(), "Copy").is_err());
        assert!(!t.path().join("Copy.stemwise").exists());
        assert!(s.export_folder().is_err());
        s.set_param_for_test();
        s.autosave(true);
        assert_eq!(files_under(&dir.join("autosave")), 0);
        assert!(!package::recovery_available(&dir));
        // The end of the tour deletes it.
        s.tour_end();
        assert!(s.doc.is_none() && !s.tour.active);
        assert!(!dir.exists());
        assert_eq!(files_under(&t.path().join("cache")), 0);
    }

    /// req: PRJ-9, UI-19
    #[test]
    fn closing_the_session_deletes_the_demo_and_a_crash_leaves_nothing_at_the_next_start() {
        let t = tmp();
        let mut s = session(&t);
        let (dir, project) = build(&s.cache).unwrap();
        s.tour_install(dir.clone(), project, s.tour.epoch).unwrap();
        drop(s);
        assert!(!dir.exists());
        // A tour that ended while its demo was being built does not open it.
        let mut s = session(&t);
        let epoch = s.tour.epoch;
        let (dir, project) = build(&s.cache).unwrap();
        s.tour_end();
        assert_eq!(s.tour_install(dir.clone(), project, epoch).err().as_deref(), Some("cancelled"));
        assert!(s.doc.is_none() && !dir.exists());
        drop(s);
        // A crash: the folder stays; the next start removes it.
        let leftover = root(&t.path().join("cache")).join("Demo song.stemwise");
        std::fs::create_dir_all(&leftover).unwrap();
        let s = Session::new(t.path().join("data"), t.path().join("cache"));
        assert!(!leftover.exists());
        drop(s);
    }

    /// req: PRJ-9, UI-17
    #[test]
    fn the_shown_mark_is_in_the_app_data_and_stops_the_automatic_start() {
        let t = tmp();
        let s = session(&t);
        let first = s.tour_info();
        assert!(!first.shown);
        assert_eq!(first.auto, std::env::var_os("STEMWISE_SELFTEST_PROJECT").is_none());
        s.tour_mark_shown().unwrap();
        assert!(t.path().join("data").join(SHOWN).exists());
        let after = s.tour_info();
        assert!(after.shown && !after.auto);
    }

    /// req: PRJ-9, AI-9
    #[test]
    fn the_proposal_comes_from_the_offline_demo_even_with_a_key_and_costs_nothing_anywhere() {
        let t = tmp();
        let mut s = session(&t);
        // A configured key and the real provider selected: the tour must not use them.
        let keys = MemoryKeyStore::default();
        keys.set("sk-or-test").unwrap();
        s.ai.keys = Box::new(keys);
        s.ai.config.offline_demo = false;
        let (dir, project) = build(&s.cache).unwrap();
        let job = s.tour_install(dir, project, s.tour.epoch).unwrap();
        // Reached before it is ready: the assistant shows the work.
        assert!(!s.tour_show_proposal().unwrap());
        assert!(s.ai.running.is_some());
        let r = propose(&job, &|_| {});
        assert!(s.tour_proposal_done(job.owner, r));
        assert!(s.ai.running.is_none());
        let p = &s.ai.pending.as_ref().expect("in review").proposal;
        assert_eq!(p.record.decider, "offline-demo");
        assert_eq!(p.record.cost_usd, 0.0);
        s.ai_apply(None, true).unwrap();
        // The call is in the demo's own record, and nowhere else.
        assert_eq!(stemwise_ai::log::total_cost(&s.app_data).1, 0);
        assert!(!s.app_data.join("ai-costs.jsonl").exists());
        // A call started by hand during the tour answers with the offline demo too.
        let job = s.ai_prepare(None, None).unwrap();
        assert!(job.decider.name().contains("offline"));
        s.tour_end();
    }

    /// req: UI-18, PRJ-9
    #[test]
    fn cancel_while_the_tour_waits_stops_the_waiting_and_not_the_proposal() {
        let t = tmp();
        let mut s = session(&t);
        let (dir, project) = build(&s.cache).unwrap();
        let job = s.tour_install(dir, project, s.tour.epoch).unwrap();
        assert!(!s.tour_show_proposal().unwrap());
        assert!(s.ai.running.is_some());
        s.ai_cancel();
        // The assistant is idle again and the tour's own work goes on.
        assert!(s.ai.running.is_none());
        assert!(!job.cancel.load(Ordering::Relaxed));
        let r = propose(&job, &|_| {});
        assert!(r.is_ok());
        // Nobody is waiting: the proposal is kept, and shown when a step asks for it again.
        assert!(!s.tour_proposal_done(job.owner, r));
        assert!(s.ai.pending.is_none());
        assert!(s.tour_show_proposal().unwrap());
        assert!(s.ai.pending.is_some());
        s.tour_end();
    }

    impl Session {
        fn set_param_for_test(&mut self) {
            let id = self.project().unwrap().tracks[0].id.clone();
            self.transact(
                stemwise_core::history::Phase::Mix,
                "Fader",
                vec![stemwise_core::command::Command::SetParam { track: id, param: "fader".into(), value: -3.0 }],
            )
            .unwrap();
            assert!(self.doc.as_ref().unwrap().dirty);
        }
    }
}
