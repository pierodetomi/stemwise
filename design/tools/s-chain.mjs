// Screen 15 — Master chain (P-017): the chain as a rack of repeatable blocks beside the editor, clipper, multiband
// compressor with two to six bands, parallel compression, section offsets, the measured proposal and the way back to
// the mix. Also the exploration that led to the rack (grid, rack, path).
import * as L from './lib.mjs';
import * as P from './parts.mjs';
import { master } from './s-master.mjs';
const { ic, tag, btn, badge, seg, field, flabel, check, tgl, px, kb, menu } = L;

const G = P.geo({});
const RACK = 288, EW = G.left - RACK - 8 - 24; // rack width (the --rack-w token) and inner width of the editor beside it
const LOUD = { target: 'Custom · −9.6 LUFS · −1 dBTP', tgtLufs: '−9.6', integ: '−9.6', refs: 'Reference A', mixText: '−20.1 LUFS, −3.0 dBTP',
  listen: { rows: [['Master', '−9.6', 'as exported'], ['Mixdown', '−20.1', '10.5 LU quieter'], ['Reference A', '−9.6', 'same loudness']] } };

// ---------- the chain: Input · free, repeatable blocks · Limiter, Dither ----------
// [name, value, state, fixed, note, dB it takes off, detail, peaks-above-body after it]
const CH = [['Input', '0.0 dB', '', 1, 0, 0, 'Gain into the chain', 17.1], ['EQ 1', '2 bands', '', 0, 1, 0.3, 'Corrective · −1.5 dB at 45 Hz', 16.8], ['Compressor', '3:1 parallel', '', 0, 2, 2.6, '−2.6 dB on the peaks', 14.2],
  ['Multiband', '3 bands', '', 0, 3, 0.5, 'Low −2.1 dB', 13.7], ['EQ 2', '2 bands', '', 0, 0, 0, 'Tone · +1.0 dB at 9 kHz', 13.9], ['Match EQ', '40 %', '', 0, 0, 0, 'Toward Reference A', 14.0], ['Width', '+1.0 dB side', '', 0, 0, 0, 'Mono below 100 Hz', 14.5],
  ['Saturation', 'Off', 'is-off', 0, 0, 0, 'Bypassed', 14.5], ['Clipper', 'Up to 4.0 dB', '', 0, 4, 4.0, '0.6 % of the song', 10.5], ['Limiter', '−1.0 dBTP', '', 1, 5, 1.9, '−1.9 dB at most', 8.6], ['Dither', '16-bit only', 'is-off', 1, 0, 0, 'TPDF at export', 8.6]];
const FREE = 9; // index of the first fixed block at the end
const rrow = (m, i, o = {}) => `<div class="rrow ${m[2]} ${o.open === m[0] ? 'on' : ''} ${m[3] ? 'is-fixed' : ''} ${o.cls || ''}">${m[3] ? ic('lock', 'sm faint') : ic('dots', 'sm faint')}<span class="faint s1 rn">${i + 1}</span><span class="b ell rname">${m[0]}</span><span class="dim s1 ell grow">${m[1]}</span>${m[5] ? `<span class="hbar gr rgr"><i style="width:${Math.min(100, m[5] * 20)}%"></i></span>` : ''}${m[4] && !o.noNotes ? badge(m[4]) : ''}<i class="led ${m[2] === 'is-off' ? 'off' : ''}"></i></div>`;
const radd = (o = {}) => `<button class="radd ${o.addOpen ? 'on' : ''}">${ic('plus', 'm')}Add module<span class="faint s1 ml-auto">${o.count || '11 of 12'}</span></button>`;
const rack = (o = {}) => { const list = o.list || CH, free = list.findIndex((m, i) => i > 0 && m[3]); return `<section class="card rack" aria-label="Master chain"><div class="ph"><span class="h">Chain</span><span class="faint s1 grow">${o.sub || 'top to bottom'}</span>${btn('Reset', 'quiet sm')}</div>
  <div class="sect grow rrows">${list.slice(0, free).map((m, i) => (o.rowAt ? o.rowAt(m, i) : rrow(m, i, o))).join('')}${radd(o)}${list.slice(free).map((m, i) => rrow(m, i + free, { ...o, cls: o.dimFixed ? 'is-dim' : '' })).join('')}${o.pop || ''}
    <span class="faint s1 rhint">${o.hint || `${ic('dots', 'sm')} drag to reorder · ${ic('lock', 'sm')} stays in place`}</span></div></section>`; };
const none = '<i hidden></i>';

// ---------- editors (beside the rack: EW wide) ----------
const ehead = (name, n, sub, right = '') => `<div class="ph">${tgl('by', true)}<span class="h">${name}</span>${n ? badge(n) : ''}<span class="faint s1 grow ell">${sub}</span>${right}</div>`;
const grRow = (l, v, p) => `<div class="fx g2"><span class="faint s1 grl">${l}</span><div class="hbar gr grow"><i style="width:${p}%"></i></div><span class="b s1 grv">${v}</span></div>`;
const info = (t, extra = '') => `<div class="tile row">${ic('info')}<span class="dim s1">${t}</span>${extra}</div>`;

// Clipper: the transfer curve, where the peaks of this master are, and what it takes off the limiter.
const clipCurve = () => { const t = 72; return `<svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true"><path class="un" d="M0 100L100 0"/><path class="th" d="M0 ${100 - t}H100"/><path class="cv" d="M0 100L${t - 8} ${100 - t + 8}Q${t} ${100 - t} ${t + 10} ${100 - t}L100 ${100 - t}"/></svg>`; };
const HIST = [2, 3, 5, 8, 12, 18, 26, 38, 52, 68, 84, 96, 100, 92, 74, 52, 33, 19, 10, 6, 4, 3, 2, 2, 1, 1];
const clipperEd = (c = {}) => `<section class="card grow" aria-label="Clipper">${ehead('Clipper', 4, 'takes the top off the tallest peaks', seg(['Soft', 'Hard'], 0, 'quiet sm', 'Shape'))}
  <div class="sect grow"><div class="fx g3 top-al"><div class="tc-curve lg">${clipCurve()}</div>
    <div class="col g2 grow"><span class="faint s1">Where the peaks of this master sit · right of the line is clipped</span>
      <div class="hist"><div class="bars">${HIST.map((h, i) => `<i class="${i >= 21 ? 'cut' : 'dimd'}" style="height:${h}%"></i>`).join('')}</div><i class="cutline" style="left:${(21 / HIST.length) * 100}%"></i><span class="cutlbl" style="left:${(21 / HIST.length) * 100}%">${c.take || '4.0'} dB under the tallest peak</span></div>
      <div class="fx between faint s1"><span>quiet</span><span>body of the mix</span><span>tallest peaks</span></div></div></div>
    <div class="fx g4 end-al"><div class="knobs clipk">${kb('Take off up to', (c.take || '4.0') + ' dB', c.kv || 0.4)}${kb('Softness', '40 %', 0.4)}</div>
      <div class="col g1 grow">${grRow('Clipper', c.cgr || '−4.0 dB', c.cp || 40)}${grRow('Limiter', c.lgr || '−1.9 dB', c.lp || 19)}</div></div>
    <span class="faint s1">${c.line || 'Without the clipper the limiter would take 6.0 dB and work over 3 dB for 21 % of the song.'}</span>
    ${info(c.info || '“Take off up to” is how many dB come off the tallest peaks that reach the clipper, wherever it sits in the chain and whatever the target. Here that is 0.6 % of the song, all of it short transients: the body is untouched. Runs at four times the rate.')}</div></section>`;
const clipper = clipperEd();

// Compressor on the master, with the dry/wet mix and the detector high-pass.
const comp = `<section class="card grow" aria-label="Compressor">${ehead('Compressor', 2, 'stereo-linked', tag(ic('level', 'sm') + 'Parallel · 60 %', 'acc'))}
  <div class="sect grow"><div class="fx g3"><div class="tc-curve lg">${L.compGeo(-9, 3, 4)}</div>
      <div class="col g3 grow">${grRow('Reduction', '−2.6 dB', 26)}<div class="fx g3 end-al">${flabel('Knee', field('4.0', 'dB'))}${flabel('Listens above', field('120', 'Hz', 'is-edit'))}</div>
        <span class="faint s1">It does not react to what is below 120 Hz, so the low end does not make the whole mix duck. Off lets it hear everything.</span></div></div>
    <div class="knobs">${kb('Threshold', '−9.0', 0.78)}${kb('Ratio', '3.0', 0.5)}${kb('Attack', '3', 0.12)}${kb('Release', '90', 0.3)}${kb('Makeup', '0.0', 0.5, { bip: true })}${kb('Mix', '60 %', 0.6, { cls: 'is-active' })}</div>
    ${info('Mix at 60 %: the compressed signal is blended with 40 % of the untouched one. The peaks keep their attack and the quiet details come up. At 100 % it is an ordinary compressor.')}</div></section>`;

// Multiband: two to six bands on the spectrum, crossovers as handles, reduction hanging from the top of each band.
const fx = (f, w) => (w * Math.log(f / 20)) / Math.log(1000);
const hz = (f) => (f >= 1000 ? (f / 1000).toFixed(1) + ' kHz' : f + ' Hz');
function mbGraph(w, h, xos, grs, selBand) {
  const xs = [0, ...xos.map((f) => fx(f, w)), w];
  const ticks = [50, 100, 200, 500, 1000, 2000, 5000, 10000].map((f) => `<span class="ft" style="left:${px(fx(f, w))}">${f >= 1000 ? f / 1000 + 'k' : f}</span>`).join('');
  return `<div class="eq mb" style="height:${px(h)}"><svg width="${w}" height="${h}" aria-hidden="true"><path class="sp" d="${L.spectrum(w, h, 21)}"/>${grs.map((g, i) => `<rect class="bd ${i === selBand ? 'is-sel' : ''}" x="${L.f1(xs[i])}" y="0" width="${L.f1(xs[i + 1] - xs[i])}" height="${h}"/><rect class="grb" x="${L.f1(xs[i] + 6)}" y="0" width="${L.f1(xs[i + 1] - xs[i] - 12)}" height="${g}" rx="2"/>`).join('')}<path class="xo" d="${xos.map((f) => `M${L.f1(fx(f, w))} 0V${h}`).join('')}"/></svg>
    ${xos.map((f, i) => `<i class="dot ${i === selBand || i === selBand - 1 ? 'is-sel' : ''}" style="left:${px(fx(f, w))};top:${px(h / 2)}"></i><span class="xl" style="left:${px(fx(f, w))}">${hz(f)}</span>`).join('')}${ticks}</div>`;
}
// One row per band, whatever their number.
const mbRow = (b, on) => `<div class="brow ${on ? 'on' : ''}">${tgl('by', true)}<span class="b ell bname">${b[0]}</span><span class="faint s1 ell brange">${b[1]}</span><div class="hbar gr grow"><i style="width:${b[3]}%"></i></div><span class="b s1 grv">${b[2]}</span>${tgl('so', false)}</div>`;
const stepper = (n) => `<div class="fx g1">${L.ib('minus', 'One band less', n <= 2 ? 'is-off' : '')}<span class="b s1 stepv">${n} bands</span>${L.ib('plus', 'One band more', n >= 6 ? 'is-off' : '')}</div>`;
const mbEd = (n, xos, bands, selBand, vals, hint, gh) => `<section class="card grow" aria-label="Multiband compressor">${ehead('Multiband', 3, 'two to six bands', stepper(n))}
  <div class="sect grow">${mbGraph(EW, gh, xos, bands.map((b) => b[3]), selBand)}
    <div class="col brows">${bands.map((b, i) => mbRow(b, i === selBand)).join('')}</div>
    <div class="knobs">${kb('Threshold', vals[0], 0.65)}${kb('Ratio', vals[1], 0.38)}${kb('Attack', vals[2], 0.3)}${kb('Release', vals[3], 0.45)}${kb('Gain', vals[4], 0.5, { bip: true })}</div>
    <span class="faint s1">${hint}</span></div></section>`;
const multiband = mbEd(3, [120, 4000], [['Low', 'below 120 Hz', '−2.1 dB', 21], ['Mid', '120 Hz – 4.0 kHz', '−0.8 dB', 8], ['High', 'above 4.0 kHz', '−1.4 dB', 14]], 0, ['−14.0', '2.5', '20', '160', '0.0'],
  'Low band selected: held steady on its own, it no longer pushes the limiter. Drag a line on the graph to move a crossover; S solos a band.', 140);
const multiband5 = mbEd(5, [70, 250, 2500, 8000], [['Sub', 'below 70 Hz', '−1.6 dB', 16], ['Low', '70 – 250 Hz', '−0.6 dB', 6], ['Mid', '250 Hz – 2.5 kHz', '0.0 dB', 0], ['Presence', '2.5 – 8.0 kHz', '−1.8 dB', 18], ['Air', 'above 8.0 kHz', '−0.9 dB', 9]], 3, ['−20.0', '2.0', '5', '80', '0.0'],
  'Presence selected: it works only when that range gets harsh in the loud sections. Double-click the graph to split a band there; drag a line onto its neighbour, or press −, to merge two.', 120);

// Sections: offsets on top of the song-wide settings, shown in a lane under the waveform and edited per section.
const OFFS = { 2: ['+0.5 dB', '+1.0 side'], 4: ['+0.5 dB', '+1.0 side'], 6: ['+0.8 dB', '+1.0 side', 'EQ'], 0: ['−1.0 dB'] };
const lane = (sel) => (pps) => `<div class="mlane" aria-label="Section offsets"><span class="faint s1 mlane-l">Offsets</span>${L.sections.map((s, i) => OFFS[i] ? `<span class="so ${i === sel ? 'is-sel' : ''}" style="left:${px(s[1] * pps)};width:${px((s[2] - s[1]) * pps - 1)}">${OFFS[i].map((t) => `<span class="ell">${t}</span>`).join('<i></i>')}</span>` : '').join('')}</div>`;
const offRow = (l, v, u, hint, cls = '') => `<div class="fx g3"><span class="b offl">${l}</span>${field(v, u, cls)}<span class="faint s1 grow">${hint}</span></div>`;
const sectionEd = `<section class="card grow" aria-label="Section offsets"><div class="ph">${ic('cut')}<span class="h">Chorus</span><span class="faint s1 grow ell">0:45 – 1:15</span>${seg(['This chorus', 'All 3'], 1, 'quiet sm', 'Scope')}</div>
  <div class="sect grow"><span class="faint s1">Offsets added to the song-wide master, only while this section plays.</span>
    ${offRow('Level', '+0.5', 'dB', 'A little harder into the limiter.', 'is-edit')}${offRow('Width', '+1.0', 'dB', 'More side than in the verses.')}${offRow('Limiter release', '±0', 'ms', 'As in the rest of the song.')}
    <span class="faint s1">EQ 2 in this section · dashed is the song-wide curve</span>
    ${P.eqGraph(EW, 110, [{ t: 'ls', f: 45, g: -1.5, q: 1 }, { t: 'pk', f: 300, g: -0.8, q: 1 }, { t: 'hs', f: 9000, g: 1.6, q: 1 }], { hpf: 0, range: 6, seed: 21, grid: true, selBand: 2, style: 'height:110px' })}
    <div class="fx g1">${[['Peak · 300 Hz', '—'], ['High shelf · 9 kHz', '+0.6 dB', 'on']].map((b) => `<div class="pill ${b[2] || ''}"><span class="faint s1">${b[0]}</span><span class="b">${b[1]}</span></div>`).join('')}</div>
    ${info('Offsets glide over a tenth of a second at the edges of the section. The loudness of the target is still measured on the whole song. Click away from the section to go back to the chain.', `<span class="grow"></span>${btn('Clear', 'quiet sm')}`)}</div></section>`;

// ---------- the assistant column ----------
const head = `<div class="ph"><span class="h">Assistant</span><div class="grow"></div>${seg(['Notes', 'Proposals', 'History'], 0, 'quiet sm')}</div>`;
const chips = `<div class="sect"><div class="fx g1 wrap">${tag('Custom · −9.6 LUFS · −1 dBTP')}${tag('Reference A')}${tag('Rock')}</div></div>`;
const askFoot = `<div class="foot"><div class="ask"><span class="grow">Tell the assistant what to change: “keep the attack of the loud hits”</span>${ic('send')}</div>
    <div class="fx between"><span class="faint s1">This request about $0.03 · $0.48 so far</span>${btn('Master with AI', 'primary', 'spark')}</div></div>`;
const NOTES = [[1, 'EQ 1', 'Low shelf −1.5 dB at 45 Hz: the sub was above the reference and cost limiter work without adding loudness.'], [2, 'Compressor', 'Fast attack, 60 % mix: 2.6 dB off the peaks, blended back so they keep their front.'],
  [3, 'Multiband', 'The low band is held on its own, so it no longer pushes the limiter.'], [4, 'Clipper', 'The peaks are few and isolated: 0.6 % of the song is clipped, up to 4.0 dB.'], [5, 'Limiter', 'Left with at most 1.9 dB. Over 3 dB for 2 % of the song, down from 21 %.']];
const notes = (on = 3) => `<aside class="card side" aria-label="Assistant">${head}${chips}
  <div class="sect"><div class="fx between"><span class="b">Master with AI · applied 21:55</span><button class="tag">${ic('ab', 'sm')}A/B</button></div>
    <p class="why dim">The mixdown has 17 dB between peaks and loudness; −9.6 LUFS leaves 8.6. The peaks come down in three stages before the limiter, so it no longer ducks the mix after each one.</p><span class="faint s1">Limiter over 3 dB 21 % → 2 % · 3 passes · 5 modules set</span></div>
  <div class="sect">${NOTES.map((n, i) => `<div class="note ${i === on ? 'on' : ''}">${badge(n[0])}<div class="col g1 grow"><span class="b">${n[1]}</span><span class="why dim">${n[2]}</span></div></div>`).join('')}</div>${askFoot}</aside>`;
const dRow = (l, a, b, good) => `<div class="dr"><span class="dim grow ell">${l}</span><span class="was">${a}</span><span class="faint">→</span><span class="to ${good ? 'okc' : ''}">${b}</span></div>`;
const pc = (name, change, why, on = true) => `<div class="pcard"><div class="fx g2">${check(on)}<span class="b grow">${name}</span></div><span class="s1">${change}</span><span class="dim s1">${why}</span></div>`;
const proposal = `<aside class="card side" aria-label="Master proposal">${head}
  <div class="sect"><div class="fx g2">${ic('spark', 'aic')}<span class="b grow">Master proposal</span><span class="faint s1">3 passes · $0.05</span></div><div class="tile">${seg(['Current master', 'Proposal'], 1, '', 'A/B')}<span class="faint s1">Both at −9.6 LUFS. <span class="kbd">B</span> switches</span></div>
    <p class="why dim">Same loudness, with the limiter doing a third of the work: the tallest peaks are shaved before it. The target is unchanged.</p></div>
  <div class="sect"><span class="faint s1">Measured on the whole song</span>
    ${dRow('Limiter over 3 dB', '21 % of the song', '2 %', 1)}${dRow('Limiter at most', '10.7 dB', '4.6 dB', 1)}${dRow('Peaks above body, short', '11.2 dB', '11.9 dB', 1)}${dRow('Loudness', '−9.6 LUFS', '−9.6 LUFS')}${dRow('Distance to reference', '1.0', '0.8', 1)}</div>
  <div class="sect pscroll"><div class="pgrid one">${pc('Order', 'Compressor before Multiband', 'So each band works on peaks already evened out.')}${pc('EQ 2 · new module', 'Added after Multiband: high shelf +1.0 dB at 9 kHz', 'Tone after the dynamics, so the compressors do not react to it.')}${pc('Compressor', 'Off → 3:1, 3 ms attack, 60 % mix', 'Takes 2.6 dB off the peaks and blends them back.')}${pc('Clipper', 'Off → takes off up to 4.0 dB', 'Few isolated peaks: clipping them is hard to hear.')}${pc('Sections', '3 choruses: +0.5 dB, +1.0 dB side', 'The choruses lift without a louder song.', false)}</div></div>
  <div class="foot"><div class="fx g2">${btn('Discard', 'quiet')}<span class="grow"></span>${btn('Apply selected (4)')}${btn('Apply all', 'primary')}</div></div></aside>`;

// ---------- too far for this mix ----------
const farBanner = `<div class="banner stack"><div class="fx g3">${ic('warn')}<div class="col grow"><span class="b">This mix is too peaky for −9.6 LUFS</span><span class="dim s1">Its peaks stand 17 dB above its loudness; this target leaves room for 8.6. The chain takes about 5 dB off cleanly; the rest dulls the transients whatever does it. Most of those peaks come from the Drums group.</span></div></div>
  <div class="fx g2"><span class="grow"></span>${btn('Keep it', 'quiet sm')}${btn('Lower the target to −11.5 LUFS', 'sm')}${btn('Tame those peaks in Mix…', 'sm primary')}</div></div>`;
const farNotes = `<aside class="card side" aria-label="Assistant">${head}${chips}
  <div class="sect"><div class="fx between"><span class="b">Master with AI · applied 21:55</span><button class="tag">${ic('ab', 'sm')}A/B</button></div>
    <p class="why dim">On target at −9.6 LUFS, but the limiter still works over 3 dB for 9 % of the song after the compressor and the clipper. More than that would be heard as distortion.</p><span class="faint s1">Limiter over 3 dB 21 % → 9 % · 3 passes</span></div>
  <div class="sect"><div class="note on colm"><div class="fx g2">${ic('warn', 'warnc')}<span class="b">What would fix it</span></div><p class="why dim">Lower the peaks of the Drums group by about 3 dB in the mix, then bounce again: the master keeps its settings. Or master to −11.5 LUFS, where the limiter stays under 3 dB.</p></div></div>${askFoot}</aside>`;
const FAR_CH = CH.map((m) => m[0] === 'Clipper' ? ['Clipper', 'Up to 3.0 dB', '', 0, 4, 3.0] : m[0] === 'Limiter' ? ['Limiter', '−1.0 dBTP', '', 1, 5, 5.0] : m);
const farClipper = clipperEd({ take: '3.0', kv: 0.3, cgr: '−3.0 dB', cp: 30, lgr: '−5.8 dB', lp: 58, line: 'The limiter still works over 3 dB for 9 % of the song.',
  info: 'Taking off more than 3.0 dB here would reach past the short transients into the body of the mix, and that is heard as distortion.' });

// ---------- reordering, adding and removing ----------
const dragRack = { sub: 'moving Compressor', dimFixed: true, hint: `${ic('lock', 'sm')} Input, Limiter and Dither cannot move, and nothing can pass them · Esc cancels`,
  rowAt: (m, i) => m[0] === 'Compressor' ? '' : (m[0] === 'EQ 1' ? '<div class="rslot" aria-hidden="true"></div>' : '') + rrow(m, i === 1 ? 2 : i, { cls: m[3] ? 'is-dim' : '' }),
  pop: `<div class="rrow is-drag" style="top:calc(var(--ctl-h) * 0.55)">${ic('dots', 'sm')}<span class="faint s1 rn">2</span><span class="b ell rname">Compressor</span><span class="dim s1 ell grow">3:1 · 60 %</span><span class="hbar gr rgr"><i style="width:52%"></i></span><i class="led"></i></div>` };
const rowMenu = menu(['#EQ 2', ['Move up', { k: '⌥↑' }], ['Move down', { k: '⌥↓' }], '-', ['Bypass', { k: '0' }], ['Reset this module', {}], ['Remove', { icon: 'trash', k: '⌫' }]], 'left:var(--sp-6);top:calc(var(--ctl-h) * 5 + var(--sp-4))');
const addPop = menu(['#Add before the limiter', ['Compressor', { icon: 'plus' }], ['Multiband', { icon: 'plus' }], ['Saturation', { icon: 'plus' }], ['Clipper', { icon: 'plus' }], '-', '#EQ: two already · Match EQ and Width: only one'], 'left:var(--sp-5);top:var(--sp-5)');

// ---------- exploration: three ways to show a chain of repeatable blocks (the rack was chosen) ----------
const flowBar = (text, extra = '') => `<div class="fx g2 flow"><span class="faint s1 grow">${text}</span>${extra}</div>`;
const xcard = (m, i) => `<button class="mod ${m[2]} ${m[0] === 'Clipper' ? 'is-open' : ''} ${m[3] ? 'is-fixed' : ''}"><span class="fx g1"><span class="faint s1">${i + 1}</span><span class="h s grow ell">${m[0]}</span>${m[3] ? ic('lock', 'sm faint') : ic('dots', 'sm faint')}<i class="led ${m[2] === 'is-off' ? 'off' : ''}"></i></span><span class="b ell">${m[1]}</span><span class="faint s1 ell">${m[6]}</span></button>`;
const addTile = `<button class="mod add">${ic('plus')}<span class="b">Add module</span><span class="faint s1">EQ, compressor, clipper…</span></button>`;
const xGrid = `<div class="col g1" aria-label="Master chain">${flowBar(`Signal flow, left to right. ${ic('dots', 'sm')} drag to reorder · ${ic('lock', 'sm')} stays in place · 11 of 12 modules`, btn('Reset order', 'quiet sm'))}
  <div class="mods ten">${CH.slice(0, FREE).map(xcard).join('')}${addTile}${CH.slice(FREE).map((m, i) => xcard(m, i + FREE)).join('')}</div></div>`;
const PH = 76;
const pcol = (m) => { const h = ((m[7] - 6) / 12) * PH; return `<button class="pcol ${m[2]} ${m[0] === 'Clipper' ? 'on' : ''} ${m[3] ? 'is-fixed' : ''}"><span class="pd ${m[5] ? '' : 'none'}">${m[5] ? '−' + m[5].toFixed(1) : '·'}</span><span class="pg" style="height:${px(PH)}"><i style="height:${px(h)}"></i></span><span class="b s1 ell pn">${m[0]}</span><span class="faint s1 ell pn">${m[1]}</span>${m[3] ? ic('lock', 'sm faint') : ic('dots', 'sm faint')}</button>`; };
const pplus = '<span class="pplus" aria-label="Add a module here">+</span>';
const xPath = `<section class="card" aria-label="Master chain"><div class="tools"><span class="b">Chain</span><span class="faint s1">Each bar is how far the peaks stand above the body after that module: 17.1 dB coming in, 8.6 dB going out. The number above is what the module takes off.</span><div class="grow"></div>${btn('Reset order', 'quiet sm')}</div>
  <div class="path">${CH.slice(0, FREE).map((m, i) => pcol(m) + (i < FREE - 1 ? pplus : '')).join('')}<button class="pcol add">${ic('plus')}<span class="b s1">Add module</span><span class="faint s1">11 of 12</span></button>${CH.slice(FREE).map(pcol).join('')}</div></section>`;

const base = { ...LOUD, clip: ['−4.0', 40], lim: ['−1.9', 19], work: ['2 % of the song'], asstHtml: notes(3) };
const prop = { rows: [['Proposal', '−9.6', 'as exported'], ['Mixdown', '−20.1', '10.5 LU quieter'], ['Reference A', '−9.6', 'same loudness']] };
// A board: the workspace with the rack (`r`) beside an editor.
const scr = (o, editor, r = {}) => master({ ...base, ...o, modsHtml: none, editorHtml: `<div class="rackrow">${rack(r)}${editor}</div>` });
// The rack with the clipper open (15a), for the tour (P-020).
export const masterChain = (o = {}) => scr(o, clipper, { open: 'Clipper' });
export default {
  'screens/15-master-chain.html': P.page('15 · Master chain', 'P-017. The chain is a rack beside the editor: a list of blocks, top to bottom. Input comes first, Limiter and Dither last; everything in between can be put in any order, and EQ, Compressor, Multiband, Saturation and Clipper can be there twice (twelve blocks at most). New: a clipper, parallel compression and a detector filter on the compressor, a multiband compressor with two to six bands, offsets per section, a proposal that shows what it measured, and a way back to the mix when the target asks too much. Replaces the module row of 9a–9c; the rest of screen 9 stands.', [
    P.frame('15a', 'The rack, clipper open', 'One row per block with its value, the gain reduction it applies and its switch; the number is the note of the assistant. A lock marks the three blocks that stay in place: the limiter must be last to guarantee the ceiling, and dither follows it. “Add module” is always in sight, where a new block lands. The editor of the selected block is beside the rack. The Loudness card gains the clipper meter and how long the limiter works over 3 dB.', scr({}, clipper, { open: 'Clipper' })),
    P.frame('15b', 'Reordering', 'Compressor dragged above EQ 1: a line shows where it lands, the three fixed blocks dim. One undo step. The same from the keyboard with ⌥↑ and ⌥↓ on the selected block.', scr({}, clipper, dragRack)),
    P.frame('15i', 'Adding a block', '“Add module” lists what can still be added and says why the rest cannot: two of a kind at most, one Match EQ, one Width. The new block lands before the limiter, off, and opens in the editor; it is one undo step. Hovering between two rows shows a “+” that adds there.', scr({}, clipper, { open: 'Clipper', addOpen: true, pop: addPop })),
    P.frame('15j', 'The menu of a block', 'Right-click on a row, or its grip: move, bypass, reset, remove. Removing is one undo step; the fixed blocks have no “Remove”.', scr({}, clipper, { open: 'EQ 2', pop: rowMenu })),
    P.frame('15c', 'Compressor with mix and detector filter', 'Mix under 100 % is parallel compression, as on tracks (7e). “Listens above” keeps the low end out of the detector.', scr({ asstHtml: notes(1) }, comp, { open: 'Compressor' })),
    P.frame('15d', 'Multiband', 'It opens with three bands over the live spectrum; one row per band with bypass, reduction and solo, and the controls of the selected band below. The crossovers are dragged on the graph. − and + change the number of bands, from two to six (15h).', scr({ asstHtml: notes(2) }, multiband, { open: 'Multiband' })),
    P.frame('15h', 'Multiband with five bands', 'The same block with more bands, as the author or the assistant may set it: here one band works alone on a range that gets harsh and the mids are left untouched.', scr({ asstHtml: notes(2) }, multiband5, { open: 'Multiband', list: CH.map((m) => m[0] === 'Multiband' ? ['Multiband', '5 bands', '', 0, 3, 0.5] : m) })),
    P.frame('15e', 'Section offsets', 'A lane under the waveform shows which sections have offsets. A section is selected: the editor shows its offsets instead of a block, and the rack has no selection. “All 3” writes the same offsets to the sections of the same name.', scr({ selSec: 2, lane: lane(2) }, sectionEd, { hint: 'The offsets act on Input level, the EQs, Width and Limiter release.' })),
    P.frame('15f', 'Proposal with what it measured', 'Every plan is rendered before it is shown: the proposal says how the limiter, the peaks and the loudness change, then lists what it sets, each with its tick: a new order, a block it adds, the blocks it changes and the section offsets. The rack shows the proposed chain.', scr({ asstHtml: proposal, listen: prop }, clipper, { open: 'Clipper', noNotes: true })),
    P.frame('15g', 'The target asks too much of this mix', 'After the chain has done what it can cleanly, the limiter still works hard. The master says so, names the family of tracks the peaks come from (measured on the stems) and offers the three ways out. “Tame those peaks in Mix…” goes to Mix with the request already written in the assistant; nothing is sent until the author confirms.', scr({ banner: farBanner, asstHtml: farNotes, clip: ['−3.0', 30], lim: ['−5.8', 58], work: ['9 % of the song', 'warnc'] }, farClipper, { open: 'Clipper', list: FAR_CH })),
  ]),
  'screens/15-master-chain-directions.html': P.page('15 · Master chain, three ways to show it', 'The exploration that led to the rack (A-080, A-081); kept as a record. With repeatable modules the chain is a list that grows: up to twelve blocks, two of a kind at most. The same master in three layouts, each with a visible way to add a module. The author chose R.', [
    P.frame('G', 'Grid', 'Cards in rows with an “Add module” tile after the last free module. Familiar and wordy: every card has room for a value and a detail. The editor loses a row of height, and the order reads in two directions (along a row, then down).', master({ ...base, modsHtml: xGrid, editorHtml: clipper })),
    P.frame('R', 'Rack (chosen)', 'The chain is a column beside the editor, top to bottom, as the inserts of a channel. One row per module with its value, its reduction and its switch; “Add module” is a row of its own where the new block would land. Twelve rows fit without scrolling and the order reads in one direction. The editor is narrower and taller.', scr({}, clipper, { open: 'Clipper' })),
    P.frame('P', 'Path', 'One column per module, left to right, with a bar that says how far the peaks stand above the body after it: the chain shows at a glance which module does the work and what is left to the limiter. A “+” sits between every two modules and an “Add module” column before the limiter. Names and values are tight at twelve modules.', master({ ...base, modsHtml: xPath, editorHtml: clipper })),
  ]),
};
