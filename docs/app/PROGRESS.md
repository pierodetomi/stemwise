# Avanzamento del prodotto

Specifica: [FOUNDATION.md](../FOUNDATION.md). Decisioni: [DECISIONS.md](DECISIONS.md).
Legenda: ⬜ da iniziare · 🟦 in corso · ✅ chiusa (gate verde) · ⚠️ chiusa con riserve

| Passo | Stato | Note |
| --- | --- | --- |
| Documento fondativo e regole per gli agenti | ✅ | 2026-11-10 |
| Sessione di design (FOUNDATION §10) | ✅ | 2026-11-10 · direzione "E · Atelier" · consegna in `design/` (vedi `design/README.md`) |
| F0 Fondamenta | ✅ | 2026-11-11 · workspace Cargo, app Tauri che si apre (`make dev`), UI sul backend finto, token e componenti importati da `design/`, gate licenze, `make reqs`/`oracle`/`disk`/`clean-stale` · disco 5,7 GB (di cui `local/` 4,0 GB) |
| F1 Core | ✅ | 2026-11-11 · documento, comandi con inverso, cronologie mix/master, gesti, routing aciclico, editing di clip/sezioni/tracce, automazione, pacchetto con autosave e recupero, import WAV/RF64/AIFF con conversione, piramidi, snapshot, schemi e tipi TS generati · 40 test (proprietà su comandi e routing) · disco 7,1 GB |
| F2 DSP | ✅ | 2026-11-11 · biquad, pan, compressore collegato con sidechain, saturazione 4x, riverbero, delay, limiter true peak, dither, match EQ, ampiezza, meter, loudness (offline e in tempo reale), true peak · parità con l'oracolo (biquad, pan, LUFS, LRA, true peak) · nessuna allocazione in `process` · `docs/app/dsp.md` · disco 7,4 GB |
| F3 Motore | ✅ | 2026-11-11 · grafo per livelli con gruppi annidati, ritorni, sidechain compensata, automazione su griglia di 32, render offline deterministico (stesso hash con blocchi 64/1000/4096), streaming con protocollo di seek senza lock, uscita cpal con ricampionamento, thread ausiliari (A-024), fetta verticale provata nell'app vera · NFR-1 p99 14%, NFR-4 36× · revisione indipendente: 2 bloccanti e 9 rilievi corretti · disco ≈ 9 GB |
| F4 Analisi | ✅ | 2026-11-11 · misure ANA-1…12 portate dalla PoC in parità con l'oracolo (stem, mix, mascheramento, quote, riferimenti, target, onset di librosa), misure per sezione, famiglie, documento `analysis` v2 con id neutri e sotto il budget di token, cache legata alle clip · ANA-17: 16 stem da 4 min in 8,6 s |
| F5 AI | ⚠️ | 2026-11-11 · contratti `mix_plan` v2 e `master_plan` v1 con schema per progetto e tutti i limiti, decisore finto, prompt, client OpenRouter provato contro un server finto (ripiego JSON mode, nuovi tentativi con la risposta sbagliata, annullamento), ciclo di verifica con regole di stop e piano migliore, revisione dallo stato attuale (corsie per sezione e valori manuali conservati), lock, ambito, log e costi anche di chiamate fallite o annullate, registro tra progetti, master AI, CLI · revisione indipendente: 3 bloccanti e 14 rilievi, corretti (A-031, A-034…A-037) · riserve: strict schema e costi da verificare col provider vero, misura del master AI in F8 (A-033) |
| F6 UI del mix | ✅ | 2026-11-11 · schermate 1, 2, 3, 6, 7: welcome, nuovo progetto con ruoli e scelta sulla frequenza, workspace con timeline (righello, loop, sezioni, clip con forme d'onda, trascinamento con snap, rifilatura, fade, taglio, selezione a riquadro, menu contestuali, gruppi annidati ripiegabili, corsie di automazione con penna, intervalli, copia e incolla), mixer, inspector (catena con bypass, editor EQ con spettro dal vivo, compressore con curva e sidechain, saturazione, mandate, uscita), ritorni, cronologia · contratto `Edit` nel core (A-038), spettro dal motore (A-039) · 11 test Playwright sui flussi, 8 catture per il confronto a vista, self-test nell'app vera con motore reale |
| F7 UI dell'AI | ✅ | 2026-11-11 · schermate 4, 5, 12 (e 3c): richiesta con ambito e stima del costo, tetto di spesa con conferma, avanzamento per fase e passata con annullamento, proposta in revisione con A/B, applica tutto o le tracce scelte o scarta, AI usage con riepilogo per tipo e modello, filtri, dettaglio leggibile e JSON grezzo, export CSV/JSON, totale su tutti i progetti, snapshot con A/B, brief e riferimenti, chiave nel portachiavi o demo offline · chiamate in un thread della sessione Tauri (A-041) · 7 test Playwright sull'AI, flusso completo nell'app vera col decisore finto (self-test) |
| F8 Master | ✅ | 2026-11-11 · mixdown con trim a −3 dBTP e hash del mix, grafo di master con catena MAS-1 sul bus e parametri dal vivo, loudness della destinazione calcolata in background, A/B a pari loudness, meter e spettro col profilo dei riferimenti, master AI misurato prima dell'applicazione e applicabile per modulo, export WAV/FLAC/AAC con encoder FLAC proprio, AAC di sistema e tag (`stemwise-export`), tetto garantito sul file decodificato · schermate 8–10 · self-test nell'app vera (anche A/B su un riferimento AAC a 44,1 kHz) · revisione indipendente: 3 bloccanti e 9 rilievi corretti (A-043: catena senza gradini, tetto sui file riletti, AAC ≤ 48 kHz, riferimenti convertiti, A/B senza clip, mixdown fuori dall'undo, lavori legati al progetto, ISRC e limiti nel core) |
| F9 Rifinitura | ✅ | 2026-11-11 · tabella unica delle scorciatoie (UI-3), Impostazioni (11: Audio, AI, Aspetto, Scorciatoie, SET-1), stati di errore e vuoti (13a–13f: file mancanti con ricollegamento, dispositivo scollegato, rete assente, progetto vuoto, forme d'onda ricostruite all'apertura, UI-6), accessibilità con axe-core e tastiera (UI-7), 60 fps con 32 tracce, import 16×4 min in 1,7 s, avvio a freddo 0,5 s in sviluppo (NFR-3), picco di memoria 1,0 GB con 32×5 min dopo analisi in streaming (NFR-5) · A-044, A-045 · 43 test Playwright, self-test nell'app vera · disco 11,3 GB dopo `clean-stale` (di cui `local/` 4,0 GB) |
| F10 Consegna | ✅ | 2026-11-11 · `make build` produce `Stemwise.app` e `Stemwise_0.1.0_aarch64.dmg` (A-046), self-test completo nell'app impacchettata con avvio a freddo di 969 ms, NFR-7 e NFR-8 verificati da test sul codice, README del prodotto, VALIDATION completa con ordine consigliato · revisione indipendente finale: 2 bloccanti e 6 rilievi corretti (A-047) · `make reqs`: nessun buco · disco 13,7 GB dopo `clean-stale` (di cui `local/` 4,0 GB, `target/debug` 7,7 GB) |
| F11 Riverbero e compressione parallela | ✅ | 2026-11-11 · P-011 · riverbero in tre algoritmi (plate di Dattorro, room e hall su rete di 16 ritardi) al posto di quello a pettini, con decadimento in secondi, dimensione, damping, modulazione deterministica e passa-basso; mix (dry/wet) del compressore di tracce e gruppi; schema di progetto 2 con migrazione; contratto `mix_plan` v3 per riverbero e mix; tavole 7a e 7e e loro implementazione · FX-9: decadimento entro il 5%, coda senza periodo ripetuto (0,04–0,10 contro 0,14–0,20 del vecchio), stesso hash a ogni blocco · NFR-1 p99 17,9% con 4 riverberi in più, NFR-4 31× · A-058 · disco 16,7 GB dopo `clean-stale` (di cui `local/` 4,0 GB, `target/debug` 9,6 GB) |
| F12 Mix AI strutturale | ✅ | 2026-11-11 · P-011 · il piano crea ritorni e gruppi (`structure`, segnaposto `n1`–`n6`), resi reali una volta nella copia di lavoro e stabili tra le iterazioni, ricreati all'applicazione con una spunta ciascuno; doppi veri misurati per coppie dello stesso ruolo; tecniche nel prompt e nel decisore finto (doppi ai lati, plate e slapback per la voce, throw di delay, bus della batteria in parallelo); spunta "Allow structural changes"; schede degli elementi nuovi nella proposta (tavole 3a, 4a, 4b) · 5 test di flusso, flusso completo da `stemwise-cli`, 60 test Playwright, self-test nell'app impacchettata (46 passi, avvio a freddo 1,4 s) · revisione indipendente: 2 bloccanti e 4 rilievi corretti (A-060) · `make reqs`: nessun buco · A-059 · disco 17,48 GB dopo `clean-stale` |
| F13 Stato del mix a colpo d'occhio | ✅ | 2026-11-13 · P-013 · design approvato dall'autore (A-066), poi implementato: sigle `HP LP EQ CMP SAT` in tre stati su intestazioni e strip (`CMP` col colore della sidechain e le barrette del parallelo), punti delle mandate e segno di automazione, colonna delle etichette con una mandata per ritorno e bus fissato a destra, elenco di chi manda nella corsia del ritorno, collegamenti della selezione, vista Overview con bypass a un clic; token resi effettivi (intestazione 344 px, strip 40, mixer 296) · UI-8 → UI-11 coperti (8 test Vitest, 6 Playwright, catture 3e, 3f, 3g, 7f–7h) · 60 fps con 32 tracce anche a mixer aperto con 4 ritorni (NFR-3) · A-067 · disco 18,07 GB |
| F14 Catena di master libera | ✅ | 2026-11-14 · P-017 · tavole 15a–15j approvate dall'autore, poi implementate: catena come lista di blocchi ripetibili in ordine libero (fissi input, limiter e dither), clipper, multibanda da due a sei bande, mix e filtro sul detector del compressore, scostamenti per sezione; schema di progetto 3 con migrazione che conserva il suono; processore con latenza costante e cambi di struttura senza scatti; `master_plan` v2 con valutazione per blocco, proposta applicabile voce per voce; rack accanto all'editor, avviso sulla destinazione troppo alta; ruoli `piano`, `woodwinds`, `choir` · MAS-11 → MAS-15 coperti (motore, core, AI, 22 test Playwright del Master, 86 in tutto) · non eseguiti: self-test nell'app impacchettata e `make build` · A-082 · disco 11,3 GB dopo un `cargo clean` |
| F15 Catena delle tracce a blocchi | ✅ | 2026-11-14 · P-018 · tavole 16a–16u, 4c, 4d approvate dall'autore (A-086), poi implementate: catena di tracce e gruppi come lista di blocchi in ordine libero (al più 12, due per tipo), catena dei ritorni (al più 6) con sidechain per il ducking, filtri risonanti, bypass con dissolvenza di 10 ms, latenza costante per classe, mono che diventa stereo al primo blocco stereo; schema di progetto 4 con migrazione a **hash identico** (provato su progetti scritti dalla build dello schema 3) e mixdown che resta aggiornato; rack nell'inspector con menu per famiglia, riordino, duplica, reset; segni (quattro e `+n`) e Overview con una riga per tipo · corretto lo scatto dopo uno scambio di grafo (la dissolvenza ora parte dopo la latenza) · FX-10 → FX-12, UI-12 coperti · A-087 · disco 11,9 GB |
| F16 Effetti nuovi | ✅ | 2026-11-14 · P-018 · gate ed expander, de-esser con ascolto del solo segnale rilevato (stato del solo motore), EQ dinamico con sidechain per banda, transient shaper, clipper e multibanda di traccia, caratteri della saturazione (`soft` invariato), exciter, bass enhancer, ampiezza, modulazione (chorus, flanger, phaser) e tremolo/auto-pan con fase legata alla posizione nel brano, harmonizer (latenza costante compensata) e doubler, riverbero `shimmer`, delay ping-pong e tape; un editor per blocco (tavole 16d–16s), automazione di ogni parametro · FX-13 → FX-24 coperti · benchmark FX-24: p99 27% (soglia 50%), NFR-1 p99 12,8%, NFR-4 30× · hash dello schema 3 invariato · processori scritti da sei subagenti e integrati (A-088) · disco 10,0 GB dopo `clean-stale` |
| F17 Idee creative | ✅ | 2026-11-14 · P-018 · misure di ANA-19 (sibilanza, cresta, transienti, fondo) e `analysis` v4 con i blocchi non classici, `mix_plan` v4 con blocchi per traccia, gruppo e ritorno, risonanze, caratteri, scostamenti dei filtri in ottave e blocchi accesi per sezione; fino a tre idee decise alla prima iterazione, tenute fuori dalla valutazione e date come contesto alle successive, applicate solo se spuntate nello stesso passo; decisore finto e prompt con le tecniche nuove; sotto-sezione Creative ideas con spunta e ascolto della sola idea (tavole 4c, 4d), righe "new block" nel diff, interruttore "Allow creative ideas", nome dell'idea nella nota della traccia; `stemwise-cli automix --ideas` · ANA-19, AI-25 → AI-27, UI-13 coperti (`ideas_flow`, client contro il server finto con lo schema v4, flusso dalla CLI, Playwright con catture 4c e 4d) · pacchetto ricostruito, self-test nell'app impacchettata (49 passi, idee ascoltate e applicate, avvio a freddo 1,2 s) · revisione indipendente: nessun bloccante, 7 difetti corretti · A-089 · disco 12,75 GB dopo `clean-stale` |
| F18 Explain this mix | ⚠️ | 2026-11-15 · P-019 · tavole 17a–17j approvate dall'autore (A-092), poi implementate: vista `Explain` nel Mix (⌘E) con la mappa del segnale (tracce per famiglia → gruppi → bus, mandate, sidechain, gruppi ripiegabili, striscia delle sezioni) e lo stesso mix in parole, per tutto il mix, per elemento e per sezione; testo a regole, glossario, copia come testo; **nessun modello**; livelli per famiglia e per sezione misurati dal core in background (`mix_levels`) · ANA-20, UI-14 → UI-16 coperti (1 test Rust, 15 Vitest, 6 Playwright; 103 in tutto) · riserve: non provata nell'app Tauri né nel pacchetto · A-093 · disco 12,28 GB |
| F19 Tour guidato | ✅ | 2026-11-15 · P-020, issue #5 · tavole 18a–18w e C8 approvate dall'autore (A-095), un solo menu dell'app con Help (UI-20, A-096), poi implementate: brano dimostrativo costruito e sintetizzato dal core (`stemwise_core::demo`, 12 stem, 68 s, tre sezioni, stesso documento e stesso hash a ogni costruzione), sessione effimera nella cache dell'app (niente recenti, autosave, salvataggio, export; eliminata alla fine e all'avvio), proposta del decisore finto in background anche con una chiave e fuori dal registro dei costi, segno "mostrato" nei dati dell'app, tour con riflettore e fumetto e i quindici passi, collegamento nella Welcome, menu di sistema ridotto e ⌘N/⌘O nell'app · PRJ-9, UI-17 → UI-20 coperti (7 test Rust, 6 Vitest, 9 Playwright sul tour; 113 Playwright in tutto) · tempi: brano aperto e suonabile in 1,1 s (soglia 3 s), proposta del decisore finto 36 s in background, mixdown e master 5 s, avvio a freddo 0,6–1,6 s · pacchetto ricostruito, self-test nell'app impacchettata 59/59 · corretti tre aggiornamenti dello store durante il rendering (rack di traccia e del Master, riferimenti del nuovo progetto, A-098) · A-097 · disco 10,5 GB dopo `clean-stale` |

Fasi F0 → F19 chiuse. Repository pubblico dal 2026-11-16 (A-101). Windows è il secondo sistema (A-104), in `main` dal
2026-11-17 con la 0.1.2, i cui pacchetti li costruisce la CI (A-105). Prossimo passo: la validazione dell'autore.

## Dove riprendere

- Ultimo passo concluso: **F19** (tag `f19`): tour guidato (P-020) e un solo menu dell'app (UI-20). Da provare
  nell'app vera su dati puliti: VALIDATION 57–61. Prima, F18 (tag `f18`): Explain this mix (P-019), da provare su un
  brano vero (VALIDATION 55–56). Prima ancora, F17 (tag `f17`): P-018, da provare all'ascolto e col modello vero (VALIDATION 47–54).
- Dopo F10, dalla validazione dell'autore: nuovo progetto con i file scelti prima che esista (A-048); otto difetti
  della prova manuale corretti (A-049); master pronto in 0,4 s su un progetto riaperto e in 8 s dopo un bounce,
  con 13 tracce reali a 96 kHz, invece di 40 s e 29 s (A-050); overlay di preparazione del master disegnato
  (schermate 9d–9k) e implementato (A-051) · disco 15,0 GB dopo `clean-stale`.
- Poi: portachiavi letto una volta (A-052), salvataggio dall'interfaccia (A-053), scrub della testina (A-054), ricerca
  della loudness che arriva sulle destinazioni forti (A-055), ascolto al livello reale con la pari loudness a richiesta
  e avviso sulla destinazione lontana dai riferimenti, disegnati (tavole 9l–9u, 10d–10e) e implementati (A-056).
- Dopo F12, dalla prova dell'autore: una nuova destinazione in Master aspetta sotto un overlay finché il livello non
  è al suo posto, invece della sola etichetta `Measuring…` (tavola 9v, A-061).
- Dalla prova dell'autore su 30 stem: una risposta troncata dal provider si ritenta in JSON mode, la richiesta porta
  un tetto di output esplicito e una chiamata fallita salva richiesta e risposta complete (A-062). La causa del guasto
  osservato resta da accertare col provider vero.
- Dall'analisi dei mix proposti su Balance (P-012, A-063): il modello riceve i livelli per famiglia e per ritorno
  (AI-22), il ciclo fa almeno due iterazioni e tiene l'ultimo piano valido, il prompt dà rapporti di bilanciamento
  e soglie di udibilità degli effetti, una revisione non ha limite di passo, le sezioni oltre i 120 s hanno i loro
  tempi veri, richiesta e auto mix partono dallo stesso pulsante.
- Scatti in riproduzione su un progetto grande: le forme d'onda non si ricalcolano più a ogni aggiornamento della
  testina (A-064); la timeline non si ridisegna più mentre la testina avanza e le mandate hanno una sezione propria
  nell'inspector (A-065).
- Dalla prova dell'autore (P-013, A-066): lo stato delle tracce si leggeva solo nell'inspector. Disegnati i segni su
  intestazioni e strip, le mandate nominate, i collegamenti della selezione e la vista Overview del mixer. Le tavole
  da guardare: `design/screens/03-mix-workspace.html` (3a, 3e, 3f, 3g), `design/screens/07-returns-routing.html`
  (7f, 7g, 7h), `design/components/index.html` (C7). Approvate il 2026-11-13 e implementate in F13 (A-067); da
  provare su un progetto vero (VALIDATION 33–34).
- Struttura del repository (issue #3, A-068): la PoC è confinata in `poc/` con codice, documenti (`poc/docs/`) e
  ambiente Python propri (`poc/.venv`); alla radice restano `app/`, `design/`, `docs/` (solo prodotto) e un README che
  fa da indice. `make oracle` rigenera fixture identici. Il sito di prodotto (issue #4) avrà la sua `web/`.
- Pulsante della richiesta fuori dal pannello (issue #6): con una traccia selezionata dal nome lungo il selettore
  dell'ambito spingeva `Revise mix` oltre il bordo dell'assistente. Ora il nome si tronca con i puntini (per intero nel
  tooltip) e il pulsante non si sposta; test Playwright di regressione.
- Pannelli a richiesta (issue #7, A-069): tre interruttori nella barra in alto del Mix (mixer, inspector, assistente),
  indipendenti, sullo stesso stato di ⌘M, ⌘I, ⌘J e delle strisce di riapertura. Non sono nelle tavole del design.
- Proposte che restano (issue #2, P-014, AI-23, A-070): ogni proposta ricevuta è conservata nel progetto; la scheda
  `Proposals` dell'assistente, in Mix e in Master, le elenca e le rimette in revisione sullo stato attuale, con conferma
  prima di applicare. Le proposte ricevute prima di questa modifica restano in elenco ma non si riaprono. Da provare
  nell'app vera (VALIDATION 35).
- Configurazione guidata dell'assistente (P-015, AI-24, A-071): un dialogo in tre passi collega il conto OpenRouter
  dal browser (OAuth con PKCE, ritorno su `localhost`), guida al credito e tiene la chiave incollata come via
  secondaria; credito esaurito, limite della chiave e chiave non più valida hanno il loro stato. Il primo avvio (1b)
  ora esiste. Provato contro il server finto; col provider vero in VALIDATION 36–37.
- Dove sta la chiave (A-072, scelta dell'autore): nel portachiavi solo se la build è firmata con un certificato Apple,
  altrimenti in un file privato dell'app. Nessuna richiesta di password in nessun caso. Con una build non firmata
  l'assistente va ricollegato una volta (VALIDATION 38). · disco 18,93 GB dopo `clean-stale` (di cui `target/debug` 11,9 GB,
  `target/release` 2,3 GB, `local/` 4,0 GB): vicino al tetto, il prossimo passo parte da un `cargo clean`.
- Strip del mixer più larghe (A-073, scelta dell'autore tra tre catture): 64 px, fader e sigle più grandi nelle sole
  strip, mixer alto 304 px.
- Colonna delle etichette del mixer larga uguale nelle due viste (A-074): le strip non si spostano più passando da
  Faders a Overview e i nomi dei ritorni non sono tagliati.
- Logo (A-075, A-076): quattro direzioni disegnate da un agente esterno (`design/brand/directions/`), scelta la **D ·
  Due stanze**, rifinita il 2026-11-14 restando fedele alla proposta: le due traiettorie sono il Mix, la freccia
  diventa la stanza piena del Master. Consegna in `design/brand/final/` (`presentation.html`). Adottato nell'app
  (A-078): icona, marchio nelle barre con la stanza della fase accesa, schermata iniziale, pagina di collegamento,
  tavole; da vedere nel pacchetto (VALIDATION 40). Indice in `design/brand/README.md`.
- Master AI misurato (issue #1, P-016, A-077): il master AI decideva alla cieca in una chiamata, su misure che non
  dicevano quanto i picchi stanno sopra il corpo del mix, e sulle destinazioni forti lasciava tutto al limiter. Ora
  riceve distanza picco-loudness, riduzione richiesta dalla destinazione e il master corrente misurato; ogni piano è
  renderizzato e misurato (lavoro di limiter e compressore, punch perso) e la decisione successiva lo corregge, fino a
  tre piani. Prompt con la tecnica; la destinazione cambia solo se chiesto a parole. Sul mixdown vero di Balance a
  −9,6 LUFS il limiter da solo lavora oltre 3 dB per il 21% del tempo, con gli stadi esistenti il 5–10%. Da provare
  col modello vero (VALIDATION 39). **Aperto**: clipper, compressore parallelo e filtro sul detector, ordine della
  catena, master per sezioni, valutazione mostrata nella proposta: da disegnare (toccano MAS-1 e il pannello). · disco 19,46 GB dopo `clean-stale`: il prossimo passo parte da un
  `cargo clean`.
- Catena di master libera (issue #1, P-017, A-079, A-081, A-082): disegnata, approvata e **implementata in F14**.
  Da provare all'ascolto e col modello vero (VALIDATION 41–45).
- Nessun genere come norma (A-080): i prompt fissi non presuppongono più un genere. Il master parla di misure e non di
  strumenti (un test lo vieta); il bilanciamento del mix parte dal tipo di musica e tiene i rapporti di A-063 per la
  musica guidata da una voce. Da provare col modello vero su brani non rock (VALIDATION 41). Deciso con l'autore: la
  catena di master è una lista di blocchi ripetibili (due per tipo, dodici in tutto). L'autore ha scelto il **rack** tra tre
  disposizioni (A-081).
- Forma d'onda del master (A-083): su un master forte era un rettangolo pieno. Ora è disegnata dal master misurato,
  con il corpo in colore pieno, i picchi come linea sottile e il mixdown scuro al suo interno (VALIDATION 46). Issue #1 chiusa.
- Intestazioni delle tracce ridimensionabili (issue #9, A-084): una maniglia sul bordo destro della colonna la allarga
  da 344 px (304 nel compatto, che restano il minimo) fino a 640, lasciando sempre 320 px alle corsie; la larghezza è
  una preferenza dell'utente, il doppio clic la riporta al minimo.
- Effetti del mix e idee creative (P-018, A-085): scelti dall'autore dopo un'analisi degli effetti possibili. Specifica
  in FOUNDATION (FX-10 → FX-24, ANA-19, AI-25 → AI-27, UI-12, UI-13, `mix_plan` v4, fasi F15 → F17). Prossimo passo:
  le tavole, consegnate il 2026-11-14 (A-086): `design/screens/16-track-chain.html` (16a–16u) e
  `design/screens/04-mix-proposal.html` (4c, 4d). **Approvate dall'autore il 2026-11-14**; F15 → F17 di seguito.
- Verifica di P-018 a fasi chiuse (A-090): tre bloccanti e undici difetti che i test non vedevano, corretti ciascuno
  col suo test. Render identico a ogni blocco dopo un bypass spento e riacceso; fase di modulazione e tremolo legata
  alla posizione anche col rate automatizzato; il ritorno di un'idea resta fermo nel ciclo; limiti della catena
  verificati al caricamento e in ogni comando; revisioni che non falliscono sui valori messi a mano; proposte riaperte
  senza blocchi doppi; exciter senza buco alla frequenza di taglio, de-esser che non scatta sulle vocali. Dopo le
  correzioni: `make bench` (NFR-1 p99 12,1%, FX-24 p99 26,9% su soglia 50%, NFR-4 31×, ANA-17 12,4 s, NFR-5 picco
  1,20 GB), pacchetto ricostruito e self-test nell'app impacchettata (49 passi su 49, avvio a freddo 1,4 s) · disco
  6,8 GB dopo un `cargo clean` del profilo di sviluppo e `clean-stale`.
- Dalla prova dell'autore su 33 stem (A-091): l'auto mix con le idee falliva su ogni risposta di un modello vero,
  perché il parser rifiutava i `null` che lo schema strict chiede per le idee non usate e per le loro mandate. Ora
  sono letti come assenti; le tre risposte registrate, rigiocate nel ciclo, diventano una proposta.
- Explain this mix (P-019, A-092, chiesto dall'autore il 2026-11-15): una vista che spiega il mix senza modello, con
  la mappa del segnale (tracce → gruppi → bus, mandate, sidechain) e lo stesso mix in parole, per elemento e per
  sezione. Tavole `design/screens/17-mix-explain.html` (17a–17j) approvate lo stesso giorno e **implementate in F18**
  (A-093). La narrazione con un modello locale è rimandata.
- Tour guidato (issue #5, P-020, A-094): specificato con l'autore il 2026-11-15, **non ancora iniziato**. Audio
  sintetizzato, giro completo in quindici passi, guidato con Next, tavole da approvare prima dell'implementazione.
  F19 parte dalle tavole della schermata 18 (`design/screens/18-tour.html`) e si ferma per l'approvazione.
  **Tavole consegnate il 2026-11-15** (A-095): 18a–18o i quindici passi coi testi definitivi, 18p–18q passi che
  aspettano proposta e mixdown, 18r finestra minima, 18s–18u avvio dalla Welcome, dal menu dell'app e con un progetto
  aperto, 18v notifica dopo `Skip tour`, 18w brano non preparabile; C8 riflettore e fumetto; note per chi implementa
  in `design/README.md` ("Tour guidato"). Approvate e **implementate in F19** (A-097, tag `f19`); da provare
  nell'app vera su dati puliti, coi testi e col suono del brano sintetico (VALIDATION 57–61).
- Verifica di F19 a fase chiusa (A-099, chiesta dall'autore): cinque difetti del tour che i test non vedevano,
  corretti ciascuno col suo test: passo 8 bloccato dopo `Apply all` o `Discard`, `Cancel` che annullava la proposta del
  tour, Invio su `Back` che andava avanti, conferma di `Discard` sotto il fumetto, scorciatoie attive da un campo di
  testo. `make check` verde (116 Playwright); pacchetto ricostruito, self-test nell'app impacchettata 59/59 (avvio a freddo
  1,4 s) · disco 10,57 GB dopo `clean-stale`.
- Landing page (issue #4, A-100): quattro direzioni in canvas, scelta la **A · One song, start to finish**; film
  dell'hero di 65 secondi in dodici scene, con la traccia dell'autore. Per scelta dell'autore il sito **non sta in
  questo repository**: vive nel repository privato `stemwise-promo` (`../stemwise-promo`), con immagine nginx e
  configurazione Traefik per il VPS. Qui resta `LICENSE` MIT alla radice. Da fare dopo l'apertura del repository:
  VALIDATION 62.
- Apertura del repository (A-101, 2026-11-16): piano in sette passi e quattro scelte dell'autore (PoC in `poc/` sotto
  GPL, email dei commit invariata, documenti di progetto in italiano con README in inglese, prima release non
  firmata). **Passi 1–5 fatti**: `main` portato a `product/v1`; metadati MIT, `poc/LICENSE` GPL, elenco delle licenze di
  terze parti nel pacchetto (`make third-party`); nome e logo riservati (`TRADEMARKS.md`); storia cercata per
  segreti e dati personali, `CLAUDE.md` aggiornato (passo 3); README pubblico in inglese con cinque schermate
  (passo 4); `CONTRIBUTING`, `SECURITY`, modelli, CI con `make ci` (non ancora eseguita), issue tradotte (passo 5).
  Passo 6: pacchetto 0.1.0 costruito da `d783166`, licenze dentro, self-test nel pacchetto 59/59 (avvio a freddo 1,4 s);
  release `v0.1.0` pubblicata su GitHub. Passo 7 preparato (descrizione, argomenti, sito
  <https://stemwise.pierodetomi.dev>, immagine di anteprima): **manca solo rendere pubblico il repository**, e subito
  dopo protezione di `main`, segnalazione delle vulnerabilità, secret scanning, prima corsa della CI · disco 10,89 GB.
- Due test Playwright instabili sotto carico, emersi rieseguendo `make check` (tre fallimenti su cinque esecuzioni,
  A-102): nel tour `Next` sembrava pronto per un fotogramma prima che il passo cominciasse a prepararsi, e il tasto
  premuto in quell'istante andava perso (corretto nel codice); il test dell'overlay del Master cercava a intervalli
  uno stato che dura 220 ms (ora è la pagina a cogliere l'istante). Dopo: i due file tre volte di fila, 96 su 96.
- **Repository pubblico dal 2026-11-16** (A-101): sicurezza e protezione di `main` accese, sito e link verificati da
  fuori. Da fare: guardare la prima corsa della CI, caricare l'immagine di anteprima (autore).
- **0.1.0 ritirata, 0.1.1 pronta** (A-103): la 0.1.0 scaricata risultava "danneggiata" perché il pacchetto non aveva
  una firma valida. Ora firma ad hoc del bundle, verificata da `make build`. Corretti anche l'ordine dell'elenco
  delle licenze (falliva sulla CI) e la causa vera del test instabile del tour (il Master prendeva il fuoco tra due
  passi). `make check` verde · disco 12,5 GB.
- 0.1.1 pubblicata e verificata scaricandola da fuori, 0.1.0 eliminata, landing aggiornata (A-103). CI: seconda corsa
  ferma ancora sull'elenco delle licenze, per due pacchetti che pnpm elencava solo in locale; l'elenco ora si ricava
  da `node_modules` senza pnpm (355 librerie).
- **CI verde** su `8738e13` (A-103): tutto il gate sul runner di GitHub, 114 test Playwright. Le code dei runner macOS
  possono far annullare una corsa: in quel caso si rilancia.
- **Windows (A-104, 2026-11-16, chiesto dall'autore).** Gate, pacchetto e app vera su Windows 11 x64: `make check`
  verde (Playwright su Edge, il motore di WebView2), installer NSIS da `make build`, self-test nell'eseguibile release
  59 su 59 con l'audio WASAPI (avvio a freddo 0,8 s). Difetti trovati e corretti: salvataggio atomico che falliva
  su Windows, Save As con i percorsi `\`, snap che leggeva solo il tasto ⌘, cartella dati della CLI. AAC con Media
  Foundation, barra dei menu solo su macOS, scorciatoie e testi per sistema, elenco licenze per tre target (497
  librerie). CI anche su `windows-latest` (job `make ci (Windows)`, ancora mai eseguito: parte alla prima PR). Da
  provare a mano: VALIDATION 64–66. Restano Linux e la firma Authenticode · disco 7,85 GB.
- **0.1.2 per macOS e Windows (A-105, 2026-11-17).** Ramo di Windows in `main` in avanti veloce, versione 0.1.2,
  `make check` verde. Nuovo workflow `release.yml`: al push di un tag `vX.Y.Z` costruisce `.dmg` e installer NSIS su
  GitHub e li allega con `SHA256SUMS.txt` a una release in bozza, pubblicata a mano. Pacchetti scaricati e verificati
  (somme, firma, versione); self-test nel `.dmg` della CI 58 su 59 a Mac in uso, con l'avvio a freddo sopra soglia
  (3,9–8,5 s): da ripetere a Mac fermo. Test di accessibilità con un tempo massimo di 90 s (superava i 30 s sul runner
  macOS). README con l'installazione su Windows; sito allineato in `stemwise-promo` · disco 14,8 GB.
- Prossimo passo: in parallelo la validazione dell'autore (docs/app/VALIDATION.md, nell'ordine
  consigliato), almeno un auto mix col modello vero e il pacchetto su un Mac pulito prima di aprire.
- Verifica dello stato: `cd app && make check` (verde), `make reqs` per i buchi residui, `git tag` per le fasi.

## Riserve

_Requisiti ridotti o non verificati, con motivo e riferimento alla decisione (FOUNDATION §9, regola 3)._

- **AI col provider vero (F5)** — schema strict per progetto, costi riportati, portachiavi: verificabili solo con la
  chiave dell'autore (VALIDATION 7–11). In mancanza dello strict l'app ripiega da sola sul JSON mode (A-031).
- ~~Misura del master AI (MAS-6)~~ — chiusa in F8: la proposta del master è misurata prima dell'applicazione
  (A-033, A-042).
- **Master AI misurato col modello vero (MAS-6)** — il ciclo è provato col decisore finto e le misure sul mixdown
  vero; che un modello reale usi gli stadi prima del limiter e tenga il target lo dice solo la prova dell'autore
  (A-077, VALIDATION 39). Il target in una revisione è tenuto dal prompt, non dal codice.
- **Catena di master libera all'ascolto e col modello vero (F14)** — clipper, multibanda, sezioni e cambi di ordine
  sono misurati su segnali sintetici: il suono, e l'uso che ne fa un modello reale, li giudica l'autore (A-082,
  VALIDATION 41–45). Lo schema strict con `anyOf` sui tipi di blocco è provato solo col server finto.
- **Da dove vengono i picchi (15g)** — l'avviso sulla destinazione troppo alta non nomina la famiglia di tracce: manca
  l'analisi degli stem contro il mixdown. La loudness più bassa che propone è una stima (A-082).
- **Famiglie per la musica orchestrale (AI-22)** — i ruoli nuovi (`piano`, `woodwinds`, `choir`) stanno nelle famiglie
  esistenti; il bilanciamento per sezioni d'orchestra non c'è (A-080, A-082).
- **Pacchetto non firmato (F10)** — l'agente non ha un Developer ID: firma e notarizzazione restano all'autore; il
  primo avvio chiede clic destro › Apri (A-046, VALIDATION 23). Il `.dmg` è senza la disposizione personalizzata
  della finestra (A-046).
- **Plate sotto 1 s (FX-9)** — il plate è granuloso per costruzione con decadimenti sotto circa 1 s: la densità è
  verificata da 2 s in su; pannello e prompt indirizzano a `room` per gli spazi corti (A-058, VALIDATION 25).
- **Mix AI strutturale all'ascolto e col modello vero (F12)** — che la struttura migliori davvero il mix, e quanta
  ne proponga un modello reale, lo giudica l'autore (VALIDATION 27, 29). Lo schema strict con i segnaposto annullabili
  (`anyOf`) è provato solo contro il server finto.
- **Soglie dei doppi (ANA-18)** — iniziali, provate su segnali sintetici: il margine tra una copia (0,13 dB per frame)
  e due esecuzioni sintetiche (0,8 dB) è stretto attorno alla soglia di 0,5 dB; da tarare su materiale vero
  (A-060, VALIDATION 28).
- **Riverberi e compressione parallela all'ascolto (F11)** — le proprietà sono misurate, il suono lo giudica l'autore
  (VALIDATION 25–26).
- **Bilanciamento del mix AI (AI-22)** — i rapporti tra le famiglie e le soglie dei ritorni nel prompt sono regole
  pratiche: che i mix migliorino lo giudica l'autore col modello vero (A-063, VALIDATION 31–32).
- **Collegamento a OpenRouter (AI-24)** — provato solo contro il server finto: il comportamento del provider dopo un
  rifiuto e la registrazione dentro la pagina di autorizzazione non sono documentati (A-071, VALIDATION 36).
- **Portachiavi silenzioso nella build firmata (AI-11)** — dipende dalla firma dell'autore: non verificabile
  dall'agente (A-072, VALIDATION 38).
- **Blocchi nuovi e idee all'ascolto e col modello vero (F15 → F17)** — le proprietà dei blocchi sono misurate su
  segnali sintetici; il suono, la soglia della sibilanza (−3 dB, A-089) e la qualità delle idee di un modello reale li
  giudica l'autore (VALIDATION 47–54). Lo schema strict con i blocchi in `anyOf` è provato solo col server finto.
- **Posizione nel brano (FX-20, FX-24)** — modulazione e tremolo ripartono dalla stessa fase a ogni posizione, anche
  col rate automatizzato, verificato con tolleranza 1e-4 nel motore e 1e-5 nei processori (non bit a bit, per
  l'accumulo in virgola mobile). Un rate cambiato a mano mentre si suona si riallinea alla partenza successiva. La
  deriva del doubler è legata alla posizione, la fase del suo pitch shifter (e di harmonizer e shimmer) riparte da
  dove parte la riproduzione (A-088, A-090).
- **Bass enhancer (FX-18)** — senza oversampling 4x e senza compensazione del livello: non servono a un generatore
  di armoniche sotto i 640 Hz che si somma al dry (A-090).
- **Transient shaper sotto i 100 Hz (FX-16)** — un tono fermo molto basso può muovere il guadagno (A-088).
- **De-esser in split (FX-14)** — un'ottava sotto la frequenza il livello scende ancora di circa 0,7 dB a 12 dB di
  riduzione (A-090).
- **Benchmark a 48 kHz (NFR-1, FX-24)** — a 96 kHz il carico non è misurato; la ricerca del pitch shifter ora non
  cresce con la frequenza (A-090).
- **Nome dell'idea nel tooltip (UI-13)** — il nome dell'idea sta nella nota della traccia (motivazione e tooltip del
  numero), non in un tooltip di ogni blocco (A-089).
- **Explain nell'app vera (F18)** — la vista è provata sul backend finto e la misura dei livelli da un test Rust; il
  comando `mix_levels` nell'app Tauri, il pacchetto e il self-test non sono stati eseguiti (A-093, VALIDATION 55–56).
- **Explain: per famiglia, senza riduzione misurata (UI-16)** — il testo dice chi sta davanti per famiglia e non per
  traccia, e descrive i compressori da rapporto e soglia: mancano il render per traccia e la misura della riduzione
  (A-093). Il Master non ha ancora la sua vista.
- **Self-test nel pacchetto 0.1.1 (A-103)** — a Mac in uso: tre esecuzioni complete su cinque, una con un passo
  senza segnale sul bus, una rimasta ferma. A Mac fermo: tre su tre, 59 su 59. I due esiti anomali non sono stati
  riprodotti e la causa non è accertata: se ricapitano a macchina ferma, è un difetto da cercare nel self-test o
  nell'avvio dell'audio.
- **Windows non provato a mano (A-104)** — gate e self-test girano sulla macchina di sviluppo; installer su un
  Windows pulito, ascolto, trascinamento con lo scaling e finestra minima su un portatile al 150% li giudica l'autore
  (VALIDATION 64–66). Pacchetto non firmato: SmartScreen avvisa. La CI su Windows non ha ancora mai girato. Il render non è identico
  bit per bit tra sistemi (libreria matematica del sistema): il test dello schema 3 gira solo su macOS.
- **Locate a un file per volta (UI-6)** — il dialogo con ricerca per cartella del design 13g è sostituito da un menu
  dei file mancanti (A-047).
