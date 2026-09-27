# Contrato local — adaptador C experimental

Data: 2026-09-26. Arena independente do workspace principal.
Unidades: `src/main.rs`, `src/clang.rs`, fixtures C.

## Decisão de interface

Usar a API C estável **libclang 18** a partir de Rust, por FFI isolada.
`clang_CompilationDatabase_fromDirectory` lê a base e tokeniza `command`;
nenhum campo da base é executado nem enviado a shell.
`clang_parseTranslationUnit2FullArgv` analisa com
`CXTranslationUnit_DetailedPreprocessingRecord`. Visitar apenas cursores
`CXCursor_InclusionDirective` e consultar `clang_getIncludedFile` fornece
origem/destino diretos, inclusive includes via macro. Diferente de uma lista
plana de dependências, não cria arestas entre todos os arquivos de uma TU.
Não usar regex para extração ou resolução. Não compilar, linkar, executar o
projeto, carregar plugins, instalar ferramentas ou cachear análises.

Referências oficiais consultadas:
- https://clang.llvm.org/docs/Tooling.html (interface C estável)
- https://clang.llvm.org/docs/LibClang.html (parsing e cursores)
- https://clang.llvm.org/docs/JSONCompilationDatabase.html (directory/argv/variantes)
- https://clang.llvm.org/doxygen/group__CINDEX.html (API C)
- Headers instalados de LLVM 18: `clang-c/Index.h`, `CXCompilationDatabase.h`.

## Configurações, alcance e flags

Uma saída independente por entrada distinta da compilation database, inclusive
quando várias entradas usam o mesmo arquivo. Entradas idênticas são deduplicadas;
as demais são ordenadas pelo conteúdo normalizado da configuração. Nunca há
união de variantes ou de TUs. Índices dos pares são determinísticos para a mesma
base, não identidade persistente após mudanças na base.

`directory` deve ser absoluto e existente; `file` é resolvido relativamente a ele.
O contexto vai ao Clang por `-working-directory`, sem mudar cwd do processo.
Preservar argv[0] para contexto do driver; ele não é executado. Aceitar drivers
clang/gcc/cc (sem wrappers) e linguagem C. Aceitar include paths, defines,
undefines, standard C, target/sysroot e lista explícita de flags no código.
Remover somente opções de geração de objeto/dependências, registrando argv original
e efetivo. Flags desconhecidas, plugins, response files, PCH, modules, forced
includes, wrappers e linguagens diferentes são erro da unidade, não ignoradas.
Ambiente de includes/SDK é registrado; a reprodução exige igual ambiente,
headers gerados previamente, libclang e arquivos em disco.

## Grafo e metadados

Nós: arquivos observados, `kind=mod`, `visibility=priv` (sem inferir API pública),
path físico absoluto canonicalizado. Arestas diretas de arquivo incluinte para
incluído: `relation=uses`, `uses_kind=import`. Não são chamadas nem referências
entre funções. IDs seguem ordem lexical dos paths; arestas deduplicadas e
ordenadas, endpoints textuais coerentes com IDs.

Headers externos e de sistema são **incluídos**, inclusive suas inclusões
transitivas observadas. Metadados classificam cada arquivo como project, external
ou system, por raiz explícita e `clang_Location_isInSystemHeader`. Isso evita
apagar caminhos de dependência; não existe filtro de stdlib Rust aqui.
Posições das diretivas (arquivo, linha, coluna, offset) ficam nas ocorrências
dos metadados: a Aresta atual da Lente não tem posição. O nó representa arquivo
inteiro; não se atribui a ele a posição de um include.

Warnings são preservados. Erro/fatal do Clang, configuração recusada ou falha de
parsing marca a TU como não analisada e resultado parcial; seu grafo fica vazio
(não mistura fragmentos de parsing falho com resultado completo). Cada TU tem
metadados com provedor, versão, argv, granularidade, limites e diagnósticos.
`manifest.json` relaciona os pares e resume completude. Exit 0 se todos completos,
2 se parcial, 1 para erro de infraestrutura/base/saída. Sem cache.

## Limites

Só includes ativos observados pelo pré-processador na configuração escolhida.
Guards podem impedir processamento repetido; não pretende modelar includes de
ramos inativos nem semântica de runtime. Não há análise de chamadas/referências,
linkagem, símbolos, dependências de build, C++, Objective-C, PCH/modules, plugins,
compiladores proprietários ou inferência de flags ausentes. Paths absolutos
implicam determinismo no mesmo checkout/ambiente, não igualdade entre máquinas.

## Verificação

Fixtures/testes inline Rust: local, -I com cwd diferente, header de sistema,
-D condicional, cadeia transitiva sem atalho, header ausente, erro de sintaxe,
variantes separadas, command sem shell, opções recusadas, determinismo e
aceitação por `lente_infra::desserializar_grafo`; impacto transitivo no núcleo.
