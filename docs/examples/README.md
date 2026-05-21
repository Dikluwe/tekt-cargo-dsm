# Exemplos de output do `crystalline-dsm`

Esta pasta contém um exemplo real de output gerado contra o
workspace do [Typst](https://github.com/typst/typst), usado como
caso de validação do Marco M2 (visualização).

## Conteúdo

### `typst-dsm.html`

HTML estático auto-contido (~350 KB) com a DSM completa do
Typst — 667 nós (443 internos + 224 externos), 8 667 imports,
18 ciclos.

Para visualizar localmente:

```bash
xdg-open docs/examples/typst-dsm.html   # Linux
open docs/examples/typst-dsm.html        # macOS
start docs/examples/typst-dsm.html       # Windows
```

Funciona offline em qualquer browser moderno (Chrome 125+,
Safari 17+, Firefox 125+ — para a Popover API; o `light-dark()`
está disponível desde 2024). Sem dependências externas.

### `screenshots/`

Capturas do HTML acima nos seis estados visuais principais
(janela 2 400 × 2 400 em Chrome headless).

| Ficheiro | O que mostra |
|---|---|
| `01-light.png` | Estado inicial em light mode — diagonal cinza, bordas vermelhas em SCCs cíclicos, pontos azuis representando imports (concentrados abaixo da diagonal — padrão DSM correcto). |
| `02-dark.png` | Mesmo HTML com `prefers-color-scheme: dark`. `light-dark()` resolve para a paleta escura; fundo `#1e293b`, células azuis brilhantes. |
| `03-hide-externals.png` | Filtro "Hide external nodes" activo. Os 224 nós externos desaparecem; restam 443 (internal_boundary do Typst). |
| `04-only-cyclic.png` | Filtro "Show only cyclic SCCs". Labels triviais ocultos; grande bloco vermelho central é o SCC do `typst-library` (~120 nós). |
| `05-search-layout.png` | Input de busca preenchido com "layout". Labels não-correspondentes ficam em `opacity: 0.15` via `[data-match="false"]`. |
| `06-tooltip.png` | Hover sintético sobre uma célula. Tooltip via Popover API mostra Row/Col/edge info; label realçado via `.is-hovered`. |

## Regeneração

Para regenerar o HTML contra qualquer workspace Cargo:

```bash
cargo run -p crystalline-dsm-cli -- <workspace_path> \
  --output /tmp/graph.json --emit-html
# Abre /tmp/dsm.html
```

Para regenerar especificamente contra o Typst (com o symlink
temporário do `Cargo.toml`, ver `00_nucleo/adr/`):

```bash
TYPST_PATH=/path/to/typst-original \
  cargo test -p crystalline-dsm-cli --test typst_smoke_test -- --ignored --nocapture
# Grava em /tmp/typst-dsm.html
```

## Métricas do exemplo

- **Pipeline total** contra Typst real: ~4,3 s.
- **Render HTML**: 62 ms.
- **Tamanho**: 348 KB.
- **Determinismo**: byte-determinístico para o mesmo input,
  excepto pelos campos `generated_at` e `tool.version`.
