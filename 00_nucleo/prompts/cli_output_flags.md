# Prompt L0: CLI — Flags `--output` e `--emit-trees`

**Camada**: L₄ (Fiação)
**Ficheiro alvo**: `04_wiring/src/main.rs` (revisão de arquivo já
  `IMPLEMENTADO` no M0)
**Passo do roadmap**: 1.4 (parte CLI, último componente) — fecha M1
**Status**: IMPLEMENTADO (revisado)

---

## Decisões de design prévias

- **ADR-0009**: serialização JSON em L₃ via DTOs. L₄ apenas
  consome `to_canonical_json` e grava.
- **ADR-0010**: dois ficheiros (`graph.json` + opcional
  `trees.json`). L₄ controla a presença do segundo via flag.

---

## Decisões locais (assumidas neste prompt)

1. **Verificar estado actual antes de aplicar**: a CLI do M0
   provavelmente tem flag `--output` com output mockado para
   `./dsm.html`. Esta revisão substitui o mock por gravação real
   do `graph.json`, e adiciona suporte a `--emit-trees`.

   Se o estado actual for diferente do esperado, adaptar este
   prompt em vez de aplicar cegamente.

2. **Default de `--output`**: passa a ser `./graph.json` (em vez
   de `./dsm.html`). Razão: o produto canónico do M1 é o JSON.
   O HTML virá no M2 e poderá ser controlado por outra flag.

3. **`--emit-trees` é flag booleana sem valor**: presente ou
   ausente. Quando presente, grava `trees.json` no mesmo
   diretório que o `--output`, com nome fixo `trees.json`.

   Razão da decisão: variantes mais complexas (especificar nome
   do `trees.json` separadamente) introduzem complexidade não
   justificada para o MVP.

4. **`generated_at` e `tool_version` são gerados em L₄**: L₃ não
   consulta o relógio nem o ambiente. L₄ obtém:
   - `tool_version` via `env!("CARGO_PKG_VERSION")` em
     compile-time.
   - `generated_at` em RFC 3339 via `chrono` ou `time` em
     runtime.

5. **Sem deserialização na CLI**: a CLI apenas grava JSON. Não
   lê JSON existente. A funcionalidade `from_canonical_json` em
   L₃ existe para consumidores externos e testes, não para a CLI
   actual.

6. **Stdout do CLI permanece informativo**: as mensagens
   amigáveis para o utilizador (via L₂) continuam impressas no
   stdout. O JSON vai apenas para ficheiro.

---

## Contexto

A CLI actual (do M0) tem estrutura:

```rust
fn main() -> ExitCode {
    let args = ...;
    // ... fluxo mockado:
    // 1. lê workspace_path
    // 2. cria ficheiro de output com conteúdo mockado
    // 3. imprime mensagens via L₂
}
```

Esta revisão substitui o conteúdo mockado pelo pipeline real
implementado nos Passos 1.1–1.4:

```
read_workspace → traverse_crate × N → extract_imports × N
              → build_graph → detect_cycles
              → to_canonical_json → gravar ficheiro
              → [se --emit-trees] to_canonical_json_trees → gravar
```

E imprime via L₂ um resumo (membros, módulos, imports, ciclos)
para feedback ao utilizador.

---

## Mudanças no parsing de CLI

A definição actual do `clap` precisa ser estendida.

### Argumentos e flags

```rust
#[derive(Parser, Debug)]
#[command(name = "crystalline-dsm", version, about)]
struct Cli {
    /// Caminho do workspace Cargo a analisar.
    workspace_path: PathBuf,

    /// Caminho do ficheiro de output do grafo JSON.
    /// Default: ./graph.json
    #[arg(short, long, default_value = "./graph.json")]
    output: PathBuf,

    /// Se presente, grava também o trees.json no mesmo
    /// diretório que --output.
    #[arg(long)]
    emit_trees: bool,

    // (Eventuais outras flags existentes do M0 são mantidas.)
}
```

Notas:

- A flag `--format` do M0 (se ainda existir como `json|html`)
  pode ser removida ou marcada como deprecated. O formato JSON
  passa a ser o único produto da CLI no M1. HTML virá no M2.
  Verificar com o estado actual; se a flag estiver lá, decidir
  remoção em conjunto com a aplicação deste prompt.

---

## Lógica principal de `main`

Pseudocódigo:

```rust
fn main() -> ExitCode {
    let cli = Cli::parse();

    // 1. Validação básica.
    if !cli.workspace_path.exists() {
        eprintln!("{}", shell::format_error(
            "Workspace não encontrado",
            &cli.workspace_path.display().to_string(),
        ));
        return ExitCode::from(1);
    }

    // 2. Pipeline.
    match run_pipeline(&cli) {
        Ok(report) => {
            println!("{}", shell::format_summary(&report));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}", shell::format_error("Falha na análise", &e));
            ExitCode::from(2)
        }
    }
}

fn run_pipeline(cli: &Cli) -> Result<PipelineReport, PipelineError> {
    // Fase 1.1: workspace
    let workspace = read_workspace(&cli.workspace_path)?;

    // Fase 1.2: trees
    let mut trees: HashMap<String, ModuleTree> = HashMap::new();
    for member in &workspace.members {
        if let Ok(tree) = traverse_crate(member) {
            trees.insert(member.name.clone(), tree);
        }
        // Falhas individuais são acumuladas no report (não fatais).
    }

    // Fase 1.3: imports
    let workspace_crate_names: Vec<String> = workspace.members
        .iter().map(|m| m.name.clone()).collect();
    let mut edges_per_crate: HashMap<String, Vec<ImportEdge>> = HashMap::new();
    for (crate_name, tree) in &trees {
        let member = workspace.find_member(crate_name)
            .expect("crate_name vem do workspace");
        if let Ok(edges) = extract_imports(member, tree, &workspace_crate_names) {
            edges_per_crate.insert(crate_name.clone(), edges);
        }
    }

    // Fase 1.4 (graph): construção
    let graph = build_graph(&workspace, &trees, &edges_per_crate);

    // Fase 1.5: ciclos
    let cycles = detect_cycles(&graph);

    // Serialização principal.
    let tool_version = env!("CARGO_PKG_VERSION");
    let generated_at = current_rfc3339_timestamp();

    let graph_json = to_canonical_json(
        &graph, &cycles, &workspace, tool_version, &generated_at,
    )?;

    std::fs::write(&cli.output, graph_json)
        .map_err(|e| PipelineError::WriteFailed {
            path: cli.output.clone(),
            source: e,
        })?;

    // Serialização opcional dos trees.
    if cli.emit_trees {
        let trees_path = derive_trees_path(&cli.output);
        let trees_json = to_canonical_json_trees(
            &trees, &workspace, tool_version, &generated_at,
        )?;
        std::fs::write(&trees_path, trees_json)
            .map_err(|e| PipelineError::WriteFailed {
                path: trees_path,
                source: e,
            })?;
    }

    Ok(PipelineReport {
        member_count: workspace.member_count(),
        module_count: trees.values().map(|t| t.node_count()).sum(),
        edge_count: graph.edge_count(),
        cycle_count: cycles.cycle_count(),
        output_path: cli.output.clone(),
        trees_path: if cli.emit_trees {
            Some(derive_trees_path(&cli.output))
        } else {
            None
        },
    })
}
```

---

## Funções auxiliares

### `derive_trees_path`

```rust
/// Dado o path do output principal, retorna o path do trees.json
/// no mesmo diretório, com nome fixo "trees.json".
fn derive_trees_path(output_path: &Path) -> PathBuf {
    let parent = output_path.parent().unwrap_or(Path::new("."));
    parent.join("trees.json")
}
```

Comportamento:
- `./graph.json` → `./trees.json`
- `/abs/path/output.json` → `/abs/path/trees.json`
- `nome_qualquer.json` (sem parent) → `./trees.json`

### `current_rfc3339_timestamp`

```rust
fn current_rfc3339_timestamp() -> String;
```

Retorna timestamp actual em formato RFC 3339 (ex:
`"2026-05-20T22:30:00Z"`). Implementação via `chrono` ou `time`
crate.

**Decisão de dependência**: adicionar `chrono` em
`04_wiring/Cargo.toml` (com feature mínima — apenas o que for
necessário para formatar timestamp). Alternativa: `time` crate.
Ambas funcionam; recomendação: `chrono` por ser mais comum.

### Struct `PipelineReport` e `PipelineError`

```rust
#[derive(Debug)]
pub struct PipelineReport {
    pub member_count: usize,
    pub module_count: usize,
    pub edge_count: usize,
    pub cycle_count: usize,
    pub output_path: PathBuf,
    pub trees_path: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Falha ao ler workspace: {0}")]
    WorkspaceError(#[from] CargoMetadataError),

    #[error("Falha ao serializar JSON: {0}")]
    JsonError(#[from] JsonSerializeError),

    #[error("Falha ao serializar trees: {0}")]
    TreesError(#[from] TreesSerializeError),

    #[error("Falha ao gravar ficheiro {path}: {source}")]
    WriteFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
```

---

## Mudança no L₂ (Casca)

A função `shell::format_summary` precisa de atualização para
mostrar os caminhos dos ficheiros gerados:

```rust
pub fn format_summary(report: &PipelineReport) -> String {
    let mut out = String::new();
    out.push_str("=== Crystalline DSM ===\n");
    out.push_str(&format!("Crates: {}\n", report.member_count));
    out.push_str(&format!("Módulos: {}\n", report.module_count));
    out.push_str(&format!("Arestas: {}\n", report.edge_count));
    out.push_str(&format!("Ciclos: {}\n", report.cycle_count));
    out.push_str(&format!("\nGrafo gravado em: {}\n", report.output_path.display()));
    if let Some(trees_path) = &report.trees_path {
        out.push_str(&format!("Trees gravadas em: {}\n", trees_path.display()));
    }
    out
}
```

A função `shell::format_error` é mantida (sem mudança).

---

## Dependências externas

`04_wiring/Cargo.toml`:

```toml
[dependencies]
clap = { version = "...", features = ["derive"] }
chrono = { version = "0.4", default-features = false, features = ["clock"] }
# As outras já presentes (crystalline-dsm-core, infra, etc).
```

A feature `default-features = false, features = ["clock"]` em
`chrono` reduz o tamanho da dependência ao mínimo necessário
para gerar timestamps.

---

## Testes esperados

### Testes unitários

Limitados em L₄ (a CLI é principalmente integração). Casos
isoláveis:

1. **`derive_trees_path`**: testa os 3 casos do docstring.

2. **`current_rfc3339_timestamp` retorna formato válido**:
   verifica regex `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$` (ou
   similar).

### Testes de integração

Localização: `04_wiring/tests/cli_integration_tests.rs` (já
existe do M0; estender).

3. **CLI básica sem `--emit-trees`**: rodar contra fixture
   `imports-simple` (existente). Verificar:
   - ExitCode SUCCESS.
   - Ficheiro `graph.json` foi criado no path esperado.
   - O JSON é parseável.
   - `trees.json` NÃO foi criado.

4. **CLI com `--emit-trees`**: rodar contra
   `imports-workspace`. Verificar:
   - ExitCode SUCCESS.
   - Ambos os ficheiros existem.
   - O `trees.json` contém as 2 árvores esperadas.

5. **`--output` em path absoluto**: o ficheiro é criado no path
   especificado, não no `cwd`.

6. **`workspace_path` inexistente**: ExitCode != 0, mensagem de
   erro no stderr.

7. **CLI contra Typst real** (`#[ignore]`, paralelo ao smoke
   test): rodar pipeline completo, verificar que ambos os
   ficheiros são criados, e que o `trees.json` (quando
   `--emit-trees`) contém entradas para os 21 crates.

---

## Critério de aceitação do prompt

- `04_wiring/src/main.rs` actualizado:
  - Parser `clap` com as flags `--output` e `--emit-trees`.
  - Pipeline real substituindo o mock.
  - Gravação de `graph.json` sempre.
  - Gravação de `trees.json` se `--emit-trees`.
- `shell::format_summary` actualizado em L₂.
- `chrono` adicionado ao `Cargo.toml` de `04_wiring`.
- Os testes (2 unitários + 5 de integração) passam.
- `cargo clippy --all-targets` sem warnings.
- `cargo test --workspace` passa.
- Status do prompt `cli.md` (do M0) actualizado para
  `IMPLEMENTADO (revisado)`.

---

## Estados anteriores possíveis (verificação na materialização)

Antes de aplicar este prompt, verificar:

1. **A flag `--output` existe?** Provável `sim` (do M0). Se
   default for `./dsm.html`, alterar para `./graph.json`.

2. **A flag `--format` existe?** Se sim, decidir entre:
   - Remover (M1 só produz JSON).
   - Manter como `json` apenas (deixar para o M2 reintroduzir
     `html`).

3. **`shell::format_summary` existe?** Se sim, estender. Se não,
   criar.

4. **Já existe alguma chamada a `to_canonical_json` em L₄?** Se
   sim, integrar em vez de duplicar.

A flexibilidade dessas verificações é deliberada: o agente que
aplicar adapta conforme encontra.

---

## Hash do prompt

A calcular após aprovação.
