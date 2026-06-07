# Laudo de Execução — Prompt 0064 (escalar ao L3 — `lente_infra`)

**Camada**: transversal (`prompts/` + cabeçalhos do `lente_infra`)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0064-escalar_l3.md`
**Estado**: `EXECUTADO` — os 8 arquivos do `lente_infra` nucleados (`@layer L3`);
**V1/V5/V6/V7 = 0**, **V3 = 0** (L3 não importa L4). V1 do projeto **20 → 12**.
V12 = 1, demais 0. Suíte **273 + 28**. `prompt/` intocado.

---

## A resposta em uma sentença

O L3 inteiro (8 arquivos do `lente_infra`, do adaptador ao cache de workspace)
entrou na convenção, com o I/O tratado como **legítimo** (V4/V13 não se aplicam) e
a direção confirmada (**V3 = 0** — depende do L1, nunca do L4).

---

## As unidades (8 — todas com interface, nenhum agregador puro)

| Unidade (prompt) | Arquivo | Visibilidade | Interface (do snapshot) |
|---|---|---|---|
| `infra` | `lib.rs` | `pub` | `extrair_grafo`/`desserializar_grafo`; `ErroAdaptador`; re-exports |
| `infra-fork` | `fork.rs` | `pub`+`pub(crate)` | `invocar_fork`/`invocar_em`; `AlvoFork`/`ErroFork` |
| `infra-workspace` | `workspace.rs` | `pub` | `enumerar_membros`/`versao_toolchain`/`chave_cache`/`extrair_grafo_cacheado`; `MembroWorkspace`/`ErroWorkspace` |
| `infra-diff` | `diff.rs` | `pub` | `ler_diff`; `ErroDiff` |
| `infra-metadata` | `metadata.rs` | `pub`+`pub(crate)` | detecção de alvo; `ErroMetadata` + DTOs de metadata |
| `infra-dto` | `dto.rs` | `pub(crate)` | `GrafoDTO`/`NoDTO`/`ArestaDTO`/`PositionDTO` |
| `infra-invocacao` | `invocacao.rs` | `pub(crate)` | `invocar` |
| `infra-traducao` | `traducao.rs` | `pub(crate)` | `traduzir` |

**Nenhum `[excluded_files]` novo**: o `lib.rs` do `lente_infra` **não** é agregador
puro (define `ErroAdaptador` + funções + re-exports), e os internos (`dto`/
`invocacao`/`traducao`/`metadata`) são **lógica ativa** → ficam no walk (prompt,
regra do 0062). Os erros do L3 (`ErroFork`/`ErroWorkspace`/`ErroDiff`/`ErroMetadata`/
`ErroAdaptador`) **moram aqui** — o `ErroLente` (L4) só os agrega (próximo prompt).

---

## O que muda no L3 (confirmado)

- **V4/V13 não dispararam** — são checks **L1**; o I/O do L3 (git/fs/rustc/processo)
  é legítimo. Nenhum "achado" de impureza, como esperado.
- **`pub(crate)` entra no snapshot** (dto/invocacao/traducao capturados) — confirma o
  achado do 0062.
- **`const` não entra no snapshot** (não há `const` significativo no L3; o do
  `catalogo` foi o caso — achado do 0063). Sem novidade aqui.
- **V2 não se aplica** (L1) — nenhum V2 novo no L3.

## V3 = 0 — a direção do L3

Confirmado por grep + linter: o `lente_infra` **não importa `lente_wiring`/
`lente_app` (L4)**. Importa o L1 (`lente_core`) — permitido (a infra materializa o
tipo de dados do núcleo). **V3 = 0** no projeto.

---

## Resultado do linter / verificação

| Item | Resultado |
|------|-----------|
| `lente_infra` (8 arquivos) | V1/V5/V6/V7 = 0 |
| **V3** | **0** (L3 sem L4) |
| Projeto | V1 **12** (era 20 — caíram os 8 do infra), V2 = 1 (`consulta`), V12 = 1, demais 0 |
| `cargo build` / `cargo test` | passa / **273 + 28, 0 falhas** |
| `prompt/` (singular) | intocado |

Fluxo travado: `--update-snapshot` (8, "0 stale") → `--fix-hashes` (8, "0 drift").

---

## O que falta (quase fim)

- **L4**: `lente_wiring` (a fiação) + `lente_app` (o binário) — `@layer L4`. Aqui o
  **V12 do `ErroLente`** se **declara intencional** (o erro agregado mora na
  composição). `lente_app`: o `main`/`erro` (dispatch + tradução).
- **V1 atual = 12** → cai a ~0 com o L4 (menos agregadores/internos/testes).
- **V2 do `consulta.rs`** — teste mínimo, prompt à parte.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Escala do molde ao L3 (`lente_infra`, `@layer L3`): os **8** arquivos nucleados — `infra` (lib.rs, fachada: `extrair_grafo`/`desserializar_grafo`/`ErroAdaptador`), `infra-fork` (`invocar_fork`/`ErroFork`/`AlvoFork`), `infra-workspace` (membros+cache, `ErroWorkspace`/`MembroWorkspace`), `infra-diff` (`ler_diff`/`ErroDiff`), `infra-metadata` (detecção de alvo, `ErroMetadata`), e os internos `pub(crate)` `infra-dto`/`infra-invocacao`/`infra-traducao` (lógica ativa → ficam no walk, regra do 0062) — prompt real em `prompts/<unidade>.md` + cabeçalho `//! Crystalline Lineage @layer L3` (replace na linha `//! Lineage:`) + snapshot gerado (`--update-snapshot`→`--fix-hashes`). **Nenhum agregador puro** no `lente_infra` → nada novo em `[excluded_files]`. **No L3**: V4/V13 não dispararam (checks L1; I/O legítimo); `pub(crate)` entra no snapshot (0062); `const` não (0063); V2 não se aplica. **V3 = 0** confirmado (L3 importa L1, nunca L4). Os erros do L3 (`Fork`/`Workspace`/`Diff`/`Metadata`/`Adaptador`) moram no L3 — nucleados aqui (o `ErroLente` do L4 os agrega, próximo prompt). Linter: `lente_infra` **V1/V5/V6/V7 = 0**, **V3 = 0**, V12 = 1; V1 do projeto 20→12. **Preserva comportamento**: suíte 273 + 28; `prompt/` intocado. Falta só o L4 (`wiring`/`app`, `@layer L4`, V12 do `ErroLente` declarado) + o teste do `consulta`. | `00_nucleo/prompts/{infra,infra-fork,infra-workspace,infra-diff,infra-metadata,infra-dto,infra-invocacao,infra-traducao}.md` (novos), `03_infra/src/*.rs` (8 cabeçalhos), `00_nucleo/lessons/0064-escalar_l3.md` |
