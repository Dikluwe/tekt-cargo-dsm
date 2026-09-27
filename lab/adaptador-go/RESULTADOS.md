# Validação — 2026-09-26

Ambiente: Linux amd64, Go 1.26.5, Rust 1.92.0. Mudanças deste trabalho
restritas a lab/adaptador-go; dependências da Lente reutilizadas por caminho.

- `cargo test --offline --manifest-path lab/adaptador-go/Cargo.toml`: 12 testes
  passaram, nenhum ignorado. Incluem transitividade calculada pelo núcleo real,
  variantes de teste, cache local de módulo, replace, workspace, plataforma,
  tags, cgo excluído, erro parcial e toolchain futuro bloqueado.
- `cargo clippy --offline --manifest-path lab/adaptador-go/Cargo.toml --all-targets -- -D warnings`:
  passou sem avisos.
- `cargo fmt --manifest-path lab/adaptador-go/Cargo.toml --check`: passou.
- CLI real: cadeia → código 0, 4 nós/2 arestas; testes incluídos → código 0,
  127 nós/697 arestas; dependência ausente → código 2, 3 nós/1 aresta;
  módulo externo já disponível → código 0, 2 nós/1 aresta;
  configuração ausente → código 1. Os dois JSONs foram lidos nas quatro
  execuções que produzem arquivos. Contagens da stdlib dependem do Go instalado.

A fixture cadeia confirma a → b e b → c na extração e distância 2 até a no
raio de impacto de c. Não há aresta direta a → c. A fixture isolada não entra
nesse raio. Saídas repetidas de módulo simples e variantes de teste foram
comparadas integralmente, incluindo metadados.

A primeira verificação de variantes revelou que o driver sintético pode ter
nome de arquivo baseado em hash no cache Go. A identificação foi corrigida
para combinar a identidade .test, o pacote original e fonte gerada absoluta.
Não depende do basename _testmain.go.

Limites: somente imports entre pacotes, cgo desligado, módulos readonly,
nenhuma afirmação de completude semântica ou validade de corpos/tipos. Caches
internos do Go não são cache de grafos. Os arquivos de saída devem ser consumidos
juntos e somente após verificar o código de retorno. Próximos passos e comandos
reproduzíveis estão no README. Não foi realizado benchmark de desempenho.
