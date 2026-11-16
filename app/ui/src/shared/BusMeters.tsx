// The output meters of the top bar (design 3a, 9a): left and right level bars, true peak, short-term loudness and
// engine load. The mix bus in Mix, the master output in Master. Every value has a fixed slot, so nothing beside them
// moves while playing.
import { useBallistics } from "./controls";
import { meterPos } from "./format";
import { useApp } from "../state/context";

function Bar({ db, peak, side }: { db: number; peak: number; side: string }) {
  const b = useBallistics(db, peak);
  return (
    <div className="hbar" data-testid={`bus-bar-${side}`}>
      <i style={{ width: `${meterPos(b.level) * 100}%` }} />
    </div>
  );
}

export function BusMeters({ label, engine = true }: { label: string; engine?: boolean }) {
  const meters = useApp((s) => s.meters);
  const playing = meters?.playing ?? false;
  const bus = meters?.bus;
  const live = (v: number | undefined) => (playing && v !== undefined && v > -120 ? v : null);
  const tp = live(bus?.[4]);
  const lufs = live(bus?.[6]);
  const n = (v: number | null) => (v === null ? "−∞" : v.toFixed(1).replace("-", "−"));
  return (
    <div className="fx g3 busm" aria-label={label} data-testid="bus-meters">
      <div className="col g1 bus-bars">
        <Bar db={bus?.[2] ?? -200} peak={bus?.[0] ?? -200} side="l" />
        <Bar db={bus?.[3] ?? -200} peak={bus?.[1] ?? -200} side="r" />
      </div>
      <div className="col">
        <span className="faint s1">True peak</span>
        <span className={`b v tp ${tp !== null && tp > -3 ? "warnc" : ""}`} data-testid="bus-tp">{n(tp)} dBTP</span>
      </div>
      <div className="col lu-col">
        <span className="faint s1">Loudness</span>
        <span className="b v lu">{n(lufs)} LUFS</span>
      </div>
      {engine && (
      <div className="col">
        <span className="faint s1">Engine</span>
        <span className="b v ld" data-testid="engine-load">{Math.round((meters?.load ?? 0) * 100)} %</span>
      </div>
      )}
    </div>
  );
}
