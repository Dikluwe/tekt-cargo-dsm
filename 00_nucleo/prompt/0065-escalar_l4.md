# Prompt: migração de convenção — escalar ao L4 (`wiring` + `app`) — último passo

**Camada**: transversal (`prompts/` + cabeçalhos do `lente_wiring` e `lente_app`)
— no `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0065-escalar_l4.md`)
**Pré-requisito**: 0064 (L1/L2/L3 migrados; V1 do projeto = 12; regra dos internos
refinada; `pub(crate)` no snapshot, `const` não).
**Objetivo**: aplicar o molde ao **L4** — `lente_wiring` (a fiação) e `lente_app`
(o binário) — `@layer L4`. Com isso a **migração de cabeçalhos fecha**: **V1 do
projeto 12 → 0**. O `ErroLente` (agrega os 4 erros L3) **mora no L4 por desígnio**:
o prompt documenta, e **V12 = 1 fica como warning intencional aceito**. Preserva
comportamento.

---

## O molde + a regra refinada (seguir igual)

- **Por arquivo com interface OU lógica ativa** → prompt de nucleação real (Propósito
  · Comportamento · Restrições · Critérios · `## Interface Snapshot` semente vazia ·
  Histórico). Inclui o `main.rs` (ponto de entrada — lógica ativa com imports, **fica
  no walk**; seu snapshot pode ser mínimo, pois `main` não é `pub`).
- **Cabeçalho**: `//! Crystalline Lineage / @prompt prompts/<unidade> / @prompt-hash
  / @layer L4 / @updated` — **`@layer L4`**. (Replace na linha `//! Lineage:` ou
  prepend se o doc não a tiver.)
- **Snapshot GERADO**, ordem **`--update-snapshot` → `--fix-hashes`**.
- **Regra dos internos (refinada)**: lógica ativa / com imports → **no walk** (prompt);
  `[excluded_files]` **só** p/ teste, quarentena, agregador puro só-`pub mod`.
- **`prompt/` intocado**.

---

## O que muda no L4 (em relação às camadas abaixo)

- **V3 = 0 esperado por construção**: o L4 é o **topo** — pode importar L1/L2/L3
  (é a composição; depende de tudo). Não há camada acima a proibir. Confirmar V3 = 0
  (não deve haver import proibido saindo do L4).
- **V12 = 1 é o `ErroLente`, intencional**: o `ErroLente` agrega os 4 erros do L3
  (`ErroFork`/`ErroWorkspace`/`ErroDiff`/`ErroMetadata`/`ErroAdaptador`) — um **erro
  de composição** que legitimamente mora no L4. O prompt da unidade que o declara
  **documenta isso** (por que está no L4, o que agrega). **V12 = 1 permanece** — é um
  warning **aceito**, não um defeito. (Não mudar config; se no futuro o linter ganhar
  exceção por-declaração, marca-se lá — fora do escopo aqui.)
- **V4/V13 não se aplicam** (L1); **V2 não se aplica** (L1).

---

## O que fazer

1. **Enumerar as unidades** de `lente_wiring` e `lente_app` (`04_wiring/.../src/`):
   arquivos com interface/lógica → nucleação (`@layer L4`); agregador puro só-`pub
   mod` → `[excluded_files]`. Esperado: `wiring` (a fiação + `ErroLente`), `app/main`
   (dispatch), `app/erro` (`traduzir(ErroLente)`). Registrar a lista real.
2. Por **unidade**: prompt em `prompts/<unidade>.md` (molde, semente vazia) + cabeçalho
   `@layer L4`. Ler código + prompt de trabalho como referência, sem mover. **Na
   unidade do `ErroLente`**: documentar a residência intencional no L4.
3. **Fluxo**: `crystalline-lint --update-snapshot .` → `crystalline-lint --fix-hashes .`.
4. **Linter completo** e confirmar:
   - Arquivos do L4: **V1/V5/V6/V7 = 0**.
   - **V1 do projeto = 0** (migração de cabeçalhos **completa**).
   - **V3 = 0**; **V12 = 1** (`ErroLente`, intencional, documentado); **demais = 0**
     (V2 = 1 segue só no `consulta`, pré-existente, fora desta migração).
5. **Verificar**: suíte **273 + 28**; `cargo build`; o **binário `lente`** ainda roda
   (smoke test rápido — `lente --help` ou equivalente; comportamento idêntico).

---

## O que NÃO fazer

- Mover/renomear o `prompt/`.
- Stub ou cópia — prompts **reais** (o do `ErroLente` explica a agregação).
- Escrever o snapshot à mão — **gerar**.
- Mudar código (o `main`/`erro` só ganham cabeçalho `//!`).
- "Consertar" o V12 do `ErroLente` — é intencional; só documentar.
- Corrigir o V2 do `consulta` — é o próximo prompt (muda código).

---

## Critérios de Verificação

```
Dado as unidades do L4 (wiring + app)
Então cada arquivo com interface/lógica tem prompt em prompts/ (snapshot gerado) +
cabeçalho //! Crystalline Lineage @layer L4; agregador puro em [excluded_files]

Dado o fluxo --update-snapshot → --fix-hashes
Então snapshots reais e hashes finais

Dado o linter completo
Então os arquivos do L4: V1/V5/V6/V7 = 0; V1 do PROJETO = 0; V3 = 0;
V12 = 1 (ErroLente, intencional, documentado); demais 0 (V2 = 1 só no consulta)

Dado a suíte e o binário
Então 273 + 28; `lente` roda — comportamento idêntico

Dado o prompt/
Então intocado
```

---

## Resultado esperado

- As unidades do L4 + os prompts (`@layer L4`, snapshot gerado); a unidade do
  `ErroLente` com a justificativa da residência no L4; agregador (se houver) em
  `[excluded_files]`.
- O linter: L4 limpo (V1/V5/V6/V7 = 0); **V1 do projeto = 0** (migração completa);
  V3 = 0; **V12 = 1** documentado e aceito; demais 0 (V2 = 1 só no `consulta`).
- A suíte **273 + 28**; o binário `lente` roda.
- **Laudo** em `00_nucleo/lessons/0065-…`: as unidades, a justificativa do `ErroLente`,
  o **V1 = 0 do projeto** (marco), e o único pendente: o V2 do `consulta` (próximo,
  muda código) — fechando a migração da convenção.

---

## Cuidados

- **`@layer L4`** (não L1/L2/L3).
- **V1 = 0 do projeto é o marco** deste prompt — confirmar que **nenhum** `.rs` ficou
  sem cabeçalho ou exclusão.
- **V12 = 1 é esperado e aceito** (`ErroLente`) — documentar, não consertar.
- **O binário deve rodar igual** — o `main` só ganhou `//!`; smoke test confirma.
- **Lógica ativa no walk** (regra refinada); exclusão só p/ teste/quarentena/agregador.
- **Comportamento idêntico** — só `//!` + prompts; suíte 273 + 28; `prompt/` intocado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Escala do molde ao L4 (`lente_wiring` + `lente_app`, `@layer L4`) — **último passo da migração de cabeçalhos**. Unidades nucleadas: `wiring` (a fiação + `ErroLente`), `app/main` (dispatch — lógica ativa, fica no walk; snapshot mínimo, `main` não é `pub`), `app/erro` (`traduzir(ErroLente)`) — prompt real em `prompts/<unidade>.md` (semente vazia) + cabeçalho `//! Crystalline Lineage @layer L4` + snapshot gerado (`--update-snapshot`→`--fix-hashes`). Agregador puro só-`pub mod` (se houver) → `[excluded_files]`. **`ErroLente`**: agrega os 4 erros do L3 (`Fork`/`Workspace`/`Diff`/`Metadata`/`Adaptador`) — erro de composição que mora no L4 por desígnio; o prompt documenta; **V12 = 1 permanece como warning intencional aceito** (sem mudar config). **V3 = 0** por construção (L4 é o topo, importa as camadas abaixo); V4/V13/V2 não se aplicam (L1). Linter: L4 **V1/V5/V6/V7 = 0**; **V1 do PROJETO = 0** (migração completa); V3 = 0; V12 = 1 documentado; demais 0; V2 = 1 segue só no `consulta` (pré-existente). **Preserva comportamento**: suíte 273 + 28; binário `lente` roda igual; `prompt/` intocado. Resta só o teste mínimo do `consulta` (próximo prompt — muda código) para zerar o V2 e encerrar. | `00_nucleo/prompts/{wiring,app-main,app-erro,...}.md` (novos do L4), `04_wiring/.../src/*.rs` (cabeçalhos), `crystalline.toml` (`[excluded_files]`, se houver agregador), `00_nucleo/lessons/0065-escalar_l4.md` |
