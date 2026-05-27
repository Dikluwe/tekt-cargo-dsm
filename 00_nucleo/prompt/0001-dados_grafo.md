# Prompt: Tipo de Dados da Forma Organizada

**Camada**: L1 — Núcleo
**Criado em**: 2026-05-27
**Estado**: `PROPOSTO`
**Decisões de origem**: spec `forma-organizada.md`, ADR-0002 (modelagem)
**Arquivos a gerar**: `01_core/entities/grafo.rs`, `01_core/entities/grafo.test.rs`
(ou o layout de teste que o projeto adotar para Rust)

---

## Contexto

Este é o primeiro componente de código do projeto-lente. Ele materializa, em
tipos Rust, a forma organizada definida em `00_nucleo/specs/forma-organizada.md`
— a estrutura que representa o grafo de dependências de um sistema.

A forma organizada é o contrato central: o cálculo do raio (L1, componente
futuro) a consome, e o adaptador (L3, componente futuro) a produz. Este
componente define apenas os **tipos** — a forma. Não define o cálculo do raio,
não desserializa JSON, não lê arquivo nenhum.

A fonte concreta hoje é o JSON do fork do `cargo-modules` (ADR-0001), mas este
tipo é **agnóstico de fonte**: ele não sabe que os dados vieram de JSON, nem do
cargo-modules, nem do Rust. É só a estrutura de um grafo de dependências. Essa
agnose é deliberada — permite que a fonte mude no futuro (outra ferramenta,
análise própria, outra linguagem) sem tocar neste núcleo.

---

## Restrições Estruturais

- **Camada L1 — pureza absoluta.** Zero I/O. Zero dependências externas. Apenas
  a biblioteca padrão do Rust. **Proibido `serde`, `serde_json`, ou qualquer
  crate externo.** A desserialização do JSON é responsabilidade do L3 (adaptador
  futuro), não deste componente.
- **Sem comportamento de cálculo.** Este componente define os tipos e, no
  máximo, construtores e acessadores triviais. O cálculo do raio (base/folha,
  alcance, profundidade) é outro componente. Não antecipar essa lógica aqui.
- **Valores fechados como enums** (ADR-0002, Decisão 1): `kind`, `visibility` e
  `relation` são enums Rust, não `String`.
- **Estrutura fiel à entrada** (ADR-0002, Decisão 2): a forma espelha o JSON —
  uma coleção de nós e uma coleção de arestas. NÃO construir aqui índices de
  adjacência ou estrutura otimizada para percurso; isso é trabalho do cálculo.
- **Sem marca de stdlib no nó** (ADR-0002, Decisão 3): o nó NÃO tem campo
  indicando se é stdlib. Essa marca é computada pelo filtro (componente futuro)
  a partir do prefixo do path.

---

## Instrução

Criar os tipos Rust que representam a forma organizada.

**Enums (valores fechados da spec):**

- `Relation` — duas variantes: `Owns`, `Uses`.
- `Visibility` — variantes cobrindo: `pub` (público), `pub(crate)`, `pub(in
  caminho)` (carregando o caminho), `pub(super)`, e privado. Modelar a variante
  `pub(in ...)` de modo a preservar o caminho que ela carrega.
- `Kind` — variantes cobrindo a lista fechada da spec: `crate`, `mod`, `fn`,
  `const fn`, `async fn`, `unsafe fn`, `struct`, `union`, `enum`, `variant`,
  `const`, `static`, `trait`, `unsafe trait`, `type`, `builtin`, `macro`.

**Structs:**

- `No` (nó): `path` (identidade canônica), `name`, `kind: Kind`,
  `visibility: Visibility`.
- `Aresta`: `from`, `to` (ambos referenciando `path` de nós), `relation:
  Relation`.
- `Grafo`: o nome do sistema-raiz (`crate`), a coleção de nós, a coleção de
  arestas.

**Pontos de modelagem a decidir na geração (documente a escolha no laudo):**

- O `path` é a identidade do nó. Decidir se o tipo do path é `String` ou um
  tipo dedicado (newtype) — o newtype dá segurança de tipo (não confundir um
  path com uma string qualquer) ao custo de verbosidade. Qualquer escolha é
  aceitável se justificada.
- Como `Grafo` guarda os nós: `Vec<No>` é fiel ao JSON. Um `HashMap<Path, No>`
  por identidade adiantaria buscas mas afastaria da entrada fiel e aproximaria
  da indexação que o ADR-0002 reservou ao cálculo. Preferir a forma fiel
  (`Vec`) salvo justificativa.

**Conversão a partir de texto (para o L3 usar depois):**

Como o L3 vai precisar converter as strings do JSON nesses enums, exponha um
meio de construir cada enum a partir de seu texto canônico (ex.: um
`impl TryFrom<&str>` ou um método `from_str` que retorna `Result`). Isto NÃO é
desserialização de JSON (que fica no L3) — é só a tradução texto→enum, lógica
pura, sem dependência externa. O caso de texto desconhecido deve retornar erro
(não panic, não valor default), para que o L3 detecte um valor que o fork
emitiu e o enum não cobre (ADR-0002, consequência da Decisão 1).

---

## Critérios de Verificação

```
Dado o texto "uses"
Quando convertido para Relation
Então resulta em Relation::Uses

Dado o texto "owns"
Quando convertido para Relation
Então resulta em Relation::Owns

Dado um texto que não está na lista fechada (ex.: "borrows")
Quando convertido para Relation (ou Kind, ou Visibility)
Então retorna erro — não panic, não valor default

Dado os textos de cada kind da lista fechada
Quando convertidos para Kind
Então cada um resulta na variante correspondente

Dado os textos de visibilidade ("pub", "pub(crate)", "pub(super)", privado, e
um "pub(in crate::a::b)")
Quando convertidos para Visibility
Então resultam nas variantes corretas, e a variante pub(in ...) preserva o
caminho "crate::a::b"

Dado um Grafo construído com alguns nós e arestas
Quando inspecionado
Então os nós e arestas estão acessíveis e preservam seus campos

Dado um Grafo vazio exceto pelo nó-raiz (zero arestas)
Quando construído
Então é um Grafo válido com um nó e nenhuma aresta
```

Casos de borda a cobrir nos testes: texto desconhecido em cada enum;
visibilidade `pub(in ...)` com caminho; grafo mínimo (só raiz).

---

## Resultado Esperado

- `01_core/entities/grafo.rs`: os enums `Relation`, `Visibility`, `Kind`; as
  structs `No`, `Aresta`, `Grafo`; as conversões texto→enum com erro para valor
  desconhecido. Tudo com o cabeçalho de linhagem apontando para este prompt.
- `01_core/entities/grafo.test.rs` (ou layout equivalente): testes cobrindo os
  critérios acima.
- **Verificação de pureza**: nenhum `use` de crate externo; nenhuma operação de
  I/O; o arquivo compila dependendo apenas da stdlib.
- **Laudo de execução** (LESSONS, L5): registrar ao final — o que o prompt
  pediu, o que foi gerado, e as decisões tácitas (tipo do path, estrutura de
  armazenamento do Grafo, forma das conversões).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação inicial. Primeiro componente do projeto. Tipo puro da forma organizada, sob spec forma-organizada e ADR-0002. | grafo.rs, grafo.test.rs |
