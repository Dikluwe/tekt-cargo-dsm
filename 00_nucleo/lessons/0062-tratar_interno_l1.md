# Laudo de Execução — Prompt 0062 (tratar o interno de lógica do L1 — restaurar a pureza)

**Camada**: transversal (`crystalline.toml` + `prompts/` + cabeçalho de um arquivo)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0062-tratar_interno_l1.md`
**Estado**: `EXECUTADO` — `vizinhanca.rs` saiu do `[excluded_files]`, ganhou prompt
mínimo + cabeçalho, **voltou ao walk**: V4/V13 o checam (e passam — é puro).
**Guarda de pureza restaurada** (provado). Suíte **273 + 28**. `prompt/` intocado.

---

## A resposta em uma sentença

O único interno de **lógica ativa L1** que estava excluído — `investiga/vizinhanca.rs`
— voltou à checagem: agora o linter o **vê** (prova: V4/V13 reinjetados disparam
nele), e ele passa por ser puro; a regra dos internos fica fixada.

---

## 1. O escopo do `[excluded_files]` — confirmado

Varredura da lista (antes do 0062): `core_lib`/`core_domain_mod`/`core_entities_mod`
(agregadores só-`pub mod`), `investiga_fontes` (E2 **quarentena**, sai),
`investiga_vizinhanca` (**lógica ativa**), `filtro_e2e_test` (teste de integração).
→ **O `vizinhanca.rs` era o único interno de lógica ATIVA do L1** lá dentro.
Nenhum outro escondido.

---

## 2. O tratamento

- **`crystalline.toml`**: removida a entrada `investiga_vizinhanca`; comentário
  reescrito fixando a regra (`[excluded_files]` reservado a teste/quarentena/
  agregador puro; lógica ativa L1 não entra).
- **`00_nucleo/prompts/vizinhanca.md`** (novo): prompt de nucleação **mínimo e
  real** — Estratégia 1 da investigação (compara conjuntos de arestas → `Veredito`),
  com a nuance da `ChaveAresta` por `id` (não path) e as restrições de pureza.
- **Cabeçalho** migrado: `//! Crystalline Lineage / @prompt prompts/vizinhanca.md
  / @prompt-hash … / @layer L1 / @updated`.
- **Fluxo**: `--update-snapshot` → `--fix-hashes` ("0 stale", "0 drift").
- **Lógica**: **intocada** (só o cabeçalho `//!` mudou no `.rs`).

---

## 3. A guarda restaurada — provado, não suposto

| Verificação | Resultado |
|---|---|
| `vizinhanca.rs` no run normal | **0 violações** (V1/V5/V6/V7 = 0; **V4 = 0, V13 = 0** — checado e puro) |
| **Reinjeção** (worktree descartável): `static mut` + `std::fs::read_to_string` em `vizinhanca.rs` | **V13 e V4 DISPARAM** (l.277/278) |

A reinjeção é a prova-chave: o arquivo **voltou ao walk**, então a pureza é
**vigiada de novo** (ao contrário do estado pré-0062, em que a mesma injeção ficava
em silêncio — laudo 0061). Worktree removido; repo real intocado.

### Achado: o snapshot **não** ficou vazio

A semente era vazia, mas o `--update-snapshot` gerou:
`{"functions":[{"name":"analisar","params":["&ArestasNo","&ArestasNo"],"return_type":"ResultadoVizinhanca"}],"types":[{"name":"ResultadoVizinhanca","kind":"enum",…}]}`.
Ou seja, o `public_interface` do linter **inclui itens `pub(crate)`** (não só `pub`).
Nuance útil para escalar: o snapshot de um interno **pode** ter conteúdo — o
`--update-snapshot` produz o correto de qualquer forma; não presumir vazio.

---

## A regra dos internos (fixada para o L2/L3/L4)

- **Interno de lógica ATIVA do L1** (`pub(crate)` com corpo, que **fica**) →
  **prompt mínimo + cabeçalho** (fica no walk → **V4/V13 checadas**). Snapshot
  conforme o linter gerar.
- **`[excluded_files]` reservado a**: testes, quarentena (será removida),
  agregadores só-`pub mod` (sem corpo — V4/V13 não teriam o que checar).
- **Interno do L3+** (`lente_infra` etc.): `[excluded_files]` **seguro** — V4/V13 não
  se aplicam a L3 (I/O legítimo), nada de guarda a perder.

---

## Resultado do linter / verificação

| Item | Resultado |
|------|-----------|
| `vizinhanca.rs` | V1/V5/V6/V7 = 0; **V4 = 0, V13 = 0** (guarda ativa, passa) |
| `[excluded_files]` | enxuto: só `fontes` (quarentena) + `filtro_e2e_test` (teste) + 3 agregadores |
| Projeto | **V1 = 24** (inalterado — era excluído, agora tem cabeçalho válido), V2 = 1 (`consulta`, pré-existente), V3 = 0, V12 = 1, demais 0 |
| `cargo test --workspace` | **273 + 28, 0 falhas** (lógica intocada) |
| `cargo build` | passa |
| `prompt/` (singular) | intocado |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Decisão do 0061 aplicada: o interno de lógica ativa do L1 ganha **prompt mínimo** em vez de exclusão, para manter a guarda de pureza. Confirmado que `investiga/vizinhanca.rs` era o **único** interno de lógica L1 ativo no `[excluded_files]` (os demais: teste, quarentena, agregadores). Removida a entrada do `crystalline.toml`; criado `prompts/vizinhanca.md` (mínimo, real — Estratégia 1: compara conjuntos de arestas → `Veredito`); cabeçalho migrado (`@layer L1`); fluxo `--update-snapshot`→`--fix-hashes` (snapshot **não** vazio — o `public_interface` do linter inclui `pub(crate)`: `analisar`/`ResultadoVizinhanca`). **Guarda restaurada e provada**: V4/V13 reinjetados (worktree descartável) **disparam** no `vizinhanca.rs` (l.277/278) — voltou ao walk; no run normal V4/V13 = 0 (puro). `[excluded_files]` enxuto (teste/quarentena/agregadores). Projeto V1=24 inalterado, V3=0, V12=1. **Regra dos internos fixada**: lógica ativa L1 → prompt mínimo (checada); exclusão só p/ teste/quarentena/agregador puro; interno L3+ → exclusão segura. **Preserva comportamento** (só `//!` + prompt + config; lógica intocada): suíte 273 + 28; `prompt/` intocado. | `00_nucleo/prompts/vizinhanca.md` (novo), `01_core/investiga/src/vizinhanca.rs` (cabeçalho), `crystalline.toml` (`[excluded_files]`), `00_nucleo/lessons/0062-tratar_interno_l1.md` |
