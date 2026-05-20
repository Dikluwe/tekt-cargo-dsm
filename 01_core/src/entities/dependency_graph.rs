/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/dependency_graph.md
 * @layer L1
 * @updated 2026-05-20
 */

use crate::entities::module_tree::NodeId;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("GraphNodeId inválido para este grafo")]
    InvalidNodeId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    pub canonical_path: String,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    Internal {
        crate_name: String,
        tree_node_id: NodeId,
    },
    External {
        kind: ExternalKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalKind {
    Stdlib,
    Crate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    pub imported_item: String,
    pub alias: Option<String>,
    pub is_reexport: bool,
    pub is_glob: bool,
    pub raw_use_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphNodeId(pub(crate) petgraph::graph::NodeIndex);

impl GraphNodeId {
    /// Construtor de teste auxiliar para simular IDs inválidos
    #[cfg(test)]
    pub(crate) fn test_new(index: usize) -> Self {
        GraphNodeId(petgraph::graph::NodeIndex::new(index))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphEdgeId(pub(crate) petgraph::graph::EdgeIndex);

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    graph: petgraph::Graph<GraphNode, GraphEdge, petgraph::Directed>,
    path_index: HashMap<String, GraphNodeId>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: petgraph::Graph::new(),
            path_index: HashMap::new(),
        }
    }

    pub fn add_internal_node(
        &mut self,
        canonical_path: String,
        crate_name: String,
        tree_node_id: NodeId,
    ) -> GraphNodeId {
        if let Some(&id) = self.path_index.get(&canonical_path) {
            return id;
        }

        let node = GraphNode {
            canonical_path: canonical_path.clone(),
            kind: NodeKind::Internal {
                crate_name,
                tree_node_id,
            },
        };
        let index = self.graph.add_node(node);
        let node_id = GraphNodeId(index);
        self.path_index.insert(canonical_path, node_id);
        node_id
    }

    pub fn add_external_node(
        &mut self,
        canonical_path: String,
        external_kind: ExternalKind,
    ) -> GraphNodeId {
        if let Some(&id) = self.path_index.get(&canonical_path) {
            return id;
        }

        let node = GraphNode {
            canonical_path: canonical_path.clone(),
            kind: NodeKind::External {
                kind: external_kind,
            },
        };
        let index = self.graph.add_node(node);
        let node_id = GraphNodeId(index);
        self.path_index.insert(canonical_path, node_id);
        node_id
    }

    pub fn add_edge(
        &mut self,
        from: GraphNodeId,
        to: GraphNodeId,
        edge: GraphEdge,
    ) -> Result<GraphEdgeId, GraphError> {
        if self.graph.node_weight(from.0).is_none() || self.graph.node_weight(to.0).is_none() {
            return Err(GraphError::InvalidNodeId);
        }
        let index = self.graph.add_edge(from.0, to.0, edge);
        Ok(GraphEdgeId(index))
    }

    pub fn find_node(&self, canonical_path: &str) -> Option<GraphNodeId> {
        self.path_index.get(canonical_path).copied()
    }

    pub fn node(&self, id: GraphNodeId) -> &GraphNode {
        &self.graph[id.0]
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn internal_node_count(&self) -> usize {
        self.graph
            .node_weights()
            .filter(|n| matches!(n.kind, NodeKind::Internal { .. }))
            .count()
    }

    pub fn external_node_count(&self) -> usize {
        self.graph
            .node_weights()
            .filter(|n| matches!(n.kind, NodeKind::External { .. }))
            .count()
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = (GraphNodeId, &GraphNode)> {
        self.graph
            .node_indices()
            .map(move |i| (GraphNodeId(i), &self.graph[i]))
    }

    pub fn internal_nodes(&self) -> impl Iterator<Item = (GraphNodeId, &GraphNode)> {
        self.all_nodes()
            .filter(|(_, n)| matches!(n.kind, NodeKind::Internal { .. }))
    }

    pub fn external_nodes(&self) -> impl Iterator<Item = (GraphNodeId, &GraphNode)> {
        self.all_nodes()
            .filter(|(_, n)| matches!(n.kind, NodeKind::External { .. }))
    }

    pub fn edge(&self, id: GraphEdgeId) -> &GraphEdge {
        &self.graph[id.0]
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn all_edges(&self) -> impl Iterator<Item = (GraphNodeId, GraphNodeId, &GraphEdge)> {
        self.graph
            .edge_references()
            .map(move |e| (GraphNodeId(e.source()), GraphNodeId(e.target()), e.weight()))
    }

    pub fn outgoing_edges(
        &self,
        from: GraphNodeId,
    ) -> impl Iterator<Item = (GraphNodeId, &GraphEdge)> {
        use petgraph::visit::EdgeRef;
        self.graph
            .edges(from.0)
            .map(move |e| (GraphNodeId(e.target()), e.weight()))
    }

    pub fn incoming_edges(
        &self,
        to: GraphNodeId,
    ) -> impl Iterator<Item = (GraphNodeId, &GraphEdge)> {
        use petgraph::visit::EdgeRef;
        self.graph
            .edges_directed(to.0, petgraph::Incoming)
            .map(move |e| (GraphNodeId(e.source()), e.weight()))
    }

    pub fn out_degree(&self, id: GraphNodeId) -> usize {
        self.graph.edges(id.0).count()
    }

    pub fn in_degree(&self, id: GraphNodeId) -> usize {
        self.graph.edges_directed(id.0, petgraph::Incoming).count()
    }

    pub(crate) fn raw_graph(&self) -> &petgraph::Graph<GraphNode, GraphEdge, petgraph::Directed> {
        &self.graph
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Grafo vazio
    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    // 2. add_internal_node único
    #[test]
    fn test_add_internal_node_single() {
        let mut graph = DependencyGraph::new();
        let path = "my_crate::foo".to_string();
        let id = graph.add_internal_node(path.clone(), "my_crate".to_string(), NodeId::test_new(0));

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.internal_node_count(), 1);
        assert_eq!(graph.external_node_count(), 0);
        assert_eq!(graph.find_node(&path), Some(id));
    }

    // 3. Deduplicação interna
    #[test]
    fn test_deduplication_internal() {
        let mut graph = DependencyGraph::new();
        let path = "my_crate::foo".to_string();
        let id1 =
            graph.add_internal_node(path.clone(), "my_crate".to_string(), NodeId::test_new(0));
        let id2 = graph.add_internal_node(path, "my_crate".to_string(), NodeId::test_new(1));

        assert_eq!(id1, id2);
        assert_eq!(graph.node_count(), 1);
    }

    // 4. add_external_node
    #[test]
    fn test_add_external_node() {
        let mut graph = DependencyGraph::new();
        let path = "std::collections".to_string();
        let id = graph.add_external_node(path.clone(), ExternalKind::Stdlib);

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.internal_node_count(), 0);
        assert_eq!(graph.external_node_count(), 1);
        assert_eq!(graph.find_node(&path), Some(id));
    }

    // 5. Deduplicação externa
    #[test]
    fn test_deduplication_external() {
        let mut graph = DependencyGraph::new();
        let path = "serde".to_string();
        let id1 = graph.add_external_node(path.clone(), ExternalKind::Crate);
        let id2 = graph.add_external_node(path, ExternalKind::Crate);

        assert_eq!(id1, id2);
        assert_eq!(graph.node_count(), 1);
    }

    // 6. Adicionar aresta
    #[test]
    fn test_add_edge() {
        let mut graph = DependencyGraph::new();
        let a =
            graph.add_internal_node("A".to_string(), "my_crate".to_string(), NodeId::test_new(0));
        let b =
            graph.add_internal_node("B".to_string(), "my_crate".to_string(), NodeId::test_new(1));

        let edge = GraphEdge {
            imported_item: "Foo".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "A::Foo".to_string(),
        };

        let edge_id = graph.add_edge(a, b, edge.clone()).unwrap();
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.edge(edge_id), &edge);
    }

    // 7. Múltiplas arestas
    #[test]
    fn test_multiple_edges() {
        let mut graph = DependencyGraph::new();
        let a =
            graph.add_internal_node("A".to_string(), "my_crate".to_string(), NodeId::test_new(0));
        let b =
            graph.add_internal_node("B".to_string(), "my_crate".to_string(), NodeId::test_new(1));

        let edge1 = GraphEdge {
            imported_item: "X".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "A::X".to_string(),
        };
        let edge2 = GraphEdge {
            imported_item: "Y".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "A::Y".to_string(),
        };

        graph.add_edge(a, b, edge1).unwrap();
        graph.add_edge(a, b, edge2).unwrap();

        assert_eq!(graph.edge_count(), 2);
    }

    // 8. add_edge com ID inválido
    #[test]
    fn test_add_edge_invalid_id() {
        let mut graph = DependencyGraph::new();
        let a =
            graph.add_internal_node("A".to_string(), "my_crate".to_string(), NodeId::test_new(0));
        let invalid = GraphNodeId::test_new(999);

        let edge = GraphEdge {
            imported_item: "X".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "A::X".to_string(),
        };

        let res = graph.add_edge(a, invalid, edge);
        assert_eq!(res, Err(GraphError::InvalidNodeId));
    }

    // 9. outgoing_edges e incoming_edges
    #[test]
    fn test_outgoing_incoming_edges() {
        let mut graph = DependencyGraph::new();
        let a =
            graph.add_internal_node("A".to_string(), "my_crate".to_string(), NodeId::test_new(0));
        let b =
            graph.add_internal_node("B".to_string(), "my_crate".to_string(), NodeId::test_new(1));
        let c =
            graph.add_internal_node("C".to_string(), "my_crate".to_string(), NodeId::test_new(2));

        let edge1 = GraphEdge {
            imported_item: "X".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "A::X".to_string(),
        };
        let edge2 = GraphEdge {
            imported_item: "Y".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "B::Y".to_string(),
        };

        graph.add_edge(a, b, edge1.clone()).unwrap();
        graph.add_edge(b, c, edge2.clone()).unwrap();

        let out_a: Vec<_> = graph.outgoing_edges(a).collect();
        assert_eq!(out_a.len(), 1);
        assert_eq!(out_a[0].0, b);
        assert_eq!(out_a[0].1, &edge1);

        let in_c: Vec<_> = graph.incoming_edges(c).collect();
        assert_eq!(in_c.len(), 1);
        assert_eq!(in_c[0].0, b);
        assert_eq!(in_c[0].1, &edge2);

        let out_c: Vec<_> = graph.outgoing_edges(c).collect();
        assert!(out_c.is_empty());
    }

    // 10. out_degree e in_degree
    #[test]
    fn test_degrees() {
        let mut graph = DependencyGraph::new();
        let a =
            graph.add_internal_node("A".to_string(), "my_crate".to_string(), NodeId::test_new(0));
        let b =
            graph.add_internal_node("B".to_string(), "my_crate".to_string(), NodeId::test_new(1));
        let c =
            graph.add_internal_node("C".to_string(), "my_crate".to_string(), NodeId::test_new(2));

        let edge = GraphEdge {
            imported_item: "X".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "X".to_string(),
        };

        graph.add_edge(a, b, edge.clone()).unwrap();
        graph.add_edge(b, c, edge).unwrap();

        assert_eq!(graph.out_degree(a), 1);
        assert_eq!(graph.in_degree(c), 1);
        assert_eq!(graph.out_degree(c), 0);
        assert_eq!(graph.in_degree(a), 0);
        assert_eq!(graph.out_degree(b), 1);
        assert_eq!(graph.in_degree(b), 1);
    }

    // 11. Iteração filtrada por kind
    #[test]
    fn test_filtered_nodes_iteration() {
        let mut graph = DependencyGraph::new();
        let _ =
            graph.add_internal_node("A".to_string(), "my_crate".to_string(), NodeId::test_new(0));
        let _ =
            graph.add_internal_node("B".to_string(), "my_crate".to_string(), NodeId::test_new(1));
        let _ = graph.add_external_node("X".to_string(), ExternalKind::Crate);
        let _ = graph.add_external_node("Y".to_string(), ExternalKind::Crate);
        let _ = graph.add_external_node("Z".to_string(), ExternalKind::Stdlib);

        assert_eq!(graph.all_nodes().count(), 5);
        assert_eq!(graph.internal_nodes().count(), 2);
        assert_eq!(graph.external_nodes().count(), 3);
    }

    // 12. all_edges retorna endpoints correctos
    #[test]
    fn test_all_edges_endpoints() {
        let mut graph = DependencyGraph::new();
        let a =
            graph.add_internal_node("A".to_string(), "my_crate".to_string(), NodeId::test_new(0));
        let b =
            graph.add_internal_node("B".to_string(), "my_crate".to_string(), NodeId::test_new(1));

        let edge = GraphEdge {
            imported_item: "Foo".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "A::Foo".to_string(),
        };

        graph.add_edge(a, b, edge.clone()).unwrap();

        let all: Vec<_> = graph.all_edges().collect();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, a);
        assert_eq!(all[0].1, b);
        assert_eq!(all[0].2, &edge);
    }
}
