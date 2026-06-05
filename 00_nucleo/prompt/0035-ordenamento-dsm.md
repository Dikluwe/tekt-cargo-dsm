# Prompt: Ordenamento da DSM — condensação + ordem topológica (`lente_estrutura`)

**Camada**: L1 (`lente_estrutura`) + L4 (fiação) + L2 (CLI + catálogo)
**Criado em**: 2026-06-04
**Estado**: `PROPOSTO`
**Decisões de origem**: trilha da DSM (origem Lattix/Structure101 do projeto); o
autor escolheu **começar pelo tijolo L1** e emitir a **matriz como dado** antes
da matriz visual. O SCC já existe (`detectar_ciclos`, laudo 0031); o tijolo novo
é o **ordenamento** — colapsar os SCCs e ordenar topologicamente, para a matriz
ter as dependências de um lado da diagonal e os ciclos como blocos na diagonal.
É o uso que separa a lente de ferramentas de texto (grep/ripgrep/tree-sitter):
ordem topológica só existe sobre grafo **dirigido e resolvido**.
**Pré-requisito**: `lente_estrutura` (`agregar_por_modulo`, `detectar_ciclos`,
laudo 0031); `analisar_estrutura` + `--estrutura` com `modo_uses` (laudos
0031/0034).
**Posição**: primeiro tijolo da DSM. Entrega a **ordem** (a matriz como dado:
módulos ordenados + blocos de ciclo + as dependências que já existem). A matriz
**visual** lê esse dado depois; um agente também o consome.
**Arquivos afetados (a confirmar na Fase 1)**: `09_estrutura/src/lib.rs`;
`04_wiring/src/lib.rs`; `02_shell/cli/src/*`; `02_shell/catalogo/src/lib.rs`;
testes.

---

## Contexto

Uma DSM é uma matriz N×N — linha e coluna são os mesmos módulos, a célula
`(i,j)` marca "linha `i` depende da coluna `j`". O valor dela vem de **ordenar**
linhas/colunas para empurrar as dependências para **um lado da diagonal**; o que
sobra do outro lado são os **ciclos**, visíveis de relance. **A ordem é o
tijolo.**

Como ordenar:

1. **Colapsar** cada SCC num ponto único — a **condensação**, que é um DAG (sem
   ciclos, por definição: cada ciclo virou um ponto).
2. **Ordenar topologicamente** o DAG da condensação.
3. **Expandir**: dentro de cada SCC os membros podem vir em qualquer ordem (são
   mutuamente cíclicos); usar ordem estável (por `path`) para determinismo.
4. A ordem final = ordem topológica da condensação, com os membros de cada SCC
   agrupados.

O resultado: uma ordem linear dos módulos em que quase toda aresta de dependência
aponta para o mesmo lado (o empilhamento do DAG), e as únicas células do "lado
errado" estão **dentro** dos SCCs — que aparecem como **blocos densos na
diagonal** (os emaranhados), com as dependências entre SCCs todas numa direção
(camadas limpas).

O SCC já existe (`detectar_ciclos`). O cálculo novo é **condensação + ordem
topológica** — L1 puro, irmão do `detectar_ciclos`.

Este prompt emite a **ordem** (módulos ordenados + blocos de SCC). As
**dependências** já são emitidas desde o laudo 0031. Ordem + dependências +
blocos = **a matriz como dado**, suficiente para um consumidor (a tela futura, ou
um agente) montar a grade. **Sem desenhar a grade aqui** — isso é a tela, depois.

**Interação com `modo_uses` (laudo 0034)**: a ordem roda sobre o grafo de módulos
no modo escolhido. Em `--so-referencia` o bloco de ciclo é o de 42; em `todas`, o
de 85 — um bloco gigante na diagonal, menos legível. A ordem funciona nos dois; a
matriz fica **mais legível em `--so-referencia`** (bloco menor + resto em
camadas). (Não muda o default; só registra que a legibilidade da DSM depende do
modo — reforça o ponto do laudo 0034.)

---

## Restrições estruturais

- **L1 puro, zero deps externas** — implementar condensação + ordem topológica
  **à mão** (sem `petgraph`), como o Tarjan foi à mão no laudo 0031.
- **Reusar o cálculo de SCC do laudo 0031.** O `detectar_ciclos` devolve SCCs
  ≥2; a condensação precisa da **partição completa** (SCCs ≥2 **+** cada nó
  restante como SCC unitário). Refatorar para expor a partição completa (ou uma
  função interna compartilhada), **não** reimplementar o Tarjan.
- **Ordem determinística**: empate na topológica resolvido por `path`; membros de
  SCC ordenados por `path`. Duas extrações → ordem idêntica.
- **Respeita `escopo` e `modo_uses`** (a ordem é do grafo já no escopo/modo
  escolhido).
- **Emitir ordem + blocos** no `--estrutura` (texto e JSON). As dependências já
  saem (laudo 0031). **Nenhuma grade/matriz desenhada** — mas o dado tem que ser
  suficiente para montá-la.
- **Não tocar o fork, a spec, a E2, nem `raio`/`ranking`.**

---

## Fase 1 — Leitura

1. **Ler**: `lente_estrutura` (o interno do `detectar_ciclos` — ele expõe a
   partição completa de SCCs ou só os ≥2? Onde refatorar para reusar);
   `agregar_por_modulo` (o grafo de módulos que sai); `analisar_estrutura` +
   `EstruturaModulos`; a saída `--estrutura` (texto e JSON) e o catálogo.
2. **Confirmar**: a condensação (colapsar SCCs) é um DAG — sanidade para a ordem
   topológica funcionar (se sobrar ciclo na condensação, o SCC não foi colapsado
   direito).

**Reportar**: onde a partição completa de SCC é obtida/refatorada, e o ponto onde
a ordem entra no `EstruturaModulos` e na saída.

---

## Fase 2 — Conserto

### `lente_estrutura`

- Refatorar o cálculo de SCC para expor a **partição completa** (SCCs ≥2 +
  unitários) — o `detectar_ciclos` continua devolvendo só os ≥2 (filtra a
  partição), sem mudar o que já entrega.
- `ordenar_dsm(grafo: &Grafo) -> OrdemDsm` (nome a confirmar): partição completa
  → condensação → ordem topológica (à mão, empate por `path`) → expandir membros
  de SCC (por `path`). `OrdemDsm` sugerido:

```rust
pub struct OrdemDsm {
    pub ordem: Vec<…>,        // módulos na ordem da DSM (por id ou path)
    pub blocos: Vec<Vec<…>>,  // os SCCs ≥2, cada um um grupo de módulos (na ordem)
}
```

### Fiação

`EstruturaModulos` ganha a ordem e os blocos (de `ordenar_dsm`).
`analisar_estrutura` chama `ordenar_dsm` sobre o grafo de módulos já agregado (no
escopo/modo escolhido). Re-exporta o tipo se a CLI precisar.

### CLI (`--estrutura`) + catálogo

- **JSON**: campos novos — `ordem` (array de paths na ordem da DSM) e `blocos`
  (array de grupos de paths = os ciclos). As `dependencias` já saem (laudo 0031).
  Ordem + dependências + blocos = a matriz como dado.
- **Texto**: listar os módulos **na ordem** (não alfabética), marcando os que
  pertencem a um bloco de ciclo (ex.: um rótulo de bloco). O `modo_uses`/escopo
  continuam declarados no cabeçalho (laudos 0030/0034).
- Rótulos no **catálogo**.

---

## Critérios de Verificação

```
Dado um grafo de módulos acíclico (A→B→C)
Quando ordenado
Então a ordem é uma ordem topológica válida (toda dep aponta para o mesmo lado);
  determinística (empate por path)

Dado um grafo com um ciclo (A→B→A, e C→A)
Quando ordenado
Então {A,B} é um bloco; C ordenado em relação ao bloco; a ordem é a topológica da condensação

Dado a condensação de qualquer grafo de módulos
Então ela é um DAG (colapsar SCC remove os ciclos) — a ordem topológica termina

Dado duas extrações do mesmo crate
Então a ordem é idêntica (empate por path)

Dado o egui (E2E #[ignore]) em --so-referencia
Então o bloco de 42 módulos é um bloco da DSM; os outros 69 ficam em camadas; ordem determinística

Dado a ordem + dependencias emitidas
Então um consumidor monta a grade N×N: eixos na ordem, célula (i,j) marcada se (mod_i, mod_j) ∈ dependencias
  (teste de consumidor: a grade reconstruída bate com as dependências)

Dado raio/ranking
Então inalterados (não-regressão)
```

Casos a cobrir:

- **Unidade, puros**: ordem de um DAG (A→B→C); ciclo de 2 vira bloco; ciclo de 3;
  nó isolado (SCC unitário sem arestas) ordenado; determinismo (empate por path).
- **Consumidor**: a partir de `ordem` + `dependencias`, reconstruir a grade e
  conferir contra as `dependencias` (a matriz como dado é suficiente).
- **E2E `#[ignore]`**: egui nos dois modos — bloco de 42 (`--so-referencia`) /
  bloco de 85 (`todas`); ordem determinística.
- **Não-regressão**: `raio`/`ranking` idênticos; campos antigos do `--estrutura`
  intactos; suíte verde.
- **Pureza L1**: `cargo tree -p lente_estrutura` só `lente_core` (sem `petgraph`).

---

## Resultado esperado

- O ordenamento da DSM: módulos ordenados de modo que as dependências fiquem de
  um lado da diagonal e os SCCs apareçam como blocos na diagonal. Emitido como
  **dado** (`ordem` + `blocos`) no `--estrutura`, ao lado das `dependencias` — a
  **matriz como dado**.
- A matriz **visual** lê isso depois; um agente (a camada de contexto estrutural)
  também consome.
- `--so-referencia` dá uma matriz mais legível (bloco menor) que `todas`.
- **Laudo** registrando: a ordem no egui (o bloco de 42 + o resto em camadas), o
  determinismo, e que o dado basta para montar a grade.

---

## O que NÃO entra

- **A matriz visual / tela**: lê este dado, depois. Não é este prompt.
- **A trilha local (consumir `position`)**: separada.
- **Mudar o default de `modo_uses`, `raio`/`ranking`, o fork, a spec, folhas.**
- **Multi-nível (crate-a-crate, item)**: a ordem é genérica (roda sobre qualquer
  grafo de módulos), mas usada no nível módulo aqui — os outros níveis reusam
  depois (o fractal), sem construí-los agora.

---

## Observação metodológica

O tijolo L1 primeiro (o cálculo difícil), emitido como **dado**, antes de
qualquer pixel — a matriz como dado serve o humano (a tela futura) **e** o agente
(a camada de contexto estrutural, a curiosidade de IA), e a tela vira pura
apresentação depois (como o protótipo de UI já mostrou).

A ordem é o exemplo mais claro do valor da lente **acima** das ferramentas de
texto: uma ordem topológica sobre uma condensação é puro cálculo sobre o grafo
**dirigido e resolvido** — nenhum grep/ripgrep/tree-sitter a produz, porque
nenhum tem direção nem resolução. Construir no nível módulo, provar no egui; a
mesma ordem generaliza para crate-a-crate e item depois (o fractal), sem
construí-los agora.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Primeiro tijolo da DSM: `lente_estrutura::ordenar_dsm` — condensação dos SCCs (partição completa, reusando o cálculo do laudo 0031) + ordem topológica à mão (determinística, empate por `path`). `analisar_estrutura`/`EstruturaModulos` ganham `ordem` + `blocos`; `--estrutura` emite ambos (texto na ordem + JSON), ao lado das `dependencias` já existentes — a matriz como dado. Respeita escopo/`modo_uses`; mais legível em `--so-referencia` (bloco de 42 vs 85). L1 puro (sem `petgraph`). `raio`/`ranking` intocados; tela visual fica para depois. | `09_estrutura/src/lib.rs`, `04_wiring/src/lib.rs`, `02_shell/cli/src/*`, `02_shell/catalogo/src/lib.rs`, `00_nucleo/lessons/0035-ordenamento-dsm.md` |
