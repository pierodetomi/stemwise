// Proposals received earlier (AI-23): the Proposals tab of the assistant in Mix and in Master. Every call that came
// back with a proposal stays listed, whatever happened to it; "Review" puts it back in review on the mix (or the
// master) as it is now, where it is listened to, applied after a confirmation, or closed. Nothing changes before.
import type { AiCallRecord } from "../bindings/AiCallRecord";
import { Icon } from "../shared/Icon";
import type { DialogState } from "../state/store";
import { useApp, useBackend } from "../state/context";
import { OUTCOME, short } from "./Usage";

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const two = (n: number) => String(n).padStart(2, "0");

/** When a proposal was received, as the list and the review show it. */
export function received(ms: number): string {
  const d = new Date(ms);
  return `${d.getDate()} ${MONTHS[d.getMonth()]} ${d.getFullYear()}, ${two(d.getHours())}:${two(d.getMinutes())}`;
}

/** The calls that came back with a proposal, newest first. */
export function proposalCalls(calls: AiCallRecord[], phase: "mix" | "master"): AiCallRecord[] {
  const kinds = phase === "mix" ? ["auto_mix", "revision"] : ["master", "master_revision"];
  return calls
    .filter((c) => kinds.includes(c.kind) && (c.outcome === "applied" || c.outcome === "applied_partly" || c.outcome === "discarded"))
    .sort((a, b) => b.started_ms - a.started_ms);
}

/** In the review of a proposal opened again: what it is shown against, and how old its numbers are. */
export function ReopenedNote({ at, phase }: { at: number; phase: "mix" | "master" }) {
  return (
    <div className="banner" role="note" data-testid="reopened-note">
      <Icon name="clock" />
      <div className="col grow">
        <span className="b">Received {received(at)}</span>
        <span className="dim s1">
          The changes are shown against your {phase} as it is now. {phase === "mix" ? "The measurements are those taken when it was received." : "It was measured again on the current mixdown."}
        </span>
      </div>
    </div>
  );
}

/** The confirmation before a proposal received earlier changes the current mix or master. */
export function reapplyDialog(at: number, phase: "mix" | "master", part: boolean, onConfirm: () => void): DialogState {
  return {
    title: `Apply this proposal to your current ${phase}?`,
    sub:
      `It was received on ${received(at)} and your ${phase} may have changed since. Applying it replaces the current settings of ` +
      `${part ? "the parts you ticked" : phase === "mix" ? "every track it touches" : "every module it sets"} with the proposal's. ` +
      "It is one step in the history: undo brings everything back.",
    confirm: part ? "Apply selected" : "Apply all",
    onConfirm,
  };
}

export function ProposalsTab({ phase, busy }: { phase: "mix" | "master"; busy?: string | null }) {
  const backend = useBackend();
  const calls = useApp((s) => s.project)!.ai.calls;
  const setView = useApp((s) => s.setView);
  const setError = useApp((s) => s.setError);
  const list = proposalCalls(calls, phase);
  const reopen = (id: string) => void (phase === "mix" ? backend.aiReopen(id) : backend.masterAiReopen(id)).catch((e) => setError(String(e)));
  return (
    <div className="sect grow scroll" data-testid="proposals">
      {list.length === 0 ? (
        <div className="empty">
          <Icon name="spark" />
          <span className="b">No proposals yet</span>
          <span className="s1">
            Every proposal the assistant sends stays here, applied or not: you can review it and apply it later.
          </span>
        </div>
      ) : (
        <span className="faint s1">
          Proposals you received, newest first. Review one to hear it against your {phase} as it is now; nothing changes until you apply it.
        </span>
      )}
      <div className="col g2" role="list" aria-label="Proposals received">
        {list.map((c) => {
          const [label, cls] = OUTCOME[c.outcome];
          const title = c.request ? `“${c.request}”` : phase === "mix" ? "Auto mix" : "Master with AI";
          const why = !c.stored ? "Received before proposals were kept with the project: only its details remain." : busy ?? undefined;
          return (
            <div key={c.id} className="tile" role="listitem" aria-label={`${title}, ${received(c.started_ms)}`}>
              <div className="fx g2">
                <span className="b grow ell" title={title}>{title}</span>
                <span className={`tag ${cls}`}>{label}</span>
              </div>
              <span className="faint s1">{received(c.started_ms)} · {short(c.model)}</span>
              <div className="fx between">
                <button type="button" className="link s1" onClick={() => setView({ screen: "usage", usageCall: c.id })}>Details</button>
                <button type="button" className={`btn sm ${why ? "is-dis" : ""}`} disabled={!!why} title={why} aria-label={`Review ${title}`} onClick={() => reopen(c.id)}>
                  Review…
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
