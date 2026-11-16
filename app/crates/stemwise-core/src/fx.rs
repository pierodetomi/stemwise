//! The chain of blocks of tracks, groups and returns (FX-10 → FX-23, P-018).
//!
//! A chain is a list of blocks in free order. Every block has an id that starts with its kind's key (`hpf`, `eq`,
//! `comp2`, `gate` …): a project of schema 3 becomes the five classic blocks `hpf`, `lpf`, `eq`, `comp`, `sat`, so
//! the parameter ids of schema 3 (`eq.2.gain`, `bypass.comp`) stay valid and its automation lanes keep working.
//! A block's parameters are written `<block id>.<key>` (`gate.threshold`, `deq.1.range`) and its bypass
//! `bypass.<block id>`; the kind, hence the range of every parameter, is read from the id.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{Compressor, EqKind, Equalizer, MbBand, Multiband, Saturation, Sidechain, TrackId};
use crate::params::{ParamSpec, Unit};

/// Blocks of one kind a chain may hold (FX-10).
pub const PER_KIND: usize = 2;
/// Blocks of a track or group chain (FX-10).
pub const TRACK_BLOCKS: usize = 12;
/// Blocks after the processor of a return (FX-11).
pub const RETURN_BLOCKS: usize = 6;
/// Bands of a dynamic EQ (FX-15) and voices of a harmonizer or doubler (FX-21).
pub const MAX_BANDS_DEQ: usize = 2;
pub const MAX_VOICES: usize = 2;
/// Bands of a track multiband (FX-17: the master's processor, two to six bands).
pub const MB_BANDS: (usize, usize) = (2, 6);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct FxBlock {
    /// `<kind key>` or `<kind key><n>` (`eq`, `eq2`): unique in the chain, stable when the block moves.
    pub id: String,
    pub fx: Fx,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Fx {
    Hpf(Filter),
    Lpf(Filter),
    Eq(Equalizer),
    Compressor(Compressor),
    Saturation(Saturation),
    Gate(Gate),
    Deesser(Deesser),
    DynamicEq(DynamicEq),
    Transient(Transient),
    Clipper(TrackClipper),
    Multiband(Multiband),
    Exciter(Exciter),
    BassEnhancer(BassEnhancer),
    Width(StereoWidth),
    Modulation(Modulation),
    Tremolo(Tremolo),
    Harmonizer(Harmonizer),
    Doubler(Doubler),
}

/// The kinds of blocks, in the order the "Add block" menu lists them by family (UI-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum FxKind {
    Hpf,
    Lpf,
    Eq,
    Compressor,
    Saturation,
    Gate,
    Deesser,
    DynamicEq,
    Transient,
    Clipper,
    Multiband,
    Exciter,
    BassEnhancer,
    Width,
    Modulation,
    Tremolo,
    Harmonizer,
    Doubler,
}

/// Families of the "Add block" menu (UI-12, design 16b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum FxFamily {
    Dynamics,
    Tone,
    Space,
    Movement,
    Pitch,
}

impl FxKind {
    pub const ALL: [FxKind; 18] = [
        FxKind::Hpf,
        FxKind::Lpf,
        FxKind::Eq,
        FxKind::Compressor,
        FxKind::Saturation,
        FxKind::Gate,
        FxKind::Deesser,
        FxKind::DynamicEq,
        FxKind::Transient,
        FxKind::Clipper,
        FxKind::Multiband,
        FxKind::Exciter,
        FxKind::BassEnhancer,
        FxKind::Width,
        FxKind::Modulation,
        FxKind::Tremolo,
        FxKind::Harmonizer,
        FxKind::Doubler,
    ];

    /// The five blocks every new track and group starts with, in their starting order (FX-1).
    pub const CLASSIC: [FxKind; 5] = [FxKind::Hpf, FxKind::Lpf, FxKind::Eq, FxKind::Compressor, FxKind::Saturation];

    /// Prefix of the block ids and of the parameter ids.
    pub fn key(self) -> &'static str {
        match self {
            FxKind::Hpf => "hpf",
            FxKind::Lpf => "lpf",
            FxKind::Eq => "eq",
            FxKind::Compressor => "comp",
            FxKind::Saturation => "sat",
            FxKind::Gate => "gate",
            FxKind::Deesser => "deess",
            FxKind::DynamicEq => "deq",
            FxKind::Transient => "trans",
            FxKind::Clipper => "clip",
            FxKind::Multiband => "mb",
            FxKind::Exciter => "exc",
            FxKind::BassEnhancer => "bass",
            FxKind::Width => "width",
            FxKind::Modulation => "mod",
            FxKind::Tremolo => "trem",
            FxKind::Harmonizer => "harm",
            FxKind::Doubler => "dbl",
        }
    }

    /// The short mark of the chain marks (UI-8, UI-12).
    pub fn mark(self) -> &'static str {
        match self {
            FxKind::Hpf => "HP",
            FxKind::Lpf => "LP",
            FxKind::Eq => "EQ",
            FxKind::Compressor => "CMP",
            FxKind::Saturation => "SAT",
            FxKind::Gate => "GT",
            FxKind::Deesser => "DS",
            FxKind::DynamicEq => "DEQ",
            FxKind::Transient => "TR",
            FxKind::Clipper => "CLP",
            FxKind::Multiband => "MB",
            FxKind::Exciter => "EXC",
            FxKind::BassEnhancer => "BAS",
            FxKind::Width => "WID",
            FxKind::Modulation => "MOD",
            FxKind::Tremolo => "TRM",
            FxKind::Harmonizer => "HRM",
            FxKind::Doubler => "DBL",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FxKind::Hpf => "High-pass",
            FxKind::Lpf => "Low-pass",
            FxKind::Eq => "EQ",
            FxKind::Compressor => "Compressor",
            FxKind::Saturation => "Saturation",
            FxKind::Gate => "Gate",
            FxKind::Deesser => "De-esser",
            FxKind::DynamicEq => "Dynamic EQ",
            FxKind::Transient => "Transient",
            FxKind::Clipper => "Clipper",
            FxKind::Multiband => "Multiband",
            FxKind::Exciter => "Exciter",
            FxKind::BassEnhancer => "Bass enhancer",
            FxKind::Width => "Width",
            FxKind::Modulation => "Modulation",
            FxKind::Tremolo => "Tremolo",
            FxKind::Harmonizer => "Harmonizer",
            FxKind::Doubler => "Doubler",
        }
    }

    pub fn family(self) -> FxFamily {
        use FxKind::*;
        match self {
            Compressor | Gate | Deesser | DynamicEq | Transient | Clipper | Multiband => FxFamily::Dynamics,
            Hpf | Lpf | Eq | Saturation | Exciter | BassEnhancer => FxFamily::Tone,
            Width => FxFamily::Space,
            Modulation | Tremolo => FxFamily::Movement,
            Harmonizer | Doubler => FxFamily::Pitch,
        }
    }

    /// The kind of a block id (`comp2` → compressor).
    pub fn from_id(id: &str) -> Option<FxKind> {
        let key = id.trim_end_matches(|c: char| c.is_ascii_digit());
        FxKind::ALL.into_iter().find(|k| k.key() == key)
    }

    /// Whether the block may turn a mono chain stereo (FX-10). The tremolo only does in auto-pan mode.
    pub fn stereo(self) -> bool {
        matches!(self, FxKind::Width | FxKind::Modulation | FxKind::Harmonizer | FxKind::Doubler)
    }
}

/// High-pass or low-pass (FX-1, FX-12): 12 dB/octave, with a resonance.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Filter {
    pub bypass: bool,
    pub freq_hz: f64,
    /// Q of the filter, 0.5 … 10; 0.707 is the flat Butterworth response of schema 3 (FX-12).
    #[serde(default = "butterworth")]
    pub resonance: f64,
}

fn butterworth() -> f64 {
    std::f64::consts::FRAC_1_SQRT_2
}

/// Gate and expander (FX-13).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Gate {
    pub bypass: bool,
    pub threshold_db: f64,
    /// How far the signal goes down below the threshold.
    pub range_db: f64,
    /// 1 … 20: at 20 it is a gate, below an expander.
    pub ratio: f64,
    pub attack_ms: f64,
    pub hold_ms: f64,
    pub release_ms: f64,
    pub detector_hpf_hz: f64,
    pub detector_lpf_hz: f64,
    pub sidechain: Option<TrackId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum DeesserMode {
    /// Only what is above the frequency goes down.
    Split,
    /// The whole signal goes down.
    Wide,
}

/// De-esser (FX-14).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Deesser {
    pub bypass: bool,
    pub freq_hz: f64,
    pub threshold_db: f64,
    pub max_reduction_db: f64,
    pub mode: DeesserMode,
}

/// Dynamic EQ (FX-15): one or two bands that move only when their detector passes the threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DynamicEq {
    pub bypass: bool,
    pub bands: Vec<DeqBand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DeqBand {
    pub enabled: bool,
    pub kind: EqKind,
    pub freq_hz: f64,
    pub q: f64,
    pub threshold_db: f64,
    /// Most gain the band takes, −12 … +6 dB: negative cuts, positive lifts.
    pub range_db: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
    /// The detector hears the same band of another track or group (FX-3).
    pub sidechain: Option<TrackId>,
}

/// Transient shaper (FX-16): no threshold.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Transient {
    pub bypass: bool,
    pub attack_db: f64,
    pub sustain_db: f64,
    /// 0 … 1: how short the attack part is.
    pub speed: f64,
    pub mix: f64,
}

/// Clipper of a track (FX-17): an absolute threshold.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct TrackClipper {
    pub bypass: bool,
    pub threshold_db: f64,
    pub softness: f64,
}

/// Exciter (FX-18).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Exciter {
    pub bypass: bool,
    pub freq_hz: f64,
    pub drive_db: f64,
    pub amount: f64,
}

/// Bass enhancer (FX-18): harmonics of the low band, no octave below.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct BassEnhancer {
    pub bypass: bool,
    pub freq_hz: f64,
    pub harmonics: f64,
    pub mix: f64,
}

/// Stereo width of a track, group or return (FX-19).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct StereoWidth {
    pub bypass: bool,
    /// Gain of the side, 0 … 2 (0 … 200%).
    pub width: f64,
    pub mono_below_hz: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ModMode {
    Chorus,
    Flanger,
    Phaser,
}

/// Modulation (FX-20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Modulation {
    pub bypass: bool,
    pub mode: ModMode,
    pub rate_hz: f64,
    /// Musical value ("1/4", "1/8.") that sets the rate from the tempo, if any.
    pub sync: Option<String>,
    pub depth: f64,
    pub feedback: f64,
    pub voices: u8,
    pub stages: u8,
    pub spread_deg: f64,
    pub mix: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TremoloMode {
    Tremolo,
    Autopan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum LfoShape {
    Sine,
    Triangle,
    /// A square with rounded edges.
    Square,
}

/// Tremolo and auto-pan (FX-20).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Tremolo {
    pub bypass: bool,
    pub mode: TremoloMode,
    pub rate_hz: f64,
    pub sync: Option<String>,
    pub depth: f64,
    pub shape: LfoShape,
    /// Phase offset of the right channel in tremolo mode, degrees.
    pub phase_deg: f64,
}

/// Harmonizer (FX-21).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Harmonizer {
    pub bypass: bool,
    pub voices: Vec<HarmVoice>,
    pub mix: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct HarmVoice {
    pub enabled: bool,
    pub semitones: f64,
    pub cents: f64,
    pub level_db: f64,
    pub pan: f64,
}

/// Doubler (FX-21): voices slightly out of tune and slowly moving in time, inside the track.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct Doubler {
    pub bypass: bool,
    pub voices: u8,
    pub detune_cents: f64,
    pub delay_ms: f64,
    pub spread: f64,
    pub mix: f64,
}

/// Character of a saturation (FX-18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum SatCharacter {
    /// The waveshaper of FX-4 (and of every project before P-018).
    #[default]
    Soft,
    Tape,
    Tube,
    Transistor,
}

impl Fx {
    pub fn kind(&self) -> FxKind {
        match self {
            Fx::Hpf(_) => FxKind::Hpf,
            Fx::Lpf(_) => FxKind::Lpf,
            Fx::Eq(_) => FxKind::Eq,
            Fx::Compressor(_) => FxKind::Compressor,
            Fx::Saturation(_) => FxKind::Saturation,
            Fx::Gate(_) => FxKind::Gate,
            Fx::Deesser(_) => FxKind::Deesser,
            Fx::DynamicEq(_) => FxKind::DynamicEq,
            Fx::Transient(_) => FxKind::Transient,
            Fx::Clipper(_) => FxKind::Clipper,
            Fx::Multiband(_) => FxKind::Multiband,
            Fx::Exciter(_) => FxKind::Exciter,
            Fx::BassEnhancer(_) => FxKind::BassEnhancer,
            Fx::Width(_) => FxKind::Width,
            Fx::Modulation(_) => FxKind::Modulation,
            Fx::Tremolo(_) => FxKind::Tremolo,
            Fx::Harmonizer(_) => FxKind::Harmonizer,
            Fx::Doubler(_) => FxKind::Doubler,
        }
    }

    pub fn bypass(&self) -> bool {
        match self {
            Fx::Hpf(b) | Fx::Lpf(b) => b.bypass,
            Fx::Eq(b) => b.bypass,
            Fx::Compressor(b) => b.bypass,
            Fx::Saturation(b) => b.bypass,
            Fx::Gate(b) => b.bypass,
            Fx::Deesser(b) => b.bypass,
            Fx::DynamicEq(b) => b.bypass,
            Fx::Transient(b) => b.bypass,
            Fx::Clipper(b) => b.bypass,
            Fx::Multiband(b) => b.bypass,
            Fx::Exciter(b) => b.bypass,
            Fx::BassEnhancer(b) => b.bypass,
            Fx::Width(b) => b.bypass,
            Fx::Modulation(b) => b.bypass,
            Fx::Tremolo(b) => b.bypass,
            Fx::Harmonizer(b) => b.bypass,
            Fx::Doubler(b) => b.bypass,
        }
    }

    pub fn set_bypass(&mut self, on: bool) {
        match self {
            Fx::Hpf(b) | Fx::Lpf(b) => b.bypass = on,
            Fx::Eq(b) => b.bypass = on,
            Fx::Compressor(b) => b.bypass = on,
            Fx::Saturation(b) => b.bypass = on,
            Fx::Gate(b) => b.bypass = on,
            Fx::Deesser(b) => b.bypass = on,
            Fx::DynamicEq(b) => b.bypass = on,
            Fx::Transient(b) => b.bypass = on,
            Fx::Clipper(b) => b.bypass = on,
            Fx::Multiband(b) => b.bypass = on,
            Fx::Exciter(b) => b.bypass = on,
            Fx::BassEnhancer(b) => b.bypass = on,
            Fx::Width(b) => b.bypass = on,
            Fx::Modulation(b) => b.bypass = on,
            Fx::Tremolo(b) => b.bypass = on,
            Fx::Harmonizer(b) => b.bypass = on,
            Fx::Doubler(b) => b.bypass = on,
        }
    }

    /// A new block of a kind, as a new track has the classic ones: switched off (the EQ on with its bands disabled),
    /// with mild starting values. A block the author adds lands bypassed and opens in the inspector (design 16b).
    pub fn new(kind: FxKind) -> Fx {
        let deq_band = |kind, freq_hz, enabled| DeqBand {
            enabled,
            kind,
            freq_hz,
            q: 1.2,
            threshold_db: -30.0,
            range_db: -4.0,
            attack_ms: 10.0,
            release_ms: 120.0,
            sidechain: None,
        };
        let voice =
            |semitones: f64, pan: f64, enabled| HarmVoice { enabled, semitones, cents: 0.0, level_db: -6.0, pan };
        match kind {
            FxKind::Hpf => Fx::Hpf(Filter { bypass: true, freq_hz: 80.0, resonance: butterworth() }),
            FxKind::Lpf => Fx::Lpf(Filter { bypass: true, freq_hz: 18000.0, resonance: butterworth() }),
            FxKind::Eq => Fx::Eq(Equalizer::track_default()),
            FxKind::Compressor => Fx::Compressor(Compressor::default()),
            FxKind::Saturation => {
                Fx::Saturation(Saturation { bypass: true, drive_db: 6.0, mix: 1.0, character: SatCharacter::Soft })
            }
            FxKind::Gate => Fx::Gate(Gate {
                bypass: true,
                threshold_db: -45.0,
                range_db: 40.0,
                ratio: 20.0,
                attack_ms: 0.5,
                hold_ms: 30.0,
                release_ms: 120.0,
                detector_hpf_hz: 20.0,
                detector_lpf_hz: 20000.0,
                sidechain: None,
            }),
            FxKind::Deesser => Fx::Deesser(Deesser {
                bypass: true,
                freq_hz: 6500.0,
                threshold_db: -30.0,
                max_reduction_db: 6.0,
                mode: DeesserMode::Split,
            }),
            FxKind::DynamicEq => Fx::DynamicEq(DynamicEq {
                bypass: true,
                bands: vec![deq_band(EqKind::Peak, 3000.0, true), deq_band(EqKind::Peak, 250.0, false)],
            }),
            FxKind::Transient => {
                Fx::Transient(Transient { bypass: true, attack_db: 3.0, sustain_db: 0.0, speed: 0.5, mix: 1.0 })
            }
            FxKind::Clipper => Fx::Clipper(TrackClipper { bypass: true, threshold_db: -6.0, softness: 0.4 }),
            FxKind::Multiband => Fx::Multiband(Multiband {
                bypass: true,
                crossovers_hz: vec![200.0, 3000.0],
                bands: vec![MbBand::default(); 3],
            }),
            FxKind::Exciter => Fx::Exciter(Exciter { bypass: true, freq_hz: 5000.0, drive_db: 6.0, amount: 0.3 }),
            FxKind::BassEnhancer => {
                Fx::BassEnhancer(BassEnhancer { bypass: true, freq_hz: 90.0, harmonics: 0.5, mix: 0.5 })
            }
            FxKind::Width => Fx::Width(StereoWidth { bypass: true, width: 1.3, mono_below_hz: 0.0 }),
            FxKind::Modulation => Fx::Modulation(Modulation {
                bypass: true,
                mode: ModMode::Chorus,
                rate_hz: 0.8,
                sync: None,
                depth: 0.5,
                feedback: 0.0,
                voices: 2,
                stages: 6,
                spread_deg: 90.0,
                mix: 0.5,
            }),
            FxKind::Tremolo => Fx::Tremolo(Tremolo {
                bypass: true,
                mode: TremoloMode::Tremolo,
                rate_hz: 4.0,
                sync: None,
                depth: 0.5,
                shape: LfoShape::Sine,
                phase_deg: 0.0,
            }),
            FxKind::Harmonizer => Fx::Harmonizer(Harmonizer {
                bypass: true,
                voices: vec![voice(7.0, 0.4, true), voice(-5.0, -0.4, false)],
                mix: 0.3,
            }),
            FxKind::Doubler => Fx::Doubler(Doubler {
                bypass: true,
                voices: 2,
                detune_cents: 8.0,
                delay_ms: 15.0,
                spread: 0.8,
                mix: 0.5,
            }),
        }
    }

    /// Sidechain sources the block listens to (FX-3, FX-13, FX-15).
    pub fn sidechains(&self) -> Vec<&TrackId> {
        match self {
            Fx::Compressor(c) => c.sidechain.iter().map(|s| &s.source).collect(),
            Fx::Gate(g) => g.sidechain.iter().collect(),
            Fx::DynamicEq(d) => d.bands.iter().filter(|b| b.enabled).filter_map(|b| b.sidechain.as_ref()).collect(),
            _ => Vec::new(),
        }
    }

    /// Clears every sidechain from `source` (the source was deleted).
    pub fn drop_sidechain(&mut self, source: &str) {
        match self {
            Fx::Compressor(c) if c.sidechain.as_ref().is_some_and(|s| s.source == source) => c.sidechain = None,
            Fx::Gate(g) if g.sidechain.as_deref() == Some(source) => g.sidechain = None,
            Fx::DynamicEq(d) => {
                for b in &mut d.bands {
                    if b.sidechain.as_deref() == Some(source) {
                        b.sidechain = None;
                    }
                }
            }
            _ => {}
        }
    }

    /// Whether the block is on but leaves the sound as it is (UI-8's "on but neutral").
    pub fn neutral(&self) -> bool {
        match self {
            Fx::Eq(e) => e.bands.iter().all(|b| !b.enabled || b.gain_db == 0.0),
            Fx::Saturation(s) => s.mix == 0.0,
            Fx::Transient(t) => t.mix == 0.0 || (t.attack_db == 0.0 && t.sustain_db == 0.0),
            Fx::Exciter(e) => e.amount == 0.0,
            Fx::BassEnhancer(b) => b.mix == 0.0 || b.harmonics == 0.0,
            Fx::Width(w) => w.width == 1.0 && w.mono_below_hz == 0.0,
            Fx::Modulation(m) => m.mix == 0.0 || m.depth == 0.0,
            Fx::Tremolo(t) => t.depth == 0.0,
            Fx::Harmonizer(h) => h.mix == 0.0 || h.voices.iter().all(|v| !v.enabled),
            Fx::Doubler(d) => d.mix == 0.0,
            Fx::DynamicEq(d) => d.bands.iter().all(|b| !b.enabled || b.range_db == 0.0),
            Fx::Deesser(d) => d.max_reduction_db == 0.0,
            Fx::Gate(g) => g.range_db == 0.0,
            Fx::Multiband(m) => m.bands.iter().all(|b| b.bypass || (b.ratio == 1.0 && b.gain_db == 0.0)),
            _ => false,
        }
    }

    /// Whether, for this block's settings, a mono chain becomes stereo from it on (FX-10).
    pub fn stereo(&self) -> bool {
        match self {
            Fx::Tremolo(t) => t.mode == TremoloMode::Autopan,
            _ => self.kind().stereo(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------------------------

struct P {
    key: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    default: f64,
    unit: Unit,
    log: bool,
}

const fn p(key: &'static str, label: &'static str, min: f64, max: f64, default: f64, unit: Unit, log: bool) -> P {
    P { key, label, min, max, default, unit, log }
}

/// The continuous parameters of a kind, without the band or voice index (`freq`, `gain` for `eq.<n>.<key>`).
fn table(kind: FxKind) -> (&'static [P], &'static [P]) {
    use Unit::*;
    const HPF: [P; 2] = [
        p("freq", "Low cut", 20.0, 2000.0, 80.0, Hz, true),
        p("res", "Resonance", 0.5, 10.0, std::f64::consts::FRAC_1_SQRT_2, Ratio, true),
    ];
    const LPF: [P; 2] = [
        p("freq", "High cut", 1000.0, 20000.0, 18000.0, Hz, true),
        p("res", "Resonance", 0.5, 10.0, std::f64::consts::FRAC_1_SQRT_2, Ratio, true),
    ];
    const EQ: [P; 3] = [
        p("freq", "Frequency", 20.0, 20000.0, 1000.0, Hz, true),
        p("gain", "Gain", -18.0, 18.0, 0.0, Db, false),
        p("q", "Width (Q)", 0.1, 10.0, 1.0, Ratio, true),
    ];
    const COMP: [P; 8] = [
        p("threshold", "Threshold", -60.0, 0.0, -18.0, Db, false),
        p("ratio", "Ratio", 1.0, 20.0, 3.0, Ratio, true),
        p("attack", "Attack", 0.1, 200.0, 10.0, Ms, true),
        p("release", "Release", 10.0, 2000.0, 120.0, Ms, true),
        p("knee", "Knee", 0.0, 12.0, 6.0, Db, false),
        p("makeup", "Makeup", 0.0, 24.0, 0.0, Db, false),
        p("mix", "Mix", 0.0, 1.0, 1.0, Percent, false),
        p("sc_hpf", "Sidechain low cut", 20.0, 500.0, 100.0, Hz, true),
    ];
    const SAT: [P; 2] =
        [p("drive", "Drive", 0.0, 24.0, 6.0, Db, false), p("mix", "Mix", 0.0, 1.0, 1.0, Percent, false)];
    const GATE: [P; 8] = [
        p("threshold", "Threshold", -80.0, 0.0, -45.0, Db, false),
        p("range", "Range", 0.0, 80.0, 40.0, Db, false),
        p("ratio", "Ratio", 1.0, 20.0, 20.0, Ratio, true),
        p("attack", "Attack", 0.05, 50.0, 0.5, Ms, true),
        p("hold", "Hold", 0.0, 500.0, 30.0, Ms, false),
        p("release", "Release", 5.0, 2000.0, 120.0, Ms, true),
        p("det_hpf", "Detector low cut", 20.0, 2000.0, 20.0, Hz, true),
        p("det_lpf", "Detector high cut", 1000.0, 20000.0, 20000.0, Hz, true),
    ];
    const DEESS: [P; 3] = [
        p("freq", "Frequency", 3000.0, 12000.0, 6500.0, Hz, true),
        p("threshold", "Threshold", -60.0, 0.0, -30.0, Db, false),
        p("reduction", "Max reduction", 0.0, 12.0, 6.0, Db, false),
    ];
    const DEQ: [P; 6] = [
        p("freq", "Frequency", 20.0, 20000.0, 3000.0, Hz, true),
        p("q", "Width (Q)", 0.1, 10.0, 1.2, Ratio, true),
        p("threshold", "Threshold", -60.0, 0.0, -30.0, Db, false),
        p("range", "Range", -12.0, 6.0, -4.0, Db, false),
        p("attack", "Attack", 0.1, 100.0, 10.0, Ms, true),
        p("release", "Release", 10.0, 1000.0, 120.0, Ms, true),
    ];
    const TRANS: [P; 4] = [
        p("attack", "Attack", -12.0, 12.0, 0.0, Db, false),
        p("sustain", "Sustain", -12.0, 12.0, 0.0, Db, false),
        p("speed", "Speed", 0.0, 1.0, 0.5, Percent, false),
        p("mix", "Mix", 0.0, 1.0, 1.0, Percent, false),
    ];
    const CLIP: [P; 2] = [
        p("threshold", "Threshold", -24.0, 0.0, -6.0, Db, false),
        p("softness", "Softness", 0.0, 1.0, 0.4, Percent, false),
    ];
    const MB: [P; 5] = [
        p("threshold", "Threshold", -60.0, 0.0, -18.0, Db, false),
        p("ratio", "Ratio", 1.0, 20.0, 2.0, Ratio, true),
        p("attack", "Attack", 0.1, 200.0, 20.0, Ms, true),
        p("release", "Release", 10.0, 2000.0, 160.0, Ms, true),
        p("gain", "Gain", -12.0, 12.0, 0.0, Db, false),
    ];
    const MB_X: [P; 1] = [p("xover", "Crossover", 20.0, 20000.0, 1000.0, Hz, true)];
    const EXC: [P; 3] = [
        p("freq", "Frequency", 1000.0, 12000.0, 5000.0, Hz, true),
        p("drive", "Drive", 0.0, 24.0, 6.0, Db, false),
        p("amount", "Amount", 0.0, 1.0, 0.3, Percent, false),
    ];
    const BASS: [P; 3] = [
        p("freq", "Frequency", 40.0, 160.0, 90.0, Hz, true),
        p("harmonics", "Harmonics", 0.0, 1.0, 0.5, Percent, false),
        p("mix", "Mix", 0.0, 1.0, 0.5, Percent, false),
    ];
    const WIDTH: [P; 2] =
        [p("width", "Width", 0.0, 2.0, 1.0, Percent, false), p("mono", "Mono below", 0.0, 250.0, 0.0, Hz, false)];
    const MOD: [P; 5] = [
        p("rate", "Rate", 0.02, 10.0, 0.8, Hz, true),
        p("depth", "Depth", 0.0, 1.0, 0.5, Percent, false),
        p("feedback", "Feedback", -0.9, 0.9, 0.0, Percent, false),
        p("spread", "Stereo spread", 0.0, 180.0, 90.0, None, false),
        p("mix", "Mix", 0.0, 1.0, 0.5, Percent, false),
    ];
    const TREM: [P; 3] = [
        p("rate", "Rate", 0.1, 20.0, 4.0, Hz, true),
        p("depth", "Depth", 0.0, 1.0, 0.5, Percent, false),
        p("phase", "Stereo phase", 0.0, 180.0, 0.0, None, false),
    ];
    const HARM: [P; 1] = [p("mix", "Mix", 0.0, 1.0, 0.3, Percent, false)];
    const HARM_V: [P; 4] = [
        p("semis", "Interval", -12.0, 12.0, 7.0, None, false),
        p("cents", "Fine", -50.0, 50.0, 0.0, None, false),
        p("level", "Level", -24.0, 6.0, -6.0, Db, false),
        p("pan", "Pan", -1.0, 1.0, 0.0, Pan, false),
    ];
    const DBL: [P; 4] = [
        p("detune", "Detune", 0.0, 30.0, 8.0, None, false),
        p("delay", "Delay", 5.0, 40.0, 15.0, Ms, true),
        p("spread", "Spread", 0.0, 1.0, 0.8, Percent, false),
        p("mix", "Mix", 0.0, 1.0, 0.5, Percent, false),
    ];
    match kind {
        FxKind::Hpf => (&HPF, &[]),
        FxKind::Lpf => (&LPF, &[]),
        FxKind::Eq => (&[], &EQ),
        FxKind::Compressor => (&COMP, &[]),
        FxKind::Saturation => (&SAT, &[]),
        FxKind::Gate => (&GATE, &[]),
        FxKind::Deesser => (&DEESS, &[]),
        FxKind::DynamicEq => (&[], &DEQ),
        FxKind::Transient => (&TRANS, &[]),
        FxKind::Clipper => (&CLIP, &[]),
        FxKind::Multiband => (&MB_X, &MB),
        FxKind::Exciter => (&EXC, &[]),
        FxKind::BassEnhancer => (&BASS, &[]),
        FxKind::Width => (&WIDTH, &[]),
        FxKind::Modulation => (&MOD, &[]),
        FxKind::Tremolo => (&TREM, &[]),
        FxKind::Harmonizer => (&HARM, &HARM_V),
        FxKind::Doubler => (&DBL, &[]),
    }
}

/// How many bands or voices a kind may have: the `<n>` of `<id>.<n>.<key>`.
fn max_index(kind: FxKind) -> usize {
    match kind {
        FxKind::Eq => crate::model::TRACK_EQ_BANDS,
        FxKind::DynamicEq => MAX_BANDS_DEQ,
        FxKind::Multiband => MB_BANDS.1,
        FxKind::Harmonizer => MAX_VOICES,
        _ => 0,
    }
}

/// A parameter key of a block: `freq`, or a band's `2.gain`; the multiband's crossovers are `xover.<n>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockKey {
    /// 0-based band or voice (written 1-based), for indexed parameters.
    pub index: Option<usize>,
    pub key: String,
}

impl std::fmt::Display for BlockKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.index, self.key.as_str()) {
            (Some(i), "xover") => write!(f, "xover.{}", i + 1),
            (Some(i), k) => write!(f, "{}.{k}", i + 1),
            (None, k) => write!(f, "{k}"),
        }
    }
}

impl BlockKey {
    pub fn parse(kind: FxKind, s: &str) -> Option<BlockKey> {
        let (plain, indexed) = table(kind);
        if let Some(n) = s.strip_prefix("xover.") {
            let n: usize = n.parse().ok()?;
            return (kind == FxKind::Multiband && n >= 1 && n < MB_BANDS.1)
                .then(|| BlockKey { index: Some(n - 1), key: "xover".into() });
        }
        if let Some((n, k)) = s.split_once('.') {
            let n: usize = n.parse().ok()?;
            if n == 0 || n > max_index(kind) || !indexed.iter().any(|x| x.key == k) {
                return None;
            }
            return Some(BlockKey { index: Some(n - 1), key: k.into() });
        }
        plain.iter().any(|x| x.key == s && x.key != "xover").then(|| BlockKey { index: None, key: s.into() })
    }

    pub fn spec(&self, kind: FxKind, full_id: String) -> ParamSpec {
        let (plain, indexed) = table(kind);
        let x = plain.iter().chain(indexed).find(|x| x.key == self.key).expect("parsed keys exist in the table");
        ParamSpec {
            id: full_id,
            label: x.label.into(),
            min: x.min,
            max: x.max,
            default: x.default,
            unit: x.unit,
            log: x.log,
            stepped: false,
        }
    }

    /// Every key of a block as it is (the bands and voices it has).
    pub fn all(fx: &Fx) -> Vec<BlockKey> {
        let kind = fx.kind();
        let (plain, indexed) = table(kind);
        let mut out: Vec<BlockKey> =
            plain.iter().filter(|x| x.key != "xover").map(|x| BlockKey { index: None, key: x.key.into() }).collect();
        let n = match fx {
            Fx::Eq(e) => e.bands.len(),
            Fx::DynamicEq(d) => d.bands.len(),
            Fx::Multiband(m) => m.bands.len(),
            Fx::Harmonizer(h) => h.voices.len(),
            _ => 0,
        };
        for i in 0..n {
            out.extend(indexed.iter().map(|x| BlockKey { index: Some(i), key: x.key.into() }));
        }
        if let Fx::Multiband(m) = fx {
            out.extend((0..m.crossovers_hz.len()).map(|i| BlockKey { index: Some(i), key: "xover".into() }));
        }
        if let Fx::Compressor(c) = fx
            && c.sidechain.is_none()
        {
            out.retain(|k| k.key != "sc_hpf");
        }
        out
    }

    /// Value of the parameter on a block (`None` if the block has no such band, voice or sidechain).
    pub fn get(&self, fx: &Fx) -> Option<f64> {
        let i = self.index;
        let k = self.key.as_str();
        Some(match fx {
            Fx::Hpf(f) | Fx::Lpf(f) => match k {
                "freq" => f.freq_hz,
                _ => f.resonance,
            },
            Fx::Eq(e) => {
                let b = e.bands.get(i?)?;
                match k {
                    "freq" => b.freq_hz,
                    "gain" => b.gain_db,
                    _ => b.q,
                }
            }
            Fx::Compressor(c) => match k {
                "threshold" => c.threshold_db,
                "ratio" => c.ratio,
                "attack" => c.attack_ms,
                "release" => c.release_ms,
                "knee" => c.knee_db,
                "makeup" => c.makeup_db,
                "mix" => c.mix,
                _ => c.sidechain.as_ref()?.hpf_hz,
            },
            Fx::Saturation(s) => match k {
                "drive" => s.drive_db,
                _ => s.mix,
            },
            Fx::Gate(g) => match k {
                "threshold" => g.threshold_db,
                "range" => g.range_db,
                "ratio" => g.ratio,
                "attack" => g.attack_ms,
                "hold" => g.hold_ms,
                "release" => g.release_ms,
                "det_hpf" => g.detector_hpf_hz,
                _ => g.detector_lpf_hz,
            },
            Fx::Deesser(d) => match k {
                "freq" => d.freq_hz,
                "threshold" => d.threshold_db,
                _ => d.max_reduction_db,
            },
            Fx::DynamicEq(d) => {
                let b = d.bands.get(i?)?;
                match k {
                    "freq" => b.freq_hz,
                    "q" => b.q,
                    "threshold" => b.threshold_db,
                    "range" => b.range_db,
                    "attack" => b.attack_ms,
                    _ => b.release_ms,
                }
            }
            Fx::Transient(t) => match k {
                "attack" => t.attack_db,
                "sustain" => t.sustain_db,
                "speed" => t.speed,
                _ => t.mix,
            },
            Fx::Clipper(c) => match k {
                "threshold" => c.threshold_db,
                _ => c.softness,
            },
            Fx::Multiband(m) => {
                if k == "xover" {
                    return m.crossovers_hz.get(i?).copied();
                }
                let b = m.bands.get(i?)?;
                match k {
                    "threshold" => b.threshold_db,
                    "ratio" => b.ratio,
                    "attack" => b.attack_ms,
                    "release" => b.release_ms,
                    _ => b.gain_db,
                }
            }
            Fx::Exciter(e) => match k {
                "freq" => e.freq_hz,
                "drive" => e.drive_db,
                _ => e.amount,
            },
            Fx::BassEnhancer(b) => match k {
                "freq" => b.freq_hz,
                "harmonics" => b.harmonics,
                _ => b.mix,
            },
            Fx::Width(w) => match k {
                "width" => w.width,
                _ => w.mono_below_hz,
            },
            Fx::Modulation(m) => match k {
                "rate" => m.rate_hz,
                "depth" => m.depth,
                "feedback" => m.feedback,
                "spread" => m.spread_deg,
                _ => m.mix,
            },
            Fx::Tremolo(t) => match k {
                "rate" => t.rate_hz,
                "depth" => t.depth,
                _ => t.phase_deg,
            },
            Fx::Harmonizer(h) => {
                if k == "mix" {
                    return Some(h.mix);
                }
                let v = h.voices.get(i?)?;
                match k {
                    "semis" => v.semitones,
                    "cents" => v.cents,
                    "level" => v.level_db,
                    _ => v.pan,
                }
            }
            Fx::Doubler(d) => match k {
                "detune" => d.detune_cents,
                "delay" => d.delay_ms,
                "spread" => d.spread,
                _ => d.mix,
            },
        })
    }

    /// Sets the parameter (the range is checked by the caller).
    pub fn set(&self, fx: &mut Fx, v: f64) -> bool {
        let Some(slot) = self.slot(fx) else { return false };
        *slot = v;
        // A tempo-synced rate stops following the tempo once the author sets it by hand (like the delay, FX-6).
        match fx {
            Fx::Modulation(m) if self.key == "rate" => m.sync = None,
            Fx::Tremolo(t) if self.key == "rate" => t.sync = None,
            _ => {}
        }
        true
    }

    fn slot<'a>(&self, fx: &'a mut Fx) -> Option<&'a mut f64> {
        let i = self.index;
        let k = self.key.as_str();
        Some(match fx {
            Fx::Hpf(f) | Fx::Lpf(f) => match k {
                "freq" => &mut f.freq_hz,
                _ => &mut f.resonance,
            },
            Fx::Eq(e) => {
                let b = e.bands.get_mut(i?)?;
                match k {
                    "freq" => &mut b.freq_hz,
                    "gain" => &mut b.gain_db,
                    _ => &mut b.q,
                }
            }
            Fx::Compressor(c) => match k {
                "threshold" => &mut c.threshold_db,
                "ratio" => &mut c.ratio,
                "attack" => &mut c.attack_ms,
                "release" => &mut c.release_ms,
                "knee" => &mut c.knee_db,
                "makeup" => &mut c.makeup_db,
                "mix" => &mut c.mix,
                _ => &mut c.sidechain.as_mut()?.hpf_hz,
            },
            Fx::Saturation(s) => match k {
                "drive" => &mut s.drive_db,
                _ => &mut s.mix,
            },
            Fx::Gate(g) => match k {
                "threshold" => &mut g.threshold_db,
                "range" => &mut g.range_db,
                "ratio" => &mut g.ratio,
                "attack" => &mut g.attack_ms,
                "hold" => &mut g.hold_ms,
                "release" => &mut g.release_ms,
                "det_hpf" => &mut g.detector_hpf_hz,
                _ => &mut g.detector_lpf_hz,
            },
            Fx::Deesser(d) => match k {
                "freq" => &mut d.freq_hz,
                "threshold" => &mut d.threshold_db,
                _ => &mut d.max_reduction_db,
            },
            Fx::DynamicEq(d) => {
                let b = d.bands.get_mut(i?)?;
                match k {
                    "freq" => &mut b.freq_hz,
                    "q" => &mut b.q,
                    "threshold" => &mut b.threshold_db,
                    "range" => &mut b.range_db,
                    "attack" => &mut b.attack_ms,
                    _ => &mut b.release_ms,
                }
            }
            Fx::Transient(t) => match k {
                "attack" => &mut t.attack_db,
                "sustain" => &mut t.sustain_db,
                "speed" => &mut t.speed,
                _ => &mut t.mix,
            },
            Fx::Clipper(c) => match k {
                "threshold" => &mut c.threshold_db,
                _ => &mut c.softness,
            },
            Fx::Multiband(m) => {
                if k == "xover" {
                    return m.crossovers_hz.get_mut(i?);
                }
                let b = m.bands.get_mut(i?)?;
                match k {
                    "threshold" => &mut b.threshold_db,
                    "ratio" => &mut b.ratio,
                    "attack" => &mut b.attack_ms,
                    "release" => &mut b.release_ms,
                    _ => &mut b.gain_db,
                }
            }
            Fx::Exciter(e) => match k {
                "freq" => &mut e.freq_hz,
                "drive" => &mut e.drive_db,
                _ => &mut e.amount,
            },
            Fx::BassEnhancer(b) => match k {
                "freq" => &mut b.freq_hz,
                "harmonics" => &mut b.harmonics,
                _ => &mut b.mix,
            },
            Fx::Width(w) => match k {
                "width" => &mut w.width,
                _ => &mut w.mono_below_hz,
            },
            Fx::Modulation(m) => match k {
                "rate" => &mut m.rate_hz,
                "depth" => &mut m.depth,
                "feedback" => &mut m.feedback,
                "spread" => &mut m.spread_deg,
                _ => &mut m.mix,
            },
            Fx::Tremolo(t) => match k {
                "rate" => &mut t.rate_hz,
                "depth" => &mut t.depth,
                _ => &mut t.phase_deg,
            },
            Fx::Harmonizer(h) => {
                if k == "mix" {
                    return Some(&mut h.mix);
                }
                let v = h.voices.get_mut(i?)?;
                match k {
                    "semis" => &mut v.semitones,
                    "cents" => &mut v.cents,
                    "level" => &mut v.level_db,
                    _ => &mut v.pan,
                }
            }
            Fx::Doubler(d) => match k {
                "detune" => &mut d.detune_cents,
                "delay" => &mut d.delay_ms,
                "spread" => &mut d.spread,
                _ => &mut d.mix,
            },
        })
    }
}

/// Specs of every block parameter, one per kind and key, written with the kind's key as block id (`gate.range`,
/// `eq.<n>.gain` as `eq.1.gain` …): for the interface, which reads a block's ranges from its kind.
pub fn all_block_specs() -> Vec<ParamSpec> {
    let mut out = Vec::new();
    for kind in FxKind::ALL {
        let fx = match Fx::new(kind) {
            Fx::Compressor(mut c) => {
                c.sidechain = Some(Sidechain { source: "x".into(), hpf_hz: 100.0 });
                Fx::Compressor(c)
            }
            f => f,
        };
        for k in BlockKey::all(&fx) {
            if k.index.is_some_and(|i| i > 0) {
                continue;
            }
            out.push(k.spec(kind, format!("{}.{k}", kind.key())));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The chain
// ---------------------------------------------------------------------------------------------

/// The five classic blocks in their starting order (FX-1), with the ids schema 3 gives them.
pub fn classic_blocks() -> Vec<FxBlock> {
    FxKind::CLASSIC.iter().map(|k| FxBlock { id: k.key().into(), fx: Fx::new(*k) }).collect()
}

/// A change to the structure of a chain (FX-10, UI-12): the interface asks, the core does it. Each is one step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FxOp {
    /// A new block at a place in the chain (`None`: last).
    Add {
        kind: FxKind,
        at: Option<usize>,
    },
    Remove {
        id: String,
    },
    Move {
        id: String,
        to: usize,
    },
    /// A copy of the block right after it.
    Duplicate {
        id: String,
    },
    /// A block back to the values a new one has (it stays on or off as it is).
    Reset {
        id: String,
    },
    /// Number of bands of a multiband (two to six) or of a dynamic EQ (one or two), or voices of a harmonizer.
    Bands {
        id: String,
        bands: usize,
    },
}

impl crate::model::Chain {
    pub fn block(&self, id: &str) -> Option<&FxBlock> {
        self.blocks.iter().find(|b| b.id == id)
    }

    pub fn block_mut(&mut self, id: &str) -> Option<&mut FxBlock> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }

    pub fn count(&self, kind: FxKind) -> usize {
        self.blocks.iter().filter(|b| b.fx.kind() == kind).count()
    }

    pub fn first(&self, kind: FxKind) -> Option<&FxBlock> {
        self.blocks.iter().find(|b| b.fx.kind() == kind)
    }

    pub fn first_mut(&mut self, kind: FxKind) -> Option<&mut FxBlock> {
        self.blocks.iter_mut().find(|b| b.fx.kind() == kind)
    }

    /// An id for a new block of a kind: its key, then the key numbered from 2.
    pub fn free_id(&self, kind: FxKind) -> String {
        let key = kind.key();
        std::iter::once(key.to_string())
            .chain((2..).map(|n| format!("{key}{n}")))
            .find(|id| self.block(id).is_none())
            .expect("an unused id exists")
    }

    /// The name a block shows: its kind, numbered in chain order when there are two ("EQ 1", "EQ 2").
    pub fn name(&self, id: &str) -> Option<String> {
        let kind = self.block(id)?.fx.kind();
        if self.count(kind) < 2 {
            return Some(kind.label().to_string());
        }
        let n = self.blocks.iter().filter(|b| b.fx.kind() == kind).position(|b| b.id == id)? + 1;
        Some(format!("{} {n}", kind.label()))
    }

    /// Every sidechain source of the chain, with the block listening.
    pub fn sidechains(&self) -> Vec<(&str, &TrackId)> {
        self.blocks.iter().flat_map(|b| b.fx.sidechains().into_iter().map(move |s| (b.id.as_str(), s))).collect()
    }

    /// Classic accessors: the first block of a kind, as the AI plan of version 3 and the analysis name them.
    pub fn hpf(&self) -> Option<&Filter> {
        self.blocks.iter().find_map(|b| if let Fx::Hpf(f) = &b.fx { Some(f) } else { None })
    }

    pub fn lpf(&self) -> Option<&Filter> {
        self.blocks.iter().find_map(|b| if let Fx::Lpf(f) = &b.fx { Some(f) } else { None })
    }

    pub fn eq(&self) -> Option<&Equalizer> {
        self.blocks.iter().find_map(|b| if let Fx::Eq(f) = &b.fx { Some(f) } else { None })
    }

    pub fn comp(&self) -> Option<&Compressor> {
        self.blocks.iter().find_map(|b| if let Fx::Compressor(f) = &b.fx { Some(f) } else { None })
    }

    pub fn sat(&self) -> Option<&Saturation> {
        self.blocks.iter().find_map(|b| if let Fx::Saturation(f) = &b.fx { Some(f) } else { None })
    }

    /// The first block of a classic kind, put back at its starting place if the author removed it (the AI plan
    /// of version 3 always has the five, FX-1).
    pub fn ensure(&mut self, kind: FxKind) -> &mut Fx {
        if self.first(kind).is_none() {
            let rank = |k: FxKind| FxKind::CLASSIC.iter().position(|c| *c == k).unwrap_or(FxKind::CLASSIC.len());
            let at = self
                .blocks
                .iter()
                .position(|b| rank(b.fx.kind()) > rank(kind) && rank(b.fx.kind()) < FxKind::CLASSIC.len())
                .unwrap_or_else(|| {
                    self.blocks.iter().rposition(|b| rank(b.fx.kind()) < rank(kind)).map_or(0, |i| i + 1)
                });
            let id = self.free_id(kind);
            self.blocks.insert(at, FxBlock { id, fx: Fx::new(kind) });
        }
        &mut self.first_mut(kind).expect("just ensured").fx
    }

    pub fn hpf_mut(&mut self) -> &mut Filter {
        let Fx::Hpf(f) = self.ensure(FxKind::Hpf) else { unreachable!("kind checked") };
        f
    }

    pub fn lpf_mut(&mut self) -> &mut Filter {
        let Fx::Lpf(f) = self.ensure(FxKind::Lpf) else { unreachable!("kind checked") };
        f
    }

    pub fn eq_mut(&mut self) -> &mut Equalizer {
        let Fx::Eq(f) = self.ensure(FxKind::Eq) else { unreachable!("kind checked") };
        f
    }

    pub fn comp_mut(&mut self) -> &mut Compressor {
        let Fx::Compressor(f) = self.ensure(FxKind::Compressor) else { unreachable!("kind checked") };
        f
    }

    pub fn sat_mut(&mut self) -> &mut Saturation {
        let Fx::Saturation(f) = self.ensure(FxKind::Saturation) else { unreachable!("kind checked") };
        f
    }

    /// The most blocks this chain may hold: 12 on a track or a group, 6 after the processor of a return.
    pub fn capacity(ret: bool) -> usize {
        if ret { RETURN_BLOCKS } else { TRACK_BLOCKS }
    }

    /// Whether one more block of this kind fits (FX-10, FX-11).
    pub fn can_add(&self, kind: FxKind, ret: bool) -> Result<(), String> {
        if self.blocks.len() >= Self::capacity(ret) {
            return Err(format!("the chain holds {} blocks at most: remove one first", Self::capacity(ret)));
        }
        if self.count(kind) >= PER_KIND {
            return Err(format!("the chain already has {PER_KIND} {}", kind.label()));
        }
        Ok(())
    }

    /// The chain after a structural change, and the id of the block it added (if any).
    pub fn edit(&self, op: &FxOp, ret: bool) -> Result<(crate::model::Chain, Option<String>), String> {
        let mut c = self.clone();
        let mut added = None;
        let pos = |c: &crate::model::Chain, id: &str| c.blocks.iter().position(|b| b.id == id).ok_or("no such block");
        match op {
            FxOp::Add { kind, at } => {
                c.can_add(*kind, ret)?;
                let id = c.free_id(*kind);
                let at = at.unwrap_or(usize::MAX).min(c.blocks.len());
                c.blocks.insert(at, FxBlock { id: id.clone(), fx: Fx::new(*kind) });
                added = Some(id);
            }
            FxOp::Remove { id } => {
                let i = pos(&c, id)?;
                c.blocks.remove(i);
            }
            FxOp::Move { id, to } => {
                let i = pos(&c, id)?;
                let b = c.blocks.remove(i);
                c.blocks.insert((*to).min(c.blocks.len()), b);
            }
            FxOp::Duplicate { id } => {
                let i = pos(&c, id)?;
                let kind = c.blocks[i].fx.kind();
                c.can_add(kind, ret)?;
                let nid = c.free_id(kind);
                let fx = c.blocks[i].fx.clone();
                c.blocks.insert(i + 1, FxBlock { id: nid.clone(), fx });
                added = Some(nid);
            }
            FxOp::Reset { id } => {
                let i = pos(&c, id)?;
                let off = c.blocks[i].fx.bypass();
                let mut fresh = Fx::new(c.blocks[i].fx.kind());
                fresh.set_bypass(off);
                c.blocks[i].fx = fresh;
            }
            FxOp::Bands { id, bands } => {
                let i = pos(&c, id)?;
                let n = *bands;
                match &mut c.blocks[i].fx {
                    Fx::Multiband(mb) => {
                        if !(MB_BANDS.0..=MB_BANDS.1).contains(&n) {
                            return Err(format!("a multiband has {} to {} bands", MB_BANDS.0, MB_BANDS.1));
                        }
                        set_mb_bands(mb, n)?;
                    }
                    Fx::DynamicEq(d) => {
                        if !(1..=MAX_BANDS_DEQ).contains(&n) {
                            return Err("a dynamic EQ has one or two bands".into());
                        }
                        let fresh = match Fx::new(FxKind::DynamicEq) {
                            Fx::DynamicEq(f) => f.bands,
                            _ => unreachable!("kind"),
                        };
                        d.bands.truncate(n);
                        while d.bands.len() < n {
                            let mut b = fresh[d.bands.len()].clone();
                            b.enabled = true;
                            d.bands.push(b);
                        }
                    }
                    Fx::Harmonizer(h) => {
                        if !(1..=MAX_VOICES).contains(&n) {
                            return Err("a harmonizer has one or two voices".into());
                        }
                        let fresh = match Fx::new(FxKind::Harmonizer) {
                            Fx::Harmonizer(f) => f.voices,
                            _ => unreachable!("kind"),
                        };
                        h.voices.truncate(n);
                        while h.voices.len() < n {
                            let mut v = fresh[h.voices.len()];
                            v.enabled = true;
                            h.voices.push(v);
                        }
                    }
                    Fx::Doubler(d) => {
                        if !(1..=MAX_VOICES).contains(&n) {
                            return Err("a doubler has one or two voices".into());
                        }
                        d.voices = n as u8;
                    }
                    _ => return Err("this block has no bands".into()),
                }
            }
        }
        c.check(ret)?;
        Ok((c, added))
    }

    /// Whether every block is one the author's controls can make (FX-10 → FX-23).
    pub fn check(&self, ret: bool) -> Result<(), String> {
        if self.blocks.len() > Self::capacity(ret) {
            return Err(format!("the chain holds {} blocks at most", Self::capacity(ret)));
        }
        for (i, b) in self.blocks.iter().enumerate() {
            if self.blocks[..i].iter().any(|x| x.id == b.id) {
                return Err("two blocks of the chain share an id".into());
            }
            if FxKind::from_id(&b.id) != Some(b.fx.kind()) {
                return Err(format!("block id `{}` does not name a {}", b.id, b.fx.kind().label()));
            }
        }
        for kind in FxKind::ALL {
            if self.count(kind) > PER_KIND {
                return Err(format!("the chain has {PER_KIND} {} at most", kind.label()));
            }
        }
        for b in &self.blocks {
            for k in BlockKey::all(&b.fx) {
                let v = k.get(&b.fx).unwrap_or(f64::NAN);
                let sp = k.spec(b.fx.kind(), String::new());
                if !(v.is_finite() && v >= sp.min && v <= sp.max) {
                    return Err(format!("{} {} out of range: {} … {}", b.fx.kind().label(), sp.label, sp.min, sp.max));
                }
            }
            match &b.fx {
                Fx::Eq(e) if e.bands.len() > crate::model::TRACK_EQ_BANDS => {
                    return Err("a track EQ has four bands".into());
                }
                Fx::Multiband(m) => {
                    let n = m.bands.len();
                    if !(MB_BANDS.0..=MB_BANDS.1).contains(&n) || m.crossovers_hz.len() + 1 != n {
                        return Err(format!("a multiband has {} to {} bands", MB_BANDS.0, MB_BANDS.1));
                    }
                    for w in m.crossovers_hz.windows(2) {
                        if w[1] < w[0] * crate::model::master_limits::MB_MIN_RATIO {
                            return Err("crossovers must rise, a third of an octave apart at least".into());
                        }
                    }
                }
                Fx::DynamicEq(d) if d.bands.is_empty() || d.bands.len() > MAX_BANDS_DEQ => {
                    return Err("a dynamic EQ has one or two bands".into());
                }
                Fx::Harmonizer(h) if h.voices.is_empty() || h.voices.len() > MAX_VOICES => {
                    return Err("a harmonizer has one or two voices".into());
                }
                Fx::Doubler(d) if !(1..=MAX_VOICES as u8).contains(&d.voices) => {
                    return Err("a doubler has one or two voices".into());
                }
                Fx::Modulation(m) if !(1..=3).contains(&m.voices) || ![4, 6, 8, 12].contains(&m.stages) => {
                    return Err("a chorus has 1 to 3 voices, a phaser 4, 6, 8 or 12 stages".into());
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Sets the number of bands of a multiband: more by halving its widest band, fewer by joining the two highest.
pub fn set_mb_bands(mb: &mut Multiband, bands: usize) -> Result<(), String> {
    use crate::model::master_limits as L;
    while mb.bands.len() > bands {
        mb.merge(mb.crossovers_hz.len() - 1);
    }
    while mb.bands.len() < bands {
        let edge = |mb: &Multiband, k: usize| -> (f64, f64) {
            let lo = if k == 0 { L::EQ_FREQ_HZ.0 } else { mb.crossovers_hz[k - 1] };
            (lo, mb.crossovers_hz.get(k).copied().unwrap_or(L::EQ_FREQ_HZ.1))
        };
        let k = (0..mb.bands.len())
            .max_by(|a, b| {
                let (w, v) = (edge(mb, *a), edge(mb, *b));
                (w.1 / w.0).total_cmp(&(v.1 / v.0))
            })
            .expect("a multiband has bands");
        let (a, b) = edge(mb, k);
        if !mb.split(k, (a * b).sqrt().round()) {
            return Err("the bands are too narrow to split further".into());
        }
    }
    Ok(())
}

/// Plain-language words for a block's state (UI-8 tooltips): kind, then "off", "on, neutral" or "on".
pub fn describe(b: &FxBlock) -> String {
    let state = if b.fx.bypass() {
        "off"
    } else if b.fx.neutral() {
        "on, neutral"
    } else {
        "on"
    };
    format!("{} {state}", b.fx.kind().label())
}

/// One kind of block as the interface sees it (UI-12): names, mark, family, a new block, and where each continuous
/// parameter lives in it. Generated from the Rust types (`fx_catalog.json`), so the interface and the mock backend
/// never restate them.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct FxInfo {
    pub kind: FxKind,
    /// Prefix of the block ids and parameter ids.
    pub key: String,
    pub mark: String,
    pub label: String,
    pub family: FxFamily,
    /// Whether the block may make a mono chain stereo.
    pub stereo: bool,
    /// Latency the block adds to every chain of its class (frames at 48 kHz; the engine pads the others to it).
    pub latency: usize,
    /// A new block of this kind.
    pub new: Fx,
    /// Continuous parameters: key (`freq`, `<n>.gain`, `xover.<n>`) and the JSON path of its value in the block,
    /// `<n>` standing for the band or voice (0-based in the path).
    pub params: Vec<FxParamPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct FxParamPath {
    pub key: String,
    pub path: String,
    pub indexed: bool,
}

/// The catalog of block kinds, in menu order. `latency` is filled by the engine's export (it owns the processors).
pub fn catalog() -> Vec<FxInfo> {
    FxKind::ALL
        .into_iter()
        .map(|kind| {
            let mut probe = Fx::new(kind);
            if let Fx::Compressor(c) = &mut probe {
                c.sidechain = Some(Sidechain { source: "x".into(), hpf_hz: 100.0 });
            }
            let mut params = Vec::new();
            let base = serde_json::to_value(&probe).expect("serialises");
            for k in BlockKey::all(&probe) {
                if k.index.is_some_and(|i| i > 0) {
                    continue;
                }
                let mut changed = probe.clone();
                k.set(&mut changed, 12345.678);
                let v = serde_json::to_value(&changed).expect("serialises");
                let path = diff_path(&base, &v, "").expect("a parameter changes one value");
                let indexed = k.index.is_some();
                // The first band's path, with its index as `<n>`.
                let path = if indexed { path.replacen(".0.", ".<n>.", 1).replacen(".0", ".<n>", 1) } else { path };
                let key = match k.index {
                    Some(_) if k.key == "xover" => "xover.<n>".to_string(),
                    Some(_) => format!("<n>.{}", k.key),
                    None => k.key.clone(),
                };
                params.push(FxParamPath { key, path, indexed });
            }
            FxInfo {
                kind,
                key: kind.key().into(),
                mark: kind.mark().into(),
                label: kind.label().into(),
                family: kind.family(),
                stereo: kind.stereo(),
                latency: 0,
                new: Fx::new(kind),
                params,
            }
        })
        .collect()
}

fn diff_path(a: &serde_json::Value, b: &serde_json::Value, at: &str) -> Option<String> {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            x.iter().find_map(|(k, v)| y.get(k).and_then(|w| diff_path(v, w, &format!("{at}.{k}"))))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.iter().zip(y).enumerate().find_map(|(i, (v, w))| diff_path(v, w, &format!("{at}.{i}")))
        }
        _ => (a != b).then(|| at.trim_start_matches('.').to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Chain;

    fn chain() -> Chain {
        Chain { trim_db: 0.0, blocks: classic_blocks(), fader_db: 0.0, pan: 0.0 }
    }

    /// req: FX-10
    #[test]
    fn ids_name_their_kind_and_keys_round_trip() {
        for kind in FxKind::ALL {
            assert_eq!(FxKind::from_id(kind.key()), Some(kind));
            assert_eq!(FxKind::from_id(&format!("{}2", kind.key())), Some(kind));
            let fx = Fx::new(kind);
            for k in BlockKey::all(&fx) {
                let s = k.to_string();
                assert_eq!(BlockKey::parse(kind, &s).as_ref(), Some(&k), "{} {s}", kind.key());
                let v = k.get(&fx).unwrap();
                let sp = k.spec(kind, s.clone());
                assert!(v >= sp.min && v <= sp.max, "{} {s} = {v}", kind.key());
            }
        }
        assert!(BlockKey::parse(FxKind::Eq, "5.gain").is_none());
        assert!(BlockKey::parse(FxKind::Gate, "bogus").is_none());
        assert_eq!(FxKind::from_id("deq"), Some(FxKind::DynamicEq));
        assert_eq!(FxKind::from_id("bogus"), None);
    }

    /// req: FX-10, FX-11
    #[test]
    fn add_move_duplicate_reset_and_limits() {
        let c = chain();
        assert!(c.check(false).is_ok());
        let (c, id) = c.edit(&FxOp::Add { kind: FxKind::Gate, at: Some(0) }, false).unwrap();
        assert_eq!(id.as_deref(), Some("gate"));
        assert_eq!(c.blocks[0].id, "gate");
        let (c, id) = c.edit(&FxOp::Duplicate { id: "eq".into() }, false).unwrap();
        assert_eq!(id.as_deref(), Some("eq2"));
        assert_eq!(c.blocks.iter().position(|b| b.id == "eq2"), Some(4));
        assert_eq!(c.name("eq2").as_deref(), Some("EQ 2"));
        assert!(c.edit(&FxOp::Duplicate { id: "eq".into() }, false).is_err(), "two of a kind at most");
        let (c, _) = c.edit(&FxOp::Move { id: "gate".into(), to: 99 }, false).unwrap();
        assert_eq!(c.blocks.last().unwrap().id, "gate");
        let mut c2 = c.clone();
        if let Fx::Gate(g) = &mut c2.block_mut("gate").unwrap().fx {
            g.range_db = 10.0;
            g.bypass = true;
        }
        let (c3, _) = c2.edit(&FxOp::Reset { id: "gate".into() }, false).unwrap();
        assert!(c3.block("gate").unwrap().fx.bypass(), "reset keeps the bypass");
        assert_eq!(c3.block("gate").unwrap().fx, {
            let mut f = Fx::new(FxKind::Gate);
            f.set_bypass(true);
            f
        });
        // Twelve on a track, six on a return.
        let mut full = chain();
        for k in [FxKind::Gate, FxKind::Deesser, FxKind::Transient, FxKind::Width, FxKind::Doubler, FxKind::Exciter] {
            full = full.edit(&FxOp::Add { kind: k, at: None }, false).unwrap().0;
        }
        let full = full.edit(&FxOp::Add { kind: FxKind::Clipper, at: None }, false).unwrap().0;
        assert_eq!(full.blocks.len(), 12);
        assert!(full.edit(&FxOp::Add { kind: FxKind::Tremolo, at: None }, false).is_err());
        let ret = Chain { trim_db: 0.0, blocks: Vec::new(), fader_db: 0.0, pan: 0.0 };
        let mut r = ret;
        for _ in 0..3 {
            r = r.edit(&FxOp::Add { kind: FxKind::Compressor, at: None }, true).map(|x| x.0).unwrap_or(r);
        }
        assert_eq!(r.count(FxKind::Compressor), 2);
        // Removing a classic block is allowed, and `ensure` puts it back at its place.
        let (mut c, _) = chain().edit(&FxOp::Remove { id: "eq".into() }, false).unwrap();
        c.eq_mut();
        assert_eq!(c.blocks.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["hpf", "lpf", "eq", "comp", "sat"]);
    }

    /// req: UI-12
    #[test]
    fn the_catalog_locates_every_parameter() {
        let cat = catalog();
        assert_eq!(cat.len(), FxKind::ALL.len());
        let eq = cat.iter().find(|c| c.kind == FxKind::Eq).unwrap();
        assert!(eq.params.iter().any(|p| p.key == "<n>.gain" && p.path == "bands.<n>.gain_db"));
        let mb = cat.iter().find(|c| c.kind == FxKind::Multiband).unwrap();
        assert!(mb.params.iter().any(|p| p.key == "xover.<n>" && p.path == "crossovers_hz.<n>"));
        let comp = cat.iter().find(|c| c.kind == FxKind::Compressor).unwrap();
        assert!(comp.params.iter().any(|p| p.key == "sc_hpf" && p.path == "sidechain.hpf_hz"));
        let gate = cat.iter().find(|c| c.kind == FxKind::Gate).unwrap();
        assert!(gate.params.iter().any(|p| p.key == "range" && p.path == "range_db"));
    }

    /// req: FX-12, FX-17
    #[test]
    fn bands_and_voices() {
        let (c, _) = chain().edit(&FxOp::Add { kind: FxKind::Multiband, at: None }, false).unwrap();
        let (c, _) = c.edit(&FxOp::Bands { id: "mb".into(), bands: 5 }, false).unwrap();
        let Fx::Multiband(m) = &c.block("mb").unwrap().fx else { panic!() };
        assert_eq!((m.bands.len(), m.crossovers_hz.len()), (5, 4));
        assert!(BlockKey::parse(FxKind::Multiband, "xover.4").is_some());
        let (c, _) = c.edit(&FxOp::Add { kind: FxKind::Harmonizer, at: None }, false).unwrap();
        let (c, _) = c.edit(&FxOp::Bands { id: "harm".into(), bands: 1 }, false).unwrap();
        let Fx::Harmonizer(h) = &c.block("harm").unwrap().fx else { panic!() };
        assert_eq!(h.voices.len(), 1);
        assert!(c.edit(&FxOp::Bands { id: "eq".into(), bands: 2 }, false).is_err());
        let Fx::Hpf(f) = &chain().blocks[0].fx else { panic!() };
        assert_eq!(f.resonance, std::f64::consts::FRAC_1_SQRT_2, "the flat filter of schema 3");
    }
}
