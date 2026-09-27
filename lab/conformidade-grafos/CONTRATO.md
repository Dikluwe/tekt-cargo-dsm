# Contrato do harness de conformidade

Arena isolada, sem alterações no workspace principal. Reutilizar
`lente_infra::desserializar_grafo`, `lente_core::calcular_raio` (módulo domain/raio),
`lente_estrutura::detectar_ciclos` e `lente_ranking::rankear` pelas APIs locais.
Não implementar parsing alternativo do grafo nem copiar algoritmos.

O modo padrão verifica a forma crua aceita pelo adaptador atual. Paths repetidos
são informados, mas não invalidam a forma crua. O modo analítico exige paths
únicos; isso é uma precondição para estes cálculos, não certificação de que toda
a cascata de resolução foi aplicada. Não resolver colisões automaticamente.

Comparação exige exatamente dois grafos analiticamente admissíveis e mapa JSON
`[[id_esquerda,id_direita], ...]` total e bijetivo. Normalizar resultados pelos IDs
da esquerda, sem inferir correspondência por nome/ordem. Conferir multiconjunto
de arestas dirigidas, relação/subtipo, graus, classificações, profundidades
completas por destino, contenção Owns, SCCs e pontuações de ranking. Desempates
lexicais de ranking não são invariantes sob renomeação e não são comparados.
Não comparar campos semânticos específicos de linguagem como se fossem topologia.

Saída determinística e legível; status 0 se todas as verificações solicitadas
passarem, 1 se qualquer leitura/validação/análise/comparação falhar, 2 para uso
inválido. Vários arquivos são todos examinados mesmo se um falhar. Nenhum
extrator é invocado; nenhuma dependência é instalada durante a análise.
Aprovação não comprova completude semântica, equivalência comportamental,
estabilidade entre extrações ou fidelidade do extrator ao código-fonte.

## Propriedades geradas (2026-09-26)

O módulo **somente de testes** `src/propriedades.rs` acrescenta um oráculo
independente de alcançabilidade/distância para grafos pequenos: matriz e
Floyd–Warshall, sem BFS nem algoritmo copiado da produção. A proibição de
reimplementar algoritmos no harness de uso permanece; esta exceção é restrita
ao resultado esperado nos testes. Resultados reais continuam vindo das APIs
públicas existentes de raio, SCC e ranking, normalizados por `assinatura`.
Nenhuma dependência da nova explicação de impacto.

Gerador SplitMix64 com aritmética u64 wrapping, sementes registradas em
`sementes.json`. Até 8 nós na origem, 9 após acrescentar isolado. IDs e paths
únicos, endpoints coerentes, `kind=mod`; Uses pode ter ciclos, autoarestas e
componentes desconectados. Owns é floresta acíclica com um pai no máximo.
Graus contam vizinhos distintos, incluindo auto-vizinhança; alcance exclui o
próprio alvo mesmo em ciclo/autoaresta. SCCs de tamanho 1 não são reportados,
conforme contrato atual de detectar_ciclos.

Cada propriedade verifica origem e transformação contra o oráculo e compara:
1. Permutação de nós/arestas: assinatura semântica inteira.
2. Renomeação bijetiva: mesma assinatura após mapa inverso de IDs/paths;
   topologia inclui relação/subtipo/multiplicidade, não posições de ranking.
3. Duplicação: mesmas métricas e vizinhos, SCCs e pontuações; multiplicidade
   das arestas não é invariante e não é comparada nessa propriedade.
4. Isolado: mesmos raios, SCCs e pontuações dos nós anteriores; não se exige
   mesmo ranking posicional nem número total de candidatos.
5. Adição de Uses: alcances anteriores são subconjuntos dos novos.
6. Adição de Uses: distâncias anteriores permanecem definidas e não aumentam.
7. Remoção de Uses: alcances novos são subconjuntos dos anteriores.
8. Inversão de Uses: troca montante/jusante e graus/profundidades respectivos;
   Owns permanece inalterado.
9. Adição de Owns válida: mesmas métricas de Uses, ciclos e pontuações;
   contexto pai/filhos pode mudar e não é comparado.

Comparações usam mapas/conjuntos ordenados, não ordem de HashMap. Casos sem
nós/arestas são válidos; testes de cobertura asseguram que o corpus padrão
não se limita a transformações vazias. Não há limiares de tempo.

Uma falha informa semente, propriedade, origem JSON, transformação JSON,
mapa inverso (quando aplicável), esperado/obtido e comando de repetição com
`LENTE_SEMENTE=<u64 decimal>`. O algoritmo de geração é versionado como v1;
repetir exige a mesma versão do gerador e dos crates sob teste. A CLI do
harness existente não muda. Não se infere completude de extratores.
