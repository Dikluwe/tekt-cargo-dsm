//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-cobertura-cli.md
//! @prompt-hash c873d286
//! @layer L3
//! @updated 2026-11-26

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use lente_core::domain::fontes_indexadas::{CoberturaCli, FontesIndexadas};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifesto {
    versao: u32,
    revisao: String,
    testes: BTreeSet<String>,
    regras: BTreeMap<String, BTreeSet<String>>,
}

pub fn anexar(arquivo: &Path, indice: &mut FontesIndexadas) -> Result<(), String> {
    let texto = std::fs::read_to_string(arquivo).map_err(|erro| erro.to_string())?;
    indice.cobertura_cli = Some(admitir(&texto, &indice.fontes)?);
    Ok(())
}

fn admitir(texto: &str, fontes: &BTreeMap<String, String>) -> Result<CoberturaCli, String> {
    let manifesto: Manifesto = serde_json::from_str(texto).map_err(|erro| erro.to_string())?;
    if manifesto.versao != 1 {
        return Err(format!("versão {} não suportada", manifesto.versao));
    }
    if manifesto.revisao.trim().is_empty() || manifesto.testes.is_empty() {
        return Err("esperadas a revisão e as suítes medidas".into());
    }
    for caminho in manifesto.testes.iter().chain(manifesto.regras.keys()) {
        let relativo = !caminho.contains('\\')
            && !caminho.contains(':')
            && caminho
                .split('/')
                .all(|parte| !matches!(parte, "" | "." | ".."));
        if !relativo || !fontes.contains_key(caminho) {
            return Err(format!("arquivo ausente do índice: {caminho}"));
        }
    }
    for (regra, testes) in &manifesto.regras {
        if let Some(teste) = testes.difference(&manifesto.testes).next() {
            return Err(format!("suíte fora da coleta para {regra}: {teste}"));
        }
    }
    Ok(CoberturaCli {
        revisao: manifesto.revisao,
        testes: manifesto.testes,
        regras: manifesto.regras,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFESTO: &str = r#"{
        "versao": 1,
        "revisao": "0123456",
        "testes": ["tests/b.rs", "tests/a.rs"],
        "regras": {
            "src/a.rs": ["tests/b.rs", "tests/a.rs", "tests/b.rs"],
            "src/b.rs": []
        }
    }"#;

    fn fontes() -> BTreeMap<String, String> {
        ["tests/a.rs", "tests/b.rs", "src/a.rs", "src/b.rs"]
            .map(|caminho| (caminho.into(), String::new()))
            .into()
    }

    #[test]
    fn admite_coleta_sem_repeticoes_e_com_regra_sem_execucao() {
        let cobertura = admitir(MANIFESTO, &fontes()).unwrap();
        assert_eq!(cobertura.revisao, "0123456");
        assert_eq!(
            cobertura.regras["src/a.rs"]
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["tests/a.rs", "tests/b.rs"]
        );
        assert!(cobertura.regras["src/b.rs"].is_empty());
    }

    #[test]
    fn rejeita_versao_arquivo_e_escopo_invalidos() {
        for (antes, depois, erro) in [
            ("\"versao\": 1", "\"versao\": 2", "versão 2"),
            ("src/a.rs", "src/../a.rs", "ausente do índice"),
            (
                "tests/b.rs\", \"tests/a.rs\"],",
                "tests/a.rs\"],",
                "suíte fora da coleta",
            ),
        ] {
            let texto = MANIFESTO.replacen(antes, depois, 1);
            assert!(admitir(&texto, &fontes()).unwrap_err().contains(erro));
        }
    }
}
