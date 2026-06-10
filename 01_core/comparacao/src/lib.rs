//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/comparacao.md
//! @prompt-hash 1fc86eec
//! @layer L1
//! @updated 2026-06-10
//! Camada:  L1 — Núcleo. Pureza: stdlib + `lente_core` + `lente_estrutura`.
//!
//! **Paridade como dado** (prompt 0074): compara **duas** [`EstruturaModulos`]
//! (antes/depois de uma refatoração) e devolve o que parou, o que só existe de
//! um lado, e como arestas/pesos/ciclos mudaram entre os pares.
//!
//! ## O pareamento (e a honestidade dele)
//!
//! Pareia pelo **path do módulo normalizado na raiz do crate** — o 1º segmento
//! (nome do crate) é descartado, então `velho::nucleo::raio` pareia com
//! `novo::nucleo::raio` mesmo com o crate renomeado. O que **não** casa é
//! declarado **sem par dos dois lados**: um módulo movido (`a::b` → `c::b`)
//! normaliza para `a::b` vs `c::b` — **diferentes** — e aparece como sem-par
//! duas vezes, **não** como detectado. Detectar movidos por similaridade é
//! trilha futura; este dado **não finge** que ela existe (teste-contrato
//! [`tests::movido_e_sem_par_dos_dois_lados`]).
//!
//! **Sem nota única**: reporta conjuntos e contagens; o julgamento é do humano
//! (proposta §3, como o raio).

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use lente_core::entities::grafo::{Grafo, Kind, No, Path, Relation};
use lente_estrutura::{DependenciaModulo, EstruturaModulos};

/// Uma aresta módulo→módulo presente nos **dois** lados (entre pareados), com
/// o peso de cada lado — o delta de acoplamento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArestaComparada {
    /// Representante do lado **antes** (paths normalizam igual nos dois lados).
    pub de: Path,
    pub para: Path,
    pub peso_antes: usize,
    pub peso_depois: usize,
}

/// Resumo dos ciclos de um lado: quantidade de SCCs ≥ 2 e tamanho do maior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResumoCiclos {
    pub quantidade: usize,
    pub maior: usize,
}

/// Os dois lados de uma comparação (prompt 0074) — `Antes` (o projeto) e
/// `Depois` (a refatoração). Conceito do **domínio** da paridade; mora aqui (L1)
/// para a fiação (L4) identificar o lado que falhou sem declarar um enum no fio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lado {
    Antes,
    Depois,
}

/// A chave de pareamento (prompt 0075). A normalização (descartar o crate) só é
/// **injetiva com um crate por lado**; num grafo de workspace, dois crates podem
/// ter submódulos homônimos (`a::ast` e `b::ast` → `ast`), colidindo a chave. Por
/// isso o modo workspace usa o **path completo** (com o crate), injetivo por
/// construção.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChavePareamento {
    /// Descarta o 1º segmento (crate). Pareia crate renomeado. = 0074.
    Normalizada,
    /// Path completo (com o crate). Para lados-workspace.
    PathCompleto,
}

impl ChavePareamento {
    /// Texto estável publicado na saída (declaração do modo).
    pub fn texto(self) -> &'static str {
        match self {
            ChavePareamento::Normalizada => "normalizada",
            ChavePareamento::PathCompleto => "path_completo",
        }
    }
}

/// Proveniência de cada lado da comparação (prompt 0075), declarada na saída: o
/// **modo** (crate/workspace), o **nº de crates** e os **fantasmas** (do 0045 —
/// nós referenciados mas não materializados; sinal, não erro). É **dado**
/// fornecido pelo L4 (extração); o L1 só o transporta — o pareamento não o usa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proveniencia {
    pub modo_antes: String,
    pub modo_depois: String,
    pub crates_antes: usize,
    pub crates_depois: usize,
    pub fantasmas_antes: Vec<Path>,
    pub fantasmas_depois: Vec<Path>,
    /// Crates não extraídos por lado (prompt 0075): `nome — motivo`. Os módulos
    /// desses crates não entram na comparação; declarar é não mentir o censo.
    pub falhas_antes: Vec<String>,
    pub falhas_depois: Vec<String>,
    /// Nós de **third-party** removidos do censo por lado (prompt 0076): num
    /// lado-workspace em escopo seu-codigo, deps externas saem. Declarar o que
    /// some é não mascarar.
    pub third_party_antes: usize,
    pub third_party_depois: usize,
}

/// Um item pareado entre os dois lados (prompt 0078): a mesma chave K4 com
/// exatamente 1 candidato de cada lado. Carrega os **dois paths** — o consumidor
/// vê o movimento sem o produto inferir nada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemPareado {
    pub kind: String,
    pub trait_: String,
    pub nome_qualificado: String,
    pub path_antes: String,
    pub path_depois: String,
}

/// Uma chave K4 presente nos dois lados com >1 candidato em pelo menos um
/// (prompt 0078): o produto **não adivinha** a correspondência — declara os
/// candidatos de cada lado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemAmbiguo {
    pub kind: String,
    pub trait_: String,
    pub nome_qualificado: String,
    pub candidatos_antes: Vec<String>,
    pub candidatos_depois: Vec<String>,
}

/// Um item sem correspondente (prompt 0078): chave de um lado só.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSemPar {
    pub kind: String,
    pub trait_: String,
    pub nome_qualificado: String,
    pub path: String,
}

/// O nível de **item** da comparação (prompt 0078), por chave K4 `(kind, trait_,
/// pai-tipo::nome)` — independente de path. As quatro categorias do contrato
/// honesto do 0074, agora no nível de item.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ComparacaoItens {
    pub pareados: Vec<ItemPareado>,
    pub ambiguos: Vec<ItemAmbiguo>,
    pub sem_par_antes: Vec<ItemSemPar>,
    pub sem_par_depois: Vec<ItemSemPar>,
}

/// O resultado da comparação entre duas estruturas (prompt 0074). Tudo
/// determinístico (ordenado por path). É o contrato da tela lado a lado
/// (prompt seguinte) e do agente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparacao {
    pub nome_antes: String,
    pub nome_depois: String,
    /// Pares `(path_antes, path_depois)` casados pela normalização.
    pub pareados: Vec<(Path, Path)>,
    /// Módulos do lado antes sem correspondente.
    pub sem_par_antes: Vec<Path>,
    /// Módulos do lado depois sem correspondente.
    pub sem_par_depois: Vec<Path>,
    /// Arestas entre pareados presentes nos dois lados (com peso de cada).
    pub arestas_comuns: Vec<ArestaComparada>,
    /// Arestas entre pareados que **sumiram** (só no antes).
    pub arestas_so_antes: Vec<DependenciaModulo>,
    /// Arestas entre pareados que **apareceram** (só no depois).
    pub arestas_so_depois: Vec<DependenciaModulo>,
    pub ciclos_antes: ResumoCiclos,
    pub ciclos_depois: ResumoCiclos,
    /// Chave de pareamento usada (prompt 0075): `normalizada` ou `path_completo`.
    pub chave: String,
    /// Proveniência declarada dos dois lados (prompt 0075).
    pub proveniencia: Proveniencia,
    /// Nível de item (prompt 0078): pareados/ambíguos/sem-par por chave K4.
    pub itens: ComparacaoItens,
}

/// Aplica a [`ChavePareamento`] a um path: `Normalizada` descarta o 1º segmento
/// (crate) — raiz vira `""`; `PathCompleto` usa o path inteiro.
fn chave_de(p: &Path, chave: ChavePareamento) -> String {
    match chave {
        ChavePareamento::PathCompleto => p.as_str().to_string(),
        ChavePareamento::Normalizada => match p.as_str().split_once("::") {
            Some((_crate, resto)) => resto.to_string(),
            None => String::new(),
        },
    }
}

/// Compara duas estruturas extraídas com **os mesmos parâmetros** (escopo/modo —
/// garantido pelo L4), pela `chave` escolhida pelo L4 (path completo se algum
/// lado é workspace). `nome_*` são rótulos; `prov` é a proveniência a declarar.
pub fn comparar_estruturas(
    antes: &EstruturaModulos,
    depois: &EstruturaModulos,
    nome_antes: &str,
    nome_depois: &str,
    chave: ChavePareamento,
    proveniencia: Proveniencia,
    itens: ComparacaoItens,
) -> Comparacao {
    // chave → path, por lado. Com `Normalizada` e um crate por lado a chave é
    // injetiva; com `PathCompleto` é injetiva por construção (path único).
    let map_a: BTreeMap<String, &Path> =
        antes.modulos.iter().map(|p| (chave_de(p, chave), p)).collect();
    let map_b: BTreeMap<String, &Path> =
        depois.modulos.iter().map(|p| (chave_de(p, chave), p)).collect();

    let mut pareados = Vec::new();
    let mut sem_par_antes = Vec::new();
    for (norm, pa) in &map_a {
        match map_b.get(norm) {
            Some(pb) => pareados.push(((*pa).clone(), (*pb).clone())),
            None => sem_par_antes.push((*pa).clone()),
        }
    }
    let mut sem_par_depois: Vec<Path> = map_b
        .iter()
        .filter(|(norm, _)| !map_a.contains_key(*norm))
        .map(|(_, pb)| (*pb).clone())
        .collect();

    // Conjunto dos norms pareados — uma aresta só entra nos deltas se ambas as
    // pontas são módulos pareados (comparar arestas de módulos sem par mentiria).
    let pareado_norm: BTreeSet<String> = map_a
        .keys()
        .filter(|k| map_b.contains_key(*k))
        .cloned()
        .collect();

    let arestas_norm = |est: &EstruturaModulos| -> BTreeMap<(String, String), (Path, Path, usize)> {
        est.dependencias
            .iter()
            .filter_map(|d| {
                let nd = chave_de(&d.de, chave);
                let np = chave_de(&d.para, chave);
                if pareado_norm.contains(&nd) && pareado_norm.contains(&np) {
                    Some(((nd, np), (d.de.clone(), d.para.clone(), d.peso)))
                } else {
                    None
                }
            })
            .collect()
    };
    let ea = arestas_norm(antes);
    let eb = arestas_norm(depois);

    let mut arestas_comuns = Vec::new();
    let mut arestas_so_antes = Vec::new();
    for (k, (de, para, peso_a)) in &ea {
        match eb.get(k) {
            Some((_, _, peso_b)) => arestas_comuns.push(ArestaComparada {
                de: de.clone(),
                para: para.clone(),
                peso_antes: *peso_a,
                peso_depois: *peso_b,
            }),
            None => arestas_so_antes.push(DependenciaModulo {
                de: de.clone(),
                para: para.clone(),
                peso: *peso_a,
            }),
        }
    }
    let arestas_so_depois: Vec<DependenciaModulo> = eb
        .iter()
        .filter(|(k, _)| !ea.contains_key(*k))
        .map(|(_, (de, para, peso))| DependenciaModulo {
            de: de.clone(),
            para: para.clone(),
            peso: *peso,
        })
        .collect();

    // Determinismo final (os BTreeMap já ordenam por norm; reforçamos por path).
    pareados.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
    sem_par_antes.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    sem_par_depois.sort_by(|a, b| a.as_str().cmp(b.as_str()));

    Comparacao {
        nome_antes: nome_antes.to_string(),
        nome_depois: nome_depois.to_string(),
        pareados,
        sem_par_antes,
        sem_par_depois,
        arestas_comuns,
        arestas_so_antes,
        arestas_so_depois,
        ciclos_antes: resumo_ciclos(antes),
        ciclos_depois: resumo_ciclos(depois),
        chave: chave.texto().to_string(),
        proveniencia,
        itens,
    }
}

fn resumo_ciclos(est: &EstruturaModulos) -> ResumoCiclos {
    ResumoCiclos {
        quantidade: est.ciclos.len(),
        maior: est.ciclos.iter().map(|c| c.modulos.len()).max().unwrap_or(0),
    }
}

// ===========================================================================
// Nível de item (prompt 0078) — chave K4, promovida fiel da Arena 0077.
// ===========================================================================

/// Item = nó de definição nomeável; exclui `mod`/`crate` (nível módulo, 0076) e
/// `builtin` (primitivos). Transcrição fiel do `e_item` da Arena 0077.
fn e_item(k: Kind) -> bool {
    matches!(
        k,
        Kind::Fn
            | Kind::Struct
            | Kind::Union
            | Kind::Enum
            | Kind::Variant
            | Kind::Const
            | Kind::Static
            | Kind::Trait
            | Kind::Type
            | Kind::Macro
    )
}

fn e_tipo(k: Kind) -> bool {
    matches!(k, Kind::Struct | Kind::Enum | Kind::Union | Kind::Trait)
}

fn kind_txt(k: Kind) -> &'static str {
    match k {
        Kind::Crate => "crate",
        Kind::Mod => "mod",
        Kind::Fn => "fn",
        Kind::Struct => "struct",
        Kind::Union => "union",
        Kind::Enum => "enum",
        Kind::Variant => "variant",
        Kind::Const => "const",
        Kind::Static => "static",
        Kind::Trait => "trait",
        Kind::Type => "type",
        Kind::Builtin => "builtin",
        Kind::Macro => "macro",
    }
}

/// Mapa `child_id → parent_id` pela aresta `Owns` (id_from=pai, id_to=filho).
fn pais_owns(grafo: &Grafo) -> HashMap<usize, usize> {
    let mut m = HashMap::new();
    for a in &grafo.edges {
        if a.relation == Relation::Owns {
            m.insert(a.id_to, a.id_from);
        }
    }
    m
}

/// Nome qualificado por **pai-tipo** (`Counter::get`) quando o pai por `Owns` é
/// tipo; senão só o nome (qualificar por módulo reintroduziria a dependência de
/// path que o 0076 zerou). Transcrição fiel da Arena 0077.
fn nome_qualificado(no: &No, pais: &HashMap<usize, usize>, por_id: &HashMap<usize, &No>) -> String {
    if let Some(p) = pais.get(&no.id).and_then(|pid| por_id.get(pid)) {
        if e_tipo(p.kind) {
            return format!("{}::{}", p.name, no.name);
        }
    }
    no.name.clone()
}

/// Componentes da chave K4 de um item: `(kind, trait_, nome_qualificado)`.
fn chave_k4(no: &No, qual: &str) -> (String, String, String) {
    (
        kind_txt(no.kind).to_string(),
        no.trait_.clone().unwrap_or_default(),
        qual.to_string(),
    )
}

/// O censo de itens de um lado por chave K4 (prompt 0078): `chave → paths`.
/// Exclui `mod`/`crate`/`builtin` e os representantes de fantasma (paths em
/// `fantasmas`). Determinístico (`BTreeMap`, paths ordenados).
fn itens_por_chave(
    grafo: &Grafo,
    fantasmas: &BTreeSet<String>,
) -> BTreeMap<(String, String, String), Vec<String>> {
    let pais = pais_owns(grafo);
    let por_id: HashMap<usize, &No> = grafo.nodes.iter().map(|n| (n.id, n)).collect();
    let mut m: BTreeMap<(String, String, String), Vec<String>> = BTreeMap::new();
    for n in &grafo.nodes {
        if !e_item(n.kind) || fantasmas.contains(n.path.as_str()) {
            continue;
        }
        let qual = nome_qualificado(n, &pais, &por_id);
        m.entry(chave_k4(n, &qual))
            .or_default()
            .push(n.path.as_str().to_string());
    }
    for v in m.values_mut() {
        v.sort();
    }
    m
}

/// **Pareamento por identidade de item** (prompt 0078): agrupa os itens dos dois
/// lados pela chave K4 e categoriza em pareados (1:1) / ambíguos (>1) / sem-par.
/// Determinístico. Os grafos já vêm filtrados (sysroot + não-membros, 0076);
/// `fantasmas_*` são os paths dos representantes a excluir do censo.
pub fn comparar_itens(
    grafo_antes: &Grafo,
    fantasmas_antes: &BTreeSet<String>,
    grafo_depois: &Grafo,
    fantasmas_depois: &BTreeSet<String>,
) -> ComparacaoItens {
    let ma = itens_por_chave(grafo_antes, fantasmas_antes);
    let mb = itens_por_chave(grafo_depois, fantasmas_depois);

    let mut pareados = Vec::new();
    let mut ambiguos = Vec::new();
    let mut sem_par_antes = Vec::new();
    let mut sem_par_depois = Vec::new();

    // Itera as chaves dos dois lados em ordem (BTreeMap → determinístico).
    let chaves: BTreeSet<&(String, String, String)> = ma.keys().chain(mb.keys()).collect();
    for ch in chaves {
        let (kind, trait_, nome) = ch;
        let a = ma.get(ch);
        let b = mb.get(ch);
        match (a, b) {
            (Some(pa), Some(pb)) => {
                if pa.len() == 1 && pb.len() == 1 {
                    pareados.push(ItemPareado {
                        kind: kind.clone(),
                        trait_: trait_.clone(),
                        nome_qualificado: nome.clone(),
                        path_antes: pa[0].clone(),
                        path_depois: pb[0].clone(),
                    });
                } else {
                    ambiguos.push(ItemAmbiguo {
                        kind: kind.clone(),
                        trait_: trait_.clone(),
                        nome_qualificado: nome.clone(),
                        candidatos_antes: pa.clone(),
                        candidatos_depois: pb.clone(),
                    });
                }
            }
            (Some(pa), None) => {
                for path in pa {
                    sem_par_antes.push(ItemSemPar {
                        kind: kind.clone(),
                        trait_: trait_.clone(),
                        nome_qualificado: nome.clone(),
                        path: path.clone(),
                    });
                }
            }
            (None, Some(pb)) => {
                for path in pb {
                    sem_par_depois.push(ItemSemPar {
                        kind: kind.clone(),
                        trait_: trait_.clone(),
                        nome_qualificado: nome.clone(),
                        path: path.clone(),
                    });
                }
            }
            (None, None) => unreachable!(),
        }
    }

    ComparacaoItens {
        pareados,
        ambiguos,
        sem_par_antes,
        sem_par_depois,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_estrutura::Ciclo;

    fn est(modulos: &[&str], deps: &[(&str, &str, usize)], ciclos: &[&[&str]]) -> EstruturaModulos {
        EstruturaModulos {
            modulos: modulos.iter().map(|m| Path::from(*m)).collect(),
            dependencias: deps
                .iter()
                .map(|(de, para, peso)| DependenciaModulo {
                    de: Path::from(*de),
                    para: Path::from(*para),
                    peso: *peso,
                })
                .collect(),
            ciclos: ciclos
                .iter()
                .map(|c| Ciclo {
                    modulos: c.iter().map(|m| Path::from(*m)).collect(),
                })
                .collect(),
            ordem: modulos.iter().map(|m| Path::from(*m)).collect(),
            blocos: vec![],
            raios: vec![],
        }
    }

    fn prov() -> Proveniencia {
        Proveniencia {
            modo_antes: "crate".into(),
            modo_depois: "crate".into(),
            crates_antes: 1,
            crates_depois: 1,
            fantasmas_antes: vec![],
            fantasmas_depois: vec![],
            falhas_antes: vec![],
            falhas_depois: vec![],
            third_party_antes: 0,
            third_party_depois: 0,
        }
    }

    /// Helper dos testes do 0074: chave Normalizada (= comportamento original).
    fn cmp(
        a: &EstruturaModulos,
        b: &EstruturaModulos,
        na: &str,
        nb: &str,
    ) -> Comparacao {
        comparar_estruturas(a, b, na, nb, ChavePareamento::Normalizada, prov(), ComparacaoItens::default())
    }

    #[test]
    fn crate_renomeado_pareia_pela_normalizacao() {
        let a = est(&["velho", "velho::nucleo", "velho::nucleo::raio"], &[], &[]);
        let b = est(&["novo", "novo::nucleo", "novo::nucleo::raio"], &[], &[]);
        let c = cmp(&a, &b, "velho", "novo");
        assert_eq!(c.pareados.len(), 3, "tudo pareia apesar do crate renomeado");
        assert!(c.sem_par_antes.is_empty() && c.sem_par_depois.is_empty());
    }

    /// TESTE-CONTRATO: um módulo movido (`k::a::x` → `k::c::x`) normaliza para
    /// `a::x` vs `c::x` — diferentes. NÃO é detectado como movido; aparece como
    /// sem-par dos DOIS lados. Se alguém meter heurística de similaridade que
    /// "adivinhe", este teste grita.
    #[test]
    fn movido_e_sem_par_dos_dois_lados() {
        let a = est(&["k", "k::a", "k::a::x"], &[], &[]);
        let b = est(&["k", "k::c", "k::c::x"], &[], &[]);
        let c = cmp(&a, &b, "k", "k");
        // `k` pareia (raiz). `a::x`/`c::x` e `a`/`c` ficam sem par.
        assert_eq!(c.pareados, vec![(Path::from("k"), Path::from("k"))]);
        assert_eq!(
            c.sem_par_antes,
            vec![Path::from("k::a"), Path::from("k::a::x")]
        );
        assert_eq!(
            c.sem_par_depois,
            vec![Path::from("k::c"), Path::from("k::c::x")]
        );
    }

    #[test]
    fn delta_de_peso_e_arestas_que_mudam() {
        // antes: a→b peso 2, a→c peso 1. depois: a→b peso 5 (subiu), a→c sumiu,
        // b→c apareceu.
        let a = est(
            &["k", "k::a", "k::b", "k::c"],
            &[("k::a", "k::b", 2), ("k::a", "k::c", 1)],
            &[],
        );
        let b = est(
            &["k", "k::a", "k::b", "k::c"],
            &[("k::a", "k::b", 5), ("k::b", "k::c", 3)],
            &[],
        );
        let c = cmp(&a, &b, "k", "k");
        assert_eq!(c.arestas_comuns.len(), 1);
        assert_eq!(c.arestas_comuns[0].peso_antes, 2);
        assert_eq!(c.arestas_comuns[0].peso_depois, 5);
        assert_eq!(c.arestas_so_antes.len(), 1, "a→c sumiu");
        assert_eq!(c.arestas_so_antes[0].para, Path::from("k::c"));
        assert_eq!(c.arestas_so_depois.len(), 1, "b→c apareceu");
        assert_eq!(c.arestas_so_depois[0].de, Path::from("k::b"));
    }

    #[test]
    fn ciclo_desfeito_aparece_no_resumo() {
        let a = est(&["k", "k::a", "k::b"], &[], &[&["k::a", "k::b"]]);
        let b = est(&["k", "k::a", "k::b"], &[], &[]);
        let c = cmp(&a, &b, "k", "k");
        assert_eq!(c.ciclos_antes.quantidade, 1);
        assert_eq!(c.ciclos_antes.maior, 2);
        assert_eq!(c.ciclos_depois.quantidade, 0, "refatoração desfez o ciclo");
    }

    #[test]
    fn lado_vazio_nao_quebra() {
        let a = est(&["k", "k::a"], &[], &[]);
        let b = est(&[], &[], &[]);
        let c = cmp(&a, &b, "k", "vazio");
        assert!(c.pareados.is_empty());
        assert_eq!(c.sem_par_antes.len(), 2);
        assert!(c.sem_par_depois.is_empty());
    }

    // ---- prompt 0075: chave de path completo (modo workspace) ----

    /// TESTE-CONTRATO do 0075: dois crates `a` e `b` no lado antes, **ambos** com
    /// submódulo `ast`; o depois tem só `a::ast`. Com a chave **normalizada**
    /// (`ast`) os dois `::ast` colidiriam; com **path completo** cada um é uma
    /// chave própria — `a::ast` pareia, `b::ast` fica sem-par, sem colisão.
    #[test]
    fn path_completo_evita_colisao_de_submodulo_homonimo() {
        let antes = est(&["a", "a::ast", "b", "b::ast"], &[], &[]);
        let depois = est(&["a", "a::ast"], &[], &[]);
        let c = comparar_estruturas(
            &antes,
            &depois,
            "ws",
            "ws",
            ChavePareamento::PathCompleto,
            prov(),
            ComparacaoItens::default(),
        );
        assert_eq!(c.chave, "path_completo");
        let pares: Vec<&str> = c.pareados.iter().map(|(x, _)| x.as_str()).collect();
        assert!(pares.contains(&"a") && pares.contains(&"a::ast"));
        // b e b::ast NÃO casaram (sem colisão com a::ast).
        let sp: Vec<&str> = c.sem_par_antes.iter().map(|p| p.as_str()).collect();
        assert!(sp.contains(&"b") && sp.contains(&"b::ast"));
        assert_eq!(c.pareados.len(), 2);
    }

    /// Contraste: com a chave **normalizada**, `a::ast` e `b::ast` colapsam na
    /// mesma chave `ast` (um sobrescreve o outro no mapa) — exatamente a colisão
    /// que o modo workspace evita. Documenta por que a chave muda.
    #[test]
    fn normalizada_colide_submodulos_homonimos_de_crates_diferentes() {
        let antes = est(&["a::ast", "b::ast"], &[], &[]);
        let depois = est(&["a::ast"], &[], &[]);
        let c = comparar_estruturas(
            &antes,
            &depois,
            "ws",
            "ws",
            ChavePareamento::Normalizada,
            prov(),
            ComparacaoItens::default(),
        );
        // ambos viram "ast" → o mapa tem 1 chave; pareia 1, e o outro "ast"
        // soma some — a perda que justifica o path completo.
        assert_eq!(c.pareados.len(), 1);
    }

    // ---- comparar_itens (prompt 0078, chave K4) ----

    use lente_core::entities::grafo::{Aresta, Modificadores, No, Visibility};

    fn item(id: usize, path: &str, kind: Kind, trait_: Option<&str>) -> No {
        No {
            id,
            path: Path::from(path),
            name: path.rsplit("::").next().unwrap_or(path).to_string(),
            kind,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: path.split("::").next().unwrap_or("").to_string(),
            trait_: trait_.map(|s| s.to_string()),
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: None,
        }
    }

    fn owns(parent: usize, child: usize) -> Aresta {
        Aresta {
            from: Path::from("p"),
            id_from: parent,
            to: Path::from("c"),
            id_to: child,
            relation: Relation::Owns,
            uses_kind: None,
        }
    }

    fn grafo(nodes: Vec<No>, edges: Vec<Aresta>) -> Grafo {
        Grafo {
            crate_name: "x".to_string(),
            nodes,
            edges,
        }
    }

    fn vazio() -> BTreeSet<String> {
        BTreeSet::new()
    }

    /// `Counter` em `a::x` (antes) e em `b::y::z` (depois), pai-módulo: pareia pela
    /// chave (struct,"",Counter), carregando os dois paths.
    #[test]
    fn item_pareia_independente_de_path() {
        let ga = grafo(
            vec![item(1, "a::x", Kind::Mod, None), item(2, "a::x::Counter", Kind::Struct, None)],
            vec![owns(1, 2)],
        );
        let gb = grafo(
            vec![item(1, "b::y::z", Kind::Mod, None), item(2, "b::y::z::Counter", Kind::Struct, None)],
            vec![owns(1, 2)],
        );
        let r = comparar_itens(&ga, &vazio(), &gb, &vazio());
        assert_eq!(r.pareados.len(), 1);
        assert_eq!(r.pareados[0].nome_qualificado, "Counter");
        assert_eq!(r.pareados[0].path_antes, "a::x::Counter");
        assert_eq!(r.pareados[0].path_depois, "b::y::z::Counter");
        assert!(r.ambiguos.is_empty() && r.sem_par_antes.is_empty() && r.sem_par_depois.is_empty());
    }

    /// `get` sob `Counter` num lado e sob `Frame` no outro: qualificadores
    /// diferentes (Counter::get vs Frame::get) → não pareiam, cada um sem-par.
    #[test]
    fn pai_tipo_qualifica_e_separa() {
        let ga = grafo(
            vec![item(1, "k::Counter", Kind::Struct, None), item(2, "k::Counter::get", Kind::Fn, None)],
            vec![owns(1, 2)],
        );
        let gb = grafo(
            vec![item(1, "k::Frame", Kind::Struct, None), item(2, "k::Frame::get", Kind::Fn, None)],
            vec![owns(1, 2)],
        );
        let r = comparar_itens(&ga, &vazio(), &gb, &vazio());
        assert!(r.pareados.is_empty(), "nada pareia: {:?}", r.pareados);
        assert!(r.sem_par_antes.iter().any(|i| i.nome_qualificado == "Counter::get"));
        assert!(r.sem_par_depois.iter().any(|i| i.nome_qualificado == "Frame::get"));
    }

    /// `fmt`/Display e `fmt`/Debug sob o mesmo pai-tipo num lado; só Display no
    /// outro → Display pareia, Debug sem-par (o `trait_` separa).
    #[test]
    fn trait_separa_as_folhas() {
        let ga = grafo(
            vec![
                item(1, "k::T", Kind::Struct, None),
                item(2, "k::T::fmt", Kind::Fn, Some("Display")),
                item(3, "k::T::fmt", Kind::Fn, Some("Debug")),
            ],
            vec![owns(1, 2), owns(1, 3)],
        );
        let gb = grafo(
            vec![item(1, "k::T", Kind::Struct, None), item(2, "k::T::fmt", Kind::Fn, Some("Display"))],
            vec![owns(1, 2)],
        );
        let r = comparar_itens(&ga, &vazio(), &gb, &vazio());
        assert!(
            r.pareados.iter().any(|p| p.trait_ == "Display" && p.nome_qualificado == "T::fmt"),
            "Display pareia"
        );
        assert!(
            r.sem_par_antes.iter().any(|i| i.trait_ == "Debug" && i.nome_qualificado == "T::fmt"),
            "Debug sem-par antes"
        );
    }

    /// Chave com 2 itens no antes e 1 no depois → ambígua, 3 candidatos, nenhum
    /// pareado.
    #[test]
    fn dois_de_um_lado_e_ambiguo() {
        let ga = grafo(
            vec![
                item(1, "k::a", Kind::Mod, None),
                item(2, "k::a::Foo", Kind::Struct, None),
                item(3, "k::b", Kind::Mod, None),
                item(4, "k::b::Foo", Kind::Struct, None),
            ],
            vec![owns(1, 2), owns(3, 4)],
        );
        let gb = grafo(
            vec![item(1, "k::c", Kind::Mod, None), item(2, "k::c::Foo", Kind::Struct, None)],
            vec![owns(1, 2)],
        );
        let r = comparar_itens(&ga, &vazio(), &gb, &vazio());
        assert!(r.pareados.is_empty());
        assert_eq!(r.ambiguos.len(), 1);
        assert_eq!(r.ambiguos[0].candidatos_antes.len(), 2);
        assert_eq!(r.ambiguos[0].candidatos_depois.len(), 1);
    }

    /// Item cujo path está nos fantasmas do lado não entra no censo.
    #[test]
    fn fantasma_excluido_do_censo() {
        let ga = grafo(vec![item(1, "k::Foo", Kind::Struct, None)], vec![]);
        let gb = grafo(vec![item(1, "k::Foo", Kind::Struct, None)], vec![]);
        let mut fant_a = BTreeSet::new();
        fant_a.insert("k::Foo".to_string());
        let r = comparar_itens(&ga, &fant_a, &gb, &vazio());
        assert!(r.pareados.is_empty());
        assert_eq!(r.sem_par_depois.len(), 1);
        assert!(r.sem_par_antes.is_empty());
    }

    #[test]
    fn comparar_itens_e_deterministico() {
        let ga = grafo(
            vec![item(1, "k::a", Kind::Mod, None), item(2, "k::a::Foo", Kind::Struct, None)],
            vec![owns(1, 2)],
        );
        let gb = grafo(vec![item(1, "k::Bar", Kind::Struct, None)], vec![]);
        let r1 = comparar_itens(&ga, &vazio(), &gb, &vazio());
        let r2 = comparar_itens(&ga, &vazio(), &gb, &vazio());
        assert_eq!(r1, r2);
    }
}
