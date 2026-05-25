/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/layer_config_detector.md
 * @layer L1
 * @updated 2026-05-25
 */

use crate::entities::dependency_graph::{DependencyGraph, GraphNodeId, NodeKind};
use crate::entities::layer_config::{Layer, LayerConfig};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerViolation {
    /// Nó de origem (o que importa).
    pub from_node: GraphNodeId,

    /// Nó de destino (o que é importado).
    pub to_node: GraphNodeId,

    /// Camada do nó de origem.
    pub from_layer: Layer,

    /// Camada do nó de destino.
    pub to_layer: Layer,
}

impl LayerViolation {
    /// Descrição textual da violação, para tooltips/relatórios.
    /// Ex: "L1 → L3 (forbidden)".
    pub fn describe(&self) -> String {
        format!(
            "{} → {} (forbidden)",
            self.from_layer.as_str(),
            self.to_layer.as_str()
        )
    }
}

/// Detecta violações de direção topológica entre as camadas no grafo
/// usando a tabela fornecida em `LayerConfig`.
pub fn detect_layer_violations(
    graph: &DependencyGraph,
    config: &LayerConfig,
) -> Vec<LayerViolation> {
    let mut violations = Vec::new();
    let mut seen = HashSet::new();

    for (from_id, to_id, _) in graph.all_edges() {
        let from_node = graph.node(from_id);
        let to_node = graph.node(to_id);

        let from_crate = match &from_node.kind {
            NodeKind::InternalWithTree { crate_name, .. } => Some(crate_name.as_str()),
            NodeKind::InternalWithoutTree { crate_name } => Some(crate_name.as_str()),
            NodeKind::External { .. } => None,
        };

        let to_crate = match &to_node.kind {
            NodeKind::InternalWithTree { crate_name, .. } => Some(crate_name.as_str()),
            NodeKind::InternalWithoutTree { crate_name } => Some(crate_name.as_str()),
            NodeKind::External { .. } => None,
        };

        // Se algum dos nós for externo, ignora.
        let (from_c, to_c) = match (from_crate, to_crate) {
            (Some(f), Some(t)) => (f, t),
            _ => continue,
        };

        // Obter as camadas. Se algum não for mapeado no config, ignora.
        let from_layer = match config.layer_of_crate(from_c) {
            Some(l) => l,
            None => continue,
        };
        let to_layer = match config.layer_of_crate(to_c) {
            Some(l) => l,
            None => continue,
        };

        // Se a dependência for proibida
        if !from_layer.can_depend_on(to_layer) {
            let pair = (from_id, to_id);
            if seen.insert(pair) {
                violations.push(LayerViolation {
                    from_node: from_id,
                    to_node: to_id,
                    from_layer,
                    to_layer,
                });
            }
        }
    }

    // Ordenar deterministicamente por (from_canonical_path, to_canonical_path) lexicográfico.
    violations.sort_by(|a, b| {
        let a_from_path = &graph.node(a.from_node).canonical_path;
        let a_to_path = &graph.node(a.to_node).canonical_path;
        let b_from_path = &graph.node(b.from_node).canonical_path;
        let b_to_path = &graph.node(b.to_node).canonical_path;

        (a_from_path, a_to_path).cmp(&(b_from_path, b_to_path))
    });

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::dependency_graph::{ExternalKind, GraphEdge};
    use crate::entities::module_tree::NodeId;
    use std::collections::HashMap;

    fn test_edge() -> GraphEdge {
        GraphEdge {
            imported_item: "Item".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "path".to_string(),
        }
    }

    #[test]
    fn test_layer_violation_describe() {
        let violation = LayerViolation {
            from_node: GraphNodeId::test_new(0),
            to_node: GraphNodeId::test_new(1),
            from_layer: Layer::L1,
            to_layer: Layer::L3,
        };
        assert_eq!(violation.describe(), "L1 → L3 (forbidden)");
    }

    #[test]
    fn test_detect_no_violations() {
        let mut graph = DependencyGraph::new();
        // L2 dependendo de L1
        let l1 =
            graph.add_internal_node_with_tree("core::x".into(), "core".into(), NodeId::test_new(0));
        let l2 = graph.add_internal_node_with_tree(
            "shell::y".into(),
            "shell".into(),
            NodeId::test_new(1),
        );
        graph.add_edge(l2, l1, test_edge()).unwrap();

        // L3 dependendo de L1
        let l3 = graph.add_internal_node_with_tree(
            "infra::z".into(),
            "infra".into(),
            NodeId::test_new(2),
        );
        graph.add_edge(l3, l1, test_edge()).unwrap();

        // L4 dependendo de L3
        let l4 = graph.add_internal_node_with_tree(
            "wiring::w".into(),
            "wiring".into(),
            NodeId::test_new(3),
        );
        graph.add_edge(l4, l3, test_edge()).unwrap();

        let mut config_map = HashMap::new();
        config_map.insert("core".to_string(), Layer::L1);
        config_map.insert("shell".to_string(), Layer::L2);
        config_map.insert("infra".to_string(), Layer::L3);
        config_map.insert("wiring".to_string(), Layer::L4);
        let config = LayerConfig::new(config_map);

        let violations = detect_layer_violations(&graph, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_detect_violation_l1_to_l3() {
        let mut graph = DependencyGraph::new();
        let l1 =
            graph.add_internal_node_with_tree("core::x".into(), "core".into(), NodeId::test_new(0));
        let l3 = graph.add_internal_node_with_tree(
            "infra::z".into(),
            "infra".into(),
            NodeId::test_new(1),
        );
        // L1 -> L3 (violação!)
        graph.add_edge(l1, l3, test_edge()).unwrap();

        let mut config_map = HashMap::new();
        config_map.insert("core".to_string(), Layer::L1);
        config_map.insert("infra".to_string(), Layer::L3);
        let config = LayerConfig::new(config_map);

        let violations = detect_layer_violations(&graph, &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].from_layer, Layer::L1);
        assert_eq!(violations[0].to_layer, Layer::L3);
    }

    #[test]
    fn test_detect_violation_l2_to_l3() {
        let mut graph = DependencyGraph::new();
        let l2 = graph.add_internal_node_with_tree(
            "shell::y".into(),
            "shell".into(),
            NodeId::test_new(0),
        );
        let l3 = graph.add_internal_node_with_tree(
            "infra::z".into(),
            "infra".into(),
            NodeId::test_new(1),
        );
        // L2 -> L3 (violação!)
        graph.add_edge(l2, l3, test_edge()).unwrap();

        let mut config_map = HashMap::new();
        config_map.insert("shell".to_string(), Layer::L2);
        config_map.insert("infra".to_string(), Layer::L3);
        let config = LayerConfig::new(config_map);

        let violations = detect_layer_violations(&graph, &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].from_layer, Layer::L2);
        assert_eq!(violations[0].to_layer, Layer::L3);
    }

    #[test]
    fn test_detect_violation_production_to_lab() {
        let mut graph = DependencyGraph::new();
        let l2 = graph.add_internal_node_with_tree(
            "shell::y".into(),
            "shell".into(),
            NodeId::test_new(0),
        );
        let lab =
            graph.add_internal_node_with_tree("lab::exp".into(), "lab".into(), NodeId::test_new(1));
        // L2 -> Lab (violação!)
        graph.add_edge(l2, lab, test_edge()).unwrap();

        let mut config_map = HashMap::new();
        config_map.insert("shell".to_string(), Layer::L2);
        config_map.insert("lab".to_string(), Layer::Lab);
        let config = LayerConfig::new(config_map);

        let violations = detect_layer_violations(&graph, &config);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].to_layer, Layer::Lab);
    }

    #[test]
    fn test_lab_to_production_allowed() {
        let mut graph = DependencyGraph::new();
        let lab =
            graph.add_internal_node_with_tree("lab::exp".into(), "lab".into(), NodeId::test_new(0));
        let l1 =
            graph.add_internal_node_with_tree("core::x".into(), "core".into(), NodeId::test_new(1));
        // Lab -> L1 (permitido!)
        graph.add_edge(lab, l1, test_edge()).unwrap();

        let mut config_map = HashMap::new();
        config_map.insert("core".to_string(), Layer::L1);
        config_map.insert("lab".to_string(), Layer::Lab);
        let config = LayerConfig::new(config_map);

        let violations = detect_layer_violations(&graph, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_externals_ignored() {
        let mut graph = DependencyGraph::new();
        let l1 =
            graph.add_internal_node_with_tree("core::x".into(), "core".into(), NodeId::test_new(0));
        let ext = graph.add_external_node("serde".into(), ExternalKind::Crate);
        // L1 -> serde (externo, sem violação)
        graph.add_edge(l1, ext, test_edge()).unwrap();

        let mut config_map = HashMap::new();
        config_map.insert("core".to_string(), Layer::L1);
        let config = LayerConfig::new(config_map);

        let violations = detect_layer_violations(&graph, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_unmapped_crates_ignored() {
        let mut graph = DependencyGraph::new();
        let l1 =
            graph.add_internal_node_with_tree("core::x".into(), "core".into(), NodeId::test_new(0));
        let unmapped = graph.add_internal_node_with_tree(
            "other::y".into(),
            "other".into(),
            NodeId::test_new(1),
        );
        // L1 -> other (não mapeado, ignorado)
        graph.add_edge(l1, unmapped, test_edge()).unwrap();

        let mut config_map = HashMap::new();
        config_map.insert("core".to_string(), Layer::L1);
        // "other" não está no config_map
        let config = LayerConfig::new(config_map);

        let violations = detect_layer_violations(&graph, &config);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_deduplication() {
        let mut graph = DependencyGraph::new();
        let l1 =
            graph.add_internal_node_with_tree("core::x".into(), "core".into(), NodeId::test_new(0));
        let l3 = graph.add_internal_node_with_tree(
            "infra::z".into(),
            "infra".into(),
            NodeId::test_new(1),
        );
        // Múltiplos imports de L1 para L3 (apenas 1 violação deve ser reportada)
        graph.add_edge(l1, l3, test_edge()).unwrap();
        graph.add_edge(l1, l3, test_edge()).unwrap();

        let mut config_map = HashMap::new();
        config_map.insert("core".to_string(), Layer::L1);
        config_map.insert("infra".to_string(), Layer::L3);
        let config = LayerConfig::new(config_map);

        let violations = detect_layer_violations(&graph, &config);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_deterministic_ordering() {
        let mut graph = DependencyGraph::new();
        let a =
            graph.add_internal_node_with_tree("core::a".into(), "core".into(), NodeId::test_new(0));
        let b =
            graph.add_internal_node_with_tree("core::b".into(), "core".into(), NodeId::test_new(1));
        let x = graph.add_internal_node_with_tree(
            "infra::x".into(),
            "infra".into(),
            NodeId::test_new(2),
        );
        let y = graph.add_internal_node_with_tree(
            "infra::y".into(),
            "infra".into(),
            NodeId::test_new(3),
        );

        // Adiciona as violações fora de ordem
        // b -> y
        graph.add_edge(b, y, test_edge()).unwrap();
        // a -> x
        graph.add_edge(a, x, test_edge()).unwrap();
        // b -> x
        graph.add_edge(b, x, test_edge()).unwrap();
        // a -> y
        graph.add_edge(a, y, test_edge()).unwrap();

        let mut config_map = HashMap::new();
        config_map.insert("core".to_string(), Layer::L1);
        config_map.insert("infra".to_string(), Layer::L3);
        let config = LayerConfig::new(config_map);

        let violations = detect_layer_violations(&graph, &config);
        assert_eq!(violations.len(), 4);

        // Ordem esperada lexicográfica por (from_path, to_path):
        // 1. core::a -> infra::x
        // 2. core::a -> infra::y
        // 3. core::b -> infra::x
        // 4. core::b -> infra::y
        assert_eq!(
            graph.node(violations[0].from_node).canonical_path,
            "core::a"
        );
        assert_eq!(graph.node(violations[0].to_node).canonical_path, "infra::x");

        assert_eq!(
            graph.node(violations[1].from_node).canonical_path,
            "core::a"
        );
        assert_eq!(graph.node(violations[1].to_node).canonical_path, "infra::y");

        assert_eq!(
            graph.node(violations[2].from_node).canonical_path,
            "core::b"
        );
        assert_eq!(graph.node(violations[2].to_node).canonical_path, "infra::x");

        assert_eq!(
            graph.node(violations[3].from_node).canonical_path,
            "core::b"
        );
        assert_eq!(graph.node(violations[3].to_node).canonical_path, "infra::y");
    }
}
