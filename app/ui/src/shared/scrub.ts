// Scrubbing the playhead (TRN-1): drag it and the position follows the pointer. Playing, the sound follows (each
// move is a locate, at most every 33 ms); stopped, only the position moves. The playhead is drawn at the pointer at
// once, without waiting for the engine's next position.
import { useCallback, useRef, useState } from "react";
import { useApp, useBackend } from "../state/context";

const EVERY_MS = 33;

export function useScrub(toFrame: (clientX: number) => number) {
  const backend = useBackend();
  const setError = useApp((s) => s.setError);
  const [frame, setFrame] = useState<number | null>(null);
  const to = useRef(toFrame);
  to.current = toFrame;
  const start = useCallback(
    (e: { clientX: number; preventDefault(): void }) => {
      e.preventDefault();
      const fail = (x: unknown) => setError(String(x));
      let last = 0;
      let pending: number | null = null;
      let timer: number | null = null;
      const send = (f: number) => {
        last = performance.now();
        void backend.locate(f).catch(fail);
      };
      const at = (x: number) => {
        const f = Math.max(0, Math.round(to.current(x)));
        setFrame(f);
        const wait = EVERY_MS - (performance.now() - last);
        if (wait <= 0) send(f);
        else {
          pending = f;
          timer ??= window.setTimeout(() => {
            timer = null;
            if (pending !== null) send(pending);
            pending = null;
          }, wait);
        }
        return f;
      };
      at(e.clientX);
      const move = (ev: PointerEvent) => void at(ev.clientX);
      const up = (ev: PointerEvent) => {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", up);
        if (timer !== null) window.clearTimeout(timer);
        send(Math.max(0, Math.round(to.current(ev.clientX))));
        // The engine's position takes over once it has the last one.
        window.setTimeout(() => setFrame(null), 120);
      };
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", up);
    },
    [backend, setError],
  );
  return { frame, start, scrubbing: frame !== null };
}
