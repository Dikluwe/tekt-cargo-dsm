# Prompt L0: Extrator de Imports (L₃)

**Camada**: L₃ (Infraestrutura)
**Ficheiro alvo**: `03_infra/src/import_extractor.rs`
**Passo do roadmap**: 1.3 — Extracção de imports (`use`)
**Status**: IMPLEMENTADO

---

## Decisões de design prévias

- **ADR-0002**: `#[cfg]` ignorado. Vale também para `use` statements
  sob `#[cfg]`: todos são processados.
- **ADR-0004**: Identidade de módulo é caminho lógico canónico.
- **Conversa 2026-05-20**: Cinco categorias de `ImportKind`, use
  lists geram arestas separadas, registar item importado e caminho
  do módulo.

---

## Decisões locais (assumidas neste prompt)

1. **Nova função dedicada, não alterar `traverse_crate`**: o
   código do Passo 1.2 (`module_traverser`) permanece intacto.
   Este passo cria uma função separada que faz a sua própria
   leitura e parsing de ficheiros. Reparsing é considerado custo
   aceitável para manter a independência dos passos
   `IMPLEMENTADO`s.

2. **Função opera por crate**: recebe um `WorkspaceMember` e o
   `ModuleTree` correspondente (já construído pelo Passo 1.2);
   retorna `Vec<ImportEdge>`.

3. **Resolução cross-crate é feita aqui**: a função recebe
   também a lista de nomes de crates do workspace, para
   classificar `WorkspaceCrate` vs `External`. Sem isso, a
   classificação ficaria sempre `External` ou `Unresolved`.

4. **Não resolver para `NodeId` do alvo**: o alvo é registado
   como string (`target_module`), não como `NodeId`. A
   resolução para `NodeId` (quando aplicável) é trabalho do
   Passo 1.4.

5. **Imports em itens não-`use`**: ignorar. Apenas `syn::Item::Use`
   é processado. Imports implícitos via prelúdio, macros, ou
   path qualification (`SomeStruct::method()`) NÃO são
   detectados.

---

## Contexto

Este módulo consome:
- Um `WorkspaceMember` (do Passo 1.1) — para saber qual crate
  estamos a analisar.
- O `ModuleTree` correspondente (do Passo 1.2) — para saber a
  estrutura de módulos e mapear cada ficheiro físico ao `NodeId`
  do módulo origem.
- A lista de nomes de crates do workspace — para classificação
  de `WorkspaceCrate`.

Produz:
- Um `Vec<ImportEdge>` contendo todas as arestas de import
  extraídas de todos os módulos do crate.

---

## Função pública principal

```rust
pub fn extract_imports(
    member: &WorkspaceMember,
    tree: &ModuleTree,
    workspace_crate_names: &[String],
) -> Result<Vec<ImportEdge>, ExtractError>;
```

### Comportamento

1. Para cada nó `(node_id, module_node)` em `tree.all_nodes()`:
   a. Se `module_node.is_inline == true`: o ficheiro é o do pai
      (já será lido quando processarmos o módulo que o contém).
      Tratamento alternativo: ler o ficheiro do pai mas restringir
      ao corpo do módulo inline. Ver "Tratamento de inline" abaixo.
   b. Se `is_inline == false`: ler `module_node.source_file`,
      parsear com `syn`, extrair `Item::Use`.

2. Para cada `Item::Use` encontrado:
   a. Determinar `is_reexport` (presença de `pub`/`pub(crate)`/
      `pub(super)` antes de `use`).
   b. Expandir a árvore de `UseTree` em items atómicos
      (ver "Expansão de UseTree").
   c. Para cada item atómico:
      - Construir `raw_use_path` (string original do caminho).
      - Resolver caminho relativo (`self::`, `super::`, `crate::`).
      - Classificar com `classify_import_kind`.
      - Construir `ImportEdge` e adicionar ao resultado.

3. Retornar o `Vec<ImportEdge>` completo.

### Tratamento de módulos inline

Há duas estratégias possíveis para extrair imports de módulos
inline (`mod foo { use a::b; }`):

**Estratégia A (mais simples)**: ao processar o ficheiro de um
módulo, descer recursivamente em qualquer `Item::Mod` com corpo
inline e extrair os `use` desses módulos, atribuindo as arestas
ao `NodeId` correcto.

**Estratégia B (mais separada)**: processar apenas os items do
nível superior de cada ficheiro. Os módulos inline já foram
adicionados ao `ModuleTree` no Passo 1.2 com `source_file` =
ficheiro do pai; ao processar esse ficheiro, descer em todos os
`mod` inline.

Recomendação: **Estratégia B com travessia explícita**. Ao
processar `source_file` do nó `parent_node`, percorrer o AST e
para cada `Item::Use` registar aresta com `from = parent_node`;
para cada `Item::Mod` com corpo, descer recursivamente atribuindo
arestas ao `NodeId` correcto (encontrado via
`tree.find_by_canonical_path` ou similar).

Esta estratégia exige saber, dentro do AST, qual o `canonical_path`
do módulo onde estamos. Implementar via passagem de contexto na
recursão.

### Expansão de `UseTree`

A árvore `syn::UseTree` modela `use` statements complexos. Casos:

- `UseTree::Path(p)` — segmento de caminho seguido de subárvore.
  Ex: `use a::b::Foo;` é `Path(a, Path(b, Name(Foo)))`.

- `UseTree::Name(n)` — nome simples no fim. Ex: o `Foo` acima.

- `UseTree::Rename(r)` — nome com alias. Ex: `use a as b;`.

- `UseTree::Glob(_)` — `*`. Ex: `use a::*;`.

- `UseTree::Group(g)` — `{X, Y, Z}`. Ex: `use a::{X, Y};`.

A expansão produz uma lista de items atómicos, cada um com:
- Caminho até ao item (prefixo acumulado).
- Nome do item (ou `"*"` para glob).
- Alias se houver.

Exemplos:

| Use statement | Items atómicos extraídos |
|---|---|
| `use a::b::Foo;` | `(a::b, Foo, None)` |
| `use a::b::Foo as Bar;` | `(a::b, Foo, Some("Bar"))` |
| `use a::b::*;` | `(a::b, *, None)` (com `is_glob=true`) |
| `use a::{X, Y};` | `(a, X, None)`, `(a, Y, None)` |
| `use a::{X, Y as Z, c::W};` | `(a, X, None)`, `(a, Y, Some("Z"))`, `(a::c, W, None)` |
| `use a::{self, X};` | `(_, a, None)` (modulo a), `(a, X, None)` |

O caso `self` em `use a::{self, X};` significa "importar o módulo
`a` em si". Tratamento: emitir aresta com `imported_item = "a"`,
`target_module = ""` (vazio, ou caminho do pai).

### Resolução de `crate::`, `self::`, `super::`

Antes de classificar, normalizar o caminho relativo:

- **`crate::a::b`** → `<from_crate>::a::b`, classificação
  `CurrentCrate`.

- **`self::a::b`** → `<from_module>::a::b`, classificação
  `CurrentCrate`. Onde `from_module` é o `canonical_path` do nó
  origem.

- **`super::a::b`** → caminho do pai do `from_module` + `::a::b`.
  Se o pai não existir (já estamos na raiz), classificação
  `Unresolved` com warning.

- **`super::super::a`** → subir dois níveis. Cada `super` é uma
  ascensão.

### Classificação (`classify_import_kind`)

Função privada:

```rust
fn classify_import_kind(
    first_segment: &str,
    from_crate: &str,
    workspace_crate_names: &[String],
) -> ImportKind;
```

Lógica:

1. Se `first_segment == from_crate`: `CurrentCrate`.
2. Se `first_segment` está em `workspace_crate_names` (excluindo o
   próprio crate): `WorkspaceCrate`.
3. Se `first_segment` é `"std"`, `"core"`, ou `"alloc"`: `Stdlib`.
4. Caso contrário: `External`.

Notas:
- A normalização de `crate::`/`self::`/`super::` deve ser feita
  ANTES de chamar `classify_import_kind` (o primeiro segmento já
  é o caminho normalizado).
- Caminhos absolutos começando com `::` (raros, edição 2018+ usa
  pouco) são tratados como `External` por defeito.

---

## Função auxiliar interna

```rust
fn extract_from_file(
    file_path: &Path,
    parent_node: NodeId,
    parent_canonical_path: &str,
    tree: &ModuleTree,
    from_crate: &str,
    workspace_crate_names: &[String],
    output: &mut Vec<ImportEdge>,
) -> Result<(), ExtractError>;
```

Responsável por processar um ficheiro completo: parsear, percorrer
items, descer em `mod` inline, extrair `use`.

---

## Tipo de erro

```rust
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("Falha ao ler ficheiro: {path}")]
    FileReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Falha ao parsear ficheiro Rust: {file}")]
    ParseFailed {
        file: PathBuf,
        #[source]
        source: syn::Error,
    },

    #[error("Caminho 'super' resolveu para fora do crate em {from_module}")]
    SuperOutOfBounds {
        from_module: String,
        raw_use_path: String,
    },
}
```

`SuperOutOfBounds` é diagnóstico; quando ocorre, a aresta é
construída com `kind: Unresolved` e o warning emitido, mas a
extracção continua. **A função não retorna `Err` por
`SuperOutOfBounds`**; só por falhas de I/O ou parsing.

Decisão: o tipo de erro listado inclui `SuperOutOfBounds` como
documentação dos casos diagnosticados, mas a função NÃO o devolve
como erro propagado. Considerar remover do enum se ficar
estranho; manter por documentação por ora.

---

## Dependências externas

Declaradas em `03_infra/Cargo.toml`:
- `syn` (com feature `full`, já presente).
- `thiserror` (já presente).

Internas:
- `crystalline-dsm-core` para `ImportEdge`, `ImportKind`,
  `NodeId`, `ModuleTree`, `WorkspaceMember`.

---

## Testes esperados

### Testes unitários (no próprio ficheiro)

Limitados a funções puras isoláveis:

1. **`classify_import_kind`**: tabela de casos cobrindo todas as
   5 categorias com inputs construídos manualmente.

2. **Resolução de `super::`**: dado um `canonical_path`,
   normalizar `super::x` correctamente. Caso de excesso de
   `super` retorna erro ou `Unresolved`.

### Testes de integração (`03_infra/tests/import_extractor_tests.rs`)

Usando fixtures novas em `tests/fixtures/`:

1. **`imports-simple`**: crate com `use a::b::Foo;` no `lib.rs`.
   Resultado: 1 aresta com `target_module = "a::b"`,
   `imported_item = "Foo"`, `kind = External` (a vem de fora).

2. **`imports-current-crate`**: crate com `use crate::utils::helper;`.
   Resultado: 1 aresta com `kind = CurrentCrate`,
   `target_module = "<crate_name>::utils"`,
   `imported_item = "helper"`.

3. **`imports-self`**: `use self::a::b;` dentro de um submódulo.
   Resultado: `kind = CurrentCrate`, caminho resolvido contra o
   submódulo onde aparece.

4. **`imports-super`**: `use super::a::b;` dentro de submódulo.
   Resultado: caminho do pai + `::a::b`, `kind = CurrentCrate`.

5. **`imports-super-out-of-bounds`**: `use super::x;` na raiz do
   crate. Resultado: `kind = Unresolved`, warning emitido.

6. **`imports-stdlib`**: `use std::collections::HashMap;`.
   Resultado: `kind = Stdlib`.

7. **`imports-workspace`**: workspace com 2 crates, crate A
   importa de crate B (`use crate_b::foo::Bar;`).
   Resultado: `kind = WorkspaceCrate`.

8. **`imports-use-list`**: `use a::{X, Y, Z};`.
   Resultado: 3 arestas separadas.

9. **`imports-glob`**: `use a::b::*;`.
   Resultado: 1 aresta com `is_glob = true`,
   `imported_item = "*"`.

10. **`imports-alias`**: `use a::Foo as Bar;`.
    Resultado: 1 aresta com `imported_item = "Foo"`,
    `alias = Some("Bar")`.

11. **`imports-reexport`**: `pub use a::Foo;`.
    Resultado: 1 aresta com `is_reexport = true`.

12. **`imports-inline-module`**: `mod sub { use a::b::Foo; }` no
    `lib.rs`.
    Resultado: 1 aresta com `from` = `NodeId` do módulo `sub`
    (não da raiz).

13. **`imports-self-in-group`**: `use a::{self, X};`.
    Resultado: 2 arestas. A primeira importa o módulo `a` em si
    (`imported_item = "a"`), a segunda importa `X`.

---

## Estrutura das fixtures novas

```
tests/fixtures/
├── (fixtures existentes)
├── imports-simple/
│   ├── Cargo.toml
│   └── src/lib.rs        (use a::b::Foo;)
├── imports-current-crate/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs        (mod utils; use crate::utils::helper;)
│       └── utils.rs      (pub fn helper() {})
├── imports-self/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs        (mod outer;)
│       └── outer/
│           ├── mod.rs    (mod inner; use self::inner::Foo;)
│           └── inner.rs  (pub struct Foo;)
├── imports-super/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs        (mod a; pub struct Foo;)
│       └── a.rs          (use super::Foo;)
├── imports-super-out-of-bounds/
│   ├── Cargo.toml
│   └── src/lib.rs        (use super::x;)
├── imports-stdlib/
│   ├── Cargo.toml
│   └── src/lib.rs        (use std::collections::HashMap;)
├── imports-workspace/
│   ├── Cargo.toml        (members = ["crate_a", "crate_b"])
│   ├── crate_a/
│   │   ├── Cargo.toml    (deps: crate_b)
│   │   └── src/lib.rs    (use crate_b::foo::Bar;)
│   └── crate_b/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs    (pub mod foo;)
│           └── foo.rs    (pub struct Bar;)
├── imports-use-list/
│   ├── Cargo.toml
│   └── src/lib.rs        (use a::{X, Y, Z};)
├── imports-glob/
│   ├── Cargo.toml
│   └── src/lib.rs        (use a::b::*;)
├── imports-alias/
│   ├── Cargo.toml
│   └── src/lib.rs        (use a::Foo as Bar;)
├── imports-reexport/
│   ├── Cargo.toml
│   └── src/lib.rs        (pub use a::Foo;)
├── imports-inline-module/
│   ├── Cargo.toml
│   └── src/lib.rs        (mod sub { use a::b::Foo; })
└── imports-self-in-group/
    ├── Cargo.toml
    └── src/lib.rs        (use a::{self, X};)
```

Para fixtures que importam crates externas (`a::b::Foo`, etc), o
crate `a` não precisa existir realmente — não compilamos as
fixtures contra dependências externas no MVP. O parser apenas vê o
texto. Marcar nos `Cargo.toml` que estas fixtures podem ter
warnings de "crate não encontrada" e isso é OK.

Alternativa: declarar as dependências fictícias usando
`[dependencies] a = { path = "../mock-a" }` apontando para
fixtures-suporte. Mais limpo mas adiciona complexidade. Decisão
para o MVP: deixar como crate externa não-existente; os testes
não compilam as fixtures, apenas leem o texto.

---

## Critério de aceitação do prompt

- O ficheiro `03_infra/src/import_extractor.rs` existe e compila.
- A função `extract_imports` tem a assinatura especificada.
- O enum `ExtractError` está definido.
- As 13 fixtures novas existem em `tests/fixtures/`.
- Os 13 testes de integração passam.
- `cargo clippy --all-targets` sem warnings novos.
- Nenhum `panic!`, `unwrap()` ou `expect()` em código de produção.
- O módulo não exporta tipos de `syn` na API pública.
- Módulo exportado em `03_infra/src/lib.rs`.

---

## Limitações conhecidas e documentadas

1. Macros que geram `use` não são detectadas (limitação herdada
   do uso de `syn`).
2. Imports implícitos via prelúdio (`Option`, `Vec`, etc) não são
   registados.
3. Path qualification (`SomeStruct::method()`) não conta como
   import.
4. `use ::absolute::path;` (caminho absoluto explícito) é tratado
   como `External`.
5. `pub(crate) use` e `pub(super) use` contam como re-export
   (mesma flag `is_reexport`); a distinção de visibilidade não é
   modelada no MVP.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-20 | Implementação do extrator de imports e testes de integração | `03_infra/src/import_extractor.rs`, `03_infra/src/lib.rs`, `03_infra/tests/import_extractor_tests.rs`, `tests/fixtures/imports-*` |

---

## Hash do prompt

A calcular após aprovação.
