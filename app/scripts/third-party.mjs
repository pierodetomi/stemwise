#!/usr/bin/env node
// Third-party notices (NFR-6): writes src-tauri/resources/THIRD-PARTY-LICENSES.txt, the list of every library that
// ships inside the app with its licence text. Permissive licences ask for exactly this: their notice travels with
// the binary. The file is tracked and bundled into Stemwise.app; `--check` fails when it is out of date.
//
// What is listed: the Rust crates the workspace links for the shipped target (normal dependencies only, no build or
// dev dependencies) and the npm packages the interface depends on, fonts included.
import { execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const app = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(app, "src-tauri/resources/THIRD-PARTY-LICENSES.txt");
const TARGET = "aarch64-apple-darwin"; // the one target that is packaged (A-046); fixed so the list is the same everywhere
const run = (cmd, cwd) => execSync(cmd, { cwd, encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });
const NOTICE = /^(licen[sc]e|copying|notice|unlicense|copyright)/i;
// Plain code-unit order: a locale-aware comparison changes with the Node and ICU version, and the list must come
// out the same on every machine.
const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

/** Licence and notice files at the top of a package directory, in a stable order. */
function notices(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isFile() && NOTICE.test(e.name))
    .map((e) => e.name)
    .sort(cmp)
    .map((name) => ({ name, text: readFileSync(join(dir, name), "utf8").replace(/\r\n/g, "\n").trim() }));
}

function rustPackages() {
  const meta = JSON.parse(run(`cargo metadata --format-version 1 --locked --filter-platform ${TARGET}`, app));
  const byId = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const seen = new Set(meta.workspace_members);
  const queue = [...meta.workspace_members];
  while (queue.length) {
    for (const dep of nodes.get(queue.pop()).deps) {
      if (seen.has(dep.pkg) || !dep.dep_kinds.some((k) => k.kind === null)) continue;
      seen.add(dep.pkg);
      queue.push(dep.pkg);
    }
  }
  const members = new Set(meta.workspace_members);
  return [...seen].filter((id) => !members.has(id)).map((id) => {
    const p = byId.get(id);
    return { kind: "Rust crate", name: p.name, version: p.version, licence: p.license ?? "see the text below",
      url: p.repository ?? p.homepage ?? `https://crates.io/crates/${p.name}`, files: notices(dirname(p.manifest_path)) };
  });
}

// The npm packages that ship: the interface's `dependencies` and theirs, followed through node_modules. Peer and
// optional peer dependencies (type declarations, mostly) are not followed: they are not bundled, and whether a
// package manager lists them changes with its version.
function npmPackages() {
  const find = (name, from) => {
    for (let dir = from; ; dir = dirname(dir)) {
      const at = join(dir, "node_modules", name, "package.json");
      if (existsSync(at)) return dirname(realpathSync(at));
      if (dirname(dir) === dir) throw new Error(`npm package not installed: ${name} (run make setup)`);
    }
  };
  const seen = new Map();
  const walk = (names, from) => {
    for (const name of names) {
      const dir = find(name, from);
      const pkg = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
      const id = `${pkg.name}@${pkg.version}`;
      if (seen.has(id)) continue;
      const repo = typeof pkg.repository === "string" ? pkg.repository : pkg.repository?.url;
      seen.set(id, { kind: "npm package", name: pkg.name, version: pkg.version, licence: pkg.license ?? "see the text below",
        url: pkg.homepage ?? repo ?? `https://www.npmjs.com/package/${pkg.name}`, files: notices(dir) });
      walk(Object.keys(pkg.dependencies ?? {}).sort(cmp), dir);
    }
  };
  const ui = join(app, "ui");
  walk(Object.keys(JSON.parse(readFileSync(join(ui, "package.json"), "utf8")).dependencies ?? {}).sort(cmp), ui);
  return [...seen.values()];
}

function render() {
  const pkgs = [...rustPackages(), ...npmPackages()]
    .sort((a, b) => cmp(a.name, b.name) || cmp(a.version, b.version) || cmp(a.kind, b.kind));
  // Many packages carry the very same text: each text is printed once, under the packages that use it.
  const texts = new Map();
  const missing = [];
  for (const p of pkgs) {
    if (!p.files.length) missing.push(p);
    for (const f of p.files) {
      const key = createHash("sha256").update(f.text).digest("hex");
      if (!texts.has(key)) texts.set(key, { text: f.text, users: [] });
      texts.get(key).users.push(`${p.name} ${p.version} (${f.name})`);
    }
  }
  const rule = "=".repeat(100);
  const lines = [
    "Stemwise: third-party software",
    "",
    "Stemwise is released under the MIT licence (see LICENSE). It includes the libraries listed here, each under",
    "its own licence. Part 1 lists them; part 2 reproduces their licence and notice texts.",
    "",
    "This file is generated by app/scripts/third-party.mjs. Do not edit it by hand.",
    "",
    rule, `PART 1 · ${pkgs.length} LIBRARIES`, rule, "",
    ...pkgs.map((p) => `${p.name} ${p.version} · ${p.licence} · ${p.kind} · ${p.url}`),
    "",
  ];
  if (missing.length) {
    lines.push("Packages that ship no licence file of their own (their licence is the one named above):", "",
      ...missing.map((p) => `${p.name} ${p.version} · ${p.licence}`), "");
  }
  lines.push(rule, "PART 2 · LICENCE AND NOTICE TEXTS", rule);
  for (const { text, users } of [...texts.values()].sort((a, b) => cmp(a.users[0], b.users[0]))) {
    lines.push("", "Used by: " + users.join(", "), "-".repeat(100), "", text, "", rule);
  }
  return { body: lines.join("\n") + "\n", count: pkgs.length, missing: missing.length };
}

const { body, count, missing } = render();
if (process.argv.includes("--check")) {
  const have = existsSync(out) ? readFileSync(out, "utf8") : "";
  if (have !== body) {
    console.error("third-party notices are out of date: run `make third-party` and commit the result");
    // Say where, so a difference between two machines can be read from the log.
    const a = have.split("\n"), b = body.split("\n");
    console.error(`  tracked file: ${a.length} lines · generated here: ${b.length} lines`);
    let shown = 0;
    for (let i = 0; i < Math.max(a.length, b.length) && shown < 12; i++) {
      if (a[i] === b[i]) continue;
      shown++;
      console.error(`  line ${i + 1}\n    tracked:   ${(a[i] ?? "<none>").slice(0, 160)}\n    generated: ${(b[i] ?? "<none>").slice(0, 160)}`);
    }
    process.exit(1);
  }
  console.log(`third-party notices: up to date (${count} libraries)`);
} else {
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, body);
  console.log(`third-party notices: ${count} libraries, ${missing} without a licence file of their own → ${out}`);
}
