//! Lineage: prompt 00_nucleo/prompt/0003-adaptador_l3.md
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0001-fonte-do-grafo-fork-externo.md
//!          00_nucleo/adr/0002-modelagem-do-grafo.md
//!          00_nucleo/adr/0003-workspace-cargo.md
//! Camada:  L3 — Infraestrutura. I/O e dependências externas permitidos.
//!
//! Adaptador da fonte: invoca o fork do `cargo-modules` num crate-alvo,
//! captura o JSON do `export-json --sysroot --compact`, deserializa em DTOs,
//! e traduz para `lente_core::Grafo`, validando os enums e os invariantes
//! da spec.
//!
//! Fronteira honesta: o fork é invocado como subprocesso (não importado como
//! biblioteca) — fronteira é processo, não API.

#![forbid(unsafe_code)]

use core::error::Error;
use core::fmt;

use lente_core::entities::grafo::{Grafo, ValorDesconhecido};

mod dto;
mod invocacao;
mod metadata;
mod traducao;
pub mod fork;

pub use metadata::ErroMetadata;

/// Modos de falha do adaptador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroAdaptador {
    /// O binário `cargo` não foi encontrado no PATH.
    BinarioNaoEncontrado,
    /// Falha de I/O ao tentar iniciar o subprocesso (ex.: diretório inválido).
    FalhaSubprocesso(String),
    /// O subprocesso terminou com código não-zero.
    /// Inclui stderr para diagnóstico (ex.: "no such subcommand: modules"
    /// quando o fork não está instalado).
    SubprocessoFalhou {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// stdout do subprocesso não é UTF-8 válido.
    SaidaNaoUtf8(String),
    /// O JSON capturado não parseia.
    JsonInvalido(String),
    /// Texto fora da lista fechada em `kind`/`visibility`/`relation`.
    ValorDesconhecido(ValorDesconhecido),
    /// Invariante (id único) violado: dois nós com mesmo `id`. Bug do fork.
    IdDuplicado(usize),
    /// Invariante (integridade referencial) violado: aresta referencia `id`
    /// ausente em `nodes`. `contexto` indica se é o lado `id_from` ou `id_to`.
    IdReferenciado { id: usize, contexto: String },
    /// Falha na descoberta do alvo via `cargo metadata` (prompt 0023).
    /// Engloba: cargo ausente, metadata com status ≠ 0 (manifesto não
    /// resolve), JSON inesperado, pacote-pedido inexistente no workspace,
    /// pacote sem [lib] e com 0/≥2 bins. A variante embrulhada preserva
    /// o modo de falha específico (vide [`ErroMetadata`]).
    DeteccaoAlvo(ErroMetadata),
}

impl From<ErroMetadata> for ErroAdaptador {
    fn from(e: ErroMetadata) -> Self {
        ErroAdaptador::DeteccaoAlvo(e)
    }
}

impl fmt::Display for ErroAdaptador {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErroAdaptador::BinarioNaoEncontrado => {
                f.write_str("binário `cargo` não encontrado no PATH")
            }
            ErroAdaptador::FalhaSubprocesso(m) => {
                write!(f, "falha ao iniciar subprocesso: {}", m)
            }
            ErroAdaptador::SubprocessoFalhou { exit_code, stderr } => {
                write!(
                    f,
                    "cargo modules export-json falhou (exit {:?}): {}",
                    exit_code, stderr
                )
            }
            ErroAdaptador::SaidaNaoUtf8(m) => {
                write!(f, "saída do subprocesso não é UTF-8: {}", m)
            }
            ErroAdaptador::JsonInvalido(m) => write!(f, "JSON inválido: {}", m),
            ErroAdaptador::ValorDesconhecido(v) => {
                write!(f, "valor desconhecido na borda: {}", v)
            }
            ErroAdaptador::IdDuplicado(id) => {
                write!(f, "invariante violado — id duplicado: {}", id)
            }
            ErroAdaptador::IdReferenciado { id, contexto } => {
                write!(
                    f,
                    "invariante violado — aresta referencia id inexistente ({}={})",
                    contexto, id
                )
            }
            ErroAdaptador::DeteccaoAlvo(e) => {
                write!(f, "detecção de alvo via cargo metadata: {}", e)
            }
        }
    }
}

impl Error for ErroAdaptador {}

/// Extrai o grafo de dependências de um crate Rust.
///
/// `caminho_crate` é o diretório do crate-alvo (que contém `Cargo.toml`).
/// Invoca `cargo modules export-json --sysroot --compact` ali e desserializa.
pub fn extrair_grafo(caminho_crate: &std::path::Path) -> Result<Grafo, ErroAdaptador> {
    let json = invocacao::invocar(caminho_crate)?;
    desserializar_grafo(&json)
}

/// Desserializa um JSON do fork (`cargo-modules export-json` 0.27.0) num
/// `Grafo` validado. Fachada limpa para chamadores externos (ex.: L4 wiring)
/// que já têm o JSON (lido de arquivo ou capturado de `fork::invocar_fork`)
/// e não precisam lidar com `serde` nem com o formato cru do DTO.
pub fn desserializar_grafo(json: &str) -> Result<Grafo, ErroAdaptador> {
    let dto: dto::GrafoDTO = serde_json::from_str(json)
        .map_err(|e| ErroAdaptador::JsonInvalido(e.to_string()))?;
    traducao::traduzir(dto)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_invalido_resulta_em_erro_diagnosticavel() {
        // Caminho de "atalho": passamos um JSON corrompido pelo parsing,
        // exercitando o ramo de JsonInvalido sem subprocess. (Função privada;
        // simulamos chamando direto serde_json + traducao seria mais limpo,
        // mas o teste de invocacao já cobre o ramo de subprocesso.)
        let json_cru = "{ isso não é JSON válido";
        let r: Result<dto::GrafoDTO, _> = serde_json::from_str(json_cru);
        assert!(r.is_err());
    }

    #[test]
    fn desserializar_grafo_valido_devolve_grafo() {
        let json = r#"{
            "crate": "t",
            "nodes": [
                {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"}
            ],
            "edges": []
        }"#;
        let g = desserializar_grafo(json).expect("JSON válido deve traduzir");
        assert_eq!(g.crate_name, "t");
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.nodes[0].id, 1);
    }

    #[test]
    fn desserializar_grafo_invalido_retorna_erro_de_json() {
        match desserializar_grafo("{ não é JSON").unwrap_err() {
            ErroAdaptador::JsonInvalido(_) => {}
            outro => panic!("esperava JsonInvalido, veio {:?}", outro),
        }
    }

    #[test]
    fn erro_implementa_display_para_cada_variante() {
        // Sanity: cada variante produz uma string não-vazia.
        let variantes = [
            ErroAdaptador::BinarioNaoEncontrado,
            ErroAdaptador::FalhaSubprocesso("erro".to_string()),
            ErroAdaptador::SubprocessoFalhou {
                exit_code: Some(1),
                stderr: "msg".to_string(),
            },
            ErroAdaptador::SaidaNaoUtf8("byte".to_string()),
            ErroAdaptador::JsonInvalido("eof".to_string()),
            ErroAdaptador::IdDuplicado(7),
            ErroAdaptador::IdReferenciado {
                id: 99,
                contexto: "id_from".to_string(),
            },
            ErroAdaptador::DeteccaoAlvo(ErroMetadata::PacoteNaoEncontrado(
                "x".to_string(),
            )),
            ErroAdaptador::DeteccaoAlvo(ErroMetadata::AlvosAmbiguos {
                bins: vec!["a".to_string(), "b".to_string()],
            }),
            ErroAdaptador::DeteccaoAlvo(ErroMetadata::AlvosAmbiguos { bins: vec![] }),
        ];
        for v in &variantes {
            assert!(!format!("{}", v).is_empty());
        }
    }

    /// Teste end-to-end: extrai o grafo do fixture interno
    /// `tests/fixtures/crate-amostra` (excluído do workspace).
    ///
    /// Requer o fork instalado e no PATH. Por isso `#[ignore]` — rodar com
    /// `cargo test -- --ignored` quando o ambiente estiver configurado.
    #[test]
    #[ignore]
    fn e2e_extrai_grafo_de_fixture() {
        let alvo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/crate-amostra");

        let g = extrair_grafo(&alvo).expect("extração deve funcionar");

        assert!(!g.nodes.is_empty(), "grafo não-vazio");
        assert_eq!(g.crate_name, "crate_amostra");

        // Identidade por id (invariante revisado): id deve ser único.
        let mut ids: Vec<_> = g.nodes.iter().map(|n| n.id).collect();
        ids.sort();
        let total = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), total, "ids únicos");

        // Estrutura esperada do fixture: nó-raiz + módulos parser e runner.
        assert!(
            g.nodes.iter().any(|n| n.path.as_str() == "crate_amostra"),
            "nó-raiz presente"
        );
        assert!(
            g.nodes
                .iter()
                .any(|n| n.path.as_str() == "crate_amostra::parser"),
            "módulo parser presente"
        );
        assert!(
            g.nodes
                .iter()
                .any(|n| n.path.as_str() == "crate_amostra::runner"),
            "módulo runner presente"
        );
    }

    /// Teste end-to-end **novo** (este prompt): extrai o grafo do próprio
    /// `lente_core`, que antes era rejeitado por colisão de path em
    /// `ErroRaio::fmt`. Agora deve passar — paths colidentes são dados
    /// legítimos, discriminados por `id`.
    ///
    /// Verifica explicitamente: pelo menos um path aparece em 2+ nós, e
    /// todos os ids são distintos. É a verificação real de que a mudança
    /// do prompt 0006 funcionou ponta-a-ponta.
    #[test]
    #[ignore]
    fn e2e_extrai_grafo_de_lente_core_com_colisao_de_path() {
        let alvo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("01_core");

        let g = extrair_grafo(&alvo).expect("extração de lente_core deve funcionar agora");

        assert_eq!(g.crate_name, "lente_core");
        assert!(g.nodes.len() > 10, "grafo deve ter pelo menos uma dezena de nós");

        // Invariante novo: ids únicos.
        use std::collections::HashSet;
        let ids: HashSet<usize> = g.nodes.iter().map(|n| n.id).collect();
        assert_eq!(ids.len(), g.nodes.len(), "ids únicos");

        // Mudança de semântica: pelo menos uma colisão de path existe
        // (`ErroRaio::fmt` Display+Debug). Antes era erro; agora é dado.
        let mut por_path: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for n in &g.nodes {
            *por_path.entry(n.path.as_str().to_string()).or_insert(0) += 1;
        }
        let colisoes: Vec<_> = por_path.iter().filter(|(_, c)| **c > 1).collect();
        assert!(
            !colisoes.is_empty(),
            "esperava ao menos uma colisão de path em lente_core"
        );

        // O caso específico que motivou o prompt 0006.
        assert!(
            por_path
                .get("lente_core::domain::raio::ErroRaio::fmt")
                .copied()
                .unwrap_or(0)
                >= 2,
            "lente_core::domain::raio::ErroRaio::fmt deve colidir (Display+Debug)"
        );

        // Prompt 0013: as duas cópias de ErroRaio::fmt agora trazem `trait_`
        // distinto direto do fork 0.27.0 — a matéria-prima que resolve a D4
        // (a nomeação por trait com id correto, sem adivinhação).
        let traits_fmt: std::collections::HashSet<String> = g
            .nodes
            .iter()
            .filter(|n| n.path.as_str() == "lente_core::domain::raio::ErroRaio::fmt")
            .filter_map(|n| n.trait_.clone())
            .collect();
        assert!(
            traits_fmt.contains("Display") && traits_fmt.contains("Debug"),
            "as duas cópias devem ter trait_ Display e Debug, veio {:?}",
            traits_fmt
        );
    }
}
