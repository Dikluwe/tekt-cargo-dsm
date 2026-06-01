# Laudo de Execução — Prompt 0021 (Medição da Lente contra o egui)

**Camada**: L5 (laudo) — registro de experimento de Arena
**Data**: 2026-06-01
**Prompt executado**: `00_nucleo/prompt/0021-medicao-egui.md`
**Estado**: `EXECUTADO` — primeira medição do sistema composto contra projeto
externo; relatório completo em `lab/medicao-egui/relatorio.md`.

---

## O que o prompt pediu

Primeira medição do sistema composto (pós-laudo 0020) contra um projeto
**externo ao próprio projeto-lente** — o workspace egui v0.34.3. Quatro
blocos de perguntas: (A) sistema funciona em projeto não-trivial?,
(B) predições da resolução de colisões se confirmam?, (C) o que a lente
diz é útil (organização dos dados; interpretação do autor)?, (D)
descobertas inesperadas.

---

## O que foi gerado

| Artefato | Local |
|----------|-------|
| Programa de medição (Arena) | `lab/medicao-egui/{Cargo.toml, src/main.rs}` |
| Checkpoints por crate | `lab/medicao-egui/checkpoints/<crate>.json` × 12 |
| Dados agregados | `lab/medicao-egui/dados.json` |
| **Relatório completo (conteúdo)** | **`lab/medicao-egui/relatorio.md`** |

Este laudo é o **registro** da execução no `lessons/`; o **conteúdo** vive no
relatório do `lab/` (padrão dos experimentos de Arena: dados brutos no lugar
deles, laudo aqui referencia).

---

## Resumo dos achados (sumário; detalhe no relatório)

### Bloco A — Sistema funciona?

- **11/12 crates ok**, tempo total **279.8 s** (~4 min 40 s, bem abaixo da
  estimativa de 1-2 h).
- **1 erro**: `egui_demo_app` falhou — `cargo modules` exige `--lib`/`--bin`
  para pacotes com bin+lib; o invocador da lente não cobre esse caso.
- Sem panic, sem erro de pipeline ao longo dos 11 crates restantes.
- 7 436 nós, 24 764 arestas, 97 colisões agregadas.

### Bloco B — Predições

| Predição | Resultado |
|----------|-----------|
| E1 ≥ 95 % | **100 %** (97/97) — confirmada com folga |
| `ImplDeTraitsDiferentes` = 0 | 0 (E2 em quarentena, esperado) |
| `MesmoItem` > 0 (reexport) | **0 — refutada** |
| `NaoDeterminado` baixo | 0 (zero macros-patológicos) |

`trait_ref` distinto em **92.8%** das colisões — sinal dominante para nomeação.

### Bloco C — O que a lente diz (organização, sem conclusão qualitativa)

- Distribuição: 77.4 % Folhas, 13.6 % Bases, 7.5 % Intermediários, 1.5 % Isolados.
- **18.5 % de todos os nós são "folhas comportamentais"** (`fmt`/`from`/`default`/etc. com raio zero) — quantificação do limite do laudo 0020.
- **Top-10 do egui (crate principal) dominado por sysroot**: 7 dos 10 são `core::*`/`alloc::*` (Option, String, Arc, Vec, NonZero...). 3 são próprios: Vec2, Color32, Id, Rect.
- `emath` (crate-base com poucas deps stdlib) tem top-10 quase todo próprio: Pos2, Rect, Vec2, Align.

### Bloco D — Descobertas

1. **`egui_demo_app` falha** — invocador não cobre múltiplos targets no mesmo pacote (após laudo 0003 cobrir múltiplos pacotes no workspace).
2. **Cold-start do rust-analyzer**: `ecolor` (primeiro crate, 169 nós) levou **123 s**; `egui` (3 694 nós, 20× maior) só **23 s**. Implicação UX futura.
3. **`MesmoItem` permanece caminho teórico** em dados reais — typst (17 crates) + egui (12 crates) = 29 crates, zero ocorrências.
4. **Sysroot domina rankings** — o filtro de stdlib (pendência registrada desde o laudo 0019) é necessário para top-N ser interpretável.
5. **7 colisões com `trait_ref` igual em egui** — piso do mecanismo trait-por-nó (provável `cfg`-conditional impls).
6. **18.5 % de folhas-comportamentais** — limite do `cargo-modules` (não vê chamadas via formatter macro, derive expansion, etc.) é grande no egui.

---

## Conclusões qualitativas: ficam com o autor

Conforme metodologia do prompt (e do laudo 0005): a medição **apresenta**
("os 10 itens com maior raio transitivo são X, Y, Z"); **não conclui** ("a
lente é útil para o egui"). A conclusão qualitativa (utilidade real para
um desenvolvedor do egui, próximos passos do projeto) fica com o autor
após leitura do relatório.

---

## Pendências reforçadas pela medição (não resolvidas aqui)

1. **Invocador para bin+lib**: o `lente_infra::invocacao` não cobre
   `cargo modules ... --lib/--bin` quando o pacote tem ambos targets.
   Bug-latente; prompt próprio depois.
2. **Filtro de stdlib**: pendência desde laudos 0013 D1 / 0019. A medição
   mostra com volume por que vale construir (top-10 dominado por stdlib
   "afoga" os itens próprios do projeto).
3. **`MesmoItem` é caminho teórico mesmo** — duas medições contra dados
   reais (29 crates total) sem ocorrências reforçam o que o ADR-0005
   §Ajuste 4 já antecipava.

---

## Por que este laudo é mais curto que os de componente

Padrão: experimentos de Arena (0005, 0007, 0009, 0011, 0021) têm o **conteúdo
denso no `lab/`** (relatório próprio com dados, tabelas, gráficos textuais);
o laudo em `lessons/` é o **registro de que a execução aconteceu**, com
sumário e ponteiro. Diferente dos laudos de componente nucleado (0001-0004,
0006, 0008, 0010, 0012-0020), que registram decisões tácitas e contratos —
ali, o laudo **é** o registro principal.

(Esse padrão emergiu na pergunta do usuário: "por que não foi salvo em
nucleo/lessons?" — para tornar o índice histórico em `lessons/` completo
mesmo para experimentos. Antes desta lesson, os experimentos de Arena
viviam só no `lab/`, sem entrada no índice. Adotada nova convenção a partir
desta.)

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | Primeira medição do sistema composto contra projeto externo (egui v0.34.3). 11/12 crates ok; 97 colisões, 100% E1, 0 MesmoItem, 0 NaoDeterminado. Conteúdo completo em `lab/medicao-egui/relatorio.md`. Padrão novo: experimentos de Arena também ganham entrada em `lessons/` (registro), mantendo conteúdo bruto em `lab/`. | `00_nucleo/lessons/0021-medicao-egui.md` (este registro); conteúdo em `lab/medicao-egui/` |
