//! Commands (HIS-1): every mutation of the project is data with an inverse.
//!
//! Commands are primitive and total: `apply` performs the change and returns the command that undoes it.
//! Higher-level edits (clip moves with trimming, deleting a return and its sends, applying an AI plan…) are
//! composed from these primitives into one transaction (`crate::history`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{
    AiNote, Brief, MasterState, MediaFile, MixBus, Project, ReferenceTrack, Section, SnapshotInfo, Tempo, Track,
    TrackId,
};
use crate::params::{self, ParamError, ParamId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    /// One continuous or switch parameter of a track (`params::ParamId` string form).
    SetParam {
        track: TrackId,
        param: String,
        value: f64,
    },
    /// Replaces a track (matched by id) with a new version: clips, chain, sends, automation, name…
    SetTrack {
        track: Box<Track>,
    },
    InsertTrack {
        index: usize,
        track: Box<Track>,
    },
    RemoveTrack {
        track: TrackId,
    },
    MoveTrack {
        track: TrackId,
        index: usize,
    },
    SetSections {
        sections: Vec<Section>,
    },
    SetBrief {
        brief: Brief,
    },
    SetTempo {
        tempo: Option<Tempo>,
    },
    SetName {
        name: String,
    },
    SetMixBus {
        bus: MixBus,
    },
    SetSampleRate {
        sample_rate: Option<u32>,
    },
    AddMedia {
        media: Box<MediaFile>,
    },
    RemoveMedia {
        media: String,
    },
    SetReferences {
        references: Vec<ReferenceTrack>,
    },
    SetSnapshots {
        snapshots: Vec<SnapshotInfo>,
    },
    SetAiNotes {
        notes: Vec<AiNote>,
        summary: Option<String>,
    },
    /// The AI call log (AI-10): written with `Document::apply_view`, never a history step.
    SetAiCalls {
        calls: Vec<crate::model::AiCallRecord>,
    },
    /// Solo of a track (view state, HIS-6: applied with `Document::apply_view`).
    SetSolo {
        track: TrackId,
        on: bool,
    },
    SetNormalizeTrim {
        on: bool,
    },
    /// AI-27: whether the assistant may propose creative ideas (view state, kept in the project).
    SetAllowIdeas {
        on: bool,
    },
    /// AI-18: whether the assistant may propose new returns and groups.
    SetAllowStructure {
        on: bool,
    },
    SetSpendCap {
        usd: Option<f64>,
    },
    /// The whole master state (master phase, HIS-5). The mixdown is kept as it is: undo and redo of master steps
    /// never move it (MDN-2); only [`Command::SetMixdown`] changes it.
    SetMaster {
        master: Box<MasterState>,
    },
    /// The clips using media `from` use media `to` instead (UI-6: a missing file replaced by another of the same
    /// shape, whose media is added first with `AddMedia`). Both entries stay in the project, so a snapshot that still
    /// names `from` finds it, and reports it missing.
    RelinkMedia {
        from: String,
        to: String,
    },
    /// A new mixdown (a bounce, MDN-1): recorded outside the histories, the file exists either way (MDN-4).
    SetMixdown {
        mixdown: Option<Box<crate::model::Mixdown>>,
    },
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CommandError {
    #[error("track `{0}` does not exist")]
    NoTrack(String),
    #[error("a track with id `{0}` already exists")]
    DuplicateTrack(String),
    #[error("index {0} is out of range")]
    BadIndex(usize),
    #[error(transparent)]
    Param(#[from] ParamError),
    #[error("media `{0}` is still used by a clip")]
    MediaInUse(String),
    #[error("{0}")]
    Invalid(String),
}

impl Command {
    /// Applies the command and returns its inverse.
    pub fn apply(self, p: &mut Project) -> Result<Command, CommandError> {
        use Command::*;
        Ok(match self {
            SetParam { track, param, value } => {
                let id: ParamId = param.parse()?;
                let t = p.track_mut(&track).ok_or_else(|| CommandError::NoTrack(track.clone()))?;
                let old = params::set(t, &id, value)?;
                SetParam { track, param, value: old }
            }
            SetTrack { track } => {
                // The limits of a chain hold whoever writes the track (FX-10, FX-11): the engine sizes its key
                // inputs and its slots on them.
                track.chain.check(track.is_return()).map_err(CommandError::Invalid)?;
                let slot = p.track_mut(&track.id).ok_or_else(|| CommandError::NoTrack(track.id.clone()))?;
                let old = std::mem::replace(slot, *track);
                SetTrack { track: Box::new(old) }
            }
            InsertTrack { index, track } => {
                if index > p.tracks.len() {
                    return Err(CommandError::BadIndex(index));
                }
                if p.track(&track.id).is_some() {
                    return Err(CommandError::DuplicateTrack(track.id.clone()));
                }
                track.chain.check(track.is_return()).map_err(CommandError::Invalid)?;
                let id = track.id.clone();
                p.tracks.insert(index, *track);
                RemoveTrack { track: id }
            }
            RemoveTrack { track } => {
                let index = p.track_index(&track).ok_or_else(|| CommandError::NoTrack(track.clone()))?;
                let t = p.tracks.remove(index);
                InsertTrack { index, track: Box::new(t) }
            }
            MoveTrack { track, index } => {
                let from = p.track_index(&track).ok_or_else(|| CommandError::NoTrack(track.clone()))?;
                if index >= p.tracks.len() {
                    return Err(CommandError::BadIndex(index));
                }
                let t = p.tracks.remove(from);
                p.tracks.insert(index, t);
                MoveTrack { track, index: from }
            }
            SetSections { sections } => {
                check_sections(&sections)?;
                SetSections { sections: std::mem::replace(&mut p.sections, sections) }
            }
            SetBrief { brief } => SetBrief { brief: std::mem::replace(&mut p.brief, brief) },
            SetTempo { tempo } => {
                if let Some(t) = &tempo
                    && !(t.bpm > 0.0 && t.bpm <= 999.0 && t.beats_per_bar > 0 && t.beat_unit > 0)
                {
                    return Err(CommandError::Invalid("invalid tempo".into()));
                }
                SetTempo { tempo: std::mem::replace(&mut p.tempo, tempo) }
            }
            SetName { name } => SetName { name: std::mem::replace(&mut p.name, name) },
            SetMixBus { bus } => SetMixBus { bus: std::mem::replace(&mut p.mix_bus, bus) },
            SetSampleRate { sample_rate } => {
                SetSampleRate { sample_rate: std::mem::replace(&mut p.sample_rate, sample_rate) }
            }
            AddMedia { media } => {
                let id = media.id.clone();
                if p.media(&id).is_none() {
                    p.media.push(*media);
                    RemoveMedia { media: id }
                } else {
                    // Same content imported twice: nothing changes.
                    SetName { name: p.name.clone() }
                }
            }
            RemoveMedia { media } => {
                if p.tracks.iter().flat_map(|t| &t.clips).any(|c| c.media == media) {
                    return Err(CommandError::MediaInUse(media));
                }
                let i = p.media.iter().position(|m| m.id == media).ok_or(CommandError::Invalid(media.clone()))?;
                AddMedia { media: Box::new(p.media.remove(i)) }
            }
            SetReferences { references } => {
                if references.len() > 3 {
                    return Err(CommandError::Invalid("at most 3 reference tracks".into()));
                }
                SetReferences { references: std::mem::replace(&mut p.references, references) }
            }
            SetSnapshots { snapshots } => SetSnapshots { snapshots: std::mem::replace(&mut p.snapshots, snapshots) },
            SetAiNotes { notes, summary } => SetAiNotes {
                notes: std::mem::replace(&mut p.ai.notes, notes),
                summary: std::mem::replace(&mut p.ai.summary, summary),
            },
            SetAiCalls { calls } => SetAiCalls { calls: std::mem::replace(&mut p.ai.calls, calls) },
            SetSolo { track, on } => {
                let t = p.track_mut(&track).ok_or_else(|| CommandError::NoTrack(track.clone()))?;
                let old = std::mem::replace(&mut t.solo, on);
                SetSolo { track, on: old }
            }
            SetNormalizeTrim { on } => SetNormalizeTrim { on: std::mem::replace(&mut p.ai.normalize_trim, on) },
            SetAllowStructure { on } => SetAllowStructure { on: std::mem::replace(&mut p.ai.allow_structure, on) },
            SetAllowIdeas { on } => SetAllowIdeas { on: std::mem::replace(&mut p.ai.allow_ideas, on) },
            SetSpendCap { usd } => SetSpendCap { usd: std::mem::replace(&mut p.ai.spend_cap_usd, usd) },
            SetMaster { master } => {
                let mut m = *master;
                m.mixdown = p.master.mixdown.clone();
                SetMaster { master: Box::new(std::mem::replace(&mut p.master, m)) }
            }
            RelinkMedia { from, to } => {
                if p.media(&to).is_none() {
                    return Err(CommandError::Invalid(format!("no media `{to}`")));
                }
                for c in p.tracks.iter_mut().flat_map(|t| t.clips.iter_mut()).filter(|c| c.media == from) {
                    c.media = to.clone();
                }
                RelinkMedia { from: to, to: from }
            }
            SetMixdown { mixdown } => {
                SetMixdown { mixdown: std::mem::replace(&mut p.master.mixdown, mixdown.map(|m| *m)).map(Box::new) }
            }
        })
    }

    /// Only parameter values change: the engine can take them on its lock-free queue without rebuilding the
    /// graph (FOUNDATION §5.3).
    pub fn is_param_only(&self) -> bool {
        matches!(self, Command::SetParam { .. })
    }

    /// Commands of the master phase (HIS-5).
    pub fn is_master(&self) -> bool {
        matches!(self, Command::SetMaster { .. })
    }
}

fn check_sections(sections: &[Section]) -> Result<(), CommandError> {
    let mut sorted: Vec<&Section> = sections.iter().collect();
    sorted.sort_by_key(|s| s.start);
    for s in &sorted {
        if s.end <= s.start || s.start < 0 {
            return Err(CommandError::Invalid(format!("section `{}` has no length", s.name)));
        }
    }
    for w in sorted.windows(2) {
        if w[1].start < w[0].end {
            return Err(CommandError::Invalid(format!("sections `{}` and `{}` overlap", w[0].name, w[1].name)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Track;

    /// A chain beyond its limits does not enter the project, whoever writes it: not through a command, not from a
    /// file edited by hand (the engine sizes its key inputs and slots on the limits).
    /// req: FX-10, FX-11, PRJ-8
    #[test]
    fn a_chain_beyond_its_limits_is_refused_by_commands_and_at_load() {
        use crate::fx::{Fx, FxBlock, FxKind};
        let mut p = Project::new("limits");
        p.tracks.push(Track::new_audio("t1".into(), "Kick", None, 1, 2));
        let good = crate::package::parse(&serde_json::to_string(&p).unwrap());
        assert!(good.is_ok(), "{:?}", good.err());
        // Three gates (two per type at most).
        let mut three = p.track("t1").unwrap().clone();
        for id in ["gate", "gate2", "gate3"] {
            three.chain.blocks.push(FxBlock { id: id.into(), fx: Fx::new(FxKind::Gate) });
        }
        // A multiband whose crossovers do not match its bands.
        let mut broken = p.track("t1").unwrap().clone();
        let mut mb = Fx::new(FxKind::Multiband);
        if let Fx::Multiband(m) = &mut mb {
            m.crossovers_hz.pop();
        }
        broken.chain.blocks.push(FxBlock { id: "mb".into(), fx: mb });
        for bad in [three, broken] {
            let before = p.clone();
            assert!(Command::SetTrack { track: Box::new(bad.clone()) }.apply(&mut p).is_err());
            let mut new = bad.clone();
            new.id = "t9".into();
            assert!(Command::InsertTrack { index: 0, track: Box::new(new) }.apply(&mut p).is_err());
            assert_eq!(p, before, "a refused command changes nothing");
            let mut by_hand = p.clone();
            *by_hand.track_mut("t1").unwrap() = bad;
            let r = crate::package::parse(&serde_json::to_string(&by_hand).unwrap());
            assert!(matches!(r, Err(crate::package::PackageError::Chain(..))), "{:?}", r.err());
        }
    }

    /// req: HIS-1
    #[test]
    fn every_command_kind_round_trips_through_its_inverse() {
        let mut p = Project::new("cmd");
        p.tracks.push(Track::new_audio("t1".into(), "Kick", None, 1, 2));
        let original = p.clone();
        let mut renamed = p.tracks[2].clone();
        renamed.name = "Kick In".into();
        let cmds = vec![
            Command::SetParam { track: "t1".into(), param: "fader".into(), value: -3.0 },
            Command::SetTrack { track: Box::new(renamed) },
            Command::InsertTrack { index: 0, track: Box::new(Track::new_group("g9".into(), "Drums")) },
            Command::MoveTrack { track: "t1".into(), index: 0 },
            Command::RemoveTrack { track: "g9".into() },
            Command::SetSections {
                sections: vec![Section { id: "s1".into(), name: "Intro".into(), color: 1, start: 0, end: 48000 }],
            },
            Command::SetBrief { brief: Brief { genre: "indie".into(), mood: "warm".into(), notes: String::new() } },
            Command::SetName { name: "New".into() },
            Command::SetMixBus { bus: MixBus { fader_db: -1.0 } },
            Command::SetSampleRate { sample_rate: Some(48000) },
            Command::SetNormalizeTrim { on: false },
            Command::SetAllowStructure { on: false },
            Command::SetAllowIdeas { on: false },
            Command::SetSpendCap { usd: Some(1.0) },
            Command::SetMaster { master: Box::new(MasterState::default()) },
        ];
        let mut inverses = Vec::new();
        for c in cmds {
            inverses.push(c.apply(&mut p).unwrap());
        }
        assert_ne!(p, original);
        for inv in inverses.into_iter().rev() {
            inv.apply(&mut p).unwrap();
        }
        assert_eq!(p, original);
    }

    /// req: SEC-1
    #[test]
    fn sections_cannot_overlap() {
        let mut p = Project::new("s");
        let s = |id: &str, a, b| Section { id: id.into(), name: id.into(), color: 1, start: a, end: b };
        let bad = Command::SetSections { sections: vec![s("a", 0, 100), s("b", 50, 200)] };
        assert!(bad.apply(&mut p).is_err());
        let empty = Command::SetSections { sections: vec![s("a", 10, 10)] };
        assert!(empty.apply(&mut p).is_err());
        let ok = Command::SetSections { sections: vec![s("a", 0, 100), s("b", 100, 200)] };
        assert!(ok.apply(&mut p).is_ok());
    }
}
