/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/json_serializer.md
 * @prompt 00_nucleo/prompts/dependency_graph-revisao.md
 * @layer L3
 * @updated 2026-05-20
 */

use crystalline_dsm_core::entities::dependency_graph::{
    DependencyGraph, ExternalKind, GraphEdge, GraphError, NodeKind,
};
use crystalline_dsm_core::entities::workspace::Workspace;
use crystalline_dsm_core::rules::cycle_detector::{Cycle, CycleKind, CycleReport};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "1.0.0";
pub const TOOL_NAME: &str = "crystalline-dsm";

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphJsonDto {
    pub schema_version: String,
    pub generated_at: String,
    pub tool: ToolInfoDto,
    pub workspace: WorkspaceInfoDto,
    pub graph: GraphDataDto,
    pub cycles: CyclesDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolInfoDto {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceInfoDto {
    pub root: String,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphDataDto {
    pub nodes: Vec<NodeDto>,
    pub edges: Vec<EdgeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeDto {
    pub canonical_path: String,
    pub kind: NodeKindDto,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub crate_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub external_kind: Option<ExternalKindDto>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeKindDto {
    Internal,
    External,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExternalKindDto {
    Crate,
    Stdlib,
}

/// `alias` é serializado mesmo quando `None` (vira `null`),
/// por previsibilidade do consumidor — ver ADR-0009.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdgeDto {
    pub from: String,
    pub to: String,
    pub imported_item: String,
    pub alias: Option<String>,
    pub is_reexport: bool,
    pub is_glob: bool,
    pub raw_use_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CyclesDto {
    pub count: usize,
    pub self_loop_count: usize,
    pub multi_node_count: usize,
    pub items: Vec<CycleDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CycleDto {
    pub kind: CycleKindDto,
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CycleKindDto {
    MultiNode,
    SelfLoop,
}

// ============================================================================
// Erros
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum JsonSerializeError {
    #[error("Falha ao serializar para JSON: {source}")]
    SerdeError {
        #[from]
        source: serde_json::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum JsonDeserializeError {
    #[error("Falha ao parsear JSON: {source}")]
    SerdeError {
        #[from]
        source: serde_json::Error,
    },

    #[error("Versão de schema incompatível: esperado 1.x.y, recebido {version}")]
    IncompatibleSchemaVersion { version: String },

    #[error("Nó referenciado em aresta não existe: {canonical_path}")]
    DanglingEdgeReference { canonical_path: String },

    #[error("Nó referenciado em ciclo não existe: {canonical_path}")]
    DanglingCycleReference { canonical_path: String },

    #[error("Erro ao construir grafo: {source}")]
    GraphConstructionError {
        #[from]
        source: GraphError,
    },
}

// ============================================================================
// Conversão Domínio → DTO
// ============================================================================

pub(crate) fn to_dto(
    graph: &DependencyGraph,
    cycles: &CycleReport,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
) -> GraphJsonDto {
    let mut nodes: Vec<NodeDto> = graph
        .all_nodes()
        .map(|(_, n)| match &n.kind {
            // Ambas as variantes internas mapeiam para o mesmo
            // NodeKindDto::Internal — `tree_node_id` não vai para o JSON.
            NodeKind::InternalWithTree { crate_name, .. }
            | NodeKind::InternalWithoutTree { crate_name } => NodeDto {
                canonical_path: n.canonical_path.clone(),
                kind: NodeKindDto::Internal,
                crate_name: Some(crate_name.clone()),
                external_kind: None,
            },
            NodeKind::External { kind } => NodeDto {
                canonical_path: n.canonical_path.clone(),
                kind: NodeKindDto::External,
                crate_name: None,
                external_kind: Some(match kind {
                    ExternalKind::Crate => ExternalKindDto::Crate,
                    ExternalKind::Stdlib => ExternalKindDto::Stdlib,
                }),
            },
        })
        .collect();
    nodes.sort_by(|a, b| a.canonical_path.cmp(&b.canonical_path));

    let mut edges: Vec<EdgeDto> = graph
        .all_edges()
        .map(|(from_id, to_id, edge)| EdgeDto {
            from: graph.node(from_id).canonical_path.clone(),
            to: graph.node(to_id).canonical_path.clone(),
            imported_item: edge.imported_item.clone(),
            alias: edge.alias.clone(),
            is_reexport: edge.is_reexport,
            is_glob: edge.is_glob,
            raw_use_path: edge.raw_use_path.clone(),
        })
        .collect();
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.imported_item.cmp(&b.imported_item))
    });

    // Ordem alfabética dentro de cada ciclo (limitação documentada: a
    // ordem cíclica real não é unicamente definida; ver ADR-0006).
    let mut cycle_items: Vec<CycleDto> = cycles
        .cycles
        .iter()
        .map(|c| {
            let mut paths: Vec<String> = c
                .nodes
                .iter()
                .map(|id| graph.node(*id).canonical_path.clone())
                .collect();
            paths.sort();
            CycleDto {
                kind: match c.kind {
                    CycleKind::MultiNode => CycleKindDto::MultiNode,
                    CycleKind::SelfLoop => CycleKindDto::SelfLoop,
                },
                nodes: paths,
            }
        })
        .collect();
    cycle_items.sort_by(|a, b| {
        b.nodes.len().cmp(&a.nodes.len()).then_with(|| {
            let aa = a.nodes.first().map(|s| s.as_str()).unwrap_or("");
            let bb = b.nodes.first().map(|s| s.as_str()).unwrap_or("");
            aa.cmp(bb)
        })
    });

    let mut members: Vec<String> = workspace.members.iter().map(|m| m.name.clone()).collect();
    members.sort();

    GraphJsonDto {
        schema_version: SCHEMA_VERSION.to_string(),
        generated_at: generated_at.to_string(),
        tool: ToolInfoDto {
            name: TOOL_NAME.to_string(),
            version: tool_version.to_string(),
        },
        workspace: WorkspaceInfoDto {
            root: workspace.root.to_string_lossy().into_owned(),
            members,
        },
        graph: GraphDataDto { nodes, edges },
        cycles: CyclesDto {
            count: cycles.cycle_count(),
            self_loop_count: cycles.self_loop_count(),
            multi_node_count: cycles.multi_node_cycle_count(),
            items: cycle_items,
        },
    }
}

// ============================================================================
// Conversão DTO → Domínio
// ============================================================================

pub(crate) fn from_dto(
    dto: GraphJsonDto,
) -> Result<(DependencyGraph, CycleReport), JsonDeserializeError> {
    if !is_compatible_schema_version(&dto.schema_version) {
        return Err(JsonDeserializeError::IncompatibleSchemaVersion {
            version: dto.schema_version,
        });
    }

    let mut graph = DependencyGraph::new();

    for node in &dto.graph.nodes {
        match node.kind {
            NodeKindDto::Internal => {
                // Reconstrução sem árvore — variante explícita por ADR-0010.
                graph.add_internal_node_without_tree(
                    node.canonical_path.clone(),
                    node.crate_name.clone().unwrap_or_default(),
                );
            }
            NodeKindDto::External => {
                let ek = match node.external_kind {
                    Some(ExternalKindDto::Crate) => ExternalKind::Crate,
                    Some(ExternalKindDto::Stdlib) => ExternalKind::Stdlib,
                    None => ExternalKind::Crate,
                };
                graph.add_external_node(node.canonical_path.clone(), ek);
            }
        }
    }

    for edge in &dto.graph.edges {
        let from_id = graph.find_node(&edge.from).ok_or_else(|| {
            JsonDeserializeError::DanglingEdgeReference {
                canonical_path: edge.from.clone(),
            }
        })?;
        let to_id = graph.find_node(&edge.to).ok_or_else(|| {
            JsonDeserializeError::DanglingEdgeReference {
                canonical_path: edge.to.clone(),
            }
        })?;
        graph.add_edge(
            from_id,
            to_id,
            GraphEdge {
                imported_item: edge.imported_item.clone(),
                alias: edge.alias.clone(),
                is_reexport: edge.is_reexport,
                is_glob: edge.is_glob,
                raw_use_path: edge.raw_use_path.clone(),
            },
        )?;
    }

    let mut cycles_vec = Vec::with_capacity(dto.cycles.items.len());
    for c in &dto.cycles.items {
        let mut ids = Vec::with_capacity(c.nodes.len());
        for path in &c.nodes {
            let id = graph.find_node(path).ok_or_else(|| {
                JsonDeserializeError::DanglingCycleReference {
                    canonical_path: path.clone(),
                }
            })?;
            ids.push(id);
        }
        cycles_vec.push(Cycle {
            nodes: ids,
            kind: match c.kind {
                CycleKindDto::MultiNode => CycleKind::MultiNode,
                CycleKindDto::SelfLoop => CycleKind::SelfLoop,
            },
        });
    }

    Ok((graph, CycleReport { cycles: cycles_vec }))
}

/// Aceita qualquer "1.x.y"; rejeita major diferente.
fn is_compatible_schema_version(version: &str) -> bool {
    matches!(version.split('.').next(), Some("1"))
}

// ============================================================================
// API pública: JSON
// ============================================================================

pub fn to_canonical_json(
    graph: &DependencyGraph,
    cycles: &CycleReport,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
) -> Result<String, JsonSerializeError> {
    let dto = to_dto(graph, cycles, workspace, tool_version, generated_at);
    Ok(serde_json::to_string_pretty(&dto)?)
}

pub fn from_canonical_json(
    json: &str,
) -> Result<(DependencyGraph, CycleReport, GraphJsonDto), JsonDeserializeError> {
    let dto: GraphJsonDto = serde_json::from_str(json)?;
    let dto_clone = dto.clone();
    let (graph, cycles) = from_dto(dto)?;
    Ok((graph, cycles, dto_clone))
}

// ============================================================================
// Testes
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crystalline_dsm_core::entities::workspace::{EntryKind, Workspace, WorkspaceMember};
    use std::path::PathBuf;

    const GENERATED_AT: &str = "2026-05-20T14:32:18Z";
    const TOOL_VERSION: &str = "0.1.0";

    fn empty_workspace() -> Workspace {
        Workspace {
            root: PathBuf::from("/test/ws"),
            members: vec![],
        }
    }

    fn workspace_with_members(names: &[&str]) -> Workspace {
        Workspace {
            root: PathBuf::from("/test/ws"),
            members: names
                .iter()
                .map(|n| WorkspaceMember {
                    name: (*n).to_string(),
                    crate_root: PathBuf::from(format!("/test/ws/{}", n)),
                    entry_kind: EntryKind::Library {
                        lib_path: PathBuf::from(format!("/test/ws/{}/src/lib.rs", n)),
                    },
                })
                .collect(),
        }
    }

    fn empty_cycles() -> CycleReport {
        CycleReport { cycles: vec![] }
    }

    fn simple_edge(item: &str, raw: &str) -> GraphEdge {
        GraphEdge {
            imported_item: item.to_string(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: raw.to_string(),
        }
    }

    fn serialize_to_value(
        graph: &DependencyGraph,
        cycles: &CycleReport,
        workspace: &Workspace,
    ) -> serde_json::Value {
        let json = to_canonical_json(graph, cycles, workspace, TOOL_VERSION, GENERATED_AT).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    // 1. Serializar grafo vazio
    #[test]
    fn test_serialize_empty_graph() {
        let graph = DependencyGraph::new();
        let v = serialize_to_value(&graph, &empty_cycles(), &empty_workspace());
        assert_eq!(v["graph"]["nodes"].as_array().unwrap().len(), 0);
        assert_eq!(v["graph"]["edges"].as_array().unwrap().len(), 0);
        assert_eq!(v["cycles"]["count"], 0);
    }

    // 2. Serializar grafo com 1 nó interno
    #[test]
    fn test_serialize_one_internal_node() {
        let mut graph = DependencyGraph::new();
        graph.add_internal_node_without_tree("my_crate::foo".to_string(), "my_crate".to_string());
        let v = serialize_to_value(&graph, &empty_cycles(), &empty_workspace());
        let nodes = v["graph"]["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0]["canonical_path"], "my_crate::foo");
        assert_eq!(nodes[0]["kind"], "internal");
        assert_eq!(nodes[0]["crate_name"], "my_crate");
        assert!(nodes[0].get("external_kind").is_none());
    }

    // 3. Serializar grafo com 1 nó externo (crate)
    #[test]
    fn test_serialize_external_crate() {
        let mut graph = DependencyGraph::new();
        graph.add_external_node("serde::de".to_string(), ExternalKind::Crate);
        let v = serialize_to_value(&graph, &empty_cycles(), &empty_workspace());
        let nodes = v["graph"]["nodes"].as_array().unwrap();
        assert_eq!(nodes[0]["kind"], "external");
        assert_eq!(nodes[0]["external_kind"], "crate");
        assert!(nodes[0].get("crate_name").is_none());
    }

    // 4. Serializar grafo com 1 nó externo (stdlib)
    #[test]
    fn test_serialize_external_stdlib() {
        let mut graph = DependencyGraph::new();
        graph.add_external_node("std::collections".to_string(), ExternalKind::Stdlib);
        let v = serialize_to_value(&graph, &empty_cycles(), &empty_workspace());
        let nodes = v["graph"]["nodes"].as_array().unwrap();
        assert_eq!(nodes[0]["external_kind"], "stdlib");
    }

    // 5. Serializar grafo com 1 aresta
    #[test]
    fn test_serialize_one_edge() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node_without_tree("A".into(), "c".into());
        let b = graph.add_internal_node_without_tree("B".into(), "c".into());
        graph.add_edge(a, b, simple_edge("Foo", "A::Foo")).unwrap();
        let v = serialize_to_value(&graph, &empty_cycles(), &empty_workspace());
        let edges = v["graph"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0]["from"], "A");
        assert_eq!(edges[0]["to"], "B");
        assert_eq!(edges[0]["imported_item"], "Foo");
        assert_eq!(edges[0]["raw_use_path"], "A::Foo");
    }

    // 6. Serializar aresta com alias None — campo "alias" deve aparecer como null
    #[test]
    fn test_serialize_edge_alias_none_appears_as_null() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node_without_tree("A".into(), "c".into());
        let b = graph.add_internal_node_without_tree("B".into(), "c".into());
        graph.add_edge(a, b, simple_edge("X", "A::X")).unwrap();
        let v = serialize_to_value(&graph, &empty_cycles(), &empty_workspace());
        let edge = &v["graph"]["edges"][0];
        assert!(edge.get("alias").is_some(), "campo alias deve existir");
        assert!(edge["alias"].is_null(), "alias deve ser null quando None");
    }

    // 7. Serializar aresta com alias Some
    #[test]
    fn test_serialize_edge_alias_some() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node_without_tree("A".into(), "c".into());
        let b = graph.add_internal_node_without_tree("B".into(), "c".into());
        let mut edge = simple_edge("Foo", "A::Foo as Bar");
        edge.alias = Some("Bar".to_string());
        graph.add_edge(a, b, edge).unwrap();
        let v = serialize_to_value(&graph, &empty_cycles(), &empty_workspace());
        assert_eq!(v["graph"]["edges"][0]["alias"], "Bar");
    }

    // 8. Serializar aresta glob
    #[test]
    fn test_serialize_edge_glob() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node_without_tree("A".into(), "c".into());
        let b = graph.add_internal_node_without_tree("B".into(), "c".into());
        let mut edge = simple_edge("*", "A::*");
        edge.is_glob = true;
        graph.add_edge(a, b, edge).unwrap();
        let v = serialize_to_value(&graph, &empty_cycles(), &empty_workspace());
        assert_eq!(v["graph"]["edges"][0]["is_glob"], true);
        assert_eq!(v["graph"]["edges"][0]["imported_item"], "*");
    }

    // 9. Serializar aresta re-export
    #[test]
    fn test_serialize_edge_reexport() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node_without_tree("A".into(), "c".into());
        let b = graph.add_internal_node_without_tree("B".into(), "c".into());
        let mut edge = simple_edge("Foo", "A::Foo");
        edge.is_reexport = true;
        graph.add_edge(a, b, edge).unwrap();
        let v = serialize_to_value(&graph, &empty_cycles(), &empty_workspace());
        assert_eq!(v["graph"]["edges"][0]["is_reexport"], true);
    }

    // 10. Ordenação alfabética de nós
    #[test]
    fn test_nodes_sorted_alphabetically() {
        let mut graph = DependencyGraph::new();
        graph.add_internal_node_without_tree("z::z".into(), "z".into());
        graph.add_internal_node_without_tree("a::a".into(), "a".into());
        graph.add_internal_node_without_tree("m::m".into(), "m".into());
        let json = to_canonical_json(
            &graph,
            &empty_cycles(),
            &empty_workspace(),
            TOOL_VERSION,
            GENERATED_AT,
        )
        .unwrap();
        let dto: GraphJsonDto = serde_json::from_str(&json).unwrap();
        let paths: Vec<&str> = dto
            .graph
            .nodes
            .iter()
            .map(|n| n.canonical_path.as_str())
            .collect();
        assert_eq!(paths, vec!["a::a", "m::m", "z::z"]);
    }

    // 11. Ordenação de arestas
    #[test]
    fn test_edges_sorted_canonically() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node_without_tree("A".into(), "c".into());
        let b = graph.add_internal_node_without_tree("B".into(), "c".into());
        let z = graph.add_internal_node_without_tree("Z".into(), "c".into());
        graph.add_edge(z, a, simple_edge("X", "Z::X")).unwrap();
        graph.add_edge(a, b, simple_edge("Y", "A::Y")).unwrap();
        graph.add_edge(a, b, simple_edge("X", "A::X")).unwrap();
        let json = to_canonical_json(
            &graph,
            &empty_cycles(),
            &empty_workspace(),
            TOOL_VERSION,
            GENERATED_AT,
        )
        .unwrap();
        let dto: GraphJsonDto = serde_json::from_str(&json).unwrap();
        let keys: Vec<(String, String, String)> = dto
            .graph
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone(), e.imported_item.clone()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("A".into(), "B".into(), "X".into()),
                ("A".into(), "B".into(), "Y".into()),
                ("Z".into(), "A".into(), "X".into()),
            ]
        );
    }

    // 12. Ciclos no JSON
    #[test]
    fn test_serialize_cycles() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node_without_tree("A".into(), "c".into());
        let b = graph.add_internal_node_without_tree("B".into(), "c".into());
        let cycles = CycleReport {
            cycles: vec![Cycle {
                nodes: vec![a, b],
                kind: CycleKind::MultiNode,
            }],
        };
        let v = serialize_to_value(&graph, &cycles, &empty_workspace());
        assert_eq!(v["cycles"]["count"], 1);
        assert_eq!(v["cycles"]["multi_node_count"], 1);
        assert_eq!(v["cycles"]["self_loop_count"], 0);
        let items = v["cycles"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["kind"], "multi_node");
        let nodes = items[0]["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);
        // Ordem alfabética dentro do ciclo
        assert_eq!(nodes[0], "A");
        assert_eq!(nodes[1], "B");
    }

    // 13. Metadados
    #[test]
    fn test_metadata_in_json() {
        let graph = DependencyGraph::new();
        let workspace = workspace_with_members(&["beta", "alpha"]);
        let json = to_canonical_json(
            &graph,
            &empty_cycles(),
            &workspace,
            TOOL_VERSION,
            GENERATED_AT,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["schema_version"], "1.0.0");
        assert_eq!(v["tool"]["name"], "crystalline-dsm");
        assert_eq!(v["tool"]["version"], TOOL_VERSION);
        assert_eq!(v["generated_at"], GENERATED_AT);
        // members ordenados alfabeticamente
        let members = v["workspace"]["members"].as_array().unwrap();
        assert_eq!(members[0], "alpha");
        assert_eq!(members[1], "beta");
    }

    // 14. Round-trip mínimo (grafo vazio)
    #[test]
    fn test_roundtrip_empty() {
        let graph = DependencyGraph::new();
        let json = to_canonical_json(
            &graph,
            &empty_cycles(),
            &empty_workspace(),
            TOOL_VERSION,
            GENERATED_AT,
        )
        .unwrap();
        let (g2, c2, _dto) = from_canonical_json(&json).unwrap();
        assert_eq!(g2.node_count(), 0);
        assert_eq!(g2.edge_count(), 0);
        assert_eq!(c2.cycle_count(), 0);
    }

    // 15. Round-trip com nós
    #[test]
    fn test_roundtrip_with_nodes() {
        let mut graph = DependencyGraph::new();
        graph.add_internal_node_without_tree("c1::a".into(), "c1".into());
        graph.add_internal_node_without_tree("c1::b".into(), "c1".into());
        graph.add_internal_node_without_tree("c2::x".into(), "c2".into());
        graph.add_external_node("serde".into(), ExternalKind::Crate);
        graph.add_external_node("std::fmt".into(), ExternalKind::Stdlib);

        let json = to_canonical_json(
            &graph,
            &empty_cycles(),
            &empty_workspace(),
            TOOL_VERSION,
            GENERATED_AT,
        )
        .unwrap();
        let (g2, _, _) = from_canonical_json(&json).unwrap();

        assert_eq!(g2.node_count(), 5);
        assert_eq!(g2.internal_node_count(), 3);
        assert_eq!(g2.external_node_count(), 2);
        assert!(g2.find_node("c1::a").is_some());
        assert!(g2.find_node("c1::b").is_some());
        assert!(g2.find_node("c2::x").is_some());
        assert!(g2.find_node("serde").is_some());
        assert!(g2.find_node("std::fmt").is_some());
    }

    // 16. Round-trip com arestas
    #[test]
    fn test_roundtrip_with_edges() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node_without_tree("A".into(), "c".into());
        let b = graph.add_internal_node_without_tree("B".into(), "c".into());
        let c = graph.add_internal_node_without_tree("C".into(), "c".into());

        let e_plain = simple_edge("Foo", "A::Foo");
        let mut e_alias = simple_edge("Bar", "A::Bar as Baz");
        e_alias.alias = Some("Baz".into());
        let mut e_glob = simple_edge("*", "A::*");
        e_glob.is_glob = true;
        let mut e_re = simple_edge("Z", "B::Z");
        e_re.is_reexport = true;
        let e_normal = simple_edge("W", "B::W");

        graph.add_edge(a, b, e_plain.clone()).unwrap();
        graph.add_edge(a, b, e_alias.clone()).unwrap();
        graph.add_edge(a, c, e_glob.clone()).unwrap();
        graph.add_edge(b, c, e_re.clone()).unwrap();
        graph.add_edge(b, c, e_normal.clone()).unwrap();

        let json = to_canonical_json(
            &graph,
            &empty_cycles(),
            &empty_workspace(),
            TOOL_VERSION,
            GENERATED_AT,
        )
        .unwrap();
        let (g2, _, _) = from_canonical_json(&json).unwrap();

        assert_eq!(g2.edge_count(), 5);

        // Para cada aresta original, encontrar uma equivalente em g2 por
        // (from, to, imported_item) — não dependemos de ordem.
        let edges_g2: Vec<_> = g2
            .all_edges()
            .map(|(f, t, e)| {
                (
                    g2.node(f).canonical_path.clone(),
                    g2.node(t).canonical_path.clone(),
                    e.clone(),
                )
            })
            .collect();

        let expected = vec![
            ("A".to_string(), "B".to_string(), e_plain),
            ("A".to_string(), "B".to_string(), e_alias),
            ("A".to_string(), "C".to_string(), e_glob),
            ("B".to_string(), "C".to_string(), e_re),
            ("B".to_string(), "C".to_string(), e_normal),
        ];
        for (f, t, e) in expected {
            assert!(
                edges_g2
                    .iter()
                    .any(|(f2, t2, e2)| f2 == &f && t2 == &t && e2 == &e),
                "aresta {} → {} ({:?}) não encontrada",
                f,
                t,
                e.imported_item
            );
        }
    }

    // 17. Round-trip com ciclos
    #[test]
    fn test_roundtrip_with_cycles() {
        let mut graph = DependencyGraph::new();
        let a = graph.add_internal_node_without_tree("A".into(), "c".into());
        let b = graph.add_internal_node_without_tree("B".into(), "c".into());
        let c = graph.add_internal_node_without_tree("C".into(), "c".into());
        let cycles = CycleReport {
            cycles: vec![
                Cycle {
                    nodes: vec![a, b],
                    kind: CycleKind::MultiNode,
                },
                Cycle {
                    nodes: vec![c],
                    kind: CycleKind::SelfLoop,
                },
            ],
        };

        let json = to_canonical_json(
            &graph,
            &cycles,
            &empty_workspace(),
            TOOL_VERSION,
            GENERATED_AT,
        )
        .unwrap();
        let (_, c2, _) = from_canonical_json(&json).unwrap();

        assert_eq!(c2.cycle_count(), 2);
        assert_eq!(c2.self_loop_count(), 1);
        assert_eq!(c2.multi_node_cycle_count(), 1);
    }

    // 18. Round-trip de cenário complexo (50 nós, ~150 arestas, alguns
    // ciclos) — proxy local do "round-trip de JSON real do Typst".
    #[test]
    fn test_roundtrip_complex_scenario() {
        let mut graph = DependencyGraph::new();
        let mut ids = Vec::new();
        for i in 0..50 {
            let path = format!("crate_{}::mod_{}", i / 10, i);
            ids.push(graph.add_internal_node_without_tree(path, format!("crate_{}", i / 10)));
        }
        // Cadeia + cross-edges
        for i in 0..49 {
            graph
                .add_edge(
                    ids[i],
                    ids[i + 1],
                    simple_edge(&format!("Item{}", i), &format!("p{}", i)),
                )
                .unwrap();
        }
        for i in 0..30 {
            graph
                .add_edge(
                    ids[i],
                    ids[(i + 7) % 50],
                    simple_edge(&format!("Cross{}", i), &format!("c{}", i)),
                )
                .unwrap();
        }
        // Externos com arestas
        let ext1 = graph.add_external_node("serde".into(), ExternalKind::Crate);
        let ext2 = graph.add_external_node("std::fmt".into(), ExternalKind::Stdlib);
        graph
            .add_edge(ids[0], ext1, simple_edge("S", "serde::S"))
            .unwrap();
        graph
            .add_edge(ids[1], ext2, simple_edge("Debug", "std::fmt::Debug"))
            .unwrap();

        // Ciclo construído explicitamente
        graph
            .add_edge(ids[5], ids[0], simple_edge("Back", "back"))
            .unwrap();
        let cycles = CycleReport {
            cycles: vec![Cycle {
                nodes: vec![ids[0], ids[1], ids[2], ids[3], ids[4], ids[5]],
                kind: CycleKind::MultiNode,
            }],
        };

        let ws = workspace_with_members(&["crate_0", "crate_1", "crate_2", "crate_3", "crate_4"]);
        let json = to_canonical_json(&graph, &cycles, &ws, TOOL_VERSION, GENERATED_AT).unwrap();
        let (g2, c2, dto) = from_canonical_json(&json).unwrap();

        assert_eq!(g2.node_count(), graph.node_count());
        assert_eq!(g2.edge_count(), graph.edge_count());
        assert_eq!(c2.cycle_count(), 1);
        assert_eq!(dto.workspace.members.len(), 5);
        // Round-trip JSON → JSON é estável
        let json2 = to_canonical_json(&g2, &c2, &ws, TOOL_VERSION, GENERATED_AT).unwrap();
        assert_eq!(
            json, json2,
            "serialização deve ser determinística entre rodadas"
        );
    }

    // 19. Schema incompatível
    #[test]
    fn test_incompatible_schema_version() {
        let json = r#"{
            "schema_version": "2.0.0",
            "generated_at": "x",
            "tool": {"name": "x", "version": "x"},
            "workspace": {"root": "x", "members": []},
            "graph": {"nodes": [], "edges": []},
            "cycles": {"count": 0, "self_loop_count": 0, "multi_node_count": 0, "items": []}
        }"#;
        let result = from_canonical_json(json);
        assert!(matches!(
            result,
            Err(JsonDeserializeError::IncompatibleSchemaVersion { .. })
        ));
    }

    // 19b — versão minor diferente (1.5.0) é aceite silenciosamente
    #[test]
    fn test_compatible_minor_version_accepted() {
        let json = r#"{
            "schema_version": "1.5.0",
            "generated_at": "x",
            "tool": {"name": "x", "version": "x"},
            "workspace": {"root": "x", "members": []},
            "graph": {"nodes": [], "edges": []},
            "cycles": {"count": 0, "self_loop_count": 0, "multi_node_count": 0, "items": []}
        }"#;
        assert!(from_canonical_json(json).is_ok());
    }

    // 20. Aresta com referência inválida
    #[test]
    fn test_dangling_edge_reference() {
        let json = r#"{
            "schema_version": "1.0.0",
            "generated_at": "x",
            "tool": {"name": "x", "version": "x"},
            "workspace": {"root": "x", "members": []},
            "graph": {
                "nodes": [
                    {"canonical_path": "A", "kind": "internal", "crate_name": "c"}
                ],
                "edges": [
                    {"from": "A", "to": "GHOST", "imported_item": "X",
                     "alias": null, "is_reexport": false, "is_glob": false,
                     "raw_use_path": "A::X"}
                ]
            },
            "cycles": {"count": 0, "self_loop_count": 0, "multi_node_count": 0, "items": []}
        }"#;
        let result = from_canonical_json(json);
        assert!(matches!(
            result,
            Err(JsonDeserializeError::DanglingEdgeReference { canonical_path }) if canonical_path == "GHOST"
        ));
    }

    // 21. Ciclo com referência inválida
    #[test]
    fn test_dangling_cycle_reference() {
        let json = r#"{
            "schema_version": "1.0.0",
            "generated_at": "x",
            "tool": {"name": "x", "version": "x"},
            "workspace": {"root": "x", "members": []},
            "graph": {
                "nodes": [
                    {"canonical_path": "A", "kind": "internal", "crate_name": "c"}
                ],
                "edges": []
            },
            "cycles": {
                "count": 1,
                "self_loop_count": 0,
                "multi_node_count": 1,
                "items": [
                    {"kind": "multi_node", "nodes": ["A", "GHOST"]}
                ]
            }
        }"#;
        let result = from_canonical_json(json);
        assert!(matches!(
            result,
            Err(JsonDeserializeError::DanglingCycleReference { canonical_path }) if canonical_path == "GHOST"
        ));
    }

    // 22. JSON malformado
    #[test]
    fn test_malformed_json() {
        let result = from_canonical_json("{ this is not json");
        assert!(matches!(
            result,
            Err(JsonDeserializeError::SerdeError { .. })
        ));
    }
}
