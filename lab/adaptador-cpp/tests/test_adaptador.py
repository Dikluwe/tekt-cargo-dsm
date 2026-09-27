# Crystalline Lineage
# @prompt lab/adaptador-cpp/README.md
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from adaptador import analyze, encode, load_commands
from make_database import create

LIBRARY = os.environ.get("LIBCLANG_PATH", "/usr/lib/llvm-18/lib/libclang.so")
FIXTURES = Path(__file__).parent / "fixtures"


class CppTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not Path(LIBRARY).is_file():
            raise unittest.SkipTest("libclang ausente: informe LIBCLANG_PATH, nada será instalado")
        cls.temp = tempfile.TemporaryDirectory(dir=Path(__file__).parent)
        cls.root = Path(cls.temp.name).resolve()
        shutil.copytree(FIXTURES, cls.root, dirs_exist_ok=True)
        cls.db = cls.root / "compile_commands.json"
        cls.entries = create(cls.root, cls.db)
        cls.graph, cls.meta = analyze(cls.db, cls.root, LIBRARY)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def evidence(self, nature=None, source=None, declaration=None):
        return [e for e in self.meta["semantic_evidence"] if
                (nature is None or e["nature"] == nature) and
                (source is None or e["source"]["file"].endswith(source)) and
                (declaration is None or e["declaration"] == declaration)]

    def test_tipo_em_header(self):
        found = self.evidence("TypeRef", "one.cpp", "Item")
        self.assertEqual(len(found), 1)
        self.assertTrue(found[0]["target"]["file"].endswith("types.hpp"))
        self.assertGreater(found[0]["source"]["line"], 0)
        self.assertGreater(found[0]["source"]["column"], 0)

    def test_chamada_declara_header_nao_redireciona_definicao(self):
        calls = self.evidence("static_call", "one.cpp", "declared(const Item &)")
        self.assertEqual(len(calls), 1)
        self.assertTrue(calls[0]["target"]["file"].endswith("types.hpp"))
        self.assertFalse(calls[0]["runtime_target"])
        mapping = self.evidence("declaration_definition", "definitions.cpp", "declared(const Item &)")
        self.assertEqual(mapping[0]["usr"], calls[0]["usr"])
        self.assertTrue(any(s["is_definition"] and s["usr"] == calls[0]["usr"] and s["location"]["file"].endswith("definitions.cpp") for s in self.meta["symbols"]))

    def test_overloads_resolvidos_por_usr(self):
        a = self.evidence("static_call", "one.cpp", "overloaded(int)")
        b = self.evidence("static_call", "one.cpp", "overloaded(double)")
        self.assertEqual((len(a), len(b)), (1, 1))
        self.assertNotEqual(a[0]["usr"], b[0]["usr"])

    def test_template_padrao_e_referencia_a_instanciacao(self):
        pattern = self.evidence("static_call", "templates.hpp", "declared(const Item &)")
        self.assertEqual(pattern[0]["template_origin"], "pattern")
        specialized = [e for e in self.evidence("static_call", "one.cpp") if e["declaration"].startswith("generic<")]
        self.assertEqual(specialized[0]["target_template"], "instantiation_or_specialization")
        # O corpo dependente não foi resolvido pelo cursor do padrão: diagnosticar.
        unresolved = [d for d in self.meta["diagnostics"] if d["code"] == "unresolved_reference"]
        self.assertTrue(any(d["template_origin"] == "pattern" for d in unresolved))
        self.assertEqual(self.meta["semantic_coverage"], "partial")

    def test_virtual_e_indireta_nao_prometem_runtime(self):
        virtual = self.evidence("virtual_dispatch_static_declaration", "one.cpp", "run()")
        self.assertEqual(len(virtual), 1)
        self.assertFalse(virtual[0]["runtime_target"])
        self.assertFalse(self.evidence("static_call", "one.cpp", "run()"))
        codes = {d["code"] for d in self.meta["diagnostics"]}
        self.assertTrue({"virtual_call_no_runtime_target", "indirect_call_no_runtime_target"} <= codes)
        self.assertFalse(self.evidence("static_call", "one.cpp", "callback"))

    def test_header_deduplicado_por_evidencia_e_aresta(self):
        shared = self.evidence("static_call", "shared.hpp", "declared(const Item &)")
        self.assertEqual(len(shared), 1)
        self.assertEqual(len(shared[0]["observed_in"]), 2)
        edges = [(e["from"], e["to"]) for e in self.graph["edges"]]
        self.assertEqual(len(edges), len(set(edges)))
        self.assertEqual(edges.count(("cpp::include/shared.hpp", "cpp::include/types.hpp")), 1)

    def test_include_sem_referencia_nao_vira_dependencia_semantica(self):
        self.assertTrue(any(e["target_file"].endswith("unused.hpp") for e in self.meta["includes"]))
        self.assertFalse(any(e["to"].endswith("unused.hpp") for e in self.graph["edges"]))

    def test_diretorio_flags_e_endpoints(self):
        self.assertEqual(self.meta["execution"]["status"], "complete")
        self.assertEqual(self.meta["execution"]["translation_units_parsed"], 3)
        nodes = {n["id"]: n for n in self.graph["nodes"]}
        self.assertTrue(all(n["kind"] == "mod" for n in nodes.values()))
        for edge in self.graph["edges"]:
            self.assertEqual(edge["from"], nodes[edge["id_from"]]["path"])
            self.assertEqual(edge["to"], nodes[edge["id_to"]]["path"])
            self.assertEqual(edge["relation"], "uses")
            self.assertIsNone(edge["uses_kind"])
        self.assertFalse(any(self.root.glob("*.o")))
        self.assertFalse(self.meta["cache"])

    def test_determinismo_reordenando_unidades(self):
        alternate = self.root / "reversed.json"
        alternate.write_text(json.dumps(list(reversed(self.entries))))
        graph, meta = analyze(alternate, self.root, LIBRARY)
        self.assertEqual(encode(graph), encode(self.graph))
        self.assertEqual(encode(meta), encode(self.meta))

    def test_configuracoes_exigem_selecao(self):
        entries = json.loads(json.dumps(self.entries))
        other = dict(entries[0], arguments=[*entries[0]["arguments"], "-DOTHER=1"])
        entries.append(other)
        db = self.root / "multi.json"
        db.write_text(json.dumps(entries))
        with self.assertRaisesRegex(ValueError, "configurações distintas"):
            analyze(db, self.root, LIBRARY)
        config = load_commands(self.db)[0]["configuration"]
        graph, meta = analyze(db, self.root, LIBRARY, config)
        self.assertEqual(graph, self.graph)
        self.assertEqual(len(meta["other_configurations"]), 1)

    def test_comando_nunca_executado_e_flag_desconhecida_rejeitada(self):
        marker = self.root / "SHOULD_NOT_EXIST"
        db = self.root / "command.json"
        db.write_text(json.dumps([dict(directory=str(self.root), file="src/two.cpp", command=f"clang++ -std=c++17 -Iinclude -c src/two.cpp ; touch {marker}")]))
        with self.assertRaises(ValueError):
            analyze(db, self.root, LIBRARY)
        self.assertFalse(marker.exists())
        entries = json.loads(json.dumps(self.entries))
        entries[0]["arguments"].extend(["-Xclang", "-load", "untrusted.so"])
        db.write_text(json.dumps(entries))
        with self.assertRaisesRegex(ValueError, "flag não suportada"):
            analyze(db, self.root, LIBRARY)

    def test_command_posix_sem_shell(self):
        db = self.root / "posix.json"
        import shlex
        entries = [dict(directory=e["directory"], file=e["file"], command=shlex.join(e["arguments"])) for e in self.entries]
        db.write_text(json.dumps(entries))
        self.assertEqual(analyze(db, self.root, LIBRARY)[0], self.graph)

    def test_erro_clang_descarta_tu_e_declara_execucao_parcial(self):
        bad = self.root / "bad.cpp"
        bad.write_text("int broken( {\n")
        db = self.root / "bad.json"
        db.write_text(json.dumps([dict(directory=str(self.root), file="bad.cpp", arguments=["clang++", "-std=c++17", "-c", "bad.cpp"])]))
        graph, meta = analyze(db, self.root, LIBRARY)
        self.assertEqual(meta["execution"]["status"], "partial")
        self.assertEqual(graph["edges"], [])
        self.assertTrue(any(d["code"] == "translation_unit_discarded" for d in meta["diagnostics"]))

    def test_macro_diagnosticada_e_configuracao_otimizacao_preservada(self):
        self.assertTrue(any(d["code"] == "macro_expansion_partial" for d in self.meta["diagnostics"]))
        entries = json.loads(json.dumps(self.entries))
        for entry in entries:
            entry["arguments"].append("-O2")
        db = self.root / "optimized.json"
        db.write_text(json.dumps(entries))
        commands = load_commands(db)
        self.assertTrue(all("-O2" in c["config"]["arguments"] for c in commands))
        self.assertNotEqual(commands[0]["configuration"], self.meta["configuration"])

    def test_modulo_cpp20_sem_suporte(self):
        source = self.root / "test.cppm"
        source.write_text("export module test;\nexport int f();\n")
        db = self.root / "module.json"
        db.write_text(json.dumps([dict(directory=str(self.root), file="test.cppm", arguments=["clang++", "-std=c++20", "-c", "test.cppm"])]))
        graph, meta = analyze(db, self.root, LIBRARY)
        self.assertEqual(meta["execution"]["status"], "partial")
        self.assertTrue(any(d["code"] == "cpp20_modules_unsupported" for d in meta["diagnostics"]))
        self.assertFalse(meta["capabilities"]["cpp20_modules"])


if __name__ == "__main__":
    unittest.main()
