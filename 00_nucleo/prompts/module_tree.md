# Prompt L0: Entidade `ModuleTree` (L₁)

**Camada**: L₁ (Núcleo)
**Ficheiro alvo**: `01_core/src/entities/module_tree.rs`
**Passo do roadmap**: 1.2 — Travessia de módulos por crate
**Status**: IMPLEMENTADO (revisado)
**Revisão**: `dependency_graph-revisao.md` (ADR-0010 — `NodeId::placeholder()` removido; pureza de L₁ restaurada).

---

## Decisões de design prévias (registadas em ADR)

- **ADR-0002**: `#[cfg(...)]` é ignorado. Todos os módulos
  declarados entram na árvore.
- **ADR-0003**: `#[path = "..."]` é resolvido em L₃. O resultado
  (ficheiro físico real) vem para L₁ como `source_file`.
- **ADR-0004**: O nó do grafo é o módulo lógico, com identificador
  canónico `<crate_name>::<module_path>`.

Esta entidade implementa a estrutura de dados que materializa a
ADR-0004.

---

## Decisões locais (assumidas neste prompt)

1. **Granularidade**: `ModuleTree` representa **um crate**. A
   agregação multi-crate (`ModuleForest`) será feita em L₄ no
   momento da composição, fora desta entidade.

2. **Extracção de imports**: NÃO faz parte deste prompt. Os campos
   relacionados a `use` statements serão adicionados no Passo 1.3,
   via prompt separado. Esta entidade modela apenas estrutura de
   módulos.

3. **Identidade do nó raiz**: cada `ModuleTree` tem exactamente um
   nó raiz, correspondente ao módulo de entrada do crate (`lib.rs`
   ou `main.rs`). O caminho lógico do nó raiz é o nome do crate
   (ex: `crystalline_dsm_core`), sem sufixo.

4. **Representação da árvore**: uso de índices (`NodeId`) em vez
   de referências (`&ModuleNode` ou `Rc<ModuleNode>`). Razão:
   simplicidade, evita lifetimes complexos, compatível com
   serialização futura.

---

## Contexto

Após o Passo 1.1 produzir um `Workspace` com `WorkspaceMember`s, o
Passo 1.2 percorre cada membro e constrói a árvore de módulos
internos. Esta entidade é o resultado dessa travessia para um
único crate.

A entidade é puramente estrutural: armazena os nós e as relações
pai-filho. Não armazena dependências entre módulos (imports), nem
metadados de análise. Isso vem em passos posteriores.

---

## Definição das structs

### `ModuleNode`

Representa um único módulo Rust. Estrutura conforme ADR-0004 mais
campos auxiliares para travessia.

```rust
pub struct ModuleNode {
    /// Identificador canónico completo.
    /// Ex: "crystalline_dsm_core::entities::workspace".
    /// Para o nó raiz: nome do crate sem sufixo (ex: "crystalline_dsm_core").
    pub canonical_path: String,

    /// Nome do crate ao qual este módulo pertence.
    /// Ex: "crystalline_dsm_core".
    pub crate_name: String,

    /// Caminho lógico do módulo dentro do crate, segmento por segmento.
    /// Ex: para "crystalline_dsm_core::entities::workspace",
    /// este campo é ["entities", "workspace"].
    /// Vazio para o nó raiz.
    pub module_path: Vec<String>,

    /// Ficheiro físico que contém este módulo.
    /// Para módulos com ficheiro próprio: caminho do próprio ficheiro.
    /// Para módulos inline: caminho do ficheiro do módulo pai.
    pub source_file: PathBuf,

    /// `true` se o módulo é declarado inline (`mod foo { ... }`).
    /// `false` se tem ficheiro próprio.
    pub is_inline: bool,

    /// `true` se o módulo foi declarado com atributo `#[path = "..."]`.
    /// Informativo apenas. A resolução já foi feita em L₃.
    pub has_custom_path: bool,
}
```

### `NodeId`

Identificador opaco de nó dentro de uma `ModuleTree`. Implementado
como wrapper de `usize`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);
```

Notas:
- O valor interno **não** é exposto na API pública.
- Construção apenas via métodos da `ModuleTree`. Utilizadores
  externos recebem `NodeId`s e usam-nos opacamente.
- Comparação de `NodeId` entre `ModuleTree`s diferentes não tem
  significado e não é validada (responsabilidade do utilizador
  não misturar).

### `ModuleTree`

A árvore completa para um crate.

```rust
pub struct ModuleTree {
    /// Nome do crate.
    pub crate_name: String,

    /// Todos os nós da árvore, indexados por `NodeId`.
    /// O nó na posição 0 é sempre o nó raiz.
    nodes: Vec<ModuleNode>,

    /// Para cada nó, a lista de filhos directos.
    /// `children[i]` é a lista de `NodeId`s filhos do nó com ID `i`.
    children: Vec<Vec<NodeId>>,

    /// Para cada nó (excepto raiz), o `NodeId` do pai.
    /// `parents[0]` é `None` (raiz não tem pai).
    parents: Vec<Option<NodeId>>,
}
```

---

## Operações em L₁

### Construção

```rust
impl ModuleTree {
    /// Cria uma nova árvore com apenas o nó raiz.
    /// `crate_name` é o nome do crate.
    /// `root_file` é o ficheiro de entrada (lib.rs ou main.rs).
    pub fn new(crate_name: String, root_file: PathBuf) -> Self;

    /// Adiciona um nó filho a um nó existente.
    /// Retorna o `NodeId` do nó recém-criado.
    /// `parent` deve ser um `NodeId` válido desta árvore.
    /// Em caso de `parent` inválido, retorna `Err(TreeError::InvalidParent)`.
    pub fn add_child(
        &mut self,
        parent: NodeId,
        module_name: String,
        source_file: PathBuf,
        is_inline: bool,
        has_custom_path: bool,
    ) -> Result<NodeId, TreeError>;
}
```

A função `add_child`:
- Calcula `canonical_path` e `module_path` automaticamente a
  partir do pai e do `module_name` fornecido.
- Calcula `crate_name` do filho como sendo igual ao do pai (não é
  parâmetro).
- Adiciona o novo `NodeId` à lista de filhos do pai.
- Adiciona entrada em `parents` apontando para o pai.

### Inspecção

```rust
impl ModuleTree {
    /// Retorna o `NodeId` do nó raiz. Sempre `NodeId(0)`.
    pub fn root(&self) -> NodeId;

    /// Retorna referência ao nó pelo seu ID.
    /// `panic!` se o ID for inválido (programação errada do
    /// utilizador). Não esconder bug.
    pub fn node(&self, id: NodeId) -> &ModuleNode;

    /// Retorna lista de filhos directos do nó.
    pub fn children(&self, id: NodeId) -> &[NodeId];

    /// Retorna o pai do nó, ou `None` se for a raiz.
    pub fn parent(&self, id: NodeId) -> Option<NodeId>;

    /// Itera sobre todos os nós em ordem de inserção (BFS-friendly).
    pub fn all_nodes(&self) -> impl Iterator<Item = (NodeId, &ModuleNode)>;

    /// Quantidade total de nós (incluindo raiz).
    pub fn node_count(&self) -> usize;

    /// Procura um nó pelo `canonical_path`. `O(n)`. Para uso
    /// pontual; não usar em hot path.
    pub fn find_by_canonical_path(&self, path: &str) -> Option<NodeId>;
}
```

### Travessia

```rust
impl ModuleTree {
    /// Itera os nós em pré-ordem (raiz primeiro, depois filhos
    /// recursivamente).
    pub fn iter_preorder(&self) -> impl Iterator<Item = (NodeId, &ModuleNode)>;

    /// Itera os nós em pós-ordem (filhos primeiro, raiz por último).
    pub fn iter_postorder(&self) -> impl Iterator<Item = (NodeId, &ModuleNode)>;
}
```

---

## Tipo de erro

```rust
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TreeError {
    #[error("NodeId inválido para esta árvore: {0:?}")]
    InvalidParent(NodeId),

    #[error("Já existe um módulo com este nome ({name}) como filho do nó pai")]
    DuplicateChild { name: String },
}
```

`thiserror` está em `l1_allowed_external` (já declarado para o
módulo `workspace.rs` no Passo 1.1).

---

## Invariantes

L₁ não valida invariantes complexas, mas mantém estas
construtivamente:

1. **Raiz sempre em índice 0**: a primeira chamada a `new` cria o
   nó raiz no índice 0. `add_child` nunca cria novos nós no
   índice 0.

2. **Coerência de `parents` e `children`**: para qualquer `node`
   com `parents[node] = Some(p)`, `children[p]` contém `node`.
   Garantido pela implementação de `add_child`.

3. **Caminhos canónicos consistentes**: o `canonical_path` de um
   nó é sempre `<crate_name>::<segmentos do module_path>`.
   Calculado automaticamente em `add_child`.

4. **Nomes únicos entre irmãos**: dois filhos directos do mesmo
   pai não podem ter o mesmo `module_name`. Tentar adicionar
   retorna `Err(DuplicateChild)`.

---

## Derives obrigatórios

- `Debug` — todas as structs e enums.
- `Clone` — `ModuleNode`, `NodeId`, `ModuleTree`.
- `PartialEq`, `Eq` — `ModuleNode`, `NodeId`, `ModuleTree`,
  `TreeError`.
- `Hash` — apenas `NodeId` e `ModuleNode`.

Sem `Serialize`/`Deserialize` neste passo (Passo 1.4 decide).

---

## Dependências externas

- `thiserror` — apenas para definição do enum de erro.

Não usar:
- `syn`, `cargo_metadata` (proibido em L₁).
- Estruturas de grafo externas (`petgraph`, etc).
- `serde` (adiado).

---

## Sobre `ModuleForest` (fora deste prompt, registado para contexto)

A agregação multi-crate (`ModuleForest`) será feita em L₄ na fase
de composição, conforme decisão arquitectural. L₄ recebe os
`ModuleTree`s individuais produzidos por L₃ para cada crate e
combina-os numa estrutura única para análise cross-crate.

A implementação concreta do `ModuleForest` virá num prompt
posterior (provavelmente no Passo 1.4, quando o grafo de
dependências for construído).

**Esta entidade `ModuleTree` não conhece o `ModuleForest`**. A
direcção da dependência é unidireccional: `ModuleForest` (futuro)
→ `ModuleTree` (este prompt) → `ModuleNode`.

---

## Testes esperados

Localização: testes inline com `#[cfg(test)]` em
`01_core/src/entities/module_tree.rs`.

Cobertura mínima:

1. **`new` cria árvore com raiz**: árvore recém-criada tem 1 nó,
   `root()` retorna `NodeId(0)`, o nó raiz tem `module_path`
   vazio, `canonical_path` igual ao `crate_name`.

2. **`add_child` em raiz**: adicionar filho à raiz; verificar que
   `canonical_path` resultante é `crate::filho`, `module_path` é
   `["filho"]`, `node_count` é 2.

3. **`add_child` em profundidade**: criar
   `crate -> a -> b -> c`. Verificar caminhos canónicos correctos
   em cada nível.

4. **Inspecção**: `children` da raiz contém o filho adicionado;
   `parent` do filho aponta para a raiz; `parent` da raiz é
   `None`.

5. **`DuplicateChild`**: tentar adicionar dois filhos com mesmo
   nome ao mesmo pai retorna `Err(DuplicateChild)`. O nome é
   permitido em pais diferentes (`a::tests` e `b::tests` coexistem).

6. **`InvalidParent`**: passar um `NodeId` construído manualmente
   com índice fora dos limites retorna `Err(InvalidParent)`.
   (Construção via método auxiliar de teste, já que `NodeId` é
   opaco na API pública.)

7. **`iter_preorder` e `iter_postorder`**: ordens correctas numa
   árvore conhecida.

8. **`find_by_canonical_path`**: retorna `Some` para caminho
   existente, `None` para inexistente.

9. **`PartialEq` em `ModuleTree`**: duas árvores construídas com
   os mesmos passos são iguais.

10. **Caso de módulo inline**: adicionar nó com `is_inline = true`.
    Verificar que `source_file` é igual ao do pai, e que o
    `is_inline` é preservado.

---

## Critério de aceitação do prompt

- O ficheiro `01_core/src/entities/module_tree.rs` existe e compila.
- Todas as structs e enums acima estão definidas exactamente como
  especificado.
- Todos os métodos têm a assinatura especificada.
- Os 10 grupos de testes acima passam.
- `cargo clippy -p crystalline-dsm-core` passa sem warnings.
- Nenhuma importação de `syn`, `cargo_metadata`, `serde` ou
  `petgraph` neste ficheiro.
- A entidade está exportada via `01_core/src/entities/mod.rs`.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-20 | Implementação da entidade ModuleTree e testes unitários | `01_core/src/entities/module_tree.rs`, `01_core/src/entities/mod.rs` |

---

## Hash do prompt

A calcular após aprovação.
