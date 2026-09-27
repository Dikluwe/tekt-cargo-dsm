# Crystalline Lineage
# @prompt lab/adaptador-zig/CONTRATO.md
# @layer Lab
# @updated 2026-09-26
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from extrair import BASE, ExtractionError, extract, json_bytes, parse_file

FIXTURE = BASE / 'fixtures/basico'


class AdaptadorZig(unittest.TestCase):
    def fixture(self, sources, modules=None):
        (BASE / '.tmp').mkdir(exist_ok=True)
        tmp = tempfile.TemporaryDirectory(dir=BASE / '.tmp', prefix='test-')
        self.addCleanup(tmp.cleanup)
        root = pathlib.Path(tmp.name)
        for name, content in sources.items():
            file = root / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text(content, encoding='utf-8')
        (root / 'modules.json').write_bytes(json_bytes(modules or {}))
        return root

    def basic(self):
        return extract(FIXTURE / 'root.zig', FIXTURE / 'modules.json')

    def test_relativo_nomeado_e_endpoints(self):
        graph, metadata = self.basic()
        self.assertEqual(len(graph['nodes']), 6)
        self.assertEqual(len(graph['edges']), 6)
        nodes = {n['id']: n for n in graph['nodes']}
        for n in nodes.values():
            self.assertEqual(n['kind'], 'mod')
        for e in graph['edges']:
            self.assertEqual(e['from'], nodes[e['id_from']]['path'])
            self.assertEqual(e['to'], nodes[e['id_to']]['path'])
            self.assertEqual((e['relation'], e['uses_kind']), ('uses', 'import'))
        named = next(o for o in metadata['occurrences'] if o['literal'] == 'pacote')
        self.assertEqual(named['status'], 'candidate')
        self.assertTrue(named['target'].endswith('modulos/pacote.zig'))

    def test_std_builtin_politica_explicita(self):
        graph, metadata = self.basic()
        occurrences = [o for o in metadata['occurrences'] if o['literal'] in ('std', 'builtin')]
        self.assertEqual(len(occurrences), 2)
        self.assertTrue(all(o['status'] == 'external-standard' for o in occurrences))
        self.assertFalse(any(n['name'] in ('std', 'builtin') for n in graph['nodes']))

    def test_import_repetido_uma_aresta_ocorrencias_preservadas(self):
        root = self.fixture({'root.zig': 'const a=@import("local.zig"); const b=@import("local.zig");', 'local.zig': 'pub const n=1;'})
        graph, metadata = extract(root / 'root.zig', root / 'modules.json')
        self.assertEqual(len(graph['edges']), 1)
        self.assertEqual(len(metadata['occurrences']), 2)

    def test_ausente_e_gerado_com_diagnostico(self):
        _, metadata = self.basic()
        codes = {d['code'] for d in metadata['diagnostics']}
        self.assertIn('unmapped-module', codes)
        self.assertIn('generated-module-missing', codes)
        self.assertEqual(metadata['completeness']['syntax'], 'partial')
        self.assertTrue(all('file' in d and 'line' in d for d in metadata['diagnostics']))

    def test_condicional_comptime_nao_avaliado(self):
        _, metadata = self.basic()
        for target in ('linux.zig', 'outro.zig'):
            occurrence = next(o for o in metadata['occurrences'] if o['literal'] == target)
            self.assertEqual(occurrence['status'], 'candidate')
            self.assertEqual(occurrence['activation'], 'unknown')
            self.assertIn('if', occurrence['contexts'])
        occurrence = next(o for o in metadata['occurrences'] if o['literal'] == 'comptime.zig')
        self.assertIn('comptime', occurrence['contexts'])
        self.assertEqual(metadata['completeness']['semantics'], 'not-evaluated')
        self.assertTrue(all(o['activation'] == 'unknown' for o in metadata['occurrences']))

    def test_cimport_fronteira_sem_codigo_c(self):
        graph, metadata = self.basic()
        occurrence = next(o for o in metadata['occurrences'] if o['builtin'] == '@cImport')
        self.assertEqual(occurrence['status'], 'external-c-boundary')
        self.assertTrue(any(d['code'] == 'c-import-external' for d in metadata['diagnostics']))
        self.assertFalse(any(n['name'].endswith('.h') for n in graph['nodes']))

    def test_parser_nao_confunde_comentarios_e_strings(self):
        _, metadata = self.basic()
        literals = [o['literal'] for o in metadata['occurrences']]
        self.assertNotIn('nao_importar.zig', literals)
        self.assertNotIn('tambem_nao_importar.zig', literals)

    def test_expressao_nao_literal_escape_e_multiline(self):
        root = self.fixture({'root.zig': 'const a=@import("loc\\x61l.zig");\nconst b=@import("local" ++ ".zig");\nconst c=@import(\n    \\\\local.zig\n);', 'local.zig': 'pub const n=1;'})
        graph, metadata = extract(root / 'root.zig', root / 'modules.json')
        self.assertEqual(len(graph['edges']), 1)
        self.assertEqual([o['status'] for o in metadata['occurrences']], ['candidate', 'nonliteral-unresolved', 'candidate'])
        self.assertEqual(metadata['occurrences'][2]['literal'], 'local.zig')

    def test_determinismo_hash_e_sem_cache(self):
        a = self.basic()
        b = self.basic()
        self.assertEqual(json_bytes(a), json_bytes(b))
        self.assertEqual(a[1]['graph_sha256'], hashlib.sha256(json_bytes(a[0])).hexdigest())
        root = self.fixture({'root.zig': 'pub const a=1;', 'local.zig': 'pub const a=1;'})
        self.assertEqual(len(extract(root/'root.zig', root/'modules.json')[0]['edges']), 0)
        (root/'root.zig').write_text('const a=@import("local.zig");')
        self.assertEqual(len(extract(root/'root.zig', root/'modules.json')[0]['edges']), 1)

    def test_root_alias_e_ciclo_sem_loop_infinito(self):
        root = self.fixture({'root.zig': 'const a=@import("local.zig");', 'local.zig': 'const a=@import("root");'})
        graph, _ = extract(root/'root.zig', root/'modules.json')
        self.assertEqual(len(graph['nodes']), 2)
        self.assertEqual(len(graph['edges']), 2)

    def test_erro_sintaxe_raiz_fatal_dependencia_parcial(self):
        root = self.fixture({'root.zig': 'const a=@import("bad.zig");', 'bad.zig': 'const = ;'})
        graph, metadata = extract(root/'root.zig', root/'modules.json')
        self.assertEqual(len(graph['nodes']), 1)
        self.assertEqual(graph['edges'], [])
        self.assertEqual(metadata['occurrences'][0]['status'], 'target-analysis-failed')
        self.assertEqual(metadata['completeness']['syntax'], 'partial')
        with self.assertRaises(ExtractionError):
            extract(root/'bad.zig', root/'modules.json')

    def test_embedfile_e_zon_fronteira_dados(self):
        root = self.fixture({'root.zig': 'const a=@import("a.zon"); const b=@embedFile("a.txt");'})
        graph, metadata = extract(root/'root.zig', root/'modules.json')
        self.assertEqual(graph['edges'], [])
        self.assertEqual([o['status'] for o in metadata['occurrences']], ['unsupported-data', 'external-data-boundary'])

    def test_cli_erro_e_recusa_sobrescrita(self):
        root = self.fixture({'root.zig': 'const = ;'})
        out = root/'out'
        command = [sys.executable, str(BASE/'extrair.py'), '--root', str(root/'root.zig'), '--modules', str(root/'modules.json'), '--out', str(out)]
        self.assertEqual(subprocess.run(command, capture_output=True).returncode, 1)
        self.assertFalse((out/'grafo.json').exists())
        self.assertEqual(json.loads((out/'metadados.json').read_text())['completeness']['syntax'], 'error')
        again = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(again.returncode, 1)
        self.assertIn('Saída já existe', again.stderr)

    def test_versao_helper_e_nomes_reservados(self):
        fake = subprocess.CompletedProcess([], 0, '{"zig_version":"0.14.0","parser":"std.zig.Ast"}', '')
        with patch('extrair.subprocess.run', return_value=fake):
            with self.assertRaises(RuntimeError):
                parse_file('unused', BASE/'bin/parser')
        root = self.fixture({'root.zig': 'pub const n=1;'}, {'std': 'root.zig'})
        with self.assertRaises(ValueError):
            extract(root/'root.zig', root/'modules.json')


if __name__ == '__main__':
    unittest.main(verbosity=2)
