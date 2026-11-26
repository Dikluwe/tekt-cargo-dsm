# Índice de referências de fonte (L1)
Hash do Código: ddf6b99d

O índice recebe fontes e posições normalizadas em linha base um e colunas
UTF-8 em bytes base zero, com fim exclusivo. Cada símbolo conserva a
identidade resolvida pelo adaptador, todas as declarações e todos os usos.

A projeção agrupa cada uso pelo par dirigido arquivo do uso → arquivo da
declaração. Conta um uso uma vez por destino, mesmo com várias declarações
nesse arquivo. Conserva referências locais para a navegação e deixa
referências sem declaração fora do grafo. Arquivos são nós de módulo e
arestas representam referências estáticas entre arquivos. O raio descreve
alcançabilidade entre arquivos. Nenhuma operação do domínio faz I/O.

O índice pode carregar, por arquivo de código, o caminho relativo e o texto
original do seu prompt Markdown admitido pelo adaptador. Essa informação serve
à navegação e não participa da projeção do grafo nem do cálculo de raio.

A prova FFI distingue ELF/DWARF de identidades confirmadas no IndexStore.

Um vínculo FFI verificado acrescenta o arquivo da implementação como
destino dos usos da declaração estrangeira. Mantém a declaração original
e as posições de uso disponíveis. O par conta o uso uma vez por arquivo.
O vínculo pode registrar os locais de chamada direta e o nome
da função envolvente, quando a posição de uso coincide com o callee.
Os chamadores indexados são projetados por identidade de símbolo e posição\nda declaração. Uma posição ambígua no SCIP exige prova direta da ponte\npara produzir uma etapa. Uma ocorrência direta confirmada no adaptador\npermanece mesmo que o SCIP registre um símbolo próprio da linguagem. O índice mantém apenas
predecessores dos pontos de entrada FFI e das funções nativas alcançadas
por chamadas diretas depois da fronteira. Um destino nativo exige um
corpo reconhecido pelo Clang e identidade SCIP inequívoca, incluindo
funções folha. Os predecessores das duas entradas podem convergir
depois do ponto FFI. A projeção percorre os ciclos uma vez por posição.
Os predecessores nativos entram pelo SCIP quando a posição de implementação
coincide com a declaração do índice. Quando falta a ocorrência SCIP, aceita
a chamada direta Clang ao nome externo de link já confirmado no ELF/DWARF.
Uma ocorrência SCIP conhecida que contradiga esse vínculo impede essa
complementação. O índice conserva a cadeia Rust e as posições verificadas.

## Verificação

```
Dadas duas declarações de um símbolo no mesmo arquivo e um uso externo Então o peso do par é um
Dado um uso local Então a posição permanece navegável sem produzir uma aresta entre arquivos
```
