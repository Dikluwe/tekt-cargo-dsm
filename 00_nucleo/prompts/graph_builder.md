# Prompt L0: Conversor `build_graph` (L₄)

**Camada**: L₄ (Fiação)
**Ficheiro alvo**: `04_wiring/src/graph_builder.rs`
**Passo do roadmap**: 1.4 — Construção do grafo
**Status**: IMPLEMENTADO


---

## Decisões de design prévias

- **ADR-0004**: Nó é módulo lógico.
- **ADR-0005**: `DependencyGraph` em L₁ usa `petgraph` internamente.
- **ADR-0006**: Externos viram nós fantasma.

---

## Decisões locais (assumidas neste prompt)

1. **Camada L₄ (Fiação)**: este conversor é orquestração — pega
   dados de L₁ (`ModuleTree`, `ImportEdge`) e produz outro dado
   de L₁ (`DependencyGraph`). Pertence a L₄ porque combina
   múltiplos resultados de L₃ num produto unificado, sem ele
   próprio fazer I/O.

   (Alternativa rejeitada: colocar em L₁. Rejeitada porque a
   função consome a saída de múltiplas chamadas a L₃, e a
   composição multi-chamada é responsabilidade típica de L₄.)

2. **Função pura sobre estruturas em memória**: nenhum I/O,
   nenhum acesso ao filesystem. Toda informação chega pelos
   parâmetros.

3. **Não emite warnings nem logs**: comportamento puramente
   funcional. Imports `Unresolved` são ignorados silenciosamente
   (o warning correspondente já foi emitido por L₃ no Passo 1.3).

4. **Determinismo**: dado o mesmo input, sempre o mesmo output.
   Em particular, a ordem de iteração sobre `HashMap` é
   inconsistente em Rust por defeito — não usar `HashMap` na
   construção; usar `BTreeMap` ou ordenar antes de inserir.

---

## Contexto

Este conversor é o ponto onde os artefactos dos Passos 1.1, 1.2 e
1.3 se combinam num único produto unificado:

```
Workspace (1.1)                  ┐
ModuleTree por crate (1.2)       ├──> build_graph ──> DependencyGraph
Vec<ImportEdge> por crate (1.3)  ┘
```

A complexidade está em:
- Resolver o `target_module` de cada `ImportEdge` para o
  `canonical_path` do nó destino.
- Distinguir internos (criar nó `Internal` apontando para o
  `ModuleTree` correcto) de externos (criar nó `External`).
- Lidar com casos onde o `target_module` referencia um módulo que
  não existe no `ModuleTree` indicado (imports inválidos,
  resolução parcial).

---

## Função pública principal

```rust
pub fn build_graph(
    workspace: &Workspace,
    trees: &HashMap<String, ModuleTree>,
    edges_per_crate: &HashMap<String, Vec<ImportEdge>>,
) -> DependencyGraph;
```

### Parâmetros

- `workspace`: o `Workspace` do Passo 1.1, usado para listar
  todos os crates internos e seus nomes.

- `trees`: mapa de `crate_name` para `ModuleTree`. Deve conter
  uma entrada para cada `WorkspaceMember`. Imports cujo
  `target_module` referencia um destes crates podem ser
  resolvidos para nó `Internal`.

- `edges_per_crate`: mapa de `crate_name` para a lista de
  `ImportEdge`s extraídas desse crate.

### Retorno

`DependencyGraph` completo, com:
- Um nó `Internal` para cada módulo de cada `ModuleTree`.
- Um nó `External` para cada caminho de módulo externo único
  referenciado.
- Uma aresta para cada `ImportEdge` (excluindo `Unresolved`).

A função NÃO retorna `Result`. Erros não são possíveis se os
inputs forem coerentes (responsabilidade do chamador garantir).

### Comportamento detalhado

**Fase 1: criar nós internos**

Para cada `(crate_name, tree)` em `trees`:
- Para cada `(node_id, module_node)` em `tree.all_nodes()`:
  - Adicionar nó interno ao grafo:
    ```
    graph.add_internal_node(
        module_node.canonical_path.clone(),
        crate_name.clone(),
        node_id,
    )
    ```

Após esta fase, o grafo tem todos os nós internos. Externos serão
adicionados sob demanda na Fase 3.

**Fase 2: indexar internos por canonical_path**

(Já feito pelo `path_index` interno do `DependencyGraph`. Esta
fase é conceitual — não há trabalho adicional.)

**Fase 3: processar arestas**

Para cada `(crate_name, edges)` em `edges_per_crate`:
- Para cada `edge` em `edges`:
  - Se `edge.kind == Unresolved`: ignorar, continuar.
  - Determinar o nó origem:
    - Origem é sempre interna.
    - `from_canonical = canonical do nó tree.node(edge.from)`.
    - `from_id = graph.find_node(from_canonical).expect(...)`.
      Sempre existe porque foi criado na Fase 1.
  - Determinar o nó destino conforme `edge.kind`:
    - `CurrentCrate` ou `WorkspaceCrate`:
      - Tentar `graph.find_node(edge.target_module)`.
      - Se encontrado: `to_id` é esse nó interno.
      - Se NÃO encontrado: o `target_module` aponta para módulo
        que não existe no `ModuleTree` (raro — pode acontecer com
        re-exports ou referências erradas). Tratar como externo
        com warning silencioso (criar `External::Crate`).
    - `External`:
      - `graph.add_external_node(edge.target_module.clone(),
        ExternalKind::Crate)`.
    - `Stdlib`:
      - `graph.add_external_node(edge.target_module.clone(),
        ExternalKind::Stdlib)`.
    - `Unresolved`: já tratado acima (ignorado).
  - Construir `GraphEdge` a partir do `ImportEdge`:
    ```rust
    let graph_edge = GraphEdge {
        imported_item: edge.imported_item.clone(),
        alias: edge.alias.clone(),
        is_reexport: edge.is_reexport,
        is_glob: edge.is_glob,
        raw_use_path: edge.raw_use_path.clone(),
    };
    ```
  - `graph.add_edge(from_id, to_id, graph_edge).expect(...)`.
    Não deve falhar porque ambos IDs vêm do mesmo grafo.

**Fase 4: retornar grafo construído**

---

## Casos especiais

### Imports cujo `target_module` está vazio

Pode acontecer com `use a::{self, X};` onde o "self" referencia o
módulo `a` em si. Conforme o Passo 1.3, neste caso
`target_module` pode ser construído como vazio ou como o caminho
do pai do `imported_item`.

Tratamento neste conversor: se `target_module` está vazio, usar
`imported_item` como `target_module` para fins de criação de nó.
Documentar como decisão local; revisitar se causar problema.

### `target_module` que parece interno mas não existe

Exemplo: `use crate::nonexistent::module;` num crate cujo
`ModuleTree` não tem `nonexistent::module`. Pode acontecer por:
- Código quebrado (não compila, mas o parser é permissivo).
- `pub use` que re-exporta de profundamente.

Tratamento: tratar como externo com `ExternalKind::Crate`. O
nó fica visível no grafo, marcado como externo. Documenta-se
como limitação conhecida.

### Múltiplas referências ao mesmo módulo externo

Esperado e comum. Garantido pela deduplicação do
`DependencyGraph` (mesmo `canonical_path` retorna mesmo ID).

---

## Determinismo

Para garantir output determinístico:

1. Iterar `trees` em ordem alfabética por `crate_name`. Usar
   `BTreeMap` se input vier como `HashMap`, ou ordenar `keys`
   explicitamente.

2. Iterar `edges_per_crate` em ordem alfabética por `crate_name`.

3. Dentro de cada `tree`, `all_nodes` deve ter ordem determinística
   (já garantido pela ADR-0004 / Passo 1.2: ordem de inserção).

4. Dentro de cada `Vec<ImportEdge>`, processar na ordem em que
   foram extraídos (ordem do parser `syn`, que é determinística).

---

## Dependências externas

`04_wiring/Cargo.toml`:
- `crystalline-dsm-core` (L₁): tipos `Workspace`, `ModuleTree`,
  `ImportEdge`, `DependencyGraph`, `GraphNode`, `GraphEdge`,
  `NodeKind`, `ExternalKind`, `ImportKind`.
- Nenhuma dependência externa nova.

Não usar:
- `syn`, `cargo_metadata`, `petgraph` directamente. Toda
  manipulação de grafo via API de `DependencyGraph`.

---

## Testes esperados

### Testes unitários (no próprio ficheiro)

Construir inputs literalmente, sem filesystem. Casos:

1. **Workspace vazio**: `trees` vazio, `edges_per_crate` vazio.
   Resultado: grafo sem nós nem arestas.

2. **Um crate sem imports**: 1 `ModuleTree` com raiz só.
   `edges_per_crate` vazio.
   Resultado: 1 nó interno, 0 arestas.

3. **Um crate, import interno (CurrentCrate)**: `ModuleTree` com
   raiz + 1 filho. 1 `ImportEdge` com `kind = CurrentCrate`,
   `from = raiz`, `target_module = canonical do filho`.
   Resultado: 2 nós internos, 1 aresta entre eles.

4. **Um crate, import externo**: `ModuleTree` com raiz. 1
   `ImportEdge` com `kind = External`, `target_module = "serde::de"`.
   Resultado: 1 nó interno + 1 nó externo, 1 aresta.

5. **Um crate, import stdlib**: análogo a 4, mas
   `kind = Stdlib`, `target_module = "std::collections"`.
   Resultado: nó externo com `ExternalKind::Stdlib`.

6. **Múltiplos imports do mesmo externo**: 3 `ImportEdge`s
   apontando para `serde::de` de módulos internos diferentes.
   Resultado: 1 nó externo `serde::de`, 3 arestas.

7. **Workspace com 2 crates, import cross-crate**: 2 `ModuleTree`s,
   um `ImportEdge` com `kind = WorkspaceCrate` apontando do
   crate A para crate B.
   Resultado: 2+ nós internos, 1 aresta cross-crate.

8. **Import com `Unresolved` é ignorado**: 1 `ImportEdge` com
   `kind = Unresolved`. Resultado: grafo sem arestas adicionais
   (nó destino não criado).

9. **Determinismo**: construir grafo duas vezes com mesmo input.
   Os dois grafos têm mesma contagem de nós, arestas, e os mesmos
   `canonical_path`s. (Comparação estrutural — `PartialEq` em
   `DependencyGraph` não existe, então comparar campos via
   métodos.)

10. **`target_module` inválido vira externo**: `ImportEdge` com
    `kind = CurrentCrate`, `target_module = "<crate>::nao_existe"`.
    Resultado: nó externo criado, marcado como
    `ExternalKind::Crate`.

11. **Import glob**: `ImportEdge` com `is_glob = true`,
    `imported_item = "*"`. Resultado: aresta criada com `is_glob`
    preservado.

12. **Import com alias**: `ImportEdge` com
    `alias = Some("Bar")`. Resultado: aresta criada com `alias`
    preservado.

13. **Re-export**: `ImportEdge` com `is_reexport = true`.
    Resultado: aresta criada com `is_reexport` preservado.

### Testes de integração

Localização: `04_wiring/tests/graph_builder_integration_tests.rs`.

1. **Pipeline completo num crate trivial**: usar fixture
   `imports-simple` do Passo 1.3.
   - Resolver workspace via `cargo_metadata_reader`.
   - Traversar módulos via `module_traverser`.
   - Extrair imports via `import_extractor`.
   - Construir grafo via `build_graph`.
   - Verificar: 1 nó interno, 1 nó externo (`a::b`), 1 aresta.

2. **Pipeline completo em workspace multi-crate**: usar fixture
   `imports-workspace`.
   - Pipeline completo.
   - Verificar: 2+ nós internos, 1 aresta cross-crate
     `WorkspaceCrate`.

3. **Smoke test no `lab/typst-original/`** (opcional,
   `#[ignore]`): executar pipeline completo. Verificar:
   - Sem panic.
   - `node_count` > 100.
   - `edge_count` > 200.

---

## Critério de aceitação do prompt

- O ficheiro `04_wiring/src/graph_builder.rs` existe e compila.
- A função `build_graph` tem a assinatura especificada.
- Os 13 testes unitários e 2 testes de integração passam.
- `cargo clippy --all-targets` sem warnings novos.
- Sem `panic!`, `unwrap()` em código de produção (o uso de
  `expect()` é permitido onde a invariante for óbvia, ex:
  "from_id sempre existe porque foi criado na Fase 1").
- Nenhuma importação de `syn`, `cargo_metadata`, `petgraph` neste
  ficheiro.
- Módulo exportado em `04_wiring/src/lib.rs` (ou equivalente).

---

## Limitações conhecidas e documentadas

1. `target_module` que aponta para módulo interno inexistente é
   tratado como externo. Caso raro, mas documentar.
2. Re-exports profundos não são "seguidos" — a aresta criada
   reflecte o `use` literal, não o item final resolvido.
3. Imports com `Unresolved` são silenciosamente ignorados. O
   warning correspondente já foi emitido em L₃ (Passo 1.3).

---

## Histórico de Revisões

- **2026-05-20**: Implementado. Todos os 13 testes unitários e 2 testes de integração passam com 100% de sucesso.

