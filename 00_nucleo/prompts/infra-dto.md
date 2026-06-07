# Prompt de Nucleação: `lente_infra::dto` — DTOs do JSON do fork (L3, interno)
Hash do Código: a14648bb

**Camada**: L3 — Infraestrutura. Fronteira de desserialização.
**Unidade**: `03_infra/src/dto.rs` (módulo **interno** `pub(crate)` do `lente_infra`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0003-adaptador_l3.md` (+ 0037
`position`).

> Prompt de **nucleação** (interno de fronteira; fica no walk — regra do 0062).

---

## Propósito

Os **objetos de transferência** que espelham o JSON do fork `cargo-modules` para a
desserialização com `serde`. São a forma **crua** (string-typed) antes da
`traducao` validar e converter para o `lente_core::Grafo` (enums fortes).

## Comportamento e invariantes

- **`GrafoDTO`/`NoDTO`/`ArestaDTO`/`PositionDTO`** (`pub(crate)`,
  `#[derive(Deserialize)]`) — espelham `crate`/`nodes`/`edges` e os campos por nó
  (`id`/`path`/`kind`/`visibility`/`trait`/`position`/…) como **texto/opcionais**,
  fiéis ao que o fork emite (campos ausentes → `Option`/default).
- `position` (`Option<PositionDTO>`, prompt 0037) — `file`/`start_line`/`end_line`.
- **Sem lógica** além da forma — a validação (texto→enum, invariantes) é da
  `traducao`.

## Restrições (L3)

- `serde` permitido (é L3, a borda de desserialização — o L1 **não** o usa).
  `pub(crate)`: a forma crua não escapa do crate.

## Critérios de Verificação

```
Dado o JSON do fork Quando serde_json::from_str::<GrafoDTO> Então parseia os campos
Dado um nó sem position Então PositionDTO ausente (Option None) — não erro
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"GrafoDTO","kind":"struct","members":["crate_name","nodes","edges"]},{"name":"NoDTO","kind":"struct","members":["id","path","name","kind","visibility","is_const","is_async","is_unsafe","is_non_exhaustive","trait_","trait_ref","cfg","macro_kind","position"]},{"name":"PositionDTO","kind":"struct","members":["file","start_line","end_line"]},{"name":"ArestaDTO","kind":"struct","members":["from","id_from","to","id_to","relation","uses_kind"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) dos DTOs. Código inalterado. | `03_infra/src/dto.rs` |
