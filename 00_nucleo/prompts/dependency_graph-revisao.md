# Prompt L0 (revisão): `DependencyGraph` — Variantes Explícitas do `NodeKind`

**Camada**: L₁ (Núcleo)
**Ficheiro alvo**: `01_core/src/entities/dependency_graph.rs`
  (revisão de arquivo já `IMPLEMENTADO`)
**Passo do roadmap**: 1.4 (revisão)
**Status**: PROPOSTO
**ADR motivadora**: ADR-0010
**Prompt original**: `dependency_graph.md` (status passa para
  `IMPLEMENTADO (revisado)`).

---

## Contexto da revisão

A ADR-0010 decidiu substituir a variante `NodeKind::Internal` por
duas variantes explícitas conforme a presença ou ausência do
`tree_node_id`:

- `InternalWithTree { crate_name, tree_node_id }` — quando o nó
  foi construído a partir de uma `ModuleTree` viva.
- `InternalWithoutTree { crate_name }` — quando o nó foi
  construído sem link com árvore (ex: deserializado do
  `graph.json`).

Esta mudança elimina a necessidade do `NodeId::placeholder()`
provisório (que era usado como sentinela em deserialização). Ao
mesmo tempo, a API de construção do `DependencyGraph` é dividida
em duas funções públicas, cada uma criando uma das variantes.

---

## Mudanças nas structs

### `NodeKind` (substitui o existente)

```rust
pub enum NodeKind {
    /// Nó interno com referência à `ModuleTree` que o produziu.
    /// Construído via `add_internal_node_with_tree`.
    InternalWithTree {
        crate_name: String,
        tree_node_id: NodeId,
    },

    /// Nó interno sem referência à `ModuleTree`.
    /// Construído via `add_internal_node_without_tree`.
    /// Estado típico quando o grafo é reconstruído a partir de JSON.
    InternalWithoutTree {
        crate_name: String,
    },

    /// Nó externo (crates.io ou stdlib).
    External {
        kind: ExternalKind,
    },
}
```

### `GraphNode` — sem mudança

```rust
pub struct GraphNode {
    pub canonical_path: String,
    pub kind: NodeKind,
}
```

A estrutura mantém-se. Apenas o conteúdo de `kind` é mais rico.

### `ExternalKind`, `GraphEdge`, `GraphNodeId`, `GraphEdgeId` — sem mudança

---

## Mudanças na API do `DependencyGraph`

### Remoção

A função `add_internal_node` é **removida** (ou marcada como
deprecated com aviso). É substituída por duas funções abaixo.

### Adição

```rust
impl DependencyGraph {
    /// Adiciona um nó interno com link para `ModuleTree`.
    /// Usado durante construção em RAM a partir de uma árvore real.
    /// Se já existir nó com o mesmo `canonical_path`, retorna o ID
    /// existente sem duplicar. Comportamento de deduplicação
    /// idêntico ao original.
    pub fn add_internal_node_with_tree(
        &mut self,
        canonical_path: String,
        crate_name: String,
        tree_node_id: NodeId,
    ) -> GraphNodeId;

    /// Adiciona um nó interno sem link para `ModuleTree`.
    /// Usado durante deserialização de `graph.json` ou outras
    /// fontes sem árvore.
    /// Deduplicação idêntica.
    pub fn add_internal_node_without_tree(
        &mut self,
        canonical_path: String,
        crate_name: String,
    ) -> GraphNodeId;
}
```

### Comportamento de deduplicação entre variantes

Se um nó com `canonical_path` "X" já existe como
`InternalWithoutTree`, e depois é chamada
`add_internal_node_with_tree("X", ...)`:

**Decisão**: retornar o `GraphNodeId` existente, sem alterar a
variante. O nó permanece `InternalWithoutTree`. Razão:
imutabilidade da identidade dos nós. Se o utilizador quiser
"promover" um nó para `InternalWithTree`, deve reconstruir o
grafo.

Caso simétrico (chamar `add_internal_node_without_tree` em nó
que já existe como `InternalWithTree`): mesmo comportamento.
Retorna o ID existente, mantém a variante `InternalWithTree`.

Esse comportamento é documentado nos docstrings das duas funções.

### Outras funções — sem mudança

`add_external_node`, `add_edge`, `find_node`, `node`,
`node_count`, `internal_node_count`, `external_node_count`,
`all_nodes`, `internal_nodes`, `external_nodes`, `edge`,
`edge_count`, `all_edges`, `outgoing_edges`, `incoming_edges`,
`out_degree`, `in_degree`, `raw_graph` (pub(crate)) — todas
mantidas com mesma assinatura.

### `internal_nodes` filtra ambas as variantes internas

A função `internal_nodes` retorna tanto `InternalWithTree`
quanto `InternalWithoutTree`. Quem precisar distinguir faz
`match` no `kind` do nó retornado.

Critério interno:
```rust
matches!(
    node.kind,
    NodeKind::InternalWithTree { .. }
        | NodeKind::InternalWithoutTree { .. }
)
```

---

## Remoção de `NodeId::placeholder()`

Em `01_core/src/entities/module_tree.rs`:

- Remover a função `pub fn placeholder() -> NodeId`.
- Verificar que nenhum outro código a usa (deve ser apenas
  `json_serializer`, que será actualizado em paralelo).

Resultado: L₁ recupera a pureza original (nenhum método público
desenhado para "facilitar L₃").

---

## Derives obrigatórios

Sem mudanças. Todas as structs e enums mantêm os derives
originais (`Debug`, `Clone`, `PartialEq`, `Eq`, `Hash` em
`NodeKind`).

---

## Invariantes

Adicionais:

5. **Variantes `InternalWithTree` e `InternalWithoutTree` são
   semanticamente equivalentes para detecção de ciclos,
   ordenação topológica e análises de grafo.** A diferença é
   apenas informacional (presença ou ausência do
   `tree_node_id`).

---

## Testes esperados (atualização)

Os 12 testes existentes precisam de ajustes:

1. **Testes que usavam `add_internal_node`**: trocar por
   `add_internal_node_with_tree` (a maioria dos testes existentes
   constrói nós com `NodeId` em mãos).

2. **Adicionar teste de `add_internal_node_without_tree`**:
   construção de nó sem árvore.

3. **Adicionar teste de deduplicação cross-variant**:
   - Adicionar nó X como `InternalWithoutTree`.
   - Tentar adicionar X como `InternalWithTree`.
   - Verificar que retorna mesmo ID e variante permanece
     `InternalWithoutTree`.
   - Caso simétrico.

4. **`internal_nodes` itera ambas as variantes**: criar grafo
   com 1 `InternalWithTree`, 1 `InternalWithoutTree`, 1
   `External`. `internal_nodes()` retorna 2.

Testes totais esperados após revisão: 14 (12 originais
actualizados + 2 novos).

---

## Critério de aceitação do prompt

- `01_core/src/entities/dependency_graph.rs` actualizado com as
  novas variantes e a API dividida.
- `01_core/src/entities/module_tree.rs` com
  `NodeId::placeholder()` removido.
- Os 14 testes passam.
- `cargo clippy -p crystalline-dsm-core` sem warnings.
- Status do prompt original `dependency_graph.md` actualizado
  para `IMPLEMENTADO (revisado)`.
- Status do prompt original `module_tree.md` actualizado para
  `IMPLEMENTADO (revisado)` (devido à remoção do placeholder).

---

## Impacto em código existente (revisões em paralelo)

Esta revisão **quebra compatibilidade** com código que usa:

- `NodeKind::Internal` em `match` ou construção literal.
- `add_internal_node` (removida).
- `NodeId::placeholder()` (removida).

Os seguintes ficheiros precisam de ajustes em paralelo:

- `04_wiring/src/graph_builder.rs`: trocar
  `add_internal_node(...)` por
  `add_internal_node_with_tree(...)`. Mudança em uma linha.

- `03_infra/src/json_serializer.rs`:
  - DTO `NodeDto`: `crate_name` movido para dentro do `kind`
    quando interno (já era; sem mudança).
  - `from_dto`: chamar `add_internal_node_without_tree`.
  - `to_dto`: `match` em `NodeKind` lida com as duas variantes
    internas, mapeando ambas para o mesmo
    `NodeKindDto::Internal { crate_name }`.

- Smoke test e outros testes que verifiquem
  `NodeKind::Internal { .. }`: actualizar para usar as novas
  variantes.

Os prompts de revisão correspondentes (`json_serializer-revisao.md`,
ajustes em `graph_builder`) são documentos separados.

---

## Hash do prompt

A calcular após aprovação.
