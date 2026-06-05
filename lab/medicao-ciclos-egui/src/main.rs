//! Medição em Arena — prompt/laudo 0032.
//!
//! Pergunta única: quanto do SCC de 85 módulos do `egui` (laudo 0031) é
//! sustentado pelo módulo-raiz `egui`?
//!
//! Método:
//! 1. Ler o JSON de `--estrutura --json` capturado em `dados/`.
//! 2. Reconstruir um `Grafo` módulo→módulo (ids artificiais, paths reais).
//! 3. Rodar `lente_estrutura::detectar_ciclos` **como está** — **portão de
//!    sanidade**: deve reproduzir o 85 do laudo 0031. Se não bater, parar.
//! 4. Remover o nó do módulo-raiz (path = nome do crate, sem `::`) e as
//!    arestas que o tocam; rodar `detectar_ciclos` de novo.
//! 5. Reportar com-raiz vs sem-raiz, e quem saiu do SCC grande.
//!
//! Controle: o `lente_core` (0 ciclos no laudo 0031) — remover a raiz não
//! pode inventar ciclo.

use std::collections::HashSet;
use std::path::PathBuf;

use serde::Deserialize;

use lente_core::entities::grafo::{
    Aresta, Grafo, Kind, Modificadores, No, Path as PathGrafo, Relation, Visibility,
};
use lente_estrutura::{Ciclo, detectar_ciclos};

#[derive(Debug, Deserialize)]
struct EstruturaJSON {
    #[allow(dead_code)]
    escopo: String,
    modulos: Vec<String>,
    dependencias: Vec<DepJSON>,
    #[allow(dead_code)]
    ciclos: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct DepJSON {
    de: String,
    para: String,
}

/// Constrói um `Grafo` artificial módulo→módulo a partir do JSON
/// `--estrutura --json`. `id` é o índice do path em `modulos` — estável
/// dentro de uma instância de medição.
fn grafo_da_estrutura(j: &EstruturaJSON) -> Grafo {
    let nodes: Vec<No> = j
        .modulos
        .iter()
        .enumerate()
        .map(|(i, p)| No {
            id: i,
            path: PathGrafo::from(p.as_str()),
            name: p.rsplit("::").next().unwrap_or(p).to_string(),
            kind: Kind::Mod,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: p.split("::").next().unwrap_or("").to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
        })
        .collect();
    let id_de: std::collections::HashMap<String, usize> = j
        .modulos
        .iter()
        .enumerate()
        .map(|(i, p)| (p.clone(), i))
        .collect();
    let mut edges: Vec<Aresta> = Vec::with_capacity(j.dependencias.len());
    for d in &j.dependencias {
        let (Some(&fa), Some(&ta)) = (id_de.get(&d.de), id_de.get(&d.para)) else {
            continue;
        };
        edges.push(Aresta {
            from: PathGrafo::from(d.de.as_str()),
            id_from: fa,
            to: PathGrafo::from(d.para.as_str()),
            id_to: ta,
            relation: Relation::Uses,
        });
    }
    Grafo {
        crate_name: j
            .modulos
            .first()
            .map(|s| s.split("::").next().unwrap_or("").to_string())
            .unwrap_or_default(),
        nodes,
        edges,
    }
}

fn remover_raiz(grafo: &Grafo, raiz_path: &str) -> Grafo {
    let id_raiz: Option<usize> = grafo
        .nodes
        .iter()
        .find(|n| n.path.as_str() == raiz_path)
        .map(|n| n.id);
    let Some(id_raiz) = id_raiz else {
        return grafo.clone();
    };
    let nodes: Vec<No> = grafo
        .nodes
        .iter()
        .filter(|n| n.id != id_raiz)
        .cloned()
        .collect();
    let edges: Vec<Aresta> = grafo
        .edges
        .iter()
        .filter(|a| a.id_from != id_raiz && a.id_to != id_raiz)
        .cloned()
        .collect();
    Grafo {
        crate_name: grafo.crate_name.clone(),
        nodes,
        edges,
    }
}

fn maior_scc(ciclos: &[Ciclo]) -> usize {
    ciclos.iter().map(|c| c.modulos.len()).max().unwrap_or(0)
}

fn modulos_saidos(antes: &[Ciclo], depois: &[Ciclo]) -> Vec<String> {
    // Une todos os módulos dos ciclos antes; subtrai os dos ciclos depois.
    let mut em_ciclo_antes: HashSet<String> = HashSet::new();
    for c in antes {
        for p in &c.modulos {
            em_ciclo_antes.insert(p.as_str().to_string());
        }
    }
    let mut em_ciclo_depois: HashSet<String> = HashSet::new();
    for c in depois {
        for p in &c.modulos {
            em_ciclo_depois.insert(p.as_str().to_string());
        }
    }
    let mut saidos: Vec<String> = em_ciclo_antes
        .difference(&em_ciclo_depois)
        .cloned()
        .collect();
    saidos.sort();
    saidos
}

fn medir(rotulo: &str, caminho: PathBuf, raiz_esperada: &str, esperado_scc_antes: usize) {
    println!("\n===== {} =====", rotulo);
    let conteudo = std::fs::read_to_string(&caminho).expect("ler json");
    let j: EstruturaJSON = serde_json::from_str(&conteudo).expect("parse json");
    println!("Fonte: {}", caminho.display());
    println!("Módulos no JSON: {}", j.modulos.len());
    println!("Dependências: {}", j.dependencias.len());
    println!("Ciclos no JSON original: {}", j.ciclos.len());

    let g = grafo_da_estrutura(&j);
    println!(
        "Grafo reconstruído: {} nós / {} arestas",
        g.nodes.len(),
        g.edges.len()
    );

    let ciclos_antes = detectar_ciclos(&g);
    let maior_antes = maior_scc(&ciclos_antes);
    println!(
        "[com a raiz]    nº SCCs ≥2: {}    maior SCC: {} módulos",
        ciclos_antes.len(),
        maior_antes
    );

    // Portão de sanidade: tem que bater com o laudo 0031.
    if esperado_scc_antes != 0 {
        assert_eq!(
            maior_antes, esperado_scc_antes,
            "PORTÃO DE SANIDADE FALHOU: o run como-está deve reproduzir {} (laudo 0031); veio {}",
            esperado_scc_antes, maior_antes
        );
        println!("  ✓ portão de sanidade OK (bate com laudo 0031)");
    } else {
        assert!(
            ciclos_antes.is_empty(),
            "PORTÃO DE SANIDADE FALHOU: esperava 0 ciclos, veio {}",
            ciclos_antes.len()
        );
        println!("  ✓ portão de sanidade OK (0 ciclos, como esperado)");
    }

    let g_sem_raiz = remover_raiz(&g, raiz_esperada);
    println!(
        "Grafo sem raiz '{}': {} nós / {} arestas",
        raiz_esperada,
        g_sem_raiz.nodes.len(),
        g_sem_raiz.edges.len()
    );
    let ciclos_depois = detectar_ciclos(&g_sem_raiz);
    let maior_depois = maior_scc(&ciclos_depois);
    println!(
        "[sem a raiz]    nº SCCs ≥2: {}    maior SCC: {} módulos",
        ciclos_depois.len(),
        maior_depois
    );

    let saidos = modulos_saidos(&ciclos_antes, &ciclos_depois);
    println!("Módulos que saíram de SCC ≥2: {}", saidos.len());
    for p in saidos.iter().take(20) {
        println!("  - {}", p);
    }
    if saidos.len() > 20 {
        println!("  … (+{} mais)", saidos.len() - 20);
    }

    if !ciclos_depois.is_empty() {
        println!("SCCs ≥2 que sobraram após remover a raiz:");
        for (i, c) in ciclos_depois.iter().enumerate() {
            println!("  SCC {} ({} módulos):", i + 1, c.modulos.len());
            for p in c.modulos.iter().take(8) {
                println!("    - {}", p.as_str());
            }
            if c.modulos.len() > 8 {
                println!("    … (+{} mais)", c.modulos.len() - 8);
            }
        }
    }
}

fn main() {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dados");

    medir(
        "egui Completo (escopo padrão)",
        base.join("estrutura-egui-completo.json"),
        "egui",
        85,
    );
    medir(
        "egui SeuCodigo (--filtrar-stdlib)",
        base.join("estrutura-egui-seu-codigo.json"),
        "egui",
        85,
    );
    medir(
        "Controle — lente_core (0 ciclos esperados)",
        base.join("estrutura-lente-core.json"),
        "lente_core",
        0,
    );
}
