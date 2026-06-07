# Prompt de Nucleação: `lente_app::erro` — tradução do erro agregado (L4)
Hash do Código: 81209ccf

**Camada**: L4 — Fiação (app). Conhece o `ErroLente` (L4) e usa o catálogo (L2).
**Unidade**: `04_wiring/app/src/erro.rs` (crate `lente_app`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0020-l2-cli.md` (a tradução
veio da CLI); subiu ao L4 no Estágio 3 (prompt 0057).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

Traduzir o **`ErroLente`** (o erro agregado da fiação, L4) em **mensagem pronta
para o usuário**, usando os templates do `lente_catalogo` (L2). Mora no `app` (L4)
porque é lá que o `ErroLente` é legitimamente conhecido — é o que **tirou o
`ErroLente` da CLI** sem movê-lo de camada (Estágio 3 do refactor V3+V12).

## Comportamento e invariantes

- **`traduzir(erro, ctx) -> String`** — `match` exaustivo sobre o `ErroLente`:
  cada variante (`Fork`/`Adaptador`/`Resolucao`/`Raio`/`IdInexistente`/
  `ForkSemUsesKind`/`Workspace`/`Diff`) vira a moldura do catálogo
  (`ERRO_*.render`) com o `Display` técnico embutido como `{detalhe}`.
- **`ContextoErro`** — o alvo que o usuário pediu (path ou `id=N`), para a mensagem
  de alvo inexistente usar o que o usuário entende, não o path interno renomeado.
- **Exaustivo**: toda variante nova do `ErroLente` exige um braço aqui (o compilador
  garante) — foi o que conectou as variantes `Workspace`/`Diff` (0045/0047).

## Restrições (L4)

- Importa o `lente_wiring` (`ErroLente`, L4) e o `lente_catalogo` (L2, lateral/para
  baixo). É composição: junta o erro da fiação com o texto da apresentação.

## Critérios de Verificação

```
Dado ErroLente::IdInexistente(42) Quando traduzir Então "Nó com id 42 não existe no grafo"
Dado ErroLente::Raio(_) com alvo pedido "foo::bar" Então usa "foo::bar" (não o interno)
Dado ErroLente::Workspace(_) Então a moldura ERRO_WORKSPACE com o detalhe
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"traduzir","params":["&ErroLente","&ContextoErro"],"return_type":"String"}],"types":[{"name":"ContextoErro","kind":"struct","members":["alvo_informado"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0065) da tradução do ErroLente. Código inalterado. | `04_wiring/app/src/erro.rs` |
