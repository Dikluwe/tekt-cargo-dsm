# Validação por propriedades — 2026-09-26

Escopo desta alteração: somente `lab/conformidade-grafos/`. Os crates de produção
e adaptadores não foram editados. Nenhuma dependência nova. A CLI do harness
foi preservada, com o novo módulo carregado apenas em `cfg(test)`.

## Resultado

- **23 testes aprovados**, 0 falhas, 0 ignorados: 11 testes existentes,
  9 propriedades e 3 testes auxiliares (oráculo, cobertura e diagnóstico).
- **576 casos de propriedades**: 9 × 64 sementes; original e transformado de
  cada caso verificados contra o oráculo (1.152 verificações de assinatura,
  incluindo casos em que a transformação é identidade).
- Build aprovado. Clippy com `--all-targets -- -D warnings` aprovado.
- Reprodução exata de `renomeacao_bijetiva` com semente 42: aprovada.
- Todos os testes de propriedades com a semente u64 máxima
  `18446744073709551615`: aprovados (12 testes incluindo auxiliares).
- Smoke da CLI existente: comparação cadeia/cadeia-renomeada com mapa explícito
  aprovada. Não foi modificada a interface da CLI.
- Nenhuma regressão foi encontrada neste corpus. Isso não constitui prova
  universal dos algoritmos nem completude semântica de extratores.

## Cobertura efetiva do corpus v1

64 sementes únicas, 0–63, com todos os tamanhos de 0 a 8 nós. São 42 grafos
com autoarestas, 35 com SCCs não unitários e 10 casos da família com duas
partições de pelo menos dois nós sem arestas Uses cruzadas.

| Transformação | Sementes cujo grafo mudou |
|---|---:|
| Permutação | 46 |
| Renomeação | 56 |
| Duplicação | 51 |
| Isolado | 64 |
| Adição (alcance) | 52 |
| Adição (distâncias) | 52 |
| Remoção | 44 |
| Inversão | 42 |
| Owns | 46 |

Contagens auxiliares em jusante: 34 raios tiveram alcance ampliado pela adição;
54 distâncias já existentes foram reduzidas; 125 raios tiveram alcance reduzido
pela remoção. São contagens de nós/pares atingidos, não sementes distintas.
Não são benchmarks nem estimativas estatísticas de cobertura.

O oráculo usa Floyd–Warshall para distâncias e alcançabilidade mútua para SCCs,
com graus calculados por conjuntos de adjacência. O resultado real usa somente
as APIs públicas de raio, ciclos e ranking. O teste auxiliar do oráculo confere
cadeia, atalho, ciclo e autoaresta contra valores explícitos conhecidos.

## Condições e comandos

Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08)`. Workspace próprio e dependências
locais do checkout compartilhado. `RUSTUP_AUTO_INSTALL=0`; execução offline;
artefatos no `target/` do harness. Sem alteração de código de produção, sem
benchmark, sem limiar de tempo e sem função de explicação de impacto.

```sh
cargo test --offline --locked --manifest-path lab/conformidade-grafos/Cargo.toml
cargo clippy --offline --locked --manifest-path lab/conformidade-grafos/Cargo.toml --all-targets -- -D warnings
cargo build --offline --locked --manifest-path lab/conformidade-grafos/Cargo.toml

LENTE_SEMENTE=42 cargo test --offline --locked \
  --manifest-path lab/conformidade-grafos/Cargo.toml \
  propriedades::renomeacao_bijetiva -- --exact --nocapture

LENTE_SEMENTE=18446744073709551615 cargo test --offline --locked \
  --manifest-path lab/conformidade-grafos/Cargo.toml propriedades:: -- --nocapture

cargo test --offline --locked --manifest-path lab/conformidade-grafos/Cargo.toml \
  propriedades::gerador_v1_e_corpus_cobrem_casos_nao_vazios -- --exact --nocapture
```

Logs: `propriedades-resultados.log`, `reproducao-resultados.log`.
A reprodução pressupõe manter o gerador v1 e as versões dos crates sob teste.
As nove propriedades e seus domínios constam em `CONTRATO.md` e `README.md`.

SHA-256 dos insumos locais da geração nesta entrega:

- `sementes.json`: `df952591fd49b2ed1dcea64229f3ed68b26a3a4c7157a5909464abf6e54d9132`
- `src/propriedades.rs`: `ef11849a70acdc62b2aa57012c7d66a5b07e5acabe3f9d3260389d087c88d4a8`
