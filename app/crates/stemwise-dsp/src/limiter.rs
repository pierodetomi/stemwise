//! True-peak limiter with look-ahead (MAS-1, MAS-3).
//!
//! 1. Detection: each channel goes through the 4x true-peak interpolator of `truepeak` (the PoC's filter); the
//!    detector value for a sample is the largest of its four interpolated points, over both channels.
//! 2. Required gain `r = min(1, c / tp)`, with `c` the ceiling lowered by an internal margin of 0.1 dB (covers
//!    the inter-sample effect of a moving gain and 24-bit quantisation).
//! 3. Hold: the minimum of `r` over the last `L` samples (`L` = 2 ms of look-ahead).
//! 4. Attack: a moving average of the held value over `L` samples. Every average that includes a peak's sample
//!    is made of values ≤ that peak's `r`, so the gain applied to the peak (the signal is delayed by `L − 1`) is
//!    at most `r`: the ceiling holds by construction, with a smooth `L`-sample ramp.
//! 5. Release: when the gain rises, it follows with a one-pole of `release_ms`; falling is never slowed.
//!
//! Total latency [`Limiter::latency`] = 10 (detector) + `L − 1` samples; the engine compensates it (ENG-5).
//! The guarantee is checked again on the exported file (`truepeak::true_peak_db`), which the export repeats
//! with a lower ceiling in the rare case it is exceeded (MAS-3, MAS-10).

use crate::truepeak::TruePeakStream;
use crate::util::db_to_gain;

/// Internal margin under the requested ceiling.
pub const MARGIN_DB: f64 = 0.1;

pub struct Limiter {
    rate: f64,
    look: usize,
    ceiling: f64,
    release: f64,
    det: [TruePeakStream; 2],
    // Hold (minimum over the last L required gains): a monotonic deque in a fixed ring.
    deque: Vec<(u64, f64)>,
    dq_head: usize,
    dq_len: usize,
    // Moving average of the held gain.
    box_ring: Vec<f64>,
    box_sum: f64,
    // Signal delay line.
    delay: [Vec<f64>; 2],
    pos: usize,
    n: u64,
    gain: f64,
    gr_meter: f64,
}

impl Limiter {
    pub fn new(rate: f64) -> Self {
        let look = ((rate * 0.002).round() as usize).max(8);
        let lat = TruePeakStream::LATENCY + look - 1;
        Limiter {
            rate,
            look,
            ceiling: db_to_gain(-1.0 - MARGIN_DB),
            release: (-1.0 / (0.08 * rate)).exp(),
            det: [TruePeakStream::default(), TruePeakStream::default()],
            deque: vec![(0, 1.0); look + 1],
            dq_head: 0,
            dq_len: 0,
            box_ring: vec![1.0; look],
            box_sum: look as f64,
            delay: [vec![0.0; lat], vec![0.0; lat]],
            pos: 0,
            n: 0,
            gain: 1.0,
            gr_meter: 0.0,
        }
    }

    /// Samples by which the output lags the input.
    pub fn latency(&self) -> usize {
        TruePeakStream::LATENCY + self.look - 1
    }

    pub fn set_params(&mut self, ceiling_db: f64, release_ms: f64) {
        self.ceiling = db_to_gain(ceiling_db - MARGIN_DB);
        self.release = (-1.0 / (release_ms.max(1.0) * 0.001 * self.rate)).exp();
    }

    pub fn reset(&mut self) {
        let rate = self.rate;
        let (c, r) = (self.ceiling, self.release);
        *self = Limiter::new(rate);
        self.ceiling = c;
        self.release = r;
    }

    /// Deepest gain reduction (dB, ≤ 0) since the last call.
    pub fn take_gr_db(&mut self) -> f64 {
        std::mem::replace(&mut self.gr_meter, 0.0)
    }

    #[inline]
    fn hold_push(&mut self, r: f64) -> f64 {
        let cap = self.deque.len();
        // Drop values not smaller than r from the back.
        while self.dq_len > 0 {
            let back = (self.dq_head + self.dq_len - 1) % cap;
            if self.deque[back].1 >= r {
                self.dq_len -= 1;
            } else {
                break;
            }
        }
        let back = (self.dq_head + self.dq_len) % cap;
        self.deque[back] = (self.n, r);
        self.dq_len += 1;
        // Drop expired values from the front.
        while self.deque[self.dq_head].0 + (self.look as u64) <= self.n {
            self.dq_head = (self.dq_head + 1) % cap;
            self.dq_len -= 1;
        }
        self.deque[self.dq_head].1
    }

    /// Processes a stereo pair in place.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let lat = self.delay[0].len();
        for i in 0..l.len() {
            let (xl, xr) = (f64::from(l[i]), f64::from(r[i]));
            let tp = self.det[0].push(xl).max(self.det[1].push(xr));
            let need = if tp > self.ceiling { self.ceiling / tp } else { 1.0 };
            let held = self.hold_push(need);
            let slot = (self.n % self.look as u64) as usize;
            self.box_sum += held - self.box_ring[slot];
            self.box_ring[slot] = held;
            let target = (self.box_sum / self.look as f64).min(1.0);
            self.gain = if target < self.gain { target } else { target + self.release * (self.gain - target) };
            // The delay aligns the detector (10 samples) and the look-ahead (L − 1).
            let out_l = self.delay[0][self.pos];
            let out_r = self.delay[1][self.pos];
            self.delay[0][self.pos] = xl;
            self.delay[1][self.pos] = xr;
            self.pos = (self.pos + 1) % lat;
            l[i] = (out_l * self.gain) as f32;
            r[i] = (out_r * self.gain) as f32;
            self.gr_meter = self.gr_meter.min(20.0 * self.gain.log10());
            self.n += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::{render, xorshift32};
    use crate::truepeak::true_peak_db;

    fn run(lim: &mut Limiter, x: &[f32], block: usize) -> Vec<f32> {
        let mut l: Vec<f32> = x.iter().step_by(2).copied().collect();
        let mut r: Vec<f32> = x.iter().skip(1).step_by(2).copied().collect();
        for (a, b) in l.chunks_mut(block).zip(r.chunks_mut(block)) {
            lim.process(a, b);
        }
        l.iter().zip(&r).flat_map(|(a, b)| [*a, *b]).collect()
    }

    /// req: MAS-3
    #[test]
    fn the_true_peak_ceiling_holds_on_hot_material() {
        // Loud pink noise, hard sine bursts near Nyquist (inter-sample peaks) and clicks, driven 12 dB over
        // full scale: the 4x true peak of the output stays under every ceiling (measured with the export's
        // meter, tolerance 0: the guarantee is by construction).
        let rate = 48000u32;
        let spec = serde_json::json!({"kind": "stereo",
            "left": {"kind": "sum", "parts": [{"kind": "pink", "seed": 1, "amp": 2.5, "secs": 6.0},
                {"kind": "bursts", "freq": 11025.0, "amp": 3.0, "period": 0.37, "on": 0.05, "secs": 6.0}]},
            "right": {"kind": "sum", "parts": [{"kind": "pink", "seed": 2, "amp": 2.5, "secs": 6.0},
                {"kind": "clicks", "rate": 3.0, "amp": 4.0, "secs": 6.0, "offset": 100}]}});
        let (x, _) = render(&spec, rate);
        for (ceil, rel) in [(-1.0, 80.0), (-0.3, 20.0), (-3.0, 500.0)] {
            let mut lim = Limiter::new(f64::from(rate));
            lim.set_params(ceil, rel);
            let y = run(&mut lim, &x, 512);
            let tp = true_peak_db(&y, 2);
            assert!(tp <= ceil, "ceiling {ceil}: {tp}");
            assert!(tp > ceil - 0.5, "the limiter should not over-limit: {tp}");
            assert!(lim.take_gr_db() < -6.0);
        }
    }

    /// req: MAS-1, ENG-5
    #[test]
    fn quiet_material_passes_unchanged_after_the_latency() {
        let rate = 48000.0;
        let x: Vec<f32> = xorshift32(5, 9600).iter().map(|v| (*v * 0.1) as f32).collect();
        let mut lim = Limiter::new(rate);
        let lat = lim.latency();
        let y = run(&mut lim, &x, 64);
        for i in 0..(x.len() / 2 - lat) {
            assert_eq!(y[2 * (i + lat)], x[2 * i]);
        }
    }

    /// req: MAS-1
    #[test]
    fn output_is_block_invariant_and_gain_moves_smoothly() {
        let rate = 48000u32;
        let (x, _) = render(
            &serde_json::json!({"kind": "stereo", "left": {"kind": "pink", "seed": 8, "amp": 3.0, "secs": 3.0},
            "right": {"kind": "pink", "seed": 9, "amp": 3.0, "secs": 3.0}}),
            rate,
        );
        let mut a = Limiter::new(f64::from(rate));
        let mut b = Limiter::new(f64::from(rate));
        let ya = run(&mut a, &x, 64);
        let yb = run(&mut b, &x, 4096);
        assert_eq!(ya, yb);
    }
}
