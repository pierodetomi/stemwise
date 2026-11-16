#!/usr/bin/env node
// npm licence gate (NFR-6): production dependencies must use a permissive licence; no dependency at all
// (dev included) may be GPL/LGPL/AGPL.
import { execSync } from "node:child_process";

const allowed = new Set(["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "MPL-2.0", "0BSD",
  "Unicode-3.0", "Unicode-DFS-2016", "OFL-1.1", "CC0-1.0", "BlueOak-1.0.0", "Python-2.0", "CC-BY-4.0"]);
const ok = (lic) => lic.split(/\s+OR\s+|[()]/).map((s) => s.trim()).filter(Boolean).some((l) => allowed.has(l))
  || lic.split(/\s+AND\s+/).every((l) => allowed.has(l.replace(/[()]/g, "").trim()));
const copyleft = (lic) => /\b(A|L)?GPL/i.test(lic) && !ok(lic);
const list = (flag) => JSON.parse(execSync(`pnpm licenses list --json ${flag}`, { cwd: new URL("../ui", import.meta.url), encoding: "utf8" }));

let failed = false;
for (const [lic, pkgs] of Object.entries(list("--prod"))) {
  if (!ok(lic)) { failed = true; console.error(`not permissive (prod): ${lic}: ${pkgs.map((p) => p.name).join(", ")}`); }
}
for (const [lic, pkgs] of Object.entries(list("--dev"))) {
  if (copyleft(lic)) { failed = true; console.error(`copyleft (dev): ${lic}: ${pkgs.map((p) => p.name).join(", ")}`); }
}
if (failed) process.exit(1);
console.log("npm licences: ok");
