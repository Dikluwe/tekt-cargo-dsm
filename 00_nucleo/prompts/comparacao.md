# Prompt de Nucleação: `lente_comparacao` — paridade entre duas estruturas (L1)
Hash do Código: 06186d23

**Camada**: L1 — Núcleo (puro). Importa `lente_core` e `lente_estrutura`.
**Unidade**: `01_core/comparacao/src/lib.rs` (crate `lente_comparacao`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0074-paridade_como_dado.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

**Paridade como dado** (prompt 0074): compara **duas** [`EstruturaModulos`] (antes
e depois de uma refatoração) e devolve o que parou, o que só existe de um lado, e
como arestas, pesos e ciclos mudaram entre os pares. É cálculo puro — a tela lado a
lado (prompt seguinte) consome este dado.

## Comportamento e invariantes

- **Pareamento** por **path normalizado na raiz do crate** (descarta o 1º segmento):
  `velho::nucleo::raio` pareia com `novo::nucleo::raio`. O que não casa é **sem-par
  dos dois lados** — um módulo movido (`a::b`→`c::b`) normaliza diferente e **não** é
  adivinhado (teste-contrato). Sem heurística de similaridade, **sem nota única**.
- **Deltas** (entre módulos pareados): `arestas_comuns` (peso de cada lado),
  `arestas_so_antes` (sumiram), `arestas_so_depois` (apareceram); `pareados`,
  `sem_par_antes`/`sem_par_depois`; `ciclos_antes`/`ciclos_depois` (quantidade +
  maior SCC). Tudo determinístico (ordenado por path).
- **Entrada com parâmetros idênticos**: o L4 garante mesmo escopo/modo nos dois lados;
  esta peça só compara os dados já extraídos.

## Restrições (L1)

- **Pureza**: stdlib + `lente_core` + `lente_estrutura`. Sem I/O, sem deps externas.
  Determinístico e testável com estruturas forjadas.

## Critérios de Verificação

```
Dado o crate renomeado entre os lados Então os módulos pareiam pela normalização
Dado um módulo movido (a::b → c::b) Então sem-par dos dois lados (teste-contrato)
Dado uma aresta que muda de peso Então arestas_comuns com peso_antes/peso_depois
Dado um ciclo desfeito Então ciclos_depois.quantidade < ciclos_antes.quantidade
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"comparar_estruturas","params":["&EstruturaModulos","&EstruturaModulos","&str","&str","ChavePareamento","Proveniencia","ComparacaoItens"],"return_type":"Comparacao"},{"name":"comparar_itens","params":["&Grafo","&BTreeSet<String>","&Grafo","&BTreeSet<String>"],"return_type":"ComparacaoItens"}],"types":[{"name":"ArestaComparada","kind":"struct","members":["de","para","peso_antes","peso_depois"]},{"name":"ResumoCiclos","kind":"struct","members":["quantidade","maior"]},{"name":"Lado","kind":"enum","members":["Antes","Depois"]},{"name":"ChavePareamento","kind":"enum","members":["Normalizada","PathCompleto"]},{"name":"Proveniencia","kind":"struct","members":["modo_antes","modo_depois","crates_antes","crates_depois","fantasmas_antes","fantasmas_depois","falhas_antes","falhas_depois","third_party_antes","third_party_depois"]},{"name":"ItemPareado","kind":"struct","members":["kind","trait_","nome_qualificado","path_antes","path_depois"]},{"name":"ItemAmbiguo","kind":"struct","members":["kind","trait_","nome_qualificado","candidatos_antes","candidatos_depois"]},{"name":"ItemSemPar","kind":"struct","members":["kind","trait_","nome_qualificado","path"]},{"name":"ComparacaoItens","kind":"struct","members":["pareados","ambiguos","sem_par_antes","sem_par_depois"]},{"name":"Comparacao","kind":"struct","members":["nome_antes","nome_depois","pareados","sem_par_antes","sem_par_depois","arestas_comuns","arestas_so_antes","arestas_so_depois","ciclos_antes","ciclos_depois","chave","proveniencia","itens"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Nucleação (prompt 0074) da peça de paridade: pareamento por path normalizado (crate renomeado pareia; movido = sem-par dos dois lados, teste-contrato) + deltas de arestas/peso/ciclos entre pareados. Puro, sem deps novas. | `01_core/comparacao/src/lib.rs`, `01_core/comparacao/Cargo.toml` |
