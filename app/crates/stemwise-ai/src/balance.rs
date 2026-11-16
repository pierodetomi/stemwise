//! Balance of a rendered mix (AI-22, A-063): how loud each family of tracks and each return comes out, relative to
//! the whole mix. The spectral distance says how the mix is toned, not who is in front: without these numbers the
//! model sets fifteen drum microphones one by one and never learns that together they cover the vocal.
//!
//! Each level is a solo render of the project as it is (same chains, groups and sidechains as in the mix): a family
//! with the returns muted, so it is the dry signal; a return alone, fed by everything that sends to it.

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use stemwise_analysis::sections::section_lufs;
use stemwise_core::model::Project;
use stemwise_core::role::Role;
use stemwise_engine::offline::render_range;

use crate::analysis_doc::{Ids, r1};

/// Renders running at once: each one streams every clip of its tracks.
const WORKERS: usize = 3;

/// The family a role is balanced with, in the order they are listed to the model.
pub const FAMILIES: [&str; 8] =
    ["lead_vocal", "backing_vocal", "drums", "percussion", "bass", "guitars", "keys", "other"];

pub fn family_of(role: Option<Role>) -> &'static str {
    match role {
        Some(Role::LeadVocal) => "lead_vocal",
        Some(Role::BackingVocal | Role::Choir) => "backing_vocal",
        Some(r) if r.is_drums() => "drums",
        Some(Role::Percussion) => "percussion",
        Some(Role::Bass) => "bass",
        Some(Role::ElectricGuitar | Role::AcousticGuitar) => "guitars",
        Some(Role::Keys | Role::Synth | Role::Pad | Role::Strings | Role::Brass | Role::Piano | Role::Woodwinds) => {
            "keys"
        }
        _ => "other",
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct FamilyLevel {
    pub name: String,
    /// Project ids of the audio tracks of the family.
    pub tracks: Vec<String>,
    /// Integrated loudness of the family alone minus that of the mix (LU); `None` when it is silent.
    pub level_lu: Option<f64>,
    /// The same inside each section, in time order; `None` where the family or the mix is silent.
    pub section_lu: Vec<Option<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct ReturnLevel {
    /// Project id of the return.
    pub id: String,
    pub level_lu: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Balance {
    pub families: Vec<FamilyLevel>,
    pub returns: Vec<ReturnLevel>,
}

/// Loudness of one section of the mix (ANA-20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct SectionLevel {
    /// Project id of the section.
    pub id: String,
    /// Integrated loudness of the mix inside the section (LUFS); `None` when it is silent or too short.
    pub lufs: Option<f64>,
}

/// ANA-20: the levels the Explain view reads (P-019). Families and returns are relative to the mix, as in the
/// balance of AI-22; `section_lu` of a family follows `sections`, which are in time order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct MixLevels {
    /// Integrated loudness of the whole mix (LUFS).
    pub mix_lufs: Option<f64>,
    pub sections: Vec<SectionLevel>,
    pub families: Vec<FamilyLevel>,
    pub returns: Vec<ReturnLevel>,
}

/// What one render is for.
enum Job {
    Mix,
    Family(usize),
    Return(usize),
}

/// The project with only the tracks of a job audible. Mutes set by the author stay; solos do not.
fn variant(p: &Project, audible: &[String], returns_muted: bool) -> Project {
    let mut q = p.clone();
    for t in q.tracks.iter_mut() {
        t.solo = audible.contains(&t.id);
        if returns_muted && t.is_return() {
            t.mute = true;
        }
    }
    q
}

/// Loudness of the whole render and of each section; silence and too-short spans are `None`.
fn levels(
    package: &Path,
    p: &Project,
    sections: &[(i64, i64)],
    end: i64,
    cancel: &AtomicBool,
) -> Option<Vec<Option<f64>>> {
    let r = render_range(package, p, 0, end, 4096, &mut |_| !cancel.load(Ordering::Relaxed)).ok()?;
    let x = r.interleaved();
    let mut spans = vec![(0, end)];
    spans.extend_from_slice(sections);
    Some(section_lufs(&x, 2, r.rate, &spans).into_iter().map(|v| v.filter(|l| l.is_finite() && *l > -70.0)).collect())
}

/// Measures the balance of the project as it is. `None` when a render fails or the call is cancelled.
pub fn measure_balance(package: &Path, p: &Project, cancel: &AtomicBool) -> Option<Balance> {
    measure_levels(package, p, cancel).map(|l| Balance { families: l.families, returns: l.returns })
}

/// ANA-20: the balance of the project as it is, with the loudness of the mix and of each of its sections. `None`
/// when a render fails or the call is cancelled.
pub fn measure_levels(package: &Path, p: &Project, cancel: &AtomicBool) -> Option<MixLevels> {
    let rate = p.sample_rate?;
    let end = p.content_end() + i64::from(rate) * 2;
    let mut sorted: Vec<&stemwise_core::model::Section> = p.sections.iter().collect();
    sorted.sort_by_key(|s| s.start);
    let secs: Vec<(i64, i64)> = sorted.iter().map(|s| (s.start, s.end)).collect();
    let mut families: Vec<FamilyLevel> = FAMILIES
        .iter()
        .map(|name| FamilyLevel {
            name: (*name).to_string(),
            tracks: p
                .tracks
                .iter()
                .filter(|t| t.is_audio() && !t.mute && !t.clips.is_empty() && family_of(t.role) == *name)
                .map(|t| t.id.clone())
                .collect(),
            level_lu: None,
            section_lu: Vec::new(),
        })
        .filter(|f| !f.tracks.is_empty())
        .collect();
    let mut returns: Vec<ReturnLevel> = p
        .tracks
        .iter()
        .filter(|t| t.is_return() && !t.mute)
        .map(|t| ReturnLevel { id: t.id.clone(), level_lu: None })
        .collect();
    let mut jobs = vec![(Job::Mix, variant(p, &[], false))];
    jobs.extend(families.iter().enumerate().map(|(i, f)| (Job::Family(i), variant(p, &f.tracks, true))));
    jobs.extend(
        returns.iter().enumerate().map(|(i, r)| (Job::Return(i), variant(p, std::slice::from_ref(&r.id), false))),
    );
    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<Option<Vec<Option<f64>>>>> = Mutex::new(vec![None; jobs.len()]);
    std::thread::scope(|s| {
        for _ in 0..WORKERS.min(jobs.len()) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= jobs.len() || cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let l = levels(package, &jobs[i].1, &secs, end, cancel);
                    if let Ok(mut d) = done.lock() {
                        d[i] = l;
                    }
                }
            });
        }
    });
    let done = done.into_inner().ok()?;
    if cancel.load(Ordering::Relaxed) || done.iter().any(Option::is_none) {
        return None;
    }
    let mix = done[0].clone()?;
    let rel = |v: Option<f64>, m: Option<f64>| Some(v? - m?);
    for ((job, _), l) in jobs.iter().zip(done) {
        let l = l?;
        match job {
            Job::Mix => {}
            Job::Family(i) => {
                families[*i].level_lu = rel(l[0], mix[0]);
                families[*i].section_lu = l.iter().zip(&mix).skip(1).map(|(v, m)| rel(*v, *m)).collect();
            }
            Job::Return(i) => returns[*i].level_lu = rel(l[0], mix[0]),
        }
    }
    let sections =
        sorted.iter().zip(mix.iter().skip(1)).map(|(s, l)| SectionLevel { id: s.id.clone(), lufs: *l }).collect();
    Some(MixLevels { mix_lufs: mix[0], sections, families, returns })
}

/// The balance as the model reads it: neutral ids, one decimal.
pub fn balance_json(b: &Balance, ids: &Ids) -> Value {
    let o = |v: Option<f64>| v.map_or(Value::Null, |x| json!(r1(x)));
    let mut families = Map::new();
    for f in &b.families {
        let mut v = json!({"tracks": f.tracks.iter().map(|t| ids.n(t)).collect::<Vec<_>>(), "level_lu": o(f.level_lu)});
        if !f.section_lu.is_empty() {
            v["section_lu"] = f.section_lu.iter().map(|x| o(*x)).collect();
        }
        families.insert(f.name.clone(), v);
    }
    let returns: Map<String, Value> = b.returns.iter().map(|r| (ids.n(&r.id), o(r.level_lu))).collect();
    json!({"families": families, "returns_lu": returns})
}
