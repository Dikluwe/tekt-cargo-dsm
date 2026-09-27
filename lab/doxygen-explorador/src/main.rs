//! Crystalline Lineage — @prompt ../CONTRATO.md
mod publicacao;
use std::{collections::BTreeMap, net::TcpListener};
fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut opts = BTreeMap::new();
    while let Some(arg) = args.next() {
        if arg == "--help" {
            println!(
                "doxygen-explorador --inventory FILE --catalog FILE --matches FILE (--port PORT | --output FILE)\n--output prepara JSON e encerra; não inicia servidor. Encerre com Ctrl-C."
            );
            return Ok(());
        }
        if ![
            "--inventory",
            "--catalog",
            "--matches",
            "--port",
            "--output",
        ]
        .contains(&arg.as_str())
        {
            return Err(format!("opção desconhecida: {arg}"));
        }
        let value = args.next().ok_or_else(|| format!("valor ausente: {arg}"))?;
        if opts.insert(arg, value).is_some() {
            return Err("opção duplicada".into());
        }
    }
    for key in ["--inventory", "--catalog", "--matches"] {
        if !opts.contains_key(key) {
            return Err(format!("opção obrigatória: {key}"));
        }
    }
    if opts.contains_key("--port") == opts.contains_key("--output") {
        return Err("informe exatamente --port OU --output".into());
    }
    let read = |key| std::fs::read(&opts[key]).map_err(|e| format!("{}: {e}", opts[key]));
    let data = doxygen_explorador::prepare(
        &read("--inventory")?,
        &read("--catalog")?,
        &read("--matches")?,
    )?;
    let body = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    if let Some(output) = opts.get("--output") {
        return publicacao::publicar(std::path::Path::new(output), &body);
    }
    let port: u16 = opts["--port"].parse().map_err(|_| "porta inválida")?;
    if port == 0 {
        return Err("porta inválida".into());
    }
    let listener =
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).map_err(|e| e.to_string())?;
    println!(
        "Explorador: http://127.0.0.1:{port} — Ctrl-C para encerrar. Vínculos verificados; cobertura declarada, sem prova semântica."
    );
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                if let Err(e) = doxygen_explorador::serve_connection(s, &body) {
                    eprintln!("HTTP: {e}");
                }
            }
            Err(e) => eprintln!("conexão: {e}"),
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Erro: {e}");
        std::process::exit(2);
    }
}
