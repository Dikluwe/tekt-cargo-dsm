//! Crystalline Lineage
//! @prompt lab/doxygen-prioridade/CONTRATO.md
//! @layer Lab
//! @updated 2026-09-26
//! Relatório experimental sobre quatro artefatos JSON explícitos.
#![forbid(unsafe_code)]

mod publicacao;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    path::PathBuf,
};

type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Coverage {
    Complete,
    Partial,
    Unknown,
}
impl Coverage {
    fn label(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unknown => "unknown",
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Location {
    file: String,
    line: u64,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Producer {
    name: String,
    version: Option<String>,
    configuration_digest: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Diagnostic {
    code: String,
    message: String,
    symbol_id: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Symbol {
    id: String,
    kind: String,
    name: String,
    qualified_name: Option<String>,
    signature: Option<String>,
    location: Option<Location>,
    brief: String,
    details: String,
    documentation_url: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Relation {
    from: String,
    to: String,
    kind: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CatalogSymbol {
    node_id: u64,
    qualified_name: Option<String>,
    signature: Option<String>,
    location: Option<Location>,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Status {
    Matched,
    Ambiguous,
    Unmatched,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Association {
    node_id: u64,
    status: Status,
    document_id: Option<String>,
    candidates: Vec<String>,
    evidence: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct PendingRelation {
    from: String,
    to: String,
    kind: String,
    reason: String,
}
fn pending(
    version: u64,
    coverage: &str,
    values: &Option<Vec<PendingRelation>>,
    ids: &std::collections::BTreeSet<&str>,
) -> std::result::Result<(), String> {
    if version == 1 {
        if values.is_some() {
            return Err("schema: pending_relations exige inventory v2".into());
        }
        return Ok(());
    }
    let rs = values
        .as_ref()
        .ok_or("schema: inventory v2 exige pending_relations")?;
    if !rs.is_empty() && coverage != "partial" {
        return Err("schema: referências pendentes exigem coverage partial".into());
    }
    for r in rs {
        let reason = match (ids.contains(r.from.as_str()), ids.contains(r.to.as_str())) {
            (false, false) => "missing_both",
            (false, true) => "missing_from",
            (true, false) => "missing_to",
            (true, true) => return Err("schema: relação resolvida na lista pendente".into()),
        };
        if r.reason != reason || r.kind.is_empty() || r.from.is_empty() || r.to.is_empty() {
            return Err("schema: referência pendente inválida".into());
        }
    }
    Ok(())
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    #[serde(rename = "schema")]
    _schema: String,
    #[serde(rename = "version")]
    _version: u64,
    producer: Producer,
    coverage: Coverage,
    symbols: Vec<Symbol>,
    relations: Vec<Relation>,
    pending_relations: Option<Vec<PendingRelation>>,
    diagnostics: Vec<Diagnostic>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    #[serde(rename = "schema")]
    _schema: String,
    #[serde(rename = "version")]
    _version: u64,
    graph_digest: String,
    symbols: Vec<CatalogSymbol>,
    #[serde(default)]
    diagnostics: Vec<Diagnostic>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matches {
    #[serde(rename = "schema")]
    _schema: String,
    #[serde(rename = "version")]
    _version: u64,
    inventory_digest: String,
    catalog_digest: String,
    matches: Vec<Association>,
    unmatched_node_ids: Vec<u64>,
    unmatched_document_ids: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Metric {
    node_id: u64,
    transitive_consumers: Option<u64>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Impact {
    #[serde(rename = "schema")]
    _schema: String,
    #[serde(rename = "version")]
    _version: u64,
    graph_digest: String,
    coverage: Coverage,
    items: Vec<Metric>,
    #[serde(default)]
    diagnostics: Vec<Diagnostic>,
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn check_digest(s: &str, field: &str) -> Result<()> {
    if !s.strip_prefix("sha256:").is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) {
        return Err(format!(
            "invalid_digest: {field} exige sha256: e 64 hexadecimais minúsculos"
        ));
    }
    Ok(())
}
fn decode<T: DeserializeOwned>(bytes: &[u8], schema: &str) -> Result<T> {
    #[derive(Deserialize)]
    struct Header {
        schema: String,
        version: u64,
    }
    let h: Header =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid_json: {schema}: {e}"))?;
    if h.schema != schema
        || !(h.version == 1 || (schema == "lente.doxygen.inventory" && h.version == 2))
    {
        return Err(format!(
            "incompatible_schema: esperado {schema} v1, recebido {} v{}",
            h.schema, h.version
        ));
    }
    serde_json::from_slice(bytes).map_err(|e| format!("invalid_json: {schema}: {e}"))
}
fn nonempty(value: &str, context: &str) -> Result<()> {
    if value.is_empty() {
        Err(format!("invalid_value: {context} vazio"))
    } else {
        Ok(())
    }
}
fn location(location: &Option<Location>, context: &str) -> Result<()> {
    if let Some(l) = location {
        nonempty(&l.file, context)?;
        if l.line == 0 {
            return Err(format!("invalid_location: {context}: linha deve ser >=1"));
        }
    }
    Ok(())
}
fn unique<T: Ord + std::fmt::Debug>(
    values: impl IntoIterator<Item = T>,
    context: &str,
) -> Result<BTreeSet<T>> {
    let mut ids = BTreeSet::new();
    for id in values {
        if ids.contains(&id) {
            return Err(format!("duplicate_id: {context}: {id:?}"));
        }
        ids.insert(id);
    }
    Ok(ids)
}
fn known<T: Ord + std::fmt::Debug>(id: &T, ids: &BTreeSet<T>, context: &str) -> Result<()> {
    if !ids.contains(id) {
        Err(format!("invalid_reference: {context}: {id:?}"))
    } else {
        Ok(())
    }
}
fn diagnostics(ds: &mut [Diagnostic], _ids: &BTreeSet<&str>, source: &str) -> Result<()> {
    for d in ds.iter() {
        nonempty(&d.code, source)?;
    }
    ds.sort();
    Ok(())
}

/// Só combina dados validados. Não consulta filesystem, grafo ou extratores.
fn report(
    inventory_bytes: &[u8],
    catalog_bytes: &[u8],
    matches_bytes: &[u8],
    impact_bytes: &[u8],
) -> Result<Value> {
    let mut inventory: Inventory = decode(inventory_bytes, "lente.doxygen.inventory")?;
    let mut catalog: Catalog = decode(catalog_bytes, "lente.symbol_catalog")?;
    let mut matches: Matches = decode(matches_bytes, "lente.doxygen.matches")?;
    let mut impact: Impact = decode(impact_bytes, "lente.impact_summary")?;
    let inventory_digest = digest(inventory_bytes);
    let catalog_digest = digest(catalog_bytes);
    check_digest(&matches.inventory_digest, "matches.inventory_digest")?;
    check_digest(&matches.catalog_digest, "matches.catalog_digest")?;
    check_digest(&catalog.graph_digest, "catalog.graph_digest")?;
    check_digest(&impact.graph_digest, "impact.graph_digest")?;
    if matches.inventory_digest != inventory_digest || matches.catalog_digest != catalog_digest {
        return Err("digest_mismatch: correspondências não pertencem aos bytes de inventory/catalog fornecidos".into());
    }
    if catalog.graph_digest != impact.graph_digest {
        return Err("graph_digest_mismatch: catálogo e impacto declaram grafos diferentes".into());
    }
    nonempty(&inventory.producer.name, "producer.name")?;
    let doc_ids = unique(
        inventory.symbols.iter().map(|s| s.id.as_str()),
        "inventory.symbols",
    )?;
    let node_ids = unique(catalog.symbols.iter().map(|s| s.node_id), "catalog.symbols")?;
    for s in &inventory.symbols {
        nonempty(&s.id, "document.id")?;
        nonempty(&s.kind, "document.kind")?;
        location(&s.location, &s.id)?;
    }
    for s in &catalog.symbols {
        location(&s.location, &format!("node {}", s.node_id))?;
    }
    pending(
        inventory._version,
        inventory.coverage.label(),
        &inventory.pending_relations,
        &doc_ids,
    )?;
    for r in &inventory.relations {
        known(&r.from.as_str(), &doc_ids, "relation.from")?;
        known(&r.to.as_str(), &doc_ids, "relation.to")?;
        nonempty(&r.kind, "relation.kind")?;
    }
    diagnostics(
        &mut inventory.diagnostics,
        &doc_ids,
        "inventory.diagnostics",
    )?;
    diagnostics(&mut catalog.diagnostics, &doc_ids, "catalog.diagnostics")?;
    diagnostics(&mut matches.diagnostics, &doc_ids, "matches.diagnostics")?;
    diagnostics(&mut impact.diagnostics, &doc_ids, "impact.diagnostics")?;
    let associated_ids = unique(matches.matches.iter().map(|a| a.node_id), "matches.matches")?;
    if associated_ids != node_ids {
        return Err(format!(
            "invalid_match_catalog: exige uma entrada por nó; ausentes {:?}, extras {:?}",
            node_ids.difference(&associated_ids).collect::<Vec<_>>(),
            associated_ids.difference(&node_ids).collect::<Vec<_>>()
        ));
    }
    let mut matched_docs = BTreeSet::new();
    let mut matched_nodes = BTreeSet::new();
    for a in &mut matches.matches {
        unique(a.candidates.iter(), "matches.candidates")?;
        for c in &a.candidates {
            known(&c.as_str(), &doc_ids, "matches.candidates")?;
        }
        if let Some(id) = &a.document_id {
            known(&id.as_str(), &doc_ids, "matches.document_id")?;
        }
        match (a.status, &a.document_id) {
            (Status::Matched, Some(id)) if a.candidates.contains(id) => {
                matched_docs.insert(id.as_str());
                matched_nodes.insert(a.node_id);
            }
            (Status::Ambiguous, None) if !a.candidates.is_empty() => {}
            (Status::Unmatched, None) => {}
            _ => {
                return Err(format!(
                    "invalid_match_status: nó {}: status/document_id/candidates incompatíveis",
                    a.node_id
                ));
            }
        }
        a.candidates.sort();
        a.evidence.sort();
    }
    unique(matches.unmatched_node_ids.iter(), "unmatched_node_ids")?;
    unique(
        matches.unmatched_document_ids.iter(),
        "unmatched_document_ids",
    )?;
    for id in &matches.unmatched_node_ids {
        known(id, &node_ids, "unmatched_node_ids")?;
        if matched_nodes.contains(id) {
            return Err(format!(
                "contradictory_match: nó {id} matched e listado como unmatched"
            ));
        }
    }
    for id in &matches.unmatched_document_ids {
        known(&id.as_str(), &doc_ids, "unmatched_document_ids")?;
        if matched_docs.contains(id.as_str()) {
            return Err(format!(
                "contradictory_match: documento {id} matched e listado como unmatched"
            ));
        }
    }
    unique(impact.items.iter().map(|m| m.node_id), "impact.items")?;
    for m in &impact.items {
        known(&m.node_id, &node_ids, "impact.items")?;
    }
    let metrics: BTreeMap<_, _> = impact
        .items
        .iter()
        .map(|m| (m.node_id, m.transitive_consumers))
        .collect();
    let associations: BTreeMap<_, _> = matches.matches.iter().map(|a| (a.node_id, a)).collect();
    let documents: BTreeMap<_, _> = inventory
        .symbols
        .iter()
        .map(|s| (s.id.as_str(), s))
        .collect();
    let metric = |id: u64| metrics.get(&id).copied().flatten();
    catalog.symbols.sort_by(|a, b| {
        metric(b.node_id)
            .cmp(&metric(a.node_id))
            .then(a.node_id.cmp(&b.node_id))
    });
    let mut items = Vec::new();
    let mut report_diagnostics = Vec::new();
    for (i, node) in catalog.symbols.iter().enumerate() {
        let a = associations[&node.node_id];
        let doc = a.document_id.as_deref().map(|id| documents[id]);
        let state = match a.status {
            Status::Matched
                if doc.is_some_and(|d| {
                    !d.brief.trim().is_empty() || !d.details.trim().is_empty()
                }) =>
            {
                "documentation_observed"
            }
            Status::Matched => "associated_without_text",
            Status::Ambiguous => "ambiguous_association",
            Status::Unmatched => "unassociated",
        };
        let count = metric(node.node_id);
        let reason = match count {
            Some(n) => format!(
                "Posição {} entre métricas fornecidas: {n} consumidores transitivos informados, ordem decrescente; empate por node_id crescente ({}). Cobertura do impacto: {}. Estado documental não altera a ordem.",
                i + 1,
                node.node_id,
                impact.coverage.label()
            ),
            None => {
                report_diagnostics.push(json!({"code":"missing_impact_metric","message":"Métrica ausente não equivale a zero; nó sem posição no ranking.","node_id":node.node_id}));
                "Sem posição: métrica ausente. Exibido após os ranqueados por node_id crescente, sem inferir impacto zero ou prioridade baixa.".into()
            }
        };
        items.push(json!({
            "node_id":node.node_id, "catalog_symbol":node,
            "transitive_consumers":count, "metric_status":if count.is_some() {"provided"} else {"missing"},
            "impact_coverage":impact.coverage, "documentary_state":state,
            "association":a, "document":doc, "rank":count.map(|_| i + 1), "position_reason":reason,
        }));
    }
    for (source, coverage) in [
        ("inventory", inventory.coverage),
        ("impact", impact.coverage),
    ] {
        if coverage != Coverage::Complete {
            report_diagnostics.push(json!({"code":"coverage_not_complete","source":source,"coverage":coverage,"message":"Cobertura declarada não completa; ausência de observação não prova ausência no programa."}));
        }
    }
    report_diagnostics.sort_by_key(Value::to_string);
    matches.unmatched_node_ids.sort();
    matches.unmatched_document_ids.sort();
    inventory.relations.sort();
    if let Some(rs) = &mut inventory.pending_relations {
        rs.sort();
    }
    Ok(json!({
        "schema":"lente.doxygen.priority_report", "version":2,
        "input_digests":{"inventory":inventory_digest,"catalog":catalog_digest,"matches":digest(matches_bytes),"impact":digest(impact_bytes)},
        "graph_digest":catalog.graph_digest,
        "coverage":{"inventory":inventory.coverage,"impact":impact.coverage},
        "coverage_scope":"Cobertura somente do conjunto de entrada declarado, nunca de toda a semântica do programa. Contagens parciais/unknown não são valores completos.",
        "producer":inventory.producer,
        "purpose":"Prioriza revisão humana. Não mede correção, qualidade arquitetural nem qualidade efetiva da documentação.",
        "ordering":"transitive_consumers DESC, node_id ASC; métricas ausentes sem rank ao final por node_id",
        "items":items,
        "unmatched_node_ids":matches.unmatched_node_ids, "unmatched_document_ids":matches.unmatched_document_ids,
        "document_relations":inventory.relations,
        "pending_relations":inventory.pending_relations.unwrap_or_default(),
        "diagnostics":{"inventory":inventory.diagnostics,"catalog":catalog.diagnostics,"matches":matches.diagnostics,"impact":impact.diagnostics,"report":report_diagnostics},
    }))
}

fn summary(r: &Value) -> String {
    let mut s = format!(
        "Revisão humana — não mede correção ou qualidade. Cobertura: inventário={}, impacto={}.\n",
        r["coverage"]["inventory"], r["coverage"]["impact"]
    );
    for item in r["items"].as_array().expect("relatório interno") {
        s.push_str(&format!("node_id={} rank={} consumidores_informados={} estado={} evidencias={} justificativa={}\n", item["node_id"], item["rank"], item["transitive_consumers"], item["documentary_state"], item["association"]["evidence"], item["position_reason"]));
    }
    s.push_str(&format!(
        "Referências pendentes: {}\n",
        r["pending_relations"]
    ));
    s.push_str(&format!("Diagnósticos: {}\n", r["diagnostics"]));
    s
}
const HELP: &str = "lente-doxygen-prioridade --inventory INVENTARIO.json --catalog CATALOGO.json --matches CORRESPONDENCIAS.json --impact IMPACTO.json --output RELATORIO.json [--summary]\nInventário v1 resolvido ou v2; catálogo/matches/impacto v1; relatório v2. Saída nova; --summary imprime resumo. Não executa Doxygen.";
struct Args {
    inventory: PathBuf,
    catalog: PathBuf,
    matches: PathBuf,
    impact: PathBuf,
    output: PathBuf,
    summary: bool,
}
fn args(args: impl IntoIterator<Item = OsString>) -> Result<Args> {
    let mut values = BTreeMap::new();
    let mut summary = false;
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        if flag == "--summary" {
            if summary {
                return Err("argument_error: --summary repetido".into());
            }
            summary = true;
            continue;
        }
        let key = flag.to_str().ok_or("argument_error: opção não UTF-8")?;
        if ![
            "--inventory",
            "--catalog",
            "--matches",
            "--impact",
            "--output",
        ]
        .contains(&key)
        {
            return Err(format!("argument_error: opção desconhecida {key:?}"));
        }
        let value = it
            .next()
            .ok_or_else(|| format!("argument_error: valor ausente para {key}"))?;
        if value.is_empty() || value.to_string_lossy().starts_with("--") {
            return Err(format!(
                "argument_error: caminho inválido para {key}; use ./ para nome iniciado por --"
            ));
        }
        if values
            .insert(key.to_string(), PathBuf::from(value))
            .is_some()
        {
            return Err(format!("argument_error: opção repetida {key}"));
        }
    }
    let mut get = |key| {
        values
            .remove(key)
            .ok_or_else(|| format!("argument_error: falta {key}"))
    };
    Ok(Args {
        inventory: get("--inventory")?,
        catalog: get("--catalog")?,
        matches: get("--matches")?,
        impact: get("--impact")?,
        output: get("--output")?,
        summary,
    })
}
fn execute(a: Args) -> Result<Option<String>> {
    let read = |p: &PathBuf| fs::read(p).map_err(|e| format!("read_error: {}: {e}", p.display()));
    let r = report(
        &read(&a.inventory)?,
        &read(&a.catalog)?,
        &read(&a.matches)?,
        &read(&a.impact)?,
    )?;
    let mut bytes =
        serde_json::to_vec_pretty(&r).map_err(|e| format!("serialization_error: {e}"))?;
    bytes.push(b'\n');
    publicacao::publicar(&a.output, &bytes)?;
    Ok(a.summary.then(|| summary(&r)))
}
fn main() {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() == 1 && a[0] == "--help" {
        println!("{HELP}");
        return;
    }
    match args(a).and_then(execute) {
        Ok(Some(s)) => print!("{s}"),
        Ok(None) => {}
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    const I: &[u8] = include_bytes!("../fixtures/inventory.json");
    const C: &[u8] = include_bytes!("../fixtures/catalog.json");
    const M: &[u8] = include_bytes!("../fixtures/matches.json");
    const P: &[u8] = include_bytes!("../fixtures/impact.json");
    fn value(b: &[u8]) -> Value {
        serde_json::from_slice(b).unwrap()
    }
    fn bytes(v: &Value) -> Vec<u8> {
        serde_json::to_vec(v).unwrap()
    }
    fn linked(i: &Value, c: &Value, m: &Value, p: &Value) -> Result<Value> {
        let (i, c) = (bytes(i), bytes(c));
        let mut m = m.clone();
        m["inventory_digest"] = json!(digest(&i));
        m["catalog_digest"] = json!(digest(&c));
        report(&i, &c, &bytes(&m), &bytes(p))
    }
    fn item(r: &Value, id: u64) -> &Value {
        r["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["node_id"] == id)
            .unwrap()
    }
    fn err(i: &Value, c: &Value, m: &Value, p: &Value, code: &str) {
        let e = linked(i, c, m, p).unwrap_err();
        assert!(e.contains(code), "esperado {code}, recebido {e}");
    }
    #[test]
    fn legacy_v1_resolved_remains_accepted() {
        let mut i = value(I);
        let c = value(C);
        let m = value(M);
        let p = value(P);
        i["version"] = json!(1);
        i.as_object_mut().unwrap().remove("pending_relations");
        assert!(linked(&i, &c, &m, &p).is_ok());
    }
    #[test]
    fn inventory_v2_pending_is_preserved_and_not_resolved() {
        let mut i = value(I);
        let c = value(C);
        let m = value(M);
        let p = value(P);
        i["version"] = json!(2);
        i["coverage"] = json!("partial");
        let id = i["symbols"][0]["id"].clone();
        i["pending_relations"] =
            json!([{"from":id,"to":"missing","kind":"references","reason":"missing_to"}]);
        let result = linked(&i, &c, &m, &p).unwrap();
        assert_eq!(result["pending_relations"], i["pending_relations"]);
        i["coverage"] = json!("complete");
        assert!(linked(&i, &c, &m, &p).is_err());
        i["coverage"] = json!("partial");
        i["pending_relations"][0]["reason"] = json!("missing_from");
        assert!(linked(&i, &c, &m, &p).is_err());
        i["pending_relations"][0]["reason"] = json!("missing_to");
        i["version"] = json!(1);
        assert!(linked(&i, &c, &m, &p).is_err());
        i["version"] = json!(2);
        i.as_object_mut().unwrap().remove("pending_relations");
        assert!(linked(&i, &c, &m, &p).is_err());
    }
    #[test]
    fn alto_impacto_vazio_e_nao_associado_sao_estados_distintos() {
        let r = report(I, C, M, P).unwrap();
        assert_eq!(item(&r, 10)["documentary_state"], "associated_without_text");
        assert_eq!(item(&r, 20)["documentary_state"], "unassociated");
        assert_eq!(item(&r, 10)["transitive_consumers"], 12);
        assert_eq!(item(&r, 20)["transitive_consumers"], 12);
        assert!(item(&r, 20)["document"].is_null());
        assert_eq!(item(&r, 10)["document"]["brief"], "");
    }
    #[test]
    fn texto_observado_de_baixo_impacto_nao_ganha_score() {
        let r = report(I, C, M, P).unwrap();
        assert_eq!(item(&r, 30)["documentary_state"], "documentation_observed");
        assert_eq!(item(&r, 30)["rank"], 4);
        assert_eq!(
            item(&r, 30)["association"]["evidence"],
            json!(["location_exact", "qualified_name_exact"])
        );
        assert!(item(&r, 30).get("score").is_none());
    }
    #[test]
    fn ambiguidades_preservam_candidatos_sem_eleicao_de_documento() {
        let r = report(I, C, M, P).unwrap();
        let a = item(&r, 40);
        assert_eq!(a["documentary_state"], "ambiguous_association");
        assert!(a["document"].is_null());
        assert_eq!(
            a["association"]["candidates"],
            json!(["choice-a", "choice-b"])
        );
    }
    #[test]
    fn empate_e_desempatado_por_id_e_justificado() {
        let r = report(I, C, M, P).unwrap();
        assert_eq!(
            r["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["node_id"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            [10, 20, 40, 30, 50]
        );
        assert_eq!(item(&r, 10)["rank"], 1);
        assert_eq!(item(&r, 20)["rank"], 2);
        for id in [10, 20] {
            assert!(
                item(&r, id)["position_reason"]
                    .as_str()
                    .unwrap()
                    .contains("node_id crescente")
            );
        }
    }
    #[test]
    fn metricas_ausentes_omitidas_e_null_nao_sao_zero() {
        let original = report(I, C, M, P).unwrap();
        assert!(item(&original, 50)["transitive_consumers"].is_null());
        assert!(item(&original, 50)["rank"].is_null());
        for entry in [
            json!({"node_id":50}),
            json!({"node_id":50,"transitive_consumers":null}),
        ] {
            let mut p = value(P);
            p["items"].as_array_mut().unwrap().push(entry);
            let r = linked(&value(I), &value(C), &value(M), &p).unwrap();
            assert!(item(&r, 50)["transitive_consumers"].is_null());
            assert!(item(&r, 50)["rank"].is_null());
            assert_eq!(item(&r, 50)["metric_status"], "missing");
            assert!(
                r["diagnostics"]["report"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["code"] == "missing_impact_metric" && d["node_id"] == 50)
            );
        }
        let mut p = value(P);
        p["items"]
            .as_array_mut()
            .unwrap()
            .push(json!({"node_id":50,"transitive_consumers":0}));
        let r = linked(&value(I), &value(C), &value(M), &p).unwrap();
        assert_eq!(item(&r, 50)["transitive_consumers"], 0);
        assert_eq!(item(&r, 50)["rank"], 5);
    }
    #[test]
    fn coverage_e_diagnosticos_preservados_sem_promocao() {
        for coverage in ["partial", "unknown", "complete"] {
            let mut i = value(I);
            let mut p = value(P);
            i["coverage"] = json!(coverage);
            p["coverage"] = json!(coverage);
            let r = linked(&i, &value(C), &value(M), &p).unwrap();
            assert_eq!(r["coverage"]["inventory"], coverage);
            assert_eq!(r["coverage"]["impact"], coverage);
            assert_eq!(item(&r, 10)["impact_coverage"], coverage);
            assert_eq!(r["diagnostics"]["inventory"], i["diagnostics"]);
            assert_eq!(r["diagnostics"]["impact"], p["diagnostics"]);
            assert_eq!(r["diagnostics"]["matches"], value(M)["diagnostics"]);
        }
    }
    #[test]
    fn vinculos_exigem_os_bytes_exatos_nao_json_equivalente() {
        let mut i = I.to_vec();
        i.push(b' ');
        assert!(report(&i, C, M, P).unwrap_err().contains("digest_mismatch"));
        assert!(
            report(I, &bytes(&value(C)), M, P)
                .unwrap_err()
                .contains("digest_mismatch")
        );
        let mut p = value(P);
        p["graph_digest"] = json!(format!("sha256:{}", "b".repeat(64)));
        assert!(
            report(I, C, M, &bytes(&p))
                .unwrap_err()
                .contains("graph_digest_mismatch")
        );
        let mut m = value(M);
        m["inventory_digest"] = json!("sha256:bad");
        assert!(
            report(I, C, &bytes(&m), P)
                .unwrap_err()
                .contains("invalid_digest")
        );
        let mut m = value(M);
        m["catalog_digest"] = json!(format!("sha256:{}", "b".repeat(64)));
        assert!(
            report(I, C, &bytes(&m), P)
                .unwrap_err()
                .contains("digest_mismatch")
        );
    }
    #[test]
    fn sha256_conhecido() {
        assert_eq!(
            digest(b"abc"),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn schemas_versoes_e_cobertura_invalidos_falham() {
        for which in 0..4 {
            for field in ["schema", "version"] {
                let mut inputs = [value(I), value(C), value(M), value(P)];
                inputs[which][field] = if field == "schema" {
                    json!("outro")
                } else {
                    json!(3)
                };
                err(
                    &inputs[0],
                    &inputs[1],
                    &inputs[2],
                    &inputs[3],
                    "incompatible_schema",
                );
            }
        }
        let mut p = value(P);
        p["coverage"] = json!("success");
        err(&value(I), &value(C), &value(M), &p, "invalid_json");
    }
    #[test]
    fn ids_repetidos_em_todas_as_colecoes_sao_recusados() {
        for (which, key) in [
            (0, "symbols"),
            (1, "symbols"),
            (2, "matches"),
            (2, "unmatched_node_ids"),
            (2, "unmatched_document_ids"),
            (3, "items"),
        ] {
            let mut inputs = [value(I), value(C), value(M), value(P)];
            let a = inputs[which][key].as_array_mut().unwrap();
            a.push(a[0].clone());
            err(
                &inputs[0],
                &inputs[1],
                &inputs[2],
                &inputs[3],
                "duplicate_id",
            );
        }
        let mut m = value(M);
        m["matches"][0]["candidates"] = json!(["empty", "empty"]);
        err(&value(I), &value(C), &m, &value(P), "duplicate_id");
    }
    #[test]
    fn referencias_invalidas_sao_recusadas() {
        for pointer in ["/relations/0/from", "/relations/0/to"] {
            let mut i = value(I);
            *i.pointer_mut(pointer).unwrap() = json!("inexistente");
            err(&i, &value(C), &value(M), &value(P), "invalid_reference");
        }
        for pointer in [
            "/matches/0/document_id",
            "/matches/0/candidates/0",
            "/unmatched_document_ids/0",
        ] {
            let mut m = value(M);
            *m.pointer_mut(pointer).unwrap() = json!("inexistente");
            err(&value(I), &value(C), &m, &value(P), "invalid_reference");
        }
        let mut m = value(M);
        m["unmatched_node_ids"] = json!([999]);
        err(&value(I), &value(C), &m, &value(P), "invalid_reference");
        let mut p = value(P);
        p["items"][0]["node_id"] = json!(999);
        err(&value(I), &value(C), &value(M), &p, "invalid_reference");
    }
    #[test]
    fn correspondencias_exaustivas_e_status_coerentes() {
        let mut m = value(M);
        m["matches"].as_array_mut().unwrap().pop();
        err(&value(I), &value(C), &m, &value(P), "invalid_match_catalog");
        let mut m = value(M);
        m["matches"][0]["node_id"] = json!(999);
        err(&value(I), &value(C), &m, &value(P), "invalid_match_catalog");
        for (index, field, val) in [
            (0, "document_id", Value::Null),
            (0, "candidates", json!([])),
            (1, "document_id", json!("empty")),
            (3, "candidates", json!([])),
        ] {
            let mut m = value(M);
            m["matches"][index][field] = val;
            err(&value(I), &value(C), &m, &value(P), "invalid_match_status");
        }
        let mut m = value(M);
        m["unmatched_node_ids"] = json!([10]);
        err(&value(I), &value(C), &m, &value(P), "contradictory_match");
        let mut m = value(M);
        m["unmatched_document_ids"] = json!(["empty"]);
        err(&value(I), &value(C), &m, &value(P), "contradictory_match");
    }
    #[test]
    fn valores_negativos_fracionarios_e_localizacao_zero_falham() {
        for v in [json!(-1), json!(1.5), json!("12")] {
            let mut p = value(P);
            p["items"][0]["transitive_consumers"] = v;
            err(&value(I), &value(C), &value(M), &p, "invalid_json");
        }
        let mut c = value(C);
        c["symbols"][0]["location"]["line"] = json!(0);
        err(&value(I), &c, &value(M), &value(P), "invalid_location");
    }
    #[test]
    fn opcionais_ausentes_nao_inventam_fatos_e_texto_e_literal() {
        let mut i = value(I);
        let mut c = value(C);
        for symbol in i["symbols"].as_array_mut().unwrap() {
            for key in [
                "qualified_name",
                "signature",
                "location",
                "documentation_url",
            ] {
                symbol.as_object_mut().unwrap().remove(key);
            }
        }
        i["producer"].as_object_mut().unwrap().remove("version");
        i["producer"]
            .as_object_mut()
            .unwrap()
            .remove("configuration_digest");
        i["symbols"][0]["details"] = json!(" \t\n");
        i["symbols"][1]["brief"] = json!("<script>texto literal</script>\u{001b}[2J");
        c["symbols"][0]["qualified_name"] = json!("demo::A( const X & )");
        c["symbols"][0]["location"]["file"] = json!("./src/../src/demo.cpp");
        let r = linked(&i, &c, &value(M), &value(P)).unwrap();
        assert!(r["producer"]["version"].is_null());
        assert!(item(&r, 10)["document"]["qualified_name"].is_null());
        assert_eq!(item(&r, 10)["documentary_state"], "associated_without_text");
        assert_eq!(item(&r, 30)["document"]["brief"], i["symbols"][1]["brief"]);
        assert_eq!(
            item(&r, 10)["catalog_symbol"]["location"]["file"],
            "./src/../src/demo.cpp"
        );
        assert!(!summary(&r).contains('\u{001b}'));
    }
    #[test]
    fn determinismo_e_ordem_semantica_sob_permutacao() {
        let original = report(I, C, M, P).unwrap();
        assert_eq!(bytes(&original), bytes(&report(I, C, M, P).unwrap()));
        let mut i = value(I);
        let mut c = value(C);
        let mut m = value(M);
        let mut p = value(P);
        i["symbols"].as_array_mut().unwrap().reverse();
        c["symbols"].as_array_mut().unwrap().reverse();
        m["matches"].as_array_mut().unwrap().reverse();
        p["items"].as_array_mut().unwrap().reverse();
        let changed = linked(&i, &c, &m, &p).unwrap();
        assert_eq!(original["items"], changed["items"]);
        assert_eq!(original["diagnostics"], changed["diagnostics"]);
        assert_ne!(original["input_digests"], changed["input_digests"]);
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            // Testes só escrevem dentro da área autorizada do experimento.
            let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
            fs::create_dir_all(&base).unwrap();
            loop {
                let p = base.join(format!(
                    "test-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&p) {
                    Ok(()) => {
                        for (n, b) in [("i", I), ("c", C), ("m", M), ("p", P)] {
                            fs::write(p.join(n), b).unwrap();
                        }
                        return Self(p);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("{e}"),
                }
            }
        }
        fn args(&self) -> Vec<OsString> {
            let mut a = vec![];
            for (flag, name) in [
                ("--inventory", "i"),
                ("--catalog", "c"),
                ("--matches", "m"),
                ("--impact", "p"),
                ("--output", "r"),
            ] {
                a.push(flag.into());
                a.push(self.0.join(name).into_os_string());
            }
            a
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn cli_grava_json_e_resumo_opcional_e_recusa_sobrescrita() {
        let f = Fixture::new();
        let mut a = f.args();
        a.push("--summary".into());
        let resumo = execute(args(a).unwrap()).unwrap().unwrap();
        assert!(resumo.contains("unassociated"));
        assert!(resumo.contains("consumidores_informados=null"));
        let original = fs::read(f.0.join("r")).unwrap();
        assert_eq!(value(&original), report(I, C, M, P).unwrap());
        assert!(
            execute(args(f.args()).unwrap())
                .unwrap_err()
                .contains("output_error")
        );
        assert_eq!(original, fs::read(f.0.join("r")).unwrap());
        fs::remove_file(f.0.join("r")).unwrap();
        assert!(execute(args(f.args()).unwrap()).unwrap().is_none());
    }
    #[test]
    fn cli_erro_nao_publica_e_nao_sobrescreve_entrada() {
        let f = Fixture::new();
        fs::write(f.0.join("p"), b"{}").unwrap();
        assert!(execute(args(f.args()).unwrap()).is_err());
        assert!(!f.0.join("r").exists());
        fs::write(f.0.join("p"), P).unwrap();
        let mut a = f.args();
        *a.last_mut().unwrap() = f.0.join("i").into_os_string();
        assert!(
            execute(args(a).unwrap())
                .unwrap_err()
                .contains("output_error")
        );
        assert_eq!(fs::read(f.0.join("i")).unwrap(), I);
    }
    #[test]
    fn cli_rejeita_argumentos_desconhecidos_ausentes_e_repetidos() {
        for a in [
            vec![],
            vec!["--inventory"],
            vec!["--outro", "x"],
            vec!["--summary", "--summary"],
            vec!["--inventory", "a", "--inventory", "b"],
        ] {
            assert!(args(a.into_iter().map(OsString::from)).is_err());
        }
    }
}
