> Integração atual: consulte [o guia comum](../../docs/doxygen.md) e a revisão
> ADR-0009 no CONTRATO.md. Inventários novos são v2, com pendências separadas.
> Relatos e contagens abaixo descrevem a entrega isolada anterior; a validação
> integrada está no laudo da rodada.

# Importador experimental de XML Doxygen

Lê XML existente e gera `lente.doxygen.inventory` v1. Não executa Doxygen,
não lê código-fonte e não integra o inventário ao grafo estrutural da Lente.
Workspace Cargo independente; contrato normativo em [CONTRATO.md](CONTRATO.md).

## Executar

Requisitos: Linux, Rust >= 1.91 e dependências Cargo disponíveis localmente.
Não há instalação automática. Os comandos abaixo usam modo offline; se faltar
uma dependência, Cargo falha em vez de baixá-la.

A partir da raiz do repositório:

```sh
cargo build --offline --locked --manifest-path lab/doxygen-importador/Cargo.toml
mkdir -p lab/doxygen-importador/out
lab/doxygen-importador/target/debug/lente-doxygen-importador \
  --xml-dir lab/doxygen-importador/fixtures/basico \
  --output lab/doxygen-importador/out/inventario.json
```

O diretório pai da saída deve existir. Escolha outro nome em novas execuções:
a ferramenta **nunca sobrescreve** um arquivo existente. Consulte o resultado
já gerado em [exemplo-inventario.json](exemplo-inventario.json): quatro símbolos,
três relações, `coverage=unknown` e diagnóstico `scope_unknown`.

Códigos de saída:

| Código | Significado |
|---|---|
| 0 | Inventário publicado; cobertura desconhecida |
| 2 | Inventário parcial publicado; consultar diagnósticos |
| 1 | Erro fatal em JSON no stderr; nenhum novo inventário publicado |

## Formato suportado

Perfil XML Doxygen **1.9.8**, raízes `doxygenindex` e `doxygen`, sem namespaces
XML. Versões explícitas diferentes são rejeitadas. Versão ausente é diagnosticada;
`producer.version` permanece desconhecida se ausente no índice. Não há validação
XSD completa nem promessa de suporte a todos os elementos dessa versão.

Referências oficiais consultadas: [saída XML no manual](https://www.doxygen.nl/manual/customize.html),
[organização dos arquivos](https://www.doxygen.nl/manual/starting.html) e
[compound.xsd da versão 1.9.8](https://raw.githubusercontent.com/doxygen/doxygen/Release_1_9_8/templates/xml/compound.xsd).
As fixtures são sintéticas e manuscritas, não capturadas de uma execução real;
veja sua [origem](fixtures/README.md).

`compounddef` e `memberdef` geram símbolos, preservando IDs e classificações.
Funções sobrecarregadas conservam IDs distintos. `qualified_name` vem apenas do
campo explícito; `signature` conserva `argsstring`, isto é, argumentos e
qualificadores, **não uma assinatura completa com tipo de retorno**. Não há
reconstrução por heurística. Localização requer arquivo e linha positiva.
Documentação ausente produz texto vazio; campos opcionais desconhecidos são null.
`configuration_digest` e `documentation_url` permanecem null.

Relações reconhecidas: `basecompoundref`, `derivedcompoundref`, `includes`,
`includedby`, `innerclass`, `innernamespace`, `innerfile`, `innerdir`, `innerpage`,
`innergroup`, `innerconcept`, `innermodule`, `references`, `referencedby`,
`reimplements`, `reimplementedby` e `ref`. A origem é o símbolo contenedor e o
alvo é o `refid` literal. A classificação não é traduzida para chamada ou uses;
`referencedby` não é invertida. Referências explicitamente externas geram
diagnóstico e não viram relações locais. Relações com alvo local não resolvido
permanecem acompanhadas de diagnóstico, sem criar símbolos fictícios.

## Cobertura e diagnósticos

Esta interface não emite `complete`: sucesso do parser não declara cobertura do
programa nem do conjunto de entrada. Sem perdas conhecidas, emite `unknown`.
Perdas identificadas produzem `partial`, preservando os dados disponíveis.

Exemplos de diagnósticos: `referenced_file_missing`, `unresolved_reference`,
`duplicate_definition`, `conflicting_definition`, `unsupported_element`,
`unsupported_kind`, `incomplete_location` e `index_definition_mismatch`.
Definições idênticas repetidas são deduplicadas com diagnóstico; definições
conflitantes colocam o ID em quarentena, sem escolher pela ordem de leitura.
Índice ausente, XML malformado, versão incompatível, caminho proibido e limites
excedidos são erros fatais. Coleções são ordenadas e deduplicadas deterministicamente.

Parágrafos, listas e seções viram texto. HTML ativo e diagramas opacos são omitidos
com diagnóstico. Marcação desconhecida não ativa conserva texto com diagnóstico.
Texto literal como `<T>` continua sendo texto: consumidores devem escapar os
campos ao renderizar HTML. Não usar documentação como HTML confiável.

## Segurança, recursos e publicação

A leitura fica restrita ao índice e aos nomes simples de compounds nele citados,
com descritor da raiz fixado e `openat`/`O_NOFOLLOW`. Links simbólicos de arquivos,
subdiretórios e arquivos não regulares são recusados. A raiz explicitamente
fornecida pode ser um link, cujo destino canônico passa a ser a raiz autorizada.
Não há leitura de fontes, schemas remotos, Doxyfile ou destinos de relações.
DTD e declarações de entidades são recusadas; não há resolução externa, rede,
XInclude ou XSLT. A recusa de marcadores DTD também abrange comentários/CDATA.

| Recurso | Limite |
|---|---:|
| XML individual / total | 4 MiB / 32 MiB |
| Compounds | 1.024 |
| Nós XML por arquivo / profundidade | 100.000 / 128 |
| Definições / relações / diagnósticos | 50.000 / 100.000 / 20.000 |
| Campo textual ou atributo / ID | 64 KiB / 512 bytes |
| JSON final | 16 MiB |

Limites excedidos abortam, sem truncamento. Não há cache, limite de CPU ou snapshot
transacional da entrada; os XML não devem mudar durante a leitura.

O JSON completo é validado em memória, escrito em temporário irmão e sincronizado.
Um hard link exclusivo publica o arquivo completo, impedindo substituição inclusive
entre processos concorrentes. O temporário é removido em falhas normais. Suporte
limitado a Linux e filesystem local com hard links; não há fallback. Não se faz
fsync do diretório: atomicidade de publicação não promete durabilidade após queda
de energia. Interrupção abrupta pode deixar um temporário órfão.

## Testes executados

```sh
cargo test --offline --locked --manifest-path lab/doxygen-importador/Cargo.toml
cargo clippy --offline --locked --manifest-path lab/doxygen-importador/Cargo.toml --all-targets --no-deps -- -D warnings
cargo fmt --manifest-path lab/doxygen-importador/Cargo.toml -- --check
```

Resultado nesta rodada: **21 testes passaram**, nenhum falhou; build e Clippy
passaram. Os testes inline cobrem funções/classes/namespaces, sobrecargas, Unicode,
documentação ausente, relações, arquivos ausentes/XML inválido, dados opcionais,
IDs conflitantes, determinismo, cobertura, limites, DTD, caminhos/symlinks e
publicação concorrente sem sobrescrita. A CLI também gerou o exemplo versionado.
Nenhum teste executou Doxygen; não houve validação contra uma instalação real nem
execução da suíte de produção, que não é dependência deste workspace.
