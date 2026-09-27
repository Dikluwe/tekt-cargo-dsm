# Contrato — integração de análises por artefatos

Script de teste stdlib em `03_infra/fixtures/analises/test_integracao.py`.
Executa os binários reais da CLI, MCP e três ferramentas opcionais. Não instala,
não usa rede, não altera exemplos existentes. Entradas e configurações ficam em
diretório temporário privado. Falta de executável é falha, nunca sucesso/skip.

Verifica equivalência CLI/MCP; remoção com consumidor no grafo anterior;
cobertura parcial sem inferir ausência; renomeação com identidades explícitas;
regras aprovadas, violação e inconclusivo; falhas de configuração; DSM parcial
com preparação e consulta de caminho direcional. Nenhum resultado parcial pode
ser promovido a completo. Resultados de domínio não são erros MCP.
