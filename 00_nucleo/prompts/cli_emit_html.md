# Prompt L0 (revisão): CLI — Flag `--emit-html`

**Camada**: L₄ (Fiação)
**Ficheiro alvo**: `04_wiring/src/main.rs` (revisão de arquivo já
  `IMPLEMENTADO`)
**Passo do roadmap**: 2.2 (componente CLI) — fecha o Passo 2.2
**Status**: IMPLEMENTADO
**Prompt original**: `cli_output_flags.md` (status passa para
  `IMPLEMENTADO (revisado)`).

---

## Decisões de design prévias

- **Passo 2.2**: `render_dsm_html` em L₃ produz `String` com o
  HTML completo. L₄ grava.
- **Padrão estabelecido**: a flag `--emit-trees` já existe e
  grava `trees.json` no mesmo diretório que `--output`. A
  `--emit-html` segue o mesmo molde.

---

## Decisões locais (assumidas neste prompt)

1. **`--emit-html` é flag booleana sem valor**: presente ou
   ausente. Quando presente, grava `dsm.html` no mesmo diretório
   que o `--output`, com nome fixo `dsm.html`.

   Razão: coerência com `--emit-trees` (que grava `trees.json`
   no mesmo diretório, nome fixo). Variantes com caminho
   customizado adicionam complexidade não justificada.

2. **`generated_at` e `tool_version` reutilizados**: os mesmos
   valores já calculados em L₄ para o `graph.json` são passados
   ao `render_dsm_html`. Não recalcular (garante coerência: os
   três artefactos têm o mesmo timestamp).

3. **Ordem de geração**: `graph.json` (sempre) →
   `trees.json` (se `--emit-trees`) → `dsm.html` (se
   `--emit-html`). Independentes entre si.

4. **O particionamento (`partition_for_dsm`) é executado apenas
   se `--emit-html`**: o HTML precisa do `PartitionedOrder`, mas
   o `graph.json` não. Para não pagar o custo do particionamento
   quando não se emite HTML, calcular sob demanda.

   (Custo do particionamento contra Typst: ~6 ms. Negligível,
   mas a separação é limpa.)

---

## Contexto

A CLI actual gera `graph.json` sempre e `trees.json`
opcionalmente. Falta a capacidade de gerar o DSM HTML
(implementado no Passo 2.2 mas ainda não acessível pela CLI).

Esta revisão adiciona a flag `--emit-html`, completando o
acesso à funcionalidade do renderizador.

---

## Mudanças no parsing de CLI

```rust
#[derive(Parser, Debug)]
#[command(name = "crystalline-dsm", version, about)]
struct Cli {
    /// Caminho do workspace Cargo a analisar.
    workspace_path: PathBuf,

    /// Caminho do ficheiro de output do grafo JSON.
    #[arg(short, long, default_value = "./graph.json")]
    output: PathBuf,

    /// Se presente, grava também o trees.json no mesmo diretório.
    #[arg(long)]
    emit_trees: bool,

    /// Se presente, grava também o dsm.html no mesmo diretório.
    #[arg(long)]
    emit_html: bool,
}
```

Apenas a flag `emit_html` é nova. As demais mantêm-se.

---

## Mudanças na lógica do pipeline

No `run_pipeline` (ou função equivalente), após a serialização
do `graph.json` e o eventual `trees.json`, adicionar:

```rust
// Geração opcional do DSM HTML.
if cli.emit_html {
    // O HTML precisa do particionamento. Calcular agora.
    let partition = partition_for_dsm(&graph);

    let html = render_dsm_html(
        &graph,
        &partition,
        &cycles,
        &workspace,
        tool_version,
        &generated_at,
    ).map_err(PipelineError::HtmlError)?;

    let html_path = derive_html_path(&cli.output);
    std::fs::write(&html_path, html)
        .map_err(|e| PipelineError::WriteFailed {
            path: html_path,
            source: e,
        })?;
}
```

---

## Função auxiliar

```rust
/// Dado o path do output principal, retorna o path do dsm.html
/// no mesmo diretório, com nome fixo "dsm.html".
fn derive_html_path(output_path: &Path) -> PathBuf {
    let parent = output_path.parent().unwrap_or(Path::new("."));
    parent.join("dsm.html")
}
```

Comportamento análogo a `derive_trees_path`:
- `./graph.json` → `./dsm.html`
- `/abs/path/output.json` → `/abs/path/dsm.html`

---

## Mudança no enum de erros

Adicionar variante ao `PipelineError`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    // ... variantes existentes ...

    #[error("Falha ao renderizar HTML: {0}")]
    HtmlError(#[from] HtmlRenderError),
}
```

---

## Mudança no `PipelineReport`

Adicionar campo para o caminho do HTML (análogo a `trees_path`):

```rust
#[derive(Debug)]
pub struct PipelineReport {
    pub member_count: usize,
    pub module_count: usize,
    pub edge_count: usize,
    pub cycle_count: usize,
    pub output_path: PathBuf,
    pub trees_path: Option<PathBuf>,
    pub html_path: Option<PathBuf>,  // NOVO
}
```

Preencher `html_path` com `Some(derive_html_path(&cli.output))`
se `--emit-html`, senão `None`.

---

## Mudança no L₂ (Casca)

A função `shell::format_summary` é estendida para mostrar o
caminho do HTML, se gerado:

```rust
pub fn format_summary(report: &PipelineReport) -> String {
    let mut out = String::new();
    // ... contagens existentes ...
    out.push_str(&format!("\nGrafo gravado em: {}\n", report.output_path.display()));
    if let Some(trees_path) = &report.trees_path {
        out.push_str(&format!("Trees gravadas em: {}\n", trees_path.display()));
    }
    if let Some(html_path) = &report.html_path {
        out.push_str(&format!("DSM HTML gravado em: {}\n", html_path.display()));
    }
    out
}
```

(Se a assinatura actual de `format_summary` for por parâmetros
soltos em vez de `&PipelineReport`, adaptar adicionando o
parâmetro `html_path: Option<&Path>`.)

---

## Dependências externas

Nenhuma nova. `render_dsm_html` e `partition_for_dsm` já estão
disponíveis (L₃ e L₁ respectivamente).

---

## Testes esperados

### Testes unitários

1. **`derive_html_path`**: testa os casos análogos a
   `derive_trees_path`:
   - `./graph.json` → `./dsm.html`.
   - `/abs/dir/out.json` → `/abs/dir/dsm.html`.
   - Sem parent → `./dsm.html`.

### Testes de integração

Localização: `04_wiring/tests/integration_tests.rs` (estender).

2. **CLI com `--emit-html`**: rodar contra fixture
   `imports-simple`. Verificar:
   - ExitCode SUCCESS.
   - `graph.json` criado.
   - `dsm.html` criado.
   - `trees.json` NÃO criado (sem a flag).
   - O HTML é não vazio e contém `<canvas`.

3. **CLI com `--emit-html` E `--emit-trees`**: ambos os
   ficheiros adicionais são gerados, mais o `graph.json`. Três
   ficheiros no total.

4. **CLI sem `--emit-html`**: `dsm.html` NÃO é criado.

5. **CLI Typst real com `--emit-html`** (`#[ignore]`): pipeline
   completo gera os três ficheiros. O HTML tem tamanho esperado
   (100 KB - 5 MB).

---

## Critério de aceitação do prompt

- `04_wiring/src/main.rs` actualizado:
  - Flag `--emit-html` no parser.
  - Geração condicional do HTML.
  - `derive_html_path` implementada.
  - `PipelineError::HtmlError` adicionada.
  - `PipelineReport.html_path` adicionado.
- `shell::format_summary` actualizado em L₂.
- Os testes (1 unitário + 4 de integração) passam.
- `cargo clippy --all-targets` sem warnings.
- `cargo test --workspace` passa.
- Status do prompt `cli_output_flags.md` actualizado para
  `IMPLEMENTADO (revisado)`.

---

## Nota sobre o Passo 2.2

Após esta revisão, o Passo 2.2 (Renderizador HTML estático)
está completo:
- ✅ Renderizador implementado (`html_renderer.rs`).
- ✅ Acessível pela CLI (`--emit-html`).
- ✅ Validado contra Typst real.

Resta apenas o item de documentação (`docs/examples/typst.png`),
que é manual e não bloqueia o avanço para o Passo 2.3.

---

## Hash do prompt

A calcular após aprovação.
