//! Imported audio (PRJ-3 … PRJ-5, PRJ-7, TRN-3): probing, copying into the package, conversion of the sample
//! rate, decoded caches, sample access and peak pyramids.
//!
//! Files in `audio/` and `references/` are immutable and named by the BLAKE3 hash of their content. WAV files are
//! read in place; other formats are decoded once into `cache/pcm/<id>.f32` (raw interleaved little-endian f32),
//! which can be deleted and rebuilt at any time.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::model::{AudioFormat, MediaFile};
use crate::wav;

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("unsupported file: {0}")]
    Unsupported(String),
    #[error("cannot decode the file: {0}")]
    Decode(String),
    #[error("the file has no audio")]
    Empty,
}

/// What the import screen shows for a file before it is imported (design 2c).
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeInfo {
    pub format: AudioFormat,
    pub channels: u16,
    pub sample_rate: u32,
    pub frames: u64,
    pub encoding: String,
}

pub fn format_of(path: &Path) -> Option<AudioFormat> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "wav" | "wave" | "bwf" | "rf64" => AudioFormat::Wav,
        "flac" => AudioFormat::Flac,
        "aif" | "aiff" | "aifc" => AudioFormat::Aiff,
        "mp3" => AudioFormat::Mp3,
        "m4a" | "aac" | "mp4" => AudioFormat::Aac,
        _ => return None,
    })
}

fn extension(f: AudioFormat) -> &'static str {
    match f {
        AudioFormat::Wav => "wav",
        AudioFormat::Flac => "flac",
        AudioFormat::Aiff => "aiff",
        AudioFormat::Mp3 => "mp3",
        AudioFormat::Aac => "m4a",
    }
}

fn open_symphonia(path: &Path) -> Result<Box<dyn symphonia::core::formats::FormatReader>, MediaError> {
    let file = Box::new(File::open(path)?);
    let mss = MediaSourceStream::new(file, Default::default());
    let mut hint = Hint::new();
    if let Some(e) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(e);
    }
    symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .map_err(|e| MediaError::Unsupported(e.to_string()))
}

/// Reads format, channels, rate and length without decoding the whole file.
pub fn probe(path: &Path) -> Result<ProbeInfo, MediaError> {
    let format = format_of(path).ok_or_else(|| MediaError::Unsupported(path.display().to_string()))?;
    if format == AudioFormat::Wav {
        let i = wav::read_info(path).map_err(|e| MediaError::Unsupported(e.to_string()))?;
        return Ok(ProbeInfo {
            format,
            channels: i.channels,
            sample_rate: i.sample_rate,
            frames: i.frames,
            encoding: i.encoding(),
        });
    }
    let reader = open_symphonia(path)?;
    let track = reader.default_track(TrackType::Audio).ok_or(MediaError::Empty)?;
    let params = track.codec_params.as_ref().and_then(|c| c.audio()).ok_or(MediaError::Empty)?;
    let channels = params.channels.as_ref().map_or(0, |c| c.count()) as u16;
    let sample_rate = params.sample_rate.ok_or(MediaError::Empty)?;
    if channels == 0 || channels > 2 {
        return Err(MediaError::Unsupported("only mono and stereo files are supported".into()));
    }
    let encoding = match (format, params.bits_per_sample) {
        (AudioFormat::Flac | AudioFormat::Aiff, Some(b)) => format!("PCM {b}-bit"),
        (AudioFormat::Mp3, _) => "MP3".into(),
        (AudioFormat::Aac, _) => "AAC".into(),
        _ => "PCM".into(),
    };
    let frames = match track.num_frames {
        Some(n) => n,
        None => decode_all(path)?.1.len() as u64 / u64::from(channels),
    };
    Ok(ProbeInfo { format, channels, sample_rate, frames, encoding })
}

/// Decodes a whole file to interleaved f32.
pub fn decode_all(path: &Path) -> Result<(ProbeInfo, Vec<f32>), MediaError> {
    let format = format_of(path).ok_or_else(|| MediaError::Unsupported(path.display().to_string()))?;
    if format == AudioFormat::Wav {
        let i = wav::read_info(path).map_err(|e| MediaError::Unsupported(e.to_string()))?;
        let f = File::open(path)?;
        let mut bytes = vec![0u8; (i.frames * i.bytes_per_frame()) as usize];
        read_exact_at(&f, &mut bytes, i.data_offset)?;
        let mut out = vec![0f32; (i.frames * u64::from(i.channels)) as usize];
        wav::decode_samples(&i, &bytes, &mut out);
        let info = ProbeInfo {
            format,
            channels: i.channels,
            sample_rate: i.sample_rate,
            frames: i.frames,
            encoding: i.encoding(),
        };
        return Ok((info, out));
    }
    let mut reader = open_symphonia(path)?;
    let track = reader.default_track(TrackType::Audio).ok_or(MediaError::Empty)?.clone();
    let params = track.codec_params.as_ref().and_then(|c| c.audio()).ok_or(MediaError::Empty)?.clone();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|e| MediaError::Decode(e.to_string()))?;
    let mut out: Vec<f32> = Vec::new();
    let mut chunk: Vec<f32> = Vec::new();
    let mut channels = params.channels.as_ref().map_or(0, |c| c.count());
    loop {
        let packet = match reader.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(e) => return Err(MediaError::Decode(e.to_string())),
        };
        if packet.track_id != track.id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => {
                channels = buf.spec().channels().count();
                chunk.resize(buf.samples_interleaved(), 0.0);
                buf.copy_to_slice_interleaved(&mut chunk);
                out.extend_from_slice(&chunk);
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => {}
            Err(e) => return Err(MediaError::Decode(e.to_string())),
        }
    }
    if channels == 0 || out.is_empty() {
        return Err(MediaError::Empty);
    }
    if channels > 2 {
        return Err(MediaError::Unsupported("only mono and stereo files are supported".into()));
    }
    let frames = (out.len() / channels) as u64;
    let encoding = match (format, params.bits_per_sample) {
        (AudioFormat::Flac | AudioFormat::Aiff, Some(b)) => format!("PCM {b}-bit"),
        (AudioFormat::Mp3, _) => "MP3".into(),
        (AudioFormat::Aac, _) => "AAC".into(),
        _ => "PCM".into(),
    };
    let info =
        ProbeInfo { format, channels: channels as u16, sample_rate: params.sample_rate.unwrap_or(0), frames, encoding };
    Ok((info, out))
}

/// High-quality sample-rate conversion of interleaved audio (sinc, 256 taps, Blackman-Harris).
/// Deterministic; the output has `round(frames · to / from)` frames and no added delay.
pub fn resample(samples: &[f32], channels: usize, from: u32, to: u32) -> Result<Vec<f32>, MediaError> {
    if from == to {
        return Ok(samples.to_vec());
    }
    let frames = samples.len() / channels;
    let ratio = f64::from(to) / f64::from(from);
    let params = SincInterpolationParameters::new(256, WindowFunction::BlackmanHarris2)
        .oversampling_factor(256)
        .interpolation(SincInterpolationType::Cubic)
        .f_cutoff(0.95);
    let mut r = Async::<f64>::new_sinc(ratio, 1.0, &params, 1024, channels, FixedAsync::Input)
        .map_err(|e| MediaError::Decode(e.to_string()))?;
    let input: Vec<f64> = samples.iter().map(|&s| f64::from(s)).collect();
    let in_adapter = InterleavedSlice::new(&input, channels, frames).map_err(|e| MediaError::Decode(e.to_string()))?;
    let needed = r.process_all_needed_output_len(frames);
    let mut output = vec![0f64; needed * channels];
    let mut out_adapter =
        InterleavedSlice::new_mut(&mut output, channels, needed).map_err(|e| MediaError::Decode(e.to_string()))?;
    let (_, produced) = r
        .process_all_into_buffer(&in_adapter, &mut out_adapter, frames, None)
        .map_err(|e| MediaError::Decode(e.to_string()))?;
    let want = ((frames as f64) * ratio).round() as usize;
    let n = want.min(produced);
    let mut out: Vec<f32> = output[..n * channels].iter().map(|&s| s as f32).collect();
    out.resize(want * channels, 0.0);
    Ok(out)
}

/// Positional read without moving a shared cursor (portable: Unix `pread`, Windows `seek_read`).
pub fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_exact_at(buf, offset)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut done = 0;
        while done < buf.len() {
            let n = file.seek_read(&mut buf[done..], offset + done as u64)?;
            if n == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            done += n;
        }
        Ok(())
    }
}

pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut h = blake3::Hasher::new();
    let mut f = File::open(path)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().to_hex()[..32].to_string())
}

/// Copies (or converts) a file into the package (PRJ-3, PRJ-5) and builds its caches.
///
/// `subdir` is `audio` for stems and `references` for reference tracks. With `convert_to`, the file is resampled
/// to that rate and stored as a 32-bit float WAV; otherwise it is copied byte for byte.
pub fn import_into(package: &Path, src: &Path, subdir: &str, convert_to: Option<u32>) -> Result<MediaFile, MediaError> {
    let info = probe(src)?;
    let original_name = src.file_name().and_then(|n| n.to_str()).unwrap_or("audio").to_string();
    let dir = package.join(subdir);
    fs::create_dir_all(&dir)?;
    let media = match convert_to.filter(|&r| r != info.sample_rate) {
        Some(rate) => {
            let (info, samples) = decode_all(src)?;
            let converted = resample(&samples, usize::from(info.channels), info.sample_rate, rate)?;
            let tmp = dir.join(format!(".import-{}.wav", std::process::id()));
            wav::write_f32(&tmp, info.channels, rate, &converted)?;
            let id = hash_file(&tmp)?;
            let path = format!("{subdir}/{id}.wav");
            // Imported files are immutable (PRJ-3): an identical file already stored stays as it is.
            let dest = package.join(&path);
            if dest.exists() {
                fs::remove_file(&tmp)?;
            } else {
                fs::rename(&tmp, &dest)?;
                make_read_only(&dest)?;
            }
            MediaFile {
                id,
                path,
                original_name,
                format: AudioFormat::Wav,
                channels: info.channels,
                sample_rate: rate,
                frames: (converted.len() / usize::from(info.channels)) as u64,
                encoding: "Float 32-bit".into(),
                converted: true,
            }
        }
        None => {
            let id = hash_file(src)?;
            let path = format!("{subdir}/{id}.{}", extension(info.format));
            let dest = package.join(&path);
            if !dest.exists() {
                let tmp = dir.join(format!(".import-{}-{id}", std::process::id()));
                fs::copy(src, &tmp)?;
                fs::rename(&tmp, &dest)?;
                make_read_only(&dest)?;
            }
            MediaFile {
                id,
                path,
                original_name,
                format: info.format,
                channels: info.channels,
                sample_rate: info.sample_rate,
                frames: info.frames,
                encoding: info.encoding,
                converted: false,
            }
        }
    };
    ensure_cache(package, &media)?;
    Ok(media)
}

/// Stores a file written inside the package (the demo project's synthesised stems, PRJ-9) as an imported one,
/// named by its content hash and read-only, by moving it instead of copying it. `src` must be in the package.
pub fn adopt(package: &Path, src: &Path, subdir: &str) -> Result<MediaFile, MediaError> {
    let info = probe(src)?;
    let original_name = src.file_name().and_then(|n| n.to_str()).unwrap_or("audio").to_string();
    fs::create_dir_all(package.join(subdir))?;
    let id = hash_file(src)?;
    let path = format!("{subdir}/{id}.{}", extension(info.format));
    let dest = package.join(&path);
    if dest.exists() {
        fs::remove_file(src)?;
    } else {
        fs::rename(src, &dest)?;
        make_read_only(&dest)?;
    }
    let media = MediaFile {
        id,
        path,
        original_name,
        format: info.format,
        channels: info.channels,
        sample_rate: info.sample_rate,
        frames: info.frames,
        encoding: info.encoding,
        converted: false,
    };
    ensure_cache(package, &media)?;
    Ok(media)
}

/// Puts back the audio of a media whose file is missing from the package (UI-6, design 13d). `src` is imported as at
/// first (converted to the media's rate if needed). The same content comes back in place: `Ok(None)`, nothing in the
/// document changes. A different file with the same channels, rate and length becomes a new media for the clips to
/// use: `Ok(Some(new))`, applied with `AddMedia` and [`crate::Command::RelinkMedia`] as one undoable step. A file at
/// another rate is converted only if the missing one was itself converted at import (PRJ-5: the author chose it
/// then); otherwise it is refused, as is anything else, leaving nothing behind.
pub fn relink(
    package: &Path,
    project: &crate::model::Project,
    m: &MediaFile,
    src: &Path,
) -> Result<Option<MediaFile>, MediaError> {
    let info = probe(src)?;
    if info.channels != m.channels {
        return Err(MediaError::Unsupported(format!(
            "this file has {} channel{}, the missing one {}",
            info.channels,
            if info.channels == 1 { "" } else { "s" },
            m.channels
        )));
    }
    if info.sample_rate != m.sample_rate && !m.converted {
        return Err(MediaError::Unsupported(format!(
            "this file is at {} Hz, the missing one at {} Hz: choose the original file",
            info.sample_rate, m.sample_rate
        )));
    }
    let new = import_into(package, src, "audio", Some(m.sample_rate))?;
    // Removes what this attempt added (the stored file and its caches) when it is refused.
    let discard = |new: &MediaFile| {
        if project.media(&new.id).is_some() || new.path == m.path {
            return;
        }
        let dest = package.join(&new.path);
        if let Ok(meta) = fs::metadata(&dest) {
            let mut perm = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perm.set_readonly(false);
            let _ = fs::set_permissions(&dest, perm);
        }
        let _ = fs::remove_file(dest);
        let _ = fs::remove_file(peaks_path(package, &new.id));
        let _ = fs::remove_file(pcm_cache_path(package, &new.id));
    };
    if new.frames != m.frames {
        discard(&new);
        return Err(MediaError::Unsupported(format!(
            "this file has {} frames, the missing one {} ({:+} frames)",
            new.frames,
            m.frames,
            new.frames as i64 - m.frames as i64
        )));
    }
    if new.id != m.id && project.media(&new.id).is_some() {
        return Err(MediaError::Unsupported("that file is already used by another clip".into()));
    }
    if new.id == m.id {
        if new.path != m.path && !package.join(&m.path).is_file() {
            fs::copy(package.join(&new.path), package.join(&m.path))?;
        }
        return Ok(None);
    }
    Ok(Some(new))
}

fn make_read_only(path: &Path) -> io::Result<()> {
    let mut perm = fs::metadata(path)?.permissions();
    perm.set_readonly(true);
    fs::set_permissions(path, perm)
}

pub fn pcm_cache_path(package: &Path, id: &str) -> PathBuf {
    package.join("cache").join("pcm").join(format!("{id}.f32"))
}

pub fn peaks_path(package: &Path, id: &str) -> PathBuf {
    package.join("cache").join("peaks").join(format!("{id}.peaks"))
}

/// A temporary extension unique to this process and call: two threads building the same cache (the background
/// builder and an engine opening the file) never write the same temporary file; the last rename wins with a whole
/// file either way.
fn tmp_suffix() -> String {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!("tmp{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

/// Builds the decoded cache (non-WAV files) and the peak pyramid if they are missing.
pub fn ensure_cache(package: &Path, media: &MediaFile) -> Result<(), MediaError> {
    let src = package.join(&media.path);
    let pcm = pcm_cache_path(package, &media.id);
    let peaks = peaks_path(package, &media.id);
    let need_pcm = media.format != AudioFormat::Wav && !pcm.exists();
    if !need_pcm && peaks.exists() {
        return Ok(());
    }
    let (_, samples) = decode_all(&src)?;
    if need_pcm {
        fs::create_dir_all(pcm.parent().expect("has parent"))?;
        let tmp = pcm.with_extension(tmp_suffix());
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        fs::write(&tmp, bytes)?;
        fs::rename(tmp, &pcm)?;
    }
    if !peaks.exists() {
        fs::create_dir_all(peaks.parent().expect("has parent"))?;
        let p = Peaks::build(&samples, usize::from(media.channels));
        let tmp = peaks.with_extension(tmp_suffix());
        fs::write(&tmp, p.to_bytes())?;
        fs::rename(tmp, &peaks)?;
    }
    Ok(())
}

/// Random access to the samples of an imported file (reader threads and offline render; never the audio thread).
pub struct PcmSource {
    file: File,
    channels: u16,
    frames: u64,
    data_offset: u64,
    wav: Option<wav::WavInfo>,
    scratch: Vec<u8>,
}

impl PcmSource {
    pub fn open(package: &Path, media: &MediaFile) -> Result<Self, MediaError> {
        if media.format == AudioFormat::Wav {
            let path = package.join(&media.path);
            let info = wav::read_info(&path).map_err(|e| MediaError::Unsupported(e.to_string()))?;
            return Ok(PcmSource {
                file: File::open(path)?,
                channels: info.channels,
                frames: info.frames,
                data_offset: info.data_offset,
                wav: Some(info),
                scratch: Vec::new(),
            });
        }
        let path = pcm_cache_path(package, &media.id);
        if !path.exists() {
            ensure_cache(package, media)?;
        }
        let file = File::open(&path)?;
        let frames = file.metadata()?.len() / (4 * u64::from(media.channels));
        Ok(PcmSource { file, channels: media.channels, frames, data_offset: 0, wav: None, scratch: Vec::new() })
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Fills `out` (interleaved, `channels` per frame) from `frame`; frames past the end are zeros.
    pub fn read(&mut self, frame: u64, out: &mut [f32]) -> io::Result<()> {
        let ch = u64::from(self.channels);
        let want = out.len() as u64 / ch;
        let avail = self.frames.saturating_sub(frame).min(want);
        let (bytes_per_sample, info) = match &self.wav {
            Some(i) => (u64::from(i.bits / 8), Some(*i)),
            None => (4, None),
        };
        let n_bytes = (avail * ch * bytes_per_sample) as usize;
        self.scratch.resize(n_bytes, 0);
        if n_bytes > 0 {
            read_exact_at(&self.file, &mut self.scratch, self.data_offset + frame * ch * bytes_per_sample)?;
        }
        let filled = (avail * ch) as usize;
        match info {
            Some(i) => wav::decode_samples(&i, &self.scratch, &mut out[..filled]),
            None => {
                for (o, b) in out[..filled].iter_mut().zip(self.scratch.as_chunks::<4>().0) {
                    *o = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                }
            }
        }
        out[filled..].fill(0.0);
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// Peak pyramids (TRN-3)
// ---------------------------------------------------------------------------------------------

/// Minimum and maximum per block and channel, at block sizes 64 · 4ⁿ frames.
#[derive(Debug, Clone, PartialEq)]
pub struct Peaks {
    pub channels: usize,
    pub frames: u64,
    /// `levels[n]` holds blocks of `64 · 4ⁿ` frames: `[block][channel] = (min, max)`.
    pub levels: Vec<Vec<[i16; 2]>>,
}

pub const PEAK_BASE: u64 = 64;

fn q(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * 32767.0).round() as i16
}

impl Peaks {
    pub fn build(samples: &[f32], channels: usize) -> Peaks {
        let frames = (samples.len() / channels) as u64;
        let base = PEAK_BASE as usize;
        let blocks = frames.div_ceil(PEAK_BASE) as usize;
        let mut level0 = Vec::with_capacity(blocks * channels);
        for b in 0..blocks {
            let start = b * base;
            let end = ((b + 1) * base).min(frames as usize);
            for c in 0..channels {
                let mut lo = f32::MAX;
                let mut hi = f32::MIN;
                for f in start..end {
                    let v = samples[f * channels + c];
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
                level0.push([q(lo), q(hi)]);
            }
        }
        let mut levels = vec![level0];
        while levels.last().expect("non-empty").len() / channels > 1 {
            let prev = levels.last().expect("non-empty");
            let n = (prev.len() / channels).div_ceil(4);
            let mut next = Vec::with_capacity(n * channels);
            for b in 0..n {
                for c in 0..channels {
                    let mut lo = i16::MAX;
                    let mut hi = i16::MIN;
                    for k in b * 4..((b + 1) * 4).min(prev.len() / channels) {
                        lo = lo.min(prev[k * channels + c][0]);
                        hi = hi.max(prev[k * channels + c][1]);
                    }
                    next.push([lo, hi]);
                }
            }
            levels.push(next);
        }
        Peaks { channels, frames, levels }
    }

    /// `columns` (min, max) pairs per channel covering `[start, end)` frames of the file, from the coarsest
    /// level that still has at least one block per column. Values are in −1…1.
    #[allow(clippy::needless_range_loop)]
    pub fn query(&self, start: u64, end: u64, columns: usize) -> Vec<Vec<[f32; 2]>> {
        let mut out = vec![vec![[0.0f32; 2]; columns]; self.channels];
        if columns == 0 || end <= start {
            return out;
        }
        let per_col = (end - start) as f64 / columns as f64;
        let mut level = 0;
        while level + 1 < self.levels.len() && (PEAK_BASE * 4u64.pow(level as u32 + 1)) as f64 <= per_col {
            level += 1;
        }
        let bsize = PEAK_BASE * 4u64.pow(level as u32);
        let data = &self.levels[level];
        let nblocks = data.len() / self.channels;
        for col in 0..columns {
            let a = start as f64 + col as f64 * per_col;
            let b = a + per_col;
            let ba = (a / bsize as f64).floor() as usize;
            let bb = ((b / bsize as f64).ceil() as usize).max(ba + 1).min(nblocks);
            for c in 0..self.channels {
                let mut lo = i16::MAX;
                let mut hi = i16::MIN;
                for k in ba..bb {
                    lo = lo.min(data[k * self.channels + c][0]);
                    hi = hi.max(data[k * self.channels + c][1]);
                }
                if lo <= hi {
                    out[c][col] = [f32::from(lo) / 32767.0, f32::from(hi) / 32767.0];
                }
            }
        }
        out
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"SWPK");
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&(self.channels as u32).to_le_bytes());
        b.extend_from_slice(&self.frames.to_le_bytes());
        b.extend_from_slice(&(self.levels.len() as u32).to_le_bytes());
        for l in &self.levels {
            b.extend_from_slice(&(l.len() as u64).to_le_bytes());
            for [lo, hi] in l {
                b.extend_from_slice(&lo.to_le_bytes());
                b.extend_from_slice(&hi.to_le_bytes());
            }
        }
        b
    }

    pub fn from_bytes(b: &[u8]) -> Option<Peaks> {
        if b.len() < 24 || &b[0..4] != b"SWPK" {
            return None;
        }
        let u32_at = |o: usize| b.get(o..o + 4).map(|s| u32::from_le_bytes(s.try_into().expect("4")));
        let u64_at = |o: usize| b.get(o..o + 8).map(|s| u64::from_le_bytes(s.try_into().expect("8")));
        let channels = u32_at(8)? as usize;
        let frames = u64_at(12)?;
        let n = u32_at(20)? as usize;
        let mut o = 24;
        let mut levels = Vec::with_capacity(n);
        for _ in 0..n {
            let len = u64_at(o)? as usize;
            o += 8;
            let mut l = Vec::with_capacity(len);
            for _ in 0..len {
                let s = b.get(o..o + 4)?;
                l.push([i16::from_le_bytes([s[0], s[1]]), i16::from_le_bytes([s[2], s[3]])]);
                o += 4;
            }
            levels.push(l);
        }
        Some(Peaks { channels, frames, levels })
    }

    pub fn load(package: &Path, id: &str) -> Option<Peaks> {
        Peaks::from_bytes(&fs::read(peaks_path(package, id)).ok()?)
    }
}
