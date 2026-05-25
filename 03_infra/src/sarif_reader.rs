/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/config_sarif_readers.md
 * @layer L3
 * @updated 2026-05-25
 */

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SarifFinding {
    /// Identificador da regra (ex: "V9").
    pub rule_id: String,

    /// Severidade ("error", "warning", "note", "none").
    pub level: String,

    /// Mensagem descritiva.
    pub message: String,

    /// Caminho do ficheiro afetado (uri do artifactLocation),
    /// como string, relativo à raiz do projeto.
    pub file_uri: String,

    /// Linha inicial, se presente.
    pub start_line: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum SarifReadError {
    #[error("Ficheiro SARIF não encontrado: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Falha de I/O ao ler SARIF: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("Falha ao parsear SARIF JSON: {source}")]
    ParseFailed {
        #[from]
        source: serde_json::Error,
    },

    #[error("Versão SARIF não suportada: {version} (esperado 2.1.0)")]
    UnsupportedVersion { version: String },
}

#[derive(Deserialize)]
struct SarifDoc {
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Deserialize)]
struct SarifRun {
    #[serde(default)]
    results: Vec<SarifResult>,
}

fn default_level() -> String {
    "warning".to_string()
}

#[derive(Deserialize)]
struct SarifResult {
    #[serde(rename = "ruleId", default)]
    rule_id: String,
    #[serde(default = "default_level")]
    level: String,
    message: SarifMessage,
    #[serde(default)]
    locations: Vec<SarifLocation>,
}

#[derive(Deserialize)]
struct SarifMessage {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: Option<SarifPhysicalLocation>,
}

#[derive(Deserialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: Option<SarifArtifactLocation>,
    region: Option<SarifRegion>,
}

#[derive(Deserialize)]
struct SarifArtifactLocation {
    uri: Option<String>,
}

#[derive(Deserialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: Option<u32>,
}

/// Lê o arquivo SARIF e extrai os findings.
pub fn read_sarif(sarif_path: &Path) -> Result<Vec<SarifFinding>, SarifReadError> {
    if !sarif_path.exists() {
        return Err(SarifReadError::FileNotFound {
            path: sarif_path.to_path_buf(),
        });
    }

    let content = fs::read_to_string(sarif_path)?;
    let doc: SarifDoc = serde_json::from_str(&content)?;

    if doc.version != "2.1.0" {
        return Err(SarifReadError::UnsupportedVersion {
            version: doc.version,
        });
    }

    let mut findings = Vec::new();
    for run in doc.runs {
        for result in run.results {
            let mut file_uri = String::new();
            let mut start_line = None;

            if let Some(phys) = result
                .locations
                .first()
                .and_then(|loc| loc.physical_location.as_ref())
            {
                if let Some(uri) = phys.artifact_location.as_ref().and_then(|a| a.uri.as_ref()) {
                    file_uri = uri.clone();
                }
                if let Some(reg) = &phys.region {
                    start_line = reg.start_line;
                }
            }

            findings.push(SarifFinding {
                rule_id: result.rule_id,
                level: result.level,
                message: result.message.text,
                file_uri,
                start_line,
            });
        }
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file(name: &str, content: &str) -> PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("test-{}-{}-{}", name, pid, nanos));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_read_sarif_valid_single_finding() {
        let json = r#"{
            "version": "2.1.0",
            "runs": [
                {
                    "tool": { "driver": { "name": "crystalline-lint" } },
                    "results": [
                        {
                            "ruleId": "V9",
                            "level": "error",
                            "message": { "text": "Public leakage detected" },
                            "locations": [
                                {
                                    "physicalLocation": {
                                        "artifactLocation": { "uri": "01_core/src/x.rs" },
                                        "region": { "startLine": 12 }
                                    }
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let file = tmp_file("sarif_single", json);
        let findings = read_sarif(&file).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "V9");
        assert_eq!(findings[0].level, "error");
        assert_eq!(findings[0].message, "Public leakage detected");
        assert_eq!(findings[0].file_uri, "01_core/src/x.rs");
        assert_eq!(findings[0].start_line, Some(12));
        fs::remove_file(file).ok();
    }

    #[test]
    fn test_read_sarif_multiple_findings() {
        let json = r#"{
            "version": "2.1.0",
            "runs": [
                {
                    "results": [
                        {
                            "ruleId": "V9",
                            "message": { "text": "leakage 1" },
                            "locations": [{ "physicalLocation": { "artifactLocation": { "uri": "a.rs" } } }]
                        },
                        {
                            "ruleId": "V11",
                            "message": { "text": "dangling 2" },
                            "locations": [{ "physicalLocation": { "artifactLocation": { "uri": "b.rs" } } }]
                        }
                    ]
                }
            ]
        }"#;
        let file = tmp_file("sarif_multi", json);
        let findings = read_sarif(&file).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule_id, "V9");
        assert_eq!(findings[0].file_uri, "a.rs");
        assert_eq!(findings[1].rule_id, "V11");
        assert_eq!(findings[1].file_uri, "b.rs");
        fs::remove_file(file).ok();
    }

    #[test]
    fn test_read_sarif_multiple_runs() {
        let json = r#"{
            "version": "2.1.0",
            "runs": [
                {
                    "results": [
                        {
                            "ruleId": "V1",
                            "message": { "text": "msg1" }
                        }
                    ]
                },
                {
                    "results": [
                        {
                            "ruleId": "V2",
                            "message": { "text": "msg2" }
                        }
                    ]
                }
            ]
        }"#;
        let file = tmp_file("sarif_runs", json);
        let findings = read_sarif(&file).unwrap();
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule_id, "V1");
        assert_eq!(findings[1].rule_id, "V2");
        fs::remove_file(file).ok();
    }

    #[test]
    fn test_file_not_found() {
        let path = PathBuf::from("/non/existent/path/report.sarif");
        let res = read_sarif(&path);
        assert!(matches!(res, Err(SarifReadError::FileNotFound { .. })));
    }

    #[test]
    fn test_malformed_json() {
        let file = tmp_file("sarif_malformed", "{ malformed json");
        let res = read_sarif(&file);
        assert!(matches!(res, Err(SarifReadError::ParseFailed { .. })));
        fs::remove_file(file).ok();
    }

    #[test]
    fn test_unsupported_version() {
        let json = r#"{
            "version": "3.0.0",
            "runs": []
        }"#;
        let file = tmp_file("sarif_v3", json);
        let res = read_sarif(&file);
        assert!(matches!(
            res,
            Err(SarifReadError::UnsupportedVersion { .. })
        ));
        fs::remove_file(file).ok();
    }

    #[test]
    fn test_finding_without_region() {
        let json = r#"{
            "version": "2.1.0",
            "runs": [
                {
                    "results": [
                        {
                            "ruleId": "V9",
                            "message": { "text": "msg" },
                            "locations": [
                                {
                                    "physicalLocation": {
                                        "artifactLocation": { "uri": "a.rs" }
                                    }
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let file = tmp_file("sarif_no_region", json);
        let findings = read_sarif(&file).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file_uri, "a.rs");
        assert_eq!(findings[0].start_line, None);
        fs::remove_file(file).ok();
    }

    #[test]
    fn test_finding_without_rule_id() {
        let json = r#"{
            "version": "2.1.0",
            "runs": [
                {
                    "results": [
                        {
                            "message": { "text": "msg" }
                        }
                    ]
                }
            ]
        }"#;
        let file = tmp_file("sarif_no_rule_id", json);
        let findings = read_sarif(&file).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "");
        fs::remove_file(file).ok();
    }

    #[test]
    fn test_results_empty() {
        let json = r#"{
            "version": "2.1.0",
            "runs": [
                {
                    "results": []
                }
            ]
        }"#;
        let file = tmp_file("sarif_empty", json);
        let findings = read_sarif(&file).unwrap();
        assert!(findings.is_empty());
        fs::remove_file(file).ok();
    }
}
