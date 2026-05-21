# ⚖️ ADR-0007: Extensão do `EntryKind` para Casos Reais

**Status**: `ACEITO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm`
**Passos afectados**: 1.1 (resolução de workspace) — revisão de
  código `IMPLEMENTADO`.

---

## Contexto

Durante a execução do smoke test contra `lab/typst-original/`
(Passo 1.1 do roadmap original / smoke test antecipado), o
`cargo_metadata_reader` falhou ao tentar processar membros do
workspace do Typst que não têm `lib.rs` nem `main.rs`.

Investigação posterior mostrou que o workspace do Typst declara:

```toml
members = ["crates/*", "docs", "tests", "tests/fuzz", "tests/wrapper"]
```

Os membros `tests`, `docs`, `tests/fuzz`, `tests/wrapper` não são
bibliotecas de produção; são crates auxiliares. Cada um pode ter
uma combinação diferente:

- Apenas targets de teste (`tests/foo.rs`).
- Apenas configuração de workspace, sem código próprio.
- `lib.rs` com `proc-macro = true` (gerador de macros).
- Etc.

A primeira reação do agente foi **silenciar o erro**: ignorar
crates sem entry point clássico, deixando o pipeline continuar.
Decisão revertida pelo utilizador (registo: "fiquei descontente
com parte sendo silenciosamente ignorado, acho que isso foi
preguiça do agente"). Silenciar destrói confiança no output da
ferramenta: o utilizador olha a DSM sem saber que um crate inteiro
foi descartado.

A decisão correcta é **modelar a realidade**: estender `EntryKind`
para representar cada tipo de crate explicitamente, em vez de
forçar todos a caberem em três categorias (`Library`, `Binary`,
`LibraryAndBinary`).

---

## Alternativas consideradas

### Alternativa A — Reverter para falha dura (`NoEntryPoint` como erro)

`read_workspace` continua falhando se algum membro não se encaixa
nas três categorias originais.

**Prós:**
- Forçaria investigação caso a caso.
- Nenhuma mudança em código `IMPLEMENTADO`.

**Contras:**
- Bloqueia uso da ferramenta contra qualquer workspace real
  não-trivial.
- Não resolve o problema fundamental: a modelagem está
  incompleta.

### Alternativa B — Reportar mas continuar (lista de skipados)

`read_workspace` retorna `Ok((Workspace, Vec<SkippedMember>))`.
Cada skipado tem nome e motivo. O caller exibe.

**Prós:**
- Nada é silenciado.
- Mudança mínima em modelo de dados.

**Contras:**
- Muda assinatura pública (regressão semântica).
- Continua tratando esses crates como "não-modeláveis", quando na
  realidade têm informação que poderia ser usada (testes, macros,
  etc).

### Alternativa C — Estender `EntryKind` com variantes novas

Adicionar variantes que cubram os casos reais encontrados.

**Prós:**
- Modela a realidade.
- Cada crate fica classificado, nenhuma informação se perde.
- Consumidores (`module_traverser`, `import_extractor`) podem
  decidir explicitamente o que fazer com cada tipo.

**Contras:**
- Mudança maior em código `IMPLEMENTADO` (struct de L₁ +
  classificador em L₃ + consumidores).
- Cada novo tipo de crate descoberto futuramente exigirá nova
  variante.

---

## Decisão

**Alternativa C: estender `EntryKind`.**

### Variantes do `EntryKind` (novo total: 6)

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
    /// Tem código mas o comportamento de import é especial
    /// (outros crates importam macros, não funções).
    ProcMacro {
        lib_path: PathBuf,
    },

    /// Crate sem lib/bin, mas com targets de teste (ex: `tests/*.rs`).
    /// Pode ter múltiplos arquivos de teste.
    TestsOnly {
        test_paths: Vec<PathBuf>,
    },

    /// Crate sem nenhum target compilável.
    /// Ex: workspace member usado apenas para agrupar configuração,
    /// ou crates malformados.
    NoSourceTarget,
}
```

### Mudanças em `WorkspaceMember`

O campo `entry_point: PathBuf` é **removido** da struct. O
caminho passa a viver dentro de cada variante de `EntryKind` (que
já contém essa informação implicitamente).

```rust
pub struct WorkspaceMember {
    pub name: String,
    pub crate_root: PathBuf,
    pub entry_kind: EntryKind,
    // entry_point removido — agora dentro do EntryKind
}
```

Métodos auxiliares em `WorkspaceMember` ou `EntryKind`:

```rust
impl EntryKind {
    /// Retorna o ficheiro de entrada primário, se houver:
    /// - Library, ProcMacro: `lib_path`.
    /// - Binary: `main_path`.
    /// - LibraryAndBinary: `lib_path` (preferência por lib).
    /// - TestsOnly: primeiro test_path (ou None se vazio).
    /// - NoSourceTarget: None.
    pub fn primary_entry(&self) -> Option<&Path>;

    /// True para variantes com código compilável tradicional
    /// (Library, Binary, LibraryAndBinary, ProcMacro).
    pub fn has_main_source(&self) -> bool;

    /// True para TestsOnly.
    pub fn is_tests_only(&self) -> bool;

    /// True para NoSourceTarget.
    pub fn is_empty(&self) -> bool;
}
```

### Mudanças em `cargo_metadata_reader` (L₃)

A função `classify_targets` é estendida para inspeccionar:

1. **`lib` com `proc-macro = true`**: detectar via inspecção dos
   `crate_types` do target lib do `cargo_metadata::Target`. Se
   contém `"proc-macro"`, classificar como `ProcMacro`.
2. **Targets de tipo `test`** (do array `targets` do
   `cargo_metadata::Package`): se não há lib/bin mas há testes,
   classificar como `TestsOnly`.
3. **Sem nenhum target válido**: classificar como `NoSourceTarget`.

A função **NÃO retorna mais** `Err(NoEntryPoint)`. Esse erro é
removido do enum `CargoMetadataError`.

### Mudanças nos consumidores

**`module_traverser`** (Passo 1.2):
- `Library`, `Binary`, `LibraryAndBinary`, `ProcMacro`: traversar
  a partir do `primary_entry()`. Comportamento idêntico ao actual.
- `TestsOnly`: traversar a partir do primeiro `test_path`.
  Comportamento estendido (suporte simples; melhorias futuras
  podem traversar todos).
- `NoSourceTarget`: retornar `Ok(ModuleTree::new(name, ???))`
  com árvore vazia (apenas raiz simbólica) ou um novo erro
  específico. **Decisão pendente** para o prompt de revisão.

**`import_extractor`** (Passo 1.3):
- Mesma lógica: usa `primary_entry()` quando disponível.
- `NoSourceTarget`: retorna `Vec` vazio sem chamar leitura.

---

## Justificação

1. **Honestidade sobre o domínio**: o ecossistema Cargo permite
   mais que três tipos de crate. Nossa modelagem deve refletir
   isso, não esconder.

2. **Composição com decisões futuras**: ter `ProcMacro` separado
   permite, futuramente, tratar imports de macros de forma
   diferenciada (`#[macro_use]`, etc).

3. **Restauração de confiança no output**: a DSM mostrará todos
   os membros do workspace, com a categoria de cada um visível
   ao utilizador. Nenhum crate é "perdido".

4. **Custo aceitável da mudança**: três ficheiros principais a
   modificar (`workspace.rs` em L₁, `cargo_metadata_reader.rs` em
   L₃, e ajustes em consumidores L₃). Testes precisam ser
   actualizados, mas a estrutura é a mesma.

---

## Consequências

### ✅ Positivas

- Workspaces reais como o do Typst são processados sem ignorar
  nada.
- Cada crate tem categoria conhecida; relatórios são honestos.
- `proc-macro` ganha tratamento explícito (preparação para
  análise mais profunda futura).
- O erro `NoEntryPoint` desaparece — não é mais necessário, e
  removê-lo simplifica a interface.

### ❌ Negativas

- Mudança em código `IMPLEMENTADO`: `workspace.rs` (L₁),
  `cargo_metadata_reader.rs` (L₃), `module_traverser.rs` (L₃),
  `import_extractor.rs` (L₃), `graph_builder.rs` (L₄), e os
  testes correspondentes. Tudo precisa ser actualizado.
- Prompts originais (`workspace.md`, `cargo_metadata_reader.md`)
  precisam ser revisados.
- Fixtures de teste precisam de novos casos (TestsOnly, ProcMacro).

### ⚙️ Acções decorrentes

1. Reverter a mudança "silenciosamente ignora" feita durante o
   smoke test em `cargo_metadata_reader.rs`. (NÃO é compatível
   com a nova modelagem.)
2. Implementar as 6 variantes em `01_core/src/entities/workspace.rs`.
3. Adaptar `classify_targets` em
   `03_infra/src/cargo_metadata_reader.rs` para usar as 6
   variantes.
4. Adaptar `module_traverser` e `import_extractor` para tratar
   `TestsOnly` (traversar primeiro test) e `NoSourceTarget` (não
   traversar).
5. Adaptar `graph_builder` se necessário (provavelmente não, já
   que consome `ModuleTree`s já construídas).
6. Adicionar fixtures de teste:
   - `tests/fixtures/proc-macro-crate/`
   - `tests/fixtures/tests-only-crate/`
   - `tests/fixtures/no-source-crate/`
7. Re-executar smoke test contra Typst. Esperado: 100% dos
   membros classificados (nenhum descartado).
8. Atualizar status dos prompts afectados:
   - `workspace_resolver.md` → `IMPLEMENTADO (revisado)`.
   - `cargo_metadata_reader.md` → `IMPLEMENTADO (revisado)`.

---

## Critérios de reavaliação

Esta ADR deve ser reaberta se:

1. Um novo tipo de crate aparecer (ex: targets `bench` apenas) e
   for representativo o suficiente para merecer variante própria.
2. A complexidade de `EntryKind` ficar incómoda. Sinal: mais de
   8 variantes ou consumidores com muitos `match` repetitivos.
3. Análise futura precisar de granularidade fina (ex: distinguir
   `dylib` de `staticlib` dentro de `Library`).

---

## Referências

- ADR-0001 — Criação da ferramenta.
- Cargo Book — Cargo Targets:
  https://doc.rust-lang.org/cargo/reference/cargo-targets.html
- Estrutura do workspace do Typst (referência empírica):
  `members = ["crates/*", "docs", "tests", "tests/fuzz", "tests/wrapper"]`.
- Smoke test contra Typst real (incidente que motivou esta ADR).
