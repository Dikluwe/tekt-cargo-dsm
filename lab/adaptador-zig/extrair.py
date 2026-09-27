# Crystalline Lineage
# @prompt lab/adaptador-zig/CONTRATO.md
# @layer Lab
# @updated 2026-09-26
"""Grafo sintático de imports Zig; nenhuma avaliação semântica/comptime."""
import argparse
import hashlib
import json
import os
import pathlib
import subprocess
from urllib.parse import quote

BASE = pathlib.Path(__file__).resolve().parent
VERSION = '0.1.0'
ZIG_VERSION = '0.15.2'
LIMITES = [
    'Granularidade arquivo; uses/import representa candidato sintático, nunca chamada ou ativação provada.',
    'Nenhuma avaliação de comptime, condições, target, optimize, opções de build ou lazy declarations.',
    'Módulos nomeados e gerados pelo build devem ser fornecidos explicitamente no mapa global.',
    'Não executa build.zig, zig build, projeto, clang, código C ou geradores.',
    'std e builtin são externos excluídos; root referencia a raiz fornecida.',
    '@cImport é fronteira externa; @embedFile e .zon são fronteiras de dados não analisadas.',
    'Mapa global não reproduz tabelas de imports por módulo nem valida limites de pacote Zig.',
    'Contextos por contenção de spans AST são conservadores; não demonstram que um ramo está ativo.',
    'Sem cache de extração; entradas devem permanecer estáveis durante a análise.',
    'Helper limitado a 16 MiB por arquivo; colunas contam bytes UTF-8.',
]


def json_bytes(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode('utf-8')


class ExtractionError(Exception):
    def __init__(self, message, metadata):
        super().__init__(message)
        self.metadata = metadata


def read_modules(filename):
    file = pathlib.Path(filename).resolve()
    raw = json.loads(file.read_text(encoding='utf-8'))
    if not isinstance(raw, dict):
        raise ValueError('O mapa deve ser um objeto JSON de nomes para caminhos ou {path, generated}.')
    modules = {}
    for name, value in sorted(raw.items()):
        if not name or name in ('std', 'builtin', 'root'):
            raise ValueError(f'Nome de módulo vazio ou reservado: {name!r}')
        if isinstance(value, str):
            value = {'path': value, 'generated': False}
        if (not isinstance(value, dict) or set(value) - {'path', 'generated'}
                or not isinstance(value.get('path'), str) or not value['path']
                or not isinstance(value.get('generated', False), bool)):
            raise ValueError(f'Módulo {name!r}: esperado caminho ou {{path, generated: bool}}.')
        modules[name] = {'path': str((file.parent / value['path']).resolve()),
                         'generated': value.get('generated', False)}
    return modules


def parse_file(file, parser):
    result = subprocess.run([str(parser), str(file)], text=True, encoding='utf-8', capture_output=True, check=False)
    if result.returncode:
        raise RuntimeError(f'Helper falhou ({result.returncode}): {result.stderr.strip()}')
    parsed = json.loads(result.stdout)
    if parsed.get('zig_version') != ZIG_VERSION or parsed.get('parser') != 'std.zig.Ast':
        raise RuntimeError('Helper incompatível; prepare novamente com Zig 0.15.2.')
    return parsed


def extract(root_file, modules_file, name='project', parser=BASE / 'bin/parser'):
    root = pathlib.Path(root_file).resolve()
    modules = read_modules(modules_file)
    parser = pathlib.Path(parser).resolve()
    diagnostics = []
    occurrences = []
    files = {}
    pairs = set()
    metadata = {
        'schema_version': 1, 'provider': 'zig-ast-experimental', 'provider_version': VERSION,
        'zig_version': ZIG_VERSION, 'parser_api': 'std.zig.Ast', 'granularity': 'file',
        'configuration': {'root': str(root), 'modules_file': str(pathlib.Path(modules_file).resolve()), 'modules': modules},
        'capabilities': {'literal_import': True, 'relative_import': True, 'explicit_named_import': True,
                         'conditional_context': 'syntactic-span-only', 'semantic_resolution': False,
                         'comptime_evaluation': False, 'build_execution': False, 'c_analysis': False,
                         'cache': False},
        'edge_semantics': 'syntactic-candidate-not-proven-active',
        'completeness': {'syntax': 'complete-within-declared-scope', 'semantics': 'not-evaluated'},
        'external_policy': {'std': 'record-and-exclude', 'builtin': 'record-and-exclude', 'cImport': 'record-and-exclude'},
        'limits': LIMITES, 'diagnostics': diagnostics, 'occurrences': occurrences,
    }

    def report(code, message, origin, occurrence=None, **extra):
        pos = {k: occurrence[k] for k in ('line', 'column') if k in (occurrence or {})}
        diagnostics.append({'code': code, 'message': message, 'file': str(origin), **pos, **extra})

    pending = [root]
    seen = {root}
    while pending:
        file = pending.pop(0)
        try:
            parsed = parse_file(file, parser)
            if parsed['diagnostics']:
                for d in parsed['diagnostics']:
                    report('syntax-error', d['code'], file, d)
                if file == root:
                    metadata['completeness']['syntax'] = 'error'
                    raise ExtractionError('Erro de parsing da raiz Zig.', metadata)
                continue
        except ExtractionError:
            raise
        except (OSError, RuntimeError, ValueError, UnicodeError) as error:
            report('parser-failure', str(error), file)
            if file == root:
                metadata['completeness']['syntax'] = 'error'
                raise ExtractionError('Não foi possível analisar a raiz Zig.', metadata) from error
            continue
        files[file] = parsed['line_count']
        for raw in sorted(parsed['occurrences'], key=lambda o: (o['line'], o['column'])):
            occurrence = {'origin': str(file), **raw, 'activation': 'unknown'}
            occurrence['contexts'] = sorted(set(occurrence['contexts']))
            occurrences.append(occurrence)
            if occurrence['contexts']:
                report('activation-uncertain', 'Contexto AST não avaliado; relação somente candidata.', file, occurrence,
                       contexts=occurrence['contexts'])
            builtin = occurrence['builtin']
            if builtin == '@cImport':
                occurrence['status'] = 'external-c-boundary'
                report('c-import-external', 'Fronteira C: bloco não avaliado; headers e código C não analisados.', file, occurrence)
                continue
            if builtin == '@embedFile':
                occurrence['status'] = 'external-data-boundary'
                report('embedded-data', 'Dependência de dados não representada por uma aresta de módulo.', file, occurrence)
                continue
            target = occurrence['literal']
            if target is None:
                occurrence['status'] = 'nonliteral-unresolved'
                report('nonliteral-import', 'Argumento não literal; não há avaliação comptime nem aresta inferida.', file, occurrence)
                continue
            if target in ('std', 'builtin'):
                occurrence['status'] = 'external-standard'
                report('standard-excluded', f'{target} registrado, sem percorrer biblioteca padrão ou builtin gerado.', file, occurrence)
                continue
            generated = False
            if target == 'root':
                destination = root
            elif target in modules:
                destination = pathlib.Path(modules[target]['path'])
                generated = modules[target]['generated']
            elif target.endswith(('.zig', '.zon')) and not pathlib.Path(target).is_absolute():
                destination = (file.parent / target).resolve()
            else:
                occurrence['status'] = 'unmapped-module'
                report('unmapped-module', 'Módulo não fornecido no mapa ou caminho absoluto não suportado; configure explicitamente.', file, occurrence, target=target)
                continue
            occurrence['target'] = str(destination)
            if destination.suffix != '.zig':
                occurrence['status'] = 'unsupported-data'
                report('unsupported-data', 'Destino não .zig (incluindo ZON) não é analisado como módulo Zig.', file, occurrence, target=str(destination))
                continue
            if not destination.is_file():
                occurrence['status'] = 'missing-source'
                report('generated-module-missing' if generated else 'missing-source',
                       'Módulo gerado ausente; execute seu gerador separadamente.' if generated else 'Arquivo ausente; pode exigir geração/configuração explícita.',
                       file, occurrence, target=str(destination))
                continue
            occurrence['status'] = 'candidate'
            pairs.add((file, destination))
            if destination not in seen:
                pending.append(destination)
                seen.add(destination)

    prefix = 'zig_syntactic_' + quote(name, safe='')
    file_list = sorted(files, key=str)
    nodes = []
    for i, file in enumerate(file_list):
        # relpath preserva arquivos externos à raiz sem colisão com nomes internos.
        parts = pathlib.Path(os.path.relpath(file, root.parent)).parts
        nodes.append({'id': i, 'path': '::'.join([prefix, *(quote(p, safe='') for p in parts)]),
                      'name': file.name, 'kind': 'mod', 'visibility': 'priv',
                      'position': {'file': str(file), 'start_line': 1, 'end_line': files[file]}})
    by_file = dict(zip(file_list, nodes))
    edges = []
    for a, b in sorted(pairs, key=lambda p: (str(p[0]), str(p[1]))):
        if b not in by_file:
            for o in occurrences:
                if o['origin'] == str(a) and o.get('target') == str(b) and o['status'] == 'candidate':
                    o['status'] = 'target-analysis-failed'
            continue
        origin, dest = by_file[a], by_file[b]
        edges.append({'id_from': origin['id'], 'from': origin['path'], 'id_to': dest['id'], 'to': dest['path'],
                      'relation': 'uses', 'uses_kind': 'import'})
    graph = {'crate': prefix, 'nodes': nodes, 'edges': edges}
    occurrences.sort(key=lambda o: (o['origin'], o['line'], o['column']))
    diagnostics.sort(key=lambda d: (d['file'], d.get('line', 0), d.get('column', 0), d['code']))
    # Condições e fronteiras C/dados impedem declaração de completude sem ressalva.
    if any(d['code'] != 'standard-excluded' for d in diagnostics):
        metadata['completeness']['syntax'] = 'partial'
    metadata['graph_sha256'] = hashlib.sha256(json_bytes(graph)).hexdigest()
    metadata['analyzed_files'] = [str(f) for f in file_list]
    return graph, metadata


def main():
    args = argparse.ArgumentParser(description='Grafo sintático Zig 0.15.2; ativação de imports NÃO avaliada.')
    args.add_argument('--root', required=True)
    args.add_argument('--modules', required=True, help='Mapa JSON explícito; use {} quando não houver módulos nomeados.')
    args.add_argument('--out', required=True, help='Diretório sem grafo.json/metadados.json anteriores.')
    args.add_argument('--name', default='project')
    options = args.parse_args()
    out = pathlib.Path(options.out)
    out.mkdir(parents=True, exist_ok=True)
    graph_file, meta_file = out / 'grafo.json', out / 'metadados.json'
    if graph_file.exists() or meta_file.exists():
        raise SystemExit('Saída já existe; escolha outro diretório.')
    try:
        graph, meta = extract(options.root, options.modules, options.name)
    except ExtractionError as error:
        with meta_file.open('xb') as output:
            output.write(json_bytes(error.metadata))
        raise SystemExit(str(error)) from error
    except (OSError, ValueError) as error:
        raise SystemExit(str(error)) from error
    with graph_file.open('xb') as output:
        output.write(json_bytes(graph))
    try:
        with meta_file.open('xb') as output:
            output.write(json_bytes(meta))
    except OSError:
        graph_file.unlink()  # remove somente o arquivo recém-criado por esta chamada
        raise
    print(f'{graph_file}\n{meta_file}\nGrafo SINTÁTICO: ativação não avaliada; consulte os metadados.')


if __name__ == '__main__':
    main()
