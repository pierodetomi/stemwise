//! Pitch processors (FX-21, FX-24): a pitch shifter, a two-voice harmonizer and a doubler. No libraries.
//!
//! - **Pitch shifter**: a delay line read by two taps whose delay sweeps over a window of `W` samples. For a
//!   ratio `r` each delay changes by `1 − r` samples per sample, so the read position moves `r` samples per sample
//!   and the pitch is multiplied by `r`. The sweep phase `φ ∈ [0, 1)` gives the delays `W·φ` and `W·frac(φ + ½)`,
//!   and the taps are weighted `½(1 − cos 2πφ)` and `½(1 + cos 2πφ)`: the weights add up to 1 and the tap that jumps
//!   (at `φ = 0` or `φ = ½`) has weight 0 at that moment, so there is no click. At ratio 1 and `φ = 0` the output
//!   is the input delayed by exactly `W/2` samples. Reads interpolate linearly. The ratio is smoothed (10 ms) so
//!   that a changing interval glides instead of clicking.
//! - **Harmonizer**: the mono sum of the input feeds two voices (pitch shifter, level, equal-power pan) added to the
//!   dry signal. The shifter's mean delay `W/2` is a latency; the whole output is late by [`Harmonizer::latency`]
//!   (the mean delay rounded up to a multiple of 32), the dry path by that amount and the voices by the remainder
//!   on top of their own delay, so that voices and dry line up in time.
//! - **Doubler**: one or two copies of the input, detuned by `±detune` cents (pitch shifter with a short window),
//!   delayed by `delay_ms` that drifts slowly (±20% at 0.13–0.21 Hz), panned by `spread`, added to the undelayed
//!   dry signal. Latency 0: the delay is the effect. The drift phase follows the song position (see
//!   [`Doubler::process`]), so the same position always gives the same sound.
//!
//! Everything is f64, deterministic and per-sample, so the output does not depend on the block size; parameters
//! are smoothed per sample (20–30 ms), so changes every 32 samples do not click.

use crate::util::{db_to_gain, flush, one_pole};

const TWO_PI: f64 = std::f64::consts::TAU;
const RATIO_SMOOTH_MS: f64 = 10.0;
const PARAM_SMOOTH_MS: f64 = 20.0;
/// Window of the harmonizer's shifters.
const HARM_WINDOW_MS: f64 = 35.0;
/// Window of the doubler's shifters (mean delay 4 ms: the shortest delay is 5 ms · 0.8).
const DOUBLER_WINDOW_MS: f64 = 8.0;
/// Latencies are multiples of this.
const LATENCY_STEP: usize = 32;

/// A delay line written then read: `read(d)` is the sample written `d` writes ago, `d = 0` the one just written.
#[derive(Debug, Clone)]
struct Ring {
    buf: Vec<f64>,
    w: usize,
}

impl Ring {
    fn new(max_delay: usize) -> Self {
        Ring { buf: vec![0.0; max_delay + 4], w: 0 }
    }

    fn clear(&mut self) {
        self.buf.fill(0.0);
        self.w = 0;
    }

    /// Writes `x` (the newest sample, delay 0).
    #[inline]
    fn write(&mut self, x: f64) {
        self.w += 1;
        if self.w == self.buf.len() {
            self.w = 0;
        }
        self.buf[self.w] = flush(x);
    }

    /// Linear interpolation; `0 ≤ d ≤ max_delay`.
    #[inline]
    fn read(&self, d: f64) -> f64 {
        let len = self.buf.len();
        let whole = d.floor();
        let frac = d - whole;
        let di = (whole as usize).min(len - 2);
        let at = |k: usize| self.buf[if self.w >= k { self.w - k } else { self.w + len - k }];
        let a = at(di);
        if frac == 0.0 { a } else { a + (at(di + 1) - a) * frac }
    }
}

/// Moves `cur` toward `target` with coefficient `k`; lands exactly.
#[inline]
fn approach(cur: &mut f64, target: f64, k: f64) {
    if *cur != target {
        *cur += k * (target - *cur);
        if (*cur - target).abs() < 1e-9 {
            *cur = target;
        }
    }
}

/// Equal-power pan gains of a position in −1..1.
#[inline]
fn pan_gains(pan: f64) -> (f64, f64) {
    let th = (pan.clamp(-1.0, 1.0) + 1.0) * std::f64::consts::FRAC_PI_4;
    (th.cos(), th.sin())
}

/// Delay-line pitch shifter with two crossfaded taps. Mono, ratio 0.25–4.
///
/// Plain crossfaded taps shift the pitch by `r` but add a frequency error: when the weight passes from one tap to
/// the other, the output phase moves by the phase difference between the two taps' signals, and that happens
/// at the wrap rate `|1 − r| / L`. Left alone the error is several hertz. So when a tap wraps (weight 0, silent) its
/// new delay is moved by up to ±`S` samples to the lag where its recent signal best matches the other tap's
/// (normalised cross-correlation over `M` samples): the two taps are then in phase for a periodic signal and the
/// error vanishes. Integer lags, deterministic, per-sample, so the output does not depend on the block size.
#[derive(Debug, Clone)]
pub struct PitchShifter {
    ring: Ring,
    /// Window in samples (even): the delay of a tap stays in `0..=win`.
    win: f64,
    /// Search range `S` (samples) and swept length `L = win − 2S`.
    search: usize,
    sweep: f64,
    /// Match length `M`.
    span: usize,
    /// Step of the alignment search, in lags and in matched samples: 1 up to 48 kHz, 2 at 96 kHz, so the burst of
    /// work at a wrap (`(2S+1)·M` reads in one sample) is the same at every rate instead of growing with its square.
    stride: usize,
    reference: Vec<f64>,
    phase: f64,
    /// Alignment offsets of the two taps (samples, within ±S).
    off: [f64; 2],
    ratio: f64,
    target: f64,
    k: f64,
}

impl PitchShifter {
    /// `window_ms` is rounded to an even number of samples; the mean delay is half of it.
    pub fn new(rate: f64, window_ms: f64) -> Self {
        let win = (2.0 * (window_ms * 0.001 * rate / 2.0).round()).max(8.0);
        let search = (win * 0.25).floor() as usize;
        let span = ((win * 0.25) as usize).max(32);
        PitchShifter {
            ring: Ring::new(win as usize + span + 2),
            win,
            search,
            sweep: win - 2.0 * search as f64,
            span,
            stride: ((rate / 48000.0).round() as usize).max(1),
            reference: vec![0.0; span],
            phase: 0.0,
            off: [0.0; 2],
            ratio: 1.0,
            target: 1.0,
            k: one_pole(RATIO_SMOOTH_MS, rate),
        }
    }

    /// Clears the line and restarts the sweep; the ratio lands on its target.
    pub fn reset(&mut self) {
        self.ring.clear();
        self.phase = 0.0;
        self.off = [0.0; 2];
        self.ratio = self.target;
    }

    /// The ratio glides to `ratio` (clamped to 0.25..4) in about 10 ms.
    pub fn set_ratio(&mut self, ratio: f64) {
        self.target = ratio.clamp(0.25, 4.0);
    }

    /// Jumps to the target ratio at once (for the first parameter set).
    pub fn snap_ratio(&mut self) {
        self.ratio = self.target;
    }

    /// Mean delay of the shifted output, in samples (half the window).
    pub fn mean_delay(&self) -> f64 {
        self.win * 0.5
    }

    /// Sweep position of tap `i` (0..1) for a phase.
    #[inline]
    fn pos(phase: f64, i: usize) -> f64 {
        let p = phase + 0.5 * i as f64;
        p - p.floor()
    }

    #[inline]
    fn delay(&self, p: f64, i: usize) -> f64 {
        self.search as f64 + self.sweep * p + self.off[i]
    }

    /// Tap `i` has just wrapped (its weight is 0): moves it to the lag that best matches tap `1 − i`.
    fn align(&mut self, i: usize, p: f64) {
        let j = 1 - i;
        let dj = self.delay(Self::pos(self.phase, j), j).round() as usize;
        let mut energy = 0.0;
        for m in (0..self.span).step_by(self.stride) {
            let v = self.ring.read((dj + m) as f64);
            self.reference[m] = v;
            energy += v * v;
        }
        if energy < 1e-20 {
            return;
        }
        let exact = self.search as f64 + self.sweep * p;
        let base = exact.round() as i64;
        let s = self.search as i64;
        let mut best = (0i64, f64::MIN);
        for delta in (-s..=s).step_by(self.stride) {
            let d = base + delta;
            if d < 0 {
                continue;
            }
            let (mut dot, mut e) = (0.0, 0.0);
            for m in (0..self.span).step_by(self.stride) {
                let v = self.ring.read((d as usize + m) as f64);
                dot += v * self.reference[m];
                e += v * v;
            }
            let score = dot / (e + 1e-12).sqrt();
            // Strictly better, scanning from the centre outward is not needed: ties keep the first (smallest) lag.
            if score > best.1 {
                best = (delta, score);
            }
        }
        self.off[i] = (base as f64 - exact) + best.0 as f64;
    }

    #[inline]
    pub fn tick(&mut self, x: f64) -> f64 {
        approach(&mut self.ratio, self.target, self.k);
        self.ring.write(x);
        let wa = 0.5 * (1.0 - (TWO_PI * self.phase).cos());
        let (p0, p1) = (Self::pos(self.phase, 0), Self::pos(self.phase, 1));
        let y = wa * self.ring.read(self.delay(p0, 0)) + (1.0 - wa) * self.ring.read(self.delay(p1, 1));
        self.phase += (1.0 - self.ratio) / self.sweep;
        self.phase -= self.phase.floor();
        for (i, old) in [p0, p1].into_iter().enumerate() {
            let p = Self::pos(self.phase, i);
            if (p - old).abs() > 0.5 {
                self.align(i, p);
            }
        }
        y
    }
}

#[inline]
fn semitones_to_ratio(semitones: f64, cents: f64) -> f64 {
    2f64.powf((semitones + cents * 0.01) / 12.0)
}

// ---------------------------------------------------------------------------------------------------------------
// Harmonizer

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HarmVoiceParams {
    pub on: bool,
    /// −12..12
    pub semitones: f64,
    /// ±50
    pub cents: f64,
    pub level_db: f64,
    /// −1..1
    pub pan: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HarmonizerParams {
    pub voices: [HarmVoiceParams; 2],
    /// Wet amount, 0..1: 0 is the dry signal alone (delayed by the latency).
    pub mix: f64,
}

#[derive(Debug, Clone)]
pub struct Harmonizer {
    shifters: [PitchShifter; 2],
    /// Delay of the voices after the shifter, to reach the common latency: 0..31 samples.
    rem: usize,
    voice_dly: [Ring; 2],
    dry: [Vec<f32>; 2],
    dry_pos: usize,
    gl: [f64; 2],
    gr: [f64; 2],
    gl_t: [f64; 2],
    gr_t: [f64; 2],
    mix: f64,
    mix_t: f64,
    k: f64,
    initialized: bool,
}

impl Harmonizer {
    fn window(rate: f64) -> f64 {
        (2.0 * (HARM_WINDOW_MS * 0.001 * rate / 2.0).round()).max(4.0)
    }

    /// Constant latency of the whole output, in samples: the shifter's mean delay rounded up to a multiple of 32.
    pub fn latency(rate: f64) -> usize {
        let mean = (Self::window(rate) * 0.5) as usize;
        mean.div_ceil(LATENCY_STEP).max(1) * LATENCY_STEP
    }

    pub fn new(rate: f64) -> Self {
        let lat = Self::latency(rate);
        let shifters = [PitchShifter::new(rate, HARM_WINDOW_MS), PitchShifter::new(rate, HARM_WINDOW_MS)];
        let rem = lat - shifters[0].mean_delay() as usize;
        Harmonizer {
            shifters,
            rem,
            voice_dly: [Ring::new(LATENCY_STEP), Ring::new(LATENCY_STEP)],
            dry: [vec![0.0; lat], vec![0.0; lat]],
            dry_pos: 0,
            gl: [0.0; 2],
            gr: [0.0; 2],
            gl_t: [0.0; 2],
            gr_t: [0.0; 2],
            mix: 0.0,
            mix_t: 0.0,
            k: one_pole(PARAM_SMOOTH_MS, rate),
            initialized: false,
        }
    }

    /// No allocation; may be called every 32 samples. The first call sets the values without gliding.
    pub fn set_params(&mut self, p: &HarmonizerParams) {
        for (v, vp) in p.voices.iter().enumerate() {
            let ratio = semitones_to_ratio(vp.semitones.clamp(-12.0, 12.0), vp.cents.clamp(-50.0, 50.0));
            self.shifters[v].set_ratio(ratio);
            let g = if vp.on { db_to_gain(vp.level_db.clamp(-60.0, 12.0)) } else { 0.0 };
            let (l, r) = pan_gains(vp.pan);
            self.gl_t[v] = g * l;
            self.gr_t[v] = g * r;
        }
        self.mix_t = p.mix.clamp(0.0, 1.0);
        if !self.initialized {
            self.initialized = true;
            self.snap();
        }
    }

    fn snap(&mut self) {
        self.gl = self.gl_t;
        self.gr = self.gr_t;
        self.mix = self.mix_t;
        self.shifters.iter_mut().for_each(PitchShifter::snap_ratio);
    }

    /// Clears the state; the parameters stay and take effect at once.
    pub fn reset(&mut self) {
        self.shifters.iter_mut().for_each(PitchShifter::reset);
        self.voice_dly.iter_mut().for_each(Ring::clear);
        self.dry.iter_mut().for_each(|d| d.fill(0.0));
        self.dry_pos = 0;
        self.snap();
    }

    /// In place; the two slices have the same length. The output is late by [`Harmonizer::latency`].
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let lat = self.dry[0].len();
        for i in 0..n {
            approach(&mut self.mix, self.mix_t, self.k);
            for v in 0..2 {
                approach(&mut self.gl[v], self.gl_t[v], self.k);
                approach(&mut self.gr[v], self.gr_t[v], self.k);
            }
            let (xl, xr) = (l[i], r[i]);
            let x = (f64::from(xl) + f64::from(xr)) * 0.5;
            let (mut wl, mut wr) = (0.0, 0.0);
            for v in 0..2 {
                let y = self.shifters[v].tick(x);
                self.voice_dly[v].write(y);
                let y = self.voice_dly[v].read(self.rem as f64);
                wl += self.gl[v] * y;
                wr += self.gr[v] * y;
            }
            let (dl, dr) = (self.dry[0][self.dry_pos], self.dry[1][self.dry_pos]);
            self.dry[0][self.dry_pos] = xl;
            self.dry[1][self.dry_pos] = xr;
            self.dry_pos += 1;
            if self.dry_pos == lat {
                self.dry_pos = 0;
            }
            l[i] = (f64::from(dl) + self.mix * wl) as f32;
            r[i] = (f64::from(dr) + self.mix * wr) as f32;
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Doubler

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DoublerParams {
    /// 1..2
    pub voices: u8,
    /// 0..30
    pub detune_cents: f64,
    /// 5..40
    pub delay_ms: f64,
    /// 0..1
    pub spread: f64,
    /// 0..1
    pub mix: f64,
}

/// Drift of the delay: rate of each voice's oscillator (Hz), relative depth, and the voices' delay ratio.
const DRIFT_HZ: [f64; 2] = [0.13, 0.21];
const DRIFT_DEPTH: f64 = 0.2;
const VOICE_DELAY: [f64; 2] = [1.0, 1.25];

#[derive(Debug, Clone)]
pub struct Doubler {
    rate: f64,
    shifters: [PitchShifter; 2],
    line: [Ring; 2],
    phase: [f64; 2],
    next_pos: i64,
    delay_ms: f64,
    delay_t: f64,
    gl: [f64; 2],
    gr: [f64; 2],
    gl_t: [f64; 2],
    gr_t: [f64; 2],
    mix: f64,
    mix_t: f64,
    k: f64,
    k_delay: f64,
    initialized: bool,
}

impl Doubler {
    pub const LATENCY: usize = 0;

    pub fn new(rate: f64) -> Self {
        // Longest delay: 40 ms · 1.25 · 1.2 = 60 ms.
        let max = (0.062 * rate).ceil() as usize;
        Doubler {
            rate,
            shifters: [PitchShifter::new(rate, DOUBLER_WINDOW_MS), PitchShifter::new(rate, DOUBLER_WINDOW_MS)],
            line: [Ring::new(max), Ring::new(max)],
            phase: [0.0; 2],
            next_pos: i64::MIN,
            delay_ms: 20.0,
            delay_t: 20.0,
            gl: [0.0; 2],
            gr: [0.0; 2],
            gl_t: [0.0; 2],
            gr_t: [0.0; 2],
            mix: 0.0,
            mix_t: 0.0,
            k: one_pole(PARAM_SMOOTH_MS, rate),
            k_delay: one_pole(30.0, rate),
            initialized: false,
        }
    }

    /// No allocation; may be called every 32 samples. The first call sets the values without gliding.
    pub fn set_params(&mut self, p: &DoublerParams) {
        let voices = p.voices.clamp(1, 2);
        let detune = p.detune_cents.clamp(0.0, 30.0);
        let spread = p.spread.clamp(0.0, 1.0);
        for v in 0..2 {
            let sign = if v == 0 { 1.0 } else { -1.0 };
            self.shifters[v].set_ratio(semitones_to_ratio(0.0, sign * detune));
            let (l, r) = pan_gains(if v == 0 { -spread } else { spread });
            let on = if usize::from(voices) > v { 1.0 } else { 0.0 };
            self.gl_t[v] = on * l;
            self.gr_t[v] = on * r;
        }
        self.delay_t = p.delay_ms.clamp(5.0, 40.0);
        self.mix_t = p.mix.clamp(0.0, 1.0);
        if !self.initialized {
            self.initialized = true;
            self.snap();
        }
    }

    fn snap(&mut self) {
        self.gl = self.gl_t;
        self.gr = self.gr_t;
        self.mix = self.mix_t;
        self.delay_ms = self.delay_t;
        self.shifters.iter_mut().for_each(PitchShifter::snap_ratio);
    }

    /// Clears the state; the parameters stay and take effect at once.
    pub fn reset(&mut self) {
        self.shifters.iter_mut().for_each(PitchShifter::reset);
        self.line.iter_mut().for_each(Ring::clear);
        self.next_pos = i64::MIN;
        self.snap();
    }

    /// In place. `pos` is the timeline frame of the first sample. The phase of the slow delay drift is
    /// `frac(f · pos / rate)` whenever `pos` is not the frame that follows the previous call (the first call,
    /// a seek, a loop), and accumulates otherwise: the same position gives the same drift.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], pos: i64) {
        let n = l.len().min(r.len());
        if pos != self.next_pos {
            for v in 0..2 {
                let x = DRIFT_HZ[v] * pos as f64 / self.rate;
                self.phase[v] = x - x.floor();
            }
        }
        self.next_pos = pos.saturating_add(n as i64);
        let ms = 0.001 * self.rate;
        for i in 0..n {
            approach(&mut self.mix, self.mix_t, self.k);
            approach(&mut self.delay_ms, self.delay_t, self.k_delay);
            let (xl, xr) = (f64::from(l[i]), f64::from(r[i]));
            let x = (xl + xr) * 0.5;
            let (mut wl, mut wr) = (0.0, 0.0);
            for v in 0..2 {
                approach(&mut self.gl[v], self.gl_t[v], self.k);
                approach(&mut self.gr[v], self.gr_t[v], self.k);
                let y = self.shifters[v].tick(x);
                self.line[v].write(y);
                let d_ms = self.delay_ms * VOICE_DELAY[v] * (1.0 + DRIFT_DEPTH * (TWO_PI * self.phase[v]).sin());
                // The shifter already delays by its mean delay.
                let extra = (d_ms * ms - self.shifters[v].mean_delay()).max(0.0);
                let y = self.line[v].read(extra);
                wl += self.gl[v] * y;
                wr += self.gr[v] * y;
                self.phase[v] += DRIFT_HZ[v] / self.rate;
                self.phase[v] -= self.phase[v].floor();
            }
            l[i] = (xl + self.mix * wl) as f32;
            r[i] = (xr + self.mix * wr) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::xorshift32;

    const FS: f64 = 48000.0;
    const A4: f64 = 440.0;

    fn sine(hz: f64, secs: f64) -> Vec<f32> {
        (0..(secs * FS) as usize).map(|i| (0.5 * (TWO_PI * hz * i as f64 / FS).sin()) as f32).collect()
    }

    /// Frequency of the strongest component in `[lo, hi]` Hz of a Hann-windowed segment: a coarse DFT scan then
    /// a fine one around the maximum. Accurate to a few hundredths of a hertz on a stable sinusoid.
    fn peak_hz(x: &[f32], lo: f64, hi: f64) -> f64 {
        let n = x.len();
        let w: Vec<f64> =
            (0..n).map(|i| f64::from(x[i]) * (0.5 - 0.5 * (TWO_PI * i as f64 / n as f64).cos())).collect();
        let mag = |f: f64| {
            let (mut re, mut im) = (0.0, 0.0);
            let step = TWO_PI * f / FS;
            let (c, s) = (step.cos(), step.sin());
            // Recursive oscillator: cheap and accurate enough over 48k samples in f64.
            let (mut cr, mut ci) = (1.0, 0.0);
            for v in &w {
                re += v * cr;
                im += v * ci;
                let t = cr * c - ci * s;
                ci = cr * s + ci * c;
                cr = t;
            }
            re * re + im * im
        };
        let mut best = (lo, f64::MIN);
        let mut f = lo;
        while f <= hi {
            let m = mag(f);
            if m > best.1 {
                best = (f, m);
            }
            f += 2.0;
        }
        let (c0, mut fine) = (best.0, (best.0, f64::MIN));
        let mut f = c0 - 2.0;
        while f <= c0 + 2.0 {
            let m = mag(f);
            if m > fine.1 {
                fine = (f, m);
            }
            f += 0.05;
        }
        fine.0
    }

    fn cents(f: f64, reference: f64) -> f64 {
        1200.0 * (f / reference).log2()
    }

    fn noise(seed: u32, n: usize) -> Vec<f32> {
        xorshift32(seed, n).iter().map(|v| *v as f32).collect()
    }

    /// Runs `process` over `l`/`r` in blocks of `block`, with the same parameters.
    fn run_harm(p: &HarmonizerParams, l: &[f32], r: &[f32], block: usize) -> (Vec<f32>, Vec<f32>) {
        let mut h = Harmonizer::new(FS);
        h.set_params(p);
        let (mut ol, mut or) = (l.to_vec(), r.to_vec());
        let mut i = 0;
        while i < l.len() {
            let e = (i + block).min(l.len());
            h.process(&mut ol[i..e], &mut or[i..e]);
            i = e;
        }
        (ol, or)
    }

    fn run_dbl(p: &DoublerParams, l: &[f32], r: &[f32], block: usize) -> (Vec<f32>, Vec<f32>) {
        let mut d = Doubler::new(FS);
        d.set_params(p);
        let (mut ol, mut or) = (l.to_vec(), r.to_vec());
        let mut i = 0;
        while i < l.len() {
            let e = (i + block).min(l.len());
            d.process(&mut ol[i..e], &mut or[i..e], i as i64);
            i = e;
        }
        (ol, or)
    }

    fn voice(on: bool, st: f64, ct: f64) -> HarmVoiceParams {
        HarmVoiceParams { on, semitones: st, cents: ct, level_db: 0.0, pan: 0.0 }
    }

    fn harm(v0: HarmVoiceParams, v1: HarmVoiceParams, mix: f64) -> HarmonizerParams {
        HarmonizerParams { voices: [v0, v1], mix }
    }

    fn dbl(voices: u8, detune: f64, delay: f64, spread: f64, mix: f64) -> DoublerParams {
        DoublerParams { voices, detune_cents: detune, delay_ms: delay, spread, mix }
    }

    /// req: FX-21
    #[test]
    fn the_shifter_lands_on_the_interval_within_five_cents() {
        // Tolerance ±5 cents (FX-21). The sweep is exact, so the measured error is a few hundredths of a cent;
        // the limit is the one of the requirement. 3 s of sine, the last second analysed (ratio settled).
        let x = sine(A4, 3.0);
        for (st, ct) in [(7.0, 0.0), (-5.0, 0.0), (12.0, 0.0), (-12.0, 0.0), (7.0, 25.0)] {
            let ratio = semitones_to_ratio(st, ct);
            let mut s = PitchShifter::new(FS, 35.0);
            s.set_ratio(ratio);
            s.snap_ratio();
            let y: Vec<f32> = x.iter().map(|v| s.tick(f64::from(*v)) as f32).collect();
            let seg = &y[2 * FS as usize..];
            let want = A4 * ratio;
            let got = peak_hz(seg, want * 0.9, want * 1.1);
            assert!(cents(got, want).abs() <= 5.0, "{st} {ct}: {got:.2} Hz, wanted {want:.2}");
        }
    }

    /// req: FX-21
    #[test]
    fn the_interval_holds_at_other_pitches_too() {
        // The tap alignment makes the error independent of the pitch (a bare crossfade is off by several hertz
        // and by a different amount at each pitch). 150 Hz needs a lag of a whole period inside ±S (8.75 ms).
        for hz in [150.0, 1000.0, 3000.0] {
            let x = sine(hz, 3.0);
            for st in [7.0, -5.0] {
                let ratio = semitones_to_ratio(st, 0.0);
                let mut s = PitchShifter::new(FS, 35.0);
                s.set_ratio(ratio);
                s.snap_ratio();
                let y: Vec<f32> = x.iter().map(|v| s.tick(f64::from(*v)) as f32).collect();
                let want = hz * ratio;
                let got = peak_hz(&y[2 * FS as usize..], want * 0.95, want * 1.05);
                assert!(cents(got, want).abs() <= 5.0, "{hz} {st}: {got:.2} Hz, wanted {want:.2}");
            }
        }
    }

    /// At 96 kHz the alignment search steps by two (lags and matched samples), so a wrap costs what it costs at
    /// 48 kHz: the interval is still within five cents. The shifter is built for 96 kHz and fed samples that the
    /// measurement reads as 48 kHz, which is the same thing an octave down.
    /// req: FX-21, NFR-1
    #[test]
    fn the_interval_holds_at_96_khz_with_the_coarser_search() {
        for hz in [150.0, 1000.0] {
            let x = sine(hz, 3.0);
            for st in [7.0, -5.0] {
                let ratio = semitones_to_ratio(st, 0.0);
                let mut s = PitchShifter::new(2.0 * FS, 35.0);
                assert_eq!(s.stride, 2);
                s.set_ratio(ratio);
                s.snap_ratio();
                let y: Vec<f32> = x.iter().map(|v| s.tick(f64::from(*v)) as f32).collect();
                let want = hz * ratio;
                let got = peak_hz(&y[2 * FS as usize..], want * 0.95, want * 1.05);
                assert!(cents(got, want).abs() <= 5.0, "{hz} {st}: {got:.2} Hz, wanted {want:.2}");
            }
        }
    }

    /// req: FX-21
    #[test]
    fn the_harmonizer_voice_shifts_by_semitones_plus_cents() {
        // A voice at +7 st +25 ct, with the dry signal also present at 440 Hz (outside the ±10% search band).
        let x = sine(A4, 3.0);
        let p = harm(voice(true, 7.0, 25.0), voice(false, 0.0, 0.0), 1.0);
        let (ol, _) = run_harm(&p, &x, &x, 512);
        let want = A4 * semitones_to_ratio(7.0, 25.0);
        let got = peak_hz(&ol[2 * FS as usize..], want * 0.95, want * 1.05);
        assert!(cents(got, want).abs() <= 5.0, "{got:.2} Hz, wanted {want:.2}");
    }

    /// req: FX-21, FX-24
    #[test]
    fn at_mix_zero_the_harmonizer_is_the_dry_signal_delayed_by_its_latency() {
        // Exact (≤ 1e-6, in practice 0): the dry path is a plain delay of `latency` samples.
        let n = 20000;
        let (l, r) = (noise(1, n), noise(2, n));
        let lat = Harmonizer::latency(FS);
        assert_eq!(lat % 32, 0);
        let p = harm(voice(true, 7.0, 0.0), voice(true, -5.0, 0.0), 0.0);
        let (ol, or) = run_harm(&p, &l, &r, 100);
        for i in 0..n {
            let (el, er) = if i >= lat { (l[i - lat], r[i - lat]) } else { (0.0, 0.0) };
            assert!((ol[i] - el).abs() <= 1e-6 && (or[i] - er).abs() <= 1e-6, "{i}");
        }
    }

    /// req: FX-21, FX-24
    #[test]
    fn the_latency_is_a_multiple_of_32_for_every_rate() {
        for rate in [44100.0, 48000.0, 88200.0, 96000.0, 192000.0] {
            let lat = Harmonizer::latency(rate);
            assert!(lat.is_multiple_of(32) && lat > 0, "{rate}: {lat}");
            let mean = PitchShifter::new(rate, HARM_WINDOW_MS).mean_delay();
            assert!(lat as f64 >= mean && (lat as f64) < mean + 32.0, "{rate}: {lat} {mean}");
        }
    }

    /// req: FX-21, FX-24
    #[test]
    fn an_unshifted_voice_lines_up_with_the_delayed_dry_signal() {
        // Voice at 0 st, 0 dB, centred, mono input: the wet part is 0.7071 × the delayed input, so the
        // cross-correlation of the wet part with the delayed dry signal peaks at lag 0 (±1 sample allowed by
        // FX-24) and the two differ by less than the f32 rounding of the output (1e-6).
        let n = 30000;
        let x = noise(3, n);
        let lat = Harmonizer::latency(FS);
        let (ol, _) = run_harm(&harm(voice(true, 0.0, 0.0), voice(false, 0.0, 0.0), 1.0), &x, &x, 333);
        let wet: Vec<f64> =
            (0..n).map(|i| f64::from(ol[i]) - if i >= lat { f64::from(x[i - lat]) } else { 0.0 }).collect();
        let corr = |lag: i64| {
            let mut s = 0.0;
            for i in lat + 8..n - 8 {
                s += wet[i] * f64::from(x[(i as i64 - lat as i64 + lag) as usize]);
            }
            s
        };
        let best = (-3i64..=3).max_by(|a, b| corr(*a).total_cmp(&corr(*b))).unwrap();
        assert!(best.abs() <= 1, "peak at lag {best}");
        for i in lat..n {
            let d = (wet[i] - std::f64::consts::FRAC_1_SQRT_2 * f64::from(x[i - lat])).abs();
            assert!(d <= 1e-6, "{i}: {d}");
        }
    }

    /// req: FX-21
    #[test]
    fn voice_level_and_pan_place_the_voice() {
        // −6.02 dB halves the voice; a voice panned hard left leaves the right channel at the dry signal.
        let n = 20000;
        let x = noise(4, n);
        let lat = Harmonizer::latency(FS);
        let mut v = voice(true, 0.0, 0.0);
        v.pan = -1.0;
        v.level_db = -6.0206;
        let (ol, or) = run_harm(&harm(v, voice(false, 0.0, 0.0), 1.0), &x, &x, 64);
        for i in lat + 4000..n {
            assert!((f64::from(or[i]) - f64::from(x[i - lat])).abs() <= 1e-6, "right {i}");
            assert!((f64::from(ol[i]) - 1.5 * f64::from(x[i - lat])).abs() <= 1e-4, "left {i}");
        }
    }

    /// req: FX-24
    #[test]
    fn at_mix_zero_the_doubler_is_the_identity() {
        let n = 20000;
        let (l, r) = (noise(5, n), noise(6, n));
        let (ol, or) = run_dbl(&dbl(2, 30.0, 25.0, 1.0, 0.0), &l, &r, 64);
        assert_eq!(Doubler::LATENCY, 0);
        for i in 0..n {
            assert!((ol[i] - l[i]).abs() <= 1e-6 && (or[i] - r[i]).abs() <= 1e-6, "{i}");
        }
    }

    /// req: FX-21
    #[test]
    fn the_doubler_voice_is_detuned_and_the_spread_pans_it() {
        // One voice (left at spread 1) detuned by 30 cents: the wet part, output minus the undelayed dry, peaks
        // at 1 kHz · 2^(30/1200) (±5 cents) in the left channel and is exactly zero in the right one.
        let x = sine(1000.0, 3.0);
        let (ol, or) = run_dbl(&dbl(1, 30.0, 20.0, 1.0, 1.0), &x, &x, 256);
        let wet: Vec<f32> = ol.iter().zip(&x).map(|(o, d)| o - d).collect();
        let want = 1000.0 * semitones_to_ratio(0.0, 30.0);
        let got = peak_hz(&wet[2 * FS as usize..], want * 0.95, want * 1.05);
        assert!(cents(got, want).abs() <= 5.0, "{got:.2} Hz, wanted {want:.2}");
        assert!(or.iter().zip(&x).all(|(o, d)| (o - d).abs() <= 1e-7), "right channel");
        // Spread 0: the voice is centred, both channels carry the same wet signal.
        let (cl, cr) = run_dbl(&dbl(2, 10.0, 20.0, 0.0, 1.0), &x, &x, 256);
        assert!(cl.iter().zip(&cr).all(|(a, b)| (a - b).abs() <= 1e-6));
    }

    /// req: FX-21
    #[test]
    fn the_doubler_delays_the_copy_by_the_set_time_within_its_drift() {
        // No detune: the copy is the input delayed by 20 ms ± 20% (the drift), plus the interpolation. The best
        // lag of the wet part against the input over 12–28 ms must fall in 16–24 ms.
        let n = 24000;
        let x = noise(8, n);
        let (ol, _) = run_dbl(&dbl(1, 0.0, 20.0, 0.0, 1.0), &x, &x, 64);
        let wet: Vec<f64> = (0..n).map(|i| f64::from(ol[i] - x[i])).collect();
        let corr = |lag: usize| (4000..8000).map(|i| wet[i] * f64::from(x[i - lag])).sum::<f64>();
        let best = (576..1344).max_by(|a, b| corr(*a).total_cmp(&corr(*b))).unwrap();
        assert!((768..=1152).contains(&best), "lag {best}");
    }

    /// req: FX-21, FX-24
    #[test]
    fn full_scale_noise_at_extreme_settings_stays_bounded() {
        // Every shifter output is a convex combination of two input samples, so |y| ≤ 1 for a full-scale input.
        // Harmonizer: dry 1 + two voices at +12 dB (3.98) with a pan gain ≤ 1 → 9; 10 allows for rounding.
        // Doubler: dry 1 + two voices at gain ≤ 1 → 3.
        let n = 5 * FS as usize;
        let (l, r) = (noise(9, n), noise(10, n));
        let mut v = voice(true, 12.0, 50.0);
        v.level_db = 12.0;
        let (ol, or) = run_harm(&harm(v, v, 1.0), &l, &r, 512);
        assert!(ol.iter().chain(or.iter()).all(|x| x.is_finite() && x.abs() <= 10.0));
        let (dl, dr) = run_dbl(&dbl(2, 30.0, 40.0, 1.0, 1.0), &l, &r, 512);
        assert!(dl.iter().chain(dr.iter()).all(|x| x.is_finite() && x.abs() <= 3.0));
    }

    /// req: FX-24, ENG-4
    #[test]
    fn the_block_size_does_not_change_the_output() {
        // Per-sample arithmetic, parameters set once: the renders are the same samples (≤ 1e-6 required).
        let n = 20000;
        let (l, r) = (noise(11, n), noise(12, n));
        let hp = harm(voice(true, 7.0, 10.0), voice(true, -12.0, -20.0), 0.8);
        let dp = dbl(2, 20.0, 18.0, 0.7, 0.6);
        let h0 = run_harm(&hp, &l, &r, 32);
        let d0 = run_dbl(&dp, &l, &r, 32);
        for block in [1000, 7] {
            let h = run_harm(&hp, &l, &r, block);
            let d = run_dbl(&dp, &l, &r, block);
            let max = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
            assert!(max(&h0.0, &h.0) <= 1e-6 && max(&h0.1, &h.1) <= 1e-6, "harmonizer {block}");
            assert!(max(&d0.0, &d.0) <= 1e-6 && max(&d0.1, &d.1) <= 1e-6, "doubler {block}");
        }
    }

    /// req: FX-24, AUT-6
    #[test]
    fn parameter_changes_every_32_samples_stay_stable() {
        // 4 s of noise with new random parameters every 32 samples: finite and inside the extreme bounds of the
        // bounded-noise test (10 and 3).
        let n = 4 * FS as usize;
        let (l, r) = (noise(13, n), noise(14, n));
        let mut h = Harmonizer::new(FS);
        let mut d = Doubler::new(FS);
        let (mut hl, mut hr, mut dl, mut dr) = (l.clone(), r.clone(), l.clone(), r.clone());
        let mut i = 0;
        let mut seed = 21u32;
        while i + 32 <= n {
            let u = xorshift32(seed, 16);
            seed += 1;
            let vp = |o: usize| HarmVoiceParams {
                on: u[o] > -0.5,
                semitones: u[o + 1] * 12.0,
                cents: u[o + 2] * 50.0,
                level_db: u[o + 3] * 6.0,
                pan: u[o + 4],
            };
            h.set_params(&harm(vp(0), vp(5), u[10].abs()));
            d.set_params(&dbl(
                1 + (u[11] > 0.0) as u8,
                u[12].abs() * 30.0,
                5.0 + u[13].abs() * 35.0,
                u[14].abs(),
                u[15].abs(),
            ));
            h.process(&mut hl[i..i + 32], &mut hr[i..i + 32]);
            d.process(&mut dl[i..i + 32], &mut dr[i..i + 32], i as i64);
            i += 32;
        }
        assert!(hl.iter().chain(hr.iter()).all(|x| x.is_finite() && x.abs() <= 10.0));
        assert!(dl.iter().chain(dr.iter()).all(|x| x.is_finite() && x.abs() <= 3.0));
    }

    /// req: FX-21, ENG-4
    #[test]
    fn the_doubler_drift_follows_the_song_position() {
        // A render started at frame P gives what a render from 0 gives from P on, because the drift phase is
        // frac(f · pos / rate) after a seek (two blocks that do not follow each other). With no detune the only
        // state is the delay line (at most 60 ms = 2880 samples), so past 3000 samples the two renders agree to 1e-3
        // (the phase error of 1 s of f64 steps is ~1e-12).
        let n = 48000;
        let x = noise(15, 2 * n);
        let p = dbl(1, 0.0, 20.0, 0.0, 1.0);
        let (full, _) = run_dbl(&p, &x, &x, 4096);
        let start = n;
        let mut d = Doubler::new(FS);
        d.set_params(&p);
        let (mut a, mut b) = (x[start - 8000..start + 4000].to_vec(), x[start - 8000..start + 4000].to_vec());
        d.process(&mut a, &mut b, (start - 8000) as i64);
        for i in 3000..12000 {
            assert!((a[i] - full[start - 8000 + i]).abs() <= 1e-3, "{i}");
        }
    }
}
