//! The open project as the app sees it: document and histories (core), audio engine and output (engine),
//! autosave and recent projects. Framework-agnostic: the Tauri commands in `main.rs` are thin wrappers.

use std::path::{Path, PathBuf};
use std::time::Instant;

use stemwise_core::command::Command;
use stemwise_core::history::{Change, Document, HistoryEntry, Phase};
use stemwise_core::model::Project;
use stemwise_core::package;
use stemwise_engine::device::{DeviceInfo, Output};
use stemwise_engine::realtime::Engine;

mod ai;
pub mod levels;
pub mod master;
pub mod tour;

pub type Res<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub use stemwise_engine::dto::{AudioPrefs, MeterSnapshot, TransportState};

pub struct Session {
    pub app_data: PathBuf,
    /// The app's cache folder: the demo project of the tour lives there (PRJ-9), never among the user's documents.
    pub cache: PathBuf,
    /// The guided tour (P-020): whether the open project is its demo, and the proposal it prepares.
    pub tour: tour::TourSide,
    /// Whether the session opens the audio output (the tests of the session never touch a device).
    audio: bool,
    pub doc: Option<Document>,
    pub dir: Option<PathBuf>,
    engine: Option<Engine>,
    output: Option<Output>,
    device_error: Option<String>,
    pub prefs: AudioPrefs,
    last_autosave: Instant,
    pub ai: ai::AiSide,
    pub master: master::MasterSide,
    pub levels: levels::LevelsSide,
    /// Changes at every open and close: a background job remembers it and drops its result when the project it
    /// worked for is no longer the open one (F8 review, S6).
    pub open_id: u64,
}

impl Session {
    pub fn new(app_data: PathBuf, cache: PathBuf) -> Self {
        let s = Self::with_audio(app_data, cache, true);
        // A demo left behind by a crash is removed at the next start (PRJ-9).
        s.tour_clear_residue();
        s
    }

    /// A session without an audio output, for the tests.
    #[cfg(test)]
    pub fn headless(app_data: PathBuf, cache: PathBuf) -> Self {
        Self::with_audio(app_data, cache, false)
    }

    fn with_audio(app_data: PathBuf, cache: PathBuf, audio: bool) -> Self {
        let prefs = std::fs::read_to_string(app_data.join("audio.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let ai = ai::AiSide::new(&app_data);
        Session {
            ai,
            master: master::MasterSide::default(),
            levels: levels::LevelsSide::default(),
            app_data,
            cache,
            tour: tour::TourSide::default(),
            audio,
            doc: None,
            dir: None,
            engine: None,
            output: None,
            device_error: None,
            prefs,
            last_autosave: Instant::now(),
            open_id: 0,
        }
    }

    pub fn project(&self) -> Res<&Project> {
        self.doc.as_ref().map(|d| &d.project).ok_or_else(|| "no project is open".to_string())
    }

    fn doc_mut(&mut self) -> Res<&mut Document> {
        self.doc.as_mut().ok_or_else(|| "no project is open".to_string())
    }

    pub fn dir(&self) -> Res<&Path> {
        self.dir.as_deref().ok_or_else(|| "no project is open".to_string())
    }

    // -----------------------------------------------------------------------------------------
    // Projects (PRJ-1, PRJ-2)
    // -----------------------------------------------------------------------------------------

    pub fn create(&mut self, parent: &Path, name: &str) -> Res<()> {
        let (dir, project) = package::create(parent, name).map_err(err)?;
        self.install(dir, project)
    }

    /// Opens a package. Returns `true` when an autosave from a crashed session can be recovered (design 13g).
    pub fn open(&mut self, dir: &Path) -> Res<bool> {
        let project = package::load(dir).map_err(err)?;
        let crashed = package::open_session(dir).map_err(err)?;
        self.install(dir.to_path_buf(), project)?;
        Ok(crashed)
    }

    pub fn recover(&mut self, accept: bool) -> Res<()> {
        let dir = self.dir()?.to_path_buf();
        if accept {
            let p = package::load_recovery(&dir).map_err(err)?;
            self.install(dir, p)?;
            if let Some(d) = &mut self.doc {
                d.dirty = true;
            }
        } else {
            package::discard_recovery(&dir);
        }
        Ok(())
    }

    fn install(&mut self, dir: PathBuf, project: Project) -> Res<()> {
        self.install_as(dir, project, false)
    }

    /// Opens a package as the open project; the tour's demo stays out of the recent projects and of crash recovery.
    fn install_as(&mut self, dir: PathBuf, project: Project, demo: bool) -> Res<()> {
        self.close();
        if !demo {
            let _ = package::touch_recent(&self.app_data, &dir, &project.name);
            let _ = package::open_session(&dir);
        }
        self.tour.active = demo;
        self.doc = Some(Document::new(project));
        self.dir = Some(dir);
        self.restart_audio();
        Ok(())
    }

    /// UI-6: a missing file located again (see `media::relink`).
    pub fn relink_media(&mut self, id: &str, src: &Path) -> Res<bool> {
        let dir = self.dir()?.to_path_buf();
        let p = self.project()?;
        let m = p.media(id).ok_or("no such media")?.clone();
        match stemwise_core::media::relink(&dir, p, &m, src).map_err(err)? {
            None => {
                let _ = stemwise_core::media::ensure_cache(&dir, &m);
                // Same document: the engine opens the file again.
                self.master_reload()?;
                Ok(false)
            }
            Some(new) => {
                let _ = stemwise_core::media::ensure_cache(&dir, &new);
                let label = format!("Locate “{}”", m.original_name);
                self.transact(Phase::Mix, &label, {
                    let to = new.id.clone();
                    vec![Command::AddMedia { media: Box::new(new) }, Command::RelinkMedia { from: id.into(), to }]
                })?;
                Ok(true)
            }
        }
    }

    pub fn close(&mut self) {
        if let Some(r) = &self.ai.running {
            r.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.ai.pending = None;
        self.ai.listening = "current".into();
        if let Some(j) = &self.master.job {
            j.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        // The loudness generation keeps counting (a late result never matches the next project's), and the loudness
        // thread, if one is running, is still the only one.
        self.master = master::MasterSide {
            epoch: self.master.epoch + 1,
            loud_running: self.master.loud_running,
            ..master::MasterSide::default()
        };
        self.open_id += 1;
        self.output = None;
        self.engine = None;
        let dir = self.dir.take();
        self.doc = None;
        if let Some(dir) = dir {
            if self.tour.active {
                // The demo is gone when the tour ends, however it ends (PRJ-9, UI-19).
                self.tour_forget();
            } else {
                package::close_session(&dir);
            }
        }
    }

    pub fn save(&mut self) -> Res<()> {
        self.not_demo()?;
        let dir = self.dir()?.to_path_buf();
        let d = self.doc_mut()?;
        package::save(&dir, &d.project).map_err(err)?;
        d.mark_saved();
        Ok(())
    }

    pub fn save_as(&mut self, parent: &Path, name: &str) -> Res<()> {
        self.not_demo()?;
        let from = self.dir()?.to_path_buf();
        let p = self.project()?.clone();
        let dir = package::save_as(&from, parent, name, &p).map_err(err)?;
        let mut q = p;
        q.name = name.to_string();
        self.install(dir, q)
    }

    /// Autosave every 30 s with unsaved changes (PRJ-2); `force` for AI transactions.
    pub fn autosave(&mut self, force: bool) {
        if self.tour.active {
            return;
        }
        if self.last_autosave.elapsed().as_secs() < package::AUTOSAVE_SECONDS && !force {
            return;
        }
        self.last_autosave = Instant::now();
        if let (Some(dir), Some(d)) = (&self.dir, &self.doc)
            && d.dirty
        {
            let _ = package::autosave(dir, &d.project);
        }
    }

    // -----------------------------------------------------------------------------------------
    // Audio (ENG-1 … ENG-3, SET-1)
    // -----------------------------------------------------------------------------------------

    /// (Re)creates engine and output for the open project at its rate (48 kHz before the first import).
    pub fn restart_audio(&mut self) {
        self.output = None;
        self.engine = None;
        if !self.audio {
            return;
        }
        let (Some(d), Some(dir)) = (&self.doc, &self.dir) else { return };
        let rate = d.project.sample_rate.unwrap_or(48000);
        let (mut engine, ap) = Engine::new(f64::from(rate), 8192);
        // No engine without an output: its queues would fill with nothing draining them (review F3, S2).
        match Output::open(ap, rate, self.prefs.device.as_deref(), self.prefs.buffer) {
            Ok(o) => {
                self.output = Some(o);
                self.device_error = None;
                if let Err(e) = engine.load(dir, &d.project) {
                    self.device_error = Some(e.to_string());
                }
                self.engine = Some(engine);
            }
            Err(e) => self.device_error = Some(e.to_string()),
        }
    }

    pub fn set_audio_prefs(&mut self, prefs: AudioPrefs) -> Res<()> {
        std::fs::create_dir_all(&self.app_data).map_err(err)?;
        std::fs::write(self.app_data.join("audio.json"), serde_json::to_string_pretty(&prefs).map_err(err)?)
            .map_err(err)?;
        self.prefs = prefs;
        self.restart_audio();
        Ok(())
    }

    pub fn devices(&self) -> Vec<DeviceInfo> {
        stemwise_engine::device::list_devices()
    }

    /// Hands a document change to the engine: parameters, bus fader and solo on the queue (no rebuild); anything
    /// else rebuilds the graph.
    fn apply(&mut self, change: &Change) {
        // Master phase: the engine plays the mixdown; mix changes only make it stale (MDN-2).
        if self.master.phase == Phase::Master {
            if change.master {
                self.master_changed();
            }
            return;
        }
        let (Some(e), Some(d), Some(dir)) = (&mut self.engine, &self.doc, &self.dir) else { return };
        if change.structural {
            // The engine plays the document again: an A/B on a proposal or a snapshot ends (AI-2, HIS-4).
            self.ai.listening = "current".into();
            if d.project.sample_rate.map(f64::from) != Some(e.rate) && d.project.sample_rate.is_some() {
                // First import changed the rate: reopen the output.
                self.restart_audio();
                return;
            }
            if let Err(err) = e.load(dir, &d.project) {
                self.device_error = Some(err.to_string());
            }
            return;
        }
        for (t, p, v) in &change.params {
            e.set_param(t, p, *v);
        }
        if let Some(db) = change.bus_fader {
            e.set_bus_fader(db);
        }
        if change.solo {
            e.set_solo(&d.project);
        }
    }

    pub fn transport(&self) -> TransportState {
        let e = self.engine.as_ref();
        TransportState {
            playing: e.is_some_and(|e| e.playing()),
            frame: e.map_or(0, |e| e.position()),
            rate: self.doc.as_ref().and_then(|d| d.project.sample_rate).unwrap_or(48000),
            load: e.map_or(0.0, |e| e.load_ratio()),
            xruns: e.map_or(0, |e| e.shared.xruns.load(std::sync::atomic::Ordering::Relaxed)),
            device: self.output.as_ref().map(|o| o.device.clone()),
            device_rate: self.output.as_ref().map(|o| o.device_rate),
            device_error: self.device_error.clone(),
            disconnected: self
                .output
                .as_ref()
                .is_some_and(|o| o.disconnected.load(std::sync::atomic::Ordering::Relaxed)),
        }
    }

    pub fn play(&mut self) {
        if let Some(e) = &mut self.engine {
            e.play();
        }
    }

    pub fn pause(&mut self) {
        if let Some(e) = &mut self.engine {
            e.pause();
        }
    }

    /// Stop returns to where playback started (TRN-1).
    pub fn stop(&mut self) {
        if let Some(e) = &mut self.engine {
            e.stop();
        }
    }

    pub fn locate(&mut self, frame: i64) {
        if let Some(e) = &mut self.engine {
            e.locate(frame.max(0));
        }
    }

    pub fn set_loop(&mut self, region: Option<(i64, i64)>) {
        if let Some(e) = &mut self.engine {
            e.set_loop(region);
        }
    }

    /// The track whose spectrum the meters carry (FX-7).
    pub fn watch(&mut self, track: Option<&str>) {
        if let Some(e) = &mut self.engine {
            e.watch(track);
        }
    }

    pub fn meters(&mut self) -> Option<MeterSnapshot> {
        let e = self.engine.as_mut()?;
        let load = e.load_ratio();
        let (f, ids) = e.poll();
        let tracks = ids.iter().zip(f.node.iter()).take(f.nodes).map(|(id, m)| (id.clone(), *m)).collect();
        let f = &f;
        let mut bus = [0f32; 8];
        bus.copy_from_slice(&f.bus[..8]);
        let spectrum = e.spectrum();
        let master_gr = [f.bus[8], f.bus[9]];
        let free = stemwise_engine::master::MAX_FREE;
        let (master_blocks, master_bands) = (f.master[..free].to_vec(), f.master[free..].to_vec());
        Some(MeterSnapshot {
            frame: f.position,
            playing: f.playing,
            tracks,
            bus,
            load,
            spectrum,
            master_gr,
            master_blocks,
            master_bands,
        })
    }

    // -----------------------------------------------------------------------------------------
    // Edits and history (HIS-1, HIS-2, HIS-5, HIS-6)
    // -----------------------------------------------------------------------------------------

    pub fn transact(&mut self, phase: Phase, label: &str, cmds: Vec<Command>) -> Res<()> {
        let change = self.doc_mut()?.transact(phase, label, cmds, None).map_err(err)?;
        self.apply(&change);
        Ok(())
    }

    pub fn view(&mut self, cmds: Vec<Command>) -> Res<()> {
        let change = self.doc_mut()?.apply_view(cmds).map_err(err)?;
        self.apply(&change);
        Ok(())
    }

    pub fn begin_gesture(&mut self, phase: Phase, label: &str) -> Res<()> {
        self.doc_mut()?.begin_gesture(phase, label).map_err(err)
    }

    pub fn gesture(&mut self, cmds: Vec<Command>) -> Res<()> {
        let change = self.doc_mut()?.gesture_apply(cmds).map_err(err)?;
        self.apply(&change);
        Ok(())
    }

    /// One user edit as one history step (§5.6): the core computes the commands.
    pub fn edit(&mut self, e: &stemwise_core::edit::Edit, label: &str) -> Res<()> {
        let d = self.doc_mut()?;
        let cmds = stemwise_core::edit::run(&mut d.project, e).map_err(err)?;
        self.transact(e.phase(), label, cmds)
    }

    /// An edit inside the open gesture, recomputed from the gesture's start (a drag in progress).
    pub fn gesture_edit(&mut self, e: &stemwise_core::edit::Edit) -> Res<()> {
        let d = self.doc_mut()?;
        let change = d.gesture_replace(|p| stemwise_core::edit::run(p, e)).map_err(err)?;
        match change {
            Ok(c) => {
                self.apply(&c);
                Ok(())
            }
            Err(e) => {
                // The gesture is back at its start; the engine must follow.
                self.reload();
                Err(e.to_string())
            }
        }
    }

    fn reload(&mut self) {
        let c = Change { structural: true, ..Default::default() };
        self.apply(&c);
    }

    pub fn end_gesture(&mut self, cancel: bool) -> Res<()> {
        let d = self.doc_mut()?;
        let change = if cancel { d.cancel_gesture() } else { d.end_gesture() }.map_err(err)?;
        if cancel {
            self.apply(&change);
        } else if self.master.phase == Phase::Master {
            // A destination dragged to a new value: the wait for its level starts now (9v).
            self.retarget_check();
        }
        Ok(())
    }

    pub fn undo(&mut self, phase: Phase) -> Res<String> {
        let (label, change) = self.doc_mut()?.undo(phase).map_err(err)?;
        self.apply(&change);
        Ok(label)
    }

    pub fn redo(&mut self, phase: Phase) -> Res<String> {
        let (label, change) = self.doc_mut()?.redo(phase).map_err(err)?;
        self.apply(&change);
        Ok(label)
    }

    pub fn goto(&mut self, phase: Phase, position: usize) -> Res<()> {
        let change = self.doc_mut()?.goto(phase, position).map_err(err)?;
        self.apply(&change);
        Ok(())
    }

    pub fn history(&self, phase: Phase) -> Res<(Vec<HistoryEntry>, usize)> {
        let d = self.doc.as_ref().ok_or("no project is open")?;
        Ok((d.history(phase).entries(), d.history(phase).position()))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}
