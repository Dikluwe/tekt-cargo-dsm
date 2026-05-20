use crystalline_dsm_infra::cargo_metadata_reader::read_workspace;
use crystalline_dsm_infra::import_extractor::extract_imports;
use crystalline_dsm_infra::module_traverser::traverse_crate;
use std::collections::HashMap;
use std::path::PathBuf;

#[path = "../src/graph_builder.rs"]
mod graph_builder;

use graph_builder::build_graph;

fn get_fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap();
    workspace_root.join("tests").join("fixtures").join(name)
}

#[test]
fn test_integration_pipeline_simple() {
    let root = get_fixture_path("imports-simple");
    let ws = read_workspace(&root).unwrap();

    let mut trees = HashMap::new();
    let mut edges_per_crate = HashMap::new();

    for member in &ws.members {
        let tree = traverse_crate(member).unwrap();
        let edges = extract_imports(member, &tree, &[]).unwrap();
        trees.insert(member.name.clone(), tree);
        edges_per_crate.insert(member.name.clone(), edges);
    }

    let graph = build_graph(&ws, &trees, &edges_per_crate);

    // Verificar: 1 nó interno (imports_simple), 1 nó externo (a::b), 1 aresta
    assert_eq!(graph.internal_node_count(), 1);
    assert_eq!(graph.external_node_count(), 1);
    assert_eq!(graph.edge_count(), 1);

    let from_id = graph.find_node("imports-simple").unwrap();

    let to_id = graph.find_node("a::b").unwrap();
    let mut out = graph.outgoing_edges(from_id);
    let (target, edge) = out.next().unwrap();
    assert_eq!(target, to_id);
    assert_eq!(edge.imported_item, "Foo");
}

#[test]
fn test_integration_pipeline_workspace() {
    let root = get_fixture_path("imports-workspace");
    let ws = read_workspace(&root).unwrap();

    let mut trees = HashMap::new();
    let mut edges_per_crate = HashMap::new();

    let ws_names: Vec<String> = ws.members.iter().map(|m| m.name.clone()).collect();

    for member in &ws.members {
        let tree = traverse_crate(member).unwrap();
        let edges = extract_imports(member, &tree, &ws_names).unwrap();
        trees.insert(member.name.clone(), tree);
        edges_per_crate.insert(member.name.clone(), edges);
    }

    let graph = build_graph(&ws, &trees, &edges_per_crate);

    // Verificar: 2+ nós internos, 1 aresta cross-crate WorkspaceCrate
    assert!(graph.internal_node_count() >= 2);
    assert_eq!(graph.edge_count(), 1);

    let from_id = graph.find_node("crate_a").unwrap();
    let to_id = graph.find_node("crate_b::foo").unwrap();
    let mut out = graph.outgoing_edges(from_id);
    let (target, edge) = out.next().unwrap();
    assert_eq!(target, to_id);
    assert_eq!(edge.imported_item, "Bar");
}
