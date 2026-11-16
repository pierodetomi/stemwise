// Stemwise design boards — copy and blocks for the target warning (boards 9q–9t, 10d–10e). Not a page module.
// Rule: the references span a range from the quietest to the loudest (integrated LUFS). A target more than 3 LU
// below the quietest reference gets a warning (amber); more than 3 LU above the loudest gets a note (neutral).
import * as L from './lib.mjs';
const { ic, btn } = L;

const acts = (fix, keep, foot) => `<div class="fx g2 wrap">${btn(fix, 'primary sm')}${btn(keep, 'quiet sm')}</div><span class="faint s1">${foot}</span>`;
const block = (cls, icon, title, text, a) => `<div class="banner stack ${cls}" role="${cls === 'info' ? 'note' : 'alert'}"><div class="fx g2">${ic(icon)}<div class="col g1 grow"><span class="b">${title}</span><span class="dim s1">${text}</span></div></div>${a}</div>`;

// In the Loudness card, under the Target select.
export const quieter1 = block('', 'warn', 'Much quieter than your reference', 'At −18 LUFS this master will sound 8.4 LU quieter than Reference A (−9.6 LUFS): clearly softer when the two play one after the other.',
  acts('Match the reference', 'Keep −18 LUFS', 'Sets the target to −9.6 LUFS and keeps the −1 dBTP ceiling · one undo step'));
export const quieter2 = block('', 'warn', 'Much quieter than your references', 'At −18 LUFS this master will sound quieter than both: 6.8 LU below Reference B (−11.2 LUFS) and 8.4 LU below Reference A (−9.6 LUFS).',
  acts('Match the references', 'Keep −18 LUFS', 'Sets the target to −11.2 LUFS, the quietest reference, and keeps the −1 dBTP ceiling · one undo step'));
export const louder1 = block('info', 'info', 'Much louder than your reference', 'At −9 LUFS this master will be 5.2 LU louder than Reference A (−14.2 LUFS). Getting there takes more limiting, which can flatten the punch, and streaming services turn loud songs down anyway.',
  acts('Match the reference', 'Keep −9 LUFS', 'Sets the target to −14.2 LUFS and keeps the −1 dBTP ceiling · one undo step'));
export const kept = `<div class="kept">${ic('info', 'sm')}<span class="grow">8.4 LU quieter than Reference A · kept on purpose. <a href="#9q">Warn me again</a></span></div>`;

// In the export dialog (10a), above the formats. Export stays available: the warning never blocks.
export const exportWarn = `<div class="banner">${ic('warn')}<div class="col grow"><span class="b">This master will be 8.4 LU quieter than your reference</span><span class="dim s1">The target is Custom −18 LUFS; Reference A plays at −9.6 LUFS. Next to similar songs it will sound clearly softer. Matching sets −9.6 LUFS and keeps the −1 dBTP ceiling.</span></div>${btn('Match the reference', 'sm')}${btn('Keep −18 LUFS', 'quiet sm')}</div>`;

// In the export report (10c), when the files were written with the gap.
export const reportWarn = `<div class="tile row">${ic('warn', 'warnc')}<span class="dim s1"><span class="b">8.4 LU quieter than Reference A.</span> The files are at −18 LUFS; the reference plays at −9.6 LUFS. If that wasn’t intended, choose “Match the reference” in the Loudness card and export again.</span></div>`;
export const reportKept = `<div class="tile row">${ic('info')}<span class="dim s1">Exported at −18 LUFS on purpose, 8.4 LU quieter than Reference A.</span></div>`;
