# Prompt: Protótipo do impacto de um diff — multi-crate (Arena)

**Tipo**: Experimento de Arena (`lab/`) — segunda rodada. Estende o protótipo
do laudo 0038.
**Camada**: bancada (sem linhagem obrigatória).
**Criado em**: 2026-06-05
**Decisões de origem**: laudo 0038 (validou a vista em camadas e a comparação de
input sobre **um** crate; vão registrado = multi-crate); decisão do autor nesta
conversa: mais uma rodada de Arena cobrindo **multi-crate** e **macro**.
**Pré-requisito**: fork atualizado (5ª rodada, emite `position`) instalado em
PATH; roda sobre o próprio repositório da lente (workspace com vários crates).
**Posição**: segunda rodada da trilha local na Arena. Mede o que falta antes de
decidir a forma do produto (CLI ou visual).

---

## Contexto

O laudo 0038 validou, sobre **um** crate (`lente_core`): a vista em camadas
(aprofundar mostra detalhe útil, passa o teste dos dez segundos) e a comparação
de input (stdin e `git diff HEAD` iguais para arquivos rastreados; untracked
cego nos dois). Mas, limitado a um crate, ele **mapeou só as mudanças de
`grafo.rs` e não viu as de `03_infra`** (`dto.rs`, `traducao.rs`) do **mesmo**
diff do 0037.

Um diff real toca **vários crates**. E — o ponto central desta rodada — o
**impacto cruza crates**: mudar um item público de `lente_core` (ex.: o `No`)
afeta quem o usa em `lente_infra`, `lente_wiring`, `lente_cli`. O protótipo de
um crate, calculando o raio só no grafo de `lente_core`, **não vê** esses
dependentes em outros crates. Para a lente responder "o que esta mudança toca"
com honestidade num workspace, o impacto precisa atravessar a fronteira de
crate.

Esta rodada estende o protótipo a multi-crate e **mede a questão do impacto
cross-crate**. Ainda é Arena — descartável. Mede para decidir a forma do
produto; não é o produto.

---

## Restrições (regime de Arena)

- **Arena (`lab/`)**: regime relaxado. Estender o `lab/proto-impacto-diff`
  existente (ou um sibling — escolha do gerador); reaproveitar o pipeline do
  0038 (parser de diff, relativização, casamento diff↔`position`,
  `calcular_raio`).
- **Não modificar nenhum crate do sistema (L1–L4).** Bug que aparecer,
  registrar.
- **Só leitura do repo.** Roda o fork e lê o `git diff`; não escreve no repo.

---

## O que esta rodada faz

### 1. Mapeamento multi-crate

- Dado um `git diff` (do repo inteiro), **mapear cada arquivo alterado ao crate
  que o contém** (pelo diretório / pelos membros do workspace; ex.:
  `01_core/src/…` → `lente_core`, `03_infra/src/…` → `lente_infra`).
- **Extrair o grafo de cada crate tocado** (via `lente_infra::extrair_grafo` por
  pacote) e mapear as mudanças de cada crate aos seus nós (mesma lógica do 0038,
  por crate).
- **Relativizar contra a raiz do repo** (`git rev-parse --show-toplevel`), uma
  raiz para todo o workspace — não a raiz de cada crate. (O `git diff` é relativo
  à raiz do repo; o `position.file` é absoluto.)
- Apresentar por crate, e por camada dentro de cada (reusar a vista do 0038).

### 2. Impacto cross-crate (a pergunta central)

Primeiro **medir o vão**, depois **testar como fechá-lo**:

- **Medir o vão**: para um item público tocado (ex.: mude `lente_core::…::No`),
  calcule o montante **só no grafo de `lente_core`** e observe se os dependentes
  em `lente_infra`/`lente_wiring`/`lente_cli` **aparecem ou não**. Predição: não
  aparecem (o grafo de um crate não contém quem está acima dele). Confirmar e
  quantificar.

- **Testar como obter um grafo que abrange o workspace.** Duas abordagens, medir
  qual funciona e a que custo:

  - **(A) Extrair o crate do topo.** `lente_cli` depende transitivamente de tudo
    no workspace. Extrair `lente_cli` e ver se o grafo **inclui os itens dos
    outros crates** (nós de `lente_core`, `lente_infra`, …) **e as arestas
    cross-crate** (alguém em `lente_cli` usando `lente_core::No`). Se incluir, o
    montante cruza crates **a partir de uma extração só** — o caminho mais
    simples. Verificar: `lente_cli` traz quantos nós? Aparecem itens de
    `lente_core`? Há aresta de `lente_cli`→`lente_core`?

  - **(B) Unir as extrações por crate.** Extrair cada crate do workspace e
    **unir** os grafos. **Cuidado com o `id`**: ele é índice do petgraph,
    **instável entre extrações** (briefing §7) — uma aresta cross-crate com
    `id_to = 42` no grafo de B **não** corresponde ao nó de `id 42` no grafo de
    A. Casar arestas cross-crate **por `path`**, não por `id` (com risco de
    colisão de path). Testar: as arestas cross-crate de fato **conectam** por
    path entre extrações? O `to` de uma aresta de `lente_infra` para um item de
    `lente_core` usa o path canônico de `lente_core` (que casa com o nó na
    extração de `lente_core`)? Quantas arestas cross-crate ficam **soltas** (sem
    nó-alvo) após a união?

- **Registrar qual abordagem funciona, e o custo** (tempo de extrair vários
  crates; complexidade e furos da união). É a pergunta técnica que decide se o
  produto precisa de um grafo de workspace, e como montá-lo.

### 3. Macro call-site (o caso que o autor pediu)

- Exercitar uma edição que toque um item **gerado por macro** (a `position` é o
  **call-site**, briefing §5) e/ou a **definição de uma macro**.
- O nó tocado é o esperado? Há surpresa — por exemplo, uma edição na definição
  da macro **não** casa com os itens que ela gera (porque a `position` deles é o
  call-site, não a definição)? Registrar o que acontece.

### 4. Colisões (opcional, secundário)

- Se algum path colidido for tocado pelo diff, observar o raio (pode estar
  impreciso sem resolução — laudo 0016). Não é o foco; registrar se aparecer.

---

## As perguntas que a rodada deve responder

Propósito da Arena: medir para decidir. Responder no relatório, sobre diffs
reais que você fizer no repo:

- **Mapeamento multi-crate**: as mudanças de cada crate tocado casam aos nós
  certos? Repetir o diff do 0037 (que tocou `01_core` e `03_infra`) e confirmar
  que agora **as mudanças de `03_infra` aparecem** (o que o 0038 perdeu).
- **Impacto cross-crate**: o montante cruza crates? Por qual abordagem (A ou B)?
  Custo de cada uma? As arestas cross-crate conectam por path? Quantas ficam
  soltas?
- **Sysroot e `position`** (retomando a dúvida do laudo 0038): a extração usa o
  `--sysroot` padrão da lente? Quantos nós **sem** `position` aparecem (devem ser
  os de stdlib)? Se 100% dos nós têm `position`, o sysroot pode estar desligado —
  e aí o raio subconta arestas de derive (Limite 1). Confirmar e registrar.
- **Macro**: o nó tocado é o certo? Surpresas?
- **Camadas em escala**: a vista em camadas ainda lê bem com vários crates e mais
  nós (teste dos dez segundos), ou vira ruído? O que precisaria recolher/filtrar?

---

## Estrutura sugerida

- Estender `lab/proto-impacto-diff/` (ou `lab/proto-impacto-diff-multi/`):
  `main.rs` ganha o mapeamento arquivo→crate, a extração por crate tocado, e a
  montagem do grafo de workspace (A e/ou B). A UI em camadas ganha um nível por
  crate (crate → arquivo → nó → montante).
- `relatorio.md`: as respostas às perguntas acima, sobre diffs reais. Descrever
  o que viu (e a comparação A versus B).
- Laudo em `00_nucleo/lessons/0039-…`: registro de que rodou, sumário e ponteiro
  (padrão Arena, laudo 0021).

---

## Resultado esperado

- O protótipo mapeia um diff multi-crate aos nós tocados de cada crate, e mostra
  o impacto **atravessando crates** (pela abordagem que a medição mostrar viável).
- `relatorio.md` com: confirmação de que o diff do 0037 agora pega `03_infra`; a
  medição do vão cross-crate e qual abordagem (A/B) o fecha e a que custo; o
  resultado do caso de macro; a confirmação do sysroot e da contagem de nós sem
  `position`; e como as camadas leem em escala.
- Laudo registro em `00_nucleo/lessons/0039-…`.
- **Zero toque em produção.** Se precisar de função nova no L4 (ex.: expor o
  grafo resolvido), registrar como decisão e dívida; preferir replicar na Arena.

---

## Cuidados

- **`id` instável entre extrações** (briefing §7): para a união (abordagem B),
  **não** casar arestas cross-crate por `id`; casar por `path`. Dentro de **uma**
  extração (abordagem A), o `id` é consistente.
- **Relativizar contra a raiz do repo**, não a de cada crate — uma raiz só para
  todo o workspace.
- **Sysroot**: confirmar que está ligado (política da lente, ADR-0001). Medir
  quantos nós vêm sem `position` (stdlib). Isso fecha a dúvida do laudo 0038
  (119/119 com `position`).
- **Macro call-site**: a `position` de item gerado por macro é o call-site; uma
  edição na definição da macro pode não casar com os itens gerados.
- **Honestidade estrutural** (briefing §7, Limite 3): o impacto é estrutural
  (`Uses`), não comportamental. A tela diz isso.
- **Resolução de colisões adiada**: grafo cru; paths colididos podem dar raio
  impreciso. Registrar colisões; resolver é outra rodada.
- **Custo de tempo**: extrair vários crates roda o fork várias vezes (o
  cold-start do rust-analyzer foi de até ~2 min por crate no laudo 0021).
  Registrar o tempo; se for proibitivo, é dado para a decisão de produto.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Segunda rodada de Arena do protótipo de impacto de diff: estende a multi-crate (mapeia arquivo→crate, extrai cada crate tocado, relativiza contra a raiz do repo) e mede a questão central — o impacto cruzando crates (abordagem A: extrair o crate do topo; abordagem B: unir extrações por path, com o `id` instável entre extrações). Exercita macro call-site. Confirma sysroot e contagem de nós sem `position` (dúvida do laudo 0038). Descartável; mede antes de decidir a forma do produto. | `lab/proto-impacto-diff*/{Cargo.toml,src/main.rs,index.html,dados/*.json,relatorio.md}`, `00_nucleo/lessons/0039-proto-impacto-diff-multicrate.md` |
