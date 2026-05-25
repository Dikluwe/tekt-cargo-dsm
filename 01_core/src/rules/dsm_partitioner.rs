/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/dsm_partitioner.md
 * @layer L1
 * @updated 2026-05-20
 */

use crate::entities::dependency_graph::{DependencyGraph, GraphNodeId, NodeKind};
use petgraph::algo::tarjan_scc;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Ordenação resultante do particionamento DSM.
///
/// A `order` posiciona dependências antes dos dependentes
/// (convenção DSM clássica: na matriz triangular inferior, a célula
/// `[i, j]` com `j < i` representa "linha `i` depende da coluna `j`").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionedOrder {
    /// Ordem linear final dos nós.
    pub order: Vec<GraphNodeId>,
    /// Para cada posição em `order`, o índice do SCC em `sccs`.
    pub scc_index_per_node: Vec<usize>,
    /// Definição dos SCCs.
    pub sccs: Vec<SccBlock>,
    /// Posição que separa internos de externos.
    pub internal_boundary: usize,
}

/// Bloco contíguo de posições em `PartitionedOrder.order` ocupado
/// por um SCC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SccBlock {
    /// Posições no `order` ocupadas por este SCC.
    pub range: std::ops::Range<usize>,
    /// `true` para SCC com 2+ nós ou com self-loop.
    pub is_cyclic: bool,
}

impl PartitionedOrder {
    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn node_at(&self, index: usize) -> GraphNodeId {
        self.order[index]
    }

    pub fn scc_for_position(&self, index: usize) -> &SccBlock {
        let scc_idx = self.scc_index_per_node[index];
        &self.sccs[scc_idx]
    }

    pub fn cyclic_scc_count(&self) -> usize {
        self.sccs.iter().filter(|s| s.is_cyclic).count()
    }

    pub fn trivial_scc_count(&self) -> usize {
        self.sccs.iter().filter(|s| !s.is_cyclic).count()
    }

    pub fn internal_positions(&self) -> std::ops::Range<usize> {
        0..self.internal_boundary
    }

    pub fn external_positions(&self) -> std::ops::Range<usize> {
        self.internal_boundary..self.order.len()
    }
}

fn is_internal_kind(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::InternalWithTree { .. } | NodeKind::InternalWithoutTree { .. }
    )
}

fn has_self_loop(graph: &DependencyGraph, node: GraphNodeId) -> bool {
    graph.outgoing_edges(node).any(|(t, _)| t == node)
}

/// Particiona o grafo em ordem DSM canónica.
///
/// Algoritmo: Tarjan SCC sobre os nós internos, depois topological
/// sort sobre o grafo condensado das SCCs. A direção do sort coloca
/// dependências (sinks no grafo de imports) antes de dependentes
/// (sources). Tie-break alfabético pelo menor `canonical_path` do
/// SCC. Nós externos vão para o fim, ordenados alfabeticamente.
pub fn partition_for_dsm(graph: &DependencyGraph) -> PartitionedOrder {
    // Separar internos e externos
    let mut internals: Vec<GraphNodeId> = graph
        .all_nodes()
        .filter(|(_, n)| is_internal_kind(&n.kind))
        .map(|(id, _)| id)
        .collect();
    // Ordenar internos é irrelevante aqui (Tarjan reordena), mas
    // facilita determinismo em iterações posteriores.
    internals.sort_by(|a, b| {
        graph
            .node(*a)
            .canonical_path
            .cmp(&graph.node(*b).canonical_path)
    });

    let mut externals: Vec<GraphNodeId> = graph
        .all_nodes()
        .filter(|(_, n)| !is_internal_kind(&n.kind))
        .map(|(id, _)| id)
        .collect();
    externals.sort_by(|a, b| {
        graph
            .node(*a)
            .canonical_path
            .cmp(&graph.node(*b).canonical_path)
    });

    let mut order: Vec<GraphNodeId> = Vec::new();
    let mut scc_index_per_node: Vec<usize> = Vec::new();
    let mut sccs: Vec<SccBlock> = Vec::new();

    if !internals.is_empty() {
        // Subgrafo apenas com nós internos e arestas entre eles
        let mut sub = petgraph::Graph::<GraphNodeId, (), petgraph::Directed>::new();
        let mut to_sub: HashMap<GraphNodeId, petgraph::graph::NodeIndex> =
            HashMap::with_capacity(internals.len());
        let mut from_sub: Vec<GraphNodeId> = Vec::with_capacity(internals.len());

        for &id in &internals {
            let n = sub.add_node(id);
            to_sub.insert(id, n);
            from_sub.push(id);
        }
        for &from_id in &internals {
            let from_sub_idx = to_sub[&from_id];
            for (to_id, _) in graph.outgoing_edges(from_id) {
                if let Some(&to_sub_idx) = to_sub.get(&to_id) {
                    sub.add_edge(from_sub_idx, to_sub_idx, ());
                }
            }
        }

        // Tarjan SCC → Vec<Vec<NodeIndex>>
        let raw_sccs = tarjan_scc(&sub);

        // Converter para Vec<Vec<GraphNodeId>> com nós ordenados alfabeticamente
        let sccs_as_ids: Vec<Vec<GraphNodeId>> = raw_sccs
            .into_iter()
            .map(|scc| {
                let mut ids: Vec<GraphNodeId> =
                    scc.into_iter().map(|n| from_sub[n.index()]).collect();
                ids.sort_by(|a, b| {
                    graph
                        .node(*a)
                        .canonical_path
                        .cmp(&graph.node(*b).canonical_path)
                });
                ids
            })
            .collect();

        // Map node → índice do SCC (no espaço dos sccs_as_ids actual)
        let mut node_to_scc: HashMap<GraphNodeId, usize> = HashMap::new();
        for (i, scc) in sccs_as_ids.iter().enumerate() {
            for &n in scc {
                node_to_scc.insert(n, i);
            }
        }

        // Arestas no condensado (cross-SCC, deduplicadas)
        let mut cond_edges: HashSet<(usize, usize)> = HashSet::new();
        for &from_id in &internals {
            let from_scc = node_to_scc[&from_id];
            for (to_id, _) in graph.outgoing_edges(from_id) {
                if let Some(&to_scc) = node_to_scc.get(&to_id)
                    && from_scc != to_scc
                {
                    cond_edges.insert((from_scc, to_scc));
                }
            }
        }

        // Topological sort do condensado, regra DSM "dependência primeiro":
        // - Aresta original from→to significa "from depende de to".
        // - Na ordem, `to` (dependência) aparece antes de `from`.
        // - in_degree[s] = quantos outros SCCs `s` depende.
        // - Quando in_degree[s] chega a 0, todas as dependências de s já
        //   foram colocadas, podemos colocar s.
        let n_sccs = sccs_as_ids.len();
        let mut in_degree = vec![0usize; n_sccs];
        let mut adj_out: Vec<Vec<usize>> = vec![Vec::new(); n_sccs];
        for &(from_scc, to_scc) in &cond_edges {
            in_degree[from_scc] += 1;
            adj_out[to_scc].push(from_scc);
        }
        // Estabilidade da ordem de visitação dos sucessores
        for adj in adj_out.iter_mut() {
            adj.sort_unstable();
        }

        let scc_key = |i: usize| -> &str { graph.node(sccs_as_ids[i][0]).canonical_path.as_str() };

        let mut heap: BinaryHeap<Reverse<(&str, usize)>> = BinaryHeap::new();
        for (i, &deg) in in_degree.iter().enumerate() {
            if deg == 0 {
                heap.push(Reverse((scc_key(i), i)));
            }
        }

        let mut topo: Vec<usize> = Vec::with_capacity(n_sccs);
        while let Some(Reverse((_, i))) = heap.pop() {
            topo.push(i);
            // Clone para evitar borrow conflict com `in_degree`
            let neighbors = adj_out[i].clone();
            for j in neighbors {
                in_degree[j] -= 1;
                if in_degree[j] == 0 {
                    heap.push(Reverse((scc_key(j), j)));
                }
            }
        }

        // Expandir SCCs em order na ordem topológica
        for &scc_orig_idx in &topo {
            let scc_nodes = &sccs_as_ids[scc_orig_idx];
            let start = order.len();
            for &n in scc_nodes {
                order.push(n);
            }
            let end = order.len();

            let is_cyclic =
                scc_nodes.len() > 1 || (scc_nodes.len() == 1 && has_self_loop(graph, scc_nodes[0]));

            let new_idx = sccs.len();
            sccs.push(SccBlock {
                range: start..end,
                is_cyclic,
            });
            for _ in start..end {
                scc_index_per_node.push(new_idx);
            }
        }
    }

    let internal_boundary = order.len();

    // Externos no fim, cada um forma SCC trivial
    for &ext_id in &externals {
        let pos = order.len();
        order.push(ext_id);
        let new_idx = sccs.len();
        sccs.push(SccBlock {
            range: pos..pos + 1,
            is_cyclic: false,
        });
        scc_index_per_node.push(new_idx);
    }

    PartitionedOrder {
        order,
        scc_index_per_node,
        sccs,
        internal_boundary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::dependency_graph::{ExternalKind, GraphEdge};
    use crate::entities::module_tree::NodeId;

    fn placeholder_edge(item: &str) -> GraphEdge {
        GraphEdge {
            imported_item: item.to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: item.to_string(),
        }
    }

    fn add_int(g: &mut DependencyGraph, path: &str) -> GraphNodeId {
        g.add_internal_node_with_tree(path.into(), "c".into(), NodeId::test_new(0))
    }

    fn add_ext(g: &mut DependencyGraph, path: &str) -> GraphNodeId {
        g.add_external_node(path.into(), ExternalKind::Crate)
    }

    fn paths(p: &PartitionedOrder, graph: &DependencyGraph) -> Vec<String> {
        p.order
            .iter()
            .map(|id| graph.node(*id).canonical_path.clone())
            .collect()
    }

    // 1. Grafo vazio
    #[test]
    fn test_empty_graph() {
        let g = DependencyGraph::new();
        let p = partition_for_dsm(&g);
        assert!(p.order.is_empty());
        assert!(p.is_empty());
        assert_eq!(p.len(), 0);
        assert!(p.sccs.is_empty());
        assert!(p.scc_index_per_node.is_empty());
        assert_eq!(p.internal_boundary, 0);
    }

    // 2. Um nó interno
    #[test]
    fn test_single_internal() {
        let mut g = DependencyGraph::new();
        add_int(&mut g, "A");
        let p = partition_for_dsm(&g);
        assert_eq!(p.order.len(), 1);
        assert_eq!(p.sccs.len(), 1);
        assert!(!p.sccs[0].is_cyclic);
        assert_eq!(p.internal_boundary, 1);
    }

    // 3. Um nó externo
    #[test]
    fn test_single_external() {
        let mut g = DependencyGraph::new();
        add_ext(&mut g, "serde");
        let p = partition_for_dsm(&g);
        assert_eq!(p.order.len(), 1);
        assert_eq!(p.internal_boundary, 0);
        assert!(!p.sccs[0].is_cyclic);
        assert_eq!(p.external_positions().count(), 1);
    }

    // 4. DAG simples A→B→C (A depende de B depende de C).
    // Regra DSM: dependência primeiro ⇒ ordem [C, B, A].
    #[test]
    fn test_dag_simple_dependency_first() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        let b = add_int(&mut g, "B");
        let c = add_int(&mut g, "C");
        g.add_edge(a, b, placeholder_edge("x")).unwrap();
        g.add_edge(b, c, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        assert_eq!(paths(&p, &g), vec!["C", "B", "A"]);
        assert_eq!(p.sccs.len(), 3);
        assert!(p.sccs.iter().all(|s| !s.is_cyclic));
        assert_eq!(p.internal_boundary, 3);
    }

    // 5. DAG com escolha alfabética: A→C, B→C.
    // C é a dependência comum, vem primeiro. A e B (sem dep mútua)
    // empatam; tie-break alfabético: A, B.
    #[test]
    fn test_dag_alphabetical_tiebreak() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        let b = add_int(&mut g, "B");
        let c = add_int(&mut g, "C");
        g.add_edge(a, c, placeholder_edge("x")).unwrap();
        g.add_edge(b, c, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        assert_eq!(paths(&p, &g), vec!["C", "A", "B"]);
    }

    // 6. Ciclo de 2 nós: A↔B
    #[test]
    fn test_cycle_two_nodes() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        let b = add_int(&mut g, "B");
        g.add_edge(a, b, placeholder_edge("x")).unwrap();
        g.add_edge(b, a, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        assert_eq!(p.sccs.len(), 1);
        assert!(p.sccs[0].is_cyclic);
        assert_eq!(p.sccs[0].range, 0..2);
        assert_eq!(paths(&p, &g), vec!["A", "B"]);
        assert_eq!(p.internal_boundary, 2);
    }

    // 7. Ciclo + DAG: A↔B (SCC) + C→A (C depende do ciclo).
    // C aponta para o ciclo ⇒ ciclo primeiro, C depois.
    // Ordem: [A, B, C].
    #[test]
    fn test_cycle_with_external_dependent() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        let b = add_int(&mut g, "B");
        let c = add_int(&mut g, "C");
        g.add_edge(a, b, placeholder_edge("x")).unwrap();
        g.add_edge(b, a, placeholder_edge("x")).unwrap();
        g.add_edge(c, a, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        assert_eq!(paths(&p, &g), vec!["A", "B", "C"]);
        assert_eq!(p.sccs.len(), 2);
        assert!(p.sccs[0].is_cyclic);
        assert_eq!(p.sccs[0].range, 0..2);
        assert!(!p.sccs[1].is_cyclic);
        assert_eq!(p.sccs[1].range, 2..3);
    }

    // 8. Self-loop A→A
    #[test]
    fn test_self_loop() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        g.add_edge(a, a, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        assert_eq!(p.sccs.len(), 1);
        assert!(p.sccs[0].is_cyclic, "self-loop conta como cíclico");
        assert_eq!(p.sccs[0].range, 0..1);
    }

    // 9. Internos + externos: A, B internos + X externo. B→X.
    // Externos NÃO entram no Tarjan; internos sao SCC triviais
    // independentes ⇒ alfabético [A, B]. X depois.
    #[test]
    fn test_internals_and_externals() {
        let mut g = DependencyGraph::new();
        let _a = add_int(&mut g, "A");
        let b = add_int(&mut g, "B");
        let x = add_ext(&mut g, "X");
        g.add_edge(b, x, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        assert_eq!(paths(&p, &g), vec!["A", "B", "X"]);
        assert_eq!(p.internal_boundary, 2);
    }

    // 10. Múltiplos externos ordenados alfabeticamente
    #[test]
    fn test_multiple_externals_sorted() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        let tokio = add_ext(&mut g, "tokio");
        let std_io = add_ext(&mut g, "std::io");
        let serde_de = add_ext(&mut g, "serde::de");
        g.add_edge(a, tokio, placeholder_edge("x")).unwrap();
        g.add_edge(a, std_io, placeholder_edge("x")).unwrap();
        g.add_edge(a, serde_de, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        assert_eq!(paths(&p, &g), vec!["A", "serde::de", "std::io", "tokio"]);
        assert_eq!(p.internal_boundary, 1);
    }

    // 11. scc_index_per_node aponta correctamente
    #[test]
    fn test_scc_index_per_node_consistency() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        let b = add_int(&mut g, "B");
        g.add_edge(a, b, placeholder_edge("x")).unwrap();
        g.add_edge(b, a, placeholder_edge("x")).unwrap();
        add_int(&mut g, "C");
        let p = partition_for_dsm(&g);
        for i in 0..p.order.len() {
            let scc = p.scc_for_position(i);
            assert!(
                scc.range.contains(&i),
                "posição {} deveria estar contida no range {:?}",
                i,
                scc.range,
            );
        }
    }

    // 12. Iteradores internal_positions / external_positions
    #[test]
    fn test_internal_external_position_iterators() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        add_int(&mut g, "B");
        let x = add_ext(&mut g, "X");
        let y = add_ext(&mut g, "Y");
        g.add_edge(a, x, placeholder_edge("x")).unwrap();
        g.add_edge(a, y, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        assert_eq!(p.internal_positions().collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(p.external_positions().collect::<Vec<_>>(), vec![2, 3]);
    }

    // 13. Ciclos internos permanecem agrupados; externos no fim
    #[test]
    fn test_cycles_dont_interact_with_externals() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        let b = add_int(&mut g, "B");
        let x = add_ext(&mut g, "X");
        g.add_edge(a, b, placeholder_edge("x")).unwrap();
        g.add_edge(b, a, placeholder_edge("x")).unwrap();
        g.add_edge(a, x, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        assert_eq!(paths(&p, &g), vec!["A", "B", "X"]);
        assert_eq!(p.cyclic_scc_count(), 1);
        assert_eq!(p.trivial_scc_count(), 1); // só o externo
        assert_eq!(p.internal_boundary, 2);
    }

    // 14. Determinismo
    #[test]
    fn test_determinism() {
        let mut g = DependencyGraph::new();
        let a = add_int(&mut g, "A");
        let b = add_int(&mut g, "B");
        let c = add_int(&mut g, "C");
        let d = add_ext(&mut g, "D");
        g.add_edge(a, b, placeholder_edge("x")).unwrap();
        g.add_edge(b, c, placeholder_edge("x")).unwrap();
        g.add_edge(c, d, placeholder_edge("x")).unwrap();
        let p1 = partition_for_dsm(&g);
        let p2 = partition_for_dsm(&g);
        assert_eq!(p1, p2);
    }

    // 15. Cenário realista pequeno
    #[test]
    fn test_realistic_small() {
        // 5 internos: cli → app → core, cli → utils, app → utils
        // 2 externos: clap, serde
        let mut g = DependencyGraph::new();
        let cli = add_int(&mut g, "cli");
        let app = add_int(&mut g, "app");
        let core = add_int(&mut g, "core");
        let utils = add_int(&mut g, "utils");
        let _data = add_int(&mut g, "data"); // sem ligações
        let clap = add_ext(&mut g, "clap");
        let serde = add_ext(&mut g, "serde");

        g.add_edge(cli, app, placeholder_edge("x")).unwrap();
        g.add_edge(app, core, placeholder_edge("x")).unwrap();
        g.add_edge(cli, utils, placeholder_edge("x")).unwrap();
        g.add_edge(app, utils, placeholder_edge("x")).unwrap();
        g.add_edge(cli, clap, placeholder_edge("x")).unwrap();
        g.add_edge(core, serde, placeholder_edge("x")).unwrap();

        let p = partition_for_dsm(&g);

        // 5 internos + 2 externos
        assert_eq!(p.order.len(), 7);
        assert_eq!(p.internal_boundary, 5);
        // Nenhum ciclo
        assert_eq!(p.cyclic_scc_count(), 0);
        assert_eq!(p.trivial_scc_count(), 7);

        let names = paths(&p, &g);
        // core e utils sao folhas (in_degree=0 no Kahn-invertido); ordem
        // por tie-break alfabetico: core antes de data antes de utils.
        // app depende de core+utils; cli depende de app+utils+clap.
        let pos = |name: &str| names.iter().position(|n| n == name).unwrap();
        assert!(pos("core") < pos("app"));
        assert!(pos("utils") < pos("app"));
        assert!(pos("utils") < pos("cli"));
        assert!(pos("app") < pos("cli"));
        // Externos no fim, alfabeticos
        assert_eq!(names[5], "clap");
        assert_eq!(names[6], "serde");
    }
}
