use crystalline_dsm_core::entities::workspace::{EntryKind, WorkspaceMember};
use crystalline_dsm_infra::module_traverser::{TraverseError, traverse_crate};
use std::path::PathBuf;

/// Resolve o caminho absoluto de uma fixture a partir de sua pasta.
fn get_fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap();
    workspace_root.join("tests").join("fixtures").join(name)
}

fn create_member(name: &str) -> WorkspaceMember {
    let root = get_fixture_path(name);
    WorkspaceMember {
        name: name.to_string(),
        crate_root: root.clone(),
        entry_kind: EntryKind::Library {
            lib_path: root.join("src/lib.rs"),
        },
    }
}

#[test]
fn test_traverse_module_tree_flat() {
    let member = create_member("module-tree-flat");
    let tree = traverse_crate(&member).expect("Falha ao percorrer crate");

    assert_eq!(tree.node_count(), 4);

    let root_id = tree.root();
    let children = tree.children(root_id);
    assert_eq!(children.len(), 3);

    // Verifica se os nós a, b e c existem
    let id_a = tree
        .find_by_canonical_path("module-tree-flat::a")
        .expect("Módulo a não encontrado");
    let id_b = tree
        .find_by_canonical_path("module-tree-flat::b")
        .expect("Módulo b não encontrado");
    let id_c = tree
        .find_by_canonical_path("module-tree-flat::c")
        .expect("Módulo c não encontrado");

    let node_a = tree.node(id_a);
    assert_eq!(node_a.module_path, vec!["a"]);
    assert!(!node_a.is_inline);
    assert!(!node_a.has_custom_path);
    assert!(node_a.source_file.ends_with("src/a.rs"));

    let node_b = tree.node(id_b);
    assert_eq!(node_b.module_path, vec!["b"]);
    assert!(!node_b.is_inline);
    assert!(!node_b.has_custom_path);
    assert!(node_b.source_file.ends_with("src/b.rs"));

    let node_c = tree.node(id_c);
    assert_eq!(node_c.module_path, vec!["c"]);
    assert!(!node_c.is_inline);
    assert!(!node_c.has_custom_path);
    assert!(node_c.source_file.ends_with("src/c.rs"));
}

#[test]
fn test_traverse_module_tree_nested() {
    let member = create_member("module-tree-nested");
    let tree = traverse_crate(&member).expect("Falha ao percorrer crate");

    assert_eq!(tree.node_count(), 4);

    let id_a = tree
        .find_by_canonical_path("module-tree-nested::a")
        .expect("a não encontrado");
    let id_b = tree
        .find_by_canonical_path("module-tree-nested::a::b")
        .expect("b não encontrado");
    let id_c = tree
        .find_by_canonical_path("module-tree-nested::a::b::c")
        .expect("c não encontrado");

    assert_eq!(tree.parent(id_c), Some(id_b));
    assert_eq!(tree.parent(id_b), Some(id_a));
    assert_eq!(tree.parent(id_a), Some(tree.root()));

    let node_c = tree.node(id_c);
    assert_eq!(node_c.module_path, vec!["a", "b", "c"]);
    assert!(node_c.source_file.ends_with("src/a/b/c.rs"));
}

#[test]
fn test_traverse_module_tree_with_mod_rs() {
    let member = create_member("module-tree-with-mod-rs");
    let tree = traverse_crate(&member).expect("Falha ao percorrer crate");

    assert_eq!(tree.node_count(), 3);

    let id_a = tree
        .find_by_canonical_path("module-tree-with-mod-rs::a")
        .expect("a não encontrado");
    let id_b = tree
        .find_by_canonical_path("module-tree-with-mod-rs::a::b")
        .expect("b não encontrado");

    let node_a = tree.node(id_a);
    assert!(node_a.source_file.ends_with("src/a/mod.rs"));

    let node_b = tree.node(id_b);
    assert_eq!(node_b.module_path, vec!["a", "b"]);
    assert!(node_b.source_file.ends_with("src/a/b.rs"));
}

#[test]
fn test_traverse_module_tree_inline() {
    let member = create_member("module-tree-inline");
    let tree = traverse_crate(&member).expect("Falha ao percorrer crate");

    assert_eq!(tree.node_count(), 3);

    let id_a = tree
        .find_by_canonical_path("module-tree-inline::a")
        .expect("a não encontrado");
    let id_b = tree
        .find_by_canonical_path("module-tree-inline::a::b")
        .expect("b não encontrado");

    let node_a = tree.node(id_a);
    assert!(node_a.is_inline);
    assert!(node_a.source_file.ends_with("src/lib.rs"));

    let node_b = tree.node(id_b);
    assert!(node_b.is_inline);
    assert!(node_b.source_file.ends_with("src/lib.rs"));
}

#[test]
fn test_traverse_module_tree_with_path_attr() {
    let member = create_member("module-tree-with-path-attr");
    let tree = traverse_crate(&member).expect("Falha ao percorrer crate");

    assert_eq!(tree.node_count(), 2);

    let id_x = tree
        .find_by_canonical_path("module-tree-with-path-attr::x")
        .expect("x não encontrado");

    let node_x = tree.node(id_x);
    assert!(node_x.has_custom_path);
    assert!(!node_x.is_inline);
    assert!(node_x.source_file.ends_with("src/custom/special.rs"));
}

#[test]
fn test_traverse_module_tree_missing_file() {
    let member = create_member("module-tree-missing-file");
    let result = traverse_crate(&member);

    assert!(result.is_err());
    let err = result.unwrap_err();
    if let TraverseError::ModuleFileNotFound { module, .. } = err {
        assert_eq!(module, "inexistente");
    } else {
        panic!("Esperava-se TraverseError::ModuleFileNotFound");
    }
}

#[test]
fn test_traverse_module_tree_syntax_error() {
    let member = create_member("module-tree-syntax-error");
    let result = traverse_crate(&member);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TraverseError::ParseFailed { .. }
    ));
}

#[test]
fn test_traverse_module_tree_cfg_duplicate() {
    let member = create_member("module-tree-cfg-duplicate");
    let tree = traverse_crate(&member).expect("Falha ao percorrer crate");

    // Deve ter apenas a raiz e o primeiro módulo 'platform' encontrado (linux.rs)
    assert_eq!(tree.node_count(), 2);

    let id_platform = tree
        .find_by_canonical_path("module-tree-cfg-duplicate::platform")
        .expect("platform não encontrado");

    let node_platform = tree.node(id_platform);
    assert!(node_platform.source_file.ends_with("src/platform/linux.rs"));
}

/// Regressão ADR-0008: entry point de target com nome não-canónico
/// (ex: `[[test]] path = "src/runner.rs"`) deve ser tratado como
/// entry-style, e `mod helper;` deve resolver para o ficheiro irmão
/// `src/helper.rs`, não para `src/runner/helper.rs`.
#[test]
fn test_traverse_tests_only_entry_custom_name() {
    let crate_root = get_fixture_path("tests-entry-custom-name").join("custom");
    let entry = crate_root.join("src/runner.rs");
    let member = WorkspaceMember {
        name: "custom".to_string(),
        crate_root,
        entry_kind: EntryKind::TestsOnly {
            test_paths: vec![entry],
        },
    };

    let tree = traverse_crate(&member).expect("Falha ao percorrer crate custom");

    assert_eq!(tree.node_count(), 2);

    let id_helper = tree
        .find_by_canonical_path("custom::helper")
        .expect("módulo helper deveria ter sido resolvido como sibling");

    let node_helper = tree.node(id_helper);
    assert!(!node_helper.is_inline);
    assert!(!node_helper.has_custom_path);
    assert!(
        node_helper.source_file.ends_with("custom/src/helper.rs"),
        "esperado custom/src/helper.rs, obtido {:?}",
        node_helper.source_file,
    );
}
