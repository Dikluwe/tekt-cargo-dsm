# Spec L0 — Forma Organizada do Grafo de Dependências

**Camada**: L0 (Semente) — especificação de estrutura de dados
**Criado em**: 2026-05-27
**Reconciliada em**: 2026-06-03 (laudo 0028 — alinha a spec com o sistema
construído: identidade por `id`, forma crua vs resolvida, campos do
descritor semântico, camada de resolução, Limite 6)
**Estado**: `CONSTRUÍDO` — os três derivados nasceram: tipo (`lente_core`),
adaptador (`lente_infra`), filtro de stdlib (`lente_filtro`). Veja
"Resultado Esperado".
**Decisão de origem**: ADR-0001 (fonte = fork do cargo-modules)
**ADRs aplicáveis**: 0002 (modelagem), 0004 (resolução de colisões),
0005 (validação pós-medição), 0006 (nomeação por trait)
**Validação**: forma confirmada contra dado real:
- crate-amostra de smoke: 17 nós / 33 arestas (2026-05-27)
- `typst_syntax` cru: 1501 nós / 4353 arestas com sysroot
- `lente_core` cru: 108 nós / 278 arestas (laudo 0025)
- `egui` cru: 3694 nós / 13937 arestas (laudo 0027)
- 17 crates do typst: 384 colisões; 97,4% resolvidas pela cascata
  (laudo da 3ª medição; ADR-0005)

---

## Contexto

Esta spec define a **forma organizada**: a estrutura de dados que representa
o grafo de dependências de um sistema de forma utilizável pela lente,
independente de como a ferramenta-fonte a produziu (proposta, seção 7, passo 2).

A forma organizada é o **contrato central** do projeto. Três componentes a
miram:

- O adaptador L3 (`lente_infra`) produz a **forma crua** lendo o JSON do fork.
- A camada de resolução (`lente_investiga` + `lente_resolve`, ambos L1) a
  transforma na **forma resolvida** (paths únicos de novo). É o que o
  ADR-0004 introduziu e o ADR-0005 validou contra o typst.
- O cálculo do raio L1 (`lente_core::domain::raio`) consome a forma
  **resolvida**.

A fonte concreta hoje é o subcomando `export-json` do fork do `cargo-modules`
(ADR-0001), cujo JSON já tem esta forma. Mas a forma organizada é definida
aqui de modo independente da fonte: se a fonte mudar (outra ferramenta,
análise própria, outra linguagem), esta forma permanece o alvo, e só o L3
muda.

Esta spec descreve **o que a forma contém** — não o cálculo do raio (`raio`),
nem a filtragem de ruído (`lente_filtro`), nem o ranking (`lente_ranking`).
Cada um tem seu prompt e seu laudo.

---

## Duas formas: crua e resolvida

A spec original (2026-05-27) tratava a forma como uma só, com unicidade de
path como invariante. A realidade exigiu separar:

- **Forma crua** — saída direta da tradução do JSON do fork pelo `lente_infra`.
  Identidade por `id`; `path` **pode** colidir (no `lente_core`, `ErroRaio::fmt`
  aparece duas vezes — `impl Display` e `#[derive(Debug)]`; no typst, 384
  colisões em 17 crates).
- **Forma resolvida** — saída da cascata `investiga → resolve` aplicada à
  crua. `path` único de novo (ex.: `ErroRaio::<Display>::fmt` e
  `ErroRaio::<Debug>::fmt`). É o que o `raio`, o `filtro` e o `ranking`
  consomem.

A unicidade de path **não some**; muda de lugar. Deixa de ser invariante da
forma crua (onde colisões são dados legítimos) e passa a ser propriedade
**garantida** da forma resolvida.

A "Camada de Resolução" abaixo descreve a passagem entre as duas.

---

## A Estrutura

```json
{
  "crate": "<nome canônico do sistema-raiz>",
  "nodes": [
    {
      "id": 42,
      "path": "meu_sistema::modulo::Item",
      "name": "Item",
      "kind": "struct",
      "visibility": "pub",
      "trait": "Display",
      "trait_ref": "Display",
      "cfg": "feature = \"x\"",
      "macro_kind": null,
      "is_const": false,
      "is_async": false,
      "is_unsafe": false,
      "is_non_exhaustive": false
    }
  ],
  "edges": [
    {
      "id_from": 1,
      "from": "meu_sistema",
      "id_to": 42,
      "to": "meu_sistema::modulo::Item",
      "relation": "owns"
    }
  ]
}
```

Três campos de topo: nome do sistema-raiz (`crate`), lista de nós (`nodes`),
lista de arestas (`edges`).

> Notação: nos nomes de campo do código Rust, `crate` vira `crate_name` (a
> palavra é reservada). Em JSON e nesta spec, segue `crate`.

### Nó (`nodes[]`) — campos da forma crua

| Campo | Significado | Restrição / Origem |
|-------|-------------|--------------------|
| `id` | Identidade canônica do nó, atribuída pelo fork. | **Único** por nó no grafo. (laudo 0006) |
| `path` | Caminho canônico do item (ex.: `meu_sistema::modulo::Item`). | **Pode colidir** entre nós distintos na forma crua; único na resolvida. |
| `name` | Nome curto do item, sem o caminho. | Pode repetir. Não é identidade. |
| `kind` | Tipo base do item. | Lista fechada (ver abaixo). Modificadores ficam em campos próprios. |
| `visibility` | Alcance de visibilidade do item. | Lista fechada (ver abaixo). |
| `trait` | Nome do trait, quando o nó é método de impl-de-trait. | `null` quando não se aplica. Campo do **descritor semântico** (laudos 0012/0013). |
| `trait_ref` | Referência do trait com seus argumentos (texto, sem parsing). | `null` quando não se aplica. Descritor. |
| `cfg` | Expressão `cfg` como texto (sem interpretação). | `null` quando ausente. Descritor. |
| `macro_kind` | Tipo de macro, quando o nó é uma macro. | `null` quando não se aplica. Descritor. |
| `is_const` / `is_async` / `is_unsafe` | Modificadores do item (separados do `kind`). | Bool. Fonte: booleanos do descritor (não a string `kind`). |
| `is_non_exhaustive` | Marcador `#[non_exhaustive]`. | Bool. Descritor. |

Os campos do **descritor semântico** (`trait`, `trait_ref`, `cfg`,
`macro_kind`, e os booleanos) nasceram nos laudos 0012/0013 — o fork passou
a emiti-los e o L3 passou a desserializá-los. A versão original da spec
(2026-05-27) listava só `path`/`name`/`kind`/`visibility`; os demais foram
adicionados aqui.

#### Campo de Cortesia: `crate_name` (no tipo, não no JSON)

O tipo `lente_core::entities::grafo::No` em Rust tem um campo
`crate_name: String`, mas ele **não corresponde** a um campo `crate` por
nó no JSON — o fork 0.27.0 não emite isso. O L3 copia o `crate` raiz do
grafo para todos os nós (laudo 0026). Consequência:

- O valor é igual para todos os nós do mesmo grafo, **inclusive os de
  stdlib** (`core::*`, `alloc::*`, `std::*`).
- O campo **não distingue** o crate-alvo da stdlib. Essa marca é por
  **prefixo do path** (ADR-0002 D3), aplicada no `lente_filtro` (laudo 0025).

#### Lista fechada de `kind` (tipo base apenas)

`crate`, `mod`, `fn`, `struct`, `union`, `enum`, `variant`, `const`,
`static`, `trait`, `type`, `builtin`, `macro`.

> Mudança vs spec 2026-05-27: a versão original misturava modificadores
> (`const fn`, `async fn`, `unsafe fn`, `unsafe trait`) na lista de `kind`.
> O código (`Kind`) separa: `kind` carrega só o tipo base; os modificadores
> vão em `is_const`/`is_async`/`is_unsafe`. Quando o fork emite uma string
> composta (ex.: `"const async fn"`), o `TryFrom` despe os modificadores
> e mantém só o último token. A separação é fonte-de-verdade pelos
> booleanos (laudo 0013), não pela string.

#### Lista fechada de `visibility`

`pub`, `pub(crate)`, `pub(in <caminho>)`, `pub(super)`, `priv`.

### Aresta (`edges[]`)

| Campo | Significado | Restrição |
|-------|-------------|-----------|
| `id_from` | `id` do nó de origem. | **Referência canônica** — resolve colisões de path (laudo 0006). |
| `from` | `path` do nó de origem. | Texto legível; pareado com `id_from`. |
| `id_to` | `id` do nó de destino. | **Referência canônica**. |
| `to` | `path` do nó de destino. | Texto legível; pareado com `id_to`. |
| `relation` | Tipo da relação. | `owns` ou `uses` (lista fechada). |

A aresta é **dirigida**: `from → to`. Direção é semântica e deve ser
preservada — invertê-la inverte o significado.

- `owns`: contenção estrutural. Um módulo "possui" os itens declarados
  dentro dele.
- `uses`: uso. Um item refere-se a outro (chama uma função, referencia um
  tipo, implementa um trait). É a dependência funcional.

---

## Invariantes da Forma

Propriedades que valem para qualquer instância válida. O L1 (cálculo,
filtro, ranking) pode assumi-las; o L3 e a camada de resolução devem
garanti-las.

### Invariantes comuns às duas formas (crua e resolvida)

1. **Identidade por `id`**: cada `id` em `nodes` é único. É por ele que as
   arestas referenciam nós canonicamente. (laudo 0006)
2. **Integridade referencial**: todo `id_from` e `id_to` de cada aresta
   referencia um `id` presente em `nodes`. Não há arestas com pontas
   soltas. (`from`/`to` em texto devem casar com o path do nó identificado.)
3. **Direção preservada**: a ordem `from → to` reflete a direção real
   da dependência.
4. **Valores fechados**: `kind` (após despir modificadores),
   `visibility` e `relation` só assumem valores das listas declaradas.
   Valor fora da lista é erro da fonte; o L3 falha na borda.
5. **Determinismo**: para o mesmo sistema sem alteração, a forma é
   idêntica entre extrações (ordenação por `id`).

### Invariante adicional da forma resolvida

6. **Unicidade de path**: na forma resolvida, cada `path` é único. A camada
   de resolução restaura essa propriedade após renomear nós colidentes
   (ex.: `Tipo::fmt` → `Tipo::<Display>::fmt` / `Tipo::<Debug>::fmt`).

---

## Camada de Resolução (entre forma crua e resolvida)

A passagem da forma crua para a resolvida é feita por dois componentes L1
**puros**, introduzidos pelo ADR-0004 e validados pelo ADR-0005:

- **`lente_investiga`** — dado um par de nós com mesmo `path`, classifica a
  colisão pela vizinhança no grafo (Estratégia 1, "E1") e, quando E1 não
  decide, pela leitura do código-fonte (Estratégia 2, "E2"). Produz um
  **veredito**: `MesmoItem` (mesma identidade alcançável por dois caminhos),
  `Distintos { evidencia }` (itens diferentes com a evidência da E1 ou E2),
  ou `NaoDeterminado` (cascata esgotada).
- **`lente_resolve`** — dado o veredito e o grafo, **aplica**: unifica
  (`MesmoItem`) ou renomeia (`Distintos`). Convenção de nomeação canônica:
  `Tipo::<Trait>::metodo` quando o veredito vem do `trait_`/`trait_ref` do
  descritor (laudo 0006); outras evidências (vizinhança disjunta, módulo
  de origem) usam convenções correlatas.

Validação empírica (ADR-0005, 3ª medição contra 17 crates do typst): **97,4%
das 384 colisões** resolvidas pela E1; a E2 deixou de ser fallback de
**decisão** e virou enriquecimento opcional de **nomeação**. O laudo 0006
do prompt-de-identidade explica por que o salto: a identidade-por-`id` no
fork novo deu à E1 chaves de aresta que de fato discriminam.

Os 2,6% que não resolvem (10 colisões no typst, todas em `typst_macros`)
têm causa específica: **código gerado por macro**. É o Limite 6 abaixo.

---

## Limites Declarados

A honestidade sobre os limites é parte da spec, não nota de rodapé (proposta,
seções 3 e 10). A forma organizada tem os seguintes limites conhecidos,
medidos contra dado real:

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
arestas que apontam para eles. Medições reais:

- `typst_syntax` com sysroot: 47 nós de stdlib (3,1%), 857 arestas → stdlib (19,7%).
- `lente_core`: 17 nós sysroot em 108 (15,7%); 98 arestas removidas pelo filtro de 278 (35,3%).
- `egui`: 60 nós sysroot em 3694 (1,6%); domina o ranking pelo montante.

Esses nós/arestas são candidatos a filtragem (ruído para a compreensão), mas
a filtragem é **delicada**: o que liga um item do sistema-alvo a um trait de
stdlib (ex.: `MinhaStruct → core::clone::Clone`) passa por um `impl` que é do
**sistema-alvo**, não da stdlib.

Verificação empírica no fork 0.27.0 (laudos 0025 e 0027 Fase 1): a
sobreposição "primeiro segmento do path ∈ sysroot ∧ `trait`/`trait_ref`
preenchido" é **zero** em `lente_core` (108 nós) e em `egui` (3694 nós).
O fork nomeia o impl-do-alvo pelo lado do alvo (`lente_core::…::ErroRaio::fmt`
com `trait: "Display"`, não `core::…`), o que torna o filtro por prefixo
do path seguro **por construção neste fork** — sem cláusula híbrida.

**Consequência**: a forma organizada **inclui** os nós/arestas de stdlib (é
tradução fiel). A decisão de esconder a stdlib é um componente L1 separado
(`lente_filtro`, laudo 0025), não parte desta forma.

### Limite 3 — Raio estrutural, não comportamental

A forma captura dependências de forma (quem usa o quê estruturalmente). Ela
não captura dependência de comportamento — se um item depende do
*comportamento* interno de outro sem uma relação estrutural visível, a forma
não a vê. Limite herdado da natureza da análise de dependências (proposta,
seção 3).

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

### Limite 6 — Colisões em código gerado por macro não são resolvíveis automaticamente

A forma organizada usa `id` para distinguir nós com mesmo `path` (Invariante 1),
e o mecanismo de resolução (ADR-0004, validado pelo ADR-0005) decide a
esmagadora maioria das colisões pela vizinhança no grafo. Resta uma classe
que não é resolvível: colisões onde o nó colidente é um **módulo gerado por
macro** (não um tipo), cujas cópias compartilham a aresta `Owns` do
módulo-pai e não têm `impl <Trait> for <tipo>` literal no código-fonte.

Exemplo medido: `typst_macros::util::kw::<nome>` — vários nós com o mesmo
path gerados por uma macro, no mesmo módulo. A resolução por vizinhança não
dispara (há aresta compartilhada com o módulo-pai), e a resolução por
código-fonte não dispara (não existe o `impl` literal — é macro-gerado).

Consequência: para esses casos, a lente reporta a colisão como não resolvida,
com diagnóstico claro, em vez de inventar uma distinção. O usuário vê que
aqueles nós são ambíguos e que a ambiguidade vem de geração por macro.

Magnitude observada: 10 de 384 colisões (2,6%) nos 17 crates do typst, todas
concentradas em `typst_macros`. Crates sem macros geradoras de nomes
colidentes não têm esse limite. Aplicado pelo prompt 0028 (laudo
correspondente), do patch original (ADR-0005, Ajuste 5).

---

## Nota de Evolução: subtipos de `uses` e a família das ambiguidades

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

### O Limite 6 também é família das ambiguidades

O Limite 6 (colisões em código gerado por macro) compartilha raiz com a
família de ambiguidades que a identidade-por-`id` resolveu: são casos onde a
fonte não expõe a distinção que existiria no código semântico. A resolução
para os outros casos veio do fork (identidade-por-nó). Para o Limite 6, a
evolução possível seria o fork identificar nós originados por expansão de
macro — caminho futuro, não trabalho do primeiro passo. O sintoma que
dispararia a reavaliação: crates onde colisões de macro sejam frequentes o
bastante para inviabilizar o uso da lente (não foi o caso no typst, onde
são 2,6% concentrados em um crate).

---

## Critérios de Verificação

Dado um JSON produzido pela fonte (fork `cargo-modules export-json --sysroot
--compact`), quando interpretado como forma organizada (crua), então:

```
- as três chaves de topo (crate, nodes, edges) estão presentes
- cada nó tem id, path, name, kind, visibility (mais o descritor quando aplicável)
- cada id em nodes é único
- cada id_from/id_to de aresta referencia um id existente em nodes
- todo kind (após despir modificadores) pertence à lista fechada
- toda visibility pertence à lista fechada
- todo relation é owns ou uses
- duas extrações do mesmo sistema sem alteração produzem JSON equivalente
  (mesmo conjunto de nós/arestas; ordenação por id determinística)
```

Adicionalmente, após a camada de resolução (`investiga` + `resolve`):

```
- cada path em nodes é único (Invariante 6 — forma resolvida)
- nós colidentes na crua foram resolvidos: unificados (MesmoItem) ou renomeados
  (Distintos, ex.: `Tipo::<Trait>::metodo`), exceto o subconjunto do Limite 6
- arestas atualizadas para apontar para os ids/paths da forma resolvida
- ids dos nós mantidos são preservados (a renomeação muda path, não id)
```

Casos de borda:
- Sistema vazio (só o nó-raiz, zero arestas): forma válida, `nodes` com um
  elemento, `edges` vazio.
- Nó sem dependências de saída: válido (folha, ninguém de quem ele depende).
- Nó sem dependências de entrada: válido (ninguém depende dele).
- Colisão verdadeira (`MesmoItem`): a forma resolvida funde os ids em um;
  caso raro em dados reais (laudo do ADR-0005 Ajuste 4).
- Colisão não resolvível (Limite 6): a forma resolvida **mantém** os ids
  colidentes; o pipeline reporta diagnóstico, não inventa distinção.

---

## Resultado Esperado

Esta spec define a forma. Os três derivados estão **construídos**:

- **Tipo de dados** (L1) — `lente_core::entities::grafo`: structs `No`,
  `Aresta`, `Grafo` + enums `Kind`/`Relation`/`Visibility` +
  `Modificadores`. Nasceu do prompt 0001; ampliado pelos prompts 0006 (id)
  e 0013 (descritor semântico).
- **Adaptador** (L3) — `lente_infra`: lê o JSON do fork, desserializa,
  traduz validando enums e invariantes da borda. Nasceu do prompt 0003;
  ampliado pelos prompts 0017/0018/0022/0023/0024 (invocador do fork,
  detecção de alvo por metadata, diagnóstico de diretório).
- **Filtro de stdlib** (L1) — `lente_filtro`: remove nós de sysroot por
  **prefixo do path** (ADR-0002 D3), preservando os impls-do-alvo
  (Limite 2 verificado seguro neste fork). Nasceu do prompt 0025
  (revisão A).

Componentes adjacentes que existem mas estão fora desta forma:

- **Resolução de colisões** — `lente_investiga` (L1) + `lente_resolve` (L1):
  prompts 0004 e 0010 (ADRs 0004/0005/0006). Operam **sobre** a forma crua,
  produzindo a resolvida.
- **Cálculo do raio** — `lente_core::domain::raio`: prompt 0002. Consome
  a forma resolvida.
- **Ranking** — `lente_ranking` (L1): prompt 0027. Consome a forma
  resolvida e filtrada.
- **Fiação + CLI** — `lente_wiring` (L4), `lente_cli` + `lente_catalogo`
  (L2): prompts 0019/0020/0027.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação inicial. Forma validada contra dado real (typst_syntax, smoke crate). Limites 1–3 medidos e declarados. | (nenhum código ainda — esta spec precede os componentes) |
| 2026-05-27 | Adição dos Limites 4 (`uses` agrega imports no nível do módulo) e 5 (reexports sem `relation` própria), observados no crate-amostra de smoke. Adicionada Nota de Evolução sobre subtipos de `uses` como horizonte rastreável. | (nenhum código) |
| 2026-06-03 | **Reconciliação com o sistema construído** (prompt/laudo 0028): identidade por `id` (forma crua) + unicidade de path como invariante da forma **resolvida** (laudo 0006); nova subseção "Camada de Resolução" descrevendo `investiga`/`resolve` (ADR-0004/0005); estrutura do nó atualizada para os ~12 campos reais do descritor semântico (laudos 0012/0013); `kind` separado dos modificadores; `crate_name` descrito conforme a realidade (crate-raiz copiado, **não** distingue stdlib — laudo 0026); **Limite 6** aplicado (do patch original — colisões geradas por macro); "Resultado Esperado" marcado como **construído** com ponteiros para os crates. Sem mudança de comportamento. | `00_nucleo/specs/forma-organizada.md`; `00_nucleo/specs/patch-spec-limite-6.md` retirado |
