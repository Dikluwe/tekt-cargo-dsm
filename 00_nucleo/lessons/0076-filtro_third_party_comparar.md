# Laudo de Execução — Prompt 0076 (censo do `--comparar` só com membros — filtro de third-party)

**Camada**: L1 (`lente_filtro`: o filtro puro) + L4 (aplicação no `extrair_lado`) +
L2 (saída declara a contagem filtrada).
**Data**: 2026-06-10
**Prompt executado**: `00_nucleo/prompt/0076-filtro_third_party_comparar.md` (era
`prompt-filtro-third-party-comparar.md`; renumerado 0076 — a tela lado a lado vira 0077).
**Estado**: `EXECUTADO` — `filtrar_nao_membros` aplicado no lado-workspace do comparar
em escopo seu-codigo; a rodada typst re-executada saiu com o censo **só de membros**.
Suíte **305 passed / 34 ignored** (exato); linter V1=0, V2=0, V12=1.

---

## A resposta em uma sentença

Num lado-workspace, "seu código" são os **membros** — o `--comparar` passou a filtrar
também as **dependências third-party** (não só sysroot), e o censo do typst vanilla
limpou de `comemo`/`ecow`/`krilla`/… para os módulos próprios do typst, deixando o
número honesto para a trilha de pareamento por identidade de item.

---

## Fase 1 — decisões e achados

1. **Onde o censo conta**: `EstruturaModulos.modulos` (módulos `Mod`/`Crate` do
   agregado). O filtro entra **antes** de `estrutura_de_grafo`, no grafo unido.
2. **Normalização hífen↔underscore**: os nomes de membro vêm do `Cargo.toml` com
   hífen (`typst-macros`); os paths do fork usam underscore (`typst_macros`).
   **Não havia norm compartilhada** (a união 0045 compara `crate_name` direto, com a
   mesma lacuna) — normalizo `'-'→'_'` dentro do filtro. Convenção registrada.
3. **Morada**: `lente_filtro` (irmã de `filtrar_stdlib`, o modelo exato — entrada
   `&Grafo`, sem conversão). `filtrar_nao_membros(grafo, &[membros])`.
4. **Fantasmas no censo (achado, sem consertar)**: a união (0045) confirma — o
   representante de fantasma tem **dono-membro** (1º segmento = membro), então **fica**;
   só os representantes **externos** (não-membro: `comemo`/`ecow`) saem. Os 448
   fantasmas do vanilla **permanecem** (count inalterado 0075→0076) — são módulos
   member-owned e **entram** na contagem de sem-par. É o dado que a trilha do resolvedor
   de colisão precisa; não consertado aqui.
5. **Limite 2 seguro por construção** (citado, não re-medido — laudos 0025/0027): o fork
   nomeia o impl-do-alvo pelo lado do alvo (`lente_core::…::fmt`, não `core::…::fmt`),
   então um impl de membro para trait externo tem path de membro e **fica**. O mesmo
   argumento que tornou o filtro de sysroot seguro vale aqui.

---

## O que mudou

- **L1 `lente_filtro`**: `filtrar_nao_membros(grafo, membros) -> Grafo` — nó fica se o
  1º segmento (normalizado) ∈ membros; senão sai com as arestas que o tocam (0 soltas);
  `crate_name`/`id` preservados; determinístico/idempotente. 4 testes (remoção+arestas,
  norm hífen↔_, fantasma-membro preservado, idempotência).
- **L4 `extrair_lado`** (lado-workspace, escopo seu-codigo): aplica `filtrar_stdlib`
  (sysroot) **e depois** `filtrar_nao_membros` (third-party) antes do censo; conta os
  nós removidos pelo segundo (= third-party, já sem sysroot). Lado-crate e escopo
  completo **intocados**.
- **L2**: a proveniência (0075) ganha `third_party_antes/depois` (nº de nós); texto e
  JSON declaram — **filtro silencioso seria mascaramento**.

---

## A rodada typst re-executada — o delta (o propósito do prompt)

`lente --comparar --antes lab/typst-original --depois .` (seu-codigo, path_completo;
cache morno do 0075, **5,12 s**):

| | 0075 (sem filtro) | **0076 (com filtro)** |
|---|---|---|
| third-party removido — antes | — | **434 nós** (≈21 módulos) |
| third-party removido — depois | — | **40 nós** |
| sem-par antes | 392 | **371** |
| sem-par depois | 177 | **175** |
| **pareados** | 1 | **0** |
| fantasmas antes / ciclos | 448 / 11 (maior 203) | 448 / 11 (maior 203) — inalterados |

- **434 nós de third-party** saíram do censo do vanilla (typst depende de muitos crates:
  `comemo`, `ecow`, `citationberg`, `krilla`, `codespan_reporting`, …); 40 do cristalino.
  No nível de **módulo** (a unidade do censo), 21 e 2 respectivamente.
- **Pareados 1 → 0**: o único "pareado" do 0075 **era ruído third-party** (um módulo de
  dep comum aos dois lados). Com o filtro, o pareamento por path tem **sinal zero** — a
  confirmação mais forte da tese do 0075: sob reorganização em camadas, path-pairing não
  serve; o número honesto é **0**.
- A amostra de sem-par antes agora é só typst (`typst::args`, `typst::eval`,
  `typst::compile`, …) — **sem** `comemo`/`ecow`/`krilla`. Censo member-only.
- Fantasmas e ciclos **inalterados** — o filtro não os toca (member-owned), como
  projetado.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **305 passed / 0 failed** (301 + 4 filtro) |
| Ignorados | **34** (exato — disciplina 0068; sem novo ignorado) |
| `filtrar_nao_membros` | 4 testes (remoção+arestas, hífen↔_, fantasma preservado, idempotência) |
| Crate×crate (0074/0075) | intocado (E2E retrocompat verde) |
| `--diff`/`--estrutura`/filtro sysroot/união | intocados |
| `crystalline-lint .` | **V1=0, V2=0**; V12=1 (`ErroLente`) |
| Rodada typst | saiu (delta acima); symlink criado e removido; typst repo limpo |

---

## Trilhas adiadas (registradas)

- **Pareamento por identidade de item** — agora com o censo honesto (member-only, 0
  pareados por path), é o próximo da fila: dar sinal sob reorganização.
- **Resolvedor de colisão / 448 fantasmas** — trilha própria; este laudo só registra que
  os representantes member-owned entram no sem-par (o dado que ela precisa).
- **Filtro de third-party no `--diff`/`--estrutura`** — sem uso declarado; decisão
  posterior.
- **A tela lado a lado** (0077) — lê este JSON.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Censo do `--comparar` em lado-workspace passa a conter **só membros**: `filtrar_nao_membros` (L1 `lente_filtro`, puro — 1º segmento do path normalizado `'-'→'_'` vs nomes-membro; remove nó+arestas; fantasmas member-owned ficam; Limite 2 seguro por construção, citado de 0025/0027), aplicado no `extrair_lado` (lado-workspace, escopo seu-codigo) **antes** do censo, após `filtrar_stdlib` (sysroot). Proveniência (0075) declara `third_party_antes/depois` (nº de nós) no texto e JSON — filtro visível, não mascarado. Crate×crate (0074/0075), `--diff`, `--estrutura`, filtro de sysroot e união (0045) intocados. **Rodada typst re-executada** (cache morno, 5,12 s): third-party removido vanilla **434 nós** (≈21 módulos) / cristalino 40; sem-par **392→371** (antes) e **177→175** (depois); **pareados 1→0** (o "1" do 0075 era ruído third-party — path-pairing tem sinal zero sob a reorganização); fantasmas (448) e ciclos (11/203 vs 5/15) inalterados. Achado registrado: representantes de fantasma são member-owned, ficam, e os module-level entram no sem-par (dado p/ a trilha do resolvedor). Suíte 305 / 34 ignored (exato); V1=0, V2=0, V12=1. Numeração: este 0076; tela lado a lado → 0077. | `01_core/filtro/src/lib.rs` (filtrar_nao_membros + 4 testes), `04_wiring/src/lib.rs` (aplicação no extrair_lado + LadoExtraido.third_party + Proveniencia), `01_core/comparacao/src/lib.rs` (Proveniencia.third_party_*), `02_shell/cli/src/saida.rs` + `02_shell/catalogo/src/lib.rs` (declaração), `00_nucleo/prompts/{filtro,comparacao,cli-saida}.md` (snapshots), `00_nucleo/prompt/0076-filtro_third_party_comparar.md` (renumerado), `00_nucleo/lessons/0076-filtro_third_party_comparar.md` |
