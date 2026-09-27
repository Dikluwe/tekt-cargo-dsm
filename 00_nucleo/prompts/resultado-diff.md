# Prompt de Nucleação: `resultado_diff` — o resultado view-agnóstico do `--diff` (domain)
Hash do Código: 1a7dcf77

**Camada**: L1 — Núcleo. Apenas stdlib. Sem I/O. **Sem `serde`.**
**Unidade**: `01_core/core/src/domain/resultado_diff.rs` (crate `lente_core`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0047-resultado_diff_orquestracao_json.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

O **resultado view-agnóstico** do modo `--diff`: um dado completo que carrega tudo
que as vistas (resumo / item / camadas) precisam. A serialização JSON **não** mora
aqui (é L2) — manter `serde` fora preserva a pureza L1.

## Comportamento e invariantes

- `FalhaCrate { crate_name, motivo }` vive em L1, apenas strings, sem tipos de infraestrutura. O nome identifica o membro Cargo que falhou.
- `ResultadoDiff.falhas` preserva cada falha de extração/resolução do workspace. `completude()` retorna o enum `CompletudeDiff::{Completa, Parcial}`, derivado da lista vazia/não vazia; não há estado redundante que possa contradizer as falhas.
- Completa significa todos os membros extraídos e resolvidos; permanecem os limites estruturais da spec. Falta total de crates é erro do pipeline L4, não ResultadoDiff vazio bem-sucedido.

- **`ResultadoDiff`** — `tocados` (cada nó tocado + seu raio), `combinado` (a união
  dos raios, para a vista resumo), o censo do untracked (`ligados`/`soltos`/
  `nao_fonte`) e os `fantasmas` (do grafo de workspace).
- **`TocadoComRaio`** (`tocado: NoTocado` + `raio: Raio`).
- **`RaioCombinado`** (`montante`/`jusante`: `Vec<(Path, usize)>`).
- **`combinar_raios(&[Raio]) -> RaioCombinado`** (puro): une os `montante`/`jusante`
  por path com a **profundidade mínima** (o mais próximo), ordenado por path
  (determinístico via `BTreeMap`).

## Restrições (L1 puro)

- **Sem `serde`** nem dep externa — só stdlib. O JSON é do L2 (mapeia este tipo).

## Critérios de Verificação

```
Dado raios com X(1),Y(2) e Y(3),Z(1) Quando combinar_raios Então X1,Y2,Z1 (mínimo, ordenado)
Dado raios vazios Então combinado vazio
Dado os mesmos raios 2× Então combinado igual (determinístico)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"combinar_raios","params":["&[Raio]"],"return_type":"RaioCombinado"}],"types":[{"name":"TocadoComRaio","kind":"struct","members":["tocado","raio"]},{"name":"RaioCombinado","kind":"struct","members":["montante","jusante"]},{"name":"FalhaCrate","kind":"struct","members":["crate_name","motivo"]},{"name":"CompletudeDiff","kind":"enum","members":["Completa","Parcial"]},{"name":"ResultadoDiff","kind":"struct","members":["falhas","tocados","combinado","ligados","soltos","nao_fonte","fantasmas"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0059) do resultado do `--diff`. Código inalterado. | `01_core/core/src/domain/resultado_diff.rs` |

| 2026-09-26 | Correção da perda de falhas no diff: completude explícita, resultados parciais consultáveis e erro se nenhum crate for analisado. | Unidade deste contrato |
