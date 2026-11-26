# Grafo de chamadas por símbolo (L1)
Hash do Código: 09ace485

Projeta os pontos de chamada fornecidos pelo adaptador e destinos
resolvidos por identidade no índice de referências. A identidade é a
posição completa da declaração, em linha base um e coluna UTF-8 base zero.
Um ponto só produz aresta quando identifica uma função de origem e
tem um destino inequívoco que é uma função reconhecida. Conserva
chamadas distintas entre o mesmo par, inclusive chamadas recursivas.
O resultado é independente da projeção de fronteiras FFI.

Dada referência de valor sem ponto de chamada, preservar só a referência.
Dado ponto ambíguo ou destino sem função reconhecida, não projetar seta.

A projeção conserva a origem indexada por ligação. Por ponto sintático, distingue ausência de referência, referência sem declaração, destinos conflitantes e declaração fora do recorte. Candidatos iguais por posição unem suas origens.

Grafos C/C++ e Go podem ser unidos mantendo identidades locais, posições,
arestas e diagnósticos. Quando houver extração nativa, o resultado conserva
a cobertura das unidades de compilação e as exclusões com motivo.
