# Prompt L0: `trees_serializer` (L₃)

**Camada**: L₃ (Infraestrutura)
**Ficheiro alvo**: `03_infra/src/trees_serializer.rs` (novo)
**Passo do roadmap**: 1.4 (parte JSON, complemento) — fechar M1
**Status**: IMPLEMENTADO
**ADR motivadora**: ADR-0010

---

## Decisões de design prévias

- **ADR-0009**: serialização em L₃ via DTOs. Princípio replicado
  aqui.
- **ADR-0010**: `trees.json` é artefacto opcional, gerado por
  flag `--emit-trees`. Conteúdo: `ModuleTree`s completas.

---

## Decisões locais (assumidas neste prompt)

1. **Schema independente do `graph.json`**: `trees.json` tem
   seu próprio `schema_version`. Versão inicial: `"1.0.0"`.

2. **DTOs próprios em L₃**: análogos aos do `json_serializer`.
   `serde` derive aqui, não em L₁.

3. **`NodeId` interno NÃO vai para o JSON**: índices opacos do
   vetor interno da `ModuleTree`. A árvore é serializada como
   lista de nós + estrutura pai-filho via `canonical_path`.

4. **Ponte com `graph.json` é por `canonical_path`**: nenhum
   campo de cross-reference numérico. Quem consumir os dois
   ficheiros casa por `canonical_path`.

5. **Pretty-print sempre + ordem canónica**: mesmas regras do
   `json_serializer`.

6. **Timestamp e versão da tool como parâmetros**: idem
   `json_serializer`. Determinismo preservado.

7. **Múltiplas árvores no mesmo ficheiro**: um workspace tem
   múltiplas `ModuleTree`s (uma por crate). O `trees.json`
   contém todas, indexadas pelo `crate_name`.

---

## Contexto

Quando o utilizador invoca a CLI com flag `--emit-trees`, dois
ficheiros são gerados:

```
output/
├── graph.json
└── trees.json
```

O `trees.json` carrega as `ModuleTree`s completas para que
consumidores externos possam:

- Navegar de um nó do grafo para o seu ficheiro fonte (lendo
  `source_file`).
- Saber se um módulo é inline ou não.
- Saber se foi declarado com `#[path]`.
- Reconstruir a hierarquia pai-filho.

Sem a flag, o `trees.json` não é gerado. O `graph.json` permanece
completo para análise estrutural; só perde a possibilidade de
navegar para os ficheiros fonte.

---

## DTOs (definidos em L₃)

### `TreesJsonDto`

DTO de topo. Reúne tudo o que vai para o ficheiro.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreesJsonDto {
    pub schema_version: String,
    pub generated_at: String,
    pub tool: ToolInfoDto,           // reusa o DTO do json_serializer
    pub workspace: WorkspaceInfoDto, // reusa o DTO do json_serializer
    pub trees: Vec<ModuleTreeDto>,
}
```

Notas:
- `tool` e `workspace` são re-exportados do `json_serializer`
  (reuso de DTO). Não duplicar definição.
- `trees` é vector ordenado alfabeticamente por `crate_name`.

### `ModuleTreeDto`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleTreeDto {
    pub crate_name: String,
    pub nodes: Vec<ModuleNodeDto>,
}
```

`nodes` em ordem determinística: pre-order da árvore (raiz
primeiro, depois filhos recursivamente). Garante reconstrução
correcta.

### `ModuleNodeDto`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleNodeDto {
    pub canonical_path: String,
    pub crate_name: String,
    pub module_path: Vec<String>,
    pub source_file: String,        // serializado como string (não PathBuf)
    pub is_inline: bool,
    pub has_custom_path: bool,
    /// Caminho canónico do módulo pai. None para a raiz.
    pub parent_canonical_path: Option<String>,
}
```

Notas:
- `source_file` é serializado como `String`, não `PathBuf`. JSON
  trata caminhos como strings; o uso de `String` aqui evita
  problemas de portabilidade do `PathBuf` em serde.
- `parent_canonical_path` é a forma de codificar a estrutura
  pai-filho sem usar IDs internos.
- A raiz tem `parent_canonical_path == None`.

---

## Funções públicas

### Serialização

```rust
pub fn to_canonical_json_trees(
    trees: &HashMap<String, ModuleTree>,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
) -> Result<String, TreesSerializeError>;
```

Comportamento:

1. Construir `TreesJsonDto` a partir dos inputs via `to_dto_trees`.
2. Ordenar `trees` por `crate_name` alfabético.
3. Dentro de cada `ModuleTree`, exportar nós em pre-order.
4. Serializar via `serde_json::to_string_pretty`.

### Deserialização

```rust
pub fn from_canonical_json_trees(
    json: &str,
) -> Result<(HashMap<String, ModuleTree>, TreesJsonDto), TreesDeserializeError>;
```

Comportamento:

1. Parsear `json` em `TreesJsonDto`.
2. Verificar `schema_version`:
   - `"1.0.0"`: aceitar.
   - Outras versões major: rejeitar.
   - Outras minor/patch: aceitar (warning silencioso).
3. Para cada `ModuleTreeDto`:
   a. Criar `ModuleTree::new(crate_name, source_file_da_raiz)`.
   b. Iterar `nodes` em ordem (pre-order esperada):
      - Primeiro elemento é a raiz; já criado.
      - Para cada não-raiz: encontrar pai via
        `find_by_canonical_path(parent_canonical_path)` e chamar
        `add_child(...)` com os dados.
4. Retornar tuplo `(HashMap<String, ModuleTree>, dto)`.

### Conversão DTO ↔ Domínio

```rust
pub(crate) fn to_dto_trees(
    trees: &HashMap<String, ModuleTree>,
    workspace: &Workspace,
    tool_version: &str,
    generated_at: &str,
) -> TreesJsonDto;

pub(crate) fn from_dto_trees(
    dto: TreesJsonDto,
) -> Result<HashMap<String, ModuleTree>, TreesDeserializeError>;
```

---

## Tipos de erro

```rust
#[derive(Debug, thiserror::Error)]
pub enum TreesSerializeError {
    #[error("Falha ao serializar para JSON: {source}")]
    SerdeError {
        #[from]
        source: serde_json::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum TreesDeserializeError {
    #[error("Falha ao parsear JSON: {source}")]
    SerdeError {
        #[from]
        source: serde_json::Error,
    },

    #[error("Versão de schema incompatível: esperado 1.x.y, recebido {version}")]
    IncompatibleSchemaVersion { version: String },

    #[error("Pai referenciado não existe: {parent_canonical_path}")]
    DanglingParentReference { parent_canonical_path: String },

    #[error("Erro ao reconstruir árvore: {source}")]
    TreeReconstructionError {
        #[from]
        source: crystalline_dsm_core::entities::module_tree::TreeError,
    },

    #[error("Árvore sem nó raiz (lista de nós vazia)")]
    EmptyTree { crate_name: String },
}
```

---

## Dependências externas

Em `03_infra/Cargo.toml`: nenhuma nova. `serde` e `serde_json`
já foram adicionados no `json_serializer`.

Dependência interna: `crystalline-dsm-core` para `ModuleTree`,
`ModuleNode`, `NodeId`, `Workspace`.

---

## Testes esperados

### Testes unitários (no próprio ficheiro)

1. **Serializar `HashMap` vazio**: zero árvores, JSON contém
   `trees: []`.

2. **Serializar uma árvore com só raiz**: `trees[0].nodes`
   tem 1 elemento, `parent_canonical_path == None`.

3. **Serializar árvore aninhada (3 níveis)**: raiz → filho → neto.
   Verificar ordem pre-order: raiz, filho, neto.

4. **Serializar múltiplas árvores**: 3 crates,
   ordem alfabética em `trees`.

5. **Serializar módulo inline**: `is_inline: true`, `source_file`
   igual ao do pai.

6. **Serializar módulo com `#[path]`**: `has_custom_path: true`.

7. **Metadados**: `schema_version`, `tool.name`, `generated_at`
   correctamente passados.

### Testes de round-trip

8. **Round-trip vazio**: `HashMap` vazio → JSON → `HashMap`. Igual.

9. **Round-trip uma árvore com raiz**: serializar, deserializar,
   verificar `crate_name`, `node_count == 1`, raiz com mesmo
   `canonical_path`.

10. **Round-trip árvore aninhada**: 4 nós em hierarquia →
    serializar → deserializar → mesmo `node_count`, mesmas
    relações pai-filho.

11. **Round-trip multiplas árvores**: 3 crates com hierarquias
    diferentes → idem.

12. **Round-trip módulo inline preservado**: `is_inline` é
    preservado.

13. **Round-trip ordem pre-order**: a deserialização funciona
    porque os pais aparecem antes dos filhos. Verificar que
    inversão da ordem gera `DanglingParentReference`.

### Testes de erros

14. **Schema incompatível**: `schema_version: "2.0.0"` →
    `IncompatibleSchemaVersion`.

15. **Pai inexistente**: `parent_canonical_path` que não está
    na lista → `DanglingParentReference`.

16. **Lista vazia em árvore**: `nodes: []` → `EmptyTree`.

17. **JSON malformado**: string inválida → `SerdeError`.

### Testes de integração

18. **Cross-reference com `graph.json`**: gerar grafo + árvore,
    serializar ambos, deserializar ambos. Para cada
    `InternalWithoutTree` no grafo, encontrar o
    `ModuleNode` correspondente na árvore via
    `canonical_path`. Verificar que `source_file` está
    acessível.

---

## Critério de aceitação do prompt

- `03_infra/src/trees_serializer.rs` existe e compila.
- DTOs definidos conforme especificado.
- Funções `to_canonical_json_trees` e
  `from_canonical_json_trees` com as assinaturas especificadas.
- Os 18 testes passam.
- `cargo clippy --all-targets` sem warnings.
- L₁ permanece inalterado (sem novas dependências).
- Módulo exportado em `03_infra/src/lib.rs`.

---

## Próximos passos (fora deste prompt)

Após implementação:

1. Em L₄: adicionar flag `--emit-trees`. Quando presente, depois
   de gravar `graph.json`, gravar também `trees.json` no mesmo
   diretório.

2. Documentar o schema em `docs/trees-schema-v1.md`.

3. Atualizar README com exemplo de uso.

---

## Limitações conhecidas

1. A reconstrução requer ordem pre-order no JSON. Se um JSON
   externo tiver ordem diferente (ex: filhos antes de pais),
   falha com `DanglingParentReference`. Documentado.

2. Não há suporte a deserialização parcial: ler só uma árvore
   do `trees.json` exige parsear o ficheiro todo.

---

## Hash do prompt

A calcular após aprovação.
