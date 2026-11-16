import { createContext, useContext } from "react";
import { useShallow } from "zustand/react/shallow";
import type { AppState, AppStore } from "./store";

export const StoreContext = createContext<AppStore | null>(null);

export function useApp<T>(sel: (s: AppState) => T): T {
  const store = useContext(StoreContext);
  if (!store) throw new Error("StoreContext missing");
  return store(sel);
}

export function useBackend() {
  return useApp((s) => s.backend);
}

/** A selector returning a fresh object or array, compared field by field. */
export function useAppShallow<T extends object>(sel: (s: AppState) => T): T {
  const store = useContext(StoreContext);
  if (!store) throw new Error("StoreContext missing");
  return store(useShallow(sel));
}

export function useStore() {
  const store = useContext(StoreContext);
  if (!store) throw new Error("StoreContext missing");
  return store;
}
