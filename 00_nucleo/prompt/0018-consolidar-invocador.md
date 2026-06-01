# Prompt: Consolidar `invocacao.rs` e `fork.rs` (`lente_infra`)

**Camada**: L3 — Infraestrutura
**Criado em**: 2026-06-01
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo 0017 D1 — duas funções no `lente_infra` invocam
o fork (interfaces diferentes, mas subprocess duplicado). Decisão do autor:
consolidar antes do L4 wiring nascer sobre o desenho duplo.
**Pré-requisito**: laudo 0017 (`fork.rs` criado).
**Posição**: passo de consolidação entre o L3 (prompt 0017) e o L4 (próximo).
**Arquivos afetados**: `03_infra/src/fork.rs`, `03_infra/src/invocacao.rs`
(provavelmente — verificar nome real), `03_infra/src/lib.rs`, testes.

---

## Contexto

Hoje o `lente_infra` tem **dois caminhos** que invocam o fork via subprocess:

- `invocacao.rs` (laudo 0003, antigo): recebe `&Path` (diretório de um
  crate), lê o `Cargo.toml`, descobre o nome do pacote, invoca o fork.
  Usado por `extrair_grafo(caminho_crate)`.
- `fork.rs` (laudo 0017, novo): recebe `&str` (nome do pacote), invoca o
  fork direto, sem ler `Cargo.toml`. Criado para o modo `--pacote` da CLI.

Os dois rodam o mesmo comando (`cargo modules export-json --sysroot
--compact --package X`). Se o comando do fork mudar, há dois lugares a
ajustar. Eliminar a duplicação **antes** do L4 nascer evita que o L4
consuma duas funções quase iguais.

---

## Restrições estruturais

- **L3 continua L3.** Subprocess é I/O legítimo.
- **Aditivo do ponto de vista de chamadores externos.** Qualquer chamador
  de fora do `lente_infra` (incluindo o `lab/medicao-colisoes/remedicao` da
  Arena, se ainda chamar a `extrair_grafo`) deve continuar funcionando. A
  refatoração é interna ao `lente_infra`.
- **Não toca o `lente_core`.** Continua puro.
- **Não toca os testes do que existe.** Os testes pré-existentes devem
  continuar passando sem modificação (ou com modificação mínima e
  registrada se algo realmente quebrar).

---

## Fase 1 — Leitura primeiro (obrigatória)

**Antes de consolidar, ler o estado real:**

1. **`03_infra/src/invocacao.rs`** completo: assinatura pública, tipo de erro
   (`ErroAdaptador`? Outro?), como descobre o nome do pacote a partir do
   `Cargo.toml`, como invoca o subprocess.

2. **`03_infra/src/fork.rs`** (acabou de ser criado, laudo 0017): a
   assinatura, `ErroFork`, como invoca.

3. **Chamadores de `invocacao::invocar`** em todo o workspace: `grep -r
   "invocacao::invocar\|invocar\(" --include "*.rs"`. Quem depende dessa
   função? `extrair_grafo`, certamente. Mais alguém? (Inclui testes,
   `lab/`.) Importante saber para garantir não-regressão.

4. **A função `extrair_grafo`**: como ela usa `invocacao::invocar` hoje?
   Pega o JSON e desserializa, presumivelmente. O detalhe interessa para
   a consolidação.

Reportar (no laudo, depois): o que foi encontrado em cada um, e como isso
afetou o desenho da consolidação.

---

## Fase 2 — Consolidar

Com a leitura na mão, o desenho-alvo é:

### Primitiva única: `fork::invocar_fork(pacote)`

Esta é a função baixo-nível que executa o subprocess. Continua como o
laudo 0017 a definiu: recebe `&str`, devolve `Result<String, ErroFork>`.
**Esta é a única função que roda `Command::new("cargo")` no crate.**

### Função de mais alto nível: descobrir pacote + invocar

A função que hoje existe em `invocacao.rs` (recebe diretório, descobre
pacote, invoca fork) passa a ser uma camada acima: ela **lê o `Cargo.toml`**
para descobrir o nome do pacote, e depois **chama `fork::invocar_fork`**
em vez de duplicar o subprocess. O nome dela depende do que existe hoje —
sugestão: manter o nome atual (se for `invocacao::invocar`, fica
`invocacao::invocar`), só mudar o corpo.

Estrutura sugerida (ajustar à realidade observada na Fase 1):

```
fork::invocar_fork(pacote: &str)        // primitiva — roda subprocess
    ↑
invocacao::invocar(diretorio: &Path)    // alto nível — lê Cargo.toml,
    ↑                                    //   delega ao fork::invocar_fork
extrair_grafo(caminho_crate: &Path)     // composição — invocacao + traducao
```

### Tipo de erro

Decisão de design (do gerador, com base na leitura): a função de alto nível
(`invocacao::invocar`) precisa propagar erros do fork **e** erros próprios
(falha ao ler `Cargo.toml`, falha ao extrair o nome do pacote dele). Duas
opções, escolher a mais natural com base no que existe:

- **Embrulhar `ErroFork` em `ErroAdaptador`** (se `ErroAdaptador` for o
  erro hoje de `invocacao`): adicionar variante `ErroAdaptador::Fork(ErroFork)`,
  com `From` impl. Mantém o contrato externo.
- **Ou** consolidar num único `enum` que cubra os dois casos, se o erro
  hoje for menor que isso. Depende do que está lá.

A escolha aqui depende totalmente do que a Fase 1 revelar. **Não decidir
sem ler.**

### Não duplicar mais

Após a consolidação:
- Existe **um único `Command::new("cargo")...export-json...`** no crate.
- Se o comando do fork mudar (nova flag), há **um lugar** para atualizar.
- Os dois pontos de entrada (`fork::invocar_fork` para o modo `--pacote`,
  `invocacao::invocar` para `extrair_grafo`) continuam, mas o segundo
  delega ao primeiro.

---

## Critérios de Verificação

```
Dado o workspace pós-laudo 0017 (com fork.rs e invocacao.rs duplicando subprocess)
Quando a consolidação é aplicada
Então existe APENAS UM Command::new("cargo") no lente_infra (grep confirma)

Dado `invocacao::invocar(diretorio)` chamado como antes (assinatura preservada)
Quando executado num diretório de crate válido
Então funciona igual antes (não-regressão de `extrair_grafo`)

Dado `fork::invocar_fork(pacote)` chamado como no laudo 0017
Quando executado
Então funciona igual (não-regressão dos testes ignored do 0017)

Dado os chamadores externos pré-existentes (extrair_grafo, lab/, etc.)
Quando o workspace é compilado e testado
Então todos verdes — interface externa preservada
```

Casos a cobrir:
- Não-regressão de TODOS os testes pré-existentes (74+ verdes, 4 ignored).
- Verificação manual: grep mostra um único subprocess do fork.
- Se foi necessário mudar algum chamador externo, registrar — mas
  preferencialmente nenhum.

---

## Resultado esperado

- `fork::invocar_fork` é a primitiva única do subprocess.
- `invocacao::invocar` (ou nome equivalente) lê `Cargo.toml`, descobre o
  pacote, delega ao `fork::invocar_fork`. Sem `Command::new` próprio.
- Erros propagados de forma natural (decisão do gerador conforme leitura).
- Workspace verde, testes ignored do fork ainda funcionando.
- **Pureza do L1** preservada.
- **Laudo** registrando:
  - O que a Fase 1 encontrou (assinaturas, erros, chamadores).
  - As decisões de design da consolidação (tipo de erro, nome da função de
    alto nível, qualquer ajuste descoberto).
  - Verificação grep do subprocess único.
  - Sinalização para o L4 (próximo prompt): qual função o L4 usa em cada
    modo da CLI (`--pacote` e `--grafo`).

---

## O que NÃO entra

- **L4 wiring**: o próximo prompt, agora sobre o desenho consolidado.
- **Mudança no fork**: nenhuma.
- **Mudança no `lente_core`**: nenhuma.
- **Mudança em chamadores externos**: nenhuma se possível; se necessária,
  registrar.

---

## Observação metodológica

Esta é uma refatoração de "tirar duplicação" que segue uma descoberta tardia
(a D1 do laudo 0017 revelou o `invocacao.rs` ao autor). O método correto é
**ler antes de mexer** — daí a Fase 1 obrigatória. Mexer sem ler é como o
laudo 0012 afirmou capacidade do fork sem verificar JSON real: gera erro
que se propaga.

A Fase 1 é barata (leitura de dois arquivos pequenos e um grep), e o ganho
em desenho correto compensa.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | Consolidação de invocacao.rs + fork.rs no lente_infra: fork::invocar_fork como primitiva única de subprocess, invocacao::invocar delega ao fork. Eliminação da duplicação antes do L4 wiring. | 03_infra/src/fork.rs, 03_infra/src/invocacao.rs, 03_infra/src/lib.rs |
