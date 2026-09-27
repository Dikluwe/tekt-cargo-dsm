# Contrato local — prioridade documental v1

Registrado antes da implementação em 2026-09-26. Escopo exclusivo:
`lab/doxygen-prioridade/`. Fontes causais: AGENTS.md vigente, spec da forma
organizada, ADR-0007, ADR-0008, docs/adaptadores.md, docs/analises-artefatos.md
e contrato de intercâmbio v1 do pedido. Workspace Cargo independente; nenhuma
integração, importação de outras tarefas, alteração de produção ou regra de CI.

## Entradas e vínculos

CLI `lente-doxygen-prioridade --inventory I --catalog C --matches M --impact P
--output R [--summary]`. Todos são arquivos JSON locais. --summary imprime
resumo textual em stdout, além de R; sem essa flag stdout permanece vazio.
Não executa Doxygen, provedores, código analisado ou downloads. Caminhos CLI são
resolvidos pelo sistema operacional a partir do cwd. Caminhos/nomes/assinaturas
contidos nos artefatos são preservados literalmente, sem normalização.

Aceita somente schemas/version 1 do pedido: lente.doxygen.inventory,
lente.symbol_catalog, lente.doxygen.matches e lente.impact_summary. SHA-256 é
calculado sobre bytes exatos lidos uma vez; compara inventory_digest/catalog_digest
das correspondências e graph_digest entre catálogo/impacto. Formato de digest:
sha256: seguido de 64 dígitos hexadecimais minúsculos. Também registra digests
exatos de matches e impact no relatório. Não recebe nem verifica bytes do grafo:
a igualdade de graph_digest é vínculo declarado, não prova da métrica.

Valida IDs únicos de símbolos documentais, catálogo, entradas matches, métricas,
candidates e listas unmatched; referências de relações, diagnósticos symbol_id,
document_id/candidates e métricas precisam existir. Exige uma entrada matches
por nó do catálogo. matched exige document_id em candidates; outros estados
exigem document_id ausente/null. ambiguous exige candidato(s), sem impor mínimo
de dois: um documento pode ser ambíguo entre vários nós. Não presume associação
bijetiva. Listas unmatched são preservadas, verificadas por referência e não
podem contradizer associações matched; não se impõe exaustividade não definida
no intercâmbio. O status por nó é a fonte da classificação documental.

Campos opcionais ausentes continuam null (nome qualificado, assinatura,
localização, URL, versões/digest de configuração); brief/details são strings
obrigatórias, inclusive vazias. Localização, quando fornecida, exige file não
vazio e line >=1. Extensões JSON desconhecidas são recusadas para evitar perda
silenciosa de metadados; diagnostics é extensão opcional explícita no catálogo e
impacto. Schemas, versões, enums e tipos incompatíveis são erros com contexto.

## Relatório e ordenação

Schema `lente.doxygen.priority_report`, version 1. Um item por nó do catálogo:
identidade e dados estruturais declarados, transitive_consumers informado ou
null, metric_status (provided/missing), impact_coverage, documentary_state,
associação original (status/document_id/candidates/evidence), documento quando
matched, rank e position_reason. Os quatro estados documentais são:

- documentation_observed: matched com brief ou details contendo algum caractere
  não branco. Texto é mantido intacto; não é avaliação de qualidade.
- associated_without_text: matched com brief/details vazios ou somente whitespace;
  significa ausência de texto no inventário recebido, não ausência no projeto.
- ambiguous_association: status ambiguous; não escolhe documento nem usa seu texto.
- unassociated: status unmatched; não significa "sem documentação".

items ordena métricas conhecidas por transitive_consumers decrescente, com
node_id crescente no empate. Todos os nós com métrica participam, independente
de documentação. rank é ordinal 1-based nesse conjunto; a justificativa declara
critério, valor e desempate. Métrica ausente (item ausente, campo omitido/null)
gera diagnóstico missing_impact_metric, valor/rank null e vai ao final por ID,
sem fingir prioridade baixa ou zero. Zero explicitamente informado é ranqueado.

coverage.inventory e coverage.impact permanecem independentes e literais:
complete, partial ou unknown. Cada item repete a cobertura de impacto. Complete
significa somente o conjunto de entrada declarado. Nenhuma execução promove
coverage. Métricas parciais/unknown são sempre contagens informadas sob essa
cobertura, não valores completos. Há diagnósticos report próprios para cobertura
não completa e métricas ausentes; diagnósticos das entradas ficam separados por
origem e preservados (inclusive repetições), ordenados por code/message/symbol_id.
Relations e listas de IDs/evidências são ordenadas, sem converter relações
Doxygen em dependências estruturais. Os textos não são executados ou renderizados
como HTML. O resumo textual escapa controles/textos com representação JSON.

## Erros, publicação e limites

Erro de argumento, leitura, JSON, vínculo ou referência impede relatório; exit 1,
diagnóstico em stderr. Sucesso, inclusive parcial ou com métrica ausente, exit 0.
Valida e serializa tudo antes de abrir a saída. Publicação create_new recusa saída
existente (inclusive symlink), protegendo entradas e resultados anteriores;
falha normal de escrita remove somente o arquivo criado nesta chamada. Sem
promessa de publicação atômica ou durabilidade em queda de energia. Artefatos
carregados são um snapshot em memória por arquivo, não transação entre arquivos.

Sem recomputar impacto, associação, qualidade de texto, correção de código ou
qualidade arquitetural. Sem score misturando confiança/texto/impacto. Não bloqueia
CI nem modifica regras existentes. Digests não autenticam produtores. IDs são
locais e não demonstram identidade entre versões. Fontes não são abertas. Dados
cabem em memória; não há limite/streaming nesta versão.

## Verificação planejada

Fixtures JSON pequenas de alto impacto com texto vazio, sem associação, ambíguo,
baixo impacto documentado, empate e métrica ausente, coverage parcial. Testes
Rust inline: estados/ranking, zero versus ausência, digests dos bytes exatos,
links divergentes, schemas/versões inválidos, referências e IDs repetidos,
campos opcionais, preservação de diagnósticos/coverage, determinismo, CLI e
recusa de sobrescrita. Build/test/clippy offline com lockfile local. Resultados
executados são registrados no README/RESULTADOS.md, sem afirmar testes de produção.

## Revisão de integração (ADR-0009, prevalece sobre v1 acima)

Inventário v2 exige pending_relations [{from,to,kind,reason}], com reason
missing_from/missing_to/missing_both verificado. relations só contém endpoints
resolvidos. Pendências exigem partial; nunca geram identidade ou dependência.
Consumidores aceitam v1 resolvido; v1 pendente exige migração explícita, sem
alterar bytes para calcular digests. IDs duplicados são erro. Diagnósticos de
IDs pendentes são contexto, não símbolos fictícios. Catálogo aceita diagnostics
opcional. Unmatched pode ter candidatos para inspeção, sem documento confirmado.
Prioridade emite priority_report v2, preservando pending_relations; explorador
expõe ambas as listas e modo finito --output, sem abrir servidor. Publicações
em arquivo usam temporário irmão + hard link exclusivo (Linux), sem sobrescrita
ou promessa de durabilidade do nome após queda de energia.
