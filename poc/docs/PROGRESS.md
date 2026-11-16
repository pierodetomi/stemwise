# Avanzamento PoC

Legenda stato: ⬜ da iniziare · 🟦 in corso · 🟨 in revisione autore · ✅ approvata

| Milestone | Requisiti | Stato | Report | Blocchi |
| --- | --- | --- | --- | --- |
| Setup repository e flusso agentico | — | ✅ | — | — |
| M0 Fondamenta | ING-1→6, REN-5 (trim), REN-7 | ✅ | [M0](milestones/M0.md) | — |
| M1 Analisi | ANA-1→13 | ✅ | [M1](milestones/M1.md) | — |
| M2 Render | REN-1→7 | ✅ | [M2](milestones/M2.md) | — |
| M3 Decisione | DEC-1→8 | ✅ | [M3](milestones/M3.md) | — |
| M4 Ciclo di verifica | EVA-1→6, REP-1 | ✅ | [M4](milestones/M4.md) | H5: `mix` 0,11 USD misurati, decisione con i costi di `revise` (D-047) |
| M5 Revisione a parole | REV-1→3 | ✅ | [M5](milestones/M5.md) | H5: ~0,22 USD per brano, accettata (D-068) |
| M6 Master | MAS-1→3 | ✅ | [M6](milestones/M6.md) | — |
| M7 Ascolto (ridotta: esito della PoC) | ~~REP-2 + protocollo §10~~ (D-079) | ✅ | [M7](milestones/M7.md) | — |

Milestone attiva: **nessuna**. PoC chiusa con esito **go** (D-080, [README](../README.md#esito-della-poc)).

Domande aperte e decisioni: [DECISIONS.md](DECISIONS.md). Report per milestone: `docs/milestones/Mx.md`
(modello in [milestones/_TEMPLATE.md](milestones/_TEMPLATE.md)).

## Criteri di accettazione e validazione manuale prevista

Ogni criterio automatico diventa un test `@pytest.mark.acceptance("Mx")`. La validazione manuale è ciò che
l'autore verifica a orecchio o a occhio prima di approvare; le istruzioni esatte vanno nel report.

- **M0** — Auto: `stemwise mix <prog> --baseline-only` su stem sintetici scrive `baseline.wav` entro −3 dBTP;
  durate diverse ⇒ padding + warning; sample rate diversi ⇒ errore; run non sovrascrive run precedenti.
  Manuale: `--baseline-only` su un brano reale in `local/`, ascolto di `baseline.wav` (nessun clip, durata ok).
- **M1** — Auto: seno 1 kHz nella banda 500–2k; rumore rosa con energia simile nelle bande di pari ampiezza in
  ottave; due stem identici ⇒ mascheramento massimo; `analysis.json` di 12 stem < 8.000 token stimati.
  Manuale: `stemwise analyze` su un brano reale, lettura di `analysis.json` per plausibilità (voce vs basso, ecc.).
- **M2** — Auto: +6 dB di gain ⇒ +6 LU ±0,1; pan −1 azzera il canale destro; il passa-alto attenua sotto il
  taglio; due render dello stesso piano ⇒ stesso hash. Manuale: render di un piano scritto a mano su un brano
  reale, ascolto A/B con `baseline.wav`.
- **M3** — Auto: flusso completo offline col decisore finto; piani invalidi ⇒ retry fino a 2 poi errore chiaro;
  risposte non JSON gestite. Manuale (rete, costa): una chiamata reale, piano valido, log completi, lettura delle
  motivazioni.
- **M4** — Auto: il report confronta la distanza della somma grezza con ogni iterazione; il piano migliore ha la
  distanza minima tra i validi; `report.md` generato. Manuale: lettura del report di un run reale.
- **M5** — Auto: "voce più avanti" (col decisore finto) ⇒ diff sui parametri della voce o degli stem che la
  coprono. Manuale: due richieste reali, ascolto, la direzione è quella chiesta?
- **M6** — Auto: `master.wav` prodotto; `master_report.json` con LUFS, true peak, distanza dal riferimento.
  Manuale: ascolto del master contro il riferimento.
- **M7** — Ridotta (D-079): niente `ab` né protocollo §10.2. Previsto in origine: Auto, `ab` produce file di pari
  durata e loudness con nomi casuali e chiave separata; Manuale, protocollo §10.2 su 3 brani e scheda compilata.
  Resta: go/no-go annotato nel README, con i dati di M3–M6.
