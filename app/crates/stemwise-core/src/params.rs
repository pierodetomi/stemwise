//! Parameter ids, ranges and access (FX-1 … FX-6, AUT-1).
//!
//! Every continuous parameter of a track, group or return has a stable string id used by automation lanes,
//! by `SetParam` commands and by the UI (`eq.2.gain`, `send.r1`, `bypass.comp` …). Bypass and mute are
//! parameters too, with values 0 or 1, automated in steps. Block parameters are `<block id>.<key>` (FX-10, see
//! [`crate::fx`]).

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::fx::{BlockKey, FxKind};
use crate::model::{ReturnFx, SILENCE_DB, Track, TrackKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParamId {
    Trim,
    Fader,
    Pan,
    Send(String),
    /// A parameter of a block of the chain (FX-10): its id and key (`eq.2.gain`, `gate.range`).
    Block(String, BlockKey),
    RevDecay,
    RevSize,
    RevDamping,
    RevModulation,
    RevPredelay,
    RevHpf,
    RevLpf,
    RevShimmer,
    DlyTime,
    DlyFeedback,
    DlyFbHpf,
    DlyFbLpf,
    DlyWow,
    DlyDrive,
    Mute,
    /// Bypass of a block (`bypass.<id>`); `bypass.fx` is the processor of a return.
    Bypass(String),
}

/// The id a return's processor answers to in `bypass.fx`.
pub const FX: &str = "fx";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Db,
    Hz,
    Ms,
    /// Seconds.
    S,
    Ratio,
    Percent,
    Pan,
    Toggle,
    None,
}

/// Range and presentation of a parameter (the UI never hard-codes them, FOUNDATION §5.6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ParamSpec {
    pub id: String,
    pub label: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub unit: Unit,
    /// Logarithmic control mapping (frequencies, times).
    pub log: bool,
    /// Automated in steps (mute, bypass) rather than with linear interpolation (AUT-1).
    pub stepped: bool,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParamError {
    #[error("unknown parameter `{0}`")]
    Unknown(String),
    #[error("parameter `{0}` does not exist on this track")]
    NotOnTrack(String),
    #[error("{param} = {value} is outside {min} … {max}")]
    OutOfRange { param: String, value: f64, min: f64, max: f64 },
}

impl fmt::Display for ParamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamId::Trim => write!(f, "trim"),
            ParamId::Fader => write!(f, "fader"),
            ParamId::Pan => write!(f, "pan"),
            ParamId::Send(t) => write!(f, "send.{t}"),
            ParamId::Block(b, k) => write!(f, "{b}.{k}"),
            ParamId::RevDecay => write!(f, "rev.decay"),
            ParamId::RevModulation => write!(f, "rev.modulation"),
            ParamId::RevLpf => write!(f, "rev.lpf"),
            ParamId::RevSize => write!(f, "rev.size"),
            ParamId::RevDamping => write!(f, "rev.damping"),
            ParamId::RevPredelay => write!(f, "rev.predelay"),
            ParamId::RevHpf => write!(f, "rev.hpf"),
            ParamId::RevShimmer => write!(f, "rev.shimmer"),
            ParamId::DlyTime => write!(f, "dly.time"),
            ParamId::DlyFeedback => write!(f, "dly.feedback"),
            ParamId::DlyFbHpf => write!(f, "dly.fb_hpf"),
            ParamId::DlyFbLpf => write!(f, "dly.fb_lpf"),
            ParamId::DlyWow => write!(f, "dly.wow"),
            ParamId::DlyDrive => write!(f, "dly.drive"),
            ParamId::Mute => write!(f, "mute"),
            ParamId::Bypass(b) => write!(f, "bypass.{b}"),
        }
    }
}

impl std::str::FromStr for ParamId {
    type Err = ParamError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let unknown = || ParamError::Unknown(s.to_string());
        if let Some(rest) = s.strip_prefix("send.") {
            return if rest.is_empty() { Err(unknown()) } else { Ok(ParamId::Send(rest.to_string())) };
        }
        if let Some(rest) = s.strip_prefix("bypass.") {
            return if rest == FX || FxKind::from_id(rest).is_some() {
                Ok(ParamId::Bypass(rest.to_string()))
            } else {
                Err(unknown())
            };
        }
        Ok(match s {
            "trim" => ParamId::Trim,
            "fader" => ParamId::Fader,
            "pan" => ParamId::Pan,
            "rev.decay" => ParamId::RevDecay,
            "rev.modulation" => ParamId::RevModulation,
            "rev.lpf" => ParamId::RevLpf,
            "rev.size" => ParamId::RevSize,
            "rev.damping" => ParamId::RevDamping,
            "rev.predelay" => ParamId::RevPredelay,
            "rev.hpf" => ParamId::RevHpf,
            "rev.shimmer" => ParamId::RevShimmer,
            "dly.time" => ParamId::DlyTime,
            "dly.feedback" => ParamId::DlyFeedback,
            "dly.fb_hpf" => ParamId::DlyFbHpf,
            "dly.fb_lpf" => ParamId::DlyFbLpf,
            "dly.wow" => ParamId::DlyWow,
            "dly.drive" => ParamId::DlyDrive,
            "mute" => ParamId::Mute,
            _ => {
                let (block, key) = s.split_once('.').ok_or_else(unknown)?;
                let kind = FxKind::from_id(block).ok_or_else(unknown)?;
                let k = BlockKey::parse(kind, key).ok_or_else(unknown)?;
                ParamId::Block(block.to_string(), k)
            }
        })
    }
}

fn spec(id: &ParamId, label: &str, min: f64, max: f64, default: f64, unit: Unit, log: bool) -> ParamSpec {
    ParamSpec { id: id.to_string(), label: label.to_string(), min, max, default, unit, log, stepped: false }
}

impl ParamId {
    /// A block parameter by its id string parts (`ParamId::block("eq", "2.gain")`).
    pub fn block(id: &str, key: &str) -> Option<ParamId> {
        format!("{id}.{key}").parse().ok()
    }

    /// Range, default and presentation. Ranges are the user's; the AI plan has tighter limits (§5.5).
    pub fn spec(&self) -> ParamSpec {
        use ParamId::*;
        match self {
            Trim => spec(self, "Trim", -36.0, 36.0, 0.0, Unit::Db, false),
            Block(b, k) => k.spec(FxKind::from_id(b).expect("parsed block ids name a kind"), self.to_string()),
            Fader => spec(self, "Level", SILENCE_DB, 12.0, 0.0, Unit::Db, false),
            Pan => spec(self, "Pan", -1.0, 1.0, 0.0, Unit::Pan, false),
            Send(_) => spec(self, "Send", SILENCE_DB, 6.0, -12.0, Unit::Db, false),
            RevDecay => spec(self, "Decay", 0.1, 10.0, 2.0, Unit::S, true),
            RevSize => spec(self, "Size", 0.0, 1.0, 0.5, Unit::Percent, false),
            RevDamping => spec(self, "Damping", 0.0, 1.0, 0.5, Unit::Percent, false),
            RevModulation => spec(self, "Modulation", 0.0, 1.0, 0.2, Unit::Percent, false),
            RevLpf => spec(self, "High cut", 1000.0, 20000.0, 9000.0, Unit::Hz, true),
            RevPredelay => spec(self, "Pre-delay", 0.0, 200.0, 20.0, Unit::Ms, false),
            RevHpf => spec(self, "Low cut", 20.0, 1000.0, 250.0, Unit::Hz, true),
            RevShimmer => spec(self, "Shimmer", 0.0, 1.0, 0.0, Unit::Percent, false),
            DlyTime => spec(self, "Time", 1.0, 2000.0, 375.0, Unit::Ms, true),
            DlyFeedback => spec(self, "Feedback", 0.0, 0.95, 0.25, Unit::Percent, false),
            DlyFbHpf => spec(self, "Feedback low cut", 20.0, 2000.0, 200.0, Unit::Hz, true),
            DlyFbLpf => spec(self, "Feedback high cut", 500.0, 20000.0, 6000.0, Unit::Hz, true),
            DlyWow => spec(self, "Wow & flutter", 0.0, 1.0, 0.0, Unit::Percent, false),
            DlyDrive => spec(self, "Drive", 0.0, 12.0, 0.0, Unit::Db, false),
            Mute | Bypass(_) => ParamSpec {
                stepped: true,
                ..spec(self, if *self == Mute { "Mute" } else { "Bypass" }, 0.0, 1.0, 0.0, Unit::Toggle, false)
            },
        }
    }

    /// Parameters of the blocks of a chain, in chain order, then their bypasses.
    fn blocks_of(track: &Track) -> (Vec<ParamId>, Vec<ParamId>) {
        let mut params = Vec::new();
        let mut bypass = Vec::new();
        for b in &track.chain.blocks {
            params.extend(BlockKey::all(&b.fx).into_iter().map(|k| ParamId::Block(b.id.clone(), k)));
            bypass.push(ParamId::Bypass(b.id.clone()));
        }
        (params, bypass)
    }

    /// Every parameter a track exposes (the automation lane menu of design 7d).
    pub fn all_for(track: &Track) -> Vec<ParamId> {
        use ParamId::*;
        let mut out = Vec::new();
        let (blocks, bypass) = Self::blocks_of(track);
        match track.kind {
            TrackKind::Audio | TrackKind::Group => {
                out.extend([Fader, Pan, Trim]);
                out.extend(blocks);
                out.extend(track.sends.iter().map(|s| Send(s.target.clone())));
                out.push(Mute);
                out.extend(bypass);
            }
            TrackKind::Return => {
                out.extend([Fader, Pan]);
                match track.ret {
                    Some(ReturnFx::Reverb { .. }) => out.extend([
                        RevDecay,
                        RevSize,
                        RevDamping,
                        RevModulation,
                        RevPredelay,
                        RevHpf,
                        RevLpf,
                        RevShimmer,
                    ]),
                    Some(ReturnFx::Delay { .. }) => {
                        out.extend([DlyTime, DlyFeedback, DlyFbHpf, DlyFbLpf, DlyWow, DlyDrive])
                    }
                    None => {}
                }
                out.extend(blocks);
                out.extend([Mute, Bypass(FX.into())]);
                out.extend(bypass);
            }
        }
        out
    }

    pub fn is_stepped(&self) -> bool {
        matches!(self, ParamId::Mute | ParamId::Bypass(_))
    }

    /// Whether the parameter belongs to the end of the chain (fader, pan, mute, sends), evaluated after its latency.
    pub fn is_post(&self) -> bool {
        matches!(self, ParamId::Fader | ParamId::Pan | ParamId::Mute | ParamId::Send(_))
    }
}

/// Specs of every parameter id (one per kind; sends as `send.<return>`, blocks with their kind's key as id), for
/// the interface and the mock backend.
pub fn all_specs() -> Vec<ParamSpec> {
    use crate::model::{ReturnFx, Send, Track};
    let mut t = Track::new_audio("x".into(), "x", None, 1, 1);
    t.sends.push(Send { target: "<return>".into(), level_db: 0.0 });
    let mut out: Vec<ParamSpec> = ParamId::all_for(&t).iter().map(|p| p.spec()).collect();
    out.extend(crate::fx::all_block_specs());
    for k in FxKind::ALL {
        out.push(ParamId::Bypass(k.key().into()).spec());
    }
    for r in [ReturnFx::default_reverb(), ReturnFx::default_delay()] {
        out.extend(ParamId::all_for(&Track::new_return("r".into(), "r", r)).iter().map(|p| p.spec()));
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.id == b.id);
    out
}

fn b(v: bool) -> f64 {
    if v { 1.0 } else { 0.0 }
}

/// Current static value of a parameter on a track (`None` if the track has no such parameter).
pub fn get(track: &Track, id: &ParamId) -> Option<f64> {
    use ParamId::*;
    let c = &track.chain;
    let chain_ok = track.kind != TrackKind::Return;
    Some(match id {
        Fader => c.fader_db,
        Pan => c.pan,
        Mute => b(track.mute),
        Send(t) => track.send_db(t)?,
        Trim if chain_ok => c.trim_db,
        Block(blk, k) => k.get(&c.block(blk)?.fx)?,
        Bypass(blk) if blk != FX => b(c.block(blk)?.fx.bypass()),
        _ => match (&track.ret, id) {
            (Some(ReturnFx::Reverb { decay_s, .. }), RevDecay) => *decay_s,
            (Some(ReturnFx::Reverb { size, .. }), RevSize) => *size,
            (Some(ReturnFx::Reverb { modulation, .. }), RevModulation) => *modulation,
            (Some(ReturnFx::Reverb { lpf_hz, .. }), RevLpf) => *lpf_hz,
            (Some(ReturnFx::Reverb { damping, .. }), RevDamping) => *damping,
            (Some(ReturnFx::Reverb { predelay_ms, .. }), RevPredelay) => *predelay_ms,
            (Some(ReturnFx::Reverb { hpf_hz, .. }), RevHpf) => *hpf_hz,
            (Some(ReturnFx::Reverb { shimmer_amount, .. }), RevShimmer) => *shimmer_amount,
            (Some(ReturnFx::Reverb { bypass, .. }), Bypass(_)) => b(*bypass),
            (Some(ReturnFx::Delay { time_ms, .. }), DlyTime) => *time_ms,
            (Some(ReturnFx::Delay { feedback, .. }), DlyFeedback) => *feedback,
            (Some(ReturnFx::Delay { feedback_hpf_hz, .. }), DlyFbHpf) => *feedback_hpf_hz,
            (Some(ReturnFx::Delay { feedback_lpf_hz, .. }), DlyFbLpf) => *feedback_lpf_hz,
            (Some(ReturnFx::Delay { wow, .. }), DlyWow) => *wow,
            (Some(ReturnFx::Delay { drive_db, .. }), DlyDrive) => *drive_db,
            (Some(ReturnFx::Delay { bypass, .. }), Bypass(_)) => b(*bypass),
            _ => return None,
        },
    })
}

/// Sets a parameter, checking its range; returns the previous value.
pub fn set(track: &mut Track, id: &ParamId, value: f64) -> Result<f64, ParamError> {
    let sp = id.spec();
    if !value.is_finite() || value < sp.min || value > sp.max {
        return Err(ParamError::OutOfRange { param: sp.id, value, min: sp.min, max: sp.max });
    }
    let old = get(track, id).ok_or_else(|| ParamError::NotOnTrack(id.to_string()))?;
    let on = value >= 0.5;
    use ParamId::*;
    let c = &mut track.chain;
    match id {
        Fader => c.fader_db = value,
        Pan => c.pan = value,
        Mute => track.mute = on,
        Send(t) => {
            if let Some(s) = track.sends.iter_mut().find(|s| &s.target == t) {
                s.level_db = value;
            }
        }
        Trim => c.trim_db = value,
        // Enabling a band is a separate edit: a parameter command changes exactly one value, so its inverse
        // restores exactly that value.
        Block(blk, k) => {
            let b = c.block_mut(blk).ok_or_else(|| ParamError::NotOnTrack(id.to_string()))?;
            k.set(&mut b.fx, value);
        }
        Bypass(blk) if blk != FX => {
            c.block_mut(blk).ok_or_else(|| ParamError::NotOnTrack(id.to_string()))?.fx.set_bypass(on);
        }
        _ => match (&mut track.ret, id) {
            (Some(ReturnFx::Reverb { decay_s, .. }), RevDecay) => *decay_s = value,
            (Some(ReturnFx::Reverb { size, .. }), RevSize) => *size = value,
            (Some(ReturnFx::Reverb { modulation, .. }), RevModulation) => *modulation = value,
            (Some(ReturnFx::Reverb { lpf_hz, .. }), RevLpf) => *lpf_hz = value,
            (Some(ReturnFx::Reverb { damping, .. }), RevDamping) => *damping = value,
            (Some(ReturnFx::Reverb { predelay_ms, .. }), RevPredelay) => *predelay_ms = value,
            (Some(ReturnFx::Reverb { hpf_hz, .. }), RevHpf) => *hpf_hz = value,
            (Some(ReturnFx::Reverb { shimmer_amount, .. }), RevShimmer) => *shimmer_amount = value,
            (Some(ReturnFx::Reverb { bypass, .. }), Bypass(_)) => *bypass = on,
            (Some(ReturnFx::Delay { time_ms, sync, .. }), DlyTime) => {
                *time_ms = value;
                *sync = None;
            }
            (Some(ReturnFx::Delay { feedback, .. }), DlyFeedback) => *feedback = value,
            (Some(ReturnFx::Delay { feedback_hpf_hz, .. }), DlyFbHpf) => *feedback_hpf_hz = value,
            (Some(ReturnFx::Delay { feedback_lpf_hz, .. }), DlyFbLpf) => *feedback_lpf_hz = value,
            (Some(ReturnFx::Delay { wow, .. }), DlyWow) => *wow = value,
            (Some(ReturnFx::Delay { drive_db, .. }), DlyDrive) => *drive_db = value,
            (Some(ReturnFx::Delay { bypass, .. }), Bypass(_)) => *bypass = on,
            _ => return Err(ParamError::NotOnTrack(id.to_string())),
        },
    }
    Ok(old)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Project, Send, Track};

    fn track_with_send() -> Track {
        let mut t = Track::new_audio("t1".into(), "Vox", None, 1, 7);
        t.sends.push(Send { target: "r1".into(), level_db: -12.0 });
        t
    }

    /// req: AUT-1
    #[test]
    fn ids_round_trip_and_every_listed_param_is_readable() {
        let p = Project::new("x");
        let mut tracks = vec![track_with_send()];
        tracks.extend(p.tracks.iter().cloned());
        let mut g = Track::new_group("g1".into(), "Drums");
        g.sends.push(Send { target: "r2".into(), level_db: -20.0 });
        tracks.push(g);
        for t in &tracks {
            let all = ParamId::all_for(t);
            assert!(all.len() >= 6);
            for id in all {
                let s = id.to_string();
                assert_eq!(s.parse::<ParamId>().unwrap(), id, "{s}");
                let v = get(t, &id).unwrap_or_else(|| panic!("{s} on {}", t.name));
                let sp = id.spec();
                assert!(v >= sp.min && v <= sp.max, "{s} = {v}");
                assert_eq!(sp.stepped, id.is_stepped());
            }
        }
    }

    /// req: AUT-1, FX-1
    #[test]
    fn set_checks_range_and_returns_previous_value() {
        let mut t = track_with_send();
        let eq3: ParamId = "eq.3.gain".parse().unwrap();
        assert_eq!(set(&mut t, &eq3, 4.5), Ok(0.0));
        assert!(!t.chain.eq().unwrap().bands[2].enabled, "a parameter command changes one value only");
        assert_eq!("bypass.comp".parse::<ParamId>(), Ok(ParamId::Bypass("comp".into())));
        assert!("gate.range".parse::<ParamId>().is_ok() && get(&t, &"gate.range".parse().unwrap()).is_none());
        assert_eq!(set(&mut t, &ParamId::Send("r1".into()), -6.0), Ok(-12.0));
        assert!(matches!(set(&mut t, &ParamId::Fader, 13.0), Err(ParamError::OutOfRange { .. })));
        assert!(matches!(set(&mut t, &ParamId::RevSize, 0.5), Err(ParamError::NotOnTrack(_))));
        assert!(matches!(set(&mut t, &ParamId::Send("r9".into()), -6.0), Err(ParamError::NotOnTrack(_))));
        assert!("eq.5.gain".parse::<ParamId>().is_err());
        assert!("bogus".parse::<ParamId>().is_err());
    }
}
