# Prompt de Nucleação: `lente_infra::workspace` — membros + extração cacheada (L3)
Hash do Código: d4e187ca

**Camada**: L3 — Infraestrutura. Filesystem, subprocesso (`rustc`), cache em disco.
**Unidade**: `03_infra/src/workspace.rs` (crate `lente_infra`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0044-infra_cache_e_membros.md`.

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **fundação de I/O do grafo de workspace**: enumerar os crates-membro e extrair o
grafo por crate com **cache com validade conservadora**. É o que o `montar_grafo_workspace`
(L4) consome.

## Comportamento e invariantes

- `MembroWorkspace.nome` preserva `[package].name` para seleção Cargo e
  diagnóstico. Não é identidade de crate. O `Grafo.crate_name` traduz o campo
  `crate` do JSON extraído e é a fonte canônica, inclusive com `[lib].name`
  personalizado; cache e resolução preservam esse campo. Não há descoberta
  adicional nem inferência do crate a partir do nome do pacote.

- **`enumerar_membros(raiz) -> Result<Vec<MembroWorkspace>, ErroWorkspace>`** — lê
  os `Cargo.toml` direto (crate `toml`), glob por filesystem, pula sub-workspace
  virtual, respeita `exclude`. **Sem `cargo metadata`** (exclui o `lab/`).
- **`versao_toolchain(diretorio: &Path) -> Result<String, _>`** — `rustc --version --verbose` + `rustc --print sysroot`
  com `current_dir(diretorio)`, no mesmo contexto de cada extração. Respeita
  overrides por membro e usa `RUSTUP_AUTO_INSTALL=0` no subprocesso. Não altera
  cwd global. Erros de spawn, status/stderr e UTF-8 preservam o diretório no diagnóstico.
- **`chave_cache(membro, raiz, versao)`** — digest SHA-256 v2; implementação e
  domínio de validade em `infra-cache-validade.md`. O digest sozinho não autoriza
  reutilização. `CacheExtracao::avaliar` expõe os motivos de bypass.
- **`extrair_grafo_cacheado(membro, raiz, versao) -> Result<Grafo, _>`** —
  conveniência para uma rodada unitária. `CacheExtracao::extrair` compartilha
  leituras/digests entre membros. Recebe contexto Rust consultado em `membro.dir`.
  Acerto elegível lê+valida; miss ou bypass invoca o fork e valida o JSON.
  Somente miss elegível publica, atomicamente, com temporário exclusivo
  por chamada (`create_new`, PID+contador), limpo em erro.
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
Dado um diretório diferente do cwd Então rustc consulta esse diretório
Dadas duas raízes e overrides por membro Então cada versão chega à sua chave
Dadas falhas de spawn/status/UTF-8 Então há diagnóstico com contexto
Dado cache quente e entradas reutilizáveis Quando extrair_grafo_cacheado Então lê sem rodar o fork
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"natureza_raiz","params":["&Path"],"return_type":"Result<NaturezaRaiz, ErroWorkspace>"},{"name":"enumerar_membros","params":["&Path"],"return_type":"Result<Vec<MembroWorkspace>, ErroWorkspace>"},{"name":"versao_toolchain","params":["&Path"],"return_type":"Result<String, ErroWorkspace>"},{"name":"chave_cache","params":["&MembroWorkspace","&Path","&str"],"return_type":"Result<String, ErroWorkspace>"},{"name":"extrair_grafo_cacheado","params":["&MembroWorkspace","&Path","&str"],"return_type":"Result<Grafo, ErroWorkspace>"}],"types":[{"name":"MembroWorkspace","kind":"struct","members":["nome","dir"]},{"name":"ErroWorkspace","kind":"enum","members":["Io","Manifesto","Fork","Adaptador","Toolchain"]},{"name":"NaturezaRaiz","kind":"enum","members":["Crate","Workspace"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-09-26 | Cache v2 conservador e contexto Rust detalhado; ver infra-cache-validade.md. | `03_infra/src/workspace.rs` |
| 2026-09-26 | Distinguir pacote Cargo de crate canônico da extração; contrato de identidade e limites de membros com falha. | `03_infra/src/workspace.rs` |
| 2026-09-26 | Descoberta de toolchain por diretório do membro; testes controlados e diagnóstico contextual. | `03_infra/src/workspace.rs` |
| 2026-06-07 | Nucleação (migração de convenção, prompt 0064) do enum/cache de workspace. Código inalterado. | `03_infra/src/workspace.rs` |
