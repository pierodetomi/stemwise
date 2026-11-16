// Screen 18 — the guided tour (P-020, UI-17 → UI-19): a spotlight on the area a step talks about, a bubble beside it,
// fifteen steps on a demo song that the app builds and synthesises itself (PRJ-9). Every step sets the state it shows.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
import { welcome } from './s-start.mjs';
import { proposal } from './s-mix.mjs';
import { explain, SONG, side, headAll, docAll, nav } from './s-explain.mjs';
import { masterChain } from './s-chain.mjs';
import { exportForm } from './s-master.mjs';
const { ic, btn, tag, f1, px } = L;

// ---------- the fifteen steps: final English text (UI-18) ----------
// title, text (one or two sentences), try (optional hint: the lit area can be used), what the step sets up.
export const STEPS = [
  { title: 'Welcome to Stemwise', text: 'A three-minute tour on a demo song: the mix, the assistant, the master and the export. Nothing you change here is saved, and nothing leaves this Mac.',
    sets: 'Welcome screen; the demo song is being prepared in the background.' },
  { title: 'Play the song', text: 'The top bar holds the transport, the position in the song and the meters of the whole mix. The demo keeps playing while you look around.',
    try: 'press Play, or Space, to listen.', sets: 'Mix phase, demo open, stopped at the start of the Chorus.' },
  { title: 'Tracks on a timeline', text: 'Each imported file becomes a track, laid out under the sections of the song. Editing never touches your files, and every change can be undone.',
    sets: 'All panels open, timeline at full zoom.' },
  { title: 'A track', text: 'The header holds the name, the role, mute, solo and the lock that keeps the assistant away. The small labels tell what is working in its chain.',
    sets: 'Lead Vox selected.' },
  { title: 'The mixer', text: 'One strip for each track, group and shared effect: level, pan, the sends to the reverb and the delay, and the meters. The grid button shows every setting at once.',
    try: 'move a fader while it plays.', sets: 'Mixer open on the Faders view.' },
  { title: 'The inspector', text: 'The chain of the selected track, in the order the sound goes through it. Open a block to change it, or switch it off to hear the difference.',
    sets: 'Inspector open on Lead Vox, EQ block open.' },
  { title: 'The assistant', text: 'Fill in the brief, then ask for an Auto mix or for a change in your own words. The model never receives audio, only measurements and names, and you see the cost before anything is sent.',
    sets: 'Assistant on Notes, no proposal yet. In the tour the offline demo answers.' },
  { title: 'A proposal to review', text: 'Nothing changes until you decide. Read why each track changes, compare by ear with A/B, then apply all of it, only the tracks you pick, or none.',
    try: 'switch between Current mix and Proposal while it plays.', sets: 'Proposal of the offline demo in review, A/B on the proposal.' },
  { title: 'One step to undo', text: 'The tour applied the proposal: it is a single step in the history, and one Undo takes all of it back. Snapshots keep named versions to compare.',
    sets: 'Proposal applied, assistant on History.' },
  { title: 'Explain this mix', text: 'Where the sound goes, drawn as a map, and the same mix told in words, built from the project without any model. Click a name to follow its path.',
    sets: 'Explain open on the whole mix.' },
  { title: 'Mix, then Master', text: 'Mix and Master are two phases, each with its own history. Mastering starts from the mixdown, the mix bounced to one stereo file: the tour has made one for you.',
    sets: 'Mix phase, Explain closed; the mixdown is ready or being made.' },
  { title: 'The master chain', text: 'One chain for the whole song, in the order you choose; the limiter and dither always come last. Click a block to open it beside the rack.',
    sets: 'Master phase, Clipper open in the editor.' },
  { title: 'Loudness and A/B', text: 'The target decides how loud the file will be, and the meters show where the master stands. A/B in the top bar compares it with the mixdown, at its real level or at the same loudness.',
    sets: 'Master phase, Loudness card in view, A/B on the master.' },
  { title: 'Export', text: 'Write WAV, FLAC and AAC in one go, with title, artist, ISRC and cover in every file. In the tour nothing is written: here you only look.',
    sets: 'Export dialog open, moved to the right; its Export button is off.' },
  { title: 'Ready for your own songs', text: 'The demo closes and you are back on the Welcome screen. Three places to start:',
    list: [['plus', 'New project', 'Drop a folder of stems on the window.'], ['spark', 'Set up the assistant', 'Connect your OpenRouter account, or use the offline demo.'], ['play', 'Help › Take the Tour', 'Shows this tour again, at any time.']],
    sets: 'Master phase under the dim; Done ends the tour (UI-19).' },
];
const N = STEPS.length;

// ---------- the demo song (PRJ-9): 68 s, three sections, the twelve tracks of the boards ----------
const DSECS = [['Verse', 0, 24, 0.65], ['Chorus', 24, 52, 1], ['Outro', 52, 68, 0.45]];
const DCLIPS = [[[0, 64]], [[0, 64]], [[0, 66]], [[0, 66]], [[0, 24], [24, 64]], [[0, 68]], [[24, 52]], [[24, 52]], [[0, 24], [52, 68]], [[24, 68]], [[2, 22], [26, 50]], [[26, 50]]];
const DROWS = P.defaultRows().map((r) => (r.k === 'a' ? { ...r, vals: [0.5, 0.74, 0.45], by: 'user', sub: 'Drawn by hand' } : r));
const D = { project: 'Demo song', tick: 10, status: 'Tour · nothing is saved', dur: '1:08', T: 68, sections: DSECS, clips: DCLIPS, rows: DROWS, loopAt: [24, 52], playAt: 31.2, pos: '0:31.20', section: 'Chorus', count: '12 tracks · 48 kHz · demo' };
// The master assistant of the demo: no master proposal; the tour never asks one.
const masterBefore = `<aside class="card side" aria-label="Assistant"><div class="ph"><span class="h">Assistant</span><div class="grow"></div>${L.seg(['Notes', 'Proposals', 'History'], 0, 'quiet sm')}</div>
  <div class="sect"><div class="fx g1 wrap">${tag('Custom · −9.6 LUFS · −1 dBTP')}${tag('No references')}${tag('Indie rock')}</div></div>
  <div class="sect"><div class="note colm"><span class="b">No master proposal yet</span><span class="dim">Master with AI measures the mixdown and proposes a chain for the target; a request in words changes only what it names.</span></div></div>
  <div class="foot"><div class="ask"><span class="grow">Tell the assistant what to change: “keep the attack of the loud hits”</span>${ic('send')}</div>
    <div class="fx between"><span class="faint s1">Offline demo in the tour · no cost</span>${btn('Master with AI', 'primary', 'spark')}</div></div></aside>`;
const DM = { project: 'Demo song', dur: '1:08', status: 'Mixdown 1 · demo', pos: '0:31.20', T: 68, sections: DSECS, tick: 10, playAt: 31.2, noRefs: true, asstHtml: masterBefore,
  listen: { names: ['Master', 'Mixdown'], rows: [['Master', '−9.6', 'as exported'], ['Mixdown', '−20.1', '10.5 LU quieter']] } };
// What the boards' song says that the demo must not: costs, references, the author's names.
const tourize = (h) => h
  .replaceAll('Paper Lanterns', 'Demo song').replaceAll('Piero De Tomi', 'Stemwise').replaceAll('Slow Furniture', 'Tour')
  .replaceAll('>2 references<', '>No references<').replaceAll('Mixdown 3', 'Mixdown 1').replaceAll('3:42 · 48 kHz', '1:08 · 48 kHz').replaceAll('12 tracks, 8 sections', '12 tracks, 3 sections').replaceAll('Distance to reference', 'Distance to target')
  .replace(/This request about [^<]*/, 'Offline demo in the tour · no cost')
  .replace(/\$0\.11 · 14\.2k tokens<\/span><a href="12-ai-usage.html" class="s1">Details<\/a>/, 'Offline demo · no cost</span>')
  .replace(/Spent so far on this run \$0\.07/, 'Offline demo · no cost')
  .replace(/AI cost so far \$0\.48 · 8 calls<\/span><a href="12-ai-usage.html" class="s1">AI usage<\/a>/, 'Runs on this Mac: nothing is sent</span>');

// Assistant before any proposal (steps 2–7): brief, nothing applied yet, the request field.
const asstBefore = `${P.brief()}<div class="sect"><div class="note colm"><span class="b">No proposal yet</span><span class="dim">Auto mix asks for the whole mix; a request in words changes only what it names. Either way you review before anything is applied.</span></div></div>${P.askFoot()}`;
// History after the tour applied the proposal (step 9).
const asstHistory = `<div class="sect"><div class="fx between"><span class="faint s1">Every step can be undone. Click a step to go back to it.</span></div>
  <div class="col"><div class="mi is-hover">${ic('spark', 'm aic')}<span class="grow ell">Auto mix applied · 11 tracks</span><span class="s1 faint">now</span></div><div class="mi"><span class="chk"></span><span class="grow ell">Demo song opened</span><span class="s1 faint">0:00</span></div></div></div>
  <div class="foot"><div class="fx g2">${btn('Undo', '', 'undo')}${btn('Redo', 'is-dis', 'redo')}</div><span class="faint s1">Mix and Master keep separate histories. Mute, solo, zoom and selection are not recorded.</span></div>`;
const asstHead = (tab) => `<div class="ph"><span class="h">Assistant</span><div class="grow"></div>${L.seg(['Notes', 'History', 'Snapshots'], tab, 'quiet sm')}</div>`;
const before = (o = {}) => tourize(P.mixScreen({ ...D, notes: false, noWhy: true, asstPanel: `<aside class="card side" aria-label="Assistant">${asstHead(0)}${asstBefore}</aside>`, ...o }));
const after = (o = {}) => tourize(P.mixScreen({ ...D, ...o }));

// ---------- spotlight and bubble ----------
// Placement rule (design/README.md): the hole is the target grown by --tour-pad and kept inside the window; the bubble
// takes the first side that fits in this order — the step's own preference, then right, left, below, above — at
// --tour-gap from the hole and at least --tour-edge from the window, centred on the target (or on its anchor) and slid
// along the side until it fits; the arrow points at the anchor. When no side fits, the bubble sits inside the hole,
// bottom-left, without an arrow.
const PAD = 4, GAP = 12, EDGE = 16, TW = 340, RING = 2;
// Heights of the bubbles as rendered (measured on these boards); the app measures the real bubble before placing it.
const HM = { 2: 217, 3: 215, 4: 215, 5: 238, 6: 215, 7: 215, 8: 238, 9: 194, 10: 215, 11: 215, 12: 194, 13: 236, 14: 194, '8w': 278, '11w': 264 };
const estH = (s, o, n) => HM[n + (o.wait ? 'w' : '')] || estH0(s, o);
const estH0 = (s, o) => {
  const lines = Math.ceil((s.text.length * 7.1) / (TW - 32));
  return 32 + 16 + 8 + 24 + 8 + lines * 21 + (s.try && !o.wait ? 24 : 0) + (o.wait ? 64 : 0) + 48;
};
function place(t, o, s, W, H) {
  const hole = { x: Math.max(RING, t.x - PAD), y: Math.max(RING, t.y - PAD) };
  hole.w = Math.min(W - RING, t.x + t.w + PAD) - hole.x; hole.h = Math.min(H - RING, t.y + t.h + PAD) - hole.y;
  const h = o.h || estH(s, o, o.n), ax = o.ax ?? t.x + t.w / 2, ay = o.ay ?? t.y + t.h / 2;
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v)), r = 24;
  const fits = { r: W - (hole.x + hole.w) >= TW + GAP + EDGE, l: hole.x >= TW + GAP + EDGE, b: H - (hole.y + hole.h) >= h + GAP + EDGE, t: hole.y >= h + GAP + EDGE };
  const side = [o.side, 'r', 'l', 'b', 't'].find((k) => k && fits[k]);
  let x, y, at, a;
  if (side === 'r' || side === 'l') { x = side === 'r' ? hole.x + hole.w + GAP : hole.x - GAP - TW; y = clamp(ay - h / 2, EDGE, H - EDGE - h); at = side === 'r' ? 'l' : 'r'; a = clamp(ay - y, r, h - r); }
  else if (side === 'b' || side === 't') { y = side === 'b' ? hole.y + hole.h + GAP : hole.y - GAP - h; x = clamp(ax - TW / 2, EDGE, W - EDGE - TW); at = side === 'b' ? 't' : 'b'; a = clamp(ax - x, r, TW - r); }
  else { x = hole.x + EDGE; y = hole.y + hole.h - EDGE - h; at = ''; }
  return { hole, x, y, at, a };
}
const bar = (n) => `<span class="tt-bar" aria-hidden="true">${Array.from({ length: N }, (_, i) => `<i class="${i + 1 < n ? 'on' : i + 1 === n ? 'now' : ''}"></i>`).join('')}</span>`;
const counter = (n) => `<div class="tt-c"><span class="nowrap">${n} of ${N}</span>${bar(n)}</div>`;
const keys = `<div class="tt-keys"><span class="kbd">→</span> next <span class="kbd">←</span> back <span class="kbd">Esc</span> leaves the tour · <span class="kbd">Space</span> plays</div>`;
// The inside of a bubble or card: counter, title, text, then the try line, the wait, the footer.
function inner(n, o = {}) {
  const s = STEPS[n - 1], first = n === 1, last = n === N;
  const wait = o.wait ? `<div class="tt-wait" role="status"><span class="fx g2">${ic('clock', 'sm')}<span class="grow">${o.wait[0]}</span><span class="faint">${o.wait[1] || ''}</span></span>${L.progress(o.wait[2] ?? 30, o.wait[3] || 'ai indet')}</div>` : '';
  const foot = first
    ? `<div class="tt-f">${btn('Skip tour', 'quiet')}<span class="grow"></span>${btn('Start', 'primary lg' + (o.focus ? ' is-focus' : ''), 'play')}</div>`
    : `<div class="tt-f">${last ? '' : btn('Skip tour', 'quiet sm')}<span class="grow"></span>${btn('Back', 'sm', 'left')}${last ? btn('Done', 'primary' + (o.focus ? ' is-focus' : ''), 'check') : `<button class="btn primary${o.wait ? ' is-dis' : ''}${o.focus ? ' is-focus' : ''}">Next${ic('right', 'm')}</button>`}</div>`;
  return `${counter(n)}<span class="h${first || last ? ' l' : ''}" id="tt-${n}">${o.title || s.title}</span><p class="tt-p">${o.text || s.text}</p>${s.list ? `<div class="tt-list">${s.list.map((x) => `<div>${ic(x[0], 'm')}<div class="col"><span class="b">${x[1]}</span><span class="dim s1">${x[2]}</span></div></div>`).join('')}</div>` : ''}${s.try && !o.wait ? `<div class="tt-try">${ic('cursor', 'sm')}Try it: ${s.try}</div>` : ''}${wait}${first ? `<span class="faint s1">${o.note || 'The demo song is synthesised on this Mac from simple sounds, so it sounds a little plain.'}</span>${keys}` : ''}${o.foot || foot}`;
}
// A step with a lit area. t: target rectangle in window pixels; o: side, ax/ay (anchor of the arrow), h, wait, focus.
export function step(n, t, o = {}) {
  const W = o.min ? 1280 : 1440, H = o.min ? 720 : 900, s = STEPS[n - 1], p = place(t, { ...o, n }, s, W, H);
  return `<div class="tour" data-step="${n}"><div class="tour-spot" style="left:${px(p.hole.x)};top:${px(p.hole.y)};width:${px(p.hole.w)};height:${px(p.hole.h)}"></div>
    <div class="tour-tip ${p.at ? 'at-' + p.at : ''}" role="dialog" aria-modal="false" aria-labelledby="tt-${n}" style="left:${px(p.x)};top:${px(p.y)};--ax:${px(p.a || 0)}">${inner(n, o)}</div>
    <div class="sr" aria-live="polite">Step ${n} of ${N}: ${s.title}</div></div>`;
}
// The opening and closing cards: centred, no lit area, the whole window dimmed.
export const card = (n, o = {}) => `<div class="tour full" data-step="${n}"><div class="tour-card" role="dialog" aria-modal="false" aria-labelledby="tt-${n}">${inner(n, o)}</div></div>`;
// The same pieces, not positioned, for the component sheet (C8).
export const tipStatic = (n, o = {}) => `<div class="tour-tip static ${o.at ? 'at-' + o.at : ''}" role="dialog" aria-labelledby="tt-${n}" style="--ax:${o.ax || '48px'}">${inner(n, o)}</div>`;
export const cardStatic = (n, o = {}) => `<div class="tour-card static" role="dialog" aria-labelledby="tt-${n}">${inner(n, o)}</div>`;
const over = (screen, layer) => screen.replace(/<\/div>$/, `${layer}</div>`);

// ---------- targets (measured on the boards; the app finds them by role and label, never by id) ----------
const R = {
  top: { x: 0, y: 0, w: 1440, h: 52 }, timeline: { x: 8, y: 52, w: 776, h: 528 }, header: { x: 8, y: 512, w: 344, h: 38 },
  mixer: { x: 8, y: 588, w: 776, h: 304 }, insp: { x: 792, y: 52, w: 316, h: 840 }, asst: { x: 1116, y: 52, w: 316, h: 840 },
  prop: { x: 792, y: 52, w: 640, h: 840 }, explain: { x: 8, y: 52, w: 1424, h: 840 },
  phase: { x: 285, y: 12, w: 135, h: 28 }, play: { x: 472, y: 7, w: 38, h: 38 }, rack: { x: 8, y: 250, w: 776, h: 642 }, loud: { x: 792, y: 52, w: 316, h: 840 },
  dlg: { x: 672, y: 152, w: 760, h: 595 }, headerMin: { x: 8, y: 392, w: 304, h: 26 },
};

// ---------- screens ----------
const welcomeFirst = welcome(true), welcomeBack = welcome(false);
const s1 = over(welcomeFirst, card(1));
const s2 = over(before({ playing: false, pos: '0:24.00', playAt: 24 }), step(2, R.top, { side: 'b', ax: R.play.x + R.play.w / 2 }));
const s3 = over(before(), step(3, R.timeline));
const s4 = over(before(), step(4, R.header));
const s5 = over(before(), step(5, R.mixer));
const s6 = over(before(), step(6, R.insp));
const s7 = over(before(), step(7, R.asst, { ay: 760 }));
const s8 = over(after({ insp: '', asstPanel: tourize(proposal({ ab: 1 })) }), step(8, R.prop));
const s8w = over(before({ asstPanel: tourize(P.assistant({ asst: 'working' })), mid: `<div class="fx g2" style="width:var(--field-w-l)">${tag(ic('spark', 'sm') + 'Auto mix · offline demo', 'ai')}<div class="grow">${L.progress(46, 'ai')}</div></div>` }), step(8, R.asst, { wait: ['Preparing the proposal…', 'a few seconds'] }));
const s9 = over(after({ asstPanel: `<aside class="card side" aria-label="Assistant">${asstHead(1)}${asstHistory}</aside>` }), step(9, R.asst, { ay: 160 }));
const s10 = over(tourize(explain(SONG(), { ...D, side: side(headAll, docAll, { nav: nav(0) }) })), step(10, R.explain));
const s11 = over(after(), step(11, R.phase, { side: 'b' }));
const s11w = over(after({ status: 'Bouncing the mixdown…' }), step(11, R.phase, { side: 'b', wait: ['Bouncing the mix to the mixdown…', '0:03 elapsed', 64, ''] }));
const mst = (o = {}) => tourize(masterChain({ ...DM, ...o }));
const s12 = over(mst(), step(12, R.rack));
const s13 = over(mst(), step(13, R.loud));
const exp = tourize(exportForm()).replace('<div class="scrim">', '<div class="scrim tour-dock">')
  .replace(/<span class="faint s1 grow">3 files · about 12 seconds<\/span>/, '<span class="faint s1 grow">In the tour nothing is written</span>').replace(/<button class="btn primary">(<svg[^]*?<\/svg>)Export 3 files<\/button>/, '<button class="btn primary is-dis" title="Exporting is off in the tour">$1Export 3 files</button>');
const s14 = over(mst({ playing: false, over: exp }), step(14, R.dlg));
const s15 = over(mst({ playing: false }), card(15));
const s4min = over(before({ min: true }), step(4, R.headerMin, { min: true }));

// Where the tour starts from and how it ends.
const tourLink = (hover) => welcomeBack.replace('class="tourlink"', `class="tourlink${hover ? ' is-hover' : ''}"`).replace(/<\/div>$/, `${hover ? '<div class="tip" style="left:calc(var(--sp-8) * 1.4);top:calc(var(--sp-8) * 8.5 + var(--sp-1))">A three-minute tour on a demo song</div>' : ''}</div>`);
// The menu bar of macOS keeps only what macOS itself provides (About, Services, Hide, Quit; Edit for text fields;
// Window). Every command of Stemwise is in the app menu, opened from the project name (A-096).
const macbar = `<div class="macbar"><b>Stemwise</b><span>Edit</span><span>Window</span><span class="grow"></span><span class="faint">Sat 15 Nov 21:52</span></div>`;
const appMenuItems = (dis) => [['New project…', { k: '⌘N' }], ['Open…', { k: '⌘O' }], '-', ['Project settings…'], ['Save', { k: '⌘S' }], ['Save as…', { k: '⇧⌘S' }], ['Close project'], '-', ['Settings…', { k: '⌘,' }], '-', ['Help', { sub: 1, cls: 'is-hover' }]];
const appMenu = (left, top, dis, head = '') => L.menu([...(head ? ['#' + head] : []), ...appMenuItems(dis)], `left:${left};top:${top}`) + L.menu([['Take the Tour', { icon: 'play', cls: dis ? 'is-dis' : 'is-hover' }]], `left:calc(${left} + var(--menu-w) - var(--sp-1));top:calc(${top} + ${head ? 'var(--sp-5) + ' : ''}var(--ctl-h) * 8 + var(--sp-1))`);
const pnameOpen = (h) => h.replace('<span class="h nowrap">Paper Lanterns</span>', `<span class="h nowrap pname-b is-open">Paper Lanterns${ic('chev', 'sm')}</span>`);
const helpBoard = `<div class="macwin">${macbar}${pnameOpen(P.mixScreen({ playing: false, pos: '1:12.00', playAt: 72, section: 'Chorus' }))}${appMenu('40px', 'calc(var(--sp-5) + var(--top-h) - var(--sp-1))', false)}${appMenu('calc(var(--sp-8) * 9)', 'calc(var(--sp-5) + var(--top-h) * 3)', true, 'While the assistant works')}</div>`;
const saveFirst = P.mixScreen({ playing: false, over: L.dialog({ size: 's', title: 'Save changes to “Paper Lanterns”?', sub: 'The tour opens its own demo song, so this project closes first. If you don’t save, the changes since the last save are lost.', foot: `${btn('Cancel', 'quiet')}<span class="grow"></span>${btn('Don’t save', 'danger')}${btn('Save', 'primary')}` }) });
const skipped = welcomeBack.replace(/<\/div>$/, `<div class="toasts">${L.toast('', 'info', 'Tour skipped', 'Take it again at any time from Help › Take the Tour, or from this screen.')}</div></div>`);
const failed = over(welcomeFirst, card(1, { title: 'The demo song can’t be prepared', text: 'Something went wrong while building it on this Mac, so the tour ends here. Everything else works as usual.', note: 'Help › Take the Tour tries again.', foot: `<div class="tt-f">${btn('Copy details', 'quiet')}<span class="grow"></span>${btn('Close', 'primary lg')}</div>` }).replace(/<div class="tt-c">.*?<\/div>/s, ''));

export default {
  'screens/18-tour.html': P.page('18 · Guided tour', 'P-020. The tour that starts by itself on the very first launch and again from Help › Take the Tour or from the Welcome screen. It runs on a demo song that the app builds and synthesises itself (PRJ-9): not a project of the user, never saved, gone when the tour ends. Fifteen steps, moved on with Next: a spotlight on the area a step talks about, which stays usable, and a bubble beside it. Every step sets up the state it shows, forwards and backwards. No model is called: the proposal comes from the offline demo, at no cost.', [
    P.frame('18a', '1 · Opening card', 'On the Welcome screen, as soon as the window is ready on the first launch (the “shown” mark is written now). No lit area: the whole window dims. The demo song is built while this is read, so Start is never a wait; if it is pressed before the song is ready, the next step says so (as in 18p).', s1),
    P.frame('18b', '2 · Top bar and transport', 'The whole top bar is lit; the arrow points at Play. The bubble goes below because nothing fits beside a full-width area. The status says it is the demo and nothing is saved. Play, Space and the loop work; the tour stops nothing.', s2),
    P.frame('18c', '3 · Timeline', 'Clips under the three sections of the demo song (Verse, Chorus, Outro) and the automation lane of Lead Vox. The bubble takes the first side with room: right, over the inspector.', s3),
    P.frame('18d', '4 · Track header', 'Lead Vox is selected by the step. The header is lit with its chain labels and send dots; mute, solo and lock can be pressed.', s4),
    P.frame('18e', '5 · Mixer', 'Faders view. A fader moved here really changes the demo and is lost at the end with everything else. A step sets only what it declares: switch to Overview, go on and come back, and the mixer is on Faders again.', s5),
    P.frame('18f', '6 · Inspector', 'The chain of Lead Vox with its EQ open. The bubble goes left: the inspector has no room on its right.', s6),
    P.frame('18g', '7 · Assistant', 'The brief of the demo is filled in, no proposal yet. The cost line says the offline demo answers: even with a key configured, the tour never calls a model and nothing enters AI usage. The arrow points at the request field.', s7),
    P.frame('18h', '8 · Proposal in review', 'The step opens the proposal of the offline demo, made in the background when the tour started, and turns A/B to the proposal. Apply and Discard work as usual; the next step applies it if it was not.', s8),
    P.frame('18i', '9 · History', 'The proposal is applied: one step, written by the assistant. Undo works; going back to step 8 puts the proposal back in review.', s9),
    P.frame('18j', '10 · Explain this mix', 'The view takes everything under the top bar, so no side has room: the bubble sits inside the lit area, bottom-left, without an arrow.', s10),
    P.frame('18k', '11 · Phase selector and mixdown', 'Back in the Mix with Explain closed. The tour bounced the mixdown in the background after the proposal was applied, so Master opens without the dialog of 8a.', s11),
    P.frame('18l', '12 · Master chain', 'Master phase, the rack beside the editor with the Clipper open. Changing phase is the usual cross-fade; the bubble appears when it is over.', s12),
    P.frame('18m', '13 · Loudness and A/B', 'The Loudness card: target, meters, the Listening rows. A/B in the top bar is named in the text and stays usable, though it is not lit.', s13),
    P.frame('18n', '14 · Export', 'The step opens the export dialog and moves it to the right edge, without its own scrim, so the bubble has room on its left. The primary button is off and says why in its tooltip: the tour writes no file.', s14),
    P.frame('18o', '15 · Closing card', 'Centred, over the dimmed Master. Done, like Skip tour, Esc or quitting the app, stops playback, closes and deletes the demo and goes back to the Welcome screen (UI-19); on a first launch that is 1b, with the invitation to set up the assistant.', s15),
    P.frame('18p', 'A step waiting for the proposal', 'If step 8 is reached before the offline demo has finished, the step lights the assistant at work and says so; Next is off until the proposal is ready, then the step shows it by itself. Back and Skip tour stay available.', s8w),
    P.frame('18q', 'A step waiting for the mixdown', 'The same at step 11 when the bounce is not done yet: the bar is determinate, in the phase accent (it is not the assistant). Next turns on when the mixdown is written; the Master opens at step 12 with the usual preparation if it is still measuring (9d).', s11w),
    P.frame('18r', 'Minimum window · 1280 × 720', 'Step 4 in the compact layout. The lit area is scrolled into view first; the bubble keeps 16 px from the window edges and slides along its side, so it is always inside the window.', s4min),
    P.frame('18s', 'Take the tour from the Welcome screen', 'A link under the two buttons, with how long it takes; its tooltip says what it is. It is there on every launch, also on the first (1b).', tourLink(true)),
    P.frame('18t', 'App menu › Help › Take the Tour', 'Every command of Stemwise lives in the app menu, opened from the project name in the top bar (Mix and Master): New project and Open, the project’s own commands, Settings, and the new Help submenu with Take the Tour. The macOS menu bar keeps only what macOS provides (About, Services, Hide, Quit; Edit for text fields; Window). Take the Tour is off while the assistant works, a bounce runs or an export runs (right). With a project open it goes through 18u. On the Welcome screen, where there is no project name, the link of 18s does the same.', helpBoard),
    P.frame('18u', 'Taking the tour with a project open', 'The tour asks what closing the project asks, with one line about why. Cancel keeps the project and no tour starts; Save or Don’t save close it, and the tour starts on its opening card. At the end the Welcome screen comes back; the project is not reopened.', saveFirst),
    P.frame('18v', 'After Skip tour', 'Skip tour or Esc on any step ends the tour as Done does; then a toast says where to find it again. On a first launch the Welcome screen is 1b, as after Done.', skipped),
    P.frame('18w', 'The demo can’t be prepared', 'If building the demo song fails (a full disk, an error), the opening card says so instead of Start, the tour ends and the app works as usual. No “shown” mark is lost: the tour can be taken again from Help.', failed),
  ]),
};
