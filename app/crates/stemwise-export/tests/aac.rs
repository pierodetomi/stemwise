//! AAC encoder through the system encoder (A-005): decoded with symphonia to check the result.

use std::f32::consts::TAU;
use std::fs::File;
use std::path::Path;

use stemwise_export::aac::{aac_available, encode_aac};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::TrackType;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

fn scratch() -> tempfile::TempDir {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::tempdir_in(base).unwrap()
}

fn sine(rate: u32, channels: u16, seconds: f32, freq: f32, amp: f32) -> Vec<f32> {
    let frames = (rate as f32 * seconds) as usize;
    let mut out = Vec::with_capacity(frames * usize::from(channels));
    for n in 0..frames {
        let v = amp * (TAU * freq * n as f32 / rate as f32).sin();
        for _ in 0..channels {
            out.push(v);
        }
    }
    out
}

struct Decoded {
    channels: usize,
    rate: u32,
    samples: Vec<f32>,
}

fn decode(path: &Path) -> Decoded {
    let mss = MediaSourceStream::new(Box::new(File::open(path).unwrap()), Default::default());
    let mut format = symphonia::default::get_probe()
        .probe(&Hint::new(), mss, FormatOptions::default(), MetadataOptions::default())
        .unwrap();
    let track = format.default_track(TrackType::Audio).unwrap();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(track.codec_params.as_ref().unwrap().audio().unwrap(), &AudioDecoderOptions::default())
        .unwrap();
    let track_id = track.id;
    let mut samples: Vec<f32> = Vec::new();
    let (mut channels, mut rate) = (0, 0);
    while let Some(packet) = format.next_packet().unwrap() {
        if packet.track_id != track_id {
            continue;
        }
        let buf = decoder.decode(&packet).unwrap();
        channels = buf.spec().channels().count();
        rate = buf.spec().rate();
        let start = samples.len();
        samples.resize(start + buf.samples_interleaved(), 0.0);
        buf.copy_to_slice_interleaved(&mut samples[start..]);
    }
    Decoded { channels, rate, samples }
}

fn rms_db(x: &[f32]) -> f32 {
    let ms = x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64;
    (10.0 * ms.log10()) as f32
}

fn check_roundtrip(channels: u16, rate: u32, kbps: u32) {
    let dir = scratch();
    let path = dir.path().join("out.m4a");
    let input = sine(rate, channels, 3.0, 1000.0, 0.5);
    encode_aac(&path, &input, channels, rate, kbps).unwrap();
    assert!(!dir.path().join("out.m4a.part").exists());

    let out = decode(&path);
    let ch = usize::from(channels);
    assert_eq!(out.channels, ch);
    assert_eq!(out.rate, rate);
    let in_frames = input.len() / ch;
    let out_frames = out.samples.len() / ch;
    // symphonia does not trim gapless info: the decoder keeps the 2112-frame AAC priming plus up to one packet of
    // padding (about 3100 frames at most), so the tolerance is four packets.
    assert!(in_frames.abs_diff(out_frames) <= 4096, "{in_frames} vs {out_frames}");

    // Channel 0, edges skipped (priming and tail).
    let ch0 = |s: &[f32]| s.iter().step_by(ch).copied().collect::<Vec<f32>>();
    let (a, b) = (ch0(&input), ch0(&out.samples));
    let a = &a[4096..a.len() - 4096];
    let b = &b[4096..b.len() - 4096];
    let diff = (rms_db(a) - rms_db(b)).abs();
    assert!(diff < 0.5, "RMS differs by {diff} dB");

    let crossings = b.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
    let freq = crossings as f32 * rate as f32 / b.len() as f32;
    assert!((freq - 1000.0).abs() < 20.0, "dominant frequency {freq} Hz");
}

/// req: MAS-10
#[test]
#[cfg(target_os = "macos")]
fn aac_is_available_on_macos() {
    assert!(aac_available());
}

/// req: MAS-10, MAS-7
#[test]
fn aac_stereo_48k_roundtrip() {
    check_roundtrip(2, 48_000, 256);
}

/// req: MAS-10
#[test]
fn aac_mono_44k_roundtrip() {
    check_roundtrip(1, 44_100, 128);
}

/// req: MAS-10
#[test]
fn aac_unsupported_channel_count_is_an_error() {
    let dir = scratch();
    let path = dir.path().join("out.m4a");
    assert!(encode_aac(&path, &[0.0; 3000], 3, 48_000, 256).is_err());
    assert!(!path.exists());
}
