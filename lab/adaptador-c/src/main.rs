//! Crystalline Lineage
//! @prompt lab/adaptador-c/CONTRATO.md
//! @layer Lab
//! @updated 2026-09-26
mod clang;

use clang::Config;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const LIMITS: &[&str] = &[
    "Somente inclusões ativas: não analisa chamadas ou referências entre funções.",
    "Uma saída por configuração e TU; não há união de variantes.",
    "Sem C++, PCH, modules, plugins, wrappers, response files ou includes forçados.",
    "Flags não suportadas rejeitam a TU. Headers gerados devem existir previamente.",
    "Paths absolutos: determinismo restrito ao mesmo checkout, ambiente e libclang.",
    "Posições de diretivas estão nos metadados, pois Aresta da Lente não tem posição.",
];
fn absolute(base: &Path, file: &str) -> Result<PathBuf, String> {
    base.join(file)
        .canonicalize()
        .map_err(|e| format!("{file}: {e}"))
}

// Lista positiva: jamais repassa mecanismos que carregam plugins ou comandos.
// Opções desconhecidas falham explicitamente, em vez de alterar a configuração.
fn arguments(c: &Config) -> Result<Vec<String>, String> {
    let dir = Path::new(&c.directory);
    if !dir.is_absolute() || !dir.is_dir() {
        return Err("directory deve ser absoluto e existente".into());
    }
    let source = absolute(dir, &c.file)?;
    let driver = c.arguments.first().ok_or("argv vazio")?;
    let name = Path::new(driver)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if !["cc", "gcc", "clang"].iter().any(|n| {
        name == *n
            || name
                .strip_prefix(&format!("{n}-"))
                .is_some_and(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit() || c == '.'))
    }) {
        return Err(format!("driver/wrapper não suportado: {driver}"));
    }
    let mut out = vec![
        driver.clone(),
        "-working-directory".into(),
        c.directory.clone(),
    ];
    let mut i = 1;
    let mut found = false;
    let mut language = false;
    while i < c.arguments.len() {
        let arg = &c.arguments[i];
        if ["-o", "-MF", "-MT", "-MQ"].contains(&arg.as_str()) {
            i += 1;
            c.arguments
                .get(i)
                .ok_or_else(|| format!("valor ausente para {arg}"))?;
        } else if [
            "-c",
            "-fsyntax-only",
            "-MD",
            "-MMD",
            "-MP",
            "-g",
            "-g0",
            "-g1",
            "-g2",
            "-g3",
        ]
        .contains(&arg.as_str())
        {
            // Somente geração de artefatos, sem efeito na análise.
        } else if [
            "-I",
            "-isystem",
            "-iquote",
            "-idirafter",
            "-D",
            "-U",
            "-isysroot",
            "--sysroot",
            "-target",
            "--target",
            "-std",
            "-x",
        ]
        .contains(&arg.as_str())
        {
            i += 1;
            let value = c
                .arguments
                .get(i)
                .ok_or_else(|| format!("valor ausente para {arg}"))?;
            if arg == "-x" {
                if value != "c" {
                    return Err("somente -x c é suportado".into());
                }
                language = true;
            }
            if arg == "-std" && !c_standard(value) {
                return Err("standard não C".into());
            }
            out.extend([arg.clone(), value.clone()]);
        } else if arg == "-xc" {
            language = true;
            out.push(arg.clone());
        } else if let Some(standard) = arg.strip_prefix("-std=") {
            if !c_standard(standard) {
                return Err("standard não C".into());
            }
            out.push(arg.clone());
        } else if [
            "-I",
            "-D",
            "-U",
            "-isystem",
            "-iquote",
            "-idirafter",
            "--sysroot=",
            "--target=",
            "-march=",
            "-mcpu=",
            "-mtune=",
        ]
        .iter()
        .any(|p| arg.starts_with(p) && arg.len() > p.len())
            || [
                "-O0",
                "-O1",
                "-O2",
                "-O3",
                "-Os",
                "-Oz",
                "-Og",
                "-Ofast",
                "-m32",
                "-m64",
                "-pthread",
                "-nostdinc",
                "-undef",
                "-ffreestanding",
                "-fshort-wchar",
                "-funsigned-char",
                "-fsigned-char",
                "-fms-extensions",
                "-fPIC",
                "-fpic",
                "-fPIE",
                "-fpie",
                "-pedantic",
                "-pedantic-errors",
            ]
            .contains(&arg.as_str())
            || arg.starts_with("-W") && !arg.contains(',')
        {
            out.push(arg.clone());
        } else if !arg.starts_with('-')
            && !arg.starts_with('@')
            && absolute(dir, arg).ok().as_ref() == Some(&source)
        {
            if found {
                return Err("fonte duplicada no argv".into());
            }
            found = true;
        } else {
            return Err(format!("opção/entrada não suportada: {arg}"));
        }
        i += 1;
    }
    if !found {
        return Err("file não encontrado no argv".into());
    }
    if !language && source.extension().and_then(|s| s.to_str()) != Some("c") {
        return Err("fonte deve ser .c ou usar -x c".into());
    }
    out.push("-fsyntax-only".into());
    out.push(source.to_string_lossy().into_owned());
    Ok(out)
}
fn c_standard(s: &str) -> bool {
    [
        "c89",
        "c90",
        "iso9899:1990",
        "iso9899:199409",
        "c99",
        "iso9899:1999",
        "c11",
        "iso9899:2011",
        "c17",
        "c18",
        "iso9899:2017",
        "c2x",
        "c23",
        "gnu89",
        "gnu90",
        "gnu99",
        "gnu11",
        "gnu17",
        "gnu18",
        "gnu2x",
        "gnu23",
    ]
    .contains(&s)
}

fn extract(config: &Config, root: &Path) -> (Value, Value) {
    let mut meta = json!({"provider":"libclang","version":clang::version(),"configuration":config.json(),
        "project_root":root,"granularity":"file","relation":"include","variant_policy":"separate-per-command-and-tu",
        "header_policy":"include-project-external-and-system","limits":LIMITS,
        "status":"partial","unanalyzed_units":[config.file],"diagnostics":[],"occurrences":[],"files":[],
        "environment": (["CPATH","C_INCLUDE_PATH","CPLUS_INCLUDE_PATH","SDKROOT","MACOSX_DEPLOYMENT_TARGET"].iter().map(|k| (*k,std::env::var(k).ok())).collect::<BTreeMap<_,_>>())});
    let empty = json!({"crate":"c-includes","nodes":[],"edges":[]});
    let argv = match arguments(config) {
        Ok(a) => a,
        Err(e) => {
            meta["diagnostics"] = json!([{"severity":3,"message":e}]);
            return (empty, meta);
        }
    };
    meta["effective_arguments"] = json!(argv);
    let parsed = match clang::parse(config, &argv) {
        Ok(p) => p,
        Err(e) => {
            meta["diagnostics"] = json!([{"severity":3,"message":e}]);
            return (empty, meta);
        }
    };
    meta["diagnostics"] = json!(parsed.diagnostics);
    if parsed.failed {
        return (empty, meta);
    }
    let source = absolute(Path::new(&config.directory), &config.file)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut paths = BTreeSet::from([source]);
    let mut pairs = BTreeSet::new();
    let mut occurrences = Vec::new();
    for inc in &parsed.includes {
        paths.extend([inc.from.clone(), inc.to.clone()]);
        pairs.insert((inc.from.clone(), inc.to.clone()));
        occurrences.push(
            json!({"from":inc.from,"to":inc.to,"relation":"include","direct":true,
            "position":{"file":inc.from,"line":inc.line,"column":inc.column,"offset":inc.offset}}),
        );
    }
    let ids: BTreeMap<_, _> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| (p.clone(), i + 1))
        .collect();
    let nodes: Vec<_> = paths.iter().map(|p|json!({"id":ids[p],"path":p,"name":Path::new(p).file_name().unwrap_or_default().to_string_lossy(),"kind":"mod","visibility":"priv"})).collect();
    let edges: Vec<_> = pairs.iter().map(|(a,b)|json!({"from":a,"id_from":ids[a],"to":b,"id_to":ids[b],"relation":"uses","uses_kind":"import"})).collect();
    meta["files"] = json!(paths.iter().map(|p|json!({"path":p,"class":if parsed.system.contains(p) {"system"} else if Path::new(p).starts_with(root) {"project"} else {"external"}})).collect::<Vec<_>>());
    meta["occurrences"] = json!(occurrences);
    meta["status"] = json!("complete");
    meta["unanalyzed_units"] = json!([]);
    (
        json!({"crate":"c-includes","nodes":nodes,"edges":edges}),
        meta,
    )
}

fn run(db: &Path, root: &Path, output: &Path) -> Result<bool, String> {
    let db = db.canonicalize().map_err(|e| e.to_string())?;
    if db.file_name().and_then(|s| s.to_str()) != Some("compile_commands.json") {
        return Err("esperado compile_commands.json".into());
    }
    // Pré-valida o JSON; impede fallback implícito para compile_flags.txt.
    let value: Value = serde_json::from_slice(&std::fs::read(&db).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if !value.is_array() || value.as_array().unwrap().is_empty() {
        return Err("base deve ter ao menos uma entrada".into());
    }
    for (i, entry) in value.as_array().unwrap().iter().enumerate() {
        if !entry["directory"].is_string()
            || !entry["file"].is_string()
            || if let Some(argv) = entry.get("arguments") {
                !argv
                    .as_array()
                    .is_some_and(|a| !a.is_empty() && a.iter().all(Value::is_string))
            } else {
                entry["command"].as_str().is_none_or(|s| s.is_empty())
            }
        {
            return Err(format!(
                "entrada {i}: directory/file e arguments ou command válidos são obrigatórios"
            ));
        }
        if entry.get("mappings").is_some() {
            return Err(format!("entrada {i}: source mappings não suportados"));
        }
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let configs = clang::database(db.parent().unwrap())?;
    if configs.is_empty() {
        return Err("base não contém comandos".into());
    }
    std::fs::create_dir_all(output).map_err(|e| e.to_string())?;
    // Não sobrescreve relatórios existentes, e não lê cache.
    if std::fs::read_dir(output)
        .map_err(|e| e.to_string())?
        .next()
        .is_some()
    {
        return Err("diretório de saída deve estar vazio".into());
    }
    let mut manifest = Vec::new();
    let mut partial = false;
    for (i, config) in configs.iter().enumerate() {
        let (graph, meta) = extract(config, &root);
        let graph_name = format!("graph-{i:04}.json");
        let meta_name = format!("metadata-{i:04}.json");
        partial |= meta["status"] != "complete";
        write(&output.join(&graph_name), &graph)?;
        write(&output.join(&meta_name), &meta)?;
        manifest.push(json!({"graph":graph_name,"metadata":meta_name,"file":config.file,"status":meta["status"]}));
    }
    write(
        &output.join("manifest.json"),
        &json!({"status":if partial {"partial"} else {"complete"},"root":root,"variants":manifest}),
    )?;
    Ok(partial)
}
fn write(path: &Path, v: &Value) -> Result<(), String> {
    std::fs::write(path, serde_json::to_string_pretty(v).unwrap() + "\n").map_err(|e| e.to_string())
}
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        eprintln!(
            "uso: lente-adaptador-c-experimental COMPILE_COMMANDS_JSON PROJECT_ROOT OUTPUT_DIR"
        );
        std::process::exit(1);
    }
    match run(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
    ) {
        Ok(false) => {}
        Ok(true) => std::process::exit(2),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/fixtures")
                .join(format!(
                    "{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
            std::fs::create_dir_all(&p).unwrap();
            Self(p.canonicalize().unwrap())
        }
        fn config(&self, file: &str, extra: &[&str]) -> Config {
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .canonicalize()
                .unwrap();
            let file = fixture.join(file).to_string_lossy().into_owned();
            let mut args = vec![
                "clang".into(),
                "-std=c11".into(),
                "-I".into(),
                fixture.join("include").to_string_lossy().into_owned(),
                "-isystem".into(),
                fixture.join("system").to_string_lossy().into_owned(),
            ];
            args.extend(extra.iter().map(|s| s.to_string()));
            args.extend(["-c".into(), file.clone(), "-o".into(), "unused.o".into()]);
            Config {
                directory: self.0.to_string_lossy().into_owned(),
                file,
                arguments: args,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .canonicalize()
            .unwrap()
    }
    fn edge(g: &Value, from: &str, to: &str) -> bool {
        g["edges"].as_array().unwrap().iter().any(|e| {
            e["from"].as_str().unwrap().ends_with(from) && e["to"].as_str().unwrap().ends_with(to)
        })
    }
    #[test]
    fn local_i_system_conditional_transitive_and_positions() {
        let f = Fixture::new();
        let (g, m) = extract(&f.config("main.c", &["-DFEATURE=1"]), &root());
        assert_eq!(m["status"], "complete", "{m:#}");
        for target in ["local.h", "via_path.h", "system_header.h", "enabled.h"] {
            assert!(edge(&g, "main.c", target), "{g:#}");
        }
        assert!(!edge(&g, "main.c", "disabled.h"));
        assert!(edge(&g, "local.h", "leaf.h"));
        assert!(!edge(&g, "main.c", "leaf.h"));
        let system = m["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["path"].as_str().unwrap().ends_with("system_header.h"))
            .unwrap();
        assert_eq!(system["class"], "system");
        let occurrence = m["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| {
                e["from"].as_str().unwrap().ends_with("main.c")
                    && e["to"].as_str().unwrap().ends_with("local.h")
            })
            .unwrap();
        assert_eq!(occurrence["position"]["line"], 2);
        assert!(occurrence["position"]["column"].as_u64().unwrap() > 0);
        assert!(!f.0.join("unused.o").exists());
    }
    #[test]
    fn cwd_relative_include_and_macro_include() {
        let f = Fixture::new();
        std::fs::create_dir(f.0.join("inc")).unwrap();
        std::fs::write(f.0.join("inc/x.h"), "#define X 1\n").unwrap();
        std::fs::write(
            f.0.join("test.c"),
            "#define HEADER <x.h>\n#include HEADER\nint x = X;\n",
        )
        .unwrap();
        let c = Config {
            directory: f.0.display().to_string(),
            file: "test.c".into(),
            arguments: vec!["clang", "-Iinc", "-std=c99", "-c", "test.c"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        };
        let before = std::env::current_dir().unwrap();
        let (g, m) = extract(&c, &f.0);
        assert_eq!(m["status"], "complete", "{m:#}");
        assert!(edge(&g, "test.c", "x.h"));
        assert_eq!(before, std::env::current_dir().unwrap());
    }
    #[test]
    fn missing_header_and_syntax_error_are_partial() {
        let f = Fixture::new();
        for file in ["missing.c", "syntax.c"] {
            let (g, m) = extract(&f.config(file, &[]), &root());
            assert_eq!(m["status"], "partial");
            assert_eq!(m["unanalyzed_units"].as_array().unwrap().len(), 1);
            assert!(
                m["diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["severity"].as_u64().unwrap() >= 3)
            );
            assert!(g["nodes"].as_array().unwrap().is_empty());
            lente_infra::desserializar_grafo(&g.to_string()).unwrap();
        }
    }
    #[test]
    fn configurations_are_separate_and_repeated_extraction_is_identical() {
        let f = Fixture::new();
        let a = f.config("main.c", &["-DFEATURE"]);
        let b = f.config("main.c", &[]);
        write(
            &f.0.join("compile_commands.json"),
            &json!([a.json(), b.json()]),
        )
        .unwrap();
        let output = f.0.join("out");
        let second = f.0.join("repeat");
        assert!(!run(&f.0.join("compile_commands.json"), &root(), &output).unwrap());
        assert!(!run(&f.0.join("compile_commands.json"), &root(), &second).unwrap());
        let mut configs = Vec::new();
        for i in 0..2 {
            let name = format!("graph-{i:04}.json");
            let bytes = std::fs::read(output.join(&name)).unwrap();
            assert_eq!(bytes, std::fs::read(second.join(&name)).unwrap());
            let graph: Value = serde_json::from_slice(&bytes).unwrap();
            assert_ne!(
                edge(&graph, "main.c", "enabled.h"),
                edge(&graph, "main.c", "disabled.h")
            );
            configs.push(edge(&graph, "main.c", "enabled.h"));
            let meta = format!("metadata-{i:04}.json");
            assert_eq!(
                std::fs::read(output.join(&meta)).unwrap(),
                std::fs::read(second.join(&meta)).unwrap()
            );
        }
        assert_ne!(configs[0], configs[1]);
    }
    #[test]
    fn graph_is_accepted_by_lente_and_transitive_impact_is_correct() {
        let f = Fixture::new();
        let (g, m) = extract(&f.config("main.c", &[]), &root());
        assert_eq!(m["status"], "complete", "{m:#}");
        let grafo = lente_infra::desserializar_grafo(&g.to_string()).unwrap();
        assert!(
            grafo
                .nodes
                .iter()
                .all(|n| n.kind == lente_core::entities::grafo::Kind::Mod)
        );
        let leaf = grafo
            .nodes
            .iter()
            .find(|n| n.path.as_str().ends_with("leaf.h"))
            .unwrap();
        let raio = lente_core::domain::raio::calcular_raio(&grafo, &leaf.path).unwrap();
        assert_eq!(raio.montante.len(), 2); // local.h e main.c, sem atalho sintético.
        for e in &grafo.edges {
            assert_eq!(
                grafo.nodes.iter().find(|n| n.id == e.id_from).unwrap().path,
                e.from
            );
            assert_eq!(
                grafo.nodes.iter().find(|n| n.id == e.id_to).unwrap().path,
                e.to
            );
        }
    }
    #[test]
    fn external_headers_are_kept_and_classified() {
        let f = Fixture::new();
        let (g, m) = extract(&f.config("main.c", &[]), &f.0);
        assert_eq!(m["status"], "complete", "{m:#}");
        assert!(edge(&g, "main.c", "local.h"));
        assert!(
            m["files"].as_array().unwrap().iter().any(|n| n["path"]
                .as_str()
                .unwrap()
                .ends_with("local.h")
                && n["class"] == "external")
        );
    }
    #[test]
    fn command_is_tokenized_but_never_executed() {
        let f = Fixture::new();
        let c = f.config("main.c", &[]);
        let quoted = c
            .arguments
            .iter()
            .map(|s| format!("\"{}\"", s.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(" ");
        write(
            &f.0.join("compile_commands.json"),
            &json!([{"directory":c.directory,"file":c.file,"command":quoted}]),
        )
        .unwrap();
        let configs = clang::database(&f.0).unwrap();
        assert_eq!(configs.len(), 1);
        let (_, m) = extract(&configs[0], &root());
        assert_eq!(m["status"], "complete", "{m:#}");
        // Mesmo command contendo sintaxe de shell continua sendo apenas dados.
        let shell_text = format!("{quoted} ; touch SHOULD_NOT_EXIST");
        write(
            &f.0.join("compile_commands.json"),
            &json!([{
                "directory":c.directory,"file":c.file,"command":shell_text
            }]),
        )
        .unwrap();
        for config in clang::database(&f.0).unwrap() {
            let (_, metadata) = extract(&config, &root());
            assert_eq!(metadata["status"], "partial");
        }
        assert!(!f.0.join("SHOULD_NOT_EXIST").exists());
        // Um shell ou wrapper como driver é recusado antes de chegar ao parser.
        let mut malicious = c;
        malicious.arguments = vec!["sh".into(), "-c".into(), "touch SHOULD_NOT_EXIST".into()];
        let (_, m) = extract(&malicious, &root());
        assert_eq!(m["status"], "partial");
        assert!(!f.0.join("SHOULD_NOT_EXIST").exists());
    }
    #[test]
    fn unsupported_options_never_silently_change_configuration() {
        let f = Fixture::new();
        for args in [
            &["-Xclang", "-load", "plugin.so"][..],
            &["@flags.rsp"],
            &["-include", "forced.h"],
            &["-x", "c++"],
            &["-fmodules"],
            &["--unknown"],
        ] {
            let (_, m) = extract(&f.config("main.c", args), &root());
            assert_eq!(m["status"], "partial", "{args:?}");
        }
    }
    #[test]
    fn real_system_header_and_repeated_guarded_include() {
        let f = Fixture::new();
        std::fs::write(
            f.0.join("guard.h"),
            "#ifndef GUARD_H\n#define GUARD_H\n#endif\n",
        )
        .unwrap();
        std::fs::write(
            f.0.join("test.c"),
            "#include <stddef.h>\n#include \"guard.h\"\n#include \"guard.h\"\nsize_t n;\n",
        )
        .unwrap();
        let c = Config {
            directory: f.0.display().to_string(),
            file: "test.c".into(),
            arguments: vec!["clang", "-c", "test.c"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        };
        let (g, m) = extract(&c, &f.0);
        assert_eq!(m["status"], "complete", "{m:#}");
        assert!(edge(&g, "test.c", "stddef.h"));
        assert!(
            m["files"].as_array().unwrap().iter().any(|n| n["path"]
                .as_str()
                .unwrap()
                .ends_with("stddef.h")
                && n["class"] == "system")
        );
        let guards = m["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["to"].as_str().unwrap().ends_with("guard.h"))
            .count();
        assert_eq!(guards, 2); // Duas diretivas ativas, só uma aresta deduplicada.
        assert_eq!(
            g["edges"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|n| n["to"].as_str().unwrap().ends_with("guard.h"))
                .count(),
            1
        );
    }
    #[test]
    fn manifest_identifies_partial_and_complete_units() {
        let f = Fixture::new();
        write(
            &f.0.join("compile_commands.json"),
            &json!([
                f.config("main.c", &[]).json(),
                f.config("missing.c", &[]).json()
            ]),
        )
        .unwrap();
        let output = f.0.join("reports");
        assert!(run(&f.0.join("compile_commands.json"), &root(), &output).unwrap());
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(output.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["status"], "partial");
        let statuses: Vec<_> = manifest["variants"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["status"].as_str().unwrap())
            .collect();
        assert_eq!(statuses, vec!["complete", "partial"]);
        for v in manifest["variants"].as_array().unwrap() {
            let graph = std::fs::read_to_string(output.join(v["graph"].as_str().unwrap())).unwrap();
            lente_infra::desserializar_grafo(&graph).unwrap();
        }
        assert!(run(&f.0.join("compile_commands.json"), &root(), &output).is_err());
    }
}
