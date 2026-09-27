# Contrato — configurações reproduzíveis para testes de provedores
Hash do Código: f7377afa

Unidade `03_infra/fixtures/provedores/gerar_configuracoes.py`, L3, suporte de testes.
Recebe diretório de saída novo; escreve configurações JSON absolutas para os seis
helpers opcionais e compilation databases para fixtures C/C++ existentes.
Cria uma fixture Python com erro sintático. Não extrai, instala, importa código
experimental, executa projeto ou altera fixtures originais. stdlib Python.
Permissões para metadados sem vínculo e ambiente ficam explícitas no JSON gerado
para C/Go. Libclang/GOOS/GOARCH do exemplo são Linux/18/linux/amd64 e podem ser
ajustados pelo usuário. Ver docs/adaptadores.md para execução dos testes opt-in.
