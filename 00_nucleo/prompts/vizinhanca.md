# Prompt de Nucleação: `vizinhanca` — Estratégia 1 da investigação (interno)
Hash do Código: cb3f1f6f

**Camada**: L1 — Núcleo. Pureza: stdlib + `lente_core`. Zero externas.
**Unidade**: `01_core/investiga/src/vizinhanca.rs` (módulo **interno** `pub(crate)`
do crate `lente_investiga`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0004-lente_investiga.md`;
ADR `0004`.

> Prompt de **nucleação mínimo** de um interno de lógica L1. Nada é `pub`
> cross-crate (só `pub(crate)`), então o Interface Snapshot é **vazio** — mas o
> arquivo é **nucleado** (não excluído) para que a guarda de pureza (V4/V13) o
> continue checando (decisão do laudo 0061: `[excluded_files]` é exclusão total).

---

## Propósito

A **Estratégia 1** da cascata do `investigar` (`lente_investiga`): decidir uma
colisão de path comparando a **vizinhança** dos dois nós no grafo — os conjuntos
de arestas de cada um. Critério **categórico** (sem thresholds mágicos).

## Comportamento e invariantes

- **`analisar(a, b) -> ResultadoVizinhanca`** (`pub(crate)`):
  - **Disjuntos** (`compartilhadas == 0` e ambos com ≥1 exclusiva) →
    `Decidiu(Veredito::Distintos { VizinhancaDisjunta })`.
  - **Idênticos** (zero exclusivas dos dois lados, ≥1 compartilhada) →
    `Decidiu(Veredito::MesmoItem)`.
  - **Resto** (sobreposição parcial, ou ambos vazios) →
    `Inconclusivo { exclusivas_a, exclusivas_b, compartilhadas }` — a cascata passa
    à Estratégia 2.
- **Identidade de aresta por `(id_from, id_to, relation)`** — **ids**, não paths:
  cópias distintas de um mesmo path (caso `Display+Debug`) têm `id_to` diferentes;
  comparar por path colapsaria-as e a vizinhança pareceria idêntica (regressão da
  remedição §6).

## Restrições (L1 puro)

- Só `std::collections::HashSet` + `lente_core`. **Sem I/O** (`std::fs`/`io`/`net`),
  **sem estado global mutável** (`static mut`) — a guarda V4/V13 o verifica.

## Critérios de Verificação

```
Dado vizinhanças disjuntas Quando analisar Então Distintos{VizinhancaDisjunta}
Dado vizinhanças idênticas Então MesmoItem
Dado sobreposição parcial (ou ambos vazios) Então Inconclusivo
Dado cópias distintas (mesmo path, id_to diferente) Então Distintos (não MesmoItem)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"analisar","params":["&ArestasNo","&ArestasNo"],"return_type":"ResultadoVizinhanca"}],"types":[{"name":"ResultadoVizinhanca","kind":"enum","members":["Decidiu","Inconclusivo"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação mínima (prompt 0062): o interno `vizinhanca` sai do `[excluded_files]` e ganha prompt+cabeçalho para voltar à guarda de pureza (laudo 0061). Lógica inalterada. | `01_core/investiga/src/vizinhanca.rs` |
