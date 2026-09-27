"""Integração local, sem dependências Python externas."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen
from http.server import ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/debug/dsm-explicavel'
spec = importlib.util.spec_from_file_location('dsm_server', ROOT / 'serve.py')
server_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(server_module)


class ServerTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(dir=ROOT / 'artifacts')
        cls.bundle = Path(cls.temp.name) / 'bundle.json'
        cls.bundle.write_bytes(subprocess.check_output([BINARY, 'prepare', ROOT / 'fixtures/partial.json']))
        cls.server = ThreadingHTTPServer(('127.0.0.1', 0), server_module.handler_for(cls.bundle, BINARY))
        cls.url = 'http://127.0.0.1:' + str(cls.server.server_port)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.thread.join()
        cls.server.server_close()
        cls.temp.cleanup()

    def query(self, consumer, changed):
        req = Request(self.url + '/api/path', data=json.dumps(dict(consumer=consumer, changed=changed)).encode(), headers={'Content-Type': 'application/json'})
        with urlopen(req) as response:
            return json.load(response)

    def test_contract_metadata_special_characters(self):
        with urlopen(self.url + '/api/data') as response:
            data = json.load(response)
            self.assertEqual(data['metadata']['execution'], 'partial')
            self.assertIn('<local> & "auditável"', data['metadata']['provider'])
            self.assertNotIn('source_graph', data)
            self.assertEqual(len(data['excluded_items']), 1)
            self.assertTrue(all(r['reference_position'] is None for r in data['relations']))
            self.assertEqual(response.headers['Cache-Control'], 'no-store')
            self.assertIn("script-src 'self'", response.headers['Content-Security-Policy'])

    def test_query_direction_cycle_and_no_path(self):
        self.assertEqual(self.query(11, 31)['path'], ['app::ui::render <&>', 'app::service::load', 'app::storage::save'])
        self.assertEqual(self.query(31, 11)['status'], 'no_path')
        self.assertEqual(self.query(41, 31)['hops'], 2)
        self.assertEqual(self.query(21, 21)['hops'], 0)

    def test_missing_endpoint_is_not_no_path(self):
        with self.assertRaises(HTTPError) as caught:
            self.query(999, 31)
        self.assertEqual(caught.exception.code, 400)
        self.assertIn('Consumidor inexistente', json.load(caught.exception)['error'])

    def test_invalid_request_is_diagnostic(self):
        for body in [b'[]', b'{"consumer":"x","changed":31}', b'{']:
            with self.assertRaises(HTTPError) as caught:
                urlopen(Request(self.url + '/api/path', data=body))
            self.assertEqual(caught.exception.code, 400)

    def test_assets_no_inline_data_or_external_scripts(self):
        with urlopen(self.url + '/') as response:
            html = response.read().decode()
            self.assertNotIn('alert(', html)
            self.assertIn('src="/app.js"', html)
            self.assertNotIn('https://', html)
        with urlopen(self.url + '/app.js') as response:
            js = response.read().decode()
            self.assertNotIn('.innerHTML', js)
            self.assertIn('node.textContent = text', js)
            self.assertIn('slice(shown, shown + 20)', js)

    def test_no_arbitrary_file_route(self):
        with self.assertRaises(HTTPError) as caught:
            urlopen(self.url + '/README.md')
        self.assertEqual(caught.exception.code, 404)

    def test_frozen_snapshot_does_not_follow_disk_changes(self):
        original = self.bundle.read_bytes()
        try:
            self.bundle.write_text('{}')
            self.assertEqual(self.query(11, 31)['hops'], 2)
        finally:
            self.bundle.write_bytes(original)


if __name__ == '__main__':
    unittest.main()
