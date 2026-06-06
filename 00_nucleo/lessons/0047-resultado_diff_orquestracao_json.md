# Laudo de Execução — Prompt 0047 (`ResultadoDiff` (L1) + `analisar_diff` (L4) + CLI `--diff` → JSON (L2))

**Camada**: L1 — Núcleo (`lente_core`) + L4 — Fiação (`lente_wiring`) + L2 — Shell (`lente_cli`)
**Data**: 2026-06-06
**Prompt executado**: `00_nucleo/prompt/0047-resultado_diff_orquestracao_json.md`
**Estado**: `EXECUTADO` — `ResultadoDiff` + `combinar_raios` (L1, puros, sem `serde`);
`analisar_diff` + `montar_resultado_diff` + `ErroLente::Diff` (L4); CLI `--diff` →
JSON (L2). Suíte verde (265 verdes + 28 ignored; os E2E novos passam com git+fork).
Pureza L1 intacta. Modo global inalterado.

---

## A entrega em uma sentença

O modo `--diff` está ligado ponta a ponta: `analisar_diff` (L4) sequencia
`ler_diff` (0046) + `montar_grafo_workspace` (0045) + `enumerar_membros` (0044) +
`mapear_diff` (0046) + `calcular_raio` por tocado, montando o `ResultadoDiff`
(L1, view-agnóstico, sem `serde`); a CLI `--diff` o emite em **JSON** (mapeado na
L2, padrão da trilha global) — confirmado no repo real: **69 tocados, 3 ligados,
0 soltos, 15 não-fonte, 0 fantasmas**.

---

## O que foi adicionado

### L1 — `01_core/src/domain/resultado_diff.rs` (módulo novo, puro, sem `serde`)

| Item | Forma | Nota |
|---|---|---|
| `TocadoComRaio` | `{ tocado: NoTocado, raio: Raio }` | o nó tocado + seu raio (vista por-item) |
| `RaioCombinado` | `{ montante: Vec<(Path, usize)>, jusante: … }` | união, profundidade mínima, ordenado |
| `ResultadoDiff` | `{ tocados, combinado, ligados, soltos, nao_fonte, fantasmas }` | completo, view-agnóstico |
| `combinar_raios` | `(&[Raio]) -> RaioCombinado` | helper puro |

Registrado em `domain/mod.rs`. **Sem `serde`** — a serialização é L2.

### L4 — `04_wiring/src/lib.rs`

- `analisar_diff(&Path) -> Result<ResultadoDiff, ErroLente>` — a orquestração.
- `montar_resultado_diff(&Grafo, MapeamentoDiff, Vec<Fantasma>) -> ResultadoDiff`
  — a **parte pura** (sem git/fork), extraída para teste sem I/O.
- `ErroLente::Diff(ErroDiff)` + `From`; re-exporta `ResultadoDiff`/`TocadoComRaio`/
  `RaioCombinado`/`combinar_raios`.

### L2 — `02_shell/cli/`

- `args.rs`: `--diff` (exclui os outros modos + `--grafo`/`--pacote`) e `--repo`.
- `main.rs`: `run_diff` (roteado antes de `construir_fonte` — diff não tem fonte).
- `saida.rs`: `formatar_diff` (JSON à mão com `serde_json::Map`, chaves do catálogo).
- `erro.rs`: braço `ErroLente::Diff` → `cat::ERRO_DIFF`.
- `catalogo`: `HELP_DIFF`/`HELP_REPO`/`ERRO_DIFF` + 11 chaves `JSON_*` do esquema.

---

## Onde ficou a montagem do `combinado` (e por quê)

**Helper L1 `combinar_raios`**, não inline na orquestração. Razão: é lógica pura
(união de mapas path→profundidade com o **mínimo** por path), testável sem I/O e
reusável pelas vistas do 0048. Mantê-la em L1 deixa `montar_resultado_diff` (L4)
uma composição rasa. A profundidade mínima (o caminho mais próximo) sai de graça
do acúmulo num `BTreeMap` (que ainda dá a ordenação por path — determinismo).

A **parte pura** da orquestração (`montar_resultado_diff`) foi separada do
`analisar_diff` (com git+fork), como o prompt sugeriu — o teste do miolo (raio por
tocado + combinado + censo + fantasmas) roda sem I/O; só o E2E é `#[ignore]`.

---

## O achado: a CLI default `--repo .` não casava nada (corrigido)

O smoke test do binário (`lente --diff` da raiz) revelou o bug: com `raiz`
**relativa** (`.`), `ler_diff` faz `raiz.join(relativo)` = `./01_core/src/x.rs`,
que **nunca** casa a `position.file` **absoluta** (`/home/…/01_core/src/x.rs`) —
resultado: **0 tocados**, e os untracked ligados virando **soltos** (a checagem
`sob_membro` passava, mas `esta_no_grafo` falhava pela mesma desreconciliação).

O E2E do wiring passava por usar uma `raiz` absoluta (`CARGO_MANIFEST_DIR.parent()`)
— mascarava o caso real da CLI.

Correção no lugar certo — **`analisar_diff` canonicaliza a `raiz`** logo na
entrada, antes de repassá-la aos três consumidores (`ler_diff`,
`enumerar_membros`, `mapear_diff`). Com a raiz canônica, `raiz.join(relativo)`
casa a `position.file` (absoluta/canônica do fork). Pós-fix, a CLI default bate o
E2E: **69 / 3 / 0 / 15 / 0**. Lição: o teste de unidade com raiz absoluta não
substituía exercitar o binário com o default relativo.

---

## O esquema do JSON (padrão da trilha global)

Montado **à mão** com `serde_json::Map` (como o per-nó/ranking/estrutura), chaves
do catálogo (ADR-0002):

```json
{
  "tocados":   [{"path","id","classificacao","montante","jusante"}],
  "combinado": {"montante":[{"path","profundidade"}], "jusante":[…]},
  "ligados":   ["caminho", …],
  "soltos":    ["caminho", …],
  "nao_fonte": ["caminho", …],
  "fantasmas": [{"path","referenciado_por":["crate", …]}]
}
```

Desserializável (teste `diff_json_tem_o_esquema_e_e_desserializavel`). `montante`/
`jusante` por tocado são **contagens** (transitivos); o `combinado` traz as
listas completas com profundidade.

**Omissão honesta — `kind`**: o prompt cita "path/kind" por tocado, mas o tipo L1
`NoTocado` (do 0046) carrega só `{ id, path }` — não há `kind` no dado, e a L2 não
tem o grafo para olhá-lo. O JSON emite `path` + `id` + o resumo do raio; `kind`
ficaria para um enriquecimento futuro do `NoTocado` (ou da orquestração), se as
vistas do 0048 o pedirem.

---

## A `raiz`: cwd ou `--repo`

`--diff` opera na raiz do repo: `--repo <caminho>` se dado, senão o **diretório
atual** (`.`). Não usa `--grafo`/`--pacote` (conflito declarado no clap). A
canonicalização (acima) torna o default `.` correto.

---

## O braço `ErroLente::Diff` na CLI (consequência de camada, de novo)

Como no 0045 (`Workspace`), a variante nova `ErroLente::Diff` quebraria o `match`
exaustivo de `02_shell/cli/src/erro.rs::traduzir`. Adicionado o braço +
`cat::ERRO_DIFF` ("Falha ao analisar o diff do repositório: {detalhe}"), no padrão
ADR-0002. Sem isso, não compila.

---

## Confirmação no repo real (E2E `#[ignore]`, git+fork)

```
cargo test -p lente_wiring e2e_analisar_diff -- --ignored --nocapture
  → diff: 69 tocados, 3 ligados, 0 soltos, 15 não-fonte, 0 fantasmas
  → 1 passed
```

- **3 ligados**: os `.rs` novos não-rastreados que o cargo compila —
  `mapeamento.rs` (0046), `diff.rs` (0046), `resultado_diff.rs` (0047). O censo do
  0046 os pega exatamente como "presente e compilado".
- **0 soltos** (nenhum `.rs` órfão), **15 não-fonte** (lessons/prompts/README `.md`).
- **0 fantasmas** — confirma 0045/0041 no grafo de workspace.
- Cada tocado tem o raio resolvido no próprio path (assert do E2E).

---

## Estado da suíte / invariantes

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **265 verdes + 28 ignored, 0 falhas** (era 258+27 no 0046; +7 unit, +1 E2E) |
| `cargo tree -p lente_core` | só o crate — **pureza L1 intacta** (`ResultadoDiff` sem `serde`) |
| Modo global da CLI | **inalterado** — `--diff` aditivo; testes existentes verdes |
| Deps novas na L1 | **nenhuma** (o JSON é L2, via `serde_json` que a CLI já tinha) |
| Binário `lente --diff` | emite JSON válido (smoke test: 69/3/0/15/0) |

---

## O que NÃO entrou (conforme o prompt)

- As **três vistas de texto** (A resumo / B por-item / C camadas) — são
  renderizadores sobre o `ResultadoDiff` e ficam para o **0048**, junto da flag
  `--vista` e do padrão texto-vs-JSON. Só JSON aqui.
- `kind` por tocado (não está no dado; ver omissão acima).

---

## Cuidados herdados

- **Pré-requisito de compilação 0037 (`No.position`)**: toda a cadeia 0037→0047
  depende dele; segue **não-commitado** na working tree.
- **Custo**: `calcular_raio` por tocado (BFS por nó). Para 69 tocados no repo
  real, o gargalo é a extração do grafo (~fork), não os BFS — o E2E pós-cache roda
  em ~0.24s. Sem lentidão a registrar.
- **Symlinks na raiz** (bug latente do 0038): a canonicalização resolve symlinks;
  se a `position.file` do fork não os resolvesse igual, poderia descasar. Não
  exercitado (sem symlinks neste repo).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Liga o modo `--diff` e emite o dado view-agnóstico. **L1**: `ResultadoDiff` (`01_core/src/domain/resultado_diff.rs`, puro, **sem `serde`**) agrega `tocados` (cada um com raio, vista por-item), `combinado` (união com profundidade mínima via `combinar_raios`, helper L1, vista resumo), o censo do untracked (0046) e os fantasmas (0045); `TocadoComRaio`/`RaioCombinado`. **L4**: `analisar_diff` sequencia `ler_diff` (0046) + `montar_grafo_workspace` (0045) + `enumerar_membros` (0044) + `mapear_diff` (0046) + `calcular_raio` por tocado; parte pura extraída em `montar_resultado_diff` (testável sem I/O); `ErroLente::Diff(ErroDiff)`. **Achado/correção**: `analisar_diff` **canonicaliza a raiz** — sem isso, a CLI default (`--repo .`, raiz relativa) não casava `position.file` absoluta (0 tocados, ligados virando soltos); o E2E mascarava por usar raiz absoluta. **L2**: CLI `--diff` (+ `--repo`) roda `analisar_diff` e emite o JSON (`formatar_diff`, à mão, chaves do catálogo, padrão da trilha global); braço `ErroLente::Diff` + `cat::ERRO_DIFF`; modo global inalterado. `kind` por tocado omitido (não está no `NoTocado`). Pureza L1 (`cargo tree -p lente_core` só o crate; sem dep nova na L1). Confirmado no repo real (E2E git+fork): 69 tocados, 3 ligados (os `.rs` novos), 0 soltos, 15 não-fonte, 0 fantasmas. Suíte 258→265 verdes + 28 ignored. Vistas de texto (A/B/C) ficam para o 0048. | `01_core/src/domain/{resultado_diff.rs (novo),mod.rs}`, `04_wiring/src/lib.rs`, `02_shell/cli/src/{args.rs,main.rs,saida.rs,erro.rs}`, `02_shell/catalogo/src/lib.rs`, `00_nucleo/lessons/0047-resultado_diff_orquestracao_json.md` |
