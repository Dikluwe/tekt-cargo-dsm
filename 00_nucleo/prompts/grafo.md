# Prompt de Nucleação: `grafo` — a forma de dados do grafo (entities)
Hash do Código: cce2b0c5

**Camada**: L1 — Núcleo. Apenas stdlib. Sem I/O. Sem deps externas.
**Unidade**: `01_core/core/src/entities/grafo.rs` (crate `lente_core`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0001-dados_grafo.md`;
ADRs `0001` (fonte do grafo), `0002` (modelagem); spec `forma-organizada.md`.

> Prompt de **nucleação** (descreve o código existente). Não é o prompt de
> trabalho — aquele fica em `prompt/`.

---

## Propósito

O **tipo de dados da forma organizada**: a representação fiel do grafo de
dependências que o fork `cargo-modules` emite. É a fundação que todos os outros
componentes consomem — listas fechadas como **enums fortes**, entrada fiel,
erro na borda para valor desconhecido.

## Comportamento e invariantes

- **`Path`** — newtype sobre `String` (segurança de tipo do path canônico).
- **Enums de lista fechada** com `TryFrom<&str>` que **erra** (`ValorDesconhecido`)
  para texto fora da lista: `Relation` (`owns`/`uses`), `UsesKind`
  (`reference`/`import`; desconhecido → `Import`, conservador), `Visibility`
  (`pub`/`pub(crate)`/`pub(super)`/`pub(in …)`/`priv`), `Kind` (tipo **base** —
  despe modificadores `const`/`async`/`unsafe`, pega o último token).
- **`Modificadores`** — `is_const`/`is_async`/`is_unsafe` (fonte: booleanos do
  fork, não a string `kind`).
- **`Posicao`** — `file` (absoluto, verbatim), `start_line`/`end_line` (1-based);
  `Option` no `No` (ausente p/ itens sem fonte / JSON antigo).
- **`No`** — identidade por `id`; `path` **pode repetir** (colisões); carrega
  `kind`/`visibility`/`modificadores`/`trait_`/`trait_ref`/`cfg`/`crate_name`/…
- **`Aresta`** — `id_from`/`id_to` (referência canônica que resolve colisões) +
  `from`/`to` (paths) + `relation` + `uses_kind: Option`.
- **`Grafo`** — `crate_name` + `nodes` + `edges`.
- **`ValorDesconhecido`** — erro de tradução texto→enum (`tipo` + `texto`),
  `Display` + `Error`.

## Restrições (L1 puro)

- Zero deps externas — **não usa `serde`** (a desserialização é do L3, `lente_infra`).
- Conversões só **texto→enum**; nenhuma lógica de cálculo aqui.

## Critérios de Verificação

```
Dado "owns"/"uses" Quando Relation::try_from Então Ok; "borrows" Então Err
Dado "const fn" Quando Kind::try_from Então Fn (modificador despido)
Dado "pub(in crate::a)" Quando Visibility::try_from Então PubIn("crate::a")
Dado um No com position None Então é estado válido (não erro)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"ValorDesconhecido","kind":"struct","members":["tipo","texto"]},{"name":"Path","kind":"struct","members":[]},{"name":"Relation","kind":"enum","members":["Owns","Uses"]},{"name":"UsesKind","kind":"enum","members":["Reference","Import"]},{"name":"Visibility","kind":"enum","members":["Pub","PubCrate","PubSuper","PubIn","Priv"]},{"name":"Kind","kind":"enum","members":["Crate","Mod","Fn","Struct","Union","Enum","Variant","Const","Static","Trait","Type","Builtin","Macro"]},{"name":"Modificadores","kind":"struct","members":["is_const","is_async","is_unsafe"]},{"name":"Posicao","kind":"struct","members":["file","start_line","end_line"]},{"name":"No","kind":"struct","members":["id","path","name","kind","modificadores","visibility","crate_name","trait_","trait_ref","cfg","macro_kind","is_non_exhaustive","position"]},{"name":"Aresta","kind":"struct","members":["from","id_from","to","id_to","relation","uses_kind"]},{"name":"Grafo","kind":"struct","members":["crate_name","nodes","edges"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0059) do tipo de dados do grafo. Código inalterado — só cabeçalho + este prompt. | `01_core/core/src/entities/grafo.rs` |
