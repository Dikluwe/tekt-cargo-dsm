# Prompt: reestruturar o L1 — `01_core` como pasta-camada (mover os crates para dentro)

**Camada**: transversal (layout do workspace) — reestrutura que preserva
comportamento
**Criado em**: 2026-06-06
**Estado**: `PROPOSTO`
**Pré-requisito**: 0037→0048 commitados; 0049 executado (`crystalline.toml`
existe).
**Decisão (sua, no 0049)**: o linter está correto — camada é diretório, e cada
crate vive dentro do diretório da sua camada. O L1 é a exceção a corrigir.
**Objetivo**: mover os **seis** crates L1 para dentro de `01_core`, fazendo do
`01_core` uma **pasta-camada pura** (como o `02_shell` já é). Com isso o **V8
zera**, e — ganho importante — o linter passa a **inspecionar a pureza** (V4, V13,
V14) dos cinco crates que antes caíam como camada Desconhecida e não eram
checados.
**Natureza**: **mudança que preserva comportamento.** Só se movem **diretórios**,
caminhos de `path` no `Cargo.toml` e o `crystalline.toml`. **Nenhuma** lógica de
código, **nenhum** nome de pacote, **nenhum** cabeçalho de linhagem muda.

---

## Estrutura-alvo

```
tekt-cargo-dsm/
├── 00_nucleo/            (L0 — intocado)
├── 01_core/              (L1 — PASTA-CAMADA pura, sem src/ próprio)
│   ├── core/             ← era 01_core/      (pacote lente_core)
│   ├── investiga/        ← era 05_investiga/ (pacote inalterado)
│   ├── resolve/          ← era 06_resolve/
│   ├── filtro/           ← era 07_filtro/
│   ├── ranking/          ← era 08_ranking/
│   └── estrutura/        ← era 09_estrutura/
├── 02_shell/             (L2 — intocado: catalogo, cli)
├── 03_infra/             (L3 — intocado)
├── 04_wiring/            (L4 — intocado)
├── lab/                  (workspace separado — só caminhos, ver abaixo)
├── Cargo.toml            (workspace — members atualizados)
└── crystalline.toml      (L1 = "01_core" cobre os seis agora)
```

Os **prefixos numéricos** (`05`–`09`) são largados lá dentro — todos são L1, o
número era ordenação de topo, agora redundante (igual a `catalogo`/`cli` no
`02_shell`, sem prefixo). Os **nomes dos pacotes** (`[package] name = "lente_*"`)
**não mudam** — só as pastas se movem, então nenhum `use lente_xxx` muda.

---

## O que fazer

1. **Mover com `git mv`** (preservar histórico — **não** `mv`/`rm`+criar):
   - O crate atual `01_core` (Cargo.toml + src/ + tests/ + o que houver) vai para
     **`01_core/core/`**. Sequência segura: mover o crate para um nome temporário,
     criar `01_core/` como pasta, mover o temporário para `01_core/core`. (Mover
     "para dentro de si mesmo" direto é frágil.)
   - `05_investiga` → `01_core/investiga`
   - `06_resolve` → `01_core/resolve`
   - `07_filtro` → `01_core/filtro` (leva o `tests/` junto)
   - `08_ranking` → `01_core/ranking`
   - `09_estrutura` → `01_core/estrutura`
2. **Atualizar o `Cargo.toml` do workspace** — os `members` para os caminhos
   novos (`01_core/core`, `01_core/investiga`, …). O `02_shell/*`, `03_infra`,
   `04_wiring` ficam.
3. **Atualizar TODA dependência `path` entre crates** (a parte delicada). Cada
   `path = "../0X_..."` recalculado para o destino novo, relativo à **nova**
   localização do crate que depende. Exemplos:
   - de `04_wiring/Cargo.toml`: `lente_core = { path = "../01_core" }` →
     `path = "../01_core/core"`; `lente_resolve = { path = "../06_resolve" }` →
     `path = "../01_core/resolve"`.
   - **entre crates L1** (agora irmãos dentro de `01_core`): de
     `01_core/resolve/Cargo.toml`, o `lente_core` fica em `path = "../core"`; um
     `lente_investiga` fica em `path = "../investiga"`.
   - O `lente_core` é a fundação — **quase todo crate depende dele**; é onde há
     mais caminhos a mexer. Ser minucioso.
4. **Atualizar o `crystalline.toml`**: `L1 = "01_core"` já cobre os seis crates
   (o V8 some). Ajustar `[module_layers]` / `[l1_ports]` se o re-run exigir, de
   modo que **V3 e V9 sigam em 0** (a direção de import e as portas continuam
   válidas — só mudou o disco, não a topologia lógica).
5. **`lab/` (workspace separado)**: se algum proto referencia os crates movidos
   via `path` (o lab importa de produção), atualizar esses caminhos também e
   verificar que o workspace do `lab` ainda resolve.
6. **Verificar** (a guarda):
   - `cargo build` no workspace — passa (todos os `path` resolvem).
   - **Suíte completa verde: 273 + 28 ignored**, 0 falhas — **comportamento
     inalterado** (era o estado do 0048; nada de lógica mudou).
   - `crystalline-lint` re-rodado (mesmos `--checks` do 0049, sem v5/v6/v7):
     **V8 = 0**.
   - Os cabeçalhos de linhagem **intocados** (continuam `@layer L1`; o `@prompt`
     aponta para o mesmo prompt — nada move em `00_nucleo/`).

---

## O ganho a reportar: pureza dos cinco crates antes não inspecionados

Antes, `investiga`/`resolve`/`filtro`/`ranking`/`estrutura` eram camada
Desconhecida (V8), então os checks de pureza do L1 (**V4** I/O, **V13** estado
mutável, **V14** externo) **não rodavam neles**. Agora rodam. **Reportar o que
disparam:**
- Se **limpos** → a pureza dos **seis** crates L1 fica confirmada (fecha o buraco
  do laudo 0049).
- Se algum **V4/V13** disparar → é achado **real** de pureza, antes escondido —
  **reportar, não silenciar**.
- **V14**: pode haver mais **falsos positivos** do mesmo tipo do 0049 (`use
  EnumLocal::*` lido como externo). Distinguir V14 real (dependência externa
  genuína) do falso positivo (glob de enum local) — não adicionar enum local ao
  `[l1_allowed_external]`.

---

## O que NÃO fazer

- Mudar lógica de código, nome de pacote, ou cabeçalho de linhagem.
- Usar `mv`/`rm`+criar (perde histórico) — só `git mv`.
- Mexer nos outros achados do 0049 (V1 cabeçalhos, V12 enums no L4, V14 falso
  positivo) — são decisões separadas; este prompt é **só** a reestrutura do L1
  (zerar o V8).
- Deixar qualquer `path` quebrado — `cargo build` tem que passar.

---

## Critérios de Verificação

```
Dado o layout
Então os seis crates L1 estão sob 01_core (core, investiga, resolve, filtro,
ranking, estrutura), e 01_core não tem src/ próprio (pasta pura)

Dado o Cargo.toml do workspace
Então os members apontam para os caminhos novos, e cargo build passa (todo path
resolve)

Dado a suíte
Quando rodada
Então 273 verdes + 28 ignored, 0 falhas — comportamento inalterado

Dado crystalline-lint re-rodado (mesmos --checks do 0049)
Então V8 = 0; e V4/V13/V14 agora rodam nos cinco crates antes alienígenas — o que
disparam está reportado (real vs falso positivo de glob de enum local)

Dado os cabeçalhos e o código
Então intocados (git mv preservou histórico; @layer L1 inalterado; nenhuma lógica
mexida)
```

---

## Resultado esperado

- A árvore nova (`01_core` pasta pura com os seis crates).
- O `Cargo.toml` do workspace atualizado + a lista de quais `Cargo.toml` de crate
  tiveram `path` recalculado.
- A mudança no `crystalline.toml`.
- Os caminhos do `lab` atualizados (se havia).
- `cargo build` ok + suíte **273 + 28** + `crystalline-lint` re-rodado com **V8 =
  0**, e o que **V4/V13/V14** mostram agora nos cinco crates recém-inspecionados
  (limpo, esperançosamente — ou o achado real, ou os falsos positivos de enum
  local).
- **Laudo** em `00_nucleo/lessons/0050-…`: os movimentos, os `path` recalculados,
  a mudança de config, os resultados de build/suíte/lint, e o veredito sobre a
  pureza dos seis crates L1.

---

## Cuidados

- **Os caminhos relativos são onde quebra** — recalcular cada `path` com cuidado;
  o `cargo build` é a guarda final.
- **O `lente_core` toca o maior número de dependências** (todos dependem da
  fundação) — ser minucioso ao mover para `01_core/core`.
- **O `lab` é workspace separado** — seus `path` para produção também mudam.
- **`git mv`** para preservar histórico.
- **Pureza recém-inspecionada** — se V4/V13 dispararem nos cinco crates, é achado
  real (antes escondido pelo V8); reportar, não calar. V14 de glob de enum local
  é falso positivo, distinguir.
- **Não tocar** cabeçalhos, código, nem os outros achados do 0049.
- **Comportamento preservado** — a suíte fica em 273 + 28; se mudar, alguma coisa
  além de mover foi alterada — investigar.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Reestrutura do L1 para a pasta-camada pura (decisão do 0049: o linter está correto — crate vive dentro do diretório da camada). `01_core` deixa de ser crate e vira pasta-camada com seis crates dentro: `core` (era `lente_core` em `01_core/`), `investiga`/`resolve`/`filtro`/`ranking`/`estrutura` (eram `05`–`09`, prefixo numérico largado). Movimento com `git mv` (histórico preservado); nomes de pacote `lente_*` inalterados (só pastas movem, nenhum `use` muda); cabeçalhos `@layer L1` intocados. Atualizados: `members` do workspace, **toda** dependência `path` entre crates (recalculada — `lente_core` é onde há mais, todos dependem dele), `crystalline.toml` (`L1=01_core` cobre os seis; V3/V9 seguem 0), caminhos do `lab` para produção. **Mudança que preserva comportamento**: nenhuma lógica, suíte fica 273 + 28. Guarda dupla: `cargo build` + suíte verde + `crystalline-lint` re-rodado com **V8 = 0**. Ganho: V4/V13/V14 passam a rodar nos cinco crates antes alienígenas — pureza dos seis crates L1 finalmente inspecionada (achado real reportado se disparar; V14 de glob de enum local é falso positivo). Não tocados: os outros achados do 0049 (V1 cabeçalhos, V12 enums no L4, V14 falso positivo) — decisões separadas. | (movidos) `01_core/{core,investiga,resolve,filtro,ranking,estrutura}`; `Cargo.toml` (workspace + cada crate com `path`); `crystalline.toml`; `lab/*/Cargo.toml` (se referencia produção); `00_nucleo/lessons/0050-...` |
