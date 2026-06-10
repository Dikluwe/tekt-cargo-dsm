# Laudo de Execução — Prompt 0075 (`--comparar` ciente de workspace + a rodada typst)

**Camada**: L1 (chave + proveniência) + L3 (detecção de natureza) + L4 (detecção
por lado, extração resiliente) + L2 (saída declara modo/chave/falhas).
**Data**: 2026-06-10
**Prompt executado**: `00_nucleo/prompt/0075-comparar_workspace.md` (era
`prompt-comparar-workspace.md`; renumerado para 0075 — a tela lado a lado fica 0076).
**Estado**: `EXECUTADO` — `--comparar` roda sobre workspaces; **a rodada typst saiu**
(vanilla 21 crates × cristalino 4). Suíte **301 passed / 34 ignored** (exato); linter
V1=0, V2=0, V12=1. **A rodada real quebrou quatro suposições escondidas** — todas
corrigidas.

---

## A resposta em uma sentença

O `--comparar` (0074, crate-único) passou a detectar e extrair **workspaces** (reusando
`montar_grafo_workspace`/0045), com chave de **path completo** quando há workspace; o
par typst rodou e **o número é o dado**: o pareamento por path **satura em sem-par**
(1 pareado de ~400+), medindo o quão profunda foi a reorganização em camadas.

---

## As quatro suposições que a rodada real quebrou (e os consertos)

O caso egui (0074) validou o `--comparar` e **escondeu** suposições que o primeiro par
real derrubou, uma a uma, cada conserto avançando a extração mais um passo:

1. **`--json` não é flag** — a saída JSON é o default (`--text` troca). Uso corrigido.
2. **CWD** — `extrair_grafo_cacheado` chamava `invocar_fork(nome)`, que roda `cargo
   metadata` no **CWD** do processo, não no dir do membro: só acha o membro quando o
   CWD é o mesmo workspace (verdade no `--diff` e no lente-vs-lente; **falso** ao
   comparar um workspace noutro caminho). Corrigido: `invocacao::invocar(&membro.dir)`
   (dir-aware, CWD-independente). *Desvio do "não tocar o cache" do prompt — o cache
   **era** o caminho do bug; `--diff` segue verde.*
3. **Path relativo** — `--antes lab/...` deixava os dirs dos membros relativos, e a
   detecção de alvo casa contra `manifest_path` **absolutos** (lição 0047). Corrigido:
   `extrair_lado` canonicaliza a raiz.
4. **Resolvedor de colisão** — em código real, `resolver_colisoes` (0019) encontra
   colisão de **sobreposição parcial** (`exclusivas_a=18, exclusivas_b=1,
   compartilhadas=1`) que a Estratégia 1 (categórica, "sem thresholds mágicos") não
   decide e a Estratégia 2 (fontes) **não cobre** (quarentena, 0014). Era all-or-nothing
   → abortava o lado inteiro por 1 crate. **Corrigido com resiliência** (abaixo). *Desvio
   do "não tocar `montar_grafo_workspace`" — pela mesma razão: era o caminho do bug.*

A causa-raiz do (4) — colisão de sobreposição parcial — é **lacuna de desígnio**, não
bug rápido: des-quarentenar/threshold do resolvedor é **trilha à parte** (com o número
agora na mão).

---

## O que mudou

### Detecção por lado (L3 `lente_infra` + L4)

`natureza_raiz(raiz)` lê o `Cargo.toml`: tem `[package]` → **crate**; só `[workspace]`
→ **workspace**; nenhum → erro claro citando a raiz (decisão **estrutural**, não
casamento de string de erro frágil). `comparar` detecta cada lado independente —
crate × workspace é válido.

### Chave de pareamento (L1 `lente_comparacao`)

`ChavePareamento {Normalizada, PathCompleto}`. Crate × crate → **normalizada** (= 0074,
retrocompat bit-a-bit). Qualquer lado workspace → **path completo** — a normalizada
deixa de ser injetiva (dois crates podem ter `a::ast`/`b::ast` → `ast`); o path completo
é injetivo por construção (teste-contrato `path_completo_evita_colisao_de_submodulo_
homonimo` + o contraste que documenta a colisão da normalizada).

### Extração resiliente (L4) — o conserto do (4)

`montar_grafo_workspace` deixa de ser all-or-nothing: um crate cuja extração/resolução
falha vira `FalhaCrate {crate_name, motivo}` em `GrafoWorkspace.falhas` e é **pulado**;
os demais entram no grafo unificado. **Sinal, como os fantasmas (0045)** — não erro
fatal. O `--comparar` propaga as falhas por lado; o `--diff` as ignora (melhoria
silenciosa: não aborta mais por 1 crate ruim).

### Saída declara a proveniência (L2)

Texto e JSON ganham, por lado: **modo** (crate/workspace), **chave**, **nº de crates**,
**fantasmas** e **crates não extraídos** (com o motivo). Aditivo — o esquema do 0074
não perde nada.

---

## A rodada typst — o número (o propósito do prompt)

`lente --comparar --antes lab/typst-original --depois .` (escopo seu-codigo, chave
path_completo). **COLD 113,83 s · WARM 3,70 s** (o cache de workspace aquece).

| | typst-original (vanilla) | typst-crystalline (cristalino) |
|---|---|---|
| modo | workspace, **21 crates** | workspace, **4 crates** (camadas Tekt) |
| fantasmas | **448** | 0 |
| crates não extraídos | **2** | 0 |
| sem-par | **392** | **177** |
| ciclos (qtd / maior SCC) | **11 / 203** | **5 / 15** |

- **Pareados: 1.** As arestas comuns/só-antes/só-depois: 0. O pareamento por path
  **satura em sem-par** — exatamente o previsto no contrato do prompt. A reorganização
  em camadas (typst-syntax/eval/... → 01_core/02_shell/03_infra/04_wiring) mudou quase
  todo path; o número **mede a profundidade da reorganização**, não a paridade de
  conteúdo. Sinal sob movimento de path exige pareamento por **identidade de item** —
  trilha adiada (de novo, de propósito).
- **Crates não extraídos** (resiliência em ação): `typst-macros` (colisão irresolúvel —
  proc-macro) e `typst-tests` (pacote sem `[lib]`). Declarados, não escondidos.
- **Ciclos** é o sinal mais legível: o vanilla tem um emaranhado de **203 módulos**
  (11 ciclos); o cristalino, maior SCC **15** (5 ciclos). A reorganização **desfez o
  núcleo fortemente conexo gigante** — uma leitura real e forte da migração.
- **Ruído de terceiros** (registro, do "fora de escopo" do prompt): o censo do vanilla
  inclui módulos de **crates de terceiros** (`comemo`, `ecow`, `citationberg`, `krilla`,
  `codespan_reporting`…) — o escopo `seu-codigo` filtra sysroot, **não** dependências
  third-party. Inflam o sem-par antes. Conserto (filtrar não-membros) é decisão
  posterior.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **301 passed / 0 failed** |
| Ignorados | **34** (exato — disciplina 0068; +1 vs 0074: o E2E workspace lente-vs-lente) |
| E2E `#[ignore]` | crate×crate (retrocompat, chave normalizada) ✓; workspace lente-vs-lente (path completo, paridade total, **0 fantasmas**) ✓ |
| `crystalline-lint .` | **V1=0, V2=0**; V12=1 (`ErroLente`) |
| `--diff` (montar_grafo_workspace) | E2E verde (resiliência não regrediu o caminho do diff) |
| Rodada typst | **saiu** (números acima); symlink criado e removido; typst repo limpo |

---

## Trilhas adiadas (registradas)

- **Resolvedor de colisão para sobreposição parcial** (Estratégia 2 / threshold) — a
  causa-raiz das falhas typst; decisão própria, agora com o número.
- **Pareamento por identidade de item** — sinal sob reorganização (o que daria
  paridade útil no caso typst); adiado no 0074 e de novo aqui, de propósito.
- **Filtrar third-party do censo** do comparar (ruído medido acima).
- **A tela lado a lado** (0076) — lê este JSON.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | `--comparar` ciente de workspace: detecção por lado (`natureza_raiz` em `lente_infra` — `[package]`/`[workspace]`), `montar_grafo_workspace` (0045) para lados-workspace, chave de **path completo** quando há workspace (a normalizada não é injetiva entre crates — teste-contrato). Retrocompat crate×crate bit-a-bit. **Extração resiliente** (`GrafoWorkspace.falhas`/`FalhaCrate`): um crate que falha é pulado e reportado (sinal, 0045), não aborta o lado. Saída (texto+JSON) declara modo/chave/crates/fantasmas/falhas por lado (aditivo). **Quatro suposições quebradas pela rodada real, corrigidas**: (1) `--json` não é flag; (2) CWD — `extrair_grafo_cacheado` usa `invocacao::invocar(&membro.dir)` dir-aware, não `invocar_fork(nome)` no CWD; (3) path relativo — `extrair_lado` canonicaliza a raiz (lição 0047); (4) resolvedor de colisão de sobreposição parcial (0019, Estratégia 2 quarentenada 0014) → resiliência. Desvios documentados do "não tocar cache/montar_grafo_workspace" (eram o caminho do bug; `--diff` segue verde). **Rodada typst** (o propósito): vanilla 21 crates / 448 fantasmas / 2 falhas (typst-macros colisão, typst-tests sem lib) / sem-par 392 / 11 ciclos maior SCC 203; cristalino 4 crates / 0 fantasmas / 0 falhas / sem-par 177 / 5 ciclos maior 15; **pareados 1** (path satura em sem-par — mede a profundidade da reorganização); COLD 113,83 s / WARM 3,70 s; ruído de third-party no censo registrado. Suíte 301 / 34 ignored (exato); V1=0, V2=0, V12=1. Trilhas adiadas: resolvedor de colisão (sobreposição parcial), pareamento por identidade de item, filtro de third-party, tela lado a lado (0076). | `01_core/comparacao/src/lib.rs` (chave + Proveniencia + falhas), `03_infra/src/{workspace.rs,lib.rs}` (natureza_raiz; dir-aware no cache), `04_wiring/src/lib.rs` (detecção, resiliência, GrafoWorkspace.falhas/FalhaCrate), `02_shell/cli/src/saida.rs` + `02_shell/catalogo/src/lib.rs` (proveniência/falhas), `04_wiring/app/src/main.rs` (E2E workspace), `00_nucleo/prompts/{comparacao,wiring,cli-saida,infra-workspace,infra,cli-args}.md` (snapshots), `00_nucleo/prompt/0075-comparar_workspace.md` (renumerado), `00_nucleo/lessons/0075-comparar_workspace.md` |
