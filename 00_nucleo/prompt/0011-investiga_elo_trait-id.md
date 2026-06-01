# Prompt: Investigação do Elo trait↔id (faseada)

**Tipo**: Experimento de Arena (`lab/`)
**Camada**: bancada — sem linhagem obrigatória. Resultado é evidência para
decidir onde corrigir a imprecisão da D4 do laudo 0010.
**Criado em**: 2026-05-27
**Decisões de origem**: laudo 0010 (lente_resolve), D4 — a evidência
`ImplDeTraitsDiferentes` carrega os traits mas não diz qual id corresponde a
qual, então a nomeação por trait pode ficar trocada.
**Pré-requisito**: JSONs das medições salvos em
`lab/medicao-colisoes/remedicao/json/`; fontes do typst disponíveis em
`lab/typst-original/`.

---

## A pergunta da investigação

Quando há colisão de path entre dois métodos de impls de traits diferentes
(ex.: `Display::fmt` e `Debug::fmt` do mesmo tipo), o `lente_resolve` quer
nomear cada cópia com seu trait (`<Display>`, `<Debug>`). Mas:

- A E2 lê o **código-fonte** — vê os traits, mas o texto **não tem ids**.
- O JSON do fork tem os **ids** distintos das cópias, mas **não tem o trait**
  (foi o que o fork colapsou).

**Existe alguma informação, disponível ao `lente_investiga`, que ligue um
trait específico a um id específico, sem depender de ordem (que é frágil)?**

Se existe e é confiável → a correção mora no `lente_investiga` (correlacionar).
Se não existe → a correção mora no fork (emitir o trait junto com o nó).

Esta investigação responde isso. Não constrói a correção — só descobre onde
ela deve morar.

---

## Fase 1 — Inspeção manual dos casos conhecidos (obrigatória)

Identificar os casos `Display+Debug` (e similares de traits distintos) nos
JSONs já salvos. A medição (ADR-0005) contou 9 casos `Debug+Display` e ~31
casos `::fmt`. Pegar uma amostra desses (todos os 9 `Display+Debug` se
viável, ou ao menos 5-6 variados).

Para cada caso, **manualmente** (lendo, não automatizando ainda):

1. Localizar os dois nós colidentes no JSON: mesmo path, ids distintos
   (ex.: `ErroRaio::fmt` ids 100 e 101).
2. Anotar **todos** os campos de cada nó no JSON: id, path, name, kind,
   visibility. Ver se algum campo difere entre as duas cópias.
3. Anotar a **vizinhança** de cada nó: as arestas com `id_from`/`id_to`
   apontando para cada id. Ver se a estrutura de arestas difere de forma
   que indique qual é qual.
4. Abrir o **código-fonte** correspondente (o arquivo `.rs` onde o tipo é
   definido), e determinar a verdade: qual impl (`Display` ou `Debug`)
   corresponde a cada cópia. **Esta determinação é o gabarito** — feita por
   leitura humana cuidadosa.
   - Importante: ao construir o gabarito, anotar **como** você soube qual é
     qual. Foi pela ordem dos impls no arquivo? Por algum atributo? Pela
     vizinhança? O sinal que o humano usa para construir o gabarito é o
     candidato a sinal automatizável.

5. Para cada caso, registrar: os campos do JSON, a vizinhança, o gabarito, e
   **qual sinal (se algum) no JSON correlaciona com o gabarito**.

### Resultado da Fase 1

Uma tabela, um caso por linha:

| Caso | id A | id B | visibility A/B | vizinhança difere? | gabarito (qual id é qual trait) | sinal que correlaciona |
|------|------|------|----------------|--------------------|---------------------------------|------------------------|

E uma conclusão de uma das três formas:

- **(a) Há um sinal claro no JSON** que liga id a trait (ex.: "o id menor é
  sempre o trait que aparece primeiro alfabeticamente", ou "a visibilidade
  difere e indica", ou "a vizinhança tem um padrão"). → Vale a Fase 2 para
  confirmar generalização.
- **(b) O único sinal é a ordem** (id na ordem textual dos impls), que é
  frágil e não garantido pelo fork. → A correção mora no fork; não vale Fase 2.
- **(c) Não há sinal nenhum** — os dois nós são indistinguíveis no JSON
  exceto pelo id arbitrário. → A correção mora no fork; não vale Fase 2.

---

## Fase 2 — Automação (somente se a Fase 1 concluir (a))

**Só executar se a Fase 1 encontrar um sinal promissor.** Se a Fase 1
concluir (b) ou (c), pular a Fase 2 e ir direto para a conclusão.

Se houver sinal candidato:

1. Construir gabarito manual para um conjunto maior de casos (não só os 5-6
   da Fase 1 — idealmente todos os ~31 `::fmt` e alguns dos
   `From<X>+From<Y>`, etc.). O gabarito continua sendo leitura do fonte,
   mas agora para mais casos.
2. Escrever código que aplica o sinal candidato (identificado na Fase 1) a
   cada caso e prediz qual id é qual trait.
3. Medir a **taxa de acerto** do sinal contra o gabarito.
4. Reportar: o sinal acerta X% dos casos. Onde erra, qual o padrão do erro?

### Resultado da Fase 2

- Taxa de acerto do sinal.
- Se alta (>95%): o sinal é confiável, a correção pode morar no
  `lente_investiga` usando esse sinal.
- Se média ou baixa: o sinal não é confiável o suficiente; a correção mora
  no fork.

---

## Restrições

- **Não modificar nenhum crate do projeto** (`lente_core`, `lente_infra`,
  `lente_investiga`, `lente_resolve`). Investigação, não construção.
- **Não modificar o fork.** A investigação pode concluir que o fork precisa
  mudar, mas não muda nada — só conclui.
- **Tudo em `lab/`** (sugestão: `lab/investiga-elo-trait-id/`). Não toca os
  outros experimentos.
- **Sem alterações em L0.** A decisão sobre onde corrigir é do autor, com
  base no relatório.

---

## Resultado esperado

Um relatório curto em `lab/investiga-elo-trait-id/relatorio.md` com:

1. A tabela da Fase 1 (casos inspecionados, campos, gabarito, sinal).
2. A conclusão da Fase 1: (a), (b), ou (c).
3. Se Fase 2 foi feita: a taxa de acerto do sinal e onde erra.
4. **Recomendação clara de onde mora a correção**: no `lente_investiga`
   (se há sinal confiável) ou no fork (se não há). Com a justificativa
   baseada nos dados.

A decisão fica com o autor; o relatório descreve e recomenda.

---

## Por que esta investigação importa

A decisão "corrigir na raiz" (escolhida no laudo 0010) só pode ser executada
quando se sabe **onde** é a raiz. Esta investigação responde isso com dado,
em vez de assumir. Se a raiz for o fork (provável, dado que o fork foi quem
colapsou a informação), a correção é mais uma rodada no fork — análoga à da
identidade-por-nó. Se a raiz for o `lente_investiga` (há sinal aproveitável
no JSON atual), a correção é local e não toca o fork.

Antes desta investigação, não há como saber qual das duas, e construir a
correção no lugar errado seria trabalho perdido.
