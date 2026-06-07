# Prompt de Nucleação: `lente_investiga` — investigar uma colisão de path
Hash do Código: 11871840

**Camada**: L1 — Núcleo. Pureza: stdlib + `lente_core`. Zero externas.
**Unidade**: `01_core/investiga/src/lib.rs` (crate `lente_investiga`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0004-lente_investiga.md`;
ADR `0004` (resolução de colisões de path).

> Prompt de **nucleação** (descreve o código existente). Os módulos internos
> `vizinhanca` (helper `pub(crate)`) e `fontes` (E2 **em quarentena**, laudo 0014)
> não têm interface pública — ficam fora da nucleação (`[excluded_files]`).

---

## Propósito

**Investigar** uma colisão de path (dois nós no mesmo path) e produzir um
[`Veredito`] (`lente_core`): mesmo item, distintos (com evidência), ou
não-determinado. É a etapa de **diagnóstico** antes da resolução (`lente_resolve`).

## Comportamento e invariantes

- **`investigar(par, vizinhanca, fontes) -> Veredito`** — uma **cascata de
  estratégias** que para na primeira conclusiva:
  - **Vizinhança disjunta**: se as arestas dos dois nós são desconexas →
    `Distintos { VizinhancaDisjunta }`.
  - **Impl de traits diferentes**: se o descritor mostra dois `impl <Trait>`
    distintos → `Distintos { ImplDeTraitsDiferentes }`.
  - Esgotou → `NaoDeterminado { diagnostico }`.
- **Tipos públicos**: `ParColidente` (o par de nós), `ArestasNo`
  (entrando/saindo de um nó), `Vizinhanca` (as `ArestasNo` dos dois), `ArquivoFonte`
  (fonte opcional para a E2 quarentenada).
- **E2 (`fontes`) em quarentena**: o parâmetro `fontes` existe mas a estratégia de
  parsing textual está desligada (laudo 0014; o fork 0.27.0 emite `trait` por nó,
  tornando-a desnecessária) — `fontes` é sempre `None` na fiação.
- **Puro**, determinístico.

## Restrições (L1 puro)

- Só stdlib + `lente_core`. Sem I/O, sem dep externa.

## Critérios de Verificação

```
Dado um par com vizinhança disjunta Quando investigar Então Distintos{VizinhancaDisjunta}
Dado um par com dois impl de traits distintos Então Distintos{ImplDeTraitsDiferentes}
Dado nada conclusivo Então NaoDeterminado com diagnóstico
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"investigar","params":["ParColidente<'_>","&Vizinhanca","Option<&[ArquivoFonte]>"],"return_type":"Veredito"}],"types":[{"name":"ParColidente","kind":"struct","members":["a","b"]},{"name":"ArestasNo","kind":"struct","members":["entrando","saindo"]},{"name":"Vizinhanca","kind":"struct","members":["a","b"]},{"name":"ArquivoFonte","kind":"struct","members":["caminho_logico","conteudo"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0060) da investigação de colisão (lib.rs). `vizinhanca`/`fontes` internos excluídos. Código inalterado. | `01_core/investiga/src/lib.rs` |
