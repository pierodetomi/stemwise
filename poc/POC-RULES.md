# CLAUDE.md — stemwise

PoC di un engine Python, guidato da CLI, che fa decidere a un modello linguistico i parametri di mix
partendo **solo da misure numeriche** degli stem. La specifica completa è in [docs/PRD.md](docs/PRD.md):
è la fonte di verità su requisiti, contratti dati, milestone e criteri di accettazione. In caso di
conflitto tra questo file e il PRD, vince il PRD; segnala il conflitto all'autore.

Nome del pacchetto e del comando: **`stemwise`** (D-006). Dove il PRD scrive `mixpoc` (nome provvisorio)
si intende `stemwise`: modulo `src/stemwise/`, comando `stemwise <analyze|mix|revise|master|report|ab>`.

Stato corrente delle milestone (leggilo sempre a inizio sessione):

@docs/PROGRESS.md

## Principi non negoziabili (PRD §1, §7)

- **Il modello non riceve mai audio.** Al modello vanno solo misure, id e ruoli degli stem, brief.
- **Il piano di mix è l'unica sorgente di verità.** Stesso piano + stessi stem ⇒ stesso file (stesso hash).
- **Ogni scelta è spiegata** (`rationale` per stem, `summary` per il piano).
- **Funzioni pure**: analisi, render e valutazione senza stato globale, testabili con array numpy in memoria.
- **Engine senza terminale**: niente `input()` né prompt interattivi; la CLI (`typer`) è un involucro sottile
  sopra funzioni che accettano percorsi e restituiscono dati.
- **Contratti neutri**: i JSON di scambio non contengono nomi di librerie né oggetti serializzati. Gli schemi
  nascono come modelli Pydantic e vengono esportati in `schemas/` a ogni modifica.
- **Formule esplicite**: ogni misura di analisi è documentata in `docs/analysis.md` (formula, finestra,
  STFT size/hop, limiti di banda, soglie, versione dell'algoritmo).
- **Determinismo**: nessuna casualità non seminata, nessuna dipendenza dall'ora nel contenuto dei file audio.
- **Vincoli tecnici sempre validi**: mix ≤ −3 dBTP senza limiter; render deterministico.
- **Lingua**: codice, commenti, docstring e messaggi CLI in **inglese**. Il campo `rationale` nella lingua del
  brief. Documentazione di processo (`docs/`, report di milestone) e comunicazione con l'autore in **italiano**.
- **Segreti**: la chiave è in `OPENROUTER_API_KEY` (env / `.env`, mai committato, mai letto dall'agente).
- Un run non modifica mai `stems/` e `references/` e non sovrascrive run precedenti.

## Regole di lavoro (PRD §9)

1. **Una milestone alla volta**, nell'ordine di `docs/PROGRESS.md`. Non iniziare la successiva senza
   l'approvazione esplicita dell'autore.
2. **Niente fuori scope** (colonna destra di PRD §3), nemmeno "già che ci siamo". Se serve, annotalo nel report
   di milestone alla voce "Fuori scope segnalato" e prosegui senza implementarlo.
3. **Ambiguità ⇒ fermati e chiedi.** Se un requisito è ambiguo o una libreria non fa ciò che serve, non
   aggirare il problema: registra la domanda in `docs/DECISIONS.md` (sezione "Domande aperte") e chiedi.
4. **Nessun test chiama la rete.** `tests/conftest.py` blocca i socket; i test col modello reale hanno
   `@pytest.mark.llm` e non girano di default.
5. **Nessun file audio nel repository.** I test generano segnali sintetici (seni, rumore rosa, impulsi,
   silenzio). Il materiale reale dell'autore sta in `local/` (ignorata da git) o fuori dal repo.
6. **Dipendenze**: solo lo stack di PRD §7. Una libreria nuova richiede l'ok dell'autore (`uv add` non è
   pre-autorizzato).

## Flusso di una milestone

Il dettaglio operativo è nei comandi `/milestone-start` e `/milestone-close` (`.claude/commands/`).

1. **Avvio** — `/milestone-start Mx`: branch `milestone/mx-<slug>`, lettura delle sezioni PRD pertinenti,
   piano in `docs/milestones/Mx.md` (requisiti → test previsti, passi, ambiguità). **Checkpoint**: presenta il
   piano e attendi l'ok dell'autore (o le risposte alle domande aperte) prima di scrivere codice.
2. **Sviluppo a passi piccoli** — per ogni passo: test prima (valori attesi noti + tolleranze esplicite),
   implementazione, `make check` verde, commit. Mai commit con test rossi.
3. **Chiusura** — `/milestone-close`: `make check`, `make reqs M=Mx`, revisione indipendente con il subagent
   `milestone-reviewer`, report di chiusura in `docs/milestones/Mx.md` (fatto, deciso, dubbi, fuori scope,
   **istruzioni di validazione manuale**), aggiornamento di `docs/PROGRESS.md`. Commit. **Stop**.
4. **Revisione dell'autore** — l'autore esegue la validazione manuale e annota l'esito nel report. Solo dopo
   un'approvazione esplicita in chat: `git merge --no-ff` su `main` e tag `mx`.

### Definition of Done di una milestone

- Tutti i requisiti della milestone implementati, ciascuno con almeno un test marcato `@pytest.mark.req("ID")`
  (o una giustificazione scritta nel report, es. verificabile solo con il modello reale).
- Ogni criterio di accettazione del PRD coperto da un test `@pytest.mark.acceptance("Mx")`.
- `make check` verde (format, lint, test). `make reqs M=Mx` senza buchi non giustificati.
- `schemas/` e `docs/analysis.md` aggiornati se toccati contratti o misure.
- Report di milestone completo, con istruzioni di validazione manuale eseguibili copiando i comandi.

## Comandi

```bash
make setup            # uv sync (crea .venv con le dipendenze dev)
make fmt              # ruff format + fix automatici
make check            # format check + lint + test (gate obbligatorio prima di ogni commit)
make test             # solo pytest (esclude i test marcati llm)
make test-llm         # test col modello reale: richiede OPENROUTER_API_KEY, usa la rete, costa
make reqs M=M1        # copertura dei requisiti PRD della milestone nei test
uv run pytest tests/test_x.py -k nome   # singolo test
```

## Convenzioni di test

- Un file di test per modulo (`tests/analysis/test_stem.py` ↔ `src/stemwise/analysis/stem.py`).
- Marker obbligatori: `@pytest.mark.req("ANA-3")` collega il test al requisito (anche più id);
  `@pytest.mark.acceptance("M1")` per i criteri di accettazione della milestone.
- Tolleranze sempre esplicite e motivate (`np.testing.assert_allclose(..., atol=0.1)  # ±0.1 LU da PRD M2`).
- Segnali sintetici e progetti di prova generati in `tmp_path`, con seed fissi. Helper condivisi in
  `tests/helpers/` (generatori di segnali, progetto sintetico).
- Il decisore finto (`decision/mock.py`) è l'unico decisore usato nei test del flusso completo.

## Architettura (PRD §4, §8)

Ciclo: **analisi → decisione → render → valutazione** (fino a 3 iterazioni), poi master dal piano migliore.
Struttura dei moduli in `src/stemwise/` come da PRD §8; ogni modulo legge/scrive JSON ed è eseguibile da solo.
Il codice dipende dall'interfaccia `Decider` (implementazioni: LLM via OpenRouter, mock deterministico).

## Git

- `main` contiene solo milestone approvate. Lavoro su `milestone/mx-<slug>`.
- Commit piccoli, messaggio in inglese, prefisso con la milestone: `M1: add band energy measure (ANA-3)`.
- Mai `push`, `merge`, `reset --hard`, `rebase` senza richiesta esplicita dell'autore.
