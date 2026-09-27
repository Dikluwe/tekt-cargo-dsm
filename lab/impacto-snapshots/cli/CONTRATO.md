# Fronteira JSON para impacto entre snapshots

Executável separado; biblioteca pura original continua apenas stdlib + lente_core.
Aceita exatamente um arquivo JSON com `format_version:1`, `antes`, `depois`,
`correspondencias` e `mudancas`. Cada snapshot contém `versao`, `grafo` inline,
`granularidade`, `natureza`, `completude` (completa/parcial/desconhecida, padrão
desconhecida) e `diagnosticos` (padrão vazio). Não inferir completude nem mudanças.
Correspondências: identidade/antes/depois. Mudanças: tipo remocao/adicao/pareada
com os campos da biblioteca. Campos desconhecidos são erros na fronteira.

Delega validação semântica e cálculo à biblioteca; traduz grafo pela infraestrutura
existente. Saída JSON contém todos os resultados da biblioteca, nomes explícitos
de enums, paths originais, lados, completude, resíduos e limites. Erros em stderr,
exit 3; relatório em stdout, exit 0 (inclusive parcial). Sem Git/build/subprocesso.
