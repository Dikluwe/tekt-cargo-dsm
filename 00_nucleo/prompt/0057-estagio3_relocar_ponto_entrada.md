# Prompt: refactor V3+V12, Estágio 3 — relocar o ponto de entrada (a CLI vira apresentação pura)

**Camadas tocadas**: **L4** (novo crate `app` no `04_wiring`, com o binário; recebe
o `main` + a tradução do `ErroLente`), **L2** (`02_shell/cli` vira biblioteca de
apresentação pura). No `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0057-estagio3_relocar_ponto_entrada.md`)
**Pré-requisito**: 0056 (Estágio 2; V3 = 1, V12 = 1 — ambos `ErroLente`).
**Decisão fechada**: ponto de entrada = **crate app no `04_wiring`**.
**Objetivo**: o binário "lente" sobe para um crate **app** em `04_wiring` (L4); a
CLI vira **apresentação pura** que só importa L1 (+ catálogo L2 lateral) e **deixa
de importar L4**. **Preserva comportamento** (mesma CLI, mesma saída).
**Delta esperado: V3 1 → 0.** V12 fica **1** (`ErroLente`, aceito — warning
legítimo).

---

## Contexto

Depois do 0056, o **único** V3 que resta é a CLI (L2) importando o `ErroLente` do
`lente_wiring` (L4). O `ErroLente` **fica** no L4 (agrega erros do L3). Então a CLI
não pode importá-lo. A solução Cristalina: o **ponto de entrada** — o que chama a
orquestração e trata o `ErroLente` — **é L4**. Sobe para um crate `app`, e a CLI
vira apresentação pura.

---

## O que fazer

1. **Criar `04_wiring/app/`** — crate L4 com o `[[bin]] name = "lente"`. Adicionar
   ao `members` do workspace. Deps `path`: `lente_wiring` (L4), `lente_core` (L1) e
   a apresentação L2 (`lente_cli`, `lente_catalogo`). **L4 importa L1/L2/L3/L4 —
   permitido.**
2. **Mover o `main`** (o ponto de entrada / o *dispatch*) de `02_shell/cli` para
   `04_wiring/app/src/main.rs`. Ele: parseia args (usando os structs `clap` da CLI
   L2), chama a orquestração (`lente_wiring`: `calcular_raio_de_alvo`,
   `analisar_diff`, …), trata o `ErroLente`, e chama os formatadores (CLI L2).
3. **Mover a tradução do `ErroLente`** (`erro.rs::traduzir`) para o `app` L4 — lá
   ela conhece legitimamente o `ErroLente` (L4) e usa os templates do `catalogo`
   (L2, para baixo). É o que **tira o `ErroLente` da CLI sem movê-lo de camada**.
4. **A CLI (`02_shell/cli`) vira biblioteca de apresentação pura**: tirar o
   `[[bin]]`; manter `args.rs` (structs `clap`) e `saida.rs` (formatadores sobre
   tipos **L1**) como módulos de lib. **Remover a dep de `lente_wiring`** do `cli`
   (não importa mais L4). O que em `erro.rs` **não** for sobre `ErroLente` (se
   houver) fica na CLI; a parte que é (`traduzir`) sai para o `app`.
5. **Atualizar o alvo do binário**: o "lente" passa a buildar de `04_wiring/app`.
   Os testes que exercem o **binário/orquestração** movem para o `app` se for o
   caso; os que testam **apresentação** ficam na CLI — a **suíte fica 273 + 28** (a
   contagem se mantém; se redistribuir, sem perder nem ganhar).
6. **Verificar**:
   - `cargo build` — o binário "lente" builda de `04_wiring/app`.
   - **Smoke test do binário** (a guarda além da suíte): rodar o "lente" nos
     comandos reais (ex.: `lente <path>`, `lente --diff --repo .`, `lente … --vista
     resumo/item/camadas`) e conferir que a **saída é idêntica** à de antes.
   - **Suíte 273 + 28**.
   - `crystalline-lint` (mesmos `--checks`): **V3 = 0**, **V12 = 1** (só
     `ErroLente`), V8/V4/V9/V13/V14 = 0.

---

## Sobre o V12 = 1 (`ErroLente`) que resta

É **warning** (não bloqueia CI) e é **legítimo**: o erro agregado mora na
composição (L4), onde o L1 e o L3 se encontram. Declarar **intencional** — um
comentário no `crystalline.toml`/no laudo explicando por que o `ErroLente` é L4
(agrega `Fork`/`Adaptador`/`Workspace`/`Diff` do L3). **Não** há mudança de config
obrigatória (é warning); se um dia o linter ganhar exceção por-declaração, ajusta.

---

## O que NÃO fazer

- Mover o `ErroLente` de camada — **fica L4**; só a **tradução** sobe ao `app`.
- Mudar o comportamento da CLI — **mesmos comandos, mesma saída** (o smoke test
  prova).
- Mudar a orquestração (`lente_wiring`) — só o **ponto de entrada** se relocaliza.

---

## Critérios de Verificação

```
Dado o 04_wiring/app
Então tem o [[bin]] "lente", o main, e a tradução do ErroLente; é L4 e importa
L1/L2/L3/L4 (permitido)

Dado a CLI (02_shell/cli)
Então é lib de apresentação pura — sem [[bin]], sem dep de lente_wiring, importando
só L1 (+ catálogo L2 lateral)

Dado cargo build
Então o binário "lente" builda de 04_wiring/app

Dado o smoke test do binário
Então a saída é IDÊNTICA à de antes nos comandos reais (radius, diff, vistas)

Dado a suíte
Então 273 + 28 (a contagem se mantém)

Dado crystalline-lint
Então V3 = 0, V12 = 1 (só ErroLente, intencional), V4/V14/V8/V9/V13 = 0
```

---

## Resultado esperado

- O crate `04_wiring/app` (`Cargo.toml` + `main.rs` + a tradução do `ErroLente`).
- A CLI virada lib (sem `[[bin]]`, sem dep de `lente_wiring`, só L1 + catálogo).
- O `members` do workspace atualizado.
- `cargo build` + **smoke test** (saída idêntica) + suíte **273 + 28** +
  `crystalline-lint` (**V3 = 0**, V12 = 1).
- **Laudo** em `00_nucleo/lessons/0057-…` com o **estado final do refactor**: V3
  8→0, V12 5→1 (`ErroLente`, intencional), e o `ErroLente` documentado como L4
  legítimo.

---

## Cuidados

- **Comportamento da CLI idêntico** — o **smoke test do binário** (não só a suíte)
  é a prova; rodar os comandos reais e comparar a saída byte a byte se der.
- **O `ErroLente` fica L4** — só a tradução sobe; não descer o erro agregado.
- **V12 = 1 é aceito** (warning; erro agregado legítimo no L4) — declarar
  intencional (comentário), sem mudança de config obrigatória.
- **Os testes do binário podem mover para o `app`** — a suíte fica 273 + 28; se a
  contagem mudar, algo além de relocar mudou.
- **O `app` (L4) importando a CLI (L2) é permitido** (L4→L2, para baixo).
- Este é o estágio mais estrutural — o binário muda de casa; conferir que o alvo
  `lente` builda e roda do `04_wiring/app`.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Estágio 3 (final) do refactor V3+V12 (decisão: ponto de entrada = crate app no `04_wiring`). Criado `04_wiring/app` — crate L4 com o `[[bin]] "lente"`, deps `path` em `lente_wiring`/`lente_core`/`lente_cli`/`lente_catalogo`. Movido o `main` (dispatch: args→orquestração→tradução de erro→formatadores) e a `traduzir(ErroLente)` de `02_shell/cli` para o `app` L4 — lá o `ErroLente` (L4) é conhecido legitimamente e usa os templates do `catalogo` (L2). A CLI (`02_shell/cli`) virou **biblioteca de apresentação pura**: sem `[[bin]]`, sem dep de `lente_wiring` (não importa mais L4), só `args.rs` (clap) + `saida.rs` (formatadores sobre tipos L1) + catálogo L2 lateral. Binário "lente" passou a buildar de `04_wiring/app`; testes de binário/orquestração movidos para o `app` (suíte 273 + 28 mantida). **Preserva comportamento**: smoke test do binário (radius/diff/vistas) com saída idêntica, além da suíte. `ErroLente` **fica L4** (agrega L3) — só a tradução subiu; V12 = 1 dele declarado intencional (warning legítimo — erro agregado na composição). Delta final: **V3 1→0** (refactor V3 8→0 completo), **V12 5→1**. | `04_wiring/app/` (novo: `Cargo.toml`, `src/main.rs`, tradução), `Cargo.toml` (workspace members), `02_shell/cli/` (vira lib: `Cargo.toml`, `src/{lib.rs,args.rs,saida.rs}`, remove `main`/`bin`/`lente_wiring`), `00_nucleo/lessons/0057-...` |
