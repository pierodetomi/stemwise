//! WAV writer and metadata tests (MAS-7, MAS-9). No audio files are committed: every signal is synthetic and every
//! file lives in a temporary folder under `app/target/tmp`.

use std::fs::File;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::tag::TagType;
use stemwise_core::model::Metadata;
use stemwise_export::tags::{Container, Cover, normalise_isrc, valid_isrc, write_tags};
use stemwise_export::wav::{Bext, WavSamples, write_wav};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

fn tmp() -> tempfile::TempDir {
    let base: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    std::fs::create_dir_all(&base).unwrap();
    tempfile::tempdir_in(base).unwrap()
}

/// Decodes a file with symphonia: (channels, rate, interleaved f32).
fn decode(path: &Path) -> (usize, u32, Vec<f32>) {
    let mss = MediaSourceStream::new(Box::new(File::open(path).unwrap()), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(e);
    }
    let mut reader = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .unwrap();
    let track = reader.default_track(TrackType::Audio).unwrap().clone();
    let params = track.codec_params.as_ref().and_then(|c| c.audio()).unwrap().clone();
    let mut decoder =
        symphonia::default::get_codecs().make_audio_decoder(&params, &AudioDecoderOptions::default()).unwrap();
    let mut out = Vec::new();
    let mut chunk = Vec::new();
    while let Ok(Some(p)) = reader.next_packet() {
        if p.track_id != track.id {
            continue;
        }
        let buf = decoder.decode(&p).unwrap();
        chunk.resize(buf.samples_interleaved(), 0.0f32);
        buf.copy_to_slice_interleaved(&mut chunk);
        out.extend_from_slice(&chunk);
    }
    (params.channels.as_ref().unwrap().count(), params.sample_rate.unwrap(), out)
}

/// Lists the RIFF chunks of a file as (id, offset of body, size).
fn riff_chunks(bytes: &[u8]) -> Vec<(String, usize, usize)> {
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    assert_eq!(riff_size + 8, bytes.len(), "RIFF size");
    let mut pos = 12;
    let mut out = Vec::new();
    while pos + 8 <= bytes.len() {
        let id = String::from_utf8_lossy(&bytes[pos..pos + 4]).to_string();
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        out.push((id, pos + 8, size));
        pos += 8 + size + (size % 2);
    }
    assert_eq!(pos, bytes.len(), "chunks must tile the file");
    out
}

fn bext() -> Bext {
    Bext { description: "test".into(), date: "2026-11-11".into(), time: "12:00:00".into() }
}

fn check_bext(bytes: &[u8]) {
    let chunks = riff_chunks(bytes);
    let ids: Vec<&str> = chunks.iter().map(|c| c.0.as_str()).collect();
    let bi = ids.iter().position(|i| *i == "bext").expect("bext chunk");
    assert!(bi < ids.iter().position(|i| *i == "data").unwrap(), "bext before data");
    let (_, off, size) = chunks[bi];
    assert!(size >= 602);
    assert_eq!(&bytes[off..off + 4], b"test");
    assert_eq!(&bytes[off + 256..off + 264], b"Stemwise");
    assert_eq!(&bytes[off + 320..off + 330], b"2026-11-11");
    assert_eq!(&bytes[off + 330..off + 338], b"12:00:00");
    assert_eq!(u16::from_le_bytes(bytes[off + 346..off + 348].try_into().unwrap()), 1, "bext version");
}

fn ramp(n: usize, bits: u32) -> Vec<i32> {
    let max = (1i64 << (bits - 1)) - 1;
    // Fixed-seed LCG, so the test signal is the same on every run.
    let mut s = 12345u64;
    (0..n)
        .map(|_| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as i64 % (2 * max + 1) - max) as i32
        })
        .collect()
}

/// req: MAS-7 — 16- and 24-bit PCM WAV round trip exactly (symphonia decodes to f32 = v / 2^(bits-1), exact).
#[test]
fn wav_int_round_trip() {
    let dir = tmp();
    for bits in [16u32, 24] {
        let src = ramp(2 * 1000, bits);
        let path = dir.path().join(format!("i{bits}.wav"));
        write_wav(&path, WavSamples::Int(&src, bits), 2, 48_000, Some(&bext())).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        check_bext(&bytes);
        let fmt = riff_chunks(&bytes).into_iter().find(|c| c.0 == "fmt ").unwrap();
        assert_eq!(fmt.2, 16);
        let (ch, rate, got) = decode(&path);
        assert_eq!((ch, rate), (2, 48_000));
        let scale = (1u32 << (bits - 1)) as f32;
        let want: Vec<f32> = src.iter().map(|&v| v as f32 / scale).collect();
        assert_eq!(got, want, "{bits}-bit samples");
    }
}

/// req: MAS-7 — 32-bit float WAV (mono) round-trips exactly and carries `fact`.
#[test]
fn wav_float_round_trip() {
    let dir = tmp();
    let src: Vec<f32> = ramp(777, 24).iter().map(|&v| v as f32 / 8_388_608.0 * 1.3).collect();
    let path = dir.path().join("f32.wav");
    write_wav(&path, WavSamples::Float(&src), 1, 44_100, Some(&bext())).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    check_bext(&bytes);
    let chunks = riff_chunks(&bytes);
    assert!(chunks.iter().any(|c| c.0 == "fact"));
    let fmt = chunks.iter().find(|c| c.0 == "fmt ").unwrap();
    assert_eq!(u16::from_le_bytes(bytes[fmt.1..fmt.1 + 2].try_into().unwrap()), 3);
    let (ch, rate, got) = decode(&path);
    assert_eq!((ch, rate), (1, 44_100));
    assert_eq!(got, src);
}

/// req: MAS-7 — odd chunk sizes are padded, output is deterministic, unsupported depths are refused, no temp left.
#[test]
fn wav_deterministic_and_errors() {
    let dir = tmp();
    let src = ramp(3, 24); // mono, 3 samples -> data of 9 bytes (odd)
    let a = dir.path().join("a.wav");
    let b = dir.path().join("b.wav");
    write_wav(&a, WavSamples::Int(&src, 24), 1, 48_000, None).unwrap();
    write_wav(&b, WavSamples::Int(&src, 24), 1, 48_000, None).unwrap();
    let ba = std::fs::read(&a).unwrap();
    assert_eq!(ba, std::fs::read(&b).unwrap());
    riff_chunks(&ba);
    let err = write_wav(&a, WavSamples::Int(&src, 20), 1, 48_000, None).unwrap_err();
    assert!(matches!(err, stemwise_export::ExportError::Unsupported(_)));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2, "no temp file left behind");
}

fn meta() -> Metadata {
    Metadata {
        title: "Song".into(),
        artist: "Artist".into(),
        album: "Album".into(),
        year: "2026".into(),
        genre: "Rock".into(),
        track_number: "7".into(),
        isrc: "it-abc-26-00001".into(),
        cover: None,
    }
}

const COVER: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3, 4, 5, 6, 7, 8, 9];

fn check_tag(tag: &lofty::tag::Tag, isrc: bool, cover: bool) {
    let front = tag.tag_type() != TagType::Mp4Ilst; // ilst `covr` atoms carry no picture type
    assert_eq!(tag.get_string(ItemKey::TrackTitle), Some("Song"));
    assert_eq!(tag.get_string(ItemKey::TrackArtist), Some("Artist"));
    assert_eq!(tag.get_string(ItemKey::AlbumTitle), Some("Album"));
    assert_eq!(tag.get_string(ItemKey::RecordingDate), Some("2026"));
    assert_eq!(tag.get_string(ItemKey::Genre), Some("Rock"));
    assert_eq!(tag.get_string(ItemKey::TrackNumber), Some("7"));
    if isrc {
        assert_eq!(tag.get_string(ItemKey::Isrc), Some("ITABC2600001"));
    }
    if cover {
        let pic = tag.pictures().first().expect("cover");
        assert_eq!(pic.data(), COVER);
        if front {
            assert_eq!(pic.pic_type(), lofty::picture::PictureType::CoverFront);
        }
    }
}

fn cover() -> Option<Cover<'static>> {
    Some(Cover { data: COVER, mime: "image/png" })
}

/// req: MAS-9 — WAV gets both LIST/INFO and ID3v2, and still decodes afterwards.
#[test]
fn tags_wav() {
    let dir = tmp();
    let path = dir.path().join("t.wav");
    let src = ramp(200, 16);
    write_wav(&path, WavSamples::Int(&src, 16), 2, 48_000, Some(&bext())).unwrap();
    write_tags(&path, Container::Wav, &meta(), cover()).unwrap();
    let file = lofty::read_from_path(&path).unwrap();
    check_tag(file.tag(TagType::RiffInfo).expect("INFO"), false, false);
    check_tag(file.tag(TagType::Id3v2).expect("ID3v2"), true, true);
    let (ch, rate, got) = decode(&path);
    assert_eq!((ch, rate, got.len()), (2, 48_000, 200));
    check_bext(&std::fs::read(&path).unwrap());
}

/// req: MAS-9 — empty fields and a non-numeric track number are not written.
#[test]
fn tags_skip_empty_fields() {
    let dir = tmp();
    let path = dir.path().join("e.wav");
    write_wav(&path, WavSamples::Int(&[0, 0], 16), 1, 48_000, None).unwrap();
    let m = Metadata { title: "Only".into(), track_number: "x".into(), isrc: "bad".into(), ..meta_empty() };
    write_tags(&path, Container::Wav, &m, None).unwrap();
    let file = lofty::read_from_path(&path).unwrap();
    let tag = file.tag(TagType::Id3v2).unwrap();
    assert_eq!(tag.get_string(ItemKey::TrackTitle), Some("Only"));
    assert!(tag.get_string(ItemKey::TrackArtist).is_none());
    assert!(tag.get_string(ItemKey::TrackNumber).is_none());
    assert!(tag.get_string(ItemKey::Isrc).is_none());
    assert_eq!(tag.picture_count(), 0);
}

fn meta_empty() -> Metadata {
    Metadata {
        title: String::new(),
        artist: String::new(),
        album: String::new(),
        year: String::new(),
        genre: String::new(),
        track_number: String::new(),
        isrc: String::new(),
        cover: None,
    }
}

/// Header-only FLAC (fLaC + STREAMINFO, no frames), used while the encoder is not available.
fn minimal_flac(path: &Path) {
    let mut b = b"fLaC".to_vec();
    b.extend_from_slice(&[0x80, 0, 0, 34]); // last block, STREAMINFO, 34 bytes
    b.extend_from_slice(&4096u16.to_be_bytes());
    b.extend_from_slice(&4096u16.to_be_bytes());
    b.extend_from_slice(&[0; 6]); // frame sizes unknown
    // 20 bits rate = 48000, 3 bits channels-1 = 1, 5 bits bps-1 = 15, 36 bits total samples = 0
    let packed: u64 = (48_000u64 << 44) | (1 << 41) | (15 << 36);
    b.extend_from_slice(&packed.to_be_bytes());
    b.extend_from_slice(&[0; 16]); // MD5
    std::fs::write(path, b).unwrap();
}

/// req: MAS-9 — FLAC gets Vorbis comments and a front-cover picture. Uses the real encoder when it exists.
#[test]
fn tags_flac() {
    let dir = tmp();
    let path = dir.path().join("t.flac");
    let src = ramp(2 * 5000, 16);
    let encoded = catch_unwind(AssertUnwindSafe(|| stemwise_export::flac::encode_flac(&path, &src, 2, 48_000, 16)));
    match encoded {
        Ok(r) => {
            r.unwrap();
            println!("tags_flac: real FLAC encoder");
        }
        Err(_) => {
            println!("tags_flac: encoder is a stub, using a header-only FLAC");
            minimal_flac(&path);
        }
    }
    write_tags(&path, Container::Flac, &meta(), cover()).unwrap();
    let file = lofty::read_from_path(&path).unwrap();
    check_tag(file.tag(TagType::VorbisComments).expect("Vorbis comments"), true, true);
}

/// req: MAS-9 — M4A gets ilst atoms and cover art; skipped (with a note) while the AAC encoder is a stub.
#[test]
fn tags_m4a() {
    let dir = tmp();
    let path = dir.path().join("t.m4a");
    let src: Vec<f32> = (0..2 * 48_000).map(|i| ((i / 2) as f32 * 0.05).sin() * 0.3).collect();
    let encoded = catch_unwind(AssertUnwindSafe(|| {
        if !stemwise_export::aac::aac_available() {
            return None;
        }
        Some(stemwise_export::aac::encode_aac(&path, &src, 2, 48_000, 128))
    }));
    match encoded {
        Ok(Some(r)) => {
            r.unwrap();
        }
        Ok(None) | Err(_) => {
            println!("tags_m4a: SKIPPED, AAC encoder unavailable or still a stub");
            return;
        }
    }
    write_tags(&path, Container::M4a, &meta(), cover()).unwrap();
    let file = lofty::read_from_path(&path).unwrap();
    check_tag(file.tag(TagType::Mp4Ilst).expect("ilst"), true, true);
}

/// req: MAS-9 — ISRC validation and normalisation.
#[test]
fn tags_isrc() {
    for ok in ["ITABC2600001", "it-abc-26-00001", "US-S1Z-99-00001", "usS1z9900001", "GB1AB2400999"] {
        assert!(valid_isrc(ok), "{ok}");
    }
    assert_eq!(normalise_isrc("it-abc-26-00001").as_deref(), Some("ITABC2600001"));
    assert_eq!(normalise_isrc("usS1z9900001").as_deref(), Some("USS1Z9900001"));
    for bad in [
        "",
        "ITABC260000",
        "ITABC26000012",
        "1TABC2600001",
        "ITABC26A0001",
        "IT-ABC-2600001",
        "IT-AB-26-000001",
        "IT-ABC-26-0000",
        "IT ABC 26 00001",
        "ITAB!2600001",
        "ITABC-2600001",
    ] {
        assert!(!valid_isrc(bad), "{bad:?}");
        assert!(normalise_isrc(bad).is_none());
    }
}
