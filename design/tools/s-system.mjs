// Screens 11, 12, 13 — Settings, AI usage, cross-cutting states.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
import { bar } from './s-start.mjs';
const { ic, tag, btn, seg, field, flabel, input, sel, check, menu, ib, toast, progress } = L;

// ---------- 11 · settings ----------
const frow = (l, hint, inner) => `<div class="frow"><div class="fl"><span class="b">${l}</span><span class="faint s1">${hint}</span></div><div class="col g2">${inner}</div></div>`;
const sw = (on, t) => `<span class="fx g2"><span class="switch ${on ? 'on' : ''}" role="switch" aria-checked="${on}"></span><span>${t}</span></span>`;
const KEYS = [['Transport', [['Play / pause', 'Space'], ['Stop and return', '↩'], ['Loop on / off', 'L'], ['Set loop to selection', '⌘L'], ['Follow playhead', 'F']]], ['Editing', [['Split at playhead', '⌘T'], ['Duplicate', '⌘D'], ['Delete', '⌫'], ['Mute clip', 'M'], ['Clip gain', '⌘G'], ['Snap on / off', 'N']]],
  ['Tools', [['Select', 'V'], ['Split', 'C'], ['Automation', 'A']]], ['History', [['Undo', '⌘Z'], ['Redo', '⇧⌘Z'], ['Save snapshot', '⌘S'], ['A/B', 'B']]], ['View', [['Zoom in / out', '⌘+ / ⌘−'], ['Zoom to fit', '⌘0'], ['Taller / shorter lanes', '⌥⌘+ / ⌥⌘−'], ['Show / hide mixer', '⌘M'], ['Show / hide inspector', '⌘I'], ['Show / hide assistant', '⌘J']]],
  ['Tracks', [['Mute track', '⇧M'], ['Solo track', 'S'], ['Lock for AI', '⇧L'], ['Group selected', '⇧⌘G'], ['Show automation', 'U']]]];
const PANES = {
  Audio: () => frow('Output device', 'Where you hear the project.', sel('Scarlett 2i2 USB', 'lg') + `<span class="faint s1">${ic('check', 'sm')} Device runs at 48 kHz, same as the project: no conversion on output.</span>`)
    + frow('Buffer size', 'Smaller is more responsive, larger is safer.', seg(['128', '256', '512', '1024'], 2, 'quiet', 'Buffer') + '<span class="faint s1">512 samples · 10.7 ms at 48 kHz</span>')
    + frow('Engine load', 'Share of each buffer the engine uses.', `<div class="fx g3"><div class="hbar grow"><i style="width:18%"></i></div><span class="b">18 %</span></div><span class="faint s1">Above 80 % you may hear dropouts: raise the buffer size.</span>`)
    + frow('Test', 'Check that sound reaches your speakers.', `<div class="fx g2">${btn('Play test tone', '', 'speaker')}</div>`),
  AI: () => frow('Assistant', 'Who answers mix and master requests.', seg(['OpenRouter (your key)', 'Offline demo'], 0, 'quiet', 'Decider') + '<span class="faint s1">The offline demo is rule-based: no key, no network, no cost.</span>')
    + frow('OpenRouter account', 'The key is kept in the macOS keychain, never in the project.', `<div class="fx g2"><span class="input sm grow"><span class="grow ell">Connected · key “Stemwise” · sk-or-••••3f9a</span></span>${btn('Connect again', '', 'link')}${btn('Disconnect', 'quiet')}</div><span class="okc s1">${ic('check', 'sm')} Key works · checked today 21:30</span><div class="fx g4"><a class="s1">Add credit on OpenRouter</a><a class="s1">Paste a key instead</a></div>`)
    + frow('Model', 'Any model id offered by OpenRouter.', input('openai/gpt-6.1-sol', 'sm') + `<div class="fx g4">${flabel('Reasoning effort', seg(['Low', 'Medium', 'High'], 1, 'quiet', 'Effort'))}${flabel('Temperature', field('0.2', ''))}</div>`)
    + frow('Spending cap', 'Ask before a project goes over this.', `<div class="fx g3">${sw(true, 'Ask me above')}${field('2.00', '$')}<span class="faint s1">per project</span></div>`)
    + frow('What is sent', 'Always visible, per call, in AI usage.', `<div class="tile row">${ic('lock', 'aic')}<span class="dim s1">Measurements, roles, track names, your brief and your requests. Never audio, file names, paths or the song title.</span></div>`)
    + frow('All projects', 'Total recorded on this Mac.', `<div class="fx g2 stretch"><div class="stat"><span class="faint s1">Cost</span><span class="v">$3.84</span></div><div class="stat"><span class="faint s1">Calls</span><span class="v">61</span></div><div class="stat"><span class="faint s1">Projects</span><span class="v">5</span></div></div>`),
  Appearance: () => frow('Track height', 'Default for new projects.', seg(['Small', 'Default', 'Tall'], 1, 'quiet', 'Track height'))
    + frow('Clips', 'What a clip shows.', sw(true, 'Name strip on clips') + sw(true, 'Waveforms') + sw(false, 'Clip gain badge always visible'))
    + frow('Time', 'Ruler and position display.', seg(['Minutes : seconds', 'Bars · beats'], 0, 'quiet', 'Time format') + '<span class="faint s1">Bars need a tempo and meter in the project.</span>')
    + frow('Meters', 'How meters behave.', sw(true, 'Hold peaks for 2 seconds') + sw(false, 'Show numeric peak above each strip'))
    + frow('Motion', 'Panel and phase transitions.', sw(false, 'Reduce motion'))
    + frow('Track colours', 'Assigned in order; change any from the track menu.', `<div class="fx g1">${Array.from({ length: 12 }, (_, i) => `<span class="swt t${i + 1}" style="background:var(--tc)"></span>`).join('')}</div>`),
  Shortcuts: () => `<div class="fx between">${input(ic('filter', 'm') + ' Filter shortcuts', 'sm', true)}<span class="faint s1">Shortcuts can be viewed here; changing them is not part of this version.</span></div>
    <div class="sheet c3" style="padding:0">${KEYS.map((g) => `<div class="col g1"><span class="b">${g[0]}</span>${g[1].map((k) => `<div class="fx between" style="height:var(--ctl-h)"><span class="dim">${k[0]}</span><span class="kbd">${k[1]}</span></div>`).join('')}</div>`).join('')}</div>`,
};
const settings = (name) => P.app({}, `${bar('Settings', btn('Done', 'primary'))}<div class="body"><nav class="card nav" aria-label="Settings sections">${['Audio', 'AI', 'Appearance', 'Shortcuts'].map((n, i) => `<div class="mi ${n === name ? 'on' : ''}">${ic(['speaker', 'spark', 'eye', 'key'][i], 'm')}<span>${n}</span></div>`).join('')}</nav><section class="card grow"><div class="ph"><span class="h">${name}</span></div><div class="form">${PANES[name]()}</div></section></div>`);

// ---------- 12 · AI usage ----------
const CALLS = [['21:55', 'Master', 'gpt-6.1-sol · medium', '1', '6.1k', '$0.02', '14 s', ['Applied', 'ok']], ['21:44', 'Auto mix', 'gpt-6.1-sol · medium', '2 passes', '31.4k', '$0.11', '1 m 06 s', ['Applied', 'ok']], ['21:41', 'Auto mix', 'gpt-6.1-sol · medium', '1 pass', '9.8k', '$0.07', '42 s', ['Cancelled', '']],
  ['21:20', 'Revision', 'gpt-6.1-sol · medium', '1', '5.1k', '$0.04', '11 s', ['Applied in part', 'acc']], ['21:12', 'Revision', 'gpt-6.1-sol · high', '3 tries', '14.9k', '$0.09', '38 s', ['Failed', 'err']], ['20:58', 'Auto mix', 'gpt-6.1-sol · medium', '3 passes', '36.2k', '$0.13', '1 m 31 s', ['Discarded', 'warn']],
  ['20:40', 'Revision', 'claude-sonnet-5.5 · medium', '1', '4.3k', '$0.02', '9 s', ['Applied', 'ok']]];
const callRow = (c, i, selIdx) => `<div class="call ${i === selIdx ? 'is-sel' : i === 1 && selIdx === undefined ? 'is-hover' : ''}"><span class="faint">${c[0]}</span><span class="b">${c[1]}</span><span class="dim ell">${c[2]}</span><span class="dim">${c[3]}</span><span class="r">${c[4]}</span><span class="r b">${c[5]}</span><span class="r dim">${c[6]}</span><span>${tag(c[7][0], c[7][1])}</span>${ic('right', 'm')}</div>`;
const callList = (selIdx) => `<div class="call head"><span>Time</span><span>Type</span><span>Model · effort</span><span>Tries</span><span class="r">Tokens</span><span class="r">Cost</span><span class="r">Time</span><span>Outcome</span><span></span></div>${CALLS.map((c, i) => callRow(c, i, selIdx)).join('')}`;
const usageTop = (extra = '') => bar('AI usage', `${extra}${btn('Back to Mix', '', 'x')}`, 'Paper Lanterns');
const summary = `<div class="fx g2 stretch"><div class="stat"><span class="faint s1">Calls</span><span class="v">7</span><span class="faint s1">4 applied · 1 failed</span></div><div class="stat"><span class="faint s1">Tokens in</span><span class="v">96.4k</span></div><div class="stat"><span class="faint s1">Tokens out</span><span class="v">11.4k</span></div><div class="stat"><span class="faint s1">Cost</span><span class="v">$0.48</span><span class="faint s1">cap $2.00 · 1 estimate</span></div><div class="stat"><span class="faint s1">Time</span><span class="v">4 m 31 s</span></div></div>`;
const breakdown = `<div class="fx g2 stretch"><div class="tile grow"><span class="b">By type</span><div class="split"><i style="flex:31"></i><i style="flex:15"></i><i style="flex:2"></i></div>
      <div class="mtable" style="grid-template-columns:1fr auto auto auto"><span class="th">Type</span><span class="th r">Calls</span><span class="th r">Tokens</span><span class="th r">Cost</span>${[['Auto mix', '3', '77.4k', '$0.31'], ['Revision', '3', '24.3k', '$0.15'], ['Master', '1', '6.1k', '$0.02']].map((r) => `<span>${r[0]}</span><span class="r dim">${r[1]}</span><span class="r dim">${r[2]}</span><span class="r b">${r[3]}</span>`).join('')}</div></div>
    <div class="tile grow"><span class="b">By model</span><div class="split"><i style="flex:46"></i><i style="flex:2"></i></div>
      <div class="mtable" style="grid-template-columns:1fr auto auto auto"><span class="th">Model</span><span class="th r">Calls</span><span class="th r">Tokens</span><span class="th r">Cost</span>${[['openai/gpt-6.1-sol', '6', '103.5k', '$0.46'], ['anthropic/claude-sonnet-5.5', '1', '4.3k', '$0.02']].map((r) => `<span class="ell">${r[0]}</span><span class="r dim">${r[1]}</span><span class="r dim">${r[2]}</span><span class="r b">${r[3]}</span>`).join('')}<span class="faint s1" style="grid-column:1/-1;border:0">Costs are the ones the provider reports. An estimate from the price list is marked “est.”</span></div></div></div>`;
const filters = (open) => `<div class="fx g2">${sel('All types')}${sel('All outcomes')}${sel('All models')}${input(ic('filter', 'm') + ' Search requests', 'sm grow', true)}<div style="position:relative">${btn('Export', open ? 'is-active' : '', 'exp')}${open ? menu([['History as CSV', { cls: 'is-hover' }], ['History as JSON'], '-', ['Copy summary']], 'right:0;top:calc(var(--ctl-h) + var(--sp-1))') : ''}</div></div>`;
const usage = () => P.app({}, `${usageTop()}<div class="body"><section class="card grow"><div class="sect g3" style="padding-top:var(--sp-3)">${summary}${breakdown}<div class="fx between"><span class="h">Calls</span><span class="faint s1">Newest first · failed and cancelled calls count with what they spent</span></div>${filters(true)}<div class="col">${callList()}</div></div></section></div>`);
const measures = `<div class="mtable" style="grid-template-columns:1.4fr 1fr repeat(5,auto)"><span class="th">Track</span><span class="th">Role</span><span class="th r">LUFS</span><span class="th r">Peak</span><span class="th r">Crest</span><span class="th r">Low %</span><span class="th r">Active</span>
  ${[['t1 · Kick', 'kick', '−19.8', '−4.1', '15.7', '62', '58 %'], ['t5 · Bass DI', 'bass', '−18.2', '−6.0', '12.2', '71', '82 %'], ['t7 · Gtr Left', 'electric gtr', '−22.4', '−7.3', '15.1', '18', '46 %'], ['t11 · Lead Vox', 'lead vocal', '−20.9', '−5.2', '15.7', '9', '71 %'], ['t12 · BVs', 'backing vox · locked', '−26.0', '−9.4', '16.6', '7', '44 %']].map((r) => `<span class="b ell">${r[0]}</span><span class="dim ell">${r[1]}</span>${r.slice(2).map((v) => `<span class="r">${v}</span>`).join('')}`).join('')}<span class="faint s1" style="grid-column:1/-1;border:0">12 tracks, 8 sections, 2 references · 4,870 tokens of measurements · ids are neutral (t1, s1), names are the track names</span></div>`;
const RAW = `{
  "version": 2,
  "iteration": 1,
  "summary": "Vocal a touch warmer in the verses.",
  "tracks": {
    "t11": {
      "gain_db": -5.8, "pan": 0.0, "hpf_hz": 90, "lpf_hz": null,
      "eq": [
        { "type": "peak", "freq_hz": 220, "gain_db": -1.5, "q": 1.2 },
        { "type": "peak", "freq_hz": 3200, "gain_db": 2.0, "q": 0.9 }
      ],
      "sections": { "s2": { "eq_gain_offsets_db": [1.0, -0.5] } },
      "rationale": "Less cut at 220 Hz and a softer 3.2 kHz in the verses."
    }
  }
}`;
const detail = (raw) => P.app({}, `${usageTop()}<div class="body"><section class="card" style="width:var(--dlg-m)"><div class="ph"><span class="h">Calls</span><div class="grow"></div>${sel('All types')}</div><div class="sect">${CALLS.map((c, i) => `<div class="li ${i === 3 ? 'is-sel' : ''}" style="height:var(--sp-7)"><div class="col grow"><span class="b">${c[1]} · ${c[0]}</span><span class="faint s1 ell">${c[2]} · ${c[4]} tokens · ${c[6]}</span></div><span class="b">${c[5]}</span>${tag(c[7][0], c[7][1])}</div>`).join('')}</div></section>
    <section class="card grow"><div class="ph"><span class="h">Revision · 21:20</span>${tag('Applied in part', 'acc')}<span class="faint s1">gpt-6.1-sol · medium · 1 try · 4.6k in, 0.5k out · $0.04 · 11 s</span><div class="grow"></div>${seg(['Readable', 'Raw JSON'], raw ? 1 : 0, 'quiet sm')}${btn('History step', 'quiet sm', 'clock')}</div>
      ${raw ? `<div class="sect grow"><div class="fx between"><span class="b">Response · mix_plan</span><div class="fx g2">${seg(['Request', 'Response'], 1, 'quiet sm')}${btn('Copy', 'sm', 'copy')}</div></div><div class="json grow">${RAW}</div><span class="faint s1">Exactly what was received. The request shows exactly what was sent: no audio, no file names, no paths, no song title.</span></div>`
    : `<div class="fx g4 top-al grow" style="padding:0 var(--sp-3) var(--sp-3)"><div class="col g3 grow"><span class="b">What was sent</span><div class="tile"><span class="faint s1">Your request · scope: Lead Vox</span><span class="text">“Vocal a touch warmer in the verses.”</span></div>
          <div class="tile"><span class="faint s1">Brief</span><span>Indie rock · warm, intimate, a little dirty</span><span class="dim">Vocal up front. Dry drums. Guitars wide but never harsh.</span></div>${measures}</div>
        <div class="col g3" style="width:var(--dlg-s)"><span class="b">What came back</span><div class="tile"><span class="faint s1">Summary</span><span class="text">Vocal a touch warmer in the verses.</span></div>
          <div class="pcard t11"><div class="fx g2">${check(true)}<i class="tdot"></i><span class="b grow">Lead Vox</span>${tag('applied', 'ok')}</div><p class="why dim">Less cut at 220 Hz and a softer 3.2 kHz in the verses.</p><div class="col">${L.diffRows([['EQ 1 · 220 Hz', '−2.5', '−1.5 dB'], ['Verse · EQ 2 offset', '0.0', '−0.5 dB'], ['Verse · EQ 1 offset', '0.0', '+1.0 dB']])}</div></div>
          <div class="pcard t10 is-off"><div class="fx g2">${check(false)}<i class="tdot"></i><span class="b grow">Strings</span>${tag('not applied')}</div><p class="why dim">Down 1 dB in the verses to leave room.</p></div>
          <span class="faint s1">Validation: plan accepted at the first try. Limits and locks respected.</span></div></div>`}</section></div>`);
const usageEmpty = () => P.app({}, `${usageTop()}<div class="body"><section class="card grow"><div class="empty grow">${ic('spark')}<span class="h">No AI calls in this project yet</span><span style="max-width:var(--dlg-s)">When you run Auto mix, ask for a change in words or master with AI, every call is listed here with what was sent, what came back, tokens and cost.</span>${btn('Back to Mix', 'lg')}</div></section></div>`);
const capDialog = L.dialog({ title: 'This request goes over your spending cap', sub: 'You asked to be warned above $2.00 per project.',
  body: `<div class="fx g2 stretch"><div class="stat"><span class="faint s1">Spent so far</span><span class="v">$1.93</span></div><div class="stat"><span class="faint s1">This request, estimated</span><span class="v">$0.13</span><span class="faint s1">about 36k tokens, up to 3 passes</span></div><div class="stat"><span class="faint s1">Would reach</span><span class="v warnc">$2.06</span></div></div><span class="fx g2 dim">${check(false)}Raise the cap to $3.00 for this project</span>`,
  foot: `${btn('Cancel', 'quiet')}${btn('Change cap…')}${btn('Run anyway', 'primary')}` });

// ---------- 13 · cross-cutting states ----------
const emptyMix = () => P.app({}, `${P.top({ playing: false, pos: '0:00.00', noSection: true, status: 'New project', engine: '0 %', loop: false })}<div class="body"><div class="left"><section class="card grow">${P.tools({ count: 'No tracks · rate not set' })}<div class="drop grow" style="margin:var(--sp-3)">${ic('wave')}<span class="h">Drop your stems here</span><span class="dim">WAV, FLAC or AIFF · one file per track</span>${btn('Choose files…', 'lg', 'folder')}</div></section>
    <section class="card mixer" style="flex:none;height:var(--mixer-h)"><div class="empty grow"><span class="faint">Strips appear here as you add tracks.</span></div>${P.strip({ n: 'Mix', cls: 'wide', tint: 'tm', bus: true, pan: 0.5, pk: 0, fy: 77, gr: 0, db: '0.0', s1: 0, s2: 0 }, {})}</section></div>
    <aside class="card side"><div class="empty grow">${ic('cursor')}<span class="b">Nothing selected</span><span class="s1">Select a track to see its chain: filters, EQ, compressor, saturation and sends.</span></div></aside>
    <aside class="card side"><div class="ph"><span class="h">Assistant</span></div><div class="empty grow">${ic('spark')}<span class="b">Add tracks first</span><span class="s1">The assistant works from measurements of your tracks, their roles and your brief.</span></div>${P.askFoot({ ctaCls: 'is-dis', est: '$0.00', spent: '$0.00 so far' })}</aside></div>`);
const spec = (title, inner, hint = '') => `<div class="spec"><h3>${title}</h3>${inner}${hint ? `<span class="hint">${hint}</span>` : ''}</div>`;
const smallEmpties = `<div class="sw free" data-phase="mix"><div class="sheet c3">
  ${spec('Assistant · no brief, no references', `<div class="empty">${ic('pen')}<span class="b">Tell the assistant what you are after</span><span class="s1">A genre, a mood and a note or two. References are optional.</span>${btn('Write the brief', 'primary')}</div>`, 'Auto mix stays available: it runs with the measurements alone.')}
  ${spec('Snapshots · none yet', `<div class="empty">${ic('copy')}<span class="b">No snapshots</span><span class="s1">Save the mix as it is now, keep working, and compare the two by ear.</span>${btn('Save snapshot', '', 'plus')}</div>`)}
  ${spec('History · nothing to undo', `<div class="empty">${ic('clock')}<span class="b">No steps yet</span><span class="s1">Every change you or the assistant make will be listed here.</span></div>`)}
  ${spec('Sections lane · empty', `<div class="stage"><div class="row secs" style="--head-w:var(--sp-8)"><div class="hd"></div><div class="ln"><span class="lane-note">${ic('plus', 'sm')} Drag here to mark intro, verse, chorus…</span></div></div></div>`, 'With sections the assistant can treat verse and chorus differently.')}
  ${spec('Inspector · sends with no returns', `<div class="empty">${ic('wave')}<span class="b">No returns in this project</span><span class="s1">Add a reverb or a delay to send this track to it.</span>${btn('Add reverb return', '', 'plus')}</div>`)}
  ${spec('Master · no reference', `<div class="empty">${ic('music')}<span class="b">No reference track</span><span class="s1">Match EQ and the reference line in the spectrum need one. Mastering works without it.</span>${btn('Add a reference…', '', 'plus')}</div>`)}
</div></div>`;
const loading = [
  P.frame('13c', 'Opening a project', 'Waveform overviews are built in the background: clips appear at once and fill in track by track. The window is usable meanwhile.', P.mixScreen({ playing: false, pos: '0:00.00', playAt: 0, section: 'Intro', engine: '2 %', clipMod: (t) => (t >= 6 ? { cls: 'is-loading' } : null),
    over: `<div class="toasts"><div class="toast">${ic('wave')}<div class="col g2 grow"><div class="fx between"><span class="b">Building waveforms</span><span class="faint s1">6 of 12</span></div>${progress(50)}</div></div><div class="toast">${ic('spark')}<div class="col g2 grow"><div class="fx between"><span class="b">Analysing tracks</span><span class="faint s1">4 of 12 · about 10 s</span></div>${progress(33, 'ai')}<span class="faint s1">Needed once per edit of the clips, before the first AI request.</span></div></div></div>` })),
];
const asstState = (kind) => {
  const m = { key: ['key', 'The assistant is not set up', 'Connect an OpenRouter account to ask the assistant, or switch to the offline demo.', btn('Set up the assistant…', 'primary') + btn('Use offline demo')],
    offline: ['offline', 'No connection to OpenRouter', 'Your mix is not affected. Requests will work again when the network is back.', btn('Try again', '') + btn('Use offline demo', 'quiet')] }[kind];
  return P.assistant({ asstBody: `${P.brief()}<div class="sect"><div class="banner ${kind === 'offline' ? 'err' : ''}" style="align-items:flex-start">${ic(m[0])}<div class="col g2 grow"><span class="b">${m[1]}</span><span class="dim s1">${m[2]}</span><div class="fx g2 wrap">${m[3]}</div></div></div></div>${P.askFoot({ ctaCls: 'is-dis' })}` });
};
const dlgStatic = (o) => L.dialog(o).replace('<div class="scrim">', '').replace(/<\/div>$/, '');
const confirms = `<div class="sw free" data-phase="mix"><div class="sheet c2" style="grid-template-columns:repeat(2,max-content)">
  ${dlgStatic({ size: 's', title: 'Delete “Gtr Right”?', sub: 'The track, its 3 clips and its automation are removed. You can undo this.', foot: btn('Cancel', 'quiet') + btn('Delete track', 'danger') })}
  ${dlgStatic({ size: 's', title: 'Delete the return “Plate”?', sub: '4 tracks send to it. Their sends to Plate are removed too. You can undo this.', foot: btn('Cancel', 'quiet') + btn('Delete return', 'danger') })}
  ${dlgStatic({ size: 's', title: 'Discard this proposal?', sub: 'Nothing was applied. The call stays in AI usage with its cost ($0.11).', foot: btn('Keep reviewing', 'quiet') + btn('Discard', 'danger') })}
  ${dlgStatic({ size: 's', title: 'Restore “Before auto mix”?', sub: 'The mix goes back to that snapshot as one step. You can undo it.', foot: btn('Cancel', 'quiet') + btn('Restore', 'primary') })}
  ${dlgStatic({ size: 's', title: 'Remove “Reference A”?', sub: 'Its copy is deleted from the project. Match EQ in Master will lose its target. This cannot be undone.', foot: btn('Cancel', 'quiet') + btn('Remove reference', 'danger') })}
  ${dlgStatic({ size: 's', title: 'Replace Mixdown 3?', sub: 'A new bounce replaces the mixdown you are mastering. Your master settings are kept.', foot: btn('Cancel', 'quiet') + btn('Bounce again', 'primary') })}
  ${dlgStatic({ size: 's', title: 'Stemwise closed unexpectedly', sub: 'An autosave from 21:46 is newer than the last saved project (21:31).', body: `<div class="tile row on">${check(true, 'radio')}<div class="col"><span class="b">Open the autosave</span><span class="dim s1">31 steps, last one “Lead Vox · EQ band 2 gain”.</span></div></div><div class="tile row">${check(false, 'radio')}<div class="col"><span class="b">Open the last saved version</span><span class="dim s1">The autosave is kept until you save again.</span></div></div>`, foot: btn('Open', 'primary') })}
  ${dlgStatic({ size: 's', title: 'Locate missing audio', sub: '2 files that belong to this project are not in its audio folder.', body: `<div class="col g1"><div class="fx between"><span class="b">07 Gtr L.wav</span>${tag('missing', 'err')}</div><div class="fx between"><span class="b">08 Gtr R.wav</span>${tag('found', 'ok')}</div></div>`, foot: btn('Skip', 'quiet') + btn('Choose folder…') + btn('Relink 1 file', 'primary') })}
</div></div>`;
const toasts = `<div class="sw free" data-phase="mix"><div class="sheet c3" style="grid-template-columns:repeat(3,max-content)">
  ${toast('ok', 'check', 'Auto mix applied', '11 tracks changed in one step.', btn('Undo', 'quiet sm'))}${toast('ok', 'check', 'Snapshot saved', '“After auto mix” is in Snapshots.')}${toast('ok', 'exp', 'Export complete', '3 files written.', btn('Show', 'quiet sm'))}
  ${toast('ai', 'spark', 'Proposal ready', 'Auto mix finished in 1 m 06 s. Review it when you like.', btn('Review', 'sm'))}${toast('warn', 'warn', 'Autosave failed', 'The disk is full. Free some space; your work is still in memory.', btn('Retry', 'quiet sm'))}${toast('err', 'x', 'The assistant could not answer', 'Invalid plan after 3 tries. $0.03 spent. Your mix is unchanged.', btn('Details', 'quiet sm'))}
  ${toast('warn', 'speaker', 'Audio device changed', 'Now playing through MacBook Pro Speakers.')}${toast('', 'info', 'Converted to 48 kHz', '“10 Strings.wav” was copied and converted for this project.')}${toast('warn', 'warn', 'Routing not allowed', 'Sending Drums to Kick would create a loop.')}
</div></div>`;
const devBanner = `<div class="banner err" style="margin:0 var(--gap) var(--gap)">${ic('speaker')}<div class="col grow"><span class="b">Scarlett 2i2 USB was disconnected</span><span class="dim s1">Playback stopped. Nothing in the project changed.</span></div>${btn('Use MacBook Pro Speakers', 'sm')}${btn('Audio settings', 'quiet sm')}</div>`;
const missBanner = `<div class="banner" style="margin:0 var(--sp-3) var(--sp-2)">${ic('warn')}<span class="grow">2 audio files are missing from the project folder. Their clips are hatched and silent.</span>${btn('Locate…', 'sm')}</div>`;

export default {
  'screens/11-settings.html': P.page('11 · Settings', 'Four sections: Audio, AI, Appearance, Shortcuts (view only).', ['Audio', 'AI', 'Appearance', 'Shortcuts'].map((n, i) => P.frame('11' + 'abcd'[i], n, ['Output device, buffer, engine load.', 'Decider, key, model, effort, temperature, spending cap, what is sent, total across projects.', 'Track height, clip content, time format, meters, motion, the 12 track colours.', 'Read-only list grouped by area, with a filter.'][i], settings(n)))),
  'screens/12-ai-usage.html': P.page('12 · AI usage', 'Reachable from both phases. Summary by type and by model, call history with filters and export, readable detail of a call with the raw JSON on request, empty state, and the spending-cap confirmation.', [
    P.frame('12a', 'Summary and history', 'Totals for the project, split by type and by model. The Export menu is open.', usage()),
    P.frame('12b', 'Call detail · readable', 'What was sent (request, brief, tracks, measures) next to what came back (summary, per-track changes with their reason, what was applied).', detail(false)),
    P.frame('12c', 'Call detail · raw JSON', 'The exact request and response, to read or copy.', detail(true)),
    P.frame('12d', 'Empty', 'No calls yet in this project.', usageEmpty()),
    P.frame('12e', 'Spending cap', 'Shown before sending when the estimate would take the project over the cap.', P.mixScreen({ over: capDialog })),
  ]),
  'screens/13-states.html': P.page('13 · Cross-cutting states', 'Empty, loading, the errors of UI-6 (missing files, different sample rate, missing key, no network, audio device disconnected), destructive confirmations and notifications.', [
    P.frame('13a', 'Empty project', 'A project with no tracks: every panel says what it is for and what to do next.', emptyMix()),
    P.frame('13b', 'Small empty states', 'Panels that can be empty inside a working project.', smallEmpties),
    ...loading,
    P.frame('13d', 'Missing files · audio device disconnected', 'Clips whose audio is missing are hatched; a banner in the timeline offers to locate them. A second banner under the top bar reports the disconnected device and offers another one.', P.mixScreen({ playing: false, engine: '—', noAuto: true, noWhy: true, clipMod: (t) => (t === 6 ? { cls: 'is-missing' } : null) }).replace('<div class="body">', devBanner + '<div class="body">').replace('<div class="tl"', missBanner + '<div class="tl"')),
    P.frame('13e', 'Missing API key', 'The assistant column explains and offers both ways out; the request button is disabled. The rest of the workspace is unaffected.', P.mixScreen({ asstPanel: asstState('key') })),
    P.frame('13f', 'No network', 'Same place, same shape: what happened, that the mix is safe, and what to do.', P.mixScreen({ asstPanel: asstState('offline') })),
    P.frame('13g', 'Confirmations and recovery', 'Destructive or replacing actions name the object and say whether they can be undone. Red only when something is deleted. The different-sample-rate choice is frame 2d.', confirms),
    P.frame('13h', 'Notifications', 'Toasts appear bottom right, stack upward, and leave after 6 seconds unless they carry an action or an error.', toasts),
  ]),
};
