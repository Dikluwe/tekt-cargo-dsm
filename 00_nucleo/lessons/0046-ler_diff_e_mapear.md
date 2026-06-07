# Laudo de Execução — Prompt 0046 (`ler_diff` (L3) + `mapear_diff` (L1) — entrada e núcleo do modo `--diff`)

**Camada**: L3 — Infra (`lente_infra`, ler o diff) + L1 — Núcleo (`lente_core`, o mapeamento)
**Data**: 2026-06-06
**Prompt executado**: `00_nucleo/prompt/0046-ler_diff_e_mapear.md`
**Estado**: `EXECUTADO` — `mapear_diff` (L1, pura) + a forma do diff nucleados em
`01_core/src/domain/mapeamento.rs`; `ler_diff` + `parse_diff` + `invocar_git` +
`ErroDiff` em `03_infra/src/diff.rs`. Suíte verde (258 verdes + 27 ignored; o E2E
novo de `ler_diff` passa com git real). Pureza L1 intacta. Aditivo.

---

## A entrega em uma sentença

A entrada e o núcleo do modo `--diff` estão de pé: `ler_diff` (L3) lê o quadro de
trabalho inteiro — rastreados (`git diff HEAD`) **e** untracked (`git ls-files`,
hunk sintético "tudo adicionado") — via uma primitiva única `invocar_git`, com
caminhos normalizados a absoluto; `mapear_diff` (L1, pura) casa as faixas com as
`position` dos nós (reconciliação relativo↔absoluto do 0038) e faz o censo do
untracked nos 3 baldes do 0043 (ligado / solto / não-fonte).

---

## A decisão estrutural: a forma do diff é L1, não L3

O prompt agrupa os tipos do diff (`DiffEstruturado`/`ArquivoDiff`/`OrigemArquivo`/
`FaixaLinhas`) sob a **Parte 1 — L3**, mas a assinatura `mapear_diff(diff:
&DiffEstruturado, …)` é **L1 e pura** (restrição dura, restada no prompt: *"só
stdlib, sem deps novas"*). Se a forma do diff morasse no `lente_infra` (L3),
`mapear_diff` importaria L3 — **inversão de dependência de camada** (L1 não pode
depender de L3).

Resolução (a mesma do `Grafo`): a **forma** é dado puro → vai para L1; o **leitor**
que a materializa do git é I/O → fica em L3. `lente_infra` já depende de
`lente_core` (usa `Grafo`), então L3 construir a `DiffEstruturado` de L1 é a direção
correta. `mapear_diff` consome só dados L1 e permanece puro
(`cargo tree -p lente_core` segue só o crate). Mesmo espírito da correção do 0045:
a restrição dura manda sobre o agrupamento narrativo do prompt.

---

## O que foi adicionado

### L1 — `01_core/src/domain/mapeamento.rs` (módulo novo, puro)

| Item | Assinatura | Nota |
|---|---|---|
| `OrigemArquivo` | enum `Rastreado` / `NaoRastreado` | de onde veio o arquivo |
| `FaixaLinhas` | `{ inicio: u32, fim: u32 }` | lado novo, 1-based inclusiva |
| `ArquivoDiff` | `{ caminho: PathBuf, origem, linhas_alteradas }` | `caminho` absoluto quando de `ler_diff` |
| `DiffEstruturado` | `{ arquivos: Vec<ArquivoDiff> }` | o diff inteiro |
| `NoTocado` | `{ id: usize, path: Path }` | mínimo (o raio é enriquecimento do 0047) |
| `MapeamentoDiff` | `{ tocados, ligados, soltos, nao_fonte }` | nós tocados + censo |
| `mapear_diff` | `(&DiffEstruturado, &Grafo, &[PathBuf]) -> MapeamentoDiff` | puro, determinístico |

Registrado em `domain/mod.rs` (`pub mod mapeamento;`).

### L3 — `03_infra/src/diff.rs` (módulo novo)

| Item | Assinatura | Nota |
|---|---|---|
| `ErroDiff` | enum `Git{codigo,stderr}` / `Parse(String)` / `Io(io::Error)` | `Display` + `Error` (com `source`) |
| `invocar_git` | `(&[&str], &Path) -> Result<String, ErroDiff>` | **primitiva única de git** |
| `parse_diff` | `(&str) -> Vec<ArquivoDiff>` | **pura**, faixas do lado novo |
| `ler_diff` | `(&Path) -> Result<DiffEstruturado, ErroDiff>` | rastreados + untracked, normaliza absoluto |
| `contar_linhas` | `(&Path) -> Result<u32, ErroDiff>` | por bytes (`\n`), tolerante a binário |

Re-exportado no `lib.rs` (`pub use diff::{ErroDiff, ler_diff};`). `parse_diff` é
privada (testada inline). Nada existente mudou — **aditivo**.

---

## Reconciliação de caminho (o pulo do gato, laudo 0038)

Os caminhos do diff são relativos ao repo; as `position.file` são **absolutas**
(0037). `arquivo_casa(pos_file, caminho)` reconcilia: casa por **igualdade** ou
por **sufixo em fronteira de segmento**. Endurecimento sobre o protótipo (que
usava `ends_with` cru): exige que o caractere antes do sufixo seja `/` (ou vazio),
para `"/repo/a/outro_lib.rs"` **não** casar `"lib.rs"`. Coberto por
`sufixo_nao_casa_em_fronteira_parcial` e `reconciliacao_relativo_do_diff_casa_position_absoluta`.

`ler_diff` normaliza para absoluto (`raiz.join(relativo)`); a reconciliação por
sufixo cobre o caso de caminho relativo (testes, e robustez a `position` não
canonicalizada).

---

## A limitação de deleção (registrada, não tratada)

Um hunk só de `-linhas` (deleção pura, `@@ -10,3 +9,0 @@`) tem `d == 0` no lado
novo → `parse_diff` **não** gera faixa. Um arquivo todo removido vem como
`+++ /dev/null` → ignorado. O nó deletado já não está no grafo pós-mudança; mapear
contra o grafo atual vê adições/modificações, não deleções. Coberto por
`parse_diff_delecao_pura_nao_gera_faixa` e `parse_diff_arquivo_removido_dev_null_e_ignorado`.

---

## O censo do untracked (os 3 baldes do 0043) — confirmado com casos

`mapear_diff` separa cada untracked:

| balde | critério | teste |
|---|---|---|
| `ligados` | caminho está nas `position.file` do grafo (o cargo compilou) | `untracked_no_grafo_vai_para_ligados_e_seus_nos_para_tocados` |
| `soltos` | `.rs` dentro de um `membros_dirs` mas **fora** do grafo (sem `mod`) | `untracked_rs_em_membro_fora_do_grafo_vai_para_soltos` |
| `nao_fonte` | fora de qualquer membro, ou não-`.rs` | `untracked_fora_de_membro_ou_nao_rs_vai_para_nao_fonte` |

Os nós de um untracked **ligado** entram em `tocados` via o hunk sintético `[1, n]`
(o teste confirma). Um **solto** não está no grafo → não toca nada (sinal acionável,
não erro). O critério de "ligado" é a interseção com as `position.file` do grafo —
a verdade-de-campo barata do 0043 (não exige segunda consulta ao cargo).

---

## `git` ≠ `cargo` (o invariante preservado)

O invariante "dois subprocessos do **cargo**" (0018/0023) é sobre o cargo. O `git`
é outra ferramenta — seus subprocessos são distintos e **não** o violam. Por
higiene, **uma primitiva única de git** (`invocar_git`), como o `fork::invocar_em`
é a única do cargo. Confirmação por grep:

```
03_infra/src/diff.rs:    Command::new("git")    # invocar_git (novo, único de git)
03_infra/src/fork.rs:    Command::new("cargo")  # export-json (existente)
03_infra/src/metadata.rs: Command::new("cargo") # cargo metadata (existente)
03_infra/src/workspace.rs: Command::new("rustc")# versao_toolchain (0044)
```

Os dois `cargo` seguem únicos; o `git` novo é um `Command::new("git")` só.

---

## Testes

**L1 `mapeamento` (inline, puro — 10, sem git/fork):** tocado por cruzamento;
módulo-arquivo que abrange a faixa junto com a `fn`; fora da faixa não toca;
reconciliação relativo↔absoluto; sufixo de fronteira parcial não casa; os 3 baldes
do censo; determinismo (`mapear_diff` 2× ⇒ `==`); nó sem `position` nunca toca.

**L3 `diff` (inline — 6 + 1 `#[ignore]`):** `parse_diff` (faixas do lado novo; hunk
sem vírgula = 1 linha; deleção pura sem faixa; `/dev/null` ignorado; múltiplos
hunks/arquivos); `Display` do `ErroDiff`; E2E `ler_diff` (`#[ignore]`, git real):

```
cargo test -p lente_infra e2e_ler_diff -- --ignored
  → repo temp: rastreado a.txt (faixa 2..2) + untracked novo.rs (hunk 1..2),
    caminhos absolutos → 1 passed
```

---

## Estado da suíte / invariantes

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **258 verdes + 27 ignored, 0 falhas** (era 242+26 no 0045; +16 unit, +1 E2E) |
| `cargo tree -p lente_core` | só o crate — **pureza L1 intacta** (`mapear_diff` só stdlib) |
| Deps novas | **nenhuma** — L1 e L3 só stdlib + o que já havia |
| Subprocesso de cargo | **nenhum novo** — só um `git` novo (distinto do invariante) |
| Funções existentes | **inalteradas** — `ler_diff`/`mapear_diff` são aditivas |

---

## O que NÃO entrou (conforme o prompt)

- A **orquestração** (L4) que liga `montar_grafo_workspace` (0045) +
  `enumerar_membros` (0044, dá os `membros_dirs`) + `ler_diff` + `mapear_diff`, e a
  **CLI + formatação** (L2) — ficam para o **0047**.
- O **enriquecimento** do `NoTocado` com raio/jusante (o protótipo 0043 tinha) —
  é trabalho da orquestração 0047; aqui `NoTocado` é mínimo (`id` + `path`).

---

## Cuidados herdados (do prompt) e seu estado

- **Pré-requisito de compilação 0037 (`No.position`)**: `mapear_diff` lê
  `No.position`; o campo segue **não-commitado** na working tree (do prompt 0037),
  como no 0045 — na ordem de pouso, 0037 precede 0045 e 0046.
- **Crate fora do repo / symlinks no `position.file`** (bug latente do 0038): não
  exercitado; a reconciliação por sufixo de segmento é mais robusta que o
  `ends_with` cru, mas não canonicaliza symlinks. Registrado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Entrada + núcleo do modo `--diff`. **L1**: `mapear_diff` (`01_core/src/domain/mapeamento.rs`, pura) devolve `tocados` (nós cuja `position` cruza uma faixa — item + módulo-arquivo, dedup por id) e o censo do untracked em 3 baldes (`ligados`/`soltos`/`nao_fonte`, laudo 0043); reconciliação relativo↔absoluto por sufixo de **fronteira de segmento** (endurece o `ends_with` cru do protótipo); `DiffEstruturado`/`ArquivoDiff`/`OrigemArquivo`/`FaixaLinhas`/`NoTocado`/`MapeamentoDiff`. **Decisão estrutural**: a forma do diff é **L1** (dado puro, como o `Grafo`), não L3 — senão `mapear_diff` importaria L3 e inverteria a camada; o prompt agrupava os tipos sob L3 por narrativa, mas a pureza (restrição dura) manda. **L3**: `ler_diff` (`03_infra/src/diff.rs`) lê `git diff HEAD --unified=0` (rastreados) + `git ls-files --others --exclude-standard` (untracked, hunk sintético "tudo adicionado", `contar_linhas` por bytes) via **primitiva única `invocar_git`**; `parse_diff` puro extrai as faixas do lado novo (deleção pura não gera faixa — documentado); caminhos normalizados a absoluto; `ErroDiff` (`Git`/`Parse`/`Io`). Aditivo: nada existente mudou. Pureza L1 (`cargo tree -p lente_core` só o crate); sem deps novas; `git` ≠ invariante do cargo. Testes: `mapear_diff` (10 unit), `parse_diff` (5 unit) + `Display`, `ler_diff` (`#[ignore]`, git real). Suíte 258 verdes + 27 ignored. Orquestração L4 + CLI L2 ficam para o 0047. | `01_core/src/domain/{mapeamento.rs (novo),mod.rs}`, `03_infra/src/{diff.rs (novo),lib.rs}`, `00_nucleo/lessons/0046-ler_diff_e_mapear.md` |
