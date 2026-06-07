# Prompt de Nucleação: `lente_infra::fork` — o invocador do fork (L3)
Hash do Código: 85066a4a

**Camada**: L3 — Infraestrutura. Subprocesso externo é I/O legítimo.
**Unidade**: `03_infra/src/fork.rs` (crate `lente_infra`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0017-l3_invocador_fork.md`
(+ 0022/0023); ADR `0001`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **primitiva única** que invoca o fork do `cargo-modules` — `cargo modules
export-json --sysroot --compact [--lib|--bin] --package <p>` — como subprocesso e
devolve o JSON cru. Não desserializa (isso é a `traducao`). Única boca do `cargo`
para o fork no crate (laudo 0018).

## Comportamento e invariantes

- **`invocar_fork(pacote) -> Result<String, ErroFork>`** — detecta o alvo
  (`--lib`/`--bin`) por `cargo metadata` e roda o fork no cwd.
- **`invocar_em(pacote, dir, alvo)`** (`pub(crate)`) — a primitiva: monta o
  `Command`, captura stdout, mapeia status/UTF-8 a `ErroFork`.
- **`AlvoFork`** (`pub(crate)`: `Lib`/`Bin(nome)`) — a flag de alvo.
- **`ErroFork`** — `FalhaSubprocess`/`StatusErro{codigo,stderr}`/`StdoutInvalido`/
  `DeteccaoAlvo`. `Display`+`Error`. (É um dos erros que o `ErroLente` do L4 agrega.)
- **Limite 1 da spec**: `--sysroot` é **fixo** (política do projeto), não opção.

## Restrições (L3)

- I/O legítimo (subprocesso do cargo). Importa só o que precisa do crate; **não
  importa o L4**.

## Critérios de Verificação

```
Dado um pacote válido Quando invocar_fork Então JSON do fork (com o crate-raiz)
Dado pacote inexistente Então Err(DeteccaoAlvo::PacoteNaoEncontrado) antes do fork
Dado cada variante de ErroFork Então Display não-vazio
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"invocar_fork","params":["&str"],"return_type":"Result<String, ErroFork>"},{"name":"invocar_em","params":["&str","Option<&Path>","Option<&AlvoFork>"],"return_type":"Result<String, ErroFork>"}],"types":[{"name":"AlvoFork","kind":"enum","members":["Lib","Bin"]},{"name":"ErroFork","kind":"enum","members":["FalhaSubprocess","StatusErro","StdoutInvalido","DeteccaoAlvo"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) do invocador do fork. Código inalterado. | `03_infra/src/fork.rs` |
