# Prompt: Filtro de stdlib (`lente_filtro`, L1) — esconder sysroot preservando os impls do alvo

**Camada**: L1 — Núcleo (lógica pura, sem deps externas)
**Criado em**: 2026-06-02
**Revisado em**: 2026-06-02 — após a Fase 1 (laudo 0025) refutar a premissa
sobre `crate` por nó no fork. Marca de stdlib volta a ser **por prefixo do
path** (ADR-0002 D3 vindicada pelo dado, não superada).
**Estado**: `PROPOSTO` (revisão A do laudo 0025)
**Decisões de origem**:
- Laudo 0021, Bloco C/D — sysroot domina os rankings (7/10 do top-10 do egui
  são `core::*`/`alloc::*`); a pendência do filtro subiu de latente para ativa.
- Spec, Limite 2 — a forma organizada **inclui** os nós de stdlib (tradução
  fiel); esconder a stdlib é um componente L1 separado, que deve preservar o
  `impl` do sistema-alvo que liga a um trait de stdlib.
- **Laudo 0025 (Fase 1)** — o fork 0.27.0 não emite `crate` por nó;
  `No.crate_name` é uniforme (= crate-raiz). Marca por `crate_name` não é
  viável; a D3 do ADR-0002 (marca pelo prefixo do path) **fica preservada**.
  A verificação do dado real (108 nós do `lente_core`) mostrou também que o
  Limite 2 fica **seguro por construção** neste fork: o path do impl-do-alvo
  é `lente_core::…::ErroRaio::fmt` (com `trait: Display`), não `core::…` —
  zero sobreposição "path em prefixo sysroot ∧ trait/trait_ref preenchido".
**Pré-requisito**: `lente_core` (tipos `Grafo`/`No`/`Path`/`Aresta`); o dado
real da medição egui (`lab/medicao-egui`) para a verificação E2E.
**Posição**: pendência 2 do laudo 0021. É **pré-requisito do modo ranking**
(o consumidor), que é prompt próprio, depois. Este prompt entrega **só o
componente L1** — sem wiring.
**Arquivos afetados (a confirmar)**: novo crate `07_filtro/` (`Cargo.toml` +
`src/lib.rs` com testes inline); `Cargo.toml` raiz (members).
**Arquivos NÃO afetados**: `00_nucleo/adr/0002-modelagem-do-grafo.md`
(D3 fica como está — vindicada pelo dado real).

---

## Contexto

O filtro é uma transformação pura `Grafo → Grafo` que remove os nós de sysroot
(`std`, `core`, `alloc`, …) e as arestas que tocam neles. Motivo: o ranking
(consumidor futuro) fica dominado por nós de stdlib, que têm montante enorme
(quase tudo os usa) e empurram os nós do alvo para baixo.

### A sutileza do Limite 2 (o risco da spec, mitigado pelo fork)

A spec descreve a fronteira fina: o que liga um item do alvo a um trait de
stdlib (`MinhaStruct → core::clone::Clone`) passa por um `impl` que é **do
alvo**, não da stdlib. Em princípio, um filtro por prefixo do path poderia
remover por engano o nó do `impl` se o fork o nomeasse com `core::…`. A
Fase 1 do laudo 0025 verificou contra o JSON real do `lente_core` e
encontrou o oposto: o fork 0.27.0 nomeia o impl pelo lado do **alvo** — o
path é `lente_core::domain::raio::ErroRaio::fmt`, com `trait: "Display"` e
`trait_ref: "Display"`. **Zero ocorrências** de "path em prefixo sysroot ∧
trait preenchido" em 108 nós. Logo: filtrar por prefixo do path **respeita
o Limite 2 por construção neste fork** — sem cláusula híbrida.

Se em algum dado futuro essa propriedade quebrar (ex.: outra versão do
fork, ou um crate exótico), o E2E do prompt detectará: um teste afirma
explicitamente que os 54 impls-do-alvo do `lente_core` continuam após o
filtro.

### Por que `path`, não `crate_name` (a revisão A)

`No.crate_name` no fork 0.27.0 vem do `crate` do **grafo** (top-level), não
de campo por nó (o fork não emite). Logo `crate_name` é igual para os 108
nós do `lente_core`, incluindo os 17 que são sysroot. Marcar por
`crate_name` neste fork **não funciona**: o filtro seria no-op, ou viraria
uma comparação com prefixo do path disfarçada. ADR-0002 D3 fica.

A coerência com o princípio do projeto — "fonte autoritativa > heurística"
— **se mantém**: o `path` é a única evidência por-nó que o fork oferece.
Não é heurística externa; é a única informação confiável disponível por
nó. Se um dia o fork emitir `crate` por nó (opção B do laudo 0025), aí sim
a supersessão da D3 será fato, não suposição — e a troca será trivial.

---

## Restrições estruturais

- **L1 puro, zero deps externas** (como `lente_investiga`/`lente_resolve`).
  Depende só de `lente_core`. Usa só stdlib (`HashSet` etc.).
- **Não muda os tipos `Grafo`/`No`/`Path`/`Aresta`** — funciona sobre o que
  já existe.
- **Preserva os `id` dos nós mantidos.** A CLI referencia nós por `id`
  (`--alvo-id N`); o filtro **não renumera**.
- **Preserva `Grafo.crate_name`.**
- **Remove só sysroot.** Nós de dependências **não**-stdlib (ex.: `emath`,
  `ecolor` quando se analisa `egui`) são **mantidos**.
- **Sem wiring.** Não toca L2/L4/CLI nem o `raio`. O consumidor (ranking) é
  prompt próprio.
- **Não toca o fork, a E2 (quarentena), nem o `raio`.**

---

## Fase 2 — Conserto (a Fase 1 está no laudo 0025; este prompt parte dela)

### Novo crate L1

```
07_filtro/
  Cargo.toml      # name=lente_filtro; deps: lente_core (path); zero externas
  src/lib.rs      # filtrar_stdlib + sysroot consts + testes inline
```
Adicionar `"07_filtro"` aos `members` do `Cargo.toml` raiz.

### Função pura

`filtrar_stdlib(grafo: &Grafo) -> Grafo`, pura, sem I/O:

- Conjunto de prefixos de sysroot: `const SYSROOT_PREFIXES: &[&str] =
  &["std", "core", "alloc", "proc_macro", "test"]` — observado no
  `lente_core` (`{core, alloc, std}`) + `proc_macro`/`test` por
  defensividade.
- Predicado interno `e_de_sysroot(path: &Path)`: verifica se o **primeiro
  segmento** do `path.as_str()` está em `SYSROOT_PREFIXES`. Comparação por
  segmento, não `starts_with` cego — evita que um hipotético crate
  `core_extras` seja confundido com `core` (defesa contra a regra-no-papel
  que perdeu casos no laudo 0008).
- Coleta `ids_removidos: HashSet<usize>` enquanto filtra `nodes`.
- Reconstrói `edges` removendo arestas em que `id_from` ou `id_to` ∈
  `ids_removidos`.
- Preserva `Grafo.crate_name`. Não renumera ids.

---

## Critérios de Verificação

```
Dado um nó de impl-do-alvo (path do alvo, com trait/trait_ref de stdlib)
Quando filtrar_stdlib roda
Então o nó é PRESERVADO; o nó do trait de stdlib é removido; as arestas para
  ele são removidas (Limite 2 respeitado — verificado por construção no
  fork 0.27.0 pelo laudo 0025)

Dado um nó com path começando por prefixo de sysroot
Quando filtrar_stdlib roda
Então o nó é removido, e toda aresta que o toca é removida

Dado um nó com path de dependência NÃO-stdlib (ex.: emath quando se analisa egui)
Quando filtrar_stdlib roda
Então o nó é mantido

Dado um grafo filtrado
Quando se comparam os id dos nós mantidos com os de antes
Então são iguais (sem renumeração)

Dado um grafo sem nenhum nó de stdlib
Quando filtrar_stdlib roda
Então o grafo sai inalterado (idempotência sobre grafo limpo)

Dado o grafo filtrado
Então Grafo.crate_name é preservado

Dado um nó cujo path tem prefixo "core_extras" (não é "core")
Quando filtrar_stdlib roda
Então o nó é mantido (comparação por segmento, não starts_with cego)
```

Casos a cobrir nos testes (todos no `src/lib.rs`, padrão do projeto):

- **Unidade, puros** (Grafos montados à mão, sem cargo/fork):
  - Limite 2: impl-do-alvo com `trait: Display` mas path `meu::T::fmt` →
    preservado.
  - Nó com path `core::fmt` → removido com arestas.
  - Nó com path `meu_dep::X` → mantido.
  - `id` preservados nos mantidos.
  - Grafo sem stdlib → inalterado (fingerprint).
  - `crate_name` preservado.
  - Falso-positivo evitado: path `core_extras::Y` mantido.
- **E2E `#[ignore]`** sobre o `lente_core` real (via `lente_infra`):
  - Sysroot some (zero nós com `path` iniciando em `std::`/`core::`/`alloc::`).
  - Contagem cai dos 108 para os 91 do alvo (registrar; se variar com
    versão do fork, ajustar a banda).
  - Os 54 impls-do-alvo de traits de stdlib (achados na Fase 1: `ErroRaio::fmt`,
    `Classificacao::clone`, etc.) permanecem.

---

## Resultado esperado

- Componente L1 `lente_filtro` que esconde sysroot preservando o Limite 2,
  por **prefixo do path** (ADR-0002 D3 preservada).
- Sem consumidor ainda (o ranking é o próximo); validado por testes de
  unidade puros + um E2E sobre o dado real do `lente_core`.
- Pureza do L1 mantida (zero deps externas no crate novo).
- **Laudo 0025 atualizado**: Fase 1 + Fase 2; registra a decisão A da
  revisão; D3 do ADR-0002 não é tocada.

---

## O que NÃO entra

- **Modo ranking / wiring / CLI**: prompt próprio — é o consumidor do filtro.
- **Remoção da E2**: quarentena.
- **Mudança nos tipos `Grafo`/`No`, no `raio`, no fork, no `lente_core`**
  (além de ser dependência): nenhuma.
- **Mudança no ADR-0002 (D3 inclusive)**: nenhuma — vindicada pelo dado.
- **Filtro de "folhas comportamentais"**: outro filtro, outra pendência.
- **Configurabilidade do conjunto de sysroot**: não. `const` basta.

---

## Observação metodológica (revisão A)

O ciclo Fase 1 → Suspensão → Reescrita é o próprio padrão do projeto em
ação: a verificação contra dado real refutou a premissa do prompt
original (`crate_name` autoritativo) e simultaneamente **vindicou** a
decisão a montante que essa premissa propunha superar (D3 do ADR-0002).
O ganho é que o filtro nasce assentado em fato observado, não em
suposição de evolução do fork — e o ADR fica em paz com a realidade do
fork 0.27.0.

Se um dia o fork emitir `crate` por nó (opção B do laudo 0025), a
supersessão da D3 voltará à mesa — mas como fato, não premissa. Até lá,
prefixo do path **é** a melhor fonte por-nó disponível, e o Limite 2
fica protegido por uma propriedade verificada do fork (não por
suposição).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-02 | Revisão A (laudo 0025): marca por prefixo do path (ADR-0002 D3 preservada) em vez de `crate_name` (fork 0.27.0 não emite por nó). Limite 2 verificado seguro por construção neste fork. | `07_filtro/{Cargo.toml,src/lib.rs}`, `Cargo.toml` (members), `00_nucleo/lessons/0025-l1-filtro-stdlib.md` |
