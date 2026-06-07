# Laudo de Execução — Prompt 0039 (Protótipo multi-crate do impacto de um diff)

**Camada**: L5 (laudo — registro de Arena)
**Data**: 2026-06-05
**Prompt executado**: `00_nucleo/prompt/0039-proto-impacto-diff-multicrate.md`
**Tipo**: Arena visual, segunda rodada do laudo 0038 — bruto em
`lab/proto-impacto-diff/`, registro aqui (padrão dos laudos 0029, 0036,
0038).
**Estado**: `EXECUTADO` — pipeline multi-crate funciona contra dado
real; UI ganha nível de crate; o `lente_infra` que o 0038 perdeu agora
aparece. Suíte de produção intacta (213 verdes + 22 ignored, idêntica
ao laudo 0037).

---

## A resposta da pergunta central

**Sim, o impacto cruza crates — e é grande.** Para os tipos públicos do
`lente_core` tocados pelo diff atual (`Posicao`, `No`):

| Nó | Raio local | Raio workspace | Δ cross-crate |
|---|---:|---:|---:|
| `lente_core::entities::grafo::Posicao` | 15 | **48** | **+33** |
| `lente_core::entities::grafo::No` | 11 | **44** | **+33** |

Os DTOs internos do `lente_infra` ficam contidos (Δ = 0). A vista
single-crate do 0038 escondia 33 dependentes reais por nó.

**A trilha local precisa de visão de workspace para responder
honestamente.**

---

## Como rodar

```bash
cd lab/proto-impacto-diff
git -C ../../ diff HEAD | cargo run --release -- \
    --repo "$(cd ../../ && pwd)" \
    --input ambos \
    --out dados/impacto-multi-ambos.json
python3 -m http.server 8080
# abrir http://localhost:8080/
```

---

## Pipeline implementado (extensão do 0038)

1. **`cargo metadata --no-deps`** → descobre os 10 crates-membros do
   workspace, com `manifest_path` para cada.
2. **Mapear arquivo→crate** por maior-prefixo-casa.
3. Ler `git diff` (stdin OU `git diff HEAD`).
4. **Extrair grafo de cada crate-membro** via `lente_infra::extrair_grafo`
   (10 extrações × ~3.3 s = ~33 s total).
5. **União por path** (abordagem B do prompt): cada path único entra
   uma vez no grafo unido; IDs reatribuídos sequencialmente; arestas
   reanchoradas pelo path em `from`/`to`. **0 arestas soltas** no
   monorepo da lente — casamento por path é robusto.
6. Para cada nó tocado: calcular **dois raios** — local (no grafo do
   crate dono) e workspace (na união). O delta mostra o cross-crate.
7. JSON estruturado por crate; UI agrupa em **três camadas**
   (crate → arquivo → nó → amostra do montante).

---

## Confirmações principais (detalhe no `relatorio.md`)

### 1. O 0038 perdia `lente_infra`; agora não mais

Mesmo diff que o 0037 produziu — 7 crates tocados; **7 nós em
`lente_infra`** (`PositionDTO`, `NoDTO`, módulo `dto`, módulo `traducao`,
função `traduzir`, etc.). O 0038 só via os 3 nós do `lente_core`.

### 2. Comparação stdin vs git: IGUAIS

Mantém o achado do 0038 no multi-crate.

### 3. Abordagem A não basta

`lente_wiring` extraído (que depende de todos os L1) traz só **subset**
dos dependentes — só o que o wiring usa. Para "todos os dependentes
de `No`", precisa da **união B**.

### 4. Sysroot e `position` (dúvida do 0038 fechada)

`--sysroot` ligado (política `lente_infra::fork::invocar_em`). **100% dos
nós têm `position`** — inclusive os de stdlib (fork lê o source da
rustc via `cargo metadata`). Os caminhos absolutos da stdlib não
batem a raiz do repo, então a `relativizar` filtra fora — **não geram
falso-positivo** na vista. Encerra a pergunta do laudo 0038.

### 5. Macro/derive — surpresa de honestidade

Os métodos derivados de `Posicao` (`clone`, `eq`, `fmt`) têm
`position` apontando para os **fontes originais dos traits na stdlib**
(`.../rustlib/.../clone.rs`, `cmp.rs`, `mod.rs`), não para o
`#[derive(...)]` no código. Implicações:

- **Não há falso-positivo**: adicionar uma struct com derives não
  marca `Clone::clone`/`Debug::fmt` como tocados.
- **Subreporte**: editar o `#[derive(...)]` (ex.: remover `Clone`)
  marca a struct mas **não** os métodos gerados. Os usuários de
  `Posicao::clone` ficam invisíveis na vista.

O briefing §5 mencionava "call-site" para itens gerados por macro; o
fork não faz isso para `#[derive]` (aponta para a stdlib em vez do
call-site). Comportamento documentado.

### 6. Casamento por path no monorepo: zero soltas

A união B reanchora as arestas pelo path em `from`/`to`. Nenhuma
aresta cross-crate aponta para path inexistente no grafo unido.
Confirma que dentro de um monorepo onde paths começam com nome do
crate, o casamento por path é estável (não há colisão entre crates).

---

## Custo

| Fase | Custo |
|---|---|
| `cargo metadata --no-deps` | <100 ms |
| 10× extração de fork | **~33 s** (cold-start do rust-analyzer) |
| União por path | <100 ms |
| Mapeamento diff→nós + raio | <100 ms |
| **Total** | **~33 s** |

**Para produto**: **inviável sem cache** numa CI ou agente reativo. O
caminho: cachear grafo por crate por commit-hash. Outro prompt.

**Para uso manual**: ~33s é tolerável; ainda mais barato que esperar
CI rodar.

---

## Decisões

- **União sempre sobre o workspace inteiro** (default). Opção
  `--so-tocados` reduz extração ao subconjunto, mas perde dependentes
  em crates não-tocados.
- **Casar arestas por path, não por id** — ids do petgraph são
  instáveis entre extrações (briefing §7). Path é a única identidade
  estável.
- **`mod tests` cai sob o módulo dono** — `cfg(test)` não entra no
  grafo do fork; coerente com o achado do 0038.
- **`nos_leves` instrumentação preservada** — conta nós sem `position`
  + sem campos de descritor. Resultado para o monorepo da lente: 0
  em todos os crates (o fork emite nós completos via `cargo
  metadata`). Para deps externas seria diferente; não exercitado.

---

## Estado da suíte

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **213 verdes + 22 ignored** — idêntica ao laudo 0037 |
| Crates de produção tocados | **Zero** — Arena pura |
| `Cargo.toml` raiz | intocado — `lab/proto-impacto-diff` tem `[workspace]` próprio |
| Subprocessos do cargo (invariante 0023) | dois únicos, intocados |

---

## Conteúdo bruto

```
lab/proto-impacto-diff/
├── Cargo.toml             # bin; deps lente_core + lente_infra + serde
├── src/main.rs            # ~750 linhas: pipeline multi-crate completo
├── index.html             # ~12 KB: UI em três camadas (crate→arquivo→nó)
├── dados/
│   ├── impacto-multi-ambos.json   # ~60 KB (multi-crate, ambos inputs)
│   ├── impacto-ambos.json         # legacy 0038 (single-crate)
│   ├── impacto-git.json           # idem
│   └── impacto-stdin.json         # idem
└── relatorio.md           # conteúdo denso (perguntas do 0039 + decisões D5–D7)
```

Conteúdo denso em `relatorio.md` (perguntas detalhadas, achado dos
derives, tabelas comparativas, decisões adicionais D5–D7).

---

## Para a próxima rodada

| Item | Estado |
|---|---|
| Multi-crate funciona | **Coberto** |
| Cross-crate explicitado (delta visível) | **Coberto** |
| Abordagem B (união por path) validada | **Coberta** — 0 arestas soltas |
| Sysroot e `position` (dúvida do 0038) | **Fechada** — 100% têm position; relativizar filtra fora-do-repo |
| Macro / `#[derive]` | **Documentado** — fork aponta para stdlib; conservador, subreporta no caso de edição do derive |
| Cache de extração (para o produto) | **Aberto** — ~33s/diff é inviável em CI |
| Modo `--diff` na CLI + Casca MCP | **Abertos** — Ponte 2 da trilha local |
| Untracked (achado do 0038) | **Aberto** |
| Filtros para diffs grandes | **Aberto** (Achado 6 do relatório) |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Segunda rodada da Arena `lab/proto-impacto-diff/` — agora multi-crate: descobre crates via `cargo metadata`, extrai todos (10×3.3s≈33s), une grafos por **path** (abordagem B; 0 arestas soltas), calcula raio local + raio workspace por nó tocado, mostra cross-crate na UI em três camadas (crate→arquivo→nó). Confirma: o `lente_infra` que o 0038 perdeu (7 nós em `dto.rs`/`traducao.rs`) agora aparece; impacto cross-crate é grande (`No` 11→44, `Posicao` 15→48); sysroot ligado mas todos os nós têm position (fork lê source da rustc); `#[derive]` aponta para stdlib (subreporta edição de derive sem falso-positivo). Zero toque no produto. | `lab/proto-impacto-diff/{src/main.rs,index.html,dados/impacto-multi-ambos.json,relatorio.md}`, `00_nucleo/lessons/0039-proto-impacto-diff-multicrate.md` |
