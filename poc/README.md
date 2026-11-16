# stemwise

> **In English.** This is the proof of concept that came before the Stemwise app: a Python command-line tool that
> tested whether a language model can make good mixing decisions from measurements alone. It ended with a "go" and
> is now frozen: it is kept as an archive and as the numeric oracle for the app's tests. Its documents are in
> Italian. It is licensed under [GPL-3.0-or-later](LICENSE), unlike the rest of the repository, which is MIT. The
> app is in [`../app`](../app/README.md).

Proof of concept di un engine di mixing e mastering assistiti da AI: un modello linguistico riceve **solo
misure numeriche** degli stem (mai audio) e restituisce un piano di mix in JSON, che l'engine applica in
locale. Obiettivo: una decisione go/no-go sul costruire lo strumento completo.

> **Il prodotto.** La PoC si è chiusa con un go. Il prodotto, Stemwise (DAW desktop per mix e master assistiti
> da AI), è in [app/](../app/README.md): specifica in [docs/FOUNDATION.md](../docs/FOUNDATION.md), avanzamento in
> [docs/app/PROGRESS.md](../docs/app/PROGRESS.md). Ciò che segue descrive la PoC, ora congelata.

- Specifica: [docs/PRD.md](docs/PRD.md)
- Avanzamento: [docs/PROGRESS.md](docs/PROGRESS.md)
- Decisioni e domande aperte: [docs/DECISIONS.md](docs/DECISIONS.md)
- Istruzioni per l'agente: [POC-RULES.md](POC-RULES.md)

## Requisiti

- macOS o Linux, Python 3.11–3.13, [uv](https://docs.astral.sh/uv/)
- Solo per il decisore reale: una chiave OpenRouter in `OPENROUTER_API_KEY` (vedi `.env.example`)

## Setup

```bash
make setup
make check
```

## Uso

Un brano è una cartella con `project.yaml` (formato in [PRD §6.1](docs/PRD.md); schema in
[schemas/project.schema.json](schemas/project.schema.json)), gli stem e gli eventuali riferimenti. Stem WAV
(PCM 16/24, float 32) o FLAC (16/24), mono o stereo, tutti alla stessa frequenza; `sample_rate: auto` la
ricava dagli stem.

```bash
uv run stemwise mix <cartella-del-brano> --baseline-only
```

Scrive la somma grezza degli stem (`baseline.wav`, WAV 24 bit stereo, true peak ≤ −3 dBTP) in una nuova
cartella `runs/<data-ora>/` del brano.

```bash
uv run stemwise analyze <cartella-del-brano>
```

Misura stem, somma grezza e riferimenti e scrive `analysis.json` in una nuova cartella `runs/<data-ora>/`:
loudness, picchi, dinamica, energia in 8 bande, centroide, transienti, stereo, igiene per ogni stem; le stesse
misure sulla somma; quote per banda, le 10 collisioni di mascheramento più forti e il profilo medio dei
riferimenti. Contratto in [schemas/analysis.schema.json](schemas/analysis.schema.json), formule in
[docs/analysis.md](docs/analysis.md).

```bash
uv run stemwise mix <cartella-del-brano> [--decider llm|mock] [--max-iter 1-3] [--model <id>] \
    [--temperature <t>] [--reasoning-effort low|medium|high|xhigh] [--export-stems]
```

Il ciclo di verifica: analisi e somma grezza, poi fino a 3 iterazioni di decisione, render e valutazione
(`--max-iter` prevale su `options.max_iterations` di `project.yaml`). Scrive in una nuova cartella
`runs/<data-ora>/`:

- `analysis.json`, `baseline.wav` e `baseline_evaluation.json` (la somma grezza valutata come termine di
  confronto);
- per ogni iterazione `iter_NN/` con `plan.json`, `mix.wav`, `evaluation.json` (distanza spettrale dal profilo
  dei riferimenti, scarti per banda, di ampiezza stereo e di dinamica, collisioni residue, vincoli tecnici,
  misure del mix), `llm_request.json` (tutto ciò che è stato inviato, tentativo per tentativo) e
  `llm_response.json` (risposte, token, costo, durata);
- `best.json` (l'iterazione migliore: distanza minima tra quelle che rispettano i vincoli) e `report.md`
  (tabella della somma grezza e di ogni iterazione, motivazioni del piano migliore, collisioni, costo).

Il ciclo si ferma quando la distanza scende sotto 1,5 dB, quando migliora meno di 0,2 dB sull'iterazione
precedente o al massimo delle iterazioni; senza riferimenti, alla prima iterazione che rispetta i vincoli.
Dalla seconda iterazione il modello riceve le ultime 2 iterazioni con le loro valutazioni; gain e bande di EQ
non possono cambiare più di ±4 e ±3 dB da un'iterazione all'altra. Al modello vanno solo misure, id, ruoli e
note degli stem e il brief: niente audio, nomi di file o titolo. Se un piano è invalido viene rimandato al
modello con gli errori fino a 2 volte; se fallisce anche così alla prima iterazione, il comando esce con errore
e la cartella del run resta con i log; a un'iterazione successiva il ciclo si ferma e tiene il risultato. Se
nessuna iterazione rispetta i vincoli tecnici il comando esce con errore (i file restano). Contratti in
[schemas/evaluation.schema.json](schemas/evaluation.schema.json) e [schemas/best.schema.json](schemas/best.schema.json),
formule in [docs/analysis.md](docs/analysis.md), sezione "Valutazione".

- `--decider llm` (default): il modello reale via OpenRouter. Configurazione da variabili d'ambiente o da
  `.env` nella cartella corrente (modello in `.env.example`): `OPENROUTER_API_KEY`, `STEMWISE_MODEL`
  (obbligatoria, nessun modello di default nel codice), `STEMWISE_REASONING_EFFORT`, `STEMWISE_TEMPERATURE`
  (default 0,2), `STEMWISE_BASE_URL`. Le opzioni della riga di comando vincono. Temperatura, seed e sforzo di
  ragionamento si mandano solo se il modello li accetta (altrimenti un warning).
- `--decider mock`: regole fisse per ruolo, senza rete, deterministico; dalla seconda iterazione corregge con
  un'EQ la banda più lontana dal target. Scrive comunque `llm_request.json` con i messaggi che il modello
  riceverebbe, così il prompt si legge gratis.

```bash
uv run stemwise mix <cartella-del-brano> --plan <mix_plan.json> [--export-stems]
```

Applica e valuta un piano di mix scritto a mano, senza il modello (formato in [PRD §6.3](docs/PRD.md); schema
in [schemas/mix_plan.schema.json](schemas/mix_plan.schema.json)): gli stessi file di `mix` con una sola
iterazione `iter_00/`. Con `--export-stems` anche gli stem processati in `iter_NN/stems/` e il ritorno del
riverbero in `iter_NN/reverb_return.wav`. Catena e formule in [docs/analysis.md](docs/analysis.md), sezione
"Render".

```bash
uv run stemwise report <cartella-del-brano> [--run <cartella-del-run>]
```

Rigenera `report.md` dai soli file del run, anche di una revisione: l'ultimo run con `best.json`, oppure quello
indicato (nome della cartella in `runs/` o percorso).

```bash
uv run stemwise revise <cartella-del-brano> "<richiesta>" [--run <cartella-del-run>] [--decider llm|mock] \
    [--model <id>] [--temperature <t>] [--reasoning-effort low|medium|high|xhigh] [--export-stems]
```

Una revisione a parole (per esempio `"voce più avanti, meno riverbero sulle chitarre"`, da 1 a 1.000 caratteri):
una sola iterazione che parte dal piano migliore dell'ultimo run (quello di `best.json`; dopo un `mix`, il piano
migliore del mix) oppure del run indicato con `--run`, con la richiesta come priorità. Scrive un nuovo run e non
tocca quello di partenza:

- `revision.json` (run e piano di partenza, richiesta), `source_plan.json` e `source_evaluation.json` (copie del
  piano di partenza e della sua valutazione);
- `analysis.json`, `baseline.wav` e `baseline_evaluation.json` copiati dal run di partenza: l'analisi non si rifà,
  si rimisurano solo i riferimenti. Se nel frattempo stem o riferimenti sono cambiati (id, ruoli, canali, durata,
  picco, profilo dei riferimenti) il comando si ferma e chiede un nuovo `mix`. Le `options` di `project.yaml`
  (`pre_normalize`, `mix_true_peak_db`) non sono salvate nel run e non vengono controllate: se le cambi, rifai
  `mix` prima di rivedere, altrimenti il diff non descrive tutto il cambio;
- `iter_00/` come per `mix`, `best.json` e `report.md` con il confronto somma grezza · piano di partenza · piano
  rivisto, il **diff dei parametri** e le motivazioni degli stem cambiati.

Il modello riceve il piano di partenza con la sua valutazione e la richiesta; i limiti di variazione (±4 dB di
gain, ±3 dB per banda di EQ) valgono rispetto al piano di partenza, quindi un cambio grande può chiedere due
revisioni. Non c'è rollback: il piano rivisto è il "migliore" del nuovo run qualunque sia la distanza dal
riferimento, purché rispetti i vincoli tecnici; se ne viola uno il comando esce con errore e i file restano per
l'ascolto. Poiché la revisione diventa l'ultimo run, una `revise` dopo l'altra parte dal piano già rivisto; per
ripartire da un piano precedente si usa `--run`. Col decisore finto la richiesta si interpreta con poche parole
chiave (soggetto: voce, basso, chitarre, batteria…; direzione: più avanti/indietro, alza/abbassa, più/meno
riverbero). Contratto in [schemas/revision.schema.json](schemas/revision.schema.json).

```bash
uv run stemwise master <cartella-del-brano> [--run <cartella-del-run>] [--mix <file>] \
    [--reference ref1|ref2|ref3|<file>] [--pcm16]
```

Il master per il confronto con un riferimento (obbligatorio): il mix viene portato su loudness e spettro medio
del riferimento con Matchering, poi passa dal suo limiter. Non usa il modello. Di default prende il `mix.wav` del
piano migliore dell'ultimo run (anche di una revisione) e il primo riferimento di `project.yaml`; `--run` sceglie
un altro run, `--reference` un altro riferimento (`ref2`, `ref3` o un file), `--mix` un file qualunque (per
esempio un tuo mix fatto a mano) alla frequenza del progetto. Scrive:

- `master/master.wav` (WAV 24 bit, alla frequenza del progetto) dentro il run; un secondo master dello stesso run
  va in `master_02/`, poi `master_03/`…: nessun file esistente cambia. Con `--mix` si crea un run nuovo che
  contiene solo `master/` e che non diventa "ultimo run" per `report` e `revise`;
- con `--pcm16` anche `master_16.wav` (16 bit, senza dither);
- `master_report.json`: loudness e true peak del master, del mix di partenza e del riferimento, distanza spettrale
  dal riferimento usato prima e dopo il master. Contratto in
  [schemas/master_report.schema.json](schemas/master_report.schema.json).

Se il true peak supera −1 dBTP il comando lo segnala con un warning ma non corregge: il limiter di Matchering
lavora sul picco di campione, quindi con riferimenti commerciali forti il warning è normale. Il master serve ad
ascoltare il mix contro il riferimento a livello e tonalità simili, non è un master finale.

Il comando `ab` (ascolto cieco, REP-2) non è stato implementato: l'autore ha chiuso la PoC senza il protocollo di
ascolto cieco (D-079). Il comando esiste e risponde "not implemented".

## Sviluppo con Claude Code

Lo sviluppo procede per milestone (PRD §9), una alla volta, ciascuna con due checkpoint con l'autore:

1. `/milestone-start M0` — l'agente legge il PRD, scrive il piano in `docs/milestones/M0.md` e si ferma
   per l'ok e per le domande aperte.
2. Sviluppo a passi piccoli, ognuno con test e `make check` verde.
3. `/milestone-close` — verifiche automatiche, revisione indipendente (subagent `milestone-reviewer`), report
   con istruzioni di validazione manuale. L'agente si ferma.
4. L'autore valida, annota l'esito nel report e approva: merge su `main` e tag della milestone.

Conviene una sessione Claude Code nuova per ogni milestone: lo stato vive nei file in `docs/`, non nella chat.

Il materiale audio reale per la validazione manuale va in `local/` (ignorata da git).

## Licenze

La PoC è distribuita sotto [GPL-3.0-or-later](LICENSE), perché dipende da `pedalboard` e `matchering`, che sono
GPLv3. Il resto del repository, prodotto compreso, è sotto licenza MIT e non usa né la PoC né queste due librerie:
dalla PoC prende solo i numeri dell'oracolo (`app/fixtures/oracle`), calcolati con NumPy, SciPy e librosa.

## Esito della PoC

**Decisione: go** (2026-11-10, D-080). Si passa allo sviluppo del prodotto.

Domanda della PoC: un modello che riceve solo misure numeriche degli stem sa decidere parametri di mix che suonano
meglio della somma grezza? Sul materiale provato, sì. Il modello di default è `openai/gpt-6.1-sol` a sforzo medio
(D-047).

### Ipotesi (PRD §2)

| Ipotesi | Soglia | Misurato | Esito |
| --- | --- | --- | --- |
| H1. Le misure bastano al modello per decidere | 100% di piani validi entro 2 nuovi tentativi | 8 chiamate reali su "Confidence" (13 stem, 3 modelli) e i test `llm` sintetici: tutte valide al primo tentativo | ✅ |
| H2. Il ciclo di verifica converge | Distanza in calo tra prima e migliore iterazione su ≥ 2 brani su 3 | "Confidence": Sol 2,1 → 0,7 dB, Terra 1,7 → 1,2 dB (somma grezza 3,7 dB) | ✅ su 1 brano |
| H3. Suona meglio della somma grezza | Mix AI preferito in ≥ 2 brani su 3, ascolto cieco a pari loudness | "Confidence": il mix migliore si distingue in meglio da somma grezza e prima iterazione; master giudicato senza problemi | ✅ secondo l'autore, ascolto non cieco |
| H4. Le richieste a parole producono il cambio chiesto | ≥ 4 su 5 nella direzione giusta | 3 su 3 (una sintetica, due reali su "Confidence") | ✅ su 3 richieste |
| H5. Il costo è trascurabile | < 0,20 USD per brano | ~0,22 USD: `mix` a 2 iterazioni 0,11, due `revise` 0,06 + 0,05 | ✅ con lo scostamento accettato (D-068) |

Vincoli tecnici: i mix restano entro −3 dBTP senza limiter e il render è deterministico (stesso piano e stessi stem
⇒ stesso hash), entrambi coperti dai test automatici.

### Limiti della verifica

La PoC si è chiusa senza il protocollo di ascolto di PRD §10.2 (D-079): un solo brano invece di tre, ascolto non
cieco e non a pari loudness, 3 richieste a parole invece di 5. Lo sbilanciamento di livello giocava contro il mix AI
(la somma grezza era circa 4 dB più forte, −17,3 contro −21,2 LUFS) che è stato comunque preferito. Il confronto con
un mix fatto a mano dall'autore non è stato fatto.

### Indicazioni per il prodotto

- **Parametri per sezione (automazioni): must-have** (D-048). Con parametri statici le chitarre distorte del chorus
  di "Confidence" restano poco udibili. Insieme alle sezioni conviene dare al modello misure nel tempo compatte, per
  esempio il loudness di ogni stem per sezione (mai audio).
- **Master con parametri scelti e motivati dal modello** (D-077): quanto imitare il riferimento o come pesarne più
  d'uno, loudness e true peak secondo la destinazione, ampiezza e compressione secondo genere e mood. Valore atteso
  minore che nel mix. Il limiter di Matchering guarda il picco di campione, quindi il master supera spesso −1 dBTP
  (D-074).
- **La distanza spettrale non misura la qualità**: il decisore finto ottiene distanze minori del primo piano del
  modello. Va bene come guida del ciclo, non come giudizio.
- **Batteria in molti stem**: le collisioni più forti sono quasi sempre tra i microfoni della stessa batteria e
  coprono quelle utili. Serve un raggruppamento per famiglia di strumenti nelle misure e nel prompt.
- **Compressore per canale** (D-034): su stem stereo sbilanciati l'immagine può spostarsi. Serve un compressore
  collegato, fuori da `pedalboard`.
- **Tempo di calcolo**: 3–7 minuti per un `mix` di 13 stem, di cui ~1,5 di chiamate al modello. Da ottimizzare
  (spettri calcolati una volta sola, parallelismo).
- **Piccole cose**: l'avviso sulla `temperature` non supportata si ripete a ogni chiamata; manca un ripiego se un
  provider rifiuta lo schema `strict` (Q-40); va confrontato un mix AI con un mix manuale, a pari loudness.
- **Portabilità** (PRD §7.1): contratti in `schemas/` e formule in `docs/analysis.md` sopravvivono a un cambio di
  linguaggio; un port della catena di render va rivalidato all'ascolto.
