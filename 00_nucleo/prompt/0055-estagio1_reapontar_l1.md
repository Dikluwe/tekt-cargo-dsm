# Prompt: refactor V3+V12, Estágio 1 — re-apontar os L1-origem (mecânico)

**Camada**: L2 (`02_shell/cli` + o `Cargo.toml` dela) — no `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0055-estagio1_reapontar_l1.md`)
**Pré-requisito**: 0054 (mapa). **Decisões fechadas**: vocabulário →
`lente_core::domain::consulta` (Estágio 2); ponto de entrada → crate app no
`04_wiring` (Estágio 3). **Nenhuma das duas afeta este estágio.**
**Objetivo**: re-apontar, na CLI, os **6 símbolos L1-origem** que hoje vêm pela
fachada `lente_wiring` para os crates L1 de origem. **Mecânico, preserva
comportamento** (mesmo tipo, outro caminho de import).
**Delta esperado: V3 8 → 4. V12 5 → 5 (inalterado).**

---

## Contexto

O 0054 confirmou: 6 dos símbolos que a CLI importa do `lente_wiring` **nascem no
L1** e o wiring só os **re-exporta** (`pub use`). Re-apontar a CLI direto para o
L1 limpa os sítios que **só** importam esses — sem mover nada. É o passo seguro,
antes dos que movem tipos (Estágio 2) e relocam o ponto de entrada (Estágio 3).

---

## O que fazer

1. **Re-apontar** em `02_shell/cli`, trocando `lente_wiring` pelo crate L1 de
   origem, para os 6 símbolos:
   | Símbolo | Novo caminho (origem L1) |
   |---|---|
   | `ResultadoDiff`, `TocadoComRaio`, `RaioCombinado` | `lente_core` (`domain::resultado_diff`) |
   | `Fantasma` | `lente_core` (`domain::uniao`) |
   | `Ciclo` | `lente_estrutura` |
   | `ItemRanking` | `lente_ranking` |
   Usar o **caminho público canônico** que cada crate expõe (o `lente_core` pode
   re-exportar na raiz ou só no módulo — usar o que ele de fato expõe).
2. **Adicionar ao `02_shell/cli/Cargo.toml`** as deps `path` dos crates L1 agora
   importados direto que ainda **não** sejam dep (`lente_core`, `lente_estrutura`,
   `lente_ranking`). L2→L1 é permitido. Caminhos relativos pós-0050 (da pasta
   `02_shell/cli`): `../../01_core/core`, `../../01_core/estrutura`,
   `../../01_core/ranking`. O `cargo build` é a guarda.
3. **Sítios mistos** (`saida.rs:16`, `saida.rs:971`): **dividir** o import — a
   parte (i) vem do L1; a parte (ii) (`Escopo`/`EstruturaModulos`/`ModoUses`;
   `DependenciaModulo`) **continua** vindo do `lente_wiring` por ora (sai no
   Estágio 2). Esses dois sítios **permanecem** como V3 — esperado.
   **Sítios só-(i)** (`saida.rs:1142`, `1227`, `1306`, `1321`): a linha inteira vai
   para o L1; o import do `lente_wiring` desaparece desses.
4. **Não tocar**: o vocabulário (ii) (`FonteGrafo`/`AlvoBusca`/`Escopo`/`ModoUses`/
   `EstruturaModulos`/`DependenciaModulo`), o `ErroLente`, nem as **chamadas de
   função** de orquestração — são Estágios 2 e 3.
5. **Deixar as re-exportações (`pub use`) no `lente_wiring`** como estão — limpar é
   opcional, depois; não é deste estágio.
6. **Verificar**: `cargo build` + suíte **273 + 28** (idêntica — comportamento
   preservado) + `crystalline-lint` re-rodado (mesmos `--checks`): **V3 = 4**,
   **V12 = 5**, V8/V4/V9/V13 = 0.

---

## O que NÃO fazer

- Mover tipos, tocar o `lente_wiring` (as re-exportações ficam), mexer no
  `ErroLente` ou nas chamadas de função — Estágios 2/3.
- Remover as re-exportações do wiring (opcional, depois).
- Mudar qualquer lógica — é só caminho de import.

---

## Critérios de Verificação

```
Dado os 6 símbolos L1-origem
Quando re-apontados na CLI
Então vêm do crate L1 de origem (lente_core/lente_estrutura/lente_ranking),
não mais do lente_wiring

Dado o cli/Cargo.toml
Então tem as deps path dos crates L1 importados direto; cargo build passa

Dado os sítios
Então os só-(i) (1142,1227,1306,1321) não importam mais do wiring; os mistos
(16,971) têm a parte (i) no L1 e a (ii) ainda no wiring

Dado a suíte
Então 273 + 28 — comportamento idêntico (mesmo tipo, outro caminho)

Dado crystalline-lint (mesmos --checks)
Então V3 = 4 (8→4), V12 = 5 (inalterado), V8/V4/V9/V13 = 0
```

---

## Resultado esperado

- Os imports re-apontados (quais linhas, de → para).
- As deps `path` adicionadas ao `02_shell/cli/Cargo.toml`.
- `cargo build` ok + suíte **273 + 28** + `crystalline-lint` (**V3 = 4**, V12 = 5).
- **Laudo** em `00_nucleo/lessons/0055-…`: os re-apontamentos, as deps, o build/
  suíte, o delta do lint.

---

## Cuidados

- **Caminho público canônico** de cada crate L1 — o `lente_core` pode expor na raiz
  ou só no módulo; usar o que ele expõe (não inventar caminho).
- **Caminhos relativos das deps** no `Cargo.toml` (pós-0050: `../../01_core/<crate>`)
  — `cargo build` é a guarda.
- **Os sítios mistos ficam parcialmente** — é esperado; a parte (ii) sai no
  Estágio 2. Não tentar limpá-los à força agora.
- **Comportamento idêntico** — a suíte 273 + 28 é a prova; se mudar, algo além de
  re-apontar mudou.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Estágio 1 do refactor V3+V12 (mapa do 0054; decisões: vocabulário→`lente_core::domain::consulta`, ponto de entrada→crate app no `04_wiring` — nenhuma afeta este estágio). Re-apontados na CLI (`02_shell/cli`) os 6 símbolos L1-origem que vinham pela fachada `lente_wiring`: `ResultadoDiff`/`TocadoComRaio`/`RaioCombinado` e `Fantasma` → `lente_core`; `Ciclo` → `lente_estrutura`; `ItemRanking` → `lente_ranking` (caminho público canônico de cada). Adicionadas ao `cli/Cargo.toml` as deps `path` dos crates L1 importados direto (`../../01_core/{core,estrutura,ranking}` pós-0050). Sítios só-(i) (`saida.rs:1142/1227/1306/1321`) deixam de importar do wiring; mistos (`saida.rs:16`, `saida.rs:971`) com a parte (i) no L1 e a (ii) ainda no wiring (sai no Estágio 2). **Mecânico, preserva comportamento** (mesmo tipo, outro caminho): suíte 273 + 28 inalterada. Re-exportações do `lente_wiring` deixadas como estão (limpeza opcional, depois). Não tocados: vocabulário (ii), `ErroLente`, chamadas de função (Estágios 2/3). Delta: **V3 8→4**, V12 5 (inalterado), V8/V4/V9/V13 = 0. | `02_shell/cli/src/*.rs` (imports), `02_shell/cli/Cargo.toml` (deps); `00_nucleo/lessons/0055-...` |
