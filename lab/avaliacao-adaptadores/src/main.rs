//! Crystalline Lineage
//! @prompt lab/avaliacao-adaptadores/CONTRATO.md
//! @layer lab
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::{self, File},
    io,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Invocation {
    argv: Vec<String>,
    cwd: String,
    timeout_ms: u64,
    #[serde(default)]
    env: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Requirement {
    name: String,
    #[serde(default)]
    files: Vec<String>,
    command: Option<Invocation>,
    version_contains: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Preparation {
    #[serde(default)]
    files: BTreeMap<String, Value>,
    #[serde(default)]
    commands: Vec<Invocation>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    exit_codes: Vec<i32>,
    nodes: usize,
    edges: usize,
    #[serde(default)]
    names: Vec<String>,
    #[serde(default)]
    pairs: Vec<[String; 2]>,
    #[serde(default)]
    metadata: BTreeMap<String, Value>,
    #[serde(default)]
    diagnostics_min: usize,
    #[serde(default)]
    diagnostic_contains: Vec<String>,
    #[serde(default)]
    edge_fields: BTreeMap<String, Value>,
    #[serde(default)]
    depths: Vec<(String, String, usize)>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    id: String,
    adapter: String,
    granularity: String,
    relations: String,
    universe: String,
    #[serde(default)]
    unsupported: Option<String>,
    #[serde(default)]
    requirements: Vec<Requirement>,
    #[serde(default)]
    preparation: Preparation,
    command: Option<Invocation>,
    graph: Option<String>,
    metadata: Option<String>,
    expected: Option<Expected>,
    #[serde(default = "two")]
    repetitions: usize,
    #[serde(default)]
    deterministic: bool,
}
fn two() -> usize {
    2
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Comparison {
    left: String,
    right: String,
    expect: String,
    reason: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Suite {
    version: u32,
    scenarios: Vec<Scenario>,
    #[serde(default)]
    comparisons: Vec<Comparison>,
}

fn check_invocation(c: &Invocation) -> Result<()> {
    if c.argv.is_empty() || c.argv[0].is_empty() || c.timeout_ms == 0 || c.cwd.is_empty() {
        return Err("argv, cwd e timeout positivo são obrigatórios".into());
    }
    let exe = Path::new(&c.argv[0])
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if [
        "sh",
        "bash",
        "dash",
        "zsh",
        "fish",
        "cmd",
        "cmd.exe",
        "powershell",
        "pwsh",
    ]
    .contains(&exe.as_str())
    {
        return Err("Shell não permitido como comando de cenário".into());
    }
    Ok(())
}
fn safe_relative(s: &str) -> bool {
    !s.is_empty()
        && Path::new(s)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
fn check_suite(s: &Suite) -> Result<()> {
    if s.scenarios.is_empty() {
        return Err("Suíte vazia".into());
    }
    if s.version != 1 {
        return Err("Versão de cenários não suportada".into());
    }
    let mut ids = BTreeSet::new();
    for c in &s.scenarios {
        if !c
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || c.id.is_empty()
            || !ids.insert(c.id.clone())
        {
            return Err("ID inválido/repetido".into());
        }
        if c.repetitions == 0
            || [&c.granularity, &c.relations, &c.universe]
                .iter()
                .any(|s| s.is_empty())
        {
            return Err(format!("{}: configuração incompleta", c.id));
        }
        if let Some(reason) = &c.unsupported {
            if reason.is_empty() {
                return Err("Caso não suportado sem razão".into());
            }
        } else if c.command.is_none()
            || c.graph.is_none()
            || c.metadata.is_none()
            || c.expected.is_none()
        {
            return Err(format!(
                "{}: faltam comando, artefatos ou expectativas",
                c.id
            ));
        }
        for k in c.preparation.files.keys() {
            if !safe_relative(k) {
                return Err(format!("Path de preparação não relativo: {k}"));
            }
        }
        for cmd in c
            .command
            .iter()
            .chain(c.preparation.commands.iter())
            .chain(c.requirements.iter().filter_map(|r| r.command.as_ref()))
        {
            check_invocation(cmd)?;
        }
        if c.expected.as_ref().is_some_and(|e| e.exit_codes.is_empty()) {
            return Err("Expectativa sem exit codes".into());
        }
    }
    for cmp in &s.comparisons {
        let a = s
            .scenarios
            .iter()
            .find(|s| s.id == cmp.left)
            .ok_or("Comparação sem cenário esquerdo")?;
        let b = s
            .scenarios
            .iter()
            .find(|s| s.id == cmp.right)
            .ok_or("Comparação sem cenário direito")?;
        if (
            a.granularity.as_str(),
            a.relations.as_str(),
            a.universe.as_str(),
        ) != (
            b.granularity.as_str(),
            b.relations.as_str(),
            b.universe.as_str(),
        ) {
            return Err(format!("Comparação incompatível: {} / {}", a.id, b.id));
        }
        if !["same", "different"].contains(&cmp.expect.as_str()) || cmp.reason.is_empty() {
            return Err("Comparação inválida".into());
        }
    }
    Ok(())
}

#[derive(Clone)]
struct Context {
    repo: PathBuf,
    lab: PathBuf,
    run: PathBuf,
    case: PathBuf,
    out: PathBuf,
}
impl Context {
    fn text(&self, s: &str) -> String {
        let tokens = [
            ("{repo}", &self.repo),
            ("{lab}", &self.lab),
            ("{run}", &self.run),
            ("{case}", &self.case),
            ("{out}", &self.out),
        ];
        // One pass: substitutions are not themselves expanded/interpreted.
        let mut rest = s;
        let mut result = String::new();
        while !rest.is_empty() {
            if let Some((token, path)) = tokens.iter().find(|(t, _)| rest.starts_with(t)) {
                result.push_str(&path.to_string_lossy());
                rest = &rest[token.len()..];
            } else {
                let ch = rest.chars().next().unwrap();
                result.push(ch);
                rest = &rest[ch.len_utf8()..];
            }
        }
        result
    }
    fn value(&self, v: &Value) -> Value {
        match v {
            Value::String(s) => json!(self.text(s)),
            Value::Array(a) => Value::Array(a.iter().map(|v| self.value(v)).collect()),
            Value::Object(o) => {
                Value::Object(o.iter().map(|(k, v)| (k.clone(), self.value(v))).collect())
            }
            _ => v.clone(),
        }
    }
    fn command(&self, c: &Invocation) -> Invocation {
        Invocation {
            argv: c.argv.iter().map(|s| self.text(s)).collect(),
            cwd: self.text(&c.cwd),
            timeout_ms: c.timeout_ms,
            env: c
                .env
                .iter()
                .map(|(k, v)| (k.clone(), self.text(v)))
                .collect(),
        }
    }
}
static SEQ: AtomicU64 = AtomicU64::new(0);
fn unique_dir(parent: &Path) -> Result<PathBuf> {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    for _ in 0..100 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let p = parent.join(format!(
            "run-{now}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&p) {
            Ok(()) => return Ok(p),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("Não foi possível reservar diretório exclusivo".into())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write_json(path: &Path, v: &impl Serialize) -> Result<()> {
    let b = serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?;
    fs::write(path, b).map_err(|e| e.to_string())
}
fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}
#[derive(Debug, Serialize)]
struct ProcessResult {
    command: Invocation,
    code: Option<i32>,
    duration_ms: f64,
    timed_out: bool,
    error: Option<String>,
    missing: bool,
    stdout: PathBuf,
    stderr: PathBuf,
}
#[cfg(unix)]
fn kill_group(pid: u32) {
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    unsafe {
        kill(-(pid as i32), 9);
    }
}
fn execute(c: &Invocation, dir: &Path, label: &str) -> Result<ProcessResult> {
    check_invocation(c)?;
    let stdout = dir.join(format!("{label}.stdout"));
    let stderr = dir.join(format!("{label}.stderr"));
    let out = File::create(&stdout).map_err(|e| e.to_string())?;
    let err = File::create(&stderr).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(&c.argv[0]);
    cmd.args(&c.argv[1..])
        .current_dir(&c.cwd)
        .envs(&c.env)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let start = Instant::now();
    let mut result = ProcessResult {
        command: c.clone(),
        code: None,
        duration_ms: 0.0,
        timed_out: false,
        error: None,
        missing: false,
        stdout,
        stderr,
    };
    // Do not start children on platforms without implemented tree cleanup.
    if !cfg!(unix) {
        result.error = Some("Execução não suportada nesta plataforma".into());
        return Ok(result);
    }
    if !Path::new(&c.cwd).is_dir() {
        result.error = Some(format!("cwd inexistente ou não diretório: {}", c.cwd));
        result.duration_ms = ms(start);
        return Ok(result);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            result.missing = e.kind() == io::ErrorKind::NotFound;
            result.error = Some(e.to_string());
            result.duration_ms = ms(start);
            return Ok(result);
        }
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                result.code = status.code();
                break;
            }
            Ok(None) => {}
            Err(e) => {
                result.error = Some(e.to_string());
                break;
            }
        }
        if start.elapsed() >= Duration::from_millis(c.timeout_ms) {
            result.timed_out = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // Also remove descendants left by a normally exiting command. Commands in
    // scenarios must not intentionally start persistent services.
    #[cfg(unix)]
    kill_group(child.id());
    if result.timed_out || result.error.is_some() {
        let _ = child.kill();
        let status = child.wait().map_err(|e| e.to_string())?;
        result.code = status.code();
    }
    result.duration_ms = ms(start);
    Ok(result)
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let i = v.len() / 2;
    Some(if v.len().is_multiple_of(2) {
        (v[i - 1] + v[i]) / 2.0
    } else {
        v[i]
    })
}
fn stats(v: &[f64]) -> Value {
    match median(v) {
        None => json!({"n":0}),
        Some(m) => {
            let deviations: Vec<_> = v.iter().map(|v| (v - m).abs()).collect();
            json!({"n":v.len(),"median_ms":m,"mad_ms":median(&deviations),"min_ms":v.iter().copied().reduce(f64::min),"max_ms":v.iter().copied().reduce(f64::max)})
        }
    }
}

fn validate_artifacts(
    graph: &str,
    meta: &Value,
    e: &Expected,
) -> Result<(lente_core::entities::grafo::Grafo, Value)> {
    let g =
        lente_infra::desserializar_grafo(graph).map_err(|e| format!("Contrato do grafo: {e:?}"))?;
    if (g.nodes.len(), g.edges.len()) != (e.nodes, e.edges) {
        return Err(format!(
            "Contagens: esperado {}/{}, obtido {}/{}",
            e.nodes,
            e.edges,
            g.nodes.len(),
            g.edges.len()
        ));
    }
    if let Some(digest) = meta.get("graph_sha256").and_then(Value::as_str)
        && digest != hash(graph.as_bytes())
    {
        return Err("Hash dos metadados não corresponde ao grafo".into());
    }
    for (pointer, value) in &e.metadata {
        if meta.pointer(pointer) != Some(value) {
            return Err(format!(
                "Metadado {pointer}: esperado {value}, obtido {:?}",
                meta.pointer(pointer)
            ));
        }
    }
    if meta["diagnostics"].as_array().map_or(0, Vec::len) < e.diagnostics_min {
        return Err("Diagnósticos esperados ausentes".into());
    }
    let diagnostics = meta["diagnostics"].to_string();
    for text in &e.diagnostic_contains {
        if !diagnostics.contains(text) {
            return Err(format!("Diagnóstico obrigatório ausente: {text}"));
        }
    }
    let raw: Value = serde_json::from_str(graph).map_err(|e| e.to_string())?;
    for edge in raw["edges"].as_array().ok_or("edges inválidas")? {
        for (key, value) in &e.edge_fields {
            if edge.get(key) != Some(value) {
                return Err(format!("Campo de aresta {key}: esperado {value}"));
            }
        }
    }
    let mut names = Vec::new();
    let mut by_id = BTreeMap::new();
    for n in raw["nodes"].as_array().ok_or("nodes inválidos")? {
        let name = n["name"].as_str().ok_or("name inválido")?.to_owned();
        names.push(name.clone());
        by_id.insert(n["id"].as_u64().ok_or("id inválido")?, name);
    }
    names.sort();
    let mut expected = e.names.clone();
    expected.sort();
    if names != expected {
        return Err(format!("Nós: esperado {expected:?}, obtido {names:?}"));
    }
    let mut pairs: Vec<[String; 2]> = raw["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            [
                by_id[&edge["id_from"].as_u64().unwrap()].clone(),
                by_id[&edge["id_to"].as_u64().unwrap()].clone(),
            ]
        })
        .collect();
    pairs.sort();
    let mut expected = e.pairs.clone();
    expected.sort();
    if pairs != expected {
        return Err(format!("Arestas: esperado {expected:?}, obtido {pairs:?}"));
    }
    // Names are the explicit correspondence for this suite's fixtures only.
    if names.iter().collect::<BTreeSet<_>>().len() != names.len() {
        return Err("Correspondência por name ambígua".into());
    }
    let signature = json!({"names":names,"pairs":pairs});
    Ok((g, signature))
}
fn analyze(g: &lente_core::entities::grafo::Grafo, e: &Expected) -> Result<Value> {
    use lente_core::domain::raio::calcular_raio;
    let paths: BTreeSet<_> = g.nodes.iter().map(|n| n.path.as_str()).collect();
    if paths.len() != g.nodes.len() {
        return Err("Forma crua válida, mas paths repetidos impedem análise resolvida".into());
    }
    let by_name: BTreeMap<_, _> = g.nodes.iter().map(|n| (n.name.as_str(), &n.path)).collect();
    let mut depths = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut isolated = Vec::new();
    for n in &g.nodes {
        let r = calcular_raio(g, &n.path).map_err(|e| format!("Raio: {e:?}"))?;
        if r.uses_entrada == 0 && r.uses_saida == 0 {
            isolated.push(n.name.clone());
        }
        let mut row = BTreeMap::new();
        for target in &g.nodes {
            if let Some(d) = r.jusante.get(&target.path) {
                row.insert(target.name.clone(), *d);
            }
        }
        depths.insert(n.name.clone(), row);
    }
    for (from, to, distance) in &e.depths {
        if !by_name.contains_key(from.as_str())
            || depths.get(from).and_then(|row| row.get(to)) != Some(distance)
        {
            return Err(format!("Profundidade {from} → {to} != {distance}"));
        }
    }
    isolated.sort();
    Ok(
        json!({"downstream_depths":depths,"isolated":isolated,"method":"lente_core::calcular_raio, all nodes"}),
    )
}

fn requirement(r: &Requirement, ctx: &Context, dir: &Path, index: usize) -> Result<Value> {
    let mut files = Vec::new();
    for path in &r.files {
        let p = ctx.text(path);
        match fs::read(&p) {
            Ok(b) => files.push(json!({"path":p,"sha256":hash(&b)})),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(
                    json!({"name":r.name,"status":"ferramenta_ausente","reason":format!("Arquivo requerido ausente: {p}"),"files":files}),
                );
            }
            Err(e) => {
                return Ok(
                    json!({"name":r.name,"status":"reprovado","reason":format!("Leitura {p}: {e}"),"files":files}),
                );
            }
        }
    }
    let mut report = json!({"name":r.name,"status":"aprovado","files":files});
    if let Some(command) = &r.command {
        let p = execute(&ctx.command(command), dir, &format!("requirement-{index}"))?;
        let text = format!(
            "{}{}",
            fs::read_to_string(&p.stdout).map_err(|e| e.to_string())?,
            fs::read_to_string(&p.stderr).map_err(|e| e.to_string())?
        );
        report["status"] = json!(if p.missing {
            "ferramenta_ausente"
        } else if p.timed_out || p.error.is_some() || p.code != Some(0) {
            "reprovado"
        } else if r
            .version_contains
            .as_ref()
            .is_some_and(|v| !text.contains(v))
        {
            "nao_suportado"
        } else {
            "aprovado"
        });
        report["version_output"] = json!(text);
        report["process"] = json!(p);
    }
    Ok(report)
}
fn prepare(s: &Scenario, ctx: &Context) -> Result<Value> {
    let start = Instant::now();
    let mut commands = Vec::new();
    for (name, value) in &s.preparation.files {
        let p = ctx.case.join(name);
        fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
        let v = ctx.value(value);
        if let Value::String(text) = v {
            fs::write(p, text).map_err(|e| e.to_string())?;
        } else {
            write_json(&p, &v)?;
        }
    }
    for (i, c) in s.preparation.commands.iter().enumerate() {
        let p = execute(&ctx.command(c), &ctx.case, &format!("prepare-{i}"))?;
        let status = if p.missing {
            "ferramenta_ausente"
        } else if p.timed_out || p.error.is_some() || p.code != Some(0) {
            "reprovado"
        } else {
            "aprovado"
        };
        commands.push(json!(p));
        if status != "aprovado" {
            return Ok(json!({"status":status,"commands":commands,"duration_ms":ms(start)}));
        }
    }
    Ok(json!({"status":"aprovado","commands":commands,"duration_ms":ms(start)}))
}
fn sample(s: &Scenario, ctx: &Context, index: usize) -> Result<Value> {
    let c = ctx.command(s.command.as_ref().unwrap());
    let p = execute(&c, ctx.out.parent().unwrap(), "adapter")?;
    let mut report = json!({"index":index,"phase":if index==0 {"first-measured"} else {"subsequent"},"status":"reprovado","directory":ctx.out,
        "adapter_command_ms":p.duration_ms,"process":p,"validation_ms":null,"analysis_ms":null});
    if p.missing {
        report["status"] = json!("ferramenta_ausente");
        return Ok(report);
    }
    let expected = s.expected.as_ref().unwrap();
    if p.timed_out || p.error.is_some() || !p.code.is_some_and(|c| expected.exit_codes.contains(&c))
    {
        report["reason"] = json!("Timeout, erro operacional ou exit code fora do declarado");
        return Ok(report);
    }
    let start = Instant::now();
    let validated = (|| {
        let graph = fs::read_to_string(ctx.text(s.graph.as_ref().unwrap()))
            .map_err(|e| format!("Leitura do grafo: {e}"))?;
        let metadata: Value = serde_json::from_slice(
            &fs::read(ctx.text(s.metadata.as_ref().unwrap()))
                .map_err(|e| format!("Leitura dos metadados: {e}"))?,
        )
        .map_err(|e| format!("JSON dos metadados: {e}"))?;
        report["metadata"] = metadata.clone();
        report["graph_sha256"] = json!(hash(graph.as_bytes()));
        if let Ok(raw) = serde_json::from_str::<Value>(&graph) {
            report["nodes"] = json!(raw["nodes"].as_array().map(Vec::len));
            report["edges"] = json!(raw["edges"].as_array().map(Vec::len));
        }
        let expanded: Expected =
            serde_json::from_value(ctx.value(&serde_json::to_value(expected).unwrap()))
                .map_err(|e| e.to_string())?;
        let (g, sig) = validate_artifacts(&graph, &metadata, &expanded)?;
        Ok::<_, String>((g, sig, expanded))
    })();
    report["validation_ms"] = json!(ms(start));
    match validated {
        Err(e) => {
            report["reason"] = json!(e);
            return Ok(report);
        }
        Ok((g, sig, expanded)) => {
            report["signature"] = sig;
            let start = Instant::now();
            let result = analyze(&g, &expanded);
            report["analysis_ms"] = json!(ms(start));
            match result {
                Ok(value) => {
                    report["analysis"] = value;
                    report["status"] = json!("aprovado");
                }
                Err(e) => report["reason"] = json!(e),
            }
        }
    }
    Ok(report)
}
fn timing_summary(samples: &[Value], key: &str) -> Value {
    let v: Vec<_> = samples
        .iter()
        .filter(|s| s["status"] == "aprovado")
        .filter_map(|s| s[key].as_f64())
        .collect();
    let later: Vec<_> = samples
        .iter()
        .filter(|s| s["index"] != 0 && s["status"] == "aprovado")
        .filter_map(|s| s[key].as_f64())
        .collect();
    json!({"approved_samples":stats(&v),"first_measured_ms":samples.first().filter(|s|s["status"]=="aprovado").and_then(|s|s[key].as_f64()),"subsequent":stats(&later)})
}
fn run_scenario(s: &Scenario, base: &Context, repetitions: Option<usize>) -> Result<Value> {
    let case = base.run.join(&s.id);
    fs::create_dir(&case).map_err(|e| e.to_string())?;
    let mut ctx = base.clone();
    ctx.case = case;
    ctx.out = ctx.case.clone();
    let mut report = json!({"id":s.id,"adapter":s.adapter,"granularity":s.granularity,"relations":s.relations,"universe":s.universe,"status":"aprovado","scenario":s,"samples":[],"requirements":[]});
    if !cfg!(unix) || s.unsupported.is_some() {
        report["status"] = json!("nao_suportado");
        report["reason"] = json!(
            s.unsupported
                .as_deref()
                .unwrap_or("Plataforma sem encerramento de grupo de processos implementado")
        );
        return Ok(report);
    }
    let mut reqs = Vec::new();
    let mut req_status = "aprovado".to_owned();
    for (i, r) in s.requirements.iter().enumerate() {
        let result = requirement(r, &ctx, &ctx.case, i)?;
        if result["status"] != "aprovado" && req_status == "aprovado" {
            req_status = result["status"].as_str().unwrap().to_owned();
        }
        reqs.push(result);
    }
    report["requirements"] = json!(reqs);
    if req_status != "aprovado" {
        report["status"] = json!(req_status);
        report["reason"] = json!("Requisito não satisfeito; veja requirements");
        return Ok(report);
    }
    let prepared = prepare(s, &ctx)?;
    report["preparation"] = prepared.clone();
    if prepared["status"] != "aprovado" {
        report["status"] = prepared["status"].clone();
        return Ok(report);
    }
    let mut samples = Vec::new();
    let mut baseline = None;
    for index in 0..repetitions.unwrap_or(s.repetitions) {
        ctx.out = ctx.case.join(format!("sample-{index:04}")).join("output");
        fs::create_dir_all(&ctx.out).map_err(|e| e.to_string())?;
        let mut result = match sample(s, &ctx, index) {
            Ok(v) => v,
            Err(e) => json!({"index":index,"status":"reprovado","reason":e}),
        };
        if s.deterministic && result["status"] == "aprovado" {
            if let Some(hash) = &baseline {
                if hash != &result["graph_sha256"] {
                    result["status"] = json!("reprovado");
                    result["reason"] = json!("Mesma extração produziu bytes diferentes no grafo");
                }
            } else {
                baseline = Some(result["graph_sha256"].clone());
            }
        }
        if result["status"] != "aprovado" {
            report["status"] = result["status"].clone();
        }
        samples.push(result);
    }
    report["timings"] = json!({"adapter_command":timing_summary(&samples,"adapter_command_ms"),"post_validation":timing_summary(&samples,"validation_ms"),"post_analysis":timing_summary(&samples,"analysis_ms")});
    report["samples"] = json!(samples);
    Ok(report)
}
fn compare(c: &Comparison, reports: &[Value]) -> Value {
    let a = reports.iter().find(|s| s["id"] == c.left).unwrap();
    let b = reports.iter().find(|s| s["id"] == c.right).unwrap();
    if a["status"] != "aprovado" || b["status"] != "aprovado" {
        return json!({"comparison":c,"status":"nao_suportado","reason":"Comparação não executada: faltam duas avaliações aprovadas"});
    }
    let sa = &a["samples"][0];
    let sb = &b["samples"][0];
    let same = sa["signature"] == sb["signature"] && sa["analysis"] == sb["analysis"];
    json!({"comparison":c,"status":if same==(c.expect=="same"){"aprovado"}else{"reprovado"},"same_named_topology":same,"mapping":"node.name; unique names verified"})
}
fn render(report: &Value) -> String {
    let f = |v: &Value| {
        v.as_f64()
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "—".into())
    };
    let clean = |s: &str| s.replace('|', "/").replace(['\n', '\r'], " ");
    let scenarios = report["scenarios"].as_array().unwrap();
    let mut text = String::from(
        "# Avaliação de adaptadores\n\nMedições sequenciais de fixtures locais. Não há ranking entre semânticas distintas.\nTempos do comando incluem toda a execução interna do adaptador; validação/análise abaixo são posteriores.\nPrimeira amostra medida não significa cache frio. Dispersão = MAD (mediana dos desvios absolutos).\n\n| Cenário | Status | Nós/arestas | Amostras aprovadas | Comando: mediana ± MAD ms | Primeira ms | Subsequentes: mediana ms |\n|---|---|---|---|---|---|---|\n",
    );
    for s in scenarios {
        let stats = &s["timings"]["adapter_command"];
        let n = s["samples"].as_array().and_then(|v| v.first());
        text.push_str(&format!(
            "| {} | {} | {}/{} | {} | {} ± {} | {} | {} |\n",
            s["id"].as_str().unwrap(),
            s["status"].as_str().unwrap(),
            n.map_or(json!(null), |v| v["nodes"].clone()),
            n.map_or(json!(null), |v| v["edges"].clone()),
            stats["approved_samples"]["n"],
            f(&stats["approved_samples"]["median_ms"]),
            f(&stats["approved_samples"]["mad_ms"]),
            f(&stats["first_measured_ms"]),
            f(&stats["subsequent"]["median_ms"])
        ));
    }
    text.push_str("\n## Tempos de etapas independentes\n\nPreparação ocorre uma vez por cenário; sondagem de requisitos fica fora desses tempos.\n\n| Cenário | Preparação ms | Validação: mediana ± MAD ms | Análise: mediana ± MAD ms |\n|---|---|---|---|\n");
    for s in scenarios {
        text.push_str(&format!(
            "| {} | {} | {} ± {} | {} ± {} |\n",
            s["id"].as_str().unwrap(),
            f(&s["preparation"]["duration_ms"]),
            f(&s["timings"]["post_validation"]["approved_samples"]["median_ms"]),
            f(&s["timings"]["post_validation"]["approved_samples"]["mad_ms"]),
            f(&s["timings"]["post_analysis"]["approved_samples"]["median_ms"]),
            f(&s["timings"]["post_analysis"]["approved_samples"]["mad_ms"])
        ));
    }
    text.push_str("\n## Versões observadas\n\n");
    let mut versions = BTreeSet::new();
    for s in scenarios {
        for r in s["requirements"].as_array().into_iter().flatten() {
            if let Some(v) = r["version_output"].as_str() {
                versions.insert(format!(
                    "{}: {}",
                    r["name"].as_str().unwrap_or("?"),
                    clean(v.trim())
                ));
            }
        }
    }
    for v in versions {
        text.push_str(&format!("- {v}\n"));
    }
    text.push_str("\n## Resultados indisponíveis ou reprovados\n\n");
    for s in scenarios {
        if s["status"] != "aprovado" {
            text.push_str(&format!(
                "- {}: {} — {}\n",
                s["id"], s["status"], s["reason"]
            ));
            for r in s["requirements"].as_array().into_iter().flatten() {
                if r["status"] != "aprovado" {
                    text.push_str(&format!(
                        "  Requisito {}: {}, {}.\n",
                        r["name"], r["status"], r["reason"]
                    ));
                }
            }
            for sample in s["samples"].as_array().into_iter().flatten() {
                if sample["status"] != "aprovado" {
                    text.push_str(&format!(
                        "  Amostra {}: {}\n",
                        sample["index"], sample["reason"]
                    ));
                }
            }
        }
    }
    text.push_str("\n## Comparações declaradas\n\n");
    for c in report["comparisons"].as_array().unwrap() {
        text.push_str(&format!(
            "- {} / {}: {}. {}\n",
            c["comparison"]["left"],
            c["comparison"]["right"],
            c["status"],
            c["comparison"]["reason"]
        ));
    }
    text.push_str("\nTodas as amostras, configurações, hashes, metadados e caminhos dos logs estão em relatorio.json. Aprovação não comprova completude semântica ou equivalência comportamental. Fixtures pequenas não sustentam promessa geral de latência.\n");
    text
}
fn main_run() -> Result<i32> {
    let lab = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let mut suite_path = lab.join("cenarios.json");
    let mut repo = lab.parent().unwrap().parent().unwrap().to_path_buf();
    let mut repetitions = None;
    let mut args = env::args().skip(1);
    while let Some(k) = args.next() {
        if k == "--help" {
            println!(
                "avaliacao-adaptadores [--suite ARQUIVO] [--repo DIR] [--repetitions N]\nResultados exclusivos em lab/avaliacao-adaptadores/runs/. Exit 0: tudo aprovado; 1: reprovado/erro; 2: avaliação indisponível/não suportada."
            );
            return Ok(0);
        }
        let v = args.next().ok_or_else(|| format!("Falta valor: {k}"))?;
        match k.as_str() {
            "--suite" => suite_path = v.into(),
            "--repo" => repo = v.into(),
            "--repetitions" => {
                let n = v.parse::<usize>().map_err(|e| e.to_string())?;
                if n == 0 {
                    return Err("Repetições deve ser positivo".into());
                }
                repetitions = Some(n);
            }
            _ => return Err(format!("Opção inválida: {k}")),
        }
    }
    let bytes = fs::read(&suite_path).map_err(|e| e.to_string())?;
    let suite: Suite = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    check_suite(&suite)?;
    let repo = repo.canonicalize().map_err(|e| e.to_string())?;
    let run = unique_dir(&lab.join("runs"))?;
    let ctx = Context {
        repo,
        lab,
        run: run.clone(),
        case: run.clone(),
        out: run.clone(),
    };
    write_json(&run.join("cenarios.json"), &suite)?;
    let mut reports = Vec::new();
    for s in &suite.scenarios {
        eprintln!("Avaliando {}…", s.id);
        reports.push(match run_scenario(s, &ctx, repetitions) {
            Ok(v) => v,
            Err(e) => {
                json!({"id":s.id,"adapter":s.adapter,"status":"reprovado","reason":e,"samples":[]})
            }
        });
    }
    let comparisons: Vec<_> = suite
        .comparisons
        .iter()
        .map(|c| compare(c, &reports))
        .collect();
    let failed = reports
        .iter()
        .chain(&comparisons)
        .any(|r| r["status"] == "reprovado");
    let unavailable = reports
        .iter()
        .chain(&comparisons)
        .any(|r| r["status"] != "aprovado");
    let report = json!({"report_version":1,"evaluator_version":env!("CARGO_PKG_VERSION"),"evaluator_binary_sha256":hash(&fs::read(env::current_exe().map_err(|e|e.to_string())?).map_err(|e|e.to_string())?),"suite_sha256":hash(&bytes),"suite":suite_path,"run":run,"repository":ctx.repo,"platform":{"os":env::consts::OS,"arch":env::consts::ARCH,"parallelism":std::thread::available_parallelism().ok().map(|n|n.get())},"repetitions_override":repetitions,"scenarios":reports,"comparisons":comparisons,"status":if failed{"reprovado"}else if unavailable{"incompleto"}else{"aprovado"},"timing_policy":"sequential wall time; first measured is not cold; post-validation/analysis independent; no internal adapter phase attribution"});
    write_json(&run.join("relatorio.json"), &report)?;
    fs::write(run.join("relatorio.md"), render(&report)).map_err(|e| e.to_string())?;
    println!("{}", run.join("relatorio.md").display());
    Ok(if failed {
        1
    } else if unavailable {
        2
    } else {
        0
    })
}
fn main() {
    match main_run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dir() -> PathBuf {
        unique_dir(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/executor-tests"))
            .unwrap()
    }
    fn context(d: &Path) -> Context {
        Context {
            repo: d.into(),
            lab: d.into(),
            run: d.into(),
            case: d.into(),
            out: d.join("output"),
        }
    }
    fn command(d: &Path, mode: &str) -> Invocation {
        Invocation {
            argv: vec![
                env::current_exe().unwrap().to_string_lossy().into_owned(),
                "--exact".into(),
                "tests::worker".into(),
                "--nocapture".into(),
            ],
            cwd: d.to_string_lossy().into_owned(),
            timeout_ms: 3000,
            env: BTreeMap::from([
                ("EVALUATOR_TEST_MODE".into(), mode.into()),
                (
                    "EVALUATOR_TEST_DIR".into(),
                    d.to_string_lossy().into_owned(),
                ),
            ]),
        }
    }
    fn expected() -> Expected {
        Expected {
            exit_codes: vec![7],
            nodes: 1,
            edges: 0,
            names: vec!["x".into()],
            pairs: vec![],
            metadata: BTreeMap::from([("/execution/status".into(), json!("partial"))]),
            diagnostics_min: 1,
            diagnostic_contains: vec![],
            edge_fields: BTreeMap::new(),
            depths: vec![],
        }
    }
    fn scenario(d: &Path) -> Scenario {
        Scenario {
            id: "case".into(),
            adapter: "test".into(),
            granularity: "file".into(),
            relations: "test-import".into(),
            universe: "local".into(),
            unsupported: None,
            requirements: vec![],
            preparation: Preparation::default(),
            command: Some(command(d, "emit")),
            graph: Some("{out}/graph.json".into()),
            metadata: Some("{out}/metadata.json".into()),
            expected: Some(expected()),
            repetitions: 2,
            deterministic: true,
        }
    }
    fn fixture() -> Value {
        json!({"crate":"test","nodes":[{"id":0,"path":"x","name":"x","kind":"mod","visibility":"pub"}],"edges":[]})
    }
    // Spawned only by explicit test commands; ordinary cargo test returns here.
    #[test]
    fn worker() {
        let Ok(mode) = env::var("EVALUATOR_TEST_MODE") else {
            return;
        };
        let d = PathBuf::from(env::var("EVALUATOR_TEST_DIR").unwrap());
        println!("{:?}", env::args().collect::<Vec<_>>());
        match mode.as_str() {
            "exit" => std::process::exit(7),
            "emit" | "unstable" => {
                let out = PathBuf::from(
                    env::var("EVALUATOR_OUTPUT")
                        .unwrap_or_else(|_| d.join("output").to_string_lossy().into_owned()),
                );
                let mut graph = fixture();
                if mode == "unstable" {
                    graph["crate"] = json!(out.to_string_lossy());
                }
                write_json(&out.join("graph.json"), &graph).unwrap();
                write_json(
                    &out.join("metadata.json"),
                    &json!({"execution":{"status":"partial"},"diagnostics":["expected"]}),
                )
                .unwrap();
                std::process::exit(7);
            }
            "tree" => {
                let mut child = Command::new(env::current_exe().unwrap())
                    .args(["--exact", "tests::worker", "--nocapture"])
                    .env("EVALUATOR_TEST_MODE", "descendant")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .unwrap();
                fs::write(d.join("descendant.pid"), child.id().to_string()).unwrap();
                // Timeout must stop the parent and descendant before this wait.
                std::thread::sleep(Duration::from_secs(10));
                child.wait().unwrap();
            }
            "descendant" => {
                std::thread::sleep(Duration::from_millis(900));
                fs::write(d.join("survived"), "bad").unwrap();
            }
            _ => {}
        }
    }
    #[test]
    fn argv_is_literal_without_shell() {
        let d = dir();
        let mut c = command(&d, "args");
        let literal = format!("$(touch {}) ; echo unsafe", d.join("injected").display());
        c.argv.extend(["--skip".into(), literal.clone()]);
        let p = execute(&c, &d, "literal").unwrap();
        assert_eq!(p.code, Some(0));
        assert!(fs::read_to_string(p.stdout).unwrap().contains(&literal));
        assert!(!d.join("injected").exists());
        c.argv = vec!["sh".into(), "-c".into(), "true".into()];
        assert!(check_invocation(&c).is_err());
    }
    #[test]
    fn preserves_exit_codes_and_missing_executable() {
        let d = dir();
        let p = execute(&command(&d, "exit"), &d, "exit").unwrap();
        assert_eq!(p.code, Some(7));
        assert!(!p.timed_out);
        let mut c = command(&d, "exit");
        c.argv[0] = d.join("not-installed").to_string_lossy().into_owned();
        let p = execute(&c, &d, "missing").unwrap();
        assert!(p.missing);
        assert!(p.code.is_none());
    }
    #[test]
    #[cfg(unix)]
    fn timeout_kills_descendants_in_process_group() {
        let d = dir();
        let mut c = command(&d, "tree");
        c.timeout_ms = 400;
        let p = execute(&c, &d, "tree").unwrap();
        assert!(p.timed_out);
        assert!(d.join("descendant.pid").exists());
        std::thread::sleep(Duration::from_millis(1000));
        assert!(!d.join("survived").exists());
    }
    #[test]
    fn partial_with_nonzero_code_is_approved() {
        let d = dir();
        fs::create_dir(d.join("output")).unwrap();
        let s = scenario(&d);
        let result = sample(&s, &context(&d), 0).unwrap();
        assert_eq!(result["status"], "aprovado");
        assert_eq!(result["process"]["code"], 7);
        assert!(result["analysis_ms"].is_number());
        assert_eq!(result["metadata"]["execution"]["status"], "partial");
    }
    #[test]
    fn invalid_graph_and_metadata_fail_validation() {
        let e = expected();
        let m = json!({"execution":{"status":"partial"},"diagnostics":[1]});
        let mut g = fixture();
        g["nodes"][0]["kind"] = json!("nonexistent");
        assert!(validate_artifacts(&g.to_string(), &m, &e).is_err());
        let mut g = fixture();
        g["edges"] = json!([{"id_from":0,"id_to":999,"from":"x","to":"absent","relation":"uses"}]);
        assert!(validate_artifacts(&g.to_string(), &m, &e).is_err());
        let mut m = m;
        m["graph_sha256"] = json!("bad");
        assert!(validate_artifacts(&fixture().to_string(), &m, &e).is_err());
    }
    #[test]
    fn raw_duplicate_paths_are_separate_from_analysis() {
        let mut g = fixture();
        let mut n = g["nodes"][0].clone();
        n["id"] = json!(1);
        n["name"] = json!("y");
        g["nodes"].as_array_mut().unwrap().push(n);
        let mut e = expected();
        e.nodes = 2;
        e.names.push("y".into());
        let m = json!({"execution":{"status":"partial"},"diagnostics":[1]});
        let (g, _) = validate_artifacts(&g.to_string(), &m, &e).unwrap();
        assert!(analyze(&g, &e).unwrap_err().contains("Forma crua válida"));
    }
    #[test]
    fn requirements_distinguish_missing_and_unsupported() {
        let d = dir();
        let ctx = context(&d);
        let r = Requirement {
            name: "missing".into(),
            files: vec!["{lab}/absent".into()],
            command: None,
            version_contains: None,
        };
        assert_eq!(
            requirement(&r, &ctx, &d, 0).unwrap()["status"],
            "ferramenta_ausente"
        );
        let r = Requirement {
            name: "version".into(),
            files: vec![],
            command: Some(command(&d, "args")),
            version_contains: Some("not-a-real-version".into()),
        };
        assert_eq!(
            requirement(&r, &ctx, &d, 1).unwrap()["status"],
            "nao_suportado"
        );
    }
    #[test]
    fn comparison_rejects_different_semantics_and_invalid_preparation() {
        let d = dir();
        let a = scenario(&d);
        let mut b = a.clone();
        b.id = "b".into();
        b.relations = "semantic-reference".into();
        let mut s = Suite {
            version: 1,
            scenarios: vec![a, b],
            comparisons: vec![Comparison {
                left: "case".into(),
                right: "b".into(),
                expect: "same".into(),
                reason: "test".into(),
            }],
        };
        assert!(check_suite(&s).unwrap_err().contains("incompatível"));
        s.comparisons.clear();
        s.scenarios[0]
            .preparation
            .files
            .insert("../outside".into(), json!("bad"));
        assert!(check_suite(&s).is_err());
    }
    #[test]
    fn statistics_separate_first_later_and_failed_samples() {
        let samples = vec![
            json!({"index":0,"status":"aprovado","t":10.0}),
            json!({"index":1,"status":"aprovado","t":2.0}),
            json!({"index":2,"status":"aprovado","t":4.0}),
            json!({"index":3,"status":"reprovado","t":1000.0}),
        ];
        let s = timing_summary(&samples, "t");
        assert_eq!(s["approved_samples"]["median_ms"], 4.0);
        assert_eq!(s["approved_samples"]["mad_ms"], 2.0);
        assert_eq!(s["subsequent"]["median_ms"], 3.0);
        assert_eq!(s["first_measured_ms"], 10.0);
        assert_eq!(stats(&[])["n"], 0);
    }
    #[test]
    fn directories_are_unique_even_with_threads() {
        let parent = dir();
        let jobs: Vec<_> = (0..8)
            .map(|_| {
                let p = parent.clone();
                std::thread::spawn(move || unique_dir(&p).unwrap())
            })
            .collect();
        let dirs: BTreeSet<_> = jobs.into_iter().map(|j| j.join().unwrap()).collect();
        assert_eq!(dirs.len(), 8);
    }
    #[test]
    fn preparations_and_repeated_outputs_stay_separate() {
        let d = dir();
        let mut s = scenario(&d);
        s.command
            .as_mut()
            .unwrap()
            .env
            .insert("EVALUATOR_OUTPUT".into(), "{out}".into());
        s.preparation
            .files
            .insert("input.json".into(), json!({"directory":"{case}"}));
        let r = run_scenario(&s, &context(&d), None).unwrap();
        assert_eq!(r["status"], "aprovado");
        assert_ne!(r["samples"][0]["directory"], r["samples"][1]["directory"]);
        assert_eq!(
            r["samples"][0]["graph_sha256"],
            r["samples"][1]["graph_sha256"]
        );
        assert!(d.join("case/input.json").exists());
    }
    #[test]
    fn deterministic_failure_retains_all_samples() {
        let d = dir();
        let mut s = scenario(&d);
        let c = s.command.as_mut().unwrap();
        c.env
            .insert("EVALUATOR_TEST_MODE".into(), "unstable".into());
        c.env.insert("EVALUATOR_OUTPUT".into(), "{out}".into());
        let r = run_scenario(&s, &context(&d), Some(3)).unwrap();
        assert_eq!(r["status"], "reprovado");
        assert_eq!(r["samples"].as_array().unwrap().len(), 3);
        assert_eq!(r["timings"]["adapter_command"]["approved_samples"]["n"], 1);
    }
    #[test]
    fn failed_preparation_stops_extraction() {
        let d = dir();
        let mut s = scenario(&d);
        s.preparation.commands.push(command(&d, "exit"));
        let r = run_scenario(&s, &context(&d), None).unwrap();
        assert_eq!(r["status"], "reprovado");
        assert!(r["samples"].as_array().unwrap().is_empty());
        assert_eq!(r["preparation"]["commands"][0]["code"], 7);
    }
    #[test]
    fn invalid_cwd_is_not_missing_tool() {
        let d = dir();
        let mut c = command(&d, "exit");
        c.cwd = d.join("absent-directory").to_string_lossy().into_owned();
        let r = execute(&c, &d, "cwd").unwrap();
        assert!(!r.missing);
        assert!(r.error.unwrap().contains("cwd"));
    }
    #[test]
    fn shipped_scenarios_are_valid() {
        let s: Suite = serde_json::from_str(include_str!("../cenarios.json")).unwrap();
        check_suite(&s).unwrap();
        let adapters: BTreeSet<_> = s.scenarios.iter().map(|s| s.adapter.as_str()).collect();
        assert_eq!(adapters.len(), 6);
    }
}
