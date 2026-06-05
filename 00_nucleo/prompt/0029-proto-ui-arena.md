# Prompt: Protótipo de Arena para UI — consumir o JSON da lente (ranking + raio)

**Camada**: Arena (`lab/`) — experimento descartável, como `lab/medicao-egui`.
**Não** é componente; não entra no workspace de produção.
**Criado em**: 2026-06-02
**Estado**: `PROPOSTO`
**Decisões de origem**: início da trilha de UI. A lente já emite JSON (ranking
e raio por nó — confirmado em `02_shell/cli/src/saida.rs::formatar_json`). Uma
UI é **consumidor** desse JSON, não parte do núcleo — apresentação fora do
núcleo, mesmo princípio do ADR-0002 do Tekt (catálogo no L2). Método do
projeto: **protótipo de Arena antes de nuclear** (padrão do `lab/`).
**Pré-requisito**: a saída JSON da CLI (`--json`): ranking
(`posição`/`impacto`/`classificação`/`path`) e raio
(`alvo`/`classificacao`/`diretos`/`transitivos`/`impactados[]`); `lente_filtro`
e `lente_ranking` já construídos (laudos 0025/0027).
**Posição**: primeiro passo da UI. Descartável, para **aprender** o que é útil
e **o que falta no JSON** antes de nuclear um componente de UI de verdade.
**Arquivos afetados**: `lab/proto-ui/` (web: HTML/JS, **sem Rust**) + um dump
JSON capturado da CLI. Nenhum crate, nenhum `members`.

---

## Contexto

A pergunta da lente é "o que quebra se eu mexer neste nó?". A UI tem dois
elementos:
- **Ranking** como porta de entrada — onde olhar primeiro num crate.
- **Raio** de um nó — quem depende dele (o `montante`), com a classificação.

Entrega recomendada (a confirmar): **web separada** que lê o JSON da CLI. O
Rust continua sendo o produtor de JSON; a UI é uma superfície nova presa a esse
contrato. (Alternativas registradas, não escolhidas: Rust→WASM, que mantém tudo
em Rust mas acopla e pesa; terminal/ratatui, leve mas ruim para grafo. Web é a
mais coerente com "apresentação fora do núcleo".)

Achado conhecido que o protótipo precisa confirmar e usar: o raio em JSON traz
`impactados` como **lista plana de paths** — **sem** as profundidades (existem
na memória, no `montante`, mas não são emitidas) e **sem** as arestas entre os
impactados. Isso basta para uma UI de **lista**, não para desenhar o impacto
como **grafo**. O protótipo trabalha com o que há e **registra** o que faltaria.

---

## Restrições estruturais

- **Descartável, em `lab/`.** Não é produção. Sem crate novo, sem tocar o
  workspace, sem `members`.
- **Sem Rust.** É web (HTML/JS). Pode usar uma lib de tabela/grafo via CDN
  (ex.: d3), ou nada — o mínimo que mostre o dado.
- **Consome o JSON da CLI como está.** **Não** muda o contrato de JSON neste
  prompt. Se faltar algo (profundidade, arestas), **registrar** como achado —
  não consertar aqui.
- **Mínimo.** O objetivo é aprender, não polir. Qualidade de protótipo.

---

## Fase 1 — Capturar e confirmar o JSON real

1. Rodar e **salvar** dois dumps reais em `lab/proto-ui/dados/`:
   - Ranking: `lente --pacote egui --ranking --json --top 30` (egui v0.34.3),
     rodado do diretório do crate egui.
   - Raio por nó: `lente --pacote lente_core --alvo-id <N> --json` (escolher um
     nó-base do ranking do `lente_core`, ex.: o `Path`, que lidera).
2. Confirmar os campos de cada um (ranking: `posição`/`impacto`/`classificação`/
   `path`; raio: `alvo`/`classificacao`/`diretos`/`transitivos`/`impactados`).
3. **Registrar o que falta para uma vista de grafo**: o raio não traz
   profundidade por impactado nem arestas entre eles. Anotar isso como o achado
   que decide a próxima iteração (se vale estender o JSON, e como).

**Reportar nas notas do protótipo**: os campos observados e a lista do que
faltaria para uma vista de grafo ou em camadas por profundidade.

---

## Fase 2 — Protótipo

Uma página local (`lab/proto-ui/index.html` + JS) que carrega os dumps de
`dados/`:

- **Vista de ranking** (porta de entrada): tabela ordenável — posição,
  impacto, classificação, path. É o "onde olhar primeiro".
- **Vista de raio** (ao selecionar um nó): mostra a classificação, os números
  (`diretos`/`transitivos`) e o conjunto `impactados` como **lista** (quem
  depende do nó). A vista de **grafo** ou **em camadas por profundidade** fica
  **adiada** até o JSON trazer profundidade/arestas — deixar isso anotado na
  própria UI ou no README do protótipo.
- Ligação simples: clicar numa linha do ranking carrega o raio daquele nó (se
  houver dump dele) — ou, no mínimo, demonstrar a navegação com os dumps
  capturados.

---

## Critérios de Verificação

```
Dado os dumps reais do egui e do lente_core em lab/proto-ui/dados/
Quando a página é aberta
Então a vista de ranking renderiza e é ordenável (posição/impacto/classificação/path)

Dado um nó selecionado com dump de raio
Quando a vista de raio abre
Então mostra classificação, diretos, transitivos e a lista de impactados

Dado o protótipo
Então há uma nota (README ou na própria UI) registrando o que o JSON precisaria
  para uma vista de grafo (profundidade por impactado, arestas entre impactados)
```

(Não há suíte de testes — é Arena. A "verificação" é o protótipo renderizar o
dado real e a nota de achados existir.)

---

## Resultado esperado

- Um protótipo web descartável que torna a saída da lente **visível**: ranking
  como entrada, raio de um nó como lista.
- Uma nota escrita do que o JSON precisaria para uma vista mais rica (grafo ou
  camadas) — o achado que alimenta a decisão de estender (ou não) o contrato de
  JSON, e o desenho do componente de UI de verdade.
- Aprendizado suficiente para decidir a forma da UI real **antes** de nuclear.

---

## O que NÃO entra

- **Componente de UI de produção / nucleação**: vem **depois** do protótipo
  ensinar. Este é o experimento, não o produto.
- **Mudar o contrato de JSON** (emitir profundidade/arestas): só **registrar** a
  falta; consertar é prompt próprio, decidido pelo que o protótipo mostrar.
- **WASM ou terminal**: web escolhida; as alternativas ficam registradas.
- **Spec, E2, filtro de folhas**: outras trilhas.

---

## Observação metodológica

Arena antes de nucleação — o mesmo padrão de `lab/medicao-egui` (laudo 0021).
O protótipo é também uma **medição**: mede se o JSON é suficiente para desenhar
a lente, do mesmo modo que a medição do egui mediu a lente contra um crate
real. O ganho não é a UI bonita; é o achado sobre o contrato de JSON, barato de
obter desenhando contra dado real, e caro de adivinhar no escuro.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-02 | Protótipo de Arena para UI: página web que consome o JSON da CLI (ranking como entrada, raio de um nó como lista), contra dumps reais de egui e lente_core. Descartável, em `lab/`, sem Rust. Registra o que o JSON precisaria (profundidade/arestas) para uma vista de grafo. | `lab/proto-ui/{index.html,*.js,dados/*.json,README}`, `00_nucleo/lessons/0029-proto-ui-arena.md` |
