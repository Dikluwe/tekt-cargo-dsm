# Prompt: raio na tela — clicar um módulo na DSM e ver o que ele alcança

**Camada**: L1 (`lente_estrutura`: cálculo novo pequeno — raio **por módulo**,
montante/jusante transitivos) + L4 (preencher) + L2 (a interação na vista
HTML). O motor do raio por item e o pipeline existentes **não mudam**.
**Criado em**: 2026-06-10
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0073-raio_na_tela.md`)
**Decisões de origem**:
- Laudo 0071, fila — "o humano, vendo a DSM, vai querer clicar um módulo e
  ver seu raio. Próxima trilha provável." Confirmado pelo autor (2026-06-10).
- Laudo 0072, magnitude — o recorte `seu-codigo` "lucra mais no raio por nó";
  esta tela é onde esse lucro chega ao humano.
- Proposta §2 — as duas perguntas da lente: hierarquia de risco e alcance da
  propagação. A DSM (0071) entrega a primeira; esta interação entrega a
  segunda **na mesma tela**.
- Padrão 0029/0036/0071 — a tela é vista; **cálculo no L1, nunca no JS**. O
  fecho transitivo que esta tela precisa é cálculo → nasce no L1.
**Pré-requisito**: estado pós-0072 (vista `--html` com default `seu-codigo`,
peso, fold; suíte 290 + ignorados verdes).
**Arquivos afetados (a confirmar na Fase 1)**: `01_core/estrutura` (raio por
módulo), `04_wiring` (preencher), `02_shell/cli` (`dsm_template.html` +
montagem), `02_shell/catalogo` (rótulos), testes.

---

## Contexto

A vista DSM mostra a forma; falta a pergunta da proposta sobre um ponto:
clicado um módulo, **o que o alcança e o que ele alcança** — montante (de
quem ele depende, transitivo: o que pode exigir mudança junto) e jusante
(quem depende dele, transitivo: o que pode sentir a mudança). Hoje isso
existe por item na CLI/MCP (`raio_do_alvo`); na tela, o humano precisa do
mesmo no nível em que está olhando: **módulo**.

### A nuance semântica que o prompt resolve antes do código

Há **duas** definições possíveis de "raio do módulo", e elas diferem:

1. **Fecho sobre o grafo agregado** (arestas módulo→módulo do 0031/0071):
   barato, mas **superestima** — se o item `a ∈ A` usa `b ∈ B` e o item
   `b' ∈ B` usa `c ∈ C`, o agregado diz `A ⇝ C` mesmo sem nenhum caminho
   real de item ligando `a` a `c`.
2. **Alcançabilidade no grafo de itens, projetada a módulos**: BFS
   multi-fonte a partir dos itens do módulo, sobre as arestas `Uses` de
   item (as mesmas do raio existente), e os módulos atingidos = a projeção.
   **Exata** no sentido da proposta ("o que está no raio de verdade").

A escolha de partida é a **2** (exata), porque a honestidade da lente é o
produto — uma tela que superestima o alcance erra para o lado que parece
seguro mas mente sobre a forma. A Fase 1 **mede** o custo da exata no egui;
se for cara (não deve ser: M módulos × BFS no grafo de itens é linear em
M·arestas), a 1 entra como fallback **com a superestimação declarada no
rótulo da tela**. Nas duas hipóteses, a semântica escolhida fica escrita na
interface e no laudo.

---

## Restrições estruturais

- **Cálculo no L1** (`lente_estrutura` ou peça nova ao lado): o JS recebe o
  raio pronto e só **pinta e lista**. Política inalterada do 0029.
- **Mesma fonte de verdade**: o raio por módulo deriva do **mesmo grafo** que
  a matriz mostra — mesmo escopo (default `seu-codigo`, 0072), mesmo modo de
  uses. Clicar não muda o universo; destaca dentro dele.
- **JSON aditivo**: se o dado entrar no `--estrutura --json` (decisão da
  Fase 1: emitir no JSON para as duas superfícies, ou embutir só no HTML),
  é campo novo, nada existente muda.
- **Pureza do L1**: BFS iterativa à mão (precedente do Kahn no 0035); sem
  deps novas.
- **Convenção Cristalina**: V1 = 0, V2 = 0 preservados; V12 = 1 inalterado.

---

## Fase 1 — Leitura e verificação (obrigatória)

1. **Onde estão as peças.** Ler como `analisar_estrutura` (L4) obtém o grafo
   de itens e o mapeamento item→módulo (a agregação do 0031 já faz essa
   conta). O raio por módulo deve **reusar** esse mapeamento, não recriá-lo.
2. **Medir a exata.** Implementar a alcançabilidade projetada (definição 2)
   e cronometrar no egui (111 módulos, escopo `seu-codigo`): se ficar na
   ordem de centenas de ms, é a escolhida; registrar o número. Só cair para
   a definição 1 com o número mostrando que a exata é inviável — e então o
   rótulo da tela declara a superestimação.
3. **Tamanho do dado.** Por módulo, duas listas de índices (na `ordem`):
   estimar o peso no JSON/HTML do egui (não deve passar de dezenas de KB).
   Decidir: campo `raios` no `--estrutura --json` (aditivo, serve agente e
   tela — preferido se o tamanho for razoável) ou só embutido no `DADOS` do
   HTML (registrar a razão se for este).
4. **Interação × fold.** Definir o comportamento ao clicar um **grupo
   dobrado**: a união dos raios dos membros (agregação de apresentação,
   coerente com a soma de pesos do 0071). Confirmar que isso é só
   união de conjuntos no JS (apresentação), não cálculo novo.

---

## Fase 2 — Construção

### L1 — o raio por módulo

- `raios_por_modulo(grafo, mapeamento) -> …`: para cada módulo, `jusante`
  (módulos com algum item que alcança itens deste, via `Uses` reversa) e
  `montante` (módulos alcançados pelos itens deste), **transitivos**,
  excluindo o próprio módulo; determinístico.
- Testes de unidade com grafos forjados pequenos cobrindo exatamente a
  nuance semântica: o caso `a∈A → b∈B`, `b'∈B → c∈C` **sem** caminho de
  item `a⇝c` — a exata **não** inclui `C` no jusante-reverso de `A`
  (este teste é o contrato da definição escolhida; se um dia alguém trocar
  pela agregada, ele grita).

### L4 — preencher

- `analisar_estrutura` calcula e anexa os raios ao resultado, no mesmo
  escopo/modo da estrutura pedida.

### L2 — a interação

- **Clicar um módulo** (rótulo ou célula da diagonal): pinta na matriz o
  jusante (uma cor) e o montante (outra), linha e coluna; painel lateral
  lista os dois conjuntos com contagens (`jusante: N módulos · montante: M`),
  paths completos, na `ordem`.
- **Clicar grupo dobrado**: união dos membros, painel indica que é grupo.
- **Limpar**: clicar fora / Esc / clicar o mesmo módulo.
- **Rótulo honesto** no painel (catálogo): a semântica escolhida
  ("alcançabilidade por item, projetada a módulos") e o limite §3 de sempre
  (estrutural, não comportamental — estar no raio não significa que quebra).
- Cores legíveis com o peso existente (o destaque não pode se confundir com
  a rampa de intensidade — decisão visual do executor, registrada).

### Testes

- L1: os de unidade acima (incluindo o teste-contrato da semântica).
- L2: a montagem HTML contém os raios e os rótulos do catálogo;
  determinística.
- E2E `#[ignore]`: vista do `lente_core` contém raios coerentes com o grafo
  pequeno conhecido.
- Não-regressão: `--text`/`--json` inalterados fora do campo novo (se ele
  entrar no JSON).

---

## Fase 3 — Uso real

Gerar a vista com raio para o **egui** e **um projeto seu corrente**, clicar
nos módulos que a DSM aponta como interessantes (o bloco de ciclo; os hubs
de maior peso) e registrar: a resposta "o que isso alcança" ajudou a decidir
algo? O painel listou o que você esperava? O que faltou — profundidade por
nível? raio por **item** dentro do módulo (drill)? — vira a fila seguinte.

---

## O que NÃO fazer

- **Não calcular no JS** — nem o fecho, nem nada além de pintar, listar e
  unir conjuntos prontos no fold.
- **Não mexer no raio por item existente** (`calcular_raio_de_alvo`) — outra
  granularidade, outro consumidor; intocado.
- **Não construir o drill para item** nesta passada — registrado como
  candidato se a Fase 3 pedir.
- **Não tocar o diff** nem o escopo do `impacto_do_diff` (deferido no 0072,
  trilha própria).
- **Não escolher a definição agregada por conveniência** sem o número da
  Fase 1 e o rótulo declarando a superestimação.

---

## Critérios de Verificação

```
Dado a vista HTML de um pacote e um clique num módulo
Então jusante e montante transitivos pintados na matriz e listados no painel
com contagens, no mesmo escopo/modo da matriz, com a semântica declarada

Dado o grafo forjado a∈A→b∈B, b'∈B→c∈C sem caminho de item a⇝c
Então o raio exato de A não inclui C (teste-contrato da definição 2)

Dado um grupo dobrado clicado
Então o destaque é a união dos membros e o painel declara o grupo

Dado --text/--json
Então inalterados fora do eventual campo aditivo de raios

Dado a suíte e o linter
Então verde, com o número EXATO de testes ignorados declarado no laudo
(disciplina pós-0068); V1 = 0, V2 = 0 preservados; V12 = 1; sem deps novas

Dado a Fase 3
Então a leitura registrada em dois projetos, com a fila seguinte nomeada
```

---

## Resultado esperado

- A segunda pergunta da proposta respondida na tela: forma (matriz) e
  alcance (raio) no mesmo lugar, com clique.
- A semântica do raio por módulo decidida por medição e escrita na
  interface.
- O dado de raios emitido (JSON aditivo ou HTML, com a razão registrada).
- **Laudo** em `00_nucleo/lessons/0073-…`: o número da medição exata-vs-
  agregada, a decisão do JSON, a leitura da Fase 3, o número exato de
  ignorados, e a fila (drill por item? profundidade? diff-na-tela?).

---

## Cuidados

- **A semântica é o produto.** O teste-contrato da definição 2 é a parte
  mais importante do prompt — ele impede a tela de mentir baratinho.
- **Registrar o número exato de ignorados** — o laudo 0071 e o 0072
  reportaram "ignorados verdes" sem o número; depois do episódio 28-vs-25
  (0068), o número exato em todo laudo é disciplina, não opção.
- **O destaque respeita o universo da matriz** — clicar não muda escopo nem
  modo; se o humano quiser o raio no `completo`, regenera a vista com
  `--completo`.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Raio na tela (fila do 0071; a 2ª pergunta da proposta §2 na mesma vista): clicar um módulo na DSM pinta e lista montante/jusante **transitivos**. Cálculo no L1 (`raios_por_modulo`, BFS à mão, precedente do Kahn 0035), **semântica exata por escolha**: alcançabilidade no grafo de itens projetada a módulos (a agregada superestima — caso `a∈A→b∈B, b'∈B→c∈C` sem `a⇝c`; teste-contrato trava a definição), com fallback agregado só se a medição da Fase 1 mostrar inviabilidade, e então com a superestimação declarada no rótulo. Mesmo escopo/modo da matriz (default `seu-codigo`, 0072); fold clicado = união (apresentação); painel com contagens + semântica + limite §3; JS só pinta/lista (padrão 0029). JSON aditivo ou só-HTML decidido na Fase 1 com tamanho medido. Fase 3 em dois projetos. Disciplina nova: número EXATO de ignorados em todo laudo (0071/0072 omitiram). Raio por item, drill e diff-na-tela fora — fila. | `01_core/estrutura` (raios_por_modulo + testes), `04_wiring` (preencher), `02_shell/cli/{saida.rs,dsm_template.html}`, `02_shell/catalogo` (rótulos), `00_nucleo/lessons/0073-raio_na_tela.md` |
