// Crystalline Lineage
// @prompt lab/adaptador-typescript/CONTRATO.md
// @layer Lab
// @updated 2026-09-26
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
// Carrega somente a dependência do adaptador, nunca código do projeto.
const ts = require('./node_modules/typescript');
const VERSION = '0.1.0';
const LIMITES = [
  'Granularidade arquivo/módulo, não função; imports não representam chamadas.',
  'Dependências externas resolvidas são registradas e excluídas do grafo.',
  'Sem dynamic import, CommonJS, import-equals ou import-types em expressões de tipo.',
  'Sem referências triple-slash, módulos ambiente, import.meta ou plugins de bundler.',
  'Somente arquivos JS/JSX/TS/TSX/MJS/CJS/MTS/CTS e declarações locais; sem recursos não JS/TS.',
  'Project references exigem análises separadas por tsconfig.',
  'Sem typecheck, emit, execução do projeto ou cache persistente.',
  'Visibilidade priv é preenchimento do contrato; não descreve exports do arquivo.',
  'IDs são determinísticos na mesma extração; não são identidade persistente entre revisões.',
];
const cmp = (a, b) => a < b ? -1 : a > b ? 1 : 0;
const normalize = p => path.resolve(p).split(path.sep).join('/');
const supported = p => /\.(?:[cm]?[jt]s|[jt]sx)$/i.test(p);
const externalPath = p => normalize(p).split('/').includes('node_modules');
const json = value => JSON.stringify(value, null, 2) + '\n';

class ExtractionError extends Error {
  constructor(message, diagnostics) { super(message); this.diagnostics = diagnostics; }
}
function diagnosticTs(d) {
  const pos = d.file && d.start !== undefined ? d.file.getLineAndCharacterOfPosition(d.start) : null;
  return { code: `TS${d.code}`, severity: 'error', message: ts.flattenDiagnosticMessageText(d.messageText, '\n'),
    ...(d.file ? { file: normalize(d.file.fileName) } : {}),
    ...(pos ? { line: pos.line + 1, column: pos.character + 1 } : {}) };
}
function extract(project, name = 'project') {
  if (ts.version !== '5.9.3') throw new Error(`TypeScript 5.9.3 requerido; encontrado ${ts.version}`);
  const config = normalize(project);
  const root = path.dirname(config);
  const configErrors = [];
  const parsed = ts.getParsedCommandLineOfConfigFile(config, {}, {
    ...ts.sys, onUnRecoverableConfigFileDiagnostic: d => configErrors.push(d),
  });
  configErrors.push(...(parsed?.errors ?? []));
  if (!parsed || configErrors.length) throw new ExtractionError('Configuração TypeScript inválida', configErrors.map(diagnosticTs));
  if (parsed.projectReferences?.length) throw new ExtractionError('Project references não suportadas', [
    { code: 'project-references', severity: 'error', file: config, message: 'Analise cada tsconfig referenciado separadamente.' },
  ]);
  const program = ts.createProgram({ rootNames: parsed.fileNames, options: { ...parsed.options, noEmit: true } });
  const optionErrors = program.getOptionsDiagnostics().filter(d => d.category === ts.DiagnosticCategory.Error);
  if (optionErrors.length) throw new ExtractionError('Opções TypeScript inválidas', optionErrors.map(diagnosticTs));
  const sources = new Map();
  for (const sf of program.getSourceFiles()) sources.set(normalize(sf.fileName), sf);
  const diagnostics = [];
  const occurrences = [];
  const files = new Map();
  const pairs = new Set();
  const pending = parsed.fileNames.map(normalize).sort(cmp);
  const queued = new Set(pending);
  function report(code, message, sf, node, extra = {}) {
    const p = node ? sf.getLineAndCharacterOfPosition(node.getStart(sf)) : { line: 0, character: 0 };
    diagnostics.push({ code, severity: 'warning', message, file: normalize(sf.fileName), line: p.line + 1, column: p.character + 1, ...extra });
  }
  for (let i = 0; i < pending.length; i++) {
    const file = pending[i];
    if (externalPath(file)) {
      diagnostics.push({ code: 'external-root-excluded', severity: 'warning', file, message: 'Raiz em node_modules excluída pela política externa.' });
      continue;
    }
    const sf = sources.get(file);
    if (!sf) throw new ExtractionError('Arquivo selecionado indisponível', [
      { code: 'source-unavailable', severity: 'error', file, message: 'O compilador não disponibilizou o arquivo selecionado/resolvido.' },
    ]);
    if (!supported(file)) { report('unsupported-resource', 'Recurso não JS/TS fora do grafo.', sf); continue; }
    const syntax = program.getSyntacticDiagnostics(sf);
    if (syntax.length) throw new ExtractionError('Sintaxe inválida', syntax.map(diagnosticTs));
    files.set(file, sf);
    for (const ref of [...sf.referencedFiles, ...sf.typeReferenceDirectives, ...sf.libReferenceDirectives]) {
      const pos = sf.getLineAndCharacterOfPosition(ref.pos);
      diagnostics.push({ code: 'triple-slash', severity: 'warning', file, line: pos.line + 1, column: pos.character + 1,
        message: 'Referência triple-slash não vira aresta.', specifier: ref.fileName });
    }
    function staticDependency(node) {
      const spec = node.moduleSpecifier;
      if (!spec || !ts.isStringLiteralLike(spec)) return;
      const clause = ts.isImportDeclaration(node) ? node.importClause : null;
      const bindings = clause?.namedBindings;
      const named = bindings && ts.isNamedImports(bindings) ? bindings.elements :
        ts.isExportDeclaration(node) && node.exportClause && ts.isNamedExports(node.exportClause) ? node.exportClause.elements : [];
      const typeOnly = Boolean(node.isTypeOnly || clause?.isTypeOnly ||
        (!clause?.name && named.length > 0 && named.every(n => n.isTypeOnly)));
      const pos = sf.getLineAndCharacterOfPosition(spec.getStart(sf));
      const occurrence = { origin: file, line: pos.line + 1, column: pos.character + 1,
        specifier: spec.text, nature: ts.isImportDeclaration(node) ? 'import' : 'reexport', type_only: typeOnly,
        type_only_specifiers: named.filter(n => n.isTypeOnly).map(n => n.name.text) };
      const mode = program.getModeForUsageLocation(sf, spec);
      const resolved = ts.resolveModuleName(spec.text, sf.fileName, parsed.options, ts.sys, undefined, undefined, mode).resolvedModule;
      if (!resolved) {
        occurrence.status = 'unresolved';
        report('unresolved-import', 'Import/reexport não resolvido pelo TypeScript; nenhuma aresta criada.', sf, spec, { specifier: spec.text });
      } else {
        const target = normalize(resolved.resolvedFileName);
        occurrence.target = target;
        if (resolved.isExternalLibraryImport || externalPath(target)) {
          occurrence.status = 'external-excluded';
          report('external-excluded', 'Dependência externa resolvida, excluída pela política do adaptador.', sf, spec, { specifier: spec.text, target });
        } else if (!supported(target)) {
          occurrence.status = 'unsupported-resource';
          report('unsupported-resource', 'Destino não JS/TS; nenhuma aresta criada.', sf, spec, { specifier: spec.text, target });
        } else if (!sources.has(target) && (parsed.options.noResolve || (!parsed.options.allowJs && /\.[cm]?jsx?$/i.test(target)))) {
          occurrence.status = 'configuration-excluded';
          report('configuration-excluded', 'Destino excluído do Program por noResolve/allowJs; nenhuma aresta criada.', sf, spec, { specifier: spec.text, target });
        } else {
          occurrence.status = 'included';
          pairs.add(JSON.stringify([file, target]));
          if (!queued.has(target)) { pending.push(target); queued.add(target); }
        }
      }
      occurrences.push(occurrence);
    }
    function visit(node) {
      if (ts.isModuleDeclaration(node)) {
        report('module-declaration', 'Módulo ambiente/namespace não modelado internamente.', sf, node);
        return; // não transformar declarações de módulo em imports de arquivo
      }
      if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) staticDependency(node);
      if (ts.isExportAssignment(node) && node.isExportEquals) report('commonjs', 'export = não suportado.', sf, node);
      if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.EqualsToken && ts.isIdentifier(node.left) && node.left.text === 'exports') report('commonjs', 'Atribuição a exports não modelada.', sf, node);
      if (ts.isImportEqualsDeclaration(node)) report('import-equals', 'import-equals não suportado; nenhuma aresta criada.', sf, node);
      if (ts.isImportTypeNode(node)) report('import-type-expression', 'Expressão de tipo import() não suportada.', sf, node);
      if (ts.isCallExpression(node)) {
        if (node.expression.kind === ts.SyntaxKind.ImportKeyword) report('dynamic-import', 'Import dinâmico não suportado.', sf, node);
        if (ts.isIdentifier(node.expression) && node.expression.text === 'require') report('commonjs', 'require/CommonJS não suportado.', sf, node);
      }
      if (ts.isPropertyAccessExpression(node) || ts.isElementAccessExpression(node)) {
        if (ts.isIdentifier(node.expression) && ['module', 'exports', 'require'].includes(node.expression.text))
          report('commonjs', 'Acesso a module/exports/require não modelado; pode ser CommonJS.', sf, node);
      }
      if (ts.isMetaProperty(node) && node.keywordToken === ts.SyntaxKind.ImportKeyword) report('import-meta', 'import.meta não modelado.', sf, node);
      ts.forEachChild(node, visit);
    }
    visit(sf);
  }
  const prefix = `ts_${encodeURIComponent(name)}`;
  const fileList = [...files.keys()].sort(cmp);
  const nodes = fileList.map((file, id) => ({ id,
    path: [prefix, ...path.relative(root, file).split(path.sep).map(encodeURIComponent)].join('::'),
    name: path.basename(file), kind: 'mod', visibility: 'priv',
    position: { file, start_line: 1, end_line: files.get(file).getLineAndCharacterOfPosition(files.get(file).text.length).line + 1 },
  }));
  const byFile = new Map(fileList.map((file, i) => [file, nodes[i]]));
  const edges = [...pairs].map(s => {
    const [a, b] = JSON.parse(s); const from = byFile.get(a); const to = byFile.get(b);
    if (!from || !to) throw new Error(`Invariante: destino ausente ${b}`);
    return { from: from.path, id_from: from.id, to: to.path, id_to: to.id, relation: 'uses', uses_kind: 'import' };
  }).sort((a, b) => a.id_from - b.id_from || a.id_to - b.id_to);
  const graph = { crate: prefix, nodes, edges };
  occurrences.sort((a, b) => cmp(a.origin, b.origin) || a.line - b.line || a.column - b.column);
  diagnostics.sort((a, b) => cmp(a.file, b.file) || (a.line ?? 0) - (b.line ?? 0) || (a.column ?? 0) - (b.column ?? 0) || cmp(a.code, b.code));
  const metadata = { schema_version: 1, provider: 'typescript-compiler-api-experimental', provider_version: VERSION,
    typescript_version: ts.version, granularity: 'file', project: config,
    coverage: diagnostics.some(d => d.code !== 'external-excluded') ? 'partial' : 'complete-within-declared-scope',
    external_policy: 'record-and-exclude', cache: 'disabled', limits: LIMITES,
    selected_files: parsed.fileNames.map(normalize).sort(cmp), analyzed_files: fileList,
    graph_sha256: crypto.createHash('sha256').update(json(graph)).digest('hex'), diagnostics, occurrences };
  return { graph, metadata };
}

function main(argv) {
  if (argv.includes('--help')) { console.log('node extrair.cjs --project tsconfig.json --out DIRETORIO_NOVO [--name projeto]'); return; }
  const args = {};
  for (let i = 0; i < argv.length; i += 2) {
    if (!['--project', '--out', '--name'].includes(argv[i]) || !argv[i + 1] || args[argv[i]]) throw new Error('Argumentos inválidos; use --help');
    args[argv[i]] = argv[i + 1];
  }
  if (!args['--project'] || !args['--out']) throw new Error('--project e --out são obrigatórios');
  const out = path.resolve(args['--out']);
  fs.mkdirSync(out, { recursive: true });
  const graphFile = path.join(out, 'grafo.json'); const metadataFile = path.join(out, 'metadados.json');
  if (fs.existsSync(graphFile) || fs.existsSync(metadataFile)) throw new Error('Saída já existe; escolha outro diretório.');
  let graphWritten = false;
  try {
    const result = extract(args['--project'], args['--name']);
    fs.writeFileSync(graphFile, json(result.graph), { flag: 'wx' });
    graphWritten = true;
    fs.writeFileSync(metadataFile, json(result.metadata), { flag: 'wx' });
    console.log(`${graphFile}\n${metadataFile}`);
  } catch (e) {
    if (graphWritten) fs.unlinkSync(graphFile);
    if (e instanceof ExtractionError) fs.writeFileSync(metadataFile, json({
      schema_version: 1, provider: 'typescript-compiler-api-experimental', provider_version: VERSION,
      typescript_version: ts.version, granularity: 'file', coverage: 'error', limits: LIMITES,
      diagnostics: e.diagnostics,
    }), { flag: 'wx' });
    throw e;
  }
}
if (require.main === module) {
  try { main(process.argv.slice(2)); }
  catch (e) { console.error(e.message); process.exitCode = 1; }
}
module.exports = { extract, ExtractionError, json };
