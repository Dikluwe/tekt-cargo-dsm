/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/cli.md
 * @prompt 00_nucleo/prompts/cli_output_flags.md
 * @layer L4
 * @updated 2026-05-20
 */

use clap::Parser;
use crystalline_dsm_core::entities::import_edge::ImportEdge;
use crystalline_dsm_core::entities::module_tree::ModuleTree;
use crystalline_dsm_core::rules::cycle_detector::detect_cycles;
use crystalline_dsm_infra::cargo_metadata_reader::{CargoMetadataError, read_workspace};
use crystalline_dsm_infra::import_extractor::extract_imports;
use crystalline_dsm_infra::json_serializer::{JsonSerializeError, to_canonical_json};
use crystalline_dsm_infra::module_traverser::traverse_crate;
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
}

#[derive(Debug)]
pub struct PipelineReport {
    pub member_count: usize,
    pub module_count: usize,
    pub edge_count: usize,
    pub cycle_count: usize,
    pub output_path: PathBuf,
    pub trees_path: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Falha ao ler workspace: {0}")]
    WorkspaceError(#[from] CargoMetadataError),

    #[error("Falha ao serializar grafo JSON: {0}")]
    JsonError(#[from] JsonSerializeError),

    #[error("Falha ao serializar trees.json: {0}")]
    TreesError(#[from] TreesSerializeError),

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
        crystalline_dsm_shell::format_start_analysis(
            &cli.workspace_path.display().to_string()
        )
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

    let graph_json =
        to_canonical_json(&graph, &cycles, &workspace, tool_version, &generated_at)?;
    std::fs::write(&cli.output, graph_json).map_err(|e| PipelineError::WriteFailed {
        path: cli.output.clone(),
        source: e,
    })?;

    let trees_path = if cli.emit_trees {
        let p = derive_trees_path(&cli.output);
        let trees_json =
            to_canonical_json_trees(&trees, &workspace, tool_version, &generated_at)?;
        std::fs::write(&p, trees_json).map_err(|e| PipelineError::WriteFailed {
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
    })
}

/// Deriva o path do `trees.json` no mesmo diretorio que `--output`.
fn derive_trees_path(output_path: &Path) -> PathBuf {
    let parent = output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    parent.join("trees.json")
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
}
