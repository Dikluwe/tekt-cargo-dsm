//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-scip-raizes-rust.md
//! @prompt-hash 27b13cd8
//! @layer L3
//! @updated 2026-11-26

use std::collections::{BTreeMap, BTreeSet};

use lente_core::domain::fontes_indexadas::{LocalFonte, SimboloIndexado};
use scip::types::descriptor::Suffix;
use syn::visit::{self, Visit};

use super::{Simbolos, declara_em_varios_arquivos};

pub(super) fn raiz_do_rust_analyzer(id: &str) -> bool {
    id.starts_with("rust-analyzer ") && id.ends_with(" crate/")
        && scip::symbol::parse_symbol(id).is_ok_and(|s| {
            matches!(s.descriptors.as_slice(), [d] if d.name == "crate" && d.suffix.enum_value() == Ok(Suffix::Namespace))
        })
}

#[derive(Default)]
pub(super) struct RaizesRust {
    pais: BTreeMap<String, BTreeSet<String>>,
    com_pais_ambiguos: BTreeSet<String>,
}

impl RaizesRust {
    pub(super) fn indexar(fontes: &BTreeMap<String, String>, simbolos: &Simbolos) -> Self {
        let modulos: BTreeMap<_, _> = fontes
            .iter()
            .filter_map(|(arquivo, texto)| {
                let ext = std::path::Path::new(arquivo).extension()?;
                (ext == "rs").then(|| (arquivo.as_str(), modulos_externos(texto)))
            })
            .collect();
        let mut pais = BTreeMap::<String, BTreeSet<String>>::new();
        let mut com_pais_ambiguos = BTreeSet::new();
        for ((_, id), simbolo) in simbolos {
            if !id.starts_with("rust-analyzer ") || !id.ends_with('/') {
                continue;
            }
            let Some(definicao) = simbolo.declaracoes.first() else {
                continue;
            };
            let ambiguo = declara_em_varios_arquivos(simbolo);
            for uso in &simbolo.usos {
                let externo = modulos
                    .get(uso.arquivo.as_str())
                    .is_some_and(|posicoes| posicoes.contains(&(uso.linha, uso.coluna)));
                if !externo {
                    continue;
                }
                if ambiguo {
                    com_pais_ambiguos.extend(simbolo.declaracoes.iter().map(|d| d.arquivo.clone()));
                } else if uso.arquivo != definicao.arquivo {
                    pais.entry(definicao.arquivo.clone())
                        .or_default()
                        .insert(uso.arquivo.clone());
                }
            }
        }
        Self {
            pais,
            com_pais_ambiguos,
        }
    }

    pub(super) fn resolver<'a>(
        &'a self,
        uso: &'a LocalFonte,
        candidatas: &BTreeMap<String, SimboloIndexado>,
        fontes: &BTreeMap<String, String>,
    ) -> Option<&'a str> {
        if uso.linha != uso.linha_fim || token(uso, fontes) != Some("crate") {
            return None;
        }
        let mut pendentes = vec![uso.arquivo.as_str()];
        let mut vistos = BTreeSet::new();
        let mut raiz = None;
        while let Some(arquivo) = pendentes.pop() {
            if !vistos.insert(arquivo) {
                continue;
            }
            if self.com_pais_ambiguos.contains(arquivo) {
                return None;
            }
            if candidatas.contains_key(arquivo) && raiz.replace(arquivo).is_some() {
                return None;
            }
            if let Some(pais) = self.pais.get(arquivo) {
                pendentes.extend(pais.iter().map(String::as_str));
            }
        }
        raiz
    }
}

fn token<'a>(uso: &LocalFonte, fontes: &'a BTreeMap<String, String>) -> Option<&'a str> {
    fontes
        .get(&uso.arquivo)?
        .lines()
        .nth(uso.linha.checked_sub(1)?)?
        .get(uso.coluna..uso.coluna_fim)
}

fn modulos_externos(texto: &str) -> BTreeSet<(usize, usize)> {
    struct Modulos<'a> {
        linhas: Vec<&'a str>,
        posicoes: BTreeSet<(usize, usize)>,
    }
    impl<'ast> Visit<'ast> for Modulos<'_> {
        fn visit_item_mod(&mut self, modulo: &'ast syn::ItemMod) {
            if modulo.content.is_none() {
                let posicao = modulo.ident.span().start();
                if let Some(linha) = self.linhas.get(posicao.line.saturating_sub(1))
                    && let Some((coluna, _)) = linha.char_indices().nth(posicao.column)
                {
                    self.posicoes.insert((posicao.line, coluna));
                }
            }
            visit::visit_item_mod(self, modulo);
        }
    }
    let Ok(arquivo) = syn::parse_file(texto) else {
        return BTreeSet::new();
    };
    let mut modulos = Modulos {
        linhas: texto.lines().collect(),
        posicoes: BTreeSet::new(),
    };
    modulos.visit_file(&arquivo);
    modulos.posicoes
}
