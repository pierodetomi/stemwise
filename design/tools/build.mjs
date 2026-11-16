// Stemwise design boards — generator. Run from the repository root: `node design/tools/build.mjs`.
// Writes static HTML (no script at view time) into design/screens/ and design/components/.
import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const only = process.argv[2];
const modules = ['s-mix', 's-start', 's-edit', 's-master', 's-chain', 's-trackchain', 's-explain', 's-tour', 's-prep', 's-listen', 's-system', 's-setup', 's-components'];
let n = 0;
for (const m of modules) {
  let mod;
  try { mod = await import(`./${m}.mjs`); } catch (e) { if (e.code === 'ERR_MODULE_NOT_FOUND' && String(e.message).includes(`${m}.mjs`)) continue; throw e; }
  for (const [file, html] of Object.entries(mod.default)) {
    if (only && !file.includes(only)) continue;
    const p = join(root, file); mkdirSync(dirname(p), { recursive: true }); writeFileSync(p, html); n++;
    console.log(file, Math.round(html.length / 1024) + ' KB');
  }
}
console.log(n + ' files written');
