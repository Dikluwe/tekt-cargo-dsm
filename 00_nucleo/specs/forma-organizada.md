# Spec L0 — Forma Organizada do Grafo de Dependências

**Camada**: L0 (Semente) — especificação de estrutura de dados
**Criado em**: 2026-05-27
**Estado**: `PROPOSTO`
**Decisão de origem**: ADR-0001 (fonte do grafo = fork do cargo-modules)
**Validação**: forma confirmada contra dado real (typst_syntax: 1501 nós /
4353 arestas com sysroot; crate de smoke: 17 nós / 33 arestas)

---

## Contexto

Esta spec define a **forma organizada**: a estrutura de dados que representa
o grafo de dependências de um sistema de forma utilizável pela lente,
independente de como a ferramenta-fonte a produziu (proposta, seção 7, passo 2).

A forma organizada é o **contrato central** do projeto. Dois componentes a
miram:

- O adaptador L3 a produz (lê o JSON do fork e o entrega nesta forma).
- O cálculo do raio L1 a consome (computa o raio de impacto sobre ela).

A fonte concreta hoje é o subcomando `export-json` do fork do `cargo-modules`
(ADR-0001), cujo JSON já tem esta forma. Mas a forma organizada é definida aqui
de modo independente da fonte: se a fonte mudar (outra ferramenta, análise
própria, outra linguagem), esta forma permanece o alvo, e só o L3 muda.

Esta spec descreve **o que a forma contém** — não o cálculo do raio (isso é a
spec do L1) nem a filtragem de ruído (isso é um componente L1 separado). A
forma é a tradução fiel do que a fonte extrai; o recorte do que o cálculo usa
é decisão posterior.

---

## A Estrutura

```json
{
  "crate": "<nome canônico do sistema-raiz>",
  "nodes": [
    { "path": "...", "name": "...", "kind": "...", "visibility": "..." }
  ],
  "edges": [
    { "from": "...", "to": "...", "relation": "owns" | "uses" }
  ]
}
```

Três campos de topo: o nome do sistema-raiz (`crate`), a lista de nós
(`nodes`), a lista de arestas (`edges`).

### Nó (`nodes[]`)

| Campo | Significado | Restrição |
|-------|-------------|-----------|
| `path` | Caminho canônico do item (ex.: `meu_sistema::modulo::Item`). | **Identidade única** do nó. Não há dois nós com o mesmo `path`. É por ele que as arestas referenciam nós. |
| `name` | Nome curto do item, sem o caminho. | Pode repetir entre nós (dois módulos podem ter um item de mesmo nome em caminhos diferentes). Não é identidade. |
| `kind` | O tipo do item. | Valor de uma lista fechada (ver abaixo). |
| `visibility` | O alcance de visibilidade do item. | Valor de uma lista fechada (ver abaixo). |

**Lista fechada de `kind`** (origem: `cargo-modules`, específica de Rust):
`crate`, `mod`, `fn`, `const fn`, `async fn`, `unsafe fn`, `struct`, `union`,
`enum`, `variant`, `const`, `static`, `trait`, `unsafe trait`, `type`,
`builtin`, `macro`.

**Lista fechada de `visibility`** (origem: `cargo-modules`, específica de Rust):
`pub`, `pub(crate)`, `pub(in crate::<path>)`, `pub(super)`, `priv`.

### Aresta (`edges[]`)

| Campo | Significado | Restrição |
|-------|-------------|-----------|
| `from` | `path` do nó de origem. | Deve referenciar um `path` existente em `nodes`. |
| `to` | `path` do nó de destino. | Deve referenciar um `path` existente em `nodes`. |
| `relation` | O tipo da relação. | `owns` ou `uses` (lista fechada de dois valores). |

A aresta é **dirigida**: `from → to`. A direção é semântica e deve ser
preservada — invertê-la inverte o significado da dependência.

- `owns`: contenção estrutural. Um módulo "possui" os itens declarados dentro
  dele. É a hierarquia de organização do código.
- `uses`: uso. Um item refere-se a outro (chama uma função, referencia um
  tipo, implementa um trait). É a dependência funcional.

---

## Invariantes da Forma

Estas propriedades devem valer para qualquer instância válida da forma
organizada. São o que o L1 pode assumir e o que o L3 deve garantir:

1. **Identidade por `path`**: cada `path` em `nodes` é único.
2. **Integridade referencial**: todo `from` e `to` de cada aresta referencia
   um `path` presente em `nodes`. Não há arestas com pontas soltas.
3. **Direção preservada**: a ordem `from → to` reflete a direção real da
   dependência tal como a fonte a extraiu.
4. **Valores fechados**: `kind` e `relation` e `visibility` só assumem valores
   das listas declaradas acima. Um valor fora da lista é erro da fonte.
5. **Determinismo**: para o mesmo sistema sem alteração, a forma é idêntica
   entre extrações (nós ordenados por `path`, arestas por `(from, to,
   relation)`). Isto permite comparar duas versões de um sistema por diferença
   (proposta, seção 6).

---

## Limites Declarados

A honestidade sobre os limites é parte da spec, não nota de rodapé (proposta,
seção 3 e 10). A forma organizada tem os seguintes limites conhecidos, medidos
contra dado real:

### Limite 1 — Dependências via derive exigem sysroot

Sem a stdlib carregada na análise, relações criadas por `#[derive(...)]` (ex.:
o `clone` gerado por `#[derive(Clone)]`) **não aparecem** no grafo — o item
derivado não é sintetizado, então as arestas para ele não existem.

Medição real (typst_syntax): sem sysroot, 970 nós / 2295 arestas; com sysroot,
1501 nós / 4353 arestas. **Cerca de metade das arestas de um crate de AST vem
de derives.** Omiti-las faria a lente mostrar um raio de impacto
sistematicamente menor que o real.

**Decisão**: a forma organizada, para a lente, é gerada **com sysroot** — a
fidelidade é o padrão. (A fonte oferece a opção como opt-in; a política de
sempre usá-la é da lente e vive no L3.)

### Limite 2 — Fronteira stdlib / sistema-alvo é fina

Com sysroot ligado, a forma inclui nós da stdlib (`std`, `core`, `alloc`) e
arestas que apontam para eles. Medição real (typst_syntax com sysroot): 47 nós
de stdlib (3,1%) e 857 arestas apontando para stdlib (19,7%).

Esses nós/arestas são candidatos a filtragem (ruído para a compreensão), mas a
filtragem é **delicada**: o que liga um item do sistema-alvo a um trait de
stdlib (ex.: `MinhaStruct → core::clone::Clone`) passa por um `impl` que é do
**sistema-alvo**, não da stdlib. Um filtro ingênuo (remover tudo que começa com
`core::`/`std::`) removeria o trait mas precisa preservar o `impl` do
sistema-alvo e as conexões internas — senão reintroduz a cegueira do Limite 1.

**Consequência**: a forma organizada **inclui** os nós/arestas de stdlib (é
tradução fiel). A decisão de esconder a stdlib é um componente L1 separado (um
filtro), não parte desta forma. O filtro deve respeitar a fronteira descrita
acima.

### Limite 3 — Raio estrutural, não comportamental

A forma captura dependências de forma (quem usa o quê estruturalmente). Ela não
captura dependência de comportamento — se um item depende do *comportamento*
interno de outro sem uma relação estrutural visível, a forma não a vê. Este é
um limite herdado da natureza da análise de dependências (proposta, seção 3),
não da fonte específica.

### Limite 4 — `uses` agrega imports no nível do módulo

A relação `uses` agrupa, sob um mesmo valor, duas origens distintas no código:
declarações de import (`use foo::bar;`) e referências diretas em corpos e
assinaturas. Declarações `use` são atribuídas ao **módulo** onde a declaração
aparece; referências diretas são atribuídas ao **item** que as faz.

Exemplo observado (crate-amostra de smoke, 2026-05-27):

- `runner::run → runner::Report (uses)` — item→item, da assinatura
  `fn run(...) -> Report` (referência direta).
- `runner → parser::tokenize (uses)` — **módulo**→item, da declaração
  `use crate::parser::{tokenize};` no topo do módulo. A aresta **não** alcança
  o item `runner::run` que de fato chama `tokenize()`.

Consequência: o raio "se eu mexer em X, quem sente?" tem um piso de
granularidade do tamanho do módulo quando o consumidor alcança X via import. A
forma não distingue, dentro do módulo, qual item depende do importado.

A forma organizada não tenta refinar isso — é tradução fiel do que a fonte
entrega. Atenuar o efeito é responsabilidade de quem consome (L1) ou tarefa
para evoluir a fonte (ver "Nota de Evolução").

### Limite 5 — Reexports não têm relation própria

Um reexport (`pub use foo::bar;`) é representado como `uses` ordinária:
`<módulo-reexportador> → bar (uses)`. Não existe valor de `relation` que
distinga reexport de uso interno.

Exemplo observado (crate-amostra de smoke, 2026-05-27): `pub use parser::Token;`
em `lib.rs` produz `crate_amostra → parser::Token (uses)`. Olhando apenas o
grafo, essa aresta é indistinguível de um uso direto da raiz a `Token` (se
houvesse).

Consequência: para perguntas sobre raio de impacto (quem depende de quem) a
indistinção é tolerável — uma aresta de dependência é uma aresta de
dependência. Para perguntas sobre **interface pública** ("o que este sistema
exibe ao mundo?") a forma não separa o que é interface intencional do que é
uso casual de implementação.

A forma aceita a indistinção como herdada. Refiná-la é decisão da fonte (ver
"Nota de Evolução").

---

## Nota de Evolução: subtipos de `uses`

Os Limites 4 e 5 — e potencialmente outros ainda não observados — compartilham
uma raiz comum: o valor `uses` agrupa categorias semanticamente distintas
(import declarado, referência direta, reexport, talvez ainda outras como
implementação de trait). A agregação é herdada da fonte
(`cargo-modules export-json`), que hoje não emite subtipos.

Esta nota existe para deixar rastreável uma evolução futura possível: se a
lente, em uso real, esbarrar nesses limites com frequência mensurável, o
caminho natural é propor ao fork a emissão de **subtipos de `uses`** em vez
de tentar reconstruí-los a partir do que já se tem. Candidatos plausíveis,
derivados dos limites já observados:

- `uses/import`    — declaração `use foo::bar;` (origem semântica: módulo).
- `uses/reference` — referência em corpo ou assinatura (origem: item).
- `uses/reexport`  — declaração `pub use foo::bar;` (origem: módulo;
  marcador de interface intencional).

Isto é horizonte declarado, não tarefa do primeiro passo (proposta, seção 10:
"multiplicidade de opções é horizonte declarado, não trabalho do primeiro
passo"). Por que não agora:

- A fonte já entrega informação suficiente para o cálculo de raio começar.
- A inadequação do `uses` único só será concreta depois que a lente estiver
  operando sobre dados reais e produzindo respostas mensuravelmente piores do
  que poderiam ser com mais granularidade.

Sintoma concreto para reavaliar: a lente, em uso real, repetidamente entregar
raios cujo módulo-pai é grande demais para o humano discernir "onde dentro" a
mudança importa, e essa falta de discernimento atrapalhar a decisão que a
lente existe para apoiar. Quando esse sintoma aparecer, o trabalho se desloca
para o fork (passo 1 da proposta, refinado), não para a forma organizada.

---

## Critérios de Verificação

Dado um JSON produzido pela fonte (fork `cargo-modules export-json --sysroot`)
Quando interpretado como forma organizada
Então:

```
- as três chaves de topo (crate, nodes, edges) estão presentes
- cada nó tem path, name, kind, visibility
- cada path em nodes é único
- cada from/to de aresta referencia um path existente em nodes
- todo kind pertence à lista fechada
- toda visibility pertence à lista fechada
- todo relation é owns ou uses
- duas extrações do mesmo sistema sem alteração produzem JSON idêntico
```

Casos de borda:
- Sistema vazio (só o nó-raiz, zero arestas): forma válida, `nodes` com um
  elemento, `edges` vazio.
- Nó sem dependências de saída: válido (folha, ninguém de quem ele depende).
- Nó sem dependências de entrada: válido (ninguém depende dele).

---

## Resultado Esperado

Esta spec define a forma. Dela derivam, em momentos posteriores:

- **Tipo de dados** (L1): a representação em código Rust desta forma (structs
  e enums com os valores fechados). Nasce de prompt próprio.
- **Adaptador** (L3): lê o JSON do fork e o materializa nesta forma,
  garantindo os invariantes. Nasce de prompt próprio.
- **Filtro de stdlib** (L1): componente que recebe a forma completa e devolve
  uma forma sem o ruído de stdlib, respeitando a fronteira do Limite 2. Nasce
  de prompt próprio.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação inicial. Forma validada contra dado real (typst_syntax, smoke crate). Limites medidos e declarados. | (nenhum código ainda — esta spec precede os componentes) |
| 2026-05-27 | Adição dos Limites 4 (`uses` agrega imports no nível do módulo) e 5 (reexports sem `relation` própria), observados no crate-amostra de smoke. Adicionada Nota de Evolução sobre subtipos de `uses` como horizonte rastreável. | (nenhum código) |
