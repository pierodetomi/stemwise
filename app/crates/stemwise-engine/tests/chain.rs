//! The chain of blocks in the engine (FX-10 → FX-12): bypass fade, constant latency, resonant filters, the chain of
//! a return, mono turning stereo, free order. Packages live in `target/tmp` and are removed at the end.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use stemwise_core::command::Command;
use stemwise_core::edit::{self, Edit};
use stemwise_core::fx::{Fx, FxKind, FxOp};
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::model::{Author, AutomationLane, AutomationPoint, Sidechain};
use stemwise_core::package;
use stemwise_core::role::Role;
use stemwise_core::wav;
use stemwise_engine::offline::{Rendered, render_range};
use stemwise_engine::realtime::Engine;

const RATE: u32 = 48000;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

struct Fx2 {
    _t: tempfile::TempDir,
    dir: PathBuf,
    doc: Document,
}

fn fixture(signals: &[(&str, u16, Vec<f32>)]) -> Fx2 {
    let t = tmp();
    let src = t.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (dir, p) = package::create(t.path(), "Chain").unwrap();
    let mut doc = Document::new(p);
    let mut choices = Vec::new();
    for (name, ch, s) in signals {
        let path = src.join(format!("{name}.wav"));
        wav::write_f32(&path, *ch, RATE, s).unwrap();
        choices.push(ImportChoice {
            path: path.display().to_string(),
            role: Role::Other,
            name: name.to_string(),
            convert: false,
        });
    }
    let cmds = import(&dir, &mut doc.project, &choices).unwrap();
    doc.transact(Phase::Mix, "import", cmds, None).unwrap();
    Fx2 { _t: t, dir, doc }
}

fn sine(n: usize, freq: f64, amp: f64) -> Vec<f32> {
    (0..n).map(|i| (amp * (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(RATE)).sin()) as f32).collect()
}

fn noise(n: usize, seed: u32, amp: f64) -> Vec<f32> {
    stemwise_dsp::testsignal::xorshift32(seed, n).iter().map(|v| (v * amp) as f32).collect()
}

fn tid(f: &Fx2, name: &str) -> String {
    f.doc.project.tracks.iter().find(|t| t.name == name).unwrap().id.clone()
}

fn edit(f: &mut Fx2, e: Edit) {
    let cmds = edit::run(&mut f.doc.project, &e).unwrap();
    f.doc.transact(Phase::Mix, "edit", cmds, None).unwrap();
}

fn set(f: &mut Fx2, track: &str, param: &str, value: f64) {
    let t = tid(f, track);
    f.doc.transact(Phase::Mix, "set", vec![Command::SetParam { track: t, param: param.into(), value }], None).unwrap();
}

fn render(f: &Fx2, end: i64, block: usize) -> Rendered {
    render_range(&f.dir, &f.doc.project, 0, end, block, &mut |_| true).unwrap()
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0f32, |m, v| m.max(v.abs()))
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| f64::from(*v) * f64::from(*v)).sum::<f64>() / x.len().max(1) as f64).sqrt()
}

fn max_diff(a: &Rendered, b: &Rendered) -> f32 {
    a.left.iter().zip(&b.left).chain(a.right.iter().zip(&b.right)).fold(0f32, |m, (x, y)| m.max((x - y).abs()))
}

/// Only the returns' own sends: a quiet project with the default returns muted.
fn mute_returns(f: &mut Fx2) {
    for n in ["Reverb", "Delay"] {
        set(f, n, "mute", 1.0);
    }
}

/// req: FX-10, AUT-1
#[test]
fn a_bypass_glides_over_about_ten_ms_whoever_sets_it() {
    let n = RATE as usize;
    let mut f = fixture(&[("Tone", 1, sine(n, 5000.0, 0.5))]);
    mute_returns(&mut f);
    set(&mut f, "Tone", "bypass.lpf", 0.0);
    set(&mut f, "Tone", "lpf.freq", 1000.0);
    // The low-pass switches off at frame 24 000 by automation.
    let t = tid(&f, "Tone");
    let mut tr = f.doc.project.track(&t).unwrap().clone();
    tr.automation.push(AutomationLane {
        param: "bypass.lpf".into(),
        points: vec![AutomationPoint { frame: 0, value: 0.0 }, AutomationPoint { frame: 24_000, value: 1.0 }],
        author: Author::User,
        visible: false,
    });
    f.doc.transact(Phase::Mix, "lane", vec![Command::SetTrack { track: Box::new(tr) }], None).unwrap();
    let r = render(&f, n as i64, 4096);
    let full = 0.5 * std::f32::consts::FRAC_1_SQRT_2; // mono pan law at the centre
    let win = |a: usize| peak(&r.left[a..a + 48]);
    assert!(win(20_000) < 0.1 * full, "filtered before: {}", win(20_000));
    // Half way through the glide (≈ 5 ms) the tone is near half its level; after ≈ 10 ms it is all there.
    let mid = win(24_000 + 216);
    assert!(mid > 0.3 * full && mid < 0.7 * full, "half way: {mid} of {full}");
    assert!(win(24_000 + 120) < mid, "it rises");
    let after = win(24_000 + 520);
    assert!((after - full).abs() < 0.02 * full, "after the glide: {after} of {full}");
    // Block size does not change the glide (ENG-4).
    assert!(max_diff(&r, &render(&f, n as i64, 64)) <= 1e-6);
}

fn bypass_lane(f: &mut Fx2, track: &str, param: &str, frames: &[(i64, f64)]) {
    let t = tid(f, track);
    let mut tr = f.doc.project.track(&t).unwrap().clone();
    tr.automation.push(AutomationLane {
        param: param.into(),
        points: frames.iter().map(|(frame, value)| AutomationPoint { frame: *frame, value: *value }).collect(),
        author: Author::User,
        visible: false,
    });
    f.doc.transact(Phase::Mix, "lane", vec![Command::SetTrack { track: Box::new(tr) }], None).unwrap();
}

/// A block switched off and on again by automation renders the same at every block size, also when a block
/// boundary falls inside the cell where the glide ends (12 481 … 12 511 at 48 kHz: the glide lasts 481 samples).
/// req: FX-10, ENG-4
#[test]
fn a_block_switched_off_and_on_again_does_not_depend_on_the_block_size() {
    let n = RATE as usize;
    let mut f = fixture(&[("Tone", 1, noise(n, 7, 0.5))]);
    mute_returns(&mut f);
    set(&mut f, "Tone", "bypass.lpf", 0.0);
    set(&mut f, "Tone", "lpf.freq", 1000.0);
    add_on(&mut f, "Tone", FxKind::Harmonizer, |x| {
        if let Fx::Harmonizer(h) = x {
            h.mix = 0.4
        }
    });
    add_on(&mut f, "Tone", FxKind::Doubler, |_| {});
    let steps = [(0, 0.0), (12_000, 1.0), (30_000, 0.0)];
    for p in ["bypass.lpf", "bypass.comp", "bypass.harm", "bypass.dbl"] {
        bypass_lane(&mut f, "Tone", p, &steps);
    }
    let a = render(&f, n as i64, 4096);
    for b in [64usize, 777, 1000, 12_481, 12_490, 12_511] {
        let d = max_diff(&a, &render(&f, n as i64, b));
        assert!(d == 0.0, "block {b}: {d:e}");
    }
}

/// A block that comes back does not play what it held when it went off: its lines start empty.
/// req: FX-10
#[test]
fn a_block_that_comes_back_starts_clean() {
    let n = RATE as usize;
    // Noise for a quarter of a second, then nothing.
    let sig: Vec<f32> = noise(n, 9, 0.5).iter().enumerate().map(|(i, v)| if i < 12_000 { *v } else { 0.0 }).collect();
    let mut f = fixture(&[("Tone", 1, sig)]);
    mute_returns(&mut f);
    add_on(&mut f, "Tone", FxKind::Doubler, |_| {});
    // Off while the noise is still playing (its delay lines are full), on again in the silence.
    bypass_lane(&mut f, "Tone", "bypass.dbl", &[(0, 0.0), (11_000, 1.0), (30_000, 0.0)]);
    let r = render(&f, n as i64, 4096);
    assert!(peak(&r.left[20_000..29_000]) == 0.0, "silent while off");
    let back = peak(&r.left[30_000..]).max(peak(&r.right[30_000..]));
    assert!(back == 0.0, "old audio came back: {back}");
}

/// req: FX-10, ENG-5
#[test]
fn the_latency_of_a_chain_does_not_move_a_track_against_the_others() {
    let n = RATE as usize / 2;
    let x = noise(n, 5, 0.3);
    let mut f = fixture(&[("A", 1, x.clone()), ("B", 1, x)]);
    mute_returns(&mut f);
    set(&mut f, "A", "mute", 1.0);
    let end = n as i64;
    let classic = render(&f, end, 4096);
    // B without its saturation: its chain is shorter, the engine pads it to the latency of A's.
    let b = tid(&f, "B");
    edit(&mut f, Edit::Chain { track: b.clone(), change: FxOp::Remove { id: "sat".into() } });
    let shorter = render(&f, end, 4096);
    assert!(max_diff(&classic, &shorter) <= 1e-6, "B stays where it was: {}", max_diff(&classic, &shorter));
    // Nobody has a saturation any more: the graph latency drops, the render is still on the timeline.
    let a = tid(&f, "A");
    edit(&mut f, Edit::Chain { track: a, change: FxOp::Remove { id: "sat".into() } });
    let none = render(&f, end, 4096);
    assert!(max_diff(&classic, &none) <= 1e-6);
    // And A unmuted sums with B sample-aligned: twice B.
    set(&mut f, "A", "mute", 0.0);
    let both = render(&f, end, 4096);
    let d = both.left.iter().zip(&none.left).fold(0f32, |m, (x, y)| m.max((x - 2.0 * y).abs()));
    assert!(d <= 1e-6, "aligned: {d}");
}

/// req: FX-12
#[test]
fn high_and_low_pass_resonate_at_their_frequency() {
    let n = RATE as usize / 2;
    let mut f = fixture(&[("Tone", 1, sine(n, 1000.0, 0.1))]);
    mute_returns(&mut f);
    set(&mut f, "Tone", "bypass.hpf", 0.0);
    set(&mut f, "Tone", "hpf.freq", 1000.0);
    let level = |f: &Fx2| rms(&render(f, n as i64, 4096).left[12_000..]) / (0.1 * 0.5); // input RMS · mono law
    let flat = level(&f);
    assert!((flat - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.02, "Butterworth: −3 dB at the cutoff ({flat})");
    set(&mut f, "Tone", "hpf.res", 8.0);
    let peaky = level(&f);
    assert!((peaky / 8.0 - 1.0).abs() < 0.05, "Q 8 lifts the cutoff by 18 dB ({peaky})");
    set(&mut f, "Tone", "bypass.hpf", 1.0);
    set(&mut f, "Tone", "bypass.lpf", 0.0);
    set(&mut f, "Tone", "lpf.freq", 1000.0);
    set(&mut f, "Tone", "lpf.res", 4.0);
    let lp = level(&f);
    assert!((lp / 4.0 - 1.0).abs() < 0.05, "low-pass too ({lp})");
}

/// req: FX-11, FX-3
#[test]
fn the_chain_of_a_return_ducks_under_the_track_it_listens_to() {
    let n = RATE as usize;
    // A voice in bursts sending to the reverb.
    let vox: Vec<f32> =
        sine(n, 330.0, 0.5).iter().enumerate().map(|(i, v)| if (i / 6000) % 2 == 0 { *v } else { 0.0 }).collect();
    let mut f = fixture(&[("Vox", 1, vox)]);
    let (v, r1) = (tid(&f, "Vox"), tid(&f, "Reverb"));
    let cmds = edit::set_send(&f.doc.project, &v, &r1, Some(-6.0)).unwrap();
    f.doc.transact(Phase::Mix, "send", cmds, None).unwrap();
    let mut rt = f.doc.project.track(&r1).unwrap().clone();
    rt.solo = true;
    f.doc.transact(Phase::Mix, "solo", vec![Command::SetTrack { track: Box::new(rt) }], None).unwrap();
    let wet = |f: &Fx2| rms(&render(f, n as i64, 4096).left);
    let plain = wet(&f);
    assert!(plain > 1e-3);
    // A compressor after the reverb, listening to the voice.
    edit(&mut f, Edit::Chain { track: r1.clone(), change: FxOp::Add { kind: FxKind::Compressor, at: None } });
    let mut rt = f.doc.project.track(&r1).unwrap().clone();
    if let Fx::Compressor(c) = &mut rt.chain.block_mut("comp").unwrap().fx {
        c.bypass = false;
        c.threshold_db = -50.0;
        c.ratio = 10.0;
        c.sidechain = Some(Sidechain { source: v.clone(), hpf_hz: 100.0 });
    }
    f.doc.transact(Phase::Mix, "duck", vec![Command::SetTrack { track: Box::new(rt) }], None).unwrap();
    let ducked = wet(&f);
    assert!(20.0 * (ducked / plain).log10() < -3.0, "ducked by {:.1} dB", 20.0 * (ducked / plain).log10());
    // The core refuses a key from a return, and a return never sends (TRK-5).
    let mut bad = f.doc.project.track(&r1).unwrap().clone();
    if let Fx::Compressor(c) = &mut bad.chain.block_mut("comp").unwrap().fx {
        c.sidechain = Some(Sidechain { source: r1.clone(), hpf_hz: 100.0 });
    }
    assert!(f.doc.transact(Phase::Mix, "bad", vec![Command::SetTrack { track: Box::new(bad) }], None).is_err());
}

/// req: FX-10, TRK-2
#[test]
fn a_mono_chain_turned_stereo_keeps_its_pan_law_and_order_is_free() {
    let n = RATE as usize / 2;
    let mut f = fixture(&[("Mono", 1, noise(n, 9, 0.3))]);
    mute_returns(&mut f);
    set(&mut f, "Mono", "pan", -0.5);
    set(&mut f, "Mono", "bypass.eq", 0.0);
    let t = tid(&f, "Mono");
    edit(
        &mut f,
        Edit::SetEqBand {
            track: t.clone(),
            band: 1,
            enabled: true,
            kind: stemwise_core::model::EqKind::Peak,
            block: None,
        },
    );
    set(&mut f, "Mono", "eq.2.gain", 6.0);
    let before = render(&f, n as i64, 4096);
    // A width block that leaves the sound as it is, at the end: the chain is stereo from it, the pan law stays.
    edit(&mut f, Edit::Chain { track: t.clone(), change: FxOp::Add { kind: FxKind::Width, at: None } });
    set(&mut f, "Mono", "width.width", 1.0);
    let after = render(&f, n as i64, 4096);
    assert!(max_diff(&before, &after) <= 1e-6, "{}", max_diff(&before, &after));
    // Two EQs in a free order: same output whatever the block size.
    edit(&mut f, Edit::Chain { track: t.clone(), change: FxOp::Duplicate { id: "eq".into() } });
    edit(&mut f, Edit::Chain { track: t.clone(), change: FxOp::Move { id: "eq2".into(), to: 0 } });
    let two = render(&f, n as i64, 4096);
    assert!(rms(&two.left) > rms(&before.left) * 1.05, "the second EQ adds its own 6 dB");
    edit(&mut f, Edit::Chain { track: t, change: FxOp::Move { id: "comp".into(), to: 0 } });
    set(&mut f, "Mono", "bypass.comp", 0.0);
    let a = render(&f, n as i64, 4096);
    let b = render(&f, n as i64, 64);
    let c = render(&f, n as i64, 1000);
    assert!(max_diff(&a, &b) <= 1e-6 && a.hash() == c.hash());
}

/// req: FX-10, ENG-1
#[test]
fn structural_changes_while_playing_do_not_click() {
    let n = 6 * RATE as usize;
    // A slow sine: any jump to silence or between graphs is far larger than one sample's step.
    let x: Vec<f32> = (0..n).map(|i| (0.3 * (i as f64 * 0.02).sin()) as f32).collect();
    let mut f = fixture(&[("Tone", 1, x)]);
    mute_returns(&mut f);
    let t = tid(&f, "Tone");
    let (mut e, mut ap) = Engine::new(f64::from(RATE), 4096);
    e.load(&f.dir, &f.doc.project).unwrap();
    e.play();
    let mut out: Vec<f32> = Vec::new();
    let mut buf = vec![0f32; 512 * 2];
    let changes = [
        FxOp::Add { kind: FxKind::Gate, at: Some(0) },
        FxOp::Move { id: "gate".into(), to: 3 },
        FxOp::Remove { id: "sat".into() },
        FxOp::Add { kind: FxKind::Saturation, at: None },
        FxOp::Duplicate { id: "eq".into() },
    ];
    let mut k = 0;
    for step in 0..400 {
        ap.callback(&mut buf, 2);
        if e.shared.position.load(Ordering::Relaxed) > 0 {
            out.extend(buf.iter().step_by(2));
        }
        e.poll();
        if step % 60 == 30 && k < changes.len() {
            edit(&mut f, Edit::Chain { track: t.clone(), change: changes[k].clone() });
            e.load(&f.dir, &f.doc.project).unwrap();
            k += 1;
        }
        std::thread::sleep(std::time::Duration::from_micros(200));
    }
    assert_eq!(k, changes.len());
    let (at, step) = out.windows(2).enumerate().fold((0, 0f32), |m, (i, w)| {
        let d = (w[1] - w[0]).abs();
        if d > m.1 { (i, d) } else { m }
    });
    assert!(step < 0.02, "largest step between samples: {step} at {at}: {:?}", &out[at.saturating_sub(4)..at + 4]);
    assert!(peak(&out) > 0.15, "it played");
}

/// Adds a block of a kind at the end of a track's chain, switched on, set by `tweak`.
fn add_on(f: &mut Fx2, track: &str, kind: FxKind, tweak: impl FnOnce(&mut Fx)) -> String {
    let t = tid(f, track);
    let before: Vec<String> = f.doc.project.track(&t).unwrap().chain.blocks.iter().map(|b| b.id.clone()).collect();
    edit(f, Edit::Chain { track: t.clone(), change: FxOp::Add { kind, at: None } });
    let mut tr = f.doc.project.track(&t).unwrap().clone();
    let b = tr.chain.blocks.iter_mut().find(|b| !before.contains(&b.id)).unwrap();
    let id = b.id.clone();
    b.fx.set_bypass(false);
    tweak(&mut b.fx);
    f.doc.transact(Phase::Mix, "block", vec![Command::SetTrack { track: Box::new(tr) }], None).unwrap();
    id
}

/// req: FX-24, FX-13, FX-14, FX-15, FX-16, FX-17, FX-18, FX-19, FX-20, FX-21
#[test]
fn every_new_block_at_its_neutral_position_leaves_the_track_as_it_is() {
    let n = RATE as usize / 2;
    let st: Vec<f32> = noise(n, 4, 0.2).iter().zip(noise(n, 5, 0.2)).flat_map(|(a, b)| [*a, b]).collect();
    let mut f = fixture(&[("A", 2, st), ("B", 1, sine(n, 220.0, 0.2))]);
    mute_returns(&mut f);
    let end = n as i64;
    let plain = render(&f, end, 4096);
    // Each kind in its neutral position (FX-24): the render does not move, also when the block adds latency.
    add_on(&mut f, "A", FxKind::Gate, |x| {
        if let Fx::Gate(g) = x {
            g.range_db = 0.0
        }
    });
    add_on(&mut f, "A", FxKind::Deesser, |x| {
        if let Fx::Deesser(d) = x {
            d.max_reduction_db = 0.0
        }
    });
    add_on(&mut f, "A", FxKind::DynamicEq, |x| {
        if let Fx::DynamicEq(d) = x {
            d.bands[0].range_db = 0.0
        }
    });
    add_on(&mut f, "A", FxKind::Transient, |x| {
        if let Fx::Transient(t) = x {
            t.attack_db = 0.0
        }
    });
    add_on(&mut f, "A", FxKind::Exciter, |x| {
        if let Fx::Exciter(e) = x {
            e.amount = 0.0
        }
    });
    add_on(&mut f, "A", FxKind::BassEnhancer, |x| {
        if let Fx::BassEnhancer(b) = x {
            b.mix = 0.0
        }
    });
    add_on(&mut f, "B", FxKind::Width, |x| {
        if let Fx::Width(w) = x {
            w.width = 1.0
        }
    });
    add_on(&mut f, "B", FxKind::Modulation, |x| {
        if let Fx::Modulation(m) = x {
            m.mix = 0.0
        }
    });
    add_on(&mut f, "B", FxKind::Tremolo, |x| {
        if let Fx::Tremolo(t) = x {
            t.depth = 0.0
        }
    });
    add_on(&mut f, "B", FxKind::Harmonizer, |x| {
        if let Fx::Harmonizer(h) = x {
            h.mix = 0.0
        }
    });
    add_on(&mut f, "B", FxKind::Doubler, |x| {
        if let Fx::Doubler(d) = x {
            d.mix = 0.0
        }
    });
    let neutral = render(&f, end, 4096);
    assert!(max_diff(&plain, &neutral) <= 1e-6, "neutral blocks moved the render: {}", max_diff(&plain, &neutral));
}

/// req: FX-24, ENG-4, FX-10
#[test]
fn a_chain_with_every_new_block_at_work_does_not_depend_on_the_block_size() {
    let n = RATE as usize;
    let vox: Vec<f32> =
        noise(n, 6, 0.3).iter().enumerate().map(|(i, v)| if (i / 6000) % 2 == 0 { *v } else { v * 0.02 }).collect();
    let mut f = fixture(&[("Vox", 1, vox), ("Kick", 1, noise(n, 7, 0.4))]);
    for k in [FxKind::Gate, FxKind::Deesser, FxKind::DynamicEq, FxKind::Transient, FxKind::Clipper, FxKind::Multiband] {
        add_on(&mut f, "Vox", k, |_| {});
    }
    for k in
        [FxKind::Exciter, FxKind::BassEnhancer, FxKind::Modulation, FxKind::Width, FxKind::Tremolo, FxKind::Harmonizer]
    {
        add_on(&mut f, "Kick", k, |x| {
            if let Fx::Harmonizer(h) = x {
                h.mix = 0.4
            }
        });
    }
    // Keyed: the vocal's dynamic EQ listens to the kick.
    let (v, k) = (tid(&f, "Vox"), tid(&f, "Kick"));
    let mut tr = f.doc.project.track(&v).unwrap().clone();
    if let Fx::DynamicEq(d) = &mut tr.chain.block_mut("deq").unwrap().fx {
        d.bands[0].sidechain = Some(k);
        d.bands[0].range_db = -6.0;
    }
    f.doc.transact(Phase::Mix, "key", vec![Command::SetTrack { track: Box::new(tr) }], None).unwrap();
    let end = n as i64;
    let a = render(&f, end, 4096);
    let b = render(&f, end, 64);
    let c = render(&f, end, 777);
    assert!(max_diff(&a, &b) <= 1e-6 && max_diff(&a, &c) <= 1e-6, "{} {}", max_diff(&a, &b), max_diff(&a, &c));
    assert!(a.left.iter().chain(&a.right).all(|x| x.is_finite() && x.abs() < 8.0));
}

/// req: FX-20, FX-23, FX-24
#[test]
fn modulation_tremolo_and_tape_follow_the_song_position() {
    let n = 6 * RATE as usize;
    let mut f = fixture(&[("Keys", 1, sine(n, 330.0, 0.3))]);
    add_on(&mut f, "Keys", FxKind::Modulation, |x| {
        if let Fx::Modulation(m) = x {
            m.mix = 0.5
        }
    });
    add_on(&mut f, "Keys", FxKind::Tremolo, |x| {
        if let Fx::Tremolo(t) = x {
            t.depth = 0.6
        }
    });
    // (The doubler's pitch shifter sweeps from where the render starts: FX-20 binds only modulation and tremolo.)
    // The delay return in tape mode, with wow.
    let k = tid(&f, "Keys");
    let r2 = tid(&f, "Delay");
    let cmds = edit::set_send(&f.doc.project, &k, &r2, Some(-10.0)).unwrap();
    f.doc.transact(Phase::Mix, "send", cmds, None).unwrap();
    let mut d = f.doc.project.track(&r2).unwrap().clone();
    if let Some(stemwise_core::model::ReturnFx::Delay { character, wow, time_ms, .. }) = &mut d.ret {
        *character = stemwise_core::model::DelayCharacter::Tape;
        *wow = 1.0;
        *time_ms = 120.0;
    }
    f.doc.transact(Phase::Mix, "tape", vec![Command::SetTrack { track: Box::new(d) }], None).unwrap();
    let full = render(&f, n as i64, 4096);
    let start = 4 * i64::from(RATE);
    let part = render_range(&f.dir, &f.doc.project, start, n as i64, 4096, &mut |_| true).unwrap();
    // From 4.5 s on (the delay lines have filled with what the full render had), the same samples.
    let from = RATE as usize / 2;
    let d =
        part.left[from..].iter().zip(&full.left[start as usize + from..]).fold(0f32, |m, (a, b)| m.max((a - b).abs()));
    // Phases accumulate per sample from the anchor of each render: after seconds the rounding of the two sums differs
    // by a few 1e-5 of the signal (−80 dB and below), far under anything audible, and never grows into a drift.
    assert!(d <= 1e-4, "{d}");
}

/// With the rate automated, the phase is still a function of the position: a render that starts after the rate
/// changed gives the samples of the full render (the engine hands the block the cycles run since the song's start).
/// req: FX-20, AUT-1
#[test]
fn an_automated_rate_keeps_the_phase_bound_to_the_song_position() {
    let n = 6 * RATE as usize;
    let mut f = fixture(&[("Keys", 1, sine(n, 330.0, 0.3))]);
    mute_returns(&mut f);
    let m = add_on(&mut f, "Keys", FxKind::Modulation, |x| {
        if let Fx::Modulation(m) = x {
            m.mix = 0.5
        }
    });
    let t = add_on(&mut f, "Keys", FxKind::Tremolo, |x| {
        if let Fx::Tremolo(t) = x {
            t.depth = 0.8
        }
    });
    // A ramp, a jump and a stretch at another rate, all before the second render starts; points off the grid of 32.
    bypass_lane(
        &mut f,
        "Keys",
        &format!("{t}.rate"),
        &[(0, 0.7), (48_011, 0.7), (120_005, 6.3), (120_005, 2.0), (150_000, 3.1)],
    );
    bypass_lane(&mut f, "Keys", &format!("{m}.rate"), &[(10_000, 0.3), (100_000, 4.0)]);
    let full = render(&f, n as i64, 4096);
    for start in [4 * i64::from(RATE), 4 * i64::from(RATE) + 17, 70_001] {
        let part = render_range(&f.dir, &f.doc.project, start, n as i64, 777, &mut |_| true).unwrap();
        // The chorus lines fill in 50 ms; from 0.5 s on, the same samples (tolerance: see the test above).
        let from = RATE as usize / 2;
        let d = part.left[from..]
            .iter()
            .zip(&full.left[start as usize + from..])
            .chain(part.right[from..].iter().zip(&full.right[start as usize + from..]))
            .fold(0f32, |m, (a, b)| m.max((a - b).abs()));
        assert!(d <= 1e-4, "from {start}: {d}");
    }
}

/// A feed of a 300 Hz tone with a loud 8 kHz component, for the graph directly.
struct Tones(usize);
impl stemwise_engine::Feed for Tones {
    fn fill(&mut self, _: usize, out: &mut [f32]) {
        for v in out {
            let t = self.0 as f64 / f64::from(RATE);
            *v = (0.3 * (2.0 * std::f64::consts::PI * 300.0 * t).sin()
                + 0.3 * (2.0 * std::f64::consts::PI * 8000.0 * t).sin()) as f32;
            self.0 += 1;
        }
    }
}

/// req: FX-14
#[test]
fn listening_to_a_de_esser_is_the_engines_only_and_hears_the_band_alone() {
    let mut f = fixture(&[("Vox", 1, sine(48000, 300.0, 0.3))]);
    mute_returns(&mut f);
    let id = add_on(&mut f, "Vox", FxKind::Deesser, |x| {
        if let Fx::Deesser(d) = x {
            d.threshold_db = -40.0
        }
    });
    let t = tid(&f, "Vox");
    let mut g = stemwise_engine::Graph::build(&f.doc.project, f64::from(RATE)).unwrap();
    g.reset(0);
    let run = |g: &mut stemwise_engine::Graph, from: i64| {
        let (mut l, mut r) = (vec![0f32; 4800], vec![0f32; 4800]);
        g.process(from, &mut l, &mut r, &mut Tones(from as usize));
        // The 300 Hz part: correlate with the tone over whole cycles.
        let lo: f64 = l
            .iter()
            .enumerate()
            .map(|(i, v)| {
                f64::from(*v) * (2.0 * std::f64::consts::PI * 300.0 * (from as f64 + i as f64) / 48000.0).sin()
            })
            .sum::<f64>()
            .abs();
        lo / l.len() as f64
    };
    run(&mut g, 0);
    let normal = run(&mut g, 4800);
    g.set_param(&t, &format!("{id}.listen"), 1.0);
    run(&mut g, 9600);
    let listening = run(&mut g, 14400);
    assert!(listening < normal * 0.25, "listening keeps only the band above the frequency: {listening} vs {normal}");
    // It is not a parameter of the document: nothing can store or export it.
    assert!(format!("{id}.listen").parse::<stemwise_core::params::ParamId>().is_err());
}
