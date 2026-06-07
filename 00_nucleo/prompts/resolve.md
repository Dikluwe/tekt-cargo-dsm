# Prompt de Nucleação: `lente_resolve` — aplicar o veredito (escada de nomeação)
Hash do Código: 5e5c2f9d

**Camada**: L1 — Núcleo. Pureza: stdlib + `lente_core`. Zero externas.
**Unidade**: `01_core/resolve/src/lib.rs` (crate `lente_resolve`, arquivo único).
**Origem de trabalho** (referência): `00_nucleo/prompt/0010-lente_resolve-v2.md`
(+ 0042 escada `trait_ref`); ADR `0006` (nomeação trait-padrão).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

**Aplicar** o veredito de uma colisão ao grafo: quando o `lente_investiga` conclui
`Distintos`, dar **identidades novas** aos nós colididos por uma **escada de
nomeação determinística** (ADR-0006), preservando a integridade referencial.

## Comportamento e invariantes

- **`aplicar(grafo, path_colidente, veredito) -> Result<Grafo, ErroResolve>`**:
  - `MesmoItem` → unifica.
  - `Distintos` → reescreve os paths dos nós colididos pela **escada**:
    - degrau 1 — insere `<trait_>` antes do último segmento;
    - degrau 2 — se 2+ ficam com o mesmo nome (mesmo `trait_`), reescreve **esses**
      por `<trait_ref>`;
    - degrau 3 — desempata por contador.
    Religa as arestas aos paths novos (0 soltas).
  - `NaoDeterminado` → `Err(ErroResolve::…)` (não inventa).
- **`ErroResolve`** — falha da resolução (colisão não-resolvida / inconsistência);
  `Display` + `Error`.
- **Puro**, determinístico (devolve grafo novo).

## Restrições (L1 puro)

- Só stdlib + `lente_core`. Sem I/O, sem dep externa.

## Critérios de Verificação

```
Dado dois nós t::T::fmt (Display e Debug) Quando aplicar(Distintos) Então t::T::<Display>::fmt e t::T::<Debug>::fmt
Dado a reescrita Então as arestas religam aos paths novos (0 soltas)
Dado NaoDeterminado Então Err(ErroResolve)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"aplicar","params":["&Grafo","&Path","&Veredito"],"return_type":"Result<Grafo, ErroResolve>"}],"types":[{"name":"ErroResolve","kind":"enum","members":["ColisaoNaoResolvida","ColisaoInexistente","IdInconsistente"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0060) da aplicação do veredito. Código inalterado. | `01_core/resolve/src/lib.rs` |
