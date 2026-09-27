# Regras arquiteturais experimentais — contrato v1

Linhagem: pedido de 2026-09-26, AGENTS.md, spec forma-organizada,
contratos de extração e comparação, entregas 13 e 17 conferidas no checkout.
13 = lab/pacote-analise/CONTRATO.md: integridade não é completude/autenticidade;
metadados desconhecidos não viram completos e IDs de execução não são inferidos.
17 = comparar_estruturas_por_identidade e prompts/comparacao.md: identidade opaca,
sem normalização Rust, sem pareamento de candidatos ambíguos. Essa API compara
estruturas agregadas, não implementa políticas; aqui usam-se mapas explícitos por
ID local para não perder a identidade dos nós crus com paths repetidos.

Somente esta pasta muda; workspace próprio, biblioteca pura (sem I/O/serde no
módulo de domínio), executor de arquivos e serialização separados. Reutilizar
explicar_caminho_impacto e detectar_ciclos, sem copiar algoritmos. Sem extração,
subprocessos, CI, integração CLI/MCP ou regras internas de crystalline-lint.

## Snapshots e qualidade

Documento local version=1: graph (relativo ao documento), graph_sha256 obrigatório
(bytes exatos), context {project,provider,provider_version,granularity,nature,
configuration,scope}, quality {execution,coverage,capabilities,diagnostics},
identities [{node_id,identity}], source_metadata opcional. Identidades canônicas
não vazias, opacas e injetivas; mapa TOTAL sobre IDs de cada grafo. Mesmo texto de
identidade nos dois mapas é correspondência explícita. Só de um lado significa
não pareado/adicionado/removido na amostra; nunca há matching por path/nome.

Execution/coverage: complete/partial/unknown. Ausência = unknown. Capabilities:
direct_dependencies, reachability, cyclic_components; ausência = false. São
alegações do produtor/chamador para o escopo/natureza declarados, não autenticação.
Digest fixa conteúdo, não prova verdade das alegações. Natureza, granularidade,
projeto, provedor/versão, configuração e escopo devem coincidir e ser conhecidos;
unknown ou divergência => inconclusivo. Granularidades: item/file/module/package.

source_metadata, quando presente, é preservado no relatório; graph_sha256 nativo
quando presente é conferido. Estados parciais/erro/desconhecido em campos conhecidos
(execution/status, status, loading, coverage, semantic_coverage, completeness/*,
semantic_completeness) BLOQUEIAM aprovação, mesmo se quality declarar complete.
Desconhecido/não avaliado/não alegado também bloqueiam; política conservadora.
Não se deriva completude positiva nem capacidade de metadados nativos. Sem esse
arquivo, quality é a declaração local explícita, não uma certificação do extrator.
Pacotes da entrega 13 podem fornecer seus graph/metadata, após verificação própria;
o executor NÃO substitui verify nem interpreta automaticamente manifestos.
Provedor/schema nativo desconhecido bloqueia conclusão, conforme a entrega 13.

## Regras, seleção e exceções

Regras version=1; enforcement obrigatório all ou new_only; rules e exceptions.
IDs de regra não vazios, únicos e fornecidos pelo usuário. Mesma política atual
é avaliada nos dois snapshots; não há histórico de revisões das próprias regras.

Seletores: {ids:[identidades],prefixes:[prefixos]}; união dos termos. Prefixo é
starts_with literal, case-sensitive, sem glob/regex, sem normalização Unicode,
sem interpretar barras/pontos/::. Nenhuma camada deriva do diretório. Seletor
vazio, termo vazio ou IDs repetidos são erro; termo sem correspondência NO ATUAL
=> regra inconclusiva, mesmo se outros termos encontrarem nós. Baseline sem um
endpoint atual não é seletor vazio: com mapa total e dados completos, é ausência
na amostra anterior. Caminhos podem passar por nós fora dos conjuntos selecionados.

- forbid_direct: from × to, somente aresta Uses direta, inclusive self-loop.
- forbid_reachability: from × to, caminho Uses de comprimento >=1; também
  inclui arestas diretas. Relata witness e hops, rotula direct/transitive.
  Para mesma ponta exige percurso fechado positivo; zero saltos não viola.
- no_new_cycles: scope seleciona SCCs que INTERSECTAM o conjunto (SCC inteiro,
  não subgrafo induzido). Cyclic SCC = tamanho >=2 ou singleton com self-loop.
  Preservar todos os membros e arestas internas; não enumerar ciclos simples.

## Ciclo novo e baseline

Uma observação cíclica atual é PREEXISTENTE se seus membros forem subconjunto
de UM SCC cíclico da baseline E TODAS as suas arestas internas já existirem
na baseline. Caso contrário é NOVA. Isso detecta mesmo número com componentes
diferentes, fusão/expansão e nova aresta interna mesmo com membros idênticos.
Remover arestas, encolher ou dividir SCC antigo, sem adicionar arestas cíclicas,
não é regressão. Todos os self-loops internos contam, mesmo em SCC de vários nós.
Mudanças de subtipo Uses não contam: natureza é declarada para TODAS as Uses.
Owns nunca participa. O relatório lista added_internal_edges; quando houver
aresta nova, sua testemunha fechada contém a primeira aresta nova na ordem
lexicográfica e um caminho mínimo de retorno. O relatório diz cyclic-component-regression, não inventário
de todos os ciclos simples. no_new_cycles só impõe observações NOVAS, mesmo em all.

Direta/transitiva: violação antiga se o MESMO par canônico satisfazia a MESMA
regra na baseline. Mudança da testemunha não cria nova violação transitiva.
IDs de violação v1 usam codificação de strings com comprimento UTF-8: regra,
tipo, par (direta/transitiva), ou membros + arestas internas do SCC. Sem hash
aleatório, timestamps, IDs locais ou paths. SCC modificado pode ter novo ID mesmo
se classificado preexistente pela relação de subconjunto; identidade não é lineage.

Exceção: id único, rule_id existente, justification não branca; until opcional
YYYY-MM-DD inclusive. as_of obrigatório passado à função/CLI, nunca relógio
implícito. Escopo pairs {from,to} para proibições; component {members} para SCC:
TODOS os membros devem pertencer ao seletor. Escopo sem match não se aplica;
ID/justificativa/data inválidos são erro. Exceções expiradas e não usadas ficam
separadas; todas as exceções válidas coincidentes são listadas. Não escondem
inconclusividade nem transformam regra excetuada em regra aprovada.

## Resultados

Regras: approved/preexisting/excepted/violation/inconclusive. Achados preservam
old/new, witness consumidor→dependência, proveniência original e exceções aplicadas.
all cobra proibições antigas e novas; new_only cobra somente novas; dívidas antigas
continuam explícitas. Exceções aplicadas aparecem em lista separada das aprovações.
Qualidade insuficiente em qualquer snapshot ou capacidade requerida ausente =>
inconclusivo; não se tenta concluir ausência de violações. Inconclusivo tem
precedência global sobre violação, mas outras violações conclusivas são preservadas.

Exit: 0 aprovado para a política/enforcement; 1 violação não excetuada;
2 inconclusivo; 3 erro de entrada/I/O/serialização. Erro de entrada não é violação.
Nenhum resultado comprova correção funcional, completude universal do extrator,
ativação runtime ou equivalência comportamental.
