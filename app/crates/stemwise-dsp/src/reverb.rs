//! Algorithmic reverb return (FX-5, FX-9): three own implementations of published designs.
//!
//! - **Plate**: Dattorro's plate ("Effect Design, Part 1", JAES 1997). Mono sum → input bandwidth low-pass → 4
//!   input allpass diffusers → a figure-eight tank of two halves, each a modulated allpass, a delay, a damping
//!   shelf, a second allpass and a second delay; the output is 7 taps per side taken across the tank. Dense from
//!   the first milliseconds, no early reflections. One departure from the paper: the four output taps it takes
//!   inside the tank allpasses are taken from the delay line that follows each of them, because the inside of an
//!   allpass is a comb and its period was audible in the tail.
//! - **Room** and **Hall**: a feedback delay network of 16 lines mixed by a Hadamard matrix (lossless before the
//!   per-line attenuation), fed in stereo (left on even lines, right on odd ones) through 2 (room) or 4 (hall)
//!   allpass diffusers, plus 6 early reflections per side taken before the diffusers. The hall has lines about
//!   2.4 times longer, more input diffusion and softer early reflections.
//!
//! - **Shimmer** (FX-22): the hall network with a pitch shifter ([`PitchShifter`], 20 ms window) in the
//!   recirculation. The tail the network produces is shifted by `shimmer_semitones` (5, 7 or 12) and fed back
//!   through the lines, mixed with the normal feedback as `(1 − a/2)·feedback + a/2·shifted` (`a = shimmer_amount`),
//!   so the loop gain stays below that of the plain hall (the shifter's output is a convex combination of its
//!   input samples) and the tail stays bounded and ends in exact silence. At `a = 0` it is the hall, sample for
//!   sample. Plate, room and hall do not know about it: their code paths are the ones they always had.
//!
//! Common to the algorithms (all arithmetic in f64, state flushed with [`flush`] so tails end in exact zeros):
//! - `decay_s` is the RT60 at mid frequencies: every loop element of length `L` samples is attenuated by
//!   `10^(−3·L / (rate·decay_s))`, so the level falls 60 dB in `decay_s` whatever the path taken;
//! - `damping` shortens the decay above ≈ 5 kHz with a first-order shelf in every loop: the high-frequency decay
//!   time is `decay_s · (1 − 0.85·damping)`;
//! - `size` scales the loop lengths (FDN 0.6–1.4×, plate 0.5–1.25×); the plate also shrinks so the sound goes
//!   around its tank at least five times within `decay_s`, otherwise short decays would be a few sparse echoes:
//!   below about 3 s the decay sets the plate's size and `size` is only an upper bound. Under about 1 s the plate
//!   is grainy by construction (its tank allpasses show as a fast flutter): a room is the better short space;
//! - `modulation` moves some delays slowly (4 of the 16 FDN lines, the two tank allpasses of the plate; up to
//!   ±1 ms, 0.5–1.3 Hz) to break up the modes. The oscillators are recursive sine generators with fixed starting
//!   phases, restarted by [`Reverb::reset`]: no random source, the same input gives the same output;
//! - pre-delay (0–200 ms), then at the output a Butterworth high-pass and low-pass per channel. 100% wet;
//! - the output gain follows `sqrt(scale) · decay_s^−0.25`, so the return level grows gently with the decay
//!   (≈ +0.75 dB per doubling) instead of 3 dB.
//!
//! Smoothing: gains, damping, pre-delay and level with a one-pole (30 ms), loop lengths with a slower one
//! (120 ms) that lands exactly on whole samples, so a settled reverb reads its delays without interpolation.
//! Parameters may change every 32 samples without clicks, and the result does not depend on the block size.
//! The algorithm is fixed at construction: changing it is a structural change of the graph.
//!
//! Properties verified by the tests (FX-9): decay time within ±10% of `decay_s`, dense tail (no periodicity in
//! its autocorrelation), decorrelated left and right, shorter highs with damping, pre-delay respected, high-pass
//! and low-pass effective, calibration levels, exact silence after the tail, determinism, block-size invariance,
//! stability under changes every 32 samples.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};
use crate::pitch::PitchShifter;
use crate::util::{flush, one_pole};

const MAX_PREDELAY_MS: f64 = 200.0;
const SMOOTH_MS: f64 = 30.0;
const SIZE_SMOOTH_MS: f64 = 120.0;
/// Corner of the damping shelf.
const DAMP_HZ: f64 = 5000.0;
/// Largest modulation excursion, in milliseconds.
const MOD_MS: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Plate,
    Room,
    Hall,
    /// The hall with a pitch-shifted feedback (FX-22).
    Shimmer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReverbParams {
    /// RT60 at mid frequencies, 0.1..10 s
    pub decay_s: f64,
    /// 0..1
    pub size: f64,
    /// 0..1
    pub damping: f64,
    /// 0..1
    pub modulation: f64,
    /// 0..200 ms
    pub predelay_ms: f64,
    /// 20..1000 Hz
    pub hpf_hz: f64,
    /// 1000..20000 Hz
    pub lpf_hz: f64,
    /// Shimmer only: interval of the shifted feedback in semitones (5, 7 or 12). Ignored by the other algorithms.
    pub shimmer_semitones: f64,
    /// Shimmer only: share of shifted signal in the feedback, 0..1. Ignored by the other algorithms.
    pub shimmer_amount: f64,
}

impl Default for ReverbParams {
    fn default() -> Self {
        ReverbParams {
            decay_s: 1.5,
            size: 0.5,
            damping: 0.5,
            modulation: 0.2,
            predelay_ms: 0.0,
            hpf_hz: 20.0,
            lpf_hz: 20000.0,
            shimmer_semitones: 12.0,
            shimmer_amount: 0.0,
        }
    }
}

/// A delay line read before it is written: `tap(d)` is the sample written `d` writes ago (`1 ≤ d ≤ max`).
#[derive(Debug, Clone)]
struct Line {
    buf: Vec<f64>,
    w: usize,
}

impl Line {
    fn new(max: usize) -> Self {
        Line { buf: vec![0.0; max + 4], w: 0 }
    }

    fn clear(&mut self) {
        self.buf.fill(0.0);
        self.w = 0;
    }

    #[inline]
    fn write(&mut self, x: f64) {
        self.buf[self.w] = flush(x);
        self.w += 1;
        if self.w == self.buf.len() {
            self.w = 0;
        }
    }

    #[inline]
    fn tap(&self, d: usize) -> f64 {
        let len = self.buf.len();
        let d = d.clamp(1, len - 1);
        let i = self.w + len - d;
        self.buf[if i >= len { i - len } else { i }]
    }

    /// Fractional delay with linear interpolation; exact (no interpolation) on whole samples.
    #[inline]
    fn tap_f(&self, d: f64) -> f64 {
        let whole = d.floor();
        let frac = d - whole;
        let a = self.tap(whole as usize);
        if frac == 0.0 { a } else { a + (self.tap(whole as usize + 1) - a) * frac }
    }

    /// Schroeder allpass around this line with coefficient `g` and delay `d`.
    #[inline]
    fn allpass(&mut self, x: f64, g: f64, d: f64) -> f64 {
        let delayed = self.tap_f(d);
        let v = x - g * delayed;
        self.write(v);
        delayed + g * v
    }
}

/// Recursive sine oscillator ("magic circle"): two multiplications per sample, stable, deterministic.
#[derive(Debug, Clone, Copy)]
struct Lfo {
    s: f64,
    c: f64,
    eps: f64,
    phase: f64,
}

impl Lfo {
    fn new(hz: f64, phase: f64, rate: f64) -> Self {
        let mut l = Lfo { s: 0.0, c: 0.0, eps: 2.0 * (std::f64::consts::PI * hz / rate).sin(), phase };
        l.reset();
        l
    }

    fn reset(&mut self) {
        self.s = self.phase.sin();
        self.c = self.phase.cos();
    }

    #[inline]
    fn tick(&mut self) -> f64 {
        self.s += self.eps * self.c;
        self.c -= self.eps * self.s;
        self.s
    }
}

/// Per-length attenuation for an RT60 of `decay_s`, and the extra high-frequency gain of the damping shelf.
#[inline]
fn loop_gains(len: f64, rate: f64, decay_s: f64, damping: f64) -> (f64, f64) {
    let g = 10f64.powf(-3.0 * len / (rate * decay_s));
    let ratio = 1.0 - 0.85 * damping;
    (g, g.powf(1.0 / ratio - 1.0))
}

/// Moves a length toward a whole-sample target; lands on it exactly.
#[inline]
fn slew(cur: &mut f64, target: f64, k: f64) {
    if *cur != target {
        *cur += k * (target - *cur);
        if (*cur - target).abs() < 1e-3 {
            *cur = target;
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Plate (Dattorro). Lengths in samples at the paper's 29761 Hz.

const PLATE_RATE: f64 = 29761.0;
const PLATE_IN: [(f64, f64); 4] = [(142.0, 0.75), (107.0, 0.75), (379.0, 0.625), (277.0, 0.625)];
/// Tank elements in loop order: half A (modulated allpass, delay, allpass, delay), then half B.
const PLATE_TANK: [f64; 8] = [672.0, 4453.0, 1800.0, 3720.0, 908.0, 4217.0, 2656.0, 3163.0];
const PLATE_LOOP: f64 = 21589.0 / PLATE_RATE;
const PLATE_DIFF_1: f64 = 0.7;
const PLATE_DIFF_2: f64 = 0.5;
const PLATE_BANDWIDTH: f64 = 0.9995;
/// Measured: energy takes this much longer than the sum of the tank lengths to go around once.
const PLATE_TRIP: f64 = 1.09;
/// Longest trip around the tank, as a fraction of the decay time.
const PLATE_TRIPS: f64 = 0.2;
/// Output taps: (tank element, position at 29761 Hz, sign).
const PLATE_LEFT: [(usize, f64, f64); 7] = [
    (5, 266.0, 1.0),
    (5, 2974.0, 1.0),
    (7, 2812.0, -1.0),
    (7, 1996.0, 1.0),
    (1, 1990.0, -1.0),
    (3, 187.0, -1.0),
    (3, 1066.0, -1.0),
];
const PLATE_RIGHT: [(usize, f64, f64); 7] = [
    (1, 353.0, 1.0),
    (1, 3627.0, 1.0),
    (3, 1228.0, -1.0),
    (3, 2673.0, 1.0),
    (5, 2111.0, -1.0),
    (7, 335.0, -1.0),
    (7, 121.0, -1.0),
];

#[derive(Debug, Clone)]
struct Plate {
    k: f64,
    input: [Line; 4],
    tank: [Line; 8],
    len: [f64; 8],
    target: [f64; 8],
    bandwidth: f64,
    lp: [f64; 2],
    lfo: [Lfo; 2],
    g: f64,
    g_target: f64,
    h: f64,
    h_target: f64,
    depth: f64,
    depth_target: f64,
    scale: f64,
}

impl Plate {
    fn new(rate: f64) -> Self {
        let k = rate / PLATE_RATE;
        let max = |n: f64| (n * k * 1.25).ceil() as usize + (MOD_MS * 0.001 * rate).ceil() as usize + 4;
        Plate {
            k,
            input: PLATE_IN.map(|(n, _)| Line::new(max(n))),
            tank: PLATE_TANK.map(|n| Line::new(max(n))),
            len: [0.0; 8],
            target: [0.0; 8],
            bandwidth: 0.0,
            lp: [0.0; 2],
            lfo: [Lfo::new(0.83, 0.0, rate), Lfo::new(1.19, 1.9, rate)],
            g: 0.0,
            g_target: 0.0,
            h: 1.0,
            h_target: 1.0,
            depth: 0.0,
            depth_target: 0.0,
            scale: 1.0,
        }
    }

    fn set(&mut self, p: &ReverbParams, rate: f64) {
        // At least five trips around the tank within the decay time.
        let scale = (0.5 + 0.75 * p.size).min(p.decay_s * PLATE_TRIPS / PLATE_LOOP).max(0.15);
        self.scale = scale;
        for i in 0..8 {
            self.target[i] = (PLATE_TANK[i] * self.k * scale).round();
        }
        // Four attenuations per trip: a quarter of the loop each.
        let quarter = PLATE_TRIP * self.target.iter().sum::<f64>() / 4.0;
        let (g, h) = loop_gains(quarter, rate, p.decay_s, p.damping);
        self.g_target = g;
        // The shelf acts once per half (two attenuations).
        self.h_target = h * h;
        // The excursion stays inside the shortest modulated allpass.
        self.depth_target = (p.modulation * MOD_MS * 0.001 * rate).min(self.target[0] * 0.25);
    }

    fn snap(&mut self) {
        self.len = self.target;
        self.g = self.g_target;
        self.h = self.h_target;
        self.depth = self.depth_target;
    }

    fn clear(&mut self) {
        self.input.iter_mut().chain(self.tank.iter_mut()).for_each(Line::clear);
        self.bandwidth = 0.0;
        self.lp = [0.0; 2];
        self.lfo.iter_mut().for_each(Lfo::reset);
    }

    #[inline]
    fn tick(&mut self, x: f64, smooth: f64, size_k: f64, damp_c: f64) -> (f64, f64) {
        self.g += smooth * (self.g_target - self.g);
        self.h += smooth * (self.h_target - self.h);
        self.depth += smooth * (self.depth_target - self.depth);
        for i in 0..8 {
            slew(&mut self.len[i], self.target[i], size_k);
        }
        let len = self.len;

        // Output taps and the feedback reads see the tank as the previous sample left it.
        let tap = |t: &[Line; 8], set: &[(usize, f64, f64); 7]| {
            let mut y = 0.0;
            for &(e, pos, sign) in set {
                y += sign * t[e].tap((pos / PLATE_TANK[e] * len[e]).round() as usize);
            }
            y
        };
        let out = (tap(&self.tank, &PLATE_LEFT), tap(&self.tank, &PLATE_RIGHT));
        let (d1a, d2a) = (self.tank[1].tap_f(len[1]), self.tank[3].tap_f(len[3]));
        let (d1b, d2b) = (self.tank[5].tap_f(len[5]), self.tank[7].tap_f(len[7]));

        self.bandwidth += PLATE_BANDWIDTH * (x - self.bandwidth);
        let mut d = self.bandwidth;
        for (i, (n, g)) in PLATE_IN.iter().enumerate() {
            d = self.input[i].allpass(d, *g, (n * self.k).round());
        }

        let (g, h, depth) = (self.g, self.h, self.depth);
        let half = |o: usize, feed: f64, delayed: f64, lp: &mut f64, lfo: &mut Lfo, tank: &mut [Line; 8]| {
            let y = tank[o].allpass(d + g * feed, -PLATE_DIFF_1, len[o] + depth * lfo.tick());
            tank[o + 1].write(y);
            *lp = flush(*lp + damp_c * (delayed - *lp));
            let u = g * (h * delayed + (1.0 - h) * *lp);
            let y = tank[o + 2].allpass(u, PLATE_DIFF_2, len[o + 2]);
            tank[o + 3].write(y);
        };
        let [lp_a, lp_b] = &mut self.lp;
        let [lfo_a, lfo_b] = &mut self.lfo;
        half(0, d2b, d1a, lp_a, lfo_a, &mut self.tank);
        half(4, d2a, d1b, lp_b, lfo_b, &mut self.tank);
        out
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Feedback delay network (room, hall).

const N: usize = 16;
/// Line lengths in milliseconds at scale 1.
const HALL_MS: [f64; N] =
    [21.1, 23.9, 27.7, 30.7, 34.9, 38.3, 42.1, 46.9, 51.7, 56.3, 61.1, 66.7, 72.7, 79.9, 87.7, 96.7];
const ROOM_MS: [f64; N] = [6.7, 8.3, 9.7, 11.3, 12.7, 14.9, 16.3, 18.1, 20.3, 22.7, 24.7, 27.1, 30.1, 33.1, 36.7, 40.9];
/// Input diffusers per side, milliseconds (the room uses the first two).
const DIFF_MS: [[f64; 4]; 2] = [[4.3, 6.7, 10.1, 13.7], [4.9, 7.3, 11.3, 14.9]];
const DIFF_G: f64 = 0.55;
/// Early reflections per side: milliseconds at scale 1 for the room (the hall is 2.4 times later) and gains.
const ER_MS: [[f64; 6]; 2] = [[3.1, 5.3, 8.9, 12.7, 17.9, 23.3], [3.7, 6.1, 9.7, 13.9, 19.1, 25.1]];
const ER_GAIN: [f64; 6] = [1.0, -0.8, 0.7, -0.55, 0.45, -0.35];
const HALL_ER_SCALE: f64 = 2.4;
/// Lines whose length is modulated, with the rate of their oscillator.
const MOD_LINES: [(usize, f64, f64); 4] = [(3, 0.53, 0.0), (7, 0.79, 1.3), (11, 1.07, 2.9), (15, 1.31, 4.4)];

/// Sign of entry `i` in row `r` of the Hadamard matrix.
const fn walsh(r: usize, i: usize) -> f64 {
    if (r & i).count_ones().is_multiple_of(2) { 1.0 } else { -1.0 }
}

/// Output signs of line `i` for the left side: a bent sequence, whose Hadamard transform is flat, so no single
/// line comes back into the output more than the others (a row of the matrix would return one line whole).
const fn out_left(i: usize) -> f64 {
    if ((i & 1) * ((i >> 1) & 1) + ((i >> 2) & 1) * ((i >> 3) & 1)).is_multiple_of(2) { 1.0 } else { -1.0 }
}

/// Right side: the left sequence times a Hadamard row, so it is bent too and orthogonal to the left one.
const fn out_right(i: usize) -> f64 {
    out_left(i) * walsh(6, i)
}

/// Input signs: bent as well, and orthogonal to both output sequences.
const fn in_sign(i: usize) -> f64 {
    out_left(i) * walsh(9, i)
}

/// In-place Hadamard transform of 16 values, scaled to be orthonormal.
#[inline]
fn hadamard(v: &mut [f64; N]) {
    let mut h = 1;
    while h < N {
        let mut i = 0;
        while i < N {
            for j in i..i + h {
                let (a, b) = (v[j], v[j + h]);
                v[j] = a + b;
                v[j + h] = a - b;
            }
            i += 2 * h;
        }
        h *= 2;
    }
    for x in v.iter_mut() {
        *x *= 0.25;
    }
}

#[derive(Debug, Clone)]
struct Fdn {
    hall: bool,
    base: [f64; N],
    lines: Vec<Line>,
    len: [f64; N],
    target: [f64; N],
    g: [f64; N],
    g_target: [f64; N],
    h: [f64; N],
    h_target: [f64; N],
    lp: [f64; N],
    diff: [Vec<Line>; 2],
    diff_len: [[f64; 4]; 2],
    early: [Line; 2],
    er_scale: f64,
    er_target: f64,
    er_level: f64,
    lfo: [Lfo; 4],
    depth: f64,
    depth_target: f64,
    scale: f64,
    shim: Option<Box<Shimmer>>,
}

impl Fdn {
    fn new(rate: f64, hall: bool) -> Self {
        let base = if hall { HALL_MS } else { ROOM_MS };
        let ms = |v: f64| v * 0.001 * rate;
        let stages = if hall { 4 } else { 2 };
        let er_max = ms(ER_MS[1][5] * if hall { HALL_ER_SCALE } else { 1.0 } * 1.4).ceil() as usize + 4;
        Fdn {
            hall,
            base,
            lines: base.iter().map(|b| Line::new(ms(b * 1.4 + MOD_MS).ceil() as usize + 4)).collect(),
            len: [0.0; N],
            target: [0.0; N],
            g: [0.0; N],
            g_target: [0.0; N],
            h: [1.0; N],
            h_target: [1.0; N],
            lp: [0.0; N],
            diff: [0, 1].map(|c| (0..stages).map(|i| Line::new(ms(DIFF_MS[c][i]).ceil() as usize + 4)).collect()),
            diff_len: [0, 1].map(|c| DIFF_MS[c].map(|v| ms(v).round())),
            early: [Line::new(er_max), Line::new(er_max)],
            er_scale: 1.0,
            er_target: 1.0,
            er_level: if hall { 0.3 } else { 0.5 },
            lfo: MOD_LINES.map(|(_, hz, phase)| Lfo::new(hz, phase, rate)),
            depth: 0.0,
            depth_target: 0.0,
            scale: 1.0,
            shim: None,
        }
    }

    /// The hall with the shifted feedback.
    fn new_shimmer(rate: f64) -> Self {
        let mut f = Fdn::new(rate, true);
        f.shim = Some(Box::new(Shimmer::new(rate)));
        f
    }

    fn set(&mut self, p: &ReverbParams, rate: f64) {
        let scale = 0.6 + 0.8 * p.size;
        self.scale = scale;
        self.er_target = scale * if self.hall { HALL_ER_SCALE } else { 1.0 };
        for i in 0..N {
            self.target[i] = (self.base[i] * 0.001 * rate * scale).round();
            let (g, h) = loop_gains(self.target[i], rate, p.decay_s, p.damping);
            self.g_target[i] = g;
            self.h_target[i] = h;
        }
        self.depth_target = p.modulation * MOD_MS * 0.001 * rate;
        if let Some(s) = &mut self.shim {
            s.set(p);
        }
    }

    fn snap(&mut self) {
        if let Some(s) = &mut self.shim {
            s.snap();
        }
        self.len = self.target;
        self.g = self.g_target;
        self.h = self.h_target;
        self.depth = self.depth_target;
        self.er_scale = self.er_target;
    }

    fn clear(&mut self) {
        self.lines.iter_mut().for_each(Line::clear);
        self.diff.iter_mut().flatten().for_each(Line::clear);
        self.early.iter_mut().for_each(Line::clear);
        self.lp = [0.0; N];
        self.lfo.iter_mut().for_each(Lfo::reset);
        if let Some(s) = &mut self.shim {
            s.clear();
        }
    }

    #[inline]
    fn tick(&mut self, x: [f64; 2], smooth: f64, size_k: f64, damp_c: f64, rate: f64) -> (f64, f64) {
        self.depth += smooth * (self.depth_target - self.depth);
        self.er_scale += size_k * (self.er_target - self.er_scale);

        let mut o = [0.0f64; N];
        for i in 0..N {
            slew(&mut self.len[i], self.target[i], size_k);
            o[i] = self.lines[i].tap_f(self.len[i]);
        }
        for (m, &(i, _, _)) in MOD_LINES.iter().enumerate() {
            let wobble = self.depth * (1.0 + self.lfo[m].tick());
            if wobble != 0.0 {
                o[i] = self.lines[i].tap_f(self.len[i] + wobble);
            }
        }
        let (mut tail_l, mut tail_r) = (0.0, 0.0);
        for i in 0..N {
            tail_l += out_left(i) * o[i];
            tail_r += out_right(i) * o[i];
        }

        // Early reflections from the dry input, then diffusion toward the network.
        let ms = 0.001 * rate * self.er_scale;
        let mut early = [0.0f64; 2];
        let mut feed = [0.0f64; 2];
        for c in 0..2 {
            for t in 0..6 {
                early[c] += ER_GAIN[t] * self.early[c].tap((ER_MS[c][t] * ms).round() as usize);
            }
            self.early[c].write(x[c]);
            let mut d = x[c];
            for (s, line) in self.diff[c].iter_mut().enumerate() {
                d = line.allpass(d, DIFF_G, self.diff_len[c][s]);
            }
            feed[c] = d;
        }

        let mut f = [0.0f64; N];
        for i in 0..N {
            self.g[i] += smooth * (self.g_target[i] - self.g[i]);
            self.h[i] += smooth * (self.h_target[i] - self.h[i]);
            self.lp[i] = flush(self.lp[i] + damp_c * (o[i] - self.lp[i]));
            f[i] = self.g[i] * (self.h[i] * o[i] + (1.0 - self.h[i]) * self.lp[i]);
        }
        if let Some(s) = &mut self.shim {
            // The tail the network is about to give back, shifted and spread over the lines with a sign
            // sequence of unit norm: |shifted| ≤ |o|, so the loop gain does not exceed the hall's.
            let tail = 0.125 * (tail_l + tail_r);
            let shifted = s.tick(tail, smooth);
            let a = 0.5 * s.amount;
            for i in 0..N {
                f[i] = f[i] * (1.0 - a) + a * self.g[i] * shifted * 0.25 * in_sign(i);
            }
        }
        hadamard(&mut f);
        for i in 0..N {
            self.lines[i].write(f[i] + in_sign(i) * feed[i % 2] * 0.5);
        }
        (tail_l * 0.25 + early[0] * self.er_level, tail_r * 0.25 + early[1] * self.er_level)
    }
}

/// The shifted feedback of the shimmer: shifter, interval and amount (both smoothed).
#[derive(Debug, Clone)]
struct Shimmer {
    shifter: PitchShifter,
    amount: f64,
    amount_target: f64,
}

impl Shimmer {
    fn new(rate: f64) -> Self {
        Shimmer { shifter: PitchShifter::new(rate, SHIMMER_WINDOW_MS), amount: 0.0, amount_target: 0.0 }
    }

    fn set(&mut self, p: &ReverbParams) {
        self.shifter.set_ratio(2f64.powf(p.shimmer_semitones.clamp(0.0, 12.0) / 12.0));
        self.amount_target = p.shimmer_amount.clamp(0.0, 1.0);
    }

    fn snap(&mut self) {
        self.amount = self.amount_target;
        self.shifter.snap_ratio();
    }

    fn clear(&mut self) {
        self.shifter.reset();
    }

    #[inline]
    fn tick(&mut self, x: f64, smooth: f64) -> f64 {
        self.amount += smooth * (self.amount_target - self.amount);
        if (self.amount_target - self.amount).abs() < 1e-9 {
            self.amount = self.amount_target;
        }
        self.shifter.tick(x)
    }
}

/// Window of the shimmer's pitch shifter.
const SHIMMER_WINDOW_MS: f64 = 20.0;

// ---------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Core {
    Plate(Box<Plate>),
    Fdn(Box<Fdn>),
}

#[derive(Debug, Clone)]
pub struct Reverb {
    rate: f64,
    algorithm: Algorithm,
    core: Core,
    predelay: [Line; 2],
    hpf: Biquad,
    lpf: Biquad,
    hpf_hz: f64,
    lpf_hz: f64,
    smooth: f64,
    size_k: f64,
    damp_c: f64,
    delay: f64,
    delay_target: f64,
    level: f64,
    level_target: f64,
    initialized: bool,
}

impl Reverb {
    /// Allocates everything the algorithm needs, including the pre-delay buffer for 200 ms.
    pub fn new(rate: f64, algorithm: Algorithm) -> Self {
        let pre = (MAX_PREDELAY_MS * 0.001 * rate).ceil() as usize + 4;
        let mut r = Reverb {
            rate,
            algorithm,
            core: match algorithm {
                Algorithm::Plate => Core::Plate(Box::new(Plate::new(rate))),
                Algorithm::Room => Core::Fdn(Box::new(Fdn::new(rate, false))),
                Algorithm::Hall => Core::Fdn(Box::new(Fdn::new(rate, true))),
                Algorithm::Shimmer => Core::Fdn(Box::new(Fdn::new_shimmer(rate))),
            },
            predelay: [Line::new(pre), Line::new(pre)],
            hpf: Biquad::default(),
            lpf: Biquad::default(),
            hpf_hz: -1.0,
            lpf_hz: -1.0,
            smooth: one_pole(SMOOTH_MS, rate),
            size_k: one_pole(SIZE_SMOOTH_MS, rate),
            damp_c: 1.0 - (-2.0 * std::f64::consts::PI * DAMP_HZ / rate).exp(),
            delay: 0.0,
            delay_target: 0.0,
            level: 0.0,
            level_target: 0.0,
            initialized: false,
        };
        r.set_params(&ReverbParams::default());
        r.initialized = false;
        r
    }

    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    /// No allocation; may be called every 32 samples.
    pub fn set_params(&mut self, p: &ReverbParams) {
        let p = ReverbParams {
            decay_s: p.decay_s.clamp(0.1, 10.0),
            size: p.size.clamp(0.0, 1.0),
            damping: p.damping.clamp(0.0, 1.0),
            modulation: p.modulation.clamp(0.0, 1.0),
            ..*p
        };
        let (scale, gain) = match &mut self.core {
            Core::Plate(c) => {
                c.set(&p, self.rate);
                (c.scale, PLATE_GAIN)
            }
            Core::Fdn(c) => {
                c.set(&p, self.rate);
                (c.scale, if c.hall { HALL_GAIN } else { ROOM_GAIN })
            }
        };
        self.level_target = gain * scale.sqrt() * p.decay_s.powf(-0.25);
        self.delay_target = p.predelay_ms.clamp(0.0, MAX_PREDELAY_MS) * 0.001 * self.rate;
        let hz = p.hpf_hz.clamp(20.0, 1000.0);
        if hz != self.hpf_hz {
            self.hpf_hz = hz;
            self.hpf.c = Coeffs::design(Kind::HighPass, hz, 0.0, BUTTERWORTH_Q, self.rate);
        }
        let hz = p.lpf_hz.clamp(1000.0, 20000.0).min(0.45 * self.rate);
        if hz != self.lpf_hz {
            self.lpf_hz = hz;
            self.lpf.c = Coeffs::design(Kind::LowPass, hz, 0.0, BUTTERWORTH_Q, self.rate);
        }
        if !self.initialized {
            self.snap();
            self.initialized = true;
        }
    }

    fn snap(&mut self) {
        self.delay = self.delay_target;
        self.level = self.level_target;
        match &mut self.core {
            Core::Plate(c) => c.snap(),
            Core::Fdn(c) => c.snap(),
        }
    }

    /// Clears the state and restarts the modulation oscillators: what follows is what a new reverb would give.
    pub fn reset(&mut self) {
        match &mut self.core {
            Core::Plate(c) => c.clear(),
            Core::Fdn(c) => c.clear(),
        }
        self.predelay.iter_mut().for_each(Line::clear);
        self.hpf.reset();
        self.lpf.reset();
        self.snap();
    }

    /// 100% wet. Stereo send bus in, stereo return out; all four slices have the same length.
    pub fn process(&mut self, in_l: &[f32], in_r: &[f32], out_l: &mut [f32], out_r: &mut [f32]) {
        let n = in_l.len().min(in_r.len()).min(out_l.len()).min(out_r.len());
        for i in 0..n {
            self.delay += self.smooth * (self.delay_target - self.delay);
            if (self.delay_target - self.delay).abs() < 1e-9 {
                self.delay = self.delay_target;
            }
            self.level += self.smooth * (self.level_target - self.level);

            // The pre-delay line is read one sample late, so a delay of 0 is the input itself.
            let x = [f64::from(in_l[i]), f64::from(in_r[i])];
            let mut pre = x;
            for c in 0..2 {
                self.predelay[c].write(x[c]);
                pre[c] = self.predelay[c].tap_f(self.delay + 1.0);
            }

            let (l, r) = match &mut self.core {
                Core::Plate(c) => c.tick((pre[0] + pre[1]) * 0.5, self.smooth, self.size_k, self.damp_c),
                Core::Fdn(c) => c.tick(pre, self.smooth, self.size_k, self.damp_c, self.rate),
            };
            let l = self.lpf.tick(0, self.hpf.tick(0, l * self.level));
            let r = self.lpf.tick(1, self.hpf.tick(1, r * self.level));
            out_l[i] = l as f32;
            out_r[i] = r as f32;
        }
    }
}

/// Output gains chosen so the pink-noise return levels match the calibration table of the tests.
const PLATE_GAIN: f64 = 1.445;
const ROOM_GAIN: f64 = 1.175;
const HALL_GAIN: f64 = 1.758;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::revmeasure::{band, correlation, periodicity, rt60};
    use crate::testsignal::{render, xorshift32};
    use serde_json::json;

    const FS: u32 = 48000;
    const ALL: [Algorithm; 4] = [Algorithm::Plate, Algorithm::Room, Algorithm::Hall, Algorithm::Shimmer];

    fn params(decay: f64) -> ReverbParams {
        ReverbParams { decay_s: decay, damping: 0.0, modulation: 0.0, ..Default::default() }
    }

    fn run(a: Algorithm, p: &ReverbParams, l: &[f32], r: &[f32], block: usize) -> (Vec<f32>, Vec<f32>) {
        let mut rv = Reverb::new(f64::from(FS), a);
        rv.set_params(p);
        let (mut ol, mut or) = (vec![0f32; l.len()], vec![0f32; l.len()]);
        let mut i = 0;
        while i < l.len() {
            let e = (i + block).min(l.len());
            rv.process(&l[i..e], &r[i..e], &mut ol[i..e], &mut or[i..e]);
            i = e;
        }
        (ol, or)
    }

    fn impulse(a: Algorithm, p: &ReverbParams, secs: f64) -> (Vec<f32>, Vec<f32>) {
        let mut l = vec![0f32; (secs * f64::from(FS)) as usize];
        l[0] = 1.0;
        run(a, p, &l, &l, 512)
    }

    fn pink_stereo(secs: f64) -> (Vec<f32>, Vec<f32>) {
        let sig = |seed: u32| render(&json!({"kind":"pink","seed":seed,"amp":0.5,"secs":secs}), FS).0;
        (sig(1), sig(2))
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
    }

    fn mid_rt60(ir: &[f32]) -> f64 {
        let rate = f64::from(FS);
        rt60(&band(ir, 500.0, 2000.0, rate), rate).expect("the tail falls 35 dB within the capture")
    }

    /// req: FX-9
    #[test]
    fn decay_time_is_the_one_set() {
        // ±10% (FX-9) with no damping and no modulation, where the decay is the same at every frequency; the
        // Schroeder T30 estimate on 500 Hz–2 kHz is itself good to a few percent. Measured: within 3% for the
        // network, within 5% for the plate.
        for a in ALL {
            for decay in [0.5, 1.5, 4.0] {
                let got = mid_rt60(&impulse(a, &params(decay), decay * 2.0 + 1.0).0);
                assert!((got / decay - 1.0).abs() <= 0.10, "{a:?} {decay}: {got:.2}");
            }
        }
        // With the default damping and modulation the mid band loses a little to the shelf and to the
        // interpolated reads: ±15%.
        for a in ALL {
            let p = ReverbParams { decay_s: 2.0, ..Default::default() };
            let got = mid_rt60(&impulse(a, &p, 5.0).0);
            assert!((got / 2.0 - 1.0).abs() <= 0.15, "{a:?}: {got:.2}");
        }
    }

    /// req: FX-9
    #[test]
    fn the_tail_has_no_repeating_period() {
        // `repeat` is the autocorrelation that comes back at a lag and again at twice that lag (2–150 ms): what a
        // comb leaves in a tail. The comb reverb this one replaces measured 0.16–0.20 at 2 s and 0.14 at 4.6 s;
        // these measure 0.04–0.08. The threshold sits between the two. Under 1 s the plate is above it by
        // construction (see the module documentation), so it is measured where a plate is used.
        let rate = f64::from(FS);
        for a in ALL {
            for decay in [2.0, 4.0] {
                let (ir, _) = impulse(a, &params(decay), 1.0);
                let (_, repeat) = periodicity(&ir, rate, 0.1, 0.5, 0.002, 0.15);
                assert!(repeat < 0.11, "{a:?} {decay}: {repeat:.3}");
            }
        }
        // The room is the short space: measured at 0.6 s too, on the part of the tail that is still above the noise.
        let (ir, _) = impulse(Algorithm::Room, &params(0.6), 1.0);
        let (_, repeat) = periodicity(&ir, rate, 0.1, 0.4, 0.002, 0.15);
        assert!(repeat < 0.11, "room 0.6: {repeat:.3}");
    }

    /// req: FX-9
    #[test]
    fn left_and_right_tails_are_decorrelated_with_a_mono_send() {
        // |correlation| < 0.3 (FX-9); measured below 0.1.
        let rate = f64::from(FS);
        for a in ALL {
            let (l, r) = impulse(a, &ReverbParams { decay_s: 2.0, ..Default::default() }, 2.5);
            let c = correlation(&l, &r, rate, 0.1, 2.0);
            assert!(c.abs() < 0.3, "{a:?}: {c:.3}");
        }
    }

    /// req: FX-9
    #[test]
    fn damping_shortens_the_highs() {
        // At damping 0.5 the decay above the shelf is 57.5% of the mid one; 8–16 kHz must fall at least 20% faster
        // than 500 Hz–2 kHz, and with no damping within 10% of it.
        let rate = f64::from(FS);
        for a in ALL {
            let hi = |damping: f64| {
                let p = ReverbParams { decay_s: 2.0, damping, modulation: 0.0, ..Default::default() };
                let (ir, _) = impulse(a, &p, 5.0);
                rt60(&band(&ir, 8000.0, 16000.0, rate), rate).unwrap() / mid_rt60(&ir)
            };
            let (flat, damped) = (hi(0.0), hi(0.5));
            assert!((flat - 1.0).abs() <= 0.1, "{a:?} flat: {flat:.2}");
            assert!(damped < 0.8, "{a:?} damped: {damped:.2}");
        }
    }

    /// req: FX-5
    #[test]
    fn return_levels_match_the_calibration() {
        // Stereo pink noise as the send: the return sits 2 dB above the send at 1.5 s for the three algorithms
        // (±1.5 dB: level estimates on noise vary by a few tenths, and early reflections weigh differently), and
        // a longer decay is louder, gently.
        let (l, r) = pink_stereo(8.0);
        let skip = 4 * FS as usize;
        let send = (rms(&l[skip..]) + rms(&r[skip..])) / 2.0;
        for a in ALL {
            let level = |decay: f64| {
                let (ol, or) = run(a, &ReverbParams { decay_s: decay, ..Default::default() }, &l, &r, 512);
                20.0 * ((rms(&ol[skip..]) + rms(&or[skip..])) / 2.0 / send).log10()
            };
            let (short, mid, long) = (level(0.5), level(1.5), level(4.0));
            assert!((mid - 2.0).abs() <= 1.5, "{a:?}: {mid:.2} dB");
            assert!(long > mid && long - short < 6.0, "{a:?}: {short:.2} {mid:.2} {long:.2}");
        }
    }

    /// req: FX-9
    #[test]
    fn silence_in_gives_exact_silence_after_the_tail() {
        // Half a second of noise, then silence; at 0.3 s of decay the state is below the flush threshold
        // (−600 dB) well within 14 s.
        let n = 16 * FS as usize;
        let mut l = vec![0f32; n];
        for (i, v) in xorshift32(7, FS as usize / 2).iter().enumerate() {
            l[i] = (*v as f32) * 0.5;
        }
        for a in ALL {
            let (ol, or) = run(a, &ReverbParams { decay_s: 0.3, ..Default::default() }, &l, &l, 1024);
            let tail = 14 * FS as usize;
            assert!(ol[tail..].iter().chain(or[tail..].iter()).all(|v| *v == 0.0), "{a:?}");
            assert!(ol[..FS as usize].iter().any(|v| *v != 0.0), "{a:?}");
        }
    }

    /// req: FX-5
    #[test]
    fn predelay_delays_the_first_output() {
        let mut l = vec![0f32; FS as usize / 2];
        l[0] = 1.0;
        for a in ALL {
            let first = |pre: f64| {
                let p = ReverbParams { predelay_ms: pre, ..Default::default() };
                let (ol, or) = run(a, &p, &l, &l, 128);
                ol.iter().position(|v| *v != 0.0).unwrap().min(or.iter().position(|v| *v != 0.0).unwrap())
            };
            // 50 ms are exactly 2400 samples, added to whatever the algorithm takes to speak.
            assert_eq!(first(50.0), first(0.0) + 2400, "{a:?}");
        }
    }

    /// req: FX-5
    #[test]
    fn high_pass_and_low_pass_shape_the_return() {
        // 50 Hz against a 2nd-order high-pass at 250 Hz, 12 kHz against a low-pass at 2 kHz: ≈ 28 dB and ≈ 31 dB
        // in theory; require ≥ 15 dB.
        let sine = |hz: f64| -> Vec<f32> {
            (0..4 * FS as usize)
                .map(|i| (0.5 * (2.0 * std::f64::consts::PI * hz * i as f64 / 48000.0).sin()) as f32)
                .collect()
        };
        let skip = 2 * FS as usize;
        for a in ALL {
            let level = |l: &[f32], hpf: f64, lpf: f64| {
                let p = ReverbParams { hpf_hz: hpf, lpf_hz: lpf, damping: 0.0, ..Default::default() };
                rms(&run(a, &p, l, l, 512).0[skip..])
            };
            let low = sine(50.0);
            let cut = 20.0 * (level(&low, 20.0, 20000.0) / level(&low, 250.0, 20000.0)).log10();
            assert!(cut >= 15.0, "{a:?} high-pass: {cut:.1}");
            let high = sine(12000.0);
            let cut = 20.0 * (level(&high, 20.0, 20000.0) / level(&high, 20.0, 2000.0)).log10();
            assert!(cut >= 15.0, "{a:?} low-pass: {cut:.1}");
        }
    }

    /// req: FX-9
    #[test]
    fn full_scale_noise_at_the_longest_decay_stays_finite() {
        let n = 10 * FS as usize;
        let l: Vec<f32> = xorshift32(3, n).iter().map(|v| *v as f32).collect();
        let r: Vec<f32> = xorshift32(4, n).iter().map(|v| *v as f32).collect();
        let p = ReverbParams { decay_s: 10.0, size: 1.0, damping: 0.0, modulation: 1.0, ..Default::default() };
        for a in ALL {
            let (ol, or) = run(a, &p, &l, &r, 512);
            // Every loop gain is below 1, so the return is bounded; 100 is a generous bound (≈ +40 dB).
            assert!(ol.iter().chain(or.iter()).all(|v| v.is_finite() && v.abs() < 100.0), "{a:?}");
        }
    }

    /// req: FX-9, ENG-4
    #[test]
    fn block_size_does_not_change_the_output() {
        let (l, r) = pink_stereo(1.5);
        let p = ReverbParams {
            decay_s: 2.5,
            size: 0.7,
            damping: 0.3,
            modulation: 0.6,
            predelay_ms: 20.0,
            ..Default::default()
        };
        for a in ALL {
            let (x, _) = run(a, &p, &l, &r, 64);
            let (y, _) = run(a, &p, &l, &r, 4096);
            // Same per-sample arithmetic, modulation included: the two renders are the same samples.
            assert!(x == y, "{a:?}");
        }
    }

    /// req: FX-9
    #[test]
    fn the_modulation_is_deterministic_and_restarts_with_reset() {
        let (l, r) = pink_stereo(1.0);
        let p = ReverbParams { modulation: 1.0, ..Default::default() };
        for a in ALL {
            let mut rv = Reverb::new(f64::from(FS), a);
            rv.set_params(&p);
            let go = |rv: &mut Reverb| {
                let (mut ol, mut or) = (vec![0f32; l.len()], vec![0f32; l.len()]);
                rv.process(&l, &r, &mut ol, &mut or);
                (ol, or)
            };
            let first = go(&mut rv);
            assert!(first != go(&mut rv), "{a:?}: the second pass starts on the first one's tail");
            rv.reset();
            assert!(first == go(&mut rv), "{a:?}: after a reset the reverb is as new");
            assert!(first == run(a, &p, &l, &r, l.len()), "{a:?}: two reverbs give the same samples");
            // And the modulation does something.
            assert!(first != run(a, &ReverbParams { modulation: 0.0, ..p }, &l, &r, l.len()), "{a:?}");
        }
    }

    /// req: FX-9, AUT-6
    #[test]
    fn parameter_changes_every_32_samples_stay_bounded() {
        let (l, r) = pink_stereo(3.0);
        for a in ALL {
            let mut rv = Reverb::new(f64::from(FS), a);
            let (mut ol, mut or) = (vec![0f32; l.len()], vec![0f32; l.len()]);
            let mut seed = 5u32;
            let mut i = 0;
            while i + 32 <= l.len() {
                let u = xorshift32(seed, 7);
                seed += 1;
                rv.set_params(&ReverbParams {
                    decay_s: 0.1 + u[0].abs() * 9.9,
                    size: u[1].abs(),
                    damping: u[2].abs(),
                    modulation: u[3].abs(),
                    predelay_ms: u[4].abs() * 200.0,
                    hpf_hz: 20.0 + u[5].abs() * 980.0,
                    lpf_hz: 1000.0 + u[6].abs() * 19000.0,
                    shimmer_semitones: [5.0, 7.0, 12.0][(u[2].abs() * 2.99) as usize],
                    shimmer_amount: u[1].abs(),
                });
                rv.process(&l[i..i + 32], &r[i..i + 32], &mut ol[i..i + 32], &mut or[i..i + 32]);
                i += 32;
            }
            assert!(ol.iter().chain(or.iter()).all(|v| v.is_finite() && v.abs() < 50.0), "{a:?}");
        }
    }

    fn shimmer(decay: f64, semitones: f64, amount: f64) -> ReverbParams {
        ReverbParams {
            decay_s: decay,
            damping: 0.0,
            modulation: 0.3,
            shimmer_semitones: semitones,
            shimmer_amount: amount,
            ..Default::default()
        }
    }

    fn peak(x: &[f32], from: f64, to: f64) -> f32 {
        let (a, b) = ((from * f64::from(FS)) as usize, (to * f64::from(FS)) as usize);
        x[a..b.min(x.len())].iter().fold(0.0f32, |m, v| m.max(v.abs()))
    }

    /// req: FX-22, FX-9
    #[test]
    fn the_shimmer_at_amount_zero_is_the_hall() {
        // The shifted feedback is mixed in with weight a/2: at a = 0 the network is the hall's, sample for sample.
        let (l, r) = pink_stereo(1.5);
        let p = ReverbParams { decay_s: 2.5, size: 0.7, damping: 0.3, modulation: 0.6, ..Default::default() };
        assert!(run(Algorithm::Shimmer, &p, &l, &r, 256) == run(Algorithm::Hall, &p, &l, &r, 256));
    }

    /// req: FX-22
    #[test]
    fn the_shimmer_tail_stays_bounded_and_does_not_grow() {
        // 20 s of tail at 3 s of decay, amount 0.6, intervals 5/7/12: the peak of the second half is below the
        // peak of the 1–3 s window and the tail is far down at the end (at least 40 dB under the peak of the
        // 1–3 s window after 19 s: the decay alone gives 120 dB).
        for st in [5.0, 7.0, 12.0] {
            let (ol, or) = impulse(Algorithm::Shimmer, &shimmer(3.0, st, 0.6), 20.0);
            assert!(ol.iter().chain(or.iter()).all(|v| v.is_finite()), "{st}");
            let early = peak(&ol, 1.0, 3.0).max(peak(&or, 1.0, 3.0));
            let late = peak(&ol, 10.0, 20.0).max(peak(&or, 10.0, 20.0));
            assert!(early > 0.0 && late < early, "{st}: {early} {late}");
            let end = peak(&ol, 19.0, 20.0).max(peak(&or, 19.0, 20.0));
            assert!(end < early * 0.01, "{st}: {end} {early}");
        }
    }

    /// req: FX-22
    #[test]
    fn the_shimmer_tail_ends_in_exact_silence_and_loud_input_stays_finite() {
        let n = 16 * FS as usize;
        let mut l = vec![0f32; n];
        for (i, v) in xorshift32(7, FS as usize / 2).iter().enumerate() {
            l[i] = (*v as f32) * 0.5;
        }
        for st in [5.0, 7.0, 12.0] {
            let (ol, or) = run(Algorithm::Shimmer, &shimmer(0.3, st, 0.6), &l, &l, 1024);
            let tail = 14 * FS as usize;
            assert!(ol[tail..].iter().chain(or[tail..].iter()).all(|v| *v == 0.0), "{st}");
            assert!(ol[..FS as usize].iter().any(|v| *v != 0.0), "{st}");
        }
        // Full-scale noise, longest decay, full amount.
        let m = 10 * FS as usize;
        let a: Vec<f32> = xorshift32(3, m).iter().map(|v| *v as f32).collect();
        let b: Vec<f32> = xorshift32(4, m).iter().map(|v| *v as f32).collect();
        let p = ReverbParams { decay_s: 10.0, size: 1.0, damping: 0.0, modulation: 1.0, ..shimmer(10.0, 12.0, 1.0) };
        let (ol, or) = run(Algorithm::Shimmer, &p, &a, &b, 512);
        assert!(ol.iter().chain(or.iter()).all(|v| v.is_finite() && v.abs() < 100.0));
    }

    /// req: FX-22, ENG-4
    #[test]
    fn the_shimmer_does_not_depend_on_the_block_size_and_restarts_with_reset() {
        let (l, r) = pink_stereo(1.5);
        for st in [5.0, 7.0, 12.0] {
            let p = ReverbParams { predelay_ms: 20.0, ..shimmer(2.5, st, 0.6) };
            let (x, _) = run(Algorithm::Shimmer, &p, &l, &r, 64);
            let (y, _) = run(Algorithm::Shimmer, &p, &l, &r, 4096);
            assert!(x == y, "{st}");
            let mut rv = Reverb::new(f64::from(FS), Algorithm::Shimmer);
            rv.set_params(&p);
            let go = |rv: &mut Reverb| {
                let (mut ol, mut or) = (vec![0f32; l.len()], vec![0f32; l.len()]);
                rv.process(&l, &r, &mut ol, &mut or);
                ol
            };
            let first = go(&mut rv);
            rv.reset();
            assert!(first == go(&mut rv), "{st}: reset");
            assert!(first == x, "{st}: the same samples as a new reverb");
        }
    }

    /// Power of a sine component of `x` at `hz` (Goertzel-style single-bin DFT).
    fn bin_power(x: &[f32], hz: f64) -> f64 {
        let w = 2.0 * std::f64::consts::PI * hz / f64::from(FS);
        let (mut re, mut im) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            re += f64::from(*v) * (w * i as f64).cos();
            im += f64::from(*v) * (w * i as f64).sin();
        }
        (re * re + im * im) / (x.len() * x.len()) as f64
    }

    /// req: FX-22
    #[test]
    fn the_shimmer_puts_energy_an_interval_above_the_input() {
        // A 0.3 s burst at 1 kHz; the tail between 0.5 and 2.5 s holds, at amount 0.6 and +12 st, at least 20 dB
        // more power in the 2 kHz bin (the octave) relative to the 1 kHz one than at amount 0 (measured 34 dB: −57 dB without, −23 dB with).
        let n = 3 * FS as usize;
        let l: Vec<f32> = (0..n)
            .map(|i| {
                if i < 14400 {
                    (0.5 * (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48000.0).sin()) as f32
                } else {
                    0.0
                }
            })
            .collect();
        let ratio = |amount: f64| {
            let (ol, _) = run(Algorithm::Shimmer, &shimmer(3.0, 12.0, amount), &l, &l, 512);
            let seg = &ol[24000..120000];
            10.0 * (bin_power(seg, 2000.0) / bin_power(seg, 1000.0)).log10()
        };
        let (off, on) = (ratio(0.0), ratio(0.6));
        assert!(on - off >= 20.0, "{off:.1} dB, {on:.1} dB");
    }
}
