//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/fontes-indexadas.md
//! @prompt-hash 67bea31b
//! @layer L1
//! @updated 2026-11-26

use std::collections::{BTreeMap, BTreeSet};

use crate::entities::grafo::{Aresta, Grafo, Kind, No, Relation, UsesKind, Visibility};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LocalFonte {
    pub arquivo: String,
    pub linha: usize,
    pub coluna: usize,
    pub linha_fim: usize,
    pub coluna_fim: usize,
}

#[derive(Debug, Default)]
pub struct SimboloIndexado {
    pub nome: String,
    pub declaracoes: BTreeSet<LocalFonte>,
    pub usos: BTreeSet<LocalFonte>,
    pub ffi: Option<VinculoFfi>,
}

#[derive(Debug)]
pub struct VinculoFfi {
    pub origem: OrigemFfi,
    pub nome_linker: String,
    pub implementacao: LocalFonte,
    pub chamadas_fronteira: ChamadasFonte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrigemFfi {
    Elf,
    IndexStore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncaoFonte {
    pub nome: String,
    pub declaracao: LocalFonte,
}

pub type ChamadasFonte = BTreeMap<LocalFonte, Option<FuncaoFonte>>;

#[derive(Debug)]
pub struct PromptFonte {
    pub caminho: String,
    pub texto: String,
    pub textfold: Option<LeituraTextFold>,
}

#[derive(Clone, Debug)]
pub struct LeituraTextFold {
    pub gerado_por_ia: bool,
    pub modelo: String,
    pub secoes: Vec<SecaoTextFold>,
}

#[derive(Clone, Debug)]
pub struct SecaoTextFold {
    pub titulo: String,
    pub profundidade: usize,
    pub topico: String,
    pub sintese: String,
    pub unidades: Vec<UnidadeTextFold>,
}

#[derive(Clone, Debug)]
pub struct UnidadeTextFold {
    pub topico: String,
    pub sintese: String,
    pub ponto: String,
    pub evidencias: Vec<String>,
    pub integral: String,
}

#[derive(Debug, Default)]
pub struct FontesIndexadas {
    pub fontes: BTreeMap<String, String>,
    pub prompts: BTreeMap<String, PromptFonte>,
    pub simbolos: Vec<SimboloIndexado>,
    pub indexadores: BTreeSet<String>,
    pub usos_sem_declaracao: usize,
    pub usos_ambiguos: usize,
    pub posicoes_invalidas: usize,
    pub artefato_ffi: Option<String>,
    pub chamadores: BTreeMap<LocalFonte, ChamadasFonte>,
    pub definicoes_nativas: BTreeMap<LocalFonte, String>,
}

pub type ParesIndexados<'a> = BTreeMap<(&'a str, &'a str), Vec<(usize, usize)>>;

impl FontesIndexadas {
    pub fn indexar_chamadores(
        &mut self,
        chamadas: &ChamadasFonte,
        chamadas_ffi_nativas: &BTreeMap<LocalFonte, ChamadasFonte>,
        corpos_nativos: &BTreeMap<LocalFonte, String>,
    ) {
        self.indexar_chamadores_com_fronteiras(
            chamadas,
            chamadas_ffi_nativas,
            corpos_nativos,
            &BTreeMap::new(),
        );
    }

    pub fn indexar_chamadores_com_fronteiras(
        &mut self,
        chamadas: &ChamadasFonte,
        chamadas_ffi_nativas: &BTreeMap<LocalFonte, ChamadasFonte>,
        corpos_nativos: &BTreeMap<LocalFonte, String>,
        fronteiras_confirmadas: &BTreeMap<usize, BTreeSet<LocalFonte>>,
    ) {
        self.chamadores.clear();
        self.definicoes_nativas.clear();
        let entradas_ffi: BTreeSet<_> = self
            .simbolos
            .iter()
            .filter_map(|simbolo| simbolo.ffi.as_ref().map(|ffi| ffi.implementacao.clone()))
            .collect();
        if entradas_ffi.is_empty() {
            return;
        }
        let destinos_nativos: BTreeSet<_> = corpos_nativos.keys().chain(&entradas_ffi).collect();
        let funcoes: BTreeSet<_> = chamadas
            .values()
            .flatten()
            .map(|f| &f.declaracao)
            .chain(destinos_nativos.iter().copied())
            .collect();
        let mut simbolos_por_declaracao = BTreeMap::new();
        let mut simbolos_por_uso = BTreeMap::new();
        for (id, simbolo) in self.simbolos.iter().enumerate() {
            for local in &simbolo.declaracoes {
                if let Some(&funcao) = funcoes.get(local) {
                    simbolos_por_declaracao
                        .entry(funcao)
                        .and_modify(|alvo| *alvo = None)
                        .or_insert(Some(id));
                }
            }
            for local in &simbolo.usos {
                if let Some((uso, _)) = chamadas.get_key_value(local) {
                    simbolos_por_uso
                        .entry(uso)
                        .and_modify(|alvo| *alvo = None)
                        .or_insert(Some(id));
                }
            }
        }
        let usos_resolvidos = |id: usize, usos: &BTreeSet<LocalFonte>| -> ChamadasFonte {
            usos.iter()
                .filter(|local| simbolos_por_uso.get(*local) == Some(&Some(id)))
                .map(|local| (local.clone(), chamadas[local].clone()))
                .collect()
        };
        let mut nativas = BTreeMap::new();
        let mut sucessores = BTreeMap::<_, Vec<_>>::new();
        for destino in destinos_nativos.iter().copied() {
            let mut usos = ChamadasFonte::new();
            if let Some(&Some(id)) = simbolos_por_declaracao.get(destino) {
                let simbolo = &self.simbolos[id];
                let multiplos_corpos = simbolo
                    .declaracoes
                    .iter()
                    .filter(|local| destinos_nativos.contains(local))
                    .nth(1)
                    .is_some();
                if !multiplos_corpos {
                    usos = usos_resolvidos(id, &simbolo.usos);
                }
            }
            if let Some(confirmadas) = chamadas_ffi_nativas.get(destino) {
                usos.extend(
                    confirmadas
                        .iter()
                        .filter(|(local, _)| !simbolos_por_uso.contains_key(*local))
                        .map(|(local, chamador)| (local.clone(), chamador.clone())),
                );
            }
            for chamador in usos.values().flatten() {
                sucessores
                    .entry(chamador.declaracao.clone())
                    .or_default()
                    .push(destino);
            }
            nativas.insert(destino, usos);
        }

        let mut descendentes: Vec<_> = entradas_ffi.iter().collect();
        let mut visitados = BTreeSet::new();
        while let Some(destino) = descendentes.pop() {
            if !visitados.insert(destino) {
                continue;
            }
            if let Some(nome) = corpos_nativos.get(destino) {
                self.definicoes_nativas
                    .insert(destino.clone(), nome.clone());
            }
            descendentes.extend(sucessores.get(destino).into_iter().flatten().copied());
        }
        let mut pendentes: Vec<_> = visitados.into_iter().cloned().collect();
        for (id, simbolo) in self.simbolos.iter_mut().enumerate() {
            if let Some(ffi) = &mut simbolo.ffi {
                ffi.chamadas_fronteira = usos_resolvidos(id, &simbolo.usos);
                ffi.chamadas_fronteira.extend(
                    fronteiras_confirmadas
                        .get(&id)
                        .into_iter()
                        .flatten()
                        .filter_map(|local| Some((local.clone(), chamadas.get(local)?.clone()))),
                );
                pendentes.extend(
                    ffi.chamadas_fronteira
                        .values()
                        .flatten()
                        .map(|f| f.declaracao.clone()),
                );
            }
        }
        let mut visitados = BTreeSet::new();
        while let Some(funcao) = pendentes.pop() {
            if !visitados.insert(funcao.clone()) {
                continue;
            }
            let chamadores = if let Some(chamadas) = nativas.remove(&funcao) {
                chamadas
            } else {
                let Some(&Some(id)) = simbolos_por_declaracao.get(&funcao) else {
                    continue;
                };
                usos_resolvidos(id, &self.simbolos[id].usos)
            };
            pendentes.extend(chamadores.values().flatten().map(|f| f.declaracao.clone()));
            self.chamadores.insert(funcao, chamadores);
        }
    }

    pub fn pares(&self) -> ParesIndexados<'_> {
        let mut pares = ParesIndexados::new();
        for (simbolo_id, simbolo) in self.simbolos.iter().enumerate() {
            let destinos: BTreeSet<_> = simbolo
                .declaracoes
                .iter()
                .chain(simbolo.ffi.iter().map(|vinculo| &vinculo.implementacao))
                .map(|local| local.arquivo.as_str())
                .collect();
            for (uso_id, uso) in simbolo.usos.iter().enumerate() {
                for &destino in &destinos {
                    if uso.arquivo != destino {
                        pares
                            .entry((uso.arquivo.as_str(), destino))
                            .or_default()
                            .push((simbolo_id, uso_id));
                    }
                }
            }
        }
        pares
    }

    pub fn grafo(&self) -> Grafo {
        let mut grafo = Grafo::new("SCIP");
        let ids: BTreeMap<_, _> = self
            .fontes
            .keys()
            .enumerate()
            .map(|(id, arquivo)| (arquivo.as_str(), id + 1))
            .collect();
        grafo.nodes = ids
            .iter()
            .map(|(&arquivo, &id)| No {
                id,
                path: arquivo.into(),
                name: arquivo.to_owned(),
                kind: Kind::Mod,
                modificadores: Default::default(),
                visibility: Visibility::Priv,
                crate_name: grafo.crate_name.clone(),
                trait_: None,
                trait_ref: None,
                cfg: None,
                macro_kind: None,
                is_non_exhaustive: false,
                position: None,
            })
            .collect();
        for ((de, para), registros) in self.pares() {
            let aresta = Aresta {
                from: de.into(),
                id_from: ids[de],
                to: para.into(),
                id_to: ids[para],
                relation: Relation::Uses,
                uses_kind: Some(UsesKind::Reference),
            };
            grafo
                .edges
                .extend(std::iter::repeat_n(aresta, registros.len()));
        }
        grafo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(arquivo: &str, linha: usize) -> LocalFonte {
        LocalFonte {
            arquivo: arquivo.into(),
            linha,
            coluna: 0,
            linha_fim: linha,
            coluna_fim: 1,
        }
    }

    #[test]
    fn projeta_cada_uso_uma_vez_por_arquivo_declarador() {
        let indice = FontesIndexadas {
            fontes: BTreeMap::from([("a.c".into(), String::new()), ("b.c".into(), String::new())]),
            simbolos: vec![SimboloIndexado {
                nome: "f".into(),
                declaracoes: BTreeSet::from([local("b.c", 1), local("b.c", 2)]),
                usos: BTreeSet::from([local("a.c", 1), local("b.c", 3)]),
                ffi: None,
            }],
            ..Default::default()
        };
        assert_eq!(
            indice.pares(),
            BTreeMap::from([(("a.c", "b.c"), vec![(0, 0)])])
        );
        assert_eq!(indice.grafo().edges.len(), 1);
        assert_eq!(indice.simbolos[0].usos.len(), 2);
    }

    #[test]
    fn ffi_acrescenta_a_implementacao_e_mantem_a_declaracao_rust() {
        let mut indice = FontesIndexadas::default();
        indice.simbolos.push(SimboloIndexado {
            nome: "alias".into(),
            declaracoes: BTreeSet::from([local("a.rs", 1)]),
            usos: BTreeSet::from([local("a.rs", 2)]),
            ffi: Some(VinculoFfi {
                origem: OrigemFfi::Elf,
                nome_linker: "funcao_cpp".into(),
                implementacao: local("b.cpp", 3),
                chamadas_fronteira: BTreeMap::new(),
            }),
        });
        assert_eq!(
            indice.pares(),
            BTreeMap::from([(("a.rs", "b.cpp"), vec![(0, 0)])])
        );
        assert_eq!(
            indice.simbolos[0].declaracoes,
            BTreeSet::from([local("a.rs", 1)])
        );
    }

    #[test]
    fn chamadores_ffi_seguem_identidade_e_guardam_recursao_sem_repetir_nos() {
        let pos = |linha| local("a.rs", linha);
        let funcao = |nome: &str, linha| {
            Some(FuncaoFonte {
                nome: nome.into(),
                declaracao: pos(linha),
            })
        };
        let criar = |nome: &str, linha, usos: &[usize]| SimboloIndexado {
            nome: nome.into(),
            declaracoes: BTreeSet::from([pos(linha)]),
            usos: usos.iter().copied().map(pos).collect(),
            ..Default::default()
        };
        let mut extern_ = criar("externa", 1, &[22, 72]);
        extern_.ffi = Some(VinculoFfi {
            origem: OrigemFfi::Elf,
            nome_linker: "externa".into(),
            implementacao: local("b.c", 1),
            chamadas_fronteira: ChamadasFonte::new(),
        });
        let mut indice = FontesIndexadas {
            simbolos: vec![
                extern_,
                criar("folha", 20, &[32]),
                criar("meio", 30, &[33, 42]),
                criar("main", 40, &[]),
                criar("folha", 50, &[62]),
                criar("distinto", 60, &[72]),
                SimboloIndexado {
                    nome: "externa".into(),
                    declaracoes: BTreeSet::from([local("b.c", 1)]),
                    usos: BTreeSet::from([local("b.c", 12), local("b.c", 17)]),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let chamadas = ChamadasFonte::from([
            (pos(22), funcao("folha", 20)),
            (pos(32), funcao("meio", 30)),
            (pos(33), funcao("meio", 30)),
            (pos(42), funcao("main", 40)),
            (pos(62), funcao("distinto", 60)),
            (pos(72), None),
            (
                local("b.c", 12),
                Some(FuncaoFonte {
                    nome: "chamar_nativa".into(),
                    declaracao: local("b.c", 11),
                }),
            ),
        ]);
        indice.indexar_chamadores(&chamadas, &BTreeMap::new(), &BTreeMap::new());
        assert_eq!(
            indice.simbolos[0]
                .ffi
                .as_ref()
                .unwrap()
                .chamadas_fronteira
                .keys()
                .collect::<Vec<_>>(),
            [&pos(22)]
        );
        assert_eq!(
            indice.chamadores[&pos(30)].keys().collect::<Vec<_>>(),
            [&pos(33), &pos(42)]
        );
        assert!(indice.chamadores.contains_key(&pos(40)));
        assert!(!indice.chamadores.contains_key(&pos(50)));
        assert!(!indice.chamadores.contains_key(&pos(60)));
        assert_eq!(
            indice.chamadores[&local("b.c", 1)]
                .keys()
                .collect::<Vec<_>>(),
            [&local("b.c", 12)]
        );
    }

    #[test]
    fn chamada_nativa_suplementa_ffi_omitida_sem_sobrepor_identidade_scip() {
        let implementacao = local("b.cpp", 1);
        let chamada = local("b.cpp", 7);
        let conflitante = local("b.cpp", 8);
        let chamador = Some(FuncaoFonte {
            nome: "entrada".into(),
            declaracao: local("b.cpp", 6),
        });
        let locais = ChamadasFonte::from([
            (chamada.clone(), chamador.clone()),
            (conflitante.clone(), chamador),
        ]);
        let mut indice = FontesIndexadas {
            simbolos: vec![
                SimboloIndexado {
                    ffi: Some(VinculoFfi {
                        origem: OrigemFfi::Elf,
                        nome_linker: "externa".into(),
                        implementacao: implementacao.clone(),
                        chamadas_fronteira: ChamadasFonte::new(),
                    }),
                    ..Default::default()
                },
                SimboloIndexado {
                    declaracoes: BTreeSet::from([local("b.cpp", 2)]),
                    usos: BTreeSet::from([conflitante]),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        indice.indexar_chamadores(
            &locais,
            &BTreeMap::from([(implementacao.clone(), locais.clone())]),
            &BTreeMap::new(),
        );
        assert_eq!(
            indice.chamadores[&implementacao].keys().collect::<Vec<_>>(),
            [&chamada]
        );
        assert!(indice.pares().is_empty());
    }

    #[test]
    fn inclui_destino_folha_e_entrada_nativa_que_converge_depois_do_ffi() {
        let entrada = local("uma.c", 1);
        let alternativa = local("outra.cpp", 3);
        let encontro = local("comum.c", 9);
        let chamada_a = local("uma.c", 2);
        let chamada_b = local("outra.cpp", 4);
        let mut indice = FontesIndexadas {
            simbolos: vec![
                SimboloIndexado {
                    ffi: Some(VinculoFfi {
                        origem: OrigemFfi::Elf,
                        nome_linker: "inicio".into(),
                        implementacao: entrada.clone(),
                        chamadas_fronteira: ChamadasFonte::new(),
                    }),
                    ..Default::default()
                },
                SimboloIndexado {
                    declaracoes: BTreeSet::from([encontro.clone()]),
                    usos: BTreeSet::from([chamada_a.clone(), chamada_b.clone()]),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let chamadas = ChamadasFonte::from([
            (
                chamada_a.clone(),
                Some(FuncaoFonte {
                    nome: "inicio".into(),
                    declaracao: entrada,
                }),
            ),
            (
                chamada_b.clone(),
                Some(FuncaoFonte {
                    nome: "alternativa".into(),
                    declaracao: alternativa,
                }),
            ),
        ]);
        let corpos = BTreeMap::from([(encontro.clone(), "folha".into())]);
        indice.indexar_chamadores(&chamadas, &BTreeMap::new(), &corpos);
        assert_eq!(indice.definicoes_nativas[&encontro], "folha");
        assert_eq!(
            indice.chamadores[&encontro].keys().collect::<BTreeSet<_>>(),
            BTreeSet::from([&chamada_a, &chamada_b])
        );
    }
}
