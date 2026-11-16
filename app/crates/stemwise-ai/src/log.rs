//! Call log and cost register (AI-10, AI-14, AI-15, AI-16).
//!
//! Every call writes a folder `ai/<id>/` in the package with the requests and responses of each attempt, the plans
//! and evaluations of each iteration; its summary (`AiCallRecord`) lives in the project. The app keeps a register
//! of the costs of all projects in its data folder, for the total shown in the AI settings.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use stemwise_core::model::{AiCallKind, AiCallRecord, AiOutcome};

pub fn call_dir(package: &Path, id: &str) -> PathBuf {
    package.join("ai").join(id)
}

pub fn write(package: &Path, id: &str, name: &str, content: &str) {
    let dir = call_dir(package, id);
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(name), content);
}

/// Like [`write`], and says whether the file is there: for what a later step relies on.
pub fn write_checked(package: &Path, id: &str, name: &str, content: &str) -> bool {
    let dir = call_dir(package, id);
    std::fs::create_dir_all(&dir).is_ok() && std::fs::write(dir.join(name), content).is_ok()
}

pub fn new_id(kind: AiCallKind) -> String {
    let k = match kind {
        AiCallKind::AutoMix => "mix",
        AiCallKind::Revision => "rev",
        AiCallKind::Master => "master",
        AiCallKind::MasterRevision => "mrev",
    };
    // Milliseconds plus a process-wide counter: two calls in the same millisecond never share a folder.
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{}-{n}-{k}", stemwise_core::package::now_ms())
}

/// Running totals of one call.
#[derive(Debug, Clone, Default)]
pub struct Meter {
    pub attempts: u32,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
    pub estimated: bool,
}

impl Meter {
    /// Adds an attempt: the provider's cost when present, otherwise an estimate (AI-15).
    pub fn add(&mut self, usage: &crate::decider::Usage, estimate: impl Fn(u64, u64) -> f64) {
        self.attempts += 1;
        self.tokens_in += usage.tokens_in;
        self.tokens_out += usage.tokens_out;
        match usage.cost_usd {
            Some(c) => self.cost_usd += c,
            None => {
                self.cost_usd += estimate(usage.tokens_in, usage.tokens_out);
                self.estimated = true;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn record(
    id: &str,
    kind: AiCallKind,
    started_ms: u64,
    model: &str,
    effort: &str,
    decider: &str,
    meter: &Meter,
    iterations: u32,
    outcome: AiOutcome,
    request: Option<String>,
    error: Option<String>,
) -> AiCallRecord {
    AiCallRecord {
        id: id.to_string(),
        kind,
        started_ms,
        duration_ms: stemwise_core::package::now_ms().saturating_sub(started_ms),
        model: model.to_string(),
        effort: effort.to_string(),
        decider: decider.to_string(),
        attempts: meter.attempts,
        iterations,
        tokens_in: meter.tokens_in,
        tokens_out: meter.tokens_out,
        cost_usd: meter.cost_usd,
        cost_estimated: meter.estimated,
        outcome,
        history_label: None,
        request,
        path: format!("ai/{id}"),
        error,
        stored: false,
    }
}

/// One line of the cross-project cost register.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct CostEntry {
    pub project: String,
    pub call: String,
    pub at_ms: u64,
    pub model: String,
    pub cost_usd: f64,
    pub estimated: bool,
}

pub fn register_cost(app_data: &Path, e: &CostEntry) {
    let path = app_data.join("ai-costs.jsonl");
    let _ = std::fs::create_dir_all(app_data);
    if let Ok(line) = serde_json::to_string(e) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{line}");
        }
    }
}

/// Total cost over all projects (AI-14, last item): cost, calls and distinct projects.
pub fn total_cost(app_data: &Path) -> (f64, usize, usize) {
    let text = std::fs::read_to_string(app_data.join("ai-costs.jsonl")).unwrap_or_default();
    let entries: Vec<CostEntry> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    let projects: std::collections::BTreeSet<&str> = entries.iter().map(|e| e.project.as_str()).collect();
    (entries.iter().map(|e| e.cost_usd).sum(), entries.len(), projects.len())
}

/// CSV export of a project's calls (AI-14).
pub fn calls_csv(calls: &[AiCallRecord]) -> String {
    let mut s = String::from(
        "id,kind,started_ms,model,effort,decider,attempts,iterations,tokens_in,tokens_out,cost_usd,cost_estimated,duration_ms,outcome,history_step,request,error\n",
    );
    let q = |x: &str| format!("\"{}\"", x.replace('"', "\"\""));
    for c in calls {
        s.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{}\n",
            q(&c.id),
            serde_json::to_string(&c.kind).unwrap_or_default().trim_matches('"'),
            c.started_ms,
            q(&c.model),
            q(&c.effort),
            q(&c.decider),
            c.attempts,
            c.iterations,
            c.tokens_in,
            c.tokens_out,
            c.cost_usd,
            c.cost_estimated,
            c.duration_ms,
            serde_json::to_string(&c.outcome).unwrap_or_default().trim_matches('"'),
            q(c.history_label.as_deref().unwrap_or("")),
            q(c.request.as_deref().unwrap_or("")),
            q(c.error.as_deref().unwrap_or("")),
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decider::Usage;

    /// req: AI-15, AI-14
    #[test]
    fn costs_are_reported_or_estimated_and_registered_across_projects() {
        let mut m = Meter::default();
        m.add(&Usage { tokens_in: 1000, tokens_out: 100, cost_usd: Some(0.01) }, |_, _| 99.0);
        assert!(!m.estimated && (m.cost_usd - 0.01).abs() < 1e-12);
        m.add(&Usage { tokens_in: 1_000_000, tokens_out: 0, cost_usd: None }, |i, o| (i + o) as f64 * 1e-6);
        assert!(m.estimated && (m.cost_usd - 1.01).abs() < 1e-9 && m.attempts == 2);
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
        std::fs::create_dir_all(&root).unwrap();
        let t = tempfile::tempdir_in(root).unwrap();
        for (p, c) in [("A", 0.5), ("B", 0.25)] {
            register_cost(
                t.path(),
                &CostEntry {
                    project: p.into(),
                    call: "x".into(),
                    at_ms: 1,
                    model: "m".into(),
                    cost_usd: c,
                    estimated: false,
                },
            );
        }
        assert_eq!(total_cost(t.path()), (0.75, 2, 2));
        let r = record(
            "1-mix",
            AiCallKind::AutoMix,
            0,
            "m",
            "medium",
            "openrouter",
            &m,
            2,
            AiOutcome::Failed,
            Some("say \"hi\"".into()),
            None,
        );
        let csv = calls_csv(&[r]);
        assert!(csv.lines().nth(1).unwrap().contains("auto_mix") && csv.contains("\"say \"\"hi\"\"\""));
    }
}
