---
description: Chiude la milestone attiva (verifiche, revisione indipendente, report, stop per l'autore)
---

Chiudi la milestone attiva indicata in `poc/docs/PROGRESS.md` (stato 🟦). Indicala qui sotto come `Mx`.

1. **Gate automatico.** Esegui `make -C poc check`. Se fallisce, correggi e ripeti: non si chiude con test rossi.
2. **Copertura dei requisiti.** Esegui `make -C poc reqs M=Mx`. Ogni requisito scoperto va coperto con un test
   oppure giustificato per iscritto nel report (es. verificabile solo col modello reale ⇒ test `llm`).
   Verifica che ogni criterio di accettazione di PRD §9 abbia un test `@pytest.mark.acceptance("Mx")`.
3. **Contratti e documenti.** Se la milestone ha toccato modelli Pydantic, rigenera e committa `poc/schemas/`;
   se ha toccato misure, aggiorna `poc/docs/analysis.md` con formule e parametri esatti.
4. **Revisione indipendente.** Lancia il subagent `milestone-reviewer` passandogli solo l'id della
   milestone e il nome del branch (non le tue conclusioni). Per ogni rilievo: correggi, oppure annota nel
   report perché non va corretto. Dopo le correzioni, ripeti il punto 1.
5. **Report.** Compila la sezione "Chiusura" di `poc/docs/milestones/Mx.md`:
   - Fatto, Deciso (con le voci aggiunte a `poc/docs/DECISIONS.md`), Dubbi e rischi, Fuori scope segnalato;
   - Verifica automatica: output sintetico di `make -C poc check` e `make -C poc reqs`;
   - **Validazione manuale**: passi numerati che l'autore può eseguire copiando i comandi, su un brano reale in
     `local/` o su un progetto sintetico, con cosa ascoltare/controllare e l'esito atteso. Se la validazione
     richiede la rete o costa, dillo esplicitamente.
   - Lascia vuota "Esito revisione autore".
6. Aggiorna `poc/docs/PROGRESS.md`: stato 🟨.
7. Commit: `Mx: close milestone (report)`.
8. **Stop.** Riassumi all'autore in italiano: cosa è stato fatto, cosa è stato deciso, cosa resta dubbio,
   e le istruzioni di validazione manuale. Non iniziare la milestone successiva.

Quando l'autore approva esplicitamente in chat: annota l'esito nel report se non l'ha fatto lui, stato ✅ in
PROGRESS.md, commit, poi `git switch main && git merge --no-ff milestone/<branch> && git tag mx`.
