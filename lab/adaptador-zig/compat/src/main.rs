//! Crystalline Lineage
//! @prompt lab/adaptador-zig/CONTRATO.md
//! @layer Lab
//! @updated 2026-09-26
use lente_core::entities::grafo::{Kind, Relation, UsesKind};

fn main() {
    let file = std::env::args()
        .nth(1)
        .expect("informe grafo da fixture de compatibilidade");
    let json = std::fs::read_to_string(file).unwrap();
    let graph =
        lente_infra::desserializar_grafo(&json).expect("JSON compatível com o contrato atual");
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 3);
    for node in &graph.nodes {
        assert_eq!(node.kind, Kind::Mod);
        assert!(node.position.is_some());
    }
    for edge in &graph.edges {
        assert_eq!(edge.relation, Relation::Uses);
        assert_eq!(edge.uses_kind, Some(UsesKind::Import));
        assert_eq!(
            edge.from,
            graph
                .nodes
                .iter()
                .find(|n| n.id == edge.id_from)
                .unwrap()
                .path
        );
        assert_eq!(
            edge.to,
            graph
                .nodes
                .iter()
                .find(|n| n.id == edge.id_to)
                .unwrap()
                .path
        );
    }
    let node = |name: &str| &graph.nodes.iter().find(|n| n.name == name).unwrap().path;
    let radius = lente_core::domain::raio::calcular_raio(&graph, node("c.zig")).unwrap();
    assert_eq!(radius.montante[node("b.zig")], 1);
    assert_eq!(radius.montante[node("root.zig")], 2);
    assert!(!radius.montante.contains_key(node("c.zig")));
    let modules = lente_estrutura::agregar_por_modulo(&graph);
    assert_eq!(lente_estrutura::detectar_ciclos(&modules).len(), 1);
    println!(
        "Compatibilidade OK: 3 nós mod, 3 arestas uses/import; impacto transitivo e 1 ciclo sintático."
    );
}
