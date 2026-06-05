# proto-dsm — protótipo de Arena (laudo 0036)

Página web descartável que **desenha a matriz DSM** a partir da saída
`lente --pacote X --estrutura --json`. Sem build, sem CDN, sem framework
— SVG inline + JS embarcado num único `index.html`.

**Não é produção.** Vive em `lab/` por design — é experimento para
**aprender se a matriz é legível e o que falta no JSON** antes de
nuclear uma tela DSM de verdade.

---

## Como rodar

`fetch()` em arquivos locais é bloqueado pelo navegador (CORS). Precisa
de um servidor HTTP mínimo:

```bash
cd lab/proto-dsm
python3 -m http.server 8080
# abrir http://localhost:8080/
```

---

## O que a página mostra

- **Grade N×N** desenhada em SVG.
  - Linhas e colunas seguem a `ordem` (topológica) emitida pelo
    `--estrutura --json` (laudo 0035).
  - Células marcadas (preto) vêm de `dependencias` (`{de, para}`).
  - **Blocos de ciclo** (`blocos` no JSON) recebem **moldura laranja**
    em torno do quadrado contíguo na diagonal.
  - Diagonal `i==j` é destacada por cor de fundo (clara) — referência
    visual; não são deps.
- **Cabeçalho** mostra o metadado: número de módulos, deps, ciclos,
  `escopo`, `modo_uses`.
- **Tooltip por hover**: ao passar o mouse sobre uma célula, mostra
  `linha → coluna` com paths completos.
- **Seletor de dump**: troca entre os 3 dumps capturados em `dados/`.

---

## Dumps em `dados/`

| Arquivo | O que é | Cmd usado |
|---|---|---|
| `estrutura-egui-so-referencia.json` | egui v0.34.3 com filtro Limite 4 | `lente --pacote egui --estrutura --so-referencia` (cwd: `<egui>/crates/egui`) |
| `estrutura-egui-todas.json` | egui sem filtro (default) | idem sem `--so-referencia` |
| `estrutura-lente-core.json` | Controle pequeno (7 módulos, 0 ciclos) | `lente --pacote lente_core --estrutura` |

---

## Achados (resumo — laudo 0036 tem o detalhe)

### 1. As `dependencias` chegam como pares binários (sem peso)

Lattix e Structure101 põem **números** nas células (densidade de
acoplamento: quantos itens distintos estão por trás de cada par
módulo→módulo). O JSON do `--estrutura` hoje só emite presença
(`{de, para}`); o peso **existe na agregação** (`agregar_por_modulo`
colapsa N arestas-de-item numa aresta-de-módulo) e é **descartado**.

Para uma matriz densidade-aware (intensidade por célula), o JSON
precisa de algo como:

```json
"dependencias": [
  { "de": "egui::context", "para": "egui::style", "peso": 23 },
  …
]
```

Mudança pequena de produto — decidida em prompt próprio se o achado
deste protótipo mostrar que vale.

Esta tela renderiza **apenas presença**. As células ficam todas da
mesma cor, sem gradiente.

### 2. A matriz é legível em 111

Visualmente: bloco de 42 emoldurado contígua e claramente na diagonal
no modo `--so-referencia`; 55 módulos como camada de base **abaixo**
do bloco; 14 módulos derivados **acima**. Lê em ~5 segundos como
fronteiras de camada.

No modo `todas`, o bloco infla para 85 — quase toda a matriz vira o
quadrado denso. Visualmente confirma o achado dos laudos 0033/0034 (o
import infla, o filtro `--so-referencia` melhora a legibilidade
arquitetural).

### 3. Rótulos em N=111 forçam abreviação

Os paths são longos (`egui::widgets::text_edit::builder`). Para caber
nos eixos, a tela mostra apenas os 2 últimos segmentos
(`text_edit::builder`). O path completo aparece no tooltip de hover.
Uma tela de produção precisaria de:
- expandir/contrair rótulos sob demanda,
- ou agrupar por subárvore (`widgets`, `containers`, …) com
  fold-by-prefix.

---

## O que NÃO está aqui

- **Tela de produção** (a partir deste protótipo).
- **Gradiente por peso** (depende de achado 1).
- **Reorder interativo** (drag-and-drop de blocos): a ordem é fixa, vem
  do `--estrutura --json`.
- **Comparação entre dois snapshots** (diff arquitetural).
- **Empacotamento como MCP/tool de agente**: o JSON já é o insumo do
  agente.
- **Testes automatizados** ou polimento.

---

## Convenção de aposentadoria

Padrão da Arena (laudos 0021 / 0027 / 0029):

- O **componente** que esta Arena vier a inspirar (uma tela DSM real,
  ou enriquecimento do JSON com peso) vive no workspace ou em outro
  lugar de produção.
- A Arena **fica** no `lab/` como registro do experimento. Se a tela
  nuclear, atualizar o laudo 0036 dizendo qual componente nasceu;
  manter este protótipo intocado.

A tela é o **lado humano** da matriz; o JSON do laudo 0035 já é o
**lado máquina** (e o lado de agente). Mesmo dado, duas superfícies.
