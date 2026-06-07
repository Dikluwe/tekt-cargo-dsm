# Laudo de Execução — Prompt 0063 (escalar ao L2 — `catalogo` + `cli`)

**Camada**: transversal (`prompts/` + cabeçalhos do L2)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0063-escalar_l2.md`
**Estado**: `EXECUTADO` — os 2 crates L2 migrados; **V1/V5/V6/V7 = 0**, **V3 = 0**
(o `cli` não importa L4). V1 do projeto **24 → 20**. V12 = 1, demais 0. Suíte
**273 + 28**. `prompt/` intocado.

---

## A resposta em uma sentença

O L2 (`lente_catalogo` + `lente_cli`) entrou na convenção: cada arquivo de
apresentação com interface nucleado (`@layer L2`), o agregador `cli/lib.rs`
excluído, e — o critério ativo aqui — **V3 = 0** confirmando que a inversão
L2→L4 fechada no refactor 0055–0057 **segue fechada**.

---

## As unidades por crate

| Crate | Nucleado (prompt, `@layer L2`) | Excluído |
|---|---|---|
| `lente_catalogo` | `lib.rs` → `catalogo.md` (`Template` + ~100 `pub const`) | — |
| `lente_cli` | `args.rs` → `cli-args.md` (`Cli`/`Vista`); `saida.rs` → `cli-saida.md` (formatadores) | `lib.rs` (só `pub mod args; pub mod saida;`) |

Granularidade **por arquivo**: o `cli` tem dois arquivos de interface (`args`,
`saida`) → dois prompts; o `lib.rs` agregador → `[excluded_files]`.

---

## O que o snapshot capturou (e o que não)

- **`catalogo`**: `{"types":[{"name":"Template","kind":"struct"}]}` — **os ~100
  `pub const` NÃO entram no snapshot**. Confirmado: o `public_interface` do linter
  captura `fn`/`struct`/`enum`, **não `const`**. Não é problema — o snapshot rastreia
  o `Template`; o **prompt** descreve o catálogo fielmente (os grupos `HELP_*`/
  `ERRO_*`/`JSON_*`/`ROTULO_*`/`DIFF_*`). Registrado para o L3 (que tem `const`
  também).
- **`cli/args`**: `Cli` (14 campos) + `Vista` (Resumo/Item/Camadas).
- **`cli/saida`**: 5 funções (`formatar`/`formatar_ranking`/`formatar_estrutura`/
  `formatar_diff`/`formatar_diff_vista`) + `AlvoPedido`/`Modo`.

Fluxo travado: `--update-snapshot` (3) → `--fix-hashes` (3).

---

## V3 = 0 — a direção L2 preservada (critério ativo)

O `cli` **não importa o `lente_wiring` (L4)**: as únicas menções a `lente_wiring`
no `cli` são **comentário** e **strings de fixture de teste** (`"lente_wiring::c"`),
não `use`. O linter (consciente de cross-crate desde o 0052) confirma **V3 = 0** no
projeto inteiro — a inversão de gravidade que o refactor 0055–0057 fechou
(vocabulário descido ao L1, ponto de entrada subido ao L4) **continua fechada**.

---

## Resultado do linter / verificação

| Item | Resultado |
|------|-----------|
| `catalogo`/`cli/args`/`cli/saida` | V1/V5/V6/V7 = 0 |
| **V3** | **0** (o `cli` sem L4) |
| Projeto | V1 **20** (era 24 — caíram 4: 3 migrados + `cli/lib.rs` excluído), V2 = 1 (`consulta`, pré-existente), V12 = 1, demais 0 |
| `cargo build` / `cargo test` | passa / **273 + 28, 0 falhas** |
| `prompt/` (singular) | intocado |

V2 = 1 segue só no `consulta.rs` — nenhum arquivo L2 introduziu V2 (catalogo/saida
têm `#[cfg(test)]`; `args` é estrutura clap sem lógica a testar, e o linter não o
flagrou).

---

## O que falta escalar

- **L3**: `lente_infra` — vários arquivos (`fork`/`workspace`/`diff`/`metadata`/
  `traducao`/`dto`/`invocacao`/`lib`). `@layer L3`. Atenção: internos do L3 com
  imports/lógica **ficam no walk** (regra refinada do 0062), mas V4/V13 não se
  aplicam a L3 — exclusão de agregador puro segue válida. Há `const` no L3? (o
  snapshot não os captura — registrar).
- **L4**: `lente_wiring` + `lente_app` — `@layer L4`; aqui o **V12 do `ErroLente`**
  se declara intencional.
- **V1 atual = 20** → cai a ~0 com L3/L4 (menos agregadores/internos/testes).
- **V2 do `consulta.rs`** — teste mínimo, prompt à parte.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Escala do molde ao L2. Nucleados (`@layer L2`): `catalogo` (`lib.rs` — `Template` + ~100 `pub const`), `cli/args` (`Cli`/`Vista`), `cli/saida` (5 formatadores + `AlvoPedido`/`Modo`) — prompt real em `prompts/<unidade>.md` + cabeçalho `//! Crystalline Lineage @layer L2` (replace no `catalogo`; **prepend** no `args`/`saida`, que tinham doc próprio sem `//! Lineage:`) + snapshot gerado (`--update-snapshot`→`--fix-hashes`). Agregador `cli/lib.rs` (só `pub mod`) → `[excluded_files]`. **Achado**: o `public_interface` do linter **não captura `pub const`** (o snapshot do `catalogo` só tem o `Template`) — o prompt descreve o catálogo fielmente. **V3 = 0** confirmado (critério ativo): o `cli` não importa `lente_wiring` (L4) — só menções em comentário/fixture; a inversão do refactor 0055–0057 segue fechada. Linter: os 2 crates **V1/V5/V6/V7 = 0**, **V3 = 0**, V12 = 1; V1 do projeto 24→20; V2 = 1 segue só no `consulta`. **Preserva comportamento**: suíte 273 + 28; `prompt/` intocado. Falta L3 (`infra`), L4 (`wiring`/`app`, `@layer L4`, V12 do `ErroLente`) + o teste do `consulta`. | `00_nucleo/prompts/{catalogo,cli-args,cli-saida}.md` (novos), `02_shell/{catalogo,cli}/src/*.rs` (cabeçalhos), `crystalline.toml` (`[excluded_files]` `cli_lib`), `00_nucleo/lessons/0063-escalar_l2.md` |
