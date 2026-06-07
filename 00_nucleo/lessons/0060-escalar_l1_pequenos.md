# Laudo de Execução — Prompt 0060 (escalar aos crates L1 pequenos)

**Camada**: transversal (`prompts/` + cabeçalhos de filtro/estrutura/investiga/resolve)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0060-escalar_l1_pequenos.md`
**Estado**: `EXECUTADO` — os 4 crates L1 pequenos migrados; **V1/V5/V6/V7 = 0** em
cada. V1 do projeto **31 → 24**. V3=0, V12=1, demais 0 (refactor preservado). Suíte
**273 + 28**. `prompt/` intocado.

---

## A resposta em uma sentença

Com o `lente_ranking` (0058) e o `lente_core` (0059) já feitos, os 4 crates L1
pequenos — `filtro`/`estrutura`/`investiga`/`resolve` — fecharam a camada L1: cada
`lib.rs` com interface nucleado, os internos sem `pub` excluídos, V1 do projeto
caindo, sem tocar código.

---

## As unidades por crate (lista real)

| Crate | Nucleado (prompt) | Excluído (`[excluded_files]`) |
|---|---|---|
| `lente_filtro` | `lib.rs` → `filtro.md` (`filtrar_stdlib`/`filtrar_so_referencia`) | `tests/e2e_lente_core.rs` (teste de integração) |
| `lente_estrutura` | `lib.rs` → `estrutura.md` (`agregar_por_modulo`/`detectar_ciclos`/`ordenar_dsm`; `Ciclo`/`DependenciaModulo`/`EstruturaModulos`/`OrdemDsm`) | — |
| `lente_investiga` | `lib.rs` → `investiga.md` (`investigar`; `ParColidente`/`ArestasNo`/`Vizinhanca`/`ArquivoFonte`) | `src/fontes.rs` (E2 quarentena), `src/vizinhanca.rs` (`pub(crate)`) |
| `lente_resolve` | `lib.rs` → `resolve.md` (`aplicar`; `ErroResolve`) | — |

4 prompts de nucleação; 3 exclusões (2 internos + 1 teste).

---

## Refinamento do molde: a regra de exclusão estendida

O 0059 excluía **agregadores só-`pub mod`**. O 0060 confirma a regra geral, mais
ampla: **arquivo sem interface pública (`pub`) → `[excluded_files]`, não
nucleado.** Três categorias caíram nela:

1. **Internos `pub(crate)`** — `investiga/vizinhanca.rs` (helper interno; o
   `analisar` é `pub(crate)`, não escapa do crate).
2. **Quarentena** — `investiga/fontes.rs` (E2, laudo 0014; será removido — não
   vale nuclear).
3. **Teste de integração** — `filtro/tests/e2e_lente_core.rs` (harness de
   verificação, não unidade arquitetural).

Quem tem `pub` (incl. re-exports) → nucleia; quem não tem → exclui. Simples e
consistente.

---

## O snapshot — gerado, casa a interface real

Confirmação (4 arquivos, "0 stale"). Exemplos:

- `resolve`: `{"functions":[{"name":"aplicar","params":["&Grafo","&Path","&Veredito"],"return_type":"Result<Grafo, ErroResolve>"}],"types":[{"name":"ErroResolve","kind":"enum","members":["ColisaoNaoResolvida","ColisaoInexistente","IdInconsistente"]}]}`
- `estrutura`: 3 funções + 4 tipos (`Ciclo`/`DependenciaModulo`/`EstruturaModulos`/`OrdemDsm`).
- `investiga`: `investigar(ParColidente, &Vizinhanca, Option<&[ArquivoFonte]>) -> Veredito` + 4 structs.

Fluxo travado: `--update-snapshot` (4) → `--fix-hashes` (4).

---

## Resultado do linter

| Check | Os 4 crates | Projeto |
|---|---|---|
| **V1/V5/V6/V7** | **0** (cada) | V1 **24** (era 31 — caíram 7: 4 migrados + 3 excluídos); V5/V6/V7 = 0 |
| **V3 / V12** | — | **0 / 1** (refactor preservado) |
| V4/V8/V9/V13/V14 | 0 | 0 |

**Critérios do prompt atendidos.**

### V2 = 1 — ainda só o `consulta.rs` (pré-existente, fora do escopo)

Nenhum dos 4 crates introduziu V2: seus `lib.rs` têm `#[cfg(test)]`. O único V2 do
projeto segue sendo o `consulta.rs` (do 0059) — **reportado, não corrigido** (muda
código). Permanece a recomendação: um teste mínimo num prompt à parte.

---

## Verificação de comportamento

| Item | Resultado |
|------|-----------|
| `cargo build --workspace` | passa |
| `cargo test --workspace` | **273 + 28, 0 falhas** — só `//!` + prompts novos |
| `prompt/` (singular) | **intocado** |
| `prompts/` (novo) | `ranking` (0058) + 7 do core (0059) + 4 destes = 12 prompts |

---

## O que falta escalar

- **L2**: `lente_catalogo` (`@layer L2`; muitos `pub const` — interface rica),
  `lente_cli` (agora lib de apresentação: `args`/`saida`).
- **L3**: `lente_infra` (vários arquivos: `fork`/`workspace`/`diff`/`metadata`/
  `traducao`/`dto`/…).
- **L4**: `lente_wiring` + `lente_app` (`@layer L4`; aqui o **V12 do `ErroLente`**
  se declara intencional).
- **V1 atual = 24** → cai a ~0 conforme L2/L3/L4 migram (menos os agregadores/
  internos/testes excluídos).
- **V2 do `consulta.rs`** — teste mínimo, prompt à parte.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Escala do molde (0058) + regra de agregadores (0059) aos 4 crates L1 pequenos. Nucleados os `lib.rs` com interface: `filtro` (`filtrar_stdlib`/`filtrar_so_referencia`), `estrutura` (`agregar_por_modulo`/`detectar_ciclos`/`ordenar_dsm` + `Ciclo`/`DependenciaModulo`/`EstruturaModulos`/`OrdemDsm`), `investiga` (`investigar` + 4 structs), `resolve` (`aplicar`/`ErroResolve`) — prompt real em `prompts/<unidade>.md` + cabeçalho `//! Crystalline Lineage @layer L1` + snapshot gerado (`--update-snapshot`→`--fix-hashes`). **Refinamento do molde**: regra de exclusão estendida — arquivo sem interface `pub` → `[excluded_files]`: `investiga/fontes.rs` (E2 quarentena), `investiga/vizinhanca.rs` (`pub(crate)`), `filtro/tests/e2e_lente_core.rs` (teste de integração). Linter: os 4 crates **V1/V5/V6/V7 = 0**; V1 do projeto 31→24; V3=0, V12=1, demais 0. V2=1 segue só no `consulta.rs` (pré-existente, reportado, não corrigido). **Preserva comportamento**: suíte 273 + 28; `prompt/` intocado. Falta L2/L3/L4 + o teste do `consulta.rs`. | `00_nucleo/prompts/{filtro,estrutura,investiga,resolve}.md` (novos), `01_core/{filtro,estrutura,investiga,resolve}/src/lib.rs` (cabeçalhos), `crystalline.toml` (`[excluded_files]`), `00_nucleo/lessons/0060-escalar_l1_pequenos.md` |
