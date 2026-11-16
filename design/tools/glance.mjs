// Stemwise design boards — reading the state of a mix without selecting anything (P-013): the Overview in the mixer,
// a large project at the small lane height, what the selection is connected to, and the sheet of the marks.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
const { ic, tgl } = L;

// ---------- a 33-track project ----------
const RET4 = ['Plate', 'Room', 'Slap', 'Throw'];
const SET = [
  ['Lead Vox', 'lead vocal', 11, -5.8, [1, 0, 1, 1, 1], [[62, '−14'], 0, [45, '−19'], [30, '−24']], { au: ['ai', 'Fader'], note: 4, out: 'Vocals' }],
  ['Vox Dbl', 'lead vocal', 11, -11, [1, 0, 1, 1, 0], [[55, '−16'], 0, 0, 0], { out: 'Vocals' }],
  ['Harmony 1', 'backing vox', 12, -13, [1, 1, 1, 1, 0], [[70, '−12'], 0, 0, 0], { out: 'BVs' }],
  ['Harmony 2', 'backing vox', 12, -13.5, [1, 1, 1, 1, 0], [[70, '−12'], 0, 0, 0], { out: 'BVs' }],
  ['BV Low', 'backing vox', 12, -14, [1, 0, 2, 1, 0], [[66, '−13'], 0, 0, 0], { out: 'BVs', lock: 1 }],
  ['BV High', 'backing vox', 12, -14.5, [1, 0, 1, 0, 0], [[66, '−13'], 0, 0, 0], { out: 'BVs', lock: 1 }],
  ['Kick In', 'kick', 1, -6.5, [1, 0, 1, 1, 0], [0, 0, 0, 0], { note: 1, out: 'Drums' }],
  ['Kick Out', 'kick', 1, -9, [0, 1, 1, 0, 0], [0, 0, 0, 0], { out: 'Drums' }],
  ['Snare Top', 'snare', 2, -8, [1, 0, 1, 1, 1], [0, [48, '−18'], 0, 0], { par: 35, out: 'Drums' }],
  ['Snare Btm', 'snare', 2, -14, [1, 0, 2, 0, 0], [0, 0, 0, 0], { out: 'Drums' }],
  ['Hat', 'hi-hat', 3, -16, [1, 0, 0, 0, 0], [0, 0, 0, 0], { out: 'Drums' }],
  ['Tom 1', 'toms', 3, -12, [1, 0, 1, 1, 0], [0, [35, '−22'], 0, 0], { out: 'Drums' }],
  ['Tom 2', 'toms', 3, -12, [1, 0, 1, 1, 0], [0, [35, '−22'], 0, 0], { out: 'Drums' }],
  ['OH L', 'overheads', 4, -11.5, [1, 0, 1, 0, 0], [0, 0, 0, 0], { out: 'Drums' }],
  ['OH R', 'overheads', 4, -11.5, [1, 0, 1, 0, 0], [0, 0, 0, 0], { out: 'Drums' }],
  ['Room', 'room', 4, -16, [0, 0, 2, 1, 0], [0, 0, 0, 0], { out: 'Drums' }],
  ['Bass DI', 'bass', 5, -7.2, [1, 0, 1, 1, 0], [0, 0, 0, 0], { sc: 0, note: 2, au: ['user', 'Fader'] }],
  ['Bass Amp', 'bass', 5, -10, [1, 1, 1, 0, 1], [0, 0, 0, 0], {}],
  ['Acoustic L', 'acoustic gtr', 6, -12.4, [1, 0, 1, 0, 0], [0, [40, '−20'], 0, 0], { out: 'Guitars' }],
  ['Acoustic R', 'acoustic gtr', 6, -12.4, [1, 0, 1, 0, 0], [0, [40, '−20'], 0, 0], { out: 'Guitars' }],
  ['Gtr Left', 'electric gtr', 7, -10.1, [1, 1, 1, 0, 0], [0, 0, 0, 0], { note: 3, out: 'Guitars' }],
  ['Gtr Right', 'electric gtr', 8, -10.3, [1, 1, 1, 0, 0], [0, 0, 0, 0], { out: 'Guitars' }],
  ['Gtr Lead', 'electric gtr', 8, -9, [1, 0, 1, 1, 1], [[30, '−24'], 0, 0, [52, '−17']], { out: 'Guitars', au: ['ai', 'Throw send'] }],
  ['Piano', 'keys', 9, -13, [1, 0, 1, 1, 0], [0, [38, '−21'], 0, 0], {}],
  ['Rhodes', 'keys', 9, -14, [0, 0, 1, 0, 0], [0, 0, 0, [44, '−19']], {}],
  ['Pad', 'synth', 10, -17, [1, 1, 0, 0, 0], [0, 0, 0, 0], {}],
  ['Strings', 'strings', 10, -15.5, [1, 0, 2, 0, 0], [[42, '−20'], 0, 0, 0], {}],
].map((a, i) => Object.assign({ n: a[0], r: a[1], tint: a[2], db: a[3], lv: 0.4 + ((i * 37) % 40) / 100, pan: 0, ch: a[4], sn: a[5], v: ['90', '9k', '2 bands', '3:1', '3 dB'] }, a[6]));
const idx = (n) => SET.findIndex((t) => t.n === n);
SET.forEach((t) => { if (t.sc !== undefined) t.sc = 0; });
const CLIPS = SET.map((t, i) => L.clipTimes[[10, 10, 11, 11, 11, 11, 0, 0, 1, 1, 2, 6, 6, 2, 2, 3, 4, 4, 5, 5, 6, 7, 6, 8, 8, 9, 9][i]]);
const GRP = (n, members, extra = {}) => Object.assign({ k: 'g', n, sub: `group · ${members} tracks`, gl: { ch: [0, 0, 1, 1, 0], sn: [0, 0, 0, 0] } }, extra);
const BIGROWS = [GRP('Vocals', 2, { gl: { ch: [1, 0, 1, 1, 0], sn: [[50, '−18'], 0, 0, 0] } }), { k: 't', i: 0, d: 1 }, { k: 't', i: 1, d: 1 },
  GRP('BVs', 4, { gl: { ch: [1, 0, 1, 1, 0], sn: [[60, '−15'], 0, 0, 0] } }), ...[2, 3, 4, 5].map((i) => ({ k: 't', i, d: 1 })),
  GRP('Drums', 10, { gl: { ch: [0, 0, 1, 1, 0], par: 50, sn: [0, 0, 0, 0] } }), ...[6, 7, 8, 9, 10, 11, 12, 13, 14].map((i) => ({ k: 't', i, d: 1 })), { k: 't', i: 16 }];
const bigStrips = () => {
  const tr = (i) => { const t = SET[i]; return { n: t.n, tint: 't' + t.tint, pan: 0.5, pk: Math.round(t.lv * 100), fy: L.fpos(t.db), gr: t.ch[3] === 1 ? 24 : 0, db: L.dB(t.db), i, ch: t.ch, v: t.v, sc: t.sc, par: t.par, sn: t.sn, au: t.au, out: t.out, parent: t.out, child: !!t.out }; };
  const g = (n, gl) => Object.assign({ n, tint: 'tb', bus: true, kind: 'group', pan: 0.5, pk: 64, fy: 70, gr: 18, db: '−4.0', v: ['', '', '1 band', '2:1', ''] }, gl);
  return [g('Vocals', BIGROWS[0].gl), tr(0), tr(1), g('BVs', BIGROWS[3].gl), tr(2), tr(3), tr(4), tr(5), g('Drums', BIGROWS[8].gl), ...[6, 7, 8, 9, 10, 11, 12, 13, 14, 15].map(tr),
    { n: 'Mix', tint: 'tm', bus: true, kind: 'mix bus', pan: 0.5, pk: 80, fy: L.fpos(0), gr: 0, db: '0.0', mix: true }];
};
const bigProject = (o = {}) => P.mixScreen(Object.assign({ tracks: SET, clips: CLIPS, rows: BIGROWS, lane: 26, sel: 0, returns: RET4, strips: bigStrips(), kinds: true, count: '27 tracks · 5 groups · 4 returns', engine: '31 %', noAuto: true }, o));

export const mixFrames = [
  P.frame('3e', 'Overview in the mixer', 'The switch at the foot of the mixer’s rail swaps the faders for a grid on the same columns: one row per chain block, one per return, then output and automation. Solid: working, with its main value. Outlined: on but neutral. Bass DI’s compressor shows the colour of the track it listens to; Snare’s its parallel mix. Click toggles a block, double-click opens it in the inspector.', P.mixScreen({ view: 'grid', hover: ['Bass DI', 3] })),
  P.frame('3f', 'A large project · small lanes', '27 tracks, 5 groups, 4 returns, small lanes. The role line hides, the marks stay: the five chain labels, a diamond for automation (blue when the assistant wrote it), one dot per return. Group headers carry the marks of the group’s own chain. The mixer scrolls sideways; rail and mix bus stay put.', bigProject()),
  P.frame('3g', 'A large project · Overview', 'The same project in the Overview. With four returns there are four send rows; the grid scrolls sideways with the strips and the labels stay put.', bigProject({ view: 'grid' })),
];

// ---------- what the selection is connected to ----------
export const PLATE = { k: 'r', n: 'Plate', sub: 'plate reverb · 1.2 s', feeds: [[11, 'Lead Vox', '−14.0'], [2, 'Snare', '−18.0'], [6, 'Acoustic', '−20.0'], [12, 'BVs', '−12.0']] };
export const ECHO = { k: 'r', n: 'Echo', sub: 'delay · 1/8.', feeds: [[11, 'Lead Vox', '−22.0'], [9, 'Rhodes', '−16.0']] };
const mark = (names, o = {}) => P.allStrips().map((s) => (names.includes(s.n) ? Object.assign(s, { cls: 'is-rel' }) : s.n === o.sel ? Object.assign(s, { cls: 'is-sel' }) : s));
export const relFrames = (returnPanel, snareChain) => [
  P.frame('7f', 'What feeds a return', 'Plate is selected. Every track that sends to it is marked in lime: header edge, send level in place of the role, its dot for Plate; in the mixer an outline and its Plate send bar. A return’s lane always lists its senders with their levels, selected or not.', P.mixScreen({ sel: -1, lane: 34, noAuto: true, relSend: 0, count: '12 tracks · 2 returns',
    rows: L.tracks.map((t, i) => (t.sn[0] ? { k: 't', i, cls: 'is-rel', rel: `sends ${t.sn[0][1]} dB` } : { k: 't', i })).concat([Object.assign({ cls: 'is-sel' }, PLATE), ECHO]),
    strips: mark(['Snare', 'Acoustic', 'Lead Vox', 'BVs'], { sel: 'Plate' }), insp: returnPanel('reverb') })),
  P.frame('7g', 'What a track goes to', 'Snare is selected. The group that takes its output and the return it sends to are marked, each saying why in its header.', P.mixScreen({ sel: 1, ti: 1, lane: 32, noAuto: true, noWhy: true, mix: '35', chain: snareChain, out: 'Drums', count: '12 tracks · 2 groups · 2 returns',
    rows: [{ k: 'g', n: 'Drums', cls: 'is-rel', rel: 'takes Snare' }, { k: 't', i: 0, d: 1 }, { k: 't', i: 1, d: 1 }, { k: 't', i: 2, d: 1 }, { k: 't', i: 3, d: 1 }, { k: 't', i: 4 }, { k: 'g', n: 'Guitars', sub: 'group · 3 tracks · folded', open: false, spans: [[0, 222]] }, { k: 't', i: 8 }, { k: 't', i: 9 }, { k: 't', i: 10 }, { k: 't', i: 11 },
      Object.assign({}, PLATE, { cls: 'is-rel', sub: 'Snare −18.0 dB' }), ECHO],
    strips: mark(['Drums', 'Plate']) })),
  P.frame('7h', 'Who listens to a track', 'Kick is selected. Bass DI is marked because its compressor listens to Kick: its CMP label is in Kick’s colour whatever is selected, and here the header says it in words. Drums is marked as Kick’s output.', P.mixScreen({ sel: 0, ti: 0, lane: 32, noAuto: true, noWhy: true, out: 'Drums', count: '12 tracks · 2 groups · 2 returns',
    chain: [['Trim', '0.0 dB'], ['HPF', '40 Hz'], ['LPF', 'Off', 'is-off'], ['EQ', '3 bands', 'is-open'], ['Comp', '4.0:1'], ['Sat', 'Off', 'is-off'], ['Fader', '−6.5'], ['Sends', '0']],
    rows: [{ k: 'g', n: 'Drums', cls: 'is-rel', rel: 'takes Kick' }, { k: 't', i: 0, d: 1 }, { k: 't', i: 1, d: 1 }, { k: 't', i: 2, d: 1 }, { k: 't', i: 3, d: 1 }, { k: 't', i: 4, cls: 'is-rel', rel: 'listens to Kick' }, { k: 'g', n: 'Guitars', sub: 'group · 3 tracks · folded', open: false, spans: [[0, 222]] }, { k: 't', i: 8 }, { k: 't', i: 9 }, { k: 't', i: 10 }, { k: 't', i: 11 }, PLATE, ECHO],
    strips: mark(['Drums', 'Bass DI']) })),
];

// ---------- component sheet ----------
export function glanceSheet(sheet, spec, states) {
  const bigp = (html) => `<div class="gl" style="--pip:14px;--fs-sig:18px;--pip-gap:6px;width:auto"><div>${html}</div></div>`;
  const one = (i, cls, o) => bigp(L.sig(i, cls, o));
  const head = (t, o = {}) => `<div class="row t${t.tint || 1} ${o.cls || ''} ${o.s ? 's' : ''}" style="--head-w:var(--head-w);width:var(--head-w)"><div class="hd"><i class="tdot"></i><div class="names"><span class="nm ell">${t.n}</span><span class="rl ell">${o.rel || t.r}</span></div>${L.glance(t, o)}${tgl('mu')}${tgl('so')}${tgl('lk', t.lock)}<div class="hbar mini"><i style="width:60%"></i></div></div></div>`;
  const T = L.tracks, t = (i) => Object.assign({ tint: i + 1 }, T[i]);
  const cell = (c, inner) => `<div class="cl ${c}" style="width:var(--strip-w-max)">${inner}</div>`;
  return sheet(2,
    spec('Chain labels', states([['High-pass', one(0, 'on')], ['Low-pass', one(1, 'on')], ['EQ', one(2, 'on')], ['Compressor', one(3, 'on')], ['Saturation', one(4, 'on')]])
      + states([['Bypassed', one(2, '')], ['On, changes nothing', one(2, 'nu')], ['Working', one(2, 'on')], ['Parallel compressor', one(3, 'on', { par: 1 })], ['Compressor listening to Kick', one(3, 'on key t1')]])
      + states([['Lead Vox · all but low-pass', bigp(L.pips(T[10]))], ['Snare · parallel', bigp(L.pips(T[1]))]]) + states([['Bass DI · keyed by Kick', bigp(L.pips(T[4]))], ['Room · flat EQ, compressor', bigp(L.pips(T[3]))], ['Actual size', `<div class="gl"><div>${L.pips(T[10])}</div></div>`]]),
      'One short label per block, always in signal order: HP, LP, EQ, CMP, SAT, as printed on any mixer. Solid: on and changing the sound. Outlined: on but neutral (an EQ with every band at 0 dB). Faint: bypassed. A parallel compressor carries two small bars; a keyed one takes the colour of the track it listens to. Enlarged here; the last one is the real size in a header.')
    + spec('Send dots and automation mark', states([['No send', bigp('<i class="sd"></i>')], ['Sends', bigp('<i class="sd on"></i>')], ['To the selected return', bigp('<i class="sd rel"></i>')], ['Automation by hand', bigp('<i class="pa user" style="margin:0"></i>')], ['Automation by the assistant', bigp('<i class="pa ai" style="margin:0"></i>')]])
      + states([['2 returns', L.glance(T[10])], ['4 returns', L.glance({ ch: T[10].ch, sn: [1, 0, 1, 1].map((x) => x && [1, '−18']) }, { returns: RET4 })], ['8 returns', L.glance({ ch: T[10].ch, sn: [1, 0, 1, 1, 0, 0, 1, 0].map((x) => x && [1, '−18']) }, { returns: RET4.concat(RET4) })]]),
      'One dot per return, in the order of the returns in the mixer; the level is in the strip and in the Overview. Above five returns the dots narrow so that eight still fit. The diamond is the shape of an automation point: lime when drawn by hand, the assistant’s blue when it wrote the lanes.')
    + spec('Track header with marks', `<div class="col g1">${head(t(10))}${head(t(4))}${head(t(1), { cls: 'is-rel', rel: 'sends −18.0 dB', rel2: 0 })}${head(t(11))}${head(t(8), { s: 1 })}</div>`,
      'The cell sits between the name and the toggles and keeps its width on every lane height, so the marks line up down the timeline. Third row: connected to the selection (lime edge, the reason in place of the role). Last row: small lane, role hidden. Clicking the cell selects the track; each mark has a tooltip in words.')
    + spec('Overview cells', states([['Working', cell('on', '90')], ['Hover', cell('on is-hover', '3:1')], ['On, neutral', cell('nu', 'flat')], ['Bypassed', cell('off', 'off')], ['Keyed', cell('on', '<i class="led t1"></i>4:1')], ['Parallel', cell('on', '3:1<span class="dim">35%</span>')]])
      + states([['Send', cell('on sn', '<i style="width:62%"></i><span>−14</span>')], ['Send to the selection', cell('on sn rel', '<i style="width:62%"></i><span>−14</span>')], ['No send', cell('off', '·')], ['Return · its senders', cell('on', '4 in')], ['Output', cell('on', 'Drums')], ['Automation · assistant', cell('on ai', 'Fader')], ['Automation · by hand', cell('on acc', 'Fader')]]),
      'One cell per block, return, output and automation, as wide as the strip. Click toggles bypass (or adds a send at the default level on an empty send cell); double-click opens the block in the inspector. Every change is the same edit the inspector would make: one undo step.')
    + spec('Strip and label rail', `<div class="fx g1 stretch" style="height:calc(var(--sp-8) * 4)">${P.rail()}${[1, 4, 10].map((i) => P.strip(P.allStrips()[i], { sel: 10 })).join('')}${P.strip(P.allStrips()[14], {})}<span style="width:var(--sp-4)"></span>${P.rail({ view: 'grid' })}${[1, 4, 10].map((i) => P.strip(P.allStrips()[i], { view: 'grid', sel: 10 })).join('')}${P.strip(P.allStrips()[14], { view: 'grid' })}</div>`,
      'Left: faders. The chain labels sit under the colour bar, one under the other at the strip’s width, like the insert slots of a console: click toggles bypass, double-click opens the block in the inspector; under the pan there is one thin send bar per return, named on the rail. A return shows a dotted line on its own row. Right: the same strips in the Overview. The two-way switch at the foot of the rail changes view; the rail and the mix bus do not scroll.'),
  );
}
