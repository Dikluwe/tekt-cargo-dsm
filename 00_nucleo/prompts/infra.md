# Prompt de Nucleação: `lente_infra` — o adaptador da fonte (L3)
Hash do Código: 870e548d

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

- ADR-0007: expõe o módulo extracao com adaptadores Rust/JSON/memória, solicitação, erro contextual e helpers de filesystem. Delegação aos invocadores/cache existentes, sem novos subprocessos Cargo.

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
- Cache v2: reexporta `CacheExtracao` e `ValidadeCache`; domínio conservador,
  compartilhamento por rodada e motivos de bypass em `infra-cache-validade.md`.
- **P0079:** `ler_mapa_correspondencia(path)` lê TOML 1.0, calcula SHA-256 dos
  bytes e traduz para o contrato puro de `lente_comparacao`. Erro de I/O,
  sintaxe ou enum desconhecido é diagnóstico de borda, nunca `Unknown` de
  pareamento.

## Restrições (L3)

Módulo público `analises` integra snapshots/regras/DSM por ferramentas externas
conforme `infra-analises.md`, sem dependência Cargo da Arena.
O módulo `scip` admite índices binários e fontes locais conforme
`infra-scip.md`.
O módulo `scip_ffi` acrescenta ligações nativas usando um ELF final,
conforme `infra-scip-ffi.md`.

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
<!-- crystalline-snapshot: {"functions":[{"name":"extrair_grafo","params":["&std::path::Path"],"return_type":"Result<Grafo, ErroAdaptador>"},{"name":"desserializar_grafo","params":["&str"],"return_type":"Result<Grafo, ErroAdaptador>"}],"types":[{"name":"ErroAdaptador","kind":"enum","members":["BinarioNaoEncontrado","FalhaSubprocesso","SubprocessoFalhou","SaidaNaoUtf8","JsonInvalido","ValorDesconhecido","IdDuplicado","IdReferenciado","DeteccaoAlvo"]}],"reexports":["cache::{CacheExtracao, ValidadeCache}","diff::{ErroDiff, ler_diff}","mapa_correspondencia::{ErroMapaCorrespondencia, ler_mapa_correspondencia}","metadata::ErroMetadata","workspace::{\n    ErroWorkspace, MembroWorkspace, NaturezaRaiz, chave_cache, enumerar_membros,\n    extrair_grafo_cacheado, natureza_raiz, versao_toolchain,\n}"]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) da fachada do adaptador. Código inalterado. | `03_infra/src/lib.rs` |

## Integração aditiva de provedores (2026-09-26)

Expõe provedores opcionais por processo, conforme ADR-0008 e infra-provedores.md; nenhum import de lab.

Instalação, protocolo e matriz verificada: `docs/adaptadores.md`. ADR-0008.

## Enriquecimento Doxygen

Exporta módulo doxygen (contrato infra-doxygen.md); projeção de catálogo/impacto
não modifica L1 nem importa crates da Arena.

O adaptador IndexStore traduz USRs e chamadas do build Swift/Clang para as fontes indexadas do núcleo.

O adaptador de chamadas Go lê apenas fontes indexadas e entrega posições para a projeção geral, independente de FFI.

O leitor de Explorar chamadas C/C++ vive em `explorar_chamadas_nativas` e
recebe base de compilação explícita para complementar um índice SCIP.

O adaptador `textfold` exporta níveis de leitura dos prompts admitidos quando
o ponto de composição o solicita.

O adaptador cobertura_cli admite o manifesto de execução da CLI conforme infra-cobertura-cli.md.
