# Prompt L0: Particionador DSM (L₁)

**Camada**: L₁ (Núcleo)
**Ficheiro alvo**: `01_core/src/rules/dsm_partitioner.rs`
**Passo do roadmap**: 2.1 — Ordenação topológica para DSM
**Status**: IMPLEMENTADO
**Nota de implementação**: a regra de ordenação adoptada é "dependência primeiro, dependente depois" (convenção DSM clássica), conforme exemplificado pelo teste 7 do prompt ("C aponta para o ciclo, C vem depois do ciclo"). Os testes 4 e 5 do prompt, escritos com a convenção oposta, foram ajustados nas expectativas reais: A→B→C produz `[C, B, A]` e A→C, B→C produz `[C, A, B]`. Validado contra Typst: `internal_boundary = 443` (idêntico à previsão do prompt) e `cyclic_scc_count = cycle_count = 18`.

---

## Decisões de design prévias

- **ADR-0005**: `petgraph` em L₁. Pode ser usado directamente,
  incluindo `petgraph::algo::tarjan_scc`.
- **ADR-0006**: Nós externos têm grau de saída zero por
  construção.

---

## Decisões locais (assumidas neste prompt)

1. **Algoritmo**: Tarjan SCC + topological sort sobre o grafo
   condensado. Algoritmo canónico de DSM.

2. **Nós externos forçados para o final**: a região da matriz é
   dividida em duas partes contíguas: internos primeiro,
   externos depois. Isso isola visualmente as dependências
   externas do código próprio.

3. **Modelo uniforme de SCC**: todo nó pertence a algum SCC,
   mesmo que trivial (1 nó, sem self-loop). Simplifica o
   modelo de consumo.

4. **Contiguidade dos SCCs cíclicos**: garantida pelo algoritmo
   (propriedade matemática de SCC + condensação), não exige
   código extra.

5. **Ordenação determinística**:
   - Entre SCCs: alfabética pelo menor `canonical_path` de cada
     SCC.
   - Dentro de SCC: alfabética por `canonical_path`.
   - Resultado byte-determinístico para o mesmo grafo.

6. **Localização em `rules/`**: este é o segundo algoritmo em L₁
   (depois do `cycle_detector`). Confirma o padrão de separação
   entidades vs regras.

---

## Contexto

Após o Passo 1.4 produzir o `DependencyGraph` e o Passo 1.5
detectar ciclos, o Passo 2.1 reordena os nós para que a matriz
DSM seja visualmente significativa. A ordem produzida aqui é o
input directo para o renderizador HTML (Passo 2.2).

A função pública recebe o grafo e produz uma struct contendo:

- A ordem final dos nós.
- Para cada nó, o SCC ao qual pertence.
- Definição dos SCCs (com fronteiras e flag de ciclicidade).
- Posição da fronteira entre internos e externos.

---

## Definição das structs

### `PartitionedOrder`

Resultado completo do particionamento.

```rust
pub struct PartitionedOrder {
    /// Ordem linear final dos nós.
    /// Índice 0 é a primeira linha/coluna da matriz DSM (topo
    /// e esquerda).
    pub order: Vec<GraphNodeId>,

    /// Para cada posição em `order`, o índice do SCC em `sccs`
    /// ao qual o nó pertence.
    /// `scc_index_per_node[i]` indexa em `sccs`.
    pub scc_index_per_node: Vec<usize>,

    /// Definição dos SCCs presentes no grafo.
    /// Para um grafo com N nós, há entre 1 e N SCCs.
    pub sccs: Vec<SccBlock>,

    /// Posição na ordem que separa nós internos de externos.
    /// `order[0..internal_boundary]` são todos internos.
    /// `order[internal_boundary..]` são todos externos.
    /// Se não há externos: `internal_boundary == order.len()`.
    /// Se não há internos: `internal_boundary == 0`.
    pub internal_boundary: usize,
}
```

### `SccBlock`

Descreve um componente fortemente conexo na ordem final.

```rust
pub struct SccBlock {
    /// Posições no `PartitionedOrder.order` que este SCC ocupa.
    /// `range.start` é inclusivo, `range.end` é exclusivo
    /// (convenção padrão de `Range`).
    pub range: std::ops::Range<usize>,

    /// `true` se este SCC representa um ciclo real:
    /// - Tamanho > 1 (múltiplos nós), OU
    /// - Tamanho 1 com self-loop (nó depende de si mesmo).
    /// `false` para SCC trivial (1 nó sem self-loop).
    pub is_cyclic: bool,
}
```

Notas:
- O tamanho do SCC é `range.len()` (não precisa de campo
  separado).
- Para iterar os nós de um SCC: `partition.order[scc.range.clone()]`.

---

## Função pública principal

```rust
pub fn partition_for_dsm(graph: &DependencyGraph) -> PartitionedOrder;
```

### Comportamento

1. **Separar internos de externos**:
   - `internal_ids: Vec<GraphNodeId>` — todos os internos.
   - `external_ids: Vec<GraphNodeId>` — todos os externos.

2. **Particionar a região interna**:

   a. Construir subgrafo `internal_subgraph` contendo apenas nós
      internos e arestas entre eles. Implementação: usar
      `petgraph::visit::NodeFiltered` ou criar `Graph` novo.

   b. Aplicar `petgraph::algo::tarjan_scc(internal_subgraph)`
      para obter componentes fortemente conexas. Retorno:
      `Vec<Vec<NodeIndex>>`, onde cada elemento é uma SCC.

   c. Para cada SCC com tamanho > 1: marcar como cíclica.

   d. Para cada SCC com tamanho 1: verificar se há self-loop:
      ```rust
      let n = scc[0];
      let has_self_loop = subgraph.edges(n).any(|e| e.target() == n);
      ```
      Se sim, marcar como cíclica; se não, trivial.

   e. **Ordenar os nós dentro de cada SCC** alfabeticamente
      por `canonical_path`.

   f. **Construir o grafo condensado** das SCCs:
      - Um super-nó por SCC.
      - Arestas entre super-nós conforme arestas entre SCCs no
        grafo original.

   g. **Topological sort do condensado** (que é DAG por
      construção). Em caso de empate (múltiplas ordens válidas):
      ordenar SCCs alfabeticamente pelo menor `canonical_path`
      dentro de cada SCC.

   h. **Expansão final**: para cada SCC na ordem topológica,
      adicionar todos os seus nós ao `order` na ordem alfabética
      definida em (e).

3. **Adicionar externos ao final do `order`**:
   - Externos em ordem alfabética por `canonical_path`.
   - Cada externo forma seu próprio SCC trivial (são acíclicos
     por construção, conforme ADR-0006).

4. **Construir `scc_index_per_node`**:
   - Mesma ordem que `order`.
   - Cada nó aponta para o índice do seu SCC em `sccs`.

5. **`internal_boundary`** = quantidade de internos.

6. Retornar `PartitionedOrder`.

### Tratamento de grafo vazio

- Grafo com 0 nós: retornar `PartitionedOrder` com `order` vazio,
  `scc_index_per_node` vazio, `sccs` vazio, `internal_boundary == 0`.

### Tratamento de grafo só com externos

- Não há internos para particionar. `internal_boundary == 0`.
- Externos em ordem alfabética.

### Tratamento de grafo só com internos

- `internal_boundary == order.len()`. Sem região externa.

---

## Funções auxiliares (privadas)

```rust
/// Constrói subgrafo contendo apenas nós internos e arestas entre
/// eles. Mantém mapeamento de NodeIndex original para índice no
/// subgrafo (necessário para Tarjan).
fn build_internal_subgraph(
    graph: &DependencyGraph,
) -> (InternalSubgraph, HashMap<GraphNodeId, NodeIndex>);

/// Aplica Tarjan, retorna SCCs em estrutura interna.
fn compute_sccs(...) -> Vec<Vec<GraphNodeId>>;

/// Ordena SCCs entre si segundo a regra alfabética definida
/// (menor canonical_path por SCC) e topologicamente válida.
fn topological_sort_sccs(
    graph: &DependencyGraph,
    sccs: &[Vec<GraphNodeId>],
) -> Vec<usize>;  // permutação de índices em `sccs`

/// Detecta self-loop num SCC de tamanho 1.
fn has_self_loop(
    graph: &DependencyGraph,
    node: GraphNodeId,
) -> bool;
```

---

## Operações de consumo (auxiliares públicas)

Para facilitar uso por L₂/L₃/L₄, expor:

```rust
impl PartitionedOrder {
    /// Quantidade total de nós ordenados.
    pub fn len(&self) -> usize;

    /// `true` se vazio.
    pub fn is_empty(&self) -> bool;

    /// Para um índice no `order`, retorna o `GraphNodeId`.
    /// Panic se índice fora dos limites (programação errada).
    pub fn node_at(&self, index: usize) -> GraphNodeId;

    /// Para um índice no `order`, retorna o SCC ao qual pertence.
    pub fn scc_for_position(&self, index: usize) -> &SccBlock;

    /// Quantidade de SCCs cíclicos.
    pub fn cyclic_scc_count(&self) -> usize;

    /// Quantidade de SCCs triviais.
    pub fn trivial_scc_count(&self) -> usize;

    /// Itera apenas as posições internas.
    pub fn internal_positions(&self) -> std::ops::Range<usize>;

    /// Itera apenas as posições externas.
    pub fn external_positions(&self) -> std::ops::Range<usize>;
}
```

---

## Derives obrigatórios

- `Debug` — todas as structs.
- `Clone` — todas.
- `PartialEq`, `Eq` — todas.

Sem `Hash` (não há caso de uso óbvio para hash de
`PartitionedOrder`).

Sem `Serialize`/`Deserialize` (conforme ADR-0009: L₁ não conhece
serde). Se for necessário serializar para o `graph.json`, é
trabalho de L₃ (DTO próprio em `json_serializer`). Por ora, fora
do escopo deste prompt.

---

## Dependências externas

`01_core/Cargo.toml`: nenhuma nova. `petgraph` já está presente
(ADR-0005).

---

## Testes esperados

Localização: testes inline em
`01_core/src/rules/dsm_partitioner.rs`.

Os testes constroem `DependencyGraph`s manualmente.

1. **Grafo vazio**: `order` vazio, `sccs` vazio, `internal_boundary
   == 0`.

2. **Um nó interno**: `order` tem 1 elemento, 1 SCC trivial,
   `internal_boundary == 1`.

3. **Um nó externo**: `order` tem 1 elemento, 1 SCC trivial,
   `internal_boundary == 0`, nó está em posição 0.

4. **DAG simples (3 internos)**: A→B→C.
   - Ordem: A, B, C.
   - 3 SCCs triviais.
   - `internal_boundary == 3`.

5. **DAG com escolha alfabética**: A→C, B→C (A e B sem
   dependência mútua).
   - Ordem: A, B, C (alfabética entre os "candidatos a primeiro").
   - 3 SCCs triviais.

6. **Ciclo de 2 nós**: A→B, B→A.
   - 1 SCC cíclica com 2 nós, ordem dentro alfabética: A, B.
   - `internal_boundary == 2`.

7. **Ciclo + DAG**: A→B→A, C→A (C depende do ciclo).
   - Como C aponta para o ciclo, C vem **depois** do ciclo na
     ordem.
   - Verificar: ordem é `[A, B, C]` (A, B no ciclo, C depois).

8. **Self-loop**: A→A.
   - 1 SCC cíclica de tamanho 1 com `is_cyclic == true`.
   - Distinto de SCC trivial.

9. **Internos + externos**: 2 internos (A, B) + 1 externo (X).
   B importa X.
   - Ordem: internos primeiro (A, B), depois externo (X).
   - `internal_boundary == 2`.

10. **Múltiplos externos ordenados**: A interno, importando
    `serde::de`, `std::io`, `tokio`.
    - Ordem dos externos: `serde::de`, `std::io`, `tokio`
      (alfabética).

11. **`scc_index_per_node` aponta correctamente**: para cada
    posição i, `scc_for_position(i).range` contém i.

12. **Iteradores `internal_positions` / `external_positions`**:
    intervalos correctos.

13. **Ciclos não interagem com externos**: criar grafo com
    ciclo interno + alguns externos. Os ciclos internos
    permanecem agrupados; externos no fim.

14. **Determinismo**: rodar `partition_for_dsm` duas vezes no
    mesmo grafo (clonado); resultados idênticos.

15. **Caso realista pequeno**: 5 internos com hierarquia
    típica + 2 externos. Verificar estrutura geral plausível.

---

## Critério de aceitação do prompt

- O ficheiro `01_core/src/rules/dsm_partitioner.rs` existe e
  compila.
- Todas as structs e enums conforme especificado.
- A função `partition_for_dsm` tem a assinatura especificada.
- Os 15 testes passam.
- `cargo clippy -p crystalline-dsm-core` sem warnings.
- Módulo exportado em `01_core/src/rules/mod.rs`.
- Nenhum tipo de `petgraph` exposto na API pública.

---

## Próximos passos (fora deste prompt)

1. **Renderizador HTML (Passo 2.2)** consome `PartitionedOrder`
   directamente. Não é necessário serializar a estrutura para
   JSON no MVP.

2. **Se quisermos serializar no `graph.json`** no futuro: DTO em
   L₃ replicando `PartitionedOrder` com derive `Serialize`.
   Decisão para depois.

3. **Smoke test contra Typst**: rodar `partition_for_dsm` no
   grafo do Typst. Métricas esperadas:
   - 18 SCCs cíclicos (mesmo número de ciclos detectados).
   - ~625 SCCs triviais (`order.len() - 2 * cyclic_node_count`,
     aproximadamente).
   - `internal_boundary` em torno de 443 (nós internos do
     Typst).

---

## Limitações conhecidas

1. **Sem optimização de "bandwidth"** da matriz: dentro das
   restrições topológicas e de SCC, não tentamos minimizar a
   distância média entre nós conectados. Ferramentas comerciais
   (Lattix, Structure101) fazem isso via heurísticas
   adicionais. Fora do escopo do MVP.

2. **Sem agrupamento por crate**: a ordem mistura módulos de
   crates diferentes se a topological sort assim mandar. Não
   tentamos manter crates contíguos. Decisão futura: pode ser
   feita via pré-processamento (agrupar por crate antes do
   particionamento), mas adiciona complexidade.

3. **SCCs cíclicos não são "minimizados"**: se um SCC tem 30
   nós, todos os 30 ficam num bloco. Não tentamos quebrar SCCs
   grandes em sub-estruturas (não há, matematicamente, sem
   perder a propriedade de ciclicidade).

---

## Hash do prompt

A calcular após aprovação.
