# Stemwise — marchio "D · Due stanze", rifinito

Consegna della rifinitura della direzione D (A-075, A-076). La pagina da aprire per prima è
[presentation.html](presentation.html): un solo file, si apre con un doppio clic, carica i font da Google Fonts.

## L'idea

Le due traiettorie della proposta D sono il **Mix**: molte parti che si avvicinano e si riuniscono, in lime.
Oltre un varco c'è il **Master**: una sola forma piena, il brano finito, in lilla.
Prima le linee, poi la stanza: la sequenza si legge da sinistra a destra, anche senza colore.

## Costruzione

Griglia di 100 × 100 (`mark.svg`).

| Elemento | Misure |
| --- | --- |
| Linee del Mix | tratto 9, estremità e giunzioni arrotondate; partono a y = 20 e y = 80 da x = 9, arrivano a y = 45,5 e y = 54,5 a x = 53, dove si toccano senza sovrapporsi |
| Curva | quella della proposta D: corsa orizzontale fino a x = 23, curva quadratica (controllo a x = 31), diagonale dritta, curva quadratica (controllo a x = 45), corsa orizzontale; tangenti continue, la diagonale occupa il 40 % centrale tra i due controlli |
| Varco | 7,5 tra la fine delle linee (con l'estremità arrotondata) e la stanza |
| Stanza del Master | rettangolo pieno x 65–95, y 37–63 (30 × 26), raggio 10 |
| Ingombro | x 4,5–95, y 15,5–84,5: centrato nel quadrato |

Versioni ottiche, ridisegnate sulla griglia dei pixel:

| File | Griglia | Tratto | Varco | Stanza |
| --- | --- | --- | --- | --- |
| `mark-20.svg` | 20 | 2 px, corse su righe intere (y 4 e 16) | 1,5 px | 6,25 × 6, raggio 2 |
| `favicon-32.svg` | 32 | 2 px | 2 px | 7 × 6, raggio 2,5 |
| `favicon-16.svg` | 16 | 1,5 px | 1,5 px | 3,75 × 4, raggio 1,5 |

Icona: squircle ad angoli continui (l'approssimazione a curve di Bézier degli angoli continui di Apple), corpo di
824 × 824 centrato sul quadrato di 1024, raggio nominale 185,4; marchio in scala 5,8 (tratto 52 px) centrato sul
corpo; ombra morbida nel margine (sfocatura 14, spostamento 12, opacità 45 %); filo interno a 2 px al 7 % del testo
principale, per staccare il bordo dal Dock scuro.

## Colori

| Uso | OKLCH | Hex |
| --- | --- | --- |
| Mix, lime (linee) | oklch(0.88 0.17 118) | `#CEE654` |
| Master, lilla (stanza) | oklch(0.81 0.12 300) | `#CDAFFF` |
| Fondo, melanzana | oklch(0.195 0.02 320) | `#19121B` |
| Testo del lockup, monocromo chiaro | oklch(0.96 0.01 320) | `#F5F0F6` |
| Sfumatura del fondo dell'icona | da `#271E2A` (in alto a sinistra) a `#19121B` a `#171223` (in basso a destra, verso l'indaco) | |

L'azzurro dell'assistente non entra nel marchio. Niente bianco né nero puri.

## Area di rispetto e dimensioni minime

- **Area di rispetto**: x = metà dell'altezza della stanza del Master (13/100 del lato del marchio), misurata dal
  segno visibile. Nel lockup vale lo stesso x, misurato sul marchio.
- **Marchio**: minimo 16 px. Sotto i 24 px si usano i disegni ottici (`mark-20.svg`, `favicon-32.svg`,
  `favicon-16.svg`); da 24 px in su `mark.svg`.
- **Lockup**: minimo 110 px di larghezza. Più piccolo, solo il marchio.
- **Icona**: tutte le dimensioni macOS dallo stesso `icon-1024.svg`.

## File

| File | Contenuto |
| --- | --- |
| `mark.svg` | marchio a colori, viewBox 100 × 100, gruppi `room-mix` e `room-master` |
| `mark-mono.svg` | marchio in `currentColor` |
| `mark-20.svg` | marchio disegnato per 20 px, gruppi `room-mix` e `room-master` in `currentColor` |
| `mono-light.svg`, `mono-dark.svg` | marchio in `#F5F0F6` e in `#19121B` |
| `favicon-32.svg`, `favicon-16.svg` | favicon col fondo, disegnati per quelle dimensioni |
| `icon-1024.svg` | icona piatta completa: squircle, sfumatura, ombra, marchio |
| `icon-layers/background.svg` | lo squircle col fondo, 1024 × 1024 |
| `icon-layers/background-fullbleed.svg` | lo stesso fondo a tutto quadrato, per Icon Composer (vedi sotto) |
| `icon-layers/room-mix.svg`, `icon-layers/room-master.svg` | le due stanze, 1024 × 1024, allineate al fondo |
| `lockup.svg` | marchio + "Stemwise" in tracciati, per il melanzana |
| `lockup-mono-light.svg`, `lockup-mono-dark.svg` | lockup in `#F5F0F6` e in `#19121B` |
| `presentation.html` | la presentazione completa |

Tutti gli SVG contengono solo tracciati: nessun testo vivo, nessuna immagine incorporata. Si rigenerano con
`python3 design/brand/build-final.py` (richiede `fonttools` e `uharfbuzz`, licenze MIT e Apache-2.0, usate solo per
convertire il font); la presentazione è costruita da `design/brand/presentation.py`.

## Decisioni prese nel tempo 2

1. **Rifinitura fedele, non una nuova forma.** Il tempo 1 ha proposto tre varianti (Soglia, Pianta, Esse); l'autore le
   ha scartate tutte e ha chiesto di rifinire la proposta D consegnata. La rifinitura tiene le due traiettorie con la
   loro curva (corsa, curva, diagonale, curva, corsa), il tratto arrotondato, il lime e il lilla sul melanzana, la
   lettura da sinistra a destra. Le varianti scartate non sono state conservate.
2. **Le due linee sono entrambe il Mix.** Nella proposta una linea era il Mix e l'altra il Master, e arrivavano
   insieme (difetto 2). Ora sono due parti dello stesso Mix che si riuniscono; il Master viene dopo, ed è una cosa
   sola.
3. **La freccia diventa la stanza del Master.** La punta di freccia (difetto 3) è sostituita da un rettangolo pieno
   arrotondato, staccato da un varco. È piena perché in monocromia le due stanze restino diverse per forma (linee
   contro superficie, difetto 5), ed è arrotondata con raggio 10/30 per somigliare a una stanza e alle schede
   dell'app più che a una pillola.
4. **Le linee si toccano senza fondersi.** Arrivano a 45,5 e 54,5 con tratto 9: si toccano sul centro ma restano due.
   Non c'è un punto d'incontro a Y, che era la ragione della lettura "merge" (difetto 1).
5. **Proporzioni.** La stanza è alta 26: più delle due linee affiancate (18), perché il Master pesi a colpo d'occhio
   quanto il Mix; con 18 si riduceva a un puntino a 16–32 px. Il segno occupa due terzi del quadrato (difetto 4).
6. **Barra in alto: stanza accesa.** Regge a 20 px, quindi è adottata: l'app colora il gruppo della fase corrente
   col suo accento e porta l'altro al 40 % (nel Mix le linee lime e la stanza al 40 %; nel Master la stanza lilla e le
   linee al 40 %). Per evitare che le parti sovrapposte si vedano attraverso, il 40 % va reso come colore mescolato col
   fondo (`color-mix(in srgb, <accento> 40%, <fondo>)`) e non come opacità; nel disegno a 20 px le due stanze comunque
   non si sovrappongono. Il marchio tutto nell'accento della fase resta il ripiego.
7. **Font.** Il nome è convertito in tracciati dallo stesso file che l'app include
   (`@fontsource/bricolage-grotesque`, SemiBold 600, OFL-1.1), con crenatura HarfBuzz e tracking −0,01 em; nessun
   glifo ritoccato.
8. **Icon Composer.** Icon Composer applica la sua maschera: per lui il fondo giusto è
   `background-fullbleed.svg`, mentre `background.svg` (lo squircle) serve per un `.icns` classico e come riferimento
   di allineamento. Le due stanze sono livelli separati, così Icon Composer può dare a ciascuna il suo vetro e la sua
   profondità.
9. **Ombra nell'icona piatta.** `icon-1024.svg` porta un'ombra morbida nel margine, come le icone macOS fino a Sequoia;
   i livelli per Icon Composer non ne hanno, perché la aggiunge il sistema.
10. **Icona piccola.** Non ci sono versioni ottiche dell'icona dell'app a 16 e 32 px: sui Mac Retina il Finder usa la
    rappresentazione doppia, e quella ridotta dall'icona da 1024 resta leggibile. I disegni ottici esistono per il
    marchio senza fondo (barra) e per i favicon.
11. **Area di rispetto e minimi** come sopra: x = mezza stanza, marchio da 16 px, lockup da 110 px.
