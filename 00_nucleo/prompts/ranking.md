# Prompt de Nucleação: `lente_ranking` — top-N por impacto (ranking)
Hash do Código: 3c39bbd3

**Camada**: L1 — Núcleo. Pureza: stdlib + `lente_core`. Zero externas.
**Unidade**: `01_core/ranking/src/lib.rs` (crate `lente_ranking`, arquivo único).
**Origem de trabalho** (referência, não copiada): `00_nucleo/prompt/0027-ranking-top-n.md`,
laudo `00_nucleo/lessons/0027-ranking-top-n.md`.

> Prompt de **nucleação** (descreve o código existente a ponto de ele ser uma
> materialização fiel), criado na migração-piloto de convenção de linhagem
> (00_nucleo/prompt/0058). Não é o prompt de trabalho — aquele fica em `prompt/`.

---

## Propósito

Computar o **top-N por impacto estrutural** de um grafo: para cada nó, o impacto
é o tamanho do seu **montante** (quantos dependem dele, via `lente_core::calcular_raio`);
ordena-se decrescente por impacto e corta-se em N. Reusa o cálculo do `lente_core`
— não duplica a indexação do grafo.

O valor analítico está em achar os nós **`Base`** (têm quem dependa deles, não
dependem de ninguém): são o que o top-N busca. Por isso o item carrega a
`Classificacao` do raio, além do impacto.

## Comportamento e invariantes

- **`ItemRanking`** — um item do ranking: `path` do nó, `impacto`
  (`= raio.montante.len()`) e a `classificacao` do `Raio`.
- **`rankear(grafo, n)`** devolve `Vec<ItemRanking>`:
  - **Ordem determinística**: decrescente por `impacto`; desempate **ascendente
    por path**.
  - **Corte em N**: se `n ≥ nº de nós`, devolve todos.
  - **Não filtra**: nós sem montante (`Folha`/`Isolado`) entram com `impacto = 0`
    — "top-N" é ordenação + corte, não seleção semântica; o consumidor decide.
  - **Um item por path**: paths repetidos (colisão não resolvida) são processados
    uma vez (`calcular_raio` é por path; o `lente_wiring` resolve colisões antes —
    este L1 não).
  - Nó cujo `calcular_raio` falha (alvo inexistente, impossível aqui) é pulado.
- **Pureza**: sem I/O, sem dependência externa; só `lente_core` (intra-L1).

## Restrições (L1 puro)

- `#![forbid(unsafe_code)]`.
- Nenhuma dependência externa (nem `serde`) — só stdlib + `lente_core`.
- `ItemRanking` é `Debug + Clone + PartialEq + Eq` — testável sem mocks.

## Critérios de Verificação

```
Dado um grafo onde X,Y,Z dependem de A (A é Base) e n=10
Quando rankear(grafo, 10)
Então A vem primeiro (impacto 3), e a ordem é determinística

Dado dois nós de mesmo impacto
Quando rankeados
Então o desempate é por path ascendente

Dado n menor que o número de nós
Quando rankear(grafo, n)
Então o resultado tem exatamente n itens (corte)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"rankear","params":["&Grafo","usize"],"return_type":"Vec<ItemRanking>"}],"types":[{"name":"ItemRanking","kind":"struct","members":["path","impacto","classificacao"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (piloto da migração de convenção, prompt 0058): descreve o `lente_ranking` existente (do trabalho 0027) no formato Cristalino, com Interface Snapshot para o V6. Código inalterado — só cabeçalho + este prompt. | `01_core/ranking/src/lib.rs` |
