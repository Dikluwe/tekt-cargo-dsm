# Prompt: `ResultadoDiff` (L1) + `analisar_diff` (L4) + CLI `--diff` → JSON (L2)

**Camada**: L1 — Núcleo (`lente_core`, o tipo do resultado) + L4 — Fiação
(`lente_wiring`, a orquestração) + L2 — Shell (`lente_cli`, a CLI + JSON)
**Criado em**: 2026-06-06
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0047-resultado_diff_orquestracao_json.md`)
**Decisões de origem**: a decisão do autor nesta conversa — **separar dado de
vista**: a orquestração produz um resultado completo e view-agnóstico; o JSON o
emite cru; as três vistas (A/B/C) são renderizadores sobre ele (ficam para o
0048). Laudo 0043 (os 4 itens; a ênfase se adapta — arquivo novo destaca jusante,
modificação destaca montante).
**Pré-requisitos**: 0044 (`enumerar_membros`), 0045 (`montar_grafo_workspace`),
0046 (`ler_diff`, `mapear_diff`). Pré-req de compilação: 0037 (`No.position`),
toda a cadeia 0037→0046.
**Arquivos afetados**: `01_core/src/` (o tipo `ResultadoDiff`), `04_wiring/src/lib.rs`
(orquestração), `02_shell/cli/src/` (modo `--diff` + JSON), testes.

---

## Contexto e escopo

A entrada e o núcleo do `--diff` estão prontos (0046). Falta ligar as peças e
emitir o resultado. Por decisão do autor, o resultado é **um só, completo e
view-agnóstico**: carrega tudo que as três vistas (resumo / por item / camadas)
precisam, e o **JSON** o emite cru para um visualizador decidir a vista.

**Este prompt**: o tipo `ResultadoDiff` (L1), a orquestração `analisar_diff` (L4)
que o monta, e a CLI `--diff` que emite o **JSON** (L2). As **três vistas de
texto** são renderizadores sobre o `ResultadoDiff` e ficam para o **0048**.

---

## Restrições estruturais (atenção à camada — três correções já ocorreram)

- **L1 — `ResultadoDiff` é dado puro**, agregando tipos L1 (`NoTocado`, `Raio`,
  `Fantasma`, paths). **Sem `serde`, sem dep externa** (L1). `cargo tree -p
  lente_core` segue só o crate. (A serialização JSON **não** mora aqui.)
- **L4 — `analisar_diff` é composição.** Sequencia L3 (`ler_diff`,
  `enumerar_membros`) + L4 (`montar_grafo_workspace`) + L1 (`mapear_diff`,
  `calcular_raio`), montando o `ResultadoDiff` (tipo L1).
- **L2 — a CLI mapeia `ResultadoDiff`→JSON** (via DTO `serde` da L2 ou à mão,
  **como a trilha global já emite JSON** — seguir o padrão existente). A CLI pode
  usar deps; a L1 não.
- **Retrocompat**: o modo global da CLI **não muda**; `--diff` é aditivo.

---

## Parte 1 — L1: `ResultadoDiff` (`lente_core`)

```rust
pub struct TocadoComRaio { pub tocado: NoTocado, pub raio: Raio }

pub struct RaioCombinado {
    pub montante: Vec<(Path, usize)>,  // união dos montantes dos tocados; profundidade = menor (mais próximo); dedup; ordenado
    pub jusante:  Vec<(Path, usize)>,  // idem para jusante
}

pub struct ResultadoDiff {
    pub tocados: Vec<TocadoComRaio>,   // cada tocado + seu raio (para a vista por-item)
    pub combinado: RaioCombinado,      // união (para a vista resumo)
    pub ligados: Vec<PathBuf>,         // censo do untracked (0046/0043)
    pub soltos: Vec<PathBuf>,
    pub nao_fonte: Vec<PathBuf>,
    pub fantasmas: Vec<Fantasma>,      // do grafo de workspace (0045)
}
```

Carregar **os dois** níveis (raio por nó **e** combinado) deixa o resultado
completo para qualquer vista: a A usa `combinado`, a B usa `tocados[].raio`, a C
arranja por camada a partir dos paths. Os agrupamentos por crate as vistas
derivam dos paths (1º segmento) — o tipo guarda listas planas. Puro, sem `serde`.

Opcional: um helper puro `combinar_raios(&[Raio]) -> RaioCombinado` (união com
profundidade mínima), testável sem I/O. Decisão do gerador (helper L1 ou inline na
orquestração) — registrar.

---

## Parte 2 — L4: `analisar_diff` (`lente_wiring`)

```rust
pub fn analisar_diff(raiz: &Path) -> Result<ResultadoDiff, ErroLente>
```

1. `lente_infra::ler_diff(raiz)` (L3) → `DiffEstruturado`.
2. `montar_grafo_workspace(raiz)` (L4, 0045) → `GrafoWorkspace { grafo, fantasmas }`.
3. `lente_infra::enumerar_membros(raiz)` (L3) → os `membros_dirs`.
4. `lente_core::mapear_diff(&diff, &grafo, &membros_dirs)` (L1) → `MapeamentoDiff`.
5. Para cada `tocado` em `MapeamentoDiff::tocados`: `calcular_raio(&grafo,
   &tocado.path)` (L1) → `Raio`; coletar `TocadoComRaio`.
6. `combinado` = união dos `montante`/`jusante` dos tocados (dedup, profundidade
   mínima).
7. Montar `ResultadoDiff` { tocados, combinado, ligados, soltos, nao_fonte (do
   mapeamento), fantasmas (do `GrafoWorkspace`) }.

`ErroLente` ganha `Diff(lente_infra::ErroDiff)` (de `ler_diff`, 0046) com `From`
para `?`. (A variante `Workspace` já existe do 0045.)

**Lembrete de camada (consequência do 0045)**: variante nova de `ErroLente` quebra
o `match` exaustivo da CLI (`02_shell/cli/src/erro.rs::traduzir`). Adicionar o
braço `ErroLente::Diff(e)` + a mensagem de catálogo, no padrão do ADR-0002 — senão
não compila.

---

## Parte 3 — L2: CLI `--diff` → JSON (`lente_cli`)

- Acrescentar o modo `--diff` à CLI. Com ele, rodar `analisar_diff(raiz)` (a
  `raiz` = diretório atual por padrão; `--repo <caminho>` opcional — decisão do
  gerador) e emitir o **JSON** do `ResultadoDiff`.
- O JSON é montado na **L2** (mapeando o `ResultadoDiff` L1 para JSON — DTO `serde`
  ou à mão, **igual a como a trilha global emite JSON**). Esquema: tocados (cada um
  com path/kind e seu raio), combinado (montante/jusante), ligados/soltos/nao_fonte,
  fantasmas.
- **Só JSON neste prompt.** As três vistas de texto (A/B/C) são o 0048; o padrão
  de saída texto-vs-JSON e a flag `--vista` entram lá.
- O modo **global** da CLI fica **inalterado**.

---

## O que NÃO muda

- O modo global da CLI (a trilha global) — `--diff` é aditivo.
- `ler_diff`/`mapear_diff` (0046), `montar_grafo_workspace` (0045),
  `enumerar_membros` (0044) — usados como estão.
- As três vistas de texto — **não** entram aqui (0048).

---

## Critérios de Verificação

```
# L1 (puro)
Dado uma lista de Raio (montantes/jusantes com sobreposição)
Quando combinar_raios
Então RaioCombinado tem a união sem repetição, profundidade = a menor por path,
ordenado (determinístico)

# L4 (a montagem pura é testável sem git/fork, se extraída; o todo é #[ignore])
Dado um grafo forjado + um MapeamentoDiff forjado
Quando montar o ResultadoDiff (a parte pura: raio por tocado + combinado + censo)
Então tocados têm raio, combinado é a união, censo e fantasmas passam adiante

Dado o repo real com uma mudança conhecida (requer git + fork) — #[ignore]
Quando analisar_diff(raiz)
Então ResultadoDiff tem os tocados esperados com raio, o censo do untracked, e
os fantasmas do grafo (0 no repo, 0045)

# L2
Dado um ResultadoDiff montado
Quando emitir o JSON
Então o JSON tem tocados (com raio), combinado, ligados/soltos/nao_fonte,
fantasmas — desserializável

Dado o modo global da CLI
Quando rodar seus testes existentes
Então todos passam (--diff é aditivo)

Dado o código todo
Então cargo tree -p lente_core só o crate (ResultadoDiff é puro, sem serde)
```

Casos: `combinar_raios` (união, profundidade mínima, determinismo); a montagem
pura do `ResultadoDiff` (sem I/O); `analisar_diff` no repo real (`#[ignore]`); o
JSON (esquema, desserializável); não-regressão do modo global; pureza L1.

---

## Resultado esperado

- `ResultadoDiff`/`TocadoComRaio`/`RaioCombinado` (L1, puros, sem `serde`); opcional
  `combinar_raios`.
- `analisar_diff` (L4) montando o resultado completo; `ErroLente::Diff` + o braço
  na CLI.
- CLI `--diff` emitindo o **JSON** do resultado (mapeado na L2).
- **Pureza L1**: `cargo tree -p lente_core` só o crate.
- Testes: `combinar_raios` (unit), montagem pura (unit), `analisar_diff`
  (`#[ignore]`), JSON (unit), não-regressão do global.
- **Laudo** em `00_nucleo/lessons/0047-…`:
  - Onde ficou a montagem do `combinado` (helper L1 ou inline) e por quê.
  - O esquema do JSON, e que ele segue o padrão da trilha global.
  - Confirmação no repo real: tocados com raio, censo, fantasmas (0).
  - A `raiz` (cwd ou `--repo`).
  - O braço `ErroLente::Diff` na CLI (consequência de camada).
  - Contagem da suíte (era 258 verdes + 27 ignored no laudo 0046).

---

## Cuidados

- **`ResultadoDiff` é L1, sem `serde`.** Se a serialização tentar entrar na L1,
  pararia a pureza — o JSON é L2 (mapeia o tipo L1). Mesmo padrão da trilha global.
- **Variante nova de `ErroLente` toca a CLI.** Adicionar o braço `Diff` + catálogo
  (ADR-0002), como no 0045 — senão o `match` exaustivo não compila.
- **A montagem pura separável.** Se der, extrair a parte sem I/O (grafo +
  mapeamento → `ResultadoDiff`) numa função testável sem git/fork; o `analisar_diff`
  completo (com git+fork) fica `#[ignore]`. Não é obrigatório, mas facilita o teste.
- **Profundidade mínima no combinado.** Ao unir os montantes, um path que aparece
  via dois tocados fica com a profundidade **menor** (o mais próximo). Determinístico
  (ordenar).
- **Custo.** `calcular_raio` por nó tocado (BFS por nó); para um diff pequeno é
  barato. Diff enorme = muitos BFS; aceitável (diffs costumam ser pequenos) —
  registrar se aparecer lento.
- **Só JSON aqui.** As vistas de texto são o 0048; não antecipar.
- **Ordem de pouso**: este vem depois de 0046 (usa `ler_diff`/`mapear_diff`); 0037
  precede toda a cadeia (a `position`).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Liga o modo `--diff` e emite o dado view-agnóstico. **L1**: `ResultadoDiff` (`lente_core`, puro, **sem `serde`**) agrega tocados-com-raio (para a vista por-item), o raio combinado (união com profundidade mínima, para a vista resumo), o censo do untracked (`ligados`/`soltos`/`nao_fonte`, 0046) e os fantasmas (0045); `TocadoComRaio`/`RaioCombinado`; opcional `combinar_raios`. **L4**: `analisar_diff` sequencia `ler_diff` (0046) + `montar_grafo_workspace` (0045) + `enumerar_membros` (0044) + `mapear_diff` (0046) + `calcular_raio` por tocado, montando o `ResultadoDiff`; `ErroLente::Diff(ErroDiff)`. **L2**: CLI `--diff` roda `analisar_diff` e emite o **JSON** do resultado (mapeado na L2, padrão da trilha global); modo global inalterado; braço `ErroLente::Diff` + catálogo (ADR-0002). Decisão do autor: dado separado da vista — o resultado é completo e view-agnóstico; as três vistas de texto (A/B/C) são renderizadores e ficam para o 0048. Pureza L1 (`cargo tree -p lente_core` só o crate; sem dep nova na L1). Pré-req: 0044/0045/0046 (e 0037 na compilação). Suíte era 258+27. | `01_core/src/{resultado_diff (novo),lib.rs}`, `04_wiring/src/lib.rs`, `02_shell/cli/src/{lib.rs,erro.rs}`, `02_shell/catalogo/src/lib.rs`, `00_nucleo/lessons/0047-...` |
