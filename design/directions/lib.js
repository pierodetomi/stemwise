/* Stemwise design directions — shared sample data and drawing helpers (design-time only). */
window.SW = (function () {
  const T = 222; // song length, seconds
  const tracks = [
    { n: 'Kick', r: 'kick', g: 'Drums', lv: 0.72, db: -6.5, pan: 0, k: 'p' },
    { n: 'Snare', r: 'snare', g: 'Drums', lv: 0.66, db: -8.0, pan: 0, k: 'p' },
    { n: 'Overheads', r: 'overheads', g: 'Drums', lv: 0.55, db: -11.5, pan: 0, k: 'p' },
    { n: 'Room', r: 'room', g: 'Drums', lv: 0.4, db: -16.0, pan: 0, k: 'p' },
    { n: 'Bass DI', r: 'bass', lv: 0.7, db: -7.2, pan: 0, k: 's' },
    { n: 'Acoustic', r: 'acoustic gtr', g: 'Guitars', lv: 0.5, db: -12.4, pan: -0.3, k: 's' },
    { n: 'Gtr Left', r: 'electric gtr', g: 'Guitars', lv: 0.58, db: -10.1, pan: -0.8, k: 's' },
    { n: 'Gtr Right', r: 'electric gtr', g: 'Guitars', lv: 0.57, db: -10.3, pan: 0.8, k: 's' },
    { n: 'Rhodes', r: 'keys', lv: 0.45, db: -14.0, pan: 0.2, k: 's' },
    { n: 'Strings', r: 'strings', lv: 0.42, db: -15.5, pan: 0, k: 's' },
    { n: 'Lead Vox', r: 'lead vocal', lv: 0.75, db: -5.8, pan: 0, k: 'v', sel: 1, ai: 1 },
    { n: 'BVs', r: 'backing vox', lv: 0.48, db: -13.0, pan: 0, k: 'v', lock: 1 },
  ];
  const clipTimes = [
    [[14, 144], [168, 214]], [[16, 144], [168, 214]], [[0, 144], [166, 222]], [[0, 144], [166, 222]],
    [[16, 112], [112, 144], [168, 216]], [[0, 222]], [[48, 80], [112, 144], [168, 208]],
    [[48, 80], [112, 144], [168, 208]], [[0, 48], [80, 112], [144, 168], [208, 222]],
    [[80, 144], [144, 208]], [[16, 46], [48, 80], [82, 110], [112, 144], [146, 166], [168, 208]],
    [[48, 80], [112, 144], [168, 208]],
  ];
  const buses = [
    { n: 'Drums', t: 'group', lv: 0.7, db: -4.0 }, { n: 'Guitars', t: 'group', lv: 0.6, db: -6.5 },
    { n: 'Plate', t: 'reverb', lv: 0.35, db: -12.0 }, { n: 'Echo', t: 'delay', lv: 0.25, db: -15.0 },
    { n: 'Mix', t: 'mix bus', lv: 0.8, db: 0.0 },
  ];
  const sections = [
    ['Intro', 0, 16, 0.45], ['Verse 1', 16, 48, 0.65], ['Chorus', 48, 80, 1], ['Verse 2', 80, 112, 0.68],
    ['Chorus', 112, 144, 1], ['Bridge', 144, 168, 0.55], ['Chorus', 168, 208, 1], ['Outro', 208, 222, 0.4],
  ];
  const playhead = 62.48, loop = [48, 80];

  function rng(a) { return function () { a |= 0; a = (a + 0x6d2b79f5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
  const inten = (t) => { for (const s of sections) if (t >= s[1] && t < s[2]) return s[3]; return 0.4; };
  const f1 = (v) => v.toFixed(1);

  /* Waveform of one clip. Returns { fill: closed envelope path, bars: vertical strokes path }. */
  function wave(ti, c0, c1, pps, h, step) {
    step = step || 2;
    const r = rng(ti * 97 + Math.round(c0) * 13 + 5), k = tracks[ti] ? tracks[ti].k : 'm';
    const w = (c1 - c0) * pps, mid = h / 2, max = mid - 2;
    let v = 0.5; const up = [], dn = []; let bars = '';
    for (let x = 0; x <= w; x += step) {
      const t = c0 + x / pps; let a;
      if (k === 'p') a = 0.22 + 0.78 * Math.pow(r(), 2.2);
      else { v = v * 0.62 + r() * 0.38; a = 0.35 + 0.65 * v; }
      if (k === 'v' && Math.sin(t * 0.83 + ti) < -0.45) a *= 0.12;
      if (k === 'm') a = 0.5 + 0.5 * a;
      a *= inten(t) * Math.min(1, x / 4, (w - x) / 4 + 0.15);
      const px = Math.max(0.6, a * max);
      up.push(f1(x) + ' ' + f1(mid - px)); dn.push(f1(x) + ' ' + f1(mid + px));
      bars += 'M' + f1(x + 0.5) + ' ' + f1(mid - px) + 'V' + f1(mid + px);
    }
    return { w, fill: 'M' + up.join('L') + 'L' + dn.reverse().join('L') + 'Z', bars };
  }
  function clips(ti, pps, h, step) {
    return clipTimes[ti].map(([a, b]) => Object.assign({ x: a * pps, t0: a, t1: b }, wave(ti, a, b, pps, h, step)));
  }
  function mixdown(w, h, step) { return wave(99, 0, T, w / T, h, step); }

  /* EQ response sketch (visual only). bands: {f, g, q, t:'peak'|'ls'|'hs'}; hpf in Hz. */
  const fx = (f, w) => (Math.log(f / 20) / Math.log(1000)) * w;
  function eq(w, h, bands, hpf, range) {
    range = range || 12; const mid = h / 2, pts = [];
    for (let x = 0; x <= w; x += 2) {
      const f = 20 * Math.pow(1000, x / w); let d = 0;
      for (const b of bands) {
        const o = Math.log2(f / b.f);
        if (b.t === 'ls') d += b.g / (1 + Math.exp(o * 3));
        else if (b.t === 'hs') d += b.g / (1 + Math.exp(-o * 3));
        else d += b.g * Math.exp(-o * o * b.q * 1.6);
      }
      if (hpf && f < hpf * 2) d -= 12 * Math.pow(Math.max(0, Math.log2((hpf * 2) / f)), 1.5) * 0.9;
      pts.push([x, Math.max(1, Math.min(h - 1, mid - (d / range) * mid))]);
    }
    const line = 'M' + pts.map((p) => f1(p[0]) + ' ' + f1(p[1])).join('L');
    const handles = bands.map((b) => ({ x: fx(b.f, w), y: mid - (b.g / range) * mid, b }));
    return { line, fill: line + 'L' + w + ' ' + mid + 'L0 ' + mid + 'Z', handles, mid, hpfX: hpf ? fx(hpf, w) : 0 };
  }
  function spectrum(w, h, seed, tilt) {
    const r = rng(seed); let v = 0.5; const pts = [];
    for (let x = 0; x <= w; x += 3) {
      v = v * 0.7 + r() * 0.3; const u = x / w;
      const env = Math.sin(Math.min(1, u * 2.4) * Math.PI / 2) * (1 - (tilt || 0.55) * u) * (u > 0.92 ? (1 - u) / 0.08 : 1);
      pts.push(f1(x) + ' ' + f1(h - env * (0.45 + 0.4 * v) * h));
    }
    return 'M0 ' + h + 'L' + pts.join('L') + 'L' + w + ' ' + h + 'Z';
  }
  const eqTicks = (w) => [50, 100, 200, 500, 1000, 2000, 5000, 10000].map((f) => ({ x: fx(f, w), l: f >= 1000 ? f / 1000 + 'k' : '' + f }));
  const voxEq = [{ f: 220, g: -2.5, q: 1.2, t: 'peak' }, { f: 3200, g: 2.0, q: 0.9, t: 'peak' }, { f: 6500, g: -1.5, q: 3, t: 'peak' }, { f: 11000, g: 2.5, q: 1, t: 'hs' }];

  /* Automation (Lead Vox fader, written by the AI per section with ramps). */
  const autoVals = [0.5, 0.5, 0.74, 0.5, 0.74, 0.4, 0.78, 0.45];
  function auto(pps, h) {
    const pts = []; const y = (v) => h - 4 - v * (h - 8);
    sections.forEach((s, i) => { pts.push([s[1] * pps + (i ? 1.5 : 0), y(autoVals[i])]); pts.push([s[2] * pps - 1.5, y(autoVals[i])]); });
    const line = 'M' + pts.map((p) => f1(p[0]) + ' ' + f1(p[1])).join('L');
    return { pts, line, fill: line + 'L' + f1(T * pps) + ' ' + h + 'L0 ' + h + 'Z' };
  }

  /* Knob: 270° arc. v in 0..1; bipolar draws from centre. Styled through .k-bg .k-val .k-ptr. */
  function knob(size, v, bipolar, sw) {
    sw = sw || 3; const c = size / 2, r = c - sw / 2 - 1;
    const P = (a) => [c + r * Math.cos(a), c + r * Math.sin(a)];
    const a0 = Math.PI * 0.75, a1 = Math.PI * 2.25, av = a0 + (a1 - a0) * v, am = (a0 + a1) / 2;
    const arc = (s, e) => { if (e < s) [s, e] = [e, s]; const p = P(s), q = P(e); return 'M' + f1(p[0]) + ' ' + f1(p[1]) + 'A' + r + ' ' + r + ' 0 ' + (e - s > Math.PI ? 1 : 0) + ' 1 ' + f1(q[0]) + ' ' + f1(q[1]); };
    const pi = [c + (r - sw - 3) * Math.cos(av), c + (r - sw - 3) * Math.sin(av)], po = [c + (r * 0.25) * Math.cos(av), c + (r * 0.25) * Math.sin(av)];
    return '<svg class="knob" width="' + size + '" height="' + size + '" viewBox="0 0 ' + size + ' ' + size + '" fill="none"><circle class="k-body" cx="' + c + '" cy="' + c + '" r="' + (r - sw) + '"/><path class="k-bg" stroke-width="' + sw + '" d="' + arc(a0, a1) + '"/><path class="k-val" stroke-width="' + sw + '" d="' + arc(bipolar ? am : a0, av) + '"/><path class="k-ptr" d="M' + f1(po[0]) + ' ' + f1(po[1]) + 'L' + f1(pi[0]) + ' ' + f1(pi[1]) + '"/></svg>';
  }

  const I = (d, x) => '<svg class="ic" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" ' + (x || '') + '>' + d + '</svg>';
  const ic = {
    play: I('<path d="M5 3.5v9l7.5-4.5z" fill="currentColor"/>'), stop: I('<rect x="4" y="4" width="8" height="8" rx="1" fill="currentColor"/>'),
    rew: I('<path d="M4 3.5v9M12.5 3.5v9L6 8z" fill="currentColor"/>'), loop: I('<path d="M11 2.5l2 2-2 2M13 4.5H5.5a3 3 0 0 0-3 3M5 13.5l-2-2 2-2M3 11.5h7.500a3 3 0 0 0 3-3"/>'),
    undo: I('<path d="M5.500 3L2.500 6l3 3M2.500 6h7a3.500 3.500 0 0 1 0 7H6"/>'), redo: I('<path d="M10.500 3l3 3-3 3M13.500 6h-7a3.500 3.500 0 0 0 0 7H10"/>'),
    lock: I('<rect x="3.500" y="7" width="9" height="6.500" rx="1.500"/><path d="M5.500 7V5a2.500 2.500 0 0 1 5 0v2"/>'), unlock: I('<rect x="3.500" y="7" width="9" height="6.500" rx="1.500"/><path d="M5.500 7V5a2.500 2.500 0 0 1 4.800-1"/>'),
    spark: I('<path d="M8 1.500l1.600 4.900 4.900 1.600-4.900 1.600L8 14.500 6.400 9.600 1.500 8l4.900-1.600z"/>'), chev: I('<path d="M4 6l4 4 4-4"/>'), right: I('<path d="M6 4l4 4-4 4"/>'),
    plus: I('<path d="M8 3v10M3 8h10"/>'), cut: I('<circle cx="4.500" cy="11.500" r="2"/><circle cx="11.500" cy="11.500" r="2"/><path d="M5.800 10L11 2.500M10.200 10L5 2.500"/>'),
    cursor: I('<path d="M3.500 2.500l9 4.500-4 1.200-1.500 4z"/>'), pen: I('<path d="M2.500 13.500l1-3.500 7.500-7.500 2.500 2.500L6 12.500z"/>'), send: I('<path d="M8 13V3M3.500 7.500L8 3l4.500 4.500"/>'),
    ear: I('<path d="M2.500 9V8a5.500 5.500 0 0 1 11 0v1"/><rect x="2" y="9" width="3" height="4.500" rx="1"/><rect x="11" y="9" width="3" height="4.500" rx="1"/>'), check: I('<path d="M3 8.500l3.200 3L13 4.500"/>'),
    clock: I('<circle cx="8" cy="8" r="5.500"/><path d="M8 5v3l2 1.500"/>'), snap: I('<path d="M4 2.500v6a4 4 0 0 0 8 0v-6M4 5h2.500M9.500 5H12"/>'),
  };
  const fmt = (s) => Math.floor(s / 60) + ':' + String(Math.floor(s % 60)).padStart(2, '0');
  const db = (v) => (v > 0 ? '+' : v < 0 ? '−' : '') + Math.abs(v).toFixed(1);
  const hues = (L, C, start, n) => Array.from({ length: n || 12 }, (_, i) => 'oklch(' + L + ' ' + C + ' ' + ((start + i * (360 / (n || 12))) % 360) + ')');
  const rationale = 'Lead vocal sat 2.1 LU under the reference vocal-to-mix balance and was masked by the guitars around 2–4 kHz in the choruses. Raised it 1.8 dB, cut some 220 Hz mud, added presence at 3.2 kHz, and lifted it a further 1.5 dB in the choruses only.';
  return { T, tracks, buses, sections, playhead, loop, clips, clipTimes, wave, mixdown, eq, spectrum, eqTicks, voxEq, auto, autoVals, knob, ic, fmt, db, hues, rng, rationale, dirs: [] };
})();
