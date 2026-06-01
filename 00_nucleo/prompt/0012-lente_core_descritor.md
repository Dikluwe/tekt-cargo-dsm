# Prompt: Descritor Semântico no `lente_core` (campos + remodelagem do Kind)

**Camada**: L1 — Núcleo (pureza absoluta)
**Criado em**: 2026-05-28
**Estado**: `PROPOSTO`
**Decisões de origem**: prompt do descritor no fork (fork 0.27.0); investigação
do descritor (`lab/investiga-descritor-fork/relatorio.md`); decisões do autor
(formas simples para trait/cfg; remodelar o Kind separando tipo base de
modificadores).
**Pré-requisito**: fork 0.27.0 publicado e instalado (emite trait, trait_ref,
is_const/is_async/is_unsafe, cfg, macro_kind, is_non_exhaustive no descritor
padrão).
**Primeiro da cascata a jusante.** Os próximos (lente_infra, lente_investiga,
lente_resolve) dependem deste.
**Arquivos afetados**: `01_core/src/entities/grafo.rs` (tipo `No`, enum `Kind`,
`TryFrom`), testes do `lente_core`.

---

## Contexto

O fork 0.27.0 passou a emitir um descritor semântico por nó. O `lente_core`
precisa modelar os campos novos no tipo `No`, e — decisão do autor —
**remodelar o enum `Kind`** para separar o tipo base dos modificadores
(`const`/`async`/`unsafe`), que hoje vêm achatados na string `kind`.

Esta é mudança **só no `lente_core`** (modelagem). O `lente_infra` (que
desserializa o JSON e preenche esses campos) é o próximo prompt da cascata,
não este. Este prompt define a **forma**; o próximo conecta a forma aos dados.

Decisões do autor sobre a forma dos campos novos (todas pela forma simples,
estruturar quando houver uso concreto):

- `trait` e `trait_ref`: `Option<String>` (sem parsing).
- `cfg`: `Option<String>` (expressão como texto, não interpretada).
- `macro_kind`: `Option<String>`.
- `is_non_exhaustive`: `bool`.
- Modificadores: separados do `Kind` (remodelagem — ver abaixo).

---

## Restrições estruturais

- **L1 — pureza absoluta.** Zero I/O, zero dependências externas, só stdlib.
  `cargo tree -p lente_core` continua mostrando só o crate.
- **Os campos novos são tipos puros.** Strings, Option, bool, enum. Nada que
  exija dependência.
- **Cuidado com não-regressão coordenada**: este crate é base de toda a
  cascata. A remodelagem do `Kind` muda o `TryFrom<&str>`, o que afeta o
  `lente_infra` (próximo prompt) e os testes de vários crates. Este prompt
  só toca o `lente_core`; mas deve deixar claro no laudo o que vai precisar
  mudar a jusante (o `lente_infra` desserializa `kind`).

---

## Parte 1 — Campos novos no `No` (formas simples)

Adicionar ao struct `No` os campos (todos opcionais/default, porque nem todo
nó os tem):

```rust
pub struct No {
    pub id: usize,
    pub path: Path,
    pub name: String,
    pub kind: Kind,                       // remodelado — ver Parte 2
    pub modificadores: Modificadores,     // NOVO — ver Parte 2
    pub visibility: Visibility,
    pub crate_name: String,
    pub trait_: Option<String>,           // NOVO — nome do trait (None se não é de impl-de-trait)
    pub trait_ref: Option<String>,        // NOVO — referência do trait com args
    pub cfg: Option<String>,              // NOVO — expressão cfg como texto
    pub macro_kind: Option<String>,       // NOVO — tipo de macro (None se não é macro)
    pub is_non_exhaustive: bool,          // NOVO
}
```

Notas:

- `trait_` com underscore porque `trait` é palavra reservada em Rust (mesmo
  padrão do `crate_name`, laudo 0001 D5).
- `trait` e `trait_ref` são `Option<String>` — sem parsing de args agora.
  Estruturar (separar nome de args) fica para quando houver uso concreto.
- `cfg` é `Option<String>` — carrega a expressão (ex.: `"all(unix, feature
  = \"x\")"`) como texto. Estruturar o tipo recursivo fica para quando a
  lente for processar cfg.
- Estes campos não afetam o cálculo do raio nem o `lente_investiga` por
  enquanto — são carregados, disponíveis para uso futuro. O único campo com
  uso concreto iminente é o `trait_`/`trait_ref` (resolve a D4 no
  `lente_investiga`, prompt futuro da cascata).

---

## Parte 2 — Remodelagem do `Kind` (separar tipo base de modificadores)

### O problema atual

Hoje o `Kind` é um enum de 17 variantes, e a string `kind` do fork vinha como
`"const async unsafe fn"` — modificadores achatados junto do tipo base. O
`TryFrom<&str>` parseia a string inteira.

### A remodelagem

Separar em dois:

- **`Kind`** passa a ser só o **tipo base**: `Fn`, `Struct`, `Enum`, `Trait`,
  `Mod`, etc. (as variantes que representam o que o item é, sem os
  modificadores).
- **`Modificadores`** (novo tipo) carrega `is_const`, `is_async`, `is_unsafe`
  como três `bool`:

```rust
pub struct Modificadores {
    pub is_const: bool,
    pub is_async: bool,
    pub is_unsafe: bool,
}
```

Quando nenhum modificador aplica (struct, enum, etc.), `Modificadores` é tudo
`false` (use `Default`).

### Fonte da verdade dos modificadores: os booleanos, não a string

**Atenção a uma armadilha**: o fork 0.27.0 emite os modificadores de DUAS
formas — embutidos na string `kind` (`"const async unsafe fn"`, mantida por
retrocompatibilidade) E como booleanos separados (`is_const` etc.). O
`lente_core`/`lente_infra` deve usar **uma fonte só** para evitar divergência.

Decisão: usar os **booleanos** como fonte da verdade dos modificadores. A
string `kind` é parseada só para o **tipo base** (descartando os
modificadores dela). Os modificadores vêm dos booleanos do fork.

Consequência para o `TryFrom<&str>` do `Kind`: ele agora parseia só o tipo
base. Se receber `"const async unsafe fn"`, deve extrair `Fn` (despir os
modificadores). Como fazer: reconhecer e descartar os prefixos `const`,
`async`, `unsafe` antes de parsear o tipo base. Os modificadores em si NÃO
saem do `TryFrom` do Kind — eles vêm dos booleanos (responsabilidade do
`lente_infra`, próximo prompt, que lê os campos `is_const` etc. do JSON e
constrói o `Modificadores`).

Alternativamente, se for mais limpo: o `TryFrom<&str>` do `Kind` aceita tanto
`"fn"` (já despido) quanto `"const async unsafe fn"` (despe e pega `fn`). O
gerador decide a forma exata e registra no laudo.

### Testes do Kind

Os testes existentes que parseavam `"const async unsafe fn"` esperando uma
variante única precisam ser ajustados: agora `"const async unsafe fn"` →
`Kind::Fn` (o tipo base), e os modificadores são responsabilidade separada.
Adicionar testes:

- `"fn"` → `Kind::Fn`, sem modificadores na string.
- `"const fn"` → `Kind::Fn` (despe o const).
- `"async unsafe fn"` → `Kind::Fn` (despe ambos).
- Tipos sem modificadores (`"struct"`, `"enum"`) → variante correspondente.
- Valor desconhecido → erro (como antes).

---

## Critérios de Verificação

```
Dado o struct No
Quando construído com os campos novos
Então tem trait_, trait_ref, cfg (Option<String>), macro_kind (Option<String>),
is_non_exhaustive (bool), e modificadores (Modificadores)

Dado Kind::try_from("const async unsafe fn")
Então retorna Ok(Kind::Fn) — o tipo base, modificadores despidos

Dado Kind::try_from("fn")
Então retorna Ok(Kind::Fn)

Dado Kind::try_from("struct")
Então retorna Ok(Kind::Struct)

Dado Kind::try_from("valor_desconhecido")
Então retorna Err (como antes)

Dado Modificadores::default()
Então is_const, is_async, is_unsafe são todos false

Dado um No com trait_ = Some("Display")
Então o campo é acessível e carrega o nome do trait
```

Casos a cobrir: construção do No com todos os campos; TryFrom do Kind
despindo modificadores; Modificadores default; os 17 tipos base parseando.

---

## Resultado esperado

- `No` com os campos novos (formas simples).
- `Kind` remodelado (só tipo base), `Modificadores` novo tipo.
- `TryFrom<&str>` do `Kind` despe modificadores e parseia o tipo base.
- Testes ajustados e novos, todos verdes.
- **Pureza**: `cargo tree -p lente_core` mostra só o crate.
- **Laudo de execução** em `00_nucleo/lessons/`: o que mudou, como o TryFrom
  trata a string com modificadores, e — importante — **o que a cascata a
  jusante vai precisar** (o lente_infra precisa ler os campos novos do JSON
  e construir Modificadores a partir dos booleanos; sinalizar isso para o
  próximo prompt).

---

## O que NÃO entra neste prompt (cascata a jusante)

- **`lente_infra`**: desserializar os campos novos do JSON, construir
  `Modificadores` dos booleanos, preencher o `No`. Próximo prompt.
- **`lente_investiga`**: usar o `trait_` por nó para resolver a D4
  (nomeação por trait com id correto). Prompt posterior.
- **`lente_resolve`**: nomear por trait com a precisão que o trait-por-nó
  agora permite. Prompt posterior.
- **Estruturar trait/cfg**: fica para quando houver uso concreto.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | Descritor semântico no lente_core: campos novos (trait_, trait_ref, cfg, macro_kind, is_non_exhaustive) em forma simples; Kind remodelado para tipo base puro; Modificadores como tipo separado (fonte: booleanos do fork). Primeiro da cascata a jusante do descritor. | 01_core/src/entities/grafo.rs |
