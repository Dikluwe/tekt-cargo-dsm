# ADR-0001: Fonte do grafo de dependências — fork mantido do cargo-modules como dependência externa

**Status**: `PROPOSTO`
**Data**: 2026-05-26

---

## Contexto

A Lente de Forma e Consequência calcula o raio de impacto estrutural a
partir de um grafo de dependências. A proposta (seção 7) determina que a
extração desse grafo não é o foco do projeto e deve ser consumida de uma
ferramenta existente, não construída do zero.

O alvo inicial de análise é código Rust. A verificação das ferramentas
disponíveis (cargo-deps, cargo-depgraph, cargo-modules) levou às seguintes
constatações, registradas para que a decisão seja auditável:

- `cargo-deps` está declarado como não mantido pelo próprio autor. Descartado.
- `cargo-depgraph` opera no nível de dependência entre crates (pacotes), não
  no nível de estrutura interna (módulos, funções, tipos). A proposta
  (seções 2 e 3) trata de "o que quebra se eu mexer aqui", onde "aqui" é um
  item de código interno. Granularidade incorreta para o propósito. Descartado.
- `cargo-modules` (v0.26.0, MPL-2.0) analisa a estrutura interna do crate:
  módulos, funções, tipos, traits. Granularidade correta. Selecionado.

O `cargo-modules` não oferece saída em formato estruturado (JSON). A inspeção
do código-fonte confirmou que a única saída do comando `dependencies` é texto
DOT (graphviz), destinado a renderização visual, não a consumo por programa.
O grafo interno é um `petgraph::StableGraph<Item, Relationship>`; o DOT é uma
serialização final desse grafo, feita pelo módulo `printer.rs`.

Para obter o grafo num formato consumível com robustez, é necessário um
printer adicional que serialize o grafo interno em JSON. Esse printer precisa
executar dentro do crate, porque cada nó (`Item`) carrega apenas uma
referência ao banco semântico do rust-analyzer (`hir::ModuleDef`); os dados
concretos (nome, caminho, visibilidade, tipo) só são resolvidos chamando
métodos que exigem acesso ao `HirDatabase`, disponível somente durante a
execução da ferramenta. Um consumidor externo não tem como resolver esses
dados. Logo, a serialização tem de acontecer dentro de um fork do
`cargo-modules`.

Surge então a pergunta estrutural: onde esse fork vive em relação ao
projeto-lente, e sob que regime do Tekt.

O fork não se encaixa em nenhuma categoria existente do Tekt v1.3:

- Não é componente nucleado (L1–L4): não é código do projeto-lente, tem
  licença própria (MPL-2.0), build próprio, e arrasta o rust-analyzer.
- Não é código de Arena (`lab/`): a Arena é para código volátil, descartável,
  em disputa. O fork é código estável, mantido, que será usado em produção
  pela lente indefinidamente. Colocá-lo na Arena declararia falsamente que é
  descartável.

Esta é uma lacuna do Tekt v1.3, observada durante este projeto: existe uma
terceira categoria de código — externo ao lattice, de autoria própria,
estável e mantido — que o framework ainda não nomeia.

---

## Decisão

O fork do `cargo-modules` é tratado como **dependência externa mantida**, e
vive em **repositório git separado**, fora do repositório do projeto-lente.

O projeto-lente não contém o fork e não o versiona. O projeto-lente apenas o
**invoca**: a camada L3 executa o binário do fork e consome o JSON que ele
produz. A relação é idêntica à de qualquer ferramenta de terceiros (como o
próprio `cargo` ou o `git`): externa ao lattice, declarada como pré-requisito.

Consequentemente:

1. **Nenhuma camada nova é criada no lattice.** O lattice canônico (L0–L4 mais
   Arena) permanece inalterado. O "externo" já está contido conceitualmente em
   L3, que é a camada de fronteira com o mundo externo (I/O, ferramentas,
   frameworks).

2. **O trabalho no fork acontece no repositório do fork**, não aqui. Inclui
   escrever o printer JSON (`printer_json.rs`), compilar e validar a saída na
   máquina de desenvolvimento (que possui a toolchain Rust e o rust-analyzer).
   Concluído e validado o printer, retorna-se ao projeto-lente para nuclear os
   componentes que consomem a saída.

3. **A linguagem do projeto-lente é Rust**, a mesma toolchain do fork. Apesar
   da simetria, a interface entre lente e fork é por invocação de binário e
   leitura de JSON (não por uso como biblioteca), para manter o desacoplamento:
   o L3 não importa o fork; o L3 executa o fork.

4. **A lacuna do Tekt fica registrada, não resolvida.** A criação de uma
   categoria formal para "dependência externa mantida" no Tekt v1.4 é deixada
   para decisão posterior do autor do framework, e não bloqueia este projeto.
   Até lá, o enquadramento operacional é "repositório externo, invocado por L3".

---

## Prompts Afetados

Este ADR precede a criação dos prompts de componente. A ordem de trabalho
segue a proposta da lente (seções 7 e 9): **dados primeiro, visualização por
último**. Após esta decisão de fonte (passo 1 da proposta — "ver o que a
ferramenta entrega"), o próximo artefato não é o cálculo do raio (L1), mas a
**forma organizada** (passo 2 da proposta — "traduzir os dados").

A forma organizada é a estrutura de dados que representa fielmente o que o
`cargo-modules` extrai, independente de como a ferramenta a produziu. Ela é o
alvo único que tanto o printer do fork (que a emite) quanto o L1 (que a
consome) miram — o que resolve a circularidade entre os dois.

Os artefatos abaixo, ainda não criados, nascerão sob o contexto desta decisão,
nesta ordem:

| Artefato | Ordem | Natureza da relação com esta decisão |
|----------|-------|--------------------------------------|
| spec L0 — forma organizada | 1º (passo 2 da proposta) | Define a estrutura fiel ao que o `cargo-modules` extrai (nó: caminho, nome, tipo, visibilidade; aresta dirigida: tipo Owns/Uses). Alvo do printer e do L1 |
| (futuro) prompt L1 — cálculo do raio | depois (passo 4 da proposta) | Consome um recorte da forma organizada e calcula o raio (base/folha, alcance, profundidade). O recorte — quais campos o cálculo usa — é decisão deste prompt, não da forma organizada |
| (futuro) prompt L3 — adaptador da fonte | depois (passo 4 da proposta) | Invoca o binário do fork e traduz o JSON para a forma organizada que o L1 consome |

---

## Consequências

**Positivas**:
- A topologia do projeto-lente permanece limpa: nenhum código sem prompt L0
  dentro das pastas do lattice; o linter de nucleação não precisa de exceções.
- A fronteira é honesta: o fork é externo porque está fora do repositório.
- A decisão é reversível: se o `cargo-modules` upstream vier a oferecer saída
  JSON (ou aceitar uma contribuição de printer/conector), o L3 troca o fork
  pela ferramenta oficial sem que nenhum outro estrato sinta.
- O L1 fica isolado de toda esta discussão: ele consome a estrutura genérica
  independentemente de como ela foi produzida.

**Negativas**:
- Dois repositórios a gerenciar.
- O ambiente onde a lente roda precisa do binário do fork instalado — um passo
  de setup adicional, e uma dependência de toolchain Rust recente (o
  `cargo-modules` v0.26 exige rust-version 1.91, edition 2024) com rust-analyzer.
- A manutenção do fork acompanha as mudanças do rust-analyzer; o printer JSON,
  por ser arquivo novo que não modifica o pipeline existente, tende a sofrer
  poucos conflitos de merge, mas depende da estabilidade da API interna
  (`Graph`, `Item`, `Relationship`).

**Neutras**:
- O printer JSON, por viver no fork MPL-2.0, é licenciado sob MPL-2.0. Isso não
  contamina o projeto-lente, que permanece sob a licença que o autor escolher;
  a MPL é copyleft por arquivo, não por projeto.

---

## Alternativas Consideradas

| Alternativa | Prós | Contras |
|-------------|------|---------|
| Submódulo git dentro do projeto-lente | Reprodutibilidade (clone traz o fork travado numa versão) | Submódulos são frágeis de manejar; o fork pesado entra no fluxo de clone; fronteira mais turva |
| Pasta `vendor/` fora do lattice | Simples no dia a dia | Exige código Rust pesado MPL dentro do repositório e ensinar o linter a ignorar a pasta; força resolver a lacuna do Tekt agora |
| Criar camada nova "L_externo" no lattice | Daria um lugar nomeado ao fork | Altera a estrutura canônica do Tekt (mudança pesada) para abrigar o que já é, conceitualmente, dependência externa; o conteúdo seria ou L3 (adaptador, já coberto) ou dependência (fork, fora do lattice) |
| Parsear o DOT em vez de gerar JSON | Não exige fork | DOT é formato de layout visual; parsing frágil; informação (tipo de aresta, visibilidade) fica em comentários e atributos misturados |
| Modificar o pipeline existente do fork (não printer irmão) | — | Alta fricção de manutenção a cada atualização do upstream |

---

## Referências

- Proposta da Lente — `proposta-lente.md`, seções 2, 3, 7
- cargo-modules — https://github.com/regexident/cargo-modules (MPL-2.0)
- Lacuna da Arena registrada para o Tekt v1.4 — ver `LESSONS.md` (a Arena como
  bancada não cobre código externo estável mantido)
