# Prompt: tratar o interno de lógica do L1 (`vizinhanca.rs`) — restaurar a pureza

**Camada**: transversal (`crystalline.toml` + `prompts/` + cabeçalho de um arquivo)
— no `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0062-tratar_interno_l1.md`)
**Pré-requisito**: 0061 (medido: `[excluded_files]` é exclusão **total**; V4/V13 são
checks **L1**; `vizinhanca.rs` é **puro**).
**Decisão do autor**: **prompt mínimo no interno** (mantém o arquivo checado).
**Objetivo**: `investiga/src/vizinhanca.rs` (interno de lógica L1, `pub(crate)`, puro)
**sai** do `[excluded_files]` e ganha um **prompt de nucleação mínimo** + cabeçalho;
com isso **volta ao walk** e **V4/V13 voltam a checá-lo** (passa — é puro). Fixa a
**regra dos internos**. Preserva comportamento.

---

## O que fazer

1. **Confirmar o escopo**: varrer o `[excluded_files]` e confirmar que o
   `vizinhanca.rs` é o **único arquivo de lógica ATIVA do L1** lá dentro — os demais
   são teste (`filtro/tests/e2e_lente_core.rs`), quarentena que sai (`investiga/
   fontes.rs`, E2) e agregadores só-`pub mod` (`lib.rs`/`mod.rs` do 0059). **Se
   houver outro interno de lógica L1 ativo escondido na lista, listá-lo** (mesmo
   tratamento).
2. **Tirar** `01_core/investiga/src/vizinhanca.rs` do `[excluded_files]` no
   `crystalline.toml`.
3. **Criar** `00_nucleo/prompts/vizinhanca.md` — prompt **mínimo e real**: Propósito
   (helper interno da Estratégia 1 do `investigar` — compara conjuntos de arestas →
   `Veredito`), Comportamento, Restrições (L1 puro), Critérios (`Dado/Quando/Então`),
   `## Interface Snapshot` (semente vazia válida — **nada é `pub` cross-crate**),
   Histórico. Ler o código + o prompt de trabalho como referência, **sem mover**.
4. **Migrar o cabeçalho**: `//! Crystalline Lineage / @prompt prompts/vizinhanca.md
   / @prompt-hash 00000000 / @layer L1 / @updated`.
5. **Fluxo** (a ordem travada): `crystalline-lint --update-snapshot .` → `crystalline-lint
   --fix-hashes .`. (O snapshot gerado será **provavelmente vazio** — nada escapa do
   crate; o `--update-snapshot` produz o correto seja qual for.)
6. **Linter completo** e confirmar:
   - `vizinhanca.rs` **no walk**: **V4 = 0 e V13 = 0** sobre ele (checado agora, e
     **passa** por ser puro) — a guarda restaurada.
   - `vizinhanca.rs`: **V1/V5/V6/V7 = 0** (cabeçalho + prompt + snapshot + não-órfão).
   - `[excluded_files]` agora **só** teste + quarentena + agregadores.
   - **V3 = 0, V12 = 1, demais = 0**; **V1 do projeto inalterado** (o arquivo era
     excluído — não contava — e agora tem cabeçalho válido — não viola).
7. **Verificar**: suíte **273 + 28** (só um comentário `//!` + um prompt + uma linha
   de config movida — a **lógica** não muda); `cargo build`; `prompt/` intocado.

---

## A regra dos internos (o que este prompt fixa, para escalar)

- **Interno de lógica ATIVA do L1** (`pub(crate)` com corpo, que **fica**) → **prompt
  mínimo + cabeçalho** (fica no walk → **pureza V4/V13 checada**). Snapshot vazio é
  esperado (sem interface cross-crate).
- **`[excluded_files]` reservado a**: testes (não são unidade de arquitetura),
  quarentena que será removida, agregadores só-`pub mod` (sem corpo — V4/V13 não
  teriam o que checar).
- **Interno do L3+** (`lente_infra` etc.): `[excluded_files]` **seguro** — V4/V13 não
  se aplicam a L3 (I/O legítimo), então não há guarda a perder.

---

## O que NÃO fazer

- Mover/renomear o `prompt/`.
- Stub ou cópia — prompt **real**, mesmo sendo mínimo.
- Escrever o snapshot à mão — **gerar**.
- Mudar a **lógica** do `vizinhanca.rs` — só o cabeçalho.
- Dar prompt à `fontes.rs` (quarentena, sai), aos testes ou aos agregadores — esses
  **continuam** em `[excluded_files]`.

---

## Critérios de Verificação

```
Dado o vizinhanca.rs fora do [excluded_files], com prompt mínimo + cabeçalho
Então está no walk e V4 = 0, V13 = 0 sobre ele (checado e puro), e V1/V5/V6/V7 = 0

Dado o [excluded_files]
Então só restam teste + quarentena + agregadores (nenhum interno de lógica L1 ativo)

Dado o projeto
Então V1 inalterado, V3 = 0, V12 = 1, demais 0; suíte 273 + 28 (lógica intocada)

Dado o prompt/
Então intocado
```

---

## Resultado esperado

- `vizinhanca.rs` fora do `[excluded_files]`, com `prompts/vizinhanca.md` (snapshot
  gerado, provável vazio) + cabeçalho `@layer L1`.
- O linter: `vizinhanca.rs` **checado por pureza** (V4/V13 = 0, passa) e limpo de
  linhagem (V1/V5/V6/V7 = 0); `[excluded_files]` enxuto; projeto V3 = 0, V12 = 1.
- A suíte **273 + 28**.
- **Laudo** em `00_nucleo/lessons/0062-…`: a confirmação de que o `vizinhanca.rs` era
  o único interno de lógica L1 excluído, o resultado do linter (guarda restaurada),
  e a regra dos internos fixada para o L2/L3/L4.

---

## Cuidados

- **A guarda é o ponto**: o objetivo não é "tirar um V1" — é que V4/V13 **voltem a
  ver** o arquivo. Confirmar isso explicitamente (não basta V1 = 0).
- **Snapshot vazio é correto** aqui (sem interface cross-crate) — não é erro; é o que
  o `--update-snapshot` deve produzir.
- **Lógica intocada** — a suíte 273 + 28 é a prova; só comentário + prompt + config.
- **`prompt/` intocado**; **prova/ordem** do fluxo como nos anteriores.
- Se a varredura do passo 1 achar **outro** interno de lógica L1 ativo excluído,
  **reportar e aplicar o mesmo** (não deixar guarda furada em silêncio).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Decisão (informada pelo 0061: `[excluded_files]` é total, V4/V13 são L1, `vizinhanca.rs` puro): tratar o interno de lógica do L1 com **prompt mínimo** em vez de exclusão. `investiga/src/vizinhanca.rs` retirado do `[excluded_files]`; `prompts/vizinhanca.md` criado (mínimo, real — helper da Estratégia 1, compara conjuntos de arestas → `Veredito`; snapshot semente vazia, pois nada é `pub` cross-crate); cabeçalho migrado (`//! Crystalline Lineage / @prompt / @prompt-hash / @layer L1 / @updated`); fluxo `--update-snapshot`→`--fix-hashes`. Resultado: `vizinhanca.rs` volta ao walk → **V4/V13 o checam e passam** (guarda de pureza restaurada); V1/V5/V6/V7 = 0; `[excluded_files]` reduzido a teste + quarentena + agregadores. **Regra dos internos fixada**: lógica ativa L1 → prompt mínimo (checado); `[excluded_files]` só para teste/quarentena/agregador puro; interno L3+ → exclusão segura (V4/V13 não se aplicam). **Preserva comportamento** (só comentário + prompt + config; lógica intocada): suíte 273 + 28; `prompt/` intocado. | `00_nucleo/prompts/vizinhanca.md` (novo), `01_core/investiga/src/vizinhanca.rs` (cabeçalho), `crystalline.toml` (`[excluded_files]`), `00_nucleo/lessons/0062-...` |
