//! Export of the master (MAS-7, MAS-9, MAS-10): file writers and encoders, isolated from the engine.
//!
//! - [`wav`]: RIFF/WAVE writer, 16/24-bit PCM or 32-bit float, with `bext`.
//! - [`flac`]: own FLAC encoder (A-013), 16/24 bit.
//! - [`aac`]: AAC in M4A through the operating system's encoder (AudioToolbox on macOS, A-005); not offered elsewhere.
//! - [`tags`]: metadata in each format's own tags (LIST/INFO and ID3 for WAV, Vorbis comments and picture for FLAC,
//!   atoms for M4A), with cover.
//!
//! Samples arrive already processed by the master chain; integer formats arrive already quantised (with TPDF dither at
//! 16 bit, done by the caller), so every writer here is deterministic (MAS-8).

pub mod aac;
pub mod dto;
pub mod flac;
pub mod pipeline;
pub mod tags;
pub mod wav;

/// Errors of the writers and encoders.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Encoder(String),
    #[error("{0}")]
    Tags(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, ExportError>;
