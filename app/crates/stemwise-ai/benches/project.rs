//! Project-scale gates (FOUNDATION §3.12), run by `make bench` in release. Exits non-zero when a gate is missed.
//!
//! - NFR-3 (import): 16 stems of 4 minutes imported, ready to play and draw (copied, peaks built), under 15 s.
//! - NFR-5 (memory): a project of 32 stems of 5 minutes — imported, played by the real-time engine (the audio
//!   callback driven as a device would), then analysed for the assistant — keeps the process under 1.5 GB at its
//!   peak. Its audio as floats would be 1.8 GB: the gate fails if the audio is ever held whole in memory.
//!
//! Sources are 24-bit WAV (mono, every fourth stereo) of seeded noise with a level envelope; everything lives in
//! `target/tmp` and is removed at the end, also on failure.

use std::path::Path;
use std::time::Instant;

use stemwise_ai::analysis_doc::analyze_project;
use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::{media, package, role::Role, wav};
use stemwise_engine::realtime::Engine;

const RATE: u32 = 48000;

/// Peak resident memory of this process so far, in bytes (macOS reports bytes, Linux kilobytes).
#[cfg(unix)]
fn peak_rss() -> u64 {
    let mut u: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: getrusage fills the struct it is given.
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut u) };
    let v = u.ru_maxrss as u64;
    if cfg!(target_os = "macos") { v } else { v * 1024 }
}

/// Peak resident memory of this process so far, in bytes: the peak working set on Windows.
#[cfg(windows)]
fn peak_rss() -> u64 {
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut c: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: the pseudo-handle of the current process needs no closing; the struct is the size we pass.
    unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut c, size) };
    c.PeakWorkingSetSize as u64
}

fn stems(src: &Path, count: u32, seconds: usize) -> Vec<ImportChoice> {
    std::fs::create_dir_all(src).unwrap();
    (0..count)
        .map(|k| {
            let ch: u16 = if k % 4 == 3 { 2 } else { 1 };
            let n = seconds * RATE as usize * usize::from(ch);
            let x: Vec<f32> = stemwise_dsp::testsignal::xorshift32(k + 1, n)
                .iter()
                .enumerate()
                .map(|(i, v)| (v * 0.3 * if (i / 48000) % 4 == 0 { 0.1 } else { 1.0 }) as f32)
                .collect();
            let path = src.join(format!("Stem {k:02}.wav"));
            wav::write_pcm(&path, ch, RATE, 24, &x, false, false).unwrap();
            ImportChoice { path: path.display().to_string(), role: Role::Other, name: format!("S{k}"), convert: false }
        })
        .collect()
}

/// Imports and builds what the app needs to play and draw every file (the decoded cache, the peaks).
fn import_ready(dir: &Path, d: &mut Document, choices: &[ImportChoice]) {
    let cmds = import(dir, &mut d.project, choices).unwrap();
    d.transact(Phase::Mix, "import", cmds, None).unwrap();
    for m in &d.project.media {
        media::ensure_cache(dir, m).unwrap();
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    let mut failed = false;

    // NFR-3: import of 16 × 4 min.
    // req: NFR-3
    {
        let tmp = tempfile::tempdir_in(&root).unwrap();
        let choices = stems(&tmp.path().join("src"), 16, 240);
        let (dir, p) = package::create(tmp.path(), "Import").unwrap();
        let mut d = Document::new(p);
        let t = Instant::now();
        import_ready(&dir, &mut d, &choices);
        let secs = t.elapsed().as_secs_f64();
        println!("NFR-3: 16 stems × 4 min imported and ready in {secs:.1} s (limit 15 s)");
        failed |= secs > 15.0;
    }

    // NFR-5: 32 × 5 min, played and analysed.
    // req: NFR-5
    {
        let tmp = tempfile::tempdir_in(&root).unwrap();
        let choices = stems(&tmp.path().join("src"), 32, 300);
        let (dir, p) = package::create(tmp.path(), "Memory").unwrap();
        let mut d = Document::new(p);
        import_ready(&dir, &mut d, &choices);
        std::fs::remove_dir_all(tmp.path().join("src")).unwrap();
        let after_import = peak_rss();
        // The real-time engine, driven like a sound card for 20 s of audio.
        let (mut e, mut ap) = Engine::new(f64::from(RATE), 4096);
        e.load(&dir, &d.project).unwrap();
        e.play();
        let mut buf = vec![0f32; 512 * 2];
        for _ in 0..(20 * RATE as usize / 512) {
            ap.callback(&mut buf, 2);
            e.poll();
            std::thread::sleep(std::time::Duration::from_micros(200));
        }
        let after_play = peak_rss();
        drop(e);
        drop(ap);
        // The assistant's analysis of every stem.
        analyze_project(&dir, &d.project, &|_| true).unwrap();
        let peak = peak_rss();
        let gb = |b: u64| b as f64 / 1e9;
        println!(
            "NFR-5: 32 stems × 5 min — peak {:.2} GB (after import {:.2}, after 20 s of playback {:.2}; limit 1.5 GB)",
            gb(peak),
            gb(after_import),
            gb(after_play)
        );
        failed |= peak > 1_500_000_000;
    }

    if failed {
        println!("project gates: FAILED");
        std::process::exit(1);
    }
}
