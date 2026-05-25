/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/cli.md
 * @prompt 00_nucleo/prompts/cli_output_flags.md
 * @layer L2
 * @updated 2026-05-20
 */

use std::path::Path;

/// Formata a mensagem de início de análise do workspace.
pub fn format_start_analysis(workspace_path: &str) -> String {
    format!("Iniciando análise do workspace em: {}...", workspace_path)
}

/// Formata o resumo do pipeline com contagens e caminhos dos ficheiros gerados.
#[allow(clippy::too_many_arguments)]
pub fn format_summary(
    members: usize,
    modules: usize,
    edges: usize,
    cycles: usize,
    output_path: &Path,
    trees_path: Option<&Path>,
    html_path: Option<&Path>,
    layer_violations: usize,
    sarif_findings: usize,
) -> String {
    let mut out = String::new();
    out.push_str("=== Crystalline DSM ===\n");
    out.push_str(&format!("Crates: {}\n", members));
    out.push_str(&format!("Módulos: {}\n", modules));
    out.push_str(&format!("Arestas: {}\n", edges));
    out.push_str(&format!("Ciclos: {}\n", cycles));
    out.push_str(&format!("\nGrafo gravado em: {}", output_path.display()));
    if let Some(trees_path) = trees_path {
        out.push_str(&format!("\nTrees gravadas em: {}", trees_path.display()));
    }
    if let Some(html_path) = html_path {
        out.push_str(&format!("\nHTML gravado em: {}", html_path.display()));
    }
    if layer_violations > 0 {
        out.push_str(&format!(
            "\n⚠ Violações de camada detectadas: {}\n",
            layer_violations
        ));
    }
    if sarif_findings > 0 {
        if layer_violations == 0 {
            out.push('\n');
        }
        out.push_str(&format!("Findings do linter (SARIF): {}\n", sarif_findings));
    }
    out
}

/// Formata uma mensagem de erro para o utilizador.
pub fn format_error(title: &str, detail: &str) -> String {
    format!("Erro: {} — {}", title, detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_start_analysis() {
        let msg = format_start_analysis("/some/path");
        assert!(msg.contains("/some/path"));
        assert!(msg.contains("Iniciando análise"));
    }

    #[test]
    fn test_format_summary_without_extras() {
        let out = PathBuf::from("./graph.json");
        let s = format_summary(3, 12, 8, 1, &out, None, None, 0, 0);
        assert!(s.contains("Crates: 3"));
        assert!(s.contains("Módulos: 12"));
        assert!(s.contains("Arestas: 8"));
        assert!(s.contains("Ciclos: 1"));
        assert!(s.contains("./graph.json"));
        assert!(!s.contains("Trees gravadas"));
        assert!(!s.contains("HTML gravado"));
        assert!(!s.contains("Violações de camada"));
    }

    #[test]
    fn test_format_summary_with_trees() {
        let out = PathBuf::from("./graph.json");
        let trees = PathBuf::from("./trees.json");
        let s = format_summary(1, 1, 0, 0, &out, Some(&trees), None, 0, 0);
        assert!(s.contains("./trees.json"));
        assert!(s.contains("Trees gravadas"));
        assert!(!s.contains("HTML gravado"));
    }

    #[test]
    fn test_format_summary_with_html() {
        let out = PathBuf::from("./graph.json");
        let html = PathBuf::from("./dsm.html");
        let s = format_summary(1, 1, 0, 0, &out, None, Some(&html), 0, 0);
        assert!(s.contains("./dsm.html"));
        assert!(s.contains("HTML gravado"));
    }

    #[test]
    fn test_format_summary_with_violations_and_findings() {
        let out = PathBuf::from("./graph.json");
        let html = PathBuf::from("./dsm.html");
        let s = format_summary(1, 1, 0, 0, &out, None, Some(&html), 2, 5);
        assert!(s.contains("Violações de camada detectadas: 2"));
        assert!(s.contains("Findings do linter (SARIF): 5"));
    }

    #[test]
    fn test_format_error() {
        let s = format_error("Falha", "detalhe");
        assert!(s.contains("Falha"));
        assert!(s.contains("detalhe"));
    }
}
