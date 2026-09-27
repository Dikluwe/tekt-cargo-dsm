# ADR-0007 — Fronteira explícita de provedores de grafo

Status: aceito. Data: 2026-09-26. Registrado antes da implementação.

## Contexto e correções já integradas

A spec separa forma organizada de sua fonte. Os algoritmos L1 já consomem
`Grafo`, mas o wiring escolhe diretamente `fork::invocar_fork`, desserialização,
extração por diretório e cache por membro. CLI/MCP leem JSON em L4.
Estão integradas as correções de falhas/completude do diff, identidade canônica
do crate separada do pacote Cargo, toolchain por membro, cache v2 conservador,
comparação por mapa declarativo e preservação de metadados traduzidos.
Essas alterações locais devem permanecer.

## Decisão

- L1 ganha somente o envelope puro `ExtracaoGrafo`: grafo cru conforme a spec,
  `IdentidadeProvedor` (RustCargo, Json, Memoria) e diagnósticos textuais.
  Não conhece executáveis, adaptadores, processos, filesystem ou L3.
- L3 define `SolicitacaoExtracao` e `ExtratorGrafo`, com adaptadores concretos
  Rust, JSON e memória. Solicitações de pacote, diretório ou membro cacheado
  são infraestrutura e permanecem fora do núcleo. Nenhum plugin dinâmico.
- L4 escolhe explicitamente o adaptador. Uma preparação injetável recebe
  extrator + solicitação, extrai uma vez, resolve colisões e aplica o escopo.
  A análise preparada conserva proveniência/diagnósticos e permite estrutura,
  ciclos, ranking e impacto sem extrair novamente e sem Cargo em testes.
- O adaptador Rust delega aos invocadores, tradutor e cache existentes, sem
  reescrever detecção de alvo, flags, sysroot, cache ou política de toolchain.
  Workspace compartilha uma instância do cache por rodada; resolução continua
  por crate antes da união. Nenhuma consulta Cargo nova para selecionar provedor.
- JSON continua usando o tradutor e suas validações; memória recebe um Grafo
  que deve satisfazer os invariantes de entrada da spec, como as demais APIs L1.
- Erros da fronteira preservam provedor, contexto da solicitação e causa tipada
  acessível por `Error::source`. Resolução/algoritmos mantêm seus erros próprios.
- Leitura de arquivos de entrada e canonicalização passam a helpers L3; L4
  compõe. I/O de apresentação (stdout/stdio/HTML) não é extração.
- Flags e formatos analíticos existentes permanecem. `FonteGrafo` continua
  sendo a fachada compatível Json/Pacote; a API explícita de extração é aditiva.
  As mensagens de falha de extração ganham contexto do provedor.

## Mapa dos acoplamentos e limites desta fase

| Local | Acoplamento | Destino/limite |
|---|---|---|
| CLI/MCP `--pacote`, `pacote`, `--grafo` | Vocabulário histórico Rust/JSON | Preservado; convertido em seleção L4 |
| Wiring obtenção unitária | Invocação do fork e tradução | Adaptadores L3 atrás da fronteira |
| Wiring workspace/diff | Enumeração Cargo, toolchain, união por crate, Git | Composição Rust explícita; não há descoberta de workspace universal |
| Wiring comparação | Detecção crate/workspace e seleção por diretório | Extração unitária pela fronteira; detecção Rust permanece em L3 |
| L3 fork/metadata/invocação/cache/workspace | Cargo, rustc, TOML, sysroot, flags e cache | Encapsulados sem reescrever sua lógica |
| L1 filtros e modelo | Prefixos core/std/alloc, `::`, enums Kind/Visibility e descritor Rust | Inalterados; novos provedores devem traduzir para esta spec |
| L1 mapeamento de diff | Arquivos `.rs`, posição de fonte e diretórios de membros | Permanece especializado; impacto sobre Grafo é independente do extrator |
| Resolução de colisões | Convenções de trait e paths Rust | Mantida; um futuro provedor pode entregar paths já únicos |

Preparação para novos provedores não significa suporte a outra linguagem já
implementado. A seleção deverá ser ampliada deliberadamente quando existir um
adaptador que cumpra a spec. Não há promessa de cache/escopo/diff universais.

## Verificação

Comparar JSON e memória com os mesmos grafos (incluindo ciclos e colisões),
comprovar uma extração por preparação e nenhuma por análise, preservar avisos,
verificar causas/contexto dos erros, testar delegação Rust por operações
controladas, executar regressões CLI/JSON/workspace/cache e suíte completa.
