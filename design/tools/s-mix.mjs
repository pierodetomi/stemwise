// Screens 3, 4, 5 — Mix workspace, proposal in review, AI at work.
import * as L from './lib.mjs';
import * as P from './parts.mjs';
import { mixFrames } from './glance.mjs';
const { ic, tag, badge, seg, btn, check } = L;

// ---------- 4 · proposal panel (takes the place of inspector + assistant) ----------
const CARDS = [
  [1, 'Kick', 1, 'Keeps its click at 3 kHz; 1 dB down with the drum group.', 3], [2, 'Snare', 0, 'A little more body at 200 Hz, faster compressor release.', 4],
  [3, 'Overheads', 0, 'High-pass at 180 Hz so the kick stays clean.', 2], [4, 'Room', 0, 'Down 2 dB and darker: the brief asks for dry drums.', 3],
  [5, 'Bass DI', 2, 'Ducks 2 dB on every kick through a sidechain.', 4], [6, 'Acoustic', 0, 'Thinned below 120 Hz, panned slightly left.', 3],
  [7, 'Gtr Left', 3, 'Wide left, 2.5 dB dip at 3 kHz where it masked the vocal.', 4], [8, 'Gtr Right', 3, 'Mirror of Gtr Left on the right side.', 4],
  [9, 'Rhodes', 0, 'Sent to Echo in the bridge only.', 2], [10, 'Strings', 0, 'Up 1.5 dB in the last chorus.', 2],
  ['b', 'Drums · group', 1, 'Group fader 1 dB down to make room for the vocal.', 1],
  ['b', 'Plate · reverb', 0, 'Shorter and darker; pre-delay 20 ms.', 4], ['b', 'Echo · delay', 0, 'Dotted eighth at 375 ms, feedback 25 %.', 3],
];
// Returns and groups the plan creates (AI-17, AI-20): icon, name, what it is, why, who uses it.
const NEW = [
  ['wave', 'Vocal plate', 'reverb return · plate, 1.1 s', 'A short, bright space for the vocal alone.', 'Lead Vox sends here'],
  ['wave', 'Slap', 'delay return · 110 ms, no repeats', 'Slapback: body on the vocal without a tail.', 'Lead Vox sends here'],
  ['group', 'Pads', 'group · Rhodes, Strings', 'One bus to tuck the pads under the vocal together.', '2 tracks move into it'],
];
// Creative ideas (P-018, AI-26, AI-27): id, name, where it plays, why, what it adds.
const IDEAS = [
  ['i1', 'Chorus on the keys', 'Choruses', 'The Rhodes open to the sides in the choruses and leave the centre to the vocal.', ['Rhodes · Modulation, chorus 1/4, 35 % · new block', 'On in the 3 choruses only']],
  ['i2', 'Filter build to the end', 'Bridge', 'The strings start dark in the bridge and open into the last chorus, so it lands bigger.', ['Strings · Low-pass resonance 2.0', 'Bridge · low-pass 3 octaves lower']],
  ['i3', 'Octave halo in the bridge', 'Bridge', 'A shimmer return fed by the lead vocal in the bridge only: a halo an octave up.', ['New return · Shimmer +12, 4.5 s', 'Lead Vox · send −18 dB in the bridge']],
];
const ideaCard = (c, on, play) => `<div class="pcard icard ${on ? 'on' : ''} ${play ? 'is-play' : ''}"><div class="fx g2">${check(on)}${ic('spark', 'm aic')}<span class="b ell grow">${c[1]}</span></div><span class="dim s1">${c[3]}</span><div class="col">${c[4].map((x) => `<span class="faint s1 ell">· ${x}</span>`).join('')}</div><div class="fx g2 mt-auto">${tag(c[2])}<span class="grow"></span>${btn(play ? 'Listening' : 'Listen', 'quiet sm' + (play ? ' on' : ''), 'ear')}</div></div>`;
const ideasBlock = (on = [], play = -1) => `<div class="fx between"><span class="b">Creative ideas</span><span class="faint s1">optional · off until you tick them · ${on.length} of ${IDEAS.length} ticked</span></div>
  <div class="pgrid three">${IDEAS.map((c, i) => ideaCard(c, on.includes(i), play === i)).join('')}</div>`;
export function proposal(o = {}) {
  const off = o.off || [], n = CARDS.length + 1 - off.length; // + Lead Vox
  // newOff: unticked by hand; newLost: ticked, but the selected tracks no longer need it.
  const newOff = o.newOff || [], newLost = o.newLost || {};
  const newCard = (c, i) => { const lost = newLost[i], out = newOff.includes(i) || lost;
    return `<div class="pcard tb ${out && !lost ? 'is-off' : ''}"><div class="fx g2">${check(!out, lost ? 'is-dis' : '')}${ic(c[0], 'm')}<span class="b ell">${c[1]}</span>${tag('New', 'ai')}<span class="grow"></span><span class="faint s1 nowrap">${lost ? 'not created' : newOff.includes(i) ? 'left out' : c[4]}</span></div><span class="faint s1 ell">${c[2]}</span><span class="${lost ? 'warnc' : 'dim'} s1 ell">${lost || c[3]}</span></div>`; };
  const made = NEW.length - newOff.length - Object.keys(newLost).length;
  const card = (c, i) => `<div class="pcard t${c[0]} ${c[0] === 'b' ? 'tb' : ''} ${off.includes(i) ? 'is-off' : ''}"><div class="fx g2">${check(!off.includes(i))}<i class="tdot"></i><span class="b ell">${c[1]}</span>${c[2] ? badge(c[2]) : ''}<span class="grow"></span><span class="faint s1 nowrap">${c[4]} changes</span></div><span class="dim s1 ell">${c[3]}</span></div>`;
  return `<aside class="card side wide" aria-label="Proposal">
    <div class="ph">${ic('spark', 'aic')}<span class="h">Proposal · Auto mix</span>${tag('Best of 2 passes', 'ai')}<div class="grow"></div><span class="faint s1">$0.11 · 14.2k tokens</span><a href="12-ai-usage.html" class="s1">Details</a></div>
    <div class="sect"><div class="tile row">${ic('ear')}<div class="col grow"><span class="b">${o.listen || (o.ab ? 'Listening to the proposal' : 'Listening to your current mix')}</span><span class="faint s1">${o.listenSub || 'Switch while it plays. Nothing is applied yet. Shortcut <span class="kbd">B</span>'}</span></div>${seg(['Current mix', 'Proposal'], o.ab ? 1 : 0, '', 'A/B')}</div>
      <p class="text">${L.SUMMARY}</p>
      <div class="fx g2 stretch"><div class="stat sm"><span class="faint s1">Distance to reference</span><span class="v">4.8 → 2.1</span><div class="hbar ai"><i style="width:44%"></i></div></div>
        <div class="stat sm"><span class="faint s1">True peak</span><span class="v">−3.1 dBTP</span><span class="okc s1">${ic('check', 'sm')} headroom kept</span></div>
        <div class="stat sm"><span class="faint s1">Loudness</span><span class="v">−16.4 LUFS</span><span class="okc s1">${ic('check', 'sm')} no clipping, mono safe</span></div></div></div>
    <div class="sect grow" style="overflow:hidden;min-height:0">
      <div class="pcard on t11"><div class="fx g2">${check(true)}<i class="tdot"></i><span class="b">Lead Vox</span>${badge(4)}<span class="grow"></span><span class="faint s1">5 changes</span>${ic('up', 'm')}</div>
        <div class="fx g4 top-al"><p class="why grow">${L.WHY}</p><div class="col" style="width:var(--field-w-l)">${L.diffRows(o.diff)}</div></div></div>
      ${o.ideas ? ideasBlock(o.ideas.on, o.ideas.play) : ''}
      <div class="fx between"><span class="b">New in this proposal</span><span class="faint s1">${made} of ${NEW.length} will be created · one undo step removes them with the rest</span></div>
      <div class="pgrid">${NEW.map(newCard).join('')}</div>
      <div class="fx between"><span class="b">Tracks, groups and returns</span><span class="faint s1">${n} of ${CARDS.length + 1} selected</span></div>
            <div class="pgrid">${CARDS.map(card).join('')}<div class="pcard is-lock t12"><div class="fx g2">${ic('lock', 'm aic')}<i class="tdot"></i><span class="b">BVs</span><span class="grow"></span><span class="faint s1">locked</span></div><span class="dim s1 ell">Sent as context, left untouched.</span></div></div></div>
    <div class="foot" style="flex-direction:row;align-items:center">${btn('Discard', 'quiet', 'x')}<span class="fx g2 s1 dim grow">${check(true)}Level every track to −23 LUFS first (trim)</span>${btn(`Apply selected (${n})`, off.length ? '' : 'is-dis')}${btn('Apply all', 'primary', 'check')}</div></aside>`;
}

const reviewChip = `<div class="fx g2">${tag(ic('spark', 'sm') + 'Reviewing a proposal', 'ai')}</div>`;
const workChip = `<div class="fx g2" style="width:var(--field-w-l)">${tag(ic('spark', 'sm') + 'Auto mix · pass 2 of 3', 'ai')}<div class="grow">${L.progress(46, 'ai')}</div></div>`;

export default {
  'screens/03-mix-workspace.html': P.page('3 · Mix workspace', 'The main screen at the reference size, its two other assistant tabs, the minimum window, the Overview and a large project. Timeline, mixer and inspector always show the same state. Every header and strip carries the marks of what its track has on (P-013): chain blocks, sends, automation.', [
    P.frame('3a', 'Mix workspace', 'Playing inside a loop. Lead Vox is selected: its chain is in the inspector, its note (4) is highlighted in the assistant, and its fader automation lane is open.', P.mixScreen()),
    P.frame('3b', 'History', 'Assistant column on the History tab. The step in progress is marked; steps written by the assistant carry the spark icon.', P.mixScreen({ tab: 1, playing: false, pos: '0:48.00', redo: false })),
    P.frame('3c', 'Snapshots and A/B', 'Two states are armed as A and B and can be switched while listening.', P.mixScreen({ tab: 2 })),
    P.frame('3d', 'Minimum window · 1280 × 720', 'Side columns narrow to 288, track headers to 304, lanes drop to the small height (clip name strips and role lines hide), the inspector drops the note card (it stays in the assistant).', P.mixScreen({ min: true })),
    ...mixFrames,
  ]),
  'screens/04-mix-proposal.html': P.page('4 · Mix workspace, proposal in review', 'The plan never applies itself. The proposal takes the place of inspector and assistant until it is applied or discarded; the timeline and mixer stay live for listening.', [
    P.frame('4a', 'Proposal in review', 'A/B is on the proposal. Every changed track has a card with its reason; the selected one is open with its parameter diff. Numbered notes match the badges on the track headers. Returns and groups the plan creates come first, marked New, each with what it is, why, and who uses it: the A/B already includes them.', P.mixScreen({ insp: '', asstPanel: proposal({ ab: 1 }), mid: reviewChip })),
    P.frame('4c', 'Creative ideas', 'P-018. Below the summary, up to three ideas the assistant proposes from the brief, the roles and the sections, apart from the plan: each only adds (a block, a send, a return, a section offset) and has its own tick, off by default. A/B plays the proposal with the ideas ticked: here one. The ideas are not part of the evaluation. New blocks of the plan itself appear in the diff of their track, marked new (De-esser on Lead Vox).', P.mixScreen({ insp: '', asstPanel: proposal({ ab: 1, ideas: { on: [0] }, diff: [['De-esser · new block', '—', '6.5 kHz'], ...L.DIFF.slice(0, 4)], listen: 'Listening to the proposal with 1 idea' }), mid: reviewChip })),
    P.frame('4d', 'Listening to one idea', '“Listen” on an idea plays the proposal with that idea alone, whatever is ticked, until it is pressed again or A/B is switched. Ticking decides what “Apply all” creates; the idea’s elements are one undo step with the rest.', P.mixScreen({ insp: '', asstPanel: proposal({ ab: 1, ideas: { on: [0], play: 2 }, diff: [['De-esser · new block', '—', '6.5 kHz'], ...L.DIFF.slice(0, 4)], listen: 'Proposal + “Octave halo in the bridge” alone', listenSub: 'Press Listen again, or switch A/B, to go back. Nothing is applied yet.' }), mid: reviewChip })),
    P.frame('4b', 'Applying only some tracks', 'Three cards are unticked: they dim, the counter on “Apply selected” updates, and A/B is back on the current mix. New elements follow: Slap was unticked by hand and is left out; Pads is not created because fewer than two of its tracks are still selected, and says so. The list scrolls under the fixed footer.', P.mixScreen({ insp: '', asstPanel: proposal({ ab: 0, off: [3, 8, 12], newOff: [1], newLost: { 2: 'Needs two of its tracks selected: only Strings is.' } }), mid: reviewChip })),
  ]),
  'screens/05-mix-ai-working.html': P.page('5 · Mix workspace, AI at work', 'Calls never block the interface. Progress is shown per phase and per pass, in the assistant column and as a chip in the top bar.', [
    P.frame('5a', 'Auto mix in progress', 'Second pass rendering. The workspace stays usable; the request field is replaced by Cancel.', P.mixScreen({ asst: 'working', mid: workChip })),
    P.frame('5b', 'Cancelled', 'After Cancel the assistant returns to its notes and a toast reports what the run spent.', P.mixScreen({ over: `<div class="toasts">${L.toast('ai', 'spark', 'Auto mix cancelled', 'Nothing was changed. The run spent $0.07, counted in AI usage.', btn('Details', 'quiet sm'))}</div>` })),
  ]),
};
