# Explorar chamadas C e C++ (L3)
Hash do Código: 3c1b5741

Recebe uma base de compilação explícita e a raiz SCIP. Admite caminhos locais de
unidades C e C++ sob a raiz, interpreta `arguments` ou `command` como argumentos
literais e usa a mesma seleção segura de opções do leitor nativo de FFI.
Nunca executa o compilador indicado. Configurações idênticas são deduplicadas.
Configurações divergentes e unidades rejeitadas são informadas na cobertura.

O leitor de libclang processa unidades em paralelo, preserva diretório e
configuração, e entrega definições e sítios de chamada do arquivo principal
com identidade USR e posições do identificador. O adaptador valida as posições
contra os bytes usados no HTML e liga cada sítio à definição local inequívoca,
inclusive entre arquivos. Usa declaração navegável ou diagnóstico quando
o corpo está fora do recorte. Despacho virtual corresponde ao destino estático
observável. Chamadas indiretas, nomes sem intervalo explícito e definições
apenas em cabeçalhos ficam fora do grafo. Unidades com erros não contribuem.

A cobertura registra o total de unidades locais, as analisadas e as exclusões
com motivo. Uma base sem unidades analisáveis é erro explícito. Os dados são
somados ao grafo Go por meio do contrato de domínio, compatíveis com a navegação
e os fluxos existentes e independentes de um artefato FFI.
