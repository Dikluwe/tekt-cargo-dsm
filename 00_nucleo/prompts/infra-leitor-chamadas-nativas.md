# Leitor de ocorrências de chamada da libclang (L3)
Hash do Código: 584e819c

Recebe somente argumentos e diretórios já admitidos pelo adaptador Rust.
Invoca a API C da libclang para construir a AST, sem executar o compilador
da base. Usa biblioteca do sistema ou `LENTE_LIBCLANG` e distribui unidades
independentes entre processos. Cada unidade emite funções com corpo,
sítios diretos com identificador escrito e seus USRs, ou o erro do Clang.
Posições são linha e coluna UTF-8 base um. Emite uma observação por unidade,
na ordem da entrada, e a versão do Clang.
