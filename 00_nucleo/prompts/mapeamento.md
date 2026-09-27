# Prompt de Nucleação: `mapeamento` — o núcleo do modo `--diff` (domain)
Hash do Código: a1f88e77

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

## API aditiva por inventário

`mapear_diff_com_inventario(diff, grafo, inventario)` retorna
`Result<MapeamentoInventario, Vec<ErroInventario>>`. A API Rust anterior e
seu comportamento (inclusive censo por `.rs` e reconciliação por sufixo)
permanecem inalterados. Não há integração automática no wiring.

- `ArquivoInventario` fornece caminho e `CoberturaArquivo`: `Analisado`
  com associações, ou `FonteNaoAnalisada`. Um analisado pode não ter nós.
- `AssociacaoArquivoNo` fornece ID e `EscopoAssociacao`: `ArquivoInteiro`
  ou `Linhas { inicio, fim }`, inclusivas, 1-based. Relação muitos-para-muitos;
  a posição de declaração do nó não participa da nova API.
- Qualquer faixa válida não vazia toca a associação de arquivo inteiro;
  uma associação de linhas exige interseção inclusiva. `EvidenciaToque`
  preserva caminho, ID e escopo da associação (não é a faixa alterada).
- `tocados` deduplica IDs e ordena por path/ID; `evidencias` deduplica e
  ordena por caminho/ID/escopo (arquivo inteiro antes de linhas, depois
  início/fim). O censo classifica **todos** os caminhos do diff, rastreados
  ou novos, em `analisados`, `fontes_nao_analisadas`, `fora_do_inventario`.
  Cada lista é ordenada e deduplicada. Ausência do inventário significa
  cobertura desconhecida, nunca irrelevância inferida pela extensão.
- Caminhos são chaves `PathBuf` já normalizadas pelo chamador. Relativos
  exigem a mesma base lógica; absolutos exigem igualdade de caminho absoluto.
  Não há casamento relativo/absoluto nem por sufixo, canonicalização, acesso
  ao filesystem, resolução de links ou conversão de caixa. Normalização e
  convenções da plataforma são responsabilidade do chamador.
- Valida todo o inventário e o diff antes de mapear: ID ausente, início zero,
  fim menor que início e caminhos duplicados no inventário são erros tipados,
  ordenados e deduplicados, com caminho e contexto. Duplicar associações ou
  entradas do diff é permitido. IDs do grafo obedecem à unicidade da spec.
- Lista de faixas vazia é válida e só participa do censo, mesmo em associação
  de arquivo inteiro. Remoções continuam fora do escopo.
- Toda a interface auxiliar vive neste componente; nenhuma dependência externa.

Verificação: C, Python, TypeScript, Zig e arquivo sem extensão; múltiplos
arquivos por pacote e múltiplos nós por arquivo; limites inclusivos; caminhos
homônimos; cobertura ausente/não analisada; erros inclusive em entradas não
alteradas; deduplicação e invariância à permutação; regressões Rust existentes.

## Restrições (L1 puro)

- Só stdlib (`std::path`, `BTreeMap`, `BTreeSet`); sem I/O; `mapear_diff` não importa o L3.

## Critérios de Verificação

```
Dado A::foo em position 10..20 e diff altera 12..14 Então A::foo está em tocados
Dado caminho relativo do diff e position absoluta Então casam (reconciliação)
Dado untracked .rs em membro fora do grafo Então solto; fora de membro Então não-fonte
Dado o mesmo diff/grafo 2× Então MapeamentoDiff igual
```

Revisão 2026-09-27: saneamento mecânico de Clippy/rustfmt, preservando
semântica e API; qualificação Option::None não é um braço curinga.

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"mapear_diff_com_inventario","params":["&DiffEstruturado","&Grafo","&[ArquivoInventario]"],"return_type":"Result<MapeamentoInventario, Vec<ErroInventario>>"},{"name":"mapear_diff","params":["&DiffEstruturado","&Grafo","&[PathBuf]"],"return_type":"MapeamentoDiff"}],"types":[{"name":"OrigemArquivo","kind":"enum","members":["Rastreado","NaoRastreado"]},{"name":"FaixaLinhas","kind":"struct","members":["inicio","fim"]},{"name":"ArquivoDiff","kind":"struct","members":["caminho","origem","linhas_alteradas"]},{"name":"DiffEstruturado","kind":"struct","members":["arquivos"]},{"name":"NoTocado","kind":"struct","members":["id","path"]},{"name":"MapeamentoDiff","kind":"struct","members":["tocados","ligados","soltos","nao_fonte"]},{"name":"EscopoAssociacao","kind":"enum","members":["ArquivoInteiro","Linhas"]},{"name":"AssociacaoArquivoNo","kind":"struct","members":["id","escopo"]},{"name":"CoberturaArquivo","kind":"enum","members":["Analisado","FonteNaoAnalisada"]},{"name":"ArquivoInventario","kind":"struct","members":["caminho","cobertura"]},{"name":"EvidenciaToque","kind":"struct","members":["caminho","id","escopo"]},{"name":"MapeamentoInventario","kind":"struct","members":["tocados","evidencias","analisados","fontes_nao_analisadas","fora_do_inventario"]},{"name":"ErroInventario","kind":"enum","members":["CaminhoDuplicado","IdInexistente","IntervaloAssociacaoInvalido","FaixaDiffInvalida"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0059) do núcleo do `--diff`. Código inalterado. | `01_core/core/src/domain/mapeamento.rs` |

| 2026-09-26 | API pura aditiva por inventário, evidências de granularidade e validação explícita. | `01_core/core/src/domain/mapeamento.rs` |
