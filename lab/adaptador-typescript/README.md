# Adaptador experimental JavaScript/TypeScript

Experimento independente em `lab/adaptador-typescript/`. Produz JSON aceito
por `lente_infra::desserializar_grafo`, sem integrar o workspace Cargo, a CLI
ou o MCP principal. **Granularidade: arquivo, não função.** Uma dependência
estática não significa chamada de função nem previsão de quebra comportamental.

## Instalação

Requisitos: Node.js >=20 e npm. TypeScript **5.9.3** está fixado em
`package.json` e `package-lock.json` e é verificado na execução.

A partir desta pasta:

```sh
npm ci --ignore-scripts --cache .npm-cache
```

Esta é uma instalação explícita, separada da análise. `extrair.cjs` nunca
invoca npm, instala ferramentas ou baixa arquivos. O compilador é carregado
da dependência local do adaptador, não do projeto analisado. Para esta execução,
a versão já disponível no ambiente foi copiada localmente, sem download.

## Uso e exemplo

```sh
node extrair.cjs --project exemplo/tsconfig.json --out saida/exemplo --name exemplo
```

Para outro projeto:

```sh
node extrair.cjs --project /caminho/projeto/tsconfig.json --out saida/projeto --name projeto
```

O diretório pode existir, mas não pode conter `grafo.json` ou `metadados.json`
anteriores. Escolha outro diretório para uma nova execução. O adaptador recusa
sobrescrita para evitar confusão com resultados antigos.

- `grafo.json`: somente `crate`, `nodes`, `edges`, no formato atual da Lente.
- `metadados.json`: provedor, versões, granularidade, projeto, política externa,
  limites, cobertura, arquivos selecionados/analisados, diagnósticos e ocorrências.
  `graph_sha256` identifica exatamente os bytes do grafo correspondente.

O exemplo gera três módulos e duas arestas:
`entrada.ts → intermediario.ts → base.ts`. Os paths usam o prefixo `ts_exemplo`,
como `ts_exemplo::src::base.ts`. Segmentos são codificados com `encodeURIComponent`
para não confundir caracteres de nomes de arquivo com o separador `::`.

O JSON pode ser fornecido à **entrada já existente** da Lente, por exemplo,
com um binário `lente` já compilado:

```sh
lente --grafo saida/exemplo/grafo.json --alvo ts_exemplo::src::base.ts
```

Não foi adicionada uma opção TypeScript à CLI/MCP. A entrada JSON atual **não
lê os metadados separados**: consulte ambos os arquivos antes de interpretar
pouco ou nenhum impacto. Não use o modo Rust `--diff` para presumir cobertura
de arquivos JS/TS. `--so-referencia` remove estes imports deliberadamente,
pois as arestas têm `uses_kind: "import"`, nunca `reference`.

## Semântica do grafo

Cada arquivo local analisado vira um nó `kind: "mod"`. `visibility: "priv"`
é um preenchimento de compatibilidade, sem inferência sobre exports públicos.
`position` contém caminho absoluto e intervalo de linhas do arquivo inteiro.
Não são criados nós de função nem arestas `owns` artificiais.

Um import ou reexport suportado gera `relation: "uses"`,
`uses_kind: "import"`, do arquivo consumidor ao arquivo dependido. Há uma
aresta por par de arquivos, mesmo quando existem múltiplas declarações. Todas
as ocorrências ficam nos metadados com linha/coluna 1-based, especificador,
natureza `import`/`reexport`, `type_only` e especificadores individuais de tipo.
Imports de tipo contam como dependências estáticas, mesmo apagados em runtime.

IDs são únicos, atribuídos por ordenação lexical dos caminhos normalizados.
`from`/`to` correspondem exatamente aos paths dos nós indicados por seus IDs.
Repetir a extração com mesmas entradas no mesmo local gera bytes idênticos,
sem timestamps. IDs podem mudar ao adicionar arquivos; posições absolutas
mudam quando o projeto é movido. Não há cache persistente nem estado entre
extrações; estruturas internas do compilador vivem somente durante a chamada.

## Seleção e resolução

Usa APIs oficiais: `getParsedCommandLineOfConfigFile`, `createProgram`,
`getModeForUsageLocation` e `resolveModuleName`; não usa regex para resolver
módulos. Regex aparece apenas na classificação das extensões suportadas.
A implementação fixa `noEmit: true`, sem mudar opções de resolução.

`tsconfig.json`/`extends`, `files`, `include`, `exclude`, `allowJs`, JSX,
`paths`, `baseUrl` e modo de resolução são respeitados. Começa pelas raízes
selecionadas e segue imports/reexports locais suportados. Como no TypeScript,
`exclude` limita a seleção inicial, não proíbe arquivos locais importados.
`noResolve` impede incluir destinos fora do Program; eles são diagnosticados.
Arquivos locais fora da pasta do tsconfig podem entrar quando importados.

Dependências resolvidas como externas pelo TypeScript ou dentro de
`node_modules` são registradas com `external-excluded`, sem nó/aresta e sem
percurso de suas dependências. Isso inclui pacotes locais ligados como externos.
Bibliotecas padrão do compilador ficam fora do grafo. Um módulo não resolvido
fica em `unresolved-import` com origem, posição e especificador; não se cria nó
fictício. Módulos ambiente/builtins sem arquivo resolvido também podem aparecer
como não resolvidos; não fazemos resolução semântica desses módulos.

## Cobertura e limites

| Construção | Tratamento |
|---|---|
| Imports relativos, side-effect e index | Resolve pelo TypeScript; aresta de arquivo |
| Aliases `paths`/`baseUrl` | Resolve segundo tsconfig |
| Reexports e `export *` | Aresta `uses/import`; natureza preservada nos metadados |
| `import type`, `export type`, especificadores `type` | Incluídos como dependência estática |
| JS/JSX com `allowJs`, TS/TSX/MTS/CTS/MJS/CJS | Arquivos aceitos; somente relações estáticas suportadas |
| NodeNext | Usa modo de resolução da ocorrência, incluindo requisitos de extensão |
| Externos | Registrados e excluídos; não mede impacto dentro de dependências |
| Import não resolvido | Diagnóstico; nenhuma aresta |
| `import()`, `require`, CommonJS, `import =`, `export =` | Diagnóstico; nenhuma aresta correspondente |
| Expressão de tipo `import(...)` | Diagnóstico; nenhuma aresta |
| Triple-slash, namespace/módulo ambiente, `import.meta` | Diagnóstico; não modelados internamente |
| JSON, CSS, Vue/Svelte e outros recursos | Fora do grafo; diagnóstico quando resolvidos |
| Project references | Erro explícito; analisar cada tsconfig separadamente |
| Plugins de bundler, chamadas, fluxo de dados e runtime | Não suportados |

`coverage: "complete-within-declared-scope"` significa que não foram encontrados
problemas além das exclusões externas declaradas. `partial` indica diagnóstico
como import irresolvido ou construção não suportada; os dados conhecidos ficam
consultáveis. O processo retorna 0 também nesse caso: o consumidor precisa ler
os metadados. Erros de configuração, opções, sintaxe e fontes indisponíveis
retornam 1, escrevem metadados com `coverage: "error"` e não publicam grafo.
Falhas operacionais, argumentos inválidos ou dependência ausente retornam 1 e
mensagem em stderr; podem ocorrer antes de qualquer metadado.

Não há typecheck ou emit. Nenhum código, plugin ou script do projeto analisado
é executado. O diagnóstico CommonJS é conservador: nomes locais chamados
`require`, `module` ou `exports` podem gerar aviso mesmo quando não são CommonJS.
O filesystem não é um snapshot transacional; mantenha as entradas estáveis
até a análise terminar.

## Testes e compatibilidade real

```sh
npm test
npm run test:compat
```

O segundo comando exige Rust >=1.91, Cargo e dependências Rust já disponíveis
localmente (`cargo --offline`). O harness tem `[workspace]` próprio em
`compat/`, com dependências por caminho para as bibliotecas da Lente. Seus
artefatos e lock ficam nesta pasta; não adiciona testes ao workspace principal.
Se faltar dependência Rust no cache, uma preparação **explícita** opcional é:

```sh
cargo fetch --manifest-path compat/Cargo.toml
```

Resultados verificados em 2026-09-26 com Node 20.20.2 e TypeScript 5.9.3:

- 12 testes Node aprovados: relativos/index/alias, fechamento local, tipos e
  reexports, externo/irresolvido, ciclos/determinismo/hash, extends/files/exclude,
  JS/JSX/TSX sem execução, NodeNext, construções não suportadas, ausência de cache,
  erros, publicação dos dois artefatos, paths sem colisão e noResolve.
- Harness real aprovado: JSON desserializado por `lente_infra`, quatro nós `mod`,
  quatro arestas `uses/import`, integridade ID/path, impacto transitivo até
  profundidade 3 e um ciclo calculados pelos algoritmos existentes.
- Nenhuma dependência do fork `cargo-modules` para extrair ou testar este adaptador.

## Referências

- [API oficial do compilador TypeScript](https://github.com/microsoft/TypeScript/wiki/Using-the-Compiler-API).
- `CONTRATO.md` nesta pasta registra a decisão experimental.
- Contrato de compatibilidade lido: `03_infra/src/dto.rs`, `traducao.rs`,
  `lib.rs` e `00_nucleo/specs/forma-organizada.md` do repositório principal.
