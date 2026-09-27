# Prompt de Nucleação: `lente_estrutura` — vista de módulos, ciclos e DSM
Hash do Código: 360d0ab1

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

## Agregação detalhada (API aditiva)

`agregar_por_modulo_detalhado(&Grafo) -> AgregacaoDetalhada` explica as células
sem alterar `agregar_por_modulo`, `pesos_modulo_a_modulo` ou suas saídas.

### Evidências e contabilidade

- `dependencias`: uma `DependenciaDetalhada` por par dirigido de IDs de módulos,
  contendo IDs e paths dos módulos, peso e índices das evidências em `grafo.edges`.
- `relacoes`: exatamente uma `RelacaoDetalhada` por registro de entrada, com seu
  índice zero-based no snapshot, cópia integral de `Aresta` (IDs, paths, relação,
  subtipo inclusive ausente) e um `DestinoRelacao` exclusivo:
  - `Agregada`: Uses entre contenedores diferentes, associada a uma célula;
  - `InternaAbsorvida`: Uses entre itens do mesmo contenedor, com ID do módulo;
  - `NaoAgregavel`: motivo tipado. Owns válido é `ContencaoOwns`, pois descreve
    hierarquia e não peso da DSM; pontas sem contenção segura são
    `ContencaoInvalida`, com problemas separados para origem e destino.
- `itens_sem_contenedor`: todos os nós sem contenção estabelecida, mesmo sem
  arestas, com IDs, paths e problemas. Inclui órfãos e hierarquias inválidas.
- Peso significa **número de registros Uses no snapshot** que contribuem para
  aquela célula. Paralelas idênticas contam separadamente, sem deduplicação;
  Reference, Import e subtipo ausente contam igualmente. Não significa número de
  posições de uso, ocorrências sintáticas ou pares distintos de itens. Mantém a
  semântica de `pesos_modulo_a_modulo` nos grafos válidos.
- Soma dos pesos = quantidade de relações `Agregada`. Cada índice de aresta
  ocorre uma única vez em `relacoes`; cada índice agregado ocorre exatamente
  uma vez nas evidências de sua célula. Internas e não agregáveis não contam.

O índice só endereça o vetor recebido nesta chamada. Não identifica uma relação
entre extrações, nem continua válido após permutar o vetor. As evidências não
inventam posição da referência a partir de `No.position`. O rustdoc apresenta
consulta em memória por IDs da célula, seguida de acesso a `grafo.edges[indice]`.

### Contenção, falhas e determinismo

A API exige IDs de nós únicos (invariante central). Usa índices por ID e
memoização da cadeia, sem procurar endpoints linearmente no vetor de nós.
Crate/Mod contém a si próprio; os demais itens usam o módulo ancestral mais
próximo, sem analisar sintaxe de paths. Paths colidentes não unificam IDs.

A nova API valida **toda a ancestralidade Owns**, inclusive acima de módulos.
Pais distintos são ambiguidade mesmo se convergirem ao mesmo módulo. Paralelas
Owns com o mesmo pai não são ambiguidade. SCCs de Owns detectam ciclos e
auto-ciclos; Tarjan é iterativo, usando uma projeção temporária de Owns para a
adjacência esperada pelo helper existente, sem mudar as evidências originais.

Ciclo, múltiplos pais ou ancestral ausente bloqueiam o nó e os descendentes que
alcançam essa obstrução. A política é conservadora: não escolhe um dos pais e
não aceita um módulo só porque foi encontrado antes da obstrução. São publicados
os problemas locais do primeiro ancestral obstruído (ambiguidade e ciclo podem
coexistir), não todos os problemas em todos os caminhos possíveis. Cadeias
válidas independentes continuam agregáveis. Endpoints ausentes são diagnosticados
por ID; ausência de qualquer módulo numa cadeia válida é `SemModulo`.

Células são ordenadas por `(path_de, path_para, id_de, id_para)`; itens sem
contenedor por `(path, id)`; relações por paths, IDs, ordem explícita de Relation
(Owns, Uses) e subtipo (ausente, Reference, Import), usando índice apenas para
empate entre registros idênticos. IDs dos pais e membros dos ciclos são
ordenados numericamente. Não se usa texto Debug como chave. Permutações mantêm
células, pesos, diagnósticos e o multiconjunto de evidências, recalculando seus
índices para o novo snapshot.

A API não devolve outro Grafo, não serializa e não altera a preservação de Owns
entre módulos feita pelo agregador legado. A integração de apresentação fica
para tarefa posterior. Pureza L1: somente stdlib e lente_core.

### Verificação adicional

Testes cobrem várias relações por célula, paralelas, todos os subtipos, relações
internas, órfão isolado e referenciado, endpoints/ancestrais ausentes, múltiplos
pais convergentes, ciclos e auto-ciclos, descendentes inválidos, Owns paralelos,
permutação válida e inválida, cadeia profunda, contabilidade exaustiva e regressão
dos pares/pesos legados em grafos válidos. O exemplo de consulta é um doctest.

Revisão 2026-09-27: saneamento mecânico de Clippy/rustfmt, preservando
semântica e API; qualificação Option::None não é um braço curinga.

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"agregar_por_modulo","params":["&Grafo"],"return_type":"Grafo"},{"name":"pesos_modulo_a_modulo","params":["&Grafo"],"return_type":"HashMap<(usize, usize), usize>"},{"name":"raios_por_modulo","params":["&Grafo"],"return_type":"Vec<RaioModulo>"},{"name":"detectar_ciclos","params":["&Grafo"],"return_type":"Vec<Ciclo>"},{"name":"ordenar_dsm","params":["&Grafo"],"return_type":"OrdemDsm"},{"name":"agregar_por_modulo_detalhado","params":["&Grafo"],"return_type":"AgregacaoDetalhada"}],"types":[{"name":"Ciclo","kind":"struct","members":["modulos"]},{"name":"DependenciaModulo","kind":"struct","members":["de","para","peso"]},{"name":"RaioModulo","kind":"struct","members":["modulo","montante","jusante"]},{"name":"EstruturaModulos","kind":"struct","members":["modulos","dependencias","ciclos","ordem","blocos","raios"]},{"name":"OrdemDsm","kind":"struct","members":["ordem","blocos"]},{"name":"ProblemaContencao","kind":"enum","members":["SemModulo","NoAusente","MultiplosPais","Ciclo"]},{"name":"ItemSemContenedor","kind":"struct","members":["id","path","problemas"]},{"name":"MotivoNaoAgregavel","kind":"enum","members":["ContencaoOwns","ContencaoInvalida"]},{"name":"DestinoRelacao","kind":"enum","members":["Agregada","InternaAbsorvida","NaoAgregavel"]},{"name":"RelacaoDetalhada","kind":"struct","members":["indice_snapshot","aresta","destino"]},{"name":"DependenciaDetalhada","kind":"struct","members":["id_de","de","id_para","para","peso","evidencias"]},{"name":"AgregacaoDetalhada","kind":"struct","members":["dependencias","relacoes","itens_sem_contenedor"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0060) da vista de estrutura. Código inalterado. | `01_core/estrutura/src/lib.rs` |

| 2026-09-26 | API aditiva de agregação detalhada: evidências por snapshot, contabilidade e validação Owns; legado preservado. | `01_core/estrutura/src/lib.rs` |
