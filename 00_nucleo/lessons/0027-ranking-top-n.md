# Laudo de Execução — Prompt 0027 (Modo ranking — top-N por impacto)

**Camada**: L5 (laudo)
**Data**: 2026-06-02
**Prompt executado**: `00_nucleo/prompt/0027-ranking-top-n.md`
**Estado**: `EXECUTADO` — modo ranking ponta-a-ponta: cálculo puro no
`lente_ranking` (L1 novo), fiação no `lente_wiring` (reuso de
extrair+resolver fatorado), CLI `--ranking [--top N]`, formatação no
catálogo/saída. 143 verdes + 15 ignored; pureza preservada; dois
subprocessos do cargo (invariante 0023). **Fecha a pendência 2 do laudo
0021** e fecha também a verificação do Limite 2 no egui (pendência do
laudo 0025).

---

## Fase 1 — Achados

### Verificação do Limite 2 no egui (a verificação central)

Roteiro: rodar o fork (`cargo modules export-json --sysroot --compact
--lib --package egui`) sobre `egui` v0.34.3 — 3694 nós, 13937 arestas —
e contar "primeiro segmento do path ∈ sysroot ∧ `trait_`/`trait_ref`
preenchido".

**Sobreposição: ZERO em 3694 nós.** Mesma propriedade verificada no
`lente_core` pelo laudo 0025 (108 nós). A regra do fork 0.27.0 "nomear
o lado do alvo" segue válida no crate difícil. **A cláusula C (preservar
impl-do-alvo cujo path cai sob sysroot) NÃO entra neste prompt** —
deixaria de ser especulação se o número fosse > 0, mas é 0.

Distribuição dos primeiros segmentos no `egui` (top-15):

| segmento | nós |
|---|---|
| `egui` | 3518 |
| `epaint` | 56 |
| `core` | 45 |
| `emath` | 20 |
| `ecolor` | 17 |
| `alloc` | 10 |
| `accesskit` | 8 |
| `std` | 5 |
| (outros < 5) | … |

Sysroot total: **60/3694 = 1.6%** do grafo do `egui`. Pequeno em
quantidade, mas (como o laudo 0021 já tinha visto) **domina o ranking**
porque o montante de stdlib é enorme.

### 730 impls-do-alvo de traits de stdlib

Contagem auxiliar: nós com path `egui::*` (ou outras deps não-stdlib) e
`trait` em `{Display, Debug, Clone, Default, PartialEq, Eq, Hash, From,
Into, AsRef, Drop, Iterator, IntoIterator, Copy, Ord, PartialOrd}` =
**730**. Todos preservados pelo filtro (verificado pelo `wiring`).

### Decisão de localização

`rankear` mora em crate novo `08_ranking/lente_ranking` — segue o padrão
`investiga`/`resolve`/`filtro` (cada componente L1 num crate). Pôr em
`lente_core::domain` faria sentido se o ranking fosse intrínseco ao
tipo `Grafo`; ele não é — é um consumidor que depende do `calcular_raio`.

### Custo dominado por extração (ancoragem da Arena)

Laudo 0021 Bloco A: tempo médio por crate = ~3s, dominado pela invocação
do fork. O laço de ranking sobre 3694 nós (egui) levou milissegundos no
E2E executado aqui. Validação prática para **não pré-otimizar**: o
ranking chama `calcular_raio` por nó, reindexando a cada chamada — bate
porque o custo verdadeiro mora antes (no fork).

---

## Fase 2 — Implementação

### Estrutura

```
08_ranking/                            (NOVO crate L1)
├── Cargo.toml          # dep: lente_core
└── src/lib.rs          # ItemRanking + rankear() + 8 testes

04_wiring/
├── Cargo.toml          # + lente_filtro, lente_ranking
└── src/lib.rs          # rankear_pacote() + obter_grafo_resolvido() (fatorado)

02_shell/catalogo/src/lib.rs
  + HELP_RANKING / HELP_TOP
  + RANKING_CABECALHO / RANKING_COLUNAS
  + JSON_RANKING / JSON_POSICAO / JSON_IMPACTO / JSON_PATH
  + ERRO_RANKING_COM_ALVO

02_shell/cli/src/{args,saida,main}.rs
  + flags --ranking / --top (conflicts_with_all com alvo/alvo-id)
  + formatar_ranking() (json e text)
  + run_ranking() (roteamento condicional)

Cargo.toml raiz: + "08_ranking" aos members
```

### API

```rust
// L1 (lente_ranking)
pub struct ItemRanking { pub path: Path, pub impacto: usize, pub classificacao: Classificacao }
pub fn rankear(grafo: &Grafo, n: usize) -> Vec<ItemRanking>

// L4 (lente_wiring)
pub use lente_ranking::ItemRanking;   // re-export — L2 não precisa depender de L1 ranking
pub fn rankear_pacote(fonte: FonteGrafo, n: usize) -> Result<Vec<ItemRanking>, ErroLente>
fn obter_grafo_resolvido(fonte: FonteGrafo) -> Result<Grafo, ErroLente>   // fatorado
```

Pipeline: `obter_grafo_resolvido` (extrai + desserializa + resolve
colisões) → `filtrar_stdlib` → `rankear` → top-N.

### Ordenação determinística

Decrescente por `impacto`; **desempate ascendente por path**. Testes
ancoram: `desempate_por_path_ascendente_e_deterministico`.

### Conflito no clap

`--ranking` é `conflicts_with_all = ["alvo", "alvo_id"]` (e vice-versa
nos dois). O `clap` curta o erro com mensagem padrão; não precisei usar
`ERRO_RANKING_COM_ALVO` (preservada no catálogo como salvaguarda futura
caso a validação saia do `derive`).

---

## Verificação

### Suíte (sem ignored)

| Crate | Verdes | Δ vs 0026 |
|-------|--------|-----------|
| lente_core | 30 | 0 |
| lente_infra | 30 | 0 |
| lente_investiga | 17 | 0 |
| lente_resolve | 11 | 0 |
| lente_filtro (lib) | 10 | 0 |
| **lente_ranking** | **8** | **+8 novo** |
| **lente_wiring** | **9** | **+3** (`rankear_pacote_*`, `modo_per_no_continua_…`) |
| lente_catalogo | 7 | 0 |
| **lente_cli** | **21** | **+5** (3 saída + 2 main) |
| **Total** | **143** | **+16** |

### Ignored (todos passam quando rodados)

| Item | Ignored | Δ |
|------|---------|---|
| lente_infra | 8 | 0 |
| lente_filtro (E2Es em `tests/`) | 3 | 0 |
| **lente_wiring** | 2 | **+1** (`e2e_ranking_do_lente_core_nao_traz_sysroot`) |
| **lente_cli** | 2 | **+1** (`e2e_ranking_lente_core_texto`) |
| **Total** | **15** | **+2** |

E2Es rodados: ambos verdes.

### Output real (ancoragem histórica)

`cargo run -p lente_cli -- --pacote lente_core --ranking --text --top 10`:

```
Ranking de impacto — top 10:
  #  Impacto  Classificação    Path
   1       39  Base             lente_core::entities::grafo::Path
   2       17  Base             lente_core::entities::grafo::Kind
   3       17  Base             lente_core::entities::grafo::Modificadores
   4       17  Base             lente_core::entities::grafo::Relation
   5       17  Base             lente_core::entities::grafo::Visibility
   6       11  Base             lente_core::domain::raio::Classificacao
   7       11  Intermediário    lente_core::entities::grafo::Aresta
   8       11  Intermediário    lente_core::entities::grafo::No
   9        7  Intermediário    lente_core::entities::grafo::Grafo
  10        7  Base             lente_core::entities::grafo::ValorDesconhecido
```

`Path` lidera (39 dependentes) — faz sentido: é o newtype mais
referenciado.

`lente --pacote egui --ranking --text --top 10` (workspace egui v0.34.3,
rodado do diretório do crate):

```
Ranking de impacto — top 10:
  #  Impacto  Classificação    Path
   1     1816  Base             emath::vec2::Vec2
   2     1683  Base             ecolor::color32::Color32
   3     1642  Base             egui::id::Id
   4     1617  Base             emath::rect::Rect
   5     1549  Base             emath::pos2::Pos2
   6     1547  Base             epaint::corner_radius::CornerRadius
   7     1508  Base             epaint::stroke::Stroke
   8     1501  Base             epaint::margin::Margin
   9     1484  Base             egui::style::TextStyle
  10     1473  Base             epaint::shadow::Shadow
```

**Compare com o laudo 0021 Bloco C**: lá, "7/10 do top-10 do egui são
`core::*`/`alloc::*`". Aqui o top-10 é 100% do ecossistema egui (egui,
emath, epaint, ecolor — dependências legítimas do egui, NÃO sysroot).
Confirmação empírica de ponta a ponta de que o filtro do laudo 0025
**ganhou seu consumidor** e fez seu trabalho.

### Invariante dos dois subprocessos (laudo 0023)

```
$ grep -rn 'Command::new("cargo")' --include "*.rs"
03_infra/src/fork.rs:117      # cargo modules
03_infra/src/metadata.rs:170  # cargo metadata
```

Continua dois únicos. O `lente_ranking` é puro L1 (zero subprocess).

### Pureza do L1 ranking

`cargo tree -p lente_ranking --depth 1` exibe só `lente_core`. Zero deps
externas; coerente com o padrão dos demais L1 (`investiga`, `resolve`,
`filtro`).

---

## Decisões tácitas

### D1 — `obter_grafo_resolvido` fatorado, mas privado

A função vive em `04_wiring/src/lib.rs` como `fn obter_grafo_resolvido`,
não `pub fn`. Razão: a fronteira de API do wiring continua sendo os
**pipelines completos** (`calcular_raio_de_alvo`, `rankear_pacote`); o
grafo resolvido cru é detalhe de implementação reusado entre os dois.
Expor `pub fn obter_grafo_resolvido` agora seria estrutura antes do uso
pedir. Quando um terceiro modo aparecer (ex.: "listar todas as
colisões resolvidas"), promove-se.

### D2 — Re-export `ItemRanking` no wiring

`pub use lente_ranking::ItemRanking` está em `04_wiring/src/lib.rs`. A
CLI (L2) consome `lente_wiring::ItemRanking` em vez de adicionar
dependência direta ao `lente_ranking` — a camada acima fala com **uma**
camada abaixo (a fronteira de fiação). O CI/`cargo tree` mostra:
`lente_cli → lente_wiring → lente_ranking`.

### D3 — Dedup de paths no `rankear`

Se o grafo tem dois nós com o mesmo path (colisão não resolvida), o L1
deduplica via `HashSet<&Path>` antes de chamar `calcular_raio` — uma
entrada por path no ranking, custo O(N). Teste
`path_repetido_aparece_uma_vez_no_ranking` ancora. No pipeline
`rankear_pacote`, a resolução acontece antes (`obter_grafo_resolvido`),
então essa proteção é defensa em profundidade contra grafos cruus
(ex.: alguém usar `lente_ranking::rankear` direto, sem wiring).

### D4 — Folha/Isolado entram no ranking com impacto 0

O `rankear` não filtra por classificação. Nós Folha (zero entrando) têm
`impacto = 0` e caem para o fim. Razão: "top-N" é ordenação + corte,
não seleção semântica. O consumidor decide se quer filtrar; o L1 não
adivinha. Caso típico: `n = 10` em grafos pequenos pode trazer folhas
no fim — comportamento honesto.

### D5 — Saída texto: 4 colunas alinhadas, sem emojis

Layout escolhido: `  #  Impacto  Classificação    Path` com
`"  {:>2}  {:>7}  {:<15}  {}"`. Larguras pensadas para top-99 com
classificações em PT-BR ("Intermediário" = 14 chars). Sem caracteres
ANSI/cores: o usuário pode `| grep` na saída sem ruído. JSON sempre
disponível para parsing programático.

### D6 — Conflito CLI via `clap` derive, não validação manual

`#[arg(long, conflicts_with_all = ["alvo", "alvo_id"])]`. O `clap`
produz erro no `parse()` antes do `run()` rodar — não precisa do
template `ERRO_RANKING_COM_ALVO` no fluxo atual. Mantive a constante
no catálogo (custo zero) para o dia em que a validação saia do
`derive` (ex.: regra dependente de combinação de outros flags).

### D7 — Arena (`lab/medicao-egui`) não foi removida

A Arena permanece como **registro de medição** (convenção do laudo
0021: experimentos de Arena ganham entrada em `lessons/`, conteúdo bruto
em `lab/`). O `lente_ranking` promove o **laço de cálculo**; a Arena
preserva o **dado histórico** + relatório completo. Não há duplicação
funcional: a Arena é instrumento de medição, o `lente_ranking` é
componente de produto.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0027 |
|-----------|-----------------|
| Laudo 0021, pendência 2 (sysroot domina ranking) | **Coberta** — filtro+ranking ponta-a-ponta; egui top-10 sem sysroot, dominado por tipos do ecossistema egui (Vec2, Color32, Id, …). |
| Laudo 0025, verificação do Limite 2 no egui | **Coberta** — sobreposição zero em 3694 nós (idêntico ao `lente_core`); cláusula C arquivada. |
| Pendência "raio-por-id" (latente) | **Não ativada** — `lente_ranking` opera sobre grafo resolvido (paths únicos); a pendência segue latente, sem nova razão para ser ativada agora. |
| Filtro de "folhas comportamentais" (Limite 3) | **Aberta** — outra pendência. |

---

## O que NÃO mudou (declaração explícita)

- `lente_core` (L1): zero toques.
- ADRs: zero toques.
- Fork (`cargo-modules`): zero toques.
- Quarentena E2 / `lente_investiga::fontes`: intacta.
- Modo per-nó (`--alvo`/`--alvo-id`): inalterado — `modo_per_no_continua_funcionando_apos_fatoracao` ancora.
- Subprocessos do cargo: continuam dois únicos.
- Assinaturas públicas pré-existentes: inalteradas (mudanças são **aditivas**).

---

## Observação metodológica

Este prompt promove o protótipo da **Arena** (`lab/medicao-egui`, laudo
0021) a componente de produto, instanciando a convenção "experimentos
de Arena ganham entrada em `lessons/`, código vira componente quando
amadurece" (candidato a LESSON do Tekt registrado no laudo 0021). E
fecha o ciclo do filtro 0025 → ranking 0027: o filtro nasceu sem
consumidor; aqui ele ganha o seu, e a verificação do Limite 2 no egui
(que ficou aberta no laudo 0025 por falta de uso real) é feita
**agora**, no momento exato em que o dado passa a ser usado de verdade.

Coerente com o princípio do projeto: "verificar antes de afirmar", mas
**no momento em que o uso passa pelo dado**, não antes (especulação) nem
depois (cegueira).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-02 | Modo ranking ponta-a-ponta: novo crate L1 `lente_ranking` (cálculo puro: `rankear(&Grafo, n) → Vec<ItemRanking>`); fiação `rankear_pacote` no `lente_wiring` (extrair+resolver fatorado em `obter_grafo_resolvido`; aplica `filtrar_stdlib` antes); CLI `--ranking [--top N]` com `conflicts_with_all` para alvo/alvo-id; rótulos e formatação JSON+texto no catálogo. Pendência 2 do laudo 0021 fechada; verificação do Limite 2 no egui (laudo 0025) fechada (sobreposição 0 em 3694 nós). Output real ancorado: top-10 do egui com `Vec2`/`Color32`/`Id`/`Rect`/… (compare com 0021 Bloco C: antes 7/10 eram `core::`/`alloc::`). 143 verdes + 15 ignored; pureza do L1 mantida; dois subprocessos do cargo (0023). | `08_ranking/{Cargo.toml,src/lib.rs}`, `04_wiring/{Cargo.toml,src/lib.rs}`, `02_shell/catalogo/src/lib.rs`, `02_shell/cli/src/{args,saida,main}.rs`, `Cargo.toml` raiz, `00_nucleo/lessons/0027-ranking-top-n.md` |
