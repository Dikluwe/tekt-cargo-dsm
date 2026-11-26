//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-chamadas-go.md
//! @prompt-hash 41c9cb8c
//! @layer L3
//! @updated 2026-11-26

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use lente_core::domain::{
    fontes_indexadas::{FontesIndexadas, FuncaoFonte, LocalFonte},
    grafo_chamadas::{GrafoChamadas, ReferenciaChamada},
};
use serde::Deserialize;
use serde_json::Value;

const LEITOR: &str = include_str!("leitor_chamadas_go.go");

#[derive(Default, Deserialize)]
struct Sintaxe {
    funcoes: Vec<Funcao>,
    chamadas: Vec<Chamada>,
}

#[derive(Deserialize)]
struct Funcao {
    arquivo: String,
    nome: String,
    local: [usize; 3],
}

#[derive(Deserialize)]
struct Chamada {
    origem: usize,
    local: [usize; 3],
}

pub fn projetar_scip(
    indice: &FontesIndexadas,
    referencias: Vec<ReferenciaChamada>,
) -> GrafoChamadas {
    let fontes = indice
        .fontes
        .iter()
        .map(|(arquivo, texto)| (arquivo.as_str(), texto.as_str()));
    projetar(fontes, referencias)
}

pub fn projetar_legado(json: &str, caminho: &Path) -> GrafoChamadas {
    let Ok(indice) = serde_json::from_str::<Value>(json) else {
        return GrafoChamadas::default();
    };
    let Some(fontes) = indice["sources"].as_object() else {
        return GrafoChamadas::default();
    };
    let fontes = fontes
        .iter()
        .filter_map(|(arquivo, texto)| Some((arquivo.as_str(), texto.as_str()?)));
    let links = ["links", "search_links"].into_iter().flat_map(|chave| {
        indice[chave]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(move |par| {
                Some((
                    par["from"].as_str()?,
                    par["to"].as_str()?,
                    par["references"].as_array()?,
                    chave,
                ))
            })
    });
    let locais = indice["local_references"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(arquivo, rs)| {
            Some((
                arquivo.as_str(),
                arquivo.as_str(),
                rs.as_array()?,
                "local_references",
            ))
        });
    let nome_arquivo = caminho
        .file_name()
        .unwrap_or(caminho.as_os_str())
        .to_string_lossy();
    let referencias = links.chain(locais).flat_map(|(de, para, rs, campo)| {
        let origem = format!("Índice Go · {nome_arquivo} · {campo}");
        rs.iter().filter_map(move |r| {
            let nome = r["symbol"].as_str()?.rsplit('.').next()?;
            let pos = |arquivo, linha: &str, coluna: &str| {
                Some(local(
                    arquivo,
                    [
                        r[linha].as_u64()?.try_into().ok()?,
                        usize::try_from(r[coluna].as_u64()?).ok()?.checked_sub(1)?,
                        nome.len(),
                    ],
                ))
            };
            Some(ReferenciaChamada {
                uso: pos(de, "use_line", "use_column")?,
                destino: pos(para, "definition_line", "definition_column"),
                origem: origem.clone(),
            })
        })
    });
    projetar(fontes, referencias)
}

fn local(arquivo: &str, [linha, coluna, tamanho]: [usize; 3]) -> LocalFonte {
    LocalFonte {
        arquivo: arquivo.into(),
        linha,
        coluna,
        linha_fim: linha,
        coluna_fim: coluna.saturating_add(tamanho),
    }
}

fn projetar<'a>(
    fontes: impl Iterator<Item = (&'a str, &'a str)>,
    referencias: impl IntoIterator<Item = ReferenciaChamada>,
) -> GrafoChamadas {
    let fontes: BTreeMap<_, _> = fontes
        .filter(|(arquivo, _)| arquivo.ends_with(".go"))
        .collect();
    let Some(sintaxe) = extrair(&fontes) else {
        return GrafoChamadas::default();
    };
    let funcoes: Vec<_> = sintaxe
        .funcoes
        .into_iter()
        .map(|f| FuncaoFonte {
            nome: f.nome,
            declaracao: local(&f.arquivo, f.local),
        })
        .collect();
    let sitios = sintaxe
        .chamadas
        .into_iter()
        .filter_map(|c| {
            let origem = funcoes.get(c.origem)?;
            Some((local(&origem.declaracao.arquivo, c.local), c.origem))
        })
        .collect();
    GrafoChamadas::resolver(funcoes, sitios, referencias)
}

fn extrair(fontes: &BTreeMap<&str, &str>) -> Option<Sintaxe> {
    if fontes.is_empty() {
        return None;
    }
    let entrada = serde_json::to_vec(fontes).ok()?;
    let agora = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let arquivo = std::env::temp_dir().join(format!(
        "lente-chamadas-{}-{}.go",
        std::process::id(),
        agora.as_nanos()
    ));
    let mut script = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&arquivo)
        .ok()?;
    let escrita = script.write_all(LEITOR.as_bytes());
    drop(script);
    let resultado = (|| {
        escrita.ok()?;
        let mut processo = Command::new("go")
            .arg("run")
            .arg(&arquivo)
            .env("GO111MODULE", "off")
            .env("GOWORK", "off")
            .env("GOTOOLCHAIN", "local")
            .env("GOENV", "off")
            .env_remove("GOOS")
            .env_remove("GOARCH")
            .env("CGO_ENABLED", "0")
            .env("GOFLAGS", "")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        if processo.stdin.take()?.write_all(&entrada).is_err() {
            let _ = processo.kill();
        }
        let saida = processo.wait_with_output().ok()?;
        if !saida.status.success() {
            return None;
        }
        serde_json::from_slice(&saida.stdout).ok()
    })();
    let _ = fs::remove_file(arquivo);
    resultado
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_separa_chamada_de_valor_e_funcao_anonima() {
        if Command::new("go").arg("version").output().is_err() {
            return;
        }
        let fonte = "package a\nfunc alvo() {}\nfunc origem() {\n alvo()\n _ = alvo\n _ = func() { alvo() }\n}\n";
        let entrada = serde_json::json!({
            "sources": {"a.go": fonte},
            "local_references": {"a.go": [
                {"symbol": "alvo", "use_line": 4, "use_column": 2, "definition_line": 2, "definition_column": 6},
                {"symbol": "alvo", "use_line": 5, "use_column": 6, "definition_line": 2, "definition_column": 6},
                {"symbol": "alvo", "use_line": 6, "use_column": 15, "definition_line": 2, "definition_column": 6}
            ]}
        });
        let grafo = projetar_legado(&entrada.to_string(), Path::new("referencias.json"));
        assert_eq!(grafo.funcoes.len(), 2);
        let [chamada] = grafo.chamadas.as_slice() else {
            panic!("esperava exatamente uma chamada");
        };
        assert_eq!((chamada.de, chamada.para), (1, 0));
        assert_eq!(chamada.uso, local("a.go", [4, 1, 4]));
        assert_eq!(
            chamada.origens,
            ["Índice Go · referencias.json · local_references".into()].into()
        );
        assert!(grafo.diagnosticos.is_empty());
    }
}
