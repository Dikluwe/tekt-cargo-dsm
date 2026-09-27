//! Crystalline Lineage
//! @prompt lab/adaptador-python/README.md
//! @layer Lab
fn main() {
    let arquivo = std::env::args().nth(1).expect("caminho do grafo JSON");
    let json = std::fs::read_to_string(arquivo).expect("ler grafo");
    let grafo = lente_infra::desserializar_grafo(&json).expect("contrato JSON vigente");
    assert!(!grafo.nodes.is_empty());
    for aresta in &grafo.edges {
        assert!(grafo.nodes.iter().any(|n| n.id == aresta.id_from && n.path == aresta.from));
        assert!(grafo.nodes.iter().any(|n| n.id == aresta.id_to && n.path == aresta.to));
    }
    println!("Contrato aceito: {} nós, {} arestas", grafo.nodes.len(), grafo.edges.len());
}
