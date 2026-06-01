# Prompt: Aplicação de Resolução de Colisões (`lente_resolve`)

**Camada**: L1 — Núcleo (pureza absoluta)
**Criado em**: 2026-05-27 (reescrito conforme ADR-0005)
**Estado**: `PROPOSTO`
**Decisões de origem**: ADR-0004 (cascata de resolução), **ADR-0005**
(validação empírica e ajustes: nomeação por contador, MesmoItem raro,
enriquecimento opcional), spec `forma-organizada.md` (Limites 1-6).
**Depende de**: `lente_core` (com `No.id`, `Aresta.id_from`/`id_to`, tipo
`Veredito`, todos já existentes). **NÃO depende de** `lente_investiga`.
**Arquivos a gerar**: novo crate `lente_resolve` no workspace; testes inline

---

## Contexto

O `lente_resolve` aplica no grafo o veredito que o `lente_investiga` produz
para uma colisão de path. É o segundo dos dois componentes do mecanismo de
resolução (ADR-0004).

A medição (3 rodadas, validada no ADR-0005) mostrou o quadro real:

- **97,4% das colisões** são decididas pela Estratégia 1 como
  `Distintos/VizinhancaDisjunta`. Este é o caso comum.
- **`MesmoItem` ocorreu 0 vezes** em 384 colisões. É caso teórico, mantido
  por completude, mas não é o caminho comum.
- A evidência dominante (`VizinhancaDisjunta`) é **topológica** — diz "as
  vizinhanças são disjuntas", **não diz qual trait** distingue as cópias.

Isso muda o desenho de nomeação em relação ao que o ADR-0004 §3 propunha
originalmente (que assumia o trait disponível). A convenção real, fixada no
ADR-0005, é: **contador por ordem de id como padrão; trait via enriquecimento
opcional**.

---

## Restrições estruturais

- **L1 — pureza absoluta.** Zero I/O, zero dependências externas, só stdlib.
  `cargo tree -p lente_resolve` mostra só o crate (+ `lente_core`).
- **Não investiga.** Recebe `Veredito` pronto. Não lê fontes, não examina
  vizinhança para decidir. Só aplica.
- **Não modifica o `Grafo` original.** Produz um `Grafo` novo (operação
  pura). O grafo de entrada permanece intacto.
- **Dependência única**: `lente_core`. Não depende de `lente_investiga`.

---

## Instrução

### Estrutura do crate

Criar novo crate no workspace (atualizar `Cargo.toml` da raiz). Nome:
`lente_resolve`. Diretório: decisão do gerador, registrar no laudo (sugestão:
análoga ao `05_investiga/`, ex.: `06_resolve/`).

`Cargo.toml`: `edition = "2024"`, `rust-version = "1.91"`, dependência única
(`lente_core` por path).

### Função pública

Dado o grafo, o path colidente, e o veredito, retorna o grafo resolvido:

```rust
pub fn aplicar(
    grafo: &Grafo,
    colisao: &Path,
    veredito: &Veredito,
) -> Result<Grafo, ErroResolve>
```

`ErroResolve` é um enum próprio cobrindo os modos de falha.

### Comportamento por variante de Veredito

**`Veredito::Distintos { evidencia }` — o caso comum (97,4% na medição).**

Separar as identidades dos nós colidentes:

- O grafo de saída tem os mesmos nós, mas as cópias colidentes recebem
  **paths novos distintos** (a colisão deixa de existir no grafo de saída).
- **Nomeação por contador (padrão)**: ordenar as cópias colidentes por `id`
  crescente. O nó com menor id vira `<path>#1`, o próximo `<path>#2`, etc.
  Ex.: `ErroRaio::fmt` (ids 100 e 101) vira `ErroRaio::fmt#1` (id 100) e
  `ErroRaio::fmt#2` (id 101). Determinístico e estável: a mesma ordem de id
  produz a mesma numeração, o que preserva comparabilidade entre versões
  (proposta §6).
- **Nomeação por trait (enriquecimento opcional)**: se a `evidencia` carrega
  informação de trait (`ImplDeTraitsDiferentes { traits }`, que vem da E2
  quando o enriquecimento está ligado — ver ADR-0005 Ajuste 3), usar o trait
  no nome: `ErroRaio::<Display>::fmt`. Quando a evidência é
  `VizinhancaDisjunta` (sem trait — o caso comum), usar o contador.

  A decisão de qual evidência chega aqui é do `lente_infra` (ele liga ou não
  o enriquecimento). O `lente_resolve` apenas reage: se há trait na evidência,
  nomeia por trait; se não, nomeia por contador. Não é o `lente_resolve` que
  decide ligar enriquecimento.

- **Redistribuição de arestas**: as arestas do grafo de entrada que
  referenciavam as cópias colidentes (por `id_from`/`id_to`) são
  redistribuídas para os paths novos. Como cada aresta referencia um `id`
  específico (graças à identidade-por-nó), **a redistribuição é determinística
  e sem ambiguidade**: a aresta com `id_to == 100` vai para `ErroRaio::fmt#1`,
  a com `id_to == 101` vai para `ErroRaio::fmt#2`. Os `id_from`/`id_to` das
  arestas permanecem (apontam para os mesmos ids); só os paths `from`/`to`
  são atualizados para os novos nomes quando referenciam um nó renomeado.

  Nota: esta é a propriedade que a identidade-por-nó garantiu e que o desenho
  original (laudo 0004) não tinha. Antes, a redistribuição era indeterminada
  (o prompt antigo do lente_resolve previa `RedistribuicaoIndeterminada`).
  Agora, com `id_from`/`id_to`, ela é determinística. A variante de erro
  `RedistribuicaoIndeterminada` **não é mais necessária**.

**`Veredito::MesmoItem` — caso teórico (0 ocorrências na medição).**

Unificar as cópias num único nó:

- O grafo de saída tem um nó com aquele path.
- As arestas que apontavam para qualquer das cópias passam a apontar para o
  nó único.
- Se as cópias divergem em `name`/`kind`/`visibility`, preservar a do menor
  id e registrar a discrepância (decisão do gerador: aviso no resultado? log?
  Justificar no laudo).
- Manter este caminho por completude, mesmo que a medição não o tenha
  exercido. Testá-lo com grafo forjado.

**`Veredito::NaoDeterminado { diagnostico }`.**

- Retorna `Err(ErroResolve::ColisaoNaoResolvida(diagnostico))`. O grafo não é
  modificado.
- O diagnóstico é repassado intacto para o chamador (`lente_infra`).
- Este é o caso do Limite 6 (colisões de macro). O `lente_resolve` não
  inventa resolução; propaga o "não consegui".

### Modos de falha (`ErroResolve`)

Enum cobrindo ao menos:

- `ColisaoNaoResolvida(String)` — veredito foi NaoDeterminado.
- `ColisaoInexistente` — o path passado não tem cópias colidentes no grafo
  (chamada incorreta — só um nó, ou nenhum, com aquele path).
- `IdInconsistente` — a evidência referencia ids que não correspondem aos
  nós colidentes do path (bug do chamador ou do lente_investiga).

A variante `RedistribuicaoIndeterminada` do prompt antigo **não existe mais**
— a identidade-por-nó tornou a redistribuição sempre determinística.

---

## Critérios de Verificação

```
Dado grafo com dois nós path "X" (ids 1 e 2) e arestas referenciando cada id
Quando aplicar com Veredito::Distintos { VizinhancaDisjunta }
Então o grafo de saída tem nós "X#1" (id 1) e "X#2" (id 2)
E as arestas foram redistribuídas pelos ids (a de id_to=1 aponta para X#1, etc.)
E não há mais colisão de path no grafo de saída

Dado o mesmo grafo, com Veredito::Distintos { ImplDeTraitsDiferentes {
traits: ("Display", "Debug") } } (enriquecimento ligado)
Quando aplicar
Então os nós viram "X::<Display>::..." e "X::<Debug>::..." (nomeação por trait)

Dado grafo com dois nós path "X" e Veredito::MesmoItem
Quando aplicar
Então o grafo de saída tem UM nó "X" e as arestas de ambos apontam para ele

Dado Veredito::NaoDeterminado
Quando aplicar
Então retorna Err(ColisaoNaoResolvida) com o diagnóstico, grafo não modificado

Dado path sem colisão (só um nó com aquele path)
Quando aplicar
Então retorna Err(ColisaoInexistente)

Dado o grafo de saída de qualquer caso de sucesso
Então invariante: ids únicos, paths únicos (após resolução), integridade
referencial das arestas (id_from/id_to referenciam nós existentes)
```

Casos a cobrir:
- `Distintos` com contador (caso comum) — verificar nomes e redistribuição.
- `Distintos` com trait (enriquecimento) — verificar nomes por trait.
- `MesmoItem` — unificação (caso teórico, mas testado).
- `NaoDeterminado` — propagação do erro.
- `ColisaoInexistente` — chamada incorreta.
- Colisão com 3+ cópias (contador #1/#2/#3) — a medição viu colisões com
  11+ cópias; o contador precisa funcionar para n > 2.
- Determinismo: aplicar duas vezes ao mesmo grafo produz o mesmo resultado
  (mesma numeração por id).

---

## Resultado esperado

- Crate `lente_resolve` no workspace.
- Função `aplicar` com os comportamentos por variante de veredito.
- Enum `ErroResolve` (sem `RedistribuicaoIndeterminada`).
- Nomeação por contador (ordem de id) como padrão; por trait quando a
  evidência traz.
- Testes inline cobrindo os critérios, incluindo n > 2 cópias.
- **Pureza**: `cargo tree -p lente_resolve` mostra só o crate.
- **Laudo de execução** em `00_nucleo/lessons/`: diretório escolhido,
  formato exato dos paths novos (encaixe com o newtype `Path` — verificar se
  `#1` e `<Display>` são aceitos pelo `Path`), decisão sobre divergência de
  campos no `MesmoItem`, e qualquer descoberta.

---

## Observações sobre o newtype `Path`

O `Path` do `lente_core` pode ter restrições sobre o que aceita como string
(verificar a implementação atual). Os nomes novos (`X#1`, `X::<Display>::fmt`)
precisam ser aceitos pelo `Path`. Se o `Path` rejeitar `#` ou `<>`, há duas
saídas: relaxar o `Path` (mudança no `lente_core`, registrar) ou escolher
outro separador. Verificar antes de assumir; registrar no laudo o que foi
encontrado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação conforme ADR-0005. Nomeação por contador (ordem de id) padrão, trait opcional. Caminho Distintos comum, MesmoItem por completude. Redistribuição determinística via id (sem RedistribuicaoIndeterminada). | novo crate lente_resolve/ |
