//! OpenRouter decider (AI-8, AI-9, AI-11, AI-15; P-005): chat completions with the plan's JSON Schema as strict
//! structured output, falling back to JSON mode when the provider refuses the schema (Q-40 of the PoC, option b).
//! The key comes from the system keychain at call time and is never logged. No model is hard-wired: the model,
//! effort and temperature come from the configuration (default `openai/gpt-6.1-sol`, medium effort; D-047).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

use crate::decider::{DecideError, Decider, DecisionInput, Exchange, Kind, Message, RawDecision, Retry, Usage};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct AiConfig {
    pub base_url: String,
    pub model: String,
    /// "low", "medium" or "high".
    pub effort: String,
    pub temperature: f64,
    /// Use the fake decider ("Offline demo", AI-9).
    pub offline_demo: bool,
    /// Prices in USD per million tokens (input, output) for cost estimates when the response has none (AI-15).
    pub price_in_per_mtok: f64,
    pub price_out_per_mtok: f64,
    /// Cap on the answer's tokens, reasoning included, sent as `max_tokens`: without it the cap is whatever the
    /// provider picks for the request (A-062). 0 sends none.
    #[serde(default = "default_max_output_tokens")]
    pub max_output_tokens: u32,
}

/// Room for the plan of a large project (a 30-track plan is about 9,000 tokens) plus the model's reasoning.
fn default_max_output_tokens() -> u32 {
    32_000
}

impl Default for AiConfig {
    fn default() -> Self {
        AiConfig {
            base_url: "https://openrouter.ai/api/v1".into(),
            model: "openai/gpt-6.1-sol".into(),
            effort: "medium".into(),
            temperature: 0.2,
            offline_demo: false,
            price_in_per_mtok: 1.25,
            price_out_per_mtok: 10.0,
            max_output_tokens: default_max_output_tokens(),
        }
    }
}

impl AiConfig {
    /// Cost estimate from tokens and the price list (AI-15, AI-16).
    pub fn estimate(&self, tokens_in: u64, tokens_out: u64) -> f64 {
        (tokens_in as f64 * self.price_in_per_mtok + tokens_out as f64 * self.price_out_per_mtok) / 1e6
    }
}

pub struct OpenRouter {
    pub cfg: AiConfig,
    key: String,
    timeout: Duration,
    /// Set once the provider refused the strict schema: the rest of the session uses JSON mode directly, so a
    /// refusal costs one request, not one per attempt.
    strict_refused: AtomicBool,
    /// Set once the provider refused the output cap: the rest of the session sends none.
    cap_refused: AtomicBool,
}

impl OpenRouter {
    pub fn new(cfg: AiConfig, key: String) -> Self {
        OpenRouter {
            cfg,
            key,
            timeout: Duration::from_secs(300),
            strict_refused: AtomicBool::new(false),
            cap_refused: AtomicBool::new(false),
        }
    }

    fn body(
        &self,
        kind: Kind,
        messages: &[Message],
        schema: &Value,
        strict: bool,
        cap: Option<u32>,
        retry: Option<&Retry>,
    ) -> Value {
        let mut msgs: Vec<Value> = messages.iter().map(|m| json!({"role": m.role, "content": m.content})).collect();
        if let Some(r) = retry {
            // As the PoC (llm.py): the invalid answer, then what is wrong with it.
            msgs.push(json!({"role": "assistant", "content": r.previous}));
            msgs.push(json!({"role": "user", "content": r.errors}));
        }
        let name = match kind {
            Kind::Mix => "mix_plan",
            Kind::Master => "master_plan",
        };
        let format = if strict {
            json!({"type": "json_schema", "json_schema": {"name": name, "strict": true, "schema": schema}})
        } else {
            json!({"type": "json_object"})
        };
        let mut body = json!({
            "model": self.cfg.model,
            "messages": msgs,
            "temperature": self.cfg.temperature,
            "reasoning": {"effort": self.cfg.effort},
            "response_format": format,
            "usage": {"include": true},
        });
        if let Some(cap) = cap {
            body["max_tokens"] = json!(cap);
        }
        body
    }

    /// One HTTP exchange, run on a helper thread so a cancel returns at once (AI-12).
    fn post(&self, body: &Value, cancel: &AtomicBool) -> Result<(u16, String), DecideError> {
        let (tx, rx) = mpsc::channel();
        let url = format!("{}/chat/completions", self.cfg.base_url.trim_end_matches('/'));
        let key = self.key.clone();
        let text = body.to_string();
        let timeout = self.timeout;
        std::thread::spawn(move || {
            let r = reqwest::blocking::Client::builder()
                .timeout(timeout)
                .build()
                .and_then(|c| {
                    c.post(url)
                        .bearer_auth(key)
                        .header("Content-Type", "application/json")
                        .header("X-Title", "Stemwise")
                        .body(text)
                        .send()
                })
                .and_then(|resp| {
                    let status = resp.status().as_u16();
                    resp.text().map(|t| (status, t))
                });
            let _ = tx.send(r);
        });
        loop {
            if cancel.load(Ordering::Relaxed) {
                // The request is out: the provider may still answer and bill it.
                return Err(DecideError::Abandoned);
            }
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(r)) => return Ok(r),
                Ok(Err(e)) => return Err(DecideError::Network(e.to_string())),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(DecideError::Network("connection lost".into()));
                }
            }
        }
    }
}

/// How a refusal for lack of credit starts, so the interface can tell the cases apart (design 14i, 14j).
pub const NO_CREDIT: &str = "the OpenRouter account has no credit left";
pub const KEY_LIMIT: &str = "the key has reached its own credit limit";

fn usage_of(v: &Value) -> Usage {
    let u = &v["usage"];
    Usage {
        tokens_in: u["prompt_tokens"].as_u64().unwrap_or(0),
        tokens_out: u["completion_tokens"].as_u64().unwrap_or(0),
        cost_usd: u["cost"].as_f64(),
    }
}

/// The JSON inside an answer: models sometimes wrap it in a Markdown fence (as the PoC's `validate.py`).
pub fn strip_fences(content: &str) -> String {
    let t = content.trim();
    if let Some(rest) = t.strip_prefix("```") {
        let body = rest.split_once('\n').map_or("", |(_, b)| b);
        return body.trim_end().trim_end_matches("```").trim().to_string();
    }
    t.to_string()
}

fn schema_refused(status: u16, text: &str) -> bool {
    let t = text.to_lowercase();
    (status == 400 || status == 422)
        && (t.contains("json_schema")
            || t.contains("response_format")
            || t.contains("structured")
            || t.contains("schema"))
}

fn cap_refused(status: u16, text: &str) -> bool {
    let t = text.to_lowercase();
    status == 400
        && (t.contains("max_tokens") || t.contains("max_completion_tokens") || t.contains("max_output_tokens"))
}

impl Decider for OpenRouter {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn model(&self) -> String {
        self.cfg.model.clone()
    }

    fn effort(&self) -> String {
        self.cfg.effort.clone()
    }

    fn decide(
        &self,
        input: &DecisionInput,
        messages: &[Message],
        schema: &Value,
        retry: Option<&Retry>,
        cancel: &AtomicBool,
    ) -> Result<RawDecision, DecideError> {
        if self.key.trim().is_empty() {
            return Err(DecideError::NoKey);
        }
        let mut strict = !self.strict_refused.load(Ordering::Relaxed);
        let mut cap = (self.cfg.max_output_tokens > 0 && !self.cap_refused.load(Ordering::Relaxed))
            .then_some(self.cfg.max_output_tokens);
        // Everything exchanged in this call, for the log of a failure, and what it spent across its requests.
        let mut exchanges: Vec<Exchange> = Vec::new();
        let mut spent = Usage::default();
        let mut cut_once = false;
        loop {
            let body = self.body(input.kind, messages, schema, strict, cap, retry);
            let (status, text) = self.post(&body, cancel)?;
            exchanges.push(Exchange { request: body.to_string(), status, response: text.clone() });
            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            spent.add(&usage_of(&v));
            if status == 200 {
                // Some providers report errors inside a 200 body.
                if let Some(m) = v["error"]["message"].as_str() {
                    return Err(DecideError::Provider { status, message: m.to_string(), usage: spent, exchanges });
                }
                let choice = &v["choices"][0];
                if choice["finish_reason"].as_str() == Some("length") {
                    if strict && !cut_once {
                        // A model can stall under a strict schema until it is cut: one more try in JSON mode, for
                        // this call only (A-062). Our own validation still checks the plan.
                        cut_once = true;
                        strict = false;
                        continue;
                    }
                    return Err(DecideError::Truncated { usage: spent, cap, exchanges });
                }
                let content = strip_fences(choice["message"]["content"].as_str().unwrap_or_default());
                return Ok(RawDecision {
                    content,
                    usage: spent,
                    model: v["model"].as_str().unwrap_or(&self.cfg.model).to_string(),
                    raw: text,
                    request: body.to_string(),
                });
            }
            if cap.is_some() && cap_refused(status, &text) {
                // The provider does not take this cap: its own default, for the rest of the session.
                cap = None;
                self.cap_refused.store(true, Ordering::Relaxed);
                continue;
            }
            if strict && schema_refused(status, &text) {
                // The provider does not take the strict schema: JSON mode plus our validation (AI-8).
                strict = false;
                self.strict_refused.store(true, Ordering::Relaxed);
                continue;
            }
            let message =
                v["error"]["message"].as_str().map_or_else(|| text.chars().take(300).collect(), str::to_string);
            if status == 402 {
                // No credit (design 14i) or the key's own limit (14j): nothing was run, nothing was spent.
                let key_limit = v["error"]["metadata"]["limit_source"].as_str() == Some("openrouter_key_limit");
                let what = if key_limit { KEY_LIMIT } else { NO_CREDIT };
                return Err(DecideError::Provider {
                    status,
                    message: format!("{what}: {message}"),
                    usage: Usage::default(),
                    exchanges,
                });
            }
            if status == 401 || status == 403 {
                return Err(DecideError::Provider {
                    status,
                    message: format!("the API key was refused: {message}"),
                    usage: Usage::default(),
                    exchanges,
                });
            }
            return Err(DecideError::Provider { status, message, usage: spent, exchanges });
        }
    }
}

#[cfg(test)]
pub mod fake_server {
    //! A local HTTP server answering chat completions from a script of canned responses (FOUNDATION §8.4: no test
    //! uses the network). It records every request body.

    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    pub struct FakeServer {
        pub url: String,
        pub requests: Arc<Mutex<Vec<serde_json::Value>>>,
    }

    /// Starts a server that answers the n-th request with `responses[n]` (status, body) and then closes.
    pub fn start(responses: Vec<(u16, String)>) -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let url = format!("http://{}", listener.local_addr().expect("addr"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let rec = requests.clone();
        std::thread::spawn(move || {
            for (status, body) in responses {
                let Ok((mut s, _)) = listener.accept() else { return };
                let mut reader = BufReader::new(s.try_clone().expect("clone"));
                let mut len = 0usize;
                let mut auth = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    let l = line.to_lowercase();
                    if let Some(v) = l.strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    if l.starts_with("authorization:") {
                        auth = line.trim().to_string();
                    }
                }
                let mut buf = vec![0u8; len];
                let _ = reader.read_exact(&mut buf);
                let mut v: serde_json::Value = serde_json::from_slice(&buf).unwrap_or_default();
                v["_auth"] = serde_json::Value::String(auth);
                rec.lock().expect("lock").push(v);
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = s.write_all(reply.as_bytes());
            }
        });
        FakeServer { url, requests }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis_doc::Ids;
    use stemwise_core::model::{Brief, Project};

    fn input() -> DecisionInput {
        let p = Project::new("x");
        DecisionInput {
            kind: Kind::Mix,
            iteration: 0,
            brief: Brief::default(),
            analysis: json!({}),
            ids: Ids::of(&p),
            project: p,
            history: vec![],
            request: None,
            start: None,
            editable: vec![],
            allow_structure: false,
            allow_ideas: false,
            allow_return_blocks: false,
        }
    }

    fn ok_body(content: &str, cost: Option<f64>) -> String {
        let mut usage = json!({"prompt_tokens": 1200, "completion_tokens": 340});
        if let Some(c) = cost {
            usage["cost"] = json!(c);
        }
        json!({"model": "openai/gpt-6.1-sol", "choices": [{"message": {"content": content}}], "usage": usage})
            .to_string()
    }

    /// req: AI-9, AI-11, AI-15
    #[test]
    fn calls_the_configured_model_with_the_strict_schema_and_reads_usage() {
        let server = fake_server::start(vec![(200, ok_body("{\"a\":1}", Some(0.0123)))]);
        let cfg = AiConfig { base_url: server.url.clone(), model: "vendor/some-model".into(), ..Default::default() };
        let d = OpenRouter::new(cfg, "sk-test".into());
        let msgs = vec![Message { role: "system".into(), content: "s".into() }];
        let r = d.decide(&input(), &msgs, &json!({"type": "object"}), None, &AtomicBool::new(false)).unwrap();
        assert_eq!(r.content, "{\"a\":1}");
        assert_eq!(r.usage, Usage { tokens_in: 1200, tokens_out: 340, cost_usd: Some(0.0123) });
        let req = &server.requests.lock().unwrap()[0];
        assert_eq!(req["model"], "vendor/some-model");
        assert_eq!(req["response_format"]["type"], "json_schema");
        assert_eq!(req["response_format"]["json_schema"]["strict"], true);
        assert_eq!(req["reasoning"]["effort"], "medium");
        assert_eq!(req["max_tokens"], 32_000, "the output cap is ours, not the provider's default");
        assert_eq!(req["_auth"].as_str().unwrap().to_lowercase(), "authorization: bearer sk-test");
        // The key travels only in the header, never in the logged request body.
        assert!(!r.request.contains("sk-test"));
    }

    /// req: AI-25, AI-26, AI-9
    #[test]
    fn the_v4_schema_with_blocks_and_ideas_travels_strict_and_its_answer_parses() {
        use crate::plan::{Editable, MixPlan, mix_plan_schema_for};
        use stemwise_core::model::Track;
        let mut p = Project::new("s");
        p.tracks.push(Track::new_audio("a".into(), "Keys", None, 2, 1));
        let ids = Ids::of(&p);
        let ed = Editable { tracks: vec![ids.n("a")], groups: vec![], returns: vec![] };
        let schema = mix_plan_schema_for(&p, &ids, &ed, false, true, false);
        let plan = json!({
            "version": 4, "iteration": 0, "summary": "s", "structure": null, "returns": {}, "groups": {}, "tracks": {},
            // The strict schema lists every idea id: the unused ones come back as `null`.
            "ideas": {
                "i1": {"name": "Wide chorus on the keys", "sections": [], "rationale": "Open in the choruses."},
                "i2": null, "i3": null
            }
        });
        let server = fake_server::start(vec![(200, ok_body(&plan.to_string(), Some(0.01)))]);
        let cfg = AiConfig { base_url: server.url.clone(), ..Default::default() };
        let d = OpenRouter::new(cfg, "sk-test".into());
        let msgs = vec![Message { role: "system".into(), content: "s".into() }];
        let r = d.decide(&input(), &msgs, &schema, None, &AtomicBool::new(false)).unwrap();
        let req = &server.requests.lock().unwrap()[0];
        assert_eq!(req["response_format"]["json_schema"]["strict"], true);
        let sent = &req["response_format"]["json_schema"]["schema"];
        assert_eq!(sent, &schema, "the schema reaches the provider as built");
        assert!(sent["properties"]["ideas"].is_object());
        // One definition per block type, referenced from the track's `blocks` (AI-25).
        assert!(sent["$defs"].as_object().is_some_and(|d| d.keys().any(|k| k.starts_with("block_"))), "{sent}");
        let parsed: MixPlan = serde_json::from_str(&r.content).unwrap();
        assert_eq!(parsed.version, 4);
        let ideas = parsed.ideas.unwrap();
        assert_eq!(ideas.len(), 1);
        assert_eq!(ideas["i1"].name, "Wide chorus on the keys");
    }

    /// req: AI-8
    #[test]
    fn falls_back_to_json_mode_when_the_schema_is_refused_and_sends_validation_feedback() {
        let refusal =
            json!({"error": {"message": "response_format json_schema is not supported by this provider"}}).to_string();
        let server = fake_server::start(vec![(400, refusal), (200, ok_body("{}", None))]);
        let d = OpenRouter::new(AiConfig { base_url: server.url.clone(), ..Default::default() }, "k".into());
        let retry = Retry { previous: "{\"bad\": 1}".into(), errors: "The plan is invalid: x".into() };
        let r = d.decide(&input(), &[], &json!({}), Some(&retry), &AtomicBool::new(false)).unwrap();
        assert_eq!(r.usage.cost_usd, None, "no cost in the response: it will be estimated (AI-15)");
        {
            let reqs = server.requests.lock().unwrap();
            assert_eq!(reqs[1]["response_format"]["type"], "json_object");
            let msgs = reqs[1]["messages"].as_array().unwrap();
            assert_eq!(msgs[msgs.len() - 2]["role"], "assistant");
            assert_eq!(msgs[msgs.len() - 2]["content"], "{\"bad\": 1}");
            assert_eq!(msgs[msgs.len() - 1]["content"], "The plan is invalid: x");
        }
        // The refusal is remembered: the next attempt goes straight to JSON mode.
        let server2 = fake_server::start(vec![(200, ok_body("```json\n{\"a\": 2}\n```", None))]);
        let d2 = OpenRouter { cfg: AiConfig { base_url: server2.url.clone(), ..Default::default() }, ..d };
        let r = d2.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap();
        assert_eq!(server2.requests.lock().unwrap()[0]["response_format"]["type"], "json_object");
        assert_eq!(r.content, "{\"a\": 2}", "Markdown fences are stripped");
    }

    /// req: AI-11, UI-6
    #[test]
    fn missing_key_network_errors_and_refused_keys_are_reported_clearly() {
        let d = OpenRouter::new(AiConfig::default(), String::new());
        assert!(matches!(d.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)), Err(DecideError::NoKey)));
        // Nothing listens on this port: a network error, not a hang.
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        let d = OpenRouter::new(AiConfig { base_url: format!("http://{closed}"), ..Default::default() }, "k".into());
        assert!(matches!(
            d.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)),
            Err(DecideError::Network(_))
        ));
        let server =
            fake_server::start(vec![(401, json!({"error": {"message": "No auth credentials found"}}).to_string())]);
        let d = OpenRouter::new(AiConfig { base_url: server.url.clone(), ..Default::default() }, "bad".into());
        let e = d.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap_err();
        assert!(e.to_string().contains("refused"), "{e}");
        assert!((AiConfig::default().estimate(1_000_000, 100_000) - 2.25).abs() < 1e-9);
        // An error inside a 200 body is an error, logged with what was sent and received.
        let inner = json!({"error": {"message": "upstream overloaded"}}).to_string();
        let server = fake_server::start(vec![(200, inner)]);
        let d = OpenRouter::new(AiConfig { base_url: server.url.clone(), ..Default::default() }, "k".into());
        let e = d.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap_err();
        assert!(e.to_string().contains("overloaded") && !e.to_string().contains("refused"), "{e}");
        let entry: Value = serde_json::from_str(&e.log_entry(&[])).unwrap();
        assert_eq!(entry["exchanges"][0]["request"]["model"], "openai/gpt-6.1-sol");
        assert_eq!(entry["exchanges"][0]["response"]["error"]["message"], "upstream overloaded");
        assert!(e.last_response().unwrap().contains("overloaded"));
    }

    /// req: AI-24, UI-6
    #[test]
    fn a_refusal_for_lack_of_credit_says_whether_it_is_the_account_or_the_key() {
        let account = json!({"error": {"code": 402, "message": "Insufficient credits", "metadata": {"limit_source": "openrouter_credits"}}});
        let key = json!({"error": {"code": 402, "message": "Key limit exceeded", "metadata": {"limit_source": "openrouter_key_limit"}}});
        let bare = json!({"error": {"code": 402, "message": "Payment required"}});
        for (body, starts) in [(account, NO_CREDIT), (key, KEY_LIMIT), (bare, NO_CREDIT)] {
            let server = fake_server::start(vec![(402, body.to_string())]);
            let d = OpenRouter::new(AiConfig { base_url: server.url.clone(), ..Default::default() }, "k".into());
            let e = d.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap_err();
            let DecideError::Provider { status, message, usage, .. } = &e else { panic!("{e}") };
            assert!(*status == 402 && message.starts_with(starts), "{message}");
            assert_eq!(*usage, Usage::default(), "a refused request spent nothing");
            assert_eq!(server.requests.lock().unwrap().len(), 1, "not retried: the answer would be the same");
        }
    }

    fn cut_body(tokens_out: u64) -> String {
        json!({"choices": [{"message": {"content": "{\"ver"}, "finish_reason": "length"}], "usage": {"prompt_tokens": 9, "completion_tokens": tokens_out}}).to_string()
    }

    /// req: AI-8, AI-10, AI-15
    #[test]
    fn an_answer_cut_off_is_retried_once_in_json_mode_and_logged_whole_when_it_fails_again() {
        // Cut under the strict schema, complete in JSON mode: the call succeeds and counts both requests.
        let server = fake_server::start(vec![(200, cut_body(4000)), (200, ok_body("{\"a\":1}", None))]);
        let d = OpenRouter::new(AiConfig { base_url: server.url.clone(), ..Default::default() }, "k".into());
        let r = d.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap();
        assert_eq!(r.content, "{\"a\":1}");
        assert_eq!((r.usage.tokens_in, r.usage.tokens_out), (1209, 4340), "the cut answer was paid for too");
        {
            let reqs = server.requests.lock().unwrap();
            assert_eq!(reqs[0]["response_format"]["type"], "json_schema");
            assert_eq!(reqs[1]["response_format"]["type"], "json_object");
        }
        // The fallback is for this call only: the next one tries the strict schema again.
        let server2 = fake_server::start(vec![(200, ok_body("{}", None))]);
        let d2 = OpenRouter { cfg: AiConfig { base_url: server2.url.clone(), ..Default::default() }, ..d };
        d2.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap();
        assert_eq!(server2.requests.lock().unwrap()[0]["response_format"]["type"], "json_schema");

        // Cut twice: an error that says what happened, with both exchanges for the log.
        let server = fake_server::start(vec![(200, cut_body(4000)), (200, cut_body(0))]);
        let d = OpenRouter::new(AiConfig { base_url: server.url.clone(), ..Default::default() }, "k".into());
        let e = d.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap_err();
        let text = e.to_string();
        assert!(text.contains("cut off") && text.contains("32000") && !text.contains("refused"), "{text}");
        assert_eq!(e.usage().unwrap().tokens_out, 4000);
        let entry: Value = serde_json::from_str(&e.log_entry(&[])).unwrap();
        let ex = entry["exchanges"].as_array().unwrap();
        assert_eq!(ex.len(), 2);
        assert_eq!(ex[0]["request"]["max_tokens"], 32_000);
        assert_eq!(ex[1]["response"]["choices"][0]["finish_reason"], "length");
        assert!(!e.log_entry(&[]).contains("Bearer"), "the key is never in the log");
    }

    /// req: AI-8
    #[test]
    fn a_refused_output_cap_is_dropped_for_the_session_and_zero_sends_none() {
        let refusal = json!({"error": {"message": "max_tokens is too large for this model"}}).to_string();
        let server = fake_server::start(vec![(400, refusal), (200, ok_body("{}", None)), (200, ok_body("{}", None))]);
        let d = OpenRouter::new(AiConfig { base_url: server.url.clone(), ..Default::default() }, "k".into());
        d.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap();
        d.decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap();
        {
            let reqs = server.requests.lock().unwrap();
            assert_eq!(reqs[0]["max_tokens"], 32_000);
            assert!(reqs[1].get("max_tokens").is_none() && reqs[2].get("max_tokens").is_none());
            assert_eq!(reqs[1]["response_format"]["type"], "json_schema", "the schema was not the problem");
        }
        let server = fake_server::start(vec![(200, ok_body("{}", None))]);
        let cfg = AiConfig { base_url: server.url.clone(), max_output_tokens: 0, ..Default::default() };
        OpenRouter::new(cfg, "k".into()).decide(&input(), &[], &json!({}), None, &AtomicBool::new(false)).unwrap();
        assert!(server.requests.lock().unwrap()[0].get("max_tokens").is_none());
        // A configuration saved before the cap existed keeps its values and gets the default cap.
        let old: AiConfig = serde_json::from_str(
            r#"{"base_url":"u","model":"m","effort":"low","temperature":0.1,"offline_demo":false,"price_in_per_mtok":1.0,"price_out_per_mtok":2.0}"#,
        )
        .unwrap();
        assert_eq!((old.model.as_str(), old.max_output_tokens), ("m", 32_000));
    }

    /// req: AI-12
    #[test]
    fn a_cancel_returns_at_once_even_if_the_server_never_answers() {
        // A server that accepts and never replies.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let _keep = listener.accept();
            std::thread::sleep(Duration::from_secs(30));
        });
        let d = OpenRouter::new(AiConfig { base_url: url, ..Default::default() }, "k".into());
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let c2 = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            c2.store(true, Ordering::Relaxed);
        });
        let t = std::time::Instant::now();
        let e = d.decide(&input(), &[], &json!({}), None, &cancel).unwrap_err();
        assert!(matches!(e, DecideError::Abandoned) && e.maybe_billed(), "an abandoned request may be billed");
        assert!(t.elapsed() < Duration::from_secs(2));
    }
}
