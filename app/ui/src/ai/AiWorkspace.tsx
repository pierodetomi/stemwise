// The Mix workspace with the assistant wired in (F7): the request foot, the call in progress, the proposal in review
// in place of inspector and assistant, the chip in the top bar, snapshots, brief and key dialogs.
import { Icon } from "../shared/Icon";
import { useApp } from "../state/context";
import { Workspace } from "../mix/Workspace";
import { AskFoot, Working } from "./Assist";
import { ProposalsTab } from "./Proposals";
import { ProposalPanel } from "./Proposal";
import { Snapshots } from "./Snapshots";

export function AiWorkspace({ onMaster }: { onMaster: () => void }) {
  const ai = useApp((s) => s.ai);
  const setView = useApp((s) => s.setView);
  const running = ai?.running ?? null;
  const p = running?.progress;
  const frac = !p ? 0 : p.phase === "analysis" ? 0.1 * p.fraction : 0.1 + (0.9 * (p.iteration - 1 + 0.5)) / p.of;
  const mid = ai?.proposal ? (
    <span className="tag ai">
      <Icon name="spark" size="sm" />
      Reviewing a proposal
    </span>
  ) : running ? (
    <div className="fx g2" style={{ width: "var(--field-w-l)" }}>
      <span className="tag ai">
        <Icon name="spark" size="sm" />
        {running.kind === "revision" ? "Revision" : "Auto mix"}
        {p && p.phase !== "analysis" ? ` · pass ${p.iteration} of ${p.of}` : " · analysis"}
      </span>
      <div className="grow">
        <div className="progress ai">
          <i style={{ width: `${Math.round(frac * 100)}%` }} />
        </div>
      </div>
    </div>
  ) : null;
  return (
    <Workspace
      onMaster={onMaster}
      mid={mid}
      side={ai?.proposal ? <ProposalPanel p={ai.proposal} /> : undefined}
      working={running && ai ? <Working ai={ai} /> : undefined}
      assistantFooter={<AskFoot onKey={() => setView({ setup: "connect" })} />}
      snapshots={<Snapshots />}
      proposals={<ProposalsTab phase="mix" busy={running ? "The assistant is working" : null} />}
    />
  );
}
