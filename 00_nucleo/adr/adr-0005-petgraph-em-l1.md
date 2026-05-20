# ⚖️ ADR-0005: `petgraph` como Dependência Permitida em L₁

**Status**: `ACEITO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm`
**Passos do roadmap relacionados**: 1.4 (Construção do grafo), 1.5 (Detecção de ciclos)

---

## Contexto

O `crystalline-dsm` precisa representar um grafo de dependências
entre módulos para construir a DSM. A representação interna do
grafo é uma decisão deixada em aberto pelo Estudo Prévio (Passo
0.1) e pela ADR-0001, que apenas listou duas opções:

- Implementação própria usando `Vec` + `HashMap`.
- Uso de biblioteca externa (`petgraph` foi mencionado).

A camada que aloja a estrutura de grafo é L₁ (Núcleo), porque o
grafo é dado puro de domínio e os algoritmos que operam sobre
ele (detecção de ciclos, ordenação topológica, particionamento
DSM) são puros. Esta ADR decide qual representação usar.

---

## Alternativas consideradas

### Alternativa A — Implementação própria (`Vec` + `HashMap`)

Manter o grafo como `Vec<Node>` + `HashMap<NodeId, Vec<EdgeId>>`
+ `Vec<Edge>`. Algoritmos (Tarjan SCC, ordenação topológica)
implementados manualmente.

**Prós:**
- Zero dependências externas em L₁ (pureza máxima).
- Controle total sobre layout de memória.
- Serialização directa (sem adaptador).
- Menos código que depender de uma biblioteca cuja API pode
  mudar.

**Contras:**
- Implementar Tarjan correctamente requer cuidado (recursão,
  detecção de SCCs, manejo da pilha). Bugs sutis comuns.
- Reinventa o que já existe maduro no ecossistema.
- Manutenção contínua dos algoritmos por nossa conta.

### Alternativa B — `petgraph` em `l1_allowed_external`

Adicionar `petgraph` à lista de dependências externas permitidas
em L₁, conforme regra do framework Tekt. Usar tipos como
`petgraph::Graph`, `petgraph::algo::tarjan_scc`,
`petgraph::algo::toposort`.

**Prós:**
- Algoritmos testados e usados em produção (anos de maturidade,
  comunidade ampla).
- Tarjan SCC já implementado, validado, optimizado.
- Outros algoritmos disponíveis sem custo (Dijkstra, BFS, DFS,
  ordenação topológica, componentes conexos).
- Reduz superfície de bugs em código próprio.

**Contras:**
- L₁ deixa de ser zero-dependency-external (mas Tekt permite via
  `l1_allowed_external`).
- API do `petgraph` evolui; necessidade de gerir versão.
- Tipos do `petgraph` na assinatura pública de L₁ podem
  contaminar L₂/L₃/L₄ se mal isolados.

### Alternativa C — Interface própria, implementação migrável

Implementar fachada própria de grafo em L₁, com algoritmos
inicialmente manuais, mas com interface desenhada para permitir
substituição futura por `petgraph`.

**Prós:**
- Flexibilidade futura.
- Pureza inicial.

**Contras:**
- Custo dobrado: implementar a fachada AGORA + implementar
  algoritmos AGORA + possivelmente migrar DEPOIS.
- "Permitir migrar depois" raramente acontece na prática
  (YAGNI). Adia custo sem eliminar.

---

## Decisão

**Alternativa B: `petgraph` em `l1_allowed_external`.**

`petgraph` é adicionado à lista `l1_allowed_external` do framework
Tekt para o projecto `crystalline-dsm`. L₁ usa `petgraph::Graph` e
algoritmos como `petgraph::algo::tarjan_scc` directamente.

### Versão a fixar

A versão exacta do `petgraph` deve ser fixada no `Cargo.toml` de
L₁ no momento da implementação do Passo 1.4. Recomendação: usar
versão estável recente, fixa em major.minor (ex: `"0.6"`), não
em `"*"`.

### Regras de isolamento

Apesar do uso interno de `petgraph`, a **API pública** de L₁
NÃO deve expor tipos de `petgraph`. A struct `DependencyGraph` de
L₁ é um wrapper sobre o `petgraph::Graph` internamente, com API
própria. Razão:

1. Se `petgraph` for substituído (ex: por implementação própria
   ou outra biblioteca), apenas L₁ precisa mudar.
2. L₂, L₃ e L₄ não devem importar `petgraph` directamente. Se
   precisarem de funcionalidade não exposta por L₁, a função
   correspondente é adicionada à API de `DependencyGraph` em L₁.

Excepção: tipos opacos como `petgraph::graph::NodeIndex` podem
aparecer na API pública se forem encapsulados num newtype de L₁
(ex: `pub struct GraphNodeId(NodeIndex)`).

---

## Justificação

1. **Algoritmos não-triviais**: Tarjan SCC tem detalhes sutis
   (especialmente em Rust, onde recursão profunda pode estourar
   a stack). Usar implementação testada elimina classe inteira
   de bugs.

2. **Princípio de não-reinvenção**: `petgraph` é uma das crates
   mais maduras do ecossistema Rust para grafos. Reimplementar
   sem motivo concreto é trabalho sem retorno.

3. **Tekt prevê excepções**: a regra `l1_allowed_external` existe
   precisamente para casos onde uma dependência externa é
   funcionalmente pura e simplifica o domínio. `petgraph` se
   qualifica: não faz I/O, não tem estado global, comportamento
   determinístico.

4. **Custo de isolamento é baixo**: o wrapper `DependencyGraph`
   em L₁ é necessário independentemente da escolha (algoritmos
   próprios ou `petgraph`). Não há custo adicional significativo
   por isolar `petgraph` por trás dele.

---

## Consequências

### ✅ Positivas

- Implementação de algoritmos é principalmente "chamar função do
  `petgraph`".
- Cobertura de ciclos, ordenação topológica, conectividade
  resolvida sem código extra.
- Foco do esforço pode ir para construção do grafo e adaptação ao
  domínio, não para algoritmos.

### ❌ Negativas

- L₁ tem uma dependência externa adicional. Cargo.lock cresce.
- Versionamento de `petgraph` torna-se preocupação contínua.
- Risco se `petgraph` ficar sem manutenção. Mitigação: `petgraph`
  é amplamente usado, risco baixo.

### ⚙️ Acções decorrentes

- Actualizar a configuração `l1_allowed_external` (em
  `crystalline.toml` ou equivalente) adicionando `petgraph`.
- No `Cargo.toml` de `01_core` (`crystalline-dsm-core`), adicionar
  `petgraph = "X.Y"` (versão a definir).
- Documentar no README a lista de dependências externas de L₁ e o
  motivo de cada uma.

---

## Critérios de reavaliação

Esta ADR deve ser reaberta se:

1. `petgraph` ficar sem manutenção (último commit > 12 meses, sem
   sucessor claro).
2. Mudanças de major version do `petgraph` forçarem refactor
   significativo do `DependencyGraph`.
3. Caso de uso real exigir algoritmo que `petgraph` não tem e seja
   mais simples implementar do zero do que adaptar.

---

## Referências

- `petgraph` crate: https://crates.io/crates/petgraph
- Documentação: https://docs.rs/petgraph
- ADR-0001 — Criação da ferramenta (escolha de Rust).
- ADR-0004 — Granularidade do nó do grafo.
- Estudo Prévio (Passo 0.1) — Seção sobre escolha de
  implementação de grafo.
