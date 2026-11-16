// Clip waveform from the precomputed peak pyramids (TRN-3): the backend returns min/max per column; the UI only
// draws them. Requests are cached per (media, span, columns) and the column count follows the zoom in powers of two.
import { memo, useEffect, useMemo, useState } from "react";
import { useBackend } from "../state/context";

const cache = new Map<string, [number, number][]>();
const pending = new Map<string, Promise<[number, number][]>>();

function columnsFor(width: number) {
  const want = Math.max(16, Math.min(2048, width / 2));
  return 2 ** Math.ceil(Math.log2(want));
}

/** The outline of a clip's peaks as an SVG path: one column per unit of x. */
export function wavePath(peaks: [number, number][], height: number): string {
  const mid = height / 2;
  const up: string[] = [];
  const dn: string[] = [];
  for (let i = 0; i < peaks.length; i++) {
    up.push(`${i} ${(mid - Math.max(0.5, Math.abs(peaks[i][1]) * (mid - 1))).toFixed(1)}`);
    dn.push(`${i} ${(mid + Math.max(0.5, Math.abs(peaks[i][0]) * (mid - 1))).toFixed(1)}`);
  }
  return `M${up.join("L")}L${dn.reverse().join("L")}Z`;
}

// The timeline renders again at every playhead update (NFR-3): a clip's waveform must cost nothing then. Up to 2048
// columns per clip were turned into a path string at each of those renders; now only when the peaks or the height
// change, and the component itself is skipped while its props are the same.
export const Waveform = memo(function Waveform({
  media, offset, length, width, height, onLoading,
}: { media: string; offset: number; length: number; width: number; height: number; onLoading?: (loading: boolean) => void }) {
  const backend = useBackend();
  const cols = columnsFor(width);
  const key = `${media}:${offset}:${length}:${cols}`;
  const [peaks, setPeaks] = useState<[number, number][] | null>(() => cache.get(key) ?? null);
  // 13c: the overview may still be building after opening; asked again when the file is ready.
  const [ready, setReady] = useState(0);
  useEffect(() => backend.onMediaReady((m) => m === media && setReady((r) => r + 1)), [backend, media]);
  // 13c: the clip shows its loading state until the overview is there.
  useEffect(() => onLoading?.(!peaks), [peaks, onLoading]);
  useEffect(() => {
    const hit = cache.get(key);
    if (hit) {
      setPeaks(hit);
      return;
    }
    let alive = true;
    let p = pending.get(key);
    if (!p) {
      p = backend.peaks(media, offset, offset + length, cols).then((ch) => {
        // Channels merged for drawing: the widest extent per column.
        const n = ch[0]?.length ?? 0;
        const out: [number, number][] = [];
        for (let i = 0; i < n; i++) {
          let lo = 0;
          let hi = 0;
          for (const c of ch) {
            lo = Math.min(lo, c[i][0]);
            hi = Math.max(hi, c[i][1]);
          }
          out.push([lo, hi]);
        }
        cache.set(key, out);
        if (cache.size > 2000) cache.delete(cache.keys().next().value!);
        return out;
      });
      pending.set(key, p);
      void p.finally(() => pending.delete(key));
    }
    p.then((v) => alive && setPeaks(v)).catch(() => {});
    return () => {
      alive = false;
    };
  }, [backend, key, media, offset, length, cols, ready]);
  const d = useMemo(() => (peaks && height > 2 ? wavePath(peaks, height) : ""), [peaks, height]);
  if (!peaks || height <= 2 || width <= 1) return null;
  const n = peaks.length;
  return (
    <svg className="wf" width={width} height={height} viewBox={`0 0 ${Math.max(1, n - 1)} ${height}`} preserveAspectRatio="none" aria-hidden="true">
      <path d={d} />
    </svg>
  );
});
