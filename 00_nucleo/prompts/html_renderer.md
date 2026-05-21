# Prompt L0: Renderizador DSM HTML (L₃)

**Camada**: L₃ (Infraestrutura)
**Ficheiro alvo**: `03_infra/src/html_renderer.rs`
**Passo do roadmap**: 2.2 — Renderizador HTML estático
**Status**: IMPLEMENTADO
**Nota de implementação**: as variáveis CSS são declaradas via `:scope { ... }` dentro do bloco `@scope (.dsm-root) { ... }` (em vez de directamente no selector `.dsm-root` aninhado). Funcionalmente idêntico — `:scope` resolve para o `.dsm-root` que define o scope — e melhora a leitura. O teste 11 do prompt verifica que NÃO há `:root { --bg-page`, o que se mantém. Validado contra Typst real: HTML de 348 KB, renderização em 62 ms.

---

## Decisões de design prévias

- **ADR-0001**: HTML estático auto-contido como output principal
  do MVP.
- **ADR-0006**: nós externos têm marcação distinta no grafo.
- **Passo 2.1**: `PartitionedOrder` fornece a ordem e os SCCs.

---

## Decisões de design (baseline desta proposta)

Estas decisões formam o baseline visual e técnico. Cada uma pode
ser questionada antes da implementação.

### Forma do output

HTML único, auto-contido. CSS e JS embutidos como tags `<style>` e
`<script>` no próprio documento. Sem dependências externas no
browser. Funciona offline ao abrir o ficheiro num browser
moderno.

### Tecnologia de renderização da matriz

**Canvas 2D HTML5** para o desenho da matriz. Razão:

- Para grafos do tamanho do Typst (667 × 667 = ~445k células), DOM
  puro é impraticável (browsers travam acima de ~10k elementos
  com listeners).
- Canvas 2D é amplamente suportado, sem dependências, e renderiza
  centenas de milhares de retângulos em milissegundos.

O Canvas é envolvido em estrutura HTML para overlays:

- Labels de linha/coluna em divs absolutos posicionados.
- Tooltip flutuante via **Popover API** (nativo, sem JS de posicionamento).
- Botões de controle em HTML normal.

Alternativa rejeitada: SVG. Para grafos grandes, SVG perde
performance ao mesmo nível do DOM. Canvas é a escolha
pragmática.

### Layout visual

```
+---------------------------------+
| Header: título, contagens       |
+---------------------------------+
| Controles: toggle externos,     |
| filtros, busca                  |
+---------------------------------+
| [Labels rotacionados 45°]       |
| [vazio]  [colunas]              |
+--+------------------------------+
|L |                              |
|i |                              |
|n |   Matriz Canvas              |
|h |                              |
|a |                              |
|s |                              |
+--+------------------------------+
| Rodapé: legenda, metadados      |
+---------------------------------+
```

- Labels de linhas à esquerda da matriz, alinhados à direita.
- Labels de colunas no topo, rotacionados -45°.
- Linhas de separação visual entre internos e externos
  (`internal_boundary`).
- Bordas em torno de SCCs cíclicos.

### Convenção da matriz

Mantém Steward/Browning (alinhada com o Passo 2.1):

- Linha i, coluna j: marca se nó em posição i depende do nó em
  posição j.
- Marca abaixo da diagonal: dependência "ordenada" (j vem antes
  de i; matriz triangular inferior é o objetivo).
- Marca acima da diagonal: dependência "fora de ordem" (sintoma
  de ciclo). Esperada apenas dentro de SCCs cíclicos.
- Diagonal (i == j): self-loop se houver; vazia caso contrário.

### Suporte de browser e tecnologias modernas

O renderizador adopta APIs CSS e JS modernas para reduzir o
volume de código embutido. Estado actual (Maio 2026) das três
tecnologias-chave:

**1. `light-dark()` CSS function:**
- Suportada em Chrome, Firefox e Safari estáveis desde 2024.
- Genuinamente **Baseline** (estável em todos os engines há > 30
  meses).
- Sem necessidade de fallback.

**2. Popover API (`popover` attribute):**
- Chrome 114+ (Maio 2023), Safari 17+ (Set 2023), Firefox 125+
  (Abril 2024).
- **Baseline Newly available** desde mid-2024.
- Maduro e cross-browser.

**3. CSS Anchor Positioning:**
- Chrome 125+ (Maio 2024).
- Safari 26.0 (Setembro 2025).
- Firefox 147 (Janeiro 2026, enabled by default).
- **NÃO é Baseline ainda** (Firefox completou suporte há ~4
  meses; "Baseline Newly available" requer suporte estável em
  todos os engines por tempo maior).
- Aceito como "cross-browser disponível desde Jan 2026, com
  fallback gracioso".

**4. CSS `@scope`:**
- Chrome 118+ (Out 2023), Safari 17.4+ (Mar 2024), Firefox 146
  (Jan 2026).
- **Baseline Newly available** muito recente.

Para browsers que não suportarem alguma destas, o
comportamento de fallback é documentado nas Limitações
Conhecidas.

### Cores

Esquema mínimo, sem semáforo (semântica fica para Passo 2.3).
Cores definidas via **CSS Custom Properties** e a função
**`light-dark()`** (Baseline), eliminando duplicação de regras
`@media (prefers-color-scheme: dark)`.

```css
.dsm-root {
  color-scheme: light dark;
  --font-base: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --cell-size: 6px; /* sobrescrito inline pelo JS */

  /* Surfaces */
  --bg-page: light-dark(#fafafa, #121212);
  --bg-matrix: light-dark(#ffffff, #1e293b);
  --bg-extern: light-dark(rgba(0,0,0,0.02), rgba(255,255,255,0.03));

  /* Células de aresta — escala discreta por intensidade */
  --cell-edge-1: light-dark(#bfdbfe, #1e3a8a);
  --cell-edge-2: light-dark(#60a5fa, #1d4ed8);
  --cell-edge-4: light-dark(#2563eb, #3b82f6);

  /* Estrutura */
  --diagonal: light-dark(#9ca3af, #4b5563);
  --border-scc: light-dark(#ef4444, #f87171);
  --divider: light-dark(#374151, #9ca3af);

  /* Texto */
  --text-primary: light-dark(#111827, #e0e0e0);
  --text-secondary: light-dark(#6b7280, #9ca3af);
  --label-highlight: light-dark(#dbeafe, #1e40af);
}
```

**Nota importante**: as custom properties são declaradas no
selector `.dsm-root` (o `<body>`), NÃO dentro de `:root`
aninhado em `@scope`. Razão: `:root` sempre referencia o
elemento raiz do documento (`<html>`), independente de
contexto. Aninhar dentro de `@scope` não escopa o `:root`. A
forma idiomática de escopar variáveis a um container é
declará-las directamente no selector do container.

`@scope (.dsm-root) { ... }` é útil para isolar **regras de
seletor** (evitar conflito com regras externas em
páginas que embutem o HTML), mas não escopa variáveis CSS de
`:root`.

- **Fundo da matriz**: `var(--bg-matrix)`.
- **Célula com aresta**: preenchimento sólido escalonado pelas
  custom properties acima. Escala discreta: 1 aresta = tom claro,
  2-3 = médio, 4+ = forte.
- **Diagonal**: `var(--diagonal)` (visual de "espinha dorsal").
- **Bordas de SCC cíclico**: contorno `var(--border-scc)` ao redor
  do bloco.
- **Linha divisória interno/externo**: separação visual clara,
  `var(--divider)`.
- **Região externa**: fundo levemente diferenciado (`--bg-extern`).

#### Cores no Canvas: cacheamento explícito

Canvas 2D não consome custom properties directamente; precisa de
strings de cor literais em `ctx.fillStyle`. O JS lê as variáveis
via `getComputedStyle()` **uma vez na inicialização** e cacheia
os valores num objecto:

```js
const root = document.querySelector('.dsm-root');
const styles = getComputedStyle(root);
const COLORS = {
  cellEdge1: styles.getPropertyValue('--cell-edge-1').trim(),
  cellEdge2: styles.getPropertyValue('--cell-edge-2').trim(),
  cellEdge4: styles.getPropertyValue('--cell-edge-4').trim(),
  diagonal: styles.getPropertyValue('--diagonal').trim(),
  borderScc: styles.getPropertyValue('--border-scc').trim(),
  // ...
};
```

Para responder a mudanças de modo claro/escuro (sistema OS),
o JS observa `matchMedia('(prefers-color-scheme: dark)')` e
recalcula o cache + redesenha o canvas. Sem isso, a matriz
ficaria desactualizada após o utilizador alternar o tema.

### Interactividade — com mínimo de JavaScript

Versão mínima viável. Onde possível, funcionalidade é delegada
a APIs nativas do browser ou a CSS, reduzindo o código JS
embutido.

1. **Hover em célula**: tooltip via **Popover API** (`popover`
   attribute). O JS apenas:
   - Identifica a célula sob o cursor (hit-testing no canvas).
   - Preenche o conteúdo do tooltip (DOM textContent).
   - Chama `tooltip.showPopover()` / `hidePopover()`.

   **Sobre posicionamento**: o tooltip usa Anchor Positioning
   quando disponível (Chrome 125+, Safari 26.0+, Firefox 147+).
   Em browsers sem suporte completo, o JS calcula um
   posicionamento básico (next to cursor) como fallback. Este
   fallback NÃO é polyfill complexo; apenas `style.left` e
   `style.top` baseados em `event.clientX/Y`.

   Conteúdo do tooltip:
   - Linha (path do nó dependente).
   - Coluna (path do nó dependido).
   - Quantidade de arestas (imports).
   - Lista dos items importados (até 5; "+ N mais" se houver
     mais).

2. **Hover em label de linha**: realce visual da linha inteira.
   O JS adiciona/remove uma classe `.is-hovered` no `<span>`
   específico:

   ```js
   labels.row[hoveredIdx].classList.add('is-hovered');
   labels.row[previousHoveredIdx]?.classList.remove('is-hovered');
   ```

   O CSS:
   ```css
   .row-labels span.is-hovered {
     color: var(--text-primary);
     background: var(--label-highlight);
   }
   ```

   **Importante**: seletores de atributo CSS NÃO interpolam
   custom properties. Padrões como `[data-idx="var(--hover-col)"]`
   NÃO funcionam — o CSS engine compara com a string literal,
   não com o valor da variável. O caminho correcto é JS aplicar
   classe directa.

3. **Hover em label de coluna**: mesmo mecanismo (classe
   `.is-hovered` no span da coluna).

4. **Click em célula**: alterna estado "pinned". JS adiciona/
   remove classe `.is-pinned` nos labels da linha e coluna
   correspondentes; opcionalmente desenha overlay no canvas
   para destacar a célula pinada.

5. **Toggle "Ocultar externos"**: botão. JS adiciona/remove
   classe `hide-externals` no `<body>`:

   ```css
   body.hide-externals .row-labels span[data-kind="external"],
   body.hide-externals .column-labels span[data-kind="external"] {
     display: none;
   }
   ```

   O canvas é redesenhado uma vez após o toggle (a região
   externa fica fora do desenho).

6. **Toggle "Apenas SCCs cíclicos"**: filtro. Mesmo padrão:
   classe `only-cyclic` no body; CSS esconde labels triviais
   via `[data-cyclic="false"]`.

7. **Busca por texto**: o JS atualiza atributo `data-match`
   nos labels:

   ```js
   for (const span of labels.row) {
     const matches = span.textContent.toLowerCase().includes(query);
     span.dataset.match = matches ? "true" : "false";
   }
   ```

   CSS:
   ```css
   .row-labels span[data-match="false"],
   .column-labels span[data-match="false"] {
     opacity: 0.15;
   }
   ```

   O canvas usa esta informação no próximo redraw para reduzir
   opacidade de células correspondentes.

Funcionalidades **NÃO** incluídas no MVP:

- Zoom semântico (ampliar uma região da matriz).
- Drag para reordenar manualmente.
- Exportar selecção como JSON / CSV.
- Animações entre filtros (apenas `transition` CSS em labels).

### Tamanho de célula

Adaptativo conforme tamanho do grafo:

- Matriz pequena (< 50 nós): 24px × 24px por célula.
- Matriz média (50-200 nós): 12px × 12px.
- Matriz grande (200-700 nós): 6px × 6px.
- Matriz muito grande (> 700 nós): 3px × 3px (no Typst).

Calculado em runtime no JS, baseado em `order.len()` e tamanho
da viewport. Expõe via CSS Custom Property `--cell-size` no
container (`style.setProperty('--cell-size', '6px')`), para que
labels e overlays possam sincronizar dimensões sem duplicar
lógica.

### Embutimento de CSS e JS

Tudo inline no HTML gerado. Tamanho estimado para Typst:

- Estrutura HTML: alguns KB.
- CSS: ~6 KB (`light-dark()` elimina ~50% das regras de dark
  mode duplicadas).
- JS: ~12-18 KB (parte da lógica de tooltip delegada a Popover
  API, mas mantém fallback básico de posicionamento).
- Dados do grafo (matriz, labels, metadados): proporcional ao
  número de arestas. Para Typst, ~200-500 KB depois de
  compressão lógica (arestas como pares de índices, não objetos
  completos).

Total esperado: < 1 MB para o Typst. Aceitável para distribuição.

### Camada

L₃ (Infraestrutura). A renderização para HTML é transformação de
formato, análoga aos serializadores JSON do Passo 1.4. L₃ não
faz I/O directo (não grava ficheiro); produz `String` com o HTML
completo. L₄ grava.

---

## Função pública principal

```rust
pub fn render_dsm_html(
    graph: &DependencyGraph,
    partition: &PartitionedOrder,
    cycles: &CycleReport,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
) -> Result<String, HtmlRenderError>;
```

Retorna o HTML completo como string. Não toca o filesystem.

---

## Estrutura interna do HTML gerado

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light dark">
  <title>Crystalline DSM — {workspace_name}</title>
  <style>{embedded_css}</style>
</head>
<body class="dsm-root">
  <header>
    <h1>{workspace_name}</h1>
    <div class="metadata">
      Generated at {generated_at} by crystalline-dsm v{tool_version}
    </div>
    <div class="stats">
      {node_count} nodes · {edge_count} edges · {cycle_count} cycles
    </div>
  </header>

  <section class="controls">
    <fieldset>
      <legend>Filters</legend>
      <button id="toggle-external" type="button">Hide external nodes</button>
      <button id="toggle-trivial" type="button">Show only cyclic SCCs</button>
      <input type="search" id="search" placeholder="Filter nodes..." autocomplete="off">
    </fieldset>
  </section>

  <main class="matrix-container" style="--cell-size: 6px;">
    <div class="column-labels" role="region" aria-label="Column labels">
      <!-- divs absolutos com labels rotacionados -->
    </div>
    <div class="row-labels" role="region" aria-label="Row labels">
      <!-- divs absolutos com labels alinhados à direita -->
    </div>
    <canvas id="dsm-matrix" role="img" aria-label="Dependency structure matrix">
    </canvas>
    <div id="tooltip" popover="manual"></div>
  </main>

  <footer>
    <div class="legend">
      <!-- swatches explicando cores -->
    </div>
  </footer>

  <script type="module">
    const GRAPH_DATA = { /* dados embutidos */ };
    {embedded_js}
  </script>
</body>
</html>
```

Notáveis:
- `<body class="dsm-root">` define o container raiz onde as
  custom properties vivem.
- `<meta name="color-scheme">` habilita dark mode nativo do OS.
- `<fieldset>`/`<legend>` para semântica dos controles.
- `role="img"` e `aria-label` no canvas para acessibilidade
  básica.
- `popover="manual"` no tooltip (controle manual via JS).
- `type="module"` no script para escopo isolado e deferred
  execution.

---

## Dados embutidos no JS (`GRAPH_DATA`)

Estrutura compactada para reduzir tamanho do HTML:

```js
const GRAPH_DATA = {
  schema_version: "1.0.0",
  workspace_name: "...",
  internal_boundary: 443,
  // Labels indexados pela posição no order.
  labels: ["...", "...", ...],

  // kinds[i] é "internal" | "external_crate" | "external_stdlib"
  kinds: ["internal", "internal", ..., "external_crate", ...],

  // crate_names[i] para nós internos; null para externos.
  crate_names: ["...", "...", null, ...],

  // Arestas como pares de índices [from_pos, to_pos] mais
  // contagem agregada e items importados.
  edges: [
    { from: 12, to: 45, count: 3, items: ["Foo", "Bar", "Baz"], has_glob: false },
    ...
  ],

  // SCCs como ranges no order, mais flag de ciclicidade.
  sccs: [
    { start: 0, end: 1, cyclic: false },
    { start: 1, end: 5, cyclic: true },
    ...
  ],

  // scc_per_position[i] indexa em sccs.
  scc_per_position: [0, 1, 1, 1, 1, 2, ...],
};
```

A serialização para esta estrutura é responsabilidade do
renderizador (em L₃, traduz do `DependencyGraph` +
`PartitionedOrder` + `CycleReport` para esta forma JS).

Notas:

- Arestas entre o mesmo par são **agregadas** no JS (diferente
  do `graph.json`, onde cada `use` individual é uma aresta).
- `items` truncado a 5 elementos. Se houver mais, adicionar
  flag `truncated: true` à aresta.

---

## Funções auxiliares (em L₃)

```rust
fn build_html_data(
    graph: &DependencyGraph,
    partition: &PartitionedOrder,
) -> HtmlGraphData;

fn aggregate_edges(graph: &DependencyGraph, partition: &PartitionedOrder)
    -> Vec<AggregatedEdge>;

fn render_css() -> &'static str;

fn render_js() -> &'static str;

fn serialize_to_js_literal(data: &HtmlGraphData) -> String;
```

O CSS e o JS embutidos vivem como ficheiros separados embutidos
via `include_str!`. Estrutura:

```
03_infra/src/html_renderer.rs        # lógica Rust
03_infra/src/html_renderer/style.css  # embutido via include_str
03_infra/src/html_renderer/script.js  # embutido via include_str
```

---

## Tipo de erro

```rust
#[derive(Debug, thiserror::Error)]
pub enum HtmlRenderError {
    #[error("Falha ao serializar dados para JS: {message}")]
    SerializationFailed { message: String },

    #[error("Configuração inválida: {detail}")]
    InvalidConfiguration { detail: String },
}
```

---

## Dependências externas

`03_infra/Cargo.toml`: nenhuma nova. Usa apenas `std`.

Não usar templating engines (handlebars, askama, tera).

---

## Implementação do JS embutido — reduzida

O JS embutido implementa apenas o estritamente necessário;
todo o resto é delegado a CSS/HTML nativo:

1. **Inicialização**:
   - Lê `GRAPH_DATA`.
   - Calcula `--cell-size` baseado em `labels.length` e viewport,
     seta via `container.style.setProperty('--cell-size', ...)`.
   - Lê cores das CSS variables via `getComputedStyle()` **uma
     vez** e cacheia em objecto `COLORS`.
   - Subscreve `matchMedia('(prefers-color-scheme: dark)')` para
     recalcular `COLORS` e redesenhar em mudança de tema.
   - Desenha matriz no canvas.

2. **Construção de índice de adjacência**:
   - `outgoing: Map<from_pos, Edge[]>`.
   - `incoming: Map<to_pos, Edge[]>`.
   - `pair_to_edge: Map<"{from}-{to}", Edge>` para lookup em
     hover.

3. **Event listeners**:
   - `mousemove` no canvas:
     a. Identifica célula sob cursor (cálculo a partir de
        `event.offsetX/Y` e `--cell-size`).
     b. Lookup no `pair_to_edge`.
     c. Preenche `textContent` do tooltip.
     d. Chama `tooltip.showPopover()`.
     e. Posicionamento: se Anchor Positioning disponível, deixa
        o browser cuidar; caso contrário, seta `left`/`top` via
        `event.clientX/Y + offset`.
     f. Adiciona classe `.is-hovered` aos labels correspondentes.
   - `mouseleave` no canvas: `tooltip.hidePopover()`, remove
     classes.
   - `click` no canvas: alterna classe `.is-pinned`.
   - Botões: toggle de classes no `<body>`.
   - Input de busca: itera labels e seta `data-match`.

4. **Tooltip**: lógica mínima. `showPopover()` /
   `hidePopover()` + `textContent`. O browser cuida do z-index
   e (quando suportado) posicionamento.

5. **Filtros**: ao detectar mudança de classe no body, redesenha
   o canvas uma vez. Labels são escondidos pelo CSS imediatamente.

6. **Busca**: o JS marca `data-match` nos labels. Canvas usa esta
   informação no próximo redraw.

O JS é **vanilla ES module**, sem frameworks.

---

## Estilos CSS principais

```css
@scope (.dsm-root) {
  body {
    color-scheme: light dark;
    --font-base: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    --cell-size: 6px;

    --bg-page: light-dark(#fafafa, #121212);
    --bg-matrix: light-dark(#ffffff, #1e293b);
    --bg-extern: light-dark(rgba(0,0,0,0.02), rgba(255,255,255,0.03));
    --cell-edge-1: light-dark(#bfdbfe, #1e3a8a);
    --cell-edge-2: light-dark(#60a5fa, #1d4ed8);
    --cell-edge-4: light-dark(#2563eb, #3b82f6);
    --diagonal: light-dark(#9ca3af, #4b5563);
    --border-scc: light-dark(#ef4444, #f87171);
    --divider: light-dark(#374151, #9ca3af);
    --text-primary: light-dark(#111827, #e0e0e0);
    --text-secondary: light-dark(#6b7280, #9ca3af);
    --label-highlight: light-dark(#dbeafe, #1e40af);

    font-family: var(--font-base);
    background: var(--bg-page);
    color: var(--text-primary);
    margin: 0;
    padding: 16px;
  }

  @media (prefers-reduced-motion: reduce) {
    * {
      transition: none !important;
      animation: none !important;
    }
  }

  .matrix-container {
    position: relative;
    display: grid;
  }

  #dsm-matrix {
    cursor: crosshair;
    border: 1px solid var(--divider);
  }

  /* Tooltip via Popover API */
  #tooltip {
    background: rgba(0, 0, 0, 0.85);
    color: white;
    padding: 8px 12px;
    border-radius: 4px;
    font-size: 12px;
    max-width: 300px;
    border: none;
  }

  .column-labels {
    position: relative;
  }

  .column-labels span {
    position: absolute;
    transform: rotate(-45deg);
    transform-origin: top left;
    white-space: nowrap;
    font-size: 10px;
    color: var(--text-secondary);
    transition: opacity 0.15s ease, color 0.15s ease;
  }

  /* Hover via classe directa (JS aplica `.is-hovered`) */
  .column-labels span.is-hovered,
  .row-labels span.is-hovered {
    color: var(--text-primary);
    background: var(--label-highlight);
  }

  /* Pin via classe directa */
  .column-labels span.is-pinned,
  .row-labels span.is-pinned {
    color: var(--text-primary);
    background: var(--label-highlight);
    font-weight: 600;
  }

  .row-labels {
    display: flex;
    flex-direction: column;
  }

  .row-labels span {
    text-align: right;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 10px;
    padding-right: 6px;
    color: var(--text-secondary);
    transition: opacity 0.15s ease, color 0.15s ease;
  }

  /* Busca: filtragem visual puramente CSS */
  .row-labels span[data-match="false"],
  .column-labels span[data-match="false"] {
    opacity: 0.15;
  }

  /* Filtro de externos */
  body.hide-externals .row-labels span[data-kind="external"],
  body.hide-externals .column-labels span[data-kind="external"] {
    display: none;
  }

  /* Filtro de SCCs triviais */
  body.only-cyclic .row-labels span[data-cyclic="false"],
  body.only-cyclic .column-labels span[data-cyclic="false"] {
    display: none;
  }
}
```

---

## Testes esperados

### Testes unitários (em L₃)

1. **`render_dsm_html` produz string não vazia**: input mínimo
   (grafo com 1 nó), output contém substrings esperadas
   (`<html>`, `<canvas>`, nome do nó nos labels, `popover` no
   tooltip).

2. **HTML contém o `workspace_name`**: dado workspace específico,
   o título aparece no `<title>` e no `<h1>`.

3. **Quantidade de labels**: número de `<span>` em
   `row-labels` deve ser igual a `order.len()`.

4. **Dados embutidos**: o JS embutido contém
   `internal_boundary` correcto.

5. **Quantidade de arestas serializadas**: número de entradas em
   `GRAPH_DATA.edges` igual ao número de pares (from, to)
   distintos.

6. **Grafo vazio**: input com 0 nós produz HTML válido.

7. **Apenas externos**: input com 1 nó externo.
   `internal_boundary == 0`.

8. **CSS contém custom properties e `light-dark()`**: output
   contém `--cell-size`, `--bg-page`, e a função `light-dark()`.

9. **Tooltip usa popover**: output contém `popover="manual"`.

10. **`@scope` presente no CSS**: output contém `@scope (.dsm-root)`.

11. **Variáveis CSS declaradas em `.dsm-root` (não em `:root`)**:
    verifica padrão correcto. Output NÃO contém
    `:root { --bg-page` (que seria incorrecto).

### Testes de integração

12. **Pipeline completo contra fixture `imports-simple`**:
    workspace → traverse → imports → graph → partition → render.
    HTML resultante existe, é não vazio, contém o nome do crate
    no `<h1>`, contém `type="module"` no script.

13. **Smoke test contra Typst** (`#[ignore]`): renderiza HTML do
    grafo do Typst real. Verifica:
    - HTML gerado tem entre 500 KB e 2 MB.
    - HTML abre num browser sem erros de console (teste manual).
    - Tempo total de renderização < 5 segundos.

### Testes visuais (manuais, fora da suite automatizada)

14. Abrir o HTML gerado num browser moderno:
    - Matriz visível.
    - Hover em célula mostra tooltip (posicionado via Anchor
      Positioning em Chrome 125+/Safari 26+/Firefox 147+, ou via
      fallback de cursor em browsers antigos).
    - Toggle de externos funciona (labels escondidos via CSS).
    - Busca filtra labels visualmente (opacidade CSS).
    - Dark mode respeita `prefers-color-scheme`.
    - Hover em labels destaca via `.is-hovered`.

---

## Critério de aceitação do prompt

- `03_infra/src/html_renderer.rs` existe e compila.
- Subdiretório `03_infra/src/html_renderer/` com `style.css` e
  `script.js`.
- Função `render_dsm_html` com a assinatura especificada.
- Os 13 testes automatizados passam (14 é manual).
- `cargo clippy --all-targets` sem warnings.
- L₁ permanece inalterado.
- Módulo exportado em `03_infra/src/lib.rs`.
- HTML gerado contra Typst real abre e é navegável (validação
  manual).

---

## Próximos passos (fora deste prompt)

1. **Em L₄**: adicionar flag `--emit-html <path>`. Análogo a
   `--emit-trees`.

2. **Captura de tela de exemplo** em `docs/examples/typst.png`
   (manual, após validar visualmente).

3. **Passo 2.3 (integração com `crystalline.toml`)**: estender
   o renderizador para destacar violações de camada.

---

## Limitações conhecidas

1. **Sem suporte a touch** (mobile/tablet). O JS é mouse-only.

2. **Acessibilidade limitada**: a matriz em canvas é puramente
   visual. Labels são DOM e o canvas tem `role="img"`, mas não
   há anúncios detalhados por célula. Limitação aceita para o
   MVP.

3. **Sem internacionalização**: tooltips, labels e mensagens em
   inglês.

4. **Anchor Positioning**: NÃO é Baseline (Firefox completou
   suporte em Jan 2026; falta tempo para atingir status
   "Baseline Newly available"). Browsers sem suporte completo
   (versões antigas, Edge Legacy, etc) usam fallback básico de
   posicionamento via `event.clientX/Y`. Não há polyfill.

5. **`light-dark()` e `@scope`**: Baseline desde 2024-2025.
   Browsers muito antigos (< 2% do tráfego global) não suportam
   e recebem fallback gracioso (sem dark mode automático no caso
   de `light-dark()`, sem escopamento de regras no caso de
   `@scope`).

6. **Performance em grafos > 2000 nós**: pode degradar.
   Optimizações (offscreen canvas, throttling de mouse events)
   ficam para versão futura.

---

## Hash do prompt

A calcular após aprovação.
