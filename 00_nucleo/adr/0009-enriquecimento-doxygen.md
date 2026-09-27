# ADR-0009 — Documentação Doxygen opcional e referências pendentes

Aceito em 2026-09-26, antes da implementação. Complementa ADR-0007/0008.

Inventário v2 mantém schema `lente.doxygen.inventory` e adiciona campo obrigatório
`pending_relations`: objetos `{from,to,kind,reason}`; reason é `missing_from`,
`missing_to` ou `missing_both`, conferido contra os IDs declarados. `relations`
contém somente endpoints resolvidos. Pendências exigem coverage=partial, não
criam símbolos nem entram em matching, métricas ou navegação. IDs duplicados e
estruturas malformadas são erros. Diagnósticos podem referenciar IDs pendentes:
são contexto de uma perda, nunca existência de um símbolo navegável.

Consumidores continuam aceitando v1 com endpoints resolvidos. V1 com endpoints
pendentes permanece erro explícito, com orientação de migração; não reinterpretar
silenciosamente v1. Migrar: preservar símbolos/diagnósticos, separar relações pela
existência dos endpoints, preencher reason, marcar partial e incrementar version.
Recalcular correspondências porque os bytes e hashes mudaram. O importador passa
a emitir v2. Catálogo, matches, impact_summary permanecem v1; relatório prioridade
passa a v2 (campo pending_relations obrigatório). Explorador preserva ambas as
listas e permite candidatos de unmatched, como o contrato do associador.

Catálogo/impacto são projeções explícitas do grafo recebido, com SHA-256 dos bytes
exatos. Localização somente de No.file/position. Política padrão de nome é opaca
(null); `rust-path` é opt-in para grafos do provedor Rust cuja spec define path
canônico; não aplica essa convenção a grafos C/C++/arquivos. Assinaturas null.
Catálogo auxiliar explícito continua possível, verificado por hash e IDs na DSM.
Raio L1 existente conta montante Uses de todos os subtipos, consumidores distintos,
sem o próprio alvo, inclusive ciclos; Owns não propaga. Paths não únicos impedem
as métricas (null, diagnóstico), sem resolver identidades por heurística. Cobertura
do grafo é uma declaração separada, default unknown, nunca inferida do parse.

Produção amplia analisar_artefatos com operações Doxygen tipadas, por executáveis
explícitos; não importa lab nem inicia servidor pelo MCP. Protocolo --output usa
arquivo privado exclusivo, só lido após conclusão e validação. Resultado parcial
exit 2 do importador é sucesso de transporte e código original preservado. CLI
retorna 0 para tais resultados; falhas continuam CLI 3/MCP isError=true.

DSM só habilita painel após conferir bytes do grafo, igualdade com source_graph
e hashes de inventário/catálogo/matches pelo explorador real em modo finito JSON.
Relações documentais jamais mudam células, pesos ou alcance. Painel usa texto,
candidatos separados, pendências e cobertura. Sem URLs automáticas ou rotas de
arquivo. Ferramentas confiáveis e entradas estáveis; hashes não autenticam origem.
