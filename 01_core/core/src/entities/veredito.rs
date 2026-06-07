//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/veredito.md
//! @prompt-hash bc56b948
//! @layer L1
//! @updated 2026-06-07
//! ADRs:    00_nucleo/adr/0004-resolucao-colisoes-path.md
//! Camada:  L1 — Núcleo. Tipo puro (sem I/O, sem deps externas).
//!
//! Veredito da investigação de uma colisão de path no grafo.
//!
//! Produzido por `lente_investiga`, consumido por `lente_resolve` (futuro).
//! Mora no `lente_core` por decisão do ADR-0004 §5: é parte do vocabulário
//! central — outros componentes futuros podem precisar dele (por ex., relatório
//! ao usuário sobre como cada colisão foi tratada).

/// Conclusão da investigação de uma colisão de path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Veredito {
    /// Os dois nós colidentes são, de fato, o mesmo item — alcançável por dois
    /// caminhos (ex.: reexports). Devem ser unificados.
    MesmoItem,
    /// Os dois nós são itens diferentes que o fork agregou no mesmo path.
    /// Devem receber identidades novas (responsabilidade do `lente_resolve`).
    Distintos { evidencia: Evidencia },
    /// A cascata de estratégias esgotou sem decidir. `diagnostico` explica
    /// o que cada estratégia tentou e por que não bastou.
    NaoDeterminado { diagnostico: String },
}

/// Evidência que sustenta o veredito `Distintos`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evidencia {
    /// As arestas dos dois nós são desconexas no grafo.
    /// `compartilhadas == 0`; ambos têm arestas exclusivas.
    VizinhancaDisjunta {
        exclusivas_a: usize,
        exclusivas_b: usize,
    },
    /// O código-fonte expõe dois `impl <Trait> for <Tipo>` distintos,
    /// cada um declarando o método em questão.
    ImplDeTraitsDiferentes { traits: (String, String) },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn veredito_mesmo_item_construido() {
        let v = Veredito::MesmoItem;
        assert!(matches!(v, Veredito::MesmoItem));
    }

    #[test]
    fn veredito_distintos_carrega_evidencia() {
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 2,
                exclusivas_b: 3,
            },
        };
        match v {
            Veredito::Distintos { evidencia: Evidencia::VizinhancaDisjunta { exclusivas_a, exclusivas_b } } => {
                assert_eq!(exclusivas_a, 2);
                assert_eq!(exclusivas_b, 3);
            }
            _ => panic!("variante errada"),
        }
    }

    #[test]
    fn veredito_distintos_com_impl_traits() {
        let v = Veredito::Distintos {
            evidencia: Evidencia::ImplDeTraitsDiferentes {
                traits: ("Display".to_string(), "Debug".to_string()),
            },
        };
        match v {
            Veredito::Distintos { evidencia: Evidencia::ImplDeTraitsDiferentes { traits } } => {
                assert_eq!(traits.0, "Display");
                assert_eq!(traits.1, "Debug");
            }
            _ => panic!("variante errada"),
        }
    }

    #[test]
    fn veredito_nao_determinado_carrega_diagnostico() {
        let v = Veredito::NaoDeterminado {
            diagnostico: "cascata esgotada".to_string(),
        };
        match v {
            Veredito::NaoDeterminado { diagnostico } => {
                assert!(diagnostico.contains("cascata"));
            }
            _ => panic!("variante errada"),
        }
    }
}
