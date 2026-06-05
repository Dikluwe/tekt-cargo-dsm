//! Medição em Arena — prompt/laudo 0033.
//!
//! Recomputar os ciclos de módulo do `egui` contando **só** as arestas
//! `uses_kind == "reference"`, contra o SCC de **85 módulos** do laudo 0031
//! (todas as arestas `uses` — sanidade). O fork passou a emitir o
//! `uses_kind` (commit `b44aa96` do clone local); a saída `--estrutura` da
//! lente **não** lê o campo, então esta medição consome o `export-json`
//! **direto do fork**, salvo em `dados/export-egui.json`.
//!
//! Método (igual em forma ao laudo 0032):
//! 1. Parsear o JSON do fork.
//! 2. Montar um `Grafo` de itens (nós + arestas Uses/Owns), aplicando o
//!    filtro de `uses_kind` na hora de incluir cada aresta `uses`.
//! 3. Rodar `lente_estrutura::agregar_por_modulo` + `detectar_ciclos`.
//! 4. Comparar **todas as uses** (sanidade — bate com 85 do laudo 0031) vs
//!    **só reference** (a resposta).
//! 5. Controle no `lente_core` (0 ciclos nas duas versões).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::Deserialize;

use lente_core::entities::grafo::{
    Aresta, Grafo, Kind, Modificadores, No, Path as PathGrafo, Relation, Visibility,
};
use lente_estrutura::{Ciclo, agregar_por_modulo, detectar_ciclos};

#[derive(Debug, Deserialize)]
struct ForkJSON {
    #[serde(rename = "crate")]
    crate_name: String,
    nodes: Vec<NodeJSON>,
    edges: Vec<EdgeJSON>,
}

#[derive(Debug, Deserialize)]
struct NodeJSON {
    id: usize,
    path: String,
    #[allow(dead_code)]
    name: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct EdgeJSON {
    from: String,
    id_from: usize,
    to: String,
    id_to: usize,
    relation: String,
    #[serde(default)]
    uses_kind: Option<String>,
}

/// Política de inclusão de aresta `uses`. `owns` é sempre incluída
/// (necessária para `agregar_por_modulo` achar o módulo contenedor).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FiltroUses {
    /// Inclui todas as `uses` independente de `uses_kind` (controle/sanidade).
    Todas,
    /// Inclui só `uses` com `uses_kind == "reference"`.
    SoReference,
}

fn kind_from(s: &str) -> Kind {
    // Despe modificadores ("const fn" → "fn"); fallback Mod para tipos
    // exóticos (não importa para agregação — só `Mod`/`Crate` viram nós
    // no agregado).
    Kind::try_from(s).unwrap_or(Kind::Mod)
}

fn montar_grafo(j: &ForkJSON, filtro: FiltroUses) -> Grafo {
    let nodes: Vec<No> = j
        .nodes
        .iter()
        .map(|n| No {
            id: n.id,
            path: PathGrafo::from(n.path.as_str()),
            name: n.path.rsplit("::").next().unwrap_or(&n.path).to_string(),
            kind: kind_from(&n.kind),
            modificadores: Modificadores::default(),
            visibility: Visibility::Priv,
            crate_name: j.crate_name.clone(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
        })
        .collect();

    // Set de ids válidos para descartar arestas que apontam para nós que
    // por algum motivo não estão em nodes (defesa em profundidade — não
    // observado em prática).
    let ids: HashSet<usize> = nodes.iter().map(|n| n.id).collect();

    let mut edges: Vec<Aresta> = Vec::with_capacity(j.edges.len());
    for e in &j.edges {
        if !ids.contains(&e.id_from) || !ids.contains(&e.id_to) {
            continue;
        }
        let relation = match e.relation.as_str() {
            "owns" => Relation::Owns,
            "uses" => {
                let incluir = match filtro {
                    FiltroUses::Todas => true,
                    FiltroUses::SoReference => {
                        e.uses_kind.as_deref() == Some("reference")
                    }
                };
                if !incluir {
                    continue;
                }
                Relation::Uses
            }
            _ => continue,
        };
        edges.push(Aresta {
            from: PathGrafo::from(e.from.as_str()),
            id_from: e.id_from,
            to: PathGrafo::from(e.to.as_str()),
            id_to: e.id_to,
            relation,
        });
    }

    Grafo {
        crate_name: j.crate_name.clone(),
        nodes,
        edges,
    }
}

fn maior_scc(ciclos: &[Ciclo]) -> usize {
    ciclos.iter().map(|c| c.modulos.len()).max().unwrap_or(0)
}

fn modulos_em_ciclos(ciclos: &[Ciclo]) -> HashSet<String> {
    let mut s = HashSet::new();
    for c in ciclos {
        for p in &c.modulos {
            s.insert(p.as_str().to_string());
        }
    }
    s
}

fn medir_caso(rotulo: &str, caminho: PathBuf, esperado_sanidade: usize) {
    println!("\n===== {} =====", rotulo);
    let conteudo = std::fs::read_to_string(&caminho).expect("ler json");
    let j: ForkJSON = serde_json::from_str(&conteudo).expect("parse json");
    println!("Fonte: {}", caminho.display());
    println!("Crate-raiz: {}", j.crate_name);
    println!("Nodes: {}    Edges: {}", j.nodes.len(), j.edges.len());

    // Distribuição de uses_kind nas arestas uses.
    let mut por_kind: HashMap<String, usize> = HashMap::new();
    let mut sem_kind: usize = 0;
    let mut total_uses: usize = 0;
    for e in &j.edges {
        if e.relation == "uses" {
            total_uses += 1;
            match &e.uses_kind {
                Some(k) => *por_kind.entry(k.clone()).or_default() += 1,
                None => sem_kind += 1,
            }
        }
    }
    println!("Arestas `uses`: {} total", total_uses);
    let mut chaves: Vec<&String> = por_kind.keys().collect();
    chaves.sort();
    for k in chaves {
        println!("  uses_kind={:<10} {}", k, por_kind[k]);
    }
    if sem_kind > 0 {
        println!("  sem uses_kind:        {}", sem_kind);
    }

    // ---- Sanidade ---------------------------------------------------------
    let g_todas = montar_grafo(&j, FiltroUses::Todas);
    println!(
        "[Todas uses]     itens: {} / arestas: {}",
        g_todas.nodes.len(),
        g_todas.edges.len()
    );
    let agg_todas = agregar_por_modulo(&g_todas);
    println!(
        "[Todas uses]     módulos: {}    deps módulo→módulo: {}",
        agg_todas
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, Kind::Mod | Kind::Crate))
            .count(),
        agg_todas
            .edges
            .iter()
            .filter(|a| a.relation == Relation::Uses)
            .count()
    );
    let ciclos_todas = detectar_ciclos(&agg_todas);
    let maior_todas = maior_scc(&ciclos_todas);
    println!(
        "[Todas uses]     SCCs ≥2: {}    maior SCC: {} módulos",
        ciclos_todas.len(),
        maior_todas
    );

    if esperado_sanidade != 0 {
        assert_eq!(
            maior_todas, esperado_sanidade,
            "PORTÃO DE SANIDADE FALHOU: esperava {} (laudo 0031), veio {}",
            esperado_sanidade, maior_todas
        );
        println!("  ✓ portão de sanidade OK (bate com laudo 0031)");
    } else {
        assert!(
            ciclos_todas.is_empty(),
            "controle: esperava 0 ciclos, veio {}",
            ciclos_todas.len()
        );
        println!("  ✓ controle OK (0 ciclos)");
    }

    // ---- Só reference -----------------------------------------------------
    let g_ref = montar_grafo(&j, FiltroUses::SoReference);
    println!(
        "[Só reference]   itens: {} / arestas: {}",
        g_ref.nodes.len(),
        g_ref.edges.len()
    );
    let agg_ref = agregar_por_modulo(&g_ref);
    println!(
        "[Só reference]   módulos: {}    deps módulo→módulo: {}",
        agg_ref
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, Kind::Mod | Kind::Crate))
            .count(),
        agg_ref
            .edges
            .iter()
            .filter(|a| a.relation == Relation::Uses)
            .count()
    );
    let ciclos_ref = detectar_ciclos(&agg_ref);
    let maior_ref = maior_scc(&ciclos_ref);
    println!(
        "[Só reference]   SCCs ≥2: {}    maior SCC: {} módulos",
        ciclos_ref.len(),
        maior_ref
    );

    if esperado_sanidade == 0 {
        // Controle: nenhum ciclo nas duas versões.
        assert!(
            ciclos_ref.is_empty(),
            "controle reference: esperava 0 ciclos, veio {}",
            ciclos_ref.len()
        );
        println!("  ✓ controle reference OK (0 ciclos)");
    }

    // Delta
    let em_todas = modulos_em_ciclos(&ciclos_todas);
    let em_ref = modulos_em_ciclos(&ciclos_ref);
    let saidos: Vec<String> = {
        let mut v: Vec<String> = em_todas.difference(&em_ref).cloned().collect();
        v.sort();
        v
    };
    let permaneceram: Vec<String> = {
        let mut v: Vec<String> = em_ref.iter().cloned().collect();
        v.sort();
        v
    };
    println!(
        "\nDelta: módulos que SAÍRAM dos ciclos com `só reference`: {}",
        saidos.len()
    );
    for p in saidos.iter().take(20) {
        println!("  - {}", p);
    }
    if saidos.len() > 20 {
        println!("  … (+{} mais)", saidos.len() - 20);
    }

    if !ciclos_ref.is_empty() {
        println!(
            "\nMódulos que PERMANECERAM em algum ciclo (acoplamento de tipo \"real\"): {}",
            permaneceram.len()
        );
        for (i, c) in ciclos_ref.iter().enumerate() {
            println!("  SCC {} ({} módulos):", i + 1, c.modulos.len());
            for p in c.modulos.iter().take(10) {
                println!("    - {}", p.as_str());
            }
            if c.modulos.len() > 10 {
                println!("    … (+{} mais)", c.modulos.len() - 10);
            }
        }
    } else {
        println!(
            "\nNenhum SCC ≥ 2 remanescente — `só reference` é DAG ao nível módulo."
        );
    }
}

fn main() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dados");

    medir_caso(
        "egui — sanidade vs. só reference",
        base.join("export-egui.json"),
        85,
    );
    medir_caso(
        "Controle — lente_core (0 ciclos esperados)",
        base.join("export-lente-core.json"),
        0,
    );
}
