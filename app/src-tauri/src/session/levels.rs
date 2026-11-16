//! Levels of the mix for the Explain view (ANA-20, P-019): how loud each family, each return and each section
//! comes out of the mix as it is. Measured from renders in a background thread and kept in memory until the mix
//! changes; nothing is written to the project and nothing leaves the computer.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use stemwise_ai::balance::MixLevels;
use stemwise_core::model::Project;

use super::{Res, Session};

#[derive(Default)]
pub struct LevelsSide {
    /// The mix the kept levels were measured on.
    done: Option<(u64, MixLevels)>,
    /// The measure under way, and how to stop it.
    running: Option<(u64, Arc<AtomicBool>)>,
}

/// One measure: a copy of the project as it was asked for.
pub struct LevelsJob {
    pub package: PathBuf,
    pub project: Project,
    pub cancel: Arc<AtomicBool>,
    key: u64,
}

pub enum LevelsAsk {
    Ready(MixLevels),
    Running,
    Start(Box<LevelsJob>),
}

fn none() -> MixLevels {
    MixLevels { mix_lufs: None, sections: Vec::new(), families: Vec::new(), returns: Vec::new() }
}

/// What the levels depend on: tracks, sections and the mix bus of the open project. Names and view state are in
/// it too: a needless new measure costs less than a stale one.
fn key_of(p: &Project, open_id: u64) -> u64 {
    let mut h = DefaultHasher::new();
    open_id.hash(&mut h);
    serde_json::to_string(&(&p.tracks, &p.sections, &p.mix_bus)).unwrap_or_default().hash(&mut h);
    h.finish()
}

impl Session {
    /// The levels of the mix as it is now, or the job that measures them. A measure of an older mix is cancelled.
    pub fn levels_ask(&mut self) -> Res<LevelsAsk> {
        let project = self.project()?;
        let key = key_of(project, self.open_id);
        if let Some((k, l)) = &self.levels.done
            && *k == key
        {
            return Ok(LevelsAsk::Ready(l.clone()));
        }
        if let Some((k, cancel)) = &self.levels.running {
            if *k == key {
                return Ok(LevelsAsk::Running);
            }
            cancel.store(true, Ordering::Relaxed);
        }
        if !project.tracks.iter().any(|t| t.is_audio() && !t.clips.is_empty()) {
            self.levels.running = None;
            return Ok(LevelsAsk::Ready(none()));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let job =
            LevelsJob { package: self.dir()?.to_path_buf(), project: project.clone(), cancel: cancel.clone(), key };
        self.levels.running = Some((key, cancel));
        Ok(LevelsAsk::Start(Box::new(job)))
    }

    /// The end of a measure: the levels to announce, unless the mix or the project changed meanwhile. A measure
    /// that failed is kept as "no levels", so it is not tried again on the same mix.
    pub fn levels_done(&mut self, job: &LevelsJob, levels: Option<MixLevels>) -> Option<MixLevels> {
        if !self.levels.running.as_ref().is_some_and(|(k, _)| *k == job.key) {
            return None;
        }
        self.levels.running = None;
        let current = self.doc.as_ref().is_some_and(|d| key_of(&d.project, self.open_id) == job.key);
        if job.cancel.load(Ordering::Relaxed) || !current {
            return None;
        }
        let l = levels.unwrap_or_else(none);
        self.levels.done = Some((job.key, l.clone()));
        Some(l)
    }
}
