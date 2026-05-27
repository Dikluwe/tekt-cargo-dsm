# Manifesto da Arquitetura Cristalina

**Contexto**: Desenvolvimento Sustentável com Agentes de IA

---

## O Problema Observado

Ao empregar modelos de linguagem para desenvolver e refatorar sistemas reais, observa-se um padrão recorrente e reproduzível: o código gerado tende a preservar a funcionalidade local enquanto degrada progressivamente a estrutura global.

Essa degradação não se manifesta como erro imediato. Ela se manifesta como perda de definição: acoplamento implícito que cresce silenciosamente, fronteiras que se dissolvem, dependências que se multiplicam sem justificativa. O sistema continua a funcionar, mas torna-se progressivamente mais difícil de entender, modificar e raciocinar sobre.

Esse comportamento não é um defeito acidental dos modelos de linguagem. É uma consequência direta de como eles operam: gerando a próxima peça mais plausível dado o contexto imediato, sem nenhum mecanismo interno que preserve a coerência global do sistema ao longo do tempo.

O problema central não é a qualidade do código gerado numa única sessão. É o que acontece ao sistema depois de centenas de sessões, cada uma localmente razoável, cada uma erodindo um pouco mais a estrutura.

Chamamos esse fenômeno de **crescimento amorfo**: expansão funcional sem preservação estrutural.

A Arquitetura Cristalina parte da hipótese de que crescimento amorfo não é inevitável. Ele é o resultado previsível de operar agentes estatísticos sem restrições estruturais explícitas e sem registro do contexto que originou cada decisão. A solução não é melhorar o agente — é redefinir o espaço no qual ele opera e tornar sua origem rastreável.

---

## A Hipótese Central

Um modelo de linguagem gera código a partir de contexto. A qualidade estrutural do código gerado é, em grande parte, função da qualidade e da organização do contexto fornecido.

Contexto ad-hoc — fragmentos colados numa janela de chat, instruções verbais imprecisas, documentação desatualizada separada do código — produz crescimento ad-hoc. Cada geração é plausível localmente. Nenhuma delas é orientada por uma estrutura global. E quando é necessário modificar o código semanas depois, o contexto original foi perdido.

A hipótese central desta arquitetura é:

> **Se o contexto que o agente recebe é estruturalmente controlado e versionado dentro do próprio projeto, o crescimento que ele produz é estruturalmente orientado e auditável.**

O mecanismo concreto é o prompt estruturado em `00_nucleo`. Não como documentação que descreve o código — mas como a **origem causal** do código. O prompt é o artefato de primeira classe. O código é sua materialização.

*Esta hipótese ainda não foi verificada empiricamente de forma sistemática. Este manifesto é a proposição. Os experimentos virão depois.*

---

## A Metáfora Estrutural

A cristalografia oferece uma metáfora precisa para o problema e para a solução proposta.

Numa solução supersaturada, moléculas em excesso buscam um estado de menor energia. Sem um ponto de referência, elas se organizam aleatoriamente, produzindo um sólido amorfo: funcional como barreira física, mas sem estrutura interna previsível, sem planos de clivagem claros, sem propriedades definidas.

Introduza um cristal semente (*seed crystal*) — um fragmento mínimo com estrutura cristalina definida — e o comportamento muda completamente. As moléculas não se organizam mais aleatoriamente. Elas aderem à geometria da semente. O crescimento que se segue é orientado, reproduzível e estruturalmente coerente com o ponto de origem.

**O seed crystal não contém o cristal final. Ele determina a forma que o cristal pode tomar.**

Esta é a função do prompt estruturado em `00_nucleo`: não conter o sistema, mas determinar a geometria segundo a qual o sistema pode crescer. Um sistema desenvolvido sem semente — com agentes gerando código diretamente a partir de instruções verbais descartadas após cada sessão — é um sistema amorfo. Pode solidificar em algo funcional. Mas não terá estrutura interna previsível, e não resistirá à pressão de evolução prolongada.

---

## O Prompt como Contrato

Em arquiteturas tradicionais, três artefatos são mantidos separados: a especificação (o que deve ser feito), o contrato de interface (como os componentes se comunicam) e o código (a implementação).

Na Arquitetura Cristalina, esses três colapsam em dois: o prompt e o código.

O prompt em `00_nucleo` contém o contexto, as restrições estruturais e a instrução de geração. O código gerado a partir dele — incluindo as interfaces que expõe — **é** o contrato. Não existe documento separado descrevendo o contrato: o código gerado pelo prompt é a realização direta da intenção registrada nele.

A arquitetura **busca** isomorfismo entre prompt e código — que a forma do código reflita fielmente a intenção do prompt. Mas não pode garantir esse isomorfismo: modelos de linguagem são agentes probabilísticos, e duas execuções do mesmo prompt produzem resultados estruturalmente equivalentes mas não idênticos. A verificação de correspondência entre prompt e código depende de julgamento humano, não de análise mecânica.

O que pode ser verificado mecanicamente é a estrutura: se `@prompt` existe, se imports respeitam as regras da camada, se L₁ está livre de I/O. A fidelidade de conteúdo — se o código faz o que o prompt pretendia — é responsabilidade do desenvolvedor que escreve o prompt e revisa o output.

Quando um componente precisa mudar, o prompt muda junto. A revisão é registrada no histórico. O código permanece rastreável à sua origem em qualquer ponto do tempo.

---

## Um Novo Paradigma de Verificação

O desenvolvimento tradicional separou sempre dois momentos: escrever o código e verificar que ele funciona. TDD aproximou esses momentos ao exigir que o teste fosse escrito antes da implementação — mas para humanos esse processo tem custo cognitivo real. Manter a especificação do teste e a implementação na cabeça simultaneamente é difícil. Por isso TDD exige disciplina e treinamento, e na prática é frequentemente abandonado sob pressão.

Com agentes de IA, esse custo não existe.

Gerar código e testes simultaneamente a partir do mesmo prompt é tão fácil quanto gerar só o código. O agente não tem o problema cognitivo que torna TDD difícil para humanos. Isso abre um paradigma que não é TDD, não é code-first — é um terceiro modo:

> **O prompt é a especificação. O código e os testes são materializações simultâneas dessa especificação.**

O prompt em L₀ descreve o comportamento esperado nos critérios de verificação. O agente gera a implementação e os testes num único ciclo. A intenção do TDD — especificar antes de implementar — é preservada. O mecanismo que tornava isso difícil para humanos é eliminado.

Uma nucleação está incompleta se não produziu testes junto com o código. O linter verifica a presença de arquivo de teste correspondente para cada componente gerado — não o conteúdo dos testes, mas sua existência como evidência de que o ciclo foi completado.

---

## Os Princípios

### I — Nucleação

A ordem estrutural não emerge espontaneamente.

Todo sistema válido deve possuir um ponto de nucleação explícito: um conjunto de prompts estruturados que estabelecem a origem causal de cada componente.

Qualquer componente cuja existência não possa ser rastreada até um prompt em `00_nucleo` é estruturalmente ilegítimo, ainda que funcionalmente correto. Sem nucleação, o crescimento é amorfo e irrastreável.

### II — Contenção

Fronteiras físicas são restrições estruturais, não organização cosmética.

A estrutura de diretórios define os limites dentro dos quais o crescimento pode ocorrer. Uma dependência que atravessa uma fronteira não autorizada é uma violação estrutural, independentemente de sua correção funcional.

### III — Gravidade

As dependências têm uma direção natural: do mais variável para o mais estável.

Componentes de alto nível dependem de componentes de baixo nível. A inversão dessa direção é uma fratura estrutural. Ciclos de dependência são degenerações que comprometem a auditabilidade e a modificabilidade do sistema.

### IV — Isolamento de Fases

Nem todo código pertence ao mesmo regime estrutural.

Código experimental deve existir em estratos isolados. Para que um componente cruze essa fronteira, ele deve ser normalizado: reescrito de forma a satisfazer os invariantes do regime estável, com prompt correspondente criado em `00_nucleo`.

### V — Primazia dos Invariantes

Invariantes arquiteturais têm precedência sobre conveniência local.

Uma modificação que preserva funcionalidade mas viola invariantes é uma regressão estrutural. A estabilidade do sistema é determinada pela preservação contínua de sua estrutura, não pela ausência momentânea de erros observáveis.

---

## A Estrutura Canônica

```
        L₄ (Fiação)
       /  \
      /    \
    L₂     L₃
 (Casca)  (Infra)
      \    /
       \  /
        L₁ (Núcleo)
         |
        L₀ (Semente)
```

A posição de um componente no lattice determina o que ele pode conhecer, de quem pode depender e como pode evoluir.

---

### L₀ — Semente

Contém os prompts estruturados que originaram cada componente do sistema, e os ADRs que documentam decisões arquiteturais globais.

**Nenhum código executável é permitido neste estrato.**

O prompt é o artefato de primeira classe. O código e os testes gerados a partir dele são sua materialização. Um componente sem prompt correspondente em L₀ é irrastreável — não é possível reproduzir, auditar ou evoluir com consistência o que não tem origem registrada.

---

### L₁ — Núcleo

Contém apenas lógica determinística essencial: entidades de domínio, regras fundamentais, algoritmos puros.

Restrições absolutas: nenhuma dependência externa, nenhuma operação de I/O, nenhum acesso a estado mutável fora do próprio escopo.

O Núcleo é a fase estrutural mais estável do sistema. A lógica aqui existe independentemente do tempo, do estado externo e das tecnologias de infraestrutura. É também o estrato de maior testabilidade: funções puras não precisam de mocks, não precisam de banco em memória, não precisam de setup complexo.

---

### L₂ — Casca

Realiza a tradução entre contextos externos e o Núcleo: validação de entrada, orquestração, adaptação de formatos.

Dependências permitidas: L₂ → L₁, L₂ → L₀. Acoplamento direto com L₃ é proibido.

A Casca é o plano de clivagem primário: separa estabilidade conceitual de variabilidade contextual.

---

### L₃ — Infraestrutura

Implementa detalhes físicos e tecnológicos: persistência, redes, sistemas de arquivos, frameworks externos.

É o estrato de maior variabilidade permitida, contida pelas interfaces definidas em L₁ e originadas pelos prompts de L₀.

Dependências permitidas: L₃ → L₁, L₃ → L₀. Nenhum outro estrato pode depender de L₃.

---

### L₄ — Fiação

O sistema totalmente materializado. Instancia componentes, injeta dependências, configura a execução.

É o único ponto onde as definições de L₀ encontram suas implementações concretas de L₃. Absorve complexidade — não a redistribui para os estratos inferiores. Lógica de negócio encontrada aqui é um defeito estrutural.

---

### L_lab — Arena

Estrato isolado para experimentos e protótipos. Código aqui não possui linhagem obrigatória e não pode ser referenciado pelo sistema principal.

Migração da Arena para o sistema exige reescrita completa com prompt correspondente criado em L₀ e testes gerados junto com o código.

---

## Agentes de IA na Arquitetura Cristalina

Agentes de IA são tratados como **agentes de crescimento** operando sob restrições físicas explícitas. Sua função é explorar o espaço de soluções permitido pelo lattice — não expandi-lo arbitrariamente.

O desenvolvedor não modifica o código diretamente. Ele modifica o prompt em L₀ e o agente reconstrói o código a partir dele. L₀ é a interface de controle do sistema — os estratos abaixo são output, não workspace.

O protocolo de operação é:

1. Verificar se existe prompt em `00_nucleo/prompts/` para o componente
2. Se não existe — parar e solicitar ao desenvolvedor
3. Se existe — ler o prompt completo, incluindo restrições, critérios de verificação e histórico
4. Gerar código **e testes** dentro das restrições declaradas
5. Registrar a revisão no histórico do prompt

O alinhamento do agente não ocorre por instrução verbal nem por filtros externos. Ocorre porque o contexto que ele recebe já está estruturalmente delimitado — o prompt define o que pode ser gerado, e a posição no lattice define o que pode ser importado.

---

## Evolução do Sistema

A maior parte da evolução ocorre por **epitaxia**: novos componentes aderem à estrutura existente com novos prompts em L₀ e código correspondente nos estratos apropriados.

Mudanças mais profundas ocorrem por **nucleação**: novos conceitos fundamentais exigem novos prompts que redefinem o espaço de soluções possíveis. Este processo é deliberado — uma nova nucleação pode exigir revisão de prompts existentes.

Transformações internas sem alteração da estrutura fundamental são **metamorfismo**: o prompt é revisado, o código é regenerado, o histórico registra a pressão que forçou a mudança.

Quando fraturas ocorrem, a arquitetura fornece **planos de clivagem** claros: a estrutura de estratos define superfícies ao longo das quais o sistema pode ser reorganizado localmente sem colapso global.

---

## Limitações Declaradas

Esta arquitetura não resolve o problema do prompt incorreto. Se L₀ contém um prompt mal formulado, o crescimento que dele deriva será estruturalmente coerente e funcionalmente incorreto. A semente determina a forma — não a verdade.

A verificação de correspondência entre prompt e código não é mecanizável: modelos de linguagem são probabilísticos, e duas execuções do mesmo prompt produzem resultados equivalentes mas não idênticos. O linter verifica estrutura — existência de `@prompt`, regras de importação, pureza de L₁, presença de testes. A fidelidade de conteúdo depende de revisão humana.

A premissa de que o desenvolvedor não toca o código diretamente é a condição mais frágil da arquitetura. Sob pressão, a tendência é modificar o código diretamente e "atualizar o prompt depois". Se isso ocorre sistematicamente, L₀ diverge do sistema real e perde sua função de origem causal. A arquitetura é tão forte quanto a disciplina de manter L₀ como único ponto de entrada.

---

## O Que Esta Arquitetura Não É

Esta arquitetura não é uma variação de Clean Architecture, Hexagonal Architecture ou DDD. Essas foram concebidas para o ciclo humano → código.

A Arquitetura Cristalina é concebida para o ciclo **humano → prompt → agente → código + testes**. A distinção fundamental está em L₀: não como camada de documentação, mas como registro causal versionado que torna o contexto da geração parte permanente do projeto.

---

## Estado Atual

Esta é uma proposição, não uma prática validada.

As observações que a motivam são reais e reproduzíveis. A hipótese central — que prompts estruturados e versionados em L₀ reduzem crescimento amorfo em sistemas desenvolvidos com agentes de IA — é plausível, mas ainda não foi testada sistematicamente.

O experimento que vai provar ou refutar a hipótese é comparar a velocidade e consistência de modificação de um sistema desenvolvido com esta arquitetura versus um desenvolvido sem ela, depois de meses de evolução. É nesse ponto que a degradação estrutural se torna visível.

O manifesto é a proposição. Os experimentos dirão se ela se sustenta.

---

## Mapeamento com Padrões da Indústria

| Estrato Cristalino | Clean Architecture | Hexagonal | DDD |
|---|---|---|---|
| L₀ (Semente) | — | — | Linguagem Ubíqua |
| L₁ (Núcleo) | Entidades | Core da Aplicação | Camada de Domínio |
| L₂ (Casca) | Adaptadores de Interface | Adaptadores Primários | Camada de Aplicação |
| L₃ (Infra) | Frameworks & Drivers | Adaptadores Secundários | Infraestrutura |
| L₄ (Fiação) | Main | — | Composition Root |
| L_lab (Arena) | — | — | Spikes / POCs |

---

## Referências

- **Clean Architecture** — Robert C. Martin, 2012
- **Hexagonal Architecture** — Alistair Cockburn, 2005
- **Domain-Driven Design** — Eric Evans, 2003
- **Functional Core, Imperative Shell** — Gary Bernhardt
- **Ports and Adapters Pattern**
- **Onion Architecture** — Jeffrey Palermo
