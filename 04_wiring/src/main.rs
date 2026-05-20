/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/cli.md
 * @layer L4
 * @updated 2026-05-20
 */

use clap::Parser;
use std::fs;
use std::path::Path;

pub mod graph_builder;

#[derive(Parser, Debug)]
#[command(name = "crystalline-dsm")]
#[command(version = "0.1.0")]
#[command(about = "Gera uma Dependency Structure Matrix (DSM) para projetos Rust", long_about = None)]
struct Cli {
    /// Caminho do workspace Cargo a ser analisado
    workspace_path: String,

    /// Caminho de destino para salvar a DSM
    #[arg(short, long, default_value = "./dsm.html")]
    output: String,

    /// Formato do relatório de saída (html ou json)
    #[arg(short, long, default_value = "html")]
    format: String,
}

fn main() {
    let cli = Cli::parse();

    // Invoca a Casca (L2) para formatar a mensagem de início
    let start_msg = crystalline_dsm_shell::format_start_analysis(&cli.workspace_path);
    println!("{}", start_msg);

    // Mock do Passo 0.3: cria um arquivo de saída simulado no caminho especificado
    let output_path = Path::new(&cli.output);

    // Cria as pastas pai caso não existam
    if let Some(parent) = output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty() && !p.exists())
    {
        fs::create_dir_all(parent).expect("Falha ao criar diretórios de saída");
    }

    // Escreve um arquivo de conteúdo mockado (JSON ou HTML dependendo do formato)
    let mock_content = match cli.format.to_lowercase().as_str() {
        "json" => r#"{"modules":[],"edges":[]}"#,
        _ => "<html><body><h1>DSM Mock</h1></body></html>",
    };

    fs::write(output_path, mock_content).expect("Falha ao escrever arquivo de saída");

    // Invoca a Casca (L2) para formatar a mensagem de sucesso
    let success_msg = crystalline_dsm_shell::format_success(&cli.output);
    println!("{}", success_msg);
}
