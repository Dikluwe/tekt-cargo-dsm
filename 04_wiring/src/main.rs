/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/cli.md
 * @prompt 00_nucleo/prompts/cli_output_flags.md
 * @prompt 00_nucleo/prompts/html_renderer.md
 * @layer L4
 * @updated 2026-05-20
 */

use clap::Parser;
use crystalline_dsm_core::entities::import_edge::ImportEdge;
use crystalline_dsm_core::entities::module_tree::ModuleTree;
use crystalline_dsm_core::rules::cycle_detector::detect_cycles;
use crystalline_dsm_core::rules::dsm_partitioner::partition_for_dsm;
use crystalline_dsm_core::rules::layer_violation_detector::detect_layer_violations;
use crystalline_dsm_infra::cargo_metadata_reader::{CargoMetadataError, read_workspace};
use crystalline_dsm_infra::crystalline_config_reader::read_layer_config;
use crystalline_dsm_infra::html_renderer::{HtmlRenderError, render_dsm_html};
use crystalline_dsm_infra::import_extractor::extract_imports;
use crystalline_dsm_infra::json_serializer::{JsonSerializeError, to_canonical_json};
use crystalline_dsm_infra::module_traverser::traverse_crate;
use crystalline_dsm_infra::sarif_reader::read_sarif;
use crystalline_dsm_infra::trees_serializer::{TreesSerializeError, to_canonical_json_trees};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub mod graph_builder;
use graph_builder::build_graph;

#[derive(Parser, Debug)]
#[command(name = "crystalline-dsm", version)]
#[command(about = "Gera uma Dependency Structure Matrix (DSM) para projetos Rust", long_about = None)]
struct Cli {
    /// Caminho do workspace Cargo a analisar
    workspace_path: PathBuf,

    /// Caminho do ficheiro de output do grafo JSON
    #[arg(short, long, default_value = "./graph.json")]
    output: PathBuf,

    /// Se presente, grava tambem trees.json no mesmo diretorio que --output
    #[arg(long)]
    emit_trees: bool,

    /// Se presente, grava tambem dsm.html no mesmo diretorio que --output
    #[arg(long)]
    emit_html: bool,

    /// Caminho opcional para o crystalline.toml (configuração de camadas)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Caminho opcional para o relatório SARIF
    #[arg(long)]
    sarif: Option<PathBuf>,
}

#[derive(Debug)]
pub struct PipelineReport {
    pub member_count: usize,
    pub module_count: usize,
    pub edge_count: usize,
    pub cycle_count: usize,
    pub output_path: PathBuf,
    pub trees_path: Option<PathBuf>,
    pub html_path: Option<PathBuf>,
    pub layer_violation_count: usize,
    pub sarif_finding_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Falha ao ler workspace: {0}")]
    WorkspaceError(#[from] CargoMetadataError),

    #[error("Falha ao serializar grafo JSON: {0}")]
    JsonError(#[from] JsonSerializeError),

    #[error("Falha ao serializar trees.json: {0}")]
    TreesError(#[from] TreesSerializeError),

    #[error("Falha ao renderizar dsm.html: {0}")]
    HtmlError(#[from] HtmlRenderError),

    #[error("Ficheiro de configuração não encontrado: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("Ficheiro SARIF não encontrado: {path}")]
    SarifNotFound { path: PathBuf },

    #[error("Falha ao ler configuração de camadas: {0}")]
    ConfigError(#[from] crystalline_dsm_infra::crystalline_config_reader::ConfigReadError),

    #[error("Falha ao ler relatório SARIF: {0}")]
    SarifError(#[from] crystalline_dsm_infra::sarif_reader::SarifReadError),

    #[error("Falha ao gravar ficheiro {path}: {source}")]
    WriteFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if !cli.workspace_path.exists() {
        eprintln!(
            "{}",
            crystalline_dsm_shell::format_error(
                "Workspace nao encontrado",
                &cli.workspace_path.display().to_string(),
            )
        );
        return ExitCode::from(1);
    }

    println!(
        "{}",
        crystalline_dsm_shell::format_start_analysis(&cli.workspace_path.display().to_string())
    );

    match run_pipeline(&cli) {
        Ok(report) => {
            println!(
                "{}",
                crystalline_dsm_shell::format_summary(
                    report.member_count,
                    report.module_count,
                    report.edge_count,
                    report.cycle_count,
                    &report.output_path,
                    report.trees_path.as_deref(),
                    report.html_path.as_deref(),
                    report.layer_violation_count,
                    report.sarif_finding_count,
                )
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!(
                "{}",
                crystalline_dsm_shell::format_error("Falha na analise", &e.to_string())
            );
            ExitCode::from(2)
        }
    }
}

fn run_pipeline(cli: &Cli) -> Result<PipelineReport, PipelineError> {
    let workspace = read_workspace(&cli.workspace_path)?;

    // Fase 1.2: trees (falhas individuais nao sao fatais)
    let mut trees: HashMap<String, ModuleTree> = HashMap::new();
    for member in &workspace.members {
        if let Ok(tree) = traverse_crate(member) {
            trees.insert(member.name.clone(), tree);
        }
    }

    // Fase 1.3: imports
    let workspace_crate_names: Vec<String> =
        workspace.members.iter().map(|m| m.name.clone()).collect();
    let mut edges_per_crate: HashMap<String, Vec<ImportEdge>> = HashMap::new();
    for (crate_name, tree) in &trees {
        let member = workspace
            .find_member(crate_name)
            .expect("crate_name vem do workspace");
        if let Ok(edges) = extract_imports(member, tree, &workspace_crate_names) {
            edges_per_crate.insert(crate_name.clone(), edges);
        }
    }

    // Fase 1.4: grafo
    let graph = build_graph(&workspace, &trees, &edges_per_crate);
    let cycles = detect_cycles(&graph);

    // Garantir directorios pai
    if let Some(parent) = cli
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty() && !p.exists())
    {
        std::fs::create_dir_all(parent).map_err(|e| PipelineError::WriteFailed {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    let tool_version = env!("CARGO_PKG_VERSION");
    let generated_at = current_rfc3339_timestamp();

    let graph_json = to_canonical_json(&graph, &cycles, &workspace, tool_version, &generated_at)?;
    std::fs::write(&cli.output, graph_json).map_err(|e| PipelineError::WriteFailed {
        path: cli.output.clone(),
        source: e,
    })?;

    let trees_path = if cli.emit_trees {
        let p = derive_trees_path(&cli.output);
        let trees_json = to_canonical_json_trees(&trees, &workspace, tool_version, &generated_at)?;
        std::fs::write(&p, trees_json).map_err(|e| PipelineError::WriteFailed {
            path: p.clone(),
            source: e,
        })?;
        Some(p)
    } else {
        None
    };

    let mut layer_violation_count = 0;
    let mut sarif_finding_count = 0;

    let html_path = if cli.emit_html {
        let p = derive_html_path(&cli.output);
        let partition = partition_for_dsm(&graph);

        let layer_violations = match resolve_config_path(cli)? {
            Some(config_path) => {
                let config = read_layer_config(&config_path, &workspace)?;
                detect_layer_violations(&graph, &config)
            }
            None => Vec::new(),
        };
        layer_violation_count = layer_violations.len();

        let sarif_findings = match &cli.sarif {
            Some(sarif_path) => {
                if !sarif_path.exists() {
                    return Err(PipelineError::SarifNotFound {
                        path: sarif_path.clone(),
                    });
                }
                read_sarif(sarif_path)?
            }
            None => Vec::new(),
        };
        sarif_finding_count = sarif_findings.len();

        let lv_opt = if layer_violations.is_empty() {
            None
        } else {
            Some(layer_violations.as_slice())
        };
        let sf_opt = if sarif_findings.is_empty() {
            None
        } else {
            Some(sarif_findings.as_slice())
        };

        let html = render_dsm_html(
            &graph,
            &partition,
            &cycles,
            &workspace,
            tool_version,
            &generated_at,
            lv_opt,
            sf_opt,
        )?;
        std::fs::write(&p, html).map_err(|e| PipelineError::WriteFailed {
            path: p.clone(),
            source: e,
        })?;
        Some(p)
    } else {
        None
    };

    Ok(PipelineReport {
        member_count: workspace.member_count(),
        module_count: trees.values().map(|t| t.node_count()).sum(),
        edge_count: graph.edge_count(),
        cycle_count: cycles.cycle_count(),
        output_path: cli.output.clone(),
        trees_path,
        html_path,
        layer_violation_count,
        sarif_finding_count,
    })
}

fn resolve_config_path(cli: &Cli) -> Result<Option<PathBuf>, PipelineError> {
    match &cli.config {
        Some(path) => {
            if path.exists() {
                Ok(Some(path.clone()))
            } else {
                Err(PipelineError::ConfigNotFound { path: path.clone() })
            }
        }
        None => {
            let default = cli.workspace_path.join("crystalline.toml");
            if default.exists() {
                Ok(Some(default))
            } else {
                Ok(None)
            }
        }
    }
}

/// Deriva o path do `trees.json` no mesmo diretorio que `--output`.
fn derive_trees_path(output_path: &Path) -> PathBuf {
    let parent = output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    parent.join("trees.json")
}

/// Deriva o path do `dsm.html` no mesmo diretorio que `--output`.
fn derive_html_path(output_path: &Path) -> PathBuf {
    let parent = output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    parent.join("dsm.html")
}

/// Timestamp UTC em RFC 3339 (formato `YYYY-MM-DDTHH:MM:SSZ`).
fn current_rfc3339_timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_trees_path_relative() {
        assert_eq!(
            derive_trees_path(Path::new("./graph.json")),
            PathBuf::from("./trees.json"),
        );
    }

    #[test]
    fn test_derive_trees_path_absolute() {
        assert_eq!(
            derive_trees_path(Path::new("/abs/path/output.json")),
            PathBuf::from("/abs/path/trees.json"),
        );
    }

    #[test]
    fn test_derive_trees_path_bare_filename() {
        assert_eq!(
            derive_trees_path(Path::new("nome.json")),
            PathBuf::from("./trees.json"),
        );
    }

    #[test]
    fn test_derive_html_path_relative() {
        assert_eq!(
            derive_html_path(Path::new("./graph.json")),
            PathBuf::from("./dsm.html"),
        );
    }

    #[test]
    fn test_derive_html_path_absolute() {
        assert_eq!(
            derive_html_path(Path::new("/abs/path/output.json")),
            PathBuf::from("/abs/path/dsm.html"),
        );
    }

    #[test]
    fn test_derive_html_path_bare_filename() {
        assert_eq!(
            derive_html_path(Path::new("nome.json")),
            PathBuf::from("./dsm.html"),
        );
    }

    #[test]
    fn test_current_rfc3339_format_shape() {
        let ts = current_rfc3339_timestamp();
        assert_eq!(ts.len(), 20, "formato esperado: YYYY-MM-DDTHH:MM:SSZ");
        let b = ts.as_bytes();
        assert_eq!(b[4], b'-');
        assert_eq!(b[7], b'-');
        assert_eq!(b[10], b'T');
        assert_eq!(b[13], b':');
        assert_eq!(b[16], b':');
        assert_eq!(b[19], b'Z');
    }

    #[test]
    fn test_run_pipeline_with_config_and_sarif() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_path = manifest
            .parent()
            .unwrap()
            .join("tests/fixtures/imports-simple");
        if !workspace_path.exists() {
            return;
        }

        let temp_dir = std::env::temp_dir().join(format!(
            "test_run_pipeline_{}",
            chrono::Utc::now().timestamp_micros()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let config_path = temp_dir.join("crystalline.toml");
        std::fs::write(
            &config_path,
            r#"[layers]
L1 = "imports-simple"
"#,
        )
        .unwrap();

        let sarif_path = temp_dir.join("sarif.json");
        std::fs::write(
            &sarif_path,
            r#"{
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "crystalline-lint"
        }
      },
      "results": [
        {
          "ruleId": "V9",
          "level": "error",
          "message": {
            "text": "Violation detected"
          },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": "imports-simple/src/lib.rs"
                },
                "region": {
                  "startLine": 1
                }
              }
            }
          ]
        }
      ]
    }
  ]
}"#,
        )
        .unwrap();

        let output_json = temp_dir.join("graph.json");

        let cli = Cli {
            workspace_path,
            output: output_json.clone(),
            emit_trees: false,
            emit_html: true,
            config: Some(config_path),
            sarif: Some(sarif_path),
        };

        let report = run_pipeline(&cli).unwrap();
        assert!(report.html_path.is_some());

        let html = std::fs::read_to_string(report.html_path.unwrap()).unwrap();
        assert!(html.contains("Crystalline DSM"));
        assert!(html.contains("1 lint finding"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
