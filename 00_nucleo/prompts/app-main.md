# Prompt de Nucleação: `lente_app::main` — o ponto de entrada (L4)
Hash do Código: 4805b2ee

**Camada**: L4 — Fiação (app/ponto de composição). Importa L1/L2/L4.
**Unidade**: `04_wiring/app/src/main.rs` (crate `lente_app`, o binário `lente`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0020-l2-cli.md` (o `main`
veio da CLI); relocado ao L4 no Estágio 3 (prompt 0057).

> Prompt de **nucleação** (lógica ativa de dispatch — fica no walk; `main`/`run`
> não são `pub`, então o snapshot é mínimo).

---

## Propósito

O **ponto de composição**: o binário `lente`. Parseia os argumentos (`lente_cli::
args`, L2), chama a orquestração (`lente_wiring`, L4), trata o `ErroLente`, e chama
os formatadores (`lente_cli::saida`, L2) — devolvendo a saída no canal certo
(stdout/stderr) e o código de saída. É o que **tira a apresentação (L2) de depender
do L4** (a CLI virou lib pura no 0057).

## Comportamento e invariantes

- **`main()`** — `Cli::parse()` → `run(cli)` → imprime o `Ok(String)` (stdout,
  exit 0) ou a mensagem de erro (stderr, exit code).
- **`run(cli)`** — roteia entre os modos (diff / estrutura / ranking / per-nó),
  monta `FonteGrafo`/`AlvoBusca`/`Escopo`/`ModoUses` (do L1) das flags, chama a
  orquestração e formata; erro do wiring → `SaidaErro` via `erro::traduzir`.
- **`SaidaErro`** — código (1=pipeline, 2=args) + mensagem já traduzida.
- **L4 importa L1/L2/L4** (composição — para baixo, permitido). A apresentação
  (args/saida) e o catálogo são L2; a orquestração é L4; o vocabulário é L1.

## Restrições (L4)

- É o topo: compõe as camadas abaixo. Não recria lógica de domínio nem formata —
  delega. O `main` não é `pub` (binário).

## Critérios de Verificação

```
Dado --grafo <json> --alvo <path> Quando run Então o JSON/texto do raio
Dado --diff --vista resumo Quando run Então a vista resumo
Dado um erro do pipeline Quando run Então SaidaErro com mensagem do catálogo
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"SaidaErro","kind":"struct","members":["codigo","mensagem"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0065) do ponto de entrada. Código inalterado. | `04_wiring/app/src/main.rs` |
