# Laudo de Execução — Prompt 0044 (`lente_infra` — extração cacheada com chave completa + enumeração de membros)

**Camada**: L3 — Infraestrutura (`lente_infra`)
**Data**: 2026-06-05
**Prompt executado**: `00_nucleo/prompt/0044-infra_cache_e_membros.md`
**Estado**: `EXECUTADO` — `enumerar_membros`, `versao_toolchain`, `chave_cache`
e `extrair_grafo_cacheado` nucleados em produção, com chave completa, cache
gitignorado e gravação atômica. Aditivo: `extrair_grafo` e o fork inalterados.
Suíte verde (235 verdes + 24 ignored no workspace; os 2 E2E novos de cache
passam com o fork real).

---

## A entrega em uma sentença

O `lente_infra` ganhou a fundação de I/O do grafo de workspace — enumeração de
membros lendo `Cargo.toml` direto (sem `cargo metadata`) e extração por crate
com cache de **chave completa** (fontes + `Cargo.toml` do membro + `Cargo.lock`
do workspace + versão do toolchain) — fechando a limitação registrada no laudo
0040 (a chave não pegava `Cargo.toml`), sem tocar `extrair_grafo`, o fork, nem
`lente_core`.

---

## O que foi adicionado (`03_infra/src/workspace.rs`, módulo novo)

| Item | Assinatura | Nota |
|---|---|---|
| `MembroWorkspace` | `{ nome: String, dir: PathBuf }` | nome de `[package].name` (≠ nome do diretório) |
| `ErroWorkspace` | enum | `Io`, `Manifesto`, `Fork`, `Adaptador`, `Toolchain`; `Display` + `Error` (com `source`) |
| `enumerar_membros` | `(raiz) -> Result<Vec<MembroWorkspace>, _>` | parse direto do TOML, glob por filesystem, pula virtual, respeita `exclude` |
| `versao_toolchain` | `() -> Result<String, _>` | `rustc --version` (subprocesso de **rustc**, não cargo) |
| `chave_cache` | `(membro, raiz, versao_toolchain) -> Result<String, _>` | SHA-256, 4 componentes em ordem fixa; isolável p/ testar invalidação |
| `extrair_grafo_cacheado` | `(membro, raiz, versao_toolchain) -> Result<Grafo, _>` | acerto = lê+desserializa; erro = fork+grava atômico+desserializa |

Re-exportados no `lib.rs` (`pub use workspace::{…}`). `extrair_grafo`,
`desserializar_grafo`, `fork::*` intocados.

---

## Decisões registradas (itens pedidos no prompt)

### Diretório do cache: `raiz/target/lente-cache/`

Sob `target/`, que o `.gitignore` da raiz já ignora (`target/` sem barra
inicial pega raiz e subcrates). **Nenhuma mudança no `.gitignore`** — confirmado
por `git check-ignore target/lente-cache/`. O cache não vai para o repositório.

### Composição da chave e ordem (fixa)

SHA-256 sobre, com rótulo de domínio separando cada bloco:

1. `FONTES` — `dir/src/**.rs`, ordenados por caminho relativo; para cada:
   `rel \0 len \0 conteúdo \0`. (Glob de filesystem — decisão 0043.)
2. `CARGO_TOML` — `len \0 conteúdo \0` do `Cargo.toml` do membro. **O que
   faltava no 0040.**
3. `CARGO_LOCK` — `len \0 conteúdo \0` do `raiz/Cargo.lock` (ausente → vazio,
   determinístico). Conservador: muda o lock → invalida **todos** os membros.
4. `TOOLCHAIN` — a string da versão.

Ordem fixa e separadores garantem que a chave não varie à toa e que
componentes adjacentes não se confundam.

### De onde vem a versão do toolchain

`rustc --version` (ex.: `rustc 1.92.0 (ded5c06cf 2025-12-08)`). Não há
`rust-toolchain.toml` neste projeto; se houvesse, o shim do rustup já
resolveria o `rustc` pinado, então `rustc --version` continua correto. Uma
chamada por rodada (o L4 consulta uma vez e passa adiante) — por isso a versão
é **parâmetro** da extração, não consulta interna por membro.

### Método de enumeração: parse direto, **sem `cargo metadata`**

`enumerar_membros` lê `raiz/Cargo.toml` (`[workspace].members`/`exclude`) e cada
`membro/Cargo.toml` (`[package].name`) via o crate `toml`. **Nenhum subprocesso
do cargo.** Confirmado por grep:

```
03_infra/src/fork.rs:117      Command::new("cargo")   # export-json (existente)
03_infra/src/metadata.rs:170  Command::new("cargo")   # cargo metadata (existente)
03_infra/src/workspace.rs:240 Command::new("rustc")   # versao_toolchain (novo, rustc)
```

Só os dois `cargo` já existentes + o `rustc` novo. **Nenhum subprocesso novo do
cargo** entrou — invariante preservado.

### Confirmação: `lab/` não é listado

`enumerar_membros(raiz_principal)` lê só `members` do `Cargo.toml` principal;
o `lab/` tem `[workspace]` próprio e não está em `members`, logo não aparece.
Coberto pelo teste `enumera_workspace_principal_sem_lab` (lê arquivos, sem
subprocesso, não-ignorado): afirma que `lente_core`/`lente_infra` estão e que
nenhum membro vem de `lab/` nem tem nome `proto*`.

### Deps novas

- **`toml = "0.8"`** — parse robusto dos `Cargo.toml` (arrays multilinha,
  comentários, globs como string). Hand-parse seria frágil; o prompt permite a
  dep. Casa com o `serde` já presente.
- **`sha2 = "0.10"`** — hash **estável entre execuções** (a `DefaultHasher` da
  stdlib usa seed aleatório por processo — imprópria para chave persistida).
  Mesma escolha validada na Arena (laudo 0040). Ambas estavam no cache do cargo
  (build `--offline` resolveu sem rede).

### Re-extração espúria do arquivo solto (0043): **aceita, não corrigida**

A enumeração de fontes da chave é por **glob de filesystem** (`dir/src/**.rs`),
de propósito. Um `.rs` novo (mesmo solto, sem `mod`) muda a chave → re-extrai.
Decisão 0043 mantida; **não** foi "corrigida" aqui. Coberto pelo teste
`chave_muda_com_novo_arquivo_fonte`.

---

## Gravação atômica e concorrência

`escrever_atomico`: grava num temp irmão (`.<nome>.<pid>.tmp`) e `rename` para o
destino — atômico no mesmo diretório; o cache nunca tem entrada parcial se o
processo morrer no meio. Teste `escrita_atomica_grava_e_nao_deixa_temp` confirma
escrita correta e ausência de `.tmp` remanescente. Concorrência: uso reativo
(uma rodada por vez); duas rodadas no mesmo crate são last-write-wins inofensivo
(mesma chave ⇒ mesmo conteúdo) — **aceito**, conforme o prompt.

---

## Testes (inline, `#[cfg(test)] mod tests`)

**Sem fork (rápidos, não-ignorados):**
- Enumeração: `enumera_membros_diretos_e_glob_pulando_virtual` (member direto +
  `crates/*` glob + virtual pulado, ordem determinística),
  `exclude_remove_membro_casado`, `casa_curinga_basico`,
  `enumera_workspace_principal_sem_lab` (não-regressão de layout real).
- Toolchain: `versao_toolchain_contem_rustc`.
- Chave estável e **invalidação por cada componente**: `chave_estavel…`,
  `chave_muda_com_fonte`, `chave_muda_com_novo_arquivo_fonte`,
  `chave_muda_com_cargo_toml_do_membro`, `chave_muda_com_cargo_lock_do_workspace`,
  `chave_muda_com_toolchain`.
- Acerto **sem fork** (pré-gravado): `cache_hit_le_sem_rodar_fork` (nome de
  membro fictício; se o fork rodasse, falharia → o Ok prova que leu o cache).
- Borda: `cache_hit_json_corrompido_propaga_erro_de_traducao` (cache não é
  auto-curado em silêncio).
- `display_cobre_variantes_de_erro_workspace`.

**Com fork (`#[ignore]`, padrão dos E2E do crate) — rodados e verdes:**
- `e2e_cache_miss_roda_fork_e_grava` — extrai `lente_core`, grava o cache.
- `e2e_cache_transparente_miss_depois_hit` — miss e hit devolvem `Grafo`s
  iguais (`g1 == g2`).

```
cargo test -p lente_infra -- --ignored workspace::tests::e2e
  → 2 passed; 0 failed (3.23s — extração real do fork)
```

---

## Estado da suíte / invariantes

| Item | Resultado |
|------|-----------|
| `cargo test --workspace --offline` | **235 verdes + 24 ignored, 0 falhas** (era 220+22 no laudo 0042; +17 verdes/+2 ignored novos do `workspace` + ajustes de outras modificações em curso) |
| Subprocesso novo do cargo | **Nenhum** — só `rustc` (novo) e os dois `cargo` existentes |
| `cargo tree -p lente_core` | só o crate — **pureza L1 intacta** (não tocado) |
| `extrair_grafo` / fork / tradução | **inalterados** (aditivo) |
| Cache no repositório | **não** — `target/lente-cache/` gitignorado |

---

## O que NÃO entrou (conforme o prompt)

- A **união** (L1) e a **orquestração** (L4, `lente_wiring`) que montam o grafo
  de workspace — ficam para o 0045.
- A **resolução** de colisões — segue na fiação (correta após 0042); o 0045 a
  reusa.
- Invalidação fina por dependência (o `Cargo.lock` invalida todos —
  conservador-correto).

---

## Para a próxima rodada (0045)

| Item | Estado |
|---|---|
| Enumeração de membros (sem cargo metadata) | **Coberto** |
| `versao_toolchain` via rustc | **Coberto** |
| Chave completa + invalidação | **Coberto** (4 componentes, testados 1 a 1) |
| Extração cacheada (acerto/erro/atômico) | **Coberto** |
| União por path (L1) + orquestração (L4) | **Aberto** — 0045 |
| Cache key inclui `Cargo.toml` | **Fechado** (era pendência do 0040) |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | `lente_infra` (L3) ganha a fundação de I/O do grafo de workspace: `enumerar_membros` (parse direto dos `Cargo.toml` via crate `toml`, glob por filesystem, pula sub-workspace virtual, respeita `exclude`, **sem `cargo metadata`** — exclui o `lab/`), `versao_toolchain` (via `rustc`, não cargo), `chave_cache` (SHA-256 de fontes + `Cargo.toml` do membro + `Cargo.lock` do workspace + toolchain, ordem fixa — fecha a limitação do 0040) e `extrair_grafo_cacheado` (acerto lê+desserializa; erro roda o fork, grava atômico, desserializa). Cache em `target/lente-cache/` (gitignorado), gravação atômica (temp+rename). Deps novas: `toml` (parse) e `sha2` (hash estável). Aditivo: `extrair_grafo` e o fork inalterados; `lente_core` intocado. Re-extração espúria do solto (0043) aceita, não corrigida. Testes inline: enumeração, chave + invalidação por cada componente (sem fork), acerto sem fork (pré-gravado), borda de cache corrompido, escrita atômica, E2E com fork (`#[ignore]`, verdes). Suíte 235 verdes + 24 ignored, 0 falhas. | `03_infra/src/{workspace.rs (novo),lib.rs}`, `03_infra/Cargo.toml`, `Cargo.lock`, `00_nucleo/lessons/0044-infra_cache_e_membros.md` |
