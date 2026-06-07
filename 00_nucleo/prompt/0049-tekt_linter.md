# Prompt: rodar o `crystalline-lint` (tekt-linter) sobre o projeto — verificação

**Camada**: transversal (o projeto inteiro) — verificação arquitetural
**Criado em**: 2026-06-06
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0049-tekt_linter.md`)
**Pré-requisito**: 0037→0048 commitados.
**Ferramenta**: `crystalline-lint` — o linter arquitetural da Cristalina/Tekt
(`https://github.com/Dikluwe/tekt-linter`). Verifica V0–V14: camadas, direção de
import, pureza do L1, linhagem dos prompts, cobertura de teste, sem import do
`lab`, etc.
**Objetivo**: **verificar** o tekt-cargo-dsm contra as regras da Cristalina;
levantar e categorizar **todas** as violações; avaliar cada uma (real /
configuração / formato de cabeçalho / estrutural). **Não corrigir em massa** —
reportar o estado, decidir depois.
**Arquivos afetados**: `crystalline.toml` (criar/ajustar para a estrutura real —
necessário para a rodada fazer sentido); o laudo. **Nada** de cabeçalho de
linhagem nem reestruturação nesta passada (decidido após o relatório).

---

## Contexto

A trilha local está completa e commitada. O `crystalline-lint` verifica se o
projeto segue a arquitetura Cristalina — os mesmos princípios que guiaram cada
prompt (L1 puro, direção de dependência, linhagem, quarentena do `lab`). Esta
rodada **verifica** o estado; não é um conserto em massa.

---

## O ponto central a descobrir: os vários crates L1

O `crystalline.toml` mapeia **um diretório por camada** (`L1 = "01_core"`). Mas o
tekt-cargo-dsm tem **seis** crates L1: `01_core`, `05_investiga`, `06_resolve`,
`07_filtro`, `08_ranking`, `09_estrutura` (mais `02_shell`/`03_infra`/`04_wiring`).
Os crates `05`–`09` estão **fora** de `01_core`.

**Determinar** como o linter os trata:
- O `[layers]` aceita uma **lista** (`L1 = ["01_core", "05_investiga", …]`)? Se
  sim, mapear os seis e seguir.
- Se **não** aceita lista, os `05`–`09` disparam **V8 (AlienFile, fatal)** — e
  isso é um **achado estrutural** (a estrutura multi-crate-L1 do projeto vs o
  modelo de-um-diretório-L1 do linter). Reportar; **não** reestruturar o projeto
  para caber no linter sem decisão.

Esta é metade da verificação — registrar exatamente o que acontece.

---

## O que fazer

1. **Instalar** o `crystalline-lint`:
   - `cargo install --git https://github.com/Dikluwe/tekt-linter crystalline-lint`,
     **ou** o binário de release (`.../releases/latest/download/crystalline-lint-linux-x86_64`).
   - **Registrar a versão** (`crystalline-lint --version`).
2. **Garantir o `crystalline.toml`** na raiz do tekt-cargo-dsm:
   - Se já existe, **usar e revisar** (registrar o conteúdo relevante).
   - Se não, **criar** mapeando a estrutura real:
     - `[layers]`: `L0=00_nucleo`, `L1` = os crates L1 (ver o ponto central acima —
       lista se aceita; senão registrar o V8), `L2=02_shell`, `L3=03_infra`,
       `L4=04_wiring`, `lab=lab`.
     - `[excluded]`: `target`, `.git`, `.cargo`.
     - `[l1_allowed_external]`: **vazio** para Rust — o L1 do projeto é puro (sem
       dep externa, princípio do projeto); deixar vazio faz o **V14 confirmar** a
       pureza (se disparar, é achado real).
     - `[module_layers]`, `[l1_ports]`, `[orphan_exceptions]`: conforme a estrutura
       (ver o ponto sobre o caminho dos prompts abaixo).
3. **Rodar** `crystalline-lint .` (formato texto). Capturar **todas** as violações
   (o texto lista tudo; o exit code é secundário).
4. **Reportar** todas as violações por **ID (V0–V14)** e por **arquivo**, com
   contagem.
5. **Avaliar** cada grupo (ver "Categorias" abaixo).

**Não** corrigir em massa: nem adicionar/reescrever cabeçalhos, nem reestruturar,
nem mexer no código para calar o linter. O config entra só para a rodada fazer
sentido. Os consertos são decididos **após** o relatório.

---

## Categorias para a avaliação

Para cada violação (ou grupo), classificar:

- **Real (arquitetural)** — um problema genuíno de camada/pureza/dependência: V3
  (import proibido), V4 (I/O no L1), V9 (vazamento de porta), V10 (import do
  `lab`), V11 (contrato sem impl), V13 (estado mutável no L1), V14 (externo no
  L1). Se algum disparar, é achado a corrigir — **sinalizar** (o projeto afirma
  esses invariantes; o linter os testa).
- **Configuração** — o config não mapeou algo certo; ajustar o `crystalline.toml`
  e re-rodar.
- **Formato de cabeçalho / linhagem** — V1 (cabeçalho `@prompt` ausente), V5/V6
  (hash/snapshot), V7 (prompt órfão). Provável fonte de muito ruído se a linhagem
  do projeto **não** estiver no formato exato do `crystalline-lint`
  (`//! Crystalline Lineage / @prompt / @prompt-hash / @layer / @updated`), **ou**
  se o caminho dos prompts divergir (ver abaixo). **Reportar**; não reescrever
  cabeçalhos em massa sem decisão.
- **Estrutural** — o ponto dos vários crates L1 (V8 nos `05`–`09`, se a lista não
  for aceita).

---

## O caminho dos prompts (provável fonte de V1/V5/V7)

O `crystalline-lint` espera os prompts em `00_nucleo/prompts/` (plural) e os
cabeçalhos referenciam `@prompt 00_nucleo/prompts/<nome>.md`. O tekt-cargo-dsm usa
`00_nucleo/prompt/` (singular) e `00_nucleo/lessons/`. **Verificar** se isso faz o
V1 (cabeçalho referenciando prompt inexistente), o V5 ou o V7 disparar, e
registrar — é desalinhamento de convenção, não defeito de código. Não renomear
diretórios nem reescrever cabeçalhos sem decisão.

---

## O que NÃO fazer

- Corrigir em massa (cabeçalhos, reestruturação) — isto é verificação.
- Reestruturar o projeto (mover `05`–`09`) para caber no linter sem decisão.
- Reescrever ou adicionar cabeçalhos de linhagem em massa.
- Mexer no código de produção para calar o linter.
- (O config é a única coisa a criar/ajustar, e só para a rodada fazer sentido.)

---

## Critérios de Verificação

```
Dado o tekt-cargo-dsm
Quando instalar o crystalline-lint
Então a versão fica registrada

Dado o crystalline.toml (existente ou criado)
Então mapeia a estrutura real, e o tratamento dos crates 05–09 está registrado
(lista aceita, ou V8 com o achado estrutural)

Dado crystalline-lint .
Então produz um relatório de violações; todas ficam registradas por ID e arquivo,
com contagem

Dado cada grupo de violação
Então está classificado: real / configuração / formato-de-cabeçalho / estrutural

Dado a passada
Então NENHUM cabeçalho foi reescrito em massa, NENHUM diretório renomeado, NENHUM
código mexido para calar o linter (só o config)
```

---

## Resultado esperado

- A **versão** do `crystalline-lint`.
- O **`crystalline.toml`** usado (existente ou criado), com destaque para como os
  **vários crates L1** foram mapeados (lista, ou V8 nos `05`–`09`).
- O **relatório completo** de violações: por ID (V0–V14), por arquivo, com
  contagem.
- A **avaliação** por grupo (real / configuração / formato / estrutural).
- **Achados reais** (se houver V3/V4/V9/V10/V11/V13/V14) destacados — são os que
  importam (o projeto afirma esses invariantes).
- O estado do **caminho dos prompts** (`prompt` vs `prompts`) e seu efeito em
  V1/V5/V7.
- Um **plano proposto** do que corrigir (cabeçalhos? config? violações reais? a
  estrutura?), para **você decidir** — não executado aqui.
- **Laudo** em `00_nucleo/lessons/0049-…`: a versão, o config, o relatório, a
  avaliação, o plano.

---

## Cuidados

- **Verificação, não conserto.** Reportar o estado; os consertos são decididos
  depois. Só o `crystalline.toml` entra agora (e só para a rodada valer).
- **Vários crates L1 é o provável crux** — descobrir se o `[layers]` aceita lista;
  se não, o V8 nos `05`–`09` é o achado estrutural (o modelo do linter vs a
  estrutura do projeto). **Não** mover os crates sem decisão.
- **Formato de linhagem / caminho dos prompts** — se o V1/V5/V7 inundar, é
  provável desalinhamento de convenção (`prompt` vs `prompts`, ou o formato do
  cabeçalho), **não** defeito de código. Reportar, não reescrever em massa.
- **Achados reais valem** — se V4 (I/O no L1), V10 (import do `lab`), V3 (import
  entre camadas) ou V14 (externo no L1) dispararem, são problemas genuínos: o
  projeto afirma o L1 puro, a quarentena do `lab`, a estratificação — o linter os
  testa. Destacar.
- **Sem corrigir o código** para calar o linter nesta passada.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Verificação arquitetural: instalar e rodar o `crystalline-lint` (tekt-linter, V0–V14) sobre o tekt-cargo-dsm. Instala (cargo `--git` ou binário de release; registra a versão); garante o `crystalline.toml` para a estrutura real — **crux**: o `[layers]` mapeia um diretório por camada, mas o projeto tem seis crates L1 (`01_core`, `05_investiga`, `06_resolve`, `07_filtro`, `08_ranking`, `09_estrutura`) — determinar se aceita lista, senão registrar o V8 (AlienFile) nos `05`–`09` como achado estrutural. `[l1_allowed_external]` vazio (o L1 é puro — V14 confirma). Roda `crystalline-lint .`, captura **todas** as violações, reporta por ID e arquivo com contagem, e classifica cada grupo (real arquitetural / configuração / formato-de-cabeçalho / estrutural). Provável ruído de V1/V5/V7 por desalinhamento de convenção (`00_nucleo/prompt` singular vs `prompts` plural do linter, ou o formato do cabeçalho `//! Crystalline Lineage`) — reportar, **não** reescrever em massa. **Verificação, não conserto**: só o `crystalline.toml` é criado/ajustado; nenhum cabeçalho reescrito, nenhum diretório renomeado, nenhum código mexido. Achados reais (V3/V4/V9/V10/V11/V13/V14) destacados. Plano de conserto proposto para decisão. | `crystalline.toml`, `00_nucleo/lessons/0049-...` |
