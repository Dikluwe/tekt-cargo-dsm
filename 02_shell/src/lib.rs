/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/cli.md
 * @layer L2
 * @updated 2026-05-20
 */

/// Formata a mensagem de início de análise do workspace.
pub fn format_start_analysis(workspace_path: &str) -> String {
    format!("Iniciando análise do workspace em: {}...", workspace_path)
}

/// Formata a mensagem de sucesso na geração do relatório.
pub fn format_success(output_path: &str) -> String {
    format!("Análise concluída com sucesso! Relatório gerado em: {}", output_path)
}
