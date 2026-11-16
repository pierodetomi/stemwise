//! Stemwise core: the project document, commands and history, the package on disk, imports and peaks
//! (FOUNDATION §5.1–§5.3). Knows nothing of audio processing, the UI or Tauri.

pub mod automation;
pub mod command;
pub mod demo;
pub mod edit;
pub mod fx;
pub mod history;
pub mod import;
pub mod media;
pub mod model;
pub mod package;
pub mod params;
pub mod role;
pub mod routing;
pub mod schema;
pub mod wav;

pub use command::Command;
pub use history::{Change, Document, Phase};
pub use model::*;
pub use role::Role;
