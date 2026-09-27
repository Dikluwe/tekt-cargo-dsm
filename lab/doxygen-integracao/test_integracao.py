#!/usr/bin/env python3
# Crystalline Lineage — @prompt lab/doxygen-integracao/CONTRATO.md
"""Harness de binários reais. Ausência de ferramenta é falha, nunca skip."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FIX = Path(__file__).parent / 'fixtures'
BINS = {n: ROOT / f'lab/doxygen-{n}/target/debug/' / ('doxygen-explorador' if n == 'explorador' else f'lente-doxygen-{n}') for n in ['importador', 'correspondencia', 'prioridade', 'explorador']}
CLI = ROOT / 'target/debug/lente'
MCP = ROOT / 'target/debug/lente-mcp'
DSM = ROOT / 'lab/dsm-explicavel/target/debug/dsm-explicavel'
COUNT = 0


def run(args, codes=(0,), **kw):
    global COUNT
    p = subprocess.run([str(x) for x in args], capture_output=True, timeout=60, **kw)
    assert p.returncode in codes, (args, p.returncode, p.stderr.decode(errors='replace'))
    COUNT += 1
    return p


def read(p):
    return json.loads(p.read_bytes())


def write(p, v):
    p.write_text(json.dumps(v, ensure_ascii=False, indent=2) + '\n')
    return p


def digest(p):
    return 'sha256:' + hashlib.sha256(p.read_bytes()).hexdigest()


def interfaces(config, directory, expected_error=None):
    path = write(directory / 'config.json', config)
    cli = run([CLI, '--analise-artefatos', path], (3,) if expected_error else (0,))
    req = {'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call', 'params': {'name': 'analisar_artefatos', 'arguments': config}}
    mcp = run([MCP], input=(json.dumps(req)+'\n').encode())
    result = json.loads(mcp.stdout)['result']
    if expected_error:
        assert result['isError'], result
        assert expected_error in result['content'][0]['text'], result
        assert expected_error in cli.stderr.decode(), cli.stderr
        return
    assert not result.get('isError'), result
    assert json.loads(cli.stdout) == json.loads(result['content'][0]['text'])
    return json.loads(cli.stdout)


def config(mode, **inputs):
    names = {'doxygen-importar':'importador','doxygen-associar':'correspondencia','doxygen-prioridade':'prioridade','doxygen-explorar':'explorador'}
    v = {'timeout_ms': 30000, 'consulta': {'modo': mode, **{k: str(v) if isinstance(v, Path) else v for k,v in inputs.items()}}}
    if mode in names:
        v['executavel'] = str(BINS[names[mode]])
    return v


def load_server():
    spec=importlib.util.spec_from_file_location('dsm_serve', ROOT/'lab/dsm-explicavel/serve.py')
    m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
    return m


def pipeline(d, xml=FIX/'xml', graph=FIX/'graph.json', real=False):
    inventory=d/'inventory.json';catalog=d/'catalog.json';matches=d/'matches.json';impact=d/'impact.json';report=d/'priority.json';panel=d/'panel.json'
    result=run([BINS['importador'],'--xml-dir',xml,'--output',inventory], (0,2))
    inv=read(inventory)
    assert inv['version']==2 and inv['coverage']!='complete'
    projection=interfaces(config('doxygen-grafo',grafo=graph,coverage='partial',alvos_metricas=[n['id'] for n in read(graph)['nodes'] if n['id'] != 60]),d)['analise']
    assert projection['graph_digest']==digest(graph)
    assert all(s['qualified_name'] is None and s['signature'] is None for s in projection['catalog']['symbols'])
    cat=projection['catalog']
    cat['symbols']=[s for s in read(FIX/'catalog-evidence.json') if s['node_id'] in {n['id'] for n in read(graph)['nodes']}]
    cat['diagnostics'].append({'code':'explicit_fixture_catalog','message':'Nomes/localizações fornecidos explicitamente pelo autor da fixture; não inferidos de paths opacos.','symbol_id':None})
    if real:
        # Independent declaration from real.hpp lines 4/6, not read from inventory.
        for symbol in cat['symbols']:
            symbol['qualified_name']=None;symbol['location']=None
        for symbol in cat['symbols']:
            if symbol['node_id'] in (10,20):
                symbol.update(qualified_name='demo::run',location={'file':'demo.hpp','line':{10:4,20:6}[symbol['node_id']]},signature=None)
    write(catalog,cat);write(impact,projection['impact'])
    run([BINS['correspondencia'],'--inventory',inventory,'--catalog',catalog,'--output',matches])
    run([BINS['prioridade'],'--inventory',inventory,'--catalog',catalog,'--matches',matches,'--impact',impact,'--output',report])
    run([BINS['explorador'],'--inventory',inventory,'--catalog',catalog,'--matches',matches,'--output',panel])
    m=read(matches);r=read(report);p=read(panel)
    assert m['inventory_digest']==digest(inventory) and m['catalog_digest']==digest(catalog)
    assert r['input_digests']=={k:digest(path) for k,path in [('inventory',inventory),('catalog',catalog),('matches',matches),('impact',impact)]}
    assert p['inventory_digest']==digest(inventory) and p['matches_digest']==digest(matches)
    assert r['coverage']['inventory']==p['coverage']==inv['coverage']
    assert r['pending_relations']==p['pending_relations']==inv['pending_relations']
    assert r['document_relations']==p['document_relations']==inv['relations']
    assert any(x['status']=='matched' for x in m['matches'])
    if not real:
        assert result.returncode==2 and inv['coverage']=='partial'
        states={x['node_id']:x for x in m['matches']}
        assert states[10]['document_id']=='int' and states[20]['document_id']=='double'
        assert states[30]['status']=='ambiguous' and states[30]['document_id'] is None
        assert states[40]['status']=='unmatched' and states[40]['candidates']==['weak']
        assert next(x for x in r['items'] if x['node_id']==50)['documentary_state']=='associated_without_text'
        assert next(x for x in r['items'] if x['node_id']==60)['transitive_consumers'] is None
        assert next(x for x in r['items'] if x['node_id']==70)['transitive_consumers']==0
        assert 'λ' in inventory.read_text()
    for mode,inputs,path in [
        ('doxygen-importar',{'xml_dir':xml},inventory),
        ('doxygen-associar',{'inventory':inventory,'catalog':catalog},matches),
        ('doxygen-prioridade',{'inventory':inventory,'catalog':catalog,'matches':matches,'impact':impact},report),
        ('doxygen-explorar',{'inventory':inventory,'catalog':catalog,'matches':matches},panel)]:
        envelope=interfaces(config(mode,**inputs),d)
        assert envelope['analise']==read(path)
        if mode=='doxygen-importar':assert envelope['execucao']['codigo_saida']==result.returncode
    # Each executable reruns on identical bytes, including importer.
    jobs=[('importador',['--xml-dir',xml],inventory,(0,2)),('correspondencia',['--inventory',inventory,'--catalog',catalog],matches,(0,)),('prioridade',['--inventory',inventory,'--catalog',catalog,'--matches',matches,'--impact',impact],report,(0,)),('explorador',['--inventory',inventory,'--catalog',catalog,'--matches',matches],panel,(0,))]
    for name,args,path,codes in jobs:
        second=d/(path.name+'.second')
        run([BINS[name],*args,'--output',second],codes)
        assert path.read_bytes()==second.read_bytes(),name
        run([BINS[name],*args,'--output',path],(1,2))
        assert path.read_bytes()==second.read_bytes()
    dsm_input=write(d/'dsm-input.json',{'format_version':1,'metadata':{'provider':'controlled fixture','execution':'partial','diagnostics':[]},'graph':read(graph)})
    prepared=run([DSM,'prepare',dsm_input]);bundle=d/'bundle.json';bundle.write_bytes(prepared.stdout)
    server=load_server()
    bound=server.documentation(read(bundle),DSM,graph,inventory,catalog,matches,BINS['explorador'])
    assert bound['graph_binding']=='verified_exact_graph_bytes'
    assert bound['pending_relations']==inv['pending_relations']
    bad=read(bundle);bad['cells'].append({'weight':99})
    try:server.documentation(bad,DSM,graph,inventory,catalog,matches,BINS['explorador'])
    except ValueError:pass
    else:raise AssertionError('DSM adulterada aceita')
    # Digest mismatch must be distinguishable in both interfaces.
    changed=d/'changed-inventory.json';changed.write_bytes(inventory.read_bytes()+b' ')
    interfaces(config('doxygen-prioridade',inventory=changed,catalog=catalog,matches=matches,impact=impact),d,'digest')
    invalid=d/'invalid-inventory.json';v=read(inventory);v['version']=99;write(invalid,v)
    interfaces(config('doxygen-associar',inventory=invalid,catalog=catalog),d,'schema')
    missing=config('doxygen-importar',xml_dir=xml);missing['executavel']=str(d/'absent')
    interfaces(missing,d,'provedor_indisponivel')
    # Broken XML must not publish a final inventory.
    broken=d/'broken';broken.mkdir();(broken/'index.xml').write_text('<broken>')
    run([BINS['importador'],'--xml-dir',broken,'--output',d/'no-result.json'],(1,))
    assert not (d/'no-result.json').exists()
    return {'inventory':inv['coverage'],'matches':{s:sum(x['status']==s for x in m['matches']) for s in ['matched','ambiguous','unmatched']},'pending':len(inv['pending_relations'])}


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output-dir',type=Path);parser.add_argument('--real-doxygen',action='store_true');args=parser.parse_args()
    for path in [*BINS.values(),CLI,MCP,DSM]:
        if not path.is_file():raise RuntimeError(f'Ferramenta ausente; compile explicitamente: {path}')
    def execute(d):
        synthetic=d/'synthetic';synthetic.mkdir();summary={'synthetic':pipeline(synthetic)}
        if args.real_doxygen:
            exe=shutil.which('doxygen')
            if not exe:raise RuntimeError('Doxygen ausente: teste real não executado')
            version=run([exe,'--version']).stdout.decode().strip()
            if version!='1.9.8':raise RuntimeError(f'Doxygen {version} incompatível; teste real não executado')
            real=d/'real';real.mkdir();shutil.copyfile(FIX/'real.hpp',real/'demo.hpp')
            doxyfile=real/'Doxyfile';doxyfile.write_text('PROJECT_NAME = demo\nOUTPUT_DIRECTORY = output\nINPUT = demo.hpp\nGENERATE_XML = YES\nGENERATE_HTML = NO\nGENERATE_LATEX = NO\nEXTRACT_ALL = YES\nQUIET = YES\nXML_NS_MEMB_FILE_SCOPE = NO\n')
            run([exe,doxyfile],cwd=real)
            # Structural fixture explicitly authored for real.hpp; no extractor is simulated.
            g=read(FIX/'graph.json');g['nodes']=[n for n in g['nodes'] if n['id'] in (1,10,20)]
            for n in g['nodes']:
                if n['id'] in (10,20):
                    line={10:4,20:6}[n['id']];n['position']={'file':'demo.hpp','start_line':line,'end_line':line}
            g['edges']=[e for e in g['edges'] if e['relation']=='owns' and e['id_to'] in (10,20)]
            real_graph=write(real/'graph.json',g)
            summary['real']={'version':version,'configuration_digest':digest(doxyfile),**pipeline(real,real/'output/xml',graph=real_graph,real=True)}
        summary['successful_process_checks']=COUNT;write(d/'results.json',summary);print(json.dumps(summary,ensure_ascii=False,indent=2))
    if args.output_dir:
        d=args.output_dir.resolve();d.mkdir(parents=True,exist_ok=False);execute(d)
    else:
        with tempfile.TemporaryDirectory(prefix='lente-doxygen-test-') as d:execute(Path(d))

if __name__=='__main__':main()
