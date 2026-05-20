# Prompt L0: Entidade `ImportEdge` (L₁)

**Camada**: L₁ (Núcleo)
**Ficheiro alvo**: `01_core/src/entities/import_edge.rs`
**Passo do roadmap**: 1.3 — Extracção de imports (`use`)
**Status**: IMPLEMENTADO

---

## Decisões de design prévias (registadas em ADR)

- **ADR-0004**: Nó do grafo é módulo lógico com identificador
  `<crate_name>::<module_path>`.

---

## Decisões locais (assumidas neste prompt)

1. **Granularidade da aresta**: registar tanto o caminho do
   módulo alvo quanto o item importado (Decisão de
   conversa, 2026-05-20).

2. **Use lists separadas**: `use a::{X, Y, Z}` gera 3 arestas
   distintas, não uma com lista. Cada aresta representa **uma**
   importação atómica.

3. **Cinco categorias de classificação**: `CurrentCrate`,
   `WorkspaceCrate`, `External`, `Stdlib`, `Unresolved`. A
   resolução é feita em L₃ no momento da construção da aresta;
   L₁ apenas modela.

4. **Reaproveitamento da grain de `ModuleTree`**: a aresta
   referencia o módulo origem via `NodeId` da `ModuleTree`. O
   módulo alvo é referenciado por **string** (caminho canónico),
   não por `NodeId`. Razão: o alvo pode estar noutro crate
   (`ModuleTree` diferente) ou ser externo (sem `ModuleTree`).

---

## Contexto

O Passo 1.3 processa cada `mod`ulo de cada `ModuleTree` produzido
no Passo 1.2 e extrai os `use` statements. Cada `use` vira uma ou
mais `ImportEdge`s (uma por item importado, conforme decisão 2).

A colecção de todas as arestas forma o grafo bruto. A construção
do grafo final, com nós e adjacências resolvidas para `NodeId`s,
fica para o Passo 1.4.

Esta entidade modela a aresta bruta, anterior à construção do
grafo. É a saída do extractor de imports e a entrada do
construtor do grafo.

---

## Definição das structs

### `ImportEdge`

```rust
pub struct ImportEdge {
    /// Nó que faz o import (módulo origem).
    /// Pertence à `ModuleTree` do crate que está a ser analisado.
    pub from: NodeId,

    /// Identificador canónico do crate de origem.
    /// Necessário porque `NodeId` sozinho não identifica univocamente
    /// entre múltiplos `ModuleTree`s.
    /// Ex: "crystalline_dsm_core".
    pub from_crate: String,

    /// Caminho lógico do módulo alvo, na forma canónica.
    /// Para imports internos: "crate_x::a::b".
    /// Para imports externos: o caminho como aparece (ex: "serde::de").
    /// Para stdlib: ex: "std::collections".
    /// Pode estar vazio em casos degenerate; ver `Unresolved`.
    pub target_module: String,

    /// Item específico importado. O segmento final do `use`.
    /// Ex: para `use a::b::Foo`, este campo é "Foo".
    /// Para `use a::b::*`, este campo é "*" e `is_glob = true`.
    /// Para `use a::b;` (importação do módulo em si), este campo é "b".
    pub imported_item: String,

    /// Classificação do import.
    pub kind: ImportKind,

    /// O caminho do `use` na forma textual, exactamente como aparece
    /// no código fonte (sem `crate::`/`self::`/`super::` resolvidos).
    /// Útil para diagnóstico e depuração.
    /// Ex: "super::a::Foo", "crate::utils::helper", "std::io::Read".
    pub raw_use_path: String,

    /// `true` se é glob import (`use a::b::*`).
    pub is_glob: bool,

    /// Alias usado em `use X as Y`. `None` se não houver alias.
    /// Quando glob, sempre `None`.
    pub alias: Option<String>,

    /// `true` se é re-export (`pub use ...`).
    pub is_reexport: bool,
}
```

### `ImportKind`

```rust
pub enum ImportKind {
    /// Import do mesmo crate que faz o uso.
    /// Resultado de resolução de `crate::`, `self::`, `super::`,
    /// ou caminhos começando com o próprio nome do crate.
    CurrentCrate,

    /// Import de outro crate dentro do mesmo workspace.
    /// Resultado de caminho começando com nome de outro membro do
    /// workspace (resolvido contra a lista de `WorkspaceMember`s).
    WorkspaceCrate,

    /// Import de crate externo (crates.io ou similar).
    /// Resultado de caminho começando com nome que não é nem
    /// workspace nem stdlib.
    External,

    /// Import da biblioteca padrão.
    /// Caminhos começando com `std::`, `core::`, ou `alloc::`.
    Stdlib,

    /// Caminho que não pôde ser classificado.
    /// Casos: caminhos relativos sem contexto suficiente,
    /// uso de macros que geram imports, ou edge cases não previstos.
    /// Sempre acompanhado de warning emitido em L₃.
    Unresolved,
}
```

---

## Operações em L₁

Esta entidade é primariamente um contentor de dados. Operações
limitadas a inspecção:

```rust
impl ImportEdge {
    /// Constrói uma `ImportEdge` com todos os campos.
    /// Construção literal; sem validação além dos tipos.
    pub fn new(
        from: NodeId,
        from_crate: String,
        target_module: String,
        imported_item: String,
        kind: ImportKind,
        raw_use_path: String,
        is_glob: bool,
        alias: Option<String>,
        is_reexport: bool,
    ) -> Self;

    /// Retorna `true` se o import é interno (CurrentCrate ou
    /// WorkspaceCrate).
    pub fn is_internal(&self) -> bool;

    /// Retorna `true` se o import é externo (External ou Stdlib).
    pub fn is_external(&self) -> bool;
}

impl ImportKind {
    /// Retorna `true` para `CurrentCrate` e `WorkspaceCrate`.
    pub fn is_internal(&self) -> bool;

    /// Retorna `true` para `External` e `Stdlib`.
    pub fn is_external(&self) -> bool;
}
```

Sem operações que mutam estado. Sem operações com I/O.

---

## Invariantes

L₁ não valida invariantes; assume que L₃ entrega dados consistentes.
As seguintes propriedades devem ser garantidas por quem constrói:

1. **Coerência de `is_glob` e `imported_item`**: se
   `is_glob == true`, então `imported_item == "*"`. Se
   `imported_item == "*"`, então `is_glob == true`.

2. **Coerência de `is_glob` e `alias`**: se `is_glob == true`,
   então `alias == None`. Glob não pode ter alias em Rust.

3. **`from_crate` não vazio**: sempre o nome do crate de origem.

4. **`raw_use_path` não vazio**: sempre o caminho textual original.

5. **`Unresolved` implica diagnóstico**: se `kind == Unresolved`,
   L₃ deve ter emitido warning correspondente.

---

## Derives obrigatórios

- `Debug` — todas as structs e enums.
- `Clone` — todas as structs e enums.
- `PartialEq`, `Eq` — todas as structs e enums.
- `Hash` — `ImportEdge` e `ImportKind`.

Sem `Serialize`/`Deserialize` (Passo 1.4 decide).

---

## Dependências externas

Nenhuma adicional. Importa `NodeId` de `module_tree`:

```rust
use crate::entities::module_tree::NodeId;
```

Não usar:
- `syn`, `cargo_metadata`, `serde`, `petgraph`.

---

## Testes esperados

Localização: testes inline com `#[cfg(test)]` em
`01_core/src/entities/import_edge.rs`.

Cobertura mínima:

1. **Construção literal**: criar `ImportEdge` com cada variante
   de `ImportKind`; verificar acesso aos campos.

2. **`is_internal` / `is_external` na `ImportEdge`**:
   - `CurrentCrate`: `is_internal == true`, `is_external == false`.
   - `WorkspaceCrate`: `is_internal == true`, `is_external == false`.
   - `External`: `is_internal == false`, `is_external == true`.
   - `Stdlib`: `is_internal == false`, `is_external == true`.
   - `Unresolved`: ambos `false` (não classificado).

3. **`is_internal` / `is_external` no `ImportKind`**: mesmo
   comportamento, testado isoladamente.

4. **Igualdade**: duas `ImportEdge` com mesmos campos são iguais
   via `PartialEq`.

5. **Hash**: duas `ImportEdge` iguais produzem mesmo hash.

6. **Caso glob**: construir com `is_glob = true`,
   `imported_item = "*"`, `alias = None`. Verificar acesso aos
   campos.

7. **Caso alias**: construir com `is_glob = false`,
   `imported_item = "Foo"`, `alias = Some("Bar".to_string())`.
   Verificar acesso aos campos.

8. **Caso re-export**: construir com `is_reexport = true`.
   Verificar acesso.

---

## Critério de aceitação do prompt

- O ficheiro `01_core/src/entities/import_edge.rs` existe e compila.
- Todas as structs e enums definidos exactamente como especificado.
- Todos os métodos com a assinatura especificada.
- Os 8 grupos de testes passam.
- `cargo clippy -p crystalline-dsm-core` sem warnings.
- Nenhuma importação de `syn`, `cargo_metadata`, `serde` ou
  `petgraph`.
- Entidade exportada via `01_core/src/entities/mod.rs`.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-20 | Implementação da entidade ImportEdge e testes unitários | `01_core/src/entities/import_edge.rs`, `01_core/src/entities/mod.rs`, `01_core/src/entities/module_tree.rs` |

---

## Hash do prompt

A calcular após aprovação.
