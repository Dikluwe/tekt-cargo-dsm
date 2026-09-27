//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-analises.md
//! @prompt-hash 11b340f4
//! @layer L3
//! @updated 2026-09-26
//! Integração por processo das análises de snapshots, regras e DSM.
use crate::provedores::{ErroFerramenta, executar_artefato, executar_artefato_em_arquivo};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuracao {
    executavel: Option<PathBuf>,
    timeout_ms: u64,
    consulta: Consulta,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "modo", rename_all = "kebab-case", deny_unknown_fields)]
enum Consulta {
    DoxygenImportar {
        xml_dir: PathBuf,
    },
    DoxygenAssociar {
        inventory: PathBuf,
        catalog: PathBuf,
    },
    DoxygenPrioridade {
        inventory: PathBuf,
        catalog: PathBuf,
        matches: PathBuf,
        impact: PathBuf,
    },
    DoxygenExplorar {
        inventory: PathBuf,
        catalog: PathBuf,
        matches: PathBuf,
    },
    DoxygenGrafo {
        grafo: PathBuf,
        #[serde(default = "opaque")]
        politica_nomes: String,
        #[serde(default = "unknown")]
        coverage: String,
        alvos_metricas: Option<Vec<u64>>,
    },
    Snapshots {
        entrada: PathBuf,
    },
    Regras {
        baseline: PathBuf,
        atual: PathBuf,
        regras: PathBuf,
        data: String,
    },
    Dsm {
        entrada: PathBuf,
    },
    CaminhoDsm {
        pacote: PathBuf,
        consumidor: u64,
        alterado: u64,
    },
}
fn opaque() -> String {
    "opaque".into()
}
fn unknown() -> String {
    "unknown".into()
}
impl Consulta {
    fn nome(&self) -> &'static str {
        match self {
            Self::DoxygenImportar { .. } => "doxygen-importar",
            Self::DoxygenAssociar { .. } => "doxygen-associar",
            Self::DoxygenPrioridade { .. } => "doxygen-prioridade",
            Self::DoxygenExplorar { .. } => "doxygen-explorar",
            Self::DoxygenGrafo { .. } => "doxygen-grafo",
            Self::Snapshots { .. } => "snapshots",
            Self::Regras { .. } => "regras",
            Self::Dsm { .. } => "dsm",
            Self::CaminhoDsm { .. } => "caminho-dsm",
        }
    }
    fn argumentos(&self) -> Result<Vec<OsString>, ErroFerramenta> {
        fn arquivo(p: &Path) -> Result<OsString, ErroFerramenta> {
            if !p.is_absolute() || !p.is_file() {
                return Err(ErroFerramenta::Parametros(format!(
                    "arquivo absoluto existente obrigatório: {}",
                    p.display()
                )));
            }
            Ok(p.as_os_str().to_owned())
        }
        Ok(match self {
            Self::DoxygenGrafo { grafo, .. } => vec![arquivo(grafo)?],
            Self::DoxygenImportar { xml_dir } => {
                if !xml_dir.is_absolute() || !xml_dir.is_dir() {
                    return Err(ErroFerramenta::Parametros(
                        "xml_dir deve ser diretório absoluto existente".into(),
                    ));
                }
                vec!["--xml-dir".into(), xml_dir.as_os_str().to_owned()]
            }
            Self::DoxygenAssociar { inventory, catalog } => vec![
                "--inventory".into(),
                arquivo(inventory)?,
                "--catalog".into(),
                arquivo(catalog)?,
            ],
            Self::DoxygenExplorar {
                inventory,
                catalog,
                matches,
            } => vec![
                "--inventory".into(),
                arquivo(inventory)?,
                "--catalog".into(),
                arquivo(catalog)?,
                "--matches".into(),
                arquivo(matches)?,
            ],
            Self::DoxygenPrioridade {
                inventory,
                catalog,
                matches,
                impact,
            } => vec![
                "--inventory".into(),
                arquivo(inventory)?,
                "--catalog".into(),
                arquivo(catalog)?,
                "--matches".into(),
                arquivo(matches)?,
                "--impact".into(),
                arquivo(impact)?,
            ],
            Self::Snapshots { entrada } => vec![arquivo(entrada)?],
            Self::Dsm { entrada } => vec!["prepare".into(), arquivo(entrada)?],
            Self::CaminhoDsm {
                pacote,
                consumidor,
                alterado,
            } => vec![
                "query".into(),
                arquivo(pacote)?,
                consumidor.to_string().into(),
                alterado.to_string().into(),
            ],
            Self::Regras {
                baseline,
                atual,
                regras,
                data,
            } => {
                let b = data.as_bytes();
                if b.len() != 10
                    || b[4] != b'-'
                    || b[7] != b'-'
                    || b.iter()
                        .enumerate()
                        .any(|(i, c)| i != 4 && i != 7 && !c.is_ascii_digit())
                {
                    return Err(ErroFerramenta::Parametros(
                        "data deve ser YYYY-MM-DD; validade civil verificada pelo avaliador".into(),
                    ));
                }
                vec![
                    "--baseline".into(),
                    arquivo(baseline)?,
                    "--current".into(),
                    arquivo(atual)?,
                    "--rules".into(),
                    arquivo(regras)?,
                    "--as-of".into(),
                    data.into(),
                    "--format".into(),
                    "json".into(),
                ]
            }
        })
    }
}

/// Código de domínio (regras) separado de falha de transporte.
pub struct ResultadoAnalise {
    pub codigo: u8,
    pub json: String,
}
fn conferir_saida(modo: &str, codigo: i32, v: &Value) -> Result<(), ErroFerramenta> {
    let valido = match modo {
        "doxygen-importar" => {
            v["schema"] == "lente.doxygen.inventory"
                && v["version"] == 2
                && v["pending_relations"].is_array()
                && v["symbols"].is_array()
                && v["relations"].is_array()
                && v["diagnostics"].is_array()
                && (if codigo == 2 {
                    v["coverage"] == "partial"
                } else {
                    v["coverage"] == "unknown"
                })
        }
        "doxygen-associar" => {
            v["schema"] == "lente.doxygen.matches" && v["version"] == 1 && v["matches"].is_array()
        }
        "doxygen-prioridade" => {
            v["schema"] == "lente.doxygen.priority_report"
                && v["version"] == 2
                && v["items"].is_array()
                && v["pending_relations"].is_array()
        }
        "doxygen-explorar" => {
            v["entries"].is_array()
                && v["pending_relations"].is_array()
                && v["graph_binding"] == "not_verified"
        }
        "regras" => {
            v["report_version"] == 1
                && v["status"].as_str()
                    == Some(match codigo {
                        0 => "aprovado",
                        1 => "violacao",
                        2 => "inconclusivo",
                        _ => "",
                    })
        }
        "snapshots" => {
            v["format_version"] == 1
                && v["mudancas"].is_array()
                && v["antes"].is_object()
                && v["depois"].is_object()
        }
        "dsm" => {
            v["format_version"] == 1
                && v["source_graph"].is_object()
                && v["cells"].is_array()
                && v["metadata"].is_object()
        }
        "caminho-dsm" => matches!(v["status"].as_str(), Some("found" | "no_path")),
        _ => false,
    };
    if valido {
        Ok(())
    } else {
        Err(ErroFerramenta::SaidaInvalida(format!(
            "contrato de saída incompatível com {modo} (código {codigo})"
        )))
    }
}
/// Executa a consulta configurada sem interpretar ou melhorar sua cobertura.
pub fn analisar_artefatos(configuracao: &str) -> Result<ResultadoAnalise, ErroFerramenta> {
    let c: Configuracao = serde_json::from_str(configuracao)
        .map_err(|e| ErroFerramenta::Parametros(e.to_string()))?;
    let args = c.consulta.argumentos()?;
    let modo = c.consulta.nome();
    if !(1..=3_600_000).contains(&c.timeout_ms) {
        return Err(ErroFerramenta::Parametros(
            "timeout_ms deve estar entre 1 e 3600000".into(),
        ));
    }
    if let Consulta::DoxygenGrafo {
        grafo,
        politica_nomes,
        coverage,
        alvos_metricas,
    } = &c.consulta
    {
        if c.executavel.is_some() {
            return Err(ErroFerramenta::Parametros(
                "doxygen-grafo não aceita executavel".into(),
            ));
        }
        let bytes = std::fs::read(grafo).map_err(|e| ErroFerramenta::Parametros(e.to_string()))?;
        let analise =
            crate::doxygen::projetar(&bytes, politica_nomes, coverage, alvos_metricas.as_deref())
                .map_err(ErroFerramenta::SaidaInvalida)?;
        return Ok(ResultadoAnalise {
            codigo: 0,
            json: json!({"analise":analise,"execucao":{"modo":modo,"codigo_saida":0,"stderr":""}})
                .to_string(),
        });
    }
    let exe = c
        .executavel
        .as_deref()
        .ok_or_else(|| ErroFerramenta::Parametros("executavel obrigatório neste modo".into()))?;
    let codigos: &[i32] = match modo {
        "regras" => &[0, 1, 2],
        "doxygen-importar" => &[0, 2],
        _ => &[0],
    };
    let (codigo, analise, stderr) = if modo.starts_with("doxygen-") {
        executar_artefato_em_arquivo(exe, &args, c.timeout_ms, codigos)?
    } else {
        executar_artefato(exe, &args, c.timeout_ms, codigos)?
    };
    conferir_saida(modo, codigo, &analise)?;
    Ok(ResultadoAnalise{codigo:if modo=="regras" {codigo as u8} else {0},json:json!({"analise":analise,"execucao":{"modo":modo,"codigo_saida":codigo,"stderr":stderr}}).to_string()})
}
/// A configuração pode ser relativa; seus caminhos internos devem ser absolutos.
pub fn analisar_arquivo(configuracao: &Path) -> Result<ResultadoAnalise, ErroFerramenta> {
    let texto = std::fs::read_to_string(configuracao)
        .map_err(|e| ErroFerramenta::Parametros(format!("{}: {e}", configuracao.display())))?;
    analisar_artefatos(&texto)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regras_preservam_status_sem_confundir_com_transporte() {
        for (code, status) in [(0, "aprovado"), (1, "violacao"), (2, "inconclusivo")] {
            assert!(
                conferir_saida("regras", code, &json!({"report_version":1,"status":status}))
                    .is_ok()
            );
            assert!(
                conferir_saida(
                    "regras",
                    (code + 1) % 3,
                    &json!({"report_version":1,"status":status})
                )
                .is_err()
            );
        }
    }
    #[test]
    fn configuraçao_estrita_e_caminho_absoluto() {
        assert!(serde_json::from_str::<Configuracao>(r#"{"executavel":"/x","timeout_ms":1,"extra":true,"consulta":{"modo":"dsm","entrada":"/x"}}"#).is_err());
        assert!(
            serde_json::from_str::<Consulta>(r#"{"modo":"dsm","entrada":"/x","completo":true}"#)
                .is_err()
        );
        assert!(
            Consulta::Dsm {
                entrada: "relativo.json".into()
            }
            .argumentos()
            .is_err()
        );
        assert!(super::executar_artefato(Path::new("/ausente"), &[], 0, &[0]).is_err());
    }
}
