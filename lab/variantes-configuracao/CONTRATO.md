# Contrato local v1 — variantes de configuração

Linhagem: pedido de 2026-09-26, AGENTS.md, spec forma-organizada, ADR-0002 e
contratos grafo/infra/estrutura. Arena independente; nenhuma API de comparação
em desenvolvimento é utilizada. Reutilização: desserializar_grafo, detectar_ciclos,
ordenar_dsm. Não executar builds, extratores, shell, configuração ou código fonte.

## Entrada

Documento JSON version=1, project (identificador explícito do projeto),
configurations (lista não vazia), paths (opcional). Cada configuração tem:
- id: identificador não vazio, atribuído pelo usuário; não é inferido de paths.
- graph: arquivo JSON existente relativo ao documento de entrada, ou absoluto.
- granularity: file/module/package/item; todos precisam concordar.
- nature: identificador explícito da semântica de observação; todos precisam
  concordar. Não vazio, nem unknown. Exemplos: syntactic-import, active-include,
  resolved-reference. O usuário garante que o mesmo nome denota o mesmo contrato.
- extraction: complete/partial/unknown; omitido significa unknown. Completo é
  execução no escopo declarado, nunca completude semântica universal.
- diagnostics: lista opcional de strings com motivos da extração.
- identity_map: lista explícita {node_id, identity}; uma identidade canônica
  não vazia para CADA nó. IDs são locais ao grafo, identidades ao projeto/amostra.
  Um ID não pode ter duas identidades nem dois nós da mesma configuração podem
  compartilhar identidade. IDs estranhos/ausentes são erro. Repetição idêntica é
  idempotente. Nomes e paths não inferem correspondência, mesmo se iguais.

Naturezas/granularidades incompatíveis e identidade ambígua: erro contextual,
sem comparação ou união parcial silenciosa. Separar amostras compatíveis para
analisá-las. Unknown na natureza/granularidade não autoriza comparação.
O tipo de nó pode mudar entre configurações: fica registrado, sem fundir atributos.

Relação canônica = (from identity, to identity, relation, uses_kind nullable).
Preservar owns/uses e distinguir import/reference/unknown. Subtipo desconhecido
fica separado e recebe diagnóstico; não equipará-lo aos conhecidos. Contenção
owns é inventariada, mas não participa de caminhos/ciclos de dependência.

## Saída

Relatório version=1 em JSON ou texto; configuração ordenada, nós e relações
ordenados, sem timestamps. Declaração do projeto preservada, não comprovada.
Repetição de configuração idêntica após normalização é idempotente. Mesmo id com
conteúdo normalizado divergente é erro (não soma como outra observação). A ordem
das entradas, mapas, diagnósticos, nós e arestas não muda o resultado; arestas
repetidas representam uma observação. IDs/paths originais continuam rastreáveis.

Cada relação e nó da união mantém observed_in. observation_class é
all-provided-configurations ou some-provided-configurations, nunca "universal".
Para relações não observadas: separar configurações complete de partial/unknown.
Ambas significam não observação; partial/unknown não prova ausência. Ausência de
nó também não prova inexistência. Relatório sempre traz esse limite e preserva
estado/diagnósticos por configuração. Não existem flags "removed" ou "absent".

Cada configuração recebe análise sobre SUAS relações, antes da união. Estrutura:
contagens, cyclic_sccs, ordem de condensação e exemplos concretos de ciclos.
Usar projeção um-para-um com paths=identidades únicas e IDs lexicográficos para
satisfazer algoritmos existentes, sem agregação nem mudança de granularidade.
Algoritmos estruturais operam somente sobre Uses; atributos/posições não são
combinados. Não exportar essa projeção como um novo grafo extraído.

## União e caminhos

A união é nomeada union-of-possibilities: não é uma configuração executada.
`paths` recebe {id, edges:[{from,to,relation:"uses",uses_kind:null|import|reference}]}.
Arestas devem existir exatamente na união e formar percurso contíguo não vazio.
Percurso fechado é permitido; caminhos não precisam ser simples. Consulta inválida
é erro explícito. Duplicatas idênticas são idempotentes; id divergente é erro.

common_configurations = interseção de observed_in de TODAS as arestas.
Vazia => not-observed-together-in-provided-configurations; não prova caminho
impossível, sobretudo com extrações parciais. Não vazia => observed-together;
só atesta coocorrência no grafo, não ativação runtime de imports condicionais.

SCC não é ciclo testemunhado. Relatar separadamente cyclic_sccs (membros) e
cycle_examples (sequência concreta de arestas contíguas e fechadas).
Escolher um ciclo simples determinístico por SCC >=2 e os self-loops observados;
não enumerar todos os ciclos. Tarjan existente omite singleton: self-loops são
tratados localmente. Calcular interseção para CADA exemplo da união. Não usar
interseção de todas as arestas do SCC nem concluir que todos/nenhum dos seus
ciclos foram observados a partir de um único exemplo. Os exemplos individuais de
cada configuração permanecem disponíveis, mesmo se a amostra da união for vazia.

## Limites e verificação

Sem inferência de configuração real, execução, autenticação, cache ou leitura de
metadados de adaptadores. Declarações de completude/natureza/identidade são do
usuário; não substituem os limites da extração. Memória proporcional aos grafos;
um BFS por SCC para exemplo (sem enumeração exponencial de ciclos). Não há
integração DSM nem dependência funcional da comparação nova. O L3 transitivamente
compila crates locais existentes, mas somente seu desserializador é chamado.

Testes inline: união com ciclo espúrio, ciclo observado, interseção vazia/não vazia,
nó de uma variante, parcial/desconhecido, identidade ambígua, incompatibilidades,
repetição/permutação, SCC com vários ciclos, self-loop, subtipos e formatos de saída.
