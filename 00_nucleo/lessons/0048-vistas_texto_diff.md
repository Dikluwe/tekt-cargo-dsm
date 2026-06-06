# Laudo de Execução — Prompt 0048 (as três vistas de texto do `--diff` — `--vista`)

**Camada**: L2 — Shell (`lente_cli`)
**Data**: 2026-06-06
**Prompt executado**: `00_nucleo/prompt/0048-vistas_texto_diff.md`
**Estado**: `EXECUTADO` — `--vista <resumo|item|camadas>` com três renderizadores de
texto sobre o `ResultadoDiff` (0047); ausência da flag → JSON (intocado). Suíte
verde (273 verdes + 28 ignored). **Nada** em L1/L4 mudou. Modo global e JSON
inalterados.

---

## A entrega em uma sentença

O modo `--diff` (e a trilha local) fecha: três vistas de texto — **resumo**
(impacto por crate), **item** (bloco por tocado), **camadas** (tocados agrupados
por crate + cross-crate) — como renderizadores **puros** sobre o `ResultadoDiff`,
selecionadas por `--vista`; sem a flag, `--diff` segue emitindo o JSON do 0047.

---

## O que foi adicionado (só L2)

| Arquivo | Mudança |
|---|---|
| `02_shell/cli/src/args.rs` | enum `Vista { Resumo, Item, Camadas }` + flag `--vista` (`requires = "diff"`) |
| `02_shell/cli/src/main.rs` | `run_diff`: `cli.vista` → `None` = JSON; `Some(v)` = `formatar_diff_vista` |
| `02_shell/cli/src/saida.rs` | `formatar_diff_vista` + os três renderizadores + helpers, todos puros |
| `02_shell/catalogo/src/lib.rs` | `HELP_VISTA` + 15 constantes de texto das vistas (ADR-0002) |

`ResultadoDiff` (L1), `analisar_diff` (L4) e o JSON (L2, 0047) — **intocados**.

---

## O layout de cada vista (exemplo real do repo, via o binário)

### `resumo` (`lente --diff --vista resumo`)

```
diff: 66 tocados em 9 crate(s)
  pode quebrar (montante): 160 — lente_core 59 · lente 30 · lente_wiring 26 · lente_infra 23 · …
  depende de (jusante): 90 — lente_core 27 · lente_infra 15 · std 8 · alloc 7 · clap_builder 7 · …
untracked: 2 compilados · 0 sem mod · 15 não-fonte
```

### `item` (`--vista item`, recorte)

```
66 tocados:
  lente  [Folha]
    pode quebrar: —   depende de: 10
  lente::args::Cli  [Intermediário]
    pode quebrar: 14   depende de: 4
```

### `camadas` (`--vista camadas`, recorte)

```
tocados por crate:
  lente_core
    domain::mapeamento::No... [Intermediário], entities::grafo::No [Intermediário], …
  pode quebrar, por crate: lente_core 59 · lente 30 · lente_wiring 26 · …
```

O rodapé do censo (`untracked: … · … · …`) e o realce do solto (`sem mod (não
compilado): cli_novo.rs`) são **compartilhados** pelas três; diferem em como
arranjam os tocados e o impacto.

---

## O agrupamento por crate (derivado do path)

O crate de um nó é o **1º segmento do path** (`crate_de` = `path.split("::").next()`).
Daí saem: a contagem de crates distintos (resumo), o agrupamento dos tocados
(camadas), e o impacto cross-crate (`combinado.montante` agrupado por crate). A
ordem por crate é **contagem desc, nome asc** — o mais impactante primeiro,
determinístico. Nenhuma recomputação: tudo deriva do `ResultadoDiff`.

---

## A ênfase adaptativa (como detecta diff só-arquivo-novo)

Critério (laudo 0043): `combinado.montante` **vazio** **e** `ligados` **não-vazio**
→ é um diff só de arquivo novo (montante ~vazio por natureza; o valor está no
**jusante**, o que o código novo passa a usar). Nesse caso a vista `resumo`
imprime a linha do **jusante antes** da do montante. Caso contrário (modificação),
lidera com o montante. Coberto por
`vista_resumo_enfase_adaptativa_arquivo_novo_lidera_jusante`.

---

## A vista `camadas` ficou por-crate (esperado) — **não** puxou o `lente_estrutura`

Conforme o prompt (e o cuidado "vista camadas leve"): `camadas` agrupa os tocados
pelo crate do path e mostra o cross-crate pelo `combinado.montante` agrupado —
tudo derivado dos paths, **sem** dependência do `lente_estrutura` (09). Uma vista
fiel ao layering do 09 fica para depois, se pedida. O `lente_cli` não ganhou
dependência nova.

---

## `kind` não foi preciso (a `classificacao` cobre o rótulo)

O cuidado do prompt ("se uma vista parecer precisar de `kind`, parar e registrar")
não disparou: o rótulo de cada nó nas vistas é a **`classificacao`** do raio
(`[Base]`/`[Folha]`/`[Intermediário]`/`[Isolado]`), que já está no
`ResultadoDiff`. Nenhum enriquecimento de L1/L4 foi necessário — `kind` segue
fora do dado, para um prompt futuro se as vistas o pedirem.

---

## JSON e modo global inalterados

- Sem `--vista`, `--diff` emite o **JSON** do 0047 (roteamento `None` → `formatar_diff`).
  O teste `diff_json_tem_o_esquema_e_e_desserializavel` (0047) segue verde.
- `--vista` tem `requires = "diff"` (clap): só vale com `--diff`.
- O modo **global** da CLI não foi tocado — seus testes passam.

---

## Testes (inline em `saida.rs`, sobre `ResultadoDiff` forjado — sem git/fork)

`vista_resumo_traz_contagens_por_crate_censo_e_solto`,
`vista_item_um_bloco_por_tocado_com_classificacao_e_contagens`,
`vista_camadas_agrupa_por_crate_e_mostra_cross_crate`,
`vista_resumo_enfase_adaptativa_arquivo_novo_lidera_jusante`,
`fantasma_aparece_so_se_maior_que_zero` (some quando 0, aparece quando > 0, nas
três), `solto_listado_em_todas_as_vistas`, `vistas_sao_deterministicas`,
`roteamento_vista_chama_o_renderizador_certo`. Mais a não-regressão do JSON e do
modo global.

---

## Estado da suíte / invariantes

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **273 verdes + 28 ignored, 0 falhas** (era 265+28 no 0047; +8 unit das vistas) |
| Camadas L1/L4 | **intocadas** — `ResultadoDiff`/`analisar_diff`/JSON inalterados |
| `cargo tree -p lente_cli` | **sem dep nova** — `camadas` não puxou `lente_estrutura` |
| Modo global da CLI | **inalterado** — `--vista` aditivo |
| Binário `lente --diff --vista …` | as três vistas renderizam (recortes reais acima) |

---

## O que NÃO entrou

- Uma vista `camadas` fiel ao layering do `lente_estrutura` (09) — fica para depois,
  se pedida (a versão leve por-crate cobre o caso prático).
- `kind` por nó — não está no dado; a `classificacao` cobre o rótulo.

---

## Cuidados herdados

- **Pré-requisito de compilação 0037 (`No.position`)** + toda a cadeia 0037→0047:
  segue não-commitada na working tree; este prompt (L2) sobre ela.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Fecha o modo `--diff` (e a trilha local) com as três vistas de texto, na L2, como renderizadores **puros** sobre o `ResultadoDiff` (0047): **resumo** (contagem de tocados/crates, montante/jusante por crate, censo, solto listado; ênfase adaptativa do 0043 — arquivo novo `[montante vazio + ligados]` lidera com o jusante, modificação com o montante), **item** (um bloco por tocado: path + `classificacao` do raio + contagens, `—` quando 0), **camadas** (tocados agrupados pelo crate do path + cross-crate via `combinado.montante` agrupado — versão leve, **sem** puxar o `lente_estrutura`). Flag `--vista <resumo\|item\|camadas>` (`requires=diff`); **ausente → JSON** (0047, intocado). Crate de um nó = 1º segmento do path; ordem por contagem desc, nome asc (determinística). `kind` não foi preciso (a `classificacao` cobre o rótulo — nada em L1/L4 mudou). Strings no catálogo (ADR-0002). Modo global e JSON inalterados. Testes sobre `ResultadoDiff` forjado (sem git/fork): as três vistas, roteamento, ênfase adaptativa, solto listado, fantasma condicional, determinismo, não-regressão. Suíte 265→273 verdes + 28 ignored. | `02_shell/cli/src/{args.rs,main.rs,saida.rs}`, `02_shell/catalogo/src/lib.rs`, `00_nucleo/lessons/0048-vistas_texto_diff.md` |
