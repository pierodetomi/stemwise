// File system paths as the backend hands them over: `/` on macOS and Linux, `\` (or both) on Windows.

const SEP = /[\\/]/;

/** The last component of a path: the file or folder name. */
export const baseName = (path: string): string => {
  const parts = path.split(SEP).filter((p) => p !== "");
  return parts[parts.length - 1] ?? path;
};

/** The folder that holds the last component, without a trailing separator (a root keeps its own). */
export const parentDir = (path: string): string => {
  const trimmed = path.replace(/[\\/]+$/, "");
  const at = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  if (at < 0) return "";
  const parent = trimmed.slice(0, at);
  // `/x` → `/`, `C:\x` → `C:\`
  if (parent === "") return trimmed[at];
  if (/^[A-Za-z]:$/.test(parent)) return parent + trimmed[at];
  return parent;
};
