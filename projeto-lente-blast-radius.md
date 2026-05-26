# Projeto: Lente de Forma e Consequência

**Estado**: Definição de propósito e objetivos
**Data**: 2026-05-25
**O que este documento é**: a definição do que se busca e por quê.
**O que este documento não é**: arquitetura, escolha de paradigma
vencedor, decisão de bibliotecas, ou plano de implementação. Esses
vêm depois, e só depois de um protótipo concreto validar a ideia.

---

## 1. O Problema

Para melhorar um sistema, é preciso entendê-lo. Entender um
sistema é conseguir segurar a forma do todo na cabeça. Nenhum
humano segura um sistema grande na cabeça lendo código linha por
linha — nem mesmo quem o escreveu.

Esse problema sempre existiu em projetos grandes feitos por muitas
pessoas. O desenvolvimento assistido por IA o intensifica e o
estende a projetos de uma pessoa só: quando a IA escreve o código,
o humano não constrói o modelo mental que antes vinha de graça com
o ato de escrever. Ele passa a "ter" um sistema que não construiu
na própria cabeça — exatamente a mesma condição de quem herda um
projeto alheio.

A consequência é uma cegueira específica: ao olhar para um ponto
do código, o humano entende o que aquele ponto faz localmente, mas
não vê o que mais no sistema é afetado se ele mexer ali. Falta o
mapa de consequência.

---

## 2. A Pergunta Central

A lente existe para responder, em até dez segundos, a uma única
pergunta:

> **"O que quebra se eu mexer aqui?"**

Não "o que este código faz". Não "qual algoritmo ele usa". Não
"onde está a configuração". A pergunta é sobre consequência
estrutural — o raio de impacto de uma mudança. Em uma expressão:
**blast radius**.

A lente transforma um medo invisível ("não sei o que vou quebrar")
numa avaliação visível ("estas partes estão no raio; estas não").

---

## 3. O Que a Lente Mostra (e o que não mostra)

### Mostra — o blast radius estrutural (realizável)

A partir da estrutura de dependências do sistema, a lente computa
e mostra:

1. **Grau de isolamento / hierarquia de risco.** Se um componente
   é base (muitos dependem dele, ele depende de pouco) ou folha
   (depende de muitos, ninguém depende dele). Mexer em base tem
   raio grande; mexer em folha tem raio contido. Isso é cálculo de
   grau no grafo.

2. **Alcance transitivo.** Dado um ponto, o conjunto de tudo que
   depende dele direta e indiretamente (a jusante, vai sentir a
   mudança) e tudo de que ele depende (a montante, pode precisar
   mudar junto), com a profundidade da propagação. Isso é fecho
   transitivo no grafo.

### Não mostra — o blast radius comportamental (o horizonte)

A lente **não** responde "vai realmente quebrar?". Saber se um
componente A quebra quando B muda exige entender o contrato
comportamental entre eles ("A assume que B se comporta assim") —
informação semântica que não se extrai com confiança da estrutura.

Isso é o **horizonte**: a direção que se persegue, nunca se
alcança. A lente mostra o que está no raio (estrutural); o humano
julga se quebra de verdade (semântico). Toda funcionalidade futura
se julga por um critério: aproxima do horizonte de forma honesta,
ou finge ter chegado? A segunda é proibida.

### Limite honesto: tipos de "mexer"

A lente vê bem o raio de mudanças que alteram ou removem a forma
de algo (mudar assinatura, remover módulo): isso quebra
estruturalmente quem depende. A lente vê mal o raio de mudanças
que alteram só o comportamento interno sem mudar a forma (mudar o
corpo de uma função mantendo a assinatura): estruturalmente não
quebra ninguém, mas pode quebrar comportamentalmente. Esse segundo
caso é o horizonte. A UI deve ser honesta sobre essa distinção em
vez de fingir cobrir os dois.

---

## 4. Dois Momentos de Uso (mesmo motor, dois gatilhos)

O cálculo do blast radius é idêntico nos dois momentos. Muda o
gatilho e o foco da apresentação. Construir o primeiro entrega a
maior parte do segundo.

### Momento A — Planejar (humano explora)

O humano abre a lente para entender um sistema e arquitetar a
próxima mudança. Seleciona um ponto, vê o raio, decide se vale e
por onde começar. É a lente de compreensão. É o caso de uso que
originou o projeto.

### Momento B — Revisar a proposta da IA (humano decide)

A IA propôs uma mudança. A lente mostra o raio daquela mudança
específica para o humano aprovar ou rejeitar com consciência da
consequência. É a lente acoplada ao loop de decisão com a IA.

Esta é a divisão de trabalho que motiva o projeto: a IA faz o
trabalho cuja correção é verificável; o humano decide o que não é
verificável, só julgável (trade-offs, escopo, alinhamento com
intenção). A lente é o que dá ao humano a informação para decidir
o que a IA não pode decidir por ele.

---

## 5. A Interface

### Princípio: paradigma conhecido primeiro, refinar a partir do concreto

A ideia (blast radius como centro) é nova. A forma honesta de
explorar algo novo não é inventar uma interface perfeita de
cabeça, mas pegar um paradigma de visualização já conhecido,
colocar a ideia dentro dele, e refinar ou trocar conforme funciona
ou não. Prototipagem sobre o conhecido, não invenção do nada.

### Paradigmas candidatos (objetivo, não escopo do primeiro passo)

A informação do blast radius é uma só; pessoas a absorvem de
formas diferentes. Por isso, no horizonte, a lente oferece mais de
uma projeção da mesma informação, e o usuário usa a que ressoa com
seu modelo mental. Isso é design para preferências distintas, não
indecisão.

Os candidatos identificados, cada um respondendo melhor a uma
inclinação de pensamento:

| Paradigma | Pergunta que atende melhor |
|-----------|----------------------------|
| Lista de consequência (ordenada por risco) | "O que conserto primeiro?" |
| Anéis de propagação (radial concêntrico) | "Quão longe alcança?" |
| Matriz / heatmap de acoplamento (DSM) | "Onde estão os hubs frágeis?" |
| Foco + contexto (lista navegável + subgrafo) | "Mostre o essencial, deixe-me explorar" |

### Escopo do primeiro passo: UM paradigma

O primeiro passo implementa **um** paradigma — aquele que se
apostar entregar os dez segundos mais diretamente. Os outros
ficam como evolução declarada, a adicionar quando o primeiro
funcionar e para servir as preferências distintas.

A escolha de qual paradigma começar é uma decisão a tomar por
prototipagem, não por argumento. A hipótese atual (a testar, não
assumida): para "o que quebra se eu mexer aqui", uma lista
priorizada por risco pode bater os dez segundos mais facilmente
que uma matriz de centenas de nós, porque entrega a resposta
ordenada em vez de exigir leitura da matriz. Mas a matriz (DSM) já
existe de trabalho anterior e pode ser ponto de partida barato. A
primeira coisa a fazer com o computador é comparar os dois num
sistema pequeno conhecido e ver qual ganha os dez segundos.

### Princípios de UI

- **Zero ruído.** Centenas de nós não viram emaranhado. Cor e
  destaque comunicam risco e raio, não estética. Nós fora do raio
  são omitidos ou recolhidos.
- **Revelação progressiva.** Começa simples, revela detalhe sob
  demanda.
- **Honestidade visual.** A UI não promete o raio comportamental
  que não tem. Mostra o estrutural com clareza e indica seu
  limite.
- **Motor acima da interface.** A vista é projeção do mesmo
  modelo computado uma vez. Trocar de paradigma não recalcula o
  grafo.

---

## 6. Relação com o Tekt

O Tekt é a arquitetura cristalina — o conjunto de regras sobre
como um sistema deve ser formado (camadas, topologia, prompts L0,
ADRs, linhagem). O nome vem de *tekton* (construtor, arquiteto).

A lente **faz parte do Tekt** no sentido de que usa a estrutura
dele e materializa a sua promessa (uma ferramenta que mostra
consequência, não código). Mas **não é o Tekt**: é a ferramenta de
visão, não a arquitetura.

A relação é de acoplamento solto:

- **Independência.** A lente funciona em qualquer projeto Rust —
  legado, vanilla, código gerado por IA, ou Tekt. Não exige que o
  projeto siga a arquitetura cristalina. Blast radius estrutural só
  precisa de dependências, que todo código tem.
- **Enriquecimento opcional.** Quando aplicada a um projeto Tekt
  (lê `crystalline.toml`, camadas, prompts), a lente mostra
  informação a mais — fronteiras de camada, violações — como
  bônus. Nunca como requisito.

A comparação entre dois sistemas (ex: Typst vanilla vs Typst
cristalino, "o que falta migrar") não é uma ferramenta separada: é
aplicar a lente a cada um e olhar a diferença. Se a lente é boa, a
comparação é um adereço que cai de graça, não um produto.

---

## 7. Restrição de Construção: Motor Emprestado

A análise de dependências (travessia de módulos, extração de
imports, construção do grafo) **não será reimplementada**. Existem
ferramentas no ecossistema Rust que já fazem isso
(`cargo-metadata`, `cargo-modules`, `cargo-deps` e afins). A lente
consome o que essas ferramentas produzem.

Esta restrição é uma lição direta do trabalho anterior, onde o
motor foi reimplementado do zero contra a instrução original de
"usar o que existe, expandindo". O valor do projeto está na lente
(a visão da consequência), não no motor (a análise). O motor é
meio; a lente é fim.

A escolha técnica de qual ferramenta usar como backend, e como, é
uma decisão posterior — depende de verificar qual delas expõe a
informação na granularidade necessária (módulo, não só crate) e em
formato consumível. Isso se confirma na fase de arquitetura, não
agora.

---

## 8. Critério de Sucesso

> Se a tela for confusa, não há blast radius que salve.

O projeto se prova ou falha na primeira impressão. Ao selecionar
um ponto do sistema, o humano deve entender o raio de impacto em
até dez segundos — sem ler documentação, sem arrastar a tela por
minutos, sem decorar legendas. Se isso não acontecer, a interface
falhou, independentemente da qualidade do motor.

Este é o teste que governa todas as decisões de design: cada
escolha de interface se julga por quanto aproxima ou afasta dos
dez segundos de compreensão.

---

## 9. O Que Vem Depois Deste Documento

Em ordem, e sem pular etapas:

1. **Decidir o paradigma inicial por prototipagem.** Pegar um
   sistema pequeno e conhecido (ex: o crystalline-dsm, ~4 crates,
   ~40 módulos), e comparar à mão / em protótipo cru qual
   paradigma (lista por risco vs matriz com raio destacado)
   entrega os dez segundos. O vencedor é por onde se começa.

2. **Confirmar o motor emprestado.** Verificar qual ferramenta
   existente expõe o grafo de dependências a nível de módulo, em
   formato consumível. Isso define o backend.

3. **Só então**, arquitetura e construção — sob a disciplina Tekt
   se o projeto adotá-la, com a lente como centro e o motor como
   dependência externa.

Nada nas etapas 1 e 2 exige escrever código de produção. A etapa 1
é exploração; a etapa 2 é verificação. A construção começa depois
das duas, não antes.

---

## 10. Disciplina de Processo (lição incorporada)

Este projeto nasce de um reset motivado por um erro: pular para a
construção antes de o propósito estar claro, e não questionar
escopo. A disciplina que evita repetir:

- Nada se constrói até a ideia estar clara o suficiente para ser
  explicada a outra pessoa em prosa.
- Multiplicidade de opções (os quatro paradigmas) é horizonte
  declarado, não escopo do primeiro passo. Um de cada vez.
- O motor não se reimplementa. Usa-se o que existe.
- A honestidade sobre limites (o que a lente não mostra) é parte
  do design, não nota de rodapé.

Esta disciplina, na parte que trata de escrever prompts e tomar
decisões com auxílio de IA, é candidata a ser incorporada ao
próprio Tekt.
