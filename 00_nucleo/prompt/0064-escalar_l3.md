# Prompt: migração de convenção — escalar ao L3 (`lente_infra`)

**Camada**: transversal (`prompts/` + cabeçalhos do `lente_infra`) — no `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0064-escalar_l3.md`)
**Pré-requisito**: 0063 (L2 migrado; regra dos internos refinada; `const` não entra
no snapshot).
**Objetivo**: aplicar o molde ao `lente_infra` (`@layer L3`) — o crate de infra com
**vários arquivos** (adaptadores de I/O: `fork`/`workspace`/`diff`/`metadata`/
`traducao`/`dto`/`invocacao`/…). Cada arquivo com interface → prompt + cabeçalho;
agregador puro → `[excluded_files]`; lógica ativa interna → fica no walk. **Confirma
os arquivos do `lente_infra` em V1/V5/V6/V7 = 0 e V3 = 0**; o V1 do projeto cai de
20. Preserva comportamento.

---

## O molde + a regra refinada (seguir igual)

- **Por arquivo com interface pública** (inclui `pub(crate)` — o snapshot os captura)
  → prompt de nucleação real (Propósito · Comportamento · Restrições · Critérios ·
  `## Interface Snapshot` semente vazia · Histórico).
- **Cabeçalho**: `//! Crystalline Lineage / @prompt prompts/<unidade> / @prompt-hash
  / @layer L3 / @updated` — **`@layer L3`**. (Se o arquivo tem doc próprio sem
  `//! Lineage:`, **prepend** as linhas do cabeçalho, como no `cli/args`/`saida`.)
- **Snapshot GERADO**, ordem **`--update-snapshot` → `--fix-hashes`**.
- **Regra dos internos (refinada)**: `[excluded_files]` suspende **TODAS** as
  checagens. Lógica ativa / com imports significativos → **fica no walk** (prompt
  mínimo). Exclusão **só** p/ teste, quarentena, agregador puro só-`pub mod`.
- **`prompt/` intocado**.

---

## O que muda no L3 (em relação ao L1/L2)

- **V4 (I/O no núcleo) e V13 (estado mutável) NÃO se aplicam** — são checks **L1**.
  O L3 **faz I/O legítimo** (git, filesystem, processos): isso é **correto**, não é
  achado. Não estranhar I/O nos adaptadores.
- **V3 é o critério ativo**: o L3 **pode** importar o L1 (depende do núcleo) e
  **não pode** importar o L4. Confirmar **V3 = 0** (nenhum import de `lente_wiring`/
  `lente_app`).
- **`const`** (se houver no L3) **não entra no snapshot** (achado do 0063) — descrever
  no prompt; o snapshot rastreia `fn`/`struct`/`enum`.
- **V2 não se aplica** (é L1) — não esperar V2 no L3.

---

## O que fazer

1. **Enumerar as unidades** do `lente_infra` (`03_infra/src/` ou caminho equivalente):
   arquivos com interface → nucleação (`@layer L3`); agregador puro só-`pub mod` →
   `[excluded_files]`; lógica ativa interna (`pub(crate)` com corpo/imports) → prompt
   mínimo (fica no walk). Registrar a lista real.
2. Por **unidade**: prompt em `prompts/<unidade>.md` (molde, semente vazia) + cabeçalho
   `@layer L3`. Ler código + prompt de trabalho como referência, sem mover.
3. **Fluxo**: `crystalline-lint --update-snapshot .` → `crystalline-lint --fix-hashes .`.
4. **Linter completo** e confirmar:
   - Arquivos do `lente_infra`: **V1/V5/V6/V7 = 0**.
   - **V3 = 0** (L3 não importa L4; importar L1 é permitido).
   - **V12 = 1** (`ErroLente`, intencional), **demais = 0**; **V1 do projeto cai** de 20.
5. **Verificar**: suíte **273 + 28**; `cargo build`.

---

## Atenção por arquivo (capturar fiel)

- **`fork`/`workspace`/`diff`/`invocacao`**: adaptadores de I/O (git/fs/processo) —
  o prompt descreve a operação e o **erro** que cada um produz (são os erros que o
  `ErroLente` do L4 agrega — `Fork`/`Adaptador`/`Workspace`/`Diff`). Nuclear os
  **tipos de erro L3** aqui (eles **moram** no L3).
- **`dto`**: objetos de transferência (fronteira L3) — descrever o que carregam.
- **`metadata`/`traducao`**: metadados e tradução de mensagens — interface pública.
- Se algum arquivo for **só agregador** (`pub mod`) → `[excluded_files]`; se for
  **interno com lógica/imports** → prompt mínimo (fica no walk).

---

## O que NÃO fazer

- Mover/renomear o `prompt/`.
- Stub ou cópia — prompts **reais**.
- Escrever o snapshot à mão — **gerar**.
- Mudar código.
- **Tratar I/O no L3 como violação** — é legítimo (V4/V13 são L1).
- Excluir arquivo de **lógica ativa**/com imports — fica no walk.
- Migrar L4 — este prompt é **só o L3**.

---

## Critérios de Verificação

```
Dado as unidades do lente_infra
Então cada arquivo com interface tem prompt em prompts/ (snapshot gerado) +
cabeçalho //! Crystalline Lineage @layer L3; agregador puro/teste em [excluded_files];
lógica ativa interna no walk (prompt mínimo)

Dado o fluxo --update-snapshot → --fix-hashes
Então snapshots reais e hashes finais

Dado o linter completo
Então os arquivos do lente_infra: V1/V5/V6/V7 = 0; V3 = 0 (sem L4); V12 = 1;
demais 0; o V1 do projeto caiu de 20

Dado a suíte
Então 273 + 28 — comportamento idêntico

Dado o prompt/
Então intocado
```

---

## Resultado esperado

- A lista real de unidades do `lente_infra` + os prompts (`@layer L3`, snapshot
  gerado); agregadores em `[excluded_files]`; internos de lógica no walk.
- O linter: `lente_infra` limpo (V1/V5/V6/V7 = 0), **V3 = 0** (sem L4), V12 = 1,
  demais 0; **V1 do projeto reduzido** (de 20).
- A suíte **273 + 28**.
- **Laudo** em `00_nucleo/lessons/0064-…`: as unidades, quaisquer internos que
  ficaram no walk (e por quê), a confirmação de V3 = 0, e o que falta (só L4
  `wiring`/`app` + o V2 do `consulta`).

---

## Cuidados

- **`@layer L3`** (não L1/L2).
- **I/O é legítimo no L3** — não confundir com violação; V4/V13 só valem no L1.
- **V3 = 0 é o critério ativo** — o L3 não pode importar o L4; confirmar.
- **`const` não no snapshot** — descrever no prompt (achado do 0063).
- **Lógica ativa fica no walk** (regra refinada do 0062); exclusão só p/ teste/
  quarentena/agregador puro.
- **Os tipos de erro do L3 moram no L3** — nuclear aqui; o `ErroLente` (L4) só os
  agrega (isso é o L4, próximo prompt).
- **Comportamento idêntico** — só `//!` + prompts; suíte 273 + 28; `prompt/` intocado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Escala do molde ao L3 (`lente_infra`, `@layer L3`): cada arquivo com interface (`fork`/`workspace`/`diff`/`metadata`/`traducao`/`dto`/`invocacao`/…) nucleado — prompt real em `prompts/<unidade>.md` (semente vazia) + cabeçalho `//! Crystalline Lineage @layer L3` (replace ou prepend conforme o doc existente) + snapshot gerado (`--update-snapshot`→`--fix-hashes`). Agregador puro só-`pub mod` → `[excluded_files]`; lógica ativa interna (com imports) → prompt mínimo (fica no walk, regra refinada do 0062). **No L3**: V4/V13 **não se aplicam** (são checks L1; o L3 faz I/O legítimo — git/fs/processo); **V3 é o critério ativo** (L3 pode importar L1, não L4 — confirmar V3 = 0); `const` **não** entra no snapshot (achado do 0063 — descrever no prompt); V2 não se aplica (L1). Os tipos de erro do L3 (`Fork`/`Adaptador`/`Workspace`/`Diff`, que o `ErroLente` do L4 agrega) **moram no L3** — nucleados aqui. Linter: `lente_infra` **V1/V5/V6/V7 = 0**, **V3 = 0**, V12 = 1, demais 0; V1 do projeto cai de 20. **Preserva comportamento**: suíte 273 + 28; `prompt/` intocado. Falta só L4 (`wiring`/`app`, `@layer L4`, V12 do `ErroLente` declarado) + o teste do `consulta`. | `00_nucleo/prompts/*.md` (novos do L3), `03_infra/.../src/*.rs` (cabeçalhos), `crystalline.toml` (`[excluded_files]`, se houver agregador), `00_nucleo/lessons/0064-...` |
