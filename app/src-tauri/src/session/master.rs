//! The master phase in the app (F8): the explicit passage through a bounce (MDN-1 … MDN-4), the master chain on a
//! second graph that plays the frozen mixdown (FOUNDATION §5.4), loudness normalised to the destination in the
//! background after every change (MAS-2), equal-loudness A/B against the mixdown and the references (MAS-5), the
//! export (MAS-7 … MAS-10) and the master AI with its result measured before applying (MAS-6, A-033).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use stemwise_ai::decider::{Decider, FakeDecider};
use stemwise_ai::mastering::MasterProposal;
use stemwise_ai::openrouter::{AiConfig, OpenRouter};
use stemwise_analysis::evaluate::spectral_distance;
use stemwise_analysis::measures::{MixMeasures, Target, measure_mix, measure_reference, target_profile};
use stemwise_core::command::Command;
use stemwise_core::history::Phase;
use stemwise_core::model::{AiCallKind, AiOutcome, Block, MasterState, Mixdown, Project, Section};
use stemwise_core::{media, package};
use stemwise_dsp::{loudness, truepeak};
use stemwise_engine::master::MasterParams;
use stemwise_engine::mixdown::{is_stale, listen_project};
use stemwise_export::dto::{
    MasterJob, MasterMeasures, MasterPrep, MasterProposalView, MasterRetarget, MasterStatus, PrepError, PrepStep,
    ReferenceView,
};
use stemwise_export::pipeline::{FileReport, render_master_from};

use super::{Res, Session, err};

/// The mixdown in memory and what the master needs from it and from the references.
pub struct MasterCache {
    pub number: u32,
    pub rate: u32,
    /// Interleaved stereo.
    pub samples: Vec<f32>,
    pub measures: MixMeasures,
    /// The mixdown's measures as the master workspace shows them (computed once).
    pub view: MasterMeasures,
    pub target: Option<Target>,
    pub references: Vec<RefCache>,
    /// Ids of the project's references when the cache was made: adding or removing one makes it stale.
    pub ref_ids: Vec<String>,
    /// References that could not be read: left out (9i).
    pub skipped: Vec<String>,
}

/// A reference ready for the A/B (MAS-5): converted once to the project's rate as stereo float in `cache/` (the
/// imported file stays as it is), with its loudness and true peak.
pub struct RefCache {
    pub id: String,
    pub name: String,
    /// Converted copy, inside the package.
    pub listen: String,
    pub lufs: Option<f64>,
    pub true_peak_db: f64,
    pub frames: u64,
}

/// Highest true peak an A/B source may reach on the output: equal loudness never clips the device (S7).
pub const AB_PEAK_DB: f64 = -1.0;

pub struct Job {
    pub view: MasterJob,
    pub cancel: Arc<AtomicBool>,
    /// The project it works for ([`Session::open_id`]).
    pub owner: u64,
}

pub struct MasterSide {
    pub phase: Phase,
    pub cache: Option<Arc<MasterCache>>,
    pub loud_gain_db: f64,
    /// Generation of the loudness computation: a result for an older state is dropped.
    pub epoch: u64,
    pub analysing: bool,
    /// A loudness thread is running (at most one; it loops while the epoch moves).
    pub loud_running: bool,
    pub measures: Option<MasterMeasures>,
    pub listening: String,
    /// Every source at the master's loudness (MAS-5, on demand); otherwise at its real level (design 9l). Not saved:
    /// each opened project starts at the real level, and the choice holds across phases while it is open.
    pub listen_same: bool,
    pub job: Option<Job>,
    pub bounced: Option<Mixdown>,
    pub last_export: Option<Vec<FileReport>>,
    pub pending: Option<Pending>,
    /// The samples of the bounce that just wrote the current mixdown (its number): the cache takes them instead of
    /// reading the file back.
    pub bounced_samples: Option<(u32, Vec<f32>)>,
    /// The master being prepared (overlay 9d–9j).
    pub prep: Option<Prep>,
    /// The destination (loudness, ceiling) the current measures were made for.
    pub measured_target: Option<(f64, f64)>,
    /// The master being brought to a new destination (overlay 9v).
    pub retarget: Option<Retarget>,
    /// Bands of a multiband heard alone (processor, a bit per band): listening only, never saved.
    pub solo: Option<(u8, u8)>,
    /// The envelope of the master as last measured, for its waveform.
    pub wave: Option<Arc<Wave>>,
}

/// A destination change whose loudness search is still running.
pub struct Retarget {
    /// The destination before the change (still heard) and the one being reached.
    from: (f64, f64),
    to: (f64, f64),
    started: std::time::Instant,
    pub fraction: f64,
}

/// A step of the preparation, as the work reports it.
#[derive(Debug, Clone, PartialEq)]
pub enum PrepKey {
    Read,
    MeasureMix,
    /// A reference converted to the project rate (its id).
    Convert(String),
    MeasureRefs,
    Loudness,
}

/// What happened to a step.
pub enum PrepEv {
    Start,
    Progress(f64),
    /// Done; with where the result came from when nothing had to be done ("bounce", "saved").
    Done(Option<&'static str>),
    /// Left out, with why (a reference that cannot be read).
    Skip(String),
}

struct PrepRun {
    key: PrepKey,
    step: PrepStep,
    /// Expected share of the work (relative).
    weight: f64,
    started: Option<std::time::Instant>,
}

/// The preparation in progress (or failed): its steps and when it started.
pub struct Prep {
    mixdown: u32,
    frames: u64,
    rate: u32,
    references: u32,
    started: std::time::Instant,
    steps: Vec<PrepRun>,
    error: Option<PrepError>,
}

/// Expected work of each step, relative (measured on a 3-minute song at 96 kHz: reading 0.1 s, measuring 2 s,
/// converting a reference 5 s, the loudness search 6–11 s).
const W_READ: f64 = 0.1;
const W_MEASURE: f64 = 2.0;
const W_CONVERT: f64 = 5.0;
const W_LOUDNESS: f64 = 8.0;

fn prep_step(kind: &str) -> PrepStep {
    PrepStep {
        kind: kind.into(),
        state: "waiting".into(),
        source: None,
        fraction: 0.0,
        ms: None,
        references: Vec::new(),
        from_rate: None,
        detail: None,
    }
}

impl Prep {
    /// The steps the cache of this project's mixdown needs: what is already saved in the package is listed as done
    /// at once; a conversion that is not needed is not listed (design 9d–9e).
    pub fn plan(package: &std::path::Path, project: &Project, bounced: bool) -> Option<Prep> {
        let md = project.master.mixdown.as_ref()?;
        let rate = project.sample_rate?;
        let mut steps = Vec::new();
        let mut read = prep_step("read_mixdown");
        if bounced {
            read.state = "done".into();
            read.source = Some("bounce".into());
            read.fraction = 1.0;
        }
        steps.push(PrepRun {
            key: PrepKey::Read,
            weight: if bounced { 0.0 } else { W_READ },
            step: read,
            started: None,
        });
        let mut mm = prep_step("measure_mixdown");
        let saved = mixdown_saved(package, md, rate).is_some();
        if saved {
            mm.state = "done".into();
            mm.source = Some("saved".into());
            mm.fraction = 1.0;
        }
        steps.push(PrepRun {
            key: PrepKey::MeasureMix,
            weight: if saved { 0.0 } else { W_MEASURE },
            step: mm,
            started: None,
        });
        let mut all_saved = true;
        for r in &project.references {
            let copy = package.join(format!("cache/listen-{}-{rate}.wav", r.id)).is_file();
            all_saved &= copy && reference_saved(package, r, rate).is_some();
            if !copy && r.media.sample_rate != rate {
                let mut c = prep_step("convert_reference");
                c.references = vec![r.media.original_name.clone()];
                c.from_rate = Some(r.media.sample_rate);
                steps.push(PrepRun { key: PrepKey::Convert(r.id.clone()), weight: W_CONVERT, step: c, started: None });
            }
        }
        if !project.references.is_empty() {
            let mut m = prep_step("measure_references");
            m.references = project.references.iter().map(|r| r.media.original_name.clone()).collect();
            if all_saved {
                m.state = "done".into();
                m.source = Some("saved".into());
                m.fraction = 1.0;
            }
            let w = if all_saved { 0.0 } else { W_MEASURE * project.references.len() as f64 };
            steps.push(PrepRun { key: PrepKey::MeasureRefs, weight: w, step: m, started: None });
        }
        steps.push(PrepRun { key: PrepKey::Loudness, weight: W_LOUDNESS, step: prep_step("loudness"), started: None });
        Some(Prep {
            mixdown: md.number,
            frames: md.frames,
            rate,
            references: project.references.len() as u32,
            started: std::time::Instant::now(),
            steps,
            error: None,
        })
    }

    pub fn event(&mut self, key: &PrepKey, ev: PrepEv) {
        let Some(run) = self.steps.iter_mut().find(|r| &r.key == key) else { return };
        let st = &mut run.step;
        match ev {
            PrepEv::Start => {
                if st.state == "waiting" {
                    st.state = "running".into();
                    run.started = Some(std::time::Instant::now());
                }
            }
            PrepEv::Progress(f) => st.fraction = f.clamp(0.0, 1.0),
            PrepEv::Done(source) => {
                if st.state == "done" {
                    return;
                }
                st.state = "done".into();
                st.fraction = 1.0;
                st.source = source.map(str::to_string);
                if source.is_some() {
                    // Nothing was done: it weighs nothing (the bar does not wait for it).
                    run.weight = 0.0;
                } else {
                    st.ms = run.started.map(|t| t.elapsed().as_millis() as u64);
                }
            }
            PrepEv::Skip(why) => {
                st.state = "skipped".into();
                st.fraction = 1.0;
                st.detail = Some(why);
            }
        }
    }

    pub fn fail(&mut self, e: PrepError) {
        for r in &mut self.steps {
            if r.step.kind == e.step && r.step.state != "done" {
                r.step.state = "failed".into();
                r.step.detail = Some(e.detail.clone());
            } else if r.step.state == "waiting" || r.step.state == "running" {
                r.step.state = "not_started".into();
            }
        }
        self.error = Some(e);
    }

    pub fn failed(&self) -> bool {
        self.error.is_some()
    }

    fn view(&self) -> MasterPrep {
        let total: f64 = self.steps.iter().map(|r| r.weight).sum();
        let done: f64 = self.steps.iter().map(|r| r.weight * r.step.fraction).sum();
        let fraction = if total > 0.0 { done / total } else { 1.0 };
        let elapsed = self.started.elapsed().as_millis() as u64;
        // A guess only after a step that had real work: before, the time per unit of work is unknown.
        let measured = self.steps.iter().any(|r| r.step.ms.is_some());
        let eta = (measured && fraction > 0.0 && self.error.is_none())
            .then(|| (elapsed as f64 * (1.0 - fraction) / fraction) as u64);
        MasterPrep {
            mixdown: self.mixdown,
            frames: self.frames,
            rate: self.rate,
            references: self.references,
            fraction,
            elapsed_ms: elapsed,
            eta_ms: eta,
            error: self.error.clone(),
            steps: self.steps.iter().map(|r| r.step.clone()).collect(),
        }
    }
}

/// A master proposal in review, measured at its own loudness (A-033).
pub struct Pending {
    pub proposal: MasterProposal,
    pub after: Option<MasterMeasures>,
    /// Loudness gain of the proposed state (its A/B plays at it, S7).
    pub loud_gain_db: Option<f64>,
    pub request: Option<String>,
    /// A proposal received earlier and opened again for review (AI-23): closing it leaves its record as it is.
    pub reopened: bool,
}

impl Default for MasterSide {
    fn default() -> Self {
        MasterSide {
            phase: Phase::Mix,
            cache: None,
            loud_gain_db: 0.0,
            epoch: 0,
            analysing: false,
            loud_running: false,
            measures: None,
            listening: "master".into(),
            listen_same: false,
            job: None,
            bounced: None,
            last_export: None,
            pending: None,
            bounced_samples: None,
            prep: None,
            measured_target: None,
            retarget: None,
            solo: None,
            wave: None,
        }
    }
}

/// Share of the song with the limiter over 3 dB past which the master says the target asks too much (15g).
const STRAIN_PCT: f64 = 5.0;

/// Whether the limiter works too hard at this loudness, and a loudness at which it would not: lower by what its
/// deepest reduction exceeds 3 dB by, to the half dB.
fn strain(m: &MasterMeasures, lufs: f64) -> Option<stemwise_export::dto::Strain> {
    let l = m.limiter.as_ref().filter(|l| l.over_3_pct > STRAIN_PCT)?;
    let down = (l.max_db - 3.0).clamp(1.0, 8.0);
    Some(stemwise_export::dto::Strain { over_3_pct: l.over_3_pct, easier_lufs: ((lufs - down) * 2.0).floor() / 2.0 })
}

fn mm(x: &[f32], rate: u32, target: Option<&Target>) -> MasterMeasures {
    let m = measure_mix(x, 2, rate).0;
    let d = stemwise_ai::mastering::dynamics(x, rate, m.lufs, m.true_peak_db);
    MasterMeasures {
        peak_to_loudness_db: d.peak_to_loudness_db,
        short_crest_db: d.short_crest_db,
        ..view_of(&m, target)
    }
}

/// A rendered master measured, with how hard its dynamics blocks work (one more pass through the chain).
fn measured(
    c: &MasterCache,
    m: &MasterState,
    sections: &[Section],
    r: &stemwise_export::pipeline::Rendered,
) -> MasterMeasures {
    use stemwise_export::dto::{BlockWork, Work};
    let tb = c.target.as_ref().and_then(|t| t.bands_db.as_ref());
    let mut p = MasterParams::of(m, c.measures.bands_db.as_ref(), tb, r.gain_db).with_sections(m, sections);
    p.limiter_on = true;
    p.clip_peak = r.clip_peak;
    let (l, rr): (Vec<f32>, Vec<f32>) = c.samples.as_chunks::<2>().0.iter().map(|x| (x[0], x[1])).unzip();
    let (_, gr) = stemwise_engine::master::render_metered(&l, &rr, f64::from(c.rate), p);
    let work = |g: stemwise_engine::master::GrStats| Work {
        max_db: g.max_db,
        mean_db: g.mean_db,
        active_pct: g.active * 100.0,
        over_3_pct: g.over_3 * 100.0,
    };
    MasterMeasures {
        limiter: Some(work(gr.limiter)),
        blocks: m
            .chain
            .blocks
            .iter()
            .zip(gr.blocks)
            .filter(|(b, _)| {
                !b.block.bypass() && matches!(b.block, Block::Compressor(_) | Block::Multiband(_) | Block::Clipper(_))
            })
            .map(|(b, g)| BlockWork { id: b.id.clone(), work: work(g) })
            .collect(),
        clip_peak: r.clip_peak,
        ..mm(&r.samples, c.rate, c.target.as_ref())
    }
}

/// The workspace's view of a mix's measures: `measure_mix` already has the loudness, the true peak and the range.
fn view_of(m: &MixMeasures, target: Option<&Target>) -> MasterMeasures {
    MasterMeasures {
        lufs: m.lufs,
        true_peak_db: if m.true_peak_db.is_finite() { m.true_peak_db } else { -200.0 },
        lra: m.dyn_range_lu,
        distance_db: match (m.bands_db.as_ref(), target.and_then(|t| t.bands_db.as_ref())) {
            (Some(b), Some(t)) => Some(spectral_distance(b, t).0),
            _ => None,
        },
        bands_db: m.bands_db,
        peak_to_loudness_db: m.lufs.filter(|_| m.true_peak_db.is_finite()).map(|l| m.true_peak_db - l),
        short_crest_db: None,
        limiter: None,
        blocks: Vec::new(),
        clip_peak: [0.0; 2],
    }
}

// ---------------------------------------------------------------------------------------------
// What the master measured, kept in the package (`cache/master/`): a project reopened with its master already made
// shows it at once. Each file names what it was measured from; a file that does not match, or cannot be read, is
// measured again. Derived data only: deleting the folder costs one more wait.
// ---------------------------------------------------------------------------------------------

const CACHE_VERSION: u32 = 1;

#[derive(serde::Serialize, serde::Deserialize)]
struct MixdownFile {
    version: u32,
    number: u32,
    mix_hash: String,
    frames: u64,
    rate: u32,
    measures: MixMeasures,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RefFile {
    version: u32,
    media: String,
    rate: u32,
    frames: u64,
    lufs: Option<f64>,
    true_peak_db: f64,
    measures: Option<MixMeasures>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct LoudFile {
    version: u32,
    /// The latest results, newest last: (what was measured, its gain, its measures, its envelope).
    entries: Vec<(String, f64, MasterMeasures, Wave)>,
}

/// Version of the loudness file: 2 since it keeps the envelope of the master.
const LOUD_VERSION: u32 = 2;

/// The envelope of a signal for its waveform: (lowest sample, highest sample, RMS) per column, both channels
/// together. The RMS is the body of the sound: it keeps its shape where the peaks are all at the ceiling.
pub type Wave = Vec<[f32; 3]>;

/// Columns of a kept envelope: enough for the widest waveform the workspace draws.
const WAVE_COLUMNS: usize = 2048;

/// The envelope of an interleaved stereo signal in `columns` columns.
pub fn envelope(x: &[f32], columns: usize) -> Wave {
    let frames = x.len() / 2;
    if frames == 0 {
        return Vec::new();
    }
    let cols = columns.clamp(16, 4096).min(frames);
    (0..cols)
        .map(|k| {
            let (a, b) = (k * frames / cols, ((k + 1) * frames / cols).max(k * frames / cols + 1).min(frames));
            let s = &x[2 * a..2 * b];
            let (lo, hi, sq) =
                s.iter().fold((0f32, 0f32, 0f64), |m, v| (m.0.min(*v), m.1.max(*v), m.2 + f64::from(*v).powi(2)));
            [lo, hi, (sq / s.len() as f64).sqrt() as f32]
        })
        .collect()
}

/// An envelope drawn in fewer columns: the extremes of the columns it joins, and the RMS of their RMS.
fn narrower(w: &Wave, columns: usize) -> Wave {
    let n = w.len();
    let cols = columns.clamp(16, 4096);
    if n <= cols {
        return w.clone();
    }
    (0..cols)
        .map(|k| {
            let s = &w[k * n / cols..((k + 1) * n / cols).max(k * n / cols + 1).min(n)];
            let sq = s.iter().map(|c| f64::from(c[2]).powi(2)).sum::<f64>() / s.len() as f64;
            [s.iter().fold(0f32, |m, c| m.min(c[0])), s.iter().fold(0f32, |m, c| m.max(c[1])), sq.sqrt() as f32]
        })
        .collect()
}

pub use stemwise_export::dto::MasterWave;

/// Results kept for the loudness (a few states: undo, A/B on proposals).
const LOUD_KEEP: usize = 16;

fn read_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Written next to the target and renamed, so a crash never leaves half a file.
fn write_json<T: serde::Serialize>(path: &std::path::Path, v: &T) {
    let Ok(text) = serde_json::to_vec(v) else { return };
    let tmp = path.with_extension("json.tmp");
    if let Some(dir) = path.parent()
        && std::fs::create_dir_all(dir).is_ok()
        && std::fs::write(&tmp, text).is_ok()
    {
        let _ = std::fs::rename(&tmp, path);
    }
}

/// What a loudness result depends on: the chain, the destination, the mixdown and the two band profiles.
pub fn loudness_key(c: &MasterCache, m: &MasterState, sections: &[Section], mix_hash: &str) -> String {
    let tb = c.target.as_ref().and_then(|t| t.bands_db.as_ref());
    // The sections count only where the master offsets them.
    let offsets: Vec<(&stemwise_core::model::SectionOffsets, i64, i64)> = m
        .sections
        .iter()
        .filter_map(|o| sections.iter().find(|s| s.id == o.section).map(|s| (o, s.start, s.end)))
        .collect();
    serde_json::to_string(&(&m.chain, &m.target, c.number, mix_hash, c.measures.bands_db.as_ref(), tb, offsets))
        .unwrap_or_default()
}

/// A loudness result kept for this state, if any.
pub fn loudness_cached(package: &std::path::Path, key: &str) -> Option<(f64, MasterMeasures, Wave)> {
    let f: LoudFile = read_json(&package.join("cache/master/loudness.json"))?;
    if f.version != LOUD_VERSION {
        return None;
    }
    f.entries.into_iter().rev().find(|(k, _, _, _)| k == key).map(|(_, g, m, w)| (g, m, w))
}

pub fn loudness_keep(package: &std::path::Path, key: String, gain: f64, measures: &MasterMeasures, wave: &Wave) {
    let path = package.join("cache/master/loudness.json");
    let mut f: LoudFile = read_json(&path).filter(|f: &LoudFile| f.version == LOUD_VERSION).unwrap_or_default();
    f.version = LOUD_VERSION;
    f.entries.retain(|(k, _, _, _)| *k != key);
    f.entries.push((key, gain, measures.clone(), wave.clone()));
    let n = f.entries.len();
    if n > LOUD_KEEP {
        f.entries.drain(..n - LOUD_KEEP);
    }
    write_json(&path, &f);
}

/// The mixdown's measures kept in the package, if they are for this very mixdown.
fn mixdown_saved(package: &std::path::Path, md: &Mixdown, rate: u32) -> Option<MixMeasures> {
    let f = read_json::<MixdownFile>(&package.join(format!("cache/master/mixdown-{}.json", md.number)))?;
    (f.version == CACHE_VERSION
        && f.number == md.number
        && f.mix_hash == md.mix_hash
        && f.frames == md.frames
        && f.rate == rate)
        .then_some(f.measures)
}

fn reference_saved(package: &std::path::Path, r: &stemwise_core::model::ReferenceTrack, rate: u32) -> Option<RefFile> {
    read_json::<RefFile>(&package.join(format!("cache/master/ref-{}-{rate}.json", r.media.id)))
        .filter(|f| f.version == CACHE_VERSION && f.media == r.media.id && f.rate == rate)
}

/// Why the mixdown file cannot be used, in the words of 9j.
fn unreadable(package: &std::path::Path, md: &Mixdown, rate: u32, e: Option<&str>, frames: Option<u64>) -> PrepError {
    let path = package.join(&md.path);
    let clock = |f: u64| {
        let s = f / u64::from(rate.max(1));
        format!("{}:{:02}", s / 60, s % 60)
    };
    let detail = if !path.is_file() {
        "The file is missing".to_string()
    } else if let Some(f) = frames {
        format!("The file ends early ({} of {})", clock(f), clock(md.frames))
    } else {
        format!("The file is not a WAV file we can read ({})", e.unwrap_or("unknown format"))
    };
    PrepError { kind: "mixdown_unreadable".into(), step: "read_mixdown".into(), detail }
}

fn internal(step: &str, e: impl std::fmt::Display) -> PrepError {
    PrepError { kind: "internal".into(), step: step.into(), detail: e.to_string() }
}

/// Loads the mixdown and what the master needs from it and from the references (off the lock). `bounced` are the
/// samples of the bounce that just wrote this mixdown, when there are: the file is not read again. Measures kept in
/// the package are reused; what is measured runs in parallel (the mixdown and each reference).
pub fn load_cache(
    package: &std::path::Path,
    project: &Project,
    bounced: Option<Vec<f32>>,
    rep: &(dyn Fn(PrepKey, PrepEv) + Sync),
) -> Result<MasterCache, PrepError> {
    use rayon::prelude::*;
    let md = project.master.mixdown.as_ref().ok_or_else(|| internal("read_mixdown", "no mixdown yet"))?;
    let rate = project.sample_rate.ok_or_else(|| internal("read_mixdown", "no sample rate"))?;
    rep(PrepKey::Read, PrepEv::Start);
    let samples = match bounced {
        Some(x) if x.len() as u64 == 2 * md.frames => {
            rep(PrepKey::Read, PrepEv::Done(Some("bounce")));
            x
        }
        _ => {
            let x = media::decode_all(&package.join(&md.path))
                .map_err(|e| unreadable(package, md, rate, Some(&e.to_string()), None))?
                .1;
            if (x.len() as u64) < 2 * md.frames {
                return Err(unreadable(package, md, rate, None, Some(x.len() as u64 / 2)));
            }
            rep(PrepKey::Read, PrepEv::Done(None));
            x
        }
    };
    let mix = || -> MixMeasures {
        if let Some(m) = mixdown_saved(package, md, rate) {
            rep(PrepKey::MeasureMix, PrepEv::Done(Some("saved")));
            return m;
        }
        rep(PrepKey::MeasureMix, PrepEv::Start);
        let m = measure_mix(&samples, 2, rate).0;
        let f = MixdownFile {
            version: CACHE_VERSION,
            number: md.number,
            mix_hash: md.mix_hash.clone(),
            frames: md.frames,
            rate,
            measures: m.clone(),
        };
        write_json(&package.join(format!("cache/master/mixdown-{}.json", md.number)), &f);
        rep(PrepKey::MeasureMix, PrepEv::Done(None));
        m
    };
    let refs = || -> Result<Vec<RefOutcome>, PrepError> {
        project.references.par_iter().map(|r| reference(package, r, rate, rep)).collect()
    };
    let (measures, refs) = rayon::join(mix, refs);
    let refs = refs?;
    if !project.references.is_empty() {
        let saved = refs.iter().all(|r| matches!(r, Ok((_, _, true))));
        rep(PrepKey::MeasureRefs, PrepEv::Done(saved.then_some("saved")));
    }
    let skipped: Vec<String> = refs.iter().filter_map(|r| r.as_ref().err().cloned()).collect();
    let refs: Vec<(RefCache, Option<MixMeasures>)> =
        refs.into_iter().filter_map(Result::ok).map(|(c, m, _)| (c, m)).collect();
    let ref_measures: Vec<MixMeasures> = refs.iter().filter_map(|(_, m)| m.clone()).collect();
    let target = target_profile(&ref_measures);
    let view = view_of(&measures, target.as_ref());
    let ref_ids = project.references.iter().map(|r| r.id.clone()).collect();
    let references = refs.into_iter().map(|(r, _)| r).collect();
    Ok(MasterCache { number: md.number, rate, samples, measures, view, target, references, ref_ids, skipped })
}

/// A reference read for the master, with its measures and whether they were saved; or its name when its file cannot
/// be decoded (it is left out, 9h, 9i).
type RefOutcome = Result<(RefCache, Option<MixMeasures>, bool), String>;

/// A reference ready for the A/B: its copy at the project rate (made once, then read back) and its measures (kept in
/// the package), and whether they were saved. `Err(name)` when its file cannot be decoded: it is left out (9h, 9i).
fn reference(
    package: &std::path::Path,
    r: &stemwise_core::model::ReferenceTrack,
    rate: u32,
    rep: &(dyn Fn(PrepKey, PrepEv) + Sync),
) -> Result<RefOutcome, PrepError> {
    let listen = format!("cache/listen-{}-{rate}.wav", r.id);
    let name = r.media.original_name.clone();
    let cache = |frames, lufs, true_peak_db| RefCache {
        id: r.id.clone(),
        name: name.clone(),
        listen: listen.clone(),
        lufs,
        true_peak_db,
        frames,
    };
    if let Some(f) = reference_saved(package, r, rate)
        && package.join(&listen).is_file()
    {
        return Ok(Ok((cache(f.frames, f.lufs, f.true_peak_db), f.measures, true)));
    }
    let convert = PrepKey::Convert(r.id.clone());
    // The converted copy, when an earlier session made it, saves the conversion (seconds for a song).
    let st: Vec<f32> = match media::decode_all(&package.join(&listen)) {
        Ok((info, x)) if info.sample_rate == rate && info.channels == 2 => x,
        _ => {
            rep(convert.clone(), PrepEv::Start);
            let Ok((info, x)) = media::decode_all(&package.join(&r.media.path)) else {
                rep(convert, PrepEv::Skip("The file is missing or damaged".into()));
                return Ok(Err(name));
            };
            let x = if info.sample_rate != rate {
                media::resample(&x, usize::from(info.channels), info.sample_rate, rate)
                    .map_err(|e| internal("convert_reference", e))?
            } else {
                x
            };
            let st: Vec<f32> = if info.channels == 1 { x.iter().flat_map(|v| [*v, *v]).collect() } else { x };
            std::fs::create_dir_all(package.join("cache")).map_err(|e| internal("convert_reference", e))?;
            stemwise_core::wav::write_f32(&package.join(&listen), 2, rate, &st)
                .map_err(|e| internal("convert_reference", e))?;
            rep(convert, PrepEv::Done(None));
            st
        }
    };
    rep(PrepKey::MeasureRefs, PrepEv::Start);
    let (measures, (lufs, tp)) = rayon::join(
        || measure_reference(&st, 2, rate),
        || rayon::join(|| loudness::integrated(&st, 2, rate), || truepeak::true_peak_db(&st, 2)),
    );
    let tp = if tp.is_finite() { tp } else { -200.0 };
    let frames = (st.len() / 2) as u64;
    write_json(
        &package.join(format!("cache/master/ref-{}-{rate}.json", r.media.id)),
        &RefFile {
            version: CACHE_VERSION,
            media: r.media.id.clone(),
            rate,
            frames,
            lufs,
            true_peak_db: tp,
            measures: measures.clone(),
        },
    );
    Ok(Ok((cache(frames, lufs, tp), measures, false)))
}

/// Whether a cache made for `c` still fits the project (same mixdown, same references).
pub fn cache_fits(c: &MasterCache, p: &Project) -> bool {
    p.master.mixdown.as_ref().map(|m| m.number) == Some(c.number)
        && c.ref_ids.iter().eq(p.references.iter().map(|r| &r.id))
}

impl Session {
    /// The master phase needs a new cache (a bounce, a reference added or removed).
    pub fn cache_needed(&self) -> bool {
        let Some(p) = self.doc.as_ref().map(|d| &d.project) else { return false };
        p.master.mixdown.is_some() && !self.master.cache.as_ref().is_some_and(|c| cache_fits(c, p))
    }

    /// The loudness the master plays at (its measure, or the destination before the first one).
    fn master_lufs(&self) -> f64 {
        let p = self.doc.as_ref().map(|d| &d.project.master);
        self.master.measures.as_ref().and_then(|m| m.lufs).or(p.map(|m| m.target.lufs)).unwrap_or(-14.0)
    }

    /// MAS-5 without clipping (S7): bringing the mixdown or a reference up to the master's loudness may push its
    /// peaks over [`AB_PEAK_DB`]; then every source of the A/B, the master included, plays lower by the same
    /// amount, so the comparison stays at equal loudness. 0 when nothing needs it.
    pub fn ab_trim_db(&self) -> f64 {
        let Some(c) = self.master.cache.as_ref() else { return 0.0 };
        let master = self.master_lufs();
        let over = |lufs: Option<f64>, tp: f64| lufs.map_or(0.0, |l| (master - l) + tp - AB_PEAK_DB).max(0.0);
        let mix = over(c.view.lufs, c.view.true_peak_db);
        c.references.iter().map(|r| over(r.lufs, r.true_peak_db)).fold(mix, f64::max)
    }

    fn master_params(&self) -> Option<MasterParams> {
        let p = self.doc.as_ref()?;
        let c = self.master.cache.as_ref();
        // A/B on a master proposal (9c): its state at its own loudness (found when the proposal was measured).
        let (state, loud) = match (&self.master.pending, self.master.listening.as_str()) {
            (Some(pend), "proposal") => (&pend.proposal.preview, pend.loud_gain_db.unwrap_or(self.master.loud_gain_db)),
            _ => (&p.project.master, self.master.loud_gain_db),
        };
        let mut q = MasterParams::of(
            state,
            c.and_then(|c| c.measures.bands_db.as_ref()),
            c.and_then(|c| c.target.as_ref()).and_then(|t| t.bands_db.as_ref()),
            loud,
        )
        .with_sections(state, &p.project.sections);
        // The clippers work from the peak the last measure found; until then they let everything through.
        q.clip_peak = match (&self.master.pending, self.master.listening.as_str()) {
            (Some(pend), "proposal") => pend.after.as_ref(),
            _ => self.master.measures.as_ref(),
        }
        .map_or([0.0; 2], |m| m.clip_peak);
        if let Some((inst, mask)) = self.master.solo {
            q = q.with_solo(inst, mask);
        }
        // At the real level (design 9l) every source plays as it is: the master exactly as the exported file.
        let same = self.master.listen_same;
        let trim = if same { self.ab_trim_db() } else { 0.0 };
        q.out_gain_db = -trim;
        if same && self.master.listening == "proposal" {
            // 9o: the proposal at the current master's loudness.
            let after = self.master.pending.as_ref().and_then(|p| p.after.as_ref()).and_then(|m| m.lufs);
            q.out_gain_db += after.map_or(0.0, |a| self.master_lufs() - a);
        }
        if self.master.listening == "mixdown" {
            // MAS-5: the mixdown at the master's loudness, lowered with everything else if it would clip.
            let master = self.master_lufs();
            let mix = c.and_then(|c| c.measures.lufs).unwrap_or(master);
            q.bypass = true;
            q.bypass_gain_db = if same { master - mix } else { 0.0 };
        }
        Some(q)
    }

    pub fn master_status(&self) -> MasterStatus {
        let p = self.doc.as_ref().map(|d| &d.project);
        let target = self.master.cache.as_ref().and_then(|c| c.target.as_ref());
        MasterStatus {
            phase: if self.master.phase == Phase::Master { "master".into() } else { "mix".into() },
            stale: p.is_some_and(is_stale),
            listening: self.master.listening.clone(),
            listen_same: self.master.listen_same,
            loud_gain_db: self.master.loud_gain_db,
            analysing: self.master.analysing,
            measures: self.master.measures.clone(),
            mixdown_measures: self.master.cache.as_ref().map(|c| c.view.clone()),
            target_bands: target.and_then(|t| t.bands_db),
            references: self
                .master
                .cache
                .as_ref()
                .map(|c| {
                    c.references
                        .iter()
                        .map(|r| ReferenceView {
                            id: r.id.clone(),
                            name: r.name.clone(),
                            lufs: r.lufs,
                            true_peak_db: r.true_peak_db,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            job: self.master.job.as_ref().map(|j| j.view.clone()),
            bounced: self.master.bounced.clone(),
            last_export: self.master.last_export.clone(),
            aac: stemwise_export::aac::aac_available(),
            ab_trim_db: self.ab_trim_db(),
            prep: self.master.prep.as_ref().map(Prep::view),
            retarget: self.master.retarget.as_ref().map(|r| MasterRetarget {
                from_lufs: r.from.0,
                from_true_peak_db: r.from.1,
                fraction: r.fraction,
                elapsed_ms: r.started.elapsed().as_millis() as u64,
            }),
            skipped_references: self.master.cache.as_ref().map(|c| c.skipped.clone()).unwrap_or_default(),
            strained: self.master.measures.as_ref().zip(p).and_then(|(m, p)| strain(m, p.master.target.lufs)),
            chain: p
                .map(|p| {
                    let c = &p.master.chain;
                    let inst = stemwise_engine::master::instances(&p.master);
                    c.blocks
                        .iter()
                        .map(|b| stemwise_export::dto::BlockView {
                            id: b.id.clone(),
                            name: c.name(&b.id).unwrap_or_default(),
                            kind: b.block.kind(),
                            inst: inst.iter().find(|(id, _)| *id == b.id).map_or(0, |(_, i)| *i),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            addable: p
                .map(|p| {
                    stemwise_core::model::BlockKind::ALL.into_iter().filter(|k| p.master.chain.can_add(*k)).collect()
                })
                .unwrap_or_default(),
            solo: self.master_solo_view().filter(|(id, _)| {
                matches!(p.and_then(|p| p.master.chain.block(id)).map(|b| &b.block), Some(Block::Multiband(_)))
            }),
            proposal: self.master.pending.as_ref().map(
                |Pending { proposal: p, after, request: req, reopened, .. }| MasterProposalView {
                    call_id: p.call_id.clone(),
                    summary: p.summary.clone(),
                    diff: p.diff.clone(),
                    record: p.record.clone(),
                    request: req.clone(),
                    changes: p.changes.clone(),
                    preview: p.preview.clone(),
                    before: self.master.measures.clone(),
                    after: after.clone(),
                    reopened: *reopened,
                },
            ),
        }
    }

    /// Loads the engine for the current phase and listening choice.
    pub fn master_reload(&mut self) -> Res<()> {
        let dir = self.dir()?.to_path_buf();
        let p = self.project()?.clone();
        if self.master.phase == Phase::Mix {
            if let Some(e) = &mut self.engine {
                e.load(&dir, &p).map_err(err)?;
            }
            return Ok(());
        }
        let md = p.master.mixdown.clone().ok_or("no mixdown yet")?;
        let rate = p.sample_rate.unwrap_or(48000);
        let params = self.master_params();
        let listen = if let Some(id) = self.master.listening.strip_prefix("reference:") {
            let c = self.master.cache.clone().ok_or("the references are still being measured")?;
            let r = c.references.iter().find(|r| r.id == id).ok_or("no such reference")?;
            // The converted copy (project rate, stereo float): at its real level, or at the master's loudness and under
            // the A/B trim (MAS-5).
            let gain = if self.master.listen_same {
                r.lufs.map_or(0.0, |l| self.master_lufs() - l) - self.ab_trim_db()
            } else {
                0.0
            };
            (listen_project(&p, &r.listen, r.frames, rate, gain), None)
        } else {
            (listen_project(&p, &md.path, md.frames, rate, 0.0), params)
        };
        if let Some(e) = &mut self.engine {
            e.load_with(&dir, &listen.0, listen.1).map_err(err)?;
        }
        Ok(())
    }

    /// Switches phase (UI-1). Master needs a mixdown (MDN-1): "no-mixdown" otherwise.
    pub fn set_phase(&mut self, master: bool) -> Res<()> {
        if master && self.project()?.master.mixdown.is_none() {
            return Err("no-mixdown".into());
        }
        self.master.phase = if master { Phase::Master } else { Phase::Mix };
        self.master.listening = "master".into();
        self.master_reload()
    }

    /// Called on every document change (from `apply`): in the master phase, a master change goes to the chain at
    /// once and the loudness is computed again in the background.
    pub fn master_changed(&mut self) -> bool {
        if self.master.phase != Phase::Master {
            return false;
        }
        let p = self.master_params();
        if let (Some(p), Some(e)) = (p, self.engine.as_mut()) {
            e.set_master(p);
        }
        self.master.epoch += 1;
        self.master.analysing = true;
        self.retarget_check();
        true
    }

    /// Whether the destination differs from the one the measures on screen were made for: then the change is not
    /// heard until the loudness search ends, and the workspace waits under an overlay (9v). Not during a drag (the
    /// gesture's end checks again), nor while the master is prepared (its own overlay, 9d).
    pub fn retarget_check(&mut self) {
        let Some(d) = &self.doc else { return };
        if d.gesture_open() {
            return;
        }
        let t = &d.project.master.target;
        let to = (t.lufs, t.true_peak_db);
        let m = &mut self.master;
        match m.measured_target {
            Some(from) if from != to && m.analysing && m.prep.is_none() => match &mut m.retarget {
                // Changed again on the way: the same wait, towards the new destination.
                Some(r) if r.to != to => {
                    r.to = to;
                    r.fraction = 0.0;
                }
                Some(_) => {}
                None => m.retarget = Some(Retarget { from, to, started: std::time::Instant::now(), fraction: 0.0 }),
            },
            _ => m.retarget = None,
        }
    }

    /// The progress of the loudness job of generation `epoch`, for the overlay 9v.
    pub fn retarget_progress(&mut self, epoch: u64, f: f64) -> bool {
        match &mut self.master.retarget {
            Some(r) if epoch == self.master.epoch => {
                r.fraction = f.clamp(0.0, 1.0);
                true
            }
            _ => false,
        }
    }

    /// What a loudness job needs: the generation, the cache, the master state and where to start the search (the
    /// gain found for the previous state, when there is one).
    #[allow(clippy::type_complexity)]
    pub fn loudness_job(&self) -> Option<(u64, Arc<MasterCache>, MasterState, Option<f64>)> {
        let c = self.master.cache.clone()?;
        Some((self.master.epoch, c, self.project().ok()?.master.clone(), self.loudness_start()))
    }

    /// Where a loudness search starts: the gain found for the state before (a nearby state), or else the distance
    /// from the mixdown's loudness to the destination less the input gain, which the chain mostly keeps. The search
    /// still measures and corrects from there (A-050).
    pub fn loudness_start(&self) -> Option<f64> {
        if self.master.measures.is_some() {
            return Some(self.master.loud_gain_db);
        }
        let m = &self.project().ok()?.master;
        let mix = self.master.cache.as_ref()?.view.lufs?;
        Some(m.target.lufs - mix - m.chain.input_gain_db)
    }

    /// The samples of a bounce for the cache of its mixdown, taken once.
    pub fn take_bounced(&mut self, number: u32) -> Option<Vec<f32>> {
        match self.master.bounced_samples.take() {
            Some((n, x)) if n == number => Some(x),
            _ => None,
        }
    }

    /// The result of a loudness job, if it is still current.
    pub fn loudness_done(&mut self, epoch: u64, gain: f64, measures: MasterMeasures, wave: Wave) {
        if epoch != self.master.epoch {
            return;
        }
        self.master.loud_gain_db = gain;
        self.master.wave = Some(Arc::new(wave));
        self.master.measures = Some(measures);
        self.master.analysing = false;
        self.master.measured_target = self.project().ok().map(|p| (p.master.target.lufs, p.master.target.true_peak_db));
        self.master.retarget = None;
        // Every measure is ready: the workspace is shown (the overlay goes).
        if self.master.prep.as_ref().is_some_and(|p| !p.failed()) {
            self.master.prep = None;
        }
        if self.master.phase == Phase::Master
            && let Some(p) = self.master_params()
            && let Some(e) = &mut self.engine
        {
            e.set_master(p);
        }
    }

    pub fn set_cache(&mut self, c: MasterCache) {
        self.master.cache = Some(Arc::new(c));
        self.master.epoch += 1;
        self.master.analysing = true;
        // Measured from scratch under the preparation's overlay.
        self.master.measured_target = None;
        self.master.retarget = None;
    }

    /// MAS-5: "master", "mixdown" or "reference:<id>".
    pub fn master_listen(&mut self, what: &str) -> Res<()> {
        let was = std::mem::replace(&mut self.master.listening, what.to_string());
        if was.starts_with("reference:") || what.starts_with("reference:") {
            // The status shows what is heard: on failure, back to the previous choice.
            if let Err(e) = self.master_reload() {
                self.master.listening = was;
                let _ = self.master_reload();
                return Err(e);
            }
            return Ok(());
        }
        if let (Some(p), Some(e)) = (self.master_params(), self.engine.as_mut()) {
            e.set_master(p);
        }
        Ok(())
    }

    /// A change to the structure of the chain (MAS-15, MAS-12) as one step of the master history; answers with the
    /// id of the block it added, if any.
    pub fn master_chain(&mut self, op: &stemwise_core::model::ChainOp) -> Res<Option<String>> {
        use stemwise_core::model::ChainOp;
        let (m, added) = self.project()?.master.chain_edit(op)?;
        let name = |id: &str| self.project().ok().and_then(|p| p.master.chain.name(id)).unwrap_or_default();
        let label = match op {
            ChainOp::Add { kind, .. } => format!("Master · {} added", kind.label()),
            ChainOp::Remove { id } => format!("Master · {} removed", name(id)),
            ChainOp::Move { id, .. } => format!("Master · {} moved", name(id)),
            ChainOp::Bands { id, .. } | ChainOp::Split { id, .. } | ChainOp::Merge { id, .. } => {
                format!("Master · {} bands", name(id))
            }
            ChainOp::Reset { id } => format!("Master · {} reset", name(id)),
            ChainOp::ResetOrder => "Master · order reset".to_string(),
        };
        self.transact(Phase::Master, &label, vec![Command::SetMaster { master: Box::new(m) }])?;
        Ok(added)
    }

    /// Bands of a multiband heard alone (15d): `block` is its id, `mask` a bit per band; no block or no bit ends it.
    /// Listening only: nothing is saved, and the export never solos.
    pub fn master_solo(&mut self, block: Option<&str>, mask: u8) -> Res<()> {
        let inst = block.and_then(|id| {
            let m = &self.project().ok()?.master;
            stemwise_engine::master::instances(m).into_iter().find(|(b, _)| b == id).map(|(_, i)| i)
        });
        self.master.solo = inst.filter(|_| mask != 0).map(|i| (i, mask));
        if let (Phase::Master, Some(p), Some(e)) = (self.master.phase, self.master_params(), self.engine.as_mut()) {
            e.set_master(p);
        }
        Ok(())
    }

    /// Listening to what a de-esser takes (FX-14): the engine only, never the document or the export. A rebuilt graph
    /// starts without it.
    pub fn listen_block(&mut self, track: &str, block: &str, on: bool) -> Res<()> {
        if let Some(e) = self.engine.as_mut() {
            e.set_param(track, &format!("{block}.listen"), if on { 1.0 } else { 0.0 });
        }
        Ok(())
    }

    /// Which bands are heard alone, as the interface shows it: (block id, mask).
    pub fn master_solo_view(&self) -> Option<(String, u8)> {
        let (inst, mask) = self.master.solo?;
        let m = &self.project().ok()?.master;
        stemwise_engine::master::instances(m).into_iter().find(|(_, i)| *i == inst).map(|(id, _)| (id, mask))
    }

    /// Real level or same loudness (design 9l–9o, MAS-5). Playback goes on: a reference is reloaded at its new gain,
    /// the rest only changes the chain's gains.
    pub fn master_listen_same(&mut self, on: bool) -> Res<()> {
        if self.master.listen_same == on {
            return Ok(());
        }
        self.master.listen_same = on;
        if self.master.phase != Phase::Master {
            return Ok(());
        }
        if self.master.listening.starts_with("reference:") {
            return self.master_reload();
        }
        if let (Some(p), Some(e)) = (self.master_params(), self.engine.as_mut()) {
            e.set_master(p);
        }
        Ok(())
    }

    // Long jobs: bounce, export, master AI.

    /// Starts a long job; returns its cancel flag and its owner (the open project).
    pub fn job_start(&mut self, kind: &str) -> Res<(Arc<AtomicBool>, u64)> {
        if let Some(j) = &self.master.job {
            return Err(format!("busy: {} in progress", j.view.kind));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.master.job = Some(Job {
            view: MasterJob { kind: kind.into(), progress: 0.0, started_ms: package::now_ms() },
            cancel: cancel.clone(),
            owner: self.open_id,
        });
        Ok((cancel, self.open_id))
    }

    /// Whether a job's project is still the open one.
    pub fn owns(&self, owner: u64) -> bool {
        self.open_id == owner && self.doc.is_some()
    }

    pub fn job_progress(&mut self, owner: u64, f: f64) {
        if let Some(j) = &mut self.master.job
            && j.owner == owner
        {
            j.view.progress = f;
        }
    }

    pub fn job_cancel(&mut self) {
        if let Some(j) = &self.master.job {
            j.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Ends a job; another project's job (the one that started it was closed) is left alone.
    pub fn job_end(&mut self, owner: u64) {
        if self.master.job.as_ref().is_some_and(|j| j.owner == owner) {
            self.master.job = None;
        }
    }

    /// Records a new mixdown (not an undo step: the file exists either way, MDN-4). In the master phase the engine
    /// switches to it at once.
    pub fn bounce_done(&mut self, md: Mixdown, samples: Vec<f32>) -> Res<()> {
        self.view(vec![Command::SetMixdown { mixdown: Some(Box::new(md.clone())) }])?;
        self.master.bounced_samples = Some((md.number, samples));
        self.master.bounced = Some(md);
        self.master.cache = None;
        // A new mixdown replaces a failed preparation (9j "Bounce again").
        self.master.prep = None;
        self.autosave(true);
        if self.master.phase == Phase::Master {
            self.master_reload()?;
        }
        Ok(())
    }

    pub fn export_done(&mut self, r: Vec<FileReport>) {
        self.master.last_export = Some(r);
    }

    /// The decider for a master AI call, as for the mix (AI-9, AI-11).
    pub fn master_ai_prepare(&mut self) -> Res<(Box<dyn Decider>, AiConfig, Arc<MasterCache>)> {
        if self.master.pending.is_some() {
            return Err("a master proposal is in review: apply or discard it first".into());
        }
        let cache = self.master.cache.clone().ok_or("the mixdown is still being measured")?;
        let config = self.ai.config.clone();
        // The tour never calls a model (PRJ-9).
        let decider: Box<dyn Decider> = if config.offline_demo || self.tour.active {
            Box::new(FakeDecider)
        } else {
            match self.ai.keys.get() {
                Some(k) => Box::new(OpenRouter::new(config.clone(), k)),
                None => return Err("no-key".into()),
            }
        };
        Ok((decider, config, cache))
    }

    pub fn master_ai_finished(
        &mut self,
        r: Result<MasterProposal, stemwise_ai::mixing::AiError>,
        after: Option<(f64, MasterMeasures)>,
        request: Option<String>,
        reopened: bool,
    ) -> Result<(), String> {
        match r {
            Ok(p) => {
                let (loud_gain_db, after) = after.map_or((None, None), |(g, m)| (Some(g), Some(m)));
                self.master.pending = Some(Pending { proposal: p, after, loud_gain_db, request, reopened });
                Ok(())
            }
            Err(e) => {
                let rec = e.record().clone();
                self.log_call(rec);
                Err(e.to_string())
            }
        }
    }

    /// MAS-6 with AI-2: the master proposal, or only some of its entries ([`MasterChange::key`]: a block, the order,
    /// the limiter, the sections, the input, the target), as one step of the master history (HIS-5).
    pub fn master_ai_apply(&mut self, only: Option<Vec<String>>) -> Res<String> {
        let Pending { proposal: p, request: req, .. } =
            self.master.pending.take().ok_or("no master proposal to apply")?;
        let all = only.is_none();
        let m = match &only {
            None => {
                // The whole proposal on the master as it is: what the plan does not set (metadata, export) stays.
                let mut m = self.project()?.master.clone();
                m.chain = p.preview.chain.clone();
                m.sections = p.preview.sections.clone();
                m.target = p.preview.target.clone();
                m
            }
            Some(keys) => stemwise_ai::mastering::master_partial(&self.project()?.master, &p.preview, keys),
        };
        m.check()?;
        let label = match &req {
            Some(r) => format!("Master revision applied: “{}”", r.chars().take(48).collect::<String>()),
            None if all => "Master proposal applied".to_string(),
            None => "Master proposal applied in part".to_string(),
        };
        let cmds = vec![Command::SetMaster { master: Box::new(m) }];
        let change = self.doc_mut()?.transact(Phase::Master, &label, cmds, Some(p.call_id.clone())).map_err(err)?;
        if self.master.listening == "proposal" {
            self.master.listening = "master".into();
        }
        self.apply(&change);
        let mut rec = p.record.clone();
        rec.outcome = if all { AiOutcome::Applied } else { AiOutcome::AppliedPartly };
        rec.history_label = Some(label.clone());
        self.log_call(rec);
        Ok(label)
    }

    /// The waveform of the master workspace: the mixdown and the master as last measured, in `columns` columns.
    pub fn master_wave(&self, columns: usize) -> MasterWave {
        let Some(c) = &self.master.cache else { return MasterWave { mixdown: Vec::new(), master: None } };
        MasterWave {
            mixdown: envelope(&c.samples, columns),
            master: self.master.wave.as_ref().map(|w| narrower(w, columns)),
        }
    }

    /// The mixdown's envelope for the waveform of the master workspace: (min, max) per column, both channels.
    pub fn mixdown_peaks(&self, columns: usize) -> Vec<[f32; 2]> {
        let Some(c) = &self.master.cache else { return Vec::new() };
        let frames = c.samples.len() / 2;
        let cols = columns.clamp(16, 4096);
        (0..cols)
            .map(|k| {
                let (a, b) = (k * frames / cols, ((k + 1) * frames / cols).max(k * frames / cols + 1).min(frames));
                c.samples[2 * a..2 * b].iter().fold([0f32, 0f32], |m, v| [m[0].min(*v), m[1].max(*v)])
            })
            .collect()
    }

    /// MAS-9: copies a cover image into the package (`masters/cover.<ext>`) and records it in the metadata.
    pub fn set_cover(&mut self, path: Option<&std::path::Path>) -> Res<()> {
        let dir = self.dir()?.to_path_buf();
        let mut m = self.project()?.master.clone();
        m.metadata.cover = match path {
            None => None,
            Some(src) => {
                let ext = src.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
                if !["jpg", "jpeg", "png"].contains(&ext.as_str()) {
                    return Err("the cover must be a JPEG or PNG image".into());
                }
                let rel = format!("masters/cover.{}", if ext == "jpeg" { "jpg" } else { ext.as_str() });
                std::fs::create_dir_all(dir.join("masters")).map_err(err)?;
                std::fs::copy(src, dir.join(&rel)).map_err(err)?;
                Some(rel)
            }
        };
        self.transact(Phase::Master, "Cover", vec![Command::SetMaster { master: Box::new(m) }])
    }

    pub fn master_ai_discard(&mut self) -> Res<()> {
        let p = self.master.pending.take().ok_or("no master proposal to discard")?;
        // A proposal opened again keeps what its record says happened to it (AI-23).
        if !p.reopened {
            let mut rec = p.proposal.record.clone();
            rec.outcome = AiOutcome::Discarded;
            self.log_call(rec);
        }
        Ok(())
    }

    /// AI-23: a master proposal received earlier, rebuilt on the master as it is now; the caller measures it
    /// before the review, like a new one (A-033).
    pub fn master_ai_reopen_prepare(&mut self, id: &str) -> Res<(MasterProposal, Arc<MasterCache>)> {
        if self.master.pending.is_some() {
            return Err("a master proposal is in review: apply or discard it first".into());
        }
        let cache = self.master.cache.clone().ok_or("the mixdown is still being measured")?;
        let project = self.project()?;
        let rec = project
            .ai
            .calls
            .iter()
            .find(|c| c.id == id && matches!(c.kind, AiCallKind::Master | AiCallKind::MasterRevision))
            .ok_or("no such master proposal")?;
        let proposal = stemwise_ai::mastering::restore_master(self.dir()?, project, rec)?;
        Ok((proposal, cache))
    }

    /// Where exports go: the folder chosen in the settings, or `masters/` inside the package.
    pub fn export_folder(&self) -> Res<PathBuf> {
        self.not_demo()?;
        let p = self.project()?;
        Ok(match &p.master.export.folder {
            Some(f) => PathBuf::from(f),
            None => self.dir()?.join("masters"),
        })
    }
}

/// The loudness gain and the measures of a master state over the cache (a background job): one gain search, one
/// render and one pass for how hard the blocks work (F8 review, N2). `sections` are the project's.
pub fn compute_loudness(
    c: &MasterCache,
    m: &MasterState,
    sections: &[Section],
    start: Option<f64>,
    progress: &mut dyn FnMut(f64),
) -> (f64, MasterMeasures, Wave) {
    let tb = c.target.as_ref().and_then(|t| t.bands_db.as_ref());
    // The search is most of the work; measuring its result the rest.
    let mut search = |f: f64| progress(0.8 * f);
    let bands = c.measures.bands_db.as_ref();
    match render_master_from(&c.samples, c.rate, m, sections, bands, tb, c.rate, start, &mut search) {
        Ok(r) => {
            // The search is over, whatever the passes it took; the measure follows.
            progress(0.8);
            (r.gain_db, measured(c, m, sections, &r), envelope(&r.samples, WAVE_COLUMNS))
        }
        Err(_) => (0.0, mm(&[], c.rate, c.target.as_ref()), Vec::new()),
    }
}

/// Measures a proposed master before it is applied (A-033): its loudness gain and its measures.
pub fn measure_master(
    c: &MasterCache,
    m: &MasterState,
    sections: &[Section],
    start: Option<f64>,
) -> Option<(f64, MasterMeasures)> {
    let tb = c.target.as_ref().and_then(|t| t.bands_db.as_ref());
    let bands = c.measures.bands_db.as_ref();
    let r = render_master_from(&c.samples, c.rate, m, sections, bands, tb, c.rate, start, &mut |_| {}).ok()?;
    Some((r.gain_db, measured(c, m, sections, &r)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: MAS-4
    #[test]
    fn the_envelope_keeps_peaks_and_body_apart_and_narrows_without_losing_either() {
        // Two seconds: quiet, then loud, with one tall sample in the quiet half.
        let n = 96_000;
        let mut x: Vec<f32> = (0..n)
            .flat_map(|i| {
                let a = if i < n / 2 { 0.1 } else { 0.5 };
                let v = a * (std::f32::consts::TAU * 220.0 * i as f32 / 48_000.0).sin();
                [v, v]
            })
            .collect();
        x[2 * 1000] = 0.9;
        let w = envelope(&x, 64);
        assert_eq!(w.len(), 64);
        // The tall sample is in the peaks of its column, not in its body.
        assert!((w[0][1] - 0.9).abs() < 1e-6 && w[0][2] < 0.12, "{:?}", w[0]);
        // The body follows the level: a sine's RMS is its amplitude over √2.
        assert!((w[10][2] - 0.1 / 2f32.sqrt()).abs() < 0.005 && (w[50][2] - 0.5 / 2f32.sqrt()).abs() < 0.01);
        assert!(w[50][0] < -0.49 && w[50][1] > 0.49);
        // Fewer columns: the extremes survive, and the body is still the RMS.
        let few = narrower(&w, 16);
        assert_eq!(few.len(), 16);
        assert!((few[0][1] - 0.9).abs() < 1e-6);
        assert!((few[12][2] - 0.5 / 2f32.sqrt()).abs() < 0.01);
        assert_eq!(narrower(&w, 128), w, "never more columns than it has");
        assert!(envelope(&[], 64).is_empty());
    }
}
