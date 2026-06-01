# Medição: Lente contra o workspace egui

**Tipo**: Experimento de Arena (`lab/`) — primeira medição contra projeto externo
**Prompt**: `00_nucleo/prompt/0021-medicao-egui.md`
**Data**: 2026-06-01
**Fork**: `cargo-modules` 0.27.0 (commit `a928eba8`)
**Lente**: estado pós-laudo 0020 (sistema composto, CLI funcional)
**Egui**: v0.34.3, workspace em `/home/dikluwe/Documentos/GitHub/egui`

---

## Bloco A — Sistema funciona em projeto não-trivial?

### Operacional

- **11 dos 12 crates** do workspace egui processados com sucesso.
- **1 erro** (`egui_demo_app`): o `cargo modules export-json` retornou *"Multiple targets present in package"* — o pacote tem `[lib]` + `[[bin]]`, e o fork exige `--lib` ou `--bin` para desambiguar. O invocador atual da lente (`lente_infra::invocacao`) **não cobre esse caso** (cobre múltiplos packages no workspace, não múltiplos targets no mesmo package). Descoberta operacional do experimento.
- **Tempo total**: **279.8s** (~4 min 40 s). Bem abaixo da estimativa do prompt (1-2 h).
- **Sem panic, sem erro de pipeline** ao longo dos 11 crates.

### Tamanhos

- Maior: **`egui`** com 3 694 nós, 13 937 arestas, 31 colisões.
- Segundo: **`epaint`** com 1 231 nós, 4 360 arestas, 29 colisões.
- **`emath`** denso: só 513 nós, mas **20 colisões** (operadores aritméticos de coordenada — predição).
- **`epaint_default_fonts`** trivial: 1 nó, 0 arestas, 0 colisões.

### Tempo

- Variação grande: **3.7 s** (epaint_default_fonts, 1 nó) a **123.1 s** (ecolor — anômalo, ver Bloco D).
- A maioria caiu entre 4 s e 25 s.
- **Total egui agregado**: 7 436 nós, 24 764 arestas, 97 colisões.

---

## Bloco B — As predições da resolução de colisões se confirmam?

| Predição | Resultado | Status |
|----------|-----------|--------|
| Cobertura E1 ≥ 95 % | **100 %** (97/97) | **confirmada com folga** |
| `ImplDeTraitsDiferentes` → 0 (pós-E2-quarentena) | **0** | confirmada |
| `MesmoItem` > 0 (reexports do egui exerceriam) | **0** | **refutada** |
| `NaoDeterminado` baixo, padrão diferente do typst | **0** | confirmada (melhor) |

### Cobertura E1: 100%

Todos os **97 casos** decididos como `Distintos / VizinhancaDisjunta`. **Zero `NaoDeterminado`** — não apareceu padrão equivalente aos `typst_macros::util::kw::*` (o "Limite 6"). O egui é mais bem-comportado nesse aspecto que o typst.

### `MesmoItem`: zero — predição refutada

A predição era que o `MesmoItem` apareceria no egui por causa de muito reexport. **Não apareceu.** Hipótese: o fork não emite reexports como nós colidentes (path duplicado) — emite como **arestas `Uses`** apontando para o nó original. Logo, mesmo com reexport intenso, a colisão por reexport não existe no JSON do fork. *Conclusão da medição*: o `MesmoItem`, no design do fork 0.27.0, parece ser **caminho teórico raramente exercitado** — não apareceu em typst (17 crates) nem em egui (12 crates). Mantido por completude do desenho, como o ADR-0005 §Ajuste 4 antecipava.

### `trait_ref` distinto vs igual

| Crate | `trait_ref` distinto | `trait_ref` igual |
|-------|---:|---:|
| egui  | 25 | 6 |
| epaint | 29 | 0 |
| emath | 20 | 0 |
| ecolor | 8 | 0 |
| egui_kittest | 4 | 1 |
| egui-wgpu | 3 | 0 |
| egui_glow | 1 | 0 |
| **Total** | **90 (92.8%)** | **7 (7.2%)** |

O `trait_ref` distingue **92.8% das colisões** — é o sinal dominante para a nomeação por trait (laudo 0015). Os **7 casos com `trait_ref` igual** (todos em `egui`) são situações onde os dois nós colidentes têm o mesmo trait nominal mas a E1 ainda decide por vizinhança disjunta. Casos plausíveis: dois `impl Default for X` em contextos `cfg` diferentes; o fork não distingue `cfg` ainda. Não investigado caso a caso.

---

## Bloco C — O que a lente diz (organização dos dados; interpretação fica com o autor)

### Distribuição de classificações no workspace (7 306 nós úteis, paths únicos)

| Classificação | Contagem | % |
|---|---:|---:|
| Isolados | 112 | 1.5 % |
| **Folhas** | **5 653** | **77.4 %** |
| Bases | 997 | 13.6 % |
| Intermediários | 544 | 7.5 % |

77 % dos nós são folhas (ninguém depende deles). É uma proporção alta. Parte disso vem do "limite Folha/comportamental" (próxima subseção).

### Folhas comportamentais (limite registrado no laudo 0020)

**1 354 nós são folhas com 0 impacto cujo nome é um método "comportamental"** (`fmt`, `from`, `into`, `default`, `clone`, `eq`, `ne`, `hash`, `cmp`, `partial_cmp`, `as_ref`, `as_mut`, `deref`, `deref_mut`, `drop`).

- 1 354 / 5 653 = **23.9% de todas as folhas são comportamentais**.
- 1 354 / 7 306 = **18.5% de todos os nós**.

São folhas estruturais corretas (ninguém *referencia* `Display::fmt` no código de aplicação — é chamado pelo formatter macro do runtime, fora do alcance do `cargo-modules`). Mas conceitualmente um humano pode dizer "se eu mexer no `Display::fmt`, quebra todo lugar que usa `{}` formatando esse tipo". O grafo estrutural não captura isso. **Quantificação do limite**: ~1 em cada 5 nós é folha comportamental.

### Top-10 do crate principal `egui` (por raio transitivo)

| Transitivo | Nó |
|---:|---|
| 2 270 | `core::option::Option` |
| 2 080 | `alloc::alloc::Global` |
| 1 905 | `alloc::string::String` |
| 1 873 | `alloc::sync::Arc` |
| 1 816 | `emath::vec2::Vec2` |
| 1 705 | `core::num::nonzero::NonZero` |
| 1 683 | `ecolor::color32::Color32` |
| 1 642 | `egui::id::Id` |
| 1 617 | `emath::rect::Rect` |
| 1 575 | `alloc::vec::Vec` |

**Sysroot domina o ranking.** Dos 10, **7 são de `core::*`/`alloc::*`** (Option, Global, String, Arc, NonZero, Vec). 3 são do próprio ecossistema egui: `Vec2`, `Color32`, `Id`, `Rect`. *Observação descritiva*: sem o filtro de stdlib (pendência mantida desde laudo 0019), os "itens centrais" do egui ficam misturados com tipos universais da linguagem, que dominam por estarem em **todo lugar**.

### Top-10 do `epaint`

Mesmo padrão — stdlib (`Global`, `Arc`, `Vec`) intercalado com `Color32`, `Vec2`, `Pos2`, `Rect`, `TextureId`, `font_types::tag::Tag`.

### Top-10 do `emath` (sem stdlib dominando)

| Transitivo | Nó |
|---:|---|
| 179 | `emath::pos2::Pos2` |
| 133 | `emath::rect::Rect` |
| 107 | `emath::vec2::Vec2` |
| 55 | `emath::align::Align` |
| 42 | `emath::align::Align2` |
| 40 | `core::result::Result` |
| 29 | `emath::range::Rangef` |

`emath` é interessante: 9/10 itens são próprios (Pos2, Rect, Vec2, Align, Align2, Rangef, etc.). É crate-base (não importa muito da stdlib), então o filtro de stdlib teria menos efeito aqui. **Pos2 com 179 impactados é coerente** — qualquer mudança no Pos2 sentido em ~1/3 dos nós do crate.

### Médias agregadas

- Média de vizinhos diretos por nó: **2.5** (soma 18 145 / n 7 306).
- Média de impacto transitivo por nó: **39.8** (soma 290 941 / n 7 306).

Médias puxadas pelas Bases e Intermediários (que têm raios grandes) e amortecidas pelos 77% de Folhas (com raio 0).

---

## Bloco D — Descobertas inesperadas

### D1 — `egui_demo_app` não roda: bin + lib no mesmo pacote

O `lente_infra::invocacao` lê o `Cargo.toml`, descobre `[package].name`, e invoca `cargo modules ... --package <nome>`. Quando o pacote tem **dois targets** (`[lib]` + `[[bin]]`, comum em apps), o fork retorna *"Multiple targets present in package, please explicitly select one via --lib or --bin flag"*. O invocador atual **não cobre esse cenário** — é o segundo modo de falha estrutural do invocador descoberto (o primeiro foi o erro do prompt 0003: "Multiple packages present in workspace", já resolvido com `--package`).

Bug-latente do invocador. Não corrigido aqui (Arena); fica registrado para prompt próprio depois.

### D2 — `ecolor` foi anomalia de tempo (cold start)

`ecolor` levou **123.1 s** sendo o **primeiro crate** rodado, com apenas 169 nós. Os subsequentes caíram drasticamente: 45 s, 19 s, 10 s, 15 s, 12 s, 12 s, 23 s, 4 s, 8 s, 3 s. O **`egui` (3 694 nós, 20× maior) levou só 23 s** — ordem de magnitude menor.

Hipótese: cold-start do rust-analyzer ao processar a primeira vez no `target/` daquele workspace (montar índices semânticos, carregar metadados). Os crates seguintes reusam estado em cache. **Implicação operacional**: a primeira invocação contra um workspace novo é cara; depois fica barato. Importante saber para qualquer UX (CLI, IDE) que use a lente — primeira chamada parece travar; depois roda em ~10 s.

### D3 — `MesmoItem` continua um caminho teórico

Predição refutada: o egui (rico em reexport) deu **zero** `MesmoItem`, igual ao typst. A explicação mais provável: o fork não modela reexport como path duplicado, mas como aresta `Uses`. Logo, o caminho `MesmoItem` no `lente_resolve` é dead code na prática contra dados reais. **Não recomenda** remover (o desenho prevê — laudo 0015 §"Limite teórico"); mas a evidência empírica reforça que é caminho raro.

### D4 — Sysroot domina rankings (filtro continua pendente)

Em todos os crates exceto o trivial `emath`, o top-10 tem itens de `core::*`/`alloc::*` ocupando 5-7 das 10 posições. Não é erro — é coerente: `Option`, `String`, `Arc`, `Vec` aparecem em **todo lugar**, então um raio transitivo deles é gigantesco. Mas para o usuário que pergunta "o que quebra se eu mexer no `egui`?", a resposta "muita coisa depende de `Option`" não é útil — `Option` está fora do controle dele.

**Pendência**: o filtro de stdlib (ADR-0002 D3 + laudo 0019 §Fica para depois) é necessário para o ranking ser interpretável. A medição confirma isso com volume.

### D5 — `trait_ref` igual em 7 casos do egui

92.8 % das colisões têm `trait_ref` distinto (sinal forte). Os 7 com `trait_ref` igual estão todos em `egui` — provavelmente `cfg`-conditional impls ou outra dimensão que o fork não distingue. A E1 ainda decide (vizinhança disjunta), então o pipeline funciona; só a nomeação por trait ficaria ambígua nesses 7. Não bloqueia, mas é o piso conhecido do mecanismo de trait-por-nó.

### D6 — Quase metade dos nós úteis do egui são "Folhas comportamentais"

18.5 % de todos os nós são folhas-comportamentais (`fmt`, `from`, `default`, etc. com raio zero). É o limite mais quantificável do que a lente *não* vê hoje. Não é bug do projeto-lente — é o que o `cargo-modules` extrai (estrutural, não chamadas via macro).

---

## Limites declarados desta medição

- **1 crate (`egui_demo_app`) não medido** por bin+lib no mesmo pacote.
- **Sysroot incluído** (`--sysroot` é política da lente). Sem o filtro futuro, o ranking mistura stdlib com itens do próprio projeto.
- **Folhas comportamentais não decompostas**: classificação por nome de método (`fmt`, etc.); não distingue `Display::fmt` de método qualquer chamado `fmt`. Aproximação suficiente para a contagem.
- **Top-10 por crate**, não pelo workspace inteiro: não foi feito ranking global. Há nós que aparecem em vários crates (`Option`, `Vec2`, `Color32`) — somá-los exigiria deduplicação cross-crate. Adiada.
- **Bloco C é descrição, não conclusão**. "A lente é útil para o egui?" é decisão do autor depois de ler este relatório.

---

## Tabela bruta por crate

| Crate | Tempo (s) | Nodes | Edges | Colisões | E1 | NaoDet | Folhas / Bases / Inter / Isolados |
|-------|---:|---:|---:|---:|---:|---:|---|
| ecolor | 123.1 | 169 | 377 | 8 | 8 | 0 | 110 / 24 / 20 / 6 |
| egui_demo_app | 0.5 | — | — | — | — | — | ERRO: bin+lib |
| egui_demo_lib | 45.6 | 702 | 1841 | 0 | 0 | 0 | ver checkpoint |
| egui_extras | 19.6 | 396 | 1111 | 0 | 0 | 0 | ver checkpoint |
| egui_glow | 10.3 | 92 | 222 | 1 | 1 | 0 | ver checkpoint |
| egui_kittest | 15.9 | 285 | 798 | 5 | 5 | 0 | ver checkpoint |
| egui-wgpu | 12.4 | 195 | 552 | 3 | 3 | 0 | ver checkpoint |
| egui-winit | 12.9 | 158 | 344 | 0 | 0 | 0 | ver checkpoint |
| **egui** | 23.2 | **3694** | 13937 | **31** | 31 | 0 | ver checkpoint |
| emath | 4.3 | 513 | 1222 | 20 | 20 | 0 | ver checkpoint |
| **epaint** | 8.3 | **1231** | 4360 | **29** | 29 | 0 | ver checkpoint |
| epaint_default_fonts | 3.7 | 1 | 0 | 0 | 0 | 0 | ver checkpoint |
| **Total** | **279.8** | **7 436** | **24 764** | **97** | **97** | **0** | 5653 / 997 / 544 / 112 |

---

## Artefatos do experimento

- `lab/medicao-egui/Cargo.toml`, `src/main.rs` — programa de medição (Arena).
- `lab/medicao-egui/checkpoints/<crate>.json` — 12 arquivos (1 por crate, incluindo o que falhou). Permitem retomar a medição sem refazer trabalho.
- `lab/medicao-egui/dados.json` — agregado completo, pronto para análises posteriores.
- `lab/medicao-egui/relatorio.md` — este documento.

---

## Histórico

| Data | Motivo |
|------|--------|
| 2026-06-01 | Primeira medição do sistema composto contra projeto externo (egui v0.34.3). 11/12 crates ok; 97 colisões, 100 % E1, 0 MesmoItem, 0 NaoDeterminado. Descobertas: `egui_demo_app` falha (bin+lib), cold-start do rust-analyzer (`ecolor` 123 s vs `egui` 23 s), sysroot domina top-10, 18.5 % de folhas-comportamentais. Conclusões qualitativas ficam com o autor. |
