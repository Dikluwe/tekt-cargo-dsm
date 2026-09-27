# Integração das entregas 21–24 por processos explícitos

## Entrega

A integração de provedores da entrega 21 foi preservada e revalidada. Adicionados
`lente --analise-artefatos CONFIG.json` e ferramenta MCP `analisar_artefatos`.
Os modos snapshots, regras, dsm e caminho-dsm usam o executor L3 existente
(prazo, grupo de processos, diretório temporário, sem shell).

A entrega 22 tinha biblioteca pura, sem fronteira de arquivos. Foi acrescentado
um workspace CLI independente em `lab/impacto-snapshots/cli`, que desserializa
entrada estrita, delega à biblioteca e publica todo o resultado em JSON.
As entregas 23 e 24 conservam seus executáveis existentes. Produção não depende
de crates da Arena; a fronteira é processo, coerente com ADR-0008.

O envelope preserva resultado integral, stderr e código original. Regras 1/2
são violação/inconclusivo, não falha de transporte. MCP retorna `isError=false`
nesses casos. Falhas são código 3 na CLI; sintaxe de argumentos ainda segue clap.

## Verificação executada

- Suíte workspace: 397 testes + 2 doctests aprovados; 45 testes opt-in ignorados
  nessa invocação (não contados como aprovados).
- Nove opt-in de provedores executados separadamente: Python, TypeScript, Zig,
  C, C++, Go, pacote verificado, cobertura parcial, raio/estrutura. Todos passaram.
- Snapshots: 22 testes da biblioteca e 2 da nova CLI passaram.
- Regras: 19 testes da biblioteca e 5 do executável passaram.
- DSM: 4 testes Rust e 7 testes HTTP passaram.
- Novo harness: 7 testes ponta a ponta com executáveis reais passaram. Cobrem
  remoção, renomeação explícita, parcialidade, regras 0/1/2, data inválida,
  ferramenta ausente, preparação DSM/caminho direcional e exclusividade CLI.
  Comparam JSON CLI/MCP; falha de pré-requisito não vira skip.
- `git diff --check` passou. Hashes e snapshots das interfaces tocadas foram
  atualizados seletivamente, preservando pendências de outras unidades.

Não se declara linter global verde: ainda existem drift V5/V6 de outras unidades
e a configuração atual faz o linter percorrer `03_infra/typescript/node_modules`,
produzindo V1 e erro PARSE em declaração TypeScript de terceiro. Não foram
reescritos contratos alheios nem excluídos fontes de produção para ocultar erros.
Não foi feita nova inspeção visual em navegador; a validação DSM desta integração
foi do preparo, consulta, equivalência CLI/MCP e servidor HTTP.

## Limites preservados

Não há captura automática de snapshots Git, inferência de identidade, conversão
universal de formatos nem servidor persistente iniciado pelo MCP. O servidor
DSM é aberto explicitamente. Cobertura continua declarada pelo provedor.
O executor requer Unix e executáveis confiáveis; não é sandbox de permissões.
Guia operacional e comandos: `docs/analises-artefatos.md`.
