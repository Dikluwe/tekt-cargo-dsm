# Prompt: Detecção de alvo por `cargo metadata` + fechar a porta `--pacote` (`lente_infra`)

**Camada**: L3 — Infraestrutura
**Criado em**: 2026-06-01
**Estado**: `PROPOSTO`
**Decisões de origem**:
- Laudo 0022, D1 — a detecção de alvo foi feita por heurística (`Cargo.toml`
  + layout `src/`) **para evitar um segundo `Command::new`**. A própria D4 do
  0022 registra o custo: a heurística pode superestimar bins em crates
  exóticos (`autobins = false`, paths customizados), virando `AlvosAmbiguos`
  por engano.
- Laudo 0022, "Pendências reforçadas" — a porta `fork::invocar_fork(pacote)`
  (o modo `lente --pacote <X>` da CLI) **não foi coberta**: sem diretório, ela
  não tinha como detectar o alvo; bin+lib por esse caminho ainda falha.
- Revisão do autor: a restrição "um só `Command::new`" do prompt 0022 era
  **larga demais**. A intenção real era "um só invocador **do fork**" (não
  reintroduzir a duplicação do laudo 0018). `cargo metadata` não é o invocador
  do fork — é descoberta de alvos, outro propósito. A regra proibiu a solução
  certa por tabela.
**Pré-requisito**: laudo 0022 (detecção de alvo via heurística; enum
`AlvoFork`; variante `ErroAdaptador::AlvosAmbiguos`).
**Posição**: correção de robustez do 0022, antes do filtro de stdlib. Continua
ortogonal ao filtro e ao ranking.
**Arquivos afetados (a confirmar na Fase 1)**: `03_infra/src/invocacao.rs`,
`03_infra/src/fork.rs`, `03_infra/src/lib.rs`, possivelmente um módulo novo
`03_infra/src/metadata.rs`, testes do `lente_infra`.

---

## Contexto

A fonte autoritativa dos alvos de um pacote é o próprio Cargo, via
`cargo metadata`. Ele entrega, por pacote, `targets[]` com `kind`
(`lib`/`rlib`/`proc-macro`/`bin`/…) e `name` — a mesma informação que o Cargo
usa internamente. Não há auto-descoberta a adivinhar: o que o `cargo metadata`
diz é o que o Cargo enxerga. A heurística do 0022 reimplementa, à mão e por
baixo, o que essa fonte responde com exatidão — e por isso pode errar.

Trocar a heurística por `cargo metadata` faz duas coisas com uma peça só:

1. **Elimina o erro da heurística** (D4 do 0022): o caso exótico que hoje
   viraria `AlvosAmbiguos` por engano deixa de existir.
2. **Fecha a porta `--pacote`**: `cargo metadata` enumera os pacotes do
   workspace a partir do cwd e responde por **nome de pacote**, sem precisar
   do diretório específico do crate. Então `fork::invocar_fork(pacote)`, que
   só tem o nome e o cwd, passa a conseguir detectar o alvo — exatamente o que
   faltava no 0022.

Custo honesto da troca: `cargo metadata` é um subprocesso (mais pesado que
abrir um arquivo; resolve o manifesto, podendo baixar o índice na primeira
vez). Exige `cargo` no PATH — já exigido, porque o fork roda por `cargo`. Para
o uso da lente (uma invocação por análise, não um laço apertado), é aceitável.

---

## Restrições estruturais

- **L3 continua L3.** Rodar `cargo metadata` é I/O legítimo aqui.
- **Invariante reformulado: um só invocador DO FORK.** O `export-json`
  (extração do grafo) continua tendo **uma única** origem
  (`fork::invocar_em`). O `cargo metadata` é um subprocesso **separado, único e
  nomeado**, com outro propósito (descobrir alvos) — não é duplicação do
  invocador do fork. Após este prompt, o crate tem dois subprocessos do cargo,
  cada um único: um `export-json` (fork) e um `metadata` (descoberta). **Esta
  é a supersessão explícita da leitura "um só `Command::new`" do laudo 0022.**
- **Aditivo para chamadores.** `fork::invocar_fork(pacote)`,
  `invocacao::invocar(diretorio)`, `extrair_grafo(diretorio)` mantêm as
  assinaturas. O modo `--pacote` **passa a funcionar** onde falhava, sem mudar
  a assinatura.
- **Não toca o `lente_core`.** Continua puro.
- **Não toca a E2 / `lente_investiga` / `fontes.rs`.** Quarentena intocada.
- **Não toca o fork (`cargo-modules`).** Usa flags e comandos que o cargo já
  tem (`--lib`/`--bin` confirmados no laudo 0022; `cargo metadata` é padrão).

---

## Fase 1 — Leitura e verificação primeiro (obrigatória)

**Antes de desenhar, ler o estado pós-0022 e verificar o `cargo metadata`
real. Não desenhar sobre suposição (mesmo princípio do 0022).**

1. **O que o 0022 deixou em `invocacao.rs`**: `detectar_alvo`,
   `parse_alvos_do_toml`, `listar_bins_dir`, `descobrir_pacote`. Quais saem,
   quais ficam (ver Fase 2). E `fork.rs`: `AlvoFork`, a assinatura de
   `invocar_em(pacote, current_dir, alvo)` e `invocar_fork(pacote)`.

2. **Verificar a saída real do `cargo metadata`** (a parte que não dá para
   assumir): rodar `cargo metadata --no-deps --format-version 1` e confirmar:
   - O caminho dos campos: `packages[].name`, `packages[].targets[].kind`
     (é uma **lista**), `packages[].targets[].name`,
     `packages[].manifest_path`. Registrar os nomes reais.
   - Os valores de `kind` para alvos de **biblioteca** (`lib`, `rlib`,
     `dylib`, `cdylib`, `staticlib`, `proc-macro`) e para **binário** (`bin`),
     e quais ignorar (`example`, `test`, `bench`, `custom-build`).
   - Como `cargo modules --lib` se comporta num crate **proc-macro** (tem alvo
     de lib, mas especial): roda? Erra? Isso decide se `proc-macro` conta como
     "tem lib" para a regra.

3. **Verificar a descoberta por nome num workspace**: rodar `cargo metadata`
   a partir da **raiz de um workspace** (caso do `lente --pacote <X>` rodado
   na raiz) e confirmar que `packages[]` lista os membros, dá para achar o
   pacote por `name`, e ler os `targets` dele — **sem** o diretório específico.
   Este é o mecanismo que fecha a porta `--pacote`.

4. **Confirmar (rápido, já visto no 0022)** que `--lib` regride pacote só-bin
   e que bin+lib sem flag falha — para manter a regra de escolha correta.

**Reportar no laudo**: nomes reais dos campos do `cargo metadata`, valores de
`kind` observados, comportamento do proc-macro, e como a descoberta por nome
funcionou a partir da raiz do workspace.

---

## Fase 2 — Conserto

### Onde mora a detecção

Um módulo/funcão de descoberta de alvos no `lente_infra` (sugestão:
`metadata.rs`), com dois pedaços separados para testabilidade:

- **Subprocesso fino**: roda `cargo metadata --no-deps --format-version 1`
  (com `--manifest-path`/`current_dir` conforme a porta) e devolve o JSON cru.
  Este é o **subprocesso único de metadata** do crate.
- **Seleção pura**: recebe o JSON (ou um struct desserializado), o nome do
  pacote-alvo, e devolve `AlvoFork` ou erro. **Sem I/O** — testável com JSON
  de amostra, como `traducao` é testado hoje. (Desserialização com `serde` é
  natural no L3.)

Essa separação espelha o padrão do crate (`fork.rs` roda o subprocesso;
`traducao` parseia) e torna a regra de escolha verificável por teste de
unidade, sem precisar de cargo no teste.

### Regra de escolha do alvo (sobre os `targets` do pacote no metadata)

| Condição (sobre `targets[].kind` do pacote) | Alvo |
|---------------------------------------------|------|
| há alvo de biblioteca (`lib`/`rlib`/`dylib`/`cdylib`/`staticlib`; proc-macro conforme Fase 1) | `AlvoFork::Lib` |
| sem lib, exatamente 1 alvo `bin` | `AlvoFork::Bin(nome)` |
| sem lib, 0 ou ≥2 alvos `bin` | `ErroAdaptador::AlvosAmbiguos { bins }` |

"Tem lib → lib" cobre bin+lib sem caso especial (a lente analisa estrutura de
biblioteca). Igual à regra do 0022 — o que muda é a **fonte** (metadata
autoritativo, não heurística de arquivos).

### As duas portas usam a mesma detecção

- `invocacao::invocar(diretorio)`: roda metadata com `--manifest-path` do
  diretório (ou `current_dir = diretorio`); acha o pacote (pela
  raiz/`manifest_path`, ou pelo nome de `descobrir_pacote` se ele ficar) e
  seleciona o alvo.
- `fork::invocar_fork(pacote)`: roda metadata no cwd; acha o pacote por
  `name == pacote`; seleciona o alvo. **Sem mudar a assinatura** — é isso que
  fecha o `--pacote`. (A sugestão `invocar_fork_em(pacote, dir)` do laudo 0022
  deixa de ser necessária: metadata por nome dispensa o diretório.)

### O que sai e o que fica (supersessão do 0022)

- **Sai**: a heurística de layout — `listar_bins_dir` e a parte de
  `parse_alvos_do_toml` que lista bins por arquivo. **Supera a D1 e a D4 do
  laudo 0022.**
- **Fica**: `ErroAdaptador::AlvosAmbiguos { bins }` (D3 do 0022) — agora
  **confiável** (sem falso-positivo da heurística). O enum `AlvoFork` (D2)
  continua `pub(crate)`.
- **Decidir na Fase 1**: `descobrir_pacote` (nome do pacote via Cargo.toml)
  ainda é útil para a porta com diretório, ou metadata também resolve o nome e
  ele sai? Se sair, garantir não-regressão dos testes que batiam nele
  (`descobre_pacote_de_cargo_toml_simples`,
  `workspace_puro_sem_package_devolve_erro_claro`) — migrando ou substituindo
  por testes equivalentes sobre o novo caminho.

### Erros

Falha do `cargo metadata` (cargo ausente, manifesto não resolve, JSON
inesperado) propaga por `ErroAdaptador`, variante nova só se necessário, com
`From` para `?`. O caso "pacote pedido não existe no workspace" (descoberta
por nome falha) merece diagnóstico próprio claro.

---

## Critérios de Verificação

```
Dado um pacote bin+lib alcançado por `fork::invocar_fork(pacote)` (modo --pacote)
Quando o invocador é chamado a partir do workspace
Então metadata acha o pacote por nome, escolhe --lib, e o fork roda (porta --pacote fechada)

Dado um pacote bin+lib alcançado por `extrair_grafo(diretorio)`
Quando o invocador é chamado
Então funciona (não-regressão do que o 0022 já cobria)

Dado um pacote só-bin único
Quando o invocador é chamado
Então metadata indica 1 bin, escolhe --bin <nome>, o fork roda

Dado um pacote só-lib
Quando o invocador é chamado
Então escolhe --lib, o fork roda

Dado um pacote com vários bins e sem lib
Quando o invocador é chamado
Então ErroAdaptador::AlvosAmbiguos { bins } com a lista correta (sem falso-positivo)

Dado um crate exótico (ex.: autobins=false) que a heurística do 0022 classificaria errado
Quando a detecção por metadata é usada
Então o alvo é o que o Cargo realmente vê (a fragilidade do 0022 está corrigida)

Dado o lente_infra após o conserto
Quando se faz grep dos subprocessos do cargo
Então há UM `export-json` (fork) e UM `metadata` (descoberta), cada um único
```

Casos a cobrir nos testes:

- **Unidade da seleção pura** (sem cargo): alimentar JSON de amostra de
  `cargo metadata` para cada caso — bin+lib → `Lib`; só-lib → `Lib`; só-bin →
  `Bin(nome)`; multi-bin sem lib → `AlvosAmbiguos`; proc-macro → conforme
  decidido na Fase 1; pacote-nome-inexistente → erro de descoberta.
- **E2E `#[ignore]`** (requer cargo + fork): manter/migrar o
  `e2e_bin_mais_lib_passa_a_funcionar` do 0022, e **adicionar** um E2E pela
  porta `--pacote` (`invocar_fork`) contra um pacote bin+lib do workspace,
  provando a porta fechada.
- **Não-regressão**: todos os testes verdes; os testes de heurística do 0022
  (`detecta_bins_em_src_bin_subdir`, etc.) são **substituídos** pelos de
  seleção pura sobre metadata — registrar quais saíram e por quê.
- **Verificação manual**: grep mostra um `export-json` e um `metadata`.

---

## Resultado esperado

- Detecção de alvo por `cargo metadata` (fonte autoritativa); heurística de
  layout removida.
- Porta `--pacote` (`fork::invocar_fork`) **fechada** para bin+lib, sem mudar
  assinatura.
- Um invocador do fork (`export-json`) e um de metadata, cada um único.
- `AlvosAmbiguos` agora confiável.
- Pureza do L1 intacta.
- **Laudo** registrando:
  - Achados da Fase 1 (campos reais do metadata, `kind`s, proc-macro,
    descoberta por nome na raiz).
  - A supersessão de D1/D4 do laudo 0022 (heurística → metadata) e o destino
    de `descobrir_pacote`.
  - As decisões de erro.
  - grep dos dois subprocessos.

---

## O que NÃO entra

- **Filtro de stdlib e modo ranking**: prompts próprios.
- **Remoção da E2**: fica em quarentena; só sai após medir outros projetos.
- **Mudança no fork (`cargo-modules`)**: nenhuma.
- **Mudança no `lente_core`**: nenhuma.
- **Cache de metadata / otimização**: não. Uma invocação por análise; não
  estruturar antes do uso pedir.

---

## Observação metodológica

Este é o padrão "superação granular em cascata" (candidato a LESSONS do Tekt):
uma restrição minha larga demais no prompt 0022 ("um só `Command::new`") →
forçou a decisão D1 (heurística) → cuja fragilidade (D4) este prompt corrige
trocando por `cargo metadata`. O downstream ficou obsoleto quando a premissa a
montante foi corrigida.

O princípio por trás da troca é o mesmo do projeto: preferir **fonte
autoritativa** a **heurística** sempre que o custo (aqui, um subprocesso a
mais) for aceitável. O `cargo metadata` responde com a verdade do Cargo; a
heurística adivinhava. Para uma ferramenta cujo lema é "verificar contra dado
real antes de afirmar", a troca é coerente — e por isso a Fase 1 verifica o
`cargo metadata` real, em vez de assumir os nomes dos campos.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | Detecção de alvo migrada de heurística (laudo 0022, D1/D4) para `cargo metadata` (fonte autoritativa); porta `--pacote` (`fork::invocar_fork`) fechada para bin+lib via descoberta por nome; invariante reformulado para "um só invocador do fork". | `03_infra/src/invocacao.rs`, `03_infra/src/fork.rs`, `03_infra/src/lib.rs`, `03_infra/src/metadata.rs` (provável), `00_nucleo/lessons/0023-l3-deteccao-alvo-metadata.md` |
