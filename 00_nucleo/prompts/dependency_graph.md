# Prompt L0: Entidade `DependencyGraph` (L₁)

**Camada**: L₁ (Núcleo)
**Ficheiro alvo**: `01_core/src/entities/dependency_graph.rs`
**Passo do roadmap**: 1.4 — Construção do grafo
**Status**: IMPLEMENTADO (revisado)
**Revisão**: `dependency_graph-revisao.md` (ADR-0010 — `NodeKind::Internal` substituído por `InternalWithTree` + `InternalWithoutTree`; API dividida em `add_internal_node_with_tree` e `add_internal_node_without_tree`).


---

## Decisões de design prévias (registadas em ADR)

- **ADR-0004**: Nó representa módulo lógico com caminho canónico.
- **ADR-0005**: `petgraph` em `l1_allowed_external`. API pública
  de L₁ NÃO expõe tipos do `petgraph`. `DependencyGraph` é
  wrapper.
- **ADR-0006**: Módulos externos viram nós fantasma. Cada
  caminho de módulo distinto (interno ou externo) é um nó único
  no grafo.

---

## Decisões locais (assumidas neste prompt)

1. **Representação interna**: `petgraph::Graph<GraphNode,
   GraphEdge, Directed>`. Direcção da aresta: do importador para
   o importado.

2. **Identidade de nó**: `GraphNodeId` é newtype opaco sobre
   `petgraph::graph::NodeIndex`. Construção apenas via métodos do
   `DependencyGraph`. Comparação entre `GraphNodeId`s de grafos
   diferentes não tem significado.

3. **Deduplicação de nós**: dois nós com o mesmo `canonical_path`
   são o mesmo nó. O `DependencyGraph` mantém índice interno
   `HashMap<String, GraphNodeId>` para deduplicação.

4. **Múltiplas arestas entre mesmos nós**: permitidas. Cada
   `ImportEdge` que conecta o mesmo par (origem, destino) gera uma
   aresta separada no grafo. Razão: queremos preservar a info de
   cada import individual (item importado, alias, glob, etc).
   Agregação para contagem é responsabilidade de quem consome (ex:
   renderizador DSM).

5. **`Unresolved` não entra no grafo**: imports com
   `ImportKind::Unresolved` são silenciosamente ignorados nesta
   entidade (mas a fonte deve ter emitido warning antes, em L₃ no
   Passo 1.3).

---

## Contexto

Esta entidade é o produto canónico do Passo 1.4: a estrutura de
dados que materializa o grafo de dependências completo. É o input
para:
- Detecção de ciclos (Passo 1.5).
- Ordenação topológica para DSM (Passo 2.1).
- Renderização HTML (Passo 2.2).
- Serialização JSON canónica (Passo 1.4 também, mas via prompt
  separado).

A construção é feita em L₄ (conversor), que combina
`ModuleTree`s e `ImportEdge`s. Esta entidade fornece a API para
essa construção (`add_node`, `add_edge`) e para análise (acesso
a nós, arestas, predecessores, sucessores).

---

## Definição das structs

### `GraphNode`

Conforme ADR-0006. Reproduzido aqui:

```rust
pub struct GraphNode {
    /// Identificador canónico. Único no grafo.
    pub canonical_path: String,

    /// Tipo de nó: interno ou externo.
    pub kind: NodeKind,
}

pub enum NodeKind {
    /// Módulo do código próprio (algum crate do workspace).
    Internal {
        /// Crate ao qual o módulo pertence.
        crate_name: String,
        /// Referência ao nó no `ModuleTree` correspondente.
        /// Permite navegação para o `ModuleNode` original.
        tree_node_id: NodeId,
    },

    /// Módulo externo (crates.io ou stdlib).
    External {
        kind: ExternalKind,
    },
}

pub enum ExternalKind {
    /// Crate da stdlib: std, core, alloc.
    Stdlib,
    /// Crate externo (crates.io, path, git).
    Crate,
}
```

### `GraphEdge`

```rust
pub struct GraphEdge {
    /// Item importado. Para glob: "*".
    pub imported_item: String,

    /// Alias, se houver.
    pub alias: Option<String>,

    /// `true` se a aresta corresponde a `pub use`.
    pub is_reexport: bool,

    /// `true` se é glob import.
    pub is_glob: bool,

    /// Caminho textual original do `use` (diagnóstico).
    pub raw_use_path: String,
}
```

Notas:
- A aresta NÃO carrega `ImportKind`. O kind é deduzido pelo
  `NodeKind` do nó destino (`Internal` ↔ import interno;
  `External` ↔ externo).
- A aresta NÃO carrega `from` nem `target_module` explícitos;
  esses são representados pela posição da aresta no grafo (nó
  origem e nó destino).

### `GraphNodeId`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphNodeId(petgraph::graph::NodeIndex);
```

Newtype opaco. O `NodeIndex` interno não é exposto.

### `GraphEdgeId`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphEdgeId(petgraph::graph::EdgeIndex);
```

Análogo a `GraphNodeId`.

### `DependencyGraph`

```rust
pub struct DependencyGraph {
    /// Grafo interno (petgraph).
    graph: petgraph::Graph<GraphNode, GraphEdge, petgraph::Directed>,

    /// Índice canonical_path -> GraphNodeId para deduplicação.
    path_index: HashMap<String, GraphNodeId>,
}
```

Campos privados. Toda interacção é via métodos.

---

## Operações em L₁

### Construção

```rust
impl DependencyGraph {
    /// Cria um grafo vazio.
    pub fn new() -> Self;

    /// Adiciona um nó interno. Se já existir nó com o mesmo
    /// canonical_path, retorna o `GraphNodeId` existente sem
    /// duplicar.
    pub fn add_internal_node(
        &mut self,
        canonical_path: String,
        crate_name: String,
        tree_node_id: NodeId,
    ) -> GraphNodeId;

    /// Adiciona um nó externo (External::Crate ou External::Stdlib).
    /// Mesmo comportamento de deduplicação por canonical_path.
    pub fn add_external_node(
        &mut self,
        canonical_path: String,
        external_kind: ExternalKind,
    ) -> GraphNodeId;

    /// Adiciona uma aresta entre dois nós existentes.
    /// Múltiplas arestas entre o mesmo par são permitidas.
    /// Em caso de `GraphNodeId` inválido, retorna `Err`.
    pub fn add_edge(
        &mut self,
        from: GraphNodeId,
        to: GraphNodeId,
        edge: GraphEdge,
    ) -> Result<GraphEdgeId, GraphError>;
}
```

### Inspecção de nós

```rust
impl DependencyGraph {
    /// Retorna o `GraphNodeId` correspondente a um canonical_path,
    /// ou None se não existir.
    pub fn find_node(&self, canonical_path: &str) -> Option<GraphNodeId>;

    /// Retorna referência ao `GraphNode`. Panic se ID inválido
    /// (programação errada do utilizador; o método não esconde bug).
    pub fn node(&self, id: GraphNodeId) -> &GraphNode;

    /// Quantidade total de nós (internos + externos).
    pub fn node_count(&self) -> usize;

    /// Quantidade de nós internos.
    pub fn internal_node_count(&self) -> usize;

    /// Quantidade de nós externos.
    pub fn external_node_count(&self) -> usize;

    /// Itera todos os nós.
    pub fn all_nodes(&self) -> impl Iterator<Item = (GraphNodeId, &GraphNode)>;

    /// Itera apenas nós internos.
    pub fn internal_nodes(&self) -> impl Iterator<Item = (GraphNodeId, &GraphNode)>;

    /// Itera apenas nós externos.
    pub fn external_nodes(&self) -> impl Iterator<Item = (GraphNodeId, &GraphNode)>;
}
```

### Inspecção de arestas

```rust
impl DependencyGraph {
    /// Retorna referência ao `GraphEdge`.
    pub fn edge(&self, id: GraphEdgeId) -> &GraphEdge;

    /// Quantidade total de arestas.
    pub fn edge_count(&self) -> usize;

    /// Itera todas as arestas com seus endpoints.
    /// Retorna (origem, destino, aresta).
    pub fn all_edges(&self) -> impl Iterator<Item = (GraphNodeId, GraphNodeId, &GraphEdge)>;

    /// Arestas que saem de um nó (sucessores).
    pub fn outgoing_edges(&self, from: GraphNodeId)
        -> impl Iterator<Item = (GraphNodeId, &GraphEdge)>;

    /// Arestas que chegam a um nó (predecessores).
    pub fn incoming_edges(&self, to: GraphNodeId)
        -> impl Iterator<Item = (GraphNodeId, &GraphEdge)>;

    /// Grau de saída de um nó.
    pub fn out_degree(&self, id: GraphNodeId) -> usize;

    /// Grau de entrada de um nó.
    pub fn in_degree(&self, id: GraphNodeId) -> usize;
}
```

### Acesso interno controlado

Para uso por outros módulos de L₁ (especialmente o detector de
ciclos do Passo 1.5), expor:

```rust
impl DependencyGraph {
    /// Acesso ao grafo `petgraph` interno.
    /// `pub(crate)` — apenas para uso dentro de L₁.
    /// Outras camadas NÃO devem usar este método.
    pub(crate) fn raw_graph(&self) -> &petgraph::Graph<GraphNode, GraphEdge, petgraph::Directed>;
}
```

Justificação: o algoritmo Tarjan (Passo 1.5) precisa do grafo
`petgraph` directamente para chamar `petgraph::algo::tarjan_scc`.
Em vez de reimplementar Tarjan, expor o grafo internamente para
L₁ permitir o reuso, mantendo o isolamento contra L₂/L₃/L₄.

---

## Tipo de erro

```rust
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("GraphNodeId inválido para este grafo")]
    InvalidNodeId,
}
```

Apenas um erro previsto: usar um `GraphNodeId` que não pertence
a este grafo (raro, mas possível se misturar IDs de grafos
diferentes).

---

## Invariantes

1. **Unicidade por `canonical_path`**: `path_index` e o grafo
   sempre coerentes. Para todo `(path, id)` em `path_index`, o nó
   em `id` tem `canonical_path == path`.

2. **Coerência de `NodeKind`**: nós adicionados via
   `add_internal_node` têm `NodeKind::Internal`; via
   `add_external_node`, `NodeKind::External`. Não há transição.

3. **Arestas só entre nós existentes**: garantido por
   `add_edge` retornar erro se algum endpoint for inválido.

---

## Derives obrigatórios

- `Debug` — todas as structs e enums.
- `Clone` — `GraphNode`, `NodeKind`, `ExternalKind`, `GraphEdge`,
  `GraphNodeId`, `GraphEdgeId`, `DependencyGraph`.
- `PartialEq`, `Eq` — `GraphNode`, `NodeKind`, `ExternalKind`,
  `GraphEdge`, `GraphNodeId`, `GraphEdgeId`, `GraphError`.
- `Hash` — `GraphNodeId`, `GraphEdgeId`.

`DependencyGraph` deriva `PartialEq`? Considerar: comparação de
grafos é não-trivial (isomorfismo). Para o MVP, `PartialEq` em
`DependencyGraph` compara `path_index` + `node_count` +
`edge_count` (igualdade rasa). Para testes, isso basta. Se for
necessário rigor (igualdade estrutural completa), reavaliar.

**Decisão**: NÃO derivar `PartialEq` em `DependencyGraph` no MVP.
Testes que precisem comparar grafos comparam contagens e
existência de nós/arestas individualmente.

Sem `Serialize`/`Deserialize` neste prompt. Passo 1.4 inclui
serialização JSON mas via prompt separado.

---

## Dependências externas

Em `01_core/Cargo.toml`:

```toml
[dependencies]
thiserror = "..."  # já presente
petgraph = "0.6"   # NOVA dependência, autorizada via ADR-0005
```

A versão `"0.6"` é sugestão; verificar a versão estável corrente
no momento da implementação e fixar.

Adicionar `petgraph` à lista `l1_allowed_external` na configuração
Tekt do projecto (`crystalline.toml` ou equivalente).

---

## Testes esperados

Localização: testes inline em
`01_core/src/entities/dependency_graph.rs`.

1. **Grafo vazio**: `new()` cria grafo com 0 nós, 0 arestas.

2. **`add_internal_node` único**: adiciona um nó interno;
   `node_count == 1`, `internal_node_count == 1`,
   `external_node_count == 0`. `find_node(canonical_path)` retorna
   o ID.

3. **Deduplicação interna**: adicionar dois nós com mesmo
   `canonical_path` retorna o mesmo `GraphNodeId`. `node_count`
   continua 1.

4. **`add_external_node`**: adiciona nó externo
   (`ExternalKind::Stdlib`); `external_node_count == 1`.

5. **Deduplicação externa**: análogo a interna.

6. **Adicionar aresta**: criar 2 nós, adicionar aresta entre
   eles; `edge_count == 1`.

7. **Múltiplas arestas**: adicionar 3 arestas entre o mesmo par;
   `edge_count == 3`. Iteração retorna todas.

8. **`add_edge` com ID inválido**: usar um `GraphNodeId`
   construído manualmente (via método de teste auxiliar) que não
   pertence ao grafo. Retorna `Err(InvalidNodeId)`.

9. **`outgoing_edges` e `incoming_edges`**: criar A→B→C.
   `outgoing_edges(A)` itera 1 aresta para B. `incoming_edges(C)`
   itera 1 aresta de B. `outgoing_edges(C)` é vazio.

10. **`out_degree` e `in_degree`**: mesmo grafo do teste 9.
    `out_degree(A) == 1`, `in_degree(C) == 1`, `out_degree(C) == 0`.

11. **Iteração filtrada por kind**: criar 2 internos + 3 externos.
    `internal_nodes` itera 2; `external_nodes` itera 3.
    `all_nodes` itera 5.

12. **`all_edges` retorna endpoints correctos**: criar A→B com
    aresta específica. `all_edges` retorna `(A_id, B_id, &edge)`.

---

## Critério de aceitação do prompt

- O ficheiro `01_core/src/entities/dependency_graph.rs` existe e
  compila.
- Todas as structs/enums conforme especificado.
- Métodos com assinaturas exactas.
- Os 12 testes passam.
- `cargo clippy -p crystalline-dsm-core` sem warnings.
- `petgraph` declarado no `Cargo.toml` de `01_core` e na lista
  `l1_allowed_external` do projecto.
- Entidade exportada via `01_core/src/entities/mod.rs`.
- API pública não expõe `petgraph::graph::NodeIndex` nem outros
  tipos de `petgraph`.

## Histórico de Revisões

- **2026-05-20**: Implementado. Todos os 12 testes unitários passam com 100% de sucesso.

