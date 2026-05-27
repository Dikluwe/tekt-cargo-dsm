# ADR-0002: Modelagem do grafo de dependências em código

**Status**: `PROPOSTO`
**Data**: 2026-05-27

---

## Contexto

A spec da forma organizada (`00_nucleo/specs/forma-organizada.md`) define a
estrutura do grafo que a lente consome: `crate`, `nodes[]` (path, name, kind,
visibility), `edges[]` (from, to, relation). A fonte concreta é o JSON do fork
do `cargo-modules` (ADR-0001), já validado contra dado real.

A spec descreve a forma de modo abstrato. Antes de materializá-la em código
Rust, há três decisões de modelagem que **transcendem um único componente** —
afetam o tipo de dados, o cálculo do raio (L1) e o filtro de stdlib (L1), e a
forma como o adaptador (L3) desserializa. Por afetarem múltiplos componentes,
são registradas aqui como ADR, e não no histórico de um prompt isolado.

---

## Decisão

### Decisão 1 — Valores fechados como enums fortes, não strings

`kind`, `relation` e `visibility` chegam no JSON como texto. No código, são
modelados como **enums Rust fortes** (ex.: `Relation::Owns`, `Relation::Uses`;
`Kind::Fn`, `Kind::Struct`, etc.), não como `String`.

Razão: a spec já declara esses campos como listas fechadas. Um enum é a
expressão fiel de "lista fechada" em Rust. A desserialização valida na borda —
um valor fora da lista vira erro no ponto de entrada (L3), não uma string
inválida que se propaga até o núcleo. O cálculo do raio compara variantes de
enum (seguro, exaustivo, o compilador obriga a tratar todos os casos) em vez de
comparar strings.

Custo aceito: a definição dos enums precisa espelhar os valores que o fork
emite. Se o fork passar a emitir um `kind` novo, o enum é atualizado. Para
`relation` (dois valores estáveis) o custo é nulo; para `kind` (17 valores
presos ao Rust) o custo é baixo e localizado.

### Decisão 2 — Estrutura fiel na entrada; indexação no cálculo

A modelagem separa duas representações:

- **Tipo de entrada** (a forma organizada): espelha o JSON — lista de nós e
  lista de arestas. Fiel, simples, desserializa direto do JSON. É o que o L3
  produz e o que entra no núcleo.
- **Estrutura de cálculo**: o componente que calcula o raio constrói, a partir
  do tipo de entrada, sua própria estrutura indexada interna (ex.: mapas de
  `path` para vizinhos de entrada e de saída) quando precisa percorrer o grafo
  com eficiência.

Razão: o cálculo do raio percorre o grafo muitas vezes ("quem depende de X",
direta e indiretamente). Sobre listas planas isso seria custoso (varrer todas
as arestas a cada consulta). Uma estrutura indexada torna o percurso eficiente.
Mas indexar a forma de entrada a afastaria do JSON e a complicaria. Separar
mantém a entrada fiel e põe a eficiência onde ela importa — no cálculo.

Restrição de pureza (L1): a estrutura indexada usa **apenas tipos da biblioteca
padrão** (ex.: `HashMap`, `Vec`). Não se adota biblioteca externa de grafos
(como `petgraph`), porque L1 não admite dependências externas. A indexação é
feita à mão sobre a stdlib.

### Decisão 3 — Marca de stdlib computada, não armazenada no nó

O filtro de stdlib (Limite 2 da spec) precisa saber quais nós são da stdlib
(`std`, `core`, `alloc`). Essa marca **não** é um campo do nó. Ela é computada
quando o filtro precisa, inspecionando o prefixo do `path`.

Razão: o JSON não traz esse campo. Adicioná-lo ao tipo afastaria o tipo da
fonte fiel (o tipo de entrada deve espelhar o JSON). Computar sob demanda
mantém o tipo enxuto e fiel.

**Esta é a mais fraca e a mais reversível das três decisões.** Ela afeta
apenas o tipo de dados e o filtro (não o cálculo do raio nem o adaptador), e
inverter para um campo armazenado, se a performance do filtro exigir, é uma
mudança localizada. Fica registrada aqui por completude da modelagem, com a
ressalva de que pode ser revista na spec do filtro sem necessidade de novo ADR
se a única motivação for performance do próprio filtro.

---

## Prompts Afetados

| Prompt / componente | Como esta decisão o molda |
|---------------------|---------------------------|
| Tipo de dados da forma organizada (L1) | Decisão 1 (enums) e Decisão 2 (entrada fiel) definem sua estrutura. Decisão 3 determina que ele NÃO tem campo de stdlib. |
| Cálculo do raio (L1) | Decisão 2 — constrói a estrutura indexada interna, só com stdlib. |
| Filtro de stdlib (L1) | Decisão 3 — computa a marca de stdlib pelo prefixo do path. |
| Adaptador da fonte (L3) | Decisão 1 — desserializa o JSON validando os enums na borda. |

---

## Consequências

**Positivas**:
- Validação dos valores fechados acontece na borda (L3), não no núcleo. Dados
  inválidos da fonte falham cedo, no ponto certo.
- O núcleo (L1) opera sobre tipos seguros e exaustivos; o compilador garante
  tratamento de todos os casos de `kind`/`relation`/`visibility`.
- A forma de entrada permanece fiel ao JSON — desserialização trivial, fácil de
  auditar contra a fonte.
- A eficiência do cálculo fica contida no componente de cálculo, sem complicar
  o tipo de dados nem ferir a pureza de L1 (sem biblioteca externa).

**Negativas**:
- Os enums precisam ser mantidos em sincronia com os valores que o fork emite.
  Um `kind` novo no fork exige atualizar o enum (e, idealmente, um teste que
  detecte valor desconhecido na desserialização).
- Há duas representações do grafo (entrada fiel + estrutura de cálculo), com um
  passo de construção entre elas. É complexidade aceita em troca de fidelidade
  na entrada e eficiência no cálculo.

**Neutras**:
- A Decisão 3 pode ser revertida localmente se a performance do filtro pedir,
  sem impacto nas outras decisões.

---

## Alternativas Consideradas

| Decisão | Alternativa rejeitada | Por quê |
|---------|----------------------|---------|
| 1 | Strings para os valores fechados | Tolerante a valores novos sem quebrar, mas adia a validação e espalha checagem de string pelo núcleo. A spec declara listas fechadas; enum é a expressão fiel disso. |
| 2 | Indexar já na forma de entrada | Afasta a entrada do JSON e a complica. Fidelidade na entrada vale mais; eficiência pertence ao cálculo. |
| 2 | Usar `petgraph` para a estrutura de cálculo | Fere a pureza de L1 (dependência externa). Indexação à mão sobre stdlib resolve sem violar o estrato. |
| 3 | Armazenar `is_stdlib` no nó | Adiciona campo que o JSON não tem, afastando o tipo da fonte fiel. Computar pelo prefixo do path é suficiente. |

---

## Referências

- ADR-0001 — fonte do grafo (fork do cargo-modules)
- `00_nucleo/specs/forma-organizada.md` — a forma que estas decisões materializam
- Pureza de L1 — `MANIFESTO.md` (L₁ não admite dependências externas)
