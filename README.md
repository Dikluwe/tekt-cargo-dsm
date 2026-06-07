# Lente de Forma e Consequência

Uma ferramenta de análise de dependências para Rust que responde a uma pergunta:
**"o que quebra se eu mexer aqui?"**

A lente lê um programa Rust como um grafo dirigido de dependências e responde
sobre ele de três formas — o impacto de mexer num item, o ranking dos itens por
impacto, e a estrutura de arquitetura do projeto inteiro (dependências entre
módulos e ciclos, na forma de uma matriz DSM). É, na prática, um
**Lattix / Structure101 para Rust**.

---

## Por que existe

Ler um programa grande inteiro é impraticável — para um humano e para a janela de
contexto de uma IA. A lente **comprime** o programa a uma forma que cabe na
decisão: milhares de itens viram dezenas de módulos, que viram o bloco que de
fato importa. Cada passo joga fora o detalhe que não serve à pergunta e mantém o
que serve.

O valor dela vem de operar sobre estrutura **resolvida**, não sobre texto. Onde
o `grep` acha onde um nome aparece, a lente sabe **o que de fato depende de quê**:
a relação é **dirigida** (A depende de B é diferente de B depende de A), a
identidade é **resolvida** (dois itens de mesmo nome em módulos diferentes não se
confundem) e o alcance é **transitivo** (quem depende de quem depende de X). Uma
ordem topológica ou um ciclo de módulos não saem de nenhuma ferramenta de texto,
porque texto não tem direção nem resolução.

---

## O que ela faz hoje

- **Raio** — dado um item, quem depende dele (o alcance de mexer nele), com uma
  classificação (folha, base, intermediário, isolado).
- **Ranking** — os itens do crate ordenados por impacto.
- **Estrutura (DSM)** — a dependência módulo→módulo do projeto, os **ciclos**
  (componentes fortemente conexos), e a **ordem** que põe as dependências de um
  lado da diagonal e os ciclos como blocos densos nela. Sai em texto e em JSON;
  um protótipo de tela renderiza a matriz.

Dois recortes, ambos com a **saída declarando qual está em uso**:

- **Escopo** (`--filtrar-stdlib`): o seu código apenas, ou o seu código com a
  stdlib. Por padrão inclui a stdlib; a flag filtra.
- **Modo de `uses`** (`--so-referencia`): contar todas as dependências, ou só as
  **referências de tipo** (descartando os imports no nível do módulo). Por padrão
  conta todas; a flag mostra só o acoplamento de tipo genuíno.

O segundo recorte revela algo concreto. No `egui`, a vista de ciclos mostra um
anel de 85 módulos — 76% do crate. Ao contar só referências, o anel cai para
**42 módulos**: a outra metade era acoplamento aparente, vindo de declarações
`use` no topo dos módulos, não de uso real de tipo. A lente separa o acoplamento
real do inflado, e diz qual você está vendo.

---

## Como funciona

```
código Rust
   │
   ▼  (fork do cargo-modules, sobre rust-analyzer)
grafo cru, resolvido e dirigido  ──── export-json
   │
   ▼  (lente: L3 lê o JSON → forma organizada; L1 resolve colisões de path)
grafo resolvido (paths únicos)
   │
   ├── raio (quem depende de um item)
   ├── ranking (itens por impacto)
   └── estrutura: agregar a módulos → detectar ciclos → ordenar a DSM
```

A fonte do grafo é um **fork do `cargo-modules`** que usa o motor do
rust-analyzer — o mesmo que uma IDE usa — para resolver nomes, tipos e traits. É
de lá que vêm a direção e a identidade resolvida. A lente é independente da fonte:
ela mira uma "forma organizada" do grafo (definida em
`00_nucleo/specs/forma-organizada.md`), e tudo acima dessa forma — raio, ranking,
ciclos, DSM — não sabe que linguagem a produziu.

---

## O método: Tekt

O projeto é construído sob uma metodologia própria, e os artefatos do método são
tão centrais quanto o código:

- **Camadas** — `L0` semente (specs, ADRs) · `L1` núcleo puro (sem dependências
  externas) · `L2` shell (CLI, catálogo de mensagens) · `L3` infra (o fork, o
  `cargo metadata`) · `L4` fiação · `L5` laudos.
- **Cada componente nasce de um prompt numerado** (`00_nucleo/prompt/`). Cada
  execução produz um **laudo** (`00_nucleo/lessons/`) que registra o que foi
  feito, o que foi verificado, e o que ficou em aberto. Decisões de projeto viram
  **ADRs** (`00_nucleo/adr/`).
- **Medir antes de afirmar.** Hipótese sem dado é só hipótese. Experimentos e
  medições descartáveis vivem em `lab/` (a "Arena") e são jogados fora depois de
  ensinarem — o que fica é o achado, registrado num laudo.
- **Honestidade sobre os limites** é parte da spec, não nota de rodapé.

---

## Limites declarados

A lente é uma ferramenta de **forma**, não de **comportamento**. Saber os limites
evita esperar dela o que ela não dá:

- **Estrutural, não comportamental** — a lente vê o que um item **referencia**
  (chama, menciona em assinatura, implementa), não o que o código **faz em
  execução**. Um grafo de chamadas em tempo de execução está fora do alcance, por
  construção.
- **Só Rust, e só código que compila** — depende da resolução do rust-analyzer.
  Código quebrado ou outra linguagem, ela não lê.
- **Precisa do fork instalado** e usa a stdlib carregada (`--sysroot`) por
  padrão, porque metade das arestas de alguns crates vem de `#[derive(...)]` e
  sumiriam sem ela.
- Outros limites medidos (fronteira com a stdlib, agregação de imports,
  reexports, colisões geradas por macro) estão documentados em
  `00_nucleo/specs/forma-organizada.md`.

---

## Estrutura do repositório

| Caminho | O que é |
|---------|---------|
| `00_nucleo/` | Semente: `specs/`, `adr/`, `prompt/`, `lessons/` (laudos). |
| `01_core/` | L1 — tipos do grafo (`No`, `Aresta`, `Grafo`) e cálculo do raio. |
| `02_shell/catalogo/`, `02_shell/cli/` | L2 — mensagens e a CLI (`lente`). |
| `03_infra/` | L3 — invocação do fork, `cargo metadata`, desserialização. |
| `04_wiring/` | L4 — fiação que liga a CLI ao núcleo. |
| `05_investiga/`, `06_resolve/` | L1 — investigação e resolução de colisões de path. |
| `07_filtro/` | L1 — filtros de grafo (stdlib, só-referência). |
| `08_ranking/` | L1 — ranking por impacto. |
| `09_estrutura/` | L1 — agregação a módulos, detecção de ciclos, ordenamento da DSM. |
| `lab/` | Arena — protótipos e medições descartáveis. |

A identidade de um nó é o `id` (atribuído pela fonte); o `path` pode colidir
entre nós distintos, e a colisão é resolvida pela cascata `investiga → resolve`.

---

## Uso

> Os comandos abaixo são ilustrativos; confirme a sintaxe exata na ajuda da CLI
> (`lente --help`). A lente roda o fork no diretório de trabalho atual, então é
> executada **de dentro do crate alvo** (ou apontando para ele).

Instale o fork (a fonte do grafo):

```bash
cargo install --path <caminho-do-fork-cargo-modules>
```

Construa a lente e rode a partir do crate que você quer analisar:

```bash
# na raiz deste workspace
cargo build --release -p lente_cli

# de dentro do crate alvo
cd /caminho/para/o/crate
/caminho/para/lente --pacote <nome-do-crate> --ranking --top 10 --text
```

Exemplos:

```bash
# ranking de impacto, só o seu código
lente --pacote meu_crate --ranking --top 10 --filtrar-stdlib

# raio de um item específico
lente --pacote meu_crate --alvo meu_crate::modulo::Item --text

# estrutura de arquitetura: ciclos + ordem da DSM, só acoplamento de tipo
lente --pacote meu_crate --estrutura --so-referencia --text

# a mesma estrutura como dado, para outra ferramenta consumir
lente --pacote meu_crate --estrutura --so-referencia --json
```

---

## Para agentes de IA

A saída `--json` é contexto estrutural **comprimido e verificável** — um agente
que vai mexer no código pode consumir o `--estrutura --json` (módulos,
dependências, ciclos, ordem da DSM) sem precisar ler o programa inteiro, e o que
ele cita é um fato derivado do grafo, não um palpite. A lente não compete com o
`grep` que o agente já tem; ela oferece a camada resolvida e dirigida que o
`grep` não alcança. (Empacotá-la como ferramenta plugável — MCP — ainda não foi
feito; o JSON já é o insumo.)

---

## Estado

- **Vista de arquitetura (global)** — completa: do grafo cru à matriz DSM, no
  terminal e em protótipo de tela.
- **Vista de impacto local** — em construção: mapear uma mudança (um diff) aos
  itens que ela toca e mostrar o raio **antes** de executá-la. O fork já emite a
  posição de cada item no fonte (arquivo + linha); falta a lente consumi-la.

O histórico completo de decisões está nos prompts (`00_nucleo/prompt/`) e laudos
(`00_nucleo/lessons/`).
