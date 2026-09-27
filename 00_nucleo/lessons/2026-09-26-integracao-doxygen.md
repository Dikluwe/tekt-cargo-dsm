# Laudo — integração documental Doxygen

Data: 2026-09-26. Escopo e decisão registrados antes do código em ADR-0009,
contratos locais e prompts L3/MCP. Guia reproduzível: `docs/doxygen.md`.
Nenhum commit, instalação, download ou alteração em outro projeto. Os quatro
experimentos já estavam presentes e não rastreados no início; foram evoluídos
sem removê-los. Cargo.toml/Cargo.lock da raiz e tipos/algoritmos L1 inalterados.

## Resultado

Inventário v2 separa relações resolvidas e pendentes com motivo verificável.
Consumidores aceitam v1 resolvido e v2; inventário v1 pendente requer migração
explícita. IDs duplicados são erro. Pendências não criam símbolos nem alteram
matching, contagens, células ou alcance. Relatório de prioridade agora é v2.
Catálogo/matches/impacto continuam v1. Diagnósticos de perdas podem mencionar IDs
não existentes; essa menção não é uma referência navegável.

Corrigida também a incompatibilidade de unmatched com candidatos: são permitidos
para inspeção, sem documento confirmado. Campos de localização são validados;
diagnósticos do catálogo também chegam ao painel. Publicação dos três consumidores
passou de escrita direta para temporário irmão + hard link exclusivo, como o
importador. Não há promessa de durabilidade de nome após queda de energia.

Projeção L3 recebe bytes reais do grafo, usa desserializador e raio existentes,
produz catálogo/impacto com o mesmo SHA-256. Nomes opacos permanecem null; rust-path
é política explicitamente declarada, sem aplicação universal a C++. Métrica
indisponível/fora da seleção permanece null. Contagem de consumidores Uses únicos,
alvo excluído mesmo em ciclos, Owns ignorado. Coverage declarada é separada do parse.

Cinco modos novos em analisar_artefatos, mesmos resultados CLI/MCP. Processos
explícitos usam executor existente, timeout/grupo/temporário, sem shell. Parcial
do importador tem código original 2, CLI 0/MCP isError=false. Erros preservam
status/stderr e categorias schema/digest/execução. Explorar prepara JSON finito,
sem iniciar servidor. DSM opcional verifica os bytes do grafo, IDs do catálogo,
source_graph, campos estruturais recalculados e vínculos dos três artefatos pelo
explorador real; relações Doxygen não mudam estrutura.

## Testes efetivamente executados

Comandos de build/test offline e harness estão no guia e READMEs. Contagens finais:

| Verificação | Resultado |
|---|---|
| `cargo test --offline --locked --workspace` | 401 aprovados; 0 falhas; 45 ignorados pelas marcações existentes |
| Importador | 21 aprovados |
| Correspondência | 19 aprovados |
| Prioridade | 20 aprovados |
| Explorador | 16 aprovados |
| Correspondência, `verificar_cli.py` | 5 aprovados |
| DSM, testes Rust | 4 aprovados |
| DSM, testes Python HTTP existentes | 7 aprovados |
| Harness legado `03_infra/fixtures/analises/test_integracao.py` | 7 aprovados: snapshots/regras/DSM/CLI sem Doxygen |
| Harness compartilhado, sintético + Doxygen real | 62 verificações de processos reais, incluindo recusas esperadas |
| Clippy `-D warnings` das quatro entregas | Passou |
| rustfmt das cinco unidades de produção modificadas e Cargo fmt das quatro entregas | Passou |
| `git diff --check` | Passou |
| Lint focado nas cinco unidades, V1–V6,V8–V10,V13,V14 | Sem violações |
| `cargo tree --edges normal -p lente_core` | Nenhuma dependência; L1 puro |
| Árvore normal de lente_infra | Sem dependências Cargo de lab |

45 testes ignorados não foram contados como aprovados; incluem testes opcionais
com ferramentas externas do produto. Não afirmamos execução de todos esses testes.
As suítes de componentes de produção alterados (infra/MCP) estão incluídas na
suíte principal. Os dois testes novos L3 cobrem bytes/hash, política de nomes,
localização, null, ciclos, seleção inválida, colisão de paths e permutação.

O harness exige binários, cria diretórios exclusivos e usa outputs reais.
Compara SHA-256 dos arquivos exatos, repetição byte a byte, decisões de matching,
cobertura e pendências, bem como CLI=MCP para cinco modos. Exercita falha schema,
digest e ferramenta ausente, XML inválido sem publicação, saídas preexistentes e
DSM adulterada. A publicação concorrente do importador continua testada localmente.

Fixture sintética: 3 matched (inclui duas sobrecargas e documento sem texto),
1 ambiguous (dois candidatos), 4 unmatched (inclui candidato insuficiente),
1 relação resolvida, 1 pendente, coverage partial. Métrica do nó 60 é null e do
70 é zero. O catálogo auxiliar é explicitamente escrito pelo autor da fixture;
o harness primeiro verifica que a projeção opaca não inventa nomes qualificados.

## XML real

`/usr/bin/doxygen --version` retornou **1.9.8**. Executado explicitamente no
harness `--real-doxygen`, com `real.hpp` próprio e Doxyfile controlado, sem
compilar/executar C++. Configuração: XML=YES, HTML/LATEX=NO, EXTRACT_ALL=YES,
QUIET=YES, XML_NS_MEMB_FILE_SCOPE=NO, INPUT=demo.hpp, OUTPUT_DIRECTORY=output.
SHA-256 dos bytes do Doxyfile:
`b4573417125179eb03fb4b44e678912468377d62cf58006b09651e363ab29f8c`.

O fluxo real produziu **2 matched e 1 unmatched**, coverage partial. O catálogo
usa declarações independentes conhecidas das linhas 4 e 6, não é copiado do
inventário para forçar uma associação. O grafo auxiliar é uma fixture estrutural
manual correspondente a esses elementos, não uma execução de provedor C++.

Perdas efetivamente observadas: localização de compound file sem linha válida e
programlisting fora do perfil; diagnósticos preservados. Não houve referência
pendente nessa fixture real; a fixture sintética cobre esse caso explicitamente.
O hash de configuração vive no resultado do harness; o importador mantém null,
pois não lê o Doxyfile. Não se ampliou o parser para outras versões.

## Inspeção visual efetiva

DSM aberta em navegador IAB, loopback porta 8876, com bundle da fixture sintética.
Selecionados os nós 10, 30 e 50: texto Unicode e missing_to visíveis no confirmado;
amb-a/amb-b separados, sem texto atribuído no ambíguo; ausência de texto explícita
no associado vazio. Screenshot inspecionado: aviso de parcialidade, seleção,
evidências e pendência legíveis. Não foi feito teste visual em mobile nem auditoria
completa de acessibilidade. Sem URLs externas abertas. Os sete testes HTTP antigos
comprovam funcionamento sem documentação. Não há screenshot versionado nesta rodada.

## Pendências globais preservadas na entrega de 26/09

Atualização: saneamento autorizado e executado em 27/09, com resultados no
[laudo de fechamento](2026-09-27-fechamento-doxygen.md). A lista abaixo registra
o estado anterior ao fechamento.

- `cargo fmt --all -- --check` acusa formatação preexistente em 17 arquivos fora
  do escopo. Uma aplicação incidental de cargo fmt foi retirada somente após
  comprovar byte a byte que cada diff era exatamente rustfmt(HEAD), conferir hashes
  contra alterações concorrentes e manter backup. Nenhuma alteração dessas unidades
  permanece. A primeira tentativa ampla de restauração foi bloqueada pela revisão
  automática; a correção restrita e comprovada foi posteriormente aprovada.
- Clippy de produção com `-D warnings` falha em quatro ocorrências preexistentes
  de `clippy::cmp_owned`: metadata.rs (duas) e diff.rs (duas em testes).
  Não foram corrigidas nem suprimidas nesta tarefa.
- Lint arquitetural global: **102 erros** (101 V1 em TypeScript vendorizado sob
  node_modules, 1 PARSE) e **21 warnings**, inclusive drift anterior em unidades
  não alteradas. Não se modificou crystalline.toml para esconder isso.
- Snapshots/hashes foram atualizados pela ferramenta oficial apenas em infra,
  infra-analises, infra-provedores, infra-doxygen e mcp. Para a atualização focada,
  usou-se configuração temporária, excluindo unidades não alteradas. A auditoria
  global foi executada separadamente com a configuração original.

## Limites remanescentes

Matching continua conservador e não verifica equivalência semântica, revisão de
fonte nem veracidade do catálogo auxiliar. Hashes provam integridade, não autoria.
Parser é subconjunto XML 1.9.8 sem XSD completo; duplicações legítimas emitidas por
outras configurações são recusadas. Assinatura documental é argsstring. Não há
nome qualificado universal a partir de path, nem assinatura C++ reconstruída.
Contagens dependem da cobertura do grafo; raio por alvo não foi otimizado para
grafos grandes. Consumidores carregam JSON em memória; o executor não limita
bytes capturados. Entradas devem permanecer estáveis durante leitura. Processos
confiáveis, grupos Unix e filesystem local com hard links; sem garantia contra
executáveis que escapem deliberadamente do grupo ou contra queda de energia.

## Arquivos alterados/adicionados nesta rodada

- `00_nucleo/adr/0009-enriquecimento-doxygen.md`;
  `00_nucleo/prompts/{infra,infra-analises,infra-provedores,infra-doxygen,mcp}.md`;
  este laudo.
- `03_infra/src/{lib,analises,provedores,doxygen}.rs`;
  `04_wiring/mcp/src/main.rs`.
- `docs/{doxygen,adaptadores,analises-artefatos}.md`.
- `lab/doxygen-importador/{CONTRATO.md,README.md,exemplo-inventario.json,src/main.rs}`.
- `lab/doxygen-correspondencia/{CONTRATO.md,README.md,src/lib.rs,src/main.rs,src/publicacao.rs,fixtures/inventory.json,fixtures/matches.json}`.
- `lab/doxygen-prioridade/{CONTRATO.md,README.md,src/main.rs,src/publicacao.rs,fixtures/inventory.json,fixtures/matches.json}`.
- `lab/doxygen-explorador/{CONTRATO.md,README.md,src/lib.rs,src/main.rs,src/publicacao.rs,assets/app.js,fixtures/inventory.json,fixtures/matches.json}`.
- `lab/doxygen-integracao/{.gitignore,CONTRATO.md,README.md,RESULTADOS.json,test_integracao.py,fixtures/graph.json,fixtures/catalog-evidence.json,fixtures/real.hpp,fixtures/xml/index.xml,fixtures/xml/namespace.xml}`.
- `lab/dsm-explicavel/{CONTRATO-DOXYGEN.md,README.md,serve.py,web/app.js,web/index.html}`.

Não foram alterados manifests/lockfiles Cargo, CLI args/renderizadores de produção,
núcleo, provedores C/C++ nem outros experimentos.
