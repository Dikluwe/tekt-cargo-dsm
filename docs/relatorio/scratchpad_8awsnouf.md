# Plano de Validação do DSM MVP

## Tarefas de Inspeção
- [x] Inspecionar Typst DSM (`file:///tmp/dsm-validation/typst-dsm.html`)
  - [x] B.1 Estrutural (Matriz quadrada, cabeçalho OK. Sem rodapé/legenda visible)
  - [x] B.2 Forma triangular (Marcas abaixo da diagonal e blocos contíguos na diagonal)
  - [x] B.3 Ciclos (Bloco SCC gigante e bloco typst-syntax visíveis)
  - [x] B.4 Interatividade (Toggle "Show only cyclic SCCs" funcionando perfeitamente)
  - [x] B.5 Performance (Rolagem fluida e carregamento instantâneo)
  - [x] Screenshots (`typst_dsm_top`, `typst_dsm_cyclic_only`, `typst_dsm_middle` tiradas)
- [x] Inspecionar typst-crystalline DSM (`file:///tmp/dsm-validation/crystalline-dsm.html`)
  - [x] B.1 Estrutural (Matriz quadrada, cabeçalho OK. Sem rodapé/legenda)
  - [x] B.2 Forma triangular (Maiores marcas abaixo da diagonal. Pequenas caixas vermelhas visíveis)
  - [x] B.4 Interatividade (Toggle cíclico e campo de busca funcionam muito bem)
  - [x] Screenshots (`crystalline_dsm_cyclic_only`, `crystalline_dsm_search` tiradas)
- [x] Inspecionar self DSM (`file:///tmp/dsm-validation/self-dsm.html`)
  - [x] B.1 Estrutural (Matriz quadrada, cabeçalho OK: 53 nós, 0 ciclos. Sem rodapé)
  - [x] B.2 Forma triangular (Perfeita. Como tem 0 ciclos, nós internos são puramente triangulares)
  - [x] B.4 Interatividade (Toggle "Hide external nodes" removeu com sucesso nós da direita)
  - [x] Screenshots (`self_dsm_internal_only` tirada)

## Notas e Descobertas
- **Ponto Importante de Arquitetura:** Toda a matriz DSM é renderizada usando um elemento `<canvas id="dsm-matrix">` em vez de elementos de tabela DOM individuais. Isso é um excelente detalhe de design, pois permite renderizar matrizes gigantescas (como a do Typst de 637x637 que exigiria mais de 400.000 células DOM) instantaneamente e sem nenhum lag de scroll.
- **Interatividade:**
  - O filtro "Show only cyclic SCCs" é altamente responsivo.
  - O filtro "Hide external nodes" funciona perfeitamente, o que é ótimo para isolar o workspace.
  - A pesquisa "Filter nodes..." destaca com precisão os nós correspondentes, aplicando opacidade reduzida aos demais, facilitando a depuração visual.
- **Estrutura:**
  - Não há rodapé com legenda. Pode ser interessante adicionar uma legenda simples no futuro explicando o significado das cores (azul para dependência, vermelho para ciclo/SCC, cinza para diagonal).


