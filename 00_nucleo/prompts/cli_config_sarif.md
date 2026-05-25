# Prompt L0 (revisão): CLI — Flags `--config` e `--sarif`

**Camada**: L₄ (Fiação)
**Ficheiro alvo**: `04_wiring/src/main.rs` (revisão de arquivo já
  `IMPLEMENTADO`)
**Passo do roadmap**: 2.3 (componente CLI) — fecha o Passo 2.3 e
  o Marco M2
**Status**: PROPOSTO
**Prompt original**: `cli_output_flags.md` /
  `cli_emit_html.md` (status passa para `IMPLEMENTADO (revisado)`).

---

## Decisões de design prévias

- **Passo 2.3, L₁**: `detect_layer_violations(graph, config)`
  produz `Vec<LayerViolation>`.
- **Passo 2.3, L₃**: `read_layer_config(toml_path, workspace)`
  produz `LayerConfig`; `read_sarif(sarif_path)` produz
  `Vec<SarifFinding>`.
- **Passo 2.3, renderizador**: `render_dsm_html` aceita
  `Option<&[LayerViolation]>` e `Option<&[SarifFinding]>`.

---

## Decisões locais (assumidas neste prompt)

1. **`--config <path>` com default `./crystalline.toml`**:
   - Se a flag for omitida E o ficheiro `./crystalline.toml`
     existir no workspace: usar automaticamente.
   - Se a flag for omitida E o ficheiro não existir: seguir sem
     detecção de camadas (comportamento do Passo 2.2).
   - Se a flag for fornecida e o ficheiro não existir: erro
     explícito (o utilizador pediu, mas não está lá).

2. **`--sarif <path>` sem default**:
   - Se a flag for omitida: sem findings do linter.
   - Se fornecida e o ficheiro não existir: erro explícito.

3. **Detecção de violações só roda se `--emit-html`**: tal como
   o particionamento. Sem HTML, não há onde destacar as
   violações. (Futuro: incluir violações no `graph.json`; fora
   do escopo agora.)

4. **`LayerConfig` derivado precisa do `Workspace`**: o
   `read_layer_config` cruza `[layers]` com `crate_root`. O
   `Workspace` já está em mãos no pipeline.

---

## Contexto

Após os Passos 2.1, 2.2 e os componentes L₁/L₃ do 2.3, falta
ligar tudo na CLI:

- Ler o `crystalline.toml` (se aplicável) → `LayerConfig`.
- Rodar `detect_layer_violations` → `Vec<LayerViolation>`.
- Ler o SARIF (se fornecido) → `Vec<SarifFinding>`.
- Passar ambos ao `render_dsm_html`.

Esta revisão completa o Marco M2.

---

## Mudanças no parsing de CLI

```rust
#[derive(Parser, Debug)]
#[command(name = "crystalline-dsm", version, about)]
struct Cli {
    workspace_path: PathBuf,

    #[arg(short, long, default_value = "./graph.json")]
    output: PathBuf,

    #[arg(long)]
    emit_trees: bool,

    #[arg(long)]
    emit_html: bool,

    /// Caminho do crystalline.toml para detecção de camadas.
    /// Se omitido, tenta ./crystalline.toml automaticamente.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Caminho de um ficheiro SARIF (output do crystalline-lint)
    /// para destacar findings na DSM.
    #[arg(long)]
    sarif: Option<PathBuf>,
}
```

Novas flags: `config` e `sarif`. As demais mantêm-se.

---

## Lógica de resolução do config

```rust
/// Decide qual crystalline.toml usar, se algum.
/// Retorna:
/// - Ok(Some(path)) se há config a usar.
/// - Ok(None) se não há config (e não foi pedido explicitamente).
/// - Err se foi pedido explicitamente mas não existe.
fn resolve_config_path(
    cli: &Cli,
) -> Result<Option<PathBuf>, PipelineError> {
    match &cli.config {
        // Flag fornecida explicitamente.
        Some(path) => {
            if path.exists() {
                Ok(Some(path.clone()))
            } else {
                Err(PipelineError::ConfigNotFound { path: path.clone() })
            }
        }
        // Flag omitida: tentar default no workspace.
        None => {
            let default = cli.workspace_path.join("crystalline.toml");
            if default.exists() {
                Ok(Some(default))
            } else {
                Ok(None)
            }
        }
    }
}
```

---

## Mudanças na lógica do pipeline

No bloco que já existe para `--emit-html` (do prompt
`cli_emit_html.md`), estender:

```rust
if cli.emit_html {
    let partition = partition_for_dsm(&graph);

    // Detecção de violações de camada (se houver config).
    let layer_violations: Vec<LayerViolation> =
        match resolve_config_path(cli)? {
            Some(config_path) => {
                let config = read_layer_config(&config_path, &workspace)?;
                detect_layer_violations(&graph, &config)
            }
            None => Vec::new(),
        };

    // Leitura de findings SARIF (se fornecido).
    let sarif_findings: Vec<SarifFinding> = match &cli.sarif {
        Some(sarif_path) => {
            if !sarif_path.exists() {
                return Err(PipelineError::SarifNotFound {
                    path: sarif_path.clone(),
                });
            }
            read_sarif(sarif_path)?
        }
        None => Vec::new(),
    };

    // Passar ao renderizador (None se vazio, para
    // retrocompatibilidade visual).
    let lv_opt = if layer_violations.is_empty() {
        None
    } else {
        Some(layer_violations.as_slice())
    };
    let sf_opt = if sarif_findings.is_empty() {
        None
    } else {
        Some(sarif_findings.as_slice())
    };

    let html = render_dsm_html(
        &graph,
        &partition,
        &cycles,
        &workspace,
        tool_version,
        &generated_at,
        lv_opt,
        sf_opt,
    )?;

    let html_path = derive_html_path(&cli.output);
    std::fs::write(&html_path, html)
        .map_err(|e| PipelineError::WriteFailed {
            path: html_path,
            source: e,
        })?;
}
```

---

## Mudanças no enum de erros

Adicionar variantes ao `PipelineError`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    // ... variantes existentes ...

    #[error("Ficheiro de configuração não encontrado: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("Ficheiro SARIF não encontrado: {path}")]
    SarifNotFound { path: PathBuf },

    #[error("Falha ao ler configuração de camadas: {0}")]
    ConfigError(#[from] ConfigReadError),

    #[error("Falha ao ler SARIF: {0}")]
    SarifError(#[from] SarifReadError),
}
```

---

## Mudanças no `PipelineReport`

Adicionar contadores de violação para exibir no summary:

```rust
#[derive(Debug)]
pub struct PipelineReport {
    // ... campos existentes ...
    pub layer_violation_count: usize,  // NOVO
    pub sarif_finding_count: usize,    // NOVO
}
```

Preencher com as contagens (0 se não aplicável).

---

## Mudança no L₂ (Casca)

`shell::format_summary` mostra as violações quando há:

```rust
pub fn format_summary(report: &PipelineReport) -> String {
    let mut out = String::new();
    // ... contagens e caminhos existentes ...

    if report.layer_violation_count > 0 {
        out.push_str(&format!(
            "\n⚠ Violações de camada detectadas: {}\n",
            report.layer_violation_count,
        ));
    }
    if report.sarif_finding_count > 0 {
        out.push_str(&format!(
            "Findings do linter (SARIF): {}\n",
            report.sarif_finding_count,
        ));
    }
    out
}
```

---

## Dependências externas

Nenhuma nova em L₄. Os leitores (`read_layer_config`,
`read_sarif`) e o detector (`detect_layer_violations`) já estão
implementados nas suas camadas.

---

## Testes esperados

### Testes de integração

Localização: `04_wiring/tests/integration_tests.rs` (estender).

1. **`--config` com toml válido + `--emit-html`**: usar uma
   fixture de workspace cristalino (criar
   `tests/fixtures/crystalline-mini/` com `crystalline.toml` e
   crates em diretórios de camada). Verificar:
   - ExitCode SUCCESS.
   - `dsm.html` gerado.
   - O HTML contém as violações esperadas (se a fixture tiver
     violações intencionais).

2. **`--config` apontando para ficheiro inexistente**:
   `Err`, ExitCode != 0.

3. **Sem `--config`, sem crystalline.toml no workspace**:
   pipeline roda, HTML gerado sem violações de camada.

4. **Sem `--config`, COM crystalline.toml no workspace**:
   detecção automática. HTML com violações (se houver).

5. **`--sarif` com ficheiro válido**: findings aparecem no
   report e no HTML.

6. **`--sarif` inexistente**: `Err`.

7. **`--config` sem `--emit-html`**: a detecção de violações
   NÃO roda (não há HTML). Sem erro, mas sem destaque.
   `graph.json` gerado normalmente.

8. **Fixture com violação intencional**: criar fixture onde um
   crate-L1 importa de um crate-L3. Verificar
   `layer_violation_count == 1` no report.

### Teste de integração contra projeto cristalino real (`#[ignore]`)

9. **Contra o próprio crystalline-dsm**: rodar a ferramenta
   sobre o próprio workspace (que é cristalino). Verificar que:
   - O `crystalline.toml` do projeto é lido.
   - As violações detectadas (idealmente 0, já que o projeto
     deve respeitar a própria arquitetura) são reportadas.
   - Se houver violações, são reais e investigáveis.

   Este teste é dogfooding: a ferramenta analisa a si mesma.

---

## Critério de aceitação do prompt

- `04_wiring/src/main.rs` actualizado:
  - Flags `--config` e `--sarif`.
  - `resolve_config_path` implementada.
  - Detecção de violações e leitura de SARIF integradas no
    bloco `--emit-html`.
  - Novas variantes de `PipelineError`.
  - Novos campos em `PipelineReport`.
- `shell::format_summary` atualizado.
- Os 8 testes de integração + 1 ignored passam.
- `cargo clippy --all-targets` sem warnings.
- `cargo test --workspace` passa.
- Status dos prompts CLI atualizados.

---

## Nota sobre o fecho do Marco M2

Após esta revisão, o Marco M2 (DSM Visual) está completo:
- ✅ 2.1 — Particionamento DSM.
- ✅ 2.2 — Renderizador HTML.
- ✅ 2.3 — Integração com crystalline.toml (camadas + SARIF).

Resta o Marco M3 (validação do MVP) e o M4 (release).

---

## Fixture sugerida: `crystalline-mini`

Para os testes, criar uma fixture mínima que seja um workspace
cristalino válido:

```
tests/fixtures/crystalline-mini/
├── crystalline.toml          # [layers] L1="01_core", L3="03_infra"
├── Cargo.toml                 # workspace members
├── 01_core/
│   ├── Cargo.toml             # crate "mini-core"
│   └── src/lib.rs             # importa de mini-infra (VIOLAÇÃO L1→L3)
└── 03_infra/
    ├── Cargo.toml             # crate "mini-infra"
    └── src/lib.rs             # importa de mini-core (OK, L3→L1)
```

Esta fixture tem uma violação intencional (L1→L3) e uma
dependência válida (L3→L1), exercitando ambos os caminhos do
detector.

---

## Hash do prompt

A calcular após aprovação.
