# Prompt L0: Detector de Ciclos (L₁)

**Camada**: L₁ (Núcleo)
**Ficheiro alvo**: `01_core/src/rules/cycle_detector.rs`
**Passo do roadmap**: 1.5 — Detecção de ciclos
**Status**: IMPLEMENTADO


---

## Decisões de design prévias

- **ADR-0005**: `petgraph` em `l1_allowed_external`. Pode ser
  usado directamente neste módulo.
- **ADR-0006**: Nós externos têm grau de saída zero por
  construção. Não participam de ciclos.

---

## Decisões locais (assumidas neste prompt)

1. **Algoritmo**: Tarjan SCC (Strongly Connected Components) via
   `petgraph::algo::tarjan_scc`. Componentes fortemente conexas
   com mais de 1 nó (ou loops em 1 nó) constituem ciclos.

2. **Localização**: `01_core/src/rules/cycle_detector.rs`. Esta
   é a primeira "regra" do projecto. Justifica criar o subdir
   `rules/` ao lado de `entities/` para manter separação
   conceptual (entidades vs algoritmos sobre elas).

3. **Self-loops**: um nó com aresta para si mesmo é tratado como
   ciclo de tamanho 1. Aparece em `tarjan_scc` como SCC de tamanho
   1 com self-loop. Lógica de detecção precisa diferenciar SCC
   trivial (1 nó, sem self-loop) de SCC com self-loop.

4. **Saída ordenada**: a ordem dos ciclos retornados é
   determinística. Ordenar por: (i) tamanho do ciclo decrescente,
   (ii) `canonical_path` do primeiro nó alfabeticamente. Saída
   reproduzível entre execuções.

---

## Contexto

Detecção de ciclos é importante em DSM para duas razões:

1. **Diagnóstico arquitectural**: ciclos entre módulos são quase
   sempre red flags. Refactor recomendado.

2. **Particionamento DSM (Passo 2.1)**: o algoritmo de
   ordenação topológica precisa saber quais nós formam ciclos
   para agrupá-los na diagonal da matriz.

Este detector serve a ambos os usos, retornando uma estrutura
que descreve cada ciclo (lista de nós participantes) e que pode
ser consumida tanto por diagnóstico textual quanto pelo
particionador.

---

## Definição das structs

### `Cycle`

```rust
pub struct Cycle {
    /// Nós que participam do ciclo, em ordem alguma estável
    /// (a ordem real do ciclo não é única; usamos ordem
    /// alfabética por canonical_path para determinismo).
    pub nodes: Vec<GraphNodeId>,

    /// Tipo do ciclo.
    pub kind: CycleKind,
}

pub enum CycleKind {
    /// Ciclo entre múltiplos módulos (a→b→a, a→b→c→a, etc).
    /// `nodes` tem ≥ 2 elementos.
    MultiNode,

    /// Self-loop (um nó depende de si mesmo).
    /// `nodes` tem exactamente 1 elemento.
    SelfLoop,
}
```

### `CycleReport`

```rust
pub struct CycleReport {
    /// Todos os ciclos detectados.
    /// Ordem: por tamanho decrescente, depois alfabética.
    pub cycles: Vec<Cycle>,
}

impl CycleReport {
    /// `true` se há pelo menos um ciclo.
    pub fn has_cycles(&self) -> bool;

    /// Quantidade total de ciclos.
    pub fn cycle_count(&self) -> usize;

    /// Quantidade de nós envolvidos em ciclos (soma de
    /// `nodes.len()` de cada ciclo).
    pub fn affected_node_count(&self) -> usize;

    /// Quantidade de self-loops.
    pub fn self_loop_count(&self) -> usize;

    /// Quantidade de ciclos multi-nó (≥ 2).
    pub fn multi_node_cycle_count(&self) -> usize;

    /// Itera apenas ciclos multi-nó.
    pub fn multi_node_cycles(&self) -> impl Iterator<Item = &Cycle>;

    /// Itera apenas self-loops.
    pub fn self_loops(&self) -> impl Iterator<Item = &Cycle>;
}
```

---

## Função pública principal

```rust
pub fn detect_cycles(graph: &DependencyGraph) -> CycleReport;
```

### Comportamento

1. Obter referência ao grafo `petgraph` interno via
   `graph.raw_graph()`. Este é o método `pub(crate)` exposto em
   `01_core` para uso por outros módulos de L₁ (conforme prompt
   do `DependencyGraph`).

2. Chamar `petgraph::algo::tarjan_scc(raw)`. Retorna
   `Vec<Vec<NodeIndex>>`: cada elemento é uma componente
   fortemente conexa.

3. Para cada SCC retornada:
   - Se tem 1 nó: verificar se existe aresta do nó para si mesmo.
     Se sim: `CycleKind::SelfLoop`. Se não: SCC trivial,
     **ignorar** (não é ciclo).
   - Se tem ≥ 2 nós: `CycleKind::MultiNode`.

4. Converter cada `Vec<NodeIndex>` em `Vec<GraphNodeId>` (newtype
   wrapper).

5. Ordenar `nodes` dentro de cada `Cycle`: alfabeticamente por
   `canonical_path` dos nós.

6. Ordenar `cycles` no `CycleReport`:
   - Primeiro critério: tamanho decrescente (`nodes.len()`).
   - Segundo critério: ordem alfabética do `canonical_path` do
     primeiro nó.

7. Construir e retornar `CycleReport`.

### Detecção de self-loop

`petgraph` representa self-loops como aresta com origem == destino.
Para verificar se um SCC de tamanho 1 tem self-loop:

```rust
fn has_self_loop(graph: &Graph<..., ...>, node: NodeIndex) -> bool {
    graph.edges(node).any(|e| e.target() == node)
}
```

### Performance

`tarjan_scc` é O(V + E), o que é adequado para grafos do tamanho
esperado (poucos milhares de nós no Typst). Sem optimizações
necessárias no MVP.

---

## Operações auxiliares (privadas)

```rust
fn node_canonical_paths(
    graph: &DependencyGraph,
    nodes: &[GraphNodeId],
) -> Vec<String>;
```

Útil para ordenação e diagnóstico. Recebe lista de IDs, retorna
lista de paths.

---

## Dependências externas

`01_core/Cargo.toml`:
- `petgraph` (já adicionado pelo `dependency_graph.rs` via
  ADR-0005).
- Nenhuma nova.

---

## Testes esperados

Localização: testes inline em
`01_core/src/rules/cycle_detector.rs`.

Os testes constroem `DependencyGraph`s manualmente para evitar
dependência de fixtures de filesystem.

1. **Grafo vazio**: `cycles == []`, `has_cycles == false`.

2. **Grafo sem ciclos (DAG)**: 3 nós, A→B→C. Sem ciclos.
   `cycles == []`.

3. **Ciclo de 2 nós**: A→B, B→A.
   Resultado: 1 ciclo `MultiNode` com `nodes` ordenado
   alfabeticamente.

4. **Ciclo de 3 nós**: A→B→C→A.
   Resultado: 1 ciclo `MultiNode` de 3 nós.

5. **Self-loop**: A→A.
   Resultado: 1 ciclo `SelfLoop` com `nodes == [A]`.

6. **Múltiplos ciclos disjuntos**: A→B→A e C→D→C.
   Resultado: 2 ciclos `MultiNode`.

7. **Ciclos sobrepostos**: A→B→A, A→C→A (3 nós, A no meio de dois
   ciclos).
   Esperado: 1 SCC de 3 nós (A, B, C todos no mesmo
   componente). Tarjan trata como **um** ciclo de 3 nós.
   Documentar este comportamento.

8. **Ordenação por tamanho**: ciclos de tamanho 3, 2, 4 devem
   aparecer na ordem 4, 3, 2.

9. **Ordenação alfabética secundária**: dois ciclos de tamanho 2
   com primeiros nós "a" e "b" devem vir nessa ordem.

10. **Nós externos não geram ciclo**: criar grafo com nó externo
    + interno apontando para externo. Sem ciclo (externo tem grau
    de saída zero).

11. **Misturando self-loops e multi-node**: A→A, B→C→B.
    Resultado: 1 self-loop + 1 multi-node. Ordenados por tamanho
    (multi-node primeiro).

12. **`affected_node_count`**: grafo com ciclo de 3 + ciclo de 2.
    `affected_node_count == 5`.

13. **`self_loop_count` e `multi_node_cycle_count`**: contagens
    correctas.

---

## Estrutura de directórios

A criação deste módulo introduz o subdir `rules/` em `01_core`.
Estrutura final esperada:

```
01_core/src/
├── lib.rs
├── entities/
│   ├── mod.rs
│   ├── workspace.rs
│   ├── module_tree.rs
│   ├── import_edge.rs
│   └── dependency_graph.rs
└── rules/
    ├── mod.rs
    └── cycle_detector.rs
```

Onde:
- `entities/` contém structs de dados puros.
- `rules/` contém algoritmos que operam sobre as entidades
  (regras analíticas, sem mutação de estado).

Esta separação **não** é uma camada nova; é apenas organização
interna de L₁. Tanto `entities/` quanto `rules/` são L₁.

---

## Critério de aceitação do prompt

- O ficheiro `01_core/src/rules/cycle_detector.rs` existe e
  compila.
- O ficheiro `01_core/src/rules/mod.rs` existe e expõe
  `cycle_detector`.
- `01_core/src/lib.rs` exporta `pub mod rules`.
- Todas as structs/enums conforme especificado.
- Função `detect_cycles` com a assinatura especificada.
- Os 13 testes passam.
- `cargo clippy -p crystalline-dsm-core` sem warnings.
- Nenhuma referência a `petgraph::graph::NodeIndex` ou
  `petgraph::graph::EdgeIndex` na API pública (uso interno via
  `raw_graph()` permitido).

---

## Histórico de Revisões

- **2026-05-20**: Implementado. Todos os 13 testes unitários passam com 100% de sucesso.

