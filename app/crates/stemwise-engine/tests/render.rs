//! Engine tests on synthetic projects (ENG, AUT, TRK, FX): packages are built in `target/tmp` and removed at the
//! end, even on failure.

use std::path::{Path, PathBuf};

use stemwise_core::command::Command;
use stemwise_core::edit;
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::model::{Author, AutomationLane, AutomationPoint, Output, Project, Sidechain};
use stemwise_core::package;
use stemwise_core::role::Role;
use stemwise_core::wav;
use stemwise_engine::offline::{Rendered, render_range};

const RATE: u32 = 48000;

struct Fixture {
    _tmp: tempfile::TempDir,
    dir: PathBuf,
    doc: Document,
}

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/// A project with one audio track per signal (mono or stereo interleaved).
fn fixture(signals: &[(&str, u16, Vec<f32>)]) -> Fixture {
    let t = tmp();
    let src = t.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (dir, p) = package::create(t.path(), "Engine").unwrap();
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
    Fixture { _tmp: t, dir, doc }
}

fn sine(n: usize, freq: f64, amp: f64) -> Vec<f32> {
    (0..n).map(|i| (amp * (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(RATE)).sin()) as f32).collect()
}

fn noise(n: usize, seed: u32, amp: f64) -> Vec<f32> {
    stemwise_dsp::testsignal::xorshift32(seed, n).iter().map(|v| (v * amp) as f32).collect()
}

fn render(f: &Fixture, end: i64, block: usize) -> Rendered {
    render_range(&f.dir, &f.doc.project, 0, end, block, &mut |_| true).unwrap()
}

fn tid(p: &Project, name: &str) -> String {
    p.tracks.iter().find(|t| t.name == name).unwrap().id.clone()
}

fn set(f: &mut Fixture, track: &str, param: &str, value: f64) {
    let t = tid(&f.doc.project, track);
    f.doc.transact(Phase::Mix, "set", vec![Command::SetParam { track: t, param: param.into(), value }], None).unwrap();
}

fn rich_project() -> Fixture {
    let n = 3 * RATE as usize;
    let st: Vec<f32> = noise(n, 7, 0.2).iter().zip(noise(n, 8, 0.2)).flat_map(|(a, b)| [*a, b]).collect();
    let mut f = fixture(&[
        ("Kick", 1, noise(n, 1, 0.5)),
        ("Bass", 1, sine(n, 55.0, 0.4)),
        ("Pad", 2, st),
        ("Vox", 1, sine(n, 440.0, 0.3)),
    ]);
    // Full chain on the vocal, sends, a group with nesting, a sidechain.
    for (p, v) in [
        ("bypass.hpf", 0.0),
        ("hpf.freq", 120.0),
        ("bypass.comp", 0.0),
        ("comp.threshold", -30.0),
        ("bypass.sat", 0.0),
        ("sat.drive", 9.0),
        ("pan", -0.4),
        ("fader", -2.0),
    ] {
        set(&mut f, "Vox", p, v);
    }
    let p = &f.doc.project;
    let (r1, r2) = (
        p.tracks.iter().find(|t| t.name == "Reverb").unwrap().id.clone(),
        p.tracks.iter().find(|t| t.name == "Delay").unwrap().id.clone(),
    );
    let vox = tid(p, "Vox");
    let mut cmds = edit::set_send(p, &vox, &r1, Some(-10.0)).unwrap();
    cmds.extend(edit::set_send(p, &tid(p, "Pad"), &r2, Some(-12.0)).unwrap());
    f.doc.transact(Phase::Mix, "sends", cmds, None).unwrap();
    let (kick, bass) = (tid(&f.doc.project, "Kick"), tid(&f.doc.project, "Bass"));
    let cmds = edit::add_group(&mut f.doc.project, "Rhythm", &[kick.clone(), bass.clone()]).unwrap();
    f.doc.transact(Phase::Mix, "group", cmds, None).unwrap();
    let mut b = f.doc.project.track(&bass).unwrap().clone();
    b.chain.comp_mut().bypass = false;
    b.chain.comp_mut().sidechain = Some(Sidechain { source: kick, hpf_hz: 60.0 });
    f.doc.transact(Phase::Mix, "sc", vec![Command::SetTrack { track: Box::new(b) }], None).unwrap();
    f
}

/// req: ENG-4
#[test]
fn render_is_deterministic_and_independent_of_the_block_size() {
    let f = rich_project();
    let end = 3 * i64::from(RATE);
    let a = render(&f, end, 64);
    let b = render(&f, end, 4096);
    let c = render(&f, end, 4096);
    assert_eq!(b.hash(), c.hash(), "same project, same hash");
    let diff =
        a.left.iter().zip(&b.left).chain(a.right.iter().zip(&b.right)).fold(0f32, |m, (x, y)| m.max((x - y).abs()));
    assert!(diff <= 1e-6, "64 vs 4096 frames: peak difference {diff}");
    assert!(a.left.iter().any(|v| v.abs() > 0.01));
    // Odd block sizes too.
    let d = render(&f, end, 1000);
    assert_eq!(d.hash(), b.hash());
}

/// req: ENG-5, TRK-3
#[test]
fn latency_is_compensated_across_groups_and_removed_from_the_render() {
    // The same impulse on a track straight to the bus and on one inside two nested groups: both arrive at the
    // clip position, on the same sample.
    let n = RATE as usize;
    let mut imp = vec![0f32; n];
    imp[1000] = 0.5;
    let mut f = fixture(&[("A", 1, imp.clone()), ("B", 1, imp)]);
    let b = tid(&f.doc.project, "B");
    let cmds = edit::add_group(&mut f.doc.project, "G1", std::slice::from_ref(&b)).unwrap();
    f.doc.transact(Phase::Mix, "g1", cmds, None).unwrap();
    let g1 = tid(&f.doc.project, "G1");
    let cmds = edit::add_group(&mut f.doc.project, "G2", &[g1]).unwrap();
    f.doc.transact(Phase::Mix, "g2", cmds, None).unwrap();
    set(&mut f, "A", "pan", -1.0);
    set(&mut f, "B", "pan", 1.0);
    let out = render(&f, n as i64, 512);
    let peak_at = |x: &[f32]| x.iter().enumerate().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs())).unwrap().0;
    assert_eq!(peak_at(&out.left), 1000);
    assert_eq!(peak_at(&out.right), 1000);
}

/// req: AUT-4, AUT-1
#[test]
fn automation_ramps_per_sample_and_matches_playback_code() {
    let n = RATE as usize;
    let mut f = fixture(&[("Dc", 1, vec![0.5; n])]);
    let t = tid(&f.doc.project, "Dc");
    let mut tr = f.doc.project.track(&t).unwrap().clone();
    // Fader from −20 dB to 0 dB over 0.5 s, pan hard left.
    tr.automation.push(AutomationLane {
        param: "fader".into(),
        points: vec![AutomationPoint { frame: 0, value: -20.0 }, AutomationPoint { frame: 24000, value: 0.0 }],
        author: Author::User,
        visible: true,
    });
    tr.chain.pan = -1.0;
    f.doc.transact(Phase::Mix, "auto", vec![Command::SetTrack { track: Box::new(tr) }], None).unwrap();
    let out = render(&f, n as i64, 333);
    // Expected gain: linear interpolation of the dB value at the 32-sample grid, then a per-sample linear ramp
    // of the linear gain between grid points. Tolerance: f32 rounding (1e-6 relative).
    for i in (0..30000).step_by(7) {
        let cell = (i / 32) * 32;
        let v = |fr: usize| -20.0 + 20.0 * (fr.min(24000) as f64 / 24000.0);
        let (ga, gb) = (10f64.powf(v(cell) / 20.0), 10f64.powf(v(cell + 32) / 20.0));
        let g = ga + (gb - ga) * ((i - cell) as f64 / 32.0);
        let want = 0.5 * g;
        assert!((f64::from(out.left[i]) - want).abs() < 1e-6 * want.max(0.01), "sample {i}: {} vs {want}", out.left[i]);
    }
    // No steps: consecutive samples differ by less than the per-sample ramp increment allows.
    let max_step = out.left.windows(2).take(24000).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
    assert!(max_step < 1e-3, "{max_step}");
}

/// req: AUT-6
#[test]
fn rapid_filter_eq_and_compressor_sweeps_stay_stable_and_click_free() {
    let n = 2 * RATE as usize;
    let mut f = fixture(&[("S", 1, sine(n, 200.0, 0.5))]);
    let t = tid(&f.doc.project, "S");
    let mut tr = f.doc.project.track(&t).unwrap().clone();
    tr.chain.hpf_mut().bypass = false;
    tr.chain.eq_mut().bands[1].enabled = true;
    tr.chain.comp_mut().bypass = false;
    let lane = |param: &str, lo: f64, hi: f64| AutomationLane {
        param: param.into(),
        // A full-range zig-zag every 20 ms: far steeper than any musical automation.
        points: (0..100).map(|k| AutomationPoint { frame: k * 960, value: if k % 2 == 0 { lo } else { hi } }).collect(),
        author: Author::User,
        visible: true,
    };
    tr.automation = vec![
        lane("hpf.freq", 20.0, 2000.0),
        lane("eq.2.freq", 100.0, 8000.0),
        lane("eq.2.gain", -18.0, 18.0),
        lane("eq.2.q", 0.1, 10.0),
        lane("comp.threshold", -60.0, 0.0),
        lane("comp.ratio", 1.0, 20.0),
    ];
    f.doc.transact(Phase::Mix, "auto", vec![Command::SetTrack { track: Box::new(tr) }], None).unwrap();
    let out = render(&f, n as i64, 256);
    assert!(out.left.iter().all(|v| v.is_finite() && v.abs() < 4.0), "unstable");
    // Click detector: the second difference of a 200 Hz sine at 0.5 is about 0.5·(2π·200/48000)² ≈ 3.4e-4; a
    // coefficient jump without interpolation shows as spikes around 1e-1. Allow 0.05.
    let d2 = out.left.windows(3).map(|w| (w[2] - 2.0 * w[1] + w[0]).abs()).fold(0f32, f32::max);
    assert!(d2 < 0.05, "click: second difference {d2}");
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt()
}

fn ret(p: &Project, name: &str) -> String {
    p.tracks.iter().find(|t| t.name == name).unwrap().id.clone()
}

fn diff(a: &Rendered, b: &Rendered) -> Vec<f32> {
    a.left.iter().zip(&b.left).map(|(x, y)| x - y).collect()
}

/// req: TRK-5, FX-5, FX-6, FX-8
#[test]
fn post_fader_sends_feed_both_returns_and_the_bus_has_only_a_fader() {
    let n = RATE as usize;
    let mut imp = vec![0f32; n];
    imp[1000] = 0.5;
    let mut f = fixture(&[("A", 1, sine(n, 1000.0, 0.3)), ("Imp", 1, imp)]);
    let (r1, r2) = (ret(&f.doc.project, "Reverb"), ret(&f.doc.project, "Delay"));
    let (a, i) = (tid(&f.doc.project, "A"), tid(&f.doc.project, "Imp"));
    set(&mut f, "Imp", "mute", 1.0);
    let dry = render(&f, n as i64, 512);
    // Reverb send: the return adds signal (FX-5).
    let cmds = edit::set_send(&f.doc.project, &a, &r1, Some(-6.0)).unwrap();
    f.doc.transact(Phase::Mix, "send", cmds, None).unwrap();
    let wet = render(&f, n as i64, 512);
    assert!(rms(&diff(&wet, &dry)[12000..]) > 0.01, "the reverb return produces signal");
    // Post-fader: the fader at −∞ silences the dry signal and the send.
    set(&mut f, "A", "fader", -144.0);
    assert!(rms(&render(&f, n as i64, 512).left[12000..]) < 1e-6, "post-fader send");
    // Delay send (FX-6): an impulse comes back after the delay time (375 ms), aligned with the timeline.
    set(&mut f, "Imp", "mute", 0.0);
    let cmds = edit::set_send(&f.doc.project, &i, &r2, Some(-6.0)).unwrap();
    f.doc.transact(Phase::Mix, "send", cmds, None).unwrap();
    let out = render(&f, n as i64, 512);
    let echo = (1000 + 18_000 - 200..1000 + 18_000 + 200)
        .max_by(|&x, &y| out.left[x].abs().total_cmp(&out.left[y].abs()))
        .unwrap();
    assert!((echo as i64 - 19_000).abs() <= 2, "echo at {echo}");
    assert!(out.left[echo].abs() > 0.05);
    // The mix bus has a fader and nothing else (FX-8): −6 dB on the bus scales the output exactly.
    let full = render(&f, n as i64, 512);
    let bus = vec![Command::SetMixBus { bus: stemwise_core::model::MixBus { fader_db: -6.0 } }];
    f.doc.transact(Phase::Mix, "bus", bus, None).unwrap();
    let half = render(&f, n as i64, 512);
    let ratio = rms(&half.left[12000..]) / rms(&full.left[12000..]);
    assert!((20.0 * ratio.log10() + 6.0).abs() < 0.01);
}

/// req: TRK-2
#[test]
fn solo_in_place_keeps_groups_members_and_returns_and_a_soloed_return_hears_its_sends() {
    let n = RATE as usize;
    let mut f =
        fixture(&[("A", 1, sine(n, 1000.0, 0.3)), ("B", 1, sine(n, 300.0, 0.3)), ("C", 1, sine(n, 2000.0, 0.3))]);
    let (a, b, c) = (tid(&f.doc.project, "A"), tid(&f.doc.project, "B"), tid(&f.doc.project, "C"));
    let r1 = ret(&f.doc.project, "Reverb");
    let cmds = edit::add_group(&mut f.doc.project, "G", std::slice::from_ref(&c)).unwrap();
    f.doc.transact(Phase::Mix, "g", cmds, None).unwrap();
    let g = tid(&f.doc.project, "G");
    let cmds = edit::set_send(&f.doc.project, &b, &r1, Some(-6.0)).unwrap();
    f.doc.transact(Phase::Mix, "send", cmds, None).unwrap();
    let with = |solo: &[&str]| {
        let mut p = f.doc.project.clone();
        for s in solo {
            p.track_mut(s).unwrap().solo = true;
        }
        render_range(&f.dir, &p, 0, n as i64, 512, &mut |_| true).unwrap()
    };
    // Goertzel-like level of one frequency in the left channel.
    let tone = |r: &Rendered, freq: f64| {
        let x = &r.left[12000..];
        let (mut re, mut im) = (0.0, 0.0);
        for (k, v) in x.iter().enumerate() {
            let ph = std::f64::consts::TAU * freq * (k + 12000) as f64 / f64::from(RATE);
            re += f64::from(*v) * ph.cos();
            im += f64::from(*v) * ph.sin();
        }
        2.0 * (re * re + im * im).sqrt() / x.len() as f64
    };
    // Solo B: A and C are silent, B is there, and the reverb return follows (it carries B's send).
    let r = with(&[&b]);
    assert!(tone(&r, 1000.0) < 1e-4 && tone(&r, 2000.0) < 1e-4 && tone(&r, 300.0) > 0.05);
    let no_send = {
        let mut p = f.doc.project.clone();
        p.track_mut(&b).unwrap().solo = true;
        p.track_mut(&b).unwrap().sends.clear();
        render_range(&f.dir, &p, 0, n as i64, 512, &mut |_| true).unwrap()
    };
    assert!(rms(&diff(&r, &no_send)[12000..]) > 0.005, "returns follow");
    // Solo the group: its member is audible, the others are not.
    let r = with(&[&g]);
    assert!(tone(&r, 2000.0) > 0.05 && tone(&r, 1000.0) < 1e-4 && tone(&r, 300.0) < 1e-4);
    // Solo the reverb return: B's dry signal is silent but its send still feeds the reverb.
    let r = with(&[&r1]);
    assert!(rms(&r.left[12000..]) > 0.005 && tone(&r, 1000.0) < 1e-4);
    let _ = a;
}

/// req: FX-3, ENG-5
#[test]
fn the_sidechain_key_ducks_its_target_and_is_latency_compensated() {
    // A: clicks every 0.25 s, hard left, inside a group (its output arrives later); B: DC, hard right, keyed by A.
    let n = RATE as usize;
    let mut clicks = vec![0f32; n];
    for k in (6000..n).step_by(12_000) {
        clicks[k] = 1.0;
    }
    let mut f = fixture(&[("A", 1, clicks), ("B", 1, vec![0.5; n])]);
    let (a, b) = (tid(&f.doc.project, "A"), tid(&f.doc.project, "B"));
    let cmds = edit::add_group(&mut f.doc.project, "G", std::slice::from_ref(&a)).unwrap();
    f.doc.transact(Phase::Mix, "g", cmds, None).unwrap();
    set(&mut f, "A", "pan", -1.0);
    set(&mut f, "B", "pan", 1.0);
    let base = render(&f, n as i64, 512);
    let mut tb = f.doc.project.track(&b).unwrap().clone();
    *tb.chain.comp_mut() = stemwise_core::model::Compressor {
        bypass: false,
        threshold_db: -40.0,
        ratio: 20.0,
        attack_ms: 0.1,
        release_ms: 30.0,
        knee_db: 0.0,
        makeup_db: 0.0,
        mix: 1.0,
        sidechain: Some(Sidechain { source: a.clone(), hpf_hz: 20.0 }),
    };
    f.doc.transact(Phase::Mix, "sc", vec![Command::SetTrack { track: Box::new(tb) }], None).unwrap();
    let out = render(&f, n as i64, 512);
    // Without the key B stays at its level; with it, B ducks right at each click, on the same sample.
    let steady = base.right[20_000];
    for click in [18_000usize, 30_000, 42_000] {
        assert_eq!(
            out.left
                .iter()
                .enumerate()
                .skip(click - 100)
                .take(200)
                .max_by(|x, y| x.1.abs().total_cmp(&y.1.abs()))
                .unwrap()
                .0,
            click
        );
        let duck = (click - 100..click + 100).find(|&k| out.right[k] < 0.5 * steady).unwrap();
        assert!((duck as i64 - click as i64).abs() <= 3, "duck at {duck}, click at {click}");
        assert!(out.right[click + 200] < 0.5 * steady && base.right[click + 200] == steady);
    }
}

/// req: AUT-1, TRK-5
#[test]
fn send_mute_bypass_and_return_lanes_are_applied() {
    let n = RATE as usize;
    let mut imp = vec![0f32; n];
    imp[1000] = 0.5;
    imp[30_000] = 0.5;
    let mut f = fixture(&[("S", 1, sine(n, 50.0, 0.4)), ("Imp", 1, imp)]);
    let (s_id, i) = (tid(&f.doc.project, "S"), tid(&f.doc.project, "Imp"));
    let (r1, r2) = (ret(&f.doc.project, "Reverb"), ret(&f.doc.project, "Delay"));
    let lane = |param: &str, pts: &[(i64, f64)]| AutomationLane {
        param: param.into(),
        points: pts.iter().map(|&(frame, value)| AutomationPoint { frame, value }).collect(),
        author: Author::User,
        visible: true,
    };
    // Bypass lane on the high-pass (1 kHz) of a 50 Hz sine: bypassed (loud) then active (quiet); mute lane at 0.75 s.
    let mut ts = f.doc.project.track(&s_id).unwrap().clone();
    *ts.chain.hpf_mut() =
        stemwise_core::model::Filter { bypass: true, freq_hz: 1000.0, resonance: std::f64::consts::FRAC_1_SQRT_2 };
    ts.automation = vec![lane("bypass.hpf", &[(0, 1.0), (24_000, 0.0)]), lane("mute", &[(0, 0.0), (36_000, 1.0)])];
    ts.chain.pan = -1.0;
    // Send lane: the reverb send of S opens at 0.5 s.
    ts.sends.push(stemwise_core::model::Send { target: r1.clone(), level_db: -144.0 });
    ts.automation.push(lane(&format!("send.{r1}"), &[(0, -144.0), (24_000, -144.0), (24_032, -6.0)]));
    let mut ti = f.doc.project.track(&i).unwrap().clone();
    // Hard right; the dry impulses (1000, 30 000) are outside the windows searched for the echoes below.
    ti.chain.pan = 1.0;
    ti.sends.push(stemwise_core::model::Send { target: r2.clone(), level_db: -6.0 });
    let mut td = f.doc.project.track(&r2).unwrap().clone();
    // Return lane: the delay time goes from 100 ms to 300 ms at 0.4 s.
    td.automation = vec![lane("dly.time", &[(0, 100.0), (19_200, 100.0), (19_232, 300.0)])];
    f.doc
        .transact(
            Phase::Mix,
            "lanes",
            vec![ts, ti, td].into_iter().map(|t| Command::SetTrack { track: Box::new(t) }).collect(),
            None,
        )
        .unwrap();
    let out = render(&f, n as i64, 512);
    // Left: S only (and its reverb). Bypassed until 0.5 s: full level; filtered after; muted from 0.75 s.
    let before = rms(&out.left[4_000..20_000]);
    let filtered = rms(&out.left[30_000..34_000]);
    assert!(before > 0.25 && filtered < 0.05, "{before} {filtered}");
    assert!(rms(&out.left[37_000..]) < 1e-3, "muted (the reverb tail of a 50 Hz tone is negligible)");
    // The reverb send opens at 0.5 s: before it, the right side of the mix carries no reverb of S.
    // Right: the delayed impulse comes back after 100 ms before the lane change and 300 ms after it.
    let peak_near =
        |k: usize| (k - 300..k + 300).max_by(|&x, &y| out.right[x].abs().total_cmp(&out.right[y].abs())).unwrap();
    assert!((peak_near(1000 + 4800) as i64 - 5800).abs() <= 2);
    assert!((peak_near(30_000 + 14_400) as i64 - 44_400).abs() <= 2);
}

/// req: AUT-6
#[test]
fn filter_automation_follows_the_curve_within_a_cell() {
    // A 100 Hz sine through a high-pass swept linearly from 20 Hz to 400 Hz over 2 s. The level after the filter
    // must follow |H(100 Hz; fc(t))| of the coefficients at the current cell (0.5 dB): with updates once per 4096
    // samples, fc would lag up to 85 ms (≈16 Hz near 100 Hz, more than 1 dB).
    let n = 2 * RATE as usize;
    let mut f = fixture(&[("S", 1, sine(n, 100.0, 0.5))]);
    let s_id = tid(&f.doc.project, "S");
    let mut t = f.doc.project.track(&s_id).unwrap().clone();
    *t.chain.hpf_mut() =
        stemwise_core::model::Filter { bypass: false, freq_hz: 20.0, resonance: std::f64::consts::FRAC_1_SQRT_2 };
    t.chain.pan = -1.0;
    t.automation = vec![AutomationLane {
        param: "hpf.freq".into(),
        points: vec![AutomationPoint { frame: 0, value: 20.0 }, AutomationPoint { frame: n as i64, value: 400.0 }],
        author: Author::User,
        visible: true,
    }];
    f.doc.transact(Phase::Mix, "auto", vec![Command::SetTrack { track: Box::new(t) }], None).unwrap();
    let out = render(&f, n as i64, 4096);
    for centre in (14_400..n - 2_400).step_by(4_800) {
        let peak = out.left[centre - 240..centre + 240].iter().fold(0f32, |m, v| m.max(v.abs()));
        let fc = 20.0 + 380.0 * centre as f64 / n as f64;
        let k = stemwise_dsp::biquad::Coeffs::design(
            stemwise_dsp::biquad::Kind::HighPass,
            fc,
            0.0,
            std::f64::consts::FRAC_1_SQRT_2,
            f64::from(RATE),
        );
        let want = 20.0 * (0.5 * 10f64.powf(k.response_db(100.0, f64::from(RATE)) / 20.0)).log10();
        let got = 20.0 * f64::from(peak).log10();
        assert!(
            (got - want).abs() < 0.5,
            "t = {:.2} s, fc {fc:.0} Hz: {got:.2} dB vs {want:.2} dB",
            centre as f64 / 48000.0
        );
    }
}

/// req: TRK-1, TRK-3, FX-1, FX-3
#[test]
fn a_rich_project_renders_finite_audio_with_its_groups_sidechain_and_returns() {
    let f = rich_project();
    let p = &f.doc.project;
    assert!(p.tracks.iter().any(|t| t.is_group()));
    assert!(matches!(p.track(&tid(p, "Kick")).unwrap().output, Output::Group(_)));
    let out = render(&f, 3 * i64::from(RATE), 4096);
    assert!(out.left.iter().chain(&out.right).all(|v| v.is_finite()));
    let peak = out.left.iter().fold(0f32, |m, v| m.max(v.abs()));
    assert!(peak > 0.05 && peak < 4.0, "{peak}");
}

/// req: FX-5, FX-9, FX-3, ENG-4
#[test]
fn the_three_reverbs_and_the_compressor_mix_render_the_same_at_any_block_size() {
    use stemwise_core::edit::Edit;
    use stemwise_core::model::ReverbAlgorithm;
    let n = 2 * RATE as usize;
    let mut f = fixture(&[("Drums", 1, noise(n, 3, 0.4)), ("Vox", 1, sine(n, 440.0, 0.3))]);
    let (drums, vox) = (tid(&f.doc.project, "Drums"), tid(&f.doc.project, "Vox"));
    // A plate and a room next to the hall of a new project, each with a send, modulation on.
    for a in [ReverbAlgorithm::Plate, ReverbAlgorithm::Room] {
        let cmds = edit::run(&mut f.doc.project, &Edit::AddReturn { reverb: true, algorithm: Some(a) }).unwrap();
        f.doc.transact(Phase::Mix, "return", cmds, None).unwrap();
    }
    let reverbs: Vec<(String, ReverbAlgorithm)> =
        f.doc.project.tracks.iter().filter_map(|t| Some((t.id.clone(), t.ret.as_ref()?.algorithm()?))).collect();
    assert_eq!(reverbs.len(), 3);
    for (i, (r, _)) in reverbs.iter().enumerate() {
        let from = if i == 1 { &vox } else { &drums };
        let cmds = edit::set_send(&f.doc.project, from, r, Some(-6.0)).unwrap();
        f.doc.transact(Phase::Mix, "send", cmds, None).unwrap();
        for (param, value) in [("rev.modulation", 0.8), ("rev.decay", 1.5)] {
            let c = Command::SetParam { track: r.clone(), param: param.into(), value };
            f.doc.transact(Phase::Mix, "set", vec![c], None).unwrap();
        }
    }
    let end = n as i64;
    let dry = {
        let mut g = f.doc.project.clone();
        for t in g.tracks.iter_mut().filter(|t| t.is_return()) {
            t.mute = true;
        }
        render_range(&f.dir, &g, 0, end, 512, &mut |_| true).unwrap()
    };
    let (a, b, c) = (render(&f, end, 64), render(&f, end, 1000), render(&f, end, 4096));
    assert_eq!(a.hash(), b.hash(), "64 vs 1000 frames");
    assert_eq!(b.hash(), c.hash(), "1000 vs 4096 frames");
    assert!(rms(&diff(&a, &dry)[RATE as usize..]) > 0.01, "the returns produce signal");

    // Parallel compression: mix 0 is the uncompressed track, mix 1 the compressed one, 0.5 in between; a change
    // of algorithm is a structural change that the next render picks up.
    for t in f.doc.project.tracks.clone().iter().filter(|t| t.is_return()) {
        set(&mut f, &t.name, "mute", 1.0);
    }
    set(&mut f, "Vox", "mute", 1.0);
    for (p, v) in [("bypass.comp", 0.0), ("comp.threshold", -40.0), ("comp.ratio", 8.0)] {
        set(&mut f, "Drums", p, v);
    }
    let level = |f: &mut Fixture, mix: f64| {
        set(f, "Drums", "comp.mix", mix);
        let r = render(f, end, 512);
        assert_eq!(r.hash(), render(f, end, 4096).hash(), "mix {mix}");
        rms(&r.left[RATE as usize..])
    };
    let (wet, half, none) = (level(&mut f, 1.0), level(&mut f, 0.5), level(&mut f, 0.0));
    set(&mut f, "Drums", "bypass.comp", 1.0);
    let bypassed = rms(&render(&f, end, 512).left[RATE as usize..]);
    assert!(wet < half && half < none, "{wet} {half} {none}");
    // Mix 0 is the dry signal: within 0.1 dB of the bypassed compressor.
    assert!((20.0 * (none / bypassed).log10()).abs() < 0.1, "{none} {bypassed}");

    let hall = &reverbs[0].0;
    let cmds = edit::run(
        &mut f.doc.project,
        &Edit::SetReverbAlgorithm { track: hall.clone(), algorithm: ReverbAlgorithm::Plate },
    )
    .unwrap();
    f.doc.transact(Phase::Mix, "algorithm", cmds, None).unwrap();
    assert_eq!(f.doc.project.track(hall).unwrap().ret.as_ref().unwrap().algorithm(), Some(ReverbAlgorithm::Plate));
    f.doc.undo(Phase::Mix).unwrap();
    assert_eq!(f.doc.project.track(hall).unwrap().ret.as_ref().unwrap().algorithm(), Some(ReverbAlgorithm::Hall));
}
