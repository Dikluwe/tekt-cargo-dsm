//! E2E `#[ignore]` para o filtro de stdlib contra dado real (laudo 0025,
//! Fase 2 — opção A). Requer fork `cargo-modules` instalado.
//!
//! O teste de unidade (em `src/lib.rs`) já cobre o Limite 2 sobre grafos
//! sintéticos. Este E2E ancora os números observados na Fase 1 contra o
//! `lente_core` real: 108 nós antes do filtro, 91 depois (17 sysroot
//! removidos: 10 `core`, 4 `alloc`, 3 `std`).
//!
//! Roda com `cargo test -p lente_filtro -- --ignored`.

use std::path::Path;

use lente_core::entities::grafo::Grafo;
use lente_filtro::filtrar_stdlib;

fn extrair_lente_core() -> Grafo {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("01_core");
    lente_infra::extrair_grafo(&raiz)
        .expect("extração do lente_core real deve funcionar")
}

#[test]
#[ignore]
fn filtra_lente_core_remove_sysroot_preserva_alvo() {
    let g = extrair_lente_core();
    let nodes_antes = g.nodes.len();

    let f = filtrar_stdlib(&g);

    // Grafo.crate_name preservado.
    assert_eq!(f.crate_name, "lente_core");

    // Nenhum nó remanescente começa com prefixo de sysroot.
    for n in &f.nodes {
        let prefixo = n.path.as_str().split("::").next().unwrap_or("");
        assert!(
            !matches!(prefixo, "core" | "std" | "alloc" | "proc_macro" | "test"),
            "nó {} de sysroot vazou: {}",
            n.id,
            n.path.as_str()
        );
    }

    // Contagem esperada (ancorada na Fase 1 do laudo 0025): 108 antes, 91
    // depois. Se variar com versão do fork, ajustar com registro no laudo.
    assert!(
        nodes_antes >= 100 && nodes_antes <= 130,
        "nodes_antes fora da banda esperada: {}",
        nodes_antes
    );
    let alvo = f.nodes.iter().filter(|n| n.crate_name == "lente_core").count();
    assert!(
        alvo >= 80 && alvo <= 110,
        "alvo fora da banda esperada (laudo 0025: ~91): {}",
        alvo
    );
}

#[test]
#[ignore]
fn limite_2_real_impl_de_traits_de_stdlib_no_lente_core_permanecem() {
    // Os impls do alvo de traits de stdlib que a Fase 1 ancorou (54 no
    // lente_core; aqui afirmamos uma amostra representativa do que NUNCA
    // pode sair).
    let g = extrair_lente_core();
    let f = filtrar_stdlib(&g);

    let casos = [
        ("lente_core::domain::raio::ErroRaio::fmt", "Display"),
        ("lente_core::domain::raio::ErroRaio::fmt", "Debug"),
        ("lente_core::domain::raio::Raio::clone", "Clone"),
        ("lente_core::domain::raio::Classificacao::eq", "PartialEq"),
        ("lente_core::domain::raio::Classificacao::hash", "Hash"),
    ];

    for (path_esperado, trait_esperado) in casos {
        let achou = f.nodes.iter().any(|n| {
            n.path.as_str() == path_esperado
                && n.trait_.as_deref() == Some(trait_esperado)
        });
        assert!(
            achou,
            "impl-do-alvo {path_esperado} (trait {trait_esperado}) foi removido — Limite 2 violado"
        );
    }
}

#[test]
#[ignore]
fn arestas_para_stdlib_somem_no_filtro() {
    let g = extrair_lente_core();
    let edges_antes = g.edges.len();

    let f = filtrar_stdlib(&g);

    // Toda aresta remanescente tem ambas as pontas em ids que sobreviveram.
    let ids_validos: std::collections::HashSet<usize> =
        f.nodes.iter().map(|n| n.id).collect();
    for a in &f.edges {
        assert!(
            ids_validos.contains(&a.id_from) && ids_validos.contains(&a.id_to),
            "aresta órfã: {}→{}",
            a.id_from,
            a.id_to
        );
    }

    // O número de arestas cai (porque toda aresta que tocava sysroot some).
    assert!(
        f.edges.len() < edges_antes,
        "esperava redução de arestas; antes={} depois={}",
        edges_antes,
        f.edges.len()
    );
}
