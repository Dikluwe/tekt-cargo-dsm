# Prompt de Nucleação: `lente_cli::saida` — os formatadores de saída (L2)
Hash do Código: 17165eea

**Camada**: L2 — Casca (apresentação pura). Os literais vêm do `lente_catalogo`
(ADR-0002).
**Unidade**: `02_shell/cli/src/saida.rs` (crate `lente_cli`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0020-l2-cli.md` (+ ranking
0027, estrutura 0031/0035, diff 0047, vistas 0048).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

Mapear os **tipos de resultado L1** para **texto humano** ou **JSON** — a saída que
o usuário lê. Apresentação pura: opera só sobre os tipos do `lente_core`/
`lente_estrutura`/`lente_ranking` (re-apontados nos Estágios 1/2 do refactor) e o
catálogo L2. **Não importa o L4** (`lente_wiring`) — a inversão foi fechada no
refactor 0055–0057.

## Comportamento e invariantes

- JSON do diff acrescenta `completude: "completa" | "parcial"` e `falhas: [{"crate_name": ..., "motivo": ...}]` (vazia no caso completo). Os campos anteriores mantêm forma e significado, restritos ao grafo disponível.
- Todas as vistas públicas do diff começam pela completude. Em análise parcial, listam nomes/motivos e avisam que pouco ou nenhum impacto observado não garante ausência de impacto. O censo também se refere ao grafo disponível.
- Teste com zero tocados distingue completo de parcial no JSON e nas três vistas e preserva motivos com aspas/quebras de linha.

- **`formatar(raio, alvo, escopo, modo)`** — o per-nó (texto ou JSON; `--verbose`
  inclui os impactados).
- **`formatar_ranking(itens, escopo, modo)`** — o top-N.
- **`formatar_estrutura(estrut, escopo, modo_uses, modo)`** — módulos/dependências/
  ciclos + ordem da DSM.
- **Vista HTML de estrutura**: a Matriz inicia ativa. O seletor oferece
  também Cartões, com uma unidade por módulo ou grupo visível. As duas vistas
  compartilham seleção, dobra e painel de raio. Selecionar um cartão ou uma
  diagonal mostra o mesmo raio estrutural transitivo já calculado no L1.
- Cada cartão conta unidades vizinhas distintas nas dependências diretas de
  saída e entrada. Exclui a própria unidade da contagem, inclusive em grupos
  dobrados. O cartão do grupo indica quantos módulos agrupa e permite abri-lo.
  Usar o caminho do módulo como identidade e o rótulo abreviado em tela.
- Em Cartões, a vizinhança direta mostra entradas, módulo central e saídas.
  Selecionar um vizinho permite continuar a exploração pelo grafo. O usuário
  pode voltar a todos os cartões ou à Matriz. O painel distingue ligações
  diretas do raio transitivo calculado no L1.
- Para cada ligação direta, o painel mostra os pares de módulos que compõem
  a ligação, com peso e até oito registros de evidência por par quando
  disponíveis. O subtipo é exibido quando conhecido. Um import por pacote
  continua mostrando os pacotes da análise. Registros com contenção
  ambígua não são apresentados como evidência.
- Um índice opcional Go tem `schema_version: 1`, `files` (caminho relativo
  .go para identificador de pacote) e `links` (pares dirigidos `from`/`to`
  com `references` contendo `symbol`, `use_line`, `use_column` e
  `definition_line`, em base um). Todos os arquivos pertencem a pacotes da
  estrutura. Todos os pares ligam arquivos distintos presentes no índice.
  Pode incluir `configuration`, um rótulo do recorte de compilação,
  `sources` (caminho indexado para o código UTF-8 do mesmo snapshot) e a
  `definition_column` de cada referência, em bytes UTF-8 e base um como
  `use_column`. `local_references` pode trazer os mesmos registros
  agrupados por caminho quando o uso e a declaração estão no mesmo arquivo.
  `definitions` pode incluir declarações por caminho indexado, com
  `symbol`, `line` e `column` na mesma convenção. As fontes, colunas
  adicionais, referências locais e declarações são opcionais.
  `search_files` pode acrescentar caminhos que só participam da busca e da
  navegação no código. `search_links` inclui referências nesse mesmo índice
  com a forma de `links`; seus extremos devem pertencer a `files` ou a
  `search_files`. Ambos os campos são opcionais. As fontes, declarações e
  referências locais aceitam caminhos de ambos os conjuntos.
  `mutant_files` pode marcar arquivos de `search_files` como cópias de
  mutantes. A busca e a lista de usos permitem consultar o Bateia sem as
  cópias, somente as cópias ou todos os arquivos. O escopo de usos pode ser
  combinado com o filtro de testes.
  `mutant_comparisons` pode agrupar pares verificados por variante
  (`label`, `files`). Cada par contém `reference`, `mutant` e
  `changes`, uma lista de blocos dirigidos com início e quantidade de
  linhas da referência e do mutante, em base zero. O primeiro par é a
  vista inicial da comparação. Ambos os textos vêm de `sources`. Os
  trechos entre blocos devem ser iguais nos dois lados. Uma variante pode
  incluir `tests`, com `suite`, `run_label`, as contagens de aprovação na
  referência e no mutante e de falha no mutante, e `failures`, a interseção
  dos subtestes que passaram na referência e falharam no mutante. Cada
  entrada tem nome, arquivo, linha e coluna do subteste e pode trazer a
  linha da falha observada. O código de teste também vem de `sources`.
  `mutant_test_matrix` pode incluir a série completa de resultados da mesma
  execução, com `suite`, `run_label`, colunas em `cases`, a linha `reference`
  e linhas por rótulo em `variants`. Cada célula usa `pass`, `fail` ou
  `skip`. Contagens e diferenças devem corresponder aos resultados
  registrados para as comparações individuais. O mapa opcional
  `case_locations` associa nomes de `cases` a `file`, `line` e `column`.
  A posição aponta para o início do nome literal do subteste no código
  incluído em `sources`. Linha e coluna começam em um, e a coluna conta bytes.
- Na presença do índice, o cartão selecionado abre a lista de arquivos com
  filtro por nome. Ao selecionar um arquivo, ele fica no centro da vista com
  cartões para os vizinhos diretos, inclusive de outros pacotes. As setas
  mostram apenas relações que entram ou saem desse arquivo. Cada seta abre
  o mesmo detalhe do painel, com símbolo, uso e declaração nas posições
  fornecidas pelo índice. Selecionar um vizinho muda o centro, e o usuário
  pode voltar à lista. Se o texto do arquivo foi incluído no índice, o uso
  ou a declaração abre o código autocontido na linha indicada. O símbolo é
  destacado na coluna da extração quando o identificador coincide com a
  fonte. No código, clicar num identificador indexado abre sua declaração e
  os usos registrados, inclusive no mesmo arquivo. A identidade usa o
  caminho e a posição da declaração. Se o mesmo ponto tiver mais de um
  vínculo, o painel mostra cada declaração correspondente. O histórico
  retorna às posições abertas dentro do código. Fechar volta à ligação do
  grafo. A busca global por nome está disponível fora do grafo de arquivos.
  Ela distingue declarações pelo caminho e posição, inclui declarações
  indexadas sem uso e abre a declaração ou a lista de usos. Quando o índice
  inclui usos em `_test.go`, a lista pode mostrar apenas esses usos. Um
  trecho `_test.go` na busca filtra os caminhos das declarações. Fechar o código
  preserva a consulta e os resultados da busca. Essas relações não alimentam
  a DSM nem o raio por pacote.
- Ao abrir um arquivo de uma variante com referência, a vista de código
  oferece comparação lado a lado. Ela mantém as linhas comuns alinhadas,
  colore as retiradas e adicionadas com números de linha, permite trocar
  o par e navegar entre os blocos, e retorna ao mesmo arquivo e posição.
  Para variantes com resultados, Testes mostra somente a diferença de
  resultado registrada e oferece atalhos para o subteste e para a linha
  da falha. O retorno preserva a comparação. A matriz de testes pode ser
  aberta na barra principal, com referência e variantes em linhas e
  subtestes em colunas. As células que passam na referência e falham na
  variante abrem a posição registrada, com retorno à mesma célula. O nome
  de cada variante abre diretamente a sua comparação com a referência,
  com retorno à linha, ao foco e à rolagem de origem na matriz. O nome de
  um subteste com posição abre o seu código e destaca o nome registrado.
  Ao retornar, o foco volta ao cabeçalho da mesma coluna e as duas rolagens
  são restauradas. Colunas sem posição mantêm o nome em texto.
  A matriz começa com todos os subtestes. O filtro de falhas exibe as
  colunas com pelo menos um resultado `fail` em `variants`, na ordem
  original e com os atalhos e posições preservados. O resumo informa
  quantas colunas são exibidas, e a seleção do filtro é mantida ao voltar
  do código ou da comparação. Se nenhuma coluna corresponder, a vista
  informa que nenhum subteste falhou nos mutantes.
- A vista com índices SCIP usa uma linha e uma coluna por arquivo. Uma
  célula dirigida abre o uso indexado com a faixa de código destacada e
  oferece atalhos para as declarações e usos do símbolo. Relações em células
  agrupadas preservam os pares de arquivos. O leitor oferece foco no arquivo
  de origem e devolve o foco à célula ao fechar. O raio estrutural continua
  baseado em referências estáticas entre arquivos.
- A cobertura do índice apresenta a contagem de usos ambíguos omitidos da
  matriz quando houver ocorrências sem destino inequívoco.
- No painel de um arquivo com um prompt admitido, **Ver prompt** abre o
  caminho e o conteúdo original em um leitor autocontido com índice de
  seções Markdown. Títulos dentro de código cercado ou comentários HTML
  não compõem o índice. O leitor trata o conteúdo como texto. **Voltar à
  matriz** fecha o leitor e conserva a seleção do arquivo. Quando o leitor
  abriu na matriz, conserva também a rolagem e repõe o foco no botão.
- Dado um arquivo sem prompt válido ou um grupo dobrado, o painel não oferece
  a ação. A vista do prompt não altera os dados de dependência nem o raio.
- Com TextFold, o leitor abre em **Tópicos** e oferece **Síntese**, **Pontos**,
  **Trechos** e **Integral**. Identifica níveis gerados por IA, mostra a
  evidência original de cada unidade ao expandir e mantém o Markdown integral
  acessível no mesmo leitor. O HTML inclui todos os níveis sem dependência de servidor.
  Uma célula FFI abre primeiro a implementação nativa, identifica o
  símbolo de link e mantém atalhos para a declaração e usos Rust.
  O identificador Rust com vínculo FFI confirmado abre a implementação
  no leitor. Na implementação, a lista de chamadas diretas reúne os
  aliases Rust ligados à mesma posição nativa e abre cada chamador
  na linha correspondente. O leitor preserva a célula de origem.
  O grafo de chamadas mostra simultaneamente as ramificações dos
  chamadores Rust alcançáveis até a implementação nativa. Cada função
  ocupa um nó por posição de declaração e cada local de chamada mantém
  uma seta própria e acionável com mouse ou teclado. Os aliases Rust
  convergem na mesma implementação confirmada. A seta abre o local de
  chamada e o nó abre a fonte correspondente no mesmo leitor. A
  visualização desenha as ligações que fecham ciclos sem reexpandir os
  nós ou enumerar rotas, e permite rolar grafos maiores que o painel.
  Acionar um nó chamador Rust destaca todas as setas alcançáveis na
  direção das chamadas até o nativo, incluindo ramificações e retornos
  de ciclos. O resto do grafo permanece visível e acionável. Abrir uma
  seta ou a implementação mantém o destaque, que pode ser trocado
  ao selecionar outro chamador ou limpo no próprio grafo.
  A comparação mantém a primeira origem e permite escolher uma segunda
  origem Rust distinta. Distingue no mesmo grafo os caminhos exclusivos
  de cada origem, a interseção e os nós em que um ramo exclusivo entra
  na interseção ou uma das origens já pertence a ela. Cada seta conserva
  a posição da chamada. Outra seleção durante a comparação substitui a
  segunda origem. Encerrar a comparação restaura o destaque da primeira.
  Acionar um nó marcado como encontro abre sua declaração Rust ou a
  implementação nativa e lista as chamadas recebidas pelos caminhos de
  A, B e em comum. Cada item abre o local da chamada sem fechar o painel
  ou alterar a dupla. Uma origem que começa no encontro aparece como
  tal. O painel permite escolher um encontro Rust como B e é recolhido
  quando a dupla muda ou a comparação termina.
  No painel, Anterior e Próximo percorrem os encontros da dupla na ordem
  visual do grafo, da esquerda para a direita e de cima para baixo em
  cada coluna. Cada passo abre o código e as chamadas do encontro e
  atualiza o marcador ativo e o contador. Os botões param nos limites.
  A lista lateral apresenta todos os encontros da dupla na mesma
  ordem. Cada item informa o símbolo e a posição e abre diretamente
  o encontro correspondente. A seleção da lista acompanha o marcador
  do grafo e os botões de navegação enquanto a dupla é mantida.
  O filtro da lista aceita partes do nome da função ou do caminho
  indexado do arquivo, sem distinguir maiúsculas de minúsculas.
  Atualiza a quantidade visível e informa quando não há resultados.
  O filtro e a comparação são preservados ao abrir código. Anterior
  e Próximo continuam percorrendo todos os encontros da dupla.
  O grafo FFI reúne chamadores Rust e C/C++ com caminho até a
  implementação selecionada. A e B podem combinar essas linguagens.
  Os encontros abrem a declaração correspondente e os links abrem
  cada chamada no arquivo de origem, mantendo a comparação.
  A vista inclui também as chamadas nativas confirmadas a partir da
  implementação FFI e os chamadores desses destinos. As origens
  podem entrar por funções diferentes e compartilhar um nó nativo
  posterior. O nó em comum é clicável mesmo quando é uma função folha.
  As pontes confirmadas participam da matriz e do mesmo grafo. A vista
  distingue Rust, C, C++, Python, Java, Go, C#, JavaScript, TypeScript,
  Kotlin, Dart e Zig nos nós e nas chamadas e mantém a comparação ao abrir
  um encontro nativo.
  A vista distingue Swift, Objective-C e Objective-C++ e mostra IndexStore
  como evidência das fronteiras produzidas pela toolchain.
- As interações acontecem no HTML autocontido. O JSON público de estrutura
  e o comando de geração mantêm seus contratos.
- **`formatar_diff(resultado)`** — o JSON view-agnóstico do `--diff` (0047).
- **`formatar_diff_vista(resultado, vista)`** — as três vistas de texto (0048):
  `resumo` (impacto por crate, ênfase adaptativa), `item` (bloco por tocado),
  `camadas` (agrupado por crate).
- **`Modo`** (`text`/`verbose`), **`AlvoPedido`** (`Path`/`Id` — mostra a tradução
  id→path quando o usuário pediu por id).
- **JSON montado à mão** (`serde_json::Map`), chaves do catálogo; texto com rótulos
  do catálogo. **Determinístico** (ordena o que itera).

## Restrições (L2 — apresentação)

- Importa só L1 (`lente_core`/`lente_estrutura`/`lente_ranking`) + `lente_catalogo`
  + `serde_json`/`clap`. **Sem `lente_wiring` (L4)** — V3 = 0.

## Critérios de Verificação

```
Dado um Raio Quando formatar (JSON) Então {alvo, classificacao, diretos, transitivos, escopo}
Dado um ResultadoDiff Quando formatar_diff_vista(Resumo) Então contagem por crate + censo + solto
Dado diff só-arquivo-novo Então a vista resumo lidera com o jusante (ênfase adaptativa)
Dado HTML de estrutura Quando alternar para Cartões Então exibir as unidades com suas contagens diretas
Dado módulo selecionado Quando alternar entre Cartões e Matriz Então manter seleção e painel de raio
Dado grupo dobrado Quando abrir pelo cartão Então exibir seus módulos e atualizar as contagens
Dado um cartão selecionado Quando focar Então mostrar apenas ele, entradas e saídas diretas
Dado vizinho direto Quando abrir os detalhes Então mostrar os pares de módulos na direção do uso
Dado um arquivo Go selecionado Então desenhar apenas as setas diretas que incidem nele
Dada uma seta Go Quando acionada Então abrir as referências do par dirigido no painel
Dada uma referência Go com fonte Quando abrir a posição Então mostrar a linha e marcar o identificador na coluna registrada
Dado um símbolo Go indexado Quando selecioná-lo no código Então permitir ir à declaração e aos usos locais e externos ao arquivo
Dado um nome indexado Quando buscá-lo da Matriz Então abrir a declaração ou os usos sem mudar para o grafo de arquivos
Dado um arquivo só da busca Quando indexar declarações e usos Então mantê-los acessíveis na busca e no código
Dadas cópias de mutantes indexadas Quando mudar o escopo Então filtrar declarações e usos pela origem, preservando os vínculos de cada cópia
Dado um mutante com par verificado Quando comparar Então alinhar linhas comuns e destacar os blocos nos lados correspondentes
Dado um subteste que passou na referência e falhou no mutante Quando abrir seu resultado Então navegar à fonte registrada e poder retornar à comparação
Dadas variantes com resultados completos Quando abrir a matriz de testes Então mostrar todos os subtestes e usar a falha da interseção correta
Dada uma linha de variante Quando acionar seu nome Então abrir a comparação e restaurar o foco na mesma linha ao voltar
Dado um subteste com posição registrada Quando acionar o cabeçalho Então abrir sua linha no código e restaurar o foco e a rolagem da coluna ao voltar
Dada a matriz Quando ativar o filtro de falhas Então exibir somente colunas com falha em alguma variante e manter os atalhos dos cabeçalhos e das células
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"formatar","params":["&Raio","&AlvoPedido","Escopo","&Modo"],"return_type":"String"},{"name":"formatar_ranking","params":["&[ItemRanking]","Escopo","&Modo"],"return_type":"String"},{"name":"formatar_estrutura","params":["&EstruturaModulos","Escopo","ModoUses","&Modo"],"return_type":"String"},{"name":"formatar_estrutura_html","params":["&EstruturaModulos","Escopo","ModoUses","&str"],"return_type":"String"},{"name":"formatar_estrutura_html_com_evidencias","params":["&EstruturaModulos","Escopo","ModoUses","&str","&AgregacaoDetalhada"],"return_type":"String"},{"name":"formatar_estrutura_html_com_referencias_go","params":["&EstruturaModulos","Escopo","ModoUses","&str","&AgregacaoDetalhada","&str"],"return_type":"Result<String, String>"},{"name":"formatar_estrutura_html_com_chamadas_go","params":["&EstruturaModulos","Escopo","ModoUses","&str","&AgregacaoDetalhada","&str","&lente_core::domain::grafo_chamadas::GrafoChamadas"],"return_type":"Result<String, String>"},{"name":"formatar_estrutura_html_scip","params":["&EstruturaModulos","&FontesIndexadas"],"return_type":"String"},{"name":"formatar_estrutura_html_scip_com_chamadas","params":["&EstruturaModulos","&FontesIndexadas","&lente_core::domain::grafo_chamadas::GrafoChamadas"],"return_type":"String"},{"name":"formatar_estrutura_html_scip_com_fluxos","params":["&EstruturaModulos","&FontesIndexadas","&lente_core::domain::grafo_chamadas::GrafoChamadas","&str"],"return_type":"Result<String, String>"},{"name":"formatar_diff","params":["&ResultadoDiff"],"return_type":"String"},{"name":"formatar_diff_vista","params":["&ResultadoDiff","Vista"],"return_type":"String"},{"name":"formatar_comparacao","params":["&Comparacao","Escopo","ModoUses","&Modo"],"return_type":"String"}],"types":[{"name":"AlvoPedido","kind":"enum","members":["Path","Id"]},{"name":"Modo","kind":"struct","members":["text","verbose"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-08-26 | P0079: saída textual e JSON separam correspondências declaradas, unknown e proveniência/hash do mapa; alegação nunca é rotulada como veredito funcional. | `02_shell/cli/src/saida.rs` |
| 2026-06-07 | Nucleação (migração de convenção, prompt 0063) dos formatadores. Código inalterado. | `02_shell/cli/src/saida.rs` |

| 2026-09-26 | Correção da perda de falhas no diff: completude explícita, resultados parciais consultáveis e erro se nenhum crate for analisado. | Unidade deste contrato |

Explorar chamadas abre do cartão ou da busca e conserva o símbolo de partida. Mostra vizinhos diretos dos dois lados. Cada expansão pertence à ocorrência do ramo e carrega apenas um nível. A seta abre a fonte destacada na linha da chamada sem descartar o ramo.

Ao selecionar uma seta, mostrar o indício sintático e o índice de referência no inspetor. Listar pontos sem seta no ramo de saída por motivo e com o ponto e candidatos clicáveis. O filtro por função ou arquivo atua sobre as setas resolvidas e o painel identifica separadamente as ocultas, preservando ramos abertos e código selecionado.

Uma seta resolvida no Explorador permite salvar o caminho desde
a função de partida, na direção que o usuário percorreu. A declaração
de partida e cada ponto de chamada formam passos clicáveis. O navegador
guarda caminhos nomeados por endereço do HTML usando posições estáveis.
Antes de abrir, a vista confere a continuidade do caminho e as posições
no grafo carregado. Erro de gravação ou caminho desatualizado aparece na
tela. Alternar entre grafo e fluxo mantém os ramos já expandidos.

A navegação por chamadas marca a origem sintática de Go e de C/C++ e mostra
a cobertura nativa com o motivo das unidades excluídas. Fluxos fornecidos
em JSON pelo usuário são visíveis como exemplos publicados com o HTML,
conservam a checagem das posições do grafo e convivem com os fluxos no navegador.
