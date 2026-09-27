//! Crystalline Lineage
//! @prompt lab/adaptador-typescript/CONTRATO.md
//! @layer Lab
//! @updated 2026-09-26
//! Verificação real do JSON no adaptador e algoritmos existentes, sem Cargo/fork na extração JS.
use lente_core::entities::grafo::{Kind, Relation, UsesKind};

fn main() {
    let arquivo = std::env::args().nth(1).expect("informe o grafo da fixture de compatibilidade");
    let json = std::fs::read_to_string(arquivo).expect("ler JSON da fixture");
    let grafo = lente_infra::desserializar_grafo(&json).expect("JSON compatível com a entrada existente");
    assert_eq!(grafo.nodes.len(), 4);
    assert_eq!(grafo.edges.len(), 4);
    for no in &grafo.nodes {
        assert_eq!(no.kind, Kind::Mod);
        assert!(no.position.is_some());
    }
    for aresta in &grafo.edges {
        assert_eq!(aresta.relation, Relation::Uses);
        assert_eq!(aresta.uses_kind, Some(UsesKind::Import));
        assert_eq!(aresta.from, grafo.nodes.iter().find(|n| n.id == aresta.id_from).unwrap().path);
        assert_eq!(aresta.to, grafo.nodes.iter().find(|n| n.id == aresta.id_to).unwrap().path);
    }
    let no = |nome: &str| &grafo.nodes.iter().find(|n| n.name == nome).unwrap().path;
    let raio = lente_core::domain::raio::calcular_raio(&grafo, no("c.ts")).unwrap();
    assert_eq!(raio.montante[no("b.ts")], 1);
    assert_eq!(raio.montante[no("a.ts")], 2);
    assert_eq!(raio.montante[no("d.ts")], 3);
    assert!(!raio.montante.contains_key(no("c.ts")));
    let agregado = lente_estrutura::agregar_por_modulo(&grafo);
    let ciclos = lente_estrutura::detectar_ciclos(&agregado);
    assert_eq!(ciclos.len(), 1);
    println!("Compatibilidade OK: 4 nós mod, 4 arestas uses/import, impacto transitivo até profundidade 3 e 1 ciclo.");
}
