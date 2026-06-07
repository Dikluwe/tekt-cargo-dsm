# Laudo de Execução — Prompt 0050 (reestruturar o L1 — `01_core` como pasta-camada)

**Camada**: transversal (layout do workspace) — reestrutura que preserva comportamento
**Data**: 2026-06-06
**Prompt executado**: `00_nucleo/prompt/0050-l1_pasta_camada.md`
**Estado**: `EXECUTADO` — os seis crates L1 movidos para dentro de `01_core/`
(pasta-camada pura); `cargo build` ok; suíte **273 + 28 ignored** (inalterada);
`crystalline-lint` re-rodado: **V8 = 0**. Nenhuma lógica, nome de pacote ou
cabeçalho de linhagem mudou.

---

## A entrega em uma sentença

`01_core` deixou de ser crate e virou **pasta-camada pura** com os seis crates L1
dentro (`core`, `investiga`, `resolve`, `filtro`, `ranking`, `estrutura`); só
diretórios, caminhos `path` e o `crystalline.toml` mudaram — **V8 zerou**, e a
pureza dos cinco crates antes alienígenas ficou **finalmente inspecionada e
confirmada** (V4/V13 = 0).

---

## Os movimentos (via `git mv` — histórico preservado)

| De | Para |
|---|---|
| `01_core` | `01_core/core` (pacote `lente_core`) |
| `05_investiga` | `01_core/investiga` |
| `06_resolve` | `01_core/resolve` |
| `07_filtro` | `01_core/filtro` (com `tests/`) |
| `08_ranking` | `01_core/ranking` |
| `09_estrutura` | `01_core/estrutura` |

Sequência segura para o `01_core` (move para dentro de si): `git mv 01_core
_core_tmp` → `mkdir 01_core` → `git mv _core_tmp 01_core/core`. Prefixos `05`–`09`
largados (todos são L1; o número era ordenação de topo). **Nomes de pacote
`lente_*` inalterados** — nenhum `use lente_xxx` mudou. `01_core/` não tem `src/`
próprio (pasta pura). Todos os `.rs` são **renomeação pura** (git status: nenhum
`.rs` com mudança de conteúdo; cabeçalhos `//! Lineage:` intocados).

---

## Os caminhos `path` recalculados

A regra: o segmento muda (`05_investiga` → `01_core/investiga`) e a **profundidade
`../`** muda conforme o crate que depende moveu ou não.

**Crates que ficaram** (apontam para os L1 movidos):

| Arquivo | dep | de → para |
|---|---|---|
| `03_infra/Cargo.toml` | lente_core | `../01_core` → `../01_core/core` |
| `02_shell/cli/Cargo.toml` | lente_core | `../../01_core` → `../../01_core/core` |
| `04_wiring/Cargo.toml` | core + 5 | `../01_core`→`../01_core/core`; `../05_investiga`→`../01_core/investiga`; …resolve/filtro/ranking/estrutura idem; `lente_infra` **inalterado** |

**Crates que moveram** (agora irmãos dentro de `01_core`):

| Arquivo | dep | de → para |
|---|---|---|
| `01_core/{investiga,resolve,ranking,estrutura}/Cargo.toml` | lente_core | `../01_core` → `../core` (irmão) |
| `01_core/filtro/Cargo.toml` | lente_core | `../01_core` → `../core` |
| `01_core/filtro/Cargo.toml` | lente_infra | `../03_infra` → `../../03_infra` (profundidade +1) |

`01_core/core/Cargo.toml`: sem dep inter-crate (fundação pura) — nada a mudar.

**Workspace `Cargo.toml`**: `members` apontam para `01_core/{core,investiga,…}`;
`02_shell/*`, `03_infra`, `04_wiring` ficaram; `exclude` intocado.

**`lab/`** (workspace separado, importa produção): 5 manifestos atualizados
(`medicao-colisoes/remedicao`, `medicao-egui`, `medicao-ciclos-egui`,
`proto-impacto-diff`, `medicao-ciclos-referencia`) — `01_core`/`05`–`09` para os
caminhos novos; refs a `03_infra` inalteradas. Resolução confirmada por `cargo
metadata --no-deps` em cada crate do lab (todos OK).

Técnica de edição: substituição **ancorada por aspas** (`"../01_core"` →
`"../01_core/core"`) — segura porque `"../01_core"` nunca casa dentro de
`"../../01_core"` (não há aspa interna). Guarda final: o `cargo build`.

---

## Verificação (a guarda dupla)

| Item | Resultado |
|------|-----------|
| `cargo build --workspace` | **passa** — todos os `path` resolvem das novas localizações |
| `cargo test --workspace` | **273 verdes + 28 ignored, 0 falhas** — idêntico ao 0048 (comportamento preservado) |
| `cargo metadata` no `lab/` | os 5 crates resolvem os `path` novos |
| Cabeçalhos `@layer`/`//! Lineage:` | **intocados** (git mv; nenhum `.rs` com conteúdo alterado) |

---

## O ganho: pureza dos cinco crates antes não inspecionados

Antes, `investiga`/`resolve`/`filtro`/`ranking`/`estrutura` eram camada
Desconhecida (V8 fatal) → os checks de pureza do L1 **não rodavam neles**. Agora
rodam:

| Check | Antes (0049) | Agora (0050) |
|---|---|---|
| **V8** (alien) | 8 fatal (05–09) | **0** ✓ |
| **V4** (I/O no L1) | não rodava nos 5 | **0** — limpo nos seis |
| **V13** (estado mutável no L1) | não rodava nos 5 | **0** — limpo nos seis |
| **V3** (direção de import) | 0 | **0** (preservado) |
| **V9** (vazamento de porta) | 0 | **0** (preservado) |

A pureza dos **seis** crates L1 (sem I/O, sem estado mutável) está confirmada —
fecha o buraco do laudo 0049.

---

## V14: o achado novo da inspeção (intra-L1) e o falso positivo remanescente

Com os cinco crates agora inspecionados, o **V14 subiu para 23**:

- **22 × `use lente_core::…`** (e um `lente_filtro` num teste) — os crates L1
  importando a fundação `lente_core` (e, em teste, a si mesmos). **Não é
  dependência externa de terceiros** — é **dependência intra-L1**, permitida no
  Tekt (mesma camada). Causa-raiz na fonte do linter: `rs_parser::resolve_layer`
  só resolve `crate::`/`super::`; **todo** `use lente_*::` vira `Layer::Unknown`,
  e o V14 (L1 + Unknown + não-autorizado) dispara. O linter **não distingue** um
  crate L1 first-party de um pacote de terceiros.
  **Resolução**: listar os seis `lente_*` em `[l1_allowed_external]` — é a
  declaração honesta "depender destes crates L1 é autorizado dentro do L1" (não
  esconde terceiros reais: `serde` etc. ainda dispararia). Pós-config: **esses 22
  somem**.
- **1 × `Kind`** (`01_core/core/src/entities/grafo.rs:183` = `use Kind::*;`) —
  **falso positivo** do mesmo tipo do 0049 (glob de enum local lido como pacote).
  **Não** adicionado ao config (seria errado — não é externo). Permanece, **a
  reportar upstream**.

**V14 final = 1** (só o `Kind`, falso positivo conhecido).

---

## A mudança no `crystalline.toml`

- O comentário do CRUX (0049) foi reescrito: o V8 **deixou de ser estrutural** —
  `01_core` agora é pasta-camada cobrindo os seis.
- `[l1_allowed_external] rust` passou de `[]` para os seis nomes `lente_*`, com
  comentário explicando a dependência intra-L1 e a limitação do `resolve_layer`.
- `[layers]`/`[module_layers]`/`[l1_ports]` inalterados — V3/V9 seguem 0.

---

## Estado final do lint (mesmos `--checks` do 0049)

| ID | Qtde | Nota |
|----|------|------|
| V8 | **0** | resolvido — o objetivo do prompt |
| V3, V4, V9, V13 | **0** | invariantes reais limpos (pureza dos seis confirmada) |
| V14 | **1** | só o `Kind` glob (falso positivo, reportar upstream) |
| V1 | 40 | formato de cabeçalho — **fora do escopo** (decisão separada, 0049) |
| V12 | 5 | enums no L4 — **fora do escopo** (decisão separada, 0049) |

V5/V6/V7 seguem como no 0049 (V7 aborta sem `00_nucleo/prompts`; rodado sem eles).

---

## O que NÃO mudou (conforme o prompt)

- Lógica de código, nomes de pacote, cabeçalhos de linhagem — **intocados**.
- Os outros achados do 0049 (V1 cabeçalhos, V12 enums no L4) — decisões separadas,
  **não** tocadas aqui.
- Comportamento — suíte 273 + 28 (se mudasse, algo além de mover teria mudado).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Reestrutura do L1 para pasta-camada pura (decisão do 0049). Os seis crates L1 movidos com `git mv` (histórico preservado) para `01_core/{core,investiga,resolve,filtro,ranking,estrutura}`; prefixos `05`–`09` largados; nomes de pacote `lente_*` e cabeçalhos `@layer L1` intocados; `01_core` sem `src/` próprio. Recalculados: `members` do workspace, toda dep `path` inter-crate (staying: `../01_core`→`../01_core/core` e `05`–`09`→`01_core/<nome>`; moved: `lente_core`→`../core` irmão, `filtro`→`lente_infra` `../03_infra`→`../../03_infra`), e os 5 manifestos do `lab`. `crystalline.toml`: comentário do CRUX reescrito (V8 resolvido), `[l1_allowed_external] rust` = os seis `lente_*` (dep intra-L1 autorizada — o linter resolve `use lente_*::` como Unknown e não distingue first-party de terceiros; rs_parser::resolve_layer só vê `crate::`/`super::`). **Mudança que preserva comportamento**: nenhuma lógica; `cargo build` ok; suíte 273 + 28 (idêntica ao 0048); `lab` resolve (cargo metadata). Lint: **V8 = 0**; V3/V4/V9/V13 = 0 (pureza dos seis L1 confirmada — fecha o buraco do 0049); V14 = 1 (só o `Kind` glob, falso positivo, reportar upstream); V1=40 e V12=5 fora do escopo. | (movidos) `01_core/{core,investiga,resolve,filtro,ranking,estrutura}`; `Cargo.toml` (workspace + `03_infra`/`04_wiring`/`02_shell/cli` + os 5 movidos); `crystalline.toml`; `lab/*/Cargo.toml` (5); `00_nucleo/lessons/0050-l1_pasta_camada.md` |
