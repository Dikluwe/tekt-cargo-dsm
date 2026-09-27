//! Crystalline Lineage
//! @prompt lab/adaptador-cpp/README.md
//! @layer Lab
fn main() {
    let file = std::env::args().nth(1).expect("arquivo de grafo JSON");
    let json = std::fs::read_to_string(file).expect("ler grafo");
    let graph = lente_infra::desserializar_grafo(&json).expect("contrato vigente da Lente");
    assert!(!graph.nodes.is_empty());
    for edge in &graph.edges {
        assert!(graph.nodes.iter().any(|n| n.id == edge.id_from && n.path == edge.from));
        assert!(graph.nodes.iter().any(|n| n.id == edge.id_to && n.path == edge.to));
    }
    println!("Contrato aceito: {} nós, {} arestas", graph.nodes.len(), graph.edges.len());
}
