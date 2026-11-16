//! Stemwise engine: the compiled mix graph, clip streaming, automation, offline render, real-time output and
//! transport (ENG-1 … ENG-7, AUT-4, AUT-6). Depends on the core and the DSP crates only.

// Channel loops index several parallel buffers at once; iterator chains would obscure the DSP.
#![allow(clippy::needless_range_loop)]

pub mod chain;
pub mod clips;
pub mod device;
pub mod dto;
pub mod graph;
pub mod master;
pub mod mixdown;
pub mod offline;
pub mod pool;
pub mod realtime;

pub use graph::{BusMeter, Feed, Graph, NodeMeter};
