//! Structural changes of a mix plan (AI-17 … AI-20): the returns and groups a plan creates.
//!
//! A plan names what it creates under `structure`, with placeholder ids (`n1` … `n6`) that the rest of the plan
//! uses like any other id. [`materialise`] turns them into real tracks of the working copy of the project, once:
//! from then on they are ordinary returns and groups, which is how later iterations of the loop see them (AI-19),
//! how the plan is rendered and evaluated, and how the proposal is listened to. [`commands`] creates them again
//! on the project as it is when the author applies the proposal, all of them or only the ticked ones (AI-20).
//!
//! Nothing here creates, duplicates or removes an audio track, removes a return or a group, or moves a track
//! between existing groups: a new return is added at the end, and a new group takes the place and the output of
//! its first member.

use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use stemwise_core::command::Command;
use stemwise_core::model::{MAX_RETURNS, Output, Project, ReturnFx, Track};
use stemwise_core::routing;
use ts_rs::TS;

use crate::analysis_doc::Ids;
use crate::plan::{
    Editable, MAX_NEW_NAME, MixPlan, NEW_GROUP_KEYS, NEW_RETURN_KEYS, NewReturnType, ReturnPlan, Structure,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum NewKind {
    Return,
    Group,
}

/// A return or a group a proposal creates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewElement {
    /// Placeholder id in the plan (`n1` …).
    pub key: String,
    pub kind: NewKind,
    /// The track as created: id, name, and for a return its processor. The plan sets its parameters afterwards.
    pub track: Track,
    /// Why the plan creates it, in the language of the brief.
    pub rationale: String,
    /// Project ids of the members of a group, in project order.
    pub members: Vec<String>,
    /// Project ids of the tracks and groups that send to a return in the proposal's plan.
    pub users: Vec<String>,
    /// The creative idea it belongs to (AI-26): created only if the author ticks the idea.
    #[serde(default)]
    pub idea: Option<String>,
}

impl NewElement {
    /// The tracks it exists for, and how many of them must be applied for it to be created (AI-20): one sender for
    /// a return, two members for a group.
    pub fn depends_on(&self) -> (&[String], usize) {
        match self.kind {
            NewKind::Return => (&self.users, 1),
            NewKind::Group => (&self.members, 2),
        }
    }
}

/// Removes every use of a placeholder id from a plan (a dropped or refused element).
fn strip(plan: &mut MixPlan, key: &str) {
    plan.returns.remove(key);
    plan.groups.remove(key);
    for t in plan.tracks.values_mut() {
        t.sends_db.remove(key);
        for m in t.idea_sends_db.values_mut() {
            m.remove(key);
        }
        for s in t.sections.values_mut() {
            s.send_offsets_db.remove(key);
        }
    }
    for g in plan.groups.values_mut() {
        g.sends_db.remove(key);
        for m in g.idea_sends_db.values_mut() {
            m.remove(key);
        }
    }
}

fn clean_name(name: &str, fallback: &str) -> String {
    let n: String = name.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(MAX_NEW_NAME).collect();
    let n = n.trim().to_string();
    if n.is_empty() { fallback.to_string() } else { n }
}

/// What [`materialise`] may do.
pub struct Allow<'a> {
    /// Whether this plan may bring structure at all (AI-18, AI-19); otherwise any is dropped with a warning.
    pub structure: bool,
    /// Project ids of the tracks and groups the request may change (AI-4, AI-5): only these go into a new group.
    pub editable: &'a HashSet<String>,
    /// A number unique to the call, from which the ids of the new tracks are made: they never collide with the
    /// project's own counter, also when the project changes while the proposal is in review.
    pub stamp: u64,
}

/// Creates in `work` the returns and groups `plan.structure` asks for, extends `ids` and `ed` with them and takes
/// `structure` out of the plan. What cannot be created is dropped with a warning and taken out of the plan too
/// (AI-19): a return nobody sends to, a group with fewer than two valid members, anything beyond the limits.
pub fn materialise(
    work: &mut Project,
    ids: &mut Ids,
    ed: &mut Editable,
    plan: &mut MixPlan,
    allow: &Allow,
) -> (Vec<NewElement>, Vec<String>) {
    let structure = plan.structure.take().unwrap_or_default();
    let mut warnings = Vec::new();
    let mut out = Vec::new();
    let keys = || NEW_RETURN_KEYS.iter().chain(NEW_GROUP_KEYS.iter());
    if !allow.structure {
        if !structure.is_empty() {
            warnings
                .push("the plan asked for new returns or groups, which this request does not allow: ignored".into());
        }
        // A placeholder id used without being created is dropped rather than refused.
        for k in keys().filter(|k| !ids.to_project.contains_key(**k)) {
            strip(plan, k);
        }
        return (out, warnings);
    }
    let Structure { returns, groups } = structure;
    for (k, _) in returns.iter().filter(|(k, _)| !NEW_RETURN_KEYS.contains(&k.as_str())) {
        warnings.push(format!("`{k}` is not a placeholder for a new return (use n1, n2 or n3): ignored"));
    }
    for (k, _) in groups.iter().filter(|(k, _)| !NEW_GROUP_KEYS.contains(&k.as_str())) {
        warnings.push(format!("`{k}` is not a placeholder for a new group (use n4, n5 or n6): ignored"));
    }
    let mut n = 0u64;
    let mut new_id = |prefix: &str| {
        n += 1;
        format!("{prefix}{}", allow.stamp * 10 + n)
    };

    // Groups first: a new return may be fed by a new group only, and is created only if that group is.
    let mut taken: HashSet<String> = HashSet::new();
    for key in NEW_GROUP_KEYS {
        let Some(ng) = groups.get(key) else {
            strip(plan, key);
            continue;
        };
        let mut members: Vec<String> = Vec::new();
        let mut output: Option<Output> = None;
        for m in &ng.members {
            let id = ids.to_project.get(m).filter(|_| !keys().any(|k| k == m));
            let reason = match id.and_then(|id| work.track(id)) {
                None => Some("is not a track or group of the project"),
                Some(t) if t.is_return() => Some("is a return: returns do not go into groups"),
                Some(t) if !allow.editable.contains(&t.id) => Some("is locked or outside the request"),
                Some(t) if taken.contains(&t.id) || members.contains(&t.id) => Some("is already in a new group"),
                Some(t) if output.as_ref().is_some_and(|o| *o != t.output) => {
                    Some("has a different output from the first member")
                }
                Some(t) => {
                    output = Some(t.output.clone());
                    members.push(t.id.clone());
                    None
                }
            };
            if let Some(why) = reason {
                warnings.push(format!("new group `{key}` ({}): member `{m}` {why}: left out", ng.name));
            }
        }
        if members.len() < 2 {
            warnings.push(format!("new group `{key}` ({}) has fewer than two valid members: not created", ng.name));
            strip(plan, key);
            continue;
        }
        let mut group = Track::new_group(new_id("g"), &clean_name(&ng.name, "Group"));
        group.output = output.expect("two members");
        let mut trial = work.clone();
        insert_group(&mut trial, &group, &members);
        if routing::validate(&trial).is_err() {
            warnings.push(format!("new group `{key}` ({}) would nest groups too deep: not created", ng.name));
            strip(plan, key);
            continue;
        }
        *work = trial;
        // Members in project order, as the interface lists them.
        members.sort_by_key(|m| work.track_index(m));
        taken.extend(members.iter().cloned());
        ids.to_neutral.insert(group.id.clone(), key.to_string());
        ids.to_project.insert(key.to_string(), group.id.clone());
        ed.groups.push(key.to_string());
        out.push(NewElement {
            key: key.to_string(),
            kind: NewKind::Group,
            track: group,
            rationale: ng.rationale.trim().to_string(),
            members,
            users: Vec::new(),
            idea: None,
        });
    }
    // Returns.
    for key in NEW_RETURN_KEYS {
        let Some(nr) = returns.get(key) else {
            strip(plan, key);
            continue;
        };
        let sent = plan
            .tracks
            .values()
            .any(|t| t.sends_db.contains_key(key) || t.idea_sends_db.values().any(|m| m.contains_key(key)))
            || plan
                .groups
                .values()
                .any(|g| g.sends_db.contains_key(key) || g.idea_sends_db.values().any(|m| m.contains_key(key)));
        if !sent {
            warnings.push(format!("new return `{key}` ({}) has no track sending to it: not created", nr.name));
            strip(plan, key);
            continue;
        }
        if work.tracks.iter().filter(|t| t.is_return()).count() >= MAX_RETURNS {
            warnings.push(format!("new return `{key}` ({}) would exceed {MAX_RETURNS} returns: not created", nr.name));
            strip(plan, key);
            continue;
        }
        // The parameters must be of the type the structure declares; otherwise the type's defaults stand.
        let fx = match (nr.kind, plan.returns.get(key)) {
            (NewReturnType::Reverb, Some(ReturnPlan::Reverb { algorithm, .. })) => ReturnFx::reverb(*algorithm),
            (NewReturnType::Delay, Some(ReturnPlan::Delay { .. })) => ReturnFx::default_delay(),
            (kind, given) => {
                if given.is_some() {
                    warnings.push(format!("`returns.{key}` is not of the type `structure` declares: defaults used"));
                    plan.returns.remove(key);
                }
                match kind {
                    NewReturnType::Reverb => ReturnFx::default_reverb(),
                    NewReturnType::Delay => ReturnFx::default_delay(),
                }
            }
        };
        let fallback = if fx.is_reverb() { "Reverb" } else { "Delay" };
        let track = Track::new_return(new_id("r"), &clean_name(&nr.name, fallback), fx);
        ids.to_neutral.insert(track.id.clone(), key.to_string());
        ids.to_project.insert(key.to_string(), track.id.clone());
        ed.returns.push(key.to_string());
        work.tracks.push(track.clone());
        out.push(NewElement {
            key: key.to_string(),
            kind: NewKind::Return,
            track,
            rationale: nr.rationale.trim().to_string(),
            members: Vec::new(),
            users: Vec::new(),
            idea: nr.idea.clone(),
        });
    }

    // Returns before groups, as the proposal lists them.
    out.sort_by(|a, b| a.key.cmp(&b.key));
    (out, warnings)
}

/// Puts a new group in the place of its first member and routes the members into it.
fn insert_group(p: &mut Project, group: &Track, members: &[String]) {
    let index = members.iter().filter_map(|m| p.track_index(m)).min().unwrap_or(p.tracks.len());
    p.tracks.insert(index, group.clone());
    for m in members {
        if let Some(t) = p.track_mut(m) {
            t.output = Output::Group(group.id.clone());
        }
    }
}

/// Fills in who sends to each new return in the plan that became the proposal.
pub fn set_users(elements: &mut [NewElement], plan: &MixPlan, ids: &Ids) {
    for e in elements.iter_mut().filter(|e| e.kind == NewKind::Return) {
        e.users = plan
            .tracks
            .iter()
            .filter(|(_, t)| {
                t.sends_db.contains_key(&e.key) || t.idea_sends_db.values().any(|m| m.contains_key(&e.key))
            })
            .map(|(n, _)| n)
            .chain(
                plan.groups
                    .iter()
                    .filter(|(_, g)| {
                        g.sends_db.contains_key(&e.key) || g.idea_sends_db.values().any(|m| m.contains_key(&e.key))
                    })
                    .map(|(n, _)| n),
            )
            .filter_map(|n| ids.to_project.get(n).cloned())
            .collect();
    }
}

/// Takes out of the working project, of the plan and of the list the new returns that the plan which became the
/// proposal no longer sends to (AI-19): a later iteration may have dropped the sends, or the only sender was a locked
/// track whose plan was ignored. Returns one warning per return taken out.
pub fn prune(work: &mut Project, ids: &mut Ids, plan: &mut MixPlan, elements: &mut Vec<NewElement>) -> Vec<String> {
    set_users(elements, plan, ids);
    let mut warnings = Vec::new();
    elements.retain(|e| {
        if e.kind != NewKind::Return || !e.users.is_empty() {
            return true;
        }
        warnings.push(format!("new return `{}` ({}) has no track sending to it: not created", e.key, e.track.name));
        work.tracks.retain(|t| t.id != e.track.id);
        strip(plan, &e.key);
        ids.to_project.remove(&e.key);
        ids.to_neutral.remove(&e.track.id);
        false
    });
    warnings
}

/// Why an element of a proposal is not created in an application (AI-20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skipped {
    /// Its own tick was removed.
    LeftOut,
    /// Too few of the tracks it exists for are applied.
    NotNeeded,
    /// The project changed during the review and it no longer fits (a member gone or routed elsewhere, no room
    /// for another return).
    NoLongerFits,
}

/// The commands that create the elements on the project as it is now, and the elements left out with the reason.
/// `only` is the set of ticked ids (tracks and new elements) of a partial application, `None` for all.
pub fn commands(
    project: &Project,
    elements: &[NewElement],
    only: Option<&HashSet<String>>,
) -> (Vec<Command>, Vec<(String, Skipped)>) {
    let mut work = project.clone();
    let mut cmds = Vec::new();
    let mut skipped = Vec::new();
    let ticked = |id: &str| only.is_none_or(|s| s.contains(id));
    // Groups first: a new return may exist only for a new group that sends to it.
    let ordered = elements
        .iter()
        .filter(|e| e.kind == NewKind::Group)
        .chain(elements.iter().filter(|e| e.kind == NewKind::Return));
    for e in ordered {
        if work.track(&e.track.id).is_some() {
            continue; // already there: the proposal is being applied a second time
        }
        if !ticked(&e.track.id) {
            skipped.push((e.track.id.clone(), Skipped::LeftOut));
            continue;
        }
        match e.kind {
            NewKind::Return => {
                // A sender counts if it is applied: ticked, still in the project (a new group: created) and not locked.
                if !e.users.iter().any(|u| ticked(u) && work.track(u).is_some_and(|t| !t.locked)) {
                    skipped.push((e.track.id.clone(), Skipped::NotNeeded));
                } else if work.tracks.iter().filter(|t| t.is_return()).count() >= MAX_RETURNS {
                    skipped.push((e.track.id.clone(), Skipped::NoLongerFits));
                } else {
                    let c = Command::InsertTrack { index: work.tracks.len(), track: Box::new(e.track.clone()) };
                    if c.clone().apply(&mut work).is_ok() {
                        cmds.push(c);
                    }
                }
            }
            NewKind::Group => {
                let members: Vec<String> = e.members.iter().filter(|m| ticked(m)).cloned().collect();
                if members.len() < 2 {
                    skipped.push((e.track.id.clone(), Skipped::NotNeeded));
                    continue;
                }
                // The members must still be there, unlocked and at the output the group was planned on.
                let fits = members
                    .iter()
                    .all(|m| work.track(m).is_some_and(|t| !t.locked && !t.is_return() && t.output == e.track.output));
                let mut trial = work.clone();
                insert_group(&mut trial, &e.track, &members);
                if !fits || routing::validate(&trial).is_err() {
                    skipped.push((e.track.id.clone(), Skipped::NoLongerFits));
                    continue;
                }
                let index = members.iter().filter_map(|m| work.track_index(m)).min().unwrap_or(work.tracks.len());
                let mut step = vec![Command::InsertTrack { index, track: Box::new(e.track.clone()) }];
                for m in &members {
                    let mut t = work.track(m).expect("checked").clone();
                    t.output = Output::Group(e.track.id.clone());
                    step.push(Command::SetTrack { track: Box::new(t) });
                }
                for c in step {
                    if c.clone().apply(&mut work).is_ok() {
                        cmds.push(c);
                    }
                }
            }
        }
    }
    // In the order of the proposal.
    skipped.sort_by_key(|(id, _)| elements.iter().position(|e| e.track.id == *id));
    (cmds, skipped)
}
