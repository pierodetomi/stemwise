# CLAUDE.md — stemwise

Stemwise è una DAW desktop per mixare e masterizzare brani già registrati con l'assistenza di un modello AI
(Tauri 2 + engine Rust + UI React/TypeScript, in `app/`). La specifica è
[docs/FOUNDATION.md](docs/FOUNDATION.md): **è la fonte di verità** su requisiti, architettura, stack, fasi e
regole. Leggila per intero a inizio sessione. In caso di conflitto: FOUNDATION vince su questo file, che vince
sul design (`design/`), che vince sul PRD della PoC.

Stato corrente (leggilo sempre a inizio sessione):

@docs/app/PROGRESS.md

Decisioni prese in autonomia e riserve: [docs/app/DECISIONS.md](docs/app/DECISIONS.md).

## Il repository è pubblico

Il codice è sotto licenza MIT ([LICENSE](LICENSE)); la PoC in `poc/` è sotto GPL-3.0-or-later perché usa due librerie
GPL; nome e logo di Stemwise sono riservati all'autore ([TRADEMARKS.md](TRADEMARKS.md)). Tutto ciò che entra in un
commit è leggibile da chiunque: niente dati personali, percorsi della macchina dell'autore, nomi di file o brani
reali, indirizzi di server, risposte di modelli registrate su materiale vero. Le librerie incluse nell'app sono
elencate in `app/src-tauri/resources/THIRD-PARTY-LICENSES.txt`: dopo ogni cambio di dipendenze si rigenera con
`make third-party` (il gate licenze lo verifica).

## La PoC è congelata

Il repository contiene anche la PoC Python chiusa con esito go, confinata in `poc/` (codice in `poc/src/stemwise/`,
`poc/tests/`, `poc/schemas/`, con `pyproject.toml` e `Makefile` propri; documenti in `poc/docs/`: `PRD.md`,
`analysis.md`, `DECISIONS.md`, `PROGRESS.md`, `milestones/`). È **in sola lettura**: serve come archivio e come
oracolo numerico (FOUNDATION §5.7). Non si modifica; la si esegue solo per generare i fixture dell'oracolo. Le sue
regole di lavoro sono archiviate in [poc/POC-RULES.md](poc/POC-RULES.md) e **non valgono per il prodotto**: in
particolare non valgono "una milestone alla volta con approvazione", "ambiguità ⇒ fermati e chiedi" e i comandi
`/milestone-start` e `/milestone-close`.

## Principi non negoziabili (FOUNDATION §1)

- **Il modello non riceve mai audio**: solo misure, ruoli, nomi delle tracce, brief, richieste. Mai percorsi,
  nomi di file o titolo del brano.
- **Il documento di progetto è l'unica sorgente di verità**: stesso progetto ⇒ stesso render offline (stesso
  hash). L'AI produce le stesse modifiche che l'utente potrebbe fare a mano.
- **Ogni scelta dell'AI è spiegata e reversibile**: una proposta applicata è un solo passo di undo.
- **Niente è distruttivo**: i file importati sono immutabili; ogni mutazione è un comando con il suo inverso.
- **Mix e master sono due fasi distinte**, con spazi di lavoro e cronologie separati.
- **Thread audio**: nessuna allocazione, nessun lock, nessun I/O. Riproduzione e render offline usano lo stesso
  codice.
- **Solo licenze permissive** (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, Unicode). Nessuna dipendenza
  GPL/LGPL/AGPL: `pedalboard` e `matchering` della PoC non si usano.
- **Segreti**: l'agente non legge `.env`, non chiede né usa chiavi, non chiama modelli reali. L'AI si sviluppa
  col decisore finto e con un server HTTP locale finto.
- **Lingua**: codice, commenti, messaggi e interfaccia in **inglese**; `rationale` e `summary` nella lingua del
  brief; documenti in `docs/` e comunicazione con l'autore in **italiano**. Ciò che legge chi arriva da fuori
  (`README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `TRADEMARKS.md`, issue e pull request) è in **inglese**.

## Modo di lavorare: sessione autonoma (FOUNDATION §9)

1. **Non fermarti e non fare domande** dopo il controllo preliminare, fino alla chiusura dell'ultima fase aperta. Le
   fasi di FOUNDATION §8.1 si eseguono in ordine, una dopo l'altra, senza attendere approvazioni. Le fasi che
   cominciano dalle tavole di design si fermano per l'approvazione dell'autore.
2. **Ambiguità ⇒ decidi e registra.** Scegli l'opzione più coerente con FOUNDATION (principi, poi requisiti, poi
   architettura, poi design), annotala in `docs/app/DECISIONS.md` (`A-nnn`: contesto, alternative, scelta,
   motivo) e prosegui.
3. **Ostacolo ⇒ fino a tre approcci diversi**, poi un ripiego che conserva il requisito, oppure il requisito
   ridotto al minimo utile e registrato tra le "Riserve" di `docs/app/PROGRESS.md`. Mai indebolire un test per
   far passare un gate.
4. **Ti fermi solo per**: un'azione distruttiva o irreversibile fuori dal repository, la necessità di un
   segreto, un conflitto con i principi che nessuna alternativa risolve.
5. **Niente fuori scope** (FOUNDATION §2, colonna destra): si annota tra i "Fuori scope segnalati" di
   `docs/app/DECISIONS.md` e si prosegue.
6. **Dipendenze nuove** ammesse senza chiedere se la licenza è permissiva e la libreria è mantenuta; ciascuna
   va annotata in `docs/app/DECISIONS.md`. Il gate licenze fa parte di `make check`.
7. **Lo stato vive nei file.** A ogni fase chiusa aggiorna `docs/app/PROGRESS.md`, fai il commit e metti il
   tag. Dopo una compattazione del contesto rileggi FOUNDATION, PROGRESS e DECISIONS e riprendi dal primo passo
   non concluso.
8. **Non dichiarare fatto ciò che non hai eseguito.** Quello che non puoi verificare (ascolto, modello reale,
   firma) va in `docs/app/VALIDATION.md` con i passi per l'autore.

## Coordinamento e subagenti

L'agente principale è l'orchestratore: possiede architettura, contratti tra moduli, integrazione, gate e
decisioni, e organizza il lavoro come ritiene meglio.

- **Si delega per ridurre i costi, non per principio.** Delega al subagente `app-implementer` (modello più
  economico) un compito che è insieme **delimitato** (un modulo o pochi file), **specificato** da un contratto
  già scritto (firma, requisiti, test o fixture attesi) e **verificabile** da un comando.
- **Non delegare** quando: il brief costerebbe quanto il lavoro; servono decisioni di architettura o molto
  contesto; il compito tocca file che altri stanno modificando; è integrazione tra moduli; bastano poche
  chiamate a strumenti. Per le ricerche ampie usa il subagente di sola lettura `Explore`.
- **Parallelo solo su file disgiunti.** Ogni brief nomina file, contratto, comando di verifica e ciò che non va
  toccato.
- **Il risultato di un subagente non è una prova**: riesegui il comando di verifica e leggi il diff prima di
  integrare. Dopo due fallimenti sullo stesso compito, il lavoro torna all'orchestratore.
- **Revisione indipendente** con `app-reviewer` solo alla chiusura di F3, F5, F8, F10 e F12.

## Spazio su disco (FOUNDATION §9, regole 18–20)

Il repository, artefatti compresi, resta sotto **20 GB**; sopra **10 GB** si pulisce.

- **Una sola cartella di build** (`app/target`) per orchestratore, subagenti e Tauri: mai un `CARGO_TARGET_DIR`
  alternativo, mai copie del repository o worktree con una propria `target`. Un solo `node_modules`.
- **Misura** con `make disk` a ogni chiusura di fase, dopo ogni `make build` o `make bench`, e almeno ogni due
  ore; annota il valore in `docs/app/PROGRESS.md`.
- **Pulisci** con `make clean-stale` sopra i 10 GB (cache incrementali, profili non in uso, pacchetti e rapporti
  vecchi, cartelle temporanee rimaste, worktree dei subagenti già integrati); `cargo clean` completo al tetto.
- I test eliminano le proprie cartelle temporanee anche quando falliscono; nessun audio renderizzato resta su
  disco.
- La pulizia tocca **solo file ignorati da git dentro il repository** e le cartelle temporanee dei test: mai
  file tracciati, mai `local/`, mai nulla fuori dal repository. È pre-autorizzata.

## Comandi (in `app/`, creati in F0)

```bash
make setup     # toolchain e dipendenze
make dev       # app Tauri in sviluppo
make ui        # UI nel browser con il backend finto
make check     # fmt, clippy -D warnings, test Rust, licenze, tipi, lint, vitest, Playwright, reqs
make test      # solo i test
make bench     # benchmark (NFR-1, NFR-4)
make reqs      # requisiti di FOUNDATION §3 senza test
make oracle    # rigenera app/fixtures/oracle dalla PoC
make build     # pacchetto del sistema: .app/.dmg su macOS, installer NSIS su Windows
make ci        # il gate come lo esegue la CI di GitHub: check senza test dei tempi di frame e senza tetto di disco
make third-party  # rigenera l'elenco delle licenze di terze parti incluso nell'app
make disk      # occupazione del repository per cartella
make clean-stale  # elimina gli artefatti che non servono più
```

`make check` verde è obbligatorio prima di ogni commit.

## Convenzioni

- **Test**: ogni test cita i requisiti che verifica (`/// req: ENG-4` in Rust, `// req: UI-5` in TypeScript).
  Tolleranze esplicite e motivate. Segnali sintetici con seme fisso, progetti di prova in cartelle temporanee.
  Nessun test usa la rete o il dispositivo audio reale. Nessun file audio nel repository.
- **Contratti**: i tipi nascono in Rust; schemi JSON esportati in `app/schemas/` e tipi TypeScript generati a
  ogni modifica. L'UI non duplica logica di dominio.
- **DSP**: ogni processore senza oracolo è documentato in `docs/app/dsp.md` (algoritmo, parametri, proprietà
  verificate).
- **Git**: si lavora sul ramo `product/v1`; `main` è il ramo pubblico e avanza solo su richiesta dell'autore; commit piccoli in inglese col prefisso di fase (`F3: add automation ramps
  (AUT-4)`); tag locale per fase chiusa (`f3`). Mai `push`, `merge` su `main`, `reset --hard`, `rebase` senza
  richiesta esplicita dell'autore.
