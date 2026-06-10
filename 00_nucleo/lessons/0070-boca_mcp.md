# Laudo de Execução — Prompt 0070 (boca MCP — a lente no laço do agente)

**Camada**: L4 (crate novo `04_wiring/mcp`, binário `lente-mcp`) + reuso da
montagem JSON do L2 (`lente_cli`). O pipeline (L1/L3/L4) **não muda**.
**Data**: 2026-06-10
**Prompt executado**: `00_nucleo/prompt/0070-boca_mcp.md`
**Estado**: `EXECUTADO` — boca MCP servindo `impacto_do_diff`/`raio_do_alvo`/
`ranking` por stdio; mesmo JSON da CLI; suíte 287 + 29 ignorados verdes; linter
V1=0/V2=0/V12=1; deps do protocolo só na boca. **Fase 0 pegou rot real (corrigida
à parte).**

---

## A resposta em uma sentença

A lente ganhou uma **boca MCP** (`lente-mcp`, L4): um servidor JSON-RPC/stdio à mão
que expõe os pipelines prontos com **o mesmo JSON** da CLI e descrições que
declaram o limite **estrutural-não-comportamental** — o Momento B da proposta, com
o pipeline e o contrato intocados.

---

## Fase 0 — sanidade dos 28 ignorados (o portão, e o que ele pegou)

**Primeira rodada completa dos `#[ignore]` desde a reestrutura 0050–0057.** O portão
**falhou** como desenhado: **21/28** verdes, **7 quebrados** por um **fóssil de
caminho do 0050** — testes apontando o `lente_core` no local pré-0050 (`01_core/`,
hoje `01_core/core/`):

| Binário | Antes | Causa |
|---|---|---|
| `e2e_lente_core` (3) | 0/3 | `.join("01_core")` → `01_core/01_core` (dobrado) |
| `lente_infra` (4 de 12) | 8/12 | `.join("01_core")`/`raiz.join("01_core")` → `01_core/Cargo.toml` inexistente |

**Decisão do autor** (consultado, pois o prompt exige "parar e relatar — conserto é
decisão à parte"): consertar agora, em mudança separada. Repontados 5 `.join`
(`lib.rs`, `traducao.rs`, `workspace.rs`×2 → `01_core/core`; `e2e_lente_core.rs` →
`core`). Restou **1** falha de outra natureza: `filtra_lente_core_remove_sysroot…`
afirmava banda de contagem ancorada no laudo 0025 (108/91) sobre o crate
**monolítico** pré-0050; o `lente_core` ~dobrou (medido: **200 antes / 178 depois**).
Banda **re-ancorada** (±15%) com registro no próprio teste. **Portão: 28/28.**

O pipeline em si estava **são** o tempo todo (os 13 testes que invocam o fork em
crates reais — `lente` app 3 + `wiring` 10 — sempre passaram); o que apodreceu foi
caminho hardcoded em fixture. Esta correção é **commit à parte** da boca.

---

## Fase 1 — decisões verificadas (não sobre suposição)

### Protocolo: JSON-RPC à mão, **não** SDK

O SDK oficial Rust é o **`rmcp`** (modelcontextprotocol/rust-sdk; v1.x atual, tokio
+ async + macros). O mínimo de um servidor stdio de ferramentas (spec **2025-06-18**)
é pequeno e estável: `initialize` (negocia versão), `notifications/initialized`,
`tools/list`, `tools/call`, `ping` — JSON-RPC 2.0, **uma mensagem por linha**.
**Escolha: JSON-RPC à mão** sobre `serde_json` (já do workspace). Razão (critério do
prompt — menor superfície que passa um cliente real):

- a superfície mínima é ~5 tipos de mensagem, **síncrona**; cada chamada roda o fork
  (bloqueante) — um runtime **async não compraria nada**;
- evita arrastar **tokio** ao workspace (política "deps externas só na boca");
- ~200 linhas auditáveis, sob nosso controle — e a spec é precisa o bastante para
  acertar o handshake (provado pelo E2E e pelo smoke).

### Reuso do JSON: depender do `lente_cli`, **não** copiar

A montagem JSON vive em `02_shell/cli/src/saida.rs`, já **pública**: `formatar`
(Raio), `formatar_diff` (ResultadoDiff, contrato 0047), `formatar_ranking`. A boca
**depende de `lente_cli` (L2)** e chama essas `pub fn` com `Modo{text:false}` —
**zero duplicação**. (L4→L2 é legal; o topo importa abaixo.)

### Erro: `ErroLente: Display`, não o `traduzir` do app

`traduzir` (catálogo) vive no `lente_app`, que é **bin-only** — importá-lo exigiria
dar um alvo lib + linhagem ao app, contra a regra "aditive / não mudar". Usei o
`ErroLente: Display` (o mecanismo de mensagem do próprio tipo: "fork: …", "id N não
existe…", "leitura do diff: …") → resultado com `isError: true`. **Desvio
documentado** do literal "mensagem do catálogo", em favor da boca estritamente
aditiva.

### Latência medida (para a decisão futura de cache — não resolvida aqui)

`lente --diff` na própria lente: **frio 35,96s** (cache limpo, fork por crate, 45
entradas) · **quente 0,07s** (cache de workspace já existe). O frio é a dor do laço
de agente; o cache torna o quente instantâneo. **Cache é prompt futuro** — aqui só o
número.

---

## Fase 2 — a boca

**Crate `04_wiring/mcp`** (`lente-mcp`), `@layer L4`, nascido na convenção
Cristalina (cabeçalho de linhagem, prompt `00_nucleo/prompts/mcp.md`, snapshot
gerado vazio — `main`/funções não são `pub`).

| Ferramenta | Pipeline | Saída |
|---|---|---|
| `impacto_do_diff` (`raiz?`) | `analisar_diff` (0047) | JSON do `ResultadoDiff` |
| `raio_do_alvo` (`pacote\|grafo`, `alvo\|alvo_id`, `escopo?`) | `calcular_raio_de_alvo` | JSON do `Raio` |
| `ranking` (`pacote\|grafo`, `top?`, `escopo?`) | `rankear_pacote` | JSON do ranking |

- **Descrições = interface honesta**: cada uma declara **ESTRUTURAL** (quem depende,
  via `Uses`) e **NÃO** comportamental / não "vai quebrar" — a proposta §3 no
  contrato, não só na doc (testado: `tools_list` exige "ESTRUTURAL"+"NÃO" em toda
  descrição).
- **Erros**: `ErroLente` e validação (fonte/alvo ausente ou ambíguo) → `isError:
  true` com a mensagem — **não panica, não silencia**. JSON inválido → `-32700`;
  método desconhecido → `-32601`.
- **stdout sagrado** (só protocolo); **sem estado** (cada chamada do zero).

**Testes**: 10 de unidade inline (envelope JSON-RPC + validação, **sem fork**) + 1
E2E `#[ignore]` por stdio (`tests/e2e_stdio.rs`, excluído da linhagem como harness)
que sobe o binário real e roda `initialize → initialized → tools/call
impacto_do_diff`, conferindo o JSON do 0047.

---

## Fase 3 — smoke real (registrado)

**Smoke de protocolo, binário real** (`target/release/lente-mcp`):

```
initialize → serverInfo {lente-mcp 0.1.0}, protocolVersion 2025-06-18 (ecoada)
tools/list → impacto_do_diff · raio_do_alvo · ranking (descrições com "ESTRUTURAL")
tools/call impacto_do_diff (E2E, repo da lente) → isError:false; conteúdo é o JSON
   do diff com as chaves do contrato 0047: combinado, tocados, ligados, soltos,
   nao_fonte, fantasmas  (~36s frio — a latência da Fase 1, sentida de ponta a ponta)
```

Isto **é** o ciclo do Momento B funcionando ponta a ponta pela boca real. O cliente
`claude` está disponível (`~/.local/bin/claude`); registrar para exercício
interativo é um comando:

```
claude mcp add lente-mcp -- <repo>/target/release/lente-mcp
```

(O exercício dentro do Claude Code, sessão viva, fica para o usuário — é o primeiro
consumidor real e o dado que informa a trilha da visualização.)

---

## Verificação

| Item | Resultado |
|------|-----------|
| Fase 0 (28 ignorados) | **28/28** após conserto do fóssil (commit à parte) |
| `cargo build --workspace` | passa |
| Suíte normal | **287 passed / 0 failed** (277 + 10 mcp) |
| Suíte `#[ignore]` | **29 passed / 0 failed** (28 + 1 mcp e2e) |
| `crystalline-lint .` | **V1=0, V2=0**, V5/V6=0; **V12=1** (`ErroLente`, intencional) — preservado |
| `cargo tree -p lente_mcp` | só `serde_json` — **sem tokio/async**; deps do protocolo só na boca |
| `cargo tree -p lente_core` | puro (só o crate) — L1 intacto |
| Pipelines/tipos L1/fork/CLI | **intocados** (boca aditiva) |

---

## Sinalização para a trilha da visualização

O uso real (E2E + smoke) mostra: a saída é o **JSON do 0047/0030** — denso (o diff
da lente traz `combinado.jusante` com dezenas de paths de sysroot no escopo
`completo`). Para o laço do agente, o que falta projetar não é mais dado, é
**recorte**: o agente quer "quantos/quais tocados e o tamanho do raio", não a lista
crua de stdlib. A próxima trilha (visualização) tem aqui o primeiro sinal: **o
escopo `seu-codigo` e um resumo (censo + top-N tocados) provavelmente importam mais
que o dump completo** — a decidir com mais uso, não argumentando.

Itens nomeados, não resolvidos: **cache** (latência fria ~36s medida); **registro
no Claude Code** para exercício interativo; eventual 4ª ferramenta se o uso pedir.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Boca MCP da lente (Momento B, proposta §4): crate novo `04_wiring/mcp` (binário `lente-mcp`, L4) servindo `impacto_do_diff` (→`analisar_diff`, JSON do `ResultadoDiff` 0047), `raio_do_alvo` (→`calcular_raio_de_alvo`, com `escopo` 0030) e `ranking` (→`rankear_pacote`) por **JSON-RPC 2.0 à mão sobre stdio** (decisão Fase 1: menor superfície, síncrona como os pipelines, sem SDK/tokio — `serde_json` já no workspace). Reusa a montagem JSON **pública** do `lente_cli` (`formatar`/`formatar_diff`/`formatar_ranking`) — zero duplicação; erro via `ErroLente: Display`→`isError:true` (o `traduzir` do app é bin-only; desvio documentado p/ manter a boca aditiva). Descrições declaram o limite **estrutural-não-comportamental** (interface, proposta §3). 10 testes de unidade (envelope+validação, sem fork) + 1 E2E `#[ignore]` por stdio (`tests/e2e_stdio.rs`, excluído como harness) que sobe o binário e valida o ciclo `initialize→initialized→tools/call` contra o contrato 0047. **Fase 0** (1ª rodada completa dos 28 ignorados desde a reestrutura 0050): portão pegou **7 quebrados por fóssil de caminho do 0050** (`lente_core` apontado em `01_core/` em vez de `01_core/core/`) — corrigidos à parte (5 `.join` repontados + 1 banda de contagem re-ancorada 108/91→200/178, o crate ~dobrou). **Fase 1**: latência medida frio **35,96s** / quente **0,07s** (cache de workspace já existe; cache novo é prompt futuro). **Fase 3**: smoke do binário real (initialize ecoa 2025-06-18; tools/list com as 3; tools/call diff devolve o JSON do 0047); `claude` disponível, registro é `claude mcp add`. Aditivo: pipelines/tipos L1/fork/CLI intocados; crate nasce na convenção (V1=0, V2=0; V12=1 inalterado). Suíte 287 + 29 ignorados verde; `lente_mcp` só `serde_json` (sem tokio); `lente_core` puro. | `04_wiring/mcp/{Cargo.toml,src/main.rs,tests/e2e_stdio.rs}` (novos), `Cargo.toml` raiz (member), `00_nucleo/prompts/mcp.md` (novo), `crystalline.toml` (`mcp_e2e_test` em `[excluded_files]`), `00_nucleo/lessons/0070-boca_mcp.md`. **À parte (fóssil 0050, commit separado)**: `03_infra/src/{lib,traducao,workspace}.rs`, `01_core/filtro/tests/e2e_lente_core.rs` |
