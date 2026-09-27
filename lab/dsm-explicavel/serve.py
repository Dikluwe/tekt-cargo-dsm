#!/usr/bin/env python3
"""Servidor local somente leitura de um pacote previamente calculado."""
# Crystalline Lineage — @prompt lab/dsm-explicavel/CONTRATO-DOXYGEN.md
import hashlib
import tempfile
import argparse
import json
from pathlib import Path
import subprocess
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent


def documentation(data, binary, graph, inventory, catalog, matches, explorer):
    paths = [graph, inventory, catalog, matches, explorer]
    if not any(paths):
        return None
    if not all(paths):
        raise ValueError('Documentação exige graph, inventory, catalog, matches e explorer')
    gb, ib, cb, mb = [p.read_bytes() for p in paths[:4]]
    cat = json.loads(cb)
    gd = 'sha256:' + hashlib.sha256(gb).hexdigest()
    if cat.get('graph_digest') != gd or json.loads(gb) != data['source_graph']:
        raise ValueError('digest: grafo documental diverge do snapshot exibido')
    if sorted(n['node_id'] for n in cat['symbols']) != sorted(n['id'] for n in data['source_graph']['nodes']):
        raise ValueError('Catálogo não cobre exatamente os IDs do grafo exibido')
    with tempfile.TemporaryDirectory(prefix='lente-dsm-documentacao-') as d:
        d = Path(d)
        for name, content in [('inventory', ib), ('catalog', cb), ('matches', mb)]:
            (d / name).write_bytes(content)
        (d / 'dsm.json').write_text(json.dumps({'format_version': 1, 'metadata': data['metadata'], 'graph': json.loads(gb)}))
        expected = subprocess.run([str(binary), 'prepare', str(d / 'dsm.json')], capture_output=True, timeout=30)
        if expected.returncode:
            raise ValueError('DSM: ' + expected.stderr.decode(errors='replace'))
        expected = json.loads(expected.stdout)
        for key in ('items', 'cells', 'aggregated_graph', 'relations', 'excluded_items', 'weight_semantics'):
            if expected[key] != data[key]:
                raise ValueError('DSM diverge do grafo verificado: ' + key)
        result = subprocess.run([str(explorer), '--inventory', str(d / 'inventory'), '--catalog', str(d / 'catalog'),
                                 '--matches', str(d / 'matches'), '--output', str(d / 'panel')], capture_output=True, timeout=30)
        if result.returncode:
            raise ValueError(f'Explorador status {result.returncode}: ' + result.stderr.decode(errors='replace'))
        panel = json.loads((d / 'panel').read_bytes())
        if panel.get('graph_digest') != gd:
            raise ValueError('digest: painel diverge do grafo')
        panel['graph_binding'] = 'verified_exact_graph_bytes'
        return panel


def handler_for(bundle, binary, graph=None, inventory=None, catalog=None, matches=None, explorer=None):
    # Congela os bytes da análise usada na tela e nas consultas desta sessão.
    payload = bundle.read_bytes()
    data = json.loads(payload)
    if data.get('format_version') != 1:
        raise ValueError('Versão de pacote incompatível')

    panel = documentation(data, binary, graph, inventory, catalog, matches, explorer)

    class Handler(BaseHTTPRequestHandler):
        def reply(self, status, body, mime='application/json; charset=utf-8'):
            self.send_response(status)
            self.send_header('Content-Type', mime)
            self.send_header('Content-Length', str(len(body)))
            self.send_header('Cache-Control', 'no-store')
            self.send_header('X-Content-Type-Options', 'nosniff')
            self.send_header('Content-Security-Policy', "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'none'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'")
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            route = urlsplit(self.path).path
            if route == '/api/data':
                # O grafo original é usado somente no backend para consulta.
                public = {k: v for k, v in data.items() if k != 'source_graph'}
                if panel is not None:
                    public['documentation'] = panel
                return self.reply(200, json.dumps(public, ensure_ascii=False).encode())
            assets = {'/': ('index.html', 'text/html'), '/app.js': ('app.js', 'text/javascript'), '/style.css': ('style.css', 'text/css')}
            if route not in assets:
                return self.reply(404, b'{"error":"Recurso inexistente"}')
            file, mime = assets[route]
            self.reply(200, (ROOT / 'web' / file).read_bytes(), mime + '; charset=utf-8')

        def do_POST(self):
            if self.path != '/api/path':
                return self.reply(404, b'{"error":"Recurso inexistente"}')
            try:
                length = int(self.headers.get('Content-Length', '0'))
                if length < 1 or length > 8192:
                    raise ValueError('Consulta inválida')
                q = json.loads(self.rfile.read(length))
                if not isinstance(q, dict):
                    raise ValueError('Consulta deve ser um objeto')
                if any(type(q.get(k)) is not int or q[k] < 0 for k in ('consumer', 'changed')):
                    raise ValueError('IDs inválidos')
                # Não consulta outro arquivo nem aceita comando fornecido pelo cliente.
                # stdin mantém o mesmo snapshot congelado da interface.
                result = subprocess.run([str(binary), 'query-stdin', str(q['consumer']), str(q['changed'])],
                                        input=payload, capture_output=True, timeout=30, check=False)
                if result.returncode:
                    raise ValueError(result.stderr.decode(errors='replace').strip())
                self.reply(200, result.stdout)
            except (ValueError, KeyError, subprocess.TimeoutExpired) as error:
                self.reply(400, json.dumps({'error': str(error)}, ensure_ascii=False).encode())

    return Handler


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/debug/dsm-explicavel')
    parser.add_argument('--port', type=int, default=8765)
    for option in ("graph", "inventory", "catalog", "matches", "explorer"):
        parser.add_argument("--" + option, type=Path)
    args = parser.parse_args()
    server = ThreadingHTTPServer(('127.0.0.1', args.port), handler_for(args.bundle, args.binary.resolve(), args.graph, args.inventory, args.catalog, args.matches, args.explorer.resolve() if args.explorer else None))
    print(f'DSM independente: http://127.0.0.1:{server.server_port}', flush=True)
    server.serve_forever()


if __name__ == '__main__':
    main()
