# ADR-0008 — Integração por processo dos adaptadores opcionais

Aceito em 2026-09-26. Complementa ADR-0007.

A auditoria encontrou implementações executáveis dos seis adaptadores em lab,
com contratos diferentes e testes locais, além do empacotador/verificador da
 tarefa 13. A preparação L4 descrita em wiring-analise.md ainda não existe no
checkout: não se presume implementação a partir do contrato ou diretório.

Integração aditiva: provedores externos são configurados explicitamente na CLI
ou ferramenta MCP analisar_provedor. Rust/JSON legados mantêm argumentos e fluxo.
A fronteira ExtratorGrafo recebe uma solicitação de ferramenta; L3 preserva o
contexto rico à parte do envelope L1, que não será alterado nesta tarefa. Json
identifica apenas transporte nesse envelope; não renomeia o produtor real.

L4 calcula diretamente sobre grafos externos com paths únicos, sem resolver
colisões Rust nem filtrar prefixos Rust. Consulta oferece raio, ranking e
estrutura; diff, comparação, filtros Rust e HTML não fazem parte da integração.
Resultados JSON incluem análise e contexto integral. Não há cache externo.

O formato verificável é consumido pelo executável existente, sem dependência
Cargo de lab e sem segunda implementação. Configuração exige caminhos absolutos
e parâmetros tipados por provedor. C seleciona uma observação TU/configuração;
Go permanece pacote; Zig permanece candidato sintático. Não se promete
uniformidade semântica a partir da uniformidade do protocolo de chamada.

Ferramentas são preparadas pelo usuário; análise não instala nem compila helpers.
Processos Unix têm timeout total e grupo próprio encerrado ao terminar. A garantia
abrange os helpers existentes, que não escapam do grupo, não executáveis hostis.
