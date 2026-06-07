# Prompt de Nucleação: `lente_filtro` — esconder o ruído (filtros do grafo)
Hash do Código: 990c2b6b

**Camada**: L1 — Núcleo. Pureza: stdlib + `lente_core`. Zero externas.
**Unidade**: `01_core/filtro/src/lib.rs` (crate `lente_filtro`, arquivo único).
**Origem de trabalho** (referência): `00_nucleo/prompt/0025-l1-filtro-stdlib.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

Dois **filtros puros** sobre o `Grafo`, que escondem ruído sem mudar a topologia
do que sobra: a stdlib (sysroot) e as arestas `Uses` de import.

## Comportamento e invariantes

- **`filtrar_stdlib(grafo) -> Grafo`**: esconde os nós de **sysroot** (`core::*`/
  `std::*`/`alloc::*`/…) por **prefixo do path** (ADR-0002 D3), e as arestas
  incidentes. **Limite 2 da spec**: preserva os `impl` do **crate-alvo** mesmo
  quando o trait é da stdlib (a fronteira delicada). Não remove nós do código do
  usuário.
- **`filtrar_so_referencia(grafo) -> Grafo`**: mantém só as arestas `Uses` cujo
  `uses_kind == Reference` (uso de tipo direto); descarta `Import` (**Limite 4**).
  Arestas `Owns` e nós ficam.
- **Pureza**: devolve um `Grafo` novo; não muta a entrada; determinístico.

## Restrições (L1 puro)

- Só stdlib + `lente_core`. Sem I/O, sem dep externa.

## Critérios de Verificação

```
Dado um grafo com core::fmt::Display Quando filtrar_stdlib Então o nó sysroot some
Dado um impl do crate-alvo de um trait da stdlib Então é preservado (Limite 2)
Dado arestas Uses Import e Reference Quando filtrar_so_referencia Então só as Reference ficam
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"filtrar_stdlib","params":["&Grafo"],"return_type":"Grafo"},{"name":"filtrar_so_referencia","params":["&Grafo"],"return_type":"Grafo"}],"types":[],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0060) dos filtros. Código inalterado. | `01_core/filtro/src/lib.rs` |
