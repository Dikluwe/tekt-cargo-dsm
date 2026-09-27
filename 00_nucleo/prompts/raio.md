# Prompt de Nucleação: `raio` — o raio de impacto de um nó (domain)
Hash do Código: 3afe071c

**Camada**: L1 — Núcleo. Apenas stdlib. Sem I/O.
**Unidade**: `01_core/core/src/domain/raio.rs` (crate `lente_core`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0002-calculo_raio.md`;
ADR `0002`; spec `forma-organizada.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

O **cálculo central da lente**: o raio de impacto estrutural de um nó-alvo —
"o que quebra se eu mexer aqui?". Sobre `Uses` (consequência), computa montante
(quem sente) e jusante (do que depende), com profundidade; sobre `Owns`
(contenção), expõe pai e filhos como **contexto** (não propagam consequência).

## Comportamento e invariantes

- **`calcular_raio(grafo, alvo) -> Result<Raio, ErroRaio>`**:
  - **BFS** por path na direção `Uses` entrando (montante) e saindo (jusante);
    cada nó com a **menor profundidade** (saltos a partir do alvo); o alvo **não**
    entra em si mesmo (termina com ciclos via visitados).
  - **`ErroRaio::AlvoInexistente(Path)`** se o alvo não está no grafo.
  - **Limite 4 da spec**: arestas `Uses` de `import` saem do módulo, não do item —
    o raio reflete esse piso (não inventa granularidade).
- **`Raio`** — `alvo`, `classificacao`, `uses_entrada`/`uses_saida` (graus
  diretos), `montante`/`jusante` (`HashMap<Path, usize>` profundidade), `owns_pai`/
  `owns_filhos` (contexto, ordenados). Helpers `profundidade_maxima_*`.
- Graus e filhos contam vizinhos distintos por path. Arestas paralelas,
  inclusive `Import` e `Reference` entre os mesmos nós, não duplicam vizinhos.
- **`Classificacao`** (sem thresholds — só zero/não-zero nas duas direções):
  `Isolado` / `Folha` (ninguém depende) / `Base` (dependem, ele não) /
  `Intermediario`.

## Consulta explicativa de caminho mínimo

**`explicar_caminho_impacto(grafo, item_alterado, consumidor)
-> Result<Option<Vec<Path>>, ErroCaminhoImpacto>`** é uma consulta pública aditiva.
`calcular_raio`, `Raio` e `ErroRaio` permanecem com suas assinaturas e resultados.

- A direção retornada é **consumidor → dependências intermediárias → item
  alterado**, com ambos os endpoints incluídos. Cada passo corresponde a `Uses`
  no sentido `from → to`. É alcance estrutural, não prova de quebra comportamental.
- `Some(caminho)` contém um único caminho com o menor número de **arestas**.
  Para endpoints distintos, `caminho.len() - 1` é igual à profundidade do consumidor
  no `montante` de `calcular_raio(grafo, item_alterado)`.
- `Ok(None)` significa endpoints existentes sem caminho nessa direção.
- `ErroCaminhoImpacto::ItemAlteradoInexistente(Path)` e
  `ErroCaminhoImpacto::ConsumidorInexistente(Path)` distinguem ausência de cada
  endpoint. Se ambos faltam, o item alterado tem precedência. O erro implementa
  `Display` e `Error`, sem mudar o enum de erro do raio existente.
- Mesmo endpoint **existente**: `Some(vec![endpoint])`, zero saltos, inclusive
  em ciclos e com autorreferências. É uma consulta reflexiva, não autoimpacto:
  o alvo continua excluído do próprio montante/jusante de `calcular_raio`.
- Empates: menor sequência lexicográfica de paths completos (`Path::Ord`),
  começando pelo consumidor. BFS usa a adjacência `Uses` de saída, ordenada e
  deduplicada. Ordem de nós/arestas e IDs não decide o resultado.
- Termina em ciclos: cada path é visitado uma vez, com um único predecessor.
  Reconstrói apenas o caminho vencedor, sem enumerar todos os caminhos.
- Arestas paralelas não repetem passos. `Owns` não forma caminho nem atalho.
  Todos os subtipos `Uses` do grafo recebido participam (incluindo import,
  reference e ausente), como no raio; escopo/filtros são responsabilidade do
  chamador. Permanece o limite de granularidade dos imports da spec.
- Entrada é a **forma resolvida**, com paths únicos e endpoints íntegros. Não
  resolve colisões, não infere identidade pelos nomes apresentados nem valida
  novamente o grafo. Não muta a entrada.
- Custos: construção dos índices lineares mais ordenação das adjacências;
  busca BFS O(V+E), reconstrução O(comprimento), memória O(V+E), considerando
  operações sobre paths. Só stdlib; sem I/O ou dependências novas.

### Verificação da consulta

```
Dado C Uses B, B Uses A Então consultar(A,C) = [C,B,A], dois saltos
Dado A e C existentes sem cadeia dirigida Então None; endpoint ausente é Err
Dado endpoints iguais existentes Então [endpoint]; mesmo path ausente é Err
Dado diamante com empate Então vence a sequência lexicográfica desde consumidor
Dado caminho direto e indireto Então vence o direto mesmo que ordene depois
Dado ciclo, isolado, paralelas e Owns Então termina, não duplica e ignora Owns
Dado qualquer par distinto alcançável Então len(caminho)-1 = raio.montante[consumidor]
```

## Restrições (L1 puro)

- `calcular_raio` é puro por path; não resolve colisões (o `lente_wiring` resolve
  antes). Sem I/O, sem dep externa.

## Critérios de Verificação

```
Dado B usa A, C usa B Quando calcular_raio(A) Então montante {B:1, C:2}
Dado X,Y,Z usam A, A não usa nada Então classificacao = Base
Dado dois caminhos para n2 (1 e 2 saltos) Então o jusante reporta 1 (mais curto)
Dado alvo inexistente Então Err(AlvoInexistente)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"explicar_caminho_impacto","params":["&Grafo","&Path","&Path"],"return_type":"Result<Option<Vec<Path>>, ErroCaminhoImpacto>"},{"name":"calcular_raio","params":["&Grafo","&Path"],"return_type":"Result<Raio, ErroRaio>"}],"types":[{"name":"Classificacao","kind":"enum","members":["Isolado","Folha","Base","Intermediario"]},{"name":"Raio","kind":"struct","members":["alvo","classificacao","uses_entrada","uses_saida","montante","jusante","owns_pai","owns_filhos"]},{"name":"ErroRaio","kind":"enum","members":["AlvoInexistente"]},{"name":"ErroCaminhoImpacto","kind":"enum","members":["ItemAlteradoInexistente","ConsumidorInexistente"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-09-26 | Consulta aditiva de caminho mínimo `Uses`, determinística desde consumidor, erros por endpoint e política reflexiva; preserva raio e deduplicação existentes. | `01_core/core/src/domain/raio.rs` |
| 2026-06-07 | Nucleação (migração de convenção, prompt 0059) do cálculo do raio. Código inalterado. | `01_core/core/src/domain/raio.rs` |
