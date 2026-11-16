//! NFR-2: the processors' `process` never allocates. A global allocator counts allocations made while a
//! thread-local flag is set; every processor runs a few blocks inside the guarded region.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static COUNT: AtomicUsize = AtomicUsize::new(0);
thread_local!(static GUARD: Cell<bool> = const { Cell::new(false) });

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if GUARD.with(|g| g.get()) {
            COUNT.fetch_add(1, Ordering::SeqCst);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if GUARD.with(|g| g.get()) {
            COUNT.fetch_add(1, Ordering::SeqCst);
        }
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn guarded(f: impl FnOnce()) -> usize {
    let before = COUNT.load(Ordering::SeqCst);
    GUARD.with(|g| g.set(true));
    f();
    GUARD.with(|g| g.set(false));
    COUNT.load(Ordering::SeqCst) - before
}

/// req: NFR-2, FX-24
#[test]
fn processors_do_not_allocate_while_processing() {
    use stemwise_dsp::biquad::{Biquad, Coeffs, Kind};
    use stemwise_dsp::clipper::{Clipper, ClipperParams};
    use stemwise_dsp::color::{BassEnhancer, BassParams, Exciter, ExciterParams};
    use stemwise_dsp::compressor::{Compressor, CompressorParams};
    use stemwise_dsp::deesser::{Deesser, DeesserParams};
    use stemwise_dsp::delay::{Delay, DelayCharacter, DelayMode, DelayParams};
    use stemwise_dsp::dyneq::{DynamicEq, DynamicEqParams};
    use stemwise_dsp::gate::{Gate, GateParams};
    use stemwise_dsp::limiter::Limiter;
    use stemwise_dsp::loudness::LoudnessMeter;
    use stemwise_dsp::master::{MatchEq, Width};
    use stemwise_dsp::meters::{Correlation, PeakRms};
    use stemwise_dsp::modulation::{LfoShape, ModMode, ModParams, Modulation, Tremolo, TremoloMode, TremoloParams};
    use stemwise_dsp::multiband::{Multiband, MultibandParams};
    use stemwise_dsp::pitch::{Doubler, DoublerParams, HarmVoiceParams, Harmonizer, HarmonizerParams};
    use stemwise_dsp::reverb::{Algorithm, Reverb, ReverbParams};
    use stemwise_dsp::saturation::{Saturation, SaturationParams};
    use stemwise_dsp::transient::{Transient, TransientParams};
    use stemwise_dsp::truepeak::TruePeakStream;
    use stemwise_dsp::width::{StereoWidth, WidthParams};

    let rate = 48000.0;
    let mut l: Vec<f32> = (0..512).map(|n| ((n as f32) * 0.01).sin() * 0.9).collect();
    let mut r = l.clone();
    let (mut ol, mut or) = (vec![0.0f32; 512], vec![0.0f32; 512]);
    let key = l.clone();
    let mut bq = Biquad::new(Coeffs::design(Kind::Peak, 1000.0, 3.0, 1.0, rate));
    let mut comp = Compressor::new(rate);
    let mut sat = Saturation::new(rate);
    let mut revs =
        [Algorithm::Plate, Algorithm::Room, Algorithm::Hall, Algorithm::Shimmer].map(|a| Reverb::new(rate, a));
    let mut clip = Clipper::new(rate);
    let mut mb = Multiband::new(rate);
    let mut dly = Delay::new(rate);
    let mut lim = Limiter::new(rate);
    let mut lm = LoudnessMeter::new(rate);
    let mut meq = MatchEq::new(rate);
    let mut wid = Width::new(rate);
    let mut tp = TruePeakStream::default();
    let mut gate = Gate::new(rate);
    let mut ds = Deesser::new(rate);
    let mut deq = DynamicEq::new(rate);
    let mut tr = Transient::new(rate);
    let mut exc = Exciter::new(rate);
    let mut bass = BassEnhancer::new(rate);
    let mut sw = StereoWidth::new(rate);
    let mut md = Modulation::new(rate);
    let mut trem = Tremolo::new(rate);
    let mut harm = Harmonizer::new(rate);
    let mut dbl = Doubler::new(rate);
    let (mut pr, mut cor) = (PeakRms::default(), Correlation::default());
    let n = guarded(|| {
        for block in 0..8 {
            let v = block as f64;
            bq.c = Coeffs::design(Kind::Peak, 1000.0 + v * 10.0, 3.0, 1.0, rate);
            bq.process(0, &mut l);
            comp.set_params(&CompressorParams { threshold_db: -20.0 - v, ..Default::default() });
            comp.process(&mut [&mut l[..], &mut r[..]], Some(&[&key[..]]));
            sat.set_params(&SaturationParams { drive_db: 6.0 + v, mix: 0.8, character: Default::default() });
            sat.process(0, &mut l);
            sat.process(1, &mut r);
            for rev in &mut revs {
                rev.set_params(&ReverbParams {
                    decay_s: 1.0 + v * 0.3,
                    size: 0.3 + v * 0.05,
                    damping: 0.5,
                    modulation: 0.5,
                    predelay_ms: 20.0,
                    hpf_hz: 200.0 + v,
                    lpf_hz: 9000.0 - v,
                    shimmer_semitones: 12.0,
                    shimmer_amount: 0.4,
                });
                rev.process(&l, &r, &mut ol, &mut or);
            }
            clip.set_params(&ClipperParams { threshold: 0.5 + v * 0.05, softness: 0.4 });
            clip.process(0, &mut l);
            clip.process(1, &mut r);
            let mut mp = MultibandParams { bands: 2 + block % 5, ..Default::default() };
            mp.band[0].comp.ratio = 4.0;
            mp.band[0].comp.threshold_db = -30.0;
            mp.xo_hz[0] = 100.0 + v * 10.0;
            mb.set_params(&mp);
            mb.process(&mut l, &mut r);
            dly.set_params(&DelayParams {
                time_ms: 300.0 + v,
                feedback: 0.4,
                feedback_hpf_hz: 200.0,
                feedback_lpf_hz: 6000.0,
                mode: DelayMode::PingPong,
                character: DelayCharacter::Tape,
                wow: 0.5,
                drive_db: 6.0,
            });
            dly.process_at(&l, &r, &mut ol, &mut or, block as i64 * 512);
            // The blocks of P-018 (FX-13 … FX-23).
            gate.set_params(&GateParams { threshold_db: -30.0 - v, ..Default::default() });
            gate.process(&mut [&mut l[..], &mut r[..]], Some(&[&key[..]]));
            ds.set_params(&DeesserParams { threshold_db: -40.0, ..Default::default() });
            ds.process(&mut [&mut l[..], &mut r[..]]);
            let mut dp = DynamicEqParams::default();
            dp.bands[0].on = true;
            dp.bands[0].range_db = -6.0;
            dp.bands[0].freq_hz = 1000.0 + v * 50.0;
            deq.set_params(&dp);
            deq.process(&mut [&mut l[..], &mut r[..]], [Some(&[&key[..]]), None]);
            tr.set_params(&TransientParams { attack_db: 6.0, sustain_db: -3.0, speed: 0.5, mix: 1.0 });
            tr.process(&mut [&mut l[..], &mut r[..]]);
            exc.set_params(&ExciterParams { freq_hz: 4000.0, drive_db: 9.0, amount: 0.5 });
            bass.set_params(&BassParams { freq_hz: 90.0, harmonics: 0.6, mix: 0.5 });
            for c in 0..2 {
                let b = if c == 0 { &mut l } else { &mut r };
                exc.process(c, b);
                bass.process(c, b);
            }
            sw.set_params(&WidthParams { width: 1.5, mono_below_hz: 120.0 + v });
            sw.process(&mut l, &mut r);
            md.set_params(&ModParams {
                mode: [ModMode::Chorus, ModMode::Flanger, ModMode::Phaser][block % 3],
                rate_hz: 0.5,
                depth: 0.5,
                feedback: 0.4,
                voices: 3,
                stages: 8,
                spread_deg: 90.0,
                mix: 0.5,
            });
            md.process(&mut l, &mut r, block as i64 * 512);
            trem.set_params(&TremoloParams {
                mode: TremoloMode::Autopan,
                rate_hz: 4.0,
                depth: 0.5,
                shape: LfoShape::Square,
                phase_deg: 90.0,
            });
            trem.process(&mut l, &mut r, block as i64 * 512);
            let voice = HarmVoiceParams { on: true, semitones: 7.0, cents: 0.0, level_db: -6.0, pan: 0.3 };
            harm.set_params(&HarmonizerParams {
                voices: [voice, HarmVoiceParams { semitones: -5.0, ..voice }],
                mix: 0.4,
            });
            harm.process(&mut l, &mut r);
            dbl.set_params(&DoublerParams { voices: 2, detune_cents: 10.0, delay_ms: 15.0, spread: 0.8, mix: 0.5 });
            dbl.process(&mut l, &mut r, block as i64 * 512);
            meq.set_gains(&[v * 0.1; 8]);
            meq.process(&mut l, &mut r);
            wid.set_params(1.0, 100.0 + v);
            wid.process(&mut l, &mut r);
            lim.set_params(-1.0, 80.0);
            lim.process(&mut l, &mut r);
            lm.process(&l, &r);
            for &s in &l {
                tp.push(f64::from(s));
            }
            pr.feed(&l);
            cor.feed(&l, &r);
            let _ = (
                pr.take(),
                cor.take(),
                lm.integrated(),
                comp.take_gr_db(),
                lim.take_gr_db(),
                clip.take_gr_db(),
                mb.take_gr_db(),
            );
        }
    });
    assert_eq!(n, 0, "allocations inside process: {n}");
    assert!(l.iter().all(|v| v.is_finite()));
}
