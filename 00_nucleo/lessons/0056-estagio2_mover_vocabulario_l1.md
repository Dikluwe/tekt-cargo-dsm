# Laudo de Execução — Prompt 0056 (refactor V3+V12, Estágio 2 — mover o vocabulário L4-nativo ao L1)

**Camadas tocadas**: L1 (`lente_core`: `consulta` novo; `lente_estrutura`: +2 tipos),
L4 (`lente_wiring`: deixa de definir, importa do L1), L2 (CLI: re-aponta a parte ii)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0056-estagio2_mover_vocabulario_l1.md`
**Estado**: `EXECUTADO` — os 6 tipos L4-nativos puros movidos para o L1. Suíte
**273 + 28** inalterada. **V3 4 → 1**, **V12 5 → 1** (sobra só `ErroLente`).
**V4/V14 = 0** — a prova de que nada externo desceu ao L1.

---

## A entrega em uma sentença

Movi os 6 tipos que o `lente_wiring` definia mas só dependem do L1 — os 4 enums
de pedido para `lente_core::domain::consulta`, e `EstruturaModulos`/`DependenciaModulo`
para `lente_estrutura` (junto do `Ciclo` que referenciam) — sem mudar lógica: o
V3 cai a 1 e o V12 a 1, ambos o `ErroLente` (que **fica** no L4, agrega L3).

---

## Os movimentos

| Tipo(s) | De | Para |
|---|---|---|
| `FonteGrafo`, `Escopo`, `ModoUses`, `AlvoBusca` | `lente_wiring` (L4) | **`lente_core::domain::consulta`** (módulo novo) |
| `EstruturaModulos`, `DependenciaModulo` | `lente_wiring` (L4) | **`lente_estrutura`** (junto do `Ciclo`) |

- **`01_core/core/src/domain/consulta.rs`** (novo) — os 4 enums, **fiéis** (mesmos
  docs, derives, `impl Default` de `Escopo`/`ModoUses`); `pub mod consulta;` no
  `domain/mod.rs`. `AlvoBusca::PorPath(Path)` usa `crate::entities::grafo::Path`.
- **`lente_estrutura`** ganhou `DependenciaModulo` e `EstruturaModulos` (que usa
  `Path`/`DependenciaModulo`/`Ciclo`, todos já no crate).

---

## Pureza L1 — a guarda-mestra (CONFIRMADA)

Os 6 tipos descem **só com derives da std** (`Debug`/`Clone`/`Copy`/`PartialEq`/
`Eq`; `FonteGrafo`/`AlvoBusca` sem derive). **Nenhum `serde`** nem outro externo
(grep confirma: zero `serde` em `consulta.rs`/`estrutura`). Provas:

- `cargo tree -p lente_core` = **só o crate** (nada arrastado).
- **V4 = 0** (sem I/O no L1), **V14 = 0** (sem externo no L1).

A serialização do projeto é à mão (na L2); a derive de `serde` **não** desceu.

---

## `lente_wiring` (L4) — deixa de definir, passa a importar

- Removidas as 6 definições (`04_wiring/src/lib.rs`).
- Adicionados re-exports do L1 (que também servem de import local nas assinaturas):
  `pub use lente_core::domain::consulta::{AlvoBusca, Escopo, FonteGrafo, ModoUses};`
  e `pub use lente_estrutura::{Ciclo, DependenciaModulo, EstruturaModulos, OrdemDsm};`.
- As 5 funções (`calcular_raio_de_alvo`/`rankear_pacote`/`analisar_estrutura`/…)
  usam os tipos nas assinaturas **sem mudança** — agora resolvem para os do L1.
- Re-exportar (em vez de só `use`) mantém compat e **não** dispara V12 (re-export
  não é declaração de enum).

---

## CLI (L2) — a parte (ii) re-apontada; só o `ErroLente` resta do wiring

| Sítio | Antes | Depois |
|---|---|---|
| `main.rs:18` | `lente_wiring::{AlvoBusca, Escopo, FonteGrafo, ModoUses}` | `lente_core::domain::consulta::{…}` |
| `saida.rs:20` | `lente_wiring::{Escopo, EstruturaModulos, ModoUses}` | `consulta::{Escopo, ModoUses}` + `lente_estrutura::EstruturaModulos` |
| `saida.rs:976` | `lente_wiring::DependenciaModulo` | `lente_estrutura::{Ciclo, DependenciaModulo}` |

Após isto, o **único** `use lente_wiring` na CLI é `erro.rs:8` (`ErroLente`) — o
V3 remanescente, que sai no Estágio 3 (relocação do ponto de entrada).

**Deps no `cli/Cargo.toml`**: `lente_estrutura` **promovido** de dev-dep para dep
**regular** (`EstruturaModulos` é usado no formatador `formatar_estrutura`,
não-teste). `lente_core`/`lente_ranking` já eram regulares.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo build --workspace` | **passa** |
| `cargo test --workspace` | **273 verdes + 28 ignored, 0 falhas** — comportamento idêntico (mesmos tipos, outro crate) |
| `cargo tree -p lente_core` | **só o crate** — pureza L1 intacta |
| `crystalline-lint` (mesmos `--checks`) | **V3 = 1**, **V12 = 1**, **V4 = 0**, **V14 = 0**, V8/V9/V13 = 0 |

Os remanescentes V3=1 e V12=1 são **ambos o `ErroLente`** (`erro.rs:8` na CLI;
`04_wiring/src/lib.rs` a declaração) — o erro agregado que **fica** no L4 (Estágio 3).

**V1 = 41** (era 40): o `consulta.rs` novo soma um arquivo Cristalino sem o
cabeçalho `@prompt` no formato do linter — **fora do escopo** (decisão de
convenção, à parte; ver 0049).

---

## Ripple — contido (como o 0054 previu)

Só a CLI e as assinaturas do `lente_wiring` referenciavam o vocabulário; **nenhum
outro crate** quebrou no `cargo build`. O mapa do 0054 estava certo — o movimento
tocou exatamente CLI + wiring (+ as casas L1 que receberam).

---

## Estado do refactor (progresso)

| Check | 0053 | 0055 (Est. 1) | **0056 (Est. 2)** | meta (Est. 3) |
|---|---|---|---|---|
| V3 | 8 | 4 | **1** | 0 |
| V12 | 5 | 5 | **1** | 1 (`ErroLente`, intencional) |

Falta o **Estágio 3**: relocar o ponto de entrada (binário sobe a um crate app no
`04_wiring`; a CLI vira apresentação pura só-L1; a tradução do `ErroLente` sobe
junto) → V3 1→0. O V12 final fica em 1 (o `ErroLente` declarado intencional).

---

## O que NÃO mudou (conforme o prompt)

- O `ErroLente` (fica L4) e o ponto de entrada (Estágio 3) — intocados.
- **Nenhuma lógica** — movimento de tipo, não de comportamento (suíte é a prova).
- Nenhum externo arrastado ao L1 (V4/V14 = 0).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Estágio 2 do refactor V3+V12 (mapa 0054; decisão: os 4 enums de pedido → `lente_core::domain::consulta`). Movidos os 6 tipos L4-nativos puros para o L1: `FonteGrafo`/`Escopo`/`ModoUses`/`AlvoBusca` → `lente_core::domain::consulta` (módulo novo, tipos fiéis com `impl Default` de `Escopo`/`ModoUses`); `EstruturaModulos`/`DependenciaModulo` → `lente_estrutura` (junto do `Ciclo`). `lente_wiring` removeu as 6 definições e passou a **re-exportar/importar** do L1 nas assinaturas das 5 funções. CLI re-apontou a parte (ii) (`main.rs:18`, `saida.rs:20/976`) para o L1 — sobra só `erro.rs:8`/`ErroLente`. `lente_estrutura` promovido a dep regular do `cli` (`EstruturaModulos` no formatador não-teste). **Pureza L1 confirmada**: os tipos descem só com derives da std, sem `serde`; `cargo tree -p lente_core` só o crate; **V4/V14 = 0**. Suíte 273 + 28 (mesmos tipos, outro crate). Ripple contido (CLI + wiring, como o 0054 previu). Delta: **V3 4→1**, **V12 5→1** (sobra só `ErroLente` nos dois). V1 41 (consulta.rs novo, fora do escopo). Não tocados: `ErroLente` (L4) e o ponto de entrada (Estágio 3). | `01_core/core/src/domain/{consulta.rs (novo),mod.rs}`, `01_core/estrutura/src/lib.rs` (+2 tipos), `04_wiring/src/lib.rs` (remoções + re-exports), `02_shell/cli/src/{main.rs,saida.rs}` + `Cargo.toml`, `00_nucleo/lessons/0056-estagio2_mover_vocabulario_l1.md` |
