//! Strict validation of plans (AI-8, AI-4): schema (parsing), limits, ids, routing; change limits between
//! iterations (DEC-5 of the PoC) and lock enforcement. Errors are written to be sent back to the model.

use std::collections::{BTreeMap, HashMap};

use stemwise_core::model::Project;

use crate::analysis_doc::Ids;
use crate::apply::{ApplyOptions, apply_mix_plan};
use crate::plan::{
    Editable, EqBand, MASTER_LIMITS, MAX_MASTER_EQ, MAX_TRACK_EQ, MIX_PLAN_VERSION, MasterPlan, MixPlan, ReturnPlan,
    limit,
};

/// Largest change of `gain_db` between iterations, and of each EQ band's gain (DEC-5).
pub const MAX_GAIN_CHANGE_DB: f64 = 4.0;
pub const MAX_EQ_CHANGE_DB: f64 = 3.0;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    /// Values of the starting plan by path: a value the plan keeps unchanged is accepted even outside the limits,
    /// because the author set it by hand (B2 of the F5 review; limits apply to what the assistant changes).
    kept: HashMap<String, f64>,
}

/// Numeric fields of a plan by path (`tracks.t1.eq[0].gain_db`), the same paths the errors use.
fn flatten(v: &serde_json::Value, prefix: &str, out: &mut HashMap<String, f64>) {
    match v {
        serde_json::Value::Object(m) => {
            for (k, x) in m {
                let p = if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") };
                flatten(x, &p, out);
            }
        }
        serde_json::Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                flatten(x, &format!("{prefix}[{i}]"), out);
            }
        }
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                out.insert(prefix.to_string(), f);
            }
        }
        _ => {}
    }
}

impl Report {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// The message sent back to the model on a retry (AI-8).
    pub fn feedback(&self) -> String {
        format!(
            "The plan is invalid. Fix these problems and return the complete plan again:\n- {}",
            self.errors.join("\n- ")
        )
    }
}

fn check(r: &mut Report, path_pattern: &str, at: &str, v: f64) {
    if r.kept.get(at).is_some_and(|k| (k - v).abs() < 1e-9) {
        return;
    }
    let l = limit(path_pattern);
    if !v.is_finite() || v < l.min || v > l.max {
        r.errors.push(format!("{at} = {v} is outside {} … {} {}", l.min, l.max, l.unit));
    }
}

fn check_eq(r: &mut Report, at: &str, eq: &[EqBand], nyquist: f64, max_bands: usize, master: bool) {
    if eq.len() > max_bands {
        r.errors.push(format!("{at}.eq has {} bands; at most {max_bands}", eq.len()));
    }
    for (i, b) in eq.iter().enumerate() {
        check(r, "eq[].freq_hz", &format!("{at}.eq[{i}].freq_hz"), b.freq_hz);
        if b.freq_hz >= nyquist {
            r.errors.push(format!("{at}.eq[{i}].freq_hz must be below {nyquist} Hz"));
        }
        if master {
            let l = MASTER_LIMITS.iter().find(|l| l.path == "eq[].gain_db").expect("master EQ limit");
            if !b.gain_db.is_finite() || b.gain_db < l.min || b.gain_db > l.max {
                r.errors.push(format!("{at}.eq[{i}].gain_db = {} is outside {} … {} dB", b.gain_db, l.min, l.max));
            }
        } else {
            check(r, "eq[].gain_db", &format!("{at}.eq[{i}].gain_db"), b.gain_db);
        }
        check(r, "eq[].q", &format!("{at}.eq[{i}].q"), b.q);
    }
}

/// Validates a mix plan against the project (ids are neutral). Locked and out-of-scope tracks, groups and returns
/// may be omitted; if present they are dropped with a warning (their values are restored: AI-4, AI-5). With a
/// starting plan (a revision, a later iteration), values kept unchanged are accepted even outside the limits.
pub fn validate_mix(plan: &mut MixPlan, p: &Project, ids: &Ids, ed: &Editable, start: Option<&MixPlan>) -> Report {
    let mut r = Report::default();
    if let Some(sp) = start {
        flatten(&serde_json::to_value(sp).unwrap_or_default(), "", &mut r.kept);
    }
    let editable = |id: &str| {
        let n = ids.n(id);
        ed.tracks.contains(&n) || ed.groups.contains(&n)
    };
    let nyquist = f64::from(p.sample_rate.unwrap_or(48000)) / 2.0;
    if plan.version != MIX_PLAN_VERSION {
        r.errors.push(format!("version must be {MIX_PLAN_VERSION}"));
    }
    let returns: BTreeMap<String, bool> = p
        .tracks
        .iter()
        .filter(|t| t.is_return())
        .map(|t| (ids.n(&t.id), t.ret.as_ref().is_some_and(|x| x.is_reverb())))
        .collect();
    let sections: Vec<String> = p.sections.iter().map(|s| ids.n(&s.id)).collect();
    // Completeness and unknown ids.
    for t in p.tracks.iter().filter(|t| t.is_audio() || t.is_group()) {
        let n = ids.n(&t.id);
        let present = if t.is_audio() { plan.tracks.contains_key(&n) } else { plan.groups.contains_key(&n) };
        if editable(&t.id) && !present {
            r.errors.push(format!(
                "{} `{n}` is missing: return every unlocked track and group",
                if t.is_audio() { "track" } else { "group" }
            ));
        }
        if !editable(&t.id) && present {
            r.warnings.push(format!("`{n}` is locked or out of scope: its plan was ignored"));
            plan.tracks.remove(&n);
            plan.groups.remove(&n);
        }
    }
    let kind_of = |n: &str| ids.to_project.get(n).and_then(|id| p.track(id)).map(|t| t.kind);
    for n in plan.tracks.keys() {
        match kind_of(n) {
            None => r.errors.push(format!("unknown id `{n}`")),
            Some(stemwise_core::model::TrackKind::Audio) => {}
            Some(_) => r.errors.push(format!("`{n}` is not an audio track: put groups under `groups` only")),
        }
    }
    for n in plan.groups.keys() {
        match kind_of(n) {
            None => r.errors.push(format!("unknown id `{n}`")),
            Some(stemwise_core::model::TrackKind::Group) => {}
            Some(_) => r.errors.push(format!("`{n}` is not a group")),
        }
    }
    // Returns that are locked or outside the request are context only.
    let dropped: Vec<String> =
        plan.returns.keys().filter(|n| returns.contains_key(*n) && !ed.returns.contains(n)).cloned().collect();
    for n in dropped {
        r.warnings.push(format!("return `{n}` is locked or out of scope: its plan was ignored"));
        plan.returns.remove(&n);
    }
    // DEC-3 of the PoC: every proposal explains itself (AI-2).
    if plan.summary.trim().is_empty() {
        r.errors.push("summary is empty: describe the plan in one or two sentences".into());
    }
    for (n, why) in
        plan.tracks.iter().map(|(n, t)| (n, &t.rationale)).chain(plan.groups.iter().map(|(n, g)| (n, &g.rationale)))
    {
        if why.trim().is_empty() {
            r.errors.push(format!("`{n}` has an empty rationale: say in one or two sentences why"));
        }
    }
    for (n, rp) in &plan.returns {
        match (returns.get(n), rp) {
            (None, _) => r.errors.push(format!("unknown return `{n}`: configure only the existing returns")),
            (Some(true), ReturnPlan::Delay { .. }) | (Some(false), ReturnPlan::Reverb { .. }) => {
                r.errors.push(format!("return `{n}` has the wrong type"))
            }
            _ => {}
        }
        match rp {
            ReturnPlan::Reverb {
                decay_s,
                size,
                damping,
                modulation,
                predelay_ms,
                return_hpf_hz,
                return_lpf_hz,
                return_gain_db,
                shimmer_semitones,
                shimmer_amount,
                ..
            } => {
                if let Some(v) = shimmer_amount {
                    check(&mut r, "returns.<reverb>.shimmer_amount", &format!("returns.{n}.shimmer_amount"), *v);
                }
                if let Some(v) = shimmer_semitones
                    && ![5.0, 7.0, 12.0].contains(v)
                {
                    r.errors.push(format!("returns.{n}.shimmer_semitones must be 5, 7 or 12"));
                }
                check(&mut r, "returns.<reverb>.decay_s", &format!("returns.{n}.decay_s"), *decay_s);
                check(&mut r, "returns.<reverb>.size", &format!("returns.{n}.size"), *size);
                check(&mut r, "returns.<reverb>.modulation", &format!("returns.{n}.modulation"), *modulation);
                check(&mut r, "returns.<reverb>.return_lpf_hz", &format!("returns.{n}.return_lpf_hz"), *return_lpf_hz);
                check(&mut r, "returns.<reverb>.damping", &format!("returns.{n}.damping"), *damping);
                check(&mut r, "returns.<reverb>.predelay_ms", &format!("returns.{n}.predelay_ms"), *predelay_ms);
                check(&mut r, "returns.<reverb>.return_hpf_hz", &format!("returns.{n}.return_hpf_hz"), *return_hpf_hz);
                check(
                    &mut r,
                    "returns.<reverb>.return_gain_db",
                    &format!("returns.{n}.return_gain_db"),
                    *return_gain_db,
                );
            }
            ReturnPlan::Delay {
                time_ms,
                feedback,
                feedback_hpf_hz,
                feedback_lpf_hz,
                return_gain_db,
                wow,
                drive_db,
                ..
            } => {
                if let Some(v) = wow {
                    check(&mut r, "returns.<delay>.wow", &format!("returns.{n}.wow"), *v);
                }
                if let Some(v) = drive_db {
                    check(&mut r, "returns.<delay>.drive_db", &format!("returns.{n}.drive_db"), *v);
                }
                check(&mut r, "returns.<delay>.time_ms", &format!("returns.{n}.time_ms"), *time_ms);
                check(&mut r, "returns.<delay>.feedback", &format!("returns.{n}.feedback"), *feedback);
                check(
                    &mut r,
                    "returns.<delay>.feedback_hpf_hz",
                    &format!("returns.{n}.feedback_hpf_hz"),
                    *feedback_hpf_hz,
                );
                check(
                    &mut r,
                    "returns.<delay>.feedback_lpf_hz",
                    &format!("returns.{n}.feedback_lpf_hz"),
                    *feedback_lpf_hz,
                );
                check(
                    &mut r,
                    "returns.<delay>.return_gain_db",
                    &format!("returns.{n}.return_gain_db"),
                    *return_gain_db,
                );
            }
        }
    }
    let check_sends = |r: &mut Report, at: &str, sends: &BTreeMap<String, f64>| {
        for (s, v) in sends {
            if !returns.contains_key(s) {
                r.errors.push(format!("{at}.sends_db: `{s}` is not a return"));
            }
            check(r, "sends_db.<return>", &format!("{at}.sends_db.{s}"), *v);
        }
    };
    let check_comp = |r: &mut Report, at: &str, owner: &str, c: &Option<crate::plan::CompressorPlan>| {
        if let Some(c) = c {
            check(r, "compressor.threshold_db", &format!("{at}.compressor.threshold_db"), c.threshold_db);
            check(r, "compressor.ratio", &format!("{at}.compressor.ratio"), c.ratio);
            check(r, "compressor.attack_ms", &format!("{at}.compressor.attack_ms"), c.attack_ms);
            check(r, "compressor.release_ms", &format!("{at}.compressor.release_ms"), c.release_ms);
            check(r, "compressor.knee_db", &format!("{at}.compressor.knee_db"), c.knee_db);
            check(r, "compressor.makeup_db", &format!("{at}.compressor.makeup_db"), c.makeup_db);
            check(r, "compressor.mix", &format!("{at}.compressor.mix"), c.mix);
            if let Some(sc) = &c.sidechain {
                let ok = ids.to_project.get(sc).and_then(|id| p.track(id)).is_some_and(|t| !t.is_return());
                if !ok || sc == owner {
                    r.errors.push(format!("{at}.compressor.sidechain `{sc}` must be another track or group"));
                }
            }
        }
    };
    let check_sat = |r: &mut Report, at: &str, s: &Option<crate::plan::SaturationPlan>| {
        if let Some(s) = s {
            check(r, "saturation.drive_db", &format!("{at}.saturation.drive_db"), s.drive_db);
            check(r, "saturation.mix", &format!("{at}.saturation.mix"), s.mix);
        }
    };
    for (n, t) in &plan.tracks {
        let at = format!("tracks.{n}");
        check(&mut r, "tracks.<id>.gain_db", &format!("{at}.gain_db"), t.gain_db);
        check(&mut r, "tracks.<id>.pan", &format!("{at}.pan"), t.pan);
        if let Some(f) = t.hpf_hz {
            check(&mut r, "tracks.<id>.hpf_hz", &format!("{at}.hpf_hz"), f);
        }
        if let Some(f) = t.lpf_hz {
            check(&mut r, "tracks.<id>.lpf_hz", &format!("{at}.lpf_hz"), f);
            if f >= nyquist {
                r.errors.push(format!("{at}.lpf_hz must be below {nyquist} Hz"));
            }
        }
        check_eq(&mut r, &at, &t.eq, nyquist, MAX_TRACK_EQ, false);
        check_comp(&mut r, &at, n, &t.compressor);
        check_sat(&mut r, &at, &t.saturation);
        check_sends(&mut r, &at, &t.sends_db);
        for (s, sp) in &t.sections {
            if !sections.contains(s) {
                r.errors.push(format!("{at}.sections: `{s}` is not a section"));
            }
            check(
                &mut r,
                "sections.<id>.gain_offset_db",
                &format!("{at}.sections.{s}.gain_offset_db"),
                sp.gain_offset_db,
            );
            for (ret, v) in &sp.send_offsets_db {
                // A throw of an idea rides the send the idea itself adds (AI-26).
                let own = sp.idea.as_ref().is_some_and(|i| t.idea_sends_db.get(i).is_some_and(|m| m.contains_key(ret)));
                if !t.sends_db.contains_key(ret) && !own {
                    r.errors.push(format!("{at}.sections.{s}.send_offsets_db.{ret}: the track has no send to `{ret}`"));
                }
                check(
                    &mut r,
                    "sections.<id>.send_offsets_db.<return>",
                    &format!("{at}.sections.{s}.send_offsets_db.{ret}"),
                    *v,
                );
            }
            if sp.eq_gain_offsets_db.len() > t.eq.len() {
                r.errors
                    .push(format!("{at}.sections.{s}.eq_gain_offsets_db has more values than the track has EQ bands"));
            }
            for (i, v) in sp.eq_gain_offsets_db.iter().enumerate() {
                check(
                    &mut r,
                    "sections.<id>.eq_gain_offsets_db[]",
                    &format!("{at}.sections.{s}.eq_gain_offsets_db[{i}]"),
                    *v,
                );
            }
        }
    }
    for (n, g) in &plan.groups {
        let at = format!("groups.{n}");
        check(&mut r, "groups.<id>.gain_db", &format!("{at}.gain_db"), g.gain_db);
        check_eq(&mut r, &at, &g.eq, nyquist, MAX_TRACK_EQ, false);
        check_comp(&mut r, &at, n, &g.compressor);
        check_sat(&mut r, &at, &g.saturation);
        check_sends(&mut r, &at, &g.sends_db);
    }
    v4(plan, p, ids, &sections, &mut r);
    // Routing: sidechains must not create a loop (FX-3, §5.5).
    if r.ok() {
        let cmds = apply_mix_plan(p, plan, ids, &ApplyOptions::default());
        let mut q = p.clone();
        for c in cmds {
            if let Err(e) = c.apply(&mut q) {
                r.errors.push(e.to_string());
            }
        }
        if let Err(e) = stemwise_core::routing::validate(&q) {
            r.errors.push(format!("routing: {e}"));
        }
    }
    r
}

/// The checks of version 4 (AI-25, AI-26, AI-6 with P-018): blocks, filter resonances and offsets, ideas. What an
/// idea may not do (change what exists) is dropped with a warning rather than failing the plan (AI-26).
fn v4(plan: &mut MixPlan, p: &Project, ids: &Ids, sections: &[String], r: &mut Report) {
    use crate::plan::{IDEA_KEYS, MAX_IDEA_NAME, MAX_NEW_BLOCKS};
    use stemwise_core::fx::FxKind;
    let ideas: Vec<String> = plan.ideas.as_ref().map(|m| m.keys().cloned().collect()).unwrap_or_default();
    if let Some(m) = &mut plan.ideas {
        if m.len() > IDEA_KEYS.len() {
            r.errors.push(format!("at most {} ideas", IDEA_KEYS.len()));
        }
        for (k, idea) in m.iter_mut() {
            if !IDEA_KEYS.contains(&k.as_str()) {
                r.errors.push(format!("idea `{k}`: use i1, i2 or i3"));
            }
            if idea.name.trim().is_empty() || idea.rationale.trim().is_empty() {
                r.errors.push(format!("idea `{k}` needs a name and a rationale"));
            }
            if idea.name.chars().count() > MAX_IDEA_NAME {
                idea.name = idea.name.chars().take(MAX_IDEA_NAME).collect();
                r.warnings.push(format!("the name of idea `{k}` was shortened to {MAX_IDEA_NAME} characters"));
            }
            for s in &idea.sections {
                if !sections.contains(s) {
                    r.errors.push(format!("idea `{k}`: `{s}` is not a section"));
                }
            }
        }
    }
    let known_idea = |i: &Option<String>, r: &mut Report, at: &str| {
        if let Some(i) = i
            && !ideas.contains(i)
        {
            r.errors.push(format!("{at}: idea `{i}` is not in `ideas`"));
        }
    };
    let tempo = p.tempo.is_some();
    // Blocks of one chain: their ids, kinds, places, limits and the room the chain has.
    let check_blocks = |owner: &str, at: &str, blocks: &mut Vec<crate::plan::BlockPlan>, r: &mut Report| {
        let Some(t) = ids.to_project.get(owner).and_then(|id| p.track(id)) else { return };
        let mut chain = t.chain.clone();
        let mut new = 0;
        let mut keep = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for bp in blocks.drain(..) {
            let bat = format!("{at}.blocks.{}", bp.id);
            if seen.contains(&bp.id) {
                r.errors.push(format!("{bat}: two blocks of the chain share the id `{}`", bp.id));
            }
            seen.push(bp.id.clone());
            known_idea(&bp.idea, r, &bat);
            let existing = ids.blocks.get(&bp.id);
            match existing {
                Some((tr, b)) if *tr == t.id => {
                    if bp.idea.is_some() {
                        r.warnings
                            .push(format!("{bat}: an idea only adds, it does not change a block that exists: dropped"));
                        continue;
                    }
                    if t.chain.block(b).is_none_or(|x| x.fx.kind() != bp.kind) {
                        r.errors.push(format!("{bat}: `{}` is a block of another type", bp.id));
                    }
                }
                Some(_) => r.errors.push(format!("{bat}: `{}` is a block of another track", bp.id)),
                None => {
                    new += 1;
                    if bp.position.is_none() {
                        r.errors.push(format!("{bat}: a new block needs a `position`"));
                    }
                    if let Err(e) = chain.can_add(bp.kind, t.is_return()) {
                        r.errors.push(format!("{bat}: {e}"));
                    } else {
                        let id = chain.free_id(bp.kind);
                        chain.blocks.push(stemwise_core::fx::FxBlock { id, fx: stemwise_core::fx::Fx::new(bp.kind) });
                    }
                }
            }
            if !tempo && bp.settings.get("rate").is_some_and(|v| !v.is_null()) {
                r.errors.push(format!("{bat}: a musical rate needs a tempo; give `rate_hz` and `rate`: null"));
            }
            let base = existing.and_then(|(_, b)| t.chain.block(b)).map(|x| x.fx.clone());
            // With a starting plan (a revision, a later iteration), a setting kept as it is in the project is
            // accepted outside the plan's limits, like the classic fields.
            let current = base.as_ref().filter(|_| !r.kept.is_empty()).map(|fx| crate::blocks::settings_of(fx, ids));
            r.errors.extend(crate::blocks::check(&bp, current.as_ref()));
            match crate::blocks::to_fx(&bp, base.as_ref(), ids) {
                Ok(fx) => {
                    let mut probe =
                        stemwise_core::model::Chain { trim_db: 0.0, blocks: Vec::new(), fader_db: 0.0, pan: 0.0 };
                    probe.blocks.push(stemwise_core::fx::FxBlock { id: bp.kind.key().into(), fx });
                    if let Err(e) = probe.check(t.is_return()) {
                        r.errors.push(format!("{bat}: {e}"));
                    }
                }
                Err(e) => r.errors.push(e),
            }
            keep.push(bp);
        }
        if new > MAX_NEW_BLOCKS {
            r.errors.push(format!("{at}: at most {MAX_NEW_BLOCKS} new blocks per chain"));
        }
        let _ = FxKind::ALL;
        *blocks = keep;
    };
    // A return of the project, or one the plan creates (AI-17).
    let created: Vec<String> =
        plan.structure.as_ref().map(|st| st.returns.keys().cloned().collect()).unwrap_or_default();
    let is_return = |k: &str| -> bool {
        created.iter().any(|c| c == k)
            || ids.to_project.get(k).and_then(|id| p.track(id)).is_some_and(|t| t.is_return())
    };
    let track_keys: Vec<String> = plan.tracks.keys().cloned().collect();
    for n in &track_keys {
        let at = format!("tracks.{n}");
        let mut blocks = std::mem::take(&mut plan.tracks.get_mut(n).expect("listed").blocks);
        check_blocks(n, &at, &mut blocks, r);
        let t = plan.tracks.get_mut(n).expect("listed");
        t.blocks = blocks;
        for (k, v) in [("hpf_resonance", t.hpf_resonance), ("lpf_resonance", t.lpf_resonance)] {
            if let Some(v) = v {
                // A resonance the author set beyond the plan's limit and the plan keeps is accepted (AI-3).
                check(r, &format!("tracks.<id>.{k}"), &format!("{at}.{k}"), v);
            }
        }
        let new_ids: Vec<String> = t.blocks.iter().map(|b| b.id.clone()).collect();
        let owner = ids.to_project.get(n).cloned().unwrap_or_default();
        // AI-26: an idea switches on only the blocks it adds, never one that exists or one of the plan.
        for (s, sp) in t.sections.iter_mut() {
            let Some(i) = sp.idea.clone() else { continue };
            let own: Vec<&String> =
                t.blocks.iter().filter(|b| b.idea.as_deref() == Some(i.as_str())).map(|b| &b.id).collect();
            sp.blocks_on.retain(|b| {
                let ok = own.contains(&b);
                if !ok {
                    r.warnings.push(format!(
                        "{at}.sections.{s}.blocks_on: idea {i} switches only the blocks it adds, not `{b}`: dropped"
                    ));
                }
                ok
            });
        }
        for (s, sp) in &t.sections {
            known_idea(&sp.idea, r, &format!("{at}.sections.{s}"));
            for (k, v) in [("hpf_offset_oct", sp.hpf_offset_oct), ("lpf_offset_oct", sp.lpf_offset_oct)] {
                if let Some(v) = v {
                    let l = crate::plan::limit(&format!("sections.<id>.{k}"));
                    if !(l.min..=l.max).contains(&v) {
                        r.errors.push(format!("{at}.sections.{s}.{k} = {v} is outside {} … {}", l.min, l.max));
                    }
                }
            }
            for b in &sp.blocks_on {
                let mine = new_ids.contains(b) || ids.blocks.get(b).is_some_and(|(tr, _)| *tr == owner);
                if !mine {
                    r.errors.push(format!("{at}.sections.{s}.blocks_on: `{b}` is not a block of this track"));
                }
            }
        }
        let sends = t.sends_db.clone();
        check_idea_sends(&at, &sends, &mut t.idea_sends_db, &ideas, &is_return, r);
    }
    let group_keys: Vec<String> = plan.groups.keys().cloned().collect();
    for n in &group_keys {
        let mut blocks = std::mem::take(&mut plan.groups.get_mut(n).expect("listed").blocks);
        check_blocks(n, &format!("groups.{n}"), &mut blocks, r);
        let g = plan.groups.get_mut(n).expect("listed");
        g.blocks = blocks;
        let sends = g.sends_db.clone();
        check_idea_sends(&format!("groups.{n}"), &sends, &mut g.idea_sends_db, &ideas, &is_return, r);
    }
    let return_keys: Vec<String> = plan.returns.keys().cloned().collect();
    for n in &return_keys {
        let mut blocks = std::mem::take(plan.returns.get_mut(n).expect("listed").blocks_mut());
        check_blocks(n, &format!("returns.{n}"), &mut blocks, r);
        *plan.returns.get_mut(n).expect("listed").blocks_mut() = blocks;
    }
    if let Some(st) = &plan.structure {
        for (k, nr) in &st.returns {
            known_idea(&nr.idea, r, &format!("structure.returns.{k}"));
        }
    }
}

/// An idea's sends (AI-26): to a return, new sends only, within the limits of the plan's sends (AI-8).
fn check_idea_sends(
    at: &str,
    existing: &std::collections::BTreeMap<String, f64>,
    idea_sends: &mut std::collections::BTreeMap<String, std::collections::BTreeMap<String, f64>>,
    ideas: &[String],
    is_return: &dyn Fn(&str) -> bool,
    r: &mut Report,
) {
    for (i, m) in idea_sends.iter_mut() {
        if !ideas.contains(i) {
            r.errors.push(format!("{at}.idea_sends_db: idea `{i}` is not in `ideas`"));
        }
        m.retain(|ret, v| {
            if !is_return(ret) {
                r.errors.push(format!("{at}.idea_sends_db.{i}.{ret}: sends go to returns only"));
                return false;
            }
            if existing.contains_key(ret) {
                r.warnings.push(format!(
                    "{at}.idea_sends_db.{i}.{ret}: an idea adds sends, it does not change one that exists: dropped"
                ));
                return false;
            }
            let l = crate::plan::limit("sends_db.<return>");
            if !(l.min..=l.max).contains(v) {
                r.errors.push(format!("{at}.idea_sends_db.{i}.{ret} = {v} is outside {} … {}", l.min, l.max));
            }
            true
        });
    }
}

/// DEC-5: limits the change from the previous iteration's plan (gain ±4 dB, EQ band gain ±3 dB by position and
/// type), clipping with warnings.
pub fn limit_changes(plan: &mut MixPlan, previous: &MixPlan) -> Vec<String> {
    let mut w = Vec::new();
    let clamp = |v: &mut f64, prev: f64, max: f64, what: String, w: &mut Vec<String>| {
        let lo = prev - max;
        let hi = prev + max;
        if *v < lo || *v > hi {
            let c = v.clamp(lo, hi);
            w.push(format!("{what} changed from {prev:.1} to {v:.1} dB: clipped to {c:.1} dB"));
            *v = c;
        }
    };
    for (n, t) in plan.tracks.iter_mut() {
        if let Some(p) = previous.tracks.get(n) {
            clamp(&mut t.gain_db, p.gain_db, MAX_GAIN_CHANGE_DB, format!("tracks.{n}.gain_db"), &mut w);
            for (i, b) in t.eq.iter_mut().enumerate() {
                // A new band, or one of another type, starts from 0 dB (`_clamp_eq` of the PoC).
                let prev = p.eq.get(i).filter(|pb| pb.kind == b.kind).map_or(0.0, |pb| pb.gain_db);
                clamp(&mut b.gain_db, prev, MAX_EQ_CHANGE_DB, format!("tracks.{n}.eq[{i}].gain_db"), &mut w);
            }
        }
    }
    for (n, g) in plan.groups.iter_mut() {
        if let Some(p) = previous.groups.get(n) {
            clamp(&mut g.gain_db, p.gain_db, MAX_GAIN_CHANGE_DB, format!("groups.{n}.gain_db"), &mut w);
            for (i, b) in g.eq.iter_mut().enumerate() {
                let prev = p.eq.get(i).filter(|pb| pb.kind == b.kind).map_or(0.0, |pb| pb.gain_db);
                clamp(&mut b.gain_db, prev, MAX_EQ_CHANGE_DB, format!("groups.{n}.eq[{i}].gain_db"), &mut w);
            }
        }
    }
    w
}

/// Checks a master plan: limits, the blocks a chain may hold (MAS-15), the bands of a multiband (MAS-12) and the
/// sections it offsets (MAS-14; `sections` are the ids the plan may name).
pub fn validate_master(plan: &MasterPlan, rate: u32, sections: &[String]) -> Report {
    use crate::plan::{MAX_MASTER_BLOCKS, MB_BANDS, MasterBlockPlan as B};
    use stemwise_core::model::{BlockKind, master_limits};
    let mut r = Report::default();
    let nyquist = f64::from(rate) / 2.0;
    if plan.version != crate::plan::MASTER_PLAN_VERSION {
        r.errors.push(format!("version must be {}", crate::plan::MASTER_PLAN_VERSION));
    }
    if plan.summary.trim().is_empty() {
        r.errors.push("summary is empty: describe the master in one or two sentences".into());
    }
    let c = |r: &mut Report, path: &str, v: f64| {
        let l = MASTER_LIMITS.iter().find(|l| l.path == path).expect("known");
        if !v.is_finite() || v < l.min || v > l.max {
            r.errors.push(format!("{path} = {v} is outside {} … {} {}", l.min, l.max, l.unit));
        }
    };
    c(&mut r, "target.lufs", plan.target.lufs);
    c(&mut r, "target.true_peak_db", plan.target.true_peak_db);
    c(&mut r, "input_gain_db", plan.input_gain_db);
    if plan.chain.len() > MAX_MASTER_BLOCKS {
        r.errors.push(format!("chain has {} blocks: {MAX_MASTER_BLOCKS} at most", plan.chain.len()));
    }
    for kind in BlockKind::ALL {
        let n = plan.chain.iter().filter(|b| b.kind() == kind).count();
        if n > kind.max() {
            r.errors.push(format!("chain has {n} {} blocks: {} at most", kind.label(), kind.max()));
        }
    }
    for (i, b) in plan.chain.iter().enumerate() {
        if b.id().trim().is_empty() {
            r.errors.push(format!("chain[{i}] has no id"));
        }
        if plan.chain[..i].iter().any(|x| x.id() == b.id()) {
            r.errors.push(format!("two blocks of the chain share the id `{}`", b.id()));
        }
        match b {
            B::Eq { bands, .. } => check_eq(&mut r, &format!("chain[{i}]"), bands, nyquist, MAX_MASTER_EQ, true),
            B::MatchEq { amount, .. } => c(&mut r, "match_eq.amount", *amount),
            B::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                knee_db,
                makeup_db,
                mix,
                listen_above_hz,
                ..
            } => {
                c(&mut r, "compressor.threshold_db", *threshold_db);
                c(&mut r, "compressor.ratio", *ratio);
                c(&mut r, "compressor.attack_ms", *attack_ms);
                c(&mut r, "compressor.release_ms", *release_ms);
                c(&mut r, "compressor.knee_db", *knee_db);
                c(&mut r, "compressor.makeup_db", *makeup_db);
                c(&mut r, "compressor.mix", *mix);
                c(&mut r, "compressor.listen_above_hz", *listen_above_hz);
            }
            B::Multiband { crossovers_hz, bands, .. } => {
                if !(MB_BANDS.0..=MB_BANDS.1).contains(&bands.len()) || crossovers_hz.len() + 1 != bands.len() {
                    r.errors.push(format!(
                        "chain[{i}]: a multiband has {} to {} bands and one crossover fewer than its bands",
                        MB_BANDS.0, MB_BANDS.1
                    ));
                }
                for (k, x) in crossovers_hz.iter().enumerate() {
                    c(&mut r, "multiband.crossovers_hz[]", *x);
                    if k > 0 && *x < crossovers_hz[k - 1] * master_limits::MB_MIN_RATIO {
                        r.errors.push(format!(
                            "chain[{i}]: crossovers must rise, at least a third of an octave apart ({} then {x})",
                            crossovers_hz[k - 1]
                        ));
                    }
                }
                for band in bands {
                    c(&mut r, "multiband.bands[].threshold_db", band.threshold_db);
                    c(&mut r, "multiband.bands[].ratio", band.ratio);
                    c(&mut r, "multiband.bands[].attack_ms", band.attack_ms);
                    c(&mut r, "multiband.bands[].release_ms", band.release_ms);
                    c(&mut r, "multiband.bands[].gain_db", band.gain_db);
                }
            }
            B::Width { side_gain_db, mono_below_hz, .. } => {
                c(&mut r, "width.side_gain_db", *side_gain_db);
                c(&mut r, "width.mono_below_hz", *mono_below_hz);
            }
            B::Saturation { drive_db, mix, .. } => {
                c(&mut r, "saturation.drive_db", *drive_db);
                c(&mut r, "saturation.mix", *mix);
            }
            B::Clipper { take_off_db, softness, .. } => {
                c(&mut r, "clipper.take_off_db", *take_off_db);
                c(&mut r, "clipper.softness", *softness);
            }
        }
    }
    c(&mut r, "limiter.release_ms", plan.limiter.release_ms);
    for (i, s) in plan.sections.iter().enumerate() {
        if !sections.contains(&s.section) {
            r.errors.push(format!("sections[{i}]: `{}` is not a section of this song", s.section));
        }
        if plan.sections[..i].iter().any(|x| x.section == s.section) {
            r.errors.push(format!("section `{}` is listed twice", s.section));
        }
        c(&mut r, "sections[].level_db", s.level_db);
        c(&mut r, "sections[].side_gain_db", s.side_gain_db);
        c(&mut r, "sections[].limiter_release_ms", s.limiter_release_ms);
        for e in &s.eq {
            match plan.chain.iter().find(|b| b.id() == e.block) {
                Some(B::Eq { bands, .. }) => {
                    if e.gains_db.len() != bands.len() {
                        r.errors.push(format!(
                            "sections[{i}]: {} offsets for the {} bands of `{}` (one per band, 0 for none)",
                            e.gains_db.len(),
                            bands.len(),
                            e.block
                        ));
                    }
                }
                _ => r.errors.push(format!("sections[{i}]: `{}` is not an EQ block of the chain", e.block)),
            }
            for g in &e.gains_db {
                c(&mut r, "sections[].eq[].gains_db[]", *g);
            }
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::plan_from_project;
    use crate::plan::{CompressorPlan, EqType};
    use stemwise_core::model::{ReturnFx, Track};

    fn project() -> Project {
        let mut p = Project::new("v");
        p.sample_rate = Some(48000);
        for (id, n) in [("a", "Kick"), ("b", "Bass")] {
            p.tracks.push(Track::new_audio(id.into(), n, None, 1, 1));
        }
        p
    }

    fn all(p: &Project, ids: &Ids) -> Editable {
        let of = |f: fn(&Track) -> bool| p.tracks.iter().filter(|t| f(t)).map(|t| ids.n(&t.id)).collect();
        Editable { tracks: of(Track::is_audio), groups: of(Track::is_group), returns: of(Track::is_return) }
    }

    /// The current mix as a plan the model could return: texts filled in.
    fn start(p: &Project, ids: &Ids) -> MixPlan {
        let mut plan = plan_from_project(p, ids);
        plan.summary = "Balance.".into();
        for t in plan.tracks.values_mut() {
            t.rationale = "Why.".into();
        }
        for g in plan.groups.values_mut() {
            g.rationale = "Why.".into();
        }
        plan
    }

    /// req: AI-26, AI-8
    #[test]
    fn an_idea_only_adds_its_sends_go_to_returns_within_limits_and_it_switches_only_its_blocks() {
        use crate::plan::{BlockPlan, BlockPosition, IdeaPlan, SectionPlan};
        use stemwise_core::fx::FxKind;
        let mut p = project();
        p.tracks.push(Track::new_group("g".into(), "Drums"));
        let ids = Ids::of(&p);
        let ed = all(&p, &ids);
        let mut plan = start(&p, &ids);
        plan.ideas = Some(
            [("i1".to_string(), IdeaPlan { name: "Idea".into(), sections: vec![], rationale: "Why.".into() })].into(),
        );
        let (t1, g1) = (ids.n("a"), ids.n("g"));
        let ret = ids.n(&p.tracks.iter().find(|t| t.is_return()).unwrap().id);
        let block = |id: &str, idea: Option<&str>| BlockPlan {
            id: id.into(),
            kind: FxKind::Width,
            idea: idea.map(str::to_string),
            position: Some(BlockPosition::Last),
            bypass: None,
            settings: Default::default(),
        };
        let t = plan.tracks.get_mut(&t1).unwrap();
        t.sends_db.remove(&ret);
        t.blocks = vec![block("mine", None), block("its", Some("i1"))];
        t.sections.insert(
            ids.n(&p.sections.first().map_or("x".to_string(), |s| s.id.clone())),
            SectionPlan { idea: Some("i1".into()), blocks_on: vec!["mine".into(), "its".into()], ..Default::default() },
        );
        let g = plan.groups.get_mut(&g1).unwrap();
        g.idea_sends_db.insert("i1".into(), [(t1.clone(), -10.0), (ret.clone(), 40.0)].into());
        let r = validate_mix(&mut plan, &p, &ids, &ed, None);
        assert!(
            r.errors
                .iter()
                .any(|e| e.contains(&format!("groups.{g1}.idea_sends_db.i1.{t1}: sends go to returns only"))),
            "{:?}",
            r.errors
        );
        assert!(r.errors.iter().any(|e| e.contains(&format!("idea_sends_db.i1.{ret} = 40"))), "{:?}", r.errors);
        assert!(
            r.warnings.iter().any(|w| w.contains("switches only the blocks it adds, not `mine`")),
            "{:?}",
            r.warnings
        );
        let s = plan.tracks[&t1].sections.values().next().unwrap();
        assert_eq!(s.blocks_on, ["its"]);
    }

    /// A throw: the idea adds a send and raises it in one section. The offset rides the idea's own send.
    /// req: AI-26
    #[test]
    fn an_idea_may_raise_its_own_send_in_a_section() {
        use crate::plan::{IdeaPlan, SectionPlan};
        let mut p = project();
        if p.sections.is_empty() {
            p.sections.push(stemwise_core::model::Section {
                id: "s".into(),
                name: "Chorus".into(),
                start: 0,
                end: 48000,
                color: Default::default(),
            });
        }
        let ids = Ids::of(&p);
        let ed = all(&p, &ids);
        let mut plan = start(&p, &ids);
        plan.ideas = Some(
            [("i1".to_string(), IdeaPlan { name: "Throw".into(), sections: vec![], rationale: "Why.".into() })].into(),
        );
        let t1 = ids.n("a");
        let ret = ids.n(&p.tracks.iter().find(|t| t.is_return()).unwrap().id);
        let sec = ids.n(&p.sections[0].id);
        let t = plan.tracks.get_mut(&t1).unwrap();
        t.sends_db.remove(&ret);
        t.sections.insert(
            sec.clone(),
            SectionPlan { idea: Some("i1".into()), send_offsets_db: [(ret.clone(), 6.0)].into(), ..Default::default() },
        );
        // Without the idea's send the offset has nothing to ride.
        let mut without = plan.clone();
        let r = validate_mix(&mut without, &p, &ids, &ed, None);
        assert!(r.errors.iter().any(|e| e.contains("has no send to")), "{:?}", r.errors);
        plan.tracks.get_mut(&t1).unwrap().idea_sends_db.insert("i1".into(), [(ret.clone(), -16.0)].into());
        let r = validate_mix(&mut plan, &p, &ids, &ed, None);
        assert!(r.ok(), "{:?}", r.errors);
        // Ticked, the send and its offset are both in the plan applied.
        let kept = crate::ideas::strip(&plan, &["i1".to_string()].into(), &[]);
        assert_eq!(kept.tracks[&t1].sends_db.get(&ret), Some(&-16.0));
        assert_eq!(kept.tracks[&t1].sections[&sec].send_offsets_db.get(&ret), Some(&6.0));
    }

    /// The chain has one modulation, the plan the loop kept adds a second: the idea's one has no room (two per
    /// type, FX-10) and is left out of the proposal with a warning, instead of being listed and silently skipped.
    /// req: AI-25, AI-26
    #[test]
    fn an_idea_block_that_no_longer_fits_the_chain_is_left_out_with_a_warning() {
        use crate::plan::{BlockPlan, BlockPosition, IdeaPlan};
        use stemwise_core::fx::{Fx, FxBlock, FxKind};
        let mut p = project();
        p.track_mut("a").unwrap().chain.blocks.push(FxBlock { id: "mod".into(), fx: Fx::new(FxKind::Modulation) });
        let ids = Ids::of(&p);
        let mut plan = start(&p, &ids);
        plan.ideas = Some(
            [("i1".to_string(), IdeaPlan { name: "Chorus".into(), sections: vec![], rationale: "Why.".into() })].into(),
        );
        let t1 = ids.n("a");
        let block = |id: &str, idea: Option<&str>| BlockPlan {
            id: id.into(),
            kind: FxKind::Modulation,
            idea: idea.map(str::to_string),
            position: Some(BlockPosition::Last),
            bypass: None,
            settings: Default::default(),
        };
        let existing = plan.tracks[&t1].blocks.len();
        plan.tracks.get_mut(&t1).unwrap().blocks.extend([block("its", Some("i1")), block("mine", None)]);
        let w = crate::ideas::fit(&mut plan, &p, &ids);
        assert!(w.len() == 1 && w[0].contains("`its`") && w[0].contains("no room"), "{w:?}");
        let left: Vec<&str> = plan.tracks[&t1].blocks[existing..].iter().map(|b| b.id.as_str()).collect();
        assert_eq!(left, ["mine"]);
    }

    /// req: AI-8
    #[test]
    fn limits_ids_completeness_texts_and_sidechains_are_checked() {
        let mut p = project();
        p.tracks.push(Track::new_group("g".into(), "Drums"));
        let ids = Ids::of(&p);
        let ed = all(&p, &ids);
        let mut plan = start(&p, &ids);
        assert!(validate_mix(&mut plan, &p, &ids, &ed, None).ok());
        let t1 = ids.n("a");
        plan.tracks.get_mut(&t1).unwrap().gain_db = 20.0;
        plan.tracks.get_mut(&t1).unwrap().eq =
            vec![EqBand { kind: EqType::Peak, freq_hz: 30000.0, gain_db: 1.0, q: 1.0 }];
        plan.tracks.get_mut(&t1).unwrap().sends_db.insert("r9".into(), -10.0);
        let r = validate_mix(&mut plan, &p, &ids, &ed, None);
        assert!(r.errors.iter().any(|e| e.contains("gain_db = 20")));
        assert!(r.errors.iter().any(|e| e.contains("below 24000")));
        assert!(r.errors.iter().any(|e| e.contains("`r9` is not a return")));
        assert!(r.feedback().starts_with("The plan is invalid"));
        let mut plan = start(&p, &ids);
        plan.tracks.remove(&ids.n("b"));
        assert!(!validate_mix(&mut plan, &p, &ids, &ed, None).ok(), "incomplete plan");
        // Empty texts (DEC-3 of the PoC) and ids under the wrong kind are refused.
        let mut plan = start(&p, &ids);
        plan.summary = " ".into();
        plan.tracks.get_mut(&t1).unwrap().rationale = String::new();
        let g = plan.groups.remove(&ids.n("g")).unwrap();
        plan.groups.insert(t1.clone(), g);
        let r = validate_mix(&mut plan, &p, &ids, &ed, None);
        assert!(r.errors.iter().any(|e| e.contains("summary is empty")));
        assert!(r.errors.iter().any(|e| e.contains("empty rationale")));
        assert!(r.errors.iter().any(|e| e.contains("is not a group")), "{:?}", r.errors);
        // A track keyed by itself is refused; a key from another track is fine.
        let mut plan = start(&p, &ids);
        let comp = CompressorPlan {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 100.0,
            knee_db: 3.0,
            makeup_db: 0.0,
            mix: 1.0,
            sidechain: Some(t1.clone()),
        };
        plan.tracks.get_mut(&t1).unwrap().compressor = Some(comp.clone());
        assert!(!validate_mix(&mut plan, &p, &ids, &ed, None).ok());
        let mut plan = start(&p, &ids);
        plan.tracks.get_mut(&ids.n("b")).unwrap().compressor = Some(comp.clone());
        assert!(validate_mix(&mut plan, &p, &ids, &ed, None).ok());
        // A real loop: the group is keyed by a track routed into it (the key would depend on the group's output).
        let mut q = p.clone();
        q.track_mut("a").unwrap().output = stemwise_core::model::Output::Group("g".into());
        let ids = Ids::of(&q);
        let mut plan = start(&q, &ids);
        let mut c = comp;
        c.sidechain = Some(ids.n("g"));
        plan.tracks.get_mut(&ids.n("a")).unwrap().compressor = Some(c);
        let r = validate_mix(&mut plan, &q, &ids, &all(&q, &ids), None);
        assert!(r.errors.iter().any(|e| e.contains("routing")), "{:?}", r.errors);
    }

    /// req: AI-3, AI-8
    #[test]
    fn values_the_author_set_outside_the_limits_are_kept_but_not_newly_chosen() {
        // B2 of the F5 review: a fader at −40 dB, a send at −3 dB and an EQ band at −12 dB set by hand.
        let mut p = project();
        p.tracks.push(Track::new_return("r".into(), "Plate", ReturnFx::default_reverb()));
        let a = p.track_mut("a").unwrap();
        a.chain.fader_db = -40.0;
        a.sends.push(stemwise_core::model::Send { target: "r".into(), level_db: -3.0 });
        a.chain.eq_mut().bands[0].enabled = true;
        a.chain.eq_mut().bands[0].gain_db = -12.0;
        let ids = Ids::of(&p);
        let ed = all(&p, &ids);
        let sp = start(&p, &ids);
        let t1 = ids.n("a");
        assert_eq!(sp.tracks[&t1].gain_db, -40.0, "the starting plan carries the exact value");
        // Echoed unchanged: valid in a revision.
        let mut plan = sp.clone();
        assert!(validate_mix(&mut plan, &p, &ids, &ed, Some(&sp)).ok());
        // Without a starting plan, or with a changed value, the limits apply.
        let mut plan = sp.clone();
        assert!(!validate_mix(&mut plan, &p, &ids, &ed, None).ok());
        let mut plan = sp.clone();
        plan.tracks.get_mut(&t1).unwrap().gain_db = -30.0;
        let r = validate_mix(&mut plan, &p, &ids, &ed, Some(&sp));
        assert_eq!(r.errors.len(), 1, "{:?}", r.errors);
        assert!(r.errors[0].contains("gain_db = -30"));
    }

    /// The interface allows more than the plan (resonance to 10, gate range to 80 dB): what the author set by hand
    /// does not make a revision fail, and the plan still cannot choose such values itself.
    /// req: AI-3, AI-8, AI-25
    #[test]
    fn blocks_and_resonances_the_author_set_outside_the_limits_are_kept_but_not_newly_chosen() {
        use stemwise_core::fx::{Fx, FxBlock, FxKind};
        let mut p = project();
        let a = p.track_mut("a").unwrap();
        for b in &mut a.chain.blocks {
            if let Fx::Hpf(f) = &mut b.fx {
                f.bypass = false;
                f.resonance = 6.0;
            }
        }
        let mut gate = Fx::new(FxKind::Gate);
        gate.set_bypass(false);
        if let Fx::Gate(g) = &mut gate {
            g.range_db = 60.0;
        }
        a.chain.blocks.push(FxBlock { id: "gate".into(), fx: gate });
        let ids = Ids::of(&p);
        let ed = all(&p, &ids);
        let sp = start(&p, &ids);
        let t1 = ids.n("a");
        assert_eq!(sp.tracks[&t1].hpf_resonance, Some(6.0));
        assert_eq!(sp.tracks[&t1].blocks.len(), 1, "the gate is in the starting plan");
        let mut plan = sp.clone();
        let r = validate_mix(&mut plan, &p, &ids, &ed, Some(&sp));
        assert!(r.ok(), "{:?}", r.errors);
        // Changed to other values outside the limits: refused.
        let mut plan = sp.clone();
        let t = plan.tracks.get_mut(&t1).unwrap();
        t.hpf_resonance = Some(5.0);
        t.blocks[0].settings.insert("range_db".into(), serde_json::json!(50.0));
        let r = validate_mix(&mut plan, &p, &ids, &ed, Some(&sp));
        assert_eq!(r.errors.len(), 2, "{:?}", r.errors);
    }

    /// req: AI-4, AI-8
    #[test]
    fn locked_tracks_are_restored_and_changes_between_iterations_are_limited() {
        let mut p = project();
        p.track_mut("b").unwrap().locked = true;
        p.tracks.push(Track::new_return("r".into(), "Plate", ReturnFx::default_reverb()));
        p.track_mut("r").unwrap().locked = true;
        let ids = Ids::of(&p);
        let ed = Editable { tracks: vec![ids.n("a")], groups: vec![], returns: vec![] };
        let prev = start(&p, &ids);
        let mut plan = prev.clone();
        plan.tracks.get_mut(&ids.n("b")).unwrap().gain_db = -12.0;
        let r = validate_mix(&mut plan, &p, &ids, &ed, None);
        let n_returns = p.tracks.iter().filter(|t| t.is_return()).count();
        assert!(r.ok() && r.warnings.len() == 1 + n_returns, "{:?}", r);
        assert!(!plan.tracks.contains_key(&ids.n("b")) && plan.returns.is_empty(), "locked track and return dropped");
        let mut next = prev.clone();
        let a = next.tracks.get_mut(&ids.n("a")).unwrap();
        a.gain_db = 9.0;
        // A new band starts from 0 dB: +7 dB is clipped to +3 dB (DEC-5, `_clamp_eq` of the PoC).
        a.eq = vec![EqBand { kind: EqType::Peak, freq_hz: 1000.0, gain_db: 7.0, q: 1.0 }];
        let w = limit_changes(&mut next, &prev);
        assert_eq!(next.tracks[&ids.n("a")].gain_db, 4.0);
        assert_eq!(next.tracks[&ids.n("a")].eq[0].gain_db, 3.0);
        assert_eq!(w.len(), 2);
    }
}
