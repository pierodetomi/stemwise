# Author validation

What the agent cannot verify on its own (FOUNDATION §8.5): listening, the real model, signing. Each item has
executable steps and the criterion for calling it done. The list is completed at every phase; the recommended order is
at the bottom.

## Audio engine (F3)

1. **Playback with no clicks or gaps.** `cd app && make dev`; open a project with at least 16 tracks (the demo project
   works too: `cargo run -p stemwise-cli -- demo ~/Music --name Prova --seconds 120`). Press Play and
   listen on headphones for two minutes, then: move a fader quickly, toggle mute and solo on several tracks, move the
   bus fader. Expected: no clicks, no dropouts; changes are smooth.
2. **Structural edits while listening.** During playback add a send or group two tracks.
   Expected: a short faded pause (a few tens of ms), then playback resumes from the same point (A-025).
3. **Loop and locate.** Set a 4-bar loop, start it, then click on the ruler outside the loop and inside it.
   Expected: return to the start of the loop with no content glitches; no clicks at the jumps.
4. **Device at a different sample rate, and buffer.** In Settings › Audio pick a 44.1 kHz output with a 48 kHz
   project, and try buffers 128, 512, 2048. Expected: correct pitch, no crackle; with 128 on a slow Mac a larger
   buffer may be needed (note which).
5. **Load with many tracks.** With 32 tracks and full chains, watch the engine load meter in the top bar during
   playback. Expected: under 30% (on the development Mac the benchmark gives 9% average,
   A-023/A-024); no crackle even with other apps open.
6. **Unplugging the device.** During playback unplug the USB headphones or change the system output. Expected:
   the app reports it (UI-6) and does not freeze.

## AI (F5)

The whole AI flow is tested offline with the fake decider and against a fake local HTTP server. What remains to be
verified with the real service, using your key (the agent uses no keys and no network):

7. **First real call and strict schema.** In Settings › AI paste the OpenRouter key, leave the default model and
   run Auto mix on the demo project. Expected: a proposal with a summary and rationales in the language of the brief.
   In the call log (folder `ai/<id>/` of the project, file `request-0-0.json`) check
   `response_format.type`. If it is `json_schema`, the provider accepts the per-project schema (A-031). If it is
   `json_object`, it rejected the schema and the app fell back to JSON mode: it works anyway, but note the model.
   Also note how many requests the first iteration made (a single one is the expected case).
   Keychain (A-052): at launch and when opening projects macOS must not ask for the password. It may ask once,
   at the first call of the session, until the app is signed ("Always Allow" holds until the next
   build).
   Menu (A-053): File › Save, Save As…, Close Project, Open… and New Project… in the menu bar do what
   they say; Edit › Copy and Paste work in text fields.
8. **Costs.** After two or three calls open AI usage. Expected: cost per call equal to that in the OpenRouter panel
   (Activity) to within a few cents; no "estimate" entry if the provider reports the cost. Then cancel
   a call halfway: it must appear with a cost marked as an estimate, and on OpenRouter it must not show as more.
9. **Keychain.** Close and reopen the app: the key is still there (Settings › AI says "Stored in the keychain";
   the key is never shown, not even masked, A-044). In Keychain Access there must be
   an entry "Stemwise". Remove the key from the app: the entry disappears.
10. **No network.** Turn off Wi-Fi and run Auto mix. Expected: a clear, recoverable error (UI-6), no
    freeze; the call appears in AI usage as failed.
11. **Revision in words, for real.** After an applied auto mix, ask "voce un po' più calda nelle strofe".
    Expected: mostly the vocal and the tracks that mask it change; the per-section pushes from the first auto mix stay
    (blue lanes still present).

## Assistant in the app (F7)

12. **A/B of the proposal by ear.** Run Auto mix, then during playback alternate "Current mix" and
    "Proposal" (or press B). Expected: the switch happens without stopping playback, with a short crossfade;
    no new step in History.
13. **Partial apply.** Uncheck two tracks and press "Apply selected". Expected: a single step in History, with
    the spark; Undo brings everything back as it was; in AI usage the call shows as "Applied in part".
14. **History export.** In AI usage › Export choose CSV and open the file in Numbers or Excel. Expected: one row
    per call, with readable costs and requests (quotes in the requests do not break the columns).
15. **Cleanup after the agent's checks.** The first runs of the self-test (before A-041) wrote
    in the app data folder, `~/Library/Application Support/app.stemwise.desktop/`, test recent projects and
    a $0 call from the offline demo (`ai-costs.jsonl`). The agent does not touch files outside the
    repository: if you want, delete the two files before using the app.

## Master and export (F8)

16. **Listening to the master.** Do Bounce & go to Master, then alternate Master, Mixdown and Reference during
    playback. Also use an MP3 or M4A reference at 44.1 kHz.
    - Expected, in "Real level" (default): the master sounds like the exported file, mixdown and reference at their true
      level, the reference at the right speed. The differences you hear are those written in the Listening section of
      the Loudness tab.
    - Expected, with "Same loudness" (or ⇧B): the sources play at the same perceived loudness. If under the rows
      "everything plays X dB lower" appears, everything plays lower by the same amount to avoid clipping.
    - Expected: changing mode or source during playback does not stop it and does not click. The chain must not
      introduce clicks either when you move the controls or turn a block on and off. The maximum sample-to-sample jump is
      verified by a test (A-043), but the ear is what counts.
17. **Targets.** Try Streaming, Loud and CD. Expected: the Integrated reading in the Loudness panel is within ±0.1 LU of the
    target after a few seconds of measurement (A-055); with Loud the limiter works harder but does not distort audibly.
    - With a reference at about −10 LUFS choose Custom at −18 LUFS. Expected: the warning "Much quieter than your
      reference" in the Loudness tab and in the export dialog. "Match the reference" brings the target to the
      reference with the same ceiling, and is a single undo step. "Keep" removes the warning, but the "kept on
      purpose" row stays with "Warn me again".
18. **Exported files.** Export WAV 24-bit, FLAC 16-bit and AAC 256 kbps with title, artist, ISRC and cover. Open the
    three files in Music and in the Finder (Get Info). Expected: they open, the tags and cover appear, the
    duration is right; the report says a true peak below the ceiling for all three.
19. **Stale mixdown.** Go back to Mix, change a fader, then choose Master. Expected: the app offers the bounce;
    after the new bounce the master settings are as before.

20. **Settings (F9).** Open Settings with ⌘, (or with the gear).
    - Audio: change device and buffer. Expected: sound resumes on the chosen device and the engine load
      updates. There is no test signal (A-044): listen to the project.
    - AI: enter the real key. Expected: "Stored in the keychain" appears, the key is never visible.
    - Appearance: the choices persist after restarting the app.

21. **Error states (F9).**
    - Missing files: close the app, delete a file from `audio/` inside the package (Show Package Contents) and
      reopen. Expected: striped, silent clips, a "missing" banner; "Locate…" with the original file restores them. The bounce
      refuses until nothing is missing.
    - Device: during playback unplug the USB headphones or the audio interface. Expected: the banner under the bar,
      playback stopped, "Use …" brings the sound back.
    - Network: with the real key and Wi-Fi off, ask for an Auto mix. Expected: "No connection to OpenRouter" in place
      of the key box, the mix intact.
    - Accessibility: with VoiceOver (⌘F5), navigate the mixer and inspector controls. Expected: every control has a
      name and a value read out; up/down arrows change the values.
22. **Cold start (NFR-3).** Open `Stemwise.app` from the Finder after restarting the Mac. Expected: the window usable
    in under 2 seconds.

## Package (F10)

23. **Installing from the .dmg.** Open `app/target/release/bundle/dmg/Stemwise_0.1.0_aarch64.dmg` and drag Stemwise
    into Applications.
    - The package is not signed or notarized: the agent has no certificates. At first launch use right-click › Open,
      then confirm.
    - Expected: the welcome within 2 seconds (item 22). Create a project with real stems (24-bit WAV and a FLAC). Expected:
      the tracks appear with waveforms and the proposed roles.
    - Distributing it to others requires a Developer ID and notarization (outside the agent's work).
24. **A real song from start to finish.** On a song of yours: brief, Auto mix with the real key, A/B listening,
    apply, a revision in words, bounce, AI master, Streaming target, export WAV 24, FLAC 16 and AAC 256.
    - Expected: the files open in Music, the loudness in the report is −14 ± 0.5 LUFS and the true peak below −1 dBTP.
    - The artistic judgment (does the mix sound better than the raw sum?) is yours: note it with the model used.

## Reverb and parallel compression (F11)

25. **The three reverbs by ear.** In a project with vocal and drums select the reverb return and send the vocal
    at −12 dB. Try Plate (decay 1.2 s), Room (0.6 s) and Hall (2 s) from the selector in the panel; then the same with the
    snare. Expected: three different characters, tails with no metallic sound or repeating echo; no click when changing
    algorithm, Decay or Size during playback. Listen in particular for:
    - Plate under 1 s: a bit grainy is expected (known limit, A-058); tell me whether it is clean from 1 s up;
    - Modulation at 0, 20% and 60% on a long tail (Hall, 4 s): at 0 the tail may "ring" on a note, at 60% it must not
      turn into a chorus;
    - Damping at 0 and at 80%: the tail darkens without shortening in the mids.
26. **Parallel compression.** On the snare (or the drum group) set Threshold −30 dB, Ratio 6, Makeup
    +8 dB, then bring Mix from 100% to 35%. Expected: the hits stay natural and the body grows; no "scooped"
    or metallic sound (dry and compressed are sample-aligned). Move Mix during playback: no click.

## Structural AI mix (F12)

27. **Proposal with structure, by ear.** On a song with vocal and drums on several tracks run Auto mix with
    "Allow structural changes" on. Expected: in the proposal, above the tracks, the "New in this proposal" section with
    the returns and groups the plan creates, each with its reason. Listen in A/B: the vocal in its own space, the drums
    more compact. Then:
    - uncheck a new element and apply: it is not created, the rest is;
    - undo with a single ⌘Z: the new tracks disappear too;
    - rerun Auto mix with the checkbox off and compare the two proposals by ear: this is where you see whether the
      structure really improves the mix.
28. **True doubles.** On a song with a part recorded twice (guitars, backing vocals) with the same role, run Auto
    mix and look in AI usage › detail at the `doubles` entry of the analysis. Expected: the pair is there, not `identical`, and the
    proposal opens them to the sides. Tell me if it recognizes as a double something that is not, or if one is missing: the thresholds
    are initial (A-059).
29. **With the real model.** With the key, run an Auto mix: see how many new things it proposes and whether the rationales hold up.
    Expected: few and useful; if it creates too many or useless ones, note the model and the song.

## New target (A-061)

30. **Changing the target in Master.** On a song of yours, in Master, switch from Streaming to Loud. Expected: after a
    moment the overlay "Bringing the master to the new target" covers the workspace, showing the new target
    and the previous one; playback can be started and stopped and still plays at the previous level; when the
    overlay disappears the level is the new one and the Loudness tab says "On target". Then ⌘Z: it goes back to
    Streaming immediately, with no overlay. To judge: whether the wait is clear and whether the bar, which advances in passes, is
    confusing.

## AI mix balance (A-063)

31. **Balance and effects by ear.** On Balance (or a song with drums on many mics, guitars,
    vocal and backing vocals) run Auto mix with no text. Expected: at least two iterations; in AI usage, in the JSON of the second
    request, `evaluation.balance` with the levels of `drums`, `guitars`, `lead_vocal`, `backing_vocal` and
    `returns_lu`. To judge by ear: the vocal in front of or level with the drums, the guitars present, the backing vocals
    audible, a reverb that can be heard on the vocals. If a ratio does not feel right, write it in the brief notes
    and retry: the ratios in the prompt are starting points to tune to your taste.
32. **Revision on a section beyond two minutes.** Write a request about a section that starts after 2:00
    (for Balance: "chitarre più avanti nel Solo") and press the button, which now says "Revise mix". Expected: the
    proposal says "You asked: …", changes that section (automation lanes) and the fader may move more than
    4 dB if the request asks for it. Enter in the field starts a new line and does not send.

## Mix state at a glance (F13)

33. **Readability on a real 30-track project.** Open Balance (or a song with 30 stems, groups and at least three
    returns) in `make dev`, small lanes (⌥⌘−). Expected: on every header and strip the badges HP LP EQ CMP SAT,
    filled where the block is working, outlined where it is on but changes nothing, dim in bypass; `CMP` in the color of the
    track listened to in sidechain and with two bars if parallel; a dot per return and the automation diamond
    (blue if written by the assistant). In the mixer a send row for each return, with names in the column on the
    left. To judge: whether without selecting anything you can tell who has effects, sends and sidechain, and whether the
    344 px headers take too much space from the lanes (A-066).
34. **Badges, links and Overview.** Click on a badge in a strip: the block goes into bypass and ⌘Z turns it back on in
    one step; double-click: the track is selected and the inspector opens that block. Select a return: the tracks that
    send to it are marked in lime with the level in place of the role, and the return's lane lists them. Select
    a track: its group says "takes …", whoever listens to it "listens to …". At the bottom of the mixer column
    switch to Overview: a click on a cell sets or removes bypass, on an empty send it creates it at −12 dB. In the browser
    (`make ui`) more returns can be added from the console with `__stemwiseMock.simulateReturns(4)`.

## Persistent proposals (A-070)

35. **Reapplying a proposal.** In the real app, on a song of yours: ask for an auto mix and discard it. Open the
    `Proposals` tab of the assistant: the proposal is in the list with date, time and outcome. `Review…`: it comes back into review with the
    note "Received …"; listen to it in A/B, then `Apply all`. Expected: a confirmation saying the current mix will change;
    after confirming the mix is the proposal's and ⌘Z brings it back in one step. Then move a few faders,
    delete a track and reopen the same proposal: the differences are against the current mix and the deleted track
    is declared among the warnings. Close and reopen the project: the proposal can still be reopened. Repeat in Master with
    `Master with AI`. Proposals received before this version are in the list but with the button disabled.

## Guided assistant setup (A-071, A-072)

36. **Connecting the account from the browser.** In `make dev` (unsigned build): Settings › AI › `Disconnect` if there is a
    key, then `Set up the assistant…` › `Connect with OpenRouter`. Expected: openrouter.ai opens on the
    authorization page, with the key name "Stemwise"; after `Authorize` the browser shows the "Connected" page and the
    dialog moves on by itself to `Add credit`. To note: whether the OpenRouter page lets you sign up and sign in
    in the same flow; what happens on refusing (the dialog should say "The connection was not approved"; if it stays
    waiting, the provider does not return after a refusal and that must be stated in 14b); whether the `Add credit` amounts are close
    to the real costs of an auto mix. Then an auto mix: no password request.
37. **Credit exhausted and key revoked.** With a low-limit key (or deleting it from openrouter.ai) ask for
    an auto mix. Expected: in the assistant column "Your OpenRouter credit has run out", "The key has reached its
    own limit" or "OpenRouter no longer accepts this key", each with its own button; the mix does not change and the
    call is at zero cost in AI usage.
38. **Where the key lives.** Unsigned build: Settings › AI says "Stored in a private file of this app"; the file
    is `~/Library/Application Support/app.stemwise.desktop/openrouter.key`, with permissions `-rw-------`. Package
    signed with your Developer ID: it says "Stored in the keychain", and connecting the account and running an auto mix never ask
    for the password, not even after installing a later build signed the same way. The key saved
    in the keychain by earlier unsigned builds is no longer needed: it can be deleted from Keychain Access
    (item "Stemwise").

## Measured AI master (A-077)

39. **AI master on a loud target.** On Balance, in Master, with the target at −9.6 LUFS and the chain
    cleared, ask "mantieni il punch del rullante e la dinamica". Expected: progress goes through several decisions and
    renders; the proposal **does not lower the target**, uses stages before the limiter (fast compressor, saturation, EQ on the
    lows) and the summary says how much the limiter still works. By ear the snare must stay more defined
    than in the limiter-only master. In `ai/<call>/` of the project, `evaluation-0.json` and `evaluation-1.json` show
    `limiter.time_over_3_db_pct` going down. If the target drops anyway or the limiter stays choked, the limit is
    in the tools (the clipper is missing) or in the model: note which of the two the summary says.

## The brand in the app (A-078)

40. **New icon and brand.** After `make build`, open `Stemwise.app`. Expected: in the Dock and in the Finder the icon is the
    aubergine squircle with the two lime lines and the lilac room, recognizable in the Finder list view too
    (16 pt). If the Dock still shows the bars icon, it is the macOS icon cache: drag the app to the Dock Trash
    and reopen it, or restart the Dock. In the top bar of Mix the two lines are lit and the room is dimmed;
    in Master the opposite. On the start screen the brand is in color above the name. When connecting the assistant,
    the final page in the browser has the colored brand next to "Stemwise".

## No genre as the norm (A-080)

41. **AI mix and master on non-rock music.** With two or three songs of different types (for example an acoustic or live-recorded
    jazz, an electronic one, an instrumental), brief with the right genre: ask for auto mix and AI master.
    Expected: on the acoustic the mix moves the faders little and adds no obvious reverbs, and the master stays light
    (little or no compression, no clipping), saying so in the summary; on the electronic, kick and bass drive the
    balance; no summary talks about drums or guitars where there are none. Note where the model
    applies rock habits anyway.

## Free master chain (F14, A-082)

42. **Clipper by ear.** On Balance at −9.6 LUFS, chain cleared: turn on the Clipper and set "Take off up to" to
    4 dB. Expected: in the Loudness tab "Limiter over 3 dB" drops a lot; by ear the hits stay defined and the
    mix no longer "breathes" after each one. Raise it until you hear distortion and see at what percentage of the song it works.
43. **Order and duplicate blocks.** Drag the compressor above the EQ, add a second EQ after the multiband, then
    remove a block. Expected: no glitch while playing, one undo step per move, the names "EQ 1" and "EQ 2".
44. **Multiband.** Turn it on, bring the bands to four, solo the lowest, drag a crossover. Expected:
    the solo is heard immediately and does not end up in the export; with all ratios at 1 the sound does not change.
45. **Sections and AI master with the new tools.** Give +0.5 dB to the choruses and listen to the entry into the chorus:
    no glitch. Then repeat point 39: expected a plan that uses clipper or multiband, possibly a different order
    or an extra block, with the before → after table in the proposal; try applying only some entries.
    Also open a project saved before this version: it must sound as before.

46. **Master waveform.** On the song that at −9 LUFS gave a solid rectangle: after the measurement the waveform
    shows the body of the song in solid color, with verses and choruses distinguishable, and a thin line for the peaks; the
    dark part in the center is the mixdown. Change target: after the new measurement the body grows or shrinks.

## Block-based track chain and new effects (F15, F16, A-087, A-088)

47. **Earlier projects.** Open a project of yours saved before F15 (Balance). Expected: it sounds as before (same
    render: the migration is proven with identical hash on synthetic projects), the mixdown does not need redoing, the automation
    lanes are in place.
48. **Rack in the inspector.** On a track: `Add block` › add a block (it starts off), turn it on, drag it
    above the compressor, duplicate it, reset it, remove it; ⌘Z undoes each step. Expected: no glitch when a block
    is turned on or moved during playback; badges on the headers and strips consistent (four and `+n`).
49. **Correctives by ear.** Gate on a tom or a snare with bleed; de-esser on a sibilant vocal (try
    `Listen` on the detected signal only and the two modes); dynamic EQ on guitars with sidechain from the vocal;
    transient shaper on a snare. Expected: they do what they say without pumping or cutting tails unnaturally.
50. **Color, space, motion, pitch.** Saturation characters (tape, tube, transistor: `soft` stays as
    before), exciter, bass enhancer on a DI bass, width on a pad (the lows stay mono), chorus/flanger/phaser,
    tempo-synced tremolo and auto-pan (the phase stays tied to the bar even after a seek), harmonizer on a vocal (the
    voices do not follow the key, that must be said), doubler, `shimmer` reverb, ping-pong and tape delay, resonant filters.
    Expected: no obvious artifacts; the judgment of the sound is yours.
51. **Return chain.** On a reverb return add a compressor with sidechain from the vocal (ducking):
    the reverb drops while the vocal sings and comes back in the pauses.

## Creative ideas (F17, A-089)

52. **Blocks in the proposal.** Auto mix with the real model on a song with a sibilant vocal and drums with bleed.
    Expected: a de-esser or a gate appear as "new block" rows in the track diff, in the right place in the chain
    after applying. The sibilance threshold (−3 dB in the prompt) is initial: tell me if the de-esser arrives where
    needed and not elsewhere (A-089).
53. **Ideas with the real model.** With "Allow creative ideas" on, auto mix: at most three ideas, different from each other, non-
    corrective, off. Try `Listen` on each (it plays the proposal with that idea alone), check one and apply all:
    a single history step naming the idea; the track note cites it; ⌘Z removes everything. Turn the
    switch off and ask again: no ideas. Also try a revision in words ("qualcosa di più audace nel bridge").
54. **Ideas by ear.** Do the checked ideas improve the song or distract from it? The judgment is yours: it is the part of P-018
    that no measurement verifies.

## Explain this mix (F18, A-092, A-093)

55. **The explanation on a real song.** Open a real project, `Explain` in the top bar (⌘E). Does the map really say
    who goes where? Is the text understandable without knowing mixing? Try a track, a return, a section, a collapsed
    group. Mark the sentences that sound wrong or useless: they are fixed templates, corrected one by one.
56. **The measured levels.** On first opening "Who is in front" says it is measuring: how long does it take on a song
    of 30 stems? While it measures does playback stay smooth? Do the numbers match what you hear (who is in front,
    which section is the loudest)? Move a fader: the part is remeasured by itself.

## Guided tour (F19, A-094 … A-097)

57. **First launch on clean data.** Close the app and move `~/Library/Application Support/app.stemwise.desktop/` elsewhere
    (keep the copy: it holds the recents, the file-based key and the cost log), then open the app. Expected: the
    "Welcome to Stemwise" card appears by itself on the Welcome screen, with no wait; `Start` leads to step 2 with the demo
    song open within 3 s. Close the app halfway through the tour and reopen it: the tour does not restart by itself; in the folder
    `~/Library/Caches/app.stemwise.desktop/tour` nothing is left. Put the data folder back in its place.
58. **The texts of the fifteen steps.** Go through the whole tour with `Next`, then go back with `←` to the beginning. Does each
    step say the right thing about what it lights up? Mark the texts to change: they all live in
    `app/ui/src/tour/text.ts` and in the boards (`design/tools/s-tour.mjs`). Also try using the lit area
    (Play, a fader, the proposal A/B) and clicking outside: nothing must happen outside.
59. **The sound of the synthetic song.** At step 2 press Play and listen to the whole song (68 s): drums, bass,
    guitars, Rhodes, strings, vocal and backing vocals recognizable as roles? Does anything bother you (clicks, whistles, wrong
    volumes)? Can the reverb and the echo be heard on the vocal? It is synthetic by choice (A-094): tell me if it is enough to understand
    what the mix does or if more is needed.
60. **Help › Take the Tour and exits.** With a project of yours open and modified, `Help › Take the Tour` from the app menu
    (project name at the top left): it asks to save with the line explaining why; `Cancel` leaves everything as it was.
    During the tour: `Skip tour`, `Esc`, `Done` and ⌘Q end the tour; after Skip and Esc a toast says where to find it again.
    After the tour the demo project is not among the recents and AI usage has no new calls.
    At step 8 (A-099): `Apply all` and `Discard` (with the confirmation above the tour) leave `Next` enabled; getting there
    in a hurry, `Cancel` while the proposal is being prepared does not block the tour. With Tab on `Back`, Enter goes back;
    Space plays even with focus on the bubble.
61. **A single app menu (UI-20).** The macOS menu bar has only Stemwise (About, Services, Hide, Quit),
    Edit and Window; copy and paste in text fields still work. In the project name menu: New
    project ⌘N, Open… ⌘O, Project settings, Save ⌘S, Save as… ⇧⌘S, Close project, Settings… ⌘,, Help › Take the Tour.
    ⌘N and ⌘O work from the Welcome screen too.

62. **Landing page (A-100, A-101).** Done on 2026-11-16: the site is at <https://stemwise.pierodetomi.dev> and its
    links to code, release, `LICENSE` and `.dmg` respond from outside. Still to do: upload the repository's preview image
    (Settings › Social preview, `docs/images/social-preview.png`) and, at every new release, update the
    `.dmg` address in `site/config.js` of the landing.

63. **Downloaded package and self-test (A-103).** Download the `.dmg` from the releases page and open it as anyone
    would: the warning about an unverified developer must appear, then **System Settings › Privacy &
    Security › Open Anyway**, never "damaged". The self-test in the package was repeated with the Mac idle on
    2026-11-16: three times out of three, 59 out of 59.
64. **Installer on Windows (A-104).** Install `Stemwise_<version>_x64-setup.exe` from `app/target/release/bundle/nsis`
    on a clean Windows: SmartScreen warns because it is not signed (**More info › Run anyway**),
    the installation asks for no administrator rights and the app appears in the Start menu. The window has no
    menu bar; the project name menu has all the commands, with shortcuts written `Ctrl+…`.
65. **Audio on Windows (A-104).** On a real song: playback with no glitches on the default device
    (WASAPI), device change in Settings › Audio, headphones unplugged during playback (13b). Listen to
    an AAC export at 256 kbit/s, which on Windows uses Media Foundation, next to the same export made on macOS.
66. **Paths and drag-and-drop on Windows (A-104).** New project from files dragged from File Explorer with the
    screen at 125% or 150%: the drop box sits under the pointer. `Save as…` into a folder with
    spaces and accented letters, then reopening from the recents. On a laptop at 150% the minimum window of 1280×720
    might not fit on the screen: tell me if that happens.

## Recommended order

1. Package and launch: 23, 22, 40.
2. Engine and listening: 1–6, then 25–26 (reverbs and parallel compression), 47–51 (block chain and new effects),
   16 and 21 (error states).
3. Assistant with the real service: 7–11, then 12–14, 27–29 (structure and doubles) and 31–32 (balance and revisions), 52–54 (blocks and creative ideas).
   Mix state at a glance: 33–34. Persistent proposals: 35. Guided setup: 36–38 (before 7–11).
4. Master and export: 17–19, 30, then 39 (measured AI master), 41 (non-rock music), 42–45 (free chain) and 46 (waveform).
5. Settings and accessibility: 20, 21 (VoiceOver).
6. A real song from start to finish: 24. The mix explanation: 55–56.
7. Guided tour and app menu: 57 (first, on clean data), 58–61.
8. Cleanup: 15.
9. Landing page and downloaded package: 62, 63.
10. Windows: 64, 65, 66.
