//! Source policies of FOUNDATION §3.12, checked on the code itself:
//!
//! - NFR-7: the only network code is the OpenRouter client (`stemwise-ai/src/openrouter.rs`), only `stemwise-ai`
//!   depends on an HTTP library, and the client is created only where the user asked for the assistant (the
//!   `ai_start` and `master_ai_start` paths of the app, the CLI's explicit commands). No telemetry anywhere.
//! - NFR-8: what is specific to one system (conditional compilation on the OS, linked frameworks, foreign functions)
//!   lives only in isolated modules behind an interface, the same on every system: the AAC encoder
//!   (`stemwise-export/src/aac.rs`, behind `aac_available` and `encode_aac`, reporting it unavailable where the
//!   system has none), the question about the signature (`stemwise-ai/src/signing.rs`) and the shell's menu bar
//!   (`src-tauri/src/platform.rs`).

use std::path::{Path, PathBuf};

fn app() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// Rust sources of the product (crates and the Tauri shell), with their path relative to `app/`.
fn sources() -> Vec<(String, String)> {
    let root = app();
    let mut out = Vec::new();
    let mut stack = vec![root.join("crates"), root.join("src-tauri/src")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "target") {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "rs" || x == "toml") {
                // `/` on every system, so that the paths below compare the same on Windows.
                let rel = p.strip_prefix(&root).unwrap().display().to_string().replace('\\', "/");
                if rel.ends_with("stemwise-cli/tests/policy.rs") {
                    continue; // this file names what it looks for
                }
                out.push((rel, std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    assert!(out.len() > 50, "the scan found the sources");
    out
}

/// Code lines only: comments dropped, so that documentation may name things.
fn code(text: &str) -> String {
    text.lines().map(|l| l.split("//").next().unwrap_or("")).collect::<Vec<_>>().join("\n")
}

/// req: NFR-7
#[test]
fn network_code_is_only_the_openrouter_client_called_on_request() {
    let network = ["reqwest", "TcpStream", "TcpListener", "UdpSocket", "std::net", "hyper::", "ureq", "tungstenite"];
    // The client, and the connection of an account from the browser (AI-24): a loopback listener for the provider's
    // callback and the exchange of its code for a key.
    let client = ["crates/stemwise-ai/src/openrouter.rs", "crates/stemwise-ai/src/connect.rs"];
    for (path, text) in sources() {
        if client.contains(&path.as_str()) {
            continue;
        }
        if path.ends_with("Cargo.toml") {
            let deps = text.split("[dev-dependencies]").next().unwrap_or("");
            let http = ["reqwest", "hyper", "ureq", "isahc", "surf", "attohttpc"]
                .iter()
                .any(|d| deps.contains(&format!("\n{d} ")));
            assert!(!http || path == "crates/stemwise-ai/Cargo.toml", "{path} depends on an HTTP client");
            continue;
        }
        let c = code(&text);
        for n in network {
            assert!(!c.contains(n), "{path} uses `{n}`: network code belongs to the OpenRouter client only");
        }
        for t in ["telemetry", "analytics", "sentry", "crash_report"] {
            assert!(!c.to_lowercase().contains(t), "{path} mentions `{t}`");
        }
        // The client is built only where the user asked for the assistant.
        if c.contains("OpenRouter::new") {
            let allowed =
                ["src-tauri/src/session/ai.rs", "src-tauri/src/session/master.rs", "crates/stemwise-cli/src/main.rs"];
            assert!(allowed.contains(&path.as_str()), "{path} creates the OpenRouter client");
        }
        // The connection starts only from the session, behind the command of the "Connect" button.
        if c.contains("Connect::begin") {
            assert_eq!(path, "src-tauri/src/session/ai.rs", "{path} starts a connection to the provider");
        }
        // The browser opens only from the shell, on pages of a fixed list or the connection's own.
        if c.contains("open_url") {
            assert_eq!(path, "src-tauri/src/main.rs", "{path} opens the browser");
        }
    }
    // The listener answers on the loopback interface only, and the pages the app can open are the provider's.
    let connect = code(&std::fs::read_to_string(app().join(client[1])).unwrap());
    let binds: Vec<&str> =
        connect.split("TcpListener::bind(").skip(1).map(|rest| &rest[..rest.len().min(16)]).collect();
    assert!(!binds.is_empty());
    for bind in binds {
        assert!(bind.contains("127.0.0.1") || bind.contains("::1"), "the callback listener is local: {bind}");
    }
    // The whole text here: `code` would cut an address at its slashes.
    let main = std::fs::read_to_string(app().join("src-tauri/src/main.rs")).unwrap();
    let pages: Vec<&str> = main.split("\"https://").skip(1).collect();
    assert!(pages.len() >= 3);
    for page in pages {
        assert!(page.starts_with("openrouter.ai"), "the shell names a page that is not the provider's");
    }
    // In the app, the client and the connection sit behind the commands the user triggers.
    assert!(
        main.contains("fn ai_start") && main.contains("fn master_ai_start") && main.contains("fn ai_connect_start")
    );
}

/// req: NFR-8
#[test]
fn platform_specific_code_is_isolated_behind_an_interface() {
    let isolated =
        ["crates/stemwise-export/src/aac.rs", "crates/stemwise-ai/src/signing.rs", "src-tauri/src/platform.rs"];
    let markers = ["target_os", "#[link(", "extern \"C\"", "objc", "core_foundation", "cocoa::", "AudioToolbox"];
    for (path, text) in sources() {
        if path.ends_with(".toml")
            || isolated.contains(&path.as_str())
            || path.contains("/tests/")
            || path.contains("/benches/")
        {
            continue;
        }
        let c = code(&text);
        for m in markers {
            assert!(!c.contains(m), "{path} has platform-specific code (`{m}`) outside the isolated modules");
        }
    }
    // Each isolated module offers the same interface everywhere, with a fallback where the system has no equivalent.
    let aac = std::fs::read_to_string(app().join(isolated[0])).unwrap();
    assert!(aac.contains("#[cfg(not(any(target_os = \"macos\", windows)))]") && aac.contains("pub fn aac_available"));
    let signing = std::fs::read_to_string(app().join(isolated[1])).unwrap();
    assert!(signing.contains("#[cfg(not(target_os = \"macos\"))]") && signing.contains("pub fn keychain_is_silent"));
    let platform = std::fs::read_to_string(app().join(isolated[2])).unwrap();
    assert!(platform.contains("#[cfg(not(target_os = \"macos\"))]") && platform.contains("pub fn with_menu_bar"));
}
