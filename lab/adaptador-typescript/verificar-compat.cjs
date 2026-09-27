// Crystalline Lineage
// @prompt lab/adaptador-typescript/CONTRATO.md
// @layer Lab
// @updated 2026-09-26
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const { extract, json } = require('./extrair.cjs');
const root = path.join(__dirname, '.tmp');
fs.mkdirSync(root, { recursive: true });
const dir = fs.mkdtempSync(path.join(root, 'compat-'));
try {
  fs.writeFileSync(path.join(dir, 'tsconfig.json'), json({ compilerOptions: { module: 'ESNext', moduleResolution: 'Bundler' }, files: ['a.ts','b.ts','c.ts','d.ts'] }));
  for (const [file, target] of [['a','b'],['b','c'],['c','a'],['d','a']]) fs.writeFileSync(path.join(dir, `${file}.ts`), `import "./${target}";\n`);
  const result = extract(path.join(dir, 'tsconfig.json'), 'compat');
  fs.writeFileSync(path.join(dir, 'grafo.json'), json(result.graph));
  const execution = spawnSync('cargo', ['run', '--offline', '--quiet', '--manifest-path', path.join(__dirname, 'compat/Cargo.toml'), '--', path.join(dir, 'grafo.json')], {
    stdio: 'inherit', env: { ...process.env, CARGO_TARGET_DIR: path.join(__dirname,'compat/target') },
  });
  if (execution.error) throw execution.error;
  process.exitCode = execution.status ?? 1;
} finally { fs.rmSync(dir, { recursive: true, force: true }); }
