// Menus of the mix workspace (design 6e, 7b, 7c, 7d): track context menu, Add menu, output and sidechain
// pickers, the parameter picker of automation lanes. Items only send edits; the core decides (TRK-3: loops are
// refused there too, the pickers just do not offer them).
import type { Backend } from "../backend/types";
import type { ParamSpec } from "../bindings/ParamSpec";
import type { Project } from "../bindings/Project";
import type { Role } from "../bindings/Role";
import type { Track } from "../bindings/Track";
import { Icon } from "../shared/Icon";
import { useMenu, type MenuItem } from "../shared/Menu";
import type { ViewState } from "../state/store";
import { useApp, useBackend } from "../state/context";
import type { useEdit } from "./bind";
import { outputChoices } from "./geometry";
import type { FxBlock } from "../bindings/FxBlock";
import type { FxKind } from "../bindings/FxKind";
import { INFO, SPEC_LABEL, blockName, kindOfId, specKey } from "./fx";

type EditFn = ReturnType<typeof useEdit>;

export const ROLES: Role[] = [
  "kick", "snare", "toms", "overheads", "room", "drums", "percussion", "bass", "electric_guitar", "acoustic_guitar", "keys",
  "synth", "pad", "strings", "brass", "lead_vocal", "backing_vocal", "fx", "other",
];
export const roleLabel = (r: Role | null) => (r ?? "other").replace(/_/g, " ");

/** Human name of an automatable parameter on a track. */
export function paramLabel(p: Project, id: string): string {
  const dot = id.indexOf(".");
  const a = dot < 0 ? id : id.slice(0, dot);
  const b = dot < 0 ? "" : id.slice(dot + 1);
  if (id === "fader") return "Fader";
  if (id === "pan") return "Pan";
  if (id === "trim") return "Trim";
  if (id === "mute") return "Mute";
  if (a === "send") return `Send · ${p.tracks.find((t) => t.id === id.slice(5))?.name ?? "return"}`;
  if (a === "rev") return `Reverb ${{ size: "size", damping: "damping", predelay: "pre-delay", hpf: "low cut", lpf: "high cut", shimmer: "shimmer" }[b] ?? b}`;
  if (a === "dly") return `Delay ${{ time: "time", feedback: "feedback", fb_hpf: "feedback low cut", fb_lpf: "feedback high cut", wow: "wow & flutter", drive: "drive" }[b] ?? b}`;
  if (a === "bypass") {
    if (b === "fx") return "Bypass · effect";
    const k = kindOfId(b);
    return `Bypass · ${k ? instName(b, k).toLowerCase().replace("eq", "EQ") : b}`;
  }
  const k = kindOfId(a);
  if (!k) return id;
  // The filters read as they always did: "Low cut", "High cut".
  if ((k === "hpf" || k === "lpf") && b === "freq") return `${k === "hpf" ? "Low cut" : "High cut"}${/\d$/.test(a) ? ` ${a.replace(/\D+/, "")}` : ""}`;
  const m = /^(\d+)\.(.+)$/.exec(b);
  const x = /^xover\.(\d+)$/.exec(b);
  const spec = SPEC_LABEL.get(specKey(id).replace(/\.\d+\./, ".1.").replace(/xover\.\d+$/, "xover.1")) ?? b;
  if (k === "eq" && m) return `EQ ${a === "eq" ? m[1] : `${a.slice(2)}·${m[1]}`} · ${{ freq: "frequency", gain: "gain", q: "width" }[m[2]] ?? m[2]}`;
  if (x) return `${instName(a, k)} · crossover ${x[1]}`;
  if (m) return `${instName(a, k)} · ${k === "harmonizer" ? "voice" : "band"} ${m[1]} ${spec.toLowerCase()}`;
  return `${instName(a, k)} ${spec.toLowerCase()}`;
}

/** A block's kind label, numbered from its id when it is the second of its kind (`comp2` → "Compressor 2"). */
function instName(id: string, k: FxKind) {
  const n = /\d+$/.exec(id)?.[0];
  const label = k === "compressor" ? "Comp" : INFO[k].label;
  return n ? `${label} ${n}` : label;
}

/** Automatable parameters of a track, grouped as in the design's picker (AUT-1): one group per block of its chain. */
export function paramGroups(p: Project, t: Track): { label: string; ids: string[] }[] {
  const returns = p.tracks.filter((x) => x.kind === "return");
  const blocks = t.chain.blocks.map((b) => ({ label: blockName(t.chain, b.id), ids: blockParams(b) }));
  const bypass = { label: "Bypass", ids: t.chain.blocks.map((b) => `bypass.${b.id}`) };
  if (t.kind === "return") {
    const fx = t.ret?.type === "reverb" ? ["rev.decay", "rev.size", "rev.damping", "rev.modulation", "rev.predelay", "rev.hpf", "rev.lpf", "rev.shimmer"] : ["dly.time", "dly.feedback", "dly.fb_hpf", "dly.fb_lpf", "dly.wow", "dly.drive"];
    return [
      { label: "", ids: ["fader", "pan"] },
      { label: t.ret?.type === "reverb" ? "Reverb" : "Delay", ids: fx },
      ...blocks,
      { label: "", ids: ["mute", "bypass.fx"] },
      ...(bypass.ids.length ? [bypass] : []),
    ];
  }
  return [{ label: "", ids: ["fader", "pan", "trim"] }, { label: "Sends", ids: returns.map((r) => `send.${r.id}`) }, ...blocks, { label: "", ids: ["mute"] }, bypass];
}

/** The continuous parameters of a block as it is (its bands and voices), in catalog order. */
export function blockParams(b: FxBlock): string[] {
  const info = INFO[b.fx.type];
  const n = b.fx.type === "eq" || b.fx.type === "dynamic_eq" || b.fx.type === "multiband" ? b.fx.bands.length : b.fx.type === "harmonizer" ? b.fx.voices.length : 0;
  const out: string[] = [];
  for (const q of info.params) {
    if (q.key === "sc_hpf" && !(b.fx.type === "compressor" && b.fx.sidechain)) continue;
    if (!q.indexed) out.push(`${b.id}.${q.key}`);
  }
  for (let i = 1; i <= n; i++) for (const q of info.params) if (q.indexed && q.key.startsWith("<n>.")) out.push(`${b.id}.${q.key.replace("<n>", String(i))}`);
  if (b.fx.type === "multiband") for (let i = 1; i < b.fx.bands.length; i++) out.push(`${b.id}.xover.${i}`);
  return out;
}

function paramItems(p: Project, t: Track, isOn: (id: string) => boolean, pick: (id: string) => void): MenuItem[] {
  const out: MenuItem[] = [];
  for (const g of paramGroups(p, t)) {
    if (!g.ids.length) continue;
    const items: MenuItem[] = g.ids.map((id) => ({ label: g.label ? paramLabel(p, id).replace(/^(EQ [\d·]+ · |Comp \d* ?|Bypass · |Reverb |Delay |Send · )/, "").replace(new RegExp(`^${g.label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")} (· )?`), "") : paramLabel(p, id), checked: isOn(id), onSelect: () => pick(id) }));
    if (g.label) out.push({ label: g.label, checked: g.ids.some(isOn), sub: items });
    else out.push(...items);
  }
  return out;
}

/** The picker of a lane's parameter (7d): showing another parameter swaps the visible lane; nothing is lost. */
export function paramMenu(p: Project, t: Track, current: string, _specs: Map<string, ParamSpec>, edit: EditFn, backend: Backend, fail: (e: unknown) => void): MenuItem[] {
  const lane = t.automation.find((l) => l.param === current);
  return [
    { header: `Automate on ${t.name}` },
    ...paramItems(p, t, (id) => id === current, (id) => {
      if (id === current) return;
      void backend.setLaneVisible(t.id, current, false).then(() => backend.setLaneVisible(t.id, id, true)).catch(fail);
    }),
    "-",
    {
      label: "Delete this automation",
      icon: "trash",
      danger: true,
      disabled: !lane?.points.length,
      onSelect: () => void edit({ op: "remove_lane", track: t.id, param: current }, `${t.name} · ${paramLabel(p, current)} automation removed`),
    },
  ];
}

interface TrackMenuCtx {
  edit: EditFn;
  backend: Backend;
  setView: (v: Partial<ViewState>) => void;
  folded: string[];
  setError: (e: string | null) => void;
  /** The tracks selected together (⌘-click, ⇧-click). */
  selTracks: string[];
}

/** The tracks a new group takes: the selection (in track order, returns left out), or the one given. */
export function groupMembers(p: Project, selTracks: string[], one: string | null): string[] {
  const sel = p.tracks.filter((t) => selTracks.includes(t.id) && t.kind !== "return").map((t) => t.id);
  if (one && !sel.includes(one)) return p.tracks.some((t) => t.id === one && t.kind !== "return") ? [one] : [];
  return sel;
}

/** One history step: a group made from the tracks given. */
export function groupTracks(p: Project, members: string[], edit: EditFn) {
  const names = members.map((id) => p.tracks.find((t) => t.id === id)?.name ?? id);
  return edit({ op: "add_group", name: "Group", members }, members.length === 1 ? `Group ${names[0]}` : `Group ${members.length} tracks`);
}

/** Context menu of a track header (6e). */
export function trackMenu(p: Project, t: Track, ctx: TrackMenuCtx): MenuItem[] {
  const { edit, backend, setView } = ctx;
  const fail = (e: unknown) => ctx.setError(String(e));
  const items: MenuItem[] = [
    { label: "Rename…", onSelect: () => setView({ renameTrack: t.id, selTrack: t.id }) },
    {
      label: "Colour",
      sub: Array.from({ length: 12 }, (_, i) => ({
        label: `Colour ${i + 1}`,
        checked: t.color === i + 1,
        custom: undefined,
        onSelect: () => void edit({ op: "rename_track", track: t.id, name: t.name, color: i + 1 }, `${t.name} · colour`),
      })),
    },
  ];
  if (t.kind === "audio") {
    items.push({
      label: `Role · ${roleLabel(t.role)}`,
      sub: ROLES.map((r) => ({ label: roleLabel(r), checked: t.role === r, onSelect: () => void edit({ op: "set_role", track: t.id, role: r }, `${t.name} · role ${roleLabel(r)}`) })),
    });
  }
  if (t.kind !== "return") {
    items.push({ label: `Output · ${outName(p, t)}`, sub: outputItems(p, t, edit) });
  }
  items.push(
    "-",
    ...(t.kind !== "return" ? [{ label: "Lock for AI", icon: "lock" as const, checked: t.locked, onSelect: () => void edit({ op: "set_locked", track: t.id, locked: !t.locked }, `${t.name} · ${t.locked ? "unlocked" : "locked"} for the assistant`) }] : []),
    {
      label: "Show automation",
      sub: paramItems(p, t, (id) => t.automation.some((l) => l.param === id && l.visible), (id) => {
        const vis = t.automation.some((l) => l.param === id && l.visible);
        void backend.setLaneVisible(t.id, id, !vis).catch(fail);
      }),
    },
    "-",
    { label: "Duplicate track", onSelect: () => void edit({ op: "duplicate_track", track: t.id }, `Duplicate ${t.name}`) },
  );
  if (t.kind !== "return") {
    const members = groupMembers(p, ctx.selTracks, t.id);
    items.push({
      label: members.length > 1 ? `Group ${members.length} selected tracks` : "Group this track",
      icon: "group",
      kbd: "⌘⇧G",
      onSelect: () => void groupTracks(p, members, edit),
    });
  }
  items.push("-", {
    label: "Delete track…",
    icon: "trash",
    danger: true,
    onSelect: () =>
      setView({
        dialog: {
          title: `Delete “${t.name}”?`,
          sub: t.kind === "group" ? "Its tracks go to the group's output. You can undo this." : "Its clips, sends and automation go with it. The audio files stay in the project. You can undo this.",
          confirm: "Delete track",
          danger: true,
          onConfirm: () => void edit({ op: "delete_track", track: t.id }, `Delete ${t.name}`),
        },
      }),
  });
  return items;
}

export function outName(p: Project, t: Track): string {
  return t.output.kind === "group" ? (p.tracks.find((g) => g.id === (t.output as { id: string }).id)?.name ?? "Group") : "Mix";
}

/** Output picker (7c): the mix bus and the groups that would not create a loop. */
export function outputItems(p: Project, t: Track, edit: EditFn): MenuItem[] {
  const cur = t.output.kind === "group" ? t.output.id : null;
  return [
    { header: `Send the output of ${t.name} to` },
    { label: "Mix bus", checked: cur === null, onSelect: () => void edit({ op: "set_output", track: t.id, output: { kind: "mix_bus" } }, `${t.name} · output Mix`) },
    ...outputChoices(p, t).map((g) => ({
      label: g.name,
      checked: cur === g.id,
      onSelect: () => void edit({ op: "set_output", track: t.id, output: { kind: "group" as const, id: g.id } }, `${t.name} · output ${g.name}`),
    })),
    "-",
    { label: "New group with this track", icon: "plus", onSelect: () => void edit({ op: "add_group", name: "Group", members: [t.id] }, `Group ${t.name}`) },
    { header: "Groups nest up to 4 levels. A choice that would create a loop is not offered." },
  ];
}

/**
 * Sidechain source picker (7d): another track or group, never one fed by this track (FX-3). `block` is the listening
 * block (a compressor, a gate, a dynamic EQ and its `band`); the first compressor if absent.
 */
export function sidechainItems(p: Project, t: Track, edit: EditFn, block?: string, band?: number): MenuItem[] {
  const blk = block ? t.chain.blocks.find((b) => b.id === block) : t.chain.blocks.find((b) => b.fx.type === "compressor");
  const fx = blk?.fx;
  const cur = !fx ? null : fx.type === "compressor" ? (fx.sidechain?.source ?? null) : fx.type === "gate" ? fx.sidechain : fx.type === "dynamic_eq" ? (fx.bands[band ?? 0]?.sidechain ?? null) : null;
  const who = fx?.type === "gate" ? "Gate" : fx?.type === "dynamic_eq" ? "The band" : "Compressor";
  const at = { block: blk?.id, band };
  const fedBy = (x: Track): boolean => {
    // x is fed by t if t routes (directly or through groups) into x.
    let o = t.output;
    for (let i = 0; i < 16 && o.kind === "group"; i++) {
      const id: string = o.id;
      if (id === x.id) return true;
      o = p.tracks.find((g) => g.id === id)?.output ?? { kind: "mix_bus" };
    }
    return false;
  };
  const sources = p.tracks.filter((x) => x.kind !== "return" && x.id !== t.id && !fedBy(x));
  return [
    { header: `${who} listens to` },
    { label: "None (this track)", checked: cur === null, onSelect: () => void edit({ op: "set_sidechain", track: t.id, source: null, ...at }, `${t.name} · sidechain off`) },
    ...sources.map((x) => ({
      label: x.kind === "group" ? `${x.name} · group` : x.name,
      checked: cur === x.id,
      onSelect: () => void edit({ op: "set_sidechain", track: t.id, source: x.id, ...at }, `${t.name} · sidechain ${x.name}`),
    })),
    { header: `Post-fader signal. ${t.name} itself and anything fed by it are not listed.` },
  ];
}

/** "Add track" in the timeline toolbar (7b). */
export function AddMenu() {
  const project = useApp((s) => s.project)!;
  const backend = useBackend();
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const selTrack = useApp((s) => s.selTrack);
  const selTracks = useApp((s) => s.selTracks);
  const menu = useMenu();
  const returns = project.tracks.filter((t) => t.kind === "return").length;
  const edit = (e: Parameters<Backend["edit"]>[0], label: string) => backend.edit(e, label).catch((x) => setError(String(x)));
  return (
    <>
      <button
        type="button"
        className="btn quiet sm"
        onClick={(e) =>
          menu.open(e.currentTarget, [
            { header: "Add to the project" },
            { label: "Audio track from file…", icon: "file", onSelect: () => setView({ screen: "new" }) },
            {
              label: "Group",
              icon: "group",
              kbd: "⌘⇧G",
              onSelect: () => {
                const members = groupMembers(project, selTracks, selTrack);
                void (members.length ? groupTracks(project, members, edit) : edit({ op: "add_group", name: "Group", members }, "Add group"));
              },
            },
            "-",
            { label: "Reverb return", icon: "wave", disabled: returns >= 8, onSelect: () => void edit({ op: "add_return", reverb: true }, "Add reverb return") },
            { label: "Delay return", icon: "wave", disabled: returns >= 8, onSelect: () => void edit({ op: "add_return", reverb: false }, "Add delay return") },
            { header: `${returns} of 8 returns in use` },
          ], "Add")
        }
      >
        <Icon name="plus" size="m" />
        Add track
      </button>
      {menu.node}
    </>
  );
}
