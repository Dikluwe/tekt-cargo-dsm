//! Crystalline Lineage — @prompt ../CONTRATO.md
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Location {
    file: String,
    line: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Diagnostic {
    code: String,
    message: String,
    symbol_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Producer {
    name: String,
    version: Option<String>,
    configuration_digest: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Symbol {
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
#[derive(Deserialize, Serialize)]
struct Relation {
    from: String,
    to: String,
    kind: String,
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
struct Inventory {
    schema: String,
    version: u32,
    producer: Producer,
    coverage: String,
    symbols: Vec<Symbol>,
    relations: Vec<Relation>,
    pending_relations: Option<Vec<PendingRelation>>,
    diagnostics: Vec<Diagnostic>,
}
#[derive(Deserialize)]
struct Node {
    node_id: u64,
    qualified_name: Option<String>,
    signature: Option<String>,
    location: Option<Location>,
}
#[derive(Deserialize)]
struct Catalog {
    schema: String,
    version: u32,
    graph_digest: String,
    symbols: Vec<Node>,
    #[serde(default)]
    diagnostics: Vec<Diagnostic>,
}
#[derive(Deserialize)]
struct Match {
    node_id: u64,
    status: String,
    document_id: Option<String>,
    candidates: Vec<String>,
    evidence: Vec<String>,
}
#[derive(Deserialize)]
struct Matches {
    schema: String,
    version: u32,
    inventory_digest: String,
    catalog_digest: String,
    matches: Vec<Match>,
    unmatched_node_ids: Vec<u64>,
    unmatched_document_ids: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn hash_shape(s: &str) -> bool {
    s.strip_prefix("sha256:").is_some_and(|h| {
        h.len() == 64
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn check(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn schema(actual: &str, version: u32, expected: &str) -> Result<(), String> {
    check(
        actual == expected
            && (version == 1 || (expected == "lente.doxygen.inventory" && version == 2)),
        &format!("schema/version incompatível: {actual} v{version}; esperado {expected} v1"),
    )
}
fn diagnostics(ds: &mut [Diagnostic], _docs: &BTreeMap<String, Symbol>) -> Result<(), String> {
    ds.sort_by(|a, b| {
        (&a.code, &a.symbol_id, &a.message).cmp(&(&b.code, &b.symbol_id, &b.message))
    });
    Ok(())
}
/// Política conservadora local, não um resolvedor de URLs ou leitor de arquivos.
pub fn safe_url(s: &str) -> bool {
    let Some(rest) = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
    else {
        return false;
    };
    if !s.is_ascii() || s.bytes().any(|b| b <= 32 || b == 127 || b == b'\\') {
        return false;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let (host, port) = authority
        .split_once(':')
        .map_or((authority, None), |(h, p)| (h, Some(p)));
    if port.is_some_and(|p| {
        p.is_empty()
            || !p.bytes().all(|b| b.is_ascii_digit())
            || p.parse::<u16>().map_or(true, |v| v == 0)
    }) {
        return false;
    }
    !host.is_empty()
        && host.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

/// Valida integralmente antes de construir qualquer associação de apresentação.
pub fn prepare(inventory: &[u8], catalog: &[u8], matches: &[u8]) -> Result<Value, String> {
    let mut inv: Inventory =
        serde_json::from_slice(inventory).map_err(|e| format!("inventário: {e}"))?;
    let mut cat: Catalog = serde_json::from_slice(catalog).map_err(|e| format!("catálogo: {e}"))?;
    let mut map: Matches =
        serde_json::from_slice(matches).map_err(|e| format!("correspondências: {e}"))?;
    schema(&inv.schema, inv.version, "lente.doxygen.inventory")?;
    schema(&cat.schema, cat.version, "lente.symbol_catalog")?;
    schema(&map.schema, map.version, "lente.doxygen.matches")?;
    check(
        matches!(inv.coverage.as_str(), "complete" | "partial" | "unknown"),
        "coverage inválida",
    )?;
    check(hash_shape(&cat.graph_digest), "graph_digest inválido")?;
    check(
        map.inventory_digest == digest(inventory),
        "inventory_digest não corresponde aos bytes recebidos",
    )?;
    check(
        map.catalog_digest == digest(catalog),
        "catalog_digest não corresponde aos bytes recebidos",
    )?;
    let mut docs = BTreeMap::new();
    let valid_location = |l: &Option<Location>| {
        check(
            l.as_ref()
                .is_none_or(|l| !l.file.trim().is_empty() && l.line > 0),
            "localização inválida",
        )
    };
    for s in inv.symbols {
        valid_location(&s.location)?;
        check(!s.id.is_empty(), "ID documental vazio")?;
        check(
            docs.insert(s.id.clone(), s).is_none(),
            "ID documental duplicado",
        )?;
    }
    pending(
        inv.version as u64,
        &inv.coverage,
        &inv.pending_relations,
        &docs.keys().map(String::as_str).collect(),
    )?;
    for r in &inv.relations {
        check(
            docs.contains_key(&r.from) && docs.contains_key(&r.to),
            "endpoint documental inexistente",
        )?;
        let _original_kind = &r.kind;
    }
    diagnostics(&mut inv.diagnostics, &docs)?;
    diagnostics(&mut map.diagnostics, &docs)?;
    let mut nodes = BTreeMap::new();
    diagnostics(&mut cat.diagnostics, &docs)?;
    for n in cat.symbols {
        valid_location(&n.location)?;
        check(nodes.insert(n.node_id, n).is_none(), "node_id duplicado")?;
    }
    let mut links = BTreeMap::new();
    let mut used = BTreeSet::new();
    let mut unmatched = BTreeSet::new();
    for mut m in map.matches {
        check(
            nodes.contains_key(&m.node_id),
            "correspondência referencia nó inexistente",
        )?;
        m.candidates.sort();
        m.evidence.sort();
        check(
            !m.candidates.windows(2).any(|p| p[0] == p[1]),
            "candidato duplicado",
        )?;
        check(
            m.candidates.iter().all(|id| docs.contains_key(id)),
            "candidato inexistente",
        )?;
        match m.status.as_str() {
            "matched" => {
                let id = m.document_id.as_ref().ok_or("matched sem document_id")?;
                check(
                    docs.contains_key(id) && m.candidates.contains(id),
                    "document_id inexistente ou fora dos candidatos",
                )?;
                used.insert(id.clone());
            }
            "ambiguous" => {
                check(
                    m.document_id.is_none() && !m.candidates.is_empty(),
                    "ambiguous exige candidatos e document_id null",
                )?;
                unmatched.insert(m.node_id);
            }
            "unmatched" => {
                check(
                    m.document_id.is_none(),
                    "unmatched não admite documento confirmado",
                )?;
                unmatched.insert(m.node_id);
            }
            _ => return Err("status de correspondência inválido".into()),
        }
        check(
            links.insert(m.node_id, m).is_none(),
            "correspondência duplicada",
        )?;
    }
    check(
        links.len() == nodes.len(),
        "correspondência ausente: deve existir uma entrada por nó",
    )?;
    let expected_docs: BTreeSet<_> = docs
        .keys()
        .filter(|id| !used.contains(*id))
        .cloned()
        .collect();
    check(
        map.unmatched_node_ids.len() == unmatched.len()
            && map
                .unmatched_node_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                == unmatched,
        "unmatched_node_ids inconsistente",
    )?;
    check(
        map.unmatched_document_ids.len() == expected_docs.len()
            && map
                .unmatched_document_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
                == expected_docs,
        "unmatched_document_ids inconsistente",
    )?;
    inv.relations
        .sort_by(|a, b| (&a.from, &a.to, &a.kind).cmp(&(&b.from, &b.to, &b.kind)));
    if let Some(rs) = &mut inv.pending_relations {
        rs.sort();
    }
    let entries: Vec<_> = nodes.into_iter().map(|(id,n)| {
        let m=&links[&id];
        let document = m.document_id.as_ref().map(|id| {
            let s=&docs[id]; json!({"symbol":s,"safe_url":s.documentation_url.as_ref().filter(|u|safe_url(u))})
        });
        let candidates: Vec<_> = m.candidates.iter().map(|id| { let d=&docs[id];
            json!({"id":id,"name":d.name,"qualified_name":d.qualified_name,"signature":d.signature,"location":d.location}) }).collect();
        json!({"node_id":id.to_string(),"qualified_name":n.qualified_name,"signature":n.signature,"location":n.location,
            "status":m.status,"evidence":m.evidence,"candidates":candidates,"document":document})
    }).collect();
    Ok(
        json!({"entries":entries,"producer":inv.producer,"coverage":inv.coverage,
        "inventory_digest":map.inventory_digest,"catalog_digest":map.catalog_digest,"matches_digest":digest(matches),
        "graph_digest":cat.graph_digest,"graph_binding":"not_verified",
        "document_relations":inv.relations,"pending_relations":inv.pending_relations.unwrap_or_default(),
        "catalog_diagnostics":cat.diagnostics,"inventory_diagnostics":inv.diagnostics,"match_diagnostics":map.diagnostics,
        "unmatched_document_ids":expected_docs}),
    )
}

pub struct Response {
    pub code: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}
pub fn route(method: &str, path: &str, data: &[u8]) -> Response {
    let (code, content_type, body): (_, _, &[u8]) = if method != "GET" {
        (405, "text/plain; charset=utf-8", b"Metodo nao permitido")
    } else {
        match path {
            "/" => (
                200,
                "text/html; charset=utf-8",
                include_bytes!("../assets/index.html"),
            ),
            "/app.js" => (
                200,
                "text/javascript; charset=utf-8",
                include_bytes!("../assets/app.js"),
            ),
            "/style.css" => (
                200,
                "text/css; charset=utf-8",
                include_bytes!("../assets/style.css"),
            ),
            "/api/data" => (200, "application/json; charset=utf-8", data),
            _ => (404, "text/plain; charset=utf-8", b"Rota inexistente"),
        }
    };
    Response {
        code,
        content_type,
        body: body.to_vec(),
    }
}
pub fn serve_connection(mut stream: TcpStream, data: &[u8]) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut request = Vec::new();
    let mut buffer = [0; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") && request.len() < 8192 {
        let n = stream.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..n]);
    }
    let text = std::str::from_utf8(&request).unwrap_or("");
    let parts: Vec<_> = text
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect();
    let response = if request.len() >= 8192 {
        Response {
            code: 431,
            content_type: "text/plain",
            body: b"Cabecalho grande".to_vec(),
        }
    } else if !request.ends_with(b"\r\n\r\n")
        || parts.len() != 3
        || !matches!(parts[2], "HTTP/1.0" | "HTTP/1.1")
    {
        Response {
            code: 400,
            content_type: "text/plain",
            body: b"Requisicao invalida".to_vec(),
        }
    } else {
        route(parts[0], parts[1], data)
    };
    let reason = match response.code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Request Header Fields Too Large",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'\r\n{}\r\n",
        response.code,
        reason,
        response.content_type,
        response.body.len(),
        if response.code == 405 {
            "Allow: GET\r\n"
        } else {
            ""
        }
    )?;
    stream.write_all(&response.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    const INV: &[u8] = include_bytes!("../fixtures/inventory.json");
    const CAT: &[u8] = include_bytes!("../fixtures/catalog.json");
    const MAP: &[u8] = include_bytes!("../fixtures/matches.json");
    fn values() -> (Value, Value, Value) {
        (
            serde_json::from_slice(INV).unwrap(),
            serde_json::from_slice(CAT).unwrap(),
            serde_json::from_slice(MAP).unwrap(),
        )
    }
    fn rebind(i: Value, c: Value, mut m: Value) -> Result<Value, String> {
        let i = serde_json::to_vec(&i).unwrap();
        let c = serde_json::to_vec(&c).unwrap();
        m["inventory_digest"] = json!(digest(&i));
        m["catalog_digest"] = json!(digest(&c));
        prepare(&i, &c, &serde_json::to_vec(&m).unwrap())
    }
    #[test]
    fn catalog_diagnostics_and_location_validation() {
        let (i, mut c, m) = values();
        c["diagnostics"] = json!([{"code":"opaque","message":"limite","symbol_id":null}]);
        assert_eq!(
            rebind(i.clone(), c.clone(), m.clone()).unwrap()["catalog_diagnostics"],
            c["diagnostics"]
        );
        c["symbols"][0]["location"] = json!({"file":"a","line":0});
        assert!(rebind(i, c, m).is_err());
    }
    #[test]
    fn legacy_v1_resolved_remains_accepted() {
        let (mut i, c, m) = values();
        i["version"] = json!(1);
        i.as_object_mut().unwrap().remove("pending_relations");
        assert!(rebind(i, c, m).is_ok());
    }
    #[test]
    fn inventory_v2_pending_is_preserved_and_not_resolved() {
        let (mut i, c, m) = values();
        i["version"] = json!(2);
        i["coverage"] = json!("partial");
        let id = i["symbols"][0]["id"].clone();
        i["pending_relations"] =
            json!([{"from":id,"to":"missing","kind":"references","reason":"missing_to"}]);
        let result = rebind(i.clone(), c.clone(), m.clone()).unwrap();
        assert_eq!(result["pending_relations"], i["pending_relations"]);
        i["coverage"] = json!("complete");
        assert!(rebind(i.clone(), c.clone(), m.clone()).is_err());
        i["coverage"] = json!("partial");
        i["pending_relations"][0]["reason"] = json!("missing_from");
        assert!(rebind(i.clone(), c.clone(), m.clone()).is_err());
        i["pending_relations"][0]["reason"] = json!("missing_to");
        i["version"] = json!(1);
        assert!(rebind(i.clone(), c.clone(), m.clone()).is_err());
        i["version"] = json!(2);
        i.as_object_mut().unwrap().remove("pending_relations");
        assert!(rebind(i.clone(), c.clone(), m.clone()).is_err());
    }
    #[test]
    fn states_and_correct_node() {
        let d = prepare(INV, CAT, MAP).unwrap();
        let e = d["entries"].as_array().unwrap();
        assert_eq!(e[0]["node_id"], "10");
        assert_eq!(e[0]["document"]["symbol"]["id"], "servico");
        assert_eq!(e[1]["status"], "ambiguous");
        assert!(e[1]["document"].is_null());
        assert_eq!(e[1]["candidates"].as_array().unwrap().len(), 2);
        assert_eq!(e[2]["status"], "unmatched");
        assert!(e[2]["document"].is_null());
        assert_eq!(d["coverage"], "partial");
        assert_eq!(d["inventory_diagnostics"][0]["code"], "entrada_parcial");
    }
    #[test]
    fn empty_unicode_and_literal_markup_preserved() {
        let d = prepare(INV, CAT, MAP).unwrap();
        assert_eq!(d["entries"][3]["document"]["symbol"]["brief"], "");
        let text = d["entries"][0]["document"]["symbol"]["details"]
            .as_str()
            .unwrap();
        assert!(text.contains("日本語"));
        assert!(text.contains("<img"));
        assert!(d["entries"][3]["document"]["safe_url"].is_null());
    }
    #[test]
    fn exact_bytes_binding() {
        let mut altered = INV.to_vec();
        altered.push(b' ');
        assert!(
            prepare(&altered, CAT, MAP)
                .unwrap_err()
                .contains("inventory_digest")
        );
        let mut altered = CAT.to_vec();
        altered.push(b'\n');
        assert!(
            prepare(INV, &altered, MAP)
                .unwrap_err()
                .contains("catalog_digest")
        );
    }
    #[test]
    fn schemas_and_versions() {
        for index in 0..3 {
            let (mut i, mut c, mut m) = values();
            [&mut i, &mut c, &mut m][index]["version"] = json!(3);
            assert!(rebind(i, c, m).unwrap_err().contains("schema/version"));
        }
        let (mut i, c, m) = values();
        i["schema"] = json!("other");
        assert!(rebind(i, c, m).is_err());
    }
    #[test]
    fn duplicate_ids_and_missing_endpoints() {
        let (mut i, c, m) = values();
        let s = i["symbols"][0].clone();
        i["symbols"].as_array_mut().unwrap().push(s);
        assert!(rebind(i, c, m).unwrap_err().contains("duplicado"));
        let (mut i, c, m) = values();
        i["relations"][0]["to"] = json!("missing");
        assert!(rebind(i, c, m).unwrap_err().contains("endpoint"));
        let (i, mut c, m) = values();
        c["symbols"][1]["node_id"] = json!(10);
        assert!(rebind(i, c, m).is_err());
    }
    #[test]
    fn bad_associations_are_rejected() {
        for field in ["node_id", "document_id", "status"] {
            let (i, c, mut m) = values();
            m["matches"][0][field] = match field {
                "node_id" => json!(999),
                _ => json!("missing"),
            };
            assert!(rebind(i, c, m).is_err());
        }
        let (i, c, mut m) = values();
        m["matches"][1]["document_id"] = json!("a");
        assert!(rebind(i, c, m).is_err());
        let (i, c, mut m) = values();
        m["matches"].as_array_mut().unwrap().pop();
        assert!(rebind(i, c, m).unwrap_err().contains("ausente"));
        let (i, c, mut m) = values();
        m["matches"][1]["candidates"] = json!(["missing"]);
        assert!(rebind(i, c, m).is_err());
    }
    #[test]
    fn inconsistent_unmatched_lists_rejected() {
        let (i, c, mut m) = values();
        m["unmatched_node_ids"] = json!([30]);
        assert!(rebind(i, c, m).is_err());
        let (i, c, mut m) = values();
        m["unmatched_document_ids"] = json!([]);
        assert!(rebind(i, c, m).is_err());
    }
    #[test]
    fn optional_fields_remain_unknown_and_large_id_precise() {
        let (mut i, mut c, mut m) = values();
        i["producer"].as_object_mut().unwrap().remove("version");
        c["symbols"][0].as_object_mut().unwrap().remove("location");
        c["symbols"][0]["node_id"] = json!(u64::MAX);
        m["matches"][0]["node_id"] = json!(u64::MAX);
        let d = rebind(i, c, m).unwrap();
        assert!(d["producer"]["version"].is_null());
        let last = &d["entries"][3];
        assert_eq!(last["node_id"], u64::MAX.to_string());
        assert!(last["location"].is_null());
    }
    #[test]
    fn empty_inventory_catalog() {
        let (mut i, mut c, mut m) = values();
        i["symbols"] = json!([]);
        i["relations"] = json!([]);
        c["symbols"] = json!([]);
        m["matches"] = json!([]);
        m["unmatched_node_ids"] = json!([]);
        m["unmatched_document_ids"] = json!([]);
        assert_eq!(rebind(i, c, m).unwrap()["entries"], json!([]));
    }
    #[test]
    fn deterministic_sorting() {
        let (i, mut c, mut m) = values();
        let expected = rebind(i.clone(), c.clone(), m.clone()).unwrap();
        c["symbols"].as_array_mut().unwrap().reverse();
        m["matches"].as_array_mut().unwrap().reverse();
        assert_eq!(rebind(i, c, m).unwrap()["entries"], expected["entries"]);
        assert_eq!(
            prepare(INV, CAT, MAP).unwrap(),
            prepare(INV, CAT, MAP).unwrap()
        );
    }
    #[test]
    fn url_policy() {
        for u in ["https://example.org/docs#item", "http://127.0.0.1:8080/a"] {
            assert!(safe_url(u), "{u}");
        }
        for u in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "//example.org",
            "https://u:p@example.org",
            "https://evil\\example.org",
            "https://example.org/\n",
            "https://",
            "https://x:0",
            "https://x:99999",
            "https://[::1]",
            "https://á.org",
        ] {
            assert!(!safe_url(u), "{u}");
        }
    }
    #[test]
    fn routes_are_exact_and_read_only() {
        for path in ["/", "/app.js", "/style.css", "/api/data"] {
            assert_eq!(route("GET", path, b"{}").code, 200);
        }
        for path in [
            "/../Cargo.toml",
            "/%2e%2e/secret",
            "/fixtures/inventory.json",
            "/api/data?file=/etc/passwd",
        ] {
            assert_eq!(route("GET", path, b"{}").code, 404);
        }
        assert_eq!(route("POST", "/api/data", b"{}").code, 405);
        let js = include_str!("../assets/app.js");
        assert!(!js.contains("innerHTML"));
        assert!(js.contains("textContent"));
    }
    #[test]
    fn real_http_response() {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            serve_connection(s, b"{\"ok\":true}").unwrap();
        });
        let mut client = TcpStream::connect(addr).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        client
            .write_all(b"GET /api/data HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        worker.join().unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("Content-Security-Policy:"));
        assert!(response.contains("Content-Length: 11"));
        assert!(response.ends_with("{\"ok\":true}"));
    }
}
