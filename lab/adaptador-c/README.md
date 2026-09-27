# Adaptador C experimental — includes por configuração

Protótipo Rust independente, restrito à Arena. Lê uma `compile_commands.json`
com **libclang 18**, extrai inclusões ativas entre arquivos e escreve grafos
compatíveis com `lente_infra::desserializar_grafo`. Não está integrado à CLI/MCP
nem ao workspace principal. **Não é análise de chamadas ou de referências entre
funções.** Contrato e decisão de interface: [CONTRATO.md](CONTRATO.md).

## Pré-requisitos e instalação explícita

- Rust 1.91+ (edition 2024; validado com Rust 1.92).
- Biblioteca de desenvolvimento **libclang 18**, que ofereça `libclang-18.so`
  ao linker/runtime. Validado no Linux x86_64 com Ubuntu clang **18.1.3**.
- Python 3 somente para gerar a compilation database das fixtures.

A instalação de libclang é manual pelo gestor de pacotes do sistema; no
Ubuntu com esse pacote disponível, `sudo apt-get install libclang-18-dev`.
O adaptador não executa esse comando nem instala qualquer ferramenta. Esta
versão tem vínculo explícito a `clang-18` e não promete portabilidade para
outros nomes de biblioteca ou sistemas operacionais.

Na raiz do repositório:

```sh
cargo build --locked --manifest-path lab/adaptador-c/Cargo.toml
cargo test --locked --manifest-path lab/adaptador-c/Cargo.toml
```

O build inicial pode buscar os crates fixados no `Cargo.lock` **local**; com
dependências já presentes, acrescente `--offline`. A execução do binário não
usa Cargo, rede, cache nem processos de instalação. As únicas dependências de
produção são serde_json e a API C de libclang. As dependências locais da Lente
são usadas apenas para verificar compatibilidade nos testes.

## Análise de um projeto

Obtenha a compilation database do próprio build (por exemplo, CMake com
`CMAKE_EXPORT_COMPILE_COMMANDS=ON`), com headers gerados já presentes. O
adaptador não roda o build. Informe a raiz usada para classificar arquivos:

```sh
lab/adaptador-c/target/debug/lente-adaptador-c-experimental \
  /caminho/build/compile_commands.json /caminho/projeto /caminho/saida-nova
```

A saída deve estar vazia. `directory` das entradas deve ser absoluto;
`file` e include paths podem ser relativos a ele. `arguments` é preferido;
`command` é tokenizado pela API oficial da compilation database, **sem shell**.
O executável descrito no comando nunca é iniciado. Wrappers e flags não
suportados produzem uma TU não analisada com diagnóstico explícito.

Saída:

- `graph-NNNN.json`: `crate`, `nodes`, `edges`, com arquivos `kind=mod`,
  `visibility=priv` e arestas diretas `uses/import`.
- `metadata-NNNN.json`: provedor/versão, configuração original e argv efetivo,
  ambiente relevante, granularidade, limites, diagnósticos, status,
  unidades não analisadas, classes de arquivos e posições de cada diretiva.
- `manifest.json`: associa cada par à unidade/configuração e resume status.

Exit codes: **0** completo; **2** parcial (arquivos de resultado disponíveis);
**1** erro de infraestrutura, base inválida ou saída inválida.

**Política de variantes:** uma saída por comando distinto e por TU; comandos
idênticos (mesmos directory/file/arguments) são deduplicados. Não há união nem
escolha silenciosa de uma configuração. `output` sozinho não altera a semântica
e não distingue variantes. Cada grafo é uma observação daquela configuração,
não um grafo completo agregado do projeto. Os índices são estáveis na repetição
da mesma base; podem mudar se a base mudar.

**Política de headers:** inclui arquivos do projeto, externos e de sistema,
assim como inclusões diretas observadas dentro deles. Metadados distinguem as
três classes; `-isystem` tem precedência sobre localização dentro da raiz.
Não criar atalho A→C para uma cadeia A→B→C. O núcleo calcula o trânsito.
Posições de diretivas ficam no arquivo de metadados, pois o contrato atual da
aresta não tem posição. Não inferir visibilidade pública a partir de um header.

Erro/fatal de parsing descarta o grafo da TU inteira (grafo vazio válido),
marca `partial`, identifica a TU e preserva os diagnósticos. Warnings são
registrados sem descartar uma TU válida. Nenhum fragmento incompleto é anunciado
como análise completa.

## Reproduzir as fixtures

```sh
python3 lab/adaptador-c/fixtures/gerar_base.py lab/adaptador-c/output/db
lab/adaptador-c/target/debug/lente-adaptador-c-experimental \
  lab/adaptador-c/output/db/compile_commands.json \
  lab/adaptador-c/fixtures lab/adaptador-c/output/relatorios
```

Esse exemplo termina com **2**, esperado: contém duas configurações de
`main.c` (com/sem FEATURE) e uma TU com header ausente. Para repetir, escolha
outro diretório vazio de saída. Nenhum arquivo `.o` é gerado.

## Cobertura e limites

| Construção/contexto | Tratamento |
|---|---|
| Include local, `-I`, `-iquote`, `-isystem`, `-idirafter` | Resolvidos pelo Clang no diretório da entrada |
| Defines/undefines, condicionais, include por macro | Somente diretivas ativas na configuração |
| Includes transitivos | Aresta de cada incluinte imediato; sem atalhos |
| Include guards e repetição | Ocorrências preservadas quando expostas; arestas deduplicadas |
| Headers externos e de sistema | Incluídos e classificados em metadados |
| `-std`, `-x c`, sysroot/target e flags aceitas | Preservados; lista explícita em `arguments` no código |
| Várias configurações de um arquivo | Pares de saída independentes |
| Header ausente ou erro de sintaxe | TU não analisada; parcial e diagnóstico |
| Chamadas, referências, símbolos e linkagem | Não analisados |
| Ramos inativos de pré-processamento | Sem arestas |
| PCH/modules/plugins, includes forçados, response files | Não suportados; rejeição explícita |
| C++, Objective-C, wrappers e flags desconhecidas | Não suportados; rejeição explícita |
| Compilador GCC/cc na base | Flags interpretadas pelo libclang; não reproduz extensões exclusivas de GCC |

Paths físicos absolutos e versão/ambiente/headers fazem parte do contexto;
o determinismo prometido é no mesmo checkout e ambiente, não entre máquinas.
O parser não faz sandbox de leituras: include paths podem apontar para fora da
raiz, coerente com a política de headers externos. Nenhum módulo/plugin do
projeto é carregado. Isso não é uma nova política do produto principal.

## Verificação executada

`cargo test --locked --offline --manifest-path lab/adaptador-c/Cargo.toml`:
**10 testes aprovados**, nenhum ignorado. `cargo clippy --all-targets -- -D warnings`
e build também aprovados (com o mesmo manifest e flags `--locked --offline`). Cobertura inclui `<stddef.h>` real,
`command` tokenizado, rejeição de flags, saída parcial, determinismo byte a byte,
aceitação pelo desserializador real da Lente e impacto de `leaf.h` sobre
`local.h` e `main.c`. Não foram alterados nem executados testes do workspace
principal; seus crates foram compilados como dependências de teste isoladas.

Fontes oficiais: [escolha da interface](https://clang.llvm.org/docs/Tooling.html),
[libclang](https://clang.llvm.org/docs/LibClang.html),
[compilation database](https://clang.llvm.org/docs/JSONCompilationDatabase.html),
[API C](https://clang.llvm.org/doxygen/group__CINDEX.html).
