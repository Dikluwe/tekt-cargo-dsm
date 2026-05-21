/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/graph_builder.md
 * @prompt 00_nucleo/prompts/dependency_graph-revisao.md
 * @layer L4
 * @updated 2026-05-20
 */

use crystalline_dsm_core::entities::dependency_graph::{DependencyGraph, ExternalKind, GraphEdge};
use crystalline_dsm_core::entities::import_edge::{ImportEdge, ImportKind};
use crystalline_dsm_core::entities::module_tree::ModuleTree;
use crystalline_dsm_core::entities::workspace::Workspace;
use std::collections::HashMap;

pub fn build_graph(
    _workspace: &Workspace,
    trees: &HashMap<String, ModuleTree>,
    edges_per_crate: &HashMap<String, Vec<ImportEdge>>,
) -> DependencyGraph {
    let mut graph = DependencyGraph::new();

    // Fase 1: criar nós internos
    // Iteramos de forma determinística por nome do crate
    let mut sorted_crate_names: Vec<&String> = trees.keys().collect();
    sorted_crate_names.sort();

    for crate_name in sorted_crate_names {
        if let Some(tree) = trees.get(crate_name) {
            for (node_id, module_node) in tree.all_nodes() {
                graph.add_internal_node_with_tree(
                    module_node.canonical_path.clone(),
                    crate_name.clone(),
                    node_id,
                );
            }
        }
    }

    // Fase 3: processar arestas
    let mut sorted_edge_crate_names: Vec<&String> = edges_per_crate.keys().collect();
    sorted_edge_crate_names.sort();

    for crate_name in sorted_edge_crate_names {
        if let (Some(edges), Some(tree)) = (edges_per_crate.get(crate_name), trees.get(crate_name))
        {
            for edge in edges {
                if edge.kind == ImportKind::Unresolved {
                    continue;
                }

                // Determinar nó origem
                let from_node = tree.node(edge.from);
                let from_canonical = &from_node.canonical_path;
                let from_id = match graph.find_node(from_canonical) {
                    Some(id) => id,
                    None => continue, // se a origem não estiver no grafo, ignora
                };

                // Tratar target_module se vazio
                let target_module = if edge.target_module.is_empty() {
                    edge.imported_item.clone()
                } else {
                    edge.target_module.clone()
                };

                // Determinar nó destino
                let to_id = match edge.kind {
                    ImportKind::CurrentCrate | ImportKind::WorkspaceCrate => {
                        match graph.find_node(&target_module) {
                            Some(id) => id,
                            None => {
                                // Caso a referência interna não exista, vira externo
                                graph.add_external_node(target_module, ExternalKind::Crate)
                            }
                        }
                    }
                    ImportKind::External => {
                        graph.add_external_node(target_module, ExternalKind::Crate)
                    }
                    ImportKind::Stdlib => {
                        graph.add_external_node(target_module, ExternalKind::Stdlib)
                    }
                    ImportKind::Unresolved => continue,
                };

                // Construir GraphEdge
                let graph_edge = GraphEdge {
                    imported_item: edge.imported_item.clone(),
                    alias: edge.alias.clone(),
                    is_reexport: edge.is_reexport,
                    is_glob: edge.is_glob,
                    raw_use_path: edge.raw_use_path.clone(),
                };

                // Adicionar aresta
                let _ = graph.add_edge(from_id, to_id, graph_edge);
            }
        }
    }

    graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use crystalline_dsm_core::entities::module_tree::NodeId;
    use std::path::PathBuf;

    fn create_empty_workspace() -> Workspace {
        Workspace {
            root: PathBuf::from("/workspace"),
            members: vec![],
        }
    }

    fn create_dummy_import_edge(
        from: NodeId,
        target_module: String,
        imported_item: String,
        kind: ImportKind,
    ) -> ImportEdge {
        ImportEdge::new(
            from,
            "my_crate".to_string(),
            target_module,
            imported_item,
            kind,
            "use dummy;".to_string(),
            false,
            None,
            false,
        )
    }

    // 1. Workspace vazio
    #[test]
    fn test_build_graph_empty() {
        let ws = create_empty_workspace();
        let trees = HashMap::new();
        let edges = HashMap::new();

        let graph = build_graph(&ws, &trees, &edges);
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    // 2. Um crate sem imports
    #[test]
    fn test_build_graph_no_imports() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        trees.insert("my_crate".to_string(), tree);

        let edges = HashMap::new();

        let graph = build_graph(&ws, &trees, &edges);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.internal_node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    // 3. Um crate, import interno (CurrentCrate)
    #[test]
    fn test_build_graph_internal_import() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root_id = tree.root();
        let _child_id = tree
            .add_child(
                root_id,
                "utils".to_string(),
                PathBuf::from("src/utils.rs"),
                false,
                false,
            )
            .unwrap();
        trees.insert("my_crate".to_string(), tree);

        let mut edges = HashMap::new();
        let edge = create_dummy_import_edge(
            root_id,
            "my_crate::utils".to_string(),
            "helper".to_string(),
            ImportKind::CurrentCrate,
        );
        edges.insert("my_crate".to_string(), vec![edge]);

        let graph = build_graph(&ws, &trees, &edges);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.internal_node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        let from_id = graph.find_node("my_crate").unwrap();
        let to_id = graph.find_node("my_crate::utils").unwrap();
        let mut out = graph.outgoing_edges(from_id);
        let (target, edge_data) = out.next().unwrap();
        assert_eq!(target, to_id);
        assert_eq!(edge_data.imported_item, "helper");
    }

    // 4. Um crate, import externo
    #[test]
    fn test_build_graph_external_import() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root_id = tree.root();
        trees.insert("my_crate".to_string(), tree);

        let mut edges = HashMap::new();
        let edge = create_dummy_import_edge(
            root_id,
            "serde::de".to_string(),
            "Deserialize".to_string(),
            ImportKind::External,
        );
        edges.insert("my_crate".to_string(), vec![edge]);

        let graph = build_graph(&ws, &trees, &edges);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.internal_node_count(), 1);
        assert_eq!(graph.external_node_count(), 1);

        let ext_id = graph.find_node("serde::de").unwrap();
        let node = graph.node(ext_id);
        assert!(matches!(
            node.kind,
            crystalline_dsm_core::entities::dependency_graph::NodeKind::External {
                kind: ExternalKind::Crate
            }
        ));
    }

    // 5. Um crate, import stdlib
    #[test]
    fn test_build_graph_stdlib_import() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root_id = tree.root();
        trees.insert("my_crate".to_string(), tree);

        let mut edges = HashMap::new();
        let edge = create_dummy_import_edge(
            root_id,
            "std::collections".to_string(),
            "HashMap".to_string(),
            ImportKind::Stdlib,
        );
        edges.insert("my_crate".to_string(), vec![edge]);

        let graph = build_graph(&ws, &trees, &edges);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.external_node_count(), 1);

        let ext_id = graph.find_node("std::collections").unwrap();
        let node = graph.node(ext_id);
        assert!(matches!(
            node.kind,
            crystalline_dsm_core::entities::dependency_graph::NodeKind::External {
                kind: ExternalKind::Stdlib
            }
        ));
    }

    // 6. Múltiplos imports do mesmo externo
    #[test]
    fn test_build_graph_multiple_same_external() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();
        let sub = tree
            .add_child(
                root,
                "sub".to_string(),
                PathBuf::from("src/sub.rs"),
                false,
                false,
            )
            .unwrap();
        trees.insert("my_crate".to_string(), tree);

        let mut edges = HashMap::new();
        let edge1 = create_dummy_import_edge(
            root,
            "serde".to_string(),
            "Serialize".to_string(),
            ImportKind::External,
        );
        let edge2 = create_dummy_import_edge(
            sub,
            "serde".to_string(),
            "Deserialize".to_string(),
            ImportKind::External,
        );
        edges.insert("my_crate".to_string(), vec![edge1, edge2]);

        let graph = build_graph(&ws, &trees, &edges);
        assert_eq!(graph.node_count(), 3); // my_crate, my_crate::sub, serde
        assert_eq!(graph.external_node_count(), 1);
        assert_eq!(graph.edge_count(), 2);
    }

    // 7. Workspace com 2 crates, import cross-crate
    #[test]
    fn test_build_graph_cross_crate() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let tree_a = ModuleTree::new("crate_a".to_string(), PathBuf::from("a/src/lib.rs"));
        let root_a = tree_a.root();
        let tree_b = ModuleTree::new("crate_b".to_string(), PathBuf::from("b/src/lib.rs"));
        trees.insert("crate_a".to_string(), tree_a);
        trees.insert("crate_b".to_string(), tree_b);

        let mut edges = HashMap::new();
        let edge = create_dummy_import_edge(
            root_a,
            "crate_b".to_string(),
            "Foo".to_string(),
            ImportKind::WorkspaceCrate,
        );
        edges.insert("crate_a".to_string(), vec![edge]);

        let graph = build_graph(&ws, &trees, &edges);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.internal_node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        let from = graph.find_node("crate_a").unwrap();
        let to = graph.find_node("crate_b").unwrap();
        let mut out = graph.outgoing_edges(from);
        assert_eq!(out.next().unwrap().0, to);
    }

    // 8. Import com Unresolved é ignorado
    #[test]
    fn test_build_graph_ignores_unresolved() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();
        trees.insert("my_crate".to_string(), tree);

        let mut edges = HashMap::new();
        let edge = create_dummy_import_edge(
            root,
            "unknown".to_string(),
            "Item".to_string(),
            ImportKind::Unresolved,
        );
        edges.insert("my_crate".to_string(), vec![edge]);

        let graph = build_graph(&ws, &trees, &edges);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    // 9. Determinismo
    #[test]
    fn test_build_graph_determinism() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        trees.insert(
            "crate_b".to_string(),
            ModuleTree::new("crate_b".to_string(), PathBuf::from("b/src/lib.rs")),
        );
        trees.insert(
            "crate_a".to_string(),
            ModuleTree::new("crate_a".to_string(), PathBuf::from("a/src/lib.rs")),
        );

        let edges = HashMap::new();

        let graph1 = build_graph(&ws, &trees, &edges);
        let graph2 = build_graph(&ws, &trees, &edges);

        assert_eq!(graph1.node_count(), graph2.node_count());
        assert_eq!(graph1.edge_count(), graph2.edge_count());
    }

    // 10. target_module inválido vira externo
    #[test]
    fn test_build_graph_invalid_target_internal_becomes_external() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();
        trees.insert("my_crate".to_string(), tree);

        let mut edges = HashMap::new();
        let edge = create_dummy_import_edge(
            root,
            "my_crate::non_existent".to_string(),
            "Foo".to_string(),
            ImportKind::CurrentCrate,
        );
        edges.insert("my_crate".to_string(), vec![edge]);

        let graph = build_graph(&ws, &trees, &edges);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.internal_node_count(), 1);
        assert_eq!(graph.external_node_count(), 1);

        let ext = graph.find_node("my_crate::non_existent").unwrap();
        let node = graph.node(ext);
        assert!(matches!(
            node.kind,
            crystalline_dsm_core::entities::dependency_graph::NodeKind::External {
                kind: ExternalKind::Crate
            }
        ));
    }

    // 11. Import glob
    #[test]
    fn test_build_graph_glob_import() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();
        trees.insert("my_crate".to_string(), tree);

        let mut edges = HashMap::new();
        let mut edge = create_dummy_import_edge(
            root,
            "serde".to_string(),
            "*".to_string(),
            ImportKind::External,
        );
        edge.is_glob = true;
        edges.insert("my_crate".to_string(), vec![edge]);

        let graph = build_graph(&ws, &trees, &edges);
        let from = graph.find_node("my_crate").unwrap();
        let mut out = graph.outgoing_edges(from);
        let (_, edge_data) = out.next().unwrap();
        assert!(edge_data.is_glob);
        assert_eq!(edge_data.imported_item, "*");
    }

    // 12. Import com alias
    #[test]
    fn test_build_graph_alias_import() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();
        trees.insert("my_crate".to_string(), tree);

        let mut edges = HashMap::new();
        let mut edge = create_dummy_import_edge(
            root,
            "serde".to_string(),
            "Serialize".to_string(),
            ImportKind::External,
        );
        edge.alias = Some("Ser".to_string());
        edges.insert("my_crate".to_string(), vec![edge]);

        let graph = build_graph(&ws, &trees, &edges);
        let from = graph.find_node("my_crate").unwrap();
        let mut out = graph.outgoing_edges(from);
        let (_, edge_data) = out.next().unwrap();
        assert_eq!(edge_data.alias, Some("Ser".to_string()));
    }

    // 13. Re-export
    #[test]
    fn test_build_graph_reexport_import() {
        let ws = create_empty_workspace();
        let mut trees = HashMap::new();
        let tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();
        trees.insert("my_crate".to_string(), tree);

        let mut edges = HashMap::new();
        let mut edge = create_dummy_import_edge(
            root,
            "serde".to_string(),
            "Serialize".to_string(),
            ImportKind::External,
        );
        edge.is_reexport = true;
        edges.insert("my_crate".to_string(), vec![edge]);

        let graph = build_graph(&ws, &trees, &edges);
        let from = graph.find_node("my_crate").unwrap();
        let mut out = graph.outgoing_edges(from);
        let (_, edge_data) = out.next().unwrap();
        assert!(edge_data.is_reexport);
    }
}
