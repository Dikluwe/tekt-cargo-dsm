# Leitor sintático de chamadas Go (L3)
Hash do Código: 8862bfaf

Usa go/parser nos textos Go recebidos como dados. Emite funções
nomeadas com corpo e callee de chamadas diretas. Considera seletores
e instanciação genérica, preserva os limites de funções anônimas
e usa posições físicas de arquivo com colunas em bytes. Não compila
ou carrega o projeto analisado e não resolve símbolos por nome.
