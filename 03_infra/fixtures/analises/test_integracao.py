# Crystalline Lineage
# @prompt 00_nucleo/prompts/fixtures-analises.md
# @prompt-hash d28b5ef6
# @layer L3
# @updated 2026-09-26
"""Testes de integração com executáveis reais, sem dependências Python externas."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
CLI = ROOT / "target/debug/lente"
MCP = ROOT / "target/debug/lente-mcp"
TOOLS = {
    "snapshots": ROOT / "lab/impacto-snapshots/cli/target/debug/lente-impacto-snapshots-cli",
    "regras": ROOT / "lab/regras-arquiteturais/target/debug/regras-arquiteturais",
    "dsm": ROOT / "lab/dsm-explicavel/target/debug/dsm-explicavel",
}
TOOLS["caminho-dsm"] = TOOLS["dsm"]


def node(i, path, kind="fn"):
    return dict(id=i, path=path, name=path, kind=kind, visibility="pub")


def edge(a, b, relation="uses"):
    return dict(id_from=a["id"], id_to=b["id"], **{
        "from": a["path"], "to": b["path"], "relation": relation,
        "uses_kind": "reference" if relation == "uses" else None})


def snapshot(version, nodes, edges, completeness="completa"):
    return dict(versao=version, grafo=dict(crate="x", nodes=nodes, edges=edges),
                granularidade="item", natureza="reference", completude=completeness,
                diagnosticos=["extração parcial"] if completeness == "parcial" else [])


class Integracao(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        for binary in {CLI, MCP, *TOOLS.values()}:
            if not binary.is_file():
                raise RuntimeError(f"compile primeiro: {binary}")

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="lente-analises-test-")
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)

    def save(self, name, value):
        path = self.directory / name
        path.write_text(json.dumps(value, ensure_ascii=False), encoding="utf-8")
        return str(path)

    def invoke(self, consulta, code=0, invalid=False, executable=None):
        config = dict(executavel=str(executable or TOOLS[consulta["modo"]]),
                      timeout_ms=15000, consulta=consulta)
        path = self.save("config.json", config)
        cli = subprocess.run([str(CLI), "--analise-artefatos", path],
                             capture_output=True, text=True, timeout=25)
        self.assertEqual(cli.returncode, code, cli.stderr)
        request = dict(jsonrpc="2.0", id=1, method="tools/call",
                       params=dict(name="analisar_artefatos", arguments=config))
        mcp = subprocess.run([str(MCP)], input=json.dumps(request) + "\n",
                             capture_output=True, text=True, timeout=25)
        self.assertEqual(mcp.returncode, 0, mcp.stderr)
        result = json.loads(mcp.stdout)["result"]
        self.assertEqual(result["isError"], invalid, result)
        if invalid:
            self.assertEqual(cli.stdout, "")
            self.assertTrue(cli.stderr.strip())
            return
        cli_json = json.loads(cli.stdout)
        self.assertEqual(cli_json, json.loads(result["content"][0]["text"]))
        self.assertEqual(cli_json["execucao"]["codigo_saida"], code)
        return cli_json["analise"]

    def comparison(self, partial=False, removal=False):
        target, caller = node(1, "x::target"), node(2, "x::caller")
        new_target, new_caller = node(100, "x::renamed"), node(200, "x::caller")
        pairs = [dict(identidade="caller", antes=2, depois=200)]
        changes = [dict(tipo="remocao", antes=1)] if removal else [dict(
            tipo="pareada", identidade="target", alteracao_declarada=False,
            renomeacao_declarada=True)]
        if not removal:
            pairs.append(dict(identidade="target", antes=1, depois=100))
        return dict(format_version=1,
                    antes=snapshot("v1", [target, caller], [edge(caller, target)]),
                    depois=snapshot("v2", [new_caller] if removal else [new_target, new_caller],
                                    [] if partial or removal else [edge(new_caller, new_target)],
                                    "parcial" if partial else "completa"),
                    correspondencias=pairs, mudancas=changes)

    def compare(self, value):
        return self.invoke(dict(modo="snapshots", entrada=self.save("input.json", value)))

    def test_removal_keeps_previous_consumers(self):
        change = self.compare(self.comparison(removal=True))["mudancas"][0]
        self.assertIsNone(change["depois"])
        self.assertEqual(change["antes"]["consumidores"][0]["elemento"]["path"], "x::caller")

    def test_partial_is_not_proven_absence(self):
        result = self.compare(self.comparison(partial=True))
        self.assertEqual(result["depois"]["origem"]["completude"], "parcial")
        self.assertEqual(result["depois"]["diagnosticos"], ["extração parcial"])
        observation = result["mudancas"][0]["consumidores"]["somente_antes"][0]
        self.assertEqual(observation["estado_outro_lado"], "nao_observado_com_cobertura_limitada")

    def test_rename_uses_explicit_correspondence(self):
        change = self.compare(self.comparison())["mudancas"][0]
        self.assertEqual(change["topologia_incidente"]["status"], "mesmas_relacoes_observadas")
        self.assertEqual(change["consumidores"]["ambos"][0]["identidade"], "caller")

    def rule_config(self, current="current.json", rules="rules.json"):
        examples = ROOT / "lab/regras-arquiteturais/examples"
        return dict(modo="regras", baseline=str(examples / "baseline.json"),
                    atual=str(examples / current), regras=str(examples / rules), data="2026-09-26")

    def test_rules_domain_codes(self):
        for code, status, current, rules in [
            (0, "aprovado", "current.json", "allowed-rules.json"),
            (1, "violacao", "current.json", "rules.json"),
            (2, "inconclusivo", "partial.json", "rules.json"),
        ]:
            with self.subTest(code=code):
                self.assertEqual(self.invoke(self.rule_config(current, rules), code)["status"], status)

    def test_invalid_date_and_missing_executable(self):
        config = self.rule_config()
        config["data"] = "2026-02-30"
        self.invoke(config, 3, invalid=True)
        self.invoke(self.rule_config(), 3, invalid=True, executable=self.directory / "absent")

    def test_dsm_prepare_and_directional_query(self):
        root = node(1, "x", "crate")
        modules = [node(i, f"x::{name}", "mod") for i, name in [(10, "ui"), (20, "service"), (30, "storage")]]
        items = [node(m["id"] + 1, m["path"] + "::f") for m in modules]
        edges = [edge(root, m, "owns") for m in modules]
        edges += [edge(m, i, "owns") for m, i in zip(modules, items)]
        edges += [edge(items[0], items[1]), edge(items[1], items[2])]
        metadata = dict(provider="fixture-local", language="Rust", granularity="item", configuration="test",
                        execution="partial", semantic_coverage="static references", diagnostics=["arquivo não extraído"])
        source = dict(format_version=1, metadata=metadata,
                      graph=dict(crate="x", nodes=[root, *modules, *items], edges=edges))
        bundle = self.invoke(dict(modo="dsm", entrada=self.save("dsm.json", source)))
        self.assertEqual(bundle["metadata"], metadata)
        self.assertTrue(bundle["cells"])
        self.assertEqual(bundle["source_graph"], source["graph"])
        path = self.save("bundle.json", bundle)
        found = self.invoke(dict(modo="caminho-dsm", pacote=path, consumidor=11, alterado=31))
        self.assertEqual(found["status"], "found")
        self.assertEqual(found["hops"], 2)
        reverse = self.invoke(dict(modo="caminho-dsm", pacote=path, consumidor=31, alterado=11))
        self.assertEqual(reverse["status"], "no_path")

    def test_cli_modes_are_exclusive(self):
        result = subprocess.run([str(CLI), "--analise-artefatos", "unused.json", "--ranking"],
                                capture_output=True, text=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot be used", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
