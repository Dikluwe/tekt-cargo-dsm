# Prompt de Nucleação: `lente_infra::invocacao` — invocar o fork num diretório (L3, interno)
Hash do Código: a21d1a95

**Camada**: L3 — Infraestrutura. Subprocesso é I/O legítimo.
**Unidade**: `03_infra/src/invocacao.rs` (módulo **interno** `pub(crate)` do `lente_infra`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0003-adaptador_l3.md`.

> Prompt de **nucleação** (interno; fica no walk — regra do 0062).

---

## Propósito

Rodar o fork **num diretório-alvo dado** (o crate a analisar): detecta o alvo via
`metadata` a partir do `manifest_path` e delega à primitiva única `fork::invocar_em`.
É o caminho usado pelo `extrair_grafo(caminho)` (fachada), distinto do `invocar_fork`
(que herda o cwd).

## Comportamento e invariantes

- **`invocar(caminho_crate) -> Result<String, ErroFork>`** (`pub(crate)`) —
  descobre o alvo do crate em `caminho` e chama `fork::invocar_em` com esse
  `current_dir` e alvo. Não duplica a montagem do `Command` (reusa a primitiva).
- Erros propagam como `ErroFork` (incl. detecção de alvo).

## Restrições (L3)

- I/O legítimo (subprocesso via a primitiva do `fork`). `pub(crate)`: não escapa.
  **Não importa o L4**.

## Critérios de Verificação

```
Dado o diretório de um crate Quando invocar Então o JSON do fork daquele crate
Dado um caminho sem Cargo.toml resolvível Então Err(ErroFork::DeteccaoAlvo)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"invocar","params":["&Path"],"return_type":"Result<String, ErroAdaptador>"}],"types":[],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) da invocação por diretório. Código inalterado. | `03_infra/src/invocacao.rs` |
