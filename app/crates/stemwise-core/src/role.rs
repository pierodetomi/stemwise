//! Track roles (PRJ-6): the PoC list (PRD §6.1) plus `room`, `strings` and `brass`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Kick,
    Snare,
    Toms,
    Overheads,
    Room,
    Drums,
    Percussion,
    Bass,
    ElectricGuitar,
    AcousticGuitar,
    Keys,
    Synth,
    Pad,
    Strings,
    Brass,
    /// Acoustic and classical music (A-080): piano on its own, woodwinds, a choir.
    Piano,
    Woodwinds,
    Choir,
    LeadVocal,
    BackingVocal,
    Fx,
    Other,
}

impl Role {
    pub const ALL: [Role; 22] = [
        Role::Kick,
        Role::Snare,
        Role::Toms,
        Role::Overheads,
        Role::Room,
        Role::Drums,
        Role::Percussion,
        Role::Bass,
        Role::ElectricGuitar,
        Role::AcousticGuitar,
        Role::Keys,
        Role::Synth,
        Role::Pad,
        Role::Strings,
        Role::Brass,
        Role::Piano,
        Role::Woodwinds,
        Role::Choir,
        Role::LeadVocal,
        Role::BackingVocal,
        Role::Fx,
        Role::Other,
    ];

    /// The snake_case name used in JSON and sent to the model.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Kick => "kick",
            Role::Snare => "snare",
            Role::Toms => "toms",
            Role::Overheads => "overheads",
            Role::Room => "room",
            Role::Drums => "drums",
            Role::Percussion => "percussion",
            Role::Bass => "bass",
            Role::ElectricGuitar => "electric_guitar",
            Role::AcousticGuitar => "acoustic_guitar",
            Role::Keys => "keys",
            Role::Synth => "synth",
            Role::Pad => "pad",
            Role::Strings => "strings",
            Role::Brass => "brass",
            Role::Piano => "piano",
            Role::Woodwinds => "woodwinds",
            Role::Choir => "choir",
            Role::LeadVocal => "lead_vocal",
            Role::BackingVocal => "backing_vocal",
            Role::Fx => "fx",
            Role::Other => "other",
        }
    }

    /// Drum roles form the drum family of ANA-16.
    pub fn is_drums(self) -> bool {
        matches!(self, Role::Kick | Role::Snare | Role::Toms | Role::Overheads | Role::Room | Role::Drums)
    }

    /// Default track tint (1..=12) for the role.
    pub fn color(self) -> u8 {
        match self {
            Role::Kick | Role::Snare | Role::Toms | Role::Overheads | Role::Room | Role::Drums => 2,
            Role::Percussion => 3,
            Role::Bass => 1,
            Role::ElectricGuitar => 4,
            Role::AcousticGuitar => 5,
            Role::Keys => 8,
            Role::Synth => 9,
            Role::Pad => 10,
            Role::Strings => 6,
            Role::Brass | Role::Woodwinds => 12,
            Role::Piano => 8,
            Role::Choir => 11,
            Role::LeadVocal => 7,
            Role::BackingVocal => 11,
            Role::Fx | Role::Other => 3,
        }
    }

    /// Proposes a role from a file name with simple keyword rules (PRJ-6). Always editable by the user.
    pub fn guess(file_name: &str) -> Role {
        let stem = file_name.rsplit_once('.').map_or(file_name, |(s, _)| s).to_lowercase();
        let words: Vec<&str> = stem.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
        let has =
            |keys: &[&str]| words.iter().any(|w| keys.iter().any(|k| w == k || (k.len() > 3 && w.starts_with(k))));
        // Order matters: the most specific rules first.
        let rules: &[(&[&str], Role)] = &[
            (&["choir", "choirs", "coro", "satb", "soprano", "sopranos"], Role::Choir),
            (
                &["bv", "bvs", "bgv", "backing", "backings", "harmony", "harmonies", "bvox", "doubles"],
                Role::BackingVocal,
            ),
            (&["vox", "vocal", "vocals", "voice", "lead", "leadvox", "vo", "singer", "vocalist"], Role::LeadVocal),
            (&["kick", "kik", "bd", "bassdrum"], Role::Kick),
            (&["snare", "snr", "sd"], Role::Snare),
            (&["tom", "toms", "floortom", "rack"], Role::Toms),
            (
                &["oh", "ohs", "overhead", "overheads", "cymbals", "hat", "hihat", "hh", "ride", "crash"],
                Role::Overheads,
            ),
            (&["room", "rooms", "amb", "ambience", "ambient"], Role::Room),
            (&["drum", "drums", "kit", "drumkit", "beat", "loop"], Role::Drums),
            (
                &["perc", "percussion", "shaker", "tamb", "tambourine", "conga", "bongo", "clap", "claps"],
                Role::Percussion,
            ),
            (&["bass", "sub", "bas", "db"], Role::Bass),
            (&["acoustic", "ac", "agtr", "acgtr", "nylon"], Role::AcousticGuitar),
            (&["gtr", "guitar", "guitars", "egtr", "elgtr", "riff", "lead_gtr"], Role::ElectricGuitar),
            (&["piano", "pno", "grand", "upright"], Role::Piano),
            (&["keys", "key", "rhodes", "organ", "wurli", "epiano", "ep"], Role::Keys),
            (&["pad", "pads"], Role::Pad),
            (&["synth", "syn", "arp", "lead_synth", "moog"], Role::Synth),
            (&["strings", "string", "str", "violin", "viola", "cello", "orch"], Role::Strings),
            (&["woodwind", "woodwinds", "flute", "clarinet", "oboe", "bassoon", "piccolo"], Role::Woodwinds),
            (&["brass", "horn", "horns", "trumpet", "tpt", "sax", "trombone"], Role::Brass),
            (&["fx", "sfx", "riser", "noise", "impact", "sweep"], Role::Fx),
        ];
        for (keys, role) in rules {
            if has(keys) {
                return *role;
            }
        }
        Role::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: PRJ-6
    #[test]
    fn guesses_roles_from_file_names() {
        let cases = [
            ("Kick In.wav", Role::Kick),
            ("01_snare_top.wav", Role::Snare),
            ("OH L.flac", Role::Overheads),
            ("Bass DI.wav", Role::Bass),
            ("Lead Vox.wav", Role::LeadVocal),
            ("BVs Chorus.aif", Role::BackingVocal),
            ("Acoustic Gtr.wav", Role::AcousticGuitar),
            ("Gtr Rhythm L.wav", Role::ElectricGuitar),
            ("Rhodes.wav", Role::Keys),
            ("Strings Ensemble.wav", Role::Strings),
            ("Horns.wav", Role::Brass),
            ("Room Mics.wav", Role::Room),
            ("Grand Piano L.wav", Role::Piano),
            ("Flute 1.wav", Role::Woodwinds),
            ("Choir SATB.wav", Role::Choir),
            ("Alto Sax.wav", Role::Brass),
            ("track 7.wav", Role::Other),
        ];
        for (name, role) in cases {
            assert_eq!(Role::guess(name), role, "{name}");
        }
    }

    /// req: PRJ-6
    #[test]
    fn every_role_has_a_distinct_name() {
        let names: std::collections::HashSet<_> = Role::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(names.len(), Role::ALL.len());
        for r in Role::ALL {
            let json = serde_json::to_string(&r).unwrap();
            assert_eq!(json, format!("\"{}\"", r.as_str()));
        }
    }
}
