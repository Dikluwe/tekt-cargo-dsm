/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/cargo_metadata_reader.md
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

    #[error("Workspace member '{name}' não tem nem lib nem binário")]
    NoEntryPoint { name: String },

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

        let (entry_kind, entry_point) = match classify_targets(package) {
            Ok(res) => res,
            Err(CargoMetadataError::NoEntryPoint { .. }) => {
                // Silenciosamente ignora pacotes que não têm lib nem binário
                // (como crates exclusivas de testes, benchmarks ou documentação)
                continue;
            }
            Err(e) => return Err(e),
        };

        let manifest_dir =
            package
                .manifest_path
                .parent()
                .ok_or_else(|| CargoMetadataError::NoEntryPoint {
                    name: package.name.clone(),
                })?;

        members.push(WorkspaceMember {
            name: package.name.clone(),
            crate_root: PathBuf::from(manifest_dir),
            entry_point,
            entry_kind,
        });
    }

    let workspace_root = PathBuf::from(metadata.workspace_root);

    Ok(Workspace {
        root: workspace_root,
        members,
    })
}

/// Classifica os targets de um pacote Cargo para identificar o ponto de entrada principal e o tipo.
fn classify_targets(
    package: &cargo_metadata::Package,
) -> Result<(EntryKind, PathBuf), CargoMetadataError> {
    let mut lib_target = None;
    let mut bin_target = None;

    for target in &package.targets {
        if target.kind.iter().any(|k| {
            k == "lib" || k == "rlib" || k == "staticlib" || k == "cdylib" || k == "proc-macro"
        }) && lib_target.is_none()
        {
            lib_target = Some(target);
        } else if target.kind.iter().any(|k| k == "bin") && bin_target.is_none() {
            bin_target = Some(target);
        }
    }

    match (lib_target, bin_target) {
        (Some(lib), Some(bin)) => Ok((
            EntryKind::LibraryAndBinary {
                main_path: PathBuf::from(&bin.src_path),
            },
            PathBuf::from(&lib.src_path),
        )),
        (Some(lib), None) => Ok((EntryKind::Library, PathBuf::from(&lib.src_path))),
        (None, Some(bin)) => Ok((EntryKind::Binary, PathBuf::from(&bin.src_path))),
        (None, None) => Err(CargoMetadataError::NoEntryPoint {
            name: package.name.clone(),
        }),
    }
}
