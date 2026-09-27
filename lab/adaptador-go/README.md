# Adaptador Go experimental

Extrai **imports diretos entre pacotes** para uma configuração explícita. Não
analisa funções. Projeto Rust isolado: não integra o workspace, CLI ou MCP da
Lente e não depende dos outros adaptadores experimentais.

## Executar

Requer Rust ≥ 1.91 e Go instalado (validado com Go 1.26.5). Não há instalação
ou download automático de módulos/toolchains Go. Execute da raiz do repositório:

```sh
cargo build --offline --manifest-path lab/adaptador-go/Cargo.toml
lab/adaptador-go/target/debug/lente-adaptador-go \
  --project lab/adaptador-go/fixtures/chain \
  --goos linux --goarch amd64 --tests exclude --work off \
  --out lab/adaptador-go/target/chain
```

`--offline` exige as dependências Rust já disponíveis localmente. A compilação
do adaptador e a extração são etapas distintas; o extrator não chama Cargo.

O diretório de saída recebe **grafo.json** (contrato atual da Lente, validado pelo
`lente_infra::desserializar_grafo`) e **metadados.json** (configuração, proveniência,
classificação dos pacotes, cobertura e diagnósticos). Transporte os dois arquivos
juntos. O grafo sozinho não comunica erros parciais nem a classificação Go.
Cada execução sobrescreve esses arquivos; use diretórios distintos para extrações
concorrentes. A dupla não é uma publicação transacional: descarte-a se houver
erro de I/O. Não há cache de grafos.

Códigos de saída: **0** = carregamento sem diagnóstico; **2** = saída parcial ou
carregamento falhou (arquivos ainda são produzidos); **1** = configuração, execução
do Go, I/O ou contrato inválido. Não consuma arquivos antigos quando o comando
retornar 1. `--help` mostra a sintaxe.

| Opção | Política |
|---|---|
| `--project DIR` | Diretório de execução do Go, obrigatório e canonicalizado |
| `--goos OS`, `--goarch ARCH` | Obrigatórias; validadas pelo Go |
| `--out DIR` | Obrigatória; relativa ao diretório de quem chama |
| `--tags a,b` | Tags adicionais; padrão vazio, ordenadas e sem duplicatas |
| `--tests exclude\|include` | Padrão `exclude`; include carrega variantes e driver, sem executar testes |
| `--work auto\|off\|ARQUIVO` | Padrão `auto`, descoberta oficial de go.work; arquivo explícito relativo ao chamador |
| `--pattern PADRÃO` | Repetível; padrão `./...`; não aceita flags do Go |
| `--module-cache DIR` | Cache Go já existente; padrão usa GOMODCACHE/GOPATH locais, registrados nos metadados |

`go.mod`, `replace`, seleção de versões, workspace e restrições de build são
resolvidos pelo próprio Go. No diretório raiz de um `go.work`, selecione os módulos
explicitamente (o padrão `./...` pode não pertencer a nenhum módulo):

```sh
lab/adaptador-go/target/debug/lente-adaptador-go \
  --project lab/adaptador-go/fixtures/workspace --goos linux --goarch amd64 \
  --work auto --pattern ./one/... --pattern ./two/... \
  --out lab/adaptador-go/target/workspace

lab/adaptador-go/target/debug/lente-adaptador-go \
  --project lab/adaptador-go/fixtures/tests --goos linux --goarch amd64 \
  --tests include --work off --out lab/adaptador-go/target/tests
```

## Estratégia e fontes oficiais

Foi escolhido [`go list`](https://pkg.go.dev/cmd/go#hdr-List_packages_or_modules)
com `-deps -e -json -mod=readonly`, tags e `-test` opcional. `-e` permite preservar
pacotes com erros; `Imports` fornece relações diretas. `Deps` é transitivo e não
gera arestas. `ImportMap` e `ForTest` permitem conservar as variantes de teste.
[`go/packages`](https://pkg.go.dev/golang.org/x/tools/go/packages) não é necessário
para este escopo: o protocolo do comando oficial fornece os dados usados, sem
acrescentar biblioteca Go ou carregar tipos/corpos de funções.

A resolução de [módulos e workspaces](https://go.dev/ref/mod) fica a cargo do Go;
`GOPROXY=off`, `GOSUMDB=off`, `GONOPROXY=none`, `GOPRIVATE` vazio e `GOVCS=*:off`
bloqueiam as rotas de busca remota. `GOTOOLCHAIN=local` desabilita a
[troca automática de toolchain](https://go.dev/doc/toolchain). Se a versão local
não satisfizer `go.mod`/`go.work`, há falha diagnosticada, sem download silencioso.
Disponibilize dependências e toolchain manualmente antes de repetir a análise.
Não se deve contornar o diagnóstico tratando o grafo parcial como completo.

`GOENV=off`, `GOFLAGS` e `GOCACHEPROG` vazios, `GO111MODULE=on`, `GOEXPERIMENT` vazio e níveis de
arquitetura padrão evitam configurações implícitas do usuário. O ambiente efetivo
é registrado via `go env -json`. `CGO_ENABLED=0` é fixo nesta versão. `-mod=readonly`
não autoriza atualizar go.mod; vendor não é usado implicitamente. O Go pode usar
seus caches e gravar dados auxiliares (por exemplo, driver sintético no GOCACHE
ou go.work.sum). Isso não é cache de extrações da Lente. Para auditar sem efeitos
no checkout original, execute sobre uma cópia. Não usamos `-compiled`, `-export`,
`go test`, `go generate`, `go run` ou binários do projeto.

## Contrato e interpretação

- Cada ImportPath completo vira um nó `kind=mod`. IDs numéricos seguem ordem
  lexicográfica e são locais à extração; identidade está em `path`.
- Imports viram `uses`/`import`, sem arestas de fechamento transitivo. Ordenação
  por path/aresta produz JSON determinístico para o mesmo contexto. Metadados
  contêm diretórios absolutos: cópias do checkout não têm bytes idênticos.
- A visibilidade `pub` satisfaz o contrato de entrada; não descreve exportação de
  membros Go. `crate=go-packages` é apenas o rótulo de compatibilidade. Não são
  inventadas posições de funções para pacotes que contêm múltiplos arquivos.
- Origem: `project` para módulos Main (todos os membros ativos do go.work),
  `stdlib` para Standard, `external` para os demais módulos, inclusive replace
  local. Pacotes sem proveniência resolvida ficam `unknown`.
- `normal`, `test-variant`, `external-test` e `test-driver` são identificados nos
  metadados. Sufixos como ` [example.test/testing.test]` permanecem no path; não
  unimos a variante recompilada ao pacote normal. `ForTest` é preservado.
- Erros de pacote/módulo/dependência, Incomplete e falhas do subprocesso aparecem
  em diagnostics; stdout válido anterior a um erro de parsing é preservado.
  Endpoint não carregado recebe nó marcador e diagnóstico, sem inventar imports.
- A classificação Go vive nos metadados; filtros de stdlib baseados em nomes
  Rust **não** devem ser aplicados. Não misture este grafo com grafos Rust.

Carregamento `complete` significa apenas que essa consulta não reportou erros.
Não comprova compilação, cobertura semântica ou equivalência comportamental.
Erros de tipos e corpos podem escapar ao carregamento de imports. Código excluído
por tags, plataforma ou cgo não pertence ao universo consultado. Dependências C,
linker, assembly, plugins, reflexão e recursos embed não viram relações entre
pacotes; relações entre linguagens estão fora deste protótipo.

## Fixtures e testes

```sh
cargo test --offline --manifest-path lab/adaptador-go/Cargo.toml
cargo clippy --offline --manifest-path lab/adaptador-go/Cargo.toml -- -D warnings
```

Os testes Rust chamam somente `go env` e `go list`; as funções Go de teste contêm
panic e não são executadas. Falta de Go é falha explícita, não teste ignorado.

| Fixture | Verificação |
|---|---|
| simple | Um pacote, repetibilidade dos dois documentos |
| chain | a → b → c, isolado; o núcleo calcula distância 2 de c até a sem aresta a → c |
| cached | Dependência externa disponível no cache local, sem replace e sem rede |
| replace | Dependência externa substituída por diretório local |
| workspace | Dois módulos Main, import entre membros de go.work |
| constraints | Tag feature, arquivos linux/arm64, exclusão de cgo |
| tests | Normal, variante interna, externa, driver e stdlib; repetibilidade |
| broken | Dependência ausente e pacote bom preservado; saída parcial |
| syntax | Erro de declaração de pacote |
| newer-toolchain | Requisito Go futuro falha sem tentar baixar toolchain |

`cached/module-cache` é uma fixture mínima de cache expandido: fontes, go.mod,
.info e ziphash, com go.sum correspondente, sem busca remota. Seus hashes são
SHA-256 no formato h1 oficial (hash da lista ordenada de hashes/nome dos arquivos).
Não é uma instrução para editar manualmente caches de produção.

Testes adicionais cobrem stream truncado, endpoint ausente, configuração obrigatória
e variáveis de bloqueio de downloads. Para verificar a saída parcial pela CLI:

```sh
lab/adaptador-go/target/debug/lente-adaptador-go \
  --project lab/adaptador-go/fixtures/broken --goos linux --goarch amd64 \
  --work off --out lab/adaptador-go/target/broken
# Esperado: código 2; leia diagnostics em target/broken/metadados.json.
```

O protótipo mantém todo o stream em memória; custo cresce com o universo de
pacotes (especialmente com testes e stdlib). Não há promessa de tempo. Próximos
passos: ensaios com projetos reais, política vendor/cgo explícita e contrato
compartilhado de proveniência antes de qualquer integração nas superfícies.
