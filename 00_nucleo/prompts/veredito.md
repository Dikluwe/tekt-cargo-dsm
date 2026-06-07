# Prompt de Nucleação: `veredito` — o veredito da investigação de colisão (entities)
Hash do Código: 37639bdb

**Camada**: L1 — Núcleo. Tipo puro (sem I/O, sem deps externas).
**Unidade**: `01_core/core/src/entities/veredito.rs` (crate `lente_core`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0004-lente_investiga.md`;
ADR `0004` (resolução de colisões de path).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

O **vocabulário central** do resultado de investigar uma **colisão de path** (dois
nós com o mesmo path). Produzido pelo `lente_investiga`, consumido pelo
`lente_resolve`. Mora no `lente_core` (ADR-0004 §5) por ser vocabulário que outros
componentes podem precisar (ex.: relatório ao usuário).

## Comportamento e invariantes

- **`Veredito`** (conclusão da investigação):
  - `MesmoItem` — os dois nós são o mesmo item alcançável por dois caminhos
    (ex.: reexports) → **unificar**.
  - `Distintos { evidencia: Evidencia }` — itens diferentes que o fork agregou no
    mesmo path → **identidades novas** (trabalho do `lente_resolve`).
  - `NaoDeterminado { diagnostico: String }` — a cascata de estratégias esgotou
    sem decidir; `diagnostico` explica o que cada uma tentou.
- **`Evidencia`** (sustenta `Distintos`):
  - `VizinhancaDisjunta { exclusivas_a, exclusivas_b }` — arestas dos dois nós
    desconexas (`compartilhadas == 0`).
  - `ImplDeTraitsDiferentes { traits: (String, String) }` — o fonte expõe dois
    `impl <Trait> for <Tipo>` distintos.

## Restrições (L1 puro)

- Tipo de dados puro: `Debug + Clone + PartialEq + Eq`. Sem I/O, sem dep externa.
- Não decide nada — só **representa** a conclusão; a decisão é do `lente_investiga`.

## Critérios de Verificação

```
Dado Veredito::MesmoItem Então matches! confirma a variante
Dado Veredito::Distintos { evidencia: VizinhancaDisjunta{..} } Então carrega a evidência
Dado Veredito::NaoDeterminado { diagnostico } Então o diagnóstico é legível
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"Veredito","kind":"enum","members":["MesmoItem","Distintos","NaoDeterminado"]},{"name":"Evidencia","kind":"enum","members":["VizinhancaDisjunta","ImplDeTraitsDiferentes"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0059) do veredito de colisão. Código inalterado. | `01_core/core/src/entities/veredito.rs` |
