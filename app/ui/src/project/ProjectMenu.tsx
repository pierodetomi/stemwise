// The app menu (UI-20): the project name in the top bar (Mix and Master) opens every command of Stemwise — new and
// open, the project's own commands, Settings, and Help with the guided tour. The macOS menu bar keeps only what
// macOS provides.
import { Icon } from "../shared/Icon";
import { useMenu } from "../shared/Menu";
import { keysOf } from "../shared/shortcuts";
import { useApp } from "../state/context";
import { TOUR_TEXT } from "../tour/text";
import { useTakeTour } from "../tour/useTour";
import { useProjectCommands } from "./commands";

export function ProjectMenuButton() {
  const project = useApp((s) => s.project)!;
  const dirty = useApp((s) => s.dirty);
  const demo = useApp((s) => s.demo);
  const setView = useApp((s) => s.setView);
  const openSettings = useApp((s) => s.openSettings);
  const cmd = useProjectCommands();
  const tour = useTakeTour();
  const menu = useMenu();
  return (
    <>
      <button
        type="button"
        className="pname h nowrap"
        data-testid="project-name"
        title="Menu: projects, settings, help"
        aria-label={`${project.name} · app menu`}
        aria-haspopup="menu"
        onClick={(e) =>
          menu.open(
            e.currentTarget,
            [
              { label: "New project…", kbd: keysOf("newProject"), onSelect: () => cmd.newProject() },
              { label: "Open…", kbd: keysOf("openProject"), onSelect: () => cmd.open() },
              "-",
              { label: "Project settings…", onSelect: () => setView({ projectOpen: true }) },
              { label: dirty && !demo ? "Save" : "Save (no changes)", kbd: keysOf("save"), disabled: demo, onSelect: () => void cmd.save() },
              { label: "Save as…", kbd: keysOf("saveAs"), disabled: demo, onSelect: () => void cmd.saveAs() },
              { label: "Close project", onSelect: () => cmd.close() },
              "-",
              { label: "Settings…", kbd: keysOf("settings"), onSelect: () => openSettings() },
              "-",
              { label: TOUR_TEXT.menuHelp, sub: [{ label: TOUR_TEXT.menuItem, icon: "play", disabled: tour.busy, onSelect: tour.take }] },
            ],
            "App menu",
          )
        }
      >
        <span className="ell">{project.name}</span>
        <Icon name="chev" size="sm" />
      </button>
      {menu.node}
    </>
  );
}
