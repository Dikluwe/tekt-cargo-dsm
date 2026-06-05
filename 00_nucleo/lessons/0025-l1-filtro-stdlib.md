# Laudo de Execução — Prompt 0025 (Filtro de stdlib) — Revisão A

**Camada**: L5 (laudo)
**Data Fase 1**: 2026-06-02
**Data Fase 2**: 2026-06-02
**Prompt executado**: `00_nucleo/prompt/0025-l1-filtro-stdlib.md` (revisão A)
**Estado**: `EXECUTADO` — após a Fase 1 refutar a premissa original
(`crate` por nó), o autor escolheu a **opção A**: D3 do ADR-0002
preservada, marca de stdlib por **prefixo do path**. O prompt foi
reescrito, e o crate `lente_filtro` (L1 puro) foi criado e validado
contra dado real do `lente_core`.

---

## Fase 1 — Achados (preservados; foi o que decidiu a revisão)

### O fork 0.27.0 não emite `crate` por nó

Rodei `cargo modules export-json --sysroot --compact --package lente_core`
e inspecionei os 108 nós e 278 arestas resultantes:

- Schema dos nós: `{id, path, name, kind, visibility[, trait, trait_ref, …]}` —
  **nenhum** tem campo `crate`. O top-level tem `crate: "lente_core"`.
- Consequência: `No.crate_name` é igual para os 108 nós, incluindo os 17
  cujo path é sysroot. Campo populado por **defaulting** (do crate-raiz),
  não por **fonte por nó**.
- O laudo 0013 D1 já tinha registrado isso. O comentário em
  `traducao.rs:59-62` permanece preciso.

### Mas o risco do Limite 2 não se materializa neste fork

| Categoria no `lente_core` | Como o fork nomeia |
|---|---|
| Impl-do-alvo de trait de stdlib (**54 ocorrências**) | path `lente_core::…::ErroRaio::fmt`, com `trait: "Display"` (distinção fica no campo `trait`, não no path) |
| Nó de trait/módulo de stdlib (17) | path `core::fmt`, `alloc::alloc::Global`, `std::*` |
| Sobreposição "path em prefixo sysroot ∧ `trait/trait_ref` preenchido" | **Zero** |

Logo: filtrar por **prefixo do path** remove sysroot **sem** tocar nos
impls-do-alvo neste fork. A D3 do ADR-0002, longe de ser superada, é
**vindicada pelo dado**.

### Conjunto observado de sysroot

Primeiro segmento do `path` no JSON do `lente_core`:

| segmento | nós |
|---|---|
| `lente_core` | 91 |
| `core` | 10 |
| `alloc` | 4 |
| `std` | 3 |

Sysroot observado: `{core, alloc, std}`. O filtro inclui também
`proc_macro` e `test` por defensividade — não observados aqui porque
`lente_core` não os exercita.

### Política para `crate_name` vazio

Sem objeto neste fork: `No.crate_name` é sempre o crate-raiz, nunca
vazio.

---

## Decisão A do autor

> "Reescrever o prompt usando prefixo do path (D3 preservada)."

A opção A:
- **Mantém a ADR-0002 D3** como está — o dado a vindica.
- **Reescreve o prompt** removendo a supersessão proposta da D3 e
  trocando "`crate_name`" → "prefixo do path".
- **Implementa o filtro** com a regra original do ADR.

A opção B (estender o fork para emitir `crate` por nó) e a C (híbrido)
ficam registradas no histórico do laudo Fase-1 mas não foram exercidas.
Caso o fork evolua no futuro, a B vira fato e a D3 será revisitada;
até lá, o filtro por prefixo é a melhor evidência por-nó disponível.

---

## Fase 2 — Implementação

### Estrutura

```
07_filtro/
├── Cargo.toml            # lente_filtro; deps: lente_core; dev-dep: lente_infra (só E2E)
├── src/lib.rs            # filtrar_stdlib + 10 testes unitários
└── tests/
    └── e2e_lente_core.rs # 3 E2Es #[ignore] contra o lente_core real
```

Adicionado `"07_filtro"` aos `members` do `Cargo.toml` raiz.

### API

```rust
pub fn filtrar_stdlib(grafo: &Grafo) -> Grafo
```

- **Predicado** `e_de_sysroot(path)`: testa o **primeiro segmento** do
  `path.as_str()` contra `SYSROOT_PREFIXES = ["std", "core", "alloc",
  "proc_macro", "test"]`. Comparação por segmento, não `starts_with` cego
  — `core_extras` ou `std_utils` (hipotéticos) não são confundidos.
  Defesa contra o tipo de erro que o laudo 0008 corrigiu na chave de
  aresta.
- **Coleta `ids_removidos: HashSet<usize>`** dos nós de sysroot.
- **Reconstrói `nodes`** mantendo os não-sysroot, com `id` preservado
  (sem renumeração).
- **Reconstrói `edges`** removendo as que `id_from` **ou** `id_to`
  referenciam nó removido.
- **Preserva `Grafo.crate_name`.**

### Pureza do L1

```
$ cargo tree -p lente_filtro --depth 1
lente_filtro v0.0.0
└── lente_core v0.0.0
[dev-dependencies]
└── lente_infra v0.0.0
```

A biblioteca depende só de `lente_core` (que por sua vez só usa stdlib
do Rust). `lente_infra` aparece como `dev-dependency` — só atinge o
target de teste, não vaza para a build do `lib`. Pureza do L1 mantida
no contrato real do crate.

---

## Verificação

### Testes unitários (10/10 verdes, sem cargo)

| Teste | O que prova |
|-------|-------------|
| `e_de_sysroot_aceita_primeiros_segmentos_conhecidos` | path em `core`/`core::…`/`alloc::…`/`std::…`/`proc_macro::…`/`test::…` → sysroot |
| `e_de_sysroot_rejeita_paths_do_alvo_e_falsos_positivos` | `meu::…`, `core_extras::…`, `std_utils`, `alloc_pool::…` → NÃO sysroot |
| **`limite_2_impl_do_alvo_de_trait_de_stdlib_e_preservado`** | **O teste central**: nó `meu::T::fmt` com `trait: Display`, aresta para `core::fmt::Display` — o impl fica, o trait some, a aresta para o trait some, as arestas internas do alvo ficam |
| `no_sysroot_e_arestas_que_o_tocam_sao_removidos` | nós `alloc::…` saem; arestas tocando (entrada **ou** saída) saem |
| `dep_nao_stdlib_e_mantida` | `emath::Vec2`, `ecolor::Color32` ficam — só sysroot vai embora |
| `ids_dos_mantidos_sao_preservados_sem_renumeracao` | ids não-contíguos `[7, 42, 99]`, sysroot remove `99`; saída `[7, 42]` |
| `grafo_crate_name_e_preservado` | `Grafo.crate_name` ≡ |
| `grafo_sem_stdlib_sai_inalterado` | `Grafo` igual via `PartialEq` (sanidade) |
| `idempotente_aplicar_duas_vezes_da_o_mesmo` | `f(f(g)) == f(g)` |
| `grafo_vazio_sai_vazio` | degenerado |

### E2Es `#[ignore]` (3/3 verdes contra `lente_core` real)

Rodadas: `cargo test -p lente_filtro -- --ignored`.

| Teste | O que prova |
|-------|-------------|
| `filtra_lente_core_remove_sysroot_preserva_alvo` | Nenhum nó remanescente tem path com prefixo sysroot; banda de contagem respeitada |
| `limite_2_real_impl_de_traits_de_stdlib_no_lente_core_permanecem` | 5 impls-do-alvo conhecidos (`ErroRaio::fmt` Display+Debug, `Raio::clone`, `Classificacao::eq`, `Classificacao::hash`) presentes após o filtro |
| `arestas_para_stdlib_somem_no_filtro` | Toda aresta remanescente tem ambas as pontas em ids vivos (sem aresta órfã); contagem de arestas cai |

### Números observados no `lente_core` (ancorados na medição executada)

```
nodes_antes=108  nodes_depois=91   (−17 sysroot)
edges_antes=278  edges_depois=180  (−98 que tocavam sysroot)
```

Coerente com a contagem por prefixo da Fase 1 (`core` 10 + `alloc` 4 +
`std` 3 = 17). A queda das arestas (35% delas tocam sysroot) ancora
quantitativamente o motivo do filtro: sysroot é um "atrator" de arestas
no grafo cru.

### Suíte completa

| Crate | Verdes | Ignored |
|-------|--------|---------|
| lente_core | 30 | 0 |
| lente_infra | 30 | 8 |
| lente_investiga | 17 | 0 |
| lente_resolve | 11 | 0 |
| lente_wiring | 6 | 1 |
| lente_catalogo | 7 | 0 |
| lente_cli | 16 | 1 |
| **lente_filtro (lib)** | **10** | **0** |
| **lente_filtro (E2E `tests/`)** | 0 (não-ignored) | **3** |

Total: **127 verdes** (era 117 no laudo 0024; +10 do `lente_filtro`).
**13 ignored** (era 9 no 0024; +3 do `lente_filtro` + 1 do
`fork::pacote_inexistente` que o 0023 já trouxe — todos passam quando
rodados). Sem regressão em nenhum crate.

### Subprocessos do cargo (invariante do laudo 0023)

```
$ grep -rn 'Command::new("cargo")' --include "*.rs"
03_infra/src/fork.rs:117      # cargo modules (export-json)
03_infra/src/metadata.rs:170  # cargo metadata
```

Dois, cada um único. O crate novo não roda subprocesso (puro L1).

---

## Decisões tácitas

### D1 — Comparação por segmento, não `starts_with`

```rust
let primeiro = match s.find("::") { Some(i) => &s[..i], None => s };
SYSROOT_PREFIXES.contains(&primeiro)
```

`path.starts_with("core")` aceitaria `"core_extras::Y"` como sysroot —
falso positivo. Comparar o **primeiro segmento** evita isso. Custo: uma
chamada de `find`. Defesa adotada antes que o problema apareça (mesmo
princípio do laudo 0008 sobre chave de aresta).

### D2 — `proc_macro` e `test` no conjunto, mesmo sem observação

A Fase 1 viu `{core, alloc, std}` no `lente_core`. Incluí `proc_macro` e
`test` por defensividade — crates exóticos podem exercitá-los (laudo
0023 já entrou em proc-macro como "tem lib"). Inclusão preventiva
porque (a) custo é uma string a mais na const, (b) o erro de **deixar**
um nó de stdlib passar é silencioso (vira ruído no ranking — o
sintoma que motivou o filtro), enquanto o erro de **remover** algo do
alvo seria detectado pelos testes do Limite 2.

### D3 — `lente_infra` como `dev-dependency`, não dependência regular

O `lib` do `lente_filtro` precisa só de `lente_core` (pureza L1). O E2E
contra dado real precisa de `lente_infra` para extrair o grafo do disco.
Solução padrão Cargo: `[dev-dependencies]`. Não vaza para a build do
`lib` (verificado por `cargo tree`). Permite escrever o E2E sem fazer
o `lente_filtro` parecer "menos puro" para downstream.

### D4 — E2Es no `tests/`, não em `#[cfg(test)] mod`

Convenção pré-existente do projeto: testes inline em `mod tests`. Os
E2Es do filtro fogem disso porque precisam da `dev-dependency`
`lente_infra` (que só está disponível para targets de teste). Pôr os
E2Es em `tests/e2e_lente_core.rs` é a forma padrão do Cargo para isso,
e mantém a inline-rule para testes que não precisam de dev-deps. O
laudo registra para não ser confundido com fuga ad hoc da convenção.

### D5 — Banda de contagem, não número exato no E2E

O E2E principal afirma `nodes_antes ∈ [100, 130]` e `alvo ∈ [80, 110]`,
não exatamente 108/91. Razão: o número exato depende da versão do
fork — mudanças no descritor (laudo 0013 e seguintes) podem fazer +/-
nós. Banda generosa amortece variações sem perder a propriedade real
("a maior parte é alvo; a minoria é sysroot"). O número observado fica
fixado neste laudo como ancoragem histórica.

### D6 — Aresta sai se **qualquer** ponta é removida

Tanto faz se o nó removido era origem ou destino — a aresta perdeu
sentido. Implementação: `!ids_removidos.contains(&a.id_from) &&
!ids_removidos.contains(&a.id_to)`. O teste
`no_sysroot_e_arestas_que_o_tocam_sao_removidos` cobre os dois sentidos
(arestas entrando E saindo do nó removido).

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0025 |
|-----------|-----------------|
| Laudo 0021, pendência 2 (sysroot domina ranking) | **Componente pronto** — falta o consumidor (modo ranking). |
| ADR-0002 D3 (marca por prefixo do path) | **Vindicada pelo dado**; não tocada. |
| Limite 2 da spec (preservar impl-do-alvo de trait de stdlib) | **Coberto** — verificado por construção contra o fork 0.27.0 (sobreposição zero); ancorado por testes. |
| Filtro de "folhas comportamentais" (Limite 3) | **Aberta** — outra pendência. |
| Modo ranking / wiring / CLI | **Aberta** — consumidor do filtro, prompt próprio (próximo). |

---

## O que NÃO mudou (declaração explícita)

- **ADR-0002 D3**: intacta. O prompt anterior propunha superá-la; a
  revisão A não o faz.
- **`lente_core` (L1)**: zero toques. `cargo tree -p lente_core` continua
  sem dependências.
- **`No.crate_name`**: campo continua existindo, populado pelo L3 com o
  crate-raiz. O filtro não o lê.
- **Fork / wiring / CLI / E2 / `raio` / `investiga` / `resolve`**: zero
  toques.
- **Assinaturas públicas** de outros crates: zero toques.

---

## Histórico do prompt (cronologia da revisão)

| Marco | Data | Conteúdo |
|-------|------|----------|
| Prompt criado | 2026-06-02 | Premissa: fork passou a emitir `crate` por nó; supersede D3. |
| **Fase 1 (este laudo, suspensão original)** | 2026-06-02 | Verificação refuta premissa: fork não emite `crate` por nó; D3 vindicada pelo dado. |
| Decisão do autor | 2026-06-02 | "Opção A" — preservar D3, marcar por prefixo do path. |
| Prompt reescrito (revisão A) | 2026-06-02 | Texto ajustado: "`crate_name`" → "prefixo do path"; observação metodológica reescrita; D3 preservada. |
| Fase 2 | 2026-06-02 | `lente_filtro` criado, 10 testes unitários + 3 E2Es verdes; números ancorados (108→91 nós, 278→180 arestas). |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-02 | (Fase 1) Verificação contra JSON real refutou premissa; supersessão da ADR-0002 D3 abandonada; aguarda decisão do autor. | `00_nucleo/lessons/0025-l1-filtro-stdlib.md` (registro de suspensão; nenhum código tocado) |
| 2026-06-02 | (Fase 2, revisão A) Após decisão do autor, prompt reescrito e implementação concluída. Crate `lente_filtro` (L1 puro) com `filtrar_stdlib(&Grafo) → Grafo` por prefixo do path; 10 testes unitários + 3 E2Es contra `lente_core` real (sysroot some, 54 impls-do-alvo preservados). 127 verdes + 13 ignored; pureza do L1 mantida (`cargo tree` confirma). | `07_filtro/{Cargo.toml,src/lib.rs,tests/e2e_lente_core.rs}`, `Cargo.toml` raiz (members), `00_nucleo/prompt/0025-l1-filtro-stdlib.md` (revisão A), `00_nucleo/lessons/0025-l1-filtro-stdlib.md` (este registro). **`00_nucleo/adr/0002-modelagem-do-grafo.md`: NÃO tocado.** |
