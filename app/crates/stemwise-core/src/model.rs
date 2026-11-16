//! The project document (FOUNDATION §5.2, §5.3): the single source of truth of a song.
//!
//! Positions and lengths are in sample frames at the project rate. Levels are in dB, frequencies in Hz,
//! times in ms. A level of [`SILENCE_DB`] or below means −∞.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::role::Role;

pub use crate::fx::Filter;

/// Current version of `project.json` (PRJ-8).
pub const SCHEMA_VERSION: u32 = 4;
/// Levels at or below this value are treated as −∞ (fader fully down, send off).
pub const SILENCE_DB: f64 = -144.0;
/// Most returns a project may hold (TRK-1).
pub const MAX_RETURNS: usize = 8;
/// Deepest group nesting (TRK-3): a track inside 4 nested groups is allowed, a fifth level is not.
pub const MAX_GROUP_DEPTH: usize = 4;
/// Number of EQ bands of a track or group chain (FX-1).
pub const TRACK_EQ_BANDS: usize = 4;
/// Number of EQ bands of the master chain (MAS-1).
pub const MASTER_EQ_BANDS: usize = 6;

pub type TrackId = String;
pub type ClipId = String;
pub type SectionId = String;
pub type MediaId = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    /// Project rate: the rate of the first import (PRJ-5). `None` until something is imported.
    pub sample_rate: Option<u32>,
    pub brief: Brief,
    pub tempo: Option<Tempo>,
    /// Imported audio (PRJ-3): immutable files in `audio/`, named by content hash.
    pub media: Vec<MediaFile>,
    /// Reference tracks (PRJ-7), at most 3.
    pub references: Vec<ReferenceTrack>,
    /// Tracks in display order: audio tracks, groups and returns (TRK-1).
    pub tracks: Vec<Track>,
    pub mix_bus: MixBus,
    pub sections: Vec<Section>,
    pub snapshots: Vec<SnapshotInfo>,
    pub master: MasterState,
    pub ai: AiState,
    /// Counter for new ids; ids are never reused within a project.
    pub next_id: u64,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema, TS)]
pub struct Brief {
    pub genre: String,
    pub mood: String,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Tempo {
    pub bpm: f64,
    pub beats_per_bar: u32,
    pub beat_unit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum AudioFormat {
    Wav,
    Flac,
    Aiff,
    Mp3,
    Aac,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct MediaFile {
    /// Content hash of the stored file (BLAKE3, hex, 32 chars).
    pub id: MediaId,
    /// Path inside the package, e.g. `audio/<hash>.wav`.
    pub path: String,
    /// File name at import, shown to the user only (never sent to the model, AI-10).
    pub original_name: String,
    pub format: AudioFormat,
    pub channels: u16,
    pub sample_rate: u32,
    pub frames: u64,
    /// Source description, e.g. "PCM 24-bit" or "Float 32-bit".
    pub encoding: String,
    /// True when the file was resampled to the project rate at import (PRJ-5).
    pub converted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ReferenceTrack {
    pub id: String,
    pub media: MediaFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Audio,
    Group,
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Output {
    MixBus,
    Group(TrackId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Track {
    pub id: TrackId,
    pub kind: TrackKind,
    pub name: String,
    /// Track tint 1..=12; 0 for the neutral bus tint (groups and returns by default).
    pub color: u8,
    /// Role of an audio track (PRJ-6); `None` for groups and returns.
    pub role: Option<Role>,
    /// Channel count of the track's clips (audio tracks); 2 for groups and returns.
    pub channels: u16,
    pub output: Output,
    pub clips: Vec<Clip>,
    /// Processing chain of audio tracks and groups (FX-1).
    pub chain: Chain,
    /// Processor of a return track (FX-5, FX-6).
    pub ret: Option<ReturnFx>,
    /// Post-fader sends to returns (TRK-5).
    pub sends: Vec<Send>,
    pub automation: Vec<AutomationLane>,
    /// Lock for the AI (AI-4).
    pub locked: bool,
    /// View state (HIS-6): changing it never enters the history, but it is saved and rendered.
    pub mute: bool,
    pub solo: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Clip {
    pub id: ClipId,
    pub media: MediaId,
    /// Timeline position of the clip's first frame.
    pub start: i64,
    /// Offset into the media file of the clip's first frame.
    pub offset: u64,
    pub length: u64,
    pub gain_db: f64,
    pub fade_in: Fade,
    pub fade_out: Fade,
    pub mute: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Fade {
    pub length: u64,
    pub shape: FadeShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum FadeShape {
    Linear,
    EqualPower,
}

/// FX-1, FX-10: trim → blocks in free order → fader → pan → sends. On a return the blocks follow its processor
/// (FX-11) and the trim is not used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Chain {
    pub trim_db: f64,
    pub blocks: Vec<crate::fx::FxBlock>,
    pub fader_db: f64,
    pub pan: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Equalizer {
    pub bypass: bool,
    pub bands: Vec<EqBand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum EqKind {
    Peak,
    LowShelf,
    HighShelf,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct EqBand {
    pub enabled: bool,
    pub kind: EqKind,
    pub freq_hz: f64,
    pub gain_db: f64,
    pub q: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Compressor {
    pub bypass: bool,
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    pub knee_db: f64,
    pub makeup_db: f64,
    /// Dry/wet, 0..1: below 1 the uncompressed signal is blended back in (parallel compression, FX-3). Tracks
    /// and groups only: the master compressor keeps it at 1.
    pub mix: f64,
    /// Detector input from another track or group, post-fader (FX-3).
    pub sidechain: Option<Sidechain>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Sidechain {
    pub source: TrackId,
    /// High-pass on the detector signal (FX-3).
    pub hpf_hz: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Saturation {
    pub bypass: bool,
    pub drive_db: f64,
    pub mix: f64,
    /// FX-18; the master's saturation is always `soft`.
    #[serde(default)]
    pub character: crate::fx::SatCharacter,
}

/// The reverbs of FX-5 (`shimmer`, FX-22: a hall with a pitch shift in its recirculation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ReverbAlgorithm {
    Plate,
    Room,
    Hall,
    Shimmer,
}

impl ReverbAlgorithm {
    pub const ALL: [ReverbAlgorithm; 4] =
        [ReverbAlgorithm::Plate, ReverbAlgorithm::Room, ReverbAlgorithm::Hall, ReverbAlgorithm::Shimmer];

    /// Name shown in the interface and used for a new return.
    pub fn label(self) -> &'static str {
        match self {
            ReverbAlgorithm::Plate => "Plate",
            ReverbAlgorithm::Room => "Room",
            ReverbAlgorithm::Hall => "Hall",
            ReverbAlgorithm::Shimmer => "Shimmer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReturnFx {
    /// FX-5. The return level is the return track's fader.
    Reverb {
        bypass: bool,
        algorithm: ReverbAlgorithm,
        /// RT60 at mid frequencies, seconds.
        decay_s: f64,
        size: f64,
        damping: f64,
        modulation: f64,
        predelay_ms: f64,
        hpf_hz: f64,
        lpf_hz: f64,
        /// FX-22 (shimmer only): interval of the pitch shift in the recirculation, +5, +7 or +12 semitones.
        #[serde(default = "shimmer_semitones")]
        shimmer_semitones: f64,
        /// FX-22: 0 … 1; at 0 the shimmer is a hall.
        #[serde(default)]
        shimmer_amount: f64,
    },
    /// FX-6, FX-23.
    Delay {
        bypass: bool,
        time_ms: f64,
        /// Musical value (e.g. "1/8", "1/4.") that sets `time_ms` from the tempo, if any.
        sync: Option<String>,
        feedback: f64,
        feedback_hpf_hz: f64,
        feedback_lpf_hz: f64,
        /// FX-23: `stereo` or `ping_pong`.
        #[serde(default)]
        mode: DelayMode,
        /// FX-23: `clean` or `tape`.
        #[serde(default)]
        character: DelayCharacter,
        /// Tape wow and flutter, 0 … 1.
        #[serde(default)]
        wow: f64,
        /// Tape drive in the feedback, 0 … 12 dB, level compensated.
        #[serde(default)]
        drive_db: f64,
    },
}

fn shimmer_semitones() -> f64 {
    12.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DelayMode {
    #[default]
    Stereo,
    /// The input summed to mono, repeats alternating left and right.
    PingPong,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DelayCharacter {
    #[default]
    Clean,
    Tape,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Send {
    pub target: TrackId,
    pub level_db: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AutomationLane {
    /// Parameter id, see [`crate::params::ParamId`] (e.g. `fader`, `eq.1.gain`, `send.trk3`).
    pub param: String,
    pub points: Vec<AutomationPoint>,
    /// Who last wrote the lane: the assistant (AUT-5) or the user (design D3).
    pub author: Author,
    /// Whether the lane is shown under the track (view state).
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AutomationPoint {
    pub frame: i64,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum Author {
    User,
    Ai,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct MixBus {
    pub fader_db: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Section {
    pub id: SectionId,
    pub name: String,
    pub color: u8,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SnapshotInfo {
    pub id: String,
    pub name: String,
    pub created_ms: u64,
    /// Path inside the package (`snapshots/<id>.json`).
    pub path: String,
}

/// The mix part of a project saved by a snapshot (HIS-4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct MixState {
    pub tracks: Vec<Track>,
    pub mix_bus: MixBus,
}

// ---------------------------------------------------------------------------------------------
// Master (MDN, MAS)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct MasterState {
    pub chain: MasterChain,
    pub target: LoudnessTarget,
    /// Reference used by match EQ, A/B and the AI (an id of `references`).
    pub reference: Option<String>,
    /// Latest mixdown (MDN-1, MDN-4).
    pub mixdown: Option<Mixdown>,
    pub metadata: Metadata,
    pub export: ExportSettings,
    /// The author keeps the destination although it is far from the references (design 9t): no warning about it.
    #[serde(default)]
    pub target_kept: bool,
    /// Offsets per section (MAS-14); sections without offsets are not listed.
    #[serde(default)]
    pub sections: Vec<SectionOffsets>,
}

/// The master chain (MAS-1): the input gain, then the free blocks in the order they process, then limiter and
/// dither, which always close it (the limiter guarantees the ceiling only if nothing follows it, MAS-3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct MasterChain {
    pub input_gain_db: f64,
    pub blocks: Vec<MasterBlock>,
    pub limiter: Limiter,
    pub dither: Dither,
}

/// A free block of the master chain: it can be moved, switched off and, for some kinds, be there twice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct MasterBlock {
    /// Stable within the project ("b1", "b2"…): section offsets, notes and the engine refer to it.
    pub id: String,
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Eq(Equalizer),
    MatchEq(MatchEq),
    Compressor(MasterComp),
    Multiband(Multiband),
    Width(Width),
    Saturation(Saturation),
    Clipper(Clipper),
}

/// The kinds of free blocks, in the order a new project lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Eq,
    MatchEq,
    Compressor,
    Multiband,
    Width,
    Saturation,
    Clipper,
}

impl BlockKind {
    pub const ALL: [BlockKind; 7] = [
        BlockKind::Eq,
        BlockKind::MatchEq,
        BlockKind::Compressor,
        BlockKind::Multiband,
        BlockKind::Width,
        BlockKind::Saturation,
        BlockKind::Clipper,
    ];

    /// How many blocks of this kind a chain may hold: the match EQ and the width exist once.
    pub fn max(self) -> usize {
        match self {
            BlockKind::MatchEq | BlockKind::Width => 1,
            _ => master_limits::PER_KIND,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BlockKind::Eq => "EQ",
            BlockKind::MatchEq => "Match EQ",
            BlockKind::Compressor => "Compressor",
            BlockKind::Multiband => "Multiband",
            BlockKind::Width => "Width",
            BlockKind::Saturation => "Saturation",
            BlockKind::Clipper => "Clipper",
        }
    }
}

impl Block {
    pub fn kind(&self) -> BlockKind {
        match self {
            Block::Eq(_) => BlockKind::Eq,
            Block::MatchEq(_) => BlockKind::MatchEq,
            Block::Compressor(_) => BlockKind::Compressor,
            Block::Multiband(_) => BlockKind::Multiband,
            Block::Width(_) => BlockKind::Width,
            Block::Saturation(_) => BlockKind::Saturation,
            Block::Clipper(_) => BlockKind::Clipper,
        }
    }

    pub fn bypass(&self) -> bool {
        match self {
            Block::Eq(b) => b.bypass,
            Block::MatchEq(b) => b.bypass,
            Block::Compressor(b) => b.bypass,
            Block::Multiband(b) => b.bypass,
            Block::Width(b) => b.bypass,
            Block::Saturation(b) => b.bypass,
            Block::Clipper(b) => b.bypass,
        }
    }

    pub fn set_bypass(&mut self, on: bool) {
        match self {
            Block::Eq(b) => b.bypass = on,
            Block::MatchEq(b) => b.bypass = on,
            Block::Compressor(b) => b.bypass = on,
            Block::Multiband(b) => b.bypass = on,
            Block::Width(b) => b.bypass = on,
            Block::Saturation(b) => b.bypass = on,
            Block::Clipper(b) => b.bypass = on,
        }
    }

    /// A block of this kind as a new project has it, switched off.
    pub fn new(kind: BlockKind) -> Block {
        match kind {
            BlockKind::Eq => Block::Eq(Equalizer { bypass: true, ..Equalizer::master_default() }),
            BlockKind::MatchEq => Block::MatchEq(MatchEq { bypass: true, amount: 0.5 }),
            BlockKind::Compressor => Block::Compressor(MasterComp::default()),
            BlockKind::Multiband => Block::Multiband(Multiband::default()),
            BlockKind::Width => Block::Width(Width { bypass: true, side_gain_db: 0.0, mono_below_hz: 120.0 }),
            BlockKind::Saturation => Block::Saturation(Saturation {
                bypass: true,
                drive_db: 3.0,
                mix: 1.0,
                character: crate::fx::SatCharacter::Soft,
            }),
            BlockKind::Clipper => Block::Clipper(Clipper::default()),
        }
    }
}

/// The compressor of the master chain: stereo-linked, with a dry/wet mix (parallel compression) and a high-pass on
/// its detector (MAS-13). It has no sidechain: nothing else plays in the master.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct MasterComp {
    pub bypass: bool,
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    pub knee_db: f64,
    pub makeup_db: f64,
    /// 0..1: share of the compressed signal; under 1 the rest is the untouched input.
    pub mix: f64,
    /// The detector does not hear what is below this frequency; 0 lets it hear everything.
    pub listen_above_hz: f64,
}

impl Default for MasterComp {
    fn default() -> Self {
        MasterComp {
            bypass: true,
            threshold_db: -14.0,
            ratio: 1.8,
            attack_ms: 30.0,
            release_ms: 200.0,
            knee_db: 6.0,
            makeup_db: 0.0,
            mix: 1.0,
            listen_above_hz: 0.0,
        }
    }
}

/// Multiband compressor (MAS-12): two to six bands split by `crossovers_hz` (ascending, one fewer than the bands).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Multiband {
    pub bypass: bool,
    pub crossovers_hz: Vec<f64>,
    pub bands: Vec<MbBand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct MbBand {
    pub bypass: bool,
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    /// Level of the band after its compressor.
    pub gain_db: f64,
}

impl Default for MbBand {
    fn default() -> Self {
        MbBand { bypass: false, threshold_db: -18.0, ratio: 2.0, attack_ms: 20.0, release_ms: 160.0, gain_db: 0.0 }
    }
}

impl Default for Multiband {
    fn default() -> Self {
        Multiband { bypass: true, crossovers_hz: vec![120.0, 4000.0], bands: vec![MbBand::default(); 3] }
    }
}

impl Multiband {
    /// Splits band `i` in two at `hz` (which must fall inside it): both halves keep its settings.
    pub fn split(&mut self, i: usize, hz: f64) -> bool {
        if i >= self.bands.len() || self.bands.len() >= master_limits::MB_BANDS.1 {
            return false;
        }
        let lo = if i == 0 { master_limits::EQ_FREQ_HZ.0 } else { self.crossovers_hz[i - 1] };
        let hi = self.crossovers_hz.get(i).copied().unwrap_or(master_limits::EQ_FREQ_HZ.1);
        if !(hz > lo * master_limits::MB_MIN_RATIO && hz * master_limits::MB_MIN_RATIO < hi) {
            return false;
        }
        self.crossovers_hz.insert(i, hz);
        self.bands.insert(i, self.bands[i]);
        true
    }

    /// Removes the crossover `i`: the two bands beside it become one, with the settings of the lower one.
    pub fn merge(&mut self, i: usize) -> bool {
        if i >= self.crossovers_hz.len() || self.bands.len() <= master_limits::MB_BANDS.0 {
            return false;
        }
        self.crossovers_hz.remove(i);
        self.bands.remove(i + 1);
        true
    }
}

/// Peak clipper (MAS-11): takes up to `take_off_db` off the tallest peaks that reach it, wherever it sits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Clipper {
    pub bypass: bool,
    pub take_off_db: f64,
    /// 0 = hard … 1 = softest knee.
    pub softness: f64,
}

impl Default for Clipper {
    fn default() -> Self {
        Clipper { bypass: true, take_off_db: 2.0, softness: 0.4 }
    }
}

/// Offsets of one section on top of the song-wide master (MAS-14): they act only while the section plays and
/// glide in and out at its edges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct SectionOffsets {
    pub section: SectionId,
    /// Added to the level that reaches the limiter.
    pub level_db: f64,
    /// Added to the side gain of the width block.
    pub side_gain_db: f64,
    /// Added to the limiter's release.
    pub limiter_release_ms: f64,
    /// Added to the gains of the bands of an EQ block.
    pub eq: Vec<EqOffsets>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct EqOffsets {
    pub block: String,
    pub gains_db: Vec<f64>,
}

impl SectionOffsets {
    pub fn new(section: &SectionId) -> Self {
        SectionOffsets {
            section: section.clone(),
            level_db: 0.0,
            side_gain_db: 0.0,
            limiter_release_ms: 0.0,
            eq: Vec::new(),
        }
    }

    /// Nothing is offset: the entry can be dropped.
    pub fn is_empty(&self) -> bool {
        self.level_db == 0.0
            && self.side_gain_db == 0.0
            && self.limiter_release_ms == 0.0
            && self.eq.iter().all(|e| e.gains_db.iter().all(|g| *g == 0.0))
    }
}

/// A change to the structure of the master chain (MAS-12, MAS-15): the interface asks, the core does it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ChainOp {
    /// A new block, switched off, at a place among the free blocks (`None`: last, just before the limiter).
    Add {
        kind: BlockKind,
        at: Option<usize>,
    },
    Remove {
        id: String,
    },
    /// Moves a block to a place among the free blocks.
    Move {
        id: String,
        to: usize,
    },
    /// Sets how many bands a multiband has: more by halving its widest band, fewer by joining the two highest.
    Bands {
        id: String,
        bands: usize,
    },
    /// Splits the band of a multiband that holds `hz`, there.
    Split {
        id: String,
        hz: f64,
    },
    /// Removes a crossover of a multiband: the bands beside it become one.
    Merge {
        id: String,
        crossover: usize,
    },
    /// A block back to the settings a new one has (it stays on or off as it is).
    Reset {
        id: String,
    },
    /// The free blocks back in the order of a new project: by kind, two of a kind side by side.
    ResetOrder,
}

impl MasterState {
    /// The master after a change to its chain, and the id of the block it added (if any). Refuses what the chain
    /// cannot hold.
    pub fn chain_edit(&self, op: &ChainOp) -> Result<(MasterState, Option<String>), String> {
        let mut m = self.clone();
        let mut added = None;
        let multiband = |m: &mut MasterState, id: &str| -> Result<usize, String> {
            m.chain
                .blocks
                .iter()
                .position(|b| b.id == id && matches!(b.block, Block::Multiband(_)))
                .ok_or_else(|| "no such multiband".to_string())
        };
        match op {
            ChainOp::Add { kind, at } => {
                let id = m.chain.add(*kind, at.unwrap_or(usize::MAX)).ok_or_else(|| {
                    if m.chain.blocks.len() >= master_limits::FREE_BLOCKS {
                        "the chain is full: remove a block first".to_string()
                    } else {
                        format!("the chain already has {} {}", kind.max(), kind.label())
                    }
                })?;
                added = Some(id);
            }
            ChainOp::Remove { id } => {
                let i = m.chain.blocks.iter().position(|b| b.id == *id).ok_or("no such block")?;
                m.chain.blocks.remove(i);
                for s in &mut m.sections {
                    s.eq.retain(|e| e.block != *id);
                }
                m.sections.retain(|s| !s.is_empty());
            }
            ChainOp::Move { id, to } => {
                let i = m.chain.blocks.iter().position(|b| b.id == *id).ok_or("no such block")?;
                let b = m.chain.blocks.remove(i);
                m.chain.blocks.insert((*to).min(m.chain.blocks.len()), b);
            }
            ChainOp::Bands { id, bands } => {
                let i = multiband(&mut m, id)?;
                let Block::Multiband(mb) = &mut m.chain.blocks[i].block else { unreachable!("checked") };
                let (lo, hi) = master_limits::MB_BANDS;
                if !(lo..=hi).contains(bands) {
                    return Err(format!("a multiband has {lo} to {hi} bands"));
                }
                while mb.bands.len() > *bands {
                    mb.merge(mb.crossovers_hz.len() - 1);
                }
                while mb.bands.len() < *bands {
                    // The widest band, in octaves, is halved (the edges of the spectrum count as 20 Hz and 20 kHz).
                    let edge = |k: usize| -> (f64, f64) {
                        let lo = if k == 0 { master_limits::EQ_FREQ_HZ.0 } else { mb.crossovers_hz[k - 1] };
                        (lo, mb.crossovers_hz.get(k).copied().unwrap_or(master_limits::EQ_FREQ_HZ.1))
                    };
                    let k = (0..mb.bands.len())
                        .max_by(|a, b| {
                            let (w, v) = (edge(*a), edge(*b));
                            (w.1 / w.0).total_cmp(&(v.1 / v.0))
                        })
                        .expect("a multiband has bands");
                    let (a, b) = edge(k);
                    if !mb.split(k, (a * b).sqrt().round()) {
                        return Err("the bands are too narrow to split further".into());
                    }
                }
            }
            ChainOp::Split { id, hz } => {
                let i = multiband(&mut m, id)?;
                let Block::Multiband(mb) = &mut m.chain.blocks[i].block else { unreachable!("checked") };
                let k = mb.crossovers_hz.iter().filter(|x| **x < *hz).count();
                if !mb.split(k, *hz) {
                    return Err("a band cannot be split there: too close to a crossover, or six bands already".into());
                }
            }
            ChainOp::Reset { id } => {
                let b = m.chain.block_mut(id).ok_or("no such block")?;
                let off = b.block.bypass();
                b.block = Block::new(b.block.kind());
                b.block.set_bypass(off);
                for s in &mut m.sections {
                    s.eq.retain(|e| e.block != *id);
                }
                m.sections.retain(|s| !s.is_empty());
            }
            ChainOp::ResetOrder => {
                let rank = |b: &MasterBlock| BlockKind::ALL.iter().position(|k| *k == b.block.kind()).unwrap_or(0);
                m.chain.blocks.sort_by_key(rank);
            }
            ChainOp::Merge { id, crossover } => {
                let i = multiband(&mut m, id)?;
                let Block::Multiband(mb) = &mut m.chain.blocks[i].block else { unreachable!("checked") };
                if !mb.merge(*crossover) {
                    return Err("a multiband keeps two bands at least".into());
                }
            }
        }
        m.check()?;
        Ok((m, added))
    }
}

impl MasterChain {
    pub fn block(&self, id: &str) -> Option<&MasterBlock> {
        self.blocks.iter().find(|b| b.id == id)
    }

    pub fn block_mut(&mut self, id: &str) -> Option<&mut MasterBlock> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }

    pub fn count(&self, kind: BlockKind) -> usize {
        self.blocks.iter().filter(|b| b.block.kind() == kind).count()
    }

    /// Whether one more block of this kind fits (MAS-15).
    pub fn can_add(&self, kind: BlockKind) -> bool {
        self.blocks.len() < master_limits::FREE_BLOCKS && self.count(kind) < kind.max()
    }

    /// The first block of a kind, in chain order.
    pub fn first(&self, kind: BlockKind) -> Option<&MasterBlock> {
        self.blocks.iter().find(|b| b.block.kind() == kind)
    }

    /// An id no block of this chain has.
    pub fn free_id(&self) -> String {
        (1..).map(|n| format!("b{n}")).find(|id| self.block(id).is_none()).expect("an unused id exists")
    }

    /// Adds a new block of a kind, switched off, at `at` (clamped; the end is just before the limiter). Returns its
    /// id, or `None` when the chain is full or already has all the blocks of that kind it may have.
    pub fn add(&mut self, kind: BlockKind, at: usize) -> Option<String> {
        if !self.can_add(kind) {
            return None;
        }
        let id = self.free_id();
        self.blocks.insert(at.min(self.blocks.len()), MasterBlock { id: id.clone(), block: Block::new(kind) });
        Some(id)
    }

    /// The name a block shows: its kind, numbered in chain order when there are two ("EQ 1", "EQ 2").
    pub fn name(&self, id: &str) -> Option<String> {
        let kind = self.block(id)?.block.kind();
        if self.count(kind) < 2 {
            return Some(kind.label().to_string());
        }
        let n = self.blocks.iter().filter(|b| b.block.kind() == kind).position(|b| b.id == id)? + 1;
        Some(format!("{} {n}", kind.label()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct MatchEq {
    pub bypass: bool,
    /// 0..1 (shown as 0–100 %).
    pub amount: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Width {
    pub bypass: bool,
    pub side_gain_db: f64,
    /// Below this frequency the signal is folded to mono; 0 disables it.
    pub mono_below_hz: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Limiter {
    pub bypass: bool,
    pub release_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Dither {
    /// TPDF dither is applied only when exporting at 16 bit (MAS-1); this switch can disable it.
    pub bypass: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct LoudnessTarget {
    /// "streaming", "loud", "cd" or "custom" (MAS-2).
    pub preset: String,
    pub lufs: f64,
    pub true_peak_db: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Mixdown {
    pub number: u32,
    /// `mixdowns/mixdown-<n>.wav`.
    pub path: String,
    /// Hash of the mix state the mixdown was rendered from (MDN-4).
    pub mix_hash: String,
    pub trim_db: f64,
    pub frames: u64,
    pub created_ms: u64,
    pub lufs: Option<f64>,
    pub true_peak_db: f64,
    pub lra: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema, TS)]
pub struct Metadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub genre: String,
    pub track_number: String,
    pub isrc: String,
    /// Cover image inside the package (`masters/cover.<ext>`), if any.
    pub cover: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ExportSettings {
    pub wav: bool,
    pub flac: bool,
    pub aac: bool,
    pub sample_rate: u32,
    /// 16, 24 or 32 (float) for WAV; 16 or 24 for FLAC.
    pub bit_depth: u32,
    pub aac_kbps: u32,
    /// Destination folder outside the package; `None` means `masters/` inside it.
    pub folder: Option<String>,
}

// ---------------------------------------------------------------------------------------------
// AI (AI-10, AI-14, AI-16)
// ---------------------------------------------------------------------------------------------

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AiState {
    /// AI-7: the auto mix proposes input trims towards −23 LUFS.
    pub normalize_trim: bool,
    /// AI-18: the plan may create returns and groups. Projects saved before the option existed allow it.
    #[serde(default = "yes")]
    pub allow_structure: bool,
    /// AI-27: the plan may propose creative ideas. Projects saved before the option existed allow it.
    #[serde(default = "yes")]
    pub allow_ideas: bool,
    /// AI-16: optional spending cap for the project, in USD.
    pub spend_cap_usd: Option<f64>,
    /// Calls of this project, oldest first (AI-10, AI-14).
    pub calls: Vec<AiCallRecord>,
    /// Notes of the current proposal or of its applied result (design: numbered notes).
    pub notes: Vec<AiNote>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AiNote {
    pub number: u32,
    /// Track or group id the note is about; `None` for a note about the whole mix or master.
    pub track: Option<TrackId>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum AiCallKind {
    AutoMix,
    Revision,
    Master,
    MasterRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum AiOutcome {
    Pending,
    Applied,
    AppliedPartly,
    Discarded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct AiCallRecord {
    pub id: String,
    pub kind: AiCallKind,
    pub started_ms: u64,
    pub duration_ms: u64,
    pub model: String,
    pub effort: String,
    pub decider: String,
    pub attempts: u32,
    pub iterations: u32,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
    /// True when the cost was estimated from tokens and the price list (AI-15).
    pub cost_estimated: bool,
    pub outcome: AiOutcome,
    /// Label of the history step the proposal became, if applied (HIS-3).
    pub history_label: Option<String>,
    /// User request of a revision (AI-3).
    pub request: Option<String>,
    /// Folder of the call inside the package (`ai/<id>`).
    pub path: String,
    pub error: Option<String>,
    /// True when the call's proposal is kept in its folder: it can be reviewed and applied again later (AI-23).
    #[serde(default)]
    pub stored: bool,
}

// ---------------------------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------------------------

impl Project {
    /// A new, empty project: one reverb and one delay return (TRK-1).
    pub fn new(name: &str) -> Self {
        let mut p = Project {
            schema_version: SCHEMA_VERSION,
            name: name.to_string(),
            sample_rate: None,
            brief: Brief::default(),
            tempo: None,
            media: Vec::new(),
            references: Vec::new(),
            tracks: Vec::new(),
            mix_bus: MixBus { fader_db: 0.0 },
            sections: Vec::new(),
            snapshots: Vec::new(),
            master: MasterState::default(),
            ai: AiState::default(),
            next_id: 1,
        };
        let r1 = p.new_id("r");
        p.tracks.push(Track::new_return(r1, "Reverb", ReturnFx::default_reverb()));
        let r2 = p.new_id("r");
        p.tracks.push(Track::new_return(r2, "Delay", ReturnFx::default_delay()));
        p
    }

    /// A fresh id with the given prefix (`t` tracks, `g` groups, `r` returns, `c` clips, `s` sections …).
    pub fn new_id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}{}", self.next_id);
        self.next_id += 1;
        id
    }

    pub fn track(&self, id: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    pub fn track_mut(&mut self, id: &str) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == id)
    }

    pub fn track_index(&self, id: &str) -> Option<usize> {
        self.tracks.iter().position(|t| t.id == id)
    }

    pub fn media(&self, id: &str) -> Option<&MediaFile> {
        self.media.iter().find(|m| m.id == id)
    }

    /// End of the last clip on the timeline, in frames.
    pub fn content_end(&self) -> i64 {
        self.tracks.iter().flat_map(|t| t.clips.iter()).map(|c| c.start + c.length as i64).max().unwrap_or(0)
    }

    pub fn mix_state(&self) -> MixState {
        MixState { tracks: self.tracks.clone(), mix_bus: self.mix_bus.clone() }
    }
}

impl Track {
    pub fn new_audio(id: TrackId, name: &str, role: Option<Role>, channels: u16, color: u8) -> Self {
        Track {
            id,
            kind: TrackKind::Audio,
            name: name.to_string(),
            color,
            role,
            channels,
            output: Output::MixBus,
            clips: Vec::new(),
            chain: Chain::default(),
            ret: None,
            sends: Vec::new(),
            automation: Vec::new(),
            locked: false,
            mute: false,
            solo: false,
        }
    }

    pub fn new_group(id: TrackId, name: &str) -> Self {
        Track { kind: TrackKind::Group, role: None, channels: 2, color: 0, ..Track::new_audio(id, name, None, 2, 0) }
    }

    pub fn new_return(id: TrackId, name: &str, fx: ReturnFx) -> Self {
        let mut t = Track::new_audio(id, name, None, 2, 0);
        t.kind = TrackKind::Return;
        t.ret = Some(fx);
        t.chain.blocks.clear();
        t.chain.fader_db = -6.0;
        t
    }

    pub fn is_audio(&self) -> bool {
        self.kind == TrackKind::Audio
    }

    pub fn is_group(&self) -> bool {
        self.kind == TrackKind::Group
    }

    pub fn is_return(&self) -> bool {
        self.kind == TrackKind::Return
    }

    pub fn clip(&self, id: &str) -> Option<&Clip> {
        self.clips.iter().find(|c| c.id == id)
    }

    pub fn send_db(&self, target: &str) -> Option<f64> {
        self.sends.iter().find(|s| s.target == target).map(|s| s.level_db)
    }

    pub fn lane(&self, param: &str) -> Option<&AutomationLane> {
        self.automation.iter().find(|l| l.param == param)
    }
}

impl Clip {
    pub fn end(&self) -> i64 {
        self.start + self.length as i64
    }
}

impl Default for Fade {
    fn default() -> Self {
        Fade { length: 0, shape: FadeShape::EqualPower }
    }
}

impl Default for Chain {
    fn default() -> Self {
        Chain { trim_db: 0.0, blocks: crate::fx::classic_blocks(), fader_db: 0.0, pan: 0.0 }
    }
}

impl Equalizer {
    pub fn track_default() -> Self {
        Equalizer {
            bypass: false,
            bands: vec![
                EqBand { enabled: false, kind: EqKind::LowShelf, freq_hz: 100.0, gain_db: 0.0, q: 0.7 },
                EqBand { enabled: false, kind: EqKind::Peak, freq_hz: 400.0, gain_db: 0.0, q: 1.0 },
                EqBand { enabled: false, kind: EqKind::Peak, freq_hz: 2500.0, gain_db: 0.0, q: 1.0 },
                EqBand { enabled: false, kind: EqKind::HighShelf, freq_hz: 8000.0, gain_db: 0.0, q: 0.7 },
            ],
        }
    }

    pub fn master_default() -> Self {
        let freqs = [(EqKind::LowShelf, 60.0), (EqKind::Peak, 200.0), (EqKind::Peak, 800.0)];
        let more = [(EqKind::Peak, 2500.0), (EqKind::Peak, 6000.0), (EqKind::HighShelf, 12000.0)];
        Equalizer {
            bypass: false,
            bands: freqs
                .iter()
                .chain(more.iter())
                .map(|&(kind, freq_hz)| EqBand { enabled: false, kind, freq_hz, gain_db: 0.0, q: 0.7 })
                .collect(),
        }
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Compressor {
            bypass: true,
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 120.0,
            knee_db: 6.0,
            makeup_db: 0.0,
            mix: 1.0,
            sidechain: None,
        }
    }
}

impl ReturnFx {
    /// The reverb of a new project: a hall.
    pub fn default_reverb() -> Self {
        ReturnFx::reverb(ReverbAlgorithm::Hall)
    }

    /// A reverb with the starting values of its algorithm: a short bright plate, a small room, a longer hall.
    pub fn reverb(algorithm: ReverbAlgorithm) -> Self {
        let (decay_s, predelay_ms, hpf_hz, lpf_hz) = match algorithm {
            ReverbAlgorithm::Plate => (1.2, 30.0, 300.0, 12000.0),
            ReverbAlgorithm::Room => (0.6, 5.0, 200.0, 10000.0),
            ReverbAlgorithm::Hall => (2.0, 20.0, 250.0, 9000.0),
            ReverbAlgorithm::Shimmer => (3.5, 30.0, 300.0, 10000.0),
        };
        ReturnFx::Reverb {
            bypass: false,
            algorithm,
            decay_s,
            size: 0.5,
            damping: 0.5,
            modulation: 0.2,
            predelay_ms,
            hpf_hz,
            lpf_hz,
            shimmer_semitones: 12.0,
            shimmer_amount: if algorithm == ReverbAlgorithm::Shimmer { 0.4 } else { 0.0 },
        }
    }

    pub fn algorithm(&self) -> Option<ReverbAlgorithm> {
        match self {
            ReturnFx::Reverb { algorithm, .. } => Some(*algorithm),
            ReturnFx::Delay { .. } => None,
        }
    }

    pub fn default_delay() -> Self {
        ReturnFx::Delay {
            bypass: false,
            time_ms: 375.0,
            sync: None,
            feedback: 0.25,
            feedback_hpf_hz: 200.0,
            feedback_lpf_hz: 6000.0,
            mode: DelayMode::Stereo,
            character: DelayCharacter::Clean,
            wow: 0.0,
            drive_db: 0.0,
        }
    }

    pub fn is_reverb(&self) -> bool {
        matches!(self, ReturnFx::Reverb { .. })
    }
}

impl Default for MasterState {
    fn default() -> Self {
        MasterState {
            chain: MasterChain {
                input_gain_db: 0.0,
                // Every kind once, switched off (the EQ is on with its bands disabled): a new master is the limiter
                // alone.
                blocks: BlockKind::ALL
                    .iter()
                    .enumerate()
                    .map(|(i, k)| MasterBlock {
                        id: format!("b{}", i + 1),
                        block: if *k == BlockKind::Eq {
                            Block::Eq(Equalizer::master_default())
                        } else {
                            Block::new(*k)
                        },
                    })
                    .collect(),
                limiter: Limiter { bypass: false, release_ms: 80.0 },
                dither: Dither { bypass: false },
            },
            target: LoudnessTarget::preset("streaming").expect("known preset"),
            reference: None,
            mixdown: None,
            metadata: Metadata::default(),
            export: ExportSettings {
                wav: true,
                flac: false,
                aac: false,
                sample_rate: 48000,
                bit_depth: 24,
                aac_kbps: 256,
                folder: None,
            },
            target_kept: false,
            sections: Vec::new(),
        }
    }
}

impl LoudnessTarget {
    /// The predefined destinations of MAS-2.
    pub fn preset(name: &str) -> Option<Self> {
        let (lufs, tp) = match name {
            "streaming" => (-14.0, -1.0),
            "loud" => (-9.0, -1.0),
            "cd" => (-10.0, -0.3),
            _ => return None,
        };
        Some(LoudnessTarget { preset: name.to_string(), lufs, true_peak_db: tp })
    }
}

impl Default for AiState {
    fn default() -> Self {
        AiState {
            normalize_trim: true,
            allow_structure: true,
            allow_ideas: true,
            spend_cap_usd: None,
            calls: Vec::new(),
            notes: Vec::new(),
            summary: None,
        }
    }
}

/// Normalises an ISRC to its 12-character uppercase form, or `None` when it is not valid (MAS-9). Accepts the
/// compact form `CCXXXYYNNNNN` and the fully hyphenated `CC-XXX-YY-NNNNN`, in any letter case.
pub fn normalise_isrc(s: &str) -> Option<String> {
    let t = s.trim();
    let compact: String = if t.contains('-') {
        let parts: Vec<&str> = t.split('-').collect();
        let lens: Vec<usize> = parts.iter().map(|p| p.chars().count()).collect();
        if lens != [2, 3, 2, 5] {
            return None;
        }
        parts.concat()
    } else {
        t.to_string()
    };
    let c: Vec<char> = compact.chars().collect();
    let ok = c.len() == 12
        && c[..2].iter().all(|ch| ch.is_ascii_alphabetic())
        && c[2..5].iter().all(|ch| ch.is_ascii_alphanumeric())
        && c[5..].iter().all(|ch| ch.is_ascii_digit());
    ok.then(|| compact.to_ascii_uppercase())
}

/// Whether `s` is an ISRC in a valid format (MAS-9).
pub fn valid_isrc(s: &str) -> bool {
    normalise_isrc(s).is_some()
}

/// Ranges of the master state an edit may set (MAS-1, the author's controls; the AI's plan has narrower ones, §5.5).
pub mod master_limits {
    pub const INPUT_DB: (f64, f64) = (-12.0, 12.0);
    pub const EQ_FREQ_HZ: (f64, f64) = (20.0, 20_000.0);
    pub const EQ_GAIN_DB: (f64, f64) = (-12.0, 12.0);
    pub const EQ_Q: (f64, f64) = (0.3, 8.0);
    pub const EQ_BANDS: usize = 6;
    pub const MATCH_AMOUNT: (f64, f64) = (0.0, 1.0);
    pub const THRESHOLD_DB: (f64, f64) = (-40.0, 0.0);
    pub const RATIO: (f64, f64) = (1.0, 10.0);
    pub const ATTACK_MS: (f64, f64) = (0.1, 100.0);
    pub const RELEASE_MS: (f64, f64) = (20.0, 1000.0);
    pub const KNEE_DB: (f64, f64) = (0.0, 12.0);
    pub const MAKEUP_DB: (f64, f64) = (0.0, 12.0);
    pub const SIDE_DB: (f64, f64) = (-6.0, 6.0);
    pub const MONO_HZ: (f64, f64) = (0.0, 250.0);
    pub const DRIVE_DB: (f64, f64) = (0.0, 18.0);
    pub const SAT_MIX: (f64, f64) = (0.0, 1.0);
    pub const LIMITER_RELEASE_MS: (f64, f64) = (20.0, 500.0);
    pub const COMP_MIX: (f64, f64) = (0.0, 1.0);
    /// 0 switches the detector filter off.
    pub const LISTEN_ABOVE_HZ: (f64, f64) = (0.0, 500.0);
    pub const MB_BANDS: (usize, usize) = (2, 6);
    /// Neighbouring crossovers stay at least a third of an octave apart.
    pub const MB_MIN_RATIO: f64 = 1.26;
    pub const MB_GAIN_DB: (f64, f64) = (-12.0, 12.0);
    pub const TAKE_OFF_DB: (f64, f64) = (0.0, 12.0);
    pub const SOFTNESS: (f64, f64) = (0.0, 1.0);
    /// Blocks of one repeatable kind (MAS-15).
    pub const PER_KIND: usize = 2;
    /// Free blocks in a chain: with input, limiter and dither they make twelve.
    pub const FREE_BLOCKS: usize = 9;
    pub const SECTION_LEVEL_DB: (f64, f64) = (-3.0, 3.0);
    pub const SECTION_SIDE_DB: (f64, f64) = (-3.0, 3.0);
    pub const SECTION_RELEASE_MS: (f64, f64) = (-200.0, 200.0);
    pub const SECTION_EQ_DB: (f64, f64) = (-3.0, 3.0);
    pub const LUFS: (f64, f64) = (-30.0, -5.0);
    pub const TRUE_PEAK_DB: (f64, f64) = (-6.0, 0.0);
    pub const RATES: [u32; 4] = [44_100, 48_000, 88_200, 96_000];
    pub const BIT_DEPTHS: [u32; 3] = [16, 24, 32];
    pub const AAC_KBPS: (u32, u32) = (64, 320);
}

impl MasterState {
    /// Whether every value is one the author's controls can set (FOUNDATION §5.6: the core keeps the document valid).
    pub fn check(&self) -> Result<(), String> {
        use master_limits as L;
        let within = |name: &str, v: f64, (lo, hi): (f64, f64)| {
            if v.is_finite() && (lo..=hi).contains(&v) {
                Ok(())
            } else {
                Err(format!("{name} out of range: {lo} … {hi}"))
            }
        };
        let c = &self.chain;
        within("input gain", c.input_gain_db, L::INPUT_DB)?;
        if c.blocks.len() > L::FREE_BLOCKS {
            return Err(format!("the chain has {} blocks at most besides input, limiter and dither", L::FREE_BLOCKS));
        }
        for (i, b) in c.blocks.iter().enumerate() {
            if b.id.is_empty() || c.blocks[..i].iter().any(|x| x.id == b.id) {
                return Err("two blocks of the chain share an id".into());
            }
        }
        for kind in BlockKind::ALL {
            if c.count(kind) > kind.max() {
                return Err(format!("the chain has {} {} at most", kind.max(), kind.label()));
            }
        }
        for b in &c.blocks {
            match &b.block {
                Block::Eq(eq) => {
                    if eq.bands.len() > L::EQ_BANDS {
                        return Err(format!("a master EQ has {} bands at most", L::EQ_BANDS));
                    }
                    for band in &eq.bands {
                        within("EQ frequency", band.freq_hz, L::EQ_FREQ_HZ)?;
                        within("EQ gain", band.gain_db, L::EQ_GAIN_DB)?;
                        within("EQ Q", band.q, L::EQ_Q)?;
                    }
                }
                Block::MatchEq(m) => within("match EQ amount", m.amount, L::MATCH_AMOUNT)?,
                Block::Compressor(k) => {
                    within("threshold", k.threshold_db, L::THRESHOLD_DB)?;
                    within("ratio", k.ratio, L::RATIO)?;
                    within("attack", k.attack_ms, L::ATTACK_MS)?;
                    within("release", k.release_ms, L::RELEASE_MS)?;
                    within("knee", k.knee_db, L::KNEE_DB)?;
                    within("makeup", k.makeup_db, L::MAKEUP_DB)?;
                    within("compressor mix", k.mix, L::COMP_MIX)?;
                    within("listens above", k.listen_above_hz, L::LISTEN_ABOVE_HZ)?;
                }
                Block::Multiband(m) => {
                    let n = m.bands.len();
                    if !(L::MB_BANDS.0..=L::MB_BANDS.1).contains(&n) || m.crossovers_hz.len() + 1 != n {
                        return Err(format!("a multiband has {} to {} bands", L::MB_BANDS.0, L::MB_BANDS.1));
                    }
                    for (i, x) in m.crossovers_hz.iter().enumerate() {
                        within("crossover", *x, L::EQ_FREQ_HZ)?;
                        if i > 0 && *x < m.crossovers_hz[i - 1] * L::MB_MIN_RATIO {
                            return Err("crossovers must rise, a third of an octave apart at least".into());
                        }
                    }
                    for band in &m.bands {
                        within("band threshold", band.threshold_db, L::THRESHOLD_DB)?;
                        within("band ratio", band.ratio, L::RATIO)?;
                        within("band attack", band.attack_ms, L::ATTACK_MS)?;
                        within("band release", band.release_ms, L::RELEASE_MS)?;
                        within("band gain", band.gain_db, L::MB_GAIN_DB)?;
                    }
                }
                Block::Width(w) => {
                    within("side gain", w.side_gain_db, L::SIDE_DB)?;
                    within("mono below", w.mono_below_hz, L::MONO_HZ)?;
                }
                Block::Saturation(x) => {
                    within("drive", x.drive_db, L::DRIVE_DB)?;
                    within("saturation mix", x.mix, L::SAT_MIX)?;
                }
                Block::Clipper(k) => {
                    within("take off", k.take_off_db, L::TAKE_OFF_DB)?;
                    within("softness", k.softness, L::SOFTNESS)?;
                }
            }
        }
        for (i, o) in self.sections.iter().enumerate() {
            if self.sections[..i].iter().any(|x| x.section == o.section) {
                return Err("a section has its offsets twice".into());
            }
            within("section level", o.level_db, L::SECTION_LEVEL_DB)?;
            within("section side gain", o.side_gain_db, L::SECTION_SIDE_DB)?;
            within("section limiter release", o.limiter_release_ms, L::SECTION_RELEASE_MS)?;
            for e in &o.eq {
                let Some(Block::Eq(eq)) = c.block(&e.block).map(|b| &b.block) else {
                    return Err("section offsets name an EQ the chain does not have".into());
                };
                if e.gains_db.len() != eq.bands.len() {
                    return Err("section EQ offsets do not match the bands of their EQ".into());
                }
                for g in &e.gains_db {
                    within("section EQ gain", *g, L::SECTION_EQ_DB)?;
                }
            }
        }
        within("limiter release", c.limiter.release_ms, L::LIMITER_RELEASE_MS)?;
        within("loudness", self.target.lufs, L::LUFS)?;
        within("ceiling", self.target.true_peak_db, L::TRUE_PEAK_DB)?;
        let x = &self.export;
        if !L::RATES.contains(&x.sample_rate) {
            return Err(format!("export rate {} Hz is not offered", x.sample_rate));
        }
        if !L::BIT_DEPTHS.contains(&x.bit_depth) {
            return Err(format!("bit depth {} is not offered", x.bit_depth));
        }
        if !(L::AAC_KBPS.0..=L::AAC_KBPS.1).contains(&x.aac_kbps) {
            return Err(format!("AAC bit rate out of range: {} … {} kbit/s", L::AAC_KBPS.0, L::AAC_KBPS.1));
        }
        let isrc = self.metadata.isrc.trim();
        if !isrc.is_empty() && !valid_isrc(isrc) {
            return Err("not an ISRC: use CC-XXX-YY-NNNNN or CCXXXYYNNNNN".into());
        }
        Ok(())
    }
}
