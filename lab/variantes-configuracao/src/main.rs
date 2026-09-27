//! Crystalline Lineage
//! @prompt lab/variantes-configuracao/CONTRATO.md
//! @updated 2026-09-26
//! Experimento isolado: nenhuma invocação de extratores ou API de comparação.
#![forbid(unsafe_code)]

use lente_core::entities::grafo::{Grafo, Relation, UsesKind};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::Path,
};
type Result<T> = std::result::Result<T, String>;
type Support = BTreeMap<Edge, BTreeSet<String>>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum State {
    Complete,
    Partial,
    #[default]
    Unknown,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mapping {
    node_id: usize,
    identity: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputVariant {
    id: String,
    graph: String,
    granularity: String,
    nature: String,
    #[serde(default)]
    extraction: State,
    #[serde(default)]
    diagnostics: Vec<String>,
    identity_map: Vec<Mapping>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Edge {
    from: String,
    to: String,
    relation: String,
    uses_kind: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    id: String,
    edges: Vec<Edge>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    version: u32,
    project: String,
    configurations: Vec<InputVariant>,
    #[serde(default)]
    paths: Vec<Query>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SourceNode {
    identity: String,
    source_id: usize,
    source_path: String,
    source_kind: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Variant {
    id: String,
    granularity: String,
    nature: String,
    extraction: State,
    diagnostics: Vec<String>,
    nodes: Vec<SourceNode>,
    relations: Vec<Edge>,
}
#[derive(Debug, Serialize)]
struct Provenance {
    observed_in: Vec<String>,
    observation_class: String,
    not_observed_in_complete: Vec<String>,
    not_observed_in_partial_or_unknown: Vec<String>,
}
#[derive(Debug, Serialize)]
struct UnionNode {
    identity: String,
    #[serde(flatten)]
    provenance: Provenance,
}
#[derive(Debug, Serialize)]
struct UnionEdge {
    #[serde(flatten)]
    edge: Edge,
    #[serde(flatten)]
    provenance: Provenance,
}
#[derive(Debug, Serialize)]
struct Witness {
    edges: Vec<Edge>,
    common_configurations: Vec<String>,
    observation: String,
}
#[derive(Debug, Serialize)]
struct PathResult {
    id: String,
    #[serde(flatten)]
    witness: Witness,
}
#[derive(Debug, Serialize)]
struct Analysis {
    node_count: usize,
    relation_count: usize,
    uses_relation_count: usize,
    cyclic_sccs: Vec<Vec<String>>,
    condensation_order: Vec<String>,
    cycle_examples: Vec<Witness>,
    cycle_examples_exhaustive: bool,
}
#[derive(Debug, Serialize)]
struct VariantAnalysis {
    #[serde(flatten)]
    variant: Variant,
    analysis: Analysis,
}
#[derive(Debug, Serialize)]
struct Union {
    view: String,
    granularity: String,
    nature: String,
    nodes: Vec<UnionNode>,
    relations: Vec<UnionEdge>,
    analysis: Analysis,
}
#[derive(Debug, Serialize)]
struct Report {
    version: u32,
    project: String,
    configurations: Vec<String>,
    per_configuration: Vec<VariantAnalysis>,
    union: Union,
    paths: Vec<PathResult>,
    diagnostics: Vec<String>,
    limits: Vec<String>,
}
fn nonempty(s: &str) -> bool {
    !s.trim().is_empty()
}
fn normalize(input: &InputVariant, graph: Grafo) -> Result<Variant> {
    if !nonempty(&input.id) {
        return Err("identificador de configuração vazio".into());
    }
    let fail = |s: &str| format!("configuração {}: {s}", input.id);
    if !matches!(
        input.granularity.as_str(),
        "file" | "module" | "package" | "item"
    ) {
        return Err(fail("granularidade não suportada ou desconhecida"));
    }
    if !nonempty(&input.nature) || input.nature.trim().eq_ignore_ascii_case("unknown") {
        return Err(fail("natureza das relações deve ser explícita"));
    }
    let ids: BTreeSet<_> = graph.nodes.iter().map(|n| n.id).collect();
    let mut map = BTreeMap::new();
    let mut reverse = BTreeMap::new();
    for m in &input.identity_map {
        if !ids.contains(&m.node_id) || !nonempty(&m.identity) {
            return Err(fail("mapa contém ID inexistente ou identidade vazia"));
        }
        if let Some(old) = map.insert(m.node_id, m.identity.clone())
            && old != m.identity
        {
            return Err(fail(
                "identidade ambígua: mesmo nó recebeu duas identidades",
            ));
        }
        if let Some(old) = reverse.insert(m.identity.clone(), m.node_id)
            && old != m.node_id
        {
            return Err(fail(
                "identidade ambígua: dois nós receberam a mesma identidade",
            ));
        }
    }
    if map.len() != ids.len() {
        return Err(fail(
            "mapa de identidade incompleto; correspondência não será inferida",
        ));
    }
    let mut nodes: Vec<_> = graph
        .nodes
        .iter()
        .map(|n| SourceNode {
            identity: map[&n.id].clone(),
            source_id: n.id,
            source_path: n.path.as_str().into(),
            source_kind: format!("{:?}", n.kind),
        })
        .collect();
    nodes.sort_by(|a, b| a.identity.cmp(&b.identity));
    let edges: BTreeSet<_> = graph
        .edges
        .iter()
        .map(|a| Edge {
            from: map[&a.id_from].clone(),
            to: map[&a.id_to].clone(),
            relation: match a.relation {
                Relation::Owns => "owns",
                Relation::Uses => "uses",
            }
            .into(),
            uses_kind: a.uses_kind.map(|k| {
                match k {
                    UsesKind::Import => "import",
                    UsesKind::Reference => "reference",
                }
                .into()
            }),
        })
        .collect();
    let diagnostics: BTreeSet<_> = input.diagnostics.iter().cloned().collect();
    Ok(Variant {
        id: input.id.clone(),
        granularity: input.granularity.clone(),
        nature: input.nature.clone(),
        extraction: input.extraction,
        diagnostics: diagnostics.into_iter().collect(),
        nodes,
        relations: edges.into_iter().collect(),
    })
}
fn load(path: &Path) -> Result<(Input, Vec<Variant>)> {
    let text = fs::read_to_string(path).map_err(|e| format!("entrada {}: {e}", path.display()))?;
    let input: Input =
        serde_json::from_str(&text).map_err(|e| format!("documento de entrada: {e}"))?;
    if input.version != 1 {
        return Err("versão de entrada incompatível; suportada: 1".into());
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let mut variants = Vec::new();
    for v in &input.configurations {
        let file = base.join(&v.graph);
        let text = fs::read_to_string(&file)
            .map_err(|e| format!("configuração {}, grafo {}: {e}", v.id, file.display()))?;
        let graph = lente_infra::desserializar_grafo(&text)
            .map_err(|e| format!("configuração {}, grafo rejeitado pela Lente: {e}", v.id))?;
        variants.push(normalize(v, graph)?);
    }
    Ok((input, variants))
}
fn provenance(observed: &BTreeSet<String>, variants: &BTreeMap<String, Variant>) -> Provenance {
    let mut complete = Vec::new();
    let mut uncertain = Vec::new();
    for (id, v) in variants {
        if !observed.contains(id) {
            if v.extraction == State::Complete {
                complete.push(id.clone());
            } else {
                uncertain.push(id.clone());
            }
        }
    }
    Provenance {
        observed_in: observed.iter().cloned().collect(),
        observation_class: if observed.len() == variants.len() {
            "all-provided-configurations"
        } else {
            "some-provided-configurations"
        }
        .into(),
        not_observed_in_complete: complete,
        not_observed_in_partial_or_unknown: uncertain,
    }
}
fn support(edges: &[Edge], observed: &Support) -> Result<Witness> {
    if edges.is_empty() {
        return Err("caminho vazio; forneça pelo menos uma aresta".into());
    }
    let mut common = None::<BTreeSet<String>>;
    for (i, e) in edges.iter().enumerate() {
        if e.relation != "uses" {
            return Err("caminhos de dependência aceitam somente uses".into());
        }
        if i > 0 && edges[i - 1].to != e.from {
            return Err("caminho não contíguo".into());
        }
        let sources = observed.get(e).ok_or_else(|| {
            format!(
                "relação do caminho não observada na união: {} -> {} ({:?})",
                e.from, e.to, e.uses_kind
            )
        })?;
        common = Some(match common {
            None => sources.clone(),
            Some(c) => c.intersection(sources).cloned().collect(),
        });
    }
    let common = common.unwrap();
    Ok(Witness {
        edges: edges.to_vec(),
        observation: if common.is_empty() {
            "not-observed-together-in-provided-configurations"
        } else {
            "observed-together"
        }
        .into(),
        common_configurations: common.into_iter().collect(),
    })
}
// One-to-one structural projection: no module aggregation, path inference or metadata fusion.
fn projection(nodes: &BTreeSet<String>, edges: &[Edge]) -> Result<Grafo> {
    let ids: BTreeMap<_, _> = nodes.iter().enumerate().map(|(i, n)| (n, i)).collect();
    let graph = json!({"crate":"structural-projection-only",
        "nodes":nodes.iter().enumerate().map(|(id,n)|json!({"id":id,"path":n,"name":n,"kind":"mod","visibility":"priv"})).collect::<Vec<_>>(),
        "edges":edges.iter().map(|e|json!({"id_from":ids[&e.from],"from":e.from,"id_to":ids[&e.to],"to":e.to,"relation":e.relation,"uses_kind":e.uses_kind})).collect::<Vec<_>>()});
    lente_infra::desserializar_grafo(&graph.to_string())
        .map_err(|e| format!("projeção estrutural: {e}"))
}
fn example(scc: &[String], edges: &[Edge]) -> Result<Vec<Edge>> {
    let members: BTreeSet<_> = scc.iter().collect();
    let start = &scc[0];
    let first = edges
        .iter()
        .find(|e| {
            e.relation == "uses" && &e.from == start && &e.to != start && members.contains(&e.to)
        })
        .ok_or("SCC sem saída interna")?;
    let mut queue = VecDeque::from([first.to.clone()]);
    let mut seen = BTreeSet::from([first.to.clone()]);
    let mut predecessor = BTreeMap::<String, Edge>::new();
    let mut adjacency = BTreeMap::<&str, Vec<&Edge>>::new();
    for e in edges
        .iter()
        .filter(|e| e.relation == "uses" && members.contains(&e.from) && members.contains(&e.to))
    {
        adjacency.entry(&e.from).or_default().push(e);
    }
    while let Some(node) = queue.pop_front() {
        if &node == start {
            break;
        }
        for e in adjacency.get(node.as_str()).into_iter().flatten() {
            if seen.insert(e.to.clone()) {
                predecessor.insert(e.to.clone(), (*e).clone());
                queue.push_back(e.to.clone());
            }
        }
    }
    let mut tail = Vec::new();
    let mut cursor = start.clone();
    while cursor != first.to {
        let e = predecessor
            .get(&cursor)
            .ok_or("SCC sem caminho de retorno")?;
        tail.push(e.clone());
        cursor = e.from.clone();
    }
    tail.reverse();
    let mut result = vec![first.clone()];
    result.extend(tail);
    Ok(result)
}
fn analyze(nodes: &BTreeSet<String>, edges: &[Edge], observed: &Support) -> Result<Analysis> {
    let g = projection(nodes, edges)?;
    let mut sccs: Vec<Vec<String>> = lente_estrutura::detectar_ciclos(&g)
        .iter()
        .map(|c| c.modulos.iter().map(|n| n.as_str().into()).collect())
        .collect();
    let mut cycles = Vec::new();
    for scc in &sccs {
        cycles.push(example(scc, edges)?);
    }
    for edge in edges
        .iter()
        .filter(|e| e.relation == "uses" && e.from == e.to)
    {
        cycles.push(vec![edge.clone()]);
        if !sccs.iter().any(|s| s.contains(&edge.from)) {
            sccs.push(vec![edge.from.clone()]);
        }
    }
    sccs.sort();
    cycles.sort();
    cycles.dedup();
    Ok(Analysis {
        node_count: nodes.len(),
        relation_count: edges.len(),
        uses_relation_count: edges.iter().filter(|e| e.relation == "uses").count(),
        cyclic_sccs: sccs,
        condensation_order: lente_estrutura::ordenar_dsm(&g)
            .ordem
            .iter()
            .map(|n| n.as_str().into())
            .collect(),
        cycle_examples: cycles
            .iter()
            .map(|c| support(c, observed))
            .collect::<Result<_>>()?,
        cycle_examples_exhaustive: false,
    })
}
fn compare(input: &Input, data: Vec<Variant>) -> Result<Report> {
    if input.version != 1 || !nonempty(&input.project) || data.is_empty() {
        return Err("exige version=1, project não vazio e configurações".into());
    }
    let mut variants = BTreeMap::new();
    for variant in data {
        if let Some(old) = variants.get(&variant.id) {
            if old != &variant {
                return Err(format!(
                    "configuração {} repetida com conteúdo divergente",
                    variant.id
                ));
            }
        } else {
            variants.insert(variant.id.clone(), variant);
        }
    }
    let first = variants.values().next().unwrap();
    let (granularity, nature) = (first.granularity.clone(), first.nature.clone());
    for v in variants.values() {
        if v.granularity != granularity || v.nature != nature {
            return Err(format!(
                "configurações incompatíveis: {} ({}/{}) e {} ({}/{}); separe as amostras",
                first.id, granularity, nature, v.id, v.granularity, v.nature
            ));
        }
    }
    let mut observed = Support::new();
    let mut node_observed = BTreeMap::<String, BTreeSet<String>>::new();
    let mut diagnostics = BTreeSet::new();
    for v in variants.values() {
        for n in &v.nodes {
            node_observed
                .entry(n.identity.clone())
                .or_default()
                .insert(v.id.clone());
        }
        for e in &v.relations {
            observed.entry(e.clone()).or_default().insert(v.id.clone());
            if e.relation == "uses" && e.uses_kind.is_none() {
                diagnostics.insert(format!(
                    "{}: subtipo de uses desconhecido; mantido separado de import/reference",
                    v.id
                ));
            }
        }
        if v.extraction != State::Complete {
            diagnostics.insert(format!(
                "{}: extração {:?}; não observação não prova ausência",
                v.id, v.extraction
            ));
        }
    }
    let union_nodes: BTreeSet<_> = node_observed.keys().cloned().collect();
    let union_edges: Vec<_> = observed.keys().cloned().collect();
    let union_analysis = analyze(&union_nodes, &union_edges, &observed)?;
    let mut queries = BTreeMap::new();
    for q in &input.paths {
        if !nonempty(&q.id) {
            return Err("id de caminho vazio".into());
        }
        if let Some(old) = queries.insert(q.id.clone(), q.clone())
            && old != *q
        {
            return Err(format!(
                "consulta {} repetida com conteúdo divergente",
                q.id
            ));
        }
    }
    let paths = queries
        .values()
        .map(|q| {
            Ok(PathResult {
                id: q.id.clone(),
                witness: support(&q.edges, &observed)
                    .map_err(|e| format!("caminho {}: {e}", q.id))?,
            })
        })
        .collect::<Result<_>>()?;
    let per_configuration = variants
        .values()
        .map(|v| {
            let nodes = v.nodes.iter().map(|n| n.identity.clone()).collect();
            Ok(VariantAnalysis {
                variant: v.clone(),
                analysis: analyze(&nodes, &v.relations, &observed)?,
            })
        })
        .collect::<Result<_>>()?;
    Ok(Report {version:1,project:input.project.clone(),configurations:variants.keys().cloned().collect(),per_configuration,
        union:Union {view:"union-of-possibilities".into(),granularity,nature,
            nodes:node_observed.iter().map(|(n,c)|UnionNode{identity:n.clone(),provenance:provenance(c,&variants)}).collect(),
            relations:observed.iter().map(|(e,c)|UnionEdge{edge:e.clone(),provenance:provenance(c,&variants)}).collect(),analysis:union_analysis},
        paths,diagnostics:diagnostics.into_iter().collect(),limits:vec![
            "Todas significa somente todas as configurações fornecidas; a amostra não cobre necessariamente todas as possíveis.".into(),
            "Não observação, sobretudo em extração parcial/desconhecida, não prova ausência.".into(),
            "União de possibilidades não representa uma configuração executada; coocorrência não prova ativação runtime.".into(),
            "SCC é componente, não um único ciclo testemunhado. Exemplos de ciclos não são exaustivos.".into(),
            "Interseção vazia refere-se ao caminho concreto consultado, não a todos os caminhos/ciclos do SCC.".into(),
            "Identidade, projeto, completude, granularidade e natureza são declarações explícitas do usuário.".into(),
        ]})
}
fn witness_text(w: &Witness) -> String {
    let route = std::iter::once(w.edges[0].from.as_str())
        .chain(w.edges.iter().map(|e| e.to.as_str()))
        .collect::<Vec<_>>()
        .join(" -> ");
    let support = if w.common_configurations.is_empty() {
        "nenhuma configuração fornecida observou este percurso integralmente".into()
    } else {
        format!(
            "observado integralmente em [{}]",
            w.common_configurations.join(", ")
        )
    };
    let kinds = w
        .edges
        .iter()
        .map(|e| e.uses_kind.as_deref().unwrap_or("unknown"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{route} (subtipos: {kinds}): {support}")
}
fn structure_text(out: &mut String, a: &Analysis) {
    out.push_str(&format!(
        "  {} nós, {} relações ({} uses); {} SCCs cíclicos.\n",
        a.node_count,
        a.relation_count,
        a.uses_relation_count,
        a.cyclic_sccs.len()
    ));
    for scc in &a.cyclic_sccs {
        out.push_str(&format!(
            "  SCC, conjunto de membros: [{}]\n",
            scc.join(", ")
        ));
    }
    for c in &a.cycle_examples {
        out.push_str(&format!("  Exemplo de ciclo: {}\n", witness_text(c)));
    }
    out.push_str(&format!(
        "  Ordem da condensação: [{}]\n",
        a.condensation_order.join(", ")
    ));
}
fn provenance_text(p: &Provenance) -> String {
    format!(
        "observada em [{}] ({}); não observada em completas [{}]; desconhecido em parciais/desconhecidas [{}]",
        p.observed_in.join(", "),
        if p.observation_class == "all-provided-configurations" {
            "todas as fornecidas"
        } else {
            "algumas das fornecidas"
        },
        p.not_observed_in_complete.join(", "),
        p.not_observed_in_partial_or_unknown.join(", ")
    )
}
fn render_text(r: &Report) -> String {
    let mut out = format!(
        "Projeto declarado: {}\nConfigurações fornecidas: [{}]\nGranularidade: {}; natureza: {}\n",
        r.project,
        r.configurations.join(", "),
        r.union.granularity,
        r.union.nature
    );
    for v in &r.per_configuration {
        out.push_str(&format!(
            "\nConfiguração {} — extração {:?}\n",
            v.variant.id, v.variant.extraction
        ));
        for d in &v.variant.diagnostics {
            out.push_str(&format!("  Diagnóstico original: {d}\n"));
        }
        for n in &v.variant.nodes {
            out.push_str(&format!(
                "  Nó {} = id {} / {} ({})\n",
                n.identity, n.source_id, n.source_path, n.source_kind
            ));
        }
        for e in &v.variant.relations {
            out.push_str(&format!(
                "  {} -> {} [{}, {}]\n",
                e.from,
                e.to,
                e.relation,
                e.uses_kind.as_deref().unwrap_or("unknown")
            ));
        }
        structure_text(&mut out, &v.analysis);
    }
    out.push_str("\nUNIÃO DE POSSIBILIDADES — não é uma configuração executada\n");
    for n in &r.union.nodes {
        out.push_str(&format!(
            "  Nó {}: {}\n",
            n.identity,
            provenance_text(&n.provenance)
        ));
    }
    for e in &r.union.relations {
        out.push_str(&format!(
            "  {} -> {} [{}, {}]: {}\n",
            e.edge.from,
            e.edge.to,
            e.edge.relation,
            e.edge.uses_kind.as_deref().unwrap_or("unknown"),
            provenance_text(&e.provenance)
        ));
    }
    structure_text(&mut out, &r.union.analysis);
    for p in &r.paths {
        out.push_str(&format!(
            "\nCaminho {}: {}\n",
            p.id,
            witness_text(&p.witness)
        ));
    }
    for d in &r.diagnostics {
        out.push_str(&format!("Diagnóstico: {d}\n"));
    }
    for limit in &r.limits {
        out.push_str(&format!("Limite: {limit}\n"));
    }
    out
}
const HELP: &str = "lente-variantes-configuracao INPUT.json [--format json|text]\nLê grafos existentes e mapas explícitos; não executa extração. Saída em stdout.\n";
fn execute(args: &[String]) -> Result<String> {
    if args.is_empty() || args == ["--help"] {
        return Ok(HELP.into());
    }
    let format = match args {
        [_] => "json",
        [_, flag, format] if flag == "--format" && matches!(format.as_str(), "json" | "text") => {
            format.as_str()
        }
        _ => return Err("uso inválido; consulte --help".into()),
    };
    let (input, variants) = load(Path::new(&args[0]))?;
    let report = compare(&input, variants)?;
    if format == "text" {
        Ok(render_text(&report))
    } else {
        serde_json::to_string_pretty(&report)
            .map(|s| s + "\n")
            .map_err(|e| e.to_string())
    }
}
fn main() {
    match execute(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{}", json!({"error":error}));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> (Input, Vec<Variant>) {
        load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join(name),
        )
        .unwrap()
    }
    fn report(name: &str) -> Report {
        let (i, v) = fixture(name);
        compare(&i, v).unwrap()
    }
    fn edge(from: &str, to: &str) -> Edge {
        Edge {
            from: from.into(),
            to: to.into(),
            relation: "uses".into(),
            uses_kind: Some("import".into()),
        }
    }
    fn serialized(r: &Report) -> String {
        serde_json::to_string(r).unwrap()
    }
    #[test]
    fn opposing_edges_only_in_union_are_not_a_witnessed_cycle() {
        let r = report("union-only.json");
        assert!(
            r.per_configuration
                .iter()
                .all(|v| v.analysis.cyclic_sccs.is_empty())
        );
        assert_eq!(r.union.analysis.cyclic_sccs, vec![vec!["X", "Y"]]);
        let w = &r.union.analysis.cycle_examples[0];
        assert_eq!(w.edges, vec![edge("X", "Y"), edge("Y", "X")]);
        assert!(w.common_configurations.is_empty());
        assert_eq!(
            w.observation,
            "not-observed-together-in-provided-configurations"
        );
        assert_eq!(r.union.view, "union-of-possibilities");
    }
    #[test]
    fn actual_cycle_is_witnessed_in_configuration_c() {
        let r = report("witnessed.json");
        assert!(r.per_configuration[0].analysis.cyclic_sccs.is_empty());
        let c = &r.per_configuration[1];
        assert_eq!(c.variant.id, "C");
        assert_eq!(c.analysis.cyclic_sccs, vec![vec!["X", "Y"]]);
        assert_eq!(
            c.analysis.cycle_examples[0].common_configurations,
            vec!["C"]
        );
        assert_eq!(
            r.union.analysis.cycle_examples[0].common_configurations,
            vec!["C"]
        );
    }
    #[test]
    fn paths_with_and_without_common_configuration() {
        let r = report("union-only.json");
        let yes = &r.paths[0];
        assert_eq!(yes.id, "observed-path");
        assert_eq!(yes.witness.common_configurations, vec!["A"]);
        let no = &r.paths[1];
        assert!(no.witness.common_configurations.is_empty());
    }
    #[test]
    fn node_present_only_in_one_variant_preserves_provenance() {
        let r = report("union-only.json");
        let z = r.union.nodes.iter().find(|n| n.identity == "Z").unwrap();
        assert_eq!(z.provenance.observed_in, vec!["A"]);
        assert_eq!(
            z.provenance.observation_class,
            "some-provided-configurations"
        );
        assert_eq!(z.provenance.not_observed_in_complete, vec!["B"]);
        assert!(
            !r.per_configuration[1]
                .variant
                .nodes
                .iter()
                .any(|n| n.identity == "Z")
        );
    }
    #[test]
    fn partial_and_unknown_extractions_never_prove_absence() {
        let (i, mut v) = fixture("partial-sample.json");
        let r = compare(&i, v.clone()).unwrap();
        let xy = r
            .union
            .relations
            .iter()
            .find(|e| e.edge == edge("X", "Y"))
            .unwrap();
        assert_eq!(xy.provenance.not_observed_in_partial_or_unknown, vec!["B"]);
        assert!(xy.provenance.not_observed_in_complete.is_empty());
        assert_eq!(
            r.per_configuration[1].variant.diagnostics,
            vec!["Falha ao ler dependências de Y."]
        );
        assert_eq!(r.per_configuration[1].variant.extraction, State::Partial);
        v[1].extraction = State::Unknown;
        let r = compare(&i, v).unwrap();
        assert_eq!(
            r.union.relations[0]
                .provenance
                .not_observed_in_partial_or_unknown,
            vec!["B"]
        );
        assert!(render_text(&r).contains("não prova ausência"));
    }
    #[test]
    fn all_means_only_all_provided_not_universal() {
        let r = report("witnessed.json");
        let xy = r
            .union
            .relations
            .iter()
            .find(|e| e.edge == edge("X", "Y"))
            .unwrap();
        assert_eq!(xy.provenance.observed_in, vec!["A", "C"]);
        assert_eq!(
            xy.provenance.observation_class,
            "all-provided-configurations"
        );
        assert!(!serialized(&r).contains("universal"));
        assert!(render_text(&r).contains("todas as fornecidas"));
    }
    #[test]
    fn ambiguous_missing_and_foreign_identities_are_rejected() {
        let (i, _) = fixture("union-only.json");
        let graph_text =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/a.json"))
                .unwrap();
        let graph = lente_infra::desserializar_grafo(&graph_text).unwrap();
        let mut spec = i.configurations[0].clone();
        spec.identity_map[1].identity = "X".into();
        assert!(
            normalize(&spec, graph.clone())
                .unwrap_err()
                .contains("ambígua")
        );
        let mut spec = i.configurations[0].clone();
        spec.identity_map.push(Mapping {
            node_id: 0,
            identity: "other".into(),
        });
        assert!(
            normalize(&spec, graph.clone())
                .unwrap_err()
                .contains("ambígua")
        );
        let mut spec = i.configurations[0].clone();
        spec.identity_map.pop();
        assert!(
            normalize(&spec, graph.clone())
                .unwrap_err()
                .contains("incompleto")
        );
        spec.identity_map.push(Mapping {
            node_id: 99,
            identity: "Z".into(),
        });
        assert!(normalize(&spec, graph).unwrap_err().contains("inexistente"));
    }
    #[test]
    fn incompatible_granularity_and_nature_have_diagnostics() {
        let (i, v) = fixture("incompatible.json");
        let e = compare(&i, v).unwrap_err();
        assert!(e.contains("incompatíveis"));
        assert!(e.contains("file"));
        assert!(e.contains("package"));
        let (i, mut v) = fixture("union-only.json");
        v[1].nature = "resolved-reference".into();
        assert!(compare(&i, v).unwrap_err().contains("resolved-reference"));
    }
    #[test]
    fn repetition_and_permutation_are_idempotent() {
        let (mut i, mut variants) = fixture("union-only.json");
        let expected = serialized(&compare(&i, variants.clone()).unwrap());
        variants.reverse();
        variants.push(variants[0].clone());
        i.paths.reverse();
        i.paths.push(i.paths[0].clone());
        assert_eq!(expected, serialized(&compare(&i, variants).unwrap()));
        // Permute the source nodes, edges, and explicit map, including duplicate edges/maps.
        let graph_text =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/a.json"))
                .unwrap();
        let mut g = lente_infra::desserializar_grafo(&graph_text).unwrap();
        let mut spec = i.configurations[0].clone();
        let original = normalize(&spec, g.clone()).unwrap();
        g.nodes.reverse();
        g.edges.reverse();
        g.edges.push(g.edges[0].clone());
        spec.identity_map.reverse();
        spec.identity_map.push(spec.identity_map[0].clone());
        assert_eq!(original, normalize(&spec, g).unwrap());
    }
    #[test]
    fn same_configuration_id_cannot_silently_union_distinct_graphs() {
        let (i, mut v) = fixture("union-only.json");
        v[1].id = "A".into();
        assert!(compare(&i, v).unwrap_err().contains("conteúdo divergente"));
    }
    #[test]
    fn scc_is_not_one_cycle_through_every_member() {
        let (mut i, mut v) = fixture("union-only.json");
        i.paths.clear();
        v.truncate(1);
        v[0].relations = vec![
            edge("X", "Y"),
            edge("Y", "X"),
            edge("Y", "Z"),
            edge("Z", "Y"),
        ];
        let r = compare(&i, v).unwrap();
        let a = &r.union.analysis;
        assert_eq!(a.cyclic_sccs, vec![vec!["X", "Y", "Z"]]);
        assert_eq!(a.cycle_examples.len(), 1);
        assert_eq!(
            a.cycle_examples[0].edges,
            vec![edge("X", "Y"), edge("Y", "X")]
        );
        assert!(!a.cycle_examples_exhaustive);
    }
    #[test]
    fn union_example_does_not_negate_a_different_observed_cycle_in_same_scc() {
        let (mut i, mut v) = fixture("union-only.json");
        i.paths.clear();
        v[0].relations = vec![edge("X", "Y"), edge("Y", "Z"), edge("Z", "Y")];
        let r = compare(&i, v).unwrap();
        assert_eq!(r.union.analysis.cyclic_sccs, vec![vec!["X", "Y", "Z"]]);
        assert!(
            r.union.analysis.cycle_examples[0]
                .common_configurations
                .is_empty()
        );
        let actual = &r.per_configuration[0].analysis.cycle_examples[0];
        assert_eq!(actual.edges, vec![edge("Y", "Z"), edge("Z", "Y")]);
        assert_eq!(actual.common_configurations, vec!["A"]);
    }
    #[test]
    fn self_loops_are_not_lost_by_scc_algorithm() {
        let (mut i, mut v) = fixture("union-only.json");
        i.paths.clear();
        v.truncate(1);
        v[0].relations = vec![edge("X", "X")];
        let r = compare(&i, v).unwrap();
        assert_eq!(r.union.analysis.cyclic_sccs, vec![vec!["X"]]);
        assert_eq!(
            r.union.analysis.cycle_examples[0].edges,
            vec![edge("X", "X")]
        );
    }
    #[test]
    fn parallel_subtypes_and_containment_are_not_silently_combined() {
        let (mut i, mut v) = fixture("union-only.json");
        i.paths.clear();
        v[0].relations = vec![edge("X", "Y")];
        let mut reference = edge("X", "Y");
        reference.uses_kind = Some("reference".into());
        let unknown = Edge {
            uses_kind: None,
            ..edge("X", "Y")
        };
        let owns = Edge {
            relation: "owns".into(),
            uses_kind: None,
            ..edge("Y", "X")
        };
        v[1].relations = vec![reference, unknown, owns];
        v[1].relations.sort();
        let r = compare(&i, v).unwrap();
        assert_eq!(r.union.relations.len(), 4);
        assert!(
            r.union
                .relations
                .iter()
                .all(|e| e.provenance.observation_class == "some-provided-configurations")
        );
        assert!(r.union.analysis.cyclic_sccs.is_empty());
        assert!(!r.diagnostics.is_empty());
    }
    #[test]
    fn missing_disconnected_empty_and_owns_paths_are_errors() {
        let mut s = Support::new();
        s.insert(edge("X", "Y"), BTreeSet::from(["A".into()]));
        s.insert(edge("Z", "X"), BTreeSet::from(["A".into()]));
        assert!(support(&[], &s).is_err());
        assert!(support(&[edge("X", "Z")], &s).is_err());
        assert!(
            support(&[edge("X", "Y"), edge("Z", "X")], &s)
                .unwrap_err()
                .contains("contíguo")
        );
        assert!(
            support(
                &[Edge {
                    relation: "owns".into(),
                    ..edge("X", "Y")
                }],
                &s
            )
            .is_err()
        );
    }
    #[test]
    fn duplicate_original_paths_are_resolved_only_by_explicit_ids() {
        let (i, _) = fixture("union-only.json");
        let graph = json!({"crate":"x","nodes":[{"id":0,"path":"collision","name":"x","kind":"mod","visibility":"priv"},{"id":1,"path":"collision","name":"y","kind":"mod","visibility":"priv"}],"edges":[{"id_from":0,"id_to":1,"from":"collision","to":"collision","relation":"uses","uses_kind":"import"}]});
        let g = lente_infra::desserializar_grafo(&graph.to_string()).unwrap();
        let mut spec = i.configurations[0].clone();
        spec.identity_map.truncate(2);
        let v = normalize(&spec, g).unwrap();
        assert_eq!(v.relations, vec![edge("X", "Y")]);
    }
    #[test]
    fn text_json_and_real_cli_entrypoint_keep_union_and_uncertainty_visible() {
        let input = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/union-only.json")
            .display()
            .to_string();
        let json = execute(std::slice::from_ref(&input)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["union"]["view"], "union-of-possibilities");
        let text = execute(&[input, "--format".into(), "text".into()]).unwrap();
        assert!(text.contains("UNIÃO DE POSSIBILIDADES"));
        assert!(
            text.contains("nenhuma configuração fornecida observou este percurso integralmente")
        );
        assert!(text.contains("SCC é componente"));
        assert!(text.contains("observado integralmente em [A]"));
    }
    #[test]
    fn pairwise_support_is_insufficient_for_the_whole_path() {
        let mut s = Support::new();
        s.insert(
            edge("X", "Y"),
            ["A", "B"].into_iter().map(String::from).collect(),
        );
        s.insert(
            edge("Y", "Z"),
            ["B", "C"].into_iter().map(String::from).collect(),
        );
        s.insert(
            edge("Z", "W"),
            ["A", "C"].into_iter().map(String::from).collect(),
        );
        assert_eq!(
            support(&[edge("X", "Y"), edge("Y", "Z")], &s)
                .unwrap()
                .common_configurations,
            vec!["B"]
        );
        assert!(
            support(&[edge("X", "Y"), edge("Y", "Z"), edge("Z", "W")], &s)
                .unwrap()
                .common_configurations
                .is_empty()
        );
    }
    #[test]
    fn partial_positive_observations_are_still_preserved() {
        let (i, mut v) = fixture("witnessed.json");
        v[1].extraction = State::Partial;
        let r = compare(&i, v).unwrap();
        assert_eq!(
            r.union.analysis.cycle_examples[0].common_configurations,
            vec!["C"]
        );
        assert_eq!(r.per_configuration[1].variant.extraction, State::Partial);
    }
    #[test]
    fn empty_graph_stays_an_empty_observation_with_declared_state() {
        let (mut i, mut v) = fixture("union-only.json");
        i.paths.clear();
        v.truncate(1);
        v[0].nodes.clear();
        v[0].relations.clear();
        v[0].extraction = State::Unknown;
        let r = compare(&i, v).unwrap();
        assert_eq!(r.union.analysis.node_count, 0);
        assert!(r.union.analysis.cyclic_sccs.is_empty());
        assert!(!r.diagnostics.is_empty());
    }
    #[test]
    fn omitted_extraction_is_unknown_and_unknown_nature_is_rejected() {
        let (i, _) = fixture("union-only.json");
        let spec: InputVariant = serde_json::from_value(json!({"id":"A","graph":"a.json","granularity":"file","nature":"unknown ","identity_map":[]})).unwrap();
        assert_eq!(spec.extraction, State::Unknown);
        assert!(
            normalize(&spec, Grafo::new("empty"))
                .unwrap_err()
                .contains("natureza")
        );
        let mut unknown_granularity = i.configurations[0].clone();
        unknown_granularity.granularity = "unknown".into();
        assert!(
            normalize(&unknown_granularity, Grafo::new("empty"))
                .unwrap_err()
                .contains("granularidade")
        );
    }
    #[test]
    fn no_configurations_and_empty_project_are_errors() {
        let (mut i, v) = fixture("union-only.json");
        assert!(compare(&i, vec![]).is_err());
        i.project.clear();
        assert!(compare(&i, v).is_err());
    }
}
