//! Master AI (MAS-6): the model gets the mixdown's measures, the reference profile (optional), the brief and the
//! destination, and returns a `master_plan` as a proposal under the same rules as the mix (AI-2, AI-8, AI-10),
//! revision by request included.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use stemwise_analysis::evaluate::spectral_distance;
use stemwise_analysis::measures::{self, MixMeasures, Target};
use stemwise_core::command::Command;
use stemwise_core::model::{
    AiCallKind, AiCallRecord, AiOutcome, Block, Clipper, EqBand as CoreBand, EqKind, EqOffsets, Equalizer,
    LoudnessTarget, MasterBlock, MasterChain, MasterComp, MasterState, MatchEq, MbBand, Multiband, Project, Saturation,
    SectionOffsets, Width, master_limits,
};
use stemwise_dsp::loudness;
use stemwise_engine::master::{GrStats, MasterParams, loudness_search, render_metered};

use crate::analysis_doc::{Ids, r1, r2};
use crate::apply::{DiffRow, TrackDiff};
use crate::decider::{DecideError, Decider, DecisionInput, Kind, Retry, prompt_estimate};
use crate::log::{self, Meter};
use crate::mixing::{AiError, AiPhase, AiProgress, MAX_RETRIES};
use crate::plan::*;
use crate::validate::validate_master;

/// Version of the master analysis document.
pub const MASTER_ANALYSIS_VERSION: u32 = 3;

/// Plans a master call may evaluate: each one is rendered and measured, and the next decision sees the result.
pub const MASTER_ITERATIONS: u32 = 3;

/// Window of the short crest factor.
const CREST_WINDOW_S: f64 = 0.4;

/// How the peaks of a signal stand against its body: what a loud destination has to reduce.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Dynamics {
    /// True peak minus integrated loudness.
    pub peak_to_loudness_db: Option<f64>,
    /// Median crest factor (sample peak minus RMS) of the 400 ms windows that are not quiet: the punch of the hits.
    pub short_crest_db: Option<f64>,
    /// Percentage of the samples within 3, 6 and 9 dB of the highest peak: small values mean few, isolated peaks.
    pub peak_share_pct: [f64; 3],
}

/// [`Dynamics`] of an interleaved stereo signal whose loudness and true peak are already measured.
pub fn dynamics(samples: &[f32], rate: u32, lufs: Option<f64>, true_peak_db: f64) -> Dynamics {
    let frames = samples.len() / 2;
    let level = |i: usize| f64::from(samples[2 * i].abs().max(samples[2 * i + 1].abs()));
    let peak = (0..frames).map(level).fold(0.0f64, f64::max);
    let mut peak_share_pct = [0.0; 3];
    if peak > 0.0 && frames > 0 {
        let limits = [3.0f64, 6.0, 9.0].map(|d| peak * 10f64.powf(-d / 20.0));
        let mut counts = [0u64; 3];
        for i in 0..frames {
            let v = level(i);
            for (c, l) in counts.iter_mut().zip(limits) {
                *c += u64::from(v > l);
            }
        }
        peak_share_pct = counts.map(|c| c as f64 * 100.0 / frames as f64);
    }
    // Crest of each window, for the windows within 10 dB of the overall RMS (silences and tails say nothing).
    let w = ((f64::from(rate) * CREST_WINDOW_S) as usize).max(1);
    let mut windows: Vec<(f64, f64)> = Vec::new();
    let mut total = 0.0f64;
    for k in 0..frames / w {
        let (mut pk, mut sq) = (0.0f64, 0.0f64);
        for i in k * w..(k + 1) * w {
            let (l, r) = (f64::from(samples[2 * i]), f64::from(samples[2 * i + 1]));
            pk = pk.max(l.abs()).max(r.abs());
            sq += l * l + r * r;
        }
        total += sq;
        windows.push((pk, sq / (2 * w) as f64));
    }
    let mean = if windows.is_empty() { 0.0 } else { total / (2 * w * windows.len()) as f64 };
    let mut crests: Vec<f64> = windows
        .iter()
        .filter(|(pk, ms)| *pk > 0.0 && *ms > mean * 0.1)
        .map(|(pk, ms)| 20.0 * pk.log10() - 10.0 * ms.log10())
        .collect();
    crests.sort_by(f64::total_cmp);
    Dynamics {
        peak_to_loudness_db: lufs.filter(|_| true_peak_db.is_finite()).map(|l| true_peak_db - l),
        short_crest_db: (!crests.is_empty()).then(|| crests[crests.len() / 2]),
        peak_share_pct,
    }
}

/// Gain reduction of a dynamics block over the whole master, as the model reads it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Reduction {
    pub max_db: f64,
    pub mean_db: f64,
    /// Percentage of the time with any reduction at all.
    #[serde(default)]
    pub time_active_pct: f64,
    /// Percentage of the time with more than 1, 3 and 6 dB of reduction.
    pub time_over_1_db_pct: f64,
    pub time_over_3_db_pct: f64,
    pub time_over_6_db_pct: f64,
}

impl From<GrStats> for Reduction {
    fn from(g: GrStats) -> Self {
        Reduction {
            max_db: r1(g.max_db),
            mean_db: r1(g.mean_db),
            time_active_pct: r1(g.active * 100.0),
            time_over_1_db_pct: r1(g.over_1 * 100.0),
            time_over_3_db_pct: r1(g.over_3 * 100.0),
            time_over_6_db_pct: r1(g.over_6 * 100.0),
        }
    }
}

/// A master rendered and measured (MAS-6): what a plan really does to the mixdown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MasterEvaluation {
    pub lufs: Option<f64>,
    pub true_peak_db: f64,
    /// Gain before the limiter that the app found to reach the destination.
    pub loudness_gain_db: f64,
    /// The destination's loudness is reached (within 0.5 LU).
    pub on_target: bool,
    pub loudness_range_lu: Option<f64>,
    pub peak_to_loudness_db: Option<f64>,
    pub short_crest_db: Option<f64>,
    /// Short crest of the mixdown minus the master's: how much punch the chain took away.
    pub short_crest_lost_db: Option<f64>,
    pub peak_share_pct: [f64; 3],
    /// Gain reduction of each compressor, multiband and clipper of the chain that is on, in chain order.
    pub blocks: Vec<BlockReduction>,
    pub limiter: Reduction,
    pub spectral_distance_db: Option<f64>,
    pub band_delta_db: Option<[f64; 8]>,
}

/// Gain reduction of one block of the chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockReduction {
    pub id: String,
    /// "compressor", "multiband" or "clipper".
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub reduction: Reduction,
}

/// What a master decision works on: the mixdown (interleaved stereo at `rate`) with its measures and the references.
pub struct MasterInput<'a> {
    pub samples: &'a [f32],
    pub rate: u32,
    pub measures: &'a MixMeasures,
    /// The references' mean profile (ANA-12), if any.
    pub target: Option<&'a Target>,
    /// Loudness and true peak of each reference, as mastered.
    pub references: Vec<(Option<f64>, f64)>,
    /// Loudness gain of the current master, if known: the first search starts from it.
    pub start_gain_db: Option<f64>,
}

impl MasterInput<'_> {
    pub fn dynamics(&self) -> Dynamics {
        dynamics(self.samples, self.rate, self.measures.lufs, self.measures.true_peak_db)
    }
}

/// Renders the mixdown through a master state at the destination's loudness and measures the result. `start` is the
/// loudness gain of a nearby state (a shorter search). Deterministic for the same `start` (MAS-8).
pub fn evaluate_master(
    inp: &MasterInput<'_>,
    mix: &Dynamics,
    project: &Project,
    m: &MasterState,
    start: Option<f64>,
) -> Option<MasterEvaluation> {
    let (l, r): (Vec<f32>, Vec<f32>) = inp.samples.as_chunks::<2>().0.iter().map(|c| (c[0], c[1])).unzip();
    let tb = inp.target.and_then(|t| t.bands_db.as_ref());
    let mut p = MasterParams::of(m, inp.measures.bands_db.as_ref(), tb, 0.0).with_sections(m, &project.sections);
    // The export always limits (MAS-3): the plan is judged as the file would be.
    p.limiter_on = true;
    let found = loudness_search(&l, &r, inp.rate, p, m.target.lufs, start, &mut |_| {});
    p.loud_gain_db = found.gain_db;
    p.clip_peak = found.clip_peak;
    let ((a, b), gr) = render_metered(&l, &r, f64::from(inp.rate), p);
    let out: Vec<f32> = a.iter().zip(&b).flat_map(|(x, y)| [*x, *y]).collect();
    let mm = measures::measure_mix(&out, 2, inp.rate).0;
    let d = dynamics(&out, inp.rate, mm.lufs, mm.true_peak_db);
    let delta = mm.bands_db.as_ref().zip(tb).map(|(x, t)| spectral_distance(x, t));
    let blocks = m
        .chain
        .blocks
        .iter()
        .zip(gr.blocks)
        .filter(|(b, _)| !b.block.bypass())
        .filter_map(|(b, g)| {
            let kind = match b.block {
                Block::Compressor(_) => "compressor",
                Block::Multiband(_) => "multiband",
                Block::Clipper(_) => "clipper",
                _ => return None,
            };
            Some(BlockReduction { id: b.id.clone(), kind: kind.into(), reduction: g.into() })
        })
        .collect();
    Some(MasterEvaluation {
        lufs: mm.lufs.map(r1),
        true_peak_db: r1(mm.true_peak_db),
        loudness_gain_db: p.loud_gain_db,
        on_target: mm.lufs.is_some_and(|v| (v - m.target.lufs).abs() <= 0.5),
        loudness_range_lu: mm.dyn_range_lu.map(r1),
        peak_to_loudness_db: d.peak_to_loudness_db.map(r1),
        short_crest_db: d.short_crest_db.map(r1),
        short_crest_lost_db: mix.short_crest_db.zip(d.short_crest_db).map(|(x, y)| r1(x - y)),
        peak_share_pct: d.peak_share_pct.map(r2),
        blocks,
        limiter: gr.limiter.into(),
        spectral_distance_db: delta.map(|x| r1(x.0)),
        band_delta_db: delta.map(|x| x.1.map(r1)),
    })
}

/// The sections of the song as the model sees them: neutral id, name, where they lie and how loud the mixdown is
/// there (the mixdown starts at frame 0 of the timeline).
fn sections_doc(inp: &MasterInput<'_>, p: &Project, ids: &Ids) -> Vec<Value> {
    let rate = f64::from(inp.rate);
    let frames = (inp.samples.len() / 2) as i64;
    let mut secs: Vec<_> = p.sections.iter().collect();
    secs.sort_by_key(|s| s.start);
    secs.iter()
        .filter(|s| s.start < frames && s.end > s.start)
        .map(|s| {
            let (a, b) = (s.start.max(0) as usize * 2, s.end.min(frames) as usize * 2);
            json!({
                "id": ids.n(&s.id),
                "name": s.name,
                "start_s": r1(s.start as f64 / rate),
                "end_s": r1(s.end as f64 / rate),
                "lufs": loudness::integrated(&inp.samples[a..b], 2, inp.rate).map(r1),
            })
        })
        .collect()
}

/// Measures of the mixdown, the references and the destination, with the current master as measured: the input of
/// the master decision (version [`MASTER_ANALYSIS_VERSION`]).
pub fn master_document(
    inp: &MasterInput<'_>,
    mix: &Dynamics,
    current: Option<&MasterEvaluation>,
    p: &Project,
) -> Value {
    let ids = Ids::of(p);
    // The blocks the master holds now, in order: their ids are the ones a plan keeps.
    let chain: Vec<Value> = p
        .master
        .chain
        .blocks
        .iter()
        .map(|b| json!({"id": b.id, "type": serde_json::to_value(b.block.kind()).unwrap_or_default(), "on": !b.block.bypass()}))
        .collect();
    let (mixdown, target_profile) = (inp.measures, inp.target);
    let delta = match (mixdown.bands_db.as_ref(), target_profile.and_then(|t| t.bands_db.as_ref())) {
        (Some(m), Some(t)) => Some(spectral_distance(m, t)),
        _ => None,
    };
    let o1 = |v: Option<f64>| v.map(r1);
    let dest = &p.master.target;
    let dest_plr = dest.true_peak_db - dest.lufs;
    let references: Vec<Value> = inp
        .references
        .iter()
        .filter_map(|(lufs, tp)| lufs.filter(|_| tp.is_finite()).map(|l| (l, *tp)))
        .map(|(l, tp)| json!({"lufs": r1(l), "peak_to_loudness_db": r1(tp - l)}))
        .collect();
    json!({
        "version": MASTER_ANALYSIS_VERSION,
        "mixdown": {
            "lufs": o1(mixdown.lufs),
            "true_peak_db": r1(mixdown.true_peak_db),
            "peak_to_loudness_db": o1(mix.peak_to_loudness_db),
            "short_crest_db": o1(mix.short_crest_db),
            "peak_share_pct": mix.peak_share_pct.map(r2),
            "loudness_range_lu": o1(mixdown.dyn_range_lu),
            "bands_db": mixdown.bands_db.map(|b| b.map(r1)),
            "stereo": mixdown.stereo.map(|s| json!({"correlation": s.correlation.map(r2), "side_mid_db": s.side_mid_db.map(r1)})),
            "low_corr": mixdown.low_corr.map(r2),
        },
        "target": target_profile.map(|t| json!({"bands_db": t.bands_db.map(|b| b.map(r1)), "side_mid_db": t.side_mid_db.map(r1), "loudness_range_lu": t.dyn_range_lu.map(r1)})),
        "references": references,
        "spectral_distance_db": delta.map(|d| r1(d.0)),
        "band_delta_db": delta.map(|d| d.1.map(r1)),
        "destination": {
            "preset": dest.preset,
            "lufs": dest.lufs,
            "true_peak_db": dest.true_peak_db,
            "peak_to_loudness_db": r1(dest_plr),
            "peak_reduction_needed_db": mix.peak_to_loudness_db.map(|m| r1((m - dest_plr).max(0.0))),
        },
        "sections": sections_doc(inp, p, &ids),
        "current_chain": chain,
        "current_master": current,
    })
}

fn eq_type(k: EqKind) -> EqType {
    match k {
        EqKind::Peak => EqType::Peak,
        EqKind::LowShelf => EqType::LowShelf,
        EqKind::HighShelf => EqType::HighShelf,
    }
}

fn eq_kind(k: EqType) -> EqKind {
    match k {
        EqType::Peak => EqKind::Peak,
        EqType::LowShelf => EqKind::LowShelf,
        EqType::HighShelf => EqKind::HighShelf,
    }
}

/// Whether a block does something: it is on, and an EQ has at least one band in use.
fn works(b: &Block) -> bool {
    !b.bypass() && !matches!(b, Block::Eq(eq) if eq.bands.iter().all(|x| !x.enabled))
}

/// A block of the master as the plan describes it (with the id the master gives it).
fn block_plan(b: &MasterBlock) -> MasterBlockPlan {
    let (id, why) = (b.id.clone(), String::new());
    match &b.block {
        Block::Eq(eq) => MasterBlockPlan::Eq {
            id,
            bands: eq
                .bands
                .iter()
                .filter(|x| x.enabled)
                .map(|x| EqBand { kind: eq_type(x.kind), freq_hz: x.freq_hz, gain_db: x.gain_db, q: x.q })
                .collect(),
            why,
        },
        Block::MatchEq(m) => MasterBlockPlan::MatchEq { id, amount: m.amount, why },
        Block::Compressor(c) => MasterBlockPlan::Compressor {
            id,
            threshold_db: c.threshold_db,
            ratio: c.ratio,
            attack_ms: c.attack_ms,
            release_ms: c.release_ms,
            knee_db: c.knee_db,
            makeup_db: c.makeup_db,
            mix: c.mix,
            listen_above_hz: c.listen_above_hz,
            why,
        },
        Block::Multiband(m) => MasterBlockPlan::Multiband {
            id,
            crossovers_hz: m.crossovers_hz.clone(),
            bands: m
                .bands
                .iter()
                .map(|x| MbBandPlan {
                    threshold_db: x.threshold_db,
                    // A band that is off compresses nothing.
                    ratio: if x.bypass { 1.0 } else { x.ratio },
                    attack_ms: x.attack_ms,
                    release_ms: x.release_ms,
                    gain_db: x.gain_db,
                })
                .collect(),
            why,
        },
        Block::Width(w) => {
            MasterBlockPlan::Width { id, side_gain_db: w.side_gain_db, mono_below_hz: w.mono_below_hz, why }
        }
        Block::Saturation(x) => MasterBlockPlan::Saturation { id, drive_db: x.drive_db, mix: x.mix, why },
        Block::Clipper(c) => MasterBlockPlan::Clipper { id, take_off_db: c.take_off_db, softness: c.softness, why },
    }
}

/// The current master as a plan (the starting point of a revision): the blocks that are on, in their order.
/// `ids` gives the sections the names the model knows them by.
pub fn plan_from_master(m: &MasterState, ids: &Ids) -> MasterPlan {
    let c = &m.chain;
    let chain: Vec<MasterBlockPlan> = c.blocks.iter().filter(|b| works(&b.block)).map(block_plan).collect();
    let on = |id: &str| chain.iter().any(|b| b.id() == id);
    MasterPlan {
        version: MASTER_PLAN_VERSION,
        summary: String::new(),
        target: MasterTarget { lufs: m.target.lufs, true_peak_db: m.target.true_peak_db },
        input_gain_db: c.input_gain_db,
        sections: m
            .sections
            .iter()
            .filter(|s| !s.is_empty() && ids.to_neutral.contains_key(&s.section))
            .map(|s| MasterSectionPlan {
                section: ids.n(&s.section),
                level_db: s.level_db,
                side_gain_db: s.side_gain_db,
                limiter_release_ms: s.limiter_release_ms,
                eq: s
                    .eq
                    .iter()
                    .filter(|e| on(&e.block))
                    .filter_map(|e| {
                        // The plan lists only the bands in use: the offsets follow them.
                        let Some(Block::Eq(eq)) = c.block(&e.block).map(|b| &b.block) else { return None };
                        let gains_db =
                            eq.bands.iter().zip(&e.gains_db).filter(|(b, _)| b.enabled).map(|(_, g)| *g).collect();
                        Some(SectionEqPlan { block: e.block.clone(), gains_db })
                    })
                    .collect(),
            })
            .collect(),
        chain,
        limiter: LimiterPlan { release_ms: c.limiter.release_ms },
        rationale: Default::default(),
    }
}

/// The master state a plan produces (mixdown, metadata and export settings unchanged). The blocks of the plan come
/// first, in its order and switched on: one that names a block of the master keeps its id, another is a new block.
/// The blocks of the master the plan does not use stay after them, switched off with their settings, as long as
/// the chain has room. `ids` turns the plan's section names back into the project's.
pub fn master_from_plan(old: &MasterState, plan: &MasterPlan, ids: &Ids) -> MasterState {
    let mut m = old.clone();
    let mut blocks: Vec<MasterBlock> = Vec::new();
    // Plan id → id in the master, for the section offsets.
    let mut names: Vec<(String, String)> = Vec::new();
    let defaults = Equalizer::master_default();
    for b in &plan.chain {
        let kind = b.kind();
        if blocks.iter().filter(|x| x.block.kind() == kind).count() >= kind.max()
            || blocks.len() >= master_limits::FREE_BLOCKS
        {
            continue;
        }
        let kept = old.chain.block(b.id()).filter(|x| x.block.kind() == kind && !blocks.iter().any(|y| y.id == x.id));
        let id = match kept {
            Some(x) => x.id.clone(),
            None => (1..)
                .map(|n| format!("b{n}"))
                .find(|id| old.chain.block(id).is_none() && !blocks.iter().any(|y| &y.id == id))
                .expect("an unused id exists"),
        };
        let block = match b {
            MasterBlockPlan::Eq { bands, .. } => Block::Eq(Equalizer {
                bypass: false,
                bands: (0..MAX_MASTER_EQ)
                    .map(|i| match bands.get(i) {
                        Some(x) => CoreBand {
                            enabled: true,
                            kind: eq_kind(x.kind),
                            freq_hz: x.freq_hz,
                            gain_db: x.gain_db,
                            q: x.q,
                        },
                        None => defaults.bands[i],
                    })
                    .collect(),
            }),
            MasterBlockPlan::MatchEq { amount, .. } => Block::MatchEq(MatchEq { bypass: false, amount: *amount }),
            MasterBlockPlan::Compressor {
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                knee_db,
                makeup_db,
                mix,
                listen_above_hz,
                ..
            } => Block::Compressor(MasterComp {
                bypass: false,
                threshold_db: *threshold_db,
                ratio: *ratio,
                attack_ms: *attack_ms,
                release_ms: *release_ms,
                knee_db: *knee_db,
                makeup_db: *makeup_db,
                mix: *mix,
                listen_above_hz: *listen_above_hz,
            }),
            MasterBlockPlan::Multiband { crossovers_hz, bands, .. } => Block::Multiband(Multiband {
                bypass: false,
                crossovers_hz: crossovers_hz.clone(),
                bands: bands
                    .iter()
                    .map(|x| MbBand {
                        bypass: false,
                        threshold_db: x.threshold_db,
                        ratio: x.ratio,
                        attack_ms: x.attack_ms,
                        release_ms: x.release_ms,
                        gain_db: x.gain_db,
                    })
                    .collect(),
            }),
            MasterBlockPlan::Width { side_gain_db, mono_below_hz, .. } => {
                Block::Width(Width { bypass: false, side_gain_db: *side_gain_db, mono_below_hz: *mono_below_hz })
            }
            MasterBlockPlan::Saturation { drive_db, mix, .. } => Block::Saturation(Saturation {
                bypass: false,
                drive_db: *drive_db,
                mix: *mix,
                character: Default::default(),
            }),
            MasterBlockPlan::Clipper { take_off_db, softness, .. } => {
                Block::Clipper(Clipper { bypass: false, take_off_db: *take_off_db, softness: *softness })
            }
        };
        names.push((b.id().to_string(), id.clone()));
        blocks.push(MasterBlock { id, block });
    }
    for b in &old.chain.blocks {
        let kind = b.block.kind();
        if blocks.iter().any(|x| x.id == b.id)
            || blocks.iter().filter(|x| x.block.kind() == kind).count() >= kind.max()
            || blocks.len() >= master_limits::FREE_BLOCKS
        {
            continue;
        }
        let mut off = b.clone();
        off.block.set_bypass(true);
        blocks.push(off);
    }
    m.chain.input_gain_db = plan.input_gain_db;
    m.chain.blocks = blocks;
    m.chain.limiter.release_ms = plan.limiter.release_ms;
    m.sections = plan
        .sections
        .iter()
        .filter_map(|s| {
            let section = ids.to_project.get(&s.section)?.clone();
            let eq =
                s.eq.iter()
                    .filter_map(|e| {
                        let block = names.iter().find(|(p, _)| *p == e.block)?.1.clone();
                        let mut gains_db = e.gains_db.clone();
                        gains_db.resize(MAX_MASTER_EQ, 0.0);
                        Some(EqOffsets { block, gains_db })
                    })
                    .collect();
            let o = SectionOffsets {
                section,
                level_db: s.level_db,
                side_gain_db: s.side_gain_db,
                limiter_release_ms: s.limiter_release_ms,
                eq,
            };
            (!o.is_empty()).then_some(o)
        })
        .collect();
    let preset = ["streaming", "loud", "cd"]
        .into_iter()
        .find(|n| {
            LoudnessTarget::preset(n)
                .is_some_and(|t| t.lufs == plan.target.lufs && t.true_peak_db == plan.target.true_peak_db)
        })
        .unwrap_or("custom");
    m.target = LoudnessTarget { preset: preset.into(), lufs: plan.target.lufs, true_peak_db: plan.target.true_peak_db };
    m
}

/// The main values of a block, for the diff: (label, unit, value).
fn block_values(b: &Block) -> Vec<(String, &'static str, f64)> {
    let hz = |f: f64| if f >= 1000.0 { format!("{:.1} kHz", f / 1000.0) } else { format!("{f:.0} Hz") };
    match b {
        Block::Eq(eq) => eq
            .bands
            .iter()
            .enumerate()
            .filter(|(_, x)| x.enabled)
            .map(|(i, x)| (format!("Band {} · {}", i + 1, hz(x.freq_hz)), "dB", x.gain_db))
            .collect(),
        Block::MatchEq(m) => vec![("Amount".into(), "%", m.amount * 100.0)],
        Block::Compressor(c) => vec![
            ("Threshold".into(), "dB", c.threshold_db),
            ("Ratio".into(), ":1", c.ratio),
            ("Attack".into(), "ms", c.attack_ms),
            ("Release".into(), "ms", c.release_ms),
            ("Mix".into(), "%", c.mix * 100.0),
            ("Listens above".into(), "Hz", c.listen_above_hz),
        ],
        Block::Multiband(m) => {
            let mut v = vec![("Bands".to_string(), "", m.bands.len() as f64)];
            v.extend(m.crossovers_hz.iter().enumerate().map(|(i, x)| (format!("Crossover {}", i + 1), "Hz", *x)));
            for (i, band) in m.bands.iter().enumerate() {
                v.push((format!("Band {} threshold", i + 1), "dB", band.threshold_db));
                v.push((format!("Band {} ratio", i + 1), ":1", if band.bypass { 1.0 } else { band.ratio }));
            }
            v
        }
        Block::Width(w) => {
            vec![("Side gain".into(), "dB", w.side_gain_db), ("Mono below".into(), "Hz", w.mono_below_hz)]
        }
        Block::Saturation(x) => vec![("Drive".into(), "dB", x.drive_db), ("Mix".into(), "%", x.mix * 100.0)],
        Block::Clipper(c) => {
            vec![("Take off up to".into(), "dB", c.take_off_db), ("Softness".into(), "%", c.softness * 100.0)]
        }
    }
}

/// One entry of a master proposal: what the author can take or leave on its own (AI-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct MasterChange {
    /// "target", "input", "limiter", "order", "sections", or `block:<id>` (an id of the proposed chain).
    pub key: String,
    /// "EQ 2", "Order", "Sections"…
    pub name: String,
    /// "new" (a block the plan adds), "on", "off", "changed", or "same".
    pub state: String,
    pub rationale: String,
    pub rows: Vec<DiffRow>,
}

/// What a plan changes, entry by entry: the order, each block, the limiter, the sections, the target.
pub fn master_changes(before: &MasterState, after: &MasterState, plan: &MasterPlan) -> Vec<MasterChange> {
    let mut out = Vec::new();
    let row = |label: &str, unit: &str, a: Option<f64>, b: Option<f64>| -> Option<DiffRow> {
        (a.zip(b).is_none_or(|(x, y)| (x - y).abs() >= 0.05) && (a.is_some() || b.is_some())).then(|| DiffRow {
            label: label.into(),
            unit: unit.into(),
            before: a,
            after: b,
            text: None,
        })
    };
    let why = |k: &str| plan.rationale.get(k).cloned().unwrap_or_default();
    let simple = |key: &str, name: &str, rows: Vec<DiffRow>, rationale: String| MasterChange {
        key: key.into(),
        name: name.into(),
        state: if rows.is_empty() { "same" } else { "changed" }.into(),
        rationale,
        rows,
    };
    let (x, y) = (&before.chain, &after.chain);
    // Order: of the blocks that work, as far as both chains have them.
    let on = |c: &MasterChain| c.blocks.iter().filter(|b| works(&b.block)).map(|b| b.id.clone()).collect::<Vec<_>>();
    let (was, now) = (on(x), on(y));
    let common =
        |ids: &[String], other: &[String]| ids.iter().filter(|i| other.contains(i)).cloned().collect::<Vec<_>>();
    if common(&was, &now) != common(&now, &was) {
        out.push(MasterChange {
            key: "order".into(),
            name: "Order".into(),
            state: "changed".into(),
            rationale: why("order"),
            rows: now
                .iter()
                .enumerate()
                .map(|(i, id)| DiffRow {
                    label: y.name(id).unwrap_or_default(),
                    unit: String::new(),
                    before: was.iter().position(|w| w == id).map(|p| p as f64 + 1.0),
                    after: Some(i as f64 + 1.0),
                    text: None,
                })
                .collect(),
        });
    }
    for b in &y.blocks {
        let old = x.block(&b.id);
        let (a_on, b_on) = (old.is_some_and(|o| works(&o.block)), works(&b.block));
        if !a_on && !b_on {
            continue;
        }
        let va = old.filter(|_| a_on).map(|o| block_values(&o.block)).unwrap_or_default();
        let vb = if b_on { block_values(&b.block) } else { Vec::new() };
        let mut rows = Vec::new();
        for (label, unit, v) in &vb {
            let a = va.iter().find(|(l, _, _)| l == label).map(|(_, _, v)| *v);
            rows.extend(row(label, unit, a, Some(*v)));
        }
        for (label, unit, v) in &va {
            if !vb.iter().any(|(l, _, _)| l == label) {
                rows.extend(row(label, unit, Some(*v), None));
            }
        }
        let state = match (old.is_some(), a_on, b_on) {
            (false, _, _) => "new",
            (_, false, true) => "on",
            (_, true, false) => "off",
            _ if rows.is_empty() => "same",
            _ => "changed",
        };
        let rationale = plan
            .chain
            .iter()
            .zip(y.blocks.iter())
            .find(|(_, yb)| yb.id == b.id)
            .map(|(p, _)| p.why().to_string())
            .unwrap_or_default();
        out.push(MasterChange {
            key: format!("block:{}", b.id),
            name: y.name(&b.id).unwrap_or_default(),
            state: state.into(),
            rationale,
            rows,
        });
    }
    let rows: Vec<DiffRow> =
        row("Release", "ms", Some(x.limiter.release_ms), Some(y.limiter.release_ms)).into_iter().collect();
    out.push(simple("limiter", "Limiter", rows, why("limiter")));
    if before.sections != after.sections {
        let rows = after
            .sections
            .iter()
            .flat_map(|s| {
                let old = before.sections.iter().find(|o| o.section == s.section);
                [
                    row(&format!("{} · level", s.section), "dB", old.map(|o| o.level_db), Some(s.level_db)),
                    row(&format!("{} · side", s.section), "dB", old.map(|o| o.side_gain_db), Some(s.side_gain_db)),
                ]
            })
            .flatten()
            .collect();
        out.push(MasterChange {
            key: "sections".into(),
            name: "Sections".into(),
            state: "changed".into(),
            rationale: why("sections"),
            rows,
        });
    }
    let rows: Vec<DiffRow> =
        row("Input gain", "dB", Some(x.input_gain_db), Some(y.input_gain_db)).into_iter().collect();
    if !rows.is_empty() {
        out.push(simple("input", "Input", rows, String::new()));
    }
    let rows: Vec<DiffRow> = [
        row("Target", "LUFS", Some(before.target.lufs), Some(after.target.lufs)),
        row("Ceiling", "dBTP", Some(before.target.true_peak_db), Some(after.target.true_peak_db)),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !rows.is_empty() {
        out.push(simple("target", "Target", rows, String::new()));
    }
    out
}

/// Rows of the master diff (one entry named "Master"): every row of [`master_changes`], labelled with its entry.
pub fn master_diff(before: &MasterState, after: &MasterState, plan: &MasterPlan) -> TrackDiff {
    let changes = master_changes(before, after, plan);
    TrackDiff {
        track: "master".into(),
        name: "Master".into(),
        note: None,
        rationale: changes.iter().map(|c| c.rationale.as_str()).filter(|r| !r.is_empty()).collect::<Vec<_>>().join(" "),
        rows: changes
            .iter()
            .flat_map(|c| {
                c.rows.iter().map(|r| DiffRow {
                    label: if r.label == c.name { r.label.clone() } else { format!("{} · {}", c.name, r.label) },
                    ..r.clone()
                })
            })
            .collect(),
    }
}

/// Applies only some entries of a proposal ([`MasterChange::key`]) to the master as it is: what is not chosen stays.
pub fn master_partial(current: &MasterState, proposed: &MasterState, only: &[String]) -> MasterState {
    let take = |k: &str| only.iter().any(|o| o == k);
    let mut m = current.clone();
    if take("target") {
        m.target = proposed.target.clone();
    }
    if take("input") {
        m.chain.input_gain_db = proposed.chain.input_gain_db;
    }
    if take("limiter") {
        m.chain.limiter.release_ms = proposed.chain.limiter.release_ms;
    }
    let chosen = |id: &str| take(&format!("block:{id}"));
    // Chosen blocks: settings from the proposal; a new one is added (if the chain has room).
    for b in &proposed.chain.blocks {
        if !chosen(&b.id) {
            continue;
        }
        let room = m.chain.can_add(b.block.kind());
        match m.chain.block_mut(&b.id) {
            Some(x) if x.block.kind() == b.block.kind() => x.block = b.block.clone(),
            Some(_) => {}
            None if room => m.chain.blocks.push(b.clone()),
            None => {}
        }
    }
    if take("order") {
        // The proposal's order for the blocks both have; the others keep their place after them.
        let rank = |id: &str| proposed.chain.blocks.iter().position(|b| b.id == id).unwrap_or(usize::MAX);
        m.chain.blocks.sort_by_key(|b| rank(&b.id));
    } else {
        // A block that was added goes where the proposal puts it among the ones it follows there.
        for b in &proposed.chain.blocks {
            if chosen(&b.id) && current.chain.block(&b.id).is_none() {
                let Some(from) = m.chain.blocks.iter().position(|x| x.id == b.id) else { continue };
                let before: Vec<&String> =
                    proposed.chain.blocks.iter().take_while(|x| x.id != b.id).map(|x| &x.id).collect();
                let at = m.chain.blocks.iter().rposition(|x| before.contains(&&x.id)).map_or(0, |i| i + 1);
                let block = m.chain.blocks.remove(from);
                let at = if from < at { at - 1 } else { at };
                m.chain.blocks.insert(at.min(m.chain.blocks.len()), block);
            }
        }
    }
    if take("sections") {
        m.sections = proposed
            .sections
            .iter()
            .cloned()
            .map(|mut s| {
                s.eq.retain(|e| m.chain.block(&e.block).is_some());
                s
            })
            .collect();
    }
    // Offsets of EQs that are no longer what they were must still fit their bands.
    for s in &mut m.sections {
        s.eq.retain(|e| matches!(m.chain.block(&e.block).map(|b| &b.block), Some(Block::Eq(eq)) if eq.bands.len() == e.gains_db.len()));
    }
    m
}

#[derive(Debug, Clone)]
pub struct MasterProposal {
    pub call_id: String,
    pub plan: MasterPlan,
    pub commands: Vec<Command>,
    pub diff: TrackDiff,
    /// The same changes entry by entry, each one appliable on its own (AI-2).
    pub changes: Vec<MasterChange>,
    pub summary: String,
    pub record: AiCallRecord,
    pub preview: MasterState,
    /// The proposed master as rendered and measured in the call; `None` for a proposal opened again (AI-23).
    pub evaluation: Option<MasterEvaluation>,
}

/// The master chain as a new project has it: a request on it asks for a whole master, not for a change.
fn untouched(m: &MasterState) -> bool {
    m.chain == MasterState::default().chain
}

/// Master AI or master revision (with `request`), as a loop (MAS-6): every plan is rendered at the destination's
/// loudness and measured, and the next decision sees what it did (how hard the limiter works, how much punch is
/// lost). At most [`MASTER_ITERATIONS`] plans; the loop stops when the model confirms the plan it has just seen
/// measured. The plan kept is the last one.
#[allow(clippy::too_many_arguments)]
pub fn run_master(
    package: &Path,
    project: &Project,
    inp: &MasterInput<'_>,
    decider: &dyn Decider,
    estimate: &(dyn Fn(u64, u64) -> f64 + Sync),
    request: Option<String>,
    progress: &(dyn Fn(AiProgress) + Sync),
    cancel: &AtomicBool,
) -> Result<MasterProposal, AiError> {
    let kind = if request.is_some() { AiCallKind::MasterRevision } else { AiCallKind::Master };
    let id = log::new_id(kind);
    let started = stemwise_core::package::now_ms();
    let mut meter = Meter::default();
    let fail = |message: String, outcome: AiOutcome, meter: &Meter, its: u32| {
        let record = log::record(
            &id,
            kind,
            started,
            &decider.model(),
            &decider.effort(),
            decider.name(),
            meter,
            its,
            outcome,
            request.clone(),
            Some(message.clone()),
        );
        log::write(package, &id, "record.json", &serde_json::to_string_pretty(&record).unwrap_or_default());
        AiError::Failed { record: Box::new(record), message }
    };
    let cancelled = || cancel.load(Ordering::Relaxed);
    let of = MASTER_ITERATIONS;
    progress(AiProgress { phase: AiPhase::Analysis, iteration: 0, of, fraction: 0.0 });
    let mix = inp.dynamics();
    // The master as it is now, measured: the model starts from what the current chain does to this mixdown.
    let current = evaluate_master(inp, &mix, project, &project.master, inp.start_gain_db);
    if cancelled() {
        return Err(fail("cancelled".into(), AiOutcome::Cancelled, &meter, 0));
    }
    let doc = master_document(inp, &mix, current.as_ref(), project);
    log::write(package, &id, "analysis.json", &doc.to_string());
    // A request on a master nobody touched yet asks for a whole master; on a worked one, for a change to it.
    let revision = request.is_some() && !untouched(&project.master);
    let ids = Ids::of(project);
    let section_ids: Vec<String> =
        sections_doc(inp, project, &ids).iter().filter_map(|s| s["id"].as_str().map(String::from)).collect();
    let schema = master_plan_schema(&section_ids);
    let rate = project.sample_rate.unwrap_or(inp.rate);
    let mut history: Vec<(Value, Value)> = Vec::new();
    let mut kept: Option<(MasterPlan, Option<MasterEvaluation>)> = None;
    let mut gain = current.as_ref().map(|e| e.loudness_gain_db);
    let mut done = 0u32;
    for it in 0..of {
        if cancelled() {
            return Err(fail("cancelled".into(), AiOutcome::Cancelled, &meter, done));
        }
        progress(AiProgress { phase: AiPhase::Decision, iteration: it + 1, of, fraction: 0.0 });
        let input = DecisionInput {
            kind: Kind::Master,
            iteration: it + 1,
            allow_structure: false,
            allow_ideas: false,
            allow_return_blocks: false,
            brief: project.brief.clone(),
            analysis: doc.clone(),
            project: project.clone(),
            ids: ids.clone(),
            history: history.clone(),
            start: revision.then(|| serde_json::to_value(plan_from_master(&project.master, &ids)).unwrap_or_default()),
            request: request.clone(),
            editable: Vec::new(),
        };
        let messages = crate::prompt::build_messages(&input);
        let mut retry: Option<Retry> = None;
        let mut plan = None;
        for attempt in 0..=MAX_RETRIES {
            if cancelled() {
                return Err(fail("cancelled".into(), AiOutcome::Cancelled, &meter, done));
            }
            let raw = match decider.decide(&input, &messages, &schema, retry.as_ref(), cancel) {
                Ok(r) => r,
                Err(e) => {
                    // A failed or abandoned call counts with what it may have spent (AI-15, A-031).
                    if let Some(usage) = e.usage() {
                        meter.add(usage, estimate);
                    } else if e.maybe_billed() {
                        meter.add(&prompt_estimate(&messages), estimate);
                    }
                    log::write(package, &id, &format!("request-{it}-{attempt}.json"), &e.log_entry(&messages));
                    if let Some(body) = e.last_response() {
                        log::write(package, &id, &format!("response-{it}-{attempt}.json"), body);
                    }
                    let outcome = if matches!(e, DecideError::Cancelled | DecideError::Abandoned) {
                        AiOutcome::Cancelled
                    } else {
                        AiOutcome::Failed
                    };
                    return Err(fail(e.to_string(), outcome, &meter, done));
                }
            };
            meter.add(&raw.usage, estimate);
            log::write(package, &id, &format!("request-{it}-{attempt}.json"), &raw.request);
            log::write(package, &id, &format!("response-{it}-{attempt}.json"), &raw.raw);
            let errors = match serde_json::from_str::<MasterPlan>(&raw.content) {
                Err(e) => format!("The response is not a valid master plan JSON ({e})."),
                Ok(p) => {
                    let r = validate_master(&p, rate, &section_ids);
                    if r.ok() {
                        plan = Some(p);
                        break;
                    }
                    r.feedback()
                }
            };
            retry = Some(Retry { previous: raw.content.clone(), errors });
        }
        let Some(mut plan) = plan else {
            // A later iteration without a valid plan: the one already measured stays (as in the mix, D-055).
            if kept.is_some() {
                break;
            }
            return Err(fail(
                format!(
                    "the model did not return a valid master plan: {}",
                    retry.map(|r| r.errors).unwrap_or_default()
                ),
                AiOutcome::Failed,
                &meter,
                done,
            ));
        };
        // The destination is the author's choice (A-035): only a request may change the target.
        if request.is_none() {
            plan.target =
                MasterTarget { lufs: project.master.target.lufs, true_peak_db: project.master.target.true_peak_db };
        }
        log::write(package, &id, &format!("plan-{it}.json"), &serde_json::to_string_pretty(&plan).unwrap_or_default());
        // The model confirms the plan it has just seen measured: nothing new to render.
        if let Some((last, eval)) = &kept
            && same_settings(last, &plan)
        {
            kept = Some((plan, eval.clone()));
            done = it + 1;
            break;
        }
        progress(AiProgress { phase: AiPhase::Render, iteration: it + 1, of, fraction: 0.0 });
        let eval = evaluate_master(inp, &mix, project, &master_from_plan(&project.master, &plan, &ids), gain);
        if cancelled() {
            return Err(fail("cancelled".into(), AiOutcome::Cancelled, &meter, done));
        }
        let ej = serde_json::to_value(&eval).unwrap_or_default();
        log::write(package, &id, &format!("evaluation-{it}.json"), &ej.to_string());
        gain = eval.as_ref().map(|e| e.loudness_gain_db).or(gain);
        history.push((serde_json::to_value(&plan).unwrap_or_default(), ej));
        kept = Some((plan, eval));
        done = it + 1;
    }
    let Some((plan, evaluation)) = kept else {
        return Err(fail("the model did not return a master plan".into(), AiOutcome::Failed, &meter, done));
    };
    // AI-23: the plan kept is the proposal; it can be reviewed and applied again later.
    log::write(package, &id, "plan.json", &serde_json::to_string_pretty(&plan).unwrap_or_default());
    let mut record = log::record(
        &id,
        kind,
        started,
        &decider.model(),
        &decider.effort(),
        decider.name(),
        &meter,
        done,
        AiOutcome::Pending,
        request,
        None,
    );
    record.stored = true;
    log::write(package, &id, "record.json", &serde_json::to_string_pretty(&record).unwrap_or_default());
    Ok(proposal_of(project, plan, record, evaluation))
}

/// Two plans that set the chain the same way (their words may differ).
fn same_settings(a: &MasterPlan, b: &MasterPlan) -> bool {
    let strip = |p: &MasterPlan| MasterPlan { summary: String::new(), rationale: Default::default(), ..p.clone() };
    strip(a) == strip(b)
}

fn proposal_of(
    project: &Project,
    plan: MasterPlan,
    record: AiCallRecord,
    evaluation: Option<MasterEvaluation>,
) -> MasterProposal {
    let master = &project.master;
    let preview = master_from_plan(master, &plan, &Ids::of(project));
    MasterProposal {
        call_id: record.id.clone(),
        diff: master_diff(master, &preview, &plan),
        changes: master_changes(master, &preview, &plan),
        summary: plan.summary.clone(),
        commands: vec![Command::SetMaster { master: Box::new(preview.clone()) }],
        plan,
        record,
        preview,
        evaluation,
    }
}

/// A plan of version 1 (one fixed chain) as a plan of version 2, with the ids a migrated project gives its blocks.
fn plan_v1(v: &Value) -> Option<MasterPlan> {
    let f = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64);
    let mut chain = Vec::new();
    let why = |k: &str| v["rationale"][k].as_str().unwrap_or_default().to_string();
    if let Some(bands) = v["eq"].as_array().filter(|b| !b.is_empty()) {
        let bands = serde_json::from_value(Value::Array(bands.clone())).ok()?;
        chain.push(MasterBlockPlan::Eq { id: "b1".into(), bands, why: why("eq") });
    }
    if let Some(a) = f(&v["match"], "amount") {
        chain.push(MasterBlockPlan::MatchEq { id: "b2".into(), amount: a, why: why("match") });
    }
    let c = &v["compressor"];
    if c.is_object() {
        chain.push(MasterBlockPlan::Compressor {
            id: "b3".into(),
            threshold_db: f(c, "threshold_db")?,
            ratio: f(c, "ratio")?,
            attack_ms: f(c, "attack_ms")?,
            release_ms: f(c, "release_ms")?,
            knee_db: f(c, "knee_db")?,
            makeup_db: f(c, "makeup_db")?,
            mix: 1.0,
            listen_above_hz: 0.0,
            why: why("compressor"),
        });
    }
    let w = &v["width"];
    if w.is_object() {
        chain.push(MasterBlockPlan::Width {
            id: "b5".into(),
            side_gain_db: f(w, "side_gain_db")?,
            mono_below_hz: f(w, "mono_below_hz")?,
            why: why("width"),
        });
    }
    let x = &v["saturation"];
    if x.is_object() {
        chain.push(MasterBlockPlan::Saturation {
            id: "b6".into(),
            drive_db: f(x, "drive_db")?,
            mix: f(x, "mix")?,
            why: why("saturation"),
        });
    }
    let mut rationale = std::collections::BTreeMap::new();
    if let Some(l) = v["rationale"]["limiter"].as_str() {
        rationale.insert("limiter".to_string(), l.to_string());
    }
    Some(MasterPlan {
        version: MASTER_PLAN_VERSION,
        summary: v["summary"].as_str()?.to_string(),
        target: MasterTarget { lufs: f(&v["target"], "lufs")?, true_peak_db: f(&v["target"], "true_peak_db")? },
        input_gain_db: f(v, "input_gain_db")?,
        chain,
        limiter: LimiterPlan { release_ms: f(&v["limiter"], "release_ms")? },
        sections: Vec::new(),
        rationale,
    })
}

/// AI-23: a master proposal received earlier, ready for review on the master as it is now: the plan the call kept
/// (`plan.json`; `plan-0.json` for calls made before the master loop), with the diff against today's settings. Plans
/// of the first version of the contract are read as what they meant.
pub fn restore_master(package: &Path, project: &Project, record: &AiCallRecord) -> Result<MasterProposal, String> {
    let gone = || "this proposal was not kept with the project: it cannot be reviewed again".to_string();
    let dir = package.join(&record.path);
    let text = std::fs::read_to_string(dir.join("plan.json"))
        .or_else(|_| std::fs::read_to_string(dir.join("plan-0.json")))
        .map_err(|_| gone())?;
    let v: Value = serde_json::from_str(&text).map_err(|_| gone())?;
    let plan = if v["version"] == 1 { plan_v1(&v) } else { serde_json::from_value(v).ok() }.ok_or_else(gone)?;
    Ok(proposal_of(project, plan, record.clone(), None))
}

/// Measures a mixdown (interleaved stereo) for the master decision.
pub fn measure_mixdown(samples: &[f32], rate: u32) -> MixMeasures {
    measures::measure_mix(samples, 2, rate).0
}
