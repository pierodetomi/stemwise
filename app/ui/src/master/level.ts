// Listening level and the target against the references (design 9l–9u, 10d–10e; MAS-2, MAS-5). Pure functions:
// the wording of the Listening section, the warning when the destination sits far from the references, and where
// a destination sits for the destination menu.
import type { MasterStatus } from "../bindings/MasterStatus";

/** Beyond this distance from the references the destination is worth a word (LU). */
export const GAP_LU = 3;

/** Shown numbers: one decimal, a real minus sign. */
export const f1 = (v: number) => v.toFixed(1).replace("-", "−");
/** A destination's loudness as the presets show it: "−18", "−9.6". */
export const ft = (v: number) => v.toFixed(1).replace(/\.0$/, "").replace("-", "−");

export interface RefLevel {
  id: string;
  /** "Reference A", "Reference B"… in the order of the project. */
  label: string;
  /** The file name. */
  name: string;
  lufs: number;
  true_peak_db: number;
}

/** The measured references (the unreadable ones of 9h are not in the status), labelled A, B… */
export function refLevels(ms: Pick<MasterStatus, "references">): RefLevel[] {
  return ms.references.flatMap((r, i) =>
    r.lufs == null ? [] : [{ id: r.id, label: `Reference ${String.fromCharCode(65 + i)}`, name: r.name, lufs: r.lufs, true_peak_db: r.true_peak_db }],
  );
}

export interface TargetGap {
  /** Below the quietest reference (a warning) or above the loudest (a note). */
  kind: "quieter" | "louder";
  /** Distance to the nearest reference (LU, positive). */
  lu: number;
  /** The nearest reference: what "Match" sets the destination to. */
  edge: RefLevel;
  /** Every reference with its distance from the destination, nearest first. */
  refs: { ref: RefLevel; lu: number }[];
}

/** D29: the references span a range; a destination more than 3 LU outside it gets a word. */
export function targetGap(target: number, refs: RefLevel[]): TargetGap | null {
  if (!refs.length) return null;
  const lo = refs.reduce((a, r) => (r.lufs < a.lufs ? r : a));
  const hi = refs.reduce((a, r) => (r.lufs > a.lufs ? r : a));
  const kind = target < lo.lufs - GAP_LU ? "quieter" : target > hi.lufs + GAP_LU ? "louder" : null;
  if (!kind) return null;
  const edge = kind === "quieter" ? lo : hi;
  const dist = refs.map((ref) => ({ ref, lu: Math.abs(ref.lufs - target) })).sort((a, b) => a.lu - b.lu);
  return { kind, lu: Math.abs(edge.lufs - target), edge, refs: dist };
}

/** Where a destination sits against the references, for the destination menu (9s). */
export function placeOf(target: number, refs: RefLevel[]): { text: string; warn: boolean } | null {
  if (!refs.length) return null;
  const lo = Math.min(...refs.map((r) => r.lufs));
  const hi = Math.max(...refs.map((r) => r.lufs));
  if (target < lo) return { text: `${f1(lo - target)} LU quieter`, warn: lo - target > GAP_LU };
  if (target > hi) return { text: `${f1(target - hi)} LU louder`, warn: target - hi > GAP_LU };
  return { text: refs.length > 1 ? "Between your references" : "Level with your reference", warn: false };
}

/** The warning's words (9q, 9r, 9u). */
export function gapWords(g: TargetGap, target: number, ceiling: number) {
  const many = g.refs.length > 1;
  const quieter = g.kind === "quieter";
  const title = `Much ${quieter ? "quieter" : "louder"} than your reference${many ? "s" : ""}`;
  const at = `At ${ft(target)} LUFS this master will`;
  let text: string;
  if (!many) {
    text = quieter
      ? `${at} sound ${f1(g.lu)} LU quieter than ${g.edge.label} (${f1(g.edge.lufs)} LUFS): clearly softer when the two play one after the other.`
      : `${at} be ${f1(g.lu)} LU louder than ${g.edge.label} (${f1(g.edge.lufs)} LUFS).`;
  } else {
    const all = g.refs.length === 2 ? "both" : "all of them";
    const list = g.refs.map(({ ref, lu }) => `${f1(lu)} LU ${quieter ? "below" : "above"} ${ref.label} (${f1(ref.lufs)} LUFS)`);
    const joined = list.length === 2 ? list.join(" and ") : `${list.slice(0, -1).join(", ")} and ${list[list.length - 1]}`;
    text = `${at} ${quieter ? "sound quieter" : "be louder"} than ${all}: ${joined}.`;
  }
  if (!quieter) text += " Getting there takes more limiting, which can flatten the punch, and streaming services turn loud songs down anyway.";
  const fix = `Match the reference${many ? "s" : ""}`;
  const which = many ? `, the ${quieter ? "quietest" : "loudest"} reference,` : "";
  const foot = `Sets the target to ${ft(g.edge.lufs)} LUFS${which} and keeps the ${ft(ceiling)} dBTP ceiling · one undo step`;
  return { title, text, fix, keep: `Keep ${ft(target)} LUFS`, foot };
}

/** A source of the A/B as the Listening section lists it (9l–9p). */
export interface ListenRow {
  value: string;
  name: string;
  lufs: number | null;
  offset: string;
  warn: boolean;
}

/**
 * The Listening section: every source with its loudness and, in words, its offset — against the master at the real
 * level, the gain it plays with at the same loudness (the trim included). The proposal takes the master's row while
 * it is heard.
 */
export function listenRows(ms: MasterStatus, gap: TargetGap | null): { rows: ListenRow[]; note: string | null } {
  const same = ms.listen_same;
  const trim = same && ms.ab_trim_db > 0.05 ? ms.ab_trim_db : 0;
  const current = ms.measures?.lufs ?? null;
  const proposal = ms.listening === "proposal" ? (ms.proposal?.after?.lufs ?? null) : null;
  const heard = ms.listening === "proposal" ? proposal : current;
  const words = (g: number) => (Math.abs(g) < 0.05 ? "unchanged" : g > 0 ? `raised ${f1(g)} dB` : `lowered ${f1(-g)} dB`);
  const against = (l: number | null) => {
    if (l == null || heard == null) return "";
    const d = l - heard;
    return Math.abs(d) < 0.05 ? "same level" : d < 0 ? `${f1(-d)} LU quieter` : `${f1(d)} LU louder`;
  };
  const gain = (l: number | null) => (l == null || current == null ? "" : words(current - l - trim));
  const far = (r: RefLevel) => !!gap && gap.kind === "quieter" && r.lufs - (heard ?? r.lufs) > GAP_LU;
  const head: ListenRow =
    ms.listening === "proposal"
      ? { value: "proposal", name: "Proposal", lufs: proposal, offset: same ? gain(proposal) : "as exported", warn: false }
      : { value: "master", name: "Master", lufs: current, offset: same ? words(-trim) : "as exported", warn: false };
  const mix = ms.mixdown_measures?.lufs ?? null;
  const rows: ListenRow[] = [
    head,
    { value: "mixdown", name: "Mixdown", lufs: mix, offset: same ? gain(mix) : against(mix), warn: false },
    ...refLevels(ms).map((r) => ({ value: `reference:${r.id}`, name: r.label, lufs: r.lufs, offset: same ? gain(r.lufs) : against(r.lufs), warn: !same && far(r) })),
  ];
  let note: string | null = null;
  if (trim && current != null) {
    // The source that forces the trim: the one whose peaks would go highest once raised.
    const cands = [
      ...(ms.mixdown_measures?.lufs != null ? [{ name: "the mixdown", l: ms.mixdown_measures.lufs, tp: ms.mixdown_measures.true_peak_db }] : []),
      ...refLevels(ms).map((r) => ({ name: r.label, l: r.lufs, tp: r.true_peak_db })),
    ];
    const worst = cands.reduce<(typeof cands)[number] | null>((a, c) => (!a || current - c.l + c.tp > current - a.l + a.tp ? c : a), null);
    if (worst) note = `Raising ${worst.name} ${f1(current - worst.l)} dB would push its peaks over −1 dBTP, so everything plays ${f1(trim)} dB lower. The export is not affected.`;
  }
  return { rows, note };
}

/** Under the proposal's A/B (9c, 9o). */
export function proposalHint(ms: MasterStatus): string {
  const cur = ms.measures?.lufs;
  const pro = ms.proposal?.after?.lufs;
  if (cur == null || pro == null) return "";
  const d = pro - cur;
  if (ms.listen_same) {
    return Math.abs(d) < 0.05 ? "The proposal plays at the current master's loudness." : `The proposal plays ${f1(Math.abs(d))} dB ${d > 0 ? "lower" : "higher"} to match the current master.`;
  }
  return Math.abs(d) < 0.05
    ? `Both at ${f1(cur)} LUFS.`
    : `The proposal is ${f1(Math.abs(d))} LU ${d > 0 ? "louder" : "quieter"}: ${f1(pro)} against ${f1(cur)} LUFS.`;
}
