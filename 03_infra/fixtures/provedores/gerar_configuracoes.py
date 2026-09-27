#!/usr/bin/env python3
# Crystalline Lineage
# @prompt 00_nucleo/prompts/fixtures-provedores.md
# @prompt-hash 0d0b11af
# @layer L3
# @updated 2026-09-26
"""Prepara configs e compilation databases para testes; não instala/executa adaptadores.
Use: python3 03_infra/fixtures/provedores/gerar_configuracoes.py /tmp/saida-nova
"""
import json
import pathlib
import shutil
import sys

repo = pathlib.Path(__file__).resolve().parents[3]
out = pathlib.Path(sys.argv[1]).resolve()
out.mkdir(parents=True, exist_ok=False)
verificador = repo / 'lab/pacote-analise/target/debug/lente-pacote-analise'

def salvar(nome, value):
    path = out / nome
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + '\n')
    return path

def config(provider, executable, helper, params, **opts):
    return salvar(provider + '.json', dict(executavel=str(executable), helper=str(helper) if helper else None,
        verificador=str(verificador), timeout_ms=30000, parametros=params, **opts))

python = shutil.which('python3') or '/usr/bin/python3'
node = shutil.which('node') or '/usr/bin/node'
config('python', python, repo / 'lab/adaptador-python/adaptador.py', {'roots':[str(repo / 'lab/adaptador-python/tests/fixtures/projeto')]})
config('typescript', node, repo / 'lab/adaptador-typescript/extrair.cjs', {'project':str(repo / 'lab/adaptador-typescript/exemplo/tsconfig.json'), 'name':'exemplo'})
config('zig', python, repo / 'lab/adaptador-zig/extrair.py', {'root':str(repo / 'lab/adaptador-zig/fixtures/basico/root.zig'), 'modules':str(repo / 'lab/adaptador-zig/fixtures/basico/modules.json'), 'name':'exemplo'})
croot = repo / 'lab/adaptador-c/fixtures'
cdb = salvar('c-db/compile_commands.json', [dict(directory=str(croot), file=source,
    arguments=['clang','-std=c11','-Iinclude','-isystem','system',*flags,'-c',source])
    for source,flags in [('main.c',[]),('main.c',['-DFEATURE=1']),('missing.c',[])]])
config('c', repo / 'lab/adaptador-c/target/debug/lente-adaptador-c-experimental', None,
    {'database':str(cdb), 'root':str(croot), 'variante':0}, permitir_metadados_sem_vinculo=True, incluir_ambiente=True)
cpproot = repo / 'lab/adaptador-cpp/tests/fixtures'
cppdb = salvar('compile_commands_cpp.json', [dict(directory=str(cpproot), file=f'src/{name}.cpp',
    arguments=['clang++','-std=c++17','-Iinclude','-DFIXTURE_CONFIG=1','-c',f'src/{name}.cpp']) for name in ('one','two','definitions')])
config('cpp', python, repo / 'lab/adaptador-cpp/adaptador.py', {'database':str(cppdb), 'root':str(cpproot), 'libclang':'/usr/lib/llvm-18/lib/libclang.so'})
config('go', repo / 'lab/adaptador-go/target/debug/lente-adaptador-go', None,
    {'project':str(repo / 'lab/adaptador-go/fixtures/chain'), 'goos':'linux', 'goarch':'amd64','tests':'exclude','work':'off'},
    permitir_metadados_sem_vinculo=True, incluir_ambiente=True)
# Fixture de erro sintático: preserva módulo válido e motivo de análise parcial.
partial = out / 'python-parcial'
partial.mkdir()
(partial / 'bom.py').write_text('import ruim\n')
(partial / 'ruim.py').write_text('def sintaxe_quebrada(\n')
cfg = json.loads((out / 'python.json').read_text())
cfg['parametros']['roots'] = [str(partial)]
salvar('python-parcial.json', cfg)
print(out)
