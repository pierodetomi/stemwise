//! Real-time path without a device (FOUNDATION §8.4): the test drives `AudioProcessor::callback` like a sound card
//! would, with the real reader thread streaming from disk.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use stemwise_core::command::Command;
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::model::Sidechain;
use stemwise_core::{edit, package, role::Role, wav};
use stemwise_engine::offline::render_range;
use stemwise_engine::realtime::{AudioProcessor, Engine, FADE};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
thread_local!(static GUARD: Cell<bool> = const { Cell::new(false) });
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if GUARD.with(|g| g.get()) {
            ALLOCS.fetch_add(1, Ordering::SeqCst);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if GUARD.with(|g| g.get()) {
            ALLOCS.fetch_add(1, Ordering::SeqCst);
        }
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static A: Counting = Counting;

const RATE: u32 = 48000;
const BLOCK: usize = 512;
/// One step of the position-encoding ramp: exact in f32 up to 2^24 steps.
const STEP: f32 = 1.0 / 2_097_152.0;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

fn project(t: &Path, tracks: &[(&str, u16, Vec<f32>)]) -> (PathBuf, Document) {
    let src = t.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let (dir, p) = package::create(t, "Rt").unwrap();
    let mut d = Document::new(p);
    let mut choices = Vec::new();
    for (name, ch, x) in tracks {
        let path = src.join(format!("{name}.wav"));
        wav::write_f32(&path, *ch, RATE, x).unwrap();
        choices.push(ImportChoice {
            path: path.display().to_string(),
            role: Role::Other,
            name: name.to_string(),
            convert: false,
        });
    }
    let cmds = import(&dir, &mut d.project, &choices).unwrap();
    d.transact(Phase::Mix, "import", cmds, None).unwrap();
    (dir, d)
}

fn music() -> Vec<(&'static str, u16, Vec<f32>)> {
    let n = 4 * RATE as usize;
    let a: Vec<f32> = (0..n).map(|i| (0.3 * (i as f64 * 0.031).sin()) as f32).collect();
    let b: Vec<f32> = stemwise_dsp::testsignal::xorshift32(3, 2 * n).iter().map(|v| (v * 0.1) as f32).collect();
    vec![("a", 1, a), ("b", 2, b)]
}

fn id(d: &Document, name: &str) -> String {
    d.project.tracks.iter().find(|t| t.name == name).unwrap().id.clone()
}

/// Runs callbacks; `keep` gets (graph start frame, samples) of each callback that advanced by exactly one block.
fn run(ap: &mut AudioProcessor, e: &mut Engine, max: usize, mut keep: impl FnMut(i64, &[f32])) {
    let mut buf = vec![0f32; BLOCK * 2];
    for _ in 0..max {
        let before = e.shared.position.load(Ordering::Relaxed);
        ap.callback(&mut buf, 2);
        let after = e.shared.position.load(Ordering::Relaxed);
        if after - before == BLOCK as i64 {
            keep(before, &buf);
        }
        e.poll();
        std::thread::sleep(std::time::Duration::from_micros(300));
    }
}

/// req: ENG-1, ENG-4, AUT-4
#[test]
fn streamed_playback_equals_the_offline_render() {
    let t = tmp();
    let (dir, mut d) = project(t.path(), &music());
    let r1 = d.project.tracks.iter().find(|t| t.name == "Reverb").unwrap().id.clone();
    let cmds = edit::set_send(&d.project, &id(&d, "a"), &r1, Some(-12.0)).unwrap();
    d.transact(Phase::Mix, "send", cmds, None).unwrap();
    let (mut e, mut ap) = Engine::new(f64::from(RATE), 4096);
    e.load(&dir, &d.project).unwrap();
    e.play();
    let mut got: Vec<(i64, Vec<f32>)> = Vec::new();
    run(&mut ap, &mut e, 300, |pos, buf| got.push((pos, buf.to_vec())));
    assert!(got.len() > 150, "playback did not start: {}", got.len());
    let lat = e.shared.latency.load(Ordering::Relaxed);
    let off = render_range(&dir, &d.project, 0, 4 * i64::from(RATE), 1024, &mut |_| true).unwrap();
    let first = got[0].0;
    for (pos, buf) in &got {
        for i in 0..BLOCK {
            let f = pos + i as i64;
            let tl = f - lat;
            // The first FADE frames of audio after the start (after the graph's latency) are faded in (no click).
            if f < first + lat + FADE as i64 || tl < 0 || tl as usize >= off.left.len() {
                continue;
            }
            assert_eq!(buf[2 * i], off.left[tl as usize], "frame {tl}");
            assert_eq!(buf[2 * i + 1], off.right[tl as usize], "frame {tl}");
        }
    }
}

/// req: TRN-1, ENG-1, AUT-4
#[test]
fn loop_content_stays_aligned_with_the_transport() {
    // A stereo ramp whose value is its frame number: the output must be the input in played order.
    let n = 4 * RATE as usize;
    let ramp: Vec<f32> = (0..n).flat_map(|i| [i as f32 * STEP, i as f32 * STEP]).collect();
    let t = tmp();
    let (dir, d) = project(t.path(), &[("ramp", 2, ramp)]);
    let (mut e, mut ap) = Engine::new(f64::from(RATE), 4096);
    e.load(&dir, &d.project).unwrap();
    let (a, b) = (48_000i64, 60_000i64);
    let mut out: Vec<f32> = Vec::new();
    // Every buffer is kept, wraps included (the gap logic below skips silence and fades).
    let collect = |ap: &mut AudioProcessor, e: &mut Engine, k: usize, out: &mut Vec<f32>| {
        let mut buf = vec![0f32; BLOCK * 2];
        for _ in 0..k {
            ap.callback(&mut buf, 2);
            out.extend(buf.iter().step_by(2));
            e.poll();
            std::thread::sleep(std::time::Duration::from_micros(300));
        }
    };
    // Playhead past the loop end when the loop is set (the reviewed failure): plays on, no wrap.
    e.locate(150_000);
    e.play();
    collect(&mut ap, &mut e, 20, &mut out);
    e.set_loop(Some((a, b)));
    collect(&mut ap, &mut e, 20, &mut out);
    // Then into the loop: wraps from b − 1 to a exactly.
    e.locate(a);
    collect(&mut ap, &mut e, 150, &mut out);
    assert!(out.len() > 100 * BLOCK);
    let (mut wraps, mut checked) = (0, 0);
    let mut since_gap = 0usize;
    for w in out.windows(2) {
        if w[0] == 0.0 || w[1] == 0.0 {
            since_gap = 0;
            continue;
        }
        since_gap += 1;
        if since_gap <= FADE {
            continue; // faded in after a start or a seek
        }
        let (x, y) = ((w[0] / STEP).round() as i64, (w[1] / STEP).round() as i64);
        if y == x + 1 {
            checked += 1;
        } else if x == b - 1 && y == a {
            wraps += 1;
        } else {
            // Only a fade-out before a seek may break the sequence; anything else is a misalignment.
            assert!(w[1].abs() < w[0].abs(), "content jumped from frame {x} to {y}");
            since_gap = 0;
        }
    }
    assert!(wraps >= 3, "{wraps} wraps");
    assert!(checked > 50 * BLOCK);
}

/// req: ENG-3, TRK-2, FX-8
#[test]
fn parameters_mute_solo_and_bus_fader_are_smooth_and_never_rebuild_the_graph() {
    let n = 4 * RATE as usize;
    let t = tmp();
    let (dir, d) = project(t.path(), &[("dc", 2, vec![0.5; 2 * n]), ("other", 2, vec![0.25; 2 * n])]);
    let (mut e, mut ap) = Engine::new(f64::from(RATE), 4096);
    e.load(&dir, &d.project).unwrap();
    e.play();
    let mut last: Vec<f32> = Vec::new();
    run(&mut ap, &mut e, 40, |_, buf| last = buf.to_vec());
    let swaps = e.shared.swaps_taken.load(Ordering::Relaxed);
    let level = |b: &[f32]| b.iter().step_by(2).copied().fold(0f32, f32::max);
    assert!((level(&last) - 0.75).abs() < 1e-4, "{}", level(&last));
    let dc = id(&d, "dc");
    let mut stream: Vec<f32> = Vec::new();
    let mut p = d.project.clone();
    for step in 0..4 {
        match step {
            0 => e.set_param(&dc, "fader", -20.0),
            1 => e.set_param(&dc, "mute", 1.0),
            2 => {
                p.track_mut(&dc).unwrap().solo = true;
                e.set_solo(&p);
            }
            _ => e.set_bus_fader(-6.0),
        }
        let mut first = true;
        run(&mut ap, &mut e, 20, |_, buf| {
            let l: Vec<f32> = buf.iter().step_by(2).copied().collect();
            if first && step < 3 {
                // The change starts within the next buffer (ENG-3).
                assert!(l.windows(2).any(|w| w[0] != w[1]), "step {step}: no change in the first buffer");
            }
            first = false;
            stream.extend(l);
        });
    }
    // No steps: with DC inputs every change is a ramp; the largest sample-to-sample change is far below a jump.
    let max_step = stream.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
    assert!(max_step < 0.02, "step of {max_step}");
    assert_eq!(e.shared.swaps_taken.load(Ordering::Relaxed), swaps, "no rebuild for parameters, mute, solo, bus");
    // End state: dc muted and soloed, so the other track is silenced by the solo: nothing is heard.
    assert!(level(&stream[stream.len() - 1000..]) < 1e-4);
}

/// req: NFR-2, ENG-6
#[test]
fn the_callback_never_allocates_with_a_full_chain_and_messages_and_meters_flow_at_50_hz() {
    let t = tmp();
    let (dir, mut d) = project(t.path(), &music());
    let (a_id, b_id) = (id(&d, "a"), id(&d, "b"));
    let returns: Vec<String> = d.project.tracks.iter().filter(|t| t.is_return()).map(|t| t.id.clone()).collect();
    for tid in [&a_id, &b_id] {
        let mut tr = d.project.track(tid).unwrap().clone();
        tr.chain.hpf_mut().bypass = false;
        tr.chain.eq_mut().bands[1].enabled = true;
        tr.chain.eq_mut().bands[1].gain_db = 3.0;
        tr.chain.comp_mut().bypass = false;
        tr.chain.sat_mut().bypass = false;
        for r in &returns {
            tr.sends.push(stemwise_core::model::Send { target: r.clone(), level_db: -12.0 });
        }
        d.transact(Phase::Mix, "chain", vec![Command::SetTrack { track: Box::new(tr) }], None).unwrap();
    }
    let mut tb = d.project.track(&b_id).unwrap().clone();
    tb.chain.comp_mut().sidechain = Some(Sidechain { source: a_id.clone(), hpf_hz: 100.0 });
    d.transact(Phase::Mix, "sc", vec![Command::SetTrack { track: Box::new(tb) }], None).unwrap();
    let (mut e, mut ap) = Engine::new(f64::from(RATE), 4096);
    e.load(&dir, &d.project).unwrap();
    e.play();
    run(&mut ap, &mut e, 60, |_, _| {});
    let mut buf = vec![0f32; BLOCK * 2];
    let mut running = 0;
    let mut p = d.project.clone();
    let send = format!("send.{}", returns[0]);
    let before = ALLOCS.load(Ordering::SeqCst);
    for k in 0..300 {
        // Control side (allowed to allocate): messages of every kind, and one graph swap.
        match k % 60 {
            5 => e.set_param(&a_id, "fader", -3.0 - k as f64 * 0.01),
            15 => e.set_param(&b_id, &send, -10.0),
            25 => {
                p.track_mut(&a_id).unwrap().solo = k % 120 == 25;
                e.set_solo(&p);
            }
            35 => e.set_bus_fader(-1.0),
            45 => e.locate(24_000),
            50 => e.set_loop(Some((24_000, 72_000))),
            55 if k == 55 => e.load(&dir, &d.project).unwrap(),
            _ => {}
        }
        GUARD.with(|g| g.set(true));
        ap.callback(&mut buf, 2);
        GUARD.with(|g| g.set(false));
        running += usize::from(ap.is_running());
        e.poll();
        std::thread::sleep(std::time::Duration::from_micros(300));
    }
    assert_eq!(ALLOCS.load(Ordering::SeqCst) - before, 0, "allocations or frees in the audio callback");
    assert!(running > 200, "the guarded callbacks must mostly be producing audio: {running}");
    // Meters: one frame per 1/50 s of audio, so at least 30 Hz (ENG-6).
    let mut count = 0;
    let mut last = -1;
    for _ in 0..94 {
        ap.callback(&mut buf, 2); // 94 × 512 ≈ 1 s
        let (f, ids) = e.poll();
        assert_eq!(ids.len(), 4);
        if f.position != last {
            count += 1;
            last = f.position;
        }
    }
    assert!(count >= 30, "{count} meter frames per second");
}

/// req: TRN-1, ENG-1
#[test]
fn pause_stop_and_rebuild_keep_the_transport_where_it_should_be() {
    let t = tmp();
    let (dir, d) = project(t.path(), &music());
    let (mut e, mut ap) = Engine::new(f64::from(RATE), 4096);
    e.load(&dir, &d.project).unwrap();
    e.locate(96_000);
    e.play();
    let mut first = None;
    run(&mut ap, &mut e, 40, |pos, _| {
        first.get_or_insert(pos);
    });
    assert_eq!(first, Some(96_000), "playback starts where it was located");
    e.pause();
    run(&mut ap, &mut e, 3, |_, _| {});
    let p = e.position();
    run(&mut ap, &mut e, 5, |_, _| {});
    assert_eq!(e.position(), p, "paused");
    // Stop returns to the start point, every time (no drift by the latency).
    for _ in 0..5 {
        e.play();
        run(&mut ap, &mut e, 15, |_, _| {});
        e.stop();
        run(&mut ap, &mut e, 5, |_, _| {});
    }
    assert_eq!(e.position(), p);
    // A rebuild while playing resumes from the same place.
    e.play();
    run(&mut ap, &mut e, 20, |_, _| {});
    let before = e.shared.position.load(Ordering::Relaxed);
    e.load(&dir, &d.project).unwrap();
    let mut after = Vec::new();
    run(&mut ap, &mut e, 30, |pos, _| after.push(pos));
    assert!(!after.is_empty());
    assert!(after[0] >= before && after[0] < before + 4 * BLOCK as i64, "{} vs {before}", after[0]);
}

/// req: FX-7
#[test]
fn the_watched_track_spectrum_shows_its_content() {
    let t = tmp();
    let (dir, d) = project(t.path(), &music());
    let (mut e, mut ap) = Engine::new(f64::from(RATE), BLOCK);
    e.load(&dir, &d.project).unwrap();
    e.watch(Some(&id(&d, "a")));
    e.play();
    run(&mut ap, &mut e, 400, |_, _| {});
    let s = e.spectrum().expect("a watched track has a spectrum");
    assert_eq!(s.len(), stemwise_engine::realtime::SPECTRUM_BANDS);
    // Track "a" is a sine at 0.031 rad/sample ≈ 237 Hz: the loudest band holds it.
    let k = s.iter().enumerate().max_by(|x, y| x.1.total_cmp(y.1)).unwrap().0;
    let centre = 20.0 * 1000f64.powf((k as f64 + 0.5) / s.len() as f64);
    assert!((180.0..320.0).contains(&centre), "peak band at {centre:.0} Hz");
    e.watch(None);
    assert!(e.spectrum().is_none());
}

/// req: MAS-1, MAS-3, MAS-4
#[test]
fn the_master_phase_plays_through_the_chain_and_takes_parameters_live() {
    use stemwise_core::model::MasterState;
    use stemwise_engine::master::MasterParams;
    // A loud stereo source, louder than the ceiling: the limiter on the bus must hold it.
    let n = 4 * RATE as usize;
    let x: Vec<f32> = stemwise_dsp::testsignal::xorshift32(9, 2 * n).iter().map(|v| (v * 0.9) as f32).collect();
    let t = tmp();
    let (dir, d) = project(t.path(), &[("mixdown", 2, x)]);
    let mut m = MasterState::default();
    m.target.true_peak_db = -1.0;
    let p = MasterParams::of(&m, None, None, 6.0);
    let (mut e, mut ap) = Engine::new(f64::from(RATE), 4096);
    e.load_with(&dir, &d.project, Some(p)).unwrap();
    e.watch(Some(stemwise_engine::realtime::BUS_ID));
    e.play();
    let mut out: Vec<f32> = Vec::new();
    run(&mut ap, &mut e, 120, |_, buf| out.extend_from_slice(buf));
    let graph = e.poll().0.graph;
    assert!(out.len() > 50 * BLOCK);
    let tp = stemwise_dsp::truepeak::true_peak_db(&out, 2);
    assert!(tp <= -1.0 + 1e-6, "true peak {tp:.3} over the ceiling");
    // The limiter reports gain reduction; the spectrum is the bus output.
    assert!(e.poll().0.bus[9] < -1.0, "limiter gain reduction {}", e.poll().0.bus[9]);
    // Read like the meters thread does, several times (the display is smoothed over time).
    let spec = (0..10).filter_map(|_| e.spectrum()).last().unwrap();
    assert!(spec.iter().cloned().fold(f32::MIN, f32::max) > -60.0, "{spec:?}");
    // Parameters change without a new graph (no rebuild, no gap).
    ALLOCS.store(0, Ordering::SeqCst);
    e.set_master(MasterParams { loud_gain_db: -20.0, ..p });
    let mut quiet: Vec<f32> = Vec::new();
    let mut buf = vec![0f32; BLOCK * 2];
    GUARD.with(|g| g.set(true));
    for _ in 0..40 {
        ap.callback(&mut buf, 2);
    }
    GUARD.with(|g| g.set(false));
    assert_eq!(ALLOCS.load(Ordering::SeqCst), 0, "the audio thread allocated");
    for _ in 0..20 {
        ap.callback(&mut buf, 2);
        quiet.extend_from_slice(&buf);
    }
    assert_eq!(e.poll().0.graph, graph, "no rebuild");
    let peak = quiet.iter().fold(0f32, |a, v| a.max(v.abs()));
    assert!(peak < 0.2, "the new loudness gain applies: peak {peak}");
}

/// req: UI-6
#[test]
fn a_missing_file_plays_silent_live_and_stops_offline_renders() {
    let t = tmp();
    let (dir, d) = project(t.path(), &music());
    // Track "a" loses its file (deleted from the package in the Finder).
    let a = d.project.track(&id(&d, "a")).unwrap().clips[0].media.clone();
    let stored = dir.join(&d.project.media(&a).unwrap().path);
    let mut perm = std::fs::metadata(&stored).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(false);
    std::fs::set_permissions(&stored, perm).unwrap();
    std::fs::remove_file(&stored).unwrap();
    // Offline: refused, never a render with a silent hole.
    assert!(matches!(
        render_range(&dir, &d.project, 0, i64::from(RATE), 1024, &mut |_| true),
        Err(stemwise_engine::offline::RenderError::Missing(1))
    ));
    // Live: the graph loads and plays; "a" is silent, "b" is heard.
    let (mut e, mut ap) = Engine::new(f64::from(RATE), 4096);
    e.load(&dir, &d.project).unwrap();
    e.play();
    let mut energy = 0.0f64;
    run(&mut ap, &mut e, 200, |_, buf| energy += buf.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>());
    assert!(energy > 0.0, "the track with its file still plays");
    // Track meters: the loudest peak each node reached over a few more callbacks.
    let (ia, ib) = (id(&d, "a"), id(&d, "b"));
    let (mut pa, mut pb) = (-300.0f32, -300.0f32);
    let mut buf = vec![0f32; BLOCK * 2];
    for _ in 0..40 {
        ap.callback(&mut buf, 2);
        let (f, ids) = e.poll();
        for (k, n) in ids.iter().enumerate().take(f.nodes) {
            if *n == ia {
                pa = pa.max(f.node[k][0]);
            } else if *n == ib {
                pb = pb.max(f.node[k][0]);
            }
        }
    }
    assert!(pb > -60.0, "the track with its file is heard: {pb} dB");
    assert!(pa < -150.0, "the missing file is silent: {pa} dB");
}
