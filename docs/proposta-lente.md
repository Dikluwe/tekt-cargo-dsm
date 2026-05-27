# Lente de Forma e Consequência

**Estado**: Definição de propósito e objetivos
**O que este documento é**: a definição do que se busca e por quê.
**O que este documento não é**: arquitetura, escolha de tecnologia,
ou plano de implementação.

**A ordem de trabalho deste projeto**: dados primeiro, visualização
por último. Olha-se o que as ferramentas conseguem extrair de um
sistema; traduz-se esses dados para uma forma organizada; e só com os
dados organizados na mão pensa-se em como mostrá-los. A visualização
é construída sobre o que comprovadamente existe, nunca imaginada
antes de se saber o que é obtível.

---

## 1. O Problema

Para melhorar um sistema, é preciso entendê-lo. Entender um sistema
é conseguir segurar a forma do todo na cabeça. Nenhum humano segura
um sistema grande na cabeça lendo código linha por linha — nem
mesmo quem o escreveu.

Esse problema se agrava no desenvolvimento assistido por IA. Quando
um humano escreve o código, ele constrói o modelo mental do sistema
no processo: a compreensão vem junto com o ato de escrever. Quando a
IA escreve o código, essa compreensão não se forma. O humano passa a
ser responsável por um sistema que não construiu na própria cabeça —
a mesma situação de quem herda um projeto feito por outra pessoa.

A consequência é uma cegueira específica. Ao olhar para um ponto do
código, o humano entende o que aquele ponto faz localmente. Mas não
vê o que mais no sistema é afetado se ele mexer ali. Falta o mapa de
consequência.

---

## 2. A Pergunta Central

A lente existe para responder, em até dez segundos, a uma única
pergunta:

> **"O que quebra se eu mexer aqui?"**

Não "o que este código faz". Não "qual algoritmo ele usa". Não "onde
está a configuração". A pergunta é sobre consequência estrutural — o
raio de impacto de uma mudança.

A lente transforma um medo invisível ("não sei o que vou quebrar")
numa avaliação visível ("estas partes estão no raio; estas não").

---

## 3. O Que a Lente Mostra

### Mostra — o raio de impacto estrutural

A partir das dependências do sistema, a lente computa e mostra:

1. **Hierarquia de risco.** Se um componente é base (muitos
   dependem dele, ele depende de pouco) ou folha (depende de muitos,
   ninguém depende dele). Mexer num componente base tem raio grande:
   muita coisa pode sentir. Mexer numa folha tem raio contido: o
   efeito fica isolado.

2. **Alcance da propagação.** Dado um ponto, o conjunto de tudo que
   depende dele direta e indiretamente (o que vai sentir a mudança)
   e tudo de que ele depende (o que pode precisar mudar junto), com a
   profundidade de quão longe o efeito se propaga.

### Não mostra — e é honesta sobre isso

A lente **não** responde "vai realmente quebrar?". Saber se um
componente A deixa de funcionar quando B muda exige entender o que A
espera do comportamento de B — informação que não se extrai com
confiança apenas das dependências.

Essa é a fronteira da ferramenta. A lente mostra o que está no raio
de impacto; o humano julga se quebra de verdade. A lente aponta onde
olhar; não substitui o julgamento de quem olha.

Esse limite define um critério permanente de design: toda
funcionalidade futura deve aproximar-se de responder melhor "o que é
afetado" de forma honesta, nunca fingir responder "vai quebrar"
quando não pode.

### Um detalhe sobre o que "mexer" significa

Há tipos diferentes de mudança, e a lente os enxerga de forma
diferente:

- Mudar a forma de algo — assinatura de uma função, remover um
  módulo — quebra estruturalmente quem depende. A lente vê bem esse
  raio.
- Mudar só o comportamento interno sem mudar a forma — reescrever o
  corpo de uma função mantendo a assinatura — não quebra ninguém
  estruturalmente, mas pode quebrar quem dependia do comportamento
  antigo. A lente vê mal esse raio.

A interface deve ser honesta sobre essa diferença, em vez de
aparentar cobrir os dois casos.

---

## 4. Dois Momentos de Uso

O cálculo do raio de impacto é o mesmo nos dois momentos. Muda o que
dispara o cálculo e o foco da apresentação.

### Momento A — Planejar

O humano abre a lente para entender um sistema e decidir a próxima
mudança. Seleciona um ponto, vê o raio, decide se vale a pena mexer e
por onde começar. É a lente de compreensão.

### Momento B — Decidir sobre uma proposta da IA

A IA propõe uma mudança. A lente mostra o raio de impacto daquela
mudança específica, para o humano aprovar ou rejeitar sabendo a
consequência. É a lente acoplada à decisão.

Esta é a divisão de trabalho que motiva o projeto: a IA faz o
trabalho cuja correção uma máquina consegue verificar; o humano
decide o que uma máquina não consegue — se a mudança vale, se está no
escopo, se faz sentido para o todo. A lente dá ao humano a
informação que ele precisa para decidir o que só ele pode decidir.

---

## 5. A Interface (vem depois dos dados)

A visualização é o último passo do trabalho, não o primeiro. Ela só
é pensada depois de se saber quais dados as ferramentas conseguem
extrair e de tê-los traduzidos para uma forma organizada (ver seções
6 e 7). Pensar na tela antes disso arriscaria imaginar algo que
depende de dados que não existem ou não são extraíveis. Com os dados
organizados na mão, a visualização é livre: trabalha sobre o que
comprovadamente se tem.

### Explorar a partir de um paradigma conhecido

Quando chegar a hora de mostrar, a forma de explorar uma ideia de
visualização nova não é inventar uma interface perfeita de cabeça,
mas pegar uma forma de visualização que já se sabe que funciona,
colocar os dados dentro dela, e refinar — ou trocar — com base no que
se mostra eficaz.

### A informação é uma; as projeções podem ser várias

O raio de impacto é uma informação só. Mas pessoas pensam de formas
diferentes, e a mesma informação comunica melhor para cada uma em
formatos diferentes. No horizonte, a lente oferece mais de uma
projeção da mesma informação, e cada um usa a que combina com seu
jeito de pensar.

Projeções candidatas, cada uma forte para uma inclinação:

| Projeção | Pergunta que atende melhor |
|----------|----------------------------|
| Lista ordenada por risco | "O que eu conserto primeiro?" |
| Anéis de propagação (do centro para fora) | "Quão longe isso alcança?" |
| Matriz de dependências | "Onde estão os pontos frágeis do todo?" |
| Foco mais contexto (lista + detalhe navegável) | "Mostre o essencial, deixe-me explorar" |

### O primeiro passo de visualização é uma projeção só

Quando a visualização começar, implementa-se primeiro **uma**
projeção — a que mais provavelmente entrega os dez segundos sobre os
dados que se tem. As outras ficam como evolução, a adicionar quando a
primeira funcionar e para servir os jeitos diferentes de pensar.

Qual começar é uma decisão a tomar testando, não argumentando, e
sobre os dados reais já organizados — não sobre dados imaginados.

### Princípios da interface

- **Dez segundos.** A medida de tudo. Se exige ler manual, arrastar a
  tela por minutos, ou decorar legenda, falhou.
- **Sem ruído.** Centenas de elementos não viram emaranhado. Cor e
  destaque comunicam risco e raio, não enfeite. O que está fora do
  raio se apaga ou recolhe.
- **Revelar aos poucos.** Começa simples, mostra detalhe sob demanda.
- **Honestidade visual.** A interface não aparenta mostrar o que não
  mostra (o raio comportamental). Deixa claro o que é e o que não é.

---

## 6. Em Que Sistemas Funciona

A lente funciona em qualquer sistema que tenha dependências — ou
seja, qualquer código. Não exige que o projeto siga nenhuma
arquitetura ou convenção particular. O raio de impacto estrutural
deriva das dependências, que todo sistema tem.

Quando o sistema analisado oferece informação extra — uma definição
de camadas, regras de arquitetura declaradas, metadados de
organização — a lente pode mostrar mais (fronteiras, violações
dessas regras). Isso é um ganho quando disponível, nunca um
requisito.

Comparar dois sistemas (por exemplo, duas versões de um projeto, para
ver o que mudou ou o que falta) não é uma função separada: é aplicar
a lente a cada um e olhar a diferença. Se a lente é boa, a comparação
sai de graça.

---

## 7. Como o Sistema é Analisado (o ponto de partida)

Este é o primeiro passo concreto do projeto. Antes de pensar em
qualquer tela, olha-se o que as ferramentas existentes conseguem
extrair de um sistema.

A análise das dependências — descobrir o que depende de quê — não é o
foco do projeto e não se constrói do zero. Existem ferramentas que já
extraem essa informação de um código. A lente parte do que essas
ferramentas conseguem entregar.

A ordem é:

1. **Ver os dados que a ferramenta entrega.** Olhar uma ferramenta
   existente e estabelecer, concretamente, que informação ela
   consegue extrair de um sistema, em que granularidade, e em que
   formato.

2. **Traduzir os dados para uma forma organizada.** A partir do que a
   ferramenta entrega, definir a estrutura de dados que representa o
   sistema de forma utilizável pela lente — independente de como a
   ferramenta os produziu.

3. **Mostrar os dados organizados.** Só então (seção 5) pensar em
   como visualizá-los para responder à pergunta dos dez segundos.

A escolha da ferramenta e a forma de consumir sua saída são as
primeiras decisões de arquitetura do projeto, tomadas olhando o que
existe — não derivadas de uma visualização imaginada de antemão.

---

## 8. Critério de Sucesso

> Se a tela for confusa, não há cálculo de raio que salve.

O projeto se prova ou falha na primeira impressão. Ao selecionar um
ponto do sistema, o humano deve entender o raio de impacto em até dez
segundos — sem ler documentação, sem arrastar a tela por minutos, sem
decorar legendas. Se isso não acontecer, a interface falhou,
independentemente de quão bom seja o cálculo por trás.

Esse teste governa todas as decisões de design: cada escolha se julga
por quanto aproxima ou afasta dos dez segundos de compreensão.

---

## 9. Próximos Passos

Em ordem, sem pular etapas. Dados primeiro, visualização por último:

1. **Ver o que a ferramenta entrega.** Olhar uma ferramenta de
   análise existente e estabelecer concretamente que dados ela extrai
   de um sistema, em que granularidade e em que formato. Esta é a
   primeira decisão de arquitetura, e ela nasce do que existe.

2. **Traduzir os dados.** Definir a forma organizada — a estrutura
   que representa o sistema de modo utilizável — a partir do que a
   ferramenta entrega.

3. **Descobrir como mostrar, testando.** Com os dados organizados na
   mão, pegar um sistema pequeno e conhecido, selecionar um ponto, e
   descobrir em qual projeção o raio de impacto se entende mais
   rápido. A vencedora é a primeira projeção a construir.

4. **Construir.** Arquitetura e implementação, com a lente como
   centro e a análise como insumo externo já estabelecido nos passos
   anteriores.

O passo 1 é verificação do que existe; o passo 2 é tradução; o passo
3 é exploração sobre dados reais. A construção plena começa no passo
4, depois que se sabe o que se tem e como mostrá-lo.

---

## 10. O Princípio que Orienta o Trabalho

Dados primeiro, visualização por último. Olha-se o que existe para
extrair, traduz-se o que se consegue, e só então pensa-se em mostrar.
Nunca o contrário: imaginar a tela e depois caçar quem tem os dados
arrisca depender do que não existe.

Nada se constrói até a ideia estar clara o suficiente para ser
explicada a outra pessoa em palavras simples. Multiplicidade de
opções é horizonte declarado, não trabalho do primeiro passo: uma
coisa de cada vez. A honestidade sobre os limites da ferramenta é
parte do design, não nota de rodapé. O esforço se concentra no que é
o valor — a lente — e não no que já existe pronto.
