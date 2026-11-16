# Decisioni e domande aperte

Registro delle decisioni prese durante lo sviluppo e delle domande che bloccano o condizionano una milestone.
Ogni voce: id, data, contesto, decisione/domanda, motivazione, milestone interessate. Le decisioni non si
cancellano: se cambiano, si aggiunge una nuova voce che sostituisce la precedente ("Sostituisce D-00x").

## Domande aperte

Da PRD §11 ("Da decidere prima di partire") e dalla lettura del PRD durante il setup.

| Id | Domanda | Blocca | Note |
| --- | --- | --- | --- |
| Q-7 | PRD §4 cita un diagramma "pipeline dell'engine · 6 passi, 1 ciclo" non presente nel file esportato. | Nessuna | Il testo di §4 basta; se il diagramma aggiunge vincoli, va riportato nel PRD. |
| Q-40 | Se un provider dichiara `structured_outputs` ma rifiuta lo schema `strict` (per esempio `minimum`/`maximum`), o se `require_parameters` con `seed`/`temperature`/`reasoning` non trova nessun endpoint, la chiamata fallisce con HTTP 400/404 e non c'è riserva: D-045 sceglie la modalità solo da `supported_parameters` (che su `/models` è l'unione dei provider). | Nessuna oggi; da decidere se succede nella validazione manuale di M3 | Rilievi 2 e 11 della revisione di chiusura di M3. (a) **Consigliata, solo se il caso si presenta**: variabile `STEMWISE_OUTPUT_MODE=json_object` per forzare la modalità JSON + validazione su quel modello, e non mandare `seed` (beneficio marginale) quando restringe l'instradamento. (b) Ripiego automatico: se il primo tentativo con `json_schema` riceve un 400 sullo schema, ripetere una volta con `json_object` (un tentativo in più fuori dal conteggio di DEC-4). (c) Lasciare così e scartare i modelli che non reggono lo schema. |

## Domande chiuse

- Q-1 (stem grezzi o bilanciati) → D-025.
- Q-2 (modello di default) → D-047 (opzione c, scelta dall'autore dopo il confronto di M3).
- Q-4 (nome del pacchetto) → D-006.
- Q-8 (trim) → D-007. Q-9 (mono nella somma grezza) → D-008. Q-10 (filtro del true peak) → D-009.
- Q-11 (validazione di `project.yaml`) → D-010. Q-12 (formati) → D-011 (integrata da D-013). Q-13 (resampling dei riferimenti) → D-012.
- Q-14 → D-014, Q-15 → D-015, Q-16 → D-016, Q-17 → D-017, Q-18 → D-018, Q-19 → D-019 (M1, opzioni consigliate).
- Q-20 (decimali di correlazioni e punteggi) → D-021.
- Q-22 (attività a banda piena) → D-022, Q-21 (onset nelle pause) → D-023, Q-23 (nomi dei riferimenti) → D-024
  (rilievi della revisione di chiusura di M1, opzioni consigliate).
- Q-5 → D-026, Q-6 → D-027, Q-24 → D-028, Q-25 → D-029, Q-26 → D-030, Q-27 → D-031, Q-28 → D-032,
  Q-29 → D-033 (M2, opzioni consigliate, approvate dall'autore con il piano).
- Q-30 → D-036, Q-31 → D-037, Q-32 → D-038, Q-33 → D-039, Q-34 → D-040, Q-35 → D-041, Q-36 → D-042,
  Q-37 → D-043, Q-38 → D-044, Q-39 → D-045 (M3, opzioni consigliate, approvate dall'autore con il piano).
- Q-41 → D-049, Q-42 → D-050, Q-43 → D-051, Q-44 → D-052, Q-45 → D-053, Q-46 → D-054, Q-47 → D-055,
  Q-48 → D-056, Q-49 → D-057 (M4, opzioni consigliate, approvate dall'autore con il piano).
- Q-50 → D-059, Q-51 → D-060, Q-52 → D-061, Q-53 → D-062, Q-54 → D-063, Q-55 → D-064, Q-56 → D-065,
  Q-57 → D-066 (M5, opzioni consigliate, approvate dall'autore con il piano).
- Q-58 → D-069, Q-59 → D-070, Q-60 → D-071, Q-61 → D-072, Q-62 → D-073, Q-63 → D-074, Q-64 → D-075,
  Q-65 → D-076 (M6, opzioni consigliate, approvate dall'autore con il piano).
- Q-3 (i 3 brani per l'ascolto) e Q-66 → Q-72 (comando `ab`, scheda, conteggio delle ipotesi): superate da D-079,
  senza decisione nel merito (M7 ridotta, protocollo §10.2 non eseguito). Il testo delle domande resta nella storia
  di git (commit `M7: plan`).

## Decisioni

### D-001 — Toolchain e struttura del repository (2026-11-09, setup)

- Python fissato a 3.12 (`.python-version`), `requires-python = ">=3.11,<3.14"`: il PRD chiede ≥ 3.11 e il
  limite superiore evita versioni non ancora supportate da `numba`/`librosa`.
- Progetto `uv`, layout `src/`, build backend `hatchling`. Qualità: `ruff` (format + lint) e `pytest`.
  Nessun type checker per ora: le librerie audio hanno stub incompleti; valutabile più avanti.
- Dipendenze runtime aggiunte milestone per milestone, solo quando servono (in M0: `typer`, `pydantic`,
  `pyyaml`, `numpy`, `soundfile`, più quanto serve per il true peak).
- `Makefile` come punto d'ingresso unico (`make check` è il gate).

### D-002 — Garanzie automatiche sulle regole del PRD (2026-11-09, setup)

- `tests/conftest.py` blocca le connessioni di rete in ogni test non marcato `llm` (PRD §9).
- `pytest` esclude di default i test `llm` (`-m "not llm"`), `--strict-markers` evita marker scritti male.
- Tracciabilità: marker `req("ID")` e `acceptance("Mx")`; `scripts/req_coverage.py` confronta gli id dei
  requisiti nel PRD con quelli citati nei test.
- `.gitignore` esclude audio (`*.wav`, `*.flac`, …), `runs/`, `local/`, `.env`.

### D-003 — Flusso git (2026-11-09, setup)

Branch per milestone `milestone/mx-<slug>`, commit piccoli con test verdi, merge `--no-ff` su `main` e tag
`mx` solo dopo l'approvazione esplicita dell'autore. Nessun remote configurato al momento.

### D-004 — Macchina di sviluppo (2026-11-09, setup) — da confermare

Rilevata: macOS (Darwin 24.6), Apple M2, arm64. Risponde alla domanda di PRD §11 sul sistema operativo,
salvo diversa indicazione dell'autore. Implicazione: le dipendenze devono avere wheel `macosx_arm64`.

### D-005 — PRD rinominato (2026-11-09, setup)

`docs/PRD — PoC engine di mixing e mastering assistiti da AI.md` → `docs/PRD.md`, per referenziarlo da
`CLAUDE.md` e dai comandi senza spazi e caratteri speciali nel percorso. Contenuto invariato.

### D-006 — Nome del pacchetto: `stemwise` (2026-11-09, setup)

Chiude Q-4. Pacchetto Python, modulo (`src/stemwise/`) e comando CLI si chiamano `stemwise`, al posto del
provvisorio `mixpoc` del PRD. Il PRD non viene modificato: dove scrive `mixpoc` si legge `stemwise`. La cartella
del repository resta `mixpoc`. Il nome vale per la PoC; quello di un eventuale prodotto si decide dopo il go
(PRD §7.1).

### D-007 — Trim del bus: solo attenuazione, obiettivo limite − 0,01 dB (2026-11-09, M0)

Chiude Q-8 (opzione b, scelta dall'autore). Il bus viene attenuato solo se il true peak supera
`mix_true_peak_db`; in quel caso il guadagno porta il true peak a `mix_true_peak_db − 0,01 dB`, così il file a
24 bit riletto resta entro il limite nonostante l'arrotondamento. Una somma già entro il limite non viene toccata.
Precisazione in sviluppo: la regola implementata è `g_dB = min(0, (C − 0,01) − TP)`, quindi anche un true peak
nella fascia (C − 0,01; C] viene attenuato di quel poco che serve; altrimenti un file appena sotto C potrebbe
superarlo dopo la quantizzazione (formula in `docs/analysis.md`).

### D-008 — Bus sempre stereo, stem mono al centro a potenza costante (2026-11-09, M0)

Chiude Q-9 (opzione a, scelta delegata all'agente). La somma grezza è sempre stereo; uno stem mono entra al
centro con guadagno √½ (−3,01 dB) per canale, la stessa legge del pan di REN-3. Motivo: un piano neutro
(gain 0, pan 0, nessun blocco, `pre_normalize: false`) riproduce la baseline, quindi il confronto tra somma
grezza e mix AI misura solo le scelte del piano e non un diverso trattamento dei mono.

### D-009 — True peak: oversampling 4x con `scipy.signal.resample_poly` (2026-11-09, M0)

Chiude Q-10 (opzione a, scelta delegata all'agente). Filtro di default di `resample_poly(x, 4, 1)`: FIR
Kaiser β = 5,0, 81 tap, frequenza di taglio alla Nyquist originale. Motivo: è un interpolatore più lungo e
più piatto del FIR di riferimento BS.1770-4 (48 tap), quindi sottostima meno i picchi tra campioni, ed è
descrivibile per intero con parametri espliciti in `docs/analysis.md`. Se in M1 serve allinearsi ai misuratori
commerciali, si potrà confrontare con il filtro della norma.

### D-010 — Validazione rigida di `project.yaml`, con `sample_rate: auto` (2026-11-09, M0)

Chiude Q-11 (opzione a, con un'aggiunta dell'autore). `name`, `sample_rate`, `brief`, `stems` obbligatori;
2–16 stem e 0–3 riferimenti; id unici nel formato `^[a-z0-9][a-z0-9_]*$`; campi sconosciuti rifiutati;
`options` facoltativo coi default del PRD, `mix_true_peak_db` ≤ −3,0, `max_iterations` 1–3.
`sample_rate` accetta un intero oppure `"auto"`: con `auto` la frequenza si ricava dagli stem ed è accettata se
tutti gli stem ne hanno una sola; altrimenti errore (ING-3). Con un intero, tutti gli stem devono coincidere.
I riferimenti non contano: vengono ricampionati alla frequenza del progetto (ING-5).
Precisazioni dopo la revisione di chiusura di M0: `mix_true_peak_db` è limitato a [−24; −3] e rifiuta `inf`/`NaN`
(con `-.inf` il trim avrebbe prodotto silenzio); gli id degli stem hanno al massimo 32 caratteri.

### D-011 — Formati accettati: WAV PCM 16/24, float 32, FLAC 16/24 (2026-11-09, M0)

Chiude Q-12 (opzione a, scelta dall'autore). Ogni altro formato o sottotipo (PCM 32 intero, float 64, 8 bit)
e più di 2 canali danno un errore chiaro con il percorso del file.

### D-012 — Resampling dei riferimenti con `scipy.signal.resample_poly` (2026-11-09, M0)

Chiude Q-13 (opzione a, scelta delegata all'agente). Rapporto razionale esatto (es. 160/147 per 44,1→48 kHz),
filtro di default (FIR Kaiser β = 5,0). Motivo: i riferimenti servono solo all'analisi a 8 bande e al loudness
normalizzato, dove la differenza con un resampler di qualità superiore (soxr) è trascurabile; evita di
anticipare `librosa` in M0 e resta documentabile. In M0 c'è solo la funzione di caricamento: `mix
--baseline-only` controlla che i riferimenti esistano senza caricarli.

### D-013 — WAV anche con header WAVE_FORMAT_EXTENSIBLE e RF64 (2026-11-09, M0)

Integra D-011, su rilievo della revisione di chiusura di M0. libsndfile classifica come formato a sé (`WAVEX`) ogni
WAV con header WAVE_FORMAT_EXTENSIBLE, comune negli export a 24 bit e float (ffmpeg, sox, varie DAW); RF64 è
l'estensione EBU del WAV oltre i 4 GB, con la stessa estensione `.wav`. Entrambi sono "WAV" per ING-2 e accettano le
stesse codifiche (PCM 16, PCM 24, float 32). Restano esclusi gli altri contenitori, per esempio W64 (Sony Wave64,
estensione `.w64`). Da rivedere se l'autore preferisce escludere RF64.

### D-014 — Parti attive: frame della STFT sopra −50 dBFS (2026-11-10, M1)

Chiude Q-14 (opzione a, approvata dall'autore con il piano di M1). Griglia: frame della STFT di analisi (Hann,
≈170 ms, hop ¼). Livello del frame = media quadratica pesata Hann, media sui canali; attivo se > −50 dBFS. Su frame
attivi: RMS, crest, bande, centroide, stereo, `active_pct`. Sul segnale intero: picchi, true peak, DC, clipping. LUFS
integrato con il solo gating BS.1770. Onset contati sul segnale intero e divisi per i secondi attivi. Rumore di fondo
dai frame inattivi.

### D-015 — Escursione dinamica = LRA di EBU Tech 3342 (2026-11-10, M1)

Chiude Q-15 (opzione a). Short-term 3 s con passo 100 ms, gate assoluto −70 LUFS e relativo −20 LU, differenza tra
95° e 10° percentile. K-weighting dello short-term con le formule BS.1770, verificato contro `pyloudnorm`.
Precisazione al passo 1: `pyloudnorm` 0.2.0 implementa già l'LRA di Tech 3342 (`Meter.loudness_range`), quindi non
serve scrivere il K-weighting a parte; il passo dello short-term è 90 ms (sovrapposizione 97%) invece di 100 ms,
comunque conforme alla norma (almeno 10 valori al secondo). Il test di coerenza del K-weighting previsto nel piano
decade; restano i quattro casi di Tech 3342.

### D-016 — Mascheramento: attività in banda a −50 dBFS, punteggio min/max frame per frame (2026-11-10, M1)

Chiude Q-16 (opzioni i e A). Uno stem è attivo in una banda in un frame se il livello in banda supera −50 dBFS.
Punteggio `Σ_{t: entrambi attivi} min(E_a,E_b) / Σ_{t: almeno uno attivo} max(E_a,E_b)`, in [0, 1]. Nel JSON i 10
punteggi più alti maggiori di 0, a pari punteggio in ordine di progetto e di banda.

### D-017 — Quote per banda sulla somma degli stem; mix di analisi = `baseline.wav` (2026-11-10, M1)

Chiude Q-17 (opzione a). La quota di uno stem in una banda è la sua energia sul bus divisa per la somma delle energie
di tutti gli stem in quella banda (le quote sommano a 100%). Il blocco `mix` di `analysis.json` misura la somma grezza
con il trim, cioè il segnale di `baseline.wav`.

### D-018 — JSON: dB limitati a ±120, DC offset in percentuale (2026-11-10, M1)

Chiude Q-18 (opzioni a, a). Nel JSON i livelli in dB sono limitati a [−120, +120] e non compaiono mai `inf` o
`NaN`; le misure che richiedono parti attive valgono `null` su uno stem tutto inattivo. Il DC offset è in percentuale
del fondo scala, con un decimale.

### D-019 — `analyze` scrive in una nuova cartella di run (2026-11-10, M1)

Chiude Q-19 (opzione a). `stemwise analyze <progetto>` crea `runs/<data-ora>/analysis.json` con le stesse regole dei
run di M0 (nessuna sovrascrittura, cartella creata solo dopo la validazione).

### D-020 — Onset: soglia assoluta sull'inviluppo non normalizzato (2026-11-10, M1)

Emersa al passo 4. Il piano prevedeva `librosa.onset.onset_detect` con i parametri di default. Ma il default
normalizza l'inviluppo sul suo massimo e usa `delta = 0,07`: su segnali sostenuti, senza attacchi, il massimo è
piccolo e le fluttuazioni diventano onset. Misurato: seno stazionario 26 onset/s, rumore bianco 10/s, rumore rosa
11/s. Un pad o un basso tenuto risulterebbero "percussivi" agli occhi del modello. Scelta: `normalize=False` e
`delta = 2,0`, cioè un picco deve superare di 2 dB la media locale del flusso spettrale mel positivo (unità di
`librosa.onset.onset_strength`, indipendente dal livello). Restano i default per il resto (`hop 512`, `n_fft 2048`,
128 bande mel, `pre_max 30 ms`, `post_max 0`, `pre_avg`/`post_avg 100 ms`, `wait 30 ms`).
Misurato con la soglia nuova: seno, rumore bianco e rumore rosa 0–0,1 onset/s; click, burst di rumore smorzati
(anche a −34 dBFS) e note pizzicate 4/s contati esattamente. Si può rivedere sul materiale reale in validazione
manuale (`onsets_per_s` di voce e pad contro batteria).
Confermata dall'autore il 2026-11-10.

### D-021 — Due decimali per correlazioni e punteggi di mascheramento (2026-11-10, M1)

Chiude Q-20 (opzione b, scelta dall'autore). In `analysis.json` `correlation`, `low_corr` e `score` (grandezze
adimensionali in [−1, 1]) sono arrotondati a due decimali, come nell'esempio di PRD §6.2; tutti gli altri numeri
reali a uno (ANA-13). Motivo: con un decimale i punteggi di mascheramento si appiattivano (collisioni diverse tutte a
`0.5`) e un `low_corr` di 0,04 sarebbe comparso come `0.0`, cioè come una violazione apparente di EVA-4. Costo:
circa 40 caratteri in più su 12 stem.

### D-022 — Potenza dei frame, attività e RMS limitati a 20 Hz–20 kHz (2026-11-10, M1)

Chiude Q-22 (opzione a, scelta dall'autore). Modifica D-014. Rilievo 2 della revisione di chiusura: la potenza del
frame includeva continua e sub-20 Hz, quindi un DC offset sopra lo 0,32% (o il rombo) rendeva attivo ogni frame
(`active_pct` 100 invece di 30, RMS −18,7 invece di −13,5 dBFS). Ora la potenza del frame, usata per la soglia di −50
dBFS, per l'RMS e per il rumore di fondo, somma solo i bin 20 Hz–20 kHz, gli stessi delle bande. Picchi, true peak,
DC offset e clipping restano sul segnale intero.

### D-023 — Onset contati solo nei frame attivi, inviluppo per canale (2026-11-10, M1)

Chiude Q-21 (opzione a, scelta dall'autore). Modifica D-014 e integra D-020. Rilievo 1 della revisione: i transitori
nelle pause sotto −50 dBFS (rientri della click) venivano contati e divisi per i soli secondi attivi (voce
sostenuta + rientri ⇒ 11,95 onset/s). Ora un onset conta solo se cade in un frame attivo (frame con il centro più
vicino all'onset). Inoltre l'inviluppo è la **media** degli inviluppi dei canali, non quello della media dei canali,
così l'antifase (L = −R) non cancella gli onset; la media, invece della somma proposta in Q-21, lascia invariata la
scala della soglia di 2 dB per gli stem dual mono.

### D-024 — Riferimenti identificati da etichette neutre in `analysis.json` (2026-11-10, M1)

Chiude Q-23 (opzione a, scelta dall'autore). Rilievo 3 della revisione: il campo `file` portava nel JSON destinato
al modello (DEC-1) il percorso scritto in `project.yaml`, anche assoluto (nome utente, artista, titolo), contro la
privacy di PRD §7. Ora ogni riferimento ha solo `id` = `ref1`, `ref2`, `ref3`, nell'ordine di `project.yaml`; il
percorso resta nei soli messaggi d'errore locali.

### D-025 — Stem grezzi da registrazione: `pre_normalize: true` resta il default (2026-11-10, M1)

Chiude Q-1. L'autore conferma che gli stem escono dalla DAW con i livelli grezzi di registrazione, senza il
bilanciamento dei fader (in validazione di M1: 26 dB tra lo stem più basso e il più alto, voce molto sotto le
chitarre). Il default `pre_normalize: true` di PRD §6.1 è quindi quello giusto per REN-1 in M2.

### D-026 — Piano scritto a mano: `mix --plan <file>` (2026-11-10, M2)

Chiude Q-5 (opzione a, approvata con il piano di M2). `stemwise mix <progetto> --plan <file.json> [--export-stems]`
salta la decisione e scrive un nuovo run con `baseline.wav` e `iter_00/` (`plan.json` copiato, `mix.wav`, stem
esportati). È un'opzione in più rispetto a PRD §8, utile anche dopo M3 per correggere a mano un piano.
Precisazione dopo la revisione di chiusura di M2: il `plan.json` scritto in `iter_00/` ha `iteration: 0`, il numero
della cartella, qualunque valore porti il file d'ingresso.

### D-027 — Modello del piano anticipato in M2 (2026-11-10, M2)

Chiude Q-6 (opzione a). `stemwise.decision.schema.MixPlan` con i limiti di PRD §6.3 nasce in M2 ed è esportato in
`schemas/mix_plan.schema.json`. Restano a M3 prompt, decisori, nuovi tentativi (DEC-4) e limiti di variazione (DEC-5).

### D-028 — Quote e mascheramento sugli stem pre-normalizzati (2026-11-10, M2)

Chiude Q-24 (opzione a). Con `pre_normalize: true` quote per banda (ANA-8) e mascheramento (ANA-9) si calcolano
sugli stem portati a −23 LUFS dal pre-gain di REN-1, cioè sul punto di partenza del piano. Il blocco `mix` resta la
somma grezza (D-017); le misure per stem restano sui livelli grezzi.

### D-029 — Filtri ed EQ: biquad RBJ in `scipy`, 12 dB/ottava (2026-11-10, M2)

Chiude Q-25 (opzione a). Passa-alto, passa-basso, EQ e passa-alto sul ritorno del riverbero sono biquad del cookbook
RBJ (forma con Q) applicati con `scipy.signal.sosfilt` in float64; passa-alto e passa-basso Butterworth del secondo
ordine (Q = 1/√2). `pedalboard` serve solo per compressore e riverbero. Motivo: i filtri del primo ordine di
`pedalboard` (6 dB/ottava, senza Q) non sono i biquad di PRD §7.1.

### D-030 — Bilanciamento lineare degli stem stereo (2026-11-10, M2)

Chiude Q-26 (opzione a). Stem stereo: `L·min(1, 1−p)`, `R·min(1, 1+p)`. Al centro lo stem resta identico.

### D-031 — Il mix dura quanto gli stem (2026-11-10, M2)

Chiude Q-27 (opzione a). La coda del riverbero oltre la fine degli stem viene tagliata.

### D-032 — `--export-stems`: stem post-pan con trim, più il ritorno del riverbero (2026-11-10, M2)

Chiude Q-28 (opzione a). In `iter_NN/stems/` un file per stem (stereo, dopo la catena fino al pan, con il trim del
bus) e `reverb_return.wav`: la loro somma è `mix.wav`, a meno dell'arrotondamento a 24 bit.
Precisazione dopo la revisione di chiusura di M2 (rilievo 1): il ritorno del riverbero si scrive in
`iter_NN/reverb_return.wav`, fuori da `stems/`, perché `reverb_return` è un id di stem valido (D-010) e i due file si
sarebbero sovrascritti. Se uno stem esportato supera 0 dBFS (possibile con stem in controfase che si cancellano nel
mix) il file a 24 bit lo taglia e la somma non torna più: in quel caso un warning lo dice (rilievo 3).

### D-033 — Casi limite del render (2026-11-10, M2)

Chiude Q-29 (opzione a). Stem muto o più corto di 400 ms ⇒ pre-gain 0 dB con warning; nessun tetto al pre-gain, warning
sopra +24 dB; il piano deve contenere esattamente gli id del progetto; frequenza del piano ≥ `fs/2` ⇒ errore.
Aggiunta dopo la revisione di chiusura di M2 (rilievo 4): stem più corti di 3 campioni ⇒ errore chiaro (i plugin di
`pedalboard` ricavano i canali dalla dimensione minore dell'array).

### D-034 — Compressore di `pedalboard` per canale, accettato (2026-11-10, M2)

Verificato al passo 4 del piano: `pedalboard.Compressor` (JUCE) ha un detector di picco per canale, quindi su uno stem
stereo i due canali vengono compressi in modo indipendente (sinistro a −10 dBFS, destro 40 dB sotto, soglia −30,
ratio 4: sinistro ridotto di 14,2 dB, destro invariato). Come proposto nel piano approvato, si accetta per la PoC e si
annota: l'immagine stereo degli stem con canali molto sbilanciati può spostarsi. Un compressore collegato richiederebbe
di uscire da `pedalboard`.

### D-035 — Validazione stretta del piano, senza coercizione (2026-11-10, M2)

Rilievo 2 della revisione di chiusura di M2. Il modello del piano accettava `"gain_db": "2"`, `"pan": true` o
`"iteration": "1"` convertendoli, mentre `schemas/mix_plan.schema.json` li rifiuta. Ora il modello è `strict`: accetta
esattamente ciò che accetta lo schema esportato (un intero resta un numero valido). Lo schema non cambia.

### D-036 — Client HTTP: `httpx` (2026-11-10, M3)

Chiude Q-30 (opzione a, rivista su richiesta dell'autore). `httpx` 0.28.1 per la chiamata a OpenRouter:
`httpx.MockTransport` simula OpenRouter nei test senza rete, timeout separati per connessione e lettura, eccezioni
distinte per timeout, connessione e stato HTTP, corpo delle risposte d'errore leggibile per i log di DEC-8. Scartati
`urllib` (stesso risultato con più codice a mano) e l'SDK `openai` (nuovi tentativi automatici, oggetti al posto del
corpo grezzo, campi di OpenRouter da `extra_body`).

### D-037 — Configurazione del decisore da ambiente e `.env` (2026-11-10, M3)

Chiude Q-31 (opzione a). `STEMWISE_MODEL` (obbligatoria con `--decider llm`, nessun default nel codice),
`STEMWISE_TEMPERATURE` (default 0,2), `STEMWISE_BASE_URL` (default `https://openrouter.ai/api/v1`),
`STEMWISE_REASONING_EFFORT` (facoltativa, `low`/`medium`/`high`/`xhigh`, mandata come `reasoning.effort`), più
`OPENROUTER_API_KEY`. Lette dall'ambiente e da `.env` nella cartella corrente (righe `CHIAVE=valore`; l'ambiente
vince); `--model`, `--temperature` e `--reasoning-effort` sopra tutto. `project.yaml` invariato.

### D-038 — `mix` in M3: una decisione e un render (2026-11-10, M3)

Chiude Q-32 (opzione a). `mix` scrive `iter_00/` con `plan.json`, `mix.wav`, `llm_request.json` e (col modello
reale) `llm_response.json`, senza `evaluation.json`. Storia delle ultime 2 iterazioni (DEC-1) e limiti di variazione
(DEC-5) sono funzioni pure testate su dati sintetici, collegate al ciclo in M4; `--max-iter` arriva in M4.

### D-039 — Output strutturato con schema per progetto (2026-11-10, M3)

Chiude Q-33 (opzione a). Schema derivato dal modello Pydantic con `stems` come oggetto che ha per proprietà
esattamente gli id del progetto (tutti obbligatori, `additionalProperties: false`), `response_format: json_schema`,
`strict: true`, `provider.require_parameters: true`. Dallo schema inviato si tolgono solo le parole chiave fuori dal
sottoinsieme `strict`; la validazione locale (D-035, `check_plan_fits`) resta l'unica autorità. Lo schema inviato è
salvato in `llm_request.json`, non in `schemas/`. Il motore impone `iteration`. Un solo blocco ```` ```json ````
viene accettato togliendo i backtick. La scelta tra `json_schema` e `json_object` segue D-045 (niente
`STEMWISE_OUTPUT_MODE`).

### D-040 — Nuovi tentativi e log delle chiamate (2026-11-10, M3)

Chiude Q-34 (opzione a). Il nuovo tentativo continua la conversazione (risposta invalida come `assistant`, errori
come `user`). Contano come piano invalido: testo non JSON, risposta vuota o troncata, schema violato, piano che non si
adatta al progetto; errori di rete e HTTP non si ritentano. `llm_request.json` e `llm_response.json` contengono
`{"attempts": [...]}` allineati per indice (corpo inviato senza intestazioni né chiave; corpo ricevuto, stato,
durata, token, costo `usage.cost` o `null`, esito della validazione). Se la decisione fallisce la cartella del run
resta, con un messaggio che la indica. Col decisore finto si scrive `llm_request.json` con i messaggi che sarebbero
inviati.
Precisazione in sviluppo (passo 7): la cartella resta solo se il modello è stato chiamato almeno una volta; un errore
prima della prima chiamata (per esempio modello non disponibile su OpenRouter) non costa nulla e la cartella viene
rimossa come in M0–M2. Valori vuoti in `.env` (come in `.env.example` copiato) valgono come non impostati.

### D-041 — Contenuto del prompt (2026-11-10, M3)

Chiude Q-35 (opzione a). Prompt di sistema in inglese con ruolo, catena, unità, limiti generati dal modello e le
avvertenze di M1–M2 (pre-gain a −23 LUFS, strumenti divisi in più stem, collisioni tra microfoni della stessa
batteria, punteggi sotto 0,05 trascurabili, livello del ritorno del riverbero, trim a −3 dBTP); `rationale` e
`summary` nella lingua del brief. Messaggio utente: brief, stem con id, ruolo e `notes`, `analysis.json`, storia,
richiesta. Il titolo del brano non viene inviato; i riferimenti restano `ref1…` (D-024).

### D-042 — Limiti di variazione: bande di EQ per posizione (2026-11-10, M3)

Chiude Q-36 (opzione a). DEC-5 limita `gain_db` dello stem a ±4 dB e `gain_db` di ogni banda di EQ a ±3 dB rispetto
all'iterazione precedente. Bande accoppiate per posizione; tipo diverso o banda nuova ⇒ guadagno precedente 0 dB;
frequenza e Q liberi; togliere una banda è ammesso; nessun limite alla prima iterazione. Ogni correzione dà un warning
e una voce nel log.

### D-043 — Regole del decisore finto (2026-11-10, M3)

Chiude Q-37 (opzione a). Funzione pura di ruoli e analisi: gain per ruolo, famiglia batteria con n > 1 stem a
−10·log₁₀ n dB, mono dello stesso ruolo alternati ±0,5, passa-alto per ruolo, un `peak` −2 dB sull'altro stem delle
collisioni più forti con voce, basso o cassa (al massimo 2 bande per stem), compressore fisso su voce e basso,
riverbero fisso con mandate per ruolo. Testi in inglese; la richiesta dell'autore è ignorata fino a M5. I valori
esatti sono documentati nel modulo `decision/mock.py`.

### D-044 — Default di `--decider`: `llm` (2026-11-10, M3)

Chiude Q-38 (opzione a). Chiave e modello vengono controllati prima di qualunque lavoro.

### D-045 — Parametri della richiesta secondo il modello (2026-11-10, M3)

Chiude Q-39 (opzione a). Prima della decisione il decisore legge i `supported_parameters` del modello (`GET /models`
di OpenRouter). `temperature` e `seed: 0` si mandano solo se supportati, altrimenti warning e nota nel log;
`json_schema` se il modello supporta `structured_outputs`, altrimenti `json_object` + validazione (DEC-2);
`reasoning.effort` se configurato.

### D-046 — Rilievi della revisione di chiusura di M3 (2026-11-10, M3)

Integra D-039, D-040, D-043 e D-044, su rilievi della revisione indipendente.
- Dopo una risposta vuota il nuovo tentativo rimanda il messaggio `assistant` con il testo `(empty answer)`, perché
  alcuni provider rifiutano i messaggi vuoti (rilievo 1).
- Il modello viene cercato su `GET /models` prima dell'analisi (`preflight`): un id sbagliato fallisce subito, senza
  cartella di run (rilievo 7, D-044).
- Se la decisione si interrompe dopo chiamate già fatte per una causa diversa da `DecisionError` (Ctrl-C, errore
  inatteso), la cartella resta e i log delle chiamate vengono scritti; se il render fallisce dopo una decisione col
  modello, il messaggio indica la cartella conservata (rilievo 8, D-040).
- Ogni piano, di qualunque decisore, ripassa da `MixPlan` e `check_plan_fits` prima del render (rilievo 5).
- I valori riportati ai limiti di DEC-5 sono arrotondati a 6 decimali (rilievo 6); un blocco ```` ```json ```` con
  fine riga CRLF è accettato (rilievo 10).
- `--decider`, `--model`, `--temperature` e `--reasoning-effort` con `--plan` o `--baseline-only` danno errore invece
  di essere ignorati (rilievo 9).

### D-047 — Modello di default: GPT-6.1 Sol, sforzo medio; H5 da rivedere coi costi reali (2026-11-10, M3)

Chiude Q-2 (opzione c, scelta dall'autore). Confronto in validazione manuale di M3 sul brano "Confidence" (13 stem),
una chiamata per modello, tutte valide al primo tentativo:

| Modello (sforzo) | Token in + out | Costo | Durata | Ascolto dell'autore |
| --- | --- | --- | --- | --- |
| `openai/gpt-6.1-terra` (high) | 6.727 + 3.545 | 0,026 USD | 47 s | decente |
| `openai/gpt-6.1-sol` (medium) | 6.727 + 3.451 | 0,051 USD | 47 s | decente, leggermente preferito |
| `anthropic/claude-haiku-5.5` (xhigh) | 10.002 + 22.768 | 0,012 USD | 101 s | scartato |

Haiku non compensa la batteria divisa in 7 stem (gruppo stimato circa 5 dB sopra la voce, contro circa 1 dB di Terra
e la parità di Sol). Default da impostare in `.env` e `.env.example`: `STEMWISE_MODEL=openai/gpt-6.1-sol`,
`STEMWISE_REASONING_EFFORT=medium` (nessun default nel codice, DEC-6; `.env.example` aggiornato dall'autore, perché
all'agente è negato l'accesso al file). **H5 resta da verificare**: con circa 5 chiamate per brano (`mix` fino a 3 + 2
`revise` del protocollo §10.2) Sol stima ~0,26 USD, sopra la soglia di 0,20; Terra ~0,13. La decisione su H5 (soglia,
conteggio delle revisioni o cambio di default) si prende in M4/M5 con i costi misurati di un brano intero.

### D-048 — Parametri per sezione (automazioni): must-have del prodotto (2026-11-10, dopo M3)

Indicazione dell'autore dopo la validazione di M3 (chitarre distorte del chorus poco udibili con parametri statici).
Nella PoC restano fuori scope (PRD §3, colonna destra; §11, "rimandato a dopo un eventuale go"). Se l'esito è go o go
con riserva, il prodotto **deve** includere parametri diversi per sezione del brano (automazioni). Con quelli diventa
utile anche dare al modello misure nel tempo compatte, per esempio la loudness di ogni stem per sezione (non la forma
d'onda, che è audio: PRD §1). Da riportare nell'esito della PoC nel README (M7).

### D-049 — Contratto di `evaluation.json` e file del ciclo (2026-11-10, M4)

Chiude Q-41 (opzione a, approvata con il piano di M4). `evaluation.json` ha i campi di PRD §6.4 più un blocco `mix`
con le misure di ANA-7 e ANA-10 del mix renderizzato, nello stesso formato del blocco `mix` di `analysis.json` (servono
a REP-1). La somma grezza è valutata in `baseline_evaluation.json` alla radice del run, stesso schema, `iteration:
null`. `is_best` dice se l'iterazione è la migliore nel momento in cui viene valutata: ogni `evaluation.json` si scrive
una sola volta. L'autorità finale è `best.json` (`iteration`, `spectral_distance_db`, `stop_reason`, percorso del
piano), aggiornato dopo ogni iterazione. Schemi in `schemas/evaluation.schema.json` e `schemas/best.schema.json`.

### D-050 — Formule e segni della valutazione (2026-11-10, M4)

Chiude Q-42 (opzione a). `offset = media(target) − media(mix)` (media aritmetica in dB delle 8 bande),
`band_delta_i = target_i − (mix_i + offset)`, distanza = media di `|band_delta_i|`. `width_delta_db =
target.side_mid_db − mix.side_mid_db`, `dyn_delta_lu = target.dyn_range_lu − mix.dyn_range_lu`: positivo = al mix
manca. `null` se manca uno dei due valori. Calcolo a piena precisione (livelli limitati a ±120 dB), arrotondamento a
un decimale solo nel file.

### D-051 — Criteri di stop e piano migliore (2026-11-10, M4)

Chiude Q-43 (opzione a). Miglioramento = distanza dell'iterazione precedente − distanza della corrente; l'iterazione
0 si confronta solo con la soglia di 1,5 dB; un'iterazione che viola i vincoli non ferma il ciclo per la soglia e non
può essere la migliore. Ordine dei motivi: `below_threshold`, `no_improvement`, `max_iterations`. Le decisioni usano
i valori scritti in `evaluation.json` (un decimale), confrontati in decimi interi; a parità vince l'iterazione più
vecchia. DEC-5 resta rispetto all'iterazione precedente. `--max-iter N` (1–3) prevale su `options.max_iterations`.

### D-052 — Ciclo senza riferimenti (2026-11-10, M4)

Chiude Q-44 (opzione a). Senza target distanza e delta sono `null`; il ciclo si ferma alla prima iterazione che
rispetta i vincoli (`stop_reason: no_target`), altrimenti prosegue fino al massimo; migliore = la prima valida.

### D-053 — Vincoli tecnici (2026-11-10, M4)

Chiude Q-45 (opzione a). `true_peak_ok`: true peak del mix trimmato in memoria ≤ `mix_true_peak_db`. `low_corr_ok`:
`low_corr > 0`; `null` (nessun frame attivo sotto 120 Hz) conta come ok. `dc_ok`: media di ogni canale ≤ 0,1% del
fondo scala (−60 dBFS), la misura di ANA-6.

### D-054 — Collisioni residue (2026-11-10, M4)

Chiude Q-46 (opzione a). Formula di ANA-9 (`masking v2`), top 10, sugli stem processati come arrivano al bus (dopo il
pan, prima del trim, senza il ritorno del riverbero); per la somma grezza sugli stem originali di `baseline.wav`,
riusando gli spettri dell'analisi. Gli spettri degli stem processati si calcolano uno alla volta durante il render.

### D-055 — Errori a metà ciclo (2026-11-10, M4)

Chiude Q-47 (opzione a). Errore all'iterazione 0 ⇒ come M3 (D-040, D-046). A k ≥ 1 ⇒ stop con `stop_reason:
decision_failed`, iterazioni fatte conservate, `best.json` e `report.md` scritti, warning, uscita 0. Nessuna iterazione
valida ⇒ `best.json` con `iteration: null`, report che lo dice, uscita 1. Interruzione ⇒ cartella conservata se il
modello è stato chiamato; `best.json` aggiornato dopo ogni iterazione.

### D-056 — `report.md` e comando `report` (2026-11-10, M4)

Chiude Q-48 (opzione a). Testo fisso in inglese; sommari e motivazioni come li ha scritti il decisore. Contenuto:
decisore e modello, brief, tabella con la somma grezza e ogni iterazione, motivo di stop, sommari, correzioni di DEC-5,
parametri principali e motivazione per stem del piano migliore, 5 collisioni principali della somma grezza e residue
del migliore, costo e token totali. Lo scrivono i run di `mix` (decisore e `--plan`). `report` lo rigenera dai soli
file del run; "ultimo run" = il più recente con `best.json`; `--run <cartella>` per un altro.

### D-057 — Retroazione del decisore finto (2026-11-10, M4)

Chiude Q-49 (opzione a). Integra D-043. Dall'iterazione 1 il finto parte dal piano precedente e, nella banda con
`|band_delta_db|` maggiore (se ≥ 1 dB), corregge lo stem con la quota maggiore in quella banda con un `peak` (Q 1, al
centro geometrico della banda) di guadagno `band_delta_db` limitato a ±3 dB: regola il `peak` già presente a quella
frequenza oppure ne aggiunge uno se lo stem ha meno di 4 bande. Senza target nessuna correzione.

### D-058 — Rilievi della revisione di chiusura di M4 (2026-11-10, M4)

Integra D-050, D-054 e D-056, su rilievi della revisione indipendente.
- Collisioni della somma grezza (rilievo 2): gli stem mono si misurano centrati su due canali a `√½`, come entrano
  nel bus di `baseline.wav` e come uno stem mono renderizzato con pan al centro. Prima la soglia di attività (media
  sui canali) era 3 dB più severa nel mix che nella somma grezza, e un piano neutro "riduceva" le collisioni senza
  cambiare nulla. Costo: una STFT in più per ogni stem mono, una volta per run.
- Scarti limitati a ±120 dB prima di entrare nel contratto (rilievo 3): con bande estreme la formula poteva superare
  i limiti dello schema e interrompere il ciclo.
- `report` (rilievo 1): un file del run malformato dà un errore chiaro invece di un traceback. Le collisioni che
  si arrotondano a 0,00 non compaiono nel report (rilievo 6). `--run` cerca prima un nome in `runs/` del brano, poi
  un percorso (rilievo 8).

### D-059 — `revise` scrive un nuovo run (2026-11-10, M5)

Chiude Q-50 (opzione a, approvata con il piano di M5). Una revisione crea `runs/<data-ora>/` con `revision.json`
(run e iterazione di partenza, percorso del piano, richiesta), `source_plan.json` e `source_evaluation.json` (copie del
piano di partenza e della sua valutazione), `analysis.json`, `baseline.wav`, `baseline_evaluation.json` (copiati dal
run di partenza), `iter_00/` (piano, `mix.wav`, valutazione, log del decisore), `best.json`, `report.md`. Il run di
partenza non viene toccato. La revisione diventa "ultimo run" (D-056): una `revise` dopo l'altra parte dal piano
rivisto. `revision.json` ha lo schema `schemas/revision.schema.json`; `stop_reason` guadagna il valore `revision`.

### D-060 — Piano di partenza e migliore di una revisione (2026-11-10, M5)

Chiude Q-51 (opzione a). `revise` parte dal piano indicato da `best.json` del run scelto (ultimo con `best.json`
oppure `--run`): dopo un `mix`, il piano migliore di quel mix. `best.json` con `iteration: null` ⇒ errore chiaro che
suggerisce `--run`. Nel run di revisione `best.json` indica `iter_00` qualunque sia la distanza (niente rollback,
REV-2), purché rispetti i vincoli tecnici (PRD §2); se ne viola uno, `best.json` ha `iteration: null`, i file restano
per l'ascolto e l'uscita è 1. La variante "salta da sola le revisioni invalide" è scartata perché è un rollback
automatico.

### D-061 — Revisione: analisi riusata, riferimenti rimisurati (2026-11-10, M5)

Chiude Q-52 (opzione a). `analysis.json`, `baseline_evaluation.json` e `baseline.wav` si copiano dal run di
partenza; si rimisurano solo i riferimenti, così il target della valutazione è a piena precisione come nel run di
partenza. Prima di tutto un controllo di coerenza: id, ruoli, canali, durata e frequenza degli stem del progetto uguali
a quelli di `analysis.json`, target rimisurato uguale (arrotondato) a quello salvato; altrimenti errore che indica il
run e suggerisce `mix`.
Precisazione in sviluppo (passo 3): il controllo confronta anche il picco di campione di ogni stem (`peak_db`, un
decimale), che costa una lettura e intercetta uno stem sostituito con la stessa durata. Le `options` di
`project.yaml` (`pre_normalize`, `mix_true_peak_db`) non sono salvate nel run di partenza e quindi non si possono
confrontare: se cambiano tra `mix` e `revise`, la revisione usa le nuove senza avviso.

### D-062 — Input del modello in revisione (2026-11-10, M5)

Chiude Q-53 (opzione a). Storia = solo il piano di partenza con la sua valutazione; richiesta nella sezione "Author's
request (top priority)"; sezione "Revision" nel prompt di sistema, presente solo con una richiesta. Le richieste delle
revisioni precedenti non si inviano. Il piano rivisto ha `iteration: 0` (la cartella, come D-026).

### D-063 — DEC-5 vale anche in revisione (2026-11-10, M5)

Chiude Q-54 (opzione a). Gain ±4 dB e bande di EQ ±3 dB rispetto al piano di partenza (D-042), con warning e voce nel
report.

### D-064 — Diff dei piani (2026-11-10, M5)

Chiude Q-55 (opzione a). Funzione pura `plan_diff(prima, dopo)` su ogni campo del piano; bande di EQ per posizione
come D-042; uguaglianza esatta dei valori scritti. Il diff sta solo nel report (rigenerabile da `source_plan.json` e
`iter_00/plan.json`), con le sole righe cambiate, gli stem invariati e le motivazioni nuove degli stem cambiati.

### D-065 — Il decisore finto legge la richiesta (2026-11-10, M5)

Chiude Q-56 (opzione a). Integra D-043. Parole chiave italiane e inglesi: soggetto (id di stem o ruolo per sinonimo,
più riverbero), direzione (su ⇒ +2 dB di gain, giù ⇒ −2 dB; con il riverbero ±3 dB sulle mandate). Richiesta non
riconosciuta ⇒ piano di partenza invariato con un `summary` che lo dice. Le regole esatte sono nel modulo
`decision/mock.py`.

### D-066 — Opzioni di `revise` e report di revisione (2026-11-10, M5)

Chiude Q-57 (opzione a). `stemwise revise <brano> "<richiesta>" [--run] [--decider] [--model] [--temperature]
[--reasoning-effort] [--export-stems]`, senza `--max-iter`; `--run` come per `report`; richiesta vuota o oltre 1.000
caratteri ⇒ errore. `report.md` di revisione in inglese: richiesta, run di partenza, decisore e costo, tabella somma
grezza · partenza · rivisto, sommario, diff, warning, collisioni prima e dopo. `stemwise report` lo riconosce da
`revision.json` e lo rigenera identico.

### D-067 — H5: scostamento contenuto accettabile (2026-11-10, M5) — da confermare col costo misurato

Indicazione dell'autore all'avvio di M5: se il costo per brano supera di poco la soglia di 0,20 USD (stima con le 2
`revise` del protocollo §10.2: ~0,23–0,30), lo scostamento è accettabile. Si conferma alla chiusura di M5 con il costo
misurato di due `revise` sul brano reale; se il costo esce molto più alto se ne riparla.

### D-068 — H5 accettata con scostamento contenuto (2026-11-10, M5)

Conferma D-067 con il costo misurato in validazione manuale di M5 su "Confidence" (Sol, sforzo medio): `mix` a 2
iterazioni 0,1116 USD, due `revise` 0,0589 e 0,0532 USD, cioè ~0,22 USD per brano col protocollo §10.2 (un `mix` e
due `revise`). La soglia di 0,20 USD di PRD §2 è superata di circa il 10%: l'autore lo giudica accettabile. In M7
H5 si valuta con questa tolleranza; se un brano con 3 iterazioni di `mix` superasse di molto ~0,25 USD, se ne
riparla.

### D-069 — Dipendenza `matchering==2.0.6` (2026-11-10, M6)

Chiude Q-58 (opzione a, approvata con il piano di M6). `uv add matchering==2.0.6` (versione bloccata, ultima
pubblicata, ottobre 2022), autorizzato dall'autore. Porta `resampy` e `statsmodels`, e con questi `pandas`, `patsy`,
`formulaic`, `interface-meta`, `wrapt`, `python-dateutil` e `tzdata` (completato dopo la revisione di chiusura). Fatti
verificati sul sorgente: elaborazione deterministica (il solo `random` nomina i file temporanei di ffmpeg), frequenza
interna 44.100 Hz di default, limiter finale a picco di campione con soglia `(2^15 − 61)/2^15` ≈ −0,016 dBFS, mono
duplicato in stereo, durata tra `fft_size` campioni e 15 minuti, errore se mix e riferimento coincidono.

### D-070 — Master alla frequenza del progetto (2026-11-10, M6)

Chiude Q-59 (opzione a). `matchering.Config(internal_sample_rate=fs del progetto)`: il mix non si ricampiona, il
master esce alla frequenza del progetto, il riferimento si ricampiona come in ING-5 (D-012). La libreria avvisa che
valori diversi da 44.100 non sono "tested properly"; il sorgente scala FFT, finestre e limiter con la frequenza.

### D-071 — `matchering` chiamato in memoria (2026-11-10, M6)

Chiude Q-60 (opzione a). Lettura (formati di D-011), ricampionamento e scrittura con `audio_io`; `matchering` riceve
array (`matchering.stages.main`). I controlli di `matchering.checker` che servono (durata, mono → stereo, mix uguale al
riferimento) si rifanno in `master.py`. Un test confronta l'uscita con quella di `mg.process` a 44,1 kHz.

### D-072 — Dove scrive `master` (2026-11-10, M6)

Chiude Q-61 (opzione a). Dentro il run scelto: `master/` se non esiste, altrimenti `master_02/`, `master_03/`…; nessun
file esistente cambia. Con `--mix` un run nuovo `runs/<data-ora>/master/` senza `best.json`, che quindi non diventa
"ultimo run" (D-056).

### D-073 — Ingressi e opzioni di `master` (2026-11-10, M6)

Chiude Q-62 (opzione a). `stemwise master <brano> [--run] [--mix <file>] [--reference <id|file>] [--pcm16]`. Mix =
`iter_NN/mix.wav` indicato da `best.json` dell'ultimo run (o di `--run`); `best.json` con `iteration: null` ⇒ errore
che suggerisce `--run` o `--mix`; `--run` con `--mix` ⇒ errore. Riferimento = `ref1`; `--reference` accetta
`ref1…ref3` o un percorso; nessun riferimento ⇒ errore (MAS-1). Mix a frequenza diversa dal progetto ⇒ errore (come
ING-3); riferimento ricampionato (ING-5).

### D-074 — True peak del master: solo warning (2026-11-10, M6)

Chiude Q-63 (opzione a). Limiter di `matchering` con le impostazioni di default; true peak misurato e warning sopra
−1 dBTP (MAS-3), nessuna correzione. Atteso: con riferimenti commerciali forti il warning sarà frequente.

### D-075 — 16 bit in aggiunta (2026-11-10, M6)

Chiude Q-64 (opzione a). `master.wav` sempre a 24 bit; `--pcm16` scrive anche `master_16.wav` dallo stesso segnale,
senza dither. Le misure di MAS-3 sono sul file a 24 bit.

### D-076 — Contenuto di `master_report.json` (2026-11-10, M6)

Chiude Q-65 (opzione a). Modello `MasterReport` → `schemas/master_report.schema.json`: origine del mix, riferimento,
frequenza, file scritti; `lufs`, `true_peak_db`, `true_peak_ok` (≤ −1 dBTP); `spectral_distance_db` e `band_delta_db`
con la formula di EVA-2 tra le bande del master (ANA-7) e quelle del riferimento usato (ANA-11); per confronto la
distanza del mix prima del master e LUFS/true peak del riferimento; `warnings`. Nessun nome di libreria. Misure sul
file a 24 bit riletto. `report.md` invariato.

### D-077 — Master deciso dal modello: idea per il prodotto (2026-11-10, M6)

Domanda dell'autore all'avvio di M6: il modello potrebbe dare un contributo creativo anche nel master? Nella PoC no:
il master serve al confronto con un riferimento (PRD §3, MAS-2 "usa Matchering"), il master guidato a parole è fuori
scope (§3, §11), e un master deciso dal modello confonderebbe la lettura di H3 (l'ascolto cieco confronta i mix a
pari loudness). Con `matchering` la scelta creativa sta tutta nella scelta del riferimento. Se l'esito è go, per il
prodotto si valuta un master con parametri scelti e motivati dal modello: quanto imitare il riferimento o come
pesarne più d'uno, loudness e true peak secondo la destinazione, ampiezza stereo e compressione secondo genere e mood,
revisione a parole. Valore atteso minore che nel mix (pochi parametri, gran parte del misurabile già coperto
dall'imitazione del riferimento). Da riportare nell'esito della PoC nel README (M7), insieme a D-048.

### D-078 — Rilievi della revisione di chiusura di M6 (2026-11-10, M6)

Integra D-071, D-074 e D-076, su rilievi della revisione indipendente.
- Ingressi mono (rilievo 1): loudness, true peak e bande di mix e riferimento si misurano sulla versione stereo che
  entra nel master (mono duplicato), altrimenti un riferimento mono risultava 3 LU sotto un master di pari livello.
- True peak (rilievo 2): `true_peak_ok` e il warning confrontano il valore a piena precisione con −1 dBTP, come
  EVA-4 (D-053); il file riporta il valore arrotondato, quindi −0,96 dBTP si legge −1,0 con `true_peak_ok: false`.
- Import (rilievo 3): `matchering` si importa solo quando si fa un master; gli altri comandi non dipendono più dalla
  libreria né dal suo import di `scipy.ndimage.filters`, deprecato.
- Un ingresso più corto di 0,4 s (un blocco di loudness) dà "too short", non "silent" (rilievo 4).
- Tolleranze del test di avvicinamento strette da 1,0 dB / ±1 LU a 0,3 dB / ±0,3 LU, circa dieci volte i valori
  misurati (rilievo 6).

### D-079 — M7 ridotta: niente `ab` né protocollo di ascolto cieco (2026-11-10, M7)

Scelta dell'autore dopo il piano di M7 (opzione (a) proposta in chat). REP-2 (`ab`) e il protocollo di PRD §10.2 su 3
brani **non si eseguono**: l'autore considera sufficienti gli ascolti delle validazioni manuali di M3–M6 su
"Confidence" e non trova utile la procedura di ascolto cieco. M7 si chiude con l'esito della PoC nel README (criterio
di PRD §9), compilato con i dati già misurati. Il comando `ab` resta nella CLI e risponde "not implemented" (codice
d'uscita 3) con il rimando a questa decisione. Superate Q-3 e Q-66 → Q-72. Limiti da tenere presenti nel leggere
l'esito: un solo brano invece di tre, ascolto non cieco e non a pari loudness (in M4 la somma grezza era ~4 dB più
forte del mix AI, quindi lo sbilanciamento giocava **contro** il mix AI, che è stato comunque preferito), richieste a
parole 3 invece di 5.
L'autore ha scartato anche una milestone esplorativa sulle automazioni per sezione dentro la PoC: si passa al prodotto
(D-080).

### D-080 — Esito della PoC: go (2026-11-10, M7)

Decisione dell'autore: **go**, si passa allo sviluppo del prodotto. Ipotesi di PRD §2 con i dati di M3–M6 (dettaglio
nel README, "Esito della PoC"):

| Ipotesi | Soglia | Misurato | Esito |
| --- | --- | --- | --- |
| H1 piani validi | 100% entro 2 nuovi tentativi | 8 chiamate reali su "Confidence" (3 modelli) e i 3 test `llm` sintetici: tutte valide al **primo** tentativo | ✅ |
| H2 il ciclo converge | in calo su ≥ 2 brani su 3 | 1 brano: Sol 2,1 → 0,7 dB, Terra 1,7 → 1,2 dB (somma grezza 3,7) | ✅ su 1 brano su 1 |
| H3 meglio della somma grezza | preferito in ≥ 2 brani su 3, cieco a pari loudness | 1 brano, ascolto non cieco: mix migliore preferito a somma grezza e iter_00 (M4) | ✅ secondo l'autore, non cieco |
| H4 richieste a parole | ≥ 4 su 5 | 3 su 3 nella direzione chiesta (M5: test sintetico e due richieste reali) | ✅ su 3 |
| H5 costo | < 0,20 USD per brano | ~0,22 USD (`mix` 0,1116 + `revise` 0,0589 + 0,0532), accettato (D-068) | ✅ con tolleranza |

Note per il prodotto, da riportare nel README: parametri per sezione (D-048), master con parametri scelti dal modello
(D-077), e i limiti osservati nelle validazioni (collisioni dominate dai microfoni di batteria, compressore per canale
D-034, limiter del master sul picco di campione D-074, 3–7 minuti per un `mix` di 13 stem, avviso su `temperature`
ripetuto a ogni chiamata, Q-7 e Q-40 lasciate aperte).

