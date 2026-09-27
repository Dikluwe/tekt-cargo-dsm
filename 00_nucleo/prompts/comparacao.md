# Prompt de Nucleação: `lente_comparacao` — paridade entre duas estruturas (L1)
Hash do Código: e27d942e

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

- **Pareamento legado** por **path normalizado na raiz do crate** (descarta o 1º segmento):
  `velho::nucleo::raio` pareia com `novo::nucleo::raio`. O que não casa é **sem-par
  dos dois lados** — um módulo movido (`a::b`→`c::b`) normaliza diferente e **não** é
  adivinhado (teste-contrato). Sem heurística de similaridade, **sem nota única**.
- **Deltas** (entre módulos pareados): `arestas_comuns` (peso de cada lado),
  `arestas_so_antes` (sumiram), `arestas_so_depois` (apareceram); `pareados`,
  `sem_par_antes`/`sem_par_depois`; `ciclos_antes`/`ciclos_depois` (quantidade +
  maior SCC). Tudo determinístico (ordenado por path).
- **Entrada com parâmetros idênticos**: o L4 garante mesmo escopo/modo nos dois lados;
  esta peça só compara os dados já extraídos.
- **Mapa declarativo opcional (P0079)**: depois do censo K4, aplica regras de
  prefixo e correspondências exatas N:M. Correspondência explícita precede
  prefixo, que precede o pareamento automático residual. Conflitos, paths
  inexistentes e cardinalidade incompatível permanecem `unknown`, com
  diagnóstico; a ordem do TOML não decide conflitos.
- **Alegação não é prova**: `declarada-fechada`, `parcial`, `ausente`,
  `fora-de-escopo` e `unknown` são dados do autor do mapa. A lente valida apenas
  sua consistência mecânica e nunca publica alegação como veredito funcional.

## Restrições (L1)

- **Pureza**: stdlib + `lente_core` + `lente_estrutura`. Sem I/O, sem deps externas.
  Determinístico e testável com estruturas forjadas.

## Critérios de Verificação

```
Dado o crate renomeado entre os lados Então os módulos pareiam pela normalização
Dado um módulo movido (a::b → c::b) Então sem-par dos dois lados (teste-contrato)
Dado uma aresta que muda de peso Então arestas_comuns com peso_antes/peso_depois
Dado um ciclo desfeito Então ciclos_depois.quantidade < ciclos_antes.quantidade
Dado mapa ausente Então o resultado anterior é preservado
Dado correspondência 1:N válida Então seus paths saem do resíduo e a relação é aplicada
Dado path inexistente ou conflito Então a relação fica unknown com diagnóstico
```

## Identidades explícitas (API aditiva)

`comparar_estruturas_por_identidade(antes, depois, identidades_antes,
identidades_depois) -> ComparacaoIdentidades` recebe duas `EstruturaModulos` e
mapas `BTreeMap<Path, String>`. Cada path original endereça um módulo no seu
próprio lado; o valor é a chave de pareamento fornecida pelo chamador. Não há
leitura de IDs da extração nem promessa automática de identidade persistente.
O chamador define a política entre versões e garante escopo e granularidade
comparáveis. A função não traduz grafos nem decide compatibilidade semântica.

- Chaves são opacas: igualdade exata, sem interpretar `/`, `.`, `::`, caixa ou
  normalização Unicode. A string vazia é válida; ausência é falta no mapa.
- Um par exige exatamente um candidato de cada lado. Chave repetida em qualquer
  lado gera `Repetida` com todos os paths envolvidos e bloqueia o grupo inteiro,
  inclusive se não houver candidatos no outro lado. Não há escolha arbitrária.
- Módulo sem entrada gera `Ausente`; entrada referente a path não listado gera
  `PathNaoListado` e é ignorada. Nunca se usa o path como fallback.
- `ParIdentidade` separa chave e paths originais antes/depois. Resíduos incluem
  todos os módulos não pareados, por ausência, ambiguidade ou chave unilateral.
- `ArestaPorIdentidade` conserva os dois paths de cada endpoint e os pesos
  opcionais por lado. `None` é ausência, diferente de `Some(0)`. Somente arestas
  entre pares inequívocos entram na comparação; as incidentes a resíduos são
  excluídas. Resumos de ciclos continuam abrangendo toda a estrutura fornecida.
- Entrada respeita `EstruturaModulos`: paths únicos e dependências agregadas
  (um peso por par dirigido). Pares e arestas saem ordenados por identidade;
  resíduos e candidatos por path; diagnósticos por fase/lado e chave ordenada.
  Permutar as listas de uma entrada válida não muda o resultado.
- Não há heurística de similaridade, equivalência comportamental, I/O ou novas
  dependências. APIs legadas, inclusive normalização Rust e mapa P0079, preservam
  suas assinaturas e comportamentos. Adaptadores e interfaces não adotam a API
  automaticamente. O rustdoc contém exemplo executável inteiramente em memória.

Verificação adicional: paths diferentes com mesma chave; paths iguais ou
semelhantes com chaves distintas; separadores e Unicode; duplicidade unilateral
ou bilateral; ausências e entradas excedentes; pesos e paths preservados;
permutação; regressões dos modos legados.

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"comparar_estruturas","params":["&EstruturaModulos","&EstruturaModulos","&str","&str","ChavePareamento","Proveniencia","ComparacaoItens"],"return_type":"Comparacao"},{"name":"comparar_estruturas_com_mapa","params":["&EstruturaModulos","&EstruturaModulos","&str","&str","ChavePareamento","Proveniencia","ComparacaoItens","Option<&MapaCorrespondencia>"],"return_type":"Comparacao"},{"name":"comparar_itens","params":["&Grafo","&BTreeSet<String>","&Grafo","&BTreeSet<String>"],"return_type":"ComparacaoItens"},{"name":"aplicar_mapa","params":["&mut ComparacaoItens","&MapaCorrespondencia","&Grafo","&Grafo"],"return_type":null},{"name":"comparar_estruturas_por_identidade","params":["&EstruturaModulos","&EstruturaModulos","&BTreeMap<Path, String>","&BTreeMap<Path, String>"],"return_type":"ComparacaoIdentidades"}],"types":[{"name":"ArestaComparada","kind":"struct","members":["de","para","peso_antes","peso_depois"]},{"name":"ResumoCiclos","kind":"struct","members":["quantidade","maior"]},{"name":"Lado","kind":"enum","members":["Antes","Depois"]},{"name":"ChavePareamento","kind":"enum","members":["Normalizada","PathCompleto"]},{"name":"Proveniencia","kind":"struct","members":["modo_antes","modo_depois","crates_antes","crates_depois","fantasmas_antes","fantasmas_depois","falhas_antes","falhas_depois","third_party_antes","third_party_depois"]},{"name":"ItemPareado","kind":"struct","members":["kind","trait_","nome_qualificado","path_antes","path_depois"]},{"name":"ItemAmbiguo","kind":"struct","members":["kind","trait_","nome_qualificado","candidatos_antes","candidatos_depois"]},{"name":"ItemSemPar","kind":"struct","members":["kind","trait_","nome_qualificado","path"]},{"name":"RegraPrefixo","kind":"struct","members":["antes","depois"]},{"name":"RelacaoDeclarada","kind":"enum","members":["UmParaUm","Divisao","Consolidacao","Grupo"]},{"name":"AlegacaoDeclarada","kind":"enum","members":["DeclaradaFechada","Parcial","Ausente","ForaDeEscopo","Unknown"]},{"name":"CorrespondenciaDeclarada","kind":"struct","members":["id","antes","depois","relacao","alegacao","evidencias","nota"]},{"name":"MapaCorrespondencia","kind":"struct","members":["caminho","sha256","prefixos","correspondencias"]},{"name":"CorrespondenciaAplicada","kind":"struct","members":["id","antes","depois","relacao","alegacao","evidencias","nota","estado"]},{"name":"ResultadoMapa","kind":"struct","members":["caminho","sha256","regras_prefixo","prefixos_aplicados","correspondencias","diagnosticos"]},{"name":"ComparacaoItens","kind":"struct","members":["pareados","ambiguos","sem_par_antes","sem_par_depois","mapa"]},{"name":"Comparacao","kind":"struct","members":["nome_antes","nome_depois","pareados","sem_par_antes","sem_par_depois","arestas_comuns","arestas_so_antes","arestas_so_depois","ciclos_antes","ciclos_depois","chave","proveniencia","itens"]},{"name":"ParIdentidade","kind":"struct","members":["identidade","path_antes","path_depois"]},{"name":"ArestaPorIdentidade","kind":"struct","members":["de","para","peso_antes","peso_depois"]},{"name":"DiagnosticoIdentidade","kind":"enum","members":["Ausente","Repetida","PathNaoListado"]},{"name":"ComparacaoIdentidades","kind":"struct","members":["pareados","sem_par_antes","sem_par_depois","arestas","diagnosticos","ciclos_antes","ciclos_depois"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Nucleação (prompt 0074) da peça de paridade: pareamento por path normalizado (crate renomeado pareia; movido = sem-par dos dois lados, teste-contrato) + deltas de arestas/peso/ciclos entre pareados. Puro, sem deps novas. | `01_core/comparacao/src/lib.rs`, `01_core/comparacao/Cargo.toml` |

| 2026-09-26 | API aditiva de identidades opacas, diagnósticos, deltas com paths originais e exemplo em memória; legado preservado. | `01_core/comparacao/src/lib.rs` |
