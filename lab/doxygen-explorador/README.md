> Integração atual: consulte [o guia comum](../../docs/doxygen.md) e a revisão
> ADR-0009 no CONTRATO.md. Inventários novos são v2, com pendências separadas.
> Relatos e contagens abaixo descrevem a entrega isolada anterior; a validação
> integrada está no laudo da rodada.

# Explorador documental Doxygen

Painel experimental independente para inspecionar estrutura e documentação.
Não integra produção, não extrai grafos e não executa Doxygen. O catálogo é
fornecido explicitamente; não pressupomos que os provedores já o exportem.

## Executar e encerrar

Da raiz do repositório, com Rust e dependências previamente disponíveis:

```sh
cargo test --offline --locked --manifest-path lab/doxygen-explorador/Cargo.toml
cargo build --offline --locked --manifest-path lab/doxygen-explorador/Cargo.toml
lab/doxygen-explorador/target/debug/doxygen-explorador \
  --inventory lab/doxygen-explorador/fixtures/inventory.json \
  --catalog lab/doxygen-explorador/fixtures/catalog.json \
  --matches lab/doxygen-explorador/fixtures/matches.json \
  --port 8897
```

Abra `http://127.0.0.1:8897`. Encerre com **Ctrl-C no terminal do servidor**.
Fechar a aba não encerra o processo. A porta é obrigatória; somente IPv4 loopback
é aceito. Não há instalação automática nem fallback online. Erros de argumentos,
leitura, contratos, digests ou bind retornam código 2 antes de servir dados.
`--help` retorna 0. O encerramento por sinal segue os códigos do sistema.

Troque os três caminhos para inspecionar seus próprios arquivos v1. Os arquivos
são lidos apenas ao iniciar, nunca modificados. Reinicie após mudá-los.
URLs `/#node=20` permitem abrir diretamente um nó. IDs não são estáveis entre
snapshots; use sempre o catálogo correspondente. Busca por nome, arquivo ou ID;
Tab percorre controles, ↑/↓ e Home/End movem o foco na lista, Enter/Espaço
selecionam. Uma busca não altera a seleção já aberta.

## Intercâmbio e limites

Veja [CONTRATO.md](CONTRATO.md) para invariantes e política HTTP/HTTPS. SHA-256
compara bytes exatos do inventário e catálogo com os declarados nas
correspondências. Alterar espaços ou quebras de linha exige regenerar os hashes.
A API só apresenta documentação confirmada após a validação completa.
Ambiguidade mostra candidatos, sem atribuir sua documentação ao nó estrutural.
O texto original, inclusive tags HTML, permanece texto literal.

`graph_digest` tem formato validado, mas seu vínculo com o grafo **não é
verificado**: o grafo não faz parte destas três entradas. O valor zero na fixture
é uma declaração demonstrativa, não o hash de um arquivo de grafo real.
Nenhum digest autentica a origem. Nomes, caminhos e assinaturas não são
normalizados e nenhuma correspondência é recalculada pelo explorador.

Cobertura completa significa apenas o conjunto declarado pelo produtor, nunca
toda a semântica do programa. Associação documental não comprova correção,
equivalência comportamental nem completude do extrator. Versões/configuração
ausentes são apresentadas como não informadas. Relações documentais têm seus
endpoints validados, mas não são convertidas em dependências ou desenhadas.

Assets estão embutidos no binário, sem CDN. As quatro rotas exatas são `/`,
`/app.js`, `/style.css`, `/api/data`; outras retornam 404, métodos diferentes
de GET retornam 405. Não há rotas para fontes, entradas ou paths documentais.
Apenas URLs HTTP(S) que satisfaçam a política conservadora viram links, abertos
por clique em outra aba. Não há busca, pré-carregamento ou leitura automática
dessas URLs. Arquivos, JavaScript, credenciais na URL, IPv6 e hosts Unicode não
são habilitados como links nesta versão.

Servidor sequencial local, sem autenticação, limite de cabeçalho de 8 KiB e
read/write timeout de 2 segundos. Não há limite de tamanho para os arquivos
carregados nem virtualização da lista: este protótipo destina-se a entradas
pequenas e confiáveis e não foi medido em projetos grandes. Todos os processos
locais com acesso à porta podem ler os dados. Não publique a porta por proxy.

## Verificações realizadas

Em 2026-09-26, Linux/Rust 1.92.0: `cargo test --offline --locked` executou
**13 testes, todos aprovados**, incluindo HTTP TCP real em loopback, schemas,
versões, digests exatos, IDs/endpoints inválidos, estados matched/ambiguous/
unmatched, ausência de correspondência, documentação vazia, Unicode, política
de links, determinismo, catálogo vazio e IDs superiores à precisão JavaScript.
Build offline e Clippy (`--all-targets -- -D warnings`) também passaram.
Smoke da CLI rejeitou porta ausente, porta zero e digest inválido (código 2);
os hashes das fixtures permaneceram iguais. O servidor de inspeção foi encerrado. Fixtures originais fornecem os três estados no
mesmo conjunto; testes mutam cópias em memória para os casos inválidos.

Inspeção realizada no navegador integrado: screenshot do layout desktop, nó 10
confirmado com Unicode e markup literal; clique no nó 20 ambíguo sem documento
confirmado; ↓ + Enter selecionando nó 30 sem associação; nó 40 com texto vazio
e URL JavaScript não clicável; busca sem resultados. Cobertura parcial,
proveniência e diagnósticos foram conferidos na árvore de acessibilidade.
Não foram validados visualmente mobile, outros navegadores ou leitores de tela.
Os testes Rust não executam o JavaScript; essa parte foi inspecionada no navegador.

Roteiro manual reproduzível: iniciar com as fixtures; selecionar 10, 20, 30 e
40; conferir cabeçalho/status e conteúdo; usar as setas e Enter na lista; buscar
`inexistente`; limpar a busca; abrir a seção de proveniência. Não clique em links
externos para testar o painel: nenhuma navegação externa é necessária.

## Proposta futura para a DSM

Adicionar um painel opcional acionado pela seleção de node_id da DSM existente.
Antes de habilitá-lo, verificar o graph_digest contra o snapshot efetivamente
exibido, e os digests do inventário/catálogo contra os bytes recebidos. Conservar
estado, evidências, cobertura e diagnósticos neste painel; mostrar candidatos
separadamente. Não alterar arestas estruturais com relações do Doxygen nem
unir linguagens. O catálogo e a política de geração de correspondências precisam
ser configurados explicitamente. Esta proposta não está implementada fora deste
workspace e não depende do código das outras tarefas Doxygen.
