//! Synthetic test signals defined identically in `app/scripts/oracle.py` (FOUNDATION §5.7).
//!
//! Signals are described by the JSON specs of the oracle fixtures; generation follows closed formulas and a
//! hand-written xorshift32 generator, never a library RNG. Output is (frames × channels) interleaved f32,
//! computed in f64 and rounded once, like the Python side.

use serde_json::Value;

/// xorshift32: x ^= x<<13; x ^= x>>17; x ^= x<<5; value = x / 2³¹ − 1, in [−1, 1).
pub fn xorshift32(seed: u32, n: usize) -> Vec<f64> {
    let mut x = if seed == 0 { 0x9E37_79B9 } else { seed };
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            f64::from(x) / 2_147_483_648.0 - 1.0
        })
        .collect()
}

fn f(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}

/// Mono signal in f64 from a spec.
pub fn mono(spec: &Value, fs: u32) -> Vec<f64> {
    let kind = spec["kind"].as_str().expect("signal kind");
    let fsf = f64::from(fs);
    // Python: int(round(secs * fs)) — round half to even on exact halves, which do not occur here.
    let n = (f(spec, "secs") * fsf).round() as usize;
    let two_pi = 2.0 * std::f64::consts::PI;
    match kind {
        "sine" => {
            let (a, fr, ph) = (f(spec, "amp"), f(spec, "freq"), f(spec, "phase"));
            (0..n).map(|t| a * (two_pi * fr * t as f64 / fsf + ph).sin()).collect()
        }
        "silence" => vec![0.0; n],
        "dc" => vec![f(spec, "amp"); n],
        "noise" => {
            let a = f(spec, "amp");
            xorshift32(spec["seed"].as_u64().expect("seed") as u32, n).into_iter().map(|w| a * w).collect()
        }
        "pink" => {
            let a = f(spec, "amp");
            let w = xorshift32(spec["seed"].as_u64().expect("seed") as u32, n);
            let mut b = [0.0f64; 7];
            w.iter()
                .map(|&x| {
                    b[0] = 0.99886 * b[0] + x * 0.0555179;
                    b[1] = 0.99332 * b[1] + x * 0.0750759;
                    b[2] = 0.96900 * b[2] + x * 0.1538520;
                    b[3] = 0.86650 * b[3] + x * 0.3104856;
                    b[4] = 0.55000 * b[4] + x * 0.5329522;
                    b[5] = -0.7616 * b[5] - x * 0.0168980;
                    let y = (b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + x * 0.5362) * 0.11 * a;
                    b[6] = x * 0.115926;
                    y
                })
                .collect()
        }
        "clicks" => {
            let mut x = vec![0.0; n];
            let period = (fsf / f(spec, "rate")).round() as usize;
            let mut i = f(spec, "offset") as usize;
            while i < n {
                x[i] = f(spec, "amp");
                i += period;
            }
            x
        }
        "bursts" => {
            let (a, fr) = (f(spec, "amp"), f(spec, "freq"));
            let period = (f(spec, "period") * fsf).round() as usize;
            let on = (f(spec, "on") * fsf).round() as usize;
            (0..n).map(|t| if t % period < on { a * (two_pi * fr * t as f64 / fsf).sin() } else { 0.0 }).collect()
        }
        "concat" => spec["parts"].as_array().expect("parts").iter().flat_map(|p| mono(p, fs)).collect(),
        "sum" => {
            let parts: Vec<Vec<f64>> = spec["parts"].as_array().expect("parts").iter().map(|p| mono(p, fs)).collect();
            let len = parts.iter().map(Vec::len).max().unwrap_or(0);
            let mut acc = vec![0.0; len];
            for p in parts {
                for (a, v) in acc.iter_mut().zip(p) {
                    *a += v;
                }
            }
            acc
        }
        "gain" => {
            let g = 10f64.powf(f(spec, "db") / 20.0);
            mono(&spec["of"], fs).into_iter().map(|v| v * g).collect()
        }
        other => panic!("unknown signal kind {other}"),
    }
}

/// Interleaved f32 signal and its channel count; `{"kind": "stereo", "left", "right"}` gives two channels.
pub fn render(spec: &Value, fs: u32) -> (Vec<f32>, usize) {
    if spec["kind"] == "stereo" {
        let l = mono(&spec["left"], fs);
        let r = mono(&spec["right"], fs);
        let n = l.len().max(r.len());
        let mut out = vec![0f32; n * 2];
        for (i, v) in l.iter().enumerate() {
            out[2 * i] = *v as f32;
        }
        for (i, v) in r.iter().enumerate() {
            out[2 * i + 1] = *v as f32;
        }
        return (out, 2);
    }
    (mono(spec, fs).into_iter().map(|v| v as f32).collect(), 1)
}

/// Reads a fixture of `app/fixtures/oracle`.
pub fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/oracle").join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e} (run `make oracle`)", path.display()));
    serde_json::from_str(&text).expect("fixture JSON")
}

/// A fixture number, mapping the ±1e9 sentinels back to infinities; `None` for JSON null.
pub fn num(v: &Value) -> Option<f64> {
    let x = v.as_f64()?;
    Some(if x >= 1e9 {
        f64::INFINITY
    } else if x <= -1e9 {
        f64::NEG_INFINITY
    } else {
        x
    })
}
