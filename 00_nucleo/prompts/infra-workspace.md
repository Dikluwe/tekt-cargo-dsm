# Prompt de Nucleação: `lente_infra::workspace` — membros + extração cacheada (L3)
Hash do Código: e3a86698

**Camada**: L3 — Infraestrutura. Filesystem, subprocesso (`rustc`), cache em disco.
**Unidade**: `03_infra/src/workspace.rs` (crate `lente_infra`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0044-infra_cache_e_membros.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **fundação de I/O do grafo de workspace**: enumerar os crates-membro e extrair o
grafo por crate com **cache de chave completa**. É o que o `montar_grafo_workspace`
(L4) consome.

## Comportamento e invariantes

- **`enumerar_membros(raiz) -> Result<Vec<MembroWorkspace>, ErroWorkspace>`** — lê
  os `Cargo.toml` direto (crate `toml`), glob por filesystem, pula sub-workspace
  virtual, respeita `exclude`. **Sem `cargo metadata`** (exclui o `lab/`).
- **`versao_toolchain() -> Result<String, _>`** — `rustc --version` (subprocesso de
  rustc, não cargo).
- **`chave_cache(membro, raiz, versao)`** — SHA-256 de 4 componentes em ordem fixa
  (fontes por glob + `Cargo.toml` do membro + `Cargo.lock` do workspace + toolchain).
- **`extrair_grafo_cacheado(membro, raiz, versao) -> Result<Grafo, _>`** — acerto
  lê+desserializa; erro roda o fork, **grava atômico** (temp+rename), desserializa.
- **`MembroWorkspace`** (`nome`/`dir`), **`ErroWorkspace`** (`Io`/`Manifesto`/
  `Fork`/`Adaptador`/`Toolchain`; `Display`+`Error`+`source`) — erro agregado pelo
  `ErroLente` (L4). Cache em `target/lente-cache/` (gitignorado).

## Restrições (L3)

- I/O legítimo (fs/rustc/cache). Deps: `toml`+`sha2` (L3). Importa o L1; **não o L4**.

## Critérios de Verificação

```
Dado o workspace Quando enumerar_membros Então os membros (sem o lab/)
Dado a mesma fonte+manifesto+lock+toolchain Então a chave é estável
Dado um .rs novo na fonte Então a chave muda (re-extrai)
Dado cache quente Quando extrair_grafo_cacheado Então lê sem rodar o fork
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"natureza_raiz","params":["&Path"],"return_type":"Result<NaturezaRaiz, ErroWorkspace>"},{"name":"enumerar_membros","params":["&Path"],"return_type":"Result<Vec<MembroWorkspace>, ErroWorkspace>"},{"name":"versao_toolchain","params":[],"return_type":"Result<String, ErroWorkspace>"},{"name":"chave_cache","params":["&MembroWorkspace","&Path","&str"],"return_type":"Result<String, ErroWorkspace>"},{"name":"extrair_grafo_cacheado","params":["&MembroWorkspace","&Path","&str"],"return_type":"Result<Grafo, ErroWorkspace>"}],"types":[{"name":"MembroWorkspace","kind":"struct","members":["nome","dir"]},{"name":"ErroWorkspace","kind":"enum","members":["Io","Manifesto","Fork","Adaptador","Toolchain"]},{"name":"NaturezaRaiz","kind":"enum","members":["Crate","Workspace"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) do enum/cache de workspace. Código inalterado. | `03_infra/src/workspace.rs` |
