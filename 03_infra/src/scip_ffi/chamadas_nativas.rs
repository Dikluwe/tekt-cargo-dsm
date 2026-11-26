//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-chamadas-nativas.md
//! @prompt-hash 9f1843d6
//! @layer L3
//! @updated 2026-11-26

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use lente_core::domain::fontes_indexadas::{ChamadasFonte, FuncaoFonte, LocalFonte};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Compilacao {
    pub(crate) directory: PathBuf,
    pub(crate) file: PathBuf,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    command: String,
}

#[derive(Default)]
pub(super) struct ChamadasNativas {
    pub locais: ChamadasFonte,
    pub por_implementacao: BTreeMap<LocalFonte, ChamadasFonte>,
    pub definicoes: BTreeMap<LocalFonte, String>,
}

pub(super) fn coletar(
    raiz: &Path,
    fontes: &BTreeMap<String, String>,
    destinos: &BTreeMap<String, LocalFonte>,
) -> ChamadasNativas {
    let mut chamadas = ChamadasNativas::default();
    if destinos.is_empty() {
        return chamadas;
    }
    let banco = ["compile_commands.json", "build/compile_commands.json"]
        .into_iter()
        .map(|relativo| raiz.join(relativo))
        .find(|caminho| caminho.is_file());
    let Some(banco) = banco else { return chamadas };
    let Ok(bytes) = fs::read(&banco) else {
        return chamadas;
    };
    let Ok(entradas) = serde_json::from_slice::<Vec<Compilacao>>(&bytes) else {
        return chamadas;
    };
    let mut unidades = BTreeMap::<_, BTreeSet<_>>::new();
    for entrada in entradas {
        let Ok(diretorio) = fs::canonicalize(banco.parent().unwrap().join(&entrada.directory))
        else {
            continue;
        };
        let Ok(caminho) = fs::canonicalize(diretorio.join(&entrada.file)) else {
            continue;
        };
        let Some(arquivo) = caminho.strip_prefix(raiz).ok().and_then(Path::to_str) else {
            continue;
        };
        let extensao = caminho
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        if !matches!(
            extensao.to_ascii_lowercase().as_str(),
            "c" | "cc" | "cp" | "cpp" | "cxx"
        ) || !fontes.contains_key(arquivo)
        {
            continue;
        }
        let opcoes = flags(&entrada, &diretorio, &caminho).map(|flags| (diretorio, flags));
        unidades.entry(caminho).or_default().insert(opcoes);
    }
    for (caminho, configuracoes) in unidades {
        let Some(Some((diretorio, flags))) = configuracoes.first() else {
            continue;
        };
        if configuracoes.len() != 1 {
            continue;
        }
        let Ok(mut clang) = Command::new("clang")
            .current_dir(diretorio)
            .env_remove("CCC_OVERRIDE_OPTIONS")
            .args(flags)
            .args([
                "--no-default-config",
                "-fsyntax-only",
                "-Xclang",
                "-ast-dump=json",
                "--",
            ])
            .arg(&caminho)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        let saida = BufReader::new(clang.stdout.take().unwrap());
        let Ok(ast) = serde_json::from_reader::<_, NoAst>(saida) else {
            let _ = clang.kill();
            let _ = clang.wait();
            continue;
        };
        if !clang.wait().is_ok_and(|status| status.success()) {
            continue;
        }
        let arquivo = caminho.strip_prefix(raiz).unwrap().to_str().unwrap();
        let texto = &fontes[arquivo];
        Extrator::novo(&caminho, arquivo, texto, destinos).extrair(&ast, &mut chamadas);
    }
    chamadas
}

pub(crate) fn flags(entrada: &Compilacao, diretorio: &Path, arquivo: &Path) -> Option<Vec<String>> {
    let argumentos;
    let origem = if entrada.arguments.is_empty() {
        argumentos = shlex::split(&entrada.command)?;
        &argumentos
    } else {
        &entrada.arguments
    };
    let mut itens = origem.iter();
    itens.next()?;
    let mut flags = Vec::new();
    while let Some(arg) = itens.next() {
        let aviso = arg.starts_with("-W")
            && !["-Wp,", "-Wl,", "-Wa,"]
                .iter()
                .any(|prefixo| arg.starts_with(prefixo));
        let fonte = !arg.starts_with(['-', '@'])
            && fs::canonicalize(diretorio.join(arg)).ok().as_deref() == Some(arquivo);
        match arg.as_str() {
            "-c" | "-MD" | "-MMD" | "-MP" => continue,
            "-o" | "-MF" | "-MT" | "-MQ" => {
                itens.next()?;
                continue;
            }
            "-I" | "-D" | "-U" | "-x" | "-isystem" | "-iquote" | "-include" | "-isysroot"
            | "-target" | "--target" => {
                let valor = itens.next()?.as_str();
                if valor.starts_with(['@', '-']) {
                    return None;
                }
                flags.extend([arg.clone(), valor.into()]);
                continue;
            }
            "-pthread"
            | "-fPIC"
            | "-fpic"
            | "-fPIE"
            | "-fpie"
            | "-fno-exceptions"
            | "-fexceptions"
            | "-fno-rtti"
            | "-frtti"
            | "-m32"
            | "-m64"
            | "-ansi"
            | "-ffunction-sections"
            | "-fdata-sections"
            | "-g"
            | "-g0"
            | "-g1"
            | "-g2"
            | "-g3"
            | "-O0"
            | "-O1"
            | "-O2"
            | "-O3"
            | "-Og"
            | "-Os"
            | "-Oz" => {}
            _ if [
                "-I",
                "-D",
                "-U",
                "-std=",
                "--sysroot=",
                "--target=",
                "-fvisibility=",
            ]
            .iter()
            .any(|prefixo| arg.starts_with(prefixo)) => {}
            _ if aviso => {}
            _ if fonte => continue,
            _ => return None,
        }
        flags.push(arg.clone());
    }
    Some(flags)
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct NoAst {
    kind: String,
    id: String,
    name: Option<String>,
    mangled_name: Option<String>,
    storage_class: Option<String>,
    previous_decl: Option<String>,
    loc: Option<Posicao>,
    range: Option<Faixa>,
    cast_kind: Option<String>,
    referenced_decl: Option<Referencia>,
    inner: Vec<Self>,
}

#[derive(Deserialize)]
struct Referencia {
    kind: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct Faixa {
    end: Posicao,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Posicao {
    offset: Option<usize>,
    tok_len: Option<usize>,
    file: Option<PathBuf>,
    included_from: Option<serde::de::IgnoredAny>,
}

struct Extrator<'a> {
    caminho: &'a Path,
    arquivo: &'a str,
    fonte: &'a str,
    destinos: &'a BTreeMap<String, LocalFonte>,
    linhas: Vec<usize>,
}

impl<'a> Extrator<'a> {
    fn novo(
        caminho: &'a Path,
        arquivo: &'a str,
        fonte: &'a str,
        destinos: &'a BTreeMap<String, LocalFonte>,
    ) -> Self {
        Self {
            caminho,
            arquivo,
            fonte,
            destinos,
            linhas: std::iter::once(0)
                .chain(fonte.match_indices('\n').map(|(offset, _)| offset + 1))
                .collect(),
        }
    }

    fn local(&self, posicao: &Posicao, nome: &str) -> Option<LocalFonte> {
        let offset = posicao.offset?;
        let fim = offset.checked_add(posicao.tok_len?)?;
        if posicao.included_from.is_some()
            || posicao.file.as_deref().is_some_and(|p| p != self.caminho)
            || self.fonte.get(offset..fim)? != nome
        {
            return None;
        }
        let linha = self.linhas.partition_point(|&inicio| inicio <= offset);
        let inicio = self.linhas[linha - 1];
        Some(LocalFonte {
            arquivo: self.arquivo.into(),
            linha,
            coluna: offset - inicio,
            linha_fim: linha,
            coluna_fim: fim - inicio,
        })
    }

    fn funcao(&self, no: &NoAst) -> Option<FuncaoFonte> {
        let nome = no.name.as_ref()?;
        let declaracao = self.local(no.loc.as_ref()?, nome)?;
        Some(FuncaoFonte {
            nome: nome.clone(),
            declaracao,
        })
    }

    fn extrair(&self, ast: &NoAst, chamadas: &mut ChamadasNativas) {
        let mut declaracoes = BTreeMap::new();
        indexar_declaracoes(ast, &mut declaracoes);
        self.visitar(ast, None, &declaracoes, chamadas);
    }

    fn visitar(
        &self,
        no: &NoAst,
        chamador: Option<&FuncaoFonte>,
        declaracoes: &BTreeMap<&str, &NoAst>,
        chamadas: &mut ChamadasNativas,
    ) {
        match no.kind.as_str() {
            "FunctionDecl" | "CXXMethodDecl" | "CXXConstructorDecl" | "CXXDestructorDecl" => {
                if let Some(funcao) = self.funcao(no) {
                    for corpo in &no.inner {
                        if matches!(corpo.kind.as_str(), "CompoundStmt" | "CXXTryStmt") {
                            chamadas
                                .definicoes
                                .insert(funcao.declaracao.clone(), funcao.nome.clone());
                            self.visitar(corpo, Some(&funcao), declaracoes, chamadas);
                        }
                    }
                }
                return;
            }
            "LambdaExpr" | "FunctionTemplateDecl" | "CXXDefaultArgExpr" | "CXXDefaultInitExpr" => {
                return;
            }
            "CallExpr" => {
                if let Some(funcao) = chamador
                    && let Some((posicao, destino)) = callee(no)
                    && let Some(local) = self.local(posicao, &destino.name)
                {
                    chamadas.locais.insert(local.clone(), Some(funcao.clone()));
                    if let Some(implementacao) = nome_linker_externo(&destino.id, declaracoes)
                        .and_then(|nome| self.destinos.get(nome))
                    {
                        chamadas
                            .por_implementacao
                            .entry(implementacao.clone())
                            .or_default()
                            .insert(local, Some(funcao.clone()));
                    }
                }
            }
            _ => {}
        }
        for filho in &no.inner {
            self.visitar(filho, chamador, declaracoes, chamadas);
        }
    }
}

fn indexar_declaracoes<'a>(no: &'a NoAst, declaracoes: &mut BTreeMap<&'a str, &'a NoAst>) {
    if no.kind == "FunctionDecl" && !no.id.is_empty() {
        declaracoes.insert(&no.id, no);
    }
    for filho in &no.inner {
        indexar_declaracoes(filho, declaracoes);
    }
}

fn nome_linker_externo<'a>(id: &str, declaracoes: &BTreeMap<&str, &'a NoAst>) -> Option<&'a str> {
    let mut declaracao = *declaracoes.get(id)?;
    let nome = declaracao.mangled_name.as_deref()?;
    while declaracao.storage_class.as_deref() != Some("static") {
        let Some(anterior) = &declaracao.previous_decl else {
            return Some(nome);
        };
        declaracao = declaracoes.get(anterior.as_str())?;
    }
    None
}

fn callee(chamada: &NoAst) -> Option<(&Posicao, &Referencia)> {
    let mut destino = chamada.inner.first()?;
    while destino.kind == "ParenExpr"
        || destino.kind == "ImplicitCastExpr"
            && destino.cast_kind.as_deref() == Some("FunctionToPointerDecay")
    {
        destino = destino.inner.first()?;
    }
    let referencia = destino.referenced_decl.as_ref()?;
    if destino.kind != "DeclRefExpr"
        || !matches!(referencia.kind.as_str(), "FunctionDecl" | "CXXMethodDecl")
    {
        return None;
    }
    Some((&destino.range.as_ref()?.end, referencia))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn qualificada_usa_o_token_final_e_chamada_indireta_nao_entra() {
        let fonte = "int f() { return ::g(); }\n";
        let referencia = |kind: &str| {
            json!({
                "kind": "DeclRefExpr", "range": {"end": {"offset": 19, "tokLen": 1}},
                "referencedDecl": {"kind": kind, "name": "g"}
            })
        };
        let ast: NoAst = serde_json::from_value(json!({
            "kind": "FunctionDecl", "name": "f", "loc": {"offset": 4, "tokLen": 1},
            "inner": [{"kind": "CompoundStmt", "inner": [
                {"kind": "CallExpr", "inner": [referencia("FunctionDecl")]},
                {"kind": "CallExpr", "inner": [referencia("VarDecl")]},
                {"kind": "DeclRefExpr", "referencedDecl": {"kind": "FunctionDecl", "name": "g"}}
            ]}]
        }))
        .unwrap();
        let mut chamadas = ChamadasNativas::default();
        Extrator::novo(Path::new("/raiz/a.cpp"), "a.cpp", fonte, &BTreeMap::new())
            .extrair(&ast, &mut chamadas);
        let [(local, Some(chamador))] = chamadas.locais.iter().collect::<Vec<_>>()[..] else {
            panic!("esperava uma chamada direta");
        };
        assert_eq!(local.coluna, 19);
        assert_eq!(chamador.declaracao.coluna, 4);
    }

    #[test]
    fn nao_repassa_plugins_arquivos_de_resposta_ou_encadeamento_shell() {
        let vetores = [
            "/usr/bin/clang -Xclang -load -Xclang plugin.so",
            "/usr/bin/clang @flags",
            "/usr/bin/clang -std=c17 ; outro",
        ];
        for command in vetores {
            let entrada = Compilacao {
                directory: "/raiz".into(),
                file: "/raiz/a.c".into(),
                arguments: Vec::new(),
                command: command.into(),
            };
            assert!(flags(&entrada, Path::new("/raiz"), Path::new("/raiz/a.c")).is_none());
        }
    }

    #[test]
    fn nome_do_linker_respeita_a_ligacao_interna_na_declaracao_anterior() {
        let ast: NoAst = serde_json::from_value(json!({
            "inner": [
                {"kind": "FunctionDecl", "id": "a", "mangledName": "alvo", "storageClass": "static"},
                {"kind": "FunctionDecl", "id": "b", "mangledName": "alvo", "previousDecl": "a"},
                {"kind": "FunctionDecl", "id": "c", "name": "alias", "mangledName": "outro"}
            ]
        })).unwrap();
        let mut declaracoes = BTreeMap::new();
        indexar_declaracoes(&ast, &mut declaracoes);
        assert_eq!(nome_linker_externo("b", &declaracoes), None);
        assert_eq!(nome_linker_externo("c", &declaracoes), Some("outro"));
    }
}
