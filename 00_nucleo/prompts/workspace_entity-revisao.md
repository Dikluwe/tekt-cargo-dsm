# Prompt L0 (revisão): Entidade `Workspace` — Extensão do `EntryKind`

**Camada**: L₁ (Núcleo)
**Ficheiro alvo**: `01_core/src/entities/workspace.rs` (revisão de
  arquivo já `IMPLEMENTADO`)
**Passo do roadmap**: 1.1 — Resolução de workspace (revisão)
**Status**: PROPOSTO
**ADR motivadora**: ADR-0007 (Extensão do `EntryKind`)
**Prompt original**: `workspace_resolver.md` (status passa de
  `IMPLEMENTADO` para `IMPLEMENTADO (revisado)`).

---

## Contexto da revisão

O prompt original definiu `EntryKind` com 3 variantes
(`Library`, `Binary`, `LibraryAndBinary`) que não cobriam todos
os casos reais. O smoke test contra Typst expôs membros do
workspace que não se encaixavam em nenhuma das três (ex: `tests`,
`docs`, `tests/fuzz`, `tests/wrapper`).

A ADR-0007 decidiu estender `EntryKind` para 6 variantes,
modelando explicitamente cada tipo de crate. Este prompt descreve
as mudanças correspondentes em L₁.

Mudança importante: o campo `entry_point: PathBuf` do
`WorkspaceMember` é **removido**. Os caminhos passam a viver
dentro de cada variante de `EntryKind`.

---

## Mudanças nas structs

### `EntryKind` (substitui o existente)

```rust
pub enum EntryKind {
    /// Crate com apenas `lib.rs`.
    Library {
        lib_path: PathBuf,
    },

    /// Crate com apenas `main.rs`.
    Binary {
        main_path: PathBuf,
    },

    /// Crate com `lib.rs` E `main.rs`.
    LibraryAndBinary {
        lib_path: PathBuf,
        main_path: PathBuf,
    },

    /// Crate com `lib.rs` marcado como `proc-macro = true`.
    /// Mesma estrutura física que `Library`, mas semântica
    /// diferente para análise de imports.
    ProcMacro {
        lib_path: PathBuf,
    },

    /// Crate sem lib/bin, mas com targets de teste integrados.
    /// `test_paths` lista os ficheiros de teste descobertos
    /// (geralmente em `tests/*.rs`).
    /// Garantido: `test_paths` não está vazio.
    TestsOnly {
        test_paths: Vec<PathBuf>,
    },

    /// Crate sem nenhum target compilável conhecido.
    /// Ex: membros de workspace usados apenas para configuração.
    /// Não tem caminho de entrada.
    NoSourceTarget,
}
```

### `WorkspaceMember` (campo `entry_point` removido)

```rust
pub struct WorkspaceMember {
    /// Nome lógico do crate.
    pub name: String,

    /// Caminho absoluto do diretório raiz do crate.
    pub crate_root: PathBuf,

    /// Tipo do crate, com caminhos de entrada embutidos.
    pub entry_kind: EntryKind,
}
```

**Mudança**: o campo `entry_point: PathBuf` foi removido. Para
obter um caminho de entrada, usar `entry_kind.primary_entry()`
(método novo).

---

## Métodos novos

### Em `EntryKind`

```rust
impl EntryKind {
    /// Retorna o ficheiro de entrada primário, se houver:
    /// - Library, ProcMacro: `lib_path`.
    /// - Binary: `main_path`.
    /// - LibraryAndBinary: `lib_path` (preferência por lib).
    /// - TestsOnly: primeiro elemento de `test_paths`.
    /// - NoSourceTarget: None.
    pub fn primary_entry(&self) -> Option<&Path>;

    /// `true` para variantes com código compilável tradicional:
    /// `Library`, `Binary`, `LibraryAndBinary`, `ProcMacro`.
    /// `false` para `TestsOnly` e `NoSourceTarget`.
    pub fn has_main_source(&self) -> bool;

    /// `true` apenas para `TestsOnly`.
    pub fn is_tests_only(&self) -> bool;

    /// `true` apenas para `NoSourceTarget`.
    pub fn is_empty(&self) -> bool;

    /// `true` apenas para `ProcMacro`.
    pub fn is_proc_macro(&self) -> bool;
}
```

### Em `Workspace` (adições)

```rust
impl Workspace {
    // ... métodos existentes mantidos ...

    /// Itera apenas membros com código tradicional (Library,
    /// Binary, LibraryAndBinary, ProcMacro).
    pub fn members_with_code(&self) -> impl Iterator<Item = &WorkspaceMember>;

    /// Itera apenas membros do tipo TestsOnly.
    pub fn tests_only_members(&self) -> impl Iterator<Item = &WorkspaceMember>;

    /// Itera apenas membros do tipo ProcMacro.
    pub fn proc_macro_members(&self) -> impl Iterator<Item = &WorkspaceMember>;

    /// Itera apenas membros NoSourceTarget.
    pub fn empty_members(&self) -> impl Iterator<Item = &WorkspaceMember>;
}
```

---

## Métodos mantidos

Todos os métodos existentes em `Workspace` continuam funcionando:
- `find_member(name)`
- `member_count()`
- `libraries()` — agora inclui `Library`, `LibraryAndBinary`,
  e `ProcMacro` (todos têm `lib_path`).
- `binaries()` — inclui `Binary` e `LibraryAndBinary`.

Comportamento de `libraries()`/`binaries()` é mantido para
compatibilidade conceitual: "se tem lib, é library; se tem bin,
é binary".

---

## Invariantes adicionais

Além das invariantes originais:

5. **`TestsOnly` tem `test_paths` não vazio**: garantido por L₃
   na construção. Se aparece `TestsOnly`, tem pelo menos 1
   caminho.

6. **`NoSourceTarget` não tem campos**: variante unit. Não há
   caminho associado.

---

## Derives obrigatórios

Iguais aos originais: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`
em todas as structs e enums.

---

## Dependências externas

Nenhuma mudança. Apenas `std`.

---

## Testes esperados (atualização)

Mantidos da versão original:

1. **Construção literal de `Workspace`**: actualizar para usar
   `entry_kind` com as novas variantes; `entry_point` removido.

2. **`find_member`**: comportamento idêntico.

3. **`member_count`**: comportamento idêntico.

4. **`libraries` e `binaries`**: actualizar para verificar que
   `ProcMacro` aparece em `libraries`, e `LibraryAndBinary`
   aparece em ambos.

5. **`PartialEq`**: idêntico.

Novos testes a adicionar:

6. **Construir `WorkspaceMember` com `ProcMacro`**: verificar que
   `entry_kind.is_proc_macro() == true`,
   `entry_kind.has_main_source() == true`,
   `entry_kind.primary_entry() == Some(...)`.

7. **Construir `WorkspaceMember` com `TestsOnly`**: verificar que
   `entry_kind.is_tests_only() == true`,
   `entry_kind.has_main_source() == false`,
   `entry_kind.primary_entry() == Some(primeiro test_path)`.

8. **Construir `WorkspaceMember` com `NoSourceTarget`**:
   `entry_kind.is_empty() == true`,
   `entry_kind.primary_entry() == None`.

9. **`members_with_code()`**: filtra correctamente um
   workspace misto com 1 Library, 1 TestsOnly, 1 ProcMacro, 1
   NoSourceTarget. Retorna 2 membros (Library + ProcMacro).

10. **`tests_only_members()`**: retorna 1 (o TestsOnly).

11. **`proc_macro_members()`**: retorna 1 (o ProcMacro).

12. **`empty_members()`**: retorna 1 (o NoSourceTarget).

---

## Critério de aceitação do prompt

- `01_core/src/entities/workspace.rs` actualizado com as 6
  variantes e novos métodos.
- Campo `entry_point` removido de `WorkspaceMember`.
- Todos os testes (incluindo os 5 originais actualizados + 7
  novos = 12 totais) passam.
- `cargo clippy -p crystalline-dsm-core` sem warnings.
- Status do prompt original `workspace_resolver.md` actualizado
  para `IMPLEMENTADO (revisado)` com nota da ADR-0007.

---

## Impacto em código existente

Esta revisão **quebra compatibilidade** com código que acessa
`member.entry_point` directamente. Os seguintes ficheiros precisam
ser actualizados em conjunto (ver prompts irmãos):

- `03_infra/src/cargo_metadata_reader.rs` — produz os
  `WorkspaceMember` com as novas variantes.
- `03_infra/src/module_traverser.rs` — consome
  `member.entry_kind.primary_entry()` em vez de
  `member.entry_point`.
- `03_infra/src/import_extractor.rs` — mesmo ajuste.
- `04_wiring/src/graph_builder.rs` — consome `Workspace` mas
  provavelmente já passa pelas trees, não pelo `entry_point`
  directo. Verificar.
- `04_wiring/tests/typst_smoke_test.rs` — actualizar impressão
  para mostrar `entry_kind` em vez de `entry_point`.

Os prompts de revisão dos outros ficheiros estão em documentos
separados.

---

## Hash do prompt

A calcular após aprovação.
