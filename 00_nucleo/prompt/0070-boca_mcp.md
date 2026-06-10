# Prompt: boca MCP — a lente no laço do agente (Momento B da proposta)

**Camada**: L4 (ponto de entrada novo, binário `lente-mcp`) + possivelmente L2
(expor/fatorar a serialização JSON existente). O pipeline (L1/L3/L4 de cálculo)
**não muda**.
**Criado em**: 2026-06-09
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0070-boca_mcp.md`)
**Decisões de origem**:
- Proposta §4, Momento B — "a IA propõe uma mudança; a lente mostra o raio
  daquela mudança, para o humano aprovar ou rejeitar sabendo a consequência".
  É a divisão de trabalho que motiva o projeto, e nenhum consumidor real a
  exercita ainda.
- Laudo 0038 — registrou explicitamente "não há MCP" como fronteira do que
  existia; a trilha local fechou no 0048 sem consumidor agente.
- Laudo 0047 — o JSON do `ResultadoDiff` é o contrato pronto que esta boca
  serve.
- Decisão do autor (2026-06-09): MCP antes da visualização — o uso real no
  laço do agente gera o dado que decide a projeção visual (proposta §5:
  "testando, não argumentando, sobre os dados reais").
**Pré-requisito**: estado pós-0068 (convenção fechada, 277 verdes + 28
ignored); modo `--diff` completo (0046–0048); `--alvo`/`--alvo-id`/`--ranking`/
`--estrutura` existentes.
**Arquivos afetados (a confirmar na Fase 1)**: crate novo `04_wiring/mcp`
(binário `lente-mcp`); `Cargo.toml` raiz (member); possivelmente
`02_shell/cli` ou um crate L2 novo, se a serialização JSON precisar ser
exposta/fatorada; testes.

---

## Contexto

A lente computa o impacto de um diff (`analisar_diff` → `ResultadoDiff`,
0047) e o raio de um alvo (`calcular_raio_de_alvo`), e emite JSON. Falta a
boca pela qual um agente (Claude Code ou outro cliente MCP) pergunta antes
de mudar e o humano vê antes de aprovar. MCP (Model Context Protocol) é o
padrão atual para isso: um servidor por stdio que anuncia ferramentas
(`tools/list`) e as executa (`tools/call`).

Este prompt entrega o servidor **mínimo e honesto**: poucas ferramentas,
respostas no JSON que já existe, e descrições que declaram a fronteira da
lente (impacto **estrutural**, não comportamental — proposta §3). Não é a
visualização; não muda o cálculo; é uma boca nova sobre pipelines prontos.

---

## Restrições estruturais

- **Ponto de entrada é L4** (precedente do 0057: o binário mora na
  composição). O crate novo `04_wiring/mcp` depende de `lente_wiring` (os
  pipelines) e do que for preciso para serializar (ver Fase 1). **Gravidade
  preservada**: nada de L1/L2/L3 passa a depender do crate novo.
- **O pipeline não muda.** `analisar_diff`, `calcular_raio_de_alvo`,
  `rankear_pacote`, `obter_grafo` ficam como estão. A boca só chama.
- **A serialização JSON é a que existe.** O JSON do `ResultadoDiff` e do
  `Raio` que a CLI emite é o contrato; a boca MCP devolve **o mesmo JSON**
  (dois consumidores, uma forma). Se hoje essa montagem é privada do
  `lente_cli`, a Fase 1 decide entre expô-la ou fatorá-la para um lugar
  comum — **sem duplicar** a montagem.
- **Convenção Cristalina desde o nascimento**: cabeçalho de linhagem,
  prompt de nucleação, snapshot — o crate novo nasce dentro da convenção
  que o 0068 fechou (V1 = 0, V2 = 0 permanecem).
- **Deps externas só na boca.** O que o protocolo exigir (SDK ou JSON-RPC à
  mão) entra no crate novo, não desce.

---

## Fase 0 — Sanidade dos ignorados (pré-condição, barata)

Antes de construir sobre os pipelines, confirmar que eles funcionam no HEAD:

```
cargo test --workspace -- --ignored
```

- Esperado: os **28** (composição do 0068: app 3 + e2e_lente_core 3 +
  infra 12 + wiring 10) **verdes**, com fork instalado em PATH.
- Registrar no laudo o resultado por binário. Se algum falhar, **parar e
  relatar** — consertar E2E quebrado é decisão à parte, não silenciosa.
- Esta é a primeira rodada completa registrada desde a reestrutura
  0050–0057; o registro vale por si.

---

## Fase 1 — Leitura e verificação (obrigatória; não desenhar sobre suposição)

1. **O protocolo real.** Verificar a especificação MCP vigente e o estado do
   SDK Rust oficial (`rmcp` / modelcontextprotocol rust-sdk): versão, o
   mínimo que um servidor de ferramentas por **stdio** precisa implementar
   (handshake `initialize`, `tools/list`, `tools/call`, formato de erro).
   Decidir **SDK vs JSON-RPC à mão** pelo critério: menor superfície que
   passa num cliente real. Registrar a escolha, a versão pinada e a razão.
2. **Onde mora o JSON hoje.** Ler `02_shell/cli` (a montagem JSON do
   `ResultadoDiff`, do `Raio`, do ranking): é função pública? `pub(crate)`?
   Decidir o caminho de reuso (expor / fatorar para crate L2 comum /
   depender de `lente_cli` como lib) e registrar. **Proibido**: copiar a
   montagem.
3. **Latência real.** Medir `analisar_diff` no repo da lente (frio e
   quente) e uma extração de pacote. O laudo 0021 registrou cold-start de
   rust-analyzer de ~123s no primeiro crate — o laço de agente sente isso.
   **Não construir cache agora**; medir e registrar, para a decisão futura
   ter número.
4. **Cliente real disponível.** Verificar como registrar um servidor MCP
   stdio no Claude Code (ou cliente equivalente disponível na máquina) para
   o smoke da Fase 3.

---

## Fase 2 — Construção

### As ferramentas (mínimo obrigatório: as duas primeiras)

| Ferramenta | Entrada | Saída | Pipeline |
|---|---|---|---|
| `impacto_do_diff` | `raiz` (opcional; default cwd) | JSON do `ResultadoDiff` (tocados com raio, combinado, censo, fantasmas) | `analisar_diff` (0047) |
| `raio_do_alvo` | `alvo` (path) ou `alvo_id`; `fonte` (pacote \| caminho de grafo.json); `escopo` (completo \| seu-codigo, default completo — 0030) | JSON do `Raio` | `calcular_raio_de_alvo` |
| `ranking` (opcional, se barato) | `pacote`, `top` (default 10), `escopo` | JSON do ranking (0027/0030) | `rankear_pacote` |

- **Descrições honestas** (o agente lê a descrição antes de chamar): cada
  ferramenta declara, no texto, que o impacto é **estrutural** (quem depende,
  via `Uses`) e **não** responde "vai realmente quebrar" nem vê o raio
  comportamental — a honestidade da proposta §3 vai no contrato da
  ferramenta, não só na doc.
- **Erros**: falha de pipeline (`ErroLente`) vira erro MCP com a mensagem
  do catálogo (a tradução já existe no padrão do `app`/0057 — reusar o
  mecanismo, não as cópias).
- **Sem estado**: cada chamada roda o pipeline do zero (a latência medida na
  Fase 1 fica registrada; otimizar é prompt futuro, se o uso pedir).

### O binário

- `lente-mcp` em `04_wiring/mcp`: lê/escreve MCP por stdio, despacha para os
  pipelines, devolve o JSON. Logs (se houver) em **stderr** — stdout é do
  protocolo.

### Testes

- **Unidade (sem subprocesso)**: o despacho ferramenta→pipeline com
  entradas forjadas onde possível; o mapeamento de `ErroLente` → erro MCP;
  validação de argumentos (alvo ausente, fonte inválida).
- **E2E `#[ignore]`** (convenção 0017/0037): subir o binário real, fazer
  `initialize` → `tools/list` → `tools/call impacto_do_diff` por stdio
  contra o próprio repo, e conferir que a resposta carrega o JSON do
  `ResultadoDiff`. Exige fork + git — `#[ignore]`, como os demais.

---

## Fase 3 — Smoke com cliente real (manual, registrado)

Registrar o servidor no cliente MCP disponível (Fase 1, item 4) e exercitar
o ciclo do Momento B uma vez, de verdade: fazer uma mudança pequena no repo
da lente, pedir ao agente que consulte `impacto_do_diff` antes de propor, e
**registrar no laudo** o que a resposta mostrou e quanto demorou. Não é
benchmark; é a primeira evidência de uso real — o dado que a decisão da
visualização vai consumir.

---

## O que NÃO fazer

- **Não construir a visualização** — próxima trilha, informada por este uso.
- **Não construir cache/daemon/índice persistente** — medir primeiro (Fase 1,
  item 3); otimização é prompt próprio se o uso doer.
- **Não duplicar a montagem JSON** — reusar/fatorar, nunca copiar.
- **Não mudar pipelines, tipos L1, fork, CLI existente** — a boca é aditiva.
- **Não inventar ferramentas além das listadas** — superfície mínima; o uso
  real dirá o que falta.

---

## Critérios de Verificação

```
Dado cargo test --workspace -- --ignored no HEAD (Fase 0)
Então os 28 rodam; resultado registrado por binário; falha interrompe o prompt

Dado o servidor iniciado e um initialize + tools/list por stdio
Então as ferramentas aparecem com descrições que declaram o limite estrutural

Dado tools/call impacto_do_diff com o repo da lente contendo uma mudança
Então a resposta carrega o MESMO JSON do `lente --diff` (mesmo contrato, 0047)

Dado tools/call raio_do_alvo com alvo válido (e com escopo seu-codigo)
Então a resposta carrega o JSON do Raio; o escopo respeitado e declarado (0030)

Dado um alvo inexistente ou fonte inválida
Então erro MCP com a mensagem do catálogo — não panic, não silêncio

Dado a suíte e o linter
Então suíte verde (277 + novos; ignorados +1 ou +2 pelos E2E novos);
crystalline-lint: V1 = 0, V2 = 0 preservados (o crate novo nasce na convenção);
V12 = 1 (ErroLente, intencional) inalterado

Dado cargo tree
Então as deps do protocolo só no crate novo; lente_core continua só o crate
```

---

## Resultado esperado

- Binário `lente-mcp` servindo `impacto_do_diff` e `raio_do_alvo` (e
  `ranking`, se entrou) por MCP/stdio, com o JSON existente como contrato e
  descrições que declaram a fronteira estrutural.
- Fase 0 registrada: a primeira rodada completa dos 28 ignorados desde a
  reestrutura.
- Latências medidas (frio/quente) registradas para a decisão futura de cache.
- Smoke real do Momento B registrado: a primeira vez que a lente é consultada
  num laço de mudança de verdade.
- **Laudo** em `00_nucleo/lessons/0070-…`: a escolha SDK vs à-mão (com
  versão), o caminho de reuso do JSON, as medições, o resultado da Fase 0,
  o smoke da Fase 3, e a sinalização para a trilha da visualização (o que o
  uso real mostrou sobre qual projeção falta).

---

## Cuidados

- **stdout é sagrado** — só protocolo; qualquer log em stderr.
- **A Fase 0 vem antes de tudo** — não construir boca sobre pipeline não
  verificado no HEAD.
- **As descrições das ferramentas são interface** — o agente decide chamar
  pelo texto; a honestidade estrutural-não-comportamental precisa estar lá.
- **Latência registrada, não resolvida** — cache sem medição seria estruturar
  antes do uso pedir.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-09 | Boca MCP da lente (Momento B da proposta §4): binário `lente-mcp` em `04_wiring/mcp` (ponto de entrada L4, precedente 0057) servindo `impacto_do_diff` (→ `analisar_diff`, JSON do `ResultadoDiff` de 0047) e `raio_do_alvo` (→ `calcular_raio_de_alvo`, com `escopo` de 0030) por MCP/stdio; `ranking` opcional. Fase 0: primeira rodada completa registrada dos 28 `#[ignore]` desde a reestrutura 0050–0057 (composição do 0068). Fase 1 verifica protocolo/SDK real (rmcp vs JSON-RPC à mão), o caminho de reuso da montagem JSON do `lente_cli` (proibido duplicar) e mede latência fria/quente (cache fica para prompt futuro, com número). Descrições das ferramentas declaram o limite estrutural-não-comportamental (proposta §3). E2E `#[ignore]` por stdio (initialize → tools/list → tools/call); smoke manual com cliente real registrado (Fase 3) — o dado que informará a trilha da visualização. Aditivo: pipelines, tipos L1, fork e CLI intocados; crate novo nasce na convenção Cristalina (V1 = 0, V2 = 0 preservados). | `04_wiring/mcp/` (novo), `Cargo.toml` raiz, possivelmente `02_shell/cli` (expor/fatorar JSON), `00_nucleo/lessons/0070-boca_mcp.md` |
