// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Tauri shell: commands and events over the Stemwise crates (FOUNDATION §5.6). No domain logic here.

mod platform;
mod session;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use session::{AudioPrefs, MeterSnapshot, Res, Session, TransportState};
use stemwise_core::command::Command;
use stemwise_core::edit::Edit;
use stemwise_core::history::{HistoryEntry, Phase};
use stemwise_core::import::{self, ImportCandidate, ImportChoice};
use stemwise_core::media::Peaks;
use stemwise_core::model::Project;
use stemwise_core::package::{self, RecentProject};
use stemwise_core::params::ParamSpec;
use stemwise_engine::device::DeviceInfo;
use tauri::{AppHandle, Emitter, Manager, State};

type Shared = Arc<Mutex<Session>>;

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
    backend: &'static str,
}

use stemwise_engine::dto::ProjectEvent;

fn lock<'a>(s: &'a State<'_, Shared>) -> std::sync::MutexGuard<'a, Session> {
    s.lock().unwrap_or_else(|p| p.into_inner())
}

/// Sends the document to the UI after a change (its read-only mirror, FOUNDATION §5.3).
fn emit_project(app: &AppHandle, s: &Session) {
    if let Some(d) = &s.doc {
        let ev = ProjectEvent {
            revision: d.revision,
            project: d.project.clone(),
            can_undo: d.history(Phase::Mix).can_undo(),
            can_redo: d.history(Phase::Mix).can_redo(),
            dirty: d.dirty,
            path: s.dir.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
            missing: s
                .dir
                .as_ref()
                .map(|dir| stemwise_core::package::missing_media(dir, &d.project))
                .unwrap_or_default(),
            demo: s.tour.active,
        };
        let _ = app.emit("project-changed", ev);
    }
}

/// Sends the master phase's state to the UI.
fn emit_master(app: &AppHandle, s: &Session) {
    let _ = app.emit("master-status", s.master_status());
}

/// Starts (or keeps) the background work of the master phase: the mixdown cache, then the loudness of the current
/// master state, again while the state keeps changing (MAS-2). At most one such thread.
fn master_work(app: &AppHandle, shared: &Shared) {
    {
        let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
        // The cache follows the current mixdown and references (a new bounce or reference replaces it).
        // A preparation that failed waits for "Try again" or a new bounce (9j), not for the next edit.
        let need_cache = s.cache_needed() && !s.master.prep.as_ref().is_some_and(|p| p.failed());
        if s.master.loud_running || (!need_cache && !s.master.analysing) {
            return;
        }
        s.master.loud_running = true;
    }
    let (app, shared) = (app.clone(), shared.clone());
    let _ = std::thread::Builder::new().name("stemwise-master".into()).spawn(move || {
        enum Step {
            Cache(std::path::PathBuf, Box<stemwise_core::Project>, u64, Option<Vec<f32>>),
            Loudness {
                epoch: u64,
                cache: std::sync::Arc<session::master::MasterCache>,
                master: Box<stemwise_core::model::MasterState>,
                sections: Vec<stemwise_core::model::Section>,
                start: Option<f64>,
                dir: std::path::PathBuf,
                mix_hash: String,
            },
        }
        loop {
            // What to do next is decided under the lock, and so is stopping: a change that arrives after this
            // thread decided to stop finds `loud_running` false and starts a new one (F8 review, S4).
            let step = {
                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                let need_cache = s.cache_needed() && !s.master.prep.as_ref().is_some_and(|p| p.failed());
                let number = s.project().ok().and_then(|p| p.master.mixdown.as_ref().map(|m| m.number));
                let step = match (s.dir.clone(), s.doc.as_ref().map(|d| d.project.clone())) {
                    (Some(dir), Some(p)) if need_cache => {
                        let bounced = number.and_then(|n| s.take_bounced(n));
                        // The overlay's steps (9d–9e): what is saved in the package is done at once.
                        s.master.prep = session::master::Prep::plan(&dir, &p, bounced.is_some());
                        emit_master(&app, &s);
                        Some(Step::Cache(dir, Box::new(p), s.open_id, bounced))
                    }
                    (Some(dir), Some(p)) if s.master.analysing => {
                        s.loudness_job().map(|(epoch, cache, m, start)| Step::Loudness {
                            epoch,
                            cache,
                            master: Box::new(m),
                            sections: p.sections.clone(),
                            start,
                            dir,
                            mix_hash: p.master.mixdown.as_ref().map(|m| m.mix_hash.clone()).unwrap_or_default(),
                        })
                    }
                    _ => None,
                };
                if step.is_none() {
                    s.master.loud_running = false;
                    // Nothing left to do: a preparation still listed is over (unless it failed, 9j).
                    if s.master.prep.as_ref().is_some_and(|p| !p.failed()) {
                        s.master.prep = None;
                        emit_master(&app, &s);
                    }
                    // Nothing to measure against (no project or no mixdown): nothing is being analysed.
                    if s.master.cache.is_none() || s.doc.is_none() {
                        s.master.analysing = false;
                    }
                }
                step
            };
            match step {
                None => break,
                Some(Step::Cache(dir, project, owner, bounced)) => {
                    // Each step reports as it goes (only to the project it works for).
                    let rep = |key: session::master::PrepKey, ev: session::master::PrepEv| {
                        let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                        if s.owns(owner)
                            && let Some(p) = s.master.prep.as_mut()
                        {
                            p.event(&key, ev);
                            emit_master(&app, &s);
                        }
                    };
                    let r = session::master::load_cache(&dir, &project, bounced, &rep);
                    let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                    match r {
                        // Only for the project and the mixdown it was read for.
                        Ok(c) if s.owns(owner) && s.project().is_ok_and(|p| session::master::cache_fits(&c, p)) => {
                            s.set_cache(c);
                            emit_master(&app, &s);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            if s.owns(owner) {
                                // 9j: the overlay says what failed and offers "Bounce again" or "Try again".
                                if let Some(p) = s.master.prep.as_mut() {
                                    p.fail(e);
                                }
                                s.master.analysing = false;
                                emit_master(&app, &s);
                            }
                            s.master.loud_running = false;
                            break;
                        }
                    }
                }
                Some(Step::Loudness { epoch, cache, master, sections, start, dir, mix_hash }) => {
                    use session::master::{PrepEv, PrepKey};
                    let step = |ev: PrepEv| {
                        let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                        // A destination change waits under its own overlay (9v).
                        let retarget = matches!(&ev, PrepEv::Progress(f) if s.retarget_progress(epoch, *f));
                        let prep = s.master.prep.as_mut().map(|p| p.event(&PrepKey::Loudness, ev)).is_some();
                        if prep || retarget {
                            emit_master(&app, &s);
                        }
                    };
                    // A state measured before (in this session or an earlier one) is not measured again.
                    let key = session::master::loudness_key(&cache, &master, &sections, &mix_hash);
                    let (gain, measures, wave) = match session::master::loudness_cached(&dir, &key) {
                        Some(r) => {
                            step(PrepEv::Done(Some("saved")));
                            r
                        }
                        None => {
                            step(PrepEv::Start);
                            let r = session::master::compute_loudness(&cache, &master, &sections, start, &mut |f| {
                                step(PrepEv::Progress(f));
                            });
                            session::master::loudness_keep(&dir, key, r.0, &r.1, &r.2);
                            step(PrepEv::Done(None));
                            r
                        }
                    };
                    let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                    // Dropped when the state moved on (the epoch never repeats, even across projects).
                    s.loudness_done(epoch, gain, measures, wave);
                    emit_master(&app, &s);
                }
            }
        }
    });
}

/// Sends the assistant's state to the UI (AI-12).
fn emit_ai(app: &AppHandle, s: &Session) {
    let _ = app.emit("ai-status", s.ai_status());
}

macro_rules! mutate {
    ($app:expr, $state:expr, |$s:ident| $body:expr) => {{
        let r = {
            let mut $s = lock(&$state);
            let r = $body;
            emit_project(&$app, &$s);
            if $s.reviewing() {
                emit_ai(&$app, &$s);
            }
            if $s.master.phase == Phase::Master {
                emit_master(&$app, &$s);
            }
            r
        };
        master_work(&$app, $state.inner());
        r
    }};
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo { version: env!("CARGO_PKG_VERSION"), backend: "tauri" }
}

#[tauri::command]
fn recent_projects(state: State<'_, Shared>) -> Vec<RecentProject> {
    package::recent_projects(&lock(&state).app_data)
}

#[tauri::command]
fn create_project(app: AppHandle, state: State<'_, Shared>, parent: String, name: String) -> Res<()> {
    mutate!(app, state, |s| s.create(&PathBuf::from(parent), &name))
}

#[tauri::command]
fn open_project(app: AppHandle, state: State<'_, Shared>, path: String) -> Res<bool> {
    let r = mutate!(app, state, |s| s.open(&PathBuf::from(path)));
    if r.is_ok() {
        build_caches(&app, state.inner());
    }
    r
}

/// Design 13c: waveform overviews (and the decoded cache of non-WAV files) missing from a package — a copy, a
/// cleared `cache/` — are built in the background, file by file; the UI redraws each clip as its file is ready.
fn build_caches(app: &AppHandle, shared: &Shared) {
    let (dir, media, owner) = {
        let s = lock_shared(shared);
        let (Some(dir), Some(d)) = (s.dir.clone(), s.doc.as_ref()) else { return };
        (dir, d.project.media.clone(), s.open_id)
    };
    let todo: Vec<_> = media
        .into_iter()
        .filter(|m| dir.join(&m.path).is_file() && !stemwise_core::media::peaks_path(&dir, &m.id).exists())
        .collect();
    if todo.is_empty() {
        return;
    }
    let (app, shared) = (app.clone(), shared.clone());
    let _ = std::thread::Builder::new().name("stemwise-peaks".into()).spawn(move || {
        for m in todo {
            if !lock_shared(&shared).owns(owner) {
                return;
            }
            if stemwise_core::media::ensure_cache(&dir, &m).is_ok() {
                let _ = app.emit("media-ready", m.id.clone());
            }
        }
    });
}

fn lock_shared(s: &Shared) -> std::sync::MutexGuard<'_, Session> {
    s.lock().unwrap_or_else(|e| e.into_inner())
}

#[tauri::command]
fn recover_project(app: AppHandle, state: State<'_, Shared>, accept: bool) -> Res<()> {
    mutate!(app, state, |s| s.recover(accept))
}

#[tauri::command]
fn close_project(state: State<'_, Shared>) {
    lock(&state).close();
}

#[tauri::command]
fn save_project(app: AppHandle, state: State<'_, Shared>) -> Res<()> {
    mutate!(app, state, |s| s.save())
}

#[tauri::command]
fn save_project_as(app: AppHandle, state: State<'_, Shared>, parent: String, name: String) -> Res<()> {
    let r = mutate!(app, state, |s| s.save_as(&PathBuf::from(parent), &name));
    // The copy has no `cache/`: its overviews are built again (13c).
    if r.is_ok() {
        build_caches(&app, state.inner());
    }
    r
}

#[tauri::command]
fn get_project(app: AppHandle, state: State<'_, Shared>) -> Res<Project> {
    let s = lock(&state);
    emit_project(&app, &s);
    s.project().cloned()
}

/// Probes files for the import screen (design 2c). Adding to the open project compares them with its rate; a new
/// project does not exist yet while its files are chosen, so the screen passes the rate of the first file it
/// already lists (or none).
#[tauri::command]
fn import_candidates(
    state: State<'_, Shared>,
    paths: Vec<String>,
    adding: bool,
    rate: Option<u32>,
) -> Res<Vec<ImportCandidate>> {
    let paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    if adding {
        let s = lock(&state);
        return Ok(import::candidates(s.project()?, &paths));
    }
    Ok(import::candidates_at(rate, &paths))
}

#[tauri::command]
fn import_files(app: AppHandle, state: State<'_, Shared>, choices: Vec<ImportChoice>) -> Res<()> {
    mutate!(app, state, |s| {
        let dir = s.dir()?.to_path_buf();
        let n = choices.len();
        let cmds = {
            let d = s.doc.as_mut().ok_or("no project is open")?;
            import::import(&dir, &mut d.project, &choices).map_err(|e| e.to_string())?
        };
        s.transact(Phase::Mix, &format!("Import {n} file{}", if n == 1 { "" } else { "s" }), cmds)
    })
}

/// UI-6 (design 13d): puts back a missing file. `true` when the clips now use another file (one undoable step),
/// `false` when the same audio was restored in place.
#[tauri::command]
fn relink_media(app: AppHandle, state: State<'_, Shared>, media: String, path: String) -> Res<bool> {
    mutate!(app, state, |s| s.relink_media(&media, std::path::Path::new(&path)))
}

#[tauri::command]
fn param_specs() -> Vec<ParamSpec> {
    stemwise_core::params::all_specs()
}

#[tauri::command]
fn set_param(
    app: AppHandle,
    state: State<'_, Shared>,
    track: String,
    param: String,
    value: f64,
    label: String,
) -> Res<()> {
    mutate!(app, state, |s| s.transact(Phase::Mix, &label, vec![Command::SetParam { track, param, value }]))
}

#[tauri::command]
fn begin_gesture(state: State<'_, Shared>, label: String, phase: Option<Phase>) -> Res<()> {
    lock(&state).begin_gesture(phase.unwrap_or(Phase::Mix), &label)
}

#[tauri::command]
fn gesture_param(state: State<'_, Shared>, track: String, param: String, value: f64) -> Res<()> {
    // No project event during a gesture: the UI already shows the value it sent.
    lock(&state).gesture(vec![Command::SetParam { track, param, value }])
}

#[tauri::command]
fn end_gesture(app: AppHandle, state: State<'_, Shared>, cancel: bool) -> Res<()> {
    mutate!(app, state, |s| s.end_gesture(cancel))
}

#[tauri::command]
fn edit(app: AppHandle, state: State<'_, Shared>, edit: Edit, label: String) -> Res<()> {
    mutate!(app, state, |s| s.edit(&edit, &label))
}

#[tauri::command]
fn gesture_edit(app: AppHandle, state: State<'_, Shared>, edit: Edit) -> Res<()> {
    // Structural edits during a drag (an automation offset) need the UI to see the result.
    mutate!(app, state, |s| s.gesture_edit(&edit))
}

#[tauri::command]
fn set_lane_visible(app: AppHandle, state: State<'_, Shared>, track: String, param: String, visible: bool) -> Res<()> {
    mutate!(app, state, |s| {
        let p = s.project()?;
        let mut t = p.track(&track).ok_or_else(|| format!("no track {track}"))?.clone();
        match t.automation.iter_mut().find(|l| l.param == param) {
            Some(l) => l.visible = visible,
            None if visible => t.automation.push(stemwise_core::AutomationLane {
                param: param.clone(),
                points: Vec::new(),
                author: stemwise_core::Author::User,
                visible: true,
            }),
            None => return Ok(()),
        }
        s.view(vec![Command::SetTrack { track: Box::new(t) }])
    })
}

#[tauri::command]
fn watch_spectrum(state: State<'_, Shared>, track: Option<String>) {
    lock(&state).watch(track.as_deref());
}

#[tauri::command]
fn set_view(app: AppHandle, state: State<'_, Shared>, track: String, what: String, on: bool) -> Res<()> {
    mutate!(app, state, |s| {
        // Mute is a parameter (ramped in the engine), solo a mask: neither rebuilds the graph (ENG-3, TRK-2).
        let cmd = match what.as_str() {
            "mute" => Command::SetParam { track, param: "mute".into(), value: if on { 1.0 } else { 0.0 } },
            "solo" => Command::SetSolo { track, on },
            _ => return Err(format!("unknown view state {what}")),
        };
        s.view(vec![cmd])
    })
}

#[tauri::command]
fn undo(app: AppHandle, state: State<'_, Shared>, phase: Phase) -> Res<String> {
    mutate!(app, state, |s| s.undo(phase))
}

#[tauri::command]
fn redo(app: AppHandle, state: State<'_, Shared>, phase: Phase) -> Res<String> {
    mutate!(app, state, |s| s.redo(phase))
}

#[tauri::command]
fn history_goto(app: AppHandle, state: State<'_, Shared>, phase: Phase, position: usize) -> Res<()> {
    mutate!(app, state, |s| s.goto(phase, position))
}

#[tauri::command]
fn history(state: State<'_, Shared>, phase: Phase) -> Res<(Vec<HistoryEntry>, usize)> {
    lock(&state).history(phase)
}

#[tauri::command]
fn play(state: State<'_, Shared>) {
    lock(&state).play();
}

#[tauri::command]
fn pause(state: State<'_, Shared>) {
    lock(&state).pause();
}

#[tauri::command]
fn stop(state: State<'_, Shared>) {
    lock(&state).stop();
}

#[tauri::command]
fn locate(state: State<'_, Shared>, frame: i64) {
    lock(&state).locate(frame);
}

#[tauri::command]
fn set_loop(state: State<'_, Shared>, start: Option<i64>, end: Option<i64>) {
    lock(&state).set_loop(start.zip(end));
}

#[tauri::command]
fn transport_state(state: State<'_, Shared>) -> TransportState {
    lock(&state).transport()
}

#[tauri::command]
fn audio_devices(state: State<'_, Shared>) -> Vec<DeviceInfo> {
    lock(&state).devices()
}

#[tauri::command]
fn audio_prefs(state: State<'_, Shared>) -> AudioPrefs {
    lock(&state).prefs.clone()
}

#[tauri::command]
fn set_audio_prefs(state: State<'_, Shared>, prefs: AudioPrefs) -> Res<()> {
    lock(&state).set_audio_prefs(prefs)
}

/// Waveform columns of a media file (TRN-3): `columns` (min, max) pairs per channel over `[start, end)` frames.
#[tauri::command]
fn peaks(state: State<'_, Shared>, media: String, start: u64, end: u64, columns: usize) -> Res<Vec<Vec<[f32; 2]>>> {
    let s = lock(&state);
    let dir = s.dir()?.to_path_buf();
    let p = Peaks::load(&dir, &media).ok_or("peaks not available")?;
    Ok(p.query(start, end, columns.min(8192)))
}

/// Self-test of the real app (A-011): `STEMWISE_SELFTEST_PROJECT` names a package; the UI runs the main flows
/// through the same commands and reports here; the report goes to `STEMWISE_SELFTEST_OUT` and the app quits.
// ---------------------------------------------------------------------------------------------
// Assistant (F7)
// ---------------------------------------------------------------------------------------------

#[tauri::command]
fn ai_status(state: State<'_, Shared>) -> stemwise_ai::dto::AiStatus {
    lock(&state).ai_status()
}

#[tauri::command]
fn ai_estimate(state: State<'_, Shared>, revision: bool) -> Res<stemwise_ai::budget::CostEstimate> {
    lock(&state).ai_estimate(revision)
}

#[tauri::command]
fn ai_start(app: AppHandle, state: State<'_, Shared>, request: Option<String>, scope: Option<Vec<String>>) -> Res<()> {
    let job = {
        let mut s = lock(&state);
        let job = s.ai_prepare(request, scope)?;
        emit_ai(&app, &s);
        job
    };
    let shared: Shared = state.inner().clone();
    std::thread::Builder::new()
        .name("stemwise-ai".into())
        .spawn(move || {
            let cfg = job.config.clone();
            let estimate = move |i: u64, o: u64| cfg.estimate(i, o);
            let last = std::sync::Mutex::new((None::<stemwise_ai::mixing::AiPhase>, 0u32, -1.0f64));
            let progress = |p: stemwise_ai::mixing::AiProgress| {
                // Throttled: a new phase or iteration, or 2 % more of the current one.
                let mut l = last.lock().unwrap_or_else(|e| e.into_inner());
                if l.0 == Some(p.phase) && l.1 == p.iteration && (p.fraction - l.2).abs() < 0.02 {
                    return;
                }
                *l = (Some(p.phase), p.iteration, p.fraction);
                drop(l);
                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                s.ai_progress(p);
                emit_ai(&app, &s);
            };
            let scope = job.opts.scope.clone();
            let request = job.opts.request.clone();
            let r = stemwise_ai::mixing::run_mix(
                &job.package,
                &job.project,
                job.decider.as_ref(),
                &estimate,
                &job.opts,
                &progress,
                &job.cancel,
            );
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            let res = s.ai_finished(scope, request, r);
            emit_ai(&app, &s);
            emit_project(&app, &s);
            if let Err(m) = res {
                let _ = app.emit("ai-error", m);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// ANA-20: the levels the Explain view reads. Answers at once when they are known for the mix as it is; otherwise
/// measures them in the background and announces them with a `mix-levels` event.
#[tauri::command]
fn mix_levels(app: AppHandle, state: State<'_, Shared>) -> Res<Option<stemwise_ai::balance::MixLevels>> {
    use session::levels::LevelsAsk;
    let job = match lock(&state).levels_ask()? {
        LevelsAsk::Ready(l) => return Ok(Some(l)),
        LevelsAsk::Running => return Ok(None),
        LevelsAsk::Start(job) => job,
    };
    let shared: Shared = state.inner().clone();
    std::thread::Builder::new()
        .name("stemwise-levels".into())
        .spawn(move || {
            let levels = stemwise_ai::balance::measure_levels(&job.package, &job.project, &job.cancel);
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(l) = s.levels_done(&job, levels) {
                let _ = app.emit("mix-levels", l);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(None)
}

#[tauri::command]
fn ai_cancel(app: AppHandle, state: State<'_, Shared>) {
    let mut s = lock(&state);
    s.ai_cancel();
    // Cancelling the wait for the tour's proposal changes the assistant's state at once.
    if s.tour.active {
        emit_ai(&app, &s);
    }
}

#[tauri::command]
fn ai_apply(app: AppHandle, state: State<'_, Shared>, only: Option<Vec<String>>, trims: bool) -> Res<String> {
    let r = mutate!(app, state, |s| s.ai_apply(only, trims));
    emit_ai(&app, &lock(&state));
    r
}

#[tauri::command]
fn ai_discard(app: AppHandle, state: State<'_, Shared>) -> Res<()> {
    let r = mutate!(app, state, |s| s.ai_discard());
    emit_ai(&app, &lock(&state));
    r
}

/// AI-23: a proposal received earlier goes back in review.
#[tauri::command]
fn ai_reopen(app: AppHandle, state: State<'_, Shared>, id: String) -> Res<()> {
    let r = mutate!(app, state, |s| s.ai_reopen(&id));
    emit_ai(&app, &lock(&state));
    r
}

#[tauri::command]
fn ai_listen(app: AppHandle, state: State<'_, Shared>, what: String) -> Res<()> {
    let mut s = lock(&state);
    let r = s.ai_listen(&what);
    emit_ai(&app, &s);
    r
}

#[tauri::command]
fn ai_detail(state: State<'_, Shared>, id: String) -> Res<stemwise_ai::dto::CallDetail> {
    lock(&state).ai_detail(&id)
}

#[tauri::command]
fn ai_export(state: State<'_, Shared>, path: String, json: bool) -> Res<()> {
    lock(&state).ai_export(std::path::Path::new(&path), json)
}

#[tauri::command]
fn ai_set_config(app: AppHandle, state: State<'_, Shared>, config: stemwise_ai::openrouter::AiConfig) -> Res<()> {
    let mut s = lock(&state);
    let r = s.ai_set_config(config);
    emit_ai(&app, &s);
    r
}

/// The provider's pages the setup links to (AI-24). A fixed list: the interface names one, it cannot pass an address.
fn provider_page(which: &str) -> Option<&'static str> {
    match which {
        "home" => Some("https://openrouter.ai"),
        "keys" => Some("https://openrouter.ai/settings/keys"),
        "credits" => Some("https://openrouter.ai/settings/credits"),
        _ => None,
    }
}

fn open_in_browser(app: &AppHandle, url: &str) -> Res<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).map_err(|e| format!("could not open the browser: {e}"))
}

#[tauri::command]
fn ai_open_page(app: AppHandle, which: String) -> Res<()> {
    open_in_browser(&app, provider_page(&which).ok_or("no such page")?)
}

/// AI-24: connects an OpenRouter account from the browser. The approval page opens at once; the wait for the
/// browser to come back runs on its own thread and ends with an "ai-connect" event: "connected", or why not
/// ("denied", "expired", "offline", "refused", "local", "cancelled").
#[tauri::command]
fn ai_connect_start(app: AppHandle, state: State<'_, Shared>) -> Res<()> {
    let (connect, cancel) = lock(&state).ai_connect_begin().map_err(|e| e.code().to_string())?;
    if let Err(e) = open_in_browser(&app, &connect.auth_url) {
        lock(&state).ai_connect_cancel();
        return Err(e);
    }
    let shared: Shared = state.inner().clone();
    std::thread::Builder::new()
        .name("stemwise-connect".into())
        .spawn(move || {
            let key = connect.wait(&cancel, stemwise_ai::connect::VALID_FOR);
            let mut s = shared.lock().unwrap_or_else(|p| p.into_inner());
            let outcome = match s.ai_connect_finished(&cancel, key) {
                Ok(()) => "connected".to_string(),
                Err(code) => code,
            };
            emit_ai(&app, &s);
            let _ = app.emit("ai-connect", outcome);
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn ai_connect_cancel(state: State<'_, Shared>) {
    lock(&state).ai_connect_cancel();
}

/// 14b: "Open the page again" while waiting for the approval.
#[tauri::command]
fn ai_connect_reopen(app: AppHandle, state: State<'_, Shared>) -> Res<()> {
    let url = lock(&state).ai.connect_url.clone().ok_or("no connection is waiting")?;
    open_in_browser(&app, &url)
}

#[tauri::command]
fn ai_set_key(app: AppHandle, state: State<'_, Shared>, key: Option<String>) -> Res<()> {
    let mut s = lock(&state);
    let r = s.ai_set_key(key);
    emit_ai(&app, &s);
    r
}

#[tauri::command]
fn set_spend_cap(app: AppHandle, state: State<'_, Shared>, usd: Option<f64>) -> Res<()> {
    mutate!(app, state, |s| s.set_spend_cap(usd))
}

#[tauri::command]
fn set_normalize_trim(app: AppHandle, state: State<'_, Shared>, on: bool) -> Res<()> {
    mutate!(app, state, |s| s.set_normalize_trim(on))
}

#[tauri::command]
fn set_allow_structure(app: AppHandle, state: State<'_, Shared>, on: bool) -> Res<()> {
    mutate!(app, state, |s| s.set_allow_structure(on))
}

/// AI-27: the ideas ticked in the review.
#[tauri::command]
fn ai_set_ideas(app: AppHandle, state: State<'_, Shared>, ideas: Vec<String>) -> Res<()> {
    let mut s = lock(&state);
    let r = s.ai_set_ideas(ideas);
    emit_ai(&app, &s);
    r
}

#[tauri::command]
fn set_allow_ideas(app: AppHandle, state: State<'_, Shared>, on: bool) -> Res<()> {
    mutate!(app, state, |s| s.set_allow_ideas(on))
}

#[tauri::command]
fn snapshot_save(app: AppHandle, state: State<'_, Shared>, name: String) -> Res<()> {
    mutate!(app, state, |s| s.snapshot_save(&name))
}

#[tauri::command]
fn snapshot_restore(app: AppHandle, state: State<'_, Shared>, id: String) -> Res<()> {
    let r = mutate!(app, state, |s| s.snapshot_restore(&id));
    emit_ai(&app, &lock(&state));
    r
}

#[tauri::command]
fn snapshot_delete(app: AppHandle, state: State<'_, Shared>, id: String) -> Res<()> {
    mutate!(app, state, |s| s.snapshot_delete(&id))
}

#[tauri::command]
fn add_reference(app: AppHandle, state: State<'_, Shared>, path: String) -> Res<()> {
    mutate!(app, state, |s| s.add_reference(std::path::Path::new(&path)))
}

#[tauri::command]
fn remove_reference(app: AppHandle, state: State<'_, Shared>, id: String) -> Res<()> {
    mutate!(app, state, |s| s.remove_reference(&id))
}

// ---------------------------------------------------------------------------------------------
// Master phase (F8)
// ---------------------------------------------------------------------------------------------

#[tauri::command]
fn master_status(state: State<'_, Shared>) -> stemwise_export::dto::MasterStatus {
    lock(&state).master_status()
}

#[tauri::command]
fn set_phase(app: AppHandle, state: State<'_, Shared>, master: bool) -> Res<()> {
    let r = {
        let mut s = lock(&state);
        let r = s.set_phase(master);
        emit_master(&app, &s);
        r
    };
    master_work(&app, state.inner());
    r
}

/// "Try again" after the master could not be prepared (9j): the preparation starts over.
#[tauri::command]
fn master_prepare_retry(app: AppHandle, state: State<'_, Shared>) {
    {
        let mut s = lock(&state);
        s.master.prep = None;
        s.master.cache = None;
        emit_master(&app, &s);
    }
    master_work(&app, state.inner());
}

#[tauri::command]
fn master_listen(app: AppHandle, state: State<'_, Shared>, what: String) -> Res<()> {
    let mut s = lock(&state);
    let r = s.master_listen(&what);
    emit_master(&app, &s);
    r
}

#[tauri::command]
fn master_listen_same(app: AppHandle, state: State<'_, Shared>, on: bool) -> Res<()> {
    let mut s = lock(&state);
    let r = s.master_listen_same(on);
    emit_master(&app, &s);
    r
}

/// MAS-15, MAS-12: a block added, removed or moved, or the bands of a multiband changed, as one master step.
#[tauri::command]
fn master_chain(app: AppHandle, state: State<'_, Shared>, op: stemwise_core::model::ChainOp) -> Res<Option<String>> {
    let mut s = lock(&state);
    let r = s.master_chain(&op);
    emit_master(&app, &s);
    emit_project(&app, &s);
    r
}

/// Solo of a multiband's bands (15d): listening only.
#[tauri::command]
fn master_solo(app: AppHandle, state: State<'_, Shared>, block: Option<String>, mask: u8) -> Res<()> {
    let mut s = lock(&state);
    let r = s.master_solo(block.as_deref(), mask);
    emit_master(&app, &s);
    r
}

/// Listening to what a de-esser takes (FX-14): listening only.
#[tauri::command]
fn listen_block(state: State<'_, Shared>, track: String, block: String, on: bool) -> Res<()> {
    lock(&state).listen_block(&track, &block, on)
}

#[tauri::command]
fn master_job_cancel(state: State<'_, Shared>) {
    lock(&state).job_cancel();
}

/// MDN-1: bounce the mix to `mixdowns/` on a thread with progress (8b); the result (8c) arrives as master status.
#[tauri::command]
fn bounce_start(app: AppHandle, state: State<'_, Shared>) -> Res<()> {
    let ((cancel, owner), dir, project) = {
        let mut s = lock(&state);
        let dir = s.dir()?.to_path_buf();
        let project = s.project()?.clone();
        // A mixdown with silent clips would be wrong without saying so (UI-6).
        let missing = stemwise_core::package::missing_media(&dir, &project);
        if !missing.is_empty() {
            return Err(format!(
                "{} audio file{} missing: locate {} before bouncing",
                missing.len(),
                if missing.len() == 1 { " is" } else { "s are" },
                if missing.len() == 1 { "it" } else { "them" }
            ));
        }
        let c = s.job_start("bounce")?;
        emit_master(&app, &s);
        (c, dir, project)
    };
    let shared: Shared = state.inner().clone();
    std::thread::Builder::new()
        .name("stemwise-bounce".into())
        .spawn(move || {
            let mut last = 0.0;
            let r = stemwise_engine::mixdown::bounce(&dir, &project, &mut |f| {
                if f - last >= 0.02 || f >= 1.0 {
                    last = f;
                    let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                    s.job_progress(owner, f);
                    emit_master(&app, &s);
                }
                !cancel.load(std::sync::atomic::Ordering::Relaxed)
            });
            {
                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                s.job_end(owner);
                // The project was closed meanwhile: its mixdown file stays in its package, unrecorded (S6).
                if !s.owns(owner) {
                    return;
                }
                match r {
                    Ok((md, samples)) => {
                        let _ = s.bounce_done(md, samples);
                        emit_project(&app, &s);
                    }
                    Err(e) => {
                        let _ = app.emit(
                            "ai-error",
                            if e.to_string() == "cancelled" {
                                "bounce cancelled".to_string()
                            } else {
                                format!("the bounce failed: {e}")
                            },
                        );
                    }
                }
                emit_master(&app, &s);
            }
            master_work(&app, &shared);
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// MAS-7 … MAS-10: export in the chosen formats with the project's settings and metadata (10b, 10c).
#[tauri::command]
fn export_start(app: AppHandle, state: State<'_, Shared>) -> Res<()> {
    let ((cancel, owner), dir, project, folder, cache) = {
        let mut s = lock(&state);
        let cache = s.master.cache.clone().ok_or("the mixdown is still being measured")?;
        let dir = s.dir()?.to_path_buf();
        let project = s.project()?.clone();
        let folder = s.export_folder()?;
        let c = s.job_start("export")?;
        emit_master(&app, &s);
        (c, dir, project, folder, cache)
    };
    let shared: Shared = state.inner().clone();
    std::thread::Builder::new()
        .name("stemwise-export".into())
        .spawn(move || {
            let m = &project.master;
            let x = &m.export;
            let cover_bytes = m.metadata.cover.as_ref().and_then(|c| std::fs::read(dir.join(c)).ok());
            let mime = m
                .metadata
                .cover
                .as_deref()
                .map_or("image/jpeg", |c| if c.ends_with(".png") { "image/png" } else { "image/jpeg" });
            let base: String = project
                .name
                .chars()
                .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
                .collect();
            let base = if base.trim().is_empty() { "Untitled".to_string() } else { base };
            let settings = stemwise_export::pipeline::Settings {
                wav: x.wav.then_some(x.bit_depth),
                flac: x.flac.then_some(x.bit_depth.min(24)),
                aac: x.aac.then_some(x.aac_kbps),
                rate: x.sample_rate,
                dither: !m.chain.dither.bypass,
                folder: &folder,
                basename: base.trim(),
                meta: &m.metadata,
                cover: cover_bytes.as_deref().map(|d| stemwise_export::tags::Cover { data: d, mime }),
                when_secs: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                sections: &project.sections,
            };
            let tb = cache.target.as_ref().and_then(|t| t.bands_db.as_ref());
            let r = stemwise_export::pipeline::export(
                &cache.samples,
                cache.rate,
                m,
                cache.measures.bands_db.as_ref(),
                tb,
                &settings,
                &mut |f| {
                    let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                    s.job_progress(owner, f);
                    emit_master(&app, &s);
                    !cancel.load(std::sync::atomic::Ordering::Relaxed) && s.owns(owner)
                },
            );
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            s.job_end(owner);
            if !s.owns(owner) {
                return;
            }
            match r {
                Ok(rep) => s.export_done(rep),
                Err(stemwise_export::ExportError::Cancelled) => {
                    let _ = app.emit("ai-error", "export cancelled: no files were kept".to_string());
                }
                Err(e) => {
                    let _ = app.emit("ai-error", format!("the export failed: {e}"));
                }
            }
            emit_master(&app, &s);
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// MAS-6: master AI or master revision on a thread; the proposal is measured before review (A-033).
#[tauri::command]
fn master_ai_start(app: AppHandle, state: State<'_, Shared>, request: Option<String>) -> Res<()> {
    let (decider, cfg, cache, dir, project, (cancel, owner)) = {
        let mut s = lock(&state);
        let (d, cfg, cache) = s.master_ai_prepare()?;
        let c = s.job_start("master_ai")?;
        emit_master(&app, &s);
        (d, cfg, cache, s.dir()?.to_path_buf(), s.project()?.clone(), c)
    };
    let start = lock(&state).loudness_start();
    let shared: Shared = state.inner().clone();
    std::thread::Builder::new()
        .name("stemwise-master-ai".into())
        .spawn(move || {
            let estimate = move |i: u64, o: u64| cfg.estimate(i, o);
            let progress = |p: stemwise_ai::mixing::AiProgress| {
                use stemwise_ai::mixing::AiPhase;
                // The current master is measured first, then each plan is decided and rendered (MAS-6).
                let step = match p.phase {
                    AiPhase::Analysis => 0.0,
                    AiPhase::Decision => 1.0 + 2.0 * f64::from(p.iteration.saturating_sub(1)),
                    AiPhase::Render | AiPhase::Evaluation => 2.0 * f64::from(p.iteration),
                };
                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                s.job_progress(owner, (step / (1.0 + 2.0 * f64::from(p.of.max(1)))).min(1.0));
                emit_master(&app, &s);
            };
            let input = stemwise_ai::mastering::MasterInput {
                samples: &cache.samples,
                rate: cache.rate,
                measures: &cache.measures,
                target: cache.target.as_ref(),
                references: cache.references.iter().map(|r| (r.lufs, r.true_peak_db)).collect(),
                start_gain_db: start,
            };
            let r = stemwise_ai::mastering::run_master(
                &dir,
                &project,
                &input,
                decider.as_ref(),
                &estimate,
                request.clone(),
                &progress,
                &cancel,
            );
            // The call already found the proposal's loudness gain: the measure for the workspace starts from it.
            let after = r.as_ref().ok().and_then(|p| {
                let from = p.evaluation.as_ref().map(|e| e.loudness_gain_db).or(start);
                session::master::measure_master(&cache, &p.preview, &project.sections, from)
            });
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            s.job_end(owner);
            if !s.owns(owner) {
                return;
            }
            let res = s.master_ai_finished(r, after, request, false);
            emit_master(&app, &s);
            emit_project(&app, &s);
            if let Err(m) = res {
                let _ = app.emit("ai-error", m);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// AI-23: a master proposal received earlier goes back in review, measured on a thread like a new one (A-033).
#[tauri::command]
fn master_ai_reopen(app: AppHandle, state: State<'_, Shared>, id: String) -> Res<()> {
    let (proposal, cache, (_cancel, owner)) = {
        let mut s = lock(&state);
        let (p, cache) = s.master_ai_reopen_prepare(&id)?;
        let c = s.job_start("master_ai")?;
        emit_master(&app, &s);
        (p, cache, c)
    };
    let sections = lock(&state).project()?.sections.clone();
    let start = lock(&state).loudness_start();
    let shared: Shared = state.inner().clone();
    std::thread::Builder::new()
        .name("stemwise-master-reopen".into())
        .spawn(move || {
            let after = session::master::measure_master(&cache, &proposal.preview, &sections, start);
            let request = proposal.record.request.clone();
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            s.job_end(owner);
            if !s.owns(owner) {
                return;
            }
            let res = s.master_ai_finished(Ok(proposal), after, request, true);
            emit_master(&app, &s);
            emit_project(&app, &s);
            if let Err(m) = res {
                let _ = app.emit("ai-error", m);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn master_ai_apply(app: AppHandle, state: State<'_, Shared>, only: Option<Vec<String>>) -> Res<String> {
    mutate!(app, state, |s| s.master_ai_apply(only))
}

/// The waveform of the master workspace: mixdown and measured master, peaks and RMS per column.
#[tauri::command]
fn master_wave(state: State<'_, Shared>, columns: usize) -> stemwise_export::dto::MasterWave {
    lock(&state).master_wave(columns)
}

#[tauri::command]
fn mixdown_peaks(state: State<'_, Shared>, columns: usize) -> Vec<[f32; 2]> {
    lock(&state).mixdown_peaks(columns)
}

#[tauri::command]
fn set_cover(app: AppHandle, state: State<'_, Shared>, path: Option<String>) -> Res<()> {
    mutate!(app, state, |s| s.set_cover(path.as_deref().map(std::path::Path::new)))
}

#[tauri::command]
fn master_ai_discard(app: AppHandle, state: State<'_, Shared>) -> Res<()> {
    let r = mutate!(app, state, |s| s.master_ai_discard());
    emit_master(&app, &lock(&state));
    r
}

#[tauri::command]
fn selftest_config() -> Option<String> {
    std::env::var("STEMWISE_SELFTEST_PROJECT").ok()
}

#[tauri::command]
fn selftest_report(app: AppHandle, report: String) {
    if let Ok(out) = std::env::var("STEMWISE_SELFTEST_OUT") {
        let _ = std::fs::write(out, report);
    }
    app.exit(0);
}

/// When the process started, for the cold-start measure (NFR-3).
static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
/// Milliseconds from the process start to the first `ui_ready` (the UI drawn and listening).
static READY_MS: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

/// NFR-3: the UI reports that it is drawn; the first call fixes the cold-start time, which it returns.
#[tauri::command]
fn ui_ready() -> u64 {
    *READY_MS.get_or_init(|| STARTED.get().map_or(0, |t| t.elapsed().as_millis() as u64))
}

// ---------------------------------------------------------------------------------------------
// Guided tour (P-020)
// ---------------------------------------------------------------------------------------------

#[tauri::command]
fn tour_info(state: State<'_, Shared>) -> stemwise_engine::dto::TourInfo {
    lock(&state).tour_info()
}

#[tauri::command]
fn tour_mark_shown(state: State<'_, Shared>) -> Res<()> {
    lock(&state).tour_mark_shown()
}

/// PRJ-9: closes the open project (the interface asked about saving first), builds the demo in the app's cache and
/// opens it; the proposal of the offline demo is then prepared in the background. Runs off the main thread.
#[tauri::command(async)]
fn tour_start(app: AppHandle, state: State<'_, Shared>) -> Res<()> {
    // One demo at a time: a tour restarted while the previous demo is still being built waits for that build to be
    // thrown away, so the two never write in the same folder.
    static BUILDING: Mutex<()> = Mutex::new(());
    let _one = BUILDING.lock().unwrap_or_else(|e| e.into_inner());
    let (cache, epoch) = {
        let mut s = lock(&state);
        s.close();
        emit_ai(&app, &s);
        (s.cache.clone(), s.tour.epoch)
    };
    let (dir, project) = session::tour::build(&cache)?;
    let job = {
        let mut s = lock(&state);
        let job = s.tour_install(dir, project, epoch)?;
        emit_project(&app, &s);
        emit_ai(&app, &s);
        emit_master(&app, &s);
        job
    };
    let shared: Shared = state.inner().clone();
    std::thread::Builder::new()
        .name("stemwise-tour".into())
        .spawn(move || {
            let progress = |p: stemwise_ai::mixing::AiProgress| {
                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                if s.tour_progress(job.owner, p) {
                    emit_ai(&app, &s);
                }
            };
            let r = session::tour::propose(&job, &progress);
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            if s.tour_proposal_done(job.owner, r) {
                emit_ai(&app, &s);
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// UI-18 step 8: the proposal in review; `false` while the offline demo is still making it.
#[tauri::command]
fn tour_proposal(app: AppHandle, state: State<'_, Shared>) -> Res<bool> {
    let mut s = lock(&state);
    let r = s.tour_show_proposal();
    emit_ai(&app, &s);
    r
}

/// UI-19: the end of the tour: playback stops, the demo closes and is deleted.
#[tauri::command]
fn tour_end(app: AppHandle, state: State<'_, Shared>) {
    let mut s = lock(&state);
    s.tour_end();
    emit_ai(&app, &s);
    emit_master(&app, &s);
}

fn main() {
    STARTED.get_or_init(std::time::Instant::now);
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init()).plugin(tauri_plugin_opener::init());
    platform::with_menu_bar(builder)
        .setup(|app| {
            // `STEMWISE_DATA` moves the app's data folder (recent projects, AI register): the self-test uses it.
            let data = std::env::var_os("STEMWISE_DATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir().join("stemwise")));
            // The tour's demo lives in the app's cache (PRJ-9); with `STEMWISE_DATA`, in a folder of it.
            let cache = match std::env::var_os("STEMWISE_DATA") {
                Some(d) => PathBuf::from(d).join("cache"),
                None => app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir().join("stemwise-cache")),
            };
            let shared: Shared = Arc::new(Mutex::new(Session::new(data, cache)));
            app.manage(shared.clone());
            // Meters and position at 40 Hz on a fixed schedule (ENG-6: at least 30 Hz), autosave check (PRJ-2).
            let handle = app.handle().clone();
            std::thread::Builder::new().name("stemwise-meters".into()).spawn(move || {
                let mut tick = 0u32;
                let period = Duration::from_millis(25);
                let mut next = std::time::Instant::now();
                loop {
                    next += period;
                    let now = std::time::Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else {
                        next = now;
                    }
                    let snap: Option<MeterSnapshot> = {
                        let mut s = shared.lock().unwrap_or_else(|p| p.into_inner());
                        tick = tick.wrapping_add(1);
                        if tick.is_multiple_of(40) {
                            s.autosave(false);
                        }
                        s.meters()
                    };
                    if let Some(m) = snap {
                        let _ = handle.emit("meters", m);
                    }
                }
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            ui_ready,
            relink_media,
            recent_projects,
            create_project,
            open_project,
            recover_project,
            close_project,
            save_project,
            save_project_as,
            get_project,
            import_candidates,
            import_files,
            param_specs,
            set_param,
            begin_gesture,
            gesture_param,
            end_gesture,
            set_view,
            edit,
            gesture_edit,
            set_lane_visible,
            watch_spectrum,
            ai_status,
            ai_estimate,
            ai_start,
            mix_levels,
            ai_cancel,
            ai_apply,
            ai_discard,
            ai_reopen,
            ai_listen,
            ai_detail,
            ai_export,
            ai_set_config,
            ai_set_key,
            ai_open_page,
            ai_connect_start,
            ai_connect_cancel,
            ai_connect_reopen,
            set_spend_cap,
            set_normalize_trim,
            set_allow_structure,
            set_allow_ideas,
            ai_set_ideas,
            snapshot_save,
            snapshot_restore,
            snapshot_delete,
            add_reference,
            remove_reference,
            master_status,
            set_phase,
            master_listen,
            master_listen_same,
            master_solo,
            listen_block,
            master_chain,
            master_prepare_retry,
            master_job_cancel,
            bounce_start,
            export_start,
            master_ai_start,
            master_ai_apply,
            master_ai_discard,
            master_ai_reopen,
            mixdown_peaks,
            master_wave,
            set_cover,
            undo,
            redo,
            history_goto,
            history,
            play,
            pause,
            stop,
            locate,
            set_loop,
            transport_state,
            audio_devices,
            audio_prefs,
            set_audio_prefs,
            peaks,
            selftest_config,
            selftest_report,
            tour_info,
            tour_mark_shown,
            tour_start,
            tour_proposal,
            tour_end,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Stemwise")
        // Quitting during the tour ends it like Done: the demo is deleted (UI-19).
        .run(|app, e| {
            if let tauri::RunEvent::Exit = e
                && let Some(state) = app.try_state::<Shared>()
            {
                state.lock().unwrap_or_else(|p| p.into_inner()).tour_end();
            }
        });
}
