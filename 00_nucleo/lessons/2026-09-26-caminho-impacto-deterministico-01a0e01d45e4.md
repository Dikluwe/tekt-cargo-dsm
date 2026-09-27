# Laudo — caminho mínimo determinístico de impacto

Data: 2026-09-26. Identificador exclusivo: `caminho-impacto-01a0e01d45e4`.

## Escopo e preservação

Mudança independente restrita a `01_core/core/src/domain/raio.rs`,
`00_nucleo/prompts/raio.md` e este laudo. As alterações locais preexistentes de
ordenação/deduplicação dos índices e seu teste de arestas paralelas foram
preservadas. Nenhum consumidor, adaptador, renderizador ou harness foi alterado.

## Interface e decisões

Nova API pública no módulo `domain::raio`:

```rust
pub fn explicar_caminho_impacto(
    grafo: &Grafo,
    item_alterado: &Path,
    consumidor: &Path,
) -> Result<Option<Vec<Path>>, ErroCaminhoImpacto>
```

A sequência inclui as pontas, na direção **consumidor → dependências → item
alterado**. Para endpoints distintos, o número de arestas (`len - 1`) coincide
com a profundidade mínima do consumidor no montante do item alterado.

`None` significa endpoints existentes sem cadeia nessa direção. O erro novo
separa `ItemAlteradoInexistente` e `ConsumidorInexistente`; quando ambos faltam,
o item alterado tem precedência. O mesmo nó existente retorna `[nó]`, zero
arestas. É reflexividade da consulta, não autoimpacto: o raio continua excluindo
o alvo do próprio montante/jusante, inclusive em ciclos.

BFS percorre `uses_saida` já ordenado por path e deduplicado. Cada path tem um
único predecessor; a primeira descoberta escolhe a sequência lexicograficamente
menor entre as de menor comprimento, desde o consumidor. Não enumera todos os
caminhos nem percorre recursivamente ciclos. `Owns` não cria passos/atalhos.
Todos os subtipos de `Uses` recebidos participam, como no cálculo existente.

O grafo precisa obedecer à forma resolvida (paths únicos, endpoints íntegros).
O resultado explica alcance estrutural, **não prova quebra comportamental**.
Não promete granularidade maior que a extração; imports continuam sujeitos ao
Limite 4 da spec. Sem I/O, serialização ou dependências novas.

A assinatura e o corpo de `calcular_raio` foram comparados textualmente com a
versão em HEAD e permanecem idênticos. `Raio` e `ErroRaio` não mudaram.

## Verificação executada

- `cargo test --offline -p lente_core explicacao --target-dir /tmp/lente-caminho-impacto-target`:
  **10 testes novos passaram**, zero falhas.
- `cargo test --offline -p lente_core --target-dir /tmp/lente-caminho-impacto-target`:
  **72 testes passaram**, zero falhas/ignorados; doc-tests também passaram.
- Casos: cadeia e direção inversa; diamante com permutação de ordem e IDs;
  desempate lexicográfico pela sequência desde consumidor; caminho direto versus
  indireto; ciclo alcançável e inalcançável; isolado; ausência de cada endpoint e
  ambos; mesmo endpoint isolado/com ciclo/self-loop; paralelas Import/Reference;
  Owns sozinho e como atalho que deve ser ignorado.
- Matriz dos **36 pares** de um grafo de seis nós: presença/comprimento de caminho
  comparados às profundidades de `calcular_raio`, com exceção reflexiva explícita.
  Os testes de exemplos também verificam direção de cada passo e imutabilidade.
- `cargo tree --offline -p lente_core --edges normal`: somente `lente_core`,
  sem dependências de terceiros em produção.
- Snapshot e hashes gerados com `crystalline-lint --update-snapshot` e
  `--fix-hashes` numa cópia temporária contendo apenas a unidade e seu contrato;
  zero stale/drift nessa unidade. Nenhuma atualização global de linhagem.
- `git diff --check` nos arquivos de implementação/contrato: sem erros.

Não se declara validação de CLI/MCP, extração real ou mudanças no pipeline:
a entrega é uma API pura aditiva com testes inline. A suíte do workspace inteiro
não foi necessária para essa mudança restrita; foi executada a suíte do crate.
