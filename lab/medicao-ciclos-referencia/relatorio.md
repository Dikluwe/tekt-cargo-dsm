# Medição: ciclos do egui contando só `reference`

**Data**: 2026-06-03
**Prompt**: `00_nucleo/prompt/0033-medicao-ciclos-referencia.md`
**Tipo**: Arena — medição descartável; sem produto.

---

## A pergunta única

Recomputando os ciclos de módulo do egui contando **só** as arestas
`uses_kind == "reference"` (uso de tipo direto em assinatura/campo), o SCC
de **85 módulos** do laudo 0031 encolhe? Por quanto?

- Se encolher **muito**, os imports inflavam (Limite 4 da spec); o
  acoplamento "real" é o resíduo.
- Se encolher **pouco**, o acoplamento de tipo é genuíno; o egui é mesmo
  fortemente entrelaçado.

## Resposta

**SCC cai de 85 → 42**, perda de **43 módulos** (≈ 51%). O **import**
(Limite 4) inflava metade do SCC. O acoplamento de tipo "real" do egui
é um SCC de 42 módulos — pouco menos da metade do crate (38% dos 111
módulos).

## Distribuição de `uses_kind` (achado de pano de fundo)

| Categoria | Arestas | % das `uses` |
|---|---|---|
| `uses_kind = reference` (tipo direto) | 8 331 | **80%** |
| `uses_kind = import` (declaração `use` no módulo) | 2 098 | 20% |
| **Total `uses`** | 10 429 | 100% |

No `lente_core` (controle): 178 reference + 10 import = 188 (proporção
parecida — referência domina em quantidade).

## Sanidade (portão obrigatório)

| | Todas as `uses` (sanidade) | Só `reference` |
|---|---|---|
| Itens (nós) | 3 694 | 3 694 |
| Arestas no grafo | 13 937 | 11 839 |
| Módulos (agregado) | 111 | 111 |
| Deps módulo→módulo | **864** | **386** |
| **SCCs ≥ 2** | **1** | **1** |
| **Maior SCC** | **85** | **42** |

Sanidade: 85 reproduzido — bate com o laudo 0031. ✓

Controle (`lente_core`): 0 ciclos em ambas as versões — o método não
inventa ciclo. ✓

## Delta — quais módulos saíram

**43 módulos** estavam no SCC só por causa do import (Limite 4). Lista
parcial (em ordem alfabética):

```
egui::atomics              egui::containers::menu
egui::atomics::atom_ext    egui::containers::modal
egui::atomics::atom_layout egui::containers::panel
egui::containers           egui::containers::resize
egui::containers::close_tag egui::containers::scene
egui::containers::combo_box egui::containers::scroll_area
egui::debug_text           egui::containers::sides
egui::drag_and_drop        egui::containers::tooltip
egui::gui_zoom             egui::containers::window
egui::introspection        egui::load::texture_loader
… (+23 mais)
```

Observação qualitativa: vários módulos que **agrupam** widgets/containers
(`egui::containers`, `egui::atomics`) e vários containers individuais
(`menu`, `modal`, `panel`, `window`, `scroll_area`, …) saíram. Faz
sentido: tipicamente um módulo container importa tipos do anel só por
declaração, sem necessariamente **usar** os tipos em sua API exposta.

## Resíduo — o ciclo de referência real

**SCC remanescente de 42 módulos**, primeiros 10:

```
egui
egui::animation_manager
egui::atomics::atom
egui::atomics::atom_kind
egui::atomics::atoms
egui::atomics::sized_atom
egui::atomics::sized_atom_kind
egui::containers::area
egui::containers::collapsing_header
egui::containers::frame
… (+32 mais)
```

Esses 42 são o **coração** do entrelaçamento real do egui: módulos que
de fato têm tipos uns dos outros nas próprias APIs. A categoria
"containers" continua presente (`area`, `frame`, `collapsing_header`), o
que sugere que eles passam tipos do core (`Ui`, `Context`, `Response`,
`Layout`) em sua interface pública — coerente com o estilo de UI imediata.

A vista de ciclos passa a ser **mais útil**: focar em 42 módulos
nomeados é tratável; 85 era opaco demais.

## Interpretação

A medição mostra que o ciclo do laudo 0031 é uma **soma de duas
realidades**:

1. **Acoplamento de tipo real (42 módulos)** — refletido nas APIs
   públicas dos módulos; reflete o estilo de UI imediata; provavelmente
   irredutível sem refatoração do egui.
2. **Inflação por import (43 módulos a mais)** — declarações `use`
   no topo de módulos que importam algo do anel sem usá-lo
   estruturalmente; é o Limite 4 da spec.

A primeira é informação **arquitetural**. A segunda é, em boa parte,
**ruído de Limite 4**.

## Decisão que o número permite

| Caminho | Justificado? | Por quê |
|---------|--------------|---------|
| Mudar `lente_estrutura` para contar **só `reference`** (default ou opção) | **Justifica investigar** | Reduz o SCC a metade; foca em acoplamento de tipo; a vista melhora muito sem perder o essencial. |
| Manter `todas as uses` como default | Tem a vantagem | de espelhar exatamente o grafo `Uses` cru da spec. |
| Adicionar **opção** (não default) | **Compatível com o padrão** do escopo (laudo 0030): default conservador, usuário escolhe. |
| Estender `Aresta` para carregar `uses_kind` (mudança de produto) | Necessário se o filtro entrar | É baixo custo; só adiciona um campo opcional. |

A medição prepara a próxima decisão de produto — mas não a toma.

### Cenário esperado para o próximo prompt

Pelo padrão do projeto (laudo 0030 — escopo como escolha do usuário),
o próximo prompt natural é:

- Estender `Aresta` para carregar `uses_kind: Option<String>` (ou enum).
- Atualizar `desserializar_grafo` no L3 para ler o campo.
- Em `analisar_estrutura`, adicionar um parâmetro `modo_uses` análogo
  ao `escopo`: `Todas` (default) ou `SoReferencia`.
- Saída declara `modo_uses` igual ao `escopo`.
- O default deve ser **`Todas`** (preserva o que o laudo 0031 mediu) ou
  **`SoReferencia`** (vista mais útil)? Decisão do autor.

## Honestidade sobre o alcance

- A medição mede **só** o efeito do `uses_kind`. **Não** filtra
  `reexport` (o fork não rotulou esse caso separadamente — ver Achado
  de pano de fundo: apenas `import` e `reference` aparecem). Se um dia
  `reexport` virar valor distinto, um terceiro corte fica registrado.
- A medição **não decide** se `reference` é a vista certa para o
  produto — só mostra que a alternativa é informativa.
- O **escopo** (`SeuCodigo` vs `Completo`) não foi exercitado aqui
  porque o laudo 0031 já confirmou invariância dos ciclos ao escopo
  (stdlib é sorvedouro). O mesmo argumento vale aqui — esta medição
  rodou no escopo Completo só por simplicidade do dump.

## Arquivos

- `dados/export-egui.json` — `cargo modules export-json --sysroot
  --compact --lib --package egui` (fork commit `ddcd3ca`, com
  `uses_kind` e `position`).
- `dados/export-lente-core.json` — idem `lente_core` (controle).
- `src/main.rs` — programa de medição.

## Rodar de novo

```
cd lab/medicao-ciclos-referencia && cargo run --release
```

Os JSONs em `dados/` são da versão `egui` v0.34.3 com o fork pós-`b44aa96`.
Para refazer com versões diferentes, re-capturar com o `cargo modules`
atualizado.
