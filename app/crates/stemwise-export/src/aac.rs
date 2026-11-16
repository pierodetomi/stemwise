//! AAC in an M4A container through the system encoder (A-005, MAS-10), behind one interface: AudioToolbox
//! `ExtAudioFile` on macOS, the Media Foundation sink writer on Windows (A-104), each through a small FFI kept inside
//! this module. Elsewhere (Linux: no system encoder, none with a permissive licence) the format is not offered.

use std::path::Path;

/// Whether this system can encode AAC (the UI hides the format otherwise, MAS-10).
pub fn aac_available() -> bool {
    sys::encoder_available()
}

/// Encodes interleaved float samples (−1…1) to AAC-LC in an M4A file at `kbps` (e.g. 256). Returns the bit rate
/// the file was written at: the one asked for, or the highest the system's encoder takes when it takes fewer (the
/// Media Foundation encoder of older Windows 10 stops at 192).
pub fn encode_aac(path: &Path, samples: &[f32], channels: u16, rate: u32, kbps: u32) -> crate::Result<u32> {
    if !matches!(channels, 1 | 2) {
        return Err(crate::ExportError::Unsupported(format!("AAC with {channels} channels")));
    }
    // AudioToolbox's AAC encoder stops at 48 kHz (F8 review, B3), and so does Media Foundation's.
    if !matches!(rate, 44_100 | 48_000) {
        return Err(crate::ExportError::Unsupported(format!("AAC at {rate} Hz")));
    }
    if kbps == 0 {
        return Err(crate::ExportError::Unsupported("AAC at 0 kbps".to_string()));
    }
    sys::encode(path, samples, channels, rate, kbps)
}

/// No AAC encoder on this system: the format is not offered.
#[cfg(not(any(target_os = "macos", windows)))]
mod sys {
    use std::path::Path;

    pub fn encoder_available() -> bool {
        false
    }

    pub fn encode(_: &Path, _: &[f32], _: u16, _: u32, _: u32) -> crate::Result<u32> {
        Err(crate::ExportError::Unsupported("AAC encoding is not available on this system".to_string()))
    }
}

/// Media Foundation (Windows): the sink writer finds the system's AAC encoder and writes the MPEG-4 container.
#[cfg(windows)]
mod sys {
    use std::path::Path;

    use windows::Win32::Media::MediaFoundation::{
        IMFActivate, IMFAttributes, MF_MT_AUDIO_AVG_BYTES_PER_SECOND, MF_MT_AUDIO_BITS_PER_SAMPLE,
        MF_MT_AUDIO_BLOCK_ALIGNMENT, MF_MT_AUDIO_NUM_CHANNELS, MF_MT_AUDIO_SAMPLES_PER_SECOND, MF_MT_MAJOR_TYPE,
        MF_MT_SUBTYPE, MF_TRANSCODE_CONTAINERTYPE, MF_VERSION, MFAudioFormat_AAC, MFAudioFormat_PCM,
        MFCreateAttributes, MFCreateMediaType, MFCreateMemoryBuffer, MFCreateSample, MFCreateSinkWriterFromURL,
        MFMediaType_Audio, MFSTARTUP_LITE, MFShutdown, MFStartup, MFT_CATEGORY_AUDIO_ENCODER, MFT_ENUM_FLAG_LOCALMFT,
        MFT_ENUM_FLAG_SORTANDFILTER, MFT_ENUM_FLAG_SYNCMFT, MFT_REGISTER_TYPE_INFO, MFTEnumEx,
        MFTranscodeContainerType_MPEG4,
    };
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize};
    use windows::core::HSTRING;

    use crate::ExportError;

    /// The highest rate every Media Foundation AAC encoder takes (24 000 bytes per second); Windows 11 takes 256 and
    /// 320 too.
    const SAFE_KBPS: u32 = 192;
    /// Frames handed to the writer at a time.
    const CHUNK_FRAMES: usize = 4096;
    /// Media Foundation counts time in units of 100 ns.
    const UNITS_PER_SECOND: i64 = 10_000_000;

    /// COM on this thread for the length of a call; left alone when the thread already has another mode.
    struct Com(bool);
    impl Com {
        fn init() -> Self {
            // SAFETY: no reserved pointer; a thread may initialise COM more than once if it uninitialises as often.
            Com(unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok())
        }
    }
    impl Drop for Com {
        fn drop(&mut self) {
            if self.0 {
                // SAFETY: pairs the successful `CoInitializeEx` of `init`.
                unsafe { CoUninitialize() };
            }
        }
    }

    /// Media Foundation started for the length of a call.
    struct Mf;
    impl Mf {
        fn start() -> windows::core::Result<Self> {
            // SAFETY: plain call; every successful start is paired by the shutdown in `drop`.
            unsafe { MFStartup(MF_VERSION, MFSTARTUP_LITE) }.map(|()| Mf)
        }
    }
    impl Drop for Mf {
        fn drop(&mut self) {
            // SAFETY: pairs the successful `MFStartup` of `start`.
            let _ = unsafe { MFShutdown() };
        }
    }

    fn fail(what: &str) -> impl Fn(windows::core::Error) -> ExportError + '_ {
        move |e| ExportError::Encoder(format!("{what} failed: {e}"))
    }

    pub fn encoder_available() -> bool {
        let _com = Com::init();
        let Ok(_mf) = Mf::start() else { return false };
        let aac = MFT_REGISTER_TYPE_INFO { guidMajorType: MFMediaType_Audio, guidSubtype: MFAudioFormat_AAC };
        let mut list: *mut Option<IMFActivate> = std::ptr::null_mut();
        let mut count = 0u32;
        let flags = MFT_ENUM_FLAG_SYNCMFT | MFT_ENUM_FLAG_LOCALMFT | MFT_ENUM_FLAG_SORTANDFILTER;
        // SAFETY: the type and the two out-parameters are live locals; the call fills an array it allocates.
        let found = unsafe { MFTEnumEx(MFT_CATEGORY_AUDIO_ENCODER, flags, None, Some(&aac), &mut list, &mut count) };
        if !list.is_null() {
            for i in 0..count as usize {
                // SAFETY: `list` holds `count` activation objects owned by the caller: each is released once.
                unsafe { std::ptr::drop_in_place(list.add(i)) };
            }
            // SAFETY: the array was allocated by the call with the COM allocator.
            unsafe { CoTaskMemFree(Some(list.cast_const().cast())) };
        }
        found.is_ok() && count > 0
    }

    pub fn encode(path: &Path, samples: &[f32], channels: u16, rate: u32, kbps: u32) -> crate::Result<u32> {
        let _com = Com::init();
        let _mf = Mf::start().map_err(fail("MFStartup"))?;
        let attempt = |kbps: u32| {
            let _ = std::fs::remove_file(path);
            let written = write(path, samples, channels, rate, kbps).map(|()| kbps);
            if written.is_err() {
                let _ = std::fs::remove_file(path);
            }
            written
        };
        match attempt(kbps) {
            // An encoder that refuses the rate takes the highest one every encoder takes.
            Err(_) if kbps > SAFE_KBPS => attempt(SAFE_KBPS),
            done => done,
        }
    }

    fn write(path: &Path, samples: &[f32], channels: u16, rate: u32, kbps: u32) -> crate::Result<()> {
        let ch = u32::from(channels);
        // SAFETY (whole block): every call is a Media Foundation method on an object created just above and alive
        // for the call; the buffer is written only between its `Lock` and `Unlock`, within the length it reports.
        unsafe {
            let mut attributes: Option<IMFAttributes> = None;
            MFCreateAttributes(&mut attributes, 1).map_err(fail("MFCreateAttributes"))?;
            let attributes =
                attributes.ok_or_else(|| ExportError::Encoder("MFCreateAttributes returned nothing".into()))?;
            attributes
                .SetGUID(&MF_TRANSCODE_CONTAINERTYPE, &MFTranscodeContainerType_MPEG4)
                .map_err(fail("container type"))?;
            let url = HSTRING::from(path);
            let writer =
                MFCreateSinkWriterFromURL(&url, None, &attributes).map_err(fail("MFCreateSinkWriterFromURL"))?;

            let out = MFCreateMediaType().map_err(fail("MFCreateMediaType"))?;
            out.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).map_err(fail("output type"))?;
            out.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC).map_err(fail("output type"))?;
            out.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).map_err(fail("output type"))?;
            out.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, rate).map_err(fail("output type"))?;
            out.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, ch).map_err(fail("output type"))?;
            out.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, kbps * 1000 / 8).map_err(fail("output type"))?;
            let stream = writer.AddStream(&out).map_err(fail("AddStream"))?;

            // The AAC encoder takes 16-bit PCM.
            let input = MFCreateMediaType().map_err(fail("MFCreateMediaType"))?;
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).map_err(fail("input type"))?;
            input.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM).map_err(fail("input type"))?;
            input.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).map_err(fail("input type"))?;
            input.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, rate).map_err(fail("input type"))?;
            input.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, ch).map_err(fail("input type"))?;
            input.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 2 * ch).map_err(fail("input type"))?;
            input.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 2 * ch * rate).map_err(fail("input type"))?;
            writer.SetInputMediaType(stream, &input, None).map_err(fail("SetInputMediaType"))?;

            writer.BeginWriting().map_err(fail("BeginWriting"))?;
            let n = usize::from(channels);
            let time = |frame: usize| frame as i64 * UNITS_PER_SECOND / i64::from(rate);
            let mut frame = 0usize;
            for block in samples.chunks(CHUNK_FRAMES * n) {
                let frames = block.len() / n;
                if frames == 0 {
                    break;
                }
                let bytes = (frames * n * 2) as u32;
                let buffer = MFCreateMemoryBuffer(bytes).map_err(fail("MFCreateMemoryBuffer"))?;
                let mut data: *mut u8 = std::ptr::null_mut();
                let mut max = 0u32;
                buffer.Lock(&mut data, Some(&mut max), None).map_err(fail("Lock"))?;
                if data.is_null() || max < bytes {
                    let _ = buffer.Unlock();
                    return Err(ExportError::Encoder("the encoder's buffer is too small".into()));
                }
                let dst = std::slice::from_raw_parts_mut(data, bytes as usize);
                for (d, v) in dst.as_chunks_mut::<2>().0.iter_mut().zip(&block[..frames * n]) {
                    *d = ((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes();
                }
                buffer.Unlock().map_err(fail("Unlock"))?;
                buffer.SetCurrentLength(bytes).map_err(fail("SetCurrentLength"))?;
                let sample = MFCreateSample().map_err(fail("MFCreateSample"))?;
                sample.AddBuffer(&buffer).map_err(fail("AddBuffer"))?;
                sample.SetSampleTime(time(frame)).map_err(fail("SetSampleTime"))?;
                sample.SetSampleDuration(time(frame + frames) - time(frame)).map_err(fail("SetSampleDuration"))?;
                writer.WriteSample(stream, &sample).map_err(fail("WriteSample"))?;
                frame += frames;
            }
            writer.Finalize().map_err(fail("Finalize"))?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod sys {
    use std::ffi::c_void;
    use std::os::raw::c_long;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::ptr;

    use crate::ExportError;

    type OSStatus = i32;
    type CFURLRef = *const c_void;
    type ExtAudioFileRef = *mut c_void;

    const fn fourcc(s: &[u8; 4]) -> u32 {
        u32::from_be_bytes(*s)
    }

    const K_AUDIO_FILE_M4A_TYPE: u32 = fourcc(b"m4af");
    const K_AUDIO_FORMAT_MPEG4_AAC: u32 = fourcc(b"aac ");
    const K_AUDIO_FORMAT_LINEAR_PCM: u32 = fourcc(b"lpcm");
    const K_AUDIO_FORMAT_FLAG_IS_FLOAT: u32 = 1;
    const K_AUDIO_FORMAT_FLAG_IS_PACKED: u32 = 8;
    const K_AUDIO_FILE_FLAGS_ERASE_FILE: u32 = 1;
    const K_AUDIO_FORMAT_PROPERTY_ENCODERS: u32 = fourcc(b"aven");
    const K_EXT_AUDIO_FILE_PROPERTY_CLIENT_DATA_FORMAT: u32 = fourcc(b"cfmt");
    const K_EXT_AUDIO_FILE_PROPERTY_AUDIO_CONVERTER: u32 = fourcc(b"acnv");
    const K_EXT_AUDIO_FILE_PROPERTY_CONVERTER_CONFIG: u32 = fourcc(b"accf");
    const K_AUDIO_CONVERTER_ENCODE_BIT_RATE: u32 = fourcc(b"brat");

    /// Frames handed to the encoder per `ExtAudioFileWrite` call.
    const CHUNK_FRAMES: usize = 4096;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct AudioStreamBasicDescription {
        sample_rate: f64,
        format_id: u32,
        format_flags: u32,
        bytes_per_packet: u32,
        frames_per_packet: u32,
        bytes_per_frame: u32,
        channels_per_frame: u32,
        bits_per_channel: u32,
        reserved: u32,
    }

    #[repr(C)]
    struct AudioBuffer {
        number_channels: u32,
        data_byte_size: u32,
        data: *mut c_void,
    }

    #[repr(C)]
    struct AudioBufferList {
        number_buffers: u32,
        buffers: [AudioBuffer; 1],
    }

    #[link(name = "AudioToolbox", kind = "framework")]
    unsafe extern "C" {
        fn ExtAudioFileCreateWithURL(
            url: CFURLRef,
            file_type: u32,
            format: *const AudioStreamBasicDescription,
            channel_layout: *const c_void,
            flags: u32,
            out_file: *mut ExtAudioFileRef,
        ) -> OSStatus;
        fn ExtAudioFileSetProperty(file: ExtAudioFileRef, property: u32, size: u32, data: *const c_void) -> OSStatus;
        fn ExtAudioFileGetProperty(file: ExtAudioFileRef, property: u32, size: *mut u32, data: *mut c_void)
        -> OSStatus;
        fn ExtAudioFileWrite(file: ExtAudioFileRef, frames: u32, data: *const AudioBufferList) -> OSStatus;
        fn ExtAudioFileDispose(file: ExtAudioFileRef) -> OSStatus;
        fn AudioConverterSetProperty(converter: *mut c_void, property: u32, size: u32, data: *const c_void)
        -> OSStatus;
        fn AudioFormatGetPropertyInfo(
            property: u32,
            specifier_size: u32,
            specifier: *const c_void,
            out_size: *mut u32,
        ) -> OSStatus;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFURLCreateFromFileSystemRepresentation(
            allocator: *const c_void,
            buffer: *const u8,
            length: c_long,
            is_directory: u8,
        ) -> CFURLRef;
        fn CFRelease(object: *const c_void);
    }

    /// Four-character code when printable, the number otherwise.
    fn describe(status: OSStatus) -> String {
        let bytes = status.to_be_bytes();
        if bytes.iter().all(|b| (0x20..0x7f).contains(b)) {
            format!("'{}' ({status})", String::from_utf8_lossy(&bytes))
        } else {
            status.to_string()
        }
    }

    fn check(function: &str, status: OSStatus) -> crate::Result<()> {
        if status == 0 {
            Ok(())
        } else {
            Err(ExportError::Encoder(format!("{function} failed with status {}", describe(status))))
        }
    }

    pub fn encoder_available() -> bool {
        let specifier = K_AUDIO_FORMAT_MPEG4_AAC;
        let mut size = 0u32;
        // SAFETY: `specifier` and `size` are live locals of the sizes passed; the call only reads the former and
        // writes the latter.
        let status = unsafe {
            AudioFormatGetPropertyInfo(
                K_AUDIO_FORMAT_PROPERTY_ENCODERS,
                size_of::<u32>() as u32,
                (&raw const specifier).cast(),
                &mut size,
            )
        };
        status == 0 && size > 0
    }

    /// An open `ExtAudioFile`, disposed on drop unless `finish` was called.
    struct OpenFile(ExtAudioFileRef);

    impl OpenFile {
        fn finish(mut self) -> crate::Result<()> {
            let file = std::mem::replace(&mut self.0, ptr::null_mut());
            // SAFETY: `file` is a valid reference returned by the create call and not used afterwards.
            check("ExtAudioFileDispose", unsafe { ExtAudioFileDispose(file) })
        }
    }

    impl Drop for OpenFile {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: the reference is valid and disposed exactly once.
                let _ = unsafe { ExtAudioFileDispose(self.0) };
            }
        }
    }

    /// A CFURL released on drop.
    struct Url(CFURLRef);

    impl Url {
        fn from_path(path: &Path) -> crate::Result<Url> {
            let bytes = path.as_os_str().as_bytes();
            // SAFETY: the buffer pointer and length describe the live `bytes` slice, which CoreFoundation copies.
            let url = unsafe {
                CFURLCreateFromFileSystemRepresentation(ptr::null(), bytes.as_ptr(), bytes.len() as c_long, 0)
            };
            if url.is_null() {
                Err(ExportError::Encoder("CFURLCreateFromFileSystemRepresentation returned NULL".to_string()))
            } else {
                Ok(Url(url))
            }
        }
    }

    impl Drop for Url {
        fn drop(&mut self) {
            // SAFETY: the URL was created with a Create function and is released exactly once.
            unsafe { CFRelease(self.0) };
        }
    }

    /// AudioToolbox takes any bit rate (it uses its nearest).
    pub fn encode(path: &Path, samples: &[f32], channels: u16, rate: u32, kbps: u32) -> crate::Result<u32> {
        encode_at(path, samples, channels, rate, kbps).map(|()| kbps)
    }

    fn encode_at(path: &Path, samples: &[f32], channels: u16, rate: u32, kbps: u32) -> crate::Result<()> {
        let mut name = path.as_os_str().to_owned();
        name.push(".part");
        let temp = PathBuf::from(name);
        let result = write_file(&temp, samples, channels, rate, kbps).and_then(|()| {
            std::fs::rename(&temp, path)?;
            Ok(())
        });
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result
    }

    fn write_file(path: &Path, samples: &[f32], channels: u16, rate: u32, kbps: u32) -> crate::Result<()> {
        let channels_n = usize::from(channels);
        let url = Url::from_path(path)?;

        let file_format = AudioStreamBasicDescription {
            sample_rate: f64::from(rate),
            format_id: K_AUDIO_FORMAT_MPEG4_AAC,
            frames_per_packet: 1024,
            channels_per_frame: u32::from(channels),
            ..Default::default()
        };
        let mut raw: ExtAudioFileRef = ptr::null_mut();
        // SAFETY: `url` is a valid CFURL, `file_format` a live ASBD, a NULL channel layout is allowed, and `raw` a
        // live out-pointer.
        check("ExtAudioFileCreateWithURL", unsafe {
            ExtAudioFileCreateWithURL(
                url.0,
                K_AUDIO_FILE_M4A_TYPE,
                &file_format,
                ptr::null(),
                K_AUDIO_FILE_FLAGS_ERASE_FILE,
                &mut raw,
            )
        })?;
        let file = OpenFile(raw);

        let client_format = AudioStreamBasicDescription {
            sample_rate: f64::from(rate),
            format_id: K_AUDIO_FORMAT_LINEAR_PCM,
            format_flags: K_AUDIO_FORMAT_FLAG_IS_FLOAT | K_AUDIO_FORMAT_FLAG_IS_PACKED,
            bytes_per_packet: 4 * u32::from(channels),
            frames_per_packet: 1,
            bytes_per_frame: 4 * u32::from(channels),
            channels_per_frame: u32::from(channels),
            bits_per_channel: 32,
            reserved: 0,
        };
        // SAFETY: `file.0` is open; the data pointer and size describe `client_format`.
        check("ExtAudioFileSetProperty(ClientDataFormat)", unsafe {
            ExtAudioFileSetProperty(
                file.0,
                K_EXT_AUDIO_FILE_PROPERTY_CLIENT_DATA_FORMAT,
                size_of::<AudioStreamBasicDescription>() as u32,
                (&raw const client_format).cast(),
            )
        })?;

        let mut converter: *mut c_void = ptr::null_mut();
        let mut converter_size = size_of::<*mut c_void>() as u32;
        // SAFETY: `file.0` is open; the out-pointers describe a pointer-sized local and its size.
        check("ExtAudioFileGetProperty(AudioConverter)", unsafe {
            ExtAudioFileGetProperty(
                file.0,
                K_EXT_AUDIO_FILE_PROPERTY_AUDIO_CONVERTER,
                &mut converter_size,
                (&raw mut converter).cast(),
            )
        })?;
        let bit_rate = kbps.saturating_mul(1000);
        // SAFETY: `converter` was just returned by the file and stays valid while the file is open; the data is a
        // live u32 of the size passed.
        check("AudioConverterSetProperty(EncodeBitRate)", unsafe {
            AudioConverterSetProperty(
                converter,
                K_AUDIO_CONVERTER_ENCODE_BIT_RATE,
                size_of::<u32>() as u32,
                (&raw const bit_rate).cast(),
            )
        })?;
        // Setting a NULL converter configuration makes the file re-read the converter and apply the new bit rate.
        let null_config: *const c_void = ptr::null();
        // SAFETY: `file.0` is open; the data is a live pointer-sized value holding NULL, which is allowed.
        check("ExtAudioFileSetProperty(ConverterConfig)", unsafe {
            ExtAudioFileSetProperty(
                file.0,
                K_EXT_AUDIO_FILE_PROPERTY_CONVERTER_CONFIG,
                size_of::<*const c_void>() as u32,
                (&raw const null_config).cast(),
            )
        })?;

        let mut chunk: Vec<f32> = Vec::with_capacity(CHUNK_FRAMES * channels_n);
        for block in samples.chunks(CHUNK_FRAMES * channels_n) {
            let frames = block.len() / channels_n;
            if frames == 0 {
                break;
            }
            chunk.clear();
            chunk.extend_from_slice(&block[..frames * channels_n]);
            let list = AudioBufferList {
                number_buffers: 1,
                buffers: [AudioBuffer {
                    number_channels: u32::from(channels),
                    data_byte_size: (chunk.len() * size_of::<f32>()) as u32,
                    data: chunk.as_mut_ptr().cast(),
                }],
            };
            // SAFETY: `file.0` is open; `list` describes one buffer over `chunk`, which holds `frames` interleaved
            // frames and outlives the call.
            check("ExtAudioFileWrite", unsafe { ExtAudioFileWrite(file.0, frames as u32, &list) })?;
        }
        file.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: MAS-10
    #[test]
    fn rejects_unsupported_layouts_without_panicking() {
        let dir = std::env::temp_dir().join("stemwise-aac-unit-never-created.m4a");
        assert!(matches!(encode_aac(&dir, &[0.0; 6], 3, 48_000, 256), Err(crate::ExportError::Unsupported(_))));
        assert!(matches!(encode_aac(&dir, &[0.0; 6], 2, 12_345, 256), Err(crate::ExportError::Unsupported(_))));
    }
}
