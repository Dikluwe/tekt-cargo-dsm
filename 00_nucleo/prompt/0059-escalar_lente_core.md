# Prompt: migração de convenção — escalar o molde para o `lente_core`

**Camada**: transversal (`prompts/` + cabeçalhos do `lente_core`) — no `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0059-escalar_lente_core.md`)
**Pré-requisito**: 0058 (molde travado no `lente_ranking`).
**Decisão fechada**: prompts de **nucleação** por **arquivo/unidade**, em `prompts/`
nova; `prompt/` **intocado** (ler, não mover).
**Objetivo**: aplicar o **molde do 0058** a **todo o `lente_core`** (`01_core/core`)
— o maior crate L1 e a fundação. Cada unidade ganha um prompt de nucleação +
cabeçalho + snapshot gerado + hash. **Confirma os arquivos do `lente_core` em
V1/V5/V6/V7 = 0.** Preserva comportamento.

---

## O molde (do 0058 — seguir igual)

- **Granularidade por arquivo**: um prompt de nucleação por `.rs` com interface
  pública; ignorar `mod.rs` trivial e fixtures. Re-exports contam como interface
  (entram no snapshot).
- **Cabeçalho**: `//! Crystalline Lineage / @prompt 00_nucleo/prompts/<unidade>.md
  / @prompt-hash <via --fix-hashes> / @layer L1 / @updated AAAA-MM-DD`. As linhas
  antigas (`//! Spec:` etc.) podem ficar abaixo.
- **Prompt de nucleação (forma)**: título · metadados · **Propósito** ·
  **Comportamento e invariantes** · **Restrições (L1 puro)** · **Critérios de
  Verificação** (`Dado/Quando/Então`) · **`## Interface Snapshot`** (semente vazia
  válida) · **`## Histórico de Revisões`**. **Real e fiel ao código** — não stub,
  não cópia do prompt de trabalho.
- **Snapshot é GERADO, não escrito à mão.** Semente vazia válida
  (`<!-- crystalline-snapshot: {"functions":[],"types":[],"reexports":[]} -->`) →
  o fluxo gera o real.
- **Ordem do fluxo**: `--update-snapshot` **antes** de `--fix-hashes` (o snapshot
  muda o prompt → o hash é o último).

---

## O que fazer

1. **Enumerar as unidades** do `lente_core` (`01_core/core/src/`): cada `.rs` com
   interface pública. Conhecidas (confirmar na fonte e completar): `entities/grafo`,
   `domain/raio`, `domain/uniao`, `domain/mapeamento`, `domain/resultado_diff`,
   `domain/consulta`; e `lib.rs`/`mod.rs` **se** expuserem interface (re-exports
   contam). Registrar a lista real.
2. Para **cada unidade**, seguir o molde:
   a. Escrever `00_nucleo/prompts/<unidade>.md` (prompt de nucleação real, com a
      semente de snapshot vazia válida). Ler o código + o prompt de trabalho
      correspondente em `prompt/` como referência, **sem mover**.
   b. Migrar o cabeçalho do `.rs` para `//! Crystalline Lineage / @prompt
      prompts/<unidade>.md / @prompt-hash 00000000 / @layer L1 / @updated`.
3. **Rodar o fluxo** (a ordem travada):
   - `crystalline-lint --update-snapshot .` — gera os snapshots reais das sementes.
   - `crystalline-lint --fix-hashes .` — preenche os `@prompt-hash` finais.
4. **Rodar o linter completo** e confirmar:
   - Os arquivos do `lente_core`: **V1/V5/V6/V7 = 0**.
   - O **V1 do projeto cai** pelos arquivos do `lente_core` migrados (de 41 para
     41 − N).
   - **V3 = 0, V12 = 1, V4/V8/V9/V13/V14 = 0** — refactor preservado.
5. **Verificar**: suíte **273 + 28** (só cabeçalhos `//!` + prompts novos — código
   intocado); `cargo build` passa.

---

## O que NÃO fazer

- Renomear/mover o `prompt/` — intocado, só leitura.
- Stubs ou cópias — prompts de **nucleação reais**.
- Escrever o snapshot à mão — **gerar** (`--update-snapshot`).
- Mudar código.
- Migrar **outros** crates — este prompt é **só o `lente_core`**.

---

## Critérios de Verificação

```
Dado as unidades do lente_core
Então cada uma tem um prompt de nucleação em prompts/ (com snapshot gerado) e o
cabeçalho //! Crystalline Lineage apontando para ele

Dado o fluxo --update-snapshot → --fix-hashes
Então os snapshots são os reais e os @prompt-hash os finais

Dado o linter completo
Então os arquivos do lente_core: V1/V5/V6/V7 = 0; o V1 do projeto caiu por eles;
V3 = 0, V12 = 1, demais 0

Dado a suíte
Então 273 + 28 — comportamento idêntico (só comentários + prompts novos)

Dado o prompt/
Então intocado (não renomeado, não movido)
```

---

## Resultado esperado

- A lista real das unidades do `lente_core` + os ~N prompts de nucleação em
  `prompts/` (com Interface Snapshot gerado).
- Os cabeçalhos do `lente_core` migrados; `--update-snapshot` + `--fix-hashes`
  aplicados na ordem.
- O linter: `lente_core` limpo (V1/V5/V6/V7 = 0); o **V1 do projeto reduzido**
  (41 − N); V3 = 0, V12 = 1, demais 0.
- A suíte **273 + 28**.
- **Laudo** em `00_nucleo/lessons/0059-…`: a lista de unidades, os prompts criados,
  o V1 restante e os crates que faltam escalar.

---

## Cuidados

- **`prompt/` intocado** — só leitura.
- **Prompts reais e fiéis** — o `lente_core` é a fundação; a qualidade do que cada
  prompt descreve importa mais aqui (é a interface mais rica e mais referenciada).
- **Snapshot gerado, ordem `update`→`fix`** (do 0058).
- **Comportamento idêntico** — só `//!` + prompts; a suíte 273 + 28 é a prova.
- **Re-exports contam como interface** — `lib.rs`/`mod.rs` com `pub use` entram no
  snapshot; `mod.rs` trivial (só `mod x;`) não precisa de prompt.
- **Sem órfão por construção** — cada prompt nucleia uma unidade com arquivo
  apontando; se um snapshot não casar ou um prompt ficar sem arquivo, é sinal de
  granularidade/derivação errada — **reportar**.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Escala do molde do 0058 para o `lente_core` (`01_core/core`), o maior crate L1 e a fundação. Para cada unidade com interface pública (`entities/grafo`, `domain/{raio,uniao,mapeamento,resultado_diff,consulta}`, e `lib.rs`/`mod.rs` se expuserem re-exports — lista real confirmada na fonte): prompt de nucleação real em `00_nucleo/prompts/<unidade>.md` (Propósito/Comportamento/Restrições L1/Critérios/Snapshot-semente/Histórico, lendo código + prompt de trabalho como referência sem mover); cabeçalho do `.rs` migrado para `//! Crystalline Lineage / @prompt prompts/<unidade> / @prompt-hash / @layer L1 / @updated`. Fluxo na ordem travada: `--update-snapshot` (gera snapshots reais das sementes vazias) → `--fix-hashes` (hashes finais). Linter completo: arquivos do `lente_core` **V1/V5/V6/V7 = 0**; V1 do projeto cai (41 − N); V3 = 0, V12 = 1, demais 0. **Preserva comportamento**: só `//!` + prompts novos; suíte 273 + 28; `prompt/` intocado. Próximo: escalar aos crates L1 pequenos (investiga/resolve/filtro/estrutura), depois L2/L3/L4. | `00_nucleo/prompts/*.md` (novos, do `lente_core`), `01_core/core/src/**/*.rs` (cabeçalhos), `00_nucleo/lessons/0059-...` |
