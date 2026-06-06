# Laudo de Execução — Prompt 0045 (`unir_grafos` (L1) + `montar_grafo_workspace` (L4) — o grafo de workspace)

**Camada**: L1 — Núcleo (`lente_core`, a união) + L4 — Fiação (`lente_wiring`, a orquestração)
**Data**: 2026-06-05
**Prompt executado**: `00_nucleo/prompt/0045-uniao_grafos_e_orquestracao.md`
**Estado**: `EXECUTADO` — `unir_grafos` (L1, pura) nucleada em `01_core/src/domain/uniao.rs`;
`resolver_colisoes` extraído no `lente_wiring` (refator que preserva comportamento);
`montar_grafo_workspace` + `GrafoWorkspace` + `ErroLente::Workspace` nucleados. Suíte
verde (242 verdes + 26 ignored; os 2 E2E novos de workspace passam com o fork real).
Pureza L1 intacta.

---

## A entrega em uma sentença

O motor do **grafo de workspace** está de pé: `unir_grafos` (L1, só stdlib) une os
grafos resolvidos por crate **por path** — definição vence referência, path órfão
vira fantasma com representante (0 arestas soltas), ids reindexados e arestas
religadas pelo path; `montar_grafo_workspace` (L4) orquestra enumerar→extrair
cacheado→resolver por crate→unir, devolvendo o grafo unificado de **todos** os
membros (421 nós, 1436 arestas, **0 fantasmas** no repo real).

---

## O que foi adicionado

### L1 — `01_core/src/domain/uniao.rs` (módulo novo)

| Item | Assinatura | Nota |
|---|---|---|
| `GrafoCrate` | `{ crate_name: String, grafo: Grafo }` | a etiqueta é o que distingue definição de referência |
| `Fantasma` | `{ path: Path, referenciado_por: Vec<String> }` | `referenciado_por` ordenado, sem repetição |
| `ResultadoUniao` | `{ grafo: Grafo, fantasmas: Vec<Fantasma> }` | grafo unificado + sinais |
| `unir_grafos` | `(Vec<GrafoCrate>) -> ResultadoUniao` | pura, determinística (itera `BTreeMap`/`BTreeSet`) |

Exposto em `domain/mod.rs` (`pub mod uniao;`). `raio.rs` e o resto do `lente_core`
intocados — a união é **aditiva**.

### L4 — `04_wiring/src/lib.rs`

- `resolver_colisoes(Grafo) -> Result<Grafo, ErroLente>` — **extraído** de
  `obter_grafo_resolvido` (era inline). Reusado pela montagem do workspace.
- `montar_grafo_workspace(&Path) -> Result<GrafoWorkspace, ErroLente>` — a
  orquestração (enumerar 0044 → versão 0044 (uma vez) → extrair cacheado 0044 por
  membro → `resolver_colisoes` por crate → `unir_grafos`).
- `GrafoWorkspace { grafo, fantasmas }` e `ErroLente::Workspace(ErroWorkspace)` com
  `From` para `?`.

---

## A correção mais importante: o discriminador definição-vs-referência

O prompt (e a seção "Cuidados") dizia: *"um nó é **definição** se o crate do grafo
de onde ele veio é igual ao `no.crate_name`"*. **Essa regra está errada** e foi
**corrigida** na implementação.

`No.crate_name` é o **crate-raiz do grafo**, que o L3 copia para **todos** os nós de
uma extração — inclusive os nós-referência a outros crates e os de sysroot (ver o
doc-comment de `No.crate_name` em `grafo.rs`: *"o valor é igual para todos os nós do
mesmo grafo"*). Logo, para um grafo etiquetado `a`, **todo** nó tem
`no.crate_name == "a"`, e a regra literal marcaria **todos** como definição — a
referência `b::Foo` carregada por `a` seria classificada como definição de `b::Foo`.

O discriminador correto, que a implementação usa, é o **prefixo do path vs a
etiqueta do grafo**:

- nó do grafo `C` é **definição** de `P` ⟺ o 1º segmento de `P` é `C`;
- senão é **referência** (outro crate carregando um nó-referência).

Sem definição **e** dono-membro do workspace → **fantasma**. Path cujo dono **não**
é membro (sysroot, deps externas) → externo legítimo, **nunca** fantasma
(coberto por `paths_externos_nao_sao_fantasmas`).

---

## Semântica da união (o que o 0039/0040/0041 validaram)

- **Definição vence referência.** Para cada path, escolhe-se a definição; as
  referências (idênticas módulo `id`) são descartadas. Empate (ou só referências)
  decide por `melhor_no`: prefere quem tem `position` (a definição completa), depois
  etiqueta, depois `id` — determinístico.
- **Fantasma com representante.** Só-referências de dono-membro → registra
  `Fantasma` e **mantém um nó-representante** para a aresta religar (**0 soltas**,
  laudo 0039).
- **Reindexação por path.** Ids antigos são por-crate e colidem entre crates;
  atribui-se id novo sequencial (0..N) por path **ordenado**, e religam-se as
  arestas pelo `from`/`to` (path é a verdade — laudo 0016).
- **Dedup de arestas** por `(id_from, id_to, relation, uses_kind)`; saída ordenada.
- **Determinismo.** Nenhuma ordem de `HashMap` vaza (a dedup usa `HashSet` só como
  filtro de pertinência; a ordenação final é explícita). `unir_grafos` duas vezes dá
  grafos `==` (`uniao_e_deterministica`).

---

## A consequência L2 que o prompt não previu: a CLI quebrou (e foi consertada)

`ErroLente::Workspace` é variante nova. O `match` **exaustivo** da CLI em
`02_shell/cli/src/erro.rs::traduzir` deixou de compilar (`E0004: non-exhaustive
patterns`). O prompt 0045 só listava `01_core/` e `04_wiring/` como afetados, mas
toda variante de `ErroLente` tem consequência no tradutor L2.

Consertado: nova mensagem de catálogo `ERRO_WORKSPACE` (`"Falha ao montar o grafo de
workspace: {detalhe}"`) e o braço `ErroLente::Workspace(e)` em `traduzir`. O catálogo
dá a moldura, o `Display` do `ErroWorkspace` entra como `{detalhe}` — mesmo padrão
das outras variantes. Sem isso a suíte não compilava.

---

## E2E real (requer fork — `#[ignore]`, rodados e verdes)

```
cargo test -p lente_wiring e2e_montar_grafo_workspace -- --ignored --test-threads=1
  → grafo de workspace: 421 nós, 1436 arestas, 0 fantasmas
  → 2 passed; 0 failed
```

| Critério (prompt) | Resultado |
|---|---|
| Contagem de nós ~ ordem da Arena (~363, laudo 0043) | **421 nós** — mesma ordem de grandeza (assert `> 300`); ver achado abaixo |
| Fantasmas vazio (laudo 0041 — colisões são folhas de raio 0) | **0 fantasmas** ✓ |
| Paths únicos no grafo unido (colisões resolvidas por crate) | ✓ (`Path::from` cru não colide — nomes do 0042) |
| Aresta cross-crate conhecida | ✓ `lente_infra` → `lente_core` presente |
| Cache morno → 2ª chamada rápida | ✓ (`< 5s`; suíte rodou em ~7s com cache já quente) |

### Achado: 421 nós vs ~363 medidos na Arena (0043)

A contagem de produção (**421**) ficou acima do protótipo da Arena (**~363**). Não é
falha — o protótipo media uma versão anterior do pipeline; a diferença plausível vem
da resolução por-crate (renomeações `<Trait>::fmt` do 0042 adicionam paths distintos
onde antes havia colisão) e de variação do fork/sysroot entre as medições. Mesma
ordem de grandeza, **0 fantasmas** (o sinal que importava), e a invariante de paths
únicos vale. Registrado, não escondido.

---

## Refator preserva comportamento (não-regressão)

`resolver_colisoes` é **exatamente** o laço que `obter_grafo_resolvido` já fazia
(detectar colisões → investigar 1º par com `fontes` vazias, E2 em quarentena →
aplicar). `obter_grafo_resolvido` apenas passou a **chamá-lo**. Os 20 testes
não-ignorados do `lente_wiring` (incluindo `pipeline_completo_renomeia_colisao_…`,
`modo_per_no_continua_funcionando_apos_fatoracao`, `verificacao_crucial_…`) são a
guarda e passam — `calcular_raio_de_alvo` inalterado.

---

## Testes da união (inline, `#[cfg(test)] mod tests` — 7 novos, sem fork)

- `definicao_vence_referencia_e_aresta_cross_crate_religa` — A refere B::Foo, B o
  define; UM nó (a definição, com `position`), aresta cross-crate religa, 0 soltas.
- `referencia_sem_definicao_vira_fantasma_com_representante` — fantasma + representante.
- `nos_identicos_para_o_mesmo_path_viram_um_so` — dedup.
- `cadeia_tres_crates_liga_e_zero_soltas` — A→B→C, 0 soltas.
- `uniao_e_deterministica` — `unir_grafos` 2× ⇒ `==`.
- `ids_reindexados_unicos_e_paths_unicos` — ids 0..N, paths únicos.
- `paths_externos_nao_sao_fantasmas` — `core::*` é externo, não fantasma.

---

## Estado da suíte / invariantes

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **242 verdes + 26 ignored, 0 falhas** (era 235+24 no 0044; +7 unit da união, +2 E2E de workspace) |
| `cargo tree -p lente_core` | só o crate — **pureza L1 intacta** (`unir_grafos` só stdlib) |
| `calcular_raio_de_alvo` | **inalterado** (refator só extraiu `resolver_colisoes`) |
| Deps novas | **nenhuma** — L1 e L4 só reusam o que já havia |
| E2E workspace real (fork) | 421 nós / 1436 arestas / **0 fantasmas**; cross-crate ✓; cache morno rápido ✓ |

---

## O que NÃO entrou (conforme o prompt)

- O modo `--diff` (L2) — o próximo passo do produto; este prompt entrega só a
  **fundação** (o grafo de workspace unificado), pronta para ele.
- Qualquer mudança no comportamento da resolução, do cache (0044) ou da extração —
  tudo reusado como está.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Fecha o motor do grafo de workspace. **L1**: `unir_grafos` (pura, `01_core/src/domain/uniao.rs`) une os grafos resolvidos por crate **por path** — definição vence referência, path só-referência de dono-membro vira **fantasma** com representante (0 soltas, 0039), reindexação por path ordenado, arestas religadas e deduplicadas, determinística (`BTreeMap`/`BTreeSet`); `GrafoCrate`/`ResultadoUniao`/`Fantasma`. **Correção do prompt**: o discriminador definição-vs-referência **não** é `etq == no.crate_name` (esse campo é o crate-raiz, igual p/ todo nó do grafo) e sim **prefixo do path vs etiqueta do grafo**. **L4**: extrai `resolver_colisoes` do `obter_grafo_resolvido` (refator que preserva comportamento; `calcular_raio_de_alvo` inalterado) e adiciona `montar_grafo_workspace` (enumera 0044 → extrai cacheado 0044 → resolve por crate → une) + `GrafoWorkspace` + `ErroLente::Workspace`. **L2** (consequência não prevista): a nova variante quebrou o `match` exaustivo da CLI — adicionado `cat::ERRO_WORKSPACE` + braço em `traduzir`. Pureza L1 preservada (`cargo tree -p lente_core` só o crate); sem deps novas. E2E real (fork): 421 nós, 1436 arestas, **0 fantasmas**, aresta cross-crate `lente_infra`→`lente_core`, cache morno rápido. Suíte 242 verdes + 26 ignored, 0 falhas. | `01_core/src/domain/{uniao.rs (novo),mod.rs}`, `04_wiring/src/lib.rs`, `02_shell/cli/src/erro.rs`, `02_shell/catalogo/src/lib.rs`, `00_nucleo/lessons/0045-uniao_grafos_e_orquestracao.md` |
