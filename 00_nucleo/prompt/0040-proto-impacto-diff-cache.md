# Prompt: Protótipo do impacto de um diff — extração incremental + cache (Arena)

**Tipo**: Experimento de Arena (`lab/`) — terceira rodada. Estende o protótipo
dos laudos 0038/0039.
**Camada**: bancada (sem linhagem obrigatória).
**Criado em**: 2026-06-05
**Decisões de origem**: laudo 0039 (multi-crate funciona; impacto cross-crate é
grande; mas a extração do workspace inteiro custa ~33s, lento demais para o uso
reativo); decisão do autor nesta conversa: medir extração **incremental + cache**
antes de decidir a forma do produto.
**Pré-requisito**: fork atualizado em PATH; roda sobre o próprio repositório da
lente.
**Posição**: terceira rodada da trilha local na Arena. Mede o **gargalo de
custo** — o que decide se a trilha serve ao propósito reativo (mostrar o impacto
antes de um agente rodar um comando).

---

## Contexto

O laudo 0039 mostrou que o impacto cruza crates e é grande (`No` 11→44,
`Posicao` 15→48), e que isso exige **visão de workspace** — extrair **todos** os
crates, inclusive os não-tocados que dependem do item alterado. Mas extrair os 10
crates custou **~33s** (cold-start do rust-analyzer por crate). Isso é lento
demais para o uso reativo, que é o propósito da trilha local.

A hipótese desta rodada: **a maioria das mudanças toca um ou poucos crates.** Se
o grafo de cada crate for **cacheado** e só os crates **mudados** forem
re-extraídos, e a união for refeita sobre o cache, o caminho morno deve ser muito
mais rápido — perto do custo de uma extração só (~3s), não de dez.

Isto é Arena — descartável. Mede para decidir se o produto é viável no uso
reativo, e como montar a extração. Não é o produto.

---

## Restrições (regime de Arena)

- **Arena (`lab/`)**: regime relaxado. Estender o `lab/proto-impacto-diff`
  (a união por path do 0039 já existe). Reaproveitar tudo do 0039.
- **Não modificar nenhum crate do sistema (L1–L4).** Bug que aparecer, registrar.
- **Só leitura do repo.**

---

## O que esta rodada faz

### 1. Cache por crate

- Cachear, por crate, o **JSON cru do fork** (a saída do `export-json`), num
  diretório (ex.: `lab/proto-impacto-diff/cache/<crate>.json`) — análogo aos
  `checkpoints/<crate>.json` do laudo 0021. Cachear o JSON cru (não o `Grafo`)
  evita precisar serializar um tipo do `lente_core` (que é puro, sem serde).
- **Chave de invalidação: hash do conteúdo dos fontes do crate.** Hash dos
  arquivos `.rs` sob o `src/` do crate (o `manifest_path` do `cargo metadata` dá
  o diretório). A chave precisa refletir a **árvore de trabalho atual** (com
  edições não-comitadas) — **não** o commit-hash, porque o uso reativo tem
  edições não-comitadas.

### 2. Extração incremental

A cada execução:
1. Descobrir os crates do workspace (`cargo metadata --no-deps`, como no 0039).
2. Para cada crate: calcular o hash atual dos fontes. Se bate o do cache →
   **reusar** o JSON cacheado (desserializar, **pular o fork**). Se não bate →
   **rodar o fork** nesse crate e atualizar o cache (JSON + hash).
3. Refazer a **união por path** sobre todos os 10 (cacheados + recém-extraídos),
   como no 0039.
4. Mapear o diff → nós, calcular raio local + workspace, apresentar (reusar o
   0039).

### 3. Medir (o foco)

Cronometrar e reportar os cenários:

| Cenário | O que mede |
|---|---|
| Cold, primeira execução (cache vazio) | custo de popular o cache (= ~33s do 0039) |
| Morno, edição em **1 crate** | re-extrai 1 + reusa 9 + une |
| Morno, edição em **2–3 crates** | re-extrai 2–3 + une |
| Morno, **sem mudança** (cache quente) | só desserializar o cache + unir |

Para cada: tempo total, quantas extrações de fork rodaram, e o tempo gasto só em
**desserializar os JSONs cacheados + unir** (para saber se a parte sem-fork já é
rápida ou se ela própria vira gargalo com 10 JSONs).

### 4. Correção do cache sob renomeação (secundário, mas importante)

Um caso que o cache complica: **renomear** um item público.

- Renomear um item público em um crate A (ex.: `lente_core::…::No` → outro nome),
  re-extrair A (hash mudou). O crate B (fonte **inalterado** → cacheado) ainda
  tem uma aresta `B::f → A::No` **por path**. Na união, essa aresta fica
  **solta** (o path `A::No` não existe mais; A tem o nome novo).
- **Isso é, no fundo, o impacto que a trilha quer mostrar**: B usava `A::No`, que
  sumiu — B quebra. A pergunta: o protótipo trata essa aresta solta como **sinal
  de impacto** (B é afetado pela renomeação), ou ela some/vira ruído?
- Observar e registrar. (No 0039, com fonte limpa, deram 0 arestas soltas; aqui,
  a renomeação deve **produzir** algumas, e elas são significativas.)

---

## As perguntas que a rodada deve responder

Propósito da Arena: medir para decidir. Responder no relatório:

- **O caminho morno é rápido o bastante para o uso reativo?** Quanto custa uma
  edição de 1 crate (a hipótese: perto de uma extração, ~3–4s)? E de 2–3 crates?
  E o cache totalmente quente (sem mudança)?
- **A parte sem-fork** (desserializar 10 JSONs cacheados + unir) é sub-segundo, ou
  vira gargalo? Se virar, vale cachear a **união** já montada e só remendar o
  crate mudado? (Registrar como possível otimização futura, não obrigatória aqui.)
- **O custo cold (primeira execução, ~33s) é aceitável?** Ele é inevitável (popular
  o cache uma vez por sessão). Registrar o número.
- **A chave de cache** (hash de conteúdo dos fontes) é robusta? Pega edições
  não-comitadas? Há caso em que ela erra (não invalida quando devia, ou invalida
  demais)?
- **Renomeação**: a aresta solta do cache de B aparece como impacto, ou some?
  É o sinal certo (B afetado) ou um furo?
- **Veredito**: incremental + cache torna o uso reativo viável? Qual o custo
  residual?

O que é "rápido o bastante" é a sua chamada (decisão de significado); como
referência grosseira: um laço interativo se sente bom abaixo de ~1–2s, e alguns
segundos são toleráveis para uma checagem antes de uma ação. A rodada **mede**; a
aceitação é sua.

---

## Estrutura sugerida

- Estender `lab/proto-impacto-diff/`: camada de cache (`cache/<crate>.json` +
  hash), o laço incremental, e a cronometragem dos cenários. A união e o
  mapeamento do 0039 ficam como estão.
- `relatorio.md`: a tabela de tempos, o veredito de viabilidade reativa, o
  resultado da renomeação, e a robustez da chave de cache.
- Laudo em `00_nucleo/lessons/0040-…`: registro de que rodou, sumário e ponteiro
  (padrão Arena, laudo 0021).

---

## Resultado esperado

- O protótipo cacheia o grafo por crate, re-extrai só os crates mudados, e refaz
  a união sobre o cache — com a cronometragem dos cenários.
- `relatorio.md` com: os tempos (cold, morno-1, morno-2/3, cache-quente), o tempo
  da parte sem-fork, o veredito de viabilidade reativa, o comportamento sob
  renomeação, e a robustez da chave.
- Laudo registro em `00_nucleo/lessons/0040-…`.
- **Zero toque em produção.**

---

## Cuidados

- **Chave de cache = árvore de trabalho atual**, não commit-hash. O uso reativo
  tem edições não-comitadas; a chave deve invalidar quando o fonte do crate muda,
  comitado ou não. Hash de conteúdo dos `.rs` do `src/` do crate.
- **Re-extrair por hash, não só pelo diff.** O diff diz quais arquivos mudaram,
  mas a invalidação por hash é a fonte de verdade (pega qualquer mudança, mesmo
  fora do diff dado). O diff serve ao **mapeamento** (quais nós tocados), não à
  invalidação.
- **`id` instável entre extrações** (briefing §7): o cache guarda **JSON cru**
  (os ids são por-extração); a união reatribui ids e reancora por path, como no
  0039. Cachear JSON cru + refazer a união é consistente com isso.
- **Renomeação dangling**: uma edição que renomeia/remove um item de A deixa
  arestas do cache de B soltas — provavelmente o sinal certo de impacto, mas
  confirmar que o protótipo o trata, não o engole.
- **Cold first-run ~33s é inevitável**; o que esta rodada testa é o caminho
  morno. Se o cold for um problema de UX, é dado para a decisão de produto.
- **Sysroot ligado** (consistente com o 0039); o cache guarda o JSON com os nós
  de stdlib (que a relativização filtra do mapeamento).
- **Resolução de colisões ainda adiada**: a união por path funde colisões
  intra-crate (4 no `lente_core`); fora do escopo desta rodada; registrar se um
  path colidido for tocado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Terceira rodada da Arena `lab/proto-impacto-diff/` — extração incremental + cache: cacheia o JSON cru do fork por crate (chave = hash de conteúdo dos fontes, que pega edições não-comitadas), re-extrai só os crates mudados, refaz a união por path sobre o cache. Mede o caminho morno (edição de 1 e de 2–3 crates) contra o cold de ~33s do 0039, e o custo da parte sem-fork. Testa a correção sob renomeação (aresta solta do cache = impacto). Decide se o uso reativo é viável. Zero toque no produto. | `lab/proto-impacto-diff/{src/main.rs,cache/*.json,relatorio.md}`, `00_nucleo/lessons/0040-proto-impacto-diff-cache.md` |
