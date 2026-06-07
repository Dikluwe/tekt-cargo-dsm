# Prompt: refactor V3+V12, Estágio 2 — mover o vocabulário L4-nativo puro para o L1

**Camadas tocadas**: **L1** (`lente_core`: novo módulo `consulta`; `lente_estrutura`:
+2 tipos), **L4** (`lente_wiring`: deixa de definir, importa do L1), **L2** (a CLI:
re-aponta a parte (ii)). No `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0056-estagio2_mover_vocabulario_l1.md`)
**Pré-requisito**: 0055 (Estágio 1; V3 = 4, V12 = 5).
**Decisão fechada**: os 4 enums de pedido → `lente_core::domain::consulta`.
**Objetivo**: mover os **6 tipos L4-nativos puros** para o L1. Preserva
comportamento (mesmos tipos, outro crate).
**Delta esperado: V3 4 → 1** (sobra só `ErroLente`). **V12 5 → 1** (sobra só
`ErroLente`).

---

## Contexto

O 0054 classificou estes 6 como **L4-nativos puros** (só dependem do L1) e mapeou
seus dependentes: **só a CLI e as assinaturas do `lente_wiring`** — ripple contido.
Movê-los ao L1 resolve a parte (ii) do V3 e tira **4 dos 5 enums** do V12. Sobra só
o `ErroLente` — que **fica** no L4 (agrega L3) e sai da CLI no Estágio 3.

---

## O que fazer

1. **Criar a casa no L1:**
   - **`01_core/core/src/domain/consulta.rs`** com `FonteGrafo`, `AlvoBusca`,
     `Escopo`, `ModoUses` (movidos do `lente_wiring`). Declarar `pub mod consulta;`
     no `domain` do `lente_core`.
   - **`EstruturaModulos`, `DependenciaModulo`** → **`lente_estrutura`** (junto do
     `Ciclo`, que eles referenciam).
   - **CRÍTICO — L1 é puro**: os tipos movidos carregam **só derives da std**
     (`Debug`/`Clone`/`PartialEq`/`Eq`/…), **NENHUM derive externo** (`serde`,
     etc.). O projeto serializa **à mão**; a derive de serde **não** desce ao L1. Se
     algum dos 6 tiver `#[derive(Serialize/Deserialize)]` ou outro externo, **parar
     e reportar** — não arrastar a dep externa para o L1. A prova: **V4 e V14
     ficam 0** após.
2. **Remover as definições** do `lente_wiring` (`04_wiring/src/lib.rs`).
3. O **`lente_wiring` passa a IMPORTAR** esses tipos do L1 (nas assinaturas das 5
   funções: `calcular_raio_de_alvo`, `rankear_pacote`, `analisar_estrutura`, etc.).
   As re-exportações (`pub use`) que sobrarem sem uso: **tirar** as órfãs (o
   `cargo build`/`clippy` aponta), ou deixar — opcional, não é o foco.
4. **Re-apontar a parte (ii) restante na CLI** (`main.rs:18`, `saida.rs` ~`20` e
   ~`976` pós-0055) para o L1: os 4 enums de `lente_core::domain::consulta`;
   `EstruturaModulos`/`DependenciaModulo` de `lente_estrutura`. **Esses sítios
   LIMPAM** (deixam de importar do `lente_wiring`).
5. **Deps no `cli/Cargo.toml`**: promover `lente_estrutura` de **dev-dep** para
   **dep regular** se `EstruturaModulos`/`DependenciaModulo` forem usados em
   **não-teste** (o `saida.rs` top-level usa) — ajustar conforme o uso real.
6. **Verificar**: `cargo build` + suíte **273 + 28** (mesmos tipos, movidos —
   comportamento idêntico) + `crystalline-lint` (mesmos `--checks`): **V3 = 1** (só
   `erro.rs`/`ErroLente`), **V12 = 1** (só `ErroLente`), **V4/V14 = 0** (L1 puro),
   V8/V9/V13 = 0.

---

## O que NÃO fazer

- Arrastar `serde` (ou qualquer externo) para o L1 junto dos tipos — é o erro a
  evitar; se um tipo precisa de serde, a derive **não** vai junto (parar/reportar).
- Tocar o `ErroLente` (fica L4) ou o ponto de entrada (Estágio 3).
- Mudar lógica — é movimento de tipo, não de comportamento.

---

## Critérios de Verificação

```
Dado o L1
Então consulta.rs no lente_core tem os 4 enums; EstruturaModulos/DependenciaModulo
no lente_estrutura; nenhum carrega derive externo (V4/V14 = 0)

Dado o lente_wiring
Então não define mais esses 6; importa-os do L1 nas assinaturas; cargo build passa

Dado a CLI
Então a parte (ii) vem do L1; main.rs:18 e os sítios de saida limpam (não importam
mais do wiring)

Dado a suíte
Então 273 + 28 — comportamento idêntico (mesmo tipo, outro crate)

Dado crystalline-lint
Então V3 = 1 (só ErroLente), V12 = 1 (só ErroLente), V4/V14 = 0, V8/V9/V13 = 0
```

---

## Resultado esperado

- `consulta.rs` (4 enums) + os 2 tipos no `lente_estrutura`; as remoções no
  `lente_wiring`.
- Os imports re-apontados no `lente_wiring` (assinaturas) e na CLI.
- As deps ajustadas no `cli/Cargo.toml` (promoção de `lente_estrutura` se preciso).
- `cargo build` ok + suíte **273 + 28** + `crystalline-lint` (**V3 = 1, V12 = 1**,
  V4/V14 = 0).
- **Laudo** em `00_nucleo/lessons/0056-…`: os movimentos, a confirmação de pureza
  (sem derive externo), o build/suíte, o delta do lint.

---

## Cuidados

- **L1 puro é a guarda-mestra** — os tipos movidos só com derives da std; serde é
  sinal de **parar** (a serialização do projeto é à mão). **V4 = 0 e V14 = 0** é a
  prova de que nada externo desceu.
- **O ripple é contido** (CLI + wiring, por 0054) — se um **outro** crate quebrar no
  `cargo build`, é um dependente que o mapa não viu; **reportar** (não emendar às
  cegas).
- **`lente_estrutura` pode virar dep regular** (não-teste) — ajustar conforme o uso.
- **Comportamento idêntico** — a suíte 273 + 28 é a prova; se mudar, algo além de
  mover tipo mudou.
- **Não tocar o `ErroLente`** — ele fica no L4; a CLI deixa de precisar dele só no
  Estágio 3 (relocação do ponto de entrada).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Estágio 2 do refactor V3+V12 (mapa 0054; decisão: os 4 enums de pedido → `lente_core::domain::consulta`). Movidos os 6 tipos L4-nativos puros para o L1: `FonteGrafo`/`AlvoBusca`/`Escopo`/`ModoUses` → `lente_core::domain::consulta` (novo módulo); `EstruturaModulos`/`DependenciaModulo` → `lente_estrutura` (junto do `Ciclo`). Removidas as definições do `lente_wiring` (`04_wiring/src/lib.rs`), que passa a importá-los do L1 nas assinaturas das 5 funções; re-exportações órfãs limpas (opcional). CLI re-aponta a parte (ii) (`main.rs:18`, `saida.rs` ~20/~976) para o L1 — esses sítios limpam. `lente_estrutura` promovido a dep regular se usado em não-teste. **Crítico/preserva comportamento**: os tipos descem ao L1 **puro**, só com derives da std (sem `serde` — a serialização é à mão); **V4/V14 = 0** é a prova. Suíte 273 + 28 inalterada (mesmos tipos, outro crate). Não tocados: `ErroLente` (fica L4) e o ponto de entrada (Estágio 3). Delta: **V3 4→1**, **V12 5→1** (sobra só `ErroLente` nos dois). | `01_core/core/src/domain/consulta.rs` (novo) + `mod.rs`; `01_core/estrutura/src/*` (+2 tipos); `04_wiring/src/lib.rs` (remoções + imports); `02_shell/cli/src/*.rs` + `Cargo.toml`; `00_nucleo/lessons/0056-...` |
