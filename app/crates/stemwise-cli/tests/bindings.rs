//! The JSON the interface reads its block catalog and parameter specs from is generated from the Rust types: the
//! committed files must be the ones `make schemas` writes, or the interface shows limits the core does not have.

use std::path::Path;

/// req: UI-12, FX-10
#[test]
fn the_generated_catalog_and_specs_are_up_to_date() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/bindings");
    let fresh = [
        ("param_specs.json", serde_json::to_string_pretty(&stemwise_core::params::all_specs()).unwrap() + "\n"),
        ("fx_catalog.json", serde_json::to_string_pretty(&stemwise_engine::chain::catalog()).unwrap() + "\n"),
    ];
    for (name, text) in fresh {
        let committed = std::fs::read_to_string(dir.join(name)).unwrap_or_default();
        assert!(committed == text, "app/ui/src/bindings/{name} is stale: run `make schemas`");
    }
}
