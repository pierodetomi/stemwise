// Screen 9, boards 9l–9t — listening at the real level (default) or at the same loudness (on demand, MAS-5), and
// the warning when the target is far from the references (MAS-2).
import * as L from './lib.mjs';
import * as P from './parts.mjs';
import { master } from './s-master.mjs';
import * as V from './level.mjs';
const { ic, px } = L;

const tip = (x, y, html, w = '') => `<div class="tip" style="left:${px(x)};top:${px(y)}${w ? `;max-width:${w}` : ''}">${html}</div>`;
const kbd = (k) => `<span class="kbd">${k}</span>`;

// Rows of the Listening section: [name, LUFS, offset in words, class]. Real level: against the master. Same
// loudness: the gain each source plays with.
const SAME = [['Master', '−14.1', 'unchanged'], ['Mixdown', '−16.9', 'raised 2.8 dB'], ['Reference A', '−12.9', 'lowered 1.2 dB']];
const TRIM = [['Master', '−14.1', 'lowered 2.5 dB'], ['Mixdown', '−18.6', 'raised 2.0 dB'], ['Reference A', '−12.9', 'lowered 3.7 dB']];
const TRIM_NOTE = 'Raising the mixdown 4.5 dB would push its peaks over −1 dBTP, so everything plays 2.5 dB lower. The export is not affected.';
const CUSTOM = { target: 'Custom · −18 LUFS · −1 dBTP', integ: '−18.0', tgtLufs: '−18.0' };
const M18 = ['Master', '−18.0', 'as exported'], MIX18 = ['Mixdown', '−18.6', '0.6 LU quieter'];
const FAR1 = [M18, MIX18, ['Reference A', '−9.6', '8.4 LU louder', 'warnc']];
const FAR2 = [...FAR1, ['Reference B', '−11.2', '6.8 LU louder', 'warnc']];
const NAMES2 = ['Master', 'Mixdown', '<span title="Reference A">Ref A</span>', '<span title="Reference B">Ref B</span>'];
const KEPT = [M18, MIX18, ['Reference A', '−9.6', '8.4 LU louder']];
const LOUD = [['Master', '−9.0', 'as exported'], ['Mixdown', '−18.6', '9.6 LU quieter'], ['Reference A', '−14.2', '5.2 LU quieter']];

// Destination menu, open under the Target select: every preset says where it sits against the references.
const mrow = (label, meta, o = {}) => `<div class="mi ${o.cls || ''}" role="menuitemradio" aria-checked="${!!o.on}"><span class="chk">${o.on ? ic('check', 'm') : ''}</span>${o.icon ? ic(o.icon, 'm') : ''}<span class="nowrap">${label}</span><span class="meta ${o.warn ? 'warnc' : ''}">${o.warn ? ic('warn', 'sm') + ' ' : ''}${meta}</span></div>`;
const destMenu = L.menu(['#Destination', mrow('Streaming · −14 LUFS · −1 dBTP', '2.8 LU quieter'), mrow('Loud · −9 LUFS · −1 dBTP', '0.6 LU louder'), mrow('CD · −10 LUFS · −0.3 dBTP', 'Between your references'),
  mrow('Custom · −18 LUFS · −1 dBTP', '6.8 LU quieter', { on: 1, warn: 1, cls: 'is-hover' }), '-', mrow('Match the references · −11.2 LUFS', 'Keeps −1 dBTP', { icon: 'level' }), mrow('Custom…', '')],
  `left:${px(804)};top:${px(152)};width:calc(var(--col-side) + var(--sp-8))`);

export default {
  'screens/09-master-listening.html': P.page('9 · Master workspace, listening level and target warning', 'The master plays at its real level by default, exactly as the exported file will sound; the mixdown and the references play at theirs. Level-matched comparison (MAS-5) is one click or ⇧B away. When the target sits far from the references, the Loudness card says so in plain words and offers a one-click fix.', [
    P.frame('9l', 'Real level (default)', 'Listening to Reference A at its real level. The Listening section of the Loudness card lists every source with its loudness and, in words, how much louder or quieter it is than the master; the row being heard is marked.', master({ ab: 2,
      over: tip(850, 52, `Compare at the same loudness: quieter sources are raised, louder ones lowered. ${kbd('⇧B')}`, '260px') })),
    P.frame('9m', 'Same loudness', 'Level-matched on demand. The mode reads in the lit “Same loudness” segment (accent outline, = sign) and in the Listening section, which now says what each source is raised or lowered by. Here the mixdown has headroom, so the master is unchanged.', master({ ab: 1, mixText: '−16.9 LUFS, −4.2 dBTP', listen: { same: true, rows: SAME } })),
    P.frame('9n', 'Same loudness, lowered to avoid clipping', 'Raising the mixdown 4.5 dB would push its peaks over −1 dBTP, so every source, the master too, plays 2.5 dB lower. The Listening section says so in each row and in one line under them.', master({ listen: { same: true, rows: TRIM, note: TRIM_NOTE } })),
    P.frame('9o', 'Master proposal in review, same loudness', 'The A/B in the Assistant column follows the same mode as the top bar (9c shows the real level). B switches between current master and proposal, ⇧B switches the mode. While the proposal is heard, the first row of the Listening section is the proposal.', master({ proposal: true, noNotes: true, listen: { same: true, rows: [['Proposal', '−14.0', 'lowered 2.6 dB'], ...TRIM.slice(1)], note: TRIM_NOTE } })),
    P.frame('9p', 'No references', 'Only Master and Mixdown to compare. Nothing to measure the target against, so no warning even with a quiet Custom target.', master({ noRefs: true, ...CUSTOM, listen: { names: ['Master', 'Mixdown'], rows: [M18, MIX18] } })),
    P.frame('9q', 'Target much quieter than the reference', 'Custom −18 LUFS against Reference A at −9.6 LUFS. The Target select turns amber and the card explains the gap in words, with the fix and a way to keep the target. Reference A’s row in the Listening section is amber too.', master({ ...CUSTOM, tgtWarn: true, lwarn: V.quieter1, listen: { rows: FAR1 } })),
    P.frame('9r', 'Several references', 'Two references, at −9.6 and −11.2 LUFS. The warning compares with the quietest one and the fix brings the target up to it. With two or more references the A/B segments shorten to “Ref A”, “Ref B”; the Listening section keeps the full names.', master({ ...CUSTOM, refs: '2 references', tgtWarn: true, lwarn: V.quieter2, listen: { names: NAMES2, rows: FAR2 } })),
    P.frame('9s', 'Destination menu', 'Each destination says where it sits against the references, amber beyond 3 LU. “Match the references” is an entry of its own (the quietest reference, keeping the ceiling).', master({ ...CUSTOM, refs: '2 references', tgtWarn: true, tgtOpen: true, lwarn: V.quieter2, listen: { names: NAMES2, rows: FAR2 }, over: destMenu })),
    P.frame('9t', 'After “Keep −18 LUFS”', 'The warning is gone for this project. A quiet line stays under the target with “Warn me again”. Reference A’s row turns neutral.', master({ ...CUSTOM, lwarn: V.kept, listen: { rows: KEPT } })),
    P.frame('9u', 'Target much louder than the reference', 'Loud −9 LUFS against Reference A at −14.2 LUFS: a neutral note, not a warning, with the same two actions.', master({ target: 'Loud · −9 LUFS · −1 dBTP', integ: '−9.0', tgtLufs: '−9.0', lwarn: V.louder1, listen: { rows: LOUD } })),
  ]),
};
