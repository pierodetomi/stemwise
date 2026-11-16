//! Creative ideas in a plan (AI-26, AI-27): an idea is a set of additions marked with its id (blocks, sends, the
//! offsets of a section, a new return). Taking an idea out leaves the rest of the plan exactly as it is, so the author
//! ticks ideas one by one and the loop renders and evaluates the plan without them.

use std::collections::{BTreeMap, HashSet};

use serde_json::{Value, json};

use crate::plan::{BlockPlan, GroupPlan, MAX_NEW_BLOCKS, MixPlan, SectionPlan, TrackPlan};

fn kept(idea: &Option<String>, keep: &HashSet<String>) -> bool {
    idea.as_ref().is_none_or(|i| keep.contains(i))
}

/// The plan with only the ideas in `keep`: the elements of the others removed, the sends of the kept ones turned
/// into ordinary sends. `idea_returns` are the placeholder ids of new returns that belong to an idea (AI-17).
pub fn strip(plan: &MixPlan, keep: &HashSet<String>, idea_returns: &[(String, String)]) -> MixPlan {
    let mut p = plan.clone();
    if let Some(m) = &mut p.ideas {
        m.retain(|k, _| keep.contains(k));
    }
    let dropped: Vec<&String> = idea_returns.iter().filter(|(_, i)| !keep.contains(i)).map(|(k, _)| k).collect();
    for k in &dropped {
        p.returns.remove(*k);
    }
    if let Some(st) = &mut p.structure {
        st.returns.retain(|_, r| kept(&r.idea, keep));
    }
    let fix_sends = |sends: &mut BTreeMap<String, f64>, idea: &mut BTreeMap<String, BTreeMap<String, f64>>| {
        for (i, m) in std::mem::take(idea) {
            if keep.contains(&i) {
                for (r, v) in m {
                    sends.entry(r).or_insert(v);
                }
            }
        }
        sends.retain(|r, _| !dropped.contains(&r));
    };
    for t in p.tracks.values_mut() {
        t.blocks.retain(|b| kept(&b.idea, keep));
        t.sections.retain(|_, s| kept(&s.idea, keep));
        // An idea's offsets for a section the plan also uses: added to the plan's own when the idea is kept.
        for (s, sp) in std::mem::take(&mut t.idea_sections) {
            if kept(&sp.idea, keep) {
                match t.sections.get_mut(&s) {
                    Some(mine) => fold(mine, sp),
                    None => {
                        t.sections.insert(s, sp);
                    }
                }
            }
        }
        for s in t.sections.values_mut() {
            s.send_offsets_db.retain(|r, _| !dropped.contains(&r));
        }
        fix_sends(&mut t.sends_db, &mut t.idea_sends_db);
    }
    for g in p.groups.values_mut() {
        g.blocks.retain(|b| kept(&b.idea, keep));
        fix_sends(&mut g.sends_db, &mut g.idea_sends_db);
    }
    for r in p.returns.values_mut() {
        r.blocks_mut().retain(|b| kept(&b.idea, keep));
    }
    p
}

/// Takes the ideas out of a plan (the loop's first iteration, AI-26): returns them as a plan of their own (the ideas,
/// their blocks, sections, sends and new returns) and leaves the rest.
pub fn take(plan: &mut MixPlan, idea_returns: &[(String, String)]) -> MixPlan {
    let ideas = plan.ideas.take();
    let mut out = MixPlan {
        version: plan.version,
        iteration: plan.iteration,
        summary: String::new(),
        structure: None,
        ideas,
        returns: BTreeMap::new(),
        tracks: BTreeMap::new(),
        groups: BTreeMap::new(),
    };
    for (k, _) in idea_returns {
        if let Some(r) = plan.returns.remove(k) {
            out.returns.insert(k.clone(), r);
        }
    }
    for (n, t) in plan.tracks.iter_mut() {
        let blocks: Vec<_> = t.blocks.iter().filter(|b| b.idea.is_some()).cloned().collect();
        t.blocks.retain(|b| b.idea.is_none());
        let sections: BTreeMap<_, _> =
            t.sections.iter().filter(|(_, s)| s.idea.is_some()).map(|(k, v)| (k.clone(), v.clone())).collect();
        t.sections.retain(|_, s| s.idea.is_none());
        let sends = std::mem::take(&mut t.idea_sends_db);
        if !blocks.is_empty() || !sections.is_empty() || !sends.is_empty() {
            out.tracks.insert(n.clone(), TrackPlan { blocks, sections, idea_sends_db: sends, ..t.clone() });
        }
    }
    for (n, g) in plan.groups.iter_mut() {
        let blocks: Vec<_> = g.blocks.iter().filter(|b| b.idea.is_some()).cloned().collect();
        g.blocks.retain(|b| b.idea.is_none());
        let sends = std::mem::take(&mut g.idea_sends_db);
        if !blocks.is_empty() || !sends.is_empty() {
            out.groups.insert(n.clone(), GroupPlan { blocks, idea_sends_db: sends, ..g.clone() });
        }
    }
    for (n, r) in plan.returns.iter_mut() {
        let blocks: Vec<_> = r.blocks().iter().filter(|b| b.idea.is_some()).cloned().collect();
        r.blocks_mut().retain(|b| b.idea.is_none());
        if !blocks.is_empty() {
            let mut copy = r.clone();
            *copy.blocks_mut() = blocks;
            out.returns.entry(n.clone()).or_insert(copy);
        }
    }
    out
}

/// Adds an idea's offsets of a section to the plan's own offsets of that section.
fn fold(a: &mut SectionPlan, b: SectionPlan) {
    a.gain_offset_db += b.gain_offset_db;
    for (r, v) in b.send_offsets_db {
        *a.send_offsets_db.entry(r).or_insert(0.0) += v;
    }
    for (i, v) in b.eq_gain_offsets_db.into_iter().enumerate() {
        match a.eq_gain_offsets_db.get_mut(i) {
            Some(x) => *x += v,
            None => a.eq_gain_offsets_db.push(v),
        }
    }
    a.hpf_offset_oct = a.hpf_offset_oct.or(b.hpf_offset_oct);
    a.lpf_offset_oct = a.lpf_offset_oct.or(b.lpf_offset_oct);
    for x in b.blocks_on {
        if !a.blocks_on.contains(&x) {
            a.blocks_on.push(x);
        }
    }
}

/// Adds an idea's blocks to a chain of the plan kept by the loop: a block id the plan now uses is renamed, a block
/// beyond the new blocks a chain may get in one proposal (AI-25) is left out with a warning. Returns the renames
/// (an empty name: left out).
fn add_blocks(
    mine: &mut Vec<BlockPlan>,
    theirs: &[BlockPlan],
    at: &str,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, String> {
    let mut renamed = BTreeMap::new();
    for b in theirs {
        let fresh = mine.iter().filter(|y| y.position.is_some()).count();
        if b.position.is_some() && fresh >= MAX_NEW_BLOCKS {
            warnings.push(format!(
                "{at}: the block `{}` of idea {} is left out, the chain already gets {MAX_NEW_BLOCKS} new blocks",
                b.id,
                b.idea.as_deref().unwrap_or("?")
            ));
            renamed.insert(b.id.clone(), String::new());
            continue;
        }
        let mut b = b.clone();
        if mine.iter().any(|y| y.id == b.id) {
            let free = (2..).map(|k| format!("{}{k}", b.id)).find(|n| !mine.iter().any(|y| &y.id == n)).expect("free");
            renamed.insert(b.id.clone(), free.clone());
            b.id = free;
        }
        mine.push(b);
    }
    renamed
}

/// Puts ideas taken by [`take`] back into the plan the loop kept (AI-26: ideas stay fixed through the loop).
/// Elements whose track the plan no longer has are left out. Nothing the plan decided is replaced: block ids that
/// collide are renamed, an idea's offsets for a section the plan also uses are kept apart (`idea_sections`).
/// Returns the warnings of what was left out.
pub fn merge(plan: &mut MixPlan, ideas: &MixPlan) -> Vec<String> {
    let mut warnings = Vec::new();
    if ideas.ideas.as_ref().is_none_or(|m| m.is_empty()) {
        return warnings;
    }
    plan.ideas = ideas.ideas.clone();
    for (k, r) in &ideas.returns {
        match plan.returns.get_mut(k) {
            Some(mine) => {
                add_blocks(mine.blocks_mut(), r.blocks(), &format!("returns.{k}"), &mut warnings);
            }
            None => {
                plan.returns.insert(k.clone(), r.clone());
            }
        }
    }
    for (n, x) in &ideas.tracks {
        let Some(t) = plan.tracks.get_mut(n) else { continue };
        let renamed = add_blocks(&mut t.blocks, &x.blocks, &format!("tracks.{n}"), &mut warnings);
        for (s, sp) in &x.sections {
            let mut sp = sp.clone();
            sp.blocks_on = sp
                .blocks_on
                .iter()
                .filter_map(|id| match renamed.get(id) {
                    Some(r) if r.is_empty() => None,
                    Some(r) => Some(r.clone()),
                    None => Some(id.clone()),
                })
                .collect();
            if t.sections.contains_key(s) {
                t.idea_sections.insert(s.clone(), sp);
            } else {
                t.sections.insert(s.clone(), sp);
            }
        }
        for (i, m) in &x.idea_sends_db {
            t.idea_sends_db.insert(i.clone(), m.clone());
        }
    }
    for (n, x) in &ideas.groups {
        let Some(g) = plan.groups.get_mut(n) else { continue };
        add_blocks(&mut g.blocks, &x.blocks, &format!("groups.{n}"), &mut warnings);
        for (i, m) in &x.idea_sends_db {
            g.idea_sends_db.insert(i.clone(), m.clone());
        }
    }
    warnings
}

/// After [`merge`]: an idea's block that no longer fits its chain (the core's limits, FX-10: the plan the loop kept
/// may have used the room) is left out with a warning, so the proposal never lists a block it cannot add (AI-25).
pub fn fit(plan: &mut MixPlan, p: &stemwise_core::model::Project, ids: &crate::analysis_doc::Ids) -> Vec<String> {
    use stemwise_core::fx::{Fx, FxBlock};
    let mut warnings = Vec::new();
    let mut fit_chain = |owner: &str, at: String, blocks: &mut Vec<BlockPlan>| -> Vec<String> {
        let Some(t) = ids.to_project.get(owner).and_then(|id| p.track(id)) else { return Vec::new() };
        let mut chain = t.chain.clone();
        let mut out = Vec::new();
        let is_new = |b: &BlockPlan| !ids.blocks.contains_key(&b.id);
        // The plan's own blocks first: they were validated and take their room.
        for pass_ideas in [false, true] {
            blocks.retain(|b| {
                if !is_new(b) || b.idea.is_some() != pass_ideas {
                    return true;
                }
                if chain.can_add(b.kind, t.is_return()).is_ok() {
                    let id = chain.free_id(b.kind);
                    chain.blocks.push(FxBlock { id, fx: Fx::new(b.kind) });
                    return true;
                }
                if pass_ideas {
                    warnings.push(format!(
                        "{at}: the block `{}` of idea {} is left out, the chain has no room for it",
                        b.id,
                        b.idea.as_deref().unwrap_or("?")
                    ));
                    out.push(b.id.clone());
                    return false;
                }
                true
            });
        }
        out
    };
    for (n, t) in plan.tracks.iter_mut() {
        let out = fit_chain(n, format!("tracks.{n}"), &mut t.blocks);
        for s in t.sections.values_mut().chain(t.idea_sections.values_mut()) {
            s.blocks_on.retain(|b| !out.contains(b));
        }
    }
    for (n, g) in plan.groups.iter_mut() {
        fit_chain(n, format!("groups.{n}"), &mut g.blocks);
    }
    for (n, r) in plan.returns.iter_mut() {
        fit_chain(n, format!("returns.{n}"), r.blocks_mut());
    }
    warnings
}

/// What the later iterations are told about the ideas already decided (§5.5): fixed, ticked by the author apart;
/// their blocks and sections are theirs.
pub fn context(ideas: &MixPlan) -> Value {
    let Some(m) = &ideas.ideas else { return Value::Null };
    let mut out = serde_json::Map::new();
    for (id, idea) in m {
        let mut tracks = serde_json::Map::new();
        for (n, t) in &ideas.tracks {
            let blocks: Vec<&String> =
                t.blocks.iter().filter(|b| b.idea.as_deref() == Some(id.as_str())).map(|b| &b.id).collect();
            let sections: Vec<&String> =
                t.sections.iter().filter(|(_, s)| s.idea.as_deref() == Some(id.as_str())).map(|(k, _)| k).collect();
            let sends = t.idea_sends_db.get(id).map(|m| m.keys().collect::<Vec<_>>()).unwrap_or_default();
            if !blocks.is_empty() || !sections.is_empty() || !sends.is_empty() {
                tracks.insert(n.clone(), json!({"blocks": blocks, "sections": sections, "sends_to": sends}));
            }
        }
        out.insert(id.clone(), json!({"name": idea.name, "tracks": tracks}));
    }
    Value::Object(out)
}

/// For each track or group (plan id) an idea in `keep` adds something to: the (name, rationale) of those ideas, in
/// idea order (UI-13).
pub fn touched(plan: &MixPlan, keep: &HashSet<String>) -> BTreeMap<String, Vec<(String, String)>> {
    let mut out: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let Some(ideas) = &plan.ideas else { return out };
    for (id, idea) in ideas.iter().filter(|(k, _)| keep.contains(*k)) {
        let mine = |b: &Option<String>| b.as_deref() == Some(id.as_str());
        let mut add = |n: &String| out.entry(n.clone()).or_default().push((idea.name.clone(), idea.rationale.clone()));
        for (n, t) in &plan.tracks {
            if t.blocks.iter().any(|b| mine(&b.idea))
                || t.sections.values().any(|s| mine(&s.idea))
                || t.idea_sends_db.contains_key(id)
            {
                add(n);
            }
        }
        for (n, g) in &plan.groups {
            if g.blocks.iter().any(|b| mine(&b.idea)) || g.idea_sends_db.contains_key(id) {
                add(n);
            }
        }
    }
    out
}

/// What an idea adds, in words for the proposal (AI-27, design 4c): one line per element.
pub fn elements(plan: &MixPlan, idea: &str, name_of: &dyn Fn(&str) -> String) -> Vec<String> {
    let mut out = Vec::new();
    for (n, t) in &plan.tracks {
        for b in t.blocks.iter().filter(|b| b.idea.as_deref() == Some(idea)) {
            out.push(format!("{} · {} (new)", name_of(n), b.kind.label()));
        }
        for (s, sp) in t.sections.iter().chain(&t.idea_sections).filter(|(_, s)| s.idea.as_deref() == Some(idea)) {
            let mut what = Vec::new();
            if let Some(o) = sp.lpf_offset_oct {
                what.push(format!("low-pass {o:+.1} oct"));
            }
            if let Some(o) = sp.hpf_offset_oct {
                what.push(format!("high-pass {o:+.1} oct"));
            }
            if sp.gain_offset_db != 0.0 {
                what.push(format!("level {:+.1} dB", sp.gain_offset_db));
            }
            if !sp.blocks_on.is_empty() {
                what.push("on only".into());
            }
            for (r, v) in &sp.send_offsets_db {
                what.push(format!("send to {} {v:+.1} dB", name_of(r)));
            }
            out.push(format!("{} · {} in {}", name_of(n), what.join(", "), name_of(s)));
        }
        if let Some(m) = t.idea_sends_db.get(idea) {
            for (r, v) in m {
                out.push(format!("{} · send to {} {v:.1} dB", name_of(n), name_of(r)));
            }
        }
    }
    for (n, g) in &plan.groups {
        for b in g.blocks.iter().filter(|b| b.idea.as_deref() == Some(idea)) {
            out.push(format!("{} · {} (new)", name_of(n), b.kind.label()));
        }
    }
    for (n, r) in &plan.returns {
        for b in r.blocks().iter().filter(|b| b.idea.as_deref() == Some(idea)) {
            out.push(format!("{} · {} (new)", name_of(n), b.kind.label()));
        }
    }
    if let Some(st) = &plan.structure {
        for (_, r) in st.returns.iter().filter(|(_, r)| r.idea.as_deref() == Some(idea)) {
            out.push(format!("new return · {}", r.name));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(v: Value) -> MixPlan {
        serde_json::from_value(v).unwrap()
    }

    fn track(blocks: Value, sections: Value) -> Value {
        json!({"gain_db": 0.0, "pan": 0.0, "hpf_hz": null, "lpf_hz": null, "eq": [], "compressor": null,
            "saturation": null, "sends_db": {}, "sections": sections, "blocks": blocks, "rationale": "r"})
    }

    /// req: AI-26
    #[test]
    fn merging_never_replaces_what_the_kept_plan_decided_and_taking_the_idea_out_leaves_it() {
        let new = |id: &str, idea: Option<&str>| json!({"id": id, "type": "modulation", "idea": idea, "position": "last", "bypass": null});
        // The first iteration: idea i1 adds `ch` to t1 and switches it on in s1.
        let mut first = plan(
            json!({"version": 4, "iteration": 0, "summary": "", "structure": null, "returns": {}, "groups": {},
            "ideas": {"i1": {"name": "Chorus", "sections": ["s1"], "rationale": "r"}},
            "tracks": {"t1": track(json!([new("ch", Some("i1"))]), json!({"s1": {"gain_offset_db": 0.0, "send_offsets_db": {},
                "eq_gain_offsets_db": [], "idea": "i1", "blocks_on": ["ch"]}}))}}),
        );
        let part = take(&mut first, &[]);
        // The kept plan (a later iteration) uses `ch` itself, an offset of its own in s1 and two other new blocks.
        let mut kept =
            plan(json!({"version": 4, "iteration": 2, "summary": "", "structure": null, "returns": {}, "groups": {},
            "tracks": {"t1": track(json!([new("ch", None), new("a", None)]), json!({"s1": {"gain_offset_db": 1.5,
                "send_offsets_db": {}, "eq_gain_offsets_db": [], "idea": null, "blocks_on": []}}))}}));
        let before = kept.clone();
        assert!(merge(&mut kept, &part).is_empty());
        let t = &kept.tracks["t1"];
        // The idea's block is renamed, its section kept apart and pointing at the renamed block.
        assert_eq!(t.blocks.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["ch", "a", "ch2"]);
        assert_eq!(t.sections["s1"].gain_offset_db, 1.5);
        assert_eq!(t.idea_sections["s1"].blocks_on, ["ch2"]);
        // Unticked: the plan exactly as the loop kept it. Ticked: the plan's offset and the idea's block, together.
        let mut off = strip(&kept, &HashSet::new(), &[]);
        off.ideas = None;
        assert_eq!(off.tracks, before.tracks);
        let on = strip(&kept, &HashSet::from(["i1".to_string()]), &[]);
        let s1 = &on.tracks["t1"].sections["s1"];
        assert!(s1.gain_offset_db == 1.5 && s1.blocks_on == ["ch2"]);
        // A fourth new block in the chain is left out, with a warning.
        let mut full = before.clone();
        full.tracks.get_mut("t1").unwrap().blocks.push(serde_json::from_value(new("b", None)).unwrap());
        let w = merge(&mut full, &part);
        assert!(w.len() == 1 && w[0].contains("left out"), "{w:?}");
        assert!(full.tracks["t1"].idea_sections["s1"].blocks_on.is_empty());
    }
}
