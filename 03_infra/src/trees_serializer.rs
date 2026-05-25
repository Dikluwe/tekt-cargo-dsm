/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/trees_serializer.md
 * @layer L3
 * @updated 2026-05-20
 */

use crate::json_serializer::{TOOL_NAME, ToolInfoDto, WorkspaceInfoDto};
use crystalline_dsm_core::entities::module_tree::{ModuleTree, TreeError};
use crystalline_dsm_core::entities::workspace::Workspace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const TREES_SCHEMA_VERSION: &str = "1.0.0";

// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreesJsonDto {
    pub schema_version: String,
    pub generated_at: String,
    pub tool: ToolInfoDto,
    pub workspace: WorkspaceInfoDto,
    pub trees: Vec<ModuleTreeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleTreeDto {
    pub crate_name: String,
    /// Nós em pre-order (raiz primeiro, depois filhos recursivamente).
    /// Garantia: o 1º elemento é sempre a raiz.
    pub nodes: Vec<ModuleNodeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleNodeDto {
    pub canonical_path: String,
    pub crate_name: String,
    pub module_path: Vec<String>,
    /// Serializado como `String` (não `PathBuf`) por portabilidade.
    pub source_file: String,
    pub is_inline: bool,
    pub has_custom_path: bool,
    /// Caminho canónico do pai. `None` para a raiz.
    pub parent_canonical_path: Option<String>,
}

// ============================================================================
// Erros
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum TreesSerializeError {
    #[error("Falha ao serializar para JSON: {source}")]
    SerdeError {
        #[from]
        source: serde_json::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum TreesDeserializeError {
    #[error("Falha ao parsear JSON: {source}")]
    SerdeError {
        #[from]
        source: serde_json::Error,
    },

    #[error("Versão de schema incompatível: esperado 1.x.y, recebido {version}")]
    IncompatibleSchemaVersion { version: String },

    #[error("Pai referenciado não existe: {parent_canonical_path}")]
    DanglingParentReference { parent_canonical_path: String },

    #[error("Erro ao reconstruir árvore: {source}")]
    TreeReconstructionError {
        #[from]
        source: TreeError,
    },

    #[error("Árvore '{crate_name}' sem nó raiz (lista de nós vazia)")]
    EmptyTree { crate_name: String },
}

// ============================================================================
// Conversão Domínio → DTO
// ============================================================================

pub(crate) fn to_dto_trees(
    trees: &HashMap<String, ModuleTree>,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
) -> TreesJsonDto {
    let mut crate_names: Vec<&String> = trees.keys().collect();
    crate_names.sort();

    let trees_dto: Vec<ModuleTreeDto> = crate_names
        .into_iter()
        .map(|name| {
            let tree = &trees[name];
            let nodes: Vec<ModuleNodeDto> = tree
                .iter_preorder()
                .map(|(id, node)| {
                    let parent_canonical = tree
                        .parent(id)
                        .map(|pid| tree.node(pid).canonical_path.clone());
                    ModuleNodeDto {
                        canonical_path: node.canonical_path.clone(),
                        crate_name: node.crate_name.clone(),
                        module_path: node.module_path.clone(),
                        source_file: node.source_file.to_string_lossy().into_owned(),
                        is_inline: node.is_inline,
                        has_custom_path: node.has_custom_path,
                        parent_canonical_path: parent_canonical,
                    }
                })
                .collect();
            ModuleTreeDto {
                crate_name: name.clone(),
                nodes,
            }
        })
        .collect();

    let mut members: Vec<String> = workspace.members.iter().map(|m| m.name.clone()).collect();
    members.sort();

    TreesJsonDto {
        schema_version: TREES_SCHEMA_VERSION.to_string(),
        generated_at: generated_at.to_string(),
        tool: ToolInfoDto {
            name: TOOL_NAME.to_string(),
            version: tool_version.to_string(),
        },
        workspace: WorkspaceInfoDto {
            root: workspace.root.to_string_lossy().into_owned(),
            members,
        },
        trees: trees_dto,
    }
}

// ============================================================================
// Conversão DTO → Domínio
// ============================================================================

pub(crate) fn from_dto_trees(
    dto: TreesJsonDto,
) -> Result<HashMap<String, ModuleTree>, TreesDeserializeError> {
    if !is_compatible_schema_version(&dto.schema_version) {
        return Err(TreesDeserializeError::IncompatibleSchemaVersion {
            version: dto.schema_version,
        });
    }

    let mut out = HashMap::new();
    for tree_dto in dto.trees {
        if tree_dto.nodes.is_empty() {
            return Err(TreesDeserializeError::EmptyTree {
                crate_name: tree_dto.crate_name,
            });
        }

        // 1º nó é a raiz (garantia da serialização em pre-order).
        let root_dto = &tree_dto.nodes[0];
        let mut tree = ModuleTree::new(
            tree_dto.crate_name.clone(),
            PathBuf::from(&root_dto.source_file),
        );

        // Demais nós: parent já deve estar inserido (ordem pre-order).
        for node_dto in tree_dto.nodes.iter().skip(1) {
            let parent_canonical = node_dto.parent_canonical_path.as_ref().ok_or_else(|| {
                TreesDeserializeError::DanglingParentReference {
                    parent_canonical_path: "<None em nó não-raiz>".to_string(),
                }
            })?;
            let parent_id = tree
                .find_by_canonical_path(parent_canonical)
                .ok_or_else(|| TreesDeserializeError::DanglingParentReference {
                    parent_canonical_path: parent_canonical.clone(),
                })?;
            let module_name = node_dto
                .module_path
                .last()
                .cloned()
                .unwrap_or_else(|| node_dto.canonical_path.clone());
            tree.add_child(
                parent_id,
                module_name,
                PathBuf::from(&node_dto.source_file),
                node_dto.is_inline,
                node_dto.has_custom_path,
            )?;
        }

        out.insert(tree_dto.crate_name, tree);
    }

    Ok(out)
}

fn is_compatible_schema_version(version: &str) -> bool {
    matches!(version.split('.').next(), Some("1"))
}

// ============================================================================
// API pública: JSON
// ============================================================================

pub fn to_canonical_json_trees(
    trees: &HashMap<String, ModuleTree>,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
) -> Result<String, TreesSerializeError> {
    let dto = to_dto_trees(trees, workspace, tool_version, generated_at);
    Ok(serde_json::to_string_pretty(&dto)?)
}

pub fn from_canonical_json_trees(
    json: &str,
) -> Result<(HashMap<String, ModuleTree>, TreesJsonDto), TreesDeserializeError> {
    let dto: TreesJsonDto = serde_json::from_str(json)?;
    let dto_clone = dto.clone();
    let trees = from_dto_trees(dto)?;
    Ok((trees, dto_clone))
}

// ============================================================================
// Testes
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crystalline_dsm_core::entities::workspace::{EntryKind, Workspace, WorkspaceMember};

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

    fn nested_tree() -> ModuleTree {
        nested_tree_named("my_crate")
    }

    fn nested_tree_named(name: &str) -> ModuleTree {
        // root → a → b
        let mut t = ModuleTree::new(name.to_string(), PathBuf::from("src/lib.rs"));
        let root = t.root();
        let a = t
            .add_child(
                root,
                "a".to_string(),
                PathBuf::from("src/a.rs"),
                false,
                false,
            )
            .unwrap();
        t.add_child(
            a,
            "b".to_string(),
            PathBuf::from("src/a/b.rs"),
            false,
            false,
        )
        .unwrap();
        t
    }

    fn serialize(trees: &HashMap<String, ModuleTree>, ws: &Workspace) -> serde_json::Value {
        let json = to_canonical_json_trees(trees, ws, TOOL_VERSION, GENERATED_AT).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    // 1. Serializar HashMap vazio
    #[test]
    fn test_serialize_empty_map() {
        let trees: HashMap<String, ModuleTree> = HashMap::new();
        let v = serialize(&trees, &empty_workspace());
        assert_eq!(v["trees"].as_array().unwrap().len(), 0);
    }

    // 2. Serializar uma árvore com só raiz
    #[test]
    fn test_serialize_one_tree_root_only() {
        let mut trees = HashMap::new();
        trees.insert(
            "my_crate".to_string(),
            ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs")),
        );
        let v = serialize(&trees, &empty_workspace());
        let arr = v["trees"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        let nodes = arr[0]["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0]["canonical_path"], "my_crate");
        assert!(nodes[0]["parent_canonical_path"].is_null());
    }

    // 3. Serializar árvore aninhada (3 níveis) — verifica ordem pre-order
    #[test]
    fn test_serialize_nested_tree_preorder() {
        let mut trees = HashMap::new();
        trees.insert("my_crate".to_string(), nested_tree());
        let v = serialize(&trees, &empty_workspace());
        let nodes = v["trees"][0]["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0]["canonical_path"], "my_crate");
        assert_eq!(nodes[1]["canonical_path"], "my_crate::a");
        assert_eq!(nodes[2]["canonical_path"], "my_crate::a::b");
        assert_eq!(nodes[1]["parent_canonical_path"], "my_crate");
        assert_eq!(nodes[2]["parent_canonical_path"], "my_crate::a");
    }

    // 4. Serializar múltiplas árvores em ordem alfabética
    #[test]
    fn test_serialize_multiple_trees_alphabetical() {
        let mut trees = HashMap::new();
        trees.insert(
            "zeta".to_string(),
            ModuleTree::new("zeta".to_string(), PathBuf::from("z/src/lib.rs")),
        );
        trees.insert(
            "alpha".to_string(),
            ModuleTree::new("alpha".to_string(), PathBuf::from("a/src/lib.rs")),
        );
        trees.insert(
            "mu".to_string(),
            ModuleTree::new("mu".to_string(), PathBuf::from("m/src/lib.rs")),
        );
        let v = serialize(&trees, &empty_workspace());
        let arr = v["trees"].as_array().unwrap();
        let names: Vec<&str> = arr
            .iter()
            .map(|t| t["crate_name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["alpha", "mu", "zeta"]);
    }

    // 5. Serializar módulo inline
    #[test]
    fn test_serialize_inline_module() {
        let mut t = ModuleTree::new("c".to_string(), PathBuf::from("src/lib.rs"));
        let root = t.root();
        t.add_child(
            root,
            "inline".to_string(),
            PathBuf::from("src/lib.rs"),
            true,
            false,
        )
        .unwrap();
        let mut trees = HashMap::new();
        trees.insert("c".to_string(), t);
        let v = serialize(&trees, &empty_workspace());
        let inline_node = &v["trees"][0]["nodes"][1];
        assert_eq!(inline_node["is_inline"], true);
        assert_eq!(inline_node["source_file"], "src/lib.rs");
    }

    // 6. Serializar módulo com #[path]
    #[test]
    fn test_serialize_custom_path_module() {
        let mut t = ModuleTree::new("c".to_string(), PathBuf::from("src/lib.rs"));
        let root = t.root();
        t.add_child(
            root,
            "x".to_string(),
            PathBuf::from("src/custom/special.rs"),
            false,
            true,
        )
        .unwrap();
        let mut trees = HashMap::new();
        trees.insert("c".to_string(), t);
        let v = serialize(&trees, &empty_workspace());
        let node = &v["trees"][0]["nodes"][1];
        assert_eq!(node["has_custom_path"], true);
        assert_eq!(node["source_file"], "src/custom/special.rs");
    }

    // 7. Metadados
    #[test]
    fn test_metadata_in_json() {
        let trees = HashMap::new();
        let ws = workspace_with_members(&["beta", "alpha"]);
        let json = to_canonical_json_trees(&trees, &ws, TOOL_VERSION, GENERATED_AT).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["schema_version"], "1.0.0");
        assert_eq!(v["tool"]["name"], "crystalline-dsm");
        assert_eq!(v["tool"]["version"], TOOL_VERSION);
        assert_eq!(v["generated_at"], GENERATED_AT);
        let members = v["workspace"]["members"].as_array().unwrap();
        assert_eq!(members[0], "alpha");
        assert_eq!(members[1], "beta");
    }

    // 8. Round-trip vazio
    #[test]
    fn test_roundtrip_empty() {
        let trees: HashMap<String, ModuleTree> = HashMap::new();
        let json = to_canonical_json_trees(&trees, &empty_workspace(), TOOL_VERSION, GENERATED_AT)
            .unwrap();
        let (back, _dto) = from_canonical_json_trees(&json).unwrap();
        assert!(back.is_empty());
    }

    // 9. Round-trip uma árvore só raiz
    #[test]
    fn test_roundtrip_root_only() {
        let mut trees = HashMap::new();
        trees.insert(
            "x".to_string(),
            ModuleTree::new("x".to_string(), PathBuf::from("src/lib.rs")),
        );
        let json = to_canonical_json_trees(&trees, &empty_workspace(), TOOL_VERSION, GENERATED_AT)
            .unwrap();
        let (back, _) = from_canonical_json_trees(&json).unwrap();
        let tree = back.get("x").expect("crate x deveria existir");
        assert_eq!(tree.node_count(), 1);
        assert_eq!(tree.node(tree.root()).canonical_path, "x");
    }

    // 10. Round-trip árvore aninhada
    #[test]
    fn test_roundtrip_nested_tree() {
        let mut trees = HashMap::new();
        trees.insert("my_crate".to_string(), nested_tree());
        let json = to_canonical_json_trees(&trees, &empty_workspace(), TOOL_VERSION, GENERATED_AT)
            .unwrap();
        let (back, _) = from_canonical_json_trees(&json).unwrap();
        let t = back.get("my_crate").unwrap();
        assert_eq!(t.node_count(), 3);
        let a = t
            .find_by_canonical_path("my_crate::a")
            .expect("nó a deveria existir");
        let b = t
            .find_by_canonical_path("my_crate::a::b")
            .expect("nó b deveria existir");
        assert_eq!(t.parent(a), Some(t.root()));
        assert_eq!(t.parent(b), Some(a));
    }

    // 11. Round-trip múltiplas árvores
    #[test]
    fn test_roundtrip_multiple_trees() {
        let mut trees = HashMap::new();
        trees.insert("alpha".to_string(), nested_tree_named("alpha"));
        let mut t_beta = ModuleTree::new("beta".to_string(), PathBuf::from("b/src/lib.rs"));
        let bs_root = t_beta.root();
        t_beta
            .add_child(
                bs_root,
                "only".to_string(),
                PathBuf::from("b/src/only.rs"),
                false,
                false,
            )
            .unwrap();
        trees.insert("beta".to_string(), t_beta);
        trees.insert(
            "gamma".to_string(),
            ModuleTree::new("gamma".to_string(), PathBuf::from("g/src/lib.rs")),
        );

        let json = to_canonical_json_trees(&trees, &empty_workspace(), TOOL_VERSION, GENERATED_AT)
            .unwrap();
        let (back, _) = from_canonical_json_trees(&json).unwrap();
        assert_eq!(back.len(), 3);
        assert_eq!(back["alpha"].node_count(), 3);
        assert_eq!(back["beta"].node_count(), 2);
        assert_eq!(back["gamma"].node_count(), 1);
    }

    // 12. Round-trip preserva is_inline
    #[test]
    fn test_roundtrip_preserves_inline() {
        let mut t = ModuleTree::new("c".to_string(), PathBuf::from("src/lib.rs"));
        let root = t.root();
        t.add_child(
            root,
            "inline".to_string(),
            PathBuf::from("src/lib.rs"),
            true,
            false,
        )
        .unwrap();
        t.add_child(
            root,
            "ext".to_string(),
            PathBuf::from("src/ext.rs"),
            false,
            false,
        )
        .unwrap();
        let mut trees = HashMap::new();
        trees.insert("c".to_string(), t);

        let json = to_canonical_json_trees(&trees, &empty_workspace(), TOOL_VERSION, GENERATED_AT)
            .unwrap();
        let (back, _) = from_canonical_json_trees(&json).unwrap();
        let t2 = &back["c"];
        let inline_id = t2.find_by_canonical_path("c::inline").unwrap();
        assert!(t2.node(inline_id).is_inline);
        let ext_id = t2.find_by_canonical_path("c::ext").unwrap();
        assert!(!t2.node(ext_id).is_inline);
    }

    // 13. Inversão da ordem gera DanglingParentReference
    #[test]
    fn test_inverted_order_yields_dangling_parent() {
        // Construir DTO manual com root + neto + filho (neto antes do filho)
        let dto = TreesJsonDto {
            schema_version: "1.0.0".into(),
            generated_at: GENERATED_AT.into(),
            tool: ToolInfoDto {
                name: TOOL_NAME.into(),
                version: TOOL_VERSION.into(),
            },
            workspace: WorkspaceInfoDto {
                root: "/x".into(),
                members: vec![],
            },
            trees: vec![ModuleTreeDto {
                crate_name: "c".into(),
                nodes: vec![
                    ModuleNodeDto {
                        canonical_path: "c".into(),
                        crate_name: "c".into(),
                        module_path: vec![],
                        source_file: "src/lib.rs".into(),
                        is_inline: false,
                        has_custom_path: false,
                        parent_canonical_path: None,
                    },
                    // Neto antes do filho — pai "c::a" ainda não existe
                    ModuleNodeDto {
                        canonical_path: "c::a::b".into(),
                        crate_name: "c".into(),
                        module_path: vec!["a".into(), "b".into()],
                        source_file: "src/a/b.rs".into(),
                        is_inline: false,
                        has_custom_path: false,
                        parent_canonical_path: Some("c::a".into()),
                    },
                ],
            }],
        };
        let json = serde_json::to_string(&dto).unwrap();
        let result = from_canonical_json_trees(&json);
        assert!(matches!(
            result,
            Err(TreesDeserializeError::DanglingParentReference { parent_canonical_path }) if parent_canonical_path == "c::a"
        ));
    }

    // 14. Schema incompatível
    #[test]
    fn test_incompatible_schema_version() {
        let json = r#"{
            "schema_version": "2.0.0",
            "generated_at": "x",
            "tool": {"name": "x", "version": "x"},
            "workspace": {"root": "x", "members": []},
            "trees": []
        }"#;
        let result = from_canonical_json_trees(json);
        assert!(matches!(
            result,
            Err(TreesDeserializeError::IncompatibleSchemaVersion { .. })
        ));
    }

    // 15. Pai inexistente
    #[test]
    fn test_dangling_parent() {
        let json = r#"{
            "schema_version": "1.0.0",
            "generated_at": "x",
            "tool": {"name": "x", "version": "x"},
            "workspace": {"root": "x", "members": []},
            "trees": [{
                "crate_name": "c",
                "nodes": [
                    {"canonical_path": "c", "crate_name": "c", "module_path": [],
                     "source_file": "src/lib.rs", "is_inline": false,
                     "has_custom_path": false, "parent_canonical_path": null},
                    {"canonical_path": "c::x", "crate_name": "c", "module_path": ["x"],
                     "source_file": "src/x.rs", "is_inline": false,
                     "has_custom_path": false, "parent_canonical_path": "GHOST"}
                ]
            }]
        }"#;
        let result = from_canonical_json_trees(json);
        assert!(matches!(
            result,
            Err(TreesDeserializeError::DanglingParentReference { parent_canonical_path }) if parent_canonical_path == "GHOST"
        ));
    }

    // 16. Lista vazia em árvore
    #[test]
    fn test_empty_tree_error() {
        let json = r#"{
            "schema_version": "1.0.0",
            "generated_at": "x",
            "tool": {"name": "x", "version": "x"},
            "workspace": {"root": "x", "members": []},
            "trees": [{"crate_name": "c", "nodes": []}]
        }"#;
        let result = from_canonical_json_trees(json);
        assert!(matches!(
            result,
            Err(TreesDeserializeError::EmptyTree { crate_name }) if crate_name == "c"
        ));
    }

    // 17. JSON malformado
    #[test]
    fn test_malformed_json() {
        let result = from_canonical_json_trees("{ not valid");
        assert!(matches!(
            result,
            Err(TreesDeserializeError::SerdeError { .. })
        ));
    }

    // 18. Cross-reference com graph.json (integração leve)
    //
    // Gera um grafo + uma árvore para o mesmo crate, serializa ambos,
    // deserializa ambos, e verifica que para cada nó interno no grafo
    // reconstruído (que vira `InternalWithoutTree`) existe um
    // `ModuleNode` correspondente na árvore reconstruída, casado pelo
    // `canonical_path`. Ponte da ADR-0010.
    #[test]
    fn test_cross_reference_with_graph_json() {
        use crate::json_serializer::{from_canonical_json, to_canonical_json};
        use crystalline_dsm_core::entities::dependency_graph::{DependencyGraph, NodeKind};
        use crystalline_dsm_core::rules::cycle_detector::CycleReport;

        // Árvore: my_crate → utils
        let mut tree = ModuleTree::new("my_crate".to_string(), PathBuf::from("src/lib.rs"));
        let root = tree.root();
        let utils = tree
            .add_child(
                root,
                "utils".to_string(),
                PathBuf::from("src/utils.rs"),
                false,
                false,
            )
            .unwrap();
        let mut trees_map = HashMap::new();
        trees_map.insert("my_crate".to_string(), tree.clone());

        // Grafo com os mesmos 2 nós (with-tree)
        let mut graph = DependencyGraph::new();
        graph.add_internal_node_with_tree("my_crate".into(), "my_crate".into(), root);
        graph.add_internal_node_with_tree("my_crate::utils".into(), "my_crate".into(), utils);

        let ws = empty_workspace();
        let cycles = CycleReport { cycles: vec![] };

        let graph_json =
            to_canonical_json(&graph, &cycles, &ws, TOOL_VERSION, GENERATED_AT).unwrap();
        let trees_json =
            to_canonical_json_trees(&trees_map, &ws, TOOL_VERSION, GENERATED_AT).unwrap();

        let (g2, _, _) = from_canonical_json(&graph_json).unwrap();
        let (trees2, _) = from_canonical_json_trees(&trees_json).unwrap();

        let tree2 = trees2.get("my_crate").expect("árvore my_crate");

        for (_, n) in g2.internal_nodes() {
            // Após round-trip, todos os internos devem ser InternalWithoutTree
            assert!(matches!(n.kind, NodeKind::InternalWithoutTree { .. }));
            // E o canonical_path deve ter um match na árvore reconstruída
            assert!(
                tree2.find_by_canonical_path(&n.canonical_path).is_some(),
                "nó {} do grafo deveria ter correspondente na árvore",
                n.canonical_path,
            );
        }
    }
}
