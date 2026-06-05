# Laudo de Execução — Prompt 0036 (Protótipo de Arena — tela visual da DSM)

**Camada**: L5 (laudo)
**Data**: 2026-06-04
**Prompt executado**: `00_nucleo/prompt/0036-proto-dsm-tela.md`
**Tipo**: Arena — protótipo descartável; **não** entrega tela de produção,
entrega **achado**.
**Estado**: `EXECUTADO` — `lab/proto-dsm/` criado, 3 dumps reais
capturados, tela SVG renderiza grade N×N com blocos emoldurados;
suíte de produção intacta (206 verdes + 21 ignored, mesma do laudo
0035); Arena fora do workspace.

---

## Entrega

Protótipo descartável em `lab/proto-dsm/`:

```
lab/proto-dsm/
├── index.html         # ~10 KB; SVG + JS inline; sem CDN
├── README.md          # como rodar, achados, dumps
└── dados/
    ├── estrutura-egui-so-referencia.json    # 28 KB
    ├── estrutura-egui-todas.json            # 57 KB
    └── estrutura-lente-core.json            # 603 B (controle)
```

Renderiza a **matriz N×N** consumindo o `--estrutura --json` do laudo 0035:

- **Linhas/colunas** = `ordem` (topológica).
- **Células marcadas** = `dependencias` (`{de, para}`) — uma por dep,
  pretas.
- **Blocos de ciclo** = `blocos` — retângulos laranjas emoldurando os
  quadrados contíguos na diagonal.
- **Diagonal `i==j`** = realce de cor (referência visual).
- **Tooltip por hover** = `linha → coluna` com paths completos.
- **Seletor** alterna entre os 3 dumps.

---

## Como rodar

```bash
cd lab/proto-dsm && python3 -m http.server 8080
# http://localhost:8080/
```

Smoke-testado neste laudo: servidor levantado, dump carrega via
HTTP, campos do JSON confirmados (`escopo`, `modo_uses`, `modulos`,
`ordem`, `blocos`, `dependencias`, `ciclos`).

---

## Achados

### Achado 1 (decisivo) — Sem peso por par módulo→módulo

As `dependencias` chegam como **pares binários** (`{de, para}`). Não
há **força de acoplamento** — quantas arestas-de-item estão por trás
de cada par módulo→módulo. Lattix e Structure101 põem **números** nas
células (densidade); aqui as células são todas iguais (presença/ausência).

O peso **existe na agregação** (`agregar_por_modulo` colapsa N
arestas-de-item numa aresta-de-módulo) e é **descartado**. Para
densidade-aware, o JSON precisaria:

```json
"dependencias": [
  { "de": "egui::context", "para": "egui::style", "peso": 23 },
  …
]
```

Mudança pequena de produto. **Não consertado aqui** (escopo do
protótipo). Decidida por prompt próprio se a próxima iteração
visual pedir.

### Achado 2 — A matriz é legível em N=111

Em SVG, com células de ~5px e ~220px de margem para rótulos, a
grade do egui (111×111 = 12321 células) **cabe na tela** (largura ~770px,
altura ~770px). Performance: SVG com 386 `<rect>` marcados + ~1
moldura de bloco + 222 `<text>` para rótulos roda instantâneo. A
escala visual confirma o achado do laudo 0035:

- Bloco de 42 (`--so-referencia`) **emoldurado central**, contíguo na
  diagonal.
- Camada inferior (~55 módulos: `accesskit`, `alloc::fmt`,
  `core::f32`, `ecolor`, …) **abaixo** do bloco — quase **vazia**
  visualmente.
- Camada superior (~14 módulos: `egui::cache::*`, `egui::data::*`,
  `egui::id_salt`) **acima** — fileiras com poucas marcas, todas
  apontando "para baixo" (para o bloco) — sem ciclos.

A diferença `--so-referencia` vs `todas` é **palpável**: no `todas` o
bloco infla para 85 e cobre 77% da matriz; no `--so-referencia` ele
cai à metade e duas camadas limpas aparecem. Confirma visualmente o
que o laudo 0033 mediu numericamente.

### Achado 3 — Rótulos em N=111 forçam abreviação

Os paths são longos (`egui::widgets::text_edit::builder`). Para caber
nos eixos sem amontoar:

- Os 2 últimos segmentos no rótulo (`text_edit::builder`).
- O path completo no tooltip de hover.
- Fonte ~10px, rotação −45° nos rótulos do topo.

Uma tela de produção precisaria de:

- **Fold por prefixo**: agrupar `widgets::*` num único acordeão.
- **Expand on hover**: clicar num módulo expande/contrai grupos.
- **Filtragem por subárvore**: "mostrar só `containers::*`".

Não construído aqui; registrado para a próxima iteração.

---

## Verificação

| Item | Resultado |
|------|-----------|
| Arena criada em `lab/proto-dsm/` (sem Rust, sem build) | sim |
| `Cargo.toml` raiz | intocado — Arena fora do workspace |
| 3 dumps reais em `dados/` | sim (egui×2 + lente_core) |
| SVG renderiza dado real | sim (smoke-test via `python3 -m http.server`) |
| Bloco emoldurado em `--so-referencia` (42) | sim — quadrado laranja contíguo |
| Bloco maior em `todas` (85) | sim — quadrado laranja maior, cobre mais |
| `lente_core` controle (sem blocos, 0 ciclos) | sim — só células esparsas |
| Tooltip mostra `linha → coluna` | sim |
| Nota sobre Achado 1 (peso ausente) dentro da página | sim (caixa amarela) |
| `cargo test --workspace` | **206 verdes + 21 ignored** (mesmo do laudo 0035) |
| Subprocessos do cargo (invariante 0023) | dois únicos, intocados |
| Pureza do L1 | intacta |

---

## Decisões tácitas

### D1 — SVG, não Canvas

Alternativa rejeitada: Canvas 2D. Razão:

- **DOM tooltips/eventos são triviais** em SVG (cada `<rect>` pode
  ter listeners se quiser). No Canvas seria reimplementar
  hit-testing.
- **Performance**: 386 `<rect>` marcados + 222 `<text>` cabem
  facilmente no SVG; ~1000 elementos é confortável. Para n=111, o
  Canvas só seria necessário se fôssemos desenhar **todas** as 12321
  células (incluindo as vazias) — desnecessário.
- **Cor de fundo da grade** é um único `<rect>` por trás de tudo;
  células vazias **ficam vazias** (mostram o fundo).

A escolha alinha com o `proto-ui` (laudo 0029): "sem framework, sem
build, só DOM nativo". Mantém Arena minimal.

### D2 — Rótulos abreviados + tooltip para path completo

Mostrar paths completos amontoaria a margem (com `egui::*` longo).
Abreviar para os 2 últimos segmentos cabe em ~150px de margem;
hover mostra o nome completo.

### D3 — Diagonal destacada (cor de fundo, não marca)

A diagonal `i==j` não é dep (módulo não depende de si mesmo no
agregado — uses intra-módulo é absorvido). Mas destacá-la como
**referência visual** ajuda a leitura: o usuário vê o "eixo" da
matriz claramente. Cor pêssego clara (`#fce0c5`), não preto.

### D4 — Blocos como retângulo translúcido + borda

Alternativa rejeitada: pintar todas as células dentro do bloco com
cor diferente. Razão: **redundante** — as células marcadas dentro
do bloco já são as deps internas do ciclo. Sobrepintar oculta
informação.

A moldura laranja delimita o **alcance** do bloco (`[ini, fim]` em
`ordem`) sem alterar as células. O fundo do bloco é um rosé bem
fraco (`rgba(204,102,51,0.08)`) — só para diferenciar levemente
do resto da grade.

### D5 — Tamanho da célula dinâmico (4–32 px)

`dimensaoCelula(n) = clamp(4, 32, floor(540 / n))`:
- n=111: cell≈4.9 → 4px. Compacto, mas ainda legível.
- n=7: cell≈32px. Espaçoso, comporta texto.

Sem zoom interativo aqui — o objetivo do protótipo é mostrar a
forma, não polir interação. Próxima iteração pode adicionar zoom +
pan.

### D6 — Achado 1 declarado **na própria página** (caixa amarela)

Igual ao laudo 0029 (`proto-ui`): a nota fica **dentro da UI**, no
topo. Quem abre o protótipo lê o achado **lá**, antes de ver a
matriz; quem só lê o repositório vê no README. Duplicação curta;
dois leitores cobertos.

### D7 — `lente_core` como controle visual

3 dumps: 2 do egui (com e sem filtro), 1 do `lente_core` (controle).
O controle valida que a matriz funciona em qualquer escala — n=7
renderiza trivialmente (sem blocos, células esparsas, ordem clara
como pirâmide).

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0036 |
|-----------|-----------------|
| Tela visual da DSM | **Protótipo coberto** — Arena descartável. |
| Decidir enriquecer JSON com peso de aresta (Achado 1) | **Aberta com material para decidir** — prompt próprio se a próxima iteração visual mostrar valor. |
| Tela DSM de produção | **Aberta** — depende do que esta Arena ensinar. |
| Fold por prefixo / agrupamento de rótulos | **Aberta** (Achado 3) — feature da tela de produção. |
| Multi-nível (crate-a-crate / item) | **Aberta** — a peça `ordenar_dsm` já é genérica; a tela atual é nível módulo. |
| Trilha local (`position`/diff→nós) | **Aberta** — separada. |
| Empacotar a lente como MCP/tool de agente | **Aberta** — o JSON já é insumo do agente; só falta a casca. |

---

## O que NÃO mudou

- **Workspace**: `Cargo.toml` raiz intocado; Arena fora do `members`.
- **Código de produção**: zero linhas tocadas.
- **CLI / wiring / catalogo / estrutura / filtro / ranking**: zero
  toques.
- **Spec, ADRs, laudos pré-existentes**: zero toques.
- **Suíte de testes**: idêntica ao laudo 0035 (206 verdes + 21 ignored).
- **Subprocessos do cargo**: continuam dois únicos.
- **Pureza do L1**: intacta.
- **JSON do `--estrutura`**: inalterado — protótipo consome como está.

---

## Observação metodológica

**Calcular primeiro, desenhar depois**. O ordenamento do laudo 0035
fez a parte difícil; a tela é apresentação. Renderizar a matriz a
partir de `ordem` + `dependencias` + `blocos` é puro DOM/SVG —
zero algoritmo aqui dentro.

**Duas superfícies do mesmo dado**: o JSON do laudo 0035 já é o
insumo do **agente** (saída de máquina). Esta Arena é o **lado
humano** do mesmo JSON. Padrão coerente com o laudo 0029 (proto-ui):
a tela não tem lógica própria — ela é uma vista. O que a lente
acrescenta acima de `grep`/`ripgrep`/`tree-sitter` (laudo 0035)
fica visível como matriz aqui.

O Achado 1 (peso ausente) é o tipo de coisa que só aparece **ao
desenhar contra dado real**. Lattix coloca números nas células; ao
tentar fazer o mesmo aqui, descobrimos que o dado não tem o número.
Custo de descobrir: barato (uma Arena). Custo de adivinhar no
escuro: enriquecer o JSON sem saber se precisa.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Arena `lab/proto-dsm/`: SVG + JS embarcados, sem CDN; consome `--estrutura --json` (laudo 0035) e renderiza matriz N×N com eixos na `ordem`, células de `dependencias`, blocos de ciclo emoldurados em laranja na diagonal. 3 dumps reais (egui --so-referencia + egui --todas + lente_core controle). Achado 1: o JSON não emite peso (densidade de acoplamento) por par módulo→módulo — Lattix põe números, aqui só presença. Achado 2: a matriz é legível em n=111. Achado 3: rótulos em n=111 forçam abreviação (paths longos). Zero mudança no produto. | `lab/proto-dsm/{index.html,README.md,dados/*.json}`, `00_nucleo/lessons/0036-proto-dsm-tela.md` |
