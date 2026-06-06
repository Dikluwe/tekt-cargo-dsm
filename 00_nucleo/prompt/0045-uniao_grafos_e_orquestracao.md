# Prompt: `unir_grafos` (L1) + `montar_grafo_workspace` (L4) — o grafo de workspace

**Camada**: L1 — Núcleo (`lente_core`, a união) + L4 — Fiação (`lente_wiring`, a
orquestração)
**Criado em**: 2026-06-05
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0045-uniao_grafos_e_orquestracao.md`)
**Decisões de origem**: laudo 0039 (unir extrações **por path**, não por id —
instável; "0 arestas soltas"); laudo 0040 (sinal de **nó fantasma** — path cujo
crate-dono não produz mas dependentes referenciam); laudo 0041 (resolver por
crate **antes** de unir; e o achado de que neste repo as colisões são folhas de
raio 0, logo a resolução **não** cria fantasma cross-crate); laudo 0044 (a
fundação L3: `enumerar_membros`, `versao_toolchain`, `extrair_grafo_cacheado`).
**Pré-requisito**: laudo 0044 (a extração cacheada e a enumeração existem em
produção).
**Arquivos afetados**: `01_core/src/` (a união, módulo novo), `04_wiring/src/lib.rs`
(orquestração + extração de um helper de resolução), testes de ambos.

---

## Contexto

A fundação L3 (0044) já dá a extração por crate cacheada e a lista de membros.
Faltam duas peças para o grafo de workspace de pé: a **união por path** (L1) e a
**orquestração** (L4). A **resolução** de colisões não é código novo — a fiação
já a faz (correta após 0042); este prompt a **reusa**, extraindo-a num helper.

Ao fim, `montar_grafo_workspace` devolve um grafo único, resolvido e unificado de
**todos** os crates — a fundação do motor, pronta para o modo `--diff` (L2, o
próximo passo do produto).

---

## Restrições estruturais

- **L1 — `unir_grafos` é pura.** Só stdlib (`HashMap`/`Vec` para agrupar por
  path), **sem petgraph, sem deps novas**. `cargo tree -p lente_core` continua só
  o crate.
- **L4 — `montar_grafo_workspace` é composição.** Importa L1 (a união), L3 (0044),
  e reusa a resolução (via o helper extraído). Sem deps externas novas.
- **Retrocompat**: `calcular_raio_de_alvo` **não muda de comportamento** — a
  extração do helper de resolução é refator que preserva o que ele faz.

---

## Parte 1 — L1: `unir_grafos` (`lente_core`)

### Tipos

```rust
pub struct GrafoCrate { pub crate_name: String, pub grafo: Grafo }
pub struct ResultadoUniao { pub grafo: Grafo, pub fantasmas: Vec<Fantasma> }
pub struct Fantasma { pub path: Path, pub referenciado_por: Vec<String> }

pub fn unir_grafos(grafos: Vec<GrafoCrate>) -> ResultadoUniao
```

### Semântica (o que o 0039/0040 validaram)

Recebe os grafos **já resolvidos por crate**, cada um etiquetado com o nome do
seu crate (a etiqueta é o que permite detectar fantasma).

**Nós — juntar por path:**
1. Agrupar todos os nós (de todos os grafos) por `path`.
2. Para cada path P:
   - **Definição vs referência**: um nó é **definição** se o crate do grafo de
     onde ele veio é igual ao `no.crate_name` (o crate dono do item). É
     **referência** se diferem (outro crate que usa o item — `lente_infra` carrega
     um nó-referência de `lente_core::Grafo`, p.ex.).
   - Se existe a **definição** (um nó cujo `source_crate == no.crate_name`) →
     usar a definição; descartar as referências (são idênticas à definição módulo
     `id` — ambas leem a mesma fonte via rust-analyzer).
   - Se **não** existe definição (só referências) → **fantasma**: P é
     referenciado mas o crate-dono não o produziu (item renomeado/removido, ou
     uma referência por nome antigo a algo que a resolução renomeou). Manter **um
     nó-representante** (uma das referências) no grafo — para as arestas
     religarem, **0 arestas soltas** — e registrar um `Fantasma { path: P,
     referenciado_por: [crates que o referenciam] }`.

**Arestas — religar por path:**
3. Coletar todas as arestas (de todos os grafos). Deduplicar arestas idênticas
   (mesma `from`/`to`/`relation`).
4. Reindexar: atribuir `id` novo e único (sequencial, 0..N) a cada nó unido.
5. Para cada aresta, religar `id_from`/`id_to` aos **ids novos** dos nós cujos
   paths são `from`/`to`. (O `from`/`to` em path é a verdade do religamento — o
   `id` antigo é por-crate e colide entre crates; o raio opera por path, então
   reindexar é seguro.)

**Saída**: `ResultadoUniao { grafo: <nós unidos, arestas religadas, ids novos>,
fantasmas }`.

### Invariantes da saída

- Paths únicos (cada path = um nó no grafo unido).
- `id` únicos (reindexados).
- Integridade referencial: todo `id_from`/`id_to` casa com um nó (0 soltas — os
  fantasmas mantêm representante).
- Determinística: unir duas vezes dá o mesmo grafo (ordenar onde iterar; nada de
  ordem de `HashMap` vazando para a saída).

---

## Parte 2 — L4: helper de resolução + `montar_grafo_workspace` (`lente_wiring`)

### 2a. Extrair o helper de resolução (refator, preserva comportamento)

Hoje a resolução vive **dentro** do `calcular_raio_de_alvo` (os helpers
`detectar_colisoes` + `resolver_uma_colisao`, laudo 0019). Extrair:

```rust
fn resolver_colisoes(grafo: Grafo) -> Result<Grafo, ErroLente>
```

— o laço "para cada path colidente, investigar o primeiro par
(`lente_investiga::investigar` com `fontes` vazias — E2 em quarentena, laudo
0014/0019 D4), aplicar `lente_resolve::aplicar`". O `calcular_raio_de_alvo` passa
a **chamar** esse helper, sem mudar o que faz. Os testes existentes do
`lente_wiring` são a guarda (não-regressão).

### 2b. `montar_grafo_workspace`

```rust
pub fn montar_grafo_workspace(raiz: &Path) -> Result<GrafoWorkspace, ErroLente>
// GrafoWorkspace { grafo: Grafo, fantasmas: Vec<Fantasma> } (ou reusar ResultadoUniao)
```

Passos:
1. `lente_infra::enumerar_membros(raiz)` (L3) → membros.
2. `lente_infra::versao_toolchain()` (L3) → versão, **uma vez**.
3. Para cada membro: `lente_infra::extrair_grafo_cacheado(membro, raiz, &versao)`
   (L3) → `Grafo` do crate.
4. Para cada `Grafo`: `resolver_colisoes` (2a) → grafo resolvido do crate.
5. `lente_core::unir_grafos(Vec<GrafoCrate { crate_name: membro.nome, grafo }>)`
   (L1) → `ResultadoUniao`.
6. Devolver o grafo unido + os fantasmas.

`ErroLente` ganha uma variante `Workspace(lente_infra::ErroWorkspace)` (os erros
de enumeração/cache/toolchain do 0044), com `From` para `?`.

---

## O que NÃO muda

- O **comportamento** do `calcular_raio_de_alvo` (só refatorado para chamar
  `resolver_colisoes`).
- A extração por crate e o cache (0044) — usados como estão.
- A **lógica** de resolução (reusada, não reescrita).
- As funções existentes do `lente_core` — `unir_grafos` é aditiva.

---

## Critérios de Verificação

```
# União (L1, pura — sem fork)
Dado o grafo do crate A (com um nó-referência a "B::Foo") e o grafo do crate B
(que define "B::Foo"), etiquetados "A" e "B"
Quando unir_grafos
Então o grafo unido tem UM nó "B::Foo" (a definição de B)
E a aresta de A para "B::Foo" religa a esse nó (0 arestas soltas)
E os ids são novos e únicos; os paths, únicos

Dado o grafo de A referenciando "B::Foo", mas o grafo de B NÃO tem "B::Foo"
Quando unir_grafos
Então fantasmas contém Fantasma { path: "B::Foo", referenciado_por: ["A"] }
E o grafo mantém um nó-representante para "B::Foo" (0 arestas soltas)

Dado nós idênticos para o mesmo path (referência de A == definição de B)
Quando unir_grafos
Então viram UM só nó no grafo unido

Dado três crates em cadeia A→B→C
Quando unir_grafos
Então as arestas cross-crate ligam a cadeia; 0 soltas

Dado o mesmo conjunto de grafos
Quando unir_grafos duas vezes
Então grafos iguais (determinístico)

# Orquestração (L4)
Dado o refator de resolver_colisoes
Quando rodar os testes existentes do lente_wiring
Então todos passam (calcular_raio_de_alvo inalterado)

Dado o workspace real (requer fork) — #[ignore]
Quando montar_grafo_workspace(raiz)
Então devolve o grafo unificado de todos os membros
E o número de nós é da ordem do medido na Arena (~363, laudo 0043)
E fantasmas está VAZIO (laudo 0041 — colisões são folhas de raio 0)
E há aresta cross-crate conhecida (ex.: lente_infra -> lente_core)
E as colisões por crate estão resolvidas (ex.: nenhum "Path::from" cru
colidindo — os nomes do 0042)

Dado montar_grafo_workspace chamado duas vezes (cache morno) — #[ignore]
Então a segunda é rápida (acertos de cache)

Dado o código todo
Então cargo tree -p lente_core só o crate (unir_grafos é pura)
```

Casos a cobrir: união (definição vence referência; fantasma; dedup de idênticos;
cadeia cross-crate; determinismo; reindexação); o refator (não-regressão do
`lente_wiring`); `montar_grafo_workspace` no workspace real (`#[ignore]`: contagem
de nós, fantasmas vazio, aresta cross-crate, colisões resolvidas, cache morno).
Mais a não-regressão da suíte.

---

## Resultado esperado

- `unir_grafos` (L1, pura), com `GrafoCrate`/`ResultadoUniao`/`Fantasma`.
- `resolver_colisoes` (L4, extraído, preserva comportamento); `calcular_raio_de_alvo`
  inalterado.
- `montar_grafo_workspace` (L4) — o grafo de workspace unificado + os fantasmas.
- **Pureza L1**: `cargo tree -p lente_core` só o crate.
- Testes: união (casos puros, sem fork) + orquestração (`#[ignore]` com fork).
- **Laudo** em `00_nucleo/lessons/0045-…`:
  - A semântica da união (definição-vence-referência; fantasma; reindexação).
  - **A contagem de fantasmas no workspace real** — esperado **0** (laudo 0041);
    se >0, é achado, registrar quais paths e quem os referencia (não esconder).
  - A contagem de nós do grafo unido vs a Arena (~363).
  - As arestas cross-crate confirmadas (uma conhecida).
  - As colisões por crate resolvidas no grafo final (os nomes do 0042).
  - A não-regressão do `calcular_raio_de_alvo` (o refator).
  - O ganho do cache morno na segunda chamada.
  - Contagem da suíte (era 235 verdes + 24 ignored no laudo 0044).

---

## Cuidados

- **Fantasma é sinal, não erro.** Esperado 0 neste repo (0041). Se aparecer, é
  porque um item foi renomeado/removido e ainda é referenciado, **ou** porque a
  resolução de um crate renomeou um item que outro crate referencia pelo nome
  antigo (o caso que o 0041 disse não morder aqui, por raio 0). A união deve
  **tratar** isso (fantasma + representante, 0 soltas), não quebrar. Registrar a
  contagem.
- **Reindexar por path.** Os `id` antigos são por-crate e colidem entre crates;
  atribuir ids novos e religar as arestas pelo `from`/`to` (path). O raio opera
  por path, então reindexar é seguro (laudo 0016).
- **Definição vence referência.** Para cada path, o nó cujo `source_crate ==
  no.crate_name` é a definição. Sem definição (só referências) → fantasma com
  representante.
- **Refator preserva comportamento.** `resolver_colisoes` é exatamente o que o
  `calcular_raio_de_alvo` já fazia; os testes dele são a guarda.
- **Determinismo.** Ordenar onde iterar (por path, por id); nada de ordem de
  `HashMap` na saída. O único resíduo de instabilidade é o `id` do petgraph entre
  extrações (briefing §7), pré-existente — a reindexação da união, aliás, dá ids
  estáveis **dentro** de uma união (sequenciais por path ordenado).
- **Pureza L1.** `unir_grafos` só stdlib; nada de I/O, nada de dep nova.
- **Primeira chamada fria no workspace real** (~33s, todos os crates no fork) — o
  E2E `#[ignore]` é lento na primeira; o cache aquece as seguintes. Esperado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Fecha o motor do grafo de workspace. L1: `unir_grafos` (pura) une os grafos resolvidos por crate **por path** — definição (nó cujo `source_crate == crate_name`) vence referência; path só com referências vira **fantasma** com nó-representante (0 arestas soltas, laudo 0039); reindexação com ids novos, arestas religadas pelo path; `GrafoCrate`/`ResultadoUniao`/`Fantasma`. L4: extrai `resolver_colisoes` do `calcular_raio_de_alvo` (refator que preserva comportamento) e adiciona `montar_grafo_workspace` (enumera 0044 → extrai cacheado 0044 → resolve por crate → une), devolvendo o grafo unificado + fantasmas; `ErroLente::Workspace`. Pureza L1 preservada (`unir_grafos` só stdlib). Fantasmas esperado 0 no repo (laudo 0041 — colisões de raio 0). Testes: união (casos puros) + orquestração (`#[ignore]` com fork: contagem ~363, fantasmas 0, aresta cross-crate, colisões resolvidas, cache morno). Não-regressão do `calcular_raio_de_alvo`. Suíte era 235+24. | `01_core/src/{união (novo),lib.rs}`, `04_wiring/src/lib.rs`, `00_nucleo/lessons/0045-...` |
