//! Crystalline Lineage
//! @prompt lab/regras-arquiteturais/CONTRATO.md
//! Local JSON boundary; no graph extraction or subprocesses.
use regras_arquiteturais::*;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextDto {
    project: String,
    provider: String,
    provider_version: String,
    granularity: String,
    nature: String,
    configuration: String,
    scope: String,
}
impl From<ContextDto> for Context {
    fn from(c: ContextDto) -> Self {
        Self {
            project: c.project,
            provider: c.provider,
            provider_version: c.provider_version,
            granularity: c.granularity,
            nature: c.nature,
            configuration: c.configuration,
            scope: c.scope,
        }
    }
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum StateDto {
    Complete,
    Partial,
    #[default]
    Unknown,
}
impl From<StateDto> for Completeness {
    fn from(s: StateDto) -> Self {
        match s {
            StateDto::Complete => Self::Complete,
            StateDto::Partial => Self::Partial,
            StateDto::Unknown => Self::Unknown,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum CapDto {
    DirectDependencies,
    Reachability,
    CyclicComponents,
}
impl From<CapDto> for Capability {
    fn from(c: CapDto) -> Self {
        match c {
            CapDto::DirectDependencies => Self::DirectDependencies,
            CapDto::Reachability => Self::Reachability,
            CapDto::CyclicComponents => Self::CyclicComponents,
        }
    }
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct QualityDto {
    #[serde(default)]
    execution: StateDto,
    #[serde(default)]
    coverage: StateDto,
    #[serde(default)]
    capabilities: Vec<CapDto>,
    #[serde(default)]
    diagnostics: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityDto {
    node_id: usize,
    identity: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotDto {
    version: u32,
    graph: String,
    graph_sha256: String,
    context: ContextDto,
    #[serde(default)]
    quality: QualityDto,
    identities: Vec<IdentityDto>,
    source_metadata: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectorDto {
    #[serde(default)]
    ids: Vec<String>,
    #[serde(default)]
    prefixes: Vec<String>,
}
impl From<SelectorDto> for Selector {
    fn from(s: SelectorDto) -> Self {
        Self {
            ids: s.ids,
            prefixes: s.prefixes,
        }
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum PolicyDto {
    ForbidDirect { from: SelectorDto, to: SelectorDto },
    ForbidReachability { from: SelectorDto, to: SelectorDto },
    NoNewCycles { scope: SelectorDto },
}
impl From<PolicyDto> for Policy {
    fn from(p: PolicyDto) -> Self {
        match p {
            PolicyDto::ForbidDirect { from, to } => Self::ForbidDirect {
                from: from.into(),
                to: to.into(),
            },
            PolicyDto::ForbidReachability { from, to } => Self::ForbidReachability {
                from: from.into(),
                to: to.into(),
            },
            PolicyDto::NoNewCycles { scope } => Self::NoNewCycles {
                scope: scope.into(),
            },
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleDto {
    id: String,
    policy: PolicyDto,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ScopeDto {
    Pairs { from: SelectorDto, to: SelectorDto },
    Component { members: SelectorDto },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExceptionDto {
    id: String,
    rule_id: String,
    scope: ScopeDto,
    justification: String,
    until: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum EnforcementDto {
    All,
    NewOnly,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesDto {
    version: u32,
    enforcement: EnforcementDto,
    rules: Vec<RuleDto>,
    #[serde(default)]
    exceptions: Vec<ExceptionDto>,
}
fn bytes(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}
pub fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T> {
    serde_json::from_slice(data).map_err(|e| format!("JSON: {e}"))
}
fn relative(doc: &Path, value: &str) -> PathBuf {
    doc.parent().unwrap_or(Path::new(".")).join(value)
}
pub fn load_rules(path: &Path) -> Result<(Rules, Value)> {
    let b = bytes(path)?;
    let dto: RulesDto = read(&b)?;
    if dto.version != 1 {
        return Err("Versão de regras não suportada".into());
    }
    let mut exceptions = Vec::new();
    for e in dto.exceptions {
        exceptions.push(Exception {
            id: e.id,
            rule_id: e.rule_id,
            justification: e.justification,
            until: e.until.as_deref().map(Date::parse).transpose()?,
            scope: match e.scope {
                ScopeDto::Pairs { from, to } => ExceptionScope::Pairs {
                    from: from.into(),
                    to: to.into(),
                },
                ScopeDto::Component { members } => ExceptionScope::Component {
                    members: members.into(),
                },
            },
        });
    }
    Ok((
        Rules {
            enforcement: match dto.enforcement {
                EnforcementDto::All => Enforcement::All,
                EnforcementDto::NewOnly => Enforcement::NewOnly,
            },
            rules: dto
                .rules
                .into_iter()
                .map(|r| Rule {
                    id: r.id,
                    policy: r.policy.into(),
                })
                .collect(),
            exceptions,
        },
        json!({"path":path,"sha256":digest(&b),"document":read::<Value>(&b)?}),
    ))
}
fn source_quality(meta: &Value, quality: &mut Quality) {
    // Delivery 13: unknown schemas do not acquire semantic interpretation.
    let schema_known = match meta["provider"].as_str() {
        Some("typescript-compiler-api-experimental" | "zig-ast-experimental") => {
            meta["schema_version"] == 1
        }
        Some("experimental-python-ast" | "experimental-libclang-cpp") => {
            meta["metadata_version"] == 1
        }
        Some("experimental-go-list") => meta["contract_version"] == 1,
        Some("libclang") => true, // C contract presently has no schema version.
        _ => false,
    };
    if !schema_known {
        quality.coverage = Completeness::Unknown;
        quality
            .diagnostics
            .push("Provedor/schema dos metadados de origem desconhecido; nenhuma aprovação".into());
    }
    let mut recognized = false;
    for pointer in [
        "/execution/status",
        "/status",
        "/loading",
        "/coverage",
        "/semantic_coverage",
        "/completeness/syntax",
        "/completeness/semantics",
        "/semantic_completeness",
    ] {
        if let Some(value) = meta.pointer(pointer) {
            recognized = true;
            let good = matches!(
                value.as_str(),
                Some("complete" | "complete-within-declared-scope")
            );
            if !good {
                quality.coverage = if matches!(value.as_str(), Some("partial" | "error" | "failed"))
                {
                    Completeness::Partial
                } else {
                    Completeness::Unknown
                };
                quality.diagnostics.push(format!(
                    "Metadados de origem bloqueiam conclusão: {pointer}={value}"
                ));
            }
        }
    }
    if !recognized {
        quality.coverage = Completeness::Unknown;
        quality
            .diagnostics
            .push("Metadados de origem sem estado reconhecido; qualidade desconhecida".into());
    }
}
pub fn load_snapshot(path: &Path) -> Result<(Snapshot, Value)> {
    let b = bytes(path)?;
    let dto: SnapshotDto = read(&b)?;
    if dto.version != 1 {
        return Err("Versão de snapshot não suportada".into());
    }
    let graph_path = relative(path, &dto.graph);
    let graph_bytes = bytes(&graph_path)?;
    let graph_hash = digest(&graph_bytes);
    if graph_hash != dto.graph_sha256 {
        return Err("graph_sha256 não corresponde aos bytes do grafo".into());
    }
    let graph = lente_infra::desserializar_grafo(
        std::str::from_utf8(&graph_bytes).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Grafo: {e:?}"))?;
    let mut quality = Quality {
        execution: dto.quality.execution.into(),
        coverage: dto.quality.coverage.into(),
        capabilities: dto
            .quality
            .capabilities
            .into_iter()
            .map(Into::into)
            .collect(),
        diagnostics: dto.quality.diagnostics,
    };
    let mut provenance = json!({"path":path,"sha256":digest(&b),"graph_path":graph_path,"graph_sha256":graph_hash,"declaration":read::<Value>(&b)?,"trust":"explicit-declaration-not-authenticated"});
    if let Some(source) = dto.source_metadata {
        let p = relative(path, &source);
        let raw = bytes(&p)?;
        let metadata: Value = read(&raw)?;
        if !metadata.is_object() {
            return Err("Metadados de origem devem ser objeto".into());
        }
        if let Some(hash) = metadata.get("graph_sha256")
            && hash.as_str() != Some(graph_hash.as_str())
        {
            return Err("Digest dos metadados nativos não corresponde ao grafo".into());
        }
        source_quality(&metadata, &mut quality);
        provenance["source_metadata"] = json!({"path":p,"sha256":digest(&raw),"document":metadata});
    }
    Ok((
        Snapshot {
            graph,
            context: dto.context.into(),
            quality,
            identities: dto
                .identities
                .into_iter()
                .map(|i| (i.node_id, i.identity))
                .collect(),
        },
        provenance,
    ))
}
fn origins(origins: &BTreeMap<String, Origin>) -> Value {
    json!(
        origins
            .iter()
            .map(|(k, o)| (
                k.clone(),
                json!({"node_id":o.node_id,"path":o.path,"name":o.name})
            ))
            .collect::<BTreeMap<_, _>>()
    )
}
pub fn report_json(
    report: &Report,
    as_of: &str,
    baseline: Value,
    current: Value,
    rules: Value,
) -> Value {
    let results:Vec<_>=report.rules.iter().map(|r|json!({"rule_id":r.id,"status":r.status,"diagnostics":r.diagnostics,"findings":r.findings.iter().map(|f|json!({"id":f.id,"rule_id":f.rule_id,"kind":f.kind,"evolution":f.evolution.text(),"from":f.from,"to":f.to,"members":f.members,"internal_edges":f.internal_edges,"added_internal_edges":f.added_internal_edges,"witness":f.witness,"hops":f.witness.len().saturating_sub(1),"dependency":if f.members.is_empty(){if f.witness.len()==2{"direct"}else{"transitive"}}else{"cyclic-component"},"exception_ids":f.exceptions,"enforced":f.enforced})).collect::<Vec<_>>()})).collect();
    json!({"report_version":1,"status":report.status.text(),"as_of":as_of,"enforcement":if report.enforcement==Enforcement::All{"all"}else{"new_only"},"rules":results,
        "exceptions":report.exceptions.iter().map(|e|json!({"id":e.id,"rule_id":e.rule_id,"status":e.status,"justification":e.justification,"finding_ids":e.findings})).collect::<Vec<_>>(),
        "origins":{"baseline":origins(&report.baseline_origins),"current":origins(&report.current_origins)},"diagnostics":report.diagnostics,
        "inputs":{"baseline":baseline,"current":current,"rules":rules},
        "limits":["Aprovação refere-se à política, enforcement e escopo declarados; não comprova correção funcional.","Identidade e capacidade são declarações explícitas; hashes fixam conteúdo, não autenticam o produtor.","Ciclos são regressões de componentes cíclicos, não enumeração de todos os ciclos simples.","Owns e mudanças de subtipo Uses não propagam esta política de dependências."]})
}
pub fn text_report(v: &Value) -> String {
    let mut out = format!(
        "Resultado: {} | enforcement={} | as_of={}\n",
        v["status"], v["enforcement"], v["as_of"]
    );
    for rule in v["rules"].as_array().unwrap() {
        out.push_str(&format!(
            "\nRegra {}: {}\n",
            rule["rule_id"], rule["status"]
        ));
        for d in rule["diagnostics"].as_array().unwrap() {
            out.push_str(&format!("  {d}\n"));
        }
        for f in rule["findings"].as_array().unwrap() {
            out.push_str(&format!(
                "  {} | {} | aplicada={} | exceções={}\n    witness={} | hops={} | membros={}\n",
                f["evolution"],
                f["id"],
                f["enforced"],
                f["exception_ids"],
                f["witness"],
                f["hops"],
                f["members"]
            ));
        }
    }
    out.push_str("\nExceções (separadas de aprovação):\n");
    for e in v["exceptions"].as_array().unwrap() {
        out.push_str(&format!(
            "  {}: {} — {}\n",
            e["id"], e["status"], e["justification"]
        ));
    }
    for d in v["diagnostics"].as_array().unwrap() {
        out.push_str(&format!("Diagnóstico: {d}\n"));
    }
    out.push_str("\nAprovação arquitetural no escopo declarado não comprova correção funcional ou completude do extrator.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_quality_is_unknown_not_complete() {
        let dto: QualityDto = read(b"{}").unwrap();
        assert!(matches!(dto.execution, StateDto::Unknown));
        assert!(dto.capabilities.is_empty());
    }
    #[test]
    fn partial_native_metadata_cannot_be_overridden() {
        let mut q = Quality {
            execution: Completeness::Complete,
            coverage: Completeness::Complete,
            capabilities: Default::default(),
            diagnostics: vec![],
        };
        source_quality(
            &json!({"execution":{"status":"complete"},"semantic_coverage":"partial"}),
            &mut q,
        );
        assert_eq!(q.coverage, Completeness::Partial);
        source_quality(&json!({"unknown_schema":true}), &mut q);
        assert_eq!(q.coverage, Completeness::Unknown);
    }
    #[test]
    fn unknown_native_schema_cannot_claim_complete() {
        let mut q = Quality {
            execution: Completeness::Complete,
            coverage: Completeness::Complete,
            capabilities: Default::default(),
            diagnostics: vec![],
        };
        source_quality(
            &json!({"provider":"experimental-python-ast","metadata_version":999,"execution":{"status":"complete"}}),
            &mut q,
        );
        assert_eq!(q.coverage, Completeness::Unknown);
    }
    #[test]
    fn strict_formats_reject_unknown_fields_and_versions() {
        assert!(read::<SelectorDto>(br#"{"glob":"**"}"#).is_err());
        assert!(read::<PolicyDto>(br#"{"kind":"forbid_magic","scope":{"ids":["A"]}}"#).is_err());
    }
    #[test]
    fn example_binding_and_versions_are_valid() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
        load_snapshot(&base.join("baseline.json")).unwrap();
        load_snapshot(&base.join("current.json")).unwrap();
        load_rules(&base.join("rules.json")).unwrap();
    }
}
