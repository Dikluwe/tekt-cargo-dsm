//! Lineage: prompt 00_nucleo/prompt/0017-l3_invocador_fork.md
//! Camada:  L3 — Infraestrutura. Processo externo é I/O legítimo aqui.
//!
//! Invocador encapsulado do fork do `cargo-modules`: executa
//! `cargo modules export-json --sysroot --compact --package <pacote>`
//! como subprocess **no diretório atual** e devolve o JSON cru como `String`.
//!
//! Esta peça **não desserializa** — produz só o texto. Quem desserializa é o
//! `traducao` (existente). Separação deliberada: o invocador é independente
//! da forma do JSON, e pode ser reutilizado pelo wiring (modo `--pacote`)
//! sem amarrar à pipeline completa.
//!
//! Limite 1 da spec: `--sysroot` é fixo (política do projeto, não opção do
//! chamador).

use core::error::Error;
use core::fmt;
use std::path::Path;
use std::process::Command;

/// Modos de falha do invocador do fork.
#[derive(Debug)]
pub enum ErroFork {
    /// Falha ao iniciar o subprocess: cargo ausente do PATH, permissão, etc.
    FalhaSubprocess(std::io::Error),
    /// Subprocess executou mas terminou com status de erro; `stderr` carrega
    /// a mensagem do fork (ex.: "package X not found in workspace").
    StatusErro {
        codigo: Option<i32>,
        stderr: String,
    },
    /// `stdout` do subprocess não é UTF-8 válido.
    StdoutInvalido(std::string::FromUtf8Error),
}

impl fmt::Display for ErroFork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErroFork::FalhaSubprocess(e) => {
                write!(
                    f,
                    "falha ao iniciar `cargo modules` (cargo no PATH? \
                     fork instalado?): {}",
                    e
                )
            }
            ErroFork::StatusErro { codigo, stderr } => {
                write!(
                    f,
                    "`cargo modules export-json` falhou (exit {:?}): {}",
                    codigo,
                    stderr.trim()
                )
            }
            ErroFork::StdoutInvalido(e) => {
                write!(f, "stdout do fork não é UTF-8: {}", e)
            }
        }
    }
}

impl Error for ErroFork {}

/// Invoca o fork e devolve o JSON cru. Roda no cwd do processo.
///
/// Comando exato: `cargo modules export-json --sysroot --compact --package
/// <pacote>`. Para usar contra um workspace específico (mudando o `cwd` só
/// para esta invocação), use [`invocar_em`] (visibilidade de crate).
pub fn invocar_fork(pacote: &str) -> Result<String, ErroFork> {
    invocar_em(pacote, None)
}

/// Versão interna que aceita um diretório de trabalho opcional. **Esta é a
/// única função do crate que roda `Command::new("cargo")`** — a primitiva
/// que `invocar_fork` (cwd herdado) e `invocacao::invocar` (cwd fornecido,
/// para `extrair_grafo`) usam.
pub(crate) fn invocar_em(
    pacote: &str,
    current_dir: Option<&Path>,
) -> Result<String, ErroFork> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "modules",
        "export-json",
        "--sysroot",
        "--compact",
        "--package",
        pacote,
    ]);
    if let Some(d) = current_dir {
        cmd.current_dir(d);
    }

    let saida = cmd.output().map_err(ErroFork::FalhaSubprocess)?;

    if !saida.status.success() {
        let stderr = String::from_utf8_lossy(&saida.stderr).into_owned();
        return Err(ErroFork::StatusErro {
            codigo: saida.status.code(),
            stderr,
        });
    }

    String::from_utf8(saida.stdout).map_err(ErroFork::StdoutInvalido)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sanidade ponta-a-ponta: invoca o fork no workspace atual e confirma
    /// que o JSON é parseável e contém o nó-raiz do crate-alvo.
    /// Requer fork instalado e `cargo test` rodado da raiz do workspace.
    #[test]
    #[ignore]
    fn invoca_fork_no_lente_core_devolve_json_valido() {
        let json = invocar_fork("lente_core").expect("fork deve rodar com sucesso");
        let v: serde_json::Value =
            serde_json::from_str(&json).expect("saída deve ser JSON válido");
        // Sanidade: o crate-raiz aparece no JSON.
        let crate_name = v["crate"].as_str().unwrap_or("");
        assert_eq!(crate_name, "lente_core");
        // E o tipo conhecido por carregar colisão de path está lá.
        let texto = json.as_str();
        assert!(
            texto.contains("ErroRaio"),
            "JSON deve conter o nó ErroRaio (sanidade do crate certo)"
        );
    }

    /// Pacote inexistente: o fork retorna exit code != 0 e mensagem no stderr.
    #[test]
    #[ignore]
    fn pacote_inexistente_retorna_status_erro_com_mensagem() {
        match invocar_fork("pacote_que_nao_existe_42") {
            Err(ErroFork::StatusErro { stderr, .. }) => {
                assert!(
                    !stderr.is_empty(),
                    "stderr deve trazer mensagem útil do fork"
                );
            }
            Ok(_) => panic!("não deveria ter retornado Ok"),
            Err(outro) => panic!("variante inesperada: {:?}", outro),
        }
    }

    /// Display cobre as três variantes (não precisa de subprocess).
    #[test]
    fn erro_implementa_display_para_cada_variante() {
        let v1 = ErroFork::FalhaSubprocess(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "x",
        ));
        let v2 = ErroFork::StatusErro {
            codigo: Some(1),
            stderr: "msg".to_string(),
        };
        // FromUtf8Error é difícil de construir diretamente; usamos bytes inválidos.
        let v3 = ErroFork::StdoutInvalido(String::from_utf8(vec![0xff, 0xfe]).unwrap_err());
        for v in [v1, v2, v3].iter() {
            assert!(!format!("{}", v).is_empty());
        }
    }
}
