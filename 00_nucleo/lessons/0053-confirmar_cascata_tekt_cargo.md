# Laudo de Execução — Prompt 0053 (confirmar a cascata: linter consertado + remover a whitelist do 0050)

**Camada**: transversal (config + verificação)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0053-confirmar_cascata_tekt_cargo.md`
**Estado**: `EXECUTADO` — linter consertado (0052) instalado; whitelist dos seis
`lente_*` **removida** do `crystalline.toml`; lint re-rodado. **Único `.rs`
intocado** — só o `crystalline.toml` mudou. A cascata do 0052 confirmada no
projeto real, **com dois achados a reportar** (um V3 real e um resíduo de V14).

---

## A resposta em uma sentença

O conserto do 0052 funciona no projeto real — os 22 `use lente_*::` agora
resolvem para L1 **sem** a whitelist (liquidada) e o falso positivo do `Kind`
sumiu; mas a nova consciência de cross-crate **revelou um achado real (V3 = 8: a
CLI L2 importa o `lente_wiring` L4)** e deixou **um resíduo (V14 = 1: um teste de
integração importando o próprio pacote)** que o conserto ainda não cobre.

---

## Versão/commit do linter consertado

- `crystalline-lint v0.1.0` (a string de versão **não** mudou) — instalado de
  `cargo install --path /home/dikluwe/Documentos/Antigravity/tekt-linter --force`.
- O distintivo vs o linter do 0049/0050: o **conserto do 0052 está na working
  tree do clone, NÃO commitado** (clone HEAD = `43e11a3`; `03_infra/rs_parser.rs`,
  `mod.rs`, `main.rs` modificados + `03_infra/crate_registry.rs` novo, todos
  unstaged). **Ressalva ao pré-requisito do prompt** ("0052 commitado"): não está
  commitado — buildei da working tree. Funciona, mas registre-se: o binário atual
  reflete um estado não-commitado do clone.

---

## A mudança no `crystalline.toml` (único artefato mexido)

`[l1_allowed_external] rust` passou de `["lente_core", …os seis…]` para `[]`.
Comentário trocado: a whitelist (aberta no 0050 como contorno do falso positivo
do V14) foi **liquidada** porque o 0052 (`classify_import` ciente das deps reais
de cada crate) resolve `use lente_*::` para a camada do membro first-party — não
mais Desconhecida —, então a dep intra-L1 já não dispara V14.

---

## O estado do lint (mesmos `--checks` do 0049/0050 — maçã com maçã)

| ID | 0050 (linter v0.1.0 + whitelist) | 0053 (linter consertado, SEM whitelist) | Veredito |
|----|----|----|----|
| **V14** | 1 (`Kind` FP; 22 `lente_*` mascarados) | **1** (`lente_filtro` em teste) | cascata provada; resíduo novo |
| **V3** | 0 (cego cross-crate) | **8** | **achado real** — L2 importa L4 |
| **V9** | 0 (cego) | **0** | disciplina de porta vale cross-crate |
| **V8** | 0 | **0** | reestrutura do 0050 preservada |
| **V4 / V13** | 0 | **0** | pureza dos seis L1 mantida |
| V1 | 40 | 40 | fora do escopo |
| V12 | 5 | 5 | fora do escopo |

### V14 = 1 — cascata provada, com um resíduo (reportado, NÃO mascarado)

- Os **22** `use lente_core::…`/`lente_*` que no 0050 só não disparavam por causa
  da whitelist agora **resolvem para L1 sem ela** — a dep intra-L1 deixou de ser
  "externa Desconhecida". A whitelist está **liquidada** (objetivo do prompt).
- O **falso positivo do `Kind`** (`use Kind::*`, glob de enum local) **sumiu** — o
  0052 faz o glob não emitir import. Confirmado: `Kind` não aparece mais no V14.
- **Resíduo (1):** `lente_filtro` em `01_core/filtro/tests/e2e_lente_core.rs:14`.
  É um **teste de integração** (`tests/`, unidade de compilação separada)
  importando o **próprio pacote** pelo nome (`use lente_filtro::…`, idioma normal
  do Rust). O `classify_import` resolve pelas **deps declaradas** do crate-dono, e
  um crate não declara a si mesmo como dep → Desconhecida → V14. **Caso que o
  conserto do 0052 não cobre** (auto-import de teste). Conforme o prompt, **não
  recoloquei a whitelist para mascarar** — reporto que o conserto tem essa lacuna
  (a tratar a montante, no `tekt-linter`: incluir o próprio pacote nas deps dos
  arquivos de `tests/`).

### V3 = 8 — ACHADO REAL (antes invisível)

Todas as 8: **a CLI L2 (`02_shell/cli`) importa o `lente_wiring` L4** — "Inversão
de gravidade: L2 não pode importar de L4". O linter antigo resolvia
`lente_wiring::` para Desconhecida → V3 cego; o 0052 resolve para L4 e a direção
fica visível.

```
02_shell/cli/src/erro.rs:8      use lente_wiring::ErroLente
02_shell/cli/src/main.rs:18     use lente_wiring::{AlvoBusca, Escopo, FonteGrafo, ModoUses}
02_shell/cli/src/saida.rs:16    use lente_wiring::{Escopo, EstruturaModulos, ItemRanking, ModoUses, ResultadoDiff, TocadoComRaio}
02_shell/cli/src/saida.rs:971   use lente_wiring::{Ciclo, DependenciaModulo}
02_shell/cli/src/saida.rs:1142  use lente_wiring::{Fantasma, RaioCombinado, ResultadoDiff, TocadoComRaio}
… +3 (saida.rs:1227, 1306, 1321)
```

É **real**: a CLI (a casca L2) depende da fiação (L4) para rodar os pipelines
(`calcular_raio_de_alvo`, `analisar_diff`, …). Na ordem do Tekt, a fiação é o
mais externo (compõe tudo); a casca não deveria importar dela. **Não consertei**
(escopo do prompt = só o `crystalline.toml`). É decisão arquitetural a tomar —
candidatos: (a) a CLI/binário ser o ponto de composição L4 (mover `02_shell/cli`
para L4, ou o `lente_wiring` ser consumido por um `main` L4); (b) reclassificar a
relação. **Reportado, não mascarado** — o prompt manda exatamente isso.

### V9 = 0 — disciplina de porta vale cross-crate

Com o 0052, o V9 passou a enxergar cross-crate e **segue 0**: nenhum crate importa
fundo num subdir de membro L1 fora das portas (`[l1_ports]` = `entities`,
`domain`). A disciplina de porta vale entre crates, não só dentro.

---

## Código e suíte — intocados

Só o `crystalline.toml` mudou. Nenhum `.rs` tocado (git status: os `.rs` seguem as
renomeações do 0050, zero mudança de conteúdo). `cargo build`/`test` inalterados —
**273 + 28 ignored** (estado do 0048/0050); não re-rodei (nada de código mudou).

---

## Veredito

- **Cascata do 0052 confirmada no projeto real**: os `lente_*` resolvem para L1, o
  `Kind` FP sumiu, e a **whitelist do 0050 foi liquidada** (`rust = []`).
- **Mas a cascata abriu dois itens**, ambos reportados, nenhum mascarado:
  1. **V3 = 8 (real)** — a CLI L2 importa o `lente_wiring` L4. Decisão arquitetural
     pendente; não é deste prompt consertar.
  2. **V14 = 1 (resíduo)** — auto-import de pacote num teste de integração; lacuna
     do conserto do 0052, a tratar a montante no `tekt-linter`.
- **Pré-requisito não plenamente satisfeito**: o conserto do 0052 está na working
  tree do clone, **não commitado** — registrado.

---

## Postscript (2026-06-07) — resíduo V14 liquidado, pré-requisito fechado

Os dois itens acima foram tratados a montante, no `tekt-linter`:

- **Resíduo V14 = 1 → 0.** A causa exata: `use lente_filtro::filtrar_stdlib`
  (`01_core/filtro/tests/e2e_lente_core.rs:14`) é um self-import pelo nome do
  próprio pacote; o `classify_import` resolvia via `module_layer(seg[1])`, e como
  `filtrar_stdlib` (função re-exportada na raiz) não está em `[module_layers]`,
  virava `Unknown` — e o `package_name` é o nome do crate (não isento como
  `crate`/`std`), logo V14. **Fix**: um self-import nunca é externo; quando o
  sub-módulo não está mapeado, cai na **camada do próprio crate** (`owner.layer`),
  não `Unknown`. Re-rodado com o binário instalado: **V14 = 0 sem a whitelist** —
  a whitelist do 0050 fica definitivamente liquidada.
- **Pré-requisito agora satisfeito.** O 0052 + o fix do resíduo estão **commitados**
  no `tekt-linter` (branch `feat/0052-classificacao-ciente-deps`: `d6a0612` o 0052,
  `c479db7` o fix). O binário em `~/.cargo/bin` foi reinstalado (`cargo install
  --path … --force`) a partir do estado commitado — não mais da working tree.
- **V3 = 8 permanece** — é o achado arquitetural real do `tekt-cargo-dsm` (CLI L2 →
  `lente_wiring` L4), fora do escopo do conserto do linter; decisão à parte.

Estado final do lint (binário instalado, mesmos `--checks`): **V14 = 0, V9 = 0,
V8 = 0, V4/V13 = 0**; V3 = 8 (achado), V1 = 40, V12 = 5 (fora do escopo).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Confirmação da cascata do 0052 no `tekt-cargo-dsm` e liquidação da whitelist do 0050. Instalado o linter consertado (`cargo install --path … --force`; `crystalline-lint v0.1.0`, **conserto do 0052 na working tree do clone, NÃO commitado** — HEAD `43e11a3` — registrado como ressalva ao pré-requisito). Removidos os seis `lente_*` de `[l1_allowed_external]` (`rust = []`; comentário trocado). Rodado `crystalline-lint .` com os mesmos `--checks` do 0049/0050. **V14: 1→1**, mas a cascata provada — os 22 `lente_*` resolvem para L1 **sem** a whitelist (liquidada) e o falso positivo do `Kind` sumiu; o resíduo (1) é `lente_filtro` num **teste de integração** importando o próprio pacote (`tests/e2e_lente_core.rs:14`), caso que o conserto não cobre — reportado, **não** mascarado (whitelist não recolocada). **V3: 0→8 — achado real** antes invisível: a CLI L2 (`02_shell/cli`) importa o `lente_wiring` L4 (inversão de gravidade); reportado, **não** consertado (escopo = só config; decisão arquitetural pendente). **V9 = 0** cross-crate-ciente (disciplina de porta vale entre crates). **V8 = 0** (reestrutura do 0050 preservada); V4/V13 = 0; V1 = 40 e V12 = 5 (fora do escopo). Código e suíte intocados (só o `crystalline.toml`; 273 + 28). | `crystalline.toml`; `00_nucleo/lessons/0053-confirmar_cascata_tekt_cargo.md` |
