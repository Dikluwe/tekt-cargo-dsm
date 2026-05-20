# Roadmap: Construção do `crystalline-dsm`

**Documento**: Plano de execução faseado
**Projecto**: `crystalline-dsm`
**ADR de origem**: ADR-0001 (Criação de Ferramenta DSM)
**Data**: 2026-05-20

Este documento lista os passos de construção do `crystalline-dsm`
do zero até o MVP funcional, conforme critérios de sucesso da
ADR-0001. Cada passo tem critério de conclusão objectivo.

Os passos são incrementais — cada um deixa o projecto num
estado compilável e testável. Não é estritamente sequencial:
passos marcados como **paralelizáveis** podem avançar fora de
ordem se útil.

---

## Fase 0 — Estudo prévio e fundação

### Passo 0.1 — Leitura de referências
**Tipo**: estudo
**Critério**: notas escritas em `docs/estudo-previo.md` cobrindo:
- Como `cargo-modules` resolve declarações `mod` recursivamente.
- Como `cargo-modules` trata `#[cfg(...)]` (compila apenas alguns
  branches? assume todos? ignora?).
- Como `cargo_metadata` expõe workspace members e suas paths.
- Pelo menos 3 implementações de DSM (académicas ou comerciais)
  e o que cada uma usa como célula da matriz (binário,
  contagem, peso).

### Passo 0.2 — Criação do repositório
**Tipo**: setup
**Critério**:
- Repositório `crystalline-dsm` criado.
- `Cargo.toml` com membro único `crystalline-dsm` (binário).
- `README.md` mínimo apontando para a ADR-0001.
- Licença escolhida e arquivada (LICENSE).
- CI básico: `cargo build` + `cargo test` em PR.

### Passo 0.3 — Esqueleto de CLI
**Tipo**: código
**Critério**:
- Binário `crystalline-dsm` compila.
- Aceita argumento posicional (caminho do workspace).
- Flag `--output <path>` (default: `./dsm.html`).
- Flag `--format <json|html>` (default: `html`).
- Comando `crystalline-dsm --version` funciona.
- Teste de integração: roda contra `tests/fixtures/empty-workspace/`
  sem crash.

---

## Fase 1 — Extracção de dados

### Passo 1.1 — Resolução de workspace
**Tipo**: código
**Critério**:
- Dado um path de workspace Cargo, listar todos os crate members.
- Usar `cargo_metadata` para resolução.
- Teste contra `tests/fixtures/multi-crate-workspace/` (criar
  fixture com 3 crates).
- Output intermédio (debug): lista de crates + paths absolutas.

### Passo 1.2 — Travessia de módulos por crate
**Tipo**: código
**Critério**:
- Para cada crate, percorrer o ficheiro de entrada (`lib.rs` ou
  `main.rs`) e seguir todas as declarações `mod` recursivamente.
- Construir árvore de módulos com path canónica (e.g.
  `typst_eval::compiler::pipeline`).
- Tratamento de `#[cfg(...)]`: documentar decisão (sugestão:
  incluir todos os branches, marcar como condicional).
- Teste: árvore esperada vs árvore extraída em fixture
  controlada.

### Passo 1.3 — Extracção de imports (`use`)
**Tipo**: código
**Critério**:
- Para cada módulo, listar todos os `use` statements parseados
  via `syn`.
- Normalizar paths: resolver `crate::`, `self::`, `super::`,
  `pub use` re-exports.
- Distinguir imports internos (mesmo workspace) de externos
  (crates.io).
- Output: lista de arestas `(módulo_origem, símbolo_destino)`.
- Teste: arestas esperadas em fixture controlada.

### Passo 1.4 — Construção do grafo
**Tipo**: código
**Critério**:
- Estrutura `DependencyGraph` com nós (módulos) e arestas
  (depende-de).
- Cada aresta tem peso (contagem de imports do módulo origem
  para o destino).
- Cada aresta marca se cruza fronteira de crate ou só de módulo.
- Serialização para JSON com schema documentado em
  `docs/json-schema.md`.
- Teste: round-trip JSON (serializa → deserializa → igual).

### Passo 1.5 — Detecção de ciclos
**Tipo**: código
**Critério**:
- Algoritmo de Tarjan ou similar para componentes fortemente
  conexas.
- Função `find_cycles(graph) -> Vec<Vec<NodeId>>`.
- Teste: ciclo conhecido em fixture é detectado; grafo acíclico
  retorna vector vazio.

---

## Fase 2 — Renderização DSM

### Passo 2.1 — Ordenação topológica para DSM
**Tipo**: código
**Critério**:
- Implementar ordenação que minimiza dependências abaixo da
  diagonal (técnica clássica de DSM: partitioning).
- Para grafos com ciclos, agrupar componentes cíclicas em
  blocos adjacentes na diagonal.
- Teste: ordenação produz matriz com ≥ 90% das arestas acima
  da diagonal em fixture estruturada.

### Passo 2.2 — Renderizador HTML estático
**Tipo**: código
**Critério**:
- Função que recebe `DependencyGraph` e produz HTML auto-contido
  (CSS + JS inline ou via `data:` URIs).
- DSM renderizada como tabela HTML com:
  - Linhas e colunas: módulos na ordem da Passo 2.1.
  - Células: vazias se sem dependência; coloridas/numeradas se
    com dependência.
  - Tooltip ao passar mouse: detalhes da aresta.
- Filtros básicos: por crate, por camada (se `crystalline.toml`
  presente).
- Teste: HTML gerado abre em navegador sem erros de console;
  teste manual documentado em `docs/teste-manual.md`.

### Passo 2.3 — Integração com `crystalline.toml`
**Tipo**: código
**Critério**:
- Se `crystalline.toml` existe no workspace, parsear secção
  `[layers]`.
- Atribuir cada módulo a uma camada baseado em path.
- Na DSM, células que representam violação de camada (L1 →
  L3, por exemplo) recebem destaque visual.
- Resumo no topo do HTML: nº de violações por tipo.
- Teste: configuração fictícia com violação intencional gera
  destaque correcto.

---

## Fase 3 — Validação no caso real

### Passo 3.1 — Execução em `lab/typst-original/`
**Tipo**: validação
**Critério**:
- Roda contra `lab/typst-original/` do `typst-crystalline`.
- Sem panic.
- Tempo de execução medido e registado.
- HTML resultante navegável; pipeline `Parse → Eval → Layout
  → Export` é visualmente reconhecível na DSM.
- Captura de tela arquivada em `docs/exemplos/typst.png`.

### Passo 3.2 — Execução em `01_core/` (typst-crystalline)
**Tipo**: validação
**Critério**:
- Roda contra o `01_core/` em desenvolvimento.
- Identifica visualmente as fronteiras entre `entities/`,
  `contracts/`, `rules/`.
- Quaisquer violações cristalinas em curso são destacadas via
  integração com `crystalline.toml`.

### Passo 3.3 — Comparação com `cargo-modules`
**Tipo**: validação
**Critério**:
- Output de `crystalline-dsm` e `cargo-modules` no mesmo projecto
  é consistente em árvore de módulos.
- Diferenças identificadas e justificadas (ex: tratamento de
  `#[cfg]` diferente).

---

## Fase 4 — Documentação e release

### Passo 4.1 — README completo
**Tipo**: documentação
**Critério**:
- Secção "O que faz", "O que não faz".
- Instalação via `cargo install crystalline-dsm`.
- Uso básico com exemplo.
- Link para exemplo gerado no Typst.
- Comparação honesta com `cargo-modules`, `cargo-depgraph`.

### Passo 4.2 — Documentação técnica
**Tipo**: documentação
**Critério**:
- `docs/json-schema.md` — schema do grafo serializado.
- `docs/algoritmos.md` — DSM partitioning, detecção de ciclos.
- `docs/limitacoes.md` — granularidade de módulo, tratamento
  de `#[cfg]`, macros não expandidos.

### Passo 4.3 — Primeira release
**Tipo**: release
**Critério**:
- Tag `v0.1.0` no repositório.
- Publicação opcional em crates.io.
- Anúncio (se desejado) em fórum Rust ou similar.

---

## Marcos (resumo)

| Marco | Conclusão se: |
|-------|--------------|
| M0 — Fundação | Passos 0.1–0.3 completos |
| M1 — Análise funcional | Passos 1.1–1.5 completos; JSON do Typst gerado |
| M2 — DSM visual | Passos 2.1–2.3 completos; HTML navegável |
| M3 — MVP validado | Passos 3.1–3.3 completos; critérios da ADR-0001 cumpridos |
| M4 — Release público | Passos 4.1–4.3 completos |

---

## O que este roadmap não cobre

- Plano de marketing ou adopção externa.
- Decisões sobre granularidade fina (função, tipo) — fica para
  versão pós-MVP.
- Integração com IDEs (LSP, plugin VS Code).
- Visualizações alternativas (sunburst, force-directed) — o
  modelo de dados está preparado, mas implementação concreta
  fica para depois.
- Análise temporal (evolução do grafo entre commits).

---

## Estado actual

**Marco actual**: pré-M0. Nenhum passo iniciado. Este roadmap
e a ADR-0001 são os primeiros artefactos do projecto.

**Próximo passo concreto**: Passo 0.1 (leitura de referências).
