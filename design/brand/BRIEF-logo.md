# Brief — quattro proposte di logo per Stemwise

Prompt autonomo per un agente AI di design: contiene tutto il necessario e non richiede l'accesso al codice
dell'app. Si incolla così com'è.

---

## Il compito

Disegna **quattro proposte di logo** per Stemwise, ognuna su una **direzione visiva diversa** (sezione 5). Ogni
proposta comprende il marchio, l'icona dell'app per macOS, la versione piccola per la barra dell'app e un lockup col
nome. Tutte e quattro devono restare dentro il linguaggio visivo dell'app, descritto nelle sezioni 2–4: palette,
tipografia e forme non si reinventano.

Oggi l'icona è fatta di **cinque barre verticali lime** di altezze diverse (più alta al centro) su un quadrato
arrotondato color melanzana. Funziona ma è anonima: sono le barre di un equalizzatore qualunque, potrebbero essere il
logo di qualsiasi app audio o di podcast. Il problema da risolvere è questo: **un segno che sia riconoscibilmente
Stemwise**, non solo "audio".

## 1. Il prodotto

Stemwise è un'app desktop per macOS per **mixare e masterizzare brani già registrati**. Non registra e non
compone: si parte dagli **stem**, le tracce separate di un brano (voce, batteria, basso, chitarre…), e si arriva al
master pronto da pubblicare.

- **Un assistente AI propone, non guida.** Su richiesta propone i parametri del mix (volumi, equalizzazione,
  compressione, riverberi…). Ogni sua scelta è numerata, spiegata con una frase e annullabile con un solo passo.
  L'utente ascolta, corregge a mano o a parole, e decide. Il modello non riceve mai l'audio, solo misure numeriche:
  l'audio resta sul computer.
- **Due fasi, due stanze.** Prima il **Mix** (molte tracce che diventano un brano), poi il **Master** (il brano
  intero rifinito per la pubblicazione). Le due fasi hanno spazi di lavoro distinti e si riconoscono a colpo
  d'occhio dal colore: **lime su melanzana** per il Mix, **lilla su indaco** per il Master.
- **Utente.** Chi scrive e registra bene ma non è un tecnico del suono, e vuole un risultato credibile senza
  imparare un mestiere né pagare abbonamenti. In secondo luogo, chi sa mixare e vuole un punto di partenza rapido e
  spiegato.
- **Tono.** Competente, calmo, artigianale: la direzione visiva dell'app si chiama **"Atelier"**. Mai tecnico per
  esibizione, mai "magico", mai aggressivo.
- **Il nome.** *Stem* (le tracce separate, ma anche lo stelo di una pianta) + *wise* (saggio, consapevole, "che sa
  come"). Entrambi i significati sono materiale legittimo; il gioco di parole botanico è possibile, non obbligatorio.
- **Slogan nella schermata iniziale:** "Mix and master the songs you have already recorded. An assistant proposes,
  you listen and decide." (l'interfaccia è in inglese).

## 2. Principi visivi dell'interfaccia

1. **Due fasi, due stanze**: lime su melanzana per il Mix, lilla su indaco per il Master.
2. **L'assistente ha un solo colore**, l'azzurro: tutto ciò che ha scritto o proposto l'AI è azzurro. Per questo
   l'azzurro **non** è il colore del prodotto.
3. **Il colore indica suono o stato, mai decorazione.** Ogni traccia ha la sua tinta; i meter sono verde menta;
   l'automazione è rosa; il resto è neutro.
4. **Leggibile da chi non è fonico**: etichette in parole semplici, niente gergo esibito.
5. **Riposante al buio**: nessun bianco pieno, nessun nero pieno, contrasti comodi per ore di lavoro in una stanza
   in penombra.
6. **Ogni stato ha anche un segnale non cromatico** (forma, lettera, contorno, posizione), non solo un colore.

## 3. Palette

Usa questi valori esatti (OKLCH è il riferimento; l'hex è la conversione sRGB).

| Ruolo | OKLCH | Hex |
| --- | --- | --- |
| Fondo finestra Mix (melanzana) | `oklch(0.195 0.02 320)` | `#19121B` |
| Pannelli Mix | `oklch(0.24 0.022 320)` | `#241C26` |
| Pozzetti e grafici (più scuro) | `oklch(0.16 0.018 320)` | `#110B12` |
| Linee e bordi | `oklch(0.36 0.026 320)` | `#433946` |
| Fondo finestra Master (indaco) | `oklch(0.19 0.03 290)` | `#141120` |
| Pannelli Master | `oklch(0.235 0.032 290)` | `#1E1B2C` |
| Testo principale (mai bianco puro) | `oklch(0.96 0.01 320)` | `#F5F0F6` |
| Testo attenuato | `oklch(0.75 0.018 320)` | `#B3AAB5` |
| **Accento Mix (lime)** | `oklch(0.88 0.17 118)` | `#CEE654` |
| Testo sul lime | `oklch(0.2 0.04 118)` | — (verde quasi nero) |
| **Accento Master (lilla)** | `oklch(0.81 0.12 300)` | `#CDAFFF` |
| Assistente AI (azzurro) | `oklch(0.85 0.09 215)` | `#85DDF3` |
| Meter (menta) | `oklch(0.82 0.13 160)` | `#6FDEA7` |
| Automazione (rosa) | `oklch(0.8 0.12 345)` | `#F59ECF` |

Le **12 tinte delle tracce** (luminosità alternata, così due tracce vicine differiscono anche per chiarezza):
`#F8A4A3` `#DE9871` `#ECC980` `#A2BC75` `#99E0BA` `#6BBBB6` `#98DAEA` `#79B1E0` `#BAC4F6` `#B99BDE` `#EFBBE3`
`#E48DA8`.

Regole sul colore per il logo:

- **Fondo**: melanzana `#19121B` (o un tono molto vicino della stessa famiglia). Mai nero puro, mai bianco puro.
- **Colore principale del segno**: lime `#CEE654`. Il lilla `#CDAFFF` è ammesso come secondo colore, soprattutto
  nella direzione D.
- Le altre tinte (azzurro, menta, rosa, tinte delle tracce) hanno un significato preciso nell'app: usale solo se
  una direzione ne fa un argomento esplicito, mai come decorazione, e mai l'azzurro come colore dominante.
- Al massimo **tre colori** nell'icona, fondo compreso. Sfumature solo leggere e solo per la profondità dell'icona
  macOS; il marchio piatto deve funzionare senza.

## 4. Tipografia e forme

### Tipografia

- Titoli e nome: **Bricolage Grotesque** (pesi 500 e 600). Tutto il resto dell'interfaccia: **Figtree**. Entrambe
  con licenza SIL Open Font License 1.1, su Google Fonts.
- Il **wordmark** "Stemwise" usa Bricolage Grotesque 600 con tracking vicino a `-0.01em`. Puoi regolare spaziatura
  e singoli glifi, ma la famiglia deve restare riconoscibile. Nei file finali il testo va convertito in tracciati.
- Niente testo dentro l'icona dell'app (al più una singola lettera, nella direzione B).

### Forme

L'app ha un set di icone proprio: **griglia 16 × 16, tratto 1,5 px, estremità e giunzioni arrotondate**
(`stroke-linecap: round; stroke-linejoin: round; fill: none`), forme piene solo per i dettagli piccoli. Alcuni
esempi reali, `viewBox="0 0 16 16"`:

```svg
<!-- loop -->
<path d="M11 2.5l2 2-2 2M13 4.5H5.5a3 3 0 0 0-3 3M5 13.5l-2-2 2-2M3 11.5h7.5a3 3 0 0 0 3-3"/>
<!-- faders: tre corsie con i cap pieni -->
<path d="M4 2.5v11M8 2.5v11M12 2.5v11"/>
<rect x="2.5" y="8.5" width="3" height="2.5" rx=".8"/><rect x="6.5" y="4" width="3" height="2.5" rx=".8"/><rect x="10.5" y="7" width="3" height="2.5" rx=".8"/>
<!-- group: una traccia che raccoglie le altre -->
<rect x="2" y="3" width="12" height="4" rx="1"/><path d="M4.5 7v5.5h9M4.5 9.8h9"/>
<!-- ear: ascolto -->
<path d="M2.5 9V8a5.5 5.5 0 0 1 11 0v1"/><rect x="2" y="9" width="3" height="4.5" rx="1"/><rect x="11" y="9" width="3" height="4.5" rx="1"/>
```

Il marchio attuale nella barra dell'app, `viewBox="0 0 18 18"`, riempito col colore d'accento della fase:

```svg
<rect x="1" y="7" width="3" height="4" rx="1.5"/><rect x="5.5" y="3" width="3" height="12" rx="1.5"/>
<rect x="10" y="5" width="3" height="8" rx="1.5"/><rect x="14.5" y="8" width="3" height="2" rx="1"/>
```

Il resto dell'interfaccia: raggi morbidi ovunque (schede e dialoghi 14 px, pulsanti 10 px, campi 8 px, clip audio
6 px), cap dei fader larghi e arrotondati, manopole con un arco che mostra il valore, clip audio con una fascia del
nome nel colore pieno della traccia, corpo tinto e forma d'onda chiara, curve di automazione fatte di punti uniti da
segmenti. Nessun angolo vivo aggressivo, nessuna ombra drammatica. Il marchio deve sembrare della stessa famiglia
di queste forme, non un oggetto estraneo.

## 4bis. Dove compare il logo nell'app

- **Barra in alto** (alta 52 px, sullo sfondo della finestra): all'estrema sinistra il marchio a **20 × 20 px**,
  monocromo nel colore d'accento della fase (lime nel Mix, lilla nel Master), seguito dal nome del progetto in
  Bricolage Grotesque e dal selettore delle fasi "Mix | Master".
- **Schermata iniziale**: a sinistra, centrati verticalmente, il marchio a **48 px**, sotto il nome "Stemwise" in
  Bricolage Grotesque a 40 px e lo slogan in Figtree 14 px testo attenuato; a destra l'elenco dei progetti recenti.
- **Pagina di collegamento all'AI** mostrata nel browser: marchio piccolo + "Stemwise" nell'intestazione di un
  riquadro.
- In futuro: un sito di prodotto, il README, l'immagine del disco d'installazione.

## 5. Le quattro direzioni

Ognuna parte da un'idea diversa; tutte restano nella palette e nelle forme delle sezioni 3–4. Le indicazioni sono un
punto di partenza: dentro ogni direzione hai libertà, purché il risultato sia chiaramente diverso dalle altre tre.

### A · Convergenza — "molti stem, un brano"

L'idea del prodotto in un gesto: più linee parallele (gli stem) che confluiscono in una sola. Linguaggio a
**tratto**, imparentato col set di icone (estremità arrotondate, spessore costante), come un diagramma di routing
ridotto all'essenziale. Le linee possono formare una S, un ventaglio o un nodo di raccolta. Facoltativo: le linee in
partenza prendono alcune tinte delle tracce e quella d'arrivo è lime, *solo* se resta leggibile a 20 px
(altrimenti tutte in un colore).
Da evitare: lo schema di un circuito, un logo di rete o di fintech.

### B · Monogramma e wordmark — "la S di Stemwise"

Direzione tipografica: una **S** costruita a partire da Bricolage Grotesque e trasformata da un elemento del mondo
dell'app: la curva di un'automazione con i suoi punti, il profilo di una clip con le maniglie di dissolvenza, una
forma d'onda continua che disegna la lettera. Il wordmark completo è curato con la stessa attenzione del marchio: è
la direzione in cui il nome conta di più.
Da evitare: una S generica da startup, o una S-onda già vista. Cerca un dettaglio proprio.

### C · L'oggetto dello strumento — "il fader che si tocca"

Un pittogramma tattile preso dai controlli dell'app: il **cap largo del fader** sulla sua corsia, una manopola con
l'arco del valore, o la combinazione dei due. Morbido, quasi un oggetto fisico. Il carattere "wise" può emergere nel
dettaglio: il fader posato in un punto preciso, non a fondo corsa (il mix giusto, non il più forte).
Da evitare: l'icona di un mixer generico o di un'app di impostazioni.

### D · Due stanze — "prima il mix, poi il master"

Il segno racconta le **due fasi**: due forme che si completano, una lime e una lilla, sul fondo melanzana (o con un
passaggio da melanzana a indaco). Per esempio due archi o due semiforme sfalsate, una forma che passa da "molte
parti" a "una superficie", una soglia tra due ambienti. È l'unica direzione in cui due accenti convivono da
protagonisti. La versione monocroma per la barra deve restare sensata anche senza il contrasto di colore (principio
6).
Da evitare: un simbolo yin-yang, una bandiera, un logo astratto di "dualità" senza legame con l'audio.

## 6. Requisiti per ogni proposta

Ogni proposta deve funzionare in **tutti** questi usi:

1. **Icona dell'app macOS**, 1024 × 1024, con la forma a squircle e la griglia delle icone Apple (corpo dell'icona
   circa 824 px centrato, angoli continui, margine per l'ombra). Deve leggersi nel Dock a 128 px e nel Finder a 32 e
   16 px. Prepara il disegno **a livelli** (fondo + uno o due livelli in primo piano) così che si possa portare in
   Icon Composer per gli effetti di vetro delle versioni recenti di macOS; consegna anche la versione piatta.
2. **Marchio per la barra**, 20 × 20 px, monocromo in `currentColor` (nell'app diventa lime o lilla a seconda della
   fase). È l'uso più severo: a 20 px il segno deve restare nitido, senza dettagli sotto 1,5 px. Se il marchio pieno
   non regge, disegna una variante semplificata dichiarata.
3. **Lockup orizzontale** marchio + wordmark, sul fondo melanzana.
4. **Versioni monocrome**: chiara (`#F5F0F6`) su fondo scuro e scura (`#19121B`) su fondo chiaro.
5. **Favicon** a 32 e 16 px.

## 7. Cosa evitare in tutte le proposte

- Cliché: note musicali, cuffie, microfoni, vinili, onde sinusoidali piene, barre da equalizzatore (è proprio il
  difetto dell'icona attuale), cervelli, scintille, stelline o "magia" per l'AI, gufi per il "wise".
- Somiglianze con marchi noti del settore audio e musica: Logic Pro, GarageBand, Ableton, FL Studio, Reaper,
  Audacity, iZotope, Spotify, SoundCloud, Splice, LANDR. Per ogni proposta indica in una riga il marchio esistente
  più vicino e perché non si confonde.
- Dettagli che spariscono sotto i 32 px, testo nell'icona, effetti 3D pesanti, fotorealismo.
- Font o risorse con licenze diverse da SIL OFL, MIT, Apache-2.0, BSD o ISC. Niente immagini raster generate da
  modelli di immagine come consegna finale: il logo è vettoriale, disegnato a tracciati.

## 8. Consegna

1. **Una pagina HTML di presentazione**, autonoma (un solo file, si apre con un doppio clic, niente server; i font
   da Google Fonts), sul fondo melanzana e con la palette della sezione 3. Una tavola per proposta (`A`, `B`, `C`,
   `D`) e una tavola finale di confronto. Ogni tavola mostra:
   - il marchio grande, con una riga sull'idea e una sulla costruzione (griglia, proporzioni);
   - l'icona macOS a 1024, 512, 128, 32 e 16 px **alle dimensioni reali**, e una simulazione nel Dock accanto ad
     altre icone neutre;
   - il marchio a 20 px dentro una riproduzione della barra in alto descritta in 4bis, una volta in Mix (lime su
     `#19121B`) e una in Master (lilla su `#141120`), con accanto un nome di progetto di esempio ("Paper Lanterns")
     e il selettore "Mix | Master";
   - il lockup nella schermata iniziale descritta in 4bis (marchio a 48 px, "Stemwise" a 40 px, slogan);
   - le versioni monocrome chiara e scura;
   - in poche righe: punti di forza, rischi, marchio esistente più vicino.

   La tavola di confronto mette le quattro icone fianco a fianco a 128, 32 e 16 px e i quattro marchi nella barra.
2. **Per ogni proposta, i file SVG**: `mark.svg` (pieno), `mark-20.svg` (versione per la barra, `fill`/`stroke` in
   `currentColor`, `viewBox` quadrato), `icon-1024.svg` (piatta), i livelli dell'icona (`icon-layer-bg.svg`,
   `icon-layer-fg.svg`, eventuale secondo livello), `lockup.svg`, `mono-light.svg`, `mono-dark.svg`. SVG puliti:
   solo tracciati, niente testo vivo, niente immagini incorporate, colori presi dalla sezione 3.
3. **Un breve testo** (in italiano) con le quattro idee in una riga ciascuna e l'elenco dei file.

## 9. Criteri con cui l'autore sceglierà

1. **Riconoscibilità**: si ricorda dopo averlo visto una volta? A 16 px nel Finder è ancora "quello di Stemwise"?
2. **Pertinenza**: dice qualcosa di *questo* prodotto (stem che diventano un brano, due fasi, l'assistente che
   propone, il mestiere) e non dell'audio in generale?
3. **Coerenza**: sta accanto alle icone e ai controlli dell'app come se fosse nato con loro?
4. **Calma**: è adatto a uno strumento in cui si passano ore al buio?
5. **Durata**: tra due anni sembrerà ancora giusto, o è una moda?

Prima di consegnare, verifica ogni proposta contro questi cinque criteri e contro la sezione 7, e correggi quello che
non regge. Se una direzione non dà un risultato all'altezza, dillo nella sua tavola invece di riempirla.
