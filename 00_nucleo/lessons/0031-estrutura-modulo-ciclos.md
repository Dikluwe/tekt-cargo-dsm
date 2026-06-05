# Laudo de Execução — Prompt 0031 (Estrutura nível módulo + ciclos)

**Camada**: L5 (laudo)
**Data**: 2026-06-03
**Prompt executado**: `00_nucleo/prompt/0031-estrutura-modulo-ciclos.md`
**Estado**: `EXECUTADO` — crate L1 novo `lente_estrutura` (agregação por
módulo + Tarjan iterativo à mão); fiação `analisar_estrutura(fonte, escopo)`
no `lente_wiring`; CLI `--estrutura` com JSON DSM-friendly e texto humano.
**Primeiro tijolo da vista global** (estilo Lattix LDM / Structure101).
176 verdes + 19 ignored; pureza do L1 mantida (Tarjan à mão, sem
`petgraph`); subprocessos do cargo continuam dois únicos (0023).

---

## Fase 1 — Regra do "módulo contenedor" + medição contra dado real

### Regra implementada (`mapa_modulo_contenedor`)

Para cada nó, sobe pela cadeia `Owns` (`pai = aresta.id_from` em arestas com
`relation = Owns` apontando para o nó) até encontrar o primeiro
`Kind::Mod` ou `Kind::Crate`. Casos cobertos por teste:

- Item direto sob módulo → módulo é o contenedor (`f` em `k::a` → `k::a`).
- Item aninhado em struct dentro de módulo → sobe struct até o módulo
  (`k::a::T::f` → `k::a`).
- Módulo aponta para si mesmo (idempotência).
- Crate aponta para si mesmo (raiz).
- Nó órfão (sem cadeia `Owns` até módulo) → ausente do mapa.
- Defesa contra ciclo teórico em `Owns` (não ocorre em dado real; custo
  zero do `BTreeSet<visitados>`).

### Medição contra dado real — `lente_core`

```
$ lente --pacote lente_core --estrutura --text
Estrutura de módulos (escopo: completo) — 7 módulos, 0 ciclos:

Ciclos:
  (nenhum ciclo entre módulos)

Dependências módulo → módulo:
  lente_core::domain::raio → core::fmt
  lente_core::domain::raio → lente_core::entities::grafo
  lente_core::entities::grafo → core::fmt

$ lente --pacote lente_core --estrutura --text --filtrar-stdlib
Estrutura de módulos (escopo: seu-codigo) — 6 módulos, 0 ciclos:
...
Dependências módulo → módulo:
  lente_core::domain::raio → lente_core::entities::grafo
```

`lente_core`: 7 → 6 módulos (–1 sysroot), 0 ciclos nos dois escopos.
Dependência arquitetural simples: `domain::raio → entities::grafo` (o que
o ADR-0002 D2 previa: cálculo depende do tipo de dados).

### Medição contra dado real — `egui` (o teste de fogo)

```
$ lente --pacote egui --estrutura --filtrar-stdlib | resumo
escopo: seu-codigo
módulos: 109     dependências: 862     ciclos: 1
  ciclo com 85 módulos

$ lente --pacote egui --estrutura | resumo
escopo: completo
módulos: 111     dependências: 864     ciclos: 1
  ciclo com 85 módulos    ← MESMO NÚMERO QUE NO SeuCodigo
```

**Achado-cabeçalho**: o `egui` tem **um ciclo de 85 módulos** —
≈ **76% dos módulos do crate** formam um único SCC. Inclui
`egui::context`, todos os widgets (`button`/`checkbox`/`slider`/…), os
containers (`area`/`panel`/`window`/…), o `style`, o `memory`, etc. É
exatamente o tipo de achado que uma ferramenta Lattix levanta: o crate
tem dependências cruzadas entre quase todos os seus módulos —
provavelmente herdado da natureza de UI (Context ↔ Ui ↔ Response ↔
Widgets ↔ Style ↔ Memory ↔ …). Não é bug da lente; é o **estado real**
da arquitetura do egui.

A vista no nível módulo é **legível**: 111 módulos vs ~3700 itens.
Cabe na cabeça e na tela.

### Invariância dos ciclos ao escopo — confirmada empiricamente

| Métrica | Completo | SeuCodigo | Δ |
|---------|----------|-----------|---|
| Módulos | 111 | 109 | −2 (sysroot) |
| Dependências | 864 | 862 | −2 (para sysroot) |
| **Ciclos** | **1** | **1** | **0** |
| **Membros do ciclo** | **85** | **85** | **0** |

Confirma a tese da Fase 1: sysroot é sorvedouro; não fecha ciclo de volta
no seu código. O escopo só altera o que aparece na **listagem**, não os
ciclos. Ancorado em teste de unidade `ciclos_sao_invariantes_ao_escopo`
e nos números acima contra `egui` real.

---

## Fase 2 — Implementação

### Estrutura

```
09_estrutura/                              (NOVO crate L1)
├── Cargo.toml                             # dep: lente_core
└── src/lib.rs                             # agregar + detectar_ciclos + 16 testes

04_wiring/
├── Cargo.toml                             # + lente_estrutura
└── src/lib.rs
    + pub struct DependenciaModulo {de, para}
    + pub struct EstruturaModulos {modulos, dependencias, ciclos}
    + pub use lente_estrutura::Ciclo
    + pub fn analisar_estrutura(fonte, escopo) → Result<EstruturaModulos, ErroLente>

02_shell/catalogo/src/lib.rs
    + HELP_ESTRUTURA
    + ESTRUTURA_CABECALHO / TITULO / SEM_CICLOS
    + JSON_MODULOS / DEPENDENCIAS / CICLOS / DE / PARA

02_shell/cli/src/{args,saida,main}.rs
    + flag --estrutura (conflicts_with_all com alvo/alvo-id/ranking)
    + formatar_estrutura (json + texto)
    + run_estrutura (roteamento condicional)

Cargo.toml raiz: + "09_estrutura" aos members
```

### Algoritmo — Tarjan iterativo à mão

Em `09_estrutura/src/lib.rs:detectar_ciclos`. Sem recursão (pilha
explícita de frames `(no, índice_vizinho_corrente)`) para evitar stack
overflow em grafos profundos. Determinismo garantido por:

- Adjacência ordenada por `id` alcançado (após `dedup`).
- DFS visita raízes na ordem **path-ascendente** dos nós.
- SCCs filtrados (≥ 2) e cada um ordenado lexicograficamente.
- Lista de SCCs ordenada pelo primeiro `path` de cada.

Validação: 16 testes unitários incluindo ciclo de 2, ciclo de 3 (A→B→C→A),
acíclico, dois ciclos disjuntos, auto-loop não conta, e — para ancorar a
**genericidade** — `detectar_ciclos` funciona sobre **grafo de itens**
também (não só de módulos), pré-requisito do "fractal".

### Pureza L1 confirmada

```
$ cargo tree -p lente_estrutura --depth 1
lente_estrutura v0.0.0
└── lente_core v0.0.0
```

Zero deps externas. SCC à mão, como o padrão dos demais L1
(`investiga`/`resolve`/`filtro`/`ranking`).

### JSON DSM-friendly (forma para UI futura)

```json
{
  "escopo": "seu-codigo",
  "modulos": ["egui", "egui::atomics", …],
  "dependencias": [{"de": "egui", "para": "ecolor"}, …],
  "ciclos": [["egui", "egui::context", "egui::ui", …]]
}
```

Linhas/colunas de uma matriz DSM são o `modulos`; células marcadas são
o `dependencias`; sombreamento de SCCs é o `ciclos`. Quando uma UI
nuclear (prompt próprio), consome este JSON direto.

---

## Verificação

### Suíte (sem ignored)

| Crate | Verdes | Δ vs 0030 |
|-------|--------|-----------|
| lente_core | 30 | 0 |
| lente_infra | 30 | 0 |
| lente_investiga | 17 | 0 |
| lente_resolve | 11 | 0 |
| lente_filtro (lib) | 10 | 0 |
| lente_ranking | 8 | 0 |
| **lente_estrutura** | **16** | **+16 novo** |
| **lente_wiring** | **14** | **+2** (analisar_estrutura, ciclos_invariantes) |
| lente_catalogo | 7 | 0 |
| **lente_cli** | **33** | **+6** (json/texto/sem_ciclos da saída + json/texto/e2e da main) |
| **Total** | **176** | **+24** |

### Ignored (todos passam quando rodados)

| Crate | Ignored | Δ |
|-------|---------|---|
| lente_infra | 8 | 0 |
| lente_filtro (tests/) | 3 | 0 |
| **lente_wiring** | 5 | **+2** (e2e_estrutura_lente_core_reporta, e2e_estrutura_egui_seu_codigo) |
| **lente_cli** | 3 | **+1** (e2e_estrutura_lente_core_texto) |
| **Total** | **19** | **+3** |

Todos rodados e verdes (3.4s para os E2Es).

### Subprocessos do cargo (invariante 0023)

```
$ grep -rn 'Command::new("cargo")' --include "*.rs"
03_infra/src/fork.rs:117      # cargo modules
03_infra/src/metadata.rs:170  # cargo metadata
```

Dois únicos, intocados. `lente_estrutura` é puro — zero subprocess.

---

## Decisões tácitas

### D1 — Crate único `lente_estrutura`, não `lente_ciclos` separado

O prompt deixava aberto: ciclos podem virar crate próprio por serem
genéricos. Optei por **um** crate cobrindo as duas funções porque:

- São consumidas **juntas** pela fiação (`agregar` → `detectar_ciclos`).
- O escopo do crate é "estrutura ao nível módulo + horizonte fractal";
  ciclos é parte natural disso.
- Coerente com `lente_investiga` (que tem E1+E2 num crate só).

Se um dia houver outro consumidor de SCC sem agregação, separar é
refatoração local.

### D2 — Hierarquia `Owns` entre módulos **preservada** no grafo agregado

Decisão deixada aberta no prompt. Preservei porque:

- O grafo de saída é um `Grafo` válido (mesmo tipo do crate); ter Owns
  entre módulos é coerente com a forma.
- Útil para uma DSM hierárquica (UI futura).
- Custo: insignificante (poucas arestas; ordem determinística).

`agregacao_preserva_owns_entre_modulos` ancora.

### D3 — `ciclo` de **um** nó não conta como ciclo (≥ 2)

Política do prompt: "SCC de tamanho ≥ 2". Um auto-loop (módulo que usa
a si mesmo via item dele mesmo — improvável, mas testado) **não** vira
ciclo. Coerente: o módulo é uma "esfera" da arquitetura; usar-se a si
mesmo é normal, não ciclo arquitetural. Teste
`ciclo_de_um_nodo_uses_de_si_mesmo_nao_conta_como_ciclo` ancora.

### D4 — Uses intra-módulo **absorvido** (não vira aresta)

Foco no prompt: agregação só emite **dependências entre módulos
distintos**. Uses dentro do mesmo módulo é estrutura interna, não
arquitetura. Reduz o grafo agregado dramaticamente (egui: ~14k arestas
de itens → ~800 entre módulos) sem perder informação que importa para
a vista global. Teste `agregacao_uses_intra_modulo_e_absorvido` ancora.

### D5 — `formatar_estrutura` põe **ciclos primeiro** no texto

Decisão de UX da CLI: o resultado-cabeçalho de uma ferramenta Lattix é
"onde estão os ciclos?". Ciclos vêm primeiro, dependências depois. A
contagem no cabeçalho (`{N} módulos, {C} ciclos`) torna o cheiro visível
sem precisar rolar.

### D6 — Tarjan **iterativo**, não recursivo

Tarjan canônico em livros é recursivo; aqui é iterativo (pilha de frames
`(nó, índice)`). Em grafos profundos (egui tem cadeias profundas via
`widgets::text_edit::*`), a recursão arrisca stack overflow no thread
default do Rust. Pilha explícita custa ~20 linhas a mais, ganha
robustez.

### D7 — Re-export `Ciclo` no wiring, **não** `DependenciaModulo` via re-export

`pub use lente_estrutura::Ciclo` reusa o tipo do L1 (estrutura interna
é só `Vec<Path>`). Já `DependenciaModulo` vive **no wiring** —
representação `(de, para)` é específica da fiação para o JSON DSM e
não tem contraparte no L1 (o L1 trabalha com `Aresta` cheia). Limite
honesto: tipos do L1 quando casam, tipos do L4 quando o L4 introduz a
forma.

### D8 — JSON DSM-friendly tem `ciclos: [[…]]`, não `[Ciclo]`

A representação em JSON é uma **lista de listas** (cada lista é os
membros do SCC), não uma lista de objetos `{modulos: [...]}`. Razão:
para uma DSM (matriz), o ciclo é um conjunto de módulos — apenas a
lista basta. Estrutura plana facilita parsing num front-end.

### D9 — `analisar_estrutura` reusa `obter_grafo`

Reuso direto do helper único do laudo 0030. Coerência: o escopo flui
pelo **mesmo** mecanismo para os três modos (raio, ranking, estrutura).
A invariância dos ciclos ao escopo emerge da invariância já confirmada
do montante + da política "stdlib é sorvedouro".

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0031 |
|-----------|-----------------|
| Vista global (origem do projeto: Lattix/Structure101) | **Primeiro tijolo coberto** — nível módulo, dentro de um crate. |
| Achado: o `egui` tem um SCC de 85 módulos (76% do crate) | **Registrado** — material para próxima conversa arquitetural. |
| DSM visual (matriz/UI) | **Aberta** — consome o JSON deste prompt; trilha de UI. |
| Nível crate-a-crate (workspace) | **Aberta** — extração multi-crate, depois. |
| Navegação multi-nível (fractal) | **Aberta** — peças prontas (`detectar_ciclos` genérico, `agregar` produz `Grafo`); construção fica para quando o uso pedir. |
| Achado 1 do laudo 0029 (`impactados` sem profundidade/arestas) | **Aberta** — outra trilha. |
| Filtro de "folhas comportamentais" (Limite 3) | **Aberta** — trilha separada. |

---

## O que NÃO mudou

- `lente_core` (L1): zero toques.
- `lente_filtro`/`lente_ranking`/`lente_investiga`/`lente_resolve` (L1):
  zero toques.
- Modos `--alvo`/`--alvo-id`/`--ranking`: zero toques no comportamento;
  apenas ganharam mais um modo mutuamente-exclusivo no clap.
- Fork (`cargo-modules`): zero toques.
- ADRs / spec: zero toques (a spec **inclui** este componente porque é
  derivado natural da forma — `Owns` e `Uses` já estavam descritos).
- Pureza do L1: `cargo tree -p lente_estrutura` mostra só `lente_core`.
- Subprocessos do cargo (invariante 0023): dois únicos.

---

## Observação metodológica

### Fractal — horizonte que o desenho **permite**, não que constrói

O prompt foi explícito: **um nível**, provar contra dado real, e os
outros viram aplicação das mesmas peças. Cumprido literalmente:

- `detectar_ciclos: &Grafo → Vec<Ciclo>` é **genérico** sobre `Grafo`. A
  detecção sobre itens (escala microscópica) ou sobre o grafo de crates
  (escala macroscópica) usa **a mesma função** — teste
  `detectar_ciclos_funciona_sobre_grafo_de_itens_tambem` ancora.
- `agregar_por_modulo: &Grafo → Grafo` **produz** um `Grafo`. Pode ser
  re-agregado (ex.: módulo→crate, se houver `Owns` entre módulos e o
  crate-raiz). Composição direta — pré-requisito do zoom.

A régua de zoom tem o **local** numa ponta (item e raio — laudos 0006
em diante) e o **global** na outra (crate e módulo — agora). Falta o
nível **workspace** (crate-a-crate). Falta navegar entre níveis. Mas a
base está pronta: a mesma peça, noutra escala.

### Validação contra dado real **antes** de declarar útil

Padrão do projeto (laudos 0021, 0025, 0027, 0029, 0030). Aqui: ranking
de utilidade da vista de módulo só foi declarado **depois** de medir o
`egui` (111 módulos, legível) e encontrar um achado não-trivial (SCC de
85). Adivinhar antes seria especulação.

### Lattix sem o overhead de Lattix

O Lattix LDM é caro, fechado e tem curva. Aqui, em ~400 linhas de Rust
puro (mais a fiação), o **resultado-cabeçalho** de uma ferramenta Lattix
está disponível na CLI:

```
$ lente --pacote <X> --estrutura --text
```

Não é tudo o que o Lattix faz — falta a matriz visual, falta o
particionamento, falta a navegação. Mas o **achado** principal — o ciclo
arquitetural — está lá. Próximo passo do projeto decide se a matriz vale
construir.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-03 | Primeiro tijolo da vista global: novo crate L1 `lente_estrutura` (`agregar_por_modulo` + `detectar_ciclos` via Tarjan iterativo à mão); fiação `analisar_estrutura(fonte, escopo)` no `lente_wiring` reusando `obter_grafo`; CLI `--estrutura` mutuamente exclusiva com os outros modos, com JSON DSM-friendly e texto humano (ciclos primeiro). Medido contra `egui` real: 111 módulos, 864 dependências, **1 ciclo de 85 módulos** (76% do crate) — **idêntico** nos dois escopos, confirmando invariância dos ciclos ao escopo. `lente_core`: 0 ciclos, 7→6 módulos. Pureza L1 mantida (`cargo tree` só mostra `lente_core`); dois subprocessos do cargo (0023). 176 verdes + 19 ignored. | `09_estrutura/{Cargo.toml,src/lib.rs}`, `04_wiring/{Cargo.toml,src/lib.rs}`, `02_shell/catalogo/src/lib.rs`, `02_shell/cli/src/{args,saida,main}.rs`, `Cargo.toml` raiz, `00_nucleo/lessons/0031-estrutura-modulo-ciclos.md` |
