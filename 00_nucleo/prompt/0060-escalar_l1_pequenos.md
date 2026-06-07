# Prompt: migração de convenção — escalar aos crates L1 pequenos

**Camada**: transversal (`prompts/` + cabeçalhos de `filtro`/`estrutura`/`investiga`/
`resolve`) — no `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0060-escalar_l1_pequenos.md`)
**Pré-requisito**: 0059 (`lente_core` migrado; molde do 0058 + regra de agregadores).
**Objetivo**: aplicar o molde aos **4 crates L1 pequenos** — `lente_filtro`,
`lente_estrutura`, `lente_investiga`, `lente_resolve` (o `ranking` já foi no 0058,
o `core` no 0059). **Confirma esses crates em V1/V5/V6/V7 = 0**; o V1 do projeto cai
de 31. Preserva comportamento.

---

## O molde + a regra (de 0058/0059 — seguir igual)

- **Granularidade por arquivo**: um prompt de nucleação por `.rs` **com interface
  pública** (inclui re-exports `pub use`). Molde: Propósito · Comportamento e
  invariantes · Restrições (L1 puro) · Critérios (`Dado/Quando/Então`) · `##
  Interface Snapshot` (semente vazia) · Histórico. **Real e fiel** — não stub, não
  cópia.
- **Agregadores só-`pub mod`** (sem interface própria: `lib.rs`/`mod.rs`
  estruturais) → **`[excluded_files]`** (path exato), **não** nucleados (ADR-0010).
- **Cabeçalho**: `//! Crystalline Lineage / @prompt prompts/<unidade> / @prompt-hash
  / @layer L1 / @updated`.
- **Snapshot GERADO**, ordem **`--update-snapshot` → `--fix-hashes`**.
- **`prompt/` intocado** (ler, não mover).

---

## O que fazer

1. Para **cada crate** (`lente_filtro`, `lente_estrutura`, `lente_investiga`,
   `lente_resolve`): **enumerar as unidades** (arquivos com interface pública;
   agregadores só-`pub mod` à parte). Registrar a lista real por crate.
2. Para **cada unidade com interface**: prompt de nucleação em `prompts/<unidade>.md`
   (molde, semente vazia) + cabeçalho migrado (`@layer L1`). Ler o código + o prompt
   de trabalho correspondente em `prompt/` como referência, sem mover.
3. Para **cada agregador só-`pub mod`**: `[excluded_files]` (não nuclear).
4. **Fluxo**: `crystalline-lint --update-snapshot .` → `crystalline-lint --fix-hashes .`.
5. **Linter completo** e confirmar: os 4 crates em **V1/V5/V6/V7 = 0**; o **V1 do
   projeto cai** pelos arquivos migrados; **V3 = 0, V12 = 1, demais = 0**.
6. **Verificar**: suíte **273 + 28** (só cabeçalhos + prompts novos); `cargo build`.

---

## Atenção por crate (pontos a capturar fiel no snapshot/prompt)

- **`lente_estrutura`**: recebeu `EstruturaModulos`/`DependenciaModulo` (do wiring,
  no 0056) **além** de `Ciclo`/`OrdemDsm` — a interface inclui os quatro.
- **`lente_resolve`**: a escada de nomeação (`aplicar_distintos`, ADR-0006/0042) e
  `ErroResolve` — capturar o que é público.
- **`lente_investiga`**: `investigar` + o que for público das fontes (a E2 está em
  quarentena — refletir só o que de fato é exposto, não o quarentenado).
- **`lente_filtro`**: `filtrar_stdlib` (e o que mais expuser).
- **V2**: se algum arquivo de interface estiver **sem `#[cfg(test)]`** (como o
  `consulta.rs` do 0059), o **V2** dispara — **reportar, não corrigir** (muda código,
  fora do escopo desta migração).

---

## O que NÃO fazer

- Renomear/mover o `prompt/`.
- Stubs ou cópias — prompts de **nucleação reais**.
- Escrever o snapshot à mão — **gerar**.
- Mudar código (inclusive não adicionar teste para o V2 — só reportar).
- Migrar **outros** crates (L2/L3/L4) — este prompt é só os **4 L1 pequenos**.

---

## Critérios de Verificação

```
Dado os 4 crates L1 pequenos
Então cada unidade com interface tem prompt de nucleação em prompts/ (snapshot
gerado) + cabeçalho //! Crystalline Lineage @layer L1; cada agregador só-pub-mod
está em [excluded_files]

Dado o fluxo --update-snapshot → --fix-hashes
Então snapshots reais e hashes finais

Dado o linter completo
Então os 4 crates: V1/V5/V6/V7 = 0; o V1 do projeto caiu por eles; V3 = 0,
V12 = 1, demais 0 (V2 reportado se surgir, não corrigido)

Dado a suíte
Então 273 + 28 — comportamento idêntico

Dado o prompt/
Então intocado
```

---

## Resultado esperado

- A lista real de unidades por crate + os prompts de nucleação em `prompts/` (com
  snapshot gerado); os agregadores em `[excluded_files]`.
- Os cabeçalhos migrados (`@layer L1`); `--update-snapshot` + `--fix-hashes`.
- O linter: os 4 crates limpos (V1/V5/V6/V7 = 0); **V1 do projeto reduzido** (de 31);
  V3 = 0, V12 = 1, demais 0; qualquer **V2** reportado.
- A suíte **273 + 28**.
- **Laudo** em `00_nucleo/lessons/0060-…`: as unidades por crate, os prompts/excluídos,
  o V1 restante, e o que falta (L2/L3/L4 + o V2 do `consulta.rs`).

---

## Cuidados

- **`prompt/` intocado**; **prompts reais e fiéis**; **snapshot gerado, ordem
  `update`→`fix`** (do 0058/0059).
- **Agregadores → `[excluded_files]`** (não nuclear); a lista cresce explícita.
- **Comportamento idêntico** — só `//!` + prompts; suíte 273 + 28.
- **`@layer L1`** nos quatro (são todos L1).
- **V2 só reportado** (corrigi-lo muda código — prompt à parte, depois).
- **Sem órfão** — cada prompt nucleia unidade com arquivo apontando; desvio = sinal
  de granularidade errada, reportar.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Escala do molde (0058) + regra de agregadores (0059) aos 4 crates L1 pequenos: `lente_filtro`, `lente_estrutura`, `lente_investiga`, `lente_resolve`. Por crate, unidades com interface nucleadas (prompt real em `prompts/<unidade>.md` com snapshot-semente; cabeçalho `//! Crystalline Lineage / @prompt / @prompt-hash / @layer L1 / @updated`); agregadores só-`pub mod` em `[excluded_files]`. Atenção: `estrutura` (recebeu `EstruturaModulos`/`DependenciaModulo` no 0056 + `Ciclo`/`OrdemDsm`), `resolve` (`aplicar_distintos`/`ErroResolve`), `investiga` (`investigar`; E2 quarentenada — só o exposto), `filtro` (`filtrar_stdlib`). Fluxo: `--update-snapshot` → `--fix-hashes`. Linter: os 4 crates **V1/V5/V6/V7 = 0**; V1 do projeto cai de 31; V3 = 0, V12 = 1, demais 0; V2 reportado se algum arquivo de interface estiver sem `#[cfg(test)]` (não corrigido — muda código). **Preserva comportamento**: suíte 273 + 28; `prompt/` intocado. Falta: L2 (`catalogo`/`cli`), L3 (`infra`), L4 (`wiring`/`app`, `@layer L4`, V12 do `ErroLente` declarado) + o V2 do `consulta.rs`. | `00_nucleo/prompts/*.md` (novos, dos 4 crates), `01_core/{filtro,estrutura,investiga,resolve}/src/*.rs` (cabeçalhos), `crystalline.toml` (`[excluded_files]`), `00_nucleo/lessons/0060-...` |
