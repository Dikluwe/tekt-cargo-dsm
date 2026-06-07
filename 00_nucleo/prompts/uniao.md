# Prompt de Nucleação: `uniao` — a união de grafos por path (domain)
Hash do Código: 46fdae35

**Camada**: L1 — Núcleo. Apenas stdlib. Sem I/O. Sem deps externas.
**Unidade**: `01_core/core/src/domain/uniao.rs` (crate `lente_core`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0045-uniao_grafos_e_orquestracao.md`;
ADR `0002`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

Unir os grafos **já resolvidos por crate** num **grafo de workspace** único,
casando nós **por path** (não por id — instável entre extrações). É a fundação do
motor multi-crate.

## Comportamento e invariantes

- **`unir_grafos(Vec<GrafoCrate>) -> ResultadoUniao`**:
  - **Definição vence referência**: para cada path, o nó cujo **1º segmento do
    path == etiqueta do crate** é a definição (vence); referências (idênticas
    módulo `id`) são descartadas. (O discriminador é prefixo-do-path vs etiqueta,
    **não** `no.crate_name`, que é igual p/ todo nó do grafo.)
  - **Fantasma**: path só com referências cujo dono **é membro** do workspace →
    `Fantasma { path, referenciado_por }` + mantém um **nó-representante** (0
    arestas soltas). Path de dono não-membro (sysroot/externo) → não é fantasma.
  - **Reindexação**: ids novos sequenciais por path **ordenado**; arestas
    religadas pelo `from`/`to` (path), dedup por `(from,to,relation,uses_kind)`.
  - **Determinística**: itera `BTreeMap`/`BTreeSet`; unir 2× dá o mesmo grafo.
- **`GrafoCrate`** (grafo + etiqueta), **`Fantasma`** (path + `referenciado_por`
  ordenado), **`ResultadoUniao`** (grafo + fantasmas).

## Restrições (L1 puro)

- Só stdlib (`HashMap`/`Vec`/`BTreeMap`); sem petgraph, sem dep externa.

## Critérios de Verificação

```
Dado A referencia B::Foo e B o define Quando unir_grafos Então UM nó (a definição); aresta religa
Dado A referencia b::Foo mas B não o tem Então fantasma com representante (0 soltas)
Dado core::* referenciado Então externo (não fantasma)
Dado o mesmo conjunto 2× Então grafos iguais (determinístico)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"unir_grafos","params":["Vec<GrafoCrate>"],"return_type":"ResultadoUniao"}],"types":[{"name":"GrafoCrate","kind":"struct","members":["crate_name","grafo"]},{"name":"Fantasma","kind":"struct","members":["path","referenciado_por"]},{"name":"ResultadoUniao","kind":"struct","members":["grafo","fantasmas"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0059) da união por path. Código inalterado. | `01_core/core/src/domain/uniao.rs` |
