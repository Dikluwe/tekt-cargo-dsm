# Adaptador C++ experimental: referências semânticas entre arquivos

Experimento independente em `lab/adaptador-cpp/`. Não depende do adaptador C,
não integra o pipeline principal, não instala dependências e não habilita cache.
Este README é o contrato local de linhagem dos fontes, fixtures e testes.

## Escolha técnica e documentação oficial

Usa a **API C do libclang**, chamada por `ctypes` da biblioteca padrão Python.
`clang_api.py` declara apenas o subconjunto necessário da ABI. Não requer o
pacote Python `clang`, nem interpreta um dump textual de AST.

Fontes oficiais consultadas em 2026-09-26:

- [Choosing the Right Interface](https://clang.llvm.org/docs/Tooling.html):
  libclang oferece interface C estável e cursores de alto nível, mas não acesso
  completo à AST. Escolhido por permitir referências resolvidas sem acoplamento
  às APIs C++ internas do compilador.
- [Libclang tutorial](https://clang.llvm.org/docs/LibClang.html): criação da
  unidade de tradução, visita a cursores e posições no código.
- [Cross-referencing in the AST](https://clang.llvm.org/doxygen/group__CINDEX__CURSOR__XREF.html):
  `clang_getCursorReferenced`, cursores canônicos e USRs sustentam a resolução.
  A identidade vem da declaração resolvida, não de coincidência de nomes.
- [JSON Compilation Database](https://clang.llvm.org/docs/JSONCompilationDatabase.html):
  `directory` dá o contexto dos caminhos; uma TU pode ter comandos alternativos.
- [Template introspection](https://clang.llvm.org/doxygen/group__CINDEX__CPP.html):
  `clang_getSpecializedCursorTemplate` indica relação com template. Não fornece,
  por si só, um inventário completo de corpos implicitamente instanciados.

Interface e testes validados com **Ubuntu libclang 18.1.3**, Python 3.12. A
biblioteca compartilhada precisa estar previamente instalada e é selecionada
explicitamente por `--libclang`; sua versão real fica nos metadados.

## Executar o exemplo

Na raiz do repositório:

```sh
mkdir -p lab/adaptador-cpp/out
python3 -B lab/adaptador-cpp/tests/make_database.py \
  lab/adaptador-cpp/out/compile_commands.json
python3 -B lab/adaptador-cpp/adaptador.py \
  --database lab/adaptador-cpp/out/compile_commands.json \
  --root lab/adaptador-cpp/tests/fixtures \
  --libclang /usr/lib/llvm-18/lib/libclang.so \
  --graph lab/adaptador-cpp/out/grafo.json \
  --metadata lab/adaptador-cpp/out/metadados.json
```

`--root` define os arquivos do projeto. Headers externos podem ser lidos pelo
Clang para resolver tipos, mas ficam fora dos nós/arestas. Referências externas
observadas ficam nas evidências com seu destino original. Arquivos sem TU na
base e headers nunca alcançados não são descobertos por busca recursiva.

O gerador de fixture materializa caminhos absolutos para este checkout; não
executa build. Para projetos reais, forneça uma base já existente.

## Comandos e configurações

Nenhum compilador, wrapper, comando da base ou shell é executado. O adaptador
passa argv para `clang_parseTranslationUnit2FullArgv`. `arguments` tem preferência;
`command` é separado com `shlex` POSIX, sem expansão ou execução. Bases Windows,
wrappers (`ccache`, `env`, scripts) e response files não são suportados.

`directory` precisa ser absoluto e existente. A TU é canonicalizada em relação
a ele e `-working-directory=<directory>` é passado ao Clang. Não se altera o cwd
global. `argv[0]` preserva o modo de driver, mas o parser é sempre a versão do
**libclang selecionado**, mesmo se o compilador original for GCC. Não há promessa
de equivalência com extensões ou headers implícitos de outra toolchain.

Suporte explícito a `-std`, `-I`, `-isystem`, `-iquote`, `-idirafter`, `-include`,
`-D`, `-U`, sysroot, target, `-x`, resource-dir, otimização, debug, warnings,
`-pthread`, `-nostdinc[++]`, RTTI/exceptions, PIC e `-m32/-m64` (veja a lista em
`load_commands`). Preserva ordem e valores. Remove somente modo `-c` e geração
de outputs/dependências (`-o`, `-MF`, `-MT`, `-MQ`, `-MJ`, `-MD`, `-MMD`, `-MP`),
registrando os argumentos removidos. Flags desconhecidas geram erro explícito;
plugins, PCH e opções de módulos não são descartados silenciosamente.

A configuração é SHA-256 de **diretório + compilador + argumentos preservados**,
sem o arquivo da TU nem outputs. Diretórios diferentes ficam em grupos distintos
mesmo quando seus flags parecem equivalentes: é uma separação conservadora.
Se houver vários grupos, a execução recusa misturá-los e lista seus hashes.
Use `--configuration <hash>` para selecionar um. Metadados registram o grupo
selecionado, comandos e hashes dos grupos excluídos. TUs repetidas no mesmo grupo
são analisadas uma vez. Não há fusão entre Debug/Release ou conjuntos de defines.
O ambiente implícito de busca de headers do libclang não é hermético; mantenha-o
igual para comparar execuções. Não inferimos configurações de build pelo nome.

## Modelo e evidências

O grafo compatível com `lente_infra::desserializar_grafo` contém somente:

- `crate="cpp"`: rótulo exigido pelo formato antigo, não um crate Rust.
- Arquivos locais como `kind="mod"`, `name` relativo à raiz e
  `path="cpp::<caminho/relativo>"`. `visibility="pub"` é compatibilidade do
  formato, não inferência de visibilidade C++. IDs sequenciais após ordenar
  caminhos são estáveis apenas para o mesmo conjunto de arquivos, não persistentes.
- Uma aresta `uses` por par origem/destino com referência semântica observada.
  `uses_kind=null`: **não** convertemos chamadas C++ em `Reference` Rust, nem
  referências semânticas em `Import`. Filtros Rust de `so-referencia` não são
  apropriados para este grafo. Não emitimos `owns` ou nós de símbolos artificiais.

**Includes não entram nas arestas semânticas.** Ficam em `metadata.includes`,
com origem/linha/coluna/offset e destino, obtidos do registro de pré-processamento.
Um header apenas incluído pode existir como nó sem nenhuma aresta para ele.

`semantic_evidence` preserva posição de expansão da origem, declaração canônica
referenciada (posição, USR e nome apresentado), natureza (`TypeRef`, `TemplateRef`,
`DeclRefExpr`, `MemberRefExpr`, base de classe, chamada estática, chamada virtual,
redeclaration ou declaration_definition), contexto de template e TUs observadoras.
Arestas só são criadas entre arquivos distintos. Referências dentro do mesmo
arquivo não entram no grafo entre arquivos.

O conjunto de evidências é deduplicado por seu conteúdo, **sem a TU observadora**.
`observed_in` acumula a proveniência. Uma referência em header visto em várias
TUs conta uma vez; a aresta entre arquivos também conta uma vez. Duas naturezas
sobre a mesma expressão (ex.: CallExpr e DeclRefExpr) podem aparecer como
**evidências distintas**, mas nunca aumentam peso da aresta. Não produzimos pesos.

## Declarações, definições, templates e dispatch

A referência aponta para a **declaração canônica**, não é redirecionada para uma
definição localizada em outra TU. `symbols` registra declarações e definições
observadas, suas posições e USR. Uma definição que corresponde a declaração em
header gera evidência `declaration_definition`. Overloads têm USRs distintos;
nomes iguais não são fundidos. Não há busca de definição no linker nem prova ODR.
USRs são chave de reconciliação nesta configuração; não promessa de identidade
persistente entre versões/toolchains ou projetos diferentes.

`template_origin` indica `pattern`, `instantiation_or_specialization` ou
`ordinary` conforme o cursor e ancestrais expostos. `target_template` descreve
se a referência mira um padrão ou especialização/instanciação observada. A API
escolhida não distingue de maneira geral instanciação implícita de especialização
explícita: o rótulo conjunto é intencional. Uma chamada no código ordinário a
`generic<Item>` tem origem ordinary e destino instantiation_or_specialization.
Chamadas dependentes não resolvidas geram diagnóstico, sem alvo inventado.
Não prometemos visitar corpos de todas as instanciações implícitas, nem aqueles
que não foram observados nas TUs selecionadas.

Chamadas virtuais só geram dependência à declaração **estaticamente conhecida**,
com natureza `virtual_dispatch_static_declaration`, `runtime_target=false` e
diagnóstico. Isso também é conservador para chamadas qualificadas/devirtualizadas.
Chamadas indiretas não geram uma chamada para a função que talvez execute; recebem
`indirect_call_no_runtime_target`. Referências explícitas a uma variável ponteiro
podem continuar sendo evidências de uso daquela declaração, sem destino runtime.
Todo `runtime_target` é false: este experimento não executa nem traça o programa.

## Completude e limites comprovados

`execution` conta TUs selecionadas e parseadas sem erros. `semantic_coverage` é
sempre `partial`, mesmo com execução complete. Erros/fatais Clang descartam a AST
daquela TU e deixam diagnóstico e nó da TU; outras TUs continuam. Warnings são
reportados. Problemas de configuração/ABI/saída são erros fatais de invocação,
sem alegação de grafo válido. Saída 0 = execução complete; 1 = partial; 2 = erro.

- **Templates:** fixture comprova padrão não dependente e referência a uma
  especialização; a chamada dependente do padrão fica diagnosticada como não
  resolvida no libclang 18. Não inferimos o corpo da instanciação ausente.
- **C++20 modules:** capacidade declarada **false**. `.cppm`, `.ixx` e `.mpp`
  são recusados por TU; cursores ModuleImport geram diagnóstico; opções de
  módulos/PCM são recusadas. Não construímos BMIs, não resolvemos imports de
  módulos. A detecção de todos os modos alternativos de module unit em `.cpp`
  não é garantida: não use a saída para certificar projetos com módulos.
- **Macros:** preprocessamento segue a configuração. Expansões observadas geram
  diagnóstico `macro_expansion_partial`; posições são de expansão, não uma trilha
  de autoria das definições/tokens. Não há grafo de dependências entre macros.
  Algumas referências geradas podem aparecer na AST, sem promessa de cobertura
  completa ou atribuição correta à macro que as originou.
- **Dispatch:** fixtures de chamada virtual e ponteiro confirmam que nenhum
  destino runtime é apresentado. Sem análise de pontos-para ou hierarquia dinâmica.
- **Headers repetidos:** fixture comprova a mesma evidência de header em duas TUs
  com `observed_in` de tamanho 2 e uma única aresta.
- **Includes:** fixture `unused.hpp` é incluída e não vira dependência semântica.
- Sem cobertura de link, assembly, AST interna completa, tipos dependentes ainda
  não resolvidos ou código não alcançado pela configuração. Não se calcula
  equivalência com Rust ou com extração só de includes.

Os metadados têm versão própria 1, capacidades, configuração, versão libclang,
evidências e SHA-256 dos bytes do grafo. O consumidor atual da Lente **não lê**
esse arquivo; aceitar o JSON só comprova compatibilidade estrutural do formato.

## Testes

```sh
LIBCLANG_PATH=/usr/lib/llvm-18/lib/libclang.so \
  python3 -B -m unittest discover -s lab/adaptador-cpp/tests -v
cargo run --offline --quiet \
  --manifest-path lab/adaptador-cpp/contract-check/Cargo.toml \
  --target-dir lab/adaptador-cpp/contract-check/target \
  -- lab/adaptador-cpp/out/grafo.json
```

O checker Cargo tem `[workspace]` próprio e só lê o JSON com o desserializador
real. Não altera manifests do projeto principal. Todos os artefatos do experimento
ficam nesta pasta. Se libclang estiver ausente, os testes que precisam dele são
explicitamente ignorados e nada é instalado. Uma execução com testes ignorados
não constitui validação do backend.
