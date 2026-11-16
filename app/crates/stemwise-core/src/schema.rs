//! Exported contracts (FOUNDATION §5.5, §5.6): JSON Schemas in `app/schemas/` and TypeScript types in
//! `app/ui/src/bindings/`, generated from the Rust types (one definition of every type).

use std::fs;
use std::io;
use std::path::Path;

use ts_rs::{Config, TS};

use crate::command::Command;
use crate::edit::Edit;
use crate::history::{HistoryEntry, Phase};
use crate::import::{ImportCandidate, ImportChoice};
use crate::model::{MixState, Project};
use crate::package::RecentProject;
use crate::params::ParamSpec;

/// JSON Schemas of the core contracts, as (file name, pretty JSON).
pub fn json_schemas() -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let mut add = |name: &'static str, schema: schemars::Schema| {
        let mut s = serde_json::to_string_pretty(&schema).expect("schema serialises");
        s.push('\n');
        out.push((name, s));
    };
    add("project.schema.json", schemars::schema_for!(Project));
    add("mix_state.schema.json", schemars::schema_for!(MixState));
    add("command.schema.json", schemars::schema_for!(Command));
    add("edit.schema.json", schemars::schema_for!(Edit));
    out
}

/// Writes the core schemas to `dir`.
pub fn write_schemas(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    for (name, text) in json_schemas() {
        fs::write(dir.join(name), text)?;
    }
    Ok(())
}

/// Writes the TypeScript bindings of the core types to `dir`.
pub fn write_bindings(dir: &Path) -> Result<(), ts_rs::ExportError> {
    let cfg = Config::new().with_out_dir(dir).with_large_int("number");
    Project::export_all(&cfg)?;
    MixState::export_all(&cfg)?;
    Command::export_all(&cfg)?;
    Edit::export_all(&cfg)?;
    HistoryEntry::export_all(&cfg)?;
    Phase::export_all(&cfg)?;
    ParamSpec::export_all(&cfg)?;
    crate::fx::FxInfo::export_all(&cfg)?;
    ImportCandidate::export_all(&cfg)?;
    ImportChoice::export_all(&cfg)?;
    RecentProject::export_all(&cfg)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: PRJ-8
    #[test]
    fn exported_schemas_are_up_to_date_and_validate_a_project() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
        for (name, text) in json_schemas() {
            let committed = fs::read_to_string(dir.join(name)).unwrap_or_default();
            assert!(committed == text, "app/schemas/{name} is stale: run `make schemas`");
        }
        // The schema accepts what the core writes.
        let schema: serde_json::Value = serde_json::from_str(&json_schemas()[0].1).unwrap();
        assert_eq!(schema["title"], "Project");
        let required = schema["required"].as_array().unwrap();
        let doc = serde_json::to_value(Project::new("x")).unwrap();
        for r in required {
            assert!(doc.get(r.as_str().unwrap()).is_some(), "{r}");
        }
    }
}
