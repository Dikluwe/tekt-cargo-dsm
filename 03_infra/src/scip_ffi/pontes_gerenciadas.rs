//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-pontes-gerenciadas.md
//! @prompt-hash 3e579170
//! @layer L3
//! @updated 2026-11-26

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use lente_core::domain::fontes_indexadas::{
    ChamadasFonte, FontesIndexadas, FuncaoFonte, LocalFonte, OrigemFfi, SimboloIndexado, VinculoFfi,
};
use serde::Deserialize;

const AST_CTYPES: &str = include_str!("ast_ctypes.py");
const AST_OUTRAS: &str = include_str!("ast_outras_ffi.py");
const AST_JNI: &str = include_str!("AstJni.java");

type Fontes<'a> = Vec<(&'a str, &'a str)>;
type Posicao = [usize; 4];

#[derive(Deserialize)]
struct Registro {
    local: Posicao,
    chamador: Option<(String, Posicao)>,
    ponte: Option<Ponte>,
    #[serde(default)]
    somente_declaracao: bool,
}

#[derive(Deserialize)]
struct Ponte {
    nome: String,
    linker: String,
    biblioteca: String,
    #[serde(default)]
    logica: bool,
    declaracao: Option<Posicao>,
    #[serde(default)]
    tipo: TipoPonte,
    arquivo_nativo: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TipoPonte {
    #[default]
    Dinamica,
    Pinvoke,
    Cgo,
    Zig,
    Node,
}

pub(super) const REGISTRO_NODE: &str = "napi_register_module_v1";

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Chave {
    linker: String,
    nome: String,
    declaracao: Option<LocalFonte>,
    tipo: TipoPonte,
    arquivo_nativo: Option<String>,
}

#[derive(Default)]
pub(super) struct Gerenciadas {
    pub chamadas: ChamadasFonte,
    pontes: BTreeMap<Chave, BTreeSet<LocalFonte>>,
}

impl Gerenciadas {
    pub fn nomes_dinamicos(&self) -> BTreeSet<&str> {
        let mut nomes = BTreeSet::new();
        for ponte in self.pontes.keys() {
            match ponte.tipo {
                TipoPonte::Dinamica | TipoPonte::Pinvoke => {
                    nomes.insert(ponte.linker.as_str());
                }
                TipoPonte::Node => {
                    nomes.insert(REGISTRO_NODE);
                }
                TipoPonte::Cgo | TipoPonte::Zig => {}
            }
        }
        nomes
    }

    pub fn nomes_cgo(&self) -> BTreeSet<&str> {
        self.nomes(TipoPonte::Cgo)
    }

    pub fn nomes_zig(&self) -> BTreeSet<&str> {
        self.nomes(TipoPonte::Zig)
    }

    pub fn nomes_node(&self) -> BTreeSet<&str> {
        self.nomes(TipoPonte::Node)
    }

    fn nomes(&self, tipo: TipoPonte) -> BTreeSet<&str> {
        self.pontes
            .keys()
            .filter(|ponte| ponte.tipo == tipo)
            .map(|ponte| ponte.linker.as_str())
            .collect()
    }

    pub fn aplicar(
        &self,
        indice: &mut FontesIndexadas,
        confirmados: &BTreeMap<String, LocalFonte>,
        exportados: &BTreeMap<&str, Option<u64>>,
        cgo: &BTreeMap<&str, Option<u64>>,
        node: &BTreeMap<&str, Option<u64>>,
        zig: &BTreeMap<&str, Option<u64>>,
    ) -> BTreeMap<usize, BTreeSet<LocalFonte>> {
        let mut chamadas_confirmadas = BTreeMap::new();
        let provado = |tabela: &BTreeMap<&str, Option<u64>>, nome: &str| {
            matches!(tabela.get(nome), Some(Some(_)))
        };
        for (chave, usos) in &self.pontes {
            let nome = chave.linker.as_str();
            let prova = match chave.tipo {
                TipoPonte::Dinamica | TipoPonte::Pinvoke => provado(exportados, nome),
                TipoPonte::Cgo => provado(cgo, nome),
                TipoPonte::Zig => provado(zig, nome),
                TipoPonte::Node => provado(exportados, REGISTRO_NODE) && provado(node, nome),
            };
            if !prova {
                continue;
            }
            let Some(implementacao) = confirmados.get(nome) else {
                continue;
            };
            if let Some(arquivo) = &chave.arquivo_nativo
                && *arquivo != implementacao.arquivo
            {
                continue;
            }
            let vinculo = VinculoFfi {
                origem: OrigemFfi::Elf,
                nome_linker: chave.linker.clone(),
                implementacao: implementacao.clone(),
                chamadas_fronteira: ChamadasFonte::new(),
            };
            if let Some(declaracao) = &chave.declaracao {
                let mut existentes = indice
                    .simbolos
                    .iter_mut()
                    .enumerate()
                    .filter(|(_, simbolo)| simbolo.declaracoes.contains(declaracao));
                if let Some((id, simbolo)) = existentes.next() {
                    if existentes.next().is_none() && simbolo.ffi.is_none() {
                        simbolo.usos.extend(usos.iter().cloned());
                        simbolo.ffi = Some(vinculo);
                        chamadas_confirmadas.insert(id, usos.clone());
                    }
                    continue;
                }
            }
            chamadas_confirmadas.insert(indice.simbolos.len(), usos.clone());
            indice.simbolos.push(SimboloIndexado {
                nome: chave.nome.clone(),
                declaracoes: chave.declaracao.iter().cloned().collect(),
                usos: usos.clone(),
                ffi: Some(vinculo),
            });
        }
        chamadas_confirmadas
    }
}

pub(super) fn coletar(
    raiz: &Path,
    solicitado: &Path,
    artefato: &Path,
    fontes: &BTreeMap<String, String>,
    dinamico: bool,
) -> Gerenciadas {
    let documentos = |extensao: &str| -> Fontes<'_> {
        fontes
            .iter()
            .filter(|(arquivo, _)| arquivo.ends_with(extensao))
            .map(|(arquivo, fonte)| (arquivo.as_str(), fonte.as_str()))
            .collect()
    };
    let python = if dinamico {
        documentos(".py")
    } else {
        Vec::new()
    };
    let java = if dinamico {
        documentos(".java")
    } else {
        Vec::new()
    };
    let addon = dinamico && solicitado.extension().is_some_and(|ext| ext == "node");
    let outras: Fontes<'_> = fontes
        .iter()
        .filter(
            |(arquivo, _)| match Path::new(arquivo).extension().and_then(|ext| ext.to_str()) {
                Some("go" | "zig") => true,
                Some(
                    "cs" | "kt" | "dart" | "js" | "jsx" | "cjs" | "mjs" | "ts" | "tsx" | "cts"
                    | "mts",
                ) => dinamico,
                Some("c" | "cc" | "cpp" | "cxx" | "cp" | "C") => addon,
                _ => false,
            },
        )
        .map(|(arquivo, fonte)| (arquivo.as_str(), fonte.as_str()))
        .collect();
    let mut extracao = Gerenciadas::default();
    for (fontes, registros) in [
        (&python, extrair_python(&python)),
        (&java, extrair_java(&java)),
        (&outras, extrair_outras(&outras)),
    ] {
        for registro in registros {
            let Some(local) = normalizar(fontes, registro.local) else {
                continue;
            };
            let chamador = registro.chamador.and_then(|(nome, posicao)| {
                Some(FuncaoFonte {
                    nome,
                    declaracao: normalizar(fontes, posicao)?,
                })
            });
            if !registro.somente_declaracao {
                extracao.chamadas.insert(local.clone(), chamador);
            }
            let Some(ponte) = registro.ponte else {
                continue;
            };
            if !mesma_biblioteca(raiz, solicitado, artefato, &ponte) {
                continue;
            }
            let declaracao = match ponte.declaracao {
                Some(posicao) => {
                    let Some(declaracao) = normalizar(fontes, posicao) else {
                        continue;
                    };
                    Some(declaracao)
                }
                None => None,
            };
            let usos = extracao
                .pontes
                .entry(Chave {
                    linker: ponte.linker,
                    nome: ponte.nome,
                    declaracao,
                    tipo: ponte.tipo,
                    arquivo_nativo: ponte.arquivo_nativo,
                })
                .or_default();
            if !registro.somente_declaracao {
                usos.insert(local);
            }
        }
    }
    extracao
}

fn normalizar(fontes: &Fontes<'_>, [indice, linha, coluna, fim]: Posicao) -> Option<LocalFonte> {
    let (arquivo, texto) = fontes.get(indice)?;
    let linha_texto = texto.lines().nth(linha.checked_sub(1)?)?;
    if coluna >= fim || linha_texto.get(coluna..fim).is_none() {
        return None;
    }
    Some(LocalFonte {
        arquivo: (*arquivo).into(),
        linha,
        coluna,
        linha_fim: linha,
        coluna_fim: fim,
    })
}

fn mesma_biblioteca(raiz: &Path, solicitado: &Path, artefato: &Path, ponte: &Ponte) -> bool {
    if matches!(ponte.tipo, TipoPonte::Cgo | TipoPonte::Zig) {
        return true;
    }
    let alias_biblioteca = |nome: &str| {
        let caminho = Path::new(nome);
        caminho.file_name() == Some(caminho.as_os_str())
            && (solicitado.file_name().and_then(|s| s.to_str()) == Some(nome)
                || fs::canonicalize(raiz.join(caminho))
                    .is_ok_and(|resolvido| resolvido == artefato))
    };
    if ponte.tipo == TipoPonte::Pinvoke && !ponte.biblioteca.contains('/') {
        let nome = &ponte.biblioteca;
        let candidatos = [
            nome.clone(),
            format!("{nome}.so"),
            format!("lib{nome}"),
            format!("lib{nome}.so"),
        ];
        return candidatos.iter().any(|nome| alias_biblioteca(nome))
            && !candidatos.iter().any(|nome| {
                fs::canonicalize(raiz.join(nome)).is_ok_and(|resolvido| resolvido != artefato)
            });
    }
    if ponte.logica {
        return alias_biblioteca(&format!("lib{}.so", ponte.biblioteca));
    }
    let caminho = Path::new(&ponte.biblioteca);
    if !caminho.is_absolute() && !ponte.biblioteca.contains('/') {
        return false;
    }
    fs::canonicalize(raiz.join(caminho)).is_ok_and(|resolvido| resolvido == artefato)
}

fn extrair_outras(fontes: &Fontes<'_>) -> Vec<Registro> {
    if fontes.is_empty() {
        return Vec::new();
    }
    let Ok(entrada) = serde_json::to_vec(fontes) else {
        return Vec::new();
    };
    executar(
        Command::new("python3").args(["-I", "-S", "-X", "utf8", "-c", AST_OUTRAS]),
        &entrada,
    )
}

fn extrair_python(fontes: &Fontes<'_>) -> Vec<Registro> {
    if fontes.is_empty() {
        return Vec::new();
    }
    let textos: Vec<_> = fontes.iter().map(|(_, texto)| *texto).collect();
    let Ok(entrada) = serde_json::to_vec(&textos) else {
        return Vec::new();
    };
    executar(
        Command::new("python3").args(["-I", "-S", "-X", "utf8", "-c", AST_CTYPES]),
        &entrada,
    )
}

fn extrair_java(fontes: &Fontes<'_>) -> Vec<Registro> {
    if fontes.is_empty() {
        return Vec::new();
    }
    let Ok(total) = u32::try_from(fontes.len()) else {
        return Vec::new();
    };
    let mut entrada = total.to_be_bytes().to_vec();
    for (arquivo, texto) in fontes {
        for dado in [arquivo, texto] {
            let Ok(tamanho) = u32::try_from(dado.len()) else {
                return Vec::new();
            };
            entrada.extend(tamanho.to_be_bytes());
            entrada.extend(dado.as_bytes());
        }
    }
    let Ok(agora) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return Vec::new();
    };
    let caminho = std::env::temp_dir().join(format!(
        "lente-jni-{}-{}.java",
        std::process::id(),
        agora.as_nanos()
    ));
    let Ok(mut auxiliar) = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&caminho)
    else {
        return Vec::new();
    };
    let escrita = auxiliar.write_all(AST_JNI.as_bytes());
    drop(auxiliar);
    let mut registros = Vec::new();
    if escrita.is_ok() {
        registros = executar(
            Command::new("java")
                .env_remove("JAVA_TOOL_OPTIONS")
                .env_remove("JDK_JAVA_OPTIONS")
                .env_remove("_JAVA_OPTIONS")
                .env_remove("CLASSPATH")
                .args(["--source", "17"])
                .arg(&caminho),
            &entrada,
        );
    }
    let _ = fs::remove_file(caminho);
    registros
}

fn executar(comando: &mut Command, entrada: &[u8]) -> Vec<Registro> {
    let Ok(mut processo) = comando
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Vec::new();
    };
    if processo.stdin.take().unwrap().write_all(entrada).is_err() {
        let _ = processo.kill();
    }
    let Ok(saida) = processo.wait_with_output() else {
        return Vec::new();
    };
    if !saida.status.success() {
        return Vec::new();
    }
    serde_json::from_slice(&saida.stdout).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localiza_entradas_multilingues_no_identificador_exato() {
        let fontes = vec![
            (
                "ponte.go",
                "package main\nimport \"C\"\nfunc origem() { C.nativa() }",
            ),
            (
                "P.cs",
                r#"using System.Runtime.InteropServices;
                class P { [DllImport("ponte", EntryPoint = "csharp")] static extern int F(); }"#,
            ),
            (
                "P.kt",
                r#"package a_b
                object P { init { System.loadLibrary("ponte") } external fun f(): Int }"#,
            ),
            (
                "ponte.ts",
                r#"const lib = Deno.dlopen("./ponte.so",
                { f: { parameters: [], result: "i32", name: "deno" } });
                function origem(): number { /* ação 🚀 */ return lib.symbols.f(); }
                lib.close();"#,
            ),
            (
                "ponte.dart",
                r#"import 'dart:ffi' as ffi;
                final lib = ffi.DynamicLibrary.open('./ponte.so');
                final f = lib.lookupFunction<ffi.Int32 Function(ffi.Int32), int Function(int)>('dart');
                int origem(int v) { return f(v); }"#,
            ),
            (
                "web/ponte.cjs",
                r#"const lib = require("../ponte.node");
                function origem() { return lib.f(); }"#,
            ),
            (
                "ponte.c",
                r#"#include <node_api.h>
                NAPI_MODULE_INIT() {
                    napi_value fn;
                    napi_create_function(env, "f", NAPI_AUTO_LENGTH, node, NULL, &fn);
                    napi_set_named_property(env, exports, "f", fn);
                    return exports;
                }"#,
            ),
        ];
        let pontes: Vec<_> = extrair_outras(&fontes)
            .into_iter()
            .filter_map(|r| {
                let ponte = r.ponte?;
                let local = normalizar(&fontes, r.local).unwrap();
                let texto = fontes[r.local[0]].1.lines().nth(local.linha - 1).unwrap();
                assert_eq!(&texto[local.coluna..local.coluna_fim], ponte.nome);
                assert_eq!(r.somente_declaracao, ponte.declaracao.is_some());
                if !r.somente_declaracao {
                    assert_eq!(r.chamador.unwrap().0, "origem");
                }
                if ponte.tipo == TipoPonte::Node {
                    assert_eq!(ponte.biblioteca, "web/../ponte.node");
                    assert_eq!(ponte.arquivo_nativo.as_deref(), Some("ponte.c"));
                }
                Some(ponte.linker)
            })
            .collect();
        assert_eq!(
            pontes,
            ["nativa", "csharp", "Java_a_1b_P_f", "deno", "dart", "node"]
        );
    }

    #[test]
    fn zig_marca_declaracao_e_chamadas_de_abi_c() {
        let fonte = r#"const c = @cImport({ @cInclude("calc.h"); @cInclude("tipos.h"); });
const Int = c.Int;
extern fn via_extern(valor: Int) callconv(.c) Int;
fn origem(valor: Int) !Int {
    _ = via_extern(valor);
    return c.via_header(valor);
}"#;
        let fontes = vec![("ffi.zig", fonte)];
        let extraidas: Vec<_> = extrair_outras(&fontes)
            .into_iter()
            .filter_map(|r| {
                let ponte = r.ponte?;
                assert_eq!(ponte.tipo, TipoPonte::Zig);
                let local = normalizar(&fontes, r.local).unwrap();
                assert_eq!(
                    &fonte.lines().nth(local.linha - 1).unwrap()[local.coluna..local.coluna_fim],
                    ponte.linker
                );
                Some((
                    local.linha,
                    ponte.linker,
                    ponte.declaracao.map(|loc| loc[1]),
                    r.chamador.map(|(nome, _)| nome),
                ))
            })
            .collect();
        assert_eq!(
            extraidas,
            [
                (3, "via_extern".into(), Some(3), None),
                (5, "via_extern".into(), Some(3), Some("origem".into())),
                (6, "via_header".into(), None, Some("origem".into())),
            ]
        );
    }

    #[test]
    fn zig_rejeita_sombra_e_recortes_sem_abi_c_identificavel() {
        let fontes = vec![
            (
                "sombra.zig",
                r#"const c = @cImport({ @cInclude("calc.h"); });
fn origem(c: OutraStruct) c_int { return c.via_header(2); }"#,
            ),
            (
                "extern.zig",
                r#"extern "fora" fn dinamica() c_int;
extern fn fora() callconv(.naked) c_int;
fn origem() c_int { return dinamica() + fora(); }"#,
            ),
            (
                "define.zig",
                r#"const c = @cImport({ @cInclude(nome_calculado); });
fn origem() c_int { return c.via_header(2); }"#,
            ),
            (
                "texto.zig",
                r#"const c = @cImport({ @cInclude("calc.h"); });
const falso =
    \\ fn origem() c_int { return c.via_header(2); }
;
"#,
            ),
        ];
        assert!(extrair_outras(&fontes).iter().all(|r| r.ponte.is_none()));
    }

    #[test]
    fn rejeita_formas_dinamicas_ou_ambiguas_sem_produzir_fronteira() {
        let fontes = vec![
            (
                "sombras.go",
                r#"package p
                import "C"
                func a(C T) { C.nativa() }"#,
            ),
            (
                "P.cs",
                r#"using System.Runtime.InteropServices;
                class P { [DllImport("ponte", EntryPoint = Nome)] static extern int F(); }"#,
            ),
            (
                "P.kt",
                r#"object P {
                init { System.loadLibrary("ponte") }
                external fun f(): Int
                external fun f(i: Int): Int
            }"#,
            ),
            (
                "define.ts",
                r#"const lib = Deno.dlopen("./ponte.so",
                { f: { parameters: [], result: "i32", name: nome } }); lib.symbols.f();"#,
            ),
            (
                "comentario.js",
                r#"// const lib = Deno.dlopen("./ponte.so", { f: {} });
                const exemplo = /const lib = Deno.dlopen[(]/;
                lib.symbols.f();"#,
            ),
            (
                "sombra.js",
                r#"const lib = Deno.dlopen("./ponte.so",
                { f: { parameters: [], result: "i32" } });
                function origem(lib) { return lib.symbols.f(); }"#,
            ),
            (
                "P.dart",
                r#"import 'dart:ffi';
                final lib = DynamicLibrary.open('./ponte.so');
                final f = lib.lookupFunction<Native, Dart>(nome); f();"#,
            ),
            (
                "registro.c",
                r#"#include <node_api.h>
                NAPI_MODULE_INIT() {
                    napi_value fn;
                    napi_create_function(env, "f", NAPI_AUTO_LENGTH, node, NULL, &fn);
                    napi_set_named_property(env, exports, "f", fn);
                    return outro_objeto;
                }"#,
            ),
            ("P.cjs", r#"const lib = require("./ponte.node"); lib.f();"#),
        ];
        assert!(extrair_outras(&fontes).iter().all(|r| r.ponte.is_none()));
    }

    #[test]
    fn biblioteca_logica_jni_exige_o_mesmo_nome_do_elf() {
        let arquivo = Path::new("/p/libreal.so");
        let mut ponte = Ponte {
            nome: "chamar".into(),
            linker: "Java_App_chamar".into(),
            biblioteca: "outra".into(),
            logica: true,
            declaracao: None,
            tipo: TipoPonte::Dinamica,
            arquivo_nativo: None,
        };
        assert!(!mesma_biblioteca(Path::new("/p"), arquivo, arquivo, &ponte));
        ponte.biblioteca = "real".into();
        assert!(mesma_biblioteca(Path::new("/p"), arquivo, arquivo, &ponte));
        ponte.logica = false;
        ponte.biblioteca = "libreal.so".into();
        assert!(!mesma_biblioteca(Path::new("/p"), arquivo, arquivo, &ponte));
    }

    #[test]
    fn reutiliza_declaracao_jni_e_exige_exportacao_dinamica() {
        let fonte = vec![("App.java", "native void f();\nf();")];
        let declaracao = normalizar(&fonte, [0, 1, 12, 13]).unwrap();
        let uso = normalizar(&fonte, [0, 2, 0, 1]).unwrap();
        let mut indice = FontesIndexadas {
            simbolos: vec![SimboloIndexado {
                nome: "f".into(),
                declaracoes: BTreeSet::from([declaracao.clone()]),
                ..Default::default()
            }],
            ..Default::default()
        };
        let dados = Gerenciadas {
            pontes: BTreeMap::from([(
                Chave {
                    linker: "Java_App_f".into(),
                    nome: "f".into(),
                    declaracao: Some(declaracao),
                    tipo: TipoPonte::Dinamica,
                    arquivo_nativo: None,
                },
                BTreeSet::from([uso.clone()]),
            )]),
            ..Default::default()
        };
        let confirmados = BTreeMap::from([(
            "Java_App_f".into(),
            LocalFonte {
                arquivo: "app.c".into(),
                ..uso.clone()
            },
        )]);
        dados.aplicar(
            &mut indice,
            &confirmados,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        assert!(indice.simbolos[0].ffi.is_none());
        dados.aplicar(
            &mut indice,
            &confirmados,
            &BTreeMap::from([("Java_App_f", Some(1))]),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        assert_eq!(indice.simbolos.len(), 1);
        assert_eq!(indice.simbolos[0].usos, BTreeSet::from([uso]));
        assert!(indice.simbolos[0].ffi.is_some());
    }
}
