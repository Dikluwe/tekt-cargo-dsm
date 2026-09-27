//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-doxygen.md
//! @prompt-hash 82ed175f
//! @layer L3
//! @updated 2026-09-26
use lente_core::domain::raio::calcular_raio;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Projeta fatos disponíveis, sem inferir identidades documentais.
pub fn projetar(
    bytes: &[u8],
    nomes: &str,
    coverage: &str,
    alvos: Option<&[u64]>,
) -> Result<Value, String> {
    if !matches!(nomes, "opaque" | "rust-path")
        || !matches!(coverage, "complete" | "partial" | "unknown")
    {
        return Err("parametros: politica_nomes ou coverage inválida".into());
    }
    let texto = std::str::from_utf8(bytes).map_err(|e| format!("schema: {e}"))?;
    let g = crate::desserializar_grafo(texto).map_err(|e| format!("schema: {e}"))?;
    let ids: BTreeSet<_> = g.nodes.iter().map(|n| n.id as u64).collect();
    if let Some(alvos) = alvos {
        let escolhidos: BTreeSet<_> = alvos.iter().copied().collect();
        if escolhidos.len() != alvos.len() || !escolhidos.is_subset(&ids) {
            return Err("parametros: alvos_metricas duplicados ou inexistentes".into());
        }
    }
    for e in &g.edges {
        if g.nodes.iter().find(|n| n.id == e.id_from).map(|n| &n.path) != Some(&e.from)
            || g.nodes.iter().find(|n| n.id == e.id_to).map(|n| &n.path) != Some(&e.to)
        {
            return Err("schema: endpoints incoerentes com IDs".into());
        }
    }
    let digest = format!("sha256:{:x}", Sha256::digest(bytes));
    let unique_paths = g
        .nodes
        .iter()
        .map(|n| &n.path)
        .collect::<BTreeSet<_>>()
        .len()
        == g.nodes.len();
    let mut nodes: Vec<_> = g.nodes.iter().collect();
    nodes.sort_by_key(|n| n.id);
    let mut diagnostics = vec![
        json!({"code":"projection_policy","symbol_id":null,"message":format!("politica_nomes={nomes}; assinatura desconhecida; coverage={coverage} declarada, sem prova semântica")}),
    ];
    let mut metrics_diagnostics = vec![
        json!({"code":"counting_policy","symbol_id":null,"message":"Montante Uses (todos os subtipos), consumidores distintos, alvo excluído inclusive em ciclos; Owns não propaga. Ausência no grafo não prova ausência no programa."}),
    ];
    let mut symbols = Vec::new();
    let mut items = Vec::new();
    for n in nodes {
        let location = n
            .position
            .as_ref()
            .filter(|p| !p.file.is_empty() && p.start_line > 0)
            .map(|p| json!({"file":p.file,"line":p.start_line}));
        if nomes == "opaque" || location.is_none() {
            diagnostics.push(json!({"code":"matching_evidence_missing","symbol_id":null,"message":format!("node_id={}: nome opaco ou localização ausente; não inferir associação", n.id)}));
        }
        symbols.push(json!({"node_id":n.id,"qualified_name":if nomes=="rust-path" {Some(n.path.as_str())} else {None},"signature":null,"location":location}));
        let selected = alvos.is_none_or(|a| a.contains(&(n.id as u64)));
        let count = if unique_paths && selected {
            Some(
                calcular_raio(&g, &n.path)
                    .map_err(|e| e.to_string())?
                    .montante
                    .len(),
            )
        } else {
            metrics_diagnostics.push(json!({"code":"metric_unavailable","symbol_id":null,"message":format!("node_id={}: {}",n.id,if !unique_paths {"paths colidentes impedem raio por path"} else {"fora da seleção explícita de métricas"})}));
            None
        };
        items.push(json!({"node_id":n.id,"transitive_consumers":count}));
    }
    let impact_coverage = if !unique_paths || alvos.is_some_and(|a| a.len() != ids.len()) {
        "partial"
    } else {
        coverage
    };
    Ok(
        json!({"schema":"lente.doxygen.graph_projection","version":1,"graph_digest":digest,
        "name_policy":nomes,"declared_coverage":coverage,
        "catalog":{"schema":"lente.symbol_catalog","version":1,"graph_digest":digest,"symbols":symbols,"diagnostics":diagnostics},
        "impact":{"schema":"lente.impact_summary","version":1,"graph_digest":digest,"coverage":impact_coverage,"items":items,"diagnostics":metrics_diagnostics}}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph() -> Value {
        json!({"crate":"demo","nodes":[
        {"id":1,"path":"demo::a","name":"a","kind":"fn","visibility":"pub","position":{"file":"a.rs","start_line":2,"end_line":3}},
        {"id":2,"path":"demo::b","name":"b","kind":"fn","visibility":"pub"}],
        "edges":[{"id_from":2,"from":"demo::b","id_to":1,"to":"demo::a","relation":"uses"},
        {"id_from":1,"from":"demo::a","id_to":2,"to":"demo::b","relation":"uses"}]})
    }
    #[test]
    fn bytes_ciclos_e_fatos() {
        let b = serde_json::to_vec(&graph()).unwrap();
        let r = projetar(&b, "opaque", "unknown", None).unwrap();
        assert!(r["catalog"]["symbols"][0]["qualified_name"].is_null());
        assert_eq!(r["impact"]["items"][0]["transitive_consumers"], 1);
        assert_eq!(r["impact"]["coverage"], "unknown");
        let mut spaced = b.clone();
        spaced.push(b' ');
        assert_ne!(
            r["graph_digest"],
            projetar(&spaced, "opaque", "unknown", None).unwrap()["graph_digest"]
        );
        let r = projetar(&b, "rust-path", "partial", Some(&[1])).unwrap();
        assert_eq!(r["catalog"]["symbols"][0]["qualified_name"], "demo::a");
        assert!(r["catalog"]["symbols"][0]["signature"].is_null());
        assert!(r["impact"]["items"][1]["transitive_consumers"].is_null());
        assert!(projetar(&b, "opaque", "unknown", Some(&[9])).is_err());
    }
    #[test]
    fn precondicoes_e_determinismo() {
        let mut g = graph();
        g["nodes"][1]["path"] = json!("demo::a");
        g["edges"] = json!([]);
        let r = projetar(&serde_json::to_vec(&g).unwrap(), "opaque", "complete", None).unwrap();
        assert!(r["impact"]["items"][0]["transitive_consumers"].is_null());
        assert_eq!(r["impact"]["coverage"], "partial");
        let a = graph();
        let mut b = a.clone();
        b["nodes"].as_array_mut().unwrap().reverse();
        assert_eq!(
            projetar(&a.to_string().into_bytes(), "opaque", "unknown", None).unwrap()["impact"]["items"],
            projetar(&b.to_string().into_bytes(), "opaque", "unknown", None).unwrap()["impact"]["items"]
        );
        b["edges"][0]["from"] = json!("errado");
        assert!(projetar(b.to_string().as_bytes(), "opaque", "unknown", None).is_err());
    }
}
