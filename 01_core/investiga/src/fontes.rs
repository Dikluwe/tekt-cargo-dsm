//! QUARENTENA DE REMOÇÃO (desde 2026-05-28, laudo 0014).
//!
//! Este módulo (E2 — parser textual de fontes) extraía o trait de impls
//! lendo o código-fonte, porque o JSON do fork não trazia o trait. O fork
//! 0.27.0 passou a emitir `trait` por nó (laudo 0013), tornando a E2
//! desnecessária para seu propósito original.
//!
//! Mantido fora do caminho da cascata, não removido, por INCERTEZA: a
//! medição de generalização contra crates de outras origens ainda não foi
//! feita. Condição de saída da quarentena:
//!   - REMOVER se a medição confirmar que E1 + trait-por-nó cobrem tudo.
//!   - RELIGAR ao caminho se a medição revelar casos que só a E2 decide.
//! Ver laudo 0014 em 00_nucleo/lessons/ para o registro completo.
//!
//! ---
//! Lineage original: prompt 00_nucleo/prompt/0004-lente_investiga.md
//!
//! Estratégia 2 — leitura de código (parser textual limitado).
//! Reconhece `impl <Trait> for <Tipo> { ... fn <metodo> ... }` por scanning
//! linha-a-linha. Trait normalizado pelo último segmento (`fmt::Display` →
//! `Display`). Limitações conhecidas (viram `NaoDeterminado`): genéricos com
//! `where` multilinha, `#[cfg]`, macros que geram impls, comentários
//! `// impl X for Y`, strings com `{`/`}`.

use lente_core::entities::veredito::{Evidencia, Veredito};

use crate::{ArquivoFonte, ParColidente};

pub(crate) enum ResultadoFontes {
    Decidiu(Veredito),
    Inconclusivo(String), // diagnóstico
}

/// E2 ponta-a-ponta a partir do par colidente (cola que vivia em `lib.rs`
/// antes da quarentena). Extrai (tipo, método) do path e investiga as fontes.
/// Fora do caminho da cascata; exercitada pelos testes para manter a E2
/// reconstruível e verificável enquanto na quarentena.
pub(crate) fn investigar_por_fontes(
    par: ParColidente<'_>,
    fontes: &[ArquivoFonte],
) -> Veredito {
    let (tipo, metodo) = match extrair_tipo_e_metodo(par.a.path.as_str()) {
        Some(p) => p,
        None => {
            return Veredito::NaoDeterminado {
                diagnostico: format!(
                    "E2: path {:?} não tem formato Tipo::método.",
                    par.a.path.as_str()
                ),
            };
        }
    };
    match analisar(&tipo, &metodo, fontes) {
        ResultadoFontes::Decidiu(v) => v,
        ResultadoFontes::Inconclusivo(d) => Veredito::NaoDeterminado { diagnostico: d },
    }
}

/// Extrai (tipo, método) dos dois últimos segmentos do path.
/// `crate::mod::ErroRaio::fmt` → `("ErroRaio", "fmt")`.
fn extrair_tipo_e_metodo(path: &str) -> Option<(String, String)> {
    let segs: Vec<&str> = path.split("::").collect();
    if segs.len() < 2 {
        return None;
    }
    let metodo = segs[segs.len() - 1].to_string();
    let tipo = segs[segs.len() - 2].to_string();
    if tipo.is_empty() || metodo.is_empty() {
        None
    } else {
        Some((tipo, metodo))
    }
}

/// Procura nos `fontes` por dois `impl <Trait> for <tipo_alvo>` distintos
/// onde cada um declare `fn <metodo_alvo>`.
pub(crate) fn analisar(
    tipo_alvo: &str,
    metodo_alvo: &str,
    fontes: &[ArquivoFonte],
) -> ResultadoFontes {
    let mut traits_com_metodo: Vec<String> = Vec::new();

    for arquivo in fontes {
        for impl_b in extrair_impls(&arquivo.conteudo, tipo_alvo) {
            if impl_b.metodos.iter().any(|m| m == metodo_alvo) {
                traits_com_metodo.push(impl_b.nome_trait);
            }
        }
    }

    // Deduplicar mantendo a ordem (pode haver impls em arquivos diferentes
    // com mesmo trait, ou repetições espúrias).
    let mut unicos: Vec<String> = Vec::new();
    for t in traits_com_metodo {
        if !unicos.contains(&t) {
            unicos.push(t);
        }
    }

    if unicos.len() >= 2 {
        ResultadoFontes::Decidiu(Veredito::Distintos {
            evidencia: Evidencia::ImplDeTraitsDiferentes {
                traits: (unicos[0].clone(), unicos[1].clone()),
            },
        })
    } else {
        ResultadoFontes::Inconclusivo(format!(
            "Estratégia 2 (fontes): {} impl(s) com método {:?} para tipo {:?} \
             — esperava 2+ traits distintos; encontrei {:?}",
            unicos.len(),
            metodo_alvo,
            tipo_alvo,
            unicos
        ))
    }
}

#[derive(Debug)]
struct ImplEncontrado {
    nome_trait: String,
    metodos: Vec<String>,
}

/// Varre o texto procurando `impl <X> for <tipo_alvo>` (com possíveis genéricos
/// no impl) e extrai, para cada bloco encontrado, o nome do trait (último
/// segmento de path) e os métodos declarados dentro.
fn extrair_impls(texto: &str, tipo_alvo: &str) -> Vec<ImplEncontrado> {
    let mut achados: Vec<ImplEncontrado> = Vec::new();
    let linhas: Vec<&str> = texto.lines().collect();
    let mut i = 0;

    while i < linhas.len() {
        let linha = linhas[i];
        if let Some(nome_trait) = trait_de_impl_for_tipo(linha, tipo_alvo) {
            let (metodos, consumidas) = extrair_corpo(&linhas, i);
            achados.push(ImplEncontrado {
                nome_trait,
                metodos,
            });
            i += consumidas.max(1);
        } else {
            i += 1;
        }
    }

    achados
}

/// Se a linha começa um bloco `impl <Trait> for <tipo_alvo>`, retorna o nome
/// simplificado do trait (último segmento). Senão, `None`.
fn trait_de_impl_for_tipo(linha: &str, tipo_alvo: &str) -> Option<String> {
    let l = linha.trim();
    if !l.starts_with("impl") {
        return None;
    }
    // Linha não pode ser comentário direto (// ...). trim() já remove indent.
    if l.starts_with("//") {
        return None;
    }
    let resto = l[4..].trim_start();

    // Pular genéricos do impl: se começar com '<', encontrar o '>' correspondente.
    let resto = if resto.starts_with('<') {
        pular_generico(resto)?.trim_start()
    } else {
        resto
    };

    // Precisa de " for " para ser impl-de-trait (não inerente).
    let pos_for = resto.find(" for ")?;
    let trait_completo = resto[..pos_for].trim();
    if trait_completo.is_empty() {
        return None;
    }
    let depois_for = resto[pos_for + 5..].trim_start();

    // Nome do tipo: até o primeiro caractere que termina identificador.
    let nome_tipo: String = depois_for
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if nome_tipo != tipo_alvo {
        return None;
    }

    // Normalizar trait: último segmento de "fmt::Display" → "Display".
    let nome_trait = trait_completo
        .rsplit("::")
        .next()
        .unwrap_or(trait_completo)
        .to_string();
    Some(nome_trait)
}

/// Dadas as linhas e o índice da linha que abriu o `impl`, varre até o `}`
/// correspondente contando chaves. Devolve métodos encontrados e quantas
/// linhas consumiu (a partir da inicial inclusive).
fn extrair_corpo(linhas: &[&str], inicio: usize) -> (Vec<String>, usize) {
    let mut metodos: Vec<String> = Vec::new();
    let mut depth: i32 = 0;
    let mut viu_primeira_chave = false;

    for (offset, linha) in linhas[inicio..].iter().enumerate() {
        // Métodos contam quando depth==1 (corpo do impl), ignorando o que está
        // dentro de funções aninhadas (depth>=2).
        if depth == 1 {
            if let Some(nome) = nome_de_fn(linha) {
                metodos.push(nome);
            }
        }
        for c in linha.chars() {
            match c {
                '{' => {
                    depth += 1;
                    viu_primeira_chave = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if viu_primeira_chave && depth <= 0 {
            return (metodos, offset + 1);
        }
    }

    (metodos, linhas.len() - inicio)
}

/// Se a linha (trimada) começa uma `fn <nome>(...)`, retorna `<nome>`.
fn nome_de_fn(linha: &str) -> Option<String> {
    let mut l = linha.trim();
    // Comentários nunca casam.
    if l.starts_with("//") {
        return None;
    }
    // Pular qualificadores comuns.
    let prefixos = [
        "pub(crate) ", "pub(super) ", "pub ", "async ", "unsafe ", "const ", "extern ",
    ];
    let mut mudou = true;
    while mudou {
        mudou = false;
        for p in &prefixos {
            if let Some(r) = l.strip_prefix(p) {
                l = r.trim_start();
                mudou = true;
            }
        }
    }
    let l = l.strip_prefix("fn ")?;
    let nome: String = l
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if nome.is_empty() {
        None
    } else {
        Some(nome)
    }
}

/// `s` começa com `<`. Encontra o `>` que fecha (contando profundidade) e
/// devolve o sufixo a partir do caractere imediatamente após.
fn pular_generico(s: &str) -> Option<&str> {
    let mut depth: i32 = 0;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[i + 1..]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arq(conteudo: &str) -> ArquivoFonte {
        ArquivoFonte {
            caminho_logico: "teste.rs".to_string(),
            conteudo: conteudo.to_string(),
        }
    }

    #[test]
    fn detecta_display_e_debug_para_erro_raio() {
        let src = r#"
pub enum ErroRaio { X }

impl fmt::Display for ErroRaio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "x")
    }
}

impl fmt::Debug for ErroRaio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "X")
    }
}
"#;
        let fontes = [arq(src)];
        match analisar("ErroRaio", "fmt", &fontes) {
            ResultadoFontes::Decidiu(Veredito::Distintos {
                evidencia: Evidencia::ImplDeTraitsDiferentes { traits },
            }) => {
                let par = (traits.0.as_str(), traits.1.as_str());
                assert!(
                    par == ("Display", "Debug") || par == ("Debug", "Display"),
                    "traits inesperados: {:?}",
                    par
                );
            }
            _ => panic!("esperava ImplDeTraitsDiferentes"),
        }
    }

    #[test]
    fn impl_inerente_e_ignorado() {
        // impl Tipo { ... } sem `for` não é impl-de-trait — não deve gerar evidência.
        let src = r#"
impl ErroRaio {
    fn novo() -> Self { todo!() }
}
"#;
        match analisar("ErroRaio", "novo", &[arq(src)]) {
            ResultadoFontes::Inconclusivo(_) => {}
            _ => panic!("impl inerente não deve fechar evidência"),
        }
    }

    #[test]
    fn metodo_dentro_de_funcao_aninhada_nao_conta() {
        // `fn outra` aparece dentro do corpo de `fmt` — depth==2, não conta.
        let src = r#"
impl Display for ErroRaio {
    fn fmt(&self, f: &mut Formatter) -> Result {
        fn outra() { /* nada */ }
        outra();
        Ok(())
    }
}
impl Debug for ErroRaio {
    fn fmt(&self, f: &mut Formatter) -> Result { Ok(()) }
}
"#;
        match analisar("ErroRaio", "fmt", &[arq(src)]) {
            ResultadoFontes::Decidiu(Veredito::Distintos {
                evidencia: Evidencia::ImplDeTraitsDiferentes { .. },
            }) => {}
            _ => panic!("método aninhado não deveria ter atrapalhado"),
        }
    }

    #[test]
    fn impl_com_genericos_e_reconhecido() {
        let src = r#"
impl<T: Clone> Display for ErroRaio<T> {
    fn fmt(&self, f: &mut Formatter) -> Result { Ok(()) }
}
"#;
        // Tipo "ErroRaio" pode ser reconhecido mesmo com `<T>` depois.
        let achados = extrair_impls(src, "ErroRaio");
        assert_eq!(achados.len(), 1);
        assert_eq!(achados[0].nome_trait, "Display");
    }

    #[test]
    fn comentario_com_impl_for_nao_e_falso_positivo() {
        let src = r#"
// impl Display for ErroRaio { ... } // comentário, não código
fn algo() {}
"#;
        let achados = extrair_impls(src, "ErroRaio");
        assert!(achados.is_empty(), "comentário não deveria casar");
    }

    #[test]
    fn so_um_impl_de_trait_e_inconclusivo() {
        let src = r#"
impl Display for ErroRaio {
    fn fmt(&self, f: &mut Formatter) -> Result { Ok(()) }
}
"#;
        match analisar("ErroRaio", "fmt", &[arq(src)]) {
            ResultadoFontes::Inconclusivo(_) => {}
            _ => panic!("um impl só não basta"),
        }
    }

    /// A E2 em quarentena segue testada ponta-a-ponta via `investigar_por_fontes`
    /// — garante que continua reconstruível enquanto fora do caminho.
    #[test]
    fn investigar_por_fontes_decide_ponta_a_ponta() {
        use lente_core::entities::grafo::{Kind, Modificadores, No, Path, Visibility};

        fn no(path: &str) -> No {
            No {
                id: 0,
                path: Path::from(path),
                name: path.rsplit("::").next().unwrap_or(path).to_string(),
                kind: Kind::Fn,
                modificadores: Modificadores::default(),
                visibility: Visibility::Pub,
                crate_name: "t".to_string(),
                trait_: None,
                trait_ref: None,
                cfg: None,
                macro_kind: None,
                is_non_exhaustive: false,
                position: None,
            }
        }

        let a = no("c::ErroRaio::fmt");
        let b = no("c::ErroRaio::fmt");
        let par = ParColidente { a: &a, b: &b };
        let src = r#"
impl fmt::Display for ErroRaio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { Ok(()) }
}
impl fmt::Debug for ErroRaio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { Ok(()) }
}
"#;
        match investigar_por_fontes(par, &[arq(src)]) {
            Veredito::Distintos {
                evidencia: Evidencia::ImplDeTraitsDiferentes { traits },
            } => {
                let p = (traits.0.as_str(), traits.1.as_str());
                assert!(p == ("Display", "Debug") || p == ("Debug", "Display"));
            }
            outro => panic!("E2 ponta-a-ponta deveria decidir, veio {:?}", outro),
        }
    }
}
