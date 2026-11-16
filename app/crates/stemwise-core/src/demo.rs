//! The demo project of the guided tour (PRJ-9, P-020): a song of 68 s at 48 kHz with the twelve tracks of the design
//! boards, two groups, two returns with sends, a sidechain, three named sections, an automation lane and a brief.
//!
//! Nothing is read from disk: the document is built here with the same edits and commands the app uses, and the
//! audio is synthesised from closed formulas with fixed seeds (no random generator of a library, no clock), so two
//! builds give the same document and the same render. The stems are written into the package and adopted as
//! imported media (named by their content hash), never copied from files shipped with the app.

use std::f64::consts::TAU;
use std::path::{Path, PathBuf};

use crate::command::Command;
use crate::edit::{self, Edit};
use crate::history::{Document, Phase};
use crate::media;
use crate::model::{
    AutomationPoint, Brief, Clip, EqKind, Fade, FadeShape, Output, Project, ReverbAlgorithm, Tempo, Track,
};
use crate::package;
use crate::role::Role;
use crate::wav;

/// The name the demo project carries in the app.
pub const NAME: &str = "Demo song";
pub const RATE: u32 = 48_000;
/// Length of the song in seconds.
pub const SECONDS: f64 = 68.0;
/// 120 BPM in 4/4: a beat is half a second, a bar two seconds.
const BEAT: f64 = 0.5;
const BAR: f64 = 2.0;
/// Verse, Chorus, Outro (seconds).
pub const SECTIONS: [(&str, f64, f64); 3] = [("Verse", 0.0, 24.0), ("Chorus", 24.0, 52.0), ("Outro", 52.0, 68.0)];
/// A, F♯m, D, E: one chord per bar (MIDI notes) and the bass root of each.
const CHORDS: [[f64; 3]; 4] = [[57.0, 61.0, 64.0], [54.0, 57.0, 61.0], [50.0, 54.0, 57.0], [52.0, 56.0, 59.0]];
const ROOTS: [f64; 4] = [45.0, 42.0, 38.0, 40.0];

#[derive(Debug, thiserror::Error)]
pub enum DemoError {
    #[error("the demo song could not be written: {0}")]
    Io(#[from] std::io::Error),
    #[error("the demo song could not be built: {0}")]
    Build(String),
}

fn build_err<E: std::fmt::Display>(e: E) -> DemoError {
    DemoError::Build(e.to_string())
}

// ---------------------------------------------------------------------------------------------
// Synthesis: closed formulas, fixed seeds
// ---------------------------------------------------------------------------------------------

fn hz(midi: f64) -> f64 {
    440.0 * 2f64.powf((midi - 69.0) / 12.0)
}

/// White noise in [-1, 1] from a sample index and a seed (SplitMix64): the same everywhere, every time.
fn noise(i: u64, seed: u64) -> f64 {
    let mut z = i.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ seed.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 52) as f64 - 1.0
}

/// Brighter noise: the difference of two neighbours.
fn bright(i: u64, seed: u64) -> f64 {
    0.5 * (noise(i, seed) - noise(i.wrapping_sub(1), seed))
}

/// Darker noise: the mean of four neighbours.
fn dark(i: u64, seed: u64) -> f64 {
    (0..4).map(|k| noise(i.wrapping_sub(k), seed)).sum::<f64>() * 0.5
}

fn section(t: f64) -> usize {
    SECTIONS.iter().position(|s| t < s.2).unwrap_or(2)
}

fn chord(t: f64) -> usize {
    ((t / BAR) as usize) % 4
}

/// The times of the last `n` events of a grid (`period` seconds) that `hit` accepts, at or before `t`.
fn last_hits(t: f64, period: f64, n: usize, hit: impl Fn(i64) -> bool) -> impl Iterator<Item = (i64, f64)> {
    let k0 = (t / period).floor() as i64;
    (0..8).map(move |d| k0 - d).filter(move |k| *k >= 0 && hit(*k)).take(n).map(move |k| (k, t - k as f64 * period))
}

/// A phase accumulator, in cycles.
#[derive(Default, Clone, Copy)]
struct Osc(f64);
impl Osc {
    fn next(&mut self, f: f64) -> f64 {
        let p = self.0;
        self.0 = (self.0 + f / f64::from(RATE)).fract();
        p
    }
}

/// One-pole smoothing (amplitudes and glides, so no change ever clicks).
#[derive(Clone, Copy)]
struct Smooth {
    v: f64,
    k: f64,
}
impl Smooth {
    fn new(ms: f64) -> Self {
        Smooth { v: 0.0, k: 1.0 - (-1000.0 / (ms * f64::from(RATE))).exp() }
    }
    fn to(&mut self, target: f64) -> f64 {
        self.v += (target - self.v) * self.k;
        self.v
    }
}

fn kick_hit(k: i64, t: f64) -> bool {
    section(t) == 1 || k % 2 == 0
}

fn kick(t: f64, i: u64) -> f64 {
    last_hits(t, BEAT, 2, |k| kick_hit(k, k as f64 * BEAT))
        .map(|(_, p)| {
            let body = (TAU * (48.0 * p + 110.0 / 28.0 * (1.0 - (-28.0 * p).exp()))).sin() * (-7.0 * p).exp();
            0.85 * body + 0.22 * (-250.0 * p).exp() * noise(i, 1)
        })
        .sum()
}

fn snare(t: f64, i: u64, seed: u64) -> f64 {
    last_hits(t, BEAT, 2, |k| k % 2 == 1)
        .map(|(_, p)| (-16.0 * p).exp() * (0.6 * bright(i, seed) + 0.4 * (TAU * 185.0 * p).sin() * (-10.0 * p).exp()))
        .sum::<f64>()
        * if section(t) == 1 { 0.6 } else { 0.5 }
}

type Gen = Box<dyn FnMut(f64, u64) -> [f64; 2]>;

/// One synthesised stem: its file, its track, its clips (seconds on the timeline).
struct Stem {
    name: &'static str,
    role: Role,
    channels: u16,
    clips: &'static [(f64, f64)],
    synth: fn() -> Gen,
}

fn g_kick() -> Gen {
    Box::new(|t, i| [kick(t, i), 0.0])
}

fn g_snare() -> Gen {
    Box::new(|t, i| [snare(t, i, 2), 0.0])
}

fn g_overheads() -> Gen {
    Box::new(|t, i| {
        let dec = if section(t) == 1 { 18.0 } else { 45.0 };
        let hat =
            |seed| last_hits(t, BEAT / 2.0, 2, |_| true).map(|(_, p)| (-dec * p).exp()).sum::<f64>() * bright(i, seed);
        let bleed = 0.35 * snare(t, i, 3) + 0.12 * kick(t, i);
        [0.12 * hat(4) + bleed, 0.17 * hat(5) + bleed]
    })
}

fn g_room() -> Gen {
    Box::new(|t, i| {
        let k: f64 = last_hits(t, BEAT, 2, |k| kick_hit(k, k as f64 * BEAT))
            .map(|(_, p)| (TAU * 52.0 * p).sin() * (-4.0 * p).exp())
            .sum();
        let s =
            |seed| last_hits(t, BEAT, 2, |k| k % 2 == 1).map(|(_, p)| (-6.0 * p).exp()).sum::<f64>() * dark(i, seed);
        [0.25 * k + 0.16 * s(6), 0.25 * k + 0.16 * s(7)]
    })
}

fn g_bass() -> Gen {
    let (mut osc, mut amp) = (Osc::default(), Smooth::new(4.0));
    Box::new(move |t, _| {
        let step = match section(t) {
            1 => BEAT / 2.0,
            2 => BEAT * 2.0,
            _ => BEAT,
        };
        let p = t % step;
        let f = hz(ROOTS[chord(t)]);
        let target = if p < step * 0.85 { (-2.5 * p).exp() } else { 0.0 };
        let a = amp.to(target);
        let ph = osc.next(f);
        let s = (TAU * ph).sin() + 0.35 * (2.0 * TAU * ph).sin() + 0.12 * (3.0 * TAU * ph).sin();
        [0.42 * a * s, 0.0]
    })
}

fn g_acoustic() -> Gen {
    Box::new(|t, _| {
        let (mut l, mut r) = (0.0, 0.0);
        for (k, p0) in last_hits(t, BEAT, 2, |_| true) {
            let c = chord(k as f64 * BEAT);
            let notes = [CHORDS[c][0], CHORDS[c][1], CHORDS[c][2], CHORDS[c][0] + 12.0];
            let up = k % 2 == 1;
            for (n, m) in notes.iter().enumerate() {
                let order = if up { 3 - n } else { n };
                let p = p0 - order as f64 * 0.012;
                if p < 0.0 {
                    continue;
                }
                // A plucked string: three partials, the higher ones dying faster (e^-4.5p, e^-7.5p, e^-10.5p).
                let a = (-1.5 * p).exp();
                let (a3, a2) = (a * a * a, a * a);
                let (s1, c1) = (TAU * hz(*m) * p).sin_cos();
                let (s2, s3) = (2.0 * s1 * c1, s1 * (3.0 - 4.0 * s1 * s1));
                let v = a3 * (s1 + a2 * (0.5 * s2 + 0.25 * a2 * s3));
                let w = [(1.0, 0.5), (0.8, 0.8), (0.5, 1.0), (0.7, 0.7)][n];
                l += 0.075 * w.0 * v;
                r += 0.075 * w.1 * v;
            }
        }
        let g = if section(t) == 1 { 0.75 } else { 1.0 };
        [g * l, g * r]
    })
}

fn electric(detune_cents: f64, late: f64, seed: u64) -> Gen {
    let mut osc = [Osc::default(); 3];
    let mut amp = Smooth::new(3.0);
    let ratio = 2f64.powf(detune_cents / 1200.0);
    Box::new(move |t0, _| {
        let t = (t0 - late).max(0.0);
        let k = (t / (BEAT / 2.0)) as u64;
        let p = t % (BEAT / 2.0);
        let accent = 0.75 + 0.25 * noise(k, seed).abs();
        let a = amp.to(accent * (0.55 + 0.45 * (-12.0 * p).exp()));
        let r = ROOTS[chord(t)] + 12.0;
        let mut x = 0.0;
        for (o, m) in osc.iter_mut().zip([r, r + 7.0, r + 12.0]) {
            let ph = o.next(hz(m) * ratio);
            for n in 1..=6 {
                x += (TAU * n as f64 * ph).sin() / n as f64;
            }
        }
        [0.38 * (2.2 * 0.3 * a * x).tanh(), 0.0]
    })
}

fn g_rhodes() -> Gen {
    Box::new(|t, _| {
        let mut v = 0.0;
        for (k, p) in last_hits(t, BAR / 2.0, 3, |_| true) {
            let c = chord(k as f64 * BAR / 2.0);
            let (e, bell) = ((-1.1 * p).exp(), (-5.0 * p).exp());
            for m in CHORDS[c] {
                let x = TAU * hz(m) * p;
                v += e * x.sin() + 0.18 * bell * (7.0 * x).sin();
            }
        }
        let trem = 0.3 * (TAU * 4.0 * t).sin();
        [0.085 * v * (1.0 + trem), 0.085 * v * (1.0 - trem)]
    })
}

/// Voices that glide from chord to chord (strings, backing vocals): smoothed pitch and level, never a click.
struct Pads {
    osc: Vec<[Osc; 2]>,
    glide: Vec<Smooth>,
    amp: Smooth,
}
impl Pads {
    fn new(voices: usize) -> Self {
        Pads { osc: vec![[Osc::default(); 2]; voices], glide: vec![Smooth::new(60.0); voices], amp: Smooth::new(120.0) }
    }
}

fn g_strings() -> Gen {
    let mut s = Pads::new(3);
    Box::new(move |t, _| {
        let c = chord(t);
        let a = s.amp.to(if section(t) == 1 { 0.8 } else { 1.0 });
        let vib = 1.0 + 0.003 * (TAU * 5.0 * t).sin();
        let (mut l, mut r) = (0.0, 0.0);
        for ((glide, osc), m) in s.glide.iter_mut().zip(s.osc.iter_mut()).zip(CHORDS[c]) {
            let f = glide.to(hz(m)) * vib;
            let pl = osc[0].next(f * 2f64.powf(-4.0 / 1200.0));
            let pr = osc[1].next(f * 2f64.powf(4.0 / 1200.0));
            let saw = |ph: f64| (TAU * ph).sin() + 0.4 * (2.0 * TAU * ph).sin() + 0.2 * (3.0 * TAU * ph).sin();
            l += saw(pl);
            r += saw(pr);
        }
        [0.06 * a * l, 0.06 * a * r]
    })
}

fn g_lead() -> Gen {
    // One note per beat from the chord, an octave up; the last two beats of every fourth bar are a breath.
    const PAT: [usize; 8] = [2, 1, 0, 1, 2, 2, 1, 0];
    let (mut osc, mut amp, mut glide) = (Osc::default(), Smooth::new(25.0), Smooth::new(30.0));
    Box::new(move |t, i| {
        let beat = (t / BEAT) as usize;
        let rest = (beat / 4) % 4 == 3 && beat % 4 >= 2;
        let m = CHORDS[chord(t)][PAT[beat % 8]] + 12.0;
        let f = glide.to(hz(m)) * (1.0 + 0.006 * (TAU * 5.5 * t).sin());
        let a = amp.to(if rest { 0.0 } else { 1.0 });
        let ph = osc.next(f);
        let w = [1.0, 0.55, 0.4, 0.3, 0.18, 0.12, 0.08];
        let v: f64 = w.iter().enumerate().map(|(n, w)| w * ((n + 1) as f64 * TAU * ph).sin()).sum();
        // A breath of "s" at the start of every phrase, for the de-esser to find.
        let p = t % (BEAT * 8.0);
        let ess = if p < 0.04 { 0.25 * bright(i, 10) } else { 0.0 };
        [0.2 * a * v + ess, 0.0]
    })
}

fn g_backing() -> Gen {
    let mut s = Pads::new(2);
    Box::new(move |t, _| {
        let c = chord(t);
        let a = s.amp.to(1.0);
        let vib = 1.0 + 0.005 * (TAU * 5.0 * t + 1.0).sin();
        let mut v = [0.0; 2];
        for (n, out) in v.iter_mut().enumerate() {
            let f = s.glide[n].to(hz(CHORDS[c][n + 1] + 12.0)) * vib;
            let ph = s.osc[n][0].next(f);
            *out = (TAU * ph).sin() + 0.3 * (2.0 * TAU * ph).sin() + 0.1 * (3.0 * TAU * ph).sin();
        }
        [0.16 * a * (0.8 * v[0] + 0.4 * v[1]), 0.16 * a * (0.4 * v[0] + 0.8 * v[1])]
    })
}

const STEMS: [Stem; 12] = [
    Stem { name: "Kick", role: Role::Kick, channels: 1, clips: &[(0.0, 64.0)], synth: g_kick },
    Stem { name: "Snare", role: Role::Snare, channels: 1, clips: &[(0.0, 64.0)], synth: g_snare },
    Stem { name: "Overheads", role: Role::Overheads, channels: 2, clips: &[(0.0, 66.0)], synth: g_overheads },
    Stem { name: "Room", role: Role::Room, channels: 2, clips: &[(0.0, 66.0)], synth: g_room },
    Stem { name: "Bass DI", role: Role::Bass, channels: 1, clips: &[(0.0, 24.0), (24.0, 64.0)], synth: g_bass },
    Stem { name: "Acoustic", role: Role::AcousticGuitar, channels: 2, clips: &[(0.0, 68.0)], synth: g_acoustic },
    Stem { name: "Gtr Left", role: Role::ElectricGuitar, channels: 1, clips: &[(24.0, 52.0)], synth: g_gtr_left },
    Stem { name: "Gtr Right", role: Role::ElectricGuitar, channels: 1, clips: &[(24.0, 52.0)], synth: g_gtr_right },
    Stem { name: "Rhodes", role: Role::Keys, channels: 2, clips: &[(0.0, 24.0), (52.0, 68.0)], synth: g_rhodes },
    Stem { name: "Strings", role: Role::Strings, channels: 2, clips: &[(24.0, 68.0)], synth: g_strings },
    Stem { name: "Lead Vox", role: Role::LeadVocal, channels: 1, clips: &[(2.0, 22.0), (26.0, 50.0)], synth: g_lead },
    Stem { name: "BVs", role: Role::BackingVocal, channels: 2, clips: &[(26.0, 50.0)], synth: g_backing },
];

fn g_gtr_left() -> Gen {
    electric(0.0, 0.0, 8)
}

fn g_gtr_right() -> Gen {
    electric(6.0, 0.007, 9)
}

fn frames(secs: f64) -> i64 {
    (secs * f64::from(RATE)).round() as i64
}

/// The interleaved samples of a stem over the span of its clips.
fn render(stem: &Stem) -> Vec<f32> {
    let (a, b) = (stem.clips[0].0, stem.clips[stem.clips.len() - 1].1);
    let (f0, f1) = (frames(a), frames(b));
    let ch = usize::from(stem.channels);
    let mut out = Vec::with_capacity((f1 - f0) as usize * ch);
    let mut g = (stem.synth)();
    for f in f0..f1 {
        let s = g(f as f64 / f64::from(RATE), f as u64);
        for c in s.iter().take(ch) {
            out.push(c.clamp(-1.0, 1.0) as f32);
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The document
// ---------------------------------------------------------------------------------------------

fn edit(d: &mut Document, e: Edit) -> Result<(), DemoError> {
    let cmds = edit::run(&mut d.project, &e).map_err(build_err)?;
    d.transact(Phase::Mix, "Demo", cmds, None).map_err(build_err)?;
    Ok(())
}

fn cmd(d: &mut Document, cmds: Vec<Command>) -> Result<(), DemoError> {
    d.transact(Phase::Mix, "Demo", cmds, None).map_err(build_err)?;
    Ok(())
}

fn set(d: &mut Document, track: &str, values: &[(&str, f64)]) -> Result<(), DemoError> {
    let cmds =
        values.iter().map(|(p, v)| Command::SetParam { track: track.into(), param: (*p).into(), value: *v }).collect();
    cmd(d, cmds)
}

fn band(d: &mut Document, track: &str, n: usize, kind: EqKind, freq: f64, gain: f64, q: f64) -> Result<(), DemoError> {
    edit(d, Edit::SetEqBand { track: track.into(), band: n, enabled: true, kind, block: None })?;
    set(d, track, &[(&format!("eq.{n}.freq"), freq), (&format!("eq.{n}.gain"), gain), (&format!("eq.{n}.q"), q)])
}

fn id_of(d: &Document, name: &str) -> Result<String, DemoError> {
    d.project.tracks.iter().find(|t| t.name == name).map(|t| t.id.clone()).ok_or_else(|| build_err(name))
}

/// Builds the demo project in `<parent>/Demo song.stemwise`, stems included. Two builds give the same document and
/// the same files (PRJ-9).
pub fn build(parent: &Path) -> Result<(PathBuf, Project), DemoError> {
    let (dir, project) = package::create(parent, NAME).map_err(build_err)?;
    let src = dir.join("cache").join("demo-src");
    std::fs::create_dir_all(&src)?;
    let mut d = Document::new(project);
    cmd(&mut d, vec![Command::SetSampleRate { sample_rate: Some(RATE) }])?;
    // The stems, one track each, before the two returns of a new project.
    // Each stem is synthesised, written and stored on its own thread; the document takes them in order.
    let stored: Vec<Result<crate::model::MediaFile, DemoError>> = std::thread::scope(|scope| {
        let jobs: Vec<_> = STEMS
            .iter()
            .map(|stem| {
                let (dir, file) = (&dir, src.join(format!("{}.wav", stem.name)));
                scope.spawn(move || {
                    wav::write_pcm(&file, stem.channels, RATE, 16, &render(stem), false, false)?;
                    media::adopt(dir, &file, "audio").map_err(build_err)
                })
            })
            .collect();
        jobs.into_iter()
            .map(|j| j.join().unwrap_or_else(|_| Err(build_err("a stem could not be synthesised"))))
            .collect()
    });
    for (stem, m) in STEMS.iter().zip(stored) {
        let m = m?;
        let span = stem.clips[0].0;
        let id = d.project.new_id("t");
        let mut t = Track::new_audio(id, stem.name, Some(stem.role), stem.channels, stem.role.color());
        for (n, (a, b)) in stem.clips.iter().enumerate() {
            let last = n + 1 == stem.clips.len() && *b >= 64.0;
            let clip_id = d.project.new_id("c");
            t.clips.push(Clip {
                id: clip_id,
                media: m.id.clone(),
                start: frames(*a),
                offset: (frames(*a) - frames(span)) as u64,
                length: (frames(*b) - frames(*a)) as u64,
                gain_db: 0.0,
                fade_in: Fade { length: frames(0.01) as u64, shape: FadeShape::EqualPower },
                fade_out: Fade { length: frames(if last { 1.5 } else { 0.01 }) as u64, shape: FadeShape::EqualPower },
                mute: false,
            });
        }
        let at = d.project.tracks.iter().position(Track::is_return).unwrap_or(d.project.tracks.len());
        cmd(
            &mut d,
            vec![Command::AddMedia { media: Box::new(m) }, Command::InsertTrack { index: at, track: Box::new(t) }],
        )?;
    }
    let _ = std::fs::remove_dir_all(&src);

    let brief = Brief {
        genre: "Indie rock".into(),
        mood: "Warm, intimate".into(),
        notes: "Vocal up front. Dry drums. Guitars wide but never harsh.".into(),
    };
    cmd(
        &mut d,
        vec![
            Command::SetBrief { brief },
            Command::SetTempo { tempo: Some(Tempo { bpm: 120.0, beats_per_bar: 4, beat_unit: 4 }) },
        ],
    )?;
    for (name, a, b) in SECTIONS {
        edit(&mut d, Edit::AddSection { name: name.into(), start: frames(a), end: frames(b) })?;
    }

    // The two returns of a new project become a plate and a dotted-eighth echo (375 ms at 120 BPM).
    let (plate, echo) = {
        let mut r = d.project.tracks.iter().filter(|t| t.is_return()).map(|t| t.id.clone());
        (r.next().ok_or_else(|| build_err("no reverb"))?, r.next().ok_or_else(|| build_err("no delay"))?)
    };
    edit(&mut d, Edit::RenameTrack { track: plate.clone(), name: "Plate".into(), color: None })?;
    edit(&mut d, Edit::SetReverbAlgorithm { track: plate.clone(), algorithm: ReverbAlgorithm::Plate })?;
    edit(&mut d, Edit::RenameTrack { track: echo.clone(), name: "Echo".into(), color: None })?;

    let t = |d: &Document, n: &str| id_of(d, n);
    let (kick, snare, oh, room) = (t(&d, "Kick")?, t(&d, "Snare")?, t(&d, "Overheads")?, t(&d, "Room")?);
    let (bass, ac, gl, gr) = (t(&d, "Bass DI")?, t(&d, "Acoustic")?, t(&d, "Gtr Left")?, t(&d, "Gtr Right")?);
    let (rhodes, strings, vox, bvs) = (t(&d, "Rhodes")?, t(&d, "Strings")?, t(&d, "Lead Vox")?, t(&d, "BVs")?);

    // Chains: the classic blocks with a few values that are not neutral, as a rough mix would have them.
    set(
        &mut d,
        &kick,
        &[("bypass.hpf", 0.0), ("hpf.freq", 40.0), ("bypass.comp", 0.0), ("comp.ratio", 4.0), ("fader", -6.5)],
    )?;
    band(&mut d, &kick, 3, EqKind::Peak, 3000.0, 2.0, 1.2)?;
    set(
        &mut d,
        &snare,
        &[
            ("bypass.hpf", 0.0),
            ("hpf.freq", 90.0),
            ("bypass.comp", 0.0),
            ("bypass.sat", 0.0),
            ("sat.drive", 3.0),
            ("fader", -8.0),
        ],
    )?;
    set(&mut d, &oh, &[("bypass.hpf", 0.0), ("hpf.freq", 200.0), ("fader", -11.5)])?;
    set(&mut d, &room, &[("bypass.comp", 0.0), ("comp.ratio", 6.0), ("comp.threshold", -24.0), ("fader", -16.0)])?;
    set(
        &mut d,
        &bass,
        &[("bypass.hpf", 0.0), ("hpf.freq", 40.0), ("bypass.comp", 0.0), ("comp.ratio", 4.0), ("fader", -7.2)],
    )?;
    set(&mut d, &ac, &[("bypass.hpf", 0.0), ("hpf.freq", 80.0), ("pan", -0.3), ("fader", -12.4)])?;
    set(
        &mut d,
        &gl,
        &[
            ("bypass.hpf", 0.0),
            ("hpf.freq", 100.0),
            ("bypass.lpf", 0.0),
            ("lpf.freq", 9000.0),
            ("pan", -0.8),
            ("fader", -10.1),
        ],
    )?;
    set(
        &mut d,
        &gr,
        &[
            ("bypass.hpf", 0.0),
            ("hpf.freq", 100.0),
            ("bypass.lpf", 0.0),
            ("lpf.freq", 9000.0),
            ("pan", 0.8),
            ("fader", -10.3),
        ],
    )?;
    set(&mut d, &rhodes, &[("pan", 0.2), ("fader", -14.0)])?;
    set(&mut d, &strings, &[("bypass.hpf", 0.0), ("hpf.freq", 120.0), ("fader", -15.5)])?;
    set(
        &mut d,
        &vox,
        &[
            ("bypass.hpf", 0.0),
            ("hpf.freq", 90.0),
            ("bypass.comp", 0.0),
            ("bypass.sat", 0.0),
            ("sat.drive", 4.0),
            ("sat.mix", 0.35),
            ("fader", -5.8),
        ],
    )?;
    band(&mut d, &vox, 2, EqKind::Peak, 220.0, -2.5, 1.2)?;
    band(&mut d, &vox, 3, EqKind::Peak, 3200.0, 2.0, 0.9)?;
    set(
        &mut d,
        &bvs,
        &[("bypass.hpf", 0.0), ("hpf.freq", 110.0), ("bypass.comp", 0.0), ("comp.ratio", 2.5), ("fader", -13.0)],
    )?;

    // Sends to the two returns, and the bass ducking under the kick.
    for (track, target, db) in [
        (&snare, &plate, -18.0),
        (&ac, &plate, -20.0),
        (&vox, &plate, -14.0),
        (&vox, &echo, -22.0),
        (&bvs, &plate, -12.0),
        (&rhodes, &echo, -16.0),
    ] {
        edit(&mut d, Edit::SetSend { track: track.clone(), target: target.clone(), level_db: Some(db) })?;
    }
    edit(&mut d, Edit::SetSidechain { track: bass.clone(), source: Some(kick.clone()), block: None, band: None })?;

    // Two groups; the drum group is a parallel compressor.
    edit(&mut d, Edit::AddGroup { name: "Drums".into(), members: vec![kick, snare, oh, room] })?;
    edit(&mut d, Edit::AddGroup { name: "Guitars".into(), members: vec![ac, gl, gr] })?;
    let (drums, guitars) = (t(&d, "Drums")?, t(&d, "Guitars")?);
    set(&mut d, &drums, &[("bypass.comp", 0.0), ("comp.ratio", 2.0), ("comp.mix", 0.5), ("fader", -4.0)])?;
    band(&mut d, &guitars, 3, EqKind::Peak, 2500.0, -1.5, 1.0)?;
    set(&mut d, &guitars, &[("fader", -6.5)])?;

    // The lead vocal's fader drawn by hand, a little louder in the chorus.
    let pts = [(0.0, -6.5), (23.9, -6.5), (24.1, -4.5), (51.9, -4.5), (52.1, -7.0), (68.0, -7.0)]
        .iter()
        .map(|(s, v)| AutomationPoint { frame: frames(*s), value: *v })
        .collect();
    edit(&mut d, Edit::SetLane { track: vox, param: "fader".into(), points: pts })?;

    let mut project = d.project;
    // Every output went through the edits above; nothing is left pointing at a group that does not exist.
    debug_assert!(project.tracks.iter().all(|t| match &t.output {
        Output::MixBus => true,
        Output::Group(g) => project.track(g).is_some(),
    }));
    project.name = NAME.into();
    package::save(&dir, &project).map_err(build_err)?;
    Ok((dir, project))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
        std::fs::create_dir_all(&root).unwrap();
        tempfile::tempdir_in(root).unwrap()
    }

    /// req: PRJ-9
    #[test]
    fn two_builds_give_the_same_document_and_files() {
        let (a, b) = (tmp(), tmp());
        let (da, pa) = build(a.path()).unwrap();
        let (db, pb) = build(b.path()).unwrap();
        assert_eq!(package::to_json(&pa), package::to_json(&pb));
        for m in &pa.media {
            assert_eq!(media::hash_file(&da.join(&m.path)).unwrap(), media::hash_file(&db.join(&m.path)).unwrap());
        }
    }

    /// req: PRJ-9
    #[test]
    fn the_demo_has_what_the_tour_names() {
        let t = tmp();
        let (dir, p) = build(t.path()).unwrap();
        assert_eq!(p.name, NAME);
        assert_eq!(p.sample_rate, Some(RATE));
        let audio: Vec<_> = p.tracks.iter().filter(|t| t.is_audio()).collect();
        assert!((8..=12).contains(&audio.len()));
        assert!(audio.iter().any(|t| t.channels == 1) && audio.iter().any(|t| t.channels == 2));
        assert!(p.tracks.iter().filter(|t| t.kind == crate::model::TrackKind::Group).count() >= 2);
        assert_eq!(p.tracks.iter().filter(|t| t.is_return()).count(), 2);
        assert!(p.tracks.iter().any(|t| !t.sends.is_empty()));
        assert!(
            serde_json::to_string(&p).unwrap().contains("\"sidechain\":{"),
            "a compressor listens to another track"
        );
        assert_eq!(p.sections.len(), 3);
        assert!(p.tracks.iter().any(|t| !t.automation.is_empty()));
        assert!(!p.brief.genre.is_empty() && p.references.is_empty());
        let end = p.content_end() as f64 / f64::from(RATE);
        assert!((60.0..=75.0).contains(&end), "{end} s");
        // The stems live in the package, as imported audio; nothing else is left behind.
        assert!(package::missing_media(&dir, &p).is_empty());
        assert!(!dir.join("cache").join("demo-src").exists());
        // Every stem sounds, and none clips.
        for m in &p.media {
            let (_, s) = media::decode_all(&dir.join(&m.path)).unwrap();
            let peak = s.iter().fold(0f32, |a, v| a.max(v.abs()));
            assert!(peak > 0.05 && peak < 0.999, "{} peaks at {peak}", m.original_name);
        }
    }
}
