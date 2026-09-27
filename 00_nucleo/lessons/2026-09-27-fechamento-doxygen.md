# Fechamento das pendências da integração Doxygen

Autorização: pedido de resolver pendências e commitar, após a entrega de 26/09.
Preservar comportamento, APIs e alterações locais. Corrigir comparações Path
sem alocações, aplicar rustfmt e reconciliar linhagem com contratos existentes.
Dependências instaladas em node_modules não pertencem ao lattice autoral.
Propostas não materializadas serão identificadas como históricas, sem fabricar
implementações nem mover os documentos. ConsultaProvedor será DTO L3 reexportado
por L4 com o mesmo caminho público e protocolo. ErroLente continua em L4 conforme
exceção expressa no contrato wiring.md. Não adicionar dependências ao núcleo.

## Correções realizadas

- Rustfmt do workspace e correções Clippy mecânicas: comparações de Path sem
  alocação, Default derivado, condicionais equivalentes, entrada HashMap,
  iteração enumerada e constantes antes dos testes. Assinatura pública legada
  de comparação com oito argumentos preservada com exceção Clippy localizada.
- ConsultaProvedor desceu ao L3 como DTO serde; o reexport L4 conserva API e JSON.
- node_modules excluído como dependência de terceiros, também no gitignore.
- Duas propostas históricas identificadas nos próprios documentos e registradas
  em orphan_exceptions com justificativa específica, sem deslocar/remover arquivos.
- Option::None qualificado em três matches exaustivos: o linter confundia o
  identificador None com wildcard. Sem alteração de decisões ou valores.
- Hashes reconciliados pela ferramenta oficial; snapshot do envelope L1 e dos
  provedores atualizado. Metadado Hash do Código reposicionado no preâmbulo do
  contrato do mapa TOML para ser reconhecido pelo gravador oficial.

## Verificação executada em 27/09

| Comando | Resultado |
|---|---|
| `cargo fmt --all -- --check` | passou |
| `cargo clippy --offline --locked --workspace --all-targets --no-deps -- -D warnings` | passou |
| `cargo test --offline --locked --workspace` | 401 aprovados, 0 falhas, 45 ignorados |
| `cargo build --offline --locked --workspace` | passou |
| `cargo test --offline --locked --manifest-path lab/doxygen-importador/Cargo.toml` | 21 aprovados |
| mesmo comando, doxygen-correspondencia | 19 aprovados |
| mesmo comando, doxygen-prioridade | 20 aprovados |
| mesmo comando, doxygen-explorador | 16 aprovados |
| `python3 lab/doxygen-integracao/test_integracao.py` | 30 verificações de processos; fixture sintética parcial, 3 matched/1 ambiguous/4 unmatched, 1 pendência preservada |
| `crystalline-lint .` | 0 erros, 1 warning V12 aceito em wiring.md; informações V20 mantidas |
| `git diff --check` | passou |

Não foi executado novamente Doxygen nesta rodada. A execução real 1.9.8 e os
62 checks da rodada anterior permanecem documentados no laudo de 26/09.
Os 45 testes ignorados pelo workspace não foram contados como aprovados.

## Exceção arquitetural mantida

O único warning restante é ErroLente em L4, explicitamente aceito pelo contrato
wiring.md como soma de erros da composição desde 0065. Ele não foi suprimido no
linter nem deslocado para o núcleo. As informações V20 indicam profundidade de
padrão; não são falhas. Nenhum check global foi desativado. Limitações semânticas
e operacionais do enriquecimento Doxygen continuam as do laudo anterior.
