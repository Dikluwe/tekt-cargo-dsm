# Medição de Prevalência de Colisões de Path

**Tipo**: Experimento de Arena (`lab/`)
**Prompt**: `00_nucleo/prompt/0005-medicao_colisoes.md`
**ADR-alvo da revisão**: `00_nucleo/adr/0004-resolucao-colisoes-path.md`
**Data**: 2026-05-27
**Estado**: medido contra projeto Rust real grande (typst). Categorias 1, 3 e 4
do prompt **não foram medidas** nesta rodada — ver "Limites declarados".

---

## TL;DR

- **384 colisões** de path próprias detectadas em **17 crates** do projeto
  typst (v0.14.2).
- **Estratégia 1 do `lente_investiga` é estruturalmente inaplicável** ao JSON
  do fork como existe hoje (descoberta crítica, ver §"Descoberta principal").
- Aplicando só a Estratégia 2 (fontes): **55 casos decididos (14.3%)**,
  **329 NaoDeterminado (85.7%)**.
- Dos NaoDeterminado, **48 (14.5%) seriam resolvíveis se a E2 entendesse
  `#[derive]`**; **281 (85.5%) estão genuinamente fora do alcance do parser
  textual** (código gerado por macros, inerent impls misturados com trait
  impls, etc.).

Os números abaixo sugerem que o ADR-0004 **precisa de revisão** — o desenho
da cascata não cumpre sua função na maior parte das colisões reais. A
interpretação fica com o autor.

---

## Método

- **Crates testados**: 17 crates internos do workspace
  `typst` (v0.14.2, edition 2024), em
  `/home/dikluwe/Documentos/Antigravity/typst-crystalline/lab/typst-original/`.
- **Preparação**: o repositório do typst tinha `Cargo.toml.original` (não
  `Cargo.toml`); criado link simbólico temporário `Cargo.toml →
  Cargo.toml.original` para o cargo achar o workspace. Link removido ao fim.
- **Extração**: para cada crate, `cargo modules export-json --sysroot
  --compact --package <nome>`. Tempo total da extração: **3min21s** (4–15s
  por crate; macros e syntax mais rápidos, typst-library mais demorado).
  Zero erros do fork. JSONs salvos em `lab/medicao-colisoes/json/`.
- **Análise**: script Python (`analisar.py`) que aplica **mentalmente** a
  cascata do `lente_investiga`. O parser textual da Estratégia 2 foi
  re-implementado em Python espelhando a lógica do
  `05_investiga/src/fontes.rs` (mesmo critério: pula `impl` inerentes, pula
  comentários, ignora genéricos antes do `for`, normaliza trait pelo último
  segmento `::`, conta `fn` em `depth==1`).
- **Decisão de meio**: Python + Markdown, conforme prompt autoriza. O
  ADR-0004 declara que `lente_resolve` ainda não existe; medir o investiga
  isolado é suficiente para esta rodada.

### Por que Python e não chamar o `lente_investiga` real?

Porque o JSON do fork **não fornece o insumo** que a Estratégia 1 do
`lente_investiga` exige. Construir um binário Rust que invocasse a função
real esbarraria no mesmo problema. Ver §"Descoberta principal".

---

## Descoberta principal — a E1 do `lente_investiga` é inaplicável aos dados reais

O `lente_investiga::investigar` espera um `Vizinhanca { a: ArestasNo, b:
ArestasNo }` — i.e., a chamadora **separa previamente** as arestas em "as do
nó A" e "as do nó B". A Estratégia 1 então compara esses dois conjuntos.

O JSON do `cargo-modules` referencia arestas por **`path`**, não por
identidade de nó. Quando há colisão (vários nós com o mesmo path), **todas
as arestas com aquele `from` ou `to` apontam para "o path"**, sem distinguir
qual cópia. A separação que a E1 exige é informação que o JSON perdeu na
serialização.

Logo: contra `cargo-modules` como existe hoje, a Estratégia 1 **nunca pode
decidir**. Toda colisão real cai diretamente na Estratégia 2 (fontes), ou em
`NaoDeterminado` se a E2 não conseguir.

**O ADR-0004 desenhou a cascata pensando que E1 cobriria o caso comum**;
medindo, a E1 é estruturalmente inerte. Três caminhos possíveis (decisão do
autor — não prescritos aqui):

1. **Pedir ao fork** identidade de nó (índice numérico ou UUID) nas arestas,
   além do path. Resolveria a E1 e seria coerente com a Nota de Evolução já
   registrada na spec sobre subtipos.
2. **Aceitar**: declarar E1 morta, redesenhar `lente_investiga` como
   "Estratégia 2 + diagnóstico". A cascata vira só uma etapa.
3. **Rederivar separação** por heurística (atribuir cada aresta a uma cópia
   por critério qualquer). Risco: introduz interpretação não baseada em
   evidência — explicitamente contra o princípio do ADR-0004.

---

## Resultados por crate

| Crate | Nodes | Edges | Colisões | Decididas (E2) | NaoDeterminado |
|-------|-------|-------|----------|----------------|----------------|
| typst (lib) | 625 | 2043 | 10 | 2 | 8 |
| typst (cli, ver nota *) | 78 | 119 | 0 | 0 | 0 |
| typst_bundle | 158 | 488 | 0 | 0 | 0 |
| typst_eval | 211 | 692 | 0 | 0 | 0 |
| typst_html | 591 | 1775 | 7 | 1 | 6 |
| typst_ide | 243 | 686 | 2 | 0 | 2 |
| typst_kit | 269 | 801 | 1 | 0 | 1 |
| typst_layout | 1152 | 5573 | 4 | 4 | 0 |
| **typst_library** | **9072** | **33276** | **316** | **35** | **281** |
| typst_macros | 277 | 856 | 16 | 0 | 16 |
| typst_pdf | 811 | 2774 | 0 | 0 | 0 |
| typst_realize | 131 | 331 | 0 | 0 | 0 |
| typst_render | 117 | 268 | 0 | 0 | 0 |
| typst_svg | 216 | 625 | 8 | 1 | 7 |
| typst_syntax | 1501 | 4353 | 6 | 5 | 1 |
| typst_timing | 41 | 63 | 0 | 0 | 0 |
| typst_utils | 262 | 555 | 14 | 7 | 7 |
| **Total** | **15755** | **55274** | **384** | **55** | **329** |

\* **Nota sobre `typst-cli`**: o JSON retorna `"crate": "typst"` porque o
package `typst-cli` produz binário `typst` (campo `[[bin]]`); o cargo-modules
adota o nome do binário. Não é duplicação, são análises distintas (lib + cli).

### Observações

- **7 dos 17 crates (41%) não têm nenhuma colisão própria**: typst-cli,
  typst-bundle, typst-eval, typst-pdf, typst-realize, typst-render,
  typst-timing. Crates "bem-comportados" existem — não é regra do ecossistema
  que todo crate Rust colida.
- **typst_library concentra 82% das colisões** (316/384). É o maior crate
  do projeto (9072 nodes) e contém a biblioteca de tipos de domínio do
  typst, com derives extensos. Distorce o agregado.
- **typst_macros tem 16 colisões, 0 decididas**: previsível — o crate de
  macros emite código gerado que não tem `impl <Trait> for X` literal no
  fonte do próprio macros.

---

## Categorização das 384 colisões

### Decididas pela Estratégia 2 (55 = 14.3%) — distribuição de padrões

| Padrão | Casos | Notas |
|--------|-------|-------|
| `Debug + Display` | 9 | O caso "canônico" do `ErroRaio` |
| `Div + Div<f64>` | 6 | Operador `Div` com self e com f64 (sistema de unidades) |
| Vários `From<X> + From<Y>` | ~20 | Tipos típicos de typst convertidos de várias fontes |
| `From<Abs> + From<Em>` | 3 | Unidades absolutas vs. relativas |
| `Sum + Sum<&'a Self>` | 2 | Operador `Sum` por valor e por referência |
| `Add + Add<T>` / `Add<Self> + Add<f64>` | 3 | Operador `Add` overloaded |
| `Mul + Mul<f64>`, `Sub + Sub<...>` | 2 | Operadores aritméticos |
| `PartialEq + PartialEq<f64>` / `PartialEq + PartialEq<&'static ...>` | 2 | Igualdade overloaded |
| `BitAnd + BitAnd<bool>`, `BitOr + BitOr<bool>` | 2 | Bit-ops overloaded |
| `AddAssign + AddAssign<&Self>` | 1 | |
| Outros (mistos) | ~5 | |

**Observação**: a maioria das decisões positivas **não é** o padrão clássico
`Debug+Display`. São impls do **mesmo trait** com type parameters diferentes
(`Add<Self>` vs `Add<f64>`). O `lente_investiga` os classifica como
`Distintos` porque o último segmento `::` mantém o `<f64>`, então
`"Add"` vs `"Add<f64>"` viram traits distintos.

### NaoDeterminado (329 = 85.7%) — distribuição por motivo

| Motivo (categorizado pela razão) | Casos | % do NaoDet |
|----------------------------------|-------|-------------|
| E2 encontrou **0** traits-de-impl (macro-gerado, ou só inerent impls) | 281 | 85.5% |
| E2 encontrou **1** trait-de-impl (provável `#[derive]` + impl manual) | 48 | 14.5% |

Cada categoria tem sub-padrões claros nos exemplos:

**Casos de "0 traits"**:
- `typst_library::engine::__ComemoCall::{clone, eq, hash}` (n=3 cada).
  Underscore-prefix delata código gerado pela proc-macro `comemo`. **O fonte
  textual não contém o `impl` literal**, o parser nunca vai achar.
- `typst_macros::cast`, `typst_macros::elem`, etc. (n=2). Funções `#[proc_macro_attribute]`;
  o cargo-modules emite cada macro em dois nós por razão interna ao
  rust-analyzer (a definição da função + expansão).
- `typst_html::HtmlElem::Type` (n=5). `Type` é um **associated type**, não um
  método. O parser só busca `fn`, ignora `type` declarations.

**Casos de "1 trait"**:
- `typst_library::diag::FileError::fmt`, `::PackageError::fmt`,
  `::Tracepoint::fmt` — todos com `1 trait: ['Display']`. Tipo padrão Rust:
  `#[derive(Debug)] enum FileError { ... } impl fmt::Display for FileError
  { fn fmt(...) }`. O parser **vê o Display** mas **não vê o derived Debug**,
  porque `#[derive(Debug)]` não gera um `impl Debug for FileError` literal
  no fonte — o rustc/cargo-modules expande, mas o `.rs` original não tem.
- `typst_html::dom::HtmlDocument::introspector` — `1 trait: ['Output']`.
  Provavelmente **inerent impl + trait impl** com mesmo método; o parser
  ignora inerent (`impl HtmlDocument { ... }` sem `for`) e acha só o impl de
  `Output`. O `lente_investiga` atualmente não distingue isso.

### Implicação direta para a E2

Estender o parser textual para reconhecer `#[derive(X)]` na declaração do
tipo recuperaria **48 casos**. Isso elevaria a taxa de decisão de 14.3%
para 26.8% (103/384). Os outros 281 (73.2%) continuariam fora.

### Padrões de colisão por número de cópias

Distribuição observada de `n_nos` (quantas cópias do mesmo path o fork
emitiu): 2, 3, 4, 5, 6, 7, 8, 9, 10, 11+. **Colisões com 11+ cópias do
mesmo path** existem em typst_library (não amostradas individualmente neste
relatório; estão no `analise.json`).

---

## Avaliação contra os três cenários do ADR-0004

Replicando os três cenários que o próprio ADR-0004 antecipa:

> **Cenário A**: se a maioria das colisões for resolvida pela Estratégia 1
> (vizinhança), a arquitetura justifica-se; Estratégia 2 fica como
> fallback raro.

→ **Não realizado.** Estratégia 1 é estruturalmente inaplicável
(Descoberta principal). 0% das colisões reais foram para a E1.

> **Cenário B**: se a maioria exigir a Estratégia 2 (código-fonte), a
> cascata vira só uma otimização menor — defensável mas com menor ganho que
> o esperado.

→ **Parcialmente realizado.** A Estratégia 2 é o único caminho efetivo,
mas decide só 14.3% (55/384). Como ela é o único caminho, a "cascata" não é
cascata — é um único degrau.

> **Cenário C**: se a Estratégia 2 raramente decide (maioria
> `NaoDeterminado` ao final), a arquitetura não cumpre sua função, e o ADR
> precisa de revisão.

→ **Realizado.** 85.7% das colisões ficam `NaoDeterminado`. Mesmo com a
extensão à `#[derive]` (mais 48 casos), a taxa subiria só para ~27%.

---

## Caminhos sugeridos (para decisão do autor, não prescrição)

Em ordem de invasividade crescente:

1. **Estender E2 a `#[derive]`**: detectar `#[derive(Debug, Clone, ...)]`
   na declaração do tipo e considerar X como trait com método sintético
   correspondente. Resolveria 48 casos sem mudar interface; o
   `lente_investiga` ganharia um sub-parser de derive. **Ganho marginal**:
   passa de 14.3% para 26.8%.

2. **Estender E2 a inerent impls**: reconhecer `impl <Tipo>` (sem `for`)
   como impl-com-trait-sintético ("Inherent"). Distinguiria
   `impl HtmlDocument { fn introspector }` de `impl Output for HtmlDocument
   { fn introspector }`. Não medido aqui — provavelmente resolveria mais
   alguns casos do grupo "1 trait" não cobertos por derive.

3. **Pedir ao fork identidade-por-nó nas arestas**: a solução estrutural
   para o problema da E1. Resolve **todas** as colisões cuja vizinhança é
   realmente disjunta no JSON original (que hoje é informação perdida na
   serialização). Coerente com a Nota de Evolução já registrada na spec.

4. **Aceitar como Limite 6 da spec**: 73% das colisões reais ficam sem
   solução automática. Declarar como limite (com diagnóstico claro ao
   usuário) — mas isso invalida o objetivo do ADR-0004 ("lente operável
   contra crates idiomáticos").

5. **Revisar o ADR-0004**: redesenhar a arquitetura com base no que a
   medição mostra. Possivelmente: investigação como única etapa (não
   cascata), priorizando saídas estruturadas do fork em vez de parsers
   textuais.

---

## Limites declarados desta medição

- **Categorias 1 e 3 do prompt não foram medidas**: crates pequenos
  idiomáticos externos (anyhow, thiserror, etc.) e crates de produção
  amplamente usados (hyper, rayon, etc.). Foram **substituídas** pelo
  workspace typst — que cobre as categorias 2 (grandes/complexos) e parte da
  4 (próprio projeto-lente não foi remedido aqui, mas os 3 crates próprios
  já foram exercitados nos prompts 0001-0004 e a colisão `ErroRaio::fmt` é
  conhecida e cobre o padrão `Debug+Display`).
- **Estratégia 1 não foi exercitada por construção** — a Descoberta
  principal mostra que ela não tem como decidir aos dados reais como
  existem. Não há como "medir" o que estruturalmente não roda.
- **Heurística da E2 para inerent impls não foi medida em isolamento**:
  os 48 casos "1 trait" misturam derives faltantes e inerent-vs-trait.
  Separar requer inspeção caso-a-caso, fora do escopo desta primeira
  rodada.
- **Sem comparação cross-projeto**: 17 crates, todos do typst, mesma
  geração de autores, padrões de código relacionados. Medir contra projetos
  de origens diferentes (anyhow, tokio, etc.) é trabalho posterior.

---

## Tempos e custo

- Extração JSON dos 17 crates: **3min21s** (4–15s por crate, exit-code 0
  para todos, zero stderr).
- Análise Python (jq não estava disponível; usado Python3 com `re` da
  stdlib): instantânea.
- Total disco: ~7 MB de JSONs (`typst_library.json` sozinho tem 4.9 MB).

---

## Artefatos do experimento

- `lab/medicao-colisoes/json/*.json` — JSONs brutos do `cargo modules
  export-json` por crate (17 arquivos).
- `lab/medicao-colisoes/json/*.err` — stderr de cada execução (todos zero
  bytes — sem erros).
- `lab/medicao-colisoes/analisar.py` — script Python (~200 linhas) que
  replica a cascata do `lente_investiga` aplicada aos JSONs.
- `lab/medicao-colisoes/analise.json` — resultado bruto estruturado da
  análise.
- `lab/medicao-colisoes/relatorio.md` — este documento.

---

## Histórico

| Data | Motivo |
|------|--------|
| 2026-05-27 | Medição inicial. 17 crates do typst v0.14.2. 384 colisões; 14.3% decididas; 85.7% NaoDeterminado. Descoberta crítica: E1 estruturalmente inaplicável ao JSON do fork. |
