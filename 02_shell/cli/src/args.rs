//! Argumentos da CLI. Todos os textos `help`/`about` vêm do
//! `lente_catalogo` (ADR-0002).

use clap::Parser;

/// Definição clap. Note que **todos** os textos de ajuda são lidos do
/// catálogo (constantes resolvidas em tempo de compilação) — nenhum literal
/// de apresentação aqui.
#[derive(Parser, Debug, Clone)]
#[command(name = "lente", about = lente_catalogo::ABOUT_CLI)]
pub struct Cli {
    /// JSON pronto.
    #[arg(long, conflicts_with = "pacote", help = lente_catalogo::HELP_GRAFO)]
    pub grafo: Option<std::path::PathBuf>,

    /// Nome de pacote (a lente invoca o fork).
    #[arg(long, conflicts_with = "grafo", help = lente_catalogo::HELP_PACOTE)]
    pub pacote: Option<String>,

    /// Alvo por path.
    #[arg(long, conflicts_with = "alvo_id", help = lente_catalogo::HELP_ALVO)]
    pub alvo: Option<String>,

    /// Alvo por id (no grafo resolvido).
    #[arg(long = "alvo-id", conflicts_with = "alvo", help = lente_catalogo::HELP_ALVO_ID)]
    pub alvo_id: Option<usize>,

    /// Saída em texto humano-legível (default é JSON).
    #[arg(long, help = lente_catalogo::HELP_TEXT)]
    pub text: bool,

    /// Inclui lista completa de impactados.
    #[arg(long, help = lente_catalogo::HELP_VERBOSE)]
    pub verbose: bool,
}
