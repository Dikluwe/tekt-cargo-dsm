//! Lineage: prompt 00_nucleo/prompt/0006-id_no_core_infra.md
//!           (antes: prompt 00_nucleo/prompt/0003-adaptador_l3.md)
//!
//! Tradução dos DTOs do JSON para os tipos do `lente_core`, validando os enums
//! na borda (ADR-0002 D1) e os invariantes da spec **revisados** (Mudança 3 do
//! patch da spec): identidade por `id` (não mais por `path`), integridade
//! referencial via `id_from`/`id_to`.
//!
//! `path` pode repetir entre nós distintos (caso comum: colisões via
//! Display+Debug, derives, operadores aritméticos overloaded). Só `id`
//! discrimina identidade.
//!
//! Função pura sobre estruturas em memória — testável sem subprocesso.

use std::collections::HashSet;

use lente_core::entities::grafo::{
    Aresta, Grafo, Kind, Modificadores, No, Path, Relation, Visibility,
};

use crate::ErroAdaptador;
use crate::dto::GrafoDTO;

pub(crate) fn traduzir(dto: GrafoDTO) -> Result<Grafo, ErroAdaptador> {
    let mut nodes: Vec<No> = Vec::with_capacity(dto.nodes.len());
    let mut ids_vistos: HashSet<usize> = HashSet::with_capacity(dto.nodes.len());
    let crate_name = dto.crate_name.clone();

    for no_dto in dto.nodes {
        let kind = Kind::try_from(no_dto.kind.as_str())
            .map_err(ErroAdaptador::ValorDesconhecido)?;
        let visibility = Visibility::try_from(no_dto.visibility.as_str())
            .map_err(ErroAdaptador::ValorDesconhecido)?;
        let path = Path::from(no_dto.path);

        if !ids_vistos.insert(no_dto.id) {
            return Err(ErroAdaptador::IdDuplicado(no_dto.id));
        }

        // Modificadores vêm dos BOOLEANOS do fork (fonte da verdade), não da
        // string `kind` — que ainda os traz embutidos por retrocompat, mas o
        // `TryFrom` do Kind já os descarta (laudo 0012).
        let modificadores = Modificadores {
            is_const: no_dto.is_const,
            is_async: no_dto.is_async,
            is_unsafe: no_dto.is_unsafe,
        };
        // `cfg` chega estruturado; serializamos para texto (a forma que o
        // `lente_core` modela). `None` quando ausente.
        let cfg = no_dto.cfg.as_ref().map(|v| v.to_string());

        nodes.push(No {
            id: no_dto.id,
            path,
            name: no_dto.name,
            kind,
            modificadores,
            visibility,
            // O fork 0.27.0 NÃO emite `crate` por nó (ver laudo 0013); usamos
            // o crate-raiz do grafo. A marca de stdlib, quando precisar, sai
            // do prefixo do path (ADR-0002 D3), não deste campo.
            crate_name: crate_name.clone(),
            trait_: no_dto.trait_,
            trait_ref: no_dto.trait_ref,
            cfg,
            macro_kind: no_dto.macro_kind,
            is_non_exhaustive: no_dto.is_non_exhaustive,
        });
    }

    let mut edges: Vec<Aresta> = Vec::with_capacity(dto.edges.len());
    for aresta_dto in dto.edges {
        let from = Path::from(aresta_dto.from);
        let to = Path::from(aresta_dto.to);
        let relation = Relation::try_from(aresta_dto.relation.as_str())
            .map_err(ErroAdaptador::ValorDesconhecido)?;

        if !ids_vistos.contains(&aresta_dto.id_from) {
            return Err(ErroAdaptador::IdReferenciado {
                id: aresta_dto.id_from,
                contexto: "id_from".to_string(),
            });
        }
        if !ids_vistos.contains(&aresta_dto.id_to) {
            return Err(ErroAdaptador::IdReferenciado {
                id: aresta_dto.id_to,
                contexto: "id_to".to_string(),
            });
        }

        edges.push(Aresta {
            from,
            id_from: aresta_dto.id_from,
            to,
            id_to: aresta_dto.id_to,
            relation,
        });
    }

    Ok(Grafo {
        crate_name: dto.crate_name,
        nodes,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{ArestaDTO, NoDTO};

    fn no_dto(id: usize, path: &str, kind: &str, vis: &str) -> NoDTO {
        NoDTO {
            id,
            path: path.to_string(),
            name: path.rsplit("::").next().unwrap_or(path).to_string(),
            kind: kind.to_string(),
            visibility: vis.to_string(),
            is_const: false,
            is_async: false,
            is_unsafe: false,
            is_non_exhaustive: false,
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
        }
    }

    fn aresta_dto(
        from: &str,
        id_from: usize,
        to: &str,
        id_to: usize,
        relation: &str,
    ) -> ArestaDTO {
        ArestaDTO {
            from: from.to_string(),
            id_from,
            to: to.to_string(),
            id_to,
            relation: relation.to_string(),
        }
    }

    #[test]
    fn traduz_grafo_minimo_valido() {
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![no_dto(1, "t", "crate", "pub")],
            edges: vec![],
        };
        let g = traduzir(dto).unwrap();
        assert_eq!(g.crate_name, "t");
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.nodes[0].id, 1);
        assert_eq!(g.nodes[0].kind, Kind::Crate);
        assert_eq!(g.nodes[0].visibility, Visibility::Pub);
        assert!(g.edges.is_empty());
    }

    #[test]
    fn traduz_grafo_com_arestas_validas() {
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![
                no_dto(1, "t", "crate", "pub"),
                no_dto(2, "t::foo", "mod", "pub"),
            ],
            edges: vec![aresta_dto("t", 1, "t::foo", 2, "owns")],
        };
        let g = traduzir(dto).unwrap();
        assert_eq!(g.edges.len(), 1);
        assert_eq!(g.edges[0].relation, Relation::Owns);
        assert_eq!(g.edges[0].id_from, 1);
        assert_eq!(g.edges[0].id_to, 2);
    }

    #[test]
    fn paths_colidentes_com_ids_distintos_sao_aceitos() {
        // O caso real do `ErroRaio::fmt`: dois nós com mesmo path,
        // discriminados pelo `id`. Antes da Mudança 3, isso era erro
        // (`PathDuplicado`); agora é dado legítimo.
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![
                no_dto(1, "t::T::fmt", "fn", "pub"),
                no_dto(2, "t::T::fmt", "fn", "pub"),
            ],
            edges: vec![],
        };
        let g = traduzir(dto).unwrap();
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.nodes[0].path, g.nodes[1].path);
        assert_ne!(g.nodes[0].id, g.nodes[1].id);
    }

    #[test]
    fn descritor_trait_e_propagado() {
        let mut n = no_dto(1, "t::T::fmt", "fn", "priv");
        n.trait_ = Some("Display".to_string());
        n.trait_ref = Some("Display".to_string());
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![n],
            edges: vec![],
        };
        let g = traduzir(dto).unwrap();
        assert_eq!(g.nodes[0].trait_.as_deref(), Some("Display"));
        assert_eq!(g.nodes[0].trait_ref.as_deref(), Some("Display"));
    }

    #[test]
    fn modificadores_vem_dos_booleanos_nao_da_string() {
        // kind "const fn" (modificador embutido na string) + is_const=true.
        let mut n = no_dto(1, "t::c", "const fn", "pub");
        n.is_const = true;
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![n],
            edges: vec![],
        };
        let g = traduzir(dto).unwrap();
        // Kind despe o modificador → tipo base Fn.
        assert_eq!(g.nodes[0].kind, Kind::Fn);
        // Modificador vem do booleano, não da string.
        assert!(g.nodes[0].modificadores.is_const);
        assert!(!g.nodes[0].modificadores.is_async);
        assert!(!g.nodes[0].modificadores.is_unsafe);
    }

    #[test]
    fn no_sem_descritor_fica_em_default() {
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![no_dto(1, "t::x", "fn", "pub")],
            edges: vec![],
        };
        let g = traduzir(dto).unwrap();
        let n = &g.nodes[0];
        assert_eq!(n.trait_, None);
        assert_eq!(n.trait_ref, None);
        assert_eq!(n.cfg, None);
        assert_eq!(n.macro_kind, None);
        assert!(!n.is_non_exhaustive);
        assert_eq!(n.modificadores, Modificadores::default());
        // crate_name vem do grafo (o fork não emite crate por nó).
        assert_eq!(n.crate_name, "t");
    }

    #[test]
    fn cfg_estruturado_do_fork_vira_texto() {
        let mut n = no_dto(1, "t::so_unix", "fn", "pub");
        n.cfg = Some(serde_json::from_str(r#"[{"Flag":"unix"}]"#).unwrap());
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![n],
            edges: vec![],
        };
        let g = traduzir(dto).unwrap();
        let cfg = g.nodes[0].cfg.as_deref().unwrap();
        assert!(cfg.contains("unix"), "cfg textual contém a flag: {}", cfg);
        assert!(cfg.contains("Flag"));
    }

    #[test]
    fn kind_desconhecido_falha_na_borda() {
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![no_dto(1, "t::x", "borrows", "pub")],
            edges: vec![],
        };
        match traduzir(dto).unwrap_err() {
            ErroAdaptador::ValorDesconhecido(v) => {
                assert_eq!(v.tipo, "Kind");
                assert_eq!(v.texto, "borrows");
            }
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    #[test]
    fn visibility_desconhecida_falha_na_borda() {
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![no_dto(1, "t::x", "fn", "hidden")],
            edges: vec![],
        };
        match traduzir(dto).unwrap_err() {
            ErroAdaptador::ValorDesconhecido(v) => assert_eq!(v.tipo, "Visibility"),
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    #[test]
    fn relation_desconhecida_falha_na_borda() {
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![
                no_dto(1, "t", "crate", "pub"),
                no_dto(2, "t::x", "fn", "pub"),
            ],
            edges: vec![aresta_dto("t", 1, "t::x", 2, "borrows")],
        };
        match traduzir(dto).unwrap_err() {
            ErroAdaptador::ValorDesconhecido(v) => assert_eq!(v.tipo, "Relation"),
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    #[test]
    fn id_duplicado_e_invariante_violado() {
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![
                no_dto(1, "t::a", "fn", "pub"),
                no_dto(1, "t::b", "fn", "pub"), // id repetido — bug do fork
            ],
            edges: vec![],
        };
        match traduzir(dto).unwrap_err() {
            ErroAdaptador::IdDuplicado(id) => assert_eq!(id, 1),
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    #[test]
    fn id_from_referenciado_inexistente_e_invariante_violado() {
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![no_dto(1, "t", "crate", "pub")],
            edges: vec![aresta_dto("fantasma", 99, "t", 1, "uses")],
        };
        match traduzir(dto).unwrap_err() {
            ErroAdaptador::IdReferenciado { id, contexto } => {
                assert_eq!(id, 99);
                assert_eq!(contexto, "id_from");
            }
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    #[test]
    fn id_to_referenciado_inexistente_e_invariante_violado() {
        let dto = GrafoDTO {
            crate_name: "t".to_string(),
            nodes: vec![no_dto(1, "t", "crate", "pub")],
            edges: vec![aresta_dto("t", 1, "fantasma", 99, "uses")],
        };
        match traduzir(dto).unwrap_err() {
            ErroAdaptador::IdReferenciado { id, contexto } => {
                assert_eq!(id, 99);
                assert_eq!(contexto, "id_to");
            }
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }
}
