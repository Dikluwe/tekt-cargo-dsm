# Resultados da tarefa do relatório documental

Data: 2026-09-26. Área exclusiva: lab/doxygen-prioridade/.
O diretório não existia no início. Nenhuma alteração preexistente foi substituída.
Contrato local registrado antes do código; cabeçalho aponta para CONTRATO.md.

## Execução verificada

Ambiente Linux com Rust/Cargo já disponíveis. Dependências fixadas no
Cargo.toml local e resolvidas offline para Cargo.lock próprio. CARGO_TARGET_DIR
foi definido explicitamente para lab/doxygen-prioridade/target; RUSTUP_AUTO_INSTALL=0.
Não houve instalação, download, execução de Doxygen ou execução de código das
outras tarefas. Nenhum crate de produção é dependência deste workspace.

- `cargo check --offline --manifest-path lab/doxygen-prioridade/Cargo.toml`: passou.
- `cargo test --offline --locked --manifest-path lab/doxygen-prioridade/Cargo.toml`:
  18 testes passaram; 0 falhas; 0 ignorados.
- `cargo build --offline --locked --manifest-path lab/doxygen-prioridade/Cargo.toml`:
  passou; binário lente-doxygen-prioridade disponível no target local.
- `cargo clippy --offline --locked --manifest-path lab/doxygen-prioridade/Cargo.toml
  --all-targets -- -D warnings`: passou, sem avisos.
- `cargo fmt --manifest-path lab/doxygen-prioridade/Cargo.toml -- --check`: passou.

As verificações de CLI real usaram diretório temporário exclusivo dentro de target,
removido depois. Foram aprovados: sucesso com --summary; execução sem --summary
com stdout vazio e bytes de JSON idênticos; inventário com espaço adicional
recusado por digest divergente sem criar relatório; saída existente recusada.

## Cobertura dos testes inline

1. Alto impacto associado sem texto versus alto impacto sem associação.
2. Texto documental observado com baixo impacto e sem score extra.
3. Associação ambígua, candidatos preservados sem eleger documento.
4. Empate por node_id com posição justificada.
5. Métrica ausente por item omitido, valor omitido/null, distinta de zero real.
6. Coverage partial/unknown/complete e diagnósticos preservados.
7. Digests dos bytes exatos e vínculos divergentes.
8. Vetor conhecido SHA-256 de abc.
9. Schemas/versões/cobertura incompatíveis.
10. IDs repetidos em símbolos, catálogo, matches, candidatos, métricas e listas.
11. Referências inválidas em relações, candidatos, documentos, diagnósticos e nós.
12. Uma entrada por nó e coerência de status/unmatched.
13. Métricas negativas/fracionárias/strings e linhas zero recusadas.
14. Opcionais ausentes, caminhos/textos literais e controles no resumo.
15. Repetição determinística e permutação com digests corretamente diferentes.
16. CLI com JSON/resumo e proteção contra sobrescrita.
17. CLI não publica em erro e não sobrescreve entrada.
18. CLI rejeita opções ausentes/desconhecidas/repetidas.

## Arquivos da entrega

- Cargo.toml e Cargo.lock: workspace isolado e dependências fixadas.
- .gitignore: target/out locais.
- CONTRATO.md: contrato causal anterior à implementação.
- src/main.rs: implementação e testes inline.
- fixtures/inventory.json, catalog.json, matches.json e impact.json: entradas
  sintéticas com vínculos válidos por bytes exatos.
- README.md: instalação offline, execução, formato, tabela da fixture e limites.
- RESULTADOS.md: este registro.

O relatório prioriza revisão humana. Não mede correção, qualidade arquitetural ou
qualidade efetiva da documentação. Não foi integrado ao produto, não alterou
regras arquiteturais/CI e não executou testes do workspace principal.
