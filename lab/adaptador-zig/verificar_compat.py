# Crystalline Lineage
# @prompt lab/adaptador-zig/CONTRATO.md
# @layer Lab
# @updated 2026-09-26
import os
import subprocess
import tempfile
from pathlib import Path
from extrair import BASE, extract, json_bytes


def main():
    (BASE/'.tmp').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=BASE/'.tmp', prefix='compat-') as tmp:
        directory = Path(tmp)
        for name, target in [('root.zig', 'b.zig'), ('b.zig', 'c.zig'), ('c.zig', 'root')]:
            (directory/name).write_text(f'const dependency = @import("{target}");\n')
        (directory/'modules.json').write_text('{}')
        graph, metadata = extract(directory/'root.zig', directory/'modules.json')
        assert metadata['completeness']['semantics'] == 'not-evaluated'
        (directory/'grafo.json').write_bytes(json_bytes(graph))
        subprocess.run(['cargo', 'run', '--offline', '--quiet', '--manifest-path', str(BASE/'compat/Cargo.toml'),
                        '--', str(directory/'grafo.json')], check=True,
                       env={**os.environ, 'CARGO_TARGET_DIR': str(BASE/'compat/target')})


if __name__ == '__main__':
    main()
