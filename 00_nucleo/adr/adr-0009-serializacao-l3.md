# ⚖️ ADR-0009: Serialização JSON via DTOs em L₃

**Status**: `ACEITO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm`
**Passo do roadmap relacionado**: 1.4 (parte JSON) — completar
  Marco M1 — Análise Funcional

---

## Contexto

O Marco M1 do roadmap exige um produto canónico serializável: o
JSON do grafo de dependências. Este produto serve como artefacto
de interoperabilidade — outras ferramentas (incluindo o
renderizador HTML do Passo 2.2) consomem o JSON em vez de
reparsear o código Rust.

Há duas localizações naturais para a serialização:

1. **L₁** com `serde` em `l1_allowed_external` — as structs do
   core derivam `Serialize`/`Deserialize` directamente.

2. **L₃** com módulo dedicado — L₁ não conhece JSON; L₃ tem
   structs DTO próprias com `serde` derive, e funções que
   convertem entre L₁ e DTO.

A ADR-0005 estabeleceu precedente de aceitar dependências externas
puras em L₁ (`petgraph`), o que tornaria a Opção 1 idiomática. A
decisão final, porém, escolhe a Opção 2 (L₃), priorizando pureza
máxima de L₁ e desacoplamento explícito entre o modelo interno e
o formato de wire.

---

## Alternativas consideradas

### Alternativa A — Serialização em L₁ com `serde` em `l1_allowed_external`

`DependencyGraph`, `GraphNode`, `Cycle`, etc derivam `Serialize`/
`Deserialize` directamente. Adiciona `serde` à lista de
dependências externas permitidas em L₁.

**Prós:**
- Idiomatic Rust (`serde` é o padrão de facto).
- Mínimo de código (derive macros).
- Round-trip via derive é robusto e testado.
- Coerente com decisão da ADR-0005 (`petgraph` em L₁).

**Contras:**
- Acoplamento entre modelo interno e formato externo: mudar o
  schema JSON pode requerer mudar structs do core.
- Mais uma dependência declarada em L₁.

### Alternativa B — Serialização em L₃ com DTOs próprios (escolhida)

L₁ não conhece `serde`. L₃ define structs DTO (`GraphJsonDto`,
`NodeJsonDto`, etc) com `serde` derive. Funções de conversão em
L₃ traduzem entre `&DependencyGraph` (L₁) e `GraphJsonDto`.

**Prós:**
- L₁ permanece sem `serde`. Pureza preservada.
- Schema JSON desacoplado do modelo interno. Mudar formato não
  toca L₁.
- Mapeamento entre estruturas internas e wire format é explícito
  e auditável (cada campo aparece em código de conversão).
- Tratamento natural de IDs opacos (`GraphNodeId`, `GraphEdgeId`):
  o DTO usa `canonical_path` como chave estável.

**Contras:**
- Mais código a manter: cada campo aparece em duas structs (L₁ e
  DTO L₃).
- Round-trip exige reconstrução cuidadosa do `DependencyGraph` via
  API pública (não derive automático).
- Risco de divergência se um campo for adicionado em L₁ mas não no
  DTO.

### Alternativa C — Serialização em L₂

Tratar JSON como "apresentação". **Rejeitada**: L₂ é interface
humana (texto formatado para terminal), não interoperabilidade
para outras ferramentas. JSON canónico é formato de máquina, não
de leitura humana.

---

## Decisão

**Alternativa B: serialização em L₃ via DTOs próprios.**

### Princípios

1. **L₁ não conhece JSON nem `serde`.** Nenhuma struct do core
   recebe derive `Serialize`/`Deserialize`. Nenhuma nova
   dependência em `l1_allowed_external`.

2. **L₃ define DTOs com `serde` derive.** As DTOs são structs
   independentes das de L₁, com nomes paralelos (`GraphJsonDto`,
   `NodeJsonDto`, etc).

3. **Conversão é explícita.** L₃ tem funções `to_dto(&DependencyGraph)
   -> GraphJsonDto` e `from_dto(GraphJsonDto) -> Result<DependencyGraph, ...>`.
   Cada campo é copiado manualmente. Se um campo for adicionado em
   L₁, o compilador NÃO força a actualização do DTO (limitação
   aceita); testes de round-trip protegem contra divergência.

4. **Ordem canónica de chaves via `BTreeMap`.** Quando o DTO
   contém mapas, usar `BTreeMap` em vez de `HashMap` para
   garantir ordem alfabética determinística na serialização.

5. **IDs opacos não vão para o JSON.** `GraphNodeId` e
   `GraphEdgeId` são índices internos do `petgraph`. O JSON
   referencia nós por `canonical_path` (chave lógica estável).
   Durante deserialização, novos IDs internos são gerados; a
   semântica é preservada via `canonical_path`.

6. **Pretty-print sempre.** Output do JSON usa `serde_json::to_string_pretty`.
   Não há modo compacto no MVP.

### Schema do JSON (versão 1)

Estrutura de alto nível:

```json
{
  "schema_version": "1.0.0",
  "generated_at": "2026-05-20T14:32:18Z",
  "tool": {
    "name": "crystalline-dsm",
    "version": "0.1.0"
  },
  "workspace": {
    "root": "/abs/path/to/workspace",
    "members": ["crystalline-dsm-core", "crystalline-dsm-infra", ...]
  },
  "graph": {
    "nodes": [
      {
        "canonical_path": "crystalline_dsm_core::entities::workspace",
        "kind": "internal",
        "crate_name": "crystalline-dsm-core"
      },
      {
        "canonical_path": "serde::de",
        "kind": "external",
        "external_kind": "crate"
      },
      ...
    ],
    "edges": [
      {
        "from": "crystalline_dsm_core::entities::workspace",
        "to": "std::collections",
        "imported_item": "HashMap",
        "alias": null,
        "is_reexport": false,
        "is_glob": false,
        "raw_use_path": "std::collections::HashMap"
      },
      ...
    ]
  },
  "cycles": {
    "count": 3,
    "self_loop_count": 0,
    "multi_node_count": 3,
    "items": [
      {
        "kind": "multi_node",
        "nodes": ["a::b", "a::c"]
      },
      ...
    ]
  }
}
```

Notas sobre o schema:

- `kind: "internal"` carrega `crate_name`; `kind: "external"`
  carrega `external_kind` (`"crate"` ou `"stdlib"`).
- `alias`, `is_reexport`, `is_glob` sempre presentes (não
  omitidos quando `null`/`false`, para previsibilidade do
  consumidor).
- `tree_node_id` do `NodeKind::Internal` NÃO vai para o JSON.
  É reconstruído na deserialização.

### Versionamento

`schema_version` segue semver:

- **major**: mudança incompatível (campo removido, semântica
  alterada).
- **minor**: campo novo opcional adicionado.
- **patch**: correcção de bug no formato (ex: ordem dentro de um
  array que era considerada importante).

Versão inicial: `"1.0.0"`.

---

## Justificação

1. **Desacoplamento explícito**: o modelo interno (`DependencyGraph`)
   pode evoluir sem impacto no formato JSON, e vice-versa. Quem
   mantém o projecto sabe que mudanças em L₁ não quebram
   consumidores externos do JSON automaticamente; isso é
   sinalização útil.

2. **Pureza preservada**: L₁ continua sem novas dependências. A
   regra `l1_allowed_external` é usada com parcimónia. `serde` é
   menos puro que `petgraph` (tem features condicionais,
   `serde_derive` puxa proc-macros pesados); manter fora de L₁
   reduz superfície.

3. **Round-trip auditável**: o código de `to_dto`/`from_dto` é
   onde residem todas as garantias de fidelidade do formato. Bug
   de round-trip aparece em código explícito, não escondido em
   derive.

4. **Precedente para outros formatos**: se o projecto adicionar
   GraphML, CSV, ou DOT no futuro, todos seguem o mesmo padrão:
   DTO em L₃, conversão explícita. Coerência arquitectural.

---

## Consequências

### ✅ Positivas

- L₁ não toca em `serde`.
- Schema JSON é cidadão de primeira classe, com versão própria
  documentada.
- Outros formatos futuros têm padrão claro a seguir.

### ❌ Negativas

- Mais código a manter (DTOs duplicam estrutura de L₁).
- Risco de divergência entre L₁ e DTO (mitigado por testes de
  round-trip).
- Adicionar campo novo em L₁ exige passo manual de actualização
  do DTO.

### ⚙️ Acções decorrentes

1. Criar módulo `03_infra/src/json_serializer.rs` com DTOs e
   funções `to_canonical_json` / `from_canonical_json`.
2. Adicionar `serde` e `serde_json` ao `Cargo.toml` de
   `03_infra` (NÃO ao de `01_core`).
3. Implementar testes de round-trip: criar grafo, serializar,
   deserializar, verificar equivalência semântica (mesmos nós,
   mesmas arestas).
4. Documentar o schema em ficheiro próprio
   (`docs/graph-schema-v1.md`). [CUMPRIDO]
5. Adicionar comando ou flag CLI em L₄ para gravar o JSON num
   ficheiro (`--output graph.json`).

---

## Critérios de reavaliação

Esta ADR deve ser reaberta se:

1. O custo de manter DTOs duplicados ficar inaceitável (sinal:
   bugs frequentes por campos esquecidos no DTO).
2. Alguma feature do `serde` (ex: `#[serde(flatten)]`,
   `#[serde(skip)]`) ficar essencial e for substancialmente mais
   simples de implementar com derive directo em L₁.
3. Outros consumidores aparecerem (ex: API HTTP, gRPC) e o
   padrão atual virar gargalo.

---

## Referências

- ADR-0001 — Criação da ferramenta.
- ADR-0005 — `petgraph` em L₁ (precedente para
  `l1_allowed_external`).
- ADR-0006 — Nós fantasma (modelagem de externos no grafo).
- `serde` crate: https://crates.io/crates/serde
- `serde_json` crate: https://crates.io/crates/serde_json
- Roadmap original Passo 1.4 — JSON canónico (item pendente).
