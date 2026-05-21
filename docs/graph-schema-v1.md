# `graph.json` — Schema v1.0.0

Este documento descreve o formato canónico do `graph.json`
produzido pelo `crystalline-dsm`. Consumidores externos
(ferramentas que leem o JSON) devem basear-se neste documento.

**Versão**: 1.0.0
**Estabilidade**: estável dentro de versões major iguais. Mudanças
incompatíveis incrementam o major (`2.0.0`, `3.0.0`, etc).

---

## Estrutura geral

O `graph.json` é um objecto JSON com seis campos de topo:

```json
{
  "schema_version": "1.0.0",
  "generated_at": "2026-05-20T22:30:00Z",
  "tool": { ... },
  "workspace": { ... },
  "graph": { ... },
  "cycles": { ... }
}
```

Todos os campos são obrigatórios em qualquer documento válido
desta versão.

---

## Campos

### `schema_version`

- **Tipo**: string.
- **Formato**: semver (`MAJOR.MINOR.PATCH`).
- **Valor para esta versão**: `"1.0.0"`.

Indica a versão do schema. Consumidores devem rejeitar documentos
com major diferente. Documentos com mesmo major e minor/patch
diferentes devem ser aceitos (compatibilidade ascendente dentro
de major).

### `generated_at`

- **Tipo**: string.
- **Formato**: RFC 3339 (ISO 8601 simplificado), sempre em UTC
  com sufixo `Z`.
- **Exemplo**: `"2026-05-20T22:30:00Z"`.

Timestamp do momento em que o documento foi gerado.

### `tool`

Objecto com informação da ferramenta que gerou o documento.

```json
{
  "name": "crystalline-dsm",
  "version": "0.1.0"
}
```

- **`name`**: string, sempre `"crystalline-dsm"` nesta
  implementação.
- **`version`**: string, versão da ferramenta no formato semver.

### `workspace`

Objecto com informação do workspace analisado.

```json
{
  "root": "/abs/path/to/workspace",
  "members": ["crate-a", "crate-b", "crate-c"]
}
```

- **`root`**: string, caminho absoluto da raiz do workspace.
- **`members`**: array de strings, nomes dos crates do workspace.
  **Ordem**: alfabética ascendente.

### `graph`

Objecto contendo o grafo propriamente dito.

```json
{
  "nodes": [ ... ],
  "edges": [ ... ]
}
```

#### `graph.nodes`

Array de objectos `NodeDto`. **Ordem**: alfabética por
`canonical_path` ascendente.

Cada nó tem dois campos sempre presentes:

- **`canonical_path`**: string, identificador único do nó. Para
  internos: `<crate_name>::<modulo>::<submodulo>`. Para externos:
  caminho lógico como escrito no `use` (ex: `"serde::de"`,
  `"std::collections"`).
- **`kind`**: string com valor `"internal"` ou `"external"`.

Campos condicionais:

- **Se `kind == "internal"`**:
  - **`crate_name`**: string, obrigatória.
- **Se `kind == "external"`**:
  - **`external_kind`**: string com valor `"crate"` ou
    `"stdlib"`, obrigatória.

Exemplo de nó interno:
```json
{
  "canonical_path": "crystalline_dsm_core::entities::workspace",
  "kind": "internal",
  "crate_name": "crystalline-dsm-core"
}
```

Exemplo de nó externo:
```json
{
  "canonical_path": "serde::de",
  "kind": "external",
  "external_kind": "crate"
}
```

#### `graph.edges`

Array de objectos `EdgeDto`. **Ordem**: lexicográfica por
`(from, to, imported_item)`.

Cada aresta tem sete campos:

- **`from`**: string, `canonical_path` do nó origem. Sempre
  corresponde a um nó interno presente em `graph.nodes`.
- **`to`**: string, `canonical_path` do nó destino. Pode ser
  interno ou externo; sempre presente em `graph.nodes`.
- **`imported_item`**: string, item importado. Para imports
  glob: `"*"`. Para imports de módulo (`use a::b;`): nome do
  módulo (`"b"`).
- **`alias`**: string ou `null`. Sempre presente (mesmo `null`).
- **`is_reexport`**: booleano. `true` se é `pub use`.
- **`is_glob`**: booleano. `true` se é `use a::b::*`. Quando
  `true`, `imported_item == "*"`.
- **`raw_use_path`**: string, caminho do `use` exactamente como
  aparece no código fonte (preserva `crate::`, `self::`,
  `super::`).

Exemplo:
```json
{
  "from": "crystalline_dsm_core::entities::workspace",
  "to": "std::collections",
  "imported_item": "HashMap",
  "alias": null,
  "is_reexport": false,
  "is_glob": false,
  "raw_use_path": "std::collections::HashMap"
}
```

**Notas**:
- Múltiplas arestas entre o mesmo par `(from, to)` são
  permitidas e comuns. Cada `use` individual gera uma aresta.
- Uma aresta de `use a::{X, Y, Z}` é dividida em 3 arestas
  separadas (uma por item importado).

### `cycles`

Objecto com informação sobre ciclos detectados no grafo.

```json
{
  "count": 3,
  "self_loop_count": 0,
  "multi_node_count": 3,
  "items": [ ... ]
}
```

- **`count`**: inteiro, total de ciclos.
- **`self_loop_count`**: inteiro, ciclos com 1 só nó (self-loops).
- **`multi_node_count`**: inteiro, ciclos com 2+ nós.
- **`count == self_loop_count + multi_node_count`** sempre.

#### `cycles.items`

Array de objectos `CycleDto`. **Ordem**: tamanho decrescente,
secundariamente alfabética pelo primeiro nó.

Cada ciclo tem dois campos:

- **`kind`**: string, `"multi_node"` ou `"self_loop"`.
- **`nodes`**: array de strings, `canonical_path`s dos nós
  participantes. **Ordem dentro do array**: alfabética
  ascendente.

Exemplo:
```json
{
  "kind": "multi_node",
  "nodes": ["a::b", "a::c", "a::d"]
}
```

Self-loops têm `nodes` com exactamente 1 elemento.

---

## Validação de documento

Um documento é considerado válido se:

1. `schema_version` é semver válido com major == `"1"`.
2. Todos os campos obrigatórios estão presentes.
3. Em cada nó: o `kind` casa com os campos condicionais
   correctos.
4. Em cada aresta: `from` e `to` correspondem a nós existentes
   em `graph.nodes`.
5. Em cada ciclo: todos os `canonical_path`s em `nodes`
   correspondem a nós existentes.
6. Ordens canónicas (alfabética em `members`, `nodes`, dentro de
   cada ciclo; lexicográfica em `edges`; por tamanho em
   `cycles.items`) são respeitadas.

A invalidação por ordem canónica é apenas para documentos
gerados pelo `crystalline-dsm`. Consumidores podem aceitar
documentos com ordem diferente vindos de outras fontes; é
responsabilidade do consumidor decidir.

---

## Determinismo

Para o mesmo input (workspace, código fonte), execuções
diferentes do `crystalline-dsm` produzem o mesmo `graph.json`
byte-a-byte, excepto pelos campos:

- `generated_at` (sempre actual).
- `tool.version` (mudará entre versões da ferramenta).

O resto do documento é byte-determinístico.

---

## Exemplo completo (mínimo)

Para um workspace com 1 crate (`my-crate`) contendo um único
módulo que importa `std::io::Read`:

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
  "graph": {
    "nodes": [
      {
        "canonical_path": "my_crate",
        "kind": "internal",
        "crate_name": "my-crate"
      },
      {
        "canonical_path": "std::io",
        "kind": "external",
        "external_kind": "stdlib"
      }
    ],
    "edges": [
      {
        "from": "my_crate",
        "to": "std::io",
        "imported_item": "Read",
        "alias": null,
        "is_reexport": false,
        "is_glob": false,
        "raw_use_path": "std::io::Read"
      }
    ]
  },
  "cycles": {
    "count": 0,
    "self_loop_count": 0,
    "multi_node_count": 0,
    "items": []
  }
}
```

---

## Schema JSON formal (referência)

Um schema JSON formal (draft 2020-12) pode ser disponibilizado
em versão futura. Por ora, esta documentação prosa é a
referência canónica.

---

## Histórico

| Versão | Data | Mudanças |
|--------|------|----------|
| 1.0.0 | 2026-05-20 | Versão inicial. |
