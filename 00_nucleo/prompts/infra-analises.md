# Contrato — análises de artefatos por processo

Unidade: `03_infra/src/analises.rs`, L3. Complementa ADR-0008.

Integra as entregas snapshots, regras e DSM por executáveis opcionais explícitos;
produção não importa crates da Arena. Configuração JSON estrita com `executavel`,
`timeout_ms` e `consulta` tagged por `modo`. Modos: snapshots (entrada), regras
(baseline, atual, regras, data), dsm (entrada) e caminho-dsm (pacote, consumidor,
alterado). Caminhos absolutos, entradas existentes. Não instala ferramentas nem
infere correspondências, cobertura ou datas.

Reutiliza o executor Unix de provedores: diretório privado por chamada, prazo,
grupo de processos, sem shell. Retorna envelope `analise` (JSON integral) e
`execucao` (modo, código original, stderr). Somente regras aceita 1/2 como
resultados de domínio: violação/inconclusivo, não falhas de transporte. Demais
modos exigem zero. Regras deve apresentar status/código coerentes.

DSM devolve o pacote preparado original, que pode ser salvo e aberto pelo
servidor local existente; não inicia servidor persistente implicitamente.
CLI publica JSON em stdout preservando código de regras. MCP retorna o mesmo
envelope, isError=false para resultados de domínio e true para falhas.

Testes: config inválida, campos desconhecidos, caminho relativo, timeout inválido;
integração com executáveis reais: remoção, regras completa/parcial e DSM parcial
com evidências e caminho. Ferramentas ausentes nunca são testes aprovados.
