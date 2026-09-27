//! Crystalline Lineage
//! @prompt lab/regras-arquiteturais/CONTRATO.md
mod files;
use regras_arquiteturais::{Date, evaluate};
use std::{collections::BTreeMap, env, path::Path};
fn run() -> Result<i32, String> {
    let mut args = env::args().skip(1);
    let mut options = BTreeMap::new();
    while let Some(key) = args.next() {
        if key == "--help" {
            println!(
                "regras-arquiteturais --baseline SNAPSHOT.json --current SNAPSHOT.json --rules REGRAS.json --as-of YYYY-MM-DD [--format json|text]\nExit 0: aprovado; 1: violação; 2: inconclusivo; 3: erro de entrada."
            );
            return Ok(0);
        }
        if !["--baseline", "--current", "--rules", "--as-of", "--format"].contains(&key.as_str()) {
            return Err(format!("Opção desconhecida: {key}"));
        }
        let value = args
            .next()
            .ok_or_else(|| format!("Falta valor para {key}"))?;
        if options.insert(key, value).is_some() {
            return Err("Opção repetida".into());
        }
    }
    let required = |k: &str| {
        options
            .get(k)
            .ok_or_else(|| format!("Opção obrigatória: {k}"))
    };
    let as_of = required("--as-of")?;
    let date = Date::parse(as_of)?;
    let format = options
        .get("--format")
        .map(String::as_str)
        .unwrap_or("json");
    if !["json", "text"].contains(&format) {
        return Err("Formato deve ser json ou text".into());
    }
    let (baseline, b) = files::load_snapshot(Path::new(required("--baseline")?))?;
    let (current, c) = files::load_snapshot(Path::new(required("--current")?))?;
    let (rules, r) = files::load_rules(Path::new(required("--rules")?))?;
    let report = evaluate(&baseline, &current, &rules, date)?;
    let output = files::report_json(&report, as_of, b, c, r);
    let rendered = if format == "json" {
        format!(
            "{}\n",
            serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?
        )
    } else {
        files::text_report(&output)
    };
    use std::io::Write;
    std::io::stdout()
        .lock()
        .write_all(rendered.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(report.status.exit_code())
}
fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("Erro de entrada: {e}");
            std::process::exit(3);
        }
    }
}
