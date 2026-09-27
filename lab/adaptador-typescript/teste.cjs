// Crystalline Lineage
// @prompt lab/adaptador-typescript/CONTRATO.md
// @layer Lab
// @updated 2026-09-26
'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const crypto = require('node:crypto');
const { extract, ExtractionError, json } = require('./extrair.cjs');
const tmp = path.join(__dirname, '.tmp');
fs.mkdirSync(tmp, { recursive: true });
function fixture(t, sources, config = {}) {
  const dir = fs.mkdtempSync(path.join(tmp, 'teste-'));
  const cfg = { compilerOptions: { module: 'ESNext', moduleResolution: 'Bundler', target: 'ES2022', ...config.compilerOptions },
    include: ['src/**/*'], ...config };
  const entries = { 'tsconfig.json': json(cfg), ...sources };
  for (const [name, text] of Object.entries(entries)) {
    const file = path.join(dir, name); fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, text);
  }
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  return { dir, config: path.join(dir, 'tsconfig.json'), run: () => extract(path.join(dir, 'tsconfig.json'), 'teste') };
}
const edgeNames = g => g.edges.map(e => `${g.nodes[e.id_from].name}->${g.nodes[e.id_to].name}`);

test('imports relativos, index, aliases e fechamento de arquivo fora de include', t => {
  const f = fixture(t, { 'src/main.ts': 'import { b } from "./dir"; import { a } from "@lib/a"; export {b,a};',
    'src/dir/index.ts': 'export const b = 1;', 'lib/a.ts': 'export const a = 1;', 'other/ignored.ts': 'export const x=1;' },
  { compilerOptions: { module: 'ESNext', moduleResolution: 'Bundler', baseUrl: '.', paths: { '@lib/*': ['lib/*'] } } });
  const r = f.run();
  assert.deepEqual(edgeNames(r.graph), ['main.ts->a.ts', 'main.ts->index.ts']);
  assert.equal(r.graph.nodes.length, 3);
  assert.equal(r.metadata.coverage, 'complete-within-declared-scope');
  for (const n of r.graph.nodes) { assert.equal(n.kind, 'mod'); assert.equal(n.visibility, 'priv'); }
  for (const e of r.graph.edges) {
    assert.equal(e.from, r.graph.nodes[e.id_from].path); assert.equal(e.to, r.graph.nodes[e.id_to].path);
    assert.equal(e.relation, 'uses'); assert.equal(e.uses_kind, 'import');
  }
});

test('reexports, import type e especificadores type preservam natureza sem duplicar arestas', t => {
  const f = fixture(t, { 'src/main.ts': 'import type { T } from "./types"; import { type T as U } from "./types"; export type { T } from "./types"; export * from "./types";',
    'src/types.ts': 'export type T = string;' });
  const r = f.run();
  assert.equal(r.graph.edges.length, 1); assert.equal(r.metadata.occurrences.length, 4);
  assert.deepEqual(r.metadata.occurrences.map(o => [o.nature, o.type_only]), [['import', true], ['import', true], ['reexport', true], ['reexport', false]]);
  assert.deepEqual(r.metadata.occurrences[1].type_only_specifiers, ['U']);
});

test('externo resolvido fica nos metadados; irresolvido deixa análise parcial', t => {
  const f = fixture(t, { 'src/main.ts': 'import {x} from "dep"; import "./ausente";',
    'node_modules/dep/package.json': '{"name":"dep","types":"index.d.ts"}',
    'node_modules/dep/index.d.ts': 'export const x: number;' });
  const r = f.run(); assert.equal(r.graph.nodes.length, 1); assert.equal(r.graph.edges.length, 0);
  assert.deepEqual(r.metadata.occurrences.map(o => o.status), ['external-excluded', 'unresolved']);
  assert.equal(r.metadata.coverage, 'partial');
  assert(r.metadata.diagnostics.some(d => d.code === 'unresolved-import' && d.specifier === './ausente' && d.line === 1));
});

test('ciclo e IDs/JSON determinísticos; hash corresponde aos bytes do grafo', t => {
  const f = fixture(t, { 'src/b.ts': 'import "./a";', 'src/a.ts': 'import "./b";' });
  const a = f.run(); const b = f.run(); assert.equal(json(a), json(b));
  assert.deepEqual(edgeNames(a.graph), ['a.ts->b.ts', 'b.ts->a.ts']);
  assert.equal(new Set(a.graph.nodes.map(n => n.id)).size, 2);
  assert.equal(a.metadata.graph_sha256, crypto.createHash('sha256').update(json(a.graph)).digest('hex'));
});

test('extends/files/exclude e allowJs selecionam arquivos sem executar o projeto', t => {
  const f = fixture(t, { 'base.json': '{"compilerOptions":{"allowJs":true,"jsx":"preserve","module":"ESNext","moduleResolution":"Bundler"}}',
    'src/a.js': 'import "./b.jsx"; throw new Error("PROJETO EXECUTADO");', 'src/b.jsx': 'export const B = <div />;',
    'src/c.tsx': 'export const C = <div />;', 'src/ignored.js': 'throw new Error("não selecionar");' },
    { extends: './base.json', files: ['src/a.js', 'src/c.tsx'], include: [], exclude: ['src/ignored.js'], compilerOptions: {} });
  const r = f.run(); assert.deepEqual(r.graph.nodes.map(n => n.name), ['a.js', 'b.jsx', 'c.tsx']);
  assert.deepEqual(edgeNames(r.graph), ['a.js->b.jsx']);
});

test('NodeNext respeita modo import e substituição .js -> .ts', t => {
  const f = fixture(t, { 'package.json': '{"type":"module"}', 'src/a.ts': 'import "./b.js"; import "./sem-extensao";',
    'src/b.ts': 'export {};', 'src/sem-extensao.ts': 'export {};' },
    { compilerOptions: { module: 'NodeNext', moduleResolution: 'NodeNext', target: 'ES2022' } });
  const r = f.run(); assert.deepEqual(edgeNames(r.graph), ['a.ts->b.ts']);
  assert(r.metadata.diagnostics.some(d => d.specifier === './sem-extensao' && d.code === 'unresolved-import'));
});

test('construções não suportadas diagnosticadas sem arestas inventadas', t => {
  const f = fixture(t, { 'src/a.ts': '/// <reference path="./b.ts" />\nimport("./b"); const x=require("./b"); module.exports=x; import q = require("./b"); type T=import("./b").T; declare module "ambient" { export * from "./b"; } const m=import.meta;',
    'src/b.ts': 'export type T = string;' });
  const r = f.run(); assert.equal(r.graph.edges.length, 0);
  const codes = new Set(r.metadata.diagnostics.map(d => d.code));
  for (const code of ['triple-slash','dynamic-import','commonjs','import-equals','import-type-expression','module-declaration','import-meta']) assert(codes.has(code), code);
  assert.equal(r.metadata.coverage, 'partial');
});

test('sem cache: edição entre extrações altera o grafo', t => {
  const f = fixture(t, { 'src/a.ts': 'export {};', 'src/b.ts': 'export {};' });
  assert.equal(f.run().graph.edges.length, 0);
  fs.writeFileSync(path.join(f.dir, 'src/a.ts'), 'import "./b";');
  assert.equal(f.run().graph.edges.length, 1);
});

test('erros de configuração, sintaxe e project references falham explicitamente', t => {
  for (const [sources, config] of [
    [{ 'src/a.ts': 'export const x = ;' }, {}],
    [{ 'src/a.ts': 'export {};' }, { compilerOptions: { moduleResolution: 'inexistente' } }],
    [{ 'src/a.ts': 'export {};' }, { references: [{ path: './outro' }] }],
  ]) {
    const f = fixture(t, sources, config); assert.throws(f.run, ExtractionError);
  }
});

test('CLI escreve dois artefatos, recusa sobrescrita e publica só metadados em erro', t => {
  const f = fixture(t, { 'src/a.ts': 'export {};' });
  const run = dir => spawnSync(process.execPath, [path.join(__dirname,'extrair.cjs'), '--project', f.config, '--out', dir], { encoding: 'utf8' });
  const out = path.join(f.dir,'out'); assert.equal(run(out).status, 0);
  assert.equal(JSON.parse(fs.readFileSync(path.join(out,'metadados.json'))).granularity, 'file');
  assert.equal(run(out).status, 1);
  fs.writeFileSync(path.join(f.dir,'src/a.ts'), 'export const x = ;');
  const fail = path.join(f.dir,'failed'); assert.equal(run(fail).status, 1);
  assert.equal(fs.existsSync(path.join(fail,'grafo.json')), false);
  assert.equal(JSON.parse(fs.readFileSync(path.join(fail,'metadados.json'))).coverage, 'error');
});

test('caminhos com delimitadores são codificados sem colisão', t => {
  const f = fixture(t, { 'src/a::b.ts': 'export {};', 'src/a/b.ts': 'export {};' });
  const r = f.run(); assert.equal(new Set(r.graph.nodes.map(n => n.path)).size, 2);
  assert(r.graph.nodes.some(n => n.path.includes('a%3A%3Ab.ts')));
});


test('noResolve respeita seleção explícita e não inventa nó para destino excluído', t => {
  const f = fixture(t, { 'src/a.ts': 'import "./b";', 'src/b.ts': 'export {};' },
    { compilerOptions: { module: 'ESNext', moduleResolution: 'Bundler', noResolve: true }, files: ['src/a.ts'], include: [] });
  const r = f.run(); assert.equal(r.graph.nodes.length, 1); assert.equal(r.graph.edges.length, 0);
  assert.equal(r.metadata.occurrences[0].status, 'configuration-excluded');
});
