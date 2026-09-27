#!/usr/bin/env python3
"""Servidor local somente leitura de um pacote previamente calculado."""
import argparse
import json
from pathlib import Path
import subprocess
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent


def handler_for(bundle, binary):
    # Congela os bytes da análise usada na tela e nas consultas desta sessão.
    payload = bundle.read_bytes()
    data = json.loads(payload)
    if data.get('format_version') != 1:
        raise ValueError('Versão de pacote incompatível')

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
    args = parser.parse_args()
    server = ThreadingHTTPServer(('127.0.0.1', args.port), handler_for(args.bundle, args.binary.resolve()))
    print(f'DSM independente: http://127.0.0.1:{server.server_port}', flush=True)
    server.serve_forever()


if __name__ == '__main__':
    main()
