# Crystalline Lineage
# @prompt lab/adaptador-python/README.md
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from adaptador import analisar, encode

FIXTURES = Path(__file__).parent / "fixtures"


def pairs(graph):
    return {(e["from"], e["to"]) for e in graph["edges"]}


def codes(metadata):
    return {d["code"] for d in metadata["diagnostics"]}


class AdaptadorTests(unittest.TestCase):
    def setUp(self):
        # Temporários permanecem exclusivamente na pasta autorizada.
        self.temp = tempfile.TemporaryDirectory(dir=Path(__file__).parent)
        self.root = Path(self.temp.name).resolve()
        self.addCleanup(self.temp.cleanup)

    def write(self, name, source):
        file = self.root / name
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(source, encoding="utf-8")
        return file

    def test_absoluto_relativo_pacote_simbolo_externo_e_ciclo(self):
        graph, meta = analisar([FIXTURES / "projeto"])
        edges = pairs(graph)
        self.assertIn(("python::main", "python::pacote::util"), edges)
        self.assertIn(("python::main", "python::pacote"), edges)
        self.assertIn(("python::pacote::util", "python::pacote::vizinho"), edges)
        self.assertIn(("python::pacote::vizinho", "python::pacote::util"), edges)
        self.assertFalse(any("Valor" in n["path"] or "dependencia_externa" in n["path"] for n in graph["nodes"]))
        self.assertIn("external_dependency", codes(meta))
        self.assertEqual(meta["execution"]["status"], "complete")
        self.assertEqual(meta["semantic_coverage"], "partial")

    def test_simbolo_nao_vira_submodulo(self):
        self.write("pkg/__init__.py", "Valor = 1\n")
        self.write("main.py", "from pkg import Valor\n")
        graph, _ = analisar([self.root])
        self.assertEqual(pairs(graph), {("python::main", "python::pkg")})

    def test_submodulo_existente_sem_binding_conflitante(self):
        self.write("pkg/__init__.py", "")
        self.write("pkg/child.py", "")
        self.write("main.py", "from pkg import child as alias\n")
        graph, meta = analisar([self.root])
        self.assertEqual(pairs(graph), {("python::main", "python::pkg"), ("python::main", "python::pkg::child")})
        self.assertNotIn("ambiguous_symbol_or_module", codes(meta))

    def test_erro_de_decodificacao_sem_posicao_inventada(self):
        file = self.write("bad.py", "")
        file.write_bytes(b"\xff\xfe invalid python")
        graph, meta = analisar([self.root])
        self.assertEqual(meta["execution"]["status"], "partial")
        self.assertIsNone(graph["nodes"][0]["position"])

    def test_simbolo_e_submodulo_ambiguos(self):
        self.write("pkg/__init__.py", "util = 1\n")
        self.write("pkg/util.py", "")
        self.write("main.py", "from pkg import util\n")
        graph, meta = analisar([self.root])
        self.assertIn("ambiguous_symbol_or_module", codes(meta))
        self.assertNotIn(("python::main", "python::pkg::util"), pairs(graph))
        self.assertIn(("python::main", "python::pkg"), pairs(graph))

    def test_raizes_ambiguas_sem_precedencia(self):
        self.write("use.py", "import main\n")
        roots = [FIXTURES / "projeto", FIXTURES / "duplicado", self.root]
        graph, meta = analisar(roots)
        self.assertIn("ambiguous_module", codes(meta))
        self.assertIn("ambiguous_import", codes(meta))
        self.assertNotIn("python::main", {n["path"] for n in graph["nodes"]})
        self.assertEqual((graph, meta), analisar(list(reversed(roots))))

    def test_erro_sintatico_nao_impede_outros_arquivos(self):
        self.write("broken.py", "def :\n")
        self.write("good.py", "import broken\n")
        graph, meta = analisar([self.root])
        self.assertEqual(len(graph["nodes"]), 2)
        self.assertIn("syntax_error", codes(meta))
        self.assertEqual(meta["execution"]["status"], "partial")
        self.assertEqual(meta["execution"]["modules_parsed"], 1)

    def test_nunca_executa_codigo(self):
        marker = self.root / "EXECUTADO"
        self.write("danger.py", f"from pathlib import Path\nPath({str(marker)!r}).write_text('erro')\nraise RuntimeError('não execute')\n")
        analisar([self.root])
        self.assertFalse(marker.exists())
        self.assertFalse((self.root / "__pycache__").exists())

    def test_namespace_dynamic_e_relativo_fora_escopo(self):
        self.write("ns/mod.py", "")
        self.write("main.py", "import ns.mod\nfrom . import nope\n__import__('dynamic')\n")
        graph, meta = analisar([self.root])
        self.assertTrue({"namespace_package_unsupported", "dynamic_import_unsupported", "relative_out_of_scope", "unresolved_local_import"} <= codes(meta))
        self.assertEqual(pairs(graph), set())

    def test_relativo_multinivel(self):
        self.write("pkg/__init__.py", "")
        self.write("pkg/sub/__init__.py", "")
        self.write("pkg/sub/use.py", "from ..helpers import X\n")
        self.write("pkg/helpers.py", "X = 1\n")
        graph, _ = analisar([self.root])
        self.assertIn(("python::pkg::sub::use", "python::pkg::helpers"), pairs(graph))

    def test_endpoints_ids_e_formato(self):
        graph, meta = analisar([FIXTURES / "projeto"])
        nodes = {n["id"]: n for n in graph["nodes"]}
        self.assertEqual(set(graph), {"crate", "nodes", "edges"})
        self.assertEqual(sorted(nodes), list(range(len(nodes))))
        self.assertTrue(all(n["kind"] == "mod" and n["visibility"] == "pub" for n in nodes.values()))
        for edge in graph["edges"]:
            self.assertEqual(edge["from"], nodes[edge["id_from"]]["path"])
            self.assertEqual(edge["to"], nodes[edge["id_to"]]["path"])
            self.assertEqual((edge["relation"], edge["uses_kind"]), ("uses", "import"))
        self.assertEqual(meta["graph_sha256"], hashlib.sha256(encode(graph).encode()).hexdigest())
        self.assertFalse(meta["identity"]["persistent"])
        self.assertFalse(meta["cache"])

    def test_deterministico_inclusive_hash_seed_e_pythonpath(self):
        self.write("use.py", "import pacote.util\n")
        outputs = []
        for seed in ("1", "222"):
            graph_file, meta_file = self.root / "graph.json", self.root / "meta.json"
            result = subprocess.run([sys.executable, "-B", str(Path(__file__).parents[1] / "adaptador.py"), "--root", str(self.root), "--graph", str(graph_file), "--metadata", str(meta_file)], env={**os.environ, "PYTHONHASHSEED": seed, "PYTHONPATH": str(FIXTURES / "projeto")}, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            outputs.append((graph_file.read_bytes(), meta_file.read_bytes()))
            self.assertIn("external_dependency", codes(json.loads(meta_file.read_text())))
        self.assertEqual(outputs[0], outputs[1])

    def test_cli_parcial_e_saidas_separadas(self):
        self.write("bad.py", "def :\n")
        script = str(Path(__file__).parents[1] / "adaptador.py")
        args = [sys.executable, "-B", script, "--root", str(self.root), "--graph", str(self.root / "graph.json"), "--metadata", str(self.root / "meta.json")]
        result = subprocess.run(args, capture_output=True)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads((self.root / "meta.json").read_text())["execution"]["status"], "partial")
        args[-1] = args[-3]
        self.assertEqual(subprocess.run(args, capture_output=True).returncode, 2)


if __name__ == "__main__":
    unittest.main()
