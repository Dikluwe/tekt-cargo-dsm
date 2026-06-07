# Prompt de Nucleação: `lente_infra::traducao` — DTO → Grafo validado (L3, interno)
Hash do Código: 89807588

**Camada**: L3 — Infraestrutura. A borda de validação texto→enum.
**Unidade**: `03_infra/src/traducao.rs` (módulo **interno** `pub(crate)` do `lente_infra`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0003-adaptador_l3.md` (+ 0013/
0033/0034/0037 — descritor, uses_kind, position).

> Prompt de **nucleação** (interno de borda; fica no walk — regra do 0062).

---

## Propósito

Converter o `GrafoDTO` (cru, string-typed) no `lente_core::Grafo` (enums fortes),
**validando na borda**: cada texto de lista fechada vira enum ou **erra**
(`ValorDesconhecido`), e os invariantes da spec são checados. É onde o JSON do fork
encontra o tipo de dados puro do L1.

## Comportamento e invariantes

- **`traduzir(dto) -> Result<Grafo, ErroAdaptador>`** (`pub(crate)`):
  - `kind`/`visibility`/`relation`/`uses_kind` → enums via `TryFrom` (texto
    desconhecido → `ValorDesconhecido`).
  - `Modificadores` dos booleanos do descritor; `position` (0037) opcional;
    `crate_name` copiado a cada nó (o fork 0.27.0 não emite `crate` por nó).
  - **Invariantes**: `id` único (senão `IdDuplicado`); integridade referencial das
    arestas (`id_from`/`id_to` existem; senão `IdReferenciado`).
- Determinística; a falha é **na borda**, não silenciosa.

## Restrições (L3)

- A validação mora aqui (não no L1). Importa o L1 (`Grafo`/enums) + o `dto`.
  **Não importa o L4**.

## Critérios de Verificação

```
Dado um DTO com kind "fn" Então No.kind = Fn
Dado um DTO com relation "borrows" Então Err(ValorDesconhecido)
Dado dois nós com mesmo id Então Err(IdDuplicado)
Dado uma aresta para id ausente Então Err(IdReferenciado)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"traduzir","params":["GrafoDTO"],"return_type":"Result<Grafo, ErroAdaptador>"}],"types":[],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) da tradução DTO→Grafo. Código inalterado. | `03_infra/src/traducao.rs` |
