# Measures and formulas

Neutral specification of the engine's measures (PRD §7.1): formula, windows, parameters and version of each
algorithm, so that another implementation can reproduce the same numbers. The reference code is
indicated for each entry, but the definition is this document.

## Conventions

- Signal: floating-point samples, full scale = 1.0, shape `(samples, channels)`, 1 or 2 channels.
  Internal processing in float32 (ING-6); level measures compute in float64.
- dBFS: `20 · log10(|x|)` with `|x| = 1.0` ⇒ 0 dBFS. The level of digital silence is `−∞`.
- Sample rate `fs` of the project: the one common to all stems (ING-3, D-010).

## Sample peak

- Version: `sample_peak v1` — code: `stemwise.analysis.stem.sample_peak_db`.
- Formula: `20 · log10( max_{n,c} |x[n,c]| )`, maximum over all samples and all channels.

## True peak

- Version: `true_peak v1` (D-009) — code: `stemwise.analysis.stem.true_peak_db`.
- Oversampling by factor `L = 4` (ANA-1), independent of `fs`.
- Interpolation: insertion of `L − 1` zeros between consecutive samples and a linear-phase low-pass FIR filter, applied to
  each channel separately (implementation: `scipy.signal.resample_poly(x, 4, 1)`).
  - Length: `2 · 10 · L + 1 = 81` taps at the frequency `L · fs`.
  - Coefficients: `g[k] = sinc((k − 40) / L) · w[k]`, `k = 0…80`, with `sinc(t) = sin(πt) / (πt)` and `w`
    a Kaiser window with parameter `β = 5.0` over 81 points; `h[k] = L · g[k] / Σ g` (gain L at DC,
    which compensates for the inserted zeros). Cutoff at the original Nyquist (`fs / 2`). Verified: coincides with the
    filter used by `resample_poly` (maximum deviation 0).
  - Compensated delay: the interpolated sample `y[m]` corresponds to the instant `m / (L · fs)`.
- Edges: before the first sample and after the last one the signal is 0 (silence). A sharp attack at full scale
  therefore produces real overshoots between samples, which are measured.
- Formula: `TP = 20 · log10( max( max_{n,c} |x[n,c]|, max_{m,c} |y[m,c]| ) )` dBTP, i.e. never below
  the sample peak.
- Computation in blocks of 65,536 samples with 32 context samples per side (more than the half-length of the filter,
  10 input samples): the result coincides with that of a single pass.
- Accuracy verified in the tests: sine at `fs / 4` with phase 45° (sample peak −3.01 dBFS, real peak
  0 dBFS) measured at +0.015 dBTP; sine at 997 Hz measured within +0.006 dB. Test tolerance ±0.1 dB.
- Differences from ITU-R BS.1770-4 Annex 2: the standard uses a 48-coefficient FIR (12 per phase); here
  the FIR is longer and flatter in the passband. At the same 4x factor the two can differ by
  a few hundredths of a dB on inter-sample peaks near the Nyquist.

## Mix bus: sum and trim

Used by the raw sum `baseline.wav` (REN-7) and, from M2, by the mix bus (REN-5).
Code: `stemwise.render.mixbus`.

- Version: `mixbus v1` (D-007, D-008).
- Mono stem on the stereo bus: `L = R = x · √½` (−3.01 dB per channel, constant power at the center, the same
  law as the pan of REN-3). Stereo stems: unchanged. The bus is always stereo.
- Sum: `B[n,c] = Σ_s S_s[n,c]`, in the order of the stems in `project.yaml`, with a float64 accumulator and
  result in float32. The stems already have the same length (padding of ING-4).
- Trim (attenuation only): with `TP_B` the true peak of the bus, `C = mix_true_peak_db` (default −3.0) and
  margin `m = 0.01 dB`, the gain is `g_dB = min(0, (C − m) − TP_B)` and the bus becomes
  `B · 10^(g_dB / 20)`. If `g_dB = 0` the bus remains identical, sample by sample.
- Reason for the margin: after quantization to 24 bits (error ≤ 1 LSB = 2⁻²³) the file read back stays within
  `C`. Verified in the tests: true peak after the trim in [−3.02; −3.00] dBTP and file read back ≤ −3.0 dBTP.
- Raw sum (REN-7): original stems, without pre-gain or plan, then only the trim.

## Analysis STFT and active parts

Common basis of bands, activity, centroid, stereo measures and masking (D-014). Code:
`stemwise.analysis.spectral`.

- Version: `spectral v1`.
- Window: periodic Hann `w[n] = 0.5 − 0.5·cos(2πn / N)`, `n = 0…N−1`.
- Size: `N = 8192 · 2^round(log2(fs / 48000))`, i.e. 8192 at 44.1 and 48 kHz (≈170 ms, bins of 5.9 Hz at
  48 kHz) and 16384 at 88.2 and 96 kHz. Hop `H = N / 4`.
- Grid: frame `t` covers the samples `[t·H, t·H + N)`; the number of frames is `1 + ⌈max(n − N, 0) / H⌉` for
  a signal of `n > 0` samples; the last frame is padded with zeros.
- Power of a bin, per channel: `p[k] = c_k · |X[k]|² / (N · Σ w²)`, with `X = rfft(w · x)`, `c_k = 1` for
  `k = 0` and `k = N/2`, `c_k = 2` otherwise. By Parseval `Σ_k p[k] = Σ w²x² / Σ w²`: the Hann-weighted mean square
  of the frame (a sine of amplitude A gives `A²/2`).
- Frame power (D-022): `P[t] = mean over channels of Σ_{k: 20 Hz ≤ f_k ≤ 20 kHz} p[k]`, i.e. over the same bins
  as the bands. Level: `10·log10(P[t])` dBFS. DC and rumble below 20 Hz do not count: a DC offset above
  0.32% does not make the whole stem "active". For a signal entirely in band `P[t]` is the Hann-weighted mean square
  of the frame.
- **Active frame**: `P[t] > 10^(−50/10)`, i.e. level above −50 dBFS (PRD §5.2).
- `active_pct` (ANA-4) = `100 · active frames / total frames`. Active seconds = `active_pct / 100 · duration`.
- Power on the bus (ANA-8, ANA-9): mean power over channels × number of channels, i.e. `x²` for a mono stem and
  `L² + R²` for a stereo one, the power that the stem carries on the stereo bus with the law of D-008.
- The STFT is computed in blocks of 128 frames, in float64, and immediately reduced to per-frame and per-band powers, the mean spectrum of the
  active frames and stereo sums: the full spectrogram is never kept in memory.

## Bands (ANA-3)

- Version: `bands v1` — code: `stemwise.analysis.spectral.band_levels_db`.
- Fixed bands in Hz: 20–60, 60–120, 120–250, 250–500, 500–2k, 2k–4k, 4k–8k, 8k–20k. The bin at frequency
  `f = k · fs / N` belongs to the band if `lo ≤ f < hi`; the last band includes 20 kHz and is limited to `fs / 2`.
  The bins below 20 Hz and above 20 kHz do not enter any band.
- Energy of band `b`: `E_b = Σ_{t active} Σ_{k ∈ b} p̄_t[k]`, with `p̄` the mean power over channels.
- Value: `10·log10(E_b / Σ_b E_b)` dB, so `Σ_b 10^(value/10) = 1`. No active frame or no energy
  in 20 Hz–20 kHz ⇒ `null`.
- Verified in the tests: a 1 kHz sine has ≥ 98.9% of its energy in 500–2k and less than −60 dB elsewhere; a pink noise
  with exact `1/f` spectrum deviates by at most 0.15 dB from the theoretical value
  `10·log10(log2(hi/lo) / log2(1000))` (the largest deviation is in the 60–120 Hz band).

## Spectral centroid (ANA-4)

- Version: `centroid v1` — code: `stemwise.analysis.spectral.centroid_hz`.
- Mean spectrum of the active frames `S[k] = mean_{t active} p̄_t[k]`; centroid
  `Σ f_k · S[k] / Σ S[k]` over the bins with 20 Hz ≤ `f_k` ≤ 20 kHz, in Hz. It is weighted by **power** (two sines of equal
  amplitude at 500 and 2000 Hz give 1250 Hz). No active frame ⇒ `null`.

## Loudness

Code: `stemwise.analysis.loudness`. Library: `pyloudnorm` 0.2.0. A new meter for each call.

### Integrated loudness (ANA-1)

- Version: `lufs v1` — ITU-R BS.1770-4.
- K-weighting as in `pyloudnorm` (`filter_class="K-weighting"`): two RBJ biquads computed for the project `fs`, high shelf
  `G = +4 dB`, `Q = 1/√2`, `fc = 1500 Hz`, then high-pass `Q = 0.5`, `fc = 38 Hz`.
- 400 ms blocks with 75% overlap; block loudness `−0.691 + 10·log10(Σ_c G_c · z_c)`, with
  `G = 1` for L and R; absolute gate −70 LUFS, relative gate −10 LU below the mean of the blocks above the absolute
  gate. Computation on the full signal (the −50 dBFS gate of D-014 does not apply: the gating of the standard
  already excludes silence).
- Mono stem: a single channel with `G = 1`, so a 997 Hz sine of amplitude 1 gives −3.01 LUFS theoretical.
- Edge cases: all blocks excluded by the gate (silence) ⇒ `−∞` (−120 in the JSON, D-018); signal shorter than
  a 400 ms block ⇒ `null`.
- Measured accuracy: 997 Hz sine at −20 dBFS mono ⇒ −23.05 LUFS (theoretical −23.01): the deviation of 0.04 LU comes from the
  RBJ approximation of the standard's filters, within the ±0.1 LU tolerance of EBU Tech 3341.

### Dynamic range (ANA-2)

- Version: `lra v1` (D-015) — EBU Tech 3342, implementation `pyloudnorm.Meter.loudness_range`.
- Short-term loudness: same formula as the blocks, with 3 s windows and 97% overlap, hence a step
  of 90 ms (11.1 values per second; the standard asks for at least 10). 1.5 s of silence are added at the end of the signal,
  as in the example of the standard.
- Absolute gate −70 LUFS, then relative gate −20 LU below the mean loudness (in power) of the remaining windows.
- Value: `P95 − P10` of the remaining windows, percentiles with linear interpolation (`numpy.percentile`).
- Edge cases: signal shorter than 1.5 s, or everything excluded by the gate ⇒ `null`.
- Verified in the tests with the four signals of Tech 3342 (stereo 1 kHz sine, 20 s per level: −20/−30,
  −20/−15, −40/−20, −50/−35/−20/−35/−50 dBFS ⇒ 10, 5, 20, 15 LU): measured deviation < 0.001 LU.

## Stem levels (ANA-1)

Code: `stemwise.analysis.stem`.

- Sample peak and true peak: sections of the same name above, on the full signal.
- RMS (`rms v2`, D-022): `10·log10( mean_{t active} P[t] )` dBFS, with `P[t]` the power of the frames of the analysis STFT,
  limited to 20 Hz–20 kHz (DC and sub-20 Hz excluded, as for the bands); no active frame ⇒ `null`. A sine of amplitude A gives `20·log10(A/√2)`.
  On signals with active stretches and silence, the frames straddling the edges lower the RMS by a few hundredths of a dB
  (measured: 0.04 dB on 10 s of sine + 10 s of silence).
- Crest factor (`crest v1`): sample peak − RMS over the active frames, in dB.

## Hygiene (ANA-6)

Code: `stemwise.analysis.stem`.

- DC offset (`dc v1`, D-018): `100 · max_c |mean_n x[n,c]|`, as a percentage of full scale, on the full signal.
- Clipped samples (`clip v1`): number of samples, over all channels, with `|x| ≥ 1 − 2⁻¹⁵`. The threshold is the
  positive maximum of a 16-bit PCM read back (32767 / 32768): it counts the full-scale samples at 16 and 24 bits and the "overs" of
  float files.
- Noise floor (`noise_floor v2`, power in 20 Hz–20 kHz): `10·log10( median_{t inactive} P[t] )` dBFS over the inactive frames
  (level ≤ −50 dBFS); `−∞` (−120 in the JSON) if the pauses are digital silence. `null` if the inactive time
  (fraction of inactive frames × duration) is below 1 s.

## Transient density (ANA-4)

- Version: `onsets v2` (D-020, D-023) — code: `stemwise.analysis.stem.onsets_per_s`. Library: `librosa` 1.0.0, with
  Python 3.12 (`.python-version`). Warning: with Python 3.11, allowed by `requires-python`, the lock resolves
  `librosa` 0.11.0 and the onset values could differ; the reference numbers are those with 3.12.
- Signal: each channel separately, on the full signal; the envelope used is the mean of the channel envelopes
  (not the envelope of the channel mean, which would cancel antiphase).
- Envelope: `librosa.onset.onset_strength` with the defaults: power mel spectrogram (`n_fft 2048`, `hop 512`,
  128 bands), in dB (`power_to_db`, `ref 1`, `top_db 80`), positive difference between consecutive frames (`lag 1`),
  mean over the bands.
- Peaks: `librosa.onset.onset_detect` with `normalize=False` and `delta = 2.0`; the other parameters at their defaults
  (`pre_max 30 ms`, `post_max 0`, `pre_avg 100 ms`, `post_avg 100 ms`, `wait 30 ms`). An onset is a peak
  of the envelope that exceeds the local mean by 2 dB.
- Filter on the active parts (D-023): an onset at position `s` (samples) belongs to the analysis frame with the nearest
  center, `t = round((s − N/2) / H)` limited to `[0, frames − 1]`; only the onsets of active frames count.
  Thus the transients below −50 dBFS in the pauses (bleed of the click from the headphones) are not counted.
- Value: number of onsets in active frames / active seconds (D-014); 0 if the stem has no active frames.
- Verified in the tests: click at 4/s over 10 s ⇒ 4.0, even in antiphase (L = −R); stationary sine, white and pink noise
  ⇒ ≤ 0.1 onsets/s; 5 s of voice + 15 s of bleed at −60 dBFS ⇒ 0.2 onsets/s (without the filter: 12). An
  abrupt stop of the signal may count as an onset.

## Stereo (ANA-5)

- Version: `stereo v1` — code: `stemwise.analysis.stem.stereo_measures`. Stereo stems only; mono ⇒ `null`.
- Sums over the active frames and the bins with 20 Hz ≤ f ≤ 20 kHz, with the same scale `c_k / (N·Σw²)` as the powers:
  `LL = Σ|L|²`, `RR = Σ|R|²`, `LR = Σ Re(L·R*)`, `MM = Σ|(L+R)/2|²`, `SS = Σ|(L−R)/2|²`, with `L`, `R` the spectra
  of the analysis STFT.
- Correlation: `LR / √(LL·RR)`, in [−1, 1]: it is the zero-lag correlation of the band-filtered signals
  (Parseval), Hann-weighted. A silent channel ⇒ 0; both silent ⇒ `null`.
- Side/mid ratio: `10·log10(SS / MM)` dB. Dual mono ⇒ `−∞`, pure antiphase ⇒ `+∞` (±120 in the JSON, D-018);
  no signal ⇒ `null`. Hard pan of a mono (`R = 0`) ⇒ 0 dB.

## Mix (ANA-7, ANA-8, ANA-10)

Code: `stemwise.analysis.mix`. The analyzed mix is the raw sum with the trim, i.e. the signal of
`baseline.wav` (D-017); it is always stereo (D-008).

- Mix measures (ANA-7): integrated LUFS, peak, true peak, RMS, crest, LRA, bands and stereo, with the same
  formulas and versions as the stem measures.
- Shares per band (ANA-8, `share v2`, levels of D-028): `E_{s,b} = Σ_{all frames} power on the bus` of stem `s` in band
  `b` (mono `x²`, stereo `L² + R²`); share `100 · E_{s,b} / Σ_s E_{s,b}` %. The shares of a band add up to 100;
  a band where no stem has energy gives 0 for all. The denominator is the sum of the energies, not the energy
  of the mix: with correlated stems (doublings, drum microphones) the two differ.
- Stem levels for shares and masking (D-028, `share v2`, `masking v2`): with `pre_normalize: true` (the
  default) per-band shares and masking are computed on the stems multiplied by the pre-gain of REN-1 (section
  "Render", brought to −23 LUFS), i.e. on the starting point of the plan; with `pre_normalize: false` on the raw
  stems. The per-stem measures and the `mix` block remain on the raw signals.
- Bass correlation (ANA-10, `low_corr v1`): same formula as the correlation of ANA-5, on the bins with
  20 Hz ≤ f < 120 Hz and on the active frames of the mix. Mono signal ⇒ 1; no active frame ⇒ `null`.

## Masking (ANA-9)

- Version: `masking v2` (D-016; stem levels of D-028, see "Mix") — code: `stemwise.analysis.masking`.
- For each pair of stems `(a, b)` and each band `k`, frame by frame on the common grid of the analysis STFT
  (the stems have the same length, ING-4):
  - stem active in the band: in-band power averaged over channels > −50 dBFS;
  - `E_a[t]`, `E_b[t]`: in-band power on the bus (mono `x²`, stereo `L² + R²`);
  - score `Σ_{t: both active} min(E_a, E_b) / Σ_{t: at least one active} max(E_a, E_b)`, in [0, 1].
- Reading: the fraction of time in which the two stems overlap in the band, weighted by the ratio between the weaker and the stronger. Two identical stems give 1; a copy 12 dB lower gives `10^(−1.2) ≈ 0.063`; stems that never play together in the same band give 0.
- In the JSON: the 10 highest scores greater than 0, in descending order; on a tie, pairs in the order of
  `project.yaml` (first `a`, then `b`) and bands in ascending order.

## References and target profile (ANA-11, ANA-12)

Code: `stemwise.analysis.reference`.

- Input: the reference already resampled to the project `fs` (ING-5, D-012), mono or stereo as it is.
- Normalization (`reference v1`): gain `10^((−23 − L) / 20)` with `L` the integrated loudness; −23 LUFS is the
  same level as the stem pre-gain (REN-1). A silent reference or one shorter than 400 ms gives an error.
- Measures: those of the mix (ANA-7 and ANA-10) on the normalized reference. Bands, side/mid and LRA do not depend on the gain
  (verified: deviation < 0.05 between a reference and its copy at −12 dB); the LUFS is −23 by construction.
- Target profile (`target v1`, ANA-12): arithmetic mean of the values of the references, field by field — bands (in
  dB, band by band), `side_mid_db`, `dyn_range_lu`. A reference enters the mean of a field only if it has a
  finite value: mono and dual-mono references do not enter the side/mid mean. No reference ⇒ `target`
  `null`; no value for a field ⇒ that field `null`.

## `analysis.json`: representation (ANA-13, D-018)

Contract: `schemas/analysis.schema.json` (generated from the model `stemwise.analysis.model.Analysis`).

- Measures are computed at full precision; limits and rounding apply only when writing the file.
- Levels in dB and LU limited to [−120, +120]: `−∞` (digital silence, dual mono) becomes −120, `+∞` (pure
  antiphase) +120. `Infinity` or `NaN` never appear in the file.
- Every real number is rounded to one decimal (Python rounding, to the nearest even on exact halves in binary), except the dimensionless quantities in [−1, 1] — `correlation`, `low_corr` and `score` of masking —
  rounded to two decimals (D-021); `−0.0` is written as `0.0`. The integers (`sample_rate`, `channels`, `clipped_samples`,
  `n_references`) remain integers.
- Measures that require active frames on a fully inactive stem: `null` (RMS, crest, bands, centroid, LRA,
  stereo); `onsets_per_s` and `active_pct` are 0.
- Fields beyond the example of PRD §6.2: `version` (1), `rms_db` for stems and mix, `peak_db` and `crest_db`
  for the mix, the `references` list with the measures of each reference. A reference is identified only by
  a neutral label `ref1`, `ref2`, `ref3` (the N-th of `project.yaml`), never by the file name or path
  (D-024, privacy of PRD §7).
- Format: objects indented by two spaces, lists of numbers on one line, keys in the order of the contract, stems
  in the order of `project.yaml`, final newline.
- Token estimate: characters / 4. Worst case built in the tests (12 stereo stems with 32-character ids,
  3 references, 10 collisions, all fields filled): about 11,650 characters, i.e. about 2,920 tokens, below the
  ceiling of 8,000.

## Render (REN-1 → REN-7)

Application of a `mix_plan.json` (contract: `schemas/mix_plan.schema.json`, model
`stemwise.decision.schema.MixPlan`). Code: `stemwise.render`. Version: `render v1`.

### Filters and EQ: RBJ biquads (D-029)

Code: `stemwise.render.biquad`. Each filter is a biquad from the Audio EQ Cookbook of R. Bristow-Johnson,
form with Q, with the coefficients divided by `a0`:

- Intermediates: `w0 = 2π·f0/fs`, `α = sin(w0)/(2Q)`, `A = 10^(G/40)` (G = gain in dB). It requires
  `0 < f0 < fs/2` and `Q > 0`.
- High-pass (`hpf_hz`, and `return_hpf_hz` of the reverb): second-order Butterworth, `Q = 1/√2`
  (−3.01 dB at f0, 12 dB/octave). `b = [(1+cos w0)/2, −(1+cos w0), (1+cos w0)/2]`, `a = [1+α, −2 cos w0, 1−α]`.
- Low-pass (`lpf_hz`): same Q. `b = [(1−cos w0)/2, 1−cos w0, (1−cos w0)/2]`, `a = [1+α, −2 cos w0, 1−α]`.
- `peak`: `b = [1+αA, −2 cos w0, 1−αA]`, `a = [1+α/A, −2 cos w0, 1−α/A]`. Full gain at f0.
- `low_shelf`, with `k = 2√A·α`:
  `b = [A((A+1)−(A−1)cos w0+k), 2A((A−1)−(A+1)cos w0), A((A+1)−(A−1)cos w0−k)]`,
  `a = [(A+1)+(A−1)cos w0+k, −2((A−1)+(A+1)cos w0), (A+1)+(A−1)cos w0−k]`. Half the gain at f0.
- `high_shelf`:
  `b = [A((A+1)+(A−1)cos w0+k), −2A((A−1)+(A+1)cos w0), A((A+1)+(A−1)cos w0−k)]`,
  `a = [(A+1)−(A−1)cos w0+k, 2((A−1)−(A+1)cos w0), (A+1)−(A−1)cos w0−k]`. Half the gain at f0.
- Application: transposed direct form (`scipy.signal.sosfilt`) in float64, zero initial state, channels
  independent; output in float32. The EQ bands are applied in cascade in the order of the plan.
- Verification: the curves of `peak` and shelves coincide within 0.05 dB with `PeakFilter`, `LowShelfFilter` and
  `HighShelfFilter` of `pedalboard` 0.9.26 (same formulas, in float32).

### Pre-gain and gain (REN-1, D-033)

Code: `stemwise.render.chain`.

- Pre-gain: `g_pre = −23 − LUFS(stem)`, with the integrated BS.1770-4 loudness of the raw stem (same function as
  ANA-1). A mono stem at −23 LUFS stays at −23 LUFS on the stereo bus, because the center at √½ per channel preserves the
  power. With `pre_normalize: false`, `g_pre = 0`.
- Muted stem (all blocks below the −70 LUFS gate) or shorter than a 400 ms block: `g_pre = 0` with a warning.
  No ceiling; warning if `g_pre > +24 dB`.
- Each gain in dB `G` (pre-gain, makeup, `gain_db`, send, return) multiplies by `10^(G/20)` in float64, with
  output in float32; `G = 0` leaves the signal identical.
- No internal clipping: before the bus trim the samples can exceed ±1.

### Pan (REN-3, D-008, D-030)

Code: `stemwise.render.pan`. Output always stereo.

- Mono stem, constant power: `θ = (p + 1)·π/4`, `L = x·cos θ`, `R = x·sin θ`. Exact values at the extremes
  (`p = −1` ⇒ `L = x`, `R = 0`; `p = +1` ⇒ `L = 0`, `R = x`) and at the center (`√½`, the law of the raw sum).
- Stereo stem, linear balance: `L' = L·min(1, 1 − p)`, `R' = R·min(1, 1 + p)`. At the center the stem remains
  identical; no channel is ever raised or moved to the other side.

### Compressor (REN-2, D-034)

Code: `stemwise.render.chain.compress`. Library implementation, not a formula: `pedalboard.Compressor`
0.9.26 (JUCE `dsp::Compressor`), with `threshold_db`, `ratio`, `attack_ms`, `release_ms` of the plan; state zeroed
at each call (`reset=True`), processing in float32.

- Peak detector, one per channel: stereo stems are **not** compressed in a linked way (verified: channel at
  −10 dBFS reduced by 14.2 dB, channel 40 dB below unchanged). No knee.
- Measured on a 1 kHz sine with peak −10 dBFS, threshold −30, ratio 4 (theoretical reduction 15 dB): 14.2 dB with
  attack/release 10/120 ms, 14.8 with 1/100, 14.8 with 0.1/20, due to the release of the detector between the peaks.
- The makeup (`makeup_db`) is a separate stage, a pure gain immediately after the compressor.
- Warning for a port (PRD §7.1): another compressor will not give the same samples.

### Stem chain (REN-2)

Code: `stemwise.render.chain.render_stem`. Fixed order:
pre-gain → high-pass → low-pass → EQ bands in the order of the plan → compressor → makeup → `gain_db` → pan.
A `null` block (or an empty `eq`) is skipped: the signal passes identical. The output is the stereo stem after the pan, from which
the send to the reverb is taken (post-fader, post-pan).

### Shared reverb (REN-4, D-031)

Code: `stemwise.render.reverb`. Library implementation: `pedalboard.Reverb` 0.9.26 (JUCE, Freeverb tunings),
state zeroed at each call, float32.

- Send of each stem: `S_s = Y_s · 10^(reverb_send_db/20)`, with `Y_s` the stereo stem after the pan (post-fader,
  post-pan). `reverb_send_db: null` ⇒ no send.
- Send bus: `Σ_s S_s` (float64, then float32) → `Reverb(room_size, damping, wet_level = 1, dry_level = 0,
  width = 1, freeze_mode = 0)` → RBJ high-pass at `return_hpf_hz` (Q = 1/√2) → `return_gain_db` → mix bus.
- `reverb: null`, or no stem with a send ⇒ no return.
- The Freeverb sums L and R before the combs: the position of the send does not change the tail (verified in the tests).
  No dry sound: the first output arrives after about 25 ms (the shortest comb).
- Duration: equal to that of the send bus; the tail beyond the end of the stems is cut off.
- Return level, measured with stereo pink noise, `damping` 0.5, return without high-pass or gain: +0.8 dB
  relative to the send with `room_size` 0, +1.8 with 0.35, +2.5 with 0.5, +6.8 with 0.9. It serves to calibrate `return_gain_db`.
- Warning for a port (PRD §7.1): another reverb will not give the same samples.

### Mix bus and exported stems (REN-5, REN-6, D-032)

Code: `stemwise.render.render.render_mix`.

- The plan must contain exactly the stems of the project and frequencies below `fs/2` (D-033), otherwise an error;
  stems shorter than 3 samples ⇒ error.
- Bus: `B = Σ_s Y_s + R`, with the stems `Y_s` after the pan in the order of `project.yaml` and the reverb return `R`
  last; float64 accumulator, result in float32; then the same trim as the raw sum (section "Mix bus",
  D-007). Duration equal to that of the stems (D-031).
- A neutral plan (gain 0, pan 0, `null` blocks, `reverb: null`) with `pre_normalize: false` gives a bus identical, bit
  for bit, to the raw sum.
- Exported stems (`--export-stems`): `Y_s · 10^(g_trim/20)` for each stem (in `stems/`) and `R · 10^(g_trim/20)` for
  the return (`reverb_return.wav`, next to `mix.wav`). Their sum is the mix, except for the float32 rounding and the 24-bit quantization of
  each file. A 24-bit file clips the samples beyond ±1: if an exported stem exceeds 0 dBFS (the trim
  guarantees only the sum) a warning reports that the sum of the files is no longer the mix.

## Evaluation (EVA-1 → EVA-4)

Version: `evaluation v1` — code: `stemwise.evaluate`. Contracts: `schemas/evaluation.schema.json`
(`evaluation.json` of each iteration and `baseline_evaluation.json` of the raw sum) and `schemas/best.schema.json`
(D-049).

- Mix measures (EVA-1): those of the `mix` block of `analysis.json` (ANA-7 and ANA-10, section "Mix"), on the mix
  rendered with the trim, i.e. the signal of `mix.wav`; for the raw sum on the signal of `baseline.wav`.
- Spectral distance (EVA-2, D-050), on the bands `m_i` of the mix and `t_i` of the target profile (ANA-12), at full
  precision and limited to ±120 dB:
  - `offset = (1/8) Σ t_i − (1/8) Σ m_i` (arithmetic mean in dB);
  - `band_delta_i = t_i − (m_i + offset)`: positive = the mix lacks energy in that band;
  - `spectral_distance_db = (1/8) Σ |band_delta_i|`.
  - Without target, or without bands in the mix (no active frame), distance and `band_delta_db` are `null`.
- Width and dynamics (EVA-3, D-050): `width_delta_db = target.side_mid_db − mix.side_mid_db` (positive = the mix is
  narrower), `dyn_delta_lu = target.dyn_range_lu − mix.dyn_range_lu` (positive = the mix has less range).
  `null` if one of the two values is missing.
- Technical constraints (EVA-4, D-053), on the full-precision measures of the mix trimmed in memory:
  - `true_peak_ok`: true peak ≤ `mix_true_peak_db` (the 0.01 dB margin of the trim, D-007, covers the 24-bit rounding of the file);
  - `low_corr_ok`: `low_corr > 0`; `null` (no active frame below 120 Hz) counts as respected;
  - `dc_ok`: DC offset (ANA-6: maximum over channels of `|mean|`, in % of full scale) ≤ 0.1% (−60 dBFS).
- Representation: as `analysis.json` (one decimal, two for correlations and scores, D-021), with the layout of the
  same file.
- Residual collisions (EVA-1, EVA-3, D-054): formula of ANA-9 (`masking v2`, section "Masking"), the 10 highest
  scores, on the stems as they arrive at the bus. Rendered mix: the stems processed after the pan and before the trim
  (`Y_s` of section "Mix bus"), without the reverb return, which is not a stem; the spectrum of each `Y_s` is
  computed right after its render. Raw sum: the original stems as they enter the bus of `baseline.wav`: the stereo ones
  with the spectra already computed by the analysis, the mono ones centered on two channels at `√½` (D-008), as a mono stem
  rendered with the pan at center, so the activity threshold (mean over channels) is the same for the raw sum and
  for the mix. In both cases the stems are used before the trim: with the absolute threshold of −50 dBFS the score
  depends on the level, and thus does not depend on how much the bus is attenuated. The `masking_top` of `analysis.json` remains
  on the pre-normalized stems (D-028), so with `pre_normalize: true` it differs from that of the raw sum.
  Example: two identical stems give 1.0 in the raw sum; with one of the two at −12 dB of gain they give `10^(−1.2) ≈
  0.06` in each band; the pan does not change the score (the power on the bus does not depend on the pan, REN-3).

## Verification loop (EVA-5, EVA-6)

Code: `stemwise.loop`. The decisions use the values written in `evaluation.json` (one decimal), compared in
integer tenths (`round(10 · d)`), so that they can be derived from the run files alone (D-051).

- Stop after iteration `k` (`stop_reason`), in order:
  1. `below_threshold`: the iteration respects the constraints and `d_k < 1.5 dB`;
  2. `no_improvement`: `k ≥ 1` and `d_{k−1} − d_k < 0.2 dB` (including a worsening); iteration 0 is not
     compared with the raw sum;
  3. `max_iterations`: `k + 1` = maximum (`--max-iter`, otherwise `options.max_iterations`, default 3).
  - Without target (no reference, D-052): `no_target` at the first iteration that respects the constraints.
  - `decision_failed` (in `best.json`): the decision of the next iteration failed (D-055).
- Best iteration (EVA-6): minimum distance among the iterations that respect all the constraints; on a tie the
  oldest; without target the first valid one; none valid ⇒ `iteration: null`. `is_best` of an iteration says
  whether it was the best when it was evaluated; `best.json` gives the final one (D-049).
- Decision `k ≥ 1`: history of the previous iterations (plan + evaluation, the prompt keeps the last 2,
  DEC-1); variation limits of DEC-5 relative to the plan of iteration `k − 1`.

### Revision (REV-1, REV-2, D-060 → D-063)

Code: `stemwise.revise`, `stemwise.runs.revise_run`. No new measures.

- A single iteration (`max_iterations` = 1) with `stop_reason: revision`: the revised plan is the best of its run if it
  respects the technical constraints, without distance comparison with the starting plan (no rollback).
- Initial history = starting plan with its `evaluation.json`; DEC-5 relative to the starting plan.
- Analysis reused from the starting run; the evaluation target is re-measured from the references with the same
  functions as ANA-11 and ANA-12 (full precision, D-061). Consistency check: frequency, duration (one decimal),
  ids and order of the stems, per stem role, channels and sample peak (`peak_db`, one decimal), number of
  references and target profile rounded as in `analysis.json`.

## Master (MAS-1 → MAS-3, D-069 → D-076)

Version: `master v1` — code: `stemwise.master`, `stemwise.runs.master_run`. Contract:
`schemas/master_report.schema.json`. No new measures: ANA-1, ANA-7, ANA-11 and EVA-2 are reused.

- Inputs (D-073): the mix (`mix.wav` of the plan of `best.json`, or a file with `--mix`) at the project `fs`,
  without resampling (otherwise an error, as ING-3); the reference resampled to the project `fs` as in
  ING-5 (D-012). Mono ⇒ same signal on the two channels. Duration of both: more than 4,096 samples, at least 0.4 s (one
  loudness block) and at most 15 minutes; measurable loudness; mix and reference different.
- Processing (MAS-2, D-070, D-071): `matchering` 2.0.6, `matchering.stages.main(mix, reference, config)` in
  float64 on the arrays, with the default `Config` except `internal_sample_rate = fs`: `fft_size` 4,096,
  `lin_log_oversampling` 4, `rms_correction_steps` 4, LOWESS smoothing (`frac` 0.0375, `it` 0, `delta` 0.001),
  pieces analyzed of 15 s, threshold `(2^15 − 61)/2^15` ≈ −0.016 dBFS, "Hyrax" limiter (attack 1 ms, hold 1 ms,
  release 3,000 ms). In summary: the reference is normalized to the peak; a gain, computed on the mid and applied
  equally to mid and side, brings the mix to the RMS of the mid of the strongest pieces of the reference; then an FIR for mid and one
  for side apply the ratio between the mean spectra (smoothed on a logarithmic scale), four RMS corrections and
  the sample-peak limiter. Deterministic. At 44.1 kHz the output coincides within 1 LSB at 24 bits with the file of
  `mg.process` (test).
- Output (D-072, D-075): `master.wav` WAV PCM 24 bit at the project `fs`, as long as the mix; with `--pcm16`
  also `master_16.wav` PCM 16 from the same signal, without dither.
- Measures (MAS-3, D-076), on the 24-bit file read back: `lufs` (integrated loudness, section "Integrated loudness") and
  `true_peak_db` (section "True peak"); `true_peak_ok` if the true peak at full precision is ≤ −1.0 dBTP (as
  EVA-4: −0.96 dBTP is written −1.0 but is not ok), otherwise a warning and no correction (D-074, D-078). `spectral_distance_db` and `band_delta_db` with the formula of
  EVA-2 between the bands of the master (ANA-7, section "Mix") and the bands of the **reference used** normalized (ANA-11,
  `reference v1`), not the average profile of ANA-12; the same distance for the starting mix, for comparison. Of the mix
  and of the reference also `lufs` and `true_peak_db` (the reference at the project `fs`, before the
  normalization), measured as they enter the master: a mono input counts on both channels (D-078). One decimal, levels limited to ±120 dB.
- Known limit: the library aligns the RMS at full band, including the energy below 20 Hz that the loudness ignores.
  With a lot of infrasonic content in the mix or the reference the master comes out more or less loud than the reference (on
  1/f pink noise down to 0.1 Hz: +1.9 LU; filtered at 20 Hz: −0.02 LU).
