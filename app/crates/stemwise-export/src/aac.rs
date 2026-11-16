//! AAC in an M4A container through the system encoder (A-005, MAS-10): AudioToolbox `ExtAudioFile` on macOS, through
//! a small FFI kept inside this module. Elsewhere the format is not offered.

use std::path::Path;

/// Whether this system can encode AAC (the UI hides the format otherwise, MAS-10).
pub fn aac_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        sys::encoder_available()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Encodes interleaved float samples (−1…1) to AAC-LC in an M4A file at `kbps` (e.g. 256).
pub fn encode_aac(path: &Path, samples: &[f32], channels: u16, rate: u32, kbps: u32) -> crate::Result<()> {
    if !matches!(channels, 1 | 2) {
        return Err(crate::ExportError::Unsupported(format!("AAC with {channels} channels")));
    }
    // AudioToolbox's AAC encoder stops at 48 kHz (F8 review, B3).
    if !matches!(rate, 44_100 | 48_000) {
        return Err(crate::ExportError::Unsupported(format!("AAC at {rate} Hz")));
    }
    if kbps == 0 {
        return Err(crate::ExportError::Unsupported("AAC at 0 kbps".to_string()));
    }
    #[cfg(target_os = "macos")]
    {
        sys::encode(path, samples, channels, rate, kbps)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (path, samples);
        Err(crate::ExportError::Unsupported("AAC encoding is only available on macOS".to_string()))
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

    pub fn encode(path: &Path, samples: &[f32], channels: u16, rate: u32, kbps: u32) -> crate::Result<()> {
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
