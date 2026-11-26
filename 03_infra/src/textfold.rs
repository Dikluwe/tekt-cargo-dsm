//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-textfold.md
//! @prompt-hash 3527aeeb
//! @layer L3
//! @updated 2026-11-26

use std::collections::{BTreeMap, btree_map::Entry};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lente_core::domain::fontes_indexadas::{
    FontesIndexadas, LeituraTextFold, SecaoTextFold, UnidadeTextFold,
};
use serde::Deserialize;

use crate::scip::{caminho_em_nucleo, resolver_fonte};

#[derive(Deserialize)]
struct Documento {
    format: String,
    version: u32,
    provenance: Proveniencia,
    sections: Vec<Secao>,
}

#[derive(Deserialize)]
struct Proveniencia {
    ai_generated: bool,
    model: String,
}

#[derive(Deserialize)]
struct Secao {
    title: String,
    depth: usize,
    topic: String,
    takeaway: String,
    units: Vec<Unidade>,
}

#[derive(Deserialize)]
struct Unidade {
    topic: String,
    takeaway: String,
    key: String,
    full: String,
    sentences: Vec<String>,
    evidence: Vec<usize>,
}

pub fn anexar(raiz: &Path, indice: &mut FontesIndexadas) -> Result<(), String> {
    anexar_com(raiz, indice, executar)
}

fn anexar_com(
    raiz: &Path,
    indice: &mut FontesIndexadas,
    executar: impl Fn(&Path) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let raiz = fs::canonicalize(raiz).map_err(|e| format!("{}: {e}", raiz.display()))?;
    let mut cache = BTreeMap::<PathBuf, LeituraTextFold>::new();
    let mut leituras = Vec::with_capacity(indice.prompts.len());
    for prompt in indice.prompts.values() {
        let leitura = (|| {
            let arquivo = resolver_fonte(&raiz, &prompt.caminho)?;
            if !arquivo.strip_prefix(&raiz).is_ok_and(caminho_em_nucleo) {
                return Err("o caminho resolvido não é um prompt Tekt".into());
            }
            let conferir = |arquivo: &Path| -> Result<(), String> {
                let bytes = fs::read(arquivo).map_err(|e| e.to_string())?;
                if bytes != prompt.texto.as_bytes() {
                    return Err("o Markdown mudou durante a geração da DSM".into());
                }
                Ok(())
            };
            conferir(&arquivo)?;
            let doc = match cache.entry(arquivo) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => {
                    let doc = projetar(&executar(e.key())?)?;
                    conferir(e.key())?;
                    e.insert(doc)
                }
            };
            Ok::<_, String>(doc.clone())
        })()
        .map_err(|e| format!("{}: {e}", prompt.caminho))?;
        leituras.push(leitura);
    }
    for (prompt, leitura) in indice.prompts.values_mut().zip(leituras) {
        prompt.textfold = Some(leitura);
    }
    Ok(())
}

fn executar(arquivo: &Path) -> Result<Vec<u8>, String> {
    let resultado = Command::new("python3")
        .args(["-I", "-m", "textfold", "get"])
        .arg(arquivo)
        .args(["--json", "--no-generate"])
        .output()
        .map_err(|e| format!("não foi possível executar python3: {e}"))?;
    if !resultado.status.success() {
        let detalhe: String = String::from_utf8_lossy(&resultado.stderr)
            .trim()
            .chars()
            .take(600)
            .collect();
        return Err(format!(
            "python3 -m textfold ({}) {detalhe}",
            resultado.status
        ));
    }
    Ok(resultado.stdout)
}

fn projetar(json: &[u8]) -> Result<LeituraTextFold, String> {
    let doc: Documento = serde_json::from_slice(json).map_err(|e| format!("JSON TextFold: {e}"))?;
    if doc.format != "textfold/doc" || doc.version != 0 {
        return Err(format!(
            "formato TextFold incompatível: {} v{}",
            doc.format, doc.version
        ));
    }
    let secoes = doc
        .sections
        .into_iter()
        .map(projetar_secao)
        .collect::<Result<_, _>>()?;
    Ok(LeituraTextFold {
        gerado_por_ia: doc.provenance.ai_generated,
        modelo: doc.provenance.model,
        secoes,
    })
}

fn projetar_secao(secao: Secao) -> Result<SecaoTextFold, String> {
    let unidades = secao
        .units
        .into_iter()
        .map(|u| {
            let evidencias = u
                .evidence
                .iter()
                .map(|&i| {
                    u.sentences
                        .get(i)
                        .cloned()
                        .ok_or_else(|| format!("índice de evidência inválido: {i}"))
                })
                .collect::<Result<_, _>>()?;
            Ok::<_, String>(UnidadeTextFold {
                topico: u.topic,
                sintese: u.takeaway,
                ponto: u.key,
                evidencias,
                integral: u.full,
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(SecaoTextFold {
        titulo: secao.title,
        profundidade: secao.depth,
        topico: secao.topic,
        sintese: secao.takeaway,
        unidades,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::domain::fontes_indexadas::PromptFonte;
    use serde_json::{Value, json};
    use std::cell::Cell;

    fn documento() -> Value {
        json!({
            "format": "textfold/doc", "version": 0,
            "provenance": {"ai_generated": true, "model": "modelo-de-teste"},
            "sections": [{
                "title": "Verificação", "depth": 2, "topic": "Validação", "takeaway": "Valide a saída.",
                "units": [{
                    "topic": "Saída", "takeaway": "A saída é válida.", "key": "A saída é válida. Foi verificada.",
                    "full": "Primeira frase. Segunda frase. Terceira frase.",
                    "sentences": ["Primeira frase.", "Segunda frase.", "Terceira frase."], "evidence": [2, 0]
                }]
            }]
        })
    }

    #[test]
    fn projeta_camadas_e_evidencias_do_textfold() {
        let leitura = projetar(documento().to_string().as_bytes()).unwrap();
        assert!(leitura.gerado_por_ia);
        assert_eq!(leitura.modelo, "modelo-de-teste");
        let secao = &leitura.secoes[0];
        assert_eq!(secao.titulo, "Verificação");
        assert_eq!(secao.profundidade, 2);
        assert_eq!(secao.topico, "Validação");
        assert_eq!(secao.sintese, "Valide a saída.");
        let unidade = &secao.unidades[0];
        assert_eq!(unidade.ponto, "A saída é válida. Foi verificada.");
        assert_eq!(unidade.evidencias, ["Terceira frase.", "Primeira frase."]);
    }

    #[test]
    fn recusa_versao_e_evidencias_invalidas() {
        let mut doc = documento();
        doc["version"] = json!(1);
        assert!(
            projetar(doc.to_string().as_bytes())
                .unwrap_err()
                .contains("incompatível")
        );
        doc["version"] = json!(0);
        doc["sections"][0]["units"][0]["evidence"] = json!([3]);
        assert!(
            projetar(doc.to_string().as_bytes())
                .unwrap_err()
                .contains("evidência inválido")
        );
    }

    #[test]
    fn vincula_resultado_ao_prompt_admitido_e_deduplica_execucao() {
        let raiz =
            std::env::temp_dir().join(format!("lente-textfold-teste-{}", std::process::id()));
        let caminho = "00_nucleo/prompts/contrato.md";
        fs::create_dir_all(raiz.join("00_nucleo/prompts")).unwrap();
        let arquivo = raiz.join(caminho);
        let texto = "# Contrato\n";
        fs::write(&arquivo, texto).unwrap();
        let mut indice = FontesIndexadas::default();
        for nome in ["a.rs", "b.rs"] {
            indice.prompts.insert(
                nome.into(),
                PromptFonte {
                    caminho: caminho.into(),
                    texto: texto.into(),
                    textfold: None,
                },
            );
        }
        let chamadas = Cell::new(0);
        let esperado = arquivo.canonicalize().unwrap();
        anexar_com(&raiz, &mut indice, |arquivo| {
            assert_eq!(arquivo, esperado);
            chamadas.set(chamadas.get() + 1);
            Ok(documento().to_string().into_bytes())
        })
        .unwrap();
        assert_eq!(chamadas.get(), 1);
        assert!(indice.prompts.values().all(|p| p.textfold.is_some()));
        fs::write(&arquivo, "# Atualização\n").unwrap();
        let falha = anexar_com(&raiz, &mut indice, |_| panic!("fonte alterada"));
        fs::remove_dir_all(raiz).unwrap();
        assert!(falha.unwrap_err().contains("Markdown mudou"));
    }
}
