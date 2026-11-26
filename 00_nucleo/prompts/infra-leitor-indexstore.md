# Leitor da API C do IndexStore (L3)
Hash do Código: 3c59f623

Abre a libIndexStore indicada via LENTE_LIB_INDEXSTORE ou a biblioteca
da toolchain Swift disponível no PATH ou selecionada por Xcode.
Usa a API pública de unidades, dependências e ocorrências. Envia ao
adaptador Rust dados de callables, posições, funções envolventes e a
identidade USR. Ignora registros de caminhos externos à raiz canônica.
Falhas de leitura são reportadas à CLI. O leitor não executa código,
build ou scripts do projeto indexado.
