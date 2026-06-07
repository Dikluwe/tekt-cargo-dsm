# Prompt de Nucleação: `raio` — o raio de impacto de um nó (domain)
Hash do Código: 88ebd575

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
- **`Classificacao`** (sem thresholds — só zero/não-zero nas duas direções):
  `Isolado` / `Folha` (ninguém depende) / `Base` (dependem, ele não) /
  `Intermediario`.

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
<!-- crystalline-snapshot: {"functions":[{"name":"calcular_raio","params":["&Grafo","&Path"],"return_type":"Result<Raio, ErroRaio>"}],"types":[{"name":"Classificacao","kind":"enum","members":["Isolado","Folha","Base","Intermediario"]},{"name":"Raio","kind":"struct","members":["alvo","classificacao","uses_entrada","uses_saida","montante","jusante","owns_pai","owns_filhos"]},{"name":"ErroRaio","kind":"enum","members":["AlvoInexistente"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0059) do cálculo do raio. Código inalterado. | `01_core/core/src/domain/raio.rs` |
