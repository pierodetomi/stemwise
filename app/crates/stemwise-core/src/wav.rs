//! Minimal WAV / RF64 reader and float WAV writer (PRJ-4).
//!
//! Imported WAV files are read in place from the package (no decoded copy): PCM 16/24/32-bit, IEEE float 32/64,
//! `WAVE_FORMAT_EXTENSIBLE`, and RF64 for files over 4 GB.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleKind {
    Int,
    Float,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavInfo {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits: u16,
    pub kind: SampleKind,
    pub data_offset: u64,
    pub frames: u64,
}

impl WavInfo {
    pub fn bytes_per_frame(&self) -> u64 {
        u64::from(self.bits / 8) * u64::from(self.channels)
    }

    pub fn encoding(&self) -> String {
        match self.kind {
            SampleKind::Int => format!("PCM {}-bit", self.bits),
            SampleKind::Float => format!("Float {}-bit", self.bits),
        }
    }
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

fn u16le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn u32le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// Parses the header of a WAV or RF64 file.
pub fn read_info(path: &Path) -> io::Result<WavInfo> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let mut head = [0u8; 12];
    f.read_exact(&mut head)?;
    let rf64 = match &head[0..4] {
        b"RIFF" => false,
        b"RF64" | b"BW64" => true,
        _ => return Err(bad("not a RIFF/RF64 file")),
    };
    if &head[8..12] != b"WAVE" {
        return Err(bad("not a WAVE file"));
    }
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut ds64_data: Option<u64> = None;
    let mut pos = 12u64;
    while pos + 8 <= len {
        f.seek(SeekFrom::Start(pos))?;
        let mut ch = [0u8; 8];
        f.read_exact(&mut ch)?;
        let id = [ch[0], ch[1], ch[2], ch[3]];
        let mut size = u64::from(u32le(&ch[4..8]));
        let body = pos + 8;
        match &id {
            b"ds64" => {
                let mut b = [0u8; 24];
                f.read_exact(&mut b)?;
                ds64_data = Some(u64::from_le_bytes(b[8..16].try_into().expect("8 bytes")));
            }
            b"fmt " => {
                let mut b = vec![0u8; size.min(64) as usize];
                f.read_exact(&mut b)?;
                if b.len() < 16 {
                    return Err(bad("short fmt chunk"));
                }
                let mut tag = u16le(&b[0..2]);
                let channels = u16le(&b[2..4]);
                let rate = u32le(&b[4..8]);
                let bits = u16le(&b[14..16]);
                if tag == 0xFFFE {
                    if b.len() < 26 {
                        return Err(bad("short extensible fmt chunk"));
                    }
                    tag = u16le(&b[24..26]);
                }
                fmt = Some((tag, channels, rate, bits));
            }
            b"data" => {
                if rf64 && size == 0xFFFF_FFFF {
                    size = ds64_data.ok_or_else(|| bad("RF64 without ds64"))?;
                }
                let (tag, channels, sample_rate, bits) = fmt.ok_or_else(|| bad("data before fmt"))?;
                let kind = match (tag, bits) {
                    (1, 16 | 24 | 32) => SampleKind::Int,
                    (3, 32 | 64) => SampleKind::Float,
                    _ => {
                        return Err(bad(&format!("unsupported WAV encoding (format {tag}, {bits} bit)")));
                    }
                };
                if channels == 0 || channels > 2 {
                    return Err(bad("only mono and stereo files are supported"));
                }
                let size = size.min(len.saturating_sub(body));
                let bpf = u64::from(bits / 8) * u64::from(channels);
                return Ok(WavInfo { channels, sample_rate, bits, kind, data_offset: body, frames: size / bpf });
            }
            _ => {}
        }
        pos = body + size + (size & 1);
    }
    Err(bad("no data chunk"))
}

/// Converts raw little-endian sample bytes to f32.
pub fn decode_samples(info: &WavInfo, bytes: &[u8], out: &mut [f32]) {
    let n = out.len().min(bytes.len() / usize::from(info.bits / 8));
    match (info.kind, info.bits) {
        (SampleKind::Int, 16) => {
            for (o, b) in out[..n].iter_mut().zip(bytes.as_chunks::<2>().0) {
                *o = f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0;
            }
        }
        (SampleKind::Int, 24) => {
            for (o, b) in out[..n].iter_mut().zip(bytes.as_chunks::<3>().0) {
                let v = i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8;
                *o = v as f32 / 8_388_608.0;
            }
        }
        (SampleKind::Int, 32) => {
            for (o, b) in out[..n].iter_mut().zip(bytes.as_chunks::<4>().0) {
                *o = (f64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]])) / 2_147_483_648.0) as f32;
            }
        }
        (SampleKind::Float, 32) => {
            for (o, b) in out[..n].iter_mut().zip(bytes.as_chunks::<4>().0) {
                *o = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            }
        }
        (SampleKind::Float, 64) => {
            for (o, b) in out[..n].iter_mut().zip(bytes.as_chunks::<8>().0) {
                *o = f64::from_le_bytes(*b) as f32;
            }
        }
        _ => out[..n].fill(0.0),
    }
}

/// Writes interleaved samples as a 32-bit float WAV (converted imports, mixdowns).
pub fn write_f32(path: &Path, channels: u16, sample_rate: u32, samples: &[f32]) -> io::Result<()> {
    let data_len = (samples.len() * 4) as u64;
    if data_len > u64::from(u32::MAX) - 64 {
        return Err(bad("file too large for RIFF"));
    }
    let mut w = io::BufWriter::new(File::create(path)?);
    w.write_all(b"RIFF")?;
    w.write_all(&((data_len + 36 + 12) as u32).to_le_bytes())?;
    w.write_all(b"WAVE")?;
    w.write_all(b"fmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&3u16.to_le_bytes())?;
    w.write_all(&channels.to_le_bytes())?;
    w.write_all(&sample_rate.to_le_bytes())?;
    w.write_all(&(sample_rate * u32::from(channels) * 4).to_le_bytes())?;
    w.write_all(&(channels * 4).to_le_bytes())?;
    w.write_all(&32u16.to_le_bytes())?;
    // `fact` chunk, required for non-PCM formats.
    w.write_all(b"fact")?;
    w.write_all(&4u32.to_le_bytes())?;
    w.write_all(&((samples.len() / usize::from(channels)) as u32).to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&(data_len as u32).to_le_bytes())?;
    for s in samples {
        w.write_all(&s.to_le_bytes())?;
    }
    w.flush()
}

/// Writes interleaved samples as integer PCM (16 or 24 bit) — used by tests to build import fixtures.
pub fn write_pcm(
    path: &Path,
    channels: u16,
    sample_rate: u32,
    bits: u16,
    samples: &[f32],
    rf64: bool,
    extensible: bool,
) -> io::Result<()> {
    let bps = usize::from(bits / 8);
    let data_len = (samples.len() * bps) as u64;
    let mut w = io::BufWriter::new(File::create(path)?);
    let fmt_len: u32 = if extensible { 40 } else { 16 };
    w.write_all(if rf64 { b"RF64" } else { b"RIFF" })?;
    w.write_all(&(if rf64 { u32::MAX } else { (4 + 8 + fmt_len as u64 + 8 + data_len) as u32 }).to_le_bytes())?;
    w.write_all(b"WAVE")?;
    if rf64 {
        w.write_all(b"ds64")?;
        w.write_all(&28u32.to_le_bytes())?;
        w.write_all(&(4 + 36 + 8 + u64::from(fmt_len) + 8 + data_len).to_le_bytes())?;
        w.write_all(&data_len.to_le_bytes())?;
        w.write_all(&((samples.len() / usize::from(channels)) as u64).to_le_bytes())?;
        w.write_all(&0u32.to_le_bytes())?;
    }
    w.write_all(b"fmt ")?;
    w.write_all(&fmt_len.to_le_bytes())?;
    w.write_all(&(if extensible { 0xFFFEu16 } else { 1u16 }).to_le_bytes())?;
    w.write_all(&channels.to_le_bytes())?;
    w.write_all(&sample_rate.to_le_bytes())?;
    w.write_all(&(sample_rate * u32::from(channels) * bps as u32).to_le_bytes())?;
    w.write_all(&(channels * bps as u16).to_le_bytes())?;
    w.write_all(&bits.to_le_bytes())?;
    if extensible {
        w.write_all(&22u16.to_le_bytes())?;
        w.write_all(&bits.to_le_bytes())?;
        w.write_all(&(if channels == 2 { 3u32 } else { 4u32 }).to_le_bytes())?;
        w.write_all(&1u16.to_le_bytes())?;
        w.write_all(&[0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71])?;
    }
    w.write_all(b"data")?;
    w.write_all(&(if rf64 { u32::MAX } else { data_len as u32 }).to_le_bytes())?;
    let scale = f64::from(1u32 << (bits - 1));
    for s in samples {
        let v = (f64::from(*s) * scale).round().clamp(-scale, scale - 1.0) as i32;
        w.write_all(&v.to_le_bytes()[..bps])?;
    }
    w.flush()
}
