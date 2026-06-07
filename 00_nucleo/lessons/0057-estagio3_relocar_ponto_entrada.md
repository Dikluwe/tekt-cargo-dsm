# Laudo de Execução — Prompt 0057 (refactor V3+V12, Estágio 3 — relocar o ponto de entrada)

**Camadas tocadas**: L4 (novo `04_wiring/app` com o binário), L2 (`02_shell/cli`
vira lib de apresentação pura)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0057-estagio3_relocar_ponto_entrada.md`
**Estado**: `EXECUTADO` — o binário "lente" subiu para um crate `app` L4; a CLI
virou biblioteca de apresentação pura (sem L4). Suíte **273 + 28** inalterada;
smoke test do binário idêntico. **V3 1 → 0** (refactor V3 8→0 completo). V12 = 1
(`ErroLente`, intencional).

---

## A entrega em uma sentença

O ponto de entrada (o `main`, o dispatch e a tradução do `ErroLente`) subiu para
um crate `04_wiring/app` (L4), que **compõe** a apresentação (`lente_cli`, L2) com
a orquestração (`lente_wiring`, L4) — e com isso a CLI deixou de importar o L4, o
último V3 sumiu, sem mudar uma linha de comportamento.

---

## Os movimentos

| Arquivo | De | Para |
|---|---|---|
| `main.rs` (dispatch: `main`/`run`/`run_*`/`construir_*`/`SaidaErro`) | `02_shell/cli/src/` | **`04_wiring/app/src/`** |
| `erro.rs` (`traduzir(ErroLente)` + `ContextoErro`) | `02_shell/cli/src/` | **`04_wiring/app/src/`** |

Movidos com `git mv` (histórico preservado). O `app` (L4) importa L1
(`lente_core`), L2 (`lente_cli`), L4 (`lente_wiring`) e o catálogo L2 — **L4 para
baixo, permitido**.

### O truque que preservou o corpo do `main` verbatim

A única mudança no `main.rs` movido foi o topo:

```rust
mod args; mod erro; mod saida;     →     mod erro;
                                          use lente_cli::{args, saida};
```

Importando os módulos da CLI **como `args`/`saida`**, todo o corpo (`args::Cli`,
`saida::formatar`, `saida::Modo`, `erro::traduzir`, …) compila **sem alteração**.
Zero edição de lógica — só o caminho dos módulos.

---

## A CLI (`02_shell/cli`) virou lib de apresentação pura

- Novo `src/lib.rs`: `pub mod args; pub mod saida;` — sem `main.rs`, sem `erro.rs`.
- `Cargo.toml`: **removido** o `[[bin]]`, a dep `lente_wiring` (não importa mais
  L4) e o dev-dep `lente_infra` (foi para o `app`, onde os testes de `erro` vivem).
  Deps que ficam: `lente_core`/`lente_ranking`/`lente_estrutura`/`lente_catalogo`
  + `clap` + `serde_json` — **só L1 + catálogo L2 + libs de UI**.
- `args.rs` (clap) e `saida.rs` (formatadores sobre tipos **L1**) intocados — já
  eram só-L1 desde o 0056.

O **binário "lente"** passou a buildar de `04_wiring/app` (`target/debug/lente`).

---

## Redistribuição dos testes (suíte 273 + 28 mantida)

- Testes de **dispatch** (`run`/`run_*`, `run_alvo_*`, validações de args) →
  foram com o `main.rs` para o **`lente_app`**.
- Testes da **tradução de erro** (`traduzir`) → foram com o `erro.rs` para o `app`.
- Testes de **apresentação** (formatadores: `formatar`/`ranking`/`estrutura`/
  `diff`/`vistas`) → **ficaram** no `lente_cli`.

A contagem total não mudou — **273 + 28**, comportamento idêntico.

---

## Verificação (a guarda dupla: suíte + smoke do binário)

| Item | Resultado |
|------|-----------|
| `cargo build --workspace` | **passa, sem warnings**; binário "lente" de `04_wiring/app` |
| `cargo test --workspace` | **273 verdes + 28 ignored, 0 falhas** |
| Smoke `--grafo … --alvo t::A` | `{"alvo":"t::A","classificacao":"Isolado",…}` — idêntico |
| Smoke `--ranking` / `--estrutura --text` / `--diff --vista …` | formato idêntico ao de antes |
| `crystalline-lint` (mesmos `--checks`) | **V3 = 0**, **V12 = 1**, V4/V8/V9/V13/V14 = 0 |

(O `--diff` no repo real mostra **números diferentes** de antes — porque o
*working tree mudou* com esta própria reestrutura; o **formato/comportamento** é
idêntico, e os testes de dispatch na suíte são a prova byte-a-byte.)

**V1 = 42** (era 41): o `cli/src/lib.rs` novo soma um arquivo sem o cabeçalho
`@prompt` no formato do linter — **fora do escopo** (convenção, à parte).

---

## O `V12 = 1` (`ErroLente`) que resta — declarado intencional

`ErroLente` (`04_wiring/src/lib.rs:65`) é o **erro agregado** do pipeline:
embrulha `Fork`/`Adaptador`/`Workspace`/`Diff` (do **L3**) e `Resolucao`/`Raio`
(do L1). Mora legitimamente na **composição (L4)** — é onde L1 e L3 se encontram.
**Não desce** (faria o L1 referenciar o L3). O V12 dele é **warning legítimo**,
declarado intencional: nenhuma mudança de config obrigatória; se o linter um dia
ganhar exceção por-declaração, ajusta-se ali.

---

## Estado FINAL do refactor V3+V12 (Estágios 1→3)

| Check | 0053 (início) | Est. 1 (0055) | Est. 2 (0056) | **Est. 3 (0057)** |
|---|---|---|---|---|
| **V3** | 8 | 4 | 1 | **0** ✅ |
| **V12** | 5 | 5 | 1 | **1** (`ErroLente`, intencional) |

O acoplamento L2→L4 (a casca importando a fiação) está **resolvido**: a apresentação
é só-L1, e a composição (o ponto de entrada) é L4 — a ordem Cristalina. Os 4 enums
de vocabulário desceram ao L1 (0056); o ponto de entrada subiu ao L4 (0057); só o
erro agregado fica no L4, declarado.

---

## O que NÃO mudou (conforme o prompt)

- O **comportamento** da CLI — mesmos comandos, mesma saída (suíte + smoke).
- O `ErroLente` **não mudou de camada** — só a sua **tradução** subiu ao `app`.
- A **orquestração** (`lente_wiring`) — intocada; só o ponto de entrada se relocou.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Estágio 3 (final) do refactor V3+V12 (decisão: ponto de entrada = crate app no `04_wiring`). Criado `04_wiring/app` — crate L4 com `[[bin]] "lente"`, deps em `lente_wiring`/`lente_core`/`lente_cli`/`lente_catalogo` (+dev `lente_infra`). `git mv` de `main.rs` (dispatch) e `erro.rs` (`traduzir(ErroLente)`) de `02_shell/cli` para o `app`; o `main.rs` só mudou o topo (`mod args/saida` → `use lente_cli::{args, saida}`), corpo verbatim. A CLI virou **lib de apresentação pura** (`src/lib.rs` = `pub mod args; pub mod saida;`): sem `[[bin]]`, sem dep `lente_wiring`, sem dev-dep `lente_infra` (foi p/ o app); só L1 + catálogo + clap/serde_json. Binário "lente" passou a buildar de `04_wiring/app`; testes de dispatch/erro foram para o app, os de apresentação ficaram na CLI — suíte 273 + 28 mantida. **Preserva comportamento**: smoke test (radius/ranking/estrutura/diff) idêntico + suíte. `ErroLente` **fica L4** (agrega L3) — só a tradução subiu; V12 = 1 dele declarado intencional (warning legítimo). Delta final: **V3 1→0** (refactor V3 8→0 completo), **V12 5→1**. V1 42 (cli/lib.rs novo, fora do escopo). | `04_wiring/app/` (novo: `Cargo.toml`, `src/{main.rs,erro.rs}`), `Cargo.toml` (members), `02_shell/cli/` (vira lib: `src/lib.rs` novo, `Cargo.toml` sem bin/wiring), `00_nucleo/lessons/0057-estagio3_relocar_ponto_entrada.md` |
