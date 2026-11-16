// Project commands (PRJ-2): new, open, save, save as, close. One place for the app menu in the top bar (UI-20) and
// the shortcuts, so they all ask the same questions (where, whether to save first).
import { useMemo } from "react";
import { useBackend, useStore } from "../state/context";
import { endTour } from "../tour/useTour";

export function useProjectCommands() {
  const backend = useBackend();
  const store = useStore();
  return useMemo(() => {
    const s = () => store.getState();
    const fail = (e: unknown) => s().setError(String(e));
    /** Opens a project (asking to recover changes from a session that ended badly). */
    const openPath = async (path: string | null) => {
      if (!path) return;
      try {
        const recover = await backend.openProject(path);
        if (recover) {
          s().setView({
            dialog: {
              title: "Recover unsaved changes?",
              sub: "Stemwise closed while this project had changes that were not saved. They were kept in an automatic copy.",
              confirm: "Recover changes",
              onConfirm: () => void backend.recoverProject(true).catch(fail),
            },
          });
        }
      } catch (e) {
        fail(e);
      }
    };
    const save = async () => {
      const p = s().project;
      if (!p) return;
      try {
        await backend.saveProject();
        s().toast("ok", "Saved", `“${p.name}” is saved.`);
      } catch (e) {
        fail(e);
      }
    };
    const saveAs = async () => {
      const p = s().project;
      if (!p) return;
      try {
        const at = await backend.pickSaveLocation(`${p.name} copy`);
        if (!at || !at.name.trim()) return;
        await backend.saveProjectAs(at.parent, at.name.trim());
        s().toast("ok", "Saved as a new project", `“${at.name.trim()}” is now open. “${p.name}” stays as it was.`);
      } catch (e) {
        fail(e);
      }
    };
    /** Closes the open project, asking first when it has unsaved changes; then `next` (another project, a new one). */
    const close = (next?: () => void) => {
      const st = s();
      // During the tour, New, Open and Close end it first (UI-18): the demo is never saved, so nothing is asked.
      if (st.tour || st.demo) {
        void endTour(store, "quiet").then(() => next?.());
        return;
      }
      const p = st.project;
      const done = async () => {
        try {
          await backend.closeProject();
          s().closeProject();
          next?.();
        } catch (e) {
          fail(e);
        }
      };
      if (!p) {
        next?.();
        return;
      }
      if (!st.dirty) {
        void done();
        return;
      }
      st.setView({
        dialog: {
          title: `Save changes to “${p.name}”?`,
          sub: "If you don’t save, the changes since the last save are lost. The automatic copy is removed when the project closes.",
          confirm: "Save",
          onConfirm: () => void backend.saveProject().then(done).catch(fail),
          alt: { label: "Don’t save", danger: true, onSelect: () => void done() },
        },
      });
    };
    const open = () =>
      close(async () => {
        await openPath(await backend.pickProject());
      });
    const newProject = () => close(() => s().setView({ screen: "new" }));
    return { save, saveAs, close, open, openPath, newProject };
  }, [backend, store]);
}
