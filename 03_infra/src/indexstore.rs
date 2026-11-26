//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-indexstore.md
//! @prompt-hash 90667ff9
//! @layer L3
//! @updated 2026-11-26

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use lente_core::domain::fontes_indexadas::{
    ChamadasFonte, FontesIndexadas, FuncaoFonte, LocalFonte, OrigemFfi, SimboloIndexado, VinculoFfi,
};
use serde::Deserialize;

const LEITOR: &str = include_str!("leitor_indexstore.py");

#[derive(Deserialize)]
struct Documento {
    arquivo: String,
    ocorrencias: Vec<Ocorrencia>,
}

#[derive(Deserialize)]
struct Ocorrencia {
    usr: String,
    nome: String,
    linguagem: u32,
    linha: usize,
    coluna: usize,
    definicao: bool,
    chamada: bool,
    chamadores: Vec<String>,
}

#[derive(Default)]
struct Uso {
    nome: String,
    chamada: bool,
    chamadores: BTreeSet<String>,
}

#[derive(Default)]
struct Simbolo {
    definicoes: BTreeMap<LocalFonte, (String, u32)>,
    usos: BTreeMap<LocalFonte, Uso>,
}

pub fn anexar(raiz: &Path, caminho: &Path, destino: &mut FontesIndexadas) -> Result<(), String> {
    let raiz = fs::canonicalize(raiz).map_err(|e| format!("{}: {e}", raiz.display()))?;
    let saida = Command::new("python3")
        .args(["-I", "-S", "-X", "utf8", "-c", LEITOR])
        .arg(&raiz)
        .arg(caminho)
        .output()
        .map_err(|e| format!("python3: {e}"))?;
    if !saida.status.success() {
        return Err(String::from_utf8_lossy(&saida.stderr).trim().to_owned());
    }
    let documentos: Vec<Documento> =
        serde_json::from_slice(&saida.stdout).map_err(|e| e.to_string())?;
    let mut indice = FontesIndexadas::default();
    let mut entradas = BTreeMap::<String, Simbolo>::new();
    for documento in documentos {
        let arquivo = documento.arquivo;
        if !indice.fontes.contains_key(&arquivo) {
            let fonte = super::scip::ler_fonte(&raiz, &arquivo)?;
            if destino
                .fontes
                .get(&arquivo)
                .is_some_and(|anterior| *anterior != fonte)
            {
                return Err(format!("fontes divergem entre os índices: {arquivo}"));
            }
            indice.fontes.insert(arquivo.clone(), fonte);
        }
        let linhas: Vec<_> = indice.fontes[&arquivo].lines().collect();
        for ocorrencia in documento.ocorrencias {
            let Some((local, nome)) = localizar(&arquivo, &linhas, &ocorrencia) else {
                continue;
            };
            let simbolo = entradas.entry(ocorrencia.usr).or_default();
            if ocorrencia.definicao {
                simbolo
                    .definicoes
                    .insert(local, (nome.into(), ocorrencia.linguagem));
            } else {
                let uso = simbolo.usos.entry(local).or_default();
                uso.nome = nome.into();
                uso.chamada |= ocorrencia.chamada;
                uso.chamadores.extend(ocorrencia.chamadores);
            }
        }
    }
    if !indice
        .fontes
        .keys()
        .any(|arquivo| arquivo.ends_with(".swift"))
    {
        return Err("nenhuma fonte Swift local no IndexStore".into());
    }
    indice.indexadores.insert("Swift/Clang IndexStore".into());
    projetar(&mut indice, &entradas);
    combinar(destino, indice);
    Ok(())
}

fn localizar<'a>(
    arquivo: &str,
    linhas: &[&'a str],
    ocorrencia: &Ocorrencia,
) -> Option<(LocalFonte, &'a str)> {
    let texto = linhas.get(ocorrencia.linha.checked_sub(1)?)?;
    let coluna = ocorrencia.coluna.checked_sub(1)?;
    let resto = texto.get(coluna..)?;
    let nome = if let Some(escapado) = resto.strip_prefix('\u{60}') {
        let fim = escapado.find('\u{60}')? + 2;
        resto.get(..fim)?
    } else {
        let fim = resto
            .char_indices()
            .find(|(_, c)| !c.is_alphanumeric() && *c != '_' && *c != '$')
            .map_or(resto.len(), |(i, _)| i);
        resto.get(..fim).filter(|s| !s.is_empty())?
    };
    let esperado = ocorrencia.nome.split(['(', ':']).next().unwrap_or("");
    let construtor_swift =
        !ocorrencia.definicao && arquivo.ends_with(".swift") && esperado == "init";
    if !construtor_swift && nome.trim_matches('\u{60}') != esperado {
        return None;
    }
    Some((
        LocalFonte {
            arquivo: arquivo.into(),
            linha: ocorrencia.linha,
            coluna,
            linha_fim: ocorrencia.linha,
            coluna_fim: coluna + nome.len(),
        },
        nome,
    ))
}

fn unico<T>(mut itens: impl Iterator<Item = T>) -> Option<T> {
    let primeiro = itens.next()?;
    itens.next().is_none().then_some(primeiro)
}

fn projetar(indice: &mut FontesIndexadas, entradas: &BTreeMap<String, Simbolo>) {
    let definicoes: BTreeMap<_, _> = entradas
        .iter()
        .filter_map(|(usr, simbolo)| Some((usr, unico(simbolo.definicoes.iter())?)))
        .collect();
    let mut locais_chamada = BTreeMap::<&LocalFonte, BTreeSet<&String>>::new();
    for (usr, simbolo) in entradas {
        for (local, uso) in &simbolo.usos {
            if uso.chamada {
                locais_chamada.entry(local).or_default().insert(usr);
            }
        }
    }
    let mut chamadas = ChamadasFonte::new();
    let mut corpos = BTreeMap::new();
    for (usr, simbolo) in entradas {
        let Some((declaracao, (nome, linguagem))) = definicoes.get(usr).copied() else {
            indice.usos_sem_declaracao += simbolo.usos.len();
            continue;
        };
        let nativo = matches!(linguagem, 0..=2) && !declaracao.arquivo.ends_with(".swift");
        if nativo {
            corpos.insert(declaracao.clone(), nome.clone());
        }
        let mut usos = BTreeSet::new();
        let mut fronteiras = BTreeMap::<&str, BTreeSet<LocalFonte>>::new();
        for (local, uso) in &simbolo.usos {
            let chamada =
                uso.chamada && locais_chamada.get(local).is_some_and(|ids| ids.len() == 1);
            if chamada {
                let chamador = unico(uso.chamadores.iter())
                    .and_then(|usr| definicoes.get(usr))
                    .map(|(declaracao, (nome, _))| FuncaoFonte {
                        nome: nome.clone(),
                        declaracao: (*declaracao).clone(),
                    });
                chamadas.insert(local.clone(), chamador);
            }
            if chamada && nativo && local.arquivo.ends_with(".swift") {
                fronteiras
                    .entry(&uso.nome)
                    .or_default()
                    .insert(local.clone());
            } else {
                usos.insert(local.clone());
            }
        }
        indice.simbolos.push(SimboloIndexado {
            nome: nome.clone(),
            declaracoes: BTreeSet::from([declaracao.clone()]),
            usos,
            ffi: None,
        });
        for (alias, usos) in fronteiras {
            indice.simbolos.push(SimboloIndexado {
                nome: alias.into(),
                declaracoes: BTreeSet::new(),
                usos,
                ffi: Some(VinculoFfi {
                    origem: OrigemFfi::IndexStore,
                    nome_linker: nome.clone(),
                    implementacao: declaracao.clone(),
                    chamadas_fronteira: ChamadasFonte::new(),
                }),
            });
        }
    }
    indice.indexar_chamadores(&chamadas, &BTreeMap::new(), &corpos);
}

fn combinar(destino: &mut FontesIndexadas, origem: FontesIndexadas) {
    let mut simbolos = BTreeMap::<LocalFonte, Option<usize>>::new();
    for (i, simbolo) in destino.simbolos.iter().enumerate() {
        for declaracao in &simbolo.declaracoes {
            simbolos
                .entry(declaracao.clone())
                .and_modify(|i| *i = None)
                .or_insert(Some(i));
        }
    }
    for simbolo in origem.simbolos {
        if let Some(Some(i)) = simbolo
            .declaracoes
            .first()
            .and_then(|local| simbolos.get(local))
        {
            destino.simbolos[*i].usos.extend(simbolo.usos);
            continue;
        }
        if let Some(ffi) = &simbolo.ffi
            && let Some(Some(i)) = simbolos.get(&ffi.implementacao)
            && destino.simbolos[*i].ffi.is_none()
        {
            destino.simbolos[*i]
                .usos
                .retain(|local| !simbolo.usos.contains(local));
        }
        destino.simbolos.push(simbolo);
    }
    for (local, chamadas) in origem.chamadores {
        let destino_chamadas = destino.chamadores.entry(local).or_default();
        for (uso, chamador) in chamadas {
            let anterior = destino_chamadas.entry(uso).or_insert(None);
            if anterior.is_none() {
                *anterior = chamador;
            }
        }
    }
    destino.definicoes_nativas.extend(origem.definicoes_nativas);
    destino.indexadores.extend(origem.indexadores);
    destino.fontes.extend(origem.fontes);
    destino.usos_sem_declaracao += origem.usos_sem_declaracao;
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
            coluna_fim: 3,
        }
    }

    #[test]
    fn preserva_implementacoes_de_identidades_diferentes_com_o_mesmo_nome() {
        let chamada_a = local("App.swift", 11);
        let chamada_b = local("App.swift", 12);
        let impl_a = local("Core.cpp", 5);
        let impl_b = local("Core.cpp", 9);
        let entradas = BTreeMap::from([
            (
                "c:@F@somar#I#".into(),
                funcao(impl_a.clone(), chamada_a.clone()),
            ),
            (
                "c:@F@somar#d#".into(),
                funcao(impl_b.clone(), chamada_b.clone()),
            ),
        ]);
        let mut indice = FontesIndexadas::default();
        projetar(&mut indice, &entradas);
        let destinos: BTreeMap<_, _> = indice
            .simbolos
            .iter()
            .filter_map(|s| {
                let ffi = s.ffi.as_ref()?;
                let uso = unico(ffi.chamadas_fronteira.keys())?;
                Some((uso.clone(), (ffi.origem, ffi.implementacao.clone())))
            })
            .collect();
        assert_eq!(
            destinos,
            BTreeMap::from([
                (chamada_a, (OrigemFfi::IndexStore, impl_a)),
                (chamada_b, (OrigemFfi::IndexStore, impl_b)),
            ])
        );
    }

    fn funcao(implementacao: LocalFonte, chamada: LocalFonte) -> Simbolo {
        Simbolo {
            definicoes: BTreeMap::from([(implementacao, ("somar".into(), 0))]),
            usos: BTreeMap::from([(
                chamada,
                Uso {
                    nome: "somar".into(),
                    chamada: true,
                    ..Uso::default()
                },
            )]),
        }
    }

    #[test]
    fn ignora_conflito_no_ponto_de_chamada_e_metodo_sem_corpo() {
        let chamada = local("App.swift", 11);
        let ident1 = funcao(local("Core.mm", 5), chamada.clone());
        let mut ident2 = funcao(local("Core.mm", 9), chamada);
        ident2.definicoes.clear();
        let entradas = BTreeMap::from([
            ("c:objc(cs)A(cm)somar:".into(), ident1),
            ("c:objc(cs)B(cm)somar:".into(), ident2),
        ]);
        let mut indice = FontesIndexadas::default();
        projetar(&mut indice, &entradas);
        assert!(indice.simbolos.iter().all(|s| s.ffi.is_none()));
        assert_eq!(indice.usos_sem_declaracao, 1);
    }

    #[test]
    fn valida_coluna_em_bytes_e_grafia_da_ocorrencia() {
        let fonte = "/* 🚀 */ somar(1)";
        let o = Ocorrencia {
            usr: "c:@F@somar#I#".into(),
            nome: "somar(_:)".into(),
            linguagem: 100,
            linha: 1,
            coluna: fonte.find("somar").unwrap() + 1,
            definicao: false,
            chamada: true,
            chamadores: Vec::new(),
        };
        let (local, nome) = localizar("App.swift", &[fonte], &o).unwrap();
        assert_eq!(&fonte[local.coluna..local.coluna_fim], nome);
        assert_eq!(nome, "somar");
        assert!(localizar("App.swift", &["/* 🚀 */ outra(1)"], &o).is_none());
    }
}
