//! Messages from the core to the interface (FOUNDATION §5.6), defined once here and exported to TypeScript.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use stemwise_core::model::Project;
use ts_rs::TS;

pub use crate::device::DeviceInfo;

/// The document after a change (its read-only mirror in the UI, §5.3).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEvent {
    pub revision: u64,
    pub project: Project,
    pub can_undo: bool,
    pub can_redo: bool,
    pub dirty: bool,
    pub path: String,
    /// Media whose file is missing from the package (UI-6, design 13d): their clips are hatched and silent.
    pub missing: Vec<String>,
    /// The open project is the demo of the guided tour (PRJ-9): an attribute of the session, never written in the
    /// document. It is not saved, autosaved, listed among the recent projects or exported.
    pub demo: bool,
}

/// What the interface needs to know about the guided tour at start (UI-17).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct TourInfo {
    /// The tour has been shown once on this Mac (its mark is in the app's data folder, not in a project).
    pub shown: bool,
    /// It starts by itself now: never shown, and not the self-test of the app.
    pub auto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct TransportState {
    pub playing: bool,
    /// Timeline position of what is heard, in frames.
    pub frame: i64,
    pub rate: u32,
    pub load: f64,
    pub xruns: u64,
    pub device: Option<String>,
    pub device_rate: Option<u32>,
    pub device_error: Option<String>,
    pub disconnected: bool,
}

/// Meters and position at ≥ 30 Hz (ENG-6). Levels are finite: −200 means silence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct MeterSnapshot {
    pub frame: i64,
    pub playing: bool,
    /// Per track: [peak L, peak R, rms L, rms R, gain reduction], dB.
    pub tracks: Vec<(String, [f32; 5])>,
    /// Bus: [peak L, peak R, rms L, rms R, true peak, momentary LUFS, short-term LUFS, correlation].
    pub bus: [f32; 8],
    pub load: f64,
    /// Spectrum of the watched track (FX-7) or of the master output (MAS-4): 96 log-spaced bands, 20 Hz–20 kHz, dBFS.
    pub spectrum: Option<Vec<f32>>,
    /// Deepest gain reduction among the master's blocks, and the limiter's, dB (master phase, MAS-4).
    pub master_gr: [f32; 2],
    /// Gain reduction of each free block of the master chain, in chain order, dB (≤ 0; 0 for blocks that reduce
    /// nothing).
    #[serde(default)]
    pub master_blocks: Vec<f32>,
    /// Gain reduction of the bands of each multiband processor (six per processor), dB.
    #[serde(default)]
    pub master_bands: Vec<f32>,
}

/// Audio preferences (SET-1).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, TS)]
pub struct AudioPrefs {
    pub device: Option<String>,
    pub buffer: Option<u32>,
}

/// Writes the TypeScript bindings of these messages.
pub fn write_bindings(dir: &std::path::Path) -> Result<(), ts_rs::ExportError> {
    let cfg = ts_rs::Config::new().with_out_dir(dir).with_large_int("number");
    ProjectEvent::export_all(&cfg)?;
    TourInfo::export_all(&cfg)?;
    TransportState::export_all(&cfg)?;
    MeterSnapshot::export_all(&cfg)?;
    AudioPrefs::export_all(&cfg)?;
    DeviceInfo::export_all(&cfg)?;
    Ok(())
}
