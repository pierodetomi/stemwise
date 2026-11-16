// Stemwise design boards — shared parts of the workspace (top bar, timeline, mixer, inspector, assistant).
import * as L from './lib.mjs';
const { ic, tgl, tag, badge, seg, btn, ib, knob, kb, meter, grm, fader, f1, px } = L;

// ---------- geometry of the reference and minimum windows ----------
export function geo(o = {}) {
  const W = o.min ? 1280 : 1440, side = o.min ? 288 : 316, head = o.head ?? (o.min ? 304 : 344);
  const left = W - 4 * 8 - (o.oneSide ? 1 : 2) * side - (o.oneSide ? -8 : 0);
  return { W, side, head, left, tlw: left - head, lane: o.lane ?? (o.min ? 26 : 38) };
}

// ---------- top bar ----------
// Master listening (boards 9l–9t): the level mode (real level by default, same loudness on demand, ⇧B) and the A/B
// source. The offsets of each source are in the Loudness card (s-master.mjs). `o.listen = { same, names }`.
export function listen(o = {}) {
  const l = o.listen || {}, names = l.names || ['Master', 'Mixdown', 'Reference A'], at = o.ab ?? 0;
  const mode = `<div class="seg quiet lmode" role="radiogroup" aria-label="Listening level"><button class="${l.same ? '' : 'on'}" role="radio" aria-checked="${!l.same}">Real level</button><button class="${l.same ? 'on same' : ''}" role="radio" aria-checked="${!!l.same}">${ic('level')}Same loudness</button></div>`;
  const src = `<div class="seg" role="radiogroup" aria-label="A/B source">${names.map((n, i) => `<button class="${i === at ? 'on' : ''}" role="radio" aria-checked="${i === at}">${n}</button>`).join('')}</div>`;
  return `<div class="fx g2${o.wait || o.hold ? ' is-dis' : ''}">${mode}${src}</div>`;
}

export function top(o = {}) {
  const master = o.phase === 'master', playing = o.playing !== false;
  const right = master
    ? `<div class="grow"></div>${listen(o)}${btn('Export', 'primary' + (o.wait || o.hold ? ' is-dis' : ''), 'exp')}`
    : `<div class="grow"></div><div class="fx g3" aria-label="Mix bus">
        <div class="col g1" style="width:var(--sp-8)"><div class="hbar"><i style="width:${playing ? 78 : 0}%"></i></div><div class="hbar"><i style="width:${playing ? 74 : 0}%"></i></div></div>
        <div class="col"><span class="faint s1">True peak</span><span class="b">${playing ? '−3.1' : '−∞'} dBTP</span></div>
        <div class="col"><span class="faint s1">Loudness</span><span class="b">${playing ? '−16.4' : '−∞'} LUFS</span></div>
        <div class="col"><span class="faint s1">Engine</span><span class="b">${o.engine || '18 %'}</span></div></div>`;
  return `<header class="top">
    <div class="fx g2">${L.mark()}<span class="h nowrap">${o.project || 'Paper Lanterns'}</span><span class="faint s1 nowrap">${o.status || (master ? 'Mixdown 3' : 'Saved 21:47')}</span></div>
    ${seg(o.phaseItems || ['Mix', 'Master'], master ? 1 : 0, 'phase', 'Phase')}
    <div class="fx g2${o.wait ? ' is-dis' : ''}">${ib('rew', 'Return to start')}<button class="playb ${playing ? '' : 'idle'}" aria-label="${playing ? 'Pause' : 'Play'}">${ic(playing ? 'pause' : 'play')}</button>${ib('stop', 'Stop')}${ib('loop', 'Loop', o.loop === false ? '' : 'on')}</div>
    <div class="fx g2"><span class="pos">${o.pos || '1:02.48'}</span><span class="faint s1 nowrap">/ ${o.dur || '3:42'}</span>${o.noSection ? '' : tag(o.section || 'Chorus')}</div>
    <div class="fx g1${o.wait || o.hold ? ' is-dis' : ''}">${ib('undo', 'Undo')}${ib('redo', 'Redo', o.redo ? '' : 'is-dis')}</div>
    ${o.mid || ''}${right}</header>`;
}

// ---------- timeline ----------
export const tools = (o = {}) => `<div class="tools">
  <div class="seg quiet sm" role="radiogroup" aria-label="Tool">${['cursor', 'cut', 'pen'].map((n, i) => `<button class="${(o.tool || 0) === i ? 'on' : ''}" aria-label="${['Select', 'Split', 'Automation'][i]}">${ic(n)}</button>`).join('')}</div>
  <button class="tag ${o.snap === false ? '' : 'acc'}">${ic('snap', 'sm')}${o.snap === false ? 'Snap off' : 'Snap · ' + (o.snapTo || 'sections')}</button>
  <div class="fx gh">${ib('minus', 'Zoom out')}${ib('plus', 'Zoom in')}</div>
  <div class="grow"></div><span class="faint s1">${o.count || '12 tracks · 48 kHz'}</span>${btn('Add track', 'quiet sm', 'plus')}</div>`;

export const defaultRows = (o = {}) => { const r = []; L.tracks.forEach((t, i) => { r.push({ k: 't', i }); if (i === 10 && !o.noAuto) r.push({ k: 'a', i: 10 }); }); return r; };

export function timeline(o = {}) {
  const g = geo(o), w = o.w ?? g.tlw, head = g.head, TT = o.T ?? L.T, SECS = o.sections ?? L.sections, LOOP = o.loopAt ?? L.loop, pps = w / TT, lane = g.lane, small = lane < 32;
  const sz = small ? 's' : lane >= 56 ? 'l' : '', waveH = small ? lane - 8 : lane - 4 - 13 - 3, st = o.state || {}, sel = o.sel === undefined ? 10 : o.sel;
  const rows = o.rows || defaultRows(o), TR = o.tracks || L.tracks, CT = o.clips || L.clipTimes, gopt = { rel: o.relSend, returns: o.returns };
  const hdCtl = (s, i) => `${tgl('mu', s.mu)}${tgl('so', s.so, s.implied ? 'implied' : '')}${tgl('lk', s.lk)}<div class="hbar mini"><i style="width:${i}%"></i></div>`;
  const out = rows.map((r) => {
    if (r.k === 't') {
      const t = TR[r.i], tint = t.tint || r.i + 1, s = Object.assign({ lk: t.lock }, st[r.i] || {});
      const clips = (o.noClips ? [] : CT[r.i]).map(([c0, c1], ci) => {
        const m = (o.clipMod && o.clipMod(r.i, ci)) || {}; if (m.hide) return '';
        const a = m.c0 ?? c0, b = m.c1 ?? c1, wv = L.wave(r.i, a, b, pps, waveH, 2.5, SECS);
        return `<div class="clip ${m.cls || ''} ${s.mu && !m.cls ? 'is-muted' : ''}" style="left:${px(a * pps + (m.dx || 0))};width:${px(wv.w - 1)}"><span class="cn">${m.name || t.n}</span><svg class="wf" width="${f1(wv.w)}" height="${waveH}" aria-hidden="true"><path d="${wv.d}"/></svg>${m.extra || ''}</div>`;
      }).join('');
      return `<div class="row t${tint} ${sz} ${sel === r.i ? 'is-sel' : ''} ${s.lk ? 'is-lock' : ''} ${r.cls || ''}">
        <div class="hd d${r.d || 0}"><i class="tdot"></i><div class="names"><span class="nm ell">${t.n}</span><span class="rl ell">${r.rel || t.r}</span></div>${t.note && o.notes !== false ? badge(t.note) : ''}${L.glance(t, gopt)}${hdCtl(s, L.fpos(t.db))}</div>
        <div class="ln">${clips}${(o.laneExtra && o.laneExtra(r.i)) || ''}</div></div>`;
    }
    if (r.k === 'g') return `<div class="row grp tb ${sz} ${r.cls || ''}">
        <div class="hd d${r.d || 0}">${ic(r.open === false ? 'right' : 'chev', 'm')}<div class="names"><span class="nm ell">${r.n}</span><span class="rl ell">${r.rel || r.sub || 'group'}</span></div>${L.glance(r.gl || L.buses.find((b) => b.n === r.n) || { ch: [0, 0, 1, 1, 0] }, gopt)}${hdCtl(r.s || {}, 70)}</div>
        <div class="ln">${(r.spans || [[0, 144], [166, 222]]).map(([a, b]) => `<i class="gsum" style="left:${px(a * pps)};width:${px((b - a) * pps)}"></i>`).join('')}</div></div>`;
    if (r.k === 'r') return `<div class="row tb ${sz} ${r.cls || ''}"><div class="hd d${r.d || 0}">${ic(r.icon || 'wave', 'm')}<div class="names"><span class="nm ell">${r.n}</span><span class="rl ell">${r.sub}</span></div><div class="gl"></div>${hdCtl({}, 55)}</div><div class="ln">${r.feeds ? `<div class="feeds"><span>${r.feeds.length} tracks send here</span>${r.feeds.map((f) => `<span class="feed t${f[0]}"><i class="tdot"></i>${f[1]}<b>${f[2]}</b></span>`).join('')}</div>` : `<span class="lane-note">${r.note || ''}</span>`}</div></div>`;
    if (r.k === 'a') {
      const a = L.autoGeo(pps, 56, r.vals, SECS, TT), ai = r.by !== 'user';
      return `<div class="row auto t${r.i + 1} ${ai ? 'by-ai' : ''} ${r.cls || ''}">
        <div class="hd d1">${r.picker || `<div class="names"><span class="b ${ai ? 'aic' : 'acc'} ell">${r.label || 'Volume over time'}</span><span class="rl ell">${r.sub || 'Set per section · note 4'}</span></div>`}<span class="s1 b">${r.val || '−5.8 dB'}</span></div>
        <div class="ln"><svg width="${w}" height="56" aria-hidden="true"><path class="ar" d="${a.fill}"/><path class="al" d="${a.line}"/></svg>${a.pts.map((p, i) => `<i class="dot ${ai ? 'ai' : ''} ${(r.selPts || []).includes(i) ? 'is-sel' : ''} ${r.hoverPt === i ? 'is-hover' : ''}" style="left:${px(p[0])};top:${px(p[1])}"></i>`).join('')}${r.extra || ''}</div></div>`;
    }
    return r.html || '';
  }).join('');
  const secs = o.noSections ? '' : SECS.map((s, i) => `<span class="sec ${s[0] === 'Chorus' ? 'hot' : ''} ${(o.secCls && o.secCls[i]) || ''}" style="left:${px(s[1] * pps)};width:${px((s[2] - s[1]) * pps - 1)}">${s[0]}${(o.secExtra && o.secExtra[i]) || ''}</span>`).join('');
  let ticks = ''; for (let t = 0; t < TT; t += o.tick || 15) ticks += `<span class="tick" style="left:${px(t * pps)}">${L.fmt(t)}</span>`;
  const lp = o.loop === false ? '' : `<div class="loop" style="left:${px(LOOP[0] * pps)};width:${px((LOOP[1] - LOOP[0]) * pps)}"></div>`;
  return `<div class="tl" style="--head-w:${head}px">
    <div class="row ruler"><div class="hd"></div><div class="ln">${lp}${ticks}</div></div>
    <div class="row secs"><div class="hd"><span class="faint s1">Song sections</span></div><div class="ln">${secs}</div></div>
    ${out}${o.play === false ? '' : `<div class="play ${o.playing === false ? 'idle' : ''}" style="left:${px(head + (o.playAt ?? L.playhead) * pps)}"></div>`}${o.tlOver || ''}</div>`;
}

// ---------- mixer ----------
// Every strip carries the glance state of its track (P-013): chain marks under the colour bar, one send row per return.
// `o.view === 'grid'` is the Overview: same columns, the faders replaced by one labelled cell per block and per return.
export function allStrips() {
  const tr = L.tracks.map((t, i) => ({ n: t.n, tint: 't' + (i + 1), pan: 0.5 + t.pan / 2, pk: Math.round(t.lv * 100), fy: L.fpos(t.db), gr: t.k === 's' ? Math.round(t.lv * 14) : Math.round(10 + t.lv * 34), db: L.dB(t.db), i, ch: t.ch, v: t.v, sc: t.sc, par: t.par, sn: t.sn, au: t.au, out: t.out }));
  const bs = L.buses.map((b) => ({ n: b.n, tint: b.t === 'mix bus' ? 'tm' : 'tb', bus: true, kind: b.t, pan: 0.5, pk: Math.round(b.lv * 100), fy: L.fpos(b.db), gr: b.t === 'group' ? 22 : 0, db: L.dB(b.db), ch: b.ch, v: b.v, par: b.par, sn: b.sn, ret: b.ret, feeds: b.feeds, mix: b.t === 'mix bus' }));
  return tr.concat(bs);
}
const BASE_BLOCKS = ['High-pass', 'Low-pass', 'EQ', 'Compressor', 'Saturation'];
const cells = (s, o, rets) => {
  const BLOCKS = o.blockRows ? o.blockRows.map((r) => r[0]) : BASE_BLOCKS;
  if (s.mix) return `<div class="cells">${BLOCKS.map(() => '<div class="cl none"></div>').join('')}${rets.map((r, i) => `<div class="cl none ${i ? '' : 'gap'}"></div>`).join('')}<div class="cl none gap"></div><div class="cl none"></div></div>`;
  const v = s.v || [], ch = s.ch || [];
  const blocks = s.ret !== undefined
    ? [`<div class="cl on">${v[0]}</div>`, `<div class="cl on">${v[1]}</div>`, `<div class="cl on">${v[2].split(' · ')[0]}</div>`, `<div class="cl on">${v[2].split(' · ')[1]}</div>`, '<div class="cl none"></div>']
    : BLOCKS.map((b, i) => (ch[i] === 1
      ? `<div class="cl on ${o.hover && o.hover[0] === s.n && o.hover[1] === i ? 'is-hover' : ''}">${i === 3 && s.sc !== undefined ? `<i class="led t${s.sc + 1}"></i>` : ''}${v[i]}${i === 3 && s.par ? `<span class="dim">${s.par}%</span>` : ''}</div>`
      : ch[i] === 2 ? `<div class="cl nu">${v[i] || 'flat'}</div>` : '<div class="cl off">off</div>'));
  while (blocks.length < BLOCKS.length) blocks.push('<div class="cl none"></div>');
  const sends = rets.map((r, i) => (s.ret === i ? `<div class="cl on ${i ? '' : 'gap'}">${s.feeds} in</div>` : s.ret !== undefined ? `<div class="cl none ${i ? '' : 'gap'}"></div>`
    : s.sn && s.sn[i] ? `<div class="cl on sn ${i ? '' : 'gap'} ${o.rel === i ? 'rel' : ''}"><i style="width:${s.sn[i][0]}%"></i><span>${s.sn[i][1]}</span></div>` : `<div class="cl off ${i ? '' : 'gap'}">·</div>`));
  return `<div class="cells">${blocks.join('')}${sends.join('')}<div class="cl gap on">${s.out || s.parent || 'Mix'}</div>${s.au ? `<div class="cl on ${s.au[0] === 'ai' ? 'ai' : 'acc'}">${s.au[1]}</div>` : '<div class="cl off">·</div>'}</div>`;
};
export const strip = (s, o = {}) => {
  const rets = o.returns || L.returns, sn = s.sn || [], grid = o.view === 'grid';
  const head = `<span class="nm ell">${s.n}</span>${o.kinds && !grid ? `<span class="kind ell">${s.kind || (s.parent ? '→ ' + s.parent : '→ Mix')}</span>` : ''}<i class="cbar"></i>`;
  const sends = `<div class="snd" aria-label="Sends">${rets.map((r, i) => `<div>${s.ret === i ? '<i class="self"></i>' : s.mix || s.ret !== undefined ? '' : `<div class="hbar ${o.rel === i && sn[i] ? 'rel' : ''}"><i style="width:${sn[i] ? sn[i][0] : 0}%"></i></div>`}</div>`).join('')}</div>`;
  const body = grid ? cells(s, o, rets)
    : `${L.chainBlock(s, o.chain)}${knob(s.pan, { size: 's', bip: true })}${sends}<div class="fz">${meter(o.silent ? 0 : s.pk)}${fader(s.fy, s.fcls || '')}${grm(o.silent ? 0 : s.gr)}</div>`;
  return `<div class="strip ${s.tint} ${s.bus ? 'bus' : ''} ${s.child ? 'child' : ''} ${o.sel === s.i ? 'is-sel' : ''} ${s.cls || ''}">${head}${body}
  ${grid ? '' : `<span class="db">${s.db}</span>`}<div class="fx gh">${tgl('mu', s.mu, 'sm')}${tgl('so', s.so, 'sm')}</div></div>`;
};
// Row labels on the left of the mixer, and the switch between the two views.
export const rail = (o = {}) => {
  const rets = o.returns || L.returns, grid = o.view === 'grid', rel = o.relSend ?? o.rel;
  const labels = grid
    ? `<div class="cells">${(o.blockRows || BASE_BLOCKS.map((b, i) => [b, L.SIG[i]])).map((b) => `<div class="cl">${b[0]}${L.sigx(b[1], 'nu', b[0])}</div>`).join('')}${rets.map((r, i) => `<div class="cl ${i ? '' : 'gap'} ${rel === i ? 'rel' : ''}">${r}</div>`).join('')}<div class="cl gap">Output</div><div class="cl">Automation</div></div>`
    : `<div class="lb pp-h" style="--chain-h:${L.CHAIN_H[o.chain || 'col']}px">Chain</div><div class="lb kn-h">Pan</div><div class="snd">${rets.map((r, i) => `<div class="lb ${rel === i ? 'rel' : ''}">${r}</div>`).join('')}</div>`;
  return `<div class="strip mxrail ${grid ? 'wide' : ''}"><span class="nm">&nbsp;</span>${o.kinds && !grid ? '<span class="kind">&nbsp;</span>' : ''}<i class="cbar" style="visibility:hidden"></i>${labels}
  <div class="seg quiet sm" role="radiogroup" aria-label="Mixer view"><button class="${grid ? '' : 'on'}" role="radio" aria-checked="${!grid}" aria-label="Faders">${ic('faders')}</button><button class="${grid ? 'on' : ''}" role="radio" aria-checked="${grid}" aria-label="Overview">${ic('grid')}</button></div></div>`;
};
export const mixer = (o = {}) => {
  const all = o.strips || allStrips(), so = { blockRows: o.blockRows, sel: o.sel === undefined ? 10 : o.sel, kinds: o.kinds, silent: o.playing === false, view: o.view, rel: o.relSend, returns: o.returns, hover: o.hover, chain: o.chain };
  const bus = all.filter((s) => s.mix), rest = all.filter((s) => !s.mix);
  return `<section class="card mixer" aria-label="Mixer">${rail(o)}<div class="mx-scroll">${rest.map((s) => strip(s, so)).join('')}</div>${bus.map((s) => strip(s, so)).join('')}</section>`;
};

// ---------- inspector ----------
export const CHAIN = [['Trim', '−2.4 dB'], ['HPF', '90 Hz'], ['LPF', 'Off', 'is-off'], ['EQ', '4 bands', 'is-open'], ['Comp', '3.0:1'], ['Sat', '4.0 dB'], ['Fader', '−5.8'], ['Sends', '2']];
export const chain = (items = CHAIN) => `<div class="chain" aria-label="Signal chain">${items.map((c) => `<button class="blk ${c[2] || ''}"><span class="b">${c[0]}</span><span>${c[1]}</span></button>`).join('')}</div>`;
export function eqGraph(w, h, bands = L.voxEq, o = {}) {
  const e = L.eqGeo(w, h, bands, o.hpf ?? 90, o.range);
  return `<div class="eq" style="${o.style || ''}"><svg width="${w}" height="${h}" aria-hidden="true">${o.grid ? `<path class="gd" d="${e.grid}"/>` : ''}<path class="sp" d="${L.spectrum(w, h, o.seed || 7)}"/>${o.ref ? `<path class="rf" d="${L.spectrum(w, h, o.seed || 7, 0.42, true)}"/>` : ''}<path class="zr" d="${e.zero}"/><path class="ar" d="${e.fill}"/><path class="cv" d="${e.line}"/></svg>
    ${e.handles.map((p, i) => `<i class="dot ${o.selBand === i ? 'is-sel' : ''}" style="left:${px(p[0])};top:${px(p[1])}"></i>`).join('')}${e.ticks.map((k) => `<span class="ft" style="left:${px(k[0])}">${k[1]}</span>`).join('')}${o.extra || ''}</div>`;
}
export const bandPills = (o = {}) => `<div class="fx g1">${[['Mud', '−2.5 dB'], ['Presence', '+2.0 dB'], ['Harsh', '−1.5 dB'], ['Air', '+2.5 dB']].map((b, i) => `<div class="pill ${o.sel === i ? 'on' : ''}"><span class="faint s1">${b[0]}</span><span class="b s1">${b[1]}</span></div>`).join('')}</div>`;
export const compBlock = (o = {}) => `<div class="sect"><div class="fx g2">${tgl('by', true)}<span class="b grow">Compressor</span>${o.sc || `<button class="sc">${ic('link', 'sm')}Sidechain · none</button>`}</div>
  <div class="fx g3"><div class="tc-curve">${L.compGeo()}</div><div class="knobs">${kb('Threshold', '−18.0', 0.62)}${kb('Ratio', '3.0', 0.3)}${kb('Attack', '10', 0.25)}${kb('Release', '120', 0.42)}${kb('Makeup', '+3.0', 0.6, { bip: true })}</div></div>
  <div class="fx g2"><span class="faint s1">Knee</span>${L.field('6.0', 'dB')}<span class="faint s1">Mix</span>${L.field(o.mix || '100', '%')}<span class="faint s1 grow">${o.mix ? 'Parallel' : 'Stereo-linked'}</span></div></div>`;
export const whyNote = (o = {}) => `<div class="sect mt-auto"><div class="note on colm"><div class="fx g2">${badge(4)}<span class="b">Why this track changed</span></div><p class="why">${L.WHY}</p><div class="col">${L.diffRows()}</div></div></div>`;
export function inspector(o = {}) {
  const g = geo(o), w = g.side - 24, ti = o.ti ?? 10, t = L.tracks[ti];
  return `<aside class="card side t${ti + 1}" aria-label="Inspector">
    <div class="ph"><i class="tdot"></i><span class="h ell">${t.n}</span>${tag(t.r)}<div class="grow"></div><button class="route ${o.routeOpen ? 'is-open' : ''}" aria-label="Output">${ic('right', 'sm')}${o.out || 'Mix'}${ic('chev', 'sm')}</button></div>
    ${chain(o.chain)}
    <div class="sect"><div class="fx g2">${tgl('by', true)}<span class="b grow">EQ</span><span class="faint s1">drag a handle · scroll for Q</span></div>${eqGraph(w, 112, L.voxEq, Object.assign({ style: 'height:112px' }, o.eq))}${bandPills()}</div>
    ${compBlock(o)}
    <div class="sect"><div class="fx g4"><div class="col g2 grow"><span class="fx g2">${tgl('by', true)}<span class="b">Saturation</span></span><div class="knobs">${kb('Drive', '4.0 dB', 0.22)}${kb('Mix', '35 %', 0.35)}</div></div>
      <div class="col g2 grow"><span class="b">Sends</span><div class="knobs">${kb('Plate', '−14.0 dB', 0.62)}${kb('Echo', '−22.0 dB', 0.38)}</div></div></div></div>
    ${o.min || o.noWhy ? '' : whyNote(o)}${o.inspOver || ''}</aside>`;
}

// ---------- assistant ----------
const asstHead = (tab) => `<div class="ph"><span class="h">Assistant</span><div class="grow"></div>${seg(['Notes', 'History', 'Snapshots'], tab, 'quiet sm')}</div>`;
export const brief = () => `<div class="sect"><div class="fx g1 wrap">${tag('Indie rock')}${tag('Warm')}${tag('Intimate')}${tag('2 references')}<button class="tag">${ic('pen', 'sm')}Edit brief</button></div><p class="dim">Vocal up front. Dry drums. Guitars wide but never harsh.</p></div>`;
export const askFoot = (o = {}) => `<div class="foot">
  <div class="ask ${o.asking ? 'is-edit' : ''}"><span class="grow">${o.asking || 'Tell the assistant what to change: “vocal a touch warmer in the verses”'}</span>${ic('send')}</div>
  <div class="fx between">${seg(['Lead Vox', 'All tracks'], o.scope ?? 0, 'quiet sm', 'Scope')}${btn(o.cta || 'Auto mix', 'primary ' + (o.ctaCls || ''), 'spark')}</div>
  <div class="fx g3 s1"><span class="dim">Allow</span><span class="fx g1">${L.check(!o.noStruct)}<span class="dim">structural changes</span></span><span class="fx g1">${L.check(!o.noIdeas)}<span class="dim">creative ideas</span></span></div>
  <div class="fx between"><span class="faint s1">This request about ${o.est || '$0.04'} · ${o.spent || '$0.41 so far'}</span><a href="12-ai-usage.html" class="s1">AI usage</a></div></div>`;
export const HISTORY = [['Lead Vox · EQ band 2 gain', '21:47', 'now'], ['Auto mix applied · 11 tracks', '21:44', 'ai'], ['Section “Bridge” resized', '21:39'], ['Gtr Right · clip split', '21:38'], ['Rhodes · fade out', '21:36'], ['Sections created · 8', '21:33'], ['Import 12 files', '21:31']];
export function assistant(o = {}) {
  const tab = o.tab || 0;
  let body;
  if (o.asst === 'working') body = `<div class="sect"><div class="fx between"><span class="b">Auto mix in progress</span><span class="faint s1">0:42 elapsed</span></div>${L.progress(46, 'ai')}
      <div class="steps"><div class="step done"><span class="n">${ic('check', 'sm')}</span><span class="grow">Analysis · 12 tracks, 8 sections</span><span class="s1 faint">6 s</span></div>
        <div class="step done"><span class="n">${ic('check', 'sm')}</span><span class="grow">Pass 1 · decide, render, evaluate</span><span class="s1 faint">distance 3.4</span></div>
        <div class="step now"><span class="n">2</span><span class="grow">Pass 2 · rendering offline</span><span class="s1">61 %</span></div>
        <div class="step"><span class="n">3</span><span class="grow">Pass 3 · only if it still improves</span></div></div></div>
    <div class="sect"><div class="note colm"><span class="b">Pass 2 of up to 3</span><div class="fx g2 wrap">${tag(ic('check', 'sm') + 'Deciding', 'ok')}${tag('Rendering', 'ai')}${tag('Evaluating')}</div><span class="dim">Distance to reference so far 4.8 → 3.4. The best pass is kept even if a later one is worse.</span></div>
      <span class="faint s1">Nothing changes in your mix until you review and apply the proposal. You can keep working meanwhile.</span></div>
    <div class="foot"><div class="fx between"><span class="faint s1">Spent so far on this run $0.07</span>${btn('Cancel', '', 'x')}</div><div class="fx between"><span class="faint s1">AI cost so far $0.48 · 8 calls</span><a href="12-ai-usage.html" class="s1">AI usage</a></div></div>`;
  else if (tab === 1) body = `<div class="sect"><div class="fx between"><span class="faint s1">Every step can be undone. Click a step to go back to it.</span></div>
      <div class="col">${HISTORY.map((h, i) => `<div class="mi ${h[2] === 'now' ? 'is-hover' : ''} ${o.histAt !== undefined && i < o.histAt ? 'is-dis' : ''}">${h[2] === 'ai' ? ic('spark', 'm aic') : `<span class="chk">${h[2] === 'now' ? ic('right', 'm') : ''}</span>`}<span class="grow ell">${h[0]}</span><span class="s1 faint">${h[1]}</span></div>`).join('')}</div></div>
    <div class="foot"><div class="fx g2">${btn('Undo', '', 'undo')}${btn('Redo', 'is-dis', 'redo')}</div><span class="faint s1">Mix and Master keep separate histories. Mute, solo, zoom and selection are not recorded.</span></div>`;
  else if (tab === 2) body = `<div class="sect"><div class="fx between"><span class="faint s1">Named states of the mix, saved in the project.</span>${btn('Save snapshot', 'sm', 'plus')}</div>
      ${[['Current mix', 'now', 'A'], ['After auto mix', '21:44', ''], ['Before auto mix', '21:43', 'B'], ['Rough balance', '21:35', '']].map((s) => `<div class="tile row ${s[2] ? 'on' : ''}"><div class="col grow"><span class="b ell">${s[0]}</span><span class="faint s1">${s[1]}</span></div>${s[2] ? tag(s[2], 'acc') : btn('Restore', 'quiet sm')}</div>`).join('')}</div>
    <div class="sect"><span class="b">Compare while listening</span><div class="fx g2">${seg(['A · Current mix', 'B · Before auto mix'], o.abAt ?? 0, '', 'A/B')}</div><span class="faint s1">Switching does not stop playback and is not an undo step. Shortcut ${'<span class="kbd">B</span>'}</span></div>
    <div class="foot"><span class="faint s1">Restoring a snapshot is one undoable step.</span></div>`;
  else body = `${brief()}<div class="sect"><div class="fx between"><span class="b">Auto mix · applied 21:44</span><button class="tag">${ic('ab', 'sm')}A/B with before</button></div><p class="why dim">${L.SUMMARY}</p><span class="faint s1">Distance to reference 4.8 → 2.1 · 11 tracks changed · 1 locked</span></div>
    <div class="sect">${L.NOTES.map((n, i) => `<div class="note ${i === 0 ? 'on' : ''} t${n[2]}">${badge(n[0])}<div class="col g1 grow"><span class="fx g2">${i === 0 ? '<i class="tdot"></i>' : ''}<span class="b">${n[1]}</span></span><span class="why dim">${n[3]}</span></div></div>`).join('')}</div>${askFoot(o)}`;
  return `<aside class="card side" aria-label="Assistant">${asstHead(o.asst === 'working' ? 0 : tab)}${o.asstBody || body}</aside>`;
}

// ---------- shell ----------
export const app = (o, inner) => `<div class="sw ${o.min ? 'min' : ''}" data-phase="${o.phase || 'mix'}">${inner}</div>`;
export function mixScreen(o = {}) {
  return app(o, `${top(o)}<div class="body"><div class="left"><section class="card" aria-label="Timeline">${tools(o)}${timeline(o)}</section>${o.mixer ?? mixer(o)}</div>${o.insp ?? inspector(o)}${o.asstPanel ?? assistant(o)}</div>${o.over || ''}`);
}
export const frame = (id, title, note, html) => `<section class="board" id="${id}"><header class="cap"><span class="cap-id">${id}</span><h2>${title}</h2><p>${note}</p></header>${html}</section>`;
export const page = (title, intro, frames, o = {}) => `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Stemwise design · ${title}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:opsz,wght@12..96,500;12..96,600&family=Figtree:wght@400;500;600&display=swap" rel="stylesheet">
<link rel="stylesheet" href="${o.up || '..'}/tokens.css">
<link rel="stylesheet" href="${o.up || '..'}/components/components.css">
<link rel="stylesheet" href="${o.up || '..'}/screens/screens.css">
</head>
<body class="boards">
<header class="doc"><a href="${o.up || '..'}/README.md">Stemwise design</a><h1>${title}</h1><p>${intro}</p></header>
${frames.join('\n')}
</body>
</html>
`;
