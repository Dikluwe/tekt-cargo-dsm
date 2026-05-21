/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/cargo_metadata_reader.md
 * @prompt 00_nucleo/prompts/cargo_metadata_reader-revisao.md
 * @layer L3
 * @updated 2026-05-20
 */

use cargo_metadata::MetadataCommand;
use crystalline_dsm_core::entities::workspace::{EntryKind, Workspace, WorkspaceMember};
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum CargoMetadataError {
    #[error("Caminho inválido ou inacessível: {path}")]
    InvalidPath { path: PathBuf },

    #[error("Falha ao executar 'cargo metadata': {source}")]
    MetadataExecutionFailed {
        #[from]
        source: cargo_metadata::Error,
    },

    #[error("Workspace não contém nenhum membro")]
    EmptyWorkspace,
}

/// Lê as informações de metadados do Cargo a partir de um diretório ou caminho do Cargo.toml.
pub fn read_workspace(workspace_path: &Path) -> Result<Workspace, CargoMetadataError> {
    // 1. Validar se o caminho existe e é acessível
    if !workspace_path.exists() {
        return Err(CargoMetadataError::InvalidPath {
            path: workspace_path.to_path_buf(),
        });
    }

    // 2. Resolver o Cargo.toml do workspace
    let manifest_path = if workspace_path.is_file() {
        workspace_path.to_path_buf()
    } else {
        workspace_path.join("Cargo.toml")
    };

    // 3. Executar o cargo metadata
    let mut command = MetadataCommand::new();
    command.manifest_path(manifest_path);
    // Ignoramos dependências de terceiros para focar apenas nas crates membros locais
    command.no_deps();

    let metadata = command
        .exec()
        .map_err(|e| CargoMetadataError::MetadataExecutionFailed { source: e })?;

    // 4. Se o workspace não contiver nenhum membro
    if metadata.workspace_members.is_empty() {
        return Err(CargoMetadataError::EmptyWorkspace);
    }

    // 5. Mapear cada membro para a entidade L1
    let mut members = Vec::new();
    for package_id in &metadata.workspace_members {
        let package = &metadata[package_id];

        let entry_kind = classify_targets(package);

        let manifest_dir = package
            .manifest_path
            .parent()
            .map(|p| PathBuf::from(p.as_str()))
            .unwrap_or_else(|| PathBuf::from(package.manifest_path.as_str()));

        members.push(WorkspaceMember {
            name: package.name.clone(),
            crate_root: manifest_dir,
            entry_kind,
        });
    }

    let workspace_root = PathBuf::from(metadata.workspace_root.as_str());

    Ok(Workspace {
        root: workspace_root,
        members,
    })
}

/// Classifica os targets de um pacote Cargo para identificar o ponto de entrada principal e o tipo.
fn classify_targets(package: &cargo_metadata::Package) -> EntryKind {
    let mut lib_path = None;
    let mut main_path = None;
    let mut is_proc_macro = false;
    let mut test_paths = Vec::new();

    for target in &package.targets {
        // 1. Procurar lib
        if target.kind.iter().any(|k| {
            k == "lib" || k == "rlib" || k == "dylib" || k == "staticlib" || k == "cdylib"
        }) && lib_path.is_none()
        {
            lib_path = Some(PathBuf::from(target.src_path.as_str()));
        }

        // 2. Procurar proc-macro
        if target.kind.iter().any(|k| k == "proc-macro")
            || target.crate_types.iter().any(|ct| ct == "proc-macro")
        {
            is_proc_macro = true;
            if lib_path.is_none() {
                lib_path = Some(PathBuf::from(target.src_path.as_str()));
            }
        }

        // 3. Procurar bin
        if target.kind.iter().any(|k| k == "bin") && main_path.is_none() {
            main_path = Some(PathBuf::from(target.src_path.as_str()));
        }

        // 4. Procurar test
        if target.kind.iter().any(|k| k == "test") {
            test_paths.push(PathBuf::from(target.src_path.as_str()));
        }
    }

    // 5. Decidir variante
    match (is_proc_macro, lib_path, main_path) {
        (true, Some(lib), _) => EntryKind::ProcMacro { lib_path: lib },
        (_, Some(lib), Some(main)) => EntryKind::LibraryAndBinary {
            lib_path: lib,
            main_path: main,
        },
        (_, Some(lib), None) => EntryKind::Library { lib_path: lib },
        (_, None, Some(main)) => EntryKind::Binary { main_path: main },
        (_, None, None) if !test_paths.is_empty() => EntryKind::TestsOnly { test_paths },
        (_, None, None) => EntryKind::NoSourceTarget,
    }
}
