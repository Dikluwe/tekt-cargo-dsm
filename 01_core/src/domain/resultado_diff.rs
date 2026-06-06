//! Lineage: prompt 00_nucleo/prompt/0047-resultado_diff_orquestracao_json.md
//! Spec:    00_nucleo/specs/forma-organizada.md
//! Camada:  L1 — Núcleo. Apenas stdlib. Sem I/O. **Sem `serde`.**
//!
//! O resultado **view-agnóstico** do modo `--diff`: um dado completo que carrega
//! tudo que as três vistas (resumo / por-item / camadas, prompt 0048) precisam.
//! A serialização JSON **não** mora aqui — é L2 (a CLI mapeia este tipo L1 para
//! JSON, como a trilha global já faz). Manter `serde` fora preserva a pureza L1.
//!
//! Carrega **os dois níveis** de raio: o raio por nó tocado (`tocados[].raio`,
//! para a vista por-item) e o raio **combinado** (`combinado`, união dos raios
//! dos tocados, para a vista resumo). Os agrupamentos por crate as vistas
//! derivam do 1º segmento do path; o tipo guarda listas planas.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use crate::domain::mapeamento::NoTocado;
use crate::domain::raio::Raio;
use crate::domain::uniao::Fantasma;
use crate::entities::grafo::Path;

/// Um nó tocado pelo diff, com seu raio de impacto calculado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocadoComRaio {
    pub tocado: NoTocado,
    pub raio: Raio,
}

/// A união dos raios dos tocados (para a vista resumo). Cada path aparece uma
/// vez, com a profundidade **mínima** (o caminho mais próximo entre os tocados).
/// Ordenado por path — determinístico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaioCombinado {
    pub montante: Vec<(Path, usize)>,
    pub jusante: Vec<(Path, usize)>,
}

/// O resultado completo do modo `--diff`, view-agnóstico (prompt 0047).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoDiff {
    /// Cada nó tocado + seu raio (para a vista por-item).
    pub tocados: Vec<TocadoComRaio>,
    /// A união dos raios (para a vista resumo).
    pub combinado: RaioCombinado,
    /// Censo do untracked (0046/0043): no grafo (compilado).
    pub ligados: Vec<PathBuf>,
    /// Censo: `.rs` em membro mas fora do grafo (presente, não compilado).
    pub soltos: Vec<PathBuf>,
    /// Censo: fora de membro, ou não-`.rs`.
    pub nao_fonte: Vec<PathBuf>,
    /// Fantasmas do grafo de workspace (0045) — esperado vazio neste repo (0041).
    pub fantasmas: Vec<Fantasma>,
}

/// Une os raios de vários nós num [`RaioCombinado`]: para `montante` e
/// `jusante`, a união por path com a **profundidade mínima** (o mais próximo),
/// ordenada por path. Puro e determinístico.
pub fn combinar_raios(raios: &[Raio]) -> RaioCombinado {
    RaioCombinado {
        montante: combinar_mapas(raios.iter().map(|r| &r.montante)),
        jusante: combinar_mapas(raios.iter().map(|r| &r.jusante)),
    }
}

/// Une vários mapas path→profundidade mantendo a menor profundidade por path.
/// O `BTreeMap` dá a ordenação por path (determinística) de graça.
fn combinar_mapas<'a>(
    mapas: impl Iterator<Item = &'a HashMap<Path, usize>>,
) -> Vec<(Path, usize)> {
    let mut menor: BTreeMap<Path, usize> = BTreeMap::new();
    for mapa in mapas {
        for (path, &prof) in mapa {
            menor
                .entry(path.clone())
                .and_modify(|atual| {
                    if prof < *atual {
                        *atual = prof;
                    }
                })
                .or_insert(prof);
        }
    }
    menor.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::raio::Classificacao;

    /// Raio mínimo com montante/jusante dados (os demais campos não importam
    /// para `combinar_raios`).
    fn raio_de(
        alvo: &str,
        montante: &[(&str, usize)],
        jusante: &[(&str, usize)],
    ) -> Raio {
        Raio {
            alvo: Path::from(alvo),
            classificacao: Classificacao::Intermediario,
            uses_entrada: montante.len(),
            uses_saida: jusante.len(),
            montante: montante
                .iter()
                .map(|(p, d)| (Path::from(*p), *d))
                .collect(),
            jusante: jusante.iter().map(|(p, d)| (Path::from(*p), *d)).collect(),
            owns_pai: None,
            owns_filhos: Vec::new(),
        }
    }

    #[test]
    fn combinar_une_sem_repeticao_e_ordena() {
        let r1 = raio_de("A", &[("X", 1), ("Y", 2)], &[("D", 1)]);
        let r2 = raio_de("B", &[("Y", 3), ("Z", 1)], &[("E", 2)]);
        let c = combinar_raios(&[r1, r2]);
        // montante: X(1), Y(menor entre 2 e 3 = 2), Z(1) — ordenado por path.
        assert_eq!(
            c.montante,
            vec![
                (Path::from("X"), 1),
                (Path::from("Y"), 2),
                (Path::from("Z"), 1),
            ]
        );
        assert_eq!(c.jusante, vec![(Path::from("D"), 1), (Path::from("E"), 2)]);
    }

    #[test]
    fn combinar_pega_profundidade_minima() {
        // Y aparece a 5 num raio e a 2 noutro → fica 2 (o mais próximo).
        let r1 = raio_de("A", &[("Y", 5)], &[]);
        let r2 = raio_de("B", &[("Y", 2)], &[]);
        let c = combinar_raios(&[r1, r2]);
        assert_eq!(c.montante, vec![(Path::from("Y"), 2)]);
    }

    #[test]
    fn combinar_e_deterministico() {
        let mk = || {
            vec![
                raio_de("A", &[("X", 1), ("Y", 2)], &[("D", 1)]),
                raio_de("B", &[("Z", 1)], &[("D", 3)]),
            ]
        };
        assert_eq!(combinar_raios(&mk()), combinar_raios(&mk()));
    }

    #[test]
    fn combinar_vazio_da_combinado_vazio() {
        let c = combinar_raios(&[]);
        assert!(c.montante.is_empty() && c.jusante.is_empty());
    }
}
