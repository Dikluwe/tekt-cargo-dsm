# Prompt: vista DSM de produção — o lado humano da lente (`--estrutura --html`)

**Camada**: L1 (mudança pequena: emitir o **peso** que `agregar_por_modulo`
hoje descarta) + L2 (a vista: template HTML autocontido + a montagem) + L4
(dispatch no `app`). O cálculo (ordenação, blocos, raio) **não muda**.
**Criado em**: 2026-06-10
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0071-vista_dsm_produto.md`)
**Decisões de origem**:
- Proposta §4, Momento A — o humano explorando o sistema para entender e
  decidir. Decisão do autor (2026-06-10): a boca MCP (0070) é a interface do
  **agente**; a interface do **humano** é a visual, e ela serve a **qualquer
  projeto Rust**, não à lente olhando para si.
- Laudo 0035 — a matriz como dado: `ordem` (topológica via condensação de
  SCCs) + `blocos` + `dependencias` no `--estrutura --json`. A parte difícil
  está pronta; a tela é apresentação.
- Laudo 0036 (Arena `proto-dsm`) — Achado 2: a matriz é **legível** em N=111
  (SVG, células ~5px, instantâneo). Achado 1: falta o **peso** por par
  módulo→módulo (conhecido em `agregar_por_modulo`, descartado). Achado 3:
  rótulos em N=111 exigem abreviação; produção precisa de **fold por
  prefixo**, expand e filtro por subárvore.
- Laudo 0029 (proto-ui) — padrão "a tela é uma vista; zero lógica própria".
**Pré-requisito**: `--estrutura --json` (0031–0035); a Arena `lab/proto-dsm`
como referência de implementação; estado pós-0070 (287 + 29 verdes).
**Arquivos afetados (a confirmar na Fase 1)**: `01_core/estrutura` (peso na
agregação), `02_shell/cli` (template + montagem HTML + flag), `04_wiring/app`
(dispatch), `02_shell/catalogo` (rótulos), testes.

---

## Contexto

A lente calcula a estrutura de um pacote (módulos, dependências, ciclos,
ordem de DSM) e emite texto e JSON. O humano que precisa **entender um
projeto Rust inteiro para decidir o que fazer** não tem como consumir isso —
a leitura da forma é visual: o triângulo de dependências, os quadrados de
ciclo na diagonal, as camadas vazias acima e abaixo. A Arena 0036 provou a
projeção contra dado real do egui; este prompt a promove a produto.

**Forma do produto: HTML autocontido.** A lente gera **um arquivo** (SVG +
JS + o dado embutidos; sem servidor, sem CDN, sem dependência de runtime) e
informa o caminho. Razões: é a forma que a Arena provou (proto-dsm é
exatamente isso, carregando o dump por HTTP — aqui o dado vai embutido);
zero deps novas pesadas (a montagem é texto sobre o JSON que já existe);
funciona em qualquer projeto Rust onde a lente rode; e abrir no navegador é
universal. A alternativa egui (app nativo, como o proto-impacto-diff) fica
explicitamente **não escolhida** nesta projeção — se uma projeção futura
(o diff em camadas) pedir interação que HTML não dê, a decisão se rediscute
com esse dado.

**Uma projeção primeiro** (proposta §5): esta é a DSM de estrutura. As
outras vistas (raio por nó, impacto de diff) **não entram** — viram
projeções seguintes, decididas pelo uso desta.

---

## Restrições estruturais

- **A tela é uma vista** (padrão 0029/0036): zero algoritmo no template. O
  JS só desenha `ordem`/`blocos`/`dependencias`/`peso` e dobra/desdobra
  rótulos. Qualquer cálculo novo que pareça necessário no JS é sinal de que
  pertence ao L1 — parar e registrar.
- **O JSON é o contrato, e a mudança nele é aditiva**: o campo `peso` entra
  nas `dependencias` (`{de, para, peso}`); nenhum campo existente muda.
  Consumidores atuais (agente via MCP/CLI, proto-dsm) não quebram.
- **Pureza do L1**: o peso é contagem na agregação que já existe — sem deps
  novas, `cargo tree -p lente_estrutura` (e core) inalterados.
- **Template embutido no binário** (`include_str!` ou equivalente) — a vista
  funciona em qualquer diretório, sem arquivos soltos para instalar.
- **Convenção Cristalina**: arquivos novos nascem com linhagem; V1 = 0,
  V2 = 0 preservados; V12 = 1 (`ErroLente`) inalterado.
- **stdout do modo `--html`**: decidir na Fase 1 entre (a) escrever arquivo
  e imprimir o caminho (sugestão: default `lente-estrutura.html` no cwd,
  `--saida <caminho>` opcional) ou (b) HTML no stdout para redirecionar.
  A opção (a) é a sugerida — coerente com "informar onde está"; registrar a
  escolha.

---

## Fase 1 — Leitura e verificação (obrigatória)

1. **Onde o peso morre.** Ler `agregar_por_modulo` (`lente_estrutura`) e
   confirmar o ponto exato em que N arestas-de-item colapsam numa
   aresta-de-módulo (Achado 1 do 0036). O peso = essa contagem. Verificar
   contra o egui real: os pares de maior peso fazem sentido (ex.:
   `egui::context` → `egui::style` com dezenas de arestas)?
2. **A Arena como referência, não como cópia.** Ler `lab/proto-dsm/index.html`
   (SVG, tooltips, molduras de bloco, seletor) e listar o que sobe a produto
   e o que muda (o dado embutido em vez de fetch; fold por prefixo novo).
3. **Nível workspace (a pergunta do "projeto como um todo").** O
   `--estrutura` hoje opera por **pacote**. Um projeto Rust real é
   frequentemente um workspace de vários crates. Verificar o caminho mais
   barato para uma vista do workspace inteiro: o grafo de workspace já
   existe (0045, do modo diff); a `ordenar_dsm` é genérica (laudo 0036,
   pendência multi-nível). **Se** agregar por crate sobre o grafo de
   workspace reusa as peças existentes sem algoritmo novo, incluir o nível
   `--workspace` nesta vista (um seletor módulo/crate no HTML). **Se exigir
   peça nova**, registrar como achado com o desenho do que faltaria e
   **não** expandir o escopo — o nível módulo por pacote já entrega valor.
4. **Tamanho do HTML embutido.** Estimar o peso do arquivo gerado para o
   egui (dump de 57 KB + template ~10 KB na Arena) — confirmar que
   autocontido é viável sem minificação esotérica.

---

## Fase 2 — Construção

### L1 — o peso (mudança pequena)

- `agregar_por_modulo` passa a contar: a aresta módulo→módulo carrega
  `peso: usize` (quantas arestas-de-item colapsaram nela).
- Propagar ao tipo de saída da estrutura e ao JSON (`{de, para, peso}`).
- Testes: agregação com pesos conhecidos; JSON com o campo; não-regressão
  dos consumidores de texto.

### L2 — a vista

- **Flag `--html`** no modo `--estrutura` (ortogonal a `--escopo`/
  `--so-referencia`/`--filtrar-stdlib` — a vista respeita e **declara** o
  escopo, como o 0030 exige de toda saída).
- **Template autocontido** com, no mínimo:
  - a grade N×N na `ordem`, células de `dependencias`;
  - **peso visível** (intensidade da célula e/ou número no hover — decisão
    do gerador; o requisito é que densidade seja distinguível de presença);
  - **blocos de ciclo emoldurados** na diagonal;
  - **fold por prefixo** (Achado 3): agrupar módulos por prefixo comum
    (`widgets::*`), clicar expande/contrai; rótulos abreviados (2 últimos
    segmentos) com path completo no tooltip;
  - **cabeçalho honesto**: pacote, escopo, modo de uses, contagens
    (módulos/dependências/ciclos), e a declaração de limite da proposta §3
    (estrutura **estática, estrutural** — não comportamental);
  - tooltip de célula: `linha → coluna` com paths e peso.
- **Sem estado persistente** no navegador (fold em memória).
- Strings de rótulo no **catálogo** (ADR-0002), como o resto da CLI.

### L4 — dispatch

- O `app` roteia `--estrutura --html` para a montagem nova; `--text`/`--json`
  intocados.

### Testes

- Unidade (sem fork): a montagem HTML sobre `EstruturaModulos` forjada —
  contém o dado embutido, o cabeçalho com escopo, os rótulos do catálogo;
  determinística.
- O JSON com `peso` (contrato aditivo, desserializável).
- E2E `#[ignore]` (convenção): `lente --pacote lente_core --estrutura --html`
  gera arquivo que contém a grade do dado real.
- Não-regressão: `--text`/`--json` byte-iguais fora do campo novo.

---

## Fase 3 — Uso real (o critério que importa)

Gerar a vista de **dois projetos que não são a lente** (sugestão: o egui —
o difícil conhecido — e um projeto seu corrente) e registrar no laudo, com
capturas ou descrição:

- a leitura "como um todo" funcionou? (camadas visíveis, bloco de ciclo
  saltando, peso distinguindo acoplamento forte de fraco);
- o fold tornou N>100 navegável?
- o que faltou para **decidir o que fazer** a partir da tela — esse registro
  é a fila das próximas projeções (raio na tela? diff na tela? nível
  workspace, se ficou de fora?).

---

## O que NÃO fazer

- **Não construir servidor, app nativo (egui) ou framework JS** — um arquivo
  autocontido.
- **Não pôr lógica no JS** além de desenhar e dobrar.
- **Não expandir para as outras projeções** (raio visual, diff visual) — uma
  projeção; as outras vêm pelo que a Fase 3 mostrar.
- **Não forçar o nível workspace** se a Fase 1 mostrar que exige peça nova —
  registrar o desenho e parar.
- **Não tocar pipelines de cálculo** além do peso na agregação.
- **Não remover a Arena** `proto-dsm` — referência histórica; o produto não
  depende dela.

---

## Critérios de Verificação

```
Dado lente --pacote <X> --estrutura --html em qualquer projeto Rust com fork
Então gera UM arquivo autocontido (sem rede, sem CDN) e informa o caminho

Dado o arquivo aberto no navegador
Então a matriz N×N na ordem topológica, blocos de ciclo emoldurados, peso
distinguível de presença, fold por prefixo funcionando, cabeçalho declarando
pacote/escopo/uses e o limite estrutural-não-comportamental

Dado o JSON do --estrutura
Então dependencias carregam peso (aditivo); consumidores existentes intactos

Dado --text e --json sem --html
Então comportamento idêntico ao atual (fora do campo peso no JSON)

Dado a suíte e o linter
Então verde (287 + novos; ignorados + E2E novo); V1 = 0, V2 = 0 preservados;
V12 = 1 inalterado; lente_core e lente_estrutura sem deps novas

Dado a Fase 3
Então a vista gerada para dois projetos externos à lente, com a leitura
registrada no laudo
```

---

## Resultado esperado

- `lente --pacote <X> --estrutura --html`: a primeira interface visual do
  produto, autocontida, funcionando em qualquer projeto Rust.
- O peso de acoplamento emitido (Achado 1 do 0036 fechado) e visível.
- Fold por prefixo (Achado 3 fechado na produção).
- A decisão do nível workspace tomada com dado (incluído se barato;
  desenhado e registrado se não).
- **Laudo** em `00_nucleo/lessons/0071-…`: as decisões da Fase 1, o uso real
  da Fase 3, e a fila das próximas projeções ditada por esse uso.

---

## Cuidados

- **O peso é a única mudança de cálculo** — e é contagem, não algoritmo.
  Qualquer coisa além disso no L1 é sinal de escopo errado.
- **A vista declara o escopo** (0030) e o limite (§3) — a honestidade é
  parte da interface, como nas descrições do MCP (0070).
- **Fase 3 em projetos que não são a lente** — o requisito do autor é
  entender projetos Rust em geral; validar só na lente seria validar no
  caso fácil e conhecido.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Vista DSM de produção (Momento A — a interface do humano, decisão do autor; o MCP 0070 é a do agente): `lente --pacote <X> --estrutura --html` gera HTML **autocontido** (SVG + JS + dado embutidos, sem servidor/CDN/deps novas) com a matriz na `ordem` topológica (0035), blocos de ciclo emoldurados, **peso** de acoplamento (Achado 1 do 0036: contagem que `agregar_por_modulo` descartava, agora emitida — mudança L1 pequena e aditiva no JSON `{de,para,peso}`) e **fold por prefixo** (Achado 3). Forma provada pela Arena `proto-dsm` (Achado 2: legível em N=111); a tela é vista, zero lógica no JS (padrão 0029). Cabeçalho declara pacote/escopo/uses e o limite estrutural-não-comportamental (§3, 0030). Fase 1 verifica o ponto da agregação, o caminho barato (ou não) para o nível **workspace** (grafo 0045 + `ordenar_dsm` genérica — incluir só se reusar peças; senão registrar o desenho) e o tamanho do autocontido. Fase 3: uso real em **dois projetos externos à lente**, com a leitura registrada — a fila das próximas projeções (raio/diff visuais) sai daí. Uma projeção só; egui-nativo explicitamente não escolhido nesta. | `01_core/estrutura` (peso), `02_shell/cli` (template + montagem + flag), `02_shell/catalogo` (rótulos), `04_wiring/app` (dispatch), testes, `00_nucleo/lessons/0071-vista_dsm_produto.md` |
