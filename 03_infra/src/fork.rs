//! Lineage: prompt 00_nucleo/prompt/0017-l3_invocador_fork.md
//!          ampliado por prompt 00_nucleo/prompt/0022-l3-invocador-bin-lib.md
//!          porta --pacote fechada por prompt 00_nucleo/prompt/0023-l3-deteccao-alvo-metadata.md
//! Camada:  L3 — Infraestrutura. Processo externo é I/O legítimo aqui.
//!
//! Invocador encapsulado do fork do `cargo-modules`: executa
//! `cargo modules export-json --sysroot --compact [--lib|--bin <nome>]
//! --package <pacote>` como subprocess e devolve o JSON cru como `String`.
//!
//! Esta peça **não desserializa** — produz só o texto. Quem desserializa é o
//! `traducao` (existente). Separação deliberada: o invocador é independente
//! da forma do JSON, e pode ser reutilizado pelo wiring (modo `--pacote`)
//! sem amarrar à pipeline completa.
//!
//! Limite 1 da spec: `--sysroot` é fixo (política do projeto, não opção do
//! chamador). A flag de alvo (`--lib`/`--bin`) é descoberta pelo módulo
//! [`crate::metadata`] (via `cargo metadata`) e flui como parâmetro para a
//! primitiva [`invocar_em`] — **único invocador do fork** no crate (laudo
//! 0018; reformulação do invariante no prompt 0023 — metadata é outro
//! subprocesso com outro propósito).

use core::error::Error;
use core::fmt;
use std::path::Path;
use std::process::Command;

/// Alvo de seleção do `cargo modules`: `--lib` ou `--bin <nome>`.
/// Calculado pela `invocacao::detectar_alvo` a partir do `Cargo.toml` + layout
/// do crate, e passado para a primitiva [`invocar_em`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AlvoFork {
    /// Analisa a biblioteca do pacote (`--lib`). Escolha natural da lente
    /// quando o pacote tem `[lib]` (com ou sem binário): a lente analisa
    /// estrutura de biblioteca.
    Lib,
    /// Analisa um binário específico (`--bin <nome>`). Usado quando o pacote
    /// não tem biblioteca e tem exatamente um binário.
    Bin(String),
}

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
    /// Falha na detecção de alvo via `cargo metadata` antes de chamar o fork.
    /// Embrulha [`crate::metadata::ErroMetadata`] (prompt 0023).
    DeteccaoAlvo(crate::metadata::ErroMetadata),
}

impl From<crate::metadata::ErroMetadata> for ErroFork {
    fn from(e: crate::metadata::ErroMetadata) -> Self {
        ErroFork::DeteccaoAlvo(e)
    }
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
            ErroFork::DeteccaoAlvo(e) => {
                write!(f, "detecção de alvo via cargo metadata: {}", e)
            }
        }
    }
}

impl Error for ErroFork {}

/// Invoca o fork e devolve o JSON cru. Roda no cwd do processo. **Cobre
/// bin+lib via descoberta por nome no `cargo metadata` do cwd** (prompt
/// 0023): o pacote é localizado por `name` no workspace que o cargo enxerga
/// a partir do cwd, e a flag de alvo (`--lib`/`--bin <nome>`) é decidida
/// pelos `targets[]` do metadata. Modo `--pacote` da CLI passa por aqui.
pub fn invocar_fork(pacote: &str) -> Result<String, ErroFork> {
    let alvo = crate::metadata::detectar_alvo_por_nome(pacote, None)?;
    invocar_em(pacote, None, Some(&alvo))
}

/// Versão interna que aceita diretório de trabalho e alvo opcionais.
/// **Esta é a única função do crate que invoca `cargo modules` (o fork)**
/// — a primitiva que `invocar_fork` (cwd herdado, alvo detectado por nome
/// no metadata) e `invocacao::invocar` (cwd fornecido, alvo detectado por
/// manifest_path no metadata) usam. O outro subprocesso do crate, em
/// [`crate::metadata`], roda `cargo metadata` (propósito diferente).
pub(crate) fn invocar_em(
    pacote: &str,
    current_dir: Option<&Path>,
    alvo: Option<&AlvoFork>,
) -> Result<String, ErroFork> {
    let mut cmd = Command::new("cargo");
    cmd.args(["modules", "export-json", "--sysroot", "--compact"]);
    match alvo {
        Some(AlvoFork::Lib) => {
            cmd.arg("--lib");
        }
        Some(AlvoFork::Bin(nome)) => {
            cmd.args(["--bin", nome]);
        }
        None => {}
    }
    cmd.args(["--package", pacote]);
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
    fn pacote_inexistente_retorna_erro_de_deteccao_de_alvo() {
        // Antes do prompt 0023: o fork seria chamado direto e devolveria
        // `StatusErro` ("package X not found in workspace"). Agora a
        // detecção via `cargo metadata` falha ANTES do fork com
        // `PacoteNaoEncontrado` — diagnóstico mais específico (sabemos que
        // o pacote nem existe no workspace; não chegamos ao fork).
        match invocar_fork("pacote_que_nao_existe_42") {
            Err(ErroFork::DeteccaoAlvo(crate::metadata::ErroMetadata::PacoteNaoEncontrado(n))) => {
                assert_eq!(n, "pacote_que_nao_existe_42");
            }
            Ok(_) => panic!("não deveria ter retornado Ok"),
            Err(outro) => panic!("variante inesperada: {:?}", outro),
        }
    }

    /// Display cobre todas as variantes (não precisa de subprocess).
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
        let v4 = ErroFork::DeteccaoAlvo(crate::metadata::ErroMetadata::PacoteNaoEncontrado(
            "x".to_string(),
        ));
        for v in [v1, v2, v3, v4].iter() {
            assert!(!format!("{}", v).is_empty());
        }
    }
}
