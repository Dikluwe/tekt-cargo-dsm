# Variantes de configuração — experimento independente

Compara observações de dependências sem afirmar que sua união ocorreu em uma
configuração real. Lê grafos já existentes, declarações de configuração e mapas
explícitos de identidade. Não executa builds, extratores ou código dos projetos.

Tudo vive neste workspace Cargo independente. Não altera adaptadores, tipos,
workspace principal, DSM ou harness. Usa apenas o desserializador atual do L3 e
os algoritmos existentes de SCC/ordenação de `lente_estrutura`; não usa a API de
comparação em desenvolvimento. Contrato normativo: [CONTRATO.md](CONTRATO.md).

## Uso

Requer Rust ≥1.91 e dependências já disponíveis para build offline. A partir da
raiz do repositório:

```sh
cargo build --offline --locked --manifest-path lab/variantes-configuracao/Cargo.toml
lab/variantes-configuracao/target/debug/lente-variantes-configuracao \
  lab/variantes-configuracao/fixtures/union-only.json --format text
lab/variantes-configuracao/target/debug/lente-variantes-configuracao \
  lab/variantes-configuracao/fixtures/union-only.json --format json
```

JSON é o formato padrão. stdout contém somente o relatório completo; erros
contextuais são JSON em stderr, exit 1. Exit 0 significa que o relatório foi
calculado, não que extrações ou cobertura sejam completas. A ferramenta não
escreve arquivos: redirecione stdout para guardar o relatório. Não instala
ferramentas, consulta variáveis de ambiente, usa rede ou habilita cache.

Relatórios produzidos pela ferramenta: [união em JSON](examples/union.json),
[união em texto](examples/union.txt) e [extração parcial](examples/partial.txt).

## Documento de entrada

```json
{
  "version": 1,
  "project": "meu-projeto",
  "configurations": [
    {
      "id": "linux-debug",
      "graph": "grafo-linux.json",
      "granularity": "file",
      "nature": "syntactic-import",
      "extraction": "partial",
      "diagnostics": ["Falha ao analisar arquivo gerado"],
      "identity_map": [
        {"node_id": 0, "identity": "X"},
        {"node_id": 1, "identity": "Y"}
      ]
    }
  ],
  "paths": [
    {
      "id": "dependencia-X-Y",
      "edges": [
        {"from": "X", "to": "Y", "relation": "uses", "uses_kind": "import"}
      ]
    }
  ]
}
```

Cada `graph` é lido relativamente ao documento, não ao cwd; paths absolutos
são aceitos. `identity_map` associa os IDs LOCAIS do grafo a identidades na amostra.
É obrigatório mapear todos os nós. Paths/names iguais não geram correspondência
automática; paths colidentes podem representar nós diferentes com IDs distintos.
Ambiguidade, nó não mapeado ou ID estranho gera erro antes da comparação.

`granularity`: file/module/package/item. `nature`: contrato de observação
explicitamente nomeado pelo usuário, por exemplo syntactic-import, active-include
ou resolved-reference. O mesmo nome precisa significar a mesma semântica em
todas as variantes. Granularidades/naturezas distintas são recusadas com diagnóstico;
separe em amostras compatíveis. Não há conversão automática entre elas.
Relações import/reference/subtipo desconhecido são inventariadas separadamente.

`extraction`: complete/partial/unknown (default unknown). `complete` descreve
somente a execução dentro do escopo declarado. Informe os motivos da extração
parcial em `diagnostics`; ficam preservados por configuração. O experimento não
lê automaticamente metadados heterogêneos nem deduz completude de um grafo vazio.
Identidade do projeto, configuração, natureza e completude são declarações do
usuário: não há autenticação nem verificação contra um build. Mapas não podem
agrupar dois nós da mesma configuração numa identidade; não há fuzzy matching.

## O que o relatório distingue

- `per_configuration`: nós com IDs/paths/tipos originais desserializados,
  relações observadas, estado e diagnósticos, SCCs, exemplos concretos de ciclos
  e ordem da condensação. Cada análise usa somente o grafo daquela configuração.
- `union.view = union-of-possibilities`: todos os nós/relações observados,
  cada qual com `observed_in`. A união não é uma configuração executada.
- `observation_class`: all-provided-configurations ou some-provided-configurations.
  A primeira significa todas as fornecidas, nunca todas as possíveis.
- `not_observed_in_complete`: sem observação no resultado de extração completa;
  também não atesta inexistência semântica fora daquele escopo.
- `not_observed_in_partial_or_unknown`: não observado em resultados parciais ou
  desconhecidos; falta de aresta/nó não é prova de ausência. Observações positivas
  de resultados parciais continuam válidas como observações e não são apagadas.
- `paths`: para cada percurso explícito, interseção de todas as configurações
  que observaram suas arestas. Interseção vazia recebe
  not-observed-together-in-provided-configurations. Isso não prova impossibilidade.

Consultas especificam relações exatas, inclusive subtipo. `uses_kind=null` ou
omitido seleciona subtipo desconhecido, não qualquer subtipo. Caminhos vazios,
descontínuos, com arestas inexistentes ou de contenção `owns` geram erro. Percursos
fechados são aceitos. Relações `owns` ficam no inventário, mas não entram em SCCs
nem caminhos de dependência. Compatibilidade de relações sintáticas não significa
ativação em runtime, mesmo se todas constarem do mesmo grafo.

## SCC e testemunho de ciclo

A função existente `detectar_ciclos` retorna SCCs >=2, apesar do nome. Por isso
`cyclic_sccs` lista conjuntos de membros e `cycle_examples` lista percursos
concretos separados. Self-loops são tratados adicionalmente. Não existe inferência
de que um SCC inteiro seja um único ciclo, nem interseção de todas as suas arestas.

O experimento escolhe um ciclo simples determinístico por SCC não trivial, mais
os self-loops. A lista **não é exaustiva**, explicitada em
`cycle_examples_exhaustive=false`. Cada exemplo tem suas arestas exatas e
`common_configurations`. Um exemplo da união sem configuração comum não elimina
outros ciclos testemunhados dentro do mesmo SCC: consulte as análises individuais
ou peça o caminho concreto. Não se resolve o problema exponencial de enumerar ciclos.

Na fixture principal:

| Observação | A | B | União |
|---|---|---|---|
| X→Y | observada | não observada | suporte {A} |
| Y→X | não observada | observada | suporte {B} |
| Y→Z | observada | não observada | suporte {A} |
| X→Y→X | não observado | não observado | {A} ∩ {B} = vazio |
| X→Y→Z | observado | não observado | {A} ∩ {A} = {A} |

A e B não têm ciclos individuais. A união tem SCC {X,Y}, mas seu ciclo exemplo
não foi observado integralmente em nenhuma das configurações fornecidas.
A fixture witnessed.json acrescenta configuração C com X→Y e Y→X; nesse caso,
o ciclo é observado em C. Z aparece apenas em A, sem ser inventado nas demais.

## Determinismo e limites

Configurações, nós, relações, consultas e diagnósticos são ordenados. Duplicatas
idênticas de arestas/mapas/configurações/consultas não alteram contagens nem o
relatório. Mesmo ID de configuração com conteúdo estrutural normalizado diferente
é erro, não união de execuções. Reordenar entradas não muda os bytes do relatório.
A normalização considera identidade, ID/path/tipo do nó, relações e declarações;
não compara posições, visibilidade ou outros atributos fora deste experimento.

Para reutilizar os algoritmos, uma projeção estrutural um-para-um atribui IDs
ordenados e paths únicos iguais às identidades explícitas. Não agrega módulos,
não altera granularidade e não exporta essa projeção como extração original.
A ordenação de condensação é dado do relatório, sem integração à DSM principal.
O L3 compila dependências transitivas existentes, inclusive comparação, mas a
ferramenta chama exclusivamente seu desserializador. Não chama a nova comparação.

Dados cabem em memória; sem limite de tamanho nem enumeração de todos os caminhos.
Há um BFS por SCC para um exemplo (e varreduras das relações). Não há inferência de
configurações omitidas, dependências de build, ativação de comptime ou runtime,
reprodutibilidade dos extratores ou correspondência probabilística de identidades.

## Testes e fixtures

```sh
cargo test --offline --locked --manifest-path lab/variantes-configuracao/Cargo.toml
cargo clippy --offline --locked --manifest-path lab/variantes-configuracao/Cargo.toml \
  --all-targets --no-deps -- -D warnings
cargo fmt --manifest-path lab/variantes-configuracao/Cargo.toml -- --check
```

Resultado em 2026-09-26, Linux/Rust 1.92: **22 testes passaram**, Clippy sem avisos.
Cobrem todos os casos solicitados, mais SCC com ciclos distintos, exemplo da união
sem suporte coexistindo com ciclo realmente observado, self-loops, subtipos/owns,
paths colidentes, interseções par-a-par insuficientes, grafos vazios, observações
positivas parciais e declarações desconhecidas. Testes inline em `src/main.rs`.
Os relatórios de exemplo foram gerados pelo executável real, sem rodar extratores.

Fixtures sintéticas (linhagem: CONTRATO.md): a.json/b.json/c.json/partial.json
são grafos no formato atual; union-only.json/witnessed.json/partial-sample.json
são solicitações válidas; incompatible.json/ambiguous.json devem falhar.
