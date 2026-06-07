# Laudo de Execução — Prompt 0054 (mapear o refactor V3+V12 antes de mover)

**Camada**: transversal (investigação) — **sem código**
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0054-mapear_refactor_v3_v12.md`
**Estado**: `EXECUTADO` — mapa completo na fonte; plano em 3 estágios com delta de
V3/V12. **Nenhum código tocado** (só leitura). Os estágios vêm depois, aprovados à
parte.

---

## A resposta em uma sentença

A hipótese do re-export **confirma-se**: dos 13 símbolos que a CLI importa do
`lente_wiring`, **6 nascem no L1 e são só re-exportados** (re-apontar = Estágio 1,
grátis), **6 são L4-nativos puros** (só dependem do L1 → mover ao L1 = Estágio 2),
e **só o `ErroLente` é L4-nativo cross-layer** (agrega 4 erros do L3 → **fica no
L4**, legítimo; a CLI deixa de precisar dele ao relocar o ponto de entrada =
Estágio 3). Resultado projetado: **V3 8→0**, **V12 5→1** (o `ErroLente`,
declarado intencional).

---

## Tabela de símbolos (onde definido · define/re-exporta · classificação)

| Símbolo | Definido em | `lente_wiring` | Classe |
|---|---|---|---|
| `ResultadoDiff` | `lente_core::domain::resultado_diff` (L1) | **re-exporta** (l.49) | **(i)** re-apontar |
| `TocadoComRaio` | idem (L1) | **re-exporta** (l.49) | **(i)** re-apontar |
| `RaioCombinado` | idem (L1) | **re-exporta** (l.49) | **(i)** re-apontar |
| `Fantasma` | `lente_core::domain::uniao` (L1) | **re-exporta** (l.52) | **(i)** re-apontar |
| `Ciclo` | `lente_estrutura` (L1) | **re-exporta** (l.53) | **(i)** re-apontar |
| `ItemRanking` | `lente_ranking` (L1) | **re-exporta** (l.54) | **(i)** re-apontar |
| `FonteGrafo` | `lente_wiring` (L4) | **define** (l.57) | **(ii)** mover ao L1 (puro: `String`) |
| `Escopo` | `lente_wiring` (L4) | **define** (l.84) | **(ii)** mover ao L1 (puro: unit) |
| `ModoUses` | `lente_wiring` (L4) | **define** (l.110) | **(ii)** mover ao L1 (puro: unit) |
| `AlvoBusca` | `lente_wiring` (L4) | **define** (l.122) | **(ii)** mover ao L1 (`Path` L1) |
| `DependenciaModulo` | `lente_wiring` (L4) | **define** (l.372) | **(ii)** mover ao L1 (`Path` L1) |
| `EstruturaModulos` | `lente_wiring` (L4) | **define** (l.387) | **(ii)** mover ao L1 (`Path`/`DependenciaModulo`/`Ciclo`, todos L1) |
| `ErroLente` | `lente_wiring` (L4) | **define** (l.130) | **(iii)** L4-nativo **cross-layer** → **fica** |

`GrafoWorkspace` (struct, l.239) é L4-nativo mas a CLI **não** o importa (não está
nos 8 sítios) — fora do refactor. Structs não disparam V12 (`allow_adapter_structs
= true`); só os **5 enums** disparam (`FonteGrafo`, `Escopo`, `ModoUses`,
`AlvoBusca`, `ErroLente`).

---

## Funções de orquestração que a CLI chama (`lente_wiring::…`)

| Função | Assinatura (resumo) | Retorna |
|---|---|---|
| `calcular_raio_de_alvo` (l.215) | `(FonteGrafo, AlvoBusca, Escopo)` | `Result<Raio, ErroLente>` |
| `rankear_pacote` (l.356) | `(FonteGrafo, usize, Escopo)` | `Result<Vec<ItemRanking>, ErroLente>` |
| `analisar_estrutura` (l.417) | `(FonteGrafo, Escopo, ModoUses)` | `Result<EstruturaModulos, ErroLente>` |
| `analisar_diff` (l.288) | `(&Path)` | `Result<ResultadoDiff, ErroLente>` |
| `montar_grafo_workspace` (l.258) | `(&Path)` | `Result<GrafoWorkspace, ErroLente>` (a CLI não chama direto) |

São o que exige o **Estágio 3** (relocação do ponto de entrada): toda chamada
devolve `Result<_, ErroLente>`, então quem chama precisa do `ErroLente`.

---

## Mapa de dependentes do vocabulário L4-nativo

Grep em todo o código (`01_core`, `03_infra`, `04_wiring`, `02_shell`):

- **`FonteGrafo`/`AlvoBusca`/`Escopo`/`ModoUses`/`EstruturaModulos`/`DependenciaModulo`**:
  usados **só** (a) pela **CLi** (L2) e (b) pelas **assinaturas das funções do
  próprio `lente_wiring`** (L4). **Nenhum outro crate** os referencia (os matches
  em `01_core/resolve` são *strings de fixture de teste* `"W::ErroLente"`, não
  imports). → Mover ao L1 toca **dois** lugares: a CLI e a fachada do wiring.
- **`ErroLente`**: idem — só a CLI e o wiring (e fixtures-string no resolve).

Consequência: o refactor é **localizado** (CLI + wiring), sem efeito em
`lente_core`/`infra`/`filtro`/`ranking`/`estrutura` além de **receber** os tipos
que descem.

---

## `ErroLente` dissecado (variante × camada)

| Variante | Tipo embrulhado | Camada |
|---|---|---|
| `Fork(ErroFork)` | `lente_infra::fork` | **L3** |
| `Adaptador(ErroAdaptador)` | `lente_infra` | **L3** |
| `Workspace(ErroWorkspace)` | `lente_infra` | **L3** |
| `Diff(ErroDiff)` | `lente_infra` | **L3** |
| `Resolucao(ErroResolve)` | `lente_resolve` | L1 |
| `Raio(ErroRaio)` | `lente_core` | L1 |
| `IdInexistente(usize)` | primitivo | — |
| `ForkSemUsesKind` | unit | — |

**Veredito: `ErroLente` NÃO desce ao L1.** Quatro variantes embrulham erros do
**L3**; descê-lo ao L1 faria o L1 referenciar o L3 (violação pior). É erro
**agregado** — mora legitimamente na composição (L4). O V12 dele é **intencional**.

---

## Decisões de significado (propostas, com trade-offs — você decide)

### A casa no L1 do vocabulário que move (Estágio 2)

- **`EstruturaModulos` + `DependenciaModulo`** → **`lente_estrutura`** (L1).
  Natural: juntam-se ao `Ciclo`/`OrdemDsm` que **já** vivem lá (o vocabulário de
  estrutura já é desse crate). Sem trade-off relevante — é a casa óbvia.
- **`FonteGrafo`/`AlvoBusca`/`Escopo`/`ModoUses`** (o "como o usuário pede") —
  três opções:
  - **(a) módulo novo no `lente_core`** (`01_core/core/src/domain/consulta.rs`).
    *Prós*: um lugar, zero crate novo, `lente_core` já é a fundação que todos
    importam. *Contras*: leve scope-creep — `lente_core` é dados-do-grafo, e isto
    é "vocabulário de pedido".
  - **(b) crate L1 novo `lente_consulta`** (`01_core/consulta`). *Prós*: casa
    dedicada e honesta; separa "pedido" de "dados". *Contras*: um crate para 4
    enums pequenos; mais um `path` em todo mundo.
  - **(c) distribuir** (`Escopo`→filtro, `ModoUses`→estrutura, …). *Contras*:
    fragmenta um vocabulário coeso; **descartada**.
  - **Recomendo (a)** — um `domain/consulta.rs` no `lente_core` — pelo custo
    mínimo; (b) se você preferir a separação semântica forte. Sua decisão.

### A forma do ponto de entrada L4 (Estágio 3)

- **(A) binário no `04_wiring`** — o `lente_wiring` ganha `[[bin]]`; o `main` (L4)
  faz args→orquestração→trata `ErroLente`→formatadores. *Prós*: sem crate novo.
  *Contras*: mistura lib + app no mesmo crate; o L4 passa a importar L2
  (catálogo/args/saida — permitido, é para baixo).
- **(B) crate L4 novo de app** (`04_wiring/app` ou `05_app`) com o `[[bin]]`;
  importa o `lente_wiring` (lib L4) + a apresentação L2. *Prós*: separação limpa
  (wiring-lib puro vs app-composição); o binário é explicitamente o
  ponto-de-composição. *Contras*: um crate novo.
  - **Recomendo (B)** — o ponto de entrada como crate L4 próprio é a forma
    Cristalina mais honesta (o `tekt-linter` põe o `main` em `04_wiring`); (A) se
    quiser evitar o crate novo. Sua decisão.

A **tradução de erro** (`erro.rs::traduzir(ErroLente)→catálogo`) **sobe junto** ao
app L4: lá ela legitimamente conhece o `ErroLente` (L4) e usa os templates do
catálogo (L2, para baixo). É o que tira o `ErroLente` da CLI sem movê-lo de camada.

---

## O plano em 3 estágios (com delta de V3/V12)

Os **8 sítios** do V3 (todos `use lente_wiring::…` em `02_shell/cli`):
`erro.rs:8`, `main.rs:18`, `saida.rs:{16,971,1142,1227,1306,1321}`.

### Estágio 1 — re-apontar os L1-origem (mecânico, preserva comportamento)

Trocar `use lente_wiring::{…}` por `use lente_core::…` / `lente_estrutura::` /
`lente_ranking::` para os **6 símbolos classe (i)**. Limpa os sítios **só-(i)**:
`saida.rs:1142, 1227, 1306, 1321` (4 sítios). Os mistos (`saida.rs:16`,
`saida.rs:971`) têm a parte (i) re-apontada mas **permanecem** (ainda importam
(ii) do wiring).
**Delta: V3 8 → 4. V12 5 → 5.** Mesmo tipo, outro caminho — comportamento idêntico.

### Estágio 2 — mover o vocabulário L4-nativo puro para o L1

Mover `FonteGrafo`/`AlvoBusca`/`Escopo`/`ModoUses` → casa escolhida (rec.: `lente_core::domain::consulta`);
`EstruturaModulos`/`DependenciaModulo` → `lente_estrutura`. O `lente_wiring` passa
a **importá-los do L1** (nas assinaturas das 5 funções) em vez de defini-los; a CLI
importa-os do L1. Limpa `main.rs:18`, `saida.rs:16`, `saida.rs:971`.
**Delta: V3 4 → 1** (só `erro.rs:8`/`ErroLente`). **V12 5 → 1** (os 4 enums
saíram; sobra `ErroLente`). Toca: wiring (fachada) + CLI + os crates L1 que
recebem os tipos.

### Estágio 3 — relocar o ponto de entrada (a CLI vira apresentação pura)

O `main` + a tradução de `ErroLente` sobem para o app L4 (rec.: crate `05_app`/`04_wiring/app`).
A CLI (`02_shell/cli`) fica **só apresentação**: `args.rs` (clap) e `saida.rs`
(formatadores sobre tipos **L1**) — importa **só L1** (e o catálogo L2, lateral
na própria L2). As chamadas de orquestração e o `ErroLente` saem da L2.
**Delta: V3 1 → 0. V12 5 → 1** (o `ErroLente` fica no L4, **declarado
intencional** — erro agregado da composição).

---

## Veredito do V12 (quantos movem, quantos ficam)

- **4 dos 5 movem** (`FonteGrafo`, `Escopo`, `ModoUses`, `AlvoBusca` → L1, Estágio 2).
- **1 fica** (`ErroLente`, L4) — **legítimo** (agrega L3); declarar intencional (ou
  aceitar como warning). V12 final = **1**.

---

## Estado projetado ao fim dos 3 estágios

| Check | Hoje (0053) | Pós-Estágio 3 |
|---|---|---|
| **V3** | 8 | **0** |
| **V12** | 5 | **1** (`ErroLente`, intencional) |
| V8, V4, V13, V9 | 0 | 0 (preservados) |
| V1 | 40 | 40 (fora do escopo) |

---

## O que NÃO foi feito (conforme o prompt)

- **Nenhum** código movido, import trocado ou binário mexido — **só o mapa**.
- A hipótese do re-export foi **confirmada na fonte** (4 linhas `pub use`, l.49–54).
- O `ErroLente` foi dissecado e **fica no L4** (não se forçou a descida).
- As decisões de significado (casa no L1, forma do ponto de entrada) foram
  **propostas com trade-offs** — a escolha é sua, antes do Estágio 1.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Mapa do refactor V3+V12 (do 0053), **sem código**. Confirmado na fonte (`04_wiring/src/lib.rs`): dos 13 símbolos que a CLI importa do `lente_wiring`, **6 são L1-origem re-exportados** (`ResultadoDiff`/`TocadoComRaio`/`RaioCombinado` ← `lente_core::domain::resultado_diff`; `Fantasma` ← `lente_core::domain::uniao`; `Ciclo` ← `lente_estrutura`; `ItemRanking` ← `lente_ranking` — `pub use`, l.49–54), **6 são L4-nativos puros** (`FonteGrafo`/`Escopo`/`ModoUses`/`AlvoBusca`/`EstruturaModulos`/`DependenciaModulo` — só dependem do L1: `String`/`Path`/`Ciclo`), e **1 é L4-nativo cross-layer** (`ErroLente`, agrega 4 erros L3 — `ErroFork`/`ErroAdaptador`/`ErroWorkspace`/`ErroDiff` — logo **fica no L4**). Dependentes do vocabulário L4-nativo: **só** a CLI e as assinaturas das 5 funções do wiring (`calcular_raio_de_alvo`/`rankear_pacote`/`analisar_estrutura`/`analisar_diff`/`montar_grafo_workspace`) — nenhum outro crate. Plano em 3 estágios: **1** re-apontar os L1-origem (V3 8→4, mecânico); **2** mover o vocabulário puro ao L1 (V3 4→1, V12 5→1); **3** relocar o ponto de entrada — `main`+tradução de erro sobem ao app L4, a CLI vira apresentação pura só-L1 (V3 1→0). Casa proposta: `EstruturaModulos`/`DependenciaModulo`→`lente_estrutura`; os 4 enums de pedido→`lente_core::domain::consulta` (rec.) ou crate `lente_consulta`. Ponto de entrada: crate L4 de app (rec.) ou `[[bin]]` no `04_wiring`. V12: 4 movem, 1 (`ErroLente`) fica intencional. Nenhum código tocado. | `00_nucleo/lessons/0054-mapear_refactor_v3_v12.md` |
