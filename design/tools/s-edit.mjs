// Screens 6, 7, 8 — Editing, Returns & routing, Bounce to Master.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
import { relFrames } from './glance.mjs';
const { ic, tag, btn, kb, menu, field, seg, check, px, tgl } = L;

const G = P.geo({}), pps = G.tlw / L.T, HEAD = G.head;
const X = (t) => HEAD + t * pps;              // x of a time inside .tl
const Y0 = 22 + 20;                            // ruler + sections
const callout = (x, y, t) => `<span class="callout" style="left:${px(x)};top:${px(y)}">${t}</span>`;

// ---------- 6 · editing (large lanes, tracks 5–12) ----------
const BIG = [4, 5, 6, 7, 8, 9, 10, 11].map((i) => ({ k: 't', i }));
const yBig = (idx) => Y0 + idx * 60;
const big = (o) => P.mixScreen(Object.assign({ rows: BIG, lane: 60, playing: false, pos: '1:36.00', section: 'Verse 2', playAt: 96, loop: false, noWhy: false, count: 'Tracks 5–12 of 12 · tall lanes' }, o));
const fadeIn = (fw, h) => `<svg class="fade" style="left:0" width="${fw}" height="${h}" aria-hidden="true"><path d="M0 0H${fw}Q${fw * 0.3} ${h * 0.2} 0 ${h}Z"/><path class="ln2" d="M0 ${h}Q${fw * 0.3} ${h * 0.2} ${fw} 0"/></svg><i class="fh is-hover" style="left:${fw}px"></i>`;
const fadeOut = (fw, h) => `<svg class="fade" style="right:0" width="${fw}" height="${h}" aria-hidden="true"><path d="M${fw} 0H0Q${fw * 0.7} ${h * 0.2} ${fw} ${h}Z"/><path class="ln2" d="M0 0Q${fw * 0.7} ${h * 0.2} ${fw} ${h}"/></svg><i class="fh" style="right:${fw}px;transform:translate(50%,-25%)"></i>`;
const ghost = (a, b) => `<div class="clip is-ghost" style="left:${px(a * pps)};width:${px((b - a) * pps - 1)}"></div>`;

const editing = [
  P.frame('6a', 'Select tool · marquee selection', 'Dragging on empty lane draws a marquee; every clip it touches is selected. Shift-click adds or removes a clip.', big({
    clipMod: (t, c) => ((t === 6 || t === 7) && c === 1) || (t === 8 && c === 2) || t === 9 ? { cls: 'is-sel' } : null,
    tlOver: `<div class="marquee" style="left:${px(X(106))};top:${px(yBig(2) + 6)};width:${px(60 * pps)};height:${px(60 * 4 - 18)}"></div>${callout(X(106) + 60 * pps + 8, yBig(5) + 30, '5 clips')}`,
  })),
  P.frame('6b', 'Dragging a selection · snap', 'Two selected clips move together; ghosts stay where they started. The snap line and the label show what the edge snapped to.', big({
    clipMod: (t, c) => ((t === 6 || t === 7) && c === 1 ? { cls: 'is-sel', dx: 8 * pps } : null),
    laneExtra: (t) => (t === 6 || t === 7 ? ghost(112, 144) : ''),
    tlOver: `<div class="snapline" style="left:${px(X(120))};top:${px(Y0)}"></div>${callout(X(120) + 8, yBig(4) + 8, '+0:08.0 · snapped to grid')}`,
  })),
  P.frame('6c', 'Split tool and trimming', 'Split tool active: a cut line follows the pointer and snaps. On the right, the end of a clip is being trimmed; the ghost shows the original length.', big({
    tool: 1, clipMod: (t, c) => (t === 9 && c === 1 ? { cls: 'is-sel', c1: 196, extra: '<i class="trim r"></i>' } : t === 4 && c === 0 ? { cls: 'is-hover' } : null),
    laneExtra: (t) => (t === 9 ? ghost(196, 208) : ''),
    tlOver: `<div class="cutline" style="left:${px(X(64))};top:${px(yBig(0))};height:60px"></div>${callout(X(64) + 8, yBig(0) + 22, ic('cut', 'sm') + ' Split at 1:04.0')}${callout(X(196) - 96, yBig(5) + 30, 'End −0:12.0')}`,
  })),
  P.frame('6d', 'Fades, clip gain, muted clip', 'Fade handles sit at the top corners of a clip; the curve is linear or equal power. Clip gain shows as a small badge. A muted clip dims and loses its colour strip.', big({
    clipMod: (t, c) => (t === 8 && c === 1 ? { cls: 'is-sel', extra: fadeIn(22, 43) + fadeOut(30, 43) } : t === 9 && c === 0 ? { extra: '<span class="gain">+2.0 dB</span>' } : t === 10 && c === 2 ? { cls: 'is-muted' } : null),
    tlOver: `<div class="tip" style="left:${px(X(80) + 30)};top:${px(yBig(4) - 30)}">Fade in · 0.9 s · equal power <span class="faint">(⌥-click: linear)</span></div>`,
  })),
  P.frame('6e', 'Context menus', 'Right-click on a clip and on a track header. Both menus are shown together here; in the app only one is open at a time.', big({
    clipMod: (t, c) => (t === 5 && c === 0 ? { cls: 'is-sel' } : null),
    tlOver: menu([['Split at playhead', { icon: 'cut', k: '⌘T' }], ['Duplicate', { icon: 'copy', k: '⌘D' }], ['Mute clip', { k: 'M', cls: 'is-hover' }], '-', ['Fade in…'], ['Fade out…'], ['Clip gain…', { k: '⌘G' }], ['Rename…'], '-', ['Delete', { icon: 'trash', k: '⌫', cls: 'danger' }]], `left:${px(X(70))};top:${px(yBig(1) + 30)}`)
      + menu([['Rename…'], ['Colour', { sub: 1 }], ['Role · electric gtr', { sub: 1 }], ['Output · Mix', { sub: 1 }], '-', ['Lock for AI', { chk: false, icon: 'lock' }], ['Show automation', { sub: 1 }], '-', ['Duplicate track'], ['Group selected tracks', { icon: 'group', k: '⌘⇧G' }], '-', ['Delete track…', { icon: 'trash', cls: 'danger' }]], `left:${px(40)};top:${px(yBig(1) + 4)}`),
  })),
  P.frame('6f', 'Automation tool · points, range, sections', 'Automation tool active on a lane drawn by hand (lime, not the assistant’s blue). Four points are selected by range; a point is hovered. In the sections lane one section is being renamed and one boundary dragged.', P.mixScreen({
    tool: 2, playing: false, loop: false, pos: '2:24.00', playAt: 144, section: 'Bridge',
    rows: L.tracks.flatMap((t, i) => (i === 10 ? [{ k: 't', i }, { k: 'a', i, by: 'user', label: 'Fader', sub: 'Drawn by hand · 12 points', selPts: [5, 6, 7, 8], hoverPt: 10, val: '−4.2 dB',
      extra: `<div class="marquee" style="left:${px(108 * pps)};top:4px;width:${px(64 * pps)};height:48px"></div>` }] : [{ k: 't', i }])),
    secCls: { 5: 'is-edit', 6: 'is-sel' }, secExtra: { 6: '<i class="grip"></i>' },
    tlOver: callout(X(208) - 136, 46, 'Chorus ends 3:28.0'),
  })),
];

// ---------- 7 · returns & routing ----------
const returnPanel = (kind) => {
  const rev = kind === 'reverb';
  return `<aside class="card side tb" aria-label="Inspector">
    <div class="ph">${ic('wave')}<span class="h">${rev ? 'Plate' : 'Echo'}</span>${tag(rev ? 'reverb return' : 'delay return')}<div class="grow"></div><button class="route">${ic('right', 'sm')}Mix${ic('chev', 'sm')}</button></div>
    <div class="sect"><div class="module is-open"><div class="mh2">${tgl('by', true)}<span class="b grow">${rev ? 'Reverb' : 'Delay'}</span>${rev ? seg(['Plate', 'Room', 'Hall'], 0, 'quiet sm', 'Algorithm') : seg(['ms', '1/8', '1/8.', '1/4'], 2, 'quiet sm', 'Time unit')}</div>
        <div class="knobs">${rev ? kb('Decay', '1.2 s', 0.54, { size: 'l' }) + kb('Size', '50 %', 0.5, { size: 'l' }) + kb('Pre-delay', '30 ms', 0.3, { size: 'l' }) : kb('Time', '375 ms', 0.36, { size: 'l' }) + kb('Feedback', '25 %', 0.36, { size: 'l' }) + kb('Level', '−9.0 dB', 0.62, { size: 'l' })}</div>
        ${rev ? `<div class="knobs">${kb('Damping', '50 %', 0.5)}${kb('Modulation', '20 %', 0.2)}</div><span class="faint s1">Dense and bright from the first instant, no early reflections. At its best from about 1 s up.</span>` : '<span class="faint s1">Dotted eighth at 120 BPM. Note values appear when the project has a tempo.</span>'}</div>
      <div class="module"><div class="mh2">${tgl('by', true)}<span class="b grow">${rev ? 'Return filter' : 'Feedback filters'}</span></div>
        <div class="fx g3">${L.flabel('High-pass', field(rev ? '300' : '200', 'Hz'))}${L.flabel('Low-pass', field(rev ? '12.0' : '6.0', 'kHz'))}${rev ? L.flabel('Return level', field('−6.0', 'dB')) : ''}</div></div></div>
    <div class="sect"><div class="fx between"><span class="b">Tracks sending here</span><span class="faint s1">post-fader</span></div>
      ${(rev ? [[11, 'Lead Vox', '−14.0', 62], [2, 'Snare', '−18.0', 48], [6, 'Acoustic', '−20.0', 40], [12, 'BVs', '−12.0', 70]] : [[11, 'Lead Vox', '−22.0', 38], [9, 'Rhodes', '−16.0', 55]]).map((s) => `<div class="fx g2 t${s[0]}"><i class="tdot"></i><span class="ell" style="width:var(--sp-8)">${s[1]}</span><div class="hbar grow"><i style="width:${s[3]}%;background:var(--tc)"></i></div><span class="s1 b" style="width:var(--sp-7);text-align:right">${s[2]} dB</span></div>`).join('')}</div>
    <div class="foot"><div class="fx between"><span class="faint s1">Returns 2 of 8</span>${btn('Delete return…', 'quiet sm', 'trash')}</div></div></aside>`;
};
const addMenu = menu(['#Add to the project', ['Audio track from file…', { icon: 'file' }], ['Group', { icon: 'group', k: '⌘⇧G' }], '-', ['Reverb return', { icon: 'wave', cls: 'is-hover' }], ['Delay return', { icon: 'wave' }], '#2 of 8 returns in use'], `right:${px(12)};top:${px(34)}`);
const NEST = [{ k: 'g', n: 'Drums', sub: 'group · 2 tracks, 1 group' }, { k: 't', i: 0, d: 1 }, { k: 't', i: 1, d: 1 }, { k: 'g', n: 'Cymbals & room', d: 1, sub: 'group · 2 tracks' }, { k: 't', i: 2, d: 2 }, { k: 't', i: 3, d: 2 },
  { k: 't', i: 4 }, { k: 'g', n: 'Guitars', sub: 'group · 3 tracks · folded', open: false, spans: [[0, 222]] }, { k: 't', i: 8 }, { k: 't', i: 9 }, { k: 'g', n: 'Vocals', sub: 'group · 2 tracks', spans: [[16, 208]] }, { k: 't', i: 10, d: 1 }, { k: 't', i: 11, d: 1 }];
const nestStrips = () => { const a = P.allStrips(), t = (i, parent) => Object.assign({}, a[i], { child: true, parent }), g = (n, parent) => ({ n, tint: 'tb', bus: true, kind: parent ? '→ ' + parent : 'group', pan: 0.5, pk: 64, fy: 70, gr: 18, db: '−4.0', s1: 0, s2: 0 });
  return [g('Drums'), t(0, 'Drums'), t(1, 'Drums'), g('Cymbals', 'Drums'), t(2, 'Cymbals'), t(3, 'Cymbals'), a[4], g('Guitars'), a[8], a[9], g('Vocals'), t(10, 'Vocals'), t(11, 'Vocals'), a[14], a[15], a[16]]; };
const routeMenu = menu(['#Send the output of Lead Vox to', ['Mix bus', { chk: false }], ['Drums', { chk: false }], ['Cymbals & room', { chk: false }], ['Guitars', { chk: false }], ['Vocals', { chk: true, cls: 'on' }], '-', ['New group from selection', { icon: 'plus' }], '#Groups nest up to 4 levels. A choice that would create a loop is not offered.'], `right:${px(12)};top:${px(40)}`);
const scMenu = menu(['#Compressor listens to', ['None (this track)', { chk: false }], ['Kick', { chk: true, cls: 'on' }], ['Snare', { chk: false }], ['Drums · group', { chk: false }], ['Lead Vox', { chk: false }], '-', `<div class="mi tall">${L.flabel('Detector high-pass', field('120', 'Hz'))}</div>`, '#Post-fader signal. Bass DI itself and anything fed by it are not listed.'], `right:${px(12)};top:${px(318)}`);
const paramMenu = menu(['#Automate on Bass DI', ['Fader', { chk: true, cls: 'on' }], ['Pan', { chk: false }], ['Trim', { chk: false }], ['Sends', { chk: false, sub: 1 }], ['Filters', { chk: false, sub: 1 }], ['EQ', { chk: false, sub: 1, cls: 'is-hover' }], ['Compressor', { chk: false, sub: 1 }], ['Saturation', { chk: false, sub: 1 }], '-', ['Mute', { chk: false }], ['Bypass', { chk: false, sub: 1 }]], `left:${px(40)};top:${px(Y0 + 5 * 38 + 44)}`)
  + menu(['#Band 2 · 800 Hz', ['Frequency', { chk: false }], ['Gain', { chk: false, cls: 'is-hover' }], ['Q', { chk: false }]], `left:${px(40 + 224)};top:${px(Y0 + 5 * 38 + 44 + 150)}`);

const routing = [
  P.frame('7a', 'Reverb return', 'A return is selected in the mixer: the inspector shows its processor, its filter, and which tracks send to it.', P.mixScreen({ sel: -1, mixer: P.mixer({ sel: -1, strips: P.allStrips().map((s) => (s.n === 'Plate' ? Object.assign(s, { cls: 'is-sel' }) : s)) }), insp: returnPanel('reverb') })),
  P.frame('7b', 'Delay return · adding a return', 'The delay panel with musical time values, and the Add menu open from the timeline toolbar. Up to 8 returns.', P.mixScreen({ sel: -1, mixer: P.mixer({ sel: -1, strips: P.allStrips().map((s) => (s.n === 'Echo' ? Object.assign(s, { cls: 'is-sel' }) : s)) }), insp: returnPanel('delay'), tlOver: addMenu })),
  P.frame('7c', 'Output selector and nested groups', 'Groups nest in the timeline (indent, fold arrow, summary bar) and in the mixer (group strip followed by its members, with the destination under each name). The output menu of Lead Vox is open.', P.mixScreen({ rows: NEST, noAuto: true, kinds: true, strips: nestStrips(), out: 'Vocals', routeOpen: true, inspOver: routeMenu, noWhy: true, count: '12 tracks · 4 groups' })),
  P.frame('7d', 'Sidechain source and automation parameter', 'Bass DI is selected. Its compressor listens to Kick (indicator with the source colour) and the source menu is open. On the left, the menu that picks which parameter a lane automates.', P.mixScreen({ sel: 4, ti: 4, noWhy: true, chain: [['Trim', '−1.2 dB'], ['HPF', '40 Hz'], ['LPF', 'Off', 'is-off'], ['EQ', '2 bands'], ['Comp', '4.0:1', 'is-open'], ['Sat', 'Off', 'is-off'], ['Fader', '−7.2'], ['Sends', '0']],
    sc: `<button class="sc on t1">${ic('link', 'sm')}Sidechain · Kick${ic('chev', 'sm')}</button>`, inspOver: scMenu,
    rows: L.tracks.flatMap((t, i) => (i === 4 ? [{ k: 't', i }, { k: 'a', i, by: 'user', vals: [0.6, 0.6, 0.5, 0.6, 0.5, 0.6, 0.5, 0.6], picker: `<span class="select is-open grow"><span class="ell">Fader</span>${ic('chev', 'm')}</span>`, val: '−7.2 dB' }] : [{ k: 't', i }])),
    tlOver: paramMenu })),
  P.frame('7e', 'Parallel compression', 'Snare is selected. The Mix of its compressor is below 100 %: the rest is the uncompressed signal, blended back in sample-aligned, so heavy settings add density while the hits stay natural. Mix is a parameter like the others: it can be automated.', P.mixScreen({ sel: 1, ti: 1, noWhy: true, mix: '35',
    chain: [['Trim', '0.0 dB'], ['HPF', '90 Hz'], ['LPF', 'Off', 'is-off'], ['EQ', '2 bands'], ['Comp', '3.0:1 · 35 %', 'is-open'], ['Sat', 'Off', 'is-off'], ['Fader', '−5.0'], ['Sends', '1']] })),
  ...relFrames(returnPanel, [['Trim', '0.0 dB'], ['HPF', '90 Hz'], ['LPF', 'Off', 'is-off'], ['EQ', '2 bands'], ['Comp', '3.0:1 · 35 %', 'is-open'], ['Sat', '3.0 dB'], ['Fader', '−8.0'], ['Sends', '1']]),
];

// ---------- 8 · bounce to master ----------
const base = (over) => P.mixScreen({ playing: false, pos: '0:00.00', playAt: 0, section: 'Intro', over });
const kv = (rows) => `<div class="mtable" style="grid-template-columns:1fr auto">${rows.map((r) => `<span class="dim">${r[0]}</span><span class="r b">${r[1]}</span>`).join('')}</div>`;
const bounce = [
  P.frame('8a', 'Confirm', 'Opens when Master is chosen in the phase selector and there is no up-to-date mixdown, or from File › Bounce. Going to Master is always this explicit step.', base(L.dialog({ title: 'Bounce & go to Master', sub: 'The mix is rendered offline to a file inside the project. Mastering works on that frozen mixdown, not on the live mix.',
    body: kv([['Length', '3:42'], ['Format', '48 kHz · 32-bit float · stereo'], ['Estimated time', 'about 9 seconds'], ['Headroom', 'turned down only if above −3 dBTP']]) + `<div class="tile row">${ic('info')}<span class="dim s1">You can come back to Mix at any time. If the mix changes later, Master will tell you and offer to bounce again, keeping your master settings.</span></div>`,
    foot: `${btn('Cancel', 'quiet')}${btn('Bounce & go to Master', 'primary')}` }))),
  P.frame('8b', 'Progress', 'Offline render, faster than real time, can be cancelled.', base(L.dialog({ size: 's', title: 'Bouncing the mix', sub: 'Rendering offline · 24× real time', body: `${L.progress(62)}<div class="fx between"><span class="dim">2:18 of 3:42</span><span class="b">62 %</span></div>`, foot: btn('Cancel', '') }))),
  P.frame('8c', 'Result', 'Measures of the new mixdown. The primary button already wears the Master accent: this is the moment the phase changes.', base(`<div data-phase="master" style="display:contents">${L.dialog({ title: 'Mixdown 3 is ready', sub: 'Saved in the project. These are its measures before any mastering.',
    body: `<div class="fx g2 stretch"><div class="stat"><span class="faint s1">Integrated loudness</span><span class="v">−18.6 LUFS</span></div><div class="stat"><span class="faint s1">True peak</span><span class="v">−3.0 dBTP</span></div><div class="stat"><span class="faint s1">Loudness range</span><span class="v">7.4 LU</span></div></div>`
      + kv([['Headroom trim', '−0.8 dB applied to reach −3 dBTP'], ['Render time', '9.2 s · 24× real time'], ['From mix state', 'step 31 · Lead Vox · EQ band 2 gain']]),
    foot: `${btn('Stay in Mix', 'quiet')}${btn('Open Master', 'primary')}` })}</div>`)),
];

export default {
  'screens/06-editing.html': P.page('6 · Editing', 'Tools (select, split, automation), drag states, fades, snap, multiple selection and context menus. Frames 6a–6e use tall lanes (vertical zoom) on tracks 5–12 so the gestures are legible.', editing),
  'screens/07-returns-routing.html': P.page('7 · Returns & routing', 'Return panels, creating and deleting a return, output selector, nested groups in timeline and mixer, sidechain source, and the parameter picker of an automation lane.', routing),
  'screens/08-bounce-to-master.html': P.page('8 · Bounce to Master', 'The explicit passage from Mix to Master: confirm, progress, result with the measures of the mixdown.', bounce),
};
