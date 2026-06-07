# Prompt: fechar o V2 — teste mínimo do `consulta.rs` (último item da migração)

**Camada**: L1 (`01_core/core/src/domain/consulta.rs`) + confirmação — no `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0068-fechar_v2_consulta.md`)
**Pré-requisito**: 0067 (V10 armado; a migração da convenção só com o **V2** pendente).
**Objetivo**: adicionar um `#[cfg(test)] mod tests` **mínimo e real** ao `consulta.rs`
(os 4 enums de pedido movidos do wiring no 0056), zerando o **último V2**. **Só
adiciona testes** — a lógica e a interface não mudam. Com isso a convenção Cristalina
**fecha**: V1 = 0, **V2 = 0**, V12 = 1 (`ErroLente`, intencional), demais 0, V10 armado.

---

## Contexto

O `consulta.rs` é o único arquivo com **V2** (`MissingTestFile`, L1): ele recebeu os
enums de pedido no 0056 (`FonteGrafo`/`Escopo`/`ModoUses`/`AlvoBusca`) mas, ao
contrário dos outros arquivos de `domain`, ficou **sem `#[cfg(test)]`**. É
pré-existente (já estava no 0058) e está fora do escopo da migração de cabeçalhos
(que **não** muda código) — por isso ficou para este prompt à parte.

---

## O que fazer

1. Em `01_core/core/src/domain/consulta.rs`, adicionar um `#[cfg(test)] mod tests`
   com testes **mínimos e reais** do **comportamento** dos enums — os **defaults**,
   que são um **contrato** (estabelecido no 0056):
   - `Escopo::default() == Escopo::Completo`
   - `ModoUses::default() == ModoUses::Todas`
   - se `FonteGrafo`/`AlvoBusca` tiverem `Default`/invariante, cobrir também; senão,
     um teste básico de igualdade/construção dos seus variantes.
   Os testes **travam** o contrato (não são placeholder).
2. **Só adiciona testes** — a **lógica** dos enums **não muda**; a **interface
   pública** **não muda** (um `mod tests` não é interface).
3. **Linter completo** e confirmar:
   - **V2 = 0** — o último V2 fechado.
   - **V1 = 0**, **V3 = 0**, **V12 = 1** (`ErroLente`, único warning restante),
     **V5/V6/V7 = 0** (o cabeçalho/prompt/snapshot do `consulta` **não** mudam —
     o `mod tests` não altera a interface), demais 0.
4. **Verificar**:
   - `cargo test` — a suíte **sobe** pelos novos testes (**273 → 273 + N** passed,
     N = nº de testes adicionados), **0 falhas**.
   - **Reconciliar o nº de `#[ignore]`**: os laudos até o 0065 traziam **28**; o 0067
     (só config) reportou **25**. Como nem o 0067 nem este prompt mexem no nº de
     testes ignorados, **registrar o número real** e dizer qual é o correto.
   - `cargo build`; `prompt/` intocado.

---

## Nuance (linhagem não deve se mexer)

Adicionar um `#[cfg(test)] mod tests` **não** muda a interface pública → o **snapshot**
do `consulta` (que rastreia `fn`/`struct`/`enum` públicos) **não** muda → **V6 = 0**;
o `@prompt-hash` é do `consulta.md` (que **não** muda) → **V5 = 0**. **Esperado:
nenhuma mudança na linhagem.** Se, contra o esperado, o snapshot acusar drift, rodar
`--update-snapshot` → `--fix-hashes`; mas o normal é **nada** mudar.

---

## O que NÃO fazer

- Mudar a **lógica** dos enums — **só** adicionar testes.
- Stub/placeholder — testes **reais** do contrato (os defaults).
- Tocar no `prompt/`.
- Mexer no `V12` (`ErroLente` é intencional).

---

## Critérios de Verificação

```
Dado o consulta.rs com #[cfg(test)] mod tests real (defaults dos enums)
Então V2 = 0 — o último V2 fechado

Dado o linter completo
Então V1 = 0, V2 = 0, V3 = 0, V12 = 1 (ErroLente), V5/V6/V7 = 0, demais 0
— a migração da convenção completa

Dado cargo test
Então 273 + N passed (N novos testes), 0 falhas; nº de #[ignore] registrado e
reconciliado (28 vs 25)

Dado o prompt/ e a lógica
Então intocados (só testes adicionados; comportamento idêntico)
```

---

## Resultado esperado

- O `consulta.rs` com o `#[cfg(test)] mod tests` mínimo; **V2 = 0**.
- O linter no **estado final da migração**: **V1 = 0, V2 = 0**, **V12 = 1**
  (`ErroLente`, warning intencional aceito), demais 0, **V10 armado**.
- A suíte com **+N testes**, 0 falhas; o nº de `#[ignore]` reconciliado.
- **Laudo** em `00_nucleo/lessons/0068-…`: o teste adicionado, o **V2 = 0**, o estado
  final do linter, a reconciliação do `#[ignore]`, e o **fecho** da migração da
  convenção Cristalina (resta só o V12 intencional do `ErroLente`).

---

## Cuidados

- **Só testes** — comportamento idêntico; a suíte **sobe** (esperado), não cai.
- **Testes reais do contrato dos defaults** (0056) — não placeholder.
- **A linhagem do `consulta` não se mexe** (o `mod tests` não é interface) — confirmar
  V5/V6 = 0 sem precisar do fluxo.
- **`prompt/` intocado**.
- Este é o **último item** — depois, a convenção Cristalina está **plenamente
  adotada** (V1 = 0, V2 = 0, V10 armado), com o único warning sendo o V12 intencional
  do `ErroLente`.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | **Último item da migração** — fechar o V2. Adicionado `#[cfg(test)] mod tests` mínimo e real ao `01_core/core/src/domain/consulta.rs` (único arquivo com V2 `MissingTestFile`, pré-existente desde que os enums de pedido vieram do wiring no 0056): testa o contrato dos defaults (`Escopo::default() == Completo`, `ModoUses::default() == Todas`; e `FonteGrafo`/`AlvoBusca` se tiverem default/invariante). **Só testes** — lógica e interface pública intactas; a linhagem (V5 hash / V6 snapshot) **não** se mexe (um `mod tests` não é interface). Linter final: **V2 = 0**; V1 = 0, V3 = 0, **V12 = 1** (`ErroLente`, intencional), demais 0; V10 armado (0067). `cargo test`: suíte sobe pelos N novos testes (273 → 273+N), 0 falhas; nº de `#[ignore]` reconciliado (28 nos laudos até 0065 vs 25 no 0067 — registrar o real, já que nada mexeu no nº de testes ignorados). `prompt/` intocado; comportamento idêntico. **Fecha a migração da convenção Cristalina**: cabeçalhos `//! Crystalline Lineage`/`@prompt`/`@prompt-hash`/`@layer`/`@updated` em toda a treliça (L1–L4), prompts de nucleação em `00_nucleo/prompts/`, snapshots gerados, V1/V2 = 0 — resta só o V12 intencional do `ErroLente`. | `01_core/core/src/domain/consulta.rs` (`#[cfg(test)] mod tests`), `00_nucleo/lessons/0068-fechar_v2_consulta.md` |
