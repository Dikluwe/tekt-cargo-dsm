# Exploração de chamadas Go (L3)
Hash do Código: 37df6614

Executa um leitor Go dedicado na fonte embutida no índice. Usa as
identidades existentes em SCIP ou no índice legado Go para ligar
as posições de uso às declarações. Faz a projeção L1 independentemente
de ELF. Ausência do leitor ou da fonte preserva a navegação anterior.
Dados legados sem coluna de declaração permanecem sem seta.

Cada referência de chamada informa o arquivo de índice e, no índice legado, a coleção. Uma referência sem posição de declaração ainda informa sua origem.
