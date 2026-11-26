# IndexStore Swift (L3)
Hash do Código: f5c75cee

Lê um IndexStore produzido pelo Swift e pelo Clang no mesmo build
através da libIndexStore da toolchain, sem executar o projeto ou o build.
Restringe as fontes ao diretório canônico informado. Usa posições de
linha e coluna em bytes com base um e associa ocorrências pela identidade
USR do compilador, incluindo sobrecargas C++ e seletores Objective-C.\nConfere o identificador visível da ocorrência com a grafia do índice.\nConstrutores Swift podem aparecer com o nome do tipo na chamada.

Uma fronteira exige uma ocorrência de chamada explícita em Swift e
uma definição nativa única indexada em C, C++ ou Objective-C. Protocolo
sem corpo, definição ambígua e ocorrência implícita ficam sem fronteira.
O destino Objective-C representa a definição estática indexada.
A relação CALLEDBY identifica o chamador. Na falta dela, aceita um
único CONTAINEDBY de função com definição indexada.

Projeta chamadas Swift e nativas no grafo de fontes da Lente.
Ao juntar com SCIP/ELF, mantém as fronteiras existentes, combina chamadas
por posição e une referências não FFI quando a declaração é inequívoca.
Usuário ou ferramenta deve fornecer as fontes do mesmo build.

Dada chamada de uma sobrecarga C++ Então abrir a sua definição exata.
Dados dois seletores iguais em classes distintas Então não unir as identidades.
Dada chamada Objective-C com declarações mas sem definição local Então não inventar implementação.
