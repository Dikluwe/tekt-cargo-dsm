/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/module_traverser.md
 * @prompt 00_nucleo/prompts/module_traverser-revisao.md
 * @layer L3
 * @updated 2026-05-20
 */

use crystalline_dsm_core::entities::module_tree::{ModuleTree, NodeId};
use crystalline_dsm_core::entities::workspace::WorkspaceMember;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum TraverseError {
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

    #[error("Ficheiro de módulo não encontrado para 'mod {module}' em {parent_file}")]
    ModuleFileNotFound {
        module: String,
        parent_file: PathBuf,
        attempted_paths: Vec<PathBuf>,
    },

    #[error("Erro de construção da árvore: {0}")]
    TreeError(#[from] crystalline_dsm_core::entities::module_tree::TreeError),
}

/// Extrai o valor do atributo `#[path = "..."]` de uma lista de atributos, se presente.
#[allow(clippy::collapsible_if)]
fn get_path_attribute(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("path") {
            if let syn::Meta::NameValue(syn::MetaNameValue {
                value:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }),
                ..
            }) = &attr.meta
            {
                return Some(lit_str.value());
            }
        }
    }
    None
}

/// `true` apenas se o nome do ficheiro for `mod.rs` — única convenção
/// sintáctica que herda entry-style por nome (ver ADR-0008).
fn is_mod_rs(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == "mod.rs")
        .unwrap_or(false)
}

/// Resolve o caminho absoluto de um módulo externo com base no arquivo pai e nos atributos.
///
/// `parent_is_entry` indica se o `parent_file` é entry-style:
/// `true` se for o ficheiro raiz da travessia (entry de target Cargo) ou
/// um `mod.rs`. Quando `true`, `mod foo;` resolve para o mesmo
/// directório do `parent_file`; quando `false`, resolve para
/// `<stem>/foo.rs`.
fn resolve_module_path(
    parent_file: &Path,
    parent_is_entry: bool,
    module_ident: &syn::Ident,
    attrs: &[syn::Attribute],
) -> Result<(PathBuf, bool), TraverseError> {
    let module_name = module_ident.to_string();
    let parent_dir = parent_file.parent().unwrap_or_else(|| Path::new("."));

    // 1. Se há #[path = "x"]
    if let Some(custom_path) = get_path_attribute(attrs) {
        let resolved = parent_dir.join(&custom_path);
        if resolved.exists() {
            return Ok((resolved, true));
        } else {
            return Err(TraverseError::ModuleFileNotFound {
                module: module_name,
                parent_file: parent_file.to_path_buf(),
                attempted_paths: vec![resolved],
            });
        }
    }

    // 2. Sem #[path] (resolução padrão)
    let search_dir = if parent_is_entry {
        parent_dir.to_path_buf()
    } else {
        let stem = parent_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        parent_dir.join(stem)
    };

    let path_a = search_dir.join(format!("{}.rs", module_name));
    let path_b = search_dir.join(&module_name).join("mod.rs");

    if path_a.exists() {
        Ok((path_a, false))
    } else if path_b.exists() {
        Ok((path_b, false))
    } else {
        Err(TraverseError::ModuleFileNotFound {
            module: module_name,
            parent_file: parent_file.to_path_buf(),
            attempted_paths: vec![path_a, path_b],
        })
    }
}

/// Percorre um vetor de itens da AST do Rust para encontrar e processar declarações de módulos.
///
/// `parent_is_entry` flui para o resolvedor: indica se o
/// `file_path` actual é entry-style (raiz da travessia ou `mod.rs`).
fn traverse_items(
    tree: &mut ModuleTree,
    parent_node: NodeId,
    file_path: &Path,
    parent_is_entry: bool,
    items: &[syn::Item],
    seen_children: &mut HashSet<(NodeId, String)>,
) -> Result<(), TraverseError> {
    for item in items {
        if let syn::Item::Mod(item_mod) = item {
            let module_name = item_mod.ident.to_string();
            let child_key = (parent_node, module_name.clone());

            // Detecção de duplicatas (ex: #[cfg] repetidos para o mesmo módulo)
            if !seen_children.insert(child_key) {
                eprintln!(
                    "Warning: módulo '{}' duplicado ignorado em {}",
                    module_name,
                    file_path.display()
                );
                continue;
            }

            if let Some((_, inline_items)) = &item_mod.content {
                // Módulo inline: mod foo { ... }
                if get_path_attribute(&item_mod.attrs).is_some() {
                    eprintln!(
                        "Warning: ignorando atributo #[path] em módulo inline '{}' em {}",
                        module_name,
                        file_path.display()
                    );
                }

                let child_id = tree.add_child(
                    parent_node,
                    module_name,
                    file_path.to_path_buf(),
                    true,
                    false,
                )?;

                // Módulo inline herda o mesmo ficheiro e o mesmo estatuto entry-style.
                traverse_items(
                    tree,
                    child_id,
                    file_path,
                    parent_is_entry,
                    inline_items,
                    seen_children,
                )?;
            } else {
                // Módulo externo: mod foo;
                let (resolved_path, has_custom_path) = resolve_module_path(
                    file_path,
                    parent_is_entry,
                    &item_mod.ident,
                    &item_mod.attrs,
                )?;

                let child_id = tree.add_child(
                    parent_node,
                    module_name,
                    resolved_path.clone(),
                    false,
                    has_custom_path,
                )?;

                // Ao descer, só `mod.rs` herda entry-style por nome (ADR-0008).
                let child_is_entry = is_mod_rs(&resolved_path);
                traverse_file(tree, child_id, &resolved_path, child_is_entry, seen_children)?;
            }
        }
    }
    Ok(())
}

/// Lê, parseia e processa um arquivo de código Rust.
///
/// `is_entry` indica se este ficheiro é entry-style: raiz da
/// travessia ou um `mod.rs`.
fn traverse_file(
    tree: &mut ModuleTree,
    parent_node: NodeId,
    file_path: &Path,
    is_entry: bool,
    seen_children: &mut HashSet<(NodeId, String)>,
) -> Result<(), TraverseError> {
    let content = fs::read_to_string(file_path).map_err(|e| TraverseError::FileReadFailed {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    let ast = syn::parse_file(&content).map_err(|e| TraverseError::ParseFailed {
        file: file_path.to_path_buf(),
        source: e,
    })?;

    traverse_items(tree, parent_node, file_path, is_entry, &ast.items, seen_children)?;
    Ok(())
}

/// Realiza a travessia completa de um crate membro do workspace para gerar sua árvore de módulos lógica.
pub fn traverse_crate(member: &WorkspaceMember) -> Result<ModuleTree, TraverseError> {
    let entry_path = match member.entry_kind.primary_entry() {
        Some(path) => path.to_path_buf(),
        None => {
            // NoSourceTarget ou similar: retorna árvore com raiz apontando para path vazio
            let tree = ModuleTree::new(member.name.clone(), PathBuf::new());
            return Ok(tree);
        }
    };

    let mut tree = ModuleTree::new(member.name.clone(), entry_path.clone());
    let mut seen_children = HashSet::new();

    let root_id = tree.root();
    // O entry point do target é, por construção, entry-style (ADR-0008).
    traverse_file(&mut tree, root_id, &entry_path, true, &mut seen_children)?;

    Ok(tree)
}
