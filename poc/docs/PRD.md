# PRD — PoC engine di mixing e mastering assistiti da AI

Nov 9, 2026 · @Piero

## 1. Contesto e obiettivo

La PoC deve rispondere a una sola domanda: un modello linguistico che riceve solo misure numeriche degli stem sa decidere parametri di mix che suonano meglio della somma grezza?

L'utente è uno solo, l'autore dei brani. Scrive e registra bene, ma non è forte in mixing e mastering. Vuole uno strumento personale, senza abbonamenti né software costosi.

Tre principi guidano tutto il progetto:

- **Il modello non riceve mai audio.** Riceve JSON con le misure e restituisce JSON con i parametri. Tutto il processing audio gira in locale con librerie gratuite.
- **Il piano di mix è l'unica sorgente di verità.** Stesso piano e stessi stem producono lo stesso file audio.
- **Ogni scelta è spiegata.** Il modello motiva ogni intervento, così l'autore capisce e può correggere.

La PoC non ha interfaccia grafica: è un engine Python guidato da riga di comando. L'esito atteso è una decisione go/no-go sul costruire lo strumento completo.

Questo documento è scritto per essere eseguito da Claude Code, una milestone alla volta (sezione 9).

## 2. Ipotesi e criteri di successo

La PoC è riuscita se conferma le cinque ipotesi qui sotto, ciascuna con una soglia verificabile.

| Ipotesi | Come si misura | Soglia |
| --- | --- | --- |
| H1. Le misure bastano al modello per decidere | Piani validi rispetto allo schema, dopo al massimo 2 tentativi | 100% delle chiamate |
| H2. Il ciclo di verifica converge | Distanza spettrale dal riferimento tra prima e migliore iterazione | In calo su almeno 2 brani su 3 |
| H3. Il risultato suona meglio della somma grezza | Ascolto cieco a pari loudness (sezione 10) | Mix AI preferito in almeno 2 brani su 3 |
| H4. Le richieste a parole producono il cambio chiesto | Richieste di revisione giudicate "andata nella direzione giusta" | Almeno 4 su 5 |
| H5. Il costo è trascurabile | Costo delle chiamate al modello per brano, dai log | Sotto 0,20 USD per brano |

Due vincoli tecnici valgono sempre, a prescindere dalle ipotesi:

- Il mix esce con true peak non oltre −3 dBTP, senza limiter.
- Il render è deterministico: stesso piano e stessi stem danno un file con lo stesso hash.

Il confronto più onesto resta quello con un mix fatto a mano dall'autore, quando esiste. Non è una soglia di successo, ma va annotato.

## 3. Scope

La PoC copre il percorso minimo dagli stem al master, con parametri statici per l'intero brano.

| Dentro la PoC | Fuori dalla PoC |
| --- | --- |
| Engine Python usato da riga di comando | Interfaccia grafica, plugin, tempo reale |
| Stem WAV o FLAC già allineati, da 2 a 16 | Editing, intonazione, timing, pulizia del rumore |
| Ruolo di ogni stem dichiarato nel file di progetto | Riconoscimento automatico degli strumenti |
| Brief: genere, mood, note libere | Audio inviato al modello |
| Da 0 a 3 tracce di riferimento, analizzate come mix stereo | Separazione in stem delle tracce di riferimento |
| Per stem: gain, pan, filtri, EQ a 4 bande, compressore, mandata al riverbero | Automazioni nel tempo e differenze tra sezioni |
| Un riverbero condiviso | Delay, saturazione, modulazioni, sidechain |
| Ciclo automatico di verifica, fino a 3 iterazioni | Plugin VST di terze parti |
| Revisione con richiesta in linguaggio naturale | Export di un progetto DAW |
| Master per confronto con un riferimento | Master guidato da richieste a parole |
| Log completo e report per ogni run | Distribuzione, installer, multiutente |

Se durante lo sviluppo serve qualcosa della colonna di destra, va segnalato e non implementato.

## 4. Architettura

L'engine è un ciclo di quattro passi attorno a una sola chiamata al modello: analisi, decisione, render, valutazione.

&#91;embedded content: pipeline dell'engine · 6 passi, 1 ciclo\]

L'analisi misura stem, mix e riferimenti. Il modello riceve solo quei numeri e restituisce un piano. Il render lo applica, la valutazione misura il risultato e, se non basta, rimanda al modello. Il master parte dal mix del piano migliore.

## 5. Requisiti funzionali

Ogni modulo legge e scrive file JSON, ed è eseguibile da solo dalla riga di comando.

### 5.1 Ingest (ING)

- **ING-1.** Carica e valida `project.yaml` (sezione 6). Errori chiari per campi mancanti o file inesistenti.
- **ING-2.** Accetta stem WAV o FLAC, mono o stereo, a 16, 24 o 32 bit float.
- **ING-3.** Tutti gli stem devono avere la stessa frequenza di campionamento. Se differiscono, errore: nessun resampling silenzioso.
- **ING-4.** Se le durate differiscono, allunga con silenzio in coda e scrive un warning.
- **ING-5.** Le tracce di riferimento vengono ricampionate alla frequenza del progetto solo per l'analisi.
- **ING-6.** Elaborazione interna in float32. Export in WAV a 24 bit.

### 5.2 Analisi (ANA)

Per ogni stem, calcolate solo sulle parti attive (sopra −50 dBFS):

- **ANA-1.** Loudness integrato in LUFS (BS.1770), picco, true peak con oversampling 4x, RMS, crest factor.
- **ANA-2.** Escursione dinamica: differenza tra 95° e 10° percentile del loudness short-term.
- **ANA-3.** Energia in 8 bande fisse, in dB relativi al totale dello stem: 20–60, 60–120, 120–250, 250–500, 500–2k, 2k–4k, 4k–8k, 8k–20k Hz.
- **ANA-4.** Centroide spettrale, densità dei transienti (onset al secondo), percentuale di tempo attivo.
- **ANA-5.** Per stem stereo: correlazione L/R e rapporto side/mid.
- **ANA-6.** Controlli di igiene: DC offset, campioni in clipping, stima del rumore di fondo.

Sul mix e sulle interazioni tra stem:

- **ANA-7.** Le stesse misure di ANA-1, ANA-2, ANA-3 e ANA-5 sulla somma.
- **ANA-8.** Quota di ogni stem sull'energia del mix, per banda, in percentuale.
- **ANA-9.** Mascheramento: per ogni coppia di stem e ogni banda, percentuale di frame in cui entrambi sono attivi in quella banda, pesata per l'energia minore dei due. Nel JSON vanno solo le 10 collisioni più forti.
- **ANA-10.** Compatibilità mono delle basse: correlazione L/R sotto 120 Hz.

Sulle tracce di riferimento:

- **ANA-11.** Le misure di ANA-7, calcolate dopo aver normalizzato il loudness, così il confronto spettrale non dipende dal volume.
- **ANA-12.** Profilo target = media dei riferimenti. Senza riferimenti il profilo è assente e il valutatore usa solo i vincoli tecnici.

Vincolo trasversale:

- **ANA-13.** `analysis.json` per 12 stem resta sotto 8.000 token stimati (caratteri diviso 4). Valori arrotondati a un decimale.

### 5.3 Decisione (DEC)

- **DEC-1.** Il modello riceve: prompt di sistema con ruolo e limiti dei parametri, brief, analisi, profilo target, storia delle ultime 2 iterazioni con le loro valutazioni, eventuale richiesta dell'autore.
- **DEC-2.** Il modello restituisce un `mix_plan.json` completo, mai un diff. Usare l'output strutturato con JSON Schema se il modello lo supporta, altrimenti modalità JSON più validazione.
- **DEC-3.** Ogni stem ha un campo `rationale` di una o due frasi. Il piano ha un campo `summary`.
- **DEC-4.** Validazione con schema e limiti (sezione 6). Se il piano è invalido, fino a 2 nuovi tentativi rimandando l'errore al modello. Poi il comando fallisce con un messaggio chiaro.
- **DEC-5.** Limite di variazione tra un'iterazione e la successiva: gain ±4 dB, ogni banda di EQ ±3 dB. I valori fuori limite vengono riportati al limite, con warning.
- **DEC-6.** Modello, temperatura (default 0,2) e URL base sono configurabili. Nessun modello cablato nel codice.
- **DEC-7.** Esiste un decisore finto, deterministico e senza rete, con regole semplici. Serve ai test e allo sviluppo offline.
- **DEC-8.** Ogni chiamata viene salvata per intero: richiesta, risposta, token, costo se la risposta lo riporta, durata.

### 5.4 Render (REN)

- **REN-1.** Pre-gain: ogni stem viene normalizzato a −23 LUFS prima del piano, così i gain del piano sono confrontabili. Disattivabile da `project.yaml`.
- **REN-2.** Catena per stem, in ordine fisso: pre-gain, passa-alto, passa-basso, EQ, compressore, makeup, gain, pan, mandata al riverbero.
- **REN-3.** Pan a potenza costante (−3 dB al centro) per gli stem mono, bilanciamento per gli stereo. Implementato in numpy.
- **REN-4.** Un solo riverbero condiviso, 100% wet, con parametri globali e passa-alto sul ritorno.
- **REN-5.** Bus di mix: somma e trim automatico per restare entro −3 dBTP. Nessun altro processing.
- **REN-6.** Scrive `mix.wav`. Con `--export-stems` scrive anche gli stem processati.
- **REN-7.** Scrive sempre anche la somma grezza (`baseline.wav`): stem originali sommati, con il solo trim.

### 5.5 Valutazione e ciclo (EVA)

- **EVA-1.** Rianalizza il mix renderizzato con le misure di ANA-7, ANA-9 e ANA-10. Fa lo stesso sulla somma grezza, come termine di confronto.
- **EVA-2.** Distanza spettrale: errore medio assoluto in dB sulle 8 bande, dopo aver allineato il livello medio a quello del target.
- **EVA-3.** Riporta anche differenza di ampiezza stereo, differenza di escursione dinamica e collisioni residue.
- **EVA-4.** Vincoli tecnici: true peak, correlazione sotto 120 Hz maggiore di 0, DC offset trascurabile.
- **EVA-5.** Il ciclo si ferma quando la distanza scende sotto 1,5 dB, oppure migliora meno di 0,2 dB, oppure si raggiunge il massimo di iterazioni (default 3).
- **EVA-6.** Tiene traccia del piano migliore: distanza minima tra quelli che rispettano i vincoli. Se un'iterazione peggiora, il migliore resta il precedente.

### 5.6 Revisione a parole (REV)

- **REV-1.** `revise` esegue una sola iterazione partendo dal piano migliore, con la richiesta dell'autore come priorità.
- **REV-2.** In revisione non c'è rollback automatico: l'autore ha chiesto il cambio e lo giudica all'ascolto.
- **REV-3.** Il report mostra il diff dei parametri rispetto al piano di partenza.

### 5.7 Master (MAS)

- **MAS-1.** Input: un `mix.wav` e una traccia di riferimento, obbligatoria nella PoC.
- **MAS-2.** Usa Matchering per allineare il mix al riferimento. Output a 24 bit, opzionale a 16 bit.
- **MAS-3.** Dopo il master misura LUFS integrato, true peak e distanza spettrale dal riferimento. Warning se il true peak supera −1 dBTP.

### 5.8 Report (REP)

- **REP-1.** Ogni run produce un `report.md`: tabella delle iterazioni con distanza, LUFS e true peak, motivazioni per stem, collisioni principali, costo totale.
- **REP-2.** `ab` prepara i file per l'ascolto cieco: stessa durata, stesso loudness, nomi casuali, chiave in un file separato.

## 6. Contratti dati

I quattro file qui sotto sono le interfacce tra i moduli. Gli schemi vanno definiti una sola volta, come modelli Pydantic, e da lì si genera il JSON Schema passato al modello. I valori negli esempi sono illustrativi.

### 6.1 `project.yaml`, scritto dall'autore

```yaml
name: "Titolo del brano"
sample_rate: 48000
brief:
  genre: "indie rock"
  mood: "caldo, intimo, un po' sporco"
  notes: "La voce deve stare davanti. Batteria asciutta."
stems:
  - id: kick
    file: stems/kick.wav
    role: kick
  - id: bass
    file: stems/bass.wav
    role: bass
  - id: gtr_l
    file: stems/guitar_left.wav
    role: electric_guitar
    notes: "ritmica, doppiata con gtr_r"
  - id: lead_vox
    file: stems/lead_vocal.wav
    role: lead_vocal
references:
  - file: references/ref1.wav
options:
  pre_normalize: true
  max_iterations: 3
  mix_true_peak_db: -3.0
```

Ruoli ammessi: `kick`, `snare`, `toms`, `overheads`, `drums`, `percussion`, `bass`, `electric_guitar`, `acoustic_guitar`, `keys`, `synth`, `pad`, `lead_vocal`, `backing_vocal`, `fx`, `other`.

### 6.2 `analysis.json`, prodotto dall'analisi

```json
{
  "project": { "sample_rate": 48000, "duration_s": 214.3 },
  "stems": {
    "lead_vox": {
      "role": "lead_vocal",
      "channels": 1,
      "lufs": -21.4,
      "peak_db": -4.1,
      "true_peak_db": -3.8,
      "crest_db": 14.2,
      "dyn_range_lu": 9.3,
      "active_pct": 62,
      "bands_db": [-38.0, -24.1, -12.3, -8.0, -5.2, -9.9, -15.4, -22.0],
      "centroid_hz": 1850,
      "onsets_per_s": 2.1,
      "stereo": null,
      "hygiene": { "dc_offset": 0.0, "clipped_samples": 0, "noise_floor_db": -72.0 }
    }
  },
  "mix": {
    "lufs": -19.8,
    "true_peak_db": -3.0,
    "dyn_range_lu": 8.1,
    "bands_db": [-14.0, -9.5, -8.8, -9.9, -7.1, -12.4, -16.0, -21.5],
    "stereo": { "correlation": 0.82, "side_mid_db": -11.0 },
    "low_corr": 0.97
  },
  "band_share_pct": { "lead_vox": [0, 1, 12, 30, 41, 38, 22, 15] },
  "masking_top": [
    { "a": "gtr_l", "b": "lead_vox", "band": "2k-4k", "score": 0.71 }
  ],
  "target": {
    "n_references": 1,
    "bands_db": [-12.5, -9.0, -9.6, -10.8, -7.5, -11.0, -14.2, -19.0],
    "side_mid_db": -9.5,
    "dyn_range_lu": 6.8
  }
}
```

### 6.3 `mix_plan.json`, deciso dal modello

```json
{
  "version": 1,
  "iteration": 1,
  "summary": "Voce avanti, chitarre aperte ai lati, basse ripulite.",
  "reverb": {
    "room_size": 0.35,
    "damping": 0.5,
    "return_hpf_hz": 250,
    "return_gain_db": -6.0
  },
  "stems": {
    "lead_vox": {
      "gain_db": 2.0,
      "pan": 0.0,
      "hpf_hz": 90,
      "lpf_hz": null,
      "eq": [
        { "type": "peak", "freq_hz": 300, "gain_db": -2.5, "q": 1.4 },
        { "type": "high_shelf", "freq_hz": 9000, "gain_db": 1.5, "q": 0.7 }
      ],
      "compressor": {
        "threshold_db": -18.0,
        "ratio": 3.0,
        "attack_ms": 10.0,
        "release_ms": 120.0,
        "makeup_db": 3.0
      },
      "reverb_send_db": -14.0,
      "rationale": "Taglio a 300 Hz per togliere la sovrapposizione con le chitarre."
    }
  }
}
```

Limiti dei parametri, applicati dalla validazione. `null` spegne il blocco.

| Parametro | Minimo | Massimo |
| --- | --- | --- |
| `gain_db` | −24 | +12 |
| `pan` | −1 (sinistra) | +1 (destra) |
| `hpf_hz` | 20 | 400 |
| `lpf_hz` | 2.000 | 20.000 |
| EQ, numero di bande | 0 | 4 |
| EQ `type` | `peak`, `low_shelf`, `high_shelf` |  |
| EQ `freq_hz` | 20 | 20.000 |
| EQ `gain_db` | −9 | +9 |
| EQ `q` | 0,3 | 8 |
| Compressore `threshold_db` | −40 | 0 |
| Compressore `ratio` | 1 | 8 |
| Compressore `attack_ms` | 0,1 | 100 |
| Compressore `release_ms` | 20 | 1.000 |
| Compressore `makeup_db` | 0 | 12 |
| `reverb_send_db` | −60 | −6 |
| Riverbero `room_size`, `damping` | 0 | 1 |
| Riverbero `return_hpf_hz` | 20 | 800 |
| Riverbero `return_gain_db` | −24 | 0 |

### 6.4 `evaluation.json`, prodotto dal valutatore

```json
{
  "iteration": 1,
  "spectral_distance_db": 1.7,
  "band_delta_db": [3.0, 1.4, -2.2, -3.0, -1.6, 0.4, 0.7, 1.3],
  "width_delta_db": -1.5,
  "dyn_delta_lu": 1.3,
  "masking_top": [
    { "a": "gtr_l", "b": "lead_vox", "band": "2k-4k", "score": 0.48 }
  ],
  "constraints": { "true_peak_ok": true, "low_corr_ok": true, "dc_ok": true },
  "is_best": true,
  "stop_reason": null
}
```

`band_delta_db` è target meno mix, dopo l'allineamento del livello medio: un valore positivo significa che al mix manca energia in quella banda.

## 7. Stack e vincoli tecnici

Tutto gira in locale in Python; l'unica chiamata di rete è quella al modello tramite OpenRouter.

| Ambito | Scelta |
| --- | --- |
| Linguaggio | Python 3.11 o superiore, progetto gestito con `uv` |
| I/O audio | `soundfile` |
| Analisi | `numpy`, `scipy`, `librosa`, `pyloudnorm` |
| Processing | `pedalboard` per filtri, EQ, compressore, riverbero |
| Master | `matchering` |
| Schemi e validazione | `pydantic` |
| Chiamate al modello | Client HTTP verso l'endpoint chat completions di OpenRouter, compatibile OpenAI |
| CLI | `typer` |
| Test | `pytest` |

Vincoli:

- **Segreti.** La chiave sta nella variabile d'ambiente `OPENROUTER_API_KEY`. Il file `.env` non entra nel repository.
- **Privacy.** Nessun audio lascia la macchina. Al modello vanno solo misure, id e ruoli degli stem, brief.
- **Niente GPU.** Deve girare su un portatile normale.
- **Astrazione del decisore.** Il codice dipende da un'interfaccia `Decider` con due implementazioni: modello reale e decisore finto.
- **Funzioni pure.** Analisi, render e valutazione sono funzioni senza stato globale, testabili con array in memoria.
- **Lingua.** Codice, commenti e messaggi della CLI in inglese. Il campo `rationale` nella lingua del brief.
- **Licenze.** `pedalboard` e `matchering` sono GPLv3. Per uso personale non comporta obblighi; conta solo in caso di distribuzione.

Il modello di default non è fissato qui: va scelto tra quelli che supportano output strutturato, partendo da uno economico (sezione 11).

### 7.1 Portabilità

Python è la scelta per la PoC, non necessariamente per l'eventuale applicazione finale: lo stack di quella si decide solo dopo un go, senza condizionamenti presi ora. Ciò che deve sopravvivere a un cambio di linguaggio sono i contratti, le formule e i risultati, non il codice.

- **Schemi come specifica neutra.** I JSON Schema generati dai modelli Pydantic vengono salvati nel repository in `schemas/` e aggiornati a ogni modifica. Sono la definizione dei contratti; il codice Python li implementa.
- **Formule esplicite.** Ogni misura dell'analisi è documentata in `docs/analysis.md` con la sua formula e i parametri esatti (finestra, dimensione e hop della STFT, limiti delle bande, soglia di attività, versione dell'algoritmo di loudness). Filtri ed EQ sono definiti come biquad standard secondo le formule del cookbook RBJ, con i coefficienti derivabili dai soli parametri del piano.
- **Engine senza terminale.** L'engine comunica solo tramite file e JSON e non legge mai dall'input interattivo. La CLI è un involucro sottile sopra funzioni che accettano percorsi e restituiscono dati. Così un altro processo, in qualunque linguaggio, può invocarlo come eseguibile o riscriverlo pezzo per pezzo.
- **Nessun dettaglio di libreria nei contratti.** I file di scambio non contengono nomi di librerie, oggetti serializzati o valori che dipendono dall'implementazione.

Un avviso per il futuro: una riscrittura della catena di render non produrrà campioni identici, perché compressore e riverbero sono implementazioni specifiche della libreria. Un port va rivalidato all'orecchio con il protocollo della sezione 10, non solo con i test. È un costo accettato e rimandato a dopo la decisione go/no-go.

## 8. CLI, repository e output

Sei comandi coprono l'intero flusso. `mixpoc` è un nome provvisorio.

```bash
mixpoc analyze <progetto>                      # scrive analysis.json
mixpoc mix <progetto> [--max-iter 3] [--decider llm|mock] [--export-stems] [--baseline-only]
mixpoc revise <progetto> "voce più avanti, meno riverbero sulle chitarre"
mixpoc master <progetto> [--mix <file>] [--reference <file>]
mixpoc report <progetto>                       # rigenera report.md dell'ultimo run
mixpoc ab <progetto>                           # prepara i file per l'ascolto cieco
```

`<progetto>` è la cartella che contiene `project.yaml`. Ogni comando esce con codice diverso da zero in caso di errore.

`revise`, `master`, `report` e `ab` lavorano sull'ultimo run, salvo indicazione diversa con `--run <cartella>`.

Struttura del repository:

```text
mixpoc/
  pyproject.toml
  README.md
  src/mixpoc/
    cli.py
    project.py          # caricamento e validazione di project.yaml
    audio_io.py
    analysis/
      stem.py
      mix.py
      masking.py
      reference.py
    decision/
      schema.py         # modelli Pydantic del piano
      prompt.py
      llm.py            # decisore reale via OpenRouter
      mock.py           # decisore finto
      validate.py
    render/
      chain.py          # catena per stem
      mixbus.py
    evaluate.py
    loop.py             # ciclo decisione, render, valutazione
    master.py
    report.py
  tests/
```

Cartella di un brano e output di un run:

```text
mio_brano/
  project.yaml
  stems/
  references/
  runs/
    2026-11-09T21-30-00/
      analysis.json
      baseline.wav
      iter_00/
        plan.json
        mix.wav
        evaluation.json
        llm_request.json
        llm_response.json
      iter_01/
      best.json           # indica l'iterazione migliore
      master/
        master.wav
        master_report.json
      report.md
```

Un run non modifica mai i file in `stems/` e `references/`, e non sovrascrive run precedenti.

## 9. Milestone

Otto milestone in sequenza: ognuna si chiude con i test verdi e una pausa per la revisione dell'autore, prima di iniziare la successiva.

1. **M0, fondamenta.** Repository, CLI vuota, caricamento di `project.yaml`, I/O audio, somma grezza.
   - Accettazione: `mixpoc mix --baseline-only` su stem sintetici, senza ancora alcun piano, scrive `baseline.wav` entro −3 dBTP.
2. **M1, analisi.** `analysis.json` completo per stem, mix e riferimenti (ANA-1 → ANA-13).
   - Accettazione: un seno a 1 kHz cade nella banda 500–2k; il rumore rosa dà energia simile nelle bande di pari ampiezza in ottave; due stem identici danno il mascheramento massimo.
3. **M2, render.** Applica un piano scritto a mano (REN-1 → REN-7).
   - Accettazione: +6 dB di gain danno +6 LU misurati, ±0,1; pan a −1 azzera il canale destro; il passa-alto attenua sotto il taglio; due render dello stesso piano hanno lo stesso hash.
4. **M3, decisione.** Schema, prompt, decisore finto, decisore reale, validazione e nuovi tentativi (DEC-1 → DEC-8).
   - Accettazione: il flusso completo gira offline con il decisore finto; con il modello reale esce un piano valido e i log sono completi.
5. **M4, ciclo di verifica.** Valutazione, criteri di stop, piano migliore, report (EVA-1 → EVA-6, REP-1).
   - Accettazione: il report confronta la distanza della somma grezza con quella di ogni iterazione; il piano marcato come migliore ha la distanza minima tra quelli validi; `report.md` viene generato.
6. **M5, revisione a parole.** Comando `revise` (REV-1 → REV-3).
   - Accettazione: con la richiesta "voce più avanti" il report mostra un diff sui parametri della voce o degli stem che la coprono.
7. **M6, master.** Comando `master` (MAS-1 → MAS-3).
   - Accettazione: `master.wav` prodotto; `master_report.json` riporta LUFS, true peak e distanza dal riferimento.
8. **M7, ascolto.** Comando `ab` (REP-2) e protocollo della sezione 10 su 3 brani reali.
   - Accettazione: scheda di ascolto compilata e decisione go/no-go annotata nel README.

Regole di lavoro per Claude Code:

- Una milestone alla volta. A fine milestone: riepilogo di cosa è stato fatto, cosa è stato deciso e cosa resta dubbio.
- Niente funzionalità fuori scope, nemmeno "già che ci siamo".
- Se un requisito è ambiguo o una libreria non fa ciò che serve, fermarsi e chiedere invece di aggirare il problema.
- Nessun test chiama la rete. I test con il modello reale stanno dietro un marker e non girano di default.

## 10. Test e valutazione

I test automatici verificano che l'engine faccia ciò che dichiara; solo l'ascolto dice se il risultato è buono.

### 10.1 Test automatici

- **Segnali sintetici generati nei test:** seni, rumore rosa, impulsi, silenzio. Nessun file audio nel repository.
- **Analisi:** valori attesi noti per ogni misura, con tolleranze esplicite.
- **Render:** un test per ogni blocco della catena e un test di determinismo sull'hash.
- **Schema:** piani validi accettati, piani fuori limite corretti o rifiutati, risposte non JSON gestite.
- **Flusso completo:** `analyze`, `mix`, `revise`, `master` con il decisore finto, su un progetto sintetico creato in una cartella temporanea.

### 10.2 Protocollo di ascolto

1. Scegliere 3 brani dell'autore, diversi per arrangiamento, da 4 a 12 stem ciascuno, con almeno un riferimento a testa.
2. Per ogni brano eseguire `mix` e poi `ab`. I file a confronto sono: somma grezza, mix AI migliore e, se esiste, mix manuale dell'autore.
3. `ab` porta tutti i file allo stesso loudness e li rinomina a caso. Senza questo passo vince sempre il file più forte.
4. Ascoltare su almeno due sistemi, per esempio cuffie e casse.
5. Per ogni file annotare un voto da 1 a 5 su bilanciamento, chiarezza, basse e spazio, più i difetti evidenti. Poi indicare il preferito.
6. Aprire la chiave solo dopo aver votato.
7. Su ciascun brano provare almeno due richieste con `revise` e annotare se il cambio va nella direzione chiesta.

### 10.3 Decisione

- **Go:** le soglie H1 → H5 della sezione 2 sono rispettate.
- **Go con riserva:** H3 fallisce ma i difetti annotati puntano a una causa precisa, per esempio una misura mancante.
- **No-go:** il mix AI non batte la somma grezza e le motivazioni del modello non spiegano i difetti.

## 11. Rischi e domande aperte

Il rischio principale è che le misure non catturino ciò che rende buono un mix: in quel caso il modello decide bene su dati che non contano.

| Rischio | Effetto | Contromisura nella PoC |
| --- | --- | --- |
| Le misure non descrivono ciò che l'orecchio giudica | Piani plausibili ma mix mediocri | Motivazioni per stem, ascolto cieco, set di misure facile da estendere |
| Inseguire lo spettro del riferimento non equivale a un buon mix | Mix forzato verso un arrangiamento diverso | Il target guida il valutatore, ma la soglia H3 è l'ascolto |
| Parametri statici, senza automazioni | Brani con forti differenze tra strofa e ritornello restano sbilanciati | Fuori scope: annotare il difetto, non risolverlo |
| Il modello oscilla o esagera | Iterazioni che peggiorano | Limiti di variazione (DEC-5) e piano migliore (EVA-6) |
| Compressore e riverbero di base | Risultato meno rifinito di una DAW | Accettato per la PoC |
| Analisi e storia fanno crescere i token | Costo e latenza in aumento | Tetto di ANA-13, storia limitata alle ultime 2 iterazioni |

Da decidere prima di partire:

- [ ] Gli stem escono dalla DAW grezzi o con il bilanciamento dei fader? Decide il default di `pre_normalize`.
- [ ] Quale modello usare come default su OpenRouter.
- [ ] Sistema operativo e macchina su cui gira la PoC.
- [ ] I 3 brani per l'ascolto, e per quali esiste un mix manuale.
- [ ] Nome definitivo del pacchetto al posto di `mixpoc`.

Rimandato a dopo un eventuale go: automazioni per sezione, sidechain tra cassa e basso, bilanciamenti presi dai riferimenti separandoli in stem, plugin VST, export di un progetto DAW, master guidato a parole.
