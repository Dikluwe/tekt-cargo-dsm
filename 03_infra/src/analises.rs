//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-analises.md
//! @prompt-hash c7c22b91
//! @layer L3
//! @updated 2026-09-26
//! Integração por processo das análises de snapshots, regras e DSM.
use crate::provedores::{ErroFerramenta, executar_artefato};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuracao {
    executavel: PathBuf,
    timeout_ms: u64,
    consulta: Consulta,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "modo", rename_all = "kebab-case", deny_unknown_fields)]
enum Consulta {
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
impl Consulta {
    fn nome(&self) -> &'static str {
        match self {
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
    let codigos: &[i32] = if modo == "regras" { &[0, 1, 2] } else { &[0] };
    let (codigo, analise, stderr) = executar_artefato(&c.executavel, &args, c.timeout_ms, codigos)?;
    conferir_saida(modo, codigo, &analise)?;
    Ok(ResultadoAnalise{codigo:codigo as u8,json:json!({"analise":analise,"execucao":{"modo":modo,"codigo_saida":codigo,"stderr":stderr}}).to_string()})
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
