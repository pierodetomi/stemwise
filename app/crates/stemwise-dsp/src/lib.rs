//! Stemwise DSP: filters, dynamics, saturation, reverb, delay, limiter, meters, loudness and true peak
//! (FX-1 … FX-6, MAS-1, MAS-4). Real-time safe: every `process` runs without allocating, locking or doing I/O.
//! Depends on no other Stemwise crate.

// Channel loops index several parallel buffers at once; iterator chains would obscure the DSP.
#![allow(clippy::needless_range_loop)]

pub mod biquad;
pub mod clipper;
pub mod color;
pub mod compressor;
pub mod deesser;
pub mod delay;
pub mod dither;
pub mod dyneq;
pub mod gate;
pub mod limiter;
pub mod loudness;
pub mod master;
pub mod meters;
pub mod modulation;
pub mod multiband;
pub mod oversample;
pub mod pan;
pub mod pitch;
pub mod reverb;
#[cfg(test)]
mod revmeasure;
pub mod saturation;
pub mod testsignal;
pub mod transient;
pub mod truepeak;
pub mod util;
pub mod width;

pub use util::*;
