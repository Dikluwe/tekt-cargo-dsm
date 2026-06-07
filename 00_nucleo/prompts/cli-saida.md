# Prompt de Nucleação: `lente_cli::saida` — os formatadores de saída (L2)
Hash do Código: de72cbe6

**Camada**: L2 — Casca (apresentação pura). Os literais vêm do `lente_catalogo`
(ADR-0002).
**Unidade**: `02_shell/cli/src/saida.rs` (crate `lente_cli`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0020-l2-cli.md` (+ ranking
0027, estrutura 0031/0035, diff 0047, vistas 0048).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

Mapear os **tipos de resultado L1** para **texto humano** ou **JSON** — a saída que
o usuário lê. Apresentação pura: opera só sobre os tipos do `lente_core`/
`lente_estrutura`/`lente_ranking` (re-apontados nos Estágios 1/2 do refactor) e o
catálogo L2. **Não importa o L4** (`lente_wiring`) — a inversão foi fechada no
refactor 0055–0057.

## Comportamento e invariantes

- **`formatar(raio, alvo, escopo, modo)`** — o per-nó (texto ou JSON; `--verbose`
  inclui os impactados).
- **`formatar_ranking(itens, escopo, modo)`** — o top-N.
- **`formatar_estrutura(estrut, escopo, modo_uses, modo)`** — módulos/dependências/
  ciclos + ordem da DSM.
- **`formatar_diff(resultado)`** — o JSON view-agnóstico do `--diff` (0047).
- **`formatar_diff_vista(resultado, vista)`** — as três vistas de texto (0048):
  `resumo` (impacto por crate, ênfase adaptativa), `item` (bloco por tocado),
  `camadas` (agrupado por crate).
- **`Modo`** (`text`/`verbose`), **`AlvoPedido`** (`Path`/`Id` — mostra a tradução
  id→path quando o usuário pediu por id).
- **JSON montado à mão** (`serde_json::Map`), chaves do catálogo; texto com rótulos
  do catálogo. **Determinístico** (ordena o que itera).

## Restrições (L2 — apresentação)

- Importa só L1 (`lente_core`/`lente_estrutura`/`lente_ranking`) + `lente_catalogo`
  + `serde_json`/`clap`. **Sem `lente_wiring` (L4)** — V3 = 0.

## Critérios de Verificação

```
Dado um Raio Quando formatar (JSON) Então {alvo, classificacao, diretos, transitivos, escopo}
Dado um ResultadoDiff Quando formatar_diff_vista(Resumo) Então contagem por crate + censo + solto
Dado diff só-arquivo-novo Então a vista resumo lidera com o jusante (ênfase adaptativa)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"formatar","params":["&Raio","&AlvoPedido","Escopo","&Modo"],"return_type":"String"},{"name":"formatar_ranking","params":["&[ItemRanking]","Escopo","&Modo"],"return_type":"String"},{"name":"formatar_estrutura","params":["&EstruturaModulos","Escopo","ModoUses","&Modo"],"return_type":"String"},{"name":"formatar_diff","params":["&ResultadoDiff"],"return_type":"String"},{"name":"formatar_diff_vista","params":["&ResultadoDiff","Vista"],"return_type":"String"}],"types":[{"name":"AlvoPedido","kind":"enum","members":["Path","Id"]},{"name":"Modo","kind":"struct","members":["text","verbose"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0063) dos formatadores. Código inalterado. | `02_shell/cli/src/saida.rs` |
