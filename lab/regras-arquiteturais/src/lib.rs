//! Crystalline Lineage
//! @prompt lab/regras-arquiteturais/CONTRATO.md
//! Pure architectural policy evaluation: no I/O, clock, serde or hashing.
use lente_core::{
    domain::raio::explicar_caminho_impacto,
    entities::grafo::{Aresta, Grafo, Relation},
};
use lente_estrutura::detectar_ciclos;
use std::collections::{BTreeMap, BTreeSet};

pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub project: String,
    pub provider: String,
    pub provider_version: String,
    pub granularity: String,
    pub nature: String,
    pub configuration: String,
    pub scope: String,
}
impl Context {
    fn known(&self) -> bool {
        [
            &self.project,
            &self.provider,
            &self.provider_version,
            &self.granularity,
            &self.nature,
            &self.configuration,
            &self.scope,
        ]
        .iter()
        .all(|s| !s.trim().is_empty() && s.as_str() != "unknown")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completeness {
    Complete,
    Partial,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    DirectDependencies,
    Reachability,
    CyclicComponents,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quality {
    pub execution: Completeness,
    pub coverage: Completeness,
    pub capabilities: BTreeSet<Capability>,
    pub diagnostics: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub graph: Grafo,
    pub context: Context,
    pub quality: Quality,
    pub identities: Vec<(usize, String)>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selector {
    pub ids: Vec<String>,
    pub prefixes: Vec<String>,
}
impl Selector {
    fn validate(&self) -> Result<()> {
        if self.ids.is_empty() && self.prefixes.is_empty() {
            return Err("Seletor vazio".into());
        }
        for list in [&self.ids, &self.prefixes] {
            if list.iter().any(|s| s.is_empty())
                || list.iter().collect::<BTreeSet<_>>().len() != list.len()
            {
                return Err("Termo vazio/repetido em seletor".into());
            }
        }
        Ok(())
    }
    fn matches(&self, id: &str) -> bool {
        self.ids.iter().any(|s| s == id) || self.prefixes.iter().any(|p| id.starts_with(p))
    }
    fn select(&self, names: &BTreeSet<String>) -> (BTreeSet<String>, Vec<String>) {
        let mut issues = Vec::new();
        for id in &self.ids {
            if !names.contains(id) {
                issues.push(format!("Identidade sem correspondência no atual: {id}"));
            }
        }
        for p in &self.prefixes {
            if !names.iter().any(|n| n.starts_with(p)) {
                issues.push(format!("Prefixo sem correspondência no atual: {p}"));
            }
        }
        (
            names.iter().filter(|n| self.matches(n)).cloned().collect(),
            issues,
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Policy {
    ForbidDirect { from: Selector, to: Selector },
    ForbidReachability { from: Selector, to: Selector },
    NoNewCycles { scope: Selector },
}
impl Policy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ForbidDirect { .. } => "forbid_direct",
            Self::ForbidReachability { .. } => "forbid_reachability",
            Self::NoNewCycles { .. } => "no_new_cycles",
        }
    }
    fn required(&self) -> BTreeSet<Capability> {
        let mut s = BTreeSet::from([Capability::DirectDependencies]);
        match self {
            Self::ForbidDirect { .. } => {}
            Self::ForbidReachability { .. } => {
                s.insert(Capability::Reachability);
            }
            Self::NoNewCycles { .. } => {
                s.insert(Capability::Reachability);
                s.insert(Capability::CyclicComponents);
            }
        }
        s
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub id: String,
    pub policy: Policy,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Enforcement {
    All,
    NewOnly,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    year: u32,
    month: u32,
    day: u32,
}
impl Date {
    pub fn parse(s: &str) -> Result<Self> {
        let b = s.as_bytes();
        if b.len() != 10
            || b[4] != b'-'
            || b[7] != b'-'
            || !b
                .iter()
                .enumerate()
                .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
        {
            return Err(format!("Data inválida: {s}"));
        }
        let year = s[..4].parse::<u32>().unwrap();
        let month = s[5..7].parse::<u32>().unwrap();
        let day = s[8..].parse::<u32>().unwrap();
        let leap =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let max = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if leap {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        };
        if year == 0 || day == 0 || day > max {
            return Err(format!("Data inválida: {s}"));
        }
        Ok(Self { year, month, day })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExceptionScope {
    Pairs { from: Selector, to: Selector },
    Component { members: Selector },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exception {
    pub id: String,
    pub rule_id: String,
    pub scope: ExceptionScope,
    pub justification: String,
    pub until: Option<Date>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    pub enforcement: Enforcement,
    pub rules: Vec<Rule>,
    pub exceptions: Vec<Exception>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Approved,
    Violation,
    Inconclusive,
}
impl Status {
    pub fn text(self) -> &'static str {
        match self {
            Self::Approved => "aprovado",
            Self::Violation => "violacao",
            Self::Inconclusive => "inconclusivo",
        }
    }
    pub fn exit_code(self) -> i32 {
        match self {
            Self::Approved => 0,
            Self::Violation => 1,
            Self::Inconclusive => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evolution {
    Preexisting,
    New,
}
impl Evolution {
    pub fn text(self) -> &'static str {
        match self {
            Self::Preexisting => "preexisting",
            Self::New => "new",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub id: String,
    pub rule_id: String,
    pub kind: String,
    pub evolution: Evolution,
    pub from: Option<String>,
    pub to: Option<String>,
    pub members: Vec<String>,
    pub internal_edges: Vec<(String, String)>,
    pub added_internal_edges: Vec<(String, String)>,
    pub witness: Vec<String>,
    pub exceptions: Vec<String>,
    pub enforced: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleResult {
    pub id: String,
    pub status: String,
    pub diagnostics: Vec<String>,
    pub findings: Vec<Finding>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExceptionResult {
    pub id: String,
    pub rule_id: String,
    pub status: String,
    pub justification: String,
    pub findings: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    pub node_id: usize,
    pub path: String,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub status: Status,
    pub enforcement: Enforcement,
    pub rules: Vec<RuleResult>,
    pub exceptions: Vec<ExceptionResult>,
    pub baseline_origins: BTreeMap<String, Origin>,
    pub current_origins: BTreeMap<String, Origin>,
    pub diagnostics: Vec<String>,
}
struct Projection {
    graph: Grafo,
    names: BTreeSet<String>,
    edges: BTreeSet<(String, String)>,
    origins: BTreeMap<String, Origin>,
}
fn project(s: &Snapshot) -> Result<Projection> {
    let mut nodes = BTreeMap::new();
    for n in &s.graph.nodes {
        if nodes.insert(n.id, n).is_some() {
            return Err(format!("ID de nó repetido: {}", n.id));
        }
    }
    let mut identities = BTreeMap::new();
    let mut names = BTreeSet::new();
    for (id, key) in &s.identities {
        if key.is_empty()
            || !nodes.contains_key(id)
            || identities.insert(*id, key.clone()).is_some()
            || !names.insert(key.clone())
        {
            return Err("Mapa de identidades vazio, excedente, repetido ou ambíguo".into());
        }
    }
    if identities.len() != nodes.len() {
        return Err("Mapa de identidades não cobre todos os nós".into());
    }
    let mut edges = BTreeSet::new();
    for e in &s.graph.edges {
        let a = nodes.get(&e.id_from).ok_or("Endpoint from inexistente")?;
        let b = nodes.get(&e.id_to).ok_or("Endpoint to inexistente")?;
        if e.from != a.path || e.to != b.path {
            return Err("Texto do endpoint incompatível com ID".into());
        }
        if e.relation == Relation::Uses {
            edges.insert((identities[&e.id_from].clone(), identities[&e.id_to].clone()));
        }
    }
    let ids: BTreeMap<_, _> = names
        .iter()
        .enumerate()
        .map(|(i, k)| (k.clone(), i))
        .collect();
    let mut projected = Vec::new();
    let mut origins = BTreeMap::new();
    for (id, n) in nodes {
        let key = &identities[&id];
        let mut new = n.clone();
        new.id = ids[key];
        new.path = key.clone().into();
        projected.push(new);
        origins.insert(
            key.clone(),
            Origin {
                node_id: id,
                path: n.path.as_str().into(),
                name: n.name.clone(),
            },
        );
    }
    projected.sort_by_key(|n| n.id);
    let graph = Grafo {
        crate_name: s.graph.crate_name.clone(),
        nodes: projected,
        edges: edges
            .iter()
            .map(|(a, b)| Aresta {
                id_from: ids[a],
                id_to: ids[b],
                from: a.clone().into(),
                to: b.clone().into(),
                relation: Relation::Uses,
                uses_kind: None,
            })
            .collect(),
    };
    Ok(Projection {
        graph,
        names,
        edges,
        origins,
    })
}
fn validate_rules(rules: &Rules) -> Result<()> {
    if rules.rules.is_empty() {
        return Err("Documento sem regras".into());
    }
    let mut by_id = BTreeMap::new();
    for r in &rules.rules {
        if r.id.trim().is_empty() || by_id.insert(r.id.clone(), &r.policy).is_some() {
            return Err("ID de regra vazio/repetido".into());
        }
        match &r.policy {
            Policy::ForbidDirect { from, to } | Policy::ForbidReachability { from, to } => {
                from.validate()?;
                to.validate()?;
            }
            Policy::NoNewCycles { scope } => scope.validate()?,
        }
    }
    let mut ids = BTreeSet::new();
    for e in &rules.exceptions {
        if e.id.trim().is_empty() || !ids.insert(&e.id) || e.justification.trim().is_empty() {
            return Err("Exceção sem ID/justificativa ou ID repetido".into());
        }
        let rule = by_id
            .get(&e.rule_id)
            .ok_or("Exceção aponta regra inexistente")?;
        match (&e.scope, rule) {
            (
                ExceptionScope::Pairs { from, to },
                Policy::ForbidDirect { .. } | Policy::ForbidReachability { .. },
            ) => {
                from.validate()?;
                to.validate()?;
            }
            (ExceptionScope::Component { members }, Policy::NoNewCycles { .. }) => {
                members.validate()?
            }
            _ => return Err("Escopo da exceção incompatível com regra".into()),
        }
    }
    Ok(())
}
/// Uses the existing BFS. Positive-length reachability handles same endpoints
/// separately, because the public explanation API returns the zero-hop path.
fn witness(p: &Projection, from: &str, to: &str) -> Result<Option<Vec<String>>> {
    if !p.names.contains(from) || !p.names.contains(to) {
        return Ok(None);
    }
    if from != to {
        return explicar_caminho_impacto(&p.graph, &to.into(), &from.into())
            .map(|r| r.map(|v| v.into_iter().map(|p| p.as_str().to_owned()).collect()))
            .map_err(|e| format!("Caminho: {e:?}"));
    }
    let mut candidates = Vec::new();
    for (a, b) in &p.edges {
        if a != from {
            continue;
        }
        if b == from {
            candidates.push(vec![from.to_owned(), from.to_owned()]);
            continue;
        }
        if let Some(mut back) = witness(p, b, from)? {
            back.insert(0, from.into());
            candidates.push(back);
        }
    }
    candidates.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));
    Ok(candidates.into_iter().next())
}
#[derive(Clone)]
struct Component {
    members: BTreeSet<String>,
    edges: BTreeSet<(String, String)>,
}
fn components(p: &Projection) -> Vec<Component> {
    let mut groups: Vec<BTreeSet<String>> = detectar_ciclos(&p.graph)
        .into_iter()
        .map(|c| c.modulos.into_iter().map(|p| p.as_str().into()).collect())
        .collect();
    let covered: BTreeSet<_> = groups.iter().flatten().cloned().collect();
    for (a, b) in &p.edges {
        if a == b && !covered.contains(a) {
            groups.push(BTreeSet::from([a.clone()]));
        }
    }
    groups.sort();
    groups
        .into_iter()
        .map(|members| Component {
            edges: p
                .edges
                .iter()
                .filter(|(a, b)| members.contains(a) && members.contains(b))
                .cloned()
                .collect(),
            members,
        })
        .collect()
}
fn violation_id(rule: &str, kind: &str, parts: impl IntoIterator<Item = String>) -> String {
    let mut out = "architectural-violation-v1:".to_owned();
    for s in [rule.to_owned(), kind.to_owned()].into_iter().chain(parts) {
        out.push_str(&format!("{}:{s}", s.len()));
    }
    out
}
fn findings(
    rule: &Rule,
    old: &Projection,
    new: &Projection,
) -> Result<(Vec<Finding>, Vec<String>)> {
    let mut result = Vec::new();
    let mut issues = Vec::new();
    match &rule.policy {
        Policy::ForbidDirect { from, to } | Policy::ForbidReachability { from, to } => {
            let (a, mut i) = from.select(&new.names);
            issues.append(&mut i);
            let (b, mut i) = to.select(&new.names);
            issues.append(&mut i);
            if !issues.is_empty() {
                return Ok((result, issues));
            }
            for from in a {
                for to in &b {
                    let direct = matches!(rule.policy, Policy::ForbidDirect { .. });
                    let path = if direct {
                        new.edges
                            .contains(&(from.clone(), to.clone()))
                            .then(|| vec![from.clone(), to.clone()])
                    } else {
                        witness(new, &from, to)?
                    };
                    if let Some(path) = path {
                        let existed = if direct {
                            old.edges.contains(&(from.clone(), to.clone()))
                        } else {
                            witness(old, &from, to)?.is_some()
                        };
                        result.push(Finding {
                            id: violation_id(
                                &rule.id,
                                rule.policy.label(),
                                [from.clone(), to.clone()],
                            ),
                            rule_id: rule.id.clone(),
                            kind: rule.policy.label().into(),
                            evolution: if existed {
                                Evolution::Preexisting
                            } else {
                                Evolution::New
                            },
                            from: Some(from.clone()),
                            to: Some(to.clone()),
                            members: vec![],
                            internal_edges: vec![],
                            added_internal_edges: vec![],
                            witness: path,
                            exceptions: vec![],
                            enforced: false,
                        });
                    }
                }
            }
        }
        Policy::NoNewCycles { scope } => {
            let (selected, i) = scope.select(&new.names);
            issues = i;
            if !issues.is_empty() {
                return Ok((result, issues));
            }
            let before = components(old);
            for c in components(new) {
                if c.members.is_disjoint(&selected) {
                    continue;
                }
                let existed = before
                    .iter()
                    .any(|b| c.members.is_subset(&b.members) && c.edges.is_subset(&b.edges));
                let mut parts = vec![c.members.len().to_string()];
                parts.extend(c.members.iter().cloned());
                parts.push(c.edges.len().to_string());
                for (a, b) in &c.edges {
                    parts.extend([a.clone(), b.clone()]);
                }
                let added: Vec<_> = c.edges.difference(&old.edges).cloned().collect();
                let first = c.members.first().unwrap().clone();
                let path = if let Some((a, b)) = added.first() {
                    if a == b {
                        vec![a.clone(), a.clone()]
                    } else {
                        let mut back =
                            witness(new, b, a)?.ok_or("Aresta interna sem retorno no SCC")?;
                        back.insert(0, a.clone());
                        back
                    }
                } else {
                    witness(new, &first, &first)?.ok_or("SCC sem testemunha positiva")?
                };
                result.push(Finding {
                    id: violation_id(&rule.id, rule.policy.label(), parts),
                    rule_id: rule.id.clone(),
                    kind: "cyclic-component-regression".into(),
                    evolution: if existed {
                        Evolution::Preexisting
                    } else {
                        Evolution::New
                    },
                    from: None,
                    to: None,
                    members: c.members.into_iter().collect(),
                    internal_edges: c.edges.into_iter().collect(),
                    added_internal_edges: added,
                    witness: path,
                    exceptions: vec![],
                    enforced: false,
                });
            }
        }
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    issues.sort();
    issues.dedup();
    Ok((result, issues))
}
fn exception_matches(e: &Exception, f: &Finding) -> bool {
    if e.rule_id != f.rule_id {
        return false;
    }
    match &e.scope {
        ExceptionScope::Pairs { from, to } => {
            f.from.as_deref().is_some_and(|n| from.matches(n))
                && f.to.as_deref().is_some_and(|n| to.matches(n))
        }
        ExceptionScope::Component { members } => {
            !f.members.is_empty() && f.members.iter().all(|n| members.matches(n))
        }
    }
}
/// Evaluate the current policy against both snapshots. Inconclusive takes
/// precedence; no exception can certify insufficient data.
pub fn evaluate(
    baseline: &Snapshot,
    current: &Snapshot,
    rules: &Rules,
    as_of: Date,
) -> Result<Report> {
    validate_rules(rules)?;
    for snapshot in [baseline, current] {
        if !["item", "file", "module", "package"].contains(&snapshot.context.granularity.as_str()) {
            return Err("Granularidade inválida".into());
        }
    }
    let old = project(baseline)?;
    let new = project(current)?;
    let mut common = Vec::new();
    if !baseline.context.known() || !current.context.known() {
        common.push("Contexto desconhecido/incompleto".into());
    }
    if baseline.context != current.context {
        common.push("Snapshots com contextos incompatíveis".into());
    }
    for (label, s) in [("baseline", baseline), ("current", current)] {
        if s.quality.execution != Completeness::Complete
            || s.quality.coverage != Completeness::Complete
        {
            common.push(format!("{label}: execução/cobertura não completa"));
        }
    }
    let mut rule_results = Vec::new();
    let mut status = Status::Approved;
    let mut ordered: Vec<_> = rules.rules.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    let mut exception_results: BTreeMap<_, _> = rules
        .exceptions
        .iter()
        .map(|e| {
            (
                e.id.clone(),
                ExceptionResult {
                    id: e.id.clone(),
                    rule_id: e.rule_id.clone(),
                    status: if e.until.is_some_and(|until| as_of > until) {
                        "expired"
                    } else {
                        "unused"
                    }
                    .into(),
                    justification: e.justification.clone(),
                    findings: vec![],
                },
            )
        })
        .collect();
    for rule in ordered {
        let mut diagnostics = common.clone();
        for (label, s) in [("baseline", baseline), ("current", current)] {
            for cap in rule.policy.required() {
                if !s.quality.capabilities.contains(&cap) {
                    diagnostics.push(format!("{label}: capacidade insuficiente: {cap:?}"));
                }
            }
        }
        let (mut found, mut selection) = if diagnostics.is_empty() {
            findings(rule, &old, &new)?
        } else {
            (vec![], vec![])
        };
        diagnostics.append(&mut selection);
        diagnostics.sort();
        diagnostics.dedup();
        let outcome = if !diagnostics.is_empty() {
            status = Status::Inconclusive;
            "inconclusive"
        } else {
            for f in &mut found {
                for e in &rules.exceptions {
                    if e.until.is_some_and(|until| as_of > until) || !exception_matches(e, f) {
                        continue;
                    }
                    f.exceptions.push(e.id.clone());
                    let record = exception_results.get_mut(&e.id).unwrap();
                    record.status = "applied".into();
                    record.findings.push(f.id.clone());
                }
                f.exceptions.sort();
                let charge = if matches!(rule.policy, Policy::NoNewCycles { .. }) {
                    f.evolution == Evolution::New
                } else {
                    rules.enforcement == Enforcement::All || f.evolution == Evolution::New
                };
                f.enforced = charge && f.exceptions.is_empty();
            }
            if found.iter().any(|f| f.enforced) {
                if status != Status::Inconclusive {
                    status = Status::Violation;
                }
                "violation"
            } else if found.iter().any(|f| !f.exceptions.is_empty()) {
                "excepted"
            } else if !found.is_empty() {
                "preexisting"
            } else {
                "approved"
            }
        };
        rule_results.push(RuleResult {
            id: rule.id.clone(),
            status: outcome.into(),
            diagnostics,
            findings: found,
        });
    }
    let mut diagnostics: Vec<_> = baseline
        .quality
        .diagnostics
        .iter()
        .map(|s| format!("baseline: {s}"))
        .chain(
            current
                .quality
                .diagnostics
                .iter()
                .map(|s| format!("current: {s}")),
        )
        .collect();
    diagnostics.sort();
    diagnostics.dedup();
    for e in exception_results.values_mut() {
        e.findings.sort();
    }
    Ok(Report {
        status,
        enforcement: rules.enforcement,
        rules: rule_results,
        exceptions: exception_results.into_values().collect(),
        baseline_origins: old.origins,
        current_origins: new.origins,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn date() -> Date {
        Date::parse("2026-09-26").unwrap()
    }
    fn selector(ids: &[&str]) -> Selector {
        Selector {
            ids: ids.iter().map(|s| s.to_string()).collect(),
            prefixes: vec![],
        }
    }
    fn snapshot(names: &[&str], edges: &[(usize, usize)]) -> Snapshot {
        let nodes:Vec<_>=names.iter().enumerate().map(|(id,n)|serde_json::json!({"id":id,"path":n,"name":n,"kind":"mod","visibility":"pub"})).collect();
        let edges:Vec<_>=edges.iter().map(|(a,b)|serde_json::json!({"id_from":a,"id_to":b,"from":names[*a],"to":names[*b],"relation":"uses","uses_kind":"import"})).collect();
        let graph = lente_infra::desserializar_grafo(
            &serde_json::json!({"crate":"fixture","nodes":nodes,"edges":edges}).to_string(),
        )
        .unwrap();
        Snapshot {
            graph,
            context: Context {
                project: "fixture".into(),
                provider: "fixture".into(),
                provider_version: "1".into(),
                granularity: "file".into(),
                nature: "static-import".into(),
                configuration: "default".into(),
                scope: "all-local".into(),
            },
            quality: Quality {
                execution: Completeness::Complete,
                coverage: Completeness::Complete,
                capabilities: BTreeSet::from([
                    Capability::DirectDependencies,
                    Capability::Reachability,
                    Capability::CyclicComponents,
                ]),
                diagnostics: vec![],
            },
            identities: names
                .iter()
                .enumerate()
                .map(|(i, n)| (i, (*n).into()))
                .collect(),
        }
    }
    fn direct() -> Rules {
        Rules {
            enforcement: Enforcement::All,
            rules: vec![Rule {
                id: "stable-rule".into(),
                policy: Policy::ForbidDirect {
                    from: selector(&["a"]),
                    to: selector(&["c"]),
                },
            }],
            exceptions: vec![],
        }
    }
    fn reach() -> Rules {
        let mut r = direct();
        r.rules[0].policy = Policy::ForbidReachability {
            from: selector(&["a"]),
            to: selector(&["c"]),
        };
        r
    }
    fn cycles() -> Rules {
        Rules {
            enforcement: Enforcement::All,
            rules: vec![Rule {
                id: "cycles".into(),
                policy: Policy::NoNewCycles {
                    scope: selector(&["a", "b", "c", "d"]),
                },
            }],
            exceptions: vec![],
        }
    }
    fn names() -> [&'static str; 4] {
        ["a", "b", "c", "d"]
    }
    fn findings(r: &Report) -> &[Finding] {
        &r.rules[0].findings
    }
    fn exemption(until: Option<&str>) -> Exception {
        Exception {
            id: "temporary".into(),
            rule_id: "stable-rule".into(),
            scope: ExceptionScope::Pairs {
                from: selector(&["a"]),
                to: selector(&["c"]),
            },
            justification: "Migração registrada em ADR local".into(),
            until: until.map(|s| Date::parse(s).unwrap()),
        }
    }
    #[test]
    fn permitted_and_forbidden_direct_dependencies() {
        let base = snapshot(&names(), &[]);
        let allowed = snapshot(&names(), &[(0, 1)]);
        assert_eq!(
            evaluate(&base, &allowed, &direct(), date()).unwrap().status,
            Status::Approved
        );
        let blocked = snapshot(&names(), &[(0, 2)]);
        let r = evaluate(&base, &blocked, &direct(), date()).unwrap();
        assert_eq!(r.status, Status::Violation);
        assert_eq!(findings(&r)[0].witness, ["a", "c"]);
    }
    #[test]
    fn indirect_path_has_witness_without_inventing_direct_edge() {
        let base = snapshot(&names(), &[]);
        let current = snapshot(&names(), &[(0, 1), (1, 2)]);
        assert_eq!(
            evaluate(&base, &current, &direct(), date()).unwrap().status,
            Status::Approved
        );
        let r = evaluate(&base, &current, &reach(), date()).unwrap();
        assert_eq!(findings(&r)[0].witness, ["a", "b", "c"]);
        assert_eq!(findings(&r)[0].evolution, Evolution::New);
    }
    #[test]
    fn old_new_and_enforcement_are_separate() {
        let before = snapshot(&names(), &[(0, 2)]);
        let mut rules = direct();
        rules.enforcement = Enforcement::NewOnly;
        let r = evaluate(&before, &before, &rules, date()).unwrap();
        assert_eq!(r.status, Status::Approved);
        assert_eq!(r.rules[0].status, "preexisting");
        assert_eq!(findings(&r)[0].evolution, Evolution::Preexisting);
        rules.enforcement = Enforcement::All;
        assert_eq!(
            evaluate(&before, &before, &rules, date()).unwrap().status,
            Status::Violation
        );
        assert_eq!(
            evaluate(&snapshot(&names(), &[]), &before, &rules, date())
                .unwrap()
                .rules[0]
                .findings[0]
                .evolution,
            Evolution::New
        );
    }
    #[test]
    fn changed_transitive_witness_keeps_violation_identity() {
        let a = snapshot(&names(), &[(0, 1), (1, 2)]);
        let b = snapshot(&names(), &[(0, 3), (3, 2)]);
        let old = evaluate(&a, &a, &reach(), date()).unwrap();
        let new = evaluate(&a, &b, &reach(), date()).unwrap();
        assert_eq!(findings(&old)[0].id, findings(&new)[0].id);
        assert_eq!(findings(&new)[0].evolution, Evolution::Preexisting);
        assert_ne!(findings(&old)[0].witness, findings(&new)[0].witness);
    }
    #[test]
    fn same_number_of_cycles_with_different_members_is_new() {
        let old = snapshot(&names(), &[(0, 1), (1, 0)]);
        let new = snapshot(&names(), &[(2, 3), (3, 2)]);
        let r = evaluate(&old, &new, &cycles(), date()).unwrap();
        assert_eq!(findings(&r).len(), 1);
        assert_eq!(findings(&r)[0].members, ["c", "d"]);
        assert_eq!(findings(&r)[0].evolution, Evolution::New);
    }
    #[test]
    fn scc_expansion_merge_and_new_internal_edge_are_regressions() {
        let old = snapshot(&names(), &[(0, 1), (1, 0), (2, 3), (3, 2)]);
        let merged = snapshot(&names(), &[(0, 1), (1, 0), (2, 3), (3, 2), (1, 2), (3, 0)]);
        assert_eq!(
            evaluate(&old, &merged, &cycles(), date()).unwrap().status,
            Status::Violation
        );
        let cycle = snapshot(&names(), &[(0, 1), (1, 2), (2, 0)]);
        let shortcut = snapshot(&names(), &[(0, 1), (1, 2), (2, 0), (0, 2)]);
        assert_eq!(
            evaluate(&cycle, &shortcut, &cycles(), date())
                .unwrap()
                .status,
            Status::Violation
        );
        assert_eq!(
            evaluate(&merged, &old, &cycles(), date()).unwrap().status,
            Status::Approved
        );
        assert_eq!(
            evaluate(&shortcut, &cycle, &cycles(), date())
                .unwrap()
                .status,
            Status::Approved
        );
    }
    #[test]
    fn cycle_witness_includes_a_new_internal_edge() {
        let before = snapshot(&names(), &[(0, 1), (1, 2), (2, 0)]);
        let now = snapshot(&names(), &[(0, 1), (1, 2), (2, 0), (0, 2)]);
        let r = evaluate(&before, &now, &cycles(), date()).unwrap();
        assert_eq!(
            findings(&r)[0].added_internal_edges,
            [("a".into(), "c".into())]
        );
        assert_eq!(findings(&r)[0].witness, ["a", "c", "a"]);
    }
    #[test]
    fn scc_shrink_without_new_edges_is_not_a_regression() {
        let old = snapshot(&names(), &[(0, 1), (1, 0), (1, 2), (2, 1)]);
        let shrunk = snapshot(&names(), &[(0, 1), (1, 0)]);
        let r = evaluate(&old, &shrunk, &cycles(), date()).unwrap();
        assert_eq!(r.status, Status::Approved);
        assert_eq!(findings(&r)[0].evolution, Evolution::Preexisting);
    }
    #[test]
    fn self_loops_and_positive_reflexive_reachability() {
        let base = snapshot(&names(), &[]);
        let now = snapshot(&names(), &[(0, 0)]);
        assert_eq!(
            evaluate(&base, &now, &cycles(), date()).unwrap().status,
            Status::Violation
        );
        let mut rules = reach();
        rules.rules[0].policy = Policy::ForbidReachability {
            from: selector(&["a"]),
            to: selector(&["a"]),
        };
        assert_eq!(
            evaluate(&base, &base, &rules, date()).unwrap().status,
            Status::Approved
        );
        assert_eq!(
            evaluate(&base, &now, &rules, date()).unwrap().rules[0].findings[0].witness,
            ["a", "a"]
        );
    }
    #[test]
    fn explicit_rename_preserves_correspondence_not_paths_or_local_ids() {
        let base = snapshot(&names(), &[(0, 2)]);
        let mut changed = snapshot(&["new/a", "renamed.b", "别名::c", "d"], &[(0, 2)]);
        changed.identities = base.identities.clone();
        for n in &mut changed.graph.nodes {
            n.id += 10;
        }
        for e in &mut changed.graph.edges {
            e.id_from += 10;
            e.id_to += 10;
        }
        for (i, _) in &mut changed.identities {
            *i += 10;
        }
        let r = evaluate(&base, &changed, &direct(), date()).unwrap();
        assert_eq!(findings(&r)[0].evolution, Evolution::Preexisting);
        assert_eq!(r.current_origins["c"].path, "别名::c");
        let mut different = base.clone();
        different.identities[0].1 = "other".into();
        assert_eq!(
            evaluate(&different, &base, &direct(), date())
                .unwrap()
                .rules[0]
                .findings[0]
                .evolution,
            Evolution::New
        );
    }
    #[test]
    fn exceptions_valid_expired_blank_and_not_approvals() {
        let base = snapshot(&names(), &[]);
        let now = snapshot(&names(), &[(0, 2)]);
        let mut r = direct();
        r.exceptions.push(exemption(Some("2026-09-26")));
        let result = evaluate(&base, &now, &r, date()).unwrap();
        assert_eq!(result.status, Status::Approved);
        assert_eq!(result.rules[0].status, "excepted");
        assert_eq!(result.exceptions[0].status, "applied");
        assert_eq!(
            evaluate(&base, &now, &r, Date::parse("2026-09-27").unwrap())
                .unwrap()
                .status,
            Status::Violation
        );
        r.exceptions[0].justification = "  ".into();
        assert!(evaluate(&base, &now, &r, date()).is_err());
    }
    #[test]
    fn cycle_exception_requires_all_members_and_expired_stays_visible() {
        let base = snapshot(&names(), &[]);
        let now = snapshot(&names(), &[(0, 1), (1, 0)]);
        let mut r = cycles();
        r.exceptions.push(Exception {
            id: "cycle-waiver".into(),
            rule_id: "cycles".into(),
            scope: ExceptionScope::Component {
                members: selector(&["a"]),
            },
            justification: "migration".into(),
            until: None,
        });
        let result = evaluate(&base, &now, &r, date()).unwrap();
        assert_eq!(result.status, Status::Violation);
        assert_eq!(result.exceptions[0].status, "unused");
        r.exceptions[0].scope = ExceptionScope::Component {
            members: selector(&["a", "b"]),
        };
        assert_eq!(
            evaluate(&base, &now, &r, date()).unwrap().status,
            Status::Approved
        );
    }
    #[test]
    fn selector_unmatched_or_partially_unmatched_is_inconclusive() {
        let g = snapshot(&names(), &[]);
        let mut r = direct();
        r.rules[0].policy = Policy::ForbidDirect {
            from: selector(&["a", "typo"]),
            to: selector(&["c"]),
        };
        assert_eq!(
            evaluate(&g, &g, &r, date()).unwrap().status,
            Status::Inconclusive
        );
        r.rules[0].policy = Policy::NoNewCycles {
            scope: Selector {
                ids: vec![],
                prefixes: vec!["missing/".into()],
            },
        };
        assert_eq!(
            evaluate(&g, &g, &r, date()).unwrap().status,
            Status::Inconclusive
        );
    }
    #[test]
    fn partial_graph_and_insufficient_capability_never_pass() {
        let good = snapshot(&names(), &[]);
        let mut bad = good.clone();
        bad.quality.coverage = Completeness::Partial;
        assert_eq!(
            evaluate(&good, &bad, &direct(), date()).unwrap().status,
            Status::Inconclusive
        );
        assert_eq!(
            evaluate(&bad, &good, &direct(), date()).unwrap().status,
            Status::Inconclusive
        );
        bad = good.clone();
        bad.quality.capabilities.remove(&Capability::Reachability);
        assert_eq!(
            evaluate(&good, &bad, &reach(), date()).unwrap().status,
            Status::Inconclusive
        );
        assert_eq!(
            evaluate(&good, &bad, &direct(), date()).unwrap().status,
            Status::Approved
        );
    }
    #[test]
    fn inconclusive_preserves_other_conclusive_violations() {
        let base = snapshot(&names(), &[]);
        let mut now = snapshot(&names(), &[(0, 2)]);
        now.quality
            .capabilities
            .remove(&Capability::CyclicComponents);
        let mut rules = direct();
        rules.rules.extend(cycles().rules);
        let result = evaluate(&base, &now, &rules, date()).unwrap();
        assert_eq!(result.status, Status::Inconclusive);
        assert!(result.rules.iter().any(|r| r.status == "violation"));
        assert!(result.rules.iter().any(|r| r.status == "inconclusive"));
    }
    #[test]
    fn incompatible_context_and_identity_ambiguity_do_not_compare() {
        let good = snapshot(&names(), &[]);
        let mut bad = good.clone();
        bad.context.granularity = "package".into();
        assert_eq!(
            evaluate(&good, &bad, &direct(), date()).unwrap().status,
            Status::Inconclusive
        );
        bad = good.clone();
        bad.identities[1].1 = "a".into();
        assert!(evaluate(&good, &bad, &direct(), date()).is_err());
        bad = good.clone();
        bad.identities.pop();
        assert!(evaluate(&good, &bad, &direct(), date()).is_err());
    }
    #[test]
    fn determinism_permutations_parallel_edges_and_rules() {
        let base = snapshot(&names(), &[]);
        let mut now = snapshot(&names(), &[(0, 1), (1, 2), (2, 1), (0, 3), (3, 2)]);
        let mut rules = reach();
        rules.rules.extend(cycles().rules);
        rules.exceptions.push(exemption(None));
        let expected = evaluate(&base, &now, &rules, date()).unwrap();
        now.graph.edges.extend(now.graph.edges.clone());
        now.graph.edges.reverse();
        now.graph.nodes.reverse();
        now.identities.reverse();
        rules.rules.reverse();
        assert_eq!(expected, evaluate(&base, &now, &rules, date()).unwrap());
        assert_eq!(
            expected
                .rules
                .iter()
                .find(|r| r.id == "stable-rule")
                .unwrap()
                .findings[0]
                .witness,
            ["a", "b", "c"]
        );
    }
    #[test]
    fn owns_does_not_propagate_and_repeated_raw_paths_are_valid() {
        let base = snapshot(&names(), &[]);
        let mut now = snapshot(&names(), &[(0, 2)]);
        now.graph.edges[0].relation = Relation::Owns;
        assert_eq!(
            evaluate(&base, &now, &direct(), date()).unwrap().status,
            Status::Approved
        );
        let mut now = snapshot(&names(), &[(0, 2)]);
        now.graph.nodes[2].path = "a".into();
        now.graph.edges[0].to = "a".into();
        assert_eq!(
            evaluate(&base, &now, &direct(), date()).unwrap().status,
            Status::Violation
        );
    }
    #[test]
    fn dates_and_length_prefixed_identifiers_are_unambiguous() {
        for invalid in [
            "2025-02-29",
            "2026-13-01",
            "2026-04-31",
            "0000-01-01",
            "26-09-26",
        ] {
            assert!(Date::parse(invalid).is_err());
        }
        assert!(Date::parse("2024-02-29").is_ok());
        assert_ne!(
            violation_id("r", "direct", ["a:b".into(), "c".into()]),
            violation_id("r", "direct", ["a".into(), "b:c".into()])
        );
    }
}
