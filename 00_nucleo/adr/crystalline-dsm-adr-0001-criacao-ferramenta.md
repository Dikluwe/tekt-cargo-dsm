# ⚖️ ADR-0001: Criação de Ferramenta DSM Open Source para Rust

**Status**: `PROPOSTO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm` (nome provisório)
**Repositório**: a criar (separado de `typst-crystalline` e `crystalline-lint`)

---

## Contexto

O ecossistema Rust não tem uma ferramenta open source equivalente
ao Lattix LDM ou Structure101 para análise arquitectural via
Dependency Structure Matrix (DSM). As ferramentas existentes
cobrem partes do problema mas nenhuma faz DSM propriamente dito:

| Ferramenta | O que faz | O que não faz |
|------------|-----------|---------------|
| `cargo-modules` | Árvore de módulos, grafo de imports internos | DSM, regras arquiteturais, ciclos cross-module |
| `cargo-depgraph` | Grafo de dependências entre crates | DSM, granularidade de módulo/função |
| `cargo-deps` | Similar a depgraph | Igual |
| `rust-analyzer` | Indexação semântica completa | Não é ferramenta de visualização |
| `crystalline-lint` | Validação de regras V1–V14 (camadas, externos) | Visualização, exploração, DSM |

O `crystalline-lint` (projecto separado já existente) cobre a
parte de **validação de regras arquiteturais**. Falta a parte de
**exploração e visualização** das dependências reais — que é o
que LLMs precisam para não se perderem em codebases grandes
(problema diagnosticado: contexto limitado, raciocínio em grafos
degradado, alucinação por falta de visão global).

A migração do Typst para Arquitetura Cristalina expõe este vazio
concretamente: o `lab/typst-original/` tem ~50k linhas em Rust
com pipeline `Parse → Eval → Layout → Export` cujas dependências
internas (chamadas, imports, tipos partilhados) não estão
visualizadas em lugar nenhum. Cada passo de migração exige
reconstruir mentalmente esse mapa.

---

## Decisão

Criar **`crystalline-dsm`** — ferramenta CLI open source em Rust
que produz Dependency Structure Matrices a partir de código
Rust, com modelo de dados extensível para outras visualizações.

### Escopo do MVP

**Dentro:**

- Análise de workspaces Cargo (múltiplas crates).
- Extracção de dependências a nível de módulo (granularidade
  inicial). Granularidade fina (função, tipo) fica para versão
  posterior.
- Modelo de dados canónico em JSON serializável — independente
  da visualização.
- Renderização DSM em HTML estático interactivo (clicável,
  filtrável).
- Detecção de ciclos no grafo de dependências.
- Integração com a configuração de camadas do `crystalline-lint`
  (`crystalline.toml`) — quando presente, a DSM destaca
  violações de camada.

**Fora (versões futuras):**

- Refactoring automatizado.
- Análise semântica profunda (apenas estrutural neste MVP).
- Suporte a outras linguagens.
- Servidor web persistente (saída é arquivo estático).

### Princípios de design

1. **Aproveitar antes de reinventar**: usar `syn` para parsing,
   estrutura de `cargo-modules` como referência para extracção
   de módulos, `cargo_metadata` para resolução de workspace.
2. **Modelo de dados primeiro, visualização depois**: o grafo
   serializado em JSON é o produto canónico. Visualizações
   (DSM, sunburst, force-directed, treemap) são consumidores.
3. **Output estático por padrão**: HTML + JS auto-contido,
   sem servidor. Distribuição e versionamento triviais.
4. **Composição com `crystalline-lint`**: ferramentas separadas,
   formato de configuração partilhado.

### Não-decisões

- **Linguagem de implementação**: Rust (decidido — coerência
  com ecossistema-alvo, reuso de `syn`/`cargo_metadata`).
- **Licença**: a definir no momento da criação do repositório
  (provável MIT ou Apache-2.0 para compatibilidade ampla).
- **Nome final**: `crystalline-dsm` é provisório; pode mudar.
- **Público-alvo**: não delimitado nesta ADR. MVP serve
  primeiro o `typst-crystalline`; generalização vem depois com
  feedback real.

---

## Alternativas consideradas

### Alternativa A — Estender `cargo-modules`

Forkar `cargo-modules` e adicionar saída DSM.

**Prós**: aproveita parser e travessia já testados; comunidade
existente.
**Contras**: `cargo-modules` foi desenhado para árvore, não para
matriz. Modelo de dados interno teria que ser reescrito.
Manutenção de fork divergente é custosa.

### Alternativa B — Plugin para `rust-analyzer`

Construir como extensão do `rust-analyzer`.

**Prós**: acesso a análise semântica completa (resolução de
nomes, tipos).
**Contras**: API instável, acoplamento ao IDE, distribuição
complexa. Excessivo para análise estrutural que `syn` resolve.

### Alternativa C — Construir do zero usando `syn`

Projecto novo, parser próprio sobre `syn`, modelo de dados
desenhado para DSM desde o início.

**Prós**: liberdade de design; modelo de dados certo desde o
começo; sem dívida de fork.
**Contras**: mais código novo. Reinventa partes de
`cargo-modules` (travessia de módulos, resolução de paths).

### Alternativa D (escolhida) — Construir do zero usando `syn`,
estudando `cargo-modules` como referência arquitectural

Mesmo que C, mas com leitura prévia documentada do código de
`cargo-modules` para evitar erros conhecidos e replicar
soluções comprovadas (resolução de `mod` declarations,
tratamento de `#[cfg]`, etc).

**Prós**: liberdade de C + redução de risco via estudo prévio.
**Contras**: requer disciplina de não copiar código sob licença
incompatível.

---

## Consequências

### ✅ Positivas

- Preenche um vazio real no ecossistema Rust open source.
- Resolve dor concreta da migração do Typst (e de qualquer
  refactor arquitectural grande em Rust).
- Composável com `crystalline-lint` sem acoplar — ferramentas
  separadas, configuração partilhada.
- Modelo de dados extensível permite múltiplas visualizações
  sem reescrever a análise.

### ❌ Negativas

- Mais um projecto para manter (além de `typst-crystalline` e
  `crystalline-lint`).
- Sem público-alvo delimitado, risco de design por intuição
  divergir de necessidade real fora do uso interno.
- Granularidade de módulo no MVP pode ser insuficiente para
  alguns casos de uso (e.g. detectar acoplamento por tipo
  específico atravessando módulos).

### ⚙️ Neutras

- Não bloqueia nem acelera a migração do Typst directamente —
  é ferramenta de apoio. Migração continua via passos
  existentes.
- Pode atrair contribuições externas ou ficar como ferramenta
  de uso interno; ambos são resultados aceitáveis.

---

## Critérios de sucesso do MVP

1. Roda em `lab/typst-original/` (workspace Cargo real, ~50k
   linhas) sem panic e em tempo razoável (target: < 30s para
   primeira execução, sem cache).
2. Produz DSM HTML interactiva navegável que torna explícito o
   pipeline `Parse → Eval → Layout → Export` do Typst.
3. Detecta pelo menos um ciclo conhecido se introduzido
   artificialmente em teste.
4. Lê `crystalline.toml` (se presente) e destaca violações de
   camada na DSM.
5. Documentação mínima: README, exemplo de uso no Typst,
   formato do JSON canónico documentado.

---

## Referências

- Lattix LDM — referência comercial de DSM:
  https://www.lattix.com
- Structure101 — referência comercial de análise arquitectural:
  https://structure101.com
- `cargo-modules` — referência open source de análise estrutural
  Rust: https://github.com/regexident/cargo-modules
- `syn` — parser de Rust para macros e análise:
  https://github.com/dtolnay/syn
- `cargo_metadata` — resolução de workspaces Cargo:
  https://github.com/oli-obk/cargo_metadata
- `crystalline-lint` — projecto irmão, validação de regras
  (repositório separado).
- DSM como técnica — Steward (1981), Eppinger (1991), e
  trabalho posterior em arquitectura de software.
