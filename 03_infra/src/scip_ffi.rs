//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-scip-ffi.md
//! @prompt-hash 482b0e31
//! @layer L3
//! @updated 2026-11-26

mod chamadas_nativas;

pub(crate) use chamadas_nativas::{Compilacao, flags as flags_compilacao};
mod pontes_gerenciadas;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::process::Command;

use lente_core::domain::fontes_indexadas::{
    ChamadasFonte, FontesIndexadas, FuncaoFonte, LocalFonte, OrigemFfi, VinculoFfi,
};
use syn::{Expr, ForeignItem, Lit, Meta, ext::IdentExt, visit::Visit};

pub fn ligar_ffi(raiz: &Path, artefato: &Path, indice: &mut FontesIndexadas) -> Result<(), String> {
    let caminho = fs::canonicalize(artefato).map_err(|e| format!("{}: {e}", artefato.display()))?;
    let raiz = fs::canonicalize(raiz).map_err(|e| format!("{}: {e}", raiz.display()))?;
    let mut cabecalho = [0; 20];
    File::open(&caminho)
        .and_then(|mut arquivo| arquivo.read_exact(&mut cabecalho))
        .map_err(|e| format!("{}: {e}", artefato.display()))?;
    if !elf_final(&cabecalho) {
        return Err(format!("{} não é um ELF final", artefato.display()));
    }
    let tabela = executar(
        "nm",
        Command::new("nm")
            .args(["--defined-only", "--format=posix", "--no-demangle", "--"])
            .arg(&caminho),
    )?;
    let mut sintaxe = SintaxeRust::default();
    for (arquivo, texto) in &indice.fontes {
        if arquivo.ends_with(".rs") {
            sintaxe.inspecionar(arquivo, texto);
        }
    }
    let gerenciadas = pontes_gerenciadas::coletar(
        &raiz,
        artefato,
        &caminho,
        &indice.fontes,
        tipo_elf(&cabecalho) == Some(3),
    );
    let nomes_dinamicos = gerenciadas.nomes_dinamicos();
    let cgo = ler_cgo(&tabela, &gerenciadas.nomes_cgo());
    let zig = ler_tabela(&tabela, &gerenciadas.nomes_zig());
    let node = ler_tabela_codigo(&tabela, &gerenciadas.nomes_node(), true);
    let exportados = if nomes_dinamicos.is_empty() {
        String::new()
    } else {
        executar(
            "nm",
            Command::new("nm")
                .args([
                    "-D",
                    "--defined-only",
                    "--extern-only",
                    "--format=posix",
                    "--no-demangle",
                    "--",
                ])
                .arg(&caminho),
        )?
    };
    let exportados = ler_tabela(&exportados, &nomes_dinamicos);
    let nomes: BTreeSet<_> = sintaxe.externos.values().map(String::as_str).collect();
    let mut enderecos = ler_tabela(&tabela, &nomes);
    for (&nome, &endereco) in exportados.iter().chain(&cgo).chain(&node).chain(&zig) {
        if let Some(endereco) = endereco {
            enderecos
                .entry(nome)
                .and_modify(|outro| {
                    if *outro != Some(endereco) {
                        *outro = None;
                    }
                })
                .or_insert(Some(endereco));
        }
    }
    let nomes_ok: Vec<_> = enderecos
        .iter()
        .filter_map(|(nome, endereco)| endereco.map(|_| format!("--name={nome}")))
        .collect();
    let mut depuracao = String::new();
    for lote in nomes_ok.chunks(256) {
        let parte = executar(
            "llvm-dwarfdump",
            Command::new("llvm-dwarfdump").args(lote).arg(&caminho),
        )?;
        depuracao.push_str(&parte);
        depuracao.push('\n');
    }
    let locais = definicoes_dwarf(&depuracao, &enderecos);
    let confirmados = confirmar_implementacoes(&raiz, &indice.fontes, locais);
    aplicar_vinculos_rust(indice, &sintaxe, &confirmados);
    let chamadas_confirmadas =
        gerenciadas.aplicar(indice, &confirmados, &exportados, &cgo, &node, &zig);
    sintaxe.chamadas.extend(gerenciadas.chamadas);
    let destinos: BTreeMap<_, _> = indice
        .simbolos
        .iter()
        .filter_map(|simbolo| simbolo.ffi.as_ref())
        .map(|ffi| (ffi.nome_linker.clone(), ffi.implementacao.clone()))
        .collect();
    let chamadas_nativas = chamadas_nativas::coletar(&raiz, &indice.fontes, &destinos);
    sintaxe.chamadas.extend(chamadas_nativas.locais);
    indice.indexar_chamadores_com_fronteiras(
        &sintaxe.chamadas,
        &chamadas_nativas.por_implementacao,
        &chamadas_nativas.definicoes,
        &chamadas_confirmadas,
    );
    indice.artefato_ffi = artefato
        .file_name()
        .map(|nome| nome.to_string_lossy().into_owned());
    Ok(())
}

fn executar(ferramenta: &str, comando: &mut Command) -> Result<String, String> {
    let saida = comando
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| format!("não foi possível executar {ferramenta}: {e}"))?;
    if !saida.status.success() {
        return Err(format!(
            "{ferramenta}: {}",
            String::from_utf8_lossy(&saida.stderr).trim()
        ));
    }
    String::from_utf8(saida.stdout).map_err(|e| e.to_string())
}

fn elf_final(bytes: &[u8; 20]) -> bool {
    matches!(tipo_elf(bytes), Some(2 | 3))
}

fn tipo_elf(bytes: &[u8; 20]) -> Option<u16> {
    if &bytes[..4] != b"\x7fELF" || !matches!(bytes[4], 1 | 2) {
        return None;
    }
    let ordem = [bytes[16], bytes[17]];
    match bytes[5] {
        1 => Some(u16::from_le_bytes(ordem)),
        2 => Some(u16::from_be_bytes(ordem)),
        _ => None,
    }
}

#[derive(Default)]
struct SintaxeRust {
    externos: BTreeMap<LocalFonte, String>,
    chamadas: ChamadasFonte,
}

impl SintaxeRust {
    fn inspecionar(&mut self, arquivo: &str, texto: &str) {
        if let Ok(ast) = syn::parse_file(texto) {
            let linhas = std::iter::once(0)
                .chain(texto.match_indices('\n').map(|(byte, _)| byte + 1))
                .collect();
            VisitanteRust {
                arquivo,
                linhas,
                sintaxe: self,
                chamador: None,
            }
            .visit_file(&ast);
        }
    }
}

type DefinicoesDwarf<'a> = BTreeMap<&'a str, BTreeSet<(&'a str, &'a str, usize)>>;

fn confirmar_implementacoes(
    raiz: &Path,
    fontes: &BTreeMap<String, String>,
    locais: DefinicoesDwarf<'_>,
) -> BTreeMap<String, LocalFonte> {
    let mut confirmados = BTreeMap::new();
    for (nome_linker, definicoes) in locais {
        if definicoes.len() != 1 {
            continue;
        }
        let &(nome_fonte, arquivo, linha) = definicoes.first().unwrap();
        let Ok(caminho) = fs::canonicalize(raiz.join(arquivo)) else {
            continue;
        };
        let Ok(relativo) = caminho.strip_prefix(raiz) else {
            continue;
        };
        let Some(arquivo) = relativo.to_str().filter(|p| arquivo_nativo(p)) else {
            continue;
        };
        let Some(texto) = fontes
            .get(arquivo)
            .and_then(|fonte| fonte.lines().nth(linha - 1))
        else {
            continue;
        };
        if let Some(coluna) = coluna_identificador(texto, nome_fonte) {
            confirmados.insert(
                nome_linker.to_owned(),
                LocalFonte {
                    arquivo: arquivo.to_owned(),
                    linha,
                    coluna,
                    linha_fim: linha,
                    coluna_fim: coluna + nome_fonte.len(),
                },
            );
        }
    }
    confirmados
}

fn aplicar_vinculos_rust(
    indice: &mut FontesIndexadas,
    sintaxe: &SintaxeRust,
    confirmados: &BTreeMap<String, LocalFonte>,
) {
    for simbolo in &mut indice.simbolos {
        let nomes: BTreeSet<_> = simbolo
            .declaracoes
            .iter()
            .filter_map(|local| sintaxe.externos.get(local))
            .collect();
        let mut nomes = nomes.into_iter();
        let Some(nome) = nomes.next() else { continue };
        if nomes.next().is_some() {
            continue;
        }
        if let Some(implementacao) = confirmados.get(nome.as_str()) {
            simbolo.ffi = Some(VinculoFfi {
                origem: OrigemFfi::Elf,
                nome_linker: nome.clone(),
                implementacao: implementacao.clone(),
                chamadas_fronteira: ChamadasFonte::new(),
            });
        }
    }
}

fn arquivo_nativo(caminho: &str) -> bool {
    Path::new(caminho)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "c" | "cc" | "cp" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx"
            )
        })
}

fn coluna_identificador(linha: &str, nome: &str) -> Option<usize> {
    let caractere = |c: char| c.is_alphanumeric() || matches!(c, '_' | '$');
    if nome.is_empty() || !nome.chars().all(caractere) {
        return None;
    }
    let mut colunas = linha.match_indices(nome).filter_map(|(coluna, _)| {
        let antes = linha[..coluna].chars().next_back();
        let depois = linha[coluna + nome.len()..].chars().next();
        (!antes.is_some_and(caractere) && !depois.is_some_and(caractere)).then_some(coluna)
    });
    let coluna = colunas.next()?;
    colunas.next().is_none().then_some(coluna)
}

fn ler_cgo<'a>(tabela: &'a str, nomes: &BTreeSet<&str>) -> BTreeMap<&'a str, Option<u64>> {
    let mut wrappers = BTreeMap::<_, usize>::new();
    for linha in tabela.lines() {
        let mut campos = linha.split_ascii_whitespace();
        let Some(nome) = campos.next().and_then(|s| s.strip_prefix("_cgo_")) else {
            continue;
        };
        let Some((hash, destino)) = nome.split_once("_Cfunc_") else {
            continue;
        };
        if !hash.is_empty()
            && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            && campos.next() == Some("T")
        {
            *wrappers.entry(destino).or_default() += 1;
        }
    }
    let vinculados = nomes
        .iter()
        .copied()
        .filter(|nome| wrappers.get(nome) == Some(&1))
        .collect();
    ler_tabela(tabela, &vinculados)
}

fn ler_tabela<'a>(tabela: &'a str, nomes: &BTreeSet<&str>) -> BTreeMap<&'a str, Option<u64>> {
    ler_tabela_codigo(tabela, nomes, false)
}

fn ler_tabela_codigo<'a>(
    tabela: &'a str,
    nomes: &BTreeSet<&str>,
    aceitar_local: bool,
) -> BTreeMap<&'a str, Option<u64>> {
    let mut encontrados = BTreeMap::new();
    for linha in tabela.lines() {
        let mut campos = linha.split_ascii_whitespace();
        let Some(nome) = campos.next().filter(|nome| nomes.contains(nome)) else {
            continue;
        };
        let tipo = campos.next();
        if !aceitar_local && matches!(tipo, Some("t" | "d" | "b" | "r" | "a" | "s" | "g")) {
            continue;
        }
        let endereco = campos
            .next()
            .filter(|_| tipo == Some("T") || (aceitar_local && tipo == Some("t")))
            .and_then(|hex| u64::from_str_radix(hex, 16).ok());
        encontrados
            .entry(nome)
            .and_modify(|anterior| *anterior = None)
            .or_insert(endereco);
    }
    encontrados
}

fn definicoes_dwarf<'a>(
    depuracao: &'a str,
    enderecos: &BTreeMap<&str, Option<u64>>,
) -> DefinicoesDwarf<'a> {
    let mut definicoes = DefinicoesDwarf::new();
    let mut linhas = depuracao.lines().map(str::trim).peekable();
    while let Some(linha) = linhas.next() {
        if !linha.starts_with("0x")
            || !linha
                .split_once(':')
                .is_some_and(|(_, tag)| tag.trim() == "DW_TAG_subprogram")
        {
            continue;
        }
        let atributos: BTreeMap<_, _> =
            std::iter::from_fn(|| linhas.next_if(|linha| linha.starts_with("DW_AT_")))
                .filter_map(|linha| {
                    let (chave, valor) = linha.split_once(char::is_whitespace)?;
                    Some((chave, valor.trim().strip_prefix('(')?.strip_suffix(')')?))
                })
                .collect();
        let texto = |chave| atributos.get(chave)?.strip_prefix('"')?.strip_suffix('"');
        let (Some(nome), Some(arquivo), Some(endereco), Some(linha)) = (
            texto("DW_AT_name"),
            texto("DW_AT_decl_file"),
            atributos
                .get("DW_AT_low_pc")
                .and_then(|valor| valor.strip_prefix("0x"))
                .and_then(|hex| u64::from_str_radix(hex, 16).ok()),
            atributos
                .get("DW_AT_decl_line")
                .and_then(|valor| valor.parse::<usize>().ok())
                .filter(|&linha| linha > 0),
        ) else {
            continue;
        };
        let nome_linker = texto("DW_AT_linkage_name").unwrap_or(nome);
        if enderecos.get(nome_linker) == Some(&Some(endereco)) {
            definicoes
                .entry(nome_linker)
                .or_default()
                .insert((nome, arquivo, linha));
        }
    }
    definicoes
}

struct VisitanteRust<'a> {
    arquivo: &'a str,
    linhas: Vec<usize>,
    sintaxe: &'a mut SintaxeRust,
    chamador: Option<FuncaoFonte>,
}

impl VisitanteRust<'_> {
    fn local(&self, ident: &syn::Ident) -> LocalFonte {
        let faixa = ident.span().byte_range();
        let linha = self.linhas.partition_point(|&inicio| inicio <= faixa.start);
        let inicio = self.linhas[linha - 1];
        LocalFonte {
            arquivo: self.arquivo.into(),
            linha,
            coluna: faixa.start - inicio,
            linha_fim: linha,
            coluna_fim: faixa.end - inicio,
        }
    }

    fn em_funcao(&mut self, ident: &syn::Ident, visitar: impl FnOnce(&mut Self)) {
        self.em_escopo(
            Some(FuncaoFonte {
                nome: ident.to_string(),
                declaracao: self.local(ident),
            }),
            visitar,
        );
    }

    fn em_escopo(&mut self, chamador: Option<FuncaoFonte>, visitar: impl FnOnce(&mut Self)) {
        let anterior = std::mem::replace(&mut self.chamador, chamador);
        visitar(self);
        self.chamador = anterior;
    }
}

impl<'ast> Visit<'ast> for VisitanteRust<'_> {
    fn visit_item_foreign_mod(&mut self, bloco: &'ast syn::ItemForeignMod) {
        if bloco
            .abi
            .name
            .as_ref()
            .is_some_and(|abi| !matches!(abi.value().as_str(), "C" | "C-unwind"))
        {
            return;
        }
        for item in &bloco.items {
            let ForeignItem::Fn(funcao) = item else {
                continue;
            };
            let Some(nome) = nome_de_link(funcao) else {
                continue;
            };
            self.sintaxe
                .externos
                .insert(self.local(&funcao.sig.ident), nome);
        }
    }

    fn visit_item_fn(&mut self, funcao: &'ast syn::ItemFn) {
        self.em_funcao(&funcao.sig.ident, |v| syn::visit::visit_item_fn(v, funcao));
    }

    fn visit_impl_item_fn(&mut self, funcao: &'ast syn::ImplItemFn) {
        self.em_funcao(&funcao.sig.ident, |v| {
            syn::visit::visit_impl_item_fn(v, funcao)
        });
    }

    fn visit_trait_item_fn(&mut self, funcao: &'ast syn::TraitItemFn) {
        self.em_funcao(&funcao.sig.ident, |v| {
            syn::visit::visit_trait_item_fn(v, funcao)
        });
    }

    fn visit_expr_closure(&mut self, expr: &'ast syn::ExprClosure) {
        self.em_escopo(None, |v| syn::visit::visit_expr_closure(v, expr));
    }

    fn visit_expr_async(&mut self, expr: &'ast syn::ExprAsync) {
        self.em_escopo(None, |v| syn::visit::visit_expr_async(v, expr));
    }

    fn visit_expr_call(&mut self, chamada: &'ast syn::ExprCall) {
        let mut funcao = chamada.func.as_ref();
        loop {
            if let Expr::Paren(expr) = funcao {
                funcao = &expr.expr;
            } else if let Expr::Group(expr) = funcao {
                funcao = &expr.expr;
            } else {
                break;
            }
        }
        if let Expr::Path(caminho) = funcao
            && let Some(segmento) = caminho.path.segments.last()
        {
            self.sintaxe
                .chamadas
                .insert(self.local(&segmento.ident), self.chamador.clone());
        }
        syn::visit::visit_expr_call(self, chamada);
    }
}

fn nome_de_link(funcao: &syn::ForeignItemFn) -> Option<String> {
    let mut nome = None;
    for atributo in &funcao.attrs {
        let path = atributo.path();
        if path.is_ident("cfg_attr") || path.is_ident("link_ordinal") {
            return None;
        }
        if path.is_ident("link_name") {
            let Meta::NameValue(meta) = &atributo.meta else {
                return None;
            };
            let Expr::Lit(expr) = &meta.value else {
                return None;
            };
            let Lit::Str(literal) = &expr.lit else {
                return None;
            };
            if nome.replace(literal.value()).is_some() {
                return None;
            }
        }
    }
    Some(nome.unwrap_or_else(|| funcao.sig.ident.unraw().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrai_alias_e_posicao_em_bytes_e_pula_link_condicional() {
        let texto = r#"unsafe extern "C" {
    #[link_name = "exportada"] /* ação 🚀 */ fn alias();
    #[cfg_attr(unix, link_name = "outro")] fn condicional();
}
unsafe extern "Rust" { fn rust(); }"#;
        let mut sintaxe = SintaxeRust::default();
        sintaxe.inspecionar("a.rs", texto);
        let [(local, nome)] = sintaxe.externos.iter().collect::<Vec<_>>()[..] else {
            panic!("esperava um extern");
        };
        assert_eq!(nome, "exportada");
        assert_eq!(
            &texto.lines().nth(1).unwrap()[local.coluna..local.coluna_fim],
            "alias"
        );
    }

    #[test]
    fn chamada_direta_distingue_identificador_de_valor_e_preserva_chamador() {
        let fonte = r#"impl Servico {
    fn calcular() {
        let ponteiro = alias;
        unsafe { /* ação 🚀 */ (ffi::alias)(1) };
        ponteiro(2);
    }
}"#;
        let mut sintaxe = SintaxeRust::default();
        sintaxe.inspecionar("servico.rs", fonte);
        let chamadas: Vec<_> = sintaxe
            .chamadas
            .iter()
            .map(|(local, chamador)| {
                let linha = fonte.lines().nth(local.linha - 1).unwrap();
                (
                    local.linha,
                    &linha[local.coluna..local.coluna_fim],
                    chamador
                        .as_ref()
                        .map(|f| (f.nome.as_str(), f.declaracao.linha)),
                )
            })
            .collect();
        assert_eq!(
            chamadas,
            [
                (4, "alias", Some(("calcular", 2))),
                (5, "ponteiro", Some(("calcular", 2)))
            ]
        );
    }

    #[test]
    fn blocos_anonimos_nao_herdam_a_identidade_da_funcao_externa() {
        let fonte = r#"fn externa() {
    let _closure = || alias();
    let _futuro = async { alias() };
    fn interna() { alias(); }
    alias();
}"#;
        let mut sintaxe = SintaxeRust::default();
        sintaxe.inspecionar("a.rs", fonte);
        let chamadores: Vec<_> = sintaxe
            .chamadas
            .values()
            .map(|f| f.as_ref().map(|f| (f.nome.as_str(), f.declaracao.linha)))
            .collect();
        assert_eq!(
            chamadores,
            [None, None, Some(("interna", 4)), Some(("externa", 1))]
        );
    }

    #[test]
    fn cgo_exige_wrapper_unico_e_node_admite_callback_local() {
        let tabela = "c_func T 10 2\n_cgo_123abcd_Cfunc_c_func T 20 2\ncallback t 30 2\n";
        let cgo = BTreeSet::from(["c_func"]);
        let node = BTreeSet::from(["callback"]);
        assert_eq!(ler_cgo(tabela, &cgo)["c_func"], Some(0x10));
        let duplicado = format!("{tabela}_cgo_12ab_Cfunc_c_func T 40 2\n");
        assert!(ler_cgo(&duplicado, &cgo).is_empty());
        assert!(ler_tabela(tabela, &node).is_empty());
        assert_eq!(
            ler_tabela_codigo(tabela, &node, true)["callback"],
            Some(0x30)
        );
    }

    #[test]
    fn tabela_conserva_ambiguidade_e_requer_simbolo_de_codigo() {
        let nomes = BTreeSet::from(["a", "b", "c", "d"]);
        let tabela = "a T 5 c\t/pasta com espaco/a.c:7\nb T 6 c\t/b.cpp:9\nb T 7 c\nc W 8 c\t/c.c:3\nd U 9\n";
        let lida = ler_tabela(tabela, &nomes);
        assert_eq!(lida["a"], Some(5));
        assert_eq!(lida["b"], None);
        assert_eq!(lida["c"], None);
        assert_eq!(lida["d"], None);
    }

    #[test]
    fn dwarf_requer_o_endereco_do_simbolo_e_marca_o_identificador_inteiro() {
        let depuracao = r#"0x10: DW_TAG_subprogram
    DW_AT_name ("funcao")
    DW_AT_decl_file ("/raiz/caminho com espaço.cpp")
    DW_AT_decl_line (4)
    DW_AT_low_pc (0x50)
0x20: DW_TAG_subprogram
    DW_AT_name ("funcao")
    DW_AT_decl_file ("/raiz/outra.cpp")
    DW_AT_decl_line (4)
    DW_AT_low_pc (0x60)"#;
        let locais = definicoes_dwarf(depuracao, &BTreeMap::from([("funcao", Some(0x50))]));
        assert_eq!(
            locais["funcao"],
            BTreeSet::from([("funcao", "/raiz/caminho com espaço.cpp", 4)])
        );
        assert_eq!(
            coluna_identificador("/* ação 🚀 */ int funcao()", "funcao"),
            Some("/* ação 🚀 */ int ".len())
        );
        assert_eq!(coluna_identificador("int funcao_x", "funcao"), None);
        assert_eq!(
            coluna_identificador("int funcao() { return funcao(); }", "funcao"),
            None
        );
    }

    #[test]
    fn arquivo_relocavel_nao_e_artefato_final() {
        let mut bytes = [0; 20];
        bytes[..6].copy_from_slice(b"\x7fELF\x02\x01");
        bytes[16] = 1;
        assert!(!elf_final(&bytes));
        bytes[16] = 3;
        assert!(elf_final(&bytes));
    }
}
