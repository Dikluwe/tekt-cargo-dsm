//! Crystalline Lineage
//! @prompt lab/pacote-analise/CONTRATO.md
//! @updated 2026-09-26
//! Arena independente; I/O local, sem execução de provedores.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[cfg(target_os = "linux")]
use std::os::unix::{
    ffi::OsStrExt,
    fs::{DirBuilderExt, OpenOptionsExt},
};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

type Result<T> = std::result::Result<T, String>;
static NEXT: AtomicU64 = AtomicU64::new(0);
const HELP: &str = "lente-pacote-analise create --graph FILE [--metadata FILE] [--evidence FILE]... --out DIR [--allow-unbound-metadata] [--include-environment-values]\nlente-pacote-analise verify DIR [--require-bound] [--manifest-sha256 HEX]\nSomente arquivos já produzidos; não executa adaptadores. Linux, publicação sem sobrescrita.\n";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Fact {
    value: String,
    source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Configuration {
    sha256: String,
    source: String,
    scope: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Summary {
    provider: Option<Fact>,
    provider_version: Option<Fact>,
    tool_version: Option<Fact>,
    granularity: Option<Fact>,
    configuration: Option<Configuration>,
    execution: Option<Fact>,
    semantic_coverage: Option<Fact>,
    scope_coverage: Option<Fact>,
    // No common execution identifier exists in the consulted adapter contracts.
    execution_identity: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    role: String,
    path: String,
    sha256: String,
    size: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Privacy {
    environment_values_opt_in: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    version: u32,
    artifacts: Vec<Artifact>,
    summary: Summary,
    source_binding: String,
    evidence_source_binding: String,
    privacy: Privacy,
}
#[derive(Default)]
struct Create {
    graph: PathBuf,
    metadata: Option<PathBuf>,
    evidence: Vec<PathBuf>,
    out: PathBuf,
    allow_unbound: bool,
    include_environment: bool,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest_valid(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn fact(meta: &Value, pointer: &str) -> Option<Fact> {
    meta.pointer(pointer)?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| Fact {
            value: s.into(),
            source: pointer.into(),
        })
}
fn summarize(meta: &Value) -> Summary {
    let provider = fact(meta, "/provider");
    let declared_name = provider.as_ref().map(|f| f.value.as_str()).unwrap_or("");
    // Semantic mappings are pinned to the metadata contracts we actually read.
    let schema_key = match declared_name {
        "typescript-compiler-api-experimental" | "zig-ast-experimental" => Some("schema_version"),
        "experimental-python-ast" | "experimental-libclang-cpp" => Some("metadata_version"),
        "experimental-go-list" => Some("contract_version"),
        _ => None,
    };
    let name = if schema_key.is_some_and(|key| meta.get(key).and_then(Value::as_u64) != Some(1)) {
        ""
    } else {
        declared_name
    };
    let (execution, semantic, scope, config, config_scope, tool) = match name {
        "typescript-compiler-api-experimental" => (
            "",
            "",
            "/coverage",
            "/project",
            "locator-only",
            "/typescript_version",
        ),
        "zig-ast-experimental" => (
            "",
            "/completeness/semantics",
            "/completeness/syntax",
            "/configuration",
            "declared-fragment",
            "/zig_version",
        ),
        "experimental-python-ast" => (
            "/execution/status",
            "/semantic_coverage",
            "",
            "/roots",
            "declared-roots",
            "",
        ),
        "experimental-libclang-cpp" => (
            "/execution/status",
            "/semantic_coverage",
            "",
            "/selected_configuration",
            "declared-fragment",
            "/clang_version",
        ),
        "experimental-go-list" => (
            "/loading",
            "/semantic_completeness",
            "",
            "/configuration",
            "declared-fragment",
            "",
        ),
        "libclang" => (
            "/status",
            "",
            "",
            "/configuration",
            "declared-fragment",
            "/version",
        ),
        _ => ("", "", "", "", "", ""),
    };
    let get = |p: &str| if p.is_empty() { None } else { fact(meta, p) };
    Summary {
        provider,
        provider_version: fact(meta, "/provider_version"),
        tool_version: get(tool),
        granularity: fact(meta, "/granularity"),
        configuration: if config.is_empty() {
            None
        } else {
            meta.pointer(config)
                .filter(|v| !v.is_null())
                .map(|v| Configuration {
                    sha256: hash(&serde_json::to_vec(v).expect("JSON value serialization")),
                    source: config.into(),
                    scope: config_scope.into(),
                })
        },
        execution: get(execution).filter(|f| {
            matches!(
                f.value.as_str(),
                "complete" | "partial" | "error" | "failed"
            )
        }),
        semantic_coverage: get(semantic),
        scope_coverage: get(scope),
        execution_identity: None,
    }
}
fn has_environment(value: &Value) -> bool {
    fn nonempty(v: &Value) -> bool {
        match v {
            Value::Null => false,
            Value::Array(a) => a.iter().any(nonempty),
            Value::Object(o) => o.values().any(nonempty),
            _ => true,
        }
    }
    match value {
        Value::Object(o) => o.iter().any(|(k, v)| {
            let key = k.to_ascii_lowercase();
            ((key.contains("env") || key == "variables" || key == "vars") && nonempty(v))
                || has_environment(v)
        }),
        Value::Array(a) => a.iter().any(has_environment),
        _ => false,
    }
}
fn metadata(bytes: Option<&[u8]>) -> Result<Value> {
    match bytes {
        None => Ok(Value::Null),
        Some(b) => {
            // Do not echo arbitrary metadata values in error messages.
            let v: Value =
                serde_json::from_slice(b).map_err(|_| "metadados: JSON inválido".to_string())?;
            if !v.is_object() {
                return Err("metadados devem ser um objeto JSON".into());
            }
            Ok(v)
        }
    }
}
fn binding(meta: &Value, graph: &[u8]) -> Result<String> {
    match meta.get("graph_sha256") {
        None | Some(Value::Null) => Ok("unknown".into()),
        Some(v) => {
            let s = v
                .as_str()
                .filter(|s| digest_valid(s))
                .ok_or("graph_sha256 malformado")?;
            if s != hash(graph) {
                return Err("grafo e metadados incompatíveis: graph_sha256 diverge (outra extração ou bytes alterados)".into());
            }
            Ok("graph-digest".into())
        }
    }
}
fn validate_graph(bytes: &[u8]) -> Result<()> {
    let s = std::str::from_utf8(bytes).map_err(|_| "grafo não é UTF-8")?;
    lente_infra::desserializar_grafo(s)
        .map(|_| ())
        .map_err(|e| format!("grafo rejeitado pela Lente: {e}"))
}
fn read_regular(path: &Path) -> Result<Vec<u8>> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let mut file = options
        .open(path)
        .map_err(|e| format!("leitura de {}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("artefato não é arquivo regular".into());
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| format!("leitura de {}: {e}", path.display()))?;
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(|e| format!("escrita de {}: {e}", path.display()))
}
struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn staging(parent: &Path) -> Result<Stage> {
    for _ in 0..100 {
        let p = parent.join(format!(
            ".lente-package-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(target_os = "linux")]
        builder.mode(0o700);
        match builder.create(&p) {
            Ok(()) => return Ok(Stage(p)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("staging: {e}")),
        }
    }
    Err("não foi possível criar staging exclusivo".into())
}
#[cfg(target_os = "linux")]
fn publish(stage: &Path, out: &Path) -> Result<()> {
    let old = std::ffi::CString::new(stage.as_os_str().as_bytes()).map_err(|_| "NUL em caminho")?;
    let new = std::ffi::CString::new(out.as_os_str().as_bytes()).map_err(|_| "NUL em caminho")?;
    // SAFETY: both CStrings are live and NUL-terminated; no pointers are retained.
    let rc = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            old.as_ptr(),
            libc::AT_FDCWD,
            new.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if rc != 0 {
        return Err(format!(
            "publicação sem sobrescrita: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn publish(_: &Path, _: &Path) -> Result<()> {
    Err("publicação suportada somente em Linux".into())
}

fn create(options: &Create) -> Result<Manifest> {
    let filename = options
        .out
        .file_name()
        .filter(|n| !n.is_empty())
        .ok_or("--out precisa nomear um novo diretório")?;
    let parent = options
        .out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|e| format!("diretório pai: {e}"))?;
    let out = parent.join(filename);
    let stage = staging(&parent)?;
    let mut inputs = vec![("graph", "graph.json".to_string(), &options.graph)];
    if let Some(p) = &options.metadata {
        inputs.push(("metadata", "metadata.json".into(), p));
    }
    for (i, p) in options.evidence.iter().enumerate() {
        inputs.push(("evidence", format!("evidence-{i:04}.bin"), p));
    }
    let mut artifacts = Vec::new();
    for (role, name, path) in inputs {
        let bytes = read_regular(path)?;
        write_new(&stage.0.join(&name), &bytes)?;
        artifacts.push(Artifact {
            role: role.into(),
            path: name,
            sha256: hash(&bytes),
            size: bytes.len() as u64,
        });
    }
    // Derive claims from the copies that will actually be published.
    let graph = read_regular(&stage.0.join("graph.json"))?;
    let meta_bytes = if options.metadata.is_some() {
        Some(read_regular(&stage.0.join("metadata.json"))?)
    } else {
        None
    };
    let meta = metadata(meta_bytes.as_deref())?;
    let source_binding = binding(&meta, &graph)?;
    if options.metadata.is_some() && source_binding == "unknown" && !options.allow_unbound {
        return Err("metadados sem graph_sha256: associação desconhecida; use --allow-unbound-metadata para preservá-los sem afirmar vínculo".into());
    }
    if has_environment(&meta) && !options.include_environment {
        return Err("metadados contêm campos de ambiente: revise e use --include-environment-values para preservar esses bytes explicitamente".into());
    }
    let manifest = Manifest {
        format: "lente-analysis-package".into(),
        version: 1,
        artifacts,
        summary: summarize(&meta),
        source_binding,
        evidence_source_binding: "unknown".into(),
        privacy: Privacy {
            environment_values_opt_in: options.include_environment,
        },
    };
    write_new(
        &stage.0.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )?;
    verify(&stage.0, false, None)?;
    publish(&stage.0, &out)?;
    Ok(manifest)
}
fn verify(dir: &Path, require_bound: bool, expected: Option<&str>) -> Result<Manifest> {
    if !fs::symlink_metadata(dir)
        .map_err(|e| format!("pacote: {e}"))?
        .is_dir()
    {
        return Err("pacote deve ser diretório, não symlink".into());
    }
    let bytes = read_regular(&dir.join("manifest.json"))?;
    if let Some(expected) = expected
        && (!digest_valid(expected) || hash(&bytes) != expected)
    {
        return Err("SHA-256 externo do manifesto diverge ou é inválido".into());
    }
    let raw: Value = serde_json::from_slice(&bytes).map_err(|_| "manifesto: JSON inválido")?;
    if raw.get("version").and_then(Value::as_u64) != Some(1) {
        return Err("versão de manifesto incompatível (suportada: 1)".into());
    }
    let manifest: Manifest =
        serde_json::from_value(raw).map_err(|_| "estrutura do manifesto incompatível")?;
    if manifest.format != "lente-analysis-package" || manifest.evidence_source_binding != "unknown"
    {
        return Err("formato ou associação de evidência incompatível".into());
    }
    let mut names = BTreeSet::from(["manifest.json".to_string()]);
    let (mut graph, mut meta_bytes) = (None, None);
    let mut evidence_index = 0;
    for a in &manifest.artifacts {
        let valid_name = match a.role.as_str() {
            "graph" => a.path == "graph.json",
            "metadata" => a.path == "metadata.json",
            "evidence" => {
                let name = format!("evidence-{evidence_index:04}.bin");
                evidence_index += 1;
                a.path == name
            }
            _ => false,
        };
        if !valid_name || !names.insert(a.path.clone()) || !digest_valid(&a.sha256) {
            return Err("artefato com role/path/digest inválido ou duplicado".into());
        }
        let bytes = read_regular(&dir.join(&a.path))?;
        if bytes.len() as u64 != a.size || hash(&bytes) != a.sha256 {
            return Err(format!("artefato alterado: {}", a.path));
        }
        match a.role.as_str() {
            "graph" => graph = Some(bytes),
            "metadata" => meta_bytes = Some(bytes),
            _ => (),
        }
    }
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let name = entry
            .map_err(|e| e.to_string())?
            .file_name()
            .into_string()
            .map_err(|_| "nome de arquivo inválido")?;
        if !names.contains(&name) {
            return Err("arquivo não inventariado no pacote".into());
        }
    }
    let graph = graph.ok_or("pacote sem grafo")?;
    validate_graph(&graph)?;
    let meta = metadata(meta_bytes.as_deref())?;
    let actual_binding = binding(&meta, &graph)?;
    if manifest.source_binding != actual_binding || manifest.summary != summarize(&meta) {
        return Err("manifesto não corresponde aos metadados originais".into());
    }
    if !manifest.privacy.environment_values_opt_in && has_environment(&meta) {
        return Err("ambiente incluído sem opt-in registrado".into());
    }
    if require_bound && actual_binding != "graph-digest" {
        return Err("associação grafo/metadados desconhecida".into());
    }
    Ok(manifest)
}
fn run(args: Vec<String>) -> Result<()> {
    if args.is_empty() || args == ["--help"] {
        print!("{HELP}");
        return Ok(());
    }
    let mut i = 1;
    let mut next = || {
        let s = args.get(i).cloned();
        i += 1;
        s
    };
    let m = match args[0].as_str() {
        "create" => {
            let mut opts = Create::default();
            let mut seen = BTreeSet::new();
            while let Some(flag) = next() {
                if flag != "--evidence" && !seen.insert(flag.clone()) {
                    return Err("opção repetida".into());
                }
                match flag.as_str() {
                    "--graph" => opts.graph = next().ok_or("falta valor de --graph")?.into(),
                    "--metadata" => {
                        opts.metadata = Some(next().ok_or("falta valor de --metadata")?.into())
                    }
                    "--out" => opts.out = next().ok_or("falta valor de --out")?.into(),
                    "--evidence" => opts
                        .evidence
                        .push(next().ok_or("falta valor de --evidence")?.into()),
                    "--allow-unbound-metadata" => opts.allow_unbound = true,
                    "--include-environment-values" => opts.include_environment = true,
                    _ => return Err("opção desconhecida; consulte --help".into()),
                }
            }
            if opts.graph.as_os_str().is_empty() || opts.out.as_os_str().is_empty() {
                return Err("create exige --graph e --out".into());
            }
            create(&opts)?
        }
        "verify" => {
            let dir = next().ok_or("verify exige diretório")?;
            let (mut bound, mut expected) = (false, None);
            while let Some(flag) = next() {
                match flag.as_str() {
                    "--require-bound" if !bound => bound = true,
                    "--manifest-sha256" if expected.is_none() => {
                        expected = Some(next().ok_or("falta SHA-256")?)
                    }
                    _ => return Err("opção de verify desconhecida ou repetida".into()),
                }
            }
            verify(Path::new(&dir), bound, expected.as_deref())?
        }
        _ => return Err("comando desconhecido; consulte --help".into()),
    };
    println!("{}",serde_json::to_string_pretty(&json!({"integrity":"verified", "source_binding":m.source_binding,"evidence_source_binding":m.evidence_source_binding,"summary":m.summary})).unwrap());
    Ok(())
}
fn main() {
    if let Err(e) = run(std::env::args().skip(1).collect()) {
        eprintln!("erro: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    struct Fixture {
        dir: Stage,
        graph: PathBuf,
        meta: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let dir = staging(&std::env::temp_dir()).unwrap();
            let graph = dir.0.join("input.json");
            let meta = dir.0.join("input-meta.json");
            fs::write(&graph, include_bytes!("../fixtures/graph.json")).unwrap();
            let bytes = fs::read(&graph).unwrap();
            fs::write(
                &meta,
                serde_json::to_vec_pretty(&json!({
                    "metadata_version":1,"provider":"experimental-python-ast",
                    "granularity":"module","execution":{"status":"complete"},
                    "semantic_coverage":"partial","roots":["/project"],
                    "graph_sha256":hash(&bytes),"diagnostics":[],"custom":{"untouched":true}
                }))
                .unwrap(),
            )
            .unwrap();
            Self { dir, graph, meta }
        }
        fn options(&self) -> Create {
            Create {
                graph: self.graph.clone(),
                metadata: Some(self.meta.clone()),
                out: self.dir.0.join("package"),
                ..Create::default()
            }
        }
        fn change_meta(&self, f: impl FnOnce(&mut Value)) {
            let mut m: Value = serde_json::from_slice(&fs::read(&self.meta).unwrap()).unwrap();
            f(&mut m);
            fs::write(&self.meta, serde_json::to_vec(&m).unwrap()).unwrap();
        }
        fn assert_no_publication(&self) {
            assert!(!self.options().out.exists());
            assert!(!fs::read_dir(&self.dir.0).unwrap().any(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".lente-package-")
            }));
        }
    }
    fn edit_manifest(dir: &Path, f: impl FnOnce(&mut Value)) {
        let path = dir.join("manifest.json");
        let mut m: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        f(&mut m);
        fs::write(path, serde_json::to_vec(&m).unwrap()).unwrap();
    }
    #[test]
    fn valid_original_metadata_and_evidence_preserved() {
        let f = Fixture::new();
        let mut o = f.options();
        let evidence = f.dir.0.join("notes.txt");
        fs::write(&evidence, b"opaque evidence\0\xff").unwrap();
        o.evidence.push(evidence.clone());
        let m = create(&o).unwrap();
        assert_eq!(m.source_binding, "graph-digest");
        assert_eq!(m.summary.execution.unwrap().value, "complete");
        assert_eq!(m.summary.semantic_coverage.unwrap().value, "partial");
        assert!(m.summary.provider_version.is_none());
        assert!(m.summary.execution_identity.is_none());
        assert_eq!(
            fs::read(&f.meta).unwrap(),
            fs::read(o.out.join("metadata.json")).unwrap()
        );
        assert_eq!(
            fs::read(evidence).unwrap(),
            fs::read(o.out.join("evidence-0000.bin")).unwrap()
        );
        verify(&o.out, true, None).unwrap();
    }
    #[test]
    fn different_extractions_rejected_before_publication() {
        let f = Fixture::new();
        fs::write(
            &f.graph,
            br#"{"crate":"another-run","nodes":[],"edges":[]}"#,
        )
        .unwrap();
        assert!(create(&f.options()).unwrap_err().contains("incompatíveis"));
        f.assert_no_publication();
    }
    #[test]
    fn tampered_artifacts_detected() {
        for file in ["graph.json", "metadata.json", "evidence-0000.bin"] {
            let f = Fixture::new();
            let mut o = f.options();
            o.evidence.push(f.graph.clone());
            create(&o).unwrap();
            fs::write(o.out.join(file), b"tampered").unwrap();
            assert!(
                verify(&o.out, false, None)
                    .unwrap_err()
                    .contains("alterado")
            );
        }
    }
    #[test]
    fn missing_artifacts_detected() {
        for file in [
            "graph.json",
            "metadata.json",
            "manifest.json",
            "evidence-0000.bin",
        ] {
            let f = Fixture::new();
            let mut o = f.options();
            o.evidence.push(f.graph.clone());
            create(&o).unwrap();
            fs::remove_file(o.out.join(file)).unwrap();
            assert!(verify(&o.out, false, None).is_err());
        }
    }
    #[test]
    fn incompatible_version_rejected() {
        let f = Fixture::new();
        let o = f.options();
        create(&o).unwrap();
        edit_manifest(&o.out, |m| m["version"] = json!(2));
        assert!(verify(&o.out, false, None).unwrap_err().contains("versão"));
    }
    #[test]
    fn invalid_graph_fails_using_real_lente_reader() {
        let f = Fixture::new();
        let invalid=br#"{"crate":"x","nodes":[{"id":0,"path":"x","name":"x","kind":"banana","visibility":"pub"}],"edges":[]}"#;
        fs::write(&f.graph, invalid).unwrap();
        f.change_meta(|m| m["graph_sha256"] = json!(hash(invalid)));
        assert!(create(&f.options()).unwrap_err().contains("Lente"));
        f.assert_no_publication();
    }
    #[test]
    fn failed_copy_leaves_no_final_or_staging() {
        let f = Fixture::new();
        let mut o = f.options();
        o.evidence.push(f.dir.0.join("missing"));
        assert!(create(&o).is_err());
        f.assert_no_publication();
    }
    #[test]
    fn two_concurrent_creators_have_exactly_one_winner() {
        let f = Fixture::new();
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let o = f.options();
                let b = barrier.clone();
                std::thread::spawn(move || {
                    b.wait();
                    create(&o).is_ok()
                })
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .map(|h| usize::from(h.join().unwrap()))
                .sum::<usize>(),
            1
        );
        verify(&f.options().out, true, None).unwrap();
        assert!(!fs::read_dir(&f.dir.0).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".lente-package-")
        }));
    }
    #[test]
    fn existing_empty_or_full_directory_never_overwritten() {
        let f = Fixture::new();
        let o = f.options();
        fs::create_dir(&o.out).unwrap();
        assert!(create(&o).is_err());
        assert_eq!(fs::read_dir(&o.out).unwrap().count(), 0);
        fs::remove_dir(&o.out).unwrap();
        create(&o).unwrap();
        let original = fs::read(o.out.join("manifest.json")).unwrap();
        assert!(create(&o).is_err());
        assert_eq!(original, fs::read(o.out.join("manifest.json")).unwrap());
    }
    #[test]
    fn relocated_package_verifies_without_sources() {
        let f = Fixture::new();
        let o = f.options();
        create(&o).unwrap();
        let other = staging(&std::env::temp_dir()).unwrap();
        let moved = other.0.join("moved");
        fs::rename(&o.out, &moved).unwrap();
        fs::remove_file(&f.graph).unwrap();
        fs::remove_file(&f.meta).unwrap();
        verify(&moved, true, None).unwrap();
    }
    #[test]
    fn absent_and_unbound_metadata_remain_unknown() {
        let f = Fixture::new();
        let mut o = f.options();
        f.change_meta(|m| {
            m.as_object_mut().unwrap().remove("graph_sha256");
        });
        assert!(create(&o).is_err());
        o.allow_unbound = true;
        assert_eq!(create(&o).unwrap().source_binding, "unknown");
        assert!(verify(&o.out, true, None).is_err());
        verify(&o.out, false, None).unwrap();
        o.metadata = None;
        o.out = f.dir.0.join("no-metadata");
        let m = create(&o).unwrap();
        assert!(m.summary.provider.is_none());
        assert!(m.summary.execution.is_none());
        assert_eq!(m.source_binding, "unknown");
    }
    #[test]
    fn environment_requires_opt_in_without_redacting_original() {
        let f = Fixture::new();
        let mut o = f.options();
        f.change_meta(|m| m["configuration"] = json!({"go_env":{"SECRET":"sensitive-value"}}));
        let error = create(&o).unwrap_err();
        assert!(error.contains("ambiente"));
        assert!(!error.contains("sensitive-value"));
        f.assert_no_publication();
        o.include_environment = true;
        create(&o).unwrap();
        let manifest = fs::read_to_string(o.out.join("manifest.json")).unwrap();
        assert!(!manifest.contains("sensitive-value"));
        assert_eq!(
            fs::read(&f.meta).unwrap(),
            fs::read(o.out.join("metadata.json")).unwrap()
        );
        verify(&o.out, true, None).unwrap();
    }
    #[test]
    fn unknown_provider_not_given_inferred_semantics() {
        let m = summarize(
            &json!({"provider":"future","execution":{"status":"complete"},"semantic_coverage":"complete","configuration":{},"version":"1"}),
        );
        assert!(m.execution.is_none());
        assert!(m.semantic_coverage.is_none());
        assert!(m.configuration.is_none());
        assert!(m.provider_version.is_none());
    }
    #[test]
    fn adapter_mappings_do_not_conflate_completeness() {
        let ts = summarize(
            &json!({"schema_version":1,"provider":"typescript-compiler-api-experimental","coverage":"complete-within-declared-scope","typescript_version":"5.9.3","project":"/x/tsconfig.json"}),
        );
        assert!(ts.execution.is_none());
        assert!(ts.semantic_coverage.is_none());
        assert!(ts.scope_coverage.is_some());
        assert_eq!(ts.configuration.unwrap().scope, "locator-only");
        let zig = summarize(
            &json!({"schema_version":1,"provider":"zig-ast-experimental","completeness":{"syntax":"complete-within-declared-scope","semantics":"not-evaluated"}}),
        );
        assert!(zig.execution.is_none());
        assert_eq!(zig.semantic_coverage.unwrap().value, "not-evaluated");
        let go = summarize(
            &json!({"contract_version":1,"provider":"experimental-go-list","loading":"complete","semantic_completeness":"not-claimed"}),
        );
        assert_eq!(go.execution.unwrap().source, "/loading");
        assert_eq!(go.semantic_coverage.unwrap().value, "not-claimed");
        let c = summarize(&json!({"provider":"libclang","status":"partial","version":"18"}));
        assert_eq!(c.execution.unwrap().value, "partial");
        assert!(c.provider_version.is_none());
        assert_eq!(c.tool_version.unwrap().value, "18");
        let cpp = summarize(
            &json!({"metadata_version":1,"provider":"experimental-libclang-cpp","execution":{"status":"complete"},"semantic_coverage":"partial","selected_configuration":{"flags":["-DTEST"]}}),
        );
        assert_eq!(cpp.execution.unwrap().value, "complete");
        assert_eq!(cpp.semantic_coverage.unwrap().value, "partial");
        assert!(cpp.configuration.is_some());
    }
    #[test]
    fn summary_and_path_manipulations_rejected() {
        for (field, value) in [("summary", json!({})), ("source_binding", json!("unknown"))] {
            let f = Fixture::new();
            let o = f.options();
            create(&o).unwrap();
            edit_manifest(&o.out, |m| m[field] = value);
            assert!(verify(&o.out, false, None).is_err());
        }
        let f = Fixture::new();
        let o = f.options();
        create(&o).unwrap();
        edit_manifest(&o.out, |m| {
            m["artifacts"][0]["path"] = json!("../input.json")
        });
        assert!(verify(&o.out, false, None).is_err());
    }
    #[test]
    fn manifest_external_anchor_detects_manifest_changes() {
        let f = Fixture::new();
        let o = f.options();
        create(&o).unwrap();
        let digest = hash(&fs::read(o.out.join("manifest.json")).unwrap());
        verify(&o.out, false, Some(&digest)).unwrap();
        edit_manifest(&o.out, |m| {
            m["privacy"]["environment_values_opt_in"] = json!(true)
        });
        assert!(verify(&o.out, false, Some(&digest)).is_err());
    }
    #[test]
    fn extra_files_and_symlink_artifacts_rejected() {
        let f = Fixture::new();
        let o = f.options();
        create(&o).unwrap();
        let extra = o.out.join("extra");
        fs::write(&extra, b"x").unwrap();
        assert!(verify(&o.out, false, None).is_err());
        fs::remove_file(extra).unwrap();
        fs::remove_file(o.out.join("graph.json")).unwrap();
        std::os::unix::fs::symlink(&f.graph, o.out.join("graph.json")).unwrap();
        assert!(verify(&o.out, false, None).is_err());
    }
    #[test]
    fn unsupported_metadata_schema_is_preserved_without_interpretation() {
        let f = Fixture::new();
        f.change_meta(|m| m["metadata_version"] = json!(2));
        let o = f.options();
        let m = create(&o).unwrap();
        assert!(m.summary.execution.is_none());
        assert!(m.summary.semantic_coverage.is_none());
        assert!(m.summary.configuration.is_none());
        assert_eq!(
            fs::read(&f.meta).unwrap(),
            fs::read(o.out.join("metadata.json")).unwrap()
        );
        verify(&o.out, true, None).unwrap();
    }
    #[test]
    fn replacing_metadata_and_its_inventory_hash_still_checks_graph_binding() {
        let f = Fixture::new();
        let o = f.options();
        create(&o).unwrap();
        f.change_meta(|m| m["graph_sha256"] = json!(hash(b"another graph")));
        let replacement = fs::read(&f.meta).unwrap();
        fs::write(o.out.join("metadata.json"), &replacement).unwrap();
        edit_manifest(&o.out, |m| {
            m["artifacts"][1]["sha256"] = json!(hash(&replacement));
            m["artifacts"][1]["size"] = json!(replacement.len());
        });
        assert!(
            verify(&o.out, false, None)
                .unwrap_err()
                .contains("incompatíveis")
        );
    }
    #[test]
    fn bad_digest_rejected_even_with_allow_unbound() {
        let f = Fixture::new();
        let mut o = f.options();
        o.allow_unbound = true;
        f.change_meta(|m| m["graph_sha256"] = json!("bad"));
        assert!(create(&o).is_err());
    }
    #[test]
    fn cli_create_verify_and_argument_errors() {
        let f = Fixture::new();
        let o = f.options();
        run(vec![
            "create".into(),
            "--graph".into(),
            f.graph.display().to_string(),
            "--metadata".into(),
            f.meta.display().to_string(),
            "--out".into(),
            o.out.display().to_string(),
        ])
        .unwrap();
        run(vec![
            "verify".into(),
            o.out.display().to_string(),
            "--require-bound".into(),
        ])
        .unwrap();
        assert!(run(vec!["create".into()]).is_err());
        assert!(run(vec!["verify".into()]).is_err());
    }
}
