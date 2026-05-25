# Prompt L0 (revisão): Renderizador HTML — Destaque de Violações

**Camada**: L₃ (Infraestrutura)
**Ficheiro alvo**: `03_infra/src/html_renderer.rs` (revisão de
  arquivo já `IMPLEMENTADO`)
**Passo do roadmap**: 2.3 — Integração com `crystalline.toml`
**Status**: IMPLEMENTADO
**Prompt original**: `html_renderer.md` (status passa para
  `IMPLEMENTADO (revisado)`).

---

## Decisões de design prévias

- **Passo 2.2**: `render_dsm_html` já existe e renderiza a matriz
  sem semântica de camadas.
- **Passo 2.3, L₁**: `detect_layer_violations` produz
  `Vec<LayerViolation>` (violações de direção topológica).
- **Passo 2.3, L₃**: `read_sarif` produz `Vec<SarifFinding>`
  (violações detectadas pelo crystalline-lint).

---

## Decisões locais (assumidas neste prompt)

1. **Dois tipos de violação, duas cores**:
   - **Violação de direção topológica** (do detector L₁):
     célula da matriz pintada de vermelho.
   - **Finding do SARIF** (do linter): nó (linha/coluna inteira)
     com marcador lateral, cor laranja/âmbar.

   Razão da distinção: violação topológica é sobre uma **aresta**
   (par origem-destino), então é uma célula. Finding do SARIF é
   sobre um **ficheiro/nó**, então afeta o nó inteiro
   (linha+coluna).

2. **Ambos os inputs são opcionais**: a função estendida aceita
   `Option<&[LayerViolation]>` e `Option<&[SarifFinding]>`. Se
   ambos `None`, o comportamento é idêntico ao Passo 2.2 (sem
   destaque). Retrocompatível.

3. **Contadores no header**: quando há violações, o header
   mostra "N layer violations · M lint findings".

4. **Cruzamento SARIF → nó**: o `SarifFinding.file_uri` é casado
   com o `source_file` dos nós (via `trees`, se disponível) ou
   com heurística sobre o `canonical_path`. Como o renderizador
   pode não ter acesso ao `source_file` (depende de
   `InternalWithTree`), o cruzamento é best-effort. Findings que
   não casam com nenhum nó são contados mas não destacados na
   matriz (aparecem só no contador).

---

## Contexto

O renderizador do Passo 2.2 desenha a matriz sem distinção de
violações. Esta revisão adiciona destaque visual para:

- Arestas que violam a direção topológica das camadas (vermelho).
- Nós com findings do crystalline-lint (marcador âmbar).

A informação de violação é computada fora do renderizador (em L₁
e nos leitores L₃) e passada como parâmetros. O renderizador só
desenha.

---

## Mudança na assinatura da função

```rust
pub fn render_dsm_html(
    graph: &DependencyGraph,
    partition: &PartitionedOrder,
    cycles: &CycleReport,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
    // NOVOS parâmetros (opcionais):
    layer_violations: Option<&[LayerViolation]>,
    sarif_findings: Option<&[SarifFinding]>,
) -> Result<String, HtmlRenderError>;
```

Quando `layer_violations` e `sarif_findings` são `None`, o output
é idêntico ao do Passo 2.2.

---

## Mudanças nos dados embutidos (`GRAPH_DATA`)

Adicionar dois campos:

```js
const GRAPH_DATA = {
  // ... campos existentes ...

  // Pares (from_pos, to_pos) que violam direção de camada.
  // Renderizados em vermelho na matriz.
  layer_violations: [
    { from: 12, to: 305, from_layer: "L1", to_layer: "L3" },
    ...
  ],

  // Posições de nós com findings do linter.
  // Mapa pos -> lista de findings (rule_id + level).
  sarif_findings: [
    { pos: 42, rule_id: "V9", level: "error" },
    ...
  ],
};
```

A construção desses arrays acontece em L₃ (no renderizador):

- `layer_violations`: traduz cada `LayerViolation` (que tem
  `GraphNodeId`s) para posições no `PartitionedOrder.order`.
- `sarif_findings`: para cada `SarifFinding`, tenta casar
  `file_uri` com um nó. Se casar, registra a posição. Se não,
  ignora (mas conta para o header).

---

## Mudanças visuais

### Na matriz (Canvas)

1. **Células de violação topológica**: ao desenhar uma célula
   que corresponde a um par em `layer_violations`, usar a cor
   `--cell-violation` (vermelho) em vez da cor normal de aresta.

2. **Nós com SARIF finding**: desenhar um marcador (pequeno
   triângulo ou quadrado âmbar) na margem da linha e da coluna
   correspondentes, fora da grade principal mas alinhado.

### No CSS (novas custom properties)

```css
@scope (.dsm-root) {
  :scope {
    /* ... existentes ... */
    --cell-violation: light-dark(#dc2626, #f87171);
    --marker-sarif: light-dark(#f59e0b, #fbbf24);
  }
}
```

### No header

```html
<div class="stats">
  {node_count} nodes · {edge_count} edges · {cycle_count} cycles
  {if has_violations}· <span class="violation-count">{N} layer violations</span>{/if}
  {if has_findings}· <span class="finding-count">{M} lint findings</span>{/if}
</div>
```

CSS:
```css
.violation-count { color: var(--cell-violation); font-weight: 600; }
.finding-count { color: var(--marker-sarif); font-weight: 600; }
```

### Na legenda (footer)

Adicionar swatches explicando as cores novas: vermelho = violação
de direção de camada; âmbar = finding do linter.

### No tooltip

Quando o hover é sobre uma célula de violação topológica, o
tooltip inclui uma linha extra: "⚠ Layer violation: L1 → L3
(forbidden)".

---

## Mudanças no JS embutido

1. **Renderização de células de violação**: ao desenhar, checar
   se o par (row, col) está no set de `layer_violations`. Se sim,
   cor vermelha.

   Construir um `Set` de chaves `"{from}-{to}"` na inicialização
   para lookup O(1).

2. **Marcadores SARIF**: após desenhar a matriz, iterar
   `sarif_findings` e desenhar marcadores nas margens.

3. **Tooltip estendido**: ao montar o conteúdo do tooltip,
   verificar se a célula é violação e adicionar a linha de aviso.

4. **Filtro novo (opcional)**: botão "Show only violations" que
   esconde tudo exceto linhas/colunas envolvidas em violações.
   Implementação CSS via classe no body, análoga aos filtros
   existentes. **Decisão**: incluir se simples; senão, adiar.

---

## Novos testes esperados

Os testes do Passo 2.2 permanecem (a função sem violações deve
continuar funcionando). Adicionar:

1. **Sem violações (retrocompat)**: chamar `render_dsm_html`
   com `None, None`. Output idêntico ao Passo 2.2 (não contém
   `layer_violations` com entradas, não contém contadores de
   violação no header).

2. **Com violação topológica**: passar 1 `LayerViolation`.
   Output contém:
   - O par no array `layer_violations` do JS.
   - "1 layer violation" no header.
   - A custom property `--cell-violation` no CSS.

3. **Com finding SARIF que casa**: passar 1 `SarifFinding`
   cujo `file_uri` casa com um nó. Output contém o finding em
   `sarif_findings` com a posição correta.

4. **Com finding SARIF que NÃO casa**: `file_uri` não
   corresponde a nenhum nó. O finding é contado no header
   ("1 lint finding") mas não aparece no array `sarif_findings`
   do JS (ou aparece com pos `-1`/sentinela; decidir).

5. **Contadores corretos**: 3 violações topológicas + 2 findings
   → header mostra "3 layer violations · 2 lint findings".

6. **Cores no CSS**: output contém `--cell-violation` e
   `--marker-sarif`.

---

## Critério de aceitação do prompt

- `render_dsm_html` com a nova assinatura (2 parâmetros
  opcionais).
- Comportamento retrocompatível quando ambos `None`.
- Células de violação topológica em vermelho.
- Marcadores SARIF nas margens.
- Contadores no header.
- Legenda atualizada.
- Tooltip estendido para violações.
- Os 6 testes novos + os testes existentes do Passo 2.2 passam.
- `cargo clippy --all-targets` sem warnings.
- L₁ permanece inalterado.

---

## Próximos passos (fora deste prompt)

1. **L₄**: orquestração final. Novas flags:
   - `--config <path>` (default `./crystalline.toml`): se
     presente e o ficheiro existe, ler `LayerConfig`, rodar
     `detect_layer_violations`, passar ao renderizador.
   - `--sarif <path>`: se presente, ler o SARIF, passar ao
     renderizador.
   - Sem essas flags ou ficheiros: renderizar sem violações
     (comportamento do Passo 2.2).

2. **Smoke test contra Typst**: o Typst não é projeto cristalino
   (não tem `crystalline.toml` nem camadas L0-L4), então o teste
   de violações usa o próprio `crystalline-dsm` ou
   `typst-crystalline` como alvo (que SÃO cristalinos). Decidir
   o alvo do teste de violações no momento da implementação de L₄.

---

## Limitações conhecidas

1. **Cruzamento SARIF → nó é best-effort**: depende de casar
   `file_uri` com `source_file`. Se o grafo foi construído sem
   `trees` (nós `InternalWithoutTree`), o `source_file` não está
   disponível e o casamento falha. Nesse caso, findings contam no
   header mas não destacam na matriz. Para destaque completo,
   rodar com `--emit-trees` implícito ou manter as trees em
   memória.

2. **Marcadores SARIF nas margens podem sobrepor** em grafos
   densos. Posicionamento básico no MVP.

3. **Um nó pode ter múltiplos findings**: o marcador indica
   presença, não quantidade. Tooltip pode listar todos.

---

## Hash do prompt

A calcular após aprovação.
