//! Crystalline Lineage
//! @prompt lab/doxygen-correspondencia/CONTRATO.md
//! Associação documental experimental; intercâmbio exclusivamente JSON.
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
struct Header {
    schema: String,
    version: u64,
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
) -> Result<(), String> {
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    schema: String,
    version: u64,
    producer: Producer,
    coverage: Coverage,
    symbols: Vec<Document>,
    relations: Vec<Relation>,
    pending_relations: Option<Vec<PendingRelation>>,
    diagnostics: Vec<Diagnostic>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Producer {
    name: String,
    version: Option<String>,
    configuration_digest: Option<String>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Coverage {
    Complete,
    Partial,
    Unknown,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Relation {
    from: String,
    to: String,
    kind: String,
}
#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Location {
    file: String,
    line: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema: String,
    version: u64,
    graph_digest: String,
    symbols: Vec<Node>,
    #[serde(default)]
    diagnostics: Vec<Diagnostic>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    node_id: u64,
    qualified_name: Option<String>,
    signature: Option<String>,
    location: Option<Location>,
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub symbol_id: Option<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Matched,
    Ambiguous,
    Unmatched,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Match {
    pub node_id: u64,
    pub status: Status,
    pub document_id: Option<String>,
    pub candidates: Vec<String>,
    pub evidence: Vec<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Matches {
    pub schema: String,
    pub version: u64,
    pub inventory_digest: String,
    pub catalog_digest: String,
    pub matches: Vec<Match>,
    pub unmatched_node_ids: Vec<u64>,
    pub unmatched_document_ids: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn nonblank(s: &str) -> bool {
    s.chars().any(|c| !c.is_whitespace())
}
fn header(bytes: &[u8], expected: &str) -> Result<(), String> {
    let h: Header =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid_header ({expected}): {e}"))?;
    if h.schema != expected
        || !(h.version == 1 || (expected == "lente.doxygen.inventory" && h.version == 2))
    {
        return Err(format!(
            "incompatible_schema_version: esperado {expected} v1, recebido {} v{}",
            h.schema, h.version
        ));
    }
    Ok(())
}
fn location(l: &Option<Location>) -> Result<(), String> {
    if l.as_ref()
        .is_some_and(|l| !nonblank(&l.file) || l.line == 0)
    {
        return Err("invalid_location: file não branco e line positivo exigidos".into());
    }
    Ok(())
}
fn diag(code: &str, id: Option<&str>, value: serde_json::Value) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        symbol_id: id.map(str::to_owned),
        message: value.to_string(),
    }
}
fn facts(n: &Node, d: &Document) -> (bool, BTreeSet<String>) {
    let mut f = BTreeSet::from(["qualified_name_exact".into()]);
    let mut loc = false;
    match (&n.location, &d.location) {
        (Some(a), Some(b)) if a == b => {
            loc = true;
            f.insert("location_exact".into());
        }
        (Some(a), Some(b)) => {
            f.insert("location_difference".into());
            if a.file != b.file {
                f.insert("file_difference".into());
            }
            if a.line != b.line {
                f.insert("line_difference".into());
            }
        }
        _ => {
            f.insert("location_missing".into());
        }
    }
    let conflict = matches!((&n.signature,&d.signature),(Some(a),Some(b)) if a!=b);
    let sig = matches!((&n.signature,&d.signature),(Some(a),Some(b)) if a==b && nonblank(a));
    if conflict {
        f.insert("signature_conflict".into());
    }
    if sig {
        f.insert("signature_exact".into());
    }
    if n.signature.as_deref().is_none_or(|s| !nonblank(s))
        || d.signature.as_deref().is_none_or(|s| !nonblank(s))
    {
        f.insert("signature_missing_or_blank".into());
    }
    (!conflict && (loc || sig), f)
}
/// Associa somente pelos fatos declarados; hashes cobrem os mesmos bytes usados
/// no parse. Não lê fontes e não interpreta texto documental ou sintaxe C++.
pub fn associate(inventory_bytes: &[u8], catalog_bytes: &[u8]) -> Result<Matches, String> {
    header(inventory_bytes, "lente.doxygen.inventory")?;
    header(catalog_bytes, "lente.symbol_catalog")?;
    let mut inv: Inventory =
        serde_json::from_slice(inventory_bytes).map_err(|e| format!("invalid_inventory: {e}"))?;
    let cat: Catalog =
        serde_json::from_slice(catalog_bytes).map_err(|e| format!("invalid_catalog: {e}"))?;
    // Campos validados pelo header, mantidos no modelo para rejeitar duplicações.
    let _ = (&inv.schema, inv.version, &cat.schema, cat.version);
    if !cat
        .graph_digest
        .strip_prefix("sha256:")
        .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("invalid_graph_digest: esperado sha256: seguido de 64 hexadecimais".into());
    }
    let mut docs = BTreeMap::new();
    let mut names: BTreeMap<&str, Vec<&Document>> = BTreeMap::new();
    for d in &inv.symbols {
        if !nonblank(&d.id) {
            return Err("invalid_document_id: ID vazio/branco".into());
        }
        if docs.insert(d.id.as_str(), d).is_some() {
            return Err(format!("duplicate_document_id: {}", d.id));
        }
        location(&d.location)?;
        // Campos documentais são consumidos/validados, nunca usados como prova.
        let _ = (&d.kind, &d.name, &d.brief, &d.details, &d.documentation_url);
        if let Some(q) = d.qualified_name.as_deref().filter(|q| nonblank(q)) {
            names.entry(q).or_default().push(d);
        }
    }
    pending(
        inv.version,
        match inv.coverage {
            Coverage::Complete => "complete",
            Coverage::Partial => "partial",
            Coverage::Unknown => "unknown",
        },
        &inv.pending_relations,
        &docs.keys().copied().collect(),
    )?;
    for r in &inv.relations {
        if !docs.contains_key(r.from.as_str()) || !docs.contains_key(r.to.as_str()) {
            return Err(format!(
                "unknown_relation_endpoint: {} -> {}; migre inventário para v2",
                r.from, r.to
            ));
        }
        let _ = &r.kind;
    }
    let mut nodes = BTreeMap::new();
    for n in &cat.symbols {
        if nodes.insert(n.node_id, n).is_some() {
            return Err(format!("duplicate_node_id: {}", n.node_id));
        }
        location(&n.location)?;
    }
    if let Some(rs) = &mut inv.pending_relations {
        rs.sort();
    }
    let mut diagnostics = vec![diag(
        "inventory_context",
        None,
        json!({"producer":inv.producer,"coverage":inv.coverage,"pending_relations":inv.pending_relations.as_deref().unwrap_or(&[]),"meaning":"Cobertura declarada do conjunto de entrada, não completude semântica"}),
    )];
    diagnostics.extend(
        inv.diagnostics
            .iter()
            .map(|d| diag("inventory_diagnostic", d.symbol_id.as_deref(), json!(d))),
    );
    diagnostics.extend(cat.diagnostics);
    let mut claims: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let mut matches = Vec::new();
    for (&id, n) in &nodes {
        let mut evaluations = Vec::new();
        if let Some(q) = n.qualified_name.as_deref().filter(|q| nonblank(q)) {
            for d in names.get(q).into_iter().flatten() {
                let (sufficient, f) = facts(n, d);
                diagnostics.push(diag("candidate_evaluation",Some(&d.id),json!({"node_id":id,"document_id":d.id,
                    "catalog":{"qualified_name":n.qualified_name,"signature":n.signature,"location":n.location},
                    "inventory":{"qualified_name":d.qualified_name,"signature":d.signature,"location":d.location},
                    "facts":f,"sufficient":sufficient})));
                evaluations.push((d.id.clone(), sufficient, f));
            }
        }
        evaluations.sort_by(|a, b| a.0.cmp(&b.0));
        let has_sufficient = evaluations.iter().any(|e| e.1);
        let chosen: Vec<_> = evaluations
            .iter()
            .filter(|e| !has_sufficient || e.1)
            .collect();
        let candidates: Vec<_> = chosen.iter().map(|e| e.0.clone()).collect();
        let mut evidence = chosen.first().map(|e| e.2.clone()).unwrap_or_default();
        for e in &chosen {
            evidence = evidence.intersection(&e.2).cloned().collect();
        }
        if has_sufficient {
            for d in &candidates {
                claims.entry(d.clone()).or_default().push(id);
            }
        } else if chosen.is_empty() {
            evidence.insert(
                if n.qualified_name.as_deref().is_none_or(|q| !nonblank(q)) {
                    "qualified_name_missing"
                } else {
                    "no_qualified_name_candidate"
                }
                .into(),
            );
        }
        let status = if !has_sufficient {
            Status::Unmatched
        } else if candidates.len() == 1 {
            Status::Matched
        } else {
            Status::Ambiguous
        };
        let document_id = if status == Status::Matched {
            Some(candidates[0].clone())
        } else {
            None
        };
        matches.push(Match {
            node_id: id,
            status,
            document_id,
            candidates,
            evidence: evidence.into_iter().collect(),
        });
    }
    let mut conflicted = BTreeSet::new();
    for (document, ids) in &claims {
        if ids.len() > 1 {
            conflicted.extend(ids.iter().copied());
            diagnostics.push(diag("document_conflict",Some(document),json!({"document_id":document,"node_ids":ids,"reason":"Documento suficiente reivindicado por múltiplos nós; nenhuma atribuição automática"})));
        }
    }
    let mut used = BTreeSet::new();
    for m in &mut matches {
        if conflicted.contains(&m.node_id) {
            m.status = Status::Ambiguous;
            m.document_id = None;
            m.evidence.push("document_claim_conflict".into());
            m.evidence.sort();
        }
        match m.status {
            Status::Matched=>{used.insert(m.document_id.clone().unwrap());},
            Status::Ambiguous=>diagnostics.push(diag("node_ambiguous",None,json!({"node_id":m.node_id,"candidates":m.candidates,"reason":"Múltiplos candidatos suficientes ou reivindicação documental conflitante"}))),
            Status::Unmatched=>diagnostics.push(diag("node_unmatched",None,json!({"node_id":m.node_id,"candidates":m.candidates,"reason":"Nenhum candidato satisfaz a política; consulte candidate_evaluation ou evidence"}))),
        }
    }
    diagnostics.sort_by(|a, b| {
        (&a.code, &a.symbol_id, &a.message).cmp(&(&b.code, &b.symbol_id, &b.message))
    });
    Ok(Matches {
        schema: "lente.doxygen.matches".into(),
        version: 1,
        inventory_digest: digest(inventory_bytes),
        catalog_digest: digest(catalog_bytes),
        unmatched_node_ids: matches
            .iter()
            .filter(|m| m.status != Status::Matched)
            .map(|m| m.node_id)
            .collect(),
        unmatched_document_ids: docs
            .keys()
            .filter(|id| !used.contains(**id))
            .map(|s| (*s).to_owned())
            .collect(),
        matches,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn inputs() -> (Value, Value) {
        (
            serde_json::from_str(include_str!("../fixtures/inventory.json")).unwrap(),
            serde_json::from_str(include_str!("../fixtures/catalog.json")).unwrap(),
        )
    }
    fn run(i: &Value, c: &Value) -> Matches {
        associate(
            &serde_json::to_vec(i).unwrap(),
            &serde_json::to_vec(c).unwrap(),
        )
        .unwrap()
    }
    fn row(r: &Matches, id: u64) -> &Match {
        r.matches.iter().find(|m| m.node_id == id).unwrap()
    }
    #[test]
    fn legacy_v1_resolved_remains_accepted() {
        let (mut i, c) = inputs();
        i["version"] = json!(1);
        i.as_object_mut().unwrap().remove("pending_relations");
        assert!(
            associate(
                &serde_json::to_vec(&i).unwrap(),
                &serde_json::to_vec(&c).unwrap()
            )
            .is_ok()
        );
    }
    #[test]
    fn inventory_v2_pending_is_preserved_and_not_resolved() {
        let (mut i, c) = inputs();
        i["version"] = json!(2);
        i["coverage"] = json!("partial");
        let id = i["symbols"][0]["id"].clone();
        i["pending_relations"] =
            json!([{"from":id,"to":"missing","kind":"references","reason":"missing_to"}]);
        let result = associate(
            &serde_json::to_vec(&i).unwrap(),
            &serde_json::to_vec(&c).unwrap(),
        )
        .unwrap();
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "inventory_context" && d.message.contains("missing_to"))
        );
        i["coverage"] = json!("complete");
        assert!(
            associate(
                &serde_json::to_vec(&i).unwrap(),
                &serde_json::to_vec(&c).unwrap()
            )
            .is_err()
        );
        i["coverage"] = json!("partial");
        i["pending_relations"][0]["reason"] = json!("missing_from");
        assert!(
            associate(
                &serde_json::to_vec(&i).unwrap(),
                &serde_json::to_vec(&c).unwrap()
            )
            .is_err()
        );
        i["pending_relations"][0]["reason"] = json!("missing_to");
        i["version"] = json!(1);
        assert!(
            associate(
                &serde_json::to_vec(&i).unwrap(),
                &serde_json::to_vec(&c).unwrap()
            )
            .is_err()
        );
        i["version"] = json!(2);
        i.as_object_mut().unwrap().remove("pending_relations");
        assert!(
            associate(
                &serde_json::to_vec(&i).unwrap(),
                &serde_json::to_vec(&c).unwrap()
            )
            .is_err()
        );
    }
    #[test]
    fn exact_location_and_name() {
        let (i, c) = inputs();
        let r = run(&i, &c);
        assert_eq!(row(&r, 10).status, Status::Matched);
        assert_eq!(row(&r, 10).document_id.as_deref(), Some("loc"));
        assert!(row(&r, 10).evidence.contains(&"location_exact".into()));
    }
    #[test]
    fn exact_signature_without_location() {
        let (i, c) = inputs();
        let r = run(&i, &c);
        assert_eq!(row(&r, 20).status, Status::Matched);
        assert!(row(&r, 20).evidence.contains(&"signature_exact".into()));
        assert!(row(&r, 20).evidence.contains(&"location_missing".into()));
    }
    #[test]
    fn overloads_and_namespaces() {
        let (i, c) = inputs();
        let r = run(&i, &c);
        assert_eq!(row(&r, 30).candidates, vec!["over-int"]);
        assert_eq!(row(&r, 40).document_id.as_deref(), Some("namespace"));
        assert!(r.unmatched_document_ids.contains(&"over-double".into()));
    }
    #[test]
    fn multiple_candidates_not_first() {
        let (i, c) = inputs();
        let r = run(&i, &c);
        let m = row(&r, 50);
        assert_eq!(m.status, Status::Ambiguous);
        assert_eq!(m.document_id, None);
        assert_eq!(m.candidates, vec!["amb-a", "amb-b"]);
        assert!(r.unmatched_node_ids.contains(&50));
        for d in &m.candidates {
            assert!(r.unmatched_document_ids.contains(d));
        }
    }
    #[test]
    fn signature_conflict_vetoes_exact_location() {
        let (i, c) = inputs();
        let r = run(&i, &c);
        let m = row(&r, 60);
        assert_eq!(m.status, Status::Unmatched);
        assert!(m.evidence.contains(&"signature_conflict".into()));
        assert!(m.evidence.contains(&"location_exact".into()));
    }
    #[test]
    fn name_only_and_no_candidate() {
        let (i, c) = inputs();
        let r = run(&i, &c);
        assert_eq!(row(&r, 70).status, Status::Unmatched);
        assert_eq!(row(&r, 70).candidates, vec!["weak"]);
        assert!(row(&r, 90).candidates.is_empty());
        assert!(
            row(&r, 100)
                .evidence
                .contains(&"qualified_name_missing".into())
        );
    }
    #[test]
    fn paths_and_signatures_not_normalized() {
        let (mut i, mut c) = inputs();
        let r = run(&i, &c);
        assert_eq!(row(&r, 80).status, Status::Unmatched);
        assert!(row(&r, 80).evidence.contains(&"file_difference".into()));
        i["symbols"][1]["signature"] = json!("void( int )");
        assert_eq!(row(&run(&i, &c), 20).status, Status::Unmatched);
        c["symbols"][1]["qualified_name"] = json!(" demo::sig");
        assert!(row(&run(&i, &c), 20).candidates.is_empty());
    }
    #[test]
    fn location_difference_declared_but_signature_rule_allowed() {
        let (mut i, mut c) = inputs();
        i["symbols"][1]["location"] = json!({"file":"a.cpp","line":1});
        c["symbols"][1]["location"] = json!({"file":"b.cpp","line":2});
        let r = run(&i, &c);
        let m = row(&r, 20);
        assert_eq!(m.status, Status::Matched);
        assert!(m.evidence.contains(&"file_difference".into()));
        assert!(m.evidence.contains(&"line_difference".into()));
    }
    #[test]
    fn duplicate_ids_are_errors_even_identical() {
        let (i, c) = inputs();
        for inventory in [true, false] {
            let (mut a, mut b) = (i.clone(), c.clone());
            let target = if inventory { &mut a } else { &mut b };
            let duplicate = target["symbols"][0].clone();
            target["symbols"].as_array_mut().unwrap().push(duplicate);
            let err = associate(
                &serde_json::to_vec(&a).unwrap(),
                &serde_json::to_vec(&b).unwrap(),
            )
            .unwrap_err();
            assert!(err.contains(if inventory {
                "duplicate_document_id"
            } else {
                "duplicate_node_id"
            }));
        }
    }
    #[test]
    fn document_conflict_blocks_all_claimants() {
        let (i, mut c) = inputs();
        let mut n = c["symbols"][0].clone();
        n["node_id"] = json!(11);
        c["symbols"].as_array_mut().unwrap().push(n);
        let r = run(&i, &c);
        for id in [10, 11] {
            assert_eq!(row(&r, id).status, Status::Ambiguous);
            assert!(
                row(&r, id)
                    .evidence
                    .contains(&"document_claim_conflict".into())
            );
        }
        assert!(r.diagnostics.iter().any(|d| d.code == "document_conflict"));
    }
    #[test]
    fn unique_claim_conflicts_with_ambiguous_claim() {
        let (mut i, mut c) = inputs();
        i["symbols"][5]["location"] = json!({"file":"a.cpp","line":5});
        let mut n = c["symbols"][4].clone();
        n["node_id"] = json!(51);
        n["signature"] = Value::Null;
        n["location"] = json!({"file":"a.cpp","line":5});
        c["symbols"].as_array_mut().unwrap().push(n);
        let r = run(&i, &c);
        assert_eq!(row(&r, 51).candidates, vec!["amb-a"]);
        assert_eq!(row(&r, 51).status, Status::Ambiguous);
    }
    #[test]
    fn row_evidence_is_intersection_not_union() {
        let (mut i, mut c) = inputs();
        i["symbols"][5]["location"] = json!({"file":"a.cpp","line":5});
        c["symbols"][4]["location"] = i["symbols"][5]["location"].clone();
        let r = run(&i, &c);
        let m = row(&r, 50);
        assert_eq!(m.status, Status::Ambiguous);
        assert!(!m.evidence.contains(&"location_exact".into()));
        assert!(m.evidence.contains(&"signature_exact".into()));
        let facts: Vec<Value> = r
            .diagnostics
            .iter()
            .filter(|d| d.code == "candidate_evaluation")
            .map(|d| serde_json::from_str(&d.message).unwrap())
            .collect();
        assert!(facts.iter().any(|d| {
            d["document_id"] == "amb-a"
                && d["facts"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("location_exact"))
        }));
    }
    #[test]
    fn reordering_preserves_decisions_not_byte_digests() {
        let (mut i, mut c) = inputs();
        let a = run(&i, &c);
        i["symbols"].as_array_mut().unwrap().reverse();
        c["symbols"].as_array_mut().unwrap().reverse();
        let b = run(&i, &c);
        assert_eq!(a.matches, b.matches);
        assert_eq!(a.diagnostics, b.diagnostics);
        assert_eq!(a.unmatched_node_ids, b.unmatched_node_ids);
        assert_eq!(a.unmatched_document_ids, b.unmatched_document_ids);
        assert_ne!(a.inventory_digest, b.inventory_digest);
        assert_ne!(a.catalog_digest, b.catalog_digest);
    }
    #[test]
    fn digests_include_exact_whitespace() {
        let (i, c) = inputs();
        let ib = serde_json::to_vec(&i).unwrap();
        let cb = serde_json::to_vec(&c).unwrap();
        let a = associate(&ib, &cb).unwrap();
        let mut extra = ib.clone();
        extra.extend_from_slice(b" \r\n");
        let b = associate(&extra, &cb).unwrap();
        assert_eq!(a.matches, b.matches);
        assert_ne!(a.inventory_digest, b.inventory_digest);
        assert_eq!(a.catalog_digest, b.catalog_digest);
        assert_eq!(
            digest(b"abc"),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn missing_optional_fields_do_not_invent_facts() {
        let (mut i, mut c) = inputs();
        for k in ["signature", "location", "qualified_name"] {
            i["symbols"][0].as_object_mut().unwrap().remove(k);
            c["symbols"][0].as_object_mut().unwrap().remove(k);
        }
        i["producer"].as_object_mut().unwrap().remove("version");
        let r = run(&i, &c);
        assert_eq!(row(&r, 10).status, Status::Unmatched);
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == "inventory_context" && d.message.contains("unknown"))
        );
    }
    #[test]
    fn schema_versions_types_and_relations_rejected() {
        let (i, c) = inputs();
        for (key, value) in [("schema", json!("wrong")), ("version", json!(3))] {
            let mut a = i.clone();
            a[key] = value;
            assert!(
                associate(
                    &serde_json::to_vec(&a).unwrap(),
                    &serde_json::to_vec(&c).unwrap()
                )
                .unwrap_err()
                .contains("incompatible_schema_version")
            );
        }
        let mut a = i.clone();
        a["relations"] = json!([{"from":"missing","to":"loc","kind":"references"}]);
        assert!(
            associate(
                &serde_json::to_vec(&a).unwrap(),
                &serde_json::to_vec(&c).unwrap()
            )
            .unwrap_err()
            .contains("unknown_relation_endpoint")
        );
        let mut b = c.clone();
        b["symbols"][0]["location"]["line"] = json!(0);
        assert!(
            associate(
                &serde_json::to_vec(&i).unwrap(),
                &serde_json::to_vec(&b).unwrap()
            )
            .unwrap_err()
            .contains("invalid_location")
        );
    }
    #[test]
    fn blank_signatures_and_simple_names_never_suffice() {
        let (mut i, mut c) = inputs();
        i["symbols"][1]["signature"] = json!("   ");
        c["symbols"][1]["signature"] = json!("   ");
        assert_eq!(row(&run(&i, &c), 20).status, Status::Unmatched);
        i["symbols"][1]["qualified_name"] = Value::Null;
        i["symbols"][1]["name"] = json!("demo::sig");
        assert!(row(&run(&i, &c), 20).candidates.is_empty());
    }
}
