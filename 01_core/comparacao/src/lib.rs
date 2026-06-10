//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/comparacao.md
//! @prompt-hash eba86950
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

use std::collections::{BTreeMap, BTreeSet};

use lente_core::entities::grafo::Path;
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
}

/// Normaliza um path de módulo descartando o 1º segmento (nome do crate). O
/// módulo-raiz do crate (1 segmento) normaliza para `""` — os dois raízes
/// pareiam entre si.
fn normalizar(p: &Path) -> String {
    match p.as_str().split_once("::") {
        Some((_crate, resto)) => resto.to_string(),
        None => String::new(),
    }
}

/// Compara duas estruturas extraídas com **os mesmos parâmetros** (escopo/modo —
/// garantido pelo L4). `nome_*` são os nomes dos crates (rótulos do cabeçalho).
pub fn comparar_estruturas(
    antes: &EstruturaModulos,
    depois: &EstruturaModulos,
    nome_antes: &str,
    nome_depois: &str,
) -> Comparacao {
    // norm → path, por lado. Dentro de um lado a normalização é injetiva (o 1º
    // segmento é sempre o mesmo crate), então não há colisão intra-lado.
    let map_a: BTreeMap<String, &Path> =
        antes.modulos.iter().map(|p| (normalizar(p), p)).collect();
    let map_b: BTreeMap<String, &Path> =
        depois.modulos.iter().map(|p| (normalizar(p), p)).collect();

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
                let nd = normalizar(&d.de);
                let np = normalizar(&d.para);
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
    }
}

fn resumo_ciclos(est: &EstruturaModulos) -> ResumoCiclos {
    ResumoCiclos {
        quantidade: est.ciclos.len(),
        maior: est.ciclos.iter().map(|c| c.modulos.len()).max().unwrap_or(0),
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

    #[test]
    fn crate_renomeado_pareia_pela_normalizacao() {
        let a = est(&["velho", "velho::nucleo", "velho::nucleo::raio"], &[], &[]);
        let b = est(&["novo", "novo::nucleo", "novo::nucleo::raio"], &[], &[]);
        let c = comparar_estruturas(&a, &b, "velho", "novo");
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
        let c = comparar_estruturas(&a, &b, "k", "k");
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
        let c = comparar_estruturas(&a, &b, "k", "k");
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
        let c = comparar_estruturas(&a, &b, "k", "k");
        assert_eq!(c.ciclos_antes.quantidade, 1);
        assert_eq!(c.ciclos_antes.maior, 2);
        assert_eq!(c.ciclos_depois.quantidade, 0, "refatoração desfez o ciclo");
    }

    #[test]
    fn lado_vazio_nao_quebra() {
        let a = est(&["k", "k::a"], &[], &[]);
        let b = est(&[], &[], &[]);
        let c = comparar_estruturas(&a, &b, "k", "vazio");
        assert!(c.pareados.is_empty());
        assert_eq!(c.sem_par_antes.len(), 2);
        assert!(c.sem_par_depois.is_empty());
    }
}
