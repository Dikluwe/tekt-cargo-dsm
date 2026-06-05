//! Lineage: prompt 00_nucleo/prompt/0003-adaptador_l3.md
//!           consolidado por prompt 00_nucleo/prompt/0018-consolidar-invocador.md
//!           ampliado por prompt 00_nucleo/prompt/0022-l3-invocador-bin-lib.md
//!           detecção migrada para metadata por prompt 0023-l3-deteccao-alvo-metadata.md
//!
//! Invocação do fork como subprocesso a partir do **diretório do crate-alvo**.
//! Descobre nome do pacote E alvo (`--lib`/`--bin <nome>`) consultando
//! `cargo metadata` (fonte autoritativa) via [`crate::metadata`], e delega o
//! subprocess de `export-json` para [`crate::fork::invocar_em`] — a primitiva
//! única do crate para o fork.
//!
//! O `--sysroot` é política da lente (ADR-0001, Limite 1 da spec) e vive
//! dentro do `fork::invocar_em`, não aqui. A heurística do laudo 0022
//! (parser TOML + layout `src/`) foi removida — a fragilidade da D4
//! daquele laudo está coberta pela fonte autoritativa do Cargo.

use std::io::ErrorKind;
use std::path::Path;

use crate::ErroAdaptador;
use crate::fork::ErroFork;

/// Descobre pacote e alvo pelo `cargo metadata` em `diretorio` e delega ao
/// `fork::invocar_em` (a primitiva única de subprocess do fork).
pub(crate) fn invocar(diretorio: &Path) -> Result<String, ErroAdaptador> {
    let (pacote, alvo) = crate::metadata::detectar_pacote_e_alvo_por_diretorio(diretorio)?;
    crate::fork::invocar_em(&pacote, Some(diretorio), Some(&alvo)).map_err(mapear_erro_fork)
}

/// Mapeia `ErroFork` para `ErroAdaptador` preservando as variantes existentes.
/// Mantém o contrato externo do `ErroAdaptador` (não-regressão dos testes).
fn mapear_erro_fork(e: ErroFork) -> ErroAdaptador {
    match e {
        ErroFork::FalhaSubprocess(io_err) if io_err.kind() == ErrorKind::NotFound => {
            ErroAdaptador::BinarioNaoEncontrado
        }
        ErroFork::FalhaSubprocess(io_err) => {
            ErroAdaptador::FalhaSubprocesso(io_err.to_string())
        }
        ErroFork::StatusErro { codigo, stderr } => ErroAdaptador::SubprocessoFalhou {
            exit_code: codigo,
            stderr,
        },
        ErroFork::StdoutInvalido(utf8_err) => {
            ErroAdaptador::SaidaNaoUtf8(utf8_err.to_string())
        }
        ErroFork::DeteccaoAlvo(e) => ErroAdaptador::DeteccaoAlvo(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Diretório inexistente: a porta de detecção curto-circuita com
    /// `DiretorioInexistente` ANTES do spawn (prompt 0024, fecha D5 do
    /// 0023). Não depende de cargo no PATH — vira teste de unidade
    /// determinístico (sem `#[ignore]`).
    #[test]
    fn diretorio_inexistente_da_diretorio_inexistente() {
        let inexistente = Path::new("/tmp/__lente_diretorio_inexistente_0024__xyz");
        match invocar(inexistente).unwrap_err() {
            ErroAdaptador::DeteccaoAlvo(crate::metadata::ErroMetadata::DiretorioInexistente(
                p,
            )) => {
                assert_eq!(p, inexistente);
            }
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    /// E2E migrado do laudo 0022: bin+lib via `extrair_grafo`
    /// (porta `invocacao::invocar`) deve funcionar. Antes do 0022, falhava
    /// por falta de flag; depois do 0022, funcionava pela heurística;
    /// agora (0023), funciona pela fonte autoritativa do `cargo metadata`.
    #[test]
    #[ignore]
    fn e2e_bin_mais_lib_via_extrair_grafo() {
        let dir = std::env::temp_dir().join("__lente_e2e_binlib_0023__");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[workspace]

[package]
name = "binlib_e2e_0023"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
path = "src/lib.rs"

[[bin]]
name = "binlib_e2e_0023"
path = "src/main.rs"
"#,
        )
        .unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn hello() -> &'static str { \"hi\" }").unwrap();
        std::fs::write(
            dir.join("src/main.rs"),
            "fn main() { println!(\"{}\", binlib_e2e_0023::hello()); }",
        )
        .unwrap();
        let json = invocar(&dir).expect("bin+lib via metadata deve produzir JSON");
        assert!(
            json.contains("\"crate\":\"binlib_e2e_0023\""),
            "JSON deve ter o crate-raiz; veio: {}",
            &json[..json.len().min(120)]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
