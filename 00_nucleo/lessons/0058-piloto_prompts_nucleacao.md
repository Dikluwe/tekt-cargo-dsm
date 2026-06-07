# Laudo de Execução — Prompt 0058 (piloto da migração de convenção de linhagem)

**Camada**: transversal (cria `prompts/` + migra um crate)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0058-piloto_prompts_nucleacao.md`
**Estado**: `EXECUTADO` — `00_nucleo/prompts/` criada; `lente_ranking` migrado;
pilar limpo no linter (**V1/V5/V6/V7 = 0** no crate). `prompt/` (singular)
**intocado**. Suíte **273 + 28** (só cabeçalho + prompt novo). **Molde travado.**

---

## A resposta em uma sentença

Num crate L1 de um arquivo (`lente_ranking`), o molde da migração foi fixado e
provado contra o linter: prompt de nucleação em `prompts/` + cabeçalho
`//! Crystalline Lineage` + Interface Snapshot **gerado pelo `--update-snapshot`**
(não à mão) + hash via `--fix-hashes` → V1/V5/V6/V7 = 0 no crate, sem tocar código.

---

## O crate-piloto: `lente_ranking`

O **menor L1 auto-contido**: um arquivo (`01_core/ranking/src/lib.rs`, 215 linhas
com testes), interface pública mínima (`ItemRanking` + `rankear`). Ideal para
travar o molde sem ruído.

---

## O MOLDE travado (para escalar aos demais crates)

### 1. Pasta e granularidade

- **`00_nucleo/prompts/`** (plural) — pasta nova, só com prompts de **nucleação**
  (descrevem código existente). O **`prompt/`** (singular, trabalho + verificação)
  fica **intocado e fora do alcance do linter** (que só varre `prompts/`).
- **Granularidade = por arquivo/unidade.** Um prompt de nucleação por `.rs` com
  interface pública (o `@prompt` é por-arquivo, e o Interface Snapshot do **V6** é
  a interface pública **daquele arquivo**). Ignorar `mod.rs` trivial e fixtures.
  Crate de um arquivo → um prompt; crate de N arquivos → N prompts.

### 2. O cabeçalho (substitui o `//! Lineage:`)

```
//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/<unidade>.md
//! @prompt-hash <8 hex — via --fix-hashes>
//! @layer L1
//! @updated AAAA-MM-DD
```

(As linhas antigas `//! Spec:`/`//! ADRs:`/`//! Lições:`/`//! Camada:` podem
**ficar abaixo** — o linter lê o bloco `@`-tags; o resto é doc normal.)

### 3. O prompt de nucleação (forma)

Seções: título · metadados (camada, unidade, origem de trabalho como referência)
· **Propósito** · **Comportamento e invariantes** · **Restrições (L1 puro)** ·
**Critérios de Verificação** (`Dado/Quando/Então`) · **`## Interface Snapshot`** ·
**`## Histórico de Revisões`**. Descreve o que a unidade faz a ponto de o código
ser materialização fiel — **não** é cópia do prompt de trabalho nem stub.

### 4. O Interface Snapshot (V6) — **gerado, não escrito à mão**

Formato exato (confirmado na fonte do linter):

```
## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[…],"types":[…],"reexports":[…]} -->
```

O JSON é a interface pública real. **Não se escreve à mão** — o linter o gera. O
do `ranking` ficou:

```json
{"functions":[{"name":"rankear","params":["&Grafo","usize"],"return_type":"Vec<ItemRanking>"}],
 "types":[{"name":"ItemRanking","kind":"struct","members":["path","impacto","classificacao"]}],
 "reexports":[]}
```

---

## O fluxo (a ordem que funciona — descoberta importante)

O `--update-snapshot` **só age sobre arquivos com violação V6**, e o V6 **só
dispara quando já existe um snapshot e ele está stale** (`prompt_stale.rs`: sem
snapshot → `None` → sem violação). Logo, **não dá para semear o primeiro snapshot
com o snapshot vazio**. A sequência que funciona:

1. **Escrever o prompt** com um **snapshot-semente deliberadamente vazio mas
   válido**: `<!-- crystalline-snapshot: {"functions":[],"types":[],"reexports":[]} -->`.
2. **Migrar o cabeçalho** do `.rs` (`@prompt` → o prompt novo; `@prompt-hash`
   placeholder `00000000`).
3. **`crystalline-lint --update-snapshot .`** — o snapshot vazio ≠ interface real
   → V6 dispara → o linter reescreve o snapshot com a interface verdadeira (e
   anota `Hash do Código:` no topo do prompt). "✅ 0 stale warnings".
4. **`crystalline-lint --fix-hashes .`** — preenche o `@prompt-hash` (`79471c54`)
   com o hash **final** do prompt (depois do snapshot). "✅ 0 drift warnings".

(`--update-snapshot` antes de `--fix-hashes`: o snapshot muda o prompt → muda o
hash; o hash tem de ser o último.)

---

## Resultado do linter

| Check | Crate-piloto (`01_core/ranking`) | Projeto |
|---|---|---|
| **V1** (cabeçalho) | **0** | 41 (resto não migrado — esperado) |
| **V5** (hash/drift) | **0** | 0 |
| **V6** (snapshot stale) | **0** | 0 |
| **V7** (órfão) | **0** | **0** — `prompts/` existe agora, scan não aborta; `ranking.md` referenciado |

**ZERO violações** nos arquivos do `lente_ranking`. O `prompt/` (56 arquivos)
**intocado**; `prompts/` só com `ranking.md`.

Nota: criar `00_nucleo/prompts/` (antes inexistente) **destravou o V7** — o scan
de prompts não aborta mais (o que forçava rodar sem v5/v6/v7 nos laudos 0049–0057).
Agora o linter roda **completo**.

---

## Verificação de comportamento

| Item | Resultado |
|------|-----------|
| `cargo build --workspace` | passa |
| `cargo test --workspace` | **273 + 28, 0 falhas** — só `//!` + prompt novo, zero código |
| `prompt/` (singular) | **intocado** (56 arquivos) |
| `prompts/` (novo) | só `ranking.md` |

---

## O que falta escalar (próximos prompts)

- Replicar o molde nos demais crates, **um arquivo/unidade por vez**:
  `lente_core` (vários: `entities/grafo`, `domain/{raio,uniao,mapeamento,resultado_diff,consulta}`),
  `lente_filtro`, `lente_estrutura`, `lente_investiga`, `lente_resolve` (L1);
  depois L2/L3/L4 (com `@layer` correto).
- Cada um: prompt de nucleação + Interface Snapshot (gerado) + cabeçalho + fix-hashes.
- **Decisão pendente**: o `ErroLente` (L4, V12=1) e os tipos do `04_wiring` — quando
  migrar o L4, decidir o `@layer` e se o V12 intencional se declara aqui também.
- À medida que os crates migram, o **V1 cai** (de 41 rumo a 0 nos migrados).

---

## Cuidados confirmados

- **`prompt/` não foi renomeado nem movido** — só lido como referência.
- **Snapshot derivado do código** (via `--update-snapshot`), não inventado.
- **Sem órfão por construção** — `ranking.md` nucleia uma unidade com arquivo
  apontando; V7=0.
- **Comportamento idêntico** — suíte 273 + 28.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Piloto da migração de convenção de linhagem (decisão: prompts de nucleação por arquivo em pasta nova `prompts/`; `prompt/` intocado). Crate-piloto `lente_ranking` (um arquivo). Criada `00_nucleo/prompts/ranking.md` (prompt de nucleação real, com Propósito/Comportamento/Restrições/Critérios/Snapshot/Histórico). Cabeçalho do `01_core/ranking/src/lib.rs` migrado para `//! Crystalline Lineage / @prompt prompts/ranking.md / @prompt-hash / @layer L1 / @updated`. **Molde travado**: granularidade por-arquivo; Interface Snapshot **gerado pelo `--update-snapshot`** (não à mão) — fluxo: snapshot-semente vazio→`--update-snapshot` (reescreve com a interface real)→`--fix-hashes` (hash final). Linter completo (agora sem abortar — `prompts/` existe): crate-piloto **V1/V5/V6/V7 = 0**; resto inalterado (V1=41). **Preserva comportamento**: suíte 273 + 28 (só cabeçalho + prompt novo). `prompt/` (56 arquivos) intocado. Próximo: escalar arquivo-a-arquivo aos demais crates. | `00_nucleo/prompts/ranking.md` (novo), `01_core/ranking/src/lib.rs` (cabeçalho), `00_nucleo/lessons/0058-piloto_prompts_nucleacao.md` |
