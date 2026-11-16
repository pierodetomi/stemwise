// Screens 1, 2 — Welcome, New project / Import.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
const { ic, tag, btn, sel, input, flabel, check } = L;

export const bar = (title, right = '', sub = '') => `<header class="top">${L.mark()}<span class="h">${title}</span>${sub ? `<span class="faint s1">${sub}</span>` : ''}<div class="grow"></div>${right}</header>`;
const thumb = (seed) => `<span class="thumb t${(seed % 12) + 1}">${Array.from({ length: 9 }, (_, i) => `<i style="height:${20 + ((seed * 7 + i * 37) % 70)}%"></i>`).join('')}</span>`;
const RECENT = [['Paper Lanterns', '12 tracks · 3:42', 'Mix', 'Today 21:47', 3], ['Low Tide (demo 4)', '9 tracks · 4:05', 'Master', 'Yesterday', 7], ['Northbound', '16 tracks · 3:28', 'Mix', '3 Nov', 10], ['Kitchen Radio', '7 tracks · 2:51', 'Master', '28 Oct', 1], ['Slow Furniture', '14 tracks · 5:12', 'Mix', '19 Oct', 5]];

export function welcome(first) {
  const left = `<div class="hero"><div class="col g2">${L.mark({ brand: true, large: true })}<span class="h xl">Stemwise</span><p class="text dim" style="max-width:var(--dlg-s)">Mix and master the songs you have already recorded. An assistant proposes, you listen and decide.</p></div>
      <div class="fx g2">${btn('New project', 'primary lg', 'plus')}${btn('Open project…', 'lg', 'folder')}</div>
      <a href="18-tour.html" class="tourlink">${ic('play', 'sm')}<span>Take the tour</span><span class="faint">· about 3 minutes</span></a>
      <span class="faint s1">Drop a folder of stems anywhere on this window to start a project from it.</span></div>`;
  const recents = `<div class="ph"><span class="h">Recent projects</span><div class="grow"></div>${input(ic('filter', 'm') + ' Filter', 'sm', true)}</div>
      <div class="sect list">${RECENT.map((r, i) => `<div class="li ${i === 0 ? 'is-hover' : ''}">${thumb(r[4])}<div class="col grow"><span class="b ell">${r[0]}</span><span class="faint s1">${r[1]}</span></div>${tag(r[2], r[2] === 'Mix' ? 'acc' : 'master')}<span class="faint s1 nowrap" style="width:var(--sp-8);text-align:right">${r[3]}</span></div>`).join('')}</div>
      <div class="foot"><div class="fx between"><span class="faint s1">Projects are folders ending in .stemwise. Imported audio is copied inside; your originals are never touched.</span></div></div>`;
  const firstRun = `<div class="ph"><span class="h">Set up the assistant</span>${tag('Optional')}</div>
      <div class="sect g4"><p class="text dim">Stemwise asks a language model for mix and master proposals through an OpenRouter account that is yours. Setting it up takes about two minutes; you can also do it later in Settings.</p>
        <div class="steps"><div class="step" style="color:var(--tx-2)"><span class="n">1</span><span class="grow">Connect your OpenRouter account</span><span class="s1 faint">in the browser</span></div><div class="step" style="color:var(--tx-2)"><span class="n">2</span><span class="grow">Add a little credit</span><span class="s1 faint">$5 is plenty to start</span></div><div class="step" style="color:var(--tx-2)"><span class="n">3</span><span class="grow">Ask for your first mix</span></div></div>
        <div class="fx g2">${btn('Set up the assistant', 'primary lg', 'spark')}${btn('Skip for now', 'lg')}</div>
        <div class="tile row">${ic('lock', 'aic')}<div class="col"><span class="b">Your audio never leaves this Mac</span><span class="dim s1">The model receives measurements, roles, track names, your brief and your requests. Never audio, file names, paths or the song title.</span></div></div>
        <div class="hr"></div>
        <div class="tile row">${ic('spark')}<div class="col grow"><span class="b">Try the offline demo</span><span class="dim s1">A built-in rule-based assistant that needs no account and no network. Good for a first look.</span></div>${btn('Use offline demo', 'sm')}</div></div>
      <div class="foot"><span class="faint s1">No recent projects yet.</span></div>`;
  return P.app({}, `<div class="page"><div class="grow fx">${left}</div><div class="pane" style="width:var(--dlg-l);margin:var(--gap);border-radius:var(--r-3)">${first ? firstRun : recents}</div></div>
    <div class="fx g4" style="padding:0 var(--sp-5) var(--sp-3)"><span class="faint s1">Version 1.0</span><a href="11-settings.html" class="s1">Settings</a><span class="grow"></span>${first ? '' : tag(ic('check', 'sm') + 'Assistant ready · openai/gpt-6.1-sol', 'ok')}</div>`);
}

const FILES = [['01 Kick In.wav', 'WAV · 24-bit', '48 kHz', 'Mono', '3:42', 'kick'], ['02 Snare Top.wav', 'WAV · 24-bit', '48 kHz', 'Mono', '3:42', 'snare'], ['03 OH.wav', 'WAV · 24-bit', '48 kHz', 'Stereo', '3:42', 'overheads'],
  ['04 Room.wav', 'WAV · 24-bit', '48 kHz', 'Stereo', '3:42', 'room'], ['05 Bass DI.wav', 'WAV · 24-bit', '48 kHz', 'Mono', '3:42', 'bass'], ['06 Acoustic.flac', 'FLAC · 24-bit', '48 kHz', 'Stereo', '3:42', 'acoustic gtr'],
  ['07 Gtr L.wav', 'WAV · 24-bit', '48 kHz', 'Mono', '3:42', 'electric gtr'], ['08 Gtr R.wav', 'WAV · 24-bit', '48 kHz', 'Mono', '3:42', 'electric gtr'], ['09 Rhodes.aif', 'AIFF · 24-bit', '48 kHz', 'Stereo', '3:42', 'keys'],
  ['10 Strings.wav', 'WAV · 16-bit', '44.1 kHz', 'Stereo', '3:42', 'strings', 'warn'], ['11 Lead Vox.wav', 'WAV · float 32', '48 kHz', 'Mono', '3:42', 'lead vocal'], ['12 take3 final.wav', 'WAV · 24-bit', '48 kHz', 'Stereo', '3:42', 'other', 'guess']];
function importScreen(state) {
  const filled = state !== 'empty';
  const table = `<div class="tbl files"><span class="th"></span><span class="th">File</span><span class="th">Format</span><span class="th">Rate</span><span class="th">Channels</span><span class="th">Length</span><span class="th">Role</span><span class="th"></span>
      ${FILES.map((f, i) => `<span class="t${i + 1}"><i class="tdot"></i></span><span class="b ell">${f[0]}</span><span class="dim">${f[1]}</span><span class="${f[6] === 'warn' ? 'warnc b' : 'dim'}">${f[2]}${f[6] === 'warn' ? ' ' + ic('warn', 'sm') : ''}</span><span class="dim">${f[3]}</span><span class="dim">${f[4]}</span><span>${sel(f[5], f[6] === 'guess' ? 'is-open' : '')}</span><span>${L.ib('x', 'Remove file')}</span>`).join('')}</div>`;
  const main = filled
    ? `<div class="fx between"><div class="fx g2"><span class="b">12 files</span><span class="faint s1">One track per file, each clip starts at zero. Roles are guessed from the file names: check them.</span></div>${btn('Add files…', 'sm', 'plus')}</div>
       <div class="banner">${ic('warn')}<span class="grow">“10 Strings.wav” is 44.1 kHz; the project is 48 kHz (set by the first file). It will not be resampled silently.</span>${btn('Convert a copy', 'sm')}${btn('Remove', 'quiet sm')}</div>${table}`
    : `<div class="drop ${state === 'over' ? 'is-over' : ''} grow">${ic('wave')}<span class="h">${state === 'over' ? 'Drop to add 12 files' : 'Drop your stems here'}</span><span class="dim">WAV, FLAC or AIFF · mono or stereo · one file per track</span>${state === 'over' ? '' : btn('Choose files…', 'lg', 'folder')}</div>`;
  const side = `<div class="ph"><span class="h">Brief</span><span class="faint s1">What the assistant should aim for</span></div>
      <div class="sect g3">${flabel('Genre', input(filled ? 'Indie rock' : 'e.g. indie rock', 'sm', !filled))}${flabel('Mood', input(filled ? 'Warm, intimate, a little dirty' : 'e.g. warm, intimate', 'sm', !filled))}${flabel('Notes', input(filled ? 'Vocal up front. Dry drums. Guitars wide but never harsh.' : 'Anything that matters to you', 'area', !filled))}</div>
      <div class="ph"><span class="h">References</span><span class="faint s1">${filled ? '2 of 3' : '0 of 3'} · optional</span></div>
      <div class="sect g2">${filled ? [['Reference A', 'MP3 · 3:51'], ['Reference B', 'FLAC · 4:07']].map((r) => `<div class="tile row">${ic('music')}<div class="col grow"><span class="b ell">${r[0]}</span><span class="faint s1">${r[1]}</span></div>${L.ib('x', 'Remove reference')}</div>`).join('') : ''}
        <div class="drop" style="height:var(--sp-8)"><span class="dim s1">${ic('plus', 'sm')} Drop a finished song you like · WAV, FLAC, AIFF, MP3, AAC</span></div>
        <span class="faint s1">References are copied into the project and used only for analysis and comparison by ear. They are never sent anywhere.</span></div>`;
  const dlg = state === 'rate' ? L.dialog({ size: 's', title: 'Different sample rate', sub: '“10 Strings.wav” is 44.1 kHz. This project runs at 48 kHz.',
    body: `<div class="tile row on">${check(true, 'radio')}<div class="col"><span class="b">Convert a copy to 48 kHz</span><span class="dim s1">High-quality conversion, saved inside the project. Your original file is not changed.</span></div></div><div class="tile row">${check(false, 'radio')}<div class="col"><span class="b">Leave this file out</span><span class="dim s1">Import the other 11 files.</span></div></div>`,
    foot: `${btn('Cancel import', 'quiet')}${btn('Continue', 'primary')}` }) : '';
  return P.app({}, `${bar('New project', `${btn('Cancel', 'quiet')}${btn('Create project', 'primary ' + (filled ? '' : 'is-dis'))}`)}
    <div class="body"><section class="card grow"><div class="ph">${flabel('', input(filled ? 'Paper Lanterns' : 'Project name', 'sm', !filled), 'grow')}<span class="faint s1 nowrap">${filled ? '48 kHz · set by the first file' : 'Sample rate is set by the first file'}</span></div><div class="sect grow g3">${main}</div></section>
      <aside class="card" style="width:var(--dlg-s)">${side}</aside></div>${dlg}`);
}

export default {
  'screens/01-welcome.html': P.page('1 · Welcome', 'Start screen: recent projects, new, open. On first launch the right pane invites to set up the assistant and can be skipped.', [
    P.frame('1a', 'Welcome', 'Recent projects with phase and last opened; the first row is hovered.', welcome(false)),
    P.frame('1b', 'First launch', 'No projects yet. The assistant setup is optional: the three steps ahead, a button that opens the setup (14a), the privacy statement, skip, or the offline demo.', welcome(true)),
  ]),
  'screens/02-new-project-import.html': P.page('2 · New project / Import', 'Drop area, file list with proposed roles, sample-rate warning, brief and references.', [
    P.frame('2a', 'Empty', 'Nothing imported yet. “Create project” is disabled until there is at least one file.', importScreen('empty')),
    P.frame('2b', 'Dragging files over', 'The drop area takes the accent while files are over the window.', importScreen('over')),
    P.frame('2c', 'Files listed', 'One row per file. A role guessed with low confidence stays highlighted until confirmed; the file at a different rate carries a warning and a banner.', importScreen('files')),
    P.frame('2d', 'Sample-rate choice', 'Asked once per mismatching file when creating the project.', importScreen('rate')),
  ]),
};
