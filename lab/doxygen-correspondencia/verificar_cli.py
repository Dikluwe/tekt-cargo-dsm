#!/usr/bin/env python3
# Crystalline Lineage
# @prompt lab/doxygen-correspondencia/CONTRATO.md
"""Testes da CLI com fixtures próprias, Python stdlib, sem instalação/download."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent
BIN = ROOT / 'target/debug/lente-doxygen-correspondencia'


class CliTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(dir=ROOT / 'target')
        self.addCleanup(self.tmp.cleanup)
        self.path = Path(self.tmp.name)
        self.inv = self.path / 'inventory.json'
        self.cat = self.path / 'catalog.json'
        self.out = self.path / 'matches.json'
        self.inv.write_bytes((ROOT / 'fixtures/inventory.json').read_bytes())
        self.cat.write_bytes((ROOT / 'fixtures/catalog.json').read_bytes())

    def call(self):
        return subprocess.run([BIN, '--inventory', self.inv, '--catalog', self.cat, '--output', self.out], capture_output=True, text=True)

    def test_exact_file_digests_and_expected_fixture(self):
        self.assertEqual(self.call().returncode, 0)
        raw = self.out.read_bytes()
        result = json.loads(raw)
        for key, file in [('inventory_digest', self.inv), ('catalog_digest', self.cat)]:
            self.assertEqual(result[key], 'sha256:' + hashlib.sha256(file.read_bytes()).hexdigest())
        self.assertEqual(raw, (ROOT / 'fixtures/matches.json').read_bytes())
        self.out.unlink()
        self.inv.write_bytes(self.inv.read_bytes() + b' \r\n')
        self.assertEqual(self.call().returncode, 0)
        changed = json.loads(self.out.read_bytes())
        self.assertNotEqual(changed['inventory_digest'], result['inventory_digest'])
        self.assertEqual(changed['matches'], result['matches'])
        self.assertEqual(changed['inventory_digest'], 'sha256:' + hashlib.sha256(self.inv.read_bytes()).hexdigest())

    def test_existing_output_and_input_not_overwritten(self):
        self.out.write_bytes(b'preservar')
        self.assertNotEqual(self.call().returncode, 0)
        self.assertEqual(self.out.read_bytes(), b'preservar')
        self.out = self.inv
        original = self.inv.read_bytes()
        self.assertNotEqual(self.call().returncode, 0)
        self.assertEqual(self.inv.read_bytes(), original)

    def test_invalid_input_has_no_output(self):
        for replacement in [b'{', b'{"schema":"wrong","version":1}', b'{"schema":"lente.doxygen.inventory","version":2}']:
            self.inv.write_bytes(replacement)
            result = self.call()
            self.assertNotEqual(result.returncode, 0)
            self.assertTrue(result.stderr.strip())
            self.assertFalse(self.out.exists())

    def test_duplicate_ids_fail_without_report(self):
        v = json.loads(self.cat.read_bytes())
        v['symbols'].append(v['symbols'][0])
        self.cat.write_text(json.dumps(v))
        result = self.call()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('duplicate_node_id', result.stderr)
        self.assertFalse(self.out.exists())

    def test_argument_validation(self):
        self.assertEqual(subprocess.run([BIN, '--help'], capture_output=True).returncode, 0)
        result = subprocess.run([BIN, '--inventory', self.inv, '--inventory', self.inv, '--output', self.out], capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.out.exists())


if __name__ == '__main__':
    unittest.main(verbosity=2)
