//! Crystalline Lineage
//! @prompt lab/impacto-snapshots/CONTRATO.md
//! @updated 2026-09-26
//! Dados sintéticos em memória; nenhuma leitura de Git, arquivos ou extração.
use lente_core::entities::grafo::{
    Aresta, Grafo, Kind, Modificadores, No, Path, Relation, UsesKind, Visibility,
};
use lente_impacto_snapshots::{Completude, Correspondencia, Mudanca, Snapshot, comparar_impacto};

fn snapshot(versao: &str, nos: &[(usize, &str)], arestas: &[(usize, usize)]) -> Snapshot {
    let nodes: Vec<_> = nos
        .iter()
        .map(|&(id, path)| No {
            id,
            path: Path::new(path),
            name: path.into(),
            kind: Kind::Fn,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: "example".into(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: None,
        })
        .collect();
    let paths: std::collections::BTreeMap<_, _> =
        nodes.iter().map(|n| (n.id, n.path.clone())).collect();
    let edges = arestas
        .iter()
        .map(|&(de, para)| Aresta {
            from: paths[&de].clone(),
            id_from: de,
            to: paths[&para].clone(),
            id_to: para,
            relation: Relation::Uses,
            uses_kind: Some(UsesKind::Reference),
        })
        .collect();
    Snapshot {
        versao: versao.into(),
        grafo: Grafo {
            crate_name: "example".into(),
            nodes,
            edges,
        },
        completude: Completude::Completa,
        granularidade: "item".into(),
        natureza: "resolved-reference".into(),
        diagnosticos: vec![],
    }
}
fn main() {
    let antes = snapshot(
        "snapshot-antes",
        &[
            (10, "legacy/api"),
            (20, "service/old"),
            (30, "caller"),
            (40, "legacy-client"),
        ],
        &[(30, 20), (40, 10)],
    );
    // ID 10 agora tem outro significado: NÃO é pareado com o antigo ID 10.
    let depois = snapshot(
        "snapshot-depois",
        &[
            (10, "new/api"),
            (200, "service/new"),
            (300, "caller"),
            (400, "legacy-client"),
            (500, "new-client"),
        ],
        &[(300, 200), (500, 10)],
    );
    let pares = [
        Correspondencia {
            identidade: "service".into(),
            antes: 20,
            depois: 200,
        },
        Correspondencia {
            identidade: "caller".into(),
            antes: 30,
            depois: 300,
        },
        Correspondencia {
            identidade: "legacy-client".into(),
            antes: 40,
            depois: 400,
        },
    ];
    let mudancas = [
        Mudanca::Remocao { antes: 10 },
        Mudanca::Adicao { depois: 10 },
        Mudanca::Pareada {
            identidade: "service".into(),
            alteracao_declarada: false,
            renomeacao_declarada: true,
        },
    ];
    let resultado = comparar_impacto(&antes, &depois, &pares, &mudancas).expect("fixture válida");
    for m in &resultado.mudancas {
        println!("\nMudança declarada: {:?}", m.mudanca);
        for (lado, raio) in [("antes", &m.antes), ("depois", &m.depois)] {
            match raio {
                None => println!("  {lado}: não aplicável; nenhum raio calculado"),
                Some(raio) => {
                    println!(
                        "  {lado}: {} em {} (ID local {})",
                        raio.alvo.path, raio.alvo.origem.versao, raio.alvo.id_local
                    );
                    for c in &raio.consumidores {
                        println!(
                            "    consumidor {} em {}, profundidade {}, identidade {:?}",
                            c.elemento.path,
                            c.elemento.origem.versao,
                            c.profundidade,
                            c.elemento.identidade
                        );
                    }
                }
            }
        }
        println!(
            "  Consumidores observados somente antes / nos dois / somente depois: {} / {} / {}",
            m.consumidores.somente_antes.len(),
            m.consumidores.ambos.len(),
            m.consumidores.somente_depois.len()
        );
        for c in m
            .consumidores
            .somente_antes
            .iter()
            .chain(&m.consumidores.somente_depois)
        {
            println!(
                "    {}: outro lado {:?}",
                c.consumidor.elemento.path, c.estado_outro_lado
            );
        }
        if let Some(t) = &m.topologia_incidente {
            println!("  Relações incidentes: {:?}", t.status);
        }
    }
    println!(
        "\nSem par: {} antes, {} depois",
        resultado.sem_par_antes.len(),
        resultado.sem_par_depois.len()
    );
    for limite in &resultado.limites {
        println!("Limite: {limite}");
    }
}
