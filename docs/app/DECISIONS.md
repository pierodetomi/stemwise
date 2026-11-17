# Decisioni del prodotto

Registro delle decisioni prese durante design e implementazione. Le decisioni dell'autore hanno id `P-nnn` e
stanno in [FOUNDATION §1.1](../FOUNDATION.md#11-decisioni-prese-con-lautore-2026-11-10); quelle prese
dall'agente in autonomia hanno id `A-nnn` e stanno qui. Una decisione non si cancella: se cambia, se ne aggiunge
una nuova che la sostituisce. Le decisioni della PoC (`D-nnn`) restano in [poc/docs/DECISIONS.md](../../poc/docs/DECISIONS.md).

Formato di una voce:

```markdown
### A-001 — Titolo (data, fase)
Contesto. Alternative considerate. **Scelta** e motivo. Requisiti toccati.
```

## Decisioni dell'agente

### A-001 — Font inclusi dai pacchetti `@fontsource` (2026-11-11, F0)
Il design carica Bricolage Grotesque e Figtree da Google Fonts; l'app deve includerli (NFR-6, design/README).
Alternative: scaricare i TTF a mano; caricarli dalla rete (vietato da NFR-7). **Scelta**: `@fontsource/figtree`
(400/500/600) e `@fontsource/bricolage-grotesque` (500/600), OFL 1.1, impacchettati da Vite; testi di licenza in
`app/ui/public/licenses/`. Requisiti: NFR-6, NFR-7, UI-2.

### A-002 — L'interfaccia importa token e stili dei componenti direttamente da `design/` (2026-11-11, F0)
Alternative: copiare `tokens.css` e `components.css` in `app/ui`; riscriverli in moduli CSS. **Scelta**: alias Vite
`@design` → `design/`; `main.tsx` importa `@design/tokens.css` e `@design/components/components.css`, e
`ui/src/styles/` aggiunge solo ciò che serve all'app (finestra fluida, stati interattivi `:hover`/`:focus-visible`
equivalenti alle classi `is-*` delle tavole). Motivo: il design resta l'unica fonte del linguaggio visivo e il
confronto a vista è diretto. Requisiti: UI-2.

### A-003 — TypeScript 6 invece di 7 (2026-11-11, F0)
`typescript-eslint` 8.72 rifiuta TypeScript 7.0. **Scelta**: `typescript@^6`, ultima linea supportata dal lint
(§8.2 richiede tipi e lint). Si potrà salire a 7 quando il lint lo supporterà.

### A-004 — Elenco delle licenze ammesse (2026-11-11, F0)
NFR-6 elenca MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, Unicode. Alcune dipendenze transitive di Tauri e degli
strumenti usano varianti equivalenti o più permissive. **Scelta**: si ammettono anche `MIT-0`, `0BSD`, `CC0-1.0`,
`BSL-1.0`, `Apache-2.0 WITH LLVM-exception`, `Unicode-DFS-2016`, `CDLA-Permissive-2.0` (Rust, `app/deny.toml`) e,
per npm, `OFL-1.1` (solo i font, come previsto da NFR-6), `BlueOak-1.0.0`, `Python-2.0`, `CC-BY-4.0` (dati di
sviluppo, non impacchettati). Nessuna licenza copyleft, nemmeno tra le dipendenze di sviluppo.

### A-005 — Encoder AAC via AudioToolbox con FFI propria (2026-11-11, preliminare)
Non esiste un encoder AAC in Rust con licenza permissiva. **Scelta**: modulo isolato dietro un trait
(`AacEncoder`), implementato su macOS con `ExtAudioFile` di AudioToolbox via FFI; altrove il formato non è offerto
(MAS-10, NFR-8).

### A-006 — Contenuto di "Appearance" nelle impostazioni (2026-11-11, preliminare)
FOUNDATION (SET-1) dice solo "aspetto". **Scelta**: le opzioni della tavola 11c, salvate nelle preferenze dell'app
(non nel progetto, sono stato di vista: HIS-6).

### A-007 — Tracciabilità dei requisiti per fase (2026-11-11, F0)
`make check` deve eseguire `make reqs` "per le fasi chiuse" (§8.2) ma §8.1 non assegna ogni id a una fase.
**Scelta**: `app/scripts/req-phases.json` assegna ogni id (per prefisso, con eccezioni) alla fase in cui va coperto;
`node scripts/reqs.mjs --closed` fallisce se un id di una fase chiusa (✅/⚠️ in PROGRESS) non è né testato né
giustificato in `REQS-JUSTIFIED.md`. NFR-6 e NFR-9 sono giustificati come passi del gate.

### A-008 — Cartelle temporanee dei test dentro `app/target/tmp` (2026-11-11, F0)
Regola 20: la pulizia tocca solo file ignorati dentro il repository e le cartelle temporanee dei test. **Scelta**: i
test Rust creano le loro cartelle (con `tempfile`, eliminate anche in caso di fallimento) sotto `app/target/tmp`,
che `make disk` misura e `make clean-stale` svuota.

### A-009 — Playwright su WebKit (2026-11-11, F0)
**Scelta**: Playwright usa il progetto WebKit, il motore più vicino a WKWebView di Tauri su macOS.

### A-010 — Ottimizzazione nei profili di sviluppo (2026-11-11, F0)
Il DSP non ottimizzato è 20–50 volte più lento e renderebbe lenti test e `make dev`. **Scelta**: `opt-level = 2`
per le dipendenze e per i crate `stemwise-dsp`, `stemwise-engine`, `stemwise-analysis` anche nel profilo `dev`;
debug ridotto (`line-tables-only`, nessuno per le dipendenze) e niente cache incrementale per le dipendenze
(regola 18).

### A-011 — Verifica dell'app Tauri senza cattura dello schermo (2026-11-11, F0)
`screencapture` chiede all'autore un permesso di sistema: non si usa. **Scelta**: l'app vera si verifica dai log e
da una modalità di autoverifica che esegue i flussi principali attraverso gli stessi comandi Tauri (introdotta con
la fetta verticale di F3); l'aspetto si confronta con le tavole nel browser, sul backend finto.

### A-012 — I WAV si leggono sul posto, gli altri formati da una cache decodificata (2026-11-11, F1)
PRJ-3 chiede di copiare i file nel pacchetto; il motore deve leggerli in streaming con accesso casuale (ENG-1).
Alternative: convertire tutto in un formato interno all'import (raddoppia lo spazio); leggere ogni formato con
symphonia con seek. **Scelta**: la copia in `audio/` è identica all'originale (nome = hash BLAKE3, sola lettura); i
WAV (PCM 16/24/32, float 32/64, EXTENSIBLE, RF64) si leggono direttamente con un parser proprio (`core::wav`);
FLAC e AIFF si decodificano una volta in `cache/pcm/<id>.f32`, rigenerabile. Le piramidi di picchi
(`cache/peaks/`) partono da blocchi di 64 campioni con fattore 4 tra livelli. Requisiti: PRJ-3, PRJ-4, ENG-1, TRN-3.

### A-013 — Encoder FLAC proprio invece di `flacenc` (2026-11-11, F1)
I file prodotti da `flacenc` 0.5.1 non sono letti né da symphonia né dal decoder di sistema di macOS
(`ExtAudioFileRead` −50), mentre un FLAC prodotto da `afconvert` si legge. **Scelta**: in F8 si scrive un encoder
FLAC in Rust (STREAMINFO, blocchi fissi da 4096, predittori fissi di ordine 0–4, codifica Rice partizionata, CRC-8
e CRC-16), verificato rileggendo i file con symphonia. Fino ad allora il test d'import copre WAV e AIFF; la parte
FLAC di PRJ-4 si aggiunge con l'encoder. Requisiti: MAS-7, PRJ-4.

### A-014 — Migrazione dello schema provata su una v0 di bozza (2026-11-11, F1)
La v1 è la prima versione pubblica di `project.json`: non esistono progetti più vecchi. Per provare il meccanismo
di PRJ-8 si definisce una "v0" di bozza (senza `ai` e `snapshots`, tempo come campo `bpm` piatto) che
`package::migrate` porta alla v1; un documento più nuovo della versione corrente si rifiuta con un messaggio chiaro.

### A-015 — Un comando di parametro cambia un solo valore (2026-11-11, F1)
Il test di proprietà sulla cronologia ha trovato che impostare il guadagno di una banda EQ la abilitava come
effetto collaterale, e l'inverso non lo annullava. **Scelta**: `SetParam` cambia esattamente un valore; abilitare
o disabilitare una banda è un `SetTrack`. L'interfaccia, quando l'utente muove una banda spenta, invia entrambe le
modifiche nello stesso gesto. Requisiti: HIS-1.

### A-016 — Mute e solo nel documento, fuori dalla cronologia (2026-11-11, F1)
HIS-6 esclude mute e solo dalla cronologia, ma P2 vuole che lo stesso progetto dia lo stesso render. **Scelta**: mute
e solo stanno nel documento (si salvano e si renderizzano) e cambiano con `Document::apply_view`, che non crea passi
di cronologia. Il mute automatizzato è una corsia come le altre.

### A-017 — Lo snap delle clip si calcola nell'interfaccia (2026-11-11, F1)
CLP-3 (snap a griglia, bordi, cursore, sezioni) serve a 60 fps durante il trascinamento. **Scelta**: è geometria
d'interazione e vive nell'UI (verificata con vitest); il core riceve solo la posizione finale del gesto e applica
CLP-4. Non è logica di dominio duplicata: il core non fa snap. `req-phases.json` assegna CLP-3 a F6.

### A-018 — Id interni brevi e mai riusati (2026-11-11, F1)
**Scelta**: gli id sono un prefisso più un contatore del progetto (`t3`, `g7`, `r1`, `c12`, `s2`), mai riusati.
Verso il modello si inviano comunque id neutri rinumerati (§5.5).

### A-019 — Processori propri: forme scelte (2026-11-11, F2)
FOUNDATION vieta le librerie GPL della PoC; compressore, riverbero, delay, saturazione e limiter sono propri
(`docs/app/dsp.md`). **Scelte**: compressore feed-forward con detector di picco collegato sulle due uscite, knee
quadratico (Giannoulis et al.) e balistica in dB; riverbero di tipo Freeverb (8 comb + 4 allpass per canale, tarature
note) calibrato sui livelli di ritorno misurati sulla PoC (±1,5 dB); delay stereo con tempo che scivola (50 ms) e
lettura frazionaria; saturazione `2·tanh(u/2)` a 4x, perché `tanh` pieno aliasava a −46 dB sul test a 15 kHz;
oversampler FIR Kaiser da 257 tap con latenza intera di 64 campioni (ENG-5); limiter true peak con detector 4x della
PoC, hold e media mobile di 2 ms (il tetto vale per costruzione) e margine interno di 0,1 dB. Requisiti: FX-3 … FX-6,
MAS-1, MAS-3.

### A-020 — Griglia dei parametri a 32 campioni ancorata al tempo (2026-11-11, F2)
AUT-6 chiede aggiornamenti al più ogni 32 campioni con coefficienti interpolati, ENG-4 l'invarianza dalla dimensione
del blocco. **Scelta**: il motore valuta automazioni e smoothing su una griglia di 32 campioni ancorata al frame 0
della timeline; i guadagni seguono una rampa per campione dentro il sotto-blocco, filtri ed EQ ricalcolano i
coefficienti a ogni sotto-blocco; i processori con smoothing interno lo fanno per campione. Così 64 e 4096 campioni
per blocco danno lo stesso risultato.

### A-021 — Rischio di prestazioni della saturazione (2026-11-11, F2)
L'oversampler a 257 tap costa circa 500 moltiplicazioni per campione e canale. Con 32 tracce stereo e la saturazione
attiva ovunque potrebbe pesare su NFR-1. **Scelta**: si misura al benchmark di F3; se serve, si riduce il filtro
(stopband 80 dB con meno tap) o si elabora il grafo su più thread.

### A-022 — Oversampling a due stadi half-band e `tanh` razionale (2026-11-11, F3; sostituisce in parte A-019)
Il FIR da 257 tap di A-019 costava 400 ns per campione (benchmark NFR-1 al 137%). **Scelta**: due stadi 2x
half-band polifase in f32 (56 e 14 tap dispari, Kaiser β 8,2 e 7,9, stopband ≥ 80 dB verificata sui coefficienti),
latenza complessiva fissata a 64 campioni (multiplo della griglia di 32, A-020) con un campione di ritardo; il
sagomatore `2·tanh(u/2)` usa la frazione continua di Lambert (errore ≤ 1e-4 fino a |x| 4,97, poi saturazione a ±1)
applicata ai quattro punti sovracampionati insieme. Costo: circa 65 ns per campione. Requisiti: FX-4, ENG-5, NFR-1.

### A-023 — Il gate di NFR-1 è sul 99° percentile del tempo di blocco (2026-11-11, F3)
FOUNDATION chiede che il callback usi "al più" il 30% del periodo. Un benchmark su un thread non real-time misura nel
massimo anche le prelazioni del sistema operativo. **Scelta**: il gate è il p99 del tempo di blocco (≤ 30%); media e
massimo sono stampati. Il benchmark copre due scenari: catena completa su 32 tracce stereo con un gruppo e mandate a
due ritorni, e lo stesso con automazione rapida di fader, pan, passa-alto, frequenza e guadagno di tutte le bande.
Misura attuale: media 9%, p99 14%, massimo 20%. Requisiti: NFR-1.

### A-024 — Thread ausiliari per il grafo in tempo reale (2026-11-11, F3)
Anche dopo A-022 il callback su un solo thread restava al 28–31% del periodo. **Scelta**: i nodi indipendenti di un
livello del grafo si dividono tra il thread audio e fino a 3 thread ausiliari (`stemwise-engine::pool`). Nessuna
allocazione né lock sul thread audio: il lavoro si assegna con un'unica parola atomica (epoca e indice) e un
compare-and-swap, il thread audio lavora anch'esso e poi attende in spin; gli ausiliari, inattivi, dormono e vengono
svegliati con `unpark`. Gli accumuli verso le destinazioni restano seriali e in ordine fisso: il risultato è identico
al percorso a un thread (stesso hash). Rischio residuo: un ausiliario descheduled a metà di un nodo ritarda il blocco;
registrato in `VALIDATION.md` (ascolto con molte tracce). Requisiti: NFR-1, NFR-2.

### A-025 — Mute, solo e fader del bus senza ricostruire il grafo; dissolvenze sul trasporto (2026-11-11, F3)
La revisione di F3 ha trovato che mute, solo e fader del bus passavano da una ricostruzione del grafo (buco di
silenzio, code tagliate). **Scelta**: il mute è il parametro `mute` (rampa di una cella); il solo è una maschera
calcolata sul lato di controllo e applicata con rampa (`SetSolo`, stato di vista); il fader del bus è un parametro.
Le modifiche strutturali vere (aggiungere una traccia, spostare una clip) sostituiscono il grafo con una dissolvenza
in uscita di 128 campioni, una breve pausa di riempimento dei buffer (qualche decina di ms) e una dissolvenza in
entrata; lo stesso per pausa, locate e loop. Il solo segue la logica solo-in-place: ritorni e gruppi che li ricevono
restano udibili; un ritorno in solo continua a ricevere le mandate; la chiave sidechain è presa prima del solo.
Requisiti: ENG-3, TRK-2, FX-8, TRN-1.

### A-026 — Flush-to-zero sul thread audio (2026-11-11, F3)
**Scelta**: il callback di `cpal` imposta FZ (aarch64) o FTZ/DAZ (x86_64) sul proprio thread alla prima chiamata, così
i denormali non costano nulla anche sui Mac Intel (P-002). Il render offline non lo imposta: il suo determinismo è per
build e thread, e la differenza è solo nel campo dei denormali (inudibile).

### A-027 — L'analisi di progetto vive nel crate `stemwise-ai` (2026-11-11, F4)
§5.1 vuole `analysis` dipendente solo da `dsp`, ma ANA-14 chiede di misurare il contenuto delle tracce dopo l'editing
delle clip (codice del motore) e ANA-15/16 usano sezioni, gruppi e ruoli (core). **Scelta**: `stemwise-analysis`
contiene solo le misure su segnali (port della PoC, parità con l'oracolo); l'analisi di un progetto (lettura delle
tracce con `TrackStream`, cache, sezioni, famiglie, documento per il modello con id neutri) sta in
`stemwise-ai::analysis_doc`, che può dipendere da core, analysis ed engine.

### A-028 — Cache dell'analisi e mix analizzato (2026-11-11, F4)
**Scelta**: ogni traccia ha una chiave (BLAKE3 di clip, media, canali, sezioni, frequenza e fine del brano): i
parametri della catena non la cambiano, l'editing delle clip sì (ANA-14). Il risultato in cache comprende le misure,
le misure per sezione, il pre-gain verso −23 LUFS e le potenze per banda dei frame (per quote e mascheramento,
calcolati sugli stem pre-normalizzati come in D-028 della PoC). Il mix analizzato è la somma grezza con la legge
del centro e il trim a −3 dBTP (la `baseline` della PoC). Le tracce si analizzano su al più 4 thread per tenere la
memoria sotto NFR-5.

### A-029 — Onset in f64 (2026-11-11, F4)
librosa calcola STFT, banco mel e decibel in f32; il port in Rust lavora in f64. Sui 22 casi dell'oracolo la densità
di onset coincide entro ±0,05 onset/s, quindi non serve emulare l'f32.

### A-030 — Il decisore finto (2026-11-11, F5)
AI-9 chiede un decisore finto deterministico con le regole della PoC estese. **Scelta** (`stemwise-ai::decider`): le
regole della PoC (guadagno, pan, passa-alto per ruolo, tagli di EQ sulle collisioni con le tracce prioritarie,
compressori su voce e basso, correzione verso il riferimento dalla seconda iterazione, regole a parole per la
revisione) estese con: sidechain del basso dalla cassa, saturazione leggera su basso e chitarre elettriche, gruppo di
batteria −1 dB, delay a un ottavo puntato del tempo (375 ms senza tempo), voce +1,5 dB nei ritornelli e più riverbero
nel bridge, parole per il delay. Le motivazioni seguono la lingua del brief: italiano se il brief contiene almeno due
parole funzionali italiane, inglese altrimenti (AI-13). Il master finto propone match EQ se c'è un riferimento,
mensole leggere dove il mixdown si scosta dal profilo, compressore se l'LRA supera 8 LU, mono sotto 120 Hz se la
correlazione delle basse è sotto 0,6, e reagisce a "più forte", "più brillante", "più largo".

### A-031 — Richiesta a OpenRouter (2026-11-11, F5)
**Scelta**: `POST /chat/completions` con `response_format` `json_schema` e `strict: true` (ripiego automatico a
`json_object` quando il provider rifiuta lo schema: 400/422 che citano schema o response_format), `reasoning.effort`
dalla configurazione, `usage.include` per avere il costo nella risposta, intestazione `X-Title: Stemwise` (nessun dato
personale). La chiave sta solo nell'intestazione e nel portachiavi; il corpo registrato nel log non la contiene. La
richiesta gira su un thread a parte: l'annullamento restituisce subito (AI-12) e la chiamata abbandonata conta con il
costo stimato dai token del prompt.

**Aggiornamento dopo la revisione di F5.**
- Lo schema mandato al modello si costruisce per progetto (`plan::mix_plan_schema_for`): ogni id modificabile è una
  proprietà obbligatoria, mandate e scostamenti per sezione sono elencati per ritorno e ammettono `null`, il tipo di
  ogni ritorno è fisso, ogni limite di `LIMITS` è dentro lo schema. È il sottoinsieme "strict" dei provider
  compatibili con OpenAI, come lo schema della PoC (D-039).
- Il ripiego a JSON mode resta ricordato per tutta la sessione del client.
- Al nuovo tentativo la risposta sbagliata torna come messaggio `assistant`, seguita dagli errori (come `llm.py`).
- Le recinzioni Markdown attorno al JSON si tolgono.
- `finish_reason: length` e un `error` dentro una risposta 200 sono errori, con il loro consumo.
- Le chiamate annullate durante la richiesta (`Abandoned`) e quelle cadute per errore di rete contano i token del
  prompt al prezzo di listino, marcati come stima.

### A-032 — Stima dei costi e tetto (2026-11-11, F5)
**Scelta**: token d'ingresso = caratteri dei messaggi / 4 per ogni iterazione, più i piani precedenti nella storia;
token d'uscita ≈ 180 per traccia o gruppo più 150 (400 per il master); costo dal listino configurabile del modello
(default 1,25 $ / 10 $ per milione di token in/out, da aggiornare dall'autore). Un tetto di spesa del progetto,
facoltativo, segnala quando speso + stima lo supererebbe (AI-16): l'interfaccia chiede conferma (tavola 12e).

### A-033 — Valutazione del master AI rimandata al grafo di master (2026-11-11, F5)
Il master AI di F5 decide sulle misure del mixdown e propone un piano validato, applicabile e annullabile; la misura
del master risultante (LUFS, true peak, distanza dal riferimento) richiede il grafo di master di F8 e si aggiunge lì
al flusso (MAS-6).

### A-034 — Ambito, ritorni e blocchi nella revisione (2026-11-11, F5, dopo la revisione)
**Contesto**: AI-4 e AI-5 parlano di tracce e gruppi; i ritorni sono condivisi da tutte le tracce.

**Scelte**:
- Una richiesta con ambito (alcune tracce) lascia intatti tutti i ritorni: cambiare il riverbero cambierebbe anche le
  tracce fuori dall'ambito.
- Un ritorno bloccato è solo contesto, come una traccia bloccata. Il modello vede `locked` sui ritorni e su tutto ciò
  che è fuori dall'ambito, sia nell'analisi sia nell'elenco delle tracce.
- Bloccare un gruppo protegge le impostazioni del gruppo, non quelle delle tracce che contiene: il lucchetto è per
  canale, come la riga del lucchetto nell'intestazione. Chi vuole proteggere le tracce le blocca una per una.
- L'applicazione conserva il bypass dei ritorni e la sincronizzazione musicale del delay se il tempo non cambia.
- Un valore che l'autore ha messo fuori dai limiti dell'assistente (un fader a −40 dB, una mandata a −3 dB) arriva
  esatto nel piano di partenza. È accettato se il modello lo rimanda invariato; i limiti valgono per ciò che cambia.
- Gli scostamenti per sezione già scritti dall'assistente si ricostruiscono dalle sue corsie (valore a metà sezione
  meno il valore statico). Così la revisione li conserva. Un parametro senza scostamenti non riceve corsie piatte.

### A-035 — Il master AI non cambia la destinazione (2026-11-11, F5, dopo la revisione)
La destinazione (LUFS e tetto del true peak) è una scelta dell'autore. Il master AI propone la catena; il `target` del
piano si riporta alla destinazione corrente, salvo in una revisione a parole, dove l'autore può chiedere un altro
livello. L'`input_gain_db` del master ha come limite ±12 dB: FOUNDATION non lo fissa, e ±12 copre la differenza
tipica tra un mixdown a −3 dBTP e qualunque destinazione, con il limiter che garantisce il tetto.

### A-036 — Nomi delle tracce verso il modello (2026-11-11, F5, dopo la revisione)
FOUNDATION ammette i nomi delle tracce e vieta nomi di file e titolo. Il nome predefinito di una traccia viene dal
nome del file, che spesso contiene il titolo o un numero di presa. **Scelta**: prima di arrivare al modello ogni nome
perde le occorrenze del nome del progetto (se lungo almeno 3 caratteri) e i numeri iniziali; un nome vuoto diventa
"Track" (`analysis_doc::model_name`). Il nome nel progetto non cambia.

### A-037 — Contratti esportati dell'AI (2026-11-11, F5, dopo la revisione)
`app/schemas/` contiene `mix_plan.schema.json` (forma generica) e `master_plan.schema.json` (con i limiti), più
quelli del core. Lo schema completo di un progetto si genera alla chiamata. Il documento `analysis` v2 resta
definito dal codice che lo costruisce (`analysis_doc::model_document`) e dai test che ne controllano campi, id neutri
e budget di token: è un oggetto JSON senza un tipo Rust da cui derivare uno schema, e duplicarlo a mano lo farebbe
divergere.

### A-038 — Le modifiche dell'interfaccia sono dati: `Edit` (2026-11-11, F6)
**Contesto**: §5.6 vuole un'interfaccia senza logica di dominio. Le modifiche di clip, sezioni, tracce e automazione
sono molte e devono restare un passo ciascuna.
**Scelta**: un'enumerazione `stemwise_core::edit::Edit`, esportata come tipo TypeScript, e un comando `edit(edit,
label)`: il core calcola i comandi (rifilatura sotto una clip spostata, controlli di routing, limiti delle corsie,
semplificazione del disegno a mano libera) e li applica come una transazione. Per i trascinamenti che devono
suonare mentre si muovono (curva spostata, mandata nuova, fader del bus) c'è `gesture_edit`: il gesto torna al suo
stato iniziale e riapplica la modifica (`Document::gesture_replace`), così il risultato non accumula errori. Le clip
invece si trascinano in anteprima nell'interfaccia e diventano un solo `edit` al rilascio. Il backend finto ha una
copia semplificata delle stesse modifiche, solo per Playwright e la verifica a vista. Nel gesto un parametro già
toccato si aggiorna dov'è (frequenza e guadagno di una banda alternati restano un passo compatto). Le chiamate a
Tauri partono in coda, una dopo l'altra, così inizio, valori e fine di un gesto arrivano in ordine.

### A-039 — Spettro in tempo reale della traccia selezionata (2026-11-11, F6)
FX-7 chiede lo spettro della traccia nell'editor EQ. **Scelta**: il thread audio copia l'uscita post-fader del nodo
osservato in un anello senza lock; il lato di controllo calcola la FFT a 4096 punti dell'analizzatore di
`stemwise-dsp` e la riduce a 96 bande logaritmiche da 20 Hz a 20 kHz, inviate con i meter (`MeterSnapshot.spectrum`).
L'osservazione segue la selezione (`watch_spectrum`). Nessuna FFT sul thread audio.

### A-040 — Stati e controlli che il design non mostra (2026-11-11, F6)
- **Zoom verticale**: un pulsante "Track height" accanto allo zoom cicla le tre altezze di corsia (TRN-3).
- **Righello**: sempre in minuti:secondi (TRN-2); con un BPM le battute danno solo il passo della griglia di snap.
- **Corsia di automazione**: nell'intestazione, oltre al selettore del parametro, il valore corrente e un pulsante per
  nasconderla. Il selettore scambia la corsia visibile, senza perdere punti; in fondo c'è "Delete this automation".
- **Automazione col mouse** (AUT-2): con la penna, clic aggiunge un punto, trascinare un punto lo sposta, trascinare
  sul vuoto disegna. `⇧`+trascinare seleziona un intervallo, `⌫` lo cancella, `⌘C`/`⌘V` lo copiano e lo
  incollano al cursore.
- **AUT-3 durante la riproduzione**: muovere un controllo automatizzato sposta la curva anche col trasporto in moto.
  La v1 non ha modalità di scrittura, quindi non c'è un comportamento alternativo da offrire.
- **Compressore**: le cinque manopole del design più il knee in un campo numerico sotto (FX-3 lo chiede regolabile).
- **Cronologia**: in fondo all'elenco una voce "Project opened" riporta allo stato iniziale.
- **Pulsante Master**: fino a F8 mostra un avviso; il dialogo 8a arriva col mixdown.
- **Riferimenti e snapshot**: nella schermata 2 i riferimenti si aggiungono dal brief dopo la creazione; il pannello
  degli snapshot è completato in F7 con HIS-4.
- **Eliminare una traccia o un ritorno** chiede conferma (13g) anche se è annullabile, come suggeriscono i puntini di
  "Delete track…" nella tavola 6e.

### A-041 — L'assistente nell'app (2026-11-11, F7)
- **Una chiamata alla volta.** Finché una proposta è in revisione non se ne avvia un'altra: va applicata o scartata.
  Durante la chiamata lo spazio di lavoro resta utilizzabile (AI-12) e la chiamata parte dallo stato del progetto
  al momento della richiesta.
- **A/B.** Il motore carica la versione da ascoltare (proposta o snapshot) senza toccare il documento e senza un
  passo di cronologia (AI-2, HIS-4). Una modifica strutturale fatta durante l'ascolto riporta l'ascolto al mix
  corrente, perché il motore suona di nuovo il documento.
- **Applicazione.** I comandi si ricalcolano sul progetto com'è al momento di applicare (`apply_commands`), così le
  modifiche fatte durante l'ascolto restano. Il passo porta l'id della chiamata (HIS-3), e la chiamata riceve
  l'esito (applicata, applicata in parte, scartata) e l'etichetta del passo. Ogni chiamata, anche fallita o
  annullata, entra nel registro dei costi di tutti i progetti, e il progetto si salva in automatico (PRJ-2).
- **Stima prima dell'invio** (AI-16). La dimensione dei messaggi si stima dal numero di tracce e sezioni, senza
  costruire l'analisi: dà un ordine di grandezza in tempo reale, mentre la chiamata vera conta i token veri. Il
  tetto si imposta nel pannello AI usage, dove porta anche "Change cap…" del dialogo 12e.
- **Snapshot.** Salvare uno snapshot aggiunge un nome alla lista senza un passo di cronologia (la lista non è il
  mix); ripristinarlo è un passo; eliminarlo toglie il nome e lascia il file nel pacchetto.
- **Brief e riferimenti** si modificano da "Edit brief" nella colonna dell'assistente (PRJ-7): il brief è un passo
  di cronologia; aggiungere o togliere un riferimento pure.
- **Chiave** (AI-11): un dialogo minimo (chiave nel portachiavi, oppure demo offline) finché le impostazioni
  complete arrivano in F9. Il self-test usa la demo offline e una chiave in memoria; con `STEMWISE_DATA` usa una
  cartella dati temporanea.

### A-042 — La fase di master nell'app (2026-11-11, F8)
- **Grafo di master.** Il motore carica un progetto di ascolto con una sola traccia stereo neutra che suona il
  mixdown (o un riferimento), e la catena di master sul bus (`Graph::set_master_chain`). I parametri arrivano su una
  coda propria di valori `Copy` (`MasterParams`), applicati senza ricostruire il grafo né allocare. Ogni
  configurazione ha la stessa latenza: i blocchi spenti sono sostituiti da linee di ritardo (raffinato in A-043).
- **Loudness della destinazione** (MAS-2). Un guadagno prima del limiter, trovato offline sul mixdown: una passata
  senza limiter per misurare, poi correttive con il limiter (fino a due; da A-055 fino a sei, guidate dalla pendenza). Si ricalcola in un thread dopo ogni modifica,
  e un'epoca scarta i risultati superati. È deterministico (MAS-8).
- **Limiter.** Il suo bypass vale solo all'ascolto: l'export limita sempre, così il tetto è garantito sul file
  (MAS-3). Il file viene poi ridecodificato e misurato col true peak 4x; un AAC sopra il tetto si ricodifica con il
  margine che serve (MAS-10).
- **A/B a pari loudness** (MAS-5). Il mixdown suona con la catena in bypass e il guadagno che lo porta alla loudness
  del master; un riferimento suona come traccia del progetto di ascolto con lo stesso criterio. La proposta del
  master AI si ascolta allo stesso modo.
- **Mixdown** (MDN-1, MDN-4). È un file WAV float in `mixdowns/`, con trim solo in attenuazione fino a −3 dBTP e
  l'hash del mix. Registrarlo non è un passo di cronologia: il file esiste comunque, e rifarlo crea il numero
  successivo.
- **Esportazioni deterministiche** (MAS-8). Il chunk `bext` riceve la data dal chiamante; i frame ID3 si ordinano
  per identificatore, perché la conversione generica di `lofty` passa per una mappa. Due export uguali danno gli
  stessi byte per WAV e FLAC; l'AAC dipende dall'encoder di sistema.
- **Ricampionamento** all'export: prima della catena, così il limiter lavora alla frequenza del file.
- **Limiti dei controlli del master nell'interfaccia.** EQ ±12 dB, ratio fino a 10 e simili: i limiti più
  stretti di §5.5 valgono per i piani dell'AI. (ISRC: vedi A-043, ora validato nel core.)
- **Master AI applicato in parte**: per modulo (input, eq, match, compressor, width, saturation, limiter,
  destinazione), come un solo passo della cronologia del master.
- **Copertina**: copiata nel pacchetto (`masters/cover.jpg|png`) e scritta in ogni formato.
- **Encoder e tag** sono nel crate `stemwise-export`. Tre subagenti li hanno scritti in parallelo su file
  separati; l'orchestratore ha rieseguito le verifiche.

### A-043 — Correzioni della revisione di F8 (2026-11-11, F8)
La revisione indipendente ha trovato 3 bloccanti e 9 rilievi. Correzioni:
- **Nessun gradino nel master** (B2, ENG-3).
  - Gli elementi continui inseguono il valore nuovo ogni 32 campioni, con costante di 10 ms. Sono:
    - i guadagni: ingresso, loudness, A/B, uscita;
    - le bande dell'EQ, che per cambiare tipo passano da 0 dB;
    - il match EQ e la larghezza.
  - Accendere o spegnere un blocco, e l'A/B, è una dissolvenza incrociata.
  - Saturazione e limiter lavorano anche da spenti e le linee di ritardo girano sempre. Una dissolvenza non fa quindi
    sentire audio vecchio (N8) né un look-ahead vuoto.
  - Con parametri costanti (render, export) l'inseguimento è già arrivato: niente cambia rispetto a prima.
  - Il test `master_changes_glide_instead_of_stepping` misura il salto massimo fra campioni dopo ogni tipo di
    modifica.
- **Parametri numerati** (S5). Ogni `set_master` ha un numero di sequenza e ogni grafo ricorda il numero con cui è
  stato costruito. Quando arriva uno scambio con parametri più vecchi, il thread audio gli applica i più recenti.
- **Tetto sul file scritto** (S2, MAS-3, MAS-10).
  - WAV e FLAC vengono riletti col decoder (symphonia per il FLAC: un decoder indipendente rilegge il nostro
    encoder, cfr. N4) e misurati.
  - Se superano il tetto si scalano i float dell'eccesso più un passo di quantizzazione, poi si riscrive.
  - Dopo 6 tentativi l'export fallisce invece di consegnare un file fuori tetto. Lo stesso vale per l'AAC.
  - I rapporti sono misurati sui file decodificati.
  - Il test copre tutti i blocchi accesi, contenuto con picchi fra i campioni, tetti a 0 e −0,1, frequenze fino a
    96 kHz e 16, 24 e 32 bit.
- **AAC fino a 48 kHz** (B3). L'encoder di AudioToolbox non va oltre. Sopra quella frequenza il dialogo non offre
  l'AAC (MAS-10) e la pipeline rifiuta le impostazioni prima di scrivere qualsiasi cosa (`pipeline::check`).
- **Annullamento dell'export** (S3). La richiesta di annullamento passa per il callback di avanzamento; i file già
  scritti da quell'export si rimuovono.
- **Riferimenti nell'A/B** (B1).
  - Alla prima misura ogni riferimento si converte una volta in `cache/listen-<id>-<rate>.wav` (stereo float alla
    frequenza del progetto); il file importato resta com'è. Così MP3, M4A e frequenze diverse suonano giusti.
  - La cache del master dipende anche dall'elenco dei riferimenti.
  - Se il cambio di ascolto fallisce, si torna alla scelta precedente.
  - Il self-test nell'app vera ascolta un riferimento AAC a 44,1 kHz in un progetto a 48 kHz.
- **A/B a pari loudness senza clip** (S7).
  - Si calcola il guadagno che porta il mixdown e ciascun riferimento alla loudness del master. Se così uno di loro
    supera −1 dBTP, tutte le sorgenti dell'A/B, master compreso, suonano più basse della stessa quantità
    (`ab_trim_db`, mostrata accanto al selettore).
  - Il trim si applica dopo il limiter, quindi non cambia il suono del master.
  - La proposta del master AI suona alla propria loudness, misurata insieme alla proposta.
- **Undo e mixdown** (S1, MDN-2).
  - `SetMaster` conserva sempre il mixdown corrente, all'applicazione come all'inverso. Solo il nuovo comando
    `SetMixdown`, registrato fuori dalle cronologie, lo cambia. Undo e redo del master non riportano più a un
    mixdown vecchio, e una copia vecchia dello stato non può sovrascrivere un bounce.
  - In fase Master il motore passa subito al mixdown nuovo.
- **Lavori legati al progetto** (S6). La sessione ha un `open_id` che cambia a ogni apertura e chiusura. Bounce,
  export e master AI lo ricordano e scartano il risultato se il progetto non è più quello. L'epoca della loudness
  non riparte da zero: un risultato vecchio non coincide mai con una richiesta nuova.
- **Thread della loudness** (S4). La decisione di fermarsi e il flag `loud_running` cambiano sotto lo stesso lock:
  una modifica arrivata dopo trova il flag spento e avvia un thread nuovo. Il calcolo del guadagno avviene una volta
  sola per modifica (N2).
- **ISRC e catena validati nel core** (S8, S9).
  - `MasterState::check` impone i limiti dei controlli (tabella `master_limits`), le frequenze, le profondità di
    bit, i bit rate e l'ISRC su ogni `SetMaster` dell'interfaccia.
  - L'ISRC vale in forma compatta o con tutti i trattini, ed è la stessa regola che usano i tag (spostata nel core).
  - Il dialogo mostra lo stato con la stessa regola e non salva un ISRC non valido.
- **Il crate `stemwise-export`** (S9) non compare in §5.1. Contiene encoder, tag e pipeline di export. Dipende da
  `engine`, che fornisce catena e render, e da `ai`, per i DTO della proposta del master mostrata accanto allo stato
  del master. L'alternativa, cioè mettere tutto in `engine`, avrebbe portato `lofty` e la FFI di AudioToolbox nel
  motore audio. È un'aggiunta all'architettura: i confini di §5.1 restano (nessun crate dipende dall'app, il core non
  dipende da nessuno).
- **Nome vuoto all'export** (N9). Il file si chiama `Untitled`. Un file con lo stesso nome nella cartella viene
  sostituito, come dice il suggerimento sul campo.
- **Restano come sono, con motivo:**
  - N1: data e ora del `bext` dall'orologio, già dichiarate in A-042.
  - N5: una sola profondità di bit per WAV e FLAC; il FLAC si ferma a 24.
  - N6: il guadagno di loudness è limitato a ±24 dB; il "fuori target" resta visibile nelle misure.

### A-044 — Rifinitura: scorciatoie e impostazioni (2026-11-11, F9)
- **Una sola tabella di scorciatoie** (`ui/src/shared/shortcuts.ts`, UI-3). La usano i gestori da tastiera, i
  suggerimenti degli strumenti e l'elenco in Impostazioni › Scorciatoie (SET-1, in sola lettura come vuole la v1).
- **Differenze dal design 11d.**
  - ⌘S salva il progetto, perché è l'abitudine di ogni applicazione macOS. Lo snapshot passa a ⇧⌘S.
  - "Clip gain ⌘G" non c'è: il guadagno della clip ha già il menu contestuale e il trascinamento.
  - Si aggiunge ⌘, per le Impostazioni.
- **Impostazioni** (schermata 11).
  - Audio: dispositivo, buffer, carico del motore, conversione di frequenza in uscita. Il "Play test tone" non c'è: il
    motore suona solo progetti e il segnale di prova richiederebbe un percorso apposta nel thread audio; si verifica
    ascoltando il progetto (VALIDATION).
  - AI: decisore, chiave nel portachiavi (mai mostrata: si vede solo se c'è), modello, sforzo, temperatura, tetto
    di spesa del progetto aperto, cosa si invia, totali su tutti i progetti (con il numero di progetti dal registro).
  - Aspetto: preferenze dell'utente, non del progetto, salvate nel `localStorage` della webview, con i valori di
    default se manca. Riguardano altezza delle corsie, nome e forma d'onda sulle clip, badge del guadagno, posizione
    in battute (solo con tempo), tenuta dei picchi, picco numerico sulle strisce e meno animazioni.
  - Il righello resta in minuti e secondi (TRN-2).
  - Si entra dall'icona a ingranaggio nelle barre Mix e Master, dalla welcome e con ⌘,.

### A-045 — Rifinitura: stati, accessibilità, prestazioni (2026-11-11, F9)
- **File mancanti** (UI-6, 13d).
  - Il core sa quali file mancano dal pacchetto (`package::missing_media`); l'evento di progetto li porta all'UI.
  - Il motore suona in silenzio le clip senza file invece di rifiutare l'intero grafo. Il bounce invece si rifiuta
    finché mancano file, perché un mixdown con clip mute sarebbe sbagliato senza dirlo.
  - "Locate…" (`media::relink`) importa il file scelto come al primo import. Se il contenuto è lo stesso torna al
    suo posto e il documento non cambia. Un file diverso con gli stessi canali e la stessa durata diventa un media
    nuovo per le clip (`RelinkMedia`, un passo annullabile). Ogni altra cosa viene rifiutata senza lasciare file.
- **Dispositivo scollegato** (13d). L'UI interroga lo stato dell'uscita ogni secondo mentre c'è un progetto aperto
  (il dispositivo può sparire anche da fermo). Un banner sotto la barra offre un altro dispositivo e le impostazioni.
- **Rete assente** (13f). Un errore "network unavailable" del client mette, al posto del riquadro della chiave,
  "No connection to OpenRouter" con "Try again" e "Use offline demo". Il toast resta per il dettaglio.
- **Stati vuoti** (13a, 13b) come nel design: timeline con zona di rilascio, mixer, inspector, assistente, brief,
  cronologia, corsia delle sezioni, mandate senza ritorni.
- **Forme d'onda all'apertura** (13c). Se nel pacchetto manca la cache (una copia non porta con sé `cache/`), un
  thread la ricostruisce file per file; ogni clip si ridisegna quando il suo file è pronto.
- **Accessibilità** (UI-7). Il test usa axe-core (WCAG 2 A e AA, violazioni serie e critiche) su welcome, Mix,
  Impostazioni, AI usage e Master, più la tastiera sui controlli continui. Tre scostamenti dal design, perché
  FOUNDATION vince:
  - un blocco spento della catena non abbassa più l'opacità del testo, ma ha contorno e valore barrato;
  - il rosso dei tag d'errore è schiarito verso il colore del testo;
  - i moduli del master sono pulsanti con stato premuto, non schede: una scheda non può contenere il LED.
  Le righe della tabella AI usage hanno celle con ruolo.
- **Memoria** (NFR-5). L'analisi per l'assistente arrivava a 2,5 GB su 32 stem da 5 minuti. Ora sta a 1,0 GB:
  - loudness integrata, LRA e true peak lavorano in streaming sul segnale, senza copie `f64`: stessi blocchi di
    pyloudnorm, valori uguali a meno di 1e-9 LU (test contro il calcolo diretto, parità con l'oracolo invariata);
  - le somme del mix e delle famiglie sono in `f32`;
  - si analizzano 2 stem alla volta invece di 4 (ANA-17 resta a 13 s su 30).
- **Prestazioni** (NFR-3, NFR-5).
  - `make bench` misura l'import (16 stem da 4 min, 1,7 s su 15) e la memoria di un progetto da 32 stem da 5 min,
    importato, suonato 20 s dal motore in tempo reale e analizzato (1,0 GB su 1,5).
  - Un test Playwright misura i 60 fps della timeline con 32 tracce visibili in riproduzione: 60 fps nel WebKit di
    Playwright; il test tollera al più il 5% di frame oltre 33 ms (misurati 0–1 su 180).
  - L'avvio a freddo è il tempo dall'avvio del processo al primo disegno dell'UI (`ui_ready`). Il self-test lo
    misura nell'app; sotto `tauri dev` include Vite e la build di debug, quindi si misura sull'app impacchettata in
    F10.

### A-046 — Consegna: pacchetto e politiche verificate (2026-11-11, F10)
- **`.dmg` senza la finestra personalizzata.** Il bundler di Tauri apre il volume nel Finder via AppleScript per
  disporre le icone. Senza una sessione Finder raggiungibile la richiesta scade e il `.dmg` non viene creato.
  - Primo tentativo: `make build` così com'era, fallito per timeout dell'AppleEvent.
  - Ripiego: `CI=true`, con cui il bundler salta quel passo. Il contenuto dell'immagine è lo stesso, senza la
    disposizione delle icone; `hdiutil verify` la dà valida.
- **Firma.** L'app ha solo la firma ad hoc del linker: senza certificati, il primo avvio richiede clic destro ›
  Apri (VALIDATION 23). Firma e notarizzazione restano all'autore.
- **Avvio a freddo** (NFR-3) misurato nell'app impacchettata col self-test: 969 ms dall'avvio del processo al primo
  disegno dell'interfaccia. Il self-test completo passa anche in release.
- **NFR-7 e NFR-8 come test sul codice.**
  - Rete: solo il client OpenRouter usa la rete, e solo `stemwise-ai` dipende da una libreria HTTP. Il client si crea
    solo dove l'utente chiede l'assistente. Nessuna telemetria.
  - Webview: nessuna API di rete nelle sorgenti dell'UI, e la CSP limita le connessioni all'app e all'IPC.
  - macOS: compilazione condizionale, framework e FFI stanno solo nel modulo AAC, che ha la stessa interfaccia e un
    ripiego altrove.

### A-047 — Correzioni della revisione finale (2026-11-11, F10)
La revisione indipendente finale ha trovato 2 bloccanti e 6 rilievi. Correzioni:
- **File mancanti mai trattati come silenzio fuori dall'ascolto** (B1).
  - Il render offline (bounce, valutazione del ciclo AI, CLI) e l'analisi rifiutano di lavorare con file mancanti:
    errore `Missing`, nessuna cache d'analisi scritta.
  - L'assistente del mix rifiuta la richiesta e il pulsante Auto mix è disattivato finché il banner della timeline
    segnala file mancanti.
  - Solo il motore dal vivo li suona in silenzio, con le clip a righe.
- **Il titolo non arriva al modello nemmeno nei nomi delle famiglie** (B2): `model_name` si applica anche lì; il test
  usa un gruppo che contiene il titolo.
- **"Locate…" e snapshot** (C2).
  - Il ricollegamento a un file diverso è `AddMedia` più `RelinkMedia { from, to }`, che ripunta soltanto le clip.
    La voce vecchia resta nel progetto: uno snapshot che la nomina la ritrova e la segnala come mancante.
  - `missing_media` segnala anche le clip che nominano un media non elencato; nel menu compaiono come "Unknown
    file", da non ricollegare.
  - `RelinkMedia` è nella strategia del test di proprietà.
- **Nessuna conversione in silenzio** (C3, PRJ-5). Un file a un'altra frequenza si accetta solo se anche il file
  mancante era stato convertito all'import. Un rifiuto rimuove anche le cache del tentativo, e il messaggio dà i frame.
- **Forme d'onda dopo "Salva con nome"** (C4): la copia ricostruisce le sue cache come all'apertura. Le clip in
  attesa mostrano lo stato `is-loading` del design 13c (C6).
- **Locate a un file per volta** (C6, design 13g). Il design mostra un dialogo con ricerca in una cartella e più file
  insieme. Qui c'è un menu dei file mancanti e una scelta per file: l'operazione è la stessa e la verifica di forma e
  contenuto resta file per file. La ricerca per cartella non c'è nella v1.
- **Cache scritte in sicurezza** (N1): i file temporanei hanno un nome unico per processo e chiamata. Anche i file
  convertiti all'import diventano di sola lettura e non sovrascrivono un file uguale già presente (N7).
- **Accessibilità** (N6): le clip si raggiungono con Tab e si selezionano con Invio; axe gira anche sulla proposta in
  revisione e sul dialogo di export.
- **Restano, con motivo:**
  - N2: le somme dell'analisi in `f32` dipendono dall'ordine dei thread al livello di 1e-7. Il documento per il
    modello arrotonda a 0,1 dB o meno, e il render non è toccato.
  - N4: NFR-3 (import) e NFR-5 si verificano col benchmark (`make bench`), come NFR-1 e NFR-4. Il benchmark di memoria
    non copre il ciclo di valutazione AI (tre copie del mix stereo: circa 350 MB in più su 5 minuti, sotto il limite).
  - N8: la regola "rete solo su azione dell'utente" è verificata sul punto di creazione del client, che è uno solo.

### A-048 — Nuovo progetto: file scelti prima che il progetto esista (2026-11-11, dopo F10, segnalato dall'autore)
- **Difetto.** Nella schermata "New project", aggiungere i file dava "no project is open": il comando
  `import_candidates` chiedeva il progetto aperto, che si crea solo con "Create project".
- **Perché i test non l'hanno visto.**
  - Il backend finto di Playwright esaminava i file anche senza un progetto aperto.
  - Il self-test nell'app vera partiva da un pacchetto già creato dalla CLI.
  - Il percorso reale della creazione non era coperto da nessuno dei due.
- **Correzione.** Il comando riceve `adding` e una frequenza di riferimento:
  - per "Add tracks" usa la frequenza del progetto aperto;
  - per un progetto nuovo usa quella del primo file già in elenco, anche fra un'aggiunta e l'altra (PRJ-5), o nessuna.
  Il backend finto ora rifiuta "adding" senza un progetto, come la shell.
- **Copertura.**
  - Il self-test nell'app vera (anche impacchettata) chiude il progetto, elenca i file in due gruppi, crea il
    progetto, importa, poi aggiunge una traccia al progetto aperto.
  - Un test del core verifica il confronto della frequenza tra gruppi.
- **Test dei 60 fps.** È caduto una volta in `make check` (12 frame lenti su 180) perché misurava in parallelo con
  altri tre browser. Ora gira da solo, dopo tutti gli altri (progetto Playwright `perf`); la soglia è invariata.

### A-049 — Correzioni dalla validazione manuale dell'autore (2026-11-11, dopo F10, segnalate dall'autore)

**Contesto.** Durante la prova manuale l'autore ha segnalato otto difetti.

1. Nella creazione del progetto non si potevano aggiungere riferimenti.
2. Il passaggio da Mix a Master chiedeva sempre un nuovo mixdown.
3. Nel master la barra spaziatrice non avviava la riproduzione.
4. I trascinamenti selezionavano il testo dell'interfaccia.
5. Le tracce non si potevano trascinare (riordino, dentro un gruppo) né raggruppare da una selezione.
6. L'icona delle impostazioni sembrava un sole.
7. Mancava un posto per vedere e cambiare le impostazioni del progetto (nome, brief, riferimenti).
8. I meter del bus in alto si allargavano e si stringevano durante la riproduzione.

**Scelte.**

1. **Riferimenti alla creazione.** Si scelgono (clic) o si trascinano sul riquadro dedicato prima che il progetto esista. Vengono aggiunti subito dopo l'import delle tracce, perché sono convertiti alla frequenza del progetto, che fissa il primo file. Per distinguere un rilascio sul riquadro dei riferimenti da quello delle tracce, l'evento di trascinamento dei file porta ora la posizione del puntatore.
2. **Mix → Master.** Se esiste un mixdown si entra sempre nel master già fatto. Se il mixdown non è aggiornato, il master lo dice con il banner esistente e offre di rifarlo conservando le impostazioni (MDN-2). Il dialogo di bounce resta solo per il primo mixdown. In più l'impronta del mix (`package::mix_hash`) ignora nome, tinta e ruolo delle tracce e il nome originale dei file: rinominare una traccia non rende più il mixdown superato.
   - Effetto collaterale: i mixdown dei progetti esistenti risultano superati una volta, per il cambio di formula dell'impronta.
3. **Scorciatoie nel master.** Il master ha il suo gestore per spazio, invio, Home, undo/redo (sulla cronologia del master) e salvataggio. Le altre scorciatoie della tabella riguardano il mix.
4. **Selezione del testo.** Disattivata su tutta l'app (`user-select: none`). Restano selezionabili i campi, il JSON grezzo, il riepilogo e le motivazioni dell'AI e il percorso del progetto.
5. **Tracce.**
   - Le intestazioni accettano ⌘-clic (aggiunge o toglie) e ⇧-clic (intervallo).
   - ⇧⌘G e il menu della traccia raggruppano tutta la selezione.
   - Il trascinamento dell'intestazione sposta la traccia o la selezione. Una linea mostra dove andrà; sul centro dell'intestazione di un gruppo la traccia va dentro.
   - Il core ha una nuova modifica, `Edit::PlaceTracks { tracks, before, output }`: riordino e instradamento in **un solo passo di undo**, con il controllo che un gruppo non finisca dentro se stesso e che un ritorno non entri in un gruppo (vale anche per `add_group`). L'UI non duplica la logica di dominio.
6. **Icona.** Rotella dentata al posto dei raggi, nel generatore del design (`design/tools/lib.mjs`) e in `iconPaths.ts`.
7. **Impostazioni del progetto.** Un dialogo "Project settings" sostituisce il dialogo del brief e si apre dal nome del progetto nella barra in alto, in Mix e in Master. Mostra nome, frequenza, tracce, posizione, brief e riferimenti (aggiunta con clic o trascinamento, rimozione). Il nome si cambia con la nuova `Edit::RenameProject` (`Command::SetName`, un passo di undo); la cartella del pacchetto non cambia. I riferimenti appaiono per nome nella colonna dell'assistente.
8. **Meter.** Ogni valore della barra (true peak, loudness, carico) ha uno spazio fisso: barre e vicini non si spostano più.
   - **Correzione (seconda segnalazione dell'autore).** La prima versione dava alle barre la classe `bars`, che `design/screens/screens.css` usa già per le barre verticali; così i meter orizzontali avevano larghezza zero e sparivano. Il test di allora verificava solo che non si spostassero. Ora la classe è `bus-bars` e i meter stanno in un componente comune (`shared/BusMeters.tsx`), presente anche nella barra del master (uscita del master).
   - **Spazio nel master.** L'etichetta "Listen at the same loudness" diventa "A/B", con la spiegazione nel tooltip. Il carico del motore resta solo nel mix. Sotto i 1360 px il master nasconde loudness ed etichetta del mixdown, che sono già nel pannello di loudness e nella scheda della forma d'onda.
   - **Test.** Ora il test verifica che le barre siano larghe almeno 40 px e si riempiano in riproduzione, in Mix e in Master, a 1440 e a 1280 px, e che la barra in alto non sfori. Rimettendo la classe sbagliata il test fallisce.

**Verifica.**
- Nuovi test: `dragged_tracks_move_in_one_step_into_and_out_of_groups` (core) e il controllo sul rinomina nel test del mixdown (engine).
- Quattro test Playwright nuovi o estesi: riferimenti alla creazione per clic e trascinamento e impostazioni del progetto; tracce trascinate dentro e fuori un gruppo e gruppo da selezione; nessuna selezione di testo e meter fermi; Mix↔Master senza bounce e spazio in entrambe le fasi.
- Nel self-test dell'app vera: rinomina senza mixdown superato e rientro nel master senza bounce, cambio del mix con master che si apre comunque, riferimento e nuovo nome sul progetto appena creato, tracce spostate dentro e fuori un gruppo con riproduzione.

### A-050 — Master pronto in fretta: misure tenute nel progetto, nessun calcolo ripetuto (2026-11-11, dopo F10, segnalato dall'autore)

**Contesto.** Con un progetto reale (13 stem stereo a 96 kHz, 3:04) l'autore vedeva il master impiegare secondi a diventare usabile. Il cambio di fase in sé dura 10–85 ms. A essere lente sono le analisi in background:
- forma d'onda e misure del mixdown: 9 s, 20 s con un riferimento a 44,1 kHz;
- loudness sul target: altri 20 s;
- tutto da rifare a ogni apertura del progetto.

Profilo sul mixdown reale:
- `measure_mix`: 3,1 s, eseguita due volte;
- misure del master: rifacevano true peak (2,1 s), loudness integrata e range che `measure_mix` aveva già;
- una passata della catena del master: 3,8 s; la ricerca della loudness ne fa 2–3, più una passata finale uguale all'ultima;
- conversione del riferimento: 5,5 s ogni volta, contro 0,05 s per rileggere la copia già convertita;
- rilettura del mixdown dopo il bounce: 0,06 s.

**Scelte.**
1. **Misure tenute nel pacchetto** (`cache/master/`): misure del mixdown (chiave: numero, impronta del mix, frame, frequenza), di ogni riferimento (chiave: id del file e frequenza) e gli ultimi 16 risultati della loudness. La chiave della loudness comprende catena, destinazione, mixdown e profili di banda. Sono dati derivati: un file che non corrisponde o non si legge viene ricalcolato, e cancellare la cartella costa solo un'attesa in più. File scritti per rinomina, mai a metà.
2. **Campioni del bounce riusati.** Il bounce passa i suoi campioni alla cache del mixdown appena scritto, senza rileggere il file. Il guadagno è piccolo (lettura di 56 ms), ma l'analisi parte subito, mentre l'autore legge il risultato del bounce.
3. **Copia convertita del riferimento.** Se esiste già in `cache/listen-…wav` si rilegge quella, senza riconvertire. Mixdown e riferimenti si misurano in parallelo.
4. **Loudness.**
   - La ricerca (`loudness_search`) può partire dal guadagno trovato per lo stato precedente. Così salta la passata di misura senza limiter, e una piccola modifica richiede di solito una passata invece di tre.
   - L'ultima passata diventa il risultato, senza ripeterla. Per farlo i guadagni sono arrotondati a 0,01 dB prima di ogni passata, non dopo.
   - Per il primo stato misurato in una sessione si parte da una stima: loudness della destinazione meno quella del mixdown, meno il guadagno d'ingresso.
   - L'export cerca sempre da zero: stesso progetto, stesso file (MAS-8).
5. **Misure.** `measure_mix` calcola spettro, true peak di ogni canale, loudness e range su thread separati, con gli stessi valori di prima. Le misure del master usano quelle di `measure_mix` invece di ricalcolarle.

**Effetto collaterale.** L'arrotondamento spostato cambia di pochi centesimi di dB il guadagno di loudness dell'export rispetto alla versione precedente. Il risultato resta deterministico.

**Dipendenze.** `rayon`, già nel workspace, entra anche nella shell `src-tauri`.

**Complemento.** L'overlay di preparazione del master, disegnato nelle schermate 9d–9k, è ancora da implementare.

### A-051 — Overlay di preparazione del master (2026-11-11, dopo F10, richiesto dall'autore)

**Contesto.** L'autore ha chiesto che, entrando in Master, il lavoro ancora in corso sia coperto da un overlay che spiega cosa sta facendo, e che lo spazio di lavoro compaia solo quando è davvero pronto. Il design è nelle schermate 9d–9k (`design/screens/09-master-preparing.html`, spec in `design/README.md`).

**Scelte.**
- **Contratto.** `MasterStatus.prep: MasterPrep | null` come proposto dal design: numero, durata e frequenza del mixdown, quanti riferimenti, frazione pesata, tempo trascorso, stima del tempo restante, eventuale errore e passi con stato, provenienza ("bounce", "saved"), avanzamento e durata. In più `skipped_references` per il banner 9i.
- **Passi.** Il piano dei passi nasce in `Prep::plan` dal pacchetto: quello che è già salvato risulta fatto subito e pesa zero, e una conversione non necessaria non viene elencata.
  - `load_cache` riporta ogni passo man mano.
  - La ricerca della loudness riporta l'avanzamento dopo ogni passata.
  - I pesi sono stimati sui tempi misurati per A-050.
- **Errori.**
  - Un mixdown illeggibile (file mancante, troncato, non leggibile) ferma la preparazione con 9j e "Bounce again".
  - Un errore interno offre "Try again" (`master_prepare_retry`).
  - Una preparazione fallita non riparte da sola a ogni modifica.
  - Un riferimento illeggibile viene saltato (9h) e nominato nel banner 9i.
- **Interfaccia.**
  - Overlay dopo 250 ms e per almeno 600 ms (`--wait-show`, `--wait-min`), sopra i gusci vuoti dello spazio di lavoro.
  - Nella barra in alto solo il selettore di fase resta attivo. Spazio, undo e gli altri tasti non fanno nulla; Esc e "Back to Mix" tornano al Mix senza fermare il lavoro.
  - Nel Mix il segmento Master mostra un anello di avanzamento.
  - Il toast "Master is ready" (oppure l'errore con "Show") ha un'azione e resta finché non si chiude. Le notifiche ora accettano un'azione.
  - Dopo una modifica del master nessun overlay, ma lo stato 9k nella scheda Loudness. In quel momento "Master with AI" e il pulsante di export aspettano la misura.
- **Test.** Tre test Playwright (overlay, Esc e anello, toast; mixdown illeggibile e riferimento saltato; stato 9k) sul backend finto, che ora simula la preparazione con passi temporizzati. Nel self-test dell'app vera c'è un controllo che la preparazione riporti i suoi passi e finisca.

### A-052 — Portachiavi letto una volta, solo quando serve la chiave (2026-11-11, dopo F10, segnalato dall'autore)

**Contesto.** All'avvio l'app chiedeva la password del portachiavi quattro volte. Ogni stato dell'assistente mandato all'interfaccia (avvio, apertura di un progetto, aggiornamenti) leggeva la chiave dal portachiavi solo per sapere se c'era (`has_key`). Con l'app non firmata macOS chiede la password a ogni lettura, e un "Consenti sempre" vale solo per quella build: la firma ad hoc cambia a ogni build.

**Scelta.** `LazyKeyStore` in `stemwise-ai::keys` avvolge il portachiavi di sistema.
- La presenza della chiave la dice un piccolo file `has-key.json` (`true` o `false`, nessun segreto) nella cartella dati dell'app, aggiornato quando la chiave si salva o si toglie.
- Il valore si legge solo quando parte una chiamata al provider, al massimo una volta per sessione, poi resta in memoria.
- Per una chiave salvata da una versione precedente (file assente) la prima richiesta di stato legge una volta e scrive il file.
- Risultato: avvio, apertura dei progetti e stato dell'assistente non toccano più il portachiavi. La password può essere chiesta al massimo una volta per sessione, alla prima chiamata all'AI col provider vero; con la demo offline mai.

**Test.** `the_keychain_is_read_at_most_once_and_never_to_know_whether_a_key_is_saved` conta le letture su un portachiavi finto.

**Non verificato.** Non è verificato con il portachiavi vero (l'agente non usa chiavi): resta in VALIDATION 7.

**Riserva.** La richiesta all'avvio della prima chiamata sparisce del tutto solo con la firma Developer ID (A-046).

### A-053 — Salvare, salvare con nome e chiudere dall'interfaccia (2026-11-11, dopo F10, segnalato dall'autore)

**Contesto.** L'autore non trovava come salvare il progetto. L'interfaccia salvava solo con ⌘S e con l'autosave, senza nessun comando visibile. "Salva con nome" (PRJ-2) e la chiusura del progetto esistevano nel backend ma non nell'interfaccia. L'app non aveva una barra dei menu propria.

**Scelte.**
- **Menu del progetto.** Il nome del progetto nella barra in alto, in Mix e in Master, apre un menu: "Project settings…", "Save" (⌘S), "Save as…" (⇧⌘S), "Close project".
- **Barra dei menu di macOS.** Menu Stemwise (Informazioni, Impostazioni ⌘,, Esci), File (New Project… ⌘N, Open… ⌘O, Close Project, Save ⌘S, Save As… ⇧⌘S), Edit per i campi di testo, Window. I comandi del File arrivano all'interfaccia (evento `menu`) e passano per gli stessi comandi del menu del progetto (`project/commands.ts`). I tasti arrivano prima alla pagina, quindi le scorciatoie dell'interfaccia funzionano come prima.
- **Chiusura.** Con modifiche non salvate, chiudere o aprire un altro progetto chiede "Save changes to …?" con Save, Don't save e Cancel. Il dialogo di conferma accetta ora una terza scelta.
- **Salva con nome.** Chiede cartella e nome con il pannello di salvataggio di sistema. La copia diventa il progetto aperto e l'originale resta com'era.
- **Scorciatoie.** ⇧⌘S è "Salva con nome", come nelle app per Mac; lo snapshot passa da ⇧⌘S a ⌥⌘S. La tabella unica delle scorciatoie (UI-3) e le Impostazioni lo mostrano.

**Verifica.**
- Un test Playwright: salvataggio dal menu, ⇧⌘S, chiusura con le tre scelte, Open e New dalla barra dei menu simulata.
- Nel self-test dell'app vera, "Salva con nome".
- La barra dei menu nativa non è verificata da un test: l'app che parte nel self-test la costruisce, ma i clic sui menu restano da provare a mano (VALIDATION).

### A-054 — Scrub della testina in Mix e in Master (2026-11-11, dopo F10, richiesto dall'autore)

**Contesto.** La testina non si poteva trascinare, né nel Mix né nel Master. Nel Mix trascinare sul righello crea già la regione di loop.

**Scelte** (concordate con l'autore, come in Logic e strumenti simili).
- **Mix.**
  - Lo scrub si fa afferrando la testina dentro il righello (entro 7 px), oppure con ⌥ e trascinamento in qualunque punto del righello.
  - Un clic altrove sul righello sposta la posizione, un trascinamento crea il loop, come prima.
  - Sotto il righello la linea della testina lascia passare i clic a clip e corsie e non la sposta.
- **Master.** Trascinare sulla forma d'onda fa scrub; un clic sposta la posizione.
- **Comportamento.** La testina segue il puntatore subito. La posizione arriva al motore al massimo ogni 33 ms, più l'ultima al rilascio. In riproduzione si sente da dove si trascina e si riprende dall'ultima posizione; a trasporto fermo si sposta solo la posizione.
- **Overlay del master.** Mentre sfuma lascia passare i clic: prima li bloccava per qualche centinaio di millisecondi dopo che il master era già pronto.

**Verifica.**
- Due test Playwright: Mix (presa nel righello, linea che lascia passare i clic, ⌥ e trascinamento) e Master (forma d'onda).
- Nel self-test dell'app vera, 20 spostamenti a 30 Hz in riproduzione: la riproduzione continua dall'ultimo, con segnale sul bus.

### A-055 — Ricerca della loudness che arriva anche sulle destinazioni forti (2026-11-11, dopo F10, segnalato dall'autore)

**Contesto.**
- Sul progetto reale, con la destinazione Loud (−9 LUFS, −1 dBTP), il master si fermava a −9,4 LUFS.
- La ricerca correggeva il guadagno come se ogni dB di guadagno desse un dB di loudness. Sotto una limitazione forte ne dà molto meno, e due passate correttive non bastavano.
- Il test tollerava 0,5 LU proprio sulle destinazioni forti, e così nascondeva il difetto.

**Scelta.**
- Dopo la prima correzione, la successiva usa la pendenza misurata tra le ultime due passate. La pendenza è limitata tra 0,25 e 1, quindi un passo non supera mai quattro volte l'errore.
- La ricerca si ferma entro 0,1 LU (`ON_TARGET_LU`) o dopo sei passate in tutto (`SEARCH_PASSES`).
- Una destinazione irraggiungibile entro ±24 dB di guadagno resta "Off target". Sul mixdown reale −7 LUFS si ferma a −7,8.
- Il risultato resta deterministico (MAS-8).

**Verifica.**
- Il test ora chiede 0,1 LU su −14, −9, −10, −8 e −7 LUFS.
- Sul mixdown reale di 13 tracce: −9,06 LUFS con la destinazione Loud (prima −9,4), −14,03 con Streaming.

### A-056 — Ascolto al livello reale e avviso sulla destinazione (2026-11-11, dopo F10, chiesto dall'autore)

**Contesto.**
- Il Master suonava sempre a pari loudness (MAS-5), quindi il livello vero del master non si sentiva mai.
- Con un master forte, anche il master stesso suonava abbassato per evitare il clip delle altre sorgenti (`ab_trim_db`).
- Una destinazione Custom a −18 LUFS contro un riferimento a −9,6 LUFS è passata inosservata fino all'ascolto del file esportato.
- Il design è nelle tavole 9l–9u e 10d–10e, decisioni D26–D31 di `design/README.md`.

**Scelte.**
- **Ascolto.**
  - "Real level" è il modo predefinito: il master suona come il file esportato, mixdown e riferimenti al loro livello, senza guadagno né abbassamento.
  - "Same loudness" è il comportamento di MAS-5 di prima, a richiesta (⇧B). La proposta di master in revisione segue lo stesso modo: a pari loudness suona alla loudness del master corrente.
  - Il modo vive nella sessione (`MasterStatus.listen_same`, comando `master_listen_same`). Non è nel progetto e non è un passo di undo: ogni progetto si apre al livello reale, e il modo resta passando dal Mix.
- **Avviso sulla destinazione.**
  - Si confronta la loudness della destinazione con l'intervallo dei riferimenti misurati.
  - Oltre 3 LU sotto il più basso c'è un avviso; oltre 3 LU sopra il più alto una nota neutra.
  - "Match" porta la destinazione al bordo più vicino come Custom, con lo stesso tetto, in un solo passo di undo.
  - "Keep" è `MasterState.target_kept` (con `serde(default)`, quindi i progetti esistenti si aprono senza migrazione), anch'esso un passo di undo.
  - Il conto e le parole stanno in `ui/src/master/level.ts`. Sono funzioni pure con test unitari, perché servono anche per destinazioni non ancora scelte (il menu) e per i file del rapporto. Il core espone già le loudness che servono (`references[].lufs`, ora con `true_peak_db`).
- **Meter della scheda Loudness.** Misurano quello che suona (il bus d'uscita), come prima. Al livello reale, il modo predefinito, coincidono con il file. Il design proponeva di misurare prima del guadagno d'ascolto, ma questo richiederebbe un secondo punto di misura nel motore per un caso, la pari loudness, che i numeri di quel modo già spiegano.
- **Barra del Master.** Le tavole non hanno i meter d'uscita, aggiunti dopo il design su richiesta dell'autore (A-049). Con i nuovi controlli la barra sbordava di 165 px già a 1440, quindi nel Master:
  - i meter tengono le barre e il true peak, ma non la loudness, che è grande nella scheda Loudness;
  - spazi e padding dei segmenti sono più stretti;
  - sotto i 1600 px si nascondono la durata totale accanto alla posizione e l'etichetta "Reviewing a master proposal" (che è già nella colonna Assistant);
  - nella finestra stretta (compact) le etichette diventano "Real", "Same", "Ref A".

  Il test della barra verifica che stia in 1440 e 1280 px, anche con due riferimenti.
- I preset del menu delle destinazioni ora usano il segno meno vero (−14), come il resto della UI.

**Verifica.**
- Test Playwright:
  - ascolto (9l–9n, ⇧B, il modo che resta tra le fasi);
  - avviso (9q–9t, menu, Keep con undo, Warn me again, export e rapporto 10d–10e, Match);
  - nota sulla destinazione più forte (9u);
  - A/B della proposta con B e ⇧B (9c, 9o).
- Test unitari della logica in `level.test.ts`.
- Self-test dell'app vera: lo stesso passaggio del mixdown, al livello reale e poi a pari loudness, cambia della differenza attesa.

### A-057 — Il focus resta dove l'ha messo l'utente quando l'overlay del master sparisce (2026-11-11, dopo F10)

**Contesto.**
- Il test dell'export è fallito due volte dentro `make check`: l'ISRC appena scritto spariva dal campo.
- Causa: quando l'overlay di preparazione finiva di sfumare, il focus veniva riportato al segmento Master (9d) in ogni caso.
- Mentre sfuma, l'overlay lascia passare i clic (A-054). Chi apriva subito l'export e scriveva perdeva il focus a metà: i tasti finivano sul segmento e il campo restava vuoto.
- Il caso si presentava nell'app vera come nel test sotto carico.

**Scelta.** Il focus torna al segmento Master solo se era rimasto sull'overlay, cioè se ora non è su nessun elemento (il corpo della pagina o un elemento rimosso).

**Verifica.** Un test Playwright apre Export e scrive nel titolo mentre l'overlay sfuma: il campo resta a fuoco e tiene il testo. Prima della correzione falliva ogni volta.

### A-058 — F11: riverbero in tre algoritmi e mix del compressore (2026-11-11, P-011)

**Contesto.** FOUNDATION chiede tre algoritmi di riverbero al posto di quello a pettini (FX-5, FX-9) e il dry/wet del
compressore (FX-3). Scelte prese costruendoli.

**Scelte.**

- **L'algoritmo è fissato alla costruzione del riverbero.** Cambiarlo è una modifica strutturale (`Edit::SetReverbAlgorithm`,
  il grafo si ricostruisce con la dissolvenza di A-024): nessun buffer da azzerare nel thread audio. I parametri restano
  quelli che erano.
- **Rete a 16 linee con matrice di Hadamard**, non 8: più densità di echi e di modi allo stesso costo per campione di
  una moltiplicazione di matrice (trasformata veloce). Schemi di segno *bent* in ingresso e in uscita: con una riga
  della matrice come schema di uscita una linea tornava intera (picco 0,2 nell'autocorrelazione).
- **Plate: prese di uscita spostate fuori dagli allpass del tank.** È l'unico scostamento dall'articolo di Dattorro:
  l'interno di un allpass è un pettine e il suo periodo si sentiva nella coda.
- **Il plate si restringe con i decadimenti corti** (almeno cinque giri del tank entro il decadimento): sotto circa 3 s
  il decadimento decide la dimensione e `size` è solo un tetto. Sotto circa 1 s il plate resta granuloso per
  costruzione: lo dicono il pannello e il prompt, che indirizzano a `room` per gli spazi corti.
- **Misura della densità (FX-9).** "Nessun picco isolato" è reso così: l'autocorrelazione della coda appiattita non
  deve tornare a un ritardo *e* al suo doppio (2–150 ms), che è l'impronta di un pettine. Il solo picco massimo non
  distingueva: anche un plate ben fatto ha una ripetizione singola tra due prese della stessa linea. Soglia 0,11: il
  vecchio riverbero misura 0,14–0,20, i nuovi 0,04–0,10. Il plate è misurato da 2 s in su.
- **Tempo di decadimento.** ±10% senza damping e senza modulazione a 0,5, 1,5 e 4 s; ±15% ai valori di default. Sotto
  0,5 s hall e plate sono fino all'11% lunghi: resta nelle note di `dsp.md`.
- **Mix del compressore sul guadagno**, non su due percorsi: `1 + mix·(g − 1)`. Dry e compresso sono lo stesso
  campione, quindi niente filtro a pettine. Il tipo `Compressor` è condiviso con il master, dove `mix` resta a 1 e
  non è esposto (FOUNDATION: il compressore del master non lo ha).
- **Contratto dell'AI già a `mix_plan` v3 per riverbero e mix.** Togliendo `room_size` dal documento il crate dell'AI
  non compilava più: i campi nuovi del riverbero (`algorithm`, `decay_s`, `size`, `modulation`, `return_lpf_hz`) e
  `compressor.mix` entrano ora, con limiti, validazione, prompt e decisore finto (che per ora lascia `mix` a 1 e
  l'algoritmo com'è). `structure`, i doppi e le tecniche restano a F12.
- **Migrazione (PRJ-8).** Schema 2: il vecchio riverbero diventa `room` con il decadimento che dava il suo
  `room_size` (retroazione `0,28·room_size + 0,7` su linee di circa 30 ms); i compressori ricevono `mix` 1. La stessa
  funzione aggiorna i file degli snapshot, che non hanno versione. Il render non è identico (P-011).
- **Nuova unità `s`** nelle specifiche dei parametri, per il decadimento.
- **Interfaccia.** Tavole 7a (selettore Plate/Room/Hall, Decay, Size, Pre-delay, Damping, Modulation, filtri) e 7e
  (compressione parallela) disegnate in `design/` prima dell'implementazione. Il campo Mix sta nella riga del Knee;
  il testo accanto dice "Parallel" sotto il 100%. "Reverb return" nel menu Add crea un hall: l'algoritmo si sceglie
  nel pannello.

**Verifica.** 12 test sul riverbero e 2 sul mix nel crate DSP, test di allocazione sui tre algoritmi, test del motore
(stesso hash con blocchi da 64, 1000 e 4096; mix 0 uguale al compressore escluso entro 0,1 dB), migrazione, un test
Playwright (algoritmo e mix, ciascuno un passo di undo), `make bench`: NFR-1 p99 17,9% con quattro riverberi in più.

### A-059 — F12: mix AI strutturale (2026-11-11, P-011)

**Contesto.** FOUNDATION chiede che il piano possa creare ritorni e gruppi (AI-17 → AI-20), la misura dei doppi veri
(ANA-18) e le tecniche di mix nel prompt e nel decisore finto (AI-21). Scelte prese costruendoli.

**Scelte.**

- **La struttura diventa reale una volta sola, nella copia di lavoro.** Dopo la prima iterazione ciò che il piano crea
  viene inserito nel progetto di lavoro (`structure::materialise`) e tolto dal piano. Da lì in poi sono ritorni e
  gruppi come gli altri: le iterazioni successive li trovano nello schema e nell'analisi senza `structure` (AI-19), il
  piano si rende e si valuta con loro, e l'ascolto in A/B li comprende. La validazione esistente non è cambiata: lavora
  su un progetto che li contiene.
- **Segnaposto fissi**: `n1`–`n3` ritorni, `n4`–`n6` gruppi, tutti presenti nello schema strict e annullabili. Lo schema
  strict vuole ogni proprietà elencata e obbligatoria, quindi non può esserci un numero libero di elementi.
- **Id degli elementi nuovi fuori dal contatore del progetto** (prefisso più l'istante della chiamata): il progetto può
  cambiare durante la revisione senza che un id coincida con una traccia aggiunta dall'autore.
- **All'applicazione la struttura si ricrea sul progetto com'è in quel momento** (`structure::commands`), non
  ripetendo comandi salvati: un comando salvato porterebbe con sé lo stato vecchio di una traccia. Se un membro è
  sparito o è stato instradato altrove, il gruppo non viene creato (`NoLongerFits`) e il resto si applica.
- **Spunta propria sugli elementi nuovi** (AI-20, aggiornato): `aiApply(only)` riceve anche i loro id. Per ciascuno
  il core espone le tracce da cui dipende e quante ne servono (un mittente per un ritorno, due membri per un gruppo):
  l'interfaccia mostra "not created" con lo stesso criterio con cui il core decide. Una mandata verso un ritorno non
  creato cade; le altre modifiche della traccia restano.
- **Niente struttura nelle richieste limitate ad alcune tracce** (AI-18, precisato in FOUNDATION): i ritorni erano già
  fuori da un ambito (A-034) e un gruppo cambia il percorso dei suoi membri.
- **Ciò che non si può creare cade con un avviso, non fa fallire il piano**: ritorno senza mandate, gruppo con meno di
  due membri validi, membro bloccato o con un'altra uscita, troppi ritorni, annidamento oltre i quattro livelli, tipo
  dei parametri diverso da quello dichiarato (valgono i default). È un errore, rimandato al modello, solo un gruppo
  creato senza la sua catena in `groups`, come per ogni gruppo esistente.
- **Doppi veri (ANA-18): confronto degli spettri per frame, non correlazione della forma d'onda.** La cache
  dell'analisi tiene la potenza per banda di ogni frame, non i campioni: il confronto frame per frame della forma
  spettrale distingue due esecuzioni (alcuni dB) da due copie (frazione di dB) senza rileggere l'audio, e riconosce
  come copia anche un "finto doppio" ritardato di pochi millisecondi, che la correlazione a ritardo zero non vedrebbe.
  FOUNDATION è aggiornato di conseguenza. Soglie iniziali: sovrapposizione ≥ 0,7, correlazione degli inviluppi ≥ 0,6,
  differenza di profilo ≤ 3 dB; copia sotto 0,5 dB per frame con inviluppi correlati oltre 0,98. Ogni traccia sta in
  una coppia al più. Da tarare su materiale vero (VALIDATION 28).
- **Decisore finto (AI-21)**, regole deterministiche:
  - doppio non `identical` → pan ±0,7 e stesso livello;
  - voce solista → ritorno "Vocal plate" (plate, 1,2 s) al posto del riverbero condiviso, e ritorno "Slap" (delay
    110 ms, feedback 0); se esistono già con quel nome li riusa, così un secondo auto mix non li ricrea;
  - voce solista nei ritornelli → mandata al delay +6 dB (throw);
  - batteria su almeno due tracce che vanno dritte al bus di mix → gruppo "Drums" con compressione parallela (mix
    35%); un gruppo di batteria già esistente riceve la stessa compressione.
- **Prompt**: sezione "New returns and groups" solo quando la struttura è permessa; sezione "Techniques" sempre (doppi,
  slapback, throw, compressione parallela, spazi distinti), ciascuna con il caso in cui evitarla.
- **Demo della riga di comando** più ricca (rullante, una chitarra suonata due volte, due sezioni), perché il flusso
  completo eserciti tutte le tecniche.

**Verifica.** Test di ANA-18 su segnali sintetici (due esecuzioni, una copia, altro ritmo, altro colore); quattro test
di flusso con decisore finto e decisore "spia" (struttura creata una volta e stabile tra iterazioni, applicazione in un
passo e undo, applicazione parziale nei cinque casi, interruttore, ambito, lock, avvisi e nuovo tentativo, revisione con
struttura); flusso completo da `stemwise-cli`; un test Playwright sulla proposta (tavole 4a e 4b) e sull'interruttore.

### A-060 — Correzioni della revisione indipendente di F11 e F12 (2026-11-11)

**Contesto.** Revisione di `f10..HEAD` con `app-reviewer`: 2 bloccanti, 4 rilievi da correggere, 3 note.

**Bloccanti, corretti.**

- **Un ritorno nuovo alimentato solo da un gruppo nuovo non veniva mai creato**, nemmeno con "Apply all": i ritorni
  erano creati prima dei gruppi, quando il loro unico mittente non esisteva ancora. Ora gruppi prima dei ritorni, sia
  nella copia di lavoro sia all'applicazione; l'elenco resta nell'ordine della proposta.
- **Un sidechain verso un gruppo nuovo non creato faceva fallire tutta l'applicazione**: la sorgente inesistente
  rendeva il routing non valido. Ora una sorgente che non c'è nel progetto cade (il compressore ascolta la propria
  traccia) e il resto si applica; vale anche per una traccia eliminata durante la revisione, difetto precedente a F12.

**Da correggere, corretti.**

- **Ritorno nuovo rimasto senza mittenti** (iterazione successiva che toglie le mandate, o unico mittente bloccato):
  prima della proposta viene tolto da progetto di lavoro, piano ed elenco, con un avviso (`structure::prune`). I comandi
  della proposta sono costruiti sul progetto con le creazioni già applicate, come farà poi l'applicazione.
- **"Non entra più" visibile nell'interfaccia.** Ogni elemento nuovo porta `fits`, calcolato sul progetto com'è ora;
  la proposta si aggiorna a ogni modifica del progetto durante la revisione e dice "The project changed since this
  proposal: it no longer fits".
- **Test che non provavano ciò che citavano.** Il test da riga di comando ora verifica dal progetto salvato ogni
  tecnica di AI-21 e che il doppio sia trovato e non `identical`; aggiunti un test sulla copia ritardata di 12 ms
  (ANA-18), uno sui casi bloccanti e sul ritorno senza mittenti, e la densità di `room` a 0,6 s (FX-9).
- **Divergenze non registrate.** I doppi richiedono anche lo stesso numero di canali (una traccia mono e una stereo
  non sono due esecuzioni dello stesso microfono; resta così). Il decisore finto ora dà ai due doppi anche la stessa
  elaborazione (filtri, EQ, compressore, saturazione), come chiede AI-21.

**Dal test sulla copia ritardata.** La regola per `identical` chiedeva anche inviluppi correlati oltre 0,98: una copia
in ritardo di 12 ms ha lo stesso colore in ogni frame (0,13 dB) ma un inviluppo spostato (0,91), quindi non veniva
riconosciuta. Ora conta solo la differenza per frame, sotto 0,5 dB. Due esecuzioni sintetiche con lo stesso inviluppo
misurano 0,8 dB: il margine è stretto e la soglia va tarata su materiale vero (VALIDATION 28).

**Note.** Un mittente bloccato durante la revisione non conta più come utente di un ritorno nuovo. La spunta "Allow
structural changes" non ricostruisce più il grafo audio.

### A-061 — Una nuova destinazione aspetta sotto un overlay (2026-11-11, dopo F12, chiesto dall'autore)

**Contesto.**
- Dopo un cambio della loudness o del tetto della destinazione, la scheda Loudness mostrava solo `Measuring…` e la
  riga "Your change is already playing" (stato 9k, A-051). L'autore ha continuato a lavorare senza accorgersi che il
  cambio non era ancora avvenuto.
- La riga era anche falsa per questo caso: una modifica della catena si sente subito, ma il livello di una nuova
  destinazione arriva solo quando la ricerca del guadagno è finita (fino a sei passate sull'intero master, A-055).

**Scelta** (tavola 9v, spec in `design/README.md`).
- **Solo la destinazione blocca.** Le altre modifiche del master restano in linea come in 9k: coprire lo spazio di
  lavoro a ogni knob romperebbe il ciclo "tocca e ascolta".
- **Contratto.** `MasterStatus.retarget` (`from_lufs`, `from_true_peak_db`, `fraction`, `elapsed_ms`), presente
  mentre la destinazione del progetto è diversa da quella per cui sono state fatte le misure a schermo. Lo decide
  il backend, quindi vale per ogni strada: preset, valori Custom, "Match the reference", undo e redo, una proposta
  di master applicata.
- **Overlay.** Lo stesso pannello della preparazione, sopra lo spazio di lavoro com'è: destinazione nuova e
  precedente, barra, due passi (ricerca del guadagno, misura del risultato), nota su cosa si sente intanto.
  Compare dopo 250 ms: una destinazione già misurata (tra gli ultimi 16 risultati tenuti nel progetto) torna
  subito, senza overlay.
- **Cosa resta attivo.** Selettore di fase e trasporto: suona il master al livello di prima, e fermarlo deve
  restare possibile. Undo, A/B, Export e gli altri tasti aspettano. `Esc` e "Back to Mix" tornano al Mix senza
  fermare il lavoro, con l'anello sul segmento Master.
- **Nessun "Cancel".** A lavoro finito il cambio è un passo di undo, e tornare alla destinazione di prima è
  immediato perché è già misurata.
- **Trascinamento.** Durante il trascinamento di un valore Custom l'overlay non compare; parte al rilascio.
- **Dialogo di export.** Se il cambio avviene lì ("Match the reference", 10d) l'overlay non compare: aspetta il
  pulsante primario, come prima.
- **Avanzamento.** La frazione conta le passate su sei possibili, quindi una ricerca che converge in due salta
  dal 28 % all'85 %. Nessuna stima del tempo restante: il numero di passate non si conosce prima.

**Verifica.**
- Un test Playwright sul backend finto: overlay con i testi, valori vecchi dietro, trasporto attivo, undo fermo,
  `Esc` con l'anello, fine con il livello nuovo, ritorno immediato con undo.
- Nel self-test dell'app vera: un cambio di destinazione è riportato come attesa fino al livello nuovo, e
  l'undo torna alla misura precedente.
- Il backend finto ora aggiorna il master anche dopo undo e redo, come l'app.

### A-062 — Risposta troncata del provider: tetto esplicito, nuovo tentativo, log completo (2026-11-13, dalla prova dell'autore)

**Contesto.**
- Su un progetto di 30 stem il secondo mix AI falliva sempre con "the provider refused the request (200): the answer
  was cut off at the model's output limit". Le due chiamate fallite sono durate 22 e 48 s, con zero token riportati,
  e non compaiono nell'Activity di OpenRouter; il primo giro aveva scritto circa 8.800 token per iterazione.
- La causa non è dimostrabile a posteriori: di una chiamata fallita il log teneva solo i messaggi e l'errore, non il
  corpo della richiesta (schema compreso) né la risposta. La richiesta inoltre non portava `max_tokens`, quindi il
  tetto era quello scelto dal provider. Il messaggio parlava di rifiuto, che non è ciò che era successo.

**Alternative.** (a) Solo il messaggio. (b) Tetto fisso nel codice. (c) Tetto nella configurazione, nuovo tentativo
in JSON mode e log completo.

**Scelta: (c).**
- **Log (AI-10).** Di ogni tentativo fallito `request-<it>-<n>.json` contiene messaggi, errore e ogni scambio col
  provider (richiesta inviata, stato, risposta); `response-<it>-<n>.json` contiene l'ultima risposta grezza. La
  chiave viaggia solo nell'header e non entra nel log.
- **Tetto.** `AiConfig.max_output_tokens` (32.000, ragionamento compreso), inviato come `max_tokens`; 0 non invia
  nulla. Una configurazione salvata prima prende il valore predefinito. Se il provider rifiuta il tetto (400 che lo
  nomina) la sessione prosegue senza. Non è esposto nelle Impostazioni: si cambia in `ai-config.json`.
- **Nuovo tentativo.** Una risposta con `finish_reason: "length"` sotto schema strict si ritenta una volta in JSON
  mode, solo per quella chiamata (un modello può bloccarsi sotto uno schema strict fino al taglio); il piano resta
  validato da noi. I token della risposta troncata si sommano al costo della chiamata (AI-15).
- **Messaggio.** Errore proprio (`Truncated`) con i token scritti e il tetto inviato; gli altri errori del provider
  dicono "answered with an error", non "refused".

**Motivo.** Senza la risposta salvata un guasto del provider non si può diagnosticare; il tetto esplicito toglie una
variabile; il nuovo tentativo copre il caso più probabile senza indebolire la validazione.

**Riserva.** La causa del guasto osservato resta ignota e la correzione non è provata col provider vero: se si
ripresenta, il log della chiamata ora contiene la risposta (VALIDATION 7–11).

### A-063 — Bilanciamento ed effetti del mix AI, sezioni oltre i 120 s, un solo pulsante (2026-11-13, P-012)

**Contesto** (analisi delle chiamate di Balance, 30 stem, e di Confidence).
- Col piano applicato, misurando gli stem con trim e fader: batteria −18,6 LUFS, voce −20,3, chitarre −20,9, cori
  −29,0. Riverbero della voce circa 20 dB sotto il segnale, slapback circa 30 dB sotto.
- L'unico obiettivo misurato del ciclo era la distanza spettrale dal riferimento. Il mix grezzo aveva 4 dB in più a
  250–500 Hz e 4 dB in meno sotto i 120 Hz: per avvicinarsi il modello tagliava 330 Hz su tutte le chitarre e le
  abbassava. A 1,1 dB il ciclo si fermava.
- Il modello decide un fader per traccia su trim normalizzati a −23 LUFS: quindici microfoni di batteria sommati
  superano una voce sola, e nessun numero glielo diceva.
- Il prompt frenava gli effetti ("sparingly", "at a low level", "create nothing unless it clearly helps").
- **Difetto.** `r1`, l'arrotondamento dei livelli (limite ±120), era usato anche per i secondi: durata del brano e
  confini delle sezioni si fermavano a 120 s. In Balance (185 s) Solo e ultimo Chorus arrivavano a durata zero, e il
  modello lo scriveva nella sintesi.
- **Difetto.** Il pulsante "Auto mix" ignorava il testo scritto nel campo: la richiesta partiva solo con Invio o
  con la freccia. Nel log di Balance non c'è nessuna revisione del mix.
- Su Confidence, "i cori al pari della lead" ha prodotto +4 dB esatti (da −7 a −3 contro +1): il limite DEC-5.

**Scelte.**
- **Bilanciamento misurato (AI-22, `balance.rs`).** Per ogni famiglia (per ruolo) un render in solo coi ritorni
  muti, per ogni ritorno un render da solo, più il mix: loudness integrata e per sezione, riportata come differenza
  dal mix in LU. Tre render alla volta. Prima della decisione (`balance_at_unity` per l'auto mix, `balance` del
  piano di partenza per la revisione) e nella valutazione di ogni iterazione.
- **Ciclo.** Un auto mix non si ferma alla prima iterazione. Il piano tenuto è l'ultimo valido, salvo che sia più
  lontano dal riferimento di oltre 1,0 dB (`SPECTRAL_TOLERANCE_DB`) rispetto a uno precedente; senza riferimenti
  l'ultimo valido. Alternativa scartata: un punteggio di bilanciamento nel ciclo, perché metterebbe un gusto fisso
  sopra il brief.
- **Prompt.** Sezioni nuove "Balance" (rapporti di partenza tra famiglie), "Tone and the target" (il riferimento è
  tono, non bilanciamento; è masterizzato) e "Space" (spazio udibile, soglie su `returns_lu`); tolta la prudenza
  sugli effetti e sulle sezioni.
- **Revisione senza limite di passo.** `limit_changes` vale solo tra le iterazioni di un auto mix.
- **Secondi.** `secs1` per durata e confini delle sezioni; `r1` resta per i livelli.
- **Un solo pulsante** (chiesto dall'autore): tolta la freccia, Invio va a capo, il pulsante dice "Auto mix" senza
  testo e "Revise mix" col testo, con una riga che spiega la differenza. Come già nel Master. Si discosta dalla
  tavola 3 del design, che mostra la freccia.

**Costi.** Un auto mix senza riferimenti passa da una a due chiamate al modello. Ogni valutazione aggiunge un render
per famiglia e per ritorno (una decina su Balance).

**Riserve.** I rapporti di partenza e le soglie dei ritorni sono regole pratiche, non misure: da tarare all'ascolto
(VALIDATION 31). L'effetto sul modello vero non è provato dall'agente. Il decisore finto non usa `balance`.

### A-064 — Forme d'onda ricalcolate a ogni aggiornamento della testina (2026-11-13, dalla prova dell'autore)

**Contesto.** Durante la riproduzione di Balance (33 tracce, 185 s) l'editor andava a scatti. La timeline si
ridisegna a ogni aggiornamento della testina e ogni clip ricostruiva la stringa del tracciato della sua forma d'onda
(fino a 2048 colonne, due `toFixed` ciascuna): il campionamento della vista web mostrava il thread principale sempre
occupato in `toFixed`, `join` e garbage collection. Il test dei 60 fps (NFR-3) non lo vedeva: usa clip strette.

**Scelta.** Il tracciato si calcola solo quando cambiano i picchi o l'altezza (`useMemo`) e il componente `Waveform`
è saltato a parità di proprietà (`memo`). Dopo la correzione il thread principale è occupato per circa il 22% durante
la riproduzione dello stesso progetto, nell'app in sviluppo.

**Non fatto.** La timeline continua a ridisegnare tutte le righe a ogni aggiornamento: separare la testina dal resto
ridurrebbe ancora il carico, ma tocca i gesti di trascinamento e va fatto a parte.

### A-065 — Testina fuori dal ridisegno della timeline; Sends in una sezione propria (2026-11-13, chiesto dall'autore)

**Testina (NFR-3, seguito di A-064).** La timeline leggeva la posizione di riproduzione come stato e si ridisegnava
per intero, righe e clip comprese, a ogni aggiornamento. Ora la posizione si legge dallo store quando un gesto ne ha
bisogno (snap, taglio alla testina, incolla, presa sul righello, menu della clip); la seguono solo la linea della
testina (componente `Playhead`), il valore corrente di ogni corsia di automazione (selettore che cambia solo col
valore) e lo scorrimento automatico (sottoscrizione allo store, senza render).

**Sends.** Nell'inspector le mandate stavano accanto alla saturazione nella stessa sezione: le manopole si
confondevano. Ora la saturazione ha la sua riga e le mandate una sezione sotto, separata da una linea, con la dicitura
"post-fader · to the returns": non sono un blocco della catena, ne escono dopo il fader. Si discosta dalla tavola 7
del design, che le affianca.

### A-066 — Stato del mix a colpo d'occhio: scelte del design (2026-11-13, P-013, chiesto dall'autore)

**Contesto.** Su un progetto di 30 stem l'autore non riusciva a capire quali tracce avessero effetti e mandate senza
selezionarle una alla volta. Nel codice: l'intestazione mostrava nome, ruolo, mute/solo/lock e mini fader; la strip
due barrette di mandata senza nome, solo per i primi due ritorni (`sendTargets.slice(0, 2)` in `Mixer.tsx`); la corsia
di un ritorno il solo numero di chi manda. Tutto il resto stava nell'inspector.

**Scelte.**

- **Sigle, non quadrati né icone.** Tre giri con l'autore. Prima cinque quadrati uguali letti per posizione: nessuno
  ricorda quale posizione è quale blocco. Poi un'icona per blocco (curve dei filtri, campana, retta piegata, onda
  squadrata): poco comprensibili e poco standard; a confronto anche una famiglia di simboli pieni (cursori, frecce,
  fiamma), in cui però passa-alto e passa-basso restavano quasi uguali. Scelte infine le **sigle** `HP LP EQ CMP SAT`,
  quelle stampate su qualunque mixer: piene quando il blocco lavora, col bordo quando è acceso ma neutro, tenui in
  bypass; `CMP` nel colore della sorgente con la sidechain e con due barrette quando è parallelo. Ritiro
  l'obiezione fatta alle lettere nella prima versione: erano lettere singole (H L E C S), queste si leggono.
- **Nelle strip le sigle stanno in colonna**, una sotto l'altra a larghezza strip e centrate, come gli slot degli
  insert di una console. Confrontate con l'autore quattro disposizioni: una riga sola chiede strip da 90 px (a 1440 ne
  starebbero 7 invece di 16); due righe obbligano a un carattere da 7 px; la griglia a due colonne taglia `CMP` a
  40 px. La colonna costa 54 px di altezza: l'altezza predefinita del mixer sale da 240 a 296 px
  (`--mixer-h-glance`) e la timeline scorre sopra.
- **Tre stati, non due.** "Acceso ma neutro" esiste perché un EQ non in bypass con le bande a 0 dB non fa nulla: con
  due soli stati il segno mentirebbe, oppure il clic sul bypass non cambierebbe ciò che si vede.
- **Clic sulle sigle nelle strip e nell'Overview, non nelle intestazioni.** Nelle strip ogni sigla è larga quanto la
  strip (36 × 10 px) e nell'Overview ogni cella lo è: lì il clic mette o toglie il bypass e il doppio clic apre il
  blocco nell'inspector (confermato dall'autore il 2026-11-13). Nelle intestazioni le sigle sono piccole e in fila:
  restano di sola lettura e il clic seleziona la traccia.
- **Colonna delle etichette nel mixer** (`--rail-w` 44 px, 72 nell'Overview) invece di sigle o colori per i ritorni:
  i ritorni restano neutri (principio 3 del design) e la riga di ogni mandata ha il nome scritto. Costo: le strip
  scendono da 42 a 40 px di minimo (`--strip-w-glance`) per far stare 16 strip e il bus a 1440.
- **Intestazione da 240 a 344 px** (da 200 a 304 alla finestra minima; `--head-w-glance`, `--head-w-glance-min`): la
  cella dei segni è larga 94 px (le cinque sigle in fila) e non deve togliere spazio al nome. Le corsie perdono
  104 px.
- **L'app non cambia finché F13 non parte.** L'interfaccia importa token e stili direttamente da `design/` (A-002):
  per questo le misure nuove sono token a parte, da sostituire a `--head-w`, `--strip-w` e `--mixer-h` in F13, e le regole che
  ritoccano markup già esistente (mandate nella strip, bus fissato a destra) valgono solo dove c'è la colonna delle
  etichette (`.mixer:has(.mxrail)`).
- **Collegamenti nel colore d'accento** (lime), non nel colore della traccia selezionata: i ritorni sono neutri e
  il loro grigio non si vedrebbe.
- **Overview senza riga del fader e senza la riga "→ gruppo"**: l'uscita è già una riga della griglia, e con 4
  ritorni la griglia non starebbe nell'altezza del mixer.
- **Tutte le tavole del Mix rigenerate** con i segni, perché il design è normativo e due versioni della stessa
  intestazione sarebbero un'ambiguità.

**Da decidere con l'autore sulle tavole.** Se la cella dei segni vale i 104 px tolti alle corsie;  se l'Overview deve
restare una vista del mixer o diventare un pannello a sé; se i collegamenti devono anche attenuare le tracce non
collegate.

### A-067 — Stato del mix a colpo d'occhio nell'app: scelte e differenze dalle tavole (2026-11-13, F13)

**Fatto.** Funzione pura `chainGlance` (`app/ui/src/mix/glance.ts`) che legge dal `Track` lo stato dei cinque blocchi
con la regola di UI-8, la sorgente della sidechain, il parallelo e l'automazione; `relations` dà, per la selezione, il
motivo di ogni collegamento (UI-10). I segni (`Marks.tsx`) sono memoizzati per traccia e non dipendono dalla testina;
nella strip solo la parte di fader e meter segue i meter. Mixer con colonna delle etichette, una mandata per ritorno,
bus fissato a destra e vista Overview (`mixerView`, stato di vista). Il blocco aperto nell'inspector passa dallo stato
locale allo store (`inspBlock`) perché il doppio clic su una sigla o una cella lo apra.

**Token.** Le misure di A-066 diventano quelle di base invece di restare token a parte: `--head-w` 344 px,
`--head-w-min` 304, `--strip-w` 40, `--mixer-h` 296; spariscono i token `-glance`. Nuovi `--mixer-h-short` (240 px,
finestra sotto 800 px di altezza, come la tavola 3d) e `--chain-col-h` (54 px, la colonna delle sigle, che la colonna
delle etichette usa per allineare "Chain"). Tavole rigenerate: cambia solo il foglio dei componenti.

**Scelte dove le tavole o FOUNDATION non dicono.**
- **Clic e doppio clic.** Un clic su una sigla o una cella attende 250 ms un eventuale secondo clic, così un doppio clic
  apre il blocco senza mettere e togliere il bypass (due passi di cronologia). Da tastiera (Invio, Spazio) il bypass è
  immediato.
- **Mandata esistente nell'Overview.** Una mandata non ha bypass nel documento: il clic su una mandata presente non fa
  nulla (toglierla perderebbe il livello), il doppio clic apre le mandate nell'inspector; su una assente il clic la crea
  al livello predefinito di `send.<return>` (−12 dB).
- **Ritorni nell'Overview.** Le prime due righe danno i filtri del ritorno (per il delay quelli del feedback), la terza
  il processore (clic = bypass del ritorno, `bypass.fx`), la quarta decadimento o tempo; la riga delle mandate dice
  quante tracce gli mandano.
- **Automazione.** Il segno è nel colore dell'assistente solo se l'assistente ha scritto tutte le corsie con punti;
  basta una corsia a mano perché sia nel colore d'accento. La cella dell'Overview dice il primo parametro e "+n".
- **Collegamenti di un gruppo selezionato.** Valgono le quattro regole di UI-10 (il gruppo che ne prende l'uscita, i
  ritorni, chi gli manda, chi lo ascolta); i membri non sono marcati: stanno già rientrati sotto il gruppo.
- **Saturazione neutra** si scrive "dry" nella cella dell'Overview.

**Differenze volute dalle tavole.**
- **Contrasto (UI-7).** Le sigle e le celle in bypass sono disegnate in `--line-strong`, sotto l'AA per il testo
  (axe-core le segnala). Nell'app prendono `--tx-3`: restano un gradino sotto la sigla bordata ("neutra") e lontane
  da quella piena.
- **Righe allineate per ritorni e bus.** Nella tavola 3a le strip dei ritorni e del bus non hanno la colonna delle
  sigle, e le loro righe delle mandate salgono di 54 px rispetto alla colonna delle etichette. Nell'app hanno una
  colonna invisibile della stessa altezza: ogni riga sta all'altezza della sua etichetta.
- **Ordine e righe delle strip come prima di F13.** L'app tiene l'ordine della timeline (gruppi prima dei loro membri)
  e la riga "→ gruppo" quando il progetto ha gruppi; le tavole mettono i gruppi in fondo e non hanno la riga.
- **Overview più larga della finestra.** Con la colonna larga dell'Overview (92 px) e 17 colonne il mixer a 1440
  scorre di lato; la tavola 3e, con 16 colonne strette, ci sta.

**Verifiche.** Vitest sulle regole (tutti gli stati, EQ neutro senza bande, con bande a 0 dB, con bande disattivate
con guadagno), Playwright su sigle senza selezione e a tutte le altezze, clic e doppio clic sulla sigla con un passo
di undo, quattro ritorni con righe allineate alla colonna, elenco di chi manda, collegamenti di un ritorno e di una
traccia fuori dalla cronologia, Overview con bypass a un clic e mandata creata; 60 fps con 32 tracce sia a mixer
chiuso sia con mixer aperto, segni e 4 ritorni (NFR-3). Il backend finto ha gli stati delle tavole nel progetto demo e
`simulateReturns(n)` per più di due ritorni. Il self-test nell'app vera non è stato rieseguito: F13 non tocca core,
motore né contratti, e il controllo resta all'autore (VALIDATION 33).

### A-068 — La PoC confinata in `poc/` (2026-11-13, dopo F13, chiesto dall'autore, issue #3)

**Contesto.** Alla radice convivevano il codice della PoC (`src/`, `tests/`, `schemas/`, `scripts/`, `pyproject.toml`,
`Makefile`, `README.md`) e, in `docs/`, i suoi documenti accanto a quelli del prodotto (due `DECISIONS.md` e due
`PROGRESS.md`). È in arrivo un terzo filone, il sito di prodotto (issue #4).

**Alternative.** (a) `poc/`, `app/` e in futuro `web/`, con la radice che fa da indice; (b) `poc/`, `web/` e il
codice dell'app alla radice.

**Scelta: (a).** Tre filoni con toolchain diverse stanno meglio in tre cartelle pari: con (b) il workspace Cargo si
mescolerebbe a `web/`, `design/` e `docs/`, e andrebbero riscritti tutti i riferimenti ad `app/` (FOUNDATION, regole,
comandi, `app/target` come unica cartella di build) senza un guadagno. `design/` resta alla radice perché serve sia
all'app sia al sito; `docs/` tiene solo FOUNDATION e `docs/app/`. `web/` nascerà con la issue #4.

**Fatto.** Spostamento con `git mv` (storia conservata): codice, `pyproject.toml`, `uv.lock`, `.python-version`,
`Makefile`, `README.md` e `.env.example` in `poc/`; `PRD.md`, `analysis.md`, `DECISIONS.md`, `PROGRESS.md` e
`milestones/` in `poc/docs/`; le regole archiviate in `poc/POC-RULES.md`, dove i loro collegamenti relativi tornano
validi. La PoC si riferisce a sé stessa solo con percorsi relativi, quindi il suo contenuto non cambia: l'unica
eccezione sono quattro collegamenti del suo README che puntavano fuori (prodotto, FOUNDATION, regole). Aggiornati:
`make oracle` e `oracle.py` (l'ambiente Python ora è `poc/.venv`), `disk.sh`, l'hook di formattazione, gli agenti e i
comandi `milestone-*`, i percorsi in FOUNDATION §5.1, §5.7 e §9 (regola 6), CLAUDE.md, i commenti dei crate e
`dsp.md`. Nuovo README di radice che fa da indice.

**Verifiche.** `make oracle` rigenera fixture identici a quelli tracciati; da `poc/` passano 674 test della PoC
su 675: `test_revise_is_deterministic` fallisce, ma falliva già prima dello spostamento (stesso esito su una copia
del commit precedente con il vecchio ambiente), quindi non dipende da questa modifica e, a PoC congelata, non si
corregge. `make check` del prodotto verde.

### A-069 — Un interruttore per pannello nella barra in alto del Mix (2026-11-13, dopo F13, chiesto dall'autore, issue #7)

**Contesto.** Mixer, inspector e assistente si chiudevano già ciascuno per conto suo (UI-4), ma solo da tastiera
(⌘M, ⌘I, ⌘J): a schermo c'era soltanto il modo di riaprirli (la striscia `Mixer` e le due colonne ripiegate). Chi non
conosce le scorciatoie non aveva modo di liberare spazio.

**Alternative.** (a) Tre interruttori nella barra in alto, sempre nello stesso posto; (b) un pulsante di chiusura
nell'intestazione di ogni pannello, lasciando la riapertura alle strisce; (c) una voce di menu.

**Scelta: (a).** Un solo posto che mostra lo stato dei tre pannelli e non si sposta quando un pannello sparisce; (b)
sparpaglia i comandi e li fa sparire insieme al pannello. Gruppo `Panels` prima delle impostazioni, con le icone già
usate dalle colonne ripiegate (`faders`, `eye`, `spark`): nome fisso (`Mixer panel`…) e stato in `aria-pressed`, la
scorciatoia nel tooltip. Lo stato è lo stesso di scorciatoie e strisce, che restano. Mentre una proposta occupa le
colonne di destra, gli interruttori di inspector e assistente escono dalla barra (non avrebbero su cosa agire, e in
quel momento alla finestra minima il posto manca); quello del mixer resta.

**Conseguenza.** Alla finestra minima (1280 px) la barra non aveva più posto con il testo `Unsaved changes ·
autosaved`: ora è quel testo a cedere, con i puntini e il tooltip per intero, e i meter del bus restano nella finestra.

**Differenza dalle tavole.** Le tavole 3a–3g non hanno questi interruttori: il design non è stato aggiornato (il suo
generatore non è stato toccato). Solo lo spazio di lavoro Mix: nel Master le colonne non si chiudono.

**Verifiche.** Playwright: ogni interruttore agisce da solo, segue scorciatoia e striscia, la barra sta in 1280 px con
modifiche non salvate; con una proposta in revisione i due laterali non ci sono. UI-4, UI-3.

### A-070 — Proposte che restano e si riapplicano (2026-11-13, dopo F13, P-014, issue #2)

**Contesto.** Una proposta viveva solo in memoria finché era in revisione: applicata o scartata, restava il registro
della chiamata (AI usage) ma non c'era modo di riaverla.

**Scelte.**
- **Cosa si conserva.** Per il mix un file `proposal.json` nella cartella della chiamata: piano, mappa degli id,
  elementi nuovi, trim, ambito, valutazioni, avvisi. Non i comandi né il diff: si ricalcolano sul progetto com'è
  quando la proposta si rivede, con lo stesso codice di ogni applicazione (`apply_commands`). Per il master basta il
  `plan-0.json` che la chiamata scriveva già. Il registro della chiamata dice se la proposta c'è (`stored`, assente
  nei progetti vecchi = no): le chiamate fatte prima di questa modifica restano in elenco ma non si riaprono.
- **Rivedere invece di applicare alla cieca.** "Review" rimette la proposta nella stessa schermata di revisione di
  una nuova (A/B, applica tutto o una parte), con una nota su quando è arrivata. Alternative scartate: un pulsante
  "Apply" diretto dall'elenco (si applicherebbe senza vedere né sentire cosa cambia oggi) e un elenco in AI usage
  (lontano dal lavoro; l'autore chiedeva l'assistente).
- **Conferma.** Solo per le proposte riaperte, prima di applicare: dice quando è arrivata, che il mix può essere
  cambiato e che le impostazioni attuali delle tracce toccate saranno sostituite; un passo di undo.
- **Esito.** Applicare una proposta riaperta aggiorna l'esito della sua chiamata; chiuderla ("Close review") lo lascia
  com'era. Nessuna chiamata nuova, nessun costo.
- **Misure.** Nel mix le valutazioni mostrate sono quelle prese alla ricezione (rimisurare vorrebbe dire due render
  a ogni riapertura); la nota lo dice. Nel master la proposta è rimisurata sul mixdown attuale, come una nuova.
- **Dove.** Scheda `Proposals` nell'assistente di Mix e di Master. Con quattro schede il selettore del Mix va su una
  riga sua: accanto al titolo non stava nella colonna da 288 px.
- **Barra in alto.** Il nome del progetto ora si tronca dopo lo stato di salvataggio, quando la barra è piena (una
  proposta in revisione alla finestra minima).
- **Backend finto.** L'annulla non riporta più indietro il registro delle chiamate AI, come nell'app.

**Differenza dalle tavole.** La scheda, la nota e la conferma non sono nel design consegnato.

**Verifiche.** Rust: una proposta scartata torna identica su un progetto immutato; dopo un fader mosso a mano e una
traccia eliminata il diff è contro lo stato di oggi, la traccia mancante è dichiarata, i ritorni non si duplicano,
l'applicazione è un solo passo; lo stesso per il master. Playwright: elenco, revisione, conferma con annulla, esito
aggiornato, chiusura senza effetti, in Mix e in Master. Non provato nell'app vera né con proposte di un modello
reale (VALIDATION 35).

### A-071 — Configurazione guidata dell'assistente: scelte del design (2026-11-13, chiesto dall'autore, approvato il 2026-11-13)

**Contesto.** In vista dell'apertura del repository l'autore vuole che chi non sa cosa sia OpenRouter arrivi comunque
ad avere l'assistente funzionante. Oggi il primo avvio (1b) e le Impostazioni (11b) chiedono di incollare una chiave.

**Verificato sulla documentazione di OpenRouter (2026-11-13).** Esiste un flusso OAuth con PKCE: l'app apre
`https://openrouter.ai/auth?callback_url=…&code_challenge=…&code_challenge_method=S256` (più `key_label` e `state`),
l'utente accede o si registra e approva, il browser torna alla `callback_url` con un `code` valido 10 minuti, l'app lo
scambia con `POST /api/v1/auth/keys` e riceve `key`, una chiave normale dell'utente. `http://localhost` è ammesso su
qualunque porta, senza registrazione dell'app. **Non** risultano: schemi URL propri, un limite di spesa impostabile
dal flusso, la lettura del saldo con una chiave normale (`GET /api/v1/credits` vuole una chiave di gestione;
`GET /api/v1/key` dà solo uso e limite della chiave). Senza credito una richiesta torna 402, con `limit_source`
`openrouter_credits` (saldo) o `openrouter_key_limit` (limite della chiave).

**Scelte.**

- **Un dialogo in tre passi** (`Connect OpenRouter`, `Add credit`, `Ready`), lo stesso dai tre ingressi: primo avvio
  (1b), Impostazioni › AI (11b), colonna dell'assistente senza chiave (13e). Tavole 14a–14k.
- **OAuth come via principale, chiave incollata come via secondaria** (14g, 14h): tre passi numerati, ciascuno col
  collegamento alla pagina giusta. Alternativa scartata: solo la guida a passi, che lascia il copia e incolla.
- **Ritorno su `localhost`**: l'app ascolta su `127.0.0.1`, porta scelta dal sistema, per una sola richiesta e al più
  10 minuti; `state` e PKCE S256. La pagina che il browser mostra (14c) la serve l'app. Da conciliare con NFR-7 al
  momento dell'implementazione: la rete resta solo OpenRouter e solo su azione dell'utente, ma si aggiunge un ascolto
  locale temporaneo.
- **Il passo del credito non verifica il saldo**, perché l'app non può leggerlo: lo dice in chiaro (14d), offre il
  collegamento alla pagina dei crediti e si chiude con "I have added credit" o "Skip for now". La mancanza di credito
  si scopre alla prima richiesta e ha il suo stato nella colonna dell'assistente (14i), distinto dal limite della
  chiave (14j) e dalla chiave non più valida (14k).
- **Il tetto di spesa per progetto si imposta già qui** (14d): è la sola protezione che l'app controlla.
- **Gli importi di 14d sono di esempio.** Nell'app vanno calcolati dal registro dei costi; finché non esiste una
  storia, da valori misurati dall'autore col modello predefinito (VALIDATION).
- **Demo offline sempre a un clic**, in 1b e nel dialogo: si può provare l'app prima di aprire un conto.

**Corretto di passaggio.** Le didascalie delle tavole non si vedevano in nessuna pagina: `.cap` del fader
(`components.css`) copriva `.cap` della tavola. La regola in `screens.css` è ora `.board>.cap` e azzera le proprietà
ereditate.

**Implementato** (P-015, AI-24). `stemwise-ai::connect` (ascolto su `127.0.0.1` e `::1`, PKCE, scambio del codice),
comandi `ai_connect_start`/`cancel`/`reopen` e `ai_open_page` (elenco fisso di pagine di OpenRouter), dialogo
`ui/src/ai/Setup.tsx`. Differenze dalle tavole:

- **Tetto di spesa in 14d solo con un progetto aperto**: il tetto è del progetto. Dal primo avvio il passo dice dove
  impostarlo.
- **Importi di 14d calcolati**, non di esempio: la stima dell'app su un brano tipo (12 tracce, 8 sezioni) ai prezzi
  del modello scelto (`budget::typical_costs`). Restano stime: da confrontare coi costi veri (VALIDATION 36).
- **"Open OpenRouter" al posto di "Open sign-up"** in 14g: la documentazione non dà l'indirizzo della registrazione;
  si apre la pagina principale.
- **Il rifiuto senza `code`** è trattato come "non approvato": la documentazione non descrive il ritorno dopo un
  rifiuto. Se il provider non torna affatto, l'attesa finisce alla scadenza o con Cancel.
- **1b era disegnata ma non costruita**: ora il primo avvio mostra l'invito al posto dei progetti recenti.
- I test di policy (NFR-7, NFR-8) ammettono `connect.rs` come secondo file di rete e `signing.rs` come secondo
  modulo specifico di macOS, e verificano che l'ascolto sia solo locale e che l'app apra solo pagine del provider.

**Non verificato.** Il flusso col provider vero (l'agente non usa conti né chiavi): tutto è provato contro il server
finto. VALIDATION 36–38.

### A-072 — Dove sta la chiave: portachiavi se silenzioso, altrimenti file privato (2026-11-13, deciso dall'autore)

**Contesto.** Al primo uso della chiave macOS chiedeva la password del portachiavi due volte. Il codice legge una
volta sola (A-052); le due richieste sono di macOS, che per un'app non firmata con un'identità stabile controlla sia
l'accesso all'elemento sia la sua lista di partizione, e li fallisce entrambi a ogni build.

**Alternative.** Solo portachiavi con la firma dell'autore (le richieste restano in `make dev` e per chi compila dai
sorgenti); sempre file (si rinuncia al portachiavi anche nel pacchetto firmato); automatico.

**Scelta dell'autore: automatico.** `signing::keychain_is_silent()` chiede al sistema se il codice in esecuzione
soddisfa `anchor apple generic` (qualunque certificato emesso da Apple; mai una firma ad hoc). Se sì: portachiavi,
come prima (`LazyKeyStore`). Se no: `FileKeyStore`, file `openrouter.key` nella cartella dati dell'app, creato con
permessi 600 e sostituito per rinomina. Fuori da macOS: portachiavi. `STEMWISE_KEY_STORE=keychain|file` forza la
scelta. `AiStatus.key_place` porta la scelta all'interfaccia, che la nomina ovunque parli della chiave.

**Conseguenze.** Con una build non firmata una chiave già salvata nel portachiavi non viene letta (leggerla
riproporrebbe le richieste): si ricollega l'assistente una volta. Il file protegge da altri utenti del computer, non
da programmi che girano con lo stesso utente; il portachiavi resta la scelta dove non costa attrito. Firmare il
pacchetto resta all'autore (A-046): con la firma l'app userà il portachiavi da sola.

**Non verificato.** Che il portachiavi sia davvero silenzioso nella build firmata (VALIDATION 38).

### A-073 — Strip del mixer più larghe (2026-11-13, scelta dell'autore)

**Contesto.** A 40 px (A-067) le strip tagliavano 9 nomi su 17 nel progetto demo e l'autore le trovava strette.

**Alternative mostrate** sul mixer vero a 1440 px con i pannelli laterali aperti: 52 px (3 nomi tagliati, 12 strip
visibili), 64 px (nessuno, 10 visibili), 64 px con controlli più grandi.

**Scelta dell'autore: la terza.** `--strip-w` 64 px (52 nella finestra compatta, `--strip-w-min`); nelle strip il
fader e il suo cap passano da 22 a 30 px (`--strip-fader-w`, `--strip-cap-w`) e le sigle della catena da 8 a 9,5 px
(`--fs-sig-strip`); intestazioni delle tracce, inspector e cursori orizzontali restano com'erano. La colonna delle
sigle cresce di 9 px: l'altezza predefinita del mixer sale a 304 px e a 250 nella finestra bassa, così la corsa del
fader non cambia. Il mixer scorre in orizzontale prima: 10 strip più il bus invece di 17 a 1440 px.

**Non toccato.** Nella finestra minima (1280×720) la corsa del fader era ed è di 30 px: poca, ma è un limite
precedente a questa modifica.

### A-074 — Colonna delle etichette del mixer: una sola larghezza (2026-11-13, segnalato dall'autore)

**Contesto.** La colonna delle etichette era larga 44 px nella vista Faders e 92 nella Overview (A-067): passando da
una vista all'altra le strip si spostavano di 48 px, e in Faders i nomi dei ritorni più lunghi venivano tagliati.

**Scelta.** `--rail-w` vale 92 px, come `--rail-w-wide`: la colonna ha la larghezza della Overview in entrambe le
viste. Le strip non si muovono e le etichette stanno per intero. Costa 48 px alle strip nella vista Faders. Test
Playwright: stessa larghezza della colonna, stesse posizioni delle strip, nessuna etichetta tagliata.

### A-075 — Logo: direzione "D · Due stanze", da rifinire in canvas (2026-11-13, scelta dell'autore)

**Contesto.** L'icona dell'app (cinque barre lime su melanzana) e il marchio nella barra in alto (quattro barre) sono
corretti ma anonimi: sono le barre di un equalizzatore qualunque. L'autore ha fatto disegnare quattro direzioni a un
agente esterno su un brief autonomo (`design/brand/BRIEF-logo.md`); le proposte sono archiviate in
`design/brand/directions/`.

**Alternative.** A · Convergenza (quattro stem che si raccolgono in un'uscita), B · Monogramma (una S come curva di
automazione), C · Il fader (un cap fermo nel punto giusto), D · Due stanze (una traiettoria lime e una lilla che si
incontrano).

**Scelta dell'autore: D.** È l'unica che racconta una caratteristica propria del prodotto, le due fasi Mix e Master
coi loro due accenti. La proposta consegnata è un punto di partenza, non il logo finale: due tratti che si uniscono
sotto una punta di freccia si leggono come l'icona generica "merge"; le due linee arrivano insieme, mentre le fasi
vengono una dopo l'altra; il segno occupa metà dell'altezza e a 16–32 px si riduce a una freccia; in monocromia (la
barra in alto, `currentColor`) resta solo il "merge"; il lettering del lockup non è Bricolage Grotesque; il fondo
dell'icona non è uno squircle ad angoli continui.

**Prossimo passo.** Una sessione di Claude Desktop in canvas col prompt `design/brand/PROMPT-canvas-D.md`: tre
varianti dentro la D, una scelta dell'autore, poi la rifinitura completa con consegna in `design/brand/final/`.
L'adozione nell'app (icone Tauri da `icon-1024`, marchio in `app/ui/src/shared/Icon.tsx` e `mark()` delle tavole,
welcome e pagina di collegamento all'AI) è un passo successivo, con i suoi controlli a vista.

### A-076 — Logo: la D rifinita (2026-11-14, su indicazione dell'autore)

**Contesto.** Il prompt `design/brand/PROMPT-canvas-D.md` chiedeva tre varianti dentro la D e poi la rifinitura di
quella scelta. Le tre varianti (Soglia: tre linee e un anello; Pianta: due stanze in pianta; Esse: una S spezzata) sono
state scartate tutte dall'autore: andava rifinita la proposta D consegnata, non disegnate forme nuove.

**Alternative.** Tenere la D com'era togliendo solo la freccia (resta un "merge" a Y, le due linee restano due
ingressi alla pari); una scala che mette in fila le due curve (perde la forma della D); le due curve della D come Mix
e, al posto della freccia, una seconda forma per il Master.

**Scelta.** L'ultima. Le due traiettorie della D, con la loro curva, sono entrambe lime: il Mix, molte parti che si
riuniscono e si toccano senza fondersi. Oltre un varco, al posto della freccia, la stanza del Master: un rettangolo
pieno arrotondato in lilla. Linee contro superficie tengono distinte le due stanze anche in monocromia; nella barra
in alto si accende la stanza della fase corrente e l'altra va al 40 % (come colore mescolato col fondo, non come
opacità). Il nome è Bricolage Grotesque 600 convertito in tracciati dal file che l'app già include.

**Motivo.** Risolve i sette difetti elencati in A-075 cambiando il meno possibile della forma approvata. Consegna in
`design/brand/final/` con README e `presentation.html`; si rigenera con `design/brand/build-final.py`. L'adozione
nell'app resta il passo successivo.

### A-077 — Master AI misurato: diagnosi e ciclo (2026-11-14, issue #1, P-016)

**Contesto.** L'autore: il master AI dà spesso un limiter strozzato che taglia le dinamiche, e alla richiesta di
conservarle abbassa il target LUFS. La domanda era se il limite fosse negli strumenti o nel modello.

**Cosa si è trovato** (sei chiamate reali salvate nei progetti dell'autore, tutte `gpt-6.1-sol`, e il mixdown di
Balance misurato con uno script):
- Il mixdown ha 17 dB tra true peak e loudness (−20,1 LUFS, picco a −3 dBFS); a −9,6 LUFS con tetto a −1 ne restano
  meno di 9. Solo lo 0,7% dei campioni è entro 9 dB dal picco: pochi colpi isolati. Stima di primo ordine: a −14 LUFS
  il limiter lavora il 2% del tempo, a −9,6 il 49%, con 8,5 dB sui picchi.
- Il modello riceveva `dyn_range_lu` (3–4 LU, la variazione di loudness tra i passaggi) e ne concludeva "dinamica già
  contenuta, nessuna compressione" in quattro risposte su sei. La distanza picco-loudness non gli arrivava.
- Una sola chiamata, senza sapere quanto avrebbe lavorato il limiter né prima né dopo; la misura del master proposto
  (A-033) andava solo all'interfaccia.
- Prompt di dieci righe, senza tecnica. Ogni chiamata era una revisione di una catena vuota con "cambia ciò che è
  chiesto e tieni il resto".
- Del limiter il piano regola solo il rilascio. Con queste leve abbassare il target era la mossa efficace.

**Scelte.**
- **Misure** (`mastering::dynamics`, analisi del master versione 2): `peak_to_loudness_db`, `short_crest_db`,
  `peak_share_pct`, `destination.peak_reduction_needed_db`, loudness dei riferimenti; `dyn_range_lu` diventa
  `loudness_range_lu` e il prompt dice cosa non significa.
- **Ciclo** (`run_master`): il master corrente e ogni piano sono renderizzati alla loudness della destinazione
  (`evaluate_master`: ricerca del guadagno, poi un render con le statistiche di riduzione,
  `engine::master::render_metered`, finestre di 10 ms) e misurati. Al più tre piani (`MASTER_ITERATIONS`); il ciclo si
  ferma quando il modello restituisce le stesse impostazioni del piano appena misurato, senza un altro render. Si tiene
  l'ultimo piano: non c'è una misura sola da ottimizzare come la distanza spettrale del mix, e il modello vede tutta
  la storia. Un'iterazione successiva senza piano valido lascia quello già misurato.
- **Prompt**: la catena descritta per ciò che fa davvero (la saturazione come clipper morbido con i dB tolti ai picchi
  a ogni drive, il guadagno di loudness trovato dall'app, il limiter come unico stadio dopo), come leggere le misure,
  come distribuire la riduzione dei picchi tra gli stadi, soglie pratiche sul limiter (oltre 3 dB per più di qualche
  per cento del tempo si sente).
- **Destinazione** (precisa A-035): con una richiesta il modello può ancora cambiarla, ma il prompt lo ammette solo
  se la richiesta chiede a parole un'altra loudness. Non è imposto dal codice: riconoscere "più volume" in ogni lingua
  non è affidabile. Se la destinazione non si raggiunge pulita, il `summary` lo dice e indica di quanto abbassarla o
  di domare i picchi nel mix.
- **Prima richiesta**: su una catena uguale a quella di un progetto nuovo la richiesta chiede un master intero (nessun
  piano di partenza); il tipo di chiamata resta `MasterRevision` perché porta una richiesta.
- **File della chiamata**: `plan-<n>.json` ed `evaluation-<n>.json` per iterazione, `plan.json` il piano tenuto;
  `restore_master` legge `plan.json` e, per le chiamate precedenti, `plan-0.json`.
- La misura per l'interfaccia (`measure_master`) parte dal guadagno già trovato nella chiamata.

**Alternative scartate.** Tenere il piano "migliore" secondo un punteggio (quale? loudness, punch e timbro tirano in
direzioni diverse); imporre il target dal codice anche nelle revisioni (toglie una richiesta legittima).

**Non fatto qui** (passo successivo, tocca MAS-1, schema di progetto e design): clipper prima del limiter, mix
parallelo e filtro sul detector del compressore di master (oggi il detector sente anche i bassi: il filtro a 100 Hz
vale solo con una sidechain esterna), ordine della catena scelto, scostamenti per sezione, ritorno al mix quando i
picchi sono troppi, la valutazione mostrata nella proposta. Il master multibanda resta fuori scope (FOUNDATION §2).

**Misura col motore sul mixdown vero** (Balance, 187 s, sonda temporanea non conservata; limiter: tempo oltre 3 dB
di riduzione e riduzione massima):

| Destinazione | Solo limiter | Saturazione 9 dB | Saturazione 12 dB | Compressore veloce + saturazione 9 dB |
| --- | --- | --- | --- | --- |
| −14 LUFS | 0,0% · 4,2 dB | 0,0% · 1,9 dB | 0,0% · 0,4 dB | 0,0% · 1,4 dB |
| −12,5 LUFS | 0,4% · 5,9 dB | 0,0% · 3,5 dB | 0,0% · 1,9 dB | 0,0% · 2,9 dB |
| −9,6 LUFS | 21,4% · 10,7 dB | 12,7% · 8,0 dB | 5,5% · 6,0 dB | 10,0% · 7,2 dB |

A −9,6 il limiter da solo lavora oltre 1 dB per il 61% del tempo. Gli stadi già presenti lo alleggeriscono molto, ma
la cresta breve persa resta 2,9 dB in ogni variante (0,1 a −14): quella perdita la decide la destinazione, non lo
strumento. Lo strumento decide **come** si perde (picchi arrotondati o tutto il mix che si abbassa dopo ogni colpo).
Per tenere più punch a quella loudness servono picchi più bassi già nel mix.

**Verifica.** `the_master_loop_measures_each_plan_and_the_next_decision_corrects_it`,
`a_request_on_an_untouched_master_asks_for_a_whole_master_and_on_a_worked_one_for_a_change`,
`the_metered_render_is_the_same_audio_and_reports_how_hard_the_limiter_works`. Col modello vero: VALIDATION 39.

### A-078 — Il marchio nuovo nell'app (2026-11-14, richiesta dell'autore)

**Contesto.** La consegna di A-076 (`design/brand/final/`) va adottata: icona dell'app, marchio nelle barre in alto,
schermata iniziale, pagina di fine collegamento nel browser, tavole del design.

**Scelta.** Il componente `Mark` (`app/ui/src/shared/Icon.tsx`) e `mark()` delle tavole (`design/tools/lib.mjs`)
disegnano la versione per 20 px; con `large` quella sulla griglia di 100 (schermata iniziale, 48 px). Le due stanze
sono due tracciati (`mark-mix`, `mark-master`) colorati da `components.css`: la stanza della fase corrente
nell'accento, l'altra al 40 % mescolata col fondo (`color-mix`), secondo `data-phase`; nelle schermate fuori da un
progetto vale il Mix. `brand` tiene lime e lilla: schermata iniziale e pagina del browser (`connect.rs`, tavola 14c),
che non ha fasi. Le icone Tauri sono generate da `icon-1024.svg` rasterizzato a 1024 px (`source.png`) con
`tauri icon`, tenendo solo i file elencati in `tauri.conf.json`. L'interfaccia nel browser (`make ui`) ha il favicon
da 32 px.

**Verifica.** `make check`; tavole rigenerate con `node design/tools/build.mjs`; icona nel pacchetto: VALIDATION 40.

### A-079 — Catena di master libera: tavole 15a–15h, in attesa di approvazione (2026-11-14, issue #1, P-017)

**Contesto.** Dopo A-077 l'autore ha chiesto di disegnare il passo successivo e ha posto due domande: se l'ordine
della catena debba restare fisso e se il master si lavori anche per sezioni. Ha inoltre deciso che il **master
multibanda entra nello scope** (era nella colonna destra di FOUNDATION §2).

**Scelte di design** (`design/screens/15-master-chain.html`, generatore `design/tools/s-chain.mjs`):
- **Dieci moduli.** Input per primo; Limiter e Dither per ultimi, fissi; in mezzo EQ, Match EQ, Compressor,
  Multiband, Width, Saturation e Clipper, **in ordine libero**. Fissi solo quelli che lo sono anche nella pratica del
  mastering: il limiter true peak chiude la catena perché il tetto (MAS-3) vale solo se nulla lo segue, e il dither
  viene dopo per definizione; Input è un guadagno d'ingresso, non un effetto. Due righe da cinque; lucchetto sui
  fissi, maniglia sui mobili; trascinamento, ⌥← e ⌥→, menu del modulo, "Reset order". Un riordino è un passo di undo.
- **Clipper mobile** (rivisto col parere dell'autore: nella prima stesura era fisso prima del limiter, ma quella è
  una consuetudine, non una regola: c'è chi taglia prima del compressore o tra due stadi). Parte prima del limiter.
  Due controlli: **"Take off up to"**, quanti dB togliere ai picchi più alti che gli arrivano, e la morbidezza; un
  istogramma dice dove stanno i picchi e quanti ne taglia. La soglia è relativa al picco misurato all'ingresso del
  clipper (l'app renderizza comunque il master per trovare il guadagno di loudness), quindi vale in ogni posizione e
  con ogni destinazione; la prima stesura la contava dal tetto, che ha senso solo subito prima del limiter.
- **Compressore**: Mix (parallelo, come sulle tracce, 7e) e "Listens above" (filtro sul detector).
- **Multibanda da due a sei bande** (deciso dall'autore: tre è la partenza, non il limite, anche e soprattutto per il
  modello). Si apre con tre (Low, Mid, High); − e + cambiano il numero, un doppio clic sul grafico divide una banda,
  trascinare un crossover sul vicino ne unisce due. Per banda bypass, solo, riduzione e i cinque controlli; da quattro
  bande in su le schede mostrano nome, intervallo e riduzione (15h). Sei come le bande dell'EQ.
- **Scostamenti per sezione**, non catene diverse: livello, lati, rilascio del limiter e guadagni dell'EQ, sommati al
  master di tutto il brano mentre la sezione suona, con rampe di un decimo di secondo. Una corsia sotto la forma
  d'onda li mostra; selezionare una sezione apre il suo editor al posto di un modulo. "All 3 choruses" scrive gli
  stessi valori sulle sezioni con lo stesso nome.
- **Proposta misurata**: prima dell'elenco dei moduli, cinque righe prima → dopo (limiter oltre 3 dB, riduzione
  massima, punch, loudness, distanza dal riferimento); ordine e sezioni sono schede con la loro spunta.
- **Ritorno al mix**: quando, fatto il possibile, il limiter lavora ancora troppo, un avviso dà tre uscite: tenere,
  abbassare la destinazione a un valore calcolato, oppure andare al Mix con la richiesta già scritta nell'assistente
  (nessun invio senza conferma).
- La scheda Loudness guadagna il meter del clipper e "Limiter over 3 dB".

**Alternative scartate.** Ordine del tutto libero, limiter compreso (rompe MAS-3); catene diverse per sezione (un
altro prodotto: salti di timbro ai confini, proposta illeggibile; confermato dall'autore); soglia del clipper in dBFS
o contata dal tetto.

**Da decidere con l'autore prima di implementare**: l'impianto di queste tavole. Poi: requisiti MAS nuovi in
FOUNDATION, schema di progetto 3 con migrazione, `master_plan` v2 (ordine, clipper, multibanda, mix, sezioni), DSP
(clipper con la misura del picco al suo ingresso, crossover Linkwitz-Riley da due a sei bande, multibanda) con `docs/app/dsp.md`, motore con ordine variabile e scostamenti a
rampa, pannello.

### A-080 — Nessun genere come norma, blocchi ripetibili, tre modi di mostrare la catena (2026-11-14, issue #1, P-017)

**Contesto.** L'autore ha chiesto se l'app sia neutra rispetto al genere (la discussione su loudness e punch veniva
dal suo mix rock) e se nella catena di master si possano avere più blocchi dello stesso tipo.

**Neutralità: cosa si è trovato.** Motore, misure, ciclo e destinazioni non presuppongono un genere. I testi fissi sì:
il prompt del master di A-077 parlava di "snare and kick" e "drum hits"; il bilanciamento del mix (A-063) dava per
scontata una band con cantante e citava rock e metal; "un mix senza spazio udibile suona incompiuto" era un gusto. I
ruoli delle tracce sono quelli di una band.

**Scelte sui prompt** (fatte):
- **Master**: misure e funzioni, mai strumenti ("short transients", "the tracks that carry them"). Un paragrafo nuovo,
  "What this music wants": il genere arriva solo da brief, riferimenti e destinazione; picchi alti e loudness range
  ampio non sono difetti in sé; dove la dinamica è l'esecuzione la catena leggera è la scelta giusta. Un test vieta
  nomi di strumenti nel prompt e nomi di generi fuori da quel paragrafo
  (`the_master_prompt_names_no_instrument_and_takes_no_genre_as_the_norm`).
- **Mix**: il bilanciamento parte dal decidere su cosa è costruito il brano. I rapporti di A-063 restano, invariati
  nei numeri, per la musica guidata da una voce; accanto: strumentale con una guida, musica costruita su ritmo e bassi,
  musica registrata come esecuzione (si conserva il bilanciamento della ripresa), tutto il resto. Lo spazio dipende
  dalla musica e dal brief. Nel prompt del mix i nomi degli strumenti restano: lì i ruoli sono dati del progetto.
- **Non fatto**: ruoli nuovi (legni, coro, pianoforte, sub, loop): toccano schema, analisi per famiglia e interfaccia;
  vanno con l'implementazione di P-017. L'avviso della tavola 15g nominerà la famiglia che porta davvero i picchi.

**Blocchi ripetibili** (deciso con l'autore, da implementare con P-017): la catena di master è una **lista di
blocchi**, non un insieme fisso. Ripetibili EQ, Compressor, Multiband, Saturation, Clipper; unici Input, Match EQ,
Width, Limiter, Dither. Al più due per tipo e dodici in tutto. Aggiungere un blocco deve avere un comando **sempre
visibile**, non solo una voce di menu (richiesta dell'autore).

**Tre modi di mostrare la catena** (`design/screens/15-master-chain-directions.html`, esplorazione):
- **G · Grid**: le schede di 15a con una terza riga e una tessera "Add module". Familiare; toglie altezza all'editor e
  l'ordine si legge in due direzioni.
- **R · Rack**: la catena è una colonna accanto all'editor, dall'alto in basso, una riga per blocco con valore,
  riduzione e interruttore; "Add module" è una riga propria. Dodici righe senza scorrere; editor più stretto.
- **P · Path**: una colonna per blocco con una barra che dice quanto i picchi stanno sopra il corpo dopo quel blocco e
  quanto il blocco ha tolto; un "+" tra ogni coppia e una colonna "Add module". Mostra chi fa il lavoro; nomi e valori
  stretti a dodici blocchi.

**In attesa**: la scelta dell'autore tra G, R e P; poi le tavole 15a–15h si ridisegnano in quella.

### A-081 — La catena di master è un rack: tavole 15 ridisegnate (2026-11-14, scelta dell'autore, P-017)

**Scelta dell'autore** tra le tre disposizioni di A-080: **R · Rack**. Le tavole di `design/screens/15-master-chain.html`
sono ridisegnate così, con i blocchi ripetibili; la pagina delle tre direzioni resta come memoria.

- **Il rack**: una colonna larga `--rack-w` (288 px, token nuovo) accanto all'editor, dall'alto in basso. Una riga per
  blocco: maniglia o lucchetto, numero d'ordine, nome, valore, barretta della riduzione che applica, numero della nota
  dell'assistente, spia. I blocchi ripetuti si chiamano "EQ 1", "EQ 2". L'editor del blocco scelto sta a destra, più
  stretto di prima (456 px utili): compressore e sezioni mettono i controlli su più righe; il multibanda ha una riga
  per banda a qualunque numero di bande, al posto delle schede.
- **Aggiungere** (15i): "Add module" è una riga sempre visibile nel punto in cui il blocco nuovo atterra (prima del
  limiter), col conto "11 of 12"; apre l'elenco dei tipi che si possono ancora aggiungere e dice perché gli altri no.
  In più un "+" al passaggio tra due righe. Il blocco nuovo nasce spento e si apre nell'editor.
- **Menu del blocco** (15j): sposta, bypass, reset, rimuovi; i fissi non hanno "Remove".
- **Riordino** (15b): trascinamento con una linea dove atterra, ⌥↑ e ⌥↓.
- La tavola 15g nomina il gruppo di tracce da cui vengono i picchi (A-080).

Elenco: 15a rack · 15b riordino · 15i aggiungere · 15j menu del blocco · 15c compressore · 15d multibanda · 15h cinque
bande · 15e sezioni · 15f proposta · 15g destinazione troppo alta. **In attesa dell'approvazione dell'autore**; poi
l'implementazione (elenco in A-079, con la catena come lista di blocchi e i ruoli nuovi di A-080).

### A-082 — F14: la catena di master libera, come è fatta (2026-11-14, P-017, tavole approvate dall'autore)

**Dati.** `MasterChain` = guadagno d'ingresso, `blocks` (lista di `MasterBlock { id, block }`, con `Block` etichettato
per tipo), limiter, dither; `MasterState.sections` porta gli scostamenti. Schema di progetto **3**: la migrazione
mette i cinque blocchi dello schema 2 nello stesso ordine e con le stesse impostazioni, più multibanda e clipper
spenti, quindi il render non cambia. Il compressore di master ha un tipo suo (`MasterComp`: mix e `listen_above_hz`,
nessuna sidechain). Le modifiche di struttura sono `ChainOp` nel core (`MasterState::chain_edit`: aggiungi, togli,
sposta, bande, dividi, unisci, reset, ordine iniziale): l'interfaccia chiede, il core fa.

**Motore.** Il processore tiene pronti due processori per tipo (uno per match EQ e ampiezza) e riceve la catena come
valore `Copy` con nove posti; chi usa quale processore lo decide l'id del blocco, così spostarlo non gli cambia lo
stato. **Latenza costante**: quella che i blocchi sovracampionati presenti non prendono la prende una linea di ritardo
(256 campioni più il limiter, contro i 64 più il limiter di prima). **Cambio di struttura all'ascolto**: l'uscita
scende al silenzio in pochi millisecondi, la catena nuova entra con lo stato pulito, si aspetta che code e limiter si
svuotino e si risale (circa 25 ms in tutto): nessuno scatto, verificato dal test dei cambi. Gli scostamenti di sezione
seguono la posizione sulla timeline con una costante di 25 ms (a regime in circa un decimo di secondo).

**Clipper.** La soglia è il picco al suo ingresso meno `take_off_db`. Quel picco si misura offline prima della
ricerca della loudness (`clip_peaks`: una passata per clipper acceso, il secondo misurato col primo al lavoro) e
viaggia con le misure del master (`MasterMeasures.clip_peak`): all'ascolto, tra una modifica a monte e la misura
nuova, il clipper usa il picco di prima. Deterministico (MAS-8).

**Misure.** `compute_loudness` fa una passata in più per sapere quanto lavora ogni blocco (`render_metered`):
`MasterMeasures` porta limiter e blocchi (massimo, media, tempo attivo, tempo oltre 3 dB), la distanza picco-loudness
e la cresta breve. Da qui la riga "Limiter over 3 dB" e l'avviso 15g: oltre il 5% del brano (`STRAIN_PCT`), con una
loudness più bassa stimata dalla riduzione massima del limiter (non cercata con un render).

**AI.** `master_plan` v2 (FOUNDATION §5.5). Un blocco che il piano non elenca viene spento e resta in coda; un EQ
senza bande in uso conta come spento. `MasterChange` dà la proposta voce per voce e `master_partial` ne applica
alcune. La valutazione porta la riduzione di ogni blocco per id. I piani v1 conservati si convertono alla lettura.
Lo schema strict usa `anyOf` sui sette tipi di blocco: provato solo col server finto. Il prompt descrive i blocchi,
l'ordine come decisione e le sezioni, senza nomi di strumenti (A-080).

**Interfaccia: scostamenti dalle tavole.**
- Con una proposta in revisione il rack mostra la catena proposta in sola lettura e al posto dell'editor c'è un
  riquadro che lo dice (la tavola 15f teneva l'editor aperto): modificare una catena che non è quella del progetto
  avrebbe confuso.
- I numeri delle note dell'assistente sulle righe del rack non ci sono: il master non conserva note numerate.
- L'istogramma del clipper è disegnato dai picchi del mixdown per colonna (un'approssimazione per l'occhio), non dal
  segnale al suo ingresso.
- L'avviso 15g non nomina la famiglia di tracce da cui vengono i picchi: servirebbe un'analisi degli stem contro il
  mixdown che non c'è (riserva).
- "Reset" nell'intestazione del rack riporta i blocchi nell'ordine di un progetto nuovo.

**Ruoli** (A-080): `piano`, `woodwinds`, `choir`, con le loro regole sui nomi dei file; nelle famiglie del
bilanciamento piano e legni stanno con le tastiere, il coro con i cori. Famiglie proprie per la musica orchestrale
non ci sono (riserva).

**Verifica.** Motore: ordine, due blocchi dello stesso tipo, latenza uguale, clipper, multibanda, sezioni, render
uguale a blocchi di 64, 1000 e 4096, nessuno scatto ai cambi. Core: migrazione, limiti, `ChainOp`. AI: piano ↔ stato,
voci, applicazione parziale, piano v1, ciclo che aggiunge il clipper. Interfaccia: 22 test Playwright del Master.
Non eseguito: il self-test nell'app impacchettata e `make build`.

### A-083 — La forma d'onda del master: picchi e corpo (2026-11-14, segnalato dall'autore)

**Contesto.** Su un master forte la forma d'onda era un rettangolo viola pieno. L'app disegnava il mixdown
moltiplicato per il guadagno e tagliato al tetto, colonna per colonna: ogni colonna di un paio di pixel contiene
migliaia di campioni, almeno uno arriva al tetto, e tutte risultano piene. Più altezza non avrebbe cambiato nulla.

**Scelta.**
- Il master si disegna da ciò che è stato **misurato davvero**: alla fine della ricerca della loudness la sessione
  tiene l'inviluppo del master renderizzato, 2048 colonne di (minimo, massimo, RMS), anche nel file
  `cache/master/loudness.json` (versione 2), così un progetto riaperto lo ha subito. Comando `master_wave`, tipo
  `MasterWave`.
- Tre strati: il **corpo** (RMS) in colore pieno, i **picchi** come linea sottile (in una prima stesura erano una
  fascia piena, che su un master forte riempiva di nuovo tutta l'altezza e l'autore non la capiva), il corpo del **mixdown**
  pieno e scuro al suo interno (provato anche come sola linea: l'autore lo preferisce pieno). Su un master limitato la linea dei picchi è quasi piatta, ed è giusto; il corpo conserva
  strofe, ritornelli e pause.
- L'altezza è la radice quadrata del livello: il corpo non si schiaccia al centro e i passaggi piano non spariscono.
- Prima della prima misura si vede solo il mixdown, in grigio.

**Alternative scartate.** Più altezza alla scheda (stesso rettangolo, più alto, e meno spazio al rack); scala in dB dei
soli picchi (resta piatta dove i picchi sono al tetto).

**Tavole.** Le tavole 9 e 15 mostrano ancora master pieno e mixdown grigio: non rigenerate.

**Verifica.** `the_envelope_keeps_peaks_and_body_apart_and_narrows_without_losing_either`; Playwright "the waveform of
a loud master keeps its shape". Sul brano vero: VALIDATION 46.

### A-084 — Intestazioni delle tracce più larghe a richiesta (2026-11-14, issue #9)

**Contesto.** La colonna delle intestazioni nella timeline era fissa a 344 px (304 nel layout compatto): con nomi lunghi
o molti segni della catena non c'era modo di dare più spazio ai nomi. L'issue chiede il ridimensionamento orizzontale
con la larghezza attuale come minimo.

**Scelta.**
- Una maniglia sottile sul bordo destro della colonna (cursore `col-resize`, linea d'accento al passaggio, durante il
  trascinamento e col fuoco), su tutta l'altezza della timeline: righello, sezioni e righe si allargano insieme.
- **Minimo** la larghezza del layout (344 px, 304 nel compatto); **massimo** 640 px e comunque tale da lasciare
  almeno 320 px alle corsie. Se la finestra si stringe, la colonna si riduce fino al minimo e torna alla larghezza
  chiesta quando la finestra si riallarga.
- È una **preferenza dell'utente** (`headW` tra le preferenze dell'interfaccia, nello storage della webview), non del
  progetto: non tocca il documento, quindi niente undo e nessun effetto sul render.
- Doppio clic: torna alla larghezza del layout. Da tastiera (UI-7) la maniglia è un `separator` focalizzabile:
  frecce ±16 px (±64 con Maiusc), Home ed End per minimo e massimo.

**Alternative scartate.** Larghezza nel progetto (è una scelta di vista, non del mix); ridimensionamento dal mixer
(la colonna delle etichette del mixer è un'altra cosa, A-074).

**Tavole.** Non disegnata: la maniglia è invisibile a riposo, le tavole restano valide alla larghezza minima.

**Verifica.** Vitest `the track header widens on request…` (`geometry.test.ts`); Playwright "the track headers widen
by dragging their edge…" (`mix.spec.ts`), anche dopo un ricaricamento.

### A-085 — Effetti del mix e idee creative: come è scritta la specifica (2026-11-14, P-018, deciso dall'autore)

**Contesto.** Dopo un'analisi degli effetti che si potrebbero aggiungere al Mix, l'autore ha scelto di includerli
tutti tranne tre: EQ con più bande e notch stretti, riverbero e delay reverse e freeze, lo-fi e bitcrusher. Ha
incluso anche shimmer, harmonizer e doubler, che FOUNDATION §2 escludeva, e ha lasciato fuori solo il finto doppio.
Ha chiesto di scrivere prima la specifica e poi le tavole, da approvare prima dell'implementazione.

**Scelte prese in autonomia** (da confermare con le tavole):
- **Catena a blocchi in ordine libero, come il master** (FX-10), invece di "posti liberi" dopo la catena fissa. Il
  gate va prima di tutto, il de-esser dopo il compressore e la modulazione in fondo: con posti fissi metà dei blocchi
  starebbe nel posto sbagliato. Si riusa l'architettura di P-017 (lista, latenza costante, cambi senza scatti). I
  cinque classici restano il punto di partenza e la migrazione dallo schema 3 conserva l'hash. Tetti: 2 per tipo e 12
  blocchi per traccia, 6 per ritorno.
- **Doubler e finto doppio.** Il doubler è un effetto dentro la traccia, con voci leggermente stonate e ritardate e
  mescolate col segnale. Il finto doppio escluso è una copia su un'altra traccia, sfasata e pannata, ed è ancora
  fuori perché crea una traccia (AI-17).
- **Clipper di traccia a soglia assoluta** (FX-17): la soglia misurata sul brano del clipper di master (MAS-11) su una
  traccia andrebbe rimisurata a ogni parametro precedente.
- **Latenza uguale per tutte le tracce** e indipendente dalla catena: aggiungere un harmonizer non deve spostare
  una traccia rispetto alle altre. Costa un ritardo fisso della riproduzione di pochi millisecondi.
- **Mono fino al primo blocco stereo**: dopo un chorus o un auto-pan il pan di una traccia mono diventa
  bilanciamento. Rendere stereo tutte le tracce mono costerebbe CPU senza dare nulla.
- **Fase degli oscillatori legata alla posizione nel brano** (FX-20, FX-23), non all'inizio del render come nella
  modulazione del riverbero: un tremolo a tempo deve stare sulla battuta anche partendo da metà brano.
- **Contratto v4 compatto.** Il piano porta solo i blocchi che aggiunge o cambia (il master li elenca tutti), perché
  con 30 tracce la risposta si allungherebbe troppo (A-062). Il modello non sposta e non toglie blocchi: aggiunge in una
  posizione nominata e spegne col bypass. Limiti dell'AI più stretti di quelli dell'interfaccia.
- **Le idee creative sono solo aggiunte**, così che togliere la spunta di un'idea lasci il resto del piano identico,
  come gli elementi nuovi di AI-20. Spunte **spente per default**: un'idea creativa è una scelta di gusto e la fa
  l'autore. Il ciclo valuta il piano senza idee, perché nessuna misura dice se un chorus è riuscito.
- **Catena dei ritorni solo con le modifiche strutturali permesse**: un ritorno è condiviso tra le tracce.
- **Tre fasi** (F15 fondamenta, F16 processori, F17 AI). Le tavole di tutte e tre si disegnano all'inizio di F15 e
  l'implementazione aspetta l'approvazione, come in F13.

**Alternative scartate.** Un blocco "FX" generico con un menu di effetti dentro: si leggerebbe peggio nei segni e
nell'Overview. Idee creative valutate nel ciclo: non c'è una misura. Idee accese per default: l'A/B suonerebbe
qualcosa che l'autore non ha scelto.

### A-086 — Effetti del mix e idee creative: tavole 16a–16u, 4c, 4d, approvate (2026-11-14, P-018)

**Contesto.** P-018 chiede le tavole di tutte e tre le fasi prima dell'implementazione (FOUNDATION §8.1, F15).

**Scelte del design.**
- **La catena sta nell'inspector**, come lista compatta (righe da 24 px) sopra l'editor del blocco scelto. Non ci sono
  colonne nuove né un pannello largo come il rack del master: la catena di una traccia si guarda accanto alla
  timeline. Trim in cima e Fader · Pan in fondo hanno il lucchetto; le mandate scendono a una riga sola in fondo.
  Sostituisce la griglia di pulsanti della catena (3a), che con dodici blocchi non ci starebbe.
- **Ogni riga ha la sua sigla**, la stessa delle intestazioni e delle strip (UI-8): chi legge una catena nel mixer la
  ritrova uguale nell'inspector. Sigle nuove: `GT DS DEQ TR CLP MB EXC BAS WID MOD TRM HRM DBL`. Un blocco con sidechain
  prende il colore della traccia che ascolta, come il compressore di oggi.
- **Il menu "Add block" sta accanto all'inspector**, con quattro colonne (Dynamics, Tone, Space & movement, Pitch). Ogni
  blocco dice a parole cosa fa, e ciò che non si può aggiungere dice perché. Space e Movement stanno in una sola colonna
  perché Space ha un solo blocco.
- **Ogni editor dice cosa fa sul suono**, oltre ai parametri: dove lavora il de-esser, quando lavora l'EQ dinamico,
  dove chiude il gate, il colpo prima e dopo, le armoniche aggiunte, la scala dell'harmonizer. L'harmonizer avverte
  che gli intervalli sono fissi e non seguono la tonalità.
- **I blocchi che suonano solo in alcune sezioni** portano il nome della sezione nella riga (`Choruses`, `Bridge`).
  Quelli aggiunti da una proposta portano `New` finché non è applicata.
- **I segni con catene lunghe** mostrano le prime quattro sigle e un `+n`; il tooltip nomina le altre. Nell'Overview c'è
  una riga per ogni tipo di blocco presente nel progetto, e con molti tipi la griglia scorre.
- **Le idee creative** sono una sotto-sezione della proposta, tre schede affiancate, ciascuna con spunta, motivazione,
  elementi, sezioni e "Listen". La tavola 4c ne mostra una spuntata per far vedere lo stato, ma il default resta spento
  (AI-27).
- **Le due spunte dell'assistente** stanno su una riga: "Allow · structural changes · creative ideas".

**Aperto per l'autore.** La disposizione nell'inspector rispetto a un rack largo come quello del master. Il numero di
blocchi da mostrare nei segni prima del `+n`. Se le idee debbano avere un ascolto anche a proposta applicata.

**Approvazione.** L'autore ha approvato le tavole il 2026-11-14 così come sono: la catena resta nell'inspector, i
segni mostrano quattro sigle prima del `+n`, l'ascolto della sola idea esiste solo con la proposta in revisione.
F15 → F17 si eseguono di seguito.

### A-087 — F15: la catena delle tracce a blocchi, come è fatta (2026-11-14, P-018)

**Dati.** `Chain` = trim, `blocks` (lista di `FxBlock { id, fx }`, `Fx` etichettato per tipo), fader, pan; un ritorno
usa la stessa lista dopo il suo processore (FX-11, al più 6). **L'id di un blocco comincia con la chiave del suo tipo**
(`hpf`, `eq`, `comp2`, `gate`, `deq`…): la migrazione dallo schema 3 dà ai cinque classici gli id `hpf`, `lpf`, `eq`,
`comp`, `sat`, così gli id dei parametri (`eq.2.gain`, `bypass.comp`) e le corsie di automazione restano validi senza
riscriverli, e dal solo id di un parametro si legge il tipo e quindi il suo intervallo. Un parametro di blocco è
`<id>.<chiave>`; un indice di banda o di voce è `<id>.<n>.<chiave>`; il bypass `bypass.<id>`. Schema di progetto **4**.
`Filter` ha la `resonance` (default 1/√2, il Butterworth di prima); `Saturation` il `character`; il riverbero
`shimmer_*`, il delay `mode`, `character`, `wow`, `drive_db` (con i default di serde, usati da F16). Le modifiche di
struttura sono `FxOp` (`Edit::Chain`): aggiungi, togli, sposta, duplica, riporta ai valori iniziali, bande o voci;
togliere un blocco toglie le sue corsie. I valori non continui di un blocco (modi, carattere, voci, sync) passano da
`Edit::SetBlock`, che riceve il blocco intero. Un blocco aggiunto arriva **in bypass** e prima del fader (tavola 16b).

**Hash identico.** I progetti dello schema 3 si rendono con lo stesso hash: verificato da un test (`schema3.rs`) su due
progetti scritti e renderizzati dalla build dello schema 3 (`app/fixtures/schema3/`, catena completa con automazione,
sidechain, gruppo in parallelo, ritorni annidati: un delay che entra in un gruppo che manda a un plate), ai blocchi da
4096, 64 e 777 campioni. Per arrivarci il motore esegue i blocchi classici con lo stesso calcolo di prima: un blocco
del tutto spento si salta, uno del tutto acceso non si miscela, la saturazione gira sempre con il mix a zero quando è
spenta. Anche il **mixdown** resta aggiornato: se l'hash del mix registrato coincide con quello che lo schema 3
calcolava (rifatto con una copia dei tipi di allora), dopo la migrazione si sostituisce con l'hash nuovo.

**Latenza costante, per classe.** Tutte le catene di tracce e gruppi prendono la latenza della più lenta di loro, tutti
i ritorni quella del ritorno più lento; il grafo allinea le classi tra loro come faceva (ENG-5). Aggiungere un blocco
con latenza non sposta mai una traccia rispetto alle altre. Alternativa scartata: una latenza unica anche per i
ritorni. Dare ai ritorni la latenza delle tracce sposta l'ingresso dei riverberi nel tempo del render, e la loro
modulazione, che parte all'inizio del render, cambierebbe fase: addio hash identico. FOUNDATION FX-10 dice "la stessa
per ogni traccia, gruppo e ritorno": è vero dentro ciascuna classe, e tra le classi nessuno si sposta. La latenza
complessiva cambia quando cambia la catena più lenta: il grafo si ricostruisce comunque.

**Bypass con dissolvenza.** Il valore del bypass scorre linearmente in 10 ms per campione, da chiunque venga (mano o
automazione); un bypass fermo dà il calcolo di prima. Unica differenza dallo schema 3: le corsie di bypass passano in
10 ms invece che in 32 campioni (lo chiede FX-10).

**Cambi di struttura all'ascolto.** Un cambio di struttura ricostruisce il grafo e lo scambia con la dissolvenza in
uscita e in entrata che il motore già aveva. **Difetto trovato e corretto**: dopo uno scambio la dissolvenza in entrata
cominciava subito, ma i primi campioni di un grafo azzerato sono il silenzio della sua latenza: il segnale entrava a
metà rampa, con uno scatto di circa 0,1 su una sinusoide di 0,3. Ora la rampa comincia dopo la latenza (test
`structural_changes_while_playing_do_not_click`).

**Mono e stereo.** Una catena mono diventa stereo al primo blocco stereo (larghezza, modulazione, tremolo in auto-pan,
harmonizer, doubler), che sia acceso o spento (è struttura, non stato). Da lì il pan fa da bilanciamento con la legge
del pan mono applicata a ciascun canale: un blocco stereo neutro lascia la traccia identica, anche nel livello al
centro (verificato).

**Sidechain.** Compressore, gate e bande dell'EQ dinamico ascoltano un'altra traccia o un gruppo, anche dalla catena
di un ritorno (ducking). Ogni chiave è allineata al blocco che la usa (latenza dei blocchi prima di lui); il core
rifiuta come prima sorgenti che sono ritorni e cicli.

**Catalogo.** Il core esporta `fx_catalog.json` (sigla, nome, famiglia, blocco nuovo, percorso di ogni parametro
dentro il blocco, trovato provando a cambiarlo): l'interfaccia e il backend finto lo usano per menu, sigle, valori e
parametri, senza ripeterli.

**Interfaccia: scostamenti dalle tavole.**
- Righe del rack: la barra della riduzione di guadagno per blocco non c'è (il motore misura la riduzione per traccia,
  non per blocco).
- Nella vista Overview un tipo di blocco che una traccia non ha si legge "off" come nella tavola 16u, ma non si
  clicca; un ritorno mette filtri, stato e tempo del processore nelle righe che la sua catena lascia vuote.
- Due blocchi dello stesso tipo stanno in una cella sola, col primo e "×2".
- Gli editor dei blocchi nuovi arrivano in F16; in F15 un blocco nuovo mostra una manopola per parametro.

**Verifica.** Core: id, chiavi, limiti, `FxOp`, catalogo, migrazione e mixdown aggiornato. Motore: hash dello schema 3,
dissolvenza di 10 ms, latenza che non sposta le tracce, risonanza (Q 8 = +18 dB alla frequenza), ducking dalla catena
di un ritorno, mono diventato stereo, ordine libero indipendente dal blocco di calcolo, nessuno scatto cambiando
struttura in riproduzione. Interfaccia: 5 test Playwright (`chain.spec.ts`), Vitest su segni, nomi e sezioni; catture
16a, 16b, 16c, 16p, 16t, 16u.

### A-088 — F16: i blocchi nuovi, come sono fatti (2026-11-14, P-018)

**Chi li ha scritti.** I processori di FX-13 → FX-23 sono stati scritti da sei subagenti su file disgiunti del crate DSP
(gate, de-esser e transient; EQ dinamico; caratteri della saturazione, exciter e bass enhancer; modulazione, tremolo e
ampiezza; pitch shifter, harmonizer, doubler e shimmer; delay ping-pong e tape), a partire da un contratto scritto
(firma, parametri, latenza, proprietà di FX-24); l'orchestratore ha rieseguito i test, letto il codice e fatto
l'integrazione nel motore. Ogni algoritmo è descritto in `docs/app/dsp.md`.

**Scelte nei processori** (da sentire nella validazione):
- EQ dinamico: la banda arriva al suo range pieno a 6 dB sopra la soglia; sotto la soglia è esattamente spenta.
- De-esser: rapporto fisso 10; `split` abbassa `x − LP(x)` (complementare esatto): sotto la frequenza il segnale non
  cambia (±0,1 dB a 500 Hz con 6,5 kHz). L'**ascolto del solo segnale rilevato** è uno stato del solo motore
  (`<id>.listen`, comando `listen_block`): non è un parametro del documento, quindi il render offline e l'export non lo
  possono avere; un grafo ricostruito riparte senza, e chiudere l'editor lo spegne.
- Transient: zone morte fisse (1,5 e 0,5 dB) e rampe (10 e 6 dB) sul rapporto tra inviluppo veloce e lento; sotto i
  100 Hz un tono fermo può muoversi di poco.
- Saturazione: `tape` = la curva morbida e un passa-basso a 12 kHz nel ramo saturato; `tube` = polarizzazione e
  passa-alto a 10 Hz; `transistor` = ginocchio più duro. `soft` è il codice di prima, invariato (hash identico).
  Cambiare carattere non ha dissolvenza propria: è un cambio di struttura (scambio del grafo con la sua dissolvenza).
- Pitch shifter: due prese su una linea di ritardo con dissolvenza a coseno; per stare entro ±5 cent, quando una presa
  salta si allinea per correlazione all'altra (una ricerca limitata a ogni salto: un costo a picchi ma limitato, misurato
  dal benchmark). Harmonizer: finestra 35 ms, latenza costante 864 campioni a 48 kHz (18 ms); la presenza di un
  harmonizer porta tutta la classe delle tracce a quella latenza (A-087).
- Doubler: il ritardo che si muove segue la posizione nel brano; lo spostamento d'intonazione no (il suo pitch shifter
  parte dall'inizio del render). FX-20 lo chiede per modulazione e tremolo, e lo rispettano.
- Fase degli LFO: ancorata alla posizione al primo blocco e accumulata per campione: il render che parte a metà brano
  differisce da quello intero di qualche 1e-5 (sotto −80 dB), per l'arrotondamento delle due somme.
- Delay: il drive nel feedback esiste solo nel carattere `tape`; `stereo` + `clean` è il codice di prima, invariato.
- Un blocco con latenza spento non gira: alla riaccensione la dissolvenza di 10 ms copre il suo stato vecchio.

**Interfaccia.** Un editor per ogni blocco nuovo (tavole 16d–16s): un disegno calcolato dalle impostazioni (curva del
gate, zona del de-esser, banda dell'EQ dinamico a riposo e al massimo, colpo prima e dopo, armoniche, immagine stereo,
LFO, scala dell'harmonizer), i controlli, una riga che dice cosa fa. Scostamenti: i disegni non mostrano la riduzione dal
vivo (il motore misura la riduzione per traccia, A-087); il delay e lo shimmer stanno nel pannello del ritorno, con
l'edit `SetReturn` per modo, carattere e intervallo.

**Verifica.** 166 test del crate DSP (proprietà di FX-24 per ogni blocco), `no_alloc` con tutti i blocchi; motore: blocchi
neutri che lasciano il render identico, catena con tutti i blocchi al lavoro indipendente dal blocco di calcolo,
modulazione, tremolo e tape legati alla posizione, ascolto del de-esser; hash dello schema 3 invariato; benchmark
FX-24: p99 27% del periodo (soglia 50%); NFR-1 p99 12,8%; NFR-4 30×. Interfaccia: 3 test Playwright
(`effects.spec.ts`) e catture 16d–16s.

### A-089 — F17: idee creative e blocchi nel piano, come sono fatti (2026-11-14, P-018)

**Misure (ANA-19).** `crates/stemwise-analysis/src/character.rs`: sibilanza delle voci, cresta breve, transienti e
fondo della batteria, calcolati con le altre misure e in cache. La sibilanza confronta la banda 5–10 kHz in frame di
5 ms con il **corpo della voce sotto i 5 kHz** sui 300 ms attorno: la prima versione, che la confrontava con la voce
intera su 50 ms, non poteva superare 0 dB (il corpo conteneva già la banda) e il test di flusso lo ha mostrato. Valori
attesi: circa −20 dB per una voce morbida, 0 e oltre per una sibilante; prompt e decisore finto suggeriscono il
de-esser sopra circa −3 dB. Soglia iniziale, da tarare (VALIDATION 52). L'analisi elenca solo i blocchi non classici
(id neutri `k…`): quelli classici stanno nei campi della v3.

**Piano v4.** Blocchi per traccia, gruppo e ritorno (`blocks`, con `$defs` per tipo e `anyOf` nello schema strict),
risonanze, carattere della saturazione, campi nuovi di riverbero e delay, scostamenti per sezione dei filtri in ottave
e `blocks_on`. Lo scostamento di un filtro agisce sul **primo filtro acceso** di quel tipo, non sul primo in assoluto:
così un'idea può aggiungere un passa-basso risonante suo (il classico resta spento) e muoverlo tra le sezioni senza
cambiare ciò che esiste (AI-26).

**Idee.** `ideas.rs`: `take` le toglie dal piano alla prima iterazione, il ciclo valuta il piano senza idee, `merge` le
rimette alla fine; `strip` tiene solo quelle spuntate al momento dell'applicazione e dell'ascolto. Un ritorno nuovo di
un'idea si crea solo con l'idea spuntata. UI-13: il nome dell'idea (con la motivazione) si aggiunge alla **nota della
traccia** che tocca, che è il testo della motivazione e il tooltip del numero nell'intestazione; i blocchi non portano
un tooltip proprio con il nome dell'idea (servirebbe un campo nel documento per una sola etichetta). Il passo di
cronologia nomina le idee applicate.

**Decisore finto.** De-esser, gate, EQ dinamico chiavato dalla voce, ampiezza, ducking sul riverbero condiviso; idee
`i1` chorus sulle tastiere nei ritornelli, `i2` filtro che si apre verso l'ultimo ritornello (ora con un passa-basso
risonante suo: la prima versione richiedeva un passa-basso già acceso e non scattava mai), `i3` delay ping-pong a nastro
su un ritorno nuovo.

**Interfaccia.** Sotto-sezione "Creative ideas" (tavole 4c, 4d) con spunta, ascolto della sola idea (che conta come
ascolto della proposta nell'A/B) e sezioni; righe "new block" nel diff; "Allow · structural changes · creative ideas"
come nella tavola 3a. CLI: `automix --ideas i1,i3|all`.

**Revisione indipendente** (`app-reviewer` su F15–F17): nessun bloccante, sette difetti corretti.
1. Una revisione perdeva la salita del filtro di un'idea applicata: la rilettura cercava la corsia sul primo filtro, la
   scrittura sul primo acceso. Ora entrambe usano lo stesso filtro (`playing_filter`); coperto in `ideas_flow`.
2. Un campo `null` di un blocco ora tiene il valore anche per bande, voci, sidechain e rate (prima svuotava le bande e
   faceva fallire il piano in strict). In JSON mode si controllano anche quante bande e voci (§5.5).
3. Le iterazioni dopo la prima ricevono le idee decise nel documento (`decided_ideas`) e una riga nel prompt. Alla
   fusione nulla del piano tenuto viene sostituito: un id di blocco già usato si rinomina, gli scostamenti di
   un'idea per una sezione che il piano usa restano a parte (`idea_sections`, fuori dallo schema) e si sommano solo con
   l'idea spuntata, un blocco oltre i tre nuovi per catena resta fuori con un avviso.
4. Le mandate di un'idea, anche dei gruppi, vanno solo ai ritorni ed entro i limiti.
5. Il ritorno nuovo di un'idea non compare più tra gli elementi "will be created": sta nella scheda dell'idea e ne
   segue la spunta (`NewElementView.idea`).
6. In una sezione di un'idea `blocks_on` accetta solo i blocchi dell'idea; gli altri sono tolti con un avviso.
7. Motore: un parametro mosso mentre il grafo vecchio si dissolve andava agli slot del grafo nuovo ma sul vecchio.
   Ora `Msg::Param` porta l'id del grafo e va a quello per cui è stato risolto (anche se è ancora in attesa).
Anche i blocchi nuovi nella stessa posizione mantengono l'ordine del piano. Restano tra le Riserve: FX-24 verificato a
1e-4 e il doubler non legato alla posizione; UI-13 senza tooltip per blocco.

**Verifica.** `tests/ideas_flow.rs` (idee decise una volta, fuori dalla valutazione, applicate solo se spuntate,
annullate in un passo; interruttore spento), test del client contro il server finto con lo schema v4, flusso completo
dalla CLI, Playwright `creative ideas` con le catture 4c e 4d, self-test nell'app impacchettata (49 passi: ascolto di
una sola idea, idea applicata nello stesso passo; avvio a freddo 1,2 s). Il self-test va eseguito con la finestra visibile:
con lo schermo spento il disegno si ferma e l'avvio a freddo risulta di minuti.

### A-090 — Verifica di P-018 dopo la chiusura: tre bloccanti e undici difetti corretti (2026-11-14)

**Contesto.** A fasi chiuse e gate verde, una verifica con tre letture indipendenti (core e motore, DSP, AI e
interfaccia) ha trovato difetti che i test non vedevano. Il primo è stato riprodotto prima di correggerlo; ogni
correzione ha il suo test.

**Bloccanti.**
1. **Render dipendente dal blocco dopo un bypass spento e riacceso** (ENG-4, FX-10). La dissolvenza finisce a metà di
   una cella di 32 e il processore si addormentava "dal pezzo dopo": quanti campioni avesse sentito dipendeva dal
   confine del blocco di calcolo (differenze fino a 2e-3 con blocchi da 12 481 a 12 511). Ora il sonno si decide una
   volta per cella, la miscela è esatta ai due estremi e un blocco che si riaccende riparte pulito (prima rimetteva in
   circolo l'audio di quando si era spento). Alternativa scartata: far finire la dissolvenza su un confine di cella
   (cambia la durata con la frequenza di campionamento e non toglie lo stato vecchio).
2. **Fase di modulazione e tremolo con il rate automatizzato** (FX-20). Dopo un seek la fase era `rate × posizione`,
   in riproduzione continua la somma degli incrementi: con una corsia sul rate, ascolto da metà brano ed export
   differivano. Ora il motore dà al blocco i cicli percorsi dall'inizio del brano, calcolati dalla corsia. Alternative
   scartate: togliere l'automazione del rate (contro AUT-1); ricalcolare `rate × posizione` a ogni cella (un rate che
   si muove farebbe saltare la fase di interi cicli). **Riserva**: un rate cambiato a mano mentre si suona prosegue
   dalla fase accumulata e si riallinea alla partenza successiva.
3. **Il ritorno di un'idea non restava fermo nel ciclo** (AI-26). Dalla seconda iterazione stava nello schema come un
   ritorno qualunque: il modello doveva riscriverlo e alla fusione i suoi parametri vincevano su quelli dell'idea. Ora
   le iterazioni successive ricevono schema, documento e validazione senza quel ritorno.

**Difetti.**
- Core: i limiti di una catena (FX-10, FX-11) valgono anche per `SetTrack`, `InsertTrack` e al caricamento: un
  `project.json` scritto a mano con più di otto chiavi o un multibanda incoerente portava a un panic, il primo sul
  thread audio. `SetBlock` toglie le corsie di bande e voci che il blocco non ha più.
- AI: una risonanza o un blocco messi a mano oltre i limiti del piano (l'interfaccia arriva più in là) non fanno più
  fallire una revisione che li lascia come sono (AI-3, come i campi classici); riapplicare una proposta non duplica
  i blocchi nuovi (AI-23); uno scostamento di sezione di un'idea può stare sulla mandata che l'idea stessa aggiunge
  (il "throw" che il prompt suggerisce); un blocco di un'idea che non ha più posto nella catena resta fuori dalla
  proposta con un avviso invece di essere elencato e poi saltato; due blocchi con lo stesso id in una catena del
  piano sono un errore.
- DSP: gate ed EQ dinamico su una catena mono leggono tutti i canali della chiave; il rivelatore del de-esser ha un
  passa-alto suo; l'exciter non scava più alla frequenza di taglio (passa-banda del dry, `docs/app/dsp.md`); la
  ricerca del pitch shifter ha un costo che non cresce con la frequenza di campionamento.
- Gate: `fx_catalog.json` e `param_specs.json` hanno un test di freschezza (`stemwise-cli/tests/bindings.rs`).

**Lasciato com'è, tra le riserve.** Il **bass enhancer** non ha oversampling né compensazione del livello, contro la
lettera di FX-18: genera armoniche con polinomi fino al quarto grado di una banda sotto i 160 Hz, quindi nulla sopra
640 Hz che un 4x possa proteggere, e aggiungere armoniche al dry alza il livello per costruzione. Il **transient
shaper** su un tono fermo sotto i 100 Hz può muovere il guadagno (già in A-088). In **split** il de-esser abbassa
ancora di circa 0,7 dB un'ottava sotto la frequenza a 12 dB di riduzione (complemento a 6 dB per ottava). Il benchmark
gira solo a 48 kHz. I limiti della catena sono ripetuti in `ui/src/mix/fx.ts`; i test Playwright delle idee provano
il backend finto.

### A-091 — Auto mix con le idee: i segnaposto `null` di `ideas` e `idea_sends_db` si leggono come assenti (2026-11-14, dalla prova dell'autore)

**Contesto.** Su AndrewCole_DeadRoses_Full (33 tracce, `openai/gpt-6.1-sol`) l'auto mix con le idee falliva dopo
tre tentativi con "invalid type: null, expected struct IdeaPlan" (0,52 $, 6 minuti). Le tre risposte erano complete
(`finish_reason: stop`) e rispettavano lo schema strict, che elenca `i1`–`i3` e ammette `null` per un'idea non usata e
per una mandata assente. Il parser accettava `null` nelle mappe di primo livello (`nonnull_map`), ma non in `ideas` e
non nella mappa interna di `idea_sends_db`: i primi due tentativi fallivano su `"i1": {"n1": null, …}`, il terzo (il
modello aveva sostituito quei `null` con −60) su `"i2": null`. Il messaggio di errore rimandato al modello parlava di
un `null` che lo schema dichiara valido, quindi i nuovi tentativi non potevano riuscire. I test non lo vedevano perché
decisore finto e server finto mandavano solo `i1`.

**Alternative.** (a) Togliere `null` dallo schema delle idee: con lo strict ogni chiave è obbligatoria, e il modello
dovrebbe inventare tre idee. (b) Mappa delle idee come array: cambia il contratto `mix_plan` v4. (c) Leggere i `null`
come assenti, come per le altre mappe.

**Scelta: (c).** `ideas` usa `nonnull_opt_map` (una mappa senza valori rimasti equivale a nessuna idea, così non
compare l'avviso delle idee non permesse); `idea_sends_db` usa `nonnull_nested_map`, che toglie i `null` ai due
livelli. Test: `unused_idea_ids_written_as_null_are_absent`, `idea_sends_written_as_null_are_absent_at_both_levels` e il
test del client contro il server finto, ora con `"i2": null, "i3": null`. Rigiocate nel ciclo vero, su una copia
del progetto, le tre risposte registrate diventano ciascuna una proposta al primo tentativo (57 comandi, idea "Soft
Tape Echo"). Per la terza restano quattro avvisi: le mandate a −60 dell'idea toccano mandate già esistenti e vengono
scartate, come previsto da AI-26.

### A-092 — Explain this mix: mappa del segnale e spiegazione a regole, tavole 17a–17j (2026-11-15, P-019, chiesto dall'autore)

**Contesto.** L'autore: l'app è ancora difficile da leggere, per un neofita e per lui stesso; non si capisce chi va
dove, chi fa cosa e perché. I segni di F13 (UI-8 → UI-11) mostrano lo **stato** a chi sa già leggerlo; mancano il
**flusso** (il routing si ricostruisce una traccia alla volta) e l'**intento** (i `rationale` restano nella proposta).
L'autore ha proposto un pulsante "spiegami questo mix", pensando a un modello locale incorporato.

**Alternative.** (a) Solo ritocchi all'esistente (tooltip più ricchi, percorso in una frase nell'inspector): utili ma
non danno la vista d'insieme. (b) Spiegazione scritta da un modello, locale o via OpenRouter: prosa migliore, ma
download e memoria (locale) o costo e conto collegato (remoto), e il rischio che un modello piccolo sbagli il routing.
(c) Mappa del segnale più testo **a regole** sui dati che il progetto ha già.

**Scelta.** (c), deciso con l'autore: per ora **nessun modello**. La proposta è **P-019**: una vista "Explain this
mix" con la mappa a sinistra e il testo a destra, entrambi derivati da documento di progetto e analisi. Tavole in
`design/screens/17-mix-explain.html`, regole in `design/README.md` (sezione "Explain this mix"). Scelte del design:

- **Vista e non modale**: prende il posto dello spazio di lavoro ma lascia viva la barra in alto, così si ascolta
  mentre si legge. `⌘E` era libera nella tabella delle scorciatoie.
- **"Shared effects" accanto a "returns"**: nel testo e nella mappa i ritorni si chiamano con la parola che un
  neofita capisce; "returns" resta come etichetta secondaria.
- **I motivi sono citazioni**: il testo a regole descrive; il perché è solo quello scritto dall'assistente nella
  proposta applicata. Nessun consiglio generato.
- **Mappa per famiglie** (quelle di AI-22) e gruppi ripiegabili, per reggere i progetti da 30 tracce.

**Motivo.** L'80% della spiegazione sono fatti del documento: a regole è esatta, istantanea, offline, uguale a parità
di progetto, e rispetta da sola i principi (niente audio né nomi di file a un modello, niente segreti). Diventa anche
la base di fatti per un'eventuale narrazione con un modello, rimandata.

**Stato.** Tavole approvate dall'autore il 2026-11-15 e implementate in F18 (A-093); FOUNDATION aggiornata (P-019,
ANA-20, UI-14 → UI-16). Dati che l'implementazione dovrà esporre e che oggi l'UI non riceve in questa forma:
loudness per famiglia e per sezione, riduzione media di ogni compressore, clip presenti per sezione.

### A-093 — Explain this mix: scelte dell'implementazione (2026-11-15, F18, P-019)

**Contesto.** Tavole 17a–17j approvate dall'autore il 2026-11-15. Restavano da decidere dove calcolare la spiegazione
e da dove prendere i livelli.

**Scelte.**

- **Mappa e testo derivati nell'interfaccia** dal documento di progetto (`ui/src/mix/explain/`: `model.ts`,
  `layout.ts`, `text.ts`), come i segni di P-013: sono letture per il disegno, non logica di dominio, e così il
  backend finto non le duplica. Le famiglie arrivano dal core (`role_families.json`, generato da `family_of`).
- **Livelli misurati dal core** (ANA-20): `measure_levels` riusa i render del bilanciamento di AI-22 e aggiunge la
  loudness del mix e delle sezioni. Comando `mix_levels`: risponde subito se il mix è già misurato, altrimenti misura
  in un thread e annuncia con l'evento `mix-levels`; una misura su un mix superato si annulla. In memoria, non nel
  progetto. Una misura fallita vale "nessun livello": la parte "Who is in front" non compare.
- **Per famiglia, non per traccia.** Le tavole dicevano "la traccia più forte, 1,4 LU sopra Kick": servirebbe un
  render per traccia (30 render su un brano grande). Il testo usa le famiglie, già misurate.
- **Nessuna riduzione misurata dei compressori.** Le tavole dicevano "about 3 dB on the peaks": la misura non c'è
  fuori dal master. La frase dice rapporto e soglia.
- **`Esc` in due tempi**: prima torna a tutto il mix, poi chiude. Le tavole dicevano solo "chiude".
- **Gruppo che manda a un ritorno**: linea tratteggiata dal piede del gruppo al ritorno sotto; da una colonna più
  profonda la linea torna indietro. Non era nelle tavole.
- **Blocco acceso da una corsia di bypass**: conta acceso nelle sezioni in cui suona, anche se a riposo è spento.
- **Misure della mappa**: le righe da 30 px finché la mappa sta nell'altezza disponibile, poi 26; 22 nella finestra
  compatta. Spazi tra famiglie ridotti rispetto alla tavola (14 px) perché le famiglie di AI-22 sono sei, non cinque.

**Verifica.** 15 test Vitest (mappa, disposizione, sezioni, testo), 6 Playwright (vista, mappa, pieghe e sezioni,
testo e livelli, scheda e copia, accessibilità e finestra minima), 1 test Rust sui livelli. **Non eseguito**: la
vista nell'app Tauri con la misura vera, `make build` e il self-test impacchettato (VALIDATION 55–56).

### A-094 — Tour guidato: specifica di P-020 (2026-11-15, issue #5, deciso con l'autore)

**Contesto.** Issue #5: al primo avvio in assoluto deve partire un tour guidato, saltabile e ripetibile da menu, su un
progetto preparato apposta, incorporato nell'app e non salvato sul disco dell'utente. Analisi fatta in una sessione a
parte dall'implementazione; la specifica è in FOUNDATION (P-020, PRJ-9, UI-17 → UI-19, schermata 18, fase F19).

**Scelte dell'autore** (quattro domande, 2026-11-15).

- **Audio sintetizzato** dal codice, non stem veri nel pacchetto né un tour senza suono: nessun file audio nel
  repository, nessun diritto da gestire, il motore vero suona. Suona sintetico: lo giudica l'autore.
- **Giro completo** (Mix, assistente, proposta, Explain, Master, export) e non il solo Mix né capitoli separati.
- **Guidato con Next**, con l'area illuminata usabile, e non passi che aspettano un'azione.
- **Tavole e approvazione** prima dell'implementazione, come F13, F15 e F18.

**Scelte dell'agente**, da rivedere con le tavole.

- Il motore legge le clip da disco: "non salvato sul disco dell'utente" diventa un pacchetto nella **cache
  dell'app**, eliminato a fine tour e, dopo un crash, all'avvio successivo. Fuori da recenti, autosave, salvataggio
  ed export.
- "Parte automaticamente" è la **scheda di apertura** mostrata da sola, con `Start` e `Skip tour`: il progetto si
  prepara mentre la si legge e l'avvio a freddo non cambia.
- Il tour parte da solo quando **manca il segno "mostrato"**, anche se ci sono progetti recenti: regola unica, e
  l'unica installazione esistente è quella dell'autore. Il segno si scrive al primo passo.
- A richiesta: menu **Help** nuovo nella barra dei menu e collegamento nella Welcome. Con un progetto aperto il tour
  lo chiude (con la solita conferma) e alla fine torna alla Welcome, senza riaprirlo.
- La proposta mostrata è **sempre del decisore finto**, anche con una chiave configurata, e non entra nel registro
  dei costi: il tour non spende e non usa la rete.
- Il progetto dimostrativo è costruito nel **core** (l'UI non duplica logica di dominio); sul backend finto il tour
  usa il brano delle tavole che c'è già. I passi trovano i loro bersagli per ruolo e tipo, non per id.
- Fuori scope (FOUNDATION §2): il progetto dimostrativo apribile fuori dal tour, per tenere la issue com'è scritta
  ("chiuso automaticamente al termine").
- I requisiti nuovi sono assegnati a F19 in `app/scripts/req-phases.json`, così `make check` resta verde finché la
  fase è aperta.

### A-095 — Tour guidato: le tavole della schermata 18 (2026-11-15, F19, da approvare)

**Contesto.** Primo tempo di F19: tavole della schermata 18 e componente del fumetto (FOUNDATION §6, UI-17 → UI-19),
prima di qualunque codice dell'app. La specifica tace su aspetto, posizionamento e diversi stati; scelte dell'agente,
da rivedere con le tavole.

- **Tavole 18a–18w e C8** in `design/screens/18-tour.html` e `design/components/index.html`, generate da
  `design/tools/s-tour.mjs`. Oltre all'elenco di §6: un secondo passo in attesa (mixdown, 18q, oltre alla proposta,
  18p), il tour chiesto con un progetto aperto (18u) e il brano dimostrativo che non si può preparare (18w), perché
  UI-17 e UI-19 li descrivono.
- **Testi definitivi** dei quindici passi in `STEPS` di `s-tour.mjs` (titolo, una o due frasi, una riga `Try it`
  facoltativa sui passi 2, 5 e 8). Alternativa scartata: testi più lunghi con elenchi; il tour deve restare sui tre
  minuti promessi.
- **Il brano delle tavole** è quello di esempio ridotto a 68 s con tre sezioni (Verse, Chorus, Outro) e le dodici
  tracce: dentro gli 8–12 stem di PRJ-9 e con tutto ciò che il tour nomina. Il contenuto vero lo decide il core in
  implementazione.
- **Anello nell'accento di fase**, non in azzurro `--ai`: il tour non è l'assistente (principio 2 del design).
  Attenuazione `--tour-dim` più leggera dello scrim dei dialoghi: il contesto attorno all'area resta leggibile.
- **Posizione del fumetto calcolata da una regola** (lato preferito, poi destra, sinistra, sotto, sopra; dentro
  l'area quando nessun lato basta) e non scritta passo per passo: vale uguale alla finestra minima e sopra 1920 px.
- **Dialogo di export spostato al bordo destro** durante il passo 14, senza il suo scrim: centrato non lascia posto
  al fumetto (340 px a sinistra con 1440, 260 con 1280). Alternative scartate: fumetto sopra il dialogo (ne copre il
  contenuto), illuminare solo i formati (il passo parla anche dei metadata).
- **Explain (passo 10) illuminata per intero** con il fumetto dentro, in basso a sinistra: mappa e testo insieme
  sono il punto del passo.
- **I passi 12 e 13 si dividono catena e destinazione**: la destinazione sta nella scheda Loudness (D7) e un solo
  riquadro che le comprenda entrambe coprirebbe quasi tutto il Master. Il passo 12 la nomina, il 13 la illumina.
- **Un passo imposta solo lo stato che dichiara**; ciò che l'utente cambia nell'area illuminata resta fino alla fine
  del tour. Tornando indietro si disfa solo ciò che il passo successivo aveva aggiunto (da 9 a 8 la proposta torna in
  revisione con un undo).
- **Tour chiesto con un progetto aperto**: la stessa conferma della chiusura (`Save changes to “…”?`) con una riga
  sul perché; `Cancel` non avvia il tour.
- **Collegamento nella Welcome** sotto i due pulsanti, con la durata, su tutte le varianti (1a, 1b, 14). La voce
  `Take the Tour` è spenta mentre l'assistente lavora, durante un bounce o un export; dove sta lo dice A-096.
- **Notifica dopo `Skip tour` ed `Esc`**, non dopo `Done`: chi finisce il giro ha già letto, nella scheda di
  chiusura, dove ritrovarlo.
- Token nuovi in `design/tokens.css`: `--tour-w`, `--tour-card-w`, `--tour-pad`, `--tour-gap`, `--tour-edge`,
  `--tour-arrow`, `--tour-dim` (nelle due fasi), `--z-tour`.
- Il generatore delle tavole accetta un brano diverso in timeline, barra in alto, striscia delle sezioni di Explain e
  forma d'onda del Master; le tavole esistenti sono rimaste identiche, salvo 1a, 1b e 14 che ora mostrano il
  collegamento al tour.

### A-096 — Un solo menu dell'app (2026-11-15, F19, chiesto dall'autore)

**Contesto.** Rivedendo la tavola 18t (menu Help nuovo nella barra dei menu di macOS) l'autore ha notato che i comandi
sono divisi tra la barra dei menu di macOS e il menu che si apre dal nome del progetto (A-053), e ha chiesto di
raccoglierli nel menu dell'app, con Help come voce nuova e con tutte le voci dell'app che oggi stanno nella barra di
sistema. Verifica fatta su `app_menu` in `app/src-tauri/src/main.rs` e su `project/ProjectMenu.tsx`: nella barra di
sistema c'erano, oltre alle voci di macOS, `Settings…` (⌘,) nel menu Stemwise e tutto File (New Project… ⌘N, Open…
⌘O, Close Project, Save ⌘S, Save As… ⇧⌘S); nel menu dell'app solo Project settings, Save, Save as e Close project.

**Scelta (UI-20).** Il menu dell'app contiene, in quest'ordine: `New project…` ⌘N, `Open…` ⌘O · `Project settings…`,
`Save` ⌘S, `Save as…` ⇧⌘S, `Close project` · `Settings…` ⌘, · `Help ›` con `Take the Tour`. La barra di macOS tiene
solo il menu Stemwise di sistema (About, Services, Hide, Hide Others, Show All, Quit), Edit (senza, copia e incolla
nei campi di testo non funzionano nella webview) e Window. Tavola 18t ridisegnata.

**Conseguenze per l'implementazione (secondo tempo di F19).**
- ⌘N e ⌘O oggi passano dalla barra di sistema: entrano nella tabella unica delle scorciatoie (UI-3), come ⌘S, ⇧⌘S e
  ⌘, che ci sono già. Il menu dell'app li mostra accanto alle voci.
- `Settings…` lascia il menu Stemwise di sistema, dove macOS di solito lo mette: scelta dell'autore, ⌘, resta.
- Il test Playwright di A-053 che apre e crea progetti "dalla barra dei menu simulata" passa al menu dell'app; la voce
  di VALIDATION sulla barra nativa diventa la verifica che lì restino solo le voci di sistema.
- Sulla Welcome non c'è il nome del progetto: New project, Open project, Settings e il tour sono già pulsanti e
  collegamenti della schermata.

### A-097 — Tour guidato: l'implementazione (2026-11-15, F19)

**Contesto.** Secondo tempo di F19, dopo l'approvazione delle tavole 18a–18w e di A-096. Scelte dell'agente dove
FOUNDATION e tavole tacciono.

**Core (PRJ-9).**
- Il brano dimostrativo sta in `stemwise_core::demo`: dodici stem sintetizzati da formule chiuse (accordi A, F♯m, D, E a
  120 BPM, rumore SplitMix64 con semi fissi, accumulatori di fase e livelli smussati perché nessun cambio di nota
  faccia click), scritti a 16 bit nel pacchetto e **adottati** come media importati con `media::adopt` (spostati, non
  copiati, nominati dall'hash); il documento è costruito con gli stessi `Edit` e `Command` dell'app. Le dodici sintesi
  girano in parallelo: in release la costruzione sta sotto il secondo (prima della parallelizzazione 4 s, quasi tutti
  per chitarra acustica e Rhodes, ora calcolati con un'esponenziale e un `sin_cos` per nota).
- Il comando `demo` della CLI resta com'era (i suoi test ne dipendono): il brano del tour è un'altra cosa.
- Contenuto oltre PRJ-9: tempo 120/4, sezioni Verse 0–24 s, Chorus 24–52 s, Outro 52–68 s; Gtr Left e Gtr Right sono
  due esecuzioni diverse della stessa parte (doppio vero per ANA-18); un "s" all'inizio di ogni frase della voce dà
  lavoro al de-esser.

**Sessione.**
- Il pacchetto sta in `<cache dell'app>/tour/Demo song.stemwise` (`~/Library/Caches/app.stemwise.desktop`; con
  `STEMWISE_DATA`, in `<dati>/cache`). Si elimina a ogni chiusura del progetto dimostrativo (anche aprendo un altro
  progetto, alla fine della sessione e all'uscita dall'app con `RunEvent::Exit`) e all'avvio.
- Il tour non usa `ai_start`: la proposta del decisore finto si prepara su un suo thread appena il brano è aperto e
  resta da parte, così i passi 2–7 mostrano l'assistente senza proposta. Il passo 8 la mette in revisione
  (`tour_proposal`); se non è pronta l'assistente mostra il lavoro (18p) e la proposta entra da sola. Una copia resta,
  così tornare dal passo 9 all'8 la rimette in revisione dopo l'undo.
- Un tour finito mentre il brano si costruisce lo butta via (contatore `epoch`): niente progetto che si apre dopo Skip.
- Il segno "mostrato" è `tour.json` nella cartella dei dati; il tour parte da solo se manca e se non è il self-test.

**Interfaccia.**
- Il tour sta in `app/ui/src/tour/` (testi in `text.ts`, macchina dei passi in `machine.ts`, posizionamento in
  `place.ts`, il resto in `Tour.tsx`). I bersagli si trovano per nome accessibile e ruolo (`aria-label` delle aree,
  `role="dialog"`, `role="radiogroup"`), ritagliati sulla parte visibile dei pannelli che scorrono.
- I clic fuori dal riflettore si fermano su quattro blocchi attorno al buco; le scorciatoie si fermano in un ascoltatore
  di `keydown` in cattura. Nei campi di testo dell'area illuminata (la richiesta all'assistente) i tasti restano
  dell'utente, tranne `Esc`.
- Il testo "Try it" del passo 8 diceva "press B": `B` è una scorciatoia, sospesa durante il tour (UI-18). Ora dice di
  passare tra Current mix e Proposal; tavole aggiornate.
- Le notifiche non si mostrano durante il tour (stanno sotto il suo livello e parlerebbero del brano dimostrativo, che
  sparisce): sono scartate. Dopo `Skip tour` ed `Esc` il toast di 18v, di tipo nuovo `info`.
- Il menu di sistema perde File e Settings (UI-20): ⌘N e ⌘O entrano nella tabella delle scorciatoie (gruppo
  "Project") e funzionano su ogni schermata; l'evento `menu` e `onMenu` del backend sono tolti.
- Il backend finto ha la stessa interfaccia (`tourInfo`, `tourMarkShown`, `tourStart`, `tourProposal`, `tourEnd`) sul
  brano di sempre, rinominato "Demo song" e senza proposte; il tour parte da solo solo con `?tour`.

**Verifica.** 7 test Rust (`demo` nel core: stesso documento e stessi file; contenuto; render con lo stesso hash nel
motore; nella sessione: cartella nella cache, niente recenti, autosave, salvataggio, export, file rimasti; residuo
eliminato all'avvio; tour finito durante la costruzione; segno "mostrato"; proposta del decisore finto anche con una
chiave e senza costi registrati), 6 Vitest (passi, tasti, posizionamento anche a 1280 × 720), 9 Playwright (primo
avvio, nessun riavvio, tutti i passi avanti e indietro, Skip ed Esc, scorciatoie e clic fuori, menu dell'app con la
domanda di salvataggio, finestra minima, axe e fuoco, brano non preparabile, con catture), 10 passi nel self-test
dell'app vera.

### A-098 — Nessuna modifica dentro un aggiornamento di stato di React (2026-11-15, F19, segnalato dall'autore)

**Contesto.** Il gate stampava `Cannot update a component (Root) while rendering a different component (ChainRack)`
da prima di F19. Causa: il riordino per trascinamento nel rack di traccia (16c) e in quello del Master (15b) mandava il
comando `move` dall'interno della funzione passata a `setDrag`, che React esegue durante il rendering; il comando
aggiornava lo store mentre il rack si disegnava, e in StrictMode poteva partire due volte. La schermata New project
faceva lo stesso con la notifica sul numero massimo di riferimenti.

**Scelta.** Il punto di arrivo del trascinamento sta in una variabile del gesto; a gesto finito lo stato si azzera e il
comando parte una volta, fuori da React. La notifica dei riferimenti si dà prima di aggiornare lo stato.

**Verifica.** Il test Playwright del riordino nel rack di traccia e uno nuovo per il trascinamento nel rack del Master
falliscono se nella console compare quell'errore; senza la correzione falliscono, con la correzione passano.

### A-099 — Tour guidato: verifica a fase chiusa (2026-11-15, F19, chiesta dall'autore)

**Contesto.** Riletto il codice di F19 e provato il tour nel browser sul backend finto. Core e sessione effimera erano
corretti; cinque difetti nell'interfaccia del tour, che i test non vedevano.

**Difetti e correzioni.**
- **Il passo 8 si bloccava** dopo `Apply all` o `Discard` nell'area illuminata: tornava "Preparing the proposal…" con
  `Next` spento per sempre. Ora il passo (e il 9) aspetta solo mentre l'assistente sta davvero lavorando; ciò che
  l'utente fa della proposta non trattiene `Next` (UI-18: nessun passo aspetta un'azione). Il passo 9 la rimette in
  revisione e la applica se serve.
- **`Cancel` durante l'attesa annullava il lavoro del tour**, perché l'assistente mostrava il flag di annullamento del
  lavoro stesso: la proposta non arrivava più. Ora `Cancel` ferma solo l'attesa (`tour_stop_waiting`): il lavoro
  prosegue e un passo successivo la richiede di nuovo. L'attesa dell'interfaccia finisce appena l'assistente smette di
  lavorare senza proposta, invece di restare occupata fino a 120 s.
- **Invio su `Back` o `Skip tour` andava avanti.** Invio su un pulsante col fuoco preme quel pulsante; le frecce su un
  fader o su una scelta dell'area illuminata la regolano. La barra spaziatrice col fuoco sul fumetto va al trasporto
  (prima premeva `Next`, perché il fumetto è un dialogo e le scorciatoie dell'app ignorano i dialoghi).
- **Il dialogo di conferma di `Discard` finiva sotto il tour**, attenuato e in parte coperto dal fumetto. Ora sta
  sopra il tour e ha la tastiera finché è aperto (`Esc` chiude il dialogo, non il tour).
- **Le scorciatoie dell'app non erano sospese da un campo di testo** (⌘, apriva le Impostazioni sotto il tour). In un
  campo ⌘ vale solo per l'editing del testo (⌘A, ⌘C, ⌘V, ⌘X, ⌘Z); le altre aspettano.

**Altre due correzioni.** Il passo 8 riporta il mix allo stato iniziale prima di mostrare la proposta (un fader mosso
nei passi precedenti non la falsa più, UI-18). La costruzione del brano è una alla volta: un tour riavviato mentre il
brano precedente è ancora in costruzione non scrive più nella stessa cartella.

**Verifica.** Un test Rust (`cancel_while_the_tour_waits_stops_the_waiting_and_not_the_proposal`), un test Vitest sui
tasti, tre test Playwright (passo 8 con apply e discard, `Cancel` durante l'attesa, tasti sul controllo col fuoco).
`make check` verde: 116 Playwright, 45 Vitest. Pacchetto ricostruito (`make build`) e self-test nell'app
impacchettata: 59 passi su 59, brano del tour aperto e suonabile in 1,1 s, proposta del decisore finto in 37 s, avvio
a freddo 1,4 s. Il self-test passa dai comandi, non dai clic: i cinque difetti nell'app vera restano da provare a mano
(VALIDATION 60).


### A-100 — Landing page del prodotto, in un repository separato (2026-11-16, issue #4, chiesta dall'autore)

**Contesto.** L'autore ha scelto in canvas la direzione "A · One song, start to finish" tra quattro proposte
(<https://claude.ai/artifact/ErqoyT53SxMCznsQsnYbq3>) e ha approvato il film dell'hero (65 secondi, dodici scene, con
una sua traccia). La landing è stata costruita il 2026-11-15 in `web/`; il giorno dopo l'autore ha deciso che il sito
promozionale non deve stare nel repository pubblico dell'app.

**Scelta.** Il sito vive nel repository privato `pierodetomi/stemwise-promo` (cartella locale
`../stemwise-promo`): pagina statica, film, immagine nginx e configurazione Traefik per il VPS, con il suo README.
Da questo repository sono stati tolti `web/`, l'eccezione di `.gitignore` per la traccia e la riga in README; la
regola "nessun file audio nel repository" resta senza eccezioni.

**Cosa resta qui.** `LICENSE` MIT alla radice, come chiedeva l'issue #4. La PoC in `poc/` dipende da librerie GPL
(`pedalboard`, `matchering`) senza includerne il codice: deciso in A-101. La
landing punta a questo repository (codice, release, licenza sul ramo `main`): i link funzionano solo da quando è
pubblico.


### A-101 — Apertura del repository: piano e scelte dell'autore (2026-11-16)

**Contesto.** Il repository va reso pubblico. Stato trovato: `main` fermo alla sola PoC (10 novembre), il prodotto su
`product/v1`; nessun file audio né `.env` nella storia; metadati di Cargo ancora `UNLICENSED`; nessuna CI né file per
chi contribuisce; la PoC importa `pedalboard` e `matchering` (GPLv3) e il suo `pyproject.toml` dichiara già
`GPL-3.0-or-later`. L'issue #4 è stata eliminata dall'autore perché riguardava il sito, ora in un altro repository.

**Scelte dell'autore.**

1. **PoC**: resta in `poc/` sotto GPL-3.0-or-later, con un suo `LICENSE` e una nota nel README; MIT per tutto il
   resto. Lo script dell'oracolo non usa le due librerie e i fixture sono solo numeri. Alternativa scartata: un
   repository separato (l'oracolo chiederebbe due checkout).
2. **Email nei commit**: resta quella che c'è; la storia non si riscrive.
3. **Lingua**: README, guida per chi contribuisce e pagina di architettura in inglese; FOUNDATION, DECISIONS,
   PROGRESS e VALIDATION restano in italiano, dichiarati come diario di progetto.
4. **Firma**: la prima release esce non firmata, con le istruzioni per il primo avvio nel README.

**Piano.** (1) `product/v1` su `main` e pendenze committate; (2) licenze: MIT nei metadati, `LICENSE` della PoC,
elenco delle licenze di terze parti nel pacchetto; (3) scansione completa dei segreti, rilettura per dati personali,
`CLAUDE.md` aggiornato; (4) README pubblico in inglese con schermate; (5) `CONTRIBUTING`, `SECURITY`, modelli, CI,
issue esistenti tradotte in inglese; (6) release `v0.1.0` con `.dmg` e checksum; (7) impostazioni del repository,
apertura, verifica dei link della landing.

**Passo 2, licenze (fatto).** Metadati MIT (`app/Cargo.toml`, `ui/package.json`, `tauri.conf.json`); `poc/LICENSE`
col testo della GPLv3 e nota nei due README (uniche modifiche alla PoC congelata, volute dall'autore: nessun codice
toccato); `app/scripts/third-party.mjs` scrive `src-tauri/resources/THIRD-PARTY-LICENSES.txt`, l'elenco delle 357
librerie incluse (355 da A-103) (crate collegati per `aarch64-apple-darwin`, pacchetti npm di produzione, font) coi testi delle
licenze, copiato nel pacchetto insieme a `LICENSE`. `make licenses` fallisce se l'elenco non è aggiornato; si
rigenera con `make third-party`. Nessuno strumento nuovo: lo script legge `cargo metadata` e `pnpm licenses`.
Ventitré librerie non hanno un file di licenza proprio nel pacchetto pubblicato: l'elenco le nomina con la loro
licenza. Non c'è una voce di menu che apra l'elenco (UI-20 fissa il menu): il file sta nelle risorse dell'app e nel
repository.

**Nome e logo riservati (scelta dell'autore).** `TRADEMARKS.md` alla radice toglie dalla licenza MIT il nome
Stemwise, il marchio, il lockup e l'icona, coi file che li contengono (`design/brand/`, `app/src-tauri/icons/`, il
favicon, il disegno di `Mark`): dice cosa si può fare senza chiedere, cosa no, e cosa deve cambiare chi pubblica un
fork. `LICENSE` resta il testo MIT puro, perché GitHub lo riconosca. La riserva vale da prima dell'apertura: nessuno
ha ricevuto il repository con il marchio sotto MIT. Non è una registrazione: un marchio registrato è un passo che
può fare solo l'autore.

**Passo 3, pulizia e controlli (fatto).** Storia intera (tutti i rami, ogni diff) cercata per chiavi OpenRouter,
OpenAI, Anthropic, AWS, GitHub, Slack, Google, JWT, chiavi private e assegnazioni di `api_key`/`token`/`password`:
un solo risultato, il valore di prova `sk-or-test-…` dei test. Nessuno scanner dedicato installato (`gitleaks`,
`trufflehog`): all'apertura va acceso il secret scanning di GitHub. File tracciati e storia: nessun percorso della
macchina dell'autore (solo `/Users/demo/…` del backend finto), nessun indirizzo IP o di server, le sole email sono
quelle degli autori nelle licenze di terze parti. Restano, per scelta da confermare, i titoli di due brani
dell'autore nei documenti ("Confidence" nella PoC, "Balance" in PROGRESS e DECISIONS) e due link ad artifact privati
di claude.ai. `CLAUDE.md` aggiornato: sezione "Il repository è pubblico", lingua dei file per chi arriva da fuori,
`main` ramo pubblico.

**Passo 4, README pubblico (fatto).** `README.md` alla radice riscritto in inglese: cosa fa (banco di mix,
assistente, Explain, master, tour), la regola "il modello non riceve audio", installazione con il primo avvio di una
build non firmata, OpenRouter e costi, cosa non fa, compilazione dai sorgenti, mappa del codice e regole, test, come
è stato costruito, licenze, assenza di garanzia, autore. Cinque schermate in `docs/images/` (JPEG a 1600 px, 1,6 MB
in tutto), ricavate dalle catture Playwright sul backend finto: nessun materiale dell'autore. `app/README.md`
riscritto in inglese come mappa per chi sviluppa; `poc/README.md` ha una nota in inglese in testa. Il README dà per
pubblicata una release (pagina delle release, checksum): diventa vero al passo 6. Il collegamento a `CONTRIBUTING.md`
si aggiunge al passo 5.

**Passo 5, comunità e CI (fatto, CI non ancora eseguita).** `CONTRIBUTING.md` (cosa aspettarsi, bug, proposte,
codice, le regole che una modifica deve rispettare, licenza dei contributi, condotta), `SECURITY.md` (segnalazione
privata tramite GitHub, cosa rientra), modelli per bug e proposte, modello di pull request. `README.md` dichiara in
apertura che il progetto è indipendente e non commerciale, senza supporto né tempi di risposta garantiti (chiesto
dall'autore). CI: `.github/workflows/check.yml` esegue `make ci` su `macos-15` a ogni push su `main` e a ogni pull
request. `make ci` è `make check` senza i test dei tempi di frame (`@perf`: un runner ospitato non ha una GPU vera)
e senza il tetto di spazio su disco. **Non provata**: gira solo dopo un push su `main`; finché il repository è
privato consuma i minuti macOS del piano gratuito (contati dieci volte). Le otto issue esistenti e i loro commenti
sono tradotti in inglese su GitHub; il titolo di un brano dell'autore e il rimando alla issue #4 eliminata sono
stati tolti dalla traduzione. All'apertura vanno accese nelle impostazioni la segnalazione privata delle
vulnerabilità e il secret scanning.

**Passo 6, prima release (pacchetto pronto, release da pubblicare).** `make build` dal commit `d783166`:
`Stemwise_0.1.0_aarch64.dmg`, 10,6 MB, SHA-256 `dc1ca447…5118ed3b`. Nel pacchetto ci sono `LICENSE` e
`THIRD-PARTY-LICENSES.txt` (`Contents/Resources`), versione 0.1.0 e nota di copyright. Firma ad hoc del linker,
nessun Developer ID. Self-test nell'app impacchettata su un progetto dimostrativo e una cartella dati temporanei:
59 passi su 59, avvio a freddo 1,4 s. La release `v0.1.0` (tag sul commit costruito, `.dmg`, checksum nelle note) si
crea su GitHub dopo l'assenso dell'autore.

**Release pubblicata.** `v0.1.0` su GitHub il 2026-11-16, col `.dmg` (il checksum riportato da GitHub coincide con
quello locale); nella landing il pulsante di download punta al file.

**Passo 7, apertura (preparato; il repository è ancora privato).** Sito: <https://stemwise.pierodetomi.dev>. Fatto:
descrizione, indirizzo del sito e quattordici argomenti del repository; wiki e progetti spenti; collegamenti a sito,
download e issue in testa al README; immagine di anteprima 1280×640 in `docs/images/social-preview.png` (va caricata
a mano in Settings › Social preview: l'API non lo permette). **Da fare subito dopo l'apertura**, perché il piano
gratuito li concede solo ai repository pubblici: protezione di `main`, segnalazione privata delle vulnerabilità,
secret scanning; poi prima corsa della CI, verifica dei link della landing, deploy.

**Repository aperto (2026-11-16, su richiesta esplicita dell'autore, dopo la sua prova e la pubblicazione del sito).**
`pierodetomi/stemwise` è pubblico. Accesi subito dopo: segnalazione privata delle vulnerabilità, secret scanning con
protezione al push, avvisi sulle dipendenze; `main` protetto da push forzati ed eliminazione, anche per
l'amministratore (nessuna pull request obbligatoria: l'autore lavora da solo). Verificati da fuori, senza accesso:
il sito <https://stemwise.pierodetomi.dev> (pagina, `healthz`, immagine di anteprima), il repository, `LICENSE` su
`main`, la pagina delle release, il `.dmg` (10 602 241 byte, come il file costruito) e la pagina di segnalazione.
Resta all'autore: caricare l'immagine di anteprima in Settings › Social preview.

### A-102 — Due test instabili sotto carico (2026-11-16)

**Contesto.** Durante l'apertura del repository `make check` è fallito tre volte su cinque, ogni volta per un solo
test Playwright, senza modifiche all'interfaccia.

**Tour (UI-18).** `busy` del passo era messo a vero da un effetto, quindi un render dopo il cambio di passo: per un
fotogramma `Next` risultava pronto, e un tasto premuto in quell'istante trovava poi il passo occupato e veniva
ignorato (visto tra il passo 14 e il 15). Ora lo stato si azzera durante il render che mostra il passo: un passo con
una preparazione è occupato dal primo fotogramma. Un utente non poteva accorgersene, se non per una freccia a vuoto.

**Overlay di preparazione del Master (UI-7, MAS-7).** Il test aspettava a intervalli la classe `is-leaving`, che
dura 220 ms: sotto carico la mancava. Ora un osservatore nella pagina, nell'istante in cui l'overlay comincia a
uscire, clicca ciò che sta sopra il pulsante Export (l'overlay, se prendesse ancora il puntatore) e il test verifica
che fosse il pulsante. Il test chiede la stessa cosa di prima, senza dipendere dai tempi del controllo.

### A-103 — La 0.1.0 scaricata risultava "danneggiata": firma del pacchetto, 0.1.1 (2026-11-16)

**Contesto.** Dopo l'apertura l'autore ha scaricato il `.dmg` della 0.1.0 dalla release: macOS ha risposto
«"Stemwise.app" is damaged and can't be opened», senza offrire "Apri comunque". La release era pubblica.

**Causa.** Il pacchetto aveva solo la firma che il linker mette sull'eseguibile, non una firma del bundle
(`codesign --verify --deep --strict`: "code has no resources but signature indicates they must be present"). Per
un'app in quarantena una firma non valida vale come file danneggiato. Né il self-test né `make build` lo vedevano:
un'app costruita in locale non passa da Gatekeeper. La firma non era mai stata verificata, nemmeno in F10.

**Scelta.** `signingIdentity: "-"` in `tauri.conf.json`: firma ad hoc dell'intero bundle, con le risorse sigillate.
`make build` ora esegue `codesign --verify --deep --strict` sul pacchetto e fallisce se la firma non è valida.
Versione 0.1.1; la 0.1.0 va ritirata. Verificato: firma valida; una copia marcata come scaricata è stata aperta
dall'autore con "Apri comunque". La firma ad hoc porta con sé il runtime protetto (`flags=adhoc,runtime`).

**Self-test nel pacchetto 0.1.1.** Cinque esecuzioni: tre complete (59 su 59, avvio a freddo 1,4 s), una con un
passo fallito ("master plays through the chain": nessun segnale sul bus), una rimasta ferma senza finire (processo
in attesa, nessun thread del motore bloccato). **Causa non accertata.** Il Mac era in uso dall'autore durante le
prove; l'ipotesi, non verificata, è che la finestra coperta o il dispositivo audio occupato fermino i tempi
dell'interfaccia. Prima di oggi il self-test non aveva mai dato esiti diversi. Da ripetere a macchina ferma
(VALIDATION 63).

**Ripetuto a Mac fermo (stesso giorno, carico 1,7).** Tre esecuzioni di fila sullo stesso pacchetto 0.1.1: 59 su
59 ogni volta, in 87–88 s, con segnale sul bus del master (−6,3…−7,3 dBFS) e avvio a freddo di 0,6–1,2 s. È
coerente con l'ipotesi del Mac in uso durante le prove precedenti, ma non la dimostra: i due esiti anomali non sono
stati riprodotti, quindi la causa resta non accertata.

**CI, prima corsa.** Su `macos-15`: formattazione, Clippy e tutti i test Rust passano; il gate si fermava
sull'elenco delle licenze, ordinato con un confronto che dipende dalla versione di Node (26 in locale, 22 sulla CI:
i nomi con `_` e `-` cambiavano posto). Ora l'ordine è per unità di codice, uguale ovunque, e il controllo stampa
le righe diverse. I passi dopo le licenze (tipi, lint, Vitest, Playwright) sulla CI non sono ancora mai girati.

**CI, seconda corsa (commit della 0.1.1).** Ordine a posto; restava una differenza di contenuto: `pnpm licenses`
elencava in locale due pacchetti in più (`@types/react` e `csstype`, dipendenze "peer" facoltative: dichiarazioni di
tipi, non incluse nell'app), e sulla CI no, con un'altra versione di pnpm. Lo script ora non chiede più l'elenco a
pnpm: parte dalle `dependencies` dell'interfaccia e le segue in `node_modules`, senza le peer. 355 librerie, lo
stesso numero che la CI calcolava. Il pacchetto 0.1.1 già pubblicato contiene l'elenco precedente, con le due voci
in più: innocuo, si allinea alla prossima release.

**CI verde (commit `8738e13`, 2026-11-16).** La terza corsa è stata annullata da GitHub per mancanza di runner
macOS su Apple Silicon; rilanciata, è passata in 25 minuti: test Rust, licenze (355 librerie), tipi, lint, Vitest
(8 file), Playwright (114 test, 6,3 minuti contro 1,7 in locale, nessuno instabile), requisiti senza buchi. Restano
fuori dalla CI, per scelta, i due test dei tempi di frame e il tetto di disco.

**Tour, la causa vera del test instabile (corregge A-102).** La traccia dei tasti aggiunta al test ha mostrato dove
finiva la freccia: su un pulsante radio. Quando l'overlay di preparazione del Master se ne va e il fuoco non è su
nulla, il Master lo riporta sul selettore Mix/Master; tra due passi del tour il fuoco è su nulla per un istante, e
la freccia successiva andava a quel selettore invece che al tour. Ora, col tour in corso, il Master non prende il
fuoco. Era un difetto vero: capitava anche a mano, cambiando passo mentre il master finiva di prepararsi. La
correzione di A-102 (passo occupato dal primo fotogramma) resta, ma non era la causa. Dopo: i tre file di test otto
volte di fila sotto carico, nessun blocco del tour. Nello stesso giro, un test del Master cercava "Back to Mix"
senza dire in quale overlay: precisato.

### A-104 — I due avvisi di Dependabot (2026-11-16)

**Contesto.** All'apertura GitHub ha segnalato due vulnerabilità "moderate" nelle dipendenze. Nessuna delle due è
nel pacchetto distribuito.

**`minimatch` < 11.0.1 (npm).** Dipendenza del solo linter (eslint e typescript-eslint). Corretto: eslint 10.15.0,
che usa già la 11, e un override in `app/ui/pnpm-workspace.yaml` (pnpm 11 non legge più il campo `pnpm` di
`package.json`) che porta alla 11 anche typescript-eslint, fermo alla 10. Una sola versione nel lockfile, la
11.0.8; lint e tipi passano.

**`glib` < 0.20 (Rust).** Arriva da Tauri solo per Linux (`gtk` 0.18, che fissa `glib` 0.18): su macOS non entra
nella compilazione (`cargo tree -i glib` senza `--target all` non trova nulla). Non si può aggiornare senza che
Tauri passi a un'altra versione di GTK. Non corretto: l'avviso va chiuso su GitHub come "codice vulnerabile non
usato", e riaperto il giorno in cui esistesse una build per Linux.

### A-104 — Windows come secondo sistema: build, gate e pacchetto (2026-11-16, chiesto dall'autore)

**Contesto.** L'app si costruiva e si pacchettizzava solo su macOS. L'autore ha chiesto di renderla multipiattaforma,
partendo da Windows, che è la macchina su cui si lavora. Lo stack era già portabile (P-002): `cpal`, `keyring`,
`symphonia` e Tauri funzionano sui tre sistemi, e il `Cargo.lock` conteneva già i pacchetti per Windows e Linux.
L'inventario ha trovato ciò che legava l'app a macOS: la build, il gate, alcuni difetti a runtime e i testi
dell'interfaccia.

**Difetti veri trovati su Windows.**
- *Salvataggio atomico rotto*: `package::write_atomic` riapriva il file temporaneo in sola lettura per `sync_all`.
  Windows rifiuta di svuotare su disco un file aperto in lettura (`FlushFileBuffers`), quindi falliva ogni
  creazione e salvataggio di progetto. Ora lo svuota lo stesso handle che lo scrive.
- *Save As*: l'interfaccia divideva i percorsi solo su `/` (`tauri.ts`, `NewProject.tsx`, `ExportDialog.tsx`), e
  con i percorsi `C:\…` sbagliava nome e cartella. Ora usa `shared/path.ts` (`baseName`, `parentDir`), con un test.
- *Snap*: nella Timeline lo snap si disattivava solo con `metaKey`, che fuori da macOS è il tasto Windows. Ora vale
  anche Ctrl, come per le altre scorciatoie.
- *CLI*: la cartella dati era `~/Library/Application Support/…` scritta a mano. Ora è `dirs::data_dir()` più
  l'identificatore, la stessa cartella di `app_data_dir` di Tauri su ogni sistema.

**Scelte.**
- *Barra dei menu*: solo su macOS. Su Windows e Linux una barra nella finestra sarebbe un secondo menu dell'app,
  contro UI-20; i campi di testo hanno già copia, incolla e annulla dalla webview. Il codice sta in
  `src-tauri/src/platform.rs`, il terzo modulo isolato di NFR-8, che ora vale per ogni sistema e non solo per
  macOS. Il test di policy normalizza i separatori dei percorsi.
- *AAC su Windows* con Media Foundation (sink writer, contenitore MPEG-4), dietro la stessa interfaccia di
  AudioToolbox; il formato si offre solo se il sistema ha un encoder AAC (le edizioni "N" senza Media Feature Pack
  non lo hanno). L'encoder vuole PCM a 16 bit. La documentazione dice fino a 192 kbit/s; Windows 11 codifica davvero
  a 256 e 320 (misurato dalla dimensione dei file: 258 e 322 kbit/s). Le scelte restano le stesse su tutti i
  sistemi; dove l'encoder rifiuta una velocità sopra 192, l'export ripiega su 192 e il rapporto lo dice
  (`encode_aac` restituisce la velocità usata). Linux resta senza AAC: non c'è un encoder di sistema, e quelli
  disponibili non hanno licenze permissive (A-005).
- *Scorciatoie*: la tabella resta scritta alla maniera del Mac. `keysInText` (`shared/keys.ts`) la mostra come
  `Ctrl+Shift+S` fuori da macOS, in menu, tooltip e Impostazioni. I testi che dicevano "this Mac" e "macOS keychain"
  usano `shared/platform.ts` ("this computer", "Windows Credential Manager"). Le parole dipendono dal sistema, il
  comportamento no. I test Playwright girano con l'user agent di Safari per Mac e restano invariati.
- *Pacchetto*: `tauri.windows.conf.json` produce un installer NSIS per l'utente corrente (senza diritti di
  amministratore), con il bootstrapper di WebView2. L'icona `.ico` c'era già ed era generata dallo stesso
  `source.png`. `make build` verifica la firma con `codesign` solo su macOS. Il pacchetto Windows **non è firmato**:
  SmartScreen avvisa al primo avvio, finché l'autore non ha un certificato Authenticode (stessa situazione della
  0.1.0 su macOS).
- *Licenze*: `make third-party` elenca l'unione delle librerie dei tre target pacchettizzati
  (`aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`): 497 librerie invece di 355, tutte
  con licenze permissive (`cargo deny` verde). Ogni pacchetto può nominare qualche libreria che non contiene, mai
  ometterne una che contiene. Dipendenze nuove: `windows` 0.62 (solo Windows, già nel lockfile tramite Tauri),
  `windows-sys` 0.61 (solo sviluppo, picco di memoria del benchmark), `dirs` 7.0 (già nel lockfile tramite Tauri).
- *Render tra sistemi*: su uno stesso sistema il render resta deterministico, ma non è identico bit per bit tra
  macOS e Windows. `sin` ed `exp` vengono dalla libreria matematica del sistema e possono differire nell'ultimo bit
  (circa −140 dB): perfino i segnali di prova dei test escono diversi. Non si passa a una libreria matematica
  propria: cambierebbe gli hash su macOS e romperebbe la prova di migrazione dello schema 3 (A-087), i cui fixture
  vengono da una build che non esiste più. Quel test gira quindi solo su macOS. Un progetto aperto su un altro
  sistema ha un mixdown da rifare, perché l'hash del mix cambia: si ricalcola da solo.
- *Gate su Windows*: `make` è GNU make (qui `gmake` di Strawberry Perl) dentro una shell bash (Git Bash o Cygwin);
  il Makefile non cambia. Su Windows Playwright usa **Edge** (`channel: "msedge"`) invece del suo WebKit, con
  l'user agent di Safari per Mac: WebView2, con cui l'app arriva all'utente, è il motore di Edge. Il WebKit di
  Playwright per Windows disegna in software: misurava 8 fps sui test di NFR-3, contro 145 e 117 fps su Edge, e
  sotto il carico della suite perdeva a intermittenza i clic sul fumetto del tour mentre si spostava (due corse su
  tre, a 8 come a 4 worker). Con Edge la suite è passata tre volte su tre, 116 test in 1,2 minuti. Su macOS e sulla
  CI resta WebKit; il progetto si chiama ancora `webkit`.
- *Difetto ARIA trovato in passing*: l'elenco dei progetti recenti nella Welcome aveva `aria-label` su un `div`
  senza ruolo, un attributo vietato per ARIA. axe lo vedeva solo quando l'elenco era già caricato al momento della
  scansione, cioè a intermittenza. Ora ha `role="group"`.
- *Self-test*: dipendeva dalla durata del brano. Dopo l'auto mix, su un brano corto la testina era già oltre la fine,
  e due passi ("reverb algorithm changed while playing", "master plays through the chain") misuravano silenzio sul
  bus. Ora quei passi riportano la testina all'inizio. Il comportamento dell'app non cambia. `clean-stale` riconosce
  gli eseguibili `.exe` con i loro `.pdb` e l'installer NSIS.

**Verificato su Windows 11 x64 (2026-11-16).** `make check` verde (116 test Playwright, 162 requisiti senza
buchi). `make build` produce `Stemwise_0.1.1_x64-setup.exe` (7 MB). Self-test nell'eseguibile release con il
dispositivo audio reale (WASAPI, cuffie USB a 48 kHz): **59 su 59**, avvio a freddo 0,8 s, carico 3%. Nello stesso
self-test passano Save As, export WAV, FLAC e AAC a 256 kbit/s sotto il tetto, e il tour col brano sintetico. Il
modulo AAC è controllato con Clippy anche per `aarch64-apple-darwin` e `x86_64-unknown-linux-gnu`. Il resto del
workspace per macOS non si è potuto compilare da qui (alcune dipendenze vogliono un compilatore C per macOS): lo
conferma la CI su `macos-15`.

**CI su Windows (stesso giorno).** Un secondo job, `make ci (Windows)` su `windows-latest`: GNU make da Chocolatey
se manca, i passi in Git Bash, Playwright su Edge, già presente sul runner. È un job a sé e non una matrice, così il
controllo `make ci` su macOS conserva il nome che la protezione di `main` può richiedere. Il nuovo `.gitattributes`
(`* text=auto eol=lf`) impedisce al checkout di Windows di convertire i file in CRLF, che romperebbe i confronti
byte per byte dei file generati. Nell'indice era già tutto LF: nessun file cambia. `make ci` provato in locale in
Git Bash con il PATH pulito: verde, 114 test Playwright. Il job gira alla prima pull request del ramo.

**Non fatto e non verificato.** Linux (pacchetti, fallback della chiave su file quando manca
Secret Service); prova a mano su un Windows pulito (installer, ascolto, trascinamento dei file con lo scaling,
finestra minima di 1280×720 su un portatile al 150%: VALIDATION 64–66); benchmark su Windows; firma Authenticode.

### A-105 — Release 0.1.2 con i pacchetti costruiti dalla CI (2026-11-17, chiesto dall'autore)

**Contesto.** L'autore ha riprovato la build su macOS dopo le modifiche per Windows e ha chiesto di chiudere il ramo
`feature/multi-platform-build` in `main` e di pubblicare la 0.1.2 per macOS e Windows, con il sito allineato. Da un
solo computer non si costruiscono i pacchetti dei due sistemi: il `.dmg` vuole macOS, l'installer NSIS Windows.

**Alternative.** (a) Release in bozza col solo `.dmg` e l'installer costruito e caricato a mano dall'autore; (b) i
pacchetti costruiti dalla CI di GitHub; (c) 0.1.2 solo per macOS. **Scelta dell'autore: (b).**

**Come.** `.github/workflows/release.yml` parte dal push di un tag `vX.Y.Z` (o a mano su un tag esistente). Un primo
job controlla che il tag sia la versione di `tauri.conf.json` e crea la release **in bozza**; due job eseguono
`make build` dal commit del tag su `macos-15` (Apple Silicon, firma ad hoc verificata da `codesign`, A-103) e su
`windows-latest` (installer NSIS per l'utente corrente, non firmato) e allegano il pacchetto; l'ultimo scarica i due
pacchetti e allega `SHA256SUMS.txt`. La bozza si pubblica a mano, dopo aver scritto le note e provato i pacchetti
scaricati: un pacchetto costruito dalla CI non è passato dal self-test. Il ramo entra in `main` in avanti veloce
(`main` era l'antenato diretto).

## Dipendenze aggiunte

| Libreria | Versione | Licenza | Perché | Fase |
| --- | --- | --- | --- | --- |
| tauri, tauri-build | 2.12 / 2.7 | MIT/Apache-2.0 | Shell desktop (P-006) | F0 |
| tauri-plugin-dialog | 2.8 | MIT/Apache-2.0 | Finestre di apertura e salvataggio | F0 |
| serde, serde_json | 1.0 | MIT/Apache-2.0 | Serializzazione dei contratti | F0 |
| react, react-dom | 19.3 | MIT | Interfaccia (§4.1) | F0 |
| zustand | 5.1 | MIT | Specchio dello stato del core | F0 |
| @tauri-apps/api, @tauri-apps/plugin-dialog | 2.12 / 2.8 | MIT/Apache-2.0 | Ponte con il core | F0 |
| @fontsource/figtree, @fontsource/bricolage-grotesque | 5.3 | OFL-1.1 | Font inclusi (A-001) | F0 |
| schemars | 1.2 | MIT | JSON Schema dei contratti (§5.5) | F1 |
| ts-rs | 12.0 | MIT | Tipi TypeScript generati dai tipi Rust | F1 |
| thiserror | 2.0 | MIT/Apache-2.0 | Errori tipizzati | F1 |
| blake3 | 1.8 | CC0-1.0/Apache-2.0 | Hash dei file importati e dello stato di mix | F1 |
| symphonia (wav, flac, aiff, pcm, mp3, aac, isomp4, alac) | 0.6 | MPL-2.0 | Decodifica (§4.1) | F1 |
| rubato, audioadapter-buffers | 5.0 / 5.2 | MIT | Conversione di frequenza (PRJ-5) | F1 |
| clap, anyhow | 4 / 1 | MIT/Apache-2.0 | `stemwise-cli` | F1 |
| proptest, tempfile (sviluppo) | 1.12 / 3.27 | MIT/Apache-2.0 | Test di proprietà, cartelle temporanee | F1 |
| realfft | 3 | MIT/Apache-2.0 | Analizzatore di spettro (FX-7, MAS-4) | F2 |
| cpal | 0.18 | Apache-2.0 | Uscita audio (ENG-2) | F3 |
| rtrb | 0.4 | MIT/Apache-2.0 | Code senza lock tra thread (ENG-1) | F3 |
| rayon | 1.12 | MIT/Apache-2.0 | Render offline e analisi in parallelo (NFR-4, ANA-17) | F3 |
| reqwest (blocking, json, rustls) | 0.13 | MIT/Apache-2.0 | Client OpenRouter (AI-9) | F5 |
| keyring | 4.2 | MIT/Apache-2.0 | Chiave nel portachiavi di sistema (AI-11) | F5 |
| vite, @vitejs/plugin-react, typescript, eslint, typescript-eslint, vitest, jsdom, @testing-library/*, @playwright/test, @tauri-apps/cli | correnti | MIT/Apache-2.0/BSD | Sviluppo e test (non impacchettati) | F0 |
| lofty | 0.25 | MIT/Apache-2.0 | Tag WAV (RIFF INFO, ID3v2), FLAC (Vorbis, immagine), M4A (atomi) (MAS-9) | F8 |
| @axe-core/playwright (e axe-core) | 4.14 | MPL-2.0 | Test di accessibilità (UI-7), solo sviluppo | F9 |
| libc (sviluppo) | 0.2 | MIT/Apache-2.0 | Picco di memoria nel benchmark (NFR-5) | F9 |
| sha2 | 0.10 | MIT/Apache-2.0 | Challenge PKCE del collegamento a OpenRouter (AI-24) | P-015 |
| getrandom | 0.3 | MIT/Apache-2.0 | Verifier PKCE e `state` casuali (AI-24) | P-015 |
| security-framework (solo macOS) | 3.7 | MIT/Apache-2.0 | Sapere se la build è firmata con un certificato Apple (AI-11, A-072); già presente come dipendenza di `keyring` | P-015 |
| tauri-plugin-opener | 2.7 | MIT/Apache-2.0 | Aprire nel browser le pagine di OpenRouter (AI-24) | P-015 |
| windows (solo Windows; Media Foundation, COM) | 0.62 | MIT/Apache-2.0 | Encoder AAC di sistema su Windows (MAS-10, A-104); già nel lockfile tramite Tauri | A-104 |
| windows-sys (sviluppo, solo Windows) | 0.61 | MIT/Apache-2.0 | Picco di memoria del benchmark su Windows (NFR-5, A-104) | A-104 |
| dirs | 7.0 | MIT/Apache-2.0 | Cartella dati della CLI uguale a quella dell'app su ogni sistema (A-104); già nel lockfile tramite Tauri | A-104 |

## Fuori scope segnalato

_Cose servite o richieste dal design che stanno nella colonna destra di FOUNDATION §2: annotate, non
implementate._
