# Prompt de Nucleação: `lente_estrutura` — vista de módulos, ciclos e DSM
Hash do Código: 9c6f70d4

**Camada**: L1 — Núcleo. Pureza: stdlib + `lente_core`. Zero externas.
**Unidade**: `01_core/estrutura/src/lib.rs` (crate `lente_estrutura`, arquivo único).
**Origem de trabalho** (referência): `00_nucleo/prompt/0031-estrutura-modulo-ciclos.md`
(+ 0035 DSM); recebeu `EstruturaModulos`/`DependenciaModulo` no Estágio 2 (0056).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **vista global** estilo Lattix/Structure101: agrega o grafo de itens ao nível de
**módulo**, detecta **ciclos** (SCCs ≥ 2) e ordena a **DSM** (matriz como dado).
Genérico — opera sobre qualquer `Grafo` (fractal: item/módulo/crate).

## Comportamento e invariantes

- **`agregar_por_modulo(grafo) -> Grafo`**: itens → módulos (pelo contenedor
  `Owns`); `Uses` viram dependências módulo→módulo; `uses` intra-módulo absorvidos.
- **`detectar_ciclos(grafo) -> Vec<Ciclo>`**: SCC à mão (Tarjan, sem `petgraph`)
  sobre as `Uses`; devolve SCCs ≥ 2; `Ciclo.modulos` ordenado lexicograficamente;
  ordem entre ciclos determinística.
- **`ordenar_dsm(grafo) -> OrdemDsm`**: ordem topológica da condensação dos SCCs
  (`ordem`) + os `blocos` (SCCs ≥ 2, intervalos contíguos) — "DSM como dado".
- **Tipos**: `Ciclo` (módulos do SCC), `OrdemDsm` (`ordem`/`blocos`),
  `DependenciaModulo` (`de`/`para`, do 0056), `EstruturaModulos`
  (`modulos`/`dependencias`/`ciclos`/`ordem`/`blocos`, do 0056) — o resultado
  completo do modo `--estrutura`.
- **Determinístico**; **puro** (devolve novo, não muta).

## Restrições (L1 puro)

- Só stdlib + `lente_core`. Sem petgraph, sem dep externa, sem I/O.

## Critérios de Verificação

```
Dado t::a::f usa t::b::g e t::b::g usa t::a::f Quando detectar_ciclos Então o ciclo {t::a,t::b}
Dado o grafo de itens Quando agregar_por_modulo Então arestas módulo→módulo (intra absorvido)
Dado uma condensação Quando ordenar_dsm Então ordem topológica + blocos dos SCCs
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"agregar_por_modulo","params":["&Grafo"],"return_type":"Grafo"},{"name":"pesos_modulo_a_modulo","params":["&Grafo"],"return_type":"HashMap<(usize, usize), usize>"},{"name":"raios_por_modulo","params":["&Grafo"],"return_type":"Vec<RaioModulo>"},{"name":"detectar_ciclos","params":["&Grafo"],"return_type":"Vec<Ciclo>"},{"name":"ordenar_dsm","params":["&Grafo"],"return_type":"OrdemDsm"}],"types":[{"name":"Ciclo","kind":"struct","members":["modulos"]},{"name":"DependenciaModulo","kind":"struct","members":["de","para","peso"]},{"name":"RaioModulo","kind":"struct","members":["modulo","montante","jusante"]},{"name":"EstruturaModulos","kind":"struct","members":["modulos","dependencias","ciclos","ordem","blocos","raios"]},{"name":"OrdemDsm","kind":"struct","members":["ordem","blocos"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0060) da vista de estrutura. Código inalterado. | `01_core/estrutura/src/lib.rs` |
