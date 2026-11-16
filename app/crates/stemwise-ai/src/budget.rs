//! Cost estimate before a call and the project's spending cap (AI-16).

use serde::{Deserialize, Serialize};
use stemwise_core::model::Project;

use crate::openrouter::AiConfig;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct CostEstimate {
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
    /// Spent so far on this project.
    pub spent_usd: f64,
    pub cap_usd: Option<f64>,
    /// The call would take the project over its cap: the UI asks for confirmation (design 12e).
    pub over_cap: bool,
}

/// Estimate of a call from the size of its first messages (characters / 4), the number of iterations and the
/// expected plan size (about 180 tokens per track or group, 400 for a master plan).
pub fn estimate(
    project: &Project,
    message_chars: usize,
    iterations: u32,
    master: bool,
    cfg: &AiConfig,
) -> CostEstimate {
    let units = project.tracks.iter().filter(|t| !t.is_return()).count() as u64;
    let out_per = if master { 400 } else { 180 * units.max(1) + 150 };
    let its = u64::from(iterations.max(1));
    // Later iterations carry the history of the last two plans and evaluations.
    let tokens_in = its * (message_chars as u64 / 4) + its.saturating_sub(1) * out_per;
    let tokens_out = its * out_per;
    let cost = if cfg.offline_demo { 0.0 } else { cfg.estimate(tokens_in, tokens_out) };
    let spent: f64 = project.ai.calls.iter().map(|c| c.cost_usd).sum();
    let cap = project.ai.spend_cap_usd;
    CostEstimate {
        tokens_in,
        tokens_out,
        cost_usd: cost,
        spent_usd: spent,
        cap_usd: cap,
        over_cap: cap.is_some_and(|c| spent + cost > c),
    }
}

/// What an auto mix and a revision cost on a typical song (12 tracks, 8 sections), at the configured prices: shown
/// while setting up the assistant, before there is a project to estimate (design 14d, AI-24).
pub fn typical_costs(cfg: &AiConfig) -> (f64, f64) {
    let mut song = Project::new("typical");
    for i in 0..12 {
        song.tracks.push(stemwise_core::model::Track::new_audio(format!("t{i}"), "t", None, 1, 1));
    }
    let priced = AiConfig { offline_demo: false, ..cfg.clone() };
    // The first messages of such a project: see `message_chars` in the app's session.
    let chars = 9000 + 1400 * 12 + 300 * 8 + 250 * 12 * 8;
    (estimate(&song, chars, 3, false, &priced).cost_usd, estimate(&song, chars, 1, false, &priced).cost_usd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use stemwise_core::model::{AiCallKind, AiCallRecord, AiOutcome, Track};

    /// req: AI-16
    #[test]
    fn estimate_and_spending_cap() {
        let mut p = Project::new("x");
        for i in 0..10 {
            p.tracks.push(Track::new_audio(format!("t{i}"), "t", None, 1, 1));
        }
        let cfg = AiConfig::default();
        let e = estimate(&p, 40_000, 3, false, &cfg);
        assert!(e.tokens_in > 30_000 && e.tokens_out > 5_000 && e.cost_usd > 0.0 && !e.over_cap);
        p.ai.spend_cap_usd = Some(0.05);
        p.ai.calls.push(AiCallRecord {
            id: "a".into(),
            kind: AiCallKind::AutoMix,
            started_ms: 0,
            duration_ms: 0,
            model: "m".into(),
            effort: "e".into(),
            decider: "openrouter".into(),
            attempts: 1,
            iterations: 1,
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.04,
            cost_estimated: false,
            outcome: AiOutcome::Applied,
            history_label: None,
            request: None,
            path: String::new(),
            error: None,
            stored: false,
        });
        let e = estimate(&p, 40_000, 3, false, &cfg);
        assert!(e.over_cap && (e.spent_usd - 0.04).abs() < 1e-12);
        let demo = AiConfig { offline_demo: true, ..AiConfig::default() };
        assert_eq!(estimate(&p, 40_000, 3, false, &demo).cost_usd, 0.0);
    }

    /// req: AI-24
    #[test]
    fn typical_costs_follow_the_price_list_also_on_the_offline_demo() {
        let cfg = AiConfig::default();
        let (mix, revision) = typical_costs(&cfg);
        assert!((0.05..0.5).contains(&mix) && revision > 0.0 && revision < mix / 2.0, "{mix} {revision}");
        let demo = AiConfig { offline_demo: true, ..cfg.clone() };
        assert_eq!(typical_costs(&demo), (mix, revision), "the price of the real thing, whatever is selected now");
        let dear = AiConfig {
            price_in_per_mtok: cfg.price_in_per_mtok * 2.0,
            price_out_per_mtok: cfg.price_out_per_mtok * 2.0,
            ..cfg
        };
        assert!((typical_costs(&dear).0 - 2.0 * mix).abs() < 1e-9);
    }
}
