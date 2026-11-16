//! The project package `<Name>.stemwise/` (PRJ-1, PRJ-2, PRJ-8, HIS-4; FOUNDATION §5.2).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::command::Command;
use crate::model::{MixState, Project, SCHEMA_VERSION, SnapshotInfo};

pub const EXTENSION: &str = "stemwise";
pub const SUBDIRS: [&str; 8] = ["audio", "references", "cache", "ai", "snapshots", "mixdowns", "masters", "autosave"];
const LOCK: &str = "autosave/session.lock";
const AUTOSAVE: &str = "autosave/project.json";
/// Autosave period (PRJ-2).
pub const AUTOSAVE_SECONDS: u64 = 30;

#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("project.json is not valid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("this project was saved by a newer version of Stemwise (schema {0})")]
    TooNew(u32),
    #[error("project.json is not valid: track “{0}”: {1}")]
    Chain(String, String),
    #[error("not a Stemwise project: {0}")]
    NotAPackage(String),
    #[error("{0} already exists")]
    Exists(String),
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Creates `<parent>/<name>.stemwise/` with its folders and an empty project (PRJ-1).
pub fn create(parent: &Path, name: &str) -> Result<(PathBuf, Project), PackageError> {
    let dir = parent.join(format!("{name}.{EXTENSION}"));
    if dir.exists() {
        return Err(PackageError::Exists(dir.display().to_string()));
    }
    for s in SUBDIRS {
        fs::create_dir_all(dir.join(s))?;
    }
    let project = Project::new(name);
    save(&dir, &project)?;
    Ok((dir, project))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    // Make sure the data is on disk before the rename publishes it. The handle that wrote it flushes it: Windows
    // refuses to flush a file opened for reading.
    let mut f = fs::File::create(&tmp)?;
    io::Write::write_all(&mut f, bytes)?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, path)
}

pub fn to_json(project: &Project) -> String {
    let mut s = serde_json::to_string_pretty(project).expect("project serialises");
    s.push('\n');
    s
}

/// Saves `project.json` atomically (temporary file + rename, FOUNDATION §5.2).
pub fn save(dir: &Path, project: &Project) -> Result<(), PackageError> {
    for s in SUBDIRS {
        fs::create_dir_all(dir.join(s))?;
    }
    write_atomic(&dir.join("project.json"), to_json(project).as_bytes())?;
    // A saved project needs no recovery.
    let _ = fs::remove_file(dir.join(AUTOSAVE));
    Ok(())
}

/// Copies a package to a new location (Save as, PRJ-2). Caches are not copied: they are rebuilt.
pub fn save_as(from: &Path, to_parent: &Path, name: &str, project: &Project) -> Result<PathBuf, PackageError> {
    let dir = to_parent.join(format!("{name}.{EXTENSION}"));
    if dir.exists() {
        return Err(PackageError::Exists(dir.display().to_string()));
    }
    for s in SUBDIRS {
        fs::create_dir_all(dir.join(s))?;
        if s == "cache" || s == "autosave" {
            continue;
        }
        let src = from.join(s);
        if src.is_dir() {
            copy_dir(&src, &dir.join(s))?;
        }
    }
    let mut p = project.clone();
    p.name = name.to_string();
    save(&dir, &p)?;
    Ok(dir)
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let t = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &t)?;
        } else {
            fs::copy(e.path(), &t)?;
        }
    }
    Ok(())
}

/// Loads `project.json`, migrating older schema versions (PRJ-8).
pub fn load(dir: &Path) -> Result<Project, PackageError> {
    let path = dir.join("project.json");
    if !path.exists() {
        return Err(PackageError::NotAPackage(dir.display().to_string()));
    }
    parse(&fs::read_to_string(path)?)
}

pub fn parse(text: &str) -> Result<Project, PackageError> {
    let value: Value = serde_json::from_str(text)?;
    // A mixdown up to date with a schema-3 mix stays up to date: the migrated chain renders the same (FX-10).
    let fresh = v3::mixdown_is_current(&value);
    let mut p: Project = serde_json::from_value(migrate(value)?)?;
    // A file edited by hand does not get past the limits of a chain (FX-10, FX-11), which the engine relies on.
    for t in &p.tracks {
        t.chain.check(t.is_return()).map_err(|e| PackageError::Chain(t.name.clone(), e))?;
    }
    if fresh {
        let h = mix_hash(&p);
        if let Some(m) = p.master.mixdown.as_mut() {
            m.mix_hash = h;
        }
    }
    Ok(p)
}

/// Brings a document of any earlier schema version to the current one (PRJ-8).
///
/// Version 0 is the pre-release draft format: no `ai` block, no `snapshots`, and the tempo as a flat `bpm` field.
pub fn migrate(mut v: Value) -> Result<Value, PackageError> {
    let version = v.get("schema_version").and_then(Value::as_u64).unwrap_or(0) as u32;
    if version > SCHEMA_VERSION {
        return Err(PackageError::TooNew(version));
    }
    if version < 1 {
        let o = v.as_object_mut().ok_or_else(|| PackageError::NotAPackage("not an object".into()))?;
        let bpm = o.remove("bpm").and_then(|b| b.as_f64());
        o.insert(
            "tempo".into(),
            bpm.map_or(Value::Null, |bpm| serde_json::json!({ "bpm": bpm, "beats_per_bar": 4, "beat_unit": 4 })),
        );
        o.entry("snapshots").or_insert_with(|| Value::Array(Vec::new()));
        o.entry("ai").or_insert_with(|| serde_json::to_value(crate::model::AiState::default()).expect("serialises"));
        o.insert("schema_version".into(), Value::from(1));
    }
    if version < 2 {
        // Version 2 (P-011): three reverb algorithms instead of the comb reverb, and the compressor's mix.
        if let Some(tracks) = v.get_mut("tracks") {
            migrate_tracks(tracks);
        }
        if let Some(comp) = v.pointer_mut("/master/chain/comp").and_then(Value::as_object_mut) {
            comp.entry("mix").or_insert(Value::from(1.0));
        }
        if let Some(o) = v.as_object_mut() {
            o.insert("schema_version".into(), Value::from(2));
        }
    }
    if version < 3 {
        // Version 3 (P-017): the master chain is a list of blocks.
        if let Some(chain) = v.pointer_mut("/master/chain") {
            migrate_master_chain(chain);
        }
        if let Some(o) = v.as_object_mut() {
            o.insert("schema_version".into(), Value::from(3));
        }
    }
    if version < 4 {
        // Version 4 (P-018): the chains of tracks and groups are lists of blocks; returns get an empty one.
        if let Some(tracks) = v.get_mut("tracks") {
            migrate_tracks(tracks);
        }
        if let Some(o) = v.as_object_mut() {
            o.insert("schema_version".into(), Value::from(4));
        }
    }
    Ok(v)
}

/// The fixed chain of schema 2 (EQ, match EQ, compressor, width, saturation) becomes the list of blocks of schema 3,
/// in the same order and with the same settings, followed by the two new kinds switched off: the render is the same.
fn migrate_master_chain(chain: &mut Value) {
    use crate::model::{Block, BlockKind};
    let Some(o) = chain.as_object_mut() else { return };
    if o.contains_key("blocks") {
        return;
    }
    let mut blocks = Vec::new();
    let mut push = |kind: &str, mut params: Value| {
        if let Some(p) = params.as_object_mut() {
            p.insert("type".into(), Value::from(kind));
        }
        let id = format!("b{}", blocks.len() + 1);
        blocks.push(serde_json::json!({ "id": id, "block": params }));
    };
    let fresh = |k: BlockKind| serde_json::to_value(Block::new(k)).expect("serialises");
    let strip = |mut v: Value| {
        if let Some(p) = v.as_object_mut() {
            p.remove("type");
        }
        v
    };
    push("eq", o.remove("eq").unwrap_or_else(|| strip(fresh(BlockKind::Eq))));
    push("match_eq", o.remove("match_eq").unwrap_or_else(|| strip(fresh(BlockKind::MatchEq))));
    let mut comp = o.remove("comp").unwrap_or_else(|| strip(fresh(BlockKind::Compressor)));
    if let Some(c) = comp.as_object_mut() {
        // The master compressor never had a sidechain; its detector heard everything.
        c.remove("sidechain");
        c.entry("mix").or_insert(Value::from(1.0));
        c.insert("listen_above_hz".into(), Value::from(0.0));
    }
    push("compressor", comp);
    push("multiband", strip(fresh(BlockKind::Multiband)));
    push("width", o.remove("width").unwrap_or_else(|| strip(fresh(BlockKind::Width))));
    push("saturation", o.remove("sat").unwrap_or_else(|| strip(fresh(BlockKind::Saturation))));
    push("clipper", strip(fresh(BlockKind::Clipper)));
    o.insert("blocks".into(), Value::Array(blocks));
}

/// Brings an array of tracks written before schema version 2 to the current shape; tracks already current are
/// left alone, so it also serves snapshot files, which carry no version.
///
/// The comb reverb becomes a `room` whose decay is the one its `room_size` gave (comb feedback
/// `0.28·room_size + 0.7` on lines of about 30 ms); the render is not the same (P-011).
///
/// The fixed chain of schema 3 becomes the five classic blocks with ids `hpf`, `lpf`, `eq`, `comp`, `sat`, in the
/// same order and with the same settings: parameter ids and automation lanes are unchanged and the render is the
/// same (FX-10). A return's unused chain becomes an empty list (FX-11).
pub fn migrate_tracks(tracks: &mut Value) {
    for t in tracks.as_array_mut().into_iter().flatten() {
        let is_return = t.get("kind").and_then(Value::as_str) == Some("return");
        if let Some(chain) = t.get_mut("chain").and_then(Value::as_object_mut)
            && !chain.contains_key("blocks")
        {
            let mut blocks = Vec::new();
            for (key, ty) in
                [("hpf", "hpf"), ("lpf", "lpf"), ("eq", "eq"), ("comp", "compressor"), ("sat", "saturation")]
            {
                let Some(mut b) = chain.remove(key) else { continue };
                if let Some(o) = b.as_object_mut() {
                    o.insert("type".into(), Value::from(ty));
                    if ty == "compressor" {
                        o.entry("mix").or_insert(Value::from(1.0));
                    }
                }
                if !is_return {
                    blocks.push(serde_json::json!({ "id": key, "fx": b }));
                }
            }
            chain.insert("blocks".into(), Value::Array(blocks));
        }
        if let Some(comp) = t.pointer_mut("/chain/comp").and_then(Value::as_object_mut) {
            comp.entry("mix").or_insert(Value::from(1.0));
        }
        let Some(ret) = t.get_mut("ret").and_then(Value::as_object_mut) else { continue };
        if let Some(room) = ret.remove("room_size").and_then(|r| r.as_f64()) {
            let decay = (-0.09 / (0.28 * room.clamp(0.0, 1.0) + 0.7).log10()).clamp(0.1, 10.0);
            ret.insert("algorithm".into(), Value::from("room"));
            ret.insert("decay_s".into(), Value::from((decay * 100.0).round() / 100.0));
            ret.insert("size".into(), Value::from(room.clamp(0.0, 1.0)));
            ret.insert("modulation".into(), Value::from(0.2));
            ret.insert("lpf_hz".into(), Value::from(20000.0));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Autosave and crash recovery (PRJ-2)
// ---------------------------------------------------------------------------------------------

/// Marks the package as open by this session. Returns `true` when a previous session did not close cleanly
/// and left an autosave newer than `project.json`: the app then offers to recover it (design 13g).
pub fn open_session(dir: &Path) -> Result<bool, PackageError> {
    fs::create_dir_all(dir.join("autosave"))?;
    let crashed = dir.join(LOCK).exists() && recovery_available(dir);
    fs::write(dir.join(LOCK), std::process::id().to_string())?;
    Ok(crashed)
}

pub fn close_session(dir: &Path) {
    let _ = fs::remove_file(dir.join(LOCK));
    let _ = fs::remove_file(dir.join(AUTOSAVE));
}

fn modified(p: &Path) -> Option<SystemTime> {
    fs::metadata(p).and_then(|m| m.modified()).ok()
}

pub fn recovery_available(dir: &Path) -> bool {
    match (modified(&dir.join(AUTOSAVE)), modified(&dir.join("project.json"))) {
        (Some(a), Some(p)) => a >= p,
        (Some(_), None) => true,
        _ => false,
    }
}

/// Writes the autosave copy (every 30 s with unsaved changes, and at every AI transaction).
pub fn autosave(dir: &Path, project: &Project) -> Result<(), PackageError> {
    fs::create_dir_all(dir.join("autosave"))?;
    write_atomic(&dir.join(AUTOSAVE), to_json(project).as_bytes())?;
    Ok(())
}

pub fn load_recovery(dir: &Path) -> Result<Project, PackageError> {
    parse(&fs::read_to_string(dir.join(AUTOSAVE))?)
}

pub fn discard_recovery(dir: &Path) {
    let _ = fs::remove_file(dir.join(AUTOSAVE));
}

// ---------------------------------------------------------------------------------------------
// Snapshots (HIS-4)
// ---------------------------------------------------------------------------------------------

/// Saves the current mix as a named snapshot file and returns the command adding it to the list.
pub fn save_snapshot(dir: &Path, project: &mut Project, name: &str) -> Result<Command, PackageError> {
    let id = project.new_id("snap");
    let path = format!("snapshots/{id}.json");
    fs::create_dir_all(dir.join("snapshots"))?;
    write_atomic(&dir.join(&path), serde_json::to_string(&project.mix_state())?.as_bytes())?;
    let mut list = project.snapshots.clone();
    list.push(SnapshotInfo { id, name: name.to_string(), created_ms: now_ms(), path });
    Ok(Command::SetSnapshots { snapshots: list })
}

pub fn load_snapshot(dir: &Path, info: &SnapshotInfo) -> Result<MixState, PackageError> {
    let mut v: Value = serde_json::from_str(&fs::read_to_string(dir.join(&info.path))?)?;
    if let Some(tracks) = v.get_mut("tracks") {
        migrate_tracks(tracks);
    }
    Ok(serde_json::from_value(v)?)
}

/// Commands restoring a mix state (an undoable step: HIS-4).
pub fn restore_commands(project: &Project, state: &MixState) -> Vec<Command> {
    let mut cmds = Vec::new();
    // Remove tracks the snapshot does not have, from the end so indices stay valid on undo.
    for t in project.tracks.iter().rev() {
        if !state.tracks.iter().any(|s| s.id == t.id) {
            cmds.push(Command::RemoveTrack { track: t.id.clone() });
        }
    }
    let mut current: Vec<String> =
        project.tracks.iter().filter(|t| state.tracks.iter().any(|s| s.id == t.id)).map(|t| t.id.clone()).collect();
    for (i, s) in state.tracks.iter().enumerate() {
        match current.iter().position(|id| id == &s.id) {
            Some(pos) => {
                if project.track(&s.id) != Some(s) {
                    cmds.push(Command::SetTrack { track: Box::new(s.clone()) });
                }
                if pos != i {
                    cmds.push(Command::MoveTrack { track: s.id.clone(), index: i });
                    let id = current.remove(pos);
                    current.insert(i, id);
                }
            }
            None => {
                cmds.push(Command::InsertTrack { index: i, track: Box::new(s.clone()) });
                current.insert(i, s.id.clone());
            }
        }
    }
    if project.mix_bus != state.mix_bus {
        cmds.push(Command::SetMixBus { bus: state.mix_bus.clone() });
    }
    cmds
}

/// Hash of the mix state a render depends on (MDN-4): tracks, bus, media and rate.
pub fn mix_hash(project: &Project) -> String {
    #[derive(serde::Serialize)]
    struct Key<'a> {
        tracks: &'a [crate::model::Track],
        bus: &'a crate::model::MixBus,
        media: &'a [crate::model::MediaFile],
        rate: Option<u32>,
    }
    let mut tracks = project.tracks.clone();
    for t in &mut tracks {
        // AI lock, lane visibility, name, tint and role do not change the sound: renaming a track does not make the
        // mixdown out of date.
        t.locked = false;
        t.name.clear();
        t.color = 0;
        t.role = None;
        for l in &mut t.automation {
            l.visible = true;
        }
    }
    let mut media = project.media.clone();
    for m in &mut media {
        m.original_name.clear();
    }
    let key = Key { tracks: &tracks, bus: &project.mix_bus, media: &media, rate: project.sample_rate };
    blake3::hash(serde_json::to_string(&key).expect("serialises").as_bytes()).to_hex()[..32].to_string()
}

// ---------------------------------------------------------------------------------------------
// Recent projects (PRJ-2), kept in the app's data folder
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub opened_ms: u64,
}

pub fn recent_projects(app_data: &Path) -> Vec<RecentProject> {
    let list: Vec<RecentProject> = fs::read_to_string(app_data.join("recent.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    list.into_iter().filter(|r| Path::new(&r.path).join("project.json").exists()).collect()
}

pub fn touch_recent(app_data: &Path, dir: &Path, name: &str) -> io::Result<()> {
    fs::create_dir_all(app_data)?;
    let path = dir.display().to_string();
    let mut list = recent_projects(app_data);
    list.retain(|r| r.path != path);
    list.insert(0, RecentProject { path, name: name.to_string(), opened_ms: now_ms() });
    list.truncate(12);
    write_atomic(&app_data.join("recent.json"), serde_json::to_string_pretty(&list)?.as_bytes())
}

/// Media used by the project's clips whose audio is not in the package (UI-6, design 13d): the file is gone, or the
/// clip names a media the project does not list. Their clips play silent in the live engine; offline renders,
/// bounces and analyses refuse to run until they are located (sorted, without repeats).
pub fn missing_media(dir: &Path, project: &crate::model::Project) -> Vec<crate::model::MediaId> {
    let used: std::collections::BTreeSet<&str> =
        project.tracks.iter().flat_map(|t| t.clips.iter()).map(|c| c.media.as_str()).collect();
    used.into_iter()
        .filter(|id| project.media(id).is_none_or(|m| !dir.join(&m.path).is_file()))
        .map(str::to_string)
        .collect()
}

/// The mix hash of schema 3 (MDN-4), computed on a document of that version: the types of its tracks as they were.
mod v3 {
    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    use crate::model::{AutomationLane, Clip, MediaFile, MixBus, Output, Send, TrackKind};

    #[derive(Serialize, Deserialize)]
    struct Filter {
        bypass: bool,
        freq_hz: f64,
    }

    #[derive(Serialize, Deserialize)]
    struct Saturation {
        bypass: bool,
        drive_db: f64,
        mix: f64,
    }

    #[derive(Serialize, Deserialize)]
    struct Chain {
        trim_db: f64,
        hpf: Filter,
        lpf: Filter,
        eq: crate::model::Equalizer,
        comp: crate::model::Compressor,
        sat: Saturation,
        fader_db: f64,
        pan: f64,
    }

    #[derive(Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum ReturnFx {
        Reverb {
            bypass: bool,
            algorithm: String,
            decay_s: f64,
            size: f64,
            damping: f64,
            modulation: f64,
            predelay_ms: f64,
            hpf_hz: f64,
            lpf_hz: f64,
        },
        Delay {
            bypass: bool,
            time_ms: f64,
            sync: Option<String>,
            feedback: f64,
            feedback_hpf_hz: f64,
            feedback_lpf_hz: f64,
        },
    }

    #[derive(Serialize, Deserialize)]
    struct Track {
        id: String,
        kind: TrackKind,
        name: String,
        color: u8,
        role: Option<crate::role::Role>,
        channels: u16,
        output: Output,
        clips: Vec<Clip>,
        chain: Chain,
        ret: Option<ReturnFx>,
        sends: Vec<Send>,
        automation: Vec<AutomationLane>,
        locked: bool,
        mute: bool,
        solo: bool,
    }

    #[derive(Serialize)]
    struct Key<'a> {
        tracks: &'a [Track],
        bus: &'a MixBus,
        media: &'a [MediaFile],
        rate: Option<u32>,
    }

    /// Whether a document of schema 3 has a mixdown made from its current mix.
    pub fn mixdown_is_current(v: &Value) -> bool {
        if v.get("schema_version").and_then(Value::as_u64) != Some(3) {
            return false;
        }
        let Some(stored) = v.pointer("/master/mixdown/mix_hash").and_then(Value::as_str) else { return false };
        let parse = || -> Option<String> {
            let mut tracks: Vec<Track> = serde_json::from_value(v.get("tracks")?.clone()).ok()?;
            for t in &mut tracks {
                t.locked = false;
                t.name.clear();
                t.color = 0;
                t.role = None;
                for l in &mut t.automation {
                    l.visible = true;
                }
            }
            let bus: MixBus = serde_json::from_value(v.get("mix_bus")?.clone()).ok()?;
            let mut media: Vec<MediaFile> = serde_json::from_value(v.get("media")?.clone()).ok()?;
            for m in &mut media {
                m.original_name.clear();
            }
            let rate: Option<u32> = serde_json::from_value(v.get("sample_rate")?.clone()).ok()?;
            let key = Key { tracks: &tracks, bus: &bus, media: &media, rate };
            Some(blake3::hash(serde_json::to_string(&key).ok()?.as_bytes()).to_hex()[..32].to_string())
        };
        parse().as_deref() == Some(stored)
    }

    #[cfg(test)]
    pub fn hash(v: &Value) -> Option<String> {
        let mut w = v.clone();
        w["master"]["mixdown"] = serde_json::json!({ "mix_hash": "" });
        let mut tracks: Vec<Track> = serde_json::from_value(w.get("tracks")?.clone()).ok()?;
        for t in &mut tracks {
            t.locked = false;
            t.name.clear();
            t.color = 0;
            t.role = None;
            for l in &mut t.automation {
                l.visible = true;
            }
        }
        let bus: MixBus = serde_json::from_value(w.get("mix_bus")?.clone()).ok()?;
        let mut media: Vec<MediaFile> = serde_json::from_value(w.get("media")?.clone()).ok()?;
        for m in &mut media {
            m.original_name.clear();
        }
        let rate: Option<u32> = serde_json::from_value(w.get("sample_rate")?.clone()).ok()?;
        let key = Key { tracks: &tracks, bus: &bus, media: &media, rate };
        Some(blake3::hash(serde_json::to_string(&key).ok()?.as_bytes()).to_hex()[..32].to_string())
    }

    #[cfg(test)]
    pub fn tracks_json(v: &Value) -> Option<String> {
        let tracks: Vec<Track> = serde_json::from_value(v.get("tracks")?.clone()).ok()?;
        serde_json::to_string(&tracks).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{Document, Phase};
    use crate::model::Track;

    fn tmp() -> tempfile::TempDir {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
        fs::create_dir_all(&root).unwrap();
        tempfile::tempdir_in(root).unwrap()
    }

    /// req: PRJ-1, PRJ-2
    #[test]
    fn create_save_and_reopen_a_package() {
        let t = tmp();
        let (dir, mut p) = create(t.path(), "My Song").unwrap();
        assert!(dir.ends_with("My Song.stemwise"));
        for s in SUBDIRS {
            assert!(dir.join(s).is_dir(), "{s}");
        }
        p.tracks.push(Track::new_audio("t9".into(), "Bass", None, 1, 1));
        save(&dir, &p).unwrap();
        assert_eq!(load(&dir).unwrap(), p);
        assert!(matches!(create(t.path(), "My Song"), Err(PackageError::Exists(_))));
        // project.json holds only relative paths.
        let text = fs::read_to_string(dir.join("project.json")).unwrap();
        assert!(!text.contains(t.path().to_str().unwrap()));
        let copy = save_as(&dir, t.path(), "Copy", &p).unwrap();
        assert_eq!(load(&copy).unwrap().name, "Copy");
    }

    /// req: PRJ-2
    #[test]
    fn autosave_is_offered_for_recovery_after_a_crash_only() {
        let t = tmp();
        let (dir, mut p) = create(t.path(), "Crash").unwrap();
        assert!(!open_session(&dir).unwrap());
        p.name = "Edited".into();
        autosave(&dir, &p).unwrap();
        // The app dies here: the lock stays. Reopening offers recovery.
        assert!(open_session(&dir).unwrap());
        assert_eq!(load_recovery(&dir).unwrap().name, "Edited");
        // A clean close leaves nothing to recover.
        close_session(&dir);
        assert!(!open_session(&dir).unwrap());
        close_session(&dir);
    }

    /// The master chain as schema 2 wrote it.
    fn old_chain() -> Value {
        serde_json::json!({
            "input_gain_db": 1.0,
            "eq": {"bypass": false, "bands": [{"enabled": true, "kind": "high_shelf", "freq_hz": 10000.0, "gain_db": 1.0, "q": 0.7}]},
            "match_eq": {"bypass": true, "amount": 0.5},
            "comp": {"bypass": false, "threshold_db": -14.0, "ratio": 2.5, "attack_ms": 30.0, "release_ms": 200.0,
                     "knee_db": 6.0, "makeup_db": 0.0, "mix": 1.0, "sidechain": null},
            "width": {"bypass": true, "side_gain_db": 0.0, "mono_below_hz": 120.0},
            "sat": {"bypass": false, "drive_db": 6.0, "mix": 1.0},
            "limiter": {"bypass": false, "release_ms": 120.0},
            "dither": {"bypass": false}
        })
    }

    /// req: PRJ-8
    #[test]
    fn older_schema_versions_are_migrated_and_newer_refused() {
        let mut v = serde_json::to_value(Project::new("Old")).unwrap();
        let o = v.as_object_mut().unwrap();
        o.remove("ai");
        o.remove("snapshots");
        o.remove("tempo");
        o.insert("bpm".into(), Value::from(120.0));
        o.insert("schema_version".into(), Value::from(0));
        let p = parse(&v.to_string()).unwrap();
        assert_eq!(p.schema_version, SCHEMA_VERSION);
        assert_eq!(p.tempo.unwrap().bpm, 120.0);
        assert!(p.ai.normalize_trim);
        // Version 1: the comb reverb and compressors without a mix (P-011).
        let mut v = serde_json::to_value(Project::new("Old")).unwrap();
        v["schema_version"] = Value::from(1);
        let ret = v["tracks"][0]["ret"].as_object_mut().unwrap();
        for k in ["algorithm", "decay_s", "size", "modulation", "lpf_hz"] {
            ret.remove(k);
        }
        ret.insert("room_size".into(), Value::from(0.5));
        v["master"]["chain"] = old_chain();
        v["master"]["chain"]["comp"].as_object_mut().unwrap().remove("mix");
        let p = parse(&v.to_string()).unwrap();
        assert_eq!(p.schema_version, SCHEMA_VERSION);
        match p.tracks[0].ret.as_ref().unwrap() {
            crate::model::ReturnFx::Reverb { algorithm, decay_s, size, .. } => {
                assert_eq!(*algorithm, crate::model::ReverbAlgorithm::Room);
                // Feedback 0.84 on ≈ 30 ms: −0.09 / log10(0.84) = 1.19 s.
                assert!((decay_s - 1.19).abs() < 0.01 && *size == 0.5);
            }
            other => panic!("{other:?}"),
        }
        // Version 2: the fixed master chain becomes the list of blocks, same order and settings (P-017).
        use crate::model::{Block, BlockKind};
        let kinds: Vec<BlockKind> = p.master.chain.blocks.iter().map(|b| b.block.kind()).collect();
        let k = BlockKind::ALL;
        assert_eq!(kinds, [k[0], k[1], k[2], k[3], k[4], k[5], k[6]]);
        assert!(p.master.check().is_ok());
        match &p.master.chain.first(BlockKind::Compressor).unwrap().block {
            Block::Compressor(c) => {
                assert_eq!((c.mix, c.ratio, c.listen_above_hz, c.bypass), (1.0, 2.5, 0.0, false));
            }
            other => panic!("{other:?}"),
        }
        match &p.master.chain.first(BlockKind::Saturation).unwrap().block {
            Block::Saturation(x) => assert_eq!((x.drive_db, x.bypass), (6.0, false)),
            other => panic!("{other:?}"),
        }
        // The kinds that did not exist arrive switched off: the master sounds as before.
        assert!(p.master.chain.first(BlockKind::Multiband).unwrap().block.bypass());
        assert!(p.master.chain.first(BlockKind::Clipper).unwrap().block.bypass());
        assert_eq!(p.master.chain.limiter.release_ms, 120.0);
        let mut newer = serde_json::to_value(Project::new("New")).unwrap();
        newer["schema_version"] = Value::from(SCHEMA_VERSION + 1);
        assert!(matches!(parse(&newer.to_string()), Err(PackageError::TooNew(_))));
    }

    /// req: HIS-4
    #[test]
    fn snapshots_restore_as_one_undoable_step() {
        let t = tmp();
        let (dir, p) = create(t.path(), "Snap").unwrap();
        let mut d = Document::new(p);
        d.transact(
            Phase::Mix,
            "add",
            vec![Command::InsertTrack { index: 0, track: Box::new(Track::new_audio("t1".into(), "A", None, 1, 1)) }],
            None,
        )
        .unwrap();
        let cmd = save_snapshot(&dir, &mut d.project, "Before").unwrap();
        d.transact(Phase::Mix, "Save snapshot", vec![cmd], None).unwrap();
        let saved = d.project.clone();
        d.transact(
            Phase::Mix,
            "fader",
            vec![Command::SetParam { track: "t1".into(), param: "fader".into(), value: -6.0 }],
            None,
        )
        .unwrap();
        d.transact(
            Phase::Mix,
            "group",
            vec![Command::InsertTrack { index: 1, track: Box::new(Track::new_group("g1".into(), "G")) }],
            None,
        )
        .unwrap();
        let state = load_snapshot(&dir, &d.project.snapshots[0]).unwrap();
        let cmds = restore_commands(&d.project, &state);
        let before_restore = d.project.clone();
        d.transact(Phase::Mix, "Restore Before", cmds, None).unwrap();
        assert_eq!(d.project.tracks, saved.tracks);
        assert_ne!(mix_hash(&before_restore), mix_hash(&d.project));
        d.undo(Phase::Mix).unwrap();
        assert_eq!(d.project, before_restore);
    }

    /// req: PRJ-2
    #[test]
    fn recent_projects_list_keeps_the_last_opened_first() {
        let t = tmp();
        let data = t.path().join("appdata");
        let (a, _) = create(t.path(), "A").unwrap();
        let (b, _) = create(t.path(), "B").unwrap();
        touch_recent(&data, &a, "A").unwrap();
        touch_recent(&data, &b, "B").unwrap();
        touch_recent(&data, &a, "A").unwrap();
        let r = recent_projects(&data);
        assert_eq!(r.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["A", "B"]);
        fs::remove_dir_all(&b).unwrap();
        assert_eq!(recent_projects(&data).len(), 1);
    }

    fn schema3(name: &str) -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/schema3/{name}.json")),
        )
        .unwrap()
    }

    /// JSON text without the whitespace between tokens, keys in their written order.
    fn minify(s: &str) -> String {
        let mut out = String::new();
        let (mut in_str, mut esc) = (false, false);
        for c in s.chars() {
            if in_str {
                out.push(c);
                if esc {
                    esc = false;
                } else if c == '\\' {
                    esc = true;
                } else if c == '"' {
                    in_str = false;
                }
            } else if c == '"' {
                in_str = true;
                out.push(c);
            } else if !c.is_whitespace() {
                out.push(c);
            }
        }
        out
    }

    /// req: FX-10, PRJ-8, MDN-4
    #[test]
    fn schema3_chains_become_the_five_classic_blocks_and_a_current_mixdown_stays_current() {
        use crate::fx::Fx;
        let text = schema3("rich");
        let p = parse(&text).unwrap();
        assert_eq!(p.schema_version, SCHEMA_VERSION);
        for t in &p.tracks {
            let ids: Vec<&str> = t.chain.blocks.iter().map(|b| b.id.as_str()).collect();
            if t.is_return() {
                assert!(ids.is_empty(), "{}", t.name);
            } else {
                assert_eq!(ids, ["hpf", "lpf", "eq", "comp", "sat"], "{}", t.name);
                assert!(t.chain.check(false).is_ok());
            }
        }
        let vox = p.tracks.iter().find(|t| t.name == "Vox").unwrap();
        assert!(matches!(&vox.chain.blocks[0].fx, Fx::Hpf(f) if !f.bypass && f.freq_hz == 120.0));
        assert!(
            vox.lane("eq.2.gain").is_some() && crate::params::get(vox, &"eq.2.gain".parse().unwrap()) == Some(-3.0)
        );
        // The schema-3 mirror serialises tracks exactly as schema 3 did, so its mix hash is the one stored then.
        let v: Value = serde_json::from_str(&text).unwrap();
        let tracks: Vec<serde_json::Value> = v["tracks"].as_array().unwrap().clone();
        let start = text.find("\"tracks\": [").unwrap() + "\"tracks\": ".len();
        let mut depth = 0;
        let mut end = start;
        for (i, c) in text[start..].char_indices() {
            match c {
                '[' | '{' => depth += 1,
                ']' | '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(tracks.len(), p.tracks.len());
        let mirrored = v3::tracks_json(&v).unwrap();
        assert_eq!(mirrored, minify(&text[start..end]), "the mirror writes what schema 3 wrote");
        // A stored hash equal to the schema-3 one is carried over to the schema-4 mix.
        let mut v2 = v.clone();
        v2["master"]["mixdown"] = serde_json::json!({
            "number": 1, "path": "mixdowns/mixdown-1.wav", "mix_hash": v3::hash(&v).unwrap(), "trim_db": 0.0,
            "frames": 10, "created_ms": 0, "lufs": null, "true_peak_db": -3.0, "lra": null });
        let q = parse(&v2.to_string()).unwrap();
        assert_eq!(q.master.mixdown.clone().unwrap().mix_hash, mix_hash(&q));
        v2["master"]["mixdown"]["mix_hash"] = Value::from("stale");
        let q = parse(&v2.to_string()).unwrap();
        assert_eq!(q.master.mixdown.unwrap().mix_hash, "stale");
    }
}
