// Screen 17 — "Explain this mix" (P-019): the map of where the sound goes and, beside it, the mix told in words.
// Everything on these boards is built from the project document and the analysis: no model is called.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
const { ic, tag, badge, btn, ib, f1, px } = L;

// ---------- sample projects ----------
// A map model: families of tracks (column 1), groups (column 2), returns (column 2, below), the mix bus (column 3).
// Track: n name, tint, lv level 0–1, out group name (none: straight to the mix bus), sn send per return ([pos, dB] or 0),
// sc name of the track its compressor listens to, au who wrote its automation, note number of the assistant's note.
const RETS = [{ n: 'Plate', sub: 'plate reverb · 1.2 s', lv: 0.35 }, { n: 'Echo', sub: 'delay · dotted 8th', lv: 0.25 }];
const tr = (i, x = {}) => { const t = L.tracks[i]; return Object.assign({ n: t.n, r: t.r, tint: i + 1, lv: t.lv, out: t.out, sn: t.sn, sc: t.sc !== undefined ? L.tracks[t.sc].n : undefined, au: t.au, note: t.note, lock: t.lock }, x); };
const SONG = () => ({
  fams: [['Drums', [0, 1, 2, 3].map((i) => tr(i))], ['Bass', [tr(4)]], ['Guitars', [5, 6, 7].map((i) => tr(i))], ['Keys and strings', [8, 9].map((i) => tr(i))], ['Vocals', [10, 11].map((i) => tr(i))]],
  groups: [{ n: 'Drums', sub: '4 tracks', lv: 0.7 }, { n: 'Guitars', sub: '3 tracks', lv: 0.6 }],
  returns: RETS.map((r) => Object.assign({}, r)), mix: '−16.4 LUFS',
});
// The same song right after the import: no groups, no returns, every fader where the file left it.
const RAW = () => ({ fams: SONG().fams.map(([n, ts]) => [n, ts.map((t) => Object.assign(t, { out: undefined, sn: [], sc: undefined, au: undefined, note: undefined, lock: undefined, lv: 0.55 }))]), groups: [], returns: [], mix: '−11.8 LUFS' });
// 27 tracks, 5 groups, 4 returns: Drums and Guitars are folded, as in the timeline.
const bt = (n, tint, lv, x = {}) => Object.assign({ n, tint, lv, sn: [] }, x);
const BIG = () => ({
  fams: [
    ['Vocals', [bt('Lead Vox', 11, 0.75, { out: 'Vocals', sn: [[62, '−14'], 0, [45, '−19'], [30, '−24']], au: ['ai'], note: 4 }), bt('Vox Dbl', 11, 0.5, { out: 'Vocals', sn: [[55, '−16'], 0, 0, 0] }),
      bt('Harmony 1', 12, 0.45, { out: 'BVs', sn: [[70, '−12'], 0, 0, 0] }), bt('Harmony 2', 12, 0.45, { out: 'BVs', sn: [[70, '−12'], 0, 0, 0] }), bt('BV Low', 12, 0.4, { out: 'BVs', lock: 1 }), bt('BV High', 12, 0.4, { out: 'BVs', lock: 1 })]],
    ['Drums', [bt('10 tracks', 1, 0.8, { out: 'Drums', fold: 1, sn: [0, [48, '−18'], 0, 0] })]],
    ['Bass', [bt('Bass DI', 5, 0.7, { sc: '10 tracks', note: 2, au: ['user'] }), bt('Bass Amp', 5, 0.55)]],
    ['Guitars', [bt('5 tracks', 7, 0.7, { out: 'Guitars', fold: 1, sn: [0, [40, '−20'], 0, [52, '−17']], note: 3 })]],
    ['Keys', [bt('Piano', 9, 0.5, { out: 'Keys', sn: [0, [38, '−21'], 0, 0] }), bt('Rhodes', 9, 0.45, { out: 'Keys', sn: [0, 0, 0, [44, '−19']] })]],
    ['Pads and strings', [bt('Pad', 10, 0.38), bt('Strings', 10, 0.42, { sn: [[42, '−20'], 0, 0, 0] })]],
  ],
  groups: [{ n: 'Vocals', sub: '2 tracks', lv: 0.75 }, { n: 'BVs', sub: '4 tracks', lv: 0.5 }, { n: 'Drums', sub: '10 tracks · folded', lv: 0.8 }, { n: 'Guitars', sub: '5 tracks · folded', lv: 0.7 }, { n: 'Keys', sub: '2 tracks', lv: 0.5 }],
  returns: [{ n: 'Plate', sub: 'plate reverb · 1.2 s', lv: 0.35 }, { n: 'Room', sub: 'room reverb · 0.5 s', lv: 0.3 }, { n: 'Slap', sub: 'delay · 110 ms', lv: 0.2 }, { n: 'Throw', sub: 'delay · quarter note', lv: 0.2 }],
  mix: '−15.9 LUFS',
});

// ---------- the map ----------
// o.sel: name of the selected element (its connections stay lit, the rest dims); o.note: { name: text } a small tag
// inside a node (what changes in a section); o.off: names of the tracks that do not play (section view);
// o.calls: extra labels on the links of the selection; o.compact: the 1280 × 720 window.
const th = (lv) => 2 + lv * 7;
function map(m, o = {}) {
  const W = o.W || 920, H = o.H || 728, c = o.compact, d = o.dense, nh = c ? 22 : d ? 26 : 30, gap = c || d ? 4 : 6, fg = c || d ? 10 : 18, fl = c || d ? 14 : 16, rh = c ? 32 : d ? 28 : 40;
  const X1 = c ? 60 : 72, W1 = c ? 176 : 196, W2 = 168, X2 = c ? 344 : 400, W3 = 128, X3 = W - 40 - W3, R1 = X1 + W1, R2 = X2 + W2;
  // column 1
  let y = 0; const rows = [], fams = [];
  m.fams.forEach(([name, ts], fi) => { if (fi) y += fg; fams.push([name, y]); y += fl; ts.forEach((t, i) => { if (i) y += gap; t.y = y; t.h = nh; y += nh; rows.push(t); }); });
  const tracksH = y, rcols = m.returns.length > 2 && c ? 2 : 1;
  m.returns.forEach((r, i) => { r.y = tracksH + (c || d ? 22 : 34) + i * (rh + (c || d ? 5 : 10)); r.h = rh; });
  const total = m.returns.length ? m.returns[m.returns.length - 1].y + rh : tracksH, oy = Math.max(26, Math.round((H - total) / 2) + 8);
  rows.concat(m.returns).forEach((e) => { e.y += oy; e.cy = e.y + e.h / 2; });
  // column 2: a group sits beside its members and is as tall as what comes into it
  m.groups.forEach((g) => {
    const mem = rows.filter((t) => t.out === g.n), sum = mem.reduce((a, t) => a + th(t.lv), 0) + (mem.length - 1) * 4;
    g.h = Math.max(c ? 34 : 44, sum + 14); g.cy = (mem[0].cy + mem[mem.length - 1].cy) / 2; g.y = g.cy - g.h / 2;
    let p = g.cy - sum / 2; mem.forEach((t) => { t.py = p + th(t.lv) / 2; p += th(t.lv) + 4; });
  });
  // groups that would overlap are pushed apart, top to bottom
  m.groups.forEach((g, i) => { const a = m.groups[i - 1]; if (a && g.y < a.y + a.h + 8) { const d = a.y + a.h + 8 - g.y; g.y += d; g.cy += d; rows.filter((t) => t.out === g.n).forEach((t) => { t.py += d; }); } });
  // column 3: everything that reaches the mix bus, in the order it comes down the page
  const direct = rows.filter((t) => !t.out), into = m.groups.concat(direct, m.returns).sort((a, b) => a.cy - b.cy);
  const msum = into.reduce((a, e) => a + th(e.lv), 0) + (into.length - 1) * 9, mh = msum + 36, mcy = oy + Math.min(tracksH / 2, H / 2 - oy + 20), my = mcy - mh / 2;
  let p = mcy - msum / 2; into.forEach((e) => { e.my = p + th(e.lv) / 2; p += th(e.lv) + 9; });
  // what the selection is connected to
  const sel = o.sel, rel = new Set(), lit = new Set(), find = (n) => rows.concat(m.groups, m.returns).find((e) => e.n === n);
  const outOf = (e) => { lit.add('o:' + e.n); if (e.out) { rel.add(e.out); lit.add('o:' + e.out); } rel.add('Mix'); };
  if (sel) {
    const e = find(sel), ri = m.returns.indexOf(e);
    if (ri >= 0) { rows.forEach((t) => { if (t.sn && t.sn[ri]) { rel.add(t.n); lit.add('s:' + t.n + '>' + e.n); } }); outOf(e); }
    else if (m.groups.includes(e)) { rows.forEach((t) => { if (t.out === e.n) { rel.add(t.n); lit.add('o:' + t.n); } }); outOf(e); }
    else if (e) {
      outOf(e); (e.sn || []).forEach((s, i) => { if (s) { const r = m.returns[i]; rel.add(r.n); lit.add('s:' + e.n + '>' + r.n); lit.add('o:' + r.n); } });
      if (e.sc) { rel.add(e.sc); lit.add('k:' + e.sc + '>' + e.n); }
      rows.forEach((t) => { if (t.sc === e.n) { rel.add(t.n); lit.add('k:' + e.n + '>' + t.n); } });
    }
  }
  (o.lit || []).forEach((n) => { rel.add(n); lit.add('o:' + n); });
  const dimming = !!sel || !!o.lit, st = (id) => (lit.has(id) ? 'rel' : ''), bz = (x1, y1, x2, y2) => `M${f1(x1)} ${f1(y1)}C${f1((x1 + x2) / 2)} ${f1(y1)} ${f1((x1 + x2) / 2)} ${f1(y2)} ${f1(x2)} ${f1(y2)}`;
  // links
  const links = [], calls = [];
  rows.forEach((t) => {
    const g = t.out && m.groups.find((x) => x.n === t.out), w = f1(th(t.lv)), off = (o.off || []).includes(t.n) ? 'is-off' : '';
    links.push(`<path class="lk t${t.tint} ${st('o:' + t.n)} ${off}" stroke-width="${w}" d="${g ? bz(R1, t.cy, X2, t.py) : `M${R1} ${f1(t.cy)}H${R2}` + bz(R2, t.cy, X3, t.my).replace('M' + f1(R2) + ' ' + f1(t.cy), '')}"/>`);
  });
  m.groups.forEach((g) => links.push(`<path class="lk tb ${st('o:' + g.n)}" stroke-width="${f1(th(g.lv))}" d="${bz(R2, g.cy, X3, g.my)}"/>`));
  m.returns.forEach((r, ri) => {
    links.push(`<path class="lk tb ${st('o:' + r.n)}" stroke-width="${f1(th(r.lv))}" d="${bz(R2, r.cy, X3, r.my)}"/>`);
    const from = rows.filter((t) => t.sn && t.sn[ri]);
    from.forEach((t, k) => {
      const id = 's:' + t.n + '>' + r.n, py = r.cy + (k - (from.length - 1) / 2) * (c ? 4 : 6), y1 = t.cy + (c ? 5 : 8);
      links.push(`<path class="lk send t${t.tint} ${st(id)}" d="${bz(R1, y1, X2, py)}"/>`);
      if (lit.has(id) && o.sendLabels !== false) calls.push([(R1 + X2) / 2 + (k % 2 ? 26 : -26), (y1 + py) / 2, t.sn[ri][1] + ' dB', 't' + t.tint]);
    });
  });
  rows.forEach((t) => { if (!t.sc) return; const s = find(t.sc), id = 'k:' + s.n + '>' + t.n, bow = X1 - 34;
    links.push(`<path class="lk key t${s.tint} ${st(id)}" d="M${X1} ${f1(s.cy)}C${bow} ${f1(s.cy)} ${bow} ${f1(t.cy)} ${X1 - 6} ${f1(t.cy)}"/><path class="lkh t${s.tint} ${st(id)}" d="M${X1 - 7} ${f1(t.cy - 4)}L${X1 - 1} ${f1(t.cy)}L${X1 - 7} ${f1(t.cy + 4)}Z"/>`);
    if (lit.has(id) && o.keyLabel) calls.push([X1 - 38, (s.cy + t.cy) / 2, o.keyLabel, 't' + s.tint]); });
  links.push(`<path class="lk tm rel" stroke-width="8" d="M${X3 + W3} ${f1(mcy)}H${W - 6}"/><path class="lkh tm rel" d="M${W - 8} ${f1(mcy - 6)}L${W} ${f1(mcy)}L${W - 8} ${f1(mcy + 6)}Z"/>`);
  // nodes
  const cls = (e) => (e.n === sel ? 'is-sel' : rel.has(e.n) ? 'is-rel' : '') + ((o.off || []).includes(e.n) ? ' is-off' : '');
  const box = (e, x, w) => `left:${x}px;top:${px(e.y)};width:${w}px;height:${px(e.h)}`;
  const nt = (e) => (o.note && o.note[e.n] ? `<span class="tag ${o.note[e.n][1] || 'acc'}">${o.note[e.n][0]}</span>` : '');
  const nodes = rows.map((t) => (t.fold
    ? `<div class="mn fold t${t.tint} ${cls(t)}" style="${box(t, X1, W1)}">${ic('right', 'sm')}<span class="nm ell grow">${t.n}</span>${t.note ? badge(t.note) : ''}</div>`
    : `<div class="mn t${t.tint} ${cls(t)}" style="${box(t, X1, W1)}"><i class="tdot"></i><span class="nm ell grow">${t.n}</span>${nt(t)}${t.lock ? ic('lock', 'sm aic') : ''}${t.au ? `<i class="pa ${t.au[0]}"></i>` : ''}${t.note && o.notes !== false ? badge(t.note) : ''}</div>`))
    .concat(m.groups.map((g) => `<div class="mn grp tb ${cls(g)}" style="${box(g, X2, W2)}">${ic('group', 'm')}<div class="col grow"><span class="nm ell">${g.n}</span>${g.h >= 40 ? `<span class="faint s1 ell">${g.sub}</span>` : ''}</div>${nt(g)}</div>`))
    .concat(m.returns.map((r, ri) => `<div class="mn ret tb ${cls(r)}" style="${box(r, X2, W2)}">${ic('wave', 'm')}<div class="col grow"><span class="nm ell">${r.n}</span>${r.h >= 32 ? `<span class="faint s1 ell">${r.sub}</span>` : ''}</div><span class="faint s1 nowrap">${rows.filter((t) => t.sn && t.sn[ri]).length} in</span></div>`))
    .concat(`<div class="mn mix tm ${rel.has('Mix') && dimming ? 'is-rel' : ''}" style="left:${X3}px;top:${px(my)};width:${W3}px;height:${px(mh)}"><span class="nm">Mix bus</span><span class="faint s1">${m.mix}</span><span class="faint s1">to Master</span></div>`);
  const heads = `<span class="xcolh" style="left:${X1}px">Tracks</span>${m.groups.length ? `<span class="xcolh" style="left:${X2}px">Groups</span>` : ''}<span class="xcolh" style="left:${X3}px">Mix bus</span>${m.returns.length ? `<span class="xcolh" style="left:${X2}px;top:${px(m.returns[0].y - (c || d ? 18 : 22))}">Shared effects</span>` : ''}`;
  return `<div class="xstage ${dimming ? 'has-sel' : ''} ${c ? 'compact' : ''}" style="height:${H}px"><svg width="${W}" height="${H}" aria-hidden="true">${links.join('')}</svg>${heads}
    ${fams.map(([n, fy]) => `<span class="xfam" style="left:${X1}px;top:${px(fy + oy)}">${n}</span>`).join('')}${nodes.join('')}
    ${calls.concat(o.calls || []).map((k) => `<span class="xcall ${k[3] || ''}" style="left:${px(k[0])};top:${px(k[1])}">${k[2]}</span>`).join('')}${o.stageOver || ''}</div>`;
}

// The song strip above the map: the whole song or one section (what plays and what changes there), with the playhead.
function song(o = {}) {
  const w = (o.W || 920) - 24 - 104, pps = w / (o.T || L.T);
  return `<div class="xsong"><button class="xs all ${o.section === undefined ? 'on' : ''}">Whole song</button><div class="xsl" style="width:${w}px">
    ${(o.sections || L.sections).map((s, i) => `<button class="xs ${o.section === s[0] ? 'on' : ''}" style="left:${px(s[1] * pps)};width:${px((s[2] - s[1]) * pps - 2)}"><span class="ell">${s[0]}</span></button>`).join('')}<i class="xplay" style="left:${px((o.playAt ?? L.playhead) * pps)}"></i></div></div>`;
}
const legend = (o = {}) => `<div class="xlegend"><span><i class="lg out"></i>Output · thicker is louder</span>${o.raw ? '' : '<span><i class="lg send"></i>Send · a copy to a shared effect</span><span><i class="lg key"></i>Listens to · sidechain</span><span><i class="pa ai"></i>Automation by the assistant</span><span><i class="pa user"></i>by hand</span>'}<span class="grow"></span>${o.compact ? '' : '<span class="faint">Click a box to explain it</span>'}</div>`;
const mapCard = (m, o = {}) => `<section class="card xmap" aria-label="Mix map"><div class="ph"><span class="h">Mix map</span><span class="faint s1">${o.count || '12 tracks · 2 groups · 2 shared effects'}</span><div class="grow"></div>${o.fold ? btn('Unfold all', 'quiet sm') : ''}</div>${song(o)}${map(m, o)}${legend(o)}</section>`;

// ---------- the text ----------
const TINT = Object.assign({ Drums: 'tb', Guitars: 'tb', Plate: 'tb', Echo: 'tb', 'Mix bus': 'tm' }, Object.fromEntries(L.tracks.map((t, i) => [t.n, 't' + (i + 1)])));
const nm = (n, tint) => `<button class="xn ${tint || TINT[n] || 'tb'}"><i class="tdot"></i>${n}</button>`;
const gt = (t, tip = '') => `<span class="gt">${t}${tip ? `<span class="tip">${tip}</span>` : ''}</span>`;
const sec = (title, inner, extra = '') => `<section class="xsec"><div class="fx g2"><h3>${title}</h3>${extra}</div>${inner}</section>`;
const li = (head, text, cls = '') => `<div class="xli ${cls}"><div class="xlh">${head}</div><p>${text}</p></div>`;
const step = (label, name, val, text, cls = 'on') => `<div class="xstep ${cls === '' ? 'is-off' : ''}"><span class="sg ${cls}">${label}</span><div class="col grow"><div class="fx g2"><span class="b">${name}</span><span class="faint s1">${val}</span></div><p>${text}</p></div></div>`;
const why = (n, text, foot) => `<div class="note on colm"><div class="fx g2">${badge(n)}<span class="b">The assistant’s reason</span></div><p class="why">${text}</p><span class="faint s1">${foot}</span></div>`;
const bal = (rows) => `<div class="xbal">${rows.map((r) => `<span class="ell ${r[3] || ''}">${r[0]}</span><div class="hbar ${r[3] || ''} ${r[4] || ''}"><i style="width:${r[1]}%"></i></div><span class="faint s1">${r[2]}</span>`).join('')}</div>`;
const stage = () => `<div class="xpan"><i class="c"></i>${L.tracks.map((t, i) => `<i class="tdot t${i + 1}" title="${t.n}" style="left:${f1(50 + t.pan * 46)}%;top:${f1(10 + i * 7.2)}%"></i>`).join('')}</div><div class="fx between faint s1"><span>Left</span><span>Centre</span><span>Right</span></div>`;

const side = (head, body, o = {}) => `<aside class="card side xside" aria-label="Explanation"><div class="ph">${head}</div>${o.nav || ''}<div class="xdoc">${body}</div>${o.free ? '' : '<i class="xfade"></i>'}
  <div class="xfoot"><span class="faint s1 grow">${o.foot || 'Read from the project as it is now · no AI call, no cost'}</span>${btn('Copy as text', 'quiet sm', 'copy')}</div></aside>`;
const headAll = `<span class="h">Explain this mix</span><div class="grow"></div><span class="kbd">Esc</span>${ib('x', 'Close')}`;
const headOne = (name, kind, tint, extra = '') => `${ib('left', 'Back to the whole mix')}<i class="tdot ${tint}"></i><span class="h ell">${name}</span>${tag(kind)}<div class="grow"></div>${extra}${ib('x', 'Close')}`;
const nav = (on = 0) => `<div class="xnav">${L.seg(['In short', 'Structure', 'Balance', 'Space', 'Over time', 'Reasons'], on, 'quiet sm', 'Sections')}</div>`;

const S_SHORT = sec('In short', `<p><b>12 tracks</b> reach the mix bus through <b>2 groups</b> and share <b>2 effects</b>. ${nm('Lead Vox')} is in front: it is the loudest track, 1.4 LU above ${nm('Kick')}. Kick and ${nm('Bass DI')} hold the low end, and the bass steps back each time the kick hits. The guitars are spread to the sides; voice, bass and drums stay in the centre. The <b>choruses</b> are fuller: three more tracks play there and the voice comes up 1.5 dB.</p>
  <div class="fx g1 wrap">${tag('11 tracks set by the assistant', 'ai')}${tag('1 locked')}${tag('3 changes by hand since')}</div>`);
const S_STRUCT = sec('How it is built', `
  ${li(`${ic('group', 'm')}<span class="b">2 ${gt('groups')}</span>`, `${nm('Drums')} takes ${nm('Kick')} ${nm('Snare')} ${nm('Overheads')} ${nm('Room')} and treats them as one instrument: one EQ, one ${gt('parallel compressor')} at 50 %, one fader for all four. ${nm('Guitars')} takes ${nm('Acoustic')} ${nm('Gtr Left')} ${nm('Gtr Right')} and shares one EQ.`)}
  ${li(`${ic('wave', 'm')}<span class="b">2 ${gt('shared effects')}</span><span class="faint s1">returns</span>`, `${nm('Plate')} is a plate reverb with a 1.2 s tail: 4 tracks send a copy of themselves to it. ${nm('Echo')} is a delay on dotted eighths: 2 tracks send to it. Only the effect comes back into the mix.`)}
  ${li(`${ic('right', 'm')}<span class="b">5 tracks go straight to the mix bus</span>`, `${nm('Bass DI')} ${nm('Rhodes')} ${nm('Strings')} ${nm('Lead Vox')} ${nm('BVs')}`)}`);
const S_BAL = sec('Who is in front', bal([['Vocals', 92, '−19.8 LUFS', 't11'], ['Drums', 84, '−21.2 LUFS', 't1'], ['Bass', 78, '−22.6 LUFS', 't5'], ['Guitars', 66, '−24.9 LUFS', 't7'], ['Keys and strings', 48, '−28.3 LUFS', 't9'], ['Shared effects', 30, '−32.0 LUFS', 'tb']])
  + `<p>How loud each family is in the mix, over the whole song. The vocals lead by 1.4 LU; keys and strings sit 8.5 LU behind them, as a background.</p>`, '<span class="faint s1">loudness of each family</span>');
const S_PAN = sec('Left to right', stage() + `<p>${nm('Gtr Left')} and ${nm('Gtr Right')} are far apart (80 % left, 80 % right); ${nm('Acoustic')} is slightly left, ${nm('Rhodes')} slightly right. The other 8 tracks are in the centre.</p>`);
const S_SPACE = sec('Space', `<p>${nm('BVs')} is the wettest track in ${nm('Plate')} (−12 dB), then ${nm('Lead Vox')} (−14 dB), ${nm('Snare')} (−18 dB) and ${nm('Acoustic')} (−20 dB): the more a track sends, the further back it sounds. Kick, bass and electric guitars send nothing: they stay dry and close.</p>`);
const S_REACT = sec('Who reacts to whom', `<p>The compressor of ${nm('Bass DI')} ${gt('listens to')} ${nm('Kick')}: each kick hit turns the bass down by about 2 dB for a moment, so the two do not pile up in the low end.</p>`);
const S_TIME = sec('What changes along the song', `
  ${li(`<span class="b">Choruses</span><span class="faint s1">0:48 · 1:52 · 2:48</span>`, `${nm('Gtr Left')} ${nm('Gtr Right')} ${nm('BVs')} play only here. ${nm('Lead Vox')} is 1.5 dB louder. ${nm('Strings')} is 1.5 dB louder in the last one.`)}
  ${li(`<span class="b">Bridge</span><span class="faint s1">2:24</span>`, `${nm('Rhodes')} sends to ${nm('Echo')} only here. Kick and snare stop.`)}
  ${li(`<span class="b">Everything else</span>`, `8 tracks keep the same settings from start to end.`)}`);
const S_WHY = sec('Reasons', `<div class="note colm"><div class="fx g2">${ic('spark', 'm aic')}<span class="b">Auto mix · applied 21:44</span></div><p class="why">${L.SUMMARY}</p></div>
  ${L.NOTES.map((n) => `<div class="note t${n[2]}">${badge(n[0])}<div class="col g1 grow"><span class="b">${n[1]}</span><span class="why dim">${n[3]}</span></div></div>`).join('')}
  <p class="faint s1">Reasons are the assistant’s own words, kept with each change it made. Changes made by hand have no reason attached.</p>`);
const docAll = S_SHORT + S_STRUCT + S_BAL + S_PAN + S_SPACE + S_REACT + S_TIME + S_WHY;

const TIP_COMP = 'A compressor turns the loud moments down, so quiet and loud parts end up closer. “3:1” says how firmly.';
const docVox = sec('Where it goes', `<p>${nm('Lead Vox')} goes <b>straight to the mix bus</b>: it is not part of a group. It also sends a copy of itself to ${nm('Plate')} (−14 dB), which puts a room around the voice, and to ${nm('Echo')} (−22 dB), a faint repeat behind it.</p>`)
  + sec('Its place in the mix', `<p>The loudest track, 1.4 LU above ${nm('Kick')}. In the centre. It plays in 6 of the 8 sections: not in the intro, not in the outro.</p>`)
  + sec('What happens to it, in order', `<div class="xsteps">
    ${step('HP', 'High-pass', '90 Hz', 'Removes what is below 90 Hz: rumble and bumps a voice does not need.')}
    ${step('EQ', 'EQ', '4 bands', 'Less mud at 220 Hz, more presence at 3.2 kHz, less harshness at 6.5 kHz, more air above 11 kHz.')}
    ${step('CMP', gt('Compressor', TIP_COMP), '3:1 · threshold −18 dB', 'Turns the loudest words down so every line is heard: about 3 dB on the peaks.')}
    ${step('SAT', 'Saturation', '4 dB · mix 35 %', 'Adds a little grit: the voice sounds denser.')}
    ${step('LVL', 'Fader', '−5.8 dB', `Its level in the mix. <span class="aic">Automated by the assistant</span>: 1.5 dB louder in the three choruses.`, 'nu')}</div>`)
  + sec('Why it is set this way', why(4, L.WHY, 'Auto mix applied at 21:44 · changed by hand since: EQ band 2 gain, 21:47'));
const docPlate = sec('What it is', `<p>A <b>${gt('shared effect')}</b>: a plate reverb with a 1.2 s tail, kept between 300 Hz and 12 kHz. Tracks do not pass through it: each one sends a copy, and only the reverb comes back, into the mix bus, at −12 dB.</p>`)
  + sec('Who sends here', bal([['BVs', 70, '−12 dB', 't12'], ['Lead Vox', 62, '−14 dB', 't11'], ['Snare', 48, '−18 dB', 't2'], ['Acoustic', 40, '−20 dB', 't6']]) + `<p>${nm('BVs')} is the wettest, ${nm('Acoustic')} the driest. The other 8 tracks send nothing here.</p>`, '<span class="faint s1">4 tracks</span>')
  + sec('What it does for the mix', `<p>Tracks that share a reverb sound as if they were in the same room. The more a track sends, the further back it sits: here the backing vocals are behind the lead.</p>`)
  + sec('What happens to it, in order', `<div class="xsteps">${step('REV', 'Plate reverb', '1.2 s · pre-delay 20 ms', 'The tail starts 20 ms after the sound, so words stay clear in front of it.')}${step('HP', 'High-pass', '300 Hz', 'No reverb on the low end: bass and kick stay tight.')}${step('LP', 'Low-pass', '12 kHz', 'A darker tail, with no hiss on the s sounds.')}</div>`)
  + sec('Why it is set this way', why('b', 'Shorter and darker; pre-delay 20 ms.', 'Auto mix applied at 21:44'));
const TIP_SC = 'A sidechain lets one track control an effect on another. Here the kick tells the bass compressor when to work.';
const docBass = sec('Where it goes', `<p>${nm('Bass DI')} goes <b>straight to the mix bus</b>. It sends nothing to the shared effects: it stays dry.</p>`)
  + sec('Its place in the mix', `<p>The third loudest track, in the centre. With ${nm('Kick')} it is the low end of the song. It plays from the first verse to the last chorus.</p>`)
  + sec('Who it reacts to', `<p>Its compressor listens to ${nm('Kick')} through a ${gt('sidechain', TIP_SC)}: each kick hit turns the bass down by about 2 dB for a moment, then lets it back.</p>`)
  + sec('What happens to it, in order', `<div class="xsteps">${step('HP', 'High-pass', '40 Hz', 'Removes what is below 40 Hz: energy that eats headroom and is not heard.')}${step('EQ', 'EQ', '2 bands', 'A little less boxiness (−2 dB at 250 Hz), a little more string noise (+1.5 dB at 900 Hz).')}${step('CMP', 'Compressor', '4:1 · listens to Kick', 'Ducks about 2 dB on every kick hit.', 'on key t1')}${step('LVL', 'Fader', '−7.2 dB', `Its level in the mix. <span class="acc">Automated by hand</span>: drawn in the second verse.`, 'nu')}</div>`)
  + sec('Why it is set this way', why(2, 'Ducks 2 dB on every kick (sidechain) to free the low end without thinning the bass.', 'Auto mix applied at 21:44 · fader automation drawn by hand, 21:52'));
const docChorus = sec('What plays here', `<p><b>11 of the 12 tracks.</b> ${nm('Gtr Left')} ${nm('Gtr Right')} ${nm('BVs')} play <b>only</b> in the choruses. ${nm('Rhodes')} is silent here.</p>`)
  + sec('What is set differently', `
    ${li(`${nm('Lead Vox')}<span class="b">1.5 dB louder</span>`, `Fader automation written by the assistant: “lifted it a further 1.5 dB in the choruses only”.`)}
    ${li(`${nm('Strings')}<span class="b">1.5 dB louder</span><span class="faint s1">last chorus only</span>`, `Fader automation written by the assistant: “Up 1.5 dB in the last chorus”.`)}
    ${li(`<span class="b">Everything else</span>`, `The other 9 tracks, the groups and the shared effects are set as in the rest of the song.`)}`)
  + sec('How it compares', bal([['Chorus', 100, '−13.9 LUFS', '', 'acc'], ['Verse 1', 70, '−18.1 LUFS'], ['Verse 2', 72, '−17.8 LUFS'], ['Bridge', 58, '−19.6 LUFS'], ['Intro', 46, '−22.4 LUFS'], ['Outro', 42, '−23.0 LUFS']]) + `<p>The loudest section: 4.2 LU above the first verse. Most of the difference comes from the tracks that enter, not from the faders.</p>`, '<span class="faint s1">loudness of the mix per section</span>');
const docRaw = sec('In short', `<p><b>Nothing has been shaped yet.</b> All 12 tracks go straight to the mix bus at the level of their files. There are no groups, no shared effects, and every chain is empty.</p><div class="fx g1 wrap">${tag('0 groups')}${tag('0 shared effects')}${tag('0 blocks working')}</div>`)
  + sec('Who is in front', bal([['Guitars', 96, '−15.1 LUFS', 't7'], ['Drums', 90, '−16.0 LUFS', 't1'], ['Bass', 74, '−19.2 LUFS', 't5'], ['Vocals', 70, '−19.8 LUFS', 't11'], ['Keys and strings', 60, '−21.5 LUFS', 't9']]) + `<p>As imported, the guitars are the loudest family and the vocals are 4.7 LU behind them.</p>`, '<span class="faint s1">loudness of each family</span>')
  + sec('Left to right', `<div class="xpan"><i class="c"></i>${L.tracks.map((t, i) => `<i class="tdot t${i + 1}" style="left:50%;top:${f1(8 + i * 7.4)}%"></i>`).join('')}</div><div class="fx between faint s1"><span>Left</span><span>Centre</span><span>Right</span></div><p>Every track is in the centre.</p>`)
  + sec('Where to start', `<div class="note colm"><p class="why">This page describes the mix; it does not judge it. To get a first balance, close it and use <b>Auto mix</b> in the assistant, or move the faders: the map and this text follow every change.</p></div>`);
const docBig = sec('In short', `<p><b>27 tracks</b> reach the mix bus through <b>5 groups</b> and share <b>4 effects</b>. ${nm('Lead Vox', 't11')} is in front, 1.1 LU above the drums. The low end is ${nm('Kick In', 't1')} with ${nm('Bass DI', 't5')}, which steps back on each kick hit. Two reverbs and two delays: the voices use ${nm('Plate')}, the drums and guitars ${nm('Room')}.</p>
    <div class="fx g1 wrap">${tag('24 tracks set by the assistant', 'ai')}${tag('2 locked')}${tag('2 groups folded in the map')}</div>`)
  + sec('How it is built', `
    ${li(`${ic('group', 'm')}<span class="b">5 ${gt('groups')}</span>`, `${nm('Vocals')} 2 tracks · ${nm('BVs')} 4 tracks · ${nm('Drums')} 10 tracks, with a ${gt('parallel compressor')} at 50 % · ${nm('Guitars')} 5 tracks · ${nm('Keys')} 2 tracks.`)}
    ${li(`${ic('wave', 'm')}<span class="b">4 ${gt('shared effects')}</span><span class="faint s1">returns</span>`, `${nm('Plate')} plate reverb, 1.2 s, 5 tracks send here · ${nm('Room')} room reverb, 0.5 s, 5 tracks · ${nm('Slap')} delay, 110 ms, 1 track · ${nm('Throw')} delay on quarter notes, 3 tracks.`)}
    ${li(`${ic('right', 'm')}<span class="b">4 tracks go straight to the mix bus</span>`, `${nm('Bass DI', 't5')} ${nm('Bass Amp', 't5')} ${nm('Pad', 't10')} ${nm('Strings', 't10')}`)}`)
  + sec('Who is in front', bal([['Vocals', 92, '−19.4 LUFS', 't11'], ['Drums', 88, '−20.5 LUFS', 't1'], ['Bass', 78, '−22.1 LUFS', 't5'], ['Guitars', 70, '−23.6 LUFS', 't7'], ['Keys', 50, '−27.9 LUFS', 't9'], ['Pads and strings', 40, '−30.2 LUFS', 't10']]), '<span class="faint s1">loudness of each family</span>')
  + S_REACT.replace(nm('Kick'), nm('Kick In', 't1'));

// ---------- screens ----------
const xbtn = (on, cls = '') => `<button class="btn xbtn ${on ? 'on' : 'quiet'} ${cls}" aria-pressed="${on}">${ic('map', 'm')}Explain</button>`;
export function explain(m, o = {}) {
  const W = o.min ? 1280 - 24 - 432 : 920, H = o.min ? 544 : 728;
  return P.app(o, `${P.top(Object.assign({ mid: xbtn(true) }, o))}<div class="body">${mapCard(m, Object.assign({ W, H, compact: o.min }, o))}${o.side}</div>${o.over || ''}`);
}
const soloBtn = btn('Solo', 'quiet sm', 'ear'), inspBtn = btn('Open in inspector', 'sm');

export { SONG, side, headAll, docAll, nav };

export default {
  'screens/17-mix-explain.html': P.page('17 · Explain this mix', 'P-019. A view of the Mix workspace that answers “who goes where, who does what, and why”: on the left the map of the signal, on the right the same mix in words. Both are read from the project document and from the analysis already in the cache: no model is called, nothing is sent, the same project always gives the same page. The top bar stays live, so the song can play while reading.', [
    P.frame('17a', 'Where it opens from', '“Explain” in the top bar of the Mix workspace, between history and the mix bus readouts; ⌘E. It replaces timeline, mixer and side panels until it is closed; playback, loop and undo keep working.', P.mixScreen({ mid: xbtn(false, 'is-hover'), over: `<div class="tip" style="left:calc(var(--frame-w) / 2 + var(--sp-8) * 2);top:calc(var(--top-h) - var(--sp-1))">Explain this mix <span class="kbd">⌘E</span></div>` })),
    P.frame('17b', 'The whole mix', 'Left, the map: tracks by family, the groups that take them, the shared effects below, the mix bus. A solid line is an output and is thicker when the track is louder; a dashed line is a send; a dotted one says whose compressor listens to whom. Right, the text: every name is a chip in the track’s colour and selects it in the map. Dotted words open a one-line explanation.', explain(SONG(), { side: side(headAll, docAll, { nav: nav(0) }) })),
    P.frame('17c', 'One track', 'Lead Vox is selected, in the map or from a chip. Its path stays lit with the send levels on it, the rest dims. The text becomes the track’s story: where it goes, its place in the mix, each block of its chain in signal order with what it does in plain words, the copies it sends, and the assistant’s reason for the last change, with what was changed by hand since.', explain(SONG(), { sel: 'Lead Vox', side: side(headOne('Lead Vox', 'lead vocal', 't11', soloBtn + inspBtn), docVox.replace(gt('Compressor', TIP_COMP), gt('Compressor'))) })),
    P.frame('17d', 'A shared effect', 'Plate is selected: the four tracks that send to it are lit, each with its level, and the text says what a return is, who feeds it and what that does to the mix.', explain(SONG(), { sel: 'Plate', side: side(headOne('Plate', 'reverb return', 'tb', soloBtn + inspBtn), docPlate) })),
    P.frame('17e', 'Who listens to whom · a term explained', 'Bass DI is selected: the dotted link from Kick is lit and says what it does. The pointer is on “sidechain”: a tooltip explains the word in one sentence. The glossary is a fixed list of terms; the tooltip never covers the map.', explain(SONG(), { sel: 'Bass DI', keyLabel: 'ducks 2 dB', side: side(headOne('Bass DI', 'bass', 't5', soloBtn + inspBtn), docBass.replace('class="gt">sidechain', 'class="gt is-open">sidechain')) })),
    P.frame('17f', 'One section', 'A section is chosen in the strip above the map (all its repeats count as one). Tracks that do not play there dim; the ones that play only there or are set differently carry a tag. The text lists what plays, what is set differently and how loud the section is next to the others.', explain(SONG(), { section: 'Chorus', section2: 1, off: ['Rhodes'], notes: false, note: { 'Lead Vox': ['+1.5 dB', 'ai'], Strings: ['+1.5 dB · last', 'ai'], 'Gtr Left': ['only here'], 'Gtr Right': ['only here'], BVs: ['only here'] }, side: side(headOne('Chorus', '3 times · 0:48, 1:52, 2:48', 'tm'), docChorus) })),
    P.frame('17g', 'A large project', '27 tracks, 5 groups, 4 shared effects. A group can fold in the map as in the timeline: its tracks become one row with their count, and the links, sends included, start from there. Here Drums and Guitars are folded; “Unfold all” opens them and the map scrolls.', explain(BIG(), { fold: 1, dense: 1, count: '27 tracks · 5 groups · 4 shared effects', engine: '31 %', side: side(headAll, docBig, { nav: nav(0) }) })),
    P.frame('17h', 'A mix nobody has touched', 'Right after the import: every track goes straight to the mix bus, all lines have the thickness of their files. The text says so, shows who is loudest as imported, and points to where to start without giving advice of its own.', explain(RAW(), { raw: 1, count: '12 tracks · no groups · no shared effects', status: 'Saved 21:31', side: side(headAll, docRaw, { nav: `<div class="xnav">${L.seg(['In short', 'Balance', 'Space'], 0, 'quiet sm', 'Sections')}</div>` }) })),
    P.frame('17i', 'Minimum window · 1280 × 720', 'The text column narrows to 432, the map uses its compact rows (22 px). A project with many tracks uses the rows in between (26 px, as in 17g) before it scrolls.', explain(SONG(), { min: true, side: side(headAll, docAll, { nav: nav(0) }) })),
    P.frame('17j', 'The whole text', 'The text of 17b from top to bottom, as it scrolls in the column. Eight sections, always in this order; a section with nothing to say is left out (no shared effects: no “Space”). “Copy as text” copies it as plain text.', `<div class="sw free" data-phase="mix"><div class="body xfree">${side(headAll, docAll, { nav: nav(0), free: 1 })}</div></div>`),
  ]),
};
