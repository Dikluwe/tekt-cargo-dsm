//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-explorar-chamadas-nativas.md
//! @prompt-hash b75b90c9
//! @layer L3
//! @updated 2026-11-26

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use lente_core::domain::{
    fontes_indexadas::{FontesIndexadas, FuncaoFonte, LocalFonte},
    grafo_chamadas::{CoberturaChamadasNativas, GrafoChamadas, ReferenciaChamada},
};
use serde::Deserialize;

use crate::scip_ffi::{Compilacao, flags_compilacao};

const LEITOR: &str = include_str!("leitor_chamadas_nativas.py");
type Opcoes = (PathBuf, Vec<String>);
type Ponto = (usize, usize); // Linha e coluna UTF-8, base 1.
type Definicao = (String, String, String, Ponto); // USR, qualificado, token, ponto.
type Sitio = (
    Ponto,
    Ponto,
    String,
    String,
    Option<(PathBuf, usize, usize)>,
);

struct Unidade {
    caminho: PathBuf,
    arquivo: String,
    opcoes: Opcoes,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Observacao {
    funcoes: Vec<Definicao>,
    chamadas: Vec<Sitio>,
    erro: Option<String>,
}

pub fn carregar(
    raiz: &Path,
    base: &Path,
    indice: &mut FontesIndexadas,
) -> Result<GrafoChamadas, String> {
    let raiz = raiz.canonicalize().map_err(|e| e.to_string())?;
    let base = base.canonicalize().map_err(|e| e.to_string())?;
    let entradas: Vec<Compilacao> =
        serde_json::from_slice(&fs::read(&base).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let mut cobertura = CoberturaChamadasNativas {
        analisadas: 0,
        falhas: BTreeMap::new(),
    };
    let unidades = selecionar_unidades(&raiz, base.parent().unwrap(), entradas, &mut cobertura);
    if unidades.is_empty() {
        return Err(erro_cobertura(&cobertura));
    }

    let (versao, observacoes) = ler_unidades(&unidades)?;
    let mut admitidas = Vec::new();
    for (unidade, observacao) in unidades.into_iter().zip(observacoes) {
        let Unidade {
            arquivo,
            opcoes: (diretorio, _),
            ..
        } = unidade;
        if let Some(erro) = observacao.erro {
            cobertura.falhas.insert(arquivo, erro);
            continue;
        }
        match crate::scip::ler_fonte(&raiz, &arquivo) {
            Ok(fonte)
                if indice
                    .fontes
                    .get(&arquivo)
                    .is_none_or(|anterior| anterior == &fonte) =>
            {
                indice.fontes.entry(arquivo.clone()).or_insert(fonte);
                cobertura.analisadas += 1;
                admitidas.push((arquivo, diretorio, observacao));
            }
            Ok(_) => {
                cobertura
                    .falhas
                    .insert(arquivo, "fonte diverge do snapshot SCIP".into());
            }
            Err(erro) => {
                cobertura.falhas.insert(arquivo, erro);
            }
        }
    }
    if cobertura.analisadas == 0 {
        return Err(erro_cobertura(&cobertura));
    }
    let mut grafo = projetar(&raiz, &indice.fontes, &versao, admitidas);
    grafo.cobertura_nativa = Some(cobertura);
    Ok(grafo)
}

fn erro_cobertura(cobertura: &CoberturaChamadasNativas) -> String {
    let primeira = cobertura.falhas.first_key_value();
    primeira.map_or_else(
        || "nenhuma unidade C/C++ local na base de compilação".into(),
        |(arquivo, erro)| format!("nenhuma unidade C/C++ analisada; {arquivo}: {erro}"),
    )
}

fn selecionar_unidades(
    raiz: &Path,
    pasta: &Path,
    entradas: Vec<Compilacao>,
    cobertura: &mut CoberturaChamadasNativas,
) -> Vec<Unidade> {
    let mut grupos = BTreeMap::<_, BTreeSet<_>>::new();
    for entrada in entradas {
        let diretorio = pasta.join(&entrada.directory);
        let Ok(caminho) = diretorio.join(&entrada.file).canonicalize() else {
            continue;
        };
        let Ok(relativo) = caminho.strip_prefix(raiz) else {
            continue;
        };
        let Some(arquivo) = relativo.to_str() else {
            continue;
        };
        let extensao = caminho.extension().and_then(|ext| ext.to_str());
        if !extensao.is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "c" | "cc" | "cp" | "cpp" | "cxx" | "c++"
            )
        }) {
            continue;
        }
        let opcoes = diretorio.canonicalize().ok().and_then(|diretorio| {
            flags_compilacao(&entrada, &diretorio, &caminho).map(|flags| (diretorio, flags))
        });
        grupos
            .entry((caminho.clone(), arquivo.to_owned()))
            .or_default()
            .insert(opcoes);
    }
    grupos
        .into_iter()
        .filter_map(|((caminho, arquivo), variantes)| {
            let erro = if variantes.len() != 1 {
                "configurações de compilação divergentes"
            } else if let Some(opcoes) = variantes.into_iter().next().flatten() {
                return Some(Unidade {
                    caminho,
                    arquivo,
                    opcoes,
                });
            } else {
                "opções de compilação fora do conjunto suportado"
            };
            cobertura.falhas.insert(arquivo, erro.into());
            None
        })
        .collect()
}

fn ler_unidades(unidades: &[Unidade]) -> Result<(String, Vec<Observacao>), String> {
    let pedido: Vec<_> = unidades
        .iter()
        .map(|u| (&u.caminho, &u.opcoes.0, &u.opcoes.1))
        .collect();
    let pedido = serde_json::to_vec(&pedido).map_err(|e| e.to_string())?;
    let mut processo = Command::new("python3")
        .args(["-I", "-S", "-X", "utf8", "-c", LEITOR])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("python3: {e}"))?;
    let escritura = processo.stdin.take().unwrap().write_all(&pedido);
    let saida = processo.wait_with_output().map_err(|e| e.to_string())?;
    if !saida.status.success() {
        return Err(String::from_utf8_lossy(&saida.stderr).trim().to_owned());
    }
    escritura.map_err(|e| e.to_string())?;
    let texto = std::str::from_utf8(&saida.stdout).map_err(|e| e.to_string())?;
    let mut linhas = texto.lines();
    let versao = serde_json::from_str(linhas.next().ok_or("resposta libclang vazia")?)
        .map_err(|e| e.to_string())?;
    let observacoes: Vec<_> = linhas
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    if observacoes.len() != unidades.len() {
        return Err("cobertura incompleta na resposta do leitor libclang".into());
    }
    Ok((versao, observacoes))
}

fn posicao(
    arquivo: &str,
    linhas: &[&str],
    (linha, coluna): Ponto,
    token: &str,
) -> Option<LocalFonte> {
    let texto = linhas.get(linha.checked_sub(1)?)?;
    let coluna = coluna.checked_sub(1)?;
    let coluna_fim = coluna.checked_add(token.len())?;
    let identificador = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    if token.is_empty()
        || texto.get(coluna..coluna_fim)? != token
        || texto[..coluna]
            .chars()
            .next_back()
            .is_some_and(identificador)
        || texto[coluna_fim..]
            .chars()
            .next()
            .is_some_and(identificador)
    {
        return None;
    }
    Some(LocalFonte {
        arquivo: arquivo.into(),
        linha,
        coluna,
        linha_fim: linha,
        coluna_fim,
    })
}

fn projetar(
    raiz: &Path,
    fontes: &BTreeMap<String, String>,
    versao: &str,
    observacoes: Vec<(String, PathBuf, Observacao)>,
) -> GrafoChamadas {
    let linhas: BTreeMap<_, Vec<_>> = fontes
        .iter()
        .map(|(arquivo, fonte)| (arquivo.as_str(), fonte.lines().collect()))
        .collect();
    let mut definicoes = BTreeMap::<String, BTreeSet<_>>::new();
    let mut funcoes = Vec::new();
    let mut ids = BTreeMap::new();
    for (arquivo, _, observacao) in &observacoes {
        for (usr, nome, token, ponto) in &observacao.funcoes {
            let Some(declaracao) = posicao(arquivo, &linhas[arquivo.as_str()], *ponto, token)
            else {
                continue;
            };
            definicoes
                .entry(usr.clone())
                .or_default()
                .insert(declaracao.clone());
            ids.entry((arquivo.clone(), *ponto)).or_insert_with(|| {
                let id = funcoes.len();
                funcoes.push(FuncaoFonte {
                    nome: nome.clone(),
                    declaracao,
                });
                id
            });
        }
    }

    let origem = format!("libclang · {versao} · base de compilação");
    let mut sitios = BTreeMap::new();
    let mut referencias = Vec::new();
    for (arquivo, diretorio, observacao) in observacoes {
        let locais = &linhas[arquivo.as_str()];
        for (envolvente, ponto, token, usr, declaracao) in observacao.chamadas {
            let Some(&de) = ids.get(&(arquivo.clone(), envolvente)) else {
                continue;
            };
            let Some(uso) = posicao(&arquivo, locais, ponto, &token) else {
                continue;
            };
            sitios.insert(uso.clone(), de);
            if let Some(destinos) = definicoes.get(&usr) {
                referencias.extend(destinos.iter().map(|destino| ReferenciaChamada {
                    uso: uso.clone(),
                    destino: Some(destino.clone()),
                    origem: origem.clone(),
                }));
                continue;
            }
            let destino = declaracao.and_then(|(caminho, linha, coluna)| {
                let absoluto = diretorio.join(caminho);
                let arquivo = absoluto.strip_prefix(raiz).ok()?.to_str()?;
                posicao(arquivo, linhas.get(arquivo)?, (linha, coluna), &token)
            });
            referencias.push(ReferenciaChamada {
                uso,
                destino,
                origem: origem.clone(),
            });
        }
    }
    GrafoChamadas::resolver(funcoes, sitios, referencias)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::domain::grafo_chamadas::MotivoSemSeta;

    #[test]
    fn resolve_por_usr_entre_arquivos_e_distingue_declaracao_sem_corpo() {
        let raiz = Path::new("/projeto");
        let fontes = [
            (
                "uso.cpp".into(),
                "void origem() { alvo(1); outro(); }".into(),
            ),
            (
                "alvo.cpp".into(),
                "void alvo(int) {}\nvoid alvo(float) {}".into(),
            ),
            ("api.h".into(), "void outro();".into()),
        ]
        .into();
        let grafo = projetar(
            raiz,
            &fontes,
            "teste",
            vec![
                (
                    "alvo.cpp".into(),
                    raiz.into(),
                    Observacao {
                        funcoes: vec![
                            ("alvo_int".into(), "alvo".into(), "alvo".into(), (1, 6)),
                            ("alvo_float".into(), "alvo".into(), "alvo".into(), (2, 6)),
                        ],
                        ..Default::default()
                    },
                ),
                (
                    "uso.cpp".into(),
                    raiz.into(),
                    Observacao {
                        funcoes: vec![("origem".into(), "origem".into(), "origem".into(), (1, 6))],
                        chamadas: vec![
                            ((1, 6), (1, 17), "alvo".into(), "alvo_int".into(), None),
                            (
                                (1, 6),
                                (1, 26),
                                "outro".into(),
                                "outro".into(),
                                Some(("api.h".into(), 1, 6)),
                            ),
                        ],
                        ..Default::default()
                    },
                ),
            ],
        );
        assert_eq!(grafo.chamadas.len(), 1);
        assert_eq!(grafo.funcoes[grafo.chamadas[0].para].declaracao.linha, 1);
        assert_eq!(grafo.chamadas[0].uso.coluna, 16);
        assert_eq!(grafo.diagnosticos.len(), 1);
        assert_eq!(grafo.diagnosticos[0].motivo, MotivoSemSeta::ForaDoRecorte);
    }
}
