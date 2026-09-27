//! Crystalline Lineage
//! @prompt lab/doxygen-correspondencia/CONTRATO.md
mod publicacao;
use std::{collections::BTreeMap, env, ffi::OsString, fs, path::PathBuf};
fn run() -> Result<(), String> {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    if args == [OsString::from("--help")] {
        println!(
            "lente-doxygen-correspondencia --inventory INVENTARIO.json --catalog CATALOGO.json --output CORRESPONDENCIAS.json"
        );
        return Ok(());
    }
    let mut options = BTreeMap::new();
    if args.len() != 6 {
        return Err("arguments: informe --inventory, --catalog e --output uma vez cada".into());
    }
    for pair in args.chunks_exact(2) {
        let key = pair[0].to_str().ok_or("arguments: opção inválida")?;
        if !["--inventory", "--catalog", "--output"].contains(&key)
            || options.insert(key, PathBuf::from(&pair[1])).is_some()
        {
            return Err(format!("arguments: opção desconhecida ou repetida {key}"));
        }
    }
    let inv = fs::read(&options["--inventory"]).map_err(|e| format!("inventory_io: {e}"))?;
    let cat = fs::read(&options["--catalog"]).map_err(|e| format!("catalog_io: {e}"))?;
    let result = lente_doxygen_correspondencia::associate(&inv, &cat)?;
    let mut bytes = serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    publicacao::publicar(&options["--output"], &bytes)?;
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
