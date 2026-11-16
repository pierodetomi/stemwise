//! Audio output through `cpal` (ENG-2): device and buffer choice, resampling to the device rate when it differs
//! from the project rate, and disconnection detection (UI-6). The `cpal` stream lives on its own thread.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;

use audioadapter_buffers::direct::SequentialSliceOfVecs;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::{
    Async, FixedAsync, Indexing, Resampler, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

use crate::realtime::AudioProcessor;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
pub struct DeviceInfo {
    pub name: String,
    pub default: bool,
    pub sample_rates: Vec<u32>,
    pub min_buffer: Option<u32>,
    pub max_buffer: Option<u32>,
}

/// Output devices of the default host (SET-1).
pub fn list_devices() -> Vec<DeviceInfo> {
    let host = cpal::default_host();
    let default = host.default_output_device().and_then(|d| d.description().ok()).map(|d| d.name().to_string());
    let Ok(devs) = host.output_devices() else { return Vec::new() };
    devs.filter_map(|d| {
        let name = d.description().ok()?.name().to_string();
        let mut rates = Vec::new();
        let (mut lo, mut hi) = (None, None);
        if let Ok(cfgs) = d.supported_output_configs() {
            for c in cfgs {
                for r in [44100u32, 48000, 88200, 96000] {
                    if r >= c.min_sample_rate() && r <= c.max_sample_rate() && !rates.contains(&r) {
                        rates.push(r);
                    }
                }
                if let cpal::SupportedBufferSize::Range { min, max } = c.buffer_size() {
                    lo = Some(lo.map_or(*min, |l: u32| l.min(*min)));
                    hi = Some(hi.map_or(*max, |h: u32| h.max(*max)));
                }
            }
        }
        rates.sort_unstable();
        Some(DeviceInfo {
            default: Some(&name) == default.as_ref(),
            name,
            sample_rates: rates,
            min_buffer: lo,
            max_buffer: hi,
        })
    })
    .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("no audio output device")]
    NoDevice,
    #[error("audio output: {0}")]
    Cpal(String),
}

/// Adapts the processor (project rate) to the device (its rate, its channel count). Allocation-free after `new`.
struct Adapter {
    ap: AudioProcessor,
    dev_channels: usize,
    resampler: Option<Async<f32>>,
    chunk: usize,
    in_bufs: Vec<Vec<f32>>,
    out_bufs: Vec<Vec<f32>>,
    inter: Vec<f32>,
    fifo: [Vec<f32>; 2],
    fifo_len: usize,
}

impl Adapter {
    fn new(ap: AudioProcessor, project_rate: u32, device_rate: u32, dev_channels: usize) -> Self {
        let chunk = 256;
        let resampler = (project_rate != device_rate).then(|| {
            let params = SincInterpolationParameters::new(128, WindowFunction::BlackmanHarris2)
                .oversampling_factor(128)
                .interpolation(SincInterpolationType::Cubic);
            Async::<f32>::new_sinc(
                f64::from(device_rate) / f64::from(project_rate),
                1.0,
                &params,
                chunk,
                2,
                FixedAsync::Input,
            )
            .expect("valid resampler")
        });
        let max_out = resampler.as_ref().map_or(chunk, |r| r.output_frames_max());
        Adapter {
            ap,
            dev_channels,
            resampler,
            chunk,
            in_bufs: vec![vec![0.0; chunk]; 2],
            out_bufs: vec![vec![0.0; max_out]; 2],
            inter: vec![0.0; chunk * 2],
            fifo: [vec![0.0; 16384], vec![0.0; 16384]],
            fifo_len: 0,
        }
    }

    fn fill(&mut self, out: &mut [f32]) {
        let ch = self.dev_channels.max(1);
        let frames = out.len() / ch;
        let Some(rs) = self.resampler.as_mut() else {
            if ch == 2 {
                self.ap.callback(out, 2);
            } else {
                // Mono or multichannel device: render stereo, then map (L+R)/2 or L/R on the first two.
                let mut done = 0;
                while done < frames {
                    let n = (frames - done).min(self.chunk);
                    self.ap.callback(&mut self.inter[..n * 2], 2);
                    for i in 0..n {
                        let (l, r) = (self.inter[2 * i], self.inter[2 * i + 1]);
                        let o = (done + i) * ch;
                        if ch == 1 {
                            out[o] = 0.5 * (l + r);
                        } else {
                            out[o] = l;
                            out[o + 1] = r;
                            out[o + 2..o + ch].fill(0.0);
                        }
                    }
                    done += n;
                }
            }
            return;
        };
        // Deliver in pieces no larger than half the FIFO, so any device buffer size works.
        let cap = self.fifo[0].len() / 2;
        let mut written = 0;
        while written < frames {
            let want = (frames - written).min(cap);
            while self.fifo_len < want {
                self.ap.callback(&mut self.inter, 2);
                for i in 0..self.chunk {
                    self.in_bufs[0][i] = self.inter[2 * i];
                    self.in_bufs[1][i] = self.inter[2 * i + 1];
                }
                let max = self.out_bufs[0].len();
                let (Ok(inp), Ok(mut outp)) = (
                    SequentialSliceOfVecs::new(&self.in_bufs, 2, self.chunk),
                    SequentialSliceOfVecs::new_mut(&mut self.out_bufs, 2, max),
                ) else {
                    return;
                };
                let Ok((_, produced)) = rs.process_into_buffer(&inp, &mut outp, None::<&Indexing>) else { return };
                let n = produced.min(self.fifo[0].len() - self.fifo_len);
                for c in 0..2 {
                    self.fifo[c][self.fifo_len..self.fifo_len + n].copy_from_slice(&self.out_bufs[c][..n]);
                }
                self.fifo_len += n;
            }
            for i in 0..want {
                let o = (written + i) * ch;
                let (l, r) = (self.fifo[0][i], self.fifo[1][i]);
                if ch == 1 {
                    out[o] = 0.5 * (l + r);
                } else {
                    out[o] = l;
                    out[o + 1] = r;
                    out[o + 2..o + ch].fill(0.0);
                }
            }
            for c in 0..2 {
                self.fifo[c].copy_within(want..self.fifo_len, 0);
            }
            self.fifo_len -= want;
            written += want;
        }
    }
}

/// Sets flush-to-zero (and denormals-are-zero on x86) on the calling thread: the audio thread then never pays
/// for denormals, also on Intel Macs (P-002). The offline render does not need it (determinism is per thread).
fn set_flush_to_zero() {
    #[cfg(target_arch = "aarch64")]
    // SAFETY: FPCR bit 24 (FZ) only changes how denormal results are flushed on this thread.
    unsafe {
        let mut fpcr: u64;
        std::arch::asm!("mrs {}, fpcr", out(reg) fpcr);
        std::arch::asm!("msr fpcr, {}", in(reg) fpcr | (1 << 24));
    }
    #[cfg(target_arch = "x86_64")]
    // SAFETY: MXCSR FTZ (bit 15) and DAZ (bit 6) only affect denormal handling on this thread.
    #[allow(deprecated)]
    unsafe {
        use std::arch::x86_64::{_mm_getcsr, _mm_setcsr};
        _mm_setcsr(_mm_getcsr() | 0x8040);
    }
}

/// An open output stream. Dropping it stops the stream.
pub struct Output {
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    pub device: String,
    pub device_rate: u32,
    pub buffer: Option<u32>,
    pub disconnected: Arc<AtomicBool>,
}

impl Output {
    /// Opens `device` (or the default) at the project rate if it supports it, else at its default rate with
    /// resampling (ENG-2); `buffer` frames if given.
    pub fn open(
        ap: AudioProcessor,
        project_rate: u32,
        device: Option<&str>,
        buffer: Option<u32>,
    ) -> Result<Output, DeviceError> {
        let (tx, rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(String, u32), DeviceError>>();
        let disconnected = Arc::new(AtomicBool::new(false));
        let disc = disconnected.clone();
        let want = device.map(str::to_string);
        let thread = std::thread::Builder::new()
            .name("stemwise-output".into())
            .spawn(move || {
                let host = cpal::default_host();
                let dev = match &want {
                    Some(name) => host.output_devices().ok().and_then(|mut it| {
                        it.find(|d| d.description().ok().is_some_and(|x| x.name() == name.as_str()))
                    }),
                    None => None,
                }
                .or_else(|| host.default_output_device());
                let Some(dev) = dev else {
                    let _ = ready_tx.send(Err(DeviceError::NoDevice));
                    return;
                };
                let name = dev.description().map(|d| d.name().to_string()).unwrap_or_default();
                let def = match dev.default_output_config() {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = ready_tx.send(Err(DeviceError::Cpal(e.to_string())));
                        return;
                    }
                };
                let supports = dev.supported_output_configs().is_ok_and(|mut it| {
                    it.any(|c| project_rate >= c.min_sample_rate() && project_rate <= c.max_sample_rate())
                });
                let rate = if supports { project_rate } else { def.sample_rate() };
                let channels = def.channels();
                // A buffer size the device does not support falls back to its default (review F3, N3).
                let fits = |b: u32| match def.buffer_size() {
                    cpal::SupportedBufferSize::Range { min, max } => b >= *min && b <= *max,
                    _ => false,
                };
                let config = cpal::StreamConfig {
                    channels,
                    sample_rate: rate,
                    buffer_size: buffer.filter(|b| fits(*b)).map_or(cpal::BufferSize::Default, cpal::BufferSize::Fixed),
                };
                let mut adapter = Adapter::new(ap, project_rate, rate, usize::from(channels));
                let disc2 = disc.clone();
                let mut ftz = false;
                let stream = dev.build_output_stream::<f32, _, _>(
                    config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        if !ftz {
                            set_flush_to_zero();
                            ftz = true;
                        }
                        adapter.fill(data);
                    },
                    move |e: cpal::Error| {
                        if matches!(e.kind(), cpal::ErrorKind::DeviceNotAvailable | cpal::ErrorKind::DeviceChanged) {
                            disc2.store(true, Ordering::Relaxed);
                        }
                    },
                    None,
                );
                let stream = match stream {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = ready_tx.send(Err(DeviceError::Cpal(e.to_string())));
                        return;
                    }
                };
                if let Err(e) = stream.play() {
                    let _ = ready_tx.send(Err(DeviceError::Cpal(e.to_string())));
                    return;
                }
                let _ = ready_tx.send(Ok((name, rate)));
                let _ = rx.recv();
                drop(stream);
            })
            .map_err(|e| DeviceError::Cpal(e.to_string()))?;
        let (device, device_rate) = ready_rx.recv().map_err(|e| DeviceError::Cpal(e.to_string()))??;
        Ok(Output { stop: Some(tx), thread: Some(thread), device, device_rate, buffer, disconnected })
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        if let Some(t) = self.stop.take() {
            let _ = t.send(());
        }
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realtime::Engine;

    /// req: ENG-2
    #[test]
    fn output_adapter_delivers_exact_frames_at_any_rate_and_channel_count() {
        // Drives the adapter as a 44.1 kHz device would, with a 48 kHz project playing nothing: the adapter
        // must deliver exactly the requested frames, silent, without panicking.
        let (_e, ap) = Engine::new(48000.0, 4096);
        let mut a = Adapter::new(ap, 48000, 44100, 2);
        for n in [512usize, 441, 1024, 37] {
            let mut out = vec![1.0f32; n * 2];
            a.fill(&mut out);
            assert!(out.iter().all(|v| *v == 0.0));
        }
        let (_e, ap) = Engine::new(48000.0, 4096);
        let mut mono = Adapter::new(ap, 48000, 48000, 1);
        let mut out = vec![1.0f32; 300];
        mono.fill(&mut out);
        assert!(out.iter().all(|v| *v == 0.0));
    }

    /// req: ENG-2
    #[test]
    fn output_resampling_keeps_pitch_and_level() {
        use stemwise_core::history::{Document, Phase};
        use stemwise_core::import::{ImportChoice, import};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
        std::fs::create_dir_all(&root).unwrap();
        let t = tempfile::tempdir_in(root).unwrap();
        let src = t.path().join("s.wav");
        let x: Vec<f32> = (0..4 * 48000)
            .map(|i| (0.5 * (std::f64::consts::TAU * 1000.0 * i as f64 / 48000.0).sin()) as f32)
            .collect();
        stemwise_core::wav::write_f32(&src, 1, 48000, &x).unwrap();
        let (dir, p) = stemwise_core::package::create(t.path(), "Rs").unwrap();
        let mut d = Document::new(p);
        let choice = ImportChoice {
            path: src.display().to_string(),
            role: stemwise_core::role::Role::Other,
            name: "s".into(),
            convert: false,
        };
        let cmds = import(&dir, &mut d.project, &[choice]).unwrap();
        d.transact(Phase::Mix, "import", cmds, None).unwrap();
        let (mut e, ap) = Engine::new(48000.0, 4096);
        e.load(&dir, &d.project).unwrap();
        e.play();
        let mut a = Adapter::new(ap, 48000, 44100, 2);
        let mut left: Vec<f32> = Vec::new();
        for _ in 0..300 {
            let mut out = vec![0f32; 441 * 2];
            a.fill(&mut out);
            left.extend(out.iter().step_by(2));
            e.poll();
            std::thread::sleep(std::time::Duration::from_micros(500));
        }
        // One second of steady output at 44.1 kHz, after the start.
        let w = &left[left.len() - 44100 - 4410..left.len() - 4410];
        let crossings = w.windows(2).filter(|p| p[0] < 0.0 && p[1] >= 0.0).count();
        assert!((crossings as i64 - 1000).abs() <= 1, "{crossings} cycles in 1 s");
        let rms = (w.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / w.len() as f64).sqrt();
        // Mono at the centre: 0.5 · √½ peak per channel, so RMS 0.25; tolerance 0.05 dB (resampler passband).
        assert!((20.0 * (rms / 0.25).log10()).abs() < 0.05, "{rms}");
    }

    /// req: ENG-2, ENG-1
    /// Smoke test on the real device (FOUNDATION §8.4): opens the default output for 100 ms with the transport
    /// stopped, so it plays silence. Run by hand at the F3 gate: `cargo test -p stemwise-engine -- --ignored`.
    #[test]
    #[ignore]
    fn real_device_smoke() {
        // A project playing with the bus fader at −∞: the transport runs on the real device, silently.
        let devs = list_devices();
        assert!(!devs.is_empty(), "no output device");
        let mut p = stemwise_core::model::Project::new("Smoke");
        p.sample_rate = Some(48000);
        p.mix_bus.fader_db = -144.0;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
        std::fs::create_dir_all(&root).unwrap();
        let t = tempfile::tempdir_in(root).unwrap();
        let (mut e, ap) = Engine::new(48000.0, 8192);
        e.load(t.path(), &p).unwrap();
        let out = Output::open(ap, 48000, None, Some(512)).expect("open");
        e.play();
        std::thread::sleep(std::time::Duration::from_millis(300));
        e.poll();
        assert!(!out.disconnected.load(Ordering::Relaxed));
        assert!(
            e.playing() && e.shared.position.load(Ordering::Relaxed) > 4800,
            "the transport advances on the device"
        );
        assert_eq!(e.shared.xruns.load(Ordering::Relaxed), 0);
        assert!(e.load_ratio() < 1.0);
        eprintln!("device {} at {} Hz; devices: {devs:?}", out.device, out.device_rate);
    }
}
