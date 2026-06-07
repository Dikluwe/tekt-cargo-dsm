# Laudo de Execução — Prompt 0037 (Consumir `position` no `No`)

**Camada**: L5 (laudo)
**Data**: 2026-06-04
**Prompt executado**: `00_nucleo/prompt/0037-position_no_core_infra.md`
**Estado**: `EXECUTADO` — `Posicao` novo em `lente_core` (stdlib só);
`No.position: Option<Posicao>` aditivo; `lente_infra` desserializa
`position` (`Option<PositionDTO>`, `#[serde(default)]`) e propaga
verbatim. **213 verdes** (+7) + **22 ignored** (+1 E2E real verde); pureza
do L1 mantida (`cargo tree -p lente_core` só o crate); dois subprocessos
do cargo (invariante 0023).

---

## A entrega — sem função visível ainda

A 5ª rodada do fork emite `position` por nó. Este prompt entrega só o
**armazenamento**: o `No` da lente passa a carregar a posição (quando o
fork a trouxe). Sem mapeamento diff→nós, sem cálculo, sem modo de CLI —
isso é trilha futura. **Primeira mudança da trilha local.**

```rust
// lente_core::entities::grafo
pub struct Posicao {
    pub file: String,        // verbatim, absoluto
    pub start_line: u32,     // 1-based
    pub end_line: u32,       // 1-based
}

pub struct No {
    …
    pub position: Option<Posicao>,   // None p/ embutidos ou JSON antigo
}
```

---

## Nome adotado: `Posicao`

O prompt deixou a decisão aberta entre `Posicao` (PT, espelhando
`No`/`Aresta`/`Raio`/`Modificadores`) e `Position` (espelhando o nome do
campo JSON, como `UsesKind` espelha `uses_kind`). Escolhi **`Posicao`**:

- Casa com o português dos demais tipos-de-tradução: `No`, `Aresta`,
  `Raio`, `Classificacao`, `Modificadores`.
- A correspondência JSON↔Rust é por **campo** (já fazemos `crate` →
  `crate_name`, `trait` → `trait_`, etc.); manter o **tipo** em PT é
  coerência estilística, não atrito de leitura.
- Quando algum dia o consumidor pedir uma `Position` específica de uma
  outra dimensão (ex.: posição na DSM), o nome fica livre para esse uso.

`UsesKind` permanece em inglês como caso especial — ele **é** um enum
que tenta ser-espelho-do-fork (`Reference`/`Import`).

---

## Comportamento confirmado do serde

`#[serde(default)]` em `position: Option<PositionDTO>` cobre os dois
caminhos de ausência **sem erro**:

1. **Nó sem o campo `position`** (item embutido — `core::any`, `alloc::*`):
   → `None` (teste `no_sem_position_no_json_vira_none`).
2. **JSON antigo sem `position` em nó nenhum** (fork pré-5ª-rodada):
   → todos os nós `position == None` (teste
   `fork_antigo_sem_position_em_nenhum_no_desserializa_sem_erro`).

Contraste **deliberado** com o `id` do prompt 0006: lá, ausência é erro
(distingue JSON velho de novo); aqui, ausência é estado válido (alguns
itens **legitimamente** não têm fonte). O diagnóstico "atualize o fork"
fica para o consumidor (modo de CLI futuro) — sem consumidor não há o
que diagnosticar.

`Option<PositionDTO>` mais `#[serde(default)]` é redundante (serde já
trata Option ausente como `None`), mas o `default` foi mantido como
**defesa em profundidade** — coerente com os demais campos opcionais
do `NoDTO` (`is_const`, `trait_`, etc.).

---

## Estrutura da mudança

```
01_core/src/entities/grafo.rs
  + pub struct Posicao { file, start_line, end_line }   (stdlib só)
  ~ pub struct No { …, pub position: Option<Posicao> }
  + 3 testes:
    posicao_carrega_arquivo_e_linhas_1_based
    no_com_position_some_e_acessivel
    no_com_position_none_e_estado_valido

03_infra/src/dto.rs
  + pub(crate) struct PositionDTO { file, start_line, end_line }
  ~ NoDTO ganha `#[serde(default)] position: Option<PositionDTO>`

03_infra/src/traducao.rs
  ~ traduzir propaga `position` verbatim:
      position: no_dto.position.map(|p| Posicao { … })
  + 4 testes:
    position_preenchida_no_json_e_lida_verbatim
    no_sem_position_no_json_vira_none
    fork_antigo_sem_position_em_nenhum_no_desserializa_sem_erro
    mistura_com_e_sem_position_e_resolvida_individualmente
  + 1 E2E #[ignore]:
    e2e_lente_core_real_traz_position_em_pelo_menos_um_no
```

**Ripple coordenado** (lição do laudo 0012): adicionar campo público ao
`No` quebra **todo** construtor literal. Atualizei **9 sites** com
`position: None` (mecânico):

| Arquivo | Tipo | Localização |
|---|---|---|
| `01_core/src/entities/grafo.rs:499` | helper de teste `no_de` | tests |
| `01_core/src/domain/raio.rs:247` | helper de teste `no` | tests |
| `03_infra/src/traducao.rs:138` | helper de teste `no_dto` (`NoDTO`) | tests |
| `05_investiga/src/lib.rs:123` | helper de teste `no` | tests |
| `05_investiga/src/fontes.rs:406` | helper de teste `no` | tests |
| `06_resolve/src/lib.rs:214` | helper de teste `no` | tests |
| `07_filtro/src/lib.rs:121` | helper de teste `no` | tests |
| `08_ranking/src/lib.rs:87` | helper de teste `no` | tests |
| `09_estrutura/src/lib.rs:471` | helper de teste `no` | tests |

A construção em `03_infra/src/traducao.rs:52` (produção) é a única que
**lê** `position` do DTO; todas as outras recebem `None`.

`lab/` (Arenas) não constrói `No` literal — só consome `Grafo` via
desserialização, então o ripple não chega lá.

---

## Verificação

### Suíte (sem ignored)

| Crate | Verdes | Δ vs commit 8098e3b (laudo 0036) |
|-------|--------|-----------|
| **lente_core** | **35** | **+3** (Posicao + 2 No.position) |
| **lente_infra** | **39** | **+4** (4 testes de tradução do position) |
| lente_investiga | 17 | 0 |
| lente_resolve | 11 | 0 |
| lente_filtro | 15 | 0 |
| lente_ranking | 8 | 0 |
| lente_estrutura | 23 | 0 |
| lente_wiring | 20 | 0 |
| lente_catalogo | 7 | 0 |
| lente_cli | 38 | 0 |
| **Total** | **213** | **+7** |

### Ignored (todos verdes quando rodados)

| | Δ |
|---|---|
| lente_infra | 9 (+1: `e2e_lente_core_real_traz_position_em_pelo_menos_um_no`) |
| lente_filtro (tests/) | 3 |
| lente_wiring | 7 |
| lente_cli | 3 |
| **Total** | **22** (+1) |

**E2E real rodado**: `cargo test -p lente_infra
e2e_lente_core_real_traz_position -- --ignored` → **verde** (3.05s). O
fork instalado (commit `ddcd3ca`, ver laudo 0033 D3) emite `position`;
pelo menos um nó do `lente_core` real chega com `Some(Posicao{…})`,
`file` não-vazio, `start_line <= end_line`. Sanidade da 5ª rodada do
fork confirmada contra dado real.

### Pureza L1 (preservada)

```
$ cargo tree -p lente_core --depth 1
lente_core v0.0.0
```

Zero deps externas. `Posicao` usa só `String` + `u32`. Coerente com a
declaração de pureza do `lente_core` (laudo 0006 + laudos seguintes).

### Subprocessos do cargo (invariante 0023)

```
03_infra/src/fork.rs:117      # cargo modules
03_infra/src/metadata.rs:170  # cargo metadata
```

Dois únicos, intocados. Prompt 0037 não introduz subprocesso.

---

## Decisões tácitas

### D1 — `Posicao` em vez de `Position`

Já justificado acima. Resumo: PT casa com o restante dos tipos-de-tradução
do crate; `UsesKind` em inglês é caso especial (espelho de enum). `Posicao`
libera o nome `Position` para outro uso futuro (ex.: posição na DSM).

### D2 — `#[serde(default)]` em `position` mesmo sendo `Option`

Serde já trata Option ausente como `None`, então `#[serde(default)]` é
**redundante semanticamente**. Mantido por:

- **Defesa em profundidade**: o comportamento fica explícito; não
  depende de detalhe do `#[derive(Deserialize)]`.
- **Coerência**: todos os demais campos opcionais do `NoDTO`
  (`is_const`, `trait_`, `cfg`, etc.) já têm `#[serde(default)]`. Manter
  o padrão simplifica leitura.
- **Custo zero**: não muda comportamento; só registra a intenção.

### D3 — Caminho armazenado verbatim (absoluto)

`position.file` chega do fork como **absoluto** (ex.:
`/home/.../01_core/src/entities/grafo.rs`). O prompt foi explícito: não
relativizar aqui. Relativizar para casar com `git diff` (que traz paths
relativos à raiz do crate) é trabalho do **mapeamento diff→nós** (prompt
futuro), com regras próprias (e potencialmente ambigüidades).

Armazenar verbatim:
- **Preserva** o que o fork disse;
- **Não decide nada** que o consumidor precise rever depois;
- **Defere** a complexidade da relativização para onde ela faz sentido.

### D4 — E2E `#[ignore]` em vez de unit test "rode o fork"

O E2E real (rodar o fork no `lente_core` e conferir `position`) é caro
(3s) e depende do fork instalado. Ficar como `#[ignore]` é a convenção do
projeto (cf. laudos 0017, 0023, 0034) para E2Es contra binário externo
— rodáveis em CI com `-- --ignored`, fora do laço de feedback rápido.

A função do E2E **aqui**: se o fork em PATH for antigo (pré-5ª-rodada),
o assert `com_position > 0` falha **imediatamente** com mensagem "fork
instalado parece antigo: nenhum nó traz position". É o mesmo padrão
"diagnóstico claro em vez de bug mudo" do laudo 0024 (`DiretorioInexistente`).

### D5 — Construir o ripple antes dos testes do `lente_infra`

Adicionar `position` ao `No` quebrou a compilação dos 9 helpers de teste
**antes** de eu poder escrever os testes do `lente_infra`. Corrigi o
ripple primeiro (build limpo), depois adicionei os testes específicos
do `position` em `traducao.rs`. Ordem inversa (testes primeiro) seria
mais sensível à integração contínua — esta ordem foi prática.

### D6 — Sem tocar Arenas (`lab/`)

As Arenas (`lab/medicao-egui`, `lab/medicao-ciclos-egui`,
`lab/medicao-ciclos-referencia`, `lab/proto-ui`, `lab/proto-dsm`) **não**
constroem `No` literal — todas consomem `Grafo` desserializado. O ripple
não chega lá. Coerente com o padrão Arena (laudo 0021): bruto em `lab/`,
componente em produção; mudanças aditivas no produto que não exigem
ação na Arena ficam silenciosamente compatíveis.

### D7 — Testes inline em vez de fixture

O prompt deixou a escolha aberta. Optei por **JSON inline** em cada
teste (≤ 20 linhas cada) em vez de criar fixture nova:

- **Explícito**: o leitor do teste vê **exatamente** o que está sendo
  desserializado, sem precisar abrir outro arquivo.
- **Independente do fork ao vivo**: testes unitários rodam offline.
- **Cenários distintos por teste**: cada cenário tem um JSON ad-hoc com
  só o que importa para aquele caso.

A fixture `crate-amostra` existente serve aos E2Es (rodam o fork); para
testes unitários da tradução, JSON inline é mais limpo.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0037 |
|-----------|-----------------|
| Consumir `position` no `No` (trilha local — pré-requisito do diff→nós) | **Coberto** — `Posicao` + `No.position`; tradução verbatim; ausência tolerada. |
| Mapeamento diff→nós | **Aberta** — prompt futuro; precisa de regra de intersecção linha↔span. |
| Relativizar `position.file` para casar com `git diff` | **Aberta** — vive no mapeamento, não aqui. |
| Cálculo do raio sobre nós tocados | **Aberta** — `calcular_raio` já existe; ligar ao subconjunto tocado é parte do modo `--diff`. |
| Modo `lente --diff …` na CLI | **Aberta** — Ponte 2 da trilha local. |
| Diagnóstico "atualize o fork" quando `position` ausente em tudo | **Aberta** — vive no modo `--diff` (sem consumidor agora, não há o que diagnosticar). |
| Casca MCP da trilha local | **Aberta** — Ponte 2 da trilha local. |

---

## O que NÃO mudou

- **Fork** (`cargo-modules`): zero toques. Usa o `position` que o fork
  já emite (5ª rodada).
- **Spec, ADRs**: zero toques. A spec da forma (laudo 0028) ainda não
  documenta `position` — caberá fazer quando o consumidor (diff→nós)
  estiver no produto. Por ora, é campo opcional aditivo, como o
  `uses_kind` antes do laudo 0034.
- **Modos da CLI** (`--alvo`/`--alvo-id`/`--ranking`/`--estrutura`):
  zero toques no comportamento. `position` está no `Grafo` mas nenhum
  modo o consome.
- **Arenas** (`lab/`): zero toques.
- **`Cargo.toml` raiz** e **`Cargo.lock`**: intocados (sem deps novas).
- **Subprocessos do cargo** (invariante 0023): dois únicos.

---

## Observação metodológica

**Aditivo é barato; verbatim é honesto.** Adicionar `position` ao `No`
como `Option<Posicao>` não muda comportamento de nenhum consumidor
existente: quem não lê o campo continua não vendo nada. O custo é
puramente o **ripple mecânico** dos 9 helpers — uma linha cada. Em
troca, o `Grafo` ganha a informação que o fork já trazia e era
descartada.

A `position` é o **primeiro tijolo da trilha local** — a vista que
mostra "o que esta mudança toca". Pelo padrão do projeto (laudos
0006/0012/0013): consumir o campo **antes** de construir o consumidor.
Quando o `--diff` for prompt próprio, o `No.position` já está pronto;
não há novo `--no-run` para coordenar entre dois prompts.

E o E2E `#[ignore]` ancora o ciclo: o fork instalado **realmente** emite
`position` no `lente_core`. Se o ambiente regredir (fork antigo), o
teste falha **com mensagem direta** — não com um bug silencioso.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Consumir `position` no `No`: tipo `Posicao` aditivo no `lente_core` (stdlib só); `No.position: Option<Posicao>` opcional por natureza; `lente_infra` desserializa `position` (Option, ausência → None) e propaga verbatim. Primeiro tijolo da trilha local (pré-requisito do mapeamento diff→nós, prompt futuro). 213 verdes + 22 ignored; pureza do L1 mantida (`cargo tree -p lente_core` só o crate); E2E real verde contra fork instalado (commit `ddcd3ca`). | `01_core/src/entities/grafo.rs`, `03_infra/src/{dto.rs,traducao.rs}`, ripple em 9 helpers de teste (`01_core`, `03_infra`, `05_investiga`, `06_resolve`, `07_filtro`, `08_ranking`, `09_estrutura`), `00_nucleo/lessons/0037-position_no_core_infra.md` |
