//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-mapa-correspondencia.md
//! @prompt-hash f040d3ff
//! @layer L3
//! @updated 2026-08-26

use lente_comparacao::{
    AlegacaoDeclarada, CorrespondenciaDeclarada, MapaCorrespondencia, RegraPrefixo,
    RelacaoDeclarada,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fmt, path::Path};

#[derive(Debug)]
pub enum ErroMapaCorrespondencia {
    Io(std::io::Error),
    Toml(String),
    Versao(u64),
    Valor(String),
}

impl fmt::Display for ErroMapaCorrespondencia {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "não foi possível ler mapa: {e}"),
            Self::Toml(e) => write!(f, "TOML inválido no mapa: {e}"),
            Self::Versao(v) => write!(f, "versão de mapa não suportada: {v} (esperada 1)"),
            Self::Valor(v) => write!(f, "valor desconhecido no mapa: {v}"),
        }
    }
}
impl std::error::Error for ErroMapaCorrespondencia {}

#[derive(Deserialize)]
struct ArquivoMapa {
    tekt_comparison_map: u64,
    #[serde(default)]
    prefix: Vec<PrefixoDto>,
    #[serde(default)]
    correspondencia: Vec<CorrespondenciaDto>,
}
#[derive(Deserialize)]
struct PrefixoDto {
    antes: String,
    depois: String,
}
#[derive(Deserialize)]
struct CorrespondenciaDto {
    id: String,
    antes: Vec<String>,
    depois: Vec<String>,
    relacao: String,
    alegacao: String,
    #[serde(default)]
    evidencias: Vec<String>,
    #[serde(default)]
    nota: String,
}

fn relacao(s: &str) -> Result<RelacaoDeclarada, ErroMapaCorrespondencia> {
    match s {
        "um-para-um" => Ok(RelacaoDeclarada::UmParaUm),
        "divisao" => Ok(RelacaoDeclarada::Divisao),
        "consolidacao" => Ok(RelacaoDeclarada::Consolidacao),
        "grupo" => Ok(RelacaoDeclarada::Grupo),
        _ => Err(ErroMapaCorrespondencia::Valor(format!("relacao={s}"))),
    }
}
fn alegacao(s: &str) -> Result<AlegacaoDeclarada, ErroMapaCorrespondencia> {
    match s {
        "declarada-fechada" => Ok(AlegacaoDeclarada::DeclaradaFechada),
        "parcial" => Ok(AlegacaoDeclarada::Parcial),
        "ausente" => Ok(AlegacaoDeclarada::Ausente),
        "fora-de-escopo" => Ok(AlegacaoDeclarada::ForaDeEscopo),
        "unknown" => Ok(AlegacaoDeclarada::Unknown),
        _ => Err(ErroMapaCorrespondencia::Valor(format!("alegacao={s}"))),
    }
}

pub fn ler_mapa_correspondencia(
    path: &Path,
) -> Result<MapaCorrespondencia, ErroMapaCorrespondencia> {
    let bytes = std::fs::read(path).map_err(ErroMapaCorrespondencia::Io)?;
    let texto =
        std::str::from_utf8(&bytes).map_err(|e| ErroMapaCorrespondencia::Toml(e.to_string()))?;
    let dto: ArquivoMapa =
        toml::from_str(texto).map_err(|e| ErroMapaCorrespondencia::Toml(e.to_string()))?;
    if dto.tekt_comparison_map != 1 {
        return Err(ErroMapaCorrespondencia::Versao(dto.tekt_comparison_map));
    }
    let correspondencias = dto
        .correspondencia
        .into_iter()
        .map(|c| {
            Ok(CorrespondenciaDeclarada {
                id: c.id,
                antes: c.antes,
                depois: c.depois,
                relacao: relacao(&c.relacao)?,
                alegacao: alegacao(&c.alegacao)?,
                evidencias: c.evidencias,
                nota: c.nota,
            })
        })
        .collect::<Result<Vec<_>, ErroMapaCorrespondencia>>()?;
    Ok(MapaCorrespondencia {
        caminho: path.display().to_string(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        prefixos: dto
            .prefix
            .into_iter()
            .map(|p| RegraPrefixo {
                antes: p.antes,
                depois: p.depois,
            })
            .collect(),
        correspondencias,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpreta_contrato_minimo() {
        let dto: ArquivoMapa =
            toml::from_str("tekt_comparison_map=1\n[[prefix]]\nantes='a'\ndepois='b'\n").unwrap();
        assert_eq!(dto.prefix.len(), 1);
    }
    #[test]
    fn enum_desconhecido_nao_vira_unknown() {
        assert!(relacao("magia").is_err());
    }
}
