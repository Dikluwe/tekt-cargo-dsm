# Prompt: migração de convenção — escalar ao L2 (`catalogo` + `cli`)

**Camada**: transversal (`prompts/` + cabeçalhos do `lente_catalogo` e `lente_cli`)
— no `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0063-escalar_l2.md`)
**Pré-requisito**: 0062 (L1 completo e com guarda de pureza; regra dos internos
fixada e **refinada** — ver abaixo).
**Objetivo**: aplicar o molde aos **2 crates L2** — `lente_catalogo` e `lente_cli`
(este já é, desde o 0057, **lib de apresentação pura**: `args`/`saida`, sem `[[bin]]`,
sem dep de `lente_wiring`). **Confirma esses crates em V1/V5/V6/V7 = 0 e V3 = 0**
(direção L2 — nada de import de L4); o V1 do projeto cai de 24. Preserva comportamento.

---

## O molde + a regra refinada dos internos (seguir igual)

- **Por arquivo com interface pública** (inclui `pub(crate)` — o snapshot do linter
  os captura, ver 0062) → prompt de nucleação real (Propósito · Comportamento ·
  Restrições · Critérios · `## Interface Snapshot` semente vazia · Histórico).
- **Cabeçalho**: `//! Crystalline Lineage / @prompt prompts/<unidade> / @prompt-hash
  / @layer L2 / @updated` — **`@layer L2`** nos dois crates.
- **Snapshot GERADO** (não à mão; pode não ser vazio); ordem **`--update-snapshot`
  → `--fix-hashes`**.
- **Regra dos internos (refinada pelo 0062/0061)**: `[excluded_files]` suspende
  **TODAS** as checagens sobre o arquivo — inclusive a **direção V3**, que vale em
  **toda** camada. Logo: arquivo de **lógica ativa** ou **com imports significativos**
  → **fica no walk** (prompt mínimo + cabeçalho), em qualquer camada. `[excluded_files]`
  **só** para: testes, quarentena (que sai), agregador puro só-`pub mod` (sem corpo
  nem imports substantivos).
- **`prompt/` intocado** (ler, não mover).

---

## O que fazer

1. Para **cada crate** (`lente_catalogo`, `lente_cli`): **enumerar as unidades** —
   arquivos com interface → nucleação; agregador puro/teste → `[excluded_files]`;
   lógica ativa interna (se houver) → prompt mínimo (fica no walk). Registrar a lista.
2. Por **unidade com interface**: prompt em `prompts/<unidade>.md` (molde, semente
   vazia) + cabeçalho `@layer L2`. Ler código + prompt de trabalho como referência,
   sem mover.
3. **Fluxo**: `crystalline-lint --update-snapshot .` → `crystalline-lint --fix-hashes .`.
4. **Linter completo** e confirmar:
   - Os 2 crates L2: **V1/V5/V6/V7 = 0**.
   - **V3 = 0** — em especial **o `cli` não importa o `lente_wiring` (L4)** (a
     inversão fechada no refactor 0055–0057 segue fechada).
   - **V12 = 1** (`ErroLente`, intencional), **demais = 0**; o **V1 do projeto cai**
     pelos arquivos migrados (de 24).
5. **Verificar**: suíte **273 + 28** (só cabeçalhos + prompts novos); `cargo build`.

---

## Atenção por crate

- **`lente_catalogo`** (`@layer L2`): interface rica de **`pub const`** (o catálogo).
  O snapshot do linter pode **não** capturar `const` (capturou funções/tipos até
  agora) — se o snapshot sair pequeno, **tudo bem**, mas o **prompt** deve descrever
  o catálogo fielmente (o que cada grupo de constantes representa). Registrar o que o
  `--update-snapshot` de fato captura para `const`.
- **`lente_cli`** (`@layer L2`): `args` (parse/estrutura de argumentos) e `saida`
  (formatação de saída) — lib de apresentação. **Confirmar V3 = 0**: sem dep nem
  import de `lente_wiring`/L4 (estado pós-0057). Se houver um `lib.rs` só-`pub mod`,
  vai para `[excluded_files]`; se reexporta (`pub use`), é unidade (nucleia).

---

## O que NÃO fazer

- Mover/renomear o `prompt/`.
- Stub ou cópia — prompts **reais**.
- Escrever o snapshot à mão — **gerar**.
- Mudar código (se aparecer V2 num arquivo de interface sem `#[cfg(test)]`, **reportar,
  não corrigir**).
- Excluir arquivo de **lógica ativa** ou com imports — esses ficam no walk.
- Migrar L3/L4 — este prompt é **só o L2**.

---

## Critérios de Verificação

```
Dado os 2 crates L2
Então cada unidade com interface tem prompt em prompts/ (snapshot gerado) +
cabeçalho //! Crystalline Lineage @layer L2; agregador puro/teste em [excluded_files]

Dado o fluxo --update-snapshot → --fix-hashes
Então snapshots reais e hashes finais

Dado o linter completo
Então os 2 crates: V1/V5/V6/V7 = 0; V3 = 0 (cli não importa L4); V12 = 1; demais 0;
o V1 do projeto caiu de 24

Dado a suíte
Então 273 + 28 — comportamento idêntico

Dado o prompt/
Então intocado
```

---

## Resultado esperado

- As unidades por crate + os prompts de nucleação em `prompts/` (`@layer L2`,
  snapshot gerado); agregadores/testes em `[excluded_files]`.
- O linter: os 2 crates limpos (V1/V5/V6/V7 = 0), **V3 = 0** (direção L2 preservada,
  `cli` sem L4), V12 = 1, demais 0; **V1 do projeto reduzido** (de 24).
- A suíte **273 + 28**.
- **Laudo** em `00_nucleo/lessons/0063-…`: as unidades, o que o snapshot capturou do
  `catalogo` (em especial `const`), a confirmação de V3 = 0 no `cli`, e o que falta
  (L3 `infra`, L4 `wiring`/`app` + o V2 do `consulta`).

---

## Cuidados

- **`@layer L2`** nos dois (não L1).
- **V3 = 0 é um critério ativo** aqui — a inversão L2→L4 foi o que o refactor
  0055–0057 fechou; confirmar que segue fechada (o `cli` sem `lente_wiring`).
- **Snapshot gerado** — não presumir vazio nem cheio; o `catalogo` (consts) pode
  surpreender; registrar o real.
- **Lógica ativa fica no walk** (regra refinada) — exclusão só p/ teste/quarentena/
  agregador puro.
- **Comportamento idêntico** — só `//!` + prompts; suíte 273 + 28; `prompt/` intocado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Escala do molde ao L2: `lente_catalogo` e `lente_cli` (lib de apresentação pós-0057: `args`/`saida`, sem `[[bin]]`/`lente_wiring`). Unidades com interface nucleadas — prompt real em `prompts/<unidade>.md` (semente vazia) + cabeçalho `//! Crystalline Lineage @layer L2` + snapshot gerado (`--update-snapshot`→`--fix-hashes`). **Regra dos internos refinada** (do 0062/0061): `[excluded_files]` suspende TODAS as checagens — inclusive direção V3 (toda camada), não só pureza V4/V13 (L1) — logo lógica ativa/com imports fica no walk em qualquer camada; exclusão só p/ teste/quarentena/agregador puro. Atenção: `catalogo` (interface rica de `pub const` — registrar o que o snapshot captura), `cli` (confirmar **V3 = 0**: sem import de L4, inversão do refactor 0055–0057 segue fechada). Linter: os 2 crates **V1/V5/V6/V7 = 0**, **V3 = 0**, V12 = 1, demais 0; V1 do projeto cai de 24; V2 reportado se surgir (não corrigido). **Preserva comportamento**: suíte 273 + 28; `prompt/` intocado. Falta: L3 (`infra`), L4 (`wiring`/`app`, `@layer L4`, V12 do `ErroLente`) + o teste do `consulta`. | `00_nucleo/prompts/*.md` (novos do L2), `02_shell/{catalogo,cli}/src/*.rs` (cabeçalhos), `crystalline.toml` (`[excluded_files]`, se houver agregador/teste), `00_nucleo/lessons/0063-...` |
