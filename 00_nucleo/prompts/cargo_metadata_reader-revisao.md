# Prompt L0 (revisão): `cargo_metadata_reader` — Classificação Estendida

**Camada**: L₃ (Infraestrutura)
**Ficheiro alvo**: `03_infra/src/cargo_metadata_reader.rs` (revisão
  de arquivo já `IMPLEMENTADO`)
**Passo do roadmap**: 1.1 — Resolução de workspace (revisão)
**Status**: PROPOSTO
**ADR motivadora**: ADR-0007 (Extensão do `EntryKind`)
**Prompt original**: `cargo_metadata_reader.md` (status passa de
  `IMPLEMENTADO` para `IMPLEMENTADO (revisado)`).

---

## Contexto da revisão

Durante o smoke test contra Typst real, foi feita uma alteração
ad-hoc neste ficheiro: membros sem `lib` nem `bin` passaram a ser
**silenciosamente ignorados**. A ADR-0007 reverte essa decisão e
substitui por classificação explícita: cada membro do workspace é
modelado por uma variante de `EntryKind` (incluindo as novas
`ProcMacro`, `TestsOnly`, `NoSourceTarget`).

Este prompt descreve:

1. Como reverter a "silenciamento" feito ad-hoc.
2. Como estender `classify_targets` para cobrir todas as 6
   variantes da ADR-0007.

---

## Reversão necessária

A mudança feita durante o smoke test (não documentada em prompt
nem ADR formal) deve ser revertida:

**Antes da reversão (estado actual)**:
```rust
// Comportamento adicionado ad-hoc: ignora silenciosamente
fn classify_targets(...) -> Option<(EntryKind, PathBuf)> {
    // retorna None se não há lib/bin, e o caller filtra
}
```

**Após reversão**: voltar ao comportamento de retornar erro, mas
substituir o erro por classificação correcta usando as novas
variantes do `EntryKind`.

O enum `CargoMetadataError` perde a variante `NoEntryPoint` (não
é mais necessária; o caso é coberto por `NoSourceTarget`).

---

## Nova lógica de `classify_targets`

```rust
fn classify_targets(
    package: &cargo_metadata::Package,
) -> EntryKind;
```

**Mudança de assinatura**: retorna `EntryKind` directamente (não
`Result<(EntryKind, PathBuf), Error>`). O caminho agora vive dentro
da variante.

### Algoritmo

Para cada `package` do workspace:

1. **Procurar `lib`**: filtrar `package.targets` por
   `target.kind == ["lib"]` ou `target.kind == ["rlib"]` ou
   `target.kind == ["dylib"]` ou `target.kind == ["staticlib"]`
   ou `target.kind == ["cdylib"]`.

   Se encontrado, registar `lib_path = target.src_path`.

2. **Procurar `proc-macro`**: detectar se algum target tem
   `crate_types` contendo `"proc-macro"`. Pode ser via
   `target.kind == ["proc-macro"]` ou via `target.crate_types`
   contendo `"proc-macro"` (a API exacta depende da versão do
   `cargo_metadata`; verificar e usar a forma idiomática).

   Se encontrado, marcar `is_proc_macro = true`.

3. **Procurar `bin`**: filtrar `package.targets` por
   `target.kind == ["bin"]`.

   Se múltiplos binários: usar o primeiro (limitação herdada do
   prompt original; documentar).

   Se encontrado, registar `main_path = target.src_path`.

4. **Procurar `test`**: filtrar `package.targets` por
   `target.kind == ["test"]`.

   Recolher todos os `src_path` num `Vec<PathBuf>`.

5. **Decidir variante**:

   ```
   se (is_proc_macro) e (lib_path existe):
       EntryKind::ProcMacro { lib_path }
   senão se (lib_path existe) e (main_path existe):
       EntryKind::LibraryAndBinary { lib_path, main_path }
   senão se (lib_path existe):
       EntryKind::Library { lib_path }
   senão se (main_path existe):
       EntryKind::Binary { main_path }
   senão se (test_paths não vazio):
       EntryKind::TestsOnly { test_paths }
   senão:
       EntryKind::NoSourceTarget
   ```

A ordem importa: `ProcMacro` vence sobre `Library` (porque um
crate proc-macro tem `lib` mas o tratamento é especial).

---

## Mudanças em `read_workspace`

A função principal mantém a assinatura:

```rust
pub fn read_workspace(workspace_path: &Path)
    -> Result<Workspace, CargoMetadataError>;
```

Mudanças internas:

1. Loop sobre `metadata.workspace_members` é igual.
2. Para cada package, chamar `classify_targets(package)` que
   agora retorna `EntryKind` directamente (sem erro de
   `NoEntryPoint`).
3. Construir `WorkspaceMember` sem o campo `entry_point`
   (removido conforme ADR-0007).

Pseudocódigo:

```rust
for package_id in &metadata.workspace_members {
    let package = &metadata[package_id];
    let crate_root = package.manifest_path.parent()
        .expect("manifest path sempre tem pai")
        .to_path_buf();
    let entry_kind = classify_targets(package);

    members.push(WorkspaceMember {
        name: package.name.clone(),
        crate_root,
        entry_kind,
    });
}
```

---

## Mudanças no enum de erros

```rust
#[derive(Debug, thiserror::Error)]
pub enum CargoMetadataError {
    #[error("Caminho inválido ou inacessível: {path}")]
    InvalidPath { path: PathBuf },

    #[error("Falha ao executar 'cargo metadata': {source}")]
    MetadataExecutionFailed {
        #[from]
        source: cargo_metadata::Error,
    },

    // REMOVIDO: NoEntryPoint (substituído por NoSourceTarget no EntryKind)

    #[error("Workspace não contém nenhum membro")]
    EmptyWorkspace,
}
```

A variante `NoEntryPoint` é **removida**. Quem dependia dela
deve passar a inspeccionar `entry_kind == NoSourceTarget`.

---

## Testes atualizados

Os 7 testes de integração originais permanecem, com ajustes:

1. **`empty-workspace`**: idêntico. `Err(EmptyWorkspace)`.

2. **`single-lib-crate`**: agora verifica
   `entry_kind == EntryKind::Library { lib_path: ... }` em vez
   de `entry_kind == EntryKind::Library` + check separado de
   `entry_point`.

3. **`single-bin-crate`**: análogo a 2.

4. **`lib-and-bin-crate`**: análogo, agora verifica os dois paths.

5. **`multi-crate-workspace`**: análogo.

6. **`invalid-path`**: idêntico.

7. **`not-a-workspace`**: idêntico.

Novos testes (e novas fixtures):

8. **`proc-macro-crate`**: workspace com 1 crate que tem
   `[lib] proc-macro = true`.
   Esperado: `entry_kind == EntryKind::ProcMacro { lib_path }`.

9. **`tests-only-crate`**: workspace com 1 crate sem lib/bin mas
   com `tests/foo.rs`.
   Esperado: `entry_kind == EntryKind::TestsOnly { test_paths }`,
   onde `test_paths` tem pelo menos 1 elemento.

10. **`no-source-crate`**: workspace com 1 crate cujo `Cargo.toml`
    declara `[package]` mas o crate não tem nenhum source file
    (ou só tem README.md).
    Esperado: `entry_kind == EntryKind::NoSourceTarget`.

11. **Smoke test re-executado contra Typst**: 100% dos membros
    classificados, nenhum erro de `NoEntryPoint`. Adicionar como
    teste manual no documento de smoke test.

---

## Estrutura das novas fixtures

```
tests/fixtures/
├── (existentes)
├── proc-macro-crate/
│   ├── Cargo.toml          # [workspace] members = ["macros"]
│   └── macros/
│       ├── Cargo.toml      # [lib] proc-macro = true
│       └── src/lib.rs      # uso de proc_macro::TokenStream
├── tests-only-crate/
│   ├── Cargo.toml          # members = ["only_tests"]
│   └── only_tests/
│       ├── Cargo.toml      # sem [lib] nem [[bin]]; tem [[test]]
│       └── tests/
│           └── integration.rs  # fn test_*()
└── no-source-crate/
    ├── Cargo.toml          # members = ["empty"]
    └── empty/
        ├── Cargo.toml      # [package] sem lib/bin
        └── README.md
```

O `tests-only-crate/only_tests/Cargo.toml` precisa declarar
explicitamente `[[test]]` apontando para `tests/integration.rs`,
porque alguns layouts não inferem automaticamente.

O `no-source-crate/empty/` é o caso mais raro. Pode requer
configuração específica no `Cargo.toml` para o Cargo aceitar (ex:
`autobins = false`, `autotests = false`). Se difícil de
reproduzir, marcar a fixture como "teste só roda em cenários
específicos".

---

## Limitações conhecidas

1. **Múltiplos binários**: ainda usamos apenas o primeiro (igual
   ao original).
2. **Múltiplos targets de teste**: agora todos são registrados em
   `TestsOnly::test_paths`. Para os outros tipos
   (`LibraryAndBinary`, etc), targets de teste adicionais são
   ignorados.
3. **Targets de bench**: não modelados. Crate com apenas
   `[[bench]]` cai em `NoSourceTarget`. Adicionar variante
   `BenchOnly` no futuro se necessário.

---

## Critério de aceitação do prompt

- `03_infra/src/cargo_metadata_reader.rs` actualizado:
  - `classify_targets` retorna `EntryKind` directamente.
  - Variante `NoEntryPoint` removida do enum de erros.
  - Mudança "silenciosamente ignora" revertida.
- As 3 fixtures novas existem em `tests/fixtures/`.
- Os 10 testes de integração passam (7 actualizados + 3 novos).
- `cargo clippy --all-targets` sem warnings.
- Smoke test contra Typst re-executado: nenhum membro descartado,
  todos classificados.
- Status do prompt original `cargo_metadata_reader.md` actualizado
  para `IMPLEMENTADO (revisado)` com nota da ADR-0007.

---

## Impacto em código existente

Esta revisão impacta os ficheiros descritos na ADR-0007. Em
particular:

- `03_infra/src/module_traverser.rs`: usar
  `member.entry_kind.primary_entry()` em vez de
  `member.entry_point`. Para `NoSourceTarget`, retornar
  `Ok(ModuleTree::new(name, ???))` com árvore vazia, OU adicionar
  variante de erro/skip explícita. **Decisão sugerida**: introduzir
  `TraverseResult::Skipped { reason }` ou similar; sem isso, o
  smoke test não consegue distinguir "traversou e está vazio" de
  "não tentou traversar".

- `03_infra/src/import_extractor.rs`: mesma lógica. Para
  `NoSourceTarget`, retornar `Vec::new()` directamente.

- `04_wiring/src/graph_builder.rs`: provavelmente sem mudança
  directa (consome `ModuleTree`s já construídas).

- `04_wiring/tests/typst_smoke_test.rs`: substituir impressão
  de `entry_kind` para mostrar a variante completa (incluindo
  paths). Exemplo: `"crate-x (Library { lib_path: ... })"`.

Os ajustes em `module_traverser` e `import_extractor` podem ser
feitos no mesmo commit que esta revisão, ou em commits separados
encadeados. Não é necessário prompt formal para esses ajustes (são
adaptações mecânicas), mas vale registar no histórico de revisões
dos prompts originais correspondentes.

---

## Hash do prompt

A calcular após aprovação.
