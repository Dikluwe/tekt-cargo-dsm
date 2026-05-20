/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/cycle_detector.md
 * @layer L1
 * @updated 2026-05-20
 */

use crate::entities::dependency_graph::{DependencyGraph, GraphNodeId};
use petgraph::visit::EdgeRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    pub nodes: Vec<GraphNodeId>,
    pub kind: CycleKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleKind {
    MultiNode,
    SelfLoop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleReport {
    pub cycles: Vec<Cycle>,
}

impl CycleReport {
    pub fn has_cycles(&self) -> bool {
        !self.cycles.is_empty()
    }

    pub fn cycle_count(&self) -> usize {
        self.cycles.len()
    }

    pub fn affected_node_count(&self) -> usize {
        self.cycles.iter().map(|c| c.nodes.len()).sum()
    }

    pub fn self_loop_count(&self) -> usize {
        self.cycles
            .iter()
            .filter(|c| matches!(c.kind, CycleKind::SelfLoop))
            .count()
    }

    pub fn multi_node_cycle_count(&self) -> usize {
        self.cycles
            .iter()
            .filter(|c| matches!(c.kind, CycleKind::MultiNode))
            .count()
    }

    pub fn multi_node_cycles(&self) -> impl Iterator<Item = &Cycle> {
        self.cycles
            .iter()
            .filter(|c| matches!(c.kind, CycleKind::MultiNode))
    }

    pub fn self_loops(&self) -> impl Iterator<Item = &Cycle> {
        self.cycles
            .iter()
            .filter(|c| matches!(c.kind, CycleKind::SelfLoop))
    }
}

fn has_self_loop(
    raw_graph: &petgraph::Graph<
        crate::entities::dependency_graph::GraphNode,
        crate::entities::dependency_graph::GraphEdge,
        petgraph::Directed,
    >,
    node: petgraph::graph::NodeIndex,
) -> bool {
    raw_graph.edges(node).any(|e| e.target() == node)
}

pub fn detect_cycles(graph: &DependencyGraph) -> CycleReport {
    let raw = graph.raw_graph();
    let sccs = petgraph::algo::tarjan_scc(raw);

    let mut cycles = Vec::new();

    for scc in sccs {
        if scc.is_empty() {
            continue;
        }

        if scc.len() == 1 {
            let node_idx = scc[0];
            if has_self_loop(raw, node_idx) {
                cycles.push(Cycle {
                    nodes: vec![GraphNodeId(node_idx)],
                    kind: CycleKind::SelfLoop,
                });
            }
        } else {
            let nodes: Vec<GraphNodeId> = scc.into_iter().map(GraphNodeId).collect();
            cycles.push(Cycle {
                nodes,
                kind: CycleKind::MultiNode,
            });
        }
    }

    // Ordenar os nodes internos de cada ciclo pelo canonical_path do nó
    for cycle in &mut cycles {
        cycle.nodes.sort_by(|a, b| {
            let path_a = &graph.node(*a).canonical_path;
            let path_b = &graph.node(*b).canonical_path;
            path_a.cmp(path_b)
        });
    }

    // Ordenar a lista de ciclos:
    // 1. Tamanho decrescente (nodes.len())
    // 2. Ordem alfabética do canonical_path do primeiro nó
    cycles.sort_by(|a, b| {
        let len_cmp = b.nodes.len().cmp(&a.nodes.len());
        if len_cmp != std::cmp::Ordering::Equal {
            len_cmp
        } else {
            let first_a = a.nodes.first().map(|n| &graph.node(*n).canonical_path);
            let first_b = b.nodes.first().map(|n| &graph.node(*n).canonical_path);
            first_a.cmp(&first_b)
        }
    });

    CycleReport { cycles }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::dependency_graph::{ExternalKind, GraphEdge};
    use crate::entities::module_tree::NodeId;

    fn create_dummy_edge() -> GraphEdge {
        GraphEdge {
            imported_item: "dummy".to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: "dummy".to_string(),
        }
    }

    // 1. Grafo vazio
    #[test]
    fn test_detect_cycles_empty() {
        let graph = DependencyGraph::new();
        let report = detect_cycles(&graph);
        assert!(!report.has_cycles());
        assert_eq!(report.cycle_count(), 0);
    }

    // 2. Grafo sem ciclos (DAG)
    #[test]
    fn test_detect_cycles_dag() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        let b = graph.add_internal_node("B".to_string(), "crate".to_string(), NodeId::test_new(1));
        let c = graph.add_internal_node("C".to_string(), "crate".to_string(), NodeId::test_new(2));

        graph.add_edge(a, b, create_dummy_edge()).unwrap();
        graph.add_edge(b, c, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert!(!report.has_cycles());
    }

    // 3. Ciclo de 2 nós
    #[test]
    fn test_detect_cycles_two_nodes() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        let b = graph.add_internal_node("B".to_string(), "crate".to_string(), NodeId::test_new(1));

        graph.add_edge(a, b, create_dummy_edge()).unwrap();
        graph.add_edge(b, a, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert!(report.has_cycles());
        assert_eq!(report.cycle_count(), 1);
        assert_eq!(report.multi_node_cycle_count(), 1);
        assert_eq!(report.self_loop_count(), 0);

        let cycle = &report.cycles[0];
        assert_eq!(cycle.kind, CycleKind::MultiNode);
        assert_eq!(cycle.nodes, vec![a, b]); // A vem antes de B alfabeticamente
    }

    // 4. Ciclo de 3 nós
    #[test]
    fn test_detect_cycles_three_nodes() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        let b = graph.add_internal_node("B".to_string(), "crate".to_string(), NodeId::test_new(1));
        let c = graph.add_internal_node("C".to_string(), "crate".to_string(), NodeId::test_new(2));

        graph.add_edge(a, b, create_dummy_edge()).unwrap();
        graph.add_edge(b, c, create_dummy_edge()).unwrap();
        graph.add_edge(c, a, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert!(report.has_cycles());
        assert_eq!(report.cycle_count(), 1);
        assert_eq!(report.cycles[0].nodes, vec![a, b, c]);
    }

    // 5. Self-loop
    #[test]
    fn test_detect_cycles_self_loop() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        graph.add_edge(a, a, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert!(report.has_cycles());
        assert_eq!(report.cycle_count(), 1);
        assert_eq!(report.self_loop_count(), 1);
        assert_eq!(report.cycles[0].nodes, vec![a]);
    }

    // 6. Múltiplos ciclos disjuntos
    #[test]
    fn test_detect_cycles_multiple_disjoint() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        let b = graph.add_internal_node("B".to_string(), "crate".to_string(), NodeId::test_new(1));
        let c = graph.add_internal_node("C".to_string(), "crate".to_string(), NodeId::test_new(2));
        let d = graph.add_internal_node("D".to_string(), "crate".to_string(), NodeId::test_new(3));

        graph.add_edge(a, b, create_dummy_edge()).unwrap();
        graph.add_edge(b, a, create_dummy_edge()).unwrap();

        graph.add_edge(c, d, create_dummy_edge()).unwrap();
        graph.add_edge(d, c, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert_eq!(report.cycle_count(), 2);
        assert_eq!(report.multi_node_cycle_count(), 2);
    }

    // 7. Ciclos sobrepostos
    #[test]
    fn test_detect_cycles_overlapping() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        let b = graph.add_internal_node("B".to_string(), "crate".to_string(), NodeId::test_new(1));
        let c = graph.add_internal_node("C".to_string(), "crate".to_string(), NodeId::test_new(2));

        graph.add_edge(a, b, create_dummy_edge()).unwrap();
        graph.add_edge(b, a, create_dummy_edge()).unwrap();
        graph.add_edge(a, c, create_dummy_edge()).unwrap();
        graph.add_edge(c, a, create_dummy_edge()).unwrap();

        // Esperado 1 SCC contendo {A, B, C}
        let report = detect_cycles(&graph);
        assert_eq!(report.cycle_count(), 1);
        assert_eq!(report.cycles[0].nodes.len(), 3);
        assert_eq!(report.cycles[0].nodes, vec![a, b, c]);
    }

    // 8. Ordenação por tamanho
    #[test]
    fn test_detect_cycles_ordering_by_size() {
        let mut graph = DependencyGraph::new();
        // Ciclo 1 (tamanho 2): B <-> C
        let b = graph.add_internal_node("B".to_string(), "crate".to_string(), NodeId::test_new(1));
        let c = graph.add_internal_node("C".to_string(), "crate".to_string(), NodeId::test_new(2));
        graph.add_edge(b, c, create_dummy_edge()).unwrap();
        graph.add_edge(c, b, create_dummy_edge()).unwrap();

        // Ciclo 2 (tamanho 3): D -> E -> F -> D
        let d = graph.add_internal_node("D".to_string(), "crate".to_string(), NodeId::test_new(3));
        let e = graph.add_internal_node("E".to_string(), "crate".to_string(), NodeId::test_new(4));
        let f = graph.add_internal_node("F".to_string(), "crate".to_string(), NodeId::test_new(5));
        graph.add_edge(d, e, create_dummy_edge()).unwrap();
        graph.add_edge(e, f, create_dummy_edge()).unwrap();
        graph.add_edge(f, d, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert_eq!(report.cycle_count(), 2);
        // O de tamanho 3 (D, E, F) deve vir primeiro
        assert_eq!(report.cycles[0].nodes.len(), 3);
        assert_eq!(report.cycles[1].nodes.len(), 2);
    }

    // 9. Ordenação alfabética secundária
    #[test]
    fn test_detect_cycles_alphabetical_ordering() {
        let mut graph = DependencyGraph::new();
        // Ciclo 1 (tamanho 2): X <-> Y
        let x = graph.add_internal_node("X".to_string(), "crate".to_string(), NodeId::test_new(0));
        let y = graph.add_internal_node("Y".to_string(), "crate".to_string(), NodeId::test_new(1));
        graph.add_edge(x, y, create_dummy_edge()).unwrap();
        graph.add_edge(y, x, create_dummy_edge()).unwrap();

        // Ciclo 2 (tamanho 2): A <-> B
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(2));
        let b = graph.add_internal_node("B".to_string(), "crate".to_string(), NodeId::test_new(3));
        graph.add_edge(a, b, create_dummy_edge()).unwrap();
        graph.add_edge(b, a, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert_eq!(report.cycle_count(), 2);
        // A <-> B deve vir antes de X <-> Y porque "A" < "X"
        assert_eq!(report.cycles[0].nodes, vec![a, b]);
        assert_eq!(report.cycles[1].nodes, vec![x, y]);
    }

    // 10. Nós externos não geram ciclo
    #[test]
    fn test_detect_cycles_external_nodes_no_cycle() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        let ext = graph.add_external_node("serde".to_string(), ExternalKind::Crate);

        graph.add_edge(a, ext, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert!(!report.has_cycles());
    }

    // 11. Misturando self-loops e multi-node
    #[test]
    fn test_detect_cycles_mixed() {
        let mut graph = DependencyGraph::new();
        // Self-loop no A
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        graph.add_edge(a, a, create_dummy_edge()).unwrap();

        // Multi-node B <-> C
        let b = graph.add_internal_node("B".to_string(), "crate".to_string(), NodeId::test_new(1));
        let c = graph.add_internal_node("C".to_string(), "crate".to_string(), NodeId::test_new(2));
        graph.add_edge(b, c, create_dummy_edge()).unwrap();
        graph.add_edge(c, b, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert_eq!(report.cycle_count(), 2);
        assert_eq!(report.multi_node_cycle_count(), 1);
        assert_eq!(report.self_loop_count(), 1);
        // O de tamanho 2 (multi-node) vem antes do de tamanho 1 (self-loop)
        assert_eq!(report.cycles[0].kind, CycleKind::MultiNode);
        assert_eq!(report.cycles[1].kind, CycleKind::SelfLoop);
    }

    // 12. affected_node_count
    #[test]
    fn test_affected_node_count() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        graph.add_edge(a, a, create_dummy_edge()).unwrap();

        let b = graph.add_internal_node("B".to_string(), "crate".to_string(), NodeId::test_new(1));
        let c = graph.add_internal_node("C".to_string(), "crate".to_string(), NodeId::test_new(2));
        graph.add_edge(b, c, create_dummy_edge()).unwrap();
        graph.add_edge(c, b, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert_eq!(report.affected_node_count(), 3);
    }

    // 13. self_loop_count e multi_node_cycle_count
    #[test]
    fn test_counts() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node("A".to_string(), "crate".to_string(), NodeId::test_new(0));
        graph.add_edge(a, a, create_dummy_edge()).unwrap();

        let report = detect_cycles(&graph);
        assert_eq!(report.self_loop_count(), 1);
        assert_eq!(report.multi_node_cycle_count(), 0);
    }
}
