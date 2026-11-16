//! Messages of the master phase to the interface (F8, FOUNDATION §5.6), generated as TypeScript.

use std::path::Path;

use serde::{Deserialize, Serialize};
use stemwise_ai::apply::TrackDiff;
use stemwise_ai::mastering::MasterChange;
use stemwise_core::model::{AiCallRecord, BlockKind, ChainOp, MasterState, Mixdown};
use ts_rs::{Config, TS};

use crate::pipeline::FileReport;

/// Measures of the master as it would export (MAS-4, design 9a and 10c).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct MasterMeasures {
    pub lufs: Option<f64>,
    pub true_peak_db: f64,
    pub lra: Option<f64>,
    /// Spectral distance to the references' profile, dB.
    pub distance_db: Option<f64>,
    /// 8-band profile of the master (dB relative to its total).
    pub bands_db: Option<[f64; 8]>,
    /// True peak minus loudness: how far the peaks stand above the body.
    #[serde(default)]
    pub peak_to_loudness_db: Option<f64>,
    /// Median peak-to-RMS of 400 ms windows (the front of the peaks).
    #[serde(default)]
    pub short_crest_db: Option<f64>,
    /// How hard the limiter works over the whole song (MAS-6).
    #[serde(default)]
    pub limiter: Option<Work>,
    /// The same for each compressor, multiband and clipper that is on, by block id.
    #[serde(default)]
    pub blocks: Vec<BlockWork>,
    /// The peak at the input of each clipper processor (MAS-11): the engine needs it to play the clippers.
    #[serde(default)]
    pub clip_peak: [f64; 2],
}

/// Gain reduction of a block over the whole song (positive dB, shares in per cent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Work {
    pub max_db: f64,
    pub mean_db: f64,
    pub active_pct: f64,
    pub over_3_pct: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct BlockWork {
    pub id: String,
    pub work: Work,
}

/// A long job of the master phase: bounce (8b), export (10b), master AI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct MasterJob {
    /// "bounce", "export" or "master_ai".
    pub kind: String,
    pub progress: f64,
    pub started_ms: u64,
}

/// A reference as the A/B offers it (MAS-5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ReferenceView {
    pub id: String,
    pub name: String,
    pub lufs: Option<f64>,
    /// True peak of the converted copy (what the A/B plays).
    pub true_peak_db: f64,
}

/// A master proposal in review (design 9c; MAS-6 with AI-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct MasterProposalView {
    pub call_id: String,
    pub summary: String,
    pub diff: TrackDiff,
    pub record: AiCallRecord,
    pub request: Option<String>,
    /// What the plan changes, entry by entry: each can be applied on its own (AI-2).
    pub changes: Vec<MasterChange>,
    /// The proposed chain (15f: the rack shows it while the proposal is in review).
    pub preview: MasterState,
    /// The proposed master measured offline before applying (A-033).
    pub before: Option<MasterMeasures>,
    pub after: Option<MasterMeasures>,
    /// AI-23: received earlier and opened again, on the master as it is now.
    pub reopened: bool,
}

/// One step of the master's preparation (design 9d–9j). The interface writes the text from `kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PrepStep {
    /// "read_mixdown", "measure_mixdown", "convert_reference", "measure_references" or "loudness".
    pub kind: String,
    /// "waiting", "running", "done", "skipped", "failed" or "not_started".
    pub state: String,
    /// Where a result came from when nothing had to be done: "bounce" or "saved".
    pub source: Option<String>,
    /// Done of this step (0–1), when it can tell.
    pub fraction: f64,
    /// How long it took, once done.
    pub ms: Option<u64>,
    /// The reference it works on (a conversion), by name; the references measured (their names).
    pub references: Vec<String>,
    /// Rate of the reference before conversion.
    pub from_rate: Option<u32>,
    /// Why it was skipped or failed.
    pub detail: Option<String>,
}

/// Why the master could not be prepared (9j).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PrepError {
    /// "mixdown_unreadable" or "internal".
    pub kind: String,
    /// The step that failed (its `kind`).
    pub step: String,
    pub detail: String,
}

/// The master being prepared: the mixdown and references read and measured, the master brought to the target
/// loudness. Present only while that work runs (or after it failed); the workspace is shown when it is gone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct MasterPrep {
    /// The mixdown's number, length and rate, and how many references (the overlay's subtitle).
    pub mixdown: u32,
    pub frames: u64,
    pub rate: u32,
    pub references: u32,
    /// Done of the whole, weighted by the expected work of each step (0–1).
    pub fraction: f64,
    pub elapsed_ms: u64,
    /// Expected time left, once a step that had real work is done.
    pub eta_ms: Option<u64>,
    pub error: Option<PrepError>,
    pub steps: Vec<PrepStep>,
}

/// The master being brought to a new destination (overlay 9v): the loudness or the ceiling changed, so the gain
/// has to be found again before the change is heard. Present only while that search runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct MasterRetarget {
    /// The destination the measures on screen were made for (what is still heard).
    pub from_lufs: f64,
    pub from_true_peak_db: f64,
    /// Done of the search and of the measure that follows it (0–1; the measure starts at 0.85).
    pub fraction: f64,
    pub elapsed_ms: u64,
}

/// Everything the master workspace shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct MasterStatus {
    /// "mix" or "master" (UI-1).
    pub phase: String,
    /// The mix changed since the mixdown (MDN-2).
    pub stale: bool,
    /// "master", "mixdown" or "reference:<id>" (MAS-5).
    pub listening: String,
    /// The A/B plays every source at the master's loudness (MAS-5, on demand); otherwise each plays at its real level,
    /// the master exactly as the exported file (design 9l).
    pub listen_same: bool,
    /// Gain that normalises the master to the destination (found offline).
    pub loud_gain_db: f64,
    /// Loudness and measures are being computed after a change.
    pub analysing: bool,
    pub measures: Option<MasterMeasures>,
    pub mixdown_measures: Option<MasterMeasures>,
    /// The references' mean 8-band profile (the line over the spectrum, MAS-4).
    pub target_bands: Option<[f64; 8]>,
    pub references: Vec<ReferenceView>,
    pub job: Option<MasterJob>,
    /// The mixdown just bounced (design 8c).
    pub bounced: Option<Mixdown>,
    pub last_export: Option<Vec<FileReport>>,
    /// AAC can be exported on this system (MAS-10).
    pub aac: bool,
    /// At the same loudness, every source plays this much lower so the loudest-matched one stays under −1 dBTP
    /// (MAS-5); 0 if not needed. Reported in both modes; applied only with `listen_same`.
    pub ab_trim_db: f64,
    pub proposal: Option<MasterProposalView>,
    /// The master being prepared (overlay 9d–9j); `None` once it is ready.
    pub prep: Option<MasterPrep>,
    /// The master being brought to a new destination (overlay 9v); `None` once the new level is in place.
    pub retarget: Option<MasterRetarget>,
    /// References that could not be read when the master was prepared: left out of the comparison (9i).
    pub skipped_references: Vec<String>,
    /// Bands of a multiband heard alone (listening only): the block's id and a bit per band.
    pub solo: Option<(String, u8)>,
    /// The free blocks of the chain as the rack lists them, in order (MAS-15).
    pub chain: Vec<BlockView>,
    /// The kinds a new block may be of (MAS-15): the others are at their limit, or the chain is full.
    pub addable: Vec<BlockKind>,
    /// The target asks more of this mix than the chain gives cleanly (15g): the limiter still works hard.
    pub strained: Option<Strain>,
}

/// What the waveform of the master workspace draws (MAS-4): the mixdown, and the master as it would export. Each
/// column is (lowest sample, highest sample, RMS), both channels together: the RMS is the body of the sound, which
/// keeps its shape where the peaks are all at the ceiling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct MasterWave {
    pub mixdown: Vec<[f32; 3]>,
    /// `None` until the master has been measured once.
    pub master: Option<Vec<[f32; 3]>>,
}

/// The limiter works over 3 dB for more than a few per cent of the song (MAS-6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Strain {
    pub over_3_pct: f64,
    /// A loudness at which the limiter would stay under 3 dB (an estimate from its deepest reduction).
    pub easier_lufs: f64,
}

/// A free block of the chain for the rack: the core names it ("EQ 2") and says what kind it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct BlockView {
    pub id: String,
    pub name: String,
    pub kind: BlockKind,
    /// Which processor of its kind plays it (0 or 1): where its bands are in the meters.
    pub inst: u8,
}

/// Writes the TypeScript bindings of the master phase.
pub fn write_bindings(dir: &Path) -> Result<(), ts_rs::ExportError> {
    let cfg = Config::new().with_out_dir(dir).with_large_int("number");
    MasterStatus::export_all(&cfg)?;
    ChainOp::export_all(&cfg)?;
    MasterWave::export_all(&cfg)?;
    FileReport::export_all(&cfg)?;
    Ok(())
}
