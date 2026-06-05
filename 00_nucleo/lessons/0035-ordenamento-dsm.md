# Laudo de Execução — Prompt 0035 (Ordenamento da DSM)

**Camada**: L5 (laudo)
**Data**: 2026-06-04
**Prompt executado**: `00_nucleo/prompt/0035-ordenamento-dsm.md`
**Estado**: `EXECUTADO` — `lente_estrutura::ordenar_dsm` (condensação dos
SCCs + Kahn determinístico, à mão); `EstruturaModulos` ganha `ordem` +
`blocos`; CLI `--estrutura` emite a **matriz como dado** (texto humano +
JSON). Reproduz no egui: bloco de 42 contíguo, com 55 módulos "abaixo" e
14 "acima" — forma clássica de DSM Lattix. 206 verdes (+12) + 21 ignored
(+1). Pureza do L1 mantida (Kahn iterativo à mão, sem `petgraph`);
subprocessos do cargo continuam dois únicos (0023).

---

## A pergunta e a entrega

> Como ordenar os módulos para a DSM ter dependências de um lado da
> diagonal e ciclos como blocos densos?

**Condensação dos SCCs → ordem topológica do DAG resultante** — o cálculo
que separa a lente das ferramentas de texto (grep/ripgrep/tree-sitter):
ordem topológica só existe sobre grafo dirigido e resolvido. Entregue:

```
$ lente --pacote lente_core --estrutura --text
Estrutura de módulos (escopo: completo, uses: todas) — 7 módulos, 0 ciclos:

Ciclos:
  (nenhum ciclo entre módulos)

Dependências módulo → módulo:
  lente_core::domain::raio → core::fmt
  lente_core::domain::raio → lente_core::entities::grafo
  lente_core::entities::grafo → core::fmt

Ordem da DSM (topológica + blocos):
   lente_core
   lente_core::domain
   lente_core::domain::raio
   lente_core::entities
   lente_core::entities::grafo
   core::fmt
   lente_core::entities::veredito
```

`domain::raio` aparece **antes** de `entities::grafo` e de `core::fmt` —
ambas as deps "apontam para frente" na DSM. Coerente com o ADR-0002 D2
("o cálculo do raio depende do tipo de dados"): a hierarquia
arquitetural fica visível na ordem.

JSON: campos novos `ordem` (array de paths) e `blocos` (array de arrays
de paths), ao lado das `dependencias` que já saíam desde o laudo 0031.
Ordem + dependências + blocos = **a matriz como dado**, suficiente para a
tela futura ou para um agente reconstruir a grade N×N.

---

## Achado decisivo no egui

`lente --pacote egui --estrutura --so-referencia`:

| | Valor |
|---|---|
| Módulos | 111 |
| Dependências | 386 (laudo 0033) |
| Ciclos | 1 (laudo 0033) |
| Maior SCC | **42 módulos** (laudo 0033) |
| **Posição do bloco em `ordem`** | **[55..97]** (42 módulos contíguos) |
| Módulos **antes** do bloco | **55** (deps externas + stdlib: `accesskit`, `alloc::fmt`, `color_hex`, `core::f32`, `ecolor`, …) |
| Módulos **depois** do bloco | **14** (módulos derivados: `egui::cache::cache_*`, `egui::data::*`, `egui::id_salt`, …) |

Forma clássica de DSM Lattix:

```
índice  0 … 54  | 55 …………… 96 | 97 … 110
        camada    bloco          camada
        de base   denso (SCC)    superior
        (deps,    do egui        (caches,
        stdlib)                  data, id)
```

A lente revela: a **maior parte do código do egui** (38% dos módulos)
está concentrada num único anel mutuamente dependente. Os outros 62% se
dividem em camadas limpas — uma "abaixo" (utilidades) e outra "acima"
(consumidores do core). Esse é exatamente o **achado-cabeçalho** que uma
ferramenta Lattix entrega, agora pelo terminal, sem matriz visual.

`e2e_dsm_egui_bloco_de_42_e_contiguo` ancora — afirma o `42` e a
contiguidade contra o egui real.

---

## Fase 1 — Refatoração e leitura

### Refatoração do Tarjan

O `detectar_ciclos` (laudo 0031) **já tinha** o Tarjan, mas filtrava SCCs
≥ 2 e descartava singletons. A condensação precisa da **partição
completa** (cada nó num SCC, possivelmente unitário). Extraí o Tarjan
para `tarjan_sccs(grafo, path_por_id) -> Vec<Vec<usize>>` —
`pub(crate)`, sem mudar `detectar_ciclos` (que agora chama o helper e
filtra). API pública intacta.

### Onde a ordem entra

`EstruturaModulos` ganha **dois campos novos**: `ordem: Vec<Path>` e
`blocos: Vec<Vec<Path>>`. `modulos` (alfabético) e `dependencias`
(alfabético) **continuam** — compatibilidade total com clientes do
laudo 0031. A `analisar_estrutura` chama `ordenar_dsm(&agg)` sobre o
grafo de módulos **já agregado**, no escopo/modo escolhido — escopo e
`modo_uses` fluem para a ordem sem mudanças.

---

## Fase 2 — Algoritmo

### `ordenar_dsm` em 4 passos

1. **`tarjan_sccs`** → partição completa (cada nó num SCC ≥ 1).
2. **DAG da condensação**: aresta `SCC(a) → SCC(b)` se há aresta `Uses
   from→to` no original com `from ∈ a`, `to ∈ b`, `a ≠ b`. Deduplicado
   via `HashSet<(usize, usize)>`.
3. **Kahn iterativo** com fila `BTreeSet<(menor_path_do_SCC, índice)>` —
   determinística por path ascendente. Cada vez que um SCC vai para a
   ordem, seus vizinhos na condensação têm `grau_entrada` decrementado;
   chegam a zero → entram na fila.
4. **Expansão**: cada SCC na ordem topológica emite seus membros (por
   `path` ascendente, convenção). SCCs ≥ 2 viram blocos.

```rust
pub struct OrdemDsm {
    pub ordem: Vec<Path>,
    pub blocos: Vec<Vec<Path>>,
}
pub fn ordenar_dsm(grafo: &Grafo) -> OrdemDsm
```

### Pureza L1 mantida

```
$ cargo tree -p lente_estrutura --depth 1
lente_estrutura v0.0.0
└── lente_core v0.0.0
```

Zero deps externas. `BTreeSet` (stdlib) basta para a fila ordenada.
Sem `petgraph` — Kahn iterativo segue o padrão dos demais algoritmos
manuais do projeto (Tarjan no 0031, Tarjan iterativo aqui também).

### Genericidade — pré-requisito do fractal

`ordenar_dsm` opera sobre **qualquer** `Grafo`, igual ao `detectar_ciclos`
do laudo 0031. O nível módulo é onde **usamos** aqui; quando o fractal
nascer (crate-a-crate, item), a mesma peça serve em outra escala. Não
construído agora — registrado como horizonte que o desenho permite.

---

## Verificação

### Suíte (sem ignored)

| Crate | Verdes | Δ vs 0034 |
|-------|--------|-----------|
| **lente_estrutura** | **23** | **+7** (DAG linear / ciclo-2 / ciclo+dep / 2 ciclos disjuntos / isolado / determinismo / consumidor-grade) |
| **lente_wiring** | **20** | **+2** (`estrutura_emite_ordem_e_blocos_do_dsm`, `consumidor_reconstroi_grade_n_x_n_a_partir_da_saida`) |
| **lente_cli** | **38** | **+3** (saida: json ordem+blocos / texto com marcador / texto sem blocos sem marcador) |
| Outros | inalterados | 0 |
| **Total** | **206** | **+12** |

### Ignored (todos verdes)

| | Δ |
|---|---|
| lente_infra | 8 |
| lente_filtro (tests/) | 3 |
| **lente_wiring** | **7** (+1: `e2e_dsm_egui_bloco_de_42_e_contiguo`) |
| lente_cli | 3 |
| **Total** | **21** (+1) |

E2Es rodados: todos verdes. **`e2e_dsm_egui_bloco_de_42_e_contiguo`**
afirma:
- existe **exatamente um** bloco com 42 membros;
- os 42 membros são **contíguos** em `ordem` (fatia consecutiva do
  vetor).

### Subprocessos do cargo (invariante 0023)

```
$ grep -rn 'Command::new("cargo")' --include "*.rs"
03_infra/src/fork.rs:117      # cargo modules
03_infra/src/metadata.rs:170  # cargo metadata
```

Dois únicos, intocados. Prompt 0035 não introduz subprocesso.

### Teste-consumidor: "matriz como dado" suficiente

Dois testes (um em `lente_estrutura`, um em `lente_wiring`) **reconstroem
a grade N×N** a partir de `ordem` + `dependencias`/`edges` e conferem que
ela bate. Prova ponta-a-ponta que o dado emitido **basta** para a tela
futura ou para um agente:

```rust
let n = ordem.len();
let idx: HashMap<&str, usize> = ordem.iter().enumerate()
    .map(|(i, p)| (p.as_str(), i)).collect();
let mut grade = vec![vec![false; n]; n];
for d in &dependencias {
    grade[idx[d.de.as_str()]][idx[d.para.as_str()]] = true;
}
// grade está pronta — não precisou de nada além de `ordem` + `dependencias`.
```

---

## Decisões tácitas

### D1 — `modulos` (alfabético) **e** `ordem` (topológica) em paralelo

Alternativa rejeitada: substituir `modulos` por `ordem`. Razão: clientes
do laudo 0031 podem depender de `modulos` ordenado alfabético (ex.: o
protótipo de UI, laudo 0029). Adicionar `ordem` é **aditivo**; clientes
novos preferem `ordem` (para DSM); clientes antigos continuam usando
`modulos`. Zero quebra.

### D2 — Kahn (BFS) em vez de DFS-based topological sort

Tarjan já produz SCCs em ordem-reversa-topológica natural. Eu poderia
simplesmente **inverter** o output do Tarjan e teria uma ordem
topológica grátis. Por que não:

- **Determinismo na **escolha** de qual SCC pop'a primeiro**: Tarjan
  ordena por "ordem de fechamento", que depende do path da DFS — só
  determinístico se a adjacência for ordenada (e é, no nosso código).
  Mas o **empate entre SCCs** não tem semântica fácil de explicar.
- **Kahn com chave `menor_path_do_SCC`**: o critério de empate é
  **explícito** e legível ("quando dois SCCs estão prontos, vai o
  alfabeticamente menor"). Mais fácil de testar e de explicar no
  laudo.

Custo: uma passada extra para montar o DAG da condensação. Trivial
(O(E)). Vale a clareza.

### D3 — Membros internos de SCC ordenados por `path`

Os membros de um SCC são **mutuamente cíclicos** — pela definição, nenhuma
ordem interna é "melhor". Convenção: **alfabética por `path`**. Coerente
com `Ciclo.modulos` (que o laudo 0031 já ordenava assim) e com o resto
do projeto. Testes ancoram.

### D4 — Aresta entre SCC e ele mesmo é descartada

`if ia == ib { continue; }` na construção da condensação: uma aresta
intra-SCC (mesmo nó-de-condensação como origem e destino) **não** vira
aresta. Caso típico: dentro de um SCC todo nó é conectado a todo nó
(transitivamente); a versão "achatada" da condensação não tem self-loops
— porque a condensação é por construção um DAG. Defesa em profundidade.

### D5 — Texto: seção "Ordem da DSM" + marcador `◆` para bloco

Decisão de UX:
- Título de seção: `Ordem da DSM (topológica + blocos):` — declara o
  que está sendo mostrado.
- Cada módulo numa linha; módulos de algum bloco recebem `◆` como
  prefixo. Módulos livres recebem espaço simples.
- O `◆` é Unicode comum (U+25C6), não emoji. Coerente com `→` e `—` que
  o texto já usa.

Para a vista visual completa (matriz N×N pintada), a tela futura
consome o JSON. O texto é o atalho legível na CLI.

### D6 — `EstruturaModulos.modulos` continua alfabético, `ordem` é topológico

Compatibilidade dupla: clientes que faziam ordem própria (`modulos.sort()`)
não precisam mudar; clientes novos (a tela DSM, este laudo) usam `ordem`.
O JSON tem **ambos**: legível para humanos (modulos alfabéticos) e
estrutural para máquinas (ordem topológica).

### D7 — `tarjan_sccs` é `pub(crate)`, não `pub`

A função interna é detalhe de implementação. Pode haver chamadores
internos no futuro (ex.: análise de "qual SCC contém este nó"), mas
expor agora seria estrutura antes do uso pedir. `detectar_ciclos` e
`ordenar_dsm` permanecem como a API pública do crate.

### D8 — Modo `Todas` vs `SoReferencia`: legibilidade da matriz

A DSM funciona nos dois modos, mas (como o prompt previu) é **muito mais
legível em `--so-referencia`**:

| | `Todas` | `--so-referencia` |
|---|---|---|
| Bloco grande | 85 módulos | 42 módulos |
| Camada de base | 26 módulos | 55 módulos |
| Camada superior | 0 módulos | 14 módulos |

No `Todas`, o bloco de 85 cobre 77% do crate — a matriz fica praticamente
toda **uma diagonal-de-bloco**. No `SoReferencia`, o bloco encolhe para
38% e duas camadas limpas aparecem: dependências externas/stdlib
(`accesskit`, `core::*`, `ecolor`, etc.) **abaixo** do bloco, módulos
derivados (`egui::cache::*`, `egui::data::*`, `egui::id_salt`) **acima**.

Coerente com o que o laudo 0033 já tinha dito: o `import` (Limite 4)
infla o ciclo. Aqui isso fica **visual** — duas camadas aparecem ao
filtrar.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0035 |
|-----------|-----------------|
| Vista global do projeto (origem Lattix/Structure101) | **Tijolo L1 entregue** — a matriz como dado. |
| Forma de DSM legível para `egui` | **Coberta** — bloco de 42 + 55 abaixo + 14 acima, contígua e determinística. |
| Tela visual da DSM | **Aberta** — consome este JSON; trilha de UI. |
| Multi-nível (crate-a-crate, item — fractal) | **Aberta** — `ordenar_dsm` genérico, pronto para o nível. |
| Filtro de folhas comportamentais (Limite 3) | **Aberta** — trilha separada. |

---

## O que NÃO mudou

- **`raio`/`ranking`**: zero toques. Não leem `ordem`/`blocos`.
- **Fork** (`cargo-modules`): zero toques.
- **Spec, ADRs**: zero toques. O ordenamento é derivação natural do
  grafo `Uses`+`Owns` já descrito na spec.
- **Modos `--alvo`/`--alvo-id`/`--ranking`** na CLI: comportamento
  inalterado (a nova saída só aparece no `--estrutura`).
- **`modulos`** em `EstruturaModulos`: continua alfabético (compat com
  laudo 0031 e protótipo de UI 0029).
- **`detectar_ciclos`** (laudo 0031): API pública e comportamento
  intactos; só refatorado internamente para reusar `tarjan_sccs`.
- **Subprocessos do cargo** (invariante 0023): dois únicos.

---

## Observação metodológica

**O tijolo L1 primeiro, emitido como dado, antes de qualquer pixel.** A
matriz visual da DSM vai consumir este JSON — exatamente como o protótipo
de UI (laudo 0029) consumiu o JSON do raio/ranking. A trilha
"calcular primeiro, desenhar depois" se mantém: o cálculo difícil
(ordem topológica sobre condensação) está pronto, e a apresentação fica
trivializada.

A ordem é o **exemplo mais claro** do valor da lente acima das
ferramentas de texto: nenhum `grep`/`ripgrep`/`tree-sitter` produz uma
ordem topológica, porque nenhum tem **direção** (eles tratam dependências
como ocorrências de texto). Aqui o grafo é **dirigido e resolvido**
(laudos 0006/ADR-0004/0034), e a ordem cai natural do Tarjan + Kahn.

A propriedade fractal está preservada: `ordenar_dsm` é genérico, opera
sobre qualquer `Grafo`. Quando crate-a-crate (workspace) ou item (raio
estrutural) virarem prompts próprios, a mesma peça serve — sem
construí-los agora.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Primeiro tijolo da DSM: `lente_estrutura::ordenar_dsm` — condensação dos SCCs (partição completa via `tarjan_sccs` reusado de `detectar_ciclos` do laudo 0031) + ordem topológica iterativa por Kahn (BTreeSet ordenado por menor-path-do-SCC, determinístico). `EstruturaModulos`/`analisar_estrutura` ganham `ordem` + `blocos` (`modulos` alfabético preservado para compat). `--estrutura` emite a matriz como dado: texto traz seção "Ordem da DSM" com marcador `◆` em módulos de bloco; JSON adiciona `ordem` + `blocos` ao lado de `dependencias`. Respeita escopo/`modo_uses` (a ordem é do grafo já no escopo/modo escolhido). Reproduzido no egui em `--so-referencia`: 111 módulos com bloco de 42 contíguo (ordem[55..97]), 55 módulos antes (deps externas + stdlib) e 14 depois (módulos derivados) — forma clássica de DSM Lattix com camadas. L1 puro (Kahn à mão, sem `petgraph`); `raio`/`ranking` intocados; dois subprocessos do cargo (0023). 206 verdes + 21 ignored. | `09_estrutura/src/lib.rs`, `04_wiring/src/lib.rs`, `02_shell/catalogo/src/lib.rs`, `02_shell/cli/src/saida.rs`, `00_nucleo/lessons/0035-ordenamento-dsm.md` |
