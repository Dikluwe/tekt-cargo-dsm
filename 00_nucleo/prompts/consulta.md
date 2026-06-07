# Prompt de Nucleação: `consulta` — o vocabulário de pedido (domain)
Hash do Código: 77030bb9

**Camada**: L1 — Núcleo. Apenas stdlib. Sem I/O. Sem deps externas.
**Unidade**: `01_core/core/src/domain/consulta.rs` (crate `lente_core`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0056-estagio2_mover_vocabulario_l1.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

O **vocabulário de pedido** da lente: como o usuário aponta a fonte do grafo, o
alvo, o escopo e o modo de `Uses`. Eram tipos do `lente_wiring` (L4); o Estágio 2
do refactor V3+V12 os trouxe para o L1 (dados puros) — a fiação os importa nas
assinaturas, e a CLI deixa de depender da fachada L4.

## Comportamento e invariantes

- **`FonteGrafo`** — `Json(String)` (JSON pronto) ou `Pacote(String)` (nome do
  pacote; a fiação invoca o fork).
- **`AlvoBusca`** — `PorPath(Path)` ou `PorId(usize)`.
- **`Escopo`** — `Completo` (default; inclui sysroot) ou `SeuCodigo` (filtra
  stdlib). `impl Default = Completo`.
- **`ModoUses`** — `Todas` (default) ou `SoReferencia` (descarta `Import`, Limite
  4). `impl Default = Todas`.

## Restrições (L1 puro)

- Dados puros: só `String`/`Path`/unit. Sem I/O, sem dep externa.
- `Escopo`/`ModoUses` derivam `Debug/Clone/Copy/PartialEq/Eq` + `Default`.

## Critérios de Verificação

```
Dado Escopo::default() Então Completo
Dado ModoUses::default() Então Todas
Dado AlvoBusca::PorPath(p) Então carrega o Path; PorId(n) Então o id
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"FonteGrafo","kind":"enum","members":["Json","Pacote"]},{"name":"Escopo","kind":"enum","members":["Completo","SeuCodigo"]},{"name":"ModoUses","kind":"enum","members":["Todas","SoReferencia"]},{"name":"AlvoBusca","kind":"enum","members":["PorPath","PorId"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0059) do vocabulário de consulta. Código inalterado. | `01_core/core/src/domain/consulta.rs` |
