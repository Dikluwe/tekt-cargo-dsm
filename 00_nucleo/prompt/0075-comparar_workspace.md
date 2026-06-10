# Prompt: `--comparar` ciente de workspace (detecção de lado + chave de path completo)

**Camada**: L4 — Fiação (detecção e orquestração) + L1 — Núcleo (`lente_core`,
a chave de pareamento) + L2 — CLI (declaração do modo na saída)
**Criado em**: 2026-06-10
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0075-comparar_workspace.md`)
**Número**: confirmar na Fase 1 o próximo número livre em `00_nucleo/prompt/`
(este documento assume a sequência após o 0074; o 0075 estava reservado
informalmente para a tela lado a lado, que NÃO é este prompt — renumerar a
tela ou este, conforme a convenção do diretório).
**Decisões de origem**: execução real do `--comparar` contra o par
typst-vanilla vs typst-cristalino (2026-06-10) — falhou na detecção de alvo
porque ambos os lados são **workspaces virtuais** (`[workspace]` sem
`[package]`) e o 0074 extrai cada lado como crate único
(`lente_infra::extrair_grafo`). Segundo achado da mesma execução, por
análise: a normalização do pareamento do 0074 (descartar o nome do crate)
assume um crate por lado e **deixa de ser injetiva** num grafo de workspace
(dois crates podem ter submódulos de mesmo nome → colisão de chave).
**Pré-requisito**: 0045 (`montar_grafo_workspace`, provado no repo real:
421 nós, 0 fantasmas); 0044 (`enumerar_membros` + extração cacheada);
0074 (`--comparar` crate-único, o contrato de sem-par declarado).
**Posição**: pré-requisito mecânico de qualquer comparação no par typst, e
de qualquer pareamento futuro por identidade de item (a trilha de "movido
por similaridade" que o 0074 deferiu). Este prompt NÃO é essa trilha.
**Arquivos afetados (a confirmar na Fase 1)**: o módulo do `--comparar` no
`lente_wiring` (a função que extrai cada lado — o 0074 a chama de algo como
`estrutura_de_raiz`; confirmar o nome real), o pareamento no `lente_core`,
a saída texto/JSON na L2, testes dos três.

---

## Contexto

O `--comparar` (0074) recebe duas raízes e compara as estruturas de módulos.
Hoje, cada raiz é extraída como **um crate** (`extrair_grafo`), e o
pareamento normaliza descartando o nome do crate — desenho correto para o
caso que o motivou (crate renomeado, egui→egui).

O primeiro par real do caso de origem do projeto (typst vanilla vs typst
cristalino) é workspace contra workspace: 21 crates de um lado, camadas
Tekt do outro. A execução falha na detecção de alvo antes de qualquer
pareamento. E, mesmo que rodasse, a normalização atual colidiria chaves
entre crates.

A peça que falta já existe: `montar_grafo_workspace` (0045) enumera os
membros, extrai cada crate com cache, resolve por crate e une num grafo só
— é o que o `--diff` usa. Este prompt liga essa peça ao `--comparar` e
ajusta a chave de pareamento para o modo workspace.

**Contrato declarado de antemão (a ressalva, não a surpresa)**: no modo
workspace, a chave é o **path completo** (com o primeiro segmento, o
crate). Só pareia o que não mudou nem de crate nem de path. Numa
reorganização profunda como a do typst-cristalino, isso satura em sem-par —
e esse número é o dado que o prompt existe para produzir: o tamanho real
dos dois lados e a medida de quão profunda foi a reorganização. Dar sinal
sob movimento de path é a trilha de identidade de item, deferida (de novo)
de propósito.

---

## Restrições estruturais

- **L1 — o pareamento continua puro.** A escolha da chave (normalizada ou
  path completo) entra como dado/parâmetro da função de pareamento, não
  como I/O. `cargo tree -p lente_core` continua só o crate.
- **L4 — a detecção de lado é composição.** Decidir "crate único ou
  workspace" lendo o `Cargo.toml` da raiz é I/O — mora em L3 se precisar de
  função nova, ou reusa o que `enumerar_membros`/`descobrir_pacote` já
  oferecem (Fase 1 decide; registrar).
- **Retrocompat total do modo crate-único.** O caso egui→egui (dois
  diretórios-de-crate) continua funcionando exatamente como no 0074, com a
  mesma normalização (descartar o crate). Os testes do 0074 são a guarda.
- **Sem flag nova na CLI.** `--comparar --antes --depois` não muda de
  assinatura; a detecção é automática por lado. A saída passa a **declarar**
  o modo de cada lado.

---

## Fase 1 — Leitura primeiro (obrigatória)

Antes de desenhar, ler o estado real:

1. O módulo do `--comparar` no wiring: o nome real da função que extrai um
   lado, como a normalização está implementada, onde o pareamento mora
   (L1? L4?).
2. O contrato de saída atual (texto e JSON): quais campos existem, para a
   extensão ser aditiva.
3. `montar_grafo_workspace` (0045): a assinatura real, o `GrafoWorkspace`
   (grafo + fantasmas).
4. Como detectar a natureza da raiz com o que já existe: `descobrir_pacote`
   falha em workspace puro com erro claro
   (`workspace_puro_sem_package_devolve_erro_claro`, teste citado no 0023)
   — talvez a detecção seja "tentar pacote; nesse erro específico, é
   workspace". Alternativa: ler o `Cargo.toml` e decidir por presença de
   `[package]`. Escolher o mais simples que não dependa de casar string de
   erro frágil; registrar.
5. Confirmar o próximo número livre de prompt e ajustar o título.

---

## O que mudar

### 1. Detecção por lado (L4, com apoio de L3)

Para cada raiz (`--antes` e `--depois`, independentes):

| `Cargo.toml` da raiz | Modo do lado | Extração |
|---|---|---|
| tem `[package]` | crate único | caminho atual do 0074 (inalterado) |
| só `[workspace]` | workspace | `montar_grafo_workspace(raiz)` |
| nenhum / ausente | erro claro | mensagem citando a raiz e o que faltou |

Os lados podem ter modos **diferentes** (um crate contra um workspace é
válido).

### 2. Chave de pareamento (L1)

| Combinação de modos | Chave |
|---|---|
| crate × crate | normalizada (descarta o crate) — **igual ao 0074** |
| qualquer lado workspace | **path completo** (com o crate) |

Razão literal: a normalização só é injetiva com um crate por lado. Com
workspace em qualquer lado, descartar o primeiro segmento pode colidir
chaves; o path completo é injetivo por construção (invariante da forma
resolvida/unida).

### 3. Fantasmas como sinal (não descartar)

`montar_grafo_workspace` devolve fantasmas. O `--comparar` não os usa no
pareamento, mas a saída **reporta a contagem por lado** (e a lista no
JSON). Descartá-los silenciosamente esconderia um sinal que o 0045 definiu
como dado, não erro.

### 4. Saída declara o modo (L2)

Texto e JSON ganham, por lado: o modo (`crate` / `workspace`), o número de
crates do lado (1 ou N), e a chave de pareamento em uso (`normalizada` /
`path_completo`). Strings novas no catálogo (ADR-0002). Campos JSON
aditivos — o esquema do 0074 não perde nada.

---

## O que NÃO muda

- O pareamento e a saída do modo crate × crate (egui→egui) — bit a bit.
- `montar_grafo_workspace`, `enumerar_membros`, o cache (0044/0045) —
  usados como estão.
- O contrato honesto do 0074: sem-par declarado dos dois lados, sem
  inferência de "movido".
- O modo `--diff` e o modo global da CLI.

---

## Critérios de Verificação

```
Dado dois diretórios-de-crate (fixtures do 0074)
Quando --comparar roda
Então a saída é idêntica à do 0074 (modo crate×crate, chave normalizada)

Dado uma raiz com [workspace] sem [package]
Quando o lado é detectado
Então o modo é workspace e a extração usa montar_grafo_workspace

Dado uma raiz sem Cargo.toml
Quando o lado é detectado
Então erro claro citando a raiz (não pânico, não falha obscura do fork)

Dado dois grafos de workspace forjados, com crates A e B de um lado tendo
ambos um submódulo "ast", e o outro lado tendo só A::ast
Quando o pareamento roda com chave de path completo
Então A::ast pareia com A::ast e B::ast fica sem-par (sem colisão de chave)

Dado um lado crate e um lado workspace
Quando o pareamento roda
Então a chave é path completo nos dois lados (declarado na saída)

Dado o mesmo par de lados
Quando --comparar roda duas vezes
Então a mesma saída (determinístico)

Dado o workspace da lente como --antes E como --depois (#[ignore], fork real)
Quando --comparar roda
Então todos os módulos pareiam e os dois sem-par são vazios
E os fantasmas reportados são 0 dos dois lados (laudo 0045)

Dado o JSON do modo workspace
Quando desserializado
Então os campos do 0074 estão presentes e os novos (modo, crates, chave,
fantasmas) são aditivos
```

Casos puros (sem fork): a chave por combinação de modos; a colisão evitada
pelo path completo; determinismo do pareamento. Casos L3/L4: a detecção
pelas três formas de `Cargo.toml`. E2E `#[ignore]`: lente vs lente
(identidade), e não-regressão das fixtures do 0074.

---

## Resultado esperado

- `--comparar` roda sobre workspaces (qualquer combinação de modos por
  lado), com a chave de pareamento correta para cada combinação e o modo
  declarado na saída.
- Retrocompat bit a bit do caso crate×crate.
- **Laudo** em `00_nucleo/lessons/`:
  - O mecanismo de detecção escolhido (e por que não casar string de erro,
    se foi o caso).
  - A chave por combinação de modos, com o caso de colisão evitada.
  - O resultado do E2E lente vs lente (identidade: 100% pareado).
  - **A primeira rodada real contra o par typst** (vanilla vs cristalino):
    módulos por lado, pareados, sem-par por lado, fantasmas por lado,
    tempo frio e morno. Este número é o propósito do prompt — se a rodada
    não couber no laudo, registrar onde ficou.
  - Contagem da suíte (não-regressão).

---

## O que NÃO entra

- **Pareamento por identidade de item / detecção de movido por
  similaridade** — a trilha que dá sinal sob reorganização. Decisão
  separada, tomada com o número deste prompt na mão.
- **A tela lado a lado** — lê o JSON deste prompt depois.
- **Normalizações novas** (folha do path, sufixos, similaridade de nome) —
  nenhuma; só as duas chaves da tabela.
- **Mudanças no fork, no cache, na união (0045), no `--diff`.**
- **Filtro de stdlib no comparar** — se o dado real mostrar ruído de
  sysroot no censo dos lados, registrar no laudo; conserto é decisão
  posterior.

---

## Observação metodológica

O caso egui validou o 0074 e ao mesmo tempo escondeu duas suposições
(lado = crate único; normalização injetiva) que o primeiro par real
derrubou em 0,07 s. O conserto reusa peça provada (0045) em vez de
construir extração nova, e mantém o contrato de honestidade do 0074: a
saturação esperada de sem-par no par typst é declarada no contrato como o
dado a produzir, não como defeito a maquiar. A decisão cara (identidade de
item) fica para depois do número existir — "medir antes de procurar
solução".

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Proposta: `--comparar` detecta a natureza de cada lado (crate vs workspace), usa `montar_grafo_workspace` (0045) para lados-workspace, e troca a chave de pareamento para path completo quando há workspace (a normalizada deixa de ser injetiva). Retrocompat crate×crate; fantasmas reportados; modo declarado na saída. | a confirmar na Fase 1 |
