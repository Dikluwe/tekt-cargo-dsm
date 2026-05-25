/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/html_renderer.md
 * @layer L3
 * @updated 2026-05-20
 */

use crate::sarif_reader::SarifFinding;
use crystalline_dsm_core::entities::dependency_graph::{
    DependencyGraph, ExternalKind, GraphNodeId, NodeKind,
};
use crystalline_dsm_core::entities::workspace::Workspace;
use crystalline_dsm_core::rules::cycle_detector::CycleReport;
use crystalline_dsm_core::rules::dsm_partitioner::PartitionedOrder;
use crystalline_dsm_core::rules::layer_violation_detector::LayerViolation;
use serde_json::json;
use std::collections::HashMap;
use std::fmt::Write as _;

const STYLE_CSS: &str = include_str!("html_renderer/style.css");
const SCRIPT_JS: &str = include_str!("html_renderer/script.js");

#[derive(Debug, thiserror::Error)]
pub enum HtmlRenderError {
    #[error("Falha ao serializar dados para JS: {message}")]
    SerializationFailed { message: String },

    #[error("Configuração inválida: {detail}")]
    #[allow(dead_code)]
    InvalidConfiguration { detail: String },
}

#[allow(clippy::too_many_arguments)]
pub fn render_dsm_html(
    graph: &DependencyGraph,
    partition: &PartitionedOrder,
    cycles: &CycleReport,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
    layer_violations: Option<&[LayerViolation]>,
    sarif_findings: Option<&[SarifFinding]>,
) -> Result<String, HtmlRenderError> {
    let data = build_html_data(
        graph,
        partition,
        workspace,
        layer_violations,
        sarif_findings,
    );
    let data_json =
        serde_json::to_string(&data).map_err(|e| HtmlRenderError::SerializationFailed {
            message: e.to_string(),
        })?;
    // Evitar quebra do </script> caso algum canonical_path contenha "</"
    let data_js_literal = data_json.replace("</", "<\\/");

    let workspace_name = workspace_display_name(workspace);
    let node_count = graph.node_count();
    let edge_count = graph.edge_count();
    let cycle_count = cycles.cycle_count();

    let mut html =
        String::with_capacity(STYLE_CSS.len() + SCRIPT_JS.len() + data_js_literal.len() + 4096);
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("<meta name=\"color-scheme\" content=\"light dark\">\n");
    let _ = writeln!(
        html,
        "<title>Crystalline DSM — {}</title>",
        html_escape(&workspace_name)
    );
    html.push_str("<style>\n");
    html.push_str(STYLE_CSS);
    html.push_str("\n</style>\n</head>\n<body class=\"dsm-root\">\n");

    let mut violations_stat = String::new();
    if let Some(violations) = layer_violations.filter(|v| !v.is_empty()) {
        let label = if violations.len() == 1 {
            "layer violation"
        } else {
            "layer violations"
        };
        let _ = write!(
            violations_stat,
            " · <span class=\"violation-count\">{} {}</span>",
            violations.len(),
            label
        );
    }
    let mut findings_stat = String::new();
    if let Some(findings) = sarif_findings.filter(|f| !f.is_empty()) {
        let label = if findings.len() == 1 {
            "lint finding"
        } else {
            "lint findings"
        };
        let _ = write!(
            findings_stat,
            " · <span class=\"finding-count\">{} {}</span>",
            findings.len(),
            label
        );
    }

    let _ = write!(
        html,
        "<header>\n\
         <h1>{}</h1>\n\
         <div class=\"metadata\">Generated at {} by crystalline-dsm v{}</div>\n\
         <div class=\"stats\">{} nodes · {} edges · {} cycles{}{}</div>\n\
         </header>\n",
        html_escape(&workspace_name),
        html_escape(generated_at),
        html_escape(tool_version),
        node_count,
        edge_count,
        cycle_count,
        violations_stat,
        findings_stat,
    );

    html.push_str(
        "<section class=\"controls\">\n\
         <fieldset>\n\
         <legend>Filters</legend>\n\
         <button id=\"toggle-external\" type=\"button\">Hide external nodes</button>\n\
         <button id=\"toggle-trivial\" type=\"button\">Show only cyclic SCCs</button>\n\
         <input type=\"search\" id=\"search\" placeholder=\"Filter nodes...\" autocomplete=\"off\">\n\
         </fieldset>\n\
         </section>\n",
    );

    html.push_str(
        "<main class=\"matrix-container\" style=\"--cell-size: 6px;\">\n\
         <div class=\"column-labels\" role=\"region\" aria-label=\"Column labels\"></div>\n\
         <div class=\"row-labels\" role=\"region\" aria-label=\"Row labels\"></div>\n\
         <canvas id=\"dsm-matrix\" role=\"img\" aria-label=\"Dependency structure matrix\"></canvas>\n\
         <div id=\"tooltip\" popover=\"manual\"></div>\n\
         </main>\n",
    );

    html.push_str(
        "<footer>\n<div class=\"legend\">\n\
         <span class=\"swatch swatch-edge\"></span> Dependency\n\
         <span class=\"swatch swatch-diagonal\"></span> Diagonal\n\
         <span class=\"swatch swatch-scc\"></span> Cyclic SCC\n\
         <span class=\"swatch swatch-violation\"></span> Layer violation\n\
         <span class=\"swatch swatch-sarif\"></span> Lint finding\n\
         </div>\n</footer>\n",
    );

    html.push_str("<script type=\"module\">\n");
    html.push_str("const GRAPH_DATA = ");
    html.push_str(&data_js_literal);
    html.push_str(";\n");
    html.push_str(SCRIPT_JS);
    html.push_str("\n</script>\n</body>\n</html>\n");

    Ok(html)
}

fn workspace_display_name(workspace: &Workspace) -> String {
    workspace
        .root
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "workspace".to_string())
}

#[derive(Default)]
struct AggEdge {
    count: usize,
    items: Vec<String>,
    has_glob: bool,
    truncated: bool,
}

fn match_uri_to_canonical_path(file_uri: &str, workspace: &Workspace) -> Option<String> {
    let normalized_uri = file_uri.replace('\\', "/");
    let parts: Vec<&str> = normalized_uri.split('/').collect();
    if parts.is_empty() {
        return None;
    }

    let first_dir = parts[0];
    let member = workspace.members.iter().find(|m| {
        m.crate_root
            .file_name()
            .and_then(|s| s.to_str())
            .map(|name| name == first_dir)
            .unwrap_or(false)
    });

    let member = member?;

    let crate_name = member.name.replace('-', "_");

    if parts.len() == 1 {
        return Some(crate_name);
    }

    let mut remainder = &parts[1..];
    if remainder.first() == Some(&"src") {
        remainder = &remainder[1..];
    }

    if remainder.is_empty() {
        return Some(crate_name);
    }

    let mut module_parts = vec![crate_name];
    for &part in remainder {
        if let Some(base) = part.strip_suffix(".rs") {
            if base != "mod" && base != "lib" && base != "main" {
                module_parts.push(base.to_string());
            }
        } else {
            module_parts.push(part.to_string());
        }
    }

    Some(module_parts.join("::"))
}

fn build_html_data(
    graph: &DependencyGraph,
    partition: &PartitionedOrder,
    workspace: &Workspace,
    layer_violations: Option<&[LayerViolation]>,
    sarif_findings: Option<&[SarifFinding]>,
) -> serde_json::Value {
    let id_to_pos: HashMap<GraphNodeId, usize> = partition
        .order
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();

    let labels: Vec<String> = partition
        .order
        .iter()
        .map(|&id| graph.node(id).canonical_path.clone())
        .collect();

    let kinds: Vec<&'static str> = partition
        .order
        .iter()
        .map(|&id| match &graph.node(id).kind {
            NodeKind::InternalWithTree { .. } | NodeKind::InternalWithoutTree { .. } => "internal",
            NodeKind::External {
                kind: ExternalKind::Crate,
            } => "external_crate",
            NodeKind::External {
                kind: ExternalKind::Stdlib,
            } => "external_stdlib",
        })
        .collect();

    let crate_names: Vec<serde_json::Value> = partition
        .order
        .iter()
        .map(|&id| match &graph.node(id).kind {
            NodeKind::InternalWithTree { crate_name, .. }
            | NodeKind::InternalWithoutTree { crate_name } => {
                serde_json::Value::String(crate_name.clone())
            }
            _ => serde_json::Value::Null,
        })
        .collect();

    // Agregar arestas por (from_pos, to_pos)
    let mut agg: HashMap<(usize, usize), AggEdge> = HashMap::new();
    for (from_id, to_id, edge) in graph.all_edges() {
        let from_pos = id_to_pos[&from_id];
        let to_pos = id_to_pos[&to_id];
        let entry = agg.entry((from_pos, to_pos)).or_default();
        entry.count += 1;
        if entry.items.len() < 5 {
            entry.items.push(edge.imported_item.clone());
        } else {
            entry.truncated = true;
        }
        if edge.is_glob {
            entry.has_glob = true;
        }
    }

    let mut edges_json: Vec<serde_json::Value> = agg
        .into_iter()
        .map(|((from, to), e)| {
            let mut obj = serde_json::Map::new();
            obj.insert("from".into(), json!(from));
            obj.insert("to".into(), json!(to));
            obj.insert("count".into(), json!(e.count));
            obj.insert("items".into(), json!(e.items));
            obj.insert("has_glob".into(), json!(e.has_glob));
            if e.truncated {
                obj.insert("truncated".into(), json!(true));
            }
            serde_json::Value::Object(obj)
        })
        .collect();
    edges_json.sort_by(|a, b| {
        let af = a["from"].as_u64().unwrap();
        let bf = b["from"].as_u64().unwrap();
        let at = a["to"].as_u64().unwrap();
        let bt = b["to"].as_u64().unwrap();
        af.cmp(&bf).then(at.cmp(&bt))
    });

    let sccs_json: Vec<serde_json::Value> = partition
        .sccs
        .iter()
        .map(|s| {
            json!({
                "start": s.range.start,
                "end": s.range.end,
                "cyclic": s.is_cyclic,
            })
        })
        .collect();

    let mut layer_violations_json = Vec::new();
    if let Some(violations) = layer_violations {
        for violation in violations {
            let from_pos = id_to_pos.get(&violation.from_node);
            let to_pos = id_to_pos.get(&violation.to_node);
            if let (Some(&from), Some(&to)) = (from_pos, to_pos) {
                layer_violations_json.push(json!({
                    "from": from,
                    "to": to,
                    "from_layer": format!("{:?}", violation.from_layer),
                    "to_layer": format!("{:?}", violation.to_layer),
                }));
            }
        }
    }

    let mut sarif_findings_json = Vec::new();
    if let Some(findings) = sarif_findings {
        for finding in findings {
            if let Some(pos) = match_uri_to_canonical_path(&finding.file_uri, workspace)
                .and_then(|target_canonical| {
                    partition
                        .order
                        .iter()
                        .find(|&&id| graph.node(id).canonical_path == target_canonical)
                })
                .and_then(|node_id| id_to_pos.get(node_id))
            {
                sarif_findings_json.push(json!({
                    "pos": pos,
                    "rule_id": finding.rule_id,
                    "level": finding.level,
                    "message": finding.message,
                    "line": finding.start_line,
                }));
            }
        }
    }

    json!({
        "schema_version": "1.0.0",
        "workspace_name": workspace_display_name(workspace),
        "internal_boundary": partition.internal_boundary,
        "labels": labels,
        "kinds": kinds,
        "crate_names": crate_names,
        "edges": edges_json,
        "sccs": sccs_json,
        "scc_per_position": partition.scc_index_per_node,
        "layer_violations": layer_violations_json,
        "sarif_findings": sarif_findings_json,
    })
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crystalline_dsm_core::entities::dependency_graph::GraphEdge;
    use crystalline_dsm_core::entities::module_tree::{ModuleTree, NodeId};
    use crystalline_dsm_core::entities::workspace::{EntryKind, WorkspaceMember};
    use crystalline_dsm_core::rules::cycle_detector::detect_cycles;
    use crystalline_dsm_core::rules::dsm_partitioner::partition_for_dsm;
    use std::path::PathBuf;

    const TOOL_VERSION: &str = "0.1.0";
    const GENERATED_AT: &str = "2026-05-20T22:30:00Z";

    /// `NodeId` válido obtido construindo uma tree dummy (a raiz é
    /// sempre `NodeId(0)`). Evita depender de `NodeId::test_new`,
    /// que é `pub(crate)` no L₁.
    fn dummy_node_id() -> NodeId {
        ModuleTree::new("dummy".into(), PathBuf::from("/")).root()
    }

    fn make_workspace(name: &str) -> Workspace {
        Workspace {
            root: PathBuf::from(format!("/tmp/{}", name)),
            members: vec![WorkspaceMember {
                name: name.to_string(),
                crate_root: PathBuf::from(format!("/tmp/{}", name)),
                entry_kind: EntryKind::Library {
                    lib_path: PathBuf::from(format!("/tmp/{}/src/lib.rs", name)),
                },
            }],
        }
    }

    fn placeholder_edge(item: &str) -> GraphEdge {
        GraphEdge {
            imported_item: item.into(),
            alias: None,
            is_reexport: false,
            is_glob: false,
            raw_use_path: item.into(),
        }
    }

    fn empty_cycles() -> CycleReport {
        CycleReport { cycles: vec![] }
    }

    // 1. render_dsm_html produz string não-vazia com substrings esperadas
    #[test]
    fn test_render_minimal_graph() {
        let mut g = DependencyGraph::new();
        g.add_internal_node_with_tree("a".into(), "a".into(), dummy_node_id());
        let p = partition_for_dsm(&g);
        let c = empty_cycles();
        let ws = make_workspace("a");
        let html =
            render_dsm_html(&g, &p, &c, &ws, TOOL_VERSION, GENERATED_AT, None, None).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("<canvas"));
        assert!(html.contains("popover=\"manual\""));
        assert!(
            html.contains("\"a\""),
            "label deve aparecer nos dados embutidos"
        );
    }

    // 2. HTML contém o workspace_name no <title> e <h1>
    #[test]
    fn test_workspace_name_in_title_and_h1() {
        let mut g = DependencyGraph::new();
        g.add_internal_node_with_tree("x".into(), "x".into(), dummy_node_id());
        let p = partition_for_dsm(&g);
        let ws = make_workspace("typst-original");
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &ws,
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        assert!(html.contains("<title>Crystalline DSM — typst-original</title>"));
        assert!(html.contains("<h1>typst-original</h1>"));
    }

    // 3. Quantidade de labels embutidos == order.len()
    #[test]
    fn test_labels_count_matches_order_len() {
        let mut g = DependencyGraph::new();
        for n in ["a", "b", "c"] {
            g.add_internal_node_with_tree(n.into(), n.into(), dummy_node_id());
        }
        let p = partition_for_dsm(&g);
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &make_workspace("w"),
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        // Os labels saem no JSON embutido: "labels":["a","b","c"]
        assert!(html.contains("\"labels\":[\"a\",\"b\",\"c\"]"));
    }

    // 4. Dados embutidos têm internal_boundary correcto
    #[test]
    fn test_internal_boundary_in_embedded_data() {
        let mut g = DependencyGraph::new();
        let a = g.add_internal_node_with_tree("A".into(), "c".into(), dummy_node_id());
        let b = g.add_internal_node_with_tree("B".into(), "c".into(), dummy_node_id());
        let x = g.add_external_node("X".into(), ExternalKind::Crate);
        g.add_edge(a, x, placeholder_edge("x")).unwrap();
        g.add_edge(b, x, placeholder_edge("x")).unwrap();
        let p = partition_for_dsm(&g);
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &make_workspace("w"),
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        assert!(
            html.contains("\"internal_boundary\":2"),
            "esperava \"internal_boundary\":2; HTML não contém"
        );
    }

    // 5. Arestas agregadas: pares (from, to) distintos
    #[test]
    fn test_aggregated_edges_count() {
        let mut g = DependencyGraph::new();
        let a = g.add_internal_node_with_tree("A".into(), "c".into(), dummy_node_id());
        let b = g.add_internal_node_with_tree("B".into(), "c".into(), dummy_node_id());
        // 3 arestas A→B (count=3), 1 aresta B→A (count=1) ⇒ 2 entradas em edges
        g.add_edge(a, b, placeholder_edge("X")).unwrap();
        g.add_edge(a, b, placeholder_edge("Y")).unwrap();
        g.add_edge(a, b, placeholder_edge("Z")).unwrap();
        g.add_edge(b, a, placeholder_edge("W")).unwrap();
        let p = partition_for_dsm(&g);
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &make_workspace("w"),
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        // Conta "count": no JSON. Deveria haver 2 (um por par).
        let occurrences = html.matches("\"count\":").count();
        assert_eq!(
            occurrences, 2,
            "esperava 2 entradas agregadas, encontrou {}",
            occurrences
        );
        assert!(html.contains("\"count\":3"));
        assert!(html.contains("\"count\":1"));
    }

    // 6. Grafo vazio: HTML válido
    #[test]
    fn test_empty_graph_renders() {
        let g = DependencyGraph::new();
        let p = partition_for_dsm(&g);
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &make_workspace("empty"),
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        assert!(html.contains("<canvas"));
        assert!(html.contains("\"labels\":[]"));
        assert!(html.contains("\"internal_boundary\":0"));
    }

    // 7. Apenas externos: internal_boundary == 0
    #[test]
    fn test_only_externals() {
        let mut g = DependencyGraph::new();
        g.add_external_node("serde".into(), ExternalKind::Crate);
        let p = partition_for_dsm(&g);
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &make_workspace("w"),
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        assert!(html.contains("\"internal_boundary\":0"));
        assert!(html.contains("\"kinds\":[\"external_crate\"]"));
    }

    // 8. CSS contém custom properties e light-dark()
    #[test]
    fn test_css_uses_light_dark_and_custom_props() {
        let g = DependencyGraph::new();
        let p = partition_for_dsm(&g);
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &make_workspace("w"),
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        assert!(html.contains("--cell-size"));
        assert!(html.contains("--bg-page"));
        assert!(html.contains("light-dark("));
    }

    // 9. Tooltip usa Popover API
    #[test]
    fn test_tooltip_uses_popover_api() {
        let g = DependencyGraph::new();
        let p = partition_for_dsm(&g);
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &make_workspace("w"),
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        assert!(html.contains("popover=\"manual\""));
        assert!(html.contains("showPopover"));
    }

    // 10. @scope (.dsm-root) está no CSS
    #[test]
    fn test_css_uses_scope() {
        let g = DependencyGraph::new();
        let p = partition_for_dsm(&g);
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &make_workspace("w"),
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        assert!(html.contains("@scope (.dsm-root)"));
    }

    // 11. Variáveis NÃO devem estar em :root { --bg-page; precisam estar
    // em .dsm-root (via :scope dentro do @scope).
    #[test]
    fn test_vars_not_in_root_selector() {
        let g = DependencyGraph::new();
        let p = partition_for_dsm(&g);
        let html = render_dsm_html(
            &g,
            &p,
            &empty_cycles(),
            &make_workspace("w"),
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();
        assert!(
            !html.contains(":root {"),
            "variáveis devem viver em .dsm-root (via :scope), não em :root"
        );
    }

    // 12. Pipeline completo contra fixture imports-simple
    #[test]
    fn test_pipeline_end_to_end_imports_simple() {
        use crate::cargo_metadata_reader::read_workspace;
        use crate::import_extractor::extract_imports;
        use crate::module_traverser::traverse_crate;

        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .unwrap()
            .join("tests/fixtures/imports-simple");
        if !root.exists() {
            // Fixture pode estar ausente em alguns checkouts; skip silencioso.
            return;
        }
        let ws = read_workspace(&root).unwrap();

        // Construir trees e edges manualmente (sem depender de build_graph que vive em L4).
        let mut trees = std::collections::HashMap::new();
        for m in &ws.members {
            if let Ok(t) = traverse_crate(m) {
                trees.insert(m.name.clone(), t);
            }
        }
        let names: Vec<String> = ws.members.iter().map(|m| m.name.clone()).collect();
        let mut all_edges_for_graph: Vec<(GraphNodeId, GraphNodeId, GraphEdge)> = Vec::new();

        // Construção minimalista: adiciona nós internos por tree, sem chamar build_graph.
        let mut graph = DependencyGraph::new();
        for (crate_name, tree) in &trees {
            for (node_id, m) in tree.all_nodes() {
                graph.add_internal_node_with_tree(
                    m.canonical_path.clone(),
                    crate_name.clone(),
                    node_id,
                );
            }
        }
        for (crate_name, tree) in &trees {
            let member = ws.find_member(crate_name).unwrap();
            if let Ok(edges) = extract_imports(member, tree, &names) {
                for e in edges {
                    let from = tree.node(e.from);
                    if let Some(from_id) = graph.find_node(&from.canonical_path) {
                        let to_path = if e.target_module.is_empty() {
                            e.imported_item.clone()
                        } else {
                            e.target_module.clone()
                        };
                        let to_id = graph.find_node(&to_path).unwrap_or_else(|| {
                            graph.add_external_node(to_path, ExternalKind::Crate)
                        });
                        all_edges_for_graph.push((
                            from_id,
                            to_id,
                            GraphEdge {
                                imported_item: e.imported_item.clone(),
                                alias: e.alias.clone(),
                                is_reexport: e.is_reexport,
                                is_glob: e.is_glob,
                                raw_use_path: e.raw_use_path.clone(),
                            },
                        ));
                    }
                }
            }
        }
        for (f, t, e) in all_edges_for_graph {
            let _ = graph.add_edge(f, t, e);
        }
        let cycles = detect_cycles(&graph);
        let partition = partition_for_dsm(&graph);
        let html = render_dsm_html(
            &graph,
            &partition,
            &cycles,
            &ws,
            TOOL_VERSION,
            GENERATED_AT,
            None,
            None,
        )
        .unwrap();

        assert!(html.contains("<h1>imports-simple</h1>"));
        assert!(html.contains("type=\"module\""));
        assert!(html.contains("<canvas"));
        assert!(
            html.len() > 8_000,
            "HTML pequeno demais: {} bytes",
            html.len()
        );
    }

    #[test]
    fn test_match_uri_to_canonical_path_helper() {
        let ws = Workspace {
            root: PathBuf::from("/workspace"),
            members: vec![WorkspaceMember {
                name: "crystalline-dsm-core".to_string(),
                crate_root: PathBuf::from("/workspace/01_core"),
                entry_kind: EntryKind::Library {
                    lib_path: PathBuf::from("/workspace/01_core/src/lib.rs"),
                },
            }],
        };

        // 1. Arquivo de modulo normal
        assert_eq!(
            match_uri_to_canonical_path("01_core/src/entities/layer_config.rs", &ws),
            Some("crystalline_dsm_core::entities::layer_config".to_string())
        );

        // 2. Raiz do crate
        assert_eq!(
            match_uri_to_canonical_path("01_core/src/lib.rs", &ws),
            Some("crystalline_dsm_core".to_string())
        );

        // 3. mod.rs
        assert_eq!(
            match_uri_to_canonical_path("01_core/src/entities/mod.rs", &ws),
            Some("crystalline_dsm_core::entities".to_string())
        );

        // 4. Crate não mapeado
        assert_eq!(
            match_uri_to_canonical_path("02_shell/src/lib.rs", &ws),
            None
        );
    }

    #[test]
    fn test_render_with_violations_and_findings() {
        use crystalline_dsm_core::entities::layer_config::Layer;
        let mut g = DependencyGraph::new();
        let from_id = g.add_internal_node_with_tree(
            "crystalline_dsm_core::entities::layer_config".into(),
            "crystalline-dsm-core".into(),
            dummy_node_id(),
        );
        let to_id = g.add_internal_node_with_tree(
            "crystalline_dsm_core::entities".into(),
            "crystalline-dsm-core".into(),
            dummy_node_id(),
        );

        let p = partition_for_dsm(&g);
        let c = empty_cycles();

        let ws = Workspace {
            root: PathBuf::from("/workspace"),
            members: vec![WorkspaceMember {
                name: "crystalline-dsm-core".to_string(),
                crate_root: PathBuf::from("/workspace/01_core"),
                entry_kind: EntryKind::Library {
                    lib_path: PathBuf::from("/workspace/01_core/src/lib.rs"),
                },
            }],
        };

        let violations = vec![LayerViolation {
            from_node: from_id,
            to_node: to_id,
            from_layer: Layer::L1,
            to_layer: Layer::L3,
        }];

        let findings = vec![SarifFinding {
            rule_id: "V9".to_string(),
            level: "error".to_string(),
            message: "Layer violation detected".to_string(),
            file_uri: "01_core/src/entities/layer_config.rs".to_string(),
            start_line: Some(10),
        }];

        let html = render_dsm_html(
            &g,
            &p,
            &c,
            &ws,
            TOOL_VERSION,
            GENERATED_AT,
            Some(&violations),
            Some(&findings),
        )
        .unwrap();

        // Verificar contadores no HTML
        assert!(html.contains("1 layer violation"));
        assert!(html.contains("1 lint finding"));

        // Verificar dados embutidos
        assert!(html.contains("\"layer_violations\":[{\"from\":0,\"from_layer\":\"L1\",\"to\":1,\"to_layer\":\"L3\"}]") ||
                html.contains("\"layer_violations\":[{\"from\":1,\"from_layer\":\"L1\",\"to\":0,\"to_layer\":\"L3\"}]"));
        assert!(
            html.contains("\"sarif_findings\":[")
                && html.contains("\"rule_id\":\"V9\"")
                && html.contains("\"level\":\"error\"")
        );
        assert!(html.contains("<span class=\"swatch swatch-violation\"></span> Layer violation"));
        assert!(html.contains("<span class=\"swatch swatch-sarif\"></span> Lint finding"));
    }
}
