//! Lineage: prompt 00_nucleo/prompt/0003-adaptador_l3.md
//!           consolidado por prompt 00_nucleo/prompt/0018-consolidar-invocador.md
//!
//! Invocação do fork como subprocesso a partir do **diretório do crate-alvo**.
//! Descobre o nome do pacote lendo o `Cargo.toml` (workspace → `--package`
//! obrigatório, laudo 0003 D3) e delega o subprocess para
//! [`crate::fork::invocar_em`] — a primitiva única do crate.
//!
//! O `--sysroot` é política da lente (ADR-0001, Limite 1 da spec) e vive
//! dentro do `fork::invocar_em`, não aqui.

use std::io::ErrorKind;
use std::path::Path;

use crate::ErroAdaptador;
use crate::fork::ErroFork;

/// Lê o `Cargo.toml` em `diretorio` e devolve o `name` da seção `[package]`.
///
/// Necessário porque, em workspace, `cargo modules` exige `--package <nome>`
/// para desambiguar — e este projeto-lente, depois do ADR-0003, é workspace.
/// O parsing é linha-a-linha (suficiente para o subset comum de TOML; evita
/// adicionar `toml` como dependência).
fn descobrir_pacote(diretorio: &Path) -> Result<String, ErroAdaptador> {
    let caminho_toml = diretorio.join("Cargo.toml");
    let conteudo = std::fs::read_to_string(&caminho_toml).map_err(|_| {
        ErroAdaptador::CargoTomlAusente(caminho_toml.to_string_lossy().into_owned())
    })?;

    let mut em_package = false;
    for linha in conteudo.lines() {
        let l = linha.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some(secao) = l.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            em_package = secao.trim() == "package";
            continue;
        }
        if em_package {
            if let Some(resto) = l.strip_prefix("name") {
                let resto = resto.trim_start().trim_start_matches('=').trim();
                if let Some(s) = resto.strip_prefix('"') {
                    if let Some(nome) = s.split('"').next() {
                        if !nome.is_empty() {
                            return Ok(nome.to_string());
                        }
                    }
                }
            }
        }
    }
    Err(ErroAdaptador::CargoTomlSemPackage(
        caminho_toml.to_string_lossy().into_owned(),
    ))
}

/// Descobre o pacote pelo `Cargo.toml` em `diretorio` e delega ao
/// `fork::invocar_em` (a primitiva única de subprocess).
pub(crate) fn invocar(diretorio: &Path) -> Result<String, ErroAdaptador> {
    let pacote = descobrir_pacote(diretorio)?;
    crate::fork::invocar_em(&pacote, Some(diretorio)).map_err(mapear_erro_fork)
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diretorio_inexistente_da_cargo_toml_ausente() {
        // Sem Cargo.toml legível -> falha na descoberta do pacote, antes do
        // subprocesso. Diagnostico claro.
        let inexistente = Path::new("/tmp/__lente_diretorio_inexistente__xyz");
        match invocar(inexistente).unwrap_err() {
            ErroAdaptador::CargoTomlAusente(_) => {}
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    #[test]
    fn descobre_pacote_de_cargo_toml_simples() {
        let dir = std::env::temp_dir().join("__lente_dp_simples__");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[package]
name = "exemplo"
version = "0.1.0"
edition = "2024"

[dependencies]
"#,
        )
        .unwrap();
        assert_eq!(descobrir_pacote(&dir).unwrap(), "exemplo");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_puro_sem_package_devolve_erro_claro() {
        let dir = std::env::temp_dir().join("__lente_dp_workspace__");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[workspace]
resolver = "2"
members = ["a", "b"]
"#,
        )
        .unwrap();
        match descobrir_pacote(&dir).unwrap_err() {
            ErroAdaptador::CargoTomlSemPackage(_) => {}
            outro => panic!("erro inesperado: {:?}", outro),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
