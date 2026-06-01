# ADR-0004: Resolução de colisões de path por cascata vizinhança → código-fonte

**Status**: `PROPOSTO`
**Data**: 2026-05-27

---

## Contexto

O laudo do prompt 0003 (adaptador L3) descobriu, ao tentar processar
`lente_core`, que o JSON do fork pode conter **paths colidentes**: dois nós
distintos com o mesmo `path` canônico. O caso concreto: `ErroRaio` tem
`impl Display` (escrito) e `#[derive(Debug)]` (gerado), e ambos declaram um
método `fmt` que o `cargo-modules` emite com o mesmo `path`
(`lente_core::domain::raio::ErroRaio::fmt`), sem distinguir o trait.

Isso viola o **invariante 1 da spec** (`forma-organizada.md`): "cada `path` em
`nodes` é único". O adaptador atual rejeita o JSON com `PathDuplicado`,
conforme o prompt 0003 explicitamente mandava ("não corrigir silenciosamente").

A consequência prática: a lente, hoje, é **inutilizável contra qualquer crate
Rust idiomático** com `impl Display + derive Debug` no mesmo tipo — o que
abrange praticamente todo crate que define enums de erro com `thiserror` ou
padrões similares. O `lente_core` é o primeiro exemplo, e provavelmente não é
exceção.

Esta decisão **transcende um único componente** — define como o projeto trata
ambiguidades da fonte, e introduz um novo crate (e potencialmente um novo
padrão de cascata para outras ambiguidades futuras). Por isso é ADR, não
histórico de prompt.

---

## A natureza do problema

Dois nós com o mesmo `path` podem ser de dois tipos:

- **Colisão verdadeira**: são de fato o mesmo item, alcançável por dois
  caminhos (ex.: reexports). A informação está duplicada, não conflitante.
- **Colisão aparente**: são itens diferentes que o `cargo-modules` agregou no
  mesmo path. O caso do `ErroRaio::fmt`: existem dois métodos distintos, mas o
  fork não preservou a distinção (não inclui o trait no path).

A spec idealiza paths únicos. A fonte real produz paths que parecem únicos mas
às vezes não são. Há duas formas de reconciliar:

1. Ajustar a spec à realidade (relaxar o invariante, identidade composta).
2. Ajustar o adaptador para reconstruir a distinção que a fonte perdeu.

Este ADR escolhe a segunda forma, por uma razão: **manter o L1 (cálculo do
raio e tipos de dados) operando sob o invariante de path único**. Mexer no
invariante 1 propagaria mudança por todo o L1 já escrito; reconstruir a
distinção no L3/adjacente preserva o L1 intacto.

---

## Decisão

### 1. Criar dois crates L1 separados: `lente_investiga` e `lente_resolve`

Dois novos membros do workspace, ambos camada **L1 — pureza absoluta**: zero
I/O, zero dependências externas, só stdlib. Diretórios a definir nos prompts
de cada um.

A separação reflete duas responsabilidades distintas:

- **`lente_investiga`**: dado um par de nós colidentes e a evidência
  disponível, **investiga e classifica** a colisão. Produz um **veredito**:
  "mesmo item" (colisão verdadeira), "distintos com tal evidência" (colisão
  aparente), ou "não consegui determinar" (cascata esgotada). Não nomeia
  identidades, não modifica o grafo. Função puramente analítica.
- **`lente_resolve`**: dado um veredito e o grafo, **aplica** a resolução. Se o
  veredito é "mesmo item", unifica os dois nós em um. Se é "distintos",
  decide como nomear as identidades novas (escolha de convenção, registrada
  abaixo) e atualiza as arestas que apontavam para o path original. Função
  puramente aplicativa — não conhece a evidência por trás do veredito.

A separação tem uma propriedade desejada: investigação e aplicação ficam
desacopladas. O `lente_investiga` é o lugar onde a evidência é interpretada;
o `lente_resolve` é o lugar onde a decisão se materializa nos dados.

### 2. Cascata de estratégias dentro do `lente_investiga`

O `lente_investiga` orquestra **duas estratégias internamente**:

**Estratégia 1 (barata): vizinhança no grafo.** Examina as arestas que entram
e saem de cada nó colidente. Se as vizinhanças forem disjuntas (cada cópia
tem usuários diferentes, ou aparece em contextos diferentes via arestas
`owns`), conclui "distintos com evidência de vizinhança". Se forem
coincidentes (praticamente as mesmas arestas), conclui "mesmo item".

**Estratégia 2 (cara): leitura do código-fonte.** Quando a vizinhança não
basta (vizinhanças ambíguas ou inexistentes), o `lente_investiga` recebe o
conteúdo dos arquivos `.rs` do crate-alvo (já lidos pelo `lente_infra`) e
reconhece **padrões textuais limitados** — blocos `impl <Trait> for <Tipo>` e
os métodos declarados em cada um. Isso permite distinguir `Display::fmt` de
`Debug::fmt` no caso do `ErroRaio`.

**O parser é deliberadamente limitado**: reconhece o padrão canônico de `impl
Trait for Tipo`. Casos avançados (genéricos com `where`, macros que geram
impls, atributos `#[cfg]`) podem não ser cobertos. Quando não cobertos, o
veredito é "não consegui determinar" — explícito e diagnosticável, não
silencioso.

A cascata mora dentro do `lente_investiga` (não fora, não no `lente_infra`)
porque escolher entre estratégias é parte da investigação, não da orquestração.

### 3. Nomeação de identidades novas (responsabilidade do `lente_resolve`)

Quando o veredito é "distintos", o `lente_investiga` reporta a evidência
(ex.: "o primeiro nó tem trait `Display` no impl; o segundo tem `Debug`") mas
não cria os novos paths. O `lente_resolve` decide a convenção de nomeação.

A convenção inicial (registrada aqui, ajustável por revisão deste ADR ou por
decisão de implementação no prompt do `lente_resolve`): incluir o trait
discriminador no path, no formato `Tipo::<Trait>::metodo` — ex.: o `fmt` de
Display vira `ErroRaio::<Display>::fmt`, o de Debug vira
`ErroRaio::<Debug>::fmt`. Isso mantém legibilidade e segue a tradição de
notação `<...>` para discriminação em Rust.

Casos que não se encaixam (evidência de tipo diferente, ex.: discriminação por
módulo de origem) ficam para o prompt do `lente_resolve` resolver com critério
similar — convenção legível, baseada na evidência reportada pelo investiga.

### 4. Divisão de responsabilidades L3 ↔ L1

- **`lente_infra` (L3)**: continua sendo o adaptador. Detecta colisões durante
  a tradução. Para cada colisão, lê (se necessário) o conteúdo dos arquivos
  `.rs` envolvidos do disco. Invoca `lente_investiga` passando o par de nós,
  a vizinhança do grafo, e o conteúdo dos arquivos. Recebe o veredito. Invoca
  `lente_resolve` passando o veredito e o grafo. Recebe o grafo resolvido. I/O
  fica em L3, conforme a gravidade Tekt.
- **`lente_investiga` (L1)**: pura investigação. Recebe estruturas e strings,
  devolve veredito.
- **`lente_resolve` (L1)**: pura aplicação. Recebe veredito e grafo, devolve
  grafo resolvido.
- **Dependências entre crates**: `lente_infra` depende de `lente_investiga`,
  `lente_resolve` e `lente_core`. `lente_investiga` e `lente_resolve` dependem
  de `lente_core`. Nenhum dos dois L1 novos depende do outro — eles se
  comunicam pelos tipos que `lente_core` define (ou que o ADR poderá expandir
  para incluir o tipo `Veredito`, se for o caso de morar no `lente_core`).

### 5. Onde o tipo `Veredito` mora

O `Veredito` é o que o `lente_investiga` produz e o `lente_resolve` consome.
Ele é a interface entre os dois crates L1 novos. Há duas opções para onde
defini-lo:

- **No `lente_core`**: trata o Veredito como parte do vocabulário central do
  projeto. Os dois crates novos importam de lá.
- **Num dos dois crates** (provavelmente `lente_investiga`, que é quem o
  produz): o outro crate importa.

Decisão: **`lente_core`**. O Veredito é tipo de dados estável que outros
componentes futuros podem precisar (ex.: uma camada de relatório no L2 que
queira mostrar os vereditos ao usuário). Coloca-lo no `lente_core` mantém o
vocabulário central e evita acoplamento `lente_resolve` → `lente_investiga`.

Isso significa que `lente_core` recebe uma adição (o tipo `Veredito`) sem
violar sua pureza — `Veredito` é tipo puro, sem I/O, perfeitamente coerente
com L1.

### 6. Quando a cascata falha

Se a Estratégia 2 também não conseguir determinar (padrão exótico, código não
disponível, etc.), o veredito é `NaoDeterminado` com diagnóstico (o que cada
estratégia tentou e por que não conseguiu). O `lente_resolve` que recebe esse
veredito retorna o grafo sem modificar e propaga o diagnóstico. O `lente_infra`
empacota isso em `Err(ErroAdaptador::ColisaoNaoResolvida)` com a explicação
completa.

Não inventa identidade, não consolida silenciosamente. O usuário pode então
ou aceitar que aquele crate não é processável (e usar a lente nos crates que
são), ou contribuir um caso de teste para a estratégia melhorar.

### 7. Escopo declarado: só colisões de path

Os dois crates novos cobrem, por ora, **só colisões de path** (a Descoberta 2
do laudo 0003). Os outros limites da spec (granularidade do `uses` via import,
reexports, raio comportamental, etc.) **não** entram aqui. Cada ambiguidade
futura que justificar resolução automática será decisão própria — ou expande
um destes crates (e este ADR é revisado), ou cria componente paralelo, ou
fica como limite declarado.

---

## Decisão tomada sem medição prévia (registrado para honestidade)

Esta decisão arquitetural foi tomada **antes de medir a abrangência real das
colisões em código Rust**. A alternativa "investigar primeiro" (rodar o
adaptador atual contra muitos crates reais, medir quantos colidem, classificar
os padrões de colisão, e só então decidir a arquitetura) foi considerada e
rejeitada — o autor preferiu construir a arquitetura antes de medir.

A consequência aceita é o risco de a proporção real das colisões não justificar
a complexidade introduzida (novo crate, parser próprio, cascata em duas
estratégias). Cenários possíveis:

- Se a maioria das colisões for resolvida pela Estratégia 1 (vizinhança), a
  arquitetura justifica-se: Estratégia 2 fica como fallback raro.
- Se a maioria exigir a Estratégia 2 (código-fonte), a cascata vira só uma
  otimização menor sobre o caminho principal — defensável mas com menor ganho.
- Se a maioria nem a Estratégia 2 resolver, o `lente_resolve` não cumpre sua
  função e este ADR precisa ser revisado.

Este ADR pode ser superseded por outro se a implementação revelar que a
proporção real fere o desenho.

---

## Prompts Afetados

| Prompt / artefato | Como esta decisão o molda |
|-------------------|---------------------------|
| Prompt do `lente_investiga` (futuro) | Define o crate novo. L1 puro. Recebe par-de-nós + vizinhança + conteúdo de arquivos; devolve Veredito (mesmo item / distintos com evidência / não determinado). Cascata interna em duas estratégias. |
| Prompt do `lente_resolve` (futuro) | Define o crate novo. L1 puro. Recebe Veredito + grafo; aplica a resolução (unifica nós ou cria identidades novas com convenção declarada); devolve grafo resolvido. |
| Adição ao `lente_core` (futura) | Define o tipo `Veredito` (e tipos auxiliares de evidência) no vocabulário central. Sem mudar nada existente do `lente_core`. |
| Prompt de modificação do `lente_infra` (futuro) | Adapta o L3 atual: detecta colisões, lê arquivos `.rs` quando precisa, invoca `lente_investiga`, recebe Veredito, invoca `lente_resolve`, recebe grafo resolvido, ou empacota erro se Veredito for `NaoDeterminado`. |
| Spec `forma-organizada.md` | Não muda os invariantes. Pode receber uma nota apontando para este ADR como mecanismo de resolução. |

---

## Consequências

**Positivas**:
- A lente passa a operar sobre crates Rust idiomáticos que hoje rejeita.
- O invariante 1 da spec permanece — o `lente_core` opera sob a forma idealizada.
- A resolução é baseada em evidência (vizinhança ou código), não em convenção
  arbitrária. Não inventa interpretação.
- A cascata permite que a estratégia barata cubra o caso comum, com a cara só
  quando necessário.
- O `lente_resolve` como L1 puro é testável sem disco e sem fonte externa.

**Negativas**:
- **Mais um crate no workspace** (`lente_resolve`), aumentando a topologia.
- **Parser próprio limitado** dentro do `lente_resolve` para reconhecer
  padrões de `impl` — exige manutenção, e tem casos que não cobre (genéricos
  complexos, macros geradoras de impl, etc.). Cada caso não coberto vira
  fallback de "não consegui resolver".
- **Decisão tomada sem dado** — risco aceito (ver seção acima).
- O `lente_infra` passa a ler arquivos `.rs` além de invocar o fork: mais I/O,
  mais modos de falha (arquivo não encontrado, encoding inválido, etc.).

**Neutras**:
- A lente continua tendo modos de falha — apenas mudou onde eles aparecem.
  Em vez de rejeitar todos os crates com colisão, agora rejeita só os que a
  cascata não resolve.

---

## Alternativas Consideradas

| Alternativa | Por quê foi rejeitada |
|-------------|----------------------|
| Spec relaxa: aceitar paths não-únicos, identidade composta | Propaga mudança por todo o `lente_core` já escrito (cálculo do raio assume path único). Mais invasiva que reconstruir a distinção no L3/adjacente. |
| Pedir ao fork: incluir trait no path para métodos de impl | Reabre o fork, que foi explicitamente fechado pelo autor. Cria nova rodada de manutenção no fork. Pode ainda ser uma evolução futura. |
| Adaptador resolve silenciosamente: numerar duplicatas ou consolidar | Adia o problema e suja os dados. O laudo 0003 e o prompt 0003 são explícitos contra resolução silenciosa. |
| `lente_resolve` como L3 (lê disco diretamente) | Tensão com a gravidade Tekt: L3 importando outro L3. Solução: deixar I/O no `lente_infra`, lógica pura no `lente_resolve` (L1). |
| Investigar primeiro (medir colisões em crates reais antes de fixar arquitetura) | Rejeitada conscientemente pelo autor. Aceito o risco da decisão sem dado (ver seção própria). |
| Aceitar como Limite 6 da spec, sem resolver | Tornaria a lente inutilizável contra crates Rust idiomáticos. Solução conservadora mas que invalida o propósito da ferramenta. |

---

## Referências

- Laudo do prompt 0003 (Descoberta 2) — onde a colisão apareceu pela primeira vez
- ADR-0001 — fonte do grafo (decisão de não modificar o fork de novo, reforçada aqui)
- ADR-0002 — modelagem do grafo (invariante 1 preservado)
- ADR-0003 — workspace Cargo (estrutura onde o novo crate entra)
- `forma-organizada.md` — invariante 1 e a Nota de Evolução sobre subtipos
