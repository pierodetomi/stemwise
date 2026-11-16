//! Editing operations built from primitive commands (CLP-1 … CLP-4, SEC-1, TRK-1, TRK-3, TRK-5).
//!
//! Each function reads the project and returns the commands of one history step; the caller applies them with
//! `Document::transact` (or inside a gesture). Nothing here mutates the project directly.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::command::Command;
use crate::model::{
    Author, AutomationPoint, Clip, EqKind, Fade, FadeShape, MAX_RETURNS, MixBus, Output, Project, ReturnFx,
    ReverbAlgorithm, Section, Send, Sidechain, Tempo, Track, TrackKind,
};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EditError {
    #[error("track `{0}` does not exist")]
    NoTrack(String),
    #[error("clip `{0}` does not exist")]
    NoClip(String),
    #[error("clips can only move between tracks with the same channel count")]
    ChannelMismatch,
    #[error("only audio tracks hold clips")]
    NotAudio,
    #[error("a project holds at most {MAX_RETURNS} returns")]
    TooManyReturns,
    #[error("{0}")]
    Invalid(String),
}

/// A clip reference: (track id, clip id).
pub type ClipRef = (String, String);

fn find_clip<'a>(p: &'a Project, r: &ClipRef) -> Result<(&'a Track, &'a Clip), EditError> {
    let t = p.track(&r.0).ok_or_else(|| EditError::NoTrack(r.0.clone()))?;
    let c = t.clip(&r.1).ok_or_else(|| EditError::NoClip(r.1.clone()))?;
    Ok((t, c))
}

fn clamp_fades(c: &mut Clip) {
    c.fade_in.length = c.fade_in.length.min(c.length);
    c.fade_out.length = c.fade_out.length.min(c.length - c.fade_in.length);
}

/// Places `new` clips on a track's clip list; clips underneath are trimmed, split or removed (CLP-4).
pub fn place_clips(existing: &[Clip], new: &[Clip], next_id: &mut dyn FnMut() -> String) -> Vec<Clip> {
    let mut out: Vec<Clip> = existing.to_vec();
    for n in new {
        let (a, b) = (n.start, n.end());
        let mut next = Vec::with_capacity(out.len() + 1);
        for c in out.drain(..) {
            let (s, e) = (c.start, c.end());
            if e <= a || s >= b {
                next.push(c);
            } else if s >= a && e <= b {
                // Fully covered: removed.
            } else if s < a && e > b {
                // The new clip sits inside: split into a head and a tail.
                let mut head = c.clone();
                head.length = (a - s) as u64;
                head.fade_out = Fade::default();
                clamp_fades(&mut head);
                let mut tail = c.clone();
                tail.id = next_id();
                tail.offset += (b - s) as u64;
                tail.start = b;
                tail.length = (e - b) as u64;
                tail.fade_in = Fade::default();
                clamp_fades(&mut tail);
                next.push(head);
                next.push(tail);
            } else if s < a {
                let mut head = c.clone();
                head.length = (a - s) as u64;
                clamp_fades(&mut head);
                next.push(head);
            } else {
                let mut tail = c.clone();
                tail.offset += (b - s) as u64;
                tail.start = b;
                tail.length = (e - b) as u64;
                clamp_fades(&mut tail);
                next.push(tail);
            }
        }
        next.push(n.clone());
        out = next;
    }
    out.sort_by_key(|c| c.start);
    out
}

/// Replaces the clips of the given tracks, one `SetTrack` per changed track.
fn set_clips(p: &Project, changes: Vec<(String, Vec<Clip>)>) -> Vec<Command> {
    changes
        .into_iter()
        .filter_map(|(id, clips)| {
            let mut t = p.track(&id)?.clone();
            if t.clips == clips {
                return None;
            }
            t.clips = clips;
            Some(Command::SetTrack { track: Box::new(t) })
        })
        .collect()
}

/// Moves clips by `delta` frames and optionally onto another track (CLP-1), trimming what lies underneath
/// (CLP-4). With `duplicate`, the originals stay in place (⌥-drag).
pub fn move_clips(
    p: &mut Project,
    clips: &[ClipRef],
    delta: i64,
    to_track: Option<&str>,
    duplicate: bool,
) -> Result<Vec<Command>, EditError> {
    let mut moving: Vec<(String, Clip)> = Vec::new();
    for r in clips {
        let (t, c) = find_clip(p, r)?;
        if let Some(dest) = to_track {
            let d = p.track(dest).ok_or_else(|| EditError::NoTrack(dest.into()))?;
            if !d.is_audio() {
                return Err(EditError::NotAudio);
            }
            if d.channels != t.channels {
                return Err(EditError::ChannelMismatch);
            }
        }
        let mut moved = c.clone();
        moved.start = (moved.start + delta).max(0);
        moving.push((to_track.unwrap_or(&r.0).to_string(), moved));
    }
    let mut tracks: Vec<String> = clips.iter().map(|r| r.0.clone()).collect();
    if let Some(d) = to_track {
        tracks.push(d.into());
    }
    tracks.sort();
    tracks.dedup();
    let mut changes = Vec::new();
    let mut ids = Vec::new();
    for (dest, c) in &mut moving {
        if duplicate {
            c.id = p.new_id("c");
        }
        ids.push((dest.clone(), c.clone()));
    }
    for id in &tracks {
        let t = p.track(id).expect("checked");
        let mut base: Vec<Clip> = if duplicate {
            t.clips.clone()
        } else {
            t.clips.iter().filter(|c| !clips.iter().any(|r| &r.0 == id && r.1 == c.id)).cloned().collect()
        };
        let incoming: Vec<Clip> = ids.iter().filter(|(d, _)| d == id).map(|(_, c)| c.clone()).collect();
        let mut counter = p.next_id;
        let placed = place_clips(&base, &incoming, &mut || {
            let s = format!("c{counter}");
            counter += 1;
            s
        });
        p.next_id = counter;
        base = placed;
        changes.push((id.clone(), base));
    }
    Ok(set_clips(p, changes))
}

/// Splits the clips crossing `frame` on the given tracks (all audio tracks if empty) — CLP-1.
pub fn split_at(p: &mut Project, tracks: &[String], frame: i64) -> Vec<Command> {
    let ids: Vec<String> = if tracks.is_empty() {
        p.tracks.iter().filter(|t| t.is_audio()).map(|t| t.id.clone()).collect()
    } else {
        tracks.to_vec()
    };
    let mut changes = Vec::new();
    for id in ids {
        let Some(t) = p.track(&id) else { continue };
        let mut clips = Vec::new();
        let mut changed = false;
        let originals = t.clips.clone();
        for c in originals {
            if c.start < frame && c.end() > frame {
                let mut head = c.clone();
                head.length = (frame - c.start) as u64;
                head.fade_out = Fade::default();
                clamp_fades(&mut head);
                let mut tail = c.clone();
                tail.id = p.new_id("c");
                tail.offset += head.length;
                tail.start = frame;
                tail.length = c.length - head.length;
                tail.fade_in = Fade::default();
                clamp_fades(&mut tail);
                clips.push(head);
                clips.push(tail);
                changed = true;
            } else {
                clips.push(c);
            }
        }
        if changed {
            changes.push((id, clips));
        }
    }
    set_clips(p, changes)
}

/// Trims a clip edge to a new timeline position (CLP-1), within the media file.
pub fn trim_clip(p: &Project, r: &ClipRef, left: bool, to: i64) -> Result<Vec<Command>, EditError> {
    let (t, c) = find_clip(p, r)?;
    let media_frames = p.media(&c.media).map_or(c.offset + c.length, |m| m.frames);
    let mut n = c.clone();
    if left {
        let earliest = c.start - c.offset as i64;
        let to = to.clamp(earliest.max(0), c.end() - 1);
        n.offset = (c.offset as i64 + (to - c.start)) as u64;
        n.length = (c.end() - to) as u64;
        n.start = to;
    } else {
        let latest = c.start + (media_frames - c.offset) as i64;
        let to = to.clamp(c.start + 1, latest);
        n.length = (to - c.start) as u64;
    }
    clamp_fades(&mut n);
    let others: Vec<Clip> = t.clips.iter().filter(|x| x.id != c.id).cloned().collect();
    let mut counter = 0u64;
    let placed = place_clips(&others, &[n], &mut || {
        counter += 1;
        format!("{}-{counter}", c.id)
    });
    Ok(set_clips(p, vec![(t.id.clone(), placed)]))
}

/// Duplicates clips right after the end of the selection (CLP-1).
pub fn duplicate_clips(p: &mut Project, clips: &[ClipRef]) -> Result<Vec<Command>, EditError> {
    let mut lo = i64::MAX;
    let mut hi = i64::MIN;
    for r in clips {
        let (_, c) = find_clip(p, r)?;
        lo = lo.min(c.start);
        hi = hi.max(c.end());
    }
    if clips.is_empty() {
        return Ok(Vec::new());
    }
    let mut cmds = Vec::new();
    let mut by_track: Vec<String> = clips.iter().map(|r| r.0.clone()).collect();
    by_track.sort();
    by_track.dedup();
    for tid in by_track {
        let refs: Vec<ClipRef> = clips.iter().filter(|r| r.0 == tid).cloned().collect();
        let mut q = p.clone();
        for c in move_clips(&mut q, &refs, hi - lo, None, true)? {
            c.clone().apply(&mut q).map_err(|e| EditError::Invalid(e.to_string()))?;
            cmds.push(c);
        }
        p.next_id = q.next_id;
    }
    Ok(cmds)
}

/// Deletes clips (CLP-1).
pub fn delete_clips(p: &Project, clips: &[ClipRef]) -> Result<Vec<Command>, EditError> {
    for r in clips {
        find_clip(p, r)?;
    }
    let mut changes = Vec::new();
    for t in &p.tracks {
        if clips.iter().any(|r| r.0 == t.id) {
            let kept = t.clips.iter().filter(|c| !clips.iter().any(|r| r.0 == t.id && r.1 == c.id)).cloned();
            changes.push((t.id.clone(), kept.collect()));
        }
    }
    Ok(set_clips(p, changes))
}

/// Changes clip properties: mute, gain (−24…+24 dB), fades (CLP-1, CLP-2).
pub fn update_clip(p: &Project, r: &ClipRef, f: impl FnOnce(&mut Clip)) -> Result<Vec<Command>, EditError> {
    let (t, c) = find_clip(p, r)?;
    let mut n = c.clone();
    f(&mut n);
    if !(-24.0..=24.0).contains(&n.gain_db) {
        return Err(EditError::Invalid("clip gain must be within −24 … +24 dB".into()));
    }
    if n.length != c.length || n.start != c.start || n.offset != c.offset || n.media != c.media {
        return Err(EditError::Invalid("use move or trim to change the clip position".into()));
    }
    clamp_fades(&mut n);
    let clips = t.clips.iter().map(|x| if x.id == c.id { n.clone() } else { x.clone() }).collect();
    Ok(set_clips(p, vec![(t.id.clone(), clips)]))
}

pub fn set_fade(
    p: &Project,
    r: &ClipRef,
    fade_in: bool,
    length: u64,
    shape: FadeShape,
) -> Result<Vec<Command>, EditError> {
    update_clip(p, r, |c| {
        let f = Fade { length, shape };
        if fade_in {
            c.fade_in = f;
        } else {
            c.fade_out = f;
        }
    })
}

// ---------------------------------------------------------------------------------------------
// Tracks, groups, returns (TRK-1, TRK-3, TRK-5)
// ---------------------------------------------------------------------------------------------

/// A new group, inserted before `before` (or at the end), optionally taking `members` as inputs.
pub fn add_group(p: &mut Project, name: &str, members: &[String]) -> Result<Vec<Command>, EditError> {
    let id = p.new_id("g");
    let mut g = Track::new_group(id.clone(), name);
    // A group created around tracks keeps them at the same output they had.
    if let Some(first) = members.first().and_then(|m| p.track(m)) {
        g.output = first.output.clone();
    }
    if members.iter().any(|m| p.track(m).is_some_and(|t| t.is_return())) {
        return Err(EditError::Invalid("a return cannot go into a group".into()));
    }
    let index = members.iter().filter_map(|m| p.track_index(m)).min().unwrap_or(p.tracks.len());
    let mut cmds = vec![Command::InsertTrack { index, track: Box::new(g) }];
    for m in members {
        let mut t = p.track(m).ok_or_else(|| EditError::NoTrack(m.clone()))?.clone();
        t.output = Output::Group(id.clone());
        cmds.push(Command::SetTrack { track: Box::new(t) });
    }
    Ok(cmds)
}

/// Moves tracks (dragged in the timeline) to a new place in one step: before `before` (or at the end of the list),
/// routed to `output`. The tracks keep their relative order; a group brings its members along, since they follow
/// its routing. Returns stay where they are: they are not routed into groups.
pub fn place_tracks(
    p: &Project,
    tracks: &[String],
    before: Option<&str>,
    output: &Output,
) -> Result<Vec<Command>, EditError> {
    let mut moving: Vec<String> = Vec::new();
    for t in &p.tracks {
        if tracks.contains(&t.id) {
            if t.is_return() {
                return Err(EditError::Invalid("a return cannot be moved into a group".into()));
            }
            moving.push(t.id.clone());
        }
    }
    if let Some(missing) = tracks.iter().find(|id| p.track(id).is_none()) {
        return Err(EditError::NoTrack(missing.clone()));
    }
    if let Output::Group(g) = output {
        // Into one of the moved tracks, or under one of them: the routing would loop.
        let mut o = Output::Group(g.clone());
        for _ in 0..=p.tracks.len() {
            let Output::Group(id) = o else { break };
            if moving.contains(&id) {
                return Err(EditError::Invalid("a group cannot go inside itself".into()));
            }
            o = p.track(&id).ok_or_else(|| EditError::NoTrack(id.clone()))?.output.clone();
        }
    }
    // The target order: the others as they are, the moved ones inserted before `before`.
    let mut order: Vec<String> = p.tracks.iter().map(|t| t.id.clone()).filter(|id| !moving.contains(id)).collect();
    let at = before.and_then(|b| order.iter().position(|id| id == b)).unwrap_or(order.len());
    order.splice(at..at, moving.iter().cloned());
    // MoveTrack commands that turn the current order into the target, then the new routing.
    let mut cur: Vec<String> = p.tracks.iter().map(|t| t.id.clone()).collect();
    let mut cmds = Vec::new();
    for (i, id) in order.iter().enumerate() {
        if cur[i] != *id {
            let from = cur.iter().position(|x| x == id).expect("same tracks");
            let t = cur.remove(from);
            cur.insert(i, t);
            cmds.push(Command::MoveTrack { track: id.clone(), index: i });
        }
    }
    for id in &moving {
        let t = p.track(id).expect("checked");
        if t.output != *output {
            let mut n = t.clone();
            n.output = output.clone();
            cmds.push(Command::SetTrack { track: Box::new(n) });
        }
    }
    Ok(cmds)
}

/// A new return of the given kind at the end of the track list (TRK-1).
pub fn add_return(p: &mut Project, fx: ReturnFx) -> Result<Vec<Command>, EditError> {
    if p.tracks.iter().filter(|t| t.is_return()).count() >= MAX_RETURNS {
        return Err(EditError::TooManyReturns);
    }
    let n = p.tracks.iter().filter(|t| t.ret.as_ref().is_some_and(|r| r.is_reverb() == fx.is_reverb())).count() + 1;
    let name = if fx.is_reverb() { format!("Reverb {n}") } else { format!("Delay {n}") };
    let id = p.new_id("r");
    Ok(vec![Command::InsertTrack { index: p.tracks.len(), track: Box::new(Track::new_return(id, &name, fx)) }])
}

/// Deletes a track. Everything pointing at it is updated in the same step: tracks routed into a deleted group
/// go to the group's output, sends to a deleted return disappear, sidechains from it are cleared.
pub fn delete_track(p: &Project, id: &str) -> Result<Vec<Command>, EditError> {
    let victim = p.track(id).ok_or_else(|| EditError::NoTrack(id.into()))?;
    let mut cmds = Vec::new();
    for t in &p.tracks {
        if t.id == id {
            continue;
        }
        let mut n = t.clone();
        if n.output == Output::Group(id.into()) {
            n.output = victim.output.clone();
        }
        n.sends.retain(|s| s.target != id);
        for b in &mut n.chain.blocks {
            b.fx.drop_sidechain(id);
        }
        n.automation.retain(|l| l.param != format!("send.{id}"));
        if &n != t {
            cmds.push(Command::SetTrack { track: Box::new(n) });
        }
    }
    cmds.push(Command::RemoveTrack { track: id.into() });
    Ok(cmds)
}

/// Duplicates a track right below it (TRK-1); clips get new ids, the copy keeps routing and sends.
pub fn duplicate_track(p: &mut Project, id: &str) -> Result<Vec<Command>, EditError> {
    let t = p.track(id).ok_or_else(|| EditError::NoTrack(id.into()))?.clone();
    let index = p.track_index(id).expect("found") + 1;
    let prefix = match t.kind {
        TrackKind::Audio => "t",
        TrackKind::Group => "g",
        TrackKind::Return => "r",
    };
    if t.is_return() && p.tracks.iter().filter(|x| x.is_return()).count() >= MAX_RETURNS {
        return Err(EditError::TooManyReturns);
    }
    let mut n = t.clone();
    n.id = p.new_id(prefix);
    n.name = format!("{} copy", t.name);
    n.locked = false;
    for c in &mut n.clips {
        c.id = p.new_id("c");
    }
    Ok(vec![Command::InsertTrack { index, track: Box::new(n) }])
}

pub fn set_output(p: &Project, id: &str, output: Output) -> Result<Vec<Command>, EditError> {
    let mut t = p.track(id).ok_or_else(|| EditError::NoTrack(id.into()))?.clone();
    t.output = output;
    Ok(vec![Command::SetTrack { track: Box::new(t) }])
}

/// Adds, changes (`Some(level)`) or removes (`None`) a post-fader send (TRK-5).
pub fn set_send(p: &Project, id: &str, target: &str, level_db: Option<f64>) -> Result<Vec<Command>, EditError> {
    let mut t = p.track(id).ok_or_else(|| EditError::NoTrack(id.into()))?.clone();
    match level_db {
        Some(level_db) => match t.sends.iter_mut().find(|s| s.target == target) {
            Some(s) => s.level_db = level_db,
            None => t.sends.push(Send { target: target.into(), level_db }),
        },
        None => {
            t.sends.retain(|s| s.target != target);
            t.automation.retain(|l| l.param != format!("send.{target}"));
        }
    }
    Ok(vec![Command::SetTrack { track: Box::new(t) }])
}

/// A structural change to a chain (FX-10): automation lanes of a removed block go with it, a duplicate gets none.
pub fn chain_edit(p: &Project, id: &str, op: &crate::fx::FxOp) -> Result<Vec<Command>, EditError> {
    track_cmd(p, id, |t| {
        if !t.is_audio() && !t.is_group() && !t.is_return() {
            return Err(EditError::Invalid("this track has no chain".into()));
        }
        let (chain, _) = t.chain.edit(op, t.is_return()).map_err(EditError::Invalid)?;
        let gone: Vec<String> =
            t.chain.blocks.iter().filter(|b| chain.block(&b.id).is_none()).map(|b| b.id.clone()).collect();
        let reset = match op {
            crate::fx::FxOp::Reset { id } | crate::fx::FxOp::Bands { id, .. } => Some(id.clone()),
            _ => None,
        };
        t.chain = chain;
        let probe = t.clone();
        t.automation.retain(|l| {
            let block = l.param.strip_prefix("bypass.").unwrap_or(&l.param);
            let block = block.split('.').next().unwrap_or("");
            if gone.iter().any(|g| g == block) {
                return false;
            }
            // A reset or a change of bands keeps the lanes the block still has.
            reset.as_deref() != Some(block)
                || l.param
                    .parse::<crate::params::ParamId>()
                    .ok()
                    .and_then(|pid| crate::params::get(&probe, &pid))
                    .is_some()
        });
        Ok(())
    })
}

/// Sets the sidechain source of a block (`None`: the first compressor; for a dynamic EQ, of its band `band`).
pub fn set_sidechain(
    p: &Project,
    id: &str,
    block: Option<&str>,
    band: Option<usize>,
    source: Option<&str>,
) -> Result<Vec<Command>, EditError> {
    use crate::fx::{Fx, FxKind};
    let mut t = p.track(id).ok_or_else(|| EditError::NoTrack(id.into()))?.clone();
    let blk = match block {
        Some(b) => t.chain.block_mut(b),
        None => t.chain.first_mut(FxKind::Compressor),
    }
    .ok_or_else(|| EditError::Invalid("no such block".into()))?;
    match &mut blk.fx {
        Fx::Compressor(c) => {
            let hpf_hz = c.sidechain.as_ref().map_or(100.0, |s| s.hpf_hz);
            c.sidechain = source.map(|s| Sidechain { source: s.into(), hpf_hz });
        }
        Fx::Gate(g) => g.sidechain = source.map(str::to_string),
        Fx::DynamicEq(d) => {
            let b = d.bands.get_mut(band.unwrap_or(0)).ok_or_else(|| EditError::Invalid("no such band".into()))?;
            b.sidechain = source.map(str::to_string);
        }
        _ => return Err(EditError::Invalid("this block has no sidechain".into())),
    }
    Ok(vec![Command::SetTrack { track: Box::new(t) }])
}

pub fn rename_track(p: &Project, id: &str, name: &str, color: Option<u8>) -> Result<Vec<Command>, EditError> {
    let mut t = p.track(id).ok_or_else(|| EditError::NoTrack(id.into()))?.clone();
    t.name = name.to_string();
    if let Some(c) = color {
        if c > 12 {
            return Err(EditError::Invalid("track colours are 0 … 12".into()));
        }
        t.color = c;
    }
    Ok(vec![Command::SetTrack { track: Box::new(t) }])
}

// ---------------------------------------------------------------------------------------------
// Sections (SEC-1)
// ---------------------------------------------------------------------------------------------

pub fn add_section(p: &mut Project, name: &str, start: i64, end: i64) -> Result<Vec<Command>, EditError> {
    if end <= start {
        return Err(EditError::Invalid("a section needs a length".into()));
    }
    let id = p.new_id("s");
    let color = (p.sections.len() % 12) as u8 + 1;
    let mut sections = p.sections.clone();
    sections.push(Section { id, name: name.into(), color, start, end });
    sections.sort_by_key(|s| s.start);
    Ok(vec![Command::SetSections { sections }])
}

/// Moves the boundary at the end of `id`: the next section, if adjacent, moves with it (design: "a boundary
/// drag moves both neighbours").
pub fn move_boundary(p: &Project, id: &str, to: i64) -> Result<Vec<Command>, EditError> {
    let mut sections = p.sections.clone();
    sections.sort_by_key(|s| s.start);
    let i = sections.iter().position(|s| s.id == id).ok_or_else(|| EditError::Invalid(format!("no section {id}")))?;
    let old_end = sections[i].end;
    let to = to.max(sections[i].start + 1);
    if let Some(next) = sections.get_mut(i + 1) {
        if next.start == old_end {
            if to >= next.end {
                return Err(EditError::Invalid("the boundary would cross the next section".into()));
            }
            next.start = to;
        } else if to > next.start {
            return Err(EditError::Invalid("sections cannot overlap".into()));
        }
    }
    sections[i].end = to;
    Ok(vec![Command::SetSections { sections }])
}

pub fn rename_section(p: &Project, id: &str, name: &str) -> Result<Vec<Command>, EditError> {
    let mut sections = p.sections.clone();
    let s = sections.iter_mut().find(|s| s.id == id).ok_or_else(|| EditError::Invalid(format!("no section {id}")))?;
    s.name = name.into();
    Ok(vec![Command::SetSections { sections }])
}

pub fn delete_section(p: &Project, id: &str) -> Vec<Command> {
    let sections = p.sections.iter().filter(|s| s.id != id).cloned().collect();
    vec![Command::SetSections { sections }]
}

// ---------------------------------------------------------------------------------------------
// Edits as data: what the UI asks for (FOUNDATION §5.6)
// ---------------------------------------------------------------------------------------------

/// A user edit, as the UI sends it. [`run`] turns it into the commands of one history step, so the UI holds no
/// domain logic: trimming under a moved clip, routing checks, lane clamping all happen here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Edit {
    MoveClips {
        clips: Vec<ClipRef>,
        delta: i64,
        to_track: Option<String>,
        duplicate: bool,
    },
    SplitAt {
        tracks: Vec<String>,
        frame: i64,
    },
    TrimClip {
        clip: ClipRef,
        left: bool,
        to: i64,
    },
    DuplicateClips {
        clips: Vec<ClipRef>,
    },
    DeleteClips {
        clips: Vec<ClipRef>,
    },
    ClipGain {
        clip: ClipRef,
        gain_db: f64,
    },
    ClipMute {
        clips: Vec<ClipRef>,
        mute: bool,
    },
    Fade {
        clip: ClipRef,
        fade_in: bool,
        length: u64,
        shape: FadeShape,
    },
    AddGroup {
        name: String,
        members: Vec<String>,
    },
    AddReturn {
        reverb: bool,
        /// Algorithm of a new reverb; the default one if absent.
        #[serde(default)]
        #[ts(optional)]
        algorithm: Option<ReverbAlgorithm>,
    },
    DeleteTrack {
        track: String,
    },
    DuplicateTrack {
        track: String,
    },
    MoveTrack {
        track: String,
        index: usize,
    },
    /// Tracks dragged to a new place: before `before` (the end if none), routed to `output` (TRK-1, TRK-3).
    PlaceTracks {
        tracks: Vec<String>,
        before: Option<String>,
        output: Output,
    },
    RenameTrack {
        track: String,
        name: String,
        color: Option<u8>,
    },
    SetLocked {
        track: String,
        locked: bool,
    },
    SetOutput {
        track: String,
        output: Output,
    },
    SetSend {
        track: String,
        target: String,
        level_db: Option<f64>,
    },
    /// The sidechain of a block: the first compressor if `block` is absent; a band of a dynamic EQ (FX-15).
    SetSidechain {
        track: String,
        source: Option<String>,
        #[serde(default)]
        #[ts(optional)]
        block: Option<String>,
        #[serde(default)]
        #[ts(optional)]
        band: Option<usize>,
    },
    /// A band of an EQ (the first one if `block` is absent) or of a dynamic EQ.
    SetEqBand {
        track: String,
        band: usize,
        enabled: bool,
        kind: EqKind,
        #[serde(default)]
        #[ts(optional)]
        block: Option<String>,
    },
    /// A change to the structure of a chain (FX-10, FX-11): add, remove, move, duplicate, reset, bands.
    Chain {
        track: String,
        change: crate::fx::FxOp,
    },
    /// A block's settings that are not continuous parameters (modes, character, voices, enabled bands, musical
    /// rate): the whole block, same id and kind. Continuous values go through `SetParam`.
    SetBlock {
        track: String,
        block: crate::fx::FxBlock,
    },
    SetDelaySync {
        track: String,
        sync: Option<String>,
    },
    /// The settings of a return's processor that are not continuous (FX-22 interval, FX-23 mode and character): the
    /// whole processor, same type. Continuous values go through `SetParam`.
    SetReturn {
        track: String,
        ret: ReturnFx,
    },
    /// Changes the algorithm of a reverb return (FX-5); its parameters stay as they are.
    SetReverbAlgorithm {
        track: String,
        algorithm: ReverbAlgorithm,
    },
    AddSection {
        name: String,
        start: i64,
        end: i64,
    },
    MoveBoundary {
        section: String,
        to: i64,
    },
    MoveSectionStart {
        section: String,
        to: i64,
    },
    RenameSection {
        section: String,
        name: String,
    },
    DeleteSection {
        section: String,
    },
    /// Replaces a lane's points (creates the lane); the lane becomes the user's (AUT-2).
    SetLane {
        track: String,
        param: String,
        points: Vec<AutomationPoint>,
    },
    RemoveLane {
        track: String,
        param: String,
    },
    /// AUT-2 freehand: the stroke replaces the lane over its span, simplified into points (tolerance in value units).
    DrawLane {
        track: String,
        param: String,
        stroke: Vec<AutomationPoint>,
        tolerance: f64,
    },
    /// AUT-2 paste: copied points (frames relative to the copy start) placed at `at`, replacing that span.
    PasteLane {
        track: String,
        param: String,
        at: i64,
        points: Vec<AutomationPoint>,
    },
    /// AUT-2 range delete: removes the points inside `[from, to]`.
    DeleteLaneRange {
        track: String,
        param: String,
        from: i64,
        to: i64,
    },
    SetRole {
        track: String,
        role: Option<crate::role::Role>,
    },
    /// AUT-3: shifts a whole curve by `delta` (a control moved while stopped).
    OffsetLane {
        track: String,
        param: String,
        delta: f64,
    },
    SetTempo {
        tempo: Option<Tempo>,
    },
    SetMixBus {
        fader_db: f64,
    },
    /// The project's name as shown (the package folder keeps its own name).
    RenameProject {
        name: String,
    },
    SetBrief {
        brief: crate::model::Brief,
    },
    /// The whole master state (master phase, its own history: HIS-5).
    SetMaster {
        master: Box<crate::model::MasterState>,
    },
}

impl Edit {
    /// Whether the edit belongs to the master phase's history (HIS-5); every edit here is a mix edit.
    pub fn phase(&self) -> crate::history::Phase {
        match self {
            Edit::SetMaster { .. } => crate::history::Phase::Master,
            _ => crate::history::Phase::Mix,
        }
    }
}

fn track_cmd(
    p: &Project,
    id: &str,
    f: impl FnOnce(&mut Track) -> Result<(), EditError>,
) -> Result<Vec<Command>, EditError> {
    let mut t = p.track(id).ok_or_else(|| EditError::NoTrack(id.into()))?.clone();
    f(&mut t)?;
    Ok(vec![Command::SetTrack { track: Box::new(t) }])
}

/// The commands of one history step for an edit. Ids for new objects are taken from the project's counter.
pub fn run(p: &mut Project, e: &Edit) -> Result<Vec<Command>, EditError> {
    use crate::automation;
    match e {
        Edit::MoveClips { clips, delta, to_track, duplicate } => {
            move_clips(p, clips, *delta, to_track.as_deref(), *duplicate)
        }
        Edit::SplitAt { tracks, frame } => Ok(split_at(p, tracks, *frame)),
        Edit::TrimClip { clip, left, to } => trim_clip(p, clip, *left, *to),
        Edit::DuplicateClips { clips } => duplicate_clips(p, clips),
        Edit::DeleteClips { clips } => delete_clips(p, clips),
        Edit::ClipGain { clip, gain_db } => update_clip(p, clip, |c| c.gain_db = *gain_db),
        Edit::ClipMute { clips, mute } => {
            let mut q = p.clone();
            let mut cmds = Vec::new();
            for r in clips {
                for c in update_clip(&q, r, |c| c.mute = *mute)? {
                    c.clone().apply(&mut q).map_err(|e| EditError::Invalid(e.to_string()))?;
                    cmds.push(c);
                }
            }
            Ok(cmds)
        }
        Edit::Fade { clip, fade_in, length, shape } => set_fade(p, clip, *fade_in, *length, *shape),
        Edit::AddGroup { name, members } => add_group(p, name, members),
        Edit::AddReturn { reverb, algorithm } => add_return(
            p,
            match (*reverb, *algorithm) {
                (true, Some(a)) => ReturnFx::reverb(a),
                (true, None) => ReturnFx::default_reverb(),
                (false, _) => ReturnFx::default_delay(),
            },
        ),
        Edit::DeleteTrack { track } => delete_track(p, track),
        Edit::DuplicateTrack { track } => duplicate_track(p, track),
        Edit::MoveTrack { track, index } => {
            p.track(track).ok_or_else(|| EditError::NoTrack(track.clone()))?;
            Ok(vec![Command::MoveTrack { track: track.clone(), index: (*index).min(p.tracks.len() - 1) }])
        }
        Edit::RenameProject { name } => {
            let name = name.trim();
            if name.is_empty() {
                return Err(EditError::Invalid("the project needs a name".into()));
            }
            Ok(vec![Command::SetName { name: name.into() }])
        }
        Edit::PlaceTracks { tracks, before, output } => place_tracks(p, tracks, before.as_deref(), output),
        Edit::RenameTrack { track, name, color } => rename_track(p, track, name, *color),
        Edit::SetLocked { track, locked } => track_cmd(p, track, |t| {
            t.locked = *locked;
            Ok(())
        }),
        Edit::SetOutput { track, output } => set_output(p, track, output.clone()),
        Edit::SetSend { track, target, level_db } => set_send(p, track, target, *level_db),
        Edit::SetSidechain { track, source, block, band } => {
            set_sidechain(p, track, block.as_deref(), *band, source.as_deref())
        }
        Edit::SetEqBand { track, band, enabled, kind, block } => track_cmd(p, track, |t| {
            use crate::fx::{Fx, FxKind};
            let blk = match block {
                Some(b) => t.chain.block_mut(b),
                None => t.chain.first_mut(FxKind::Eq),
            }
            .ok_or_else(|| EditError::Invalid("no such EQ".into()))?;
            let none = || EditError::Invalid(format!("no EQ band {band}"));
            match &mut blk.fx {
                Fx::Eq(e) => {
                    let b = e.bands.get_mut(*band).ok_or_else(none)?;
                    b.enabled = *enabled;
                    b.kind = *kind;
                }
                Fx::DynamicEq(e) => {
                    let b = e.bands.get_mut(*band).ok_or_else(none)?;
                    b.enabled = *enabled;
                    b.kind = *kind;
                }
                _ => return Err(EditError::Invalid("not an EQ".into())),
            }
            Ok(())
        }),
        Edit::Chain { track, change } => chain_edit(p, track, change),
        Edit::SetBlock { track, block } => {
            let bpm = p.tempo.map(|t| t.bpm);
            track_cmd(p, track, |t| {
                let ret = t.is_return();
                let cur = t.chain.block_mut(&block.id).ok_or_else(|| EditError::Invalid("no such block".into()))?;
                if cur.fx.kind() != block.fx.kind() {
                    return Err(EditError::Invalid("a block keeps its kind".into()));
                }
                let mut b = block.clone();
                // A musical rate sets the rate from the tempo (FX-20), like the delay's time (FX-6).
                let rate = |sync: &Option<String>| -> Result<Option<f64>, EditError> {
                    match (sync.as_deref(), bpm) {
                        (Some(v), Some(bpm)) => musical_ms(v, bpm)
                            .map(|ms| Some((1000.0 / ms).clamp(0.02, 20.0)))
                            .ok_or_else(|| EditError::Invalid(format!("unknown note value {v}"))),
                        _ => Ok(None),
                    }
                };
                match &mut b.fx {
                    crate::fx::Fx::Modulation(m) => {
                        if let Some(hz) = rate(&m.sync)? {
                            m.rate_hz = hz.clamp(0.02, 10.0);
                        }
                    }
                    crate::fx::Fx::Tremolo(m) => {
                        if let Some(hz) = rate(&m.sync)? {
                            m.rate_hz = hz.clamp(0.1, 20.0);
                        }
                    }
                    _ => {}
                }
                *cur = b;
                t.chain.check(ret).map_err(EditError::Invalid)?;
                // The lanes of what the block no longer has (a band, a voice) go with it, as in a change of bands.
                let probe = t.clone();
                let id = block.id.as_str();
                t.automation.retain(|l| {
                    let of = l.param.strip_prefix("bypass.").unwrap_or(&l.param).split('.').next().unwrap_or("");
                    of != id
                        || l.param
                            .parse::<crate::params::ParamId>()
                            .ok()
                            .and_then(|pid| crate::params::get(&probe, &pid))
                            .is_some()
                });
                Ok(())
            })
        }
        Edit::SetReturn { track, ret } => track_cmd(p, track, |t| match (&t.ret, ret) {
            (Some(ReturnFx::Reverb { .. }), ReturnFx::Reverb { shimmer_semitones, .. }) => {
                if ![5.0, 7.0, 12.0].contains(shimmer_semitones) {
                    return Err(EditError::Invalid("the shimmer interval is +5, +7 or +12 semitones".into()));
                }
                t.ret = Some(ret.clone());
                Ok(())
            }
            (Some(ReturnFx::Delay { .. }), ReturnFx::Delay { .. }) => {
                t.ret = Some(ret.clone());
                Ok(())
            }
            _ => Err(EditError::Invalid("a return keeps its type".into())),
        }),
        Edit::SetReverbAlgorithm { track, algorithm } => track_cmd(p, track, |t| match &mut t.ret {
            Some(ReturnFx::Reverb { algorithm: a, .. }) => {
                *a = *algorithm;
                Ok(())
            }
            _ => Err(EditError::Invalid("not a reverb return".into())),
        }),
        Edit::SetDelaySync { track, sync } => {
            let bpm = p.tempo.map(|t| t.bpm);
            track_cmd(p, track, |t| match &mut t.ret {
                Some(ReturnFx::Delay { sync: s, time_ms, .. }) => {
                    if let (Some(v), Some(bpm)) = (sync.as_deref(), bpm) {
                        *time_ms =
                            musical_ms(v, bpm).ok_or_else(|| EditError::Invalid(format!("unknown note value {v}")))?;
                    }
                    *s = sync.clone();
                    Ok(())
                }
                _ => Err(EditError::Invalid("only a delay return has a musical time".into())),
            })
        }
        Edit::AddSection { name, start, end } => {
            if p.sections.iter().any(|s| s.start < *end && *start < s.end) {
                return Err(EditError::Invalid("sections cannot overlap".into()));
            }
            add_section(p, name, *start, *end)
        }
        Edit::MoveBoundary { section, to } => move_boundary(p, section, *to),
        Edit::MoveSectionStart { section, to } => {
            let mut sections = p.sections.clone();
            sections.sort_by_key(|s| s.start);
            let i = sections
                .iter()
                .position(|s| &s.id == section)
                .ok_or_else(|| EditError::Invalid(format!("no section {section}")))?;
            if i > 0 && sections[i - 1].end == sections[i].start {
                let prev = sections[i - 1].id.clone();
                return move_boundary(p, &prev, *to);
            }
            let lo = if i > 0 { sections[i - 1].end } else { 0 };
            sections[i].start = (*to).clamp(lo, sections[i].end - 1);
            Ok(vec![Command::SetSections { sections }])
        }
        Edit::RenameSection { section, name } => rename_section(p, section, name),
        Edit::DeleteSection { section } => Ok(delete_section(p, section)),
        Edit::SetLane { track, param, points } => automation::set_lane(p, track, param, points.clone(), Author::User)
            .map(|c| vec![c])
            .ok_or_else(|| EditError::Invalid(format!("`{param}` cannot be automated on this track"))),
        Edit::DrawLane { track, param, stroke, tolerance } => {
            // Clamp to the parameter's range first, so the simplification keeps the shape that will be heard.
            let spec = param
                .parse::<crate::params::ParamId>()
                .map_err(|_| EditError::Invalid(format!("`{param}` is not a parameter")))?
                .spec();
            let mut s: Vec<AutomationPoint> = stroke
                .iter()
                .map(|x| AutomationPoint { frame: x.frame, value: x.value.clamp(spec.min, spec.max) })
                .collect();
            automation::sort_points(&mut s);
            let (Some(a), Some(b)) = (s.first().map(|x| x.frame), s.last().map(|x| x.frame)) else {
                return Ok(Vec::new());
            };
            let existing = p.track(track).and_then(|t| t.lane(param)).map(|l| l.points.clone()).unwrap_or_default();
            let mut pts = automation::delete_range(&existing, a, b);
            pts.extend(automation::simplify(&s, *tolerance));
            automation::set_lane(p, track, param, pts, Author::User)
                .map(|c| vec![c])
                .ok_or_else(|| EditError::Invalid(format!("`{param}` cannot be automated on this track")))
        }
        Edit::PasteLane { track, param, at, points } => {
            let existing = p.track(track).and_then(|t| t.lane(param)).map(|l| l.points.clone()).unwrap_or_default();
            let pts = automation::paste_range(&existing, points, *at);
            automation::set_lane(p, track, param, pts, Author::User)
                .map(|c| vec![c])
                .ok_or_else(|| EditError::Invalid(format!("`{param}` cannot be automated on this track")))
        }
        Edit::DeleteLaneRange { track, param, from, to } => {
            let existing = p.track(track).and_then(|t| t.lane(param)).map(|l| l.points.clone()).unwrap_or_default();
            automation::set_lane(p, track, param, automation::delete_range(&existing, *from, *to), Author::User)
                .map(|c| vec![c])
                .ok_or_else(|| EditError::Invalid(format!("no lane `{param}`")))
        }
        Edit::SetRole { track, role } => track_cmd(p, track, |t| {
            if !t.is_audio() {
                return Err(EditError::NotAudio);
            }
            t.role = *role;
            Ok(())
        }),
        Edit::RemoveLane { track, param } => track_cmd(p, track, |t| {
            t.automation.retain(|l| &l.param != param);
            Ok(())
        }),
        Edit::OffsetLane { track, param, delta } => automation::offset_lane(p, track, param, *delta)
            .map(|c| vec![c])
            .ok_or_else(|| EditError::Invalid(format!("no lane `{param}`"))),
        Edit::SetTempo { tempo } => Ok(vec![Command::SetTempo { tempo: *tempo }]),
        Edit::SetBrief { brief } => Ok(vec![Command::SetBrief { brief: brief.clone() }]),
        Edit::SetMaster { master } => {
            master.check().map_err(EditError::Invalid)?;
            // The mixdown is not the author's to set: bounces record it (MDN-1); a stale copy cannot move it back.
            let mut m = master.clone();
            m.mixdown = p.master.mixdown.clone();
            Ok(vec![Command::SetMaster { master: m }])
        }
        Edit::SetMixBus { fader_db } => {
            if !(-144.0..=12.0).contains(fader_db) {
                return Err(EditError::Invalid("the mix bus fader is −∞ … +12 dB".into()));
            }
            Ok(vec![Command::SetMixBus { bus: MixBus { fader_db: *fader_db } }])
        }
    }
}

/// Milliseconds of a musical value at a tempo (FX-6): "1/4", "1/8", "1/8." (dotted), "1/8t" (triplet).
pub fn musical_ms(v: &str, bpm: f64) -> Option<f64> {
    let (base, mult) = if let Some(b) = v.strip_suffix('.') {
        (b, 1.5)
    } else if let Some(b) = v.strip_suffix('t') {
        (b, 2.0 / 3.0)
    } else {
        (v, 1.0)
    };
    let den: f64 = base.strip_prefix("1/")?.parse().ok()?;
    if !(1.0..=64.0).contains(&den) || bpm <= 0.0 {
        return None;
    }
    Some(60_000.0 / bpm * 4.0 / den * mult)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: MAS-1, MAS-9, MAS-12, MAS-14, MAS-15
    #[test]
    fn set_master_keeps_the_document_valid() {
        let mut p = crate::model::Project::new("t");
        let ok = |p: &mut crate::model::Project, f: &dyn Fn(&mut crate::model::MasterState)| {
            let mut m = p.master.clone();
            f(&mut m);
            let valid = run(p, &Edit::SetMaster { master: Box::new(m.clone()) }).is_ok();
            if valid {
                p.master = m;
            }
            valid
        };
        assert!(ok(&mut p, &|m| m.metadata.isrc = "IT-ABC-26-00001".into()));
        assert!(ok(&mut p, &|m| m.metadata.isrc = "itabc2600001".into()));
        assert!(ok(&mut p, &|m| m.metadata.isrc = String::new()));
        // Half-hyphenated or malformed: refused, the same rule as the export's tags.
        assert!(!ok(&mut p, &|m| m.metadata.isrc = "ITABC-26-00001".into()));
        assert!(!ok(&mut p, &|m| m.metadata.isrc = "IT-ABC-26-0001".into()));
        use crate::model::{Block, BlockKind, SectionOffsets};
        assert!(!ok(&mut p, &|m| {
            if let Some(Block::Compressor(c)) = m.chain.blocks.iter_mut().map(|b| &mut b.block).nth(2) {
                c.ratio = 50.0;
            }
        }));
        // MAS-15: two of a repeatable kind at most, one match EQ, one width, nine free blocks in all.
        assert!(ok(&mut p, &|m| assert!(m.chain.add(BlockKind::Eq, 0).is_some())));
        assert_eq!(p.master.chain.name("b8").as_deref(), Some("EQ 1"));
        assert_eq!(p.master.chain.name("b1").as_deref(), Some("EQ 2"));
        assert!(p.master.chain.add(BlockKind::Eq, 0).is_none() && p.master.chain.add(BlockKind::Width, 0).is_none());
        assert!(ok(&mut p, &|m| assert!(m.chain.add(BlockKind::Clipper, 99).is_some())));
        assert!(p.master.chain.add(BlockKind::Saturation, 0).is_none(), "nine free blocks are the limit");
        assert!(!ok(&mut p, &|m| {
            let twin = m.chain.blocks[0].clone();
            m.chain.blocks.push(twin);
        }));
        // MAS-12: two to six bands, crossovers rising a third of an octave apart.
        let mb = |m: &mut crate::model::MasterState, f: &dyn Fn(&mut crate::model::Multiband)| {
            for b in &mut m.chain.blocks {
                if let Block::Multiband(x) = &mut b.block {
                    f(x);
                }
            }
        };
        assert!(ok(&mut p, &|m| mb(m, &|x| assert!(x.split(1, 1000.0)))));
        assert!(!ok(&mut p, &|m| mb(m, &|x| x.crossovers_hz[1] = 125.0)));
        assert!(ok(&mut p, &|m| mb(m, &|x| {
            assert!(!x.split(0, 119.0), "too close to the crossover above");
            assert!(x.merge(0) && x.merge(0));
            assert!(!x.merge(0), "two bands are the least");
        })));
        // The structure changes the interface asks for (ChainOp).
        use crate::model::ChainOp;
        let op = |p: &mut crate::model::Project, o: ChainOp| {
            let r = p.master.chain_edit(&o);
            if let Ok((m, _)) = &r {
                p.master = m.clone();
            }
            r.map(|(_, id)| id)
        };
        let mb_id = p.master.chain.first(BlockKind::Multiband).unwrap().id.clone();
        let bands = |p: &crate::model::Project| match &p.master.chain.block(&mb_id).unwrap().block {
            Block::Multiband(x) => (x.bands.len(), x.crossovers_hz.clone()),
            _ => unreachable!(),
        };
        assert_eq!(op(&mut p, ChainOp::Bands { id: mb_id.clone(), bands: 5 }), Ok(None));
        assert_eq!(bands(&p).0, 5);
        assert!(bands(&p).1.windows(2).all(|w| w[1] > w[0] * 1.26), "{:?}", bands(&p).1);
        assert!(op(&mut p, ChainOp::Bands { id: mb_id.clone(), bands: 7 }).is_err());
        assert_eq!(op(&mut p, ChainOp::Bands { id: mb_id.clone(), bands: 2 }), Ok(None));
        assert_eq!(op(&mut p, ChainOp::Split { id: mb_id.clone(), hz: 2000.0 }), Ok(None));
        assert_eq!(op(&mut p, ChainOp::Merge { id: mb_id.clone(), crossover: 0 }), Ok(None));
        assert_eq!(bands(&p), (2, vec![2000.0]));
        let first = p.master.chain.blocks[0].id.clone();
        assert_eq!(op(&mut p, ChainOp::Move { id: first.clone(), to: 3 }), Ok(None));
        assert_eq!(p.master.chain.blocks[3].id, first);
        assert!(op(&mut p, ChainOp::Add { kind: BlockKind::Compressor, at: None }).unwrap_err().contains("full"));
        assert_eq!(op(&mut p, ChainOp::Remove { id: first.clone() }), Ok(None));
        let new = op(&mut p, ChainOp::Add { kind: BlockKind::Compressor, at: Some(0) }).unwrap().unwrap();
        assert!(p.master.chain.blocks[0].id == new && p.master.chain.blocks[0].block.bypass());
        assert!(op(&mut p, ChainOp::Add { kind: BlockKind::Compressor, at: None }).is_err(), "two compressors already");
        assert_eq!(op(&mut p, ChainOp::ResetOrder), Ok(None));
        let kinds: Vec<BlockKind> = p.master.chain.blocks.iter().map(|b| b.block.kind()).collect();
        assert!(kinds.windows(2).all(|w| {
            let at = |k: &BlockKind| BlockKind::ALL.iter().position(|x| x == k).unwrap();
            at(&w[0]) <= at(&w[1])
        }));
        assert_eq!(op(&mut p, ChainOp::Reset { id: mb_id.clone() }), Ok(None));
        assert_eq!(bands(&p).0, 3, "a multiband as a new one has it");
        // MAS-14: offsets of a section within their limits, on an EQ the chain has.
        assert!(ok(&mut p, &|m| m.sections =
            vec![SectionOffsets { level_db: 0.5, ..SectionOffsets::new(&"s1".to_string()) }]));
        assert!(!ok(&mut p, &|m| m.sections[0].level_db = 9.0));
        assert!(!ok(&mut p, &|m| {
            m.sections[0].eq = vec![crate::model::EqOffsets { block: "nope".into(), gains_db: vec![] }];
        }));
        assert!(!ok(&mut p, &|m| m.chain.input_gain_db = f64::NAN));
        assert!(!ok(&mut p, &|m| m.export.sample_rate = 22_050));
        assert!(!ok(&mut p, &|m| m.target.true_peak_db = 1.0));
        assert!(crate::model::MasterState::default().check().is_ok());
    }
    use crate::history::{Document, Phase};
    use crate::model::{AudioFormat, MediaFile};

    fn media(id: &str, frames: u64) -> MediaFile {
        MediaFile {
            id: id.into(),
            path: format!("audio/{id}.wav"),
            original_name: "x.wav".into(),
            format: AudioFormat::Wav,
            channels: 1,
            sample_rate: 48000,
            frames,
            encoding: "PCM 24-bit".into(),
            converted: false,
        }
    }

    fn clip(id: &str, start: i64, length: u64) -> Clip {
        Clip {
            id: id.into(),
            media: "m1".into(),
            start,
            offset: 0,
            length,
            gain_db: 0.0,
            fade_in: Fade::default(),
            fade_out: Fade::default(),
            mute: false,
        }
    }

    fn doc() -> Document {
        let mut p = Project::new("e");
        p.media.push(media("m1", 1_000_000));
        let mut a = Track::new_audio("t1".into(), "A", None, 1, 1);
        a.clips = vec![clip("c1", 0, 1000), clip("c2", 2000, 1000)];
        let mut b = Track::new_audio("t2".into(), "B", None, 1, 2);
        b.clips = vec![clip("c3", 0, 5000)];
        let s = Track::new_audio("t3".into(), "Stereo", None, 2, 3);
        p.tracks.extend([a, b, s]);
        p.next_id = 100;
        Document::new(p)
    }

    fn apply(d: &mut Document, cmds: Vec<Command>) {
        d.transact(Phase::Mix, "edit", cmds, None).unwrap();
    }

    /// req: CLP-1, CLP-4
    #[test]
    fn moving_a_clip_over_another_trims_the_one_underneath_in_one_step() {
        let mut d = doc();
        let cmds = move_clips(&mut d.project, &[("t1".into(), "c1".into())], 1500, None, false).unwrap();
        apply(&mut d, cmds);
        let t = d.project.track("t1").unwrap();
        assert_eq!(t.clips.len(), 2);
        assert_eq!((t.clips[0].start, t.clips[0].length), (1500, 1000));
        // c2 (2000..3000) lost its head up to 2500.
        assert_eq!((t.clips[1].start, t.clips[1].offset, t.clips[1].length), (2500, 500, 500));
        assert_eq!(d.history(Phase::Mix).position(), 1);
        d.undo(Phase::Mix).unwrap();
        assert_eq!(d.project.track("t1").unwrap().clips, doc().project.track("t1").unwrap().clips);
    }

    /// req: CLP-1, CLP-4
    #[test]
    fn a_clip_dropped_inside_another_splits_it_and_moves_across_tracks_only_with_same_format() {
        let mut d = doc();
        let cmds = move_clips(&mut d.project, &[("t1".into(), "c1".into())], 2000, Some("t2"), false).unwrap();
        apply(&mut d, cmds);
        let b = d.project.track("t2").unwrap();
        let spans: Vec<_> = b.clips.iter().map(|c| (c.start, c.offset, c.length)).collect();
        assert_eq!(spans, vec![(0, 0, 2000), (2000, 0, 1000), (3000, 3000, 2000)]);
        assert_eq!(d.project.track("t1").unwrap().clips.len(), 1);
        let err = move_clips(&mut d.project, &[("t2".into(), "c3".into())], 0, Some("t3"), false);
        assert_eq!(err, Err(EditError::ChannelMismatch));
    }

    /// req: CLP-1
    #[test]
    fn split_trim_duplicate_delete_and_mute() {
        let mut d = doc();
        let cmds = split_at(&mut d.project, &[], 500);
        apply(&mut d, cmds);
        let t = d.project.track("t1").unwrap();
        assert_eq!(
            t.clips.iter().map(|c| (c.start, c.offset, c.length)).collect::<Vec<_>>()[..2],
            [(0, 0, 500), (500, 500, 500)]
        );
        // Trim the left edge of c2 earlier than its media allows: clamps at offset 0.
        let cmds = trim_clip(&d.project, &("t1".into(), "c2".into()), true, 1500).unwrap();
        apply(&mut d, cmds);
        let c2 = d.project.track("t1").unwrap().clip("c2").unwrap().clone();
        assert_eq!((c2.start, c2.offset, c2.length), (2000, 0, 1000));
        let cmds = trim_clip(&d.project, &("t1".into(), "c2".into()), false, 2600).unwrap();
        apply(&mut d, cmds);
        assert_eq!(d.project.track("t1").unwrap().clip("c2").unwrap().length, 600);
        let cmds = duplicate_clips(&mut d.project, &[("t1".into(), "c2".into())]).unwrap();
        apply(&mut d, cmds);
        let t = d.project.track("t1").unwrap();
        assert!(t.clips.iter().any(|c| c.start == 2600 && c.length == 600 && c.id != "c2"));
        let cmds = update_clip(&d.project, &("t1".into(), "c2".into()), |c| c.mute = true).unwrap();
        apply(&mut d, cmds);
        assert!(d.project.track("t1").unwrap().clip("c2").unwrap().mute);
        let cmds = delete_clips(&d.project, &[("t1".into(), "c2".into())]).unwrap();
        apply(&mut d, cmds);
        assert!(d.project.track("t1").unwrap().clip("c2").is_none());
    }

    /// req: CLP-2
    #[test]
    fn fades_and_clip_gain_have_limits() {
        let d = doc();
        let r: ClipRef = ("t1".into(), "c1".into());
        let cmds = set_fade(&d.project, &r, true, 5000, FadeShape::Linear).unwrap();
        let Command::SetTrack { track } = &cmds[0] else { panic!() };
        assert_eq!(track.clip("c1").unwrap().fade_in.length, 1000);
        assert!(update_clip(&d.project, &r, |c| c.gain_db = 30.0).is_err());
        assert!(update_clip(&d.project, &r, |c| c.gain_db = -24.0).is_ok());
    }

    /// req: TRK-1, TRK-5
    #[test]
    fn returns_can_be_created_up_to_eight_and_deleting_one_removes_its_sends() {
        let mut d = doc();
        let r1 = d.project.tracks.iter().find(|t| t.is_return()).unwrap().id.clone();
        let cmds = set_send(&d.project, "t1", &r1, Some(-12.0)).unwrap();
        apply(&mut d, cmds);
        for _ in 0..6 {
            let cmds = add_return(&mut d.project, ReturnFx::default_reverb()).unwrap();
            apply(&mut d, cmds);
        }
        assert_eq!(add_return(&mut d.project, ReturnFx::default_delay()), Err(EditError::TooManyReturns));
        let cmds = delete_track(&d.project, &r1).unwrap();
        apply(&mut d, cmds);
        assert!(d.project.track("t1").unwrap().sends.is_empty());
        assert!(d.project.track(&r1).is_none());
        d.undo(Phase::Mix).unwrap();
        assert_eq!(d.project.track("t1").unwrap().send_db(&r1), Some(-12.0));
    }

    /// req: TRK-1, TRK-3
    #[test]
    fn groups_take_members_and_deleting_a_group_reroutes_them() {
        let mut d = doc();
        let cmds = add_group(&mut d.project, "Drums", &["t1".into(), "t2".into()]).unwrap();
        apply(&mut d, cmds);
        let g = d.project.tracks.iter().find(|t| t.is_group()).unwrap().id.clone();
        assert_eq!(d.project.track("t1").unwrap().output, Output::Group(g.clone()));
        let cmds = duplicate_track(&mut d.project, "t1").unwrap();
        apply(&mut d, cmds);
        assert_eq!(d.project.tracks.iter().filter(|t| t.name == "A copy").count(), 1);
        let cmds = delete_track(&d.project, &g).unwrap();
        apply(&mut d, cmds);
        assert_eq!(d.project.track("t1").unwrap().output, Output::MixBus);
    }

    /// req: TRK-1, TRK-3
    #[test]
    fn dragged_tracks_move_in_one_step_into_and_out_of_groups() {
        let mut d = doc();
        let ids = |d: &Document| d.project.tracks.iter().map(|t| t.id.clone()).collect::<Vec<_>>();
        let (a, b) = ("t1".to_string(), "t2".to_string());
        // Reorder: the first track goes to the end.
        let cmds =
            run(&mut d.project, &Edit::PlaceTracks { tracks: vec![a.clone()], before: None, output: Output::MixBus })
                .unwrap();
        d.transact(Phase::Mix, "move", cmds, None).unwrap();
        assert_eq!(ids(&d).last(), Some(&a));
        // Into a group made from both tracks, then one of them out again, before the group.
        let cmds =
            run(&mut d.project, &Edit::AddGroup { name: "Bus".into(), members: vec![a.clone(), b.clone()] }).unwrap();
        d.transact(Phase::Mix, "group", cmds, None).unwrap();
        let g = d.project.tracks.iter().find(|t| t.is_group()).unwrap().id.clone();
        assert!([&a, &b].iter().all(|id| d.project.track(id).unwrap().output == Output::Group(g.clone())));
        let cmds = run(
            &mut d.project,
            &Edit::PlaceTracks { tracks: vec![b.clone()], before: Some(g.clone()), output: Output::MixBus },
        )
        .unwrap();
        let pos = d.history(Phase::Mix).position();
        d.transact(Phase::Mix, "out", cmds, None).unwrap();
        assert_eq!(d.history(Phase::Mix).position(), pos + 1, "one undo step");
        assert_eq!(d.project.track(&b).unwrap().output, Output::MixBus);
        let order = ids(&d);
        assert!(order.iter().position(|x| *x == b) < order.iter().position(|x| *x == g));
        // A group cannot go inside itself, nor a return into a group.
        let into_self = Edit::PlaceTracks { tracks: vec![g.clone()], before: None, output: Output::Group(g.clone()) };
        assert!(run(&mut d.project, &into_self).is_err());
        let cmds = add_return(&mut d.project, ReturnFx::default_reverb()).unwrap();
        d.transact(Phase::Mix, "ret", cmds, None).unwrap();
        let r = d.project.tracks.iter().find(|t| t.is_return()).unwrap().id.clone();
        let ret_in = Edit::PlaceTracks { tracks: vec![r.clone()], before: None, output: Output::Group(g.clone()) };
        assert!(run(&mut d.project, &ret_in).is_err());
        assert!(run(&mut d.project, &Edit::AddGroup { name: "X".into(), members: vec![r] }).is_err());
        // Undo puts the track back in the group, in its place.
        d.undo(Phase::Mix).unwrap();
        d.undo(Phase::Mix).unwrap();
        assert_eq!(d.project.track(&b).unwrap().output, Output::Group(g));
    }

    /// req: SEC-1
    #[test]
    fn sections_are_created_renamed_moved_and_deleted() {
        let mut d = doc();
        let cmds = add_section(&mut d.project, "Intro", 0, 1000).unwrap();
        apply(&mut d, cmds);
        let cmds = add_section(&mut d.project, "Verse", 1000, 3000).unwrap();
        apply(&mut d, cmds);
        let intro = d.project.sections[0].id.clone();
        let cmds = move_boundary(&d.project, &intro, 1500).unwrap();
        apply(&mut d, cmds);
        assert_eq!((d.project.sections[0].end, d.project.sections[1].start), (1500, 1500));
        assert!(move_boundary(&d.project, &intro, 3000).is_err());
        let cmds = rename_section(&d.project, &intro, "Opening").unwrap();
        apply(&mut d, cmds);
        assert_eq!(d.project.sections[0].name, "Opening");
        let cmds = delete_section(&d.project, &intro);
        apply(&mut d, cmds);
        assert_eq!(d.project.sections.len(), 1);
        let overlapping = add_section(&mut d.project, "Bad", 2000, 2500).unwrap();
        assert!(d.transact(Phase::Mix, "x", overlapping, None).is_err());
    }

    /// req: CLP-1, CLP-2, SEC-1, TRK-1, TRK-3, AUT-2, AUT-3
    #[test]
    fn edits_as_data_run_as_one_step_each_and_round_trip_as_json() {
        let mut d = doc();
        let edits = vec![
            Edit::SplitAt { tracks: vec!["t2".into()], frame: 2500 },
            Edit::ClipGain { clip: ("t1".into(), "c1".into()), gain_db: -3.0 },
            Edit::ClipMute { clips: vec![("t1".into(), "c1".into()), ("t1".into(), "c2".into())], mute: true },
            Edit::Fade { clip: ("t1".into(), "c2".into()), fade_in: true, length: 200, shape: FadeShape::EqualPower },
            Edit::AddSection { name: "Verse".into(), start: 0, end: 4000 },
            Edit::AddReturn { reverb: false, algorithm: None },
            Edit::AddGroup { name: "Bus".into(), members: vec!["t1".into(), "t2".into()] },
            Edit::SetLane {
                track: "t1".into(),
                param: "fader".into(),
                points: vec![AutomationPoint { frame: 0, value: -6.0 }, AutomationPoint { frame: 1000, value: 40.0 }],
            },
            Edit::OffsetLane { track: "t1".into(), param: "fader".into(), delta: -2.0 },
            Edit::SetEqBand { track: "t1".into(), band: 1, enabled: true, kind: EqKind::HighShelf, block: None },
            Edit::SetLocked { track: "t2".into(), locked: true },
            Edit::SetMixBus { fader_db: -1.5 },
            Edit::RenameProject { name: " Song ".into() },
        ];
        for (i, e) in edits.iter().enumerate() {
            let back: Edit = serde_json::from_str(&serde_json::to_string(e).unwrap()).unwrap();
            assert_eq!(&back, e);
            let cmds = run(&mut d.project, e).unwrap();
            d.transact(Phase::Mix, "edit", cmds, None).unwrap();
            assert_eq!(d.history(Phase::Mix).position(), i + 1, "{e:?}");
        }
        let t1 = d.project.track("t1").unwrap();
        assert!(t1.clips.iter().all(|c| c.mute) && t1.clips[0].gain_db == -3.0);
        assert_eq!(d.project.track("t2").unwrap().clips.len(), 2);
        // The lane is clamped to the fader range (+12 dB), then offset by −2 dB.
        let lane = t1.lane("fader").unwrap();
        assert_eq!((lane.points[0].value, lane.points[1].value), (-8.0, 10.0));
        assert!(t1.chain.eq().unwrap().bands[1].enabled && t1.chain.eq().unwrap().bands[1].kind == EqKind::HighShelf);
        assert!(matches!(t1.output, Output::Group(_)));
        assert!(d.project.track("t2").unwrap().locked);
        assert_eq!(d.project.mix_bus.fader_db, -1.5);
        // An overlapping section and an unknown lane are refused before touching the document.
        assert!(run(&mut d.project, &Edit::AddSection { name: "x".into(), start: 3000, end: 5000 }).is_err());
        assert!(
            run(&mut d.project, &Edit::SetLane { track: "t1".into(), param: "send.zz".into(), points: vec![] })
                .is_err()
        );
        // Every step undoes back to the start.
        while d.undo(Phase::Mix).is_ok() {}
        let start = doc();
        assert_eq!(d.project.tracks.iter().filter(|t| !t.is_return()).count(), 3);
        assert_eq!(d.project.track("t1").unwrap().clips, start.project.track("t1").unwrap().clips);
        assert!(d.project.sections.is_empty() && d.project.mix_bus == start.project.mix_bus);
    }

    /// req: AUT-2
    #[test]
    fn freehand_strokes_paste_and_range_delete_edit_a_lane_in_one_step_each() {
        let mut d = doc();
        let p = |frame, value| AutomationPoint { frame, value };
        // A straight stroke of 100 samples simplifies to its two ends.
        let stroke: Vec<_> = (0..100).map(|k| p(k * 10, -10.0 + k as f64 * 0.1)).collect();
        let e = Edit::DrawLane { track: "t1".into(), param: "pan".into(), stroke, tolerance: 0.05 };
        let c = run(&mut d.project, &e).unwrap();
        apply(&mut d, c);
        let lane = d.project.track("t1").unwrap().lane("pan").unwrap().points.clone();
        // Clamped to the pan range (flat at −1 until frame 900), then simplified to the corners.
        let got: Vec<(i64, f64)> = lane.iter().map(|x| (x.frame, (x.value * 1000.0).round() / 1000.0)).collect();
        assert_eq!(got, vec![(0, -1.0), (900, -1.0), (990, -0.1)]);
        let e =
            Edit::PasteLane { track: "t1".into(), param: "pan".into(), at: 2000, points: vec![p(0, 0.5), p(100, 0.5)] };
        let c = run(&mut d.project, &e).unwrap();
        apply(&mut d, c);
        assert_eq!(d.project.track("t1").unwrap().lane("pan").unwrap().points.len(), 5);
        let e = Edit::DeleteLaneRange { track: "t1".into(), param: "pan".into(), from: 1500, to: 2500 };
        let c = run(&mut d.project, &e).unwrap();
        apply(&mut d, c);
        assert_eq!(d.project.track("t1").unwrap().lane("pan").unwrap().points.len(), 3);
        assert_eq!(d.history(Phase::Mix).position(), 3);
        let c =
            run(&mut d.project, &Edit::SetRole { track: "t1".into(), role: Some(crate::role::Role::Bass) }).unwrap();
        apply(&mut d, c);
        assert_eq!(d.project.track("t1").unwrap().role, Some(crate::role::Role::Bass));
    }

    /// req: SEC-1
    #[test]
    fn moving_a_section_start_moves_the_shared_boundary() {
        let mut d = doc();
        for (n, a, b) in [("A", 0, 1000), ("B", 1000, 2000)] {
            let c = run(&mut d.project, &Edit::AddSection { name: n.into(), start: a, end: b }).unwrap();
            apply(&mut d, c);
        }
        let b = d.project.sections[1].id.clone();
        let c = run(&mut d.project, &Edit::MoveSectionStart { section: b, to: 800 }).unwrap();
        apply(&mut d, c);
        assert_eq!((d.project.sections[0].end, d.project.sections[1].start), (800, 800));
    }

    /// req: FX-6
    #[test]
    fn musical_delay_times_follow_the_tempo() {
        assert_eq!(musical_ms("1/4", 120.0), Some(500.0));
        assert_eq!(musical_ms("1/8.", 120.0), Some(375.0));
        assert!((musical_ms("1/8t", 120.0).unwrap() - 166.6667).abs() < 1e-3);
        assert_eq!(musical_ms("3/4", 120.0), None);
    }
}
