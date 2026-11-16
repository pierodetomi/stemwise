// Screen 9, boards 9d–9k and 9v — preparing the master: the overlay shown while the master's background analyses run.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
import { master } from './s-master.mjs';
const { ic, btn, tag } = L;

// The master body as empty shells: what is coming, with no number and no waveform in it (behind the scrim).
const CHAIN = ['Input', 'EQ', 'Match EQ', 'Compressor', 'Width', 'Saturation', 'Limiter', 'Dither'];
const ghost = () => `<div class="body ghost" aria-hidden="true"><div class="left">
    <section class="card"><div class="tools"><span class="b faint">Mixdown 3</span></div><div class="mwave"></div></section>
    <div class="mods">${CHAIN.map((m, i) => `<div class="mod"><span class="fx g2"><span class="faint s1">${i + 1}</span><span class="h s faint">${m}</span></span><span class="faint s1">&nbsp;</span><span class="faint s1">&nbsp;</span></div>`).join('')}</div>
    <section class="card grow"><div class="ph"><span class="h">EQ</span></div></section></div>
  <aside class="card side"><div class="ph"><span class="h">Loudness</span></div></aside><aside class="card side"><div class="ph"><span class="h">Assistant</span></div></aside></div>`;

// One step: state is done | now | wait | warn | fail | later (not started after a failure).
const N = { done: () => ic('check', 'sm'), warn: () => ic('warn', 'sm'), fail: () => ic('x', 'sm') };
const step = (state, n, label, sub, meta) => `<div class="step ${state === 'wait' ? '' : state}"><span class="n">${N[state] ? N[state]() : n}</span><span class="col grow"><span>${label}</span>${sub ? `<span class="sub s1">${sub}</span>` : ''}</span><span class="s1 ${state === 'now' ? '' : state === 'warn' ? 'warnc' : state === 'fail' ? 'dangerc' : 'faint'}">${meta}</span></div>`;

// The overlay: a status panel over the master body. The top bar stays (phase selector live, the rest disabled).
const panel = (o) => `<div class="scrim below-top"><div class="dialog" role="status" aria-live="polite" aria-label="${o.title}">
  <div class="dlg-h"><span class="h l">${o.title}</span><span class="dim text">${o.sub}</span></div>
  <div class="dlg-b">${o.bar === false ? '' : `<div class="col g1">${L.progress(o.p)}<div class="fx between"><span class="b">${o.p} %</span><span class="faint s1">${o.eta}</span></div></div>`}
    <div class="steps phase">${o.steps.join('')}</div>${o.note || ''}</div>
  <div class="dlg-f">${o.foot || `<span class="faint s1 grow">Going back to Mix keeps this going.</span>${btn('Back to Mix <span class="kbd">Esc</span>', '', '')}`}</div></div></div>`;

const prep = (o) => P.app({ phase: 'master' }, `${P.top({ phase: 'master', noSection: true, playing: false, pos: '0:00.00', wait: true, status: o.status || 'Mixdown 3 · preparing' })}${ghost()}${panel(o)}`);
const info = (t) => `<div class="tile row">${ic('info')}<span class="dim s1">${t}</span></div>`;
const SAVED = 'Saved in the project, so this runs once per mixdown and reference.';

const afterBounce = prep({ title: 'Preparing the master', sub: 'Mixdown 3 · 3:42 · 48 kHz · 2 references', p: 38, eta: '0:06 elapsed · about 12 s left',
  steps: [step('done', 1, 'Reading Mixdown 3', '', 'From the bounce'), step('done', 2, 'Measuring the mixdown', 'Loudness, peaks, spectrum, waveform', '2.1 s'),
    step('now', 3, 'Converting Reference A to 48 kHz', 'From 44.1 kHz · the copy stays in the project', '46 %'), step('wait', 4, 'Measuring the references', 'Reference A, Reference B', 'Waiting'),
    step('wait', 5, 'Bringing the master to the target loudness', 'Streaming · −14 LUFS · −1 dBTP', 'Waiting')],
  note: info(`The master opens when every measure is ready, so the first thing you hear is already at the target loudness. ${SAVED}`) });

const reopened = prep({ title: 'Preparing the master', sub: 'Mixdown 3 · 3:42 · 48 kHz · no references', p: 82, eta: '0:03 elapsed · about 1 s left',
  steps: [step('done', 1, 'Reading Mixdown 3', '', '1.6 s'), step('done', 2, 'Measuring the mixdown', 'Loudness, peaks, spectrum, waveform', 'Saved in the project'),
    step('now', 3, 'Bringing the master to the target loudness', 'Streaming · −14 LUFS · −1 dBTP', '70 %')],
  note: `<span class="faint s1">No references in this project, so there is nothing to convert or compare. Add one in Project settings.</span>` });

const refWarn = prep({ title: 'Preparing the master', sub: 'Mixdown 3 · 3:42 · 48 kHz · 2 references', p: 61, eta: '0:05 elapsed · about 4 s left',
  steps: [step('done', 1, 'Reading Mixdown 3', '', '1.5 s'), step('done', 2, 'Measuring the mixdown', 'Loudness, peaks, spectrum, waveform', 'Saved in the project'),
    step('warn', 3, 'Reference B can’t be read', 'The file is missing or damaged · skipped', 'Skipped'), step('now', 4, 'Measuring the references', 'Reference A', '52 %'),
    step('wait', 5, 'Bringing the master to the target loudness', 'Streaming · −14 LUFS · −1 dBTP', 'Waiting')],
  note: info('A reference that can’t be read doesn’t stop the master: it is left out of the comparison, and Master tells you when it opens.') });

const failed = prep({ title: 'The master can’t be prepared', sub: 'Mixdown 3 could not be read: the file is missing from the project or damaged.', bar: false, status: 'Mixdown 3 · unreadable',
  steps: [step('fail', 1, 'Reading Mixdown 3', 'The file ends early (2:18 of 3:42)', 'Failed'), step('later', 2, 'Measuring the mixdown', '', 'Not started'), step('later', 3, 'Measuring the references', '', 'Not started'),
    step('later', 4, 'Bringing the master to the target loudness', '', 'Not started')],
  note: info('Your mix and your master settings are safe. Bouncing again writes a new mixdown from the current mix and keeps every master setting.'),
  foot: `${btn('Copy details', 'quiet', 'copy')}<span class="grow"></span>${btn('Back to Mix')}${btn('Bounce again', 'primary', 'loop')}` });

// Back in Mix while the master is prepared: a ring on the Master segment, then a toast when it is ready.
const ring = (p) => `<svg class="ring" viewBox="0 0 16 16" aria-hidden="true"><circle class="bgc" cx="8" cy="8" r="6"/><circle class="fgc" cx="8" cy="8" r="6" stroke-dasharray="${L.f1(p * 37.7)} 37.7"/></svg>`;
const tipAt = `<div class="tip" style="left:${L.px(250)};top:${L.px(46)}">Preparing the master · 64 % · keeps going while you mix</div>`;
const inMix = P.mixScreen({ playing: true, phaseItems: ['Mix', `Master${ring(0.64)}`], over: tipAt });
const readyToast = P.mixScreen({ playing: true, over: `<div class="toasts">${L.toast('ok', 'check', 'Master is ready', 'Mixdown 3 is measured and at −14 LUFS for Streaming.', `<span data-phase="master" style="display:contents">${btn('Open Master', 'primary sm')}</span>`)}</div>` });

const refBanner = `<div class="banner">${ic('warn')}<div class="col grow"><span class="b">Reference B can’t be read</span><span class="dim s1">The comparison, the match EQ and the A/B use Reference A only until you replace or remove it.</span></div>${btn('Project settings…', 'sm')}${btn('Not now', 'quiet sm')}</div>`;

// 9v: a new target. The same panel over the workspace as it is (not the shells): the values behind are the old target's.
const retarget = master({ hold: true, target: 'Custom · −9.6 LUFS · −0.3 dBTP', over: panel({ title: 'Bringing the master to the new target', sub: 'Custom · −9.6 LUFS · −0.3 dBTP · was −14 LUFS, −1 dBTP', p: 42, eta: '0:04 elapsed',
  steps: [step('now', 1, 'Finding the gain for −9.6 LUFS', 'The whole master is rendered and measured until it is on target', '50 %'), step('wait', 2, 'Measuring the result', 'Loudness, true peak, range', 'Waiting')],
  note: info('Until this is done the master still plays at the level it had. The workspace comes back when the new level is in place, so what you hear and what you read are the new target.') }) });

export default {
  'screens/09-master-preparing.html': P.page('9 · Master workspace, preparing', 'When Master opens (after a bounce, on a reopened project, after a reference changes) the mixdown and the references are read and measured and the master is brought to the target loudness. Until all of that is done an overlay covers the workspace and lists the steps; then the workspace appears complete, at once.', [
    P.frame('9d', 'Preparing · after a bounce', 'The mixdown comes from the bounce in memory, so reading it is done at once. Reference A is at 44.1 kHz and is converted first. Top bar: phase selector live, transport, undo, A/B and Export disabled.', afterBounce),
    P.frame('9e', 'Preparing · reopened, no references', 'Measures saved in the project are listed as done with “Saved in the project”. Steps that do not apply (no references) are not listed.', reopened),
    P.frame('9f', 'Back to Mix while preparing', 'Esc or “Back to Mix” returns to the mix without cancelling. The Master segment shows a ring with the progress; its tooltip says what is running.', inMix),
    P.frame('9g', 'Ready while in Mix', 'When the preparation ends in Mix, a toast offers to open Master. Opening it then is instant.', readyToast),
    P.frame('9h', 'A reference can’t be read', 'Not an error: the step turns amber with the warning icon, is skipped, and the preparation goes on.', refWarn),
    P.frame('9i', 'Opened with a skipped reference', 'The workspace appears as usual with a banner that names the reference that was left out.', master({ banner: refBanner })),
    P.frame('9j', 'The mixdown can’t be read', 'The only blocking error. The failed step is red with a cross; the rest are not started. Bounce again (8b, then 9d) or go back to Mix.', failed),
    P.frame('9k', 'Measuring again after a change', 'A master change (here the EQ high shelf) plays at once and is measured again in the background. No overlay: the Loudness card says it is measuring and dims the last values; everything stays usable. Not for a change of the target: see 9v.', master({ measuring: true })),
    P.frame('9v', 'A new target', 'The loudness or the ceiling of the destination changed (a preset, Custom values, “Match the reference”, undo or redo of one of these, a master proposal that sets it). Unlike 9k the change is not heard until the gain is found again, so the workspace waits under the same overlay, over the workspace as it is. Top bar: phase selector and transport live (the master plays at the level it had); undo, A/B and Export disabled.', retarget),
  ]),
};
