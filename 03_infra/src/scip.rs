//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-scip.md
//! @prompt-hash bdb00e6e
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

mod raizes_rust;
use raizes_rust::{RaizesRust, raiz_do_rust_analyzer};

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
    let raizes = if simbolos
        .iter()
        .any(|((_, id), simbolo)| declara_em_varios_arquivos(simbolo) && raiz_do_rust_analyzer(id))
    {
        RaizesRust::indexar(&fontes.fontes, &simbolos)
    } else {
        RaizesRust::default()
    };
    let mut referencias = Vec::new();
    for (id, simbolo) in simbolos {
        // A exploração Go recebe os candidatos para diagnosticar ambiguidades.
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
        anexar_simbolo(&mut fontes, &raizes, &id.1, simbolo);
    }
    Ok((fontes, referencias))
}

type Identidade = (Option<String>, String);
type Simbolos = BTreeMap<Identidade, SimboloIndexado>;
type UsosGo = BTreeMap<Identidade, BTreeMap<LocalFonte, BTreeSet<String>>>;

fn declara_em_varios_arquivos(simbolo: &SimboloIndexado) -> bool {
    simbolo
        .declaracoes
        .first()
        .zip(simbolo.declaracoes.last())
        .is_some_and(|(primeira, ultima)| primeira.arquivo != ultima.arquivo)
}

fn anexar_simbolo(
    fontes: &mut FontesIndexadas,
    raizes: &RaizesRust,
    id: &str,
    mut simbolo: SimboloIndexado,
) {
    if simbolo.declaracoes.is_empty() {
        fontes.usos_sem_declaracao += simbolo.usos.len();
        return;
    }
    simbolo
        .usos
        .retain(|local| !simbolo.declaracoes.contains(local));
    if !declara_em_varios_arquivos(&simbolo) {
        fontes.simbolos.push(simbolo);
        return;
    }

    let raiz_rust = raiz_do_rust_analyzer(id);
    let mut por_arquivo = BTreeMap::new();
    for declaracao in simbolo.declaracoes {
        let grupo = por_arquivo
            .entry(declaracao.arquivo.clone())
            .or_insert_with(|| SimboloIndexado {
                nome: if raiz_rust {
                    "crate".into()
                } else {
                    simbolo.nome.clone()
                },
                ..Default::default()
            });
        grupo.declaracoes.insert(declaracao);
    }
    for uso in simbolo.usos {
        if raiz_rust && let Some(arquivo) = raizes.resolver(&uso, &por_arquivo, &fontes.fontes) {
            por_arquivo
                .get_mut(arquivo)
                .expect("raiz candidata")
                .usos
                .insert(uso);
        } else {
            fontes.usos_ambiguos += 1;
        }
    }
    fontes.simbolos.extend(por_arquivo.into_values());
}

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
    use protobuf::EnumOrUnknown;

    fn ocorrencia(id: &str, faixa: [i32; 3], definicao: bool) -> Occurrence {
        Occurrence {
            symbol: id.into(),
            range: faixa.into(),
            symbol_roles: if definicao {
                SymbolRole::Definition as i32
            } else {
                0
            },
            ..Default::default()
        }
    }

    fn carregar_fixture(nome: &str, fontes: Vec<(&str, &str, Vec<Occurrence>)>) -> FontesIndexadas {
        let raiz = std::env::temp_dir().join(format!("lente-scip-{nome}-{}", std::process::id()));
        fs::create_dir(&raiz).unwrap();
        let documents = fontes
            .into_iter()
            .map(|(arquivo, texto, occurrences)| {
                fs::write(raiz.join(arquivo), texto).unwrap();
                Document {
                    relative_path: arquivo.into(),
                    position_encoding: PositionEncoding::UTF8CodeUnitOffsetFromLineStart.into(),
                    occurrences,
                    ..Default::default()
                }
            })
            .collect();
        let caminho = raiz.join("teste.scip");
        ::scip::write_message_to_file(
            &caminho,
            Index {
                documents,
                ..Default::default()
            },
        )
        .unwrap();
        let resultado = carregar_indices(&raiz, &[caminho]);
        fs::remove_dir_all(raiz).unwrap();
        resultado.unwrap()
    }

    #[test]
    fn retira_usos_ambiguos_e_conserva_usos_com_um_arquivo_declarador() {
        let f = "scip-test cargo pacote . f().";
        let g = "scip-test cargo pacote . g().";
        let indice = carregar_fixture(
            "colisao",
            vec![
                (
                    "a.rs",
                    "f\ng\ng",
                    vec![
                        ocorrencia(f, [0, 0, 1], true),
                        ocorrencia(g, [1, 0, 1], true),
                        ocorrencia(g, [2, 0, 1], true),
                    ],
                ),
                ("b.rs", "f", vec![ocorrencia(f, [0, 0, 1], true)]),
                (
                    "uso.rs",
                    "f g",
                    vec![
                        ocorrencia(f, [0, 0, 1], false),
                        ocorrencia(g, [0, 2, 3], false),
                    ],
                ),
            ],
        );
        assert_eq!(indice.usos_ambiguos, 1);
        assert_eq!(indice.usos_sem_declaracao, 0);
        let pares = indice.pares();
        assert_eq!(
            pares.keys().copied().collect::<Vec<_>>(),
            [("uso.rs", "a.rs")]
        );
        assert_eq!(pares[&("uso.rs", "a.rs")].len(), 1);
        let declaracoes_f = indice
            .simbolos
            .iter()
            .filter(|s| s.nome == "f")
            .collect::<Vec<_>>();
        assert_eq!(declaracoes_f.len(), 2);
        assert!(
            declaracoes_f
                .iter()
                .all(|s| s.declaracoes.len() == 1 && s.usos.is_empty())
        );
    }

    #[test]
    fn crate_local_aponta_para_a_raiz_comprovada_pelos_modulos() {
        let raiz = "rust-analyzer cargo pacote 0.1.0 crate/";
        let regras = "rust-analyzer cargo pacote 0.1.0 regras/";
        let v5 = "rust-analyzer cargo pacote 0.1.0 regras/v5/";
        let lib = "/* á */ mod regras;";
        let coluna = i32::try_from(lib.find("regras").unwrap()).unwrap();
        for compartilhado in [false, true] {
            let mut principais = vec![ocorrencia(raiz, [0, 0, 0], true)];
            let main = if compartilhado {
                principais.push(ocorrencia(regras, [0, 4, 10], false));
                "mod regras;"
            } else {
                ""
            };
            let indice = carregar_fixture(
                if compartilhado {
                    "raizes-compartilhadas"
                } else {
                    "raiz-lib"
                },
                vec![
                    (
                        "lib.rs",
                        lib,
                        vec![
                            ocorrencia(raiz, [0, 0, 0], true),
                            ocorrencia(regras, [0, coluna, coluna + 6], false),
                        ],
                    ),
                    ("main.rs", main, principais),
                    (
                        "regras.rs",
                        "mod v5;",
                        vec![
                            ocorrencia(regras, [0, 0, 0], true),
                            ocorrencia(v5, [0, 4, 6], false),
                        ],
                    ),
                    (
                        "v5.rs",
                        "use crate::a;\nuse outro::b;",
                        vec![
                            ocorrencia(v5, [0, 0, 0], true),
                            ocorrencia(raiz, [0, 4, 9], false),
                            ocorrencia(raiz, [1, 4, 9], false),
                        ],
                    ),
                ],
            );
            assert_eq!(indice.usos_ambiguos, if compartilhado { 2 } else { 1 });
            let pares = indice.pares();
            assert!(!pares.contains_key(&("v5.rs", "main.rs")));
            assert_eq!(
                pares.get(&("v5.rs", "lib.rs")).map(Vec::len),
                if compartilhado { None } else { Some(1) }
            );
        }
    }

    fn posicao_no_documento(
        indexador: &str,
        codificacao: EnumOrUnknown<PositionEncoding>,
        coluna: i32,
    ) -> Result<Option<LocalFonte>, String> {
        let arquivo = "a.rs";
        let documento = Document {
            relative_path: arquivo.into(),
            position_encoding: codificacao,
            occurrences: vec![Occurrence {
                range: vec![0, coluna, coluna + 1],
                symbol: "local 0".into(),
                symbol_roles: SymbolRole::Definition as i32,
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut fontes = FontesIndexadas {
            fontes: BTreeMap::from([(arquivo.into(), "🚀éx".into())]),
            ..Default::default()
        };
        let mut simbolos = BTreeMap::new();
        carregar_documento(
            Path::new("."),
            indexador,
            &documento,
            &mut fontes,
            &mut simbolos,
            "índice de teste",
            &mut BTreeMap::new(),
        )?;
        Ok(simbolos
            .into_values()
            .next()
            .and_then(|simbolo| simbolo.declaracoes.into_iter().next()))
    }

    fn conferir_offset(indexador: &str, codificacao: PositionEncoding, coluna: i32) {
        let local = posicao_no_documento(indexador, codificacao.into(), coluna)
            .unwrap_or_else(|erro| panic!("{indexador} / {codificacao:?}: {erro}"))
            .unwrap_or_else(|| panic!("{indexador} / {codificacao:?}: posição descartada"));
        assert_eq!(
            (local.coluna, local.coluna_fim),
            (6, 7),
            "{indexador} / {codificacao:?}"
        );
    }

    #[test]
    fn usa_o_padrao_de_codificacao_de_cada_indexador() {
        use PositionEncoding::UnspecifiedPositionEncoding;
        for indexador in ["rust-analyzer", "scip-clang", "scip-go", "scip-zig"] {
            conferir_offset(indexador, UnspecifiedPositionEncoding, 6);
        }
        for indexador in [
            "scip-python",
            "scip-java",
            "scip-typescript",
            "scip-dotnet",
            "scip-dart",
        ] {
            conferir_offset(indexador, UnspecifiedPositionEncoding, 3);
        }
    }

    #[test]
    fn codificacao_explicita_prevalece_sobre_o_padrao_do_indexador() {
        use PositionEncoding::*;
        for (indexador, codificacao, coluna) in [
            ("scip-java", UTF8CodeUnitOffsetFromLineStart, 6),
            ("rust-analyzer", UTF16CodeUnitOffsetFromLineStart, 3),
            ("scip-go", UTF32CodeUnitOffsetFromLineStart, 2),
            ("sem-padrao", UTF16CodeUnitOffsetFromLineStart, 3),
        ] {
            conferir_offset(indexador, codificacao, coluna);
        }
    }

    #[test]
    fn recusa_codificacao_desconhecida_ou_ausente_sem_padrao() {
        use PositionEncoding::UnspecifiedPositionEncoding;
        for (indexador, codificacao) in [
            ("sem-padrao", UnspecifiedPositionEncoding.into()),
            ("rust-analyzer", EnumOrUnknown::from_i32(999)),
        ] {
            let erro = posicao_no_documento(indexador, codificacao, 6).unwrap_err();
            assert!(
                erro.contains("codificação da posição SCIP"),
                "{indexador}: {erro}"
            );
        }
    }

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
