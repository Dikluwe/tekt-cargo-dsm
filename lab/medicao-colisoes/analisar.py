#!/usr/bin/env python3
"""
Analisa colisões de path nos JSONs produzidos por `cargo modules export-json`.
Aplica mentalmente a cascata do lente_investiga (vizinhança → fontes).

Uso: python3 analisar.py <pasta_json> <pasta_src_typst>

Saída: relatório por crate no stdout, formato Markdown.
"""

import json
import os
import re
import sys
from collections import defaultdict
from itertools import combinations
from pathlib import Path


def detectar_colisoes(grafo):
    """Retorna dict {path: [nos]} para paths com 2+ nós."""
    por_path = defaultdict(list)
    for n in grafo["nodes"]:
        por_path[n["path"]].append(n)
    return {p: ns for p, ns in por_path.items() if len(ns) > 1}


def vizinhanca(grafo, path):
    """Arestas entrando e saindo do path (como tuplas comparáveis)."""
    entrando = []
    saindo = []
    for a in grafo["edges"]:
        chave = (a["from"], a["to"], a["relation"])
        if a["to"] == path:
            entrando.append(chave)
        if a["from"] == path:
            saindo.append(chave)
    return set(entrando + saindo)


def classificar_vizinhanca(v_a, v_b):
    """
    Critério do lente_investiga (D5 do laudo 0004):
    - disjunta (zero compartilhadas, ambos com exclusivas) → Distintos
    - idêntica (zero exclusivas dos dois lados, ≥1 compartilhada) → MesmoItem
    - resto → Inconclusivo
    """
    compart = v_a & v_b
    exc_a = v_a - v_b
    exc_b = v_b - v_a
    if not v_a and not v_b:
        return ("Inconclusivo", "ambos vazios", len(exc_a), len(exc_b), len(compart))
    if not compart and exc_a and exc_b:
        return ("Distintos/VizinhancaDisjunta", "", len(exc_a), len(exc_b), 0)
    if not exc_a and not exc_b and compart:
        return ("MesmoItem", "", 0, 0, len(compart))
    return ("Inconclusivo", "sobreposição parcial", len(exc_a), len(exc_b), len(compart))


def tipo_metodo_do_path(path):
    """`A::B::ErroRaio::fmt` → ('ErroRaio', 'fmt'). None se < 2 segs."""
    segs = path.split("::")
    if len(segs) < 2:
        return None
    return (segs[-2], segs[-1])


def encontrar_arquivos_rs(diretorio_crate):
    """Lista recursivamente todos os .rs em <crate>/src/."""
    src = Path(diretorio_crate) / "src"
    if not src.exists():
        return []
    return list(src.rglob("*.rs"))


def extrair_impls_de_trait_para_tipo(conteudo, tipo_alvo):
    """
    Parser textual minimalista — mesma lógica do lente_investiga::fontes:
    procura `impl <Trait> for <tipo_alvo>` (com possíveis genéricos `<...>`
    antes do trait), retorna o nome simples do trait (último segmento `::`)
    e os métodos declarados dentro do bloco (depth==1).
    """
    linhas = conteudo.split("\n")
    achados = []  # [(trait_simples, [metodos])]
    i = 0
    while i < len(linhas):
        linha = linhas[i].strip()
        if linha.startswith("//") or not linha.startswith("impl"):
            i += 1
            continue
        # após 'impl', pular genéricos `<...>` se houver
        resto = linha[4:].lstrip()
        if resto.startswith("<"):
            d = 0
            j = 0
            while j < len(resto):
                c = resto[j]
                if c == "<":
                    d += 1
                elif c == ">":
                    d -= 1
                    if d == 0:
                        resto = resto[j + 1 :].lstrip()
                        break
                j += 1
            else:
                i += 1
                continue
        # precisa ter ' for '
        if " for " not in resto:
            i += 1
            continue
        trait_completo, depois_for = resto.split(" for ", 1)
        trait_completo = trait_completo.strip()
        if not trait_completo:
            i += 1
            continue
        # nome do tipo: prefixo de identificador em depois_for
        nome_tipo = re.match(r"([A-Za-z_][A-Za-z0-9_]*)", depois_for)
        if not nome_tipo or nome_tipo.group(1) != tipo_alvo:
            i += 1
            continue
        trait_simples = trait_completo.split("::")[-1]
        # contar chaves até fechar; coletar `fn <nome>` em depth==1
        metodos = []
        depth = 0
        viu_primeira = False
        for k in range(i, len(linhas)):
            l = linhas[k].strip()
            if depth == 1 and not l.startswith("//"):
                # remover qualificadores
                lc = l
                for p in ("pub(crate) ", "pub(super) ", "pub ", "async ", "unsafe ", "const ", "extern "):
                    while lc.startswith(p):
                        lc = lc[len(p) :].lstrip()
                m = re.match(r"fn\s+([A-Za-z_][A-Za-z0-9_]*)", lc)
                if m:
                    metodos.append(m.group(1))
            for c in linhas[k]:
                if c == "{":
                    depth += 1
                    viu_primeira = True
                elif c == "}":
                    depth -= 1
            if viu_primeira and depth <= 0:
                achados.append((trait_simples, metodos))
                i = k + 1
                break
        else:
            i += 1
    return achados


def investigar_via_fontes(diretorio_crate, tipo, metodo):
    """
    Procura nos .rs do crate por 2+ `impl <Trait> for <tipo>` declarando
    `fn <metodo>`. Retorna lista deduplicada de traits.
    """
    traits = []
    for rs in encontrar_arquivos_rs(diretorio_crate):
        try:
            conteudo = rs.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue
        for trait, metodos in extrair_impls_de_trait_para_tipo(conteudo, tipo):
            if metodo in metodos and trait not in traits:
                traits.append(trait)
    return traits


def analisar_crate(json_path, dir_workspace_typst, crates_dir_typst):
    """Retorna dict com estatísticas e detalhes do crate."""
    with open(json_path) as f:
        grafo = json.load(f)

    nome_crate = grafo["crate"]
    # Diretório do crate (typst-utils → crates/typst-utils/)
    dir_crate = Path(crates_dir_typst) / nome_crate.replace("_", "-")
    # fallback
    if not dir_crate.exists():
        dir_crate = None

    colisoes = detectar_colisoes(grafo)

    total_nodes = len(grafo["nodes"])
    total_edges = len(grafo["edges"])

    # Quais colisões são do próprio crate (não stdlib)?
    prefixo = nome_crate + "::"
    proprias = {p: ns for p, ns in colisoes.items() if p == nome_crate or p.startswith(prefixo)}

    resultados = []
    for path, nos in proprias.items():
        # Para cada par investigamos (combinação 2 a 2; mas registramos o primeiro veredito)
        n_nodes = len(nos)
        # Vizinhança é uma só (o path é uma só string; arestas referenciam-no por valor).
        # Para investigar, simulamos: separar arestas em "que se associam a um dos nos" é impossível
        # sem mais info do que JSON dá. O lente_investiga assume que a chamadora fornece
        # vizinhanças disjuntas (por origem/destino) já separadas — algo que cargo-modules
        # não fornece direto. Vamos olhar a vizinhança AGREGADA do path e ver o que dá pra dizer.
        viz_agregada = vizinhanca(grafo, path)

        # Aplicamos o critério: se o path-colidente tem só uma vizinhança (uma lista de arestas),
        # não conseguimos separar "qual aresta é de qual cópia". A Estratégia 1 do
        # lente_investiga não se aplica diretamente — só sabemos quantas arestas o path tem.
        # Registramos isso e vamos direto à Estratégia 2.
        tipo_m = tipo_metodo_do_path(path)
        if tipo_m is None or dir_crate is None:
            resultados.append({
                "path": path,
                "n_nos": n_nodes,
                "arestas_agregadas": len(viz_agregada),
                "veredito": "NaoDeterminado",
                "razao": "sem segmentos suficientes ou sem fontes locais",
                "traits": [],
            })
            continue

        tipo, metodo = tipo_m
        traits = investigar_via_fontes(str(dir_crate), tipo, metodo)
        if len(traits) >= 2:
            resultados.append({
                "path": path,
                "n_nos": n_nodes,
                "arestas_agregadas": len(viz_agregada),
                "veredito": "Distintos/ImplDeTraitsDiferentes",
                "razao": "",
                "traits": traits,
            })
        else:
            resultados.append({
                "path": path,
                "n_nos": n_nodes,
                "arestas_agregadas": len(viz_agregada),
                "veredito": "NaoDeterminado",
                "razao": f"E2 encontrou {len(traits)} trait(s): {traits}",
                "traits": traits,
            })

    # Agregados do crate
    return {
        "nome": nome_crate,
        "total_nodes": total_nodes,
        "total_edges": total_edges,
        "colisoes_totais": len(colisoes),
        "colisoes_proprias": len(proprias),
        "stdlib_colisoes": len(colisoes) - len(proprias),
        "detalhes": resultados,
    }


def main():
    if len(sys.argv) < 3:
        print("uso: analisar.py <pasta_json> <crates_dir_typst>", file=sys.stderr)
        sys.exit(1)
    pasta_json = sys.argv[1]
    crates_dir = sys.argv[2]
    jsons = sorted(Path(pasta_json).glob("*.json"))
    total_proprias = 0
    total_decidiu_e2 = 0
    total_naodeterminado = 0
    por_crate = []
    padroes_traits = defaultdict(int)
    for jp in jsons:
        r = analisar_crate(jp, crates_dir, crates_dir)
        por_crate.append(r)
        total_proprias += r["colisoes_proprias"]
        for d in r["detalhes"]:
            if d["veredito"] == "Distintos/ImplDeTraitsDiferentes":
                total_decidiu_e2 += 1
                # par ordenado
                pair = tuple(sorted(d["traits"][:2]))
                padroes_traits[pair] += 1
            elif d["veredito"] == "NaoDeterminado":
                total_naodeterminado += 1
    # Saída JSON estruturada para depois converter em Markdown
    out = {
        "por_crate": por_crate,
        "agregados": {
            "total_colisoes_proprias": total_proprias,
            "decididas_via_fontes": total_decidiu_e2,
            "nao_determinado": total_naodeterminado,
            "padroes_traits": {f"{a}+{b}": n for (a, b), n in padroes_traits.items()},
        },
    }
    print(json.dumps(out, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
