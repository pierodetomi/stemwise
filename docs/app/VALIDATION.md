# Validazione dell'autore

Ciò che l'agente non può verificare da solo (FOUNDATION §8.5): ascolto, modello reale, firma. Ogni voce ha passi
eseguibili e il criterio per dirla superata. L'elenco si completa a ogni fase; l'ordine consigliato è in fondo.

## Motore audio (F3)

1. **Riproduzione senza click né buchi.** `cd app && make dev`; apri un progetto con almeno 16 tracce (anche il
   progetto dimostrativo: `cargo run -p stemwise-cli -- demo ~/Music --name Prova --seconds 120`). Premi Play e
   ascolta in cuffia per due minuti, poi: muovi un fader velocemente, metti e togli mute e solo su più tracce, muovi il
   fader del bus. Atteso: nessun click, nessuna interruzione; i cambi sono morbidi.
2. **Modifiche strutturali durante l'ascolto.** Durante la riproduzione aggiungi una mandata o raggruppa due tracce.
   Atteso: una breve pausa sfumata (qualche decina di ms), poi la riproduzione riprende dallo stesso punto (A-025).
3. **Loop e locate.** Imposta un loop di 4 battute, avvialo, poi clicca sul righello fuori dal loop e dentro.
   Atteso: ritorno all'inizio del loop senza scatti di contenuto; nessun click ai salti.
4. **Dispositivo a frequenza diversa e buffer.** In Impostazioni › Audio scegli un'uscita a 44,1 kHz con un progetto a
   48 kHz, e prova buffer 128, 512, 2048. Atteso: intonazione giusta, nessun crepitio; con 128 su un Mac lento può
   servire un buffer più grande (annotare quale).
5. **Carico con molte tracce.** Con 32 tracce e catene complete, guarda l'indicatore di carico del motore nella barra
   superiore durante la riproduzione. Atteso: sotto il 30% (sul Mac di sviluppo il benchmark dà il 9% medio,
   A-023/A-024); nessun crepitio anche con altre app aperte.
6. **Scollegare il dispositivo.** Durante la riproduzione stacca le cuffie USB o cambia uscita di sistema. Atteso:
   l'app lo segnala (UI-6) e non si blocca.

## AI (F5)

Tutto il flusso AI è provato offline col decisore finto e contro un server HTTP locale finto. Restano da verificare
col servizio vero, con la tua chiave (l'agente non usa chiavi né rete):

7. **Prima chiamata reale e schema strict.** In Impostazioni › AI incolla la chiave OpenRouter, lascia il modello
   predefinito e lancia Auto mix sul progetto dimostrativo. Atteso: una proposta con sintesi e motivazioni nella
   lingua del brief. Nel log della chiamata (cartella `ai/<id>/` del progetto, file `request-0-0.json`) controlla
   `response_format.type`. Se è `json_schema`, il provider accetta lo schema per progetto (A-031). Se è
   `json_object`, ha rifiutato lo schema e l'app è passata al JSON mode: funziona comunque, ma annota il modello.
   Annota anche quante richieste ha fatto la prima iterazione (una sola è il caso atteso).
   Portachiavi (A-052): all'avvio e all'apertura dei progetti macOS non deve chiedere la password. Può chiederla
   una volta, alla prima chiamata della sessione, finché l'app non è firmata ("Consenti sempre" vale fino alla
   build successiva).
   Menu (A-053): File › Save, Save As…, Close Project, Open… e New Project… della barra dei menu fanno quello che
   dicono; Edit › Copy e Paste funzionano nei campi di testo.
8. **Costi.** Dopo due o tre chiamate apri AI usage. Atteso: costo per chiamata uguale a quello del pannello di
   OpenRouter (Activity) entro qualche centesimo; nessuna voce "stima" se il provider riporta il costo. Poi annulla
   una chiamata a metà: deve comparire con un costo marcato come stima, e su OpenRouter non deve risultare di più.
9. **Portachiavi.** Chiudi e riapri l'app: la chiave c'è ancora (Impostazioni › AI dice "Stored in the keychain";
   la chiave non si mostra mai, nemmeno mascherata, A-044). In Accesso Portachiavi deve
   esistere una voce "Stemwise". Togli la chiave dall'app: la voce sparisce.
10. **Rete assente.** Spegni il Wi-Fi e lancia Auto mix. Atteso: un errore chiaro e recuperabile (UI-6), nessun
    blocco; la chiamata compare in AI usage come fallita.
11. **Revisione a parole sul serio.** Dopo un auto mix applicato, chiedi "voce un po' più calda nelle strofe".
    Atteso: cambiano soprattutto voce e tracce che la coprono; le spinte per sezione del primo auto mix restano
    (corsie azzurre ancora presenti).

## Assistente nell'app (F7)

12. **A/B della proposta all'ascolto.** Lancia Auto mix, poi durante la riproduzione alterna "Current mix" e
    "Proposal" (o premi B). Atteso: il passaggio avviene senza fermare la riproduzione, con una breve dissolvenza;
    nessun passo nuovo in History.
13. **Applica in parte.** Togli la spunta a due tracce e premi "Apply selected". Atteso: un solo passo in History, con
    la scintilla; Undo riporta tutto com'era; in AI usage la chiamata risulta "Applied in part".
14. **Export della storia.** In AI usage › Export scegli CSV e apri il file in Numbers o Excel. Atteso: una riga
    per chiamata, con costi e richieste leggibili (le virgolette nelle richieste non spezzano le colonne).
15. **Pulizia dopo le verifiche dell'agente.** Le prime esecuzioni del self-test (prima di A-041) hanno scritto
    nella cartella dati dell'app, `~/Library/Application Support/app.stemwise.desktop/`, i progetti recenti di
    prova e una chiamata da 0 $ della demo offline (`ai-costs.jsonl`). L'agente non tocca file fuori dal
    repository: se vuoi, cancella i due file prima di usare l'app.

## Master ed export (F8)

16. **Ascolto del master.** Fai Bounce & go to Master, poi alterna Master, Mixdown e Reference durante la
    riproduzione. Usa anche un riferimento MP3 o M4A a 44,1 kHz.
    - Atteso, in "Real level" (predefinito): il master suona come il file esportato, mixdown e riferimento al loro
      livello vero, il riferimento alla velocità giusta. Gli scarti che senti sono quelli scritti nella sezione
      Listening della scheda Loudness.
    - Atteso, con "Same loudness" (o ⇧B): le sorgenti suonano alla stessa loudness percepita. Se sotto le righe
      compare "everything plays X dB lower", tutto suona più basso della stessa quantità per non saturare.
    - Atteso: cambiare modo o sorgente durante la riproduzione non la ferma e non fa click. Anche la catena non deve
      introdurre click quando muovi i controlli o accendi e spegni un blocco. Il salto massimo fra campioni è
      verificato da un test (A-043), ma conta l'orecchio.
17. **Destinazioni.** Prova Streaming, Loud e CD. Atteso: l'Integrated del pannello Loudness sta entro ±0,1 LU dalla
    destinazione dopo qualche secondo di misura (A-055); con Loud il limiter lavora di più ma non distorce in modo
    udibile.
    - Con un riferimento a circa −10 LUFS scegli Custom a −18 LUFS. Atteso: l'avviso "Much quieter than your
      reference" nella scheda Loudness e nel dialogo di export. "Match the reference" porta la destinazione al
      riferimento con lo stesso tetto, ed è un solo passo di undo. "Keep" toglie l'avviso, ma resta la riga "kept on
      purpose" con "Warn me again".
18. **File esportati.** Esporta WAV 24 bit, FLAC 16 bit e AAC 256 kbps con titolo, artista, ISRC e copertina. Apri i
    tre file in Musica e nel Finder (Ottieni informazioni). Atteso: si aprono, i tag e la copertina compaiono, la
    durata è giusta; il rapporto dice un true peak sotto il tetto per tutti e tre.
19. **Mixdown non aggiornato.** Torna a Mix, cambia un fader, poi scegli Master. Atteso: l'app propone il bounce;
    dopo il nuovo bounce le impostazioni del master sono quelle di prima.

20. **Impostazioni (F9).** Apri Impostazioni con ⌘, (o con l'ingranaggio).
    - Audio: cambia dispositivo e buffer. Atteso: il suono riprende sul dispositivo scelto e il carico del motore si
      aggiorna. Il segnale di prova non c'è (A-044): ascolta il progetto.
    - AI: inserisci la chiave vera. Atteso: compare "Stored in the keychain", la chiave non si vede mai.
    - Aspetto: le scelte restano dopo il riavvio dell'app.

21. **Stati di errore (F9).**
    - File mancanti: chiudi l'app, cancella un file da `audio/` dentro il pacchetto (Mostra contenuto del pacchetto) e
      riapri. Atteso: clip a righe, silenziose, banner "missing"; "Locate…" col file originale le riporta. Il bounce si
      rifiuta finché manca qualcosa.
    - Dispositivo: durante la riproduzione stacca le cuffie USB o la scheda. Atteso: il banner sotto la barra, la
      riproduzione ferma, "Use …" riporta il suono.
    - Rete: con la chiave vera e il Wi-Fi spento, chiedi un Auto mix. Atteso: "No connection to OpenRouter" al posto
      del riquadro della chiave, il mix intatto.
    - Accessibilità: con VoiceOver (⌘F5), naviga i controlli del mixer e dell'inspector. Atteso: ogni controllo ha un
      nome e un valore letti; frecce su/giù cambiano i valori.
22. **Avvio a freddo (NFR-3).** Apri `Stemwise.app` dal Finder dopo un riavvio del Mac. Atteso: la finestra usabile
    in meno di 2 secondi.

## Pacchetto (F10)

23. **Installazione dal .dmg.** Apri `app/target/release/bundle/dmg/Stemwise_0.1.0_aarch64.dmg` e trascina Stemwise
    in Applicazioni.
    - Il pacchetto non è firmato né notarizzato: l'agente non ha certificati. Al primo avvio usa clic destro › Apri,
      poi conferma.
    - Atteso: la welcome entro 2 secondi (voce 22). Crea un progetto con stem veri (WAV 24 bit e un FLAC). Atteso:
      le tracce compaiono con le forme d'onda e i ruoli proposti.
    - Per distribuirlo ad altri servono un Developer ID e la notarizzazione (fuori dal lavoro dell'agente).
24. **Un brano vero dall'inizio alla fine.** Su un tuo brano: brief, Auto mix con la chiave vera, ascolto in A/B,
    applica, una revisione a parole, bounce, master AI, destinazione Streaming, export WAV 24, FLAC 16 e AAC 256.
    - Atteso: i file si aprono in Musica, la loudness nel rapporto è a −14 ± 0,5 LUFS e il true peak sotto −1 dBTP.
    - Il giudizio artistico (il mix suona meglio della somma grezza?) è tuo: annotalo con il modello usato.

## Riverbero e compressione parallela (F11)

25. **I tre riverberi all'ascolto.** In un progetto con voce e batteria seleziona il ritorno riverbero e manda la voce
    a −12 dB. Prova Plate (decay 1,2 s), Room (0,6 s) e Hall (2 s) dal selettore nel pannello; poi lo stesso con il
    rullante. Atteso: tre caratteri diversi, code senza suono metallico né eco che si ripete; nessun click cambiando
    algoritmo, Decay o Size durante la riproduzione. Ascolta in particolare:
    - il Plate sotto 1 s: è atteso un po' granuloso (limite noto, A-058); dimmi se da 1 s in su è pulito;
    - Modulation a 0, 20% e 60% su una coda lunga (Hall, 4 s): a 0 la coda può "suonare" su una nota, al 60% non deve
      diventare un chorus;
    - Damping a 0 e all'80%: la coda si scurisce senza accorciarsi sulle medie.
26. **Compressione parallela.** Sul rullante (o sul gruppo della batteria) imposta Threshold −30 dB, Ratio 6, Makeup
    +8 dB, poi porta Mix dal 100% al 35%. Atteso: i colpi restano naturali e il corpo cresce; nessun suono "scavato"
    o metallico (dry e compresso sono allineati al campione). Muovi Mix durante la riproduzione: nessun click.

## Mix AI strutturale (F12)

27. **Proposta con struttura, all'ascolto.** Su un brano con voce e batteria su più tracce lancia Auto mix con
    "Allow structural changes" attivo. Atteso: nella proposta, sopra le tracce, la sezione "New in this proposal" con
    i ritorni e i gruppi che il piano crea, ciascuno con il motivo. Ascolta in A/B: la voce nel suo spazio, la batteria
    più compatta. Poi:
    - togli la spunta a un elemento nuovo e applica: non viene creato, il resto sì;
    - annulla con un solo ⌘Z: spariscono anche le tracce nuove;
    - rilancia Auto mix con la spunta disattivata e confronta le due proposte a orecchio: è qui che si vede se la
      struttura migliora davvero il mix.
28. **Doppi veri.** Su un brano con una parte registrata due volte (chitarre, cori) con lo stesso ruolo, lancia Auto
    mix e guarda in AI usage › dettaglio la voce `doubles` dell'analisi. Atteso: la coppia c'è, non `identical`, e la
    proposta le apre ai lati. Dimmi se riconosce come doppio qualcosa che non lo è, o se ne manca uno: le soglie sono
    iniziali (A-059).
29. **Col modello vero.** Con la chiave, un Auto mix: guarda quante cose nuove propone e se le motivazioni reggono.
    Atteso: poche e utili; se ne crea troppe o inutili, annota modello e brano.

## Nuova destinazione (A-061)

30. **Cambio della destinazione in Master.** Su un tuo brano, in Master, passa da Streaming a Loud. Atteso: dopo un
    istante l'overlay "Bringing the master to the new target" copre lo spazio di lavoro, con la destinazione nuova
    e quella di prima; la riproduzione si può avviare e fermare e suona ancora al livello di prima; quando
    l'overlay sparisce il livello è quello nuovo e la scheda Loudness dice "On target". Poi ⌘Z: si torna a
    Streaming subito, senza overlay. Da giudicare: se l'attesa è chiara e se la barra, che avanza a passate, non
    confonde.

## Bilanciamento del mix AI (A-063)

31. **Bilanciamento ed effetti all'ascolto.** Su Balance (o un brano con batteria su molti microfoni, chitarre,
    voce e cori) lancia Auto mix senza testo. Atteso: almeno due iterazioni; in AI usage, nel JSON della seconda
    richiesta, `evaluation.balance` con i livelli di `drums`, `guitars`, `lead_vocal`, `backing_vocal` e
    `returns_lu`. Da giudicare a orecchio: la voce davanti o alla pari della batteria, le chitarre presenti, i cori
    udibili, un riverbero che si sente sulle voci. Se un rapporto non ti torna, scrivilo nelle note del brief e
    riprova: i rapporti nel prompt sono punti di partenza da tarare sul tuo gusto.
32. **Revisione su una sezione oltre i due minuti.** Scrivi una richiesta su una sezione che comincia dopo 2:00
    (per Balance: "chitarre più avanti nel Solo") e premi il pulsante, che ora dice "Revise mix". Atteso: la
    proposta dice "You asked: …", cambia quella sezione (corsie di automazione) e il fader può muoversi di più di
    4 dB se la richiesta lo chiede. Invio nel campo va a capo e non invia.

## Stato del mix a colpo d'occhio (F13)

33. **Leggibilità su un progetto vero da 30 tracce.** Apri Balance (o un brano con 30 stem, gruppi e almeno tre
    ritorni) in `make dev`, corsie piccole (⌥⌘−). Atteso: su ogni intestazione e strip le sigle HP LP EQ CMP SAT,
    piene dove il blocco lavora, bordate dove è acceso ma non cambia nulla, tenui in bypass; `CMP` nel colore della
    traccia ascoltata in sidechain e con due barrette se parallelo; un punto per ritorno e il rombo dell'automazione
    (blu se scritta dall'assistente). Nel mixer una riga di mandata per ogni ritorno, coi nomi nella colonna a
    sinistra. Da giudicare: se senza selezionare nulla capisci chi ha effetti, mandate e sidechain, e se le
    intestazioni da 344 px tolgono troppo spazio alle corsie (A-066).
34. **Sigle, collegamenti e Overview.** Clic su una sigla in una strip: il blocco va in bypass e ⌘Z lo riaccende in
    un passo; doppio clic: la traccia si seleziona e l'inspector apre quel blocco. Seleziona un ritorno: le tracce che
    gli mandano si marcano in lime col livello al posto del ruolo, e la corsia del ritorno le elenca. Seleziona una
    traccia: il suo gruppo dice "takes …", chi la ascolta "listens to …". In fondo alla colonna del mixer passa
    all'Overview: un clic su una cella mette o toglie il bypass, su una mandata vuota la crea a −12 dB. Nel browser
    (`make ui`) più ritorni si aggiungono dalla console con `__stemwiseMock.simulateReturns(4)`.

## Proposte che restano (A-070)

35. **Riapplicare una proposta.** Nell'app vera, su un tuo brano: chiedi un auto mix e scartalo. Apri la scheda
    `Proposals` dell'assistente: la proposta è in elenco con data, ora ed esito. `Review…`: torna in revisione con la
    nota "Received …"; ascoltala in A/B, poi `Apply all`. Atteso: una conferma che dice che il mix attuale cambierà;
    dopo la conferma il mix è quello della proposta e ⌘Z lo riporta indietro in un passo. Poi muovi qualche fader,
    elimina una traccia e riapri la stessa proposta: le differenze sono contro il mix di adesso e la traccia eliminata
    è dichiarata tra gli avvisi. Chiudi e riapri il progetto: la proposta è ancora riapribile. Ripeti in Master con
    `Master with AI`. Le proposte ricevute prima di questa versione sono in elenco ma col pulsante disattivato.

## Configurazione guidata dell'assistente (A-071, A-072)

36. **Collegare il conto dal browser.** In `make dev` (build non firmata): Impostazioni › AI › `Disconnect` se c'è una
    chiave, poi `Set up the assistant…` › `Connect with OpenRouter`. Atteso: si apre openrouter.ai sulla pagina di
    autorizzazione, col nome della chiave "Stemwise"; dopo `Authorize` il browser mostra la pagina "Connected" e il
    dialogo passa da solo ad `Add credit`. Da annotare: se la pagina di OpenRouter fa registrare e ricaricare nello
    stesso passaggio; cosa succede rifiutando (il dialogo dovrebbe dire "The connection was not approved"; se resta
    in attesa, il provider non torna dopo un rifiuto e va detto in 14b); se gli importi di `Add credit` sono vicini
    ai costi veri di un auto mix. Poi un auto mix: nessuna richiesta di password.
37. **Credito esaurito e chiave revocata.** Con una chiave a limite basso (o cancellandola da openrouter.ai) chiedi
    un auto mix. Atteso: nella colonna dell'assistente "Your OpenRouter credit has run out", "The key has reached its
    own limit" oppure "OpenRouter no longer accepts this key", ciascuno col suo pulsante; il mix non cambia e la
    chiamata è a costo zero in AI usage.
38. **Dove sta la chiave.** Build non firmata: Impostazioni › AI dice "Stored in a private file of this app"; il file
    è `~/Library/Application Support/app.stemwise.desktop/openrouter.key`, con permessi `-rw-------`. Pacchetto
    firmato col tuo Developer ID: dice "Stored in the keychain", e collegare il conto e fare un auto mix non chiedono
    mai la password, nemmeno dopo aver installato una build successiva firmata allo stesso modo. La chiave salvata
    nel portachiavi dalle build non firmate precedenti non serve più: si può eliminare da Accesso Portachiavi
    (elemento "Stemwise").

## Master AI misurato (A-077)

39. **Master AI su una destinazione forte.** Su Balance, in Master, con la destinazione a −9,6 LUFS e la catena
    azzerata, chiedi "mantieni il punch del rullante e la dinamica". Atteso: l'avanzamento passa per più decisioni e
    render; la proposta **non abbassa il target**, usa stadi prima del limiter (compressore veloce, saturazione, EQ sui
    bassi) e il riepilogo dice quanto lavora ancora il limiter. All'ascolto il rullante deve restare più definito
    del master col solo limiter. In `ai/<chiamata>/` del progetto, `evaluation-0.json` e `evaluation-1.json` mostrano
    `limiter.time_over_3_db_pct` in discesa. Se il target scende comunque o il limiter resta strozzato, il limite è
    negli strumenti (manca il clipper) o nel modello: annota quale delle due cose dice il riepilogo.

## Il marchio nell'app (A-078)

40. **Icona e marchio nuovi.** Dopo `make build`, apri `Stemwise.app`. Atteso: nel Dock e nel Finder l'icona è lo
    squircle melanzana con le due linee lime e la stanza lilla, riconoscibile anche nella vista a elenco del Finder
    (16 pt). Se il Dock mostra ancora l'icona a barre, è la cache delle icone di macOS: trascina l'app nel Cestino del
    Dock e riaprila, oppure riavvia il Dock. Nella barra in alto del Mix sono accese le due linee e la stanza è smorzata;
    in Master il contrario. Nella schermata iniziale il marchio è a colori sopra il nome. Collegando l'assistente,
    la pagina finale nel browser ha il marchio a colori accanto a "Stemwise".

## Nessun genere come norma (A-080)

41. **Mix e master AI su musica non rock.** Con due o tre brani di tipo diverso (per esempio uno acustico o jazz
    registrato dal vivo, uno elettronico, uno strumentale), brief col genere giusto: chiedi auto mix e master AI.
    Atteso: sull'acustico il mix sposta poco i fader e non aggiunge riverberi evidenti, e il master resta leggero
    (poca o nessuna compressione, niente clipping) dicendolo nel riepilogo; sull'elettronico cassa e basso guidano il
    bilanciamento; nessun riepilogo parla di batteria o chitarre dove non ci sono. Annota dove il modello applica
    comunque abitudini da rock.

## Catena di master libera (F14, A-082)

42. **Clipper all'ascolto.** Su Balance a −9,6 LUFS, catena azzerata: accendi il Clipper e porta "Take off up to" a
    4 dB. Atteso: nella scheda Loudness "Limiter over 3 dB" scende molto; all'ascolto i colpi restano definiti e il
    mix non "respira" più dopo ognuno. Sali finché senti distorsione e guarda a che percentuale del brano lavora.
43. **Ordine e blocchi doppi.** Trascina il compressore sopra l'EQ, aggiungi un secondo EQ dopo il multibanda, poi
    togli un blocco. Atteso: nessuno scatto mentre suona, un passo di undo per ogni mossa, i nomi "EQ 1" e "EQ 2".
44. **Multibanda.** Accendilo, porta le bande a quattro, metti in solo la più bassa, trascina un crossover. Atteso:
    il solo si sente subito e non finisce nell'export; con tutti i rapporti a 1 il suono non cambia.
45. **Sezioni e master AI con gli strumenti nuovi.** Dai +0,5 dB ai ritornelli e ascolta l'ingresso nel ritornello:
    nessuno scatto. Poi ripeti il punto 39: atteso un piano che usa clipper o multibanda, eventualmente un ordine
    diverso o un blocco in più, con la tabella prima → dopo nella proposta; prova ad applicare solo alcune voci.
    Apri anche un progetto salvato prima di questa versione: deve suonare come prima.

46. **Forma d'onda del master.** Sul brano che a −9 LUFS dava un rettangolo pieno: dopo la misura la forma d'onda
    mostra il corpo del brano in colore pieno, con strofe e ritornelli distinguibili, e una linea sottile per i picchi; la parte
    scura al centro è il mixdown. Cambia destinazione: dopo la nuova misura il corpo cresce o cala.

## Catena delle tracce a blocchi ed effetti nuovi (F15, F16, A-087, A-088)

47. **Progetti di prima.** Apri un tuo progetto salvato prima di F15 (Balance). Atteso: suona come prima (stesso
    render: la migrazione è provata a hash identico su progetti sintetici), il mixdown non risulta da rifare, le corsie
    di automazione sono al loro posto.
48. **Rack nell'inspector.** Su una traccia: `Add block` › aggiungi un blocco (nasce spento), accendilo, trascinalo
    sopra il compressore, duplicalo, resettalo, toglilo; ⌘Z annulla ogni passo. Atteso: nessuno scatto quando un blocco
    si accende o si sposta in riproduzione; sigle sulle intestazioni e sulle strip coerenti (quattro e `+n`).
49. **Correttivi all'ascolto.** Gate su un tom o un rullante con rientri; de-esser su una voce sibilante (prova
    `Listen` sul solo segnale rilevato e le due modalità); EQ dinamico su chitarre con la sidechain dalla voce;
    transient shaper su un rullante. Atteso: fanno quello che dicono senza pompare o tagliare le code in modo innaturale.
50. **Colore, spazio, movimento, pitch.** Caratteri della saturazione (tape, tube, transistor: `soft` resta come
    prima), exciter, bass enhancer su un basso DI, ampiezza su un pad (le basse restano mono), chorus/flanger/phaser,
    tremolo e auto-pan a tempo (la fase resta legata alla battuta anche dopo un seek), harmonizer su una voce (le
    voci non seguono la tonalità, va detto), doubler, riverbero `shimmer`, delay ping-pong e tape, filtri risonanti.
    Atteso: nessun artefatto evidente; il giudizio sul suono è tuo.
51. **Catena dei ritorni.** Su un ritorno di riverbero aggiungi un compressore con la sidechain dalla voce (ducking):
    il riverbero si abbassa mentre la voce canta e torna nelle pause.

## Idee creative (F17, A-089)

52. **Blocchi nella proposta.** Auto mix col modello vero su un brano con una voce sibilante e batteria con rientri.
    Atteso: un de-esser o un gate appaiono come righe "new block" nel diff della traccia, al posto giusto della catena
    dopo l'applicazione. La soglia della sibilanza (−3 dB nel prompt) è iniziale: dimmi se il de-esser arriva dove
    serve e non altrove (A-089).
53. **Idee col modello vero.** Con "Allow creative ideas" acceso, auto mix: al più tre idee, diverse tra loro, non
    correttive, spente. Prova `Listen` su ognuna (suona la proposta con quell'idea sola), spuntane una e applica tutto:
    un solo passo di cronologia che nomina l'idea; la nota della traccia la cita; ⌘Z toglie tutto. Spegni
    l'interruttore e richiedi: nessuna idea. Prova anche una revisione a parole ("qualcosa di più audace nel bridge").
54. **Idee all'ascolto.** Le idee spuntate migliorano il brano o lo distraggono? Il giudizio è tuo: è la parte di P-018
    che nessuna misura verifica.

## Explain this mix (F18, A-092, A-093)

55. **La spiegazione su un brano vero.** Apri un progetto vero, `Explain` nella barra in alto (⌘E). La mappa dice
    davvero chi va dove? Il testo si capisce senza sapere di mix? Prova una traccia, un ritorno, una sezione, un gruppo
    ripiegato. Segna le frasi che suonano sbagliate o inutili: sono modelli fissi, si correggono uno a uno.
56. **I livelli misurati.** Alla prima apertura "Who is in front" dice che sta misurando: quanto ci mette su un brano
    di 30 stem? Mentre misura la riproduzione resta fluida? I numeri tornano con quello che senti (chi sta davanti,
    quale sezione è la più forte)? Muovi un fader: la parte si rimisura da sola.

## Tour guidato (F19, A-094 … A-097)

57. **Primo avvio su dati puliti.** Chiudi l'app e sposta altrove `~/Library/Application Support/app.stemwise.desktop/`
    (tieni la copia: dentro ci sono i recenti, la chiave in file e il registro dei costi), poi apri l'app. Atteso: la
    scheda "Welcome to Stemwise" compare da sola sulla Welcome, senza attese; `Start` porta al passo 2 col brano
    dimostrativo aperto entro 3 s. Chiudi l'app a metà tour e riaprila: il tour non riparte da solo; nella cartella
    `~/Library/Caches/app.stemwise.desktop/tour` non resta nulla. Rimetti la cartella dei dati al suo posto.
58. **I testi dei quindici passi.** Fai tutto il giro con `Next`, poi torna indietro con `←` fino all'inizio. Ogni
    passo dice la cosa giusta su quello che illumina? Segna i testi da cambiare: stanno tutti in
    `app/ui/src/tour/text.ts` e nelle tavole (`design/tools/s-tour.mjs`). Prova anche a usare l'area illuminata
    (Play, un fader, l'A/B della proposta) e a cliccare fuori: fuori non deve succedere niente.
59. **Il suono del brano sintetico.** Al passo 2 premi Play e ascolta tutto il brano (68 s): batteria, basso,
    chitarre, Rhodes, archi, voce e cori sono riconoscibili come ruoli? Qualcosa dà fastidio (click, fischi, volumi
    sbagliati)? Il riverbero e l'eco si sentono sulla voce? È sintetico per scelta (A-094): dimmi se basta a capire
    cosa fa il mix o se serve di più.
60. **Help › Take the Tour e uscite.** Con un progetto tuo aperto e modificato, `Help › Take the Tour` dal menu dell'app
    (nome del progetto in alto a sinistra): chiede di salvare con la riga sul perché; `Cancel` lascia tutto com'era.
    Durante il tour: `Skip tour`, `Esc`, `Done` e ⌘Q finiscono il tour; dopo Skip ed Esc un toast dice dove ritrovarlo.
    Dopo il tour il progetto dimostrativo non è tra i recenti e AI usage non ha chiamate nuove.
    Al passo 8 (A-099): `Apply all` e `Discard` (con la conferma sopra il tour) lasciano `Next` attivo; arrivandoci
    in fretta, `Cancel` mentre la proposta si prepara non blocca il giro. Con Tab su `Back`, Invio torna indietro;
    Spazio suona anche col fuoco sul fumetto.
61. **Un solo menu dell'app (UI-20).** La barra dei menu di macOS ha solo Stemwise (About, Services, Hide, Quit),
    Edit e Window; copia e incolla nei campi di testo funzionano ancora. Nel menu del nome del progetto: New
    project ⌘N, Open… ⌘O, Project settings, Save ⌘S, Save as… ⇧⌘S, Close project, Settings… ⌘,, Help › Take the Tour.
    ⌘N e ⌘O funzionano anche dalla Welcome.

62. **Landing page (A-100, A-101).** Fatto il 2026-11-16: il sito è su <https://stemwise.pierodetomi.dev> e i suoi
    link a codice, release, `LICENSE` e `.dmg` rispondono da fuori. Resta da caricare l'immagine di anteprima del
    repository (Settings › Social preview, `docs/images/social-preview.png`) e, a ogni nuova release, aggiornare
    l'indirizzo del `.dmg` in `site/config.js` della landing.

63. **Pacchetto scaricato e self-test (A-103).** Scarica il `.dmg` dalla pagina delle release e aprilo come farebbe
    chiunque: deve comparire l'avviso sullo sviluppatore non verificato, poi **Impostazioni di Sistema › Privacy e
    sicurezza › Apri comunque**, mai "danneggiato". Il self-test nel pacchetto è stato ripetuto a Mac fermo il
    2026-11-16: tre volte su tre, 59 su 59.
64. **Installer su Windows (A-104).** Installa `Stemwise_<versione>_x64-setup.exe` da `app/target/release/bundle/nsis`
    su un Windows pulito: SmartScreen avvisa perché non è firmato (**Ulteriori informazioni › Esegui comunque**),
    l'installazione non chiede diritti di amministratore e l'app compare nel menu Start. La finestra non ha una
    barra dei menu; il menu del nome del progetto ha tutti i comandi, con le scorciatoie scritte `Ctrl+…`.
65. **Audio su Windows (A-104).** Su un brano vero: riproduzione senza scatti con il dispositivo predefinito
    (WASAPI), cambio di dispositivo in Impostazioni › Audio, cuffie scollegate durante la riproduzione (13b). Ascolta
    un export AAC a 256 kbit/s, che su Windows usa Media Foundation, accanto allo stesso export fatto su macOS.
66. **Percorsi e trascinamento su Windows (A-104).** Nuovo progetto da file trascinati da Esplora file con lo
    schermo al 125% o al 150%: il riquadro di rilascio sta sotto il puntatore. `Save as…` in una cartella con
    spazi e lettere accentate, poi riapertura dai recenti. Su un portatile al 150% la finestra minima di 1280×720
    potrebbe non entrare nello schermo: dimmi se succede.

## Ordine consigliato

1. Pacchetto e avvio: 23, 22, 40.
2. Motore e ascolto: 1–6, poi 25–26 (riverberi e compressione parallela), 47–51 (catena a blocchi ed effetti nuovi),
   16 e 21 (stati di errore).
3. Assistente col servizio vero: 7–11, poi 12–14, 27–29 (struttura e doppi) e 31–32 (bilanciamento e revisioni), 52–54 (blocchi e idee creative).
   Stato del mix a colpo d'occhio: 33–34. Proposte che restano: 35. Configurazione guidata: 36–38 (prima di 7–11).
4. Master ed export: 17–19, 30, poi 39 (master AI misurato), 41 (musica non rock), 42–45 (catena libera) e 46 (forma d'onda).
5. Impostazioni e accessibilità: 20, 21 (VoiceOver).
6. Brano vero dall'inizio alla fine: 24. La spiegazione del mix: 55–56.
7. Tour guidato e menu dell'app: 57 (per primo, su dati puliti), 58–61.
8. Pulizia: 15.
9. Landing page e pacchetto scaricato: 62, 63.
10. Windows: 64, 65, 66.
