> Integração atual: consulte [o guia comum](../../docs/doxygen.md) e a revisão
> ADR-0009 no CONTRATO.md. Inventários novos são v2, com pendências separadas.
> Relatos e contagens abaixo descrevem a entrega isolada anterior; a validação
> integrada está no laudo da rodada.

# Prioridade de revisão documental

Experimento independente que combina **impacto estrutural fornecido** e
**associações documentais fornecidas**. Não extrai código/XML, executa Doxygen,
recalcula impacto ou associa símbolos. Não integra produção, substitui provedores
ou altera regras de arquitetura/CI.

O relatório prioriza **revisão humana**: não mede correção, qualidade arquitetural
nem qualidade efetiva da documentação. Texto observado não significa texto bom;
símbolo não associado não significa símbolo sem documentação.

## Executar offline

Requer Rust >=1.91 e as dependências já disponíveis localmente. Nenhuma ferramenta
é instalada e nenhuma dependência é baixada pelos comandos abaixo. Da raiz do
repositório:

```sh
cd lab/doxygen-prioridade
export RUSTUP_AUTO_INSTALL=0
export CARGO_TARGET_DIR="$PWD/target"
cargo build --offline --locked
mkdir -p out
./target/debug/lente-doxygen-prioridade \
  --inventory fixtures/inventory.json \
  --catalog fixtures/catalog.json \
  --matches fixtures/matches.json \
  --impact fixtures/impact.json \
  --output out/relatorio.json \
  --summary
```

`--output` exige arquivo novo e pai existente. Para repetir, escolha outro nome;
resultados anteriores e entradas não são sobrescritos. `--summary` é opcional:
imprime resumo textual em stdout, com estados, evidências, contagens, cobertura,
justificativas e diagnósticos. Sem a opção, stdout fica vazio. Ajuda: `--help`.
Exit 0 significa relatório produzido, inclusive parcial/com métricas ausentes;
exit 1 significa erro de argumentos, arquivo, schema, referência ou vínculo, com
diagnóstico em stderr. Não usar o exit 0 para inferir completude.

## Entradas e verificações

Entradas JSON do contrato de intercâmbio v1:

| Arquivo | Schema |
|---|---|
| Inventário | `lente.doxygen.inventory` |
| Catálogo explícito | `lente.symbol_catalog` |
| Correspondências | `lente.doxygen.matches` |
| Impacto explícito | `lente.impact_summary` |

Nem o catálogo nem o resumo de impacto são presumidos como saída da CLI atual.
A ferramenta recebe somente JSON; não importa código dos outros experimentos.

São exigidos inventory_digest e catalog_digest nas correspondências iguais ao
SHA-256 dos **bytes exatos** recebidos. Até mudança de espaços pode invalidar o
vínculo. graph_digest do catálogo e impacto precisa ser igual. Digests usam
`sha256:` e 64 hexadecimais minúsculos. O relatório também registra os digests de
matches e impact. Não recebe os bytes do grafo: verifica o vínculo declarado,
sem comprovar a contagem ou a autenticidade do produtor.

Valida IDs repetidos, referências de documentos/relações/diagnósticos/candidatos,
nós das métricas e status coerentes. Exige uma correspondência por nó do catálogo.
`matched` exige document_id existente dentro de candidates; `ambiguous` mantém
candidato(s) sem selecionar um; `unmatched` mantém document_id null. As listas
unmatched recebidas são preservadas, verificadas e não podem contradizer matched.
Não impõe correspondência bijetiva ou refaz as evidências declaradas.

Campos opcionais ausentes são null; não se inventam nome qualificado, assinatura,
localização, URL ou versão. `brief` e `details` são strings obrigatórias e podem
ser vazias. Nomes, assinaturas e caminhos internos ficam intactos, sem normalização
ou tentativa de casar caminhos relativos/absolutos. Localizações fornecidas são
1-based. IDs são locais ao inventário/catálogo, sem estabilidade entre versões.

Schemas/versões desconhecidos são recusados. Campos desconhecidos também são
recusados para evitar descarte silencioso. Catálogo e impacto aceitam a extensão
opcional `diagnostics`, no mesmo formato code/message/symbol_id; quando symbol_id
estiver presente, referencia um documento do inventário. Campos numéricos exigem
inteiros não negativos u64; métricas negativas, fracionárias ou strings são erros.

## Saída e interpretação

JSON com schema `lente.doxygen.priority_report`, version 1. `items` contém uma
entrada para cada nó do catálogo, com símbolo estrutural, associação, documento
selecionado quando houver, evidências, contagem, estado documental, cobertura da
métrica, rank e justificativa. `producer`, relações documentais, listas unmatched
e diagnósticos das entradas são preservados. Relações Doxygen não viram arestas
estruturais.

| `documentary_state` | Significado |
|---|---|
| `documentation_observed` | Associação matched com texto não branco em brief ou details |
| `associated_without_text` | Associação matched; texto recebido vazio/somente whitespace |
| `ambiguous_association` | Associação ambígua; documento não eleito |
| `unassociated` | Símbolo sem associação; situação documental desconhecida |

Os textos originais permanecem intactos. A checagem de whitespace é apenas para
classificar presença de texto, não normaliza o documento. Não há HTML gerado ou
executado; conteúdos recebidos são strings literais em JSON. O resumo textual
escapa caracteres de controle através da representação JSON.

A ordem é transparente: consumidores transitivos **informados**, decrescente,
e node_id crescente no empate. Todos os estados participam, sem score ponderado
por documentação/confiança. `rank` é ordinal nessa lista de métricas conhecidas.

Métrica ausente (item inexistente ou contagem omitida/null) gera diagnóstico
`missing_impact_metric`, `transitive_consumers: null`, `metric_status: "missing"`
e `rank: null`. Esses itens aparecem ao final por ID, sem serem chamados de
baixo impacto. Zero explicitamente informado é valor conhecido e participa do
ranking normalmente.

`coverage.inventory` e `coverage.impact` conservam independentemente complete,
partial ou unknown. Cada item repete impact_coverage; position_reason declara
essa cobertura. Não há promoção automática ou combinação opaca de coberturas.
Complete descreve **somente o conjunto de entrada declarado**; métricas parciais
não são contagens completas do programa. Diagnósticos ficam separados por origem
em `diagnostics`, com os avisos gerados em `diagnostics.report`.

Todas as coleções de saída têm ordem determinística. Mesmos bytes de entrada
produzem mesmos bytes de saída, sem timestamps ou caminhos da execução.
Permutar coleções de entrada mantém a ordem semântica dos itens, mas altera os
digests quando os bytes mudam, como exigido pelo vínculo.

## Fixture fornecida

Os quatro JSONs são sintéticos, pequenos e locais. Não resultam de execução de
Doxygen. O graph_digest é um identificador sintético comum, não a afirmação de
que um grafo real foi medido. Correspondências/evidências são declarações da
fixture, não conclusão de um algoritmo de associação executado aqui.

| Nó | Consumidores informados | Estado | Rank |
|---|---:|---|---:|
| 10 | 12 | associated_without_text | 1 |
| 20 | 12 | unassociated | 2 |
| 40 | 7 | ambiguous_association | 3 |
| 30 | 1 | documentation_observed | 4 |
| 50 | ausente | associated_without_text | null |

Cobertura documental e de impacto: partial. Os documentos choice-a/choice-b são
candidatos ambíguos e permanecem sem seleção. O nó 50 tem documentação associada,
mas não foi informado valor estrutural para ele.

## Testes reproduzíveis

Ainda dentro deste workspace, com CARGO_TARGET_DIR apontando para seu target:

```sh
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo fmt -- --check
```

Resultado executado: **18 testes aprovados, zero falhas/ignorados**, Clippy sem
avisos e formatação conferida. Também foram executadas quatro verificações com
o binário real: relatório/resumo, repetição byte a byte, digest divergente sem
publicação e recusa de sobrescrita. Detalhes em [RESULTADOS.md](RESULTADOS.md).

## Limites

Não verifica a verdade das evidências fornecidas nem recalcula associações ou
métricas. Não avalia conteúdo, correção, arquitetura ou semântica de chamadas.
Dados com schema v1 não representam toda a semântica do programa. Os hashes não
autenticam o produtor, nem o graph_digest prova os valores do resumo estrutural.
Arquivos são carregados em memória, sem streaming/limite de tamanho. A leitura
não é transacional entre os quatro arquivos. Saída create_new protege contra
sobrescrita, mas não promete publicação atômica/durabilidade após encerramento
abrupto. Nenhuma integração com produção ou execução da suíte principal é
alegada nesta entrega. O contrato normativo está em [CONTRATO.md](CONTRATO.md).
