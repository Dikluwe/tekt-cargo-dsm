# Crystalline Lineage
# @prompt lab/adaptador-zig/CONTRATO.md
# @layer Lab
# @updated 2026-09-26
"""Compilação explícita somente do helper local; nenhuma instalação."""
import pathlib
import subprocess
import tempfile

BASE = pathlib.Path(__file__).resolve().parent

def main():
    version = subprocess.check_output(['zig', 'version'], text=True).strip()
    if version != '0.15.2':
        raise SystemExit(f'Zig 0.15.2 requerido; encontrado {version}. Nenhuma instalação realizada.')
    (BASE / 'bin').mkdir(exist_ok=True)
    (BASE / '.tmp').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=BASE / '.tmp', prefix='compile-') as tmp:
        subprocess.run(['zig', 'build-exe', str(BASE / 'parser.zig'), '-O', 'ReleaseSafe',
                        '--cache-dir', str(pathlib.Path(tmp) / 'local'),
                        '--global-cache-dir', str(pathlib.Path(tmp) / 'global'),
                        '-femit-bin=' + str(BASE / 'bin/parser')], check=True)
    print('Helper Zig 0.15.2 preparado; nenhum build.zig executado.')

if __name__ == '__main__':
    main()
