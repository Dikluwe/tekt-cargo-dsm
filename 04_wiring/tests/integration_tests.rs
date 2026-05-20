use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_cli_version() {
    // Obtém o caminho do binário compilado pela macro interna do Cargo
    let bin_path = env!("CARGO_BIN_EXE_crystalline-dsm-cli");

    let output = Command::new(bin_path)
        .arg("--version")
        .output()
        .expect("Falha ao rodar o comando da CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("crystalline-dsm"));
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_cli_empty_workspace_default_html() {
    let bin_path = env!("CARGO_BIN_EXE_crystalline-dsm-cli");
    let fixture_path = "tests/fixtures/empty-workspace";
    let default_output = Path::new("dsm.html");

    // Remove arquivo de teste residual se existir
    if default_output.exists() {
        fs::remove_file(default_output).ok();
    }

    let output = Command::new(bin_path)
        .arg(fixture_path)
        .output()
        .expect("Falha ao rodar o comando da CLI");

    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Iniciando análise do workspace em"));
    assert!(stdout.contains("Análise concluída com sucesso!"));

    // Verifica se o arquivo HTML padrão foi criado fisicamente no disco
    assert!(default_output.exists());
    let content = fs::read_to_string(default_output).unwrap();
    assert!(content.contains("<html>"));

    // Limpeza
    fs::remove_file(default_output).ok();
}

#[test]
fn test_cli_empty_workspace_json_output() {
    let bin_path = env!("CARGO_BIN_EXE_crystalline-dsm-cli");
    let fixture_path = "tests/fixtures/empty-workspace";
    let custom_output = Path::new("tests/fixtures/output.json");

    // Remove arquivo de teste residual se existir
    if custom_output.exists() {
        fs::remove_file(custom_output).ok();
    }

    let output = Command::new(bin_path)
        .arg(fixture_path)
        .arg("-o")
        .arg(custom_output)
        .arg("-f")
        .arg("json")
        .output()
        .expect("Falha ao rodar o comando da CLI");

    assert!(output.status.success());

    // Verifica se o arquivo JSON customizado foi criado fisicamente no disco
    assert!(custom_output.exists());
    let content = fs::read_to_string(custom_output).unwrap();
    assert!(content.contains("{\"modules\":[],\"edges\":[]}"));

    // Limpeza
    fs::remove_file(custom_output).ok();
}
