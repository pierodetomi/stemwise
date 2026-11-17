#!/usr/bin/env node
// Requirement traceability (FOUNDATION §8.3): lists the ids of §3 with no test citing them.
//   node scripts/reqs.mjs            -> report every uncovered id (exit 0)
//   node scripts/reqs.mjs --closed   -> fail if an id of a closed phase is neither tested nor justified
import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const app = join(dirname(fileURLToPath(import.meta.url)), "..");
const repo = join(app, "..");
const foundation = readFileSync(join(repo, "docs/FOUNDATION.md"), "utf8");

// 1. Requirement ids of §3 (ranges such as "ANA-1 → ANA-12" are expanded).
const section3 = foundation.slice(foundation.indexOf("## 3. Functional requirements"), foundation.indexOf("## 4. Technology stack"));
const ids = new Set();
for (const m of section3.matchAll(/\*\*([A-Z]{2,4})-(\d+)(?: → ([A-Z]{2,4})-(\d+))?\.\*\*/g)) {
  const [, p, a, , b] = m;
  for (let i = Number(a); i <= Number(b ?? a); i++) ids.add(`${p}-${i}`);
}

// 2. Ids cited by tests: `/// req: A, B` in Rust, `// req: A, B` in TypeScript.
const covered = new Map();
function walk(dir) {
  if (!existsSync(dir)) return;
  for (const name of readdirSync(dir)) {
    if (["node_modules", "target", "dist", "bindings", "test-results", "playwright-report"].includes(name)) continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p);
    else if (/\.(rs|ts|tsx)$/.test(name)) {
      const text = readFileSync(p, "utf8");
      for (const m of text.matchAll(/\/\/\/?\s*req:\s*([A-Z0-9 ,\-]+)/g)) {
        for (const id of m[1].split(/[,\s]+/).filter(Boolean)) {
          if (!covered.has(id)) covered.set(id, []);
          covered.get(id).push(p.slice(app.length + 1));
        }
      }
    }
  }
}
for (const d of ["crates", "src-tauri/src", "ui/src", "ui/e2e"]) walk(join(app, d));

// 3. Justified ids: lines "- **ID** — reason" in docs/app/REQS-JUSTIFIED.md.
const justifiedPath = join(repo, "docs/app/REQS-JUSTIFIED.md");
const justified = new Set();
if (existsSync(justifiedPath)) {
  for (const m of readFileSync(justifiedPath, "utf8").matchAll(/^- \*\*([A-Z]{2,4}-\d+)\*\*/gm)) justified.add(m[1]);
}

// 4. Closed phases from docs/app/PROGRESS.md (✅ or ⚠️).
const progress = readFileSync(join(repo, "docs/app/PROGRESS.md"), "utf8");
const closed = new Set([...progress.matchAll(/^\| (F\d+) [^|]*\| (✅|⚠️)/gmu)].map((m) => m[1]));
const phases = JSON.parse(readFileSync(join(app, "scripts/req-phases.json"), "utf8"));
const phaseOf = (id) => phases.exact[id] ?? phases.prefix[id.split("-")[0]] ?? "F10";

const unknown = [...covered.keys()].filter((id) => !ids.has(id));
const missing = [...ids].filter((id) => !covered.has(id));
const holes = missing.filter((id) => !justified.has(id));
const closedHoles = holes.filter((id) => closed.has(phaseOf(id)));

const closedMode = process.argv.includes("--closed");
console.log(`requirements: ${ids.size} · tested: ${ids.size - missing.length} · justified: ${missing.length - holes.length} · holes: ${holes.length}`);
if (!closedMode) {
  for (const id of holes) console.log(`  ${id.padEnd(8)} (${phaseOf(id)})`);
}
if (unknown.length) {
  console.error(`unknown ids cited by tests: ${unknown.join(", ")}`);
  process.exit(1);
}
if (closedMode && closedHoles.length) {
  console.error(`uncovered requirements of closed phases (${[...closed].join(", ")}): ${closedHoles.join(", ")}`);
  process.exit(1);
}
if (process.argv.includes("--strict") && holes.length) process.exit(1);
