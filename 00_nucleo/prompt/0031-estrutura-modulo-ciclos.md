# Prompt: Estrutura ao nível de módulo + detecção de ciclos (`lente_estrutura`)

**Camada**: L1 (crate novo) + L4 (fiação) + L2 (CLI + catálogo)
**Criado em**: 2026-06-03
**Estado**: `PROPOSTO`
**Decisões de origem**: a ideia do projeto nasceu de querer um Lattix LDM /
Structure101 para Rust — ferramentas de **arquitetura**, que respondem a uma
pergunta **global** ("qual a forma do sistema, e onde estão os ciclos?") via uma
DSM. A lente até aqui responde a pergunta **local** (o raio). O autor priorizou
a global, para decidir refatoração. Este é o primeiro tijolo dela: **agregar o
grafo de itens ao nível de módulo** (pela relação `Owns`) e **detectar ciclos**
entre módulos — o resultado-cabeçalho de uma ferramenta de arquitetura.
**Pré-requisito**: `lente_core` (`Grafo`, relações `Owns`/`Uses`, `Kind::Mod` e
`Kind::Crate`); `obter_grafo(fonte, escopo)` do laudo 0030; o roteamento de modo
da CLI.
**Posição**: primeiro tijolo da trilha global (estilo Lattix), no nível
**módulo, dentro de um crate**. A DSM visual, o nível crate-a-crate (workspace),
e a navegação por níveis (o "fractal") vêm **depois**, sobre esta base.
**Arquivos afetados (a confirmar na Fase 1)**: crate novo
`09_estrutura/lente_estrutura`; `04_wiring/src/lib.rs`; `02_shell/cli/src/*`;
`02_shell/catalogo/src/lib.rs`; `Cargo.toml` raiz (members); testes.

---

## Contexto

O grafo tem a matéria-prima das ferramentas de arquitetura: `Owns` é a
hierarquia de contenção (crate contém módulos, módulo contém itens), `Uses` é a
dependência. Este prompt produz a vista global no nível certo:

1. **Agregar ao nível de módulo.** Para cada aresta `Uses` item_x → item_y,
   achar o módulo que contém x e o que contém y; se forem módulos diferentes,
   isso é uma dependência **módulo→módulo**. O resultado é um grafo pequeno
   (dezenas de módulos em vez de milhares de itens) — legível por um humano.
2. **Detectar ciclos.** Componentes fortemente conexos (SCC) de tamanho ≥ 2 no
   grafo de módulos: grupos de módulos que dependem uns dos outros em volta.
   Ciclo entre módulos é cheiro de arquitetura — o uso número um do Lattix.

### O nível importa

Item (o grafo cru, 3694 nós no egui) é granular demais: matriz ilegível, e
ciclo entre itens é quase sempre ruído (recursão mútua etc.). **Módulo** é onde
ciclo de arquitetura é legível e acionável. Este prompt faz **módulo, dentro de
um crate**. Crate-a-crate (workspace) precisa extrair vários crates e combinar
— etapa posterior.

### Nota fractal (horizonte, NÃO construir agora)

A **mesma** operação serve em qualquer nível, porque em todo nível há as mesmas
duas relações (conter e usar): a detecção de ciclos roda sobre **qualquer**
`Grafo`, e a agregação **produz** um `Grafo`. Então o nível crate-a-crate e o
nível item reusam estas peças depois, de graça. Mas **não** construir navegação
multi-nível agora — é estrutura antes do uso pedir. Construir **um** nível,
prová-lo contra o egui, e os outros viram aplicação da mesma peça.

### Escopo (do laudo 0030)

Esta vista aceita `escopo` (default `Completo`, como o resto). Fato a confirmar
e usar: **ciclos entre os seus módulos são invariantes ao escopo** — a stdlib
nunca depende do seu código, então um módulo de stdlib (`core::fmt`, etc.) é
sempre um **sorvedouro** (depende-se dele, ele não depende de você) e **nunca**
fecha um ciclo de volta no seu código. O escopo só muda se módulos de stdlib
**aparecem na listagem** de módulos, não os ciclos. (Mesma natureza da
invariância do montante, laudo 0030.)

---

## Restrições estruturais

- **L1 puro, zero deps externas.** Implementar SCC **à mão** (Tarjan ou
  Kosaraju) — **sem** `petgraph` nem outra lib. Igual ao padrão de
  `investiga`/`resolve`/`filtro`/`ranking`.
- **Reusar o tipo `Grafo`** para o grafo de módulos: nós = os `No` de módulo
  (`Kind::Mod`/`Kind::Crate`), arestas = as dependências agregadas (`Uses`
  módulo→módulo). `id` dos nós de módulo **preservados**.
- **Ciclos genéricos sobre qualquer `Grafo`** (amigável ao fractal), mas
  **usados** só no nível módulo aqui.
- **Respeita `escopo`** (default `Completo`); ciclos invariantes ao escopo.
- **Saída em texto é o entregável**; JSON com forma que uma **DSM futura**
  consome. **Nenhuma matriz/UI** neste prompt.
- **Saída determinística** (SCCs e seus membros ordenados).
- **Modo novo na CLI**, mutuamente exclusivo com `--ranking`/`--alvo`/`--alvo-id`.
- **Não toca o fork, os tipos `Grafo`/`No` (só consome), nem a E2** (quarentena).

---

## Fase 1 — Leitura e medição contra o egui (obrigatória)

1. **Ler**: `Grafo` (`Owns`/`Uses`; `Kind::Mod`/`Kind::Crate`); `obter_grafo`
   (escopo); o roteamento de modo da CLI; o catálogo.
2. **Confirmar "módulo que contém o item"**: subir a cadeia de `Owns` até o
   `Mod`/`Crate` mais próximo. Verificar em nós reais: um método cujo pai `Owns`
   é um `struct`, cujo pai é um módulo — a subida tem que **alcançar** o módulo.
   Tratar: a raiz (crate), itens direto sob o crate, e nós órfãos (Isolados).
3. **Medir contra o egui** (o ponto): agregar o egui ao nível de módulo e
   **reportar** — quantos módulos (é legível? dezenas?), quantos ciclos, e
   **quais** ciclos (que módulos). A vista no nível módulo é legível e útil?
   Mesma disciplina da medição do laudo 0021: provar contra dado real antes de
   declarar útil.
4. **Confirmar a invariância ao escopo dos ciclos** (módulos de stdlib são
   sorvedouros; não entram em ciclo com o seu código).

**Reportar no laudo**: a regra de "módulo contenedor", a medição do egui
(módulos, ciclos), e a invariância confirmada.

---

## Fase 2 — Conserto

### Crate novo `lente_estrutura`

```
09_estrutura/
├── Cargo.toml   # dep: lente_core; zero deps externas
└── src/lib.rs   # agregar_por_modulo + detectar_ciclos + testes
```
Somar `"09_estrutura"` aos `members` do `Cargo.toml` raiz. (Se o gerador
preferir, ciclos podem virar crate próprio `lente_ciclos` por serem genéricos —
decisão da Fase 1; um crate cobrindo os dois também é coerente.)

- `agregar_por_modulo(grafo: &Grafo) -> Grafo`: nós = módulos; aresta
  módulo(x)→módulo(y) quando item x `Uses` item y e módulo(x) ≠ módulo(y)
  (dependência intra-módulo **não** vira aresta). `id`/`path` dos módulos
  preservados. (Manter a hierarquia `Owns` entre módulos no resultado é
  **opcional** — útil para a navegação futura, dispensável para a saída deste
  prompt; decisão do gerador.)
- `detectar_ciclos(grafo: &Grafo) -> Vec<Ciclo>`: SCC à mão sobre as arestas
  `Uses`; devolve os SCCs de tamanho ≥ 2 (`Ciclo` = o conjunto de módulos do
  SCC), ordenados deterministicamente.

### Fiação

`analisar_estrutura(fonte, escopo) -> Result<EstruturaModulos, ErroLente>` (nome
a confirmar): `obter_grafo(fonte, escopo)` → `agregar_por_modulo` →
`detectar_ciclos`. Reusa `obter_grafo`. Re-exporta o tipo de resultado para a
CLi (padrão do laudo 0027/0030).

### CLI

- Flag de modo `--estrutura` (nome a confirmar), mutuamente exclusiva com
  `--ranking`/`--alvo`/`--alvo-id`. Respeita `--filtrar-stdlib` (escopo) e
  `--text`/`--json`.
- **Texto**: declara o escopo no cabeçalho; lista os **ciclos** em destaque (o
  resultado-cabeçalho); lista as dependências módulo→módulo (ou, no mínimo, os
  módulos e de quem cada um depende).
- **JSON** (forma para a DSM futura): `{ escopo, modulos: [...],
  dependencias: [{de, para}], ciclos: [[modulos]] }`.
- Rótulos no **catálogo**.

---

## Critérios de Verificação

```
Dado um grafo sintético com um ciclo de módulos conhecido (A usa B, B usa A via itens)
Quando detectar_ciclos roda sobre o grafo agregado
Então encontra o SCC {A, B}

Dado um grafo de módulos acíclico
Quando detectar_ciclos roda
Então nenhum ciclo

Dado a agregação
Quando um item de A usa um item de B
Então há aresta módulo A→B; uses intra-módulo NÃO viram aresta; ids dos módulos preservados

Dado --estrutura com e sem --filtrar-stdlib
Então os ciclos são IGUAIS (invariantes ao escopo); a listagem de módulos inclui/exclui módulos de stdlib conforme o escopo

Dado --estrutura
Quando a CLI parseia junto com --ranking ou --alvo
Então erro de conflito claro (modos mutuamente exclusivos)

Dado --estrutura --json
Então a saída tem { escopo, modulos, dependencias, ciclos }

Dado o egui (E2E #[ignore])
Quando analisado
Então reporta a contagem de módulos e os ciclos (ancorado)
```

Casos a cobrir:

- **Unidade, puros** (grafos à mão): ciclo de 2 módulos; ciclo de 3 (A→B→C→A);
  acíclico; agregação (uses item→item lifted; intra-módulo ignorado; ids
  preservados); módulo contenedor de um item aninhado (método sob struct sob
  módulo); nó órfão/Isolado.
- **E2E `#[ignore]`** contra o egui: contagem de módulos e ciclos, ancorados; e
  a invariância ao escopo (ciclos iguais com/sem filtro).
- **Não-regressão**: modos `--alvo`/`--ranking` inalterados; suíte verde.
- **Pureza L1**: `cargo tree -p lente_estrutura` mostra só `lente_core` (sem
  `petgraph`).

---

## Resultado esperado

- Primeiro tijolo global: estrutura no nível módulo + ciclos, em texto na CLI,
  com JSON moldado para uma DSM futura.
- Medido contra o egui (módulos, ciclos) — provado antes de declarado útil.
- As peças (ciclos sobre qualquer `Grafo`; agregação que produz `Grafo`)
  reutilizáveis para crate-a-crate e item depois (o fractal), **sem** construí-las
  agora.
- **Laudo** registrando: a regra de módulo contenedor, a medição do egui, e a
  invariância ao escopo dos ciclos.

---

## O que NÃO entra

- **DSM visual / matriz / UI**: consome este JSON depois; não é este prompt.
- **Nível crate-a-crate (workspace)**: precisa de extração multi-crate; etapa
  posterior.
- **Navegação multi-nível (o fractal)**: horizonte; construir um nível primeiro.
- **A trilha local** (mapear um diff para nós; panorama na confirmação do
  agente): trilha separada, não desenvolvida aqui.
- **`petgraph` ou outra lib de grafo**: pureza L1 — SCC à mão.
- **Enriquecer o JSON do raio (Achado 1)**, **filtro de folhas (Limite 3)**,
  **remoção da E2**: outras trilhas.

---

## Observação metodológica

A origem do projeto é a vista global (Lattix/Structure101); este é o primeiro
tijolo dela, no nível onde ciclo de arquitetura é legível (módulo). Medido
contra o egui **antes** de declarar útil — mesma disciplina do laudo 0021.

O "fractal" (a mesma vista em outros níveis) é o **horizonte que o desenho
permite**, não o que se constrói agora: a régua de zoom tem o local numa ponta
(o item e sua vizinhança, o raio) e o global na outra (crate e módulo) — o mesmo
grafo em escalas diferentes. Construir um nível, prová-lo, e os outros viram a
mesma peça noutra escala. "Não estruturar antes do uso pedir" aplicado ao zoom:
não construir o zoom infinito antes de um nível provar valor.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-03 | Primeiro tijolo da vista global (estilo Lattix): crate L1 `lente_estrutura` com `agregar_por_modulo` (grafo de itens → grafo de módulos via `Owns`) e `detectar_ciclos` (SCC à mão, ≥2 módulos); fiação `analisar_estrutura` (reusa `obter_grafo`, respeita escopo); CLI `--estrutura` (texto + JSON moldado para DSM futura). Medido contra o egui. Ciclos invariantes ao escopo. Peças reusáveis para crate-a-crate e item depois (fractal), sem construí-las agora. | `09_estrutura/{Cargo.toml,src/lib.rs}`, `04_wiring/src/lib.rs`, `02_shell/cli/src/*`, `02_shell/catalogo/src/lib.rs`, `Cargo.toml` raiz, `00_nucleo/lessons/0031-estrutura-modulo-ciclos.md` |
