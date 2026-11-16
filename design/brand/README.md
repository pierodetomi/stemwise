# Stemwise — marchio

Il lavoro sul logo dell'app: il marchio, l'icona macOS, il marchio nella barra in alto e il lockup col nome. Le
decisioni sono in [A-075 e A-076](../../docs/app/DECISIONS.md).

> **Diritti riservati.** Nome e logo di Stemwise, e tutto il contenuto di questa cartella, non rientrano nella licenza
> MIT del repository. Cosa si può fare e cosa no: [TRADEMARKS.md](../../TRADEMARKS.md).

## Stato

| Passo | Stato | Dove |
| --- | --- | --- |
| Brief per quattro direzioni | ✅ 2026-11-13 | [BRIEF-logo.md](BRIEF-logo.md) |
| Quattro direzioni (agente esterno) | ✅ 2026-11-13 | [directions/](directions/) |
| Scelta dell'autore: **D · Due stanze** | ✅ 2026-11-13 | A-075 |
| Rifinitura della D | ✅ 2026-11-14 | [final/](final/) (apri `final/presentation.html`), A-076; si rigenera con `build-final.py` |
| Adozione nell'app (icone Tauri, marchio in barra, tavole del design) | ✅ 2026-11-14 | A-078 |

Nell'app il marchio vive in `app/ui/src/shared/Icon.tsx` (`Mark`), nelle tavole in `mark()` di
`design/tools/lib.mjs`, con i colori in `design/components/components.css`; le icone in `app/src-tauri/icons/` sono
generate da `final/icon-1024.svg`.

## Le quattro direzioni

Disegnate da un agente esterno a partire da [BRIEF-logo.md](BRIEF-logo.md). Sono un archivio e un riferimento: non
fanno parte del prodotto.

| Direzione | Idea | Cartella |
| --- | --- | --- |
| A · Convergenza | quattro stem entrano distinti e si raccolgono in un'unica uscita | [directions/A-convergenza](directions/A-convergenza) |
| B · Monogramma | una S costruita come curva di automazione, con tre punti | [directions/B-monogramma](directions/B-monogramma) |
| C · Il fader | un cap di fader fermo nel punto giusto, non a fondo corsa | [directions/C-fader](directions/C-fader) |
| **D · Due stanze** (scelta) | due traiettorie, lime (Mix) e lilla (Master), che si incontrano in un'uscita | [directions/D-due-stanze](directions/D-due-stanze) |

[directions/logo-directions.html](directions/logo-directions.html) mostra le quattro proposte: icona da 1024 a 16 px,
Dock, barra in alto in Mix e Master, lockup nella welcome, monocromie e confronto finale. Si apre con un doppio clic e
carica i font da Google Fonts.

Ogni cartella contiene `mark.svg`, `mark-20.svg` (in `currentColor`), `icon-1024.svg`, `icon-layer-bg.svg`,
`icon-layer-fg.svg`, `lockup.svg`, `mono-light.svg` e `mono-dark.svg`.

Limiti dichiarati dall'agente e osservati qui:

- il lettering dei lockup è un disegno geometrico a tratto, **non** Bricolage Grotesque 600: va rifatto;
- il fondo dell'icona è un rettangolo con raggio semplice (`rx="218"`), non lo squircle ad angoli continui di macOS;
- i livelli per Icon Composer sono solo separati in due file, non assemblati;
- in `C-fader/mono-dark.svg` i dettagli interni hanno lo stesso colore del cap e spariscono (non riguarda la D).
