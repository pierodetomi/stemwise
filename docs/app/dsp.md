# Processori DSP

Documentazione dei processori del crate `stemwise-dsp` (FOUNDATION §5.7: ogni processore senza oracolo è
descritto qui con algoritmo, parametri e proprietà verificate). Il testo di ciascuna sezione è il commento di
modulo del file sorgente, in inglese come il codice; questo file si rigenera copiandolo.

Convenzioni comuni: calcolo interno in f64 (f32 nell'oversampling, A-022), uscita f32; nessuna allocazione, lock
o I/O in `process` (verificato da `crates/stemwise-dsp/tests/no_alloc.rs`, NFR-2); parametri aggiornabili ogni 32
campioni (`util::SUBBLOCK`) con smoothing per campione, quindi risultato indipendente dalla dimensione del blocco
(ENG-4). Parità con l'oracolo della PoC: biquad e pan (±0,01 dB), loudness integrato e LRA (±0,1 LU), true peak
(±0,1 dB; misurato entro 0,01 dB) su tutti i casi di `app/fixtures/oracle`.

## Biquad RBJ (FX-2)

Sorgente: [`biquad.rs`](../../app/crates/stemwise-dsp/src/biquad.rs).

RBJ biquads (FX-2): the same formulas as the PoC (poc/docs/analysis.md, "Filtri ed EQ").

Coefficients are computed and the filter runs in f64, transposed direct form II, one state per channel.

## Compressore (FX-3, MAS-1)

Sorgente: [`compressor.rs`](../../app/crates/stemwise-dsp/src/compressor.rs).

Feed-forward compressor with a stereo-linked detector (FX-3, MAS-1; closes D-034 of the PoC).

- Detector: per sample, the larger absolute value over the channels of the key signal (the processed signal
  itself, or a sidechain source through a Butterworth high-pass). One detector for both channels: stereo is
  compressed in a linked way.
- Gain computer (in dB) with a soft knee of width `W` (Giannoulis, Massberg & Reiss 2012):
  below `T − W/2` no change; above `T + W/2`, `T + (x − T)/R`; in between the quadratic
  `x + (1/R − 1)(x − T + W/2)² / (2W)`.
- Ballistics: smoothed gain reduction in dB, branching one-pole: attack coefficient `exp(−1/(τa·fs))` when the
  reduction grows, release `exp(−1/(τr·fs))` when it shrinks.
- Makeup: a gain after the compressor, smoothed per sample (20 ms) so changes do not click.
- Mix (dry/wet, tracks and groups; P-011): the output gain is `1 + mix·(g − 1)`, that is `x·(1 − mix) + x·g·mix` on the
  same sample, so the dry and the compressed signal never comb. Smoothed like the makeup. At 1 the code path is the
  one without a mix; the master compressor is always at 1.
- Meter: the deepest reduction since the last `take_gr_db`.
- Filtro passa-alto del rivelatore (MAS-13): senza chiave di sidechain, `detector_hpf_hz > 0` fa ascoltare al
  rivelatore il segnale attraverso un Butterworth del 2° ordine a quella frequenza (stato proprio, per canale); il
  percorso audio non è toccato. 0 lo disattiva (predefinito). Con una chiave resta in vigore `sidechain_hpf_hz`.
  Verificato: un seno a 50 Hz a −6 dBFS (soglia −20 dB, rapporto 4) ha almeno 6 dB di riduzione in meno con 200 Hz,
  un seno a 2 kHz è ridotto allo stesso modo (±0,2 dB).

Properties verified by the tests: static curve and knee, attack and release times, linking, sidechain with
its high-pass, makeup, mix (0 is the input, 0.5 the mean of dry and wet at every sample, smoothed changes),
stability and block-size invariance.

## Saturazione (FX-4)

Sorgente: [`saturation.rs`](../../app/crates/stemwise-dsp/src/saturation.rs).

Soft saturation with 4x oversampling (FX-4).

**Algorithm.** Per sample: input gain `g = 10^(drive_db/20)`, shaper `y = shape(g * x) * c(g)` with
`shape(u) = 2 tanh(u / 2)` (unity small-signal slope; ceiling 2 keeps the knee soft enough for alias rejection) evaluated at 4x
through [`Oversampler4`] (257-tap Kaiser linear-phase FIR, ~90 dB stopband, see that module), then
`out = dry_delayed * (1 - mix) + y * mix`.

**Level compensation.** `c(g) = A / shape(g * A)` with reference peak `A = 10^(-18/20)` (the level of a
-18 dBFS sine), computed analytically per sample from the smoothed gain: the shaped level at the reference
amplitude equals the unshaped one. At drive 0 dB `c` is about 1.0, so low-level signals pass transparently.

**Smoothing.** `drive` (linear gain) and `mix` are smoothed per sample with a 5 ms one-pole towards the
targets set by `set_params`, independently per channel; output is therefore independent of block size.

**Latency.** [`Saturation::LATENCY`] = [`Oversampler4::LATENCY`] = 64 samples, integer; the dry path goes
through a 64-sample delay so that `mix` blends phase-aligned signals. Engine compensates it (ENG-5).

**Properties verified (tests).** Exact latency (peak and mix 0), transparency at -40 dBFS (+-0.1 dB), level
within +-1 dB for drive 0..18 dB at -18 dBFS, 3rd harmonic present at drive 18 dB, aliasing of the 3rd harmonic
of 15 kHz at 48 kHz below -60 dB, no NaN/inf at drive 24 dB on full-scale noise, block-size invariance
(<= 1e-6), independent channels. Process does not allocate, lock or perform I/O.

## Oversampling 4x

Sorgente: [`oversample.rs`](../../app/crates/stemwise-dsp/src/oversample.rs).

4x oversampler (one channel): two cascaded 2x half-band stages, linear phase, around a nonlinear callback.

**Algorithm.** Each stage is a Kaiser-windowed half-band FIR (`h[c] = ½`, zero at the other even offsets, odd
taps `½·sinc((k − c)/2)·w[k]` normalised so they sum to ½), in polyphase form: upsampling copies the centre
sample and computes one dot product of the odd taps; downsampling computes one dot product of the odd taps over
the odd-phase history plus the centre term. Arithmetic in f32 with contiguous, multi-accumulator dot products
(vectorisable), histories mirrored to avoid modulo.
- Stage 1 (base ↔ 2x): `c = 56` odd taps (113-tap prototype), Kaiser β 8.2: passband to 20 kHz and stopband
  from 24.1 kHz at 44.1 kHz (≥ 80 dB).
- Stage 2 (2x ↔ 4x): `c = 14` odd taps (29-tap prototype), Kaiser β 7.9: only has to reject the images above
  68.2 kHz (≥ 80 dB); what folds into 20–44 kHz at 2x is removed by stage 1 on the way down.

**Latency.** Stage 1 delays by `c₁ = 56` base samples round trip, stage 2 by `c₂/2 = 7`; a 1-sample delay line
pads the total to exactly [`Oversampler4::LATENCY`] = 64 base samples, a multiple of the engine's 32-sample
parameter grid (ENG-5, A-020).

**Cost.** About 160 multiply-adds per base sample instead of ~520 for a single 257-tap 4x FIR (A-022).

**Properties verified (tests).** Identity round trip flat within 0.1 dB at 1 kHz and 18 kHz (48 kHz), impulse
peak exactly at `LATENCY`, stopbands of both stages ≥ 80 dB, reset clears the state.

## Riverbero (FX-5, FX-9)

Sorgente: [`reverb.rs`](../../app/crates/stemwise-dsp/src/reverb.rs); misure dei test in
[`revmeasure.rs`](../../app/crates/stemwise-dsp/src/revmeasure.rs). Sostituisce il riverbero a pettini della prima
consegna (P-011, A-058).

Three own implementations of published designs, chosen per return (the algorithm is fixed when the graph is built:
changing it is a structural change).

- **Plate**: Dattorro's plate ("Effect Design, Part 1", JAES 1997). Mono sum → input bandwidth low-pass → 4 input
  allpass diffusers (142, 107, 379, 277 samples at 29761 Hz; 0.75, 0.75, 0.625, 0.625) → a figure-eight tank of two
  halves, each a modulated allpass (672 / 908, coefficient 0.7), a delay (4453 / 4217), a damping shelf, a second
  allpass (1800 / 2656, coefficient 0.5) and a second delay (3720 / 3163). Output: 7 taps per side across the tank.
  One departure from the paper: the four taps it takes inside the tank allpasses are taken from the delay line that
  follows each of them, because the inside of an allpass is a comb and its period was audible in the tail.
- **Room** and **Hall**: a feedback delay network of 16 lines mixed by a Hadamard matrix (lossless before the per-line
  attenuation). Lines of 6.7–40.9 ms (room) and 21.1–96.7 ms (hall) at scale 1. Fed in stereo (left on even lines,
  right on odd ones) through 2 (room) or 4 (hall) allpass diffusers, plus 6 early reflections per side taken before
  the diffusers (3–25 ms in the room, 2.4 times later and softer in the hall). Input and output sign patterns are
  *bent* sequences: their Hadamard transform is flat, so no single line comes back into the output more than the
  others (with a row of the matrix as the output pattern one line returned whole: a peak of 0.2 in the
  autocorrelation at that line's length).

Parameters (the user's ranges; the AI plan has tighter ones, FOUNDATION §5.5):

| Parameter | Range | Meaning |
| --- | --- | --- |
| `decay_s` | 0.1–10 s | RT60 at mid frequencies: every loop element of length `L` is attenuated by `10^(−3·L/(rate·decay_s))` |
| `size` | 0–1 | Scale of the loop lengths: network 0.6–1.4×, plate 0.5–1.25× |
| `damping` | 0–1 | First-order shelf at 5 kHz in every loop: the high-frequency decay is `decay_s·(1 − 0.85·damping)` |
| `modulation` | 0–1 | Slow movement (0.5–1.3 Hz, up to ±1 ms) of 4 of the 16 lines, or of the two tank allpasses |
| `predelay_ms` | 0–200 ms | Before everything else |
| `hpf_hz`, `lpf_hz` | 20–1000 Hz, 1–20 kHz | Butterworth on each output channel |

Details that matter:

- **The plate shrinks with short decays.** The sound goes around the tank at least five times within `decay_s`, so
  below about 3 s the decay sets the plate's size and `size` is only an upper bound. Measured: energy takes 1.09 times
  the sum of the tank lengths to go around once; the attenuation uses that figure.
- **Under about 1 s the plate is grainy by construction**: its tank allpasses show as a fast flutter (their impulse
  response is an echo train that only later trips around the tank diffuse). A room is the better short space; the
  interface says so under the algorithm selector and the prompt tells the model.
- **Deterministic modulation.** Recursive sine oscillators ("magic circle") with fixed starting phases, restarted by
  `reset`: no random source. Two reverbs given the same input give the same samples, and so does one reverb after a
  reset.
- **Smoothing.** Gains, damping, pre-delay and level with a one-pole (30 ms); loop lengths with a slower one (120 ms)
  that lands exactly on whole samples, so a settled reverb reads its delays without interpolation (interpolated only
  while `size` moves, and on the modulated lines).
- **Level.** The output gain follows `sqrt(scale)·decay_s^−0.25`: the return grows gently with the decay (about
  +0.75 dB per doubling) instead of 3 dB. Calibrated with stereo pink noise as the send: +2 dB over the send at 1.5 s
  for the three algorithms.
- All arithmetic in f64; state flushed so tails end in exact zeros. 100% wet.

**Measured** (48 kHz, impulse response; `repeat` is the autocorrelation of the flattened tail that comes back at a lag
and again at twice that lag, 2–150 ms: what a comb leaves):

| | RT60 vs `decay_s` (0.5–6 s, no damping) | `repeat` at 2 s / 4.6 s | L/R correlation of the tail |
| --- | --- | --- | --- |
| Plate | within 5% | 0.06–0.10 / 0.05 (0.13–0.3 under 1 s) | below 0.03 |
| Room | within 3% | 0.07–0.08 | below 0.11 |
| Hall | within 3% (0.3 s: +11%) | 0.05–0.06 | below 0.08 |
| Comb reverb it replaces | — | 0.16–0.20 / 0.14 | below 0.02 |

With the default damping (0.5) and modulation (0.2) the mid-band RT60 is 3–9% short (the shelf and the interpolated
reads take a little from 2 kHz).

**Properties verified by the tests (FX-9).** Decay time within ±10% of `decay_s` at 0.5, 1.5 and 4 s with no damping
(±15% at the defaults); `repeat` below 0.11 at 2 and 4 s; |L/R correlation| below 0.3 with a mono send; highs at least
20% shorter than mids at damping 0.5 and within 10% at 0; pre-delay exact to the sample; high-pass and low-pass
effective (≥ 15 dB); calibration levels (±1.5 dB) growing with the decay; exact silence after the tail; determinism
and restart on reset; the same samples with blocks of 64 and 4096; bounded output with full-scale noise at the longest
decay and with every parameter changing every 32 samples; no allocation in `process` (`tests/no_alloc.rs`). In the
engine: the same hash with blocks of 64, 1000 and 4096 with the three algorithms sending at once, and NFR-1 with four
reverbs added to the benchmark (p99 18% of the buffer period).

**Not verifiable by the agent.** How they sound: `docs/app/VALIDATION.md`, points 25–26.

## Delay (FX-6)

Sorgente: [`delay.rs`](../../app/crates/stemwise-dsp/src/delay.rs).

Stereo delay return (FX-6).

Structure (f64 arithmetic, state flushed with [`flush`] so tails end in exact zeros):
- two independent delay lines (L, R) with the same time, buffers for 2.1 s, written with
  `input + feedback · filtered(read)`; the output is the delayed signal (100% wet);
- feedback path: RBJ Butterworth (Q 1/√2) high-pass at `feedback_hpf_hz` then low-pass at `feedback_lpf_hz`;
  `feedback` is clamped to 0..0.95;
- time changes glide: the delay time (in samples) follows its target with a one-pole of about 50 ms per sample,
  and the line is read with linear interpolation at the fractional position, so sweeps do not click and the
  result does not depend on the block size. The first `set_params` call (and `reset`) snaps the smoothed time.
  Filter coefficients are redesigned in `set_params` only when their frequency changes.

Properties verified by the tests: echo position `round(time · rate)` (±1 sample), echo decay of
`20 · log10(feedback)` dB per repeat with open filters (±0.5 dB), boundedness at feedback 0.95 with full-scale
noise, no clicks while sweeping the time, block-size invariance, exact silence after the tail.

## Limiter true peak (MAS-1, MAS-3)

Sorgente: [`limiter.rs`](../../app/crates/stemwise-dsp/src/limiter.rs).

True-peak limiter with look-ahead (MAS-1, MAS-3).

1. Detection: each channel goes through the 4x true-peak interpolator of `truepeak` (the PoC's filter); the
   detector value for a sample is the largest of its four interpolated points, over both channels.
2. Required gain `r = min(1, c / tp)`, with `c` the ceiling lowered by an internal margin of 0.1 dB (covers
   the inter-sample effect of a moving gain and 24-bit quantisation).
3. Hold: the minimum of `r` over the last `L` samples (`L` = 2 ms of look-ahead).
4. Attack: a moving average of the held value over `L` samples. Every average that includes a peak's sample
   is made of values ≤ that peak's `r`, so the gain applied to the peak (the signal is delayed by `L − 1`) is
   at most `r`: the ceiling holds by construction, with a smooth `L`-sample ramp.
5. Release: when the gain rises, it follows with a one-pole of `release_ms`; falling is never slowed.

Total latency [`Limiter::latency`] = 10 (detector) + `L − 1` samples; the engine compensates it (ENG-5).
The guarantee is checked again on the exported file (`truepeak::true_peak_db`), which the export repeats
with a lower ceiling in the rare case it is exceeded (MAS-3, MAS-10).

## Dither TPDF (MAS-1, MAS-7)

Sorgente: [`dither.rs`](../../app/crates/stemwise-dsp/src/dither.rs).

TPDF dither for 16-bit export (MAS-1, MAS-7): the sum of two independent uniform values of ±½ LSB, from a
seeded xorshift32 generator, so the same master gives the same file (MAS-8).

## Match EQ e ampiezza stereo (MAS-1)

Sorgente: [`master.rs`](../../app/crates/stemwise-dsp/src/master.rs).

Master-only blocks (MAS-1): match EQ towards a reference profile and stereo width.

## Catena di master (MAS-1, MAS-5; ENG-3)

Sorgente: [`master.rs`](../../app/crates/stemwise-engine/src/master.rs) del motore.

The chain is: input gain, 6-band EQ, match EQ, compressor, width, saturation, loudness gain, limiter, then the
listening trim. New parameters never step inside the signal:

- **Continuous values glide** every 32 samples towards their target with a 10 ms one-pole. These are the gains,
  the EQ bands (frequency and Q in the log domain; a change of band type goes through 0 dB), the match EQ gains, the
  side gain and the mono frequency.
- **Gains ramp per sample** within the 32-sample sub-block.
- **Switching a block on or off** is a cross-fade between its input and its output. The A/B against the unprocessed
  input is a cross-fade too.
  - EQ, match EQ and width glide to the identity, then stop running with their state cleared.
  - Saturation and limiter keep running when off, so a fade-in never starts from an empty oversampler or look-ahead.
  - Their dry delay lines always run, so a fade never plays stale audio.
- **With constant parameters** the glides start at their target, so offline renders are unchanged and deterministic.

Verified:

- `master_changes_glide_instead_of_stepping`: the largest step between consecutive samples after each kind of change
  stays near that of the sine itself.
- `mono_below_removes_the_side_of_the_low_end_only`: side −24 dB an octave below the crossover, mid within 1 %.

## Meter (TRK-2, MAS-4)

Sorgente: [`meters.rs`](../../app/crates/stemwise-dsp/src/meters.rs).

Meters (TRK-2, MAS-4): peak and RMS per channel, stereo correlation, and a spectrum analyser.

The audio thread only updates the cheap accumulators (`PeakRms`, `Correlation`); the spectrum is computed off
the audio thread from samples copied into a lock-free ring (ENG-6), so its FFT never runs in the callback.

## Loudness (ANA-1, ANA-2, MAS-4)

Sorgente: [`loudness.rs`](../../app/crates/stemwise-dsp/src/loudness.rs).

Loudness (ITU-R BS.1770-4, EBU Tech 3342): offline measures identical to the PoC (`pyloudnorm` 0.2,
poc/docs/analysis.md "Loudness") and a real-time meter for the master (MAS-4).

## True peak (ANA-1, MAS-3)

Sorgente: [`truepeak.rs`](../../app/crates/stemwise-dsp/src/truepeak.rs).

True peak with 4x oversampling (ANA-1, MAS-3): the PoC's interpolator (poc/docs/analysis.md "True peak"):
81-tap Kaiser (β = 5) windowed sinc, cut-off at the original Nyquist, gain 4, zero-phase, silence outside.

## Clipper (MAS-11)

Sorgente: [`clipper.rs`](../../app/crates/stemwise-dsp/src/clipper.rs).

Clipper di picco con oversampling 4x (lo stesso `Oversampler4` della saturazione, latenza 64 campioni, intera).
La forma è calcolata alla frequenza 4x su ciascuno dei quattro punti, dispari-simmetrica, con soglia `T` e inizio
del ginocchio `a = T·(1 − 0,5·softness)`:

- `|x| ≤ a`: `|x|` (nessuna compensazione di livello, guadagno unitario sotto il ginocchio);
- sopra: `T − (T − a)·exp(−(|x| − a)/(T − a))`, continua con derivata continua in `a`, asintoto `T`;
- `softness = 0`: clip netto `min(|x|, T)`.

Parametri: `threshold` (ampiezza lineare, > 0) e `softness` (0…1), smussati per campione con un polo a 5 ms,
indipendentemente per canale; `settle` salta ai valori obiettivo. Il misuratore `take_gr_db` dà il guadagno più
profondo applicato a un campione 4x dall'ultima chiamata (≤ 0 dB, 0 se nulla è stato tagliato).

Il segnale 4x non supera mai `T`; dopo la decimazione il filtro a semibanda può sovraelongare di una frazione di dB
(Gibbs su un'onda a cima piatta): i test ammettono 0,5 dB.

Verificato: latenza esatta, trasparenza sotto il ginocchio (±0,01 dB), picco ≤ `T` + 0,5 dB con un seno sovraccarico
di 12 dB, misuratore ≈ −6,02 dB su un picco noto (±0,3 dB) e 0 sotto il ginocchio, terza armonica più bassa con
`softness` 1 che con 0, aliasing di un seno a 15 kHz (48 kHz) sotto −50 dB, invarianza rispetto al blocco (blocchi 1,
64, 1000, ≤ 1e-6), canali indipendenti, nessun NaN su rumore a fondo scala con soglia 0,01. `process` non alloca.

## Compressore multibanda (MAS-12)

Sorgente: [`multiband.rs`](../../app/crates/stemwise-dsp/src/multiband.rs).

Compressore a 2–6 bande, latenza zero. Il crossover è un Linkwitz-Riley del 4° ordine (due biquad Butterworth in
cascata per pendenza) disposto ad albero: la banda più bassa si stacca per prima, `band j = LP(fj)(resto j)`,
`resto j+1 = HP(fj)(resto j)`, l'ultima banda è il resto. La somma di un LR4 passa-basso e passa-alto alla stessa
frequenza è il passa-tutto del 2° ordine `(s² − √2 s + 1)/(s² + √2 s + 1)`: ogni banda staccata a `fj` passa quindi
attraverso un passa-tutto (Q = 1/√2) a ciascuna frequenza di crossover successiva, così la somma di tutte le bande è
`AP(f0)·…·AP(f(N−2))·x`, un passa-tutto dell'ingresso (modulo piatto) per ogni numero di bande.

Ogni banda passa per un `Compressor` collegato in stereo (riuso di `compressor.rs`, comprese `mix` e
`detector_hpf_hz`); una banda con `on = false` passa non compressa; se qualche banda ha `solo`, si sommano solo le
bande in solo. I crossover sono limitati a 20 Hz … 0,45·frequenza di campionamento, crescenti e distanti almeno 1/3
di ottava. Cambiare il numero di bande azzera i filtri; cambiare le frequenze sostituisce solo i coefficienti. Tutta
la memoria per 6 bande è allocata in `new`; `process` lavora a blocchi di 256 campioni su buffer preallocati.

Verificato: somma piatta entro ±0,1 dB tra 30 Hz e 18 kHz per 2, 3, 4, 5 e 6 bande; isolamento (50 Hz a −6 dBFS
compresso nella banda 0, riduzione solo lì, 5 kHz mescolato entro 0,2 dB); solo (oltre 24 dB di attenuazione
un'ottava dopo il crossover); banda spenta; cambi di crossover tra i blocchi senza NaN; limiti e ordine dei
crossover; invarianza rispetto al blocco (1, 64, 1000, ≤ 1e-6); collegamento stereo; nessuna allocazione
(`tests/no_alloc.rs`).

## Catena a blocchi (FX-10 → FX-12)

- **Ordine e latenza.** I blocchi di una traccia girano nell'ordine della lista. Ogni tipo ha una latenza fissa in
  campioni, multipla della griglia di 32 (saturazione e gli altri blocchi sovracampionati: 64). Ogni catena di tracce
  e gruppi è portata da una linea di ritardo alla latenza della catena più lenta del progetto; i ritorni alla latenza
  del ritorno più lento. I parametri di un blocco sono valutati al tempo dell'audio che toccano: latenza d'ingresso del
  nodo più la latenza dei blocchi prima di lui.
- **Bypass.** Il valore scorre linearmente in 10 ms (`FADE_S`), per campione, con stato portato tra i blocchi di
  calcolo: il risultato non dipende dalla loro dimensione. Un blocco del tutto spento si salta, uno del tutto acceso
  non si miscela. **Se il processore dorme si decide una volta per cella di 32 campioni**, dallo stato della
  dissolvenza all'inizio della cella: la dissolvenza dura 481 campioni a 48 kHz e finisce a metà cella, e decidere
  per pezzo faceva dipendere lo stato congelato dal punto in cui il blocco di calcolo tagliava la cella (A-090). Un
  blocco che si riaccende **riparte da uno stato pulito** (linee e filtri azzerati, fase degli LFO ridata dalla
  posizione): non suona l'audio che aveva dentro quando si è spento. Proprietà verificate: con bypass 0 → 1 → 0 su
  passa-basso, compressore, harmonizer e doubler il render è identico bit a bit con blocchi da 4096, 64, 777, 1000 e
  da 12 481 a 12 511 (il confine cade nella cella dove finisce la dissolvenza); un doubler spento sul rumore e
  riacceso nel silenzio dà silenzio esatto. Un blocco con latenza miscela l'uscita con il proprio ingresso
  ritardato della stessa latenza; da spento passa l'ingresso ritardato.
- **Filtri risonanti (FX-12).** Biquad RBJ passa-alto e passa-basso con Q = `resonance` (0,5–10; 1/√2 è il Butterworth
  di prima). Proprietà verificata: alla frequenza di taglio il guadagno è Q (−3 dB a 1/√2, +18 dB a Q 8, ±5%).
- **Mono → stereo.** Il primo blocco stereo copia il canale nel secondo; il pan finale usa la legge mono su ciascun
  canale, così un blocco stereo neutro non cambia il livello.

## Blocchi nuovi delle tracce (FX-13 → FX-24)

Regole comuni (FX-24), verificate per ogni blocco dai test del suo modulo: in posizione neutra il segnale non cambia
(≤ 1e-6, a parte la latenza costante); nessuna allocazione in `process` (`tests/no_alloc.rs`); risultato indipendente
dalla dimensione dei blocchi di calcolo (chunk di 32, 1000 e 7 campioni, ≤ 1e-6); stabile con parametri che cambiano
ogni 32 campioni; uscita limitata con rumore a piena scala. Nel motore: tutti i blocchi in posizione neutra lasciano il
render identico; una catena con tutti i blocchi al lavoro non dipende dal blocco di calcolo; modulazione, tremolo e
delay tape danno dal secondo 4 gli stessi campioni del render intero (≤ 1e-4: le fasi si accumulano per campione da
un punto di ancoraggio diverso, l'arrotondamento differisce di qualche 1e-5). Latenze: saturazione, clipper, exciter e
bass enhancer 64 campioni; harmonizer 864 a 48 kHz (finestra di 35 ms, multiplo di 32); tutti gli altri 0.
Benchmark (FX-24): 32 tracce con la catena classica e 8 con harmonizer, multibanda a quattro bande e chorus: p99 del
callback 27% del periodo (soglia 50%).

### Gate / expander (FX-13)

Rivelatore: massimo del valore assoluto sui canali della chiave (il segnale stesso o il sidechain: **tutti** i canali della chiave anche quando il segnale è mono, così una sorgente tutta a destra lo apre lo stesso; vale anche per l'EQ dinamico, A-090), attraverso passa-alto e passa-basso Butterworth del 2° ordine (20 Hz e 20 kHz = spenti), poi un inseguitore di picco (attacco istantaneo, decadimento 10 ms). Curva (dB): sopra la soglia 0 dB; sotto `−(ratio − 1)·(soglia − livello)` limitato a `−range` (a ratio 20 raggiunge il range). Hold: dopo la discesa sotto soglia il guadagno resta a 0 dB per `hold_ms`. Il guadagno in dB insegue il bersaglio con un polo singolo per campione (`attack_ms` in apertura, `release_ms` in chiusura). Latenza 0; misuratore: guadagno più profondo dall'ultima `take_gr_db`.

Parametri: soglia, range 0–80 dB, ratio 1–20, attacco 0,05–50 ms, hold 0–500 ms, rilascio 5–2000 ms, HPF 20–2000 Hz, LPF 1–20 kHz.

Proprietà verificate: regime sotto soglia = −range ±0,5 dB (sinusoide a −60 dBFS, soglia −30, range 40); sopra la soglia invariato (differenza ≤ 1e-4 dopo 100 ms); range 0 = identità (≤ 1e-6); la chiave del sidechain apre il gate (±0,1 dB) mentre senza chiave chiude (< −39 dB); hold di 200 ms mantiene aperto, poi chiude a −range ±0,5 dB; misuratore ≈ −range ±0,5 dB; stesso risultato con blocchi da 32, 1000 e 7 (≤ 1e-6); rumore a fondo scala con parametri che cambiano ogni 32 campioni: finito e |uscita| ≤ |ingresso| (il guadagno non supera 0 dB).

### De-esser (FX-14)

Divisione: `high = x − LP(x)`, con LP Butterworth del 2° ordine alla frequenza scelta; la frequenza è lisciata (polo singolo da 20 ms) su una griglia di 32 campioni contata dal `reset`, quindi indipendente dal blocco. Rivelatore: `high` attraverso un passa-alto Butterworth del 2° ordine alla stessa frequenza (il solo complemento scende di 6 dB per ottava: una vocale forte a 2 kHz arrivava a un rivelatore a 6 kHz appena 6 dB sotto e faceva scendere le alte senza sibilanti; col passa-alto è 25 dB sotto, A-090), poi la sua potenza (massimo tra i canali collegati), con attacco 1 ms e rilascio 60 ms; livello = `10·log10(potenza) + 3,01 dB` (picco di una sinusoide nella banda). Riduzione: rapporto 10 oltre la soglia, limitata a `max_reduction_db`, lisciata (2 ms). `Split`: `y = x − (1 − g)·high` (sotto la frequenza il livello non cambia: il complemento del passa-basso è un passa-alto del 1° ordine in quadratura, a 500 Hz con angolo 6 kHz sta 19 dB sotto e l'errore di livello è < 0,05 dB). `Wide`: `y = g·x`. `listen`: l'uscita è solo `high`. Latenza 0.

Parametri: frequenza 3–12 kHz, soglia, riduzione massima 0–12 dB, modo, ascolto.

Proprietà verificate: un tono a 2 kHz a −12 dBFS con angolo a 6 kHz e soglia −30 non produce riduzione (> −0,1 dB), uno a 8 kHz sì (< −6 dB); in Split un tono a 500 Hz con un 8 kHz forte (riduzione ≥ 4 dB) mantiene il livello entro ±0,1 dB mentre l'8 kHz scende ≥ 4 dB; Wide abbassa entrambi allo stesso modo (±0,1 dB); riduzione massima 0 = identità (≤ 1e-6) in entrambi i modi; la riduzione non supera il massimo; ascolto: 500 Hz ≤ −15 dB, 8 kHz entro ±3 dB; invarianza al blocco con frequenza che si muove (≤ 1e-6); rumore a fondo scala con parametri variabili: finito, |uscita| ≤ 4.

## EQ dinamico (FX-15, `dyneq.rs`)

**Algoritmo.** Fino a due bande (`Peak`, `LowShelf`, `HighShelf`). Per ogni banda un *detector* filtra la chiave
(la sidechain della banda, oppure il segnale stesso) con la forma della banda: passa-banda a 0 dB di picco a `freq_hz`
con il `q` della banda per `Peak`, passa-basso (low shelf) o passa-alto (high shelf) Butterworth a `freq_hz`. I valori
assoluti dei canali sono collegati (massimo) e seguiti da un inseguitore di picco con attacco e rilascio a un polo.
Con `over = livello_dB − soglia_dB` il guadagno obiettivo è `range_db · clamp(over / 6, 0, 1)`: **la banda raggiunge
l'intero range a 6 dB sopra la soglia** e a soglia o sotto è esattamente 0 dB. Il guadagno è lisciato (polo da 2 ms) e
applicato come biquad RBJ (peak, low shelf, high shelf). Sotto 1e-4 dB con obiettivo 0 il guadagno scatta a 0 esatto e
il filtro è l'identità (la banda si muove **solo** quando il detector supera la soglia, FX-24). Frequenza e Q del
filtro d'uscita sono lisciati (5 ms, in log) sulla griglia dei coefficienti, perché un'automazione ad alto Q non faccia
esplodere il biquad.

**Parametri.** `on`, `shape`, `freq_hz`, `q` (0,1–10), `threshold_db`, `range_db` (−12…+6: negativo taglia, positivo
esalta), `attack_ms`, `release_ms`. `take_gain_db()` restituisce per banda il guadagno di massima ampiezza (con segno)
dall'ultima chiamata.

**Latenza.** 0 campioni. I coefficienti sono ricalcolati ogni 32 campioni contati dal `reset` (indipendenza dalla
dimensione del blocco); envelope e guadagno sono per campione.

**Proprietà verificate.**
- tono a 3 kHz sotto soglia: identità (≤ 1e-6) e `take_gain_db` = 0;
- 14 dB sopra soglia con range −6: il 3 kHz scende di 6 dB (±0,5), un 300 Hz mescolato resta invariato (±0,1 dB);
- range +4: l'esaltazione è +4 dB (±0,5);
- con la chiave: la chitarra a 3 kHz scende di 6 dB (±0,5) solo mentre il burst della chiave è forte, dopo il rilascio è
  identica all'ingresso (≤ 1e-6); con chiave silenziosa è identità (≤ 1e-6);
- low shelf: taglia i 100 Hz di 6 dB (±0,7) e lascia i 5 kHz (±0,2 dB);
- neutro (range 0 o bande spente): identità ≤ 1e-6;
- indipendenza dal blocco (32, 1000, 7): differenza ≤ 1e-6;
- rumore a piena scala con +6 dB, Q 10, tempi nulli: finito e |y| ≤ 16 (due bande in serie, soglia generosa);
- parametri che cambiano ogni 32 campioni su rumore per 3 s: finito e |y| ≤ 64.
`process` non alloca, non prende lock e non fa I/O.

### Transient shaper (FX-16)

Senza soglia, tutto sui rapporti. Inviluppo base: inseguitore di picco del massimo assoluto tra i canali (attacco istantaneo, decadimento 10 ms). Due inseguitori a polo singolo dell'inviluppo: veloce (`4 ms·0,15^speed`) e lento (`40 ms·0,3^speed`). `d = 20·log10(veloce/lento)`: parte d'attacco `d` oltre una zona morta di 1,5 dB (piena a 10 dB), parte di sostegno `−d` oltre 0,5 dB (piena a 6 dB). Guadagno in dB = `attack_db·w_att + sustain_db·w_sus`, lisciato (0,25 ms), poi `y = x·(1 − mix + mix·g)`. Latenza 0. Un tono stabile ha pesi 0 (veloce = lento).

Parametri: attacco ±12 dB, sostegno ±12 dB, velocità 0–1, mix 0–1.

Proprietà verificate: una sinusoide stabile (200 Hz e 1 kHz, velocità 0 e 1, tutte le combinazioni ±12 dB) non cambia livello (±0,1 dB); con attacco +6 dB il picco dei primi 15 ms di un burst sale di ≥ 3 dB, con −6 dB scende di ≥ 3 dB; la curva di guadagno a −30 e −6 dBFS coincide entro 0,5 dB; attacco = sostegno = 0 o mix 0 = identità (≤ 1e-6); invarianza al blocco (≤ 1e-6); rumore a fondo scala con parametri variabili: finito, |uscita| ≤ 16.

## Colore (FX-18): carattere della saturazione, exciter, bass enhancer

### Carattere della saturazione (`saturation.rs`)

`SaturationParams` ha il campo `character`: `Soft`, `Tape`, `Tube`, `Transistor`. `Soft` è l'algoritmo originale,
bit per bit (i progetti esistenti mantengono l'hash di render). Gli altri usano lo stesso percorso a 4x, la stessa
compensazione del livello (livello di riferimento −18 dBFS, calcolato sulla curva statica del carattere) e la stessa
latenza di 64 campioni.

- **Tape**: curva soft, poi un passa-basso a un polo a 12 kHz (alla frequenza 4x) solo nel percorso wet: la caduta
  gentile degli acuti.
- **Tube**: asimmetrica: bias 0,3 (in unità del tetto) prima della curva soft, risposta al bias tolta e pendenza
  riportata a 1, poi passa-alto a un polo a 10 Hz (4x) che toglie la componente continua. Dominano le armoniche pari.
- **Transistor**: `u / (1 + u^4)^(1/4)`, tetto 1: ginocchio duro, armoniche dispari, più brillante di `Soft`.

Il cambio di carattere non è sfumato (è una scelta discreta, non un bersaglio dell'automazione).

Proprietà verificate: `Soft` identico a un checksum registrato prima della modifica; Tube con la 2ª armonica ≥ 10 dB sopra
Soft (e > −50 dB sul fondamentale); Transistor con la 3ª armonica ≥ 3 dB sopra Soft (drive 18); Tape ≥ 1,5 dB sotto Soft a
10 kHz e entro 0,3 dB a 1 kHz; livello entro 1,5 dB per drive 0…18; aliasing < −50 dB (15 kHz, drive 18); mix 0 = dry
ritardato (≤ 1e-6); limitato (< 4) su rumore a drive 24; Tube senza continua (< 2e-3); invarianza al blocco (7, 32, 1000)
≤ 1e-6; stabile con parametri ogni 32 campioni.

### Exciter (`color.rs`)

`HP(freq)` Butterworth del 2º ordine → saturazione `2 tanh(g u / 2)` a 4x con compensazione del livello → `HP(freq)`
ancora → `× amount`, sommato al dry ritardato di 64 campioni. I due passa-alto girano la banda di 180° alla
frequenza di taglio (tornerebbe come `−0,5·x` e scaverebbe un buco fino a 6 dB): alla parte generata si aggiunge
perciò il **passa-banda del dry ritardato** alla stessa frequenza, `BP = 1 − LP − HP` con Q 1, anch'esso per
`amount` (A-090). Con questo la parte reale di ciò che si somma non è mai negativa (fino al guadagno ai piccoli
segnali della saturazione a drive 24, 1,31) e la risposta non scende mai sotto il dry; il prezzo è un rinforzo che
comincia circa un'ottava sotto la frequenza. Parametri: `freq_hz` 1000…12000, `drive_db` 0…24, `amount`
0…1. Latenza 64 (`Exciter::LATENCY`). Drive, amount lisciati per campione (5 ms); `freq` lisciata e filtri ridisegnati
ogni 32 campioni contati dal `reset` (indipendente dal blocco).

Proprietà verificate: `amount` 0 = dry ritardato (≤ 1e-6); con `amount` 1 un seno da due ottave sotto a due sopra la
frequenza non esce mai più basso di com'è entrato (> −0,05 dB, a drive 0 e 24); un seno a 200 Hz con corner a 3 kHz
guadagna < 0,1 dB anche a drive 24; un seno a 6 kHz guadagna > 3 dB; compaiono le armoniche (3ª a 12 kHz > −50 dB); limitato (< 5) su rumore;
invarianza al blocco ≤ 1e-6; stabile con parametri ogni 32 campioni.

### Bass enhancer (`color.rs`)

Il segnale ritardato passa da `LP(freq)` (2º ordine); un inseguitore di picco (attacco istantaneo, rilascio 200 ms) lo
normalizza in [−1, 1]; le armoniche sono `env · (0,5 T2 + 0,35 T3 + 0,15 T4)` (polinomi di Chebyshev: per un seno
esattamente 2ª, 3ª e 4ª armonica, senza fondamentale, col livello che segue l'ingresso); poi `HP(freq)` del 4º ordine
(due biquad) così che nulla sotto `freq` viene aggiunto, nessuna ottava sotto (anche la continua di T2 è tolta);
scalato per `harmonics · mix` e sommato al dry ritardato. Senza sovracampionamento: le armoniche restano sotto 4 ×
`freq` ≤ 640 Hz. Parametri: `freq_hz` 40…160, `harmonics` 0…1, `mix` 0…1. Latenza 64 (`BassEnhancer::LATENCY`).

Proprietà verificate: `mix` 0 o `harmonics` 0 = dry ritardato (≤ 1e-6); su un seno a 60 Hz (corner 100 Hz) 120 e 180 Hz
> −35 dB sul fondamentale e continua, 20, 30, 40 Hz < −60 dB; le armoniche scalano con `harmonics` (metà = −6 dB ±1);
limitato (< 3) su rumore; invarianza al blocco ≤ 1e-6; stabile con parametri ogni 32 campioni.

## Movimento: modulazione, tremolo/autopan e ampiezza (FX-19, FX-20, FX-24)

**Regola di fase (FX-20).** La fase di ogni LFO dipende solo dalla posizione nel brano: `process` riceve `pos`, il
frame della timeline del primo campione. Il processore tiene `next_pos`; se `pos != next_pos` (prima chiamata dopo
`reset`, salto, loop) la fase diventa `frac(rate_hz · pos / rate)` e i lisciatori si agganciano ai valori obiettivo,
altrimenti la fase si accumula per campione. Un render che parte da 30 s dà gli stessi campioni del render intero.
Quando la velocità **cambia lungo il brano** (automazione) `rate · pos` non è più la fase giusta: il motore passa al
processore, a ogni partenza (seek, blocco che si riaccende), i cicli percorsi dall'inizio del brano (`set_phase`),
calcolati in forma chiusa dalla corsia del rate come li accumula il processore, cioè il valore di ogni cella di 32
campioni tenuto per la cella (`Param::cycles` in `graph.rs`, una somma per segmento della corsia). Proprietà
verificata nel motore: con rampe e gradini sul rate di tremolo e modulazione, il render da 4 s, da 4 s + 17 campioni
e da 70 001 dà gli stessi campioni del render intero (≤ 1e-4). Un rate cambiato **a mano durante la riproduzione**
continua dalla fase accumulata, senza scatto, e torna legato alla posizione alla partenza successiva (A-090).

**Modulazione (`Modulation`).** LFO sinusoidale per canale, il destro sfasato di `spread_deg`; letture del ritardo
con interpolazione lineare; uscita `dry·(1−mix) + wet·mix`. Chorus: 1–3 voci con basi 12/16/20 ms, ritardo
`base + depth·3 ms·lfo`. Flanger: ritardo `0,5 ms + depth·7,5 ms·(0,5+0,5·lfo)` con feedback (−0,9…0,9) nella linea.
Phaser: 4/6/8/12 passa-tutto del primo ordine `(a + z⁻¹)/(1 + a z⁻¹)`, `a = (t−1)/(t+1)`, `t = tan(π f/fs)`,
`f = 200 Hz · 20^(depth·(0,5+0,5·lfo))`, feedback dell'uscita della catena con un campione di ritardo. `depth`,
`mix`, `feedback` e lo sfasamento stereo sono lisciati a 20 ms. Neutro: `mix` 0. Latenza 0. Limite: con |feedback| ≤ 0,9
il percorso wet ha guadagno ≤ 10.

**Tremolo e autopan (`Tremolo`).** LFO unipolare `u = 0,5 + 0,5·lfo` (sinusoide, triangolo, quadra con fronti lisci
`tanh(5 sin)/tanh(5)`). Tremolo: `gain = 1 − depth·(1 − u)`, canale destro sfasato di `phase_deg`. Autopan: posizione
`depth·lfo`, angolo `θ = (p+1)π/4`, `L' = √2 cosθ·L`, `R' = √2 sinθ·R`: potenza costante (gL² + gR² = 2), entrambi i
guadagni 1 al centro. Neutro: `depth` 0. Latenza 0.

**Proprietà verificate.** Regola di fase (0–40 s contro 30–40 s, confronto da 30,5 s, ≤ 1e-5), identità al neutro
(≤ 1e-6), invarianza dal blocco (blocchi da 32, 1000, 7: ≤ 1e-6), rumore a fondo scala con impostazioni estreme finito e
≤ 10,5, parametri cambiati ogni 32 campioni stabili, notch del phaser che si sposta (> 6 dB di oscillazione, 0 con depth 0),
inviluppo del tremolo (4 Hz, depth 1: ≤ 1e-6), potenza costante dell'autopan (≤ 1e-4) con energia che alterna L/R.

## Ampiezza (`StereoWidth`, FX-19)

Mid/side: `M = (L+R)/2`, `S = (L−R)/2`, `S' = S·width` (0–2, liscia a 20 ms), `L' = M+S'`, `R' = M−S'`. Con
`mono_below_hz` > 0 il side passa prima per un passa-alto Linkwitz-Riley (due Butterworth in cascata, 24 dB/ottava);
la frequenza è lisciata e i coefficienti ridisegnati ogni 32 campioni contati dal `reset`. A 0 il filtro è escluso
(resta in esecuzione a 1 Hz per tenere caldo lo stato). Neutro: `width` 1 e mono 0. Latenza 0.

**Proprietà verificate.** Identità al neutro (≤ 1e-6), `width` 0 ⇒ L = R, `width` 2 raddoppia il side (≤ 1e-6),
mono a 120 Hz: 50 Hz nel side attenuato ≥ 20 dB (LR4: circa 30 dB), 1 kHz entro 0,5 dB; ingresso L = R invariato a
qualsiasi ampiezza (≤ 1e-6); invarianza dal blocco; stabilità con parametri che cambiano ogni 32 campioni (≤ 2,5).

## Pitch: pitch shifter, armonizzatore, doubler e shimmer (FX-21, FX-22, FX-24)

File: `pitch.rs`, `reverb.rs`. Nessuna libreria; tutto in f64, per campione, quindi l'uscita non dipende dalla dimensione del blocco.

### Pitch shifter (`PitchShifter`)

Una linea di ritardo letta da due tap il cui ritardo scorre su una finestra di `W` campioni (35 ms nell'armonizzatore, 8 ms nel doubler, 20 ms nello shimmer). Con rapporto `r` ogni ritardo cambia di `1 − r` campioni per campione, quindi la posizione di lettura avanza di `r` e l'altezza si moltiplica per `r`. La fase di scorrimento `φ` dà i ritardi `S + L·φ` e `S + L·frac(φ + ½)` e i pesi `½(1 − cos 2πφ)` e `½(1 + cos 2πφ)`: sommano a 1 e il tap che salta (a `φ = 0` o `½`) ha peso 0 in quel momento, quindi nessun click. Lettura con interpolazione lineare. Il rapporto è lisciato (10 ms).

**Allineamento dei tap.** Un incrocio semplice sbaglia la frequenza di diversi hertz: a ogni salto la fase d'uscita si sposta della differenza di fase tra i due tap, alla frequenza `|1 − r| / L`. Quando un tap salta (peso 0) il suo nuovo ritardo viene spostato di al più `±S` campioni (`S = W/4`, `L = W − 2S`) sul ritardo in cui il suo segnale recente (`M = W/4` campioni) ha la correlazione normalizzata massima con quello dell'altro tap. Lag interi, deterministico, a costo contenuto (una ricerca per salto). La ricerca avanza con passo
`round(fs / 48 kHz)` sia nei lag sia nei campioni confrontati: a 96 kHz il lavoro di un salto è quello di 48 kHz
invece di quattro volte tanto (A-090); a 44,1 e 48 kHz il passo è 1 e nulla cambia. Verificato: a 96 kHz
l'intervallo resta entro 5 cent. A rapporto 1 e `φ = 0` non ci sono salti e l'uscita è l'ingresso ritardato esattamente di `W/2`.

Latenza propria: ritardo medio `W/2` (`mean_delay()`).

### Armonizzatore (`Harmonizer`)

La somma mono dell'ingresso alimenta due voci: shifter di `semitoni + cent` (±12 st, ±50 ct), livello (−60…+12 dB), pan a potenza costante, sommate al secco con `mix` (0…1). Latenza costante `Harmonizer::latency(rate)` = ritardo medio dello shifter arrotondato per eccesso a un multiplo di 32 (864 campioni a 48 kHz); il secco è ritardato di tale valore, le voci del resto in più. Con `mix = 0` l'uscita è il secco ritardato, esattamente. Parametri lisciati (20 ms) e impostati senza scivolamento alla prima chiamata.

### Doubler (`Doubler`)

Una o due copie dell'ingresso, scordate di `+detune` e `−detune` cent (shifter con finestra di 8 ms, ritardo medio 4 ms), ritardate di `delay_ms` (5–40 ms; la seconda voce ×1,25) con una deriva lenta di ±20% a 0,13 e 0,21 Hz, panoramizzate da `spread` (voce 1 a sinistra, voce 2 a destra), sommate al secco non ritardato. Latenza 0: il ritardo è l'effetto. Fase della deriva dalla posizione del brano: se `pos` non è il fotogramma successivo alla chiamata precedente la fase è `frac(f·pos/rate)`, altrimenti si accumula.

### Shimmer (`Algorithm::Shimmer`)

La rete Hall con uno shifter (finestra 20 ms) nella ricircolazione: la coda che la rete sta per restituire, spostata di `shimmer_semitones` (5, 7 o 12), rientra nelle linee con peso `a/2` (`a = shimmer_amount`): `(1 − a/2)·feedback + a/2·spostato`. Poiché il segnale spostato è una combinazione convessa dei campioni d'ingresso, il guadagno d'anello non supera quello della Hall: coda limitata e silenzio esatto alla fine. A `a = 0` è la Hall, campione per campione; Plate, Room e Hall non conoscono lo shimmer.

### Proprietà verificate

| Test | Requisito | Soglia |
| --- | --- | --- |
| intervallo di un sinusoide a 440 Hz: +7, −5, +12, −12 st e +7 st +25 ct | FX-21 | ±5 cent (misurato: sotto 0,1 cent) |
| stesso intervallo a 150, 1000, 3000 Hz (+7, −5) | FX-21 | ±5 cent |
| voce dell'armonizzatore a +7 st +25 ct, secco presente | FX-21 | ±5 cent |
| armonizzatore a mix 0 = secco ritardato della latenza | FX-21, FX-24 | ≤ 1e-6 |
| latenza multiplo di 32 e ≥ ritardo medio, per 44,1–192 kHz | FX-24 | esatto |
| voce a 0 st allineata al secco: correlazione al lag 0, differenza dal secco ritardato × 0,7071 | FX-24 | picco a ±1 campione, ≤ 1e-6 |
| livello e pan della voce | FX-21 | ≤ 1e-4 |
| doubler a mix 0 = identità, latenza 0 | FX-24 | ≤ 1e-6 |
| doubler: voce scordata di 30 ct (±5 ct) a sinistra, destra intatta; spread 0 centrato | FX-21 | ±5 cent |
| doubler: ritardo della copia 20 ms ± 20% | FX-21 | lag 16–24 ms |
| doubler: fase della deriva dalla posizione dopo un seek | ENG-4 | ≤ 1e-3 oltre 60 ms |
| rumore a fondo scala, impostazioni estreme | FX-24 | armonizzatore ≤ 10, doubler ≤ 3 (combinazione convessa: secco 1 + voci) |
| indipendenza dal blocco (32, 1000, 7) | FX-24 | ≤ 1e-6 |
| parametri casuali ogni 32 campioni per 4 s | FX-24 | finiti, stessi limiti |
| shimmer a quantità 0 = Hall; proprietà FX-9 (decadimento ±10%, decorrelazione, densità, smorzamento, silenzio esatto, indipendenza dal blocco) | FX-22, FX-9 | come per gli altri algoritmi |
| shimmer a 0,6 con 5/7/12 st: coda che non cresce in 20 s, silenzio esatto, indipendenza dal blocco, reset | FX-22 | picco 10–20 s sotto quello 1–3 s; ultimo secondo ≥ 40 dB sotto |
| shimmer a +12 st: potenza all'ottava rispetto alla fondamentale | FX-22 | ≥ 20 dB sopra quantità 0 (misurato: 34 dB) |

## Delay: modo ping-pong e carattere tape (FX-23)

Estensione del delay stereo (FX-6). Con `mode = Stereo` e `character = Clean` il percorso è quello originale,
invariato: l'uscita è identica bit per bit (checksum dorato registrato col codice precedente), qualunque siano `wow` e
`drive_db`, così gli hash di render dei progetti esistenti non cambiano.

**Parametri nuovi.** `mode` (`Stereo`, `PingPong`), `character` (`Clean`, `Tape`), `wow` 0..1, `drive_db` 0..12 dB
(wow e drive agiscono solo con `Tape`).

**Algoritmo.**
- `PingPong`: l'ingresso sommato a mono `(L+R)/2` alimenta solo la linea sinistra; ogni ripetizione passa all'altro
  canale (L, R, L, R…) con lo stesso tempo e gli stessi filtri di feedback (passa-alto poi passa-basso).
- `Tape`: il tempo è modulato da wow (1 Hz, ±0,4% del tempo) più flutter (7 Hz, ±0,1%), entrambi scalati da `wow`
  (lisciato come il tempo), lettura con interpolazione lineare. La fase degli LFO dipende solo dalla posizione nel
  brano: `process_at(.., pos)` la ricalcola come `frac(f·pos/rate)` alla prima chiamata e dopo ogni salto, altrimenti
  la accumula; `process` usa una posizione interna che parte da 0 dopo `new`/`reset`.
- `drive_db`: saturazione morbida `tanh(g·x)/g`, `g = 10^(drive/20)`, sul segnale di feedback dopo i filtri (guadagno
  unitario per segnali piccoli, pendenza ≤ 1: l'anello resta contrattivo). Il valore scritto è al massimo
  `|in| + 0,95/g`. `g` e `wow` sono lisciati con lo stesso passa-basso del tempo (50 ms).

**Proprietà verificate.**
- Stereo/Clean: checksum identico all'originale con `wow = 0, drive = 0` e con `wow = 1, drive = 12`.
- Ping-pong: l'impulso dà ripetizioni alternate L, R, L, R a multipli del tempo (ampiezza 0,5^(k−1) ±20%, altro canale
  < 1e-3).
- Tape con `wow = 0` e `drive = 0` uguale al clean (±1e-6).
- Wow: il render che parte a 30 s coincide con quello completo da 30,5 s in poi (≤ 1e-5).
- Drive 12 dB, feedback 0,95, rumore a fondo scala: limitato (< 1,3), in entrambi i modi.
- Indipendenza dalla dimensione del blocco (32, 1000, 7; ±1e-6) e parametri cambiati ogni 32 campioni: finito e
  limitato (< 1,6 con drive ≥ 6 dB).
Latenza: 0.

