use crystalline_dsm_core::entities::import_edge::ImportKind;
use crystalline_dsm_core::entities::workspace::{EntryKind, WorkspaceMember};
use crystalline_dsm_infra::import_extractor::extract_imports;
use crystalline_dsm_infra::module_traverser::traverse_crate;
use std::path::PathBuf;

fn get_fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap();
    workspace_root.join("tests").join("fixtures").join(name)
}

fn create_member(fixture_name: &str) -> WorkspaceMember {
    let root = get_fixture_path(fixture_name);
    WorkspaceMember {
        name: fixture_name.replace('-', "_"),
        crate_root: root.clone(),
        entry_kind: EntryKind::Library {
            lib_path: root.join("src/lib.rs"),
        },
    }
}

#[test]
fn test_imports_simple() {
    let member = create_member("imports-simple");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].target_module, "a::b");
    assert_eq!(edges[0].imported_item, "Foo");
    assert_eq!(edges[0].kind, ImportKind::External);
    assert!(!edges[0].is_glob);
    assert!(edges[0].alias.is_none());
    assert!(!edges[0].is_reexport);
}

#[test]
fn test_imports_current_crate() {
    let member = create_member("imports-current-crate");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    // Filtrar apenas imports crate:: (pode haver outras coisas)
    let crate_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == ImportKind::CurrentCrate)
        .collect();

    assert_eq!(crate_edges.len(), 1);
    assert_eq!(crate_edges[0].imported_item, "helper");
    assert!(crate_edges[0].target_module.contains("utils"));
}

#[test]
fn test_imports_self() {
    let member = create_member("imports-self");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    let self_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == ImportKind::CurrentCrate)
        .collect();

    assert_eq!(self_edges.len(), 1);
    assert_eq!(self_edges[0].imported_item, "Foo");
    assert!(self_edges[0].raw_use_path.contains("self"));
}

#[test]
fn test_imports_super() {
    let member = create_member("imports-super");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    let super_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.raw_use_path.contains("super"))
        .collect();

    assert_eq!(super_edges.len(), 1);
    assert_eq!(super_edges[0].imported_item, "Foo");
    assert_eq!(super_edges[0].kind, ImportKind::CurrentCrate);
}

#[test]
fn test_imports_super_out_of_bounds() {
    let member = create_member("imports-super-out-of-bounds");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].kind, ImportKind::Unresolved);
}

#[test]
fn test_imports_stdlib() {
    let member = create_member("imports-stdlib");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].kind, ImportKind::Stdlib);
    assert_eq!(edges[0].imported_item, "HashMap");
    assert_eq!(edges[0].target_module, "std::collections");
}

#[test]
fn test_imports_workspace() {
    let member_a = WorkspaceMember {
        name: "crate_a".to_string(),
        crate_root: get_fixture_path("imports-workspace/crate_a"),
        entry_kind: EntryKind::Library {
            lib_path: get_fixture_path("imports-workspace/crate_a/src/lib.rs"),
        },
    };

    let tree = traverse_crate(&member_a).unwrap();
    let ws_names = vec!["crate_a".to_string(), "crate_b".to_string()];
    let edges = extract_imports(&member_a, &tree, &ws_names).unwrap();

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].kind, ImportKind::WorkspaceCrate);
    assert_eq!(edges[0].imported_item, "Bar");
    assert!(edges[0].target_module.contains("crate_b"));
}

#[test]
fn test_imports_use_list() {
    let member = create_member("imports-use-list");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    assert_eq!(edges.len(), 3);
    let items: Vec<&str> = edges.iter().map(|e| e.imported_item.as_str()).collect();
    assert!(items.contains(&"X"));
    assert!(items.contains(&"Y"));
    assert!(items.contains(&"Z"));
}

#[test]
fn test_imports_glob() {
    let member = create_member("imports-glob");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    assert_eq!(edges.len(), 1);
    assert!(edges[0].is_glob);
    assert_eq!(edges[0].imported_item, "*");
}

#[test]
fn test_imports_alias() {
    let member = create_member("imports-alias");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].imported_item, "Foo");
    assert_eq!(edges[0].alias, Some("Bar".to_string()));
    assert!(!edges[0].is_glob);
}

#[test]
fn test_imports_reexport() {
    let member = create_member("imports-reexport");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    assert_eq!(edges.len(), 1);
    assert!(edges[0].is_reexport);
}

#[test]
fn test_imports_inline_module() {
    let member = create_member("imports-inline-module");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].imported_item, "Foo");

    // O from deve ser o NodeId do módulo inline 'sub', não o da raiz
    let sub_id = tree
        .find_by_canonical_path("imports_inline_module::sub")
        .expect("sub não encontrado");
    assert_eq!(edges[0].from, sub_id);
}

#[test]
fn test_imports_self_in_group() {
    let member = create_member("imports-self-in-group");
    let tree = traverse_crate(&member).unwrap();
    let edges = extract_imports(&member, &tree, &[]).unwrap();

    assert_eq!(edges.len(), 2);

    // Uma é o self (importa o módulo a em si), outra é X
    let items: Vec<&str> = edges.iter().map(|e| e.imported_item.as_str()).collect();
    assert!(items.contains(&"a"));
    assert!(items.contains(&"X"));
}
