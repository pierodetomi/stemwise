//! The analysis of a project (ANA-13 … ANA-17) and its `analysis` v3 document for the model (FOUNDATION §5.5).
//!
//! - Each audio track is measured on its clip-edited content before the chain (ANA-14): the same `TrackStream`
//!   the engine plays. Results are cached in `cache/analysis/` under a key of the track's clips, media, channels,
//!   sections and rate: changing parameters never invalidates them, editing clips does.
//! - Tracks are analysed in parallel with bounded memory (ANA-17); the raw mix (stems summed with the centre law
//!   and trimmed to −3 dBTP, the PoC's `baseline`) and the family sums (ANA-16) are accumulated as tracks finish.
//! - The document sent to the model uses neutral ids (`t1`, `g1`, `r1`, `s1`), track names and roles, never paths,
//!   file names or the song title (AI-10); numbers are rounded as in the PoC (ANA-13).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use stemwise_analysis::measures::{self, Collision, MixMeasures, StemMeasures, Target};
use stemwise_analysis::sections;
use stemwise_analysis::spectral::{self, BAND_LABELS, N_BANDS, Spectrum};
use stemwise_core::media::{self, MediaError};
use stemwise_core::model::{Output, Project, ReturnFx, Track, TrackKind};
use stemwise_engine::clips::TrackStream;

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error(transparent)]
    Media(#[from] MediaError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("the project has no audio to analyse")]
    Empty,
    #[error("cancelled")]
    Cancelled,
    /// UI-6, ANA-14: the measures are of the tracks' content; a missing file is not measured as silence (and no
    /// cache is written for it).
    #[error("{0} audio file(s) missing from the project folder: locate them first")]
    Missing(usize),
}

/// Neutral ids sent to the model and their mapping back to project ids (§5.5).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ids {
    pub to_neutral: HashMap<String, String>,
    pub to_project: HashMap<String, String>,
    /// Neutral ids of the blocks the analysis lists (`k1` …, ANA-19, AI-25): (track id, block id).
    #[serde(default)]
    pub blocks: std::collections::BTreeMap<String, (String, String)>,
}

impl Ids {
    pub fn of(p: &Project) -> Ids {
        let mut ids = Ids::default();
        let (mut t, mut g, mut r) = (0, 0, 0);
        for tr in &p.tracks {
            let n = match tr.kind {
                TrackKind::Audio => {
                    t += 1;
                    format!("t{t}")
                }
                TrackKind::Group => {
                    g += 1;
                    format!("g{g}")
                }
                TrackKind::Return => {
                    r += 1;
                    format!("r{r}")
                }
            };
            ids.to_neutral.insert(tr.id.clone(), n.clone());
            ids.to_project.insert(n, tr.id.clone());
        }
        let mut secs: Vec<_> = p.sections.iter().collect();
        secs.sort_by_key(|s| s.start);
        for (i, s) in secs.iter().enumerate() {
            let n = format!("s{}", i + 1);
            ids.to_neutral.insert(s.id.clone(), n.clone());
            ids.to_project.insert(n, s.id.clone());
        }
        let mut k = 0;
        for tr in &p.tracks {
            for b in crate::blocks::listed(&tr.chain) {
                k += 1;
                ids.blocks.insert(format!("k{k}"), (tr.id.clone(), b.id.clone()));
            }
        }
        ids
    }

    /// The neutral id of a listed block of a track.
    pub fn block_n(&self, track: &str, block: &str) -> Option<String> {
        self.blocks.iter().find(|(_, (t, b))| t == track && b == block).map(|(n, _)| n.clone())
    }

    pub fn n(&self, project_id: &str) -> String {
        self.to_neutral.get(project_id).cloned().unwrap_or_else(|| project_id.to_string())
    }
}

/// Cached result for one track (full precision).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackAnalysis {
    pub id: String,
    pub key: String,
    pub measures: StemMeasures,
    pub section_lufs: Vec<Option<f64>>,
    pub section_active_pct: Vec<f64>,
    /// Gain to bring the track to −23 LUFS (AI-7, REN-1 of the PoC); 0 when silent.
    pub pre_gain_db: f64,
    /// Band powers per analysis frame (for masking and shares) and the channel count.
    pub band_power: Vec<[f64; N_BANDS]>,
    pub channels: usize,
    /// ANA-19 (P-018); absent from caches written before it, which are then measured again.
    #[serde(default)]
    pub character: Option<stemwise_analysis::character::Character>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Family {
    pub name: String,
    pub members: Vec<String>,
    pub lufs: Option<f64>,
    pub bands_db: Option<[f64; N_BANDS]>,
}

/// Full-precision analysis of a project.
#[derive(Debug, Clone)]
pub struct ProjectAnalysis {
    pub rate: u32,
    pub duration_s: f64,
    pub tracks: Vec<TrackAnalysis>,
    pub mix: MixMeasures,
    pub mix_dc_pct: f64,
    pub mix_section_bands: Vec<Option<[f64; N_BANDS]>>,
    pub shares: Vec<[f64; N_BANDS]>,
    /// Pairs of track ids (project ids) with band index and score, strongest first, families excluded (ANA-16).
    pub collisions: Vec<(String, String, usize, f64)>,
    /// ANA-18: pairs of tracks that are two performances of one part (ids, score, same signal).
    pub doubles: Vec<(String, String, f64, bool)>,
    pub families: Vec<Family>,
    pub references: Vec<(String, MixMeasures)>,
    pub target: Option<Target>,
}

/// Roles whose sibilance is measured (ANA-19).
pub fn is_voice(r: stemwise_core::role::Role) -> bool {
    use stemwise_core::role::Role::*;
    matches!(r, LeadVocal | BackingVocal | Choir)
}

/// Roles whose floor between the hits is measured (ANA-19).
pub fn is_drum(r: stemwise_core::role::Role) -> bool {
    use stemwise_core::role::Role::*;
    matches!(r, Kick | Snare | Toms | Overheads | Room | Drums | Percussion)
}

/// Roles whose parts are recorded twice to be opened to the sides (ANA-18): not drums, bass or effects, and not
/// tracks without a known role.
pub fn can_double(r: stemwise_core::role::Role) -> bool {
    use stemwise_core::role::Role::*;
    matches!(r, ElectricGuitar | AcousticGuitar | Keys | Synth | Pad | Strings | Brass | LeadVocal | BackingVocal)
}

fn section_spans(p: &Project) -> Vec<(i64, i64)> {
    let mut s: Vec<_> = p.sections.iter().map(|s| (s.start, s.end)).collect();
    s.sort();
    s
}

/// Cache key of a track: clips, media, channels, sections, rate (ANA-14) and the analysis version.
pub fn track_key(p: &Project, t: &Track) -> String {
    let media: Vec<_> = t.clips.iter().filter_map(|c| p.media(&c.media)).map(|m| (&m.id, m.frames)).collect();
    let v = json!({"v": 2, "clips": t.clips, "media": media, "ch": t.channels, "secs": section_spans(p), "rate": p.sample_rate, "end": p.content_end()});
    blake3::hash(v.to_string().as_bytes()).to_hex()[..32].to_string()
}

fn cache_path(package: &Path, key: &str) -> std::path::PathBuf {
    package.join("cache").join("analysis").join(format!("{key}.json"))
}

fn read_track(package: &Path, p: &Project, t: &Track, frames: usize) -> Result<Vec<f32>, AnalysisError> {
    let mut s = TrackStream::open(package, p, t)?;
    let mut out = vec![0f32; frames * s.channels()];
    s.read(&mut out)?;
    Ok(out)
}

fn spectrum_from(t: &TrackAnalysis, rate: u32) -> Spectrum {
    Spectrum {
        sample_rate: rate,
        n_samples: 0,
        channels: t.channels,
        n_fft: spectral::fft_size(rate),
        hop: spectral::hop_size(rate),
        frame_power: vec![0.0; t.band_power.len()],
        band_power: t.band_power.clone(),
        active: vec![false; t.band_power.len()],
        active_spectrum: Vec::new(),
        stereo_full: None,
        stereo_low: None,
    }
}

/// Families (ANA-16): the drum roles, and every group with at least two audio tracks.
pub fn families(p: &Project) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let drums: Vec<String> = p
        .tracks
        .iter()
        .filter(|t| t.is_audio() && t.role.is_some_and(|r| r.is_drums()))
        .map(|t| t.id.clone())
        .collect();
    if drums.len() >= 2 {
        out.push(("drums".to_string(), drums));
    }
    for g in p.tracks.iter().filter(|t| t.is_group()) {
        let members: Vec<String> = p
            .tracks
            .iter()
            .filter(|t| t.is_audio() && stemwise_core::routing::depth(p, &t.id) > 0 && is_inside(p, t, &g.id))
            .map(|t| t.id.clone())
            .collect();
        if members.len() >= 2 && !out.iter().any(|(_, m)| *m == members) {
            out.push((g.name.clone(), members));
        }
    }
    out
}

fn is_inside(p: &Project, t: &Track, group: &str) -> bool {
    let mut cur = t;
    for _ in 0..p.tracks.len() {
        match &cur.output {
            Output::Group(g) if g == group => return true,
            Output::Group(g) => match p.track(g) {
                Some(n) => cur = n,
                None => return false,
            },
            Output::MixBus => return false,
        }
    }
    false
}

/// Analyses a project. `progress` receives values in 0…1 and may return `false` to cancel (AI-12).
pub fn analyze_project(
    package: &Path,
    p: &Project,
    progress: &(dyn Fn(f64) -> bool + Sync),
) -> Result<ProjectAnalysis, AnalysisError> {
    let rate = p.sample_rate.ok_or(AnalysisError::Empty)?;
    let missing = stemwise_core::package::missing_media(package, p).len();
    if missing > 0 {
        return Err(AnalysisError::Missing(missing));
    }
    let frames = p.content_end().max(0) as usize;
    if frames == 0 {
        return Err(AnalysisError::Empty);
    }
    let audio: Vec<&Track> = p.tracks.iter().filter(|t| t.is_audio()).collect();
    let secs = section_spans(p);
    let fams = families(p);
    // Sums in f32 (NFR-5: a 5-minute stereo sum is 115 MB, not 230); 32 tracks add an error far below the
    // measures' resolution.
    let mix = Mutex::new(vec![0f32; frames * 2]);
    let fam_sums: Vec<Mutex<Vec<f32>>> = fams.iter().map(|_| Mutex::new(vec![0f32; frames * 2])).collect();
    let done = Mutex::new(0usize);
    let total = audio.len() + 1;
    // Bounded parallelism: at most 2 tracks in memory at once (NFR-5: a 5-minute stem with its spectra is a few
    // hundred MB while it is measured).
    let pool =
        rayon::ThreadPoolBuilder::new().num_threads(2).build().map_err(|e| std::io::Error::other(e.to_string()))?;
    let results: Vec<Result<TrackAnalysis, AnalysisError>> = pool.install(|| {
        use rayon::prelude::*;
        audio
            .par_iter()
            .map(|t| {
                let key = track_key(p, t);
                let cached: Option<TrackAnalysis> = std::fs::read_to_string(cache_path(package, &key))
                    .ok()
                    .and_then(|s| serde_json::from_str::<TrackAnalysis>(&s).ok())
                    .filter(|c| c.character.is_some());
                let ch = usize::from(t.channels.clamp(1, 2));
                let mut result = cached;
                // The audio is always read: the raw mix and the family sums need every track.
                {
                    let x = read_track(package, p, t, frames)?;
                    // Raw mix and family sums (centre law for mono).
                    let add = |buf: &mut Vec<f32>| {
                        for i in 0..frames {
                            if ch == 1 {
                                let v = x[i] * std::f32::consts::FRAC_1_SQRT_2;
                                buf[2 * i] += v;
                                buf[2 * i + 1] += v;
                            } else {
                                buf[2 * i] += x[2 * i];
                                buf[2 * i + 1] += x[2 * i + 1];
                            }
                        }
                    };
                    add(&mut mix.lock().expect("lock"));
                    for (k, (_, m)) in fams.iter().enumerate() {
                        if m.contains(&t.id) {
                            add(&mut fam_sums[k].lock().expect("lock"));
                        }
                    }
                    if result.is_none() {
                        let (m, spec) = measures::measure_stem(&x, ch, rate);
                        let pre = match m.lufs {
                            Some(l) if l.is_finite() => -23.0 - l,
                            _ => 0.0,
                        };
                        let ta = TrackAnalysis {
                            id: t.id.clone(),
                            key: key.clone(),
                            section_lufs: sections::section_lufs(&x, ch, rate, &secs),
                            section_active_pct: sections::section_active_pct(&spec, &secs),
                            measures: m,
                            pre_gain_db: pre,
                            band_power: spec.band_power.clone(),
                            channels: ch,
                            character: Some(stemwise_analysis::character::measure(
                                &x,
                                ch,
                                rate,
                                t.role.is_some_and(is_voice),
                                t.role.is_some_and(is_drum),
                            )),
                        };
                        let path = cache_path(package, &key);
                        if let Some(dir) = path.parent() {
                            let _ = std::fs::create_dir_all(dir);
                        }
                        let _ = std::fs::write(&path, serde_json::to_string(&ta).unwrap_or_default());
                        result = Some(ta);
                    }
                }
                let mut d = done.lock().expect("lock");
                *d += 1;
                if !progress(*d as f64 / total as f64) {
                    return Err(AnalysisError::Cancelled);
                }
                result.ok_or(AnalysisError::Empty)
            })
            .collect()
    });
    let mut tracks = Vec::new();
    for r in results {
        tracks.push(r?);
    }
    // Mix: the raw sum trimmed to −3 dBTP (PoC D-007/D-017).
    let mut mix: Vec<f32> = mix.into_inner().expect("lock");
    let tp = stemwise_dsp::truepeak::true_peak_db(&mix, 2);
    let trim = (-3.0 - 0.01 - tp).min(0.0);
    let g = 10f64.powf(trim / 20.0);
    for v in &mut mix {
        *v = (f64::from(*v) * g) as f32;
    }
    let (mix_m, mix_spec) = measures::measure_mix(&mix, 2, rate);
    let mix_dc_pct = measures::dc_offset_pct(&mix, 2);
    let mix_section_bands = sections::section_bands(&mix_spec, &secs);
    drop(mix);
    // Families.
    let families: Vec<Family> = fams
        .iter()
        .zip(fam_sums)
        .map(|((name, members), sum)| {
            let x: Vec<f32> = sum.into_inner().expect("lock");
            let spec = spectral::analyze(&x, 2, rate);
            Family {
                name: name.clone(),
                members: members.clone(),
                lufs: stemwise_dsp::loudness::integrated(&x, 2, rate),
                bands_db: spectral::band_levels_db(&spec),
            }
        })
        .collect();
    // Shares and masking on the pre-normalised stems when AI-7 is on (D-028), families excluded (ANA-16).
    let specs: Vec<Spectrum> = tracks
        .iter()
        .map(|t| {
            let mut s = spectrum_from(t, rate);
            if p.ai.normalize_trim {
                let k = 10f64.powf(t.pre_gain_db / 10.0);
                for f in &mut s.band_power {
                    for v in f.iter_mut() {
                        *v *= k;
                    }
                }
            }
            s
        })
        .collect();
    let refs: Vec<&Spectrum> = specs.iter().collect();
    let shares = measures::band_share_pct(&refs);
    let same_family = |i: usize, j: usize| {
        families.iter().any(|f| f.members.contains(&tracks[i].id) && f.members.contains(&tracks[j].id))
    };
    let collisions: Vec<(String, String, usize, f64)> = measures::collisions(&refs, &same_family)
        .into_iter()
        .map(|Collision { a, b, band, score }| (tracks[a].id.clone(), tracks[b].id.clone(), band, score))
        .collect();
    // True doubles among tracks of the same role (ANA-18).
    let role = |i: usize| p.track(&tracks[i].id).and_then(|t| t.role).filter(|r| can_double(*r));
    let doubles: Vec<(String, String, f64, bool)> = measures::doubles(&refs, &|i, j| {
        role(i).is_some() && role(i) == role(j) && tracks[i].channels == tracks[j].channels
    })
    .into_iter()
    .map(|d| (tracks[d.a].id.clone(), tracks[d.b].id.clone(), d.score, d.identical))
    .collect();
    // References and target (ANA-11, ANA-12).
    let mut references = Vec::new();
    for r in &p.references {
        let (info, x) = media::decode_all(&package.join(&r.media.path))?;
        let x = if info.sample_rate != rate {
            media::resample(&x, usize::from(info.channels), info.sample_rate, rate)?
        } else {
            x
        };
        if let Some(m) = measures::measure_reference(&x, usize::from(info.channels), rate) {
            references.push((r.id.clone(), m));
        }
    }
    let target = measures::target_profile(&references.iter().map(|(_, m)| m.clone()).collect::<Vec<_>>());
    progress(1.0);
    Ok(ProjectAnalysis {
        rate,
        duration_s: frames as f64 / f64::from(rate),
        tracks,
        mix: mix_m,
        mix_dc_pct,
        mix_section_bands,
        shares,
        collisions,
        doubles,
        families,
        references,
        target,
    })
}

// ---------------------------------------------------------------------------------------------
// The document for the model (ANA-13, AI-10)
// ---------------------------------------------------------------------------------------------

/// One decimal, ties to even (the PoC's rounding); levels limited to ±120.
pub fn r1(v: f64) -> f64 {
    secs1(v.clamp(-120.0, 120.0))
}

/// One decimal for a time in seconds: unlike a level it has no ±120 limit (a song lasts longer than two minutes).
pub fn secs1(v: f64) -> f64 {
    let r = (v * 10.0).round_ties_even() / 10.0;
    if r == 0.0 { 0.0 } else { r }
}

/// Two decimals for quantities in −1…1.
pub fn r2(v: f64) -> f64 {
    let r = (v * 100.0).round_ties_even() / 100.0;
    if r == 0.0 { 0.0 } else { r }
}

fn o1(v: Option<f64>) -> Value {
    v.map_or(Value::Null, |x| json!(r1(x)))
}

fn bands(v: Option<&[f64; N_BANDS]>) -> Value {
    v.map_or(Value::Null, |b| json!(b.iter().map(|x| r1(*x)).collect::<Vec<_>>()))
}

fn stereo(s: Option<measures::StereoMeasures>) -> Value {
    s.map_or(Value::Null, |s| json!({"correlation": s.correlation.map(r2), "side_mid_db": s.side_mid_db.map(r1)}))
}

fn mix_value(m: &MixMeasures) -> serde_json::Map<String, Value> {
    let mut o = serde_json::Map::new();
    o.insert("lufs".into(), o1(m.lufs));
    o.insert("true_peak_db".into(), json!(r1(m.true_peak_db)));
    o.insert("dyn_range_lu".into(), o1(m.dyn_range_lu));
    o.insert("bands_db".into(), bands(m.bands_db.as_ref()));
    o.insert("stereo".into(), stereo(m.stereo));
    o.insert("low_corr".into(), m.low_corr.map_or(Value::Null, |c| json!(r2(c))));
    o
}

/// The `analysis` v3 document (FOUNDATION §5.5) for the model: neutral ids, no paths or file names.
/// A track name as the model sees it (AI-10, A-036): names are allowed, but a name taken from a file may carry the
/// song title or a take number. The project name is removed, as are leading track numbers; an empty result becomes
/// "Track".
pub fn model_name(name: &str, project: &str) -> String {
    let mut n = name.to_string();
    let title = project.trim();
    if title.chars().count() >= 3 {
        let lower = n.to_lowercase();
        let t = title.to_lowercase();
        let mut out = String::new();
        let mut i = 0;
        while let Some(k) = lower[i..].find(&t) {
            out.push_str(&n[i..i + k]);
            i += k + t.len();
        }
        out.push_str(&n[i..]);
        n = out;
    }
    let trimmed = n
        .trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ' || c == '-' || c == '_' || c == '.')
        .trim_matches(|c: char| c == ' ' || c == '-' || c == '_');
    let clean: String = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() { "Track".into() } else { clean }
}

pub fn model_document(a: &ProjectAnalysis, p: &Project, ids: &Ids) -> Value {
    let mut secs: Vec<_> = p.sections.iter().collect();
    secs.sort_by_key(|s| s.start);
    let rate = f64::from(a.rate);
    let sections: Vec<Value> = secs
        .iter()
        .map(|s| json!({"id": ids.n(&s.id), "name": s.name, "start_s": secs1(s.start as f64 / rate), "end_s": secs1(s.end as f64 / rate)}))
        .collect();
    let returns: Vec<Value> = p
        .tracks
        .iter()
        .filter(|t| t.is_return())
        .map(|t| {
            let mut r = json!({"id": ids.n(&t.id), "type": if t.ret.as_ref().is_some_and(ReturnFx::is_reverb) { "reverb" } else { "delay" }, "locked": t.locked});
            if let Some(a) = t.ret.as_ref().and_then(ReturnFx::algorithm) {
                r["algorithm"] = json!(a);
            }
            if let Some(b) = k_blocks(t, ids) {
                r["blocks"] = b;
            }
            r
        })
        .collect();
    let group_of = |t: &Track| match &t.output {
        Output::Group(g) => json!(ids.n(g)),
        Output::MixBus => Value::Null,
    };
    let mut tracks = serde_json::Map::new();
    for ta in &a.tracks {
        let t = p.track(&ta.id).expect("analysed from this project");
        let m = &ta.measures;
        tracks.insert(
            ids.n(&t.id),
            json!({
                "name": model_name(&t.name, &p.name),
                "role": t.role.map(|r| r.as_str()),
                "group": group_of(t),
                "locked": t.locked,
                "channels": m.channels,
                "lufs": o1(m.lufs),
                "peak_db": r1(m.peak_db),
                "true_peak_db": r1(m.true_peak_db),
                "crest_db": o1(m.crest_db),
                "dyn_range_lu": o1(m.dyn_range_lu),
                "active_pct": r1(m.active_pct),
                "bands_db": bands(m.bands_db.as_ref()),
                "centroid_hz": m.centroid_hz.map(|c| c.round()),
                "onsets_per_s": r1(m.onsets_per_s),
                "stereo": stereo(m.stereo),
                "section_lufs": ta.section_lufs.iter().map(|v| o1(*v)).collect::<Vec<_>>(),
                "section_active_pct": ta.section_active_pct.iter().map(|v| v.round()).collect::<Vec<_>>(),
            }),
        );
        // ANA-19 (v4): only what the track has, so the document stays under the budget of ANA-13.
        if let Some(c) = &ta.character
            && let Some(o) = tracks.get_mut(&ids.n(&t.id)).and_then(Value::as_object_mut)
        {
            for (k, v) in [
                ("sibilance_db", c.sibilance_db.map(r1)),
                ("sibilance_hz", c.sibilance_hz.map(|v| v.round())),
                ("crest_db", c.crest_db.map(r1)),
                ("transient_db", c.transient_db.map(r1)),
                ("floor_db", c.floor_db.map(r1)),
            ] {
                if let Some(v) = v {
                    o.insert(k.into(), json!(v));
                }
            }
            if k_blocks(t, ids).is_some() {
                o.insert("blocks".into(), k_blocks(t, ids).unwrap_or_default());
            }
        } else if let Some(o) = tracks.get_mut(&ids.n(&t.id)).and_then(Value::as_object_mut)
            && let Some(b) = k_blocks(t, ids)
        {
            o.insert("blocks".into(), b);
        }
    }
    let groups: BTreeMap<String, Value> = p
        .tracks
        .iter()
        .filter(|t| t.is_group())
        .map(|t| {
            let mut g = json!({"name": model_name(&t.name, &p.name), "group": group_of(t), "locked": t.locked});
            if let Some(b) = k_blocks(t, ids) {
                g["blocks"] = b;
            }
            (ids.n(&t.id), g)
        })
        .collect();
    let mut mix = mix_value(&a.mix);
    mix.insert(
        "section_bands_db".into(),
        json!(a.mix_section_bands.iter().map(|b| bands(b.as_ref())).collect::<Vec<_>>()),
    );
    let shares: BTreeMap<String, Value> = a
        .tracks
        .iter()
        .zip(&a.shares)
        .map(|(t, s)| (ids.n(&t.id), json!(s.iter().map(|v| v.round()).collect::<Vec<_>>())))
        .collect();
    let masking: Vec<Value> = a
        .collisions
        .iter()
        .take(10)
        .map(|(x, y, b, s)| json!({"a": ids.n(x), "b": ids.n(y), "band": BAND_LABELS[*b], "score": r2(*s)}))
        .collect();
    let families: Vec<Value> = a
        .families
        .iter()
        .map(|f| json!({"name": model_name(&f.name, &p.name), "members": f.members.iter().map(|m| ids.n(m)).collect::<Vec<_>>(), "lufs": o1(f.lufs), "bands_db": bands(f.bands_db.as_ref())}))
        .collect();
    let doubles: Vec<Value> = a
        .doubles
        .iter()
        .map(|(x, y, s, same)| json!({"a": ids.n(x), "b": ids.n(y), "score": r2(*s), "identical": same}))
        .collect();
    let target = a.target.as_ref().map_or(Value::Null, |t| {
        json!({"n_references": t.n_references, "bands_db": bands(t.bands_db.as_ref()), "side_mid_db": o1(t.side_mid_db), "dyn_range_lu": o1(t.dyn_range_lu)})
    });
    let mut project = json!({"sample_rate": a.rate, "duration_s": secs1(a.duration_s)});
    if let Some(t) = p.tempo {
        project["tempo"] = json!(t.bpm);
        project["meter"] = json!(format!("{}/{}", t.beats_per_bar, t.beat_unit));
    }
    json!({
        "version": 4,
        "project": project,
        "sections": sections,
        "returns": returns,
        "tracks": tracks,
        "groups": groups,
        "families": families,
        "mix": mix,
        "band_share_pct": shares,
        "masking_top": masking,
        "doubles": doubles,
        "target": target,
    })
}

/// The blocks of a chain the analysis lists (ANA-19, AI-25): neutral id, type, state and place in the chain, for the
/// blocks the fields of version 3 do not address. `None` when there are none (the document stays short).
fn k_blocks(t: &Track, ids: &Ids) -> Option<Value> {
    let listed: Vec<Value> = crate::blocks::listed(&t.chain)
        .into_iter()
        .filter_map(|b| {
            let n = ids.block_n(&t.id, &b.id)?;
            let at = t.chain.blocks.iter().position(|x| x.id == b.id)?;
            Some(json!({"id": n, "type": b.fx.kind(), "bypass": b.fx.bypass(), "position": at}))
        })
        .collect();
    (!listed.is_empty()).then(|| json!(listed))
}

/// Estimated tokens of a document (characters / 4, as in the PoC).
pub fn estimated_tokens(doc: &Value) -> usize {
    doc.to_string().chars().count() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: AI-10
    #[test]
    fn track_names_lose_the_song_title_and_take_numbers_before_reaching_the_model() {
        assert_eq!(model_name("03 Paper Lanterns Lead Vox", "Paper Lanterns"), "Lead Vox");
        assert_eq!(model_name("paper lanterns_mix", "Paper Lanterns"), "mix");
        assert_eq!(model_name("Kick In", "Paper Lanterns"), "Kick In");
        assert_eq!(model_name("12", "x"), "Track");
        assert_eq!(model_name("Bass", "Ba"), "Bass", "short titles are not stripped");
    }
}
