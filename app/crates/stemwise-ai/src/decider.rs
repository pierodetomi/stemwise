//! The `Decider` interface (AI-9) and the fake decider ("Offline demo"): deterministic rules on roles, analysis,
//! sections and tempo, with no network. It is a development and test tool, not a mixer: the numbers are plausible.
//!
//! Rules of the fake mix decider (extending the PoC's D-043, D-057, D-065):
//! - gain by role; a drum family split over n stems gets a further −10·log10(n) dB; pan: kick, snare, bass, lead
//!   vocal and stereo stems centred, other mono stems alternate −0.5 / +0.5 within their role;
//! - high-pass by role; a −2 dB peak on the lower-priority stem of each strong collision with a priority stem;
//! - compressors on lead vocal and bass; the bass is keyed by the kick when both exist (sidechain);
//! - light saturation on bass and electric guitars; group gains (drums −1 dB);
//! - returns: reverb at fixed settings, delay at a dotted eighth of the tempo (375 ms without one); sends by role;
//! - sections: the lead vocal +1.5 dB in sections named chorus/ritornello, +3 dB of reverb send in a bridge;
//! - iteration ≥ 1: one EQ correction towards the target in the band with the largest difference (D-057);
//! - a request: keyword rules clause by clause (gain, reverb or delay sends; Italian and English words).
//!
//! Rationales and summary follow the language of the brief (Italian or English, AI-13).

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

use serde_json::Value;
use stemwise_core::fx::SatCharacter;
use stemwise_core::model::{Brief, Project, ReverbAlgorithm};
use stemwise_core::role::Role;

use crate::analysis_doc::Ids;
use crate::plan::*;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Mix,
    Master,
}

/// Everything a decision may use. The model sees only `messages`; the fake decider reads the structured fields.
#[derive(Debug, Clone)]
pub struct DecisionInput {
    pub kind: Kind,
    pub iteration: u32,
    pub brief: Brief,
    /// The `analysis` v2 document (mix) or the master analysis document.
    pub analysis: Value,
    pub project: Project,
    pub ids: Ids,
    /// Last iterations as (plan, evaluation), oldest first.
    pub history: Vec<(Value, Value)>,
    pub request: Option<String>,
    /// The starting plan: previous iteration or current state (revision).
    pub start: Option<Value>,
    /// Neutral ids the decision may change (AI-4, AI-5).
    pub editable: Vec<String>,
    /// Whether this decision may create returns and groups (AI-17 … AI-19).
    pub allow_structure: bool,
    /// Whether this decision may propose creative ideas (AI-26, AI-27: the first iteration, when allowed).
    pub allow_ideas: bool,
    /// Whether the chains of returns may change (AI-25: a return is shared; structural changes allowed, no scope).
    pub allow_return_blocks: bool,
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Usage {
    pub tokens_in: u64,
    pub tokens_out: u64,
    /// Cost reported by the provider (AI-15); `None` when the response does not carry it.
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawDecision {
    pub content: String,
    pub usage: Usage,
    pub model: String,
    /// The raw response body, for the log (AI-10).
    pub raw: String,
    /// The JSON body of the request, for the log.
    pub request: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DecideError {
    #[error("no API key: add your OpenRouter key in Settings › AI, or use the offline demo")]
    NoKey,
    #[error("network unavailable: {0}")]
    Network(String),
    #[error("the provider answered with an error ({status}): {message}")]
    Provider { status: u16, message: String, usage: Usage, exchanges: Vec<Exchange> },
    /// The answer stopped before the plan was complete (`finish_reason: "length"`), also after the retry in JSON mode.
    #[error(
        "the answer was cut off before the plan was complete: the model stopped after {} output tokens (cap sent: {}). \
         The provider's full answer is in the call's log",
        usage.tokens_out,
        cap.map_or_else(|| "none".to_string(), |c| c.to_string())
    )]
    Truncated { usage: Usage, cap: Option<u32>, exchanges: Vec<Exchange> },
    #[error("cancelled")]
    Cancelled,
    /// Cancelled while the provider was answering: the request may be billed (A-031: counted at the estimate).
    #[error("cancelled")]
    Abandoned,
    #[error("{0}")]
    Other(String),
}

/// One HTTP exchange with the provider, kept for the log of a failed call (AI-10): the request body as sent (the key
/// travels only in the header) and the response body as received.
#[derive(Debug, Clone, PartialEq)]
pub struct Exchange {
    pub request: String,
    pub status: u16,
    pub response: String,
}

impl Usage {
    /// Adds what another request of the same call spent.
    pub fn add(&mut self, other: &Usage) {
        self.tokens_in += other.tokens_in;
        self.tokens_out += other.tokens_out;
        self.cost_usd = match (self.cost_usd, other.cost_usd) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0.0) + b.unwrap_or(0.0)),
        };
    }
}

impl DecideError {
    /// What the provider reported as spent by a failed call (AI-15).
    pub fn usage(&self) -> Option<&Usage> {
        match self {
            DecideError::Provider { usage, .. } | DecideError::Truncated { usage, .. } => Some(usage),
            _ => None,
        }
    }

    /// The content of the log file of a failed attempt (AI-10): the messages, the error and every exchange with the
    /// provider, each with the request as sent and the response as received (parsed when it is JSON).
    pub fn log_entry(&self, messages: &[Message]) -> String {
        let exchanges = match self {
            DecideError::Provider { exchanges, .. } | DecideError::Truncated { exchanges, .. } => exchanges.as_slice(),
            _ => &[],
        };
        let parsed = |s: &str| serde_json::from_str::<serde_json::Value>(s).unwrap_or_else(|_| s.into());
        let list: Vec<serde_json::Value> = exchanges
            .iter()
            .map(|x| {
                serde_json::json!({"request": parsed(&x.request), "status": x.status, "response": parsed(&x.response)})
            })
            .collect();
        serde_json::json!({"messages": messages, "error": self.to_string(), "exchanges": list}).to_string()
    }

    /// The last response body received from the provider, if any.
    pub fn last_response(&self) -> Option<&str> {
        match self {
            DecideError::Provider { exchanges, .. } | DecideError::Truncated { exchanges, .. } => {
                exchanges.last().map(|x| x.response.as_str())
            }
            _ => None,
        }
    }

    /// The request may have reached the provider and been billed (AI-15: a call counts with what it spent).
    pub fn maybe_billed(&self) -> bool {
        matches!(self, DecideError::Abandoned | DecideError::Network(_))
    }
}

/// A retry after an invalid answer (AI-8): the answer goes back as the assistant's turn, the errors as the user's.
#[derive(Debug, Clone, PartialEq)]
pub struct Retry {
    pub previous: String,
    pub errors: String,
}

/// Usage of a request that got no answer: the prompt's tokens estimated from its length (A-031).
pub fn prompt_estimate(messages: &[Message]) -> Usage {
    let chars: usize = messages.iter().map(|m| m.content.chars().count()).sum();
    Usage { tokens_in: (chars / 4) as u64, tokens_out: 0, cost_usd: None }
}

pub trait Decider: Send + Sync {
    /// Name shown in the AI usage panel ("openrouter", "offline-demo").
    fn name(&self) -> &str;
    fn model(&self) -> String;
    fn effort(&self) -> String;
    /// One call. `retry` carries the previous invalid answer and its validation errors (AI-8).
    fn decide(
        &self,
        input: &DecisionInput,
        messages: &[Message],
        schema: &Value,
        retry: Option<&Retry>,
        cancel: &AtomicBool,
    ) -> Result<RawDecision, DecideError>;
}

// ---------------------------------------------------------------------------------------------
// Fake decider
// ---------------------------------------------------------------------------------------------

pub struct FakeDecider;

const IT_WORDS: [&str; 16] = [
    "il", "la", "le", "di", "che", "più", "voce", "non", "con", "per", "una", "caldo", "batteria", "chitarre", "deve",
    "stare",
];

/// "it" when the brief (or request) reads Italian, else "en".
pub fn language(brief: &Brief, request: Option<&str>) -> &'static str {
    let text = format!("{} {} {} {}", brief.genre, brief.mood, brief.notes, request.unwrap_or("")).to_lowercase();
    let hits = text.split(|c: char| !c.is_alphanumeric()).filter(|w| IT_WORDS.contains(w)).count();
    if hits >= 2 { "it" } else { "en" }
}

fn role_gain(r: Role) -> f64 {
    match r {
        Role::Kick | Role::Snare | Role::Drums | Role::Bass => 0.0,
        Role::Toms | Role::Overheads | Role::Room => -2.0,
        Role::Percussion => -4.0,
        Role::ElectricGuitar
        | Role::AcousticGuitar
        | Role::Keys
        | Role::Synth
        | Role::Strings
        | Role::Brass
        | Role::Piano
        | Role::Woodwinds => -2.0,
        Role::Pad | Role::Fx => -6.0,
        Role::LeadVocal => 2.0,
        Role::BackingVocal | Role::Choir => -4.0,
        Role::Other => -3.0,
    }
}

fn role_hpf(r: Role) -> Option<f64> {
    match r {
        Role::Kick | Role::Drums | Role::Bass => None,
        Role::Snare | Role::Keys => Some(80.0),
        // A piano's lowest notes are music: no high-pass.
        Role::Piano => None,
        Role::Toms | Role::Synth => Some(60.0),
        Role::Overheads | Role::Room | Role::Percussion | Role::Pad | Role::BackingVocal | Role::Fx => Some(120.0),
        Role::ElectricGuitar | Role::AcousticGuitar | Role::LeadVocal => Some(90.0),
        Role::Strings | Role::Brass | Role::Woodwinds | Role::Choir => Some(100.0),
        Role::Other => Some(80.0),
    }
}

fn reverb_send(r: Role) -> Option<f64> {
    match r {
        Role::LeadVocal => Some(-20.0),
        Role::BackingVocal => Some(-18.0),
        Role::Snare => Some(-26.0),
        Role::ElectricGuitar | Role::AcousticGuitar | Role::Keys => Some(-24.0),
        Role::Pad | Role::Strings => Some(-22.0),
        _ => None,
    }
}

const PRIORITY: [Role; 3] = [Role::LeadVocal, Role::Bass, Role::Kick];

fn band_centre(b: usize) -> f64 {
    let (lo, hi) = stemwise_analysis::spectral::BANDS_HZ[b];
    (lo * hi).sqrt().round()
}

fn texts(lang: &str) -> [&'static str; 8] {
    if lang == "it" {
        [
            "Livello e pan secondo il ruolo; filtro passa-alto per togliere il rimbombo.",
            "Taglio di 2 dB dove copre una traccia prioritaria.",
            "Compressione leggera per tenere il livello costante.",
            "Si abbassa sotto la cassa (sidechain) per lasciarle spazio.",
            "Un po' di saturazione per dare corpo.",
            "Più presente nei ritornelli.",
            "Bilanciamento iniziale per ruolo, con spazio per la voce e un ambiente condiviso.",
            "Gruppo leggermente più basso per fare spazio alla voce.",
        ]
    } else {
        [
            "Level and pan by role; a low cut removes rumble.",
            "A 2 dB cut where it covers a priority track.",
            "Light compression keeps the level steady.",
            "Ducks under the kick (sidechain) to leave it room.",
            "A little saturation adds body.",
            "More present in the choruses.",
            "Starting balance by role, with room for the vocal and a shared space.",
            "Group slightly lower to make room for the vocal.",
        ]
    }
}

/// Names of the returns the rules create for the lead vocal; a later run finds them by name and reuses them.
const VOCAL_PLATE: &str = "Vocal plate";
const SLAP: &str = "Slap";

/// Heavy settings blended under the uncompressed signal (AI-21).
fn parallel_comp() -> CompressorPlan {
    CompressorPlan {
        threshold_db: -30.0,
        ratio: 6.0,
        attack_ms: 3.0,
        release_ms: 80.0,
        knee_db: 3.0,
        makeup_db: 8.0,
        mix: 0.35,
        sidechain: None,
    }
}

/// The sentences of the mixing techniques (AI-21), in the language of the brief.
fn tech(italian: bool, i: usize) -> &'static str {
    let (it, en) = [
        (
            "Doppio aperto ai lati allo stesso livello dell'altra esecuzione.",
            "A true double: opened to the sides at the level of the other take.",
        ),
        (
            "Un plate corto solo per la voce, al posto dell'ambiente condiviso.",
            "A short plate for the vocal alone, instead of the shared space.",
        ),
        ("Slapback per dare corpo senza coda.", "A slapback adds body without a tail."),
        ("Un throw di delay nei ritornelli.", "A delay throw in the choruses."),
        (
            "Compressione parallela: una parte molto compressa sotto la batteria naturale la rende più densa.",
            "Parallel compression: a heavily compressed part under the natural kit makes it denser.",
        ),
        (
            "Nuovi: uno spazio dedicato per la voce e un bus per la batteria dove servono.",
            "New: a space of its own for the vocal and a bus for the drums where they help.",
        ),
    ][i];
    if italian { it } else { en }
}

impl FakeDecider {
    pub fn mix_plan(&self, inp: &DecisionInput) -> MixPlan {
        let lang = language(&inp.brief, inp.request.as_deref());
        let tx = texts(lang);
        let p = &inp.project;
        let ids = &inp.ids;
        let mut plan = match &inp.start {
            Some(v) if inp.request.is_some() || inp.iteration > 0 => {
                serde_json::from_value::<MixPlan>(v.clone()).unwrap_or_else(|_| self.rules(inp, &tx))
            }
            _ => self.rules(inp, &tx),
        };
        plan.iteration = inp.iteration;
        if let Some(req) = &inp.request {
            self.request(inp, &mut plan, req, lang);
        } else if inp.iteration > 0 {
            self.feedback(inp, &mut plan, lang);
        }
        // Every entry explains itself (AI-2): what the request did not touch says so.
        let kept = if lang == "it" { "Invariato rispetto al mix attuale." } else { "Unchanged from the current mix." };
        for why in
            plan.tracks.values_mut().map(|t| &mut t.rationale).chain(plan.groups.values_mut().map(|g| &mut g.rationale))
        {
            if why.trim().is_empty() {
                *why = kept.to_string();
            }
        }
        if plan.summary.trim().is_empty() {
            plan.summary = kept.to_string();
        }
        // Only editable tracks are returned.
        plan.tracks.retain(|n, _| inp.editable.contains(n));
        let created: Vec<String> = plan.structure.iter().flat_map(|s| s.groups.keys().cloned()).collect();
        plan.groups.retain(|n, _| inp.editable.contains(n) || created.contains(n));
        let _ = (p, ids);
        plan
    }

    fn rules(&self, inp: &DecisionInput, tx: &[&str; 8]) -> MixPlan {
        let p = &inp.project;
        let ids = &inp.ids;
        let audio: Vec<_> = p.tracks.iter().filter(|t| t.is_audio()).collect();
        let drums = audio.iter().filter(|t| t.role.is_some_and(|r| r.is_drums())).count();
        let family_db = if drums > 1 { -10.0 * (drums as f64).log10() } else { 0.0 };
        let reverb = p.tracks.iter().find(|t| t.ret.as_ref().is_some_and(|r| r.is_reverb())).map(|t| ids.n(&t.id));
        let delay = p.tracks.iter().find(|t| t.ret.as_ref().is_some_and(|r| !r.is_reverb())).map(|t| ids.n(&t.id));
        let kick = audio.iter().find(|t| t.role == Some(Role::Kick)).map(|t| ids.n(&t.id));
        let mut side_count: std::collections::HashMap<Role, usize> = std::collections::HashMap::new();
        let by_role = |r: Role| audio.iter().filter(|t| t.role == Some(r) && t.channels == 1).count();
        let collisions = inp.analysis["masking_top"].as_array().cloned().unwrap_or_default();
        let role_of = |n: &str| ids.to_project.get(n).and_then(|id| p.track(id)).and_then(|t| t.role);
        let it = tx[0].starts_with("Livello");
        let mut tracks = BTreeMap::new();
        for t in &audio {
            let n = ids.n(&t.id);
            let role = t.role.unwrap_or(Role::Other);
            let mut gain = role_gain(role) + if role.is_drums() { family_db } else { 0.0 };
            gain = (gain * 10.0).round() / 10.0;
            let centred = matches!(role, Role::Kick | Role::Snare | Role::Bass | Role::LeadVocal) || t.channels == 2;
            let pan = if centred || by_role(role) < 2 {
                0.0
            } else {
                let k = side_count.entry(role).or_default();
                *k += 1;
                if *k % 2 == 1 { -0.5 } else { 0.5 }
            };
            let mut why = vec![tx[0]];
            let mut eq = Vec::new();
            for c in &collisions {
                if eq.len() >= 2 || c["score"].as_f64().unwrap_or(0.0) < 0.05 {
                    continue;
                }
                let (a, b) = (c["a"].as_str().unwrap_or(""), c["b"].as_str().unwrap_or(""));
                if a != n && b != n {
                    continue;
                }
                let other = if a == n { b } else { a };
                let mine = PRIORITY.iter().position(|r| *r == role);
                let theirs = role_of(other).and_then(|r| PRIORITY.iter().position(|x| *x == r));
                let cut_me = match (mine, theirs) {
                    (None, Some(_)) => true,
                    (Some(m), Some(o)) => m > o,
                    _ => false,
                };
                if cut_me {
                    let band = stemwise_analysis::spectral::BAND_LABELS
                        .iter()
                        .position(|l| Some(*l) == c["band"].as_str())
                        .unwrap_or(4);
                    let f = band_centre(band);
                    if f < 0.9 * f64::from(p.sample_rate.unwrap_or(48000)) / 2.0
                        && !eq.iter().any(|e: &EqBand| e.freq_hz == f)
                    {
                        eq.push(EqBand { kind: EqType::Peak, freq_hz: f, gain_db: -2.0, q: 1.0 });
                    }
                }
            }
            if !eq.is_empty() {
                why.push(tx[1]);
            }
            let compressor = match role {
                Role::LeadVocal | Role::Bass => {
                    why.push(tx[2]);
                    let sc = if role == Role::Bass { kick.clone() } else { None };
                    if sc.is_some() {
                        why.push(tx[3]);
                    }
                    Some(CompressorPlan {
                        threshold_db: -24.0,
                        ratio: 3.0,
                        attack_ms: if role == Role::Bass { 20.0 } else { 10.0 },
                        release_ms: if role == Role::Bass { 200.0 } else { 120.0 },
                        knee_db: 6.0,
                        makeup_db: 2.0,
                        mix: 1.0,
                        sidechain: sc,
                    })
                }
                _ => None,
            };
            let saturation = match role {
                Role::Bass => Some(SaturationPlan { drive_db: 4.0, mix: 0.4, character: Some(SatCharacter::Tube) }),
                Role::ElectricGuitar => {
                    Some(SaturationPlan { drive_db: 3.0, mix: 0.3, character: Some(SatCharacter::Tape) })
                }
                _ => None,
            };
            if saturation.is_some() {
                why.push(tx[4]);
            }
            let mut sends = BTreeMap::new();
            if let (Some(r), Some(db)) = (&reverb, reverb_send(role)) {
                sends.insert(r.clone(), db);
            }
            if let (Some(d), Role::LeadVocal) = (&delay, role) {
                sends.insert(d.clone(), -22.0);
            }
            let mut sections = BTreeMap::new();
            if role == Role::LeadVocal {
                for s in &p.sections {
                    let name = s.name.to_lowercase();
                    let mut sp = SectionPlan::default();
                    if name.contains("chorus") || name.contains("ritornello") {
                        sp.gain_offset_db = 1.5;
                        // AI-21: a delay throw, the send to the delay raised in the choruses only.
                        if let Some(d) = &delay {
                            sp.send_offsets_db.insert(d.clone(), 6.0);
                        }
                    }
                    if (name.contains("bridge") || name.contains("ponte"))
                        && let Some(r) = &reverb
                    {
                        sp.send_offsets_db.insert(r.clone(), 3.0);
                    }
                    if sp != SectionPlan::default() {
                        sections.insert(ids.n(&s.id), sp);
                    }
                }
                if !sections.is_empty() {
                    why.push(tx[5]);
                }
            }
            tracks.insert(
                n,
                TrackPlan {
                    gain_db: gain,
                    pan,
                    hpf_hz: role_hpf(role),
                    lpf_hz: None,
                    eq,
                    compressor,
                    saturation,
                    sends_db: sends,
                    sections,
                    hpf_resonance: None,
                    lpf_resonance: None,
                    blocks: Vec::new(),
                    idea_sends_db: BTreeMap::new(),
                    rationale: why.join(" "),
                    idea_sections: BTreeMap::new(),
                },
            );
        }
        let mut groups: BTreeMap<String, GroupPlan> = p
            .tracks
            .iter()
            .filter(|t| t.is_group())
            .map(|g| {
                let drumish = p.tracks.iter().any(|t| {
                    t.output == stemwise_core::model::Output::Group(g.id.clone())
                        && t.role.is_some_and(|r| r.is_drums())
                });
                (
                    ids.n(&g.id),
                    GroupPlan {
                        gain_db: if drumish { -1.0 } else { 0.0 },
                        eq: Vec::new(),
                        // AI-21: a drum bus is compressed in parallel.
                        compressor: drumish.then(parallel_comp),
                        saturation: None,
                        sends_db: BTreeMap::new(),
                        blocks: Vec::new(),
                        idea_sends_db: BTreeMap::new(),
                        rationale: if drumish {
                            format!("{} {}", tx[7], tech(it, 4))
                        } else if tx[0].starts_with("Livello") {
                            "Livello del gruppo invariato.".to_string()
                        } else {
                            "Group level kept as it is.".to_string()
                        },
                    },
                )
            })
            .collect();
        let mut returns = BTreeMap::new();
        for t in p.tracks.iter().filter(|t| t.is_return()) {
            let r = if let Some(algorithm) = t.ret.as_ref().and_then(|x| x.algorithm()) {
                // The return keeps its algorithm, with a decay that suits it.
                ReturnPlan::Reverb {
                    algorithm,
                    decay_s: match algorithm {
                        ReverbAlgorithm::Plate => 1.2,
                        ReverbAlgorithm::Room => 0.6,
                        ReverbAlgorithm::Hall => 1.8,
                        ReverbAlgorithm::Shimmer => 3.5,
                    },
                    size: 0.5,
                    damping: 0.5,
                    modulation: 0.2,
                    predelay_ms: 20.0,
                    return_hpf_hz: 250.0,
                    return_lpf_hz: 9000.0,
                    return_gain_db: -6.0,
                    shimmer_semitones: None,
                    shimmer_amount: None,
                    blocks: Vec::new(),
                }
            } else {
                let time = p.tempo.map_or(375.0, |t| (60_000.0 / t.bpm * 0.75).clamp(20.0, 1000.0).round());
                ReturnPlan::Delay {
                    time_ms: time,
                    feedback: 0.25,
                    feedback_hpf_hz: 200.0,
                    feedback_lpf_hz: 6000.0,
                    return_gain_db: -9.0,
                    mode: None,
                    character: None,
                    wow: None,
                    drive_db: None,
                    blocks: Vec::new(),
                }
            };
            returns.insert(ids.n(&t.id), r);
        }
        // AI-21, true doubles: two performances of one part are opened to the sides, at one level.
        for d in inp.analysis["doubles"].as_array().into_iter().flatten() {
            let (a, b) = (d["a"].as_str().unwrap_or(""), d["b"].as_str().unwrap_or(""));
            if d["identical"].as_bool().unwrap_or(true)
                || !inp.editable.iter().any(|e| e == a)
                || !inp.editable.iter().any(|e| e == b)
            {
                continue;
            }
            let level = match (tracks.get(a), tracks.get(b)) {
                (Some(x), Some(y)) => ((x.gain_db + y.gain_db) / 2.0 * 10.0).round() / 10.0,
                _ => continue,
            };
            // The same processing on both: the second take gets the first one's filters, EQ, compressor and saturation.
            let first = tracks[a].clone();
            for (n, pan) in [(a, -0.7), (b, 0.7)] {
                let t: &mut TrackPlan = tracks.get_mut(n).expect("checked");
                t.pan = pan;
                t.gain_db = level;
                (t.hpf_hz, t.lpf_hz) = (first.hpf_hz, first.lpf_hz);
                t.eq = first.eq.clone();
                t.compressor = first.compressor.clone();
                t.saturation = first.saturation.clone();
                t.rationale = format!("{} {}", t.rationale, tech(it, 0));
            }
        }
        // AI-21, a space and a slapback of its own for the lead vocal. Returns of an earlier proposal are reused.
        let mut structure = Structure::default();
        let vocal = audio
            .iter()
            .find(|t| t.role == Some(Role::LeadVocal) && inp.editable.contains(&ids.n(&t.id)))
            .map(|t| ids.n(&t.id));
        let named = |name: &str| p.tracks.iter().find(|t| t.is_return() && t.name == name).map(|t| ids.n(&t.id));
        let mut n_returns = p.tracks.iter().filter(|t| t.is_return()).count();
        if let Some(v) = &vocal {
            let plate = named(VOCAL_PLATE).or_else(|| {
                (inp.allow_structure && n_returns < stemwise_core::model::MAX_RETURNS).then(|| {
                    n_returns += 1;
                    structure.returns.insert(
                        NEW_RETURN_KEYS[0].into(),
                        NewReturn {
                            kind: NewReturnType::Reverb,
                            name: VOCAL_PLATE.into(),
                            rationale: tech(it, 1).into(),
                            idea: None,
                        },
                    );
                    NEW_RETURN_KEYS[0].to_string()
                })
            });
            let slap = named(SLAP).or_else(|| {
                (inp.allow_structure && n_returns < stemwise_core::model::MAX_RETURNS).then(|| {
                    structure.returns.insert(
                        NEW_RETURN_KEYS[1].into(),
                        NewReturn {
                            kind: NewReturnType::Delay,
                            name: SLAP.into(),
                            rationale: tech(it, 2).into(),
                            idea: None,
                        },
                    );
                    NEW_RETURN_KEYS[1].to_string()
                })
            });
            let t: &mut TrackPlan = tracks.get_mut(v).expect("an audio track");
            if let Some(k) = plate {
                // The vocal leaves the shared reverb for its own plate.
                if let Some(r) = &reverb {
                    t.sends_db.remove(r);
                    for s in t.sections.values_mut() {
                        s.send_offsets_db.remove(r);
                    }
                }
                t.sends_db.insert(k.clone(), -14.0);
                t.rationale = format!("{} {}", t.rationale, tech(it, 1));
                returns.insert(
                    k,
                    ReturnPlan::Reverb {
                        algorithm: ReverbAlgorithm::Plate,
                        decay_s: 1.2,
                        size: 0.5,
                        damping: 0.4,
                        modulation: 0.3,
                        predelay_ms: 30.0,
                        return_hpf_hz: 300.0,
                        return_lpf_hz: 12000.0,
                        return_gain_db: -8.0,
                        shimmer_semitones: None,
                        shimmer_amount: None,
                        blocks: Vec::new(),
                    },
                );
            }
            if let Some(k) = slap {
                t.sends_db.insert(k.clone(), -18.0);
                t.rationale = format!("{} {}", t.rationale, tech(it, 2));
                returns.insert(
                    k,
                    ReturnPlan::Delay {
                        time_ms: 110.0,
                        feedback: 0.0,
                        feedback_hpf_hz: 200.0,
                        feedback_lpf_hz: 5000.0,
                        return_gain_db: -12.0,
                        mode: None,
                        character: None,
                        wow: None,
                        drive_db: None,
                        blocks: Vec::new(),
                    },
                );
            }
            if delay.as_ref().is_some_and(|d| t.sections.values().any(|s| s.send_offsets_db.contains_key(d))) {
                t.rationale = format!("{} {}", t.rationale, tech(it, 3));
            }
        }
        // AI-21, parallel compression: drum tracks that go straight to the mix bus get a bus of their own.
        let kit: Vec<String> = audio
            .iter()
            .filter(|t| {
                t.role.is_some_and(|r| r.is_drums())
                    && t.output == stemwise_core::model::Output::MixBus
                    && inp.editable.contains(&ids.n(&t.id))
            })
            .map(|t| ids.n(&t.id))
            .collect();
        if inp.allow_structure && kit.len() >= 2 {
            structure.groups.insert(
                NEW_GROUP_KEYS[0].into(),
                NewGroup { name: "Drums".into(), members: kit, rationale: tech(it, 4).into() },
            );
            groups.insert(
                NEW_GROUP_KEYS[0].into(),
                GroupPlan {
                    gain_db: 0.0,
                    eq: Vec::new(),
                    compressor: Some(parallel_comp()),
                    saturation: None,
                    sends_db: BTreeMap::new(),
                    blocks: Vec::new(),
                    idea_sends_db: BTreeMap::new(),
                    rationale: tech(it, 4).into(),
                },
            );
        }
        let ideas = self.p018(inp, &mut tracks, &mut returns, &mut structure, it);
        MixPlan {
            version: MIX_PLAN_VERSION,
            iteration: inp.iteration,
            summary: if structure.is_empty() { tx[6].to_string() } else { format!("{} {}", tx[6], tech(it, 5)) },
            structure: (!structure.is_empty()).then_some(structure),
            ideas,
            returns,
            tracks,
            groups,
        }
    }

    /// The techniques of P-018 (AI-21) and the creative ideas (AI-26), each by a deterministic rule on the roles,
    /// the measures of ANA-19 and the sections.
    fn p018(
        &self,
        inp: &DecisionInput,
        tracks: &mut BTreeMap<String, TrackPlan>,
        returns: &mut BTreeMap<String, ReturnPlan>,
        structure: &mut Structure,
        it: bool,
    ) -> Option<BTreeMap<String, IdeaPlan>> {
        use serde_json::json;
        use stemwise_core::fx::FxKind;
        let p = &inp.project;
        let ids = &inp.ids;
        let say = |en: &str, itx: &str| if it { itx.to_string() } else { en.to_string() };
        let block = |id: &str, kind: FxKind, pos: BlockPosition, idea: Option<&str>, v: serde_json::Value| BlockPlan {
            id: id.into(),
            kind,
            idea: idea.map(str::to_string),
            position: Some(pos),
            bypass: None,
            settings: v.as_object().cloned().unwrap_or_default(),
        };
        let measure = |n: &str, key: &str| inp.analysis["tracks"][n][key].as_f64();
        let role = |n: &str| ids.to_project.get(n).and_then(|id| p.track(id)).and_then(|t| t.role);
        let vocal = tracks.keys().find(|n| role(n) == Some(Role::LeadVocal)).cloned();
        let names: Vec<String> = tracks.keys().cloned().collect();
        for n in &names {
            let r = role(n);
            let t = tracks.get_mut(n).expect("listed");
            let add = |t: &mut TrackPlan, b: BlockPlan, why: String| {
                if t.blocks.iter().filter(|x| x.position.is_some()).count() < MAX_NEW_BLOCKS {
                    t.blocks.push(b);
                    t.rationale = format!("{} {why}", t.rationale);
                }
            };
            // De-esser on voices whose esses stand out (ANA-19).
            if matches!(r, Some(Role::LeadVocal | Role::BackingVocal))
                && measure(n, "sibilance_db").is_some_and(|v| v > -3.0)
            {
                let hz = measure(n, "sibilance_hz").unwrap_or(6500.0).clamp(3000.0, 12000.0);
                add(
                    t,
                    block(
                        "deess",
                        FxKind::Deesser,
                        BlockPosition::AfterCompressor,
                        None,
                        json!({"freq_hz": hz, "threshold_db": -30.0, "max_reduction_db": 5.0, "mode": "split"}),
                    ),
                    say(
                        "A de-esser after the compressor tames the esses.",
                        "Un de-esser dopo il compressore doma le sibilanti.",
                    ),
                );
            }
            // A gate on drums with a high floor between the hits.
            if matches!(r, Some(Role::Kick | Role::Snare | Role::Toms))
                && measure(n, "floor_db").is_some_and(|v| v > -30.0)
            {
                add(
                    t,
                    block(
                        "gate",
                        FxKind::Gate,
                        BlockPosition::First,
                        None,
                        json!({"threshold_db": -40.0, "range_db": 20.0, "ratio": 20.0, "attack_ms": 0.5, "hold_ms": 40.0, "release_ms": 120.0}),
                    ),
                    say(
                        "A gate takes 20 dB of bleed out between the hits.",
                        "Un gate toglie 20 dB di rientri tra un colpo e l'altro.",
                    ),
                );
            }
            // Guitars and keys dip at 3 kHz only while the vocal sings, instead of a fixed cut.
            if let Some(v) = &vocal
                && matches!(r, Some(Role::ElectricGuitar | Role::AcousticGuitar | Role::Keys | Role::Piano))
                && inp.editable.contains(v)
            {
                add(
                    t,
                    block(
                        "dyn",
                        FxKind::DynamicEq,
                        BlockPosition::BeforeCompressor,
                        None,
                        json!({"bands": [{"type": "peak", "freq_hz": 3000.0, "q": 1.2, "threshold_db": -35.0, "range_db": -3.0, "attack_ms": 10.0, "release_ms": 120.0, "sidechain": v}]}),
                    ),
                    say(
                        "A dynamic EQ dips 3 kHz only while the vocal sings.",
                        "Un EQ dinamico scava i 3 kHz solo mentre canta la voce.",
                    ),
                );
            }
            // Width on stereo pads and keys, never on bass and kick.
            if matches!(r, Some(Role::Pad | Role::Keys | Role::Synth | Role::Strings))
                && inp.analysis["tracks"][n]["channels"].as_u64() == Some(2)
            {
                add(
                    t,
                    block(
                        "wide",
                        FxKind::Width,
                        BlockPosition::Last,
                        None,
                        json!({"width": 1.3, "mono_below_hz": 120.0}),
                    ),
                    say(
                        "A little more width, the lows kept in the centre.",
                        "Un po' più di ampiezza, le basse al centro.",
                    ),
                );
            }
        }
        // Ducking: the shared reverb steps back while the vocal sings (FX-11), when the chains of returns may change.
        if inp.allow_return_blocks
            && let Some(v) = &vocal
            && let Some((rn, ReturnPlan::Reverb { blocks, .. })) = returns
                .iter_mut()
                .find(|(k, r)| matches!(r, ReturnPlan::Reverb { .. }) && ids.to_project.contains_key(*k))
        {
            let _ = rn;
            blocks.push(block("duck", FxKind::Compressor, BlockPosition::Last, None, json!({"threshold_db": -30.0, "ratio": 4.0, "attack_ms": 5.0, "release_ms": 250.0, "knee_db": 6.0, "makeup_db": 0.0, "mix": 1.0, "sidechain": v})));
        }
        if !(inp.allow_ideas && inp.iteration == 0) {
            return None;
        }
        // Creative ideas (AI-26): optional, different, not corrective; from roles and sections only.
        let mut ideas = BTreeMap::new();
        let mut secs: Vec<_> = p.sections.iter().collect();
        secs.sort_by_key(|s| s.start);
        let chorus: Vec<String> = secs
            .iter()
            .filter(|s| s.name.to_lowercase().contains("chorus") || s.name.to_lowercase().contains("ritornello"))
            .map(|s| ids.n(&s.id))
            .collect();
        // i1: a chorus that opens the keys in the choruses.
        if let Some(k) = names.iter().find(|n| {
            matches!(role(n), Some(Role::Keys | Role::Pad | Role::Synth | Role::Piano))
                && tracks[n.as_str()].sections.is_empty()
        }) {
            let t = tracks.get_mut(k).expect("listed");
            t.blocks.push(block(
                "ch",
                FxKind::Modulation,
                BlockPosition::Last,
                Some("i1"),
                json!({"mode": "chorus", "rate_hz": 0.8, "depth": 0.4, "voices": 2, "spread_deg": 120.0, "mix": 0.35}),
            ));
            for c in &chorus {
                t.sections.insert(
                    c.clone(),
                    SectionPlan { idea: Some("i1".into()), blocks_on: vec!["ch".into()], ..SectionPlan::default() },
                );
            }
            ideas.insert(
                "i1".into(),
                IdeaPlan {
                    name: "Wide chorus on the keys".into(),
                    sections: chorus.clone(),
                    rationale: say(
                        "The keys open up in the choruses and leave the centre to the vocal.",
                        "Le tastiere si aprono nei ritornelli e lasciano il centro alla voce.",
                    ),
                },
            );
        }
        // i2: a filter that opens into the last chorus.
        let before_last = chorus
            .last()
            .and_then(|c| secs.iter().position(|s| ids.n(&s.id) == *c))
            .filter(|i| *i > 0)
            .map(|i| ids.n(&secs[i - 1].id));
        if let Some(sec) = before_last
            && let Some(k) = names.iter().find(|n| {
                matches!(role(n), Some(Role::Pad | Role::Strings | Role::Synth))
                    && tracks[n.as_str()].sections.is_empty()
            })
        {
            // A resonant low-pass of its own, open over the song, closed by 2.5 octaves in the section before.
            let t = tracks.get_mut(k).expect("listed");
            t.blocks.push(block(
                "build",
                FxKind::Lpf,
                BlockPosition::Last,
                Some("i2"),
                json!({"freq_hz": 16000.0, "resonance": 2.0}),
            ));
            t.sections.insert(
                sec.clone(),
                SectionPlan { idea: Some("i2".into()), lpf_offset_oct: Some(-2.5), ..SectionPlan::default() },
            );
            ideas.insert(
                "i2".into(),
                IdeaPlan {
                    name: "Filter build into the last chorus".into(),
                    sections: vec![sec],
                    rationale: say(
                        "The part starts closed and opens into the last chorus.",
                        "La parte parte chiusa e si apre fino all'ultimo ritornello.",
                    ),
                },
            );
        }
        // i3: a ping-pong tape delay just for the vocal's throws, when new returns are allowed.
        if inp.allow_structure
            && let Some(v) = &vocal
            && !structure.returns.contains_key(NEW_RETURN_KEYS[2])
        {
            structure.returns.insert(
                NEW_RETURN_KEYS[2].into(),
                NewReturn {
                    kind: NewReturnType::Delay,
                    name: "Ping-pong".into(),
                    rationale: say(
                        "Repeats bouncing left and right.",
                        "Ripetizioni che rimbalzano tra destra e sinistra.",
                    ),
                    idea: Some("i3".into()),
                },
            );
            returns.insert(
                NEW_RETURN_KEYS[2].into(),
                ReturnPlan::Delay {
                    time_ms: 375.0,
                    feedback: 0.35,
                    feedback_hpf_hz: 300.0,
                    feedback_lpf_hz: 5000.0,
                    return_gain_db: -10.0,
                    mode: Some(stemwise_core::model::DelayMode::PingPong),
                    character: Some(stemwise_core::model::DelayCharacter::Tape),
                    wow: Some(0.3),
                    drive_db: Some(3.0),
                    blocks: Vec::new(),
                },
            );
            let t = tracks.get_mut(v).expect("listed");
            t.idea_sends_db.insert("i3".into(), BTreeMap::from([(NEW_RETURN_KEYS[2].to_string(), -16.0)]));
            ideas.insert(
                "i3".into(),
                IdeaPlan {
                    name: "Ping-pong tape throws".into(),
                    sections: Vec::new(),
                    rationale: say(
                        "The vocal's phrases bounce between the sides, a little worn like tape.",
                        "Le frasi della voce rimbalzano tra i lati, un po' consumate come un nastro.",
                    ),
                },
            );
        }
        (!ideas.is_empty()).then_some(ideas)
    }

    /// D-057: one EQ correction towards the target from the last evaluation.
    fn feedback(&self, inp: &DecisionInput, plan: &mut MixPlan, lang: &str) {
        let Some((_, eval)) = inp.history.last() else { return };
        let Some(deltas) = eval["band_delta_db"].as_array() else { return };
        let d: Vec<f64> = deltas.iter().filter_map(Value::as_f64).collect();
        if d.len() != 8 {
            return;
        }
        let band = (0..8).max_by(|&a, &b| d[a].abs().total_cmp(&d[b].abs()).then(b.cmp(&a))).unwrap_or(0);
        if d[band].abs() < 1.0 {
            return;
        }
        let f = band_centre(band);
        if f >= 0.9 * f64::from(inp.project.sample_rate.unwrap_or(48000)) / 2.0 {
            return;
        }
        let shares = &inp.analysis["band_share_pct"];
        let Some(n) = plan
            .tracks
            .keys()
            .max_by(|a, b| {
                let sa = shares[a.as_str()][band].as_f64().unwrap_or(0.0);
                let sb = shares[b.as_str()][band].as_f64().unwrap_or(0.0);
                sa.total_cmp(&sb).then(b.cmp(a))
            })
            .cloned()
        else {
            return;
        };
        let corr = d[band].clamp(-3.0, 3.0);
        let t = plan.tracks.get_mut(&n).expect("key from the map");
        let gain;
        if let Some(b) = t.eq.iter_mut().find(|b| b.kind == EqType::Peak && b.freq_hz == f) {
            b.gain_db = ((b.gain_db + corr).clamp(-9.0, 9.0) * 10.0).round() / 10.0;
            gain = b.gain_db;
        } else if t.eq.len() < MAX_TRACK_EQ {
            gain = (corr * 10.0).round() / 10.0;
            t.eq.push(EqBand { kind: EqType::Peak, freq_hz: f, gain_db: gain, q: 1.0 });
        } else {
            return;
        }
        t.rationale.push_str(&if lang == "it" {
            format!(" Iterazione {}: picco a {f} Hz ora {gain:+.1} dB, verso il riferimento.", inp.iteration)
        } else {
            format!(" Iteration {}: peak at {f} Hz now {gain:+.1} dB, towards the target.", inp.iteration)
        });
    }

    /// D-065, extended: keyword rules of a request, clause by clause.
    fn request(&self, inp: &DecisionInput, plan: &mut MixPlan, req: &str, lang: &str) {
        let p = &inp.project;
        let ids = &inp.ids;
        let words = |s: &str| -> Vec<String> {
            s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect()
        };
        let role_words: &[(&[&str], &[Role])] = &[
            (&["voce", "vocal", "vocals", "voice", "cantato", "vox"], &[Role::LeadVocal]),
            (&["cori", "coro", "backing"], &[Role::BackingVocal]),
            (&["basso", "bass"], &[Role::Bass]),
            (&["cassa", "kick"], &[Role::Kick]),
            (&["rullante", "snare"], &[Role::Snare]),
            (
                &["batteria", "drums", "drum"],
                &[Role::Kick, Role::Snare, Role::Toms, Role::Overheads, Role::Room, Role::Drums],
            ),
            (&["chitarra", "chitarre", "guitar", "guitars"], &[Role::ElectricGuitar, Role::AcousticGuitar]),
            (&["tastiera", "tastiere", "keys", "piano"], &[Role::Keys]),
            (&["archi", "strings"], &[Role::Strings]),
        ];
        let up =
            ["avanti", "forte", "presente", "alza", "aumenta", "louder", "forward", "upfront", "up", "more", "più"];
        let down =
            ["indietro", "piano", "meno", "abbassa", "riduci", "quieter", "softer", "lower", "back", "down", "less"];
        let reverb_w = ["riverbero", "riverberi", "reverb", "ambiente"];
        let delay_w = ["delay", "eco", "echo"];
        let reverb = p.tracks.iter().find(|t| t.ret.as_ref().is_some_and(|r| r.is_reverb())).map(|t| ids.n(&t.id));
        let delay = p.tracks.iter().find(|t| t.ret.as_ref().is_some_and(|r| !r.is_reverb())).map(|t| ids.n(&t.id));
        let mut changed = Vec::new();
        let lower = req.to_lowercase();
        for clause in lower.split([',', ';', '.']).flat_map(|c| c.split(" e ")).flat_map(|c| c.split(" and ")) {
            let w = words(clause);
            let has = |list: &[&str]| w.iter().any(|x| list.contains(&x.as_str()));
            // "più" alone means up only with no down word ("più indietro" is down).
            let dir = match (has(&up), has(&down)) {
                (true, false) => 1.0,
                (false, true) => -1.0,
                (true, true) if w.iter().any(|x| x == "più") && has(&down) => -1.0,
                _ => 0.0,
            };
            if dir == 0.0 {
                continue;
            }
            let mut subjects: Vec<String> = Vec::new();
            for (ws, roles) in role_words {
                if has(ws) {
                    for t in p.tracks.iter().filter(|t| t.is_audio() && t.role.is_some_and(|r| roles.contains(&r))) {
                        subjects.push(ids.n(&t.id));
                    }
                }
            }
            for t in p.tracks.iter().filter(|t| t.is_audio()) {
                if w.iter().any(|x| *x == t.name.to_lowercase()) {
                    subjects.push(ids.n(&t.id));
                }
            }
            let send_to = if has(&reverb_w) {
                reverb.clone()
            } else if has(&delay_w) {
                delay.clone()
            } else {
                None
            };
            if subjects.is_empty() && send_to.is_some() {
                subjects = plan
                    .tracks
                    .iter()
                    .filter(|(_, t)| t.sends_db.contains_key(send_to.as_ref().unwrap()))
                    .map(|(n, _)| n.clone())
                    .collect();
            }
            for n in subjects {
                let Some(t) = plan.tracks.get_mut(&n) else { continue };
                match &send_to {
                    Some(r) => {
                        let v = t.sends_db.get(r).copied();
                        let nv = match v {
                            Some(x) => (x + 3.0 * dir).clamp(-60.0, -6.0),
                            None if dir > 0.0 => -24.0,
                            None => continue,
                        };
                        if Some(nv) != v {
                            t.sends_db.insert(r.clone(), nv);
                            changed.push(n.clone());
                        }
                    }
                    None => {
                        let nv = (t.gain_db + 2.0 * dir).clamp(-24.0, 12.0);
                        if nv != t.gain_db {
                            t.gain_db = nv;
                            changed.push(n.clone());
                        }
                    }
                }
                let note = if lang == "it" { " Modificata come richiesto." } else { " Changed as you asked." };
                if !t.rationale.ends_with(note) {
                    t.rationale.push_str(note);
                }
            }
        }
        plan.summary = match (changed.is_empty(), lang) {
            (true, "it") => "Richiesta non capita, oppure i parametri sono già al limite: nessuna modifica.".into(),
            (true, _) => {
                "The request was not understood, or the parameters are already at their limit: nothing changed.".into()
            }
            (false, "it") => format!("Revisione: modificate {} tracce come richiesto.", changed.len()),
            (false, _) => format!("Revision: changed {} tracks as asked.", changed.len()),
        };
    }

    pub fn master_plan(&self, inp: &DecisionInput) -> MasterPlan {
        use crate::plan::MasterBlockPlan as B;
        let lang = language(&inp.brief, inp.request.as_deref());
        let it = lang == "it";
        let a = &inp.analysis;
        let target = &inp.project.master.target;
        // The id of the master's first block of a kind, or a word of its own for a block the plan adds.
        let id_of = |kind: &str| {
            a["current_chain"]
                .as_array()
                .and_then(|c| c.iter().find(|b| b["type"] == kind))
                .and_then(|b| b["id"].as_str())
                .map_or_else(|| format!("new_{kind}"), String::from)
        };
        // A later iteration (MAS-6): the plan just measured, with a clipper when the limiter worked too hard on few
        // isolated peaks; otherwise unchanged, which ends the loop.
        if let Some((last, eval)) = inp.history.last()
            && let Ok(mut plan) = serde_json::from_value::<MasterPlan>(last.clone())
        {
            let hard = eval["limiter"]["time_over_3_db_pct"].as_f64().unwrap_or(0.0) > 5.0;
            let sparse = a["mixdown"]["peak_share_pct"][2].as_f64().unwrap_or(100.0) < 2.0;
            if hard && sparse && !plan.chain.iter().any(|b| matches!(b, B::Clipper { .. })) {
                let needed = a["destination"]["peak_reduction_needed_db"].as_f64().unwrap_or(6.0);
                plan.chain.push(B::Clipper {
                    id: id_of("clipper"),
                    take_off_db: (needed - 3.0).clamp(2.0, 6.0),
                    softness: 0.4,
                    why: if it {
                        "Il limiter lavorava troppo su pochi picchi isolati: il clipper li abbassa prima.".into()
                    } else {
                        "The limiter worked too hard on a few isolated peaks: the clipper lowers them first.".into()
                    },
                });
            }
            return plan;
        }
        let mut plan = match &inp.start {
            Some(v) if inp.request.is_some() => serde_json::from_value::<MasterPlan>(v.clone()).ok(),
            _ => None,
        }
        .unwrap_or_else(|| {
            let deltas: Vec<f64> =
                a["band_delta_db"].as_array().map(|v| v.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
            let mut bands = Vec::new();
            if deltas.len() == 8 && deltas[7] > 1.0 {
                bands.push(EqBand {
                    kind: EqType::HighShelf,
                    freq_hz: 10000.0,
                    gain_db: deltas[7].clamp(0.0, 2.0),
                    q: 0.7,
                });
            }
            if deltas.len() == 8 && deltas[0] < -1.5 {
                bands.push(EqBand {
                    kind: EqType::LowShelf,
                    freq_hz: 60.0,
                    gain_db: deltas[0].clamp(-2.0, 0.0),
                    q: 0.7,
                });
            }
            let has_ref = !a["target"].is_null();
            let low_corr = a["mixdown"]["low_corr"].as_f64().unwrap_or(1.0);
            let lra = a["mixdown"]["loudness_range_lu"].as_f64().unwrap_or(6.0);
            let mut chain = Vec::new();
            if !bands.is_empty() {
                chain.push(B::Eq { id: id_of("eq"), bands, why: String::new() });
            }
            if has_ref {
                chain.push(B::MatchEq { id: id_of("match_eq"), amount: 0.5, why: String::new() });
            }
            if lra > 8.0 {
                chain.push(B::Compressor {
                    id: id_of("compressor"),
                    threshold_db: -18.0,
                    ratio: 1.8,
                    attack_ms: 30.0,
                    release_ms: 200.0,
                    knee_db: 6.0,
                    makeup_db: 0.0,
                    mix: 1.0,
                    listen_above_hz: 0.0,
                    why: String::new(),
                });
            }
            if low_corr < 0.6 {
                chain.push(B::Width {
                    id: id_of("width"),
                    side_gain_db: 0.0,
                    mono_below_hz: 120.0,
                    why: String::new(),
                });
            }
            MasterPlan {
                version: MASTER_PLAN_VERSION,
                summary: String::new(),
                target: MasterTarget {
                    lufs: target.lufs.clamp(-24.0, -6.0),
                    true_peak_db: target.true_peak_db.clamp(-3.0, -0.1),
                },
                input_gain_db: 0.0,
                chain,
                limiter: LimiterPlan { release_ms: 80.0 },
                sections: Vec::new(),
                rationale: BTreeMap::new(),
            }
        });
        if let Some(req) = &inp.request {
            let r = req.to_lowercase();
            let any = |ws: &[&str]| ws.iter().any(|w| r.contains(w));
            if any(&["più forte", "louder", "loud"]) {
                plan.target.lufs = (plan.target.lufs + 1.0).min(-6.0);
            }
            if any(&["più piano", "quieter", "softer"]) {
                plan.target.lufs = (plan.target.lufs - 1.0).max(-24.0);
            }
            if any(&["brillante", "brighter", "bright", "aria", "air"]) {
                if !plan.chain.iter().any(|b| matches!(b, B::Eq { .. })) {
                    plan.chain.insert(0, B::Eq { id: id_of("eq"), bands: Vec::new(), why: String::new() });
                }
                if let Some(B::Eq { bands, .. }) = plan.chain.iter_mut().find(|b| matches!(b, B::Eq { .. })) {
                    match bands.iter_mut().find(|b| b.kind == EqType::HighShelf) {
                        Some(b) => b.gain_db = (b.gain_db + 1.0).min(6.0),
                        None => bands.push(EqBand { kind: EqType::HighShelf, freq_hz: 10000.0, gain_db: 1.0, q: 0.7 }),
                    }
                }
            }
            if any(&["largo", "wider", "wide", "ampio"]) {
                if !plan.chain.iter().any(|b| matches!(b, B::Width { .. })) {
                    plan.chain.push(B::Width {
                        id: id_of("width"),
                        side_gain_db: 0.0,
                        mono_below_hz: 120.0,
                        why: String::new(),
                    });
                }
                if let Some(B::Width { side_gain_db, .. }) =
                    plan.chain.iter_mut().find(|b| matches!(b, B::Width { .. }))
                {
                    *side_gain_db = (*side_gain_db + 1.0).min(6.0);
                }
            }
            // A lift for the sections named like a chorus, when the request asks for it (MAS-14).
            if any(&["ritornell", "chorus"]) {
                plan.sections = a["sections"]
                    .as_array()
                    .map(|v| {
                        v.iter()
                            .filter(|s| s["name"].as_str().is_some_and(|n| n.to_lowercase().contains("chorus")))
                            .filter_map(|s| s["id"].as_str())
                            .map(|id| crate::plan::MasterSectionPlan {
                                section: id.to_string(),
                                level_db: 0.5,
                                side_gain_db: 0.0,
                                limiter_release_ms: 0.0,
                                eq: Vec::new(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
            }
        }
        plan.summary = if it {
            format!(
                "Master verso {:.0} LUFS e {:.1} dBTP, correzioni tonali leggere.",
                plan.target.lufs, plan.target.true_peak_db
            )
        } else {
            format!(
                "Master towards {:.0} LUFS and {:.1} dBTP with light tonal corrections.",
                plan.target.lufs, plan.target.true_peak_db
            )
        };
        for b in &mut plan.chain {
            if b.why().is_empty() {
                *b.why_mut() = match (&*b, it) {
                    (B::Eq { .. }, true) => "Piccoli ritocchi dove il mixdown si scosta dal riferimento.",
                    (B::Eq { .. }, false) => "Small touches where the mixdown departs from the reference.",
                    (B::MatchEq { .. }, true) => "A metà strada verso il profilo del riferimento.",
                    (B::MatchEq { .. }, false) => "Half-way toward the profile of the reference.",
                    (_, true) => "Regolato sulle misure del mixdown.",
                    (_, false) => "Set from the measures of the mixdown.",
                }
                .to_string();
            }
        }
        plan.rationale.insert(
            "limiter".to_string(),
            if it {
                "Il limiter garantisce il tetto di true peak."
            } else {
                "The limiter guarantees the true-peak ceiling."
            }
            .to_string(),
        );
        plan
    }
}

impl Decider for FakeDecider {
    fn name(&self) -> &str {
        "offline-demo"
    }

    fn model(&self) -> String {
        "offline-demo".into()
    }

    fn effort(&self) -> String {
        "-".into()
    }

    fn decide(
        &self,
        input: &DecisionInput,
        messages: &[Message],
        _schema: &Value,
        _retry: Option<&Retry>,
        cancel: &AtomicBool,
    ) -> Result<RawDecision, DecideError> {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(DecideError::Cancelled);
        }
        let content = match input.kind {
            Kind::Mix => serde_json::to_string(&self.mix_plan(input)),
            Kind::Master => serde_json::to_string(&self.master_plan(input)),
        }
        .map_err(|e| DecideError::Other(e.to_string()))?;
        let chars: usize = messages.iter().map(|m| m.content.chars().count()).sum();
        let request = serde_json::json!({"model": "offline-demo", "messages": messages}).to_string();
        Ok(RawDecision {
            usage: Usage {
                tokens_in: (chars / 4) as u64,
                tokens_out: (content.chars().count() / 4) as u64,
                cost_usd: Some(0.0),
            },
            raw: serde_json::json!({"choices": [{"message": {"content": content}}]}).to_string(),
            content,
            model: "offline-demo".into(),
            request,
        })
    }
}
