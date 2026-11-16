//! Stemwise analysis: the PoC's measures ported to Rust (ANA-1 … ANA-12, poc/docs/analysis.md), plus measures over
//! sections and families (ANA-15, ANA-16) and the evaluation of a mix against a target (EVA of the PoC).
//! Depends on the DSP crate only.

// Measure code indexes parallel per-band and per-channel arrays.
#![allow(clippy::needless_range_loop)]

pub mod character;
pub mod evaluate;
pub mod measures;
pub mod onsets;
pub mod sections;
pub mod spectral;
