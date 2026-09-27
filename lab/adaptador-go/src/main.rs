//! Crystalline Lineage
//! @prompt lab/adaptador-go/CONTRATO.md
//! @layer lab

use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::PathBuf,
    process::Command,
};

const HELP: &str = "lente-adaptador-go --project DIR --goos OS --goarch ARCH --out DIR [--tests exclude|include] [--tags tag1,tag2] [--work auto|off|ARQUIVO] [--module-cache DIR] [--pattern ./...]...\nSaídas: grafo.json e metadados.json. cgo=0, módulos readonly, downloads bloqueados, sem cache.";
const POLICY: &[(&str, &str)] = &[
    ("GOTOOLCHAIN", "local"),
    ("GOPROXY", "off"),
    ("GOSUMDB", "off"),
    ("GONOPROXY", "none"),
    ("GONOSUMDB", "none"),
    ("GOPRIVATE", ""),
    ("GOVCS", "*:off"),
    ("GOENV", "off"),
    ("GOFLAGS", ""),
    ("GO111MODULE", "on"),
    ("CGO_ENABLED", "0"),
    ("GOEXPERIMENT", ""),
    ("GOCACHEPROG", ""),
];
const LIMITS: &[&str] = &[
    "Granularidade de pacote: não descreve funções nem comprova equivalência comportamental.",
    "Somente imports Go para esta configuração; não é prova de compilação ou completude semântica.",
    "cgo desabilitado: arquivos que importam C e dependências nativas ficam fora do universo.",
    "Não representa relações de linker, assembly, plugins, reflexão, recursos embed ou outras linguagens.",
    "Testes incluídos acrescentam variantes e drivers sintéticos; nenhum teste é executado.",
    "Visibilidade pub e campo crate são acomodações do contrato; filtros de stdlib Rust não classificam Go.",
];

#[derive(Debug, Clone)]
struct Options {
    project: PathBuf,
    out: PathBuf,
    goos: String,
    goarch: String,
    tests: bool,
    tags: String,
    work: String,
    patterns: Vec<String>,
    module_cache: Option<PathBuf>,
}
impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut pairs = BTreeMap::new();
        let mut patterns = Vec::new();
        let mut it = args.into_iter();
        while let Some(key) = it.next() {
            if ![
                "--project",
                "--out",
                "--goos",
                "--goarch",
                "--tests",
                "--tags",
                "--work",
                "--pattern",
                "--module-cache",
            ]
            .contains(&key.as_str())
            {
                return Err(format!("Opção desconhecida: {key}\n{HELP}"));
            }
            let value = it.next().ok_or_else(|| format!("Falta valor para {key}"))?;
            if value.is_empty() || value.starts_with('-') {
                return Err(format!("Valor inválido para {key}"));
            }
            if key == "--pattern" {
                patterns.push(value);
            } else if pairs.insert(key.clone(), value).is_some() {
                return Err(format!("Opção repetida: {key}"));
            }
        }
        let mut required = |k: &str| {
            pairs
                .remove(k)
                .ok_or_else(|| format!("Obrigatório: {k}\n{HELP}"))
        };
        let project = PathBuf::from(required("--project")?);
        let out = PathBuf::from(required("--out")?);
        let goos = required("--goos")?;
        let goarch = required("--goarch")?;
        let tests = match pairs.remove("--tests").as_deref().unwrap_or("exclude") {
            "include" => true,
            "exclude" => false,
            _ => return Err("--tests deve ser include ou exclude".into()),
        };
        if patterns.is_empty() {
            patterns.push("./...".into());
        }
        patterns.sort();
        patterns.dedup();
        let tags = pairs.remove("--tags").unwrap_or_default();
        let tags = tags
            .split(',')
            .filter(|s| !s.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(",");
        Ok(Self {
            project,
            out,
            goos,
            goarch,
            tests,
            tags,
            work: pairs.remove("--work").unwrap_or_else(|| "auto".into()),
            patterns,
            module_cache: pairs.remove("--module-cache").map(PathBuf::from),
        })
    }
    fn command(&self) -> Command {
        let mut cmd = Command::new("go");
        cmd.current_dir(&self.project)
            .envs(POLICY.iter().copied())
            .env("GOOS", &self.goos)
            .env("GOARCH", &self.goarch)
            .env("GOWORK", &self.work);
        if let Some(cache) = &self.module_cache {
            cmd.env("GOMODCACHE", cache);
        }
        // Architecture feature levels affect build tags. Use official defaults,
        // not hidden settings inherited from the caller.
        for key in [
            "GOAMD64",
            "GOARM",
            "GOARM64",
            "GO386",
            "GOMIPS",
            "GOMIPS64",
            "GOPPC64",
            "GORISCV64",
            "GOWASM",
            "GOFIPS140",
        ] {
            cmd.env_remove(key);
        }
        cmd
    }
}
fn string<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
fn strings(v: &Value, k: &str) -> Vec<String> {
    v.get(k)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
fn diagnostic(diags: &mut BTreeSet<String>, context: &str, error: &Value) {
    if !error.is_null() {
        diags.insert(format!("{context}: {error}"));
    }
}

struct Extraction {
    graph: Value,
    metadata: Value,
    partial: bool,
}
fn convert(bytes: &[u8], mut diags: BTreeSet<String>) -> Extraction {
    let mut packages = BTreeMap::new();
    for result in serde_json::Deserializer::from_slice(bytes).into_iter::<Value>() {
        match result {
            Ok(p) if !string(&p, "ImportPath").is_empty() => {
                let path = string(&p, "ImportPath").to_owned();
                diagnostic(&mut diags, &path, &p["Error"]);
                diagnostic(&mut diags, &path, &p["Module"]["Error"]);
                for err in p["DepsErrors"].as_array().into_iter().flatten() {
                    diagnostic(&mut diags, &path, err);
                }
                if p["Incomplete"] == true {
                    diags.insert(format!("{path}: carregamento incompleto"));
                }
                if packages.insert(path.clone(), p).is_some() {
                    diags.insert(format!("{path}: identidade repetida no go list"));
                }
            }
            Ok(_) => {
                diags.insert("Objeto sem ImportPath no go list".into());
            }
            Err(e) => {
                diags.insert(format!("Stream JSON inválido: {e}"));
                break;
            }
        }
    }
    if packages.is_empty() {
        diags.insert("Nenhum pacote carregado".into());
    }
    let mut edges = BTreeSet::new();
    for (path, p) in &packages {
        for import in strings(p, "Imports") {
            let target = p["ImportMap"]
                .get(&import)
                .and_then(Value::as_str)
                .unwrap_or(&import)
                .to_owned();
            edges.insert((path.clone(), target));
        }
    }
    let missing: BTreeSet<_> = edges
        .iter()
        .map(|(_, t)| t.clone())
        .filter(|t| !packages.contains_key(t))
        .collect();
    for path in missing {
        diags.insert(format!("{path}: endpoint não carregado; nó marcador"));
        packages.insert(path.clone(), json!({"ImportPath":path,"Unresolved":true}));
    }
    let ids: BTreeMap<_, _> = packages
        .keys()
        .enumerate()
        .map(|(id, p)| (p.clone(), id))
        .collect();
    let mut nodes = Vec::new();
    let mut details = Vec::new();
    for (path, p) in &packages {
        let for_test = string(p, "ForTest");
        // go list can store the generated driver under a hash in GOCACHE.
        // Match the documented .test identity to an actual tested package.
        let driver = string(p, "Name") == "main"
            && strings(p, "GoFiles")
                .iter()
                .any(|file| std::path::Path::new(file).is_absolute())
            && path
                .strip_suffix(".test")
                .and_then(|base| packages.get(base))
                .is_some_and(|base| {
                    !strings(base, "TestGoFiles").is_empty()
                        || !strings(base, "XTestGoFiles").is_empty()
                });
        let variant = if driver {
            "test-driver"
        } else if !for_test.is_empty() && path == &format!("{for_test}_test [{for_test}.test]") {
            "external-test"
        } else if !for_test.is_empty() {
            "test-variant"
        } else {
            "normal"
        };
        let origin = if p["Standard"] == true {
            "stdlib"
        } else if p["Module"]["Main"] == true {
            "project"
        } else if !p["Module"].is_null() {
            "external"
        } else {
            "unknown"
        };
        let name = if string(p, "Name").is_empty() {
            path.as_str()
        } else {
            string(p, "Name")
        };
        nodes.push(json!({"id":ids[path],"path":path,"name":name,"kind":"mod","visibility":"pub"}));
        details.push(json!({"path":path,"origin":origin,"variant":variant,"for_test":for_test,
            "module":p["Module"],"directory":p["Dir"],"dep_only":p["DepOnly"]==true,
            "loaded": p["Unresolved"] != true && !string(p,"Dir").is_empty(),
            "error":p["Error"],"incomplete":p["Incomplete"]==true,
            "go_files":strings(p,"GoFiles"),"ignored_go_files":strings(p,"IgnoredGoFiles"),
            "test_go_files":strings(p,"TestGoFiles"),"external_test_go_files":strings(p,"XTestGoFiles")}));
    }
    let edges: Vec<_> = edges.into_iter().map(|(f,t)| json!({"id_from":ids[&f],"id_to":ids[&t],"from":f,"to":t,"relation":"uses","uses_kind":"import"})).collect();
    let partial = !diags.is_empty();
    Extraction {
        graph: json!({"crate":"go-packages","nodes":nodes,"edges":edges}),
        metadata: json!({"provider":"experimental-go-list","contract_version":1,"language":"Go","granularity":"package",
            "loading":if partial {"partial"} else {"complete"},"semantic_completeness":"not-claimed",
            "cache":"bypass","limitations":LIMITS,"diagnostics":diags,"packages":details}),
        partial,
    }
}
fn extract(options: &Options) -> Result<Extraction, String> {
    let mut options = options.clone();
    options.project = options
        .project
        .canonicalize()
        .map_err(|e| format!("Diretório do projeto: {e}"))?;
    if options.work != "auto" && options.work != "off" {
        options.work = PathBuf::from(&options.work)
            .canonicalize()
            .map_err(|e| format!("go.work: {e}"))?
            .to_string_lossy()
            .into_owned();
    }
    if let Some(cache) = &options.module_cache {
        options.module_cache = Some(
            cache
                .canonicalize()
                .map_err(|e| format!("Cache de módulos: {e}"))?,
        );
    }
    let env_output = options
        .command()
        .args([
            "env",
            "-json",
            "GOOS",
            "GOARCH",
            "GOVERSION",
            "GOROOT",
            "GOMOD",
            "GOWORK",
            "GOPATH",
            "GOMODCACHE",
            "CGO_ENABLED",
            "GOAMD64",
            "GOARM",
            "GOARM64",
            "GO386",
            "GOMIPS",
            "GOMIPS64",
            "GOPPC64",
            "GORISCV64",
            "GOWASM",
            "GOFIPS140",
        ])
        .output()
        .map_err(|e| {
            format!("Não foi possível executar Go instalado: {e}. Instale o toolchain manualmente.")
        })?;
    let mut diags = BTreeSet::new();
    let go_env: Value = match serde_json::from_slice(&env_output.stdout) {
        Ok(v) => v,
        Err(e) => {
            diags.insert(format!("go env: JSON inválido: {e}"));
            Value::Null
        }
    };
    if !env_output.status.success() {
        diags.insert(format!(
            "go env falhou: {}",
            String::from_utf8_lossy(&env_output.stderr).trim()
        ));
    }
    let mut args = vec![
        "list".to_owned(),
        "-deps".into(),
        "-e".into(),
        "-json".into(),
        "-mod=readonly".into(),
        format!("-tags={}", options.tags),
    ];
    if options.tests {
        args.push("-test".into());
    }
    args.extend(options.patterns.clone());
    let output = options
        .command()
        .args(&args)
        .output()
        .map_err(|e| format!("go list: {e}"))?;
    if !output.status.success() {
        diags.insert(format!("go list terminou com {}", output.status));
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !stderr.is_empty() {
        diags.insert(format!("go list stderr: {stderr}"));
    }
    let mut extraction = convert(&output.stdout, diags);
    extraction.metadata["configuration"] = json!({"project":options.project,"goos":options.goos,"goarch":options.goarch,
        "tags":options.tags,"tests":if options.tests {"include"} else {"exclude"},"work":options.work,"module_mode":"readonly","cgo":false,
        "go_env":go_env,"command":args,"environment_policy":POLICY.iter().copied().collect::<BTreeMap<_,_>>()});
    extraction.metadata["guidance"] = json!(
        "Dependência/toolchain ausente: disponibilize manualmente a versão requerida no cache local ou configure replace/go.work. Downloads automáticos bloqueados. Reveja diagnostics; não trate saída parcial como completa."
    );
    let json = serde_json::to_string(&extraction.graph).map_err(|e| e.to_string())?;
    lente_infra::desserializar_grafo(&json)
        .map_err(|e| format!("Grafo rejeitado pela Lente: {e:?}"))?;
    Ok(extraction)
}
fn run() -> Result<i32, String> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["--help"] {
        println!("{HELP}");
        return Ok(0);
    }
    let options = Options::parse(args)?;
    let extraction = extract(&options)?;
    fs::create_dir_all(&options.out).map_err(|e| e.to_string())?;
    for (name, value) in [
        ("grafo.json", &extraction.graph),
        ("metadados.json", &extraction.metadata),
    ] {
        let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        fs::write(options.out.join(name), bytes).map_err(|e| format!("{name}: {e}"))?;
    }
    eprintln!(
        "{} pacotes, {} imports diretos; carregamento {}. Leia metadados.json.",
        extraction.graph["nodes"].as_array().unwrap().len(),
        extraction.graph["edges"].as_array().unwrap().len(),
        extraction.metadata["loading"]
    );
    Ok(if extraction.partial { 2 } else { 0 })
}
fn main() {
    match run() {
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
    fn options(fixture: &str) -> Options {
        Options {
            project: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join(fixture),
            out: PathBuf::new(),
            goos: "linux".into(),
            goarch: "amd64".into(),
            tests: false,
            tags: String::new(),
            work: "off".into(),
            patterns: vec!["./...".into()],
            module_cache: None,
        }
    }
    fn edge(g: &Value, f: &str, t: &str) -> bool {
        g["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["from"] == f && e["to"] == t)
    }
    fn package<'a>(e: &'a Extraction, path: &str) -> &'a Value {
        e.metadata["packages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["path"] == path)
            .unwrap()
    }
    #[test]
    fn simple_and_deterministic() {
        let a = extract(&options("simple")).unwrap();
        let b = extract(&options("simple")).unwrap();
        assert!(!a.partial, "{}", a.metadata);
        assert_eq!(a.graph, b.graph);
        assert_eq!(a.metadata, b.metadata);
        assert_eq!(a.graph["nodes"].as_array().unwrap().len(), 1);
        assert_eq!(package(&a, "example.test/simple")["origin"], "project");
    }
    #[test]
    fn direct_imports_and_core_transitivity() {
        let e = extract(&options("chain")).unwrap();
        assert!(!e.partial, "{}", e.metadata);
        assert_eq!(e.graph["edges"].as_array().unwrap().len(), 2);
        assert!(!edge(
            &e.graph,
            "example.test/chain/a",
            "example.test/chain/c"
        ));
        let graph = lente_infra::desserializar_grafo(&e.graph.to_string()).unwrap();
        let raio = lente_core::domain::raio::calcular_raio(&graph, &"example.test/chain/c".into())
            .unwrap();
        assert_eq!(raio.montante[&"example.test/chain/a".into()], 2);
        assert_eq!(raio.montante[&"example.test/chain/b".into()], 1);
        assert!(
            !raio
                .montante
                .contains_key(&"example.test/chain/isolated".into())
        );
    }
    #[test]
    fn external_local_replace() {
        let e = extract(&options("replace/app")).unwrap();
        assert!(!e.partial, "{}", e.metadata);
        assert!(edge(&e.graph, "example.test/app", "example.test/external"));
        let p = package(&e, "example.test/external");
        assert_eq!(p["origin"], "external");
        assert_eq!(p["module"]["Replace"]["Path"], "../external");
    }
    #[test]
    fn external_available_in_module_cache_without_replace() {
        let mut o = options("cached/app");
        o.module_cache =
            Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/cached/module-cache"));
        let e = extract(&o).unwrap();
        assert!(!e.partial, "{}", e.metadata);
        assert!(edge(
            &e.graph,
            "example.test/cacheconsumer",
            "example.test/cached"
        ));
        let p = package(&e, "example.test/cached");
        assert_eq!(p["origin"], "external");
        assert_eq!(p["module"]["Version"], "v1.0.0");
        assert!(p["module"]["Replace"].is_null());
    }
    #[test]
    fn workspace() {
        let mut o = options("workspace");
        o.work = "auto".into();
        o.patterns = vec!["./one/...".into(), "./two/...".into()];
        let e = extract(&o).unwrap();
        assert!(!e.partial, "{}", e.metadata);
        assert!(edge(&e.graph, "example.test/one", "example.test/two"));
        assert_eq!(package(&e, "example.test/two")["origin"], "project");
        assert!(
            e.metadata["configuration"]["go_env"]["GOWORK"]
                .as_str()
                .unwrap()
                .ends_with("go.work")
        );
    }
    #[test]
    fn tags_os_arch_and_cgo() {
        let mut o = options("constraints");
        let a = extract(&o).unwrap();
        assert!(!a.partial, "{}", a.metadata);
        assert!(edge(
            &a.graph,
            "example.test/constraints",
            "example.test/constraints/linuxonly"
        ));
        assert!(!edge(
            &a.graph,
            "example.test/constraints",
            "example.test/constraints/extra"
        ));
        o.tags = "feature".into();
        o.goos = "darwin".into();
        o.goarch = "arm64".into();
        let b = extract(&o).unwrap();
        assert!(!b.partial, "{}", b.metadata);
        assert!(edge(
            &b.graph,
            "example.test/constraints",
            "example.test/constraints/extra"
        ));
        assert!(edge(
            &b.graph,
            "example.test/constraints",
            "example.test/constraints/armonly"
        ));
        assert!(!edge(
            &b.graph,
            "example.test/constraints",
            "example.test/constraints/linuxonly"
        ));
        assert!(
            strings(package(&b, "example.test/constraints"), "ignored_go_files")
                .contains(&"cgo.go".into())
        );
    }
    #[test]
    fn tests_keep_variants_and_stdlib() {
        let mut o = options("tests");
        let normal = extract(&o).unwrap();
        assert!(!normal.partial);
        assert_eq!(normal.graph["nodes"].as_array().unwrap().len(), 1);
        o.tests = true;
        let e = extract(&o).unwrap();
        assert!(!e.partial, "{}", e.metadata);
        assert_eq!(package(&e, "example.test/testing")["variant"], "normal");
        assert_eq!(
            package(&e, "example.test/testing [example.test/testing.test]")["variant"],
            "test-variant"
        );
        assert_eq!(
            package(&e, "example.test/testing_test [example.test/testing.test]")["for_test"],
            "example.test/testing"
        );
        assert_eq!(package(&e, "testing")["origin"], "stdlib");
        assert_eq!(
            package(&e, "example.test/testing_test [example.test/testing.test]")["variant"],
            "external-test"
        );
        let repeated = extract(&o).unwrap();
        assert_eq!(e.graph, repeated.graph);
        assert_eq!(e.metadata, repeated.metadata);
        assert!(edge(
            &e.graph,
            "example.test/testing_test [example.test/testing.test]",
            "example.test/testing [example.test/testing.test]"
        ));
        assert_eq!(
            package(&e, "example.test/testing.test")["variant"],
            "test-driver"
        );
    }
    #[test]
    fn missing_dependency_preserves_partial_graph() {
        let e = extract(&options("broken")).unwrap();
        assert!(e.partial);
        assert!(edge(
            &e.graph,
            "example.test/broken",
            "example.invalid/missing"
        ));
        assert_eq!(package(&e, "example.test/broken/good")["loaded"], true);
        assert!(!e.metadata["diagnostics"].as_array().unwrap().is_empty());
    }
    #[test]
    fn required_newer_toolchain_is_not_downloaded() {
        let e = extract(&options("newer-toolchain")).unwrap();
        assert!(e.partial);
        let diagnostics = e.metadata["diagnostics"].to_string();
        assert!(diagnostics.contains("GOTOOLCHAIN=local"), "{diagnostics}");
        assert!(e.graph["nodes"].as_array().unwrap().is_empty());
    }
    #[test]
    fn invalid_package() {
        let e = extract(&options("syntax")).unwrap();
        assert!(e.partial);
    }
    #[test]
    fn malformed_stream_and_missing_endpoint() {
        let e = convert(
            br#"{"ImportPath":"a","Imports":["b"],"Deps":["c"]} {bad"#,
            BTreeSet::new(),
        );
        assert!(e.partial);
        assert!(edge(&e.graph, "a", "b"));
        assert!(!edge(&e.graph, "a", "c"));
        assert_eq!(package(&e, "b")["loaded"], false);
        lente_infra::desserializar_grafo(&e.graph.to_string()).unwrap();
    }
    #[test]
    fn required_config_and_offline_policy() {
        assert!(Options::parse(Vec::<String>::new()).is_err());
        let cmd = options("simple").command();
        let envs: BTreeMap<_, _> = cmd
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        for (k, v) in POLICY {
            assert_eq!(envs[*k].as_deref(), Some(*v));
        }
        assert!(envs["GOAMD64"].is_none());
    }
}
