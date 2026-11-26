//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/grafo-chamadas.md
//! @prompt-hash b14770cc
//! @layer L1
//! @updated 2026-11-26

use std::collections::{BTreeMap, BTreeSet};

use super::fontes_indexadas::{FuncaoFonte, LocalFonte};

pub struct ReferenciaChamada {
    pub uso: LocalFonte,
    pub destino: Option<LocalFonte>,
    pub origem: String,
}

pub struct LigacaoChamada {
    pub de: usize,
    pub para: usize,
    pub uso: LocalFonte,
    pub origens: BTreeSet<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MotivoSemSeta {
    SemReferencia,
    SemDeclaracao,
    Ambigua,
    ForaDoRecorte,
}

pub struct DiagnosticoChamada {
    pub de: usize,
    pub uso: LocalFonte,
    pub motivo: MotivoSemSeta,
    pub candidatos: BTreeMap<LocalFonte, BTreeSet<String>>,
    pub origens: BTreeSet<String>,
}

pub struct CoberturaChamadasNativas {
    pub analisadas: usize,
    pub falhas: BTreeMap<String, String>,
}

#[derive(Default)]
pub struct GrafoChamadas {
    pub funcoes: Vec<FuncaoFonte>,
    pub chamadas: Vec<LigacaoChamada>,
    pub diagnosticos: Vec<DiagnosticoChamada>,
    pub cobertura_nativa: Option<CoberturaChamadasNativas>,
}

impl GrafoChamadas {
    pub fn anexar(&mut self, mut outro: Self) {
        let base = self.funcoes.len();
        self.funcoes.append(&mut outro.funcoes);
        self.chamadas
            .extend(outro.chamadas.into_iter().map(|mut chamada| {
                chamada.de += base;
                chamada.para += base;
                chamada
            }));
        self.diagnosticos
            .extend(outro.diagnosticos.into_iter().map(|mut diagnostico| {
                diagnostico.de += base;
                diagnostico
            }));
        self.cobertura_nativa = outro.cobertura_nativa.or(self.cobertura_nativa.take());
    }

    pub fn resolver(
        funcoes: Vec<FuncaoFonte>,
        sitios: BTreeMap<LocalFonte, usize>,
        referencias: impl IntoIterator<Item = ReferenciaChamada>,
    ) -> Self {
        let ids: BTreeMap<_, _> = funcoes
            .iter()
            .enumerate()
            .map(|(id, funcao)| (&funcao.declaracao, id))
            .collect();
        let mut por_uso = BTreeMap::<_, BTreeMap<_, BTreeSet<_>>>::new();
        for referencia in referencias {
            if sitios.contains_key(&referencia.uso) {
                por_uso
                    .entry(referencia.uso)
                    .or_default()
                    .entry(referencia.destino)
                    .or_default()
                    .insert(referencia.origem);
            }
        }

        let mut chamadas = Vec::new();
        let mut diagnosticos = Vec::new();
        for (uso, de) in sitios {
            let mut destinos = por_uso.remove(&uso).unwrap_or_default();
            let sem_declaracao = destinos.remove(&None);
            let candidatos: BTreeMap<_, _> = destinos
                .into_iter()
                .filter_map(|(local, origens)| Some((local?, origens)))
                .collect();
            let motivo = match candidatos.len() {
                0 if sem_declaracao.is_none() => MotivoSemSeta::SemReferencia,
                0 => MotivoSemSeta::SemDeclaracao,
                1 => {
                    let (destino, origens) = candidatos.first_key_value().unwrap();
                    if let Some(&para) = ids.get(destino) {
                        chamadas.push(LigacaoChamada {
                            de,
                            para,
                            uso,
                            origens: origens.clone(),
                        });
                        continue;
                    }
                    MotivoSemSeta::ForaDoRecorte
                }
                _ => MotivoSemSeta::Ambigua,
            };
            diagnosticos.push(DiagnosticoChamada {
                de,
                uso,
                motivo,
                candidatos,
                origens: sem_declaracao.unwrap_or_default(),
            });
        }
        Self {
            funcoes,
            chamadas,
            diagnosticos,
            cobertura_nativa: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ponto(linha: usize) -> LocalFonte {
        LocalFonte {
            arquivo: "a.go".into(),
            linha,
            coluna: 0,
            linha_fim: linha,
            coluna_fim: 1,
        }
    }

    #[test]
    fn separa_destinos_inequivocos_ausencias_e_ambiguidades_por_posicao() {
        let funcoes = [1, 3]
            .map(|linha| FuncaoFonte {
                nome: "f".into(),
                declaracao: ponto(linha),
            })
            .to_vec();
        let sitios = [(2, 0), (4, 1), (5, 1), (6, 1), (8, 1), (9, 1)]
            .map(|(linha, id)| (ponto(linha), id))
            .into();
        let referencias = [
            (2, Some(3), "a.scip"),
            (2, Some(3), "b.scip"),
            (4, Some(3), "a.scip"),
            (5, Some(1), "a.scip"),
            (5, Some(9), "b.scip"),
            (6, Some(8), "a.scip"),
            (7, Some(1), "a.scip"),
            (8, None, "a.scip"),
        ]
        .map(|(uso, destino, origem)| ReferenciaChamada {
            uso: ponto(uso),
            destino: destino.map(ponto),
            origem: origem.into(),
        });
        let grafo = GrafoChamadas::resolver(funcoes, sitios, referencias);
        let chamadas: Vec<_> = grafo
            .chamadas
            .iter()
            .map(|c| (c.de, c.para, c.uso.linha))
            .collect();
        assert_eq!(chamadas, [(0, 1, 2), (1, 1, 4)]);
        assert_eq!(
            grafo.chamadas[0].origens,
            BTreeSet::from(["a.scip".into(), "b.scip".into()])
        );
        let diagnosticos: Vec<_> = grafo
            .diagnosticos
            .iter()
            .map(|d| (d.uso.linha, &d.motivo, d.candidatos.len()))
            .collect();
        assert_eq!(
            diagnosticos,
            [
                (5, &MotivoSemSeta::Ambigua, 2),
                (6, &MotivoSemSeta::ForaDoRecorte, 1),
                (8, &MotivoSemSeta::SemDeclaracao, 0),
                (9, &MotivoSemSeta::SemReferencia, 0),
            ]
        );
    }
}
