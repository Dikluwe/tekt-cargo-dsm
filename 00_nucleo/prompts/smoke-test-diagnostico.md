# Prompt L0 (revisão): Smoke Test — Diagnóstico Detalhado de Falhas

**Camada**: L₄ (Fiação) — teste de integração
**Ficheiro alvo**: `04_wiring/tests/typst_smoke_test.rs` (revisão
  de arquivo já `IMPLEMENTADO`)
**Status**: PROPOSTO
**Prompt original**: `typst-smoke-test.md` (status passa para
  `IMPLEMENTADO (revisado)` após aplicação).

---

## Contexto da revisão

O smoke test contra Typst real reportou "20/21 OK (1 falha em
typst-tests, ModuleFileNotFound para módulo args)". A informação
disponível foi suficiente para identificar o problema (`mod args`
não encontrado em `typst-tests`) mas insuficiente para
**investigar** sem voltar ao código manualmente:

- Não mostra qual é o ficheiro pai (onde o `mod args` está
  declarado).
- Não mostra a lista de `attempted_paths` (locais onde o
  resolvedor procurou e não encontrou).
- Não distingue claramente entre `ModuleFileNotFound`,
  `ParseFailed`, `FileReadFailed`.

A enum `TraverseError` já carrega toda essa informação no campo
`attempted_paths` (de `ModuleFileNotFound`). O problema é apenas
de impressão.

Esta revisão melhora o relatório do smoke test para mostrar
todos os detalhes disponíveis em cada falha, sem mudar a lógica
do pipeline.

---

## Mudanças mínimas

### Imprimir detalhes da falha em `traverse_failures`

**Antes** (estado actual):

```rust
if !traverse_failures.is_empty() {
    println!("Falhas:");
    for (name, err) in &traverse_failures {
        println!("  - {}: {:?}", name, err);
    }
}
```

**Depois** (com detalhe):

```rust
if !traverse_failures.is_empty() {
    println!("\nFalhas em traversal:");
    for (name, err) in &traverse_failures {
        println!("  Crate: {}", name);
        match err {
            TraverseError::ModuleFileNotFound {
                module,
                parent_file,
                attempted_paths,
            } => {
                println!("    Tipo: ModuleFileNotFound");
                println!("    Módulo procurado: {}", module);
                println!("    Declarado em: {}", parent_file.display());
                println!("    Caminhos tentados:");
                for p in attempted_paths {
                    println!("      - {}", p.display());
                }
            }
            TraverseError::ParseFailed { file, source } => {
                println!("    Tipo: ParseFailed");
                println!("    Ficheiro: {}", file.display());
                println!("    Erro de parser: {}", source);
            }
            TraverseError::FileReadFailed { path, source } => {
                println!("    Tipo: FileReadFailed");
                println!("    Caminho: {}", path.display());
                println!("    Erro de I/O: {}", source);
            }
            TraverseError::TreeError(e) => {
                println!("    Tipo: TreeError");
                println!("    Detalhe: {:?}", e);
            }
        }
    }
}
```

### Imprimir detalhes em `extract_failures`

Mesma melhoria no bloco análogo da Fase 3:

```rust
if !extract_failures.is_empty() {
    println!("\nFalhas em extracção de imports:");
    for (name, err) in &extract_failures {
        println!("  Crate: {}", name);
        match err {
            ExtractError::FileReadFailed { path, source } => {
                println!("    Tipo: FileReadFailed");
                println!("    Caminho: {}", path.display());
                println!("    Erro de I/O: {}", source);
            }
            ExtractError::ParseFailed { file, source } => {
                println!("    Tipo: ParseFailed");
                println!("    Ficheiro: {}", file.display());
                println!("    Erro de parser: {}", source);
            }
            ExtractError::SuperOutOfBounds {
                from_module,
                raw_use_path,
            } => {
                println!("    Tipo: SuperOutOfBounds");
                println!("    Módulo: {}", from_module);
                println!("    Use path: {}", raw_use_path);
            }
        }
    }
}
```

### Imports adicionais

No topo do ficheiro, importar as variantes específicas:

```rust
use crystalline_dsm_infra::module_traverser::TraverseError;
use crystalline_dsm_infra::import_extractor::ExtractError;
```

(Se já houver wildcard imports, ajustar conforme necessário.)

---

## O que NÃO muda

- A lógica do pipeline (todas as 5 fases) permanece igual.
- A marcação `#[ignore]` permanece.
- A leitura de `TYPST_PATH` permanece.
- Os critérios de sucesso (assertions) permanecem.
- O resumo final com tempos permanece.

Esta revisão é apenas cosmética/diagnóstica: ela troca uma
impressão genérica (`{:?}`) por impressão estruturada e legível.

---

## Como executar e o que esperar

```bash
export TYPST_PATH=/caminho/para/typst-original
cargo test --ignored typst_smoke_test -- --nocapture
```

A secção de falhas do output deve, após a revisão, mostrar algo
como:

```
Falhas em traversal:
  Crate: typst-tests
    Tipo: ModuleFileNotFound
    Módulo procurado: args
    Declarado em: /.../typst-original/tests/src/tests.rs
    Caminhos tentados:
      - /.../typst-original/tests/src/args.rs
      - /.../typst-original/tests/src/args/mod.rs
```

Com essa informação, a investigação subsequente fica fácil:
basta ir ao `parent_file` indicado, ler o contexto da declaração
`mod args;`, e ver onde o ficheiro `args.rs` realmente está no
projecto.

---

## Critério de aceitação do prompt

- `04_wiring/tests/typst_smoke_test.rs` actualizado com os blocos
  detalhados acima.
- `cargo build --tests` compila sem warnings.
- `cargo clippy --all-targets` sem warnings.
- `cargo test` (sem `--ignored`) continua a passar (este teste é
  ignored, não afecta a suite).
- Re-execução manual com `TYPST_PATH` mostra os detalhes
  esperados.
- Status do prompt `typst-smoke-test.md` actualizado para
  `IMPLEMENTADO (revisado)` com nota desta revisão.

---

## Próximos passos (fora deste prompt)

Após esta revisão, o passo seguinte é:

1. Re-executar o smoke test contra Typst.
2. Ler o output detalhado da falha em `typst-tests`.
3. Inspeccionar manualmente o ficheiro `parent_file` indicado.
4. Decidir entre as três rotas:
   - Bug do resolvedor (expandir convenções) → nova ADR + prompt.
   - Limitação aceitável (documentar) → ADR documentando.
   - Bug do modelo `TestsOnly` (talvez `primary_entry` deveria
     retornar `None`) → ADR + prompt de revisão de L₁/L₃.

Essas decisões ficam **fora** deste prompt. Este prompt apenas
melhora a observabilidade.

---

## Hash do prompt

A calcular após aprovação.
