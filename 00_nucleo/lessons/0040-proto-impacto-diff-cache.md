# Laudo de Execução — Prompt 0040 (Protótipo do impacto de um diff — cache + incremental)

**Camada**: L5 (laudo — registro de Arena)
**Data**: 2026-06-05
**Prompt executado**: `00_nucleo/prompt/0040-proto-impacto-diff-cache.md`
**Tipo**: Arena visual, terceira rodada do laudo 0038/0039 — bruto em
`lab/proto-impacto-diff/`, registro aqui (padrão dos laudos 0021,
0029, 0036, 0038, 0039).
**Estado**: `EXECUTADO` — cache por crate + extração incremental
funcionando contra o monorepo da lente. Cinco cenários cronometrados;
veredito: uso reativo **viável**. Suíte de produção intacta
(**213 verdes + 22 ignored**, idêntica ao laudo 0037).

---

## A resposta da pergunta central

**O uso reativo é viável**. Com cache do JSON cru por crate (chave =
SHA-256 dos fontes), o caminho típico (edição em 1 crate → consulta)
cai de **~33 s para ~3 s**, e o cache totalmente quente para **~70 ms**.

| Cenário | Extraídas | Reusadas | Fork | **Total** | Veredito |
|---|---:|---:|---:|---:|---|
| **Cache quente** (nada mudou) | 0 | 10 | 0.00 s | **0.07 s** | ✓ instantâneo |
| **Morno-1** (1 crate alterado) | 1 | 9 | 2.95 s | **3.02 s** | ✓ interativo |
| **Morno-3** (3 crates alterados) | 3 | 7 | 9.69 s | **9.77 s** | ⚠ tolerável |
| **Cold** (cache vazio) | 10 | 0 | 31.68 s | **31.76 s** | ✗ inevitável (1×/sessão) |
| **Renomeação** (cache stale) | 0 | 10 | 0.00 s | **0.07 s + 1 fantasma** | ✓ sinal correto |

A parte sem-fork (desserializar + unir + mapear) é **< 10 ms total**
para 10 crates — não vira gargalo. **Cachear a união já montada não
se justifica.**

---

## Como rodar

```bash
cd lab/proto-impacto-diff

# Cache quente (após 1ª execução, sem mudança no fonte):
cargo run --release -- --repo "$(cd ../../ && pwd)" --input git \
    --out dados/cache-quente.json

# Morno-N (simular invalidação de N crates):
cargo run --release -- --repo "$(cd ../../ && pwd)" --input git \
    --invalidar lente_core --out dados/cache-morno1.json

# Cold (do zero):
cargo run --release -- --repo "$(cd ../../ && pwd)" --input git \
    --limpar-cache --out dados/cache-cold.json

# Simular renomeação para detectar fantasmas:
cargo run --release -- --repo "$(cd ../../ && pwd)" --input git \
    --simular-renomeacao "lente_core::entities::grafo::No=>NoRenomeado" \
    --out dados/cache-renomeacao.json

python3 -m http.server 8080
# abrir http://localhost:8080/ — selector com os 7 dumps
```

---

## Pipeline implementado (extensão do 0039)

1. `cargo metadata --no-deps` → 10 crates-membros (do 0039).
2. **Para cada crate**: hash dos `.rs` sob `src/` (SHA-256
   determinístico, path-relativo + 0x00 + len + 0x00 + content +
   0x00). Compara com `cache/<crate>.hash`. Se bate, reusa
   `cache/<crate>.json`. Se não, roda o fork e atualiza.
3. **União por path** sobre todos os 10 (do 0039) + **detecção de
   fantasmas** (paths cujo primeiro segmento é um crate do workspace
   mas que não estão entre as origens — sinal de cache stale ou
   renomeação).
4. Mapeamento diff→nós + raio (do 0039).
5. JSON estrutura `Cronometria` (total / metadata / fork / cache /
   desser+união / mapeamento) e `CacheResumo` (cenário, extraídas vs
   reusadas, tempos).

---

## Confirmações principais (detalhe no `relatorio.md`)

### 1. Cache faz o caminho morno reativo

3 s para uma edição em 1 crate é o tempo de uma rodada de `cargo
check`. Abaixo do limiar interativo (~5 s) que o prompt usa como
referência. Morno-3 (10 s) ainda é tolerável.

### 2. Parte sem-fork é insignificante (~10 ms)

Hipótese do prompt §3: a parte sem-fork pode virar gargalo com 10
JSONs. Refutada — 10 ms para o workspace inteiro. **Cachear a
união já montada não é otimização que se justifique** (registrado).

### 3. SHA-256 dos fontes pega edições não-comitadas

Editar comentário em `01_core/src/entities/grafo.rs` muda o hash de
`lente_core`. Re-extração dispara. Coerente com o uso reativo.

**Limitação**: não pega `Cargo.toml` (mudança de features/deps).
Registrada para o produto.

### 4. Renomeação NÃO produz arestas soltas no monorepo (contra-hipótese)

O prompt §4 antecipava arestas soltas (`B::f → A::No` ficando órfã
quando `A::No` some). **No monorepo da lente, não acontece**: cada
extração inclui **nós-referência** dos crates dependentes (com
`position` própria via `cargo metadata`). O cache stale de B
carrega `A::No` como nó próprio. A união casa.

**Mas o sinal está em outra forma — implementei**: um **nó fantasma**
é um path cujo primeiro segmento bate um crate do workspace, mas
que **não** está entre as origens (crates cujos caches o
produziram).

Para a renomeação simulada (`lente_core::entities::grafo::No` →
`NoRenomeado`, só no cache do `lente_core`):

```
↳ fantasmas (sinal de cache stale / renomeação):
    lente_core::entities::grafo::No
      esperado em: lente_core
      vem de:      [lente_estrutura, lente_infra, lente_investiga, lente_resolve]
```

**A lista "vem de" é EXATAMENTE a lista de crates impactados pela
renomeação.** O sinal certo, em forma melhor que a esperada (lista
direta dos afetados).

### 5. Cold é inevitável (~32 s)

10 extrações × ~3 s = ~32 s. Cada crate paga seu cold-start do
rust-analyzer. **Sem como evitar** (sem mudar o fork). Para uso
humano: tolerável (1×/sessão). Para CI/agente reativo: pré-aquecer
o cache antes do uso.

---

## Decisões

- **Cache do JSON cru**, não do `Grafo`. O `lente_core::Grafo` é
  puro (sem `serde`); cachear JSON evita mexer no produto. Custo
  de re-desserializar: ~1 ms/crate × 10 = ~9 ms total. Trivial.
- **Chave = SHA-256 dos fontes** (não commit-hash). Pega edições
  não-comitadas — requisito do uso reativo. `DefaultHasher` da
  stdlib não serve (seed muda entre runs).
- **`--invalidar <c1,c2,...>`** para cronometrar morno-N sem editar
  o repo (Arena não pode editar produção).
- **Detecção de fantasmas como sinal de renomeação** — substitui a
  expectativa de "arestas soltas" do prompt, que não se manifesta no
  monorepo (caches dependentes carregam nós-referência).
- **Cache em `lab/proto-impacto-diff/cache/`** (Arena, autocontido).
  No produto, viraria `~/.cache/lente/<repo-hash>/`.

---

## Custo

| Fase | Cold | Morno-1 | Morno-3 | Quente |
|---|---:|---:|---:|---:|
| `cargo metadata --no-deps` | 56 ms | 56 ms | 56 ms | 56 ms |
| Extração de fork (acumulada) | 31.68 s | 2.95 s | 9.69 s | 0 |
| Cache I/O (read+write) | <1 ms | <1 ms | <1 ms | <1 ms |
| Desserializar + unir + mapear | 9 ms | 9 ms | 9 ms | 9 ms |
| **Total** | **31.76 s** | **3.02 s** | **9.77 s** | **0.07 s** |

---

## Estado da suíte

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **213 verdes + 22 ignored** — idêntica ao laudo 0037/0038/0039 |
| Crates de produção tocados | **Zero** — Arena pura |
| `Cargo.toml` raiz | intocado — `lab/proto-impacto-diff` tem `[workspace]` próprio |
| Subprocessos do cargo (invariante 0023) | dois únicos, intocados |
| Fork tocado | **Não** — só invocado |

---

## Conteúdo bruto

```
lab/proto-impacto-diff/
├── Cargo.toml             # bin; deps lente_core + lente_infra + serde + sha2
├── src/main.rs            # ~1100 linhas: pipeline multi-crate + cache + incremental
├── index.html             # UI com selector de 7 dumps + resumo de cache + fantasmas
├── cache/
│   ├── <crate>.json       # JSON cru cacheado, 1 por crate (10 arquivos)
│   └── <crate>.hash       # SHA-256 dos fontes do crate
├── dados/
│   ├── cache-cold.json          # 31.76s total
│   ├── cache-quente.json        # 0.07s total
│   ├── cache-morno1.json        # 3.02s total (lente_core invalidado)
│   ├── cache-morno3.json        # 9.77s total (3 crates invalidados)
│   ├── cache-renomeacao.json    # 0.07s, 1 fantasma detectado
│   ├── impacto-multi-ambos.json # legacy 0039
│   └── impacto-ambos.json...    # legacy 0038
└── relatorio.md           # conteúdo denso (perguntas do 0040 + decisões D8–D11)
```

Conteúdo denso em `relatorio.md` (tabelas de tempo, robustez da
chave de cache, achado dos fantasmas, decisões D8–D11).

---

## Para a próxima rodada

| Item | Estado |
|---|---|
| Cache por crate + incremental | **Coberto** — morno-1 ~3 s, quente ~70 ms |
| Veredito de viabilidade reativa | **Coberto** — sim, viável |
| Renomeação como sinal de impacto | **Coberta** — via fantasmas (origens = afetados) |
| Robustez da chave de cache | **Coberta** — pega edição não-comitada; limitação em `Cargo.toml` registrada |
| Custo cold (~32 s) | **Documentado, inevitável** — pré-aquecimento como estratégia |
| Modo `--diff` na CLI de produto | **Aberto** — Ponte 2 da trilha local |
| Casca MCP | **Aberto** |
| Untracked (achado do 0038) | **Aberto** |
| Filtros para diffs grandes | **Aberto** (Achado 6 do relatório do 0039) |
| Cache key inclui `Cargo.toml` | **Aberto** — futuro do produto |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Terceira rodada da Arena `lab/proto-impacto-diff/` — cache do JSON cru por crate (chave = SHA-256 determinístico dos fontes, pega edições não-comitadas) + extração incremental (re-extrai só os crates cujo hash mudou) + união por path sobre o cache. Cinco cenários cronometrados: cold 31.8s (1×/sessão, inevitável), morno-1 3.0s (interativo), morno-3 9.8s (tolerável), quente 70ms (instantâneo). Parte sem-fork = 10ms (refuta hipótese de gargalo do prompt §3). Renomeação **não** produz arestas soltas no monorepo (caches dependentes carregam nós-referência); sinal correto implementado via **nós fantasma** — path com primeiro segmento batendo crate do workspace mas sem origens; a lista de origens nomeia EXATAMENTE os crates afetados pela renomeação. Veredito: uso reativo **viável**. Zero toque no produto. | `lab/proto-impacto-diff/{src/main.rs,Cargo.toml,index.html,cache/*,dados/cache-*.json,relatorio.md}`, `00_nucleo/lessons/0040-proto-impacto-diff-cache.md` |
