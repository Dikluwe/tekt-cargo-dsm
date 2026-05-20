# ⚖️ ADR-0006: Nós Fantasma para Módulos Externos

**Status**: `ACEITO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm`
**Passo do roadmap relacionado**: 1.4 — Construção do grafo

---

## Contexto

O Passo 1.3 produz uma lista de `ImportEdge`s. Cada aresta tem
um `from` (`NodeId` do módulo que faz o import) e um
`target_module` (string com o caminho lógico do alvo).

A partir disto, o Passo 1.4 constrói o grafo final. Para imports
internos (`CurrentCrate` ou `WorkspaceCrate`), o alvo é um módulo
real, presente no `ModuleTree` correspondente; a aresta liga dois
nós conhecidos.

Para imports externos (`External` ou `Stdlib`), o alvo não está
em nenhum `ModuleTree`: é uma crate de fora ou da biblioteca
padrão. Não temos a estrutura interna desses crates; não vamos
parsear `serde`, `tokio`, ou `std`.

Como representar essas dependências no grafo? Três opções foram
identificadas e a escolha tem implicações na DSM produzida e nas
análises subsequentes (ciclos, particionamento).

---

## Alternativas consideradas

### Alternativa A — Resolver no Passo 1.4 sem criar nó

A aresta externa é descartada na construção do grafo. O grafo
final contém apenas dependências internas. Imports externos são
preservados em metadado separado (lista paralela), não como
arestas do grafo.

**Prós:**
- Grafo fica focado em arquitectura interna.
- Detecção de ciclos opera apenas sobre código próprio.
- DSM mais legível para análise interna.

**Contras:**
- Perda de visibilidade sobre acoplamento externo (qual módulo
  depende mais de `serde`? não dá para ver).
- Análise de surface de API externa fica fora do grafo.
- Para projectos com forte acoplamento externo, a DSM esconde
  informação valiosa.

### Alternativa B — Deixar como `Unresolved`, não criar aresta

Idem à Alternativa A, mas mais radical: mesmo casos `External` e
`Stdlib` não geram aresta. Apenas internas. Externos aparecem
apenas em log/diagnóstico.

**Prós:**
- Grafo mínimo, foco máximo no interno.

**Contras:**
- Igual à A, agravado: a categoria `External` deixa de ter
  utilidade prática no grafo.

### Alternativa C — Nó fantasma por módulo externo

Cada módulo externo referenciado (ex: `serde::de`,
`std::collections`) vira um nó no grafo, marcado como "externo".
Arestas conectam módulos internos a esses nós fantasma.

**Prós:**
- Acoplamento externo visível na DSM.
- Análise de "quais externos são usados, por quem, e quanto" é
  trivial.
- Modelo uniforme: todos os imports geram arestas.
- Particionamento DSM pode agrupar nós externos numa região
  separada (ex: linha/coluna inferior direita).

**Contras:**
- Grafo cresce. Para um projecto com 50 internos e 30 externos
  referenciados, vão a 80 nós em vez de 50.
- Ciclos envolvendo externos não fazem sentido lógico (nunca há
  ciclo `interno → externo → interno` porque externos não voltam
  ao interno).
- Risco de poluir a DSM se houver muitos externos.

---

## Decisão

**Alternativa C: nós fantasma por módulo externo.**

Cada módulo externo (`External` ou `Stdlib`) referenciado por
algum import vira um nó no grafo. Os nós são marcados com flag
`is_external` (ou via tipo enum `NodeKind`), permitindo
diferenciá-los dos nós internos.

### Granularidade do nó fantasma

Um nó por **caminho de módulo externo distinto**. Exemplos:

| Import | Nó fantasma criado |
|---|---|
| `use serde::Serialize` | `serde` (ou `serde::Serialize`? ver abaixo) |
| `use serde::de::Deserializer` | `serde::de` |
| `use std::collections::HashMap` | `std::collections` |
| `use std::io::Read` | `std::io` |

**Decisão de granularidade**: o nó fantasma representa o **módulo
alvo do import**, não o item. Para `use serde::de::Deserializer`,
o módulo alvo é `serde::de` e é esse o nó. O item
(`Deserializer`) fica registado na aresta (no campo
`imported_item` já existente).

Para imports do crate raiz (`use serde::Serialize` onde o módulo
alvo é `serde` em si), o nó é simplesmente `serde`.

### Estrutura do nó

```rust
pub struct GraphNode {
    /// Identificador canónico. Pode ser interno ou externo.
    /// Internos: "crystalline_dsm_core::entities::workspace".
    /// Externos: "serde::de", "std::collections".
    pub canonical_path: String,

    /// Tipo de nó.
    pub kind: NodeKind,
}

pub enum NodeKind {
    /// Nó representa módulo do código próprio (workspace).
    Internal {
        /// Crate ao qual o módulo pertence.
        crate_name: String,
        /// Referência ao nó no `ModuleTree` correspondente.
        tree_node_id: NodeId,
    },

    /// Nó representa módulo externo (crates.io ou stdlib).
    External {
        /// Categoria do externo.
        kind: ExternalKind,
    },
}

pub enum ExternalKind {
    /// Crate da biblioteca padrão (std, core, alloc).
    Stdlib,
    /// Crate externo (crates.io ou path/git).
    Crate,
}
```

### Regras de criação

1. Para cada `ImportEdge` com `kind == CurrentCrate` ou
   `WorkspaceCrate`: criar (ou reusar) nó `Internal` para origem
   e destino.

2. Para cada `ImportEdge` com `kind == External`: criar (ou
   reusar) nó `External { kind: Crate }` para destino. Origem é
   sempre `Internal`.

3. Para cada `ImportEdge` com `kind == Stdlib`: criar (ou reusar)
   nó `External { kind: Stdlib }` para destino.

4. Imports `Unresolved`: emitir warning, não criar nó nem aresta.
   (Diferente da Alternativa A/B: aqui o tratamento é o mesmo das
   alternativas para o caso `Unresolved`.)

5. Deduplicação por `canonical_path`: dois imports diferentes
   apontando para `std::collections` reusam o mesmo nó.

---

## Justificação

1. **Visibilidade arquitectural**: a DSM com externos mostra
   informação real e útil. Saber que 40 módulos dependem de
   `serde` é uma observação arquitectural válida.

2. **Uniformidade do modelo**: todos os imports são representados
   da mesma forma (arestas no grafo). Reduz casos especiais no
   código de análise downstream.

3. **Composição com particionamento DSM**: o algoritmo de
   particionamento (Passo 2.1) pode agrupar nós externos numa
   região separada da matriz, mantendo legibilidade.

4. **Mitigação do risco de poluição**: a flag `is_external` (ou
   `kind`) permite filtros simples na renderização ("ocultar
   externos", "mostrar só stdlib", etc) sem mudar o grafo.

5. **Detecção de ciclos**: como `petgraph::algo::tarjan_scc` opera
   sobre o grafo todo, ciclos envolvendo nós externos seriam
   detectados. Como externos não têm arestas de saída no nosso
   grafo (não parseamos suas dependências), eles não participam de
   ciclos por construção. Não é problema.

---

## Consequências

### ✅ Positivas

- DSM mostra acoplamento externo, útil para análise arquitectural.
- Modelo uniforme: arestas internas e externas seguem o mesmo
  caminho de código.
- Análises como "quais externos mais usados", "quem usa o quê"
  ficam triviais.
- Suporte natural a futuras melhorias (badges de licença,
  estatísticas de uso por crate externo).

### ❌ Negativas

- Grafo maior. Para projecto Typst, talvez 50-100 nós externos
  adicionados aos internos.
- Renderização DSM precisa considerar mais células.
- Risco de o grafo ficar visualmente poluído se não houver filtro
  na UI.

### ⚙️ Acções decorrentes

- A struct `GraphNode` e enum `NodeKind` devem ser implementadas
  conforme esta ADR no Passo 1.4.
- O renderizador HTML (Passo 2.2) deve oferecer filtro de
  visibilidade para nós externos (default: mostrar; opção:
  ocultar).
- Documentar no README que nós externos aparecem por defeito;
  como filtrar.
- Detecção de ciclos (Passo 1.5) não precisa de tratamento
  especial: nós externos não geram ciclos por terem grau de saída
  zero.

---

## Critérios de reavaliação

Esta ADR deve ser reaberta se:

1. Em uso real, a poluição visual da DSM por nós externos provar
   ser problema mesmo com filtros disponíveis.
2. Caso de uso novo exigir não modelar externos (ex: análise
   focada em comparar arquitecturas internas entre versões).
3. Performance da DSM (renderização ou análise) degradar a níveis
   inaceitáveis por causa do número de nós externos.

---

## Referências

- ADR-0001 — Criação da ferramenta.
- ADR-0004 — Granularidade do nó (caminho lógico de módulo).
- ADR-0005 — `petgraph` em L₁.
- Lattix LDM — usa nós para externos com agrupamento na DSM.
- Structure101 — mesma abordagem.
