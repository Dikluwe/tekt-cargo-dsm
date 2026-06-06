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
    #[arg(
        long,
        conflicts_with_all = ["alvo_id", "ranking", "estrutura"],
        help = lente_catalogo::HELP_ALVO,
    )]
    pub alvo: Option<String>,

    /// Alvo por id (no grafo resolvido).
    #[arg(
        long = "alvo-id",
        conflicts_with_all = ["alvo", "ranking", "estrutura"],
        help = lente_catalogo::HELP_ALVO_ID,
    )]
    pub alvo_id: Option<usize>,

    /// Modo ranking (prompt 0027): top-N por impacto no pacote.
    #[arg(
        long,
        conflicts_with_all = ["alvo", "alvo_id", "estrutura"],
        help = lente_catalogo::HELP_RANKING,
    )]
    pub ranking: bool,

    /// N do top-N do ranking. Default 10. Só faz sentido com `--ranking`.
    #[arg(long, default_value_t = 10, help = lente_catalogo::HELP_TOP)]
    pub top: usize,

    /// Modo estrutura (prompt 0031): vista global do pacote — módulos,
    /// dependências e ciclos. Mutuamente exclusivo com os outros modos.
    #[arg(
        long,
        conflicts_with_all = ["alvo", "alvo_id", "ranking", "diff"],
        help = lente_catalogo::HELP_ESTRUTURA,
    )]
    pub estrutura: bool,

    /// Modo diff (prompt 0047): mapeia o diff do repositório aos nós tocados
    /// e emite o impacto em JSON. Opera na raiz do repo (não usa
    /// `--grafo`/`--pacote`). Mutuamente exclusivo com os outros modos.
    #[arg(
        long,
        conflicts_with_all = ["alvo", "alvo_id", "ranking", "estrutura", "grafo", "pacote"],
        help = lente_catalogo::HELP_DIFF,
    )]
    pub diff: bool,

    /// Raiz do repositório no modo `--diff` (default: diretório atual).
    #[arg(long, help = lente_catalogo::HELP_REPO)]
    pub repo: Option<std::path::PathBuf>,

    /// Modo de inclusão das arestas `Uses` no `--estrutura` (prompt 0034):
    /// presente = só `Uses` de referência (uso de tipo direto). Ausente =
    /// todas as `Uses` (default). Ortogonal aos outros flags; só tem
    /// efeito no modo estrutura.
    #[arg(long = "so-referencia", help = lente_catalogo::HELP_SO_REFERENCIA)]
    pub so_referencia: bool,

    /// Escopo do grafo: presente = `SeuCodigo` (filtra stdlib),
    /// ausente = `Completo` (default — inclui stdlib). Ortogonal a
    /// `--ranking`/`--alvo`/`--alvo-id`. Prompt 0030.
    #[arg(long = "filtrar-stdlib", help = lente_catalogo::HELP_FILTRAR_STDLIB)]
    pub filtrar_stdlib: bool,

    /// Saída em texto humano-legível (default é JSON).
    #[arg(long, help = lente_catalogo::HELP_TEXT)]
    pub text: bool,

    /// Inclui lista completa de impactados.
    #[arg(long, help = lente_catalogo::HELP_VERBOSE)]
    pub verbose: bool,
}
