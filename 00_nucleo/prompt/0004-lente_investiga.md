# Prompt: Investigação de Colisões de Path (`lente_investiga`)

**Camada**: L1 — Núcleo (pureza absoluta)
**Criado em**: 2026-05-27
**Estado**: `PROPOSTO`
**Decisões de origem**: ADR-0004 (resolução de colisões de path), ADR-0003
(workspace), ADR-0002 (modelagem), spec `forma-organizada.md`
**Depende de**: adição do tipo `Veredito` ao `lente_core` (ver instrução)
**Arquivos a gerar**: novo crate `lente_investiga` no workspace (diretório a
escolher, sugestão `01_core_investiga/` ou `05_investiga/` ou similar — decisão
do gerador, registrar no laudo); módulos internos para as duas estratégias;
testes inline

---

## Contexto

Quando o adaptador L3 (`lente_infra`) traduz o JSON do fork para o `Grafo`,
pode encontrar **colisões de path**: dois nós distintos com o mesmo `path`
canônico (Descoberta 2 do laudo 0003). Caso conhecido: `ErroRaio` tem `impl
Display` e `#[derive(Debug)]`, ambos com método `fmt`, que o `cargo-modules`
emite como dois nós com o mesmo path `lente_core::domain::raio::ErroRaio::fmt`.

O ADR-0004 decidiu resolver essas colisões em vez de aceitá-las como limite.
A resolução é dividida em duas responsabilidades:

- **`lente_investiga`** (este componente): investiga e **classifica** a
  colisão. Não modifica grafo, não nomeia identidades.
- **`lente_resolve`** (componente irmão, futuro): recebe a classificação e
  **aplica** no grafo.

Este componente é puramente analítico.

---

## Restrições Estruturais

- **L1 — pureza absoluta.** Zero I/O, zero dependências externas, só stdlib.
  Sem `serde`, sem rede, sem disco, sem relógio. O `cargo tree -p
  lente_investiga` deve mostrar só o crate.
- **Não nomeia identidades.** Quando concluir "distintos", reporta a evidência
  mas não cria paths novos. Nomear é responsabilidade do `lente_resolve` (ADR-
  0004, item 3).
- **Não modifica o grafo.** Recebe dados, devolve `Veredito`. Operação pura.
- **Dependências de outros crates do workspace**: depende apenas de
  `lente_core`. Não depende de `lente_resolve` nem de `lente_infra`.

---

## Pré-requisito: tipo `Veredito` no `lente_core`

Antes do `lente_investiga` ser escrito, o `lente_core` precisa receber o tipo
`Veredito` e seus auxiliares. Esta adição **é parte deste prompt**, mas em
arquivo separado (sugestão: `01_core/src/entities/veredito.rs`, reexportado
por `entities/mod.rs`).

Tipos a adicionar ao `lente_core`:

- `Veredito` (enum) com variantes:
  - `MesmoItem` — colisão verdadeira; os dois nós são o mesmo, devem ser
    unificados.
  - `Distintos { evidencia: Evidencia }` — colisão aparente; os dois nós são
    diferentes, eis a evidência da distinção.
  - `NaoDeterminado { diagnostico: String }` — cascata esgotada; descrever em
    diagnostico o que cada estratégia tentou e por que não bastou.
- `Evidencia` (enum) com variantes:
  - `VizinhancaDisjunta` — as arestas que entram/saem de cada nó são
    diferentes o bastante para indicar itens distintos. Pode carregar um
    resumo (ex.: contagens de arestas exclusivas de cada lado).
  - `ImplDeTraitsDiferentes { traits: (String, String) }` — a estratégia de
    código-fonte encontrou dois blocos `impl <Trait> for <Tipo>` com o método
    em questão; carrega os nomes dos traits (ex.: `("Display", "Debug")`).
  - (outros podem ser adicionados conforme novas evidências apareçam, mas as
    duas acima cobrem as estratégias deste prompt)

`Veredito` e `Evidencia` são tipos puros, sem I/O — perfeitamente coerentes
com L1. Devem ter testes próprios mínimos (construção e inspeção) no
`lente_core`, e seus testes existentes devem continuar passando (não-regressão).

---

## Instrução

### Estrutura do crate

Criar novo crate no workspace (atualizar o `Cargo.toml` da raiz para incluir o
novo membro). Nome: `lente_investiga`. Diretório: decisão do gerador, registrar
no laudo. Sugestão: dentro de um diretório análogo a `01_core/` (ex.:
`05_investiga/` ou similar), porque o crate é L1 mas conceitualmente uma
extensão analítica do núcleo.

`Cargo.toml` do crate: `edition = "2024"`, `rust-version = "1.91"`, dependência
única (`lente_core = { path = "../01_core" }`).

### Função pública

Uma operação principal: dado o par de nós colidentes, a vizinhança no grafo, e
opcionalmente o conteúdo de arquivos `.rs`, retorna o `Veredito`.

Assinatura sugerida (ajustável):

```rust
pub fn investigar(
    par: &ParColidente,
    vizinhanca: &Vizinhanca,
    fontes: Option<&[ArquivoFonte]>,
) -> Veredito
```

Onde:

- `ParColidente`: par de referências aos dois nós do grafo que colidem (mesmo
  `path`). Pode ser uma struct com referências aos `No` do `lente_core`, ou
  índices.
- `Vizinhanca`: estrutura que dá, para cada nó do par, as arestas que entram e
  saem dele. Tipo a definir no próprio `lente_investiga` (ou no `lente_core`
  se o gerador julgar que faz mais sentido).
- `ArquivoFonte`: caminho lógico (string, sem leitura de disco) + conteúdo
  como `String`. Tipo a definir aqui ou no `lente_core`. Marcado como
  `Option<&[...]>` porque a Estratégia 1 (vizinhança) não precisa de fontes;
  só a Estratégia 2 precisa.

### Cascata interna (duas estratégias)

A função `investigar` orquestra duas estratégias internamente, em ordem:

**Estratégia 1 — vizinhança no grafo.**

Compara as arestas que entram e saem de cada nó. Critério inicial (ajustável
por experimentação registrada no laudo):

- Se ambos os nós têm arestas e essas arestas são **disjuntas** (origens e
  destinos diferentes), conclui `Distintos { evidencia: VizinhancaDisjunta
  {...} }`.
- Se as arestas são **idênticas ou quase idênticas** (mesmas origens e
  destinos), conclui `MesmoItem`. (Caso raro mas conceitualmente possível.)
- Se há sobreposição parcial significativa, ou se um dos nós tem poucas
  arestas e a comparação fica inconclusiva, **a estratégia não decide** e
  passa para a Estratégia 2.

O critério exato de "disjuntas", "idênticas", "inconclusivo" é decisão do
gerador, registrada no laudo com a justificativa. Sugestões: usar conjuntos
de `(origem, destino)`, ou comparar listas ordenadas. Não inventar
heurísticas com thresholds mágicos; preferir critérios categóricos (laudo
0002 D1 estabeleceu esse princípio para o cálculo do raio; vale aqui também).

**Estratégia 2 — leitura de código (texto).**

Recebe o conteúdo dos arquivos `.rs` do crate-alvo como `String` (lido pelo
`lente_infra`, este crate não toca disco). Reconhece **padrões textuais
limitados** para encontrar blocos `impl <Trait> for <Tipo>`:

- Buscar ocorrências de `impl <algo> for <Tipo>` onde `<Tipo>` é o tipo dos
  dois nós colidentes.
- Em cada bloco encontrado, identificar os métodos declarados (`fn <nome>`).
- Se o método em questão aparece em dois blocos `impl` com `<Trait>`
  diferentes, conclui `Distintos { evidencia: ImplDeTraitsDiferentes {
  traits: (T1, T2) } }`.

**Casos não cobertos pelo parser** (decisão registrada no ADR-0004):

- Genéricos com `where` clauses complexas.
- Atributos `#[cfg(...)]` que mudam quais impls são gerados.
- Macros que geram impls (ex.: `pin_project_lite`, `derive` exóticos).
- Comentários, strings ou código dentro de macros que confundam o padrão.

Quando o parser não distingue, ou não encontra dois `impl` para o tipo, ou os
métodos não se diferenciam por trait, conclui `NaoDeterminado { diagnostico:
... }` com mensagem explicando o que foi tentado.

**Quando ambas as estratégias falham**: `Veredito::NaoDeterminado` com
diagnóstico do que cada uma tentou.

### Estrutura interna do crate

Sugestão (ajustável pelo gerador, justificada no laudo):

- `src/lib.rs` — função pública `investigar`, reexporta tipos auxiliares.
- `src/vizinhanca.rs` — implementação da Estratégia 1.
- `src/fontes.rs` — implementação da Estratégia 2 (parser de impl).
- Testes inline em cada módulo.

---

## Critérios de Verificação

```
Dado dois nós com mesmo path, com vizinhanças completamente disjuntas
Quando investigar é chamado
Então retorna Veredito::Distintos com evidência VizinhancaDisjunta

Dado dois nós com mesmo path, com vizinhanças idênticas
Quando investigar é chamado
Então retorna Veredito::MesmoItem

Dado dois nós com mesmo path, vizinhanças ambíguas (parcial), SEM fontes
Quando investigar é chamado (sem fontes)
Então retorna Veredito::NaoDeterminado com diagnóstico mencionando que a
Estratégia 1 foi inconclusiva e a Estratégia 2 não foi tentada

Dado o caso ErroRaio: dois nós "ErroRaio::fmt", vizinhanças ambíguas, com
fontes contendo dois blocos "impl Display for ErroRaio" e "impl Debug for
ErroRaio" cada um declarando "fn fmt"
Quando investigar é chamado COM as fontes
Então retorna Veredito::Distintos com evidência ImplDeTraitsDiferentes {
traits: ("Display", "Debug") }

Dado fontes com padrão não-canônico (ex.: impl gerado por macro)
Quando investigar é chamado
Então retorna Veredito::NaoDeterminado com diagnóstico explicando que o
parser não reconheceu o padrão

Dado entrada inválida (ex.: par de nós com paths diferentes — não é
realmente uma colisão)
Quando investigar é chamado
Então retorna Veredito::NaoDeterminado OU erro de programação (decisão do
gerador, justificada no laudo)
```

Casos a cobrir nos testes:

- Caso canônico de Display+Debug (o ErroRaio simplificado, como string
  literal no teste, sem ler disco).
- Vizinhança disjunta clara (nó A com 3 usuários, nó B com 3 usuários
  diferentes).
- Vizinhança idêntica (mesmo item duplicado).
- Vizinhança ambígua + ausência de fontes → não determinado.
- Vizinhança ambígua + fontes não-canônicas → não determinado.
- Construção dos tipos `Veredito` e `Evidencia` no `lente_core` (testes
  mínimos no crate-core).

---

## Resultado Esperado

- Adição do tipo `Veredito` (e auxiliares) ao `lente_core`, com testes
  mínimos. Não-regressão: os 22 testes existentes do `lente_core` continuam
  passando.
- Crate `lente_investiga` no workspace, com `Cargo.toml` próprio e dependência
  só em `lente_core`.
- Função `investigar` implementando a cascata.
- Testes inline cobrindo os critérios.
- **Pureza**: `cargo tree -p lente_investiga` mostra só o crate; `cargo tree
  -p lente_core` continua puro.
- **Laudo de execução** em `00_nucleo/lessons/`: decisões tácitas (diretório
  escolhido, critérios exatos de "disjuntas/idênticas/inconclusivo", estrutura
  de `Vizinhanca`, comportamento exato em entrada inválida, qualquer caso de
  borda descoberto durante a implementação).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação inicial. Primeiro dos dois componentes do mecanismo de resolução do ADR-0004. Tipo Veredito adicionado ao lente_core. | `lente_core/src/entities/veredito.rs`, novo crate `lente_investiga/` |
