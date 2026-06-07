//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/filtro.md
//! @prompt-hash 59f298bd
//! @layer L1
//! @updated 2026-06-07
//!          ampliado por prompt 00_nucleo/prompt/0034-modo-uses-estrutura.md
//! Spec:    00_nucleo/specs/forma-organizada.md (Limite 2 — fronteira fina)
//! ADRs:    00_nucleo/adr/0002-modelagem-do-grafo.md (D3 preservada)
//! Lições:  00_nucleo/lessons/0025-l1-filtro-stdlib.md
//!          00_nucleo/lessons/0034-modo-uses-estrutura.md
//! Camada:  L1 — Núcleo. Pureza: stdlib do Rust + `lente_core`. Zero externas.
//!
//! Duas transformações puras `&Grafo -> Grafo`:
//!
//! - [`filtrar_stdlib`]: esconde nós de sysroot (`std`, `core`, `alloc`,
//!   `proc_macro`, `test`) por prefixo do path (ADR-0002 D3; laudo 0025).
//! - [`filtrar_so_referencia`]: mantém apenas as arestas `Uses` cujo
//!   `uses_kind` é `Reference` (uso de tipo direto); descarta `Import`
//!   (declaração `use` no nível do módulo, Limite 4 da spec) e arestas
//!   `Uses` sem `uses_kind` (JSON antigo). Arestas `Owns` são preservadas.
//!   Prompt 0034 (medido em 0033: 85→42 SCC no egui).
//!
//! Identidade do nó é por `id` — os filtros **não renumeram**.

#![forbid(unsafe_code)]

use std::collections::HashSet;

use lente_core::entities::grafo::{Grafo, Path, Relation, UsesKind};

/// Prefixos de path tratados como sysroot pela lente. Conjunto observado
/// no `lente_core` (`std`, `core`, `alloc`) ampliado com `proc_macro` e
/// `test` por defensividade — ambos podem aparecer em crates exóticos
/// (proc-macros e harnesses de teste embarcados).
const SYSROOT_PREFIXES: &[&str] = &["std", "core", "alloc", "proc_macro", "test"];

/// `true` se o **primeiro segmento** do path está em [`SYSROOT_PREFIXES`].
///
/// Comparação **por segmento**, não `starts_with` cego — um hipotético
/// crate `core_extras` ou `std_utils` **não** seria confundido com `core`
/// ou `std`. Defesa contra o tipo de erro que o laudo 0008 corrigiu na
/// chave de aresta.
fn e_de_sysroot(path: &Path) -> bool {
    let s = path.as_str();
    let primeiro = match s.find("::") {
        Some(i) => &s[..i],
        None => s,
    };
    SYSROOT_PREFIXES.contains(&primeiro)
}

/// Remove os nós de sysroot do grafo e as arestas que os tocam.
///
/// - `Grafo.crate_name` é **preservado**.
/// - `id` dos nós mantidos é **preservado** (sem renumeração).
/// - Uma aresta sai se `id_from` **ou** `id_to` referencia nó removido.
/// - Idempotente em grafos sem stdlib (mesma forma de saída).
pub fn filtrar_stdlib(grafo: &Grafo) -> Grafo {
    let mut ids_removidos: HashSet<usize> = HashSet::new();
    let mut nodes_novos = Vec::with_capacity(grafo.nodes.len());
    for n in &grafo.nodes {
        if e_de_sysroot(&n.path) {
            ids_removidos.insert(n.id);
        } else {
            nodes_novos.push(n.clone());
        }
    }
    let edges_novos: Vec<_> = grafo
        .edges
        .iter()
        .filter(|a| !ids_removidos.contains(&a.id_from) && !ids_removidos.contains(&a.id_to))
        .cloned()
        .collect();
    Grafo {
        crate_name: grafo.crate_name.clone(),
        nodes: nodes_novos,
        edges: edges_novos,
    }
}

/// Mantém apenas as arestas `Uses` cujo subtipo é `Reference` (uso de tipo
/// direto em assinatura/campo). Descarta:
///
/// - arestas `Uses` com `uses_kind == Some(Import)` — declaração `use` no
///   nível do módulo (Limite 4 da spec). Tipicamente infla ciclos no nível
///   módulo (laudo 0033: 51% do SCC do egui era artefato disso).
/// - arestas `Uses` com `uses_kind == None` — JSON antigo, sem o campo.
///   O `lente_filtro` **não** distingue ausente de presente: este é filtro
///   puro. Detectar fork antigo é responsabilidade da fiação
///   (`lente_wiring::analisar_estrutura`), que emite diagnóstico antes de
///   o usuário ver um resultado silenciosamente errado.
///
/// Arestas `Owns` são **preservadas** — necessárias para
/// `lente_estrutura::agregar_por_modulo` achar o módulo contenedor.
///
/// Nós são preservados; `Grafo.crate_name` é preservado; `id`s não
/// renumerados — mesma garantia do `filtrar_stdlib`.
pub fn filtrar_so_referencia(grafo: &Grafo) -> Grafo {
    let edges_novos: Vec<_> = grafo
        .edges
        .iter()
        .filter(|a| match a.relation {
            Relation::Owns => true,
            Relation::Uses => a.uses_kind == Some(UsesKind::Reference),
        })
        .cloned()
        .collect();
    Grafo {
        crate_name: grafo.crate_name.clone(),
        nodes: grafo.nodes.clone(),
        edges: edges_novos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::entities::grafo::{
        Aresta, Kind, Modificadores, No, Relation, Visibility,
    };

    /// Cria nó de teste mínimo. Os campos não exercitados ficam no default
    /// do tipo (Modificadores Default, sem trait/trait_ref/cfg/macro_kind).
    fn no(id: usize, path: &str, kind: Kind) -> No {
        No {
            id,
            path: Path::from(path),
            name: path.rsplit("::").next().unwrap_or(path).to_string(),
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

    fn no_com_trait(id: usize, path: &str, trait_nome: &str) -> No {
        let mut n = no(id, path, Kind::Fn);
        n.trait_ = Some(trait_nome.to_string());
        n.trait_ref = Some(trait_nome.to_string());
        n
    }

    fn aresta(id_from: usize, from: &str, id_to: usize, to: &str, rel: Relation) -> Aresta {
        Aresta {
            from: Path::from(from),
            id_from,
            to: Path::from(to),
            id_to,
            relation: rel,
            uses_kind: None,
        }
    }

    fn aresta_uses(
        id_from: usize,
        from: &str,
        id_to: usize,
        to: &str,
        kind: UsesKind,
    ) -> Aresta {
        Aresta {
            from: Path::from(from),
            id_from,
            to: Path::from(to),
            id_to,
            relation: Relation::Uses,
            uses_kind: Some(kind),
        }
    }

    // ---- Predicado de segmento -----------------------------------------------

    #[test]
    fn e_de_sysroot_aceita_primeiros_segmentos_conhecidos() {
        assert!(e_de_sysroot(&Path::from("core")));
        assert!(e_de_sysroot(&Path::from("core::fmt")));
        assert!(e_de_sysroot(&Path::from("alloc::alloc::Global")));
        assert!(e_de_sysroot(&Path::from("std::collections::HashMap")));
        assert!(e_de_sysroot(&Path::from("proc_macro::TokenStream")));
        assert!(e_de_sysroot(&Path::from("test::Bencher")));
    }

    #[test]
    fn e_de_sysroot_rejeita_paths_do_alvo_e_falsos_positivos() {
        assert!(!e_de_sysroot(&Path::from("meu")));
        assert!(!e_de_sysroot(&Path::from("meu::T::fmt")));
        // Falso-positivo do `starts_with`: NÃO devem ser sysroot.
        assert!(!e_de_sysroot(&Path::from("core_extras::Y")));
        assert!(!e_de_sysroot(&Path::from("std_utils")));
        assert!(!e_de_sysroot(&Path::from("alloc_pool::X")));
    }

    // ---- Limite 2 (a verificação central) ------------------------------------

    /// Caso da spec: `MinhaStruct::fmt` com `trait: Display` é IMPL DO ALVO
    /// (path no crate-alvo). O filtro NÃO pode removê-lo. A aresta do alvo
    /// para o trait de stdlib (`core::fmt::Display`) é removida porque o
    /// destino é stdlib — não porque o impl foi removido.
    #[test]
    fn limite_2_impl_do_alvo_de_trait_de_stdlib_e_preservado() {
        let mut g = Grafo::new("meu");
        g.nodes = vec![
            no(1, "meu", Kind::Crate),
            no(10, "meu::T", Kind::Struct),
            no_com_trait(20, "meu::T::fmt", "Display"), // <- impl do alvo
            no(100, "core::fmt::Display", Kind::Trait), // <- trait de stdlib
        ];
        g.edges = vec![
            aresta(1, "meu", 10, "meu::T", Relation::Owns),
            aresta(10, "meu::T", 20, "meu::T::fmt", Relation::Owns),
            aresta(20, "meu::T::fmt", 100, "core::fmt::Display", Relation::Uses),
        ];

        let f = filtrar_stdlib(&g);

        // O impl do alvo permanece, com id e trait preservados.
        let impl_alvo = f.nodes.iter().find(|n| n.id == 20).expect("impl preservado");
        assert_eq!(impl_alvo.path.as_str(), "meu::T::fmt");
        assert_eq!(impl_alvo.trait_.as_deref(), Some("Display"));

        // O trait de stdlib some.
        assert!(f.nodes.iter().all(|n| n.id != 100));

        // A aresta que tocava o trait some.
        assert!(f.edges.iter().all(|a| a.id_to != 100 && a.id_from != 100));

        // As arestas internas do alvo permanecem (owns).
        assert!(f.edges.iter().any(|a| a.id_from == 1 && a.id_to == 10));
        assert!(f.edges.iter().any(|a| a.id_from == 10 && a.id_to == 20));
    }

    // ---- Remoção e arestas tocadas ------------------------------------------

    #[test]
    fn no_sysroot_e_arestas_que_o_tocam_sao_removidos() {
        let mut g = Grafo::new("meu");
        g.nodes = vec![
            no(1, "meu", Kind::Crate),
            no(2, "meu::usuario", Kind::Fn),
            no(50, "alloc::alloc::Global", Kind::Struct),
        ];
        g.edges = vec![
            aresta(1, "meu", 2, "meu::usuario", Relation::Owns),
            // dois sentidos para garantir cobertura
            aresta(2, "meu::usuario", 50, "alloc::alloc::Global", Relation::Uses),
            aresta(50, "alloc::alloc::Global", 2, "meu::usuario", Relation::Uses),
        ];

        let f = filtrar_stdlib(&g);

        assert!(f.nodes.iter().all(|n| n.id != 50));
        assert_eq!(f.edges.len(), 1);
        assert_eq!(f.edges[0].id_from, 1);
        assert_eq!(f.edges[0].id_to, 2);
    }

    // ---- Dependências não-stdlib ficam --------------------------------------

    #[test]
    fn dep_nao_stdlib_e_mantida() {
        // Quando se analisa `egui`, nós cujo path começa por `emath`/`ecolor`
        // são dependências externas mas NÃO sysroot — devem ficar.
        let mut g = Grafo::new("egui");
        g.nodes = vec![
            no(1, "egui", Kind::Crate),
            no(2, "egui::Context", Kind::Struct),
            no(10, "emath::Vec2", Kind::Struct),
            no(11, "ecolor::Color32", Kind::Struct),
        ];
        g.edges = vec![
            aresta(2, "egui::Context", 10, "emath::Vec2", Relation::Uses),
            aresta(2, "egui::Context", 11, "ecolor::Color32", Relation::Uses),
        ];

        let f = filtrar_stdlib(&g);

        assert_eq!(f.nodes.len(), 4); // nada removido
        assert_eq!(f.edges.len(), 2);
    }

    // ---- Identidade (id) e Grafo.crate_name ---------------------------------

    #[test]
    fn ids_dos_mantidos_sao_preservados_sem_renumeracao() {
        let mut g = Grafo::new("meu");
        // ids deliberadamente não-contíguos.
        g.nodes = vec![
            no(7, "meu", Kind::Crate),
            no(42, "meu::T", Kind::Struct),
            no(99, "core::fmt", Kind::Mod),
        ];
        g.edges = vec![];

        let f = filtrar_stdlib(&g);
        let ids: Vec<usize> = f.nodes.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![7, 42]);
    }

    #[test]
    fn grafo_crate_name_e_preservado() {
        let mut g = Grafo::new("meu");
        g.nodes = vec![no(1, "core::fmt", Kind::Mod)];
        let f = filtrar_stdlib(&g);
        assert_eq!(f.crate_name, "meu");
    }

    // ---- Idempotência sobre grafo sem stdlib --------------------------------

    #[test]
    fn grafo_sem_stdlib_sai_inalterado() {
        let mut g = Grafo::new("meu");
        g.nodes = vec![
            no(1, "meu", Kind::Crate),
            no(2, "meu::T", Kind::Struct),
            no(3, "meu::T::metodo", Kind::Fn),
        ];
        g.edges = vec![
            aresta(1, "meu", 2, "meu::T", Relation::Owns),
            aresta(2, "meu::T", 3, "meu::T::metodo", Relation::Owns),
        ];
        let f = filtrar_stdlib(&g);
        assert_eq!(f, g);
    }

    #[test]
    fn idempotente_aplicar_duas_vezes_da_o_mesmo() {
        let mut g = Grafo::new("meu");
        g.nodes = vec![
            no(1, "meu", Kind::Crate),
            no(50, "core::fmt", Kind::Mod),
            no(51, "std::io", Kind::Mod),
        ];
        g.edges = vec![
            aresta(1, "meu", 50, "core::fmt", Relation::Uses),
            aresta(1, "meu", 51, "std::io", Relation::Uses),
        ];
        let uma = filtrar_stdlib(&g);
        let duas = filtrar_stdlib(&uma);
        assert_eq!(uma, duas);
    }

    // ---- Caso degenerado ----------------------------------------------------

    #[test]
    fn grafo_vazio_sai_vazio() {
        let g = Grafo::new("meu");
        let f = filtrar_stdlib(&g);
        assert_eq!(f.crate_name, "meu");
        assert!(f.nodes.is_empty());
        assert!(f.edges.is_empty());
    }

    // ---- filtrar_so_referencia (prompt 0034) --------------------------------

    /// Grafo-padrão para os testes do `filtrar_so_referencia`: um módulo
    /// com dois itens; uma aresta `Uses Reference` entre os itens; uma
    /// aresta `Uses Import` do módulo para um terceiro; e as `Owns` de
    /// hierarquia. O filtro deve preservar Reference + todas as Owns;
    /// descartar Import.
    fn grafo_uses_misto() -> Grafo {
        let mut g = Grafo::new("meu");
        g.nodes = vec![
            no(1, "meu", Kind::Crate),
            no(10, "meu::a", Kind::Mod),
            no(11, "meu::a::f", Kind::Fn),
            no(12, "meu::a::g", Kind::Fn),
            no(20, "meu::b", Kind::Mod),
        ];
        g.edges = vec![
            // Owns sempre None
            aresta(1, "meu", 10, "meu::a", Relation::Owns),
            aresta(10, "meu::a", 11, "meu::a::f", Relation::Owns),
            aresta(10, "meu::a", 12, "meu::a::g", Relation::Owns),
            aresta(1, "meu", 20, "meu::b", Relation::Owns),
            // Uses Reference: f usa g (uso de tipo)
            aresta_uses(11, "meu::a::f", 12, "meu::a::g", UsesKind::Reference),
            // Uses Import: módulo `a` declara `use meu::b` no topo
            aresta_uses(10, "meu::a", 20, "meu::b", UsesKind::Import),
        ];
        g
    }

    #[test]
    fn filtrar_so_referencia_preserva_uses_reference_e_descarta_import() {
        let g = grafo_uses_misto();
        let antes_uses: usize = g.edges.iter().filter(|a| a.relation == Relation::Uses).count();
        assert_eq!(antes_uses, 2);

        let f = filtrar_so_referencia(&g);
        let depois_uses: Vec<_> = f
            .edges
            .iter()
            .filter(|a| a.relation == Relation::Uses)
            .collect();
        assert_eq!(depois_uses.len(), 1, "só a aresta Reference sobrevive");
        assert_eq!(depois_uses[0].uses_kind, Some(UsesKind::Reference));
        assert_eq!(depois_uses[0].id_from, 11);
        assert_eq!(depois_uses[0].id_to, 12);
    }

    #[test]
    fn filtrar_so_referencia_preserva_todas_as_owns() {
        let g = grafo_uses_misto();
        let antes_owns = g.edges.iter().filter(|a| a.relation == Relation::Owns).count();
        let f = filtrar_so_referencia(&g);
        let depois_owns = f.edges.iter().filter(|a| a.relation == Relation::Owns).count();
        assert_eq!(antes_owns, depois_owns, "Owns intactas");
    }

    #[test]
    fn filtrar_so_referencia_descarta_uses_sem_kind() {
        // Cenário do "fork antigo": aresta `Uses` chega sem `uses_kind`.
        // Política deste filtro (puro): descarta — só `Reference` sobrevive.
        // O diagnóstico de fork-antigo é responsabilidade da fiação.
        let mut g = Grafo::new("meu");
        g.nodes = vec![
            no(1, "meu::a", Kind::Fn),
            no(2, "meu::b", Kind::Fn),
        ];
        g.edges = vec![aresta(1, "meu::a", 2, "meu::b", Relation::Uses)]; // sem kind
        let f = filtrar_so_referencia(&g);
        assert!(
            f.edges.iter().all(|a| a.relation != Relation::Uses),
            "uses sem kind deve sair"
        );
    }

    #[test]
    fn filtrar_so_referencia_preserva_nos_ids_e_crate_name() {
        let g = grafo_uses_misto();
        let f = filtrar_so_referencia(&g);
        assert_eq!(f.crate_name, "meu");
        // Mesmos nós (5).
        assert_eq!(f.nodes.len(), 5);
        let ids: Vec<_> = f.nodes.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![1, 10, 11, 12, 20]);
    }

    #[test]
    fn filtrar_so_referencia_idempotente() {
        let g = grafo_uses_misto();
        let f1 = filtrar_so_referencia(&g);
        let f2 = filtrar_so_referencia(&f1);
        assert_eq!(f1, f2);
    }
}
