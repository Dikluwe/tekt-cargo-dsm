# Contrato L0 — validade do cache de extração (2026-09-26)
Hash do Código: d0188c94

Unidade: `03_infra/src/cache.rs` (L3), integrada por `workspace.rs`.
Precede a implementação. Complementa `infra-workspace.md`, a spec da forma
organizada e ADRs 0001/0002/0003. A descoberta de toolchain por diretório deve
ser integrada antes da alteração da extração cacheada.

## Decisão de subprocessos

Preservar a enumeração por TOML e a detecção de alvo por `metadata` já existente.
Não acrescentar `cargo metadata` à construção de cada chave: resolver todo o
workspace alteraria custo, resolução de rede e a convenção dos prompts 0023/0044.
O fechamento conservador lê caminhos locais nos manifestos, incluindo tabelas
por target, dependências herdadas do workspace e caminhos de patch/replace.
Não substitui a resolução Cargo: dependências registry/git e configurações que
alteram resolução impedem o reaproveitamento, em vez de fingir completude.

## Validade e representação

Uma chave v2 inclui identidade canônica da raiz e do membro, nome do pacote,
manifestos raiz/membro/dependências locais transitivas (inclusive externas),
locks existentes, arquivos das árvores locais fora de `.git` e `target`, contexto
de ambiente, configuração Cargo e seleção do toolchain, identidade do executável
do fork (caminho canônico + conteúdo) e identificação do Rust no diretório do
membro (versão detalhada e caminho do sysroot). A árvore inteira da raiz também
entra no digest, inclusive manifestos de outros membros que possam influenciar
resolução/unificação de features. O fechamento inclui os membros declarados
(e o pacote raiz), porque o fork carrega o workspace; suas dependências também
podem tornar toda a rodada inelegível. Globs fora do subconjunto da enumeração
fazem bypass. Arquivos irrelevantes podem invalidar: custo
conservador aceito. Caminhos são bytes nativos, sem conversão lossy. Componentes têm tag e
comprimento prefixado; coleções têm ordenação determinística. Ausente difere de
arquivo vazio. Versões anteriores nunca são reutilizadas.

Uma rodada compartilha leituras/digests de arquivos e parsing de manifestos e identidade do
fork. Não manter memoização global entre rodadas. O chamador não pode editar os
insumos durante a rodada: não é snapshot transacional do filesystem.

## Domínio conservador de reutilização

Só reutilizar fechamento exclusivamente local, com manifestos compreendidos,
sem build scripts, proc macros, symlinks nas árvores ou fontes que possam incluir
entradas externas/ambiente por macros ou atributos de caminho. A busca por `include`, `env`, `path`,
`macro_rules` e `concat` nos bytes de `.rs` também rejeita comentários/strings;
não é um parser Rust. Targets com outras extensões fazem bypass. Workspaces
ancestrais/herança de dependências externas, patch/replace, native links e
cargo-features instáveis também fazem bypass. Fontes fora de
`src` mas dentro das árvores entram no digest; targets externos à árvore impedem
reutilização. Configuração Cargo presente, wrappers/flags de compilação ou
identidade do fork não verificável também causam bypass. Não interpretar código
Rust para provar ausência de I/O: a detecção deliberadamente sobreaproxima e
pode rejeitar macros inocentes. Bibliotecas de sysroot e instalação de toolchain
são assumidas íntegras/imutáveis para uma identificação Rust; alterações manuais
nessa instalação estão fora do domínio de validade. Registrar essa limitação no
laudo e na documentação pública da API.

Bypass significa não ler NEM publicar cache: extrair e validar normalmente.
Um digest calculável não é sozinho autorização para reutilizar. Falhas de leitura
relevantes são erros contextualizados; só NotFound de arquivo opcional é ausência.

## Publicação

Validar o JSON pelo adaptador da spec antes de criar/publicar entrada. Temporários
irmãos criados exclusivamente (`create_new`), identificados por PID e contador
atômico, com tentativa nova em colisão. Escrever tudo e renomear; limpar temporário
em erro. Nenhum consumidor vê conteúdo parcial. Cache corrompido permanece erro
explícito de tradução. Concorrência com entradas estáveis permite last-write-wins.

## Verificação mínima

Estabilidade; alterações no manifesto raiz, dependência local transitiva externa,
fonte fora de src, contexto Rust/fork e diretório canônico; bypass de entradas
não rastreáveis; erros de leitura; ausência versus vazio; componentes não ambíguos;
JSON inválido não publicado; publicação concorrente sem temporários compartilhados
ou restos. Testar compartilhamento de leituras na rodada. Registrar suíte,
impacto esperado em desempenho e limites, sem alegar benchmark não executado.

## API e custo

`CacheExtracao::default()` inicia uma rodada com ambiente capturado.
`avaliar(membro, raiz, contexto_rust)` retorna `ValidadeCache { chave,
motivos_bypass }`; `extrair` usa essa decisão. A fachada `chave_cache` mantém
compatibilidade como gerador de digest, sem prometer autorização de cache.
`extrair_grafo_cacheado` mantém a conveniência para um membro isolado. O wiring
usa uma única instância por montagem, consultando o Rust para cada membro.

Não há subprocesso Cargo novo para fingerprint. Há duas consultas Rust por
membro (versão detalhada e sysroot), além dos subprocessos preexistentes em miss.
I/O/hash crescem com bytes únicos rastreados por rodada, incluindo a raiz inteira
e o executável do fork. Bytes/digests/árvores e manifestos são memoizados somente
nessa rodada. Memória cresce com esses bytes. Bypass paga extração em toda rodada;
nenhuma promessa universal de cache quente abaixo de dez segundos permanece.

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"ValidadeCache","kind":"struct","members":["chave","motivos_bypass"]},{"name":"CacheExtracao","kind":"struct","members":["arquivos","membros","manifestos","digests","arvores","ambiente","fork"]}],"reexports":[]} -->
