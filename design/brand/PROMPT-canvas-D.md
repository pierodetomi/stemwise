# Prompt della sessione canvas — logo "D · Due stanze"

Da incollare in una nuova sessione di Claude Desktop (canvas). Il prompt è autonomo: la sessione non ha accesso al
repository. Si può allegare in più [directions/logo-directions.html](directions/logo-directions.html) perché veda le
quattro proposte di partenza, ma non è necessario: il codice della D è già nel prompt.

Quando la sessione consegna, i file vanno in `design/brand/final/` e il passo successivo è l'adozione nell'app
(A-075).

````text
Sei il designer del marchio di Stemwise. In questa sessione rifinisci il logo scelto dall'autore, la direzione
"D · Due stanze", fino a un marchio pronto per l'adozione: marchio, icona dell'app per macOS, marchio piccolo per
la barra dell'app, favicon e lockup col nome. Tutto quello che ti serve è in questo messaggio. Comunica con me in
italiano; i testi dentro le tavole che imitano l'app restano in inglese, come l'interfaccia.

Il lavoro ha due tempi e una sola pausa.

TEMPO 1 — Tre varianti dentro la D
Proponi tre varianti della direzione D, nettamente diverse tra loro (non tre ritocchi della stessa forma): una che
evolve la proposta di partenza correggendone i difetti (sezione 5) e due che esprimono le "due stanze" in un altro
modo (la sezione 6 ha alcuni spunti, non obbligatori). Per ciascuna mostra sul canvas:
- un nome e due righe sull'idea;
- il marchio grande;
- l'icona macOS a 1024, 128, 32 e 16 px alle dimensioni reali, e in un Dock accanto a icone neutre;
- il marchio a 20 px nella barra in alto dell'app, una volta in Mix e una in Master (sezione 4);
- la versione monocroma;
- in tre righe: cosa racconta, il rischio, il marchio esistente più vicino.
Chiudi con un confronto delle tre a 128, 32 e 16 px. Poi fermati e chiedimi quale variante scelgo. È l'unica
domanda che puoi farmi in tutta la sessione.

TEMPO 2 — Rifinitura completa, senza interruzioni
Dopo la mia scelta rifinisci la variante fino alla consegna della sezione 8, senza fermarti. Se un punto è
ambiguo, decidi tu nel modo più coerente con questo messaggio e annota la decisione nel README finale. Prima di
consegnare verifica la variante contro i criteri della sezione 9 e correggi quello che non regge.

────────────────────────────────────────

1. IL PRODOTTO

Stemwise è un'app desktop per macOS per mixare e masterizzare brani già registrati. Non registra e non compone: si
parte dagli stem, le tracce separate di un brano (voce, batteria, basso, chitarre…), e si arriva al master pronto
da pubblicare.
- Un assistente AI propone, non guida: ogni sua scelta è numerata, spiegata con una frase e annullabile con un solo
  passo. L'utente ascolta, corregge, decide. Il modello non riceve mai l'audio, solo misure.
- Due fasi, due stanze. Prima il Mix (molte tracce che diventano un brano), poi il Master (il brano intero
  rifinito per la pubblicazione). Sono due spazi di lavoro distinti, con cronologie separate, e si riconoscono a
  colpo d'occhio dal colore: lime su melanzana per il Mix, lilla su indaco per il Master. Il passaggio dal Mix al
  Master è un momento esplicito, un dialogo che produce il mixdown; dal Master si può sempre tornare al Mix.
- Utente: chi scrive e registra bene ma non è un tecnico del suono, e vuole un risultato credibile senza imparare
  un mestiere. Tono della direzione visiva, chiamata "Atelier": competente, calmo, artigianale. Mai tecnico per
  esibizione, mai "magico", mai aggressivo.
- Il nome: stem (le tracce separate, ma anche lo stelo di una pianta) + wise (saggio, "che sa come").
- Slogan della schermata iniziale: "Mix and master the songs you have already recorded. An assistant proposes,
  you listen and decide."

2. PRINCIPI VISIVI DELL'INTERFACCIA

1. Due fasi, due stanze: lime su melanzana per il Mix, lilla su indaco per il Master.
2. L'assistente ha un solo colore, l'azzurro: tutto ciò che ha scritto l'AI è azzurro. Per questo l'azzurro non è
   il colore del prodotto e non entra nel logo.
3. Il colore indica suono o stato, mai decorazione.
4. Leggibile da chi non è fonico.
5. Riposante al buio: nessun bianco pieno, nessun nero pieno.
6. Ogni stato ha anche un segnale non cromatico (forma, lettera, contorno, posizione), non solo un colore.

3. PALETTE, TIPOGRAFIA, FORME

Palette (OKLCH è il riferimento, l'hex la conversione sRGB):
- Fondo finestra Mix, melanzana: oklch(0.195 0.02 320) · #19121B
- Pannelli Mix: oklch(0.24 0.022 320) · #241C26
- Pozzetti, più scuro: oklch(0.16 0.018 320) · #110B12
- Linee: oklch(0.36 0.026 320) · #433946
- Fondo finestra Master, indaco: oklch(0.19 0.03 290) · #141120
- Pannelli Master: oklch(0.235 0.032 290) · #1E1B2C
- Testo principale: oklch(0.96 0.01 320) · #F5F0F6
- Testo attenuato: oklch(0.75 0.018 320) · #B3AAB5
- Accento Mix, lime: oklch(0.88 0.17 118) · #CEE654
- Accento Master, lilla: oklch(0.81 0.12 300) · #CDAFFF
- Azzurro dell'AI (da non usare nel logo): oklch(0.85 0.09 215) · #85DDF3
Nel logo: fondo melanzana (anche con un passaggio leggero verso l'indaco), lime e lilla come soli due colori del
segno, testo #F5F0F6. Mai nero o bianco puro. Sfumature solo leggere e solo per la profondità dell'icona macOS; il
marchio piatto deve funzionare senza.

Tipografia: titoli e nome in Bricolage Grotesque (pesi 500 e 600), il resto dell'interfaccia in Figtree; entrambe
SIL Open Font License 1.1, su Google Fonts. Il wordmark "Stemwise" è Bricolage Grotesque 600, tracking circa
-0.01em; puoi regolare spaziatura e singoli glifi, ma la famiglia deve restare riconoscibile.

Forme: le icone dell'app sono su griglia 16 × 16, tratto 1,5 px, estremità e giunzioni arrotondate, senza
riempimento. Due esempi reali (viewBox 0 0 16 16):
  loop:   <path d="M11 2.5l2 2-2 2M13 4.5H5.5a3 3 0 0 0-3 3M5 13.5l-2-2 2-2M3 11.5h7.5a3 3 0 0 0 3-3"/>
  group:  <rect x="2" y="3" width="12" height="4" rx="1"/><path d="M4.5 7v5.5h9M4.5 9.8h9"/>
Il resto dell'interfaccia: raggi morbidi (schede 14 px, pulsanti 10 px, clip 6 px), cap dei fader larghi e
arrotondati, nessun angolo vivo aggressivo, nessuna ombra drammatica. Il marchio deve sembrare della stessa
famiglia.

4. DOVE VIVE IL LOGO

- Icona dell'app macOS: 1024 × 1024 con la griglia delle icone Apple (corpo circa 824 px centrato, squircle ad
  angoli continui, margine per l'ombra). Si legge nel Dock a 128 px e nel Finder a 32 e 16 px.
- Barra in alto dell'app: alta 52 px, sullo sfondo della finestra. All'estrema sinistra il marchio a 20 × 20 px,
  poi il nome del progetto (Bricolage Grotesque, per esempio "Paper Lanterns") e il selettore "Mix | Master".
  Oggi il marchio in barra è monocromo nel colore d'accento della fase: lime nel Mix (su #19121B), lilla nel Master
  (su #141120).
- Schermata iniziale: marchio a 48 px, sotto "Stemwise" in Bricolage Grotesque 600 a 40 px e lo slogan in Figtree
  14 px #B3AAB5; a destra l'elenco dei progetti recenti.
- Pagina nel browser per il collegamento all'AI: marchio piccolo + "Stemwise" nell'intestazione di un riquadro.
- In futuro: sito di prodotto, README, immagine del disco d'installazione.

5. LA PROPOSTA DI PARTENZA E I SUOI DIFETTI

La direzione D consegnata da un primo giro di esplorazione (viewBox 0 0 100 100):

<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <path d="M16 25H43Q50 25 55 32L64 43Q68 48 76 48H84" fill="none" stroke="#CEE654" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M16 75H43Q50 75 55 68L64 57Q68 52 76 52H84" fill="none" stroke="#CDAFFF" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>
  <path d="M75 39L86 50L75 61" fill="none" stroke="#CDAFFF" stroke-width="8" stroke-linecap="round" stroke-linejoin="round"/>
</svg>

Nell'icona sta su un rettangolo #19121B di 824 px con rx 218, scalato di 6 volte e centrato.

Cosa funziona: è l'unica delle quattro direzioni che racconta qualcosa di proprio del prodotto (le due fasi coi
loro due colori); il tratto arrotondato è della stessa famiglia delle icone; lime e lilla sul melanzana stanno
bene insieme.

Cosa non funziona, e va risolto:
1. Si legge come l'icona generica "merge" (due strade che si uniscono, la fusione di due rami): è un simbolo
   d'interfaccia già visto ovunque, non un marchio.
2. Racconta la cosa sbagliata: le due linee arrivano insieme, in parallelo, come due ingressi alla pari. Le fasi
   invece vengono una dopo l'altra: prima si mixa, poi si masterizza; e il Mix è "molte parti → un brano", il
   Master è "un brano → rifinito".
3. La punta di freccia è un cliché (export, avanti, invio) e nel punto d'incontro si sovrappone alla fine della
   linea lime: a 32 px diventa una macchia.
4. Il segno occupa metà dell'altezza del suo quadrato: a 16–32 px resta una freccia minuscola.
5. In monocromia, cioè proprio nella barra in alto, le due stanze spariscono e resta solo il "merge".
6. Il lettering del lockup non è Bricolage Grotesque ma un disegno geometrico a tratto.
7. Il fondo dell'icona è un rettangolo con raggio semplice, non lo squircle ad angoli continui di macOS.

6. SPUNTI PER LE VARIANTI (facoltativi)

- La soglia: molte linee sottili (gli stem) entrano nella prima stanza e diventano una; quella sola linea passa
  una soglia ed entra nella seconda stanza, dove si posa o si chiude. Prima lime, poi lilla: la sequenza si legge
  da sinistra a destra.
- La pianta: due stanze viste dall'alto, come nella pianta di un architetto (l'Atelier), contigue e con un varco
  tra loro. La prima è divisa in strisce (le tracce), la seconda è piena o ha una sola forma (il brano). Anche in
  monocromia restano due stanze, perché lo dice la geometria.
- Il passaggio continuo: un solo tratto che cambia colore a metà, lime poi lilla, e nel suo percorso disegna una
  S (Stemwise) o una forma che va da "aperta" a "chiusa".
Qualunque cosa tu scelga, le due stanze devono riconoscersi anche senza colore (principio 6).

Un'occasione da valutare per la barra in alto: se il marchio è fatto di due parti separabili, l'app potrebbe
accendere la stanza della fase corrente nel suo accento e attenuare l'altra (per esempio al 40 %), invece di
colorare tutto il marchio. Il marchio direbbe così anche in quale fase sei, con la posizione oltre che col colore.
Mostralo nelle tavole della barra; se non regge a 20 px, resta il marchio monocromo.

7. COSA EVITARE

- Frecce, simboli "merge", "play", "fast forward", segnali stradali.
- Cliché audio e AI: note, cuffie, microfoni, vinili, onde sinusoidali, barre da equalizzatore, cervelli,
  scintille, stelline, gufi.
- Yin-yang, bandiere, simboli astratti di "dualità" senza legame col prodotto.
- Somiglianze con marchi noti: Logic Pro, GarageBand, Ableton, FL Studio, Reaper, Audacity, iZotope, Spotify,
  SoundCloud, Splice, LANDR; e con le icone "merge" di Git e GitHub.
- Dettagli sotto 1,5 px a 20 px, testo nell'icona, effetti 3D pesanti, fotorealismo, immagini raster generate:
  il logo è vettoriale, disegnato a tracciati.
- Font o risorse con licenze diverse da SIL OFL, MIT, Apache-2.0, BSD o ISC.

8. CONSEGNA (TEMPO 2)

Una cartella "final" scaricabile con questi file. SVG puliti: solo tracciati, niente testo vivo, niente immagini
incorporate, colori della sezione 3.
- mark.svg — il marchio a colori, viewBox quadrato.
- mark-mono.svg — monocromo in currentColor.
- mark-20.svg — disegnato per 20 px (spessori e posizioni allineati alla griglia di 20, varianti ottiche se
  servono), in currentColor. Se adotti l'idea della stanza accesa (sezione 6), le due stanze sono due gruppi
  <g id="room-mix"> e <g id="room-master"> in currentColor, così che l'app possa colorarli e attenuarli da CSS.
- favicon-32.svg e favicon-16.svg — con il fondo, disegnati per quelle dimensioni.
- icon-1024.svg — l'icona piatta completa, con lo squircle ad angoli continui.
- icon-layers/ — i livelli per Icon Composer: background.svg (lo squircle col fondo), e un file per ciascun
  elemento in primo piano (per esempio room-mix.svg e room-master.svg), tutti a 1024 × 1024 e allineati.
- lockup.svg — marchio + "Stemwise" in Bricolage Grotesque 600 convertito in tracciati, sul fondo trasparente,
  pensato per il melanzana; lockup-mono-light.svg (#F5F0F6) e lockup-mono-dark.svg (#19121B).
- mono-light.svg e mono-dark.svg — il marchio in #F5F0F6 e in #19121B.
- presentation.html — una pagina autonoma (un solo file, si apre con un doppio clic, font da Google Fonts) con: il
  marchio e la sua griglia di costruzione; l'icona a 1024, 512, 128, 32 e 16 px alle dimensioni reali e nel Dock;
  la barra in alto in Mix e in Master; la schermata iniziale col lockup; le monocromie su chiaro e su scuro; area
  di rispetto e dimensione minima; usi corretti e scorretti (non ruotare, non ricolorare con altre tinte, non
  allungare, non mettere su fondi chiari a colori).
- README.md, in italiano: l'idea in tre righe, la costruzione (griglia, spessori, raggi), i colori, l'area di
  rispetto, le dimensioni minime, l'elenco dei file, le decisioni prese nel tempo 2.
Se non riesci a convertire Bricolage Grotesque in tracciati nel canvas, consegna il lockup con il testo vivo e il
font dichiarato, e dillo chiaramente nel README: lo convertirò io. Non sostituirlo con un lettering inventato.

9. CRITERI DI VERIFICA

1. Riconoscibilità: si ricorda dopo averlo visto una volta? A 16 px nel Finder è ancora "quello di Stemwise"?
2. Pertinenza: racconta le due fasi, una dopo l'altra, e non l'audio in generale né una fusione qualsiasi?
3. Senza colore: in monocromia restano due stanze?
4. Coerenza: sta accanto alle icone e ai controlli dell'app come se fosse nato con loro?
5. Calma: è adatto a uno strumento in cui si passano ore al buio?
6. Durata: tra due anni sembrerà ancora giusto?
````
