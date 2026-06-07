//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/ranking.md
//! @prompt-hash 79471c54
//! @layer L1
//! @updated 2026-06-07
//!
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0002-modelagem-do-grafo.md
//! Lições:  00_nucleo/lessons/0027-ranking-top-n.md
//! Camada:  L1 — Núcleo. Pureza: stdlib + `lente_core`. Zero externas.
//!
//! Cálculo do top-N por impacto: para cada nó, `calcular_raio`; ordena
//! decrescente por `montante.len()`; corta no top-N. Reusa o cálculo do
//! `lente_core` (não duplica a indexação).
//!
//! **Promoção do protótipo de Arena** (`lab/medicao-egui` do laudo 0021).
//! O laço original foi validado contra o egui (12 crates, 11/12 OK); aqui
//! ele vira componente puro com ordem determinística e item tipado.
//!
//! Não otimiza pré-uso. `calcular_raio` reconstrói índices a cada chamada
//! — a Arena mostrou que custo é dominado pela extração, não pelo laço
//! (laudo 0027 Fase 1; ancoragem 0021). Se um dia medir-se lento, otimiza
//! com índice único; até lá, simplicidade > antecipação.

#![forbid(unsafe_code)]

use lente_core::domain::raio::{Classificacao, calcular_raio};
use lente_core::entities::grafo::{Grafo, Path};

/// Item do ranking: path do nó, impacto (montante.len), e a classificação
/// do `Raio`. A classificação carrega valor analítico — nós `Base` (têm
/// quem depende deles, não dependem de ninguém) são exatamente o que o
/// top-N quer encontrar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRanking {
    pub path: Path,
    pub impacto: usize,
    pub classificacao: Classificacao,
}

/// Top-N por impacto (`montante.len()`). Ordem decrescente; desempate por
/// path ascendente (determinístico). Corte em `n` — se `n` ≥ número de
/// nós, devolve todos.
///
/// Nós sem `montante` (Folha, Isolado) ainda entram no ranking com
/// `impacto = 0`; o consumidor decide se filtra. A função em si não filtra
/// para manter a transformação pura: "top-N" é critério de **ordenação +
/// corte**, não de seleção semântica.
///
/// Nós com path repetido no grafo: improvável após resolução de colisões
/// (`lente_resolve`), mas se ocorrer, cada nó é classificado uma vez
/// — `calcular_raio` é por path, então paths repetidos seriam conflados.
/// O wiring (`rankear_pacote`) aplica resolução antes; este L1 não.
pub fn rankear(grafo: &Grafo, n: usize) -> Vec<ItemRanking> {
    let mut itens: Vec<ItemRanking> = Vec::with_capacity(grafo.nodes.len());
    let mut paths_vistos: std::collections::HashSet<&Path> =
        std::collections::HashSet::with_capacity(grafo.nodes.len());
    for no in &grafo.nodes {
        if !paths_vistos.insert(&no.path) {
            // Path já processado — evita N chamadas a calcular_raio quando há
            // colisões não resolvidas. Mantém uma entrada por path no ranking.
            continue;
        }
        let raio = match calcular_raio(grafo, &no.path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        itens.push(ItemRanking {
            path: no.path.clone(),
            impacto: raio.montante.len(),
            classificacao: raio.classificacao,
        });
    }
    // Decrescente por impacto; desempate ascendente por path (determinístico).
    itens.sort_by(|a, b| {
        b.impacto
            .cmp(&a.impacto)
            .then_with(|| a.path.as_str().cmp(b.path.as_str()))
    });
    itens.truncate(n);
    itens
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::entities::grafo::{
        Aresta, Kind, Modificadores, No, Relation, Visibility,
    };

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

    fn uses(id_from: usize, from: &str, id_to: usize, to: &str) -> Aresta {
        Aresta {
            from: Path::from(from),
            id_from,
            to: Path::from(to),
            id_to,
            relation: Relation::Uses,
            uses_kind: None,
        }
    }

    /// Monta um grafo em que `base` tem impacto = 3 (a, b, c dependem dele),
    /// `meio` tem impacto = 1 (só `a` depende), e `a`, `b`, `c` têm 0.
    /// Hierarquia de impacto: base > meio > {a,b,c}.
    fn grafo_simples() -> Grafo {
        let mut g = Grafo::new("meu");
        g.nodes = vec![
            no(1, "meu::base", Kind::Fn),
            no(2, "meu::meio", Kind::Fn),
            no(3, "meu::a", Kind::Fn),
            no(4, "meu::b", Kind::Fn),
            no(5, "meu::c", Kind::Fn),
        ];
        g.edges = vec![
            uses(3, "meu::a", 1, "meu::base"),
            uses(4, "meu::b", 1, "meu::base"),
            uses(5, "meu::c", 1, "meu::base"),
            uses(3, "meu::a", 2, "meu::meio"),
            uses(2, "meu::meio", 1, "meu::base"),
        ];
        g
    }

    #[test]
    fn ordem_decrescente_por_impacto() {
        // base alcança {a,b,c,meio} via reverse-uses BFS → impacto 4.
        // meio alcança {a} via reverse-uses BFS → impacto 1.
        // a, b, c → 0.
        let r = rankear(&grafo_simples(), 10);
        assert_eq!(r[0].path.as_str(), "meu::base");
        assert!(r[0].impacto > r[1].impacto);
        assert_eq!(r[1].path.as_str(), "meu::meio");
    }

    #[test]
    fn corte_em_n_limita_o_resultado() {
        let r = rankear(&grafo_simples(), 2);
        assert_eq!(r.len(), 2);
        // Os dois primeiros são base e meio (ordem por impacto).
        let paths: Vec<&str> = r.iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, vec!["meu::base", "meu::meio"]);
    }

    #[test]
    fn n_maior_que_nos_devolve_todos() {
        let r = rankear(&grafo_simples(), 999);
        assert_eq!(r.len(), 5);
    }

    #[test]
    fn desempate_por_path_ascendente_e_deterministico() {
        // Três nós sem ninguém dependendo deles → impacto 0; desempate por path.
        let mut g = Grafo::new("meu");
        g.nodes = vec![
            no(1, "meu::z_ultimo", Kind::Fn),
            no(2, "meu::a_primeiro", Kind::Fn),
            no(3, "meu::m_meio", Kind::Fn),
        ];
        g.edges = vec![];
        let r = rankear(&g, 10);
        let paths: Vec<&str> = r.iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, vec!["meu::a_primeiro", "meu::m_meio", "meu::z_ultimo"]);
    }

    #[test]
    fn classificacao_aparece_no_item() {
        // base não tem uses_saida → é Base. a/b/c não tem uses_entrada → Folha.
        let r = rankear(&grafo_simples(), 10);
        let base = r.iter().find(|i| i.path.as_str() == "meu::base").unwrap();
        assert_eq!(base.classificacao, Classificacao::Base);
        let a = r.iter().find(|i| i.path.as_str() == "meu::a").unwrap();
        assert_eq!(a.classificacao, Classificacao::Folha);
    }

    #[test]
    fn grafo_vazio_devolve_vazio() {
        let g = Grafo::new("meu");
        assert!(rankear(&g, 10).is_empty());
    }

    #[test]
    fn n_zero_devolve_vazio_sem_panic() {
        let r = rankear(&grafo_simples(), 0);
        assert!(r.is_empty());
    }

    /// Paths repetidos: o ranking não chama calcular_raio duas vezes para
    /// o mesmo path. Garante determinismo e custo controlado.
    #[test]
    fn path_repetido_aparece_uma_vez_no_ranking() {
        let mut g = Grafo::new("meu");
        // Dois nós com o mesmo path (colisão não resolvida).
        g.nodes = vec![
            no(1, "meu::T::fmt", Kind::Fn),
            no(2, "meu::T::fmt", Kind::Fn),
        ];
        g.edges = vec![];
        let r = rankear(&g, 10);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].path.as_str(), "meu::T::fmt");
    }
}
