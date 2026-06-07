# Prompt: Protótipo visual do impacto de um diff (Arena)

**Tipo**: Experimento de Arena (`lab/`) — protótipo visual. Primeira
exploração da trilha local na tela.
**Camada**: bancada (sem linhagem obrigatória).
**Criado em**: 2026-06-04
**Decisões de origem**: briefing da trilha local; laudo 0037 (`No.position`
consumido); decisões do autor nesta conversa: protótipo **visual** na Arena
(não no produto); apresentação **em camadas** (aproximar para ver nós mais
finos); ler o diff pelos **dois caminhos** (stdin e invocando `git`) para
comparar.
**Pré-requisito**: fork atualizado (5ª rodada, emite `position`) instalado em
PATH; o protótipo roda sobre o **próprio repositório da lente**.
**Posição**: primeiro protótipo da trilha local. Mede a ideia (diff → nós
tocados → raio → camadas) sobre dado real, **antes** de decidir a forma do
produto.

---

## Contexto

A trilha local mostra **o que uma mudança toca** antes de um agente executar o
comando. O cálculo já existe: `lente_core::domain::raio::calcular_raio(grafo,
&alvo)` devolve o `montante` (quem depende do alvo, com profundidade) e a
`classificacao`. O laudo 0037 fez o `No` carregar `position` (`Option<Posicao>`
com `file`/`start_line`/`end_line`). Faltava casar um diff aos nós e mostrar.

Este é o **primeiro protótipo** da trilha local na tela. O autor escolheu uma
vista **visual em camadas**: começa no nível de cima (o módulo que foi tocado)
e, ao aproximar, aparecem os nós mais finos (a função, e o que está abaixo
dela), cada um com o seu raio. É a ideia "multi-nível" e o princípio "revelar
aos poucos" da proposta.

Isto é **Arena — descartável**. Não é o produto. Não há modo `--diff` na CLI,
não há MCP. O objetivo é **medir** a ideia sobre diffs reais do repo da lente,
para então decidir a forma do produto (CLI ou visual; qual input). Conforme a
proposta: "descobrir como mostrar, testando", sobre dados reais já organizados.

O autor também pediu para **comparar os dois caminhos de input** (stdin e a
lente invocando `git diff`) antes de comprometer. Isso é exatamente o que a
Arena serve: medir a diferença antes de mexer no produto.

---

## Restrições (regime de Arena)

- **Arena (`lab/`)**: regime relaxado. Sem linhagem obrigatória, sem cabeçalho
  `@prompt` no programa. Pode importar `lente_infra` e `lente_core` (e
  `lente_wiring` se quiser) por path. Pode usar deps externas (`egui`/`eframe`
  para o visual — espelhar a montagem do `lab/proto-dsm`/`lab/proto-ui`).
- **Não modificar nenhum crate do sistema (L1–L4).** Se um bug aparecer,
  registrar; corrigir é outro prompt. (O `No.position` já existe desde o 0037.)
- **Só leitura do repo analisado.** O protótipo roda o fork e lê o `git diff`;
  não escreve nada no repositório.

---

## O que o protótipo faz (o pipeline)

### 1. Obter o grafo do crate alvo, com `position`

- Alvo: **um crate** do repo da lente (argumento; padrão sugerido `lente_core`).
  Um crate só mantém o primeiro protótipo limitado. Multi-crate é refinamento.
- Extrair via `lente_infra::extrair_grafo` (recebe nome do pacote ou caminho do
  manifesto — o gerador confirma a assinatura). O grafo agora traz `position`
  por nó (laudo 0037).
- **Resolução de colisões — decisão do gerador (mesma questão do prompt 0021).**
  Para o **primeiro** protótipo, recomendo usar o **grafo cru** (sem rodar
  `lente_investiga`/`lente_resolve`): o mapeamento diff→nós usa `position`, que
  não depende de resolução. Registrar quantas colisões de path o crate alvo tem
  (paths com 2+ nós). Se forem poucas — provável no repo da lente — o raio fica
  bom o bastante para a exploração. Se forem muitas, resolver é refinamento de
  uma próxima rodada, não deste protótipo. (Se preferir já resolver, replique a
  composição na Arena — `extrair` + `investigar` + `aplicar` — e confirme que a
  resolução **preserva** `position` nos nós renomeados.)

### 2. Ler um `git diff` — pelos dois caminhos, para comparar

Esta é a comparação que o autor pediu. Suportar os dois e registrar a diferença:

- **(i) De stdin**: `git diff | <prototipo> …` — o programa lê o diff do stdin.
- **(ii) Invocando `git`**: o programa roda `git diff HEAD` (captura mudanças
  encenadas e não-encenadas contra o último commit) via `std::process::Command`
  no diretório do repo.

Em ambos, parsear os cabeçalhos de hunk `@@ -a,b +c,d @@` e os nomes de arquivo.
**Usar as faixas do lado novo (`+c,d`)**, não as do lado velho: o fork analisou
a árvore de trabalho **atual** (com a edição), e os números do lado novo
correspondem a ela.

Registrar no relatório: os conjuntos `(arquivo, faixa-de-linhas)` saem iguais
pelos dois caminhos? Há diferença com arquivos novos (untracked), renomeados, ou
encenados-versus-não? Qual caminho é mais confiável/cômodo?

### 3. Relativizar o caminho

`position.file` é **absoluto** (ex.: `/home/.../01_core/src/entities/grafo.rs`);
o `git diff` traz caminhos **relativos** à raiz do repo (ex.:
`01_core/src/entities/grafo.rs`). Torná-los comparáveis (tirar o prefixo da raiz
do `position.file`, ou tornar os do diff absolutos com a raiz do repo).
Registrar o método — é a fonte de bug mais provável da trilha (briefing §7).

### 4. Mapear diff → nós tocados, com a cadeia de camadas

- Para cada `(arquivo, faixa-de-linhas)` alterado, achar os nós cujo
  `position.file` casa e cujo `[start_line, end_line]` **intersecta** a faixa.
- **Camadas (decisão do autor):** quando uma linha cai em vários nós encaixados
  (um módulo que contém uma função, que contém …), guardar a **cadeia inteira de
  contenção**, do mais externo ao mais interno. A cadeia vem das arestas `Owns`
  (no `Raio`, `owns_pai`/`owns_filhos`; ou seguindo `Owns` no grafo) e/ou do
  encaixe dos spans de `position`. O nó mais interno é o item específico tocado;
  a cadeia até o módulo são as camadas.

### 5. Calcular o raio por camada

- Para cada nó da cadeia (no mínimo o mais interno; idealmente cada um),
  `calcular_raio(&grafo, &no.path)` → `Raio`.
- O raio muda por camada: a função mais interna tem um `montante` específico; o
  módulo que a contém, um mais largo. É isso que "aproximar para ver mais"
  revela — nós mais finos, cada um com o seu raio.

### 6. Apresentar em camadas (egui)

- **Vista de partida**: a camada de cima — o(s) módulo(s) tocado(s). Mostrar o
  nó, a `classificacao`, e o tamanho do `montante` (quantos dependem dele).
- **Aproximar / aprofundar** (clique ou zoom): revelar os nós mais finos dentro
  (a função tocada, depois itens aninhados), cada um com o seu `montante`.
- **Honestidade visual** (proposta): deixar claro que isto é impacto
  **estrutural** (quem depende, via `Uses`), **não** comportamental (o que
  quebra em runtime). Rotular isso na tela.
- A tela pode ser **tosca** — é Arena. O ponto é sentir o aprofundar em camadas
  sobre um diff real. Espelhar o andaime egui do `proto-dsm`.
- Ao selecionar um nó, mostrar o seu `montante` (quem depende, com
  profundidade) — o conjunto "o que esta mudança toca". A forma (anéis de
  propagação, lista aninhada, etc.) é escolha do gerador; **as camadas com
  aprofundar são o requisito**.

---

## As perguntas que o protótipo deve ajudar a responder

É o propósito da Arena: medir para decidir a forma do produto. Responder no
relatório, sobre diffs reais que você fez no repo:

- **Comparação de input (a decisão pendente do autor):** stdin contra
  invocar `git diff HEAD` — os conjuntos `(arquivo, faixa)` casam? Diferenças com
  untracked / renomeado / encenado? Qual é mais confiável e mais cômodo?
- **Relativização:** casou limpo, ou houve descasamento (raiz do workspace
  versus raiz do crate, symlink, etc.)?
- **Camadas:** o aprofundar lê bem? Na camada do módulo, dá para entender o
  impacto em ~10 segundos (o teste da proposta)? Descer até a função acrescenta
  detalhe útil ou vira ruído?
- **O mapeamento em si:** para uma edição conhecida (mexer no corpo de uma
  função), o protótipo acha o nó mais interno e a cadeia certos? Algum nó
  faltando (ex.: itens gerados por macro, cuja `position` é o call-site —
  briefing §5)? Algum falso-positivo?
- **Bordas da intersecção:** o que acontece numa edição em nível de módulo
  (entre itens)? numa edição que cruza vários itens? numa função nova (linhas
  adicionadas)?
- **Honestidade:** a natureza só-estrutural fica clara, ou um leitor poderia
  confundir com impacto comportamental?

---

## Estrutura sugerida

- `lab/proto-impacto-diff/Cargo.toml`: bin; deps por path para `lente_infra`
  (extração) e `lente_core` (`Grafo`, `No`, `Posicao`, `Aresta`, `Raio`,
  `Classificacao`, `calcular_raio`); `egui`/`eframe` (mesma versão do
  `proto-dsm`).
- `lab/proto-impacto-diff/src/main.rs`: argumentos (crate alvo; modo de input
  `stdin`|`git`; caminho do repo), o pipeline acima, a janela egui.
- `lab/proto-impacto-diff/relatorio.md`: as respostas às perguntas acima, sobre
  diffs reais. Descrever (e/ou capturar tela d)o que você viu.

Padrão de Arena: o conteúdo denso mora no `lab/` (o `relatorio.md`); o laudo em
`00_nucleo/lessons/0038-…` é o **registro** de que rodou, com sumário e ponteiro.

---

## Resultado esperado

- Um programa egui em `lab/proto-impacto-diff` que, dado um crate alvo e um
  `git diff` (stdin ou invocado), mostra **em camadas** os nós tocados e o raio
  de cada um, com o aprofundar revelando os mais finos.
- `relatorio.md` com as observações (comparação de inputs, qualidade das
  camadas, acertos e erros do mapeamento, honestidade visual, colisões no crate
  alvo).
- Laudo em `00_nucleo/lessons/0038-…` registrando a execução (sumário +
  ponteiro), padrão Arena.
- **Nenhum crate do sistema modificado.** Se precisou de função nova no L4,
  registrar como decisão e dívida; preferir replicar na Arena.

---

## Cuidados (da trilha local e da Arena)

- **Fork atualizado em PATH.** Se o binário for antigo, não vem `position` → o
  mapeamento acha zero nós. Dar diagnóstico claro ("`position` ausente — fork
  desatualizado?"), como o E2E do 0037, em vez de tela vazia muda.
- **Caminho absoluto contra relativo** é a fonte de bug mais provável (briefing
  §7). Relativizar com cuidado; registrar o método.
- **Lado novo do diff** (`+c,d`), não o lado velho — o fork analisou a árvore
  de trabalho atual.
- **Macro call-site**: itens gerados por macro têm `position` do call-site. Uma
  edição no call-site casa com eles; uma edição na definição da macro pode não
  casar como o esperado. Registrar se aparecer.
- **Limite estrutural** (briefing §7, Limite 3): o raio é estrutural (`Uses`),
  não comportamental. A tela precisa dizer isso (honestidade visual).
- **Determinismo**: `position` é determinística. A instabilidade residual entre
  extrações é só o `id` do petgraph (pré-existente, no schema do fork) — não
  atribuir ao protótipo.
- **Grafo cru (sem resolução)**: paths colididos podem dar raio impreciso (laudo
  0016). Registrar a contagem de colisões no crate alvo. Resolver é refinamento
  de uma próxima rodada, não deste protótipo.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Protótipo visual de Arena para o impacto de um diff: extrai o grafo de um crate da lente (com `position`, do laudo 0037), lê um `git diff` por stdin e invocando `git` (para comparar), relativiza o caminho, mapeia diff→nós com a cadeia de contenção, calcula o raio por camada, e mostra em camadas no egui (aprofundar revela os mais finos). Descartável; mede a ideia da trilha local sobre dado real antes de decidir a forma do produto. | `lab/proto-impacto-diff/{Cargo.toml,src/main.rs,relatorio.md}`, `00_nucleo/lessons/0038-proto-impacto-diff.md` |
