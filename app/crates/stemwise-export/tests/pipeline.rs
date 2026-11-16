//! The export of a master on test signals (F8 gate): the true peak of every file, measured with 4x oversampling on
//! its decoded content, stays within the destination's ceiling (MAS-3, MAS-10); loudness reaches the destination
//! (MAS-2); lossless files are deterministic (MAS-8); rates are converted (MAS-7); tags are written (MAS-9).

use std::path::Path;

use stemwise_core::media::decode_all;
use stemwise_core::model::{MasterState, Metadata};
use stemwise_dsp::testsignal::xorshift32;
use stemwise_dsp::{loudness, truepeak};
use stemwise_export::ExportError;
use stemwise_export::pipeline::{Settings, export};

const R: u32 = 48000;

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

/// A dynamic stereo "mixdown" at −3 dBTP-ish: drums-like bursts, a bass line, a bright partial.
fn mixdown(seconds: usize) -> Vec<f32> {
    let n = seconds * R as usize;
    let noise = xorshift32(7, 2 * n);
    let mut x = Vec::with_capacity(2 * n);
    for i in 0..n {
        let t = i as f64 / f64::from(R);
        let hit = (-(t * 2.0).fract() * 30.0).exp();
        let bass = 0.35 * (std::f64::consts::TAU * 55.0 * t).sin();
        let bright = 0.08 * (std::f64::consts::TAU * 7000.0 * t).sin();
        let env = 0.6 + 0.4 * (std::f64::consts::TAU * t / seconds as f64).sin().abs();
        x.push((env * (bass + 0.4 * hit * noise[2 * i] + bright)) as f32);
        x.push((env * (bass * 0.95 + 0.4 * hit * noise[2 * i + 1] - bright)) as f32);
    }
    x
}

fn meta() -> Metadata {
    Metadata {
        title: "Test".into(),
        artist: "Stemwise".into(),
        album: "Gate".into(),
        year: "2026".into(),
        genre: "Test".into(),
        track_number: "1".into(),
        isrc: "IT-ABC-26-00001".into(),
        cover: None,
    }
}

/// req: MAS-2, MAS-3, MAS-7, MAS-10
#[test]
fn every_exported_file_stays_under_the_ceiling_and_reaches_the_destination() {
    let x = mixdown(10);
    let t = tmp();
    for (preset, lufs, tp, rate) in
        [("streaming", -14.0, -1.0, 48000), ("loud", -9.0, -1.0, 44100), ("cd", -10.0, -0.3, 44100)]
    {
        let mut m = MasterState::default();
        m.target.preset = preset.into();
        m.target.lufs = lufs;
        m.target.true_peak_db = tp;
        let folder = t.path().join(preset);
        let s = Settings {
            wav: Some(16),
            flac: Some(24),
            aac: stemwise_export::aac::aac_available().then_some(256),
            rate,
            dither: true,
            folder: &folder,
            basename: "Test",
            meta: &meta(),
            cover: None,
            when_secs: 1_794_355_200,
            sections: &[],
        };
        let reports = export(&x, R, &m, None, None, &s, &mut |_| true).unwrap();
        assert!(reports.len() >= 2);
        for r in &reports {
            let (info, y) = decode_all(Path::new(&r.path)).unwrap();
            assert_eq!(info.sample_rate, rate, "{}", r.format);
            let measured = truepeak::true_peak_db(&y, 2);
            assert!(measured <= tp + 1e-6, "{preset} {}: true peak {measured:.3} dBTP over {tp}", r.format);
            assert!((r.true_peak_db - measured).abs() < 0.05 || r.format.starts_with("AAC"), "report matches the file");
            let l = loudness::integrated(&y, 2, rate).unwrap();
            assert!((l - lufs).abs() < 0.7, "{preset} {}: {l:.2} LUFS", r.format);
        }
    }
}

/// req: MAS-8, MAS-9
#[test]
fn lossless_exports_are_deterministic_and_tagged() {
    let x = mixdown(4);
    let t = tmp();
    let m = MasterState::default();
    let run = |dir: &str| {
        let folder = t.path().join(dir);
        let s = Settings {
            wav: Some(24),
            flac: Some(16),
            aac: None,
            rate: R,
            dither: true,
            folder: &folder,
            basename: "Same",
            meta: &meta(),
            cover: None,
            when_secs: 1_794_355_200,
            sections: &[],
        };
        export(&x, R, &m, None, None, &s, &mut |_| true).unwrap();
        (std::fs::read(folder.join("Same.wav")).unwrap(), std::fs::read(folder.join("Same.flac")).unwrap())
    };
    let (a, b) = (run("a"), run("b"));
    let first_diff = |x: &[u8], y: &[u8]| x.iter().zip(y).position(|(p, q)| p != q).map(|i| (i, x.len(), y.len()));
    assert_eq!(first_diff(&a.0, &b.0), None, "WAV bytes (first difference, lengths)");
    assert_eq!(a.0.len(), b.0.len());
    assert_eq!(first_diff(&a.1, &b.1), None, "FLAC bytes");
    assert_eq!(a.1.len(), b.1.len());
    // ISRC and title are in the files (normalised ISRC).
    for bytes in [&a.0, &a.1] {
        let text = String::from_utf8_lossy(bytes);
        assert!(text.contains("ITABC2600001") && text.contains("Test"));
    }
}

/// A loud, crest-heavy master: every block of the chain on, a hot EQ, and content with inter-sample peaks (a sine at
/// a quarter of the rate, 45° off the samples, peaks 3 dB above its samples).
fn hot_master(lufs: f64, tp: f64) -> MasterState {
    let mut m = MasterState::default();
    m.target.preset = "custom".into();
    m.target.lufs = lufs;
    m.target.true_peak_db = tp;
    let c = &mut m.chain;
    c.input_gain_db = 6.0;
    // Every kind at work (MAS-1, MAS-11, MAS-12): the ceiling must hold whatever the chain does.
    for b in &mut c.blocks {
        use stemwise_core::model::Block;
        b.block.set_bypass(false);
        match &mut b.block {
            Block::Eq(eq) => {
                eq.bands[0].enabled = true;
                eq.bands[0].gain_db = 6.0;
                eq.bands[3].enabled = true;
                eq.bands[3].gain_db = 4.0;
            }
            Block::Compressor(k) => {
                k.ratio = 4.0;
                k.attack_ms = 0.3;
            }
            Block::Width(w) => w.side_gain_db = 3.0,
            Block::Saturation(x) => x.drive_db = 9.0,
            Block::Clipper(k) => k.take_off_db = 3.0,
            Block::Multiband(_) | Block::MatchEq(_) => {}
        }
    }
    m
}

fn spiky(seconds: usize, rate: u32) -> Vec<f32> {
    let mut x = mixdown(seconds);
    let n = x.len() / 2;
    for i in 0..n {
        let t = i as f64 / f64::from(rate);
        let s = 0.3
            * (std::f64::consts::FRAC_PI_2 * i as f64 + std::f64::consts::FRAC_PI_4).sin()
            * if (t * 4.0).fract() < 0.05 { 1.0 } else { 0.0 };
        x[2 * i] += s as f32;
        x[2 * i + 1] -= s as f32;
    }
    x
}

/// req: MAS-3, MAS-10
#[test]
fn the_ceiling_holds_on_quantised_files_for_any_chain_and_rate() {
    let t = tmp();
    for (k, (lufs, tp, rate, bits)) in
        [(-6.0, 0.0, 96000, 16), (-7.0, -0.1, 88200, 16), (-8.0, -0.1, 44100, 24), (-6.0, 0.0, 48000, 32)]
            .into_iter()
            .enumerate()
    {
        let x = spiky(6, R);
        let m = hot_master(lufs, tp);
        let folder = t.path().join(format!("hot{k}"));
        let s = Settings {
            wav: Some(bits),
            flac: (bits != 32).then_some(bits),
            aac: (rate <= 48000 && stemwise_export::aac::aac_available()).then_some(256),
            rate,
            dither: true,
            folder: &folder,
            basename: "Hot",
            meta: &meta(),
            cover: None,
            when_secs: 1_794_355_200,
            sections: &[],
        };
        let reports = export(&x, R, &m, None, None, &s, &mut |_| true).unwrap();
        for r in &reports {
            let (_, y) = decode_all(Path::new(&r.path)).unwrap();
            let measured = truepeak::true_peak_db(&y, 2);
            assert!(measured <= tp + 1e-9, "{k} {}: {measured:.4} dBTP over {tp}", r.format);
            assert!((r.true_peak_db - measured).abs() < 1e-9, "{k} {}: the report is measured on the file", r.format);
        }
    }
}

/// req: MAS-7, MAS-10
#[test]
fn aac_above_48_khz_is_refused_before_anything_is_written() {
    if !stemwise_export::aac::aac_available() {
        return;
    }
    let t = tmp();
    let folder = t.path().join("refused");
    let s = Settings {
        wav: Some(24),
        flac: None,
        aac: Some(256),
        rate: 96000,
        dither: true,
        folder: &folder,
        basename: "No",
        meta: &meta(),
        cover: None,
        when_secs: 0,
        sections: &[],
    };
    let e = export(&mixdown(1), R, &MasterState::default(), None, None, &s, &mut |_| true).unwrap_err();
    assert!(matches!(e, ExportError::Unsupported(_)), "{e}");
    assert!(!folder.exists(), "nothing written");
}

/// req: MAS-7
#[test]
fn a_cancelled_export_leaves_no_files() {
    let t = tmp();
    let folder = t.path().join("cancel");
    let s = Settings {
        wav: Some(24),
        flac: Some(24),
        aac: None,
        rate: R,
        dither: true,
        folder: &folder,
        basename: "Gone",
        meta: &meta(),
        cover: None,
        when_secs: 0,
        sections: &[],
    };
    // Cancel once the first file is written.
    let e = export(&mixdown(2), R, &MasterState::default(), None, None, &s, &mut |f| f < 0.7).unwrap_err();
    assert!(matches!(e, ExportError::Cancelled));
    assert_eq!(std::fs::read_dir(&folder).map(|d| d.count()).unwrap_or(0), 0);
}
