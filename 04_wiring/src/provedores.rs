//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/wiring-provedores.md
//! @prompt-hash fdfc60fc
//! @layer L4
//! @updated 2026-09-26
//! Composição externa: não aplica convenções de resolução ou filtragem Rust.

use lente_cli::saida::{self, AlvoPedido, Modo};
use lente_core::{
    domain::{
        consulta::{Escopo, ModoUses},
        raio::calcular_raio,
    },
    entities::grafo::Path,
};
use lente_infra::{
    extracao::{CausaProvedor, ExtratorGrafo, SolicitacaoExtracao},
    provedores::{ErroFerramenta, ExtratorFerramenta, PedidoFerramenta},
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "modo", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ConsultaProvedor {
    Ranking { top: usize },
    Raio { alvo: String },
    RaioId { alvo_id: usize },
    Estrutura,
}

pub fn analisar_provedor(
    provedor: &str,
    configuracao: &str,
    consulta: ConsultaProvedor,
) -> Result<String, ErroFerramenta> {
    if matches!(&consulta, ConsultaProvedor::Raio { alvo } if alvo.is_empty()) {
        return Err(ErroFerramenta::Parametros("alvo vazio".into()));
    }
    let pedido = PedidoFerramenta::ler(provedor, configuracao)?;
    let mut extrator = ExtratorFerramenta::default();
    let extracao = extrator
        .extrair(SolicitacaoExtracao::Ferramenta(&pedido))
        .map_err(|e| match e.causa {
            CausaProvedor::Ferramenta(e) => e,
            _ => ErroFerramenta::Execucao(e.to_string()),
        })?;
    let grafo = &extracao.grafo;
    let modo = Modo {
        text: false,
        verbose: true,
    };
    let analise = match consulta {
        ConsultaProvedor::Ranking { top } => {
            saida::formatar_ranking(&lente_ranking::rankear(grafo, top), Escopo::Completo, &modo)
        }
        ConsultaProvedor::Estrutura => {
            let estrutura = super::estrutura_de_grafo(grafo.clone(), ModoUses::Todas)
                .map_err(|e| ErroFerramenta::SaidaInvalida(e.to_string()))?;
            saida::formatar_estrutura(&estrutura, Escopo::Completo, ModoUses::Todas, &modo)
        }
        consulta => {
            let (path, pedido) = match consulta {
                ConsultaProvedor::Raio { alvo } => {
                    (Path::from(alvo.as_str()), AlvoPedido::Path(alvo))
                }
                ConsultaProvedor::RaioId { alvo_id } => {
                    let n = grafo
                        .nodes
                        .iter()
                        .find(|n| n.id == alvo_id)
                        .ok_or_else(|| {
                            ErroFerramenta::Parametros(format!("alvo_id {alvo_id} ausente"))
                        })?;
                    (n.path.clone(), AlvoPedido::Id(alvo_id))
                }
                _ => unreachable!(),
            };
            let raio = calcular_raio(grafo, &path)
                .map_err(|e| ErroFerramenta::Parametros(e.to_string()))?;
            saida::formatar(&raio, &pedido, Escopo::Completo, &modo)
        }
    };
    let analise: serde_json::Value =
        serde_json::from_str(&analise).map_err(|e| ErroFerramenta::SaidaInvalida(e.to_string()))?;
    let contexto = extrator
        .contexto
        .expect("extrator bem-sucedido conserva contexto");
    Ok(json!({ "analise": analise, "extracao": contexto.dados }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consulta_incompativel_e_rejeitada_sem_extracao() {
        assert!(
            serde_json::from_str::<ConsultaProvedor>(
                r#"{"modo":"ranking","top":2,"filtrar_stdlib":true}"#
            )
            .is_err()
        );
        assert!(serde_json::from_str::<ConsultaProvedor>(r#"{"modo":"diff"}"#).is_err());
        assert!(matches!(
            analisar_provedor(
                "python",
                "{}",
                ConsultaProvedor::Raio {
                    alvo: String::new()
                }
            ),
            Err(ErroFerramenta::Parametros(_))
        ));
    }
}
