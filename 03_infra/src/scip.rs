//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-scip.md
//! @prompt-hash 57fdd1af
//! @layer L3
//! @updated 2026-11-26

use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use ::scip::types::{
    Document, Index, Occurrence, PositionEncoding, SymbolRole, occurrence::Typed_range,
};
use lente_core::domain::{
    fontes_indexadas::{FontesIndexadas, LocalFonte, PromptFonte, SimboloIndexado},
    grafo_chamadas::ReferenciaChamada,
};
use protobuf::Message;

pub fn carregar_indices(raiz: &Path, indices: &[PathBuf]) -> Result<FontesIndexadas, String> {
    carregar_indices_com_referencias_go(raiz, indices).map(|(fontes, _)| fontes)
}

pub fn carregar_indices_com_referencias_go(
    raiz: &Path,
    indices: &[PathBuf],
) -> Result<(FontesIndexadas, Vec<ReferenciaChamada>), String> {
    let raiz = fs::canonicalize(raiz).map_err(|e| format!("{}: {e}", raiz.display()))?;
    let mut fontes = FontesIndexadas::default();
    let mut simbolos = BTreeMap::new();
    let mut usos_go = UsosGo::default();
    for arquivo in indices {
        let mut leitor =
            BufReader::new(File::open(arquivo).map_err(|e| format!("{}: {e}", arquivo.display()))?);
        let indice = Index::parse_from_reader(&mut leitor)
            .map_err(|e| format!("{}: {e}", arquivo.display()))?;
        let ferramenta = &indice.metadata.tool_info;
        if !ferramenta.name.is_empty() {
            fontes.indexadores.insert(
                format!("{} {}", ferramenta.name, ferramenta.version)
                    .trim()
                    .into(),
            );
        }
        let nome = arquivo
            .file_name()
            .unwrap_or(arquivo.as_os_str())
            .to_string_lossy();
        let rotulo = format!("SCIP · {nome}");
        let ferramenta_exibida = format!("{} {}", ferramenta.name, ferramenta.version);
        let origem = if ferramenta_exibida.trim().is_empty() {
            rotulo
        } else {
            format!("{rotulo} · {}", ferramenta_exibida.trim())
        };
        for documento in indice.documents {
            carregar_documento(
                &raiz,
                &ferramenta.name,
                &documento,
                &mut fontes,
                &mut simbolos,
                &origem,
                &mut usos_go,
            )?;
        }
    }
    if fontes.fontes.is_empty() {
        return Err("nenhum documento local nos índices SCIP".into());
    }
    let mut referencias = Vec::new();
    for (id, mut simbolo) in simbolos {
        if let Some(usos) = usos_go.remove(&id) {
            for (uso, origens) in usos {
                for origem in origens {
                    if simbolo.declaracoes.is_empty() {
                        referencias.push(ReferenciaChamada {
                            uso: uso.clone(),
                            destino: None,
                            origem,
                        });
                    } else {
                        referencias.extend(simbolo.declaracoes.iter().filter(|&d| d != &uso).map(
                            |d| ReferenciaChamada {
                                uso: uso.clone(),
                                destino: Some(d.clone()),
                                origem: origem.clone(),
                            },
                        ));
                    }
                }
            }
        }
        if simbolo.declaracoes.is_empty() {
            fontes.usos_sem_declaracao += simbolo.usos.len();
        } else {
            simbolo
                .usos
                .retain(|local| !simbolo.declaracoes.contains(local));
            fontes.simbolos.push(simbolo);
        }
    }
    Ok((fontes, referencias))
}

type Identidade = (Option<String>, String);
type Simbolos = BTreeMap<Identidade, SimboloIndexado>;
type UsosGo = BTreeMap<Identidade, BTreeMap<LocalFonte, BTreeSet<String>>>;

fn identidade(arquivo: &str, simbolo: &str) -> (Option<String>, String) {
    (
        simbolo.starts_with("local ").then(|| arquivo.to_owned()),
        simbolo.to_owned(),
    )
}

pub(super) fn resolver_fonte(raiz: &Path, arquivo: &str) -> Result<PathBuf, String> {
    if arquivo
        .chars()
        .any(|c| c.is_control() || c == '\\' || c == ':')
        || arquivo
            .split('/')
            .any(|parte| matches!(parte, "" | "." | ".."))
    {
        return Err(format!("caminho inválido no índice SCIP: {arquivo:?}"));
    }
    let caminho = fs::canonicalize(raiz.join(arquivo)).map_err(|e| format!("{arquivo}: {e}"))?;
    if !caminho.starts_with(raiz) || !caminho.is_file() {
        return Err(format!(
            "documento SCIP precisa apontar para um arquivo dentro da raiz: {arquivo}"
        ));
    }
    Ok(caminho)
}

pub(super) fn ler_fonte(raiz: &Path, arquivo: &str) -> Result<String, String> {
    fs::read_to_string(resolver_fonte(raiz, arquivo)?).map_err(|e| format!("{arquivo}: {e}"))
}

fn prompt_declarado(fonte: &str) -> Option<&str> {
    let mut resto = fonte.trim_start_matches('\u{feff}').trim_start();
    let mut prompt = None;
    while !resto.is_empty() {
        let (comentario, proximo) = if let Some(bloco) = resto.strip_prefix("/*") {
            bloco.split_once("*/")?
        } else if let Some(bloco) = resto.strip_prefix("<!--") {
            bloco.split_once("-->")?
        } else if let Some(linha) = resto.strip_prefix("//").or_else(|| resto.strip_prefix('#')) {
            linha.split_once('\n').unwrap_or((linha, ""))
        } else {
            break;
        };
        for linha in comentario.lines() {
            let texto = linha
                .trim_start()
                .trim_start_matches(['!', '/', '*'])
                .trim();
            if let Some(caminho) = texto
                .strip_prefix("@prompt")
                .filter(|s| s.is_empty() || s.starts_with(char::is_whitespace))
                && prompt.replace(caminho.trim()).is_some()
            {
                return None;
            }
        }
        resto = proximo.trim_start();
    }
    prompt
}

pub(super) fn caminho_em_nucleo(caminho: &Path) -> bool {
    caminho.extension().is_some_and(|ext| ext == "md")
        && (caminho.starts_with("00_nucleo/prompts") || caminho.starts_with("00_nucleo/prompt"))
}

pub fn anexar_prompts(raiz: &Path, indice: &mut FontesIndexadas) {
    indice.prompts.clear();
    let Ok(raiz) = fs::canonicalize(raiz) else {
        return;
    };
    for (arquivo, fonte) in &indice.fontes {
        let Some(caminho) = prompt_declarado(fonte) else {
            continue;
        };
        if !caminho_em_nucleo(Path::new(caminho)) {
            continue;
        }
        let Ok(resolvido) = resolver_fonte(&raiz, caminho) else {
            continue;
        };
        if !resolvido.strip_prefix(&raiz).is_ok_and(caminho_em_nucleo) {
            continue;
        }
        let Ok(texto) = fs::read_to_string(resolvido) else {
            continue;
        };
        indice.prompts.insert(
            arquivo.clone(),
            PromptFonte {
                caminho: caminho.into(),
                texto,
                textfold: None,
            },
        );
    }
}

fn carregar_documento(
    raiz: &Path,
    ferramenta: &str,
    documento: &Document,
    fontes: &mut FontesIndexadas,
    simbolos: &mut Simbolos,
    origem: &str,
    usos_go: &mut UsosGo,
) -> Result<(), String> {
    let arquivo = &documento.relative_path;
    let texto = match fontes.fontes.entry(arquivo.clone()) {
        Entry::Occupied(fonte) => fonte.into_mut(),
        Entry::Vacant(fonte) => fonte.insert(ler_fonte(raiz, arquivo)?),
    };
    if !documento.text.is_empty() && documento.text != *texto {
        return Err(format!(
            "fonte local diverge do texto no índice SCIP: {arquivo}"
        ));
    }
    let unidade = match (documento.position_encoding.enum_value(), ferramenta) {
        (
            Ok(PositionEncoding::UnspecifiedPositionEncoding),
            "rust-analyzer" | "scip-clang" | "scip-go" | "scip-zig",
        ) => PositionEncoding::UTF8CodeUnitOffsetFromLineStart,
        (
            Ok(PositionEncoding::UnspecifiedPositionEncoding),
            "scip-python" | "scip-java" | "scip-typescript" | "scip-dotnet" | "scip-dart",
        ) => PositionEncoding::UTF16CodeUnitOffsetFromLineStart,
        (Ok(codificacao), _) if codificacao != PositionEncoding::UnspecifiedPositionEncoding => {
            codificacao
        }
        _ => {
            return Err(format!(
                "codificação da posição SCIP ausente ou desconhecida: {arquivo}"
            ));
        }
    };
    let linhas: Vec<_> = texto
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    for info in &documento.symbols {
        if !info.symbol.is_empty() && !info.display_name.is_empty() {
            simbolos
                .entry(identidade(arquivo, &info.symbol))
                .or_default()
                .nome
                .clone_from(&info.display_name);
        }
    }
    for ocorrencia in &documento.occurrences {
        if ocorrencia.symbol.is_empty() {
            continue;
        }
        let Some(local) = local_da_ocorrencia(arquivo, ocorrencia, unidade, &linhas) else {
            fontes.posicoes_invalidas += 1;
            continue;
        };
        let simbolo = simbolos
            .entry(identidade(arquivo, &ocorrencia.symbol))
            .or_default();
        if simbolo.nome.is_empty() {
            let linha = linhas[local.linha - 1];
            let fim = if local.linha == local.linha_fim {
                local.coluna_fim
            } else {
                linha.len()
            };
            let trecho = linha[local.coluna..fim].trim();
            simbolo.nome = if trecho.is_empty() {
                arquivo.clone()
            } else {
                trecho.into()
            };
        }
        let locais = if ocorrencia.symbol_roles & SymbolRole::Definition as i32 != 0 {
            &mut simbolo.declaracoes
        } else {
            if arquivo.ends_with(".go") {
                usos_go
                    .entry(identidade(arquivo, &ocorrencia.symbol))
                    .or_default()
                    .entry(local.clone())
                    .or_default()
                    .insert(origem.to_owned());
            }
            &mut simbolo.usos
        };
        locais.insert(local);
    }
    Ok(())
}

fn local_da_ocorrencia(
    arquivo: &str,
    ocorrencia: &Occurrence,
    unidade: PositionEncoding,
    linhas: &[&str],
) -> Option<LocalFonte> {
    let (linha, coluna, linha_fim, coluna_fim) = match &ocorrencia.typed_range {
        Some(Typed_range::SingleLineRange(r)) => {
            (r.line, r.start_character, r.line, r.end_character)
        }
        Some(Typed_range::MultiLineRange(r)) => {
            (r.start_line, r.start_character, r.end_line, r.end_character)
        }
        Some(_) => return None,
        None => match ocorrencia.range.as_slice() {
            &[l, c, fim] => (l, c, l, fim),
            &[l, c, lf, cf] => (l, c, lf, cf),
            _ => return None,
        },
    };
    let linha = usize::try_from(linha).ok()?;
    let linha_fim = usize::try_from(linha_fim).ok()?;
    let coluna = coluna_em_bytes(linhas.get(linha)?, coluna, unidade)?;
    let coluna_fim = coluna_em_bytes(linhas.get(linha_fim)?, coluna_fim, unidade)?;
    ((linha, coluna) <= (linha_fim, coluna_fim)).then(|| LocalFonte {
        arquivo: arquivo.into(),
        linha: linha + 1,
        coluna,
        linha_fim: linha_fim + 1,
        coluna_fim,
    })
}

fn coluna_em_bytes(linha: &str, offset: i32, unidade: PositionEncoding) -> Option<usize> {
    use PositionEncoding::*;
    let offset = usize::try_from(offset).ok()?;
    if unidade == UTF8CodeUnitOffsetFromLineStart {
        return linha.is_char_boundary(offset).then_some(offset);
    }
    let mut unidades = 0;
    for (byte, caractere) in linha.char_indices() {
        if unidades == offset {
            return Some(byte);
        }
        unidades += if unidade == UTF16CodeUnitOffsetFromLineStart {
            caractere.len_utf16()
        } else {
            1
        };
    }
    (unidades == offset).then_some(linha.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colunas_respeitam_unicode_e_rejeitam_offsets_entre_unidades() {
        use PositionEncoding::*;
        assert_eq!(
            coluna_em_bytes("🚀éx", 6, UTF8CodeUnitOffsetFromLineStart),
            Some(6)
        );
        assert_eq!(
            coluna_em_bytes("🚀éx", 3, UTF16CodeUnitOffsetFromLineStart),
            Some(6)
        );
        assert_eq!(
            coluna_em_bytes("🚀éx", 2, UTF32CodeUnitOffsetFromLineStart),
            Some(6)
        );
        assert_eq!(
            coluna_em_bytes("🚀éx", 1, UTF16CodeUnitOffsetFromLineStart),
            None
        );
        assert_eq!(
            coluna_em_bytes("🚀éx", 5, UTF8CodeUnitOffsetFromLineStart),
            None
        );
        assert_eq!(
            coluna_em_bytes("🚀éx", 20, UTF8CodeUnitOffsetFromLineStart),
            None
        );
    }

    #[test]
    fn caminhos_e_simbolos_locais_tem_fronteiras_explicitas() {
        assert!(ler_fonte(Path::new("."), "../fora").is_err());
        assert!(ler_fonte(Path::new("."), "/absoluto").is_err());
        assert_ne!(identidade("a.rs", "local 0"), identidade("b.rs", "local 0"));
        assert_eq!(identidade("a.rs", "global"), identidade("b.rs", "global"));
    }

    #[test]
    fn prompt_vem_de_uma_unica_declaracao_no_cabecalho() {
        let caminho = "00_nucleo/prompts/modulo com espaço.md";
        for fonte in [
            format!("\u{feff}//! Linhagem\r\n//! @prompt {caminho}\r\n//! @prompt-hash 00000000"),
            format!("#!/usr/bin/env python3\n# @prompt {caminho}\nprint('ok')"),
            format!("/**\n * @prompt {caminho}\n */\nint x;"),
            format!("<!-- @prompt {caminho} -->\n<div></div>"),
        ] {
            assert_eq!(prompt_declarado(&fonte), Some(caminho));
        }
        assert_eq!(
            prompt_declarado(&format!("fn f() {{}}\n// @prompt {caminho}")),
            None
        );
        assert_eq!(
            prompt_declarado(&format!("//! @prompt {caminho}\n//! @prompt outro.md")),
            None
        );
    }

    #[test]
    fn anexa_somente_markdown_do_nucleo_sem_afetar_o_grafo() {
        let raiz = std::env::temp_dir().join(format!("lente-prompts-{}", std::process::id()));
        fs::create_dir(&raiz).unwrap();
        fs::create_dir_all(raiz.join("00_nucleo/prompts")).unwrap();
        fs::create_dir(raiz.join("00_nucleo/prompt")).unwrap();
        let texto = "# Contrato\n\n## Verificação\nTexto integral.\n";
        fs::write(raiz.join("00_nucleo/prompts/a.md"), texto).unwrap();
        fs::write(raiz.join("00_nucleo/prompt/historico.md"), texto).unwrap();
        fs::write(raiz.join("outro.md"), "fora do núcleo").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("../../outro.md", raiz.join("00_nucleo/prompts/atalho.md"))
            .unwrap();
        let mut indice = FontesIndexadas::default();
        for (arquivo, caminho) in [
            ("a.rs", "00_nucleo/prompts/a.md"),
            ("historico.rs", "00_nucleo/prompt/historico.md"),
            ("externo.rs", "outro.md"),
            ("ausente.rs", "00_nucleo/prompts/ausente.md"),
            ("relativo.rs", "00_nucleo/prompts/../../outro.md"),
            ("symlink.rs", "00_nucleo/prompts/atalho.md"),
        ] {
            indice
                .fontes
                .insert(arquivo.into(), format!("//! @prompt {caminho}\n"));
        }
        anexar_prompts(&raiz, &mut indice);
        fs::remove_dir_all(raiz).unwrap();
        assert_eq!(
            indice
                .prompts
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["a.rs", "historico.rs"]
        );
        assert_eq!(indice.prompts["a.rs"].caminho, "00_nucleo/prompts/a.md");
        assert_eq!(indice.prompts["a.rs"].texto, texto);
        assert_eq!(indice.grafo().nodes.len(), indice.fontes.len());
    }
}
