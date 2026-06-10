# Prompt de Nucleação: `lente_wiring` — a fiação (L4)
Hash do Código: 80cfa53f

**Camada**: L4 — Fiação (composição). Importa L1/L2/L3 (é o topo, compõe tudo).
**Unidade**: `04_wiring/src/lib.rs` (crate `lente_wiring`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0019-l4-wiring.md` (+ 0027/
0030/0031/0034/0045/0047 — os pipelines).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **composição** dos pipelines da lente, ponta a ponta: extrai/recebe o grafo,
resolve colisões, aplica o escopo, e responde cada modo (per-nó / ranking /
estrutura / diff / grafo de workspace). **Não** formata nem lê argumentos (isso é
L2); **não** define vocabulário de pedido (desceu ao L1 no Estágio 2) — só **fia**
as camadas abaixo.

## Comportamento e invariantes

- **Pipelines** (`pub fn`):
  - `calcular_raio_de_alvo(fonte, alvo, escopo) -> Result<Raio, ErroLente>`.
  - `rankear_pacote(fonte, n, escopo) -> Result<Vec<ItemRanking>, _>`.
  - `analisar_estrutura(fonte, escopo, modo_uses) -> Result<EstruturaModulos, _>`.
  - `montar_grafo_workspace(raiz) -> Result<GrafoWorkspace, _>` (enumera→extrai
    cacheado→resolve por crate→une).
  - `analisar_diff(raiz) -> Result<ResultadoDiff, _>` (canonicaliza a raiz; ler_diff
    + grafo de workspace + mapear_diff + raio por tocado).
- **`GrafoWorkspace`** (`pub struct`: `grafo` + `fantasmas`).
- **`ErroLente`** (`pub enum`) — o **erro agregado da composição** (ver abaixo).
- **Re-exports**: o vocabulário L1 (`Escopo`/`ModoUses`/`FonteGrafo`/`AlvoBusca` de
  `consulta`; `ResultadoDiff`/`TocadoComRaio`/`RaioCombinado`/`combinar_raios`;
  `Fantasma`; `Ciclo`/`OrdemDsm`/`DependenciaModulo`/`EstruturaModulos`;
  `ItemRanking`) — para os consumidores do fio.

## `ErroLente` — residência no L4 por desígnio (V12 = 1 intencional)

`ErroLente` **agrega** os erros das camadas internas via `From`: `Fork`/
`Adaptador`/`Workspace`/`Diff` (do **L3**) e `Resolucao`/`Raio` (do L1), mais
`IdInexistente`/`ForkSemUsesKind`. **É um erro de composição** — só na fiação, onde
L1 e L3 se encontram, faz sentido juntá-los. **Não desce ao L1** (faria o L1
referenciar o L3). Por isso o **V12 = 1 sobre o `ErroLente` é intencional e
aceito** — um `enum` no L4 que é legítimo (não é "L4 criando tipo de domínio";
é o tipo-soma dos erros que a composição propaga com `?`).

## Restrições (L4)

- O **topo**: importa L1/L2/L3 (depende de tudo abaixo). Não há camada acima.
  V3 = 0 por construção.

## Critérios de Verificação

```
Dado um JSON com colisão Quando calcular_raio_de_alvo Então o raio do alvo resolvido
Dado a raiz do repo Quando montar_grafo_workspace Então o grafo unificado + fantasmas
Dado um ErroFork Quando ? num pipeline Então ErroLente::Fork (agregação via From)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"calcular_raio_de_alvo","params":["FonteGrafo","AlvoBusca","Escopo"],"return_type":"Result<Raio, ErroLente>"},{"name":"montar_grafo_workspace","params":["&std::path::Path"],"return_type":"Result<GrafoWorkspace, ErroLente>"},{"name":"analisar_diff","params":["&std::path::Path"],"return_type":"Result<ResultadoDiff, ErroLente>"},{"name":"rankear_pacote","params":["FonteGrafo","usize","Escopo"],"return_type":"Result<Vec<ItemRanking>, ErroLente>"},{"name":"analisar_estrutura","params":["FonteGrafo","Escopo","ModoUses"],"return_type":"Result<EstruturaModulos, ErroLente>"},{"name":"comparar","params":["&std::path::Path","&std::path::Path","Escopo","ModoUses"],"return_type":"Result<Comparacao, ErroComparar>"}],"types":[{"name":"ErroLente","kind":"enum","members":["Fork","Adaptador","Resolucao","Raio","IdInexistente","ForkSemUsesKind","Workspace","Diff"]},{"name":"GrafoWorkspace","kind":"struct","members":["grafo","fantasmas"]},{"name":"ErroComparar","kind":"struct","members":["lado","erro"]}],"reexports":["lente_core::domain::resultado_diff::{\n    RaioCombinado, ResultadoDiff, TocadoComRaio, combinar_raios,\n}","lente_core::domain::consulta::{AlvoBusca, Escopo, FonteGrafo, ModoUses}","lente_core::domain::uniao::Fantasma","lente_estrutura::{Ciclo, DependenciaModulo, EstruturaModulos, OrdemDsm}","lente_ranking::ItemRanking","lente_comparacao::{ArestaComparada, Comparacao, Lado, ResumoCiclos}"]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0065) da fiação. `ErroLente` documentado como erro de composição L4 (V12=1 intencional). Código inalterado. | `04_wiring/src/lib.rs` |
