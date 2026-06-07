# Prompt: Protótipo do impacto de um diff — colisões na união (Arena)

**Tipo**: Experimento de Arena (`lab/`) — quarta rodada. Estende o protótipo
dos laudos 0038/0039/0040.
**Camada**: bancada (sem linhagem obrigatória).
**Criado em**: 2026-06-05
**Decisões de origem**: laudo 0039/0040 (a união por path **funde** colisões de
path intra-crate — 4 no `lente_core` — distorcendo o raio do path colidido, em
silêncio); decisão do autor nesta conversa: fechar o gap de **colisões** antes de
nuclear o produto.
**Pré-requisito**: fork atualizado em PATH; roda sobre o repositório da lente.
**Posição**: quarta rodada da trilha local na Arena. Fecha a correção do raio
**no miolo** (a união) antes de nuclear.

---

## Contexto

A união por path (laudos 0039/0040) trata "cada path único entra uma vez". Mas o
grafo é **cru** — sem resolução de colisões. Quando 2+ nós compartilham um path
dentro de um crate (4 casos no `lente_core`), a união os **funde** num só. Se um
path colidido for tocado por um diff, o raio dele fica **errado, em silêncio** —
o pior tipo de erro para uma ferramenta cujo valor é mostrar impacto honesto.

A boa notícia: a lente **já resolve colisões**. O `lente_investiga` classifica
cada colisão (`Veredito`: `Distintos`/`MesmoItem`/`NaoDeterminado`, com
`Evidencia`) e o `lente_resolve` aplica o veredito (renomeia paths como
`M::T::<Display>::fmt`, redistribui arestas por id). O wiring usa essa cascata
**por-grafo** (laudo 0019).

Esta rodada **aplica essa máquina ao caso multi-crate**: resolver cada crate
**antes** de unir, de modo que os paths intra-crate fiquem únicos e a união não
funda mais nada. O que a resolução **não** decidir (`NaoDeterminado`) é
**avisado** ("raio impreciso aqui"). Ainda é Arena — valida a correção antes de o
produto herdá-la.

---

## Restrições (regime de Arena)

- **Arena (`lab/`)**: regime relaxado. Estender o `lab/proto-impacto-diff`.
  Reaproveitar tudo dos 0039/0040 (cache, extração, união, mapeamento).
- **Não modificar nenhum crate do sistema (L1–L4).** Usar `lente_investiga` e
  `lente_resolve` como **bibliotecas** (replicar o laço de resolução na Arena,
  como o 0039 replicou a união — não tocar o L4).
- **Só leitura do repo.**

---

## O que esta rodada faz

### 1. Censo de colisões

Para cada crate, contar os paths com **2+ nós** (colisões). Reportar: total no
workspace, por crate, e — após investigar — o **tipo** de cada
(`Distintos`/`MesmoItem`/`NaoDeterminado`).

### 2. Resolver por crate, antes de unir

Replicar o laço de resolução em **cada grafo de crate** (como o wiring faz
por-grafo, laudo 0019), **antes** da união:

- Detectar colisões (paths com 2+ nós) no grafo do crate.
- `lente_investiga::investigar(...)` cada uma → `Veredito`. Usar **E1**
  (vizinhança); a **E2 (fontes) está em quarentena** (invariante) — registrar se
  alguma colisão precisaria dela.
- `lente_resolve::aplicar(...)` o veredito → paths intra-crate ficam **únicos**.
- **Depois**, unir os grafos resolvidos por path. Os paths intra-crate agora são
  únicos, e os cross-crate já eram (prefixados pelo nome do crate, laudo 0039) —
  então a união **não funde mais nada**.

### 3. Avisar o que não resolve

Colisões `NaoDeterminado` (a resolução não decide) ficam como o blob fundido,
**marcadas**: "raio impreciso aqui (colisão não resolvida)". É a parte "avisar"
da decisão.

### 4. Antes/depois num path colidido tocado

Os 4 paths colididos do `lente_core` não foram tocados pelo diff natural do 0037.
Para demonstrar a correção: identificar um path colidido, **simular tocá-lo**
(flag, à la `--simular-renomeacao` do 0040), e mostrar o **raio cru-fundido**
(errado) contra os **raios resolvidos-distintos** (corretos). Quantificar a
diferença.

### 5. Custo

Medir o tempo que a resolução adiciona. Hipótese: **desprezível** — é lógica L1
pura sobre as colisões, **sem fork novo** (o fork já rodou na extração/cache).

---

## As perguntas que a rodada deve responder

- **Censo**: quantas colisões no workspace, por crate, e de que tipo
  (`Distintos`/`MesmoItem`/`NaoDeterminado`)?
- **União limpa**: resolver-por-crate-antes-de-unir produz uma união com paths
  únicos — **zero fusão indevida**?
- **Cobertura**: quantas resolvem limpo (E1) e quantas ficam `NaoDeterminado`
  (precisam do aviso)? Alguma precisaria da E2 (em quarentena)?
- **Correção do raio**: o raio de um path colidido tocado fica certo após
  resolver, contra o cru-fundido? Qual a diferença?
- **Custo**: quanto a resolução adiciona ao caminho morno (deve ser desprezível)?
- **Órfãos cross-crate (confirmação)**: renomear paths colididos pode, em tese,
  orfanar uma aresta cross-crate que referenciava o path colidido antigo.
  **Predição: raro/nenhum** — paths colididos costumam ser métodos internos de
  impl que outros crates não referenciam pelo path colidido. Confirmar: a
  resolução cria **novos** órfãos/fantasmas (do 0040)? Se sim, distinguir o
  fantasma-de-resolução (renomeação interna) do fantasma-de-edição-real (impacto).
- **Veredito**: o produto deve resolver por crate antes de unir, e avisar o
  `NaoDeterminado`? Há motivo para não?

---

## Estrutura sugerida

- Estender `lab/proto-impacto-diff/`: o passo de resolução por crate (entre a
  extração/cache e a união), o censo, o aviso de `NaoDeterminado`, e o
  antes/depois simulado. A UI marca os nós com raio impreciso.
- `relatorio.md`: o censo, a cobertura E1, o antes/depois do raio, o custo, a
  confirmação de órfãos, o veredito.
- Laudo em `00_nucleo/lessons/0041-…`: registro, sumário e ponteiro (padrão Arena).

---

## Resultado esperado

- O protótipo resolve cada crate antes de unir, dá raio correto por nó distinto,
  e marca o `NaoDeterminado`.
- `relatorio.md` com: censo de colisões, cobertura E1 vs `NaoDeterminado`, raio
  antes/depois num path colidido, custo da resolução, confirmação de órfãos
  cross-crate, e o veredito para o produto.
- Laudo registro em `00_nucleo/lessons/0041-…`.
- **Zero toque em produção.**

---

## Cuidados

- **Resolver ANTES de unir.** A união cru já funde os colididos; resolver depois é
  tarde — a fusão destruiu os nós distintos. A ordem é extrair → resolver por
  crate → unir.
- **E2 em quarentena** (invariante): usar E1 (vizinhança). Registrar se alguma
  colisão do `lente` precisaria da E2.
- **Replicar o laço na Arena**, não tocar o L4. O wiring resolve por-grafo (laudo
  0019); aqui o protótipo faz o mesmo por crate, com `investiga`/`resolve` como
  bibliotecas. Reaproveitar a lógica do wiring é referência, não import do L4.
- **`id` instável entre extrações** (briefing §7): a resolução opera **dentro** de
  uma extração (ids consistentes ali); a redistribuição de arestas por id do
  `lente_resolve` (laudo 0010) é válida nesse escopo. A união continua por path.
- **Determinismo**: a nomeação da resolução é determinística (laudo 0010, ordem
  por id). Confirmar que a união resolvida é estável entre rodadas.
- **`NaoDeterminado` é raro**: typst + egui (29 crates) tiveram padrões conhecidos
  (laudo 0021). Registrar o que aparece no `lente`.
- **Fantasma de resolução vs de edição**: a renomeação da resolução pode disparar
  o detector de fantasmas do 0040 sem ser impacto real — distinguir.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Quarta rodada da Arena `lab/proto-impacto-diff/` — fecha o gap de colisões na união multi-crate: censo de colisões por crate; resolução por crate (via `lente_investiga`+`lente_resolve` como bibliotecas, E1; E2 em quarentena) **antes** de unir, de modo que a união por path não funda mais colididos; aviso de "raio impreciso" para `NaoDeterminado`; antes/depois do raio num path colidido simulado; custo da resolução (hipótese: desprezível, sem fork novo); confirmação de que a renomeação não orfana arestas cross-crate. Veredito para o produto: resolver por crate antes de unir + avisar o não-resolvido. Zero toque no produto. | `lab/proto-impacto-diff/{src/main.rs,index.html,dados/colisoes-*.json,relatorio.md}`, `00_nucleo/lessons/0041-proto-impacto-diff-colisoes.md` |
