# Prompt L0 (revisão): `module_traverser` — Propagação de Entry-Style

**Camada**: L₃ (Infraestrutura)
**Ficheiro alvo**: `03_infra/src/module_traverser.rs` (revisão
  de arquivo já `IMPLEMENTADO`)
**Passo do roadmap**: 1.2 — Travessia de módulos por crate (revisão)
**Status**: PROPOSTO
**ADR motivadora**: ADR-0008 (Propagação de entry-style)
**Prompt original**: `module_traverser.md` (status passa de
  `IMPLEMENTADO` para `IMPLEMENTADO (revisado)`).

---

## Contexto da revisão

O smoke test contra Typst, após a ADR-0007, falhou em 1 de 21
crates (`typst-tests`):

```
ModuleFileNotFound:
  Módulo procurado: args
  Declarado em: /.../typst-original/tests/src/tests.rs
  Caminhos tentados:
    - /.../typst-original/tests/src/tests/args.rs
    - /.../typst-original/tests/src/tests/args/mod.rs
```

`tests.rs` é o entry point do target `[[test]]`. Cargo trata-o
como entry-style (resolve `mod x;` no mesmo directório), mas o
nosso resolvedor só reconhece entry-style pelos nomes `lib.rs`,
`main.rs` e `mod.rs`. O `args.rs` real está em
`tests/src/args.rs` (irmão de `tests.rs`).

A ADR-0008 decide propagar a noção "entry-style" a partir do
`WorkspaceMember` (que já carrega o path do entry point), em vez
de inferir por nome.

---

## Mudanças mínimas

### Assinaturas (privadas) afectadas

```rust
fn resolve_module_path(
    parent_file: &Path,
    parent_is_entry: bool,        // NOVO
    module_ident: &syn::Ident,
    attrs: &[syn::Attribute],
) -> Result<(PathBuf, bool), TraverseError>;

fn traverse_items(
    tree: &mut ModuleTree,
    parent_node: NodeId,
    file_path: &Path,
    parent_is_entry: bool,        // NOVO
    items: &[syn::Item],
    seen_children: &mut HashSet<(NodeId, String)>,
) -> Result<(), TraverseError>;

fn traverse_file(
    tree: &mut ModuleTree,
    parent_node: NodeId,
    file_path: &Path,
    is_entry: bool,               // NOVO
    seen_children: &mut HashSet<(NodeId, String)>,
) -> Result<(), TraverseError>;
```

`traverse_crate` (público) **não muda de assinatura**.

### Algoritmo

1. **`traverse_crate`**: chama `traverse_file(..., &entry_path, true, ...)`
   (a 1ª chamada marca a raiz como entry-style).

2. **`traverse_file`**: passa `is_entry` para `traverse_items` sem
   alteração.

3. **`traverse_items`**, ao processar um `mod foo;` externo:
   - Chama `resolve_module_path(file_path, parent_is_entry, ...)`.
   - Calcula `child_is_entry` para a recursão:
     ```rust
     let child_is_entry = is_mod_rs(&resolved_path);
     ```
     onde `is_mod_rs` retorna `true` se o nome do ficheiro for
     literalmente `"mod.rs"`. Esta é a única convenção sintáctica
     que herda entry-style por nome (qualquer outro nome é
     descendente normal).
   - Recursivamente: `traverse_file(..., resolved_path, child_is_entry, ...)`.

4. **`resolve_module_path`** substitui o bloco:
   ```rust
   let is_mod_or_entry = file_name == "lib.rs"
       || file_name == "main.rs"
       || file_name == "mod.rs";
   ```
   por simplesmente:
   ```rust
   let is_mod_or_entry = parent_is_entry;
   ```
   O resto da função (resolução `#[path = "..."]`, busca em
   `search_dir`) permanece igual.

### Helper

```rust
fn is_mod_rs(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == "mod.rs")
        .unwrap_or(false)
}
```

(Pode ser inline; não justifica módulo separado.)

---

## O que NÃO muda

- API pública (`traverse_crate`, enum `TraverseError`).
- Tratamento de `#[path = "..."]` (ainda vence sobre tudo).
- Detecção de duplicados e módulos inline.
- Mensagens de erro (`ModuleFileNotFound` continua com os mesmos
  campos).
- Comportamento para crates com entry clássico (`lib.rs`/`main.rs`):
  funcionalmente idêntico, porque a 1ª chamada já passa
  `is_entry = true`.

---

## Testes esperados

### Existentes (revalidar)

Todos os 8 testes de `module_traverser_tests.rs` devem continuar
a passar sem modificação. Como nenhum usa entry point com nome
não-canónico, o comportamento observado é idêntico.

### Novo teste de integração

**Fixture sugerida**: `tests/fixtures/tests-entry-custom-name/`

Estrutura:
```
tests/fixtures/tests-entry-custom-name/
├── Cargo.toml          # [workspace] members = ["custom"]
└── custom/
    ├── Cargo.toml      # [[test]] name="runner" path="src/runner.rs"
    └── src/
        ├── runner.rs   # mod helper; fn dummy() {}
        └── helper.rs   # pub fn h() {}
```

Teste esperado: traversal de `custom` produz uma `ModuleTree`
com raiz `custom` e filho `helper`. Sem `ModuleFileNotFound`.

Este teste isola o caso da ADR-0008 sem depender de Typst.

### Teste unitário adicional (opcional)

Em `module_traverser.rs::mod tests`, exercitar `is_mod_rs` para
`mod.rs`, `lib.rs`, `foo.rs`, `mod`, e caminho sem extensão.

---

## Critério de aceitação do prompt

- `03_infra/src/module_traverser.rs` actualizado com:
  - Parâmetro `is_entry`/`parent_is_entry` nas 3 funções
    privadas.
  - `traverse_crate` passa `true` na 1ª chamada.
  - `resolve_module_path` usa `parent_is_entry` em vez do check
    por nome (excepto para `mod.rs` que é detectado ao recursar,
    não dentro de `resolve_module_path`).
- Fixture nova `tests/fixtures/tests-entry-custom-name/` criada.
- Teste de integração novo passa.
- Os 8 testes existentes continuam a passar.
- `cargo clippy --all-targets` sem warnings.
- Smoke test contra Typst re-executado: 21/21 crates traversados
  com sucesso, **zero** falhas em Fase 2.
- Status do prompt original `module_traverser.md` actualizado
  para `IMPLEMENTADO (revisado)` com nota da ADR-0008.

---

## Impacto em código existente

Mudança contida em `module_traverser.rs`. Não afecta:
- `cargo_metadata_reader.rs` (não chama traverser).
- `import_extractor.rs` (consome `ModuleTree`, não chama
  resolver).
- `graph_builder.rs` (consome `ModuleTree`).
- `typst_smoke_test.rs` (apenas re-executar para validar).

A API pública de L₃ não muda; downstream não precisa de ajustes.

---

## Hash do prompt

A calcular após aprovação.
