//! From plans to project changes and back (AI-2, AI-4 … AI-7, AUT-5, MAS-6).
//!
//! - [`plan_from_project`]: the current mix as a plan (the starting point of a revision, AI-3, and the "before" of
//!   the diff).
//! - [`apply_mix_plan`]: the commands of one proposal (HIS-3): chains, sends, returns, per-section automation
//!   written as lanes with ramps at the boundaries (AUT-5), input trims towards −23 LUFS (AI-7), the numbered notes.
//!   Locked tracks and tracks outside the scope are left untouched (AI-4, AI-5).
//! - [`diff`]: rows of parameter changes per track for the review screen.

use std::collections::{BTreeMap, HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use stemwise_core::automation::{section_points, value_at};
use stemwise_core::command::Command;
use stemwise_core::fx::{Fx, FxKind};
use stemwise_core::model::{
    AiNote, Author, AutomationLane, Compressor, EqBand as CoreBand, EqKind, Project, ReturnFx, Saturation, Send,
    Sidechain, Track,
};
use ts_rs::TS;

use crate::analysis_doc::Ids;
use crate::plan::{
    BlockPlan, BlockPosition, CompressorPlan, EqBand, EqType, GroupPlan, MIX_PLAN_VERSION, MixPlan, ReturnPlan,
    SaturationPlan, SectionPlan, TrackPlan,
};

/// Default ramp of per-section values at boundaries, in seconds (AUT-5).
pub const SECTION_RAMP_S: f64 = 0.08;

fn eq_type(k: EqKind) -> EqType {
    match k {
        EqKind::Peak => EqType::Peak,
        EqKind::LowShelf => EqType::LowShelf,
        EqKind::HighShelf => EqType::HighShelf,
    }
}

fn eq_kind(k: EqType) -> EqKind {
    match k {
        EqType::Peak => EqKind::Peak,
        EqType::LowShelf => EqKind::LowShelf,
        EqType::HighShelf => EqKind::HighShelf,
    }
}

fn eq_of(t: &Track) -> Vec<EqBand> {
    let Some(eq) = t.chain.eq().filter(|e| !e.bypass) else { return Vec::new() };
    eq.bands
        .iter()
        .filter(|b| b.enabled)
        .map(|b| EqBand { kind: eq_type(b.kind), freq_hz: b.freq_hz, gain_db: b.gain_db, q: b.q })
        .collect()
}

fn comp_of(t: &Track, ids: &Ids) -> Option<CompressorPlan> {
    let c = t.chain.comp()?;
    (!c.bypass).then(|| CompressorPlan {
        threshold_db: c.threshold_db,
        ratio: c.ratio,
        attack_ms: c.attack_ms,
        release_ms: c.release_ms,
        knee_db: c.knee_db,
        makeup_db: c.makeup_db,
        mix: c.mix,
        sidechain: c.sidechain.as_ref().map(|s| ids.n(&s.source)),
    })
}

fn sat_of(t: &Track) -> Option<SaturationPlan> {
    t.chain.sat().filter(|s| !s.bypass).map(|s| SaturationPlan {
        drive_db: s.drive_db,
        mix: s.mix,
        character: Some(s.character),
    })
}

fn sends_of(t: &Track, ids: &Ids) -> BTreeMap<String, f64> {
    t.sends.iter().map(|s| (ids.n(&s.target), s.level_db)).collect()
}

/// Value of the assistant's lane of `param` in the middle of a section, if the assistant wrote one.
fn ai_lane_mid(t: &Track, param: &str, mid: i64) -> Option<f64> {
    t.automation
        .iter()
        .find(|l| l.param == param && l.author == Author::Ai && !l.points.is_empty())
        .and_then(|l| value_at(&l.points, mid, false))
}

/// The value of a stepped lane the assistant wrote, at a frame.
fn ai_lane_step(t: &Track, param: &str, at: i64) -> Option<f64> {
    t.automation
        .iter()
        .find(|l| l.param == param && l.author == Author::Ai && !l.points.is_empty())
        .and_then(|l| value_at(&l.points, at, true))
}

/// The id of the first EQ of a chain, the one the fields of version 3 address (`eq` in every chain of schema 3).
fn eq_id(t: &Track) -> String {
    t.chain.first(FxKind::Eq).map_or_else(|| "eq".to_string(), |b| b.id.clone())
}

/// The listed blocks of a chain as the plan writes them (AI-25: a revision starts from them).
fn blocks_of(t: &Track, ids: &Ids) -> Vec<BlockPlan> {
    crate::blocks::listed(&t.chain)
        .into_iter()
        .filter_map(|b| {
            Some(BlockPlan {
                id: ids.block_n(&t.id, &b.id)?,
                kind: b.fx.kind(),
                idea: None,
                position: None,
                bypass: Some(b.fx.bypass()),
                settings: crate::blocks::settings_of(&b.fx, ids),
            })
        })
        .collect()
}

/// A musical rate set from the tempo (FX-20), as the core does for a block the author edits; no tempo, no sync.
fn sync_rate(fx: &mut Fx, bpm: Option<f64>) {
    let hz = |sync: &Option<String>| {
        sync.as_deref().zip(bpm).and_then(|(v, b)| stemwise_core::edit::musical_ms(v, b)).map(|ms| 1000.0 / ms)
    };
    match fx {
        Fx::Modulation(m) => match hz(&m.sync) {
            Some(r) => m.rate_hz = r.clamp(0.02, 10.0),
            None => m.sync = None,
        },
        Fx::Tremolo(m) => match hz(&m.sync) {
            Some(r) => m.rate_hz = r.clamp(0.1, 20.0),
            None => m.sync = None,
        },
        _ => {}
    }
}

/// Applies the plan's blocks to a chain (AI-25): a listed block takes the plan's settings, a new one goes at its
/// position if the chain has room. Returns the plan's block ids with the blocks they became.
fn apply_blocks(p: &Project, t: &mut Track, blocks: &[BlockPlan], ids: &Ids) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut placed: HashMap<BlockPosition, usize> = HashMap::new();
    let bpm = p.tempo.map(|x| x.bpm);
    for bp in blocks {
        let existing = ids
            .blocks
            .get(&bp.id)
            .filter(|(tr, b)| *tr == t.id && t.chain.block(b).is_some_and(|x| x.fx.kind() == bp.kind))
            .map(|(_, b)| b.clone());
        let base = existing.as_ref().and_then(|b| t.chain.block(b)).map(|b| b.fx.clone());
        let Ok(mut fx) = crate::blocks::to_fx(bp, base.as_ref(), ids) else { continue };
        sync_rate(&mut fx, bpm);
        // A sidechain source that is not in the project (a group left out of a partial application) is dropped.
        for s in fx.sidechains().into_iter().cloned().collect::<Vec<_>>() {
            if p.track(&s).is_none() || s == t.id {
                fx.drop_sidechain(&s);
            }
        }
        match existing {
            Some(b) => {
                if let Some(x) = t.chain.block_mut(&b) {
                    x.fx = fx;
                }
                map.insert(bp.id.clone(), b);
            }
            None => {
                // The proposal applied a second time (reopened from the list, AI-23, or all after some tracks): the
                // block it added is already there, exactly as the plan sets it. It is not added again.
                let taken: Vec<&String> = map.values().collect();
                if let Some(b) = t.chain.blocks.iter().find(|b| b.fx == fx && !taken.contains(&&b.id)) {
                    let id = b.id.clone();
                    map.insert(bp.id.clone(), id);
                    continue;
                }
                if t.chain.can_add(bp.kind, t.is_return()).is_err() {
                    continue;
                }
                let id = t.chain.free_id(bp.kind);
                // Several new blocks at one position keep the plan's order: after the ones already put there.
                let pos = bp.position.unwrap_or(BlockPosition::Last);
                let n = placed.entry(pos).or_insert(0usize);
                let shift = if matches!(pos, BlockPosition::First | BlockPosition::AfterCompressor) { *n } else { 0 };
                *n += 1;
                let at = (crate::blocks::insert_at(&t.chain, pos) + shift).min(t.chain.blocks.len());
                t.chain.blocks.insert(at, stemwise_core::fx::FxBlock { id: id.clone(), fx });
                map.insert(bp.id.clone(), id);
            }
        }
    }
    map
}

fn tenth(v: f64) -> f64 {
    let r = (v * 10.0).round() / 10.0;
    if r == 0.0 { 0.0 } else { r }
}

/// Per-section offsets rebuilt from the lanes the assistant wrote (AI-3: a revision starts from the current state,
/// per-section decisions included): the lane's value in the middle of each section minus the static value.
fn sections_of(
    p: &Project,
    t: &Track,
    ids: &Ids,
    eq: &[EqBand],
    sends: &BTreeMap<String, f64>,
) -> BTreeMap<String, SectionPlan> {
    let mut out = BTreeMap::new();
    for s in &p.sections {
        let mid = (s.start + s.end) / 2;
        let gain = ai_lane_mid(t, "fader", mid).map_or(0.0, |v| tenth(v - t.chain.fader_db));
        let send_offsets_db: BTreeMap<String, f64> = t
            .sends
            .iter()
            .filter_map(|x| {
                let level = sends.get(&ids.n(&x.target))?;
                let v = ai_lane_mid(t, &format!("send.{}", x.target), mid)?;
                let off = tenth(v - level);
                (off != 0.0).then(|| (ids.n(&x.target), off))
            })
            .collect();
        let mut eq_offs: Vec<f64> = eq
            .iter()
            .enumerate()
            .map(|(i, b)| {
                ai_lane_mid(t, &format!("{}.{}.gain", eq_id(t), i + 1), mid).map_or(0.0, |v| tenth(v - b.gain_db))
            })
            .collect();
        while eq_offs.last() == Some(&0.0) {
            eq_offs.pop();
        }
        // v4: the filters moved by octaves, and the blocks on only in some sections (AI-6).
        let oct = |kind: FxKind| -> Option<f64> {
            let b = playing_filter(&t.chain, kind)?;
            let (Fx::Hpf(f) | Fx::Lpf(f)) = &b.fx else { return None };
            let v = ai_lane_mid(t, &format!("{}.freq", b.id), mid)?;
            let o = tenth((v / f.freq_hz).log2());
            (o != 0.0).then_some(o)
        };
        let (hpf_offset_oct, lpf_offset_oct) = (oct(FxKind::Hpf), oct(FxKind::Lpf));
        let blocks_on: Vec<String> = crate::blocks::listed(&t.chain)
            .into_iter()
            .filter(|b| ai_lane_step(t, &format!("bypass.{}", b.id), mid) == Some(0.0))
            .filter_map(|b| ids.block_n(&t.id, &b.id))
            .collect();
        if gain != 0.0
            || !send_offsets_db.is_empty()
            || !eq_offs.is_empty()
            || hpf_offset_oct.is_some()
            || lpf_offset_oct.is_some()
            || !blocks_on.is_empty()
        {
            out.insert(
                ids.n(&s.id),
                SectionPlan {
                    gain_offset_db: gain,
                    send_offsets_db,
                    eq_gain_offsets_db: eq_offs,
                    idea: None,
                    hpf_offset_oct,
                    lpf_offset_oct,
                    blocks_on,
                },
            );
        }
    }
    out
}

/// The current mix as a plan: the exact values (manual ones outside the assistant's limits included, B2 of the F5
/// review) and the per-section offsets rebuilt from the assistant's lanes.
pub fn plan_from_project(p: &Project, ids: &Ids) -> MixPlan {
    let mut tracks = BTreeMap::new();
    let mut groups = BTreeMap::new();
    let mut returns = BTreeMap::new();
    let note = |id: &str| {
        p.ai.notes.iter().find(|n| n.track.as_deref() == Some(id)).map(|n| n.text.clone()).unwrap_or_default()
    };
    for t in &p.tracks {
        let n = ids.n(&t.id);
        if t.is_audio() {
            let eq = eq_of(t);
            let sends_db = sends_of(t, ids);
            let sections = sections_of(p, t, ids, &eq, &sends_db);
            tracks.insert(
                n,
                TrackPlan {
                    gain_db: t.chain.fader_db,
                    pan: t.chain.pan,
                    hpf_hz: t.chain.hpf().filter(|f| !f.bypass).map(|f| f.freq_hz),
                    lpf_hz: t.chain.lpf().filter(|f| !f.bypass).map(|f| f.freq_hz),
                    eq,
                    compressor: comp_of(t, ids),
                    saturation: sat_of(t),
                    sends_db,
                    sections,
                    hpf_resonance: t.chain.hpf().map(|f| f.resonance),
                    lpf_resonance: t.chain.lpf().map(|f| f.resonance),
                    blocks: blocks_of(t, ids),
                    idea_sends_db: BTreeMap::new(),
                    rationale: note(&t.id),
                    idea_sections: BTreeMap::new(),
                },
            );
        } else if t.is_group() {
            groups.insert(
                n,
                GroupPlan {
                    gain_db: t.chain.fader_db,
                    eq: eq_of(t),
                    compressor: comp_of(t, ids),
                    saturation: sat_of(t),
                    sends_db: sends_of(t, ids),
                    blocks: blocks_of(t, ids),
                    idea_sends_db: BTreeMap::new(),
                    rationale: note(&t.id),
                },
            );
        } else if let Some(fx) = &t.ret {
            let g = t.chain.fader_db;
            returns.insert(
                n,
                match fx {
                    ReturnFx::Reverb {
                        algorithm,
                        decay_s,
                        size,
                        damping,
                        modulation,
                        predelay_ms,
                        hpf_hz,
                        lpf_hz,
                        shimmer_semitones,
                        shimmer_amount,
                        ..
                    } => ReturnPlan::Reverb {
                        shimmer_semitones: Some(*shimmer_semitones),
                        shimmer_amount: Some(*shimmer_amount),
                        blocks: blocks_of(t, ids),
                        algorithm: *algorithm,
                        decay_s: *decay_s,
                        size: *size,
                        damping: *damping,
                        modulation: *modulation,
                        predelay_ms: *predelay_ms,
                        return_hpf_hz: *hpf_hz,
                        return_lpf_hz: *lpf_hz,
                        return_gain_db: g,
                    },
                    ReturnFx::Delay {
                        time_ms,
                        feedback,
                        feedback_hpf_hz,
                        feedback_lpf_hz,
                        mode,
                        character,
                        wow,
                        drive_db,
                        ..
                    } => ReturnPlan::Delay {
                        mode: Some(*mode),
                        character: Some(*character),
                        wow: Some(*wow),
                        drive_db: Some(*drive_db),
                        blocks: blocks_of(t, ids),
                        time_ms: *time_ms,
                        feedback: *feedback,
                        feedback_hpf_hz: *feedback_hpf_hz,
                        feedback_lpf_hz: *feedback_lpf_hz,
                        return_gain_db: g,
                    },
                },
            );
        }
    }
    MixPlan {
        version: MIX_PLAN_VERSION,
        iteration: 0,
        summary: p.ai.summary.clone().unwrap_or_default(),
        structure: None,
        ideas: None,
        returns,
        tracks,
        groups,
    }
}

/// Options of an application.
#[derive(Debug, Clone, Default)]
pub struct ApplyOptions {
    /// Input trims (dB) per project track id, towards −23 LUFS (AI-7); empty to leave trims alone.
    pub trims: HashMap<String, f64>,
    /// Tracks the request may touch (AI-5); `None` = all unlocked ones.
    pub scope: Option<HashSet<String>>,
    /// Tracks applied partially (AI-2 "apply selected"): `None` = all.
    pub only: Option<HashSet<String>>,
}

fn set_eq(t: &mut Track, eq: &[EqBand]) {
    let defaults = stemwise_core::model::Equalizer::track_default();
    if eq.is_empty() && t.chain.eq().is_none() {
        return;
    }
    let e = t.chain.eq_mut();
    e.bypass = false;
    e.bands = (0..4)
        .map(|i| match eq.get(i) {
            Some(b) => {
                CoreBand { enabled: true, kind: eq_kind(b.kind), freq_hz: b.freq_hz, gain_db: b.gain_db, q: b.q }
            }
            None => defaults.bands[i],
        })
        .collect();
}

/// The compressor of a plan. A sidechain source that is not in the project (a new group left out of a partial
/// application, a track removed during the review) is dropped: the compressor listens to its own track.
fn set_comp(p: &Project, t: &mut Track, c: &Option<CompressorPlan>, ids: &Ids) {
    match c {
        None => {
            if let Some(stemwise_core::fx::Fx::Compressor(c)) =
                t.chain.first_mut(stemwise_core::fx::FxKind::Compressor).map(|b| &mut b.fx)
            {
                c.bypass = true;
            }
        }
        Some(c) => {
            let hpf = t.chain.comp().and_then(|c| c.sidechain.as_ref()).map_or(100.0, |s| s.hpf_hz);
            *t.chain.comp_mut() = Compressor {
                bypass: false,
                threshold_db: c.threshold_db,
                ratio: c.ratio,
                attack_ms: c.attack_ms,
                release_ms: c.release_ms,
                knee_db: c.knee_db,
                makeup_db: c.makeup_db,
                mix: c.mix,
                sidechain: c
                    .sidechain
                    .as_ref()
                    .and_then(|s| ids.to_project.get(s))
                    .filter(|s| p.track(s).is_some())
                    .map(|s| Sidechain { source: s.clone(), hpf_hz: hpf }),
            };
        }
    }
}

fn set_sat(t: &mut Track, s: &Option<SaturationPlan>) {
    match s {
        None => {
            if let Some(stemwise_core::fx::Fx::Saturation(s)) =
                t.chain.first_mut(stemwise_core::fx::FxKind::Saturation).map(|b| &mut b.fx)
            {
                s.bypass = true;
            }
        }
        Some(s) => {
            let character = s.character.unwrap_or_else(|| t.chain.sat().map(|x| x.character).unwrap_or_default());
            *t.chain.sat_mut() = Saturation { bypass: false, drive_db: s.drive_db, mix: s.mix, character };
        }
    }
}

/// Sends of a plan; a send to a return that is not in the project (a new one left out of a partial application,
/// AI-20) is dropped.
fn set_sends(p: &Project, t: &mut Track, sends: &BTreeMap<String, f64>, ids: &Ids) {
    t.sends = sends
        .iter()
        .filter_map(|(r, db)| ids.to_project.get(r).map(|id| Send { target: id.clone(), level_db: *db }))
        .filter(|s| p.track(&s.target).is_some())
        .collect();
}

fn lane(t: &mut Track, param: String, points: Vec<stemwise_core::model::AutomationPoint>) {
    t.automation.retain(|l| l.param != param);
    if !points.is_empty() {
        t.automation.push(AutomationLane { param, points, author: Author::Ai, visible: true });
    }
}

fn has_offset(v: &[f64]) -> bool {
    v.iter().any(|x| x.abs() > 1e-9)
}

/// Per-section offsets as automation lanes (AUT-5). The lanes the assistant wrote before are replaced; a parameter
/// without any offset gets no lane, so flat lanes never cover the author's static values.
fn write_sections(p: &Project, t: &mut Track, base: &TrackPlan, ids: &Ids, made: &HashMap<String, String>) {
    t.automation.retain(|l| l.author != Author::Ai);
    if p.sections.is_empty() || base.sections.is_empty() {
        return;
    }
    let rate = f64::from(p.sample_rate.unwrap_or(48000));
    let ramp = (SECTION_RAMP_S * rate) as i64;
    let mut secs: Vec<_> = p.sections.iter().collect();
    secs.sort_by_key(|s| s.start);
    let sec_plan = |s: &stemwise_core::model::Section| base.sections.get(&ids.n(&s.id));
    let gains: Vec<f64> = secs.iter().map(|s| sec_plan(s).map_or(0.0, |x| x.gain_offset_db)).collect();
    let sections: Vec<_> = secs.iter().map(|s| (*s).clone()).collect();
    if has_offset(&gains) {
        lane(t, "fader".into(), clamp_points(section_points(&sections, base.gain_db, &gains, ramp), -144.0, 12.0));
    }
    for (r, level) in &base.sends_db {
        let offs: Vec<f64> =
            secs.iter().map(|s| sec_plan(s).and_then(|x| x.send_offsets_db.get(r).copied()).unwrap_or(0.0)).collect();
        if let Some(rid) = ids.to_project.get(r)
            && p.track(rid).is_some()
            && has_offset(&offs)
        {
            lane(t, format!("send.{rid}"), clamp_points(section_points(&sections, *level, &offs, ramp), -144.0, 6.0));
        }
    }
    let eq = eq_id(t);
    for (i, b) in base.eq.iter().enumerate().take(4) {
        let offs: Vec<f64> = secs
            .iter()
            .map(|s| sec_plan(s).and_then(|x| x.eq_gain_offsets_db.get(i).copied()).unwrap_or(0.0))
            .collect();
        if has_offset(&offs) {
            lane(
                t,
                format!("{eq}.{}.gain", i + 1),
                clamp_points(section_points(&sections, b.gain_db, &offs, ramp), -18.0, 18.0),
            );
        }
    }
    // v4 (AI-6, FX-12): the filters move by octaves between sections, gliding at the boundaries like the rest.
    for (kind, lo, hi) in [(FxKind::Hpf, 20.0, 2000.0), (FxKind::Lpf, 1000.0, 20000.0)] {
        let octs: Vec<f64> = secs
            .iter()
            .map(|s| {
                sec_plan(s)
                    .and_then(|x| if kind == FxKind::Hpf { x.hpf_offset_oct } else { x.lpf_offset_oct })
                    .unwrap_or(0.0)
            })
            .collect();
        // The filter that plays: the first one on (a filter an idea adds, the classic one switched off), else the
        // first.
        let Some(b) = playing_filter(&t.chain, kind) else { continue };
        let (Fx::Hpf(f) | Fx::Lpf(f)) = &b.fx else { continue };
        if has_offset(&octs) {
            let base_hz = f.freq_hz;
            let offs: Vec<f64> = octs.iter().map(|o| base_hz * (2f64.powf(*o) - 1.0)).collect();
            let id = b.id.clone();
            lane(t, format!("{id}.freq"), clamp_points(section_points(&sections, base_hz, &offs, ramp), lo, hi));
        }
    }
    // v4 (AI-6): blocks on only in some sections: a bypass lane in steps, off outside the sections that list them
    // (the bypass glides over 10 ms, FX-10).
    let mut on_ids: Vec<&String> = base.sections.values().flat_map(|x| x.blocks_on.iter()).collect();
    on_ids.sort();
    on_ids.dedup();
    for pid in on_ids {
        let Some(real) = made
            .get(pid)
            .cloned()
            .or_else(|| ids.blocks.get(pid).filter(|(tr, _)| *tr == t.id).map(|(_, b)| b.clone()))
        else {
            continue;
        };
        if t.chain.block(&real).is_none() {
            continue;
        }
        let mut pts = vec![stemwise_core::model::AutomationPoint { frame: 0, value: 1.0 }];
        for s in &secs {
            let on = sec_plan(s).is_some_and(|x| x.blocks_on.contains(pid));
            pts.push(stemwise_core::model::AutomationPoint { frame: s.start, value: if on { 0.0 } else { 1.0 } });
            pts.push(stemwise_core::model::AutomationPoint { frame: s.end, value: 1.0 });
        }
        // Steps: one point per change, in order (a later point at the same frame wins).
        let mut clean: Vec<stemwise_core::model::AutomationPoint> = Vec::new();
        for x in pts {
            if let Some(last) = clean.last_mut()
                && last.frame == x.frame
            {
                *last = x;
                continue;
            }
            if clean.last().is_none_or(|l| l.value != x.value) {
                clean.push(x);
            }
        }
        if let Some(b) = t.chain.block_mut(&real) {
            b.fx.set_bypass(false);
        }
        lane(t, format!("bypass.{real}"), clean);
    }
}

fn clamp_points(
    mut p: Vec<stemwise_core::model::AutomationPoint>,
    lo: f64,
    hi: f64,
) -> Vec<stemwise_core::model::AutomationPoint> {
    for x in &mut p {
        x.value = x.value.clamp(lo, hi);
    }
    p
}

/// The filter whose frequency moves between sections (AI-6, A-089): the first one on (a filter an idea added, the
/// classic one switched off), else the first. Written and read back the same way.
fn playing_filter(c: &stemwise_core::model::Chain, kind: FxKind) -> Option<&stemwise_core::fx::FxBlock> {
    c.blocks.iter().find(|b| b.fx.kind() == kind && !b.fx.bypass()).or_else(|| c.first(kind))
}

/// The commands of a mix proposal, numbered notes included (HIS-3). The plan must be validated first.
pub fn apply_mix_plan(p: &Project, plan: &MixPlan, ids: &Ids, opt: &ApplyOptions) -> Vec<Command> {
    let mut cmds = Vec::new();
    let allowed = |id: &str| -> bool {
        let t = p.track(id);
        !t.is_some_and(|t| t.locked)
            && opt.scope.as_ref().is_none_or(|s| s.contains(id))
            && opt.only.as_ref().is_none_or(|s| s.contains(id))
    };
    let mut notes: Vec<AiNote> = Vec::new();
    for t in &p.tracks {
        let n = ids.n(&t.id);
        let mut new = t.clone();
        if t.is_audio() {
            let Some(tp) = plan.tracks.get(&n) else { continue };
            if !allowed(&t.id) {
                continue;
            }
            new.chain.fader_db = tp.gain_db;
            new.chain.pan = tp.pan;
            for (kind, hz) in [(FxKind::Hpf, tp.hpf_hz), (FxKind::Lpf, tp.lpf_hz)] {
                match hz {
                    Some(f) => {
                        if let Fx::Hpf(x) | Fx::Lpf(x) = new.chain.ensure(kind) {
                            x.bypass = false;
                            x.freq_hz = f;
                        }
                    }
                    None => {
                        if let Some(b) = new.chain.first_mut(kind) {
                            b.fx.set_bypass(true);
                        }
                    }
                }
            }
            for (kind, q) in [(FxKind::Hpf, tp.hpf_resonance), (FxKind::Lpf, tp.lpf_resonance)] {
                if let (Some(q), Some(b)) = (q, new.chain.first_mut(kind))
                    && let Fx::Hpf(x) | Fx::Lpf(x) = &mut b.fx
                {
                    x.resonance = q.clamp(0.5, 10.0);
                }
            }
            set_eq(&mut new, &tp.eq);
            set_comp(p, &mut new, &tp.compressor, ids);
            set_sat(&mut new, &tp.saturation);
            set_sends(p, &mut new, &tp.sends_db, ids);
            let made = apply_blocks(p, &mut new, &tp.blocks, ids);
            if let Some(trim) = opt.trims.get(&t.id) {
                new.chain.trim_db = trim.clamp(-36.0, 36.0);
            }
            write_sections(p, &mut new, tp, ids, &made);
            if !tp.rationale.is_empty() {
                notes.push(AiNote { number: 0, track: Some(t.id.clone()), text: tp.rationale.clone() });
            }
        } else if t.is_group() {
            let Some(gp) = plan.groups.get(&n) else { continue };
            if !allowed(&t.id) {
                continue;
            }
            new.chain.fader_db = gp.gain_db;
            set_eq(&mut new, &gp.eq);
            set_comp(p, &mut new, &gp.compressor, ids);
            set_sat(&mut new, &gp.saturation);
            set_sends(p, &mut new, &gp.sends_db, ids);
            apply_blocks(p, &mut new, &gp.blocks, ids);
            if !gp.rationale.is_empty() {
                notes.push(AiNote { number: 0, track: Some(t.id.clone()), text: gp.rationale.clone() });
            }
        } else {
            let Some(rp) = plan.returns.get(&n) else { continue };
            // Returns are shared by every track: a request limited to some tracks leaves them alone (A-034).
            if opt.only.as_ref().is_some_and(|s| !s.contains(&t.id)) || t.locked || opt.scope.is_some() {
                continue;
            }
            let (bypass, sync, old_time) = match &t.ret {
                Some(ReturnFx::Reverb { bypass, .. }) => (*bypass, None, 0.0),
                Some(ReturnFx::Delay { bypass, sync, time_ms, .. }) => (*bypass, sync.clone(), *time_ms),
                None => (false, None, 0.0),
            };
            match (rp, &mut new.ret) {
                (
                    ReturnPlan::Reverb {
                        algorithm,
                        decay_s,
                        size,
                        damping,
                        modulation,
                        predelay_ms,
                        return_hpf_hz,
                        return_lpf_hz,
                        return_gain_db,
                        shimmer_semitones: plan_semis,
                        shimmer_amount: plan_amount,
                        blocks,
                    },
                    Some(ReturnFx::Reverb { .. }),
                ) => {
                    let (mut shimmer_semitones, mut shimmer_amount) = match &t.ret {
                        Some(ReturnFx::Reverb { shimmer_semitones, shimmer_amount, .. }) => {
                            (*shimmer_semitones, *shimmer_amount)
                        }
                        _ => (12.0, 0.0),
                    };
                    if let Some(v) = plan_semis.filter(|v| [5.0, 7.0, 12.0].contains(v)) {
                        shimmer_semitones = v;
                    }
                    if let Some(v) = plan_amount {
                        shimmer_amount = v.clamp(0.0, 1.0);
                    }
                    apply_blocks(p, &mut new, blocks, ids);
                    new.ret = Some(ReturnFx::Reverb {
                        shimmer_semitones,
                        shimmer_amount,
                        bypass,
                        algorithm: *algorithm,
                        decay_s: *decay_s,
                        size: *size,
                        damping: *damping,
                        modulation: *modulation,
                        predelay_ms: *predelay_ms,
                        hpf_hz: *return_hpf_hz,
                        lpf_hz: *return_lpf_hz,
                    });
                    new.chain.fader_db = *return_gain_db;
                }
                (
                    ReturnPlan::Delay {
                        time_ms,
                        feedback,
                        feedback_hpf_hz,
                        feedback_lpf_hz,
                        return_gain_db,
                        mode: plan_mode,
                        character: plan_character,
                        wow: plan_wow,
                        drive_db: plan_drive,
                        blocks,
                    },
                    Some(ReturnFx::Delay { .. }),
                ) => {
                    // A musical time stays synced while the time is unchanged.
                    let (mut mode, mut character, mut wow, mut drive_db) = match &t.ret {
                        Some(ReturnFx::Delay { mode, character, wow, drive_db, .. }) => {
                            (*mode, *character, *wow, *drive_db)
                        }
                        _ => Default::default(),
                    };
                    mode = plan_mode.unwrap_or(mode);
                    character = plan_character.unwrap_or(character);
                    wow = plan_wow.map_or(wow, |v| v.clamp(0.0, 1.0));
                    drive_db = plan_drive.map_or(drive_db, |v| v.clamp(0.0, 12.0));
                    apply_blocks(p, &mut new, blocks, ids);
                    new.ret = Some(ReturnFx::Delay {
                        mode,
                        character,
                        wow,
                        drive_db,
                        bypass,
                        time_ms: *time_ms,
                        sync: if (time_ms - old_time).abs() < 0.5 { sync } else { None },
                        feedback: *feedback,
                        feedback_hpf_hz: *feedback_hpf_hz,
                        feedback_lpf_hz: *feedback_lpf_hz,
                    });
                    new.chain.fader_db = *return_gain_db;
                }
                _ => continue,
            }
        }
        if &new != t {
            cmds.push(Command::SetTrack { track: Box::new(new) });
        }
    }
    // Notes are numbered in the order the plan returns them (design: numbered notes).
    let order: Vec<String> = plan.tracks.keys().chain(plan.groups.keys()).cloned().collect();
    notes.sort_by_key(|n| {
        order.iter().position(|o| Some(o) == n.track.as_ref().map(|t| ids.n(t)).as_ref()).unwrap_or(usize::MAX)
    });
    for (i, n) in notes.iter_mut().enumerate() {
        n.number = i as u32 + 1;
    }
    let summary = (!plan.summary.is_empty()).then(|| plan.summary.clone());
    if let Some(only) = &opt.only {
        // A partial application keeps the notes of the applied tracks only (numbers unchanged).
        notes.retain(|n| n.track.as_ref().is_some_and(|t| only.contains(t)));
    }
    cmds.push(Command::SetAiNotes { notes, summary });
    cmds
}

/// One row of a proposal's diff (design 4a "diff row").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DiffRow {
    pub label: String,
    pub unit: String,
    pub before: Option<f64>,
    pub after: Option<f64>,
    /// A block the proposal adds (AI-25): what it is set to, in words, instead of a number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub text: Option<String>,
}

/// A track's changes in a proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct TrackDiff {
    pub track: String,
    pub name: String,
    pub note: Option<u32>,
    pub rationale: String,
    pub rows: Vec<DiffRow>,
}

fn row(rows: &mut Vec<DiffRow>, label: &str, unit: &str, before: Option<f64>, after: Option<f64>) {
    let same = match (before, after) {
        (Some(a), Some(b)) => (a - b).abs() < 0.05,
        (None, None) => true,
        _ => false,
    };
    if !same {
        rows.push(DiffRow { label: label.into(), unit: unit.into(), before, after, text: None });
    }
}

/// What a new block is set to, in a few words (the diff row of AI-25).
fn block_words(fx: &Fx) -> String {
    let hz = |f: f64| if f >= 1000.0 { format!("{:.1} kHz", f / 1000.0) } else { format!("{f:.0} Hz") };
    let mut w = match fx {
        Fx::Deesser(d) => hz(d.freq_hz),
        Fx::DynamicEq(d) => d.bands.iter().filter(|b| b.enabled).map(|b| hz(b.freq_hz)).collect::<Vec<_>>().join(" · "),
        Fx::Gate(g) => format!("{:.0} dB range", g.range_db),
        Fx::Transient(t) => format!("{:+.1} dB attack", t.attack_db),
        Fx::Width(w) => format!("{:.0} %", w.width * 100.0),
        Fx::Modulation(m) => format!("{:?}", m.mode).to_lowercase(),
        Fx::Tremolo(t) => format!("{:?}", t.mode).to_lowercase(),
        Fx::Harmonizer(h) => h
            .voices
            .iter()
            .filter(|v| v.enabled)
            .map(|v| format!("{:+.0} st", v.semitones))
            .collect::<Vec<_>>()
            .join(" "),
        Fx::Doubler(d) => format!("{:.0} ct", d.detune_cents),
        Fx::Exciter(e) => hz(e.freq_hz),
        Fx::BassEnhancer(b) => hz(b.freq_hz),
        Fx::Compressor(c) => format!("{:.1}:1", c.ratio),
        Fx::Clipper(c) => format!("{:.1} dBFS", c.threshold_db),
        Fx::Multiband(m) => format!("{} bands", m.bands.len()),
        Fx::Saturation(s) => format!("{:.1} dB", s.drive_db),
        Fx::Hpf(f) | Fx::Lpf(f) => hz(f.freq_hz),
        Fx::Eq(e) => format!("{} bands", e.bands.iter().filter(|b| b.enabled).count()),
    };
    if fx.bypass() {
        w.push_str(" · off");
    }
    w
}

/// Parameter differences between the project before and after the proposal's commands (AI-2).
pub fn diff(before: &Project, after: &Project) -> Vec<TrackDiff> {
    let mut out = Vec::new();
    for b in &before.tracks {
        let Some(a) = after.track(&b.id) else { continue };
        let mut rows = Vec::new();
        let (cb, ca) = (&b.chain, &a.chain);
        row(&mut rows, "Trim", "dB", Some(cb.trim_db), Some(ca.trim_db));
        row(
            &mut rows,
            if b.is_return() { "Return level" } else { "Fader" },
            "dB",
            Some(cb.fader_db),
            Some(ca.fader_db),
        );
        row(&mut rows, "Pan", "", Some(cb.pan), Some(ca.pan));
        row(
            &mut rows,
            "Low cut",
            "Hz",
            cb.hpf().filter(|f| !f.bypass).map(|f| f.freq_hz),
            ca.hpf().filter(|f| !f.bypass).map(|f| f.freq_hz),
        );
        row(
            &mut rows,
            "High cut",
            "Hz",
            cb.lpf().filter(|f| !f.bypass).map(|f| f.freq_hz),
            ca.lpf().filter(|f| !f.bypass).map(|f| f.freq_hz),
        );
        for i in 0..4 {
            let g = |c: &stemwise_core::model::Chain| {
                c.eq().filter(|e| !e.bypass).and_then(|e| e.bands.get(i)).filter(|x| x.enabled).copied()
            };
            let (x, y) = (g(cb), g(ca));
            let f = y.or(x).map_or(0.0, |v| v.freq_hz);
            let label = format!(
                "EQ {} · {}",
                i + 1,
                if f >= 1000.0 { format!("{:.1} kHz", f / 1000.0) } else { format!("{f:.0} Hz") }
            );
            row(&mut rows, &label, "dB", x.map(|v| v.gain_db), y.map(|v| v.gain_db));
        }
        let comp = |c: &stemwise_core::model::Chain| c.comp().filter(|x| !x.bypass).cloned();
        let (x, y) = (comp(cb), comp(ca));
        row(&mut rows, "Comp threshold", "dB", x.as_ref().map(|c| c.threshold_db), y.as_ref().map(|c| c.threshold_db));
        row(&mut rows, "Comp ratio", ":1", x.as_ref().map(|c| c.ratio), y.as_ref().map(|c| c.ratio));
        row(&mut rows, "Comp makeup", "dB", x.as_ref().map(|c| c.makeup_db), y.as_ref().map(|c| c.makeup_db));
        let sat = |c: &stemwise_core::model::Chain| c.sat().filter(|x| !x.bypass).map(|x| x.drive_db);
        row(&mut rows, "Saturation drive", "dB", sat(cb), sat(ca));
        let targets: Vec<&String> = b
            .sends
            .iter()
            .chain(&a.sends)
            .map(|s| &s.target)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        for r in targets {
            let name = after.track(r).map_or(r.as_str(), |t| t.name.as_str());
            row(&mut rows, &format!("Send · {name}"), "dB", b.send_db(r), a.send_db(r));
        }
        if let (
            Some(ReturnFx::Reverb { decay_s: dx, size: x, .. }),
            Some(ReturnFx::Reverb { decay_s: dy, size: y, .. }),
        ) = (&b.ret, &a.ret)
        {
            row(&mut rows, "Decay", "s", Some(*dx), Some(*dy));
            row(&mut rows, "Size", "%", Some(x * 100.0), Some(y * 100.0));
        }
        if let (
            Some(ReturnFx::Delay { time_ms: x, feedback: fx, .. }),
            Some(ReturnFx::Delay { time_ms: y, feedback: fy, .. }),
        ) = (&b.ret, &a.ret)
        {
            row(&mut rows, "Time", "ms", Some(*x), Some(*y));
            row(&mut rows, "Feedback", "%", Some(fx * 100.0), Some(fy * 100.0));
        }
        // AI-25: a block the proposal adds is a row of its own, marked new.
        for blk in &a.chain.blocks {
            if cb.block(&blk.id).is_none_or(|x| x.fx.kind() != blk.fx.kind()) {
                rows.push(DiffRow {
                    label: format!("{} · new block", ca.name(&blk.id).unwrap_or_else(|| blk.fx.kind().label().into())),
                    unit: String::new(),
                    before: None,
                    after: None,
                    text: Some(block_words(&blk.fx)),
                });
            }
        }
        let auto_b = b.automation.iter().filter(|l| l.author == Author::Ai).count();
        let auto_a = a.automation.iter().filter(|l| l.author == Author::Ai).count();
        if auto_a > 0 || auto_b > 0 {
            row(&mut rows, "Section lanes", "", Some(auto_b as f64), Some(auto_a as f64));
        }
        let note = after.ai.notes.iter().find(|n| n.track.as_deref() == Some(&b.id));
        if !rows.is_empty() || note.is_some() {
            out.push(TrackDiff {
                track: b.id.clone(),
                name: b.name.clone(),
                note: note.map(|n| n.number),
                rationale: note.map(|n| n.text.clone()).unwrap_or_default(),
                rows,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::SectionPlan;
    use stemwise_core::history::{Document, Phase};
    use stemwise_core::model::Section;

    fn project() -> Project {
        let mut p = Project::new("x");
        p.sample_rate = Some(48000);
        for (id, name) in [("t1", "Kick"), ("t2", "Bass"), ("t3", "Vox")] {
            p.tracks.push(Track::new_audio(id.into(), name, None, 1, 1));
        }
        p.tracks.push(Track::new_group("g1".into(), "Drums"));
        p.sections = vec![
            Section { id: "sa".into(), name: "Verse".into(), color: 1, start: 0, end: 480_000 },
            Section { id: "sb".into(), name: "Chorus".into(), color: 2, start: 480_000, end: 960_000 },
        ];
        p
    }

    /// req: AI-4, AI-6, AI-7, AUT-5, HIS-3
    #[test]
    fn a_plan_becomes_one_transaction_with_lanes_trims_notes_and_respects_locks() {
        let mut p = project();
        p.track_mut("t2").unwrap().locked = true;
        let ids = Ids::of(&p);
        let mut plan = plan_from_project(&p, &ids);
        let vox = ids.n("t3");
        let tp = plan.tracks.get_mut(&vox).unwrap();
        tp.gain_db = 2.0;
        tp.eq = vec![EqBand { kind: EqType::Peak, freq_hz: 300.0, gain_db: -2.5, q: 1.4 }];
        tp.sends_db.insert("r1".into(), -14.0);
        tp.sections.insert(
            "s2".into(),
            SectionPlan {
                gain_offset_db: 1.5,
                send_offsets_db: [("r1".to_string(), -3.0)].into(),
                eq_gain_offsets_db: vec![1.0],
                ..Default::default()
            },
        );
        tp.rationale = "Forward.".into();
        plan.tracks.get_mut(&ids.n("t2")).unwrap().gain_db = -10.0;
        plan.tracks.get_mut(&ids.n("t1")).unwrap().rationale = "Kick.".into();
        let opt =
            ApplyOptions { trims: [("t3".to_string(), 4.5), ("t2".to_string(), 9.0)].into(), ..Default::default() };
        let cmds = apply_mix_plan(&p, &plan, &ids, &opt);
        let mut d = Document::new(p.clone());
        d.transact(Phase::Mix, "Apply auto mix", cmds, Some("call-1".into())).unwrap();
        let v = d.project.track("t3").unwrap();
        assert_eq!((v.chain.fader_db, v.chain.trim_db), (2.0, 4.5));
        // Chorus: +1.5 dB on the fader lane, −3 dB on the send lane, +1 dB on EQ band 1, with 80 ms ramps.
        let at = |param: &str, f: i64| {
            stemwise_core::automation::value_at(&v.lane(param).unwrap().points, f, false).unwrap()
        };
        assert_eq!(at("fader", 100_000), 2.0);
        assert_eq!(at("fader", 700_000), 3.5);
        assert_eq!(at("send.r1", 700_000), -17.0);
        assert_eq!(at("eq.1.gain", 700_000), -1.5);
        assert!(v.lane("fader").unwrap().author == Author::Ai);
        // The locked bass is untouched, its trim too (AI-4).
        assert_eq!(d.project.track("t2").unwrap(), p.track("t2").unwrap());
        // Notes numbered in plan order; one undo removes everything (HIS-3).
        assert_eq!(
            d.project.ai.notes.iter().map(|n| (n.number, n.track.clone().unwrap())).collect::<Vec<_>>(),
            vec![(1, "t1".into()), (2, "t3".into())]
        );
        let rows = diff(&p, &d.project);
        let vox_rows = &rows.iter().find(|r| r.track == "t3").unwrap().rows;
        assert!(vox_rows.iter().any(|r| r.label == "Fader" && r.before == Some(0.0) && r.after == Some(2.0)));
        assert!(vox_rows.iter().any(|r| r.label.starts_with("EQ 1") && r.after == Some(-2.5)));
        d.undo(Phase::Mix).unwrap();
        assert_eq!(d.project, p);
    }

    /// req: AI-5, AI-2
    #[test]
    fn scope_and_partial_application_limit_the_touched_tracks() {
        let p = project();
        let ids = Ids::of(&p);
        let mut plan = plan_from_project(&p, &ids);
        for t in plan.tracks.values_mut() {
            t.gain_db = -3.0;
        }
        let only: HashSet<String> = ["t1".to_string()].into();
        let cmds = apply_mix_plan(&p, &plan, &ids, &ApplyOptions { scope: Some(only.clone()), ..Default::default() });
        let changed: Vec<_> = cmds
            .iter()
            .filter_map(|c| if let Command::SetTrack { track } = c { Some(track.id.clone()) } else { None })
            .collect();
        assert_eq!(changed, vec!["t1".to_string()]);
        let cmds = apply_mix_plan(
            &p,
            &plan,
            &ids,
            &ApplyOptions { only: Some(["t2".to_string(), "t3".to_string()].into()), ..Default::default() },
        );
        assert_eq!(cmds.iter().filter(|c| matches!(c, Command::SetTrack { .. })).count(), 2);
    }

    /// req: AI-3, AI-6, AUT-5
    #[test]
    fn per_section_decisions_survive_a_revision_that_does_not_touch_them() {
        // B1 of the F5 review: the auto mix lifts the vocal in the chorus; a later revision starts from the current
        // state, so the lift is in the starting plan and an echo of it keeps the lane.
        let p = project();
        let ids = Ids::of(&p);
        let mut plan = plan_from_project(&p, &ids);
        let vox = ids.n("t3");
        let tp = plan.tracks.get_mut(&vox).unwrap();
        tp.gain_db = -2.0;
        tp.eq = vec![EqBand { kind: EqType::Peak, freq_hz: 3000.0, gain_db: 1.0, q: 1.0 }];
        tp.sends_db.insert("r1".into(), -14.0);
        tp.sections.insert(
            "s2".into(),
            SectionPlan {
                gain_offset_db: 1.5,
                send_offsets_db: BTreeMap::new(),
                eq_gain_offsets_db: vec![0.5],
                ..Default::default()
            },
        );
        let mut d = Document::new(p.clone());
        d.transact(Phase::Mix, "auto mix", apply_mix_plan(&p, &plan, &ids, &ApplyOptions::default()), None).unwrap();
        let v = d.project.track("t3").unwrap();
        assert!(v.lane("fader").is_some() && v.lane("eq.1.gain").is_some());
        assert!(v.lane("send.r1").is_none(), "no offset on the send: no flat lane covering the static level");
        // The revision's starting plan carries the offsets back.
        let again = plan_from_project(&d.project, &ids);
        let sp = &again.tracks[&vox].sections;
        assert_eq!(sp.len(), 1, "{sp:?}");
        assert_eq!((sp["s2"].gain_offset_db, sp["s2"].eq_gain_offsets_db.clone()), (1.5, vec![0.5]));
        // Echoing it (a revision about something else) leaves the lanes as they were.
        let cmds = apply_mix_plan(&d.project, &again, &ids, &ApplyOptions::default());
        let before = d.project.track("t3").unwrap().clone();
        d.transact(Phase::Mix, "revision", cmds, None).unwrap();
        let after = d.project.track("t3").unwrap();
        assert_eq!(after.automation, before.automation);
        assert_eq!(after.chain, before.chain);
    }

    /// req: AI-3
    #[test]
    fn manual_values_outside_the_assistant_limits_reach_the_plan_unchanged() {
        let mut p = project();
        p.track_mut("t1").unwrap().chain.fader_db = -40.0;
        let ids = Ids::of(&p);
        let plan = plan_from_project(&p, &ids);
        assert_eq!(plan.tracks[&ids.n("t1")].gain_db, -40.0);
        // Applied back, nothing moves (no +16 dB jump on a track the request did not name).
        let cmds = apply_mix_plan(&p, &plan, &ids, &ApplyOptions::default());
        assert!(cmds.iter().all(|c| !matches!(c, Command::SetTrack { track } if track.id == "t1")));
    }
}
