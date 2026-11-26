# Lente de Forma e Consequência

Uma ferramenta de análise de dependências para Rust que responde a uma pergunta:
**"o que quebra se eu mexer aqui?"**

A lente lê um programa Rust como um grafo dirigido de dependências e responde
sobre ele de três formas — o impacto de mexer num item, o ranking dos itens por
impacto, e a estrutura de arquitetura do projeto inteiro (dependências entre
módulos e ciclos, na forma de uma matriz DSM). É, na prática, um
**Lattix / Structure101 para Rust**.

---

## Por que existe

Ler um programa grande inteiro é impraticável — para um humano e para a janela de
contexto de uma IA. A lente **comprime** o programa a uma forma que cabe na
decisão: milhares de itens viram dezenas de módulos, que viram o bloco que de
fato importa. Cada passo joga fora o detalhe que não serve à pergunta e mantém o
que serve.

O valor dela vem de operar sobre estrutura **resolvida**, não sobre texto. Onde
o `grep` acha onde um nome aparece, a lente sabe **o que de fato depende de quê**:
a relação é **dirigida** (A depende de B é diferente de B depende de A), a
identidade é **resolvida** (dois itens de mesmo nome em módulos diferentes não se
confundem) e o alcance é **transitivo** (quem depende de quem depende de X). Uma
ordem topológica ou um ciclo de módulos não saem de nenhuma ferramenta de texto,
porque texto não tem direção nem resolução.

---

## O que ela faz hoje

- **Raio** — dado um item, quem depende dele (o alcance de mexer nele), com uma
  classificação (folha, base, intermediário, isolado).
- **Ranking** — os itens do crate ordenados por impacto.
- **Estrutura (DSM)** — a dependência módulo→módulo do projeto, os **ciclos**
  (componentes fortemente conexos), e a **ordem** que põe as dependências de um
  lado da diagonal e os ciclos como blocos densos nela. Sai em texto e em JSON;
  um protótipo de tela renderiza a matriz.

Dois recortes, ambos com a **saída declarando qual está em uso**:

- **Escopo** (`--filtrar-stdlib`): o seu código apenas, ou o seu código com a
  stdlib. Por padrão inclui a stdlib; a flag filtra.
- **Modo de `uses`** (`--so-referencia`): contar todas as dependências, ou só as
  **referências de tipo** (descartando os imports no nível do módulo). Por padrão
  conta todas; a flag mostra só o acoplamento de tipo genuíno.

O segundo recorte revela algo concreto. No `egui`, a vista de ciclos mostra um
anel de 85 módulos — 76% do crate. Ao contar só referências, o anel cai para
**42 módulos**: a outra metade era acoplamento aparente, vindo de declarações
`use` no topo dos módulos, não de uso real de tipo. A lente separa o acoplamento
real do inflado, e diz qual você está vendo.

---

## Como funciona

```
código Rust
   │
   ▼  (fork do cargo-modules, sobre rust-analyzer)
grafo cru, resolvido e dirigido  ──── export-json
   │
   ▼  (lente: L3 lê o JSON → forma organizada; L1 resolve colisões de path)
grafo resolvido (paths únicos)
   │
   ├── raio (quem depende de um item)
   ├── ranking (itens por impacto)
   └── estrutura: agregar a módulos → detectar ciclos → ordenar a DSM
```

A fonte do grafo é um **fork do `cargo-modules`** que usa o motor do
rust-analyzer — o mesmo que uma IDE usa — para resolver nomes, tipos e traits. É
de lá que vêm a direção e a identidade resolvida. A lente é independente da fonte:
ela mira uma "forma organizada" do grafo (definida em
`00_nucleo/specs/forma-organizada.md`), e tudo acima dessa forma — raio, ranking,
ciclos, DSM — não sabe que linguagem a produziu.

## DSM por arquivo com SCIP

O modo SCIP reúne referências de Rust, C e C++ em uma matriz navegável.
Gere os índices e o HTML a partir do mesmo snapshot da raiz do projeto.
Para C e C++, o `compile_commands.json` deve refletir as unidades e as
opções da compilação que você quer analisar.

```sh
rust-analyzer scip --output rust.scip .
scip-clang --compdb-path=compile_commands.json --index-output-path=clang.scip
lente --estrutura --html --raiz-scip . \
  --indice-scip rust.scip --indice-scip clang.scip --saida lente-scip.html
```

Use apenas a opção `--indice-scip` da linguagem que quer analisar num
projeto com um único indexador. A linha da matriz indica o arquivo do uso
e a coluna indica o arquivo da declaração. Uma célula abre a posição do
uso e oferece as declarações e os demais usos indexados do símbolo.
O painel permite focar a vizinhança direta do arquivo.

Selecione um arquivo pela diagonal da matriz. Se ele tiver um `@prompt`
apontando para um Markdown local sob `00_nucleo/prompts/` ou `00_nucleo/prompt/`,
o painel oferece **Ver prompt**. O leitor inclui o texto e um índice clicável
das seções no próprio HTML. **Voltar à matriz** conserva o arquivo selecionado.

### Nomes das colunas e testes pela CLI

Na matriz por arquivo, **Nomes das colunas** alterna entre **Caminho completo**
e **Nome do arquivo**. Passe o cursor pelo cabeçalho para consultar o caminho
inteiro em qualquer modo.

O painel do arquivo da regra pode incluir **Testes pela CLI**. Cada nome abre
o arquivo de teste na DSM. A seção informa a revisão e quantas suítes foram
medidas. Montante e jusante continuam seguindo as referências estáticas do SCIP.

Gere o manifesto e o índice a partir do mesmo estado do código. No
`tekt-linter`, o coletor de laboratório usa Python 3 e o componente
`llvm-tools` da toolchain ativa do Rust. Passe os alvos Cargo que quer medir:

```sh
python3 lab/coletar-cobertura-cli.py -o target/cobertura-cli.json \
  analysis_policy_cli resolution_completeness_cli v19_report_cli
rust-analyzer scip --output target/rust.scip .
lente --estrutura --html --raiz-scip . --indice-scip target/rust.scip \
  --cobertura-cli target/cobertura-cli.json --saida target/lente-cobertura.html
```

O JSON tem `versao: 1`, `revisao`, `testes` (suítes medidas) e `regras`
(arquivo da regra → suítes que executaram checagens). Os caminhos são relativos
à raiz e pertencem ao índice. Uma lista vazia registra que nenhuma execução
da regra foi observada nas suítes da coleta. A indicação de execução vale para
o arquivo de teste e para o escopo registrado. As asserções de cada caso são
descritas pelo próprio teste. O HTML incorpora o manifesto e abre offline.

### Níveis TextFold para prompts Tekt

O TextFold é uma dependência externa e opcional. Instale o pacote Python em
um ambiente virtual fora do checkout, reutilizável entre projetos Tekt, e
mantenha o `python3` desse ambiente no `PATH` ao gerar a DSM.

```sh
ambiente_textfold="${XDG_DATA_HOME:-$HOME/.local/share}/tekt/textfold-venv"
python3 -m venv "$ambiente_textfold"
. "$ambiente_textfold/bin/activate"
python3 -m pip install 'textfold==0.10.57'

mkdir -p target
rust-analyzer scip --output target/rust.scip .
lente --estrutura --html --raiz-scip . --indice-scip target/rust.scip \
  --textfold --saida target/lente-textfold.html
```

Em **Ver prompt**, alterne entre **Tópicos**, **Síntese**, **Pontos**, **Trechos**
e **Integral**. O adaptador L3 usa os níveis disponíveis na biblioteca local do
TextFold ou a extração local, sem chamar um modelo. O HTML abre offline e a
vista integral conserva o Markdown do repositório.

O HTML incorpora o código dos arquivos indexados. Com `--artefato-ffi`,
a Lente também liga funções externas de Rust às implementações C/C++
de ABI C que constam no artefato. Passe o executável ELF final (ou a
biblioteca compartilhada que contém a implementação) com símbolos e
informações de depuração, gerado a partir do mesmo build e das mesmas
fontes dos índices. O adaptador usa GNU `nm` e `llvm-dwarfdump` no `PATH`.

```sh
lente --estrutura --html --raiz-scip . \
  --indice-scip rust.scip --indice-scip clang.scip \
  --artefato-ffi caminho/do/elf --saida lente-scip.html
```

Uma célula FFI abre a implementação diretamente e oferece a declaração
`extern` e os usos em Rust. O nome de exportação pode vir de
`#[link_name]`. A ligação exige um símbolo global de código e o mesmo
endereço no DWARF. O código da implementação vem do arquivo C/C++
indexado, na linha de declaração registrada no binário.

No leitor Rust, clique no nome sublinhado dentro de `extern` para abrir
a implementação nativa. Na implementação, **Chamadas na fronteira FFI**
reúne as chamadas ligadas à mesma função e leva à linha de cada uma.
As referências ao valor da função aparecem em **Outros
usos indexados**.

O **Grafo de chamadas** reúne os caminhos de chamadas diretas que o SCIP
resolve até a implementação em C/C++. Cada seta abre a linha da chamada
e destaca o símbolo. Clique num nó para abrir a declaração ou
implementação. Setas tracejadas indicam ligações que fecham ciclos.
Selecionar um chamador destaca seus caminhos até a implementação.
As demais setas continuam visíveis e clicáveis. Use **Limpar destaque**
para voltar à visão geral.

Para comparar duas origens, selecione o primeiro chamador e acione
**Comparar chamadores**. Selecione o segundo no grafo. A legenda
identifica os caminhos exclusivos de A e B e os trechos em comum.
Clique num ponto de encontro para abrir o código e ver as chamadas de A,
B e em comum que chegam ali. Os links abrem cada chamada e mantêm a
comparação. No grafo, selecionar outro chamador troca B e mantém A.
Para usar um encontro que seja chamador como B, use a ação no painel.
No painel, **Anterior** e **Próximo** percorrem os encontros na ordem do
grafo e abrem o código e as chamadas de cada ponto com A e B selecionados.
A lista **Encontros** aparece ao escolher A e B. Clique em qualquer
entrada para abrir seu código e suas chamadas na mesma comparação.
Use **Filtrar por função ou arquivo** para buscar na lista por nome ou
parte do caminho. **Anterior** e **Próximo** percorrem a comparação inteira.
A e B podem combinar chamadores Rust, C e C++ que chegam à mesma
implementação FFI ou se encontram depois dela em uma função nativa.
Para incluir as chamadas C/C++, disponibilize o
`compile_commands.json` em `--raiz-scip` ou no subdiretório `build` e
o comando `clang` no PATH. A Lente cruza as chamadas diretas por nome
detectadas pelo Clang com as posições e identidades do índice SCIP das
unidades compiladas. Se o SCIP omitir uma chamada ao destino FFI, o nome
externo resolvido pelo Clang pode ligá-la à implementação já confirmada
no ELF/DWARF.


### Explorar chamadas Go

Em **Cartões**, selecione um pacote ou arquivo e abra **Explorar chamadas**.
Na busca por símbolo Go, a ação abre a função do resultado diretamente.
Com um índice Go via SCIP, use também **Buscar função** no topo.

O painel mostra **Chamadas** e **Chamadores** imediatos. Clique em
**Expandir** para abrir um nível daquele ramo. A seta abre o uso na linha
da chamada com o identificador destacado. **Declaração** abre o corpo do
vizinho. A exploração usa as referências resolvidas do snapshot e a
sintaxe Go para identificar chamadas, com a toolchain Go no PATH.

```sh
lente --estrutura --html --raiz-scip . \
  --indice-scip bateia.scip --saida lente-bateia-chamadas.html
```

O índice legado de `--referencias-go` também funciona quando inclui a
fonte e as posições de uso e declaração. A vista cobre chamadas diretas
entre funções nomeadas indexadas com corpo.

A seta também mostra a identificação sintática por Go e o arquivo de
índice que ligou o uso à declaração. Em **Pontos sem seta**, cada
entrada abre o código e explica se faltou uma referência no índice,
faltou a declaração, houve destinos conflitantes ou o destino ficou
fora do recorte. Destinos candidatos aparecem com a origem de cada um.
**Filtrar ligações** restringe os vizinhos por função ou arquivo.
**Ocultas pelo filtro** permite consultar as ligações resolvidas que o
filtro escondeu. Limpar o filtro restaura os ramos abertos.

Para guardar um caminho, expanda o ramo no grafo e selecione a seta do
último passo que quer incluir. Clique em **Salvar fluxo** e dê um nome.
O fluxo começa na declaração de partida e conserva as chamadas até a
seta escolhida. Clique nos passos ou use **Anterior** e **Próximo**
para abrir o código. **Fluxos salvos** recupera os caminhos ao reabrir
o mesmo HTML no mesmo navegador. A Lente confere os passos com o índice
do HTML antes de mostrar a navegação.

### Explorar chamadas C e C++

Gere o índice SCIP e passe o `compile_commands.json` do mesmo build. A Lente
usa Python 3 e a libclang do sistema. Se necessário, indique a biblioteca via
`LENTE_LIBCLANG`.

```sh
scip-clang --compdb-path=build/compile_commands.json --index-output-path=clang.scip
lente --estrutura --html --raiz-scip . --indice-scip clang.scip \
  --base-compilacao build/compile_commands.json --saida lente-cpp.html
```

Use **Buscar função** para navegar por métodos qualificados como
`SoGLRenderActionP::renderSingle`, ou **Explorar chamadas** no cartão do arquivo.
A cobertura no topo informa quantas unidades foram analisadas e os motivos
das exclusões. O grafo liga chamadas diretas a corpos nas unidades compiladas.
Métodos virtuais mostram o destino estático. Chamadas indiretas, definições
apenas em cabeçalhos e identificadores expandidos por macros podem ficar de fora.

Para distribuir um caminho junto do HTML, passe `--fluxos fluxos.json`. O
arquivo usa a lista de fluxos do armazenamento local da Lente. Cada item tem
`nome`, `raiz`, `lado` (0 para chamadas, 1 para chamadores) e `passos` (uma
lista de triplas `[declaração de origem, declaração de destino, uso]`). Cada
posição tem `[arquivo, linha, coluna, linha final, coluna final]`, com
linhas a partir de 1 e colunas UTF-8 a partir de 0. Os fluxos publicados
aparecem junto aos fluxos locais em **Fluxos salvos**.

### Python ctypes e Java JNI

Inclua os índices SCIP de Python, Java e C/C++ e passe a biblioteca
compartilhada ELF com informações de depuração em `--artefato-ffi`.
O modo analisa Python com `python3` e Java com um JDK 17 ou superior.
Use o mesmo `compile_commands.json` das fontes nativas.

```sh
lente --estrutura --html --raiz-scip . \
  --indice-scip python.scip --indice-scip java.scip --indice-scip clang.scip \
  --artefato-ffi build/libmisto.so --saida lente-ffi.html
```

Em Python, a Lente acompanha `ctypes.CDLL` com caminho literal e
chamadas de atributo do objeto retornado. Caminhos relativos são
resolvidos a partir de `--raiz-scip`. Em Java, acompanha invocações
resolvidas de métodos `static native` com nome JNI curto exportado e
carga literal via `System.load` ou `System.loadLibrary` na classe.
Sobrecargas nativas, registros JNI dinâmicos e resolução de nomes
calculados da biblioteca ficam fora desse recorte.

Abra uma célula FFI de Python ou Java na matriz e use **Comparar
chamadores** para selecionar as duas origens. O grafo reúne os caminhos
que chegam ao mesmo código nativo e permite abrir a declaração ou as
chamadas de cada ponto de encontro.

### Go, C#, JavaScript/TypeScript, Kotlin/JVM, Dart e Zig

Passe os índices SCIP das linguagens que quer comparar, o índice de C/C++
e o ELF com símbolos e depuração do mesmo build. A extração dessas pontes
usa `python3` para ler a fonte, sem executar o projeto indexado.

| Linguagem | O que a Lente liga |
| --- | --- |
| Go | Chamada direta `C.funcao(...)` via cgo. O executável ou biblioteca final deve conter o wrapper gerado e a definição C/C++. |
| C# | Métodos `static extern` com `DllImport` e `static partial` com `LibraryImport` de `System.Runtime.InteropServices`. Usa o `EntryPoint` literal ou o nome do método. |
| JavaScript/TypeScript com Deno | `const` inicializado por `Deno.dlopen` com caminho e tabela de símbolos literais, seguido de `lib.symbols.funcao(...)`. |
| JavaScript/TypeScript com Node-API | `const` inicializado por `require("./addon.node")` e chamada direta do método exportado. O índice C/C++ deve conter o registro literal em `NAPI_MODULE_INIT`, com `napi_create_function` seguido de `napi_set_named_property` e retorno de `exports`. |
| Kotlin/JVM | `external fun` sem sobrecarga em `object` com `System.loadLibrary("nome")` literal no inicializador. Usa o nome JNI curto da classe JVM. |
| Dart | `final` inicializado por `DynamicLibrary.open` com caminho literal, seguido de `lookupFunction<Native, Dart>("simbolo")` em outro `final` chamado diretamente. |
| Zig | Chamadas diretas a `extern fn` de ABI C e a funções importadas por `const c = @cImport({ @cInclude("arquivo.h"); });`. A implementação C ou C++ com ABI C precisa estar no mesmo ELF. |

O caminho relativo do Deno ou Dart parte de `--raiz-scip`. O caminho
de `require` parte do diretório do próprio arquivo. Em P/Invoke e JNI, a
Lente confere o nome da biblioteca ou um alias na raiz. Esse recorte
cobre ligações literais, sem sobrecarga Kotlin e com as formas de
registro de addon descritas acima.

Este comando compara C# e Kotlin com duas entradas na mesma biblioteca.

```sh
lente --estrutura --html --raiz-scip . \
  --indice-scip dotnet.scip --indice-scip kotlin.scip --indice-scip clang.scip \
  --artefato-ffi build/libmisto.so --saida lente-ffi.html
```

Abra uma célula FFI, selecione o primeiro chamador, clique em **Comparar
chamadores** e selecione o segundo. As setas abrem os usos e a lista
**Encontros** leva às declarações compartilhadas e às chamadas de A e B.

Para Zig, gere um índice como `cog-zig --output zig.scip src/main.zig`
a partir de um projeto com `build.zig.zon`. Inclua os arquivos Zig que
quer navegar e passe `--indice-scip zig.scip` e `--indice-scip clang.scip`
com `--artefato-ffi` apontando para o executável ou biblioteca vinculada.
O recorte cobre declarações de topo com ABI C e inclusões literais via `@cInclude`.
Aliases de macros, chamadas indiretas e bibliotecas usadas apenas via loader
em tempo de execução ficam fora do recorte.


### Swift com C, C++ e Objective-C

A Lente usa o IndexStore produzido pelo compilador para associar cada
chamada Swift à definição nativa com a mesma identidade USR. A matriz abre
o corpo da função, método ou seletor na linha indexada. Sobrecargas C++
e seletores iguais em classes Objective-C têm identidades distintas.

Com SwiftPM, gere o IndexStore com `swift build --enable-index-store`.
Use `swift build --show-bin-path` para localizar o diretório de build e
passe o subdiretório `index/store`. Em Xcode, habilite a indexação do
build e use `Index.noindex/DataStore` do DerivedData. O mesmo Store
deve incluir as unidades de implementação nativa compiladas. Para
compilar manualmente, passe `-index-store-path` ao Swift e ao Clang.

```sh
lente --estrutura --html --raiz-scip . \
  --indice-swift caminho/do/IndexStore --saida lente-swift.html
```

O modo usa `python3` e encontra a `libIndexStore` pela toolchain
Swift no PATH ou pelo Xcode ativo. Para outra instalação, indique o
arquivo da biblioteca em `LENTE_LIB_INDEXSTORE`. É possível combinar
`--indice-swift` com `--indice-scip` e o ELF das demais pontes.

A navegação Objective-C abre a implementação estática identificada no
índice. O despacho em execução pode escolher uma subclasse ou outra
implementação. Ocorrências com múltiplos destinos ou sem definição
nativa indexada ficam sem ligação FFI.

---

## O método: Tekt

O projeto é construído sob uma metodologia própria, e os artefatos do método são
tão centrais quanto o código:

- **Camadas** — `L0` semente (specs, ADRs) · `L1` núcleo puro (sem dependências
  externas) · `L2` shell (CLI, catálogo de mensagens) · `L3` infra (o fork, o
  `cargo metadata`) · `L4` fiação · `L5` laudos.
- **Cada componente nasce de um prompt numerado** (`00_nucleo/prompt/`). Cada
  execução produz um **laudo** (`00_nucleo/lessons/`) que registra o que foi
  feito, o que foi verificado, e o que ficou em aberto. Decisões de projeto viram
  **ADRs** (`00_nucleo/adr/`).
- **Medir antes de afirmar.** Hipótese sem dado é só hipótese. Experimentos e
  medições descartáveis vivem em `lab/` (a "Arena") e são jogados fora depois de
  ensinarem — o que fica é o achado, registrado num laudo.
- **Honestidade sobre os limites** é parte da spec, não nota de rodapé.

---

## Limites declarados

A lente é uma ferramenta de **forma**, não de **comportamento**. Saber os limites
evita esperar dela o que ela não dá:

- **Estrutural, não comportamental** — a lente vê o que um item **referencia**
  (chama, menciona em assinatura, implementa), não o que o código **faz em
  execução**. Um grafo de chamadas em tempo de execução está fora do alcance, por
  construção.
- **Extração nativa por pacote Rust** — depende da resolução do rust-analyzer.
  O modo SCIP consome os índices gerados com a configuração de cada linguagem.
- **Precisa do fork instalado** e usa a stdlib carregada (`--sysroot`) por
  padrão, porque metade das arestas de alguns crates vem de `#[derive(...)]` e
  sumiriam sem ela.
- Outros limites medidos (fronteira com a stdlib, agregação de imports,
  reexports, colisões geradas por macro) estão documentados em
  `00_nucleo/specs/forma-organizada.md`.

---

## Estrutura do repositório

| Caminho | O que é |
|---------|---------|
| `00_nucleo/` | Semente: `specs/`, `adr/`, `prompt/`, `lessons/` (laudos). |
| `01_core/` | L1 — tipos do grafo (`No`, `Aresta`, `Grafo`) e cálculo do raio. |
| `02_shell/catalogo/`, `02_shell/cli/` | L2 — mensagens e a CLI (`lente`). |
| `03_infra/` | L3 — invocação do fork, `cargo metadata`, desserialização. |
| `04_wiring/` | L4 — fiação que liga a CLI ao núcleo. |
| `05_investiga/`, `06_resolve/` | L1 — investigação e resolução de colisões de path. |
| `07_filtro/` | L1 — filtros de grafo (stdlib, só-referência). |
| `08_ranking/` | L1 — ranking por impacto. |
| `09_estrutura/` | L1 — agregação a módulos, detecção de ciclos, ordenamento da DSM. |
| `lab/` | Arena — protótipos e medições descartáveis. |

A identidade de um nó é o `id` (atribuído pela fonte); o `path` pode colidir
entre nós distintos, e a colisão é resolvida pela cascata `investiga → resolve`.

---

## Uso

> Os comandos abaixo são ilustrativos; confirme a sintaxe exata na ajuda da CLI
> (`lente --help`). A lente roda o fork no diretório de trabalho atual, então é
> executada **de dentro do crate alvo** (ou apontando para ele).

Instale o fork (a fonte do grafo):

```bash
cargo install --path <caminho-do-fork-cargo-modules>
```

Construa a lente e rode a partir do crate que você quer analisar:

```bash
# na raiz deste workspace
cargo build --release -p lente_cli

# de dentro do crate alvo
cd /caminho/para/o/crate
/caminho/para/lente --pacote <nome-do-crate> --ranking --top 10 --text
```

Exemplos:

```bash
# ranking de impacto, só o seu código
lente --pacote meu_crate --ranking --top 10 --filtrar-stdlib

# raio de um item específico
lente --pacote meu_crate --alvo meu_crate::modulo::Item --text

# estrutura de arquitetura: ciclos + ordem da DSM, só acoplamento de tipo
lente --pacote meu_crate --estrutura --so-referencia --text

# a mesma estrutura como dado, para outra ferramenta consumir
lente --pacote meu_crate --estrutura --so-referencia --json
```

---

## Para agentes de IA

A saída `--json` é contexto estrutural **comprimido e verificável** — um agente
que vai mexer no código pode consumir o `--estrutura --json` (módulos,
dependências, ciclos, ordem da DSM) sem precisar ler o programa inteiro, e o que
ele cita é um fato derivado do grafo, não um palpite. A lente não compete com o
`grep` que o agente já tem; ela oferece a camada resolvida e dirigida que o
`grep` não alcança. (Empacotá-la como ferramenta plugável — MCP — ainda não foi
feito; o JSON já é o insumo.)

---

## Estado

- **Vista de arquitetura (global)** — completa: do grafo cru à matriz DSM, no
  terminal e em protótipo de tela.
- **Vista de impacto local** — em construção: mapear uma mudança (um diff) aos
  itens que ela toca e mostrar o raio **antes** de executá-la. O fork já emite a
  posição de cada item no fonte (arquivo + linha); falta a lente consumi-la.

O histórico completo de decisões está nos prompts (`00_nucleo/prompt/`) e laudos
(`00_nucleo/lessons/`).
