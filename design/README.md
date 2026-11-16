# Stemwise — design

Consegna della sessione di design (FOUNDATION §7 e §10), 10 novembre 2026. Direzione scelta dall'autore:
**E · Atelier**.

Durante l'implementazione questo design è normativo per **aspetto e disposizione**; `docs/FOUNDATION.md` lo è per
il **comportamento**. Dove manca uno stato, lo si deriva da token e componenti esistenti e lo si annota in
`docs/app/DECISIONS.md`.

## Contenuto

| File | Cosa contiene |
| --- | --- |
| [tokens.css](tokens.css) | Tutti i token come variabili CSS: colori, tipografia, spaziatura, raggi, ombre, durate e curve, palette delle tracce, accenti delle due fasi, misure dei controlli e della shell, soglie dei gesti. |
| [components/components.css](components/components.css) | Gli stili di ogni componente, con le classi di stato. Usa solo token. |
| [components/index.html](components/index.html) | Il foglio dei componenti con tutti gli stati (tavole C1–C6). |
| [screens/screens.css](screens/screens.css) | Cornice delle tavole e disposizioni a livello di schermata. Usa solo token. |
| `screens/NN-*.html` | Una pagina per ogni schermata di FOUNDATION §6; ogni pagina contiene una tavola per stato. |
| `tools/` | Il generatore delle tavole (dati di esempio, forme d'onda, parti condivise). Non fa parte del prodotto. |
| [brand/](brand/README.md) | Il lavoro sul logo: brief, le quattro direzioni proposte, la scelta (D · Due stanze, A-075) e il prompt della sessione canvas che la rifinisce. |

Le tavole sono **HTML statico**: si aprono nel browser con un doppio clic, senza server e senza script. Ogni tavola
è una finestra dell'app a 1440×900 (1280×720 dove indicato), con un'etichetta (`3a`, `7c`…) e una riga che dice
cosa mostra.

Per rigenerarle dopo una modifica al generatore (serve Node 20 o più recente):

```bash
node design/tools/build.mjs
```

`design/tools/shot.sh <pagina.html> <uscita.png> [larghezza] [altezza]` produce un'immagine di una pagina con
Chrome senza interfaccia; è lo strumento usato per controllare le tavole.

## Direzione scelta: E · Atelier

Combinazione chiesta dall'autore dopo aver visto le quattro direzioni iniziali:

- **struttura** della direzione A: timeline in alto, mixer sotto, inspector e assistente come due colonne a destra;
- **palette e controlli** della direzione D: fondo melanzana, lime per il Mix, lilla per il Master, schede
  arrotondate, cap dei fader larghi, etichette in linguaggio semplice;
- **note numerate** della direzione B: lo stesso numero sull'intestazione della traccia, nell'inspector e
  nell'elenco delle note dell'assistente;
- **clip** della direzione B portate alla forma classica della regione: fascia del nome nel colore pieno della
  traccia, corpo tinto, forma d'onda chiara.

Le cinque direzioni esplorate restano nel canvas della sessione
(<https://claude.ai/artifact/KU16jqpeLSrN9Jpfrb29gp>, privato dell'autore). Non servono all'implementazione.

## Principi

1. **Due fasi, due stanze.** Mix è lime su melanzana, Master è lilla su indaco; cambia anche la disposizione
   (molte corsie contro una sola forma d'onda e una catena). Il selettore di fase è sempre in alto a sinistra e
   mostra sempre entrambe le fasi. Il passaggio è un dialogo esplicito (tavole 8a–8c).
2. **L'assistente propone, non guida.** Ogni sua scelta ha un numero, una frase e un diff. Tutto ciò che ha
   scritto o che gli è precluso usa un solo colore, l'azzurro `--ai`: note, corsie di automazione scritte da lui,
   lucchetti, avanzamento delle chiamate. Non c'è una trascrizione di chat: la richiesta a parole è un campo dentro
   lo strumento, con ambito e costo stimato accanto.
3. **Il colore indica suono o stato, mai decorazione.** Le tinte delle tracce stanno su clip, strip, intestazione e
   inspector; i meter sono verde menta; il resto è neutro.
4. **Leggibile da chi non è fonico.** Etichette in parole ("Mud", "Presence", "evens out 3 dB"), unità sempre
   scritte, controlli avanzati presenti ma ordinati secondo la catena del segnale.
5. **Riposante al buio.** Nessun bianco pieno e nessun nero pieno; testo minimo 5,4:1 sul fondo più chiaro;
   playhead e meter leggibili con la coda dell'occhio perché sono gli elementi più luminosi del loro riquadro.
6. **Ogni stato ha anche un segnale non cromatico**: icona, lettera, contorno o posizione.

## Indice delle tavole

| Schermata di §6 | File | Tavole |
| --- | --- | --- |
| 1 · Welcome | [screens/01-welcome.html](screens/01-welcome.html) | 1a recenti, nuovo, apri · 1b primo avvio con invito a configurare l'AI (saltabile, con demo offline) · con P-020 entrambe col collegamento `Take the tour` |
| 2 · New project / Import | [screens/02-new-project-import.html](screens/02-new-project-import.html) | 2a vuoto · 2b file trascinati sopra · 2c elenco con formato, frequenza, canali, durata, ruolo; avviso di frequenza; brief; riferimenti · 2d scelta sulla frequenza diversa |
| 3 · Mix workspace | [screens/03-mix-workspace.html](screens/03-mix-workspace.html) | 3a schermata principale · 3b pannello cronologia · 3c snapshot e A/B · 3d finestra minima 1280×720 · con P-013: 3e Overview del mixer · 3f progetto grande a corsie basse · 3g lo stesso in Overview; tutte le tavole portano i segni della catena, delle mandate e dell'automazione · Con P-011: sotto il campo della richiesta, la spunta "Allow structural changes" (ritorni e gruppi nuovi). Con P-018: due spunte sotto il campo della richiesta, "structural changes" e "creative ideas". |
| 4 · Proposta in revisione | [screens/04-mix-proposal.html](screens/04-mix-proposal.html) | 4a sintesi, valutazione, tracce con diff e motivazione, A/B, applica tutto / selezionate / scarta · 4b applicazione parziale Con P-011: 4a e 4b mostrano prima i ritorni e i gruppi che il piano crea ("New in this proposal"), ciascuno con spunta propria, tipo, motivo e chi lo usa; in 4b uno è escluso a mano e uno non viene creato perché restano selezionate meno di due delle sue tracce. · con P-018: 4c le idee creative (spunta spenta per default, blocchi nuovi nel diff della traccia) · 4d ascolto di una sola idea |
| 5 · AI al lavoro | [screens/05-mix-ai-working.html](screens/05-mix-ai-working.html) | 5a avanzamento per fase e per passata, annulla · 5b annullata |
| 6 · Editing | [screens/06-editing.html](screens/06-editing.html) | 6a selezione multipla · 6b trascinamento con snap · 6c taglio e rifilatura · 6d fade, gain di clip, clip in mute · 6e menu contestuali di clip e traccia · 6f strumento automazione e modifica delle sezioni |
| 7 · Returns & routing | [screens/07-returns-routing.html](screens/07-returns-routing.html) | 7a ritorno riverbero con selettore dell'algoritmo (plate, room, hall), decadimento, dimensione, pre-delay, damping, modulazione e filtri · 7b ritorno delay e creazione di un ritorno · 7c selettore dell'uscita e gruppi annidati in timeline e mixer · 7d sorgente sidechain e parametro da automatizzare · 7e compressione parallela (Mix del compressore sotto il 100 %) · con P-013: 7f chi manda a un ritorno · 7g dove va una traccia · 7h chi ascolta una traccia in sidechain |
| 8 · Bounce to Master | [screens/08-bounce-to-master.html](screens/08-bounce-to-master.html) | 8a conferma · 8b avanzamento · 8c esito con le misure del mixdown |
| 9 · Master workspace | [screens/09-master-workspace.html](screens/09-master-workspace.html) | 9a schermata principale · 9b mixdown non aggiornato · 9c proposta di master in revisione |
| 9 · Master workspace, preparazione | [screens/09-master-preparing.html](screens/09-master-preparing.html) | 9d preparazione dopo un bounce · 9e progetto riaperto, misure salvate, senza riferimenti · 9f ritorno al Mix durante la preparazione · 9g pronto mentre si è nel Mix · 9h riferimento illeggibile · 9i Master aperto con un riferimento escluso · 9j mixdown illeggibile · 9k nuova misura dopo una modifica (senza overlay) · 9v nuova destinazione (overlay) |
| 9 · Master workspace, livello d'ascolto e avviso sulla destinazione | [screens/09-master-listening.html](screens/09-master-listening.html) | 9l livello reale (predefinito) · 9m stessa loudness · 9n stessa loudness con abbassamento anti-clip · 9o proposta di master a pari loudness · 9p senza riferimenti · 9q destinazione molto più bassa del riferimento · 9r più riferimenti · 9s menu delle destinazioni · 9t dopo "Keep" · 9u destinazione molto più alta del riferimento |
| 10 · Export | [screens/10-export.html](screens/10-export.html) | 10a formati, frequenza, bit, dither, metadata con ISRC e copertina, cartella · 10b avanzamento · 10c rapporto · 10d export con l'avviso sulla destinazione · 10e rapporto dopo un export con lo scarto |
| 11 · Settings | [screens/11-settings.html](screens/11-settings.html) | 11a Audio · 11b AI · 11c Appearance · 11d Shortcuts |
| 12 · AI usage | [screens/12-ai-usage.html](screens/12-ai-usage.html) | 12a riepilogo per tipo e per modello, storia con filtri, menu di export · 12b dettaglio leggibile · 12c JSON grezzo · 12d vuoto · 12e superamento del tetto di spesa |
| 13 · Stati trasversali | [screens/13-states.html](screens/13-states.html) | 13a progetto vuoto · 13b piccoli stati vuoti · 13c caricamento · 13d file mancanti e dispositivo scollegato · 13e chiave assente · 13f rete assente · 13g conferme distruttive e recupero dopo un crash · 13h notifiche |
| 14 · Assistant setup (P-015, A-071) | [screens/14-assistant-setup.html](screens/14-assistant-setup.html) | 14a connessione con OpenRouter · 14b attesa del browser · 14c pagina mostrata nel browser · 14d credito e tetto di spesa · 14e pronto · 14f non approvato · 14g chiave incollata a mano, in tre passi · 14h chiave inserita · 14i credito esaurito · 14j limite della chiave raggiunto · 14k chiave non più valida; aggiornate 1b, 11b e 13e, che aprono il dialogo |
| 15 · Master chain (P-017, A-079, A-081; approvata e implementata in F14, A-082) | [screens/15-master-chain.html](screens/15-master-chain.html) | La catena come rack accanto all'editor, con blocchi ripetibili: 15a rack, clipper aperto con "Take off up to" · 15b riordino · 15i aggiungere un blocco · 15j menu del blocco (sposta, bypass, reset, rimuovi) · 15c compressore con Mix e filtro sul detector · 15d multibanda, da due a sei bande · 15h lo stesso con cinque · 15e scostamenti per sezione · 15f proposta con le misure prima → dopo, ordine, blocchi nuovi e sezioni · 15g la destinazione chiede troppo a questo mix. Sostituisce la riga dei moduli di 9a–9c; il resto della schermata 9 resta. |
| 15 · Master chain, esplorazione (A-080; scelta: R, A-081) | [screens/15-master-chain-directions.html](screens/15-master-chain-directions.html) | Lo stesso master con undici blocchi in tre disposizioni: G griglia con tessera "Add module" · R rack verticale accanto all'editor · P percorso con la barra dei picchi sopra il corpo dopo ogni blocco. Resta come memoria: le tavole 15 sono ridisegnate nel rack. |
| 16 · Track chain (P-018, A-086; approvate dall'autore il 2026-11-14) | [screens/16-track-chain.html](screens/16-track-chain.html) | 16a la catena nell'inspector, De-esser aperto · 16b menu dei blocchi per famiglia · 16c riordino · 16d Gate · 16e EQ dinamico con sidechain · 16f Transient · 16g caratteri della saturazione · 16h Bass enhancer · 16i Exciter · 16j Width · 16k Modulation · 16l Tremolo · 16m Harmonizer · 16n Doubler · 16o filtro risonante che si apre tra le sezioni · 16p catena di un ritorno: ducking · 16q riverbero gated · 16r delay ping-pong e tape · 16s riverbero shimmer · 16t segni con catene lunghe · 16u Overview con catene lunghe |
| 17 · Explain this mix (P-019, A-092; approvate dall'autore il 2026-11-15, implementate in F18, A-093) | [screens/17-mix-explain.html](screens/17-mix-explain.html) | 17a dove si apre (`Explain` nella barra in alto, ⌘E) · 17b tutto il mix: mappa del segnale e testo · 17c una traccia · 17d un effetto condiviso (ritorno) · 17e chi ascolta chi (sidechain) e un termine spiegato · 17f una sezione del brano · 17g progetto grande con gruppi ripiegati · 17h mix mai toccato · 17i finestra minima · 17j il testo intero |
| 18 · Guided tour (P-020, A-095, A-096; approvate dall'autore il 2026-11-15, implementate in F19, A-097) | [screens/18-tour.html](screens/18-tour.html) | i quindici passi: 18a scheda di apertura sulla Welcome · 18b barra in alto e trasporto · 18c timeline · 18d intestazione di traccia · 18e mixer · 18f inspector · 18g assistente · 18h proposta in revisione · 18i cronologia · 18j Explain · 18k selettore di fase e mixdown · 18l catena del Master · 18m loudness e A/B · 18n export · 18o scheda di chiusura; poi 18p passo che aspetta la proposta · 18q passo che aspetta il mixdown · 18r finestra minima · 18s collegamento nella Welcome · 18t menu dell'app con Help › Take the Tour (A-096) · 18u tour con un progetto aperto · 18v notifica dopo `Skip tour` · 18w progetto dimostrativo non preparabile |

Foglio dei componenti, [components/index.html](components/index.html):

| Tavola | Componenti di §6 |
| --- | --- |
| C1 · Controls | fader, knob, pan, meter (picco/RMS, riduzione di guadagno, LUFS), pulsanti, toggle (mute, solo, lock, bypass), controllo segmentato, campo numerico, altri campi, tag |
| C2 · Overlays and feedback | menu, tooltip, tab, dialogo, toast, barra di avanzamento, banner; livello d'ascolto del Master, sezione Listening, avviso e nota sulla destinazione |
| C3 · Timeline | clip, maniglie di fade, punto di automazione, sezione, righello, cursore di riproduzione, intestazione di traccia |
| C4 · Mixer and inspector | strip di canale, modulo di catena, curva EQ, curva di trasferimento, selettore di routing, indicatore di sidechain |
| C5 · Assistant and AI usage | scheda di proposta, riga di diff, nota, riga di chiamata AI, tabella di misure, campo della richiesta, passi di avanzamento |
| C6 · Foundations | palette delle due fasi, colori di stato, 12 tinte di traccia, tipografia, spaziatura, raggi, animazione, icone |
| C7 · Glance marks (P-013) | sigle dei cinque blocchi della catena (HP LP EQ CMP SAT) nei tre stati, compressore parallelo e con sidechain, punti delle mandate, segno di automazione, intestazione con i segni, celle dell'Overview, strip con la colonna delle etichette |
| C8 · Guided tour (P-020) | riflettore; fumetto di un passo (freccia, focus su Next), in attesa, dentro l'area illuminata; scheda di apertura e di chiusura |

Stati mostrati per ogni controllo: normale, hover, attivo, focus da tastiera, disabilitato e, dove ha senso,
automatizzato e bloccato per l'AI. Le classi `is-hover`, `is-active`, `is-focus`, `is-dis`, `is-auto`, `is-lock`,
`is-sel` esistono per mostrarli staticamente; nell'app corrispondono a `:hover`, `:active`, `:focus-visible`,
`[disabled]` e ad attributi di stato.

## Comportamenti non deducibili dalle tavole

### Ridimensionamento

Riferimento 1440×900, minimo 1280×720, fluido fino a 4K.

- **Fissi:** barra superiore (`--top-h`), colonne laterali (`--col-side`), intestazione delle tracce (`--head-w`),
  gutter (`--gap`).
- **Fluidi:** l'area delle corsie prende tutta la larghezza che resta; le strip del mixer si dividono la larghezza
  tra `--strip-w` e `--strip-w-max`; la timeline prende l'altezza che resta sopra il mixer.
- **Sotto 1360 px di larghezza (layout compatto, tavola 3d):** colonne laterali a `--col-side-min`, intestazioni a
  `--head-w-min`, strip a `--strip-w-min`, mini fader delle intestazioni nascosto, curva di trasferimento del
  compressore nascosta, scheda della motivazione nascosta nell'inspector (resta nell'assistente).
- **Sotto 800 px di altezza:** le corsie passano a `--lane-s`; a quell'altezza la clip perde la fascia del nome e
  l'intestazione la riga del ruolo.
- **Mixer:** altezza `--mixer-h`, ridimensionabile dall'utente tra `--mixer-h-min` e `--mixer-h-max` trascinando il
  bordo superiore. Sotto la larghezza minima delle strip scorre in orizzontale e il bus di mix resta fissato a
  destra.
- **Pannelli richiudibili, in quest'ordine di priorità** (il primo è quello che l'app propone di chiudere per primo
  quando lo spazio non basta, e ciascuno ha la sua scorciatoia): assistente (`⌘J`) → inspector (`⌘I`) → mixer
  (`⌘M`). Una colonna chiusa diventa una barra di `--rail`; il mixer chiuso diventa una striscia di soli meter
  alta `--bridge-h`. La timeline non si chiude mai.
- **Sopra 1920 px:** le colonne laterali crescono fino a `--col-side-max`; il resto va alla timeline. L'interfaccia
  non scala: su 4K si vedono più tracce e più tempo.
- **Master:** stessa regola per le colonne; forma d'onda e catena prendono la larghezza, l'editor del modulo
  l'altezza che resta.

### Densità

Tre altezze di corsia: `--lane-s` (26), `--lane` (38, predefinita), `--lane-l` (60). Lo zoom verticale passa da
una all'altra; le tavole 6a–6e usano quella alta. La corsia di automazione è sempre alta `--auto-h`. Master ha
volutamente meno elementi e numeri più grandi (`--fs-6`).

### Animazioni

Si animano solo `opacity` e `transform`. Nessuna ombra e nessuna sfocatura su elementi in movimento.

| Cosa | Durata | Curva |
| --- | --- | --- |
| hover, pressione | `--dur-1` | `--ease-out` |
| toggle, tab, menu in apertura | `--dur-2` | `--ease-out` (i menu si chiudono subito) |
| pannelli che si aprono o chiudono, dialoghi | `--dur-3` | `--ease-in-out` |
| cambio di fase Mix ↔ Master | `--dur-4` | `--ease-in-out`: dissolvenza incrociata dei due spazi di lavoro, l'accento cambia a metà |
| barra indeterminata | 1,2 s in ciclo | `--ease-linear` |

Playhead, meter, forme d'onda, curve EQ e automazione sono disegnati su canvas e non usano transizioni CSS: il
playhead si muove per `transform`, i meter salgono subito e scendono a 20 dB/s, il picco resta 2 s. Con
"Reduce motion" attivo restano solo le dissolvenze a `--dur-1`.

### Trascinamenti e gesti

- Un clic diventa trascinamento dopo `--drag-threshold`. `Esc` annulla il gesto in corso e non lascia passi di
  undo; ogni gesto concluso è un solo passo.
- **Controlli continui** (fader, knob, pan, campi numerici): trascinamento verticale, corsa completa in
  `--knob-travel`; `⇧` moltiplica per `--fine`; rotella per passi; doppio clic per il valore predefinito; clic sul
  valore per digitarlo, `↩` conferma, `Esc` annulla. Un valore fuori limite colora il bordo di rosso e non si
  applica.
- **Controllo automatizzato:** il cap o l'arco diventano rosa (`--auto`) e seguono la curva; mosso a trasporto
  fermo sposta l'intera curva.
- **Traccia bloccata per l'AI:** lucchetto azzurro acceso, clip attenuate; i controlli restano dell'utente.
- **Clip:** trascinare il corpo sposta (anche tra tracce dello stesso formato; la corsia di destinazione prende un
  contorno d'accento); i bordi rifilano; le maniglie di fade compaiono in hover agli angoli alti; `⌥` trascinando
  duplica. Lo snap aggancia entro `--snap-dist` a griglia, bordi di clip, cursore e confini di sezione, mostra una
  linea azzurra e un'etichetta, e si sospende tenendo `⌘`. Una clip lasciata sopra un'altra rifila quella sotto.
- **Selezione:** clic, `⇧`-clic per aggiungere o togliere, riquadro trascinando su una corsia vuota.
- **Automazione:** con lo strumento automazione, clic sulla curva aggiunge un punto, trascinare lo sposta,
  trascinare sul vuoto disegna a mano libera (semplificato in punti al rilascio), riquadro per selezionare un
  intervallo, `⌫` elimina. Le corsie scritte dall'assistente sono azzurre; appena l'utente tocca un punto la
  corsia diventa lime.
- **Sezioni:** trascinare sulla corsia vuota ne crea una; un confine si trascina spostando entrambe le vicine;
  doppio clic rinomina.
- **EQ:** le maniglie si muovono in frequenza e guadagno; la rotella cambia Q; doppio clic azzera la banda.

### Assistente

- Le note sono numerate nell'ordine in cui il piano le restituisce; il numero resta lo stesso finché la proposta
  (o il suo risultato applicato) è quella corrente. Clic su una nota seleziona la traccia; clic sul numero
  nell'intestazione apre la nota.
- Durante una chiamata lo spazio di lavoro resta utilizzabile; il campo della richiesta è sostituito da "Cancel".
- La proposta in revisione occupa il posto di inspector e assistente; timeline e mixer restano vivi per l'ascolto.
  L'A/B non ferma la riproduzione e non entra nella cronologia.
- Il selettore **Master** nel selettore di fase apre il dialogo 8a se non c'è un mixdown aggiornato; altrimenti
  cambia fase subito (con la preparazione 9d–9e se le misure del master non sono pronte).

### Notifiche, tooltip, menu, dialoghi

Toast in basso a destra, impilati verso l'alto, via dopo `--toast-life` se non hanno un'azione e non sono errori.
Tooltip dopo `--tip-delay` in hover, subito con il focus da tastiera. Nei dialoghi il focus è intrappolato, `Esc`
annulla, `↩` esegue l'azione primaria salvo quando è distruttiva.

### Preparazione del master (9d–9k)

Aggiunta del 11 novembre 2026, su richiesta dell'autore. Quando si entra in Master il mixdown e i riferimenti
vengono letti e misurati e il master viene portato alla loudness della destinazione; finché questo lavoro non è
finito lo spazio di lavoro del Master non è utilizzabile (forma d'onda vuota, loudness ignota) e non va mostrato a
metà.

**Quando appare l'overlay.** Ogni volta che la cache del master va (ri)costruita: "Open Master" dopo un bounce
(8c), il selettore Master su un progetto riaperto che ha già un mixdown, "Bounce again" da 9b, un riferimento
aggiunto, tolto o sostituito mentre si è in Master. Il lavoro parte appena il mixdown è scritto (già mentre si
legge 8c); su un progetto riaperto parte solo quando si entra in Master. L'overlay compare solo se l'attesa supera
`--wait-show` (250 ms); una volta comparso resta almeno `--wait-min` (600 ms), così non lampeggia. Se tutto è già
salvato nel progetto, Master si apre senza overlay.

**Cosa copre.** Tutto sotto la barra superiore: lo spazio di lavoro sotto è disegnato solo come gusci vuoti
(nomi di schede e moduli, nessun valore) dietro lo scrim. La barra superiore resta: nome, stato
`Mixdown N · preparing`, selettore di fase attivo; trasporto, posizione, undo/redo, A/B ed Export disabilitati
(`--dis`). Spazio, tasti del trasporto e `⌘Z`/`⇧⌘Z` non fanno nulla.

**Pannello** (componente dialogo di taglia m, ma `role="status"` e non modale):

| Elemento | Testo |
| --- | --- |
| Titolo | `Preparing the master` |
| Sottotitolo | `Mixdown 3 · 3:42 · 48 kHz · 2 references` (`1 reference`, `no references`) |
| Sotto la barra | a sinistra `38 %`; a destra `0:06 elapsed · about 12 s left`. Il tempo restante compare dopo il primo passo misurato (prima solo `0:06 elapsed`); sotto un secondo diventa `almost done` |
| Nota (con riferimenti) | `The master opens when every measure is ready, so the first thing you hear is already at the target loudness. Saved in the project, so this runs once per mixdown and reference.` |
| Nota (senza riferimenti) | `No references in this project, so there is nothing to convert or compare. Add one in Project settings.` |
| Piede | `Going back to Mix keeps this going.` e il pulsante `Back to Mix` con `Esc` |

La barra è determinata, nell'accento di fase (lilla, non l'azzurro `--ai`: non è l'assistente), pesata sul lavoro
stimato di ogni passo; la frazione la calcola il backend.

**Passi**, in quest'ordine. Un passo che non serve (nessun riferimento, riferimento già alla frequenza del
progetto o con la copia convertita già nel progetto) **non è elencato**. Un passo il cui risultato è già pronto è
elencato come fatto subito, con la provenienza al posto della durata.

| Passo | Etichetta | Riga secondaria | A destra |
| --- | --- | --- | --- |
| lettura | `Reading Mixdown 3` | — | `1.6 s` · `From the bounce` |
| misura del mixdown | `Measuring the mixdown` | `Loudness, peaks, spectrum, waveform` | `2.1 s` · `Saved in the project` |
| conversione (uno per riferimento) | `Converting Reference A to 48 kHz` | `From 44.1 kHz · the copy stays in the project` | `46 %` |
| misura dei riferimenti | `Measuring the references` | i nomi: `Reference A, Reference B` | `52 %` · `Saved in the project` |
| loudness | `Bringing the master to the target loudness` | `Streaming · −14 LUFS · −1 dBTP` | `70 %` |

Stati del passo: in attesa (cerchio col numero, `Waiting`), in corso (numero su accento, grassetto, percentuale),
fatto (spunta su verde, durata o provenienza), saltato con avviso (triangolo su ambra, `Skipped`), fallito (croce
su rosso, `Failed`), non iniziato dopo un errore (attenuato, `Not started`). Ogni stato ha anche l'icona o il
numero, non solo il colore.

**Fine.** All'ultimo passo la barra arriva al 100 %, l'overlay sfuma in `--dur-3` e sotto c'è già lo spazio di
lavoro completo (forma d'onda, misure, loudness, catena): appare tutto insieme, senza riempimenti progressivi. Il
focus torna al segmento Master del selettore. Con "Reduce motion" la dissolvenza è `--dur-1`.

**Esc e ritorno al Mix (9f, 9g).** `Esc`, "Back to Mix" e il segmento Mix fanno la stessa cosa: si torna al Mix
**senza annullare**. La preparazione continua sui thread ausiliari; il segmento Master mostra un anello con
l'avanzamento e il tooltip `Preparing the master · 64 % · keeps going while you mix`. Se finisce mentre si è nel
Mix compare il toast `Master is ready` / `Mixdown 3 is measured and at −14 LUFS for Streaming.` con l'azione
`Open Master` (resta finché non si chiude, perché ha un'azione); a quel punto Master si apre subito. Se si torna in
Master prima della fine, l'overlay riprende dallo stato corrente. Se la cache diventa inutile (nuovo bounce,
riferimenti cambiati) il lavoro riparte e i passi ancora validi restano fatti. Chiudere il progetto lo abbandona.

**Niente "Cancel".** Il lavoro non cambia il progetto (scrive solo cache), serve comunque per usare il Master e
fermarlo significherebbe rifarlo da capo al ritorno. L'unica uscita è "Back to Mix", che non butta via niente.

**Errori.**

- *Mixdown illeggibile* (file mancante, troncato, formato inatteso) — l'unico errore bloccante (9j). Titolo
  `The master can’t be prepared`; sottotitolo `Mixdown 3 could not be read: the file is missing from the project
  or damaged.`; il passo fallito porta la causa (`The file ends early (2:18 of 3:42)`, `The file is missing`,
  `The file is not a WAV file`); nota `Your mix and your master settings are safe. Bouncing again writes a new
  mixdown from the current mix and keeps every master setting.`; piede `Copy details` · `Back to Mix` ·
  `Bounce again` (primario, porta a 8b e poi a 9d). Nessuna barra.
- *Errore inatteso in un passo successivo* (memoria, errore interno) — stesso schema, sottotitolo `Something went
  wrong while measuring the references.` (il passo in minuscolo), primario `Try again`.
- *Riferimento illeggibile* — non blocca (9h): il passo diventa `Reference B can’t be read` / `The file is
  missing or damaged · skipped`, la preparazione continua, e Master si apre con il banner (9i)
  `Reference B can’t be read` / `The comparison, the match EQ and the A/B use Reference A only until you replace
  or remove it.` con `Project settings…` e `Not now`.
- Un errore mentre si è nel Mix arriva come toast di errore `The master can’t be prepared` con l'azione
  `Show`, che apre Master su 9j.

**Nuove misure dopo una modifica (9k): niente overlay.** Ogni cambio del master (un knob, un bypass, undo/redo
in Master; non la destinazione, vedi 9v) arriva subito alla catena e si sente; in background si rifà solo la ricerca
della loudness. Coprire lo spazio di lavoro a ogni gesto romperebbe il ciclo "tocca e ascolta", quindi lo stato è
in linea, nella scheda Loudness: etichetta `Measuring…` (orologio, accento) al posto di `On target`, barra
indeterminata, riga `Your change is already playing. The level match to −14 LUFS follows when the measure is
done.`, loudness integrata e true peak in `--tx-3` con la riga `before the change`. I meter in tempo reale
continuano. Anche qui lo stato compare solo dopo `--wait-show`. Mentre misura: A/B attivo (usa l'ultima misura),
"Master with AI" disabilitato con tooltip `Waiting for the loudness measure`, Export apribile ma col primario
`Measuring…` disabilitato finché la misura non arriva.

**Nuova destinazione (9v): overlay.** Aggiunta del 11 novembre 2026, su richiesta dell'autore (A-061). Cambiare
la loudness o il tetto della destinazione (un preset, i valori Custom, `Match the reference`, undo o redo di uno
di questi, una proposta di master che la imposta) è l'unica modifica che **non si sente subito**: il livello nuovo
arriva solo quando la ricerca del guadagno è finita. Con lo stato in linea di 9k l'autore ha continuato a lavorare
credendo che il cambio fosse già avvenuto. Per questo qui lo spazio di lavoro aspetta sotto lo stesso pannello
della preparazione, sopra lo spazio di lavoro **com'è** (non i gusci: i valori dietro lo scrim sono quelli della
destinazione di prima).

| Elemento | Testo |
| --- | --- |
| Titolo | `Bringing the master to the new target` |
| Sottotitolo | `Custom · −9.6 LUFS · −0.3 dBTP · was −14 LUFS, −1 dBTP` |
| Sotto la barra | a sinistra `42 %`; a destra `0:04 elapsed` (nessuna stima: il numero di passate non si conosce prima) |
| Passo 1 | `Finding the gain for −9.6 LUFS` (se cambia solo il tetto: `Finding the gain under the −0.3 dBTP ceiling`) / `The whole master is rendered and measured until it is on target` / percentuale |
| Passo 2 | `Measuring the result` / `Loudness, true peak, range` / `Waiting`, poi `Working…` |
| Nota | `Until this is done the master still plays at the level it had. The workspace comes back when the new level is in place, so what you hear and what you read are the new target.` |
| Piede | `Going back to Mix keeps this going.` e `Back to Mix` con `Esc` |

Regole: compare dopo `--wait-show` e resta almeno `--wait-min`, come 9d (una destinazione già misurata torna
subito, senza overlay). Durante il trascinamento di un valore Custom non compare: parte al rilascio. Barra
superiore: selettore di fase e **trasporto attivi** (si può fermare o ascoltare: suona il master al livello di
prima); undo/redo, A/B ed Export disabilitati; `⌘Z` e gli altri tasti non fanno nulla, `Esc` torna al Mix senza
fermare il lavoro e il segmento Master mostra l'anello come in 9f. Nessun "Cancel": a lavoro finito il cambio è un
passo di undo, e la destinazione di prima è già misurata, quindi tornarci è immediato. Alla fine il focus torna al
selettore della destinazione. Se il cambio avviene dentro il dialogo di export (10d) l'overlay non compare: lì
aspetta il pulsante primario. Campo del backend: `MasterStatus.retarget` (`from_lufs`, `from_true_peak_db`,
`fraction`, `elapsed_ms`), presente solo durante l'attesa.

**Accessibilità.** Il pannello è `role="status"` con `aria-live="polite"`: si annuncia il passo che inizia, non
le percentuali. La barra ha `role="progressbar"` con il valore. All'apertura il focus va su "Back to Mix"; `Tab`
raggiunge anche il selettore di fase.

**Campo richiesto al backend** (oggi c'è solo `MasterStatus.analysing`). Proposta: `MasterStatus.prep:
MasterPrep | null`, presente solo durante la costruzione della cache, con `mixdown` (numero), `fraction` (0–1,
pesata), `elapsed_ms`, `eta_ms | null`, `error | null` (`kind`: `mixdown_unreadable` o `internal`, `step`,
`detail`) e `steps: PrepStep[]`; ogni `PrepStep` ha `kind` (`read_mixdown`, `measure_mixdown`,
`convert_reference`, `measure_references`, `loudness`), `state` (`waiting`, `running`, `done`, `skipped`,
`failed`, `not_started`), `source` (`bounce`, `saved` o nessuna), `fraction`, `ms`, `reference` (id e nome) e
`from_rate` dove servono. I testi li compone la UI dal `kind`. Regola: `prep` presente ⇒ overlay; `prep` assente
e `analysing` vero ⇒ stato in linea di 9k.

### Livello d'ascolto e avviso sulla destinazione (9l–9u, 10d–10e)

Aggiunta del 11 novembre 2026, su richiesta dell'autore. Prima il Master suonava sempre a pari loudness (MAS-5): il
livello vero del master non si sentiva mai, e una destinazione Custom a −18 LUFS contro un riferimento a −9,6 LUFS è
passata inosservata fino all'ascolto del file esportato.

**Due modi d'ascolto.** Nella barra del Master, prima del selettore della sorgente A/B, un controllo segmentato
`quiet` della stessa altezza: `Real level | Same loudness` (il secondo con l'icona `level`, "=").

- **Real level (predefinito).** Il master suona esattamente come il file esportato; mixdown e riferimenti al loro
  livello reale, senza guadagno né abbassamento.
- **Same loudness (a richiesta).** Il comportamento di MAS-5 di prima: mixdown e riferimenti portati alla loudness
  misurata del master; se così un picco supera −1 dBTP, tutte le sorgenti, master compreso, scendono di
  `ab_trim_db`.
- Lo stato si legge a colpo d'occhio: il segmento acceso (in Same loudness con fondo e contorno d'accento e "="),
  l'intestazione della sezione Listening e il verbo delle righe (`quieter`/`louder` contro `raised`/`lowered`).
- **Scorciatoia `⇧B`** (`Same loudness on / off`); `B` resta all'A/B di proposta e snapshot (la regola di `B` deve
  escludere `⇧`). Il tooltip del segmento: `Compare at the same loudness: quieter sources are raised, louder ones
  lowered. ⇧B`.
- Il modo non è nel progetto e non è un passo di undo: a ogni apertura del progetto si parte da Real level; resta
  quello scelto finché il progetto è aperto, anche passando dal Mix. Cambiare modo non ferma la riproduzione (come
  l'A/B, senza click).
- Durante la preparazione (9d) il controllo è disabilitato con il resto della barra.
- Con due o più riferimenti i segmenti della sorgente diventano `Ref A`, `Ref B` (nome intero nel tooltip e nella
  sezione Listening).

**Sezione Listening della scheda Loudness**, sotto loudness integrata e true peak: intestazione `Listening` ·
`Real level` (o `= Same loudness` in accento) e a destra `⇧B same loudness` (`⇧B real level`). Una riga per
sorgente: nome, loudness integrata misurata, scarto a parole. La riga che si ascolta ha l'icona dell'altoparlante e
il fondo `--bg-2`.

| Modo | Master | Mixdown, riferimenti |
| --- | --- | --- |
| Real level | `as exported` | scarto in LU contro il master: `4.5 LU quieter`, `1.2 LU louder` |
| Same loudness | `unchanged`, o `lowered 2.5 dB` con l'abbassamento | guadagno con cui suona: `raised 2.8 dB`, `lowered 1.2 dB` (abbassamento compreso) |

Con l'abbassamento attivo, sotto le righe: `Raising the mixdown 4.5 dB would push its peaks over −1 dBTP, so
everything plays 2.5 dB lower. The export is not affected.` (la sorgente che lo causa, con il suo guadagno). Durante
la revisione di una proposta di master, quando si ascolta la proposta la prima riga è `Proposal`.

**Proposta in revisione (9c, 9o).** Il selettore `Current master | Proposal` della colonna Assistant segue lo stesso
modo globale e non ha un interruttore proprio. Sotto, a livello reale: `Real level` / `The proposal is 0.1 LU
louder: −14.0 against −14.1 LUFS. B switches · ⇧B same loudness`; a pari loudness: `= Same loudness` / `The proposal
plays 0.1 dB lower to match the current master. B switches · ⇧B real level`.

**Avviso sulla destinazione.** I riferimenti (quelli misurati; gli esclusi di 9h no) definiscono un intervallo di
loudness integrata, dal più basso al più alto. Si confronta la **loudness della destinazione** (non la misura del
master), quindi l'avviso è immediato e non aspetta la ricerca della loudness.

- Destinazione **più di 3 LU sotto il riferimento più basso** → avviso ambra (`banner stack`): `Much quieter than your
  reference` / `At −18 LUFS this master will sound 8.4 LU quieter than Reference A (−9.6 LUFS): clearly softer when
  the two play one after the other.` Con più riferimenti: `Much quieter than your references` / `At −18 LUFS this
  master will sound quieter than both: 6.8 LU below Reference B (−11.2 LUFS) and 8.4 LU below Reference A (−9.6
  LUFS).` (con tre o più: `than all of them`).
- Destinazione **più di 3 LU sopra il riferimento più alto** → nota neutra (`banner stack info`): `Much louder than
  your reference` / `At −9 LUFS this master will be 5.2 LU louder than Reference A (−14.2 LUFS). Getting there takes
  more limiting, which can flatten the punch, and streaming services turn loud songs down anyway.` Solo nella scheda
  Loudness e nel menu: il troppo forte si sente subito a livello reale e si vede nella riduzione del limiter.
- Dentro l'intervallo, entro 3 LU dai bordi o senza riferimenti (9p): niente.
- **Azioni:** `Match the reference(s)` (primario) porta la destinazione al bordo più vicino dell'intervallo (il
  riferimento più basso se sotto, il più alto se sopra), come Custom e **con il tetto di prima**; un solo passo di undo
  (`Target matched to the references`). Riga sotto: `Sets the target to −9.6 LUFS and keeps the −1 dBTP ceiling · one
  undo step`. `Keep −18 LUFS` (secondario) tiene la destinazione.
- **Dove compare:** sotto il selettore Target della scheda Loudness, con il selettore a bordo ambra e il triangolo
  (9q, 9r); la riga del riferimento nella sezione Listening in ambra col triangolo; nel menu delle destinazioni (9s)
  ogni voce dice dove sta (`2.8 LU quieter`, `0.6 LU louder`, `Between your references`, in ambra col triangolo oltre
  i 3 LU) e c'è la voce `Match the references · −11.2 LUFS` (`Keeps −1 dBTP`); nel dialogo di export (10d) come
  banner sopra i formati, con le stesse azioni: Export resta disponibile, e "Match the reference" lì porta il primario
  a `Measuring…` come in 9k; nel rapporto (10e) una riga ambra `8.4 LU quieter than Reference A. … If that wasn’t
  intended, choose “Match the reference” in the Loudness card and export again.`
- **Keep** vale per il progetto: salvato nel progetto, è un passo di undo della cronologia del Master (`Kept the
  target`). Da quel momento nessun avviso né nota per la destinazione, in nessun posto; sotto il selettore resta la
  riga discreta `8.4 LU quieter than Reference A · kept on purpose. Warn me again` (9t) finché lo scarto supera la
  soglia, e il rapporto di export scrive in neutro `Exported at −18 LUFS on purpose, 8.4 LU quieter than Reference
  A.` "Warn me again" toglie il Keep (anch'esso un passo di undo).

**Da decidere in implementazione.** Se i meter della scheda Loudness misurano prima o dopo il guadagno d'ascolto;
proposta: prima, cioè sempre il livello reale della sorgente, perché i numeri della scheda parlano del file. Campi
richiesti al backend: il modo d'ascolto (`MasterStatus.listen_same: bool` e il comando per cambiarlo), la loudness
del master (già in `measures`), dei riferimenti (già in `references`) e del mixdown (già in `mixdown_measures`), il
guadagno di ogni sorgente a pari loudness (oggi si deduce da `ab_trim_db`), e nel progetto il Keep
(`master.target_kept: bool`).

## Font e icone

- **Bricolage Grotesque** (titoli) e **Figtree** (tutto il resto): entrambe SIL Open Font License 1.1, quindi
  includibili nell'app. Le tavole le caricano da Google Fonts; **l'app deve includerne i file** (pesi 500 e 600
  di Bricolage Grotesque; 400, 500 e 600 di Figtree) con i testi di licenza. Cifre tabulari ovunque.
- Il JSON grezzo usa il font monospazio di sistema, che non viene distribuito.
- **Icone:** insieme proprio, disegnato per questo progetto (griglia 16 px, tratto 1,5 px, estremità arrotondate).
  I tracciati sono in `tools/lib.mjs` (oggetto `IC`) e tutte le icone sono in C6.

## Decisioni prese in autonomia

| N. | Decisione | Motivo |
| --- | --- | --- |
| D1 | Il mixer è un pannello inferiore, non una vista alternativa. | Mantiene visibile insieme a timeline e inspector lo stesso stato (TRK-4). |
| D2 | La proposta in revisione sostituisce inspector e assistente con un unico pannello largo. | Servono sintesi, valutazione e 15 schede leggibili; timeline e mixer devono restare per l'ascolto. |
| D3 | Le corsie scritte dall'assistente sono azzurre, quelle disegnate a mano prendono l'accento di fase. | Rende visibile chi ha scritto cosa senza testo. |
| D4 | Il selettore di fase dice "Mix" e "Master", senza numeri. | Richiesta dell'autore. |
| D5 | Le bande dell'EQ di traccia hanno un nome in parole (Mud, Presence, Harsh, Air) derivato dalla frequenza; i valori numerici restano. | L'utente non è un fonico. |
| D6 | La catena nell'inspector è una griglia di 8 blocchi nell'ordine fisso di FX-1 (fader e pan in un solo blocco); l'uscita è un selettore nell'intestazione. | In una riga sola i valori si tagliavano. |
| D7 | In Master la destinazione sta nella scheda Loudness, non nella barra superiore. | La barra non conteneva insieme A/B, destinazione ed export a 1440 px. |
| D8 | La catena di master è una griglia 4×2 numerata 1–8. | Otto riquadri in una riga tagliavano i nomi. |
| D9 | Nel Master la forma d'onda mostra il master (acceso) sopra il mixdown (grigio). | Fa vedere a colpo d'occhio quanto lavora la catena. |
| D10 | Il nome delle clip è a 9 px in grassetto. | Richiesta dell'autore; è l'unico testo sotto gli 11 px ed è ridondante con l'intestazione della traccia. |
| D11 | Sotto 32 px di corsia la clip perde la fascia del nome. | Non resterebbe spazio per la forma d'onda. |
| D12 | I meter dei canali sono a tinta unita: menta, ambra sopra −6 dBFS, cappuccio rosso sull'over. | Nessun gradiente su elementi che cambiano a 30 Hz. |
| D13 | Layout compatto sotto 1360 px e ordine di chiusura dei pannelli (assistente, inspector, mixer). | Vedi "Ridimensionamento". |
| D14 | La demo offline è proposta già al primo avvio. | AI-9: il decisore finto è selezionabile; evita un vicolo cieco a chi non ha una chiave. |
| D15 | Il dialogo di recupero dopo un crash e il ricollegamento dei file mancanti sono disegnati in 13g. | PRJ-2 e UI-6 li richiedono, §6 non li elenca come schermate. |
| D16 | Il risultato del bounce (8c) usa già l'accento del Master sul pulsante primario. | Segna il momento del passaggio di fase. |
| D17 | Aggiunti token per i gesti (soglia di trascinamento, distanza di snap, corsa dei knob, moltiplicatore fine, ritardi). | Sono misure che l'implementazione deve condividere. |
| D18 | Le tavole sono generate da uno script e consegnate come HTML statico. | Quarantanove tavole con forme d'onda realistiche non sono mantenibili a mano; l'HTML risultante non ha dipendenze. |
| D19 | La preparazione del master copre il corpo dello spazio di lavoro, non la barra superiore. | Il selettore di fase deve restare usabile per tornare al Mix; il resto della barra è disabilitato. |
| D20 | Nessun "Cancel" durante la preparazione: "Back to Mix" (e `Esc`) torna al Mix e il lavoro continua. | Il lavoro non tocca il progetto e serve comunque; annullarlo farebbe solo rifarlo. |
| D21 | Le nuove misure dopo un cambio del master non usano l'overlay ma uno stato in linea nella scheda Loudness. | La catena suona già la modifica; bloccare a ogni gesto renderebbe il Master inutilizzabile. |
| D22 | Un riferimento illeggibile è un avviso e viene saltato; solo il mixdown illeggibile blocca. | Senza riferimenti il master funziona; senza mixdown no. |
| D23 | I passi che non servono non sono elencati; quelli già pronti compaiono fatti, con la provenienza. | L'elenco descrive quello che succede davvero e spiega perché è veloce. |
| D24 | Il passo in corso e la barra della preparazione usano l'accento di fase, non l'azzurro `--ai`. | L'azzurro è riservato a ciò che fa l'assistente. |
| D25 | Token `--wait-show` e `--wait-min` per le attese in background. | Evitano che overlay e stato in linea lampeggino; sono valori che l'implementazione deve condividere. |
| D26 | Il Master suona al livello reale per impostazione predefinita; la pari loudness (MAS-5) è un modo a richiesta, `⇧B`. | L'utente deve sentire il master come il file esportato; il confronto a pari loudness resta per giudicare il suono. |
| D27 | Il modo d'ascolto e la sorgente sono due controlli segmentati della stessa altezza nella barra; gli scarti stanno nella sezione Listening della scheda Loudness. | Con gli scarti nei segmenti la barra non conteneva due riferimenti a 1440 px; la scheda Loudness è dove si parla di livelli e li dice a parole. |
| D28 | Un solo modo d'ascolto per sorgenti e proposta; il selettore della proposta non ha un interruttore proprio. | Un solo posto per un solo stato. |
| D29 | L'avviso confronta la destinazione con l'intervallo dei riferimenti: oltre 3 LU sotto il più basso avviso, oltre 3 LU sopra il più alto nota neutra; la correzione porta al bordo più vicino e tiene il tetto. | Avvisa solo quando la destinazione è fuori da tutti i riferimenti; il troppo forte si sente subito e non serve all'export. |
| D30 | "Keep" vale per tutto il progetto ed è un passo di undo; resta una riga discreta con "Warn me again". | Nessuna insistenza, ma lo scarto resta visibile e tracciato nel rapporto di export. |
| D31 | Con due o più riferimenti i segmenti si accorciano in `Ref A`, `Ref B`. | Spazio della barra; il nome intero è nel tooltip e nella sezione Listening. |

Nomi, titoli, percorsi e valori nelle tavole sono contenuto di esempio.

## Verifica dell'inventario (FOUNDATION §6)

Schermate: 1 → 1a–1b · 2 → 2a–2d · 3 → 3a–3d · 4 → 4a–4b · 5 → 5a–5b · 6 → 6a–6f · 7 → 7a–7e · 8 → 8a–8c ·
9 → 9a–9u · 10 → 10a–10e · 11 → 11a–11d · 12 → 12a–12e · 13 → 13a–13h. Totale 69 tavole di schermata e 6 di
componenti.

Dettagli di §6 e dove si trovano:

- barra superiore (nome, selettore di fase, trasporto, posizione, loop, undo/redo, meter del bus, carico): 3a;
- timeline (righello, sezioni, intestazioni, clip, automazione, cursore, loop): 3a, C3;
- mixer (meter, fader, pan, mandate, riduzione di guadagno; gruppi, ritorni, bus): 3a, 7c, C4;
- inspector (catena con bypass, EQ, compressore, saturazione, mandate, motivazione): 3a;
- pannello AI (brief, riferimenti, Auto mix, richiesta a parole, stima, costo con accesso ad AI usage): 3a;
- pannello cronologia (passi, snapshot, A/B): 3b, 3c;
- creazione ed eliminazione di un ritorno: 7b (menu), 7a (pulsante), 13g (conferma);
- errori di UI-6: file mancanti 13d e 13g · frequenze diverse 2c e 2d · chiave assente 13e · rete assente 13f ·
  dispositivo scollegato 13d.

Componenti: tutti i 29 elencati in §6 sono in C1–C5 (tabella sopra).

## Limiti noti

- Le tavole sono state controllate renderizzandole con Chrome a 1440 px di larghezza; non sono state provate in
  altri browser né nella webview di Tauri.
- Il contrasto è stato calcolato dai token (testo da 5,4:1 a 16,5:1 sui quattro fondi di entrambe le fasi; testo
  scuro su accenti, stati e tinte da 6,3:1 a 13,2:1). Gli stati disabilitati e le clip in mute sono volutamente
  sotto soglia.
- Le posizioni di clip, punti e menu nelle tavole sono coordinate di esempio scritte in linea: non sono misure
  di design. Le misure sono solo nei token.
- Gli stati hover, attivo e focus sono statici: le transizioni descritte sopra non sono dimostrate.
- Nel mixer a 1440 px i nomi lunghi delle strip sono troncati; il nome intero è nel tooltip e nell'intestazione
  della traccia.

## Explain this mix (schermata 17, P-019)

Aggiunta del 15 novembre 2026, chiesta dall'autore: l'interfaccia mostra lo stato del mix ma non il flusso né
l'intento, e chi non è fonico non capisce chi va dove e perché. Approvata il 2026-11-15 e implementata in F18; le differenze
dall'implementazione sono in A-093.

**Cos'è.** Una vista del Mix, aperta da `Explain` nella barra in alto (⌘E, `Esc` per chiudere), che prende il posto
di timeline, mixer e pannelli. La barra in alto resta viva: si ascolta mentre si legge. A sinistra la **mappa**, a
destra lo **stesso mix in parole**. Tutto è letto dal documento di progetto e dall'analisi già in cache: **nessun
modello**, nessuna chiamata, nessun costo; stesso progetto, stessa pagina. È di sola lettura: non crea passi di undo.

**Mappa.** Tre colonne, da sinistra a destra: tracce raggruppate per famiglia, gruppi (accanto ai loro membri),
bus di mix; gli effetti condivisi (ritorni) stanno sotto i gruppi. Un livello di annidamento dei gruppi in più
aggiunge una colonna.

| Segno | Significato |
| --- | --- |
| Linea piena nel colore della traccia | Uscita. Lo spessore segue il livello (2–9 px). |
| Linea tratteggiata | Mandata: una copia verso un effetto condiviso. |
| Linea punteggiata con freccia, nel colore della sorgente | Sidechain: chi ascolta chi. |
| Rombo azzurro / lime | Automazione scritta dall'assistente / a mano (come UI-8). |
| Numero azzurro | Nota dell'assistente, lo stesso numero di intestazioni e inspector. |
| Riga col bordo e `›` | Gruppo ripiegato: le sue tracce in una riga, con il conteggio. |

Un clic su un riquadro lo seleziona: restano accesi il suo percorso fino al bus, le mandate con il livello scritto
sopra e le sidechain; il resto si attenua. Doppio clic: chiude la vista e apre l'elemento nell'inspector. La
striscia sopra la mappa sceglie tutto il brano o una sezione (le ripetizioni contano come una): le tracce che lì
non suonano si attenuano, quelle che suonano solo lì o hanno valori diversi portano un'etichetta.

**Testo.** Sempre nello stesso ordine; una parte senza contenuto non compare.

| Parte | Da dove viene |
| --- | --- |
| In short | Conteggi del documento; traccia e famiglia più forti dall'analisi; pan; differenze tra sezioni. |
| How it is built | Gruppi con i membri e i blocchi accesi sul gruppo; ritorni con tipo, parametri e chi manda; tracce dirette al bus. |
| Who is in front | Loudness per famiglia (AI-22, già calcolata). |
| Left to right | Pan di ogni traccia. |
| Space | Mandate per ritorno, dalla più alta alla più bassa. |
| Who reacts to whom | Sidechain del documento; riduzione media misurata dal motore. |
| What changes along the song | Clip presenti per sezione, automazione e scostamenti per sezione. |
| Reasons | `summary` e `rationale` dell'ultima proposta applicata, **citati** come sono (nella lingua del brief). |

Selezionando una traccia, un gruppo o un ritorno il testo diventa la sua scheda: dove va, il suo posto nel mix, i
blocchi della catena **in ordine di segnale** (sigla, nome, valore, una frase su cosa fa), il motivo dell'assistente
e ciò che è stato cambiato a mano dopo. Selezionando una sezione: cosa suona, cosa è diverso, loudness a confronto.

**Regole del testo.** Ogni frase è un modello fisso riempito con valori del progetto: una frase per tipo di blocco
(per esempio compressore: `Turns the loudest {words|hits|notes} down…: about {GR} dB on the peaks`, con il
sostantivo scelto dal ruolo della traccia). I nomi di tracce, gruppi e ritorni sono **chip** col colore
dell'elemento: un clic lo seleziona nella mappa. I termini tecnici (`group`, `shared effect`, `sidechain`,
`compressor`, `parallel compressor`…) hanno la sottolineatura punteggiata e un tooltip di una frase, da un glossario
fisso. Il testo **descrive e non giudica**: nessun consiglio, nessun aggettivo di qualità. I motivi sono solo quelli
che l'assistente ha scritto; un cambio fatto a mano non ha motivo. `Copy as text` copia il testo semplice.

**Misure.** Colonna del testo 496 px (`--col-explain`, 432 al minimo); riga di una traccia nella mappa 30 px
(`--map-node`), 26 con molte tracce, 22 nella finestra minima; oltre, la mappa scorre in verticale.

**Fuori da queste tavole.** La narrazione scritta da un modello (locale o remoto) sopra questi fatti: decisione
rimandata dall'autore. Il Master non ha ancora la sua versione della vista.

## Tour guidato (schermata 18, P-020)

Aggiunta del 15 novembre 2026 (issue #5, specifica in FOUNDATION: P-020, PRJ-9, UI-17 → UI-19). Tavole 18a–18w e
componente C8; scelte in A-095 e A-096. Approvate dall'autore il 2026-11-15 e implementate in F19; le differenze
dall'implementazione sono in A-097 (il "Try it" del passo 8 non nomina più il tasto B, sospeso durante il tour).

**Cos'è.** Un giro dell'app in quindici passi su un **brano dimostrativo** che l'app costruisce e sintetizza da sé
(PRJ-9): non è un progetto dell'utente, non si salva, sparisce alla fine. Parte da solo al primo avvio (scheda 18a,
sulla Welcome) e a richiesta da `Help › Take the Tour` (18t) e dal collegamento della Welcome (18s). Ogni passo ha un
riflettore sull'area di cui parla e un fumetto accanto; si va avanti con `Next`, nessun passo aspetta un gesto. I
testi definitivi dei passi sono in `tools/s-tour.mjs` (`STEPS`), da copiare così come sono nelle stringhe dell'app.

**Il brano delle tavole.** Le tavole usano le dodici tracce del brano di esempio (`lib.mjs`) su 68 s con tre sezioni
(Verse, Chorus, Outro), due gruppi, due ritorni, la sidechain del basso sulla cassa e la corsia di automazione di Lead
Vox disegnata a mano: è ciò che PRJ-9 chiede di mostrare. Nome del progetto `Demo song`, stato `Tour · nothing is
saved`, nessun riferimento, nessun costo (`Offline demo · no cost`). Le tavole del Master e di Explain riprendono il
resto del brano di esempio (catena, misure, testo): sono contenuto di esempio.

**I passi e lo stato che stabiliscono.** Ogni passo dichiara lo stato che gli serve e lo imposta entrando, sia
avanzando sia tornando indietro (UI-18). Il bersaglio si trova per **ruolo e nome accessibile** (`aria-label` delle
aree, ruolo `dialog`, il pulsante Play), mai per id.

| Passo | Tavola | Bersaglio | Stato che il passo imposta | Lato del fumetto |
| --- | --- | --- | --- | --- |
| 1 Welcome to Stemwise | 18a | nessuno: scheda al centro | Welcome; il brano si prepara in background | — |
| 2 Play the song | 18b | barra in alto, freccia su Play | Mix, progetto dimostrativo aperto, testina all'inizio del Chorus | sotto |
| 3 Tracks on a timeline | 18c | `Timeline` | pannelli aperti, zoom intero | destra |
| 4 A track | 18d | intestazione di Lead Vox | Lead Vox selezionata | destra |
| 5 The mixer | 18e | `Mixer` | mixer aperto in vista Faders | destra |
| 6 The inspector | 18f | `Inspector` | inspector aperto su Lead Vox, EQ aperto | sinistra |
| 7 The assistant | 18g | `Assistant`, freccia sul campo della richiesta | scheda Notes, nessuna proposta | sinistra |
| 8 A proposal to review | 18h (18p in attesa) | `Proposal` | proposta del decisore finto in revisione, A/B sulla proposta | sinistra |
| 9 One step to undo | 18i | `Assistant` | proposta applicata (un passo), scheda History | sinistra |
| 10 Explain this mix | 18j | tutto sotto la barra | vista Explain sul mix intero | dentro |
| 11 Mix, then Master | 18k (18q in attesa) | selettore di fase | Mix, Explain chiusa, mixdown fatto o in corso | sotto |
| 12 The master chain | 18l | rack ed editor | Master, Clipper aperto | destra |
| 13 Loudness and A/B | 18m | scheda `Loudness` | Master, A/B sul master | sinistra |
| 14 Export | 18n | dialogo di export | dialogo aperto e spostato a destra, Export spento | sinistra |
| 15 Ready for your own songs | 18o | nessuno: scheda al centro | Master sotto l'attenuazione; `Done` chiude | — |

Tornando indietro un passo disfa solo ciò che il passo successivo aveva aggiunto e il precedente non dichiara: da 9 a
8 la proposta torna in revisione (undo dell'applicazione); da 12 a 11 si torna al Mix; da 14 a 13 il dialogo si chiude.
Ciò che l'utente ha cambiato dentro l'area illuminata (un fader, un blocco spento) resta finché un passo non dichiara
il contrario; tutto si perde alla fine.

**Riflettore.** Un livello sopra tutta la finestra (`--z-tour`, sopra dialoghi e toast). Il buco è il bersaglio
allargato di `--tour-pad` (4 px), con il raggio delle schede (`--r-3`), un anello di 2 px nell'**accento di fase**
(lime nel Mix, lilla nel Master: il tour non è l'assistente, quindi niente `--ai`) e attorno l'attenuazione
`--tour-dim`, più leggera dello scrim dei dialoghi perché il contesto resti leggibile. Il buco non esce dalla
finestra: su un'area larga quanto la finestra (la barra in alto) è tirato dentro quanto basta a far vedere l'anello.
Dentro il buco l'app funziona (Play, un fader, un blocco); fuori i clic non fanno niente. Il tour non usa lo scrim dei
dialoghi: un dialogo illuminato (export) perde il suo e si sposta al bordo destro, a `--gap` dal bordo, per lasciare
posto al fumetto (18n). Le schede di apertura e chiusura attenuano tutta la finestra.

**Fumetto: posizione.** Larghezza `--tour-w` (340). Si prova un lato dopo l'altro e si prende il primo in cui il
fumetto entra con `--tour-gap` (12) dal buco e almeno `--tour-edge` (16) dalla finestra: prima il lato preferito dal
passo (sotto per i bersagli della barra in alto), poi **destra, sinistra, sotto, sopra**. Sul lato scelto il fumetto è
centrato sul bersaglio (o sul suo punto d'ancoraggio, per esempio Play) e scivola lungo il lato finché sta tutto nella
finestra. La freccia (quadrato di `--tour-arrow` ruotato) sta sul lato rivolto al bersaglio e punta all'ancoraggio,
mai a meno di 24 px dagli angoli del fumetto. Se nessun lato basta (Explain, 18j) il fumetto sta **dentro** il buco, in
basso a sinistra a 16 px dai suoi bordi, senza freccia. L'altezza è quella reale del fumetto, misurata prima di
posizionarlo. Alla finestra minima (18r) prima si porta in vista il bersaglio (scorrimento della timeline o del
mixer), poi si posiziona il fumetto con la stessa regola; a ogni ridimensionamento della finestra si ricalcola.

**Fumetto: contenuto.** Contatore `3 of 15` con una barra di quindici segni (fatti nell'accento, il corrente chiaro);
titolo nel carattere dei titoli; una o due frasi; una riga `Try it: …` facoltativa quando l'area invita a un gesto
(passi 2, 5, 8); in fondo `Skip tour`, `Back`, `Next` (`Done` all'ultimo, senza `Skip tour`). La scheda di apertura ha
`Skip tour` e `Start`, la riga sul suono sintetico e i tasti del tour.

**Passi che aspettano (18p, 18q).** Proposta e mixdown si preparano in background dall'avvio del tour. Se un passo
arriva prima che siano pronti, il fumetto aggiunge una riga di stato (`Preparing the proposal…` in azzurro, perché la fa
l'assistente; `Bouncing the mix to the mixdown…` con barra determinata nell'accento di fase) e `Next` è spento; appena
il risultato c'è il passo lo mostra da solo e `Next` si accende. `Back` e `Skip tour` restano. Il passo 8 in attesa
illumina l'assistente al lavoro (stessa vista di 5a); il Master al passo 12 può ancora mostrare la preparazione di 9d.

**Animazioni.** Solo `opacity` e `transform`, come nel resto dell'app. Al cambio di passo il fumetto e l'anello
svaniscono in `--dur-1`; il buco si sposta e si ridimensiona con un `transform` (traslazione e scala) in `--dur-3`
`--ease-in-out`; il fumetto ricompare al suo posto in `--dur-2` `--ease-out`. Il cambio di fase (passi 11 → 12) è la
solita dissolvenza di `--dur-4`: il fumetto compare quando è finita. Comparsa e uscita del tour: dissolvenza
dell'attenuazione in `--dur-3`. Con **"Reduce motion"** niente spostamenti né scale: buco, anello e fumetto si
scambiano con una dissolvenza di `--dur-1`.

**Tastiera e fuoco.** `→` e `Invio` avanti, `←` indietro, `Esc` salta il tour; la barra spaziatrice resta al trasporto
(anche col fuoco sul fumetto). Le altre scorciatoie dell'app sono sospese; i comandi di progetto della barra dei menu
(New, Open, Close) chiudono prima il tour. A ogni passo il fuoco va su `Next` (su `Start` e `Done` nelle schede);
`Tab` gira tra il fumetto e i controlli dell'area illuminata, e solo lì. Il fumetto è un `dialog` con il titolo come
etichetta, non modale; il cambio di passo è annunciato da una regione `aria-live="polite"` (`Step 3 of 15: Tracks on a
timeline`). Contrasto come il resto dell'interfaccia (testo su `--bg-2` almeno 5,4:1).

**Uscita (UI-19).** `Done`, `Skip tour`, `Esc` e la chiusura dell'app fanno la stessa cosa: si ferma la riproduzione,
il progetto dimostrativo si chiude e si elimina, torna la Welcome (1b al primo avvio, con l'invito all'assistente), le
preferenze restano quelle di prima. Dopo `Skip tour` o `Esc` compare il toast di 18v (`Tour skipped`), senza azione,
via dopo `--toast-life`. Senza dispositivo audio il tour continua senza suono, con lo stato di 13d nella barra. Se il
brano non si può preparare la scheda di apertura lo dice (18w) e il tour finisce.

**Avvio a richiesta.** Il collegamento della Welcome sta sotto `New project` e `Open project…`, con la durata; il
tooltip dice cos'è (18s). Con un progetto aperto la voce è nel **menu dell'app**, che si apre dal nome del progetto
e raccoglie ora tutti i comandi di Stemwise (UI-20, A-096): `New project…` ⌘N, `Open…` ⌘O · `Project settings…`,
`Save` ⌘S, `Save as…` ⇧⌘S, `Close project` · `Settings…` ⌘, · `Help ›` `Take the Tour`. La barra dei menu di macOS
tiene solo il menu Stemwise di sistema, Edit e Window. `Take the Tour` è spento mentre l'assistente lavora, durante
un bounce o un export (18t). Con un progetto aperto si passa per
la conferma di chiusura, con una riga che dice perché (18u); `Cancel` lascia tutto com'era.

**Misure.** `--tour-w` 340, `--tour-card-w` 480, `--tour-pad` 4, `--tour-gap` 12, `--tour-edge` 16, `--tour-arrow` 12,
`--tour-dim`, `--z-tour` 45. Le posizioni dei riflettori nelle tavole sono misurate sulle tavole stesse (`R` in
`s-tour.mjs`); nell'app vengono dal bersaglio vero.

**Generatore.** `s-tour.mjs`; per riusare le parti delle altre schermate `timeline`, `top`, `wave`, `autoGeo`, la
striscia delle sezioni di Explain e la forma d'onda del Master accettano un brano diverso (`T`, `sections`), e
`proposal`, `explain`, `masterChain` sono esportate. Le altre tavole restano identiche, salvo 1a, 1b e 14 che
mostrano il collegamento al tour.
