# Manifesto de testes pela CLI (L3)
Hash do Código: f445000d

Lê um JSON local e admite a versão 1. O formato registra `versao`,
`revisao`, `testes` (lista de suítes medidas) e `regras` (caminho da regra
para a lista de suítes que executaram alguma função de checagem). Uma lista
vazia em `regras` declara uma regra sem execução observada nessa coleta.
Todos os caminhos são relativos à raiz e coincidem com arquivos do índice.
Cada suíte de uma regra pertence ao escopo medido. A revisão dá proveniência
à coleta. O produtor e o índice usam o mesmo estado do código.

A saída é metadado de navegação. O adaptador não cria arestas e só atualiza o
índice depois de admitir todo o manifesto.

## Verificação

- Dado um manifesto válido com execuções repetidas, a vista recebe uma lista
  ordenada sem repetições por regra e conserva as regras sem observação.
- Versão incompatível, caminhos fora do índice ou execuções fora do escopo
  interrompem a exportação solicitada.
