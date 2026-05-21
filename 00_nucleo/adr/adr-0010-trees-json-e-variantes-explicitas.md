# ⚖️ ADR-0010: Artefacto Auxiliar `trees.json` e Variantes Explícitas do `NodeKind`

**Status**: `ACEITO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm`
**Passo do roadmap**: 1.4 (parte JSON, complemento) — fechar M1

---

## Contexto

A implementação do `json_serializer` (ADR-0009) expôs uma
limitação real do modelo: o campo `tree_node_id` em
`NodeKind::Internal` referencia uma `ModuleTree` viva. Quando o
grafo é serializado em JSON e deserializado depois, a árvore
original não está disponível, e a referência não pode ser
reconstruída.

O agente resolveu provisoriamente expondo `NodeId::placeholder()`
em L₁, retornando um sentinela (`usize::MAX`). A implementação
funciona, mas tem dois problemas:

1. **Letra do encapsulamento violada**: L₁ ganhou um método
   público desenhado especificamente para facilitar a
   deserialização em L₃. Acoplamento invertido (L₁ pensando em L₃).

2. **Sentinela mágica**: `usize::MAX` parece um `NodeId` válido.
   Quem consumir o campo sem ler a documentação pode usar como se
   fosse referência real.

Análise empírica mostrou que **nenhum consumidor actual** do
`tree_node_id` existe: nem o detector de ciclos, nem o smoke
test, nem o futuro renderizador HTML precisam do campo. O futuro
hipotético em que faria falta é "navegação do nó do grafo para o
ficheiro fonte", o que pode ser resolvido por outras vias.

Duas decisões emergem desta análise e ficam registadas nesta ADR:

1. **Como manter a informação da `ModuleTree` para casos
   futuros, sem inflar o JSON principal.**
2. **Como modelar o estado em memória do `NodeKind::Internal` de
   forma honesta, sem placeholders mágicos.**

---

## Decisões

### Decisão 1: artefacto auxiliar `trees.json` (Forma 4)

A informação completa das `ModuleTree`s, quando o utilizador
quiser preservá-la, vai num ficheiro JSON separado:
`trees.json`, lado a lado com o `graph.json` principal.

**Geração**: controlada por flag CLI `--emit-trees`. Sem a flag,
apenas `graph.json` é gerado.

**Localização**: o `trees.json` vai no mesmo diretório do
`graph.json`, com nome fixo.

**Conteúdo**: as `ModuleTree`s completas (cada `ModuleNode` com
`canonical_path`, `crate_name`, `source_file`, `is_inline`,
`has_custom_path`, etc), serializadas de forma análoga ao
`graph.json` (DTOs em L₃, serde derive em DTOs, ordem canónica).

**Ponte entre os dois ficheiros**: `canonical_path`. Quem ler os
dois ficheiros casa nós do grafo com módulos da árvore por essa
chave. **Não há índice numérico cruzado**.

**Schema versionado**: `trees.json` carrega seu próprio
`schema_version` independente do `graph.json`. Versão inicial:
`"1.0.0"`.

### Decisão 2: `NodeKind::Internal` substituído por duas variantes

A variante `Internal` é dividida em duas, conforme a presença ou
ausência do `tree_node_id`:

```rust
pub enum NodeKind {
    /// Nó interno com link para a `ModuleTree` que o produziu.
    /// Estado típico quando o grafo é construído em memória a
    /// partir de uma árvore real.
    InternalWithTree {
        crate_name: String,
        tree_node_id: NodeId,
    },

    /// Nó interno sem link para `ModuleTree`.
    /// Estado típico quando o grafo é reconstruído a partir de
    /// JSON, ou quando construído por uma fonte que não fornece
    /// árvore.
    InternalWithoutTree {
        crate_name: String,
    },

    /// Nó externo (crates.io ou stdlib).
    External {
        kind: ExternalKind,
    },
}
```

Razões:

- A diferença "tem link com árvore" / "não tem link" passa a ser
  parte do tipo. Quem ler o código sabe imediatamente o que cada
  variante contém.
- O compilador força tratamento explícito de ambos os casos em
  `match`. Sem placeholder, sem sentinela.
- O nome descreve o conteúdo da variante (presença ou ausência
  do campo), não a história de como o nó chegou.

### Decisão 3: `tree_node_id` NÃO vai para o JSON

O `tree_node_id` é índice opaco do `petgraph` interno, instável
entre execuções. Não vai para nenhum dos dois JSONs (`graph.json`
nem `trees.json`).

Consequência: ao deserializar `graph.json`, todos os nós internos
viram `InternalWithoutTree`. Para reconstruir as referências (se
desejado), o consumidor lê também `trees.json` e casa pelo
`canonical_path`.

### Decisão 4: `NodeId::placeholder()` é removido de L₁

A função `NodeId::placeholder()`, adicionada provisoriamente em
`01_core/src/entities/module_tree.rs`, é removida. Deixa de ser
necessária com a Decisão 2 (deserialização produz
`InternalWithoutTree`, sem precisar de `NodeId` sentinela).

### Decisão 5: API de construção do `DependencyGraph` recebe duas funções

Em vez de uma única `add_internal_node`, agora são duas funções
públicas, alinhadas com as variantes:

```rust
impl DependencyGraph {
    pub fn add_internal_node_with_tree(
        &mut self,
        canonical_path: String,
        crate_name: String,
        tree_node_id: NodeId,
    ) -> GraphNodeId;

    pub fn add_internal_node_without_tree(
        &mut self,
        canonical_path: String,
        crate_name: String,
    ) -> GraphNodeId;
}
```

Quem constrói o grafo escolhe explicitamente. `graph_builder` em
L₄ usa `add_internal_node_with_tree`. `json_serializer::from_dto`
em L₃ usa `add_internal_node_without_tree`.

### Decisão 6: `trees_serializer` é módulo novo em L₃

Análogo ao `json_serializer`, mas para `ModuleTree`s. Vive em
`03_infra/src/trees_serializer.rs`. Define DTOs próprios para
módulos e árvores, com derive `Serialize`/`Deserialize`. L₁
permanece sem `serde`.

---

## Alternativas consideradas

### Alternativa A — `Option<NodeId>` no campo

```rust
NodeKind::Internal { crate_name: String, tree_node_id: Option<NodeId> }
```

Pró: menos código.
Contra: a razão da ausência não está modelada; `None` poderia
significar qualquer coisa.

Rejeitada em favor da Decisão 2 (variantes explícitas).

### Alternativa B — manter `NodeId::placeholder()` com ADR justificando

Pró: nada muda no código atual.
Contra: sentinela mágica continua presente. Risco real de uso
incorrecto.

Rejeitada.

### Alternativa C — remover totalmente o `tree_node_id`

Pró: simplicidade máxima.
Contra: perde a possibilidade futura de navegar do grafo para a
árvore. Reintroduzir depois exige mudança maior.

Rejeitada em favor da preservação opcional via `trees.json`.

### Alternativa D — `trees.json` embutido em `graph.json`

```json
{
  "graph": { ... },
  "trees": { ... }  // opcional
}
```

Pró: um único ficheiro.
Contra: schema do `graph.json` muda conforme presença ou ausência
da seção. Consumidores precisam tratar a variabilidade.

Rejeitada em favor de dois ficheiros físicos (Forma 4.1).

---

## Justificação

1. **Honestidade do tipo**: `InternalWithTree` e
   `InternalWithoutTree` colocam a informação relevante no nome.
   Não há valor mágico.

2. **Separação de artefactos**: `graph.json` permanece enxuto e
   estável. `trees.json` é opt-in para quem precisa.

3. **Coerência com YAGNI controlado**: a informação da árvore
   está disponível **se** o utilizador pedir. Sem pedir, não
   onera o output. Sem perder a possibilidade de uso futuro.

4. **Sem novas dependências em L₁**: a remoção do
   `placeholder()` reverte o desvio identificado na revisão do
   `json_serializer`. L₁ volta a estar totalmente alinhado com a
   intenção da ADR-0009 ("L₁ não conhece JSON nem `serde`").

---

## Consequências

### ✅ Positivas

- L₁ recupera pureza: sem `placeholder()`, sem sentinela
  mágica.
- O `match` em `NodeKind` é forçado pelo compilador a tratar
  cada caso. Sem omissão silenciosa.
- O `graph.json` continua compacto e estável; quem só quer o
  grafo paga só pelo grafo.
- A funcionalidade hipotética futura ("navegar do nó para o
  ficheiro fonte") tem caminho claro: ler `trees.json` e casar
  por `canonical_path`.

### ❌ Negativas

- Mudança em código `IMPLEMENTADO` (`dependency_graph.rs`,
  `graph_builder.rs`, `json_serializer.rs`, e testes
  correspondentes).
- Novo módulo `trees_serializer.rs` a manter (com seus DTOs,
  testes, schema próprio).
- O `match` em `NodeKind` ganha uma variante a mais — todos os
  consumidores precisam ser actualizados.
- A flag `--emit-trees` é nova superfície de configuração.

### ⚙️ Acções decorrentes

1. Atualizar `01_core/src/entities/dependency_graph.rs`:
   - Renomear `NodeKind::Internal` para
     `NodeKind::InternalWithTree`.
   - Adicionar `NodeKind::InternalWithoutTree { crate_name }`.
   - Substituir `add_internal_node` por
     `add_internal_node_with_tree` e `add_internal_node_without_tree`.

2. Remover `NodeId::placeholder()` de
   `01_core/src/entities/module_tree.rs`.

3. Atualizar `04_wiring/src/graph_builder.rs`:
   - Chamar `add_internal_node_with_tree` (era
     `add_internal_node`).

4. Atualizar `03_infra/src/json_serializer.rs`:
   - DTOs: `NodeKindDto::Internal { crate_name }` (sem
     campo de `tree_node_id`).
   - `from_dto`: usar `add_internal_node_without_tree`.
   - `to_dto`: mapear ambas as variantes
     (`InternalWithTree` e `InternalWithoutTree`) para o mesmo
     `NodeKindDto::Internal { crate_name }`.

5. Criar `03_infra/src/trees_serializer.rs` (módulo novo).

6. Em L₄, adicionar flag `--emit-trees` e lógica de gravação.

7. Atualizar testes:
   - `dependency_graph.rs`: ajustar testes para usar as novas
     APIs.
   - `json_serializer.rs`: round-trip de `InternalWithTree`
     produz `InternalWithoutTree` no destino (documentar e
     testar).
   - `graph_builder.rs`: ajustar para nova API.

8. Atualizar prompts `IMPLEMENTADO`s afectados:
   - `dependency_graph.md` → `IMPLEMENTADO (revisado)`.
   - `graph_builder.md` → `IMPLEMENTADO (revisado)`.
   - `json_serializer.md` → `IMPLEMENTADO (revisado)`.

9. Documentar schemas:
   - `docs/json-schema-v1.md` (graph.json).
   - `docs/trees-schema-v1.md` (trees.json).

---

## Critérios de reavaliação

Esta ADR deve ser reaberta se:

1. Surgir consumidor real que precise do `tree_node_id` em todos
   os nós (com ou sem `trees.json`), forçando uma reformulação.
2. Os dois ficheiros separados (`graph.json` + `trees.json`)
   provarem ser inconvenientes na prática (sinal: utilizadores
   esquecem de gerar o `trees.json` quando precisam dele).
3. O schema independente do `trees.json` divergir do
   `graph.json` a ponto de ficar custoso manter ambos
   compatíveis.

---

## Referências

- ADR-0004 — Granularidade do nó.
- ADR-0006 — Nós fantasma (definição original do
  `tree_node_id`).
- ADR-0009 — Serialização em L₃ via DTOs.
- Implementação do `json_serializer` (que expôs o problema do
  placeholder).
