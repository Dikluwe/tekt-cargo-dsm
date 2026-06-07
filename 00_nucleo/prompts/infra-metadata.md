# Prompt de Nucleação: `lente_infra::metadata` — detecção de alvo (L3)
Hash do Código: c0fac9bf

**Camada**: L3 — Infraestrutura. Subprocesso `cargo metadata` é I/O legítimo.
**Unidade**: `03_infra/src/metadata.rs` (crate `lente_infra`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0023-l3-deteccao-alvo-metadata.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

Descobrir **qual alvo** (`--lib` ou `--bin <nome>`) o fork deve analisar para um
pacote, via `cargo metadata` — antes de invocar o fork. Resolve bin+lib por nome
no workspace que o cargo enxerga.

## Comportamento e invariantes

- **`ErroMetadata`** (`pub`) — os modos de falha da descoberta: cargo ausente,
  metadata com status ≠ 0 (manifesto não resolve), JSON inesperado, **pacote
  inexistente** no workspace, **alvos ambíguos** (0 ou ≥2 bins sem lib).
  `Display`+`Error`. Embrulhado em `ErroFork`/`ErroAdaptador`.
- **`detectar_alvo*`** (`pub(crate)`) — roda `cargo metadata --format-version 1`,
  localiza o pacote por `name`, decide a flag de alvo pelos `targets[]` (prefere
  `[lib]`; senão o único `[bin]`).

## Restrições (L3)

- I/O legítimo (`cargo metadata` — subprocesso de propósito distinto do fork).
  Importa só o necessário; **não importa o L4**.

## Critérios de Verificação

```
Dado um pacote com [lib] Então AlvoFork::Lib
Dado um pacote sem lib e um único bin Então AlvoFork::Bin(nome)
Dado pacote inexistente Então Err(PacoteNaoEncontrado)
Dado 0 ou ≥2 bins sem lib Então Err(AlvosAmbiguos)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"invocar_metadata","params":["Option<&Path>"],"return_type":"Result<MetadataOutput, ErroMetadata>"},{"name":"selecionar_alvo","params":["&MetadataPackage"],"return_type":"Result<AlvoFork, ErroMetadata>"},{"name":"detectar_alvo_por_nome","params":["&str","Option<&Path>"],"return_type":"Result<AlvoFork, ErroMetadata>"},{"name":"detectar_pacote_e_alvo_por_diretorio","params":["&Path"],"return_type":"Result<(String, AlvoFork), ErroMetadata>"}],"types":[{"name":"MetadataOutput","kind":"struct","members":["packages"]},{"name":"MetadataPackage","kind":"struct","members":["name","manifest_path","targets"]},{"name":"MetadataTarget","kind":"struct","members":["name","kind"]},{"name":"ErroMetadata","kind":"enum","members":["BinarioNaoEncontrado","FalhaSubprocess","StatusErro","StdoutInvalido","JsonInvalido","PacoteNaoEncontrado","PacoteNoDiretorioNaoEncontrado","AlvosAmbiguos","DiretorioInexistente"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) da detecção de alvo. Código inalterado. | `03_infra/src/metadata.rs` |
