# Laudo de Execução — Prompt 0055 (refactor V3+V12, Estágio 1 — re-apontar os L1-origem)

**Camada**: L2 (`02_shell/cli`)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0055-estagio1_reapontar_l1.md`
**Estado**: `EXECUTADO` — os 6 símbolos L1-origem re-apontados da fachada
`lente_wiring` para os crates L1 de origem. **Mecânico, preserva comportamento**:
suíte **273 + 28** inalterada. **V3 8 → 4**, V12 = 5 (inalterado).

---

## A entrega em uma sentença

Trocar o caminho de import de 6 tipos (que nascem no L1 e o `lente_wiring` só
re-exporta) — sem mover nada, sem mudar lógica — limpou os 4 sítios que **só**
importavam esses, baixando o V3 de 8 para 4; os 2 sítios mistos ficaram
parcialmente (a parte L4-nativa sai no Estágio 2).

---

## Os re-apontamentos (de → para)

| Símbolo | Antes | Depois (origem L1) |
|---|---|---|
| `ResultadoDiff`, `TocadoComRaio`, `RaioCombinado` | `lente_wiring` | `lente_core::domain::resultado_diff` |
| `Fantasma` | `lente_wiring` | `lente_core::domain::uniao` |
| `Ciclo` | `lente_wiring` | `lente_estrutura` |
| `ItemRanking` | `lente_wiring` | `lente_ranking` |

(Caminho público canônico de cada crate: `lente_core` **não** re-exporta na raiz —
só `pub mod domain/entities` —, então é o caminho de módulo; `Ciclo`/`ItemRanking`
são `pub struct` na raiz dos seus crates.)

**Linhas tocadas em `02_shell/cli/src/saida.rs`:**

| Linha | Tipo de sítio | Resultado |
|---|---|---|
| 16 (top-level) | misto | dividido: (i) `ResultadoDiff`/`TocadoComRaio`→`lente_core`, `ItemRanking`→`lente_ranking`; (ii) `Escopo`/`EstruturaModulos`/`ModoUses` **ficam** no wiring |
| 971 (teste) | misto | dividido: `Ciclo`→`lente_estrutura`; `DependenciaModulo` **fica** no wiring |
| 1142, 1227, 1306, 1321 (teste) | só-(i) | linha inteira vai para o L1; **somem** do `lente_wiring` |

`main.rs:18` (`AlvoBusca`/`Escopo`/`FonteGrafo`/`ModoUses`) e `erro.rs:8`
(`ErroLente`) **não tocados** — são Estágios 2 e 3.

---

## As deps `path` adicionadas a `02_shell/cli/Cargo.toml`

- `[dependencies]`: `lente_ranking = { path = "../../01_core/ranking" }` — o
  `ItemRanking` é usado no formatador `formatar_ranking` (**não-teste**), logo dep
  regular.
- `[dev-dependencies]`: `lente_estrutura = { path = "../../01_core/estrutura" }` —
  o `Ciclo` só aparece em teste (após `#[cfg(test)]`, l.746), logo dev-dep.
- `lente_core` **já** era dep regular (cobre `ResultadoDiff`/`TocadoComRaio`/
  `Fantasma`). Caminhos pós-0050 (`../../01_core/<crate>`). `cargo build` passou.

---

## O que NÃO mudou (conforme o prompt)

- O vocabulário L4-nativo (`FonteGrafo`/`AlvoBusca`/`Escopo`/`ModoUses`/
  `EstruturaModulos`/`DependenciaModulo`), o `ErroLente`, e as **chamadas de função**
  de orquestração — **intocados** (Estágios 2 e 3).
- As **re-exportações** (`pub use`) no `lente_wiring` — deixadas como estão
  (limpeza é opcional, depois).
- **Nenhuma lógica** — só caminho de import. Mesmo tipo, outro caminho.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo build -p lente_cli` | **passa** (deps `path` resolvem) |
| `cargo test --workspace` | **273 verdes + 28 ignored, 0 falhas** — idêntico ao 0048/0050 (comportamento preservado) |
| `crystalline-lint` (mesmos `--checks`) | **V3 = 4** (8→4), **V12 = 5** (inalterado), V8/V4/V9/V13 = 0, **V14 = 0** |

Os **4 V3 remanescentes** (esperados): `erro.rs:8` (`ErroLente`, Estágio 3),
`main.rs:18` (vocab ii, Estágio 2), `saida.rs:20` e `saida.rs:976` (vocab ii
restante, Estágio 2).

Nota: o **V14 = 0** (o resíduo do 0053 — self-import de pacote num teste — foi
fechado a montante no `tekt-linter`, conforme o postscript do laudo 0053; o
binário instalado já reflete o conserto).

---

## Estado do refactor (progresso dos 3 estágios)

| Check | 0053 | **0055 (Est. 1)** | meta (pós-Est. 3) |
|---|---|---|---|
| V3 | 8 | **4** | 0 |
| V12 | 5 | 5 | 1 (`ErroLente`) |

Estágio 2 (mover o vocabulário puro ao `lente_core::domain::consulta` + os de
estrutura ao `lente_estrutura`) levará V3 4→1 e V12 5→1; Estágio 3 (ponto de
entrada → crate app no `04_wiring`) leva V3 1→0.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Estágio 1 do refactor V3+V12 (mapa do 0054). Re-apontados na CLI (`02_shell/cli`) os 6 símbolos L1-origem que vinham pela fachada `lente_wiring`: `ResultadoDiff`/`TocadoComRaio`/`RaioCombinado`→`lente_core::domain::resultado_diff`, `Fantasma`→`lente_core::domain::uniao`, `Ciclo`→`lente_estrutura`, `ItemRanking`→`lente_ranking`. Sítios só-(i) (`saida.rs:1142/1227/1306/1321`, em teste) deixam de importar do wiring; mistos (`saida.rs:16` top-level, `saida.rs:971` teste) divididos — parte (i) no L1, parte (ii) ainda no wiring (sai no Est. 2). Deps add ao `cli/Cargo.toml`: `lente_ranking` (regular — `ItemRanking` no formatador) e `lente_estrutura` (dev — `Ciclo` só em teste); `lente_core` já era dep. **Mecânico, preserva comportamento**: suíte 273 + 28 inalterada. Re-exportações do `lente_wiring` deixadas; vocabulário (ii)/`ErroLente`/chamadas de função intocados (Est. 2/3). Lint: **V3 8→4**, V12 = 5, V8/V4/V9/V13 = 0, V14 = 0 (resíduo do 0053 fechado a montante). | `02_shell/cli/src/saida.rs` (imports), `02_shell/cli/Cargo.toml` (deps); `00_nucleo/lessons/0055-estagio1_reapontar_l1.md` |
