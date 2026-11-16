//! Stereo delay return (FX-6, FX-23).
//!
//! Structure (f64 arithmetic, state flushed with [`flush`] so tails end in exact zeros):
//! - two independent delay lines (L, R) with the same time, buffers for 2.1 s, written with
//!   `input + feedback · filtered(read)`; the output is the delayed signal (100% wet);
//! - feedback path: RBJ Butterworth (Q 1/√2) high-pass at `feedback_hpf_hz` then low-pass at `feedback_lpf_hz`;
//!   `feedback` is clamped to 0..0.95;
//! - time changes glide: the delay time (in samples) follows its target with a one-pole of about 50 ms per sample,
//!   and the line is read with linear interpolation at the fractional position, so sweeps do not click and the
//!   result does not depend on the block size. The first `set_params` call (and `reset`) snaps the smoothed time.
//!   Filter coefficients are redesigned in `set_params` only when their frequency changes.
//!
//! FX-23 adds two modes and two characters; mode `Stereo` with character `Clean` runs the original code path
//! unchanged (bit-identical output, whatever `wow` and `drive_db` are):
//! - `PingPong`: the input summed to mono (`(L+R)/2`) feeds the left line only; every repeat crosses to the other
//!   channel (L, R, L, R…) at the same time, through the same feedback filters;
//! - `Tape`: the delay time is modulated by wow (1 Hz, ±0.4% of the time) plus flutter (7 Hz, ±0.1%), both scaled by
//!   `wow`, read with linear interpolation; the LFO phases are a function of the song position only (`process_at`),
//!   so a render started anywhere is identical to the same part of a full render. `drive_db` adds a soft saturation
//!   `tanh(g·x)/g` (g = 10^(drive/20), unity slope at zero) on the feedback signal, which keeps the loop bounded by
//!   `|in| + 0.95/g` and stable.
//!
//! Properties verified by the tests: echo position `round(time · rate)` (±1 sample), echo decay of
//! `20 · log10(feedback)` dB per repeat with open filters (±0.5 dB), boundedness at feedback 0.95 with full-scale
//! noise, no clicks while sweeping the time, block-size invariance, exact silence after the tail, golden checksum of
//! the Stereo/Clean path, alternation of the ping-pong repeats, tape with no wow and no drive equal to clean,
//! position-only dependence of the wow, bounded saturated loop.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};
use crate::util::{flush, one_pole};

const MAX_TIME_MS: f64 = 2000.0;
const BUFFER_SECS: f64 = 2.1;
const GLIDE_MS: f64 = 50.0;
const WOW_HZ: f64 = 1.0;
const FLUTTER_HZ: f64 = 7.0;
/// Relative time deviation at `wow` = 1.
const WOW_DEPTH: f64 = 0.004;
const FLUTTER_DEPTH: f64 = 0.001;
/// `next_pos` value that never matches a position: the LFO phases are recomputed on the next tape call.
const NO_POS: i64 = i64::MIN;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DelayMode {
    #[default]
    Stereo,
    PingPong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DelayCharacter {
    #[default]
    Clean,
    Tape,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelayParams {
    /// 1..2000 ms
    pub time_ms: f64,
    /// 0..0.95
    pub feedback: f64,
    /// 20..2000 Hz
    pub feedback_hpf_hz: f64,
    /// 500..20000 Hz
    pub feedback_lpf_hz: f64,
    pub mode: DelayMode,
    pub character: DelayCharacter,
    /// 0..1, wow and flutter amount (Tape only)
    pub wow: f64,
    /// 0..12 dB, saturation in the feedback loop (Tape only)
    pub drive_db: f64,
}

#[derive(Debug, Clone)]
pub struct Delay {
    rate: f64,
    buf: [Vec<f64>; 2],
    write: usize,
    hpf: Biquad,
    lpf: Biquad,
    hpf_hz: f64,
    lpf_hz: f64,
    feedback: f64,
    smooth: f64,
    time: f64,
    time_target: f64,
    initialized: bool,
    mode: DelayMode,
    character: DelayCharacter,
    wow_target: f64,
    wow: f64,
    gain_target: f64,
    gain: f64,
    wow_phase: f64,
    flutter_phase: f64,
    next_pos: i64,
    run_pos: i64,
}

impl Delay {
    /// Allocates buffers for 2.1 s.
    pub fn new(rate: f64) -> Self {
        let len = (BUFFER_SECS * rate).ceil() as usize + 4;
        let mut d = Delay {
            rate,
            buf: [vec![0.0; len], vec![0.0; len]],
            write: 0,
            hpf: Biquad::default(),
            lpf: Biquad::default(),
            hpf_hz: -1.0,
            lpf_hz: -1.0,
            feedback: 0.0,
            smooth: one_pole(GLIDE_MS, rate),
            time: 0.0,
            time_target: 0.0,
            initialized: false,
            mode: DelayMode::Stereo,
            character: DelayCharacter::Clean,
            wow_target: 0.0,
            wow: 0.0,
            gain_target: 1.0,
            gain: 1.0,
            wow_phase: 0.0,
            flutter_phase: 0.0,
            next_pos: NO_POS,
            run_pos: 0,
        };
        d.set_params(&DelayParams {
            time_ms: 250.0,
            feedback: 0.0,
            feedback_hpf_hz: 20.0,
            feedback_lpf_hz: 20000.0,
            mode: DelayMode::Stereo,
            character: DelayCharacter::Clean,
            wow: 0.0,
            drive_db: 0.0,
        });
        d.initialized = false;
        d
    }

    /// No allocation; may be called up to every 32 samples.
    pub fn set_params(&mut self, p: &DelayParams) {
        self.feedback = p.feedback.clamp(0.0, 0.95);
        self.time_target = p.time_ms.clamp(1.0, MAX_TIME_MS) * 0.001 * self.rate;
        let hp = p.feedback_hpf_hz.clamp(20.0, 2000.0);
        if hp != self.hpf_hz {
            self.hpf_hz = hp;
            self.hpf.c = Coeffs::design(Kind::HighPass, hp, 0.0, BUTTERWORTH_Q, self.rate);
        }
        let lp = p.feedback_lpf_hz.clamp(500.0, 20000.0);
        if lp != self.lpf_hz {
            self.lpf_hz = lp;
            self.lpf.c = Coeffs::design(Kind::LowPass, lp, 0.0, BUTTERWORTH_Q, self.rate);
        }
        self.mode = p.mode;
        self.character = p.character;
        self.wow_target = p.wow.clamp(0.0, 1.0);
        self.gain_target = 10f64.powf(p.drive_db.clamp(0.0, 12.0) / 20.0);
        if !self.initialized {
            self.time = self.time_target;
            self.wow = self.wow_target;
            self.gain = self.gain_target;
            self.initialized = true;
        }
    }

    pub fn reset(&mut self) {
        for b in &mut self.buf {
            b.fill(0.0);
        }
        self.hpf.reset();
        self.lpf.reset();
        self.time = self.time_target;
        self.wow = self.wow_target;
        self.gain = self.gain_target;
        self.next_pos = NO_POS;
        self.run_pos = 0;
    }

    /// 100% wet delay. Uses an internal running song position (0 after `new`/`reset`).
    pub fn process(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32]) {
        let n = in_l.len().min(in_r.len()).min(out_l.len()).min(out_r.len());
        let pos = self.run_pos;
        self.process_at(in_l, in_r, out_l, out_r, pos);
        self.run_pos = pos + n as i64;
    }

    /// Like [`Delay::process`], with `pos` the timeline frame of the first sample. The tape LFO phase depends only on
    /// the song position: it is recomputed whenever `pos` is not the frame following the previous call (first call,
    /// seek) and accumulated otherwise.
    pub fn process_at(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32], pos: i64) {
        let n = in_l.len().min(in_r.len()).min(out_l.len()).min(out_r.len());
        if self.mode == DelayMode::Stereo && self.character == DelayCharacter::Clean {
            self.next_pos = NO_POS;
            self.process_classic(in_l, in_r, out_l, out_r, n);
            return;
        }
        self.process_general(in_l, in_r, out_l, out_r, n, pos);
    }

    /// The original stereo clean path, kept unchanged (bit-exact output).
    fn process_classic(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32], n: usize) {
        let len = self.buf[0].len();
        for i in 0..n {
            self.time += self.smooth * (self.time_target - self.time);
            if (self.time_target - self.time).abs() < 1e-9 {
                self.time = self.time_target;
            }
            let whole = self.time.floor();
            let frac = self.time - whole;
            let w = whole as usize;
            let ia = (self.write + len - w) % len;
            let ib = (self.write + len - w - 1) % len;
            let input = [f64::from(in_l[i]), f64::from(in_r[i])];
            let mut out = [0.0; 2];
            for ch in 0..2 {
                let a = self.buf[ch][ia];
                let b = self.buf[ch][ib];
                let d = if frac == 0.0 { a } else { a + (b - a) * frac };
                out[ch] = d;
                let fb = self.lpf.tick(ch, self.hpf.tick(ch, d)) * self.feedback;
                self.buf[ch][self.write] = flush(input[ch] + fb);
            }
            out_l[i] = out[0] as f32;
            out_r[i] = out[1] as f32;
            self.write += 1;
            if self.write == len {
                self.write = 0;
            }
        }
    }

    fn process_general(
        &mut self,
        in_l: &[f32],
        in_r: &[f32],
        out_l: &mut [f32],
        out_r: &mut [f32],
        n: usize,
        pos: i64,
    ) {
        let len = self.buf[0].len();
        let tape = self.character == DelayCharacter::Tape;
        let ping_pong = self.mode == DelayMode::PingPong;
        if tape {
            if pos != self.next_pos {
                let p = pos as f64 / self.rate;
                self.wow_phase = (WOW_HZ * p).rem_euclid(1.0);
                self.flutter_phase = (FLUTTER_HZ * p).rem_euclid(1.0);
            }
            self.next_pos = pos + n as i64;
        } else {
            self.next_pos = NO_POS;
        }
        let wow_inc = WOW_HZ / self.rate;
        let flutter_inc = FLUTTER_HZ / self.rate;
        let max_time = (len - 3) as f64;
        for i in 0..n {
            self.time += self.smooth * (self.time_target - self.time);
            if (self.time_target - self.time).abs() < 1e-9 {
                self.time = self.time_target;
            }
            let mut t = self.time;
            let mut drive = false;
            if tape {
                self.wow += self.smooth * (self.wow_target - self.wow);
                if (self.wow_target - self.wow).abs() < 1e-9 {
                    self.wow = self.wow_target;
                }
                self.gain += self.smooth * (self.gain_target - self.gain);
                if (self.gain_target - self.gain).abs() < 1e-9 {
                    self.gain = self.gain_target;
                }
                drive = self.gain > 1.0;
                if self.wow > 0.0 {
                    let w = WOW_DEPTH * (std::f64::consts::TAU * self.wow_phase).sin();
                    let f = FLUTTER_DEPTH * (std::f64::consts::TAU * self.flutter_phase).sin();
                    t = (t * (1.0 + self.wow * (w + f))).clamp(1.0, max_time);
                }
                self.wow_phase += wow_inc;
                if self.wow_phase >= 1.0 {
                    self.wow_phase -= 1.0;
                }
                self.flutter_phase += flutter_inc;
                if self.flutter_phase >= 1.0 {
                    self.flutter_phase -= 1.0;
                }
            }
            let whole = t.floor();
            let frac = t - whole;
            let w = whole as usize;
            let ia = (self.write + len - w) % len;
            let ib = (self.write + len - w - 1) % len;
            let mut d = [0.0; 2];
            for ch in 0..2 {
                let a = self.buf[ch][ia];
                let b = self.buf[ch][ib];
                d[ch] = if frac == 0.0 { a } else { a + (b - a) * frac };
            }
            let mut fb = [0.0; 2];
            for ch in 0..2 {
                let mut x = self.lpf.tick(ch, self.hpf.tick(ch, d[ch]));
                if drive {
                    x = (self.gain * x).tanh() / self.gain;
                }
                fb[ch] = x * self.feedback;
            }
            let (l, r) = (f64::from(in_l[i]), f64::from(in_r[i]));
            if ping_pong {
                self.buf[0][self.write] = flush(0.5 * (l + r) + fb[1]);
                self.buf[1][self.write] = flush(fb[0]);
            } else {
                self.buf[0][self.write] = flush(l + fb[0]);
                self.buf[1][self.write] = flush(r + fb[1]);
            }
            out_l[i] = d[0] as f32;
            out_r[i] = d[1] as f32;
            self.write += 1;
            if self.write == len {
                self.write = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::xorshift32;

    const FS: u32 = 48000;

    fn params(time: f64, fb: f64, hp: f64, lp: f64) -> DelayParams {
        DelayParams {
            time_ms: time,
            feedback: fb,
            feedback_hpf_hz: hp,
            feedback_lpf_hz: lp,
            mode: DelayMode::Stereo,
            character: DelayCharacter::Clean,
            wow: 0.0,
            drive_db: 0.0,
        }
    }

    fn run(p: &DelayParams, l: &[f32], r: &[f32], block: usize) -> (Vec<f32>, Vec<f32>) {
        let mut d = Delay::new(f64::from(FS));
        d.set_params(p);
        let (mut ol, mut or) = (vec![0f32; l.len()], vec![0f32; l.len()]);
        let mut i = 0;
        while i < l.len() {
            let e = (i + block).min(l.len());
            d.process(&l[i..e], &r[i..e], &mut ol[i..e], &mut or[i..e]);
            i = e;
        }
        (ol, or)
    }

    fn impulse(n: usize) -> Vec<f32> {
        let mut v = vec![0f32; n];
        v[0] = 1.0;
        v
    }

    /// req: FX-6
    #[test]
    fn impulse_returns_at_the_delay_time() {
        // ±1 sample: interpolation may spread an echo onto the neighbouring sample.
        for ms in [10.0, 123.4, 500.0] {
            let imp = impulse(FS as usize);
            let (ol, or) = run(&params(ms, 0.0, 20.0, 20000.0), &imp, &imp, 256);
            let want = (ms * f64::from(FS) / 1000.0).round() as i64;
            for o in [&ol, &or] {
                let pos = o.iter().position(|v| v.abs() > 0.1).unwrap() as i64;
                assert!((pos - want).abs() <= 1, "{ms}: {pos} vs {want}");
            }
        }
    }

    /// req: FX-6
    #[test]
    fn echoes_decay_by_the_feedback() {
        // ±0.5 dB: a 1 kHz burst sits in the flat band of the open filters (20 Hz HPF, 20 kHz LPF, both well under
        // 0.1 dB there), so each repeat is the previous one times the feedback; peak sampling adds ≈ 0.02 dB.
        let fb = 0.6;
        let ms = 100.0;
        let step = (ms * f64::from(FS) / 1000.0) as usize;
        let mut imp = vec![0f32; 6 * step];
        for (i, v) in imp.iter_mut().take(480).enumerate() {
            *v = (0.5 * (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48000.0).sin()) as f32;
        }
        let (ol, _) = run(&params(ms, fb, 20.0, 20000.0), &imp, &imp, 512);
        let peak = |k: usize| ol[k * step - 3..k * step + 484].iter().fold(0f64, |m, v| m.max(f64::from(v.abs())));
        for k in 1..4 {
            let db = 20.0 * (peak(k + 1) / peak(k)).log10();
            let want = 20.0 * fb.log10();
            assert!((db - want).abs() <= 0.5, "repeat {k}: {db:.2} vs {want:.2}");
        }
    }

    /// req: FX-6
    #[test]
    fn maximum_feedback_with_full_scale_noise_stays_bounded() {
        let n = 10 * FS as usize;
        let l: Vec<f32> = xorshift32(3, n).iter().map(|v| *v as f32).collect();
        let r: Vec<f32> = xorshift32(4, n).iter().map(|v| *v as f32).collect();
        let (ol, or) = run(&params(30.0, 0.95, 20.0, 20000.0), &l, &r, 512);
        // Steady state of a 0.95 feedback loop is at most 1/(1−0.95) = 20 times the input peak (the filters
        // only reduce it); 25 leaves headroom for filter overshoot.
        assert!(ol.iter().chain(or.iter()).all(|v| v.is_finite() && v.abs() < 25.0));
    }

    /// req: FX-6
    #[test]
    fn time_sweep_does_not_click() {
        let n = 2 * FS as usize;
        let l: Vec<f32> =
            (0..n).map(|i| (0.5 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48000.0).sin()) as f32).collect();
        let max_in = l.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
        let mut d = Delay::new(f64::from(FS));
        let (mut ol, mut or) = (vec![0f32; n], vec![0f32; n]);
        let mut i = 0;
        while i + 32 <= n {
            let t = (i as f64 / f64::from(FS)).min(1.0);
            d.set_params(&params(100.0 + 500.0 * t, 0.5, 20.0, 20000.0));
            d.process(&l[i..i + 32], &l[i..i + 32], &mut ol[i..i + 32], &mut or[i..i + 32]);
            i += 32;
        }
        // Skip the first second (echo start-up); a gliding read head changes pitch, so steps may exceed the input's
        // by a small factor; ×4 is the specified limit for a click.
        let skip = FS as usize + 100;
        let max_out = ol[skip..].windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
        assert!(max_out <= max_in * 4.0, "{max_out} vs {max_in}");
        // The same check over the whole sweep, including the start-up echo.
        let max_all = ol.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
        assert!(max_all <= max_in * 4.0, "{max_all} vs {max_in}");
    }

    /// req: FX-6
    #[test]
    fn block_size_does_not_change_the_output() {
        let n = 2 * FS as usize;
        let l: Vec<f32> = xorshift32(8, n).iter().map(|v| *v as f32 * 0.5).collect();
        let r: Vec<f32> = xorshift32(9, n).iter().map(|v| *v as f32 * 0.5).collect();
        let p = params(333.0, 0.7, 100.0, 5000.0);
        let (a, _) = run(&p, &l, &r, 64);
        let (b, _) = run(&p, &l, &r, 4096);
        // Same per-sample arithmetic: any difference above 1e-6 would be a design error, not rounding.
        assert!(a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= 1e-6));
    }

    /// req: FX-6
    #[test]
    fn silence_after_the_tail_is_exact_zero() {
        // 20 ms line at feedback 0.5: 1e-30 is reached after about 100 repeats (2 s); check from 6 s on.
        let n = 8 * FS as usize;
        let imp = impulse(n);
        let (ol, or) = run(&params(20.0, 0.5, 20.0, 20000.0), &imp, &imp, 1024);
        let tail = 6 * FS as usize;
        assert!(ol[tail..].iter().chain(or[tail..].iter()).all(|v| *v == 0.0));
        assert!(ol.iter().any(|v| *v != 0.0));
    }

    fn full(p: &DelayParams, mode: DelayMode, ch: DelayCharacter, wow: f64, drive: f64) -> DelayParams {
        DelayParams { mode, character: ch, wow, drive_db: drive, ..*p }
    }

    fn noise(n: usize, s1: u32, s2: u32, g: f32) -> (Vec<f32>, Vec<f32>) {
        (
            xorshift32(s1, n).iter().map(|v| *v as f32 * g).collect(),
            xorshift32(s2, n).iter().map(|v| *v as f32 * g).collect(),
        )
    }

    /// req: FX-23, FX-24
    #[test]
    fn stereo_clean_is_bit_identical_to_the_original_whatever_wow_and_drive() {
        // Checksum (FNV-1a over the f32 bits) recorded by running the code that existed before FX-23.
        const GOLDEN: u64 = 0x5fb0fe735ff8c856;
        let n = 3 * FS as usize;
        let (l, r) = noise(n, 21, 22, 0.5);
        for (wow, drive) in [(0.0, 0.0), (1.0, 12.0)] {
            let mut d = Delay::new(f64::from(FS));
            let (mut ol, mut or) = (vec![0f32; n], vec![0f32; n]);
            let mut i = 0;
            while i < n {
                let t = (i / 32) as f64;
                let p = params(200.0 + 50.0 * (t * 0.01).sin(), 0.7, 100.0 + 10.0 * (t * 0.02).cos(), 6000.0);
                d.set_params(&full(&p, DelayMode::Stereo, DelayCharacter::Clean, wow, drive));
                let e = (i + 32).min(n);
                d.process_at(&l[i..e], &r[i..e], &mut ol[i..e], &mut or[i..e], i as i64);
                i = e;
            }
            let mut h: u64 = 0xcbf29ce484222325;
            for v in ol.iter().chain(or.iter()) {
                h ^= u64::from(v.to_bits());
                h = h.wrapping_mul(0x100000001b3);
            }
            assert_eq!(h, GOLDEN, "wow {wow} drive {drive}");
        }
    }

    /// req: FX-23
    #[test]
    fn ping_pong_repeats_alternate_between_the_channels() {
        let ms = 100.0;
        let step = (ms * f64::from(FS) / 1000.0) as usize;
        let imp = impulse(6 * step);
        let p = full(&params(ms, 0.5, 20.0, 20000.0), DelayMode::PingPong, DelayCharacter::Clean, 0.0, 0.0);
        let (ol, or) = run(&p, &imp, &imp, 256);
        // The 20 kHz low-pass smears each repeat over a few samples (unity DC gain): measure the echo as the sum
        // over ±16 samples, which is its amplitude; the other channel is measured the same way.
        let sum = |o: &[f32], k: usize| o[k * step - 16..k * step + 17].iter().sum::<f32>();
        for k in 1..=4usize {
            let (on, off) = if k % 2 == 1 { (&ol, &or) } else { (&or, &ol) };
            // Echo k has amplitude 0.5^(k-1) (mono input 1); 20% covers the 20 Hz high-pass droop (≈6% per pass, its tail lies outside the window). The other
            // channel carries only filter leakage, far below 1e-3 (the high-pass tail has long decayed).
            let want = 0.5f32.powi(k as i32 - 1);
            assert!((sum(on, k) - want).abs() < 0.2 * want, "k {k}: {}", sum(on, k));
            assert!(sum(off, k).abs() < 1e-3, "k {k}: off {}", sum(off, k));
        }
    }

    /// req: FX-23, FX-24
    #[test]
    fn tape_without_wow_and_drive_equals_clean() {
        let n = 2 * FS as usize;
        let (l, r) = noise(n, 5, 6, 0.5);
        let p = params(180.0, 0.6, 80.0, 8000.0);
        let (a, ar) = run(&p, &l, &r, 32);
        let (b, br) = run(&full(&p, DelayMode::Stereo, DelayCharacter::Tape, 0.0, 0.0), &l, &r, 32);
        // Same arithmetic when the modulation and saturation are off: 1e-6 is the float-to-float tolerance.
        assert!(a.iter().zip(&b).chain(ar.iter().zip(&br)).all(|(x, y)| (x - y).abs() <= 1e-6));
    }

    /// req: FX-23
    #[test]
    fn tape_wow_depends_only_on_the_song_position() {
        let total = 31 * FS as usize;
        let start = 30 * FS as usize;
        let (l, r) = noise(total, 31, 32, 0.5);
        // Feedback 0.2 at 50 ms: after 0.5 s (10 repeats) the missing history is below 0.2^10 ≈ 1e-7.
        let p = full(&params(50.0, 0.2, 20.0, 20000.0), DelayMode::Stereo, DelayCharacter::Tape, 1.0, 0.0);
        let render = |from: usize| {
            let mut d = Delay::new(f64::from(FS));
            d.set_params(&p);
            let (mut ol, mut or) = (vec![0f32; total - from], vec![0f32; total - from]);
            let mut i = from;
            while i < total {
                let e = (i + 32).min(total);
                let (o, e2) = (i - from, e - from);
                d.process_at(&l[i..e], &r[i..e], &mut ol[o..e2], &mut or[o..e2], i as i64);
                i = e;
            }
            (ol, or)
        };
        let (fl, fr) = render(0);
        let (pl, pr) = render(start);
        // The LFO phase is recomputed from the position vs accumulated; the difference is rounding (≈1e-10 turns).
        let skip = FS as usize / 2;
        for (f, q) in [(&fl, &pl), (&fr, &pr)] {
            let m = (start + skip..total).map(|i| (f[i] - q[i - start]).abs()).fold(0f32, f32::max);
            assert!(m <= 1e-5, "{m}");
        }
        // The modulation is audible: it differs from the unmodulated render.
        let (cl, _) = run(
            &full(&p, DelayMode::Stereo, DelayCharacter::Clean, 0.0, 0.0),
            &l[..FS as usize],
            &r[..FS as usize],
            32,
        );
        assert!(cl.iter().zip(&fl).any(|(a, b)| (a - b).abs() > 1e-3));
    }

    /// req: FX-23
    #[test]
    fn saturated_loop_stays_bounded_at_maximum_feedback() {
        let n = 10 * FS as usize;
        let (l, r) = noise(n, 3, 4, 1.0);
        for mode in [DelayMode::Stereo, DelayMode::PingPong] {
            let p = full(&params(30.0, 0.95, 20.0, 20000.0), mode, DelayCharacter::Tape, 1.0, 12.0);
            let (ol, or) = run(&p, &l, &r, 512);
            // Stored value ≤ |in| + 0.95/g with g = 10^(12/20) ≈ 3.98, i.e. ≤ 1.24; the read interpolates between
            // stored values and so stays inside that range.
            assert!(ol.iter().chain(or.iter()).all(|v| v.is_finite() && v.abs() < 1.3), "{mode:?}");
        }
    }

    /// req: FX-23, FX-24
    #[test]
    fn new_modes_do_not_depend_on_the_block_size() {
        let n = 2 * FS as usize;
        let (l, r) = noise(n, 8, 9, 0.5);
        for mode in [DelayMode::Stereo, DelayMode::PingPong] {
            let p = full(&params(333.0, 0.7, 100.0, 5000.0), mode, DelayCharacter::Tape, 0.8, 6.0);
            let (a, ar) = run(&p, &l, &r, 32);
            let (b, br) = run(&p, &l, &r, 1000);
            let (c, cr) = run(&p, &l, &r, 7);
            // Same per-sample arithmetic and position-only LFO: any difference above 1e-6 is a design error.
            for (x, y) in [(&a, &b), (&a, &c), (&ar, &br), (&ar, &cr)] {
                assert!(x.iter().zip(y.iter()).all(|(u, v)| (u - v).abs() <= 1e-6), "{mode:?}");
            }
        }
    }

    /// req: FX-23, FX-24
    #[test]
    fn parameters_changing_every_32_samples_stay_stable() {
        let n = 6 * FS as usize;
        let (l, r) = noise(n, 11, 12, 1.0);
        for mode in [DelayMode::Stereo, DelayMode::PingPong] {
            let mut d = Delay::new(f64::from(FS));
            let (mut ol, mut or) = (vec![0f32; n], vec![0f32; n]);
            let mut i = 0;
            let mut s = 1u32;
            let mut rnd = || {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                f64::from(s) / f64::from(u32::MAX)
            };
            while i < n {
                let mut p = params(20.0 + 400.0 * rnd(), 0.95 * rnd(), 20.0 + 500.0 * rnd(), 2000.0 + 15000.0 * rnd());
                p = full(&p, mode, DelayCharacter::Tape, rnd(), 6.0 + 6.0 * rnd());
                d.set_params(&p);
                let e = (i + 32).min(n);
                d.process_at(&l[i..e], &r[i..e], &mut ol[i..e], &mut or[i..e], i as i64);
                i = e;
            }
            // Drive ≥ 6 dB (g ≥ 2) bounds the stored value by |in| + 0.95/2 ≈ 1.48.
            assert!(ol.iter().chain(or.iter()).all(|v| v.is_finite() && v.abs() < 1.6), "{mode:?}");
        }
    }
}
