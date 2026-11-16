//! AI contracts (FOUNDATION §5.5): `mix_plan` v4 and `master_plan` v2. The limits live here once: validation,
//! the JSON Schema sent to the model and the limits table of the prompt are all derived from [`LIMITS`].

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use stemwise_core::model::{Project, ReverbAlgorithm};

use crate::analysis_doc::Ids;

/// A map whose `null` values mean "absent" (the strict schema lists every id; `null` = no send, no offset).
fn nonnull_map<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<BTreeMap<String, T>, D::Error> {
    let m: Option<BTreeMap<String, Option<T>>> = Option::deserialize(d)?;
    Ok(m.unwrap_or_default().into_iter().filter_map(|(k, v)| v.map(|v| (k, v))).collect())
}

/// An optional map whose `null` values mean "absent", like [`nonnull_map`]; a map with no value left is absent too
/// (the strict schema lists `i1`–`i3` and a plan with one idea writes `"i2": null, "i3": null`).
fn nonnull_opt_map<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<BTreeMap<String, T>>, D::Error> {
    let m = nonnull_map(d)?;
    Ok((!m.is_empty()).then_some(m))
}

/// A map of maps whose `null` values mean "absent" at both levels (`idea_sends_db`: an idea without sends is `null`,
/// a return the idea does not send to is `null`).
#[allow(clippy::type_complexity)]
fn nonnull_nested_map<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<BTreeMap<String, BTreeMap<String, T>>, D::Error> {
    let m: Option<BTreeMap<String, Option<BTreeMap<String, Option<T>>>>> = Option::deserialize(d)?;
    Ok(m.unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v.into_iter().filter_map(|(r, x)| x.map(|x| (r, x))).collect())))
        .collect())
}

/// A list given as `null` is empty (models sometimes write `"eq": null`).
fn null_vec<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Vec<T>, D::Error> {
    Ok(Option::<Vec<T>>::deserialize(d)?.unwrap_or_default())
}

pub const MIX_PLAN_VERSION: u32 = 4;
/// Ideas a plan may propose (AI-26), their placeholder ids, the longest name.
pub const IDEA_KEYS: [&str; 3] = ["i1", "i2", "i3"];
pub const MAX_IDEA_NAME: usize = 32;
/// New blocks a plan may add to one chain (AI-25).
pub const MAX_NEW_BLOCKS: usize = 3;
pub const MASTER_PLAN_VERSION: u32 = 2;
/// Free blocks a master plan may list (MAS-15).
pub const MAX_MASTER_BLOCKS: usize = 9;
/// Bands of a multiband (MAS-12).
pub const MB_BANDS: (usize, usize) = (2, 6);
pub const MAX_TRACK_EQ: usize = 4;
pub const MAX_MASTER_EQ: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EqType {
    Peak,
    LowShelf,
    HighShelf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EqBand {
    #[serde(rename = "type")]
    pub kind: EqType,
    pub freq_hz: f64,
    pub gain_db: f64,
    pub q: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompressorPlan {
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    pub knee_db: f64,
    pub makeup_db: f64,
    /// Dry/wet: 1 is the compressed signal alone, lower values are parallel compression (FX-3).
    pub mix: f64,
    /// Id of a track or group whose post-fader signal drives the detector, or null.
    pub sidechain: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaturationPlan {
    pub drive_db: f64,
    pub mix: f64,
    /// FX-18 (v4); absent keeps the track's.
    #[serde(default)]
    pub character: Option<stemwise_core::fx::SatCharacter>,
}

/// Per-section deviations of a track (AI-6), written as automation (AUT-5).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectionPlan {
    #[serde(default)]
    pub gain_offset_db: f64,
    #[serde(default, deserialize_with = "nonnull_map")]
    pub send_offsets_db: BTreeMap<String, f64>,
    /// One value per EQ band of the track, by position.
    #[serde(default, deserialize_with = "null_vec")]
    pub eq_gain_offsets_db: Vec<f64>,
    /// v4: the idea these offsets belong to (AI-26): all of them, or none.
    #[serde(default)]
    pub idea: Option<String>,
    /// v4: the high-pass and low-pass frequency moved by octaves in this section (AI-6, FX-12).
    #[serde(default)]
    pub hpf_offset_oct: Option<f64>,
    #[serde(default)]
    pub lpf_offset_oct: Option<f64>,
    /// v4: blocks on in this section only (bypassed in the sections that do not list them, AI-6).
    #[serde(default, deserialize_with = "null_vec")]
    pub blocks_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrackPlan {
    pub gain_db: f64,
    pub pan: f64,
    pub hpf_hz: Option<f64>,
    pub lpf_hz: Option<f64>,
    #[serde(deserialize_with = "null_vec")]
    pub eq: Vec<EqBand>,
    pub compressor: Option<CompressorPlan>,
    pub saturation: Option<SaturationPlan>,
    #[serde(deserialize_with = "nonnull_map")]
    pub sends_db: BTreeMap<String, f64>,
    #[serde(default, deserialize_with = "nonnull_map")]
    pub sections: BTreeMap<String, SectionPlan>,
    /// v4 (FX-12): Q of the filters; absent keeps the track's.
    #[serde(default)]
    pub hpf_resonance: Option<f64>,
    #[serde(default)]
    pub lpf_resonance: Option<f64>,
    /// v4 (AI-25): blocks the plan adds or changes; a block not listed stays as it is.
    #[serde(default, deserialize_with = "null_vec")]
    pub blocks: Vec<BlockPlan>,
    /// v4 (AI-26): sends that belong to an idea, by idea and return.
    #[serde(default, deserialize_with = "nonnull_nested_map")]
    pub idea_sends_db: BTreeMap<String, BTreeMap<String, f64>>,
    pub rationale: String,
    /// Not from the model: the section offsets of an idea for sections the plan kept by the loop also uses (AI-26,
    /// A-089). Kept apart so that taking the idea out leaves the plan's own offsets; folded in when it is ticked.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schemars(skip)]
    pub idea_sections: BTreeMap<String, SectionPlan>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GroupPlan {
    pub gain_db: f64,
    #[serde(deserialize_with = "null_vec")]
    pub eq: Vec<EqBand>,
    pub compressor: Option<CompressorPlan>,
    pub saturation: Option<SaturationPlan>,
    #[serde(deserialize_with = "nonnull_map")]
    pub sends_db: BTreeMap<String, f64>,
    #[serde(default, deserialize_with = "null_vec")]
    pub blocks: Vec<BlockPlan>,
    #[serde(default, deserialize_with = "nonnull_nested_map")]
    pub idea_sends_db: BTreeMap<String, BTreeMap<String, f64>>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum ReturnPlan {
    Reverb {
        algorithm: ReverbAlgorithm,
        decay_s: f64,
        size: f64,
        damping: f64,
        modulation: f64,
        predelay_ms: f64,
        return_hpf_hz: f64,
        return_lpf_hz: f64,
        return_gain_db: f64,
        /// v4, shimmer only (FX-22).
        #[serde(default)]
        shimmer_semitones: Option<f64>,
        #[serde(default)]
        shimmer_amount: Option<f64>,
        /// v4 (FX-11, AI-25): blocks after the reverb; only with structural changes allowed.
        #[serde(default, deserialize_with = "null_vec")]
        blocks: Vec<BlockPlan>,
    },
    Delay {
        time_ms: f64,
        feedback: f64,
        feedback_hpf_hz: f64,
        feedback_lpf_hz: f64,
        return_gain_db: f64,
        /// v4 (FX-23).
        #[serde(default)]
        mode: Option<stemwise_core::model::DelayMode>,
        #[serde(default)]
        character: Option<stemwise_core::model::DelayCharacter>,
        #[serde(default)]
        wow: Option<f64>,
        #[serde(default)]
        drive_db: Option<f64>,
        #[serde(default, deserialize_with = "null_vec")]
        blocks: Vec<BlockPlan>,
    },
}

impl ReturnPlan {
    pub fn blocks(&self) -> &[BlockPlan] {
        match self {
            ReturnPlan::Reverb { blocks, .. } | ReturnPlan::Delay { blocks, .. } => blocks,
        }
    }

    pub fn blocks_mut(&mut self) -> &mut Vec<BlockPlan> {
        match self {
            ReturnPlan::Reverb { blocks, .. } | ReturnPlan::Delay { blocks, .. } => blocks,
        }
    }
}

/// Where a new block goes in its chain (AI-25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlockPosition {
    First,
    BeforeCompressor,
    AfterCompressor,
    Last,
}

/// A block the plan adds or changes (AI-25). `id` is a block the analysis lists (`k…`) or a new word; a new block
/// has a `position`. Its settings are the fields of its type (`type` is the kind), as in FOUNDATION §5.5; what a
/// plan leaves out keeps its value (a new block: the value of a new one).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BlockPlan {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: stemwise_core::fx::FxKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idea: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<BlockPosition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bypass: Option<bool>,
    /// The block's settings, as the fields of its type.
    #[serde(flatten)]
    pub settings: serde_json::Map<String, Value>,
}

/// An idea of the plan (AI-26): only additions, chosen by the author one by one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IdeaPlan {
    pub name: String,
    #[serde(default, deserialize_with = "null_vec")]
    pub sections: Vec<String>,
    pub rationale: String,
}

/// Placeholder ids of what a plan creates (AI-17): returns first, then groups. They are used in the rest of the
/// plan like the ids of existing returns and groups.
pub const NEW_RETURN_KEYS: [&str; 3] = ["n1", "n2", "n3"];
pub const NEW_GROUP_KEYS: [&str; 3] = ["n4", "n5", "n6"];
/// Longest name of a new return or group.
pub const MAX_NEW_NAME: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NewReturnType {
    Reverb,
    Delay,
}

/// A return the plan creates; its parameters are under `returns.<id>` like any other return's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewReturn {
    #[serde(rename = "type")]
    pub kind: NewReturnType,
    pub name: String,
    pub rationale: String,
    /// v4: the idea this return belongs to (AI-26).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idea: Option<String>,
}

/// A group the plan creates around existing tracks or groups; its chain is under `groups.<id>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewGroup {
    pub name: String,
    #[serde(default, deserialize_with = "null_vec")]
    pub members: Vec<String>,
    pub rationale: String,
}

/// What a plan adds to the structure of the mix (AI-17): nothing else is created, moved or removed.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Structure {
    #[serde(default, deserialize_with = "nonnull_map")]
    pub returns: BTreeMap<String, NewReturn>,
    #[serde(default, deserialize_with = "nonnull_map")]
    pub groups: BTreeMap<String, NewGroup>,
}

impl Structure {
    pub fn is_empty(&self) -> bool {
        self.returns.is_empty() && self.groups.is_empty()
    }
}

/// `mix_plan` v4 (FOUNDATION §5.5); plans of version 3 kept in projects read as they are (AI-23).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MixPlan {
    pub version: u32,
    pub iteration: u32,
    pub summary: String,
    /// New returns and groups (AI-17); absent in later iterations, where the structure is already decided (AI-19).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure: Option<Structure>,
    /// v4 (AI-26): creative ideas; absent when they are not allowed and in later iterations.
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "nonnull_opt_map")]
    pub ideas: Option<BTreeMap<String, IdeaPlan>>,
    #[serde(default, deserialize_with = "nonnull_map")]
    pub returns: BTreeMap<String, ReturnPlan>,
    #[serde(deserialize_with = "nonnull_map")]
    pub tracks: BTreeMap<String, TrackPlan>,
    #[serde(default, deserialize_with = "nonnull_map")]
    pub groups: BTreeMap<String, GroupPlan>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MasterTarget {
    pub lufs: f64,
    pub true_peak_db: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LimiterPlan {
    pub release_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MbBandPlan {
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    pub gain_db: f64,
}

/// A free block of the master chain as the plan sets it. `id` is the id of a block the master already has, or any
/// other word for a block the plan adds; `why` is its reason, in the language of the brief.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MasterBlockPlan {
    Eq {
        id: String,
        #[serde(deserialize_with = "null_vec")]
        bands: Vec<EqBand>,
        #[serde(default)]
        why: String,
    },
    MatchEq {
        id: String,
        amount: f64,
        #[serde(default)]
        why: String,
    },
    Compressor {
        id: String,
        threshold_db: f64,
        ratio: f64,
        attack_ms: f64,
        release_ms: f64,
        knee_db: f64,
        makeup_db: f64,
        mix: f64,
        listen_above_hz: f64,
        #[serde(default)]
        why: String,
    },
    Multiband {
        id: String,
        crossovers_hz: Vec<f64>,
        bands: Vec<MbBandPlan>,
        #[serde(default)]
        why: String,
    },
    Width {
        id: String,
        side_gain_db: f64,
        mono_below_hz: f64,
        #[serde(default)]
        why: String,
    },
    Saturation {
        id: String,
        drive_db: f64,
        mix: f64,
        #[serde(default)]
        why: String,
    },
    Clipper {
        id: String,
        take_off_db: f64,
        softness: f64,
        #[serde(default)]
        why: String,
    },
}

impl MasterBlockPlan {
    pub fn id(&self) -> &str {
        match self {
            MasterBlockPlan::Eq { id, .. }
            | MasterBlockPlan::MatchEq { id, .. }
            | MasterBlockPlan::Compressor { id, .. }
            | MasterBlockPlan::Multiband { id, .. }
            | MasterBlockPlan::Width { id, .. }
            | MasterBlockPlan::Saturation { id, .. }
            | MasterBlockPlan::Clipper { id, .. } => id,
        }
    }

    pub fn why(&self) -> &str {
        match self {
            MasterBlockPlan::Eq { why, .. }
            | MasterBlockPlan::MatchEq { why, .. }
            | MasterBlockPlan::Compressor { why, .. }
            | MasterBlockPlan::Multiband { why, .. }
            | MasterBlockPlan::Width { why, .. }
            | MasterBlockPlan::Saturation { why, .. }
            | MasterBlockPlan::Clipper { why, .. } => why,
        }
    }

    pub fn why_mut(&mut self) -> &mut String {
        match self {
            MasterBlockPlan::Eq { why, .. }
            | MasterBlockPlan::MatchEq { why, .. }
            | MasterBlockPlan::Compressor { why, .. }
            | MasterBlockPlan::Multiband { why, .. }
            | MasterBlockPlan::Width { why, .. }
            | MasterBlockPlan::Saturation { why, .. }
            | MasterBlockPlan::Clipper { why, .. } => why,
        }
    }

    pub fn kind(&self) -> stemwise_core::model::BlockKind {
        use stemwise_core::model::BlockKind as K;
        match self {
            MasterBlockPlan::Eq { .. } => K::Eq,
            MasterBlockPlan::MatchEq { .. } => K::MatchEq,
            MasterBlockPlan::Compressor { .. } => K::Compressor,
            MasterBlockPlan::Multiband { .. } => K::Multiband,
            MasterBlockPlan::Width { .. } => K::Width,
            MasterBlockPlan::Saturation { .. } => K::Saturation,
            MasterBlockPlan::Clipper { .. } => K::Clipper,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectionEqPlan {
    /// `id` of an EQ block of `chain`.
    pub block: String,
    /// One offset per band of that EQ.
    pub gains_db: Vec<f64>,
}

/// Offsets of one section on top of the song-wide master (MAS-14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MasterSectionPlan {
    pub section: String,
    pub level_db: f64,
    pub side_gain_db: f64,
    pub limiter_release_ms: f64,
    #[serde(default, deserialize_with = "null_vec")]
    pub eq: Vec<SectionEqPlan>,
}

/// `master_plan` v2 (FOUNDATION §5.5): the free blocks in processing order, the limiter, offsets per section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MasterPlan {
    pub version: u32,
    pub summary: String,
    pub target: MasterTarget,
    pub input_gain_db: f64,
    /// The blocks that work, in the order they process. A block of the master that is not listed is switched off.
    #[serde(deserialize_with = "null_vec")]
    pub chain: Vec<MasterBlockPlan>,
    pub limiter: LimiterPlan,
    #[serde(default, deserialize_with = "null_vec")]
    pub sections: Vec<MasterSectionPlan>,
    /// Reasons that belong to no block: `order`, `limiter`, `sections`.
    #[serde(default, deserialize_with = "nonnull_map")]
    pub rationale: BTreeMap<String, String>,
}

/// One bounded field of a plan: (path pattern, minimum, maximum, unit). Normative (§5.5, PRD §6.3).
pub struct Limit {
    pub path: &'static str,
    pub min: f64,
    pub max: f64,
    pub unit: &'static str,
}

const fn l(path: &'static str, min: f64, max: f64, unit: &'static str) -> Limit {
    Limit { path, min, max, unit }
}

pub const LIMITS: &[Limit] = &[
    l("tracks.<id>.gain_db", -24.0, 12.0, "dB"),
    l("tracks.<id>.pan", -1.0, 1.0, ""),
    l("tracks.<id>.hpf_hz", 20.0, 400.0, "Hz"),
    l("tracks.<id>.lpf_hz", 2000.0, 20000.0, "Hz"),
    l("eq[].freq_hz", 20.0, 20000.0, "Hz"),
    l("eq[].gain_db", -9.0, 9.0, "dB"),
    l("eq[].q", 0.3, 8.0, ""),
    l("compressor.threshold_db", -40.0, 0.0, "dB"),
    l("compressor.ratio", 1.0, 8.0, ""),
    l("compressor.attack_ms", 0.1, 100.0, "ms"),
    l("compressor.release_ms", 20.0, 1000.0, "ms"),
    l("compressor.knee_db", 0.0, 12.0, "dB"),
    l("compressor.makeup_db", 0.0, 12.0, "dB"),
    l("compressor.mix", 0.1, 1.0, ""),
    l("saturation.drive_db", 0.0, 18.0, "dB"),
    l("saturation.mix", 0.0, 1.0, ""),
    l("sends_db.<return>", -60.0, -6.0, "dB"),
    l("groups.<id>.gain_db", -24.0, 12.0, "dB"),
    l("returns.<reverb>.decay_s", 0.3, 6.0, "s"),
    l("returns.<reverb>.size", 0.0, 1.0, ""),
    l("returns.<reverb>.damping", 0.0, 1.0, ""),
    l("returns.<reverb>.modulation", 0.0, 1.0, ""),
    l("returns.<reverb>.predelay_ms", 0.0, 100.0, "ms"),
    l("returns.<reverb>.return_hpf_hz", 20.0, 800.0, "Hz"),
    l("returns.<reverb>.return_lpf_hz", 2000.0, 20000.0, "Hz"),
    l("returns.<reverb>.return_gain_db", -24.0, 0.0, "dB"),
    l("returns.<delay>.time_ms", 20.0, 1000.0, "ms"),
    l("returns.<delay>.feedback", 0.0, 0.7, ""),
    l("returns.<delay>.feedback_hpf_hz", 20.0, 800.0, "Hz"),
    l("returns.<delay>.feedback_lpf_hz", 1000.0, 20000.0, "Hz"),
    l("returns.<delay>.return_gain_db", -24.0, 0.0, "dB"),
    l("sections.<id>.gain_offset_db", -6.0, 6.0, "dB"),
    l("sections.<id>.send_offsets_db.<return>", -12.0, 12.0, "dB"),
    l("sections.<id>.eq_gain_offsets_db[]", -4.0, 4.0, "dB"),
    l("tracks.<id>.hpf_resonance", 0.5, 4.0, ""),
    l("tracks.<id>.lpf_resonance", 0.5, 4.0, ""),
    l("sections.<id>.hpf_offset_oct", -3.0, 3.0, "oct"),
    l("sections.<id>.lpf_offset_oct", -3.0, 3.0, "oct"),
    l("gate.threshold_db", -70.0, -10.0, "dB"),
    l("gate.range_db", 0.0, 40.0, "dB"),
    l("gate.ratio", 1.0, 20.0, ""),
    l("gate.attack_ms", 0.1, 20.0, "ms"),
    l("gate.hold_ms", 0.0, 200.0, "ms"),
    l("gate.release_ms", 10.0, 1000.0, "ms"),
    l("deesser.freq_hz", 3000.0, 12000.0, "Hz"),
    l("deesser.max_reduction_db", 0.0, 10.0, "dB"),
    l("dynamic_eq.bands[].range_db", -9.0, 4.0, "dB"),
    l("dynamic_eq.bands[].q", 0.3, 6.0, ""),
    l("transient.attack_db", -9.0, 9.0, "dB"),
    l("transient.sustain_db", -9.0, 9.0, "dB"),
    l("clipper.threshold_db", -18.0, 0.0, "dB"),
    l("multiband.bands[].gain_db", -6.0, 6.0, "dB"),
    l("exciter.freq_hz", 1500.0, 12000.0, "Hz"),
    l("exciter.amount", 0.0, 0.6, ""),
    l("bass_enhancer.freq_hz", 40.0, 160.0, "Hz"),
    l("bass_enhancer.harmonics", 0.0, 0.8, ""),
    l("width.width", 0.0, 1.6, ""),
    l("width.mono_below_hz", 0.0, 250.0, "Hz"),
    l("modulation.rate_hz", 0.02, 8.0, "Hz"),
    l("modulation.depth", 0.0, 1.0, ""),
    l("modulation.feedback", -0.8, 0.8, ""),
    l("modulation.mix", 0.0, 0.6, ""),
    l("tremolo.rate_hz", 0.1, 12.0, "Hz"),
    l("tremolo.depth", 0.0, 0.8, ""),
    l("harmonizer.voices[].semitones", -12.0, 12.0, "st"),
    l("harmonizer.mix", 0.0, 0.5, ""),
    l("doubler.detune_cents", 0.0, 25.0, "ct"),
    l("doubler.delay_ms", 5.0, 40.0, "ms"),
    l("doubler.mix", 0.0, 0.6, ""),
    l("returns.<reverb>.shimmer_amount", 0.0, 0.6, ""),
    l("returns.<delay>.wow", 0.0, 1.0, ""),
    l("returns.<delay>.drive_db", 0.0, 9.0, "dB"),
];

pub const MASTER_LIMITS: &[Limit] = &[
    l("target.lufs", -24.0, -6.0, "LUFS"),
    l("target.true_peak_db", -3.0, -0.1, "dBTP"),
    l("input_gain_db", -12.0, 12.0, "dB"),
    l("eq[].freq_hz", 20.0, 20000.0, "Hz"),
    l("eq[].gain_db", -6.0, 6.0, "dB"),
    l("eq[].q", 0.3, 8.0, ""),
    l("match_eq.amount", 0.0, 1.0, ""),
    l("compressor.threshold_db", -40.0, 0.0, "dB"),
    l("compressor.ratio", 1.0, 4.0, ""),
    l("compressor.attack_ms", 0.1, 100.0, "ms"),
    l("compressor.release_ms", 20.0, 1000.0, "ms"),
    l("compressor.knee_db", 0.0, 12.0, "dB"),
    l("compressor.makeup_db", 0.0, 12.0, "dB"),
    l("compressor.mix", 0.2, 1.0, ""),
    l("compressor.listen_above_hz", 0.0, 300.0, "Hz"),
    l("multiband.crossovers_hz[]", 40.0, 12000.0, "Hz"),
    l("multiband.bands[].threshold_db", -40.0, 0.0, "dB"),
    l("multiband.bands[].ratio", 1.0, 4.0, ""),
    l("multiband.bands[].attack_ms", 0.1, 100.0, "ms"),
    l("multiband.bands[].release_ms", 20.0, 1000.0, "ms"),
    l("multiband.bands[].gain_db", -6.0, 6.0, "dB"),
    l("width.side_gain_db", -6.0, 6.0, "dB"),
    l("width.mono_below_hz", 0.0, 250.0, "Hz"),
    l("saturation.drive_db", 0.0, 18.0, "dB"),
    l("saturation.mix", 0.0, 1.0, ""),
    l("clipper.take_off_db", 0.0, 6.0, "dB"),
    l("clipper.softness", 0.0, 1.0, ""),
    l("limiter.release_ms", 20.0, 500.0, "ms"),
    l("sections[].level_db", -2.0, 2.0, "dB"),
    l("sections[].side_gain_db", -3.0, 3.0, "dB"),
    l("sections[].limiter_release_ms", -100.0, 100.0, "ms"),
    l("sections[].eq[].gains_db[]", -2.0, 2.0, "dB"),
];

/// Limit of a path pattern.
pub fn limit(path: &str) -> &'static Limit {
    LIMITS.iter().chain(MASTER_LIMITS).find(|l| l.path == path).unwrap_or_else(|| panic!("no limit for {path}"))
}

/// Markdown table of the limits for the prompt (generated, never copied by hand: §5.5).
pub fn limits_table(limits: &[Limit]) -> String {
    let mut s = String::from("| parameter | min | max |\n| --- | --- | --- |\n");
    for l in limits {
        let unit = if l.unit.is_empty() { String::new() } else { format!(" {}", l.unit) };
        s.push_str(&format!("| {} | {}{unit} | {}{unit} |\n", l.path, l.min, l.max));
    }
    s
}

/// The exported contracts of the AI (FOUNDATION §5.5), as (file name, pretty JSON) for `app/schemas/`: the generic
/// shapes. What is sent to the model is the per-project schema of [`mix_plan_schema_for`] with every limit.
pub fn json_schemas() -> Vec<(&'static str, String)> {
    let pretty = |v: Value| serde_json::to_string_pretty(&v).expect("schema") + "\n";
    vec![
        ("mix_plan.schema.json", pretty(serde_json::to_value(schemars::schema_for!(MixPlan)).expect("schema"))),
        ("master_plan.schema.json", pretty(master_plan_schema(&[]))),
    ]
}

/// Builds schemas from the limits table and records which limits it used (the test checks that none is left out).
struct Builder<'a> {
    limits: &'a [Limit],
    used: Vec<&'static str>,
}

impl Builder<'_> {
    fn lim(&mut self, path: &str) -> (f64, f64) {
        let l = self.limits.iter().find(|l| l.path == path).unwrap_or_else(|| panic!("no limit for {path}"));
        self.used.push(l.path);
        (l.min, l.max)
    }
    fn num(&mut self, path: &str) -> Value {
        let (a, b) = self.lim(path);
        json!({"type": "number", "minimum": a, "maximum": b})
    }
    fn num_or_null(&mut self, path: &str) -> Value {
        let (a, b) = self.lim(path);
        json!({"type": ["number", "null"], "minimum": a, "maximum": b})
    }
    fn eq(&mut self, max: usize) -> Value {
        let band = obj(vec![
            ("type", json!({"type": "string", "enum": ["peak", "low_shelf", "high_shelf"]})),
            ("freq_hz", self.num("eq[].freq_hz")),
            ("gain_db", self.num("eq[].gain_db")),
            ("q", self.num("eq[].q")),
        ]);
        json!({"type": "array", "items": band, "maxItems": max})
    }
    fn comp(&mut self, sidechain: Option<&[String]>) -> Value {
        let mut f = vec![
            ("threshold_db", self.num("compressor.threshold_db")),
            ("ratio", self.num("compressor.ratio")),
            ("attack_ms", self.num("compressor.attack_ms")),
            ("release_ms", self.num("compressor.release_ms")),
            ("knee_db", self.num("compressor.knee_db")),
            ("makeup_db", self.num("compressor.makeup_db")),
        ];
        if let Some(ids) = sidechain {
            // Tracks and groups: the master compressor has neither a mix nor a sidechain.
            f.push(("mix", self.num("compressor.mix")));
            let mut e: Vec<Value> = ids.iter().map(|i| json!(i)).collect();
            e.push(Value::Null);
            f.push(("sidechain", json!({"type": ["string", "null"], "enum": e})));
        }
        nullable(obj(f))
    }
    /// The saturation of a track or group (v4): with its character.
    fn sat4(&mut self) -> Value {
        nullable(obj(vec![
            ("drive_db", self.num("saturation.drive_db")),
            ("mix", self.num("saturation.mix")),
            ("character", json!({"type": ["string", "null"], "enum": ["soft", "tape", "tube", "transistor", null]})),
        ]))
    }
    /// One block kind of `mix_plan` v4 (AI-25), every field nullable (`null` keeps the value).
    fn block(&mut self, kind: stemwise_core::fx::FxKind, keys: &[String], ideas: bool, tempo: bool) -> Value {
        use crate::blocks::{Field, RATES, fields};
        let any_num = || json!({"type": ["number", "null"]});
        let key = |keys: &[String]| {
            let mut e: Vec<Value> = keys.iter().map(|i| json!(i)).collect();
            e.push(Value::Null);
            json!({"type": ["string", "null"], "enum": e})
        };
        let mut f: Vec<(&str, Value)> = vec![
            ("id", json!({"type": "string", "minLength": 1})),
            ("type", json!({"type": "string", "enum": [kind.key_name()]})),
            (
                "position",
                json!({"type": ["string", "null"], "enum": ["first", "before_compressor", "after_compressor", "last", null]}),
            ),
            ("bypass", json!({"type": ["boolean", "null"]})),
        ];
        if ideas {
            f.push(("idea", json!({"type": ["string", "null"], "enum": ["i1", "i2", "i3", null]})));
        }
        for (name, field) in fields(kind) {
            let v = match field {
                Field::Num(Some(path)) => self.num_or_null(path),
                Field::Num(None) => any_num(),
                Field::Int(a, b) => json!({"type": ["integer", "null"], "minimum": a, "maximum": b}),
                Field::IntOf(v) => {
                    let mut e: Vec<Value> = v.iter().map(|x| json!(x)).collect();
                    e.push(Value::Null);
                    json!({"type": ["integer", "null"], "enum": e})
                }
                Field::Enum(v) => {
                    let mut e: Vec<Value> = v.iter().map(|x| json!(x)).collect();
                    e.push(Value::Null);
                    json!({"type": ["string", "null"], "enum": e})
                }
                Field::Key => key(keys),
                Field::Rate => {
                    if tempo {
                        let mut e: Vec<Value> = RATES.iter().map(|x| json!(x)).collect();
                        e.push(Value::Null);
                        json!({"type": ["string", "null"], "enum": e})
                    } else {
                        json!({"type": "null"})
                    }
                }
                Field::EqBands => self.eq(MAX_TRACK_EQ),
                Field::DeqBands => {
                    let band = obj(vec![
                        ("type", json!({"type": "string", "enum": ["peak", "low_shelf", "high_shelf"]})),
                        ("freq_hz", self.num("eq[].freq_hz")),
                        ("q", self.num("dynamic_eq.bands[].q")),
                        ("threshold_db", json!({"type": "number", "minimum": -60, "maximum": 0})),
                        ("range_db", self.num("dynamic_eq.bands[].range_db")),
                        ("attack_ms", json!({"type": "number", "minimum": 0.1, "maximum": 100})),
                        ("release_ms", json!({"type": "number", "minimum": 10, "maximum": 1000})),
                        ("sidechain", key(keys)),
                    ]);
                    json!({"type": ["array", "null"], "items": band, "minItems": 1, "maxItems": 2})
                }
                Field::MbBands => {
                    let band = obj(vec![
                        ("threshold_db", self.num("compressor.threshold_db")),
                        ("ratio", self.num("compressor.ratio")),
                        ("attack_ms", self.num("compressor.attack_ms")),
                        ("release_ms", self.num("compressor.release_ms")),
                        ("gain_db", self.num("multiband.bands[].gain_db")),
                    ]);
                    json!({"type": ["array", "null"], "items": band, "minItems": 2, "maxItems": 4})
                }
                Field::Crossovers => {
                    json!({"type": ["array", "null"], "items": {"type": "number", "minimum": 40, "maximum": 12000}, "minItems": 1, "maxItems": 3})
                }
                Field::HarmVoices => {
                    let v = obj(vec![
                        ("semitones", self.num("harmonizer.voices[].semitones")),
                        ("cents", json!({"type": "number", "minimum": -50, "maximum": 50})),
                        ("level_db", json!({"type": "number", "minimum": -24, "maximum": 6})),
                        ("pan", json!({"type": "number", "minimum": -1, "maximum": 1})),
                    ]);
                    json!({"type": ["array", "null"], "items": v, "minItems": 1, "maxItems": 2})
                }
            };
            f.push((name, v));
        }
        obj(f)
    }
}

fn idea_mark() -> Value {
    json!({"type": ["string", "null"], "enum": ["i1", "i2", "i3", null]})
}

/// The `$defs` of the block kinds and the schema of a chain's `blocks`: one of the kinds, at most this many.
fn blocks_defs(b: &mut Builder, keys: &[String], ideas: bool, tempo: bool) -> serde_json::Map<String, Value> {
    stemwise_core::fx::FxKind::ALL
        .into_iter()
        .map(|k| (format!("block_{}", k.key_name()), b.block(k, keys, ideas, tempo)))
        .collect()
}

fn blocks_ref() -> Value {
    let any: Vec<Value> = stemwise_core::fx::FxKind::ALL
        .into_iter()
        .map(|k| json!({"$ref": format!("#/$defs/block_{}", k.key_name())}))
        .collect();
    json!({"type": "array", "items": {"anyOf": any}, "maxItems": 12})
}

/// The kind's name in the plan (`dynamic_eq`, `bass_enhancer`).
trait KindName {
    fn key_name(&self) -> &'static str;
}

impl KindName for stemwise_core::fx::FxKind {
    fn key_name(&self) -> &'static str {
        use stemwise_core::fx::FxKind::*;
        match self {
            Hpf => "hpf",
            Lpf => "lpf",
            Eq => "eq",
            Compressor => "compressor",
            Saturation => "saturation",
            Gate => "gate",
            Deesser => "deesser",
            DynamicEq => "dynamic_eq",
            Transient => "transient",
            Clipper => "clipper",
            Multiband => "multiband",
            Exciter => "exciter",
            BassEnhancer => "bass_enhancer",
            Width => "width",
            Modulation => "modulation",
            Tremolo => "tremolo",
            Harmonizer => "harmonizer",
            Doubler => "doubler",
        }
    }
}

fn obj(fields: Vec<(&str, Value)>) -> Value {
    let required: Vec<&str> = fields.iter().map(|(k, _)| *k).collect();
    let props: serde_json::Map<String, Value> = fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    json!({"type": "object", "properties": props, "required": required, "additionalProperties": false})
}

fn obj_owned(fields: Vec<(String, Value)>) -> Value {
    let required: Vec<&String> = fields.iter().map(|(k, _)| k).collect();
    let required = json!(required);
    let props: serde_json::Map<String, Value> = fields.into_iter().collect();
    json!({"type": "object", "properties": props, "required": required, "additionalProperties": false})
}

fn nullable(v: Value) -> Value {
    json!({"anyOf": [v, {"type": "null"}]})
}

/// Who the plan may touch, in neutral ids (AI-4, AI-5).
#[derive(Debug, Clone, Default)]
pub struct Editable {
    pub tracks: Vec<String>,
    pub groups: Vec<String>,
    pub returns: Vec<String>,
}

/// JSON Schema of the mix plan for one project (strict structured output): every editable id is a required
/// property, sends and section offsets are listed per return and nullable, and every numeric limit of [`LIMITS`] is
/// embedded. Locked or out-of-scope ids are not in it.
/// `structure` adds what the plan may create (AI-17): the `structure` object and the placeholder ids among returns,
/// groups and sends, all nullable (`null` = not used).
/// `ideas` adds the creative ideas (AI-26) and the `idea` marks; `return_blocks` the chains of returns (AI-25).
pub fn mix_plan_schema_for(
    p: &Project,
    ids: &Ids,
    ed: &Editable,
    structure: bool,
    ideas: bool,
    return_blocks: bool,
) -> Value {
    mix_schema(p, ids, ed, structure, ideas, return_blocks).0
}

fn mix_schema(
    p: &Project,
    ids: &Ids,
    ed: &Editable,
    structure: bool,
    ideas: bool,
    return_blocks: bool,
) -> (Value, Vec<&'static str>) {
    let mut b = Builder { limits: LIMITS, used: Vec::new() };
    let mut all_returns: Vec<String> = p.tracks.iter().filter(|t| t.is_return()).map(|t| ids.n(&t.id)).collect();
    if structure {
        all_returns.extend(NEW_RETURN_KEYS.iter().map(|k| k.to_string()));
    }
    let sections: Vec<String> = p.sections.iter().map(|s| ids.n(&s.id)).collect();
    let keys: Vec<String> = p.tracks.iter().filter(|t| !t.is_return()).map(|t| ids.n(&t.id)).collect();
    let sends = |b: &mut Builder| {
        obj_owned(all_returns.iter().map(|r| (r.clone(), b.num_or_null("sends_db.<return>"))).collect())
    };
    let section = |b: &mut Builder| {
        nullable(obj(vec![
            ("gain_offset_db", b.num("sections.<id>.gain_offset_db")),
            (
                "send_offsets_db",
                obj_owned(
                    all_returns
                        .iter()
                        .map(|r| (r.clone(), b.num_or_null("sections.<id>.send_offsets_db.<return>")))
                        .collect(),
                ),
            ),
            ("eq_gain_offsets_db", {
                let (lo, hi) = b.lim("sections.<id>.eq_gain_offsets_db[]");
                json!({"type": "array", "items": {"type": "number", "minimum": lo, "maximum": hi}, "maxItems": MAX_TRACK_EQ})
            }),
            ("hpf_offset_oct", b.num_or_null("sections.<id>.hpf_offset_oct")),
            ("lpf_offset_oct", b.num_or_null("sections.<id>.lpf_offset_oct")),
            ("blocks_on", json!({"type": "array", "items": {"type": "string"}})),
            ("idea", if ideas { idea_mark() } else { json!({"type": "null"}) }),
        ]))
    };
    let idea_sends = |b: &mut Builder| {
        obj_owned(
            IDEA_KEYS
                .iter()
                .map(|i| {
                    (
                        i.to_string(),
                        nullable(obj_owned(
                            all_returns.iter().map(|r| (r.clone(), b.num_or_null("sends_db.<return>"))).collect(),
                        )),
                    )
                })
                .collect(),
        )
    };
    let tracks: Vec<(String, Value)> = ed
        .tracks
        .iter()
        .map(|n| {
            let sc: Vec<String> = keys.iter().filter(|k| *k != n).cloned().collect();
            let f = vec![
                ("gain_db", b.num("tracks.<id>.gain_db")),
                ("pan", b.num("tracks.<id>.pan")),
                ("hpf_hz", b.num_or_null("tracks.<id>.hpf_hz")),
                ("lpf_hz", b.num_or_null("tracks.<id>.lpf_hz")),
                ("eq", b.eq(MAX_TRACK_EQ)),
                ("compressor", b.comp(Some(&sc))),
                ("saturation", b.sat4()),
                ("sends_db", sends(&mut b)),
                ("sections", obj_owned(sections.iter().map(|s| (s.clone(), section(&mut b))).collect())),
                ("hpf_resonance", b.num_or_null("tracks.<id>.hpf_resonance")),
                ("lpf_resonance", b.num_or_null("tracks.<id>.lpf_resonance")),
                ("blocks", blocks_ref()),
                ("idea_sends_db", if ideas { idea_sends(&mut b) } else { json!({"type": "null"}) }),
                ("rationale", json!({"type": "string", "minLength": 1})),
            ];
            (n.clone(), obj(f))
        })
        .collect();
    let group = |b: &mut Builder, n: &str| {
        let sc: Vec<String> = keys.iter().filter(|k| *k != n).cloned().collect();
        obj(vec![
            ("gain_db", b.num("groups.<id>.gain_db")),
            ("eq", b.eq(MAX_TRACK_EQ)),
            ("compressor", b.comp(Some(&sc))),
            ("saturation", b.sat4()),
            ("sends_db", sends(b)),
            ("blocks", blocks_ref()),
            ("idea_sends_db", if ideas { idea_sends(b) } else { json!({"type": "null"}) }),
            ("rationale", json!({"type": "string", "minLength": 1})),
        ])
    };
    let mut groups: Vec<(String, Value)> = ed.groups.iter().map(|n| (n.clone(), group(&mut b, n))).collect();
    if structure {
        groups.extend(NEW_GROUP_KEYS.iter().map(|n| (n.to_string(), nullable(group(&mut b, n)))));
    }
    let reverb = |b: &mut Builder| {
        obj(vec![
            ("type", json!({"type": "string", "enum": ["reverb"]})),
            ("algorithm", json!({"type": "string", "enum": ["plate", "room", "hall", "shimmer"]})),
            ("decay_s", b.num("returns.<reverb>.decay_s")),
            ("size", b.num("returns.<reverb>.size")),
            ("damping", b.num("returns.<reverb>.damping")),
            ("modulation", b.num("returns.<reverb>.modulation")),
            ("predelay_ms", b.num("returns.<reverb>.predelay_ms")),
            ("return_hpf_hz", b.num("returns.<reverb>.return_hpf_hz")),
            ("return_lpf_hz", b.num("returns.<reverb>.return_lpf_hz")),
            ("return_gain_db", b.num("returns.<reverb>.return_gain_db")),
            ("shimmer_semitones", json!({"type": ["number", "null"], "enum": [5, 7, 12, null]})),
            ("shimmer_amount", b.num_or_null("returns.<reverb>.shimmer_amount")),
            ("blocks", if return_blocks { blocks_ref() } else { json!({"type": "array", "maxItems": 0}) }),
        ])
    };
    let delay = |b: &mut Builder| {
        obj(vec![
            ("type", json!({"type": "string", "enum": ["delay"]})),
            ("time_ms", b.num("returns.<delay>.time_ms")),
            ("feedback", b.num("returns.<delay>.feedback")),
            ("feedback_hpf_hz", b.num("returns.<delay>.feedback_hpf_hz")),
            ("feedback_lpf_hz", b.num("returns.<delay>.feedback_lpf_hz")),
            ("return_gain_db", b.num("returns.<delay>.return_gain_db")),
            ("mode", json!({"type": ["string", "null"], "enum": ["stereo", "ping_pong", null]})),
            ("character", json!({"type": ["string", "null"], "enum": ["clean", "tape", null]})),
            ("wow", b.num_or_null("returns.<delay>.wow")),
            ("drive_db", b.num_or_null("returns.<delay>.drive_db")),
            ("blocks", if return_blocks { blocks_ref() } else { json!({"type": "array", "maxItems": 0}) }),
        ])
    };
    let mut returns: Vec<(String, Value)> = ed
        .returns
        .iter()
        .filter_map(|n| {
            let t = ids.to_project.get(n).and_then(|id| p.track(id))?;
            Some((n.clone(), if t.ret.as_ref()?.is_reverb() { reverb(&mut b) } else { delay(&mut b) }))
        })
        .collect();
    let mut top = Vec::new();
    if structure {
        // A new return is a reverb or a delay, as its entry in `structure` says; `null` when not created.
        for n in NEW_RETURN_KEYS {
            returns.push((n.to_string(), json!({"anyOf": [reverb(&mut b), delay(&mut b), {"type": "null"}]})));
        }
        let text = |max: usize| json!({"type": "string", "minLength": 1, "maxLength": max});
        let members: Vec<&String> = ed.tracks.iter().chain(&ed.groups).collect();
        let new_return = nullable(obj(vec![
            ("type", json!({"type": "string", "enum": ["reverb", "delay"]})),
            ("name", text(MAX_NEW_NAME)),
            ("rationale", json!({"type": "string", "minLength": 1})),
            ("idea", if ideas { idea_mark() } else { json!({"type": "null"}) }),
        ]));
        let new_group = nullable(obj(vec![
            ("name", text(MAX_NEW_NAME)),
            ("members", json!({"type": "array", "items": {"type": "string", "enum": members}, "minItems": 2})),
            ("rationale", json!({"type": "string", "minLength": 1})),
        ]));
        top.push((
            "structure",
            obj(vec![
                ("returns", obj_owned(NEW_RETURN_KEYS.iter().map(|n| (n.to_string(), new_return.clone())).collect())),
                ("groups", obj_owned(NEW_GROUP_KEYS.iter().map(|n| (n.to_string(), new_group.clone())).collect())),
            ]),
        ));
    }
    let mut fields = vec![
        ("version", json!({"type": "integer", "enum": [MIX_PLAN_VERSION]})),
        ("iteration", json!({"type": "integer", "minimum": 0})),
        ("summary", json!({"type": "string", "minLength": 1})),
    ];
    fields.extend(top);
    if ideas {
        let text = json!({"type": "string", "minLength": 1, "maxLength": MAX_IDEA_NAME});
        let idea = nullable(obj(vec![
            ("name", text),
            ("sections", json!({"type": "array", "items": {"type": "string", "enum": sections}})),
            ("rationale", json!({"type": "string", "minLength": 1})),
        ]));
        fields.push(("ideas", obj_owned(IDEA_KEYS.iter().map(|k| (k.to_string(), idea.clone())).collect())));
    }
    fields.extend([("returns", obj_owned(returns)), ("tracks", obj_owned(tracks)), ("groups", obj_owned(groups))]);
    let mut schema = obj(fields);
    // The block kinds once, referenced by every chain (AI-25): the schema stays small with many tracks.
    let keys_all: Vec<String> = keys.clone();
    schema["$defs"] = Value::Object(blocks_defs(&mut b, &keys_all, ideas, p.tempo.is_some()));
    (schema, b.used)
}

/// JSON Schema of the master plan, with every limit of [`MASTER_LIMITS`]. `sections` are the ids of the sections
/// the plan may offset (none: the plan lists no section).
pub fn master_plan_schema(sections: &[String]) -> Value {
    master_schema(sections).0
}

fn master_schema(sections: &[String]) -> (Value, Vec<&'static str>) {
    let mut b = Builder { limits: MASTER_LIMITS, used: Vec::new() };
    let text = || json!({"type": "string"});
    let kind = |k: &str| json!({"type": "string", "enum": [k]});
    let block = |k: &str, mut fields: Vec<(&str, Value)>| {
        let mut all = vec![("type", kind(k)), ("id", json!({"type": "string", "minLength": 1}))];
        all.append(&mut fields);
        all.push(("why", text()));
        obj(all)
    };
    let eq = b.eq(MAX_MASTER_EQ);
    let band = obj(vec![
        ("threshold_db", b.num("multiband.bands[].threshold_db")),
        ("ratio", b.num("multiband.bands[].ratio")),
        ("attack_ms", b.num("multiband.bands[].attack_ms")),
        ("release_ms", b.num("multiband.bands[].release_ms")),
        ("gain_db", b.num("multiband.bands[].gain_db")),
    ]);
    let xo = b.num("multiband.crossovers_hz[]");
    let fields: Vec<(&str, Vec<(&str, Value)>)> = vec![
        ("eq", vec![("bands", eq)]),
        ("match_eq", vec![("amount", b.num("match_eq.amount"))]),
        (
            "compressor",
            vec![
                ("threshold_db", b.num("compressor.threshold_db")),
                ("ratio", b.num("compressor.ratio")),
                ("attack_ms", b.num("compressor.attack_ms")),
                ("release_ms", b.num("compressor.release_ms")),
                ("knee_db", b.num("compressor.knee_db")),
                ("makeup_db", b.num("compressor.makeup_db")),
                ("mix", b.num("compressor.mix")),
                ("listen_above_hz", b.num("compressor.listen_above_hz")),
            ],
        ),
        (
            "multiband",
            vec![
                (
                    "crossovers_hz",
                    json!({"type": "array", "items": xo, "minItems": MB_BANDS.0 - 1, "maxItems": MB_BANDS.1 - 1}),
                ),
                ("bands", json!({"type": "array", "items": band, "minItems": MB_BANDS.0, "maxItems": MB_BANDS.1})),
            ],
        ),
        ("width", vec![("side_gain_db", b.num("width.side_gain_db")), ("mono_below_hz", b.num("width.mono_below_hz"))]),
        ("saturation", vec![("drive_db", b.num("saturation.drive_db")), ("mix", b.num("saturation.mix"))]),
        ("clipper", vec![("take_off_db", b.num("clipper.take_off_db")), ("softness", b.num("clipper.softness"))]),
    ];
    let blocks: Vec<Value> = fields.into_iter().map(|(k, f)| block(k, f)).collect();
    let section = obj(vec![
        // An empty `enum` is not a valid schema: without sections the list below can only be empty anyway.
        (
            "section",
            if sections.is_empty() { json!({"type": "string"}) } else { json!({"type": "string", "enum": sections}) },
        ),
        ("level_db", b.num("sections[].level_db")),
        ("side_gain_db", b.num("sections[].side_gain_db")),
        ("limiter_release_ms", b.num("sections[].limiter_release_ms")),
        (
            "eq",
            json!({"type": "array", "maxItems": 2, "items": obj(vec![
                ("block", json!({"type": "string"})),
                ("gains_db", json!({"type": "array", "items": b.num("sections[].eq[].gains_db[]"), "maxItems": MAX_MASTER_EQ})),
            ])}),
        ),
    ]);
    let reasons = ["order", "limiter", "sections"];
    let schema = obj(vec![
        ("version", json!({"type": "integer", "enum": [MASTER_PLAN_VERSION]})),
        ("summary", json!({"type": "string", "minLength": 1})),
        ("target", obj(vec![("lufs", b.num("target.lufs")), ("true_peak_db", b.num("target.true_peak_db"))])),
        ("input_gain_db", b.num("input_gain_db")),
        ("chain", json!({"type": "array", "items": {"anyOf": blocks}, "maxItems": MAX_MASTER_BLOCKS})),
        ("limiter", obj(vec![("release_ms", b.num("limiter.release_ms"))])),
        // Without sections to offset the list can only be empty.
        ("sections", json!({"type": "array", "items": section, "maxItems": sections.len()})),
        (
            "rationale",
            obj_owned(reasons.iter().map(|k| (k.to_string(), json!({"type": ["string", "null"]}))).collect()),
        ),
    ]);
    (schema, b.used)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: AI-8
    #[test]
    fn the_schema_and_the_limits_table_come_from_one_definition() {
        use stemwise_core::model::{ReturnFx, Section, Track};
        let mut p = Project::new("s");
        p.tracks.retain(|t| !t.is_return()); // a new project has one return per kind
        p.tracks.push(Track::new_audio("a".into(), "Kick", None, 1, 1));
        p.tracks.push(Track::new_audio("b".into(), "Bass", None, 1, 2));
        p.tracks.push(Track::new_group("g".into(), "Drums"));
        p.tracks.push(Track::new_return("r".into(), "Plate", ReturnFx::default_reverb()));
        p.tracks.push(Track::new_return("d".into(), "Echo", ReturnFx::default_delay()));
        p.sections.push(Section { id: "s".into(), name: "Verse".into(), color: 1, start: 0, end: 10 });
        let ids = Ids::of(&p);
        let ed = Editable { tracks: vec![ids.n("a")], groups: vec![ids.n("g")], returns: vec![ids.n("r"), ids.n("d")] };
        let (s, used) = mix_schema(&p, &ids, &ed, false, true, true);
        // Every limit of the table reaches the schema (a missing one fails here).
        for l in LIMITS {
            assert!(used.contains(&l.path), "limit {} is not in the schema", l.path);
        }
        let (_, used) = master_schema(&["s1".to_string()]);
        for l in MASTER_LIMITS {
            assert!(used.contains(&l.path), "master limit {} is not in the schema", l.path);
        }
        let t = &s["properties"]["tracks"];
        assert_eq!(t["required"], json!(["t1"]), "the locked track is not in the schema");
        assert_eq!(t["properties"]["t1"]["properties"]["gain_db"]["minimum"], -24.0);
        assert_eq!(t["properties"]["t1"]["properties"]["eq"]["maxItems"], 4);
        assert_eq!(t["properties"]["t1"]["properties"]["sends_db"]["required"], json!(["r1", "r2"]));
        assert_eq!(s["properties"]["returns"]["properties"]["r2"]["properties"]["feedback"]["maximum"], 0.7);
        // Strict mode: every object closes its properties and requires all of them.
        fn closed(v: &Value) {
            if let Some(o) = v.as_object() {
                if o.get("type") == Some(&json!("object")) {
                    assert_eq!(o["additionalProperties"], json!(false));
                    assert_eq!(o["required"].as_array().unwrap().len(), o["properties"].as_object().unwrap().len());
                }
                o.values().for_each(closed);
            } else if let Some(a) = v.as_array() {
                a.iter().for_each(closed);
            }
        }
        closed(&s);
        closed(&master_plan_schema(&["s1".to_string()]));
        let t = limits_table(LIMITS);
        assert!(t.contains("| returns.<delay>.feedback | 0 | 0.7 |"), "{t}");
        assert!(t.contains("| sections.<id>.eq_gain_offsets_db[] | -4 dB | 4 dB |"));
        // Every limit of §5.5 that is new in v2.
        for p in [
            "compressor.knee_db",
            "saturation.drive_db",
            "saturation.mix",
            "sends_db.<return>",
            "returns.<reverb>.predelay_ms",
        ] {
            limit(p);
        }
        assert!(limits_table(MASTER_LIMITS).contains("| limiter.release_ms | 20 ms | 500 ms |"));
    }

    /// req: AI-8
    #[test]
    fn exported_ai_schemas_are_up_to_date() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
        for (name, text) in json_schemas() {
            let on_disk = std::fs::read_to_string(dir.join(name)).unwrap_or_default();
            assert_eq!(on_disk, text, "{name} is stale: run `make schemas`");
        }
    }

    /// req: AI-8
    #[test]
    fn the_foundation_example_parses_and_unknown_fields_are_refused() {
        let example = r#"{
          "version": 3, "iteration": 1, "summary": "Voce avanti.",
          "returns": {
            "r1": { "type": "reverb", "algorithm": "hall", "decay_s": 2.2, "size": 0.6, "damping": 0.5, "modulation": 0.2, "predelay_ms": 20, "return_hpf_hz": 250, "return_lpf_hz": 9000, "return_gain_db": -6.0 },
            "r2": { "type": "delay", "time_ms": 375, "feedback": 0.25, "feedback_hpf_hz": 200, "feedback_lpf_hz": 6000, "return_gain_db": -9.0 }
          },
          "tracks": {
            "t4": {
              "gain_db": 2.0, "pan": 0.0, "hpf_hz": 90, "lpf_hz": null,
              "eq": [{ "type": "peak", "freq_hz": 300, "gain_db": -2.5, "q": 1.4 }],
              "compressor": { "threshold_db": -18.0, "ratio": 3.0, "attack_ms": 10.0, "release_ms": 120.0, "knee_db": 6.0, "makeup_db": 3.0, "mix": 1.0, "sidechain": null },
              "saturation": null, "sends_db": { "r1": -14.0 },
              "sections": { "s3": { "gain_offset_db": 1.5, "send_offsets_db": { "r1": -3.0 }, "eq_gain_offsets_db": [1.0] } },
              "rationale": "Taglio a 300 Hz."
            }
          },
          "groups": { "g1": { "gain_db": -1.0, "eq": [], "compressor": null, "saturation": null, "sends_db": {}, "rationale": "Batteria giù." } }
        }"#;
        let p: MixPlan = serde_json::from_str(example).unwrap();
        assert_eq!(p.tracks["t4"].sections["s3"].gain_offset_db, 1.5);
        let bad = example.replace("\"pan\": 0.0", "\"pan\": 0.0, \"width\": 1");
        assert!(serde_json::from_str::<MixPlan>(&bad).is_err());
        let master = r#"{"version": 2, "summary": "x", "target": {"lufs": -14.0, "true_peak_db": -1.0}, "input_gain_db": 0.0,
          "chain": [
            {"type": "compressor", "id": "b3", "threshold_db": -9.0, "ratio": 3.0, "attack_ms": 3.0, "release_ms": 90.0, "knee_db": 4.0, "makeup_db": 0.0, "mix": 0.6, "listen_above_hz": 120.0, "why": "a"},
            {"type": "eq", "id": "b1", "bands": [{"type": "high_shelf", "freq_hz": 10000, "gain_db": 1.0, "q": 0.7}], "why": "b"},
            {"type": "multiband", "id": "b4", "crossovers_hz": [120, 4000], "bands": [
              {"threshold_db": -14.0, "ratio": 2.5, "attack_ms": 20.0, "release_ms": 160.0, "gain_db": 0.0},
              {"threshold_db": 0.0, "ratio": 1.0, "attack_ms": 20.0, "release_ms": 160.0, "gain_db": 0.0},
              {"threshold_db": 0.0, "ratio": 1.0, "attack_ms": 20.0, "release_ms": 160.0, "gain_db": 0.0}], "why": "c"},
            {"type": "clipper", "id": "clip", "take_off_db": 4.0, "softness": 0.4, "why": "d"}],
          "limiter": {"release_ms": 80.0},
          "sections": [{"section": "s3", "level_db": 0.5, "side_gain_db": 1.0, "limiter_release_ms": 0.0, "eq": [{"block": "b1", "gains_db": [0.6]}]}],
          "rationale": {"order": "e", "limiter": "f", "sections": null}}"#;
        let m: MasterPlan = serde_json::from_str(master).unwrap();
        assert_eq!(m.chain.iter().map(MasterBlockPlan::id).collect::<Vec<_>>(), ["b3", "b1", "b4", "clip"]);
        assert_eq!(m.sections[0].eq[0].gains_db, [0.6]);
        assert_eq!(m.rationale.len(), 2, "a null reason is no reason");
        assert!(
            serde_json::from_str::<MasterPlan>(&master.replace("\"softness\": 0.4", "\"softness\": 0.4, \"x\": 1"))
                .is_err()
        );
        // Strict output lists every id with null for "none"; models also write `eq: null`.
        let nulls = example
            .replace("\"sends_db\": { \"r1\": -14.0 }", "\"sends_db\": { \"r1\": -14.0, \"r2\": null }")
            .replace("\"eq\": [],", "\"eq\": null,");
        let p: MixPlan = serde_json::from_str(&nulls).unwrap();
        assert_eq!(p.tracks["t4"].sends_db.len(), 1);
        assert!(p.groups["g1"].eq.is_empty());
    }

    /// req: AI-26
    #[test]
    fn unused_idea_ids_written_as_null_are_absent() {
        let base = json!({"version": 4, "iteration": 0, "summary": "s", "tracks": {}, "groups": {}});
        let with = |ideas: Value| {
            let mut v = base.clone();
            v["ideas"] = ideas;
            serde_json::from_value::<MixPlan>(v).unwrap()
        };
        // The answer of a real model to the strict schema: one idea, the other ids `null`.
        let one =
            with(json!({"i1": {"name": "Soft Tape Echo", "sections": [], "rationale": "r"}, "i2": null, "i3": null}));
        let ideas = one.ideas.expect("one idea");
        assert_eq!(ideas.keys().collect::<Vec<_>>(), ["i1"]);
        assert_eq!(ideas["i1"].name, "Soft Tape Echo");
        // No idea at all is the same as no `ideas`.
        assert!(with(json!({"i1": null, "i2": null, "i3": null})).ideas.is_none());
        assert!(with(Value::Null).ideas.is_none());
        assert!(serde_json::from_value::<MixPlan>(base).unwrap().ideas.is_none());
    }

    /// req: AI-26
    #[test]
    fn idea_sends_written_as_null_are_absent_at_both_levels() {
        let group = |sends: Value| {
            json!({"version": 4, "iteration": 0, "summary": "s", "tracks": {}, "groups": {"g1": {
                "gain_db": 0, "eq": [], "compressor": null, "saturation": null, "rationale": "r",
                "sends_db": {"r1": null}, "idea_sends_db": sends
            }}})
        };
        // The shape a real model wrote: one idea sending to one return, every other id `null`.
        let sends = json!({"i1": {"n1": null, "n2": -16, "r1": null}, "i2": null, "i3": null});
        let p: MixPlan = serde_json::from_value(group(sends)).unwrap();
        let s = &p.groups["g1"].idea_sends_db;
        assert_eq!(s.len(), 1);
        assert_eq!(s["i1"], BTreeMap::from([("n2".to_string(), -16.0)]));
        let p: MixPlan = serde_json::from_value(group(Value::Null)).unwrap();
        assert!(p.groups["g1"].idea_sends_db.is_empty());
    }
}
