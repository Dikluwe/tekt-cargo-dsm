//! Lineage: prompt 00_nucleo/prompt/0004-lente_investiga.md
//!
//! Estratégia 1 — vizinhança no grafo. Compara conjuntos de arestas dos dois
//! nós do par. Critério categórico (sem thresholds mágicos):
//!
//! - Compartilhadas == 0 e ambos com ≥1 exclusiva → `Distintos`.
//! - Exclusivas A == 0 e Exclusivas B == 0 e Compartilhadas ≥ 1 → `MesmoItem`.
//! - Resto (sobreposição parcial, ou ambos vazios) → inconclusivo
//!   (`ResultadoVizinhanca::Inconclusivo`); a cascata passa à Estratégia 2.

use std::collections::HashSet;

use lente_core::entities::grafo::{Aresta, Relation};
use lente_core::entities::veredito::{Evidencia, Veredito};

use crate::ArestasNo;

/// O que a Estratégia 1 conseguiu concluir.
pub(crate) enum ResultadoVizinhanca {
    /// Conclusão direta — usar este veredito.
    Decidiu(Veredito),
    /// Não conseguiu decidir; passar para a próxima estratégia.
    Inconclusivo {
        exclusivas_a: usize,
        exclusivas_b: usize,
        compartilhadas: usize,
    },
}

pub(crate) fn analisar(a: &ArestasNo, b: &ArestasNo) -> ResultadoVizinhanca {
    let set_a = colecionar(a);
    let set_b = colecionar(b);

    let compartilhadas = set_a.intersection(&set_b).count();
    let exclusivas_a = set_a.difference(&set_b).count();
    let exclusivas_b = set_b.difference(&set_a).count();

    // Caso "ambos vazios": nada a comparar → inconclusivo.
    if set_a.is_empty() && set_b.is_empty() {
        return ResultadoVizinhanca::Inconclusivo {
            exclusivas_a,
            exclusivas_b,
            compartilhadas,
        };
    }

    // Caso "disjuntos": nenhuma aresta comum e ambos têm pelo menos uma.
    if compartilhadas == 0 && exclusivas_a > 0 && exclusivas_b > 0 {
        return ResultadoVizinhanca::Decidiu(Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a,
                exclusivas_b,
            },
        });
    }

    // Caso "idênticos": zero exclusivas dos dois lados, pelo menos uma compartilhada.
    if exclusivas_a == 0 && exclusivas_b == 0 && compartilhadas > 0 {
        return ResultadoVizinhanca::Decidiu(Veredito::MesmoItem);
    }

    // Sobreposição parcial, ou um lado vazio com o outro não — passa adiante.
    ResultadoVizinhanca::Inconclusivo {
        exclusivas_a,
        exclusivas_b,
        compartilhadas,
    }
}

/// Coleta arestas do nó (entradas + saídas) num conjunto comparável.
fn colecionar(arestas: &ArestasNo) -> HashSet<ChaveAresta> {
    let mut s = HashSet::new();
    for a in &arestas.entrando {
        s.insert(ChaveAresta::de(a));
    }
    for a in &arestas.saindo {
        s.insert(ChaveAresta::de(a));
    }
    s
}

/// Identidade comparável de uma aresta: `(id_from, id_to, relation)`.
///
/// Usa **ids**, não paths, porque arestas que apontam para cópias distintas
/// de um mesmo path (caso `Display+Debug`) têm `id_to` diferentes mas o
/// mesmo `to`-path. Comparar por path colapsa-as numa só chave e a
/// vizinhança de cópias distintas parece idêntica — o bug que a remedição
/// (`lab/medicao-colisoes/remedicao/relatorio.md` §6) revelou.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct ChaveAresta {
    id_from: usize,
    id_to: usize,
    relation: Relation,
}

impl ChaveAresta {
    fn de(a: &Aresta) -> Self {
        ChaveAresta {
            id_from: a.id_from,
            id_to: a.id_to,
            relation: a.relation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::entities::grafo::Path;

    fn id_de(s: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        h.finish() as usize
    }

    fn aresta(from: &str, to: &str, r: Relation) -> Aresta {
        Aresta {
            from: Path::from(from),
            id_from: id_de(from),
            to: Path::from(to),
            id_to: id_de(to),
            relation: r,
        }
    }

    #[test]
    fn vizinhancas_disjuntas_decidem_distintos() {
        let a = ArestasNo {
            entrando: vec![aresta("X", "T", Relation::Uses)],
            saindo: vec![aresta("T", "Y", Relation::Uses)],
        };
        let b = ArestasNo {
            entrando: vec![aresta("P", "T", Relation::Uses)],
            saindo: vec![aresta("T", "Q", Relation::Uses)],
        };
        match analisar(&a, &b) {
            ResultadoVizinhanca::Decidiu(Veredito::Distintos {
                evidencia: Evidencia::VizinhancaDisjunta {
                    exclusivas_a,
                    exclusivas_b,
                },
            }) => {
                assert_eq!(exclusivas_a, 2);
                assert_eq!(exclusivas_b, 2);
            }
            outro => panic!("esperava Distintos por vizinhança disjunta, veio {:?}", matches!(outro, ResultadoVizinhanca::Inconclusivo { .. })),
        }
    }

    #[test]
    fn vizinhancas_identicas_decidem_mesmo_item() {
        let arestas = vec![
            aresta("X", "T", Relation::Uses),
            aresta("T", "Y", Relation::Uses),
        ];
        let a = ArestasNo {
            entrando: vec![arestas[0].clone()],
            saindo: vec![arestas[1].clone()],
        };
        let b = ArestasNo {
            entrando: vec![arestas[0].clone()],
            saindo: vec![arestas[1].clone()],
        };
        match analisar(&a, &b) {
            ResultadoVizinhanca::Decidiu(Veredito::MesmoItem) => {}
            _ => panic!("esperava MesmoItem por vizinhança idêntica"),
        }
    }

    #[test]
    fn sobreposicao_parcial_e_inconclusiva() {
        let a = ArestasNo {
            entrando: vec![
                aresta("X", "T", Relation::Uses),
                aresta("Y", "T", Relation::Uses),
            ],
            saindo: vec![],
        };
        let b = ArestasNo {
            entrando: vec![
                aresta("X", "T", Relation::Uses), // compartilhada
                aresta("Z", "T", Relation::Uses), // exclusiva
            ],
            saindo: vec![],
        };
        match analisar(&a, &b) {
            ResultadoVizinhanca::Inconclusivo {
                exclusivas_a,
                exclusivas_b,
                compartilhadas,
            } => {
                assert_eq!(exclusivas_a, 1);
                assert_eq!(exclusivas_b, 1);
                assert_eq!(compartilhadas, 1);
            }
            _ => panic!("esperava Inconclusivo"),
        }
    }

    #[test]
    fn ambos_vazios_e_inconclusivo() {
        let a = ArestasNo {
            entrando: vec![],
            saindo: vec![],
        };
        let b = ArestasNo {
            entrando: vec![],
            saindo: vec![],
        };
        match analisar(&a, &b) {
            ResultadoVizinhanca::Inconclusivo { .. } => {}
            _ => panic!("esperava Inconclusivo"),
        }
    }

    /// Salvaguarda contra a regressão da remedição §6: duas cópias do mesmo
    /// path (caso `Display+Debug`) recebem arestas com **mesmo
    /// `from`/`to`/`relation`** mas **`id_to` distintos**. A `ChaveAresta`
    /// precisa incluir os ids para que essas arestas sejam contadas como
    /// distintas — caso contrário, vizinhança parece idêntica e o veredito
    /// vira `MesmoItem` (errado).
    #[test]
    fn vizinhancas_de_copias_distintas_decidem_distintos() {
        // Cópia A do "X::fmt": id=100; cópia B: id=101. X tem id=42.
        // Cada cópia recebe UMA aresta `Owns` que aponta para SEU id.
        // No JSON do fork novo, esse é exatamente o cenário do `Display+Debug`.
        let a = ArestasNo {
            entrando: vec![Aresta {
                from: Path::from("X"),
                id_from: 42,
                to: Path::from("X::fmt"),
                id_to: 100,
                relation: Relation::Owns,
            }],
            saindo: vec![],
        };
        let b = ArestasNo {
            entrando: vec![Aresta {
                from: Path::from("X"),
                id_from: 42,
                to: Path::from("X::fmt"),
                id_to: 101,
                relation: Relation::Owns,
            }],
            saindo: vec![],
        };
        match analisar(&a, &b) {
            ResultadoVizinhanca::Decidiu(Veredito::Distintos {
                evidencia:
                    Evidencia::VizinhancaDisjunta {
                        exclusivas_a,
                        exclusivas_b,
                    },
            }) => {
                assert_eq!(exclusivas_a, 1);
                assert_eq!(exclusivas_b, 1);
            }
            outro => panic!(
                "esperava Distintos por vizinhança disjunta (ids distintos); \
                 veio inconclusivo? {}",
                matches!(outro, ResultadoVizinhanca::Inconclusivo { .. })
            ),
        }
    }
}
