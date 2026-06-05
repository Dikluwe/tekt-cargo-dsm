# Prompt: Invocador cobre pacotes com bin+lib (`lente_infra`)

**Camada**: L3 — Infraestrutura
**Criado em**: 2026-06-01
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo 0021 (medição egui), Bloco A — 11/12 crates ok;
a única falha foi `egui_demo_app`, que tem **binário + biblioteca** e o
`cargo modules` exige `--lib` ou `--bin` para desambiguar. O invocador atual
(`fork::invocar_em`) não passa nenhuma das duas. Decisão do autor: consertar
este débito antes de iniciar o filtro de stdlib / ranking.
**Pré-requisito**: laudo 0018 (invocador consolidado — `fork::invocar_em` é a
primitiva única de subprocess; `invocacao::invocar` delega a ela).
**Posição**: débito técnico do laudo 0021 (pendência 4 da transferência). Não
depende do filtro nem do ranking; é ortogonal a eles.
**Arquivos afetados (a confirmar na Fase 1)**: `03_infra/src/fork.rs`,
`03_infra/src/invocacao.rs`, `03_infra/src/lib.rs`, testes do `lente_infra`.

---

## Contexto

O invocador monta um comando fixo:

```
cargo modules export-json --sysroot --compact --package <pacote>
```

Para um pacote que tem **só biblioteca** ou **só um binário**, o comando
resolve sozinho qual alvo analisar. Para um pacote que tem **biblioteca e
binário** (ex.: `egui_demo_app`), o `cargo modules` não sabe qual analisar e
aborta pedindo `--lib` ou `--bin <nome>`. O comando atual não passa nenhuma,
então esses pacotes falham.

A lente analisa **estrutura de biblioteca** ("o que quebra se eu mexer neste
item da lib"). Então, quando há os dois alvos, a escolha natural é a
biblioteca (`--lib`).

Importante: o conserto é **no `lente_infra`**, escolhendo a flag certa. **Não
é mudança no fork** — `--lib`/`--bin` são flags de seleção de alvo que o
`cargo modules` já oferece (a Fase 1 confirma os nomes exatos contra o
binário real).

---

## Restrições estruturais

- **L3 continua L3.** Ler `Cargo.toml` / inspecionar o diretório do crate é
  I/O legítimo aqui.
- **Subprocess único preservado (laudo 0018).** Continua havendo **um único**
  `Command::new("cargo")` no crate. A flag de alvo flui para dentro dessa
  primitiva; não se cria um segundo ponto de invocação.
- **Aditivo para chamadores.** As assinaturas públicas (`fork::invocar_fork`,
  `invocacao::invocar`, `extrair_grafo`) continuam funcionando. Pacotes que
  hoje funcionam (só-lib, só-bin) devem continuar funcionando.
- **Não toca o `lente_core`.** Continua puro.
- **Não toca a E2 / `lente_investiga` / `fontes.rs`.** A E2 permanece em
  quarentena; este prompt não a remove nem a mexe.
- **Não toca os testes do que existe** além do necessário; os pré-existentes
  passam sem modificação (ou com modificação mínima e registrada se algo
  realmente quebrar).

---

## Fase 1 — Leitura e verificação primeiro (obrigatória)

**Antes de desenhar, ler o estado real e verificar o comportamento real do
fork. Não desenhar sobre suposição.**

1. **`03_infra/src/fork.rs`**: a primitiva `invocar_em(pacote, current_dir)` —
   como monta os args, o `ErroFork`. E `invocar_fork(pacote)` (cwd, usado pelo
   wiring no modo `--pacote`).

2. **`03_infra/src/invocacao.rs`**: `descobrir_pacote(diretorio)` (parser
   linha-a-linha do `Cargo.toml`, hoje só pega `[package] name`) e
   `invocar(diretorio)` (lê o Cargo.toml e chama `invocar_em(pacote, Some(dir))`).

3. **Chamadores das duas portas**, em todo o workspace:
   `grep -rn "invocar_fork\|invocacao::invocar\|extrair_grafo" --include "*.rs"`.
   Confirmar: o wiring (`04_wiring`) usa `fork::invocar_fork` no
   `FonteGrafo::Pacote`; `extrair_grafo` usa `invocacao::invocar`; as arenas
   em `lab/` chamam quem? Saber isso é necessário para não-regressão.

4. **Verificar o comportamento real do `cargo modules` (a parte que NÃO dá
   para assumir):**
   - Os nomes exatos das flags de seleção de alvo do `export-json`
     (`cargo modules export-json --help`). Confirmar que são `--lib` e
     `--bin <nome>` (ou registrar os nomes reais).
   - A mensagem de erro exata num pacote bin+lib sem flag (rodar o comando
     atual contra um pacote bin+lib — `egui_demo_app` serve, ou um fixture
     local criado para isto).
   - Que `--lib` num pacote **só-binário** falha (e como), e que `--bin <nome>`
     num pacote **só-lib** falha — para saber o que a detecção precisa evitar.

**Reportar no laudo**: o que cada leitura/verificação encontrou, e como isso
moldou o desenho (em especial, os nomes reais das flags e o texto do erro).

---

## Fase 2 — Conserto

### Regra de escolha do alvo

A partir dos alvos do pacote no diretório:

- **Tem biblioteca** (com ou sem binário) → `--lib`. (A lente analisa estrutura
  de biblioteca; quando há os dois, a lib é a escolha.)
- **Só binário, um único** → `--bin <nome>` (ou o que a Fase 1 mostrar ser
  necessário; se o comando sem flag já resolve binário único, manter sem flag).
- **Só-lib** → `--lib` (explícito) ou inalterado, conforme a Fase 1 mostrar
  que não regride.
- **Vários binários, sem lib** → caso de borda reconhecido, **fora do escopo
  imediato**: emitir erro com diagnóstico claro listando os binários, em vez
  de adivinhar. (Coerente com o tratamento de casos fora de alcance do
  projeto, como o Limite 6.) Não resolver agora — só não falhar de forma
  obscura.

### Onde a detecção mora

A detecção precisa de um diretório (para inspecionar os alvos). As duas portas
têm diretórios diferentes:

- `invocacao::invocar(diretorio)` tem o diretório explícito.
- `fork::invocar_fork(pacote)` roda no cwd (o `current_dir` da primitiva é
  `None`); a detecção, nesse caminho, usa o cwd.

Desenho-alvo (ajustar à realidade da Fase 1):

- Uma **função de detecção** que, dado um diretório, devolve o alvo escolhido
  (sugestão de tipo: um `enum Alvo { Lib, Bin(String) }` interno ao crate).
  Pode reaproveitar / ficar ao lado de `descobrir_pacote`, que já lê o
  `Cargo.toml` do mesmo diretório.
- A escolha de alvo **flui como parâmetro para a primitiva `invocar_em`**, que
  acrescenta a flag aos args. Continua sendo o único `Command::new`.
- Como detectar os alvos: decidir na Fase 1 entre (a) inspecionar `Cargo.toml`
  (`[lib]`, `[[bin]]`) somado às convenções de arquivo (`src/lib.rs`,
  `src/main.rs`, `src/bin/*.rs`), atento à auto-descoberta do Cargo; ou (b)
  `cargo metadata --no-deps` e ler `targets[].kind`. A (b) é mais robusta mas
  é outro subprocess; a (a) é mais leve mas precisa cobrir a auto-descoberta.
  Escolher com base no que a Fase 1 revelar; registrar a escolha e a razão.

### Erros

Os modos de falha novos (não conseguiu determinar alvo; vários bins sem lib)
propagam pelos tipos de erro que já existem (`ErroFork` / `ErroAdaptador`),
adicionando variante só se necessário, com `From` para uso natural com `?`.
Decisão do gerador conforme a leitura.

---

## Critérios de Verificação

```
Dado um pacote com biblioteca E binário (ex.: egui_demo_app, ou fixture local)
Quando o invocador é chamado
Então o comando inclui --lib e o fork roda com sucesso (deixa de falhar)

Dado um pacote só-biblioteca (ex.: lente_core, ou fixture)
Quando o invocador é chamado
Então funciona como antes (não-regressão)

Dado um pacote só-binário, alvo único
Quando o invocador é chamado
Então o fork roda com sucesso

Dado um pacote com vários binários e sem biblioteca
Quando o invocador é chamado
Então erro com diagnóstico claro listando os binários (não falha obscura)

Dado o lente_infra após o conserto
Quando se faz grep por Command::new("cargo")
Então há APENAS UM (subprocess único do laudo 0018 preservado)

Dado todos os testes pré-existentes do workspace
Quando compilado e testado
Então todos verdes (108 + ignored), interface externa preservada
```

Casos a cobrir nos testes:

- **Unidade, da detecção de alvo** (espelhar o estilo dos testes de
  `descobrir_pacote`, que montam diretórios temporários): `Cargo.toml`
  + layout `src/` para cada caso — bin+lib → `Lib`; só-lib → `Lib`; só-bin →
  `Bin(nome)`; vários bins sem lib → erro claro.
- **E2E `#[ignore]`** (requer fork instalado) contra um pacote bin+lib real
  (o `egui_demo_app` da medição), confirmando que agora produz JSON.
- **Não-regressão**: todos os testes existentes verdes; os `#[ignore]` do
  fork (0017) ainda válidos.
- **Verificação manual**: grep do `Command::new` único.

---

## Resultado esperado

- O invocador escolhe o alvo (`--lib` / `--bin <nome>`) conforme os alvos do
  pacote; pacotes bin+lib passam a funcionar.
- As duas portas (`invocacao::invocar` com diretório; `fork::invocar_fork` no
  cwd) cobertas.
- Subprocess único preservado.
- Caso "vários bins sem lib" com diagnóstico claro, sem resolução automática.
- Pureza do L1 intacta (este prompt não toca o L1).
- **Laudo** registrando:
  - O que a Fase 1 encontrou (assinaturas, chamadores, e principalmente os
    nomes reais das flags e o texto do erro do `cargo modules`).
  - A escolha do método de detecção (Cargo.toml+layout ou `cargo metadata`) e
    a razão.
  - As decisões de erro.
  - Verificação grep do subprocess único.
  - Estado dos casos de borda (vários bins sem lib).

---

## O que NÃO entra

- **Filtro de stdlib e modo ranking**: prompts próprios, depois desta decisão.
- **Remoção da E2**: fica em quarentena; só sai após medir outros projetos.
- **Mudança no fork (`cargo-modules`)**: nenhuma — usa flags que ele já tem.
- **Mudança no `lente_core`**: nenhuma.
- **Resolver o caso "vários binários sem lib"**: só diagnosticar com clareza,
  não escolher por adivinhação.

---

## Observação metodológica

Este prompt tem uma Fase 1 com **verificação contra o binário real** (flags e
erro do `cargo modules`), não só leitura de código. A razão é o princípio do
projeto: o laudo 0012 afirmou capacidade do fork sem verificar o JSON real e o
laudo 0013 o refutou. Aqui o desenho da detecção depende de qual flag o fork
aceita e de como ele falha — fatos que só o binário real responde. Verificar
antes é barato (um `--help` e uma invocação contra um pacote bin+lib) e evita
desenhar sobre suposição.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | Invocador passa a escolher alvo (--lib/--bin) conforme os alvos do pacote, cobrindo pacotes bin+lib (débito do laudo 0021, Bloco A). Subprocess único preservado. | 03_infra/src/fork.rs, 03_infra/src/invocacao.rs, 03_infra/src/lib.rs, testes |
