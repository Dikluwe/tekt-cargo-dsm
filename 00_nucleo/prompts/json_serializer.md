# Prompt L0: `json_serializer` (L₃)

**Camada**: L₃ (Infraestrutura)
**Ficheiro alvo**: `03_infra/src/json_serializer.rs`
**Passo do roadmap**: 1.4 (parte JSON) — completar M1
**Status**: IMPLEMENTADO (revisado)
**ADR motivadora**: ADR-0009 (Serialização em L₃ via DTOs)
**Revisão**: ADR-0010 — `to_dto` mapeia tanto `InternalWithTree` quanto `InternalWithoutTree` para o mesmo `NodeKindDto::Internal`; `from_dto` reconstrói sempre como `InternalWithoutTree`. `NodeId::placeholder()` deixa de ser usado (foi removido de L₁).

---

## Decisões de design prévias (registadas em ADR)

- **ADR-0009**: serialização em L₃ via DTOs. L₁ não conhece JSON.
- **ADR-0006**: nós externos têm `kind: "external"` no JSON,
  com `external_kind` diferenciando `crate` e `stdlib`.
- **ADR-0004**: identidade no JSON é `canonical_path`.

---

## Decisões locais (assumidas neste prompt)

1. **Schema versionado**: `schema_version: "1.0.0"` no topo.
   Mudanças no formato seguem semver.

2. **Pretty-print sempre**: usar `serde_json::to_string_pretty`.
   Sem modo compacto no MVP.

3. **Ordem canónica via `BTreeMap`**: quando DTOs contiverem
   estruturas de mapa, usar `BTreeMap` em vez de `HashMap` para
   garantir ordem alfabética determinística.

4. **Arrays ordenados explicitamente**: nós e arestas no JSON
   aparecem em ordem determinística (por `canonical_path`
   alfabético para nós; pelos `canonical_path`s de origem e
   destino para arestas).

5. **Timestamp e versão da tool são parâmetros**: a função
   `to_canonical_json` não consulta o relógio do sistema nem
   procura a versão do binário. Esses dados vêm como argumento.
   Razão: tornar a função determinística e testável (mesma input
   sempre produz mesmo output).

6. **`tree_node_id` não aparece no JSON**: é índice interno do
   `petgraph`, instável entre construções. Round-trip reconstrói
   novos IDs preservando semântica via `canonical_path`.

---

## Contexto

Este módulo é o produto canónico do M1: transforma o
`DependencyGraph` + `CycleReport` num documento JSON serializável
que outras ferramentas (renderizador HTML do Passo 2.2, scripts
externos, etc) consomem.

A serialização é desacoplada do modelo interno: L₁ continua sem
conhecer JSON. As estruturas DTO vivem inteiramente em L₃, com
derive `Serialize`/`Deserialize`.

---

## DTOs (definidos em L₃)

### `GraphJsonDto`

DTO de topo. Reúne tudo o que vai para o JSON.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphJsonDto {
    pub schema_version: String,
    pub generated_at: String,         // RFC 3339 timestamp
    pub tool: ToolInfoDto,
    pub workspace: WorkspaceInfoDto,
    pub graph: GraphDataDto,
    pub cycles: CyclesDto,
}
```

### `ToolInfoDto`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolInfoDto {
    pub name: String,      // sempre "crystalline-dsm"
    pub version: String,   // versão da ferramenta
}
```

### `WorkspaceInfoDto`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceInfoDto {
    pub root: String,          // path absoluto da raiz
    pub members: Vec<String>,  // nomes dos crates, ordem alfabética
}
```

### `GraphDataDto`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphDataDto {
    pub nodes: Vec<NodeDto>,
    pub edges: Vec<EdgeDto>,
}
```

### `NodeDto`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeDto {
    pub canonical_path: String,
    pub kind: NodeKindDto,
    // Campos condicionais conforme kind:
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub crate_name: Option<String>,           // só para internal
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub external_kind: Option<ExternalKindDto>, // só para external
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeKindDto {
    Internal,
    External,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExternalKindDto {
    Crate,
    Stdlib,
}
```

Nota sobre `skip_serializing_if`: campos opcionais que dependem
do `kind` são omitidos do JSON quando ausentes. Mantém o output
limpo sem `null`s ruidosos. Excepção: `alias` em `EdgeDto`
**sempre aparece** (mesmo como `null`) para previsibilidade do
consumidor.

### `EdgeDto`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdgeDto {
    pub from: String,           // canonical_path do nó origem
    pub to: String,              // canonical_path do nó destino
    pub imported_item: String,
    pub alias: Option<String>,   // SEMPRE serializado (mesmo null)
    pub is_reexport: bool,
    pub is_glob: bool,
    pub raw_use_path: String,
}
```

### `CyclesDto`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CyclesDto {
    pub count: usize,
    pub self_loop_count: usize,
    pub multi_node_count: usize,
    pub items: Vec<CycleDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CycleDto {
    pub kind: CycleKindDto,
    pub nodes: Vec<String>,    // canonical_paths
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CycleKindDto {
    MultiNode,
    SelfLoop,
}
```

---

## Funções públicas

### Serialização

```rust
pub fn to_canonical_json(
    graph: &DependencyGraph,
    cycles: &CycleReport,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
) -> Result<String, JsonSerializeError>;
```

Comportamento:

1. Construir `GraphJsonDto` a partir dos inputs via `to_dto`.
2. Ordenar `nodes` por `canonical_path` alfabético.
3. Ordenar `edges` por `(from, to, imported_item)` lexicográfico.
4. Ordenar `cycles.items` por:
   - Tamanho decrescente (`nodes.len()`).
   - Em empate, `canonical_path` do primeiro nó alfabético.
5. Serializar via `serde_json::to_string_pretty`.

A função NÃO consulta o relógio do sistema. `generated_at` é
parâmetro, formato RFC 3339 (ex: `"2026-05-20T14:32:18Z"`).

### Deserialização

```rust
pub fn from_canonical_json(
    json: &str,
) -> Result<(DependencyGraph, CycleReport, GraphJsonDto), JsonDeserializeError>;
```

Comportamento:

1. Parsear `json` em `GraphJsonDto`.
2. Verificar `schema_version`:
   - `"1.0.0"`: aceitar.
   - Outras versões major: rejeitar com erro descritivo.
   - Outras versões minor/patch: aceitar com warning silencioso
     (ou via tracing).
3. Reconstruir `DependencyGraph`:
   - Iterar `nodes`, chamar `add_internal_node` ou
     `add_external_node` conforme `kind`.
   - Iterar `edges`, resolver `from` e `to` para `GraphNodeId`
     via `find_node`, chamar `add_edge`.
4. Reconstruir `CycleReport` a partir de `cycles.items`,
   resolvendo `canonical_path`s para `GraphNodeId`s.
5. Retornar tuplo `(grafo, ciclos, DTO original)`. O DTO é
   retornado também para o caller ter acesso a metadados
   (timestamp, versão, etc) que não cabem no grafo.

Notas importantes:

- A reconstrução do `DependencyGraph` produz IDs internos
  **diferentes** dos originais. Apenas a semântica é preservada.
- Em `NodeDto::Internal`, o `tree_node_id` original não é
  recuperado (não está no JSON). Durante reconstrução, é
  substituído por `NodeId(0)` ou `NodeId` sentinel. Documentado.

### Conversão DTO ↔ Domínio

Funções internas (`pub(crate)` ou privadas):

```rust
pub(crate) fn to_dto(
    graph: &DependencyGraph,
    cycles: &CycleReport,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
) -> GraphJsonDto;

pub(crate) fn from_dto(
    dto: GraphJsonDto,
) -> Result<(DependencyGraph, CycleReport), JsonDeserializeError>;
```

Estas funções podem ser expostas para testes inline ou para
ferramentas que queiram trabalhar directamente com DTOs sem ir
para JSON.

---

## Tipos de erro

```rust
#[derive(Debug, thiserror::Error)]
pub enum JsonSerializeError {
    #[error("Falha ao serializar para JSON: {source}")]
    SerdeError {
        #[from]
        source: serde_json::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum JsonDeserializeError {
    #[error("Falha ao parsear JSON: {source}")]
    SerdeError {
        #[from]
        source: serde_json::Error,
    },

    #[error("Versão de schema incompatível: esperado 1.x.y, recebido {version}")]
    IncompatibleSchemaVersion { version: String },

    #[error("Nó referenciado em aresta não existe: {canonical_path}")]
    DanglingEdgeReference { canonical_path: String },

    #[error("Nó referenciado em ciclo não existe: {canonical_path}")]
    DanglingCycleReference { canonical_path: String },

    #[error("Erro ao construir grafo: {source}")]
    GraphConstructionError {
        #[from]
        source: GraphError,  // de L1
    },
}
```

---

## Dependências externas

Em `03_infra/Cargo.toml` (NÃO em `01_core/Cargo.toml`):

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
# thiserror já presente
```

---

## Testes esperados

### Testes unitários (no próprio ficheiro)

1. **Serializar grafo vazio**: `DependencyGraph` sem nós produz
   JSON com `graph.nodes == []`, `graph.edges == []`.

2. **Serializar grafo com 1 nó interno**: verifica estrutura e
   campos.

3. **Serializar grafo com 1 nó externo (crate)**: `kind:
   "external"`, `external_kind: "crate"`.

4. **Serializar grafo com 1 nó externo (stdlib)**: `external_kind:
   "stdlib"`.

5. **Serializar grafo com 1 aresta**: verifica `from`, `to`,
   `imported_item`, etc.

6. **Serializar aresta com alias `None`**: `alias: null` aparece
   no JSON (não é omitido).

7. **Serializar aresta com alias `Some`**: `alias: "Bar"`.

8. **Serializar aresta glob**: `is_glob: true`,
   `imported_item: "*"`.

9. **Serializar aresta re-export**: `is_reexport: true`.

10. **Ordenação alfabética de nós**: criar grafo com 3 nós em
    ordem inversa; serializar; verificar que aparecem em ordem
    alfabética.

11. **Ordenação de arestas**: análogo.

12. **Ciclos no JSON**: criar grafo com ciclo, gerar
    `CycleReport`, serializar; verificar estrutura `cycles`.

13. **Metadados**: `schema_version == "1.0.0"`, `tool.name ==
    "crystalline-dsm"`, `generated_at` igual ao parâmetro
    passado.

### Testes de round-trip

14. **Round-trip mínimo**: grafo vazio → JSON → grafo. Os dois
    grafos têm mesmo `node_count` e `edge_count`.

15. **Round-trip com nós**: 3 nós internos + 2 externos →
    JSON → grafo. Verificar contagens e que cada
    `canonical_path` original existe no grafo reconstruído.

16. **Round-trip com arestas**: 5 arestas com variações (alias,
    glob, re-export) → JSON → grafo. Verificar que cada aresta
    pode ser encontrada por `(from, to, imported_item)`.

17. **Round-trip com ciclos**: grafo com ciclo → JSON +
    CycleReport → grafo + CycleReport. Verificar
    `cycle_count` igual.

18. **Round-trip de JSON real do Typst**: gerar JSON do smoke
    test, salvar, ler de volta, comparar contagens. Pode ser
    teste `#[ignore]` igual ao smoke test.

### Testes de erros

19. **Schema incompatível**: JSON com `schema_version: "2.0.0"`
    deve retornar `IncompatibleSchemaVersion`.

20. **Aresta com referência inválida**: JSON com `edge.to`
    apontando para `canonical_path` inexistente em `nodes` deve
    retornar `DanglingEdgeReference`.

21. **Ciclo com referência inválida**: análogo.

22. **JSON malformado**: string que não é JSON válido deve
    retornar `SerdeError`.

---

## Critério de aceitação do prompt

- `03_infra/src/json_serializer.rs` existe e compila.
- DTOs definidos conforme especificado.
- Funções `to_canonical_json` e `from_canonical_json` com as
  assinaturas especificadas.
- Os 22 testes acima passam.
- `cargo clippy --all-targets` sem warnings.
- `serde` e `serde_json` adicionados ao `Cargo.toml` de
  `03_infra` (e NÃO ao de `01_core`).
- Módulo exportado em `03_infra/src/lib.rs`.
- L₁ permanece inalterado (nenhuma struct do core ganha `serde`
  derive).

---

## Próximos passos (fora deste prompt)

Após implementação:

1. Em L₄, adicionar lógica para gravar o JSON em ficheiro:
   - Flag CLI `--output <path>` ou similar.
   - Função em L₄ que chama `to_canonical_json` e grava com
     `std::fs::write`.
   - Geração do `generated_at` (chamar relógio do sistema aqui,
     fora de L₃).
   - Leitura da versão da tool (de `env!("CARGO_PKG_VERSION")`).

2. Documentar o schema em `docs/json-schema-v1.md`.

3. Adicionar exemplo de JSON gerado contra Typst no README ou
   numa pasta `examples/`.

Esses três passos ficam fora deste prompt; o módulo
`json_serializer` em si está completo com o conteúdo descrito.

---

## Limitações conhecidas

1. `tree_node_id` é perdido no round-trip (não está no JSON).
   Aceitável: é índice interno, sem significado fora do grafo.

2. Ordem dentro de cada ciclo é alfabética por `canonical_path`,
   não a "ordem cíclica" real (que não é unicamente definida).
   Documentado também na ADR-0006.

3. Não há compressão. JSON de Typst pode chegar a 1-2 MB.
   Aceitável para o MVP. Futuro: opção de gzip.

---

## Hash do prompt

A calcular após aprovação.
