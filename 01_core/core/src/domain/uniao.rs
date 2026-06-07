//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/uniao.md
//! @prompt-hash c43b96e2
//! @layer L1
//! @updated 2026-06-07
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0002-modelagem-do-grafo.md
//! Camada:  L1 — Núcleo. Apenas stdlib. Sem I/O. Sem deps externas.
//!
//! União de grafos por crate num **grafo de workspace** único.
//!
//! Os grafos chegam **já resolvidos por crate** (a fiação L4 resolve antes de
//! unir, laudo 0041) e **etiquetados** com o nome do crate de onde vieram —
//! a etiqueta é o que distingue uma **definição** de uma **referência**.
//!
//! ## Definição vs referência (o discriminador correto)
//!
//! O campo [`No::crate_name`](crate::entities::grafo::No::crate_name) é o
//! **crate-raiz do grafo**, idêntico para todos os nós de uma extração
//! (inclusive os nós-referência a outros crates e os de sysroot). Logo ele
//! **não** discrimina definição de referência. O discriminador real é o
//! **prefixo do path** vs a **etiqueta do grafo** (laudos 0040/0043):
//!
//! - Um nó do grafo etiquetado `C` é **definição** do path `P` quando o
//!   primeiro segmento de `P` é `C` (o crate dono do item produziu o nó).
//! - É **referência** quando difere (outro crate carrega um nó-referência —
//!   ex.: `lente_infra` carrega `lente_core::Grafo`).
//!
//! Para cada path: se há definição, ela vence (descarta as referências, que
//! são idênticas módulo `id`). Se há só referências e o crate-dono é um
//! membro do workspace, é **fantasma** — referenciado mas não produzido
//! (item renomeado/removido). Mantém-se um nó-representante (0 arestas
//! soltas, laudo 0039) e registra-se o fantasma.
//!
//! Paths cujo primeiro segmento **não** é membro do workspace (sysroot,
//! deps externas) nunca têm definição aqui e **não** são fantasmas — são
//! externos legítimos, com nó-representante.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::entities::grafo::{Aresta, Grafo, No, Path, Relation, UsesKind};

/// Um grafo de crate etiquetado com o nome do seu crate (a etiqueta permite
/// distinguir definição de referência na união).
#[derive(Debug, Clone)]
pub struct GrafoCrate {
    pub crate_name: String,
    pub grafo: Grafo,
}

/// Um path referenciado por algum crate mas **não produzido** pelo crate-dono
/// (primeiro segmento do path). Sinal de cache stale / renomeação / remoção
/// (laudo 0040). Esperado vazio neste repo (laudo 0041).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fantasma {
    pub path: Path,
    /// Crates (etiquetas) que referenciam o path. Ordenado, sem repetição.
    pub referenciado_por: Vec<String>,
}

/// Resultado da união: o grafo unificado (paths únicos, ids reindexados,
/// arestas religadas por path) e os fantasmas detectados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoUniao {
    pub grafo: Grafo,
    pub fantasmas: Vec<Fantasma>,
}

/// Une os grafos resolvidos por crate num grafo de workspace.
///
/// Determinística: itera por estruturas ordenadas (`BTreeMap`/`BTreeSet`);
/// nenhuma ordem de `HashMap` vaza para a saída. Unir o mesmo conjunto duas
/// vezes dá o mesmo grafo.
pub fn unir_grafos(grafos: Vec<GrafoCrate>) -> ResultadoUniao {
    let membros: BTreeSet<&str> = grafos.iter().map(|g| g.crate_name.as_str()).collect();

    // 1. Agrupar todas as cópias de nó por path: (etiqueta_do_grafo, nó).
    let mut copias_por_path: BTreeMap<&str, Vec<(&str, &No)>> = BTreeMap::new();
    for gc in &grafos {
        for n in &gc.grafo.nodes {
            copias_por_path
                .entry(n.path.as_str())
                .or_default()
                .push((gc.crate_name.as_str(), n));
        }
    }

    // 2. Para cada path: escolher o nó (definição vence) e detectar fantasma.
    let mut escolhido_por_path: BTreeMap<&str, No> = BTreeMap::new();
    let mut fantasmas: Vec<Fantasma> = Vec::new();
    for (path, copias) in &copias_por_path {
        let dono = path.split("::").next().unwrap_or("");
        let definicoes: Vec<&(&str, &No)> =
            copias.iter().filter(|(etq, _)| *etq == dono).collect();
        let escolhido = if !definicoes.is_empty() {
            melhor_no(definicoes.into_iter().copied())
        } else {
            if membros.contains(dono) {
                let mut refs: Vec<String> =
                    copias.iter().map(|(etq, _)| etq.to_string()).collect();
                refs.sort();
                refs.dedup();
                fantasmas.push(Fantasma {
                    path: Path::from(*path),
                    referenciado_por: refs,
                });
            }
            melhor_no(copias.iter().copied())
        };
        escolhido_por_path.insert(path, escolhido);
    }

    // 3. Reindexar: ids novos sequenciais por path ordenado.
    let mut id_por_path: BTreeMap<&str, usize> = BTreeMap::new();
    let mut nodes: Vec<No> = Vec::with_capacity(escolhido_por_path.len());
    for (i, (path, mut no)) in escolhido_por_path.into_iter().enumerate() {
        no.id = i;
        id_por_path.insert(path, i);
        nodes.push(no);
    }

    // 4. Arestas: religar por path, deduplicar idênticas. Endpoint sem nó
    //    (não deveria ocorrer — os fantasmas mantêm representante) é descartado.
    let mut vistas: HashSet<(usize, usize, Relation, Option<UsesKind>)> = HashSet::new();
    let mut edges: Vec<Aresta> = Vec::new();
    for gc in &grafos {
        for a in &gc.grafo.edges {
            let (Some(&id_from), Some(&id_to)) = (
                id_por_path.get(a.from.as_str()),
                id_por_path.get(a.to.as_str()),
            ) else {
                continue;
            };
            if !vistas.insert((id_from, id_to, a.relation, a.uses_kind)) {
                continue;
            }
            edges.push(Aresta {
                from: a.from.clone(),
                id_from,
                to: a.to.clone(),
                id_to,
                relation: a.relation,
                uses_kind: a.uses_kind,
            });
        }
    }
    edges.sort_by(|a, b| {
        (a.id_from, a.id_to, rel_ord(a.relation), uk_ord(a.uses_kind)).cmp(&(
            b.id_from,
            b.id_to,
            rel_ord(b.relation),
            uk_ord(b.uses_kind),
        ))
    });

    fantasmas.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));

    ResultadoUniao {
        grafo: Grafo {
            crate_name: "workspace".to_string(),
            nodes,
            edges,
        },
        fantasmas,
    }
}

/// Escolhe o melhor nó entre cópias do mesmo path: prefere o que tem
/// `position` (a definição completa, vs uma referência leve); empate decide
/// pela etiqueta e depois pelo id — determinístico.
fn melhor_no<'a>(copias: impl Iterator<Item = (&'a str, &'a No)>) -> No {
    copias
        .min_by(|(etq_a, na), (etq_b, nb)| {
            let pa = na.position.is_some();
            let pb = nb.position.is_some();
            // `true` deve vir antes de `false`: comparar `pb.cmp(pa)`.
            pb.cmp(&pa)
                .then_with(|| etq_a.cmp(etq_b))
                .then_with(|| na.id.cmp(&nb.id))
        })
        .map(|(_, n)| n.clone())
        .expect("cópias não-vazias por construção")
}

fn rel_ord(r: Relation) -> u8 {
    match r {
        Relation::Owns => 0,
        Relation::Uses => 1,
    }
}

fn uk_ord(uk: Option<UsesKind>) -> u8 {
    match uk {
        None => 0,
        Some(UsesKind::Reference) => 1,
        Some(UsesKind::Import) => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::grafo::{Kind, Modificadores, Posicao, Visibility};

    /// Constrói um nó com `crate_name` = a etiqueta do grafo (como o L3 faz:
    /// crate-raiz copiado para todo nó). `tem_pos` marca definição (com
    /// `position`) vs referência leve (sem).
    fn no(id: usize, path: &str, crate_raiz: &str, tem_pos: bool) -> No {
        No {
            id,
            path: Path::from(path),
            name: path.rsplit("::").next().unwrap_or(path).to_string(),
            kind: Kind::Fn,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: crate_raiz.to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: if tem_pos {
                Some(Posicao {
                    file: format!("/{}/src/lib.rs", crate_raiz),
                    start_line: 1,
                    end_line: 2,
                })
            } else {
                None
            },
        }
    }

    fn aresta(from: &str, to: &str, rel: Relation) -> Aresta {
        Aresta {
            from: Path::from(from),
            id_from: 0,
            to: Path::from(to),
            id_to: 0,
            relation: rel,
            uses_kind: if rel == Relation::Uses {
                Some(UsesKind::Reference)
            } else {
                None
            },
        }
    }

    fn gc(crate_name: &str, nodes: Vec<No>, edges: Vec<Aresta>) -> GrafoCrate {
        GrafoCrate {
            crate_name: crate_name.to_string(),
            grafo: Grafo {
                crate_name: crate_name.to_string(),
                nodes,
                edges,
            },
        }
    }

    fn paths(g: &Grafo) -> Vec<&str> {
        g.nodes.iter().map(|n| n.path.as_str()).collect()
    }

    #[test]
    fn definicao_vence_referencia_e_aresta_cross_crate_religa() {
        // A referencia B::Foo (nó-referência, sem position) e usa-o;
        // B define B::Foo (com position).
        let grafo_a = gc(
            "a",
            vec![
                no(0, "a", "a", true),
                no(1, "a::usa", "a", true),
                no(2, "b::Foo", "a", false), // referência leve
            ],
            vec![aresta("a::usa", "b::Foo", Relation::Uses)],
        );
        let grafo_b = gc(
            "b",
            vec![no(0, "b", "b", true), no(1, "b::Foo", "b", true)],
            vec![],
        );

        let r = unir_grafos(vec![grafo_a, grafo_b]);
        assert!(r.fantasmas.is_empty(), "sem fantasma: B define Foo");
        // UM nó b::Foo, e é a definição (tem position).
        let foo: Vec<&No> = r.grafo.nodes.iter().filter(|n| n.path.as_str() == "b::Foo").collect();
        assert_eq!(foo.len(), 1, "b::Foo deve ser único");
        assert!(foo[0].position.is_some(), "vence a definição (com position)");
        // A aresta a::usa → b::Foo religa (0 soltas): ambos os endpoints existem.
        let id_usa = r.grafo.nodes.iter().find(|n| n.path.as_str() == "a::usa").unwrap().id;
        let id_foo = foo[0].id;
        assert!(
            r.grafo.edges.iter().any(|e| e.id_from == id_usa && e.id_to == id_foo
                && e.relation == Relation::Uses),
            "aresta cross-crate deve religar"
        );
        // Integridade: todo endpoint casa um nó.
        let ids: BTreeSet<usize> = r.grafo.nodes.iter().map(|n| n.id).collect();
        for e in &r.grafo.edges {
            assert!(ids.contains(&e.id_from) && ids.contains(&e.id_to), "0 arestas soltas");
        }
    }

    #[test]
    fn referencia_sem_definicao_vira_fantasma_com_representante() {
        // A referencia b::Foo, mas B NÃO tem b::Foo (renomeado/removido).
        let grafo_a = gc(
            "a",
            vec![no(0, "a::usa", "a", true), no(1, "b::Foo", "a", false)],
            vec![aresta("a::usa", "b::Foo", Relation::Uses)],
        );
        let grafo_b = gc("b", vec![no(0, "b", "b", true)], vec![]);

        let r = unir_grafos(vec![grafo_a, grafo_b]);
        assert_eq!(r.fantasmas.len(), 1);
        assert_eq!(r.fantasmas[0].path.as_str(), "b::Foo");
        assert_eq!(r.fantasmas[0].referenciado_por, vec!["a".to_string()]);
        // Representante mantido → aresta religa, 0 soltas.
        assert!(r.grafo.nodes.iter().any(|n| n.path.as_str() == "b::Foo"));
        let ids: BTreeSet<usize> = r.grafo.nodes.iter().map(|n| n.id).collect();
        for e in &r.grafo.edges {
            assert!(ids.contains(&e.id_from) && ids.contains(&e.id_to));
        }
    }

    #[test]
    fn nos_identicos_para_o_mesmo_path_viram_um_so() {
        // Referência de A == definição de B (mesmo path).
        let grafo_a = gc("a", vec![no(5, "b::Foo", "a", false)], vec![]);
        let grafo_b = gc("b", vec![no(9, "b::Foo", "b", true)], vec![]);
        let r = unir_grafos(vec![grafo_a, grafo_b]);
        assert_eq!(r.grafo.nodes.iter().filter(|n| n.path.as_str() == "b::Foo").count(), 1);
    }

    #[test]
    fn cadeia_tres_crates_liga_e_zero_soltas() {
        // A→B→C: a::f usa b::g; b::g usa c::h.
        let grafo_a = gc(
            "a",
            vec![no(0, "a::f", "a", true), no(1, "b::g", "a", false)],
            vec![aresta("a::f", "b::g", Relation::Uses)],
        );
        let grafo_b = gc(
            "b",
            vec![no(0, "b::g", "b", true), no(1, "c::h", "b", false)],
            vec![aresta("b::g", "c::h", Relation::Uses)],
        );
        let grafo_c = gc("c", vec![no(0, "c::h", "c", true)], vec![]);

        let r = unir_grafos(vec![grafo_a, grafo_b, grafo_c]);
        assert!(r.fantasmas.is_empty());
        // 4 paths: a::f, b::g, c::h. (sem nós-raiz neste fixture)
        assert_eq!(paths(&r.grafo), vec!["a::f", "b::g", "c::h"]);
        // Duas arestas Uses, ligando a cadeia; 0 soltas.
        assert_eq!(r.grafo.edges.len(), 2);
        let ids: BTreeSet<usize> = r.grafo.nodes.iter().map(|n| n.id).collect();
        for e in &r.grafo.edges {
            assert!(ids.contains(&e.id_from) && ids.contains(&e.id_to));
        }
    }

    #[test]
    fn uniao_e_deterministica() {
        let montar = || {
            vec![
                gc(
                    "a",
                    vec![no(0, "a::f", "a", true), no(1, "b::g", "a", false)],
                    vec![aresta("a::f", "b::g", Relation::Uses)],
                ),
                gc("b", vec![no(0, "b::g", "b", true)], vec![]),
            ]
        };
        let r1 = unir_grafos(montar());
        let r2 = unir_grafos(montar());
        assert_eq!(r1, r2);
    }

    #[test]
    fn ids_reindexados_unicos_e_paths_unicos() {
        let grafo_a = gc(
            "a",
            vec![no(99, "a::x", "a", true), no(7, "a::y", "a", true)],
            vec![],
        );
        let grafo_b = gc("b", vec![no(99, "b::z", "b", true)], vec![]);
        let r = unir_grafos(vec![grafo_a, grafo_b]);
        let ids: Vec<usize> = r.grafo.nodes.iter().map(|n| n.id).collect();
        // ids sequenciais 0..N
        assert_eq!(ids, (0..r.grafo.nodes.len()).collect::<Vec<_>>());
        // paths únicos
        let set: BTreeSet<&str> = r.grafo.nodes.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(set.len(), r.grafo.nodes.len());
    }

    #[test]
    fn paths_externos_nao_sao_fantasmas() {
        // core::fmt::Display referenciado por A; "core" não é membro → externo,
        // não fantasma. Mantém representante.
        let grafo_a = gc(
            "a",
            vec![no(0, "a::f", "a", true), no(1, "core::fmt::Display", "a", false)],
            vec![aresta("a::f", "core::fmt::Display", Relation::Uses)],
        );
        let r = unir_grafos(vec![grafo_a]);
        assert!(r.fantasmas.is_empty(), "core::* é externo, não fantasma");
        assert!(r.grafo.nodes.iter().any(|n| n.path.as_str() == "core::fmt::Display"));
    }
}
