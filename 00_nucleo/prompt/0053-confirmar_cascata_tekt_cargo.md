# Prompt: confirmar a cascata no `tekt-cargo-dsm` — linter consertado + remover a whitelist do 0050

**Camada**: transversal (config + verificação) — no `tekt-cargo-dsm`.
**Criado em**: 2026-06-06
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0053-confirmar_cascata_tekt_cargo.md`)
**Pré-requisito**: 0052 (conserto landado e commitado no clone do `tekt-linter`);
0050 (a whitelist dos seis `lente_*` no `crystalline.toml`).
**Objetivo**: provar, no projeto real, que o conserto do 0052 (a) faz o **V14
zerar sem a whitelist** (os `lente_*` resolvem para L1; o `Kind` some), (b) torna
o **V3 = 0 significativo** (capaz de pegar cross-crate), e (c) deixa o **V9** agora
ciente de cross-crate — a vigiar. **Único artefato mexido: `crystalline.toml`**
(remover os seis `lente_*`). **Nada de código, cabeçalho ou estrutura.**

---

## Contexto

O 0052 consertou o linter a montante (o `resolve_layer` ciente das dependências
reais). A cascata foi provada num fixture descartável. Falta prová-la no projeto
real e **liquidar o débito que o 0050 abriu de propósito**: a whitelist dos seis
`lente_*` no `[l1_allowed_external]`, que era contorno para o falso positivo do
V14 — agora desnecessária.

---

## O que fazer

1. **Instalar/buildar o linter consertado** do clone local onde o 0052 landou
   (`cargo install --path …` ou o binário). **Registrar a versão/commit** (para
   não confundir com a v0.1.0 do 0049/0050).
2. No `tekt-cargo-dsm`, **remover os seis `lente_*`** do `[l1_allowed_external]`
   do `crystalline.toml` — deixar `rust = []` (ou só externos reais, se houver
   algum legítimo no L1). Trocar o comentário: a whitelist saiu porque o 0052
   tornou a classificação ciente de first-party (não é mais preciso autorizar dep
   intra-L1 à mão).
3. **Rodar** `crystalline-lint .` com os **mesmos `--checks` do 0049/0050**
   (`v1,v2,v3,v4,v8,v9,v10,v11,v12,v13,v14` — sem v5/v6/v7 por causa do abort do
   V7). Comparação maçã-com-maçã.
4. **Confirmar e reportar** (delta vs 0050):
   - **V14 = 0** sem a whitelist — os `lente_*` resolvem para L1 (não Unknown), o
     `Kind` não emite import. Se sobrar V14, é externo **real** — reportar
     (esperado 0; o L1 é puro).
   - **V3 = 0, agora significativo** — capaz de pegar direção entre crates. Se
     **não** for 0, é **achado real**: uma violação de direção cross-crate antes
     escondida — reportar (não mascarar).
   - **V9** — agora ciente de cross-crate (o fio do 0052). **Vigiar.** Se disparar,
     é achado real de disciplina de porta (um crate importando fundo num subdir de
     membro L1 que não é porta declarada em `[l1_ports]`) — reportar; pode pedir
     ajuste do `[l1_ports]` ou conserto real. Se 0, a disciplina de porta vale
     cross-crate.
   - **V8 = 0** — a reestrutura do 0050 segue.
   - **V1 = 40, V12 = 5** — seguem (fora do escopo; decisões à parte).
5. O **código e a suíte do `tekt-cargo-dsm` NÃO mudam** (só o `crystalline.toml`)
   — `cargo build`/`test` inalterados (273 + 28); não precisa re-rodar, só
   constatar que nenhum `.rs` foi tocado.

---

## O que NÃO fazer

- Mexer em código, cabeçalho ou estrutura — só o `crystalline.toml`.
- Re-adicionar a whitelist se o V14 zerar (o objetivo é removê-la). Se o V14
  **não** zerar sem ela, **não** recolocar para mascarar — reportar que algo do
  conserto não pegou.
- Calar um V9 real (a disciplina de porta cross-crate agora é visível).

---

## Critérios de Verificação

```
Dado o linter consertado (0052) instalado
Então a versão/commit está registrada (distinta da v0.1.0)

Dado os seis lente_* removidos do [l1_allowed_external]
Quando crystalline-lint . (mesmos --checks do 0049/0050)
Então V14 = 0 SEM a whitelist (cascata provada no projeto real; whitelist liquidada)

Dado o V3
Então 0 e agora significativo — ou o achado real, se uma violação cross-crate
surgir (reportada, não mascarada)

Dado o V9 (agora ciente de cross-crate)
Então 0, ou o achado de porta reportado

Dado V8
Então 0 (reestrutura do 0050 preservada)

Dado o código
Então intocado — só o crystalline.toml mudou; suíte segue 273 + 28
```

---

## Resultado esperado

- A **versão/commit** do linter consertado.
- O `crystalline.toml` com a whitelist removida (`rust = []`, comentário trocado).
- O **estado do lint**: V14 = 0 (sem whitelist), V3 = 0 (significativo) ou o
  achado, V9 (0 ou achado de porta), V8 = 0; V1 = 40 e V12 = 5 seguem.
- O **veredito**: a cascata do 0052 confirmada no projeto real; a whitelist do
  0050 liquidada.
- **Laudo** em `00_nucleo/lessons/0053-…`: a versão, a mudança no config, o estado
  do lint (delta vs 0050), e o veredito.

---

## Cuidados

- **O V9 agora morde cross-crate** — se disparar, é real; reportar, não calar. Pode
  ser sinal honesto de que um subdir interno de membro L1 está sendo importado por
  fora sem ser porta.
- **A whitelist sai de vez** — se o V14 não zerar sem ela, o conserto do 0052 não
  pegou em algum caso; reportar isso, **não** recolocar a whitelist para mascarar.
- **Mesmos `--checks` do 0049/0050** — para o delta ser comparável.
- **Só o `crystalline.toml`** — nenhum `.rs`, nenhum cabeçalho, nenhuma pasta.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Confirmação da cascata do 0052 no projeto real e liquidação do débito do 0050. Instalado o linter consertado do clone local (versão/commit registrada, distinta da v0.1.0). Removidos os seis `lente_*` do `[l1_allowed_external]` do `crystalline.toml` (`rust = []`; comentário trocado — o 0052 tornou a classificação ciente de first-party, dispensando a autorização manual de dep intra-L1). Rodado `crystalline-lint .` com os mesmos `--checks` do 0049/0050. Esperado e reportado: **V14 = 0** sem a whitelist (os `lente_*` resolvem para L1, o `Kind` não emite import) — cascata provada, whitelist liquidada; **V3 = 0 significativo** (ou achado real de direção cross-crate, se surgir); **V9** vigiado (agora ciente de cross-crate pelo 0052 — achado de porta reportado se disparar, não calado); **V8 = 0** (reestrutura do 0050 preservada); V1 = 40 e V12 = 5 seguem (fora do escopo). Código e suíte intocados (só o `crystalline.toml`; 273 + 28 inalterado). | `crystalline.toml`; `00_nucleo/lessons/0053-...` |
