// Screens 9, 10 — Master workspace, Export.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
import * as V from './level.mjs';
const { ic, tag, btn, badge, seg, field, flabel, input, sel, check, tgl, px, ib } = L;

const G = P.geo({}), W = G.left - 24; // inner width of a card in the left column
const MODS = [['Input', '0.0 dB', 'Gain into the chain'], ['EQ', '4 of 6 bands', '+1.0 dB at 10 kHz', 'is-open', 1], ['Match EQ', '50 %', 'Toward Reference A', '', 2], ['Compressor', '1.8:1', '−1.2 dB reduction', '', 3],
  ['Width', '+0.5 dB side', 'Mono below 120 Hz'], ['Saturation', 'Off', 'Bypassed', 'is-off'], ['Limiter', '−1.0 dBTP', '−2.4 dB reduction', '', 4], ['Dither', '16-bit only', 'TPDF at export', 'is-off']];
const MNOTES = [[1, 'EQ', 'Slightly dark against Reference A above 8 kHz: a gentle high shelf, and 1 dB less at 250 Hz.'], [2, 'Match EQ', 'Half-way toward the reference profile; full match would thin the low mids.'],
  [3, 'Compressor', 'Only 1.2 dB of glue, slow attack, so the choruses keep their lift.'], [4, 'Limiter', '4.5 dB of gain to reach −14 LUFS with the ceiling at −1 dBTP.']];

// Notes of the applied master plan, following the target (and without the match EQ when there are no references).
const mnotes = (o) => {
  const t = parseFloat((o.tgtLufs || '−14.0').replace('−', '-')), g = (t + 18.6).toFixed(1);
  return MNOTES.filter((n) => !(o.noRefs && n[0] === 2)).map((n) => n[0] === 1 && o.noRefs ? [1, 'EQ', 'Slightly dark for indie rock above 8 kHz: a gentle high shelf, and 1 dB less at 250 Hz.']
    : n[0] === 4 && o.tgtLufs ? [4, 'Limiter', `${g} dB of gain to reach ${o.tgtLufs.replace('.0', '')} LUFS with the ceiling at −1 dBTP.`] : n);
};
function waveCard(o = {}) {
  const TT = o.T || L.T, SECS = o.sections || L.sections, pps = (G.left) / TT, m = L.wave(99, 0, TT, pps, 104, 2.5, SECS), d = L.wave(98, 0, TT, pps, 60, 2.5, SECS);
  let ticks = ''; for (let t = 0; t < TT; t += o.tick || 15) ticks += `<span class="tick" style="left:${px(t * pps)}">${L.fmt(t)}</span>`;
  return `<section class="card" aria-label="Mixdown"><div class="tools"><span class="b">Mixdown 3</span><span class="faint s1">3:42 · 48 kHz · measured ${o.mixText || '−18.6 LUFS, −3.0 dBTP'}</span><div class="grow"></div>${tag('Bright: master · grey: mixdown')}${btn('Bounce again', 'quiet sm', 'loop')}</div>
    <div class="mwave">${ticks}<div class="loop" style="left:${px(L.loop[0] * pps)};width:${px((L.loop[1] - L.loop[0]) * pps)};bottom:auto;height:var(--ruler-h)"></div>
      <svg width="${G.left}" height="104" aria-hidden="true"><path d="${m.d}"/></svg><svg class="dimw" width="${G.left}" height="60" style="top:calc(var(--ruler-h) + 22px)" aria-hidden="true"><path d="${d.d}"/></svg>
      ${SECS.map((s, i) => `<span class="sec ${s[0] === 'Chorus' ? 'hot' : ''}${o.selSec === i ? ' is-sel' : ''}" style="left:${px(s[1] * pps)};width:${px((s[2] - s[1]) * pps - 1)}">${s[0]}</span>`).join('')}
      <div class="play" style="left:${px((o.playAt ?? L.playhead) * pps)}"></div></div>${o.lane ? o.lane(pps) : ''}</section>`;
}
const NOREF_MEQ = ['Match EQ', 'Off', 'No reference to match', 'is-off'];
const mods = (o = {}) => o.modsHtml || `<div class="mods" aria-label="Master chain">${MODS.map((m0, i) => { const m = o.noRefs && i === 2 ? NOREF_MEQ : m0; return `<button class="mod ${m[3] || ''}"><span class="fx g2"><span class="faint s1">${i + 1}</span><span class="h s grow ell">${m[0]}</span><i class="led ${m[3] === 'is-off' ? 'off' : ''}"></i></span><span class="b ell">${m[1]}</span><span class="faint s1 ell">${m[2]}</span>${m[4] && !o.noNotes ? badge(m[4]) : ''}</button>`; }).join('')}</div>`;
const editor = (o = {}) => o.editorHtml || `<section class="card grow" aria-label="EQ"><div class="ph">${tgl('by', true)}<span class="h">EQ</span>${badge(1)}<span class="faint s1">6 bands · ${o.noRefs ? '' : 'dashed line is the profile of Reference A · '}grey is the live spectrum</span><div class="grow"></div>${seg(['±6 dB', '±12 dB'], 0, 'quiet sm', 'Range')}</div>
    <div class="sect grow">${P.eqGraph(W, 220, L.masterEq, { hpf: 0, range: 6, ref: !o.noRefs, seed: 21, grid: true, selBand: 3, style: 'height:220px' })}
      <div class="fx g1">${[['Low shelf', '60 Hz', '+0.8 dB'], ['Peak', '250 Hz', '−1.0 dB'], ['Peak', '3.0 kHz', '+0.6 dB'], ['High shelf', '10 kHz', '+1.0 dB', 'on'], ['Band 5', 'Off', '', 'is-off'], ['Band 6', 'Off', '', 'is-off']].map((b) => `<div class="pill ${b[3] || ''}"><span class="faint s1">${b[0]} · ${b[1]}</span><span class="b">${b[2] || '—'}</span></div>`).join('')}</div>
      <div class="fx g3 end-al">${flabel('Type', sel('High shelf'))}${flabel('Frequency', field('10.0', 'kHz'))}${flabel('Gain', field('+1.0', 'dB', 'is-edit'))}${flabel('Q', field('0.70', ''))}<div class="grow"></div><span class="faint s1">Drag a handle, scroll to change Q, double-click to reset. Hold ⇧ for fine steps.</span></div></div></section>`;
const lm = (l, v, p, tgt) => `<div class="lm"><span class="b s1">${v}</span><div class="meter l"><i class="pk" style="height:${p + 6}%"></i><i class="rms" style="height:${p}%"></i>${tgt ? `<i class="tgt" style="top:${100 - tgt}%"></i>` : ''}</div><span class="faint s1">${l}</span></div>`;
// Listening section of the Loudness card (boards 9l–9t): every source of the A/B with its loudness and, in words, its
// offset — against the master at the real level, the gain it plays with at the same loudness. The row being heard is
// marked; `o.listen = { same, rows: [[name, lufs, offset, cls]], note }`.
export const ROWS_REAL = [['Master', '−14.1', 'as exported'], ['Mixdown', '−18.6', '4.5 LU quieter'], ['Reference A', '−12.9', '1.2 LU louder']];
export const lsect = (o) => {
  const l = o.listen || {}, rows = l.rows || ROWS_REAL, at = o.ab ?? 0;
  const head = l.same ? `<span class="fx g1 b s1 acc">${ic('level', 'sm')}Same loudness</span>` : `<span class="b s1">Real level</span>`;
  return `<div class="sect"><div class="fx between"><span class="fx g2"><span class="faint s1">Listening</span>${head}</span><span class="faint s1"><span class="kbd">⇧B</span> ${l.same ? 'real level' : 'same loudness'}</span></div>
    <div class="lrows">${rows.map((r, i) => `<div class="lrow ${i === at ? 'on' : ''}"><span class="fx g1 ell">${i === at ? ic('speaker', 'sm') : '<i class="sp-ic"></i>'}<span class="ell">${r[0]}</span></span><span class="r">${r[1]} LUFS</span><span class="r ${r[3] || ''}">${r[3] === 'warnc' ? ic('warn', 'sm') + ' ' : ''}${r[2]}</span></div>`).join('')}</div>
    ${l.note ? `<span class="faint s1">${l.note}</span>` : ''}</div>`;
};
// Target select: `o.target` (label), `o.tgtWarn` (amber icon and border), `o.lwarn` (the warning or the kept note under it).
const tsel = (o) => `<span class="select lg ${o.tgtWarn ? 'warn' : ''} ${o.tgtOpen ? 'is-open' : ''}">${o.tgtWarn ? ic('warn', 'm warnc') : ''}<span class="ell grow">${o.target || 'Streaming · −14 LUFS · −1 dBTP'}</span>${ic('chev', 'm')}</span>`;
const loud = (o = {}) => `<aside class="card side" aria-label="Loudness"><div class="ph"><span class="h">Loudness</span><div class="grow"></div>${o.measuring ? tag(ic('clock', 'sm') + 'Measuring…', 'acc') : tag(ic('check', 'sm') + 'On target', 'ok')}</div>
    <div class="sect">${flabel('Target', tsel(o))}${o.lwarn || ''}${o.measuring ? `${L.progress(25, 'indet')}<span class="dim s1">Your change is already playing. The level match to −14 LUFS follows when the measure is done.</span>` : ''}<div class="fx g5${o.measuring ? ' is-stale' : ''}"><div class="col"><span class="faint s1">Integrated</span><span class="big">${o.integ || '−14.1'}</span><span class="faint s1">LUFS · ${o.measuring ? 'before the change' : 'target ' + (o.tgtLufs || '−14.0')}</span></div><div class="col"><span class="faint s1">True peak</span><span class="big">−1.0</span><span class="faint s1">dBTP · ceiling −1.0</span></div></div></div>${lsect(o)}
    <div class="sect grow"><div class="lmeters">${lm('Moment.', '−13.2', 78, 72)}${lm('Short', '−13.8', 74, 72)}${lm('Integr.', '−14.1', 72, 72)}${lm(o.clip ? 'Pk L' : 'Peak L', '−1.2', 92)}${lm(o.clip ? 'Pk R' : 'Peak R', '−1.0', 94)}${o.clip ? `<div class="lm"><span class="b s1">${o.clip[0]}</span><div class="meter l gr"><i style="height:${o.clip[1]}%"></i></div><span class="faint s1">Clipper</span></div>` : ''}<div class="lm"><span class="b s1">${o.lim ? o.lim[0] : '−2.4'}</span><div class="meter l gr"><i style="height:${o.lim ? o.lim[1] : 22}%"></i></div><span class="faint s1">Limiter</span></div></div></div>
    <div class="sect">${o.work ? `<div class="fx between"><span class="faint s1">Limiter over 3 dB</span><span class="b ${o.work[1] || ''}">${o.work[0]}</span></div>` : ''}<div class="fx between"><span class="faint s1">Loudness range</span><span class="b">6.2 LU</span></div><div class="fx between"><span class="faint s1">Stereo correlation</span><span class="b">+0.62</span></div><div class="corr"><i style="left:81%"></i></div></div>
    <div class="sect"><span class="faint s1">${o.noRefs ? 'Spectrum' : 'Spectrum against Reference A (dashed)'}</span><div class="eq" style="height:var(--sp-8)"><svg width="${G.side - 24}" height="64" aria-hidden="true"><path class="sp" d="${L.spectrum(G.side - 24, 64, 21, 0.5)}"/>${o.noRefs ? '' : `<path class="rf" d="${L.spectrum(G.side - 24, 64, 21, 0.42, true)}"/>`}</svg></div></div></aside>`;
function masterAsst(o = {}) {
  if (o.asstHtml) return o.asstHtml;
  const head = `<div class="ph"><span class="h">Assistant</span><div class="grow"></div>${seg(['Notes', 'History', 'Snapshots'], 0, 'quiet sm')}</div>`;
  if (o.proposal) return `<aside class="card side" aria-label="Master proposal">${head}
    <div class="sect"><div class="fx g2">${ic('spark', 'aic')}<span class="b grow">Master proposal</span><span class="faint s1">$0.02</span></div><div class="tile">${seg(['Current master', 'Proposal'], 1, '', 'A/B')}${o.listen?.same
      ? `<span class="fx g1 s1 acc">${ic('level', 'sm')}<span class="b">Same loudness</span></span><span class="faint s1">The proposal plays 0.1 dB lower to match the current master. <span class="kbd">B</span> switches · <span class="kbd">⇧B</span> real level</span>`
      : `<span class="b s1">Real level</span><span class="faint s1">The proposal is 0.1 LU louder: −14.0 against −14.1 LUFS. <span class="kbd">B</span> switches · <span class="kbd">⇧B</span> same loudness</span>`}</div>
      <p class="why dim">A touch less bright and a little more glue, as you asked. Loudness and ceiling stay on the Streaming target.</p><span class="faint s1">Distance to reference 1.2 → 1.0 · −14.0 LUFS · −1.0 dBTP guaranteed</span></div>
    <div class="sect">${[['EQ', 'High shelf +1.0 → +0.4 dB', 'Less air: the request was “a little less bright”.', 1], ['Compressor', 'Ratio 1.8 → 2.0 · threshold −14 → −15 dB', 'Slightly more glue without flattening the choruses.', 1], ['Limiter', 'Release 80 → 120 ms', 'Smoother on the loudest hits.', 1], ['Match EQ', 'Unchanged · 50 %', 'Kept as it was.', 0]].map((c, i) => `<div class="pcard ${c[3] ? '' : 'is-lock'}"><div class="fx g2">${c[3] ? check(i !== 2) : ''}<span class="b grow">${c[0]}</span></div><span class="s1">${c[1]}</span><span class="dim s1">${c[2]}</span></div>`).join('')}</div>
    <div class="foot"><div class="fx g2">${btn('Discard', 'quiet')}<span class="grow"></span>${btn('Apply selected (2)')}${btn('Apply all', 'primary')}</div></div></aside>`;
  return `<aside class="card side" aria-label="Assistant">${head}
    <div class="sect"><div class="fx g1 wrap">${tag(o.target || 'Streaming · −14 LUFS · −1 dBTP')}${o.noRefs ? '' : tag(o.refs || 'Reference A')}${tag('Indie rock')}</div></div>
    <div class="sect"><div class="fx between"><span class="b">Master with AI · applied 21:55</span><button class="tag">${ic('ab', 'sm')}A/B</button></div>${o.noRefs ? `<p class="why dim">Mixdown measured −18.6 LUFS and slightly dark for indie rock. A gentle shelf opens the top; the limiter brings it to target.</p><span class="faint s1">3 modules set</span>` : `<p class="why dim">Mixdown measured −18.6 LUFS and slightly dark against the reference. A gentle shelf and half match close the gap; the limiter brings it to target.</p><span class="faint s1">Distance to reference 3.4 → 1.2 · 4 modules set</span>`}</div>
    <div class="sect">${mnotes(o).map((n, i) => `<div class="note ${i === 0 ? 'on' : ''}">${badge(n[0])}<div class="col g1 grow"><span class="b">${n[1]}</span><span class="why dim">${n[2]}</span></div></div>`).join('')}</div>
    <div class="foot"><div class="ask"><span class="grow">Tell the assistant what to change: “a little less bright, keep the punch”</span>${ic('send')}</div>
      <div class="fx between"><span class="faint s1">This request about $0.01 · $0.43 so far</span>${btn('Master with AI', 'primary', 'spark')}</div><div class="fx between"><span class="faint s1">Only measurements of the mixdown are sent.</span><a href="12-ai-usage.html" class="s1">AI usage</a></div></div></aside>`;
}
export const master = (o = {}) => P.app({ phase: 'master' }, `${P.top(Object.assign({ phase: 'master', noSection: true }, o))}<div class="body"><div class="left">${o.banner || ''}${waveCard(o)}${mods(o)}${editor(o)}</div>${loud(o)}${masterAsst(o)}</div>${o.over || ''}`);
export const stale = `<div class="banner">${ic('warn')}<div class="col grow"><span class="b">The mix has changed since this mixdown</span><span class="dim s1">3 steps in Mix after Mixdown 3. You are still mastering the older version.</span></div>${btn('Bounce again and keep these settings', 'sm')}${btn('Not now', 'quiet sm')}</div>`;

// ---------- 10 · export ----------
const fmtRow = (on, name, sub, opts) => `<div class="tile row ${on ? 'on' : ''}">${check(on)}<div class="col grow"><span class="b">${name}</span><span class="faint s1">${sub}</span></div>${opts}</div>`;
export const exportForm = (o = {}) => L.dialog({ size: 'l', title: 'Export master', sub: o.sub || 'Streaming target · −14 LUFS · −1 dBTP. One export can write several formats.',
  body: `${o.warn || ''}<div class="fx g5 top-al"><div class="col g2 grow"><span class="b">Formats</span>
      ${fmtRow(true, 'WAV', 'Uncompressed', sel('24-bit') + sel('48 kHz'))}${fmtRow(true, 'FLAC', 'Lossless, smaller', sel('16-bit') + sel('44.1 kHz'))}${fmtRow(true, 'AAC', 'M4A, for sharing', sel('256 kbps') + sel('44.1 kHz'))}
      <div class="tile row">${ic('info')}<span class="dim s1">Dither (TPDF) is added to the 16-bit FLAC only. Files at 44.1 kHz are converted from 48 kHz at high quality. The true peak of the AAC is checked on the decoded file.</span></div>
      ${flabel('Folder', `<div class="fx g2">${input('Music › Stemwise exports › Paper Lanterns', 'sm grow')}${btn('Choose…')}</div>`)}</div>
    <div class="col g2" style="width:var(--field-w-l)"><span class="b">Metadata</span><div class="fx g3"><div class="cover" role="img" aria-label="Cover art"></div><div class="col g1">${btn('Replace cover…', 'sm', 'image')}<span class="faint s1">JPEG or PNG, square</span></div></div>
      ${flabel('Title', input('Paper Lanterns', 'sm'))}${flabel('Artist', input('Piero De Tomi', 'sm'))}${flabel('Album', input('Slow Furniture', 'sm'))}
      <div class="fx g2">${flabel('Year', field('2026', '', 'fill'), 'grow')}${flabel('Track', field('3', '', 'fill'), 'grow')}</div>${flabel('Genre', input('Indie rock', 'sm'))}${flabel('ISRC', input('IT-A12-26-00003', 'sm'))}<span class="faint s1">${ic('check', 'sm')} Valid ISRC format</span></div></div>`,
  foot: `<span class="faint s1 grow">3 files · about 12 seconds</span>${btn('Cancel', 'quiet')}${btn('Export 3 files', 'primary', 'exp')}` });
const exportProgress = L.dialog({ title: 'Exporting', sub: 'Rendering the master once, then writing each format.',
  body: [['WAV · 24-bit · 48 kHz', 100, 'Written', 'ok'], ['FLAC · 16-bit · 44.1 kHz', 64, 'Converting rate, adding dither', ''], ['AAC · 256 kbps · 44.1 kHz', 0, 'Waiting', '']].map((f) => `<div class="col g1"><div class="fx between"><span class="b">${f[0]}</span><span class="s1 ${f[3] === 'ok' ? 'okc' : 'faint'}">${f[3] === 'ok' ? ic('check', 'sm') + ' ' : ''}${f[2]}</span></div>${L.progress(f[1])}</div>`).join(''),
  foot: btn('Cancel', '') });
export const exportReport = (o = {}) => L.dialog({ size: 'l', title: 'Export complete', sub: '3 files written to Music › Stemwise exports › Paper Lanterns.',
  body: `<div class="mtable" style="grid-template-columns:2fr 1fr 1fr 1fr 1fr 1.4fr"><span class="th">File</span><span class="th r">Loudness</span><span class="th r">True peak</span><span class="th r">Range</span><span class="th r">To reference</span><span class="th">Notes</span>
      ${(o.rows || [['Paper Lanterns.wav', '−14.1 LUFS', '−1.0 dBTP', '6.2 LU', '1.2', 'On target'], ['Paper Lanterns.flac', '−14.1 LUFS', '−1.0 dBTP', '6.2 LU', '1.2', 'Dither added'], ['Paper Lanterns.m4a', '−14.4 LUFS', '−1.0 dBTP', '6.2 LU', '1.3', 'Re-encoded 0.3 dB lower']]).map((r) => `<span class="b ell">${r[0]}</span><span class="r">${r[1]}</span><span class="r">${r[2]}</span><span class="r">${r[3]}</span><span class="r">${r[4]}</span><span class="dim ell">${r[5]}</span>`).join('')}</div>
    ${o.note || ''}${o.aacNote === false ? '' : `<div class="tile row">${ic('info')}<span class="dim s1">The first AAC encode peaked at −0.7 dBTP after decoding, above the −1 dBTP ceiling, so it was encoded again 0.3 dB lower. Metadata and cover were written to all three files.</span></div>`}`,
  foot: `${btn('Copy report', 'quiet', 'copy')}<span class="grow"></span>${btn('Show in Finder')}${btn('Done', 'primary')}` });

// Boards 10d–10e: Custom −18 LUFS against Reference A at −9.6 LUFS (the case of 9q).
const FAR = { target: 'Custom · −18 LUFS · −1 dBTP', integ: '−18.0', tgtLufs: '−18.0', tgtWarn: true, lwarn: V.quieter1, listen: { rows: [['Master', '−18.0', 'as exported'], ['Mixdown', '−18.6', '0.6 LU quieter'], ['Reference A', '−9.6', '8.4 LU louder', 'warnc']] } };
const FAR_ROWS = [['Paper Lanterns.wav', '−18.0 LUFS', '−1.0 dBTP', '6.8 LU', '1.4', 'On target'], ['Paper Lanterns.flac', '−18.0 LUFS', '−1.0 dBTP', '6.8 LU', '1.4', 'Dither added'], ['Paper Lanterns.m4a', '−18.0 LUFS', '−1.2 dBTP', '6.8 LU', '1.4', 'On target']];

export default {
  'screens/09-master-workspace.html': P.page('9 · Master workspace', 'One waveform, a chain of eight modules in fixed order, loudness and spectrum against the reference, and the assistant for the master. Lilac accent and a cooler ground tell it apart from Mix at a glance.', [
    P.frame('9a', 'Master workspace', 'EQ module open in the editor. Modules set by the assistant carry the number of their note. The listening level (real by default) and the A/B source live in the top bar; the target and the Listening offsets in the Loudness card.', master()),
    P.frame('9b', 'Mixdown out of date', 'The mix changed after the bounce: a banner offers to bounce again keeping the master settings. Nothing is blocked.', master({ banner: stale, status: 'Mixdown 3 · out of date' })),
    P.frame('9c', 'Master proposal in review', 'A request in words produced a proposal: per-module changes with their reason, A/B with the current master (at the real level here, at the same loudness in 9o), apply all, apply selected, or discard.', master({ proposal: true, noNotes: true, listen: { rows: [['Proposal', '−14.0', 'as exported'], ['Mixdown', '−18.6', '4.6 LU quieter'], ['Reference A', '−12.9', '1.1 LU louder']] } })),
  ]),
  'screens/10-export.html': P.page('10 · Export', 'Formats (also several at once), rate, bit depth, dither, metadata with ISRC and cover, folder; then progress and the final report.', [
    P.frame('10a', 'Export settings', 'Three formats ticked. Dither applies only where a 16-bit file is written.', master({ playing: false, over: exportForm() })),
    P.frame('10b', 'Progress', 'Per-file progress with the current step in words.', master({ playing: false, over: exportProgress })),
    P.frame('10c', 'Report', 'Measures of every written file, distance to the reference, and what the export had to adjust.', master({ playing: false, over: exportReport() })),
    P.frame('10d', 'Export with the target warning', 'Same warning as the Loudness card (9q), above the formats. Export stays available. “Match the reference” changes the target here (one undo step) and the primary waits for the new measure, as in 9k.', master({ playing: false, ...FAR, over: exportForm({ sub: 'Custom target · −18 LUFS · −1 dBTP. One export can write several formats.', warn: V.exportWarn }) })),
    P.frame('10e', 'Report after exporting with the gap', 'The files were written 8.4 LU below the reference without “Keep”: the report says it in amber with the way back. After “Keep” the same line is neutral.', master({ playing: false, ...FAR, over: exportReport({ rows: FAR_ROWS, note: V.reportWarn, aacNote: false }) })),
  ]),
};
