# Requisiti senza test automatico dedicato

Ogni id di FOUNDATION §3 deve avere un test che lo cita (§8.3) oppure una voce qui, con il motivo. `make reqs`
legge le righe nella forma `- **ID** — motivo`.

## Verificati da un passo del gate, non da un test

- **NFR-6** — Il gate licenze è un passo di `make check`: `cargo deny check licenses` (elenco ammesso in
  `app/deny.toml`) e `app/scripts/licenses-npm.mjs` (dipendenze di produzione permissive, nessuna copyleft
  nemmeno tra quelle di sviluppo). Un test che rieseguisse gli stessi strumenti non aggiungerebbe nulla.
- **NFR-9** — `app/scripts/disk.sh --check` è l'ultimo passo di `make check` e fallisce sopra i 20 GB; `make
  disk` riporta l'occupazione per cartella.

## Verificati dal benchmark (`make bench`, profilo release)

- **NFR-1** — `app/crates/stemwise-engine/benches/engine.rs`: 32 tracce stereo a 48 kHz con catena completa (filtri,
  4 bande di EQ, compressore, saturazione, mandate a due ritorni, un gruppo), blocchi da 512 campioni; fallisce se il
  tempo medio del callback supera il 30% del periodo. Un test di debug non misurerebbe nulla di significativo.
- **NFR-4** — stesso benchmark: render offline di 16 tracce da 60 s con catena completa; fallisce sotto 20× il tempo
  reale.

Nota: il tempo di ANA-17 (16 stem da 4 minuti in meno di 30 s) è misurato da `crates/stemwise-ai/benches/analysis.rs`
(`make bench`); il test `cache_follows_clip_edits_not_parameters_and_progress_is_reported` verifica il parallelismo e
l'avanzamento visibile.
