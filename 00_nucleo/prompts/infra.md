# Prompt de Nucleação: `lente_infra` — o adaptador da fonte (L3)
Hash do Código: 042c8d92

**Camada**: L3 — Infraestrutura. I/O e deps externas permitidos.
**Unidade**: `03_infra/src/lib.rs` (crate `lente_infra`, fachada).
**Origem de trabalho** (referência): `00_nucleo/prompt/0003-adaptador_l3.md`;
ADRs `0001` (fonte), `0002` (modelagem).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **fachada do L3**: extrai o grafo de um crate Rust invocando o fork do
`cargo-modules` como subprocesso e desserializando o JSON em `lente_core::Grafo`,
validando os enums e os invariantes da spec na **borda**. Re-exporta as peças
internas (`fork`, `diff`, `metadata`, `workspace`).

## Comportamento e invariantes

- **`extrair_grafo(caminho) -> Result<Grafo, ErroAdaptador>`**: invoca o fork no
  diretório e desserializa.
- **`desserializar_grafo(json) -> Result<Grafo, ErroAdaptador>`**: fachada limpa
  para quem já tem o JSON (lido de arquivo/stdin) — `serde` + tradução.
- **`ErroAdaptador`** (enum, `Display`+`Error`) — o erro de L3 do adaptador:
  binário ausente, subprocesso falhou, saída não-UTF8, JSON inválido, **valor
  desconhecido** na borda (`kind`/`visibility`/`relation`), invariante violado
  (id duplicado, id referenciado inexistente), detecção de alvo.
- **Re-exports**: `ler_diff`/`ErroDiff` (do `diff`), `ErroMetadata`, e
  `enumerar_membros`/`versao_toolchain`/`chave_cache`/`extrair_grafo_cacheado`/
  `ErroWorkspace`/`MembroWorkspace` (do `workspace`).

## Restrições (L3)

- I/O legítimo (subprocesso do cargo). Importa o L1 (`lente_core`); **não importa
  o L4**. `serde`/`serde_json` permitidos (é L3).

## Critérios de Verificação

```
Dado JSON válido do fork Quando desserializar_grafo Então um Grafo com o crate-raiz
Dado JSON com valor fora da lista fechada Então Err(ValorDesconhecido)
Dado dois nós com mesmo id Então Err(IdDuplicado) (invariante)
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"extrair_grafo","params":["&std::path::Path"],"return_type":"Result<Grafo, ErroAdaptador>"},{"name":"desserializar_grafo","params":["&str"],"return_type":"Result<Grafo, ErroAdaptador>"}],"types":[{"name":"ErroAdaptador","kind":"enum","members":["BinarioNaoEncontrado","FalhaSubprocesso","SubprocessoFalhou","SaidaNaoUtf8","JsonInvalido","ValorDesconhecido","IdDuplicado","IdReferenciado","DeteccaoAlvo"]}],"reexports":["diff::{ErroDiff, ler_diff}","metadata::ErroMetadata","workspace::{\n    ErroWorkspace, MembroWorkspace, NaturezaRaiz, chave_cache, enumerar_membros,\n    extrair_grafo_cacheado, natureza_raiz, versao_toolchain,\n}"]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) da fachada do adaptador. Código inalterado. | `03_infra/src/lib.rs` |
