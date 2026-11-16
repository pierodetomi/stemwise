# Stemwise — documento fondativo del prodotto

10 novembre 2026 · autore: Piero De Tomi · stato: **approvato come base di design e implementazione** ·
esteso l'11 novembre 2026 con il mix AI strutturale e il nuovo riverbero (P-011, fasi F11 e F12) ·
esteso il 13 novembre 2026 con lo stato del mix a colpo d'occhio (P-013, fase F13) e con le proposte
che restano (P-014) e con la configurazione guidata dell'assistente (P-015) ·
esteso il 14 novembre 2026 con il master AI misurato (P-016) e con la catena di master libera, in disegno (P-017) ·
esteso il 15 novembre 2026 con la spiegazione del mix (P-019, fase F18) e con il tour guidato (P-020, fase F19)

Questo documento è la fonte di verità del prodotto Stemwise: cosa fa, come è fatto, con quale stack, in quale
ordine si costruisce e con quali regole lavorano gli agenti. Sostituisce il PRD della PoC ([PRD.md](../poc/docs/PRD.md)) come
riferimento per il lavoro nuovo; il PRD e il codice Python restano come archivio e come oracolo numerico (§5.7).

Il lavoro avviene in **due sessioni**, ciascuna avviata con un prompt già pronto:

1. **Sessione di design** (Claude Desktop, canvas) — [§10](#10-prompt-della-sessione-di-design).
2. **Sessione di implementazione** (Claude Code, in questo repository) — [§11](#11-prompt-della-sessione-di-implementazione).

In caso di conflitto: questo documento vince su `CLAUDE.md`, che vince sul design, che vince sul PRD della PoC.
Un conflitto trovato durante l'implementazione si risolve secondo questo ordine e si annota in
`docs/app/DECISIONS.md`, senza fermarsi.

---

## 1. Visione

Stemwise è una **DAW multitraccia per mixare e masterizzare brani già registrati**, in cui un modello linguistico
fa da assistente al mix: propone parametri motivati, l'utente ascolta, corregge a mano o a parole, e tiene il
controllo. Non è una DAW per registrare o comporre.

**Utente.** Chi scrive e registra bene ma non è forte in mixing e mastering (l'autore per primo), e vuole un
risultato credibile senza imparare un mestiere né pagare abbonamenti. Secondo profilo: chi sa mixare e vuole un
punto di partenza rapido e spiegato.

**Principi di prodotto** (ereditati dalla PoC e confermati dal suo esito, [README](../poc/README.md#esito-della-poc)):

- **P1. Il modello non riceve mai audio.** Riceve misure numeriche, ruoli, nomi delle tracce, brief e richieste;
  restituisce parametri. Tutto l'audio resta sulla macchina. L'utente può sempre vedere cosa viene inviato.
- **P2. Il documento di progetto è l'unica sorgente di verità.** Stesso progetto ⇒ stesso render offline (stesso
  hash). L'AI non ha un canale privilegiato: produce le stesse modifiche che l'utente potrebbe fare a mano.
- **P3. Ogni scelta dell'AI è spiegata** (motivazione per traccia, sintesi del piano) e **reversibile** (una
  proposta applicata è un solo passo di undo).
- **P4. Niente è distruttivo.** I file importati non vengono mai modificati; ogni operazione è un comando
  annullabile.
- **P5. Mix e master sono due fasi distinte**, con spazi di lavoro, strumenti e cronologie separati.
- **P6. Nessun compromesso sul motore audio**: tempo reale nativo, senza allocazioni né lock nel thread audio.

### 1.1 Decisioni prese con l'autore (2026-11-10)

| Id | Decisione |
| --- | --- |
| P-001 | **Prodotto distribuibile**: solo dipendenze con licenze permissive; nessuna libreria GPL/LGPL. `pedalboard` e `matchering` della PoC non si usano: il DSP è riscritto. |
| P-002 | **macOS nella v1, codice portabile**: stack cross-platform; la v1 si verifica e si pacchettizza solo su macOS (Apple Silicon e Intel). Windows e Linux sono un lavoro successivo di build e collaudo, non una riscrittura. |
| P-003 | Nella v1 entrano: **curve di automazione libere**, **editing delle clip**, **delay e saturazione interni**. |
| P-004 | **Fuori dalla v1: plugin AU/VST3 di terze parti.** |
| P-005 | **Accesso al modello con chiave propria via OpenRouter**, salvata nel portachiavi di sistema. Nessun backend. |
| P-006 | Stack: **Tauri 2 + engine Rust + UI React/TypeScript** (§4). |
| P-007 | Due sessioni autonome (design, implementazione); delega ai subagenti solo dove riduce i costi (§9). |
| P-008 | Rientrano nella v1 (revisione dell'autore): **ritorni multipli, sidechain e routing strutturato** (gruppi annidati), **automazione di tutti i parametri continui della catena**, **export FLAC e AAC con metadata e ISRC**. Restano fuori il grafo di routing arbitrario, il DDP e l'MP3 (encoder LGPL, in conflitto con P-001). |
| P-009 | **Pannello "AI usage"** nell'app: modelli, chiamate, costi e storia leggibile di ogni chiamata (AI-14). |
| P-010 | **Tetto allo spazio su disco** occupato dal repository durante lo sviluppo, con pulizia periodica (§9, regole 18–20). |
| P-011 | (2026-11-11, dopo la consegna di F10) **Mix AI strutturale e riverbero nuovo.** L'AI può proporre modifiche alla struttura del mix che non alterano il brano: **ritorni dedicati** e **gruppi nuovi** (AI-17 → AI-20); riconosce i **doppi veri** e li apre ai lati (ANA-18, AI-21); usa slapback, "throw" di delay e **compressione parallela** (dry/wet del compressore, FX-3). Il riverbero a pettini è **sostituito** da tre algoritmi di qualità superiore (FX-5, FX-9), **senza retrocompatibilità del render**: non esistono ancora progetti veri. Restano fuori: il finto doppio da una sola take, l'ampiezza per traccia, il delay ping-pong, la creazione o la duplicazione di tracce audio da parte dell'AI. Si costruisce nelle fasi F11 e F12 (§8.1). |
| P-012 | (2026-11-13, dalla prova dell'autore su un brano di 30 stem) **Bilanciamento ed effetti del mix AI.** I mix proposti lasciavano la batteria davanti a tutto, chitarre e voce indietro, cori quasi assenti e riverberi inudibili: l'unico obiettivo misurato era lo spettro del riferimento. Il modello riceve ora i livelli per famiglia e per ritorno (AI-22), il ciclo fa almeno due iterazioni e tiene l'ultimo piano valido (AI-1), una revisione non ha limiti di passo e parte dallo stesso pulsante dell'auto mix (AI-3). |
| P-013 | (2026-11-13, dalla prova dell'autore su un brano di 30 stem) **Lo stato del mix a colpo d'occhio.** Per sapere quali tracce avevano effetti, mandate o sidechain bisognava selezionarle una alla volta: lo stato viveva solo nell'inspector, e le strip mostravano due mandate senza nome. Ogni intestazione e ogni strip porta ora i **segni** di ciò che la traccia ha acceso (UI-8), le mandate sono **tutte visibili e nominate** (UI-9), la selezione mostra **a cosa è collegata** (UI-10) e il mixer ha una vista **Overview** a griglia (UI-11). Solo interfaccia: nessun cambio al documento di progetto, al motore o ai contratti dell'AI. Tavole 3e–3g, 7f–7h, C7; si costruisce nella fase F13 (§8.1). |
| P-015 | (2026-11-13, in vista dell'apertura del repository) **Configurazione guidata dell'assistente.** Chi non sa cosa sia OpenRouter doveva procurarsi una chiave e incollarla. L'app ora **collega il conto dal browser** (flusso OAuth con PKCE del provider): un pulsante, l'approvazione sul sito, e la chiave arriva all'app da sola (AI-24). Il credito e la chiave incollata a mano sono passi guidati; la mancanza di credito ha il suo stato. La chiave sta nel portachiavi solo dove l'app può leggerlo **senza chiedere la password** (build firmata con un certificato Apple); altrimenti in un file privato dell'app (AI-11). Resta P-005: chiave dell'utente, nessun backend. Tavole 14a–14k. |
| P-014 | (2026-11-13, issue #2) **Proposte che restano.** Una proposta dell'assistente si poteva applicare solo al momento: scartata era persa, e applicata non si poteva riapplicare dopo aver cambiato i parametri a mano. Ogni proposta ricevuta resta nel progetto, in Mix e in Master: si rivede quando e cosa era, si riascolta contro lo stato attuale e si applica dopo una conferma esplicita (AI-23). |
| P-016 | (2026-11-14, issue #1, dalla prova dell'autore) **Master AI misurato.** Sulle destinazioni forti il master AI lasciava tutto il lavoro al limiter e, alla richiesta di più dinamica, abbassava la loudness: decideva alla cieca, in una sola chiamata, su misure che non dicevano quanto i picchi stanno sopra il corpo del mix. Ora riceve quella distanza, quanta ne toglie la destinazione e quanto lavora il limiter con la catena corrente; ogni piano è **renderizzato e misurato** e la decisione successiva lo corregge (MAS-6). La destinazione resta dell'autore: cambia solo se la richiesta lo chiede a parole. Clipper, ordine della catena e master per sezioni sono il passo successivo, da disegnare. |
| P-017 | (2026-11-14, issue #1, deciso dall'autore) **Catena di master libera.** Il master multibanda **entra nello scope**. La catena passa a dieci moduli: Input per primo, Limiter e Dither per ultimi, e in mezzo EQ, Match EQ, Compressor, Multiband, Width, Saturation e Clipper in **ordine libero**, a mano e dall'AI. Nuovi: clipper (quanti dB togliere ai picchi più alti), multibanda da due a sei bande, mix parallelo e filtro sul detector del compressore, **scostamenti per sezione** su pochi parametri, la proposta con le misure prima e dopo, il ritorno al mix quando la destinazione chiede troppo. La catena è una **lista di blocchi** mostrata come rack accanto all'editor: EQ, Compressor, Multiband, Saturation e Clipper possono starci due volte, dodici blocchi in tutto (A-080, A-081). Tavole 15a–15j (A-079, A-081), approvate dall'autore il 14 novembre 2026 e implementate nella fase F14 (MAS-11 → MAS-15, schema di progetto 3, `master_plan` v2). |
| P-018 | (2026-11-14, deciso dall'autore dopo un'analisi degli effetti possibili) **Effetti del mix e idee creative.** La catena di tracce e gruppi diventa una **lista di blocchi in ordine libero**, come quella del master (P-017): i cinque blocchi di sempre restano il punto di partenza, e se ne aggiungono tredici. **Correttivi**: gate ed expander, de-esser, EQ dinamico con sidechain, transient shaper. **Dinamica e colore**: clipper e multibanda anche su tracce e gruppi, caratteri della saturazione (tape, tube, transistor), exciter, bass enhancer. **Spazio**: ampiezza per traccia, una catena di blocchi anche sui ritorni (ducking, EQ del ritorno, riverbero gated), delay ping-pong e tape, riverbero `shimmer`. **Movimento**: modulazione (chorus, flanger, phaser), tremolo e auto-pan a tempo, filtri risonanti. **Pitch**: harmonizer e doubler come effetti sulla traccia. Gli scostamenti per sezione dell'AI arrivano a frequenze dei filtri e blocchi accesi solo in alcune sezioni. Il piano del mix aggiunge e regola blocchi (AI-25) e propone fino a tre **idee creative** in una sotto-sezione della proposta, ciascuna con la sua spunta, spenta per default (AI-26, AI-27). Restano fuori: il finto doppio da una sola take su un'altra traccia, riverbero e delay reverse, freeze, lo-fi e bitcrusher, EQ con più bande e notch stretti, la correzione dell'intonazione. Si disegna e si costruisce nelle fasi F15 → F17 (§8.1), con l'approvazione delle tavole prima dell'implementazione: tavole 16a–16u, 4c e 4d (A-086), approvate dall'autore il 14 novembre 2026. |
| P-019 | (2026-11-15, chiesto dall'autore) **Explain this mix.** L'interfaccia mostra lo stato del mix a chi sa già leggerlo; mancano il flusso (chi va dove) e l'intento (chi fa cosa e perché), e chi non è fonico si perde. Una vista del Mix, **Explain**, mette a sinistra la **mappa del segnale** (tracce per famiglia → gruppi → bus, mandate, sidechain: UI-15) e a destra **lo stesso mix in parole**, per tutto il mix, per elemento e per sezione (UI-16). È costruita **a regole** dal documento di progetto e da misure del render (ANA-20): **nessun modello**, nessuna chiamata, nessun costo; i motivi sono solo quelli già scritti dall'assistente. Sola lettura: nessun cambio al documento, al motore o ai contratti dell'AI. La narrazione scritta da un modello, locale o remoto, è rimandata. Tavole 17a–17j; fase F18 (§8.1). |
| P-020 | (2026-11-15, issue #5, deciso con l'autore) **Tour guidato.** Chi apre l'app per la prima volta non sa com'è fatta né da dove cominciare. Al **primo avvio in assoluto** parte da solo un tour che porta in giro per l'app su un **progetto dimostrativo incorporato** (PRJ-9): non è un progetto dell'utente, il documento è costruito dal codice e l'audio è **sintetizzato** da formule chiuse, così nessun file audio entra nel repository o nel pacchetto; si apre all'inizio del tour e sparisce alla fine. Il tour è un **giro completo** in quindici passi (trasporto, timeline, tracce, mixer, inspector, assistente, proposta in revisione, cronologia, Explain, Master, export), **guidato con Next**: riflettore sull'area spiegata, fumetto con Back, Next e Skip; è il tour ad aprire pannelli e viste, e l'area illuminata resta usabile (UI-17 → UI-19). Si salta in ogni momento e si ripete da **Help › Take the Tour** nel menu dell'app (UI-20) e dalla schermata Welcome. **Nessun modello**: la proposta mostrata viene dal decisore finto, senza rete e senza costi. Nessun cambio allo schema di progetto, al motore o ai contratti dell'AI. Restano fuori: passi che avanzano solo quando l'utente compie un'azione, capitoli separati, il progetto dimostrativo apribile fuori dal tour, stem reali nel pacchetto. Tavole della schermata 18, da approvare prima dell'implementazione; fase F19 (§8.1). |

## 2. Scope della v1

| Dentro la v1 | Fuori dalla v1 |
| --- | --- |
| Import di stem WAV/FLAC/AIFF su tracce separate | Registrazione audio, MIDI, strumenti virtuali |
| Timeline con clip: taglia, sposta, rifila, fade, gain, mute | Time-stretch, correzione dell'intonazione, warp, comping |
| Channel strip interno: trim, HPF, LPF, EQ 4 bande, compressore, saturazione, fader, pan, mandate; con P-018 catena a blocchi in ordine libero con gate, de-esser, EQ dinamico, transient shaper, clipper, multibanda, exciter, bass enhancer, ampiezza, modulazione, tremolo e auto-pan, harmonizer e doubler | Plugin AU/VST3/CLAP di terze parti |
| Ritorni multipli (riverberi e delay) con mandate, gruppi annidati, sidechain del compressore, bus di mix | Grafo di routing arbitrario (traccia verso traccia, uscite multiple, mandate pre-fader) |
| Sezioni del brano e automazione di tutti i parametri continui della catena, più mute e bypass | Registrazione dell'automazione in tempo reale da controller esterni |
| Mix AI (ciclo di verifica), revisione a parole, lock delle tracce, pannello di uso e costi dell'AI | Audio inviato al modello, riconoscimento automatico degli strumenti dall'audio, rilevamento automatico delle sezioni |
| Mix AI strutturale (P-011): ritorni dedicati e gruppi creati dall'AI, doppi veri riconosciuti, compressione parallela | Tracce audio create o duplicate dall'AI, finto doppio da una sola take (copia sfasata e pannata su un'altra traccia) |
| Riverbero in quattro algoritmi (plate, room, hall, shimmer), delay stereo, ping-pong e tape, catena di blocchi sui ritorni (P-018) | Riverbero a convoluzione e risposte all'impulso; riverbero e delay reverse, freeze |
| Idee creative dell'AI (P-018): blocchi, mandate, ritorni e scostamenti per sezione proposti a parte, ciascuno con la sua spunta | Lo-fi e bitcrusher; EQ delle tracce con più di 4 bande o notch stretti |
| Undo/redo illimitato, snapshot con nome, confronto A/B | Collaborazione, cloud, account |
| Mixdown e fase di master separata, master AI, riferimenti, master multibanda (P-017) | Separazione in stem dei riferimenti |
| Export WAV, FLAC e AAC con metadata (titolo, artista, album, anno, ISRC, copertina) | MP3 (encoder con licenza LGPL), DDP e assemblaggio di album |
| macOS 13+, pacchetto `.app`/`.dmg` | Build verificati per Windows/Linux, firma e notarizzazione (richiedono l'account Apple dell'autore), aggiornamento automatico |
| Interfaccia in inglese, stringhe centralizzate | Altre lingue dell'interfaccia |
| Tour guidato al primo avvio e a richiesta, su un progetto dimostrativo incorporato con audio sintetizzato (P-020) | Tour a compiti (passi che aspettano un'azione), capitoli separati, video, guida in linea; progetto dimostrativo apribile fuori dal tour; stem reali nel pacchetto |

Regola: ciò che sta a destra **non si implementa**, nemmeno in parte. Se serve, si annota in
`docs/app/DECISIONS.md` alla voce "Fuori scope segnalato" e si prosegue.

## 3. Requisiti funzionali

Ogni requisito ha un id stabile. Ogni id deve avere almeno un test automatico che lo cita (§8.3) oppure una
giustificazione scritta in `docs/app/REQS-JUSTIFIED.md`.

### 3.1 Progetto e file (PRJ)

- **PRJ-1.** Un progetto è una cartella-pacchetto `<Nome>.stemwise/` (§5.2) con `project.json` versionato.
- **PRJ-2.** Nuovo, apri, salva, salva con nome, progetti recenti. Autosave ogni 30 s e a ogni transazione AI;
  recupero dopo un crash.
- **PRJ-3.** L'import **copia** i file nel pacchetto (`audio/`); gli originali non vengono mai toccati né
  riferiti. I file in `audio/` sono immutabili: nessun comando li riscrive.
- **PRJ-4.** Import per trascinamento o da finestra di dialogo, più file insieme: ogni file diventa una traccia
  con una clip che parte da zero. Formati: WAV (PCM 16/24, float 32, anche EXTENSIBLE/RF64), FLAC, AIFF; mono o
  stereo.
- **PRJ-5.** La frequenza del progetto è quella del primo import. Un file a frequenza diversa non viene
  ricampionato in silenzio: l'app chiede se convertirlo (copia convertita nel pacchetto) o annullare.
- **PRJ-6.** A ogni traccia si assegna un **ruolo** (elenco della PoC, PRD §6.1, più `room`, `strings`, `brass` e, per la
  musica acustica e classica, `piano`, `woodwinds`, `choir`: A-080).
  Il ruolo è proposto dal nome del file con regole semplici e sempre modificabile.
- **PRJ-7.** Brief del brano: genere, mood, note libere. Tracce di riferimento (0–3; WAV/FLAC/AIFF/MP3/AAC),
  copiate nel pacchetto e usate solo per analisi e ascolto di confronto.
- **PRJ-8.** `project.json` è valido rispetto a uno schema JSON esportato in `app/schemas/`; un progetto di una
  versione di schema precedente viene migrato all'apertura.
- **PRJ-9.** **Progetto dimostrativo incorporato** (P-020). L'app contiene un progetto dimostrativo che serve solo al
  tour (UI-17) e **non è un progetto dell'utente**. Il core lo costruisce dal codice: un documento valido per lo
  schema corrente (nessuna versione nuova di schema) e l'audio **sintetizzato** da formule chiuse con seme fisso,
  mai da file inclusi nel repository o nel pacchetto.
  - **Contenuto**: un brano di 60–75 s a 48 kHz con 8–12 stem mono e stereo, coi nomi e i ruoli del brano delle
    tavole dove possibile (batteria, basso, chitarre o tastiere, voce, cori), abbastanza da mostrare ogni cosa che
    il tour nomina: almeno due gruppi, due ritorni (un riverbero e un delay) con mandate, una sidechain, tre
    sezioni con nome, una corsia di automazione, catene coi blocchi classici e qualche valore non neutro, brief
    compilato, nessun riferimento.
  - **Deterministico**: due costruzioni danno lo stesso documento e lo stesso hash di render.
  - **Effimero**: all'avvio del tour il progetto è materializzato in una cartella nella cache dell'app (mai tra i
    documenti dell'utente) ed è eliminato quando il tour finisce, in qualunque modo finisca; un residuo lasciato da
    un crash è eliminato all'avvio successivo. Non compare tra i recenti, non ha autosave né recupero, non si salva
    (Save e Save As non agiscono), non esporta file. Le modifiche fatte durante il tour si perdono: ogni tour parte
    dallo stesso stato.
  - **Senza modello**: la proposta dell'assistente e il mixdown che il tour mostra sono calcolati sul computer,
    la proposta **sempre col decisore finto**, anche se una chiave è configurata; nessuna chiamata di rete (NFR-7),
    niente nel registro dei costi tra progetti.
  - **Tempi** (Apple Silicon): il primo passo del tour è a schermo senza attese; il progetto è aperto e suonabile
    entro 3 s dall'avvio del tour; proposta e mixdown si preparano in background e un passo che ne ha bisogno
    prima che siano pronti lo dice e aspetta. L'avvio a freddo dell'app resta sotto i 2 s (NFR-3).
  - Che il progetto aperto è quello dimostrativo è un attributo della **sessione**, esposto all'interfaccia: non è
    scritto nel documento.

### 3.2 Trasporto e timeline (TRN, CLP, SEC)

- **TRN-1.** Play, stop, pausa, ritorno all'inizio, posizionamento col clic sul righello, loop di una regione,
  scorrimento che segue il cursore.
- **TRN-2.** Righello in minuti:secondi; griglia facoltativa in battute se l'utente imposta BPM e metro (solo
  visualizzazione e snap).
- **TRN-3.** Zoom orizzontale e verticale, scorrimento fluido, forme d'onda disegnate da piramidi di picchi
  precalcolate.
- **CLP-1.** Operazioni sulle clip: seleziona (anche multipla), sposta nel tempo e tra tracce dello stesso
  formato, dividi al cursore, rifila i bordi, duplica, elimina, mute.
- **CLP-2.** Fade in e fade out per clip (lineare o a potenza costante) e gain di clip (−24…+24 dB).
- **CLP-3.** Snap a griglia, bordi delle clip, cursore e confini di sezione; disattivabile.
- **CLP-4.** Le clip di una traccia non si sovrappongono: spostare una clip sopra un'altra rifila quella sotto
  (operazione annullabile come un solo passo).
- **SEC-1.** Sezioni del brano: intervalli con nome (intro, verse, chorus…) e colore su una corsia dedicata;
  crea, rinomina, sposta i confini, elimina. Le sezioni non si sovrappongono.
- **SEC-2.** Le sezioni sono la griglia su cui l'AI decide parametri diversi (AI-6) e su cui l'analisi misura nel
  tempo (ANA-15).

### 3.3 Tracce, mixer e catena (TRK, FX)

- **TRK-1.** Tipi di traccia: audio, gruppo, ritorno (riverbero o delay), bus di mix. I ritorni si creano e si
  eliminano liberamente (fino a 8); un progetto nuovo ne ha uno per tipo. Ordina, rinomina,
  colora, elimina, duplica.
- **TRK-2.** Mute, solo (solo-in-place; i ritorni seguono), fader (−∞…+12 dB), pan (potenza costante per mono,
  bilanciamento per stereo), meter di picco e RMS per traccia, true peak e LUFS sul bus di mix.
- **TRK-3.** Routing strutturato: l'uscita di una traccia audio, di un gruppo o di un ritorno va al bus di mix
  o a un gruppo. I gruppi si annidano (fino a 4 livelli) e hanno la stessa catena di una traccia. Il core
  rifiuta ogni assegnazione che creerebbe un ciclo.
- **TRK-5.** Mandate post-fader da tracce audio e gruppi verso qualunque ritorno, ciascuna col suo livello.
- **TRK-4.** Vista mixer (strip verticali) e intestazioni di traccia nella timeline mostrano lo stesso stato.
- **FX-1.** Catena per traccia e per gruppo: trim → blocchi (FX-10) → fader → pan → mandate (post-fader) ai
  ritorni. Una traccia o un gruppo nuovi hanno i cinque blocchi **classici** nell'ordine di partenza: passa-alto →
  passa-basso → EQ (4 bande: peak, low shelf, high shelf) → compressore → saturazione. Ogni blocco ha un bypass.
- **FX-2.** Filtri ed EQ sono biquad RBJ come nella PoC ([analysis.md](../poc/docs/analysis.md), "Render"); stessi limiti
  dei parametri (§5.5).
- **FX-3.** Compressore feed-forward con detector **collegato in stereo** (chiude D-034 della PoC): soglia,
  ratio, attacco, rilascio, knee, makeup; meter della riduzione di guadagno. **Sidechain**: il detector può
  ascoltare un'altra traccia o un gruppo (segnale post-fader), con passa-alto sul detector; il core rifiuta le
  sorgenti che creerebbero un ciclo. **Mix** (dry/wet, 0–100%, default 100%): miscela il segnale compresso con
  quello non compresso, allineati al campione; è la **compressione parallela** di tracce e gruppi (P-011) e si
  automatizza come gli altri parametri (AUT-1). Il compressore del master non lo ha.
- **FX-4.** Saturazione: waveshaper morbido con oversampling 4x, parametri `drive_db` e `mix`, compensazione
  automatica del livello.
- **FX-5.** Ritorno di tipo riverbero (algoritmico, implementazione propria) in **tre algoritmi** a scelta:
  `plate` (plate di Dattorro: denso e brillante, senza riflessioni iniziali, pensato per le voci), `room` e
  `hall` (rete di ritardi con retroazione di almeno 8 linee, con riflessioni iniziali; `hall` più lungo e più
  diffuso). Parametri: algoritmo, tempo di decadimento (0,1–10 s), dimensione, pre-delay (0–200 ms), damping
  delle alte, modulazione, passa-alto, passa-basso e livello del ritorno. Sostituisce il riverbero a pettini
  della prima consegna, che viene **rimosso** (P-011): un progetto precedente si apre con il riverbero convertito
  in `room` (PRJ-8), senza garanzia di render identico. Con P-018 si aggiunge un quarto algoritmo, `shimmer`
  (FX-22).
- **FX-6.** Ritorno di tipo delay stereo: tempo (ms, o valore musicale se c'è il BPM), feedback, passa-alto e
  passa-basso nel feedback, livello del ritorno; con P-018 anche il modo ping-pong e il carattere tape (FX-23).
- **FX-7.** Editor grafico dell'EQ (curva con maniglie trascinabili e spettro della traccia in tempo reale) e
  pannello del compressore con curva di trasferimento.
- **FX-8.** Bus di mix: fader e meter. Nessun processore sul bus nella fase di mix: la dinamica finale appartiene
  al master.
- **FX-9.** Qualità del riverbero, verificata da test su proprietà per ciascuno dei tre algoritmi (soglie
  motivate nel test e riportate in `docs/app/dsp.md`):
  - il tempo di decadimento misurato su un impulso (RT60 tra 500 Hz e 2 kHz) sta entro ±10% del valore impostato;
  - la coda è densa: l'autocorrelazione normalizzata della coda non ha picchi isolati (nessuna eco ripetuta né
    risonanza metallica dominante);
  - con ingresso mono, sinistra e destra della coda sono decorrelate (|correlazione| < 0,3);
  - col damping le alte decadono prima delle medie; il pre-delay è rispettato; la coda finisce in silenzio esatto;
  - la modulazione è deterministica: oscillatori con fase iniziale fissa, azzerati all'inizio di ogni render,
    nessun generatore casuale in esecuzione; il risultato non dipende dalla dimensione del blocco (ENG-4);
  - nessuna allocazione in `process`, stabilità con parametri che cambiano ogni 32 campioni (AUT-6);
  - NFR-1 resta rispettato con 4 ritorni riverbero aggiunti al benchmark.
- **FX-10.** **Catena a blocchi** (P-018). Tra trim e fader la catena di tracce e gruppi è una **lista** di blocchi in
  ordine libero, come quella del master (MAS-15). Tipi: i classici `hpf`, `lpf`, `eq`, `compressor`, `saturation` e i
  nuovi `gate`, `deesser`, `dynamic_eq`, `transient`, `clipper`, `multiband`, `exciter`, `bass_enhancer`, `width`,
  `modulation`, `tremolo`, `harmonizer`, `doubler` (FX-12 → FX-21). Ogni tipo al più **due volte**, al più **12
  blocchi** per catena. Aggiungere, togliere, spostare, duplicare e riportare un blocco ai valori iniziali sono un
  passo di undo ciascuno; togliere un blocco classico è permesso e lo si può rimettere. Requisiti comuni:
  - un cambio di struttura in riproduzione non produce scatti; mettere o togliere un bypass, a mano o
    dall'automazione, passa con una dissolvenza di circa 10 ms;
  - la **latenza** di una traccia non dipende dalla sua catena: è la stessa per ogni traccia, gruppo e ritorno, e
    aggiungere un blocco non sposta mai una traccia rispetto alle altre (ENG-5);
  - una traccia mono resta mono fino al primo blocco **stereo** (`width`, `modulation`, `tremolo` in auto-pan,
    `harmonizer`, `doubler`); da lì la catena è stereo e il pan fa da bilanciamento (TRK-2). Un blocco stereo che
    riceve un segnale mono lo dice nel suo pannello quando non ha effetto (`width`);
  - un progetto dello schema 3 si apre con i cinque blocchi classici nell'ordine di sempre e **lo stesso render**
    (stesso hash);
  - i segni della catena (UI-8) e la vista Overview (UI-11) seguono la lista (UI-12).
- **FX-11.** **Catena dei ritorni** (P-018). Ogni ritorno ha, dopo il suo processore, una lista di blocchi degli
  stessi tipi di FX-10, vuota per default, al più 6. Serve al **ducking** (compressore con sidechain dalla traccia
  che suona sopra l'effetto), all'EQ del ritorno e al **riverbero gated** (gate con sidechain dal rullante). Le
  sidechain seguono FX-3: il core rifiuta le sorgenti che creerebbero un ciclo. Un ritorno continua a non mandare ad
  altri ritorni (TRK-5).
- **FX-12.** **Filtri risonanti**: passa-alto e passa-basso hanno una `resonance` (Q da 0,5 a 10; 0,707, il
  comportamento di oggi, è il default), automatizzabile come la frequenza. Con gli scostamenti per sezione (AI-6)
  e l'automazione fanno i filtri che si aprono e si chiudono tra le sezioni.
- **FX-13.** **Gate / expander**: soglia, range (0–80 dB: quanto abbassa sotto la soglia), ratio (1–20; a 20 è un
  gate), attacco, hold, rilascio, passa-alto e passa-basso sul detector, sidechain facoltativa da un'altra traccia o
  da un gruppo (FX-3). Apertura e chiusura senza click.
- **FX-14.** **De-esser**: frequenza (3–12 kHz), soglia, riduzione massima (0–12 dB), modo `split` (abbassa solo ciò
  che sta sopra la frequenza) o `wide` (abbassa tutto il segnale); ascolto del solo segnale rilevato, che non entra
  nel progetto né nell'export.
- **FX-15.** **EQ dinamico**: una o due bande (peak, low shelf, high shelf) con frequenza, Q, soglia, guadagno
  massimo (−12…+6 dB: negativo taglia, positivo esalta quando il segnale della banda supera la soglia), attacco e
  rilascio. Il detector ascolta la stessa banda oppure, con la sidechain, la stessa banda di un'altra traccia o di un
  gruppo (FX-3): così le chitarre scendono a 3 kHz **solo quando canta la voce**.
- **FX-16.** **Transient shaper**: attacco (±12 dB), sustain (±12 dB), velocità (0–1: quanto è corta la parte di
  attacco), mix. Non ha soglia: il risultato non dipende dal livello d'ingresso.
- **FX-17.** **Clipper e multibanda su tracce e gruppi**: lo stesso processore di MAS-12 per il multibanda (2–6
  bande); il clipper è quello di MAS-11 con una **soglia assoluta** (−24…0 dBFS) al posto della soglia misurata sul
  brano, perché sulla traccia il suo ingresso cambia a ogni parametro precedente.
- **FX-18.** **Colore**: la saturazione ha un `character`: `soft` (quella di FX-4, default e valore dei progetti
  esistenti), `tape` (picchi arrotondati e un leggero calo delle alte), `tube` (asimmetrica, armoniche pari),
  `transistor` (più dura, armoniche dispari). **Exciter**: frequenza (1–12 kHz), drive, quantità: satura solo ciò che
  sta sopra la frequenza e lo rimescola. **Bass enhancer**: frequenza (40–160 Hz), armoniche (0–1), mix: genera
  armoniche della banda bassa perché il basso si legga anche su casse piccole; non genera ottave sotto. Tutti con
  oversampling 4x e compensazione automatica del livello, come FX-4.
- **FX-19.** **Ampiezza** per traccia, gruppo e ritorno: ampiezza (0–200%, guadagno del side) e mono sotto una
  frequenza (0–250 Hz), come il blocco Width del master (MAS-1).
- **FX-20.** **Movimento**. **Modulazione**: modo `chorus`, `flanger` o `phaser`; velocità (0,02–10 Hz, o valore
  musicale se c'è il BPM), profondità, feedback (flanger e phaser, −0,9…0,9), voci (chorus, 1–3), stadi (phaser: 4,
  6, 8, 12), apertura stereo (0–180°), mix. **Tremolo**: modo `tremolo` o `autopan`, velocità (Hz o valore musicale),
  profondità, forma (sinusoide, triangolo, quadra smussata), sfasamento stereo per il tremolo. Per tutti e due la
  **fase dipende solo dalla posizione nel brano**: lo stesso campione del brano ha la stessa fase in riproduzione da
  qualunque punto e nel render offline; col BPM la fase è agganciata alla battuta 1.
- **FX-21.** **Pitch**. **Harmonizer**: una o due voci, ciascuna con intervallo (−12…+12 semitoni, più ±50 cent),
  livello e pan, più il mix. **Doubler**: una o due voci con un piccolo scostamento d'intonazione (0–30 cent), un
  ritardo (5–40 ms) che si muove lentamente, apertura stereo e mix. Pitch shifter scritto da noi, deterministico, con
  latenza costante compensata (FX-10). Il doubler è un effetto **dentro la traccia**: non crea tracce né copie della
  clip (§2). Nessuna correzione dell'intonazione.
- **FX-22.** **Riverbero `shimmer`**: un quarto algoritmo di FX-5, con una trasposizione d'intonazione nel ricircolo:
  intervallo (+5, +7 o +12 semitoni, default +12) e quantità (0–1). Con quantità 0 vale tutto FX-9; con quantità
  sopra 0 la coda resta limitata, finisce in silenzio esatto e non dipende dalla dimensione del blocco.
- **FX-23.** **Delay ping-pong e tape**: modo `stereo` (quello di FX-6, default) o `ping_pong` (ingresso sommato in
  mono, ripetizioni alternate tra sinistra e destra); carattere `clean` (default) o `tape`, con wow e flutter
  (0–1, fase legata alla posizione nel brano come FX-20) e drive nel feedback (0–12 dB) con il livello compensato.
- **FX-24.** **Qualità dei blocchi nuovi**, verificata da test su proprietà per ciascun blocco (soglie motivate nel
  test e riportate in `docs/app/dsp.md`):
  - in posizione neutra (mix a 0, quantità o profondità a 0, guadagni a 0 dB) il blocco lascia il segnale com'è
    (differenza ≤ 1e-6, a parte la latenza compensata);
  - nessuna allocazione in `process`, risultato indipendente dalla dimensione del blocco (ENG-4), stabilità con
    parametri che cambiano ogni 32 campioni (AUT-6), uscita limitata con rumore a piena scala;
  - gate: sotto la soglia il segnale scende del range impostato (±0,5 dB); de-esser in `split`: sotto la frequenza
    il segnale non cambia; EQ dinamico: la banda si muove solo quando il detector supera la soglia; transient
    shaper: una sinusoide stabile non cambia livello; harmonizer: intervallo misurato su una sinusoide entro ±5 cent;
    modulazione e tremolo: il render che parte a 30 s dà gli stessi campioni del render intero da 30 s in poi;
    ping-pong: le ripetizioni si alternano tra i canali;
  - NFR-1 è rispettato con la catena classica; con 8 delle 32 tracce che portano anche un harmonizer, un multibanda a
    quattro bande e un chorus il callback resta sotto il 50% del periodo.

### 3.4 Automazione (AUT)

- **AUT-1.** Corsie di automazione per **ogni parametro continuo** di tracce audio, gruppi e ritorni: fader,
  pan, mandate, trim, frequenze dei filtri, frequenza/gain/Q di ogni banda di EQ, soglia/ratio/attacco/
  rilascio/makeup del compressore, drive e mix della saturazione, parametri dei ritorni e, con P-018, ogni parametro
  continuo dei blocchi di FX-10 → FX-23 (anche nelle catene dei ritorni); più mute e bypass dei blocchi. Punti con interpolazione lineare; mute e bypass sono a gradini.
- **AUT-6.** I parametri automatizzati di filtri, EQ e compressore si aggiornano a intervalli di al più 32
  campioni con coefficienti interpolati: nessun click né instabilità, anche con curve ripide (verificato da un
  test su sweep rapidi).
- **AUT-2.** Modifica: aggiungi, sposta, elimina punti; selezione a intervallo; disegno a mano libera
  semplificato in punti; copia/incolla di un intervallo.
- **AUT-3.** Con una corsia attiva, il controllo corrispondente mostra il valore automatizzato; muoverlo a
  trasporto fermo sposta l'intera curva (offset), non scrive punti.
- **AUT-4.** Il render applica le curve con rampa a campione (nessun gradino udibile) ed è identico tra
  riproduzione e render offline.
- **AUT-5.** Le decisioni per sezione dell'AI (AI-6) vengono scritte **come punti di automazione** nelle stesse
  corsie, con una rampa ai confini di sezione (default 80 ms): restano modificabili a mano.

### 3.5 Cronologia e non distruttività (HIS)

- **HIS-1.** Ogni modifica del progetto è un **comando** con il suo inverso. Undo e redo illimitati nella
  sessione, da menu, scorciatoia e pannello cronologia (elenco leggibile dei passi, clic per tornare a un punto).
- **HIS-2.** I gesti continui (trascinare un fader, una clip, un punto) producono un solo passo.
- **HIS-3.** L'applicazione di una proposta AI è **una sola transazione**, con etichetta e riferimento al log
  della chiamata.
- **HIS-4.** Snapshot del mix con nome, persistenti nel progetto: salva, ripristina (annullabile), confronta A/B
  due stati durante l'ascolto senza interrompere la riproduzione.
- **HIS-5.** Mix e master hanno cronologie separate.
- **HIS-6.** Mute/solo, zoom, selezione e posizione del cursore non entrano nella cronologia (stato di vista).

### 3.6 Motore audio (ENG)

- **ENG-1.** Riproduzione in tempo reale dell'intero grafo, con lettura da disco in streaming (thread di lettura
  e buffer circolari per traccia). Il thread audio non alloca, non blocca, non fa I/O.
- **ENG-2.** Scelta del dispositivo di uscita e della dimensione del buffer; se il dispositivo non supporta la
  frequenza del progetto, ricampionamento in uscita.
- **ENG-3.** Ogni modifica di parametro arriva al motore entro un buffer, con smoothing (nessun click).
- **ENG-4.** **Render offline** con lo stesso codice del grafo in tempo reale, più veloce del tempo reale,
  deterministico: stesso progetto ⇒ stesso hash del file. Il risultato non dipende dalla dimensione del blocco
  (differenza ≤ 1e-6 di picco tra blocchi da 64 e da 4096 campioni).
- **ENG-5.** Compensazione dei ritardi introdotti dall'oversampling della saturazione e dal lookahead del limiter.
- **ENG-6.** Meter e posizione di riproduzione inviati all'interfaccia ad almeno 30 Hz, senza passare dal thread
  audio in modo bloccante.
- **ENG-7.** Un CLI senza interfaccia (`stemwise-cli`) espone analisi, render, mix AI con decisore finto,
  mixdown e master su un progetto: serve ai test di flusso completo e alla verifica autonoma.

### 3.7 Analisi (ANA)

- **ANA-1 → ANA-12.** Le misure della PoC (PRD §5.2), con le formule di [analysis.md](../poc/docs/analysis.md), riscritte in
  Rust e verificate contro l'oracolo (§5.7).
- **ANA-13.** Il JSON di analisi inviato al modello resta sotto 12.000 token stimati per 24 tracce e 8 sezioni.
- **ANA-14.** Le misure si calcolano sul contenuto della traccia **dopo l'editing delle clip e prima della
  catena**. La cache si invalida quando cambiano le clip, non quando cambiano i parametri.
- **ANA-15.** Misure nel tempo, compatte: loudness di ogni traccia per sezione e percentuale di tempo attivo per
  sezione; profilo in bande del mix per sezione.
- **ANA-16.** Famiglie: gli stem con ruoli di batteria (o nello stesso gruppo) si riportano anche come famiglia
  (loudness e bande della somma); il mascheramento **dentro** una famiglia non entra nelle collisioni principali.
- **ANA-17.** Analisi in parallelo sulle tracce, con avanzamento visibile; 16 stem di 4 minuti in meno di 30 s
  su Apple Silicon.
- **ANA-18.** **Doppi veri** (P-011): per ogni coppia di tracce audio con lo stesso ruolo l'analisi misura quanto
  sono due esecuzioni della stessa parte: correlazione degli inviluppi di livello, somiglianza del profilo in
  bande, sovrapposizione del tempo attivo, e differenza della forma spettrale frame per frame per distinguere due
  esecuzioni (alcuni dB) da due copie dello stesso segnale (una frazione di dB, anche a livelli diversi o con
  pochi millisecondi di ritardo: marcate `identical`, aprirle ai lati non serve). Ruoli considerati: chitarre,
  tastiere, synth, pad, archi, fiati e voci. Le coppie
  sopra soglia entrano nel documento di analisi come `doubles` (id delle due tracce, punteggio, `identical`).
  Soglie iniziali motivate nel test e da tarare nella validazione dell'autore; il tetto ANA-13 resta rispettato.
- **ANA-19.** **Misure per i blocchi nuovi** (P-018), calcolate come ANA-14 e sotto il tetto di ANA-13: per le tracce
  con ruoli vocali la **sibilanza** (di quanto i picchi brevi tra 5 e 10 kHz superano il corpo della voce, e la
  frequenza dove cadono); per ogni traccia la **cresta** (picco su RMS breve) e i **transienti** (di quanto gli
  attacchi rilevati dagli onset stanno sopra il livello che li segue); per le tracce con ruoli di batteria il
  **fondo** (livello tra un colpo e l'altro rispetto ai colpi, che dice quanto rientro c'è); per le tracce stereo la
  correlazione e la quota del side; il tempo e la metrica del progetto, se noti. Per ogni traccia, gruppo e ritorno
  l'analisi elenca anche i blocchi che non sono classici (id, tipo, stato, posizione).

- **ANA-20.** **Livelli per la spiegazione** (P-019). Per la vista Explain il core misura, dal render del mix com'è
  (stessi render di AI-22): la loudness di ogni famiglia e di ogni ritorno rispetto al mix, sull'intero brano e per
  sezione, e la loudness del mix intero e di ogni sezione. La misura gira in background, non blocca l'interfaccia, si
  annulla se il mix cambia e resta in memoria finché il mix non cambia. Nessun dato esce dal computer.

### 3.8 Mix assistito dall'AI (AI)

- **AI-1.** **Auto mix**: da brief, analisi e profilo dei riferimenti, il modello restituisce un piano completo
  (§5.5). Ciclo di verifica come nella PoC (render offline, valutazione, fino a 3 iterazioni, criteri di stop e
  piano migliore: PRD §5.5), con due differenze (P-012): un auto mix fa **almeno due iterazioni**, perché solo
  la seconda conosce il bilanciamento di un piano renderizzato (AI-22); il piano tenuto è l'**ultimo valido**,
  a meno che si allontani dal riferimento di oltre 1 dB rispetto a uno precedente.
- **AI-2.** Il piano non si applica da solo: diventa una **proposta** in revisione, con sintesi, motivazione per
  traccia, diff dei parametri rispetto allo stato attuale, valutazione (distanza dal riferimento prima/dopo,
  vincoli tecnici) e costo. L'utente ascolta la proposta in A/B con lo stato attuale, poi **applica tutto**,
  **applica solo le tracce scelte** o **scarta**.
- **AI-3.** **Revisione a parole**: una richiesta in linguaggio naturale ("voce più avanti, meno riverbero sulle
  chitarre") produce una proposta di una sola iterazione che parte dallo stato attuale, comprese le modifiche
  manuali. I limiti di variazione tra iterazioni (AI-8) **non valgono** per una revisione: è un passo solo,
  chiesto dall'autore e giudicato a orecchio (P-012). Nell'interfaccia richiesta e auto mix partono dallo
  **stesso pulsante**: col testo è una revisione, senza è un auto mix; Invio nel campo va a capo.
- **AI-4.** **Lock**: una traccia (o un gruppo) bloccata non viene modificata dall'AI; il modello la riceve come
  contesto e la validazione ne ripristina i valori.
- **AI-5.** Ambito: l'utente può limitare una richiesta alle tracce selezionate; le altre si comportano come
  bloccate per quella richiesta.
- **AI-6.** **Parametri per sezione**: se il progetto ha sezioni, il piano può contenere per ogni traccia
  scostamenti per sezione di gain, mandate e gain delle bande di EQ (D-048 della PoC), applicati come automazione (AUT-5).
  Con P-018 anche la frequenza di passa-alto e passa-basso (in ottave) e i **blocchi accesi solo in alcune sezioni**
  (bypass nelle altre, con la dissolvenza di FX-10).
- **AI-7.** Normalizzazione d'ingresso: l'auto mix imposta il **trim** di ogni traccia verso −23 LUFS (REN-1
  della PoC), come parte visibile della proposta; disattivabile.
- **AI-8.** Validazione stretta del piano rispetto a schema e limiti; fino a 2 nuovi tentativi con l'errore
  rimandato al modello; limiti di variazione tra le iterazioni di un auto mix (PRD DEC-5). Se il provider rifiuta lo schema
  `strict`, un ripiego automatico alla modalità JSON più validazione (Q-40 della PoC, opzione b).
- **AI-9.** Interfaccia `Decider` con due implementazioni: OpenRouter e **decisore finto** deterministico (regole
  della PoC estese a sezioni, delay, saturazione e, con P-011, a struttura, algoritmi di riverbero, compressione
  parallela e doppi). Il decisore finto è selezionabile nelle impostazioni ("Offline demo") ed è l'unico usato
  nei test.
- **AI-10.** Trasparenza: per ogni chiamata si salvano nel progetto richiesta, risposta, token, costo e durata;
  il dettaglio è consultabile nel pannello "AI usage" (AI-14). Al modello non vanno mai audio, percorsi, nomi
  di file o titolo del brano.
- **AI-11.** Chiave OpenRouter **nel portachiavi di sistema quando l'app può leggerlo senza che il sistema chieda
  la password** (su macOS: build firmata con un certificato emesso da Apple, la cui identità non cambia da una build
  all'altra); **altrimenti in un file della cartella dati dell'app leggibile solo dall'utente** (permessi 600), mai
  nel progetto, nei log o nel file delle impostazioni. L'interfaccia dice sempre dove sta la chiave. Modello, sforzo di ragionamento e temperatura
  configurabili; nessun modello cablato nel codice (default di configurazione: `openai/gpt-6.1-sol`, sforzo
  medio, D-047). Stato chiaro quando manca la chiave o la rete.
- **AI-12.** Le chiamate non bloccano l'interfaccia, mostrano l'avanzamento per fase (analisi, decisione, render,
  valutazione) e si possono annullare.
- **AI-13.** `rationale` e `summary` nella lingua del brief.
- **AI-14.** **Pannello "AI usage"**, raggiungibile da entrambe le fasi:
  - riepilogo del progetto: numero di chiamate, token in ingresso e in uscita, costo totale, durata totale,
    ripartiti per tipo (auto mix, revisione, master) e **per modello usato**;
  - storia cronologica delle chiamate: data, tipo, modello e sforzo, tentativi, token, costo, durata, esito
    (applicata, applicata in parte, scartata, fallita, annullata) e collegamento al passo di cronologia;
  - dettaglio di una chiamata in forma leggibile: richiesta resa come brief, richiesta dell'utente, tracce con
    ruoli e lock, tabelle delle misure; risposta resa come sintesi, modifiche per traccia e motivazioni; il JSON
    grezzo di richiesta e risposta resta consultabile e copiabile;
  - export della storia in CSV e JSON;
  - totale dei costi **su tutti i progetti** nelle impostazioni dell'AI (registro nell'area dati dell'app).
- **AI-15.** Il costo è quello riportato dal provider nella risposta; se manca, viene stimato dai token e dal
  listino del modello e marcato come stima. Le chiamate fallite o annullate contano con ciò che hanno speso.
- **AI-16.** Prima di una chiamata l'app mostra una stima di token e costo; un tetto di spesa per progetto,
  facoltativo, chiede conferma quando verrebbe superato.
- **AI-17.** **Modifiche strutturali** (P-011): il piano può, nella sezione `structure` (§5.5):
  - **creare ritorni** (riverbero, con il suo algoritmo, o delay), fino a 3 per proposta ed entro il tetto di 8
    (TRK-1), per dare a una famiglia di tracce il suo spazio (per esempio un plate per la voce, un delay corto per
    lo slapback);
  - **creare gruppi**, fino a 3 per proposta ed entro i 4 livelli (TRK-3), e spostarvi tracce audio o gruppi
    esistenti: almeno due membri, tutti con la stessa uscita; il gruppo prende quell'uscita e la posizione del
    primo membro. Mandate e parametri dei membri non cambiano per effetto dello spostamento.

  Il piano **non** crea, duplica o elimina tracce audio, non elimina ritorni o gruppi, non sposta tracce tra
  gruppi esistenti e non tocca clip e sezioni: il brano resta quello. Ogni modifica strutturale è una modifica
  che l'utente può fare a mano con gli stessi comandi (P2).
- **AI-18.** Le modifiche strutturali si possono **disattivare** dal pannello AI ("Allow structural changes",
  attivo per default, ricordato nel progetto): disattivate, lo schema inviato non contiene `structure`. Una
  richiesta limitata ad alcune tracce (AI-5) non porta struttura: un ritorno è condiviso e un gruppo cambia il
  percorso dei suoi membri. Una traccia bloccata (AI-4) non viene spostata in un gruppo nuovo.
- **AI-19.** **Struttura stabile nel ciclo**: la struttura si decide alla prima iterazione; le iterazioni
  successive la ricevono come data e regolano solo i parametri (il loro schema non contiene `structure`). Una
  revisione a parole (AI-3) può proporre struttura nella sua unica iterazione. La validazione (AI-8) scarta con
  un avviso, senza far fallire il piano, un ritorno nuovo verso cui nessuna traccia manda e un gruppo nuovo con
  meno di due membri validi.
- **AI-20.** **Proposta e applicazione** (estende AI-2, HIS-3): ritorni e gruppi nuovi compaiono nella proposta
  come righe proprie, con nome, motivazione e parametri; l'ascolto in A/B li comprende. "Applica tutto" li crea
  nella stessa transazione delle altre modifiche: un solo passo di undo li toglie. Ogni elemento nuovo ha la
  **sua spunta**: tolta, non viene creato, e le mandate verso di esso o lo spostamento nel gruppo cadono, mentre
  le altre modifiche delle tracce restano. Con "applica solo le tracce scelte" un ritorno nuovo spuntato si crea
  se almeno una traccia scelta manda verso di esso, e un gruppo nuovo spuntato si crea con i soli membri scelti,
  se sono almeno due; altrimenti la proposta dice perché non viene creato. "Scarta" non crea nulla.
- **AI-21.** **Tecniche di mix nel prompt e nel decisore finto** (P-011):
  - **doppi veri**: una coppia riportata da ANA-18 e non `identical` si tratta come coppia: pan opposti e
    simmetrici, livelli allineati, stessa elaborazione;
  - **slapback**: un delay dedicato con tempo breve (80–140 ms) e feedback nullo, a cui manda la voce;
  - **"throw" di delay**: mandata al delay alzata solo in alcune sezioni (AI-6);
  - **compressione parallela**: compressore deciso con `mix` sotto 1 su batteria, voce o sui loro gruppi;
  - **spazi distinti**: algoritmo e decadimento del riverbero scelti per famiglia (plate corto per la voce,
    room o hall per il resto).
  - con P-018, tra le correzioni: **de-esser** sulle voci con sibilanza alta; **EQ dinamico** con sidechain al
    posto di un taglio fisso quando due tracce si mascherano solo mentre suonano insieme; **gate** sulle batterie
    con fondo alto; **transient shaper** per dare o togliere colpo; **ducking** dei ritorni dalla voce; **pompa da
    sidechain** (cassa verso basso o pad) dove il genere la chiede e mai come default; ampiezza sui pad e sulle
    tastiere stereo, mai su basso e cassa; carattere della saturazione scelto per ruolo e brief;
  - con P-018, tra le idee creative (AI-26): modulazione, tremolo e auto-pan, harmonizer, doubler, shimmer,
    ping-pong, delay tape, riverbero gated, filtri che si aprono tra le sezioni, throw.

  Il prompt descrive ogni tecnica con il caso in cui serve e quello in cui va evitata; il decisore finto ne
  applica ciascuna con una regola deterministica, verificata dal flusso completo di `stemwise-cli`.
- **AI-22.** **Bilanciamento misurato** (P-012): il modello riceve quanto suona ogni **famiglia** di tracce (voce
  principale, cori, batteria, percussioni, basso, chitarre, tastiere, altro) e ogni **ritorno** rispetto al mix
  intero, in LU, sul brano e per sezione. Lo riceve prima di decidere (auto mix: con tutti i fader a 0 dB sui
  trim normalizzati; revisione: sul mix attuale) e nella valutazione di ogni iterazione, misurato sul render del
  piano. Ogni livello è un render in solo del progetto: la famiglia senza i ritorni, il ritorno da solo. Il
  prompt dà i rapporti di partenza tra le famiglie e la soglia sotto cui un ritorno non si sente, dice che il
  profilo dei riferimenti descrive il tono e non il bilanciamento, e che brief e richieste vengono prima.
- **AI-23.** **Proposte che restano** (P-014). Ogni chiamata che torna con una proposta, di mix o di master, la
  conserva nella sua cartella del progetto, qualunque cosa ne sia stato (applicata, applicata in parte,
  scartata). L'assistente le elenca in Mix e in Master, dalla più recente, con quando è arrivata, la richiesta
  e l'esito. Una proposta dell'elenco torna **in revisione sullo stato attuale**: le differenze sono ricalcolate
  sul mix (o sul master) di adesso, le tracce che non ci sono più o sono bloccate restano fuori e lo si dice,
  ritorni e gruppi già creati non si creano due volte, l'A/B funziona come per una proposta nuova. Applicarla
  chiede una **conferma esplicita** che dice che il mix attuale cambierà, ed è un solo passo di cronologia
  (AI-2, HIS-3); chiuderla non cambia nulla e non ne altera l'esito registrato. Riaprire una proposta non
  chiama il modello e non costa.
- **AI-24.** **Configurazione guidata dell'assistente** (P-015). Un dialogo in tre passi, lo stesso dal primo avvio,
  dalle Impostazioni e dalla colonna dell'assistente senza chiave. **Collegamento**: l'app apre la pagina di
  autorizzazione di OpenRouter nel browser (OAuth con PKCE S256 e `state`), aspetta il ritorno su `localhost`,
  scambia il codice con una chiave dell'utente e la salva (AI-11); la chiave non passa mai dal browser né
  dall'interfaccia. Rifiuto, scadenza (dieci minuti), rete assente e annullamento hanno ciascuno il suo esito, e
  uscire dal dialogo ferma l'attesa. **Credito**: quanto costano un mix e una revisione ai prezzi del modello
  scelto, il collegamento alla pagina dei crediti, e detto in chiaro che l'app non può leggere il saldo.
  **Chiave incollata a mano** come via secondaria, in tre passi numerati. Quando il provider rifiuta una chiamata
  l'assistente distingue **credito esaurito**, **limite della chiave** e **chiave non più valida**, ciascuno con la
  sua via d'uscita; una chiamata rifiutata non costa nulla e non cambia il mix. La demo offline resta sempre a un
  clic.
- **AI-25.** **Blocchi nel piano** (P-018): il piano può aggiungere blocchi alle catene di tracce, gruppi e ritorni
  (FX-10, FX-11) e regolare quelli che ci sono, entro i limiti di §5.5; al più 3 blocchi nuovi per catena in una
  proposta. Il modello non sposta i blocchi esistenti e non li toglie: li spegne con il bypass. Un blocco nuovo va in
  una posizione nominata (`first`, `before_compressor`, `after_compressor`, `last`); a mano si sposta come ogni
  altro. Lock (AI-4) e ambito (AI-5) valgono anche per le catene: un ritorno è condiviso, quindi una richiesta
  limitata ad alcune tracce non ne cambia la catena. Nella proposta un blocco nuovo è una riga marcata come nuova
  nel diff della sua traccia.
- **AI-26.** **Idee creative** (P-018): il piano può proporre fino a **3 idee**, ciascuna con id (`i1`–`i3`), nome (in
  inglese, al più 32 caratteri), motivazione (nella lingua del brief) e, se serve, le sezioni in cui suona. Gli
  elementi di un'idea sono marcati con il suo id e possono essere solo **aggiunte**: blocchi nuovi, mandate nuove,
  ritorni nuovi (AI-17, se le modifiche strutturali sono permesse) e scostamenti per sezione. Un'idea non cambia i
  parametri di ciò che esiste: la validazione (AI-8) scarta con un avviso un elemento che lo fa, senza far fallire il
  piano. Togliere un'idea lascia il resto del piano identico. Le idee nascono dal brief, dai ruoli e dalle sezioni,
  non dalle misure: il prompt le chiede facoltative, diverse tra loro e non correttive. Si decidono alla prima
  iterazione e restano ferme nel ciclo (come AI-19); il ciclo renderizza e valuta il piano **senza** idee. Una
  revisione a parole può proporne nella sua unica iterazione ("qualcosa di più audace nel bridge"). Il decisore
  finto ne propone con regole deterministiche sui ruoli e sulle sezioni.
- **AI-27.** **Idee nella proposta** (P-018): la proposta ha una sotto-sezione **Creative ideas**, una scheda per idea
  con nome, motivazione, sezioni ed elementi. Ogni idea ha la **sua spunta, spenta per default**. L'A/B suona la
  proposta con le idee spuntate; ogni scheda ha un ascolto della proposta con quella sola idea. "Applica tutto"
  applica le idee spuntate nella stessa transazione (un passo di undo, HIS-3); con "applica solo le tracce scelte"
  di un'idea si applicano gli elementi delle tracce scelte, e un ritorno nuovo dell'idea segue AI-20. Un interruttore
  "Allow creative ideas" nel pannello AI, attivo per default e ricordato nel progetto, toglie `ideas` dallo schema.
  Una richiesta limitata ad alcune tracce (AI-5) può proporre idee solo su quelle e senza ritorni nuovi. Le proposte
  conservate (AI-23) riaprono anche le loro idee.

### 3.9 Dal mix al master (MDN)

- **MDN-1.** Il passaggio alla fase di master è un'azione esplicita ("Bounce & go to Master"): render offline
  del mix in `mixdowns/` (WAV float 32), con trim solo in attenuazione per restare entro −3 dBTP.
- **MDN-2.** Il master lavora **sul mixdown congelato**, non sul mix vivo. Se il mix cambia dopo il mixdown, la
  fase di master lo segnala e offre di rifarlo mantenendo le impostazioni di master.
- **MDN-3.** Si può tornare al mix in ogni momento; lo stato del master resta nel progetto.
- **MDN-4.** Ogni mixdown registra l'hash dello stato di mix da cui nasce.

### 3.10 Master (MAS)

- **MAS-1.** La catena di master (P-017): gain d'ingresso → i **blocchi liberi** nell'ordine scelto → guadagno di
  loudness trovato dall'app → limiter true peak con lookahead → dither (solo a 16 bit). I blocchi liberi sono EQ
  parametrico (6 bande), match EQ verso il profilo del riferimento (8 bande, quantità 0–100%), compressore collegato,
  compressore multibanda (MAS-12), ampiezza stereo (gain del side, mono sotto una frequenza), saturazione e clipper
  (MAS-11). Input, limiter e dither restano al loro posto: il limiter chiude la catena perché il tetto (MAS-3) vale
  solo se nulla lo segue. Ogni blocco ha un bypass.
- **MAS-2.** Destinazioni predefinite e personalizzabili: Streaming (−14 LUFS, −1 dBTP), Loud (−9 LUFS, −1 dBTP),
  CD (−10 LUFS, −0,3 dBTP).
- **MAS-3.** Il limiter **garantisce** il tetto di true peak (misurato con oversampling 4x sul file esportato):
  chiude D-074 della PoC.
- **MAS-4.** Meter: LUFS momentaneo, short-term e integrato, LRA, true peak, correlazione, spettro con il profilo
  del riferimento sovrapposto.
- **MAS-5.** Confronto A/B **a pari loudness** tra master, mixdown non processato e riferimento.
- **MAS-6.** **Master AI**: il modello riceve le misure del mixdown, il profilo e la loudness dei riferimenti
  (facoltativi), brief e destinazione, e restituisce un piano di master motivato (§5.5), come proposta (stesse regole
  di AI-2, AI-8, AI-10). Revisione a parole anche qui (D-077 della PoC). **È un ciclo misurato** (P-016): prima della
  decisione il master corrente è renderizzato alla loudness della destinazione e misurato; ogni piano proposto lo è a
  sua volta, e la decisione successiva riceve piano e misura (loudness raggiunta, riduzione del limiter e del
  compressore in profondità e in tempo, punch perso, distanza dal riferimento). Al più tre piani; il ciclo si chiude
  quando il modello conferma il piano appena misurato, e la proposta è l'ultimo piano. La **destinazione è
  dell'autore**: senza richiesta il piano la conserva; con una richiesta cambia solo se questa chiede a parole
  un'altra loudness, e chiedere più dinamica non lo è. Una richiesta su un master mai toccato chiede un master
  intero, non una revisione.
- **MAS-7.** Export: WAV (16/24 bit, float 32), FLAC (16/24 bit) e AAC in contenitore M4A (bitrate a scelta);
  44,1/48/88,2/96 kHz con conversione di qualità; dither TPDF a 16 bit; più formati in un solo export; rapporto
  di export (LUFS, true peak, LRA, distanza dal riferimento).
- **MAS-9.** Metadata: titolo, artista, album, anno, genere, numero di traccia, ISRC (validato nel formato),
  copertina. Scritti nei tag propri di ogni formato (chunk `LIST/INFO`, `bext` e `ID3` per WAV; commenti Vorbis
  e immagine per FLAC; atomi per M4A) e conservati nel progetto.
- **MAS-10.** L'AAC usa l'encoder del sistema operativo dietro un'interfaccia isolata (AudioToolbox su macOS);
  dove l'encoder manca, il formato non viene offerto. Per i formati con perdita, il true peak si misura sul file
  decodificato e, se supera il tetto, l'export si ripete con il margine necessario.
- **MAS-8.** Il master è deterministico: stesso mixdown e stesso piano ⇒ stesso file.
- **MAS-11.** **Clipper**: toglie fino a N dB ai picchi più alti che gli arrivano (0–12 dB), con sovracampionamento
  4x e una morbidezza regolabile; sotto la soglia non tocca nulla. La soglia è il picco misurato al suo ingresso
  sull'intero brano meno N dB: vale in ogni posizione della catena e con ogni destinazione. Finché quel picco non è
  misurato il clipper lascia passare tutto.
- **MAS-12.** **Compressore multibanda**: da due a sei bande separate da crossover Linkwitz-Riley del quarto ordine
  (la somma delle bande è piatta), ciascuna col suo compressore collegato (soglia, rapporto, attacco, rilascio,
  guadagno) e il suo bypass; crossover ad almeno un terzo d'ottava l'uno dall'altro. Il numero di bande si cambia a
  mano e dall'AI; l'ascolto di una banda in solo non entra nel progetto né nell'export.
- **MAS-13.** Il compressore di master ha un **mix** (compressione parallela) e un filtro passa-alto sul **detector**
  ("Listens above", 0 = spento): ciò che sta sotto quella frequenza non lo fa lavorare.
- **MAS-14.** **Scostamenti per sezione**: ogni sezione del brano può avere, sopra il master di tutto il brano e solo
  mentre suona, uno scostamento di livello verso il limiter (±3 dB), di guadagno dei lati (±3 dB, con il blocco
  Width acceso), di rilascio del limiter (±200 ms) e dei guadagni delle bande di un EQ (±3 dB). Entrano ed escono
  con una rampa di circa un decimo di secondo. La loudness della destinazione resta misurata sull'intero brano.
- **MAS-15.** **Blocchi ripetibili e ordine libero**: la catena è una lista. EQ, compressore, multibanda, saturazione
  e clipper possono esserci **due volte**, match EQ e ampiezza una; al più nove blocchi liberi (dodici con input,
  limiter e dither). Aggiungere, togliere, spostare e riportare un blocco ai valori iniziali sono un passo di undo
  ciascuno. All'ascolto un cambio di struttura non produce scatti; la latenza è la stessa per ogni catena.

### 3.11 Interfaccia e impostazioni (UI, SET)

- **UI-1.** Due spazi di lavoro, **Mix** e **Master**, con un selettore di fase sempre visibile e un'identità
  visiva che li distingue a colpo d'occhio.
- **UI-2.** Schermate e stati elencati in §6, fedeli al design consegnato.
- **UI-3.** Scorciatoie da tastiera per trasporto, editing, undo/redo, zoom, strumenti; elenco consultabile.
- **UI-4.** Finestra minima 1280×720; layout fluido fino a 4K; pannelli ridimensionabili e richiudibili.
- **UI-5.** Ogni controllo continuo: trascinamento, rotella, doppio clic per il default, inserimento numerico,
  modificatore per la regolazione fine.
- **UI-6.** Errori e stati vuoti espliciti e recuperabili (file mancanti, frequenze diverse, chiave assente, rete
  assente, dispositivo audio scollegato).
- **UI-7.** Accessibilità di base: navigazione da tastiera dei controlli, contrasto AA per testo e stati,
  etichette per i lettori di schermo sui controlli non testuali.
- **UI-8.** **Segni della catena** (P-013). Ogni intestazione di traccia e di gruppo nella timeline e ogni strip
  del mixer mostra, senza selezionare nulla, cinque segni in ordine di segnale (passa-alto, passa-basso, EQ,
  compressore, saturazione), **ciascuno con la sua sigla** (`HP`, `LP`, `EQ`, `CMP`, `SAT`), così che si legga quale
  blocco è senza ricordarne la posizione, e in uno di tre stati: **spento** (bypass), **acceso ma neutro** (non in bypass,
  ma non cambia il suono: EQ con tutte le bande attive a 0 dB o senza bande attive, saturazione con mix a 0),
  **al lavoro**. Il compressore con sidechain prende il colore della traccia che ascolta; quello con mix sotto il
  100% è segnato come parallelo. Un segno a parte dice che la traccia ha automazione, nel colore dell'assistente se
  le corsie le ha scritte lui. I segni restano a tutte le altezze di corsia e alla finestra minima, hanno
  un'etichetta a parole (tooltip e lettore di schermo) e derivano dal documento: nessuno stato proprio. Nelle strip
  le sigle stanno in colonna, larghe quanto la strip: lì un clic su una sigla mette o toglie il bypass del blocco e
  un doppio clic lo apre nell'inspector, con lo stesso comando dell'inspector (un passo di undo). Nelle
  intestazioni le sigle sono di sola lettura: il clic seleziona la traccia. Con P-018 i segni seguono la catena a
  blocchi (UI-12).
- **UI-9.** **Mandate leggibili** (P-013). Ogni strip mostra una mandata per **ogni** ritorno del progetto (fino a
  8), nello stesso ordine, col suo livello; una colonna di etichette fissa a sinistra del mixer dà il nome di ogni
  riga. Ogni intestazione mostra un punto per ritorno, acceso dove la traccia manda. La corsia di un ritorno
  elenca sempre chi gli manda, con colore, nome e livello.
- **UI-10.** **Collegamenti della selezione** (P-013). Selezionare una traccia, un gruppo o un ritorno marca in
  timeline e nel mixer ciò a cui è collegato, dicendo il motivo a parole: il gruppo che ne prende l'uscita, i
  ritorni a cui manda, le tracce che gli mandano (con il livello), le tracce il cui compressore lo ascolta. I
  segni dei collegamenti non cambiano il documento e non entrano nella cronologia.
- **UI-11.** **Overview del mixer** (P-013). Il mixer ha due viste sulle stesse colonne, scelte da un selettore:
  i fader, e una griglia con una riga per blocco della catena, una per ritorno, l'uscita e l'automazione. Ogni
  cella dice lo stato e il valore principale del blocco (frequenza, numero di bande, rapporto, drive, livello della
  mandata). Un clic su una cella mette o toglie il bypass (su una mandata assente la crea al livello predefinito);
  un doppio clic apre il blocco nell'inspector. Ogni modifica è lo stesso comando dell'inspector: un passo di undo.
- **UI-12.** **Catena a blocchi nell'interfaccia** (P-018). L'inspector mostra la catena come una lista, come il rack
  del master (schermata 15): un menu per aggiungere un blocco diviso per famiglia (Dynamics, Tone, Space, Movement,
  Pitch), trascinamento per l'ordine, duplica, riporta ai valori iniziali, bypass; ogni blocco nuovo ha il suo
  editor, con le curve dove servono (riduzione del de-esser e dell'EQ dinamico, curva del gate, forma della
  modulazione). Un ritorno mostra la sua lista sotto il processore. I segni di UI-8 diventano uno per blocco,
  nell'ordine della catena, ciascuno con la sua sigla (`HP`, `LP`, `EQ`, `CMP`, `SAT`, `GT`, `DS`, `DEQ`, `TR`, `CLP`,
  `MB`, `EXC`, `BAS`, `WID`, `MOD`, `TRM`, `HRM`, `DBL`); quando non ci stanno se ne vedono i primi e un `+n`. La vista
  Overview (UI-11) ha una riga per ogni tipo di blocco presente nel progetto.
- **UI-13.** **Idee creative nell'interfaccia** (P-018): la sotto-sezione Creative ideas nella proposta (schermata 4),
  con spunta e ascolto per idea (AI-27); l'interruttore "Allow creative ideas" nel pannello AI; gli elementi di
  un'idea applicata portano il nome dell'idea nel tooltip e nella motivazione della traccia.
- **UI-14.** **Vista Explain** (P-019, schermata 17). Nel Mix un pulsante `Explain` nella barra in alto (⌘E; `Esc`
  chiude) sostituisce timeline, mixer e pannelli con la mappa (UI-15) e il testo (UI-16). La barra in alto resta
  viva: trasporto, loop e undo funzionano. La vista è di **sola lettura**: non crea passi di cronologia e non
  modifica il progetto. Non chiama alcun modello e non usa la rete; a parità di progetto e di misure mostra la
  stessa pagina. Segue ogni modifica del progetto mentre è aperta.
- **UI-15.** **Mappa del mix** (P-019). Colonne da sinistra a destra: tracce audio raggruppate per famiglia (le
  famiglie di AI-22), gruppi accanto ai loro membri (una colonna per livello di annidamento), bus di mix; i ritorni
  ("shared effects") sotto i gruppi. Linea piena: uscita, tanto più spessa quanto più alto è il fader; tratteggiata:
  mandata; punteggiata con freccia nel colore della sorgente: sidechain. Sui riquadri: rombo dell'automazione
  (assistente o mano), lucchetto, numero della nota dell'assistente. Un clic seleziona un elemento: restano accesi
  il suo percorso fino al bus, le sue mandate col livello, chi lo alimenta e le sidechain; il resto si attenua.
  Doppio clic: chiude la vista e seleziona l'elemento nello spazio di lavoro. Un gruppo si ripiega in una riga col
  conteggio delle tracce. Una striscia delle sezioni sceglie tutto il brano o una sezione (le sezioni con lo stesso
  nome contano come una): le tracce che lì non suonano si attenuano, quelle che suonano solo lì o hanno valori
  diversi portano un'etichetta.
- **UI-16.** **Il mix in parole** (P-019). Testo in inglese costruito da frasi fisse riempite coi valori del
  progetto, in parti sempre nello stesso ordine (una parte senza contenuto non compare): in breve; struttura
  (gruppi, ritorni, tracce dirette al bus); chi sta davanti (ANA-20; finché la misura non c'è la parte lo dice);
  da sinistra a destra (pan); spazio (mandate per ritorno); chi reagisce a chi (sidechain); cosa cambia lungo il
  brano (clip presenti per sezione, automazione); motivi (`summary` e note dell'assistente, **citati**). Con un
  elemento selezionato il testo è la sua scheda: dove va, dove sta, i blocchi accesi della catena in ordine di
  segnale con una frase su cosa fa ciascuno, il motivo dell'assistente se c'è. Con una sezione: cosa suona, cosa è
  diverso, la loudness a confronto con le altre. I nomi sono chip col colore dell'elemento e lo selezionano nella
  mappa; i termini tecnici hanno una spiegazione di una frase da un glossario fisso. Il testo descrive e non
  giudica: nessun consiglio. `Copy as text` copia il testo semplice.
- **UI-17.** **Tour: avvio e ripetizione** (P-020, schermata 18). L'app ricorda nella sua cartella dei dati (non nel
  progetto) se il tour è già stato mostrato. Se non lo è mai stato, al primo avvio il tour **parte da solo** appena
  l'interfaccia è pronta, anche se esistono progetti recenti; il segno "mostrato" si scrive quando compare il primo
  passo, così un tour interrotto o un'app chiusa a metà non lo fanno ripartire da solo. A richiesta parte dalla voce
  **Help › Take the Tour** del **menu dell'app** (UI-20) e da un collegamento nella schermata Welcome. Con un
  progetto aperto il tour chiede prima quello che chiede la chiusura del progetto (salva, scarta, annulla), poi lo
  chiude. La voce di menu è disabilitata mentre l'assistente lavora o è in corso un bounce o un export. Il tour non
  parte mai da solo nel self-test dell'app né sul backend finto, se non chiesto in modo esplicito: i test
  esistenti non cambiano.
- **UI-18.** **Tour: i passi** (P-020). Ogni passo ha un **riflettore** sull'area di cui parla (il resto è attenuato)
  e un **fumetto** accanto: titolo, una o due frasi in inglese, contatore (`3 of 15`), `Back`, `Next` (`Done`
  all'ultimo), `Skip tour`. Il primo e l'ultimo passo sono schede al centro, senza riflettore.
  - **È il tour a guidare l'interfaccia.** Ogni passo dichiara lo stato che gli serve (fase, pannelli aperti, vista,
    selezione, proposta in revisione o applicata) e lo stabilisce da solo, andando avanti come tornando indietro:
    un passo si raggiunge da qualunque altro e mostra sempre la stessa cosa. Se l'utente ha cambiato qualcosa
    nell'area illuminata, il passo che ne dipende lo ripristina.
  - **L'area illuminata resta usabile**: si può premere Play, muovere un fader, aprire un blocco. Fuori dal
    riflettore i clic non agiscono. Per avanzare basta `Next`: nessun passo aspetta un'azione.
  - **Tastiera**: `→` o `Invio` avanti, `←` indietro, `Esc` salta il tour; la barra spaziatrice resta al trasporto.
    Le altre scorciatoie dell'app sono sospese finché il tour è attivo. I comandi di progetto del menu dell'app
    (New, Open, Close), raggiungibili quando la barra in alto è illuminata, chiudono prima il tour; Save e Save as non
    agiscono (PRJ-9).
  - **Passi, in quest'ordine** (i testi definitivi stanno nelle tavole): 1 scheda di apertura (cos'è Stemwise, che
    cosa si vedrà, circa tre minuti, `Start` e `Skip tour`); 2 trasporto e barra in alto (Play fa sentire il brano
    dimostrativo); 3 timeline, clip e sezioni (ogni file importato è una traccia, niente è distruttivo); 4
    intestazione di traccia (ruolo, mute, solo, lock, segni della catena); 5 mixer (strip, fader, mandate, gruppi,
    ritorni, vista Overview); 6 inspector e catena a blocchi della traccia selezionata; 7 assistente (brief, Auto
    mix e revisione a parole, **il modello non riceve mai audio**, stima del costo prima dell'invio); 8 proposta in
    revisione (sintesi, diff e motivi per traccia, A/B, applica tutto o le tracce scelte o scarta); 9 cronologia
    (la proposta applicata è un solo passo di undo; snapshot); 10 Explain; 11 selettore di fase e mixdown (mix e
    master sono due fasi); 12 Master: catena e destinazione; 13 Master: meter e A/B; 14 export (formati e
    metadata); 15 scheda di chiusura (da dove cominciare: New project, collegare l'assistente; il tour si rifà da
    Help).
  - **Accessibilità** (UI-7): il fumetto è un dialogo con etichetta, il fuoco resta tra il fumetto e l'area
    illuminata, il cambio di passo è annunciato ai lettori di schermo, contrasto AA, nessuna animazione con
    "reduce motion". Alla finestra minima (UI-4) il fumetto sta sempre dentro la finestra e l'area illuminata è
    portata in vista.
- **UI-19.** **Tour: uscita** (P-020). `Done`, `Skip tour`, `Esc` e la chiusura dell'app finiscono il tour allo stesso
  modo: la riproduzione si ferma, il progetto dimostrativo si chiude e si elimina (PRJ-9), l'interfaccia torna alla
  schermata Welcome e le preferenze dell'utente (aspetto, pannelli, larghezze) restano quelle di prima del tour.
  Chi salta riceve una notifica che dice dove ritrovarlo. Al primo avvio, finito il tour, la Welcome mostra come
  sempre l'invito a configurare l'assistente (schermata 1b). Senza dispositivo audio il tour prosegue senza suono,
  con lo stato di UI-6. Se il progetto dimostrativo non si può preparare, il tour lo dice, finisce e l'app resta
  usabile.
- **UI-20.** **Un solo menu dell'app** (2026-11-15, deciso dall'autore con P-020). Tutti i comandi di Stemwise stanno
  nel menu che si apre dal nome del progetto nella barra in alto, in Mix e in Master: `New project…` (⌘N), `Open…`
  (⌘O); `Project settings…`, `Save` (⌘S), `Save as…` (⇧⌘S), `Close project`; `Settings…` (⌘,); il sottomenu `Help`
  con `Take the Tour`. La barra dei menu di macOS tiene solo ciò che dà macOS: il menu Stemwise con About, Services,
  Hide, Hide Others, Show All e Quit; Edit, che serve ai campi di testo; Window. Le scorciatoie restano le stesse e le
  gestisce l'interfaccia (UI-3). Sulla schermata Welcome, dove non c'è un progetto, gli stessi comandi sono i suoi
  pulsanti e collegamenti.
- **SET-1.** Impostazioni: dispositivo audio e buffer; AI (chiave, modello, sforzo, decisore finto); aspetto;
  scorciatoie (solo consultazione nella v1).

### 3.12 Requisiti non funzionali (NFR)

- **NFR-1.** 32 tracce stereo a 48 kHz con catena completa, buffer da 512: il callback audio usa al più il 30%
  del periodo del buffer su Apple Silicon (benchmark automatico in release).
- **NFR-2.** Nessuna allocazione nel callback audio (verificato da un test con allocatore che la rifiuta).
- **NFR-3.** Interfaccia a 60 fps durante la riproduzione con 32 tracce visibili; avvio a freddo sotto 2 s;
  import di 16 stem di 4 minuti sotto 15 s.
- **NFR-4.** Render offline di 16 tracce almeno 20 volte più veloce del tempo reale.
- **NFR-5.** Memoria: l'audio non viene caricato per intero in RAM; un progetto di 32 stem di 5 minuti resta
  sotto 1,5 GB.
- **NFR-6.** Solo licenze permissive (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0, Unicode), verificate in
  automatico a ogni `make check`. Font e icone con licenza che ne consente l'inclusione (OFL, MIT, ISC).
- **NFR-7.** Nessuna chiamata di rete oltre a OpenRouter, e solo su azione dell'utente. Nessuna telemetria. Il
  collegamento del conto (AI-24) aggiunge un **ascolto locale temporaneo**: solo sulle interfacce di loopback, solo
  mentre l'utente sta collegando, per una sola risposta e al più dieci minuti. L'app apre nel browser solo pagine di
  OpenRouter, da un elenco fisso.
- **NFR-8.** Il codice non contiene nulla di specifico di macOS fuori da moduli isolati dietro un'interfaccia.
- **NFR-9.** Durante lo sviluppo il repository, artefatti di build compresi, resta sotto **20 GB** (§9, regole
  18–20).

## 4. Stack tecnologico

### 4.1 Scelta

| Ambito | Scelta | Note |
| --- | --- | --- |
| Shell desktop | **Tauri 2** | Webview di sistema (WKWebView su macOS), pacchetto piccolo, backend Rust nello stesso processo |
| Motore, DSP, analisi, AI, modello dati | **Rust** (workspace Cargo) | Tempo reale nativo; compilatore e `cargo test` come verifica stretta per gli agenti |
| Uscita audio | `cpal` | CoreAudio / WASAPI / ALSA con lo stesso codice |
| Decodifica | `symphonia` (MPL-2.0) | WAV, FLAC, AIFF; MP3/AAC per i riferimenti |
| Scrittura WAV | `hound` | |
| Scrittura FLAC | `flacenc` (o equivalente in Rust puro) | |
| Scrittura AAC | Encoder di sistema (AudioToolbox) dietro un trait | Unico modulo specifico di piattaforma oltre all'audio |
| Metadata | `lofty` (o equivalente) | Tag di WAV, FLAC, M4A |
| Ricampionamento | `rubato` | Import, riferimenti, uscita, export |
| FFT | `realfft` / `rustfft` | |
| Code senza lock | `rtrb` (o equivalente) | Parametri e meter tra thread audio e resto |
| HTTP | `reqwest` con `rustls` | Solo OpenRouter |
| Portachiavi | `keyring`; `security-framework` per sapere se la build è firmata | Keychain / Credential Manager / Secret Service; file privato dove il portachiavi chiederebbe la password (AI-11) |
| Contratti | `serde` + `schemars`; tipi TypeScript generati (`tauri-specta` o `ts-rs`) | Una sola definizione dei tipi, in Rust |
| Interfaccia | **React + TypeScript + Vite** | |
| Stato UI | `zustand` | Specchio dello stato del core, mai sorgente di verità |
| Stile | CSS con token del design come variabili (Tailwind 4 ammesso) | |
| Primitive accessibili | Radix UI | Menu, dialoghi, popover, tooltip |
| Grafica densa | Canvas 2D; WebGL solo se serve ai 60 fps | Timeline, forme d'onda, meter, spettro, curve |
| Test | `cargo test`, `proptest`, `criterion`; `vitest`; Playwright | |
| Gate licenze | `cargo-deny`, controllo licenze npm | |

Le versioni si fissano all'avvio dell'implementazione (ultima stabile di ciascuna), con lockfile committati.

### 4.2 Perché questo stack

Il criterio era doppio: massima efficacia dello sviluppo agentico, nessun compromesso evidente su prestazioni,
interfaccia e hardware.

| Alternativa | Motore audio | UI e design | Sviluppo agentico | Portabilità | Esito |
| --- | --- | --- | --- | --- | --- |
| **Tauri 2 + Rust + React** | Nativo, sicuro per costruzione | Il design in canvas è già HTML/CSS: si traduce quasi uno a uno | Molto alto: compilatore severo, test veloci, UI verificabile nel browser | Stesso codice sui tre sistemi | **Scelta** |
| JUCE (C++) | Standard del settore, hosting plugin pronto | UI imperativa, lenta da iterare, lontana dal design web | Medio: build lente, errori di memoria a runtime, UI non verificabile in automatico | Buona | Scartata: il suo vantaggio principale (plugin) è fuori scope; licenza commerciale |
| Swift + AVAudioEngine | Ottimo su Apple | SwiftUI fatica con timeline dense; design da ritradurre | Medio: verifica dell'UI difficile per un agente | Nessuna | Scartata: chiude Windows/Linux (P-002) |
| Electron + sidecar Python (PoC) | Python non regge il tempo reale; GPL | Come Tauri | Alto sull'UI, basso sul motore | Buona | Scartata: P-001 e P6 |
| Electron + addon nativo | Nativo | Come Tauri | Alto | Buona | Scartata: stesso disegno di Tauri con pacchetto più pesante e un ponte in più |

Punti che rendono lo sviluppo agentico efficace con lo stack scelto:

- **Verifica senza orecchie.** Il motore è una libreria pura: il render offline è lo stesso codice della
  riproduzione, quindi i test su segnali sintetici coprono ciò che si sentirà.
- **Oracolo numerico.** Le misure della PoC producono valori attesi con cui confrontare il port in Rust (§5.7).
- **UI verificabile nel browser.** L'interfaccia parla al backend attraverso un'interfaccia `Backend` con due
  implementazioni: Tauri e un **backend finto in memoria**. Con il secondo l'intera UI gira in un browser:
  Playwright la collauda e l'agente la confronta a vista con il design.
- **Un solo linguaggio per i contratti.** I tipi nascono in Rust e si generano per TypeScript: un disallineamento
  è un errore di compilazione, non un bug a runtime.

Limiti noti e contromisure:

- **La webview non è un toolkit nativo.** Il tempo reale non la attraversa mai: l'audio vive in Rust, l'UI riceve
  solo meter e posizione a 30–60 Hz su un canale binario e disegna su canvas. Le forme d'onda arrivano come
  piramidi di picchi già calcolate, non come audio.
- **Linux usa WebKitGTK**, più debole: è un rischio del lavoro futuro, non della v1.
- **Compressore e riverbero propri** non suoneranno come quelli della PoC: il port si convalida con le misure e
  con l'ascolto finale dell'autore (§8.5), come previsto da PRD §7.1.

## 5. Architettura

### 5.1 Struttura del repository

La PoC è **congelata** e confinata in `poc/` (A-068). Il prodotto vive in `app/`. Il sito di prodotto, quando
ci sarà, avrà la sua cartella `web/`.

```text
stemwise/
  CLAUDE.md                 # regole per gli agenti
  docs/
    FOUNDATION.md           # questo documento
    app/                    # PROGRESS, DECISIONS, REQS-JUSTIFIED, VALIDATION, dsp.md
  design/                   # consegna della sessione di design (§7)
  poc/                      # PoC Python: sola lettura, archivio e oracolo
    src/, tests/, schemas/, scripts/, pyproject.toml, Makefile
    docs/                   # PRD.md, analysis.md, DECISIONS.md, PROGRESS.md, milestones/
    POC-RULES.md            # regole di lavoro della PoC, archiviate
  app/
    Makefile                # setup, check, test, bench, reqs, oracle, dev, build
    Cargo.toml              # workspace
    crates/
      stemwise-core/        # documento di progetto, comandi, undo/redo, persistenza, schemi
      stemwise-dsp/         # biquad, compressore, saturazione, riverbero, delay, limiter, pan, meter
      stemwise-analysis/    # misure, mascheramento, profilo target, valutazione
      stemwise-engine/      # grafo, trasporto, streaming, automazione, render offline, uscita cpal
      stemwise-ai/          # Decider, prompt, OpenRouter, decisore finto, validazione, ciclo
      stemwise-cli/         # comandi senza interfaccia
    src-tauri/              # comandi e canali Tauri: involucro sottile sopra i crate
    ui/                     # React + TypeScript
      src/backend/          # interfaccia Backend: tauri.ts, mock.ts
    schemas/                # JSON Schema esportati dai tipi Rust
    fixtures/oracle/        # valori attesi generati dalla PoC (JSON, mai audio)
    scripts/                # reqs, oracle, licenze
```

Dipendenze tra crate: `dsp` e `core` non dipendono da nessun altro; `analysis` → `dsp`; `engine` → `core`,
`dsp`; `ai` → `core`, `analysis`, `engine` (render offline del ciclo); `cli` e `src-tauri` → tutti. Nessun crate
sotto `src-tauri` conosce Tauri.

### 5.2 Pacchetto di progetto

```text
MySong.stemwise/
  project.json          # documento: versione di schema, tracce, clip, catene, automazioni, sezioni, brief, master
  audio/                # file importati (immutabili), nome = hash del contenuto
  references/
  cache/                # picchi, analisi (rigenerabile, eliminabile)
  ai/                   # una cartella per chiamata: request.json, response.json, plan.json, evaluation.json
  snapshots/
  mixdowns/             # mixdown-<n>.wav + mixdown-<n>.json (hash dello stato di mix, trim, misure)
  masters/              # file esportati + rapporto
  autosave/
```

`project.json` contiene solo percorsi relativi al pacchetto. Il salvataggio è atomico (scrittura su file
temporaneo e rinomina).

### 5.3 Stato, comandi e cronologia

- Il **documento di progetto** vive nel core Rust ed è l'unica sorgente di verità. L'UI ne tiene una copia in
  sola lettura, aggiornata da eventi.
- Ogni mutazione è un `Command` (dati, non closure) con `apply` e `invert`. Una `Transaction` raggruppa più
  comandi in un passo di cronologia. I gesti continui aprono una transazione al press e la chiudono al release;
  durante il gesto il motore riceve i valori in anteprima senza toccare la cronologia.
- Lo stato di vista (zoom, selezione, mute/solo, pannelli aperti) sta nell'UI e, dove serve, in una sezione
  separata del progetto che non genera cronologia.
- Il motore audio riceve un'istantanea immutabile del grafo a ogni modifica strutturale (scambio atomico di
  puntatore, rilascio della vecchia fuori dal thread audio) e i cambi di parametro su una coda senza lock.

### 5.4 Motore audio

```text
thread di lettura ──buffer circolari──▶ thread audio (cpal) ──▶ dispositivo
                                           │   ▲
                                 meter/pos │   │ parametri, istantanee del grafo
                                           ▼   │
                                        thread di controllo ◀──▶ UI (canali Tauri)
```

- Grafo aciclico **strutturato**: clip → traccia (catena) → gruppi annidati (catena) → bus di mix; mandate
  post-fader verso i ritorni; ritorni → gruppo o bus di mix; ingressi sidechain dei compressori. A ogni modifica
  strutturale il core ricalcola l'ordine topologico e rifiuta i cicli; il thread audio riceve l'ordine già
  calcolato.
- Il **render offline** chiama la stessa funzione `process` del callback, con lettura sincrona dei file.
- Il determinismo vale per stessa build e stessa architettura; l'identità bit a bit tra sistemi diversi non è
  garantita né richiesta.
- La fase di master usa un secondo grafo, più semplice (mixdown → catena di master), con lo stesso trasporto.

### 5.5 Contratti dell'AI

I contratti sono tipi Rust esportati in `app/schemas/`. Nessun nome di libreria nei JSON. Gli esempi sono
illustrativi; i limiti sono normativi.

**`mix_plan` (versione 3)** — estende quello della PoC (PRD §6.3); la versione 3 aggiunge `structure`, gli
algoritmi di riverbero e il `mix` del compressore (P-011):

```json
{
  "version": 3,
  "iteration": 1,
  "summary": "Voce avanti nel suo plate, chitarre doppie aperte ai lati, batteria più compatta, chorus più pieno.",
  "structure": {
    "returns": {
      "n1": { "type": "reverb", "name": "Vocal plate", "rationale": "Uno spazio corto e brillante solo per la voce." },
      "n2": { "type": "delay", "name": "Slap", "rationale": "Slapback per dare corpo alla voce senza coda." }
    },
    "groups": {
      "n4": { "name": "Drums", "members": ["t1", "t3"], "rationale": "Un bus per comprimere la batteria in parallelo." }
    }
  },
  "returns": {
    "r1": { "type": "reverb", "algorithm": "hall", "decay_s": 2.2, "size": 0.6, "damping": 0.5, "modulation": 0.2, "predelay_ms": 20, "return_hpf_hz": 250, "return_lpf_hz": 9000, "return_gain_db": -6.0 },
    "r2": { "type": "delay", "time_ms": 375, "feedback": 0.25, "feedback_hpf_hz": 200, "feedback_lpf_hz": 6000, "return_gain_db": -9.0 },
    "n1": { "type": "reverb", "algorithm": "plate", "decay_s": 1.1, "size": 0.4, "damping": 0.3, "modulation": 0.3, "predelay_ms": 40, "return_hpf_hz": 300, "return_lpf_hz": 12000, "return_gain_db": -8.0 },
    "n2": { "type": "delay", "time_ms": 110, "feedback": 0.0, "feedback_hpf_hz": 200, "feedback_lpf_hz": 5000, "return_gain_db": -12.0 }
  },
  "tracks": {
    "t4": {
      "gain_db": 2.0, "pan": 0.0, "hpf_hz": 90, "lpf_hz": null,
      "eq": [{ "type": "peak", "freq_hz": 300, "gain_db": -2.5, "q": 1.4 }],
      "compressor": { "threshold_db": -18.0, "ratio": 3.0, "attack_ms": 10.0, "release_ms": 120.0, "knee_db": 6.0, "makeup_db": 3.0, "mix": 1.0, "sidechain": null },
      "saturation": null,
      "sends_db": { "n1": -14.0, "n2": -18.0, "r2": -24.0 },
      "sections": { "s3": { "gain_offset_db": 1.5, "send_offsets_db": { "r2": 6.0 }, "eq_gain_offsets_db": [1.0] } },
      "rationale": "Taglio a 300 Hz per togliere la sovrapposizione con le chitarre; +1,5 dB e un throw di delay nel chorus."
    },
    "t2": {
      "gain_db": 0.0, "pan": 0.0, "hpf_hz": 40, "lpf_hz": null, "eq": [],
      "compressor": { "threshold_db": -24.0, "ratio": 4.0, "attack_ms": 5.0, "release_ms": 90.0, "knee_db": 3.0, "makeup_db": 0.0, "mix": 1.0, "sidechain": "t1" },
      "saturation": null, "sends_db": {}, "sections": {},
      "rationale": "Il basso si abbassa di poco a ogni colpo di cassa (sidechain da t1) per liberare le basse."
    }
  },
  "groups": {
    "g1": { "gain_db": -1.0, "eq": [], "compressor": null, "saturation": null, "sends_db": {}, "rationale": "Chitarre 1 dB sotto per far spazio alla voce." },
    "n4": {
      "gain_db": 0.0, "eq": [],
      "compressor": { "threshold_db": -30.0, "ratio": 6.0, "attack_ms": 3.0, "release_ms": 80.0, "knee_db": 3.0, "makeup_db": 8.0, "mix": 0.35, "sidechain": null },
      "saturation": null, "sends_db": {},
      "rationale": "Compressione parallela: il 35% molto compresso sotto la batteria naturale la rende più densa."
    }
  }
}
```

**Struttura** (AI-17 → AI-20). `structure` elenca ciò che il piano crea: ritorni (`type`, `name`, `rationale`) e
gruppi (`name`, `members`, `rationale`). Gli elementi nuovi hanno id segnaposto già elencati nello schema del
progetto (`n1` … `n3` per i ritorni, `n4` … `n6` per i gruppi; `null` dove non servono), e si usano nel resto del piano come quelli esistenti: `returns.<n>` e `groups.<n>` per i
parametri, `sends_db.<n>` per le mandate. All'applicazione il core assegna gli id veri. `name` è in inglese
(lingua dell'interfaccia), al più 24 caratteri. Il modello non crea, duplica o elimina tracce audio, non elimina
ritorni o gruppi e non cambia l'uscita di una traccia se non mettendola in un gruppo nuovo. `structure` manca
dallo schema quando le modifiche strutturali sono disattivate (AI-18) e nelle iterazioni successive alla prima
(AI-19), che ricevono la struttura già decisa come contesto.

L'auto mix garantisce che esistano almeno un ritorno riverbero e uno delay (creati, se mancano, come parte della
proposta). `sidechain` è l'id di una traccia o di un gruppo, oppure `null`; la validazione rifiuta le sorgenti
che creerebbero un ciclo.

Limiti: quelli della tabella di PRD §6.3 per i parametri già presenti (tranne `room_size` del riverbero, che non
esiste più), più:

| Parametro | Minimo | Massimo |
| --- | --- | --- |
| Compressore `mix` | 0,1 | 1 |
| Riverbero `algorithm` | `plate`, `room`, `hall` | |
| Riverbero `decay_s` | 0,3 | 6 |
| Riverbero `size` | 0 | 1 |
| Riverbero `damping` | 0 | 1 |
| Riverbero `modulation` | 0 | 1 |
| Riverbero `return_lpf_hz` | 2.000 | 20.000 |
| Ritorni nuovi per proposta | 0 | 3 |
| Gruppi nuovi per proposta | 0 | 3 |
| Membri di un gruppo nuovo | 2 | — |
| Compressore `knee_db` | 0 | 12 |
| Saturazione `drive_db` | 0 | 18 |
| Saturazione `mix` | 0 | 1 |
| `sends_db.<ritorno>` | −60 | −6 |
| Riverbero `predelay_ms` | 0 | 100 |
| Delay `time_ms` | 20 | 1.000 |
| Delay `feedback` | 0 | 0,7 |
| Delay `feedback_hpf_hz` | 20 | 800 |
| Delay `feedback_lpf_hz` | 1.000 | 20.000 |
| Delay `return_gain_db` | −24 | 0 |
| Sezione `gain_offset_db` | −6 | +6 |
| Sezione `send_offsets_db.<ritorno>` | −12 | +12 |
| Sezione `eq_gain_offsets_db[]` (per posizione di banda) | −4 | +4 |

Il trim (AI-7) non è nel piano: lo calcola l'engine dalla misura di loudness e lo aggiunge alla proposta. Le
tracce bloccate compaiono nel messaggio come contesto, con `"locked": true`, e non nel piano restituito.

**`mix_plan` (versione 4)** (P-018) — la versione 3 più i blocchi, le idee e gli scostamenti nuovi. Si mostrano solo
le aggiunte:

```json
{
  "version": 4,
  "ideas": {
    "i1": { "name": "Wide chorus on the keys", "sections": ["s3", "s5"], "rationale": "Le tastiere si aprono nei ritornelli e lasciano il centro alla voce." },
    "i2": { "name": "Filter build into the last chorus", "sections": ["s4"], "rationale": "Il pad parte chiuso e si apre fino al ritornello finale." }
  },
  "tracks": {
    "t4": {
      "hpf_resonance": null, "lpf_resonance": null,
      "blocks": [
        { "id": "k1", "type": "deesser", "position": "after_compressor", "freq_hz": 6500, "threshold_db": -28.0, "max_reduction_db": 5.0, "mode": "split" }
      ],
      "rationale": "Sibilanza alta (+7 dB a 7 kHz): de-esser dopo il compressore."
    },
    "t5": {
      "blocks": [
        { "id": "k2", "type": "dynamic_eq", "position": "before_compressor", "bands": [
          { "type": "peak", "freq_hz": 3000, "q": 1.2, "threshold_db": -30.0, "range_db": -3.0, "attack_ms": 10.0, "release_ms": 120.0, "sidechain": "t4" }] }
      ],
      "rationale": "Le chitarre scendono a 3 kHz solo mentre canta la voce, invece di un taglio fisso."
    },
    "t7": {
      "blocks": [
        { "id": "k3", "idea": "i1", "type": "modulation", "position": "last", "mode": "chorus", "rate": "1/4", "rate_hz": null, "depth": 0.4, "feedback": 0.0, "voices": 2, "stages": null, "spread_deg": 120, "mix": 0.35 }
      ],
      "sections": { "s3": { "blocks_on": ["k3"] }, "s5": { "blocks_on": ["k3"] } }
    },
    "t8": {
      "lpf_hz": 18000, "lpf_resonance": 2.0,
      "sections": { "s4": { "idea": "i2", "lpf_offset_oct": -3.0 } }
    }
  },
  "returns": {
    "r1": {
      "blocks": [
        { "id": "k4", "type": "compressor", "position": "last", "threshold_db": -30.0, "ratio": 4.0, "attack_ms": 5.0, "release_ms": 250.0, "knee_db": 6.0, "makeup_db": 0.0, "mix": 1.0, "sidechain": "t4" }
      ]
    }
  }
}
```

**Blocchi.** `blocks` elenca solo i blocchi che il piano **aggiunge o cambia**: un blocco che non c'è resta com'è
(al contrario del master, perché le tracce sono molte e la risposta deve restare corta, A-062). `id` è l'id di un
blocco esistente (l'analisi li elenca, ANA-19) oppure una parola nuova per un blocco nuovo; un blocco nuovo ha la sua
`position` (AI-25); `"bypass": true` spegne un blocco esistente. I blocchi classici restano nei campi della
versione 3 (`hpf_hz`, `eq`, `compressor`, `saturation`), a cui si aggiungono `hpf_resonance`, `lpf_resonance` e
`saturation.character`. Lo schema per progetto ha un `anyOf` sui tipi di blocco, come `master_plan` v2. Nei ritorni
`blocks` è ammesso solo se le modifiche strutturali sono permesse (un ritorno è condiviso, AI-25); il riverbero ha
anche `shimmer_semitones` e `shimmer_amount`, il delay `mode`, `character`, `wow` e `drive_db`.

**Idee.** `ideas` manca dallo schema quando le idee sono disattivate (AI-27) e nelle iterazioni dopo la prima, che
ricevono le idee già decise come contesto. Un elemento appartiene a un'idea con `"idea": "<id>"`: un blocco, una
sezione di una traccia (tutti i suoi scostamenti), una voce di `structure`. Una mandata di un'idea si scrive in
`idea_sends_db.<idea>.<ritorno>` della traccia, separata da `sends_db`.

Limiti aggiunti dalla versione 4:

| Parametro | Minimo | Massimo |
| --- | --- | --- |
| Idee per proposta | 0 | 3 |
| Blocchi nuovi per catena in una proposta | 0 | 3 |
| `hpf_resonance`, `lpf_resonance` | 0,5 | 4 |
| Sezione `hpf_offset_oct`, `lpf_offset_oct` | −3 | +3 |
| Gate `threshold_db` / `range_db` / `ratio` | −70 / 0 / 1 | −10 / 40 / 20 |
| Gate `attack_ms` / `hold_ms` / `release_ms` | 0,1 / 0 / 10 | 20 / 200 / 1.000 |
| De-esser `freq_hz` / `max_reduction_db` | 3.000 / 0 | 12.000 / 10 |
| EQ dinamico: bande / `range_db` / `q` | 1 / −9 / 0,3 | 2 / +4 / 6 |
| Transient `attack_db` / `sustain_db` | −9 | +9 |
| Clipper di traccia `threshold_db` | −18 | 0 |
| Multibanda di traccia: bande / guadagno di banda | 2 / −6 | 4 / +6 |
| Exciter `freq_hz` / `amount` | 1.500 / 0 | 12.000 / 0,6 |
| Bass enhancer `freq_hz` / `harmonics` | 40 / 0 | 160 / 0,8 |
| Width `width` / `mono_below_hz` | 0 / 0 | 1,6 / 250 |
| Modulazione `rate_hz` / `depth` / `feedback` / `mix` | 0,02 / 0 / −0,8 / 0 | 8 / 1 / 0,8 / 0,6 |
| Tremolo `rate_hz` / `depth` | 0,1 / 0 | 12 / 0,8 |
| Harmonizer: voci / intervallo / mix | 1 / −12 / 0 | 2 / +12 / 0,5 |
| Doubler `detune_cents` / `delay_ms` / `mix` | 0 / 5 / 0 | 25 / 40 / 0,6 |
| Riverbero `shimmer_amount` | 0 | 0,6 |
| Delay `wow` / `drive_db` | 0 / 0 | 1 / 9 |

I valori musicali (`rate`, `time` del delay) sono ammessi solo se il progetto ha un tempo. Gli altri parametri dei
blocchi nuovi hanno nel piano gli stessi limiti dell'interfaccia (FX-13 → FX-23). I piani della versione 3 conservati
nei progetti si leggono ancora (AI-23).

**`analysis` (versione 3)** — quello della PoC (PRD §6.2) più: `sections` (id, nome, inizio, fine),
`tracks.<id>.section_lufs`, `tracks.<id>.section_active_pct`, `mix.section_bands_db`, `families` (ANA-16), `returns` (id, tipo e,
per i riverberi, algoritmo), `doubles` (ANA-18: coppie di tracce, punteggio, `identical`) e,
per ogni traccia, `name`, `group`, `locked`. Gli id inviati sono neutri (`t1`, `g1`, `s1`). La **versione 4**
(P-018) aggiunge le misure di ANA-19 (`sibilance_db` e `sibilance_hz` per le voci, `crest_db`, `transient_db`,
`floor_db` per la batteria, `stereo` per le tracce stereo), `tempo` e `meter` del progetto se noti, e `blocks` per
ogni traccia, gruppo e ritorno (id neutro `k…`, tipo, bypass, posizione dei blocchi non classici).

**`master_plan` (versione 2)** (P-017):

```json
{
  "version": 2,
  "summary": "Stessa loudness con un terzo del lavoro del limiter: i picchi scendono prima.",
  "target": { "lufs": -9.6, "true_peak_db": -1.0 },
  "input_gain_db": 0.0,
  "chain": [
    { "type": "compressor", "id": "b3", "threshold_db": -9.0, "ratio": 3.0, "attack_ms": 3.0, "release_ms": 90.0, "knee_db": 4.0, "makeup_db": 0.0, "mix": 0.6, "listen_above_hz": 120, "why": "…" },
    { "type": "eq", "id": "b1", "bands": [{ "type": "low_shelf", "freq_hz": 45, "gain_db": -1.5, "q": 0.7 }], "why": "…" },
    { "type": "multiband", "id": "b4", "crossovers_hz": [120, 4000], "bands": [
      { "threshold_db": -14.0, "ratio": 2.5, "attack_ms": 20.0, "release_ms": 160.0, "gain_db": 0.0 },
      { "threshold_db": 0.0, "ratio": 1.0, "attack_ms": 20.0, "release_ms": 160.0, "gain_db": 0.0 },
      { "threshold_db": 0.0, "ratio": 1.0, "attack_ms": 20.0, "release_ms": 160.0, "gain_db": 0.0 }], "why": "…" },
    { "type": "eq", "id": "tone", "bands": [{ "type": "high_shelf", "freq_hz": 9000, "gain_db": 1.0, "q": 0.7 }], "why": "…" },
    { "type": "clipper", "id": "b7", "take_off_db": 4.0, "softness": 0.4, "why": "…" }
  ],
  "limiter": { "release_ms": 80.0 },
  "sections": [{ "section": "s3", "level_db": 0.5, "side_gain_db": 0.0, "limiter_release_ms": 0.0, "eq": [{ "block": "tone", "gains_db": [0.6] }] }],
  "rationale": { "order": "…", "limiter": "…", "sections": "…" }
}
```

`chain` elenca solo i blocchi che lavorano, nell'ordine in cui lavorano: un blocco del master che il piano non
elenca viene spento e resta in coda con le sue impostazioni. `id` è l'id di un blocco che il master ha già (l'analisi
li elenca in `current_chain`) oppure una parola qualunque per un blocco nuovo. Tipi: `eq`, `match_eq`, `compressor`,
`multiband`, `width`, `saturation`, `clipper`. Ogni blocco porta la sua motivazione in `why`; `rationale` ha solo
`order`, `limiter` e `sections`. La proposta si applica intera o **voce per voce** (ogni blocco, l'ordine, il
limiter, le sezioni, l'ingresso, la destinazione). I piani della versione 1 conservati nei progetti si leggono
ancora (AI-23).

**Analisi del master (versione 3)** — ciò che il modello legge prima di decidere un master (MAS-6):
`mixdown` (`lufs`, `true_peak_db`, `peak_to_loudness_db`, `short_crest_db`: cresta mediana su finestre di 400 ms,
`peak_share_pct`: quota dei campioni entro 3, 6 e 9 dB dal picco, `loudness_range_lu`, `bands_db`, `stereo`,
`low_corr`), `target` (profilo medio dei riferimenti), `references` (loudness e distanza picco-loudness di ciascuno,
senza nomi), `band_delta_db`, `destination` (con `peak_to_loudness_db` e `peak_reduction_needed_db`: di quanto i
picchi devono scendere in tutto) e `current_master`, la **valutazione** del master corrente. Una valutazione ha
`lufs`, `on_target`, `loudness_gain_db`, `peak_to_loudness_db`, `short_crest_db`, `short_crest_lost_db`,
`peak_share_pct`, `blocks` (per ogni compressore, multibanda e clipper acceso, col suo id) e `limiter` (`max_db`,
`mean_db`, percentuale del tempo con una riduzione qualunque e oltre 1, 3 e 6 dB), `spectral_distance_db`,
`band_delta_db`. Dalla versione 3 l'analisi porta anche `sections` (id neutro, nome, inizio, fine e loudness del
mixdown in ciascuna) e `current_chain` (i blocchi del master con id, tipo e stato). La stessa valutazione accompagna ogni piano nella storia delle
iterazioni.

Limiti del master: al più 9 blocchi, due per tipo (uno per `match_eq` e `width`); EQ fino a 6 bande, gain ±6 dB;
`match_eq.amount` 0–1; ratio 1–4; `compressor.mix` 0,2–1; `compressor.listen_above_hz` 0–300; multibanda con 2–6
bande, crossover 40–12 000 Hz ad almeno un terzo d'ottava, guadagno di banda ±6 dB; `side_gain_db` ±6;
`mono_below_hz` 0–250; `clipper.take_off_db` 0–6; `target.lufs` −24…−6; `target.true_peak_db` −3…−0,1;
`limiter.release_ms` 20–500; scostamenti di sezione: livello ±2 dB, lati ±3 dB, rilascio ±100 ms, EQ ±2 dB.

Il prompt di sistema parte da quello della PoC (`poc/src/stemwise/decision/prompt.py`), con la tabella dei limiti
**generata dallo schema**, mai copiata a mano.

### 5.6 Interfaccia tra UI e core

- Comandi (richiesta/risposta) per mutazioni e interrogazioni; eventi per i cambi di stato (patch del
  documento); canale binario per meter, posizione e spettri; protocollo dedicato per le piramidi di picchi.
- L'UI non contiene logica di dominio: niente calcoli di dB, tempi o validazioni duplicati rispetto al core, se
  non per il disegno.
- `ui/src/backend/mock.ts` implementa la stessa interfaccia con un progetto dimostrativo in memoria (tracce,
  forme d'onda sintetiche, meter simulati, una proposta AI di esempio): è ciò che usano Playwright e la verifica
  a vista.

### 5.7 La PoC come oracolo

`poc/src/stemwise/` non si modifica. `make oracle` (in `app/`) esegue uno script che genera segnali sintetici con seme
fisso, li misura con le funzioni della PoC e scrive in `app/fixtures/oracle/` **solo JSON** (parametri del
segnale e valori attesi). I test Rust rigenerano gli stessi segnali e confrontano:

| Ambito | Tolleranza rispetto all'oracolo |
| --- | --- |
| Loudness integrato, LRA | ±0,1 LU |
| True peak, picco, RMS, crest | ±0,1 dB |
| Energia per banda, quote, centroide | ±0,2 dB · ±1 punto percentuale · ±2% |
| Mascheramento, correlazioni | ±0,02 |
| Biquad (risposta in frequenza), pan, gain | ±0,01 dB |

Compressore, saturazione, riverbero, delay e limiter non hanno oracolo: si verificano con proprietà (guadagno
statico, costanti di tempo, tetto rispettato, stabilità, assenza di NaN e di denormali) documentate in
`docs/app/dsp.md`. I generatori di segnali devono essere definiti in modo identico nei due linguaggi (formule
chiuse o campioni scritti nel JSON per i segnali brevi), mai con generatori casuali delle librerie.

## 6. Inventario dell'interfaccia

Elenco vincolante per il design e per l'implementazione. Dimensione di riferimento 1440×900; minima 1280×720.

**Schermate**

1. **Welcome** — progetti recenti, nuovo progetto, apri; primo avvio con invito a configurare l'AI (saltabile).
2. **New project / Import** — area di trascinamento, elenco dei file con formato, canali, durata, ruolo
   proposto e modificabile; avviso di frequenza diversa; brief (genere, mood, note); riferimenti.
3. **Mix workspace** — la schermata principale:
   - barra superiore: nome del progetto, selettore di fase **Mix | Master**, trasporto, posizione, loop,
     undo/redo, meter del bus (true peak, LUFS), carico del motore;
   - timeline: righello, corsia delle sezioni, intestazioni di traccia (nome, ruolo, colore, mute/solo, lock,
     mini fader), clip con forme d'onda, corsie di automazione espandibili, cursore, regione di loop;
   - mixer (pannello inferiore o vista alternativa): strip con meter, fader, pan, mandate, riduzione di guadagno;
     gruppi, ritorni, bus di mix; con P-013: segni della catena su ogni intestazione e strip (UI-8), una mandata
     per ritorno con la colonna delle etichette (UI-9), vista **Overview** a griglia (UI-11), anche su un
     progetto grande con corsie basse;
   - inspector della traccia selezionata: catena con bypass, editor EQ, compressore, saturazione, mandate,
     **motivazione dell'AI** per quella traccia;
   - pannello AI: brief, riferimenti, Auto mix, barra della revisione a parole, interruttori "Allow structural
     changes" (AI-18) e "Allow creative ideas" (AI-27), stima del costo prima dell'invio, costo del progetto finora con accesso ad "AI usage";
   - pannello cronologia: passi di undo, snapshot, confronto A/B.
4. **Mix workspace, proposta in revisione** — sintesi, elenco delle tracce con diff e motivazione, **ritorni e
   gruppi nuovi come righe proprie** (marcati come nuovi, con nome, motivazione, parametri e, per i gruppi, i
   membri; AI-20), valutazione prima/dopo, A/B all'ascolto, applica tutto / applica selezionate / scarta; con
   "applica selezionate" si vede quali elementi nuovi verranno creati; con P-018 i blocchi nuovi nel diff e la
   sotto-sezione **Creative ideas** (una scheda per idea, spunta spenta per default, ascolto della sola idea).
5. **Mix workspace, AI al lavoro** — avanzamento per fase e per iterazione, annulla.
6. **Editing** — strumenti (selezione, taglio, automazione), stati di trascinamento, fade, snap, selezione
   multipla, menu contestuali di clip e traccia.
7. **Returns & routing** — pannelli dei ritorni (riverbero con selettore dell'algoritmo e i parametri di FX-5,
   delay), creazione ed eliminazione di un ritorno, selettore dell'uscita di una traccia, gruppi annidati nel
   mixer e nella timeline, selettore della sorgente sidechain e controllo **mix** nel compressore (FX-3), scelta
   del parametro da automatizzare in una corsia; con P-013: i collegamenti della selezione (chi manda a un
   ritorno, dove va una traccia, chi la ascolta in sidechain; UI-10) e l'elenco di chi manda nella corsia del
   ritorno.
8. **Bounce to Master** — conferma, avanzamento, esito con le misure del mixdown.
9. **Master workspace** — forma d'onda del mixdown, catena di master a moduli, meter di loudness e spettro con
   profilo del riferimento, destinazione, pannello AI del master, A/B a pari loudness (master / mixdown /
   riferimento), avviso di mixdown non aggiornato.
10. **Export** — formati (WAV, FLAC, AAC, anche insieme), frequenza, bit, dither, metadata (con ISRC e
    copertina), cartella; avanzamento; rapporto finale.
11. **Settings** — Audio, AI, Appearance, Shortcuts.
12. **AI usage** — riepilogo (chiamate, token, costo, per tipo e per modello), storia delle chiamate con
    filtri, dettaglio leggibile di una chiamata (richiesta e risposta, con il JSON grezzo a richiesta), export,
    stato vuoto; conferma di superamento del tetto di spesa.
13. **Stati trasversali** — vuoti, caricamento, errori di §3.11 UI-6, conferme distruttive, notifiche.
14. **Assistant setup** (P-015) — collegamento del conto OpenRouter dal browser, attesa, pagina mostrata nel
    browser, credito, pronto, esiti negativi, chiave incollata a mano, e gli stati di credito esaurito, limite della
    chiave e chiave non più valida nella colonna dell'assistente.
15. **Master chain** (P-017) — catena a dieci moduli con ordine libero al centro, clipper,
    compressore con mix e filtro sul detector, multibanda, scostamenti per sezione, proposta misurata, destinazione
    troppo alta per il mix.
16. **Track chain** (P-018) — la catena a blocchi nell'inspector di traccia, gruppo e ritorno (UI-12): menu per
    famiglia, ordine, stati di trascinamento, un editor per ciascun blocco nuovo di FX-12 → FX-23, segni e Overview
    con le catene lunghe, il delay e il riverbero coi modi nuovi.

17. **Explain this mix** (P-019) — la vista che spiega il mix (UI-14 → UI-16): tutto il mix, una traccia, un
    ritorno, una sidechain con un termine spiegato, una sezione, un progetto grande coi gruppi ripiegati, un mix mai
    toccato, la finestra minima, il testo intero.

18. **Guided tour** (P-020) — il tour (UI-17 → UI-19): scheda di apertura sulla Welcome, un passo col riflettore
    per ciascuna area (barra in alto, timeline, intestazione, mixer, inspector, assistente, proposta, cronologia,
    Explain, selettore di fase, Master, export), scheda di chiusura, un passo che aspetta la proposta o il mixdown,
    la finestra minima, il collegamento nella Welcome e la voce Help del menu dell'app (UI-20), la notifica dopo `Skip tour`.

**Componenti** (ciascuno con gli stati: normale, hover, attivo, focus, disabilitato, automatizzato, bloccato
dove ha senso): fader, knob, pan, meter (picco/RMS, riduzione di guadagno, LUFS), pulsanti e toggle (mute, solo,
lock, bypass), campo numerico, menu, tooltip, dialogo, toast, tab, clip, maniglie di fade, punto di automazione,
sezione, strip di canale, modulo di catena, curva EQ, curva di trasferimento, righello, cursore di riproduzione,
barra di avanzamento, scheda di proposta, riga di diff, riga di chiamata AI, tabella di misure, selettore di
routing, indicatore di sidechain; con P-018: blocco di catena di traccia, menu dei blocchi, scheda di un'idea; con P-013: segni della catena, punti delle mandate, segno di automazione, cella
dell'Overview, colonna delle etichette del mixer; con P-020: riflettore e fumetto del tour.

## 7. Passaggio dal design all'implementazione

La sessione di design consegna, nella cartella `design/` del repository:

- `design/README.md` — indice: direzione scelta, principi, elenco delle schermate e dei file, note per chi
  implementa (comportamenti non deducibili dalle tavole: resize, densità, animazioni, stati).
- `design/tokens.css` — tutti i token come variabili CSS (colori, tipografia, spaziatura, raggi, ombre, durate,
  colori di traccia, colori delle due fasi).
- `design/screens/` — una tavola per schermata e stato di §6 (HTML/CSS autosufficiente, oppure immagine più
  specifica misurata).
- `design/components/` — il foglio dei componenti con tutti gli stati.

Se la sessione di design non può scrivere nel repository, l'autore esporta le tavole e le copia in `design/`
prima di avviare l'implementazione. L'implementazione non parte senza `design/README.md` e `design/tokens.css`.

Durante l'implementazione il design è normativo per aspetto e disposizione; questo documento lo è per
comportamento. Dove il design manca di uno stato, l'agente lo deriva dai token e dai componenti esistenti e lo
annota in `docs/app/DECISIONS.md`.

## 8. Piano di implementazione

### 8.1 Fasi

Si eseguono in ordine, **senza pause tra una e l'altra**. Ogni fase si chiude quando il suo gate è verde; lo
stato si aggiorna in `docs/app/PROGRESS.md`.

| Fase | Contenuto | Gate (oltre a `make check` verde) |
| --- | --- | --- |
| **F0 Fondamenta** | Workspace Cargo, app Tauri che si avvia, UI con backend finto, token del design importati, `Makefile`, gate licenze, script dei requisiti, `make oracle`, `make disk` e `make clean-stale` con i profili di build di regola 18 | L'app si apre con `make dev`; l'UI gira nel browser col backend finto; `make oracle` scrive i fixture; `make disk` riporta l'occupazione per cartella |
| **F1 Core** | Documento di progetto, schemi, comandi e undo/redo, pacchetto, autosave, import e copia dei file, piramidi di picchi | PRJ, HIS coperti; routing senza cicli (TRK-3); test di proprietà: ogni sequenza casuale di comandi seguita dagli undo riporta al documento iniziale |
| **F2 DSP** | Tutti i processori di FX e MAS, meter, loudness, true peak | Parità con l'oracolo (§5.7); proprietà dei processori; nessuna allocazione in `process` |
| **F3 Motore** | Grafo strutturato (gruppi annidati, ritorni multipli, sidechain), clip, automazione di tutti i parametri, render offline, streaming, uscita `cpal`, trasporto; **fetta verticale**: apri un progetto, play, muovi un fader dall'UI | ENG coperti; hash del render stabile; invarianza dalla dimensione del blocco; benchmark NFR-1 e NFR-4 |
| **F4 Analisi** | Port delle misure, misure per sezione, famiglie, valutazione | ANA coperti; parità con l'oracolo; ANA-17 |
| **F5 AI** | Contratti, prompt, decisore finto, OpenRouter, validazione e ritentativi, ciclo, revisione, lock, log e registro dei costi, master AI | Flusso completo offline da `stemwise-cli` col decisore finto; client HTTP provato contro un server locale finto |
| **F6 UI del mix** | Timeline, mixer, inspector, editing delle clip, automazione, sezioni, ritorni, cronologia | UI-2 per le schermate 1–3, 6, 7; Playwright sui flussi principali; confronto a vista col design |
| **F7 UI dell'AI** | Brief, riferimenti, auto mix, proposta in revisione con A/B, revisione a parole, pannello AI usage, snapshot | Schermate 4, 5, 12; AI-14 → AI-16; flusso completo nell'app col decisore finto |
| **F8 Master** | Mixdown, grafo di master, meter, destinazioni, master AI, A/B a pari loudness, export nei tre formati con metadata | MDN, MAS coperti; true peak dell'export entro il tetto su segnali di prova; schermate 8–10 |
| **F9 Rifinitura** | Impostazioni, errori e stati vuoti, scorciatoie, accessibilità, prestazioni dell'UI | UI-3 → UI-7, SET-1, NFR-3, NFR-5; schermate 11, 13 |
| **F10 Consegna** | Pacchetto `.app`/`.dmg`, revisione indipendente finale, README, guida di validazione | `make build` produce il pacchetto; `make reqs` senza buchi; `docs/app/VALIDATION.md` completo |
| **F11 Riverbero e compressione parallela** (P-011) | Tre algoritmi di riverbero al posto di quello a pettini, parametri nuovi nel documento con migrazione di schema, `mix` del compressore, motore e automazione, tavole di design aggiornate (schermata 7, pannello del compressore) e loro implementazione, `docs/app/dsp.md` | FX-3, FX-5, FX-9 coperti; hash del render stabile e invarianza dal blocco con i riverberi nuovi; `make bench` (NFR-1 con 4 riverberi); confronto a vista col design |
| **F12 Mix AI strutturale** (P-011) | Misura dei doppi, contratti `mix_plan` e `analysis` versione 3, validazione e applicazione della struttura, ciclo con struttura stabile, prompt e decisore finto con le tecniche, tavole di design aggiornate (schermate 3 e 4) e loro implementazione, pacchetto ricostruito, VALIDATION aggiornato | ANA-18, AI-17 → AI-21 coperti; flusso completo da `stemwise-cli` e nell'app col decisore finto, con ritorni e gruppi creati e annullati in un passo; client provato contro il server finto con lo schema v3; revisione indipendente; `make reqs` senza buchi |
| **F13 Stato del mix a colpo d'occhio** (P-013) | Tavole di design (3e–3g, 7f–7h, C7, e le tavole esistenti rigenerate con i segni), poi, **dopo l'approvazione delle tavole da parte dell'autore**: segni su intestazioni e strip, mandate per ritorno con la colonna delle etichette, collegamenti della selezione, vista Overview; nessuna modifica a core, motore e contratti | UI-8 → UI-11 coperti; nessun ridisegno della timeline legato alla testina (A-064, A-065) e 60 fps con 32 tracce (NFR-3); confronto a vista col design; `make reqs` senza buchi |
| **F14 Catena di master libera** (P-017) | Catena come lista di blocchi ripetibili con ordine libero, clipper, multibanda da due a sei bande, mix e filtro sul detector del compressore, scostamenti per sezione, schema di progetto 3 con migrazione, `master_plan` v2 con valutazione per blocco, rack e editor (tavole 15a–15j), proposta misurata applicabile voce per voce, avviso sulla destinazione troppo alta | MAS-11 → MAS-15 coperti; render uguale a ogni blocco di calcolo; progetti dello schema 2 aperti con lo stesso suono; flusso col decisore finto; `make reqs` senza buchi |
| **F15 Catena delle tracce a blocchi** (P-018) | Tavole di design per **tutte** le fasi di P-018 (schermata 16, aggiornamenti di 3, 4 e 7, componenti), poi, **dopo l'approvazione dell'autore**: catena di tracce e gruppi come lista, catena dei ritorni, filtri risonanti, dissolvenza dei bypass, latenza costante, schema di progetto 4 con migrazione, inspector a rack, segni e Overview sulla lista | FX-10 → FX-12, UI-12 coperti; progetti dello schema 3 aperti con lo stesso hash; render uguale a ogni blocco di calcolo; nessuno scatto cambiando struttura; confronto a vista col design |
| **F16 Effetti nuovi** (P-018) | I processori e gli editor di FX-13 → FX-23 (gate, de-esser, EQ dinamico, transient shaper, clipper e multibanda di traccia, caratteri della saturazione, exciter, bass enhancer, ampiezza, modulazione, tremolo, harmonizer, doubler, shimmer, ping-pong e tape), automazione dei loro parametri, `docs/app/dsp.md` | FX-13 → FX-24 coperti; `make bench` con il caso di FX-24; confronto a vista col design |
| **F17 Idee creative** (P-018) | Misure di ANA-19, `analysis` v4 e `mix_plan` v4, blocchi e scostamenti nuovi nel piano, idee con validazione e applicazione, ciclo con idee ferme, prompt e decisore finto con le tecniche nuove, proposta con Creative ideas e interruttore, pacchetto ricostruito, VALIDATION aggiornato | ANA-19, AI-25 → AI-27, UI-13 coperti; flusso completo da `stemwise-cli` e nell'app col decisore finto, con idee applicate e annullate in un passo; client provato contro il server finto con lo schema v4; revisione indipendente; `make reqs` senza buchi |
| **F18 Explain this mix** (P-019) | Tavole della schermata 17, poi, **dopo l'approvazione dell'autore**: misura dei livelli in background (ANA-20), vista Explain con mappa e testo a regole, glossario, copia del testo, scorciatoia ⌘E | ANA-20, UI-14 → UI-16 coperti; flusso Playwright sul backend finto; confronto a vista con le tavole 17a–17j |
| **F19 Tour guidato** (P-020) | Tavole della schermata 18 (e componente del fumetto), poi, **dopo l'approvazione dell'autore**: progetto dimostrativo costruito e sintetizzato dal core, sessione effimera, comandi del tour tra UI e core con i tipi generati, segno "mostrato" nei dati dell'app, tour con riflettore e fumetto e i quindici passi, menu dell'app unico con Help (UI-20) e collegamento nella Welcome, backend finto, self-test, VALIDATION | PRJ-9, UI-17 → UI-20 coperti; stesso hash a ogni costruzione del progetto dimostrativo e nessun file rimasto dopo la chiusura; nessuna chiamata di rete e nessun costo registrato; Playwright sul backend finto (primo avvio, tutti i passi avanti e indietro, salto, ripetizione, finestra minima, axe); self-test del tour nell'app vera; tempi di PRJ-9 e avvio a freddo misurati; confronto a vista con le tavole; `make reqs` senza buchi |

F11 e F12 si eseguono dopo F10, in ordine, con le stesse regole di §9 (anche dove §9 e §11 parlano di "F10"
come ultima fase). Le tavole di `design/` si aggiornano all'inizio di ciascuna fase, prima dell'interfaccia.
F13 (P-013) e F15 (P-018) sono le fasi che **si fermano a metà**: consegnate le tavole, l'implementazione aspetta
che l'autore le approvi. Le tavole di F15 coprono anche F16 e F17; approvate, F15 → F17 si eseguono di seguito.
Anche F18 (P-019) e F19 (P-020) si fermano a metà per l'approvazione delle tavole.

### 8.2 Gate comune: `make check` (in `app/`)

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test --workspace` · gate licenze ·
controllo dei tipi TypeScript · lint · `vitest` · Playwright sul backend finto · `make reqs` per le fasi chiuse · controllo del tetto di spazio (NFR-9).
Nessun commit con il gate rosso. I benchmark girano con `make bench` al gate delle fasi che li citano.

### 8.3 Tracciabilità dei requisiti

Ogni test cita gli id che verifica: in Rust con un commento di documentazione `/// req: ENG-4, AUT-4` sopra il
test, in TypeScript con `// req: UI-5` sopra il caso. `make reqs` elenca gli id di §3 senza test; ogni id
scoperto deve comparire in `docs/app/REQS-JUSTIFIED.md` con il motivo (per esempio: verificabile solo
all'ascolto).

### 8.4 Regole dei test

- Nessun test usa la rete: il client OpenRouter si prova contro un server HTTP locale finto.
- Nessun file audio nel repository: segnali sintetici con seme fisso, progetti di prova in cartelle temporanee.
- Nessun test apre il dispositivo audio reale, tranne un test di fumo marcato `#[ignore]`, eseguito a mano al
  gate di F3.
- Tolleranze sempre esplicite e motivate.

### 8.5 Cosa resta all'autore dopo l'implementazione

L'agente non può ascoltare né usare una chiave reale. `docs/app/VALIDATION.md` elenca, con passi eseguibili:
ascolto di un brano reale (click, glitch, qualità di compressore, riverbero, delay, saturazione, limiter);
auto mix e revisione con il modello reale e lettura dei costi; master contro un riferimento; firma e
notarizzazione; controprova su un brano diverso da "Confidence" (limite della PoC, M7). Con P-011 si aggiungono:
ascolto dei tre algoritmi di riverbero su voce e batteria (code metalliche, modulazione udibile); ascolto in A/B
di una proposta con struttura (ritorni dedicati, gruppo con compressione parallela, doppi aperti) contro la
stessa senza; taratura delle soglie dei doppi su un brano con chitarre doppiate davvero. Con P-018: ascolto di
ogni blocco nuovo su materiale vero (de-esser e gate soprattutto, harmonizer, doubler e shimmer per gli artefatti
d'intonazione), e idee creative proposte dal modello vero su brani di generi diversi.

## 9. Regole per gli agenti

Valgono per la sessione di implementazione e sono riportate in `CLAUDE.md`.

**Autonomia**

1. Dopo il controllo preliminare (§11), l'agente **non si ferma e non fa domande** fino al completamento di F10.
2. Un'ambiguità si risolve scegliendo l'opzione più coerente con questo documento, nell'ordine: principi (§1),
   requisiti (§3), architettura (§5), design. La scelta si registra in `docs/app/DECISIONS.md` (id `A-nnn`,
   contesto, alternative, scelta, motivo) e si prosegue.
3. Un ostacolo tecnico (una libreria che non fa ciò che serve, un test che non si riesce a rendere verde) si
   affronta con al più tre approcci diversi; poi si sceglie il ripiego che conserva il requisito, oppure si
   degrada il requisito al minimo utile, lo si registra tra le "Riserve" in `docs/app/PROGRESS.md` e si prosegue.
   Non si aggira mai un gate indebolendo un test.
4. Le uniche ragioni per fermarsi: un'azione distruttiva o irreversibile fuori dal repository, la necessità di un
   segreto, oppure un conflitto con i principi P1–P6 che nessuna alternativa risolve.

**Confini**

5. Niente fuori scope (§2, colonna destra).
6. La PoC è in sola lettura: ciò che sta sotto `poc/` non si modifica. La si esegue solo per generare l'oracolo.
7. Nuove dipendenze ammesse senza chiedere, se la licenza è tra quelle di NFR-6 e la libreria è mantenuta;
   ciascuna si annota in `docs/app/DECISIONS.md`. Nessuna dipendenza GPL/LGPL/AGPL, nemmeno di sviluppo se
   finisce nel pacchetto.
8. Segreti: l'agente non legge `.env`, non chiede né usa chiavi, non fa chiamate a modelli reali.
9. Git: lavoro sul ramo `product/v1`; commit piccoli col prefisso di fase (`F3: add automation ramps (AUT-4)`);
   un tag locale per fase chiusa (`f3`). Mai `push`, `merge` su `main`, `reset --hard`, `rebase`.
10. Lingua: codice, commenti, messaggi e interfaccia in inglese; documenti in `docs/` in italiano.

**Coordinamento e subagenti**

11. L'agente principale è l'orchestratore: possiede architettura, contratti tra moduli, integrazione, gate e
    decisioni. Organizza il lavoro come ritiene meglio.
12. **Si delega per ridurre i costi, non per principio.** Conviene delegare a un subagente su un modello più
    economico (`app-implementer`) quando il compito è insieme: delimitato (un modulo o pochi file), specificato
    da un contratto già scritto (firma, requisiti, test o fixture attesi) e verificabile da un comando. Esempi:
    il port di una misura con il suo fixture dell'oracolo; un processore DSP con le sue proprietà; un componente
    dell'interfaccia a partire dalla sua tavola; una serie di test da una tabella di casi.
13. **Non si delega** quando: il brief costerebbe quanto il lavoro; servono decisioni di architettura o un
    contesto ampio; il compito tocca file che altri stanno modificando; l'integrazione tra moduli; meno di una
    manciata di chiamate a strumenti. Per le ricerche ampie nel codice si usa il subagente di sola lettura più
    economico.
14. Lavori in parallelo solo su insiemi di file disgiunti; due agenti non scrivono mai gli stessi file. Ogni
    brief nomina file, contratto, comando di verifica e ciò che non va toccato.
15. Il risultato di un subagente non è una prova: l'orchestratore riesegue il comando di verifica e legge il
    diff prima di integrare. Un subagente che fallisce due volte lo stesso compito non si rilancia: il lavoro
    torna all'orchestratore.
16. Una **revisione indipendente** (`app-reviewer`, contesto nuovo, sola lettura) si fa alla chiusura di F3, F5,
    F8 e F10, non a ogni fase: serve alla qualità, e il suo costo si giustifica solo su ciò che è rischioso
    (tempo reale, contratti AI, master, consegna). I rilievi fondati si correggono prima di chiudere la fase.
17. A ogni chiusura di fase l'orchestratore aggiorna `docs/app/PROGRESS.md` (stato, gate, riserve) in modo che
    il lavoro sia riprendibile da una sessione nuova senza altra memoria.

**Spazio su disco**

18. **Prevenzione, impostata in F0.** Una sola cartella di build per tutto il workspace (`app/target`), usata
    anche dai subagenti e dalla build di Tauri: nessun `CARGO_TARGET_DIR` alternativo, nessuna copia del
    repository o worktree con una propria `target`. Profili di sviluppo e di test con informazioni di debug
    ridotte (`debug = "line-tables-only"`) e senza compilazione incrementale per le dipendenze; un solo
    `node_modules` (store di pnpm fuori dal repository); i test scrivono solo in cartelle temporanee che
    eliminano alla fine, anche quando falliscono; nessun file audio renderizzato resta su disco dopo un test.
19. **Misura.** `make disk` riporta l'occupazione del repository e delle sue cartelle principali (`app/target`
    per profilo, `node_modules`, pacchetti prodotti, rapporti dei test, cartelle temporanee dei test). Si esegue
    a ogni chiusura di fase, dopo ogni `make build` o `make bench`, e comunque almeno ogni due ore di lavoro; il
    valore si annota in `docs/app/PROGRESS.md`.
20. **Pulizia.** Soglia di attenzione **10 GB**: `make clean-stale` elimina ciò che non serve più (cache
    incrementali, artefatti di profili non in uso, build di versioni superate delle dipendenze, pacchetti
    `.app`/`.dmg` precedenti, rapporti e tracce di Playwright, cartelle temporanee rimaste, worktree e rami
    locali dei subagenti già integrati). Tetto **20 GB** (NFR-9): `cargo clean` completo e ricostruzione del
    solo profilo in uso; `make check` fallisce sopra il tetto. La pulizia tocca **solo file ignorati da git
    dentro il repository** e le cartelle temporanee create dai test: mai file tracciati, mai `local/`, mai
    nulla fuori dal repository. Queste eliminazioni sono pre-autorizzate e non richiedono conferma.

---

## 10. Prompt della sessione di design

Da incollare in una nuova sessione di Claude Desktop (canvas), **allegando questo file** (`FOUNDATION.md`).

````text
Sei il designer di prodotto di Stemwise, una DAW desktop per mixare e masterizzare brani già registrati con
l'assistenza di un modello AI. Il documento allegato (FOUNDATION.md) è la specifica: leggi per intero le sezioni
1, 2, 3 e 6 prima di disegnare. La sezione 6 è l'inventario vincolante di schermate, stati e componenti; la
sezione 7 descrive cosa devi consegnare.

Il lavoro ha due tempi e una sola pausa.

TEMPO 1 — Quattro direzioni visive
Proponi 4 direzioni visive e creative nettamente diverse tra loro (non quattro varianti di colore della stessa
idea). Per ciascuna mostra sul canvas:
- un nome e tre righe sul concetto e su quale utente fa sentire a casa;
- la schermata "Mix workspace" completa a 1440×900 con contenuto realistico (12 tracce con ruoli veri, forme
  d'onda, sezioni del brano, una corsia di automazione aperta, mixer, inspector con EQ, pannello AI con una
  motivazione);
- un estratto del "Master workspace" che mostri come la direzione distingue le due fasi;
- palette, tipografia e i componenti chiave (fader, knob, meter, clip, pulsante primario).
Poi fermati e chiedimi quale direzione scelgo. È l'unica domanda che puoi farmi in tutta la sessione.

TEMPO 2 — Tutto il resto, senza interruzioni
Dopo la mia scelta, disegna tutte le schermate, gli stati e i componenti della sezione 6, nell'ordine in cui
sono elencati, senza fermarti e senza chiedermi altro. Se un punto è ambiguo, decidi tu nel modo più coerente
con la direzione scelta e con la specifica, e annota la decisione nell'indice finale. Non considerare finito il
lavoro finché ogni voce della sezione 6 non ha la sua tavola: alla fine verifica l'elenco voce per voce.

Vincoli di prodotto
- Mix e Master sono due fasi distinte: devono riconoscersi a colpo d'occhio (colore d'accento, disposizione,
  densità), con un selettore di fase sempre visibile. Il passaggio dal mix al master è un momento esplicito.
- L'AI è un assistente, non un pilota automatico: le sue proposte si leggono, si ascoltano in A/B, si accettano
  in tutto o in parte, e ogni scelta ha una motivazione visibile. Niente estetica da chatbot generico: le
  richieste a parole vivono dentro lo strumento.
- Chi usa l'app non è un fonico: la densità deve restare leggibile, i controlli avanzati non devono intimidire.
  Ma deve sembrare uno strumento audio serio, non un giocattolo.
- Si lavora per ore, spesso in ambienti poco illuminati: il contrasto dev'essere riposante, i meter e lo stato
  di riproduzione leggibili con la coda dell'occhio. Testo e stati con contrasto almeno AA.
- Ogni controllo ha gli stati: normale, hover, attivo, focus da tastiera, disabilitato e, dove ha senso,
  automatizzato e bloccato dall'utente (lock per l'AI).
- I colori delle tracce sono una palette di almeno 12 tinte distinguibili che funziona su clip, strip e
  intestazioni.
- Interfaccia in inglese. Nessuna imitazione dell'aspetto di DAW esistenti: l'identità dev'essere propria.

Vincoli di implementazione (l'interfaccia sarà React + CSS dentro una webview; timeline, forme d'onda, meter e
curve saranno disegnati su canvas)
- Finestra minima 1280×720, riferimento 1440×900, fluida fino a 4K: indica cosa si ridimensiona, cosa si
  richiude e in che ordine.
- Solo font con licenza che ne consente l'inclusione in un'app distribuita (OFL o equivalente), al massimo due
  famiglie, con cifre tabulari per i valori numerici. Icone da un set con licenza MIT/ISC oppure disegnate da te
  come SVG.
- Effetti realizzabili in CSS senza costi di rendering continui: niente sfocature animate o ombre pesanti su
  elementi che si muovono a 60 fps.
- Tutte le misure derivano da token: nessun valore isolato nelle tavole.

Consegna (sezione 7 del documento)
- tokens.css: tutti i token come variabili CSS (colori, tipografia, spaziatura, raggi, ombre, durate e curve di
  animazione, palette delle tracce, accenti delle due fasi).
- Una tavola per ogni schermata e stato della sezione 6, in HTML/CSS autosufficiente che usa quei token.
- Il foglio dei componenti con tutti gli stati.
- README.md: direzione scelta, principi, indice delle tavole con il nome di ogni file, comportamenti non
  deducibili dalle tavole (resize, animazioni, trascinamenti, densità), decisioni prese in autonomia.
Organizza i file come design/README.md, design/tokens.css, design/screens/, design/components/. Se hai accesso
alla cartella del progetto, scrivili lì; altrimenti rendili scaricabili con quei nomi.
````

## 11. Prompt della sessione di implementazione

Da incollare in una nuova sessione di Claude Code aperta su questo repository, **dopo** che `design/` è stata
popolata. `.claude/settings.json` autorizza già i comandi di sviluppo (`make`, `cargo`, `pnpm`, `node`, `uv run`,
i comandi git di lavoro); per i casi non coperti conviene avviarla con una modalità di permessi che non chieda
conferme a ogni comando.

````text
Devi implementare Stemwise dall'inizio alla fine, in autonomia, seguendo docs/FOUNDATION.md, che è la fonte di
verità del prodotto. Leggilo per intero prima di qualunque altra cosa, poi leggi CLAUDE.md, docs/app/PROGRESS.md
e docs/app/DECISIONS.md, e design/README.md con design/tokens.css.

CONTROLLO PRELIMINARE — l'unico momento in cui puoi farmi domande
Prima di scrivere codice verifica e riferiscimi in un solo messaggio:
1. che design/README.md e design/tokens.css esistano e che ogni schermata della sezione 6 di FOUNDATION.md abbia
   la sua tavola (elenca quelle mancanti);
2. che gli strumenti necessari siano installati (rustc, cargo, node, pnpm, Xcode command line tools, uv per
   l'oracolo della PoC) e ciò che eventualmente manca;
3. le contraddizioni o i vuoti che hai trovato tra FOUNDATION.md e il design e che non puoi risolvere con le
   regole della sezione 9: per ciascuno proponi la tua scelta.
Se non c'è nulla che blocchi, dichiaralo e parti subito senza aspettare risposta. Se qualcosa blocca davvero
(design assente, strumento mancante che non puoi installare), fermati qui e chiedi: è l'unica pausa ammessa.

ESECUZIONE — senza interruzioni fino al termine di F10
- Lavora sul ramo product/v1. Esegui le fasi F0 → F10 della sezione 8.1 nell'ordine, una dopo l'altra, senza
  chiedermi conferme, approvazioni o chiarimenti tra una fase e l'altra. Una fase è chiusa solo quando il suo
  gate è verde.
- Applica le regole della sezione 9. In particolare: le ambiguità le risolvi tu e le registri in
  docs/app/DECISIONS.md; gli ostacoli li affronti con approcci alternativi e, se serve, con un ripiego
  registrato tra le riserve; non indebolisci mai un test per far passare un gate.
- Coordina il lavoro come ritieni meglio. Delega ai subagenti solo quando riduce i costi (regole 11–17): compiti
  delimitati, con contratto e verifica già scritti, al subagente app-implementer; ricerche ampie al subagente di
  sola lettura; revisione indipendente con app-reviewer alla chiusura di F3, F5, F8 e F10. Verifica sempre tu il
  risultato di un subagente prima di integrarlo.
- A ogni fase chiusa: aggiorna docs/app/PROGRESS.md, fai il commit, metti il tag locale della fase, e prosegui
  con la successiva nello stesso turno. Lo stato deve stare nei file, non nella conversazione: se il contesto
  viene compattato, rileggi FOUNDATION.md, PROGRESS.md e DECISIONS.md e riprendi dal primo passo non concluso.
- Tieni sotto controllo lo spazio su disco (regole 18–20): una sola cartella di build, make disk a ogni fase e
  dopo ogni build o benchmark, make clean-stale sopra i 10 GB, mai oltre i 20 GB.
- Non usare la rete se non per scaricare dipendenze. Non leggere .env, non usare chiavi, non chiamare modelli
  reali: l'AI si sviluppa e si prova con il decisore finto e con un server HTTP locale finto.
- Non fare push, non fare merge su main, non modificare la PoC.

VERIFICA DI CIÒ CHE COSTRUISCI
- Motore, DSP, analisi e AI: test automatici su segnali sintetici, parità con l'oracolo della PoC, flussi
  completi da stemwise-cli.
- Interfaccia: test Playwright sul backend finto e confronto a vista di ogni schermata con la sua tavola in
  design/screens (apri l'UI nel browser, guarda il risultato, correggi le differenze). Avvia anche l'app Tauri
  vera e verifica che i flussi principali funzionino con il motore reale.
- Non dichiarare finita una cosa che non hai eseguito. Ciò che non puoi verificare (l'ascolto, il modello reale,
  la firma) va elencato in docs/app/VALIDATION.md con i passi che dovrò eseguire io.

TERMINE
Hai finito quando F10 è chiusa: make check verde, make reqs senza buchi non giustificati, pacchetto .app/.dmg
prodotto, revisione finale fatta e rilievi corretti, PROGRESS.md e VALIDATION.md completi. Solo allora scrivimi
un rapporto finale: cosa funziona e come l'hai verificato, le riserve, le decisioni prese in autonomia più
rilevanti, cosa devo validare io e in che ordine.
````

## 12. Rischi

| Rischio | Effetto | Contromisura |
| --- | --- | --- |
| DSP proprio di qualità inferiore alla PoC (compressore, riverbero, limiter) | Mix e master meno rifiniti | Proprietà verificate dai test, algoritmi noti e documentati in `docs/app/dsp.md`, ascolto finale dell'autore come primo punto di `VALIDATION.md` |
| Sessione autonoma molto lunga: deriva o perdita di contesto | Lavoro incoerente o ripetuto | Stato nei file (regola 17), gate per fase, tag per fase, revisione indipendente nei punti rischiosi |
| Glitch audio non rilevabili dai test | Click o dropout all'ascolto | Stesso codice per riproduzione e render, test di allocazione, benchmark, fetta verticale già in F3 |
| Fedeltà al design non verificabile al pixel | Interfaccia approssimativa | Token condivisi, backend finto nel browser, confronto a vista schermata per schermata |
| Il prompt con sezioni e famiglie supera il budget di token o confonde il modello | Costo o piani peggiori | Tetto ANA-13 verificato da un test; misura dei costi reali nella validazione dell'autore |
| La PoC è stata convalidata su un solo brano | Generalità non dimostrata | Controprova su un brano diverso nella validazione dell'autore (§8.5) |
| Gli artefatti di build e di test fanno crescere il repository senza limite | Disco saturo, sessione bloccata | Regole 18–20, `make disk` e `make clean-stale` da F0, tetto verificato da `make check` |
| Il routing strutturato e l'automazione estesa allargano il motore | F3 più lunga e più fragile | Grafo aciclico calcolato fuori dal thread audio, test di proprietà sul routing, revisione indipendente a F3 |
| Prestazioni della webview con molte tracce | Scatti durante la riproduzione | Canvas, piramidi di picchi, canale binario; NFR-3 misurato in F9, WebGL come ripiego |
| Il riverbero nuovo rispetta le proprietà ma non convince all'ascolto | Più ritorni senza un guadagno reale | Algoritmi pubblicati e noti (Dattorro, FDN), proprietà di FX-9, ascolto dell'autore prima di chiudere il giudizio su F12 |
| Il modello abusa della struttura (ritorni e gruppi inutili) o il ciclo non sa giudicarla: la distanza spettrale non la vede | Mix più complicati, non migliori | Tetti per proposta, struttura decisa una volta sola (AI-19), elementi inutili scartati dalla validazione, interruttore AI-18, righe proprie nella proposta con A/B |
| Falsi doppi: due parti diverse con lo stesso ruolo scambiate per una coppia | Pan sbagliati | Più misure insieme in ANA-18, soglie prudenti e tarate dall'autore; resta una proposta, visibile e scartabile per traccia |
