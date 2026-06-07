# Prompt de Nucleação: `lente_infra::diff` — ler o diff (L3)
Hash do Código: cb5f85c2

**Camada**: L3 — Infraestrutura. Subprocesso `git` é I/O legítimo.
**Unidade**: `03_infra/src/diff.rs` (crate `lente_infra`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0046-ler_diff_e_mapear.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **entrada** do modo `--diff`: lê o diff do repositório e o estrutura na forma de
dados L1 (`DiffEstruturado`, de `lente_core`). Cobre rastreados e untracked.

## Comportamento e invariantes

- **`ler_diff(raiz) -> Result<DiffEstruturado, ErroDiff>`**:
  - **Rastreados**: `git diff HEAD --unified=0`, parseado por `parse_diff` (pura) em
    faixas do **lado novo**.
  - **Untracked**: `git ls-files --others --exclude-standard`; hunk sintético "tudo
    adicionado" (`{1, n_linhas}`, contagem por bytes).
  - Caminhos normalizados para **absolutos** (casam `No.position.file`).
- **`invocar_git(args, dir)`** (privado) — **primitiva única de git** (o `git` é
  outra ferramenta, distinta do invariante "dois cargo").
- **`parse_diff`** (privado, puro): faixas do lado novo; **deleção pura não gera
  faixa** (limitação documentada).
- **`ErroDiff`** — `Git{codigo,stderr}`/`Parse`/`Io`. `Display`+`Error` (com
  `source`). Erro agregado pelo `ErroLente` (L4).

## Restrições (L3)

- I/O legítimo (`git`). Importa o L1 (a forma do diff mora lá); **não o L4**.

## Critérios de Verificação

```
Dado um diff unificado com +linhas Quando parse_diff Então as faixas do lado novo
Dado um hunk só de deleção (+c,0) Então não gera faixa
Dado um repo com rastreado+untracked Quando ler_diff Então os dois (caminhos absolutos)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"ler_diff","params":["&Path"],"return_type":"Result<DiffEstruturado, ErroDiff>"}],"types":[{"name":"ErroDiff","kind":"enum","members":["Git","Parse","Io"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) do leitor de diff. Código inalterado. | `03_infra/src/diff.rs` |
