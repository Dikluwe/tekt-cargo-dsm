# Prompt de Nucleação: `lente_cli::args` — os argumentos da CLI (L2)
Hash do Código: 81615e0c

**Camada**: L2 — Casca (apresentação pura). Os textos de ajuda vêm do
`lente_catalogo` (ADR-0002).
**Unidade**: `02_shell/cli/src/args.rs` (crate `lente_cli`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0020-l2-cli.md` (+ os modos
0027/0030/0031/0034/0047/0048).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **definição declarativa** (clap `derive`) dos argumentos do binário `lente`:
qual modo o usuário pede e com quais opções. Só **estrutura de argumentos** — a
composição (chamar a orquestração) vive no `lente_app` (L4), e a apresentação dos
resultados no `saida`.

## Comportamento e invariantes

- **`Cli`** (`#[derive(Parser)]`) — as flags: fonte (`--grafo`/`--pacote`,
  mutuamente exclusivas), alvo (`--alvo`/`--alvo-id`), modos (`--ranking`/`--top`,
  `--estrutura`/`--so-referencia`, `--diff`/`--repo`/`--vista`), escopo
  (`--filtrar-stdlib`), saída (`--text`/`--verbose`). Os modos são mutuamente
  exclusivos via `conflicts_with_all`; `--vista` `requires --diff`.
- **`Vista`** (`#[derive(ValueEnum)]`) — `Resumo`/`Item`/`Camadas` (do `--diff`,
  prompt 0048).
- **Todos os `help`/`about`** vêm do `lente_catalogo` — **nenhum literal de
  apresentação** aqui (ADR-0002).

## Restrições (L2 — apresentação)

- Importa só `clap` + `lente_catalogo`. Não importa L1/L3/L4 (a estrutura de args é
  independente da orquestração).

## Critérios de Verificação

```
Dado --grafo e --pacote juntos Então clap rejeita (conflicts_with)
Dado --vista sem --diff Então clap rejeita (requires)
Dado --vista camadas Então Vista::Camadas
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"Vista","kind":"enum","members":["Resumo","Item","Camadas"]},{"name":"Cli","kind":"struct","members":["grafo","pacote","alvo","alvo_id","ranking","top","estrutura","diff","repo","vista","so_referencia","filtrar_stdlib","text","html","saida","verbose"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0063) dos argumentos. Código inalterado. | `02_shell/cli/src/args.rs` |
