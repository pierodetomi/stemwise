// Screen 16 — Track chain (P-018): the chain of tracks, groups and returns as a list of free blocks in the inspector,
// the menu that adds them by family, an editor for every new block, the chain of a return, the new modes of reverb and
// delay, filters that open between sections, and the marks and Overview with long chains.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
const { ic, tag, btn, seg, field, flabel, tgl, px, kb, f1 } = L;

const W = 292; // inner width of the inspector (--col-side less its padding)
// Every block type: name, label, family, what it does in plain words (the menu says it).
const CAT = {
  hpf: ['High-pass', 'HP', 'Tone', 'Removes the lows below a frequency'], lpf: ['Low-pass', 'LP', 'Tone', 'Removes the highs above a frequency'],
  eq: ['EQ', 'EQ', 'Tone', 'Four bands on the spectrum'], comp: ['Compressor', 'CMP', 'Dynamics', 'Evens out the level'], sat: ['Saturation', 'SAT', 'Tone', 'Warmth and grit, four characters'],
  gate: ['Gate', 'GT', 'Dynamics', 'Silences what sits between the hits'], ds: ['De-esser', 'DS', 'Dynamics', 'Tames harsh s and t sounds'],
  deq: ['Dynamic EQ', 'DEQ', 'Dynamics', 'A band that moves only when needed'], tr: ['Transient', 'TR', 'Dynamics', 'More or less attack, no threshold'],
  clip: ['Clipper', 'CLP', 'Dynamics', 'Shaves the tallest peaks'], mb: ['Multiband', 'MB', 'Dynamics', 'A compressor per range'],
  exc: ['Exciter', 'EXC', 'Tone', 'Adds sparkle to the top'], bass: ['Bass enhancer', 'BAS', 'Tone', 'Bass you hear on small speakers'],
  wid: ['Width', 'WID', 'Space', 'Narrower or wider stereo'], mod: ['Modulation', 'MOD', 'Movement', 'Chorus, flanger, phaser'],
  trm: ['Tremolo', 'TRM', 'Movement', 'Level or pan that moves in time'], hrm: ['Harmonizer', 'HRM', 'Pitch', 'Voices at a fixed interval'],
  dbl: ['Doubler', 'DBL', 'Pitch', 'A second performance, inside the track'],
};
const FAM = ['Dynamics', 'Tone', 'Space', 'Movement', 'Pitch'];

// ---------- the rack in the inspector ----------
// A block: [type, value, state ('' working · 'nu' on but neutral · 'off' bypassed), reduction 0–100, extra html]
const sgOf = (k, st, cls = '') => L.sigx(CAT[k][1], (st === 'off' ? '' : st === 'nu' ? 'nu' : 'on') + ' ' + cls, CAT[k][0]);
const trow = (b, o = {}) => { const [k, val, st, gr, extra] = b;
  return `<div class="rrow tr ${st === 'off' ? 'is-off' : ''} ${o.open === k ? 'on' : ''} ${o.cls || ''}">${ic('dots', 'sm faint')}${sgOf(k, st, b[5] || '')}<span class="b ell rname">${b[6] || CAT[k][0]}</span><span class="dim s1 ell grow">${val}</span>${extra || ''}${gr ? `<span class="hbar gr rgr"><i style="width:${gr}%"></i></span>` : ''}<i class="led ${st === 'off' ? 'off' : ''}"></i></div>`; };
const fixedRow = (name, val) => `<div class="rrow tr is-fixed">${ic('lock', 'sm faint')}<span class="b ell rname">${name}</span><span class="dim s1 ell grow">${val}</span></div>`;
const addRow = (o = {}) => `<button class="radd tr ${o.addOpen ? 'on' : ''}">${ic('plus', 'm')}Add block<span class="faint s1 ml-auto">${o.count || ''}</span></button>`;
const rack = (list, o = {}) => `<div class="sect tch"><div class="fx g2"><span class="b grow">${o.title || 'Chain'}</span><span class="faint s1">${o.sub || 'top to bottom'}</span></div>
  <div class="col rrows">${o.first === false ? '' : fixedRow(...(o.first || ['Trim', o.trim || '−2.4 dB']))}${o.rowsHtml || list.map((b) => trow(b, o)).join('')}${addRow({ ...o, count: o.count || `${list.length} of ${o.max || 12}` })}${o.last === false ? '' : fixedRow(...(o.last || ['Fader · Pan', '−5.8 dB · C']))}${o.pop || ''}</div></div>`;
const sendRows = (rows) => `<div class="sect tsnd"><div class="fx g2 s1"><span class="b">Sends</span>${rows.map((r) => `<span class="fx g1 grow"><span class="dim ell">${r[0]}</span><div class="hbar grow"><i style="width:${r[2]}%"></i></div><span class="b">${r[1]}</span></span>`).join('')}</div></div>`;
function insp(o) {
  return `<aside class="card side t${o.tint} tinsp" aria-label="Inspector"><div class="ph"><i class="tdot"></i><span class="h ell">${o.name}</span>${tag(o.role)}<div class="grow"></div><button class="route" aria-label="Output">${ic('right', 'sm')}${o.out || 'Mix'}${ic('chev', 'sm')}</button></div>
    ${o.top || ''}${rack(o.list, o)}${o.editor || ''}${o.sends === false ? '' : sendRows(o.sends || [['Plate', '−14.0', 62], ['Echo', '−22.0', 38]])}</aside>`;
}

// ---------- editors ----------
const ehead = (k, sub, right = '') => `<div class="fx g2">${tgl('by', true)}${sgOf(k, '')}<span class="b">${CAT[k][0]}</span><span class="faint s1 grow ell">${sub}</span>${right}</div>`;
const ed = (k, sub, body, right = '') => `<div class="sect ted">${ehead(k, sub, right)}${body}</div>`;
const info = (t) => `<div class="tile row tinfo">${ic('info')}<span class="dim s1">${t}</span></div>`;
const grRow = (l, v, p) => `<div class="fx g2"><span class="faint s1 grl">${l}</span><div class="hbar gr grow"><i style="width:${p}%"></i></div><span class="b s1 grv">${v}</span></div>`;
const key = (t, n) => `<button class="sc on t${t}">${ic('link', 'sm')}Listens to ${n}</button>`;
const nokey = `<button class="sc">${ic('link', 'sm')}Sidechain · none</button>`;
const knobs = (...k) => `<div class="knobs">${k.join('')}</div>`;
const fx = (f, w = W) => (Math.log(f / 20) / Math.log(1000)) * w;
const ticks = (w = W) => [100, 1000, 10000].map((f) => `<span class="ft" style="left:${px(fx(f, w))}">${f >= 1000 ? f / 1000 + 'k' : f}</span>`).join('');
const well = (h, svg, extra = '', cls = '') => `<div class="eq fxg ${cls}" style="height:${px(h)}"><svg width="${W}" height="${h}" aria-hidden="true">${svg}</svg>${extra}</div>`;
const legend = (items) => `<div class="fx g3 s1 faint lgd">${items.map((i) => `<span class="fx g1"><i class="lg ${i[0]}"></i>${i[1]}</span>`).join('')}</div>`;

// De-esser on the lead vocal: the band above the frequency, how far it can go and where it is now.
const dsEd = () => { const h = 76, e = L.eqGeo(W, h, [{ f: 6500, g: -4.2, q: 1, t: 'hs' }], 0, 12), m = L.eqGeo(W, h, [{ f: 6500, g: -6, q: 1, t: 'hs' }], 0, 12), x = fx(6500);
  return ed('ds', 'split · above 6.5 kHz', `${well(h, `<rect class="bz" x="${f1(x)}" y="0" width="${f1(W - x)}" height="${h}"/><path class="sp" d="${L.spectrum(W, h, 7)}"/><path class="zr" d="${e.zero}"/><path class="rf" d="${m.line}"/><path class="ar" d="${e.fill}"/><path class="cv" d="${e.line}"/>`, `<i class="dot is-sel" style="left:${px(x)};top:${px(h / 2 + 13)}"></i>${ticks()}`)}
    ${legend([['cv', 'taking off now'], ['rf', 'as far as it goes']])}
    <div class="fx g2">${seg(['Split', 'Wide'], 0, 'quiet sm', 'Mode')}<span class="grow"></span>${btn('Listen to what it takes', 'quiet sm', 'ear')}</div>
    ${knobs(kb('Frequency', '6.5 kHz', 0.7), kb('Threshold', '−28 dB', 0.45), kb('At most', '6.0 dB', 0.5))}${grRow('Reduction', '−4.2 dB', 42)}
    ${info('Split lowers only what is above 6.5 kHz, and only while an s or a t is too loud. Wide lowers the whole voice.')}`);
};

// Gate on the kick: the hits, the bleed between them, where the gate closes.
const gateCurve = (thr = 62, rng = 30) => `<svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true"><path class="un" d="M0 100L100 0"/><path class="th" d="M${thr} 0V100"/><path class="cv" d="M0 100L${thr - 6} 100L${thr} ${100 - thr + rng * 0.2}L${thr} ${100 - thr}L100 0"/></svg>`;
const hits = (h, o = {}) => { const xs = [14, 50, 86, 122, 158, 194, 230, 266], body = [];
  for (let x = 2; x < W; x += 3) { const near = xs.some((c) => x >= c && x < c + 22), a = near ? 0.9 - (xs.reduce((m, c) => (x >= c ? Math.min(m, x - c) : m), 99) / 26) : 0.12 + 0.06 * Math.sin(x * 1.7); body.push(`M${x} ${f1(h / 2 - a * h * 0.45)}V${f1(h / 2 + a * h * 0.45)}`); }
  const closed = xs.map((c, i) => { const a = c + (o.hold || 26), b = (xs[i + 1] || W + 14) - 2; return b > a ? `<rect class="cz" x="${a}" y="0" width="${b - a}" height="${h}"/>` : ''; }).join('');
  return `${closed}<path class="wv" d="${body.join('')}"/>`; };
const gateEd = () => ed('gate', 'closes 30 dB between the hits', `${well(64, hits(64), '', 'wvw')}${legend([['cz', 'closed: the rest of the kit, 24 dB down, is taken away']])}
    <div class="fx g3"><div class="tc-curve">${gateCurve()}</div><div class="col g2 grow">${nokey}<div class="fx g2">${flabel('Detector above', field('50', 'Hz'))}${flabel('below', field('8.0', 'kHz'))}</div></div></div>
    ${knobs(kb('Threshold', '−38', 0.4), kb('Range', '30 dB', 0.4), kb('Ratio', 'gate', 1), kb('Attack', '0.5', 0.1), kb('Hold', '40', 0.25), kb('Release', '120', 0.3))}
    ${info('Ratio under 20 is a gentler expander; less range sounds more natural.')}`);

// Dynamic EQ on a guitar, listening to the lead vocal.
const deqEd = () => { const h = 76, e = L.eqGeo(W, h, [{ f: 3000, g: -2.1, q: 1.2 }], 0, 9), m = L.eqGeo(W, h, [{ f: 3000, g: -3, q: 1.2 }], 0, 9), x = fx(3000);
  const lane = L.sections.map((s) => `<i class="${s[0] === 'Chorus' || s[0].startsWith('Verse') ? 'on' : ''}" style="left:${f1((s[1] / L.T) * 100)}%;width:${f1(((s[2] - s[1]) / L.T) * 100 - 0.6)}%"></i>`).join('');
  return ed('deq', 'one band · 3.0 kHz', `${well(h, `<path class="sp" d="${L.spectrum(W, h, 31)}"/><path class="zr" d="${e.zero}"/><path class="rf" d="${m.line}"/><path class="ar" d="${e.fill}"/><path class="cv" d="${e.line}"/>`, `<i class="dot is-sel" style="left:${px(x)};top:${px(h / 2 + 12)}"></i>${ticks()}`)}
    ${legend([['cv', 'now'], ['rf', 'as far as it goes'], ['zr', 'when the vocal rests']])}
    <div class="fx g2">${key(11, 'Lead Vox')}<span class="grow"></span>${btn('Band', 'quiet sm', 'plus')}</div>
    <div class="col g1"><span class="faint s1">When it works · the vocal’s verses and choruses</span><div class="when">${lane}</div></div>
    ${knobs(kb('Frequency', '3.0 kHz', 0.62), kb('Q', '1.2', 0.35), kb('Threshold', '−30', 0.4), kb('Range', '−3.0', 0.3, { bip: true }), kb('Attack', '10', 0.2), kb('Release', '120', 0.35))}
    ${info('Dips only while Lead Vox sings there: the guitar keeps its bite in the gaps.')}`); };

// Transient shaper on the snare: one hit before and after.
const envPath = (h, a, s, fill) => { const pts = []; for (let x = 0; x <= W; x += 3) { const t = x / W; let v = t < 0.05 ? 0.08 : t < 0.09 ? 0.08 + ((t - 0.05) / 0.04) * a : a * Math.exp(-(t - 0.09) * (5.5 / s)); pts.push(f1(x) + ' ' + f1(h - 4 - Math.min(1, v) * (h - 10))); } const d = 'M' + pts.join('L'); return fill ? d + `L${W} ${h}L0 ${h}Z` : d; };
const trEd = () => { const h = 70; return ed('tr', 'more attack, shorter tail', `${well(h, `<path class="ar" d="${envPath(h, 1, 0.75, 1)}"/><path class="rf" d="${envPath(h, 0.72, 1)}"/><path class="cv" d="${envPath(h, 1, 0.75)}"/>`, '<span class="wl" style="left:11%">attack</span><span class="wl" style="left:46%">sustain</span>')}
    ${legend([['cv', 'after'], ['rf', 'the hit as recorded']])}
    ${knobs(kb('Attack', '+4.0 dB', 0.72, { bip: true }), kb('Sustain', '−3.0 dB', 0.33, { bip: true }), kb('Speed', '40 %', 0.4), kb('Mix', '100 %', 1))}
    ${info('No threshold: soft and loud hits are shaped alike. Speed: how long counts as attack.')}`); };

// Saturation with its four characters.
const harm = (vals, even) => `<div class="bars harm">${vals.map((v, i) => `<i class="${(i % 2 === 0) === even ? '' : 'dimd'}" style="height:${v}%"></i>`).join('')}</div>`;
const satEd = () => ed('sat', 'tube · even harmonics', `<div class="fx g2">${seg(['Soft', 'Tape', 'Tube', 'Transistor'], 2, 'quiet sm', 'Character')}</div>
    <div class="fx g3"><div class="tc-curve"><svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true"><path class="un" d="M0 100L100 0"/><path class="cv" d="M0 100C30 70 40 46 50 50S78 18 100 10"/></svg></div>
      <div class="col g1 grow"><span class="faint s1">Harmonics it adds · 2nd to 7th</span>${harm([90, 34, 52, 18, 26, 9], true)}<span class="faint s1">lit: the even ones, the warm ones</span></div></div>
    ${knobs(kb('Drive', '6.0 dB', 0.33), kb('Mix', '60 %', 0.6))}
    <div class="col g1 s1 chars">${[['Soft', 'rounds evenly, the default'], ['Tape', 'rounds the peaks, a little darker'], ['Tube', 'warmer, even harmonics, not symmetric'], ['Transistor', 'harder, odd harmonics: grit']].map((c, i) => `<div class="fx g2 ${i === 2 ? 'on' : ''}"><span class="b chn">${c[0]}</span><span class="dim">${c[1]}</span></div>`).join('')}</div>
    ${info('Level is matched: Drive changes the colour, not the loudness.')}`);

// Bass enhancer and exciter: what they add, on the spectrum.
const lines = (fs, h) => fs.map((f) => `<path class="hm" d="M${f1(fx(f))} ${h * 0.3}V${h}"/>`).join('');
const bassEd = () => { const h = 80; return ed('bass', 'harmonics of what is under 90 Hz', `${well(h, `<rect class="bz" x="0" y="0" width="${f1(fx(90))}" height="${h}"/><path class="sp" d="${L.spectrum(W, h, 44, 0.8)}"/>${lines([110, 165, 220, 330], h)}`, `<span class="wl" style="left:${px(fx(30))}">fundamentals</span><span class="wl ai" style="left:${px(fx(140))}">added harmonics</span>${ticks()}`)}
    ${knobs(kb('Frequency', '90 Hz', 0.4), kb('Harmonics', '40 %', 0.4), kb('Mix', '50 %', 0.5))}
    ${info('Small speakers cannot play the lowest notes, but they play their harmonics and the ear fills in the note.')}`); };
const excEd = () => { const h = 80, x = fx(5000); return ed('exc', 'above 5.0 kHz', `${well(h, `<rect class="bz" x="${f1(x)}" y="0" width="${f1(W - x)}" height="${h}"/><path class="sp" d="${L.spectrum(W, h, 52)}"/><path class="ar" d="M${f1(x)} ${h}L${f1(x + 20)} ${h * 0.55}L${f1(W - 10)} ${h * 0.62}L${W} ${h}Z"/>`, `<i class="dot is-sel" style="left:${px(x)};top:${px(h * 0.55)}"></i><span class="wl ai" style="left:${px(x + 26)}">added sparkle</span>${ticks()}`)}
    ${knobs(kb('Frequency', '5.0 kHz', 0.64), kb('Drive', '6.0 dB', 0.33), kb('Amount', '25 %', 0.25))}
    ${info('Saturates only the top and blends it back: presence without more EQ.')}`); };

// Width: the stereo image of the track.
const widEd = () => ed('wid', '140 % · mono under 120 Hz', `<div class="stage"><span class="faint s1">L</span><div class="sfield grow"><i class="span" style="left:8%;right:8%"></i><i class="orig" style="left:22%;right:22%"></i><i class="c"></i></div><span class="faint s1">R</span></div>
    ${legend([['acc', 'with Width'], ['rf', 'as recorded']])}
    <div class="col g1"><span class="faint s1">Correlation</span><div class="corr"><i style="left:62%"></i></div><div class="fx between faint s1"><span>−1 out of phase</span><span>+1 mono</span></div></div>
    ${knobs(kb('Width', '140 %', 0.7), kb('Mono below', '120 Hz', 0.48))}
    ${info('On a mono track there is nothing to widen: put Width after a Modulation or a Doubler.')}`);

// Modulation on the Rhodes: the sweep over four bars, left and right apart.
const lfo = (h, ph = 0, n = 4) => { const pts = []; for (let x = 0; x <= W; x += 3) pts.push(f1(x) + ' ' + f1(h / 2 - Math.sin((x / W) * n * 2 * Math.PI + ph) * h * 0.32)); return 'M' + pts.join('L'); };
const bars = (h, n = 4) => Array.from({ length: n + 1 }, (_, i) => `<path class="gd" d="M${f1((i / n) * W)} 0V${h}"/>`).join('');
const modEd = () => { const h = 72; return ed('mod', 'chorus · 1/4 · locked to the bars', `<div class="fx g2">${seg(['Chorus', 'Flanger', 'Phaser'], 0, 'quiet sm', 'Mode')}<span class="grow"></span>${seg(['1', '2', '3'], 1, 'quiet sm', 'Voices')}</div>
    ${well(h, `${bars(h)}<path class="rf" d="${lfo(h, 2.1)}"/><path class="cv" d="${lfo(h)}"/>`, [1, 2, 3, 4].map((b) => `<span class="wl" style="left:${f1(((b - 1) / 4) * 100 + 1)}%">bar ${b + 16}</span>`).join(''))}
    ${legend([['cv', 'left'], ['rf', 'right · 120° apart']])}
    ${knobs(kb('Rate', '1/4', 0.5, { cls: 'is-active' }), kb('Depth', '40 %', 0.4), kb('Feedback', '—', 0, { cls: 'is-dis' }), kb('Spread', '120°', 0.66), kb('Mix', '35 %', 0.35))}
    ${info('The sweep follows the bars, not the play button: what you hear from the middle is what is exported.')}`); };

// Tremolo on a guitar.
const sq = (h) => { const pts = []; for (let x = 0; x <= W; x += 2) { const s = Math.sin((x / W) * 8 * 2 * Math.PI); pts.push(f1(x) + ' ' + f1(h / 2 - Math.tanh(s * 4) * h * 0.3)); } return 'M' + pts.join('L'); };
const trmEd = () => { const h = 64; return ed('trm', 'tremolo · 1/8 · square', `<div class="fx g2">${seg(['Tremolo', 'Auto-pan'], 0, 'quiet sm', 'Mode')}<span class="grow"></span>${seg(['Sine', 'Triangle', 'Square'], 2, 'quiet sm', 'Shape')}</div>
    ${well(h, `${bars(h, 2)}<path class="cv" d="${sq(h)}"/>`, '<span class="wl" style="left:1%">bar 49</span><span class="wl" style="left:51%">bar 50</span>')}
    ${knobs(kb('Rate', '1/8', 0.5, { cls: 'is-active' }), kb('Depth', '60 %', 0.6), kb('Stereo phase', '0°', 0))}
    ${info('Auto-pan moves the track left and right instead: a mono track becomes stereo from here on.')}`); };

// Harmonizer on the lead vocal.
const ladder = (marks) => { const st = Array.from({ length: 25 }, (_, i) => i - 12); return `<div class="ladder">${st.map((s) => `<i class="${s % 12 === 0 ? 'oct' : ''} ${marks.includes(s) ? 'on' : ''}" title="${s > 0 ? '+' : ''}${s}"></i>`).join('')}</div><div class="fx between faint s1"><span>−12 · octave down</span><span>0</span><span>+12 · octave up</span></div>`; };
const voice = (n, i, c, lv, p) => `<div class="fx g2 vrow"><span class="b vn">Voice ${n}</span>${field(i, 'st', n === 1 ? 'is-edit' : '')}${field(c, 'ct')}${field(lv, 'dB')}${field(p, '')}</div>`;
const hrmEd = () => ed('hrm', 'two voices · +4 and +7', `<div class="fx g2">${seg(['1 voice', '2 voices'], 1, 'quiet sm', 'Voices')}</div>${ladder([4, 7])}
    <div class="col g1">${voice(1, '+4', '0', '−6.0', 'L30')}${voice(2, '+7', '+6', '−8.0', 'R30')}</div>
    ${knobs(kb('Mix', '30 %', 0.3))}
    <div class="tile row twarn">${ic('warn', 'warnc')}<span class="dim s1">Fixed intervals: +4 is a major third above every note, so on a moving melody some notes will clash. Listen, or keep it to a few phrases.</span></div>`);

// Doubler on the lead vocal.
const dblWave = (h, dx, dy, cls) => { const pts = []; for (let x = 0; x <= W; x += 3) pts.push(f1(x + dx) + ' ' + f1(h / 2 + dy - Math.sin(x * 0.09) * Math.sin(x * 0.013) * h * 0.34)); return `<path class="${cls}" d="M${pts.join('L')}"/>`; };
const dblEd = () => { const h = 64; return ed('dbl', 'one voice · 12 cents · 18 ms', `<div class="fx g2">${seg(['1 voice', '2 voices'], 0, 'quiet sm', 'Voices')}</div>
    ${well(h, `${dblWave(h, 6, 2, 'rf')}${dblWave(h, 0, 0, 'cv')}`)}${legend([['cv', 'the take'], ['rf', 'the double: a little later, a little out of tune, moving']])}
    ${knobs(kb('Detune', '12 ct', 0.4), kb('Delay', '18 ms', 0.37), kb('Movement', '30 %', 0.3), kb('Width', '80 %', 0.8), kb('Mix', '35 %', 0.35))}
    ${info('Inside this track: no copy of the clip, no new track.')}`); };

// Low-pass with resonance, and its frequency set per section.
const lpCurve = (h, fc, q) => { const pts = []; for (let x = 0; x <= W; x += 2) { const f = 20 * Math.pow(1000, x / W), r = f / fc, m = 1 / Math.sqrt((1 - r * r) ** 2 + (r / q) ** 2), d = 20 * Math.log10(m); pts.push(f1(x) + ' ' + f1(Math.max(1, Math.min(h - 1, h / 2 - (d / 18) * (h / 2))))); } const l = 'M' + pts.join('L'); return { l, fill: l + `L${W} ${h / 2}L0 ${h / 2}Z` }; };
const lpEd = () => { const h = 80, a = lpCurve(h, 2200, 2.0), b = lpCurve(h, 16000, 2.0), x = fx(2200);
  return ed('lpf', '2.2 kHz in the bridge · resonance 2.0', `${well(h, `<path class="sp" d="${L.spectrum(W, h, 61)}"/><path class="zr" d="M0 ${h / 2}H${W}"/><path class="rf" d="${b.l}"/><path class="ar" d="${a.fill}"/><path class="cv" d="${a.l}"/>`, `<i class="dot is-sel" style="left:${px(x)};top:${px(h / 2 - 14)}"></i>${ticks()}`)}
    ${legend([['cv', 'in the bridge'], ['rf', 'the rest of the song']])}
    ${knobs(kb('Frequency', '16 kHz', 0.92), kb('Resonance', '2.0', 0.35), kb('Bridge', '−3 oct', 0.2, { cls: 'is-active', bip: true }))}
    ${info('The Bridge offset is automation on the frequency lane: it glides at the edges and can be edited by hand.')}`); };

// The chain of a return: a compressor that ducks the plate under the vocal.
const duck = (h) => { const voc = [[10, 40], [52, 100], [118, 160], [176, 230], [246, 286]]; let d = ''; const pts = [];
  let y = h * 0.22; for (let x = 0; x <= W; x += 3) { const on = voc.some((v) => x >= v[0] && x < v[1]), t = on ? h * 0.62 : h * 0.22; y += (t - y) * (on ? 0.6 : 0.12); pts.push(f1(x) + ' ' + f1(y)); }
  d = voc.map((v) => `<rect class="vz" x="${v[0]}" y="${h - 10}" width="${v[1] - v[0]}" height="6" rx="2"/>`).join(''); return `${d}<path class="cv" d="M${pts.join('L')}"/>`; };
const plateTop = (algo = 0) => `<div class="sect"><div class="fx g2">${tgl('by', true)}<span class="b grow">Reverb</span>${L.sel(['Plate', 'Room', 'Hall', 'Shimmer'][algo])}</div>
  ${knobs(kb('Decay', algo === 3 ? '4.5 s' : '1.2 s', algo === 3 ? 0.7 : 0.25), kb('Size', '40 %', 0.4), kb('Pre-delay', '20 ms', 0.2), kb('Damping', '30 %', 0.3))}</div>`;
const duckEd = () => { const h = 64; return ed('comp', 'ducking · listens to Lead Vox', `${well(h, duck(h), '<span class="wl" style="left:1%">Plate level</span>')}${legend([['vz', 'Lead Vox sings']])}
    <div class="fx g2">${key(11, 'Lead Vox')}<span class="grow"></span>${tag('−6 dB while it sings', 'acc')}</div>
    ${knobs(kb('Threshold', '−30', 0.4), kb('Ratio', '4.0', 0.4), kb('Attack', '5', 0.12), kb('Release', '250', 0.5), kb('Mix', '100 %', 1))}
    ${info('The reverb steps back while the vocal sings and blooms in the gaps.')}`); };

// Gated reverb on a room return keyed by the snare.
const gtail = (h) => { const hs = [20, 92, 164, 236], pts = []; for (let x = 0; x <= W; x += 2) { let v = 0.04; for (const c of hs) if (x >= c) { const t = x - c; v = Math.max(v, t < 2 ? 0.9 : t < 46 ? 0.75 * Math.exp(-t / 90) : t < 52 ? 0.75 * Math.exp(-46 / 90) * (1 - (t - 46) / 6) : 0); } pts.push(f1(x) + ' ' + f1(h - 4 - v * (h - 8))); } const l = 'M' + pts.join('L'); return `<path class="ar" d="${l}L${W} ${h}L0 ${h}Z"/><path class="cv" d="${l}"/>`; };
const gatedEd = () => { const h = 64; return ed('gate', 'listens to Snare · hold 180 ms', `${well(h, gtail(h), '<span class="wl" style="left:1%">Room after each snare: full, then cut</span>')}
    <div class="fx g2">${key(2, 'Snare')}</div>
    ${knobs(kb('Threshold', '−24', 0.6), kb('Range', '60 dB', 0.75), kb('Hold', '180', 0.6), kb('Release', '30', 0.08))}
    ${info('A big room that stops dead after every snare: the gated drum sound.')}`); };

// Delay in ping-pong with the tape character.
const ping = (h) => Array.from({ length: 7 }, (_, i) => { const x = 18 + i * 38, a = 0.85 * Math.pow(0.72, i), up = i % 2 === 0; return `<rect class="${up ? 'pl' : 'pr'}" x="${x}" y="${f1(up ? h / 2 - a * (h / 2 - 4) : h / 2)}" width="14" height="${f1(a * (h / 2 - 4))}" rx="2"/>`; }).join('') + `<path class="zr" d="M0 ${h / 2}H${W}"/>`;
const delayTop = `<div class="sect ted"><div class="fx g2">${tgl('by', true)}<span class="b grow">Delay</span>${seg(['Stereo', 'Ping-pong'], 1, 'quiet sm', 'Mode')}</div>
  ${well(72, ping(72), '<span class="wl" style="left:1%">left</span><span class="wl" style="left:1%;top:auto;bottom:4px">right</span>')}
  <div class="fx g2">${seg(['Clean', 'Tape'], 1, 'quiet sm', 'Character')}<span class="faint s1 grow">repeats darken and wobble</span></div>
  ${knobs(kb('Time', '1/8.', 0.5, { cls: 'is-active' }), kb('Feedback', '35 %', 0.5), kb('Wow', '30 %', 0.3), kb('Drive', '4.0 dB', 0.44))}
  <div class="fx g2">${flabel('Feedback above', field('200', 'Hz'))}${flabel('below', field('6.0', 'kHz'))}</div>
  ${info('Ping-pong bounces the repeats between the sides. Tape adds a slow wobble and a little drive to each repeat.')}</div>`;
// Shimmer: the reverb with an interval in its tail.
const shimTail = (h) => { const a = [], b = []; for (let x = 0; x <= W; x += 3) { const t = x / W; a.push(f1(x) + ' ' + f1(h - 4 - Math.exp(-t * 3.2) * (h - 10))); b.push(f1(x) + ' ' + f1(h - 4 - Math.min(1, t * 3) * Math.exp(-t * 2.2) * (h - 10) * 0.55)); } return `<path class="ar" d="M${a.join('L')}L${W} ${h}L0 ${h}Z"/><path class="cv" d="M${a.join('L')}"/><path class="rf ai" d="M${b.join('L')}"/>`; };
const shimTop = `<div class="sect ted"><div class="fx g2">${tgl('by', true)}<span class="b grow">Reverb</span>${seg(['Plate', 'Room', 'Hall', 'Shimmer'], 3, 'quiet sm', 'Algorithm')}</div>
  ${well(72, shimTail(72), '<span class="wl" style="left:1%">the tail</span><span class="wl ai" style="left:40%">an octave up, growing in</span>')}
  <div class="fx g2"><span class="faint s1">Interval</span>${seg(['+5', '+7', '+12'], 2, 'quiet sm', 'Interval')}<span class="faint s1 grow">semitones</span></div>
  ${knobs(kb('Decay', '4.5 s', 0.7), kb('Amount', '35 %', 0.35), kb('Size', '80 %', 0.8), kb('Damping', '40 %', 0.4), kb('Pre-delay', '30 ms', 0.3))}
  <div class="fx g2">${flabel('Return above', field('300', 'Hz'))}${flabel('below', field('9.0', 'kHz'))}</div>
  ${info('A halo above the sound, for pads and long notes. Amount 0 is the Hall.')}</div>`;

// ---------- the menu to add a block ----------
const bmenu = (o = {}) => { const dis = o.dis || {}; const cols = [['Dynamics', ['gate', 'ds', 'deq', 'tr', 'comp', 'clip', 'mb']], ['Tone', ['hpf', 'lpf', 'eq', 'sat', 'exc', 'bass']], ['Space & movement', ['wid', 'mod', 'trm']], ['Pitch', ['hrm', 'dbl']]];
  const it = (k) => `<div class="bmi ${dis[k] ? 'is-dis' : ''} ${o.hover === k ? 'is-hover' : ''}">${sgOf(k, dis[k] ? 'off' : '')}<div class="col"><span class="b s1">${CAT[k][0]}</span><span class="faint s1">${dis[k] || CAT[k][3]}</span></div></div>`;
  return `<div class="bmenu" role="menu" style="${o.style || ''}"><div class="fx g2 bmh"><span class="b grow">Add a block</span><span class="faint s1">lands ${o.where || 'before the fader'}</span></div><div class="bcols">${cols.map((c) => `<div class="col g1"><span class="mh">${c[0]}</span>${c[1].map(it).join('')}</div>`).join('')}</div></div>`; };

// ---------- the boards ----------
const VOX = [['hpf', '90 Hz', ''], ['eq', '4 bands', ''], ['comp', '3.0:1', '', 30], ['ds', '6.5 kHz', '', 42, tag('New', 'ai')], ['sat', 'Tube · 4 dB', ''], ['dbl', '12 ct', 'off', 0, tag('Choruses', '')]];
const vox = (o = {}) => insp({ tint: 11, name: 'Lead Vox', role: 'lead vocal', list: VOX, open: 'ds', editor: dsEd(), ...o });
const scr = (o, i) => P.mixScreen({ insp: i, strips: longStrips(false), ...o });
// The rows as they are while De-esser is dragged above Compressor.
const dragRows = `${trow(VOX[0])}${trow(VOX[1])}<div class="rslot" aria-hidden="true"></div>${trow(VOX[2], { cls: 'is-ghost' })}${trow(VOX[4])}${trow(VOX[5])}
  <div class="rrow tr is-drag" style="top:calc(var(--ctl-h-s) * 3 + var(--sp-2))">${ic('dots', 'sm')}${sgOf('ds', '')}<span class="b ell rname">De-esser</span><span class="dim s1 ell grow">6.5 kHz</span><i class="led"></i></div>`;
const menuOpen = bmenu({ style: 'left:150px;top:250px', hover: 'gate', dis: { comp: 'Two already: remove one first', ds: 'Two already' }, where: 'before the fader' });

// Bigger chains for the marks and the Overview.
const BLK = {
  'Kick': [['HP', 1], ['GT', 1, 'Gate'], ['EQ', 1], ['CMP', 1], ['TR', 1, 'Transient']], 'Snare': [['HP', 1], ['GT', 1, 'Gate'], ['TR', 1, 'Transient'], ['EQ', 1], ['CMP', 1], ['SAT', 1]],
  'Bass DI': [['HP', 1], ['EQ', 1], ['CMP', 1, 'Compressor · listens to Kick', 'key t1'], ['SAT', 1, 'Saturation · tube'], ['BAS', 1, 'Bass enhancer']], 'Gtr Left': [['HP', 1], ['LP', 1], ['EQ', 1], ['DEQ', 1, 'Dynamic EQ · listens to Lead Vox', 'key t11']],
  'Gtr Right': [['HP', 1], ['LP', 1], ['EQ', 1], ['DEQ', 1, 'Dynamic EQ · listens to Lead Vox', 'key t11'], ['TRM', 0, 'Tremolo · bridge only']], 'Rhodes': [['EQ', 1], ['MOD', 1, 'Modulation · chorus'], ['WID', 1, 'Width']],
  'Strings': [['HP', 1], ['LP', 1, 'Low-pass · resonance 2.0'], ['EQ', 2], ['WID', 1, 'Width']], 'Lead Vox': [['HP', 1], ['EQ', 1], ['CMP', 1], ['DS', 1, 'De-esser'], ['SAT', 1, 'Saturation · tube'], ['DBL', 0, 'Doubler · choruses only'], ['HRM', 0, 'Harmonizer · bridge only']],
};
const ROWS = [['High-pass', 'HP'], ['Low-pass', 'LP'], ['EQ', 'EQ'], ['Compressor', 'CMP'], ['Saturation', 'SAT'], ['Gate', 'GT'], ['De-esser', 'DS'], ['Dynamic EQ', 'DEQ'], ['Transient', 'TR'], ['Modulation', 'MOD'], ['Bass enhancer', 'BAS'], ['Width', 'WID'], ['Tremolo', 'TRM'], ['Doubler', 'DBL'], ['Harmonizer', 'HRM']];
const VAL = { GT: '−38', DS: '6.5k', DEQ: '3k', TR: '+4', BAS: '90', WID: '140%', MOD: 'chorus', TRM: '1/8', DBL: '12ct', HRM: '+4 +7' };
const withBlk = (t) => (BLK[t.n] ? { ...t, blk: BLK[t.n].map((b) => [b[0], b[1], b[2], b[3]]), blkMax: 5 } : t);
const gridStrip = (s) => { const b = BLK[s.n]; if (!b) return s; const ch = ROWS.slice(0, 10).map((r) => { const x = b.find((y) => y[0] === r[1]); return x ? (x[1] === 1 ? 1 : x[1] === 2 ? 2 : 0) : 0; });
  return { ...s, ch, v: ROWS.slice(0, 10).map((r, i) => (s.v && i < 5 ? s.v[i] : VAL[r[1]] || '')) }; };
const longTracks = L.tracks.map(withBlk);
const longStrips = (grid) => P.allStrips().map((s) => (grid ? gridStrip(s) : withBlk(s)));

const PLATE_LIST = [['comp', 'ducks under Lead Vox', '', 50, '', 'key t11'], ['eq', '2 bands', '']];
const plateInsp = insp({ tint: 'b', name: 'Plate', role: 'reverb return', out: 'Mix', top: plateTop(0), list: PLATE_LIST, open: 'comp', editor: duckEd(), first: false, last: ['Fader · Pan', '−12.0 dB · C'], title: 'After the reverb', count: '2 of 6', sends: false });

export default {
  'screens/16-track-chain.html': P.page('16 · Track chain', 'P-018. The chain of a track or a group is a list of blocks in any order, like the master’s rack (screen 15), shown in the inspector: Trim first, Fader and Pan last, everything in between free. A new track starts with the five blocks of always; thirteen more can be added, each at most twice, twelve in all. Returns get a short chain of their own after the reverb or the delay. The marks of P-013 follow the list. Replaces the row of chain buttons at the top of the inspector (3a); the rest of the inspector stands.', [
    P.frame('16a', 'The chain in the inspector, De-esser open', 'One row per block, top to bottom, with its label, its value, the reduction it applies and its switch. A block the assistant added is marked New until the proposal is applied; a block that only plays in some sections says where. The selected block opens below the list; Sends stay at the foot (A-065). The header of Lead Vox shows the same labels, in the same order.', scr({ tracks: longTracks }, vox())),
    P.frame('16b', 'Adding a block', '“Add block” opens a menu beside the inspector, one column per family, each block with what it does in plain words. What cannot be added says why. The block lands just before the fader, bypassed, and opens below the list; dragging puts it elsewhere. One undo step.', scr({ tracks: longTracks, over: menuOpen }, vox({ addOpen: true }))),
    P.frame('16c', 'Reordering', 'De-esser dragged above Compressor: a line shows where it lands. Trim, Fader and Pan cannot move. The same with ⌥↑ and ⌥↓ on the selected row. One undo step, and no click while it plays.', scr({ tracks: longTracks }, vox({ rowsHtml: dragRows, sub: 'moving De-esser' }))),
    P.frame('16d', 'Gate', 'On the kick mic: the hits, the bleed between them and where the gate closes. The detector can listen to a band only, or to another track (sidechain).', scr({ tracks: longTracks, sel: 0, ti: 0 }, insp({ tint: 1, name: 'Kick', role: 'kick', out: 'Drums', list: [['hpf', '40 Hz', ''], ['gate', '−38 dB', '', 60], ['eq', '3 bands', ''], ['comp', '4.0:1', '', 26], ['tr', '+4 / −3', '']], open: 'gate', editor: gateEd(), trim: '0.0 dB', last: ['Fader · Pan', '−6.5 dB · C'], sends: [['Plate', '—', 0], ['Echo', '—', 0]] }))),
    P.frame('16e', 'Dynamic EQ with sidechain', 'On Gtr Left, listening to Lead Vox: the band dips at 3 kHz only while the vocal sings there. The curve shows how far it can go and where it is now; the strip below says when it works.', scr({ tracks: longTracks, sel: 6, ti: 6 }, insp({ tint: 7, name: 'Gtr Left', role: 'electric gtr', out: 'Guitars', list: [['hpf', '100 Hz', ''], ['lpf', '9 kHz', ''], ['eq', '2 bands', ''], ['deq', '3.0 kHz', '', 21, '', 'key t11']], open: 'deq', editor: deqEd(), trim: '−1.1 dB', last: ['Fader · Pan', '−10.1 dB · L80'], sends: [['Plate', '—', 0], ['Echo', '—', 0]] }))),
    P.frame('16f', 'Transient shaper', 'On the snare: one hit as recorded and after. No threshold.', scr({ tracks: longTracks, sel: 1, ti: 1 }, insp({ tint: 2, name: 'Snare', role: 'snare', out: 'Drums', list: [['hpf', '90 Hz', ''], ['gate', '−34 dB', '', 40], ['tr', '+4 / −3', ''], ['eq', '2 bands', ''], ['comp', '3.0:1', '', 30], ['sat', 'Soft · 3 dB', '']], open: 'tr', editor: trEd(), trim: '+0.8 dB', last: ['Fader · Pan', '−8.0 dB · C'], sends: [['Plate', '−18.0', 48], ['Echo', '—', 0]] }))),
    P.frame('16g', 'Saturation and its characters', 'The saturation of always gains a character: Soft (as before, and what existing projects keep), Tape, Tube, Transistor. Each says in words what it does and which harmonics it adds.', scr({ tracks: longTracks, sel: 4, ti: 4 }, insp({ tint: 5, name: 'Bass DI', role: 'bass', list: [['hpf', '40 Hz', ''], ['eq', '2 bands', ''], ['comp', '4.0:1', '', 30, '', 'key t1'], ['sat', 'Tube · 6 dB', ''], ['bass', '90 Hz', '']], open: 'sat', editor: satEd(), trim: '−0.6 dB', last: ['Fader · Pan', '−7.2 dB · C'], sends: [['Plate', '—', 0], ['Echo', '—', 0]] }))),
    P.frame('16h', 'Bass enhancer', 'On the same bass: harmonics of the band under 90 Hz, so the bass line reads on a phone. Nothing is generated below.', scr({ tracks: longTracks, sel: 4, ti: 4 }, insp({ tint: 5, name: 'Bass DI', role: 'bass', list: [['hpf', '40 Hz', ''], ['eq', '2 bands', ''], ['comp', '4.0:1', '', 30, '', 'key t1'], ['sat', 'Tube · 6 dB', ''], ['bass', '90 Hz', '']], open: 'bass', editor: bassEd(), trim: '−0.6 dB', last: ['Fader · Pan', '−7.2 dB · C'], sends: [['Plate', '—', 0], ['Echo', '—', 0]] }))),
    P.frame('16i', 'Exciter', 'On the acoustic guitar: saturation of the top only, blended back.', scr({ tracks: longTracks, sel: 5, ti: 5 }, insp({ tint: 6, name: 'Acoustic', role: 'acoustic gtr', out: 'Guitars', list: [['hpf', '80 Hz', ''], ['eq', '3 bands', ''], ['comp', '2.0:1', '', 14], ['exc', '5.0 kHz', '']], open: 'exc', editor: excEd(), trim: '−1.8 dB', last: ['Fader · Pan', '−12.4 dB · L30'], sends: [['Plate', '−20.0', 40], ['Echo', '—', 0]] }))),
    P.frame('16j', 'Width', 'On the strings: the stereo image as recorded and with the block, and the correlation. Mono below a frequency keeps the low end centred.', scr({ tracks: longTracks, sel: 9, ti: 9 }, insp({ tint: 10, name: 'Strings', role: 'strings', list: [['hpf', '120 Hz', ''], ['lpf', '16 kHz', ''], ['eq', 'flat', 'nu'], ['wid', '140 %', '']], open: 'wid', editor: widEd(), trim: '+1.2 dB', last: ['Fader · Pan', '−15.5 dB · C'], sends: [['Plate', '—', 0], ['Echo', '—', 0]] }))),
    P.frame('16k', 'Modulation', 'Chorus on the Rhodes, at a quarter note: the sweep over four bars, left and right 120° apart. Flanger and Phaser add feedback; Phaser its stages.', scr({ tracks: longTracks, sel: 8, ti: 8 }, insp({ tint: 9, name: 'Rhodes', role: 'keys', list: [['eq', '1 band', ''], ['mod', 'Chorus · 1/4', '', 0, tag('Choruses', '')], ['wid', '130 %', '']], open: 'mod', editor: modEd(), trim: '+0.4 dB', last: ['Fader · Pan', '−14.0 dB · R20'], sends: [['Plate', '—', 0], ['Echo', '−16.0', 55]] }))),
    P.frame('16l', 'Tremolo', 'On Gtr Right, in the bridge only: level in eighth notes with a square shape. Auto-pan moves the pan instead.', scr({ tracks: longTracks, sel: 7, ti: 7 }, insp({ tint: 8, name: 'Gtr Right', role: 'electric gtr', out: 'Guitars', list: [['hpf', '100 Hz', ''], ['lpf', '9 kHz', ''], ['eq', '2 bands', ''], ['deq', '3.0 kHz', '', 18, '', 'key t11'], ['trm', '1/8 · 60 %', '', 0, tag('Bridge', '')]], open: 'trm', editor: trmEd(), trim: '−1.0 dB', last: ['Fader · Pan', '−10.3 dB · R80'], sends: [['Plate', '—', 0], ['Echo', '—', 0]] }))),
    P.frame('16m', 'Harmonizer', 'Two voices at fixed intervals under the lead vocal in the bridge. The ladder shows where they sit; the warning says what a fixed interval means on a melody.', scr({ tracks: longTracks }, vox({ list: [...VOX.slice(0, 5), ['hrm', '+4 · +7', '', 0, tag('Bridge', '')]], open: 'hrm', editor: hrmEd() }))),
    P.frame('16n', 'Doubler', 'A second performance made inside the track: slightly late, slightly out of tune, moving. In the choruses only.', scr({ tracks: longTracks }, vox({ list: [...VOX.slice(0, 5), ['dbl', '12 ct · 18 ms', '', 0, tag('Choruses', '')]], open: 'dbl', editor: dblEd() }))),
    P.frame('16o', 'Filters that open between sections', 'Low-pass with resonance on the strings, three octaves lower in the bridge. The section offset is automation on the frequency lane, glides at the edges and can be edited by hand.', scr({ tracks: longTracks, sel: 9, ti: 9, rows: L.tracks.map((t, i) => ({ k: 't', i })).flatMap((r) => (r.i === 9 ? [r, { k: 'a', i: 9, label: 'Low-pass · frequency', sub: 'Bridge −3 oct · idea 2', val: '16 kHz', vals: [0.85, 0.85, 0.85, 0.85, 0.85, 0.3, 0.85, 0.85] }] : [r])) },
      insp({ tint: 10, name: 'Strings', role: 'strings', list: [['hpf', '120 Hz', ''], ['lpf', '16 kHz · res 2.0', '', 0, tag('Bridge', '')], ['eq', 'flat', 'nu'], ['wid', '140 %', '']], open: 'lpf', editor: lpEd(), trim: '+1.2 dB', last: ['Fader · Pan', '−15.5 dB · C'], sends: [['Plate', '—', 0], ['Echo', '—', 0]] }))),
    P.frame('16p', 'The chain of a return: ducking', 'Plate is selected (its row is at the foot of the timeline). Under the reverb, its own list of blocks (up to six): a compressor that listens to Lead Vox lowers the plate while the vocal sings, and an EQ. The compressor label takes Lead Vox’s colour, as on a track.', scr({ tracks: longTracks, sel: -1, noAuto: true, rows: [...L.tracks.map((t, i) => ({ k: 't', i })), { k: 'r', n: 'Plate', sub: 'plate · ducks under Lead Vox', cls: 'is-sel', feeds: [[11, 'Lead Vox', '−14.0'], [2, 'Snare', '−18.0'], [6, 'Acoustic', '−20.0'], [12, 'BVs', '−12.0']] }] }, plateInsp)),
    P.frame('16q', 'Gated reverb', 'A room return keyed by the snare: full after each hit, then cut. The gate is the same block as on a track.', scr({ tracks: longTracks, sel: -1, noAuto: true }, insp({ tint: 'b', name: 'Snare room', role: 'reverb return', top: plateTop(1).replace('1.2 s', '1.8 s'), list: [['gate', 'listens to Snare', '', 70, '', 'key t2'], ['eq', '1 band', '']], open: 'gate', editor: gatedEd(), first: false, last: ['Fader · Pan', '−9.0 dB · C'], title: 'After the reverb', count: '2 of 6', sends: false }))),
    P.frame('16r', 'Delay: ping-pong and tape', 'The delay return gains a mode (Stereo, Ping-pong) and a character (Clean, Tape). The diagram shows where the repeats land.', scr({ tracks: longTracks, sel: -1, noAuto: true }, insp({ tint: 'b', name: 'Echo', role: 'delay return', top: delayTop, list: [], first: false, last: ['Fader · Pan', '−15.0 dB · C'], title: 'After the delay', count: '0 of 6', sends: false }))),
    P.frame('16s', 'Reverb: shimmer', 'A fourth algorithm: the tail shifted up by an interval at each pass, growing in. Amount at 0 is the Hall.', scr({ tracks: longTracks, sel: -1, noAuto: true }, insp({ tint: 'b', name: 'Halo', role: 'reverb return', top: shimTop, list: [], first: false, last: ['Fader · Pan', '−16.0 dB · C'], title: 'After the reverb', count: '0 of 6', sends: false }))),
    P.frame('16t', 'Marks with long chains', 'Headers and strips carry one label per block in chain order, with the labels of the new blocks. Where they do not fit, the first four and “+n”; its tooltip names the rest. A bypassed block is faint, as before; a block keyed by another track takes that track’s colour.', P.mixScreen({ tracks: longTracks, strips: longStrips(false), sel: 10, noAuto: true, insp: vox() })),
    P.frame('16u', 'Overview with long chains', 'One row per type of block present in the project: the five of always, then the others in the order of the menu. With many types the grid scrolls down: here Bass enhancer, Width, Tremolo, Doubler and Harmonizer are below, then the sends, output and automation. Labels scroll with their rows.', P.mixScreen({ tracks: longTracks, strips: longStrips(true), view: 'grid', blockRows: ROWS.slice(0, 10), sel: 10, noAuto: true, insp: vox() })),
  ]),
};
