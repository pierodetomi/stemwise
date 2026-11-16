//! RIFF/WAVE writer (MAS-7): 16/24-bit PCM or 32-bit IEEE float, with a `bext` chunk (EBU Tech 3285: description,
//! originator "Stemwise", origination date and time, coding history). Tags are added by [`crate::tags`].

use std::io::Write;
use std::path::Path;

use crate::ExportError;

/// Samples to write: already-quantised integers with their bit depth (16 or 24), or floats (32-bit float file).
pub enum WavSamples<'a> {
    Int(&'a [i32], u32),
    Float(&'a [f32]),
}

/// What the `bext` chunk carries.
#[derive(Debug, Clone, Default)]
pub struct Bext {
    /// Up to 256 ASCII characters (truncated).
    pub description: String,
    /// "YYYY-MM-DD".
    pub date: String,
    /// "HH:MM:SS".
    pub time: String,
}

/// Appends `s` as ASCII to `out`, truncated or zero-padded to exactly `len` bytes (non-ASCII becomes `?`).
fn push_fixed(out: &mut Vec<u8>, s: &str, len: usize) {
    let start = out.len();
    out.extend(s.chars().map(|c| if c.is_ascii() { c as u8 } else { b'?' }).take(len));
    out.resize(start + len, 0);
}

fn push_chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

fn bext_body(bext: &Bext, channels: u16, rate: u32, bits: u32, float: bool) -> Vec<u8> {
    let mut b = Vec::with_capacity(602 + 64);
    push_fixed(&mut b, &bext.description, 256);
    push_fixed(&mut b, "Stemwise", 32);
    push_fixed(&mut b, "", 32); // OriginatorReference
    push_fixed(&mut b, &bext.date, 10);
    push_fixed(&mut b, &bext.time, 8);
    b.extend_from_slice(&0u64.to_le_bytes()); // TimeReference
    b.extend_from_slice(&1u16.to_le_bytes()); // Version
    b.extend_from_slice(&[0u8; 64]); // UMID
    b.extend_from_slice(&[0u8; 10]); // loudness value, range, max true peak, momentary, short-term
    b.extend_from_slice(&[0u8; 180]); // Reserved
    let coding = format!(
        "A={},F={},W={},M={},T=Stemwise\r\n",
        if float { "PCM-FLOAT" } else { "PCM" },
        rate,
        bits,
        if channels == 1 { "mono" } else { "stereo" }
    );
    b.extend_from_slice(coding.as_bytes());
    b
}

pub fn write_wav(
    path: &Path,
    samples: WavSamples<'_>,
    channels: u16,
    rate: u32,
    bext: Option<&Bext>,
) -> crate::Result<()> {
    if !(1..=2).contains(&channels) {
        return Err(ExportError::Unsupported(format!("{channels} channels")));
    }
    let (bits, float, frames_samples) = match &samples {
        WavSamples::Int(s, 16) => (16u32, false, s.len()),
        WavSamples::Int(s, 24) => (24, false, s.len()),
        WavSamples::Int(_, b) => return Err(ExportError::Unsupported(format!("{b}-bit integer WAV"))),
        WavSamples::Float(s) => (32, true, s.len()),
    };
    if frames_samples % channels as usize != 0 {
        return Err(ExportError::Unsupported("sample count is not a multiple of the channel count".into()));
    }
    let frames = (frames_samples / channels as usize) as u32;
    let block_align = channels as u32 * bits / 8;

    let mut fmt = Vec::with_capacity(18);
    fmt.extend_from_slice(&(if float { 3u16 } else { 1u16 }).to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&rate.to_le_bytes());
    fmt.extend_from_slice(&(rate * block_align).to_le_bytes());
    fmt.extend_from_slice(&(block_align as u16).to_le_bytes());
    fmt.extend_from_slice(&(bits as u16).to_le_bytes());
    if float {
        fmt.extend_from_slice(&0u16.to_le_bytes()); // cbSize
    }

    let mut data = Vec::with_capacity(frames_samples * (bits as usize / 8));
    match samples {
        WavSamples::Int(s, 16) => {
            for &v in s {
                data.extend_from_slice(&(v.clamp(i16::MIN as i32, i16::MAX as i32) as i16).to_le_bytes());
            }
        }
        WavSamples::Int(s, _) => {
            for &v in s {
                data.extend_from_slice(&v.clamp(-(1 << 23), (1 << 23) - 1).to_le_bytes()[..3]);
            }
        }
        WavSamples::Float(s) => {
            for &v in s {
                data.extend_from_slice(&v.to_le_bytes());
            }
        }
    }

    let mut body = Vec::with_capacity(data.len() + 800);
    body.extend_from_slice(b"WAVE");
    push_chunk(&mut body, b"fmt ", &fmt);
    if float {
        push_chunk(&mut body, b"fact", &frames.to_le_bytes());
    }
    if let Some(b) = bext {
        push_chunk(&mut body, b"bext", &bext_body(b, channels, rate, bits, float));
    }
    push_chunk(&mut body, b"data", &data);

    let mut file = Vec::with_capacity(body.len() + 8);
    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&(body.len() as u32).to_le_bytes());
    file.extend_from_slice(&body);

    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = std::path::PathBuf::from(tmp);
    let result = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&file)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(ExportError::from)
}
