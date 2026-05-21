# `trees.json` — Schema v1.0.0

Este documento descreve o formato canónico do `trees.json`,
artefacto auxiliar produzido pelo `crystalline-dsm` quando
invocado com a flag `--emit-trees`. Consumidores externos que
queiram navegar do `graph.json` para ficheiros fonte devem ler o
`trees.json` em paralelo e casar entradas por `canonical_path`.

**Versão**: 1.0.0
**Estabilidade**: estável dentro de versões major iguais.

---

## Relação com `graph.json`

O `trees.json` é independente do `graph.json`:

- Cada um tem `schema_version` próprio.
- Cada um pode evoluir separadamente.
- A ponte entre os dois é a chave `canonical_path` (campo
  presente em ambos os documentos).

Para um consumidor que quer "dado um nó do grafo, encontrar o
seu ficheiro fonte":

1. Ler `node.canonical_path` no `graph.json`.
2. Procurar no `trees.json` o `ModuleNode` com o mesmo
   `canonical_path`.
3. Usar o campo `source_file` desse `ModuleNode`.

---

## Estrutura geral

```json
{
  "schema_version": "1.0.0",
  "generated_at": "2026-05-20T22:30:00Z",
  "tool": { ... },
  "workspace": { ... },
  "trees": [ ... ]
}
```

Todos os campos são obrigatórios.

Os campos `schema_version`, `generated_at`, `tool` e `workspace`
têm a mesma semântica do `graph.json` (ver `graph-schema-v1.md`).
O `schema_version` aqui é independente do schema do grafo: pode
divergir.

---

## `trees`

Array de objectos `ModuleTreeDto`, um por crate do workspace.
**Ordem**: alfabética por `crate_name` ascendente.

Cada elemento:

```json
{
  "crate_name": "my-crate",
  "nodes": [ ... ]
}
```

- **`crate_name`**: string, nome do crate. Casa com o
  `crate_name` dos nós do `graph.json` que pertencem a este
  crate.
- **`nodes`**: array de `ModuleNodeDto`, todos os módulos da
  árvore deste crate. **Ordem**: pre-order (raiz primeiro,
  depois filhos recursivamente).

A ordem pre-order é essencial para reconstrução: ao deserializar,
cada nó (excepto a raiz) referencia o pai por `canonical_path`,
e o pai precisa já existir no momento da inserção do filho.

---

## `ModuleNodeDto`

Cada elemento de `tree.nodes`:

```json
{
  "canonical_path": "my_crate::utils::helper",
  "crate_name": "my-crate",
  "module_path": ["utils", "helper"],
  "source_file": "/abs/path/to/src/utils/helper.rs",
  "is_inline": false,
  "has_custom_path": false,
  "parent_canonical_path": "my_crate::utils"
}
```

### Campos

- **`canonical_path`**: string, identificador único do módulo
  dentro do workspace inteiro. Mesmo formato dos nós internos do
  `graph.json`.

- **`crate_name`**: string, nome do crate ao qual este módulo
  pertence.

- **`module_path`**: array de strings, segmentos do caminho do
  módulo dentro do crate (sem o prefixo do `crate_name`). Vazio
  para o nó raiz do crate.

- **`source_file`**: string, caminho absoluto do ficheiro físico
  que contém este módulo. Para módulos inline, o caminho é o do
  ficheiro do módulo pai (porque o módulo inline vive no mesmo
  ficheiro).

- **`is_inline`**: booleano. `true` se o módulo foi declarado
  inline (`mod foo { ... }`), `false` se tem ficheiro próprio.

- **`has_custom_path`**: booleano. `true` se o módulo foi
  declarado com atributo `#[path = "..."]`, indicando que o
  caminho do ficheiro não segue a convenção padrão.

- **`parent_canonical_path`**: string ou `null`. `canonical_path`
  do módulo pai. `null` apenas para o nó raiz da árvore (o módulo
  do `lib.rs` ou `main.rs` do crate). Para todos os outros nós,
  deve referenciar um nó que apareça **antes** deste no array
  `nodes` (regra de pre-order).

---

## Exemplo completo (mínimo)

Para um crate com hierarquia:
```
my_crate (lib.rs)
└── utils (utils.rs)
    └── helper (utils/helper.rs)
```

Onde `my-crate/src/utils.rs` declara `mod helper;`:

```json
{
  "schema_version": "1.0.0",
  "generated_at": "2026-05-20T22:30:00Z",
  "tool": {
    "name": "crystalline-dsm",
    "version": "0.1.0"
  },
  "workspace": {
    "root": "/home/user/my-workspace",
    "members": ["my-crate"]
  },
  "trees": [
    {
      "crate_name": "my-crate",
      "nodes": [
        {
          "canonical_path": "my_crate",
          "crate_name": "my-crate",
          "module_path": [],
          "source_file": "/home/user/my-workspace/my-crate/src/lib.rs",
          "is_inline": false,
          "has_custom_path": false,
          "parent_canonical_path": null
        },
        {
          "canonical_path": "my_crate::utils",
          "crate_name": "my-crate",
          "module_path": ["utils"],
          "source_file": "/home/user/my-workspace/my-crate/src/utils.rs",
          "is_inline": false,
          "has_custom_path": false,
          "parent_canonical_path": "my_crate"
        },
        {
          "canonical_path": "my_crate::utils::helper",
          "crate_name": "my-crate",
          "module_path": ["utils", "helper"],
          "source_file": "/home/user/my-workspace/my-crate/src/utils/helper.rs",
          "is_inline": false,
          "has_custom_path": false,
          "parent_canonical_path": "my_crate::utils"
        }
      ]
    }
  ]
}
```

---

## Validação de documento

Um documento é considerado válido se:

1. `schema_version` é semver válido com major == `"1"`.
2. Todos os campos obrigatórios estão presentes.
3. Em cada `tree.nodes`:
   - O primeiro elemento tem `parent_canonical_path == null`
     (é a raiz).
   - Apenas um elemento pode ter `parent_canonical_path == null`
     dentro da mesma árvore.
   - Cada `parent_canonical_path` não-nulo referencia um
     `canonical_path` que aparece **antes** dele na lista.
   - Cada `canonical_path` é único dentro da árvore.
4. Ordem alfabética em `trees` (por `crate_name`).

---

## Casos especiais

### Módulos inline

Um módulo declarado `mod foo { ... }` partilha o `source_file`
com o pai. Exemplo:

```json
{
  "canonical_path": "my_crate::tests",
  "crate_name": "my-crate",
  "module_path": ["tests"],
  "source_file": "/abs/path/src/lib.rs",
  "is_inline": true,
  "has_custom_path": false,
  "parent_canonical_path": "my_crate"
}
```

O `source_file` aponta para `lib.rs` (do pai), e `is_inline:
true` sinaliza a natureza do módulo.

### Módulos com `#[path]`

Quando um módulo é declarado com `#[path = "x.rs"]`, o
`source_file` aponta para o ficheiro real (não o local padrão),
e `has_custom_path: true` sinaliza.

### Crates sem nenhum módulo (raiz só)

A árvore tem apenas 1 elemento (o nó raiz com `module_path: []`).
Comum em crates muito simples ou bibliotecas com tudo em
`lib.rs`.

### Crates do tipo `TestsOnly`, `ProcMacro`, etc

Crates classificados como `TestsOnly` (apenas testes integrados,
sem `lib.rs`/`main.rs`) ainda aparecem em `trees`, com a árvore
construída a partir do primeiro `test_path` do
`EntryKind::TestsOnly`. O `source_file` da raiz aponta para esse
ficheiro de teste.

Crates `ProcMacro` aparecem normalmente, com `source_file` da
raiz apontando para o `lib.rs`.

Crates `NoSourceTarget` (sem nenhum ponto de entrada) aparecem em
`trees` com a árvore contendo apenas um nó raiz simbólico cujo
`source_file` é a string vazia (`""`). A decisão de manter o nó
raiz vem do `module_traverser` (ver ADR-0007/0008), para que
nenhum crate do workspace seja silenciosamente omitido.

---

## Determinismo

Tal como o `graph.json`, o `trees.json` é byte-determinístico
para o mesmo input, excepto pelos campos `generated_at` e
`tool.version`.

---

## Histórico

| Versão | Data | Mudanças |
|--------|------|----------|
| 1.0.0 | 2026-05-20 | Versão inicial. |
