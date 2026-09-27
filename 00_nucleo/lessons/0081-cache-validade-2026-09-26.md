# Cache de extração — validade conservadora (2026-09-26)

## Resultado

Corrigida a reutilização indevida descrita nos itens 1 e 6 do laudo 0080.
A correção de descoberta do toolchain por diretório foi integrada primeiro no
checkout compartilhado; a identificação agora inclui `rustc --version --verbose`
e `rustc --print sysroot`, sempre no diretório do membro, com
`RUSTUP_AUTO_INSTALL=0`. Não há mudança do cwd global. Erros de execução, status,
UTF-8 e stdout vazio permanecem diagnósticos contextualizados.

O contrato `00_nucleo/prompts/infra-cache-validade.md` foi escrito antes do código.
`cache.rs` implementa a análise de validade; `workspace.rs` integra extração e
publicação. A fachada exporta `CacheExtracao` e `ValidadeCache`. O wiring mantém
uma instância por rodada. Alterações locais preexistentes e o trabalho concorrente
de identidade pacote/crate foram preservados; não foi criado commit ou merge.

## Entradas e política

- Chave SHA-256 v2, com tags e comprimentos, caminhos em bytes nativos e ordem
  determinística. A ausência de arquivo difere de arquivo vazio. Chaves antigas
  não são reaproveitadas.
- Raiz e membro canônicos, pacote, contexto Rust por membro, ambiente e identidade
  do fork por conteúdo/caminho, sem depender apenas da versão declarada do fork.
- Manifestos e locks, árvore da raiz e árvores dos caminhos locais transitivos,
  inclusive fora do workspace e fontes configuradas fora de `src`. Consideram-se
  também os outros membros declarados e o pacote raiz: o fork carrega o workspace
  e sua resolução pode ser afetada por eles.
- Configurações Cargo ancestrais e do CARGO_HOME, arquivos de seleção Rust e
  variáveis de compilação são observados. Se a configuração global não puder ser
  localizada, não se autoriza reaproveitamento.
- Só `NotFound` de entrada opcional representa ausência. Leituras obrigatórias,
  diretórios usados como arquivos, manifestos inválidos e dependências locais
  inexistentes produzem erro com contexto, sem conteúdo vazio silencioso.

`motivos_bypass` torna a decisão inspecionável. Bypass não lê nem publica cache;
extrai e valida normalmente. É aplicado a dependências registry/git, build
scripts, proc macros, configuração Cargo, wrappers/flags, links simbólicos,
fontes com entradas externas/ambiente ou construção dinâmica de macros,
patch/replace, native links, cargo-features, herança/workspaces externos não
resolvidos e alvos fora da árvore rastreada. Caminhos locais explícitos continuam
rastreados; herança de workspace externo não é interpretada usando a tabela do
workspace consumidor. Isso evita erros espúrios, além de impedir reutilização.

## Subprocessos e publicação

Não foi acrescentado `cargo metadata` ao cálculo da chave nem substituída a
convenção de enumeração. Permanecem a enumeração por TOML e a consulta de alvo
por metadata já existente. A decisão e o custo da alternativa estão no contrato.
O contexto Rust exige duas consultas por membro; identidade do fork usa arquivo,
sem executar um subprocesso extra para `--version`.

JSON é validado pelo adaptador antes de criar/publicar a entrada. Temporários
irmãos usam PID + contador atômico e `create_new`, com nova tentativa em colisão.
A gravação completa/sync precede rename; erro limpa o temporário. Chamadas
concorrentes não compartilham arquivo temporário. Cache corrompido continua
produzindo erro de tradução explícito, sem ser aceito silenciosamente.

## Evidência

- `cargo test --offline --workspace --target-dir /tmp/lente-cache-validation --quiet`:
  **348 passaram, zero falharam, 34 ignorados**.
- Regressões cobrem estabilidade, manifesto raiz/outro membro, dependências locais
  transitivas externas/herdadas, fontes fora de src, mudança de diretório,
  toolchain/fork/ambiente, configuração ancestral/global, compartilhamento de
  leituras/digests, framing com NUL, erro de leitura, symlink circular e bypass.
- Publicação: JSON inválido não cria entrada; bypass não lê/substitui entrada;
  temporários distintos simultâneos, leitor durante oito writers sem conteúdo
  parcial, limpeza após falha de rename.
- Testes de toolchain preservam diretório/contexto e diagnósticos, sem instalação
  de múltiplos toolchains. Testes de fiação usam fronteira de extração controlada
  para não exigir fork instalado ou ausência de configuração Cargo no host.
- `crystalline-lint --checks v2,v3,v4,v9,v10,v13,v14 .`: sem violações.
- V5/V6: sem achados nos quatro arquivos de código desta tarefa. Persistem
  achados fora desse escopo. Hashes e snapshots atualizados pontualmente.
- `git diff --check`: sem erros. Não foram executados os testes E2E ignorados que
  invocam o fork nem um benchmark de extração real. A suíte usa artefatos isolados
  em `/tmp`, evitando interferência com outras tarefas no checkout.

## Desempenho esperado e limites remanescentes

O custo de leitura/hash passa a ser proporcional aos bytes únicos rastreados por
rodada, incluindo árvore inteira da raiz, dependências externas locais e binário
do fork. Esses bytes, seus digests, árvores e parsing de manifestos são
compartilhados entre membros. A memória também cresce com os bytes retidos;
percursos/combinação de digests por membro continuam existindo. Não há memoização
persistente de filesystem que possa ignorar edições na rodada seguinte.

A política é deliberadamente mais restritiva. **Este próprio workspace contém
dependências registry e portanto faz bypass para todos os membros**: rodadas
repetidas voltam a pagar extração. O limiar antigo de cache quente não é uma
promessa nesse domínio. Workspaces exclusivamente locais e sem entradas dinâmicas
podem reutilizar o cache. A análise textual também considera comentários/strings;
pode causar bypass desnecessário. Arquivos irrelevantes na raiz e variáveis de
ambiente sem influência podem invalidar chaves desnecessariamente. São custos
aceitos por correção, não ganhos de desempenho medidos.

A rodada exige entradas estáveis; não oferece snapshot transacional contra
edições externas concorrentes. Instalações de toolchain/sysroot são consideradas
íntegras e imutáveis para a identificação detalhada e caminho consultados:
modificação manual de suas bibliotecas/fontes sem mudar a identificação não é
rastreada. O fingerprint do fork cobre o executável, não uma instalação arbitrária
mutável de suas bibliotecas de runtime. Não se declara cache universal/completo.
A enumeração de membros conserva o subconjunto histórico de glob; padrões fora
dele fazem bypass, mas a enumeração em si não foi redesenhada nesta tarefa.
