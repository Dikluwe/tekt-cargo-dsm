# Verificação — 2026-09-26

Escrita restrita a `lab/regras-arquiteturais/`. Biblioteca pura e executor em
workspace independente. Nenhuma integração CI/CLI/MCP nem alteração em adaptadores,
tipos compartilhados, harness ou crystalline-lint.

## Pré-requisitos conferidos

- Spec `forma-organizada`: ID local é identidade no grafo cru; paths podem repetir.
- Envelope/contratos atuais: execução completa não comprova completude semântica.
- Entrega 13, pacote de análise: leitura do README/CONTRATO; associação por digest,
  proveniência desconhecida e versões de schema não são promovidas a certeza.
- Entrega 17, comparação por identidades opacas: contrato, implementação pública
  e laudo conferidos. Não se usa matching Rust nem ID local como identidade entre
  snapshots. Sua API de estruturas agregadas não foi duplicada ou modificada.
- APIs de explicação de caminho e SCC reutilizadas diretamente.

## Resultado

- **24 testes aprovados**: 19 da biblioteca e 5 da borda de arquivos, sem ignorados.
- Clippy de todos os targets com `-D warnings`: aprovado.
- `cargo fmt --check` e build: aprovados.
- CLI real: exemplos de aprovado (0), violação (1), inconclusivo (2) e erro de
  entrada (3) produziram os resultados esperados, em JSON/texto quando aplicável.
- Digest incorreto: erro 3, sem relatório de aprovação.
- Repetição das quatro avaliações com saída JSON: bytes idênticos com mesmos
  arquivos, paths dos argumentos e data.

O exemplo principal retorna 1: alcançabilidade nova app:A → data:B → data:C,
SCC novo data:B ↔ data:C, e dependência direta antiga app:A → data:B sob exceção.
O exemplo new_only mantém a dívida preexistente visível, sem cobrá-la como nova.
O exemplo parcial retorna 2 mesmo com a declaração local de qualidade completa.

Relatórios locais reproduzíveis, ignorados por Git:

- [Violações JSON](out/violations.json) e [texto](out/violations.txt).
- [Aprovação](out/approved.json).
- [Dívida preexistente](out/preexisting.json).
- [Inconclusivo por parcialidade](out/partial.json).

## Limites verificados

Completude/capacidades/identidades são declarações explícitas; hashes não as
certificam. Provedores nativos desconhecidos ou que declaram semântica parcial/
não avaliada permanecem inconclusivos. Compatibilidade exige o mesmo contexto.
Exceções não eliminam insuficiência de dados. Ciclo significa regressão de SCC
segundo a relação de subconjunto de membros/arestas, com autoarestas; não se
enumeram ciclos simples. IDs de observações de SCC podem mudar quando seu conteúdo
muda, inclusive em reduções não regressivas. Nenhuma aprovação comprova correção
funcional. Não foi feito benchmark nem se promete latência para seletores grandes.

Comandos completos, exemplos de regras e formatos estão no README/CONTRATO.
