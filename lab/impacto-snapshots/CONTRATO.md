# Impacto entre snapshots — contrato experimental v1

Origem: pedido de 2026-09-26; AGENTS.md, spec forma-organizada, ADR-0002,
contratos raio/comparacao/mapeamento/infra-mapa-correspondencia.
Resultados prévios consultados (tarefas 17 e 18):
- `00_nucleo/lessons/2026-09-26-identidades-opacas-comparacao-01a0e01d45e4.md`:
  pares explícitos 1:1, sem heurística de path/ID; 23 testes e doctest passaram.
- `00_nucleo/lessons/2026-09-26-mapeamento-inventario-01a0e01d-6243.md`:
  cobertura explícita e validação integral; remoções fora do escopo; 17 testes de
  mapeamento/79 core passaram. Falha concorrente de wiring/cache registrada no
  laudo não é corrigida nem incorporada a este experimento puro.

## Entrada e pureza

Biblioteca somente stdlib + lente_core; zero I/O, serde, Git, processos, leitores
ou dependências de infraestrutura. Função comparar_impacto recebe referências a
Snapshot antes/depois, lista de Correspondencia e lista explícita de Mudanca.
Snapshot contém Grafo, versão declarada não vazia, Completude (Completa/Parcial/
Desconhecida), diagnósticos, granularidade/natureza explícitas e compatíveis.
Versão é um rótulo do chamador, não SHA/revisão verificada. Lado + versão acompanham
cada elemento retornado, inclusive quando as duas versões têm o mesmo rótulo.

Correspondencia {identidade opaca, antes:id_local, depois:id_local} é 1:1. Strings
opacas, inclusive vazias, são válidas (decisão da tarefa 17); não normalizar nomes,
Unicode, prefixos ou paths. Validar duplicata exata, identidade reutilizada,
endpoint reutilizado e referência ausente em TODO o mapa, antes de calcular.
Duplicatas exatas de correspondência são erro, não regra de precedência.

Mudanca = Remocao {antes}, Adicao {depois}, Pareada {identidade,
alteracao_declarada, renomeacao_declarada}. Pareada exige correspondência e pelo
menos uma das duas declarações. Renomeação não é inferida do path; pode coexistir
com alteração e nunca é tratada como evidência de mudança nas relações.
Remoção/adição não pode referenciar endpoint já pareado; declarar Pareada nesse
caso. Ausência de mapa não é declaração de remoção/adição: mudanças são fornecidas.
Mudanças idênticas são deduplicadas; declarações diferentes para o mesmo par são
erro. Listas vazias de mapas ou mudanças são válidas e não inferem nada.

Validar IDs únicos, endpoints existentes e paths textuais dos endpoints coerentes
com os IDs. Paths de nós podem colidir: projetar cada nó para um path temporário
único derivado do ID SOMENTE dentro daquele snapshot, preservando as arestas.
Esta projeção satisfaz a pré-condição do raio por path sem parear IDs entre lados.
A projeção não é exposta, não resolve nomes semanticamente e não altera os grafos.
Owns é contexto e não propaga impacto; não se emite seu pai arbitrário na saída.

## Cálculo e comparação

Reutilizar calcular_raio, sempre separadamente:
- Remocao: raio antes; depois=None, não um raio vazio.
- Adicao: antes=None; raio depois.
- Pareada: ambos os raios.

ImpactoSnapshot preserva alvo, classificação, graus diretos, consumidores
(montante) e dependências (jusante) com menores profundidades e origem explícita.
Chave de comparação de consumidor pareado = identidade opaca; sem par =
(lado,id_local). Igualdade de IDs ou paths em lados diferentes nunca pareia nós.
Resultado contém sem_par_antes/depois abrangendo TODOS os nós sem correspondência.

Conjuntos de consumidores: somente_antes, ambos, somente_depois. Essas categorias
são observações nos raios, NÃO existência/inexistência de elementos nos snapshots.
Cada unilateral conserva contraparte quando há par e EstadoOutroLado:
SemCorrespondencia, AlvoNaoComparavel, NaoAlcancadoNoGrafoCompleto ou
NaoObservadoComCoberturaLimitada. Comparação bilateral=false em adição/remoção;
não concluir que seus consumidores deixaram de depender/apareceram no outro lado.
Sem par continua sem par mesmo que exista nó de mesmo ID/path na outra versão.
Snapshots parciais/desconhecidos preservam evidência positiva, mas não permitem
inferir ausência de dependência pelo silêncio do grafo. Diagnósticos são mantidos.

Para cada mudança pareada, comparar também relações Uses DIRETAS incidentes ao
alvo (entrada/saída), pela identidade dos dois endpoints e subtipo. Paralelas
idênticas são deduplicadas; import/reference/desconhecido permanecem distintos.
Não combinar grafos para calcular raio. TopologiaIncidente separa comuns e
observações unilaterais, com status MesmasRelacoesObservadas,
DiferencasNasRelacoesObservadas ou ComparacaoLimitada (snapshot não completo ou
endpoint sem par). Não é comparação da topologia global; mudanças transitivas
podem alterar consumidores sem alterar relações incidentes. Adição/remoção têm
topologia_incidente=None. Status nunca certifica preservação comportamental.

## Determinismo e erros

Tipos ordenáveis usam discriminantes explícitos/Ord e campos, não strings Debug.
Saídas, diagnósticos originais e erros ordenados/deduplicados. Mudar ordem de nós,
arestas, correspondências, mudanças ou diagnósticos não muda o resultado. IDs são
exibidos como proveniência local; renumerá-los pode mudar a representação, nunca
transformá-los em identidade entre extrações. Validação é integral e retorna
Vec<Erro> sem resultado parcial se há entrada inválida. Nenhuma escolha first-wins.

## Limites

Declarações de identidade, mudanças, versão, completude e escopo são do chamador.
Não descobrir mudanças, renomes, splits/merges ou cobertura de fontes. Não usar
mapear_diff automaticamente: este componente não lê arquivos ou posições. Sem
integração com a API de comparação de módulos, Git, CLI, MCP ou renderizadores.
Raio considera todos os subtipos Uses, inclusive imports, no escopo fornecido.
Sem prova de quebra nem de preservação comportamental. Sem execução real.
Construção de índices do raio por mudança; sem cache persistente ou otimização
multi-alvo. Exemplo executável constrói dados em memória e apenas imprime resultado.
