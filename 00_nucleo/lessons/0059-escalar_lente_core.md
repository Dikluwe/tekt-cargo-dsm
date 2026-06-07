# Laudo de Execução — Prompt 0059 (escalar o molde para o `lente_core`)

**Camada**: transversal (`prompts/` + cabeçalhos do `lente_core`)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0059-escalar_lente_core.md`
**Estado**: `EXECUTADO` — 7 unidades do `lente_core` nucleadas; 3 agregadores
excluídos; **V1/V5/V6/V7 = 0** no crate. V1 do projeto **41 → 31**. Refactor
preservado (V3=0, V12=1). Suíte **273 + 28**. `prompt/` intocado.

---

## A resposta em uma sentença

O molde do 0058 escalou ao maior crate L1: cada um dos 7 arquivos-com-interface do
`lente_core` ganhou prompt de nucleação + cabeçalho Cristalino + snapshot gerado;
os 3 agregadores (`lib.rs`/`mod.rs`, puro `pub mod`) foram **excluídos** (não
nucleados); o crate ficou limpo nos checks de linhagem, sem tocar código.

---

## As unidades do `lente_core` (lista real, da fonte)

| Unidade | Arquivo | Interface (resumo do snapshot gerado) |
|---|---|---|
| `grafo` | `entities/grafo.rs` | 11 tipos (`No`/`Aresta`/`Grafo`/`Path`/`Relation`/`UsesKind`/`Visibility`/`Kind`/`Modificadores`/`Posicao`/`ValorDesconhecido`) |
| `veredito` | `entities/veredito.rs` | `Veredito`, `Evidencia` |
| `raio` | `domain/raio.rs` | `calcular_raio`; `Classificacao`/`Raio`/`ErroRaio` |
| `uniao` | `domain/uniao.rs` | `unir_grafos`; `GrafoCrate`/`Fantasma`/`ResultadoUniao` |
| `mapeamento` | `domain/mapeamento.rs` | `mapear_diff`; `OrigemArquivo`/`FaixaLinhas`/`ArquivoDiff`/`DiffEstruturado`/`NoTocado`/`MapeamentoDiff` |
| `resultado_diff` | `domain/resultado_diff.rs` | `combinar_raios`; `TocadoComRaio`/`RaioCombinado`/`ResultadoDiff` |
| `consulta` | `domain/consulta.rs` | `FonteGrafo`/`Escopo`/`ModoUses`/`AlvoBusca` |

Cada uma: `00_nucleo/prompts/<unidade>.md` (nucleação real — Propósito/
Comportamento/Restrições/Critérios/Snapshot/Histórico) + cabeçalho
`//! Crystalline Lineage / @prompt … / @prompt-hash … / @layer L1 / @updated`.

---

## Refinamento do molde: os agregadores são EXCLUÍDOS, não nucleados

Os 3 arquivos **só-`pub mod`** sem interface própria — `lib.rs`,
`domain/mod.rs`, `entities/mod.rs` — disparavam V1 mas **não têm o que nuclear**.
Decisão (coerente com o ADR-0010 do linter: agregadores estruturais ficam fora da
topologia de camadas): **excluí-los** via `[excluded_files]` (path relativo
exato), **não** dar-lhes prompt:

```toml
[excluded_files]
core_lib          = "01_core/core/src/lib.rs"
core_domain_mod   = "01_core/core/src/domain/mod.rs"
core_entities_mod = "01_core/core/src/entities/mod.rs"
```

**Regra para escalar**: arquivo com interface pública (incl. re-exports `pub use`)
→ nucleação; agregador só-`pub mod` → `[excluded_files]`. (O `[excluded_files]` é
exato-por-path, então a lista cresce com a migração — explícito, aceito.)

---

## O snapshot — gerado, casa a interface real

Confirmação de que o `--update-snapshot` deriva a interface verdadeira (não a
semente vazia). Ex.: `raio` ficou
`{"functions":[{"name":"calcular_raio","params":["&Grafo","&Path"],"return_type":"Result<Raio, ErroRaio>"}],"types":[{"name":"Classificacao",…},{"name":"Raio",…},{"name":"ErroRaio",…}]}`.
O `grafo` capturou os 11 tipos. Fluxo na ordem travada:
`--update-snapshot` (7 arquivos, "0 stale") → `--fix-hashes` (7 hashes, "0 drift").

---

## Resultado do linter

| Check | `lente_core` (`01_core/core`) | Projeto |
|---|---|---|
| **V1** (cabeçalho) | **0** | **31** (era 41 — caíram os 10: 7 migrados + 3 excluídos) |
| **V5** (hash) | **0** | 0 |
| **V6** (snapshot) | **0** | 0 |
| **V7** (órfão) | **0** | 0 (cada prompt novo tem arquivo apontando) |
| **V3 / V12** | — | **0 / 1** (refactor preservado — só o `ErroLente`) |
| V4/V8/V9/V13/V14 | 0 | 0 |

**Os critérios do prompt (V1/V5/V6/V7 = 0 no `lente_core`) — atendidos.**

### Achado: V2 = 1 em `consulta.rs` (pré-existente, fora do escopo)

O `consulta.rs` (os 4 enums de pedido, movidos do wiring no 0056) **não tem
`#[cfg(test)]`** — os outros arquivos do domínio (`raio`/`uniao`/`mapeamento`/
`resultado_diff`) têm. O **V2** (cobertura de teste do núcleo) dispara nele. É
**pré-existente** (já estava no run do 0058, antes desta migração) e o **único V2
do projeto**. Fora do escopo deste prompt (que **não muda código**) — **reportado,
não corrigido**. Recomendação para um prompt futuro: um `#[cfg(test)] mod tests`
mínimo no `consulta.rs` (ex.: `Escopo::default() == Completo`,
`ModoUses::default() == Todas`) zera o V2 do projeto.

---

## Verificação de comportamento

| Item | Resultado |
|------|-----------|
| `cargo build --workspace` | passa |
| `cargo test --workspace` | **273 + 28, 0 falhas** — só `//!` + prompts novos |
| `prompt/` (singular) | **intocado** |
| `prompts/` (novo) | `ranking` (0058) + 7 do `lente_core` |

---

## O que falta escalar

- **Crates L1 pequenos**: `lente_filtro`, `lente_estrutura`, `lente_investiga`,
  `lente_resolve` (cada um, em geral, um `lib.rs` com interface → um prompt; os
  agregadores excluídos). `lente_ranking` já foi (0058).
- **L2/L3/L4**: `lente_catalogo`/`lente_cli` (L2), `lente_infra` (L3 — vários
  arquivos), `lente_wiring`/`lente_app` (L4 — com `@layer L4`; aqui o V12 do
  `ErroLente` se declara). Atenção ao `@layer` correto por camada.
- À medida que migram, o **V1 cai** de 31 rumo a 0.
- **V2 do `consulta.rs`** — um teste mínimo, num prompt à parte (muda código).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Escala do molde (0058) para o `lente_core`. 7 unidades com interface (`entities/{grafo,veredito}`, `domain/{raio,uniao,mapeamento,resultado_diff,consulta}`) nucleadas: prompt real em `00_nucleo/prompts/<unidade>.md` (semente de snapshot vazia) + cabeçalho `//! Crystalline Lineage / @prompt / @prompt-hash / @layer L1 / @updated`. Fluxo travado: `--update-snapshot` (gera os 7 snapshots reais — ex.: `grafo` com 11 tipos) → `--fix-hashes` (7 hashes). **Refinamento do molde**: os 3 agregadores só-`pub mod` (`lib.rs`, `domain/mod.rs`, `entities/mod.rs`) **excluídos** via `[excluded_files]` (path exato), não nucleados (ADR-0010). Linter: `lente_core` **V1/V5/V6/V7 = 0**; V1 do projeto 41→31; V3=0, V12=1, demais 0. **Achado V2=1** em `consulta.rs` (sem `#[cfg(test)]` — pré-existente do 0056, único V2 do projeto): reportado, **não** corrigido (fora do escopo; muda código). **Preserva comportamento**: suíte 273 + 28; `prompt/` intocado. Falta escalar aos demais crates (L1 pequenos, depois L2/L3/L4 com `@layer` próprio). | `00_nucleo/prompts/{grafo,veredito,raio,uniao,mapeamento,resultado-diff,consulta}.md` (novos), `01_core/core/src/{entities,domain}/*.rs` (cabeçalhos), `crystalline.toml` (`[excluded_files]`), `00_nucleo/lessons/0059-escalar_lente_core.md` |
