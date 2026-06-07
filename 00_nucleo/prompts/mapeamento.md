# Prompt de Nucleação: `mapeamento` — o núcleo do modo `--diff` (domain)
Hash do Código: 4e15905a

**Camada**: L1 — Núcleo. Apenas stdlib. Sem I/O. Sem deps externas.
**Unidade**: `01_core/core/src/domain/mapeamento.rs` (crate `lente_core`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0046-ler_diff_e_mapear.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

O **núcleo do modo `--diff`**: mapeia um diff estruturado aos nós do grafo que
ele toca, e faz o **censo do untracked** (ligado / solto / não-fonte). A **forma**
do diff mora aqui (dado puro, como o `Grafo`) para `mapear_diff` não importar o L3
(o **leitor** `ler_diff` é L3).

## Comportamento e invariantes

- **Forma do diff**: `OrigemArquivo` (`Rastreado`/`NaoRastreado`), `FaixaLinhas`
  (`inicio`/`fim`, 1-based inclusiva, lado novo), `ArquivoDiff` (`caminho`,
  `origem`, `linhas_alteradas`), `DiffEstruturado` (`arquivos`).
- **`mapear_diff(diff, grafo, membros_dirs) -> MapeamentoDiff`** (puro, determinístico):
  - **`tocados`**: nós cuja `position.file` **casa** o caminho (igualdade ou
    sufixo em **fronteira de segmento** — reconcilia relativo↔absoluto, laudo
    0038) **e** cuja faixa de linhas **cruza** uma `FaixaLinhas`; dedup por id;
    pega o item **e** o módulo-arquivo que o abrange.
  - **Censo do untracked** (laudo 0043): `ligados` (caminho está no grafo →
    compilado), `soltos` (`.rs` em `membros_dirs` mas fora do grafo → presente,
    não compilado), `nao_fonte` (fora de membro ou não-`.rs`).
  - Saídas ordenadas (determinístico).
- **`NoTocado`** (`id` + `path`), **`MapeamentoDiff`** (`tocados`/`ligados`/
  `soltos`/`nao_fonte`).

## Restrições (L1 puro)

- Só stdlib (`std::path`, `BTreeSet`); sem I/O; `mapear_diff` não importa o L3.

## Critérios de Verificação

```
Dado A::foo em position 10..20 e diff altera 12..14 Então A::foo está em tocados
Dado caminho relativo do diff e position absoluta Então casam (reconciliação)
Dado untracked .rs em membro fora do grafo Então solto; fora de membro Então não-fonte
Dado o mesmo diff/grafo 2× Então MapeamentoDiff igual
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"mapear_diff","params":["&DiffEstruturado","&Grafo","&[PathBuf]"],"return_type":"MapeamentoDiff"}],"types":[{"name":"OrigemArquivo","kind":"enum","members":["Rastreado","NaoRastreado"]},{"name":"FaixaLinhas","kind":"struct","members":["inicio","fim"]},{"name":"ArquivoDiff","kind":"struct","members":["caminho","origem","linhas_alteradas"]},{"name":"DiffEstruturado","kind":"struct","members":["arquivos"]},{"name":"NoTocado","kind":"struct","members":["id","path"]},{"name":"MapeamentoDiff","kind":"struct","members":["tocados","ligados","soltos","nao_fonte"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0059) do núcleo do `--diff`. Código inalterado. | `01_core/core/src/domain/mapeamento.rs` |
