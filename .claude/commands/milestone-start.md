---
description: Avvia una milestone della PoC (piano + checkpoint con l'autore, nessun codice)
argument-hint: <M0..M7>
---

Avvia la milestone **$ARGUMENTS**. In questa fase **non scrivi codice di produzione**: produci il piano e ti
fermi per l'approvazione dell'autore.

1. **Precondizioni.** Leggi `poc/docs/PROGRESS.md`. Verifica che tutte le milestone precedenti siano ✅ e che
   nessun'altra sia 🟦. Verifica `git status` pulito. Se una condizione non vale, fermati e spiegalo.
2. **Lettura della specifica.** Leggi in `poc/docs/PRD.md`: §1–§3 (principi e scope), la sezione dei requisiti
   della milestone (§5.x), i contratti dati coinvolti (§6), §7, la voce della milestone in §9 e §10.1. Leggi
   `poc/docs/DECISIONS.md` e i report delle milestone precedenti in `poc/docs/milestones/` (soprattutto "Dubbi" e
   "Esito revisione autore").
3. **Domande bloccanti.** Controlla in DECISIONS.md le domande aperte che bloccano $ARGUMENTS. Elenca
   ogni nuova ambiguità trovata (requisito interpretabile in più modi, libreria che forse non fa ciò che
   serve, conflitto con lo scope). Aggiungile come `Q-n` in DECISIONS.md con le alternative possibili.
4. **Branch.** `git switch -c milestone/<mx>-<slug>` (es. `milestone/m1-analysis`).
5. **Piano.** Crea `poc/docs/milestones/$ARGUMENTS.md` dal modello `poc/docs/milestones/_TEMPLATE.md` e compila solo
   la sezione "Piano":
   - tabella requisiti → test previsti, con tolleranze esplicite e motivate;
   - tabella criteri di accettazione → test `acceptance`;
   - passi piccoli e ordinati, ciascuno chiudibile con `make -C poc check` verde e un commit;
   - eventuali nuove dipendenze (solo stack PRD §7) e documenti da aggiornare (`poc/docs/analysis.md`, `poc/schemas/`).
6. Aggiorna `poc/docs/PROGRESS.md`: stato 🟦, link al report, milestone attiva.
7. Commit: `$ARGUMENTS: plan`.
8. **Checkpoint.** Presenta all'autore, in italiano e in modo sintetico: passi, test chiave, domande aperte
   con le alternative e la tua raccomandazione. **Fermati e attendi** l'ok o le risposte. Non iniziare il
   primo passo nello stesso turno.

Dopo l'ok: esegui i passi uno alla volta (test prima, implementazione, `make -C poc check` verde, commit
`$ARGUMENTS: <cosa> (<ID requisiti>)`). Se durante lo sviluppo emerge un'ambiguità nuova, registrala e chiedi
invece di aggirarla. Quando tutti i passi sono chiusi, usa `/milestone-close`.
