// Stemwise design boards — sample data, geometry and atoms. Design-time only: nothing here ships in the app.
export const T = 222; // song length in seconds
export const tracks = [
  { n: 'Kick', r: 'kick', g: 'Drums', lv: 0.72, db: -6.5, pan: 0, k: 'p', note: 1 },
  { n: 'Snare', r: 'snare', g: 'Drums', lv: 0.66, db: -8.0, pan: 0, k: 'p' },
  { n: 'Overheads', r: 'overheads', g: 'Drums', lv: 0.55, db: -11.5, pan: 0, k: 'p' },
  { n: 'Room', r: 'room', g: 'Drums', lv: 0.4, db: -16.0, pan: 0, k: 'p' },
  { n: 'Bass DI', r: 'bass', lv: 0.7, db: -7.2, pan: 0, k: 's', note: 2 },
  { n: 'Acoustic', r: 'acoustic gtr', g: 'Guitars', lv: 0.5, db: -12.4, pan: -0.3, k: 's' },
  { n: 'Gtr Left', r: 'electric gtr', g: 'Guitars', lv: 0.58, db: -10.1, pan: -0.8, k: 's', note: 3 },
  { n: 'Gtr Right', r: 'electric gtr', g: 'Guitars', lv: 0.57, db: -10.3, pan: 0.8, k: 's' },
  { n: 'Rhodes', r: 'keys', lv: 0.45, db: -14.0, pan: 0.2, k: 's' },
  { n: 'Strings', r: 'strings', lv: 0.42, db: -15.5, pan: 0, k: 's' },
  { n: 'Lead Vox', r: 'lead vocal', lv: 0.75, db: -5.8, pan: 0, k: 'v', note: 4 },
  { n: 'BVs', r: 'backing vox', lv: 0.48, db: -13.0, pan: 0, k: 'v', lock: 1 },
];
export const clipTimes = [
  [[14, 144], [168, 214]], [[16, 144], [168, 214]], [[0, 144], [166, 222]], [[0, 144], [166, 222]],
  [[16, 112], [112, 144], [168, 216]], [[0, 222]], [[48, 80], [112, 144], [168, 208]],
  [[48, 80], [112, 144], [168, 208]], [[0, 48], [80, 112], [144, 168], [208, 222]],
  [[80, 144], [144, 208]], [[16, 46], [48, 80], [82, 110], [112, 144], [146, 166], [168, 208]],
  [[48, 80], [112, 144], [168, 208]],
];
export const buses = [
  { n: 'Drums', t: 'group', lv: 0.7, db: -4.0 }, { n: 'Guitars', t: 'group', lv: 0.6, db: -6.5 },
  { n: 'Plate', t: 'reverb', lv: 0.35, db: -12.0 }, { n: 'Echo', t: 'delay', lv: 0.25, db: -15.0 },
  { n: 'Mix', t: 'mix bus', lv: 0.8, db: 0.0 },
];
// Glance state (P-013). ch: the five chain blocks in signal order (high-pass, low-pass, EQ, compressor, saturation),
// 0 bypassed · 1 working · 2 on but neutral; v: what each block says in the Overview; sc: index of the track the
// compressor listens to; par: compressor mix below 100 %; sn: send level per return, as [fader position, dB] or 0;
// au: who wrote the automation ('ai' | 'user') and on which parameter; out: where the output goes.
export const returns = ['Plate', 'Echo'];
const GL = [
  { ch: [1, 0, 1, 1, 0], v: ['40', '', '3 bands', '4:1', ''], sn: [0, 0], out: 'Drums' },
  { ch: [1, 0, 1, 1, 1], v: ['90', '', '2 bands', '3:1', '3 dB'], par: 35, sn: [[48, '−18'], 0], out: 'Drums' },
  { ch: [1, 0, 1, 0, 0], v: ['200', '', '2 bands', '', ''], sn: [0, 0], out: 'Drums' },
  { ch: [0, 0, 2, 1, 0], v: ['', '', 'flat', '6:1', ''], sn: [0, 0], out: 'Drums' },
  { ch: [1, 0, 1, 1, 0], v: ['40', '', '2 bands', '4:1', ''], sc: 0, sn: [0, 0], au: ['user', 'Fader'] },
  { ch: [1, 0, 1, 0, 0], v: ['80', '', '3 bands', '', ''], sn: [[40, '−20'], 0], out: 'Guitars' },
  { ch: [1, 1, 1, 0, 0], v: ['100', '9k', '2 bands', '', ''], sn: [0, 0], out: 'Guitars' },
  { ch: [1, 1, 1, 0, 0], v: ['100', '9k', '2 bands', '', ''], sn: [0, 0], out: 'Guitars' },
  { ch: [0, 0, 1, 0, 0], v: ['', '', '1 band', '', ''], sn: [0, [55, '−16']] },
  { ch: [1, 0, 2, 0, 0], v: ['120', '', 'flat', '', ''], sn: [0, 0] },
  { ch: [1, 0, 1, 1, 1], v: ['90', '', '4 bands', '3:1', '4 dB'], sn: [[62, '−14'], [38, '−22']], au: ['ai', 'Fader'] },
  { ch: [1, 0, 1, 1, 0], v: ['110', '', '2 bands', '2.5:1', ''], sn: [[70, '−12'], 0] },
];
tracks.forEach((t, i) => Object.assign(t, GL[i]));
const BGL = {
  Drums: { ch: [0, 0, 1, 1, 0], v: ['', '', '1 band', '2:1', ''], par: 50, sn: [0, 0] },
  Guitars: { ch: [0, 0, 1, 0, 0], v: ['', '', '2 bands', '', ''], sn: [0, 0] },
  Plate: { ret: 0, v: ['300', '12k', 'plate · 1.2 s'], feeds: 4 }, Echo: { ret: 1, v: ['200', '6k', 'delay · 1/8.'], feeds: 2 },
};
buses.forEach((b) => Object.assign(b, BGL[b.n] || {}));
export const sections = [
  ['Intro', 0, 16, 0.45], ['Verse 1', 16, 48, 0.65], ['Chorus', 48, 80, 1], ['Verse 2', 80, 112, 0.68],
  ['Chorus', 112, 144, 1], ['Bridge', 144, 168, 0.55], ['Chorus', 168, 208, 1], ['Outro', 208, 222, 0.4],
];
export const playhead = 62.48, loop = [48, 80];
export const WHY = 'Lead vocal sat 2.1 LU under the reference vocal-to-mix balance and was masked by the guitars around 2–4 kHz in the choruses. Raised it 1.8 dB, cut some 220 Hz mud, added presence at 3.2 kHz, and lifted it a further 1.5 dB in the choruses only.';
export const SUMMARY = 'Vocal forward, guitars opened to the sides, fuller choruses. Drums 1 dB down to make room; bass ducks slightly under the kick.';
export const DIFF = [['Fader', '−7.6', '−5.8 dB'], ['EQ 1 · 220 Hz', '0.0', '−2.5 dB'], ['EQ 2 · 3.2 kHz', '0.0', '+2.0 dB'], ['Comp threshold', '−14.0', '−18.0 dB'], ['Chorus · fader offset', '0.0', '+1.5 dB']];
export const NOTES = [
  [4, 'Lead Vox', 11, 'Up 1.8 dB, less mud, more presence; a further 1.5 dB in the choruses.'],
  [1, 'Kick · Snare', 1, 'Drums sit 1 dB lower as a group so the vocal has room; the kick keeps its click at 3 kHz.'],
  [2, 'Bass DI', 5, 'Ducks 2 dB on every kick (sidechain) to free the low end without thinning the bass.'],
  [3, 'Gtr Left · Right', 7, 'Opened wide and dipped 2.5 dB at 3 kHz, only where they masked the vocal.'],
];

// ---------- helpers ----------
function rng(a) { return function () { a |= 0; a = (a + 0x6d2b79f5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
const inten = (t, secs = sections) => { for (const s of secs) if (t >= s[1] && t < s[2]) return s[3]; return 0.4; };
export const f1 = (v) => (Math.round(v * 10) / 10).toString();
const clamp = (v) => Math.max(0, Math.min(1, v));
export const fmt = (s) => Math.floor(s / 60) + ':' + String(Math.floor(s % 60)).padStart(2, '0');
export const dB = (v) => (v > 0 ? '+' : v < 0 ? '−' : '') + Math.abs(v).toFixed(1);
export const panL = (p) => (p === 0 ? 'C' : (p < 0 ? 'L' : 'R') + Math.round(Math.abs(p) * 100));
export const fpos = (d) => Math.round(clamp((d + 40) / 52) * 100);
export const px = (v) => f1(v) + 'px';

export function wave(ti, c0, c1, pps, h, step = 2.5, secs = sections) {
  const r = rng(ti * 97 + Math.round(c0) * 13 + 5), k = tracks[ti] ? tracks[ti].k : 'm';
  const w = (c1 - c0) * pps, mid = h / 2, max = mid - 1; let v = 0.5; const up = [], dn = [];
  for (let x = 0; x <= w; x += step) {
    const t = c0 + x / pps; let a;
    if (k === 'p') a = 0.22 + 0.78 * Math.pow(r(), 2.2); else { v = v * 0.62 + r() * 0.38; a = 0.35 + 0.65 * v; }
    if (k === 'v' && Math.sin(t * 0.83 + ti) < -0.45) a *= 0.12;
    if (k === 'm') a = 0.5 + 0.5 * a;
    a *= inten(t, secs) * Math.min(1, x / 4 + 0.2, (w - x) / 4 + 0.2);
    const p = Math.max(0.6, a * max); up.push(f1(x) + ' ' + f1(mid - p)); dn.push(f1(x) + ' ' + f1(mid + p));
  }
  return { w, d: 'M' + up.join('L') + 'L' + dn.reverse().join('L') + 'Z' };
}
const fx = (f, w) => (Math.log(f / 20) / Math.log(1000)) * w;
export const voxEq = [{ f: 220, g: -2.5, q: 1.2 }, { f: 3200, g: 2.0, q: 0.9 }, { f: 6500, g: -1.5, q: 3 }, { f: 11000, g: 2.5, q: 1, t: 'hs' }];
export const masterEq = [{ f: 60, g: 0.8, q: 1, t: 'ls' }, { f: 250, g: -1.0, q: 1.2 }, { f: 3000, g: 0.6, q: 0.8 }, { f: 10000, g: 1.0, q: 0.7, t: 'hs' }];
export function eqGeo(w, h, bands, hpf, range = 12) {
  const mid = h / 2, pts = [];
  for (let x = 0; x <= w; x += 2) {
    const f = 20 * Math.pow(1000, x / w); let d = 0;
    for (const b of bands) { const o = Math.log2(f / b.f); d += b.t === 'ls' ? b.g / (1 + Math.exp(o * 3)) : b.t === 'hs' ? b.g / (1 + Math.exp(-o * 3)) : b.g * Math.exp(-o * o * b.q * 1.6); }
    if (hpf && f < hpf * 2) d -= 12 * Math.pow(Math.max(0, Math.log2((hpf * 2) / f)), 1.5) * 0.9;
    pts.push(f1(x) + ' ' + f1(Math.max(1, Math.min(h - 1, mid - (d / range) * mid))));
  }
  const line = 'M' + pts.join('L'), fs = [50, 100, 200, 500, 1000, 2000, 5000, 10000];
  return { line, fill: line + 'L' + w + ' ' + mid + 'L0 ' + mid + 'Z', zero: 'M0 ' + mid + 'H' + w, grid: fs.map((f) => 'M' + f1(fx(f, w)) + ' 0V' + h).join(''),
    handles: bands.map((b) => [fx(b.f, w), mid - (b.g / range) * mid]), ticks: fs.map((f) => [fx(f, w), f >= 1000 ? f / 1000 + 'k' : '' + f]) };
}
export function spectrum(w, h, seed, tilt = 0.55, smooth = false) {
  const r = rng(seed); let v = 0.5; const pts = [];
  for (let x = 0; x <= w; x += 3) {
    v = v * 0.7 + r() * 0.3; const u = x / w;
    const env = Math.sin(Math.min(1, u * 2.4) * Math.PI / 2) * (1 - tilt * u) * (u > 0.92 ? (1 - u) / 0.08 : 1);
    pts.push(f1(x) + ' ' + f1(h - env * (smooth ? 0.7 : 0.45 + 0.4 * v) * h));
  }
  return smooth ? 'M' + pts.join('L') : 'M0 ' + h + 'L' + pts.join('L') + 'L' + w + ' ' + h + 'Z';
}
export const autoVals = [0.5, 0.5, 0.74, 0.5, 0.74, 0.4, 0.78, 0.45];
export function autoGeo(pps, h, vals = autoVals, secs = sections, TT = T) {
  const pts = [], y = (v) => h - 6 - v * (h - 12), n = secs.length - 1;
  secs.forEach((s, i) => { const v = vals[i]; if (i === 0 || vals[i - 1] !== v) pts.push([s[1] * pps + (i ? 2 : 0), y(v)]); if (i === n || vals[i + 1] !== v) pts.push([s[2] * pps - (i === n ? 0 : 2), y(v)]); });
  const line = 'M' + pts.map((p) => f1(p[0]) + ' ' + f1(p[1])).join('L');
  return { pts, line, fill: line + 'L' + f1(TT * pps) + ' ' + h + 'L0 ' + h + 'Z' };
}
export function compGeo(thr = -18, ratio = 3, knee = 6) {
  const s = 100, X = (d) => ((d + 48) / 48) * s, pts = [];
  for (let d = -48; d <= 0; d += 1) { let o = d; if (d > thr + knee / 2) o = thr + (d - thr) / ratio; else if (d > thr - knee / 2) o = d + ((1 / ratio - 1) * Math.pow(d - thr + knee / 2, 2)) / (2 * knee); pts.push(f1(X(d)) + ' ' + f1(s - X(o))); }
  return `<svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true"><path class="un" d="M0 100L100 0"/><path class="th" d="M${f1(X(thr))} 0V100"/><path class="cv" d="M${pts.join('L')}"/></svg>`;
}

// ---------- icons (own set, 16 px grid, 1.5 px stroke) ----------
const IC = {
  play: '<path class="s" d="M5 3.500v9l7.500-4.500z"/>', pause: '<rect class="s" x="4" y="3.500" width="3" height="9" rx="1"/><rect class="s" x="9" y="3.500" width="3" height="9" rx="1"/>',
  stop: '<rect class="s" x="4" y="4" width="8" height="8" rx="1.500"/>', rew: '<path d="M4 3.500v9"/><path class="s" d="M12.500 3.500v9L6 8z"/>',
  loop: '<path d="M11 2.500l2 2-2 2M13 4.500H5.500a3 3 0 0 0-3 3M5 13.500l-2-2 2-2M3 11.500h7.500a3 3 0 0 0 3-3"/>',
  undo: '<path d="M5.500 3L2.500 6l3 3M2.500 6h7a3.500 3.500 0 0 1 0 7H6"/>', redo: '<path d="M10.500 3l3 3-3 3M13.500 6h-7a3.500 3.500 0 0 0 0 7H10"/>',
  lock: '<rect x="3.500" y="7" width="9" height="6.500" rx="1.500"/><path d="M5.500 7V5a2.500 2.500 0 0 1 5 0v2"/>', unlock: '<rect x="3.500" y="7" width="9" height="6.500" rx="1.500"/><path d="M5.500 7V5a2.500 2.500 0 0 1 4.800-1"/>',
  spark: '<path d="M8 1.500l1.600 4.900 4.900 1.600-4.900 1.600L8 14.500 6.400 9.600 1.500 8l4.900-1.600z"/>', chev: '<path d="M4 6l4 4 4-4"/>', right: '<path d="M6 4l4 4-4 4"/>', up: '<path d="M4 10l4-4 4 4"/>',
  plus: '<path d="M8 3v10M3 8h10"/>', minus: '<path d="M3 8h10"/>', x: '<path d="M4 4l8 8M12 4l-8 8"/>', check: '<path d="M3 8.500l3.200 3L13 4.500"/>',
  cut: '<circle cx="4.500" cy="11.500" r="2"/><circle cx="11.500" cy="11.500" r="2"/><path d="M5.800 10L11 2.500M10.200 10L5 2.500"/>',
  cursor: '<path d="M3.500 2.500l9 4.500-4 1.200-1.500 4z"/>', pen: '<path d="M2.500 13.500l1-3.500 7.500-7.500 2.500 2.500L6 12.500z"/>', send: '<path d="M8 13V3M3.500 7.500L8 3l4.500 4.500"/>',
  ear: '<path d="M2.500 9V8a5.500 5.500 0 0 1 11 0v1"/><rect x="2" y="9" width="3" height="4.500" rx="1"/><rect x="11" y="9" width="3" height="4.500" rx="1"/>',
  clock: '<circle cx="8" cy="8" r="5.500"/><path d="M8 5v3l2 1.500"/>', snap: '<path d="M4 2.500v6a4 4 0 0 0 8 0v-6M4 5h2.500M9.500 5H12"/>',
  dots: '<circle class="s" cx="3.500" cy="8" r="1.200"/><circle class="s" cx="8" cy="8" r="1.200"/><circle class="s" cx="12.500" cy="8" r="1.200"/>',
  folder: '<path d="M2 4.500a1 1 0 0 1 1-1h3l1.500 1.500H13a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>', file: '<path d="M4 2h5l3 3v8a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1z"/><path d="M9 2v3h3"/>',
  trash: '<path d="M3 4.500h10M6.500 4.500V3h3v1.500M4.500 4.500l.5 8.500h6l.5-8.500"/>', warn: '<path d="M8 2.500L14 13H2z"/><path d="M8 6.500v3M8 11.200v.1"/>',
  info: '<circle cx="8" cy="8" r="5.500"/><path d="M8 7.500V11M8 5.200v.1"/>', gear: '<circle cx="8" cy="8" r="2"/><path d="M6.90 3.23L7.08 1.46L8.92 1.46L9.10 3.23A4.9 4.9 0 0 1 10.60 3.84L11.97 2.73L13.27 4.03L12.16 5.40A4.9 4.9 0 0 1 12.77 6.90L14.54 7.08L14.54 8.92L12.770 9.10A4.9 4.9 0 0 1 12.16 10.60L13.27 11.97L11.97 13.27L10.60 12.16A4.9 4.9 0 0 1 9.10 12.77L8.92 14.54L7.08 14.54L6.90 12.77A4.9 4.9 0 0 1 5.40 12.16L4.03 13.27L2.73 11.97L3.84 10.60A4.9 4.9 0 0 1 3.23 9.10L1.46 8.92L1.46 7.08L3.23 6.90A4.9 4.9 0 0 1 3.84 5.40L2.73 4.03L4.03 2.73L5.40 3.84A4.9 4.9 0 0 1 6.90 3.23z"/>',
  wave: '<path d="M2 8h1.500M5 5v6M8 3v10M11 5.500v5M13 8h1"/>', exp: '<path d="M8 10V2.500M5 5.500l3-3 3 3M3 10v2.500a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V10"/>',
  link: '<path d="M6.500 9.500a2.500 2.500 0 0 0 3.500 0l2.500-2.500a2.500 2.500 0 0 0-3.500-3.500l-.7.700M9.500 6.500a2.500 2.500 0 0 0-3.500 0L3.500 9a2.500 2.500 0 0 0 3.500 3.500l.7-.7"/>',
  copy: '<rect x="5.500" y="5.500" width="8" height="8" rx="1.500"/><path d="M10.500 5.500V4a1.500 1.500 0 0 0-1.500-1.500H4A1.500 1.500 0 0 0 2.500 4v5A1.500 1.500 0 0 0 4 10.500h1.500"/>',
  power: '<path d="M8 2v5.500M4.600 4.200a5 5 0 1 0 6.800 0"/>', key: '<circle cx="5" cy="11" r="2.500"/><path d="M7 9l6-6M10.500 5.500l2 2M12.500 3.500l1 1"/>',
  offline: '<path d="M2 6a9 9 0 0 1 12 0M4.500 8.700a5 5 0 0 1 7 0M8 12v.1M2.500 2.500l11 11"/>', speaker: '<path d="M2.500 6.500h2L8 3.500v9l-3.500-3h-2zM10.500 6a3 3 0 0 1 0 4M12.200 4.200a5.500 5.500 0 0 1 0 7.600"/>',
  group: '<rect x="2" y="3" width="12" height="4" rx="1"/><path d="M4.500 7v5.500h9M4.500 9.800h9"/>', filter: '<path d="M2.500 3.500h11L9.500 8.500v4l-3 1v-5z"/>',
  ab: '<path d="M2.500 12L5 4l2.500 8M3.400 9.500h3.200M10 4h2a1.800 1.800 0 0 1 0 4h-2zM10 8h2.500a2 2 0 0 1 0 4H10z"/>', image: '<rect x="2.500" y="3" width="11" height="10" rx="1.500"/><path d="M2.500 10.500l3-3 3 3 2-2 3 3"/>',
  faders: '<path d="M4 2.500v11M8 2.500v11M12 2.500v11"/><rect class="s" x="2.500" y="8.500" width="3" height="2.500" rx=".8"/><rect class="s" x="6.500" y="4" width="3" height="2.500" rx=".8"/><rect class="s" x="10.500" y="7" width="3" height="2.500" rx=".8"/>', grid: '<rect x="2.500" y="2.500" width="11" height="11" rx="1.500"/><path d="M2.500 6.200h11M2.500 9.800h11M6.200 2.500v11M9.800 2.500v11"/>',
  map: '<circle cx="3" cy="4" r="1.500"/><circle cx="3" cy="12" r="1.500"/><circle cx="13" cy="8" r="1.500"/><path d="M4.500 4H6q2 0 2.500 2t3 2M4.500 12H6q2 0 2.500-2"/>', left: '<path d="M10 4L6 8l4 4"/>',
  level: '<path d="M3 5.500h10M3 10.500h10"/>', music: '<path d="M6 12V3.500l7-1.500v8.500"/><circle cx="4.500" cy="12" r="1.500"/><circle cx="11.500" cy="10.500" r="1.500"/>', eye: '<path d="M1.500 8s2.500-4.500 6.500-4.500S14.500 8 14.500 8 12 12.500 8 12.500 1.500 8 1.500 8z"/><circle cx="8" cy="8" r="2"/>',
};
export const ICON_NAMES = Object.keys(IC);
export const ic = (n, cls = '') => `<svg class="ic ${cls}" viewBox="0 0 16 16" aria-hidden="true">${IC[n]}</svg>`;


// ---------- glance marks (P-013) ----------
const BLOCKS = ['High-pass', 'Low-pass', 'EQ', 'Compressor', 'Saturation'];
// One short label per block, the ones printed on any mixer: nothing to decode and no position to remember. Solid when the
// block is working, outlined when it is on but neutral, faint when bypassed; a keyed compressor takes its source's tint,
// a parallel one carries two small bars.
export const SIG = ['HP', 'LP', 'EQ', 'CMP', 'SAT'];
export const sig = (i, cls = '', o = {}) => `<span class="sg ${cls}" title="${o.title || BLOCKS[i]}">${SIG[i]}${o.par ? '<i class="pl"></i>' : ''}</span>`;
// P-018: a chain of free blocks. `s.blk` lists [label, state (0 off, 1 working, 2 neutral), title, extra class] in chain
// order; past `s.blkMax` labels the rest folds into a "+n".
export const sigx = (label, cls = '', title = '') => `<span class="sg ${cls}" title="${title || label}">${label}</span>`;
const blkList = (s) => { const max = s.blkMax || 99, b = s.blk, shown = b.length > max ? b.slice(0, max - 1) : b;
  return shown.map((x) => sigx(x[0], (x[1] === 1 ? 'on' : x[1] === 2 ? 'nu' : '') + ' ' + (x[3] || ''), x[2])).concat(b.length > max ? [sigx('+' + (b.length - shown.length), 'more', b.slice(shown.length).map((x) => x[2] || x[0]).join(' · '))] : []); };
export const pipList = (s = {}) => s.blk ? blkList(s) : (s.ch || [0, 0, 0, 0, 0]).map((c, i) => {
  const key = i === 3 && c === 1 && s.sc !== undefined, par = i === 3 && c === 1 && s.par;
  return sig(i, key ? `on key t${s.sc + 1}` : c === 1 ? 'on' : c === 2 ? 'nu' : '', { par, title: `${BLOCKS[i]} · ${c === 1 ? (key ? 'listens to ' + tracks[s.sc].n : par ? 'parallel, ' + s.par + ' %' : 'on') : c === 2 ? 'on, changes nothing' : 'off'}` });
});
export const pips = (s) => pipList(s).join('');
// In a strip the labels can be laid out four ways (`layout`): 'col' one under the other at the strip's width, 'grid' two
// columns, 'rows' two rows, 'row' a single row (needs a strip more than twice as wide).
export const chainBlock = (s, layout = 'col') => {
  const p = s && (s.ch || s.blk) ? pipList(s) : [];
  if (layout === 'rows') return `<div class="gl rows"><div>${p.slice(0, 3).join('')}</div><div>${p.slice(3).join('')}</div></div>`;
  return `<div class="gl ${layout}">${p.join('')}</div>`;
};
export const CHAIN_H = { col: 62, grid: 32, rows: 21, row: 10 };
// The cell of a track header: chain labels above; below, one dot per return and the automation mark. `rel` is the index
// of the return the selection is about.
export const glance = (s = {}, o = {}) => {
  const sn = s.sn || (o.returns || returns).map(() => 0);
  return `<div class="gl ${sn.length > 5 ? 'many' : ''}" aria-label="Chain and sends"><div>${s.ch || s.blk ? pips(s) : ''}</div><div>${s.ret !== undefined ? '' : sn.map((x, i) => `<i class="sd ${x ? (o.rel === i ? 'rel' : 'on') : ''}" title="${(o.returns || returns)[i]}${x ? ' · ' + x[1] + ' dB' : ' · no send'}"></i>`).join('')}${s.au ? `<i class="pa ${s.au[0]}" title="Automation · ${s.au[1]}"></i>` : ''}</div></div>`;
};

// ---------- atoms ----------
export function knob(v, { size = '', bip = false } = {}) {
  const c = 20, sw = 4, r = 17, P = (a) => [c + r * Math.cos(a), c + r * Math.sin(a)];
  const a0 = Math.PI * 0.75, a1 = Math.PI * 2.25, av = a0 + (a1 - a0) * v, am = (a0 + a1) / 2;
  const arc = (s, e) => { if (e < s) [s, e] = [e, s]; if (e - s < 0.03) e = s + 0.03; const p = P(s), q = P(e); return `M${f1(p[0])} ${f1(p[1])}A${r} ${r} 0 ${e - s > Math.PI ? 1 : 0} 1 ${f1(q[0])} ${f1(q[1])}`; };
  const ri = r - sw - 2;
  return `<svg class="knob ${size}" viewBox="0 0 40 40" aria-hidden="true"><circle class="k-body" cx="20" cy="20" r="${ri}"/><path class="k-bg" d="${arc(a0, a1)}"/><path class="k-val" d="${arc(bip ? am : a0, av)}"/><path class="k-ptr" d="M${f1(c + ri * 0.3 * Math.cos(av))} ${f1(c + ri * 0.3 * Math.sin(av))}L${f1(c + (ri - 1) * Math.cos(av))} ${f1(c + (ri - 1) * Math.sin(av))}"/></svg>`;
}
export const kb = (l, val, v, o = {}) => `<div class="kb ${o.cls || ''}">${knob(v, o)}<span class="kv">${val}</span><span class="kl">${l}</span></div>`;
export const tgl = (kind, on = false, cls = '') => {
  const m = { mu: ['M', 'Mute'], so: ['S', 'Solo'], lk: [ic('lock'), 'Lock for AI'], by: [ic('power'), 'Bypass'] }[kind];
  return `<button class="tgl ${kind} ${on ? 'on' : ''} ${cls}" aria-label="${m[1]}" aria-pressed="${!!on}">${m[0]}</button>`;
};
export const meter = (pk, cls = '') => `<div class="meter ${cls}"><i class="pk" style="height:${pk}%"></i><i class="rms" style="height:${Math.round(pk * 0.72)}%"></i></div>`;
export const grm = (v) => `<div class="meter gr"><i style="height:${v}%"></i></div>`;
export const fader = (pos, cls = '') => `<div class="fader ${cls}"><div class="cap" style="bottom:${pos}%"></div></div>`;
export const field = (v, u = '', cls = '') => `<span class="field ${cls}"><span class="v">${v}</span><span class="u">${u}</span></span>`;
export const btn = (t, cls = '', icon = '') => `<button class="btn ${cls}">${icon ? ic(icon, 'm') : ''}${t}</button>`;
export const ib = (icon, label, cls = '') => `<button class="ib ${cls}" aria-label="${label}">${ic(icon)}</button>`;
export const seg = (items, on = 0, cls = '', label = '') => `<div class="seg ${cls}" role="tablist"${label ? ` aria-label="${label}"` : ''}>${items.map((t, i) => `<button class="${i === on ? 'on' : ''}" role="tab" aria-selected="${i === on}">${t}</button>`).join('')}</div>`;
export const tag = (t, cls = '') => `<span class="tag ${cls}">${t}</span>`;
export const badge = (n) => `<span class="badge">${n}</span>`;
export const check = (on, cls = '') => `<span class="check ${on ? 'on' : ''} ${cls}" role="checkbox" aria-checked="${!!on}">${ic('check')}</span>`;
export const sel = (t, cls = '') => `<span class="select ${cls}"><span class="ell">${t}</span>${ic('chev', 'm')}</span>`;
export const flabel = (l, inner, cls = '') => `<label class="flabel ${cls}"><span class="l">${l}</span>${inner}</label>`;
export const input = (t, cls = '', ph = false) => `<span class="input ${cls}"><span class="${ph ? 'ph-tx' : ''} grow ell">${t}</span></span>`;
export const mi = (t, o = {}) => `<div class="mi ${o.cls || ''}" role="menuitem">${o.chk !== undefined ? `<span class="chk">${o.chk ? ic('check', 'm') : ''}</span>` : ''}${o.icon ? ic(o.icon, 'm') : ''}<span>${t}</span>${o.k ? `<span class="kbd">${o.k}</span>` : ''}${o.sub ? `<span class="ml-auto fx">${ic('right', 'm')}</span>` : ''}</div>`;
export const menu = (items, style = '', cls = '') => `<div class="menu ${cls}" role="menu" style="${style}">${items.map((x) => (x === '-' ? '<div class="ms"></div>' : typeof x === 'string' && x.startsWith('#') ? `<div class="mh">${x.slice(1)}</div>` : Array.isArray(x) ? mi(x[0], x[1] || {}) : x)).join('')}</div>`;
export const dialog = (o) => `<div class="scrim"><div class="dialog ${o.size || ''}" role="dialog" aria-label="${o.title}"><div class="dlg-h"><span class="h l">${o.title}</span>${o.sub ? `<span class="dim text">${o.sub}</span>` : ''}</div>${o.body ? `<div class="dlg-b">${o.body}</div>` : ''}<div class="dlg-f">${o.foot}</div></div></div>`;
export const toast = (kind, icon, title, text, action = '') => `<div class="toast ${kind}" role="status">${ic(icon)}<div class="col g1 grow"><span class="b">${title}</span><span class="dim">${text}</span></div>${action}</div>`;
export const progress = (p, cls = '') => `<div class="progress ${cls}" role="progressbar" aria-valuenow="${p}"><i style="width:${p}%"></i></div>`;
export const diffRows = (rows = DIFF, off = []) => rows.map((d, i) => `<div class="dr ${off.includes(i) ? 'is-off' : ''}"><span class="dim grow ell">${d[0]}</span><span class="was">${d[1]}</span><span class="faint">→</span><span class="to">${d[2]}</span></div>`).join('');
// The mark (A-076, design/brand/final): drawn for 20 px; `large` is the 100-grid drawing (from 24 px up); `brand`
// keeps lime and lilac instead of lighting the room of the current phase.
export const MARK_20 = '<path class="mark-mix" stroke-width="2" d="M1.75 4H4.4Q6 4 6.84 5.5L7.96 7.5Q8.8 9 10.5 9M1.75 16H4.4Q6 16 6.84 14.5L7.96 12.5Q8.8 11 10.5 11"/><path class="mark-master" d="M15 7H17.25A2 2 0 0 1 19.25 9V11A2 2 0 0 1 17.25 13H15A2 2 0 0 1 13 11V9A2 2 0 0 1 15 7Z"/>';
export const MARK_100 = '<path class="mark-mix" stroke-width="9" d="M9 20H23Q31 20 35.2 27.65L40.8 37.85Q45 45.5 53 45.5M9 80H23Q31 80 35.2 72.35L40.8 62.15Q45 54.5 53 54.5"/><path class="mark-master" d="M75 37H85A10 10 0 0 1 95 47V53A10 10 0 0 1 85 63H75A10 10 0 0 1 65 53V47A10 10 0 0 1 75 37Z"/>';
export const mark = (o = {}) => `<svg class="mark${o.brand ? ' brand' : ''}" viewBox="0 0 ${o.large ? 100 : 20} ${o.large ? 100 : 20}" aria-hidden="true">${o.large ? MARK_100 : MARK_20}</svg>`;
