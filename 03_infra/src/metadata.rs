//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-metadata.md
//! @prompt-hash 93c70d4d
//! @layer L3
//! @updated 2026-06-07
//!          diagnóstico de diretório por prompt 00_nucleo/prompt/0024-l3-diretorio-inexistente-diagnostico.md
//! Camada:  L3 — Infraestrutura. Subprocesso é I/O legítimo aqui.
//!
//! Descoberta de alvos por `cargo metadata` — fonte autoritativa do Cargo.
//! Substitui a heurística do laudo 0022 (parser TOML + layout `src/`) por
//! consulta ao metadata, eliminando o falso-positivo `AlvosAmbiguos` em
//! crates exóticos (`autobins = false`, paths customizados) e fechando a
//! porta `--pacote` (`fork::invocar_fork`) para bin+lib: a descoberta por
//! nome funciona a partir do cwd, sem precisar do diretório do crate.
//!
//! **Subprocesso único de metadata do crate** — irmão do `fork::invocar_em`
//! para `export-json`. Cada um único, propósitos distintos: este descobre
//! alvos; o do `fork` extrai o grafo. (Reformulação do invariante "um só
//! `Command::new`" do laudo 0022, explícita no prompt 0023.)

use core::error::Error;
use core::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::fork::AlvoFork;

/// Saída mínima de `cargo metadata --no-deps --format-version 1`: só os
/// campos que a lente lê. Demais campos do JSON são ignorados (serde
/// descarta o que não há no struct).
#[derive(Debug, Deserialize)]
pub(crate) struct MetadataOutput {
    pub packages: Vec<MetadataPackage>,
}

/// Um pacote no workspace. Identificado por `name` (descoberta por nome,
/// porta `--pacote`) ou por `manifest_path` (descoberta por diretório,
/// porta `extrair_grafo`).
#[derive(Debug, Deserialize)]
pub(crate) struct MetadataPackage {
    pub name: String,
    pub manifest_path: String,
    pub targets: Vec<MetadataTarget>,
}

/// Um alvo do pacote (lib/bin/proc-macro/example/test/bench/custom-build).
/// `kind` é uma lista porque o Cargo permite múltiplas formas para o mesmo
/// alvo (raro em prática; a lente lê a primeira que case).
#[derive(Debug, Deserialize)]
pub(crate) struct MetadataTarget {
    pub name: String,
    pub kind: Vec<String>,
}

/// `kind`s que contam como biblioteca para a lente.
///
/// `proc-macro` entra porque (verificado na Fase 1 do prompt 0023)
/// `cargo modules export-json --lib` num crate proc-macro funciona; o
/// grafo da macro é estrutura de biblioteca como qualquer outra.
const KINDS_LIB: &[&str] = &["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"];
const KIND_BIN: &str = "bin";

/// Modos de falha da descoberta de alvos.
///
/// Tipo próprio (não `ErroAdaptador` nem `ErroFork`) para permitir que ambas
/// as portas do invocador (`invocacao::invocar` e `fork::invocar_fork`)
/// embrulhem-no nos seus respectivos erros sem duplicar a lógica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroMetadata {
    /// `cargo` não está no PATH (igual ao caso do fork).
    BinarioNaoEncontrado,
    /// Falha de I/O ao iniciar o subprocesso de metadata.
    FalhaSubprocess(String),
    /// `cargo metadata` rodou mas terminou com status ≠ 0. Stderr embalado
    /// para diagnóstico (ex.: "could not find `Cargo.toml`").
    StatusErro {
        codigo: Option<i32>,
        stderr: String,
    },
    /// stdout do metadata não é UTF-8 válido.
    StdoutInvalido(String),
    /// JSON do metadata não bate com o schema esperado.
    JsonInvalido(String),
    /// O nome de pacote pedido não existe no workspace que o metadata enxerga.
    /// Modo de falha da porta `--pacote` quando o usuário digitou nome errado.
    PacoteNaoEncontrado(String),
    /// Não há, no metadata, pacote cujo `manifest_path` aponte para o diretório
    /// pedido. Modo de falha da porta `extrair_grafo` quando o caminho dado
    /// não é o diretório de um crate (workspace puro, diretório qualquer).
    PacoteNoDiretorioNaoEncontrado(String),
    /// O pacote existe e tem alvos, mas nenhum é lib e há 0 ou ≥2 binários.
    /// Caso de borda reconhecido do prompt 0022 (D3 preservada): a lente
    /// analisa estrutura de biblioteca; quando só há binários e há mais de
    /// um, não há escolha automática segura.
    AlvosAmbiguos { bins: Vec<String> },
    /// O diretório passado para a porta de detecção por diretório não
    /// existe (ou não é diretório). Detectado **antes** do spawn do
    /// subprocesso, para não colidir com `BinarioNaoEncontrado` ("cargo
    /// ausente do PATH"): ambos seriam `io::ErrorKind::NotFound` se a
    /// checagem fosse só pelo `Command::output()`. Fecha a D5 do laudo
    /// 0023 (prompt 0024). Causa distinta de
    /// [`PacoteNoDiretorioNaoEncontrado`] (lá o diretório existe mas
    /// nenhum pacote do metadata casa).
    DiretorioInexistente(PathBuf),
}

impl fmt::Display for ErroMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErroMetadata::BinarioNaoEncontrado => {
                f.write_str("binário `cargo` não encontrado no PATH")
            }
            ErroMetadata::FalhaSubprocess(m) => {
                write!(f, "falha ao iniciar `cargo metadata`: {}", m)
            }
            ErroMetadata::StatusErro { codigo, stderr } => {
                write!(
                    f,
                    "`cargo metadata` falhou (exit {:?}): {}",
                    codigo,
                    stderr.trim()
                )
            }
            ErroMetadata::StdoutInvalido(m) => {
                write!(f, "stdout do `cargo metadata` não é UTF-8: {}", m)
            }
            ErroMetadata::JsonInvalido(m) => {
                write!(f, "JSON do `cargo metadata` inesperado: {}", m)
            }
            ErroMetadata::PacoteNaoEncontrado(n) => {
                write!(
                    f,
                    "pacote `{}` não existe no workspace (cargo metadata não o lista)",
                    n
                )
            }
            ErroMetadata::PacoteNoDiretorioNaoEncontrado(p) => {
                write!(
                    f,
                    "não há pacote cujo Cargo.toml seja {} (diretório não é crate?)",
                    p
                )
            }
            ErroMetadata::AlvosAmbiguos { bins } => {
                if bins.is_empty() {
                    f.write_str(
                        "pacote sem alvo analisável: nem biblioteca nem binário",
                    )
                } else {
                    write!(
                        f,
                        "pacote sem biblioteca e com múltiplos binários — \
                         a lente analisa estrutura de biblioteca; \
                         binários encontrados: {}",
                        bins.join(", ")
                    )
                }
            }
            ErroMetadata::DiretorioInexistente(p) => {
                write!(f, "diretório não existe: {}", p.display())
            }
        }
    }
}

impl Error for ErroMetadata {}

/// Roda `cargo metadata --no-deps --format-version 1` no `current_dir` (ou
/// cwd herdado) e desserializa para `MetadataOutput`. **Único `Command::new`
/// de metadata do crate.**
pub(crate) fn invocar_metadata(current_dir: Option<&Path>) -> Result<MetadataOutput, ErroMetadata> {
    let mut cmd = Command::new("cargo");
    cmd.args(["metadata", "--no-deps", "--format-version", "1"]);
    if let Some(d) = current_dir {
        cmd.current_dir(d);
    }
    let saida = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ErroMetadata::BinarioNaoEncontrado
        } else {
            ErroMetadata::FalhaSubprocess(e.to_string())
        }
    })?;
    if !saida.status.success() {
        return Err(ErroMetadata::StatusErro {
            codigo: saida.status.code(),
            stderr: String::from_utf8_lossy(&saida.stderr).into_owned(),
        });
    }
    let stdout = String::from_utf8(saida.stdout)
        .map_err(|e| ErroMetadata::StdoutInvalido(e.to_string()))?;
    serde_json::from_str(&stdout).map_err(|e| ErroMetadata::JsonInvalido(e.to_string()))
}

/// Seleção pura: dado o pacote (já encontrado), aplica a regra de escolha.
///
/// Regra (prompt 0023):
/// - há alvo de biblioteca (qualquer `kind` em [`KINDS_LIB`]) → [`AlvoFork::Lib`].
/// - sem lib, exatamente 1 alvo `bin` → [`AlvoFork::Bin(nome)`].
/// - sem lib, 0 ou ≥2 bins → [`ErroMetadata::AlvosAmbiguos`].
pub(crate) fn selecionar_alvo(pkg: &MetadataPackage) -> Result<AlvoFork, ErroMetadata> {
    let tem_lib = pkg
        .targets
        .iter()
        .any(|t| t.kind.iter().any(|k| KINDS_LIB.contains(&k.as_str())));
    if tem_lib {
        return Ok(AlvoFork::Lib);
    }
    let bins: Vec<String> = pkg
        .targets
        .iter()
        .filter(|t| t.kind.iter().any(|k| k == KIND_BIN))
        .map(|t| t.name.clone())
        .collect();
    if bins.len() == 1 {
        Ok(AlvoFork::Bin(bins.into_iter().next().unwrap()))
    } else {
        Err(ErroMetadata::AlvosAmbiguos { bins })
    }
}

/// Descoberta **por nome**: usada pela porta `fork::invocar_fork(pacote)`
/// (modo `--pacote` da CLI). Roda metadata no cwd e procura o pacote por
/// `name`. Fecha a pendência do laudo 0022.
pub(crate) fn detectar_alvo_por_nome(
    pacote: &str,
    current_dir: Option<&Path>,
) -> Result<AlvoFork, ErroMetadata> {
    let md = invocar_metadata(current_dir)?;
    let pkg = md
        .packages
        .iter()
        .find(|p| p.name == pacote)
        .ok_or_else(|| ErroMetadata::PacoteNaoEncontrado(pacote.to_string()))?;
    selecionar_alvo(pkg)
}

/// Descoberta **por diretório**: usada pela porta `invocacao::invocar`
/// (caminho do `extrair_grafo`). Roda metadata com `current_dir = diretorio`
/// e procura o pacote cujo `manifest_path` é `diretorio/Cargo.toml`.
/// Devolve nome **e** alvo para o chamador não precisar inspecionar o JSON
/// de novo.
///
/// **Checagem de existência ANTES do spawn** (prompt 0024, fecha D5 do 0023):
/// `Command::current_dir(d)` num diretório inexistente falha com
/// `io::ErrorKind::NotFound`, indistinguível de `cargo` ausente do PATH.
/// Em vez de inferir depois, curto-circuitamos com
/// [`ErroMetadata::DiretorioInexistente`] aqui. Custo: uma chamada `is_dir`
/// (stat barato); ganho: diagnóstico preciso. O caminho `--pacote`
/// (`current_dir = None`) não precisa da checagem — cwd sempre existe.
pub(crate) fn detectar_pacote_e_alvo_por_diretorio(
    diretorio: &Path,
) -> Result<(String, AlvoFork), ErroMetadata> {
    if !diretorio.is_dir() {
        return Err(ErroMetadata::DiretorioInexistente(diretorio.to_path_buf()));
    }
    let md = invocar_metadata(Some(diretorio))?;
    let manifest_alvo = diretorio.join("Cargo.toml");
    let pkg = md
        .packages
        .iter()
        .find(|p| PathBuf::from(&p.manifest_path) == manifest_alvo)
        .ok_or_else(|| {
            ErroMetadata::PacoteNoDiretorioNaoEncontrado(
                manifest_alvo.to_string_lossy().into_owned(),
            )
        })?;
    let alvo = selecionar_alvo(pkg)?;
    Ok((pkg.name.clone(), alvo))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: monta um `MetadataOutput` desserializando um JSON literal.
    /// Os testes de seleção operam só sobre o JSON — sem cargo, sem I/O.
    fn parse(json: &str) -> MetadataOutput {
        serde_json::from_str(json).expect("JSON de teste deve ser válido")
    }

    fn pkg<'a>(md: &'a MetadataOutput, nome: &str) -> &'a MetadataPackage {
        md.packages.iter().find(|p| p.name == nome).unwrap()
    }

    const JSON_BIN_MAIS_LIB: &str = r#"{
        "packages": [{
            "name": "p", "manifest_path": "/x/Cargo.toml",
            "targets": [
                {"name":"p","kind":["lib"]},
                {"name":"p","kind":["bin"]}
            ]
        }]
    }"#;

    const JSON_SO_LIB: &str = r#"{
        "packages": [{
            "name": "p", "manifest_path": "/x/Cargo.toml",
            "targets": [{"name":"p","kind":["lib"]}]
        }]
    }"#;

    const JSON_SO_BIN_UNICO: &str = r#"{
        "packages": [{
            "name": "p", "manifest_path": "/x/Cargo.toml",
            "targets": [{"name":"so-bin","kind":["bin"]}]
        }]
    }"#;

    const JSON_MULTI_BIN_SEM_LIB: &str = r#"{
        "packages": [{
            "name": "p", "manifest_path": "/x/Cargo.toml",
            "targets": [
                {"name":"a","kind":["bin"]},
                {"name":"b","kind":["bin"]}
            ]
        }]
    }"#;

    const JSON_PROC_MACRO: &str = r#"{
        "packages": [{
            "name": "pm", "manifest_path": "/x/Cargo.toml",
            "targets": [{"name":"pm","kind":["proc-macro"]}]
        }]
    }"#;

    const JSON_WORKSPACE_DOIS_PKGS: &str = r#"{
        "packages": [
            {"name": "lib_pkg", "manifest_path": "/a/Cargo.toml",
             "targets": [{"name":"lib_pkg","kind":["lib"]}]},
            {"name": "bin_pkg", "manifest_path": "/b/Cargo.toml",
             "targets": [{"name":"bin_pkg","kind":["bin"]}]}
        ]
    }"#;

    // ---- Regra de seleção -------------------------------------------------

    #[test]
    fn bin_mais_lib_seleciona_lib() {
        // O caso que motivou o prompt 0022 (egui_demo_app). Agora pela fonte
        // autoritativa, não pela heurística.
        assert_eq!(selecionar_alvo(pkg(&parse(JSON_BIN_MAIS_LIB), "p")).unwrap(), AlvoFork::Lib);
    }

    #[test]
    fn so_lib_seleciona_lib() {
        assert_eq!(selecionar_alvo(pkg(&parse(JSON_SO_LIB), "p")).unwrap(), AlvoFork::Lib);
    }

    #[test]
    fn so_bin_unico_seleciona_bin_com_nome_do_target() {
        // Nome vem do `target.name` (não do `package.name`), que é o que o
        // `cargo modules --bin <nome>` espera. A heurística do 0022 usava
        // `package.name` para o caso `src/main.rs` — convergente quando
        // coincidem, divergente quando o `[[bin]]` tem name próprio.
        assert_eq!(
            selecionar_alvo(pkg(&parse(JSON_SO_BIN_UNICO), "p")).unwrap(),
            AlvoFork::Bin("so-bin".to_string())
        );
    }

    #[test]
    fn proc_macro_conta_como_lib() {
        // Fase 1 do prompt 0023 verificou contra binário real: `cargo modules
        // --lib` num proc-macro funciona. Por isso `proc-macro` está em
        // KINDS_LIB.
        assert_eq!(selecionar_alvo(pkg(&parse(JSON_PROC_MACRO), "pm")).unwrap(), AlvoFork::Lib);
    }

    #[test]
    fn multi_bin_sem_lib_devolve_ambiguo_com_nomes() {
        match selecionar_alvo(pkg(&parse(JSON_MULTI_BIN_SEM_LIB), "p")).unwrap_err() {
            ErroMetadata::AlvosAmbiguos { bins } => {
                assert_eq!(bins, vec!["a".to_string(), "b".to_string()]);
                // Display lista os bins — diagnóstico claro.
                let msg = format!(
                    "{}",
                    ErroMetadata::AlvosAmbiguos {
                        bins: vec!["a".to_string(), "b".to_string()]
                    }
                );
                assert!(msg.contains("a"));
                assert!(msg.contains("b"));
            }
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    /// Caso de borda: pacote sem nenhum alvo (não há cargo válido que
    /// produza isso, mas a regra degrada com diagnóstico claro em vez de
    /// pânico).
    #[test]
    fn sem_nenhum_alvo_devolve_ambiguo_vazio() {
        let json = r#"{
            "packages": [{
                "name":"p","manifest_path":"/x/Cargo.toml","targets":[]
            }]
        }"#;
        match selecionar_alvo(pkg(&parse(json), "p")).unwrap_err() {
            ErroMetadata::AlvosAmbiguos { bins } => assert!(bins.is_empty()),
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    // ---- Descoberta por nome (porta --pacote) -----------------------------

    #[test]
    fn descoberta_por_nome_acha_pacote_certo_num_workspace() {
        let md = parse(JSON_WORKSPACE_DOIS_PKGS);
        // Simula o que `detectar_alvo_por_nome` faz após chamar metadata.
        let achado = md.packages.iter().find(|p| p.name == "bin_pkg").unwrap();
        assert_eq!(
            selecionar_alvo(achado).unwrap(),
            AlvoFork::Bin("bin_pkg".to_string())
        );
        let achado = md.packages.iter().find(|p| p.name == "lib_pkg").unwrap();
        assert_eq!(selecionar_alvo(achado).unwrap(), AlvoFork::Lib);
    }

    // ---- Descoberta por diretório (porta extrair_grafo) -------------------

    #[test]
    fn descoberta_por_manifest_path_em_workspace() {
        let md = parse(JSON_WORKSPACE_DOIS_PKGS);
        // O `extrair_grafo` recebe um diretório; o `detectar_pacote_e_alvo`
        // procura o pacote cujo `manifest_path == diretorio/Cargo.toml`.
        let manifest = PathBuf::from("/a/Cargo.toml");
        let achado = md
            .packages
            .iter()
            .find(|p| PathBuf::from(&p.manifest_path) == manifest)
            .unwrap();
        assert_eq!(achado.name, "lib_pkg");
        assert_eq!(selecionar_alvo(achado).unwrap(), AlvoFork::Lib);
    }

    // ---- Display dos erros (sanity) ---------------------------------------

    #[test]
    fn display_cobre_todas_as_variantes_de_erro_metadata() {
        let variantes = [
            ErroMetadata::BinarioNaoEncontrado,
            ErroMetadata::FalhaSubprocess("io".to_string()),
            ErroMetadata::StatusErro {
                codigo: Some(101),
                stderr: "x".to_string(),
            },
            ErroMetadata::StdoutInvalido("byte".to_string()),
            ErroMetadata::JsonInvalido("eof".to_string()),
            ErroMetadata::PacoteNaoEncontrado("zzz".to_string()),
            ErroMetadata::PacoteNoDiretorioNaoEncontrado("/x/Cargo.toml".to_string()),
            ErroMetadata::AlvosAmbiguos { bins: vec![] },
            ErroMetadata::AlvosAmbiguos {
                bins: vec!["a".to_string()],
            },
            ErroMetadata::DiretorioInexistente(PathBuf::from("/tmp/nada")),
        ];
        for v in &variantes {
            assert!(!format!("{}", v).is_empty());
        }
    }

    /// Sanity da checagem do prompt 0024: a porta por diretório curto-circuita
    /// com `DiretorioInexistente` antes do spawn — sem precisar de cargo.
    #[test]
    fn por_diretorio_curto_circuita_em_diretorio_inexistente() {
        let dir = std::path::PathBuf::from("/tmp/__lente_md_inexistente_0024__");
        // Garantia: o diretório não existe no instante do teste.
        let _ = std::fs::remove_dir_all(&dir);
        match detectar_pacote_e_alvo_por_diretorio(&dir).unwrap_err() {
            ErroMetadata::DiretorioInexistente(p) => assert_eq!(p, dir),
            outro => panic!("variante inesperada: {:?}", outro),
        }
    }

    /// Não-regressão: um arquivo (não-diretório) também dispara
    /// `DiretorioInexistente` (a checagem é `is_dir`, não `exists`).
    #[test]
    fn por_diretorio_arquivo_e_nao_diretorio_tambem_curto_circuita() {
        let arquivo = std::env::temp_dir().join("__lente_md_arquivo_0024__");
        std::fs::write(&arquivo, "x").unwrap();
        let res = detectar_pacote_e_alvo_por_diretorio(&arquivo);
        let _ = std::fs::remove_file(&arquivo);
        match res.unwrap_err() {
            ErroMetadata::DiretorioInexistente(p) => assert_eq!(p, arquivo),
            outro => panic!("variante inesperada: {:?}", outro),
        }
    }

    // ---- E2E `#[ignore]` — porta --pacote contra o workspace real ---------

    /// E2E: roda metadata no workspace do projeto-lente e confirma que a
    /// descoberta por nome de `lente_core` (só-lib) devolve `Lib`. Requer
    /// `cargo` no PATH (já requerido pelo fork).
    #[test]
    #[ignore]
    fn e2e_descoberta_por_nome_no_workspace_real() {
        let alvo = detectar_alvo_por_nome("lente_core", None)
            .expect("descoberta de lente_core deve funcionar do cwd raiz");
        assert_eq!(alvo, AlvoFork::Lib);
    }

    /// E2E: pacote inexistente → `PacoteNaoEncontrado` (diagnóstico próprio
    /// para o usuário que digitou nome errado no `--pacote`).
    #[test]
    #[ignore]
    fn e2e_pacote_inexistente_da_erro_proprio() {
        match detectar_alvo_por_nome("pacote_que_nao_existe_2026", None) {
            Err(ErroMetadata::PacoteNaoEncontrado(n)) => {
                assert_eq!(n, "pacote_que_nao_existe_2026");
            }
            outro => panic!("variante inesperada: {:?}", outro),
        }
    }

    /// E2E **da porta `--pacote` fechada** (pendência do laudo 0022): monta
    /// fixture bin+lib excluído do workspace pai, e chama
    /// `detectar_alvo_por_nome` com `Some(dir)` (semanticamente equivalente
    /// a rodar `fork::invocar_fork` com cwd=`dir`). Antes do 0023, `invocar_fork`
    /// no cwd de um bin+lib falhava por não passar `--lib`/`--bin`; agora a
    /// detecção por metadata acha o pacote por nome e devolve `Lib`.
    #[test]
    #[ignore]
    fn e2e_porta_pacote_descobre_bin_mais_lib_por_nome() {
        let dir = std::env::temp_dir().join("__lente_e2e_porta_pacote__");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            r#"
[workspace]

[package]
name = "porta_pacote_e2e"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
path = "src/lib.rs"

[[bin]]
name = "porta_pacote_e2e"
path = "src/main.rs"
"#,
        )
        .unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub fn x() {}").unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
        let alvo = detectar_alvo_por_nome("porta_pacote_e2e", Some(&dir))
            .expect("metadata deve achar o pacote por nome a partir do diretório");
        assert_eq!(alvo, AlvoFork::Lib);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
