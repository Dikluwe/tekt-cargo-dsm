use crystalline_dsm_core::entities::workspace::EntryKind;
use crystalline_dsm_infra::cargo_metadata_reader::{CargoMetadataError, read_workspace};
use std::path::PathBuf;

/// Resolve o caminho absoluto de uma fixture a partir de sua pasta.
fn get_fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Em workspaces virtuais do Cargo, a pasta tests/fixtures fica na raiz,
    // que está um nível acima da sub-crate 03_infra
    let workspace_root = manifest_dir.parent().unwrap();
    workspace_root.join("tests").join("fixtures").join(name)
}

#[test]
fn test_read_empty_workspace() {
    let path = get_fixture_path("empty-workspace");
    let result = read_workspace(&path);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CargoMetadataError::EmptyWorkspace
    ));
}

#[test]
fn test_read_single_lib_crate() {
    let path = get_fixture_path("single-lib-crate");
    let result = read_workspace(&path).expect("Falha ao ler workspace");

    assert_eq!(result.members.len(), 1);
    let member = &result.members[0];
    assert_eq!(member.name, "a");
    if let EntryKind::Library { lib_path } = &member.entry_kind {
        assert!(lib_path.ends_with("src/lib.rs"));
    } else {
        panic!("Esperava-se EntryKind::Library");
    }
}

#[test]
fn test_read_single_bin_crate() {
    let path = get_fixture_path("single-bin-crate");
    let result = read_workspace(&path).expect("Falha ao ler workspace");

    assert_eq!(result.members.len(), 1);
    let member = &result.members[0];
    assert_eq!(member.name, "a");
    if let EntryKind::Binary { main_path } = &member.entry_kind {
        assert!(main_path.ends_with("src/main.rs"));
    } else {
        panic!("Esperava-se EntryKind::Binary");
    }
}

#[test]
fn test_read_lib_and_bin_crate() {
    let path = get_fixture_path("lib-and-bin-crate");
    let result = read_workspace(&path).expect("Falha ao ler workspace");

    assert_eq!(result.members.len(), 1);
    let member = &result.members[0];
    assert_eq!(member.name, "a");
    if let EntryKind::LibraryAndBinary { lib_path, main_path } = &member.entry_kind {
        assert!(lib_path.ends_with("src/lib.rs"));
        assert!(main_path.ends_with("src/main.rs"));
    } else {
        panic!("Esperava-se EntryKind::LibraryAndBinary");
    }
}

#[test]
fn test_read_multi_crate_workspace() {
    let path = get_fixture_path("multi-crate-workspace");
    let result = read_workspace(&path).expect("Falha ao ler workspace");

    assert_eq!(result.members.len(), 3);

    let names: Vec<String> = result.members.iter().map(|m| m.name.clone()).collect();
    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"b".to_string()));
    assert!(names.contains(&"c".to_string()));

    for member in &result.members {
        if let EntryKind::Library { lib_path } = &member.entry_kind {
            assert!(lib_path.ends_with("src/lib.rs"));
        } else {
            panic!("Membro {} deveria ser Library", member.name);
        }
    }
}

#[test]
fn test_read_invalid_path() {
    let path = PathBuf::from("caminho-inexistente-xyz");
    let result = read_workspace(&path);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CargoMetadataError::InvalidPath { .. }
    ));
}

#[test]
fn test_read_not_a_workspace() {
    let path = get_fixture_path("not-a-workspace");
    let result = read_workspace(&path);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CargoMetadataError::MetadataExecutionFailed { .. }
    ));
}

#[test]
fn test_read_proc_macro_crate() {
    let path = get_fixture_path("proc-macro-crate");
    let result = read_workspace(&path).expect("Falha ao ler workspace");

    assert_eq!(result.members.len(), 1);
    let member = &result.members[0];
    assert_eq!(member.name, "macros");
    if let EntryKind::ProcMacro { lib_path } = &member.entry_kind {
        assert!(lib_path.ends_with("macros/src/lib.rs"));
    } else {
        panic!("Esperava-se EntryKind::ProcMacro");
    }
}

#[test]
fn test_read_tests_only_crate() {
    let path = get_fixture_path("tests-only-crate");
    let result = read_workspace(&path).expect("Falha ao ler workspace");

    assert_eq!(result.members.len(), 1);
    let member = &result.members[0];
    assert_eq!(member.name, "only_tests");
    if let EntryKind::TestsOnly { test_paths } = &member.entry_kind {
        assert_eq!(test_paths.len(), 1);
        assert!(test_paths[0].ends_with("only_tests/tests/integration.rs"));
    } else {
        panic!("Esperava-se EntryKind::TestsOnly");
    }
}

#[test]
fn test_read_no_source_crate() {
    let path = get_fixture_path("no-source-crate");
    let result = read_workspace(&path).expect("Falha ao ler workspace");

    assert_eq!(result.members.len(), 1);
    let member = &result.members[0];
    assert_eq!(member.name, "empty");
    assert!(matches!(member.entry_kind, EntryKind::NoSourceTarget));
}
