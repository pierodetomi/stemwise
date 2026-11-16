//! Blocks in the mix plan (`mix_plan` v4, AI-25): a block of the plan is its kind's fields, as FOUNDATION §5.5 writes
//! them; here they become the core's block (`stemwise_core::fx::Fx`) and back. The few names that differ: `rate` is
//! the musical value of the core's `sync`; a band's `type` is the core's `kind`; bands and voices of the plan are on.

use serde_json::{Map, Value, json};
use stemwise_core::fx::{Fx, FxBlock, FxKind};
use stemwise_core::model::Chain;

use crate::analysis_doc::Ids;
use crate::plan::{BlockPlan, BlockPosition, LIMITS};

/// The blocks the analysis lists and a plan addresses by a `k…` id: every block but the first of each classic kind,
/// which the fields of version 3 (`hpf_hz`, `eq`, `compressor`, `saturation`) already address.
pub fn listed(c: &Chain) -> Vec<&FxBlock> {
    c.blocks
        .iter()
        .filter(|b| !(FxKind::CLASSIC.contains(&b.fx.kind()) && c.first(b.fx.kind()).is_some_and(|f| f.id == b.id)))
        .collect()
}

/// Fields of a kind the plan may set, with the limit path that bounds them (`None`: the interface's range only).
pub fn fields(kind: FxKind) -> &'static [(&'static str, Field)] {
    use Field::*;
    match kind {
        FxKind::Hpf | FxKind::Lpf => &[("freq_hz", Num(None)), ("resonance", Num(Some("tracks.<id>.hpf_resonance")))],
        FxKind::Eq => &[("bands", EqBands)],
        FxKind::Compressor => &[
            ("threshold_db", Num(Some("compressor.threshold_db"))),
            ("ratio", Num(Some("compressor.ratio"))),
            ("attack_ms", Num(Some("compressor.attack_ms"))),
            ("release_ms", Num(Some("compressor.release_ms"))),
            ("knee_db", Num(Some("compressor.knee_db"))),
            ("makeup_db", Num(Some("compressor.makeup_db"))),
            ("mix", Num(Some("compressor.mix"))),
            ("sidechain", Key),
        ],
        FxKind::Saturation => &[
            ("drive_db", Num(Some("saturation.drive_db"))),
            ("mix", Num(Some("saturation.mix"))),
            ("character", Enum(&["soft", "tape", "tube", "transistor"])),
        ],
        FxKind::Gate => &[
            ("threshold_db", Num(Some("gate.threshold_db"))),
            ("range_db", Num(Some("gate.range_db"))),
            ("ratio", Num(Some("gate.ratio"))),
            ("attack_ms", Num(Some("gate.attack_ms"))),
            ("hold_ms", Num(Some("gate.hold_ms"))),
            ("release_ms", Num(Some("gate.release_ms"))),
            ("sidechain", Key),
        ],
        FxKind::Deesser => &[
            ("freq_hz", Num(Some("deesser.freq_hz"))),
            ("threshold_db", Num(None)),
            ("max_reduction_db", Num(Some("deesser.max_reduction_db"))),
            ("mode", Enum(&["split", "wide"])),
        ],
        FxKind::DynamicEq => &[("bands", DeqBands)],
        FxKind::Transient => &[
            ("attack_db", Num(Some("transient.attack_db"))),
            ("sustain_db", Num(Some("transient.sustain_db"))),
            ("speed", Num(None)),
            ("mix", Num(None)),
        ],
        FxKind::Clipper => &[("threshold_db", Num(Some("clipper.threshold_db"))), ("softness", Num(None))],
        FxKind::Multiband => &[("crossovers_hz", Crossovers), ("bands", MbBands)],
        FxKind::Exciter => &[
            ("freq_hz", Num(Some("exciter.freq_hz"))),
            ("drive_db", Num(None)),
            ("amount", Num(Some("exciter.amount"))),
        ],
        FxKind::BassEnhancer => &[
            ("freq_hz", Num(Some("bass_enhancer.freq_hz"))),
            ("harmonics", Num(Some("bass_enhancer.harmonics"))),
            ("mix", Num(None)),
        ],
        FxKind::Width => &[("width", Num(Some("width.width"))), ("mono_below_hz", Num(Some("width.mono_below_hz")))],
        FxKind::Modulation => &[
            ("mode", Enum(&["chorus", "flanger", "phaser"])),
            ("rate", Rate),
            ("rate_hz", Num(Some("modulation.rate_hz"))),
            ("depth", Num(Some("modulation.depth"))),
            ("feedback", Num(Some("modulation.feedback"))),
            ("voices", Int(1, 3)),
            ("stages", IntOf(&[4, 6, 8, 12])),
            ("spread_deg", Num(None)),
            ("mix", Num(Some("modulation.mix"))),
        ],
        FxKind::Tremolo => &[
            ("mode", Enum(&["tremolo", "autopan"])),
            ("rate", Rate),
            ("rate_hz", Num(Some("tremolo.rate_hz"))),
            ("depth", Num(Some("tremolo.depth"))),
            ("shape", Enum(&["sine", "triangle", "square"])),
            ("phase_deg", Num(None)),
        ],
        FxKind::Harmonizer => &[("voices", HarmVoices), ("mix", Num(Some("harmonizer.mix")))],
        FxKind::Doubler => &[
            ("voices", Int(1, 2)),
            ("detune_cents", Num(Some("doubler.detune_cents"))),
            ("delay_ms", Num(Some("doubler.delay_ms"))),
            ("spread", Num(None)),
            ("mix", Num(Some("doubler.mix"))),
        ],
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Field {
    Num(Option<&'static str>),
    Int(i64, i64),
    IntOf(&'static [i64]),
    Enum(&'static [&'static str]),
    /// A track or group id, or null (sidechain).
    Key,
    /// A musical value, or null; only with a tempo.
    Rate,
    EqBands,
    DeqBands,
    MbBands,
    Crossovers,
    HarmVoices,
}

/// Musical values a rate may take (FX-20).
pub const RATES: [&str; 8] = ["1/1", "1/2", "1/4", "1/4.", "1/8", "1/8.", "1/8t", "1/16"];

fn neutral_key(v: &Value, ids: &Ids) -> Value {
    match v.as_str() {
        Some(s) => json!(ids.n(s)),
        None => Value::Null,
    }
}

fn project_key(v: &Value, ids: &Ids) -> Value {
    match v.as_str() {
        Some(s) => json!(ids.to_project.get(s).cloned().unwrap_or_else(|| s.to_string())),
        None => Value::Null,
    }
}

/// The settings of a core block as a plan writes them (for a revision's starting plan and the analysis).
pub fn settings_of(fx: &Fx, ids: &Ids) -> Map<String, Value> {
    let mut v = serde_json::to_value(fx).expect("serialises");
    let o = v.as_object_mut().expect("a block is an object");
    o.remove("type");
    o.remove("bypass");
    let kind = fx.kind();
    if let Some(s) = o.remove("sync") {
        o.insert("rate".into(), s);
    }
    match kind {
        FxKind::Compressor => {
            let sc = o.get("sidechain").and_then(|s| s.get("source")).cloned().unwrap_or(Value::Null);
            o.insert("sidechain".into(), neutral_key(&sc, ids));
        }
        FxKind::Gate => {
            let sc = o.get("sidechain").cloned().unwrap_or(Value::Null);
            o.insert("sidechain".into(), neutral_key(&sc, ids));
            o.remove("detector_hpf_hz");
            o.remove("detector_lpf_hz");
        }
        FxKind::Eq | FxKind::DynamicEq => {
            if let Some(Value::Array(bands)) = o.get_mut("bands") {
                bands.retain(|b| b.get("enabled") == Some(&json!(true)));
                for b in bands.iter_mut() {
                    let b = b.as_object_mut().expect("band");
                    b.remove("enabled");
                    if let Some(k) = b.remove("kind") {
                        b.insert("type".into(), k);
                    }
                    if let Some(sc) = b.get("sidechain").cloned() {
                        b.insert("sidechain".into(), neutral_key(&sc, ids));
                    }
                }
            }
        }
        FxKind::Multiband => {
            if let Some(Value::Array(bands)) = o.get_mut("bands") {
                for b in bands.iter_mut() {
                    b.as_object_mut().expect("band").remove("bypass");
                }
            }
        }
        FxKind::Harmonizer => {
            if let Some(Value::Array(v)) = o.get_mut("voices") {
                v.retain(|x| x.get("enabled") == Some(&json!(true)));
                for x in v.iter_mut() {
                    x.as_object_mut().expect("voice").remove("enabled");
                }
            }
        }
        _ => {}
    }
    o.clone()
}

/// The core block a plan's block makes: `base` (the block as it is, or a new one) with the plan's settings over it.
/// A new block from a plan is on unless the plan says `bypass`.
pub fn to_fx(bp: &BlockPlan, base: Option<&Fx>, ids: &Ids) -> Result<Fx, String> {
    let kind = bp.kind;
    let start = base.cloned().unwrap_or_else(|| {
        let mut f = Fx::new(kind);
        f.set_bypass(false);
        f
    });
    let mut v = serde_json::to_value(&start).expect("serialises");
    let o = v.as_object_mut().expect("object");
    let known = fields(kind);
    for (k, val) in &bp.settings {
        let Some((_, field)) = known.iter().find(|(f, _)| f == k) else {
            return Err(format!("block {} ({}): unknown field `{k}`", bp.id, kind.key()));
        };
        // `null` keeps the value (the prompt says so; strict mode requires every field): bands, voices, keys and
        // musical rates included. A new block keeps its neutral default.
        if val.is_null() {
            continue;
        }
        match field {
            Field::Rate => {
                o.insert("sync".into(), val.clone());
            }
            Field::Key => {
                let src = project_key(val, ids);
                if kind == FxKind::Compressor {
                    let hpf = o.get("sidechain").and_then(|s| s.get("hpf_hz")).cloned().unwrap_or(json!(100.0));
                    o.insert(
                        "sidechain".into(),
                        if src.is_null() { Value::Null } else { json!({"source": src, "hpf_hz": hpf}) },
                    );
                } else {
                    o.insert("sidechain".into(), src);
                }
            }
            Field::EqBands => {
                let defaults = stemwise_core::model::Equalizer::track_default().bands;
                let given = val.as_array().cloned().unwrap_or_default();
                if given.len() > defaults.len() {
                    return Err(format!("block {}: an EQ has four bands", bp.id));
                }
                let bands: Vec<Value> = (0..defaults.len())
                    .map(|i| match given.get(i) {
                        Some(b) => json!({"enabled": true, "kind": b.get("type").cloned().unwrap_or(json!("peak")),
                            "freq_hz": b.get("freq_hz").cloned().unwrap_or(json!(1000.0)),
                            "gain_db": b.get("gain_db").cloned().unwrap_or(json!(0.0)), "q": b.get("q").cloned().unwrap_or(json!(1.0))}),
                        None => serde_json::to_value(defaults[i]).expect("serialises"),
                    })
                    .collect();
                o.insert("bands".into(), Value::Array(bands));
            }
            Field::DeqBands => {
                let given = val.as_array().cloned().unwrap_or_default();
                let bands: Vec<Value> = given
                    .iter()
                    .map(|b| {
                        let mut m = b.as_object().cloned().unwrap_or_default();
                        if let Some(t) = m.remove("type") {
                            m.insert("kind".into(), t);
                        }
                        m.entry("kind").or_insert(json!("peak"));
                        m.insert("enabled".into(), json!(true));
                        let sc = m.get("sidechain").cloned().unwrap_or(Value::Null);
                        m.insert("sidechain".into(), project_key(&sc, ids));
                        for (k, d) in [
                            ("q", 1.2),
                            ("threshold_db", -30.0),
                            ("attack_ms", 10.0),
                            ("release_ms", 120.0),
                            ("freq_hz", 3000.0),
                            ("range_db", -3.0),
                        ] {
                            m.entry(k).or_insert(json!(d));
                        }
                        Value::Object(m)
                    })
                    .collect();
                o.insert("bands".into(), Value::Array(bands));
            }
            Field::MbBands => {
                let given = val.as_array().cloned().unwrap_or_default();
                let bands: Vec<Value> = given
                    .iter()
                    .map(|b| {
                        let mut m = b.as_object().cloned().unwrap_or_default();
                        m.insert("bypass".into(), json!(false));
                        Value::Object(m)
                    })
                    .collect();
                o.insert("bands".into(), Value::Array(bands));
            }
            Field::HarmVoices => {
                let given = val.as_array().cloned().unwrap_or_default();
                let voices: Vec<Value> = given
                    .iter()
                    .map(|b| {
                        let mut m = b.as_object().cloned().unwrap_or_default();
                        m.insert("enabled".into(), json!(true));
                        for (k, d) in [("cents", 0.0), ("level_db", -6.0), ("pan", 0.0)] {
                            m.entry(k).or_insert(json!(d));
                        }
                        Value::Object(m)
                    })
                    .collect();
                o.insert("voices".into(), Value::Array(voices));
            }
            _ => {
                if !val.is_null() {
                    o.insert(k.clone(), val.clone());
                }
            }
        }
    }
    if let Some(by) = bp.bypass {
        o.insert("bypass".into(), json!(by));
    }
    // A musical rate given without its value in hertz: the core sets the rate from the tempo when applied.
    serde_json::from_value(v).map_err(|e| format!("block {} ({}): {e}", bp.id, kind.key()))
}

/// The plan's limits of a block's settings (AI-8): the errors of the fields outside them. `current` are the settings
/// the block has in the project, if it exists: the interface allows more than the plan does, and a value the author
/// set by hand and the plan keeps as it is is accepted (AI-3), as for the classic fields.
pub fn check(bp: &BlockPlan, current: Option<&Map<String, Value>>) -> Vec<String> {
    let mut out = Vec::new();
    let mut counts = Vec::new();
    let lim = |path: &str| LIMITS.iter().find(|l| l.path == path);
    let same = |a: &Value, b: Option<&Value>| match (a.as_f64(), b.and_then(Value::as_f64)) {
        (Some(x), Some(y)) => (x - y).abs() < 1e-9,
        _ => false,
    };
    let mut num = |path: String, v: &Value, was: Option<&Value>, what: &str| {
        if same(v, was) {
            return;
        }
        if let (Some(l), Some(x)) = (lim(&path), v.as_f64())
            && !(l.min..=l.max).contains(&x)
        {
            out.push(format!("block {}: {what} = {x} is outside {} … {}", bp.id, l.min, l.max));
        }
    };
    let k = bp.kind.key_name();
    for (f, field) in fields(bp.kind) {
        let Some(v) = bp.settings.get(*f) else { continue };
        match field {
            Field::Num(Some(path)) => num((*path).to_string(), v, current.and_then(|c| c.get(*f)), f),
            Field::DeqBands | Field::MbBands | Field::HarmVoices => {
                // How many (§5.5): enforced by the strict schema, checked here for the JSON-mode fallback (AI-8).
                if let Some(a) = v.as_array() {
                    let (lo, hi, what) = match field {
                        Field::MbBands => (2, 4, "bands"),
                        Field::HarmVoices => (1, 2, "voices"),
                        _ => (1, 2, "bands"),
                    };
                    if !(lo..=hi).contains(&a.len()) {
                        counts.push(format!("block {}: {} {what}, from {lo} to {hi}", bp.id, a.len()));
                    }
                }
                for (i, b) in v.as_array().into_iter().flatten().enumerate() {
                    let was = current.and_then(|c| c.get(*f)).and_then(|a| a.get(i));
                    for (bk, bv) in b.as_object().into_iter().flatten() {
                        let p = if *field == Field::HarmVoices {
                            format!("{k}.voices[].{bk}")
                        } else {
                            format!("{k}.bands[].{bk}")
                        };
                        num(p, bv, was.and_then(|w| w.get(bk)), bk);
                    }
                }
            }
            _ => {}
        }
    }
    out.extend(counts);
    out
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// The kind's name as the limits table writes it (`dynamic_eq`, `bass_enhancer`).
trait KindName {
    fn key_name(&self) -> &'static str;
}

impl KindName for FxKind {
    fn key_name(&self) -> &'static str {
        match self {
            FxKind::Hpf => "hpf",
            FxKind::Lpf => "lpf",
            FxKind::Eq => "eq",
            FxKind::Compressor => "compressor",
            FxKind::Saturation => "saturation",
            FxKind::Gate => "gate",
            FxKind::Deesser => "deesser",
            FxKind::DynamicEq => "dynamic_eq",
            FxKind::Transient => "transient",
            FxKind::Clipper => "clipper",
            FxKind::Multiband => "multiband",
            FxKind::Exciter => "exciter",
            FxKind::BassEnhancer => "bass_enhancer",
            FxKind::Width => "width",
            FxKind::Modulation => "modulation",
            FxKind::Tremolo => "tremolo",
            FxKind::Harmonizer => "harmonizer",
            FxKind::Doubler => "doubler",
        }
    }
}

/// Where a new block goes in a chain (AI-25): first, around the first compressor, or last (before the fader).
pub fn insert_at(c: &Chain, pos: BlockPosition) -> usize {
    let comp = c.blocks.iter().position(|b| b.fx.kind() == FxKind::Compressor);
    match pos {
        BlockPosition::First => 0,
        BlockPosition::BeforeCompressor => comp.unwrap_or(c.blocks.len()),
        BlockPosition::AfterCompressor => comp.map_or(c.blocks.len(), |i| i + 1),
        BlockPosition::Last => c.blocks.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stemwise_core::fx::classic_blocks;

    fn bp(kind: FxKind, settings: Value) -> BlockPlan {
        BlockPlan {
            id: "k9".into(),
            kind,
            idea: None,
            position: Some(BlockPosition::AfterCompressor),
            bypass: None,
            settings: settings.as_object().cloned().unwrap_or_default(),
        }
    }

    /// req: AI-25, AI-8
    #[test]
    fn null_keeps_the_value_and_counts_are_checked_without_the_strict_schema() {
        let ids = Ids::default();
        let mut base = Fx::new(FxKind::DynamicEq);
        let Fx::DynamicEq(d) = &mut base else { unreachable!() };
        d.bands[0].freq_hz = 2500.0;
        d.bands[0].sidechain = Some("v".into());
        let bands = d.bands.clone();
        // Strict mode sends every field: switching the block off with the rest `null` changes nothing else.
        let off = BlockPlan { bypass: Some(true), ..bp(FxKind::DynamicEq, json!({"bands": null})) };
        let Fx::DynamicEq(got) = to_fx(&off, Some(&base), &ids).unwrap() else { unreachable!() };
        assert!(got.bypass && got.bands == bands);
        assert!(check(&bp(FxKind::DynamicEq, json!({"bands": []})), None).iter().any(|e| e.contains("from 1 to 2")));
        assert!(
            check(&bp(FxKind::Multiband, json!({"bands": [{}, {}, {}, {}, {}]})), None)
                .iter()
                .any(|e| e.contains("from 2 to 4"))
        );
    }

    /// req: AI-25
    #[test]
    fn plan_blocks_become_core_blocks_and_back() {
        let ids = Ids::default();
        let d = to_fx(
            &bp(
                FxKind::Deesser,
                json!({"freq_hz": 6500, "threshold_db": -28.0, "max_reduction_db": 5.0, "mode": "split"}),
            ),
            None,
            &ids,
        )
        .unwrap();
        assert!(matches!(&d, Fx::Deesser(x) if !x.bypass && x.freq_hz == 6500.0 && x.max_reduction_db == 5.0));
        let deq = to_fx(
            &bp(FxKind::DynamicEq, json!({"bands": [{"type": "peak", "freq_hz": 3000, "q": 1.2, "threshold_db": -30.0, "range_db": -3.0, "attack_ms": 10.0, "release_ms": 120.0, "sidechain": "t4"}]})),
            None,
            &ids,
        )
        .unwrap();
        let Fx::DynamicEq(x) = &deq else { panic!() };
        assert_eq!((x.bands.len(), x.bands[0].enabled, x.bands[0].sidechain.as_deref()), (1, true, Some("t4")));
        let m = to_fx(&bp(FxKind::Modulation, json!({"mode": "chorus", "rate": "1/4", "rate_hz": null, "depth": 0.4, "voices": 2, "stages": null, "spread_deg": 120, "mix": 0.35, "feedback": 0.0})), None, &ids).unwrap();
        assert!(matches!(&m, Fx::Modulation(x) if x.sync.as_deref() == Some("1/4") && x.mix == 0.35 && x.stages == 6));
        // Back: the same settings, with the plan's names.
        let s = settings_of(&m, &ids);
        assert_eq!(s["rate"], json!("1/4"));
        assert!(s.get("bypass").is_none() && s.get("sync").is_none());
        // Unknown fields and the plan's limits are refused.
        assert!(to_fx(&bp(FxKind::Gate, json!({"bogus": 1})), None, &ids).is_err());
        assert!(!check(&bp(FxKind::Width, json!({"width": 1.9})), None).is_empty());
        assert!(check(&bp(FxKind::Width, json!({"width": 1.4})), None).is_empty());
        // AI-3: a value the author set beyond the plan's limit is accepted when the plan keeps it, not a new one.
        let was = json!({"width": 1.9});
        assert!(check(&bp(FxKind::Width, json!({"width": 1.9})), was.as_object()).is_empty());
        assert!(!check(&bp(FxKind::Width, json!({"width": 1.8})), was.as_object()).is_empty());
        assert!(!check(&bp(FxKind::Harmonizer, json!({"voices": [{"semitones": 14}]})), None).is_empty());
        // Positions and the listed blocks.
        let c = Chain { trim_db: 0.0, blocks: classic_blocks(), fader_db: 0.0, pan: 0.0 };
        assert_eq!(insert_at(&c, BlockPosition::AfterCompressor), 4);
        assert_eq!(insert_at(&c, BlockPosition::BeforeCompressor), 3);
        assert!(listed(&c).is_empty(), "the five classic blocks are addressed by the fields of version 3");
    }
}
