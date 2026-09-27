# Agregação detalhada com evidências — 2026-09-26

Contrato: `00_nucleo/prompts/estrutura.md`.
Código: `01_core/estrutura/src/lib.rs`.

## Entrega e política

API aditiva `agregar_por_modulo_detalhado(&Grafo) -> AgregacaoDetalhada`.
Cada célula identifica módulos por ID/path, conserva o peso e lista índices das
arestas originais em `grafo.edges`. A lista de relações preserva cada Aresta
integralmente, com IDs, paths, relação e subtipo. O índice é endereço no snapshot,
sem promessa de estabilidade entre extrações ou após reordenação. Não há posição
de referência inferida de posição de nó.

Cada registro recebe exatamente um destino: agregado, interno absorvido ou não
agregável com motivo tipado. Owns válido é explicitamente contenção, não peso.
Nós sem módulo ou com contenção inválida são listados mesmo sem Uses. O peso
continua contando registros Uses, incluindo paralelas idênticas e subtipos
diferentes; não conta posições sintáticas ou pares distintos.

A nova API valida toda a ancestralidade Owns, inclusive acima de módulos.
Múltiplos pais distintos são ambiguidade mesmo que convirjam; repetição do mesmo
pai não é. SCCs iterativos detectam ciclos e auto-ciclos. Uma projeção temporária
de Owns permite reutilizar o Tarjan existente sem alterar as evidências.
A primeira obstrução ancestral encontrada bloqueia a contenção; não se escolhe
um pai e não se enumeram todos os caminhos. Problemas locais de ambiguidade e
ciclo podem coexistir. A política é conservadora somente na API nova.

Índices por ID e memoização evitam buscas lineares repetidas por endpoints.
Ordenação usa paths, IDs e discriminantes explícitos dos enums, nunca Debug.
Resultados semânticos são invariantes sob permutação; índices são recalculados
para a ordem do snapshot. IDs únicos são precondição do contrato de Grafo.

## Verificação

- `cargo test --offline -p lente_estrutura --target-dir /tmp/lente-agregacao-detalhada-target`:
  35 testes unitários passaram (25 anteriores, dez novos) e um doctest passou.
- Evidências: múltiplos itens por célula; paralelas idênticas; Reference, Import e
  subtipo ausente; preservação integral de campos e índices; posição de nó não
  transportada como posição de referência.
- Exclusões: internas, Owns, órfão isolado/referenciado, endpoint e ancestral
  ausentes; múltiplos pais convergentes, ciclos, auto-ciclos e descendentes.
- Propriedade de contabilização: índices em relações formam exatamente a faixa
  de entrada, e contribuições agregadas são exatamente as evidências das células.
- Permutação válida/inválida mantém evidências e classificações semanticamente.
- Cadeia com 4097 nós verifica contenção mais próxima e execução sem recursão.
- Regressão verifica pares/pesos contra as APIs legadas em grafos válidos e
  imutabilidade da entrada. Conteúdo de produção e testes anteriores foi
  comparado com cópia inicial: integralmente preservado, exceto data/hash.
- `cargo tree --offline -p lente_estrutura --edges normal`: só lente_core local.
- Snapshot e hashes atualizados por crystalline-lint em cópia temporária
  restrita aos arquivos autorizados: zero stale/drift após atualização.
- `git diff --check` nos arquivos da tarefa passou.

## Escopo e limites

Somente implementação, contrato e este laudo foram escritos nesta tarefa.
Nenhum tipo compartilhado, raio, comparação, renderizador, adaptador ou wiring
foi alterado. Integração HTML/CLI/MCP permanece posterior. Alterações locais
anteriores foram preservadas. A suíte executada é a do crate afetado, não a suíte
completa do workspace. A agregação explica relações estruturais observadas;
não comprova quebra comportamental nem completude da extração.
