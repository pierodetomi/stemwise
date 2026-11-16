# Misure e formule

Specifica neutra delle misure dell'engine (PRD §7.1): formula, finestre, parametri e versione di ogni
algoritmo, così che un'altra implementazione possa riprodurre gli stessi numeri. Il codice di riferimento è
indicato per ogni voce, ma la definizione è questo documento.

## Convenzioni

- Segnale: campioni in virgola mobile, fondo scala = 1,0, forma `(campioni, canali)`, 1 o 2 canali.
  Elaborazione interna in float32 (ING-6); le misure di livello calcolano in float64.
- dBFS: `20 · log10(|x|)` con `|x| = 1,0` ⇒ 0 dBFS. Il livello del silenzio digitale è `−∞`.
- Frequenza di campionamento `fs` del progetto: quella comune a tutti gli stem (ING-3, D-010).

## Picco campionato

- Versione: `sample_peak v1` — codice: `stemwise.analysis.stem.sample_peak_db`.
- Formula: `20 · log10( max_{n,c} |x[n,c]| )`, massimo su tutti i campioni e tutti i canali.

## True peak

- Versione: `true_peak v1` (D-009) — codice: `stemwise.analysis.stem.true_peak_db`.
- Oversampling di fattore `L = 4` (ANA-1), indipendente da `fs`.
- Interpolazione: inserimento di `L − 1` zeri tra campioni consecutivi e filtro FIR passa-basso a fase
  lineare, applicato a ogni canale separatamente (implementazione: `scipy.signal.resample_poly(x, 4, 1)`).
  - Lunghezza: `2 · 10 · L + 1 = 81` tap alla frequenza `L · fs`.
  - Coefficienti: `g[k] = sinc((k − 40) / L) · w[k]`, `k = 0…80`, con `sinc(t) = sin(πt) / (πt)` e `w`
    finestra di Kaiser di parametro `β = 5,0` su 81 punti; `h[k] = L · g[k] / Σ g` (guadagno L in continua,
    che compensa gli zeri inseriti). Taglio alla Nyquist originale (`fs / 2`). Verificato: coincide con il
    filtro usato da `resample_poly` (scarto massimo 0).
  - Ritardo compensato: il campione interpolato `y[m]` corrisponde all'istante `m / (L · fs)`.
- Bordi: prima del primo e dopo l'ultimo campione il segnale vale 0 (silenzio). Un attacco brusco a fondo
  scala produce quindi sovraelongazioni reali tra campioni, che vengono misurate.
- Formula: `TP = 20 · log10( max( max_{n,c} |x[n,c]|, max_{m,c} |y[m,c]| ) )` dBTP, cioè mai sotto il
  picco campionato.
- Calcolo in blocchi da 65.536 campioni con 32 campioni di contesto per lato (più della semilunghezza del
  filtro, 10 campioni d'ingresso): il risultato coincide con quello in un passo solo.
- Accuratezza verificata nei test: seno a `fs / 4` con fase 45° (picco campionato −3,01 dBFS, picco reale
  0 dBFS) misurato a +0,015 dBTP; seno a 997 Hz misurato entro +0,006 dB. Tolleranza dei test ±0,1 dB.
- Differenze rispetto a ITU-R BS.1770-4 Annex 2: la norma usa un FIR di 48 coefficienti (12 per fase); qui
  il FIR è più lungo e più piatto in banda passante. A parità di fattore 4x i due possono differire di
  qualche centesimo di dB sui picchi tra campioni vicini alla Nyquist.

## Bus di mix: somma e trim

Usati dalla somma grezza `baseline.wav` (REN-7) e, da M2, dal bus del mix (REN-5).
Codice: `stemwise.render.mixbus`.

- Versione: `mixbus v1` (D-007, D-008).
- Stem mono sul bus stereo: `L = R = x · √½` (−3,01 dB per canale, potenza costante al centro, la stessa
  legge del pan di REN-3). Stem stereo: invariati. Il bus è sempre stereo.
- Somma: `B[n,c] = Σ_s S_s[n,c]`, nell'ordine degli stem in `project.yaml`, con accumulatore float64 e
  risultato in float32. Gli stem hanno già la stessa lunghezza (padding di ING-4).
- Trim (solo attenuazione): con `TP_B` il true peak del bus, `C = mix_true_peak_db` (default −3,0) e
  margine `m = 0,01 dB`, il guadagno è `g_dB = min(0, (C − m) − TP_B)` e il bus diventa
  `B · 10^(g_dB / 20)`. Se `g_dB = 0` il bus resta identico, campione per campione.
- Motivo del margine: dopo la quantizzazione a 24 bit (scarto ≤ 1 LSB = 2⁻²³) il file riletto resta entro
  `C`. Verificato nei test: true peak dopo il trim in [−3,02; −3,00] dBTP e file riletto ≤ −3,0 dBTP.
- Somma grezza (REN-7): stem originali, senza pre-gain né piano, poi solo il trim.

## STFT di analisi e parti attive

Base comune di bande, attività, centroide, misure stereo e mascheramento (D-014). Codice:
`stemwise.analysis.spectral`.

- Versione: `spectral v1`.
- Finestra: Hann periodica `w[n] = 0,5 − 0,5·cos(2πn / N)`, `n = 0…N−1`.
- Dimensione: `N = 8192 · 2^round(log2(fs / 48000))`, cioè 8192 a 44,1 e 48 kHz (≈170 ms, bin da 5,9 Hz a
  48 kHz) e 16384 a 88,2 e 96 kHz. Hop `H = N / 4`.
- Griglia: il frame `t` copre i campioni `[t·H, t·H + N)`; il numero di frame è `1 + ⌈max(n − N, 0) / H⌉` per
  un segnale di `n > 0` campioni; l'ultimo frame è completato con zeri.
- Potenza di un bin, per canale: `p[k] = c_k · |X[k]|² / (N · Σ w²)`, con `X = rfft(w · x)`, `c_k = 1` per
  `k = 0` e `k = N/2`, `c_k = 2` altrimenti. Per Parseval `Σ_k p[k] = Σ w²x² / Σ w²`: la media quadratica
  pesata Hann del frame (un seno di ampiezza A dà `A²/2`).
- Potenza del frame (D-022): `P[t] = media sui canali di Σ_{k: 20 Hz ≤ f_k ≤ 20 kHz} p[k]`, cioè sugli stessi bin
  delle bande. Livello: `10·log10(P[t])` dBFS. Continua e rombo sotto 20 Hz non contano: un DC offset sopra lo
  0,32% non rende più "attivo" tutto lo stem. Per un segnale tutto in banda `P[t]` è la media quadratica pesata
  Hann del frame.
- **Frame attivo**: `P[t] > 10^(−50/10)`, cioè livello sopra −50 dBFS (PRD §5.2).
- `active_pct` (ANA-4) = `100 · frame attivi / frame totali`. Secondi attivi = `active_pct / 100 · durata`.
- Potenza sul bus (ANA-8, ANA-9): potenza media sui canali × numero di canali, cioè `x²` per uno stem mono e
  `L² + R²` per uno stereo, la potenza che lo stem porta sul bus stereo con la legge di D-008.
- La STFT è calcolata a blocchi di 128 frame, in float64, e ridotta subito a potenze per frame e banda, spettro
  medio dei frame attivi e somme stereo: lo spettrogramma completo non viene mai tenuto in memoria.

## Bande (ANA-3)

- Versione: `bands v1` — codice: `stemwise.analysis.spectral.band_levels_db`.
- Bande fisse in Hz: 20–60, 60–120, 120–250, 250–500, 500–2k, 2k–4k, 4k–8k, 8k–20k. Il bin a frequenza
  `f = k · fs / N` appartiene alla banda se `lo ≤ f < hi`; l'ultima banda include 20 kHz ed è limitata a `fs / 2`.
  I bin sotto 20 Hz e sopra 20 kHz non entrano in nessuna banda.
- Energia della banda `b`: `E_b = Σ_{t attivi} Σ_{k ∈ b} p̄_t[k]`, con `p̄` la potenza media sui canali.
- Valore: `10·log10(E_b / Σ_b E_b)` dB, quindi `Σ_b 10^(valore/10) = 1`. Nessun frame attivo o nessuna energia
  in 20 Hz–20 kHz ⇒ `null`.
- Verificato nei test: un seno a 1 kHz ha ≥ 98,9% dell'energia in 500–2k e meno di −60 dB altrove; un rumore
  rosa con spettro esatto `1/f` si scosta al massimo di 0,15 dB dal valore teorico
  `10·log10(log2(hi/lo) / log2(1000))` (lo scarto maggiore è nella banda 60–120 Hz).

## Centroide spettrale (ANA-4)

- Versione: `centroid v1` — codice: `stemwise.analysis.spectral.centroid_hz`.
- Spettro medio dei frame attivi `S[k] = media_{t attivi} p̄_t[k]`; centroide
  `Σ f_k · S[k] / Σ S[k]` sui bin con 20 Hz ≤ `f_k` ≤ 20 kHz, in Hz. È pesato in **potenza** (due seni di pari
  ampiezza a 500 e 2000 Hz danno 1250 Hz). Nessun frame attivo ⇒ `null`.

## Loudness

Codice: `stemwise.analysis.loudness`. Libreria: `pyloudnorm` 0.2.0. Un misuratore nuovo per ogni chiamata.

### Loudness integrato (ANA-1)

- Versione: `lufs v1` — ITU-R BS.1770-4.
- K-weighting come in `pyloudnorm` (`filter_class="K-weighting"`): due biquad RBJ calcolati per la `fs` del
  progetto, high shelf `G = +4 dB`, `Q = 1/√2`, `fc = 1500 Hz`, poi passa-alto `Q = 0,5`, `fc = 38 Hz`.
- Blocchi da 400 ms con sovrapposizione del 75%; loudness di blocco `−0,691 + 10·log10(Σ_c G_c · z_c)`, con
  `G = 1` per L e R; gate assoluto −70 LUFS, gate relativo −10 LU sotto la media dei blocchi sopra il gate
  assoluto. Calcolo sul segnale intero (il gate di −50 dBFS di D-014 non si applica: il gating della norma
  esclude già il silenzio).
- Stem mono: un solo canale con `G = 1`, quindi un seno a 997 Hz di ampiezza 1 dà −3,01 LUFS teorici.
- Casi limite: tutti i blocchi esclusi dal gate (silenzio) ⇒ `−∞` (−120 nel JSON, D-018); segnale più corto di
  un blocco da 400 ms ⇒ `null`.
- Accuratezza misurata: seno 997 Hz a −20 dBFS mono ⇒ −23,05 LUFS (teorico −23,01): lo scarto di 0,04 LU viene
  dall'approssimazione RBJ dei filtri della norma, entro la tolleranza ±0,1 LU di EBU Tech 3341.

### Escursione dinamica (ANA-2)

- Versione: `lra v1` (D-015) — EBU Tech 3342, implementazione `pyloudnorm.Meter.loudness_range`.
- Loudness short-term: stessa formula dei blocchi, con finestre di 3 s e sovrapposizione del 97%, quindi passo
  di 90 ms (11,1 valori al secondo; la norma ne chiede almeno 10). Al segnale vengono aggiunti 1,5 s di
  silenzio in coda, come nell'esempio della norma.
- Gate assoluto −70 LUFS, poi gate relativo −20 LU sotto la loudness media (in potenza) delle finestre rimaste.
- Valore: `P95 − P10` delle finestre rimaste, percentili con interpolazione lineare (`numpy.percentile`).
- Casi limite: segnale più corto di 1,5 s, o tutto escluso dal gate ⇒ `null`.
- Verificato nei test con i quattro segnali di Tech 3342 (seno 1 kHz stereo, 20 s per livello: −20/−30,
  −20/−15, −40/−20, −50/−35/−20/−35/−50 dBFS ⇒ 10, 5, 20, 15 LU): scarto misurato < 0,001 LU.

## Livelli dello stem (ANA-1)

Codice: `stemwise.analysis.stem`.

- Picco campionato e true peak: sezioni omonime sopra, sul segnale intero.
- RMS (`rms v2`, D-022): `10·log10( media_{t attivi} P[t] )` dBFS, con `P[t]` la potenza dei frame della STFT di
  analisi, limitata a 20 Hz–20 kHz (DC e sub-20 Hz esclusi, come per le bande); nessun frame attivo ⇒ `null`. Un seno di ampiezza A dà `20·log10(A/√2)`. Sui segnali con tratti
  attivi e silenzio i frame a cavallo dei bordi abbassano l'RMS di qualche centesimo di dB (misurato: 0,04 dB
  su 10 s di seno + 10 s di silenzio).
- Crest factor (`crest v1`): picco campionato − RMS sui frame attivi, in dB.

## Igiene (ANA-6)

Codice: `stemwise.analysis.stem`.

- DC offset (`dc v1`, D-018): `100 · max_c |media_n x[n,c]|`, in percentuale del fondo scala, sul segnale intero.
- Campioni in clipping (`clip v1`): numero di campioni, su tutti i canali, con `|x| ≥ 1 − 2⁻¹⁵`. La soglia è il
  massimo positivo di un PCM a 16 bit riletto (32767 / 32768): conta i fondo scala a 16 e 24 bit e gli "over" dei
  file float.
- Rumore di fondo (`noise_floor v2`, potenza in 20 Hz–20 kHz): `10·log10( mediana_{t inattivi} P[t] )` dBFS sui frame inattivi
  (livello ≤ −50 dBFS); `−∞` (−120 nel JSON) se le pause sono silenzio digitale. `null` se il tempo inattivo
  (frazione di frame inattivi × durata) è sotto 1 s.

## Densità dei transienti (ANA-4)

- Versione: `onsets v2` (D-020, D-023) — codice: `stemwise.analysis.stem.onsets_per_s`. Libreria: `librosa` 1.0.0, con
  Python 3.12 (`.python-version`). Attenzione: con Python 3.11, ammesso da `requires-python`, il lock risolve
  `librosa` 0.11.0 e i valori di onset potrebbero differire; i numeri di riferimento sono quelli con 3.12.
- Segnale: ogni canale separatamente, sul segnale intero; l'inviluppo usato è la media degli inviluppi dei canali
  (non l'inviluppo della media dei canali, che cancellerebbe l'antifase).
- Inviluppo: `librosa.onset.onset_strength` con i default: spettrogramma mel di potenza (`n_fft 2048`, `hop 512`,
  128 bande), in dB (`power_to_db`, `ref 1`, `top_db 80`), differenza positiva tra frame consecutivi (`lag 1`),
  media sulle bande.
- Picchi: `librosa.onset.onset_detect` con `normalize=False` e `delta = 2,0`; gli altri parametri di default
  (`pre_max 30 ms`, `post_max 0`, `pre_avg 100 ms`, `post_avg 100 ms`, `wait 30 ms`). Un onset è un picco
  dell'inviluppo che supera di 2 dB la media locale.
- Filtro sulle parti attive (D-023): un onset alla posizione `s` (campioni) appartiene al frame di analisi con il
  centro più vicino, `t = round((s − N/2) / H)` limitato a `[0, frame − 1]`; contano solo gli onset di frame attivi.
  Così i transitori sotto −50 dBFS nelle pause (rientro della click dalle cuffie) non vengono contati.
- Valore: numero di onset in frame attivi / secondi attivi (D-014); 0 se lo stem non ha frame attivi.
- Verificato nei test: click a 4/s su 10 s ⇒ 4,0, anche in antifase (L = −R); seno, rumore bianco e rosa
  stazionari ⇒ ≤ 0,1 onset/s; 5 s di voce + 15 s di rientri a −60 dBFS ⇒ 0,2 onset/s (senza il filtro: 12). Un
  arresto brusco del segnale può contare come onset.

## Stereo (ANA-5)

- Versione: `stereo v1` — codice: `stemwise.analysis.stem.stereo_measures`. Solo stem stereo; mono ⇒ `null`.
- Somme sui frame attivi e sui bin con 20 Hz ≤ f ≤ 20 kHz, con la stessa scala `c_k / (N·Σw²)` delle potenze:
  `LL = Σ|L|²`, `RR = Σ|R|²`, `LR = Σ Re(L·R*)`, `MM = Σ|(L+R)/2|²`, `SS = Σ|(L−R)/2|²`, con `L`, `R` gli spettri
  della STFT di analisi.
- Correlazione: `LR / √(LL·RR)`, in [−1, 1]: è la correlazione a ritardo zero dei segnali filtrati in banda
  (Parseval), pesata Hann. Un canale muto ⇒ 0; entrambi muti ⇒ `null`.
- Rapporto side/mid: `10·log10(SS / MM)` dB. Dual mono ⇒ `−∞`, antifase pura ⇒ `+∞` (±120 nel JSON, D-018);
  nessun segnale ⇒ `null`. Hard pan di un mono (`R = 0`) ⇒ 0 dB.

## Mix (ANA-7, ANA-8, ANA-10)

Codice: `stemwise.analysis.mix`. Il mix analizzato è la somma grezza con il trim, cioè il segnale di
`baseline.wav` (D-017); è sempre stereo (D-008).

- Misure del mix (ANA-7): LUFS integrato, picco, true peak, RMS, crest, LRA, bande e stereo, con le stesse
  formule e versioni delle misure dello stem.
- Quote per banda (ANA-8, `share v2`, livelli di D-028): `E_{s,b} = Σ_{tutti i frame} potenza sul bus` dello stem `s` nella banda
  `b` (mono `x²`, stereo `L² + R²`); quota `100 · E_{s,b} / Σ_s E_{s,b}` %. Le quote di una banda sommano a 100;
  una banda dove nessuno stem ha energia dà 0 per tutti. Il denominatore è la somma delle energie, non l'energia
  del mix: con stem correlati (doppiature, microfoni della batteria) le due cose differiscono.
- Livelli degli stem per quote e mascheramento (D-028, `share v2`, `masking v2`): con `pre_normalize: true` (il
  default) quote per banda e mascheramento si calcolano sugli stem moltiplicati per il pre-gain di REN-1 (sezione
  "Render", portati a −23 LUFS), cioè sul punto di partenza del piano; con `pre_normalize: false` sugli stem
  grezzi. Le misure per stem e il blocco `mix` restano sui segnali grezzi.
- Correlazione delle basse (ANA-10, `low_corr v1`): stessa formula della correlazione di ANA-5, sui bin con
  20 Hz ≤ f < 120 Hz e sui frame attivi del mix. Segnale mono ⇒ 1; nessun frame attivo ⇒ `null`.

## Mascheramento (ANA-9)

- Versione: `masking v2` (D-016; livelli degli stem di D-028, vedi "Mix") — codice: `stemwise.analysis.masking`.
- Per ogni coppia di stem `(a, b)` e ogni banda `k`, frame per frame sulla griglia comune della STFT di analisi
  (gli stem hanno la stessa lunghezza, ING-4):
  - stem attivo nella banda: potenza in banda media sui canali > −50 dBFS;
  - `E_a[t]`, `E_b[t]`: potenza in banda sul bus (mono `x²`, stereo `L² + R²`);
  - punteggio `Σ_{t: entrambi attivi} min(E_a, E_b) / Σ_{t: almeno uno attivo} max(E_a, E_b)`, in [0, 1].
- Lettura: la frazione del tempo in cui i due stem si sovrappongono nella banda, pesata dal rapporto tra il più
  debole e il più forte. Due stem identici danno 1; una copia 12 dB più bassa dà `10^(−1,2) ≈ 0,063`; stem che
  non suonano mai insieme nella stessa banda danno 0.
- Nel JSON: i 10 punteggi più alti maggiori di 0, in ordine decrescente; a pari punteggio, coppie nell'ordine di
  `project.yaml` (prima `a`, poi `b`) e bande in ordine crescente.

## Riferimenti e profilo target (ANA-11, ANA-12)

Codice: `stemwise.analysis.reference`.

- Ingresso: il riferimento già ricampionato alla `fs` del progetto (ING-5, D-012), mono o stereo così com'è.
- Normalizzazione (`reference v1`): guadagno `10^((−23 − L) / 20)` con `L` il loudness integrato; −23 LUFS è lo
  stesso livello del pre-gain degli stem (REN-1). Un riferimento silenzioso o più corto di 400 ms dà errore.
- Misure: quelle del mix (ANA-7 e ANA-10) sul riferimento normalizzato. Bande, side/mid e LRA non dipendono dal
  guadagno (verificato: scarto < 0,05 tra un riferimento e la sua copia a −12 dB); il LUFS vale −23 per
  costruzione.
- Profilo target (`target v1`, ANA-12): media aritmetica dei valori dei riferimenti, campo per campo — bande (in
  dB, banda per banda), `side_mid_db`, `dyn_range_lu`. Un riferimento entra nella media di un campo solo se ha un
  valore finito: i riferimenti mono e dual mono non entrano in quella di side/mid. Nessun riferimento ⇒ `target`
  `null`; nessun valore per un campo ⇒ quel campo `null`.

## `analysis.json`: rappresentazione (ANA-13, D-018)

Contratto: `schemas/analysis.schema.json` (generato dal modello `stemwise.analysis.model.Analysis`).

- Le misure si calcolano a piena precisione; limiti e arrotondamento si applicano solo alla scrittura del file.
- Livelli in dB e LU limitati a [−120, +120]: `−∞` (silenzio digitale, dual mono) diventa −120, `+∞` (antifase
  pura) +120. Nel file non compaiono mai `Infinity` o `NaN`.
- Ogni numero reale è arrotondato a un decimale (arrotondamento di Python, al pari più vicino sulle metà esatte in
  binario), tranne le grandezze adimensionali in [−1, 1] — `correlation`, `low_corr` e `score` del mascheramento —
  arrotondate a due decimali (D-021); `−0,0` è scritto come `0,0`. Gli interi (`sample_rate`, `channels`, `clipped_samples`,
  `n_references`) restano interi.
- Misure che richiedono frame attivi su uno stem tutto inattivo: `null` (RMS, crest, bande, centroide, LRA,
  stereo); `onsets_per_s` e `active_pct` valgono 0.
- Campi in più rispetto all'esempio di PRD §6.2: `version` (1), `rms_db` per stem e mix, `peak_db` e `crest_db`
  per il mix, la lista `references` con le misure di ciascun riferimento. Un riferimento è identificato solo da
  un'etichetta neutra `ref1`, `ref2`, `ref3` (l'N-esimo di `project.yaml`), mai dal nome o dal percorso del file
  (D-024, privacy di PRD §7).
- Formato: oggetti indentati di due spazi, liste di numeri su una riga, chiavi nell'ordine del contratto, stem
  nell'ordine di `project.yaml`, nuova riga finale.
- Stima dei token: caratteri / 4. Caso peggiore costruito nei test (12 stem stereo con id di 32 caratteri,
  3 riferimenti, 10 collisioni, tutti i campi pieni): circa 11.650 caratteri, cioè circa 2.920 token, sotto il
  tetto di 8.000.

## Render (REN-1 → REN-7)

Applicazione di un `mix_plan.json` (contratto: `schemas/mix_plan.schema.json`, modello
`stemwise.decision.schema.MixPlan`). Codice: `stemwise.render`. Versione: `render v1`.

### Filtri ed EQ: biquad RBJ (D-029)

Codice: `stemwise.render.biquad`. Ogni filtro è un biquad del cookbook di R. Bristow-Johnson (Audio EQ Cookbook),
forma con Q, con i coefficienti divisi per `a0`:

- Intermedi: `w0 = 2π·f0/fs`, `α = sin(w0)/(2Q)`, `A = 10^(G/40)` (G = guadagno in dB). Si richiede
  `0 < f0 < fs/2` e `Q > 0`.
- Passa-alto (`hpf_hz`, e `return_hpf_hz` del riverbero): Butterworth del secondo ordine, `Q = 1/√2`
  (−3,01 dB a f0, 12 dB/ottava). `b = [(1+cos w0)/2, −(1+cos w0), (1+cos w0)/2]`, `a = [1+α, −2 cos w0, 1−α]`.
- Passa-basso (`lpf_hz`): stesso Q. `b = [(1−cos w0)/2, 1−cos w0, (1−cos w0)/2]`, `a = [1+α, −2 cos w0, 1−α]`.
- `peak`: `b = [1+αA, −2 cos w0, 1−αA]`, `a = [1+α/A, −2 cos w0, 1−α/A]`. Guadagno pieno a f0.
- `low_shelf`, con `k = 2√A·α`:
  `b = [A((A+1)−(A−1)cos w0+k), 2A((A−1)−(A+1)cos w0), A((A+1)−(A−1)cos w0−k)]`,
  `a = [(A+1)+(A−1)cos w0+k, −2((A−1)+(A+1)cos w0), (A+1)+(A−1)cos w0−k]`. Metà del guadagno a f0.
- `high_shelf`:
  `b = [A((A+1)+(A−1)cos w0+k), −2A((A−1)+(A+1)cos w0), A((A+1)+(A−1)cos w0−k)]`,
  `a = [(A+1)−(A−1)cos w0+k, 2((A−1)−(A+1)cos w0), (A+1)−(A−1)cos w0−k]`. Metà del guadagno a f0.
- Applicazione: forma diretta trasposta (`scipy.signal.sosfilt`) in float64, stato iniziale nullo, canali
  indipendenti; uscita in float32. Le bande di EQ si applicano in cascata nell'ordine del piano.
- Verifica: le curve di `peak` e shelf coincidono entro 0,05 dB con `PeakFilter`, `LowShelfFilter` e
  `HighShelfFilter` di `pedalboard` 0.9.26 (stesse formule, in float32).

### Pre-gain e gain (REN-1, D-033)

Codice: `stemwise.render.chain`.

- Pre-gain: `g_pre = −23 − LUFS(stem)`, con il loudness integrato BS.1770-4 dello stem grezzo (stessa funzione di
  ANA-1). Uno stem mono a −23 LUFS resta a −23 LUFS sul bus stereo, perché il centro a √½ per canale conserva la
  potenza. Con `pre_normalize: false`, `g_pre = 0`.
- Stem muto (tutti i blocchi sotto il gate di −70 LUFS) o più corto di un blocco da 400 ms: `g_pre = 0` con warning.
  Nessun tetto; warning se `g_pre > +24 dB`.
- Ogni guadagno in dB `G` (pre-gain, makeup, `gain_db`, mandata, ritorno) moltiplica per `10^(G/20)` in float64, con
  uscita in float32; `G = 0` lascia il segnale identico.
- Nessun clipping interno: prima del trim del bus i campioni possono superare ±1.

### Pan (REN-3, D-008, D-030)

Codice: `stemwise.render.pan`. Uscita sempre stereo.

- Stem mono, potenza costante: `θ = (p + 1)·π/4`, `L = x·cos θ`, `R = x·sin θ`. Valori esatti agli estremi
  (`p = −1` ⇒ `L = x`, `R = 0`; `p = +1` ⇒ `L = 0`, `R = x`) e al centro (`√½`, la legge della somma grezza).
- Stem stereo, bilanciamento lineare: `L' = L·min(1, 1 − p)`, `R' = R·min(1, 1 + p)`. Al centro lo stem resta
  identico; nessun canale viene mai alzato né spostato sull'altro lato.

### Compressore (REN-2, D-034)

Codice: `stemwise.render.chain.compress`. Implementazione della libreria, non una formula: `pedalboard.Compressor`
0.9.26 (JUCE `dsp::Compressor`), con `threshold_db`, `ratio`, `attack_ms`, `release_ms` del piano; stato azzerato
a ogni chiamata (`reset=True`), elaborazione in float32.

- Detector di picco, uno per canale: gli stem stereo **non** sono compressi in modo collegato (verificato: canale a
  −10 dBFS ridotto di 14,2 dB, canale 40 dB sotto invariato). Nessun knee.
- Misurato su un seno a 1 kHz di picco −10 dBFS, soglia −30, ratio 4 (riduzione teorica 15 dB): 14,2 dB con
  attacco/rilascio 10/120 ms, 14,8 con 1/100, 14,8 con 0,1/20, per il rilascio del detector tra i picchi.
- Il makeup (`makeup_db`) è uno stadio a parte, un guadagno puro subito dopo il compressore.
- Avviso per un port (PRD §7.1): un altro compressore non darà gli stessi campioni.

### Catena per stem (REN-2)

Codice: `stemwise.render.chain.render_stem`. Ordine fisso:
pre-gain → passa-alto → passa-basso → bande di EQ nell'ordine del piano → compressore → makeup → `gain_db` → pan.
Un blocco `null` (o `eq` vuoto) è saltato: il segnale passa identico. L'uscita è lo stem stereo dopo il pan, da cui
si prende la mandata al riverbero (post-fader, post-pan).

### Riverbero condiviso (REN-4, D-031)

Codice: `stemwise.render.reverb`. Implementazione della libreria: `pedalboard.Reverb` 0.9.26 (JUCE, tarature di
Freeverb), stato azzerato a ogni chiamata, float32.

- Mandata di ogni stem: `S_s = Y_s · 10^(reverb_send_db/20)`, con `Y_s` lo stem stereo dopo il pan (post-fader,
  post-pan). `reverb_send_db: null` ⇒ nessuna mandata.
- Bus delle mandate: `Σ_s S_s` (float64, poi float32) → `Reverb(room_size, damping, wet_level = 1, dry_level = 0,
  width = 1, freeze_mode = 0)` → passa-alto RBJ a `return_hpf_hz` (Q = 1/√2) → `return_gain_db` → bus del mix.
- `reverb: null`, oppure nessuno stem con una mandata ⇒ nessun ritorno.
- Il Freeverb somma L e R prima dei comb: la posizione della mandata non cambia la coda (verificato nei test).
  Nessun suono secco: la prima uscita arriva dopo circa 25 ms (il comb più corto).
- Durata: uguale a quella del bus delle mandate; la coda oltre la fine degli stem è tagliata.
- Livello del ritorno, misurato con rumore rosa stereo, `damping` 0,5, ritorno senza passa-alto né gain: +0,8 dB
  rispetto alla mandata con `room_size` 0, +1,8 con 0,35, +2,5 con 0,5, +6,8 con 0,9. Serve a tarare `return_gain_db`.
- Avviso per un port (PRD §7.1): un altro riverbero non darà gli stessi campioni.

### Bus del mix e stem esportati (REN-5, REN-6, D-032)

Codice: `stemwise.render.render.render_mix`.

- Il piano deve contenere esattamente gli stem del progetto e frequenze sotto `fs/2` (D-033), altrimenti errore;
  stem più corti di 3 campioni ⇒ errore.
- Bus: `B = Σ_s Y_s + R`, con gli stem `Y_s` dopo il pan nell'ordine di `project.yaml` e il ritorno del riverbero `R`
  per ultimo; accumulatore float64, risultato in float32; poi lo stesso trim della somma grezza (sezione "Bus di mix",
  D-007). Durata uguale a quella degli stem (D-031).
- Un piano neutro (gain 0, pan 0, blocchi `null`, `reverb: null`) con `pre_normalize: false` dà un bus identico, bit
  per bit, alla somma grezza.
- Stem esportati (`--export-stems`): `Y_s · 10^(g_trim/20)` per ogni stem (in `stems/`) e `R · 10^(g_trim/20)` per
  il ritorno (`reverb_return.wav`, accanto a `mix.wav`). La loro somma è il mix, a meno dell'arrotondamento float32 e della quantizzazione a 24 bit di
  ogni file. Un file a 24 bit taglia i campioni oltre ±1: se uno stem esportato supera 0 dBFS (il trim
  garantisce solo la somma) un warning segnala che la somma dei file non è più il mix.

## Valutazione (EVA-1 → EVA-4)

Versione: `evaluation v1` — codice: `stemwise.evaluate`. Contratti: `schemas/evaluation.schema.json`
(`evaluation.json` di ogni iterazione e `baseline_evaluation.json` della somma grezza) e `schemas/best.schema.json`
(D-049).

- Misure del mix (EVA-1): quelle del blocco `mix` di `analysis.json` (ANA-7 e ANA-10, sezione "Mix"), sul mix
  renderizzato con il trim, cioè il segnale di `mix.wav`; per la somma grezza sul segnale di `baseline.wav`.
- Distanza spettrale (EVA-2, D-050), sulle bande `m_i` del mix e `t_i` del profilo target (ANA-12), a piena
  precisione e limitate a ±120 dB:
  - `offset = (1/8) Σ t_i − (1/8) Σ m_i` (media aritmetica in dB);
  - `band_delta_i = t_i − (m_i + offset)`: positivo = al mix manca energia in quella banda;
  - `spectral_distance_db = (1/8) Σ |band_delta_i|`.
  - Senza target, o senza bande nel mix (nessun frame attivo), distanza e `band_delta_db` valgono `null`.
- Ampiezza e dinamica (EVA-3, D-050): `width_delta_db = target.side_mid_db − mix.side_mid_db` (positivo = il mix è
  più stretto), `dyn_delta_lu = target.dyn_range_lu − mix.dyn_range_lu` (positivo = il mix ha meno escursione).
  `null` se manca uno dei due valori.
- Vincoli tecnici (EVA-4, D-053), sulle misure a piena precisione del mix trimmato in memoria:
  - `true_peak_ok`: true peak ≤ `mix_true_peak_db` (il margine di 0,01 dB del trim, D-007, copre l'arrotondamento a
    24 bit del file);
  - `low_corr_ok`: `low_corr > 0`; `null` (nessun frame attivo sotto 120 Hz) conta come rispettato;
  - `dc_ok`: DC offset (ANA-6: massimo sui canali di `|media|`, in % del fondo scala) ≤ 0,1% (−60 dBFS).
- Rappresentazione: come `analysis.json` (un decimale, due per correlazioni e punteggi, D-021), con il layout dello
  stesso file.
- Collisioni residue (EVA-1, EVA-3, D-054): formula di ANA-9 (`masking v2`, sezione "Mascheramento"), i 10 punteggi
  più alti, sugli stem come arrivano al bus. Mix renderizzato: gli stem processati dopo il pan e prima del trim
  (`Y_s` della sezione "Bus del mix"), senza il ritorno del riverbero, che non è uno stem; lo spettro di ogni `Y_s` si
  calcola subito dopo il suo render. Somma grezza: gli stem originali come entrano nel bus di `baseline.wav`: gli stereo
  con gli spettri già calcolati dall'analisi, i mono centrati su due canali a `√½` (D-008), come uno stem mono
  renderizzato con pan al centro, così la soglia di attività (media sui canali) è la stessa per la somma grezza e
  per il mix. In entrambi i casi si usano gli stem prima del trim: con la soglia assoluta di −50 dBFS il punteggio
  dipende dal livello, e così non dipende da quanto il bus viene attenuato. Il `masking_top` di `analysis.json` resta
  sugli stem pre-normalizzati (D-028), quindi con `pre_normalize: true` differisce da quello della somma grezza.
  Esempio: due stem identici danno 1,0 nella somma grezza; con uno dei due a −12 dB di gain danno `10^(−1,2) ≈
  0,06` in ogni banda; il pan non cambia il punteggio (la potenza sul bus non dipende dal pan, REN-3).

## Ciclo di verifica (EVA-5, EVA-6)

Codice: `stemwise.loop`. Le decisioni usano i valori scritti in `evaluation.json` (un decimale), confrontati in
decimi interi (`round(10 · d)`), così si possono ricavare dai soli file del run (D-051).

- Stop dopo l'iterazione `k` (`stop_reason`), nell'ordine:
  1. `below_threshold`: l'iterazione rispetta i vincoli e `d_k < 1,5 dB`;
  2. `no_improvement`: `k ≥ 1` e `d_{k−1} − d_k < 0,2 dB` (anche un peggioramento); l'iterazione 0 non si
     confronta con la somma grezza;
  3. `max_iterations`: `k + 1` = massimo (`--max-iter`, altrimenti `options.max_iterations`, default 3).
  - Senza target (nessun riferimento, D-052): `no_target` alla prima iterazione che rispetta i vincoli.
  - `decision_failed` (in `best.json`): la decisione dell'iterazione successiva è fallita (D-055).
- Iterazione migliore (EVA-6): distanza minima tra le iterazioni che rispettano tutti i vincoli; a parità la più
  vecchia; senza target la prima valida; nessuna valida ⇒ `iteration: null`. `is_best` di un'iterazione dice se
  era la migliore quando è stata valutata; `best.json` dà quella finale (D-049).
- Decisione `k ≥ 1`: storia delle iterazioni precedenti (piano + valutazione, il prompt tiene le ultime 2,
  DEC-1); limiti di variazione di DEC-5 rispetto al piano dell'iterazione `k − 1`.

### Revisione (REV-1, REV-2, D-060 → D-063)

Codice: `stemwise.revise`, `stemwise.runs.revise_run`. Nessuna misura nuova.

- Una sola iterazione (`max_iterations` = 1) con `stop_reason: revision`: il piano rivisto è il migliore del suo
  run se rispetta i vincoli tecnici, senza confronto di distanza con il piano di partenza (niente rollback).
- Storia iniziale = piano di partenza con la sua `evaluation.json`; DEC-5 rispetto al piano di partenza.
- Analisi riusata dal run di partenza; il target della valutazione è rimisurato dai riferimenti con le stesse
  funzioni di ANA-11 e ANA-12 (piena precisione, D-061). Controllo di coerenza: frequenza, durata (un decimale),
  id e ordine degli stem, per stem ruolo, canali e picco di campione (`peak_db`, un decimale), numero dei
  riferimenti e profilo target arrotondato come in `analysis.json`.

## Master (MAS-1 → MAS-3, D-069 → D-076)

Versione: `master v1` — codice: `stemwise.master`, `stemwise.runs.master_run`. Contratto:
`schemas/master_report.schema.json`. Nessuna misura nuova: si riusano ANA-1, ANA-7, ANA-11 ed EVA-2.

- Ingressi (D-073): il mix (`mix.wav` del piano di `best.json`, o un file con `--mix`) alla `fs` del progetto,
  senza ricampionamento (altrimenti errore, come ING-3); il riferimento ricampionato alla `fs` del progetto come in
  ING-5 (D-012). Mono ⇒ stesso segnale sui due canali. Durata di entrambi: più di 4.096 campioni, almeno 0,4 s (un
  blocco di loudness) e al massimo 15 minuti; loudness misurabile; mix e riferimento diversi.
- Elaborazione (MAS-2, D-070, D-071): `matchering` 2.0.6, `matchering.stages.main(mix, riferimento, config)` in
  float64 sugli array, con `Config` di default salvo `internal_sample_rate = fs`: `fft_size` 4.096,
  `lin_log_oversampling` 4, `rms_correction_steps` 4, livellamento LOWESS (`frac` 0,0375, `it` 0, `delta` 0,001),
  pezzi analizzati di 15 s, soglia `(2^15 − 61)/2^15` ≈ −0,016 dBFS, limiter "Hyrax" (attacco 1 ms, hold 1 ms,
  release 3.000 ms). In sintesi: il riferimento si normalizza al picco; un guadagno, calcolato sul mid e applicato
  uguale a mid e side, porta il mix all'RMS del mid dei pezzi più forti del riferimento; poi un FIR per mid e uno
  per side applicano il rapporto tra gli spettri medi (livellati in scala logaritmica), quattro correzioni di RMS e
  il limiter a picco di campione. Deterministico. A 44,1 kHz l'uscita coincide entro 1 LSB a 24 bit con il file di
  `mg.process` (test).
- Uscita (D-072, D-075): `master.wav` WAV PCM 24 bit alla `fs` del progetto, lungo quanto il mix; con `--pcm16`
  anche `master_16.wav` PCM 16 dallo stesso segnale, senza dither.
- Misure (MAS-3, D-076), sul file a 24 bit riletto: `lufs` (loudness integrato, sezione "Loudness integrato") e
  `true_peak_db` (sezione "True peak"); `true_peak_ok` se il true peak a piena precisione è ≤ −1,0 dBTP (come
  EVA-4: −0,96 dBTP si scrive −1,0 ma non è ok), altrimenti un warning e nessuna correzione (D-074, D-078). `spectral_distance_db` e `band_delta_db` con la formula di
  EVA-2 tra le bande del master (ANA-7, sezione "Mix") e le bande del riferimento **usato** normalizzato (ANA-11,
  `reference v1`), non il profilo medio di ANA-12; la stessa distanza per il mix di partenza, per confronto. Del mix
  e del riferimento anche `lufs` e `true_peak_db` (il riferimento alla `fs` del progetto, prima della
  normalizzazione), misurati come entrano nel master: un ingresso mono vale sui due canali (D-078). Un decimale, livelli limitati a ±120 dB.
- Limite noto: la libreria allinea l'RMS a banda piena, compresa l'energia sotto i 20 Hz che il loudness ignora.
  Con molto contenuto infrasonico nel mix o nel riferimento il master esce più o meno forte del riferimento (su
  rumore rosa 1/f fino a 0,1 Hz: +1,9 LU; filtrato a 20 Hz: −0,02 LU).

