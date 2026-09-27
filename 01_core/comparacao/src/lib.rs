//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/comparacao.md
//! @prompt-hash bb97a6e1
//! @layer L1
//! @updated 2026-09-26
//! Camada:  L1 — Núcleo. Pureza: stdlib + `lente_core` + `lente_estrutura`.
//!
//! **Paridade como dado** (prompt 0074): compara **duas** [`EstruturaModulos`]
//! (antes/depois de uma refatoração) e devolve o que parou, o que só existe de
//! um lado, e como arestas/pesos/ciclos mudaram entre os pares.
//!
//! A API [`comparar_estruturas_por_identidade`] aceita chaves opacas fornecidas
//! pelo chamador, separadas dos paths apresentados. A descrição abaixo refere-se
//! ao pareamento legado por normalização Rust.
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

/// Regra mecânica de movimento: troca um prefixo com fronteira `::` e exige
/// que o sufixo restante exista literalmente no outro lado (P0079).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegraPrefixo {
    pub antes: String,
    pub depois: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelacaoDeclarada {
    UmParaUm,
    Divisao,
    Consolidacao,
    Grupo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlegacaoDeclarada {
    DeclaradaFechada,
    Parcial,
    Ausente,
    ForaDeEscopo,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrespondenciaDeclarada {
    pub id: String,
    pub antes: Vec<String>,
    pub depois: Vec<String>,
    pub relacao: RelacaoDeclarada,
    pub alegacao: AlegacaoDeclarada,
    pub evidencias: Vec<String>,
    pub nota: String,
}

/// Entrada pura já desserializada pelo L3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapaCorrespondencia {
    pub caminho: String,
    pub sha256: String,
    pub prefixos: Vec<RegraPrefixo>,
    pub correspondencias: Vec<CorrespondenciaDeclarada>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrespondenciaAplicada {
    pub id: String,
    pub antes: Vec<String>,
    pub depois: Vec<String>,
    pub relacao: RelacaoDeclarada,
    pub alegacao: AlegacaoDeclarada,
    pub evidencias: Vec<String>,
    pub nota: String,
    pub estado: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResultadoMapa {
    pub caminho: String,
    pub sha256: String,
    pub regras_prefixo: usize,
    pub prefixos_aplicados: Vec<ItemPareado>,
    pub correspondencias: Vec<CorrespondenciaAplicada>,
    pub diagnosticos: Vec<String>,
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
    pub mapa: ResultadoMapa,
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
    comparar_estruturas_com_mapa(
        antes,
        depois,
        nome_antes,
        nome_depois,
        chave,
        proveniencia,
        itens,
        None,
    )
}

// API pública legada: manter parâmetros evita quebra nos consumidores.
#[allow(clippy::too_many_arguments)]
pub fn comparar_estruturas_com_mapa(
    antes: &EstruturaModulos,
    depois: &EstruturaModulos,
    nome_antes: &str,
    nome_depois: &str,
    chave: ChavePareamento,
    proveniencia: Proveniencia,
    mut itens: ComparacaoItens,
    mapa: Option<&MapaCorrespondencia>,
) -> Comparacao {
    // chave → path, por lado. Com `Normalizada` e um crate por lado a chave é
    // injetiva; com `PathCompleto` é injetiva por construção (path único).
    let map_a: BTreeMap<String, &Path> = antes
        .modulos
        .iter()
        .map(|p| (chave_de(p, chave), p))
        .collect();
    let map_b: BTreeMap<String, &Path> = depois
        .modulos
        .iter()
        .map(|p| (chave_de(p, chave), p))
        .collect();

    let mut pareados = Vec::new();
    let mut destino_usado = BTreeSet::new();
    let mut sem_par_antes = Vec::new();
    for (norm, pa) in &map_a {
        let por_prefixo: BTreeSet<&Path> = mapa
            .into_iter()
            .flat_map(|m| &m.prefixos)
            .filter_map(|r| troca_prefixo(pa.as_str(), &r.antes, &r.depois))
            .filter_map(|p| depois.modulos.iter().find(|m| m.as_str() == p))
            .collect();
        let pb = if por_prefixo.len() == 1 {
            por_prefixo.iter().next().copied()
        } else {
            map_b.get(norm).copied()
        };
        match pb {
            Some(pb) if destino_usado.insert(pb.as_str().to_string()) => {
                pareados.push(((*pa).clone(), pb.clone()))
            }
            _ => sem_par_antes.push((*pa).clone()),
        }
    }
    let mut sem_par_depois: Vec<Path> = depois
        .modulos
        .iter()
        .filter(|pb| !destino_usado.contains(pb.as_str()))
        .cloned()
        .collect();
    if let Some(m) = mapa {
        let mut modulos_prefixo: Vec<ItemPareado> = pareados
            .iter()
            .filter(|(a, b)| {
                m.prefixos.iter().any(|r| {
                    troca_prefixo(a.as_str(), &r.antes, &r.depois).as_deref() == Some(b.as_str())
                })
            })
            .map(|(a, b)| ItemPareado {
                kind: "mod".into(),
                trait_: String::new(),
                nome_qualificado: a.as_str().rsplit("::").next().unwrap_or(a.as_str()).into(),
                path_antes: a.as_str().into(),
                path_depois: b.as_str().into(),
            })
            .collect();
        modulos_prefixo.sort_by(|a, b| a.path_antes.cmp(&b.path_antes));
        itens.mapa.prefixos_aplicados.splice(0..0, modulos_prefixo);
    }

    let a_para_b: BTreeMap<String, String> = pareados
        .iter()
        .map(|(a, b)| (a.as_str().to_string(), b.as_str().to_string()))
        .collect();
    let b_para_a: BTreeMap<String, String> = pareados
        .iter()
        .map(|(a, b)| (b.as_str().to_string(), a.as_str().to_string()))
        .collect();
    let arestas_a = |est: &EstruturaModulos| -> BTreeMap<(String, String), (Path, Path, usize)> {
        est.dependencias
            .iter()
            .filter_map(|d| {
                a_para_b.get(d.de.as_str())?;
                a_para_b.get(d.para.as_str())?;
                Some((
                    (d.de.as_str().to_string(), d.para.as_str().to_string()),
                    (d.de.clone(), d.para.clone(), d.peso),
                ))
            })
            .collect()
    };
    let arestas_b = |est: &EstruturaModulos| -> BTreeMap<(String, String), (Path, Path, usize)> {
        est.dependencias
            .iter()
            .filter_map(|d| {
                let de = b_para_a.get(d.de.as_str())?.clone();
                let para = b_para_a.get(d.para.as_str())?.clone();
                Some(((de, para), (d.de.clone(), d.para.clone(), d.peso)))
            })
            .collect()
    };
    let ea = arestas_a(antes);
    let eb = arestas_b(depois);

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
        maior: est
            .ciclos
            .iter()
            .map(|c| c.modulos.len())
            .max()
            .unwrap_or(0),
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
    if let Some(p) = pais.get(&no.id).and_then(|pid| por_id.get(pid))
        && e_tipo(p.kind)
    {
        return format!("{}::{}", p.name, no.name);
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
        mapa: ResultadoMapa::default(),
    }
}

fn troca_prefixo(path: &str, antes: &str, depois: &str) -> Option<String> {
    if path == antes {
        return Some(depois.to_string());
    }
    path.strip_prefix(antes)
        .and_then(|r| r.strip_prefix("::"))
        .map(|sufixo| format!("{depois}::{sufixo}"))
}

fn cardinalidade_valida(c: &CorrespondenciaDeclarada) -> bool {
    match c.relacao {
        RelacaoDeclarada::UmParaUm => c.antes.len() == 1 && c.depois.len() == 1,
        RelacaoDeclarada::Divisao => c.antes.len() == 1 && c.depois.len() > 1,
        RelacaoDeclarada::Consolidacao => c.antes.len() > 1 && c.depois.len() == 1,
        RelacaoDeclarada::Grupo => !c.antes.is_empty() && !c.depois.is_empty(),
    }
}

/// Aplica o mapa sobre o censo já calculado. Explicitamente declarados são
/// validados contra os grafos completos; erro preserva `unknown` e não consome
/// o resíduo. O retorno continua determinístico independentemente da ordem.
pub fn aplicar_mapa(
    itens: &mut ComparacaoItens,
    mapa: &MapaCorrespondencia,
    grafo_antes: &Grafo,
    grafo_depois: &Grafo,
) {
    let paths_a: BTreeSet<String> = grafo_antes
        .nodes
        .iter()
        .map(|n| n.path.as_str().to_string())
        .collect();
    let paths_b: BTreeSet<String> = grafo_depois
        .nodes
        .iter()
        .map(|n| n.path.as_str().to_string())
        .collect();
    let mut usados_a = BTreeSet::new();
    let mut usados_b = BTreeSet::new();
    let mut declaradas = mapa.correspondencias.clone();
    declaradas.sort_by(|a, b| a.id.cmp(&b.id));
    let mut cont_a: BTreeMap<String, usize> = BTreeMap::new();
    let mut cont_b: BTreeMap<String, usize> = BTreeMap::new();
    let mut cont_id: BTreeMap<String, usize> = BTreeMap::new();
    for c in &declaradas {
        *cont_id.entry(c.id.clone()).or_default() += 1;
        for p in &c.antes {
            *cont_a.entry(p.clone()).or_default() += 1;
        }
        for p in &c.depois {
            *cont_b.entry(p.clone()).or_default() += 1;
        }
    }
    itens.mapa = ResultadoMapa {
        caminho: mapa.caminho.clone(),
        sha256: mapa.sha256.clone(),
        regras_prefixo: mapa.prefixos.len(),
        ..ResultadoMapa::default()
    };

    for c in declaradas {
        let mut erros = Vec::new();
        if cont_id.get(&c.id).copied().unwrap_or(0) > 1 {
            erros.push("id duplicado".to_string());
        }
        if !cardinalidade_valida(&c) {
            erros.push("cardinalidade incompatível".to_string());
        }
        for p in &c.antes {
            if !paths_a.contains(p) {
                erros.push(format!("path antes inexistente: {p}"));
            }
            if cont_a.get(p).copied().unwrap_or(0) > 1 {
                erros.push(format!("path antes em mais de uma relação: {p}"));
            }
        }
        for p in &c.depois {
            if !paths_b.contains(p) {
                erros.push(format!("path depois inexistente: {p}"));
            }
            if cont_b.get(p).copied().unwrap_or(0) > 1 {
                erros.push(format!("path depois em mais de uma relação: {p}"));
            }
        }
        let estado = if erros.is_empty() {
            "aplicada"
        } else {
            "unknown"
        };
        if erros.is_empty() {
            usados_a.extend(c.antes.iter().cloned());
            usados_b.extend(c.depois.iter().cloned());
        } else {
            itens
                .mapa
                .diagnosticos
                .extend(erros.into_iter().map(|e| format!("{}: {e}", c.id)));
        }
        itens.mapa.correspondencias.push(CorrespondenciaAplicada {
            id: c.id,
            antes: c.antes,
            depois: c.depois,
            relacao: c.relacao,
            alegacao: c.alegacao,
            evidencias: c.evidencias,
            nota: c.nota,
            estado: estado.to_string(),
        });
    }

    // Correspondências explícitas válidas precedem o automático.
    itens
        .pareados
        .retain(|p| !usados_a.contains(&p.path_antes) && !usados_b.contains(&p.path_depois));
    itens.ambiguos.retain(|a| {
        !a.candidatos_antes.iter().any(|p| usados_a.contains(p))
            && !a.candidatos_depois.iter().any(|p| usados_b.contains(p))
    });
    itens.sem_par_antes.retain(|i| !usados_a.contains(&i.path));
    itens.sem_par_depois.retain(|i| !usados_b.contains(&i.path));

    // Prefixos só atuam no resíduo e nunca por similaridade.
    let mut regras = mapa.prefixos.clone();
    regras.sort_by(|a, b| (&a.antes, &a.depois).cmp(&(&b.antes, &b.depois)));
    let mut pares = Vec::new();
    for a in &itens.sem_par_antes {
        let candidatos: BTreeSet<String> = regras
            .iter()
            .filter_map(|r| troca_prefixo(&a.path, &r.antes, &r.depois))
            .filter(|p| itens.sem_par_depois.iter().any(|b| &b.path == p))
            .collect();
        if candidatos.len() == 1 {
            let b = candidatos.iter().next().unwrap();
            pares.push(ItemPareado {
                kind: a.kind.clone(),
                trait_: a.trait_.clone(),
                nome_qualificado: a.nome_qualificado.clone(),
                path_antes: a.path.clone(),
                path_depois: b.clone(),
            });
        } else if candidatos.len() > 1 {
            itens.mapa.diagnosticos.push(format!(
                "prefixo ambíguo para {}: {} destinos",
                a.path,
                candidatos.len()
            ));
        }
    }
    let pa: BTreeSet<String> = pares.iter().map(|p| p.path_antes.clone()).collect();
    let pb: BTreeSet<String> = pares.iter().map(|p| p.path_depois.clone()).collect();
    itens.sem_par_antes.retain(|i| !pa.contains(&i.path));
    itens.sem_par_depois.retain(|i| !pb.contains(&i.path));
    pares.sort_by(|a, b| a.path_antes.cmp(&b.path_antes));
    itens.mapa.prefixos_aplicados = pares;
}

/// Um par declarado pelo chamador. A identidade é opaca; os paths são originais.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParIdentidade {
    pub identidade: String,
    pub path_antes: Path,
    pub path_depois: Path,
}

/// Dependência entre pares inequívocos. `None` significa aresta ausente naquele
/// lado; `Some(0)` continua sendo uma aresta presente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArestaPorIdentidade {
    pub de: ParIdentidade,
    pub para: ParIdentidade,
    pub peso_antes: Option<usize>,
    pub peso_depois: Option<usize>,
}

/// Problemas de cobertura ou unicidade dos mapas fornecidos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticoIdentidade {
    Ausente {
        lado: Lado,
        path: Path,
    },
    /// Todos os candidatos permanecem sem par, inclusive no lado não ambíguo.
    Repetida {
        identidade: String,
        antes: Vec<Path>,
        depois: Vec<Path>,
    },
    /// Entrada do mapa que não identifica um módulo desta estrutura.
    PathNaoListado {
        lado: Lado,
        path: Path,
    },
}

/// Comparação estrutural por identidade declarada, sem inferir equivalência
/// comportamental. Arestas incidentes a módulos sem par não são comparadas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparacaoIdentidades {
    pub pareados: Vec<ParIdentidade>,
    pub sem_par_antes: Vec<Path>,
    pub sem_par_depois: Vec<Path>,
    pub arestas: Vec<ArestaPorIdentidade>,
    pub diagnosticos: Vec<DiagnosticoIdentidade>,
    /// Resumos da estrutura inteira, incluindo módulos sem par.
    pub ciclos_antes: ResumoCiclos,
    pub ciclos_depois: ResumoCiclos,
}

/// Compara módulos usando mapas `path original -> identidade opaca` por lado.
///
/// O path apenas endereça o módulo dentro de cada entrada (paths únicos da
/// forma resolvida). Nenhum separador, prefixo, caixa ou Unicode é normalizado;
/// inclusive a chave vazia é válida. IDs locais da extração não são consultados
/// nem constituem identidade persistente. O chamador fornece a política de
/// identidade entre versões e garante escopo e granularidade comparáveis.
///
/// Só pareia uma chave com exatamente um candidato em cada lado. Ausências e
/// duplicidades geram diagnósticos; entradas excedentes no mapa são diagnosticadas
/// e ignoradas. Saídas são ordenadas por identidade, ou por path nos resíduos.
/// Dependências seguem o contrato de `EstruturaModulos`: um peso agregado por
/// par dirigido de módulos. As APIs legadas continuam independentes desta API.
///
/// Exemplo inteiramente em memória:
/// ```
/// use std::collections::BTreeMap;
/// use lente_core::entities::grafo::Path;
/// use lente_estrutura::EstruturaModulos;
/// use lente_comparacao::comparar_estruturas_por_identidade;
/// let antes = EstruturaModulos {
///     modulos: vec![Path::from("src/antigo.py")], dependencias: vec![],
///     ciclos: vec![], ordem: vec![], blocos: vec![], raios: vec![],
/// };
/// let depois = EstruturaModulos {
///     modulos: vec![Path::from("novo.modulo")], ..antes.clone()
/// };
/// let ids_a = BTreeMap::from([(Path::from("src/antigo.py"), "componente-α".into())]);
/// let ids_b = BTreeMap::from([(Path::from("novo.modulo"), "componente-α".into())]);
/// let resultado = comparar_estruturas_por_identidade(&antes, &depois, &ids_a, &ids_b);
/// assert_eq!(resultado.pareados[0].path_antes.as_str(), "src/antigo.py");
/// assert_eq!(resultado.pareados[0].path_depois.as_str(), "novo.modulo");
/// assert!(resultado.diagnosticos.is_empty());
/// ```
pub fn comparar_estruturas_por_identidade(
    antes: &EstruturaModulos,
    depois: &EstruturaModulos,
    identidades_antes: &BTreeMap<Path, String>,
    identidades_depois: &BTreeMap<Path, String>,
) -> ComparacaoIdentidades {
    let mut diagnosticos = Vec::new();
    let mut grupos: BTreeMap<String, (Vec<Path>, Vec<Path>)> = BTreeMap::new();
    let mut sem_par_antes: BTreeSet<Path> = antes.modulos.iter().cloned().collect();
    let mut sem_par_depois: BTreeSet<Path> = depois.modulos.iter().cloned().collect();
    for (lado, paths, mapa) in [
        (Lado::Antes, &sem_par_antes, identidades_antes),
        (Lado::Depois, &sem_par_depois, identidades_depois),
    ] {
        for path in paths {
            if let Some(identidade) = mapa.get(path) {
                let grupo = grupos.entry(identidade.clone()).or_default();
                match lado {
                    Lado::Antes => grupo.0.push(path.clone()),
                    Lado::Depois => grupo.1.push(path.clone()),
                }
            } else {
                diagnosticos.push(DiagnosticoIdentidade::Ausente {
                    lado,
                    path: path.clone(),
                });
            }
        }
        for path in mapa.keys().filter(|path| !paths.contains(*path)) {
            diagnosticos.push(DiagnosticoIdentidade::PathNaoListado {
                lado,
                path: path.clone(),
            });
        }
    }
    let mut pareados = Vec::new();
    for (identidade, (a, b)) in grupos {
        if a.len() > 1 || b.len() > 1 {
            diagnosticos.push(DiagnosticoIdentidade::Repetida {
                identidade,
                antes: a,
                depois: b,
            });
        } else if let ([path_antes], [path_depois]) = (a.as_slice(), b.as_slice()) {
            sem_par_antes.remove(path_antes);
            sem_par_depois.remove(path_depois);
            pareados.push(ParIdentidade {
                identidade,
                path_antes: path_antes.clone(),
                path_depois: path_depois.clone(),
            });
        }
    }
    let mut pesos: BTreeMap<(usize, usize), (Option<usize>, Option<usize>)> = BTreeMap::new();
    for (lado, estrutura) in [(Lado::Antes, antes), (Lado::Depois, depois)] {
        let indices: BTreeMap<&Path, usize> = pareados
            .iter()
            .enumerate()
            .map(|(i, par)| {
                (
                    match lado {
                        Lado::Antes => &par.path_antes,
                        Lado::Depois => &par.path_depois,
                    },
                    i,
                )
            })
            .collect();
        for dep in &estrutura.dependencias {
            if let (Some(&de), Some(&para)) = (indices.get(&dep.de), indices.get(&dep.para)) {
                let peso = pesos.entry((de, para)).or_default();
                match lado {
                    Lado::Antes => peso.0 = Some(dep.peso),
                    Lado::Depois => peso.1 = Some(dep.peso),
                }
            }
        }
    }
    let arestas = pesos
        .into_iter()
        .map(
            |((de, para), (peso_antes, peso_depois))| ArestaPorIdentidade {
                de: pareados[de].clone(),
                para: pareados[para].clone(),
                peso_antes,
                peso_depois,
            },
        )
        .collect();
    ComparacaoIdentidades {
        pareados,
        sem_par_antes: sem_par_antes.into_iter().collect(),
        sem_par_depois: sem_par_depois.into_iter().collect(),
        arestas,
        diagnosticos,
        ciclos_antes: resumo_ciclos(antes),
        ciclos_depois: resumo_ciclos(depois),
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
    fn cmp(a: &EstruturaModulos, b: &EstruturaModulos, na: &str, nb: &str) -> Comparacao {
        comparar_estruturas(
            a,
            b,
            na,
            nb,
            ChavePareamento::Normalizada,
            prov(),
            ComparacaoItens::default(),
        )
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
            vec![
                item(1, "a::x", Kind::Mod, None),
                item(2, "a::x::Counter", Kind::Struct, None),
            ],
            vec![owns(1, 2)],
        );
        let gb = grafo(
            vec![
                item(1, "b::y::z", Kind::Mod, None),
                item(2, "b::y::z::Counter", Kind::Struct, None),
            ],
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
            vec![
                item(1, "k::Counter", Kind::Struct, None),
                item(2, "k::Counter::get", Kind::Fn, None),
            ],
            vec![owns(1, 2)],
        );
        let gb = grafo(
            vec![
                item(1, "k::Frame", Kind::Struct, None),
                item(2, "k::Frame::get", Kind::Fn, None),
            ],
            vec![owns(1, 2)],
        );
        let r = comparar_itens(&ga, &vazio(), &gb, &vazio());
        assert!(r.pareados.is_empty(), "nada pareia: {:?}", r.pareados);
        assert!(
            r.sem_par_antes
                .iter()
                .any(|i| i.nome_qualificado == "Counter::get")
        );
        assert!(
            r.sem_par_depois
                .iter()
                .any(|i| i.nome_qualificado == "Frame::get")
        );
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
            vec![
                item(1, "k::T", Kind::Struct, None),
                item(2, "k::T::fmt", Kind::Fn, Some("Display")),
            ],
            vec![owns(1, 2)],
        );
        let r = comparar_itens(&ga, &vazio(), &gb, &vazio());
        assert!(
            r.pareados
                .iter()
                .any(|p| p.trait_ == "Display" && p.nome_qualificado == "T::fmt"),
            "Display pareia"
        );
        assert!(
            r.sem_par_antes
                .iter()
                .any(|i| i.trait_ == "Debug" && i.nome_qualificado == "T::fmt"),
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
            vec![
                item(1, "k::c", Kind::Mod, None),
                item(2, "k::c::Foo", Kind::Struct, None),
            ],
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
            vec![
                item(1, "k::a", Kind::Mod, None),
                item(2, "k::a::Foo", Kind::Struct, None),
            ],
            vec![owns(1, 2)],
        );
        let gb = grafo(vec![item(1, "k::Bar", Kind::Struct, None)], vec![]);
        let r1 = comparar_itens(&ga, &vazio(), &gb, &vazio());
        let r2 = comparar_itens(&ga, &vazio(), &gb, &vazio());
        assert_eq!(r1, r2);
    }

    fn mapa(
        prefixos: Vec<RegraPrefixo>,
        correspondencias: Vec<CorrespondenciaDeclarada>,
    ) -> MapaCorrespondencia {
        MapaCorrespondencia {
            caminho: "mapa.toml".into(),
            sha256: "abc".into(),
            prefixos,
            correspondencias,
        }
    }

    #[test]
    fn prefixo_pareia_apenas_com_fronteira_de_segmento() {
        let a = est(&["old::ast", "old::ast::alpha", "old::asterisco"], &[], &[]);
        let b = est(
            &[
                "new::entities::ast",
                "new::entities::ast::alpha",
                "new::entities::asterisco",
            ],
            &[],
            &[],
        );
        let m = mapa(
            vec![RegraPrefixo {
                antes: "old::ast".into(),
                depois: "new::entities::ast".into(),
            }],
            vec![],
        );
        let r = comparar_estruturas_com_mapa(
            &a,
            &b,
            "a",
            "b",
            ChavePareamento::PathCompleto,
            prov(),
            ComparacaoItens::default(),
            Some(&m),
        );
        assert_eq!(r.pareados.len(), 2);
        assert!(
            r.pareados
                .iter()
                .any(|(x, y)| x.as_str() == "old::ast::alpha"
                    && y.as_str() == "new::entities::ast::alpha")
        );
        assert!(
            r.sem_par_antes
                .iter()
                .any(|p| p.as_str() == "old::asterisco")
        );
    }

    #[test]
    fn divisao_valida_consumo_residuo_e_preserva_alegacao() {
        let ga = grafo(vec![item(1, "a::apply", Kind::Fn, None)], vec![]);
        let gb = grafo(
            vec![
                item(1, "b::arith", Kind::Fn, None),
                item(2, "b::equal", Kind::Fn, None),
            ],
            vec![],
        );
        let c = CorrespondenciaDeclarada {
            id: "apply".into(),
            antes: vec!["a::apply".into()],
            depois: vec!["b::arith".into(), "b::equal".into()],
            relacao: RelacaoDeclarada::Divisao,
            alegacao: AlegacaoDeclarada::DeclaradaFechada,
            evidencias: vec!["e.md".into()],
            nota: "doc".into(),
        };
        let mut r = comparar_itens(&ga, &vazio(), &gb, &vazio());
        aplicar_mapa(&mut r, &mapa(vec![], vec![c]), &ga, &gb);
        assert!(r.sem_par_antes.is_empty() && r.sem_par_depois.is_empty());
        assert_eq!(r.mapa.correspondencias[0].estado, "aplicada");
        assert_eq!(
            r.mapa.correspondencias[0].alegacao,
            AlegacaoDeclarada::DeclaradaFechada
        );
    }

    #[test]
    fn cardinalidade_invalida_e_path_inexistente_ficam_unknown_sem_consumir() {
        let ga = grafo(vec![item(1, "a::x", Kind::Fn, None)], vec![]);
        let gb = grafo(vec![item(1, "b::y", Kind::Fn, None)], vec![]);
        let c = CorrespondenciaDeclarada {
            id: "ruim".into(),
            antes: vec!["a::x".into()],
            depois: vec!["b::y".into(), "b::z".into()],
            relacao: RelacaoDeclarada::UmParaUm,
            alegacao: AlegacaoDeclarada::Unknown,
            evidencias: vec![],
            nota: String::new(),
        };
        let mut r = comparar_itens(&ga, &vazio(), &gb, &vazio());
        aplicar_mapa(&mut r, &mapa(vec![], vec![c]), &ga, &gb);
        assert_eq!(r.mapa.correspondencias[0].estado, "unknown");
        assert!(!r.mapa.diagnosticos.is_empty());
        assert_eq!(r.sem_par_antes.len(), 1);
    }

    #[test]
    fn conflito_nao_e_resolvido_pela_ordem() {
        let ga = grafo(vec![item(1, "a::x", Kind::Fn, None)], vec![]);
        let gb = grafo(
            vec![
                item(1, "b::y", Kind::Fn, None),
                item(2, "b::z", Kind::Fn, None),
            ],
            vec![],
        );
        let mk = |id: &str, d: &str| CorrespondenciaDeclarada {
            id: id.into(),
            antes: vec!["a::x".into()],
            depois: vec![d.into()],
            relacao: RelacaoDeclarada::UmParaUm,
            alegacao: AlegacaoDeclarada::Parcial,
            evidencias: vec![],
            nota: String::new(),
        };
        let mut r = comparar_itens(&ga, &vazio(), &gb, &vazio());
        aplicar_mapa(
            &mut r,
            &mapa(vec![], vec![mk("a", "b::y"), mk("b", "b::z")]),
            &ga,
            &gb,
        );
        assert_eq!(
            r.mapa
                .correspondencias
                .iter()
                .filter(|c| c.estado == "unknown")
                .count(),
            2
        );
        assert!(
            r.mapa
                .diagnosticos
                .iter()
                .any(|d| d.contains("mais de uma relação"))
        );
    }

    fn ids(pares: &[(&str, &str)]) -> BTreeMap<Path, String> {
        pares
            .iter()
            .map(|(p, k)| (Path::from(*p), (*k).into()))
            .collect()
    }

    #[test]
    fn identidade_preserva_paths_e_compara_dependencias() {
        let a = est(
            &["src/a.py", "lib/b.hpp"],
            &[("src/a.py", "lib/b.hpp", 2), ("lib/b.hpp", "src/a.py", 0)],
            &[&["src/a.py", "lib/b.hpp"]],
        );
        let b = est(
            &["pacote.alpha", "novo::β"],
            &[("pacote.alpha", "novo::β", 5), ("novo::β", "novo::β", 1)],
            &[],
        );
        let c = comparar_estruturas_por_identidade(
            &a,
            &b,
            &ids(&[("src/a.py", "A"), ("lib/b.hpp", "B")]),
            &ids(&[("pacote.alpha", "A"), ("novo::β", "B")]),
        );
        assert_eq!(
            c.pareados,
            vec![
                ParIdentidade {
                    identidade: "A".into(),
                    path_antes: "src/a.py".into(),
                    path_depois: "pacote.alpha".into()
                },
                ParIdentidade {
                    identidade: "B".into(),
                    path_antes: "lib/b.hpp".into(),
                    path_depois: "novo::β".into()
                }
            ]
        );
        assert_eq!(
            c.arestas
                .iter()
                .map(|e| (e.peso_antes, e.peso_depois))
                .collect::<Vec<_>>(),
            vec![(Some(2), Some(5)), (Some(0), None), (None, Some(1))]
        );
        assert_eq!(c.arestas[0].de, c.pareados[0]);
        assert_eq!(c.arestas[0].para, c.pareados[1]);
        assert!(c.diagnosticos.is_empty());
        assert_eq!(c.ciclos_antes.quantidade, 1);
        assert_eq!(c.ciclos_depois.quantidade, 0);
    }

    #[test]
    fn identidade_diferente_nao_pareia_paths_iguais_ou_semelhantes() {
        let a = est(&["a::x", "igual"], &[], &[]);
        let b = est(&["b::x", "igual"], &[], &[]);
        let c = comparar_estruturas_por_identidade(
            &a,
            &b,
            &ids(&[("a::x", "1"), ("igual", "2")]),
            &ids(&[("b::x", "3"), ("igual", "4")]),
        );
        assert!(c.pareados.is_empty());
        assert_eq!(c.sem_par_antes.len(), 2);
        assert_eq!(c.sem_par_depois.len(), 2);
        assert!(c.diagnosticos.is_empty());
    }

    #[test]
    fn identidades_sao_opacas_inclusive_unicode_e_vazio() {
        let nomes = ["/", ".", "::", "ação/模块::β.x", "", "é", "e\u{301}"];
        let a = est(&nomes, &[], &[]);
        let mapa = ids(&nomes.iter().map(|n| (*n, *n)).collect::<Vec<_>>());
        let c = comparar_estruturas_por_identidade(&a, &a, &mapa, &mapa);
        assert_eq!(c.pareados.len(), nomes.len());
        assert!(c.diagnosticos.is_empty());
        for par in c.pareados {
            assert_eq!(par.identidade, par.path_antes.as_str());
        }
    }

    #[test]
    fn identidade_repetida_em_qualquer_lado_bloqueia_todos_os_candidatos() {
        for (na, nb) in [(2, 1), (1, 2), (2, 2), (2, 0), (0, 2)] {
            let a = est(&["a", "b"][..na], &[], &[]);
            let b = est(&["c", "d"][..nb], &[], &[]);
            let ma = a
                .modulos
                .iter()
                .cloned()
                .map(|p| (p, "id".into()))
                .collect();
            let mb = b
                .modulos
                .iter()
                .cloned()
                .map(|p| (p, "id".into()))
                .collect();
            let c = comparar_estruturas_por_identidade(&a, &b, &ma, &mb);
            assert!(c.pareados.is_empty());
            assert_eq!(c.sem_par_antes, a.modulos);
            assert_eq!(c.sem_par_depois, b.modulos);
            assert_eq!(
                c.diagnosticos,
                vec![DiagnosticoIdentidade::Repetida {
                    identidade: "id".into(),
                    antes: a.modulos,
                    depois: b.modulos
                }]
            );
        }
    }

    #[test]
    fn ausencias_nao_usam_path_como_fallback_e_extras_sao_diagnosticados() {
        let a = est(&["a", "b"], &[("a", "b", 1)], &[]);
        let c = comparar_estruturas_por_identidade(
            &a,
            &a,
            &ids(&[("a", "ok"), ("fora", "ok")]),
            &ids(&[("a", "ok")]),
        );
        assert_eq!(c.pareados.len(), 1);
        assert!(c.arestas.is_empty());
        assert_eq!(c.sem_par_antes, vec![Path::from("b")]);
        assert_eq!(c.sem_par_depois, vec![Path::from("b")]);
        assert_eq!(
            c.diagnosticos,
            vec![
                DiagnosticoIdentidade::Ausente {
                    lado: Lado::Antes,
                    path: "b".into()
                },
                DiagnosticoIdentidade::PathNaoListado {
                    lado: Lado::Antes,
                    path: "fora".into()
                },
                DiagnosticoIdentidade::Ausente {
                    lado: Lado::Depois,
                    path: "b".into()
                }
            ]
        );
    }

    #[test]
    fn permutar_estruturas_preserva_todo_resultado_por_identidade() {
        let mut a = est(
            &["a", "b", "c", "d", "e"],
            &[("a", "b", 2), ("b", "a", 3), ("c", "a", 1)],
            &[],
        );
        let mut b = est(
            &["v", "w", "x", "y", "z"],
            &[("v", "w", 4), ("w", "v", 1)],
            &[],
        );
        let ma = ids(&[("a", "1"), ("b", "2"), ("c", "dup"), ("d", "dup")]);
        let mb = ids(&[("v", "1"), ("w", "2"), ("x", "dup"), ("y", "dup")]);
        let esperado = comparar_estruturas_por_identidade(&a, &b, &ma, &mb);
        for _ in 0..5 {
            a.modulos.rotate_left(1);
            b.modulos.reverse();
            a.dependencias.reverse();
            b.dependencias.reverse();
            assert_eq!(
                comparar_estruturas_por_identidade(&a, &b, &ma, &mb),
                esperado
            );
        }
    }
}
