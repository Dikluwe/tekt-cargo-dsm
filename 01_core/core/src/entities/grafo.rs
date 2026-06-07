//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/grafo.md
//! @prompt-hash 0d481569
//! @layer L1
//! @updated 2026-06-07
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0001-fonte-do-grafo-fork-externo.md
//!          00_nucleo/adr/0002-modelagem-do-grafo.md
//! Camada:  L1 — Núcleo. Apenas stdlib. Sem I/O. Sem cálculo.

use core::error::Error;
use core::fmt;

/// Erro de tradução texto→enum.
///
/// Levanta-se quando o L3 receber, num campo de lista fechada
/// (`kind`, `relation`, `visibility`), um texto que não corresponde a
/// nenhuma variante conhecida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValorDesconhecido {
    pub tipo: &'static str,
    pub texto: String,
}

impl fmt::Display for ValorDesconhecido {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "valor {:?} desconhecido para {}", self.texto, self.tipo)
    }
}

impl Error for ValorDesconhecido {}

/// Caminho canônico de um item.
///
/// Newtype sobre `String` para distinguir um path do grafo de uma string
/// qualquer (segurança de tipo na assinatura de funções do cálculo).
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path(String);

impl Path {
    pub fn new(s: impl Into<String>) -> Self {
        Path(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Path {
    fn from(s: String) -> Self {
        Path(s)
    }
}

impl From<&str> for Path {
    fn from(s: &str) -> Self {
        Path(s.to_string())
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Relação dirigida entre dois nós (lista fechada da spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Relation {
    Owns,
    Uses,
}

impl TryFrom<&str> for Relation {
    type Error = ValorDesconhecido;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "owns" => Ok(Relation::Owns),
            "uses" => Ok(Relation::Uses),
            outro => Err(ValorDesconhecido {
                tipo: "Relation",
                texto: outro.to_string(),
            }),
        }
    }
}

/// Subtipo da relação `Uses` — emitido pelo fork `cargo-modules` no campo
/// `uses_kind` da aresta (laudo 0033). Distingue uso de tipo direto
/// (`Reference` — assinatura/campo) de declaração de import no nível do
/// módulo (`Import` — Limite 4 da spec). Aresta `Owns` não tem subtipo
/// (`uses_kind = None`).
///
/// O fork **funde** reexports em `Import` no momento (laudo 0033 D6);
/// se um dia distinguir `Reexport`, fica a evolução natural (variante
/// nova). Por isso `TryFrom` mapeia valores desconhecidos para `Import`
/// — comportamento conservador, coerente com o que o fork faz hoje.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UsesKind {
    Reference,
    Import,
}

impl TryFrom<&str> for UsesKind {
    type Error = ValorDesconhecido;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "reference" => Ok(UsesKind::Reference),
            "import" => Ok(UsesKind::Import),
            outro => Err(ValorDesconhecido {
                tipo: "UsesKind",
                texto: outro.to_string(),
            }),
        }
    }
}

/// Visibilidade de um item (lista fechada da spec).
///
/// `PubIn` preserva o caminho declarado em `pub(in <caminho>)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Visibility {
    Pub,
    PubCrate,
    PubSuper,
    PubIn(String),
    Priv,
}

impl TryFrom<&str> for Visibility {
    type Error = ValorDesconhecido;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "pub" => Ok(Visibility::Pub),
            "pub(crate)" => Ok(Visibility::PubCrate),
            "pub(super)" => Ok(Visibility::PubSuper),
            "priv" => Ok(Visibility::Priv),
            outro => {
                if let Some(resto) = outro
                    .strip_prefix("pub(in ")
                    .and_then(|t| t.strip_suffix(')'))
                {
                    let caminho = resto.trim();
                    if !caminho.is_empty() {
                        return Ok(Visibility::PubIn(caminho.to_string()));
                    }
                }
                Err(ValorDesconhecido {
                    tipo: "Visibility",
                    texto: outro.to_string(),
                })
            }
        }
    }
}

/// Tipo **base** do item — sem os modificadores `const`/`async`/`unsafe`,
/// que vivem em [`Modificadores`]. A string `kind` do fork pode trazer os
/// modificadores embutidos (`"const async unsafe fn"`); o `TryFrom` despe-os
/// e mantém só o tipo base (`Fn`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Crate,
    Mod,
    Fn,
    Struct,
    Union,
    Enum,
    Variant,
    Const,
    Static,
    Trait,
    Type,
    Builtin,
    Macro,
}

impl TryFrom<&str> for Kind {
    type Error = ValorDesconhecido;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // O tipo base é o ÚLTIMO token. Modificadores (`const`/`async`/
        // `unsafe`) precedem-no e são responsabilidade de `Modificadores`
        // (preenchido a partir dos booleanos do fork, no `lente_infra`).
        // Pegar o último token resolve a ambiguidade `const` (item Const,
        // sozinho) vs `const fn` (modificador + Fn).
        let base = s.rsplit(' ').next().unwrap_or(s);
        use Kind::*;
        let kind = match base {
            "crate" => Crate,
            "mod" => Mod,
            "fn" => Fn,
            "struct" => Struct,
            "union" => Union,
            "enum" => Enum,
            "variant" => Variant,
            "const" => Const,
            "static" => Static,
            "trait" => Trait,
            "type" => Type,
            "builtin" => Builtin,
            "macro" => Macro,
            _ => {
                return Err(ValorDesconhecido {
                    tipo: "Kind",
                    texto: s.to_string(),
                });
            }
        };
        Ok(kind)
    }
}

/// Modificadores de um item (separados do [`Kind`], que é só o tipo base).
/// Fonte da verdade: os booleanos do descritor do fork (não a string `kind`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Modificadores {
    pub is_const: bool,
    pub is_async: bool,
    pub is_unsafe: bool,
}

/// Posição de um nó no código-fonte, como o fork `cargo-modules` a emite
/// (prompt 0037 — 5ª rodada do fork).
///
/// Ausente para itens sem fonte (tipos embutidos da stdlib, ou crates de
/// dependência cujo fonte não foi processado). Para itens gerados por
/// macro, é a posição do **call-site** (o fork não inventa expansão).
/// Linhas contadas a partir de 1 (1-based).
///
/// `file` é armazenado **verbatim** como o fork o emite — caminho
/// **absoluto**. Relativizar para casar com paths de um `git diff` é
/// trabalho do mapeamento diff→nós (prompt futuro), não deste nível.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posicao {
    pub file: String,
    pub start_line: u32,
    pub end_line: u32,
}

/// Nó do grafo. Identidade canônica é `id` (atribuído pela fonte; o
/// `path` **pode repetir** entre nós distintos no mesmo grafo — ex.: dois
/// métodos `fmt` colidentes via Display+Debug).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct No {
    pub id: usize,
    pub path: Path,
    pub name: String,
    pub kind: Kind,
    pub modificadores: Modificadores,
    pub visibility: Visibility,
    /// Crate-raiz do **grafo**, copiado para cada nó pelo L3.
    ///
    /// O fork `cargo-modules` 0.27.0 **não** emite `crate` por nó (laudo
    /// 0013 D1); o L3 (`lente_infra::traducao`) preenche este campo com o
    /// `Grafo.crate_name` para todos os nós — inclusive os de sysroot
    /// (`core::*`, `alloc::*`, `std::*`).
    ///
    /// **Não distingue stdlib** do crate-alvo: o valor é igual para todos
    /// os nós do mesmo grafo. A marca de stdlib é por **prefixo do path**
    /// (ADR-0002 D3), aplicada no `lente_filtro` (laudo 0025).
    pub crate_name: String,
    /// Nome do trait, quando o nó é método de impl-de-trait. `None` caso não.
    pub trait_: Option<String>,
    /// Referência do trait com seus argumentos (texto, sem parsing).
    pub trait_ref: Option<String>,
    /// Expressão `cfg` como texto (sem interpretação).
    pub cfg: Option<String>,
    /// Tipo de macro, quando o nó é uma macro. `None` caso não.
    pub macro_kind: Option<String>,
    pub is_non_exhaustive: bool,
    /// Posição do nó no código-fonte (prompt 0037). `None` quando o item
    /// não tem fonte (tipo embutido) ou quando o JSON é de um fork antigo
    /// que ainda não emite o campo. **Não é erro** quando ausente — é a
    /// natureza opcional do campo (diferente do `id`, que é obrigatório).
    pub position: Option<Posicao>,
}

/// Aresta dirigida do grafo. `id_from`/`id_to` são a referência canônica
/// (resolvem colisões); `from`/`to` permanecem para legibilidade.
///
/// `uses_kind` (prompt 0034) é `Some` apenas para arestas `Uses` quando o
/// fork emite o campo (`cargo-modules` pós-commit `b44aa96`). É `None`
/// para arestas `Owns` e para arestas `Uses` desserializadas de JSON
/// antigo (sem o campo). Consumidores que filtram por subtipo (ex.:
/// `lente_filtro::filtrar_so_referencia`) precisam tratar o caso `None`
/// explicitamente — ver a documentação dessa função.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aresta {
    pub from: Path,
    pub id_from: usize,
    pub to: Path,
    pub id_to: usize,
    pub relation: Relation,
    pub uses_kind: Option<UsesKind>,
}

/// Grafo de dependências de um sistema. Fiel à forma organizada.
///
/// `crate_name` corresponde ao campo `crate` do JSON; renomeado por ser
/// `crate` palavra reservada em Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grafo {
    pub crate_name: String,
    pub nodes: Vec<No>,
    pub edges: Vec<Aresta>,
}

impl Grafo {
    pub fn new(crate_name: impl Into<String>) -> Self {
        Grafo {
            crate_name: crate_name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relation_owns_e_uses_traduzem() {
        assert_eq!(Relation::try_from("owns").unwrap(), Relation::Owns);
        assert_eq!(Relation::try_from("uses").unwrap(), Relation::Uses);
    }

    #[test]
    fn relation_desconhecida_retorna_erro() {
        let err = Relation::try_from("borrows").unwrap_err();
        assert_eq!(err.tipo, "Relation");
        assert_eq!(err.texto, "borrows");
    }

    #[test]
    fn uses_kind_reference_e_import_traduzem() {
        assert_eq!(UsesKind::try_from("reference").unwrap(), UsesKind::Reference);
        assert_eq!(UsesKind::try_from("import").unwrap(), UsesKind::Import);
    }

    #[test]
    fn uses_kind_desconhecido_retorna_erro() {
        // O fork não distingue `reexport` hoje (laudo 0033 D6) — mas se um
        // dia distinguir, o `try_from` falhará aqui, sinalizando que cabe
        // adicionar variante. Por enquanto, a borda detecta valor estranho.
        let err = UsesKind::try_from("reexport").unwrap_err();
        assert_eq!(err.tipo, "UsesKind");
        assert_eq!(err.texto, "reexport");
    }

    // ---- prompt 0037: Posicao + No.position ---------------------------------

    #[test]
    fn posicao_carrega_arquivo_e_linhas_1_based() {
        let p = Posicao {
            file: "/abs/src/lib.rs".to_string(),
            start_line: 10,
            end_line: 42,
        };
        assert_eq!(p.file, "/abs/src/lib.rs");
        assert_eq!(p.start_line, 10);
        assert_eq!(p.end_line, 42);
    }

    /// Constrói um nó com `Some(Posicao)`; confere acesso aos três campos.
    #[test]
    fn no_com_position_some_e_acessivel() {
        let n = No {
            id: 7,
            path: Path::from("k::a"),
            name: "a".to_string(),
            kind: Kind::Fn,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: "k".to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: Some(Posicao {
                file: "/abs/k/src/lib.rs".to_string(),
                start_line: 5,
                end_line: 9,
            }),
        };
        let p = n.position.as_ref().expect("position presente");
        assert_eq!(p.file, "/abs/k/src/lib.rs");
        assert_eq!(p.start_line, 5);
        assert_eq!(p.end_line, 9);
    }

    /// `None` em `position` é estado válido (prompt 0037 §"position é
    /// opcional por natureza" — itens embutidos ou JSON antigo). Não é
    /// erro construir um nó assim.
    #[test]
    fn no_com_position_none_e_estado_valido() {
        let n = No {
            id: 8,
            path: Path::from("core::fmt"),
            name: "fmt".to_string(),
            kind: Kind::Mod,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: "k".to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: None,
        };
        assert!(n.position.is_none());
    }

    #[test]
    fn kind_cobre_os_treze_tipos_base() {
        let pares: &[(&str, Kind)] = &[
            ("crate", Kind::Crate),
            ("mod", Kind::Mod),
            ("fn", Kind::Fn),
            ("struct", Kind::Struct),
            ("union", Kind::Union),
            ("enum", Kind::Enum),
            ("variant", Kind::Variant),
            ("const", Kind::Const),
            ("static", Kind::Static),
            ("trait", Kind::Trait),
            ("type", Kind::Type),
            ("builtin", Kind::Builtin),
            ("macro", Kind::Macro),
        ];
        for (texto, esperado) in pares {
            assert_eq!(
                Kind::try_from(*texto).unwrap(),
                *esperado,
                "kind {:?}",
                texto
            );
        }
    }

    #[test]
    fn kind_despe_modificadores_e_pega_tipo_base() {
        assert_eq!(Kind::try_from("const fn").unwrap(), Kind::Fn);
        assert_eq!(Kind::try_from("async fn").unwrap(), Kind::Fn);
        assert_eq!(Kind::try_from("unsafe fn").unwrap(), Kind::Fn);
        assert_eq!(Kind::try_from("const async unsafe fn").unwrap(), Kind::Fn);
        assert_eq!(Kind::try_from("unsafe trait").unwrap(), Kind::Trait);
    }

    #[test]
    fn kind_const_sozinho_e_o_tipo_const_nao_modificador() {
        // Ambiguidade resolvida pelo último token: "const" sozinho é o item
        // Const; "const fn" é Fn com modificador.
        assert_eq!(Kind::try_from("const").unwrap(), Kind::Const);
        assert_eq!(Kind::try_from("const fn").unwrap(), Kind::Fn);
    }

    #[test]
    fn kind_desconhecido_retorna_erro() {
        // Último token não é tipo base conhecido.
        let err = Kind::try_from("frobnicate").unwrap_err();
        assert_eq!(err.tipo, "Kind");
        assert_eq!(err.texto, "frobnicate");
    }

    #[test]
    fn visibility_textos_canonicos_traduzem() {
        assert_eq!(Visibility::try_from("pub").unwrap(), Visibility::Pub);
        assert_eq!(
            Visibility::try_from("pub(crate)").unwrap(),
            Visibility::PubCrate
        );
        assert_eq!(
            Visibility::try_from("pub(super)").unwrap(),
            Visibility::PubSuper
        );
        assert_eq!(Visibility::try_from("priv").unwrap(), Visibility::Priv);
    }

    #[test]
    fn visibility_pub_in_preserva_caminho() {
        let v = Visibility::try_from("pub(in crate::a::b)").unwrap();
        assert_eq!(v, Visibility::PubIn("crate::a::b".to_string()));
    }

    #[test]
    fn visibility_pub_in_vazio_e_erro() {
        let err = Visibility::try_from("pub(in )").unwrap_err();
        assert_eq!(err.tipo, "Visibility");
    }

    #[test]
    fn visibility_desconhecida_retorna_erro() {
        let err = Visibility::try_from("hidden").unwrap_err();
        assert_eq!(err.tipo, "Visibility");
        assert_eq!(err.texto, "hidden");
    }

    /// Constrói um `No` com os campos do descritor em default (None/false).
    fn no_de(id: usize, path: &str, name: &str, kind: Kind) -> No {
        No {
            id,
            path: Path::from(path),
            name: name.to_string(),
            kind,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: "meu".to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: None,
        }
    }

    #[test]
    fn grafo_construido_preserva_nos_e_arestas() {
        let mut g = Grafo::new("meu");
        let raiz = no_de(1, "meu", "meu", Kind::Crate);
        let filho = no_de(2, "meu::foo", "foo", Kind::Mod);
        g.nodes.push(raiz.clone());
        g.nodes.push(filho.clone());
        g.edges.push(Aresta {
            from: Path::from("meu"),
            id_from: 1,
            to: Path::from("meu::foo"),
            id_to: 2,
            relation: Relation::Owns,
            uses_kind: None,
        });

        assert_eq!(g.crate_name, "meu");
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.nodes[0], raiz);
        assert_eq!(g.nodes[1], filho);
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.edges[0].relation, Relation::Owns);
        assert_eq!(g.edges[0].from.as_str(), "meu");
        assert_eq!(g.edges[0].to.as_str(), "meu::foo");
        assert_eq!(g.edges[0].id_from, 1);
        assert_eq!(g.edges[0].id_to, 2);
    }

    #[test]
    fn grafo_minimo_so_raiz_e_valido() {
        let mut g = Grafo::new("solo");
        g.nodes.push(no_de(1, "solo", "solo", Kind::Crate));
        assert_eq!(g.nodes.len(), 1);
        assert!(g.edges.is_empty());
    }

    #[test]
    fn modificadores_default_tudo_false() {
        let m = Modificadores::default();
        assert!(!m.is_const);
        assert!(!m.is_async);
        assert!(!m.is_unsafe);
    }

    #[test]
    fn no_carrega_descritor_semantico() {
        let mut n = no_de(7, "c::T::fmt", "fmt", Kind::Fn);
        n.trait_ = Some("Display".to_string());
        n.trait_ref = Some("Display".to_string());
        n.modificadores = Modificadores {
            is_const: true,
            is_async: false,
            is_unsafe: true,
        };
        n.is_non_exhaustive = true;
        n.cfg = Some("unix".to_string());
        n.macro_kind = None;

        assert_eq!(n.trait_.as_deref(), Some("Display"));
        assert_eq!(n.cfg.as_deref(), Some("unix"));
        assert!(n.modificadores.is_const && n.modificadores.is_unsafe);
        assert!(n.is_non_exhaustive);
    }

    #[test]
    fn valor_desconhecido_implementa_display() {
        let err = Relation::try_from("xyz").unwrap_err();
        let s = format!("{}", err);
        assert!(s.contains("xyz"));
        assert!(s.contains("Relation"));
    }
}
