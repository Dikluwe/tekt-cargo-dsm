# Prompt L0: Particionador DSM (L₁)

**Camada**: L₁ (Núcleo)
**Ficheiro alvo**: `01_core/src/rules/dsm_partitioner.rs`
**Passo do roadmap**: 2.1 — Ordenação topológica para DSM
**Status**: IMPLEMENTADO (revisado)

---

## Nota retroactiva sobre a divergência identificada

A versão inicial deste prompt continha inconsistência interna
entre os testes esperados:

- Testes 4 e 5 foram redigidos como se a ordem seguisse
  topological sort directo (dependente antes da dependência:
  A→B→C ⇒ `[A, B, C]`).
- Teste 7 foi redigido como se seguisse a convenção DSM clássica
  Steward/Browning (dependência antes do dependente: A↔B + C→A ⇒
  `[A, B, C]`).

São regras opostas para "quem vem antes". A implementação
adoptou a convenção DSM clássica em todos os casos (regra única
e coerente), e os testes 4 e 5 foram ajustados para reflectir
essa convenção. Esta revisão do prompt incorpora a correcção
para que leitores futuros não tenham que reconstruir a
discussão.

**A regra final é**:

- **Dependência vem antes na ordem; dependente vem depois.**
- Em DSM Steward/Browning (formato "linha depende de coluna"),
  as marcas ficam abaixo da diagonal triangular.
- A→B significa "A depende de B" ⇒ B aparece antes de A na
  ordem.

Esta convenção é a usada por Lattix LDM, Structure101, e a
literatura clássica de DSM (Steward 1981; Browning 2001).
Ferramentas como NDepend usam a convenção oposta ("linha é
usada por coluna"); ambas existem na literatura, e nenhuma é
"correta" em absoluto. A escolha aqui é por consistência interna
e alinhamento com as referências académicas dominantes.

---

## Decisões de design prévias

- **ADR-0005**: `petgraph` em L₁. Pode ser usado directamente,
  incluindo `petgraph::algo::tarjan_scc`.
- **ADR-0006**: Nós externos têm grau de saída zero por
  construção.

---

## Decisões locais (assumidas neste prompt)

1. **Algoritmo**: Tarjan SCC + topological sort (Kahn) sobre o
   grafo condensado das SCCs. Algoritmo canónico de DSM.

2. **Convenção de ordenação**: Steward/Browning — dependência
   vem antes do dependente.

3. **Nós externos forçados para o final**: a região da matriz é
   dividida em duas partes contíguas: internos primeiro,
   externos depois. Isola visualmente dependências externas do
   código próprio.

4. **Modelo uniforme de SCC**: todo nó pertence a algum SCC,
   mesmo que trivial (1 nó, sem self-loop). Simplifica o
   modelo de consumo.

5. **Contiguidade dos SCCs cíclicos**: garantida pelo algoritmo
   (propriedade matemática de SCC + condensação).

6. **Ordenação determinística**:
   - Entre SCCs no Kahn: heap por menor `canonical_path` de cada
     SCC. Em empate de "fila pronta", o SCC com menor
     `canonical_path` é processado primeiro.
   - Dentro de SCC: alfabética por `canonical_path`.
   - Resultado byte-determinístico para o mesmo grafo.

7. **Localização em `rules/`**: segundo algoritmo em L₁ (depois
   do `cycle_detector`).

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

```rust
pub struct PartitionedOrder {
    /// Ordem linear final dos nós.
    /// Índice 0 é a primeira linha/coluna da matriz DSM (topo
    /// e esquerda).
    /// Nós em posições mais baixas são dependências (folhas);
    /// nós em posições mais altas são dependentes.
    pub order: Vec<GraphNodeId>,

    /// Para cada posição em `order`, o índice do SCC em `sccs`
    /// ao qual o nó pertence.
    pub scc_index_per_node: Vec<usize>,

    /// Definição dos SCCs presentes no grafo.
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

```rust
pub struct SccBlock {
    /// Posições no `PartitionedOrder.order` que este SCC ocupa.
    /// `range.start` é inclusivo, `range.end` é exclusivo.
    pub range: std::ops::Range<usize>,

    /// `true` se este SCC representa um ciclo real:
    /// - Tamanho > 1 (múltiplos nós), OU
    /// - Tamanho 1 com self-loop (nó depende de si mesmo).
    /// `false` para SCC trivial (1 nó sem self-loop).
    pub is_cyclic: bool,
}
```

Notas:
- O tamanho do SCC é `range.len()`.
- Para iterar os nós de um SCC: `partition.order[scc.range.clone()]`.

---

## Função pública principal

```rust
pub fn partition_for_dsm(graph: &DependencyGraph) -> PartitionedOrder;
```

### Comportamento

1. **Separar internos de externos** em listas distintas.

2. **Particionar a região interna**:

   a. Construir subgrafo `internal_subgraph` contendo apenas nós
      internos e arestas entre eles.

   b. Aplicar `petgraph::algo::tarjan_scc(internal_subgraph)`.

   c. Para cada SCC com tamanho > 1: marcar como cíclico.

   d. Para cada SCC com tamanho 1: verificar self-loop. Se sim,
      cíclico; se não, trivial.

   e. **Ordenar os nós dentro de cada SCC** alfabeticamente por
      `canonical_path`.

   f. **Construir o grafo condensado** das SCCs (um super-nó por
      SCC; arestas entre super-nós conforme arestas entre SCCs no
      original).

   g. **Topological sort do condensado via Kahn**:
      - Calcular `in_degree` de cada super-nó.
      - Inicializar heap com super-nós de `in_degree == 0`.
      - Ordenar empate na heap pelo menor `canonical_path` do
        SCC.
      - Processar: remover do heap, adicionar à ordem, decrementar
        `in_degree` dos vizinhos, adicionar à heap os que ficaram
        com `in_degree == 0`.

      **Convenção crítica para a direcção das arestas no
      condensado**: para que dependências apareçam antes de
      dependentes (Steward/Browning), uma aresta `A→B` no grafo
      original (A depende de B) deve ser convertida em **B
      antes A na ordem**. Implementação: o algoritmo Kahn deve
      processar primeiro os nós que **não têm ninguém apontando
      para eles dentro do contexto invertido** — ou seja, os nós
      que são "folhas de dependência" (não dependem de ninguém
      interno). Na prática, isto corresponde a inverter a
      contagem: contar `in_degree` no sentido **inverso** das
      arestas originais.

      Formulação equivalente sem inversão explícita: tratar as
      arestas como `to → from` no grafo condensado. Quando A
      depende de B, registrar como `B → A` no condensado.

   h. **Expansão final**: para cada SCC na ordem topológica
      resultante de (g), adicionar todos os seus nós ao `order`
      na ordem alfabética definida em (e).

3. **Adicionar externos ao final do `order`**:
   - Externos em ordem alfabética por `canonical_path`.
   - Cada externo forma seu próprio SCC trivial.

4. **Construir `scc_index_per_node`** mesma ordem que `order`.

5. **`internal_boundary`** = quantidade de internos.

6. Retornar `PartitionedOrder`.

### Tratamento de grafo vazio

- Grafo com 0 nós: `order` vazio, `sccs` vazio,
  `internal_boundary == 0`.

### Tratamento de grafo só com externos

- `internal_boundary == 0`. Externos em ordem alfabética.

### Tratamento de grafo só com internos

- `internal_boundary == order.len()`. Sem região externa.

---

## Operações de consumo (auxiliares públicas)

```rust
impl PartitionedOrder {
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn node_at(&self, index: usize) -> GraphNodeId;
    pub fn scc_for_position(&self, index: usize) -> &SccBlock;
    pub fn cyclic_scc_count(&self) -> usize;
    pub fn trivial_scc_count(&self) -> usize;
    pub fn internal_positions(&self) -> std::ops::Range<usize>;
    pub fn external_positions(&self) -> std::ops::Range<usize>;
}
```

---

## Derives obrigatórios

- `Debug`, `Clone`, `PartialEq`, `Eq` em todas as structs.
- Sem `Hash`, sem `Serialize`/`Deserialize` (vide ADR-0009).

---

## Dependências externas

`01_core/Cargo.toml`: nenhuma nova. `petgraph` já presente.

---

## Testes esperados

Localização: testes inline em
`01_core/src/rules/dsm_partitioner.rs`.

1. **Grafo vazio**: `order` vazio, `sccs` vazio,
   `internal_boundary == 0`.

2. **Um nó interno**: `order` tem 1 elemento, 1 SCC trivial,
   `internal_boundary == 1`.

3. **Um nó externo**: `order` tem 1 elemento, 1 SCC trivial,
   `internal_boundary == 0`, nó em posição 0.

4. **DAG simples (3 internos)**: A→B→C (A depende de B, B
   depende de C).
   - Ordem esperada: `[C, B, A]` (dependência mais funda
     primeiro: C, depois B, depois A).
   - 3 SCCs triviais.
   - `internal_boundary == 3`.

5. **DAG com folha comum**: A→C, B→C (A e B dependem ambos de
   C; A e B sem relação mútua).
   - Ordem esperada: `[C, A, B]` (C primeiro como dependência
     comum; A e B depois, em ordem alfabética entre si).
   - 3 SCCs triviais.

6. **Ciclo de 2 nós**: A→B, B→A.
   - 1 SCC cíclica com 2 nós, ordem dentro alfabética: `[A, B]`.
   - `internal_boundary == 2`.

7. **Ciclo + DAG**: A↔B, C→A (A e B em ciclo; C depende de A).
   - C depende do ciclo, então o ciclo é "dependência" e C é
     "dependente". O ciclo vem antes; C depois.
   - Ordem esperada: `[A, B, C]` (A e B no SCC cíclico, em ordem
     alfabética; depois C como SCC trivial).

8. **Self-loop**: A→A.
   - 1 SCC cíclica de tamanho 1 com `is_cyclic == true`.
   - Distinto de SCC trivial.

9. **Internos + externos**: 2 internos (A, B) + 1 externo (X).
   B importa X (B depende de X).
   - Internos primeiro, externos depois.
   - Dentro dos internos, ordem natural: A não depende de
     nada, B depende de X (mas X é externo, não conta no
     particionamento interno). A e B no mesmo nível
     topológico interno → ordem alfabética: `[A, B, X]`.
   - `internal_boundary == 2`.

10. **Múltiplos externos ordenados**: A interno, importando
    `serde::de`, `std::io`, `tokio`.
    - Externos em ordem alfabética: `serde::de`, `std::io`,
      `tokio`.

11. **`scc_index_per_node` aponta correctamente**: para cada
    posição i, `scc_for_position(i).range` contém i.

12. **Iteradores `internal_positions` / `external_positions`**:
    intervalos correctos.

13. **Ciclos não interagem com externos**: ciclo interno +
    alguns externos. Ciclos internos permanecem agrupados;
    externos no fim.

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
   directamente.

2. **Se quisermos serializar no `graph.json`** no futuro: DTO em
   L₃ replicando `PartitionedOrder` com derive `Serialize`.

3. **Smoke test contra Typst**: rodar `partition_for_dsm` no
   grafo do Typst. Métricas esperadas:
   - 18 SCCs cíclicos (mesmo número de ciclos detectados).
   - `internal_boundary` em torno de 443 (nós internos do
     Typst).

---

## Limitações conhecidas

1. **Sem optimização de "bandwidth"** da matriz: dentro das
   restrições topológicas e de SCC, não tentamos minimizar a
   distância média entre nós conectados.

2. **Sem agrupamento por crate**: a ordem mistura módulos de
   crates diferentes se a topological sort assim mandar.

3. **SCCs cíclicos não são "minimizados"**: SCCs grandes ficam
   num bloco. Não tentamos quebrar.

---

## Hash do prompt

A calcular após aprovação.
