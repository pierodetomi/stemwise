//! Performance gates (FOUNDATION §3.12): NFR-1 (real-time load) and NFR-4 (offline render speed).
//! Run with `make bench` (release). Exits non-zero when a gate is missed.

use std::path::Path;
use std::time::Instant;

use stemwise_core::command::Command;
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::{edit, package, role::Role, wav};
use stemwise_engine::graph::{Feed, Graph};

use stemwise_engine::offline::render_range;

const RATE: u32 = 48000;

struct Noise(u32);
impl Feed for Noise {
    fn fill(&mut self, _: usize, out: &mut [f32]) {
        for v in out {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 17;
            self.0 ^= self.0 << 5;
            *v = (self.0 as f32 / u32::MAX as f32 - 0.5) * 0.4;
        }
    }
}

/// Four more reverb returns, one per algorithm and a second plate, modulation on (FX-9): `full_chain` then sends
/// every track to each of them.
fn add_reverbs(d: &mut Document) {
    use stemwise_core::edit::{self, Edit};
    use stemwise_core::model::ReverbAlgorithm as A;
    for a in [A::Plate, A::Room, A::Hall, A::Plate] {
        let cmds = edit::run(&mut d.project, &Edit::AddReturn { reverb: true, algorithm: Some(a) }).unwrap();
        d.transact(Phase::Mix, "return", cmds, None).unwrap();
    }
}

/// Every block of the chain active, sends to every return.
fn full_chain(d: &mut Document) {
    let returns: Vec<String> = d.project.tracks.iter().filter(|t| t.is_return()).map(|t| t.id.clone()).collect();
    let ids: Vec<String> = d.project.tracks.iter().filter(|t| t.is_audio()).map(|t| t.id.clone()).collect();
    for id in ids {
        let mut t = d.project.track(&id).unwrap().clone();
        t.chain.hpf_mut().bypass = false;
        t.chain.lpf_mut().bypass = false;
        for b in &mut t.chain.eq_mut().bands {
            b.enabled = true;
            b.gain_db = 2.0;
        }
        t.chain.comp_mut().bypass = false;
        t.chain.sat_mut().bypass = false;
        t.chain.trim_db = -2.0;
        for r in &returns {
            t.sends.push(stemwise_core::model::Send { target: r.clone(), level_db: -15.0 });
        }
        d.transact(Phase::Mix, "chain", vec![Command::SetTrack { track: Box::new(t) }], None).unwrap();
    }
}

fn project(dir: &Path, n_tracks: usize, secs: f64, stereo: bool) -> (std::path::PathBuf, Document) {
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (pkg, p) = package::create(dir, "Bench").unwrap();
    let mut d = Document::new(p);
    let frames = (secs * f64::from(RATE)) as usize;
    let ch = if stereo { 2 } else { 1 };
    let mut choices = Vec::new();
    for k in 0..n_tracks {
        let s: Vec<f32> =
            stemwise_dsp::testsignal::xorshift32(k as u32 + 1, frames * ch).iter().map(|v| (v * 0.3) as f32).collect();
        let path = src.join(format!("t{k}.wav"));
        wav::write_f32(&path, ch as u16, RATE, &s).unwrap();
        choices.push(ImportChoice {
            path: path.display().to_string(),
            role: Role::Other,
            name: format!("T{k}"),
            convert: false,
        });
    }
    let cmds = import(&pkg, &mut d.project, &choices).unwrap();
    d.transact(Phase::Mix, "import", cmds, None).unwrap();
    std::fs::remove_dir_all(&src).unwrap();
    (pkg, d)
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    let mut ok = true;

    // NFR-1: 32 stereo tracks, full chain, 512-frame buffers at 48 kHz. The gate is on the 99th percentile of the
    // block time (A-023): the maximum of a non-real-time benchmark thread includes OS preemption.
    for automated in [false, true] {
        let tmp = tempfile::tempdir_in(&root).unwrap();
        let (_pkg, mut d) = project(tmp.path(), 32, 0.1, true);
        add_reverbs(&mut d);
        full_chain(&mut d);
        let ids: Vec<String> = d.project.tracks.iter().filter(|t| t.is_audio()).map(|t| t.id.clone()).take(8).collect();
        let cmds = edit::add_group(&mut d.project, "Drums", &ids).unwrap();
        d.transact(Phase::Mix, "group", cmds, None).unwrap();
        if automated {
            // Every track: fader, pan, high-pass, and the frequency and gain of every EQ band, as fast zig-zags.
            let all: Vec<String> = d.project.tracks.iter().filter(|t| t.is_audio()).map(|t| t.id.clone()).collect();
            for id in all {
                let mut t = d.project.track(&id).unwrap().clone();
                let lane = |param: &str, lo: f64, hi: f64| stemwise_core::model::AutomationLane {
                    param: param.into(),
                    points: (0..400)
                        .map(|k| stemwise_core::model::AutomationPoint {
                            frame: k * 2400,
                            value: if k % 2 == 0 { lo } else { hi },
                        })
                        .collect(),
                    author: stemwise_core::model::Author::User,
                    visible: true,
                };
                t.automation = vec![lane("fader", -10.0, 0.0), lane("pan", -0.5, 0.5), lane("hpf.freq", 40.0, 200.0)];
                for b in 1..=4 {
                    t.automation.push(lane(&format!("eq.{b}.freq"), 200.0 * b as f64, 400.0 * b as f64));
                    t.automation.push(lane(&format!("eq.{b}.gain"), -3.0, 3.0));
                }
                d.transact(Phase::Mix, "auto", vec![Command::SetTrack { track: Box::new(t) }], None).unwrap();
            }
        }
        let mut g = Graph::build(&d.project, f64::from(RATE)).unwrap();
        // The real-time configuration: the audio thread plus the engine's helper threads (A-024).
        g.pool = Some(stemwise_engine::pool::Pool::new(stemwise_engine::pool::Pool::default_helpers()));
        g.reset(0);
        let (mut l, mut r) = (vec![0f32; 512], vec![0f32; 512]);
        let mut feed = Noise(1);
        for k in 0..50 {
            g.process(k * 512, &mut l, &mut r, &mut feed);
        }
        let blocks = 2000;
        let mut times = Vec::with_capacity(blocks);
        for k in 50..50 + blocks as i64 {
            let s = Instant::now();
            g.process(k * 512, &mut l, &mut r, &mut feed);
            times.push(s.elapsed().as_secs_f64());
        }
        times.sort_by(f64::total_cmp);
        let period = 512.0 / f64::from(RATE);
        let mean = times.iter().sum::<f64>() / blocks as f64;
        let p99 = times[blocks * 99 / 100];
        let max = times[blocks - 1];
        let what = if automated { "full chain + automation of every track" } else { "full chain" };
        println!(
            "NFR-1: 32 stereo tracks, {what}: mean {:.1}%, p99 {:.1}%, max {:.1}% of {:.2} ms",
            mean / period * 100.0,
            p99 / period * 100.0,
            max / period * 100.0,
            period * 1e3
        );
        if p99 / period > 0.30 {
            println!("NFR-1: FAILED (p99 > 30%)");
            ok = false;
        }
    }

    // FX-24: the same 32 tracks with the classic chain, and 8 of them also carrying a harmonizer, a four-band
    // multiband and a chorus: the callback stays under 50% of the period.
    {
        use stemwise_core::fx::{Fx, FxKind, FxOp};
        let tmp = tempfile::tempdir_in(&root).unwrap();
        let (_pkg, mut d) = project(tmp.path(), 32, 0.1, true);
        add_reverbs(&mut d);
        full_chain(&mut d);
        let ids: Vec<String> = d.project.tracks.iter().filter(|t| t.is_audio()).map(|t| t.id.clone()).take(8).collect();
        for id in ids {
            for (kind, bands) in [(FxKind::Harmonizer, None), (FxKind::Multiband, Some(4)), (FxKind::Modulation, None)]
            {
                let mut t = d.project.track(&id).unwrap().clone();
                let (c, added) = t.chain.edit(&FxOp::Add { kind, at: None }, false).unwrap();
                t.chain = c;
                let id_b = added.unwrap();
                if let Some(n) = bands {
                    t.chain = t.chain.edit(&FxOp::Bands { id: id_b.clone(), bands: n }, false).unwrap().0;
                }
                let b = t.chain.block_mut(&id_b).unwrap();
                b.fx.set_bypass(false);
                match &mut b.fx {
                    Fx::Harmonizer(h) => h.voices.iter_mut().for_each(|v| v.enabled = true),
                    Fx::Multiband(m) => m.bands.iter_mut().for_each(|x| x.ratio = 3.0),
                    _ => {}
                }
                d.transact(Phase::Mix, "fx", vec![Command::SetTrack { track: Box::new(t) }], None).unwrap();
            }
        }
        let mut g = Graph::build(&d.project, f64::from(RATE)).unwrap();
        g.pool = Some(stemwise_engine::pool::Pool::new(stemwise_engine::pool::Pool::default_helpers()));
        g.reset(0);
        let (mut l, mut r) = (vec![0f32; 512], vec![0f32; 512]);
        let mut feed = Noise(1);
        for k in 0..50 {
            g.process(k * 512, &mut l, &mut r, &mut feed);
        }
        let blocks = 2000;
        let mut times = Vec::with_capacity(blocks);
        for k in 50..50 + blocks as i64 {
            let s = Instant::now();
            g.process(k * 512, &mut l, &mut r, &mut feed);
            times.push(s.elapsed().as_secs_f64());
        }
        times.sort_by(f64::total_cmp);
        let period = 512.0 / f64::from(RATE);
        let mean = times.iter().sum::<f64>() / blocks as f64;
        let p99 = times[blocks * 99 / 100];
        println!(
            "FX-24: 32 tracks, 8 with harmonizer + 4-band multiband + chorus: mean {:.1}%, p99 {:.1}% of {:.2} ms",
            mean / period * 100.0,
            p99 / period * 100.0,
            period * 1e3
        );
        if p99 / period > 0.50 {
            println!("FX-24: FAILED (p99 > 50%)");
            ok = false;
        }
    }

    // NFR-4: offline render of 16 tracks (60 s, mono, full chain) at least 20× faster than real time.
    {
        let tmp = tempfile::tempdir_in(&root).unwrap();
        let (pkg, mut d) = project(tmp.path(), 16, 60.0, false);
        full_chain(&mut d);
        let t = Instant::now();
        let out = render_range(&pkg, &d.project, 0, 60 * i64::from(RATE), 4096, &mut |_| true).unwrap();
        let secs = t.elapsed().as_secs_f64();
        let speed = 60.0 / secs;
        println!("NFR-4: 16 tracks × 60 s rendered in {secs:.2} s: {speed:.1}× real time (hash {})", out.hash());
        if speed < 20.0 {
            println!("NFR-4: FAILED (< 20×)");
            ok = false;
        }
    }
    if !ok {
        std::process::exit(1);
    }
}
