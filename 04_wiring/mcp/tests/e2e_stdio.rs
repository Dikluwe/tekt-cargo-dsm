//! E2E por stdio da boca MCP (prompt 0070, convenção 0017/0037): sobe o binário
//! `lente-mcp` real e exercita o ciclo `initialize` → `notifications/initialized`
//! → `tools/call impacto_do_diff` contra o próprio repo, conferindo que a
//! resposta carrega o JSON do `ResultadoDiff` (contrato 0047). Exige o fork
//! `cargo-modules` + git — por isso `#[ignore]`, como os demais E2E.

use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[ignore]
fn e2e_stdio_initialize_e_call_diff() {
    // CARGO_MANIFEST_DIR = 04_wiring/mcp → sobe duas vezes p/ a raiz do workspace.
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("raiz do workspace")
        .to_path_buf();

    let mut filho = Command::new(env!("CARGO_BIN_EXE_lente-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("subir lente-mcp");

    let entrada = format!(
        "{}\n{}\n{}\n",
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"impacto_do_diff","arguments":{"raiz": raiz.to_str().unwrap()}}}),
    );
    filho
        .stdin
        .take()
        .unwrap()
        .write_all(entrada.as_bytes())
        .unwrap();

    let saida = filho.wait_with_output().expect("colher saída");
    let texto = String::from_utf8_lossy(&saida.stdout);
    let linhas: Vec<&str> = texto.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(linhas.len(), 2, "esperava 2 respostas (initialize, call): {}", texto);

    // Resposta 1: initialize ecoa a versão pedida.
    let r1: Value = serde_json::from_str(linhas[0]).unwrap();
    assert_eq!(r1["id"], 1);
    assert_eq!(r1["result"]["protocolVersion"], "2025-06-18");

    // A notificação initialized NÃO gera resposta (só 2 linhas no total).

    // Resposta 2: tools/call do diff carrega o MESMO JSON do `lente --diff` (0047).
    let r2: Value = serde_json::from_str(linhas[1]).unwrap();
    assert_eq!(r2["id"], 2);
    assert_eq!(r2["result"]["isError"], false);
    let payload = r2["result"]["content"][0]["text"].as_str().unwrap();
    let diff: Value = serde_json::from_str(payload).expect("o conteúdo é JSON do diff");
    assert!(
        diff.get("combinado").is_some() || diff.get("tocados").is_some(),
        "JSON do diff sem os campos do contrato 0047: {}",
        payload
    );
}
