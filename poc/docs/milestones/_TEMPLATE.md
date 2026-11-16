# Mx — <nome della milestone>

- Stato: 🟦 in corso | 🟨 in revisione autore | ✅ approvata
- Branch: `milestone/mx-<slug>`
- Requisiti PRD: <es. ANA-1 → ANA-13>
- Sezioni PRD lette: <es. §5.2, §6.2, §9, §10.1>

## Piano (scritto all'avvio, approvato dall'autore prima di scrivere codice)

### Requisiti → test previsti

| Id | Cosa verificare | Test previsto | Tolleranza |
| --- | --- | --- | --- |
| ANA-1 | … | `tests/analysis/test_stem.py::test_…` | ±0,1 LU |

### Criteri di accettazione → test

| Criterio (PRD §9) | Test |
| --- | --- |
| … | `…` (`@pytest.mark.acceptance("Mx")`) |

### Passi

1. …

### Ambiguità e domande

- Nessuna / Q-x in [DECISIONS.md](../DECISIONS.md)

## Chiusura

### Fatto

- …

### Deciso

- D-xxx … (dettagli in DECISIONS.md)

### Dubbi e rischi

- …

### Fuori scope segnalato

- Nessuno / …

### Verifica automatica

```text
<output sintetico di `make check` e `make reqs M=Mx`>
```

Requisiti senza test automatico e motivo: …

Esito `milestone-reviewer`: <sintesi dei rilievi e come sono stati gestiti>

### Validazione manuale (per l'autore)

Prerequisiti: …

1. Comando:
   ```bash
   uv run stemwise …
   ```
   Cosa controllare / ascoltare: …
   Esito atteso: …

### Esito revisione autore

_Compilato dall'autore._

- Data:
- Validazione manuale: ok / problemi: …
- Decisione: approvata / da rivedere: …
