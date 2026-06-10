# Prompt: censo do `--comparar` só com membros (filtro de third-party em lado-workspace)

**Camada**: L1 — Núcleo (o filtro, função pura) + L4 — Fiação (aplicação no
comparar) + L2 — CLI (saída declara o que foi filtrado)
**Criado em**: 2026-06-10
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0076-filtro_third_party_comparar.md`)
**Número**: confirmar na Fase 1 o próximo número livre em `00_nucleo/prompt/`
(após o 0075; o 0076 está reservado para a tela lado a lado — este prompt não
é a tela; renumerar conforme a convenção do diretório).
**Decisões de origem**: laudo 0075, seção "Ruído de terceiros" — o censo do
lado vanilla na rodada typst incluiu módulos de crates de terceiros
(`comemo`, `ecow`, `citationberg`, `krilla`, `codespan_reporting`, …). O
escopo `seu-codigo` filtra **sysroot**, não dependências third-party. Os 392
sem-par do lado antes estão inflados por código que não é do sistema-alvo —
qualquer paridade calculada sobre esse censo está errada antes de começar.
Este filtro é pré-requisito de honestidade para o pareamento por identidade
de item (trilha adiada do 0075).
**Pré-requisito**: 0075 (`--comparar` ciente de workspace; o lado-workspace
já carrega a lista de membros via `enumerar_membros`); 0025 (`lente_filtro`,
o filtro de sysroot e a verificação do Limite 2); 0045 (a semântica
membro/externo da união).
**Posição**: passo 1 da fila pós-0075 (filtro → checagem dos fantasmas →
identidade de item → tela).
**Arquivos afetados (a confirmar na Fase 1)**: `lente_filtro` ou
`lente_comparacao` (L1 — onde o filtro mora, decisão abaixo),
`04_wiring/src/lib.rs` (aplicação no `extrair_lado`/comparar),
`02_shell/cli/src/saida.rs` + catálogo (contagem do filtrado), testes.

---

## Contexto

O grafo de um lado-workspace contém três populações de nós: os **membros**
(o código do sistema), o **sysroot** (`std`/`core`/`alloc`/…, já filtrado
pelo escopo `seu-codigo` via `lente_filtro`), e as **dependências
third-party** (crates externos referenciados pelo código). A rodada typst
mostrou que a terceira população entra no censo do comparar e infla o
sem-par.

O discriminador para "membro vs third-party" já existe no sistema: a união
(0045) decide dono pelo **primeiro segmento do path**, e o lado-workspace
já enumera os membros (`enumerar_membros`, 0044). O que falta é uma função
pura que, dado o grafo e o conjunto de nomes-membro, remova os nós (e as
arestas que os tocam) cujo dono não é membro — e a fiação aplicá-la no
comparar.

**Semântica declarada**: num lado-workspace, "seu código" significa **os
membros do workspace**. O filtro de third-party entra no escopo
`seu-codigo` existente, ao lado do de sysroot — **sem flag nova**. Num
lado-crate (modo crate×crate do 0074), nada muda.

**O que este filtro NÃO resolve (declarado para não confundir)**: os 448
fantasmas do lado vanilla. `typst-macros` **é membro** — os representantes
de fantasma que as referências a ele geram têm dono-membro e **passam**
pelo filtro. Fantasmas são a trilha do resolvedor de colisão, separada.

---

## Restrições estruturais

- **L1 — o filtro é puro.** Recebe `(Grafo, conjunto de nomes-membro)` e
  devolve o grafo filtrado. Só stdlib + `lente_core`. A lista de membros
  vem de fora (L3/L4 a fornece); o filtro não enumera nada.
- **Retrocompat crate×crate.** O modo crate×crate do comparar não é tocado
  (bit a bit, guarda do 0074/0075). A assimetria (workspace filtra
  third-party, crate não) é registrada na saída pela proveniência que o
  0075 já criou (a chave/modo declarados por lado).
- **`--diff` e `--estrutura` não mudam.** O filtro nasce consumido só pelo
  comparar. Se for útil aos outros modos, é decisão posterior com uso
  declarado ("não estruturar antes do uso pedir"). Para isso, a função
  mora num lugar reusável — ver decisão de morada abaixo.
- **Sem deps novas.**

---

## Fase 1 — Leitura primeiro (obrigatória)

1. **O que o censo do comparar conta hoje**: módulos do grafo unido? nós?
   Onde a contagem de sem-par é montada, para o filtro entrar **antes**
   dela.
2. **A forma dos nomes**: `enumerar_membros` devolve nomes com hífen
   (`typst-macros`, do `Cargo.toml`); os paths usam underscore
   (`typst_macros`). Confirmar onde a normalização hífen↔underscore já
   existe no projeto (o harness do oráculo tem um `norm`; a união do 0045
   pode já normalizar) e reusar a convenção — **não** inventar uma segunda.
3. **Morada do filtro**: `lente_filtro` já é o crate de filtros de grafo
   (sysroot, 0025). A opção natural é uma função irmã
   (`filtrar_nao_membros(grafo, membros)`). Alternativa: dentro de
   `lente_comparacao`, se a Fase 1 mostrar que o tipo de entrada do censo
   não é `Grafo`. Escolher o que exigir menos conversão; registrar.
4. **Representantes de fantasma no censo**: verificar e **registrar no
   laudo** se os representantes (0045) entram na contagem de módulos/sem-par.
   Não consertar aqui — é o dado que a trilha dos fantasmas precisa.
5. Confirmar o próximo número livre de prompt.

---

## O que mudar

### 1. O filtro (L1)

```
filtrar_nao_membros(grafo, membros: &<conjunto de nomes normalizados>) -> Grafo
```

- Um nó **fica** se o primeiro segmento do seu path, normalizado
  (hífen↔underscore, a convenção da Fase 1), pertence a `membros`.
- Um nó **sai** caso contrário; as arestas que tocam um nó removido saem
  junto (0 arestas soltas — invariante das outras operações de grafo do
  projeto).
- Determinístico; ordem de saída estável (mesma disciplina do 0045).

**Sobre o risco do Limite 2** (o que fez o filtro de sysroot ser delicado):
o medo era remover o nó de impl do alvo junto com o trait externo. A
verificação dos laudos 0025/0027 mostrou que este fork nomeia o
impl-do-alvo pelo lado do alvo (`lente_core::…::fmt`, não `core::…::fmt`)
— o filtro por primeiro segmento é seguro por construção. O mesmo
argumento vale aqui: um impl de membro para trait third-party tem path de
membro. A Fase 1 não precisa re-medir; precisa só **citar** essa base no
laudo.

### 2. Aplicação no comparar (L4)

No caminho de lado-workspace (o `extrair_lado` do 0075), após montar e
unir: aplicar o filtro com os membros que `enumerar_membros` já devolveu,
**antes** do censo e do pareamento. Lado-crate: caminho inalterado.

### 3. Saída declara o filtrado (L2)

A proveniência por lado (0075) ganha a contagem: quantos nós/módulos de
third-party foram removidos do censo daquele lado. Texto e JSON, aditivo,
strings no catálogo (ADR-0002). Razão: o número que some precisa ser
visível, senão o filtro vira mascaramento.

---

## O que NÃO muda

- Modo crate×crate do comparar (0074) — bit a bit.
- `--diff`, `--estrutura`, o filtro de sysroot existente, a união (0045),
  o cache (0044).
- Fantasmas: continuam no grafo e na proveniência como estão (são
  dono-membro; este filtro não os toca).

---

## Critérios de Verificação

```
Dado um grafo forjado com nós de membro (m::a, m::b), de third-party
(ecow::x) e uma aresta m::a -> ecow::x
Quando filtrar_nao_membros com membros = {m}
Então ecow::x sai, a aresta que o tocava sai, m::a e m::b ficam, 0 soltas

Dado um nó de membro cujo nome no Cargo.toml tem hífen (typst-macros) e
path com underscore (typst_macros::kw)
Quando filtrar_nao_membros com membros normalizados
Então o nó fica (a normalização casa hífen com underscore)

Dado um representante de fantasma de dono-membro
Quando filtrar_nao_membros
Então ele fica (o filtro não remove fantasmas)

Dado o mesmo grafo e o mesmo conjunto de membros
Quando filtrar duas vezes
Então a mesma saída (determinístico, idempotente)

Dado dois diretórios-de-crate (fixtures do 0074/0075)
Quando --comparar roda
Então a saída é idêntica à do 0075 (crate×crate intocado)

Dado o workspace da lente como --antes E como --depois (#[ignore], fork real)
Quando --comparar roda
Então a paridade total se mantém e a proveniência mostra a mesma contagem
de third-party filtrado dos dois lados

Dado o JSON do comparar pós-filtro
Quando desserializado
Então os campos do 0075 estão presentes e a contagem de filtrado é aditiva
```

Casos puros (sem fork): remoção com arestas, normalização hífen↔underscore,
fantasma-representante preservado, idempotência/determinismo. E2E
`#[ignore]`: retrocompat crate×crate; lente vs lente. A rodada typst entra
no laudo, não na suíte.

---

## Resultado esperado

- `filtrar_nao_membros` (L1, pura), aplicada no lado-workspace do comparar
  antes do censo; saída declara a contagem filtrada por lado.
- **Laudo** em `00_nucleo/lessons/`:
  - A morada escolhida para o filtro e a convenção de normalização reusada.
  - A base do Limite 2 citada (0025/0027) — por que o filtro por primeiro
    segmento é seguro neste fork.
  - O achado da Fase 1 sobre representantes de fantasma no censo
    (entram ou não na contagem de sem-par) — **sem consertar**, só o dado.
  - **A rodada typst re-executada** (cache morno do 0075): sem-par
    antes/depois do filtro nos dois lados, e a contagem de third-party
    removido por lado. O delta contra os 392/177 do 0075 é o resultado
    citável deste prompt.
  - Contagem da suíte (era 301 verdes + 34 ignored no 0075).

---

## O que NÃO entra

- **O resolvedor de colisão / os 448 fantasmas** — trilha própria; aqui só
  o dado da Fase 1 sobre representantes no censo.
- **Pareamento por identidade de item** — próximo da fila, depois deste.
- **Aplicar o filtro ao `--diff`/`--estrutura`** — sem uso declarado ainda.
- **Flag nova** — o filtro entra na semântica do escopo `seu-codigo` em
  lado-workspace.
- **Tela lado a lado (0076).**

---

## Observação metodológica

O filtro existe para um número servir, não por completude — o mesmo padrão
do filtro de sysroot (laudo 0021 mostrou o ranking dominado por `core::*`;
o 0025 o consertou). Aqui o 0075 mostrou o sem-par dominado por
third-party; este prompt o conserta **antes** de o pareamento por
identidade de item ser desenhado, para que o primeiro número daquela
trilha já nasça honesto. E a contagem do que foi removido vai para a
saída porque filtro silencioso é mascaramento — o princípio do
"silenciosamente ignorado" que o projeto já pagou para aprender.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Proposta: censo do `--comparar` em lado-workspace passa a conter só membros — `filtrar_nao_membros` (L1, pura; primeiro segmento do path vs nomes-membro normalizados), aplicada antes do censo/pareamento; proveniência declara a contagem filtrada por lado; crate×crate intocado; fantasmas preservados (trilha separada). | a confirmar na Fase 1 |
