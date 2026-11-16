//! FLAC encoder (MAS-7, PRJ-4): round trips through the decoder, STREAMINFO signature, compression, import.

use std::path::Path;

use stemwise_core::history::{Document, Phase};
use stemwise_core::import::{ImportChoice, import};
use stemwise_core::media::{self, decode_all};
use stemwise_core::{package, role::Role};
use stemwise_export::flac::{encode_flac, md5};

fn tmp() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

fn xorshift(seed: u32, n: usize, bits: u32) -> Vec<i32> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            (s as i32) >> (32 - bits)
        })
        .collect()
}

fn sine(n: usize, channels: usize, bits: u32) -> Vec<i32> {
    let amp = f64::from((1i32 << (bits - 1)) - 1) * 0.8;
    (0..n * channels)
        .map(|i| {
            let f = if i % channels == 0 { 440.0 } else { 660.0 };
            (amp * (std::f64::consts::TAU * f * (i / channels) as f64 / 44100.0).sin()).round() as i32
        })
        .collect()
}

fn decode(path: &Path, bits: u32) -> (u16, Vec<i32>) {
    let (info, f) = decode_all(path).unwrap();
    let scale = f64::from(1u32 << (bits - 1));
    (info.channels, f.iter().map(|&v| (f64::from(v) * scale).round() as i32).collect())
}

fn round_trip(name: &str, samples: &[i32], channels: u16, bits: u32) {
    let dir = tmp();
    let path = dir.path().join("a.flac");
    encode_flac(&path, samples, channels, 44100, bits).unwrap();
    let (ch, back) = decode(&path, bits);
    assert_eq!(ch, channels, "{name}");
    assert_eq!(back.len(), samples.len(), "{name}");
    assert!(back == samples, "{name}: decoded samples differ");
}

/// req: MAS-7
#[test]
fn flac_round_trip_is_bit_exact() {
    for bits in [16u32, 24] {
        for channels in [1u16, 2] {
            let ch = usize::from(channels);
            for n in [4096 * 3 + 1000, 4096 * 2, 100, 1] {
                let noise = xorshift(7 + n as u32, n * ch, bits);
                round_trip(&format!("noise {bits}/{channels}/{n}"), &noise, channels, bits);
                round_trip(&format!("sine {bits}/{channels}/{n}"), &sine(n, ch, bits), channels, bits);
                round_trip(&format!("silence {bits}/{channels}/{n}"), &vec![0; n * ch], channels, bits);
            }
        }
    }
}

/// req: MAS-7
#[test]
fn flac_round_trip_extremes_and_correlated_channels() {
    for bits in [16u32, 24] {
        let max = (1i32 << (bits - 1)) - 1;
        let min = -(1i32 << (bits - 1));
        let n = 5000;
        let ext: Vec<i32> = (0..n * 2).map(|i| if (i / 2 + i % 2) % 3 == 0 { max } else { min }).collect();
        round_trip("extremes", &ext, 2, bits);
        // Near-identical channels exercise the side channel (bits + 1 wide).
        let a = xorshift(3, n, bits);
        let mut b = Vec::new();
        for (i, &v) in a.iter().enumerate() {
            b.push(v);
            b.push(if i % 2 == 0 { v } else { (v / 2).clamp(min, max) });
        }
        round_trip("correlated", &b, 2, bits);
        let opposed: Vec<i32> = a.iter().flat_map(|&v| [v.clamp(min + 1, max), -v.clamp(min + 1, max)]).collect();
        round_trip("opposed", &opposed, 2, bits);
    }
}

/// req: MAS-7
#[test]
fn flac_streaminfo_md5_matches_audio() {
    for bits in [16u32, 24] {
        let samples = sine(4096 + 123, 2, bits);
        let dir = tmp();
        let path = dir.path().join("a.flac");
        encode_flac(&path, &samples, 2, 48000, bits).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"fLaC");
        assert_eq!(bytes[4], 0x80);
        assert_eq!(&bytes[5..8], &[0, 0, 34]);
        let si = &bytes[8..42];
        assert_eq!(u16::from_be_bytes([si[0], si[1]]), 4096);
        assert_eq!(u16::from_be_bytes([si[2], si[3]]), 4096);
        let packed = u64::from_be_bytes(si[10..18].try_into().unwrap());
        assert_eq!(packed >> 44, 48000);
        assert_eq!((packed >> 41) & 7, 1);
        assert_eq!((packed >> 36) & 31, u64::from(bits - 1));
        assert_eq!(packed & ((1 << 36) - 1), (4096 + 123) as u64);
        let raw: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()[..(bits / 8) as usize].to_vec()).collect();
        assert_eq!(&si[18..34], &md5(&raw));
    }
}

/// req: MAS-7
#[test]
fn flac_md5_known_vectors_public() {
    let hex = |d: [u8; 16]| d.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!(hex(md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(hex(md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
}

/// req: MAS-7
#[test]
fn flac_compresses_a_sine_and_is_deterministic() {
    let samples = sine(44100, 2, 16);
    let dir = tmp();
    let (a, b) = (dir.path().join("a.flac"), dir.path().join("b.flac"));
    encode_flac(&a, &samples, 2, 44100, 16).unwrap();
    encode_flac(&b, &samples, 2, 44100, 16).unwrap();
    let raw = samples.len() as u64 * 2;
    let size = std::fs::metadata(&a).unwrap().len();
    assert!(size * 3 < raw, "sine: {size} bytes vs {raw} raw");
    assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    assert!(!dir.path().join("a.flac.tmp").exists());
}

/// req: MAS-7
#[test]
fn flac_rejects_unsupported_formats() {
    let dir = tmp();
    let p = dir.path().join("x.flac");
    assert!(encode_flac(&p, &[0; 6], 3, 44100, 16).is_err());
    assert!(encode_flac(&p, &[0; 4], 2, 44100, 32).is_err());
    assert!(encode_flac(&p, &[0; 4], 2, 44100, 8).is_err());
    assert!(!p.exists());
}

/// req: PRJ-4
#[test]
fn flac_stem_imports_into_a_project() {
    let dir = tmp();
    let n = 4096 * 2 + 77;
    let samples = {
        let mut v = sine(n, 2, 24);
        for (i, x) in xorshift(11, n * 2, 24).iter().enumerate() {
            v[i] += x >> 12;
        }
        v
    };
    let src = dir.path().join("Bass_stem.flac");
    encode_flac(&src, &samples, 2, 48000, 24).unwrap();
    let (pkg, mut project) = package::create(dir.path(), "Song").unwrap();
    let mut doc = Document::new(project.clone());
    let choice =
        ImportChoice { path: src.display().to_string(), role: Role::Bass, name: "Bass".into(), convert: false };
    let cmds = import(&pkg, &mut project, &[choice]).unwrap();
    doc.project = project;
    doc.transact(Phase::Mix, "import", cmds, None).unwrap();
    let m = &doc.project.media[0];
    assert_eq!((m.channels, m.sample_rate, m.frames), (2, 48000, n as u64));
    let pcm = std::fs::read(media::pcm_cache_path(&pkg, &m.id)).unwrap();
    assert_eq!(pcm.len(), n * 2 * 4);
    let scale = f64::from(1u32 << 23);
    for (i, c) in pcm.as_chunks::<4>().0.iter().enumerate() {
        let v = f32::from_le_bytes(*c);
        assert_eq!((f64::from(v) * scale).round() as i32, samples[i], "sample {i}");
    }
}
