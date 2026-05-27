//! Lineage: prompt 00_nucleo/prompt/0001-dados_grafo.md
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

/// Tipo do item (lista fechada da spec, específica de Rust).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Crate,
    Mod,
    Fn,
    ConstFn,
    AsyncFn,
    UnsafeFn,
    Struct,
    Union,
    Enum,
    Variant,
    Const,
    Static,
    Trait,
    UnsafeTrait,
    Type,
    Builtin,
    Macro,
}

impl TryFrom<&str> for Kind {
    type Error = ValorDesconhecido;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        use Kind::*;
        let kind = match s {
            "crate" => Crate,
            "mod" => Mod,
            "fn" => Fn,
            "const fn" => ConstFn,
            "async fn" => AsyncFn,
            "unsafe fn" => UnsafeFn,
            "struct" => Struct,
            "union" => Union,
            "enum" => Enum,
            "variant" => Variant,
            "const" => Const,
            "static" => Static,
            "trait" => Trait,
            "unsafe trait" => UnsafeTrait,
            "type" => Type,
            "builtin" => Builtin,
            "macro" => Macro,
            outro => {
                return Err(ValorDesconhecido {
                    tipo: "Kind",
                    texto: outro.to_string(),
                });
            }
        };
        Ok(kind)
    }
}

/// Nó do grafo. Identidade canônica é `path`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct No {
    pub path: Path,
    pub name: String,
    pub kind: Kind,
    pub visibility: Visibility,
}

/// Aresta dirigida do grafo. `from` e `to` referenciam `path` de nós.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aresta {
    pub from: Path,
    pub to: Path,
    pub relation: Relation,
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
    fn kind_cobre_lista_fechada_inteira() {
        let pares: &[(&str, Kind)] = &[
            ("crate", Kind::Crate),
            ("mod", Kind::Mod),
            ("fn", Kind::Fn),
            ("const fn", Kind::ConstFn),
            ("async fn", Kind::AsyncFn),
            ("unsafe fn", Kind::UnsafeFn),
            ("struct", Kind::Struct),
            ("union", Kind::Union),
            ("enum", Kind::Enum),
            ("variant", Kind::Variant),
            ("const", Kind::Const),
            ("static", Kind::Static),
            ("trait", Kind::Trait),
            ("unsafe trait", Kind::UnsafeTrait),
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
    fn kind_desconhecido_retorna_erro() {
        let err = Kind::try_from("extern fn").unwrap_err();
        assert_eq!(err.tipo, "Kind");
        assert_eq!(err.texto, "extern fn");
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

    #[test]
    fn grafo_construido_preserva_nos_e_arestas() {
        let mut g = Grafo::new("meu");
        let raiz = No {
            path: Path::from("meu"),
            name: "meu".to_string(),
            kind: Kind::Crate,
            visibility: Visibility::Pub,
        };
        let filho = No {
            path: Path::from("meu::foo"),
            name: "foo".to_string(),
            kind: Kind::Mod,
            visibility: Visibility::Pub,
        };
        g.nodes.push(raiz.clone());
        g.nodes.push(filho.clone());
        g.edges.push(Aresta {
            from: Path::from("meu"),
            to: Path::from("meu::foo"),
            relation: Relation::Owns,
        });

        assert_eq!(g.crate_name, "meu");
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.nodes[0], raiz);
        assert_eq!(g.nodes[1], filho);
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.edges[0].relation, Relation::Owns);
        assert_eq!(g.edges[0].from.as_str(), "meu");
        assert_eq!(g.edges[0].to.as_str(), "meu::foo");
    }

    #[test]
    fn grafo_minimo_so_raiz_e_valido() {
        let mut g = Grafo::new("solo");
        g.nodes.push(No {
            path: Path::from("solo"),
            name: "solo".to_string(),
            kind: Kind::Crate,
            visibility: Visibility::Pub,
        });
        assert_eq!(g.nodes.len(), 1);
        assert!(g.edges.is_empty());
    }

    #[test]
    fn valor_desconhecido_implementa_display() {
        let err = Relation::try_from("xyz").unwrap_err();
        let s = format!("{}", err);
        assert!(s.contains("xyz"));
        assert!(s.contains("Relation"));
    }
}
