/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/import_extractor.md
 * @layer L3
 * @updated 2026-05-20
 */

use crystalline_dsm_core::entities::import_edge::{ImportEdge, ImportKind};
use crystalline_dsm_core::entities::module_tree::{ModuleTree, NodeId};
use crystalline_dsm_core::entities::workspace::WorkspaceMember;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("Falha ao ler ficheiro: {path}")]
    FileReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Falha ao parsear ficheiro Rust: {file}")]
    ParseFailed {
        file: PathBuf,
        #[source]
        source: syn::Error,
    },
}

/// Classifica um import a partir do primeiro segmento do caminho normalizado.
fn classify_import_kind(
    first_segment: &str,
    from_crate: &str,
    workspace_crate_names: &[String],
) -> ImportKind {
    if first_segment == from_crate {
        ImportKind::CurrentCrate
    } else if workspace_crate_names.iter().any(|n| n == first_segment) {
        ImportKind::WorkspaceCrate
    } else if first_segment == "std" || first_segment == "core" || first_segment == "alloc" {
        ImportKind::Stdlib
    } else {
        ImportKind::External
    }
}

/// Resultado atômico da expansão de uma UseTree.
struct AtomicImport {
    /// Segmentos de caminho até o módulo (excluindo o item final).
    prefix_segments: Vec<String>,
    /// Item importado (nome, "*", ou self resolvido).
    item: String,
    /// Alias se houver.
    alias: Option<String>,
    /// Se é glob.
    is_glob: bool,
}

/// Expande recursivamente uma syn::UseTree em items atómicos.
fn expand_use_tree(tree: &syn::UseTree, prefix: &[String], output: &mut Vec<AtomicImport>) {
    match tree {
        syn::UseTree::Path(p) => {
            let mut new_prefix = prefix.to_vec();
            new_prefix.push(p.ident.to_string());
            expand_use_tree(&p.tree, &new_prefix, output);
        }
        syn::UseTree::Name(n) => {
            output.push(AtomicImport {
                prefix_segments: prefix.to_vec(),
                item: n.ident.to_string(),
                alias: None,
                is_glob: false,
            });
        }
        syn::UseTree::Rename(r) => {
            output.push(AtomicImport {
                prefix_segments: prefix.to_vec(),
                item: r.ident.to_string(),
                alias: Some(r.rename.to_string()),
                is_glob: false,
            });
        }
        syn::UseTree::Glob(_) => {
            output.push(AtomicImport {
                prefix_segments: prefix.to_vec(),
                item: "*".to_string(),
                alias: None,
                is_glob: true,
            });
        }
        syn::UseTree::Group(g) => {
            for sub in &g.items {
                expand_use_tree(sub, prefix, output);
            }
        }
    }
}

/// Resolve caminhos relativos (crate::, self::, super::) em caminhos canónicos.
fn resolve_relative_path(
    segments: &[String],
    item: &str,
    from_crate: &str,
    from_canonical_path: &str,
    tree: &ModuleTree,
    from_node: NodeId,
) -> (Vec<String>, String, ImportKind) {
    if segments.is_empty() {
        // Item simples sem path: ex `use Foo;`
        return (vec![], item.to_string(), ImportKind::External);
    }

    let first = &segments[0];

    if first == "crate" {
        // crate::a::b -> from_crate::a::b
        let mut resolved = vec![from_crate.to_string()];
        resolved.extend(segments[1..].iter().cloned());
        let kind = ImportKind::CurrentCrate;
        return (resolved, item.to_string(), kind);
    }

    if first == "self" {
        // self::a::b -> from_canonical_path::a::b
        let mut resolved: Vec<String> = from_canonical_path
            .split("::")
            .map(|s| s.to_string())
            .collect();
        resolved.extend(segments[1..].iter().cloned());
        let kind = ImportKind::CurrentCrate;
        return (resolved, item.to_string(), kind);
    }

    if first == "super" {
        // Conta quantos super:: temos no início
        let mut super_count = 0;
        for seg in segments {
            if seg == "super" {
                super_count += 1;
            } else {
                break;
            }
        }

        // Sobe na árvore o número correto de vezes
        let mut current = from_node;
        let mut resolved_ok = true;
        for _ in 0..super_count {
            if let Some(parent) = tree.parent(current) {
                current = parent;
            } else {
                // Saiu do crate
                eprintln!(
                    "Warning: 'super' resolveu para fora do crate em {}",
                    from_canonical_path
                );
                resolved_ok = false;
                break;
            }
        }

        if !resolved_ok {
            // Retorna Unresolved
            let mut all_segs = segments.to_vec();
            all_segs.push(item.to_string());
            return (all_segs, item.to_string(), ImportKind::Unresolved);
        }

        let parent_path = tree.node(current).canonical_path.clone();
        let mut resolved: Vec<String> = parent_path.split("::").map(|s| s.to_string()).collect();
        resolved.extend(segments[super_count..].iter().cloned());
        let kind = ImportKind::CurrentCrate;
        return (resolved, item.to_string(), kind);
    }

    // Caminho absoluto normal, classificar pelo primeiro segmento
    let kind = classify_import_kind(first, from_crate, &[]);
    (segments.to_vec(), item.to_string(), kind)
}

/// Determina se um item Use é um re-export (pub use).
fn is_reexport(vis: &syn::Visibility) -> bool {
    !matches!(vis, syn::Visibility::Inherited)
}

/// Processa os items de um nível do AST, extraindo imports e descendo em módulos inline.
#[allow(clippy::collapsible_if)]
fn extract_from_items(
    items: &[syn::Item],
    current_node: NodeId,
    current_canonical_path: &str,
    tree: &ModuleTree,
    from_crate: &str,
    workspace_crate_names: &[String],
    output: &mut Vec<ImportEdge>,
) {
    for item in items {
        match item {
            syn::Item::Use(item_use) => {
                let reexport = is_reexport(&item_use.vis);

                let mut atomics = Vec::new();
                expand_use_tree(&item_use.tree, &[], &mut atomics);

                for atomic in atomics {
                    let (resolved_segments, resolved_item, mut kind) = resolve_relative_path(
                        &atomic.prefix_segments,
                        &atomic.item,
                        from_crate,
                        current_canonical_path,
                        tree,
                        current_node,
                    );

                    // Se não resolvemos por relative path, classificar pelo primeiro segmento
                    if kind != ImportKind::CurrentCrate && kind != ImportKind::Unresolved {
                        if let Some(first) = resolved_segments.first() {
                            kind = classify_import_kind(first, from_crate, workspace_crate_names);
                        }
                    }

                    // Construir raw_use_path original
                    let mut raw_parts = atomic.prefix_segments.clone();
                    raw_parts.push(atomic.item.clone());
                    let raw_use_path = raw_parts.join("::");

                    // target_module: os segmentos de prefixo resolvidos
                    let target_module = resolved_segments.join("::");

                    // Para `use a::{self, X}`, self importa o módulo a em si
                    let imported_item = if atomic.item == "self" {
                        // O item é o próprio módulo (último do prefix)
                        atomic.prefix_segments.last().cloned().unwrap_or_default()
                    } else {
                        resolved_item
                    };

                    output.push(ImportEdge::new(
                        current_node,
                        from_crate.to_string(),
                        target_module,
                        imported_item,
                        kind,
                        raw_use_path,
                        atomic.is_glob,
                        atomic.alias,
                        reexport,
                    ));
                }
            }
            syn::Item::Mod(item_mod) => {
                // Descer em módulos inline para extrair imports com NodeId correcto
                if let Some((_, inline_items)) = &item_mod.content {
                    let mod_name = item_mod.ident.to_string();
                    let child_canonical = format!("{}::{}", current_canonical_path, mod_name);

                    // Encontrar o NodeId correspondente no tree
                    if let Some(child_node_id) = tree.find_by_canonical_path(&child_canonical) {
                        extract_from_items(
                            inline_items,
                            child_node_id,
                            &child_canonical,
                            tree,
                            from_crate,
                            workspace_crate_names,
                            output,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

/// Processa um ficheiro completo: parsear, percorrer items, descer em mod inline, extrair use.
fn extract_from_file(
    file_path: &Path,
    parent_node: NodeId,
    parent_canonical_path: &str,
    tree: &ModuleTree,
    from_crate: &str,
    workspace_crate_names: &[String],
    output: &mut Vec<ImportEdge>,
) -> Result<(), ExtractError> {
    let content = fs::read_to_string(file_path).map_err(|e| ExtractError::FileReadFailed {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    let ast = syn::parse_file(&content).map_err(|e| ExtractError::ParseFailed {
        file: file_path.to_path_buf(),
        source: e,
    })?;

    extract_from_items(
        &ast.items,
        parent_node,
        parent_canonical_path,
        tree,
        from_crate,
        workspace_crate_names,
        output,
    );

    Ok(())
}

/// Extrai todas as arestas de import de todos os módulos de um crate.
pub fn extract_imports(
    member: &WorkspaceMember,
    tree: &ModuleTree,
    workspace_crate_names: &[String],
) -> Result<Vec<ImportEdge>, ExtractError> {
    if member.entry_kind.is_empty() {
        return Ok(Vec::new());
    }

    let mut output = Vec::new();
    let from_crate = &member.name;

    // Rastrear ficheiros já processados para evitar duplicação
    // (módulos inline partilham o ficheiro do pai)
    let mut processed_files: HashSet<PathBuf> = HashSet::new();

    for (node_id, module_node) in tree.all_nodes() {
        // Módulos inline são processados quando o ficheiro do pai é processado
        if module_node.is_inline {
            continue;
        }

        // Evitar processar o mesmo ficheiro mais de uma vez
        if !processed_files.insert(module_node.source_file.clone()) {
            continue;
        }

        extract_from_file(
            &module_node.source_file,
            node_id,
            &module_node.canonical_path,
            tree,
            from_crate,
            workspace_crate_names,
            &mut output,
        )?;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_import_kind_all_categories() {
        let ws = vec!["crate_b".to_string(), "crate_c".to_string()];

        assert_eq!(
            classify_import_kind("my_crate", "my_crate", &ws),
            ImportKind::CurrentCrate
        );
        assert_eq!(
            classify_import_kind("crate_b", "my_crate", &ws),
            ImportKind::WorkspaceCrate
        );
        assert_eq!(
            classify_import_kind("std", "my_crate", &ws),
            ImportKind::Stdlib
        );
        assert_eq!(
            classify_import_kind("core", "my_crate", &ws),
            ImportKind::Stdlib
        );
        assert_eq!(
            classify_import_kind("alloc", "my_crate", &ws),
            ImportKind::Stdlib
        );
        assert_eq!(
            classify_import_kind("serde", "my_crate", &ws),
            ImportKind::External
        );
        assert_eq!(
            classify_import_kind("tokio", "my_crate", &ws),
            ImportKind::External
        );
    }

    #[test]
    fn test_expand_use_tree_simple() {
        let code = "use a::b::Foo;";
        let file = syn::parse_file(code).unwrap();
        if let syn::Item::Use(item_use) = &file.items[0] {
            let mut atomics = Vec::new();
            expand_use_tree(&item_use.tree, &[], &mut atomics);
            assert_eq!(atomics.len(), 1);
            assert_eq!(atomics[0].prefix_segments, vec!["a", "b"]);
            assert_eq!(atomics[0].item, "Foo");
            assert!(atomics[0].alias.is_none());
            assert!(!atomics[0].is_glob);
        }
    }

    #[test]
    fn test_expand_use_tree_group() {
        let code = "use a::{X, Y, Z};";
        let file = syn::parse_file(code).unwrap();
        if let syn::Item::Use(item_use) = &file.items[0] {
            let mut atomics = Vec::new();
            expand_use_tree(&item_use.tree, &[], &mut atomics);
            assert_eq!(atomics.len(), 3);
            assert_eq!(atomics[0].item, "X");
            assert_eq!(atomics[1].item, "Y");
            assert_eq!(atomics[2].item, "Z");
        }
    }

    #[test]
    fn test_expand_use_tree_glob() {
        let code = "use a::b::*;";
        let file = syn::parse_file(code).unwrap();
        if let syn::Item::Use(item_use) = &file.items[0] {
            let mut atomics = Vec::new();
            expand_use_tree(&item_use.tree, &[], &mut atomics);
            assert_eq!(atomics.len(), 1);
            assert_eq!(atomics[0].item, "*");
            assert!(atomics[0].is_glob);
        }
    }

    #[test]
    fn test_expand_use_tree_rename() {
        let code = "use a::Foo as Bar;";
        let file = syn::parse_file(code).unwrap();
        if let syn::Item::Use(item_use) = &file.items[0] {
            let mut atomics = Vec::new();
            expand_use_tree(&item_use.tree, &[], &mut atomics);
            assert_eq!(atomics.len(), 1);
            assert_eq!(atomics[0].item, "Foo");
            assert_eq!(atomics[0].alias, Some("Bar".to_string()));
        }
    }
}
