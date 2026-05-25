/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/config_sarif_readers.md
 * @layer L3
 * @updated 2026-05-25
 */

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crystalline_dsm_core::entities::layer_config::{Layer, LayerConfig};
use crystalline_dsm_core::entities::workspace::Workspace;

#[derive(Debug, thiserror::Error)]
pub enum ConfigReadError {
    #[error("crystalline.toml não encontrado: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Falha ao ler crystalline.toml: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("Falha ao parsear TOML: {source}")]
    ParseFailed {
        #[from]
        source: toml::de::Error,
    },

    #[error("Seção [layers] ausente no crystalline.toml")]
    NoLayersSection,
}

#[derive(Deserialize)]
struct ConfigDoc {
    layers: Option<HashMap<String, String>>,
}

/// Lê a seção `[layers]` de um `crystalline.toml` e cruza com os
/// `crate_root`s do workspace para construir o `LayerConfig`.
pub fn read_layer_config(
    toml_path: &Path,
    workspace: &Workspace,
) -> Result<LayerConfig, ConfigReadError> {
    if !toml_path.exists() {
        return Err(ConfigReadError::FileNotFound {
            path: toml_path.to_path_buf(),
        });
    }

    let content = fs::read_to_string(toml_path)?;
    let doc: ConfigDoc = toml::from_str(&content)?;

    let layers_table = doc.layers.ok_or(ConfigReadError::NoLayersSection)?;

    // Guardar mapa: diretório -> Layer
    let mut dir_to_layer = HashMap::new();
    for (key, val) in layers_table {
        if let Some(layer) = Layer::from_config_key(&key) {
            dir_to_layer.insert(val, layer);
        }
    }

    let mut crate_to_layer = HashMap::new();
    for member in &workspace.members {
        let last_component = member.crate_root.file_name().and_then(|s| s.to_str());
        if let Some(&layer) = last_component.and_then(|lc| dir_to_layer.get(lc)) {
            // Normaliza o nome substituindo '-' por '_'
            let normalized_name = member.name.replace('-', "_");
            crate_to_layer.insert(normalized_name, layer);
        }
    }

    Ok(LayerConfig::new(crate_to_layer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crystalline_dsm_core::entities::workspace::{EntryKind, WorkspaceMember};

    fn tmp_file(name: &str, content: &str) -> PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("test-{}-{}-{}", name, pid, nanos));
        fs::write(&path, content).unwrap();
        path
    }

    fn create_mock_member(name: &str, folder: &str) -> WorkspaceMember {
        WorkspaceMember {
            name: name.to_string(),
            crate_root: PathBuf::from(format!("/abs/path/{}", folder)),
            entry_kind: EntryKind::NoSourceTarget,
        }
    }

    #[test]
    fn test_read_valid_config() {
        let toml_content = r#"
            [layers]
            L1 = "01_core"
            L2 = "02_shell"
            L3 = "03_infra"
            L4 = "04_wiring"
            lab = "lab"
        "#;
        let file = tmp_file("valid_toml", toml_content);

        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member("crystalline-dsm-core", "01_core"),
                create_mock_member("crystalline-dsm-shell", "02_shell"),
                create_mock_member("crystalline-dsm-infra", "03_infra"),
            ],
        };

        let config = read_layer_config(&file, &ws).unwrap();
        assert_eq!(config.len(), 3);
        assert_eq!(
            config.layer_of_crate("crystalline_dsm_core"),
            Some(Layer::L1)
        );
        assert_eq!(
            config.layer_of_crate("crystalline_dsm_shell"),
            Some(Layer::L2)
        );
        assert_eq!(
            config.layer_of_crate("crystalline_dsm_infra"),
            Some(Layer::L3)
        );

        fs::remove_file(file).ok();
    }

    #[test]
    fn test_file_not_found() {
        let path = PathBuf::from("/non/existent/path/crystalline.toml");
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![],
        };
        let res = read_layer_config(&path, &ws);
        assert!(matches!(res, Err(ConfigReadError::FileNotFound { .. })));
    }

    #[test]
    fn test_malformed_toml() {
        let toml_content = r#"
            [layers
            L1 = "01_core"
        "#;
        let file = tmp_file("malformed_toml", toml_content);
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![],
        };
        let res = read_layer_config(&file, &ws);
        assert!(matches!(res, Err(ConfigReadError::ParseFailed { .. })));
        fs::remove_file(file).ok();
    }

    #[test]
    fn test_no_layers_section() {
        let toml_content = r#"
            [project]
            name = "test"
        "#;
        let file = tmp_file("no_layers", toml_content);
        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![],
        };
        let res = read_layer_config(&file, &ws);
        assert!(matches!(res, Err(ConfigReadError::NoLayersSection)));
        fs::remove_file(file).ok();
    }

    #[test]
    fn test_crate_outside_topology() {
        let toml_content = r#"
            [layers]
            L1 = "01_core"
        "#;
        let file = tmp_file("outside_topology", toml_content);

        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member("core", "01_core"),
                create_mock_member("outside", "99_outside"),
            ],
        };

        let config = read_layer_config(&file, &ws).unwrap();
        assert_eq!(config.len(), 1);
        assert_eq!(config.layer_of_crate("core"), Some(Layer::L1));
        assert_eq!(config.layer_of_crate("outside"), None);

        fs::remove_file(file).ok();
    }

    #[test]
    fn test_normalization_name() {
        let toml_content = r#"
            [layers]
            L1 = "01_core"
        "#;
        let file = tmp_file("normalization", toml_content);

        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![create_mock_member("crystalline-dsm-core", "01_core")],
        };

        let config = read_layer_config(&file, &ws).unwrap();
        // Deve registrar com underscores
        assert_eq!(
            config.layer_of_crate("crystalline_dsm_core"),
            Some(Layer::L1)
        );
        // O original com hífens não deve achar no LayerConfig, a menos que normalizado a jusante
        assert_eq!(config.layer_of_crate("crystalline-dsm-core"), None);

        fs::remove_file(file).ok();
    }

    #[test]
    fn test_unknown_layer_ignored() {
        let toml_content = r#"
            [layers]
            L1 = "01_core"
            L5 = "05_extra"
        "#;
        let file = tmp_file("unknown_layer", toml_content);

        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![
                create_mock_member("core", "01_core"),
                create_mock_member("extra", "05_extra"),
            ],
        };

        let config = read_layer_config(&file, &ws).unwrap();
        assert_eq!(config.len(), 1);
        assert_eq!(config.layer_of_crate("core"), Some(Layer::L1));
        assert_eq!(config.layer_of_crate("extra"), None);

        fs::remove_file(file).ok();
    }

    #[test]
    fn test_extra_sections_ignored() {
        let toml_content = r#"
            [project]
            name = "test"

            [layers]
            L1 = "01_core"

            [rules]
            v9 = "error"
        "#;
        let file = tmp_file("extra_sections", toml_content);

        let ws = Workspace {
            root: PathBuf::from("/abs/path"),
            members: vec![create_mock_member("core", "01_core")],
        };

        let config = read_layer_config(&file, &ws).unwrap();
        assert_eq!(config.len(), 1);
        assert_eq!(config.layer_of_crate("core"), Some(Layer::L1));

        fs::remove_file(file).ok();
    }
}
