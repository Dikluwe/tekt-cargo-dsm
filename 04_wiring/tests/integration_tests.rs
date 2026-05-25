use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap();
    workspace_root.join("tests").join("fixtures").join(name)
}

/// Cria um directorio temporario unico para este teste e retorna o path.
fn tmpdir(label: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("crystalline-dsm-{}-{}-{}", label, pid, nanos));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_crystalline-dsm-cli")
}

#[test]
fn test_cli_version() {
    let output = Command::new(bin_path())
        .arg("--version")
        .output()
        .expect("Falha ao rodar o comando da CLI");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("crystalline-dsm"));
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_cli_workspace_nao_encontrado() {
    let output = Command::new(bin_path())
        .arg("/caminho/que/nao/existe/__crystalline_test__")
        .output()
        .expect("Falha ao rodar a CLI");
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Workspace nao encontrado"));
}

#[test]
fn test_cli_imports_simple_sem_emit_trees() {
    let dir = tmpdir("simple");
    let output_path = dir.join("graph.json");
    let trees_path = dir.join("trees.json");

    let status = Command::new(bin_path())
        .arg(fixture("imports-simple"))
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Falha ao rodar a CLI");
    assert!(
        status.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&status.stderr)
    );

    assert!(output_path.exists(), "graph.json deveria existir");
    assert!(
        !trees_path.exists(),
        "trees.json NAO deveria existir sem --emit-trees"
    );

    // Verificar conteudo JSON parseavel
    let content = fs::read_to_string(&output_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&content).expect("JSON valido");
    assert_eq!(v["schema_version"], "1.0.0");
    assert_eq!(v["tool"]["name"], "crystalline-dsm");
    assert!(!v["graph"]["nodes"].as_array().unwrap().is_empty());

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_imports_simple_com_emit_html() {
    let dir = tmpdir("simple-html");
    let output_path = dir.join("graph.json");
    let html_path = dir.join("dsm.html");

    let status = Command::new(bin_path())
        .arg(fixture("imports-simple"))
        .arg("--output")
        .arg(&output_path)
        .arg("--emit-html")
        .output()
        .expect("Falha ao rodar a CLI");
    assert!(
        status.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&status.stderr)
    );

    assert!(output_path.exists());
    assert!(
        html_path.exists(),
        "dsm.html deveria existir com --emit-html"
    );

    let html = fs::read_to_string(&html_path).unwrap();
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<canvas"));
    assert!(html.contains("popover=\"manual\""));

    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("HTML gravado"));

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_imports_workspace_com_emit_trees() {
    let dir = tmpdir("ws-emit-trees");
    let output_path = dir.join("graph.json");
    let trees_path = dir.join("trees.json");

    let status = Command::new(bin_path())
        .arg(fixture("imports-workspace"))
        .arg("--output")
        .arg(&output_path)
        .arg("--emit-trees")
        .output()
        .expect("Falha ao rodar a CLI");
    assert!(
        status.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&status.stderr)
    );

    assert!(output_path.exists());
    assert!(
        trees_path.exists(),
        "trees.json deveria existir com --emit-trees"
    );

    let trees_v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&trees_path).unwrap()).unwrap();
    assert_eq!(trees_v["schema_version"], "1.0.0");
    let trees_arr = trees_v["trees"].as_array().unwrap();
    assert_eq!(trees_arr.len(), 2, "esperava 2 arvores (crate_a + crate_b)");

    // Resumo no stdout menciona ambos os ficheiros
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("graph.json"));
    assert!(stdout.contains("trees.json"));

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_output_path_absoluto_em_subdiretorio_inexistente() {
    let dir = tmpdir("absolute-nested");
    // Sub-directório que NÃO existe — a CLI deve criar
    let output_path = dir.join("subdir").join("out").join("graph.json");
    assert!(!output_path.exists());
    assert!(!output_path.parent().unwrap().exists());

    let status = Command::new(bin_path())
        .arg(fixture("imports-simple"))
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Falha ao rodar a CLI");
    assert!(
        status.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert!(output_path.exists());

    fs::remove_dir_all(&dir).ok();
}

/// Smoke test contra Typst real — paralelo ao typst_smoke_test.rs.
/// Requer `TYPST_PATH` apontando para um workspace Cargo válido.
#[test]
#[ignore = "requer TYPST_PATH apontando para lab/typst-original/"]
fn test_cli_typst_real_com_emit_trees() {
    let typst_path = std::env::var("TYPST_PATH").expect("TYPST_PATH nao definida");
    let typst_path = Path::new(&typst_path);
    assert!(
        typst_path.exists(),
        "TYPST_PATH inexistente: {:?}",
        typst_path
    );

    let dir = tmpdir("typst");
    let output_path = dir.join("graph.json");
    let trees_path = dir.join("trees.json");

    let status = Command::new(bin_path())
        .arg(typst_path)
        .arg("--output")
        .arg(&output_path)
        .arg("--emit-trees")
        .output()
        .expect("Falha ao rodar a CLI");
    assert!(
        status.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&status.stderr)
    );

    let graph: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output_path).unwrap()).unwrap();
    let trees: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&trees_path).unwrap()).unwrap();

    assert_eq!(graph["workspace"]["members"].as_array().unwrap().len(), 21);
    assert_eq!(trees["trees"].as_array().unwrap().len(), 21);

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_imports_simple_sem_emit_html() {
    let dir = tmpdir("simple-sem-html");
    let output_path = dir.join("graph.json");
    let html_path = dir.join("dsm.html");

    let status = Command::new(bin_path())
        .arg(fixture("imports-simple"))
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Falha ao rodar a CLI");
    assert!(status.status.success());

    assert!(output_path.exists());
    assert!(
        !html_path.exists(),
        "dsm.html nao deveria existir sem --emit-html"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_cli_imports_simple_com_html_e_trees() {
    let dir = tmpdir("simple-html-trees");
    let output_path = dir.join("graph.json");
    let html_path = dir.join("dsm.html");
    let trees_path = dir.join("trees.json");

    let status = Command::new(bin_path())
        .arg(fixture("imports-simple"))
        .arg("--output")
        .arg(&output_path)
        .arg("--emit-html")
        .arg("--emit-trees")
        .output()
        .expect("Falha ao rodar a CLI");
    assert!(status.status.success());

    assert!(output_path.exists());
    assert!(html_path.exists());
    assert!(trees_path.exists());

    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(stdout.contains("graph.json"));
    assert!(stdout.contains("trees.json"));
    assert!(stdout.contains("HTML gravado"));

    fs::remove_dir_all(&dir).ok();
}

#[test]
#[ignore = "requer TYPST_PATH apontando para lab/typst-original/"]
fn test_cli_typst_real_com_emit_html() {
    let typst_path = std::env::var("TYPST_PATH").expect("TYPST_PATH nao definida");
    let typst_path = Path::new(&typst_path);
    assert!(
        typst_path.exists(),
        "TYPST_PATH inexistente: {:?}",
        typst_path
    );

    let dir = tmpdir("typst-html");
    let output_path = dir.join("graph.json");
    let html_path = dir.join("dsm.html");
    let trees_path = dir.join("trees.json");

    let status = Command::new(bin_path())
        .arg(typst_path)
        .arg("--output")
        .arg(&output_path)
        .arg("--emit-html")
        .arg("--emit-trees")
        .output()
        .expect("Falha ao rodar a CLI");
    assert!(status.status.success());

    assert!(output_path.exists());
    assert!(html_path.exists());
    assert!(trees_path.exists());

    let metadata = fs::metadata(&html_path).unwrap();
    let size = metadata.len();
    assert!(size >= 100 * 1024, "HTML muito pequeno: {} bytes", size);
    assert!(size <= 5 * 1024 * 1024, "HTML muito grande: {} bytes", size);

    fs::remove_dir_all(&dir).ok();
}
