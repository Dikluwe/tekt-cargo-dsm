//! Lineage: prompt 00_nucleo/prompt/0004-lente_investiga.md
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0004-resolucao-colisoes-path.md
//! Camada:  L1 — Núcleo. Apenas stdlib. Sem I/O.
//!
//! Investigação de colisões de path no grafo.
//!
//! Decide pela **Estratégia 1** (vizinhança no grafo). A Estratégia 2
//! (parser de fontes, `fontes.rs`) está em **quarentena de remoção** desde o
//! laudo 0014 — o trait passou a vir por nó do fork 0.27.0, tornando-a
//! desnecessária para seu propósito. Mantida no repo, fora do caminho.
//!
//! Não modifica grafo, não nomeia identidades novas — só classifica a colisão.
//! Aplicar a resolução é tarefa do `lente_resolve`.

#![forbid(unsafe_code)]

use lente_core::entities::grafo::{Aresta, No};
use lente_core::entities::veredito::Veredito;

// E2 em quarentena (laudo 0014): fora do caminho da cascata, mantida no repo
// e testada. `allow(dead_code)` porque nada do fluxo principal a referencia.
#[allow(dead_code)]
mod fontes;
mod vizinhanca;

/// Par de nós colidentes (mesmo `path`).
#[derive(Debug, Clone, Copy)]
pub struct ParColidente<'a> {
    pub a: &'a No,
    pub b: &'a No,
}

/// Arestas que entram e saem de um nó.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArestasNo {
    pub entrando: Vec<Aresta>,
    pub saindo: Vec<Aresta>,
}

/// Vizinhança no grafo dos dois nós colidentes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vizinhanca {
    pub a: ArestasNo,
    pub b: ArestasNo,
}

/// Conteúdo de um arquivo de código-fonte para a Estratégia 2.
/// O `caminho_logico` é descritivo (usado em diagnóstico); o `conteudo` é o
/// que o parser examina. Este crate **não toca disco** — quem lê é o L3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArquivoFonte {
    pub caminho_logico: String,
    pub conteudo: String,
}

/// Função principal. Decide se as cópias colidentes são distintas ou o mesmo
/// item, pela **vizinhança no grafo** (Estratégia 1).
///
/// A Estratégia 2 (parser de fontes, `fontes.rs`) está **em quarentena de
/// remoção** desde o laudo 0014: o fork 0.27.0 passa a emitir `trait` por nó
/// (laudo 0013), então o trait para nomeação vem do próprio `No` (lido pelo
/// `lente_resolve`), não de leitura de fontes. A E2 saiu do caminho; o
/// parâmetro `fontes` é ignorado e mantido só por compatibilidade de
/// assinatura. Ver `fontes.rs` e o laudo 0014 para a condição de saída da
/// quarentena.
///
/// Pré-condição lógica: `par.a.path == par.b.path`. Se forem diferentes,
/// devolve `NaoDeterminado` com diagnóstico (em vez de panic) — assim um
/// chamador errado descobre o problema pelo veredito, não pelo crash.
pub fn investigar(
    par: ParColidente<'_>,
    vizinhanca: &Vizinhanca,
    fontes: Option<&[ArquivoFonte]>,
) -> Veredito {
    // E2 em quarentena (laudo 0014): `fontes` não é mais consultado no caminho.
    let _ = fontes;

    if par.a.path != par.b.path {
        return Veredito::NaoDeterminado {
            diagnostico: format!(
                "investigar(): par.a.path ({}) != par.b.path ({}) — não é \
                 realmente uma colisão",
                par.a.path, par.b.path
            ),
        };
    }

    // Estratégia 1 — vizinhança. Decide, ou conclui NaoDeterminado.
    match vizinhanca::analisar(&vizinhanca.a, &vizinhanca.b) {
        vizinhanca::ResultadoVizinhanca::Decidiu(v) => v,
        vizinhanca::ResultadoVizinhanca::Inconclusivo {
            exclusivas_a,
            exclusivas_b,
            compartilhadas,
        } => Veredito::NaoDeterminado {
            diagnostico: format!(
                "Estratégia 1 (vizinhança): inconclusiva — \
                 exclusivas_a={}, exclusivas_b={}, compartilhadas={}. \
                 Estratégia 2 (fontes) em quarentena — fora do caminho \
                 (laudo 0014).",
                exclusivas_a, exclusivas_b, compartilhadas
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::entities::grafo::{Kind, Modificadores, Path, Relation, Visibility};
    use lente_core::entities::veredito::Evidencia;

    fn id_de(s: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        h.finish() as usize
    }

    fn no(path: &str) -> No {
        No {
            id: id_de(path),
            path: Path::from(path),
            name: path.rsplit("::").next().unwrap_or(path).to_string(),
            kind: Kind::Fn,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: "t".to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: None,
        }
    }

    fn aresta(from: &str, to: &str, r: Relation) -> Aresta {
        Aresta {
            from: Path::from(from),
            id_from: id_de(from),
            to: Path::from(to),
            id_to: id_de(to),
            relation: r,
            uses_kind: None,
        }
    }

    fn arq(conteudo: &str) -> ArquivoFonte {
        ArquivoFonte {
            caminho_logico: "teste.rs".to_string(),
            conteudo: conteudo.to_string(),
        }
    }

    #[test]
    fn vizinhancas_disjuntas_decidem_sem_fontes() {
        let a = no("M::ErroRaio::fmt");
        let b = no("M::ErroRaio::fmt");
        let par = ParColidente { a: &a, b: &b };
        let viz = Vizinhanca {
            a: ArestasNo {
                entrando: vec![aresta("M::usa_a", "M::ErroRaio::fmt", Relation::Uses)],
                saindo: vec![],
            },
            b: ArestasNo {
                entrando: vec![aresta("M::usa_b", "M::ErroRaio::fmt", Relation::Uses)],
                saindo: vec![],
            },
        };
        let v = investigar(par, &viz, None);
        match v {
            Veredito::Distintos {
                evidencia: Evidencia::VizinhancaDisjunta { .. },
            } => {}
            outro => panic!("esperava VizinhancaDisjunta, veio {:?}", outro),
        }
    }

    #[test]
    fn vizinhancas_identicas_decidem_mesmo_item() {
        let a = no("M::T::f");
        let b = no("M::T::f");
        let par = ParColidente { a: &a, b: &b };
        let aresta_comum = aresta("M::caller", "M::T::f", Relation::Uses);
        let viz = Vizinhanca {
            a: ArestasNo {
                entrando: vec![aresta_comum.clone()],
                saindo: vec![],
            },
            b: ArestasNo {
                entrando: vec![aresta_comum.clone()],
                saindo: vec![],
            },
        };
        let v = investigar(par, &viz, None);
        assert_eq!(v, Veredito::MesmoItem);
    }

    #[test]
    fn vizinhanca_ambigua_sem_fontes_e_nao_determinado() {
        let a = no("M::T::f");
        let b = no("M::T::f");
        let par = ParColidente { a: &a, b: &b };
        let aresta_comum = aresta("M::caller", "M::T::f", Relation::Uses);
        let aresta_extra = aresta("M::outro", "M::T::f", Relation::Uses);
        let viz = Vizinhanca {
            a: ArestasNo {
                entrando: vec![aresta_comum.clone()],
                saindo: vec![],
            },
            b: ArestasNo {
                entrando: vec![aresta_comum, aresta_extra],
                saindo: vec![],
            },
        };
        let v = investigar(par, &viz, None);
        match v {
            Veredito::NaoDeterminado { diagnostico } => {
                assert!(diagnostico.contains("Estratégia 1"));
                // E2 em quarentena: o diagnóstico anuncia isso, não "sem fontes".
                assert!(diagnostico.contains("quarentena"));
            }
            _ => panic!("esperava NaoDeterminado"),
        }
    }

    #[test]
    fn e2_fora_do_caminho_vizinhanca_ambigua_com_fontes_da_nao_determinado() {
        // Mesmo passando fontes que a E2 resolveria (Display+Debug), com a E2
        // em quarentena o `investigar` NÃO a chama: vizinhança ambígua →
        // NaoDeterminado. (A E2 isolada continua testada em `fontes.rs`.)
        let a = no("lente_core::domain::raio::ErroRaio::fmt");
        let b = no("lente_core::domain::raio::ErroRaio::fmt");
        let par = ParColidente { a: &a, b: &b };

        let comum = aresta("X", "lente_core::domain::raio::ErroRaio::fmt", Relation::Uses);
        let viz = Vizinhanca {
            a: ArestasNo {
                entrando: vec![comum.clone(), aresta("Y", "lente_core::domain::raio::ErroRaio::fmt", Relation::Uses)],
                saindo: vec![],
            },
            b: ArestasNo {
                entrando: vec![comum.clone(), aresta("Z", "lente_core::domain::raio::ErroRaio::fmt", Relation::Uses)],
                saindo: vec![],
            },
        };

        let src = r#"
impl fmt::Display for ErroRaio { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { Ok(()) } }
impl fmt::Debug for ErroRaio { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { Ok(()) } }
"#;
        let v = investigar(par, &viz, Some(&[arq(src)]));
        assert!(
            matches!(v, Veredito::NaoDeterminado { .. }),
            "E2 fora do caminho: deve dar NaoDeterminado mesmo com fontes, veio {:?}",
            v
        );
    }

    #[test]
    fn par_com_paths_diferentes_nao_e_colisao() {
        let a = no("M::X");
        let b = no("M::Y");
        let par = ParColidente { a: &a, b: &b };
        let viz = Vizinhanca::default();
        let v = investigar(par, &viz, None);
        match v {
            Veredito::NaoDeterminado { diagnostico } => {
                assert!(diagnostico.contains("não é realmente uma colisão"));
            }
            _ => panic!("esperava NaoDeterminado por programação errada"),
        }
    }
}
