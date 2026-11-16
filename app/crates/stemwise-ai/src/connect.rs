//! Connecting an OpenRouter account from the browser (AI-24, P-015): the provider's OAuth flow with PKCE, so the
//! user never copies a key.
//!
//! 1. The app listens on the loopback interface, on a port the system picks, and opens
//!    `https://openrouter.ai/auth?callback_url=http://localhost:<port>/callback&code_challenge=…` in the browser.
//! 2. The user signs in (or signs up) there and approves. The browser comes back to the callback with a `code`.
//! 3. The app exchanges the code and its PKCE verifier for a key (`POST /api/v1/auth/keys`) and answers the browser
//!    with a small page saying how it went.
//!
//! The listener exists only while the user is connecting: it answers one callback, checks the `state` it issued,
//! and closes after `VALID_FOR` (the provider's codes last ten minutes) or on cancel. It is bound to the loopback
//! addresses only, so nothing outside this computer can reach it. With `openrouter.rs` this is the only network
//! code of the product (NFR-7); nothing here runs unless the user pressed "Connect".

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

/// The name the key gets in the user's OpenRouter account.
pub const KEY_LABEL: &str = "Stemwise";
/// How long the provider's authorization code lasts.
pub const VALID_FOR: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ConnectError {
    #[error("the connection was not approved")]
    Denied,
    #[error("the approval took longer than 10 minutes")]
    Expired,
    #[error("cancelled")]
    Cancelled,
    #[error("OpenRouter could not be reached: {0}")]
    Network(String),
    #[error("OpenRouter refused the connection ({status}): {message}")]
    Refused { status: u16, message: String },
    #[error("could not wait for the browser on this computer: {0}")]
    Local(String),
}

impl ConnectError {
    /// The word the interface switches on (design 14f).
    pub fn code(&self) -> &'static str {
        match self {
            ConnectError::Denied => "denied",
            ConnectError::Expired => "expired",
            ConnectError::Cancelled => "cancelled",
            ConnectError::Network(_) => "offline",
            ConnectError::Refused { .. } => "refused",
            ConnectError::Local(_) => "local",
        }
    }
}

/// A connection in progress: the page to open and the listener waiting for the browser to come back.
pub struct Connect {
    /// The page the user approves on.
    pub auth_url: String,
    listeners: Vec<TcpListener>,
    verifier: String,
    state: String,
    exchange_url: String,
}

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().fold(0u32, |a, b| (a << 8) | u32::from(*b)) << (8 * (3 - chunk.len()));
        for i in 0..=chunk.len() {
            out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

fn random(bytes: usize) -> Result<String, ConnectError> {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).map_err(|e| ConnectError::Local(e.to_string()))?;
    Ok(base64url(&buf))
}

/// The PKCE challenge of a verifier (S256).
pub fn challenge(verifier: &str) -> String {
    base64url(&Sha256::digest(verifier.as_bytes()))
}

fn percent(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn unpercent(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = (b[i] == b'%' && i + 2 < b.len())
            .then(|| std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()))
            .flatten();
        match hex {
            Some(v) => {
                out.push(v);
                i += 3;
            }
            None => {
                out.push(if b[i] == b'+' { b' ' } else { b[i] });
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query(path: &str, name: &str) -> Option<String> {
    let (_, q) = path.split_once('?')?;
    q.split('&').filter_map(|kv| kv.split_once('=')).find(|(k, _)| *k == name).map(|(_, v)| unpercent(v))
}

/// The Stemwise mark in its colours, drawn for 20 px (design/brand/final/mark-20.svg, A-076).
const MARK: &str = "<svg viewBox=\"0 0 20 20\" aria-hidden=\"true\"><path d=\"M1.75 4H4.4Q6 4 6.84 5.5L7.96 7.5Q8.8 9 10.5 9M1.75 16H4.4Q6 \
     16 6.84 14.5L7.96 12.5Q8.8 11 10.5 11\" fill=\"none\" stroke=\"#CEE654\" stroke-width=\"2\" stroke-linecap=\"round\" \
     stroke-linejoin=\"round\"/><path d=\"M15 7H17.25A2 2 0 0 1 19.25 9V11A2 2 0 0 1 17.25 13H15A2 2 0 0 1 13 11V9A2 2 0 0 1 15 \
     7Z\" fill=\"#CDAFFF\"/></svg>";

/// The page the browser shows at the end (design 14c). Self-contained: no script, nothing loaded from anywhere.
fn landing(ok: bool) -> String {
    let (mark, title, text) = if ok {
        ("✓", "Connected", "You can close this tab and go back to Stemwise.")
    } else {
        ("✕", "Not connected", "The request was declined or has expired. Go back to Stemwise to try again.")
    };
    let tint = if ok { "oklch(0.82 0.13 160)" } else { "oklch(0.7 0.19 22)" };
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" \
         content=\"width=device-width, initial-scale=1\"><title>Stemwise</title><style>\
         body{{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:oklch(0.195 0.02 320);\
         color:oklch(0.96 0.01 320);font:15px/1.45 system-ui,sans-serif}}main{{width:min(420px,calc(100vw - 32px));\
         box-sizing:border-box;padding:24px;border-radius:14px;background:oklch(0.24 0.022 320);border:1px solid oklch(0.36 0.026 320)}}\
         h1{{margin:0 0 16px;font-size:22px;display:flex;align-items:center;gap:8px}}h1 svg{{width:20px;height:20px;flex:none}}\
         .t{{display:flex;gap:12px;align-items:center;padding:10px 12px;\
         border-radius:8px;background:oklch(0.285 0.024 320)}}.m{{color:{tint};font-size:18px}}b{{display:block}}\
         span{{color:oklch(0.84 0.015 320);font-size:13px}}p{{margin:16px 0 0;color:oklch(0.75 0.018 320);font-size:12px}}</style></head><body>\
         <main><h1>{MARK}Stemwise</h1><div class=\"t\"><i class=\"m\">{mark}</i><div><b>{title}</b><span>{text}</span></div>\
         </div><p>This page comes from the Stemwise app on your computer, not from the internet.</p></main></body></html>"
    )
}

fn reply(stream: &mut TcpStream, status: &str, body: &str) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

/// The path of one request ("/callback?code=…"), or nothing when it is not a GET we understand.
fn request_path(stream: &TcpStream) -> Option<String> {
    stream.set_nonblocking(false).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    // The rest of the head, so the browser sees its request taken before the answer.
    for _ in 0..100 {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }
    let mut parts = first.split_whitespace();
    (parts.next()? == "GET").then(|| parts.next().map(str::to_string)).flatten()
}

impl Connect {
    /// Starts listening and builds the page to open. `base_url` is the API base of the configuration
    /// (`https://openrouter.ai/api/v1`): the authorization page sits at the same origin.
    pub fn begin(base_url: &str) -> Result<Connect, ConnectError> {
        let local = |e: std::io::Error| ConnectError::Local(e.to_string());
        let v4 = TcpListener::bind("127.0.0.1:0").map_err(local)?;
        let port = v4.local_addr().map_err(local)?.port();
        let mut listeners = vec![v4];
        // A browser may resolve "localhost" to the IPv6 loopback first: the same port there, when it is free.
        if let Ok(v6) = TcpListener::bind(("::1", port)) {
            listeners.push(v6);
        }
        for l in &listeners {
            l.set_nonblocking(true).map_err(local)?;
        }
        let verifier = random(32)?;
        let state = random(16)?;
        let api = base_url.trim_end_matches('/');
        let origin = api.strip_suffix("/api/v1").unwrap_or(api);
        let callback = format!("http://localhost:{port}/callback");
        let auth_url = format!(
            "{origin}/auth?callback_url={}&code_challenge={}&code_challenge_method=S256&key_label={}&state={}",
            percent(&callback),
            challenge(&verifier),
            percent(KEY_LABEL),
            state
        );
        Ok(Connect { auth_url, listeners, verifier, state, exchange_url: format!("{api}/auth/keys") })
    }

    fn exchange(&self, code: &str) -> Result<String, ConnectError> {
        let body = serde_json::json!({"code": code, "code_verifier": self.verifier, "code_challenge_method": "S256"});
        let resp = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .and_then(|c| c.post(&self.exchange_url).header("X-Title", "Stemwise").json(&body).send())
            .map_err(|e| ConnectError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        let text = resp.text().map_err(|e| ConnectError::Network(e.to_string()))?;
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        match v["key"].as_str().map(str::trim).filter(|k| !k.is_empty()) {
            Some(key) if status == 200 => Ok(key.to_string()),
            _ => Err(ConnectError::Refused {
                status,
                message: v["error"]["message"].as_str().map_or_else(|| "no key in the answer".into(), str::to_string),
            }),
        }
    }

    /// Waits for the browser to come back and returns the key. Ends on cancel and after `valid_for`.
    pub fn wait(self, cancel: &AtomicBool, valid_for: Duration) -> Result<String, ConnectError> {
        let started = Instant::now();
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(ConnectError::Cancelled);
            }
            if started.elapsed() >= valid_for {
                return Err(ConnectError::Expired);
            }
            let Some(mut stream) = self.listeners.iter().find_map(|l| l.accept().ok().map(|(s, _)| s)) else {
                std::thread::sleep(Duration::from_millis(25));
                continue;
            };
            let Some(path) = request_path(&stream) else { continue };
            if path.split('?').next() != Some("/callback") {
                reply(&mut stream, "404 Not Found", "");
                continue;
            }
            let Some(code) = query(&path, "code") else {
                // The provider sends the user back without a code when they decline.
                reply(&mut stream, "200 OK", &landing(false));
                return Err(ConnectError::Denied);
            };
            if query(&path, "state").as_deref() != Some(self.state.as_str()) {
                // Not the answer to the page this app opened: ignored, the real one may still come.
                reply(&mut stream, "400 Bad Request", &landing(false));
                continue;
            }
            let result = self.exchange(&code);
            reply(&mut stream, "200 OK", &landing(result.is_ok()));
            return result;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openrouter::fake_server;
    use std::io::Read;

    /// What a browser does with the callback: one GET, and the page it gets back.
    fn browse(auth_url: &str, query: &str) -> String {
        let callback = unpercent(&super::query(auth_url, "callback_url").unwrap());
        let port: u16 = callback.split(':').nth(2).unwrap().split('/').next().unwrap().parse().unwrap();
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(s, "GET /callback{query} HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n").unwrap();
        let mut page = String::new();
        let _ = s.read_to_string(&mut page);
        page
    }

    fn base(server: &fake_server::FakeServer) -> String {
        format!("{}/api/v1", server.url)
    }

    /// req: AI-24
    #[test]
    fn the_browser_comes_back_with_a_code_and_the_app_gets_a_key() {
        let server = fake_server::start(vec![(200, r#"{"key":"sk-or-v1-new"}"#.into())]);
        let c = Connect::begin(&base(&server)).unwrap();
        let url = c.auth_url.clone();
        assert!(url.starts_with(&format!("{}/auth?callback_url=http%3A%2F%2Flocalhost%3A", server.url)), "{url}");
        assert_eq!(query(&url, "code_challenge_method").as_deref(), Some("S256"));
        assert_eq!(query(&url, "key_label").as_deref(), Some("Stemwise"));
        let sent_challenge = query(&url, "code_challenge").unwrap();
        let state = query(&url, "state").unwrap();
        assert!(state.len() >= 20 && sent_challenge.len() == 43, "random state, SHA-256 challenge");
        let browser = std::thread::spawn(move || {
            // A request that is not the callback, and one with a state this app never issued, change nothing.
            assert!(browse(&url, "/../favicon.ico").starts_with("HTTP/1.1 404"));
            assert!(browse(&url, "?code=stolen&state=someone-else").starts_with("HTTP/1.1 400"));
            browse(&url, &format!("?code=abc%2F123&state={state}"))
        });
        let key = c.wait(&AtomicBool::new(false), Duration::from_secs(10)).unwrap();
        assert_eq!(key, "sk-or-v1-new");
        let page = browser.join().unwrap();
        assert!(page.starts_with("HTTP/1.1 200") && page.contains("Connected") && !page.contains("Not connected"));
        assert!(!page.contains("sk-or"), "the key never reaches the browser");
        let reqs = server.requests.lock().unwrap();
        assert_eq!(reqs.len(), 1, "one exchange: the stray requests reached no one");
        assert_eq!(reqs[0]["code"], "abc/123");
        assert_eq!(reqs[0]["code_challenge_method"], "S256");
        assert_eq!(challenge(reqs[0]["code_verifier"].as_str().unwrap()), sent_challenge, "PKCE: verifier matches");
        assert_eq!(reqs[0]["_auth"], "", "no credential is sent to get one");
    }

    /// req: AI-24
    #[test]
    fn declined_expired_cancelled_and_refused_connections_say_which() {
        // Declined: the browser comes back without a code.
        let c = Connect::begin("http://127.0.0.1:9/api/v1").unwrap();
        let url = c.auth_url.clone();
        let browser = std::thread::spawn(move || browse(&url, "?error=access_denied"));
        assert_eq!(c.wait(&AtomicBool::new(false), Duration::from_secs(10)), Err(ConnectError::Denied));
        assert!(browser.join().unwrap().contains("Not connected"));

        // Nobody comes back: the wait ends by itself, and the port is free again.
        let c = Connect::begin("http://127.0.0.1:9/api/v1").unwrap();
        let url = c.auth_url.clone();
        let t = Instant::now();
        assert_eq!(c.wait(&AtomicBool::new(false), Duration::from_millis(120)), Err(ConnectError::Expired));
        assert!(t.elapsed() < Duration::from_secs(2));
        let callback = unpercent(&query(&url, "callback_url").unwrap());
        let port: u16 = callback.split(':').nth(2).unwrap().split('/').next().unwrap().parse().unwrap();
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err(), "nothing listens once the wait is over");

        // Cancelled by the user.
        let c = Connect::begin("http://127.0.0.1:9/api/v1").unwrap();
        assert_eq!(c.wait(&AtomicBool::new(true), VALID_FOR), Err(ConnectError::Cancelled));

        // The provider refuses the code: the browser is told, and so is the app.
        let server = fake_server::start(vec![(403, r#"{"error":{"message":"Invalid code or code_verifier"}}"#.into())]);
        let c = Connect::begin(&base(&server)).unwrap();
        let url = c.auth_url.clone();
        let state = query(&url, "state").unwrap();
        let browser = std::thread::spawn(move || browse(&url, &format!("?code=x&state={state}")));
        let e = c.wait(&AtomicBool::new(false), Duration::from_secs(10)).unwrap_err();
        assert_eq!((e.code(), e.to_string().contains("Invalid code")), ("refused", true), "{e}");
        assert!(browser.join().unwrap().contains("Not connected"));

        // The provider cannot be reached for the exchange.
        let closed = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        let c = Connect::begin(&format!("http://{closed}/api/v1")).unwrap();
        let url = c.auth_url.clone();
        let state = query(&url, "state").unwrap();
        std::thread::spawn(move || browse(&url, &format!("?code=x&state={state}")));
        assert_eq!(c.wait(&AtomicBool::new(false), Duration::from_secs(10)).unwrap_err().code(), "offline");
    }

    /// req: AI-24
    #[test]
    fn encoding_helpers_follow_the_standards() {
        // RFC 4648 §10 vectors, without padding, in the URL alphabet.
        for (plain, enc) in
            [("", ""), ("f", "Zg"), ("fo", "Zm8"), ("foo", "Zm9v"), ("foob", "Zm9vYg"), ("fooba", "Zm9vYmE")]
        {
            assert_eq!(base64url(plain.as_bytes()), enc);
        }
        assert_eq!(base64url(&[0xfb, 0xff]), "-_8");
        // RFC 7636 appendix B.
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        assert_eq!(percent("http://localhost:5/a b"), "http%3A%2F%2Flocalhost%3A5%2Fa%20b");
        assert_eq!(unpercent("a%2Fb+c%"), "a/b c%");
        assert_ne!(random(16).unwrap(), random(16).unwrap());
    }
}
