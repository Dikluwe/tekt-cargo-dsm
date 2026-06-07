# Laudo de Execução — Prompt 0065 (escalar ao L4 — `wiring` + `app`, último passo)

**Camada**: transversal (`prompts/` + cabeçalhos do L4)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0065-escalar_l4.md`
**Estado**: `EXECUTADO` — L4 migrado; **V1 do projeto = 0** (migração de cabeçalhos
**completa**). V3 = 0, V12 = 1 (`ErroLente`, intencional, documentado). Suíte
**273 + 28**; binário `lente` roda. `prompt/` intocado.

---

## A resposta em uma sentença

O L4 (`lente_wiring` + `lente_app`) entrou na convenção e — com a Arena (`lab/`) e
o crate-fixture excluídos como não-lattice — o projeto inteiro chegou a **V1 = 0**:
todo `.rs` tem cabeçalho Cristalino ou exclusão justificada; resta só o `ErroLente`
(V12, intencional) e o V2 do `consulta` (próximo).

---

## As unidades do L4 (3, todas nucleadas — `@layer L4`)

| Unidade (prompt) | Arquivo | Interface (do snapshot) |
|---|---|---|
| `wiring` | `04_wiring/src/lib.rs` | 5 pipelines (`calcular_raio_de_alvo`/`rankear_pacote`/`analisar_estrutura`/`analisar_diff`/`montar_grafo_workspace`); `ErroLente`/`GrafoWorkspace`; re-exports do L1 |
| `app-main` | `04_wiring/app/src/main.rs` | `SaidaErro` (dispatch; `main`/`run` não-`pub`) |
| `app-erro` | `04_wiring/app/src/erro.rs` | `traduzir(&ErroLente, &ContextoErro) -> String`; `ContextoErro` |

Nenhum agregador puro no L4 → nada novo em `[excluded_files]` por agregação.

---

## `ErroLente` — V12 = 1 intencional, documentado

`ErroLente` (`wiring/lib.rs`) **agrega** os erros das camadas internas via `From`:
`Fork`/`Adaptador`/`Workspace`/`Diff` (do **L3**) + `Resolucao`/`Raio` (do L1) +
`IdInexistente`/`ForkSemUsesKind`. É um **erro de composição** — só na fiação, onde
L1 e L3 se encontram, juntá-los faz sentido; **não desce ao L1** (faria L1→L3). O
prompt `wiring.md` documenta a residência. **V12 = 1 permanece como warning
aceito** — não é defeito, é o tipo-soma dos erros que a composição propaga com `?`.
Config **não** mudada (se o linter ganhar exceção por-declaração, marca-se lá).

---

## Como o projeto chegou a V1 = 0 (o marco)

O V1 = 12 antes deste prompt era **3 L4 + 3 fixtures + 6 lab**. Resolução:

- **3 L4** → nucleados (acima).
- **3 fixtures** (`03_infra/tests/fixtures/crate-amostra/src/*.rs`) — crate-fixture
  de teste, **excluído do workspace cargo** (`exclude` no `Cargo.toml`): harness, não
  arquitetura → `[excluded] fixture = "crate-amostra"` (poda o subdir no walk).
- **6 lab** (`lab/...`) — a **Arena** (manifesto Tekt: código descartável, workspace
  próprio, **fora do lattice**) → `[excluded] arena = "lab"` + **removido de
  `[layers]`** (não é camada). Poda a Arena inteira do walk.

Decisão registrada: exclusão de `[excluded]` (por **diretório**) para subtrees
não-lattice (Arena, fixtures), distinta de `[excluded_files]` (por **arquivo
exato**, p/ agregadores/quarentena pontuais).

---

## Estado FINAL do linter (migração de linhagem)

| Check | Projeto | Nota |
|---|---|---|
| **V1** | **0** ✅ | migração de cabeçalhos **completa** |
| V3 | 0 | direção preservada (refactor 0055–0057 fechado) |
| V5/V6/V7 | 0 | hashes/snapshots/órfãos limpos |
| V4/V8/V9/V10/V11/V13/V14 | 0 | invariantes reais limpos |
| **V12** | **1** | `ErroLente` (L4) — **intencional, documentado** |
| **V2** | **1** | `consulta.rs` (pré-existente — **próximo prompt**, muda código) |

Fluxo travado: `--update-snapshot` (3, "0 stale") → `--fix-hashes` (3, "0 drift").

---

## Verificação de comportamento

| Item | Resultado |
|------|-----------|
| `cargo build --workspace` | passa |
| `cargo test --workspace` | **273 + 28, 0 falhas** |
| Binário `lente --help` | roda — `"O que quebra se eu mexer aqui?"` (comportamento idêntico) |
| `prompt/` (singular) | intocado |

---

## O que resta (encerra a migração)

- **Só o V2 = 1 do `consulta.rs`** — um `#[cfg(test)] mod tests` mínimo (ex.:
  `Escopo::default() == Completo`, `ModoUses::default() == Todas`) zera o último V2.
  **Muda código** → prompt à parte (próximo). Depois disso, o linter fica **só com
  o V12 = 1 do `ErroLente`** (warning intencional aceito) — a convenção Cristalina
  plenamente adotada.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | **Último passo da migração de cabeçalhos** — L4 (`@layer L4`). Nucleados: `wiring` (`04_wiring/src/lib.rs` — 5 pipelines + `ErroLente` + `GrafoWorkspace` + re-exports), `app-main` (`04_wiring/app/src/main.rs` — dispatch, `SaidaErro`; `main`/`run` não-`pub`), `app-erro` (`04_wiring/app/src/erro.rs` — `traduzir(ErroLente)` + `ContextoErro`) — prompt real + cabeçalho `//! Crystalline Lineage @layer L4` (replace ou prepend) + snapshot gerado (`--update-snapshot`→`--fix-hashes`). `ErroLente` documentado como erro de composição L4 (agrega 4 erros L3) — **V12 = 1 intencional aceito** (config não mudada). **V1 do projeto = 0** (marco): além dos 3 do L4, excluídos os não-lattice — `[excluded] arena = "lab"` (Arena, descartável, removida também de `[layers]`) e `fixture = "crate-amostra"` (crate-fixture de teste, excluído do workspace) — exclusão por **diretório** (subtree não-lattice), distinta de `[excluded_files]` (arquivo exato). V3 = 0 (L4 é o topo); V4/V13/V2 não se aplicam (L1). Linter final: **V1 = 0**, V3/V5/V6/V7/V8/V9/V10/V11/V13/V14 = 0, **V12 = 1** (`ErroLente`, documentado), **V2 = 1** (só o `consulta`, pré-existente). **Preserva comportamento**: suíte 273 + 28; binário `lente` roda igual; `prompt/` intocado. Resta só o teste mínimo do `consulta` (próximo prompt, muda código) para zerar o V2 e encerrar a migração. | `00_nucleo/prompts/{wiring,app-main,app-erro}.md` (novos), `04_wiring/src/lib.rs` + `04_wiring/app/src/{main,erro}.rs` (cabeçalhos), `crystalline.toml` (`[layers]`/`[excluded]`), `00_nucleo/lessons/0065-escalar_l4.md` |
