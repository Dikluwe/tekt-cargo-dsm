# Prompt L0: Traversador de Módulos (L₃)

**Camada**: L₃ (Infraestrutura)
**Ficheiro alvo**: `03_infra/src/module_traverser.rs`
**Passo do roadmap**: 1.2 — Travessia de módulos por crate
**Status**: IMPLEMENTADO (revisado)
**Revisão**: `module_traverser-revisao.md` (ADR-0008 — propagação de entry-style a partir do `WorkspaceMember`; remoção do check por nome `lib.rs`/`main.rs`).

---

## Decisões de design prévias (registadas em ADR)

- **ADR-0002 (ACEITO)**: `#[cfg(...)]` é ignorado. Todo `mod foo;`
  declarado entra na árvore, independentemente de qualquer
  atributo `#[cfg]`.
- **ADR-0003 (PROPOSTO, pendente de aprovação)**: `#[path = "..."]`
  é resolvido completamente, excepto dentro de módulos inline.
  Em caso de conflito (dois `mod nome;` com `#[path]` diferentes,
  cada um sob `#[cfg]` distinto), o primeiro encontrado vence.
- **ADR-0004 (PROPOSTO)**: Nó é módulo lógico com identificador
  `<crate_name>::<module_path>`.

---

## Decisões locais (assumidas neste prompt)

1. **Extracção de imports**: NÃO faz parte deste prompt. A
   travessia apenas mapeia a estrutura de módulos. Os `use`
   statements serão extraídos no Passo 1.3 num passo subsequente
   sobre os mesmos ficheiros (ou guardando o `syn::File` para
   reaproveitamento — decisão fora deste prompt).

2. **Granularidade do resultado**: produz um `ModuleTree` por
   crate. A agregação `ModuleForest` é responsabilidade de L₄.

3. **Tratamento de macros**: limitação documentada da ADR-0004 e
   do Estudo Prévio. `mod` dentro de invocações de macro
   (`macro_rules!` ou proc-macros) NÃO é detectado. Sem warning
   (o parser nem sabe que está lá).

4. **Tratamento de `extern crate`**: ignorado. Apenas projectos
   Edição 2018+ são suportados, conforme Estudo Prévio.

5. **Hierarquia de funções**: a função pública principal opera
   sobre um `WorkspaceMember`. Funções auxiliares privadas operam
   sobre ficheiros individuais.

---

## Contexto

Este módulo consome a saída do `cargo_metadata_reader` (Passo 1.1)
e produz, para cada `WorkspaceMember`, um `ModuleTree` completo.

O fluxo conceptual é:

```
WorkspaceMember
    └─> ler entry_point (lib.rs ou main.rs)
        └─> parsear com syn -> syn::File
            └─> percorrer items, identificar `mod` declarations
                ├─> mod inline: criar nó, recursar no bloco
                └─> mod externo: resolver ficheiro, ler, recursar
                    (com #[path] se presente)
```

A complexidade real está em:
- Resolução de caminhos de ficheiros (padrão e custom).
- Recursão controlada (detectar ciclos de inclusão, se possível).
- Tratamento de erros (ficheiros faltantes, parsing falha).

---

## Função pública principal

```rust
pub fn traverse_crate(
    member: &WorkspaceMember,
) -> Result<ModuleTree, TraverseError>;
```

### Comportamento

1. Cria `ModuleTree::new(member.name.clone(), member.entry_point.clone())`.

2. Lê o ficheiro `member.entry_point` para string.

3. Parseia com `syn::parse_file`. Em caso de erro, retorna
   `Err(TraverseError::ParseFailed { file, source })`.

4. Para cada item `syn::Item::Mod` no ficheiro, processa:
   a. Se for inline (`mod foo { ... }`):
      - Calcula o `canonical_path` (raiz + nome do módulo).
      - Adiciona como nó com `is_inline = true`, `source_file` igual
        ao do pai, `has_custom_path = false`.
      - **Ignora qualquer `#[path]` aqui** (limitação da ADR-0003).
        Se presente, emitir warning via `tracing::warn!` (ou
        `eprintln!` no MVP) e seguir.
      - Recursão dentro do bloco do módulo (no AST, não em ficheiro).
   b. Se for externo (`mod foo;`):
      - Verifica se há `#[path]` entre os atributos.
      - Resolve o ficheiro físico (ver "Resolução de ficheiro" abaixo).
      - Lê e parseia recursivamente.
      - Adiciona o nó e os seus descendentes ao `ModuleTree`.

5. Retorna o `ModuleTree` populado.

### Resolução de ficheiro de módulo externo

Dado `mod foo;` em ficheiro `parent_file.rs` localizado em
`parent_dir/`:

1. **Se há `#[path = "x"]`**:
   - Resolver `x` relativo a `parent_dir`.
   - Se ficheiro existe: usar.
   - Se não existe: retornar `Err(TraverseError::ModuleFileNotFound)`.

2. **Sem `#[path]`** (resolução padrão):
   - Tentar `parent_dir/foo.rs`. Se existe: usar.
   - Tentar `parent_dir/foo/mod.rs`. Se existe: usar.
   - Se nenhum existe: retornar `Err(TraverseError::ModuleFileNotFound)`.

3. **Casos especiais**:
   - Se o ficheiro pai é o `entry_point` (`lib.rs` ou `main.rs`):
     `parent_dir` é `src/` do crate. Vale a regra normal.
   - Se o ficheiro pai é `xxx/mod.rs`: `parent_dir` é `xxx/`.
   - Se o ficheiro pai é `xxx.rs` (não-mod): `parent_dir` é
     `xxx/`. Submódulos vivem em `xxx/foo.rs` ou `xxx/foo/mod.rs`.

   Esta é a regra padrão do compilador Rust (Edição 2018+).

### Detecção de duplicatas por `#[cfg]` + `#[path]`

Quando dois `mod foo;` aparecem no mesmo ficheiro (típico de
`#[cfg(linux)] mod foo;` + `#[cfg(windows)] mod foo;`), como
`#[cfg]` é ignorado (ADR-0002), ambos seriam processados. A regra
da ADR-0003 é: o **primeiro encontrado vence**, os seguintes
geram warning e são ignorados.

Implementação:
- Manter um `HashSet<(NodeId, String)>` de (pai, nome) já
  processados durante a travessia.
- Ao processar `mod foo;`, verificar se `(pai_actual, "foo")` já
  está no set.
- Se sim: `tracing::warn!` ou `eprintln!` e ignorar.
- Se não: processar e adicionar ao set.

---

## Função auxiliar interna

```rust
fn traverse_file(
    tree: &mut ModuleTree,
    parent_node: NodeId,
    file_path: &Path,
    seen_children: &mut HashSet<(NodeId, String)>,
) -> Result<(), TraverseError>;
```

Função recursiva privada. Para cada `mod` encontrado no ficheiro:
- Verifica duplicatas (set de irmãos já vistos).
- Resolve o ficheiro do módulo (padrão ou `#[path]`).
- Adiciona o nó ao `tree` via `tree.add_child(...)`.
- Recursivamente trata o conteúdo do novo módulo.

---

## Tipo de erro

```rust
#[derive(Debug, thiserror::Error)]
pub enum TraverseError {
    #[error("Falha ao ler ficheiro: {path}")]
    FileReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Falha ao parsear ficheiro Rust: {file}")]
    ParseFailed {
        file: PathBuf,
        #[source]
        source: syn::Error,
    },

    #[error("Ficheiro de módulo não encontrado para 'mod {module}' em {parent_file}")]
    ModuleFileNotFound {
        module: String,
        parent_file: PathBuf,
        attempted_paths: Vec<PathBuf>,
    },

    #[error("Erro de construção da árvore: {0}")]
    TreeError(#[from] crystalline_dsm_core::entities::module_tree::TreeError),
}
```

---

## Dependências externas

Declaradas em `03_infra/Cargo.toml`:
- `syn` (com feature `full` para parsing completo de arquivos).
- `thiserror` (já presente).

Internas:
- `crystalline-dsm-core` (para `ModuleTree`, `ModuleNode`,
  `NodeId`, `TreeError`, e também `WorkspaceMember`).

NÃO usar:
- `cargo_metadata` directamente (a info de workspace vem via
  `WorkspaceMember`, que é L₁).
- `proc-macro2` / `quote` (não estamos a gerar código).

---

## Testes esperados

### Testes unitários (no próprio ficheiro)

Limitados. A maioria da lógica precisa de ficheiros no disco.
Casos possíveis sem filesystem:
- Verificar que o erro `ModuleFileNotFound` contém os campos
  esperados quando construído manualmente.

### Testes de integração (`03_infra/tests/module_traverser_tests.rs`)

Usando fixtures novas em `tests/fixtures/`:

1. **`module-tree-flat`**: crate único com `lib.rs` contendo 3
   `mod` declarations em ficheiros irmãos.
   ```
   src/lib.rs       (mod a; mod b; mod c;)
   src/a.rs         (vazio)
   src/b.rs         (vazio)
   src/c.rs         (vazio)
   ```
   Resultado esperado: `ModuleTree` com 4 nós (raiz + 3 filhos).
   Todos os filhos têm `is_inline = false`, `has_custom_path = false`.

2. **`module-tree-nested`**: crate com hierarquia mais profunda.
   ```
   src/lib.rs       (mod a;)
   src/a.rs         (mod b;)
   src/a/b.rs       (mod c;)
   src/a/b/c.rs     (vazio)
   ```
   Resultado esperado: árvore linear com 4 nós.
   `canonical_path` do nó mais profundo: `crate_name::a::b::c`.

3. **`module-tree-with-mod-rs`**: variação usando `mod.rs`.
   ```
   src/lib.rs       (mod a;)
   src/a/mod.rs     (mod b;)
   src/a/b.rs       (vazio)
   ```
   Resultado esperado: 3 nós, mesmo formato de `canonical_path`
   que a versão anterior.

4. **`module-tree-inline`**: módulos inline.
   ```
   src/lib.rs       (mod a { mod b { fn f() {} } })
   ```
   Resultado esperado: 3 nós (raiz, `a`, `a::b`). Todos os
   inline têm `is_inline = true` e `source_file` apontando para
   `lib.rs`.

5. **`module-tree-with-path-attr`**: `#[path]` em uso.
   ```
   src/lib.rs       (#[path = "custom/special.rs"] mod x;)
   src/custom/special.rs  (vazio)
   ```
   Resultado esperado: 2 nós (raiz + `x`). O nó `x` tem
   `has_custom_path = true` e `source_file` apontando para
   `src/custom/special.rs`.

6. **`module-tree-missing-file`**: `mod` declarado sem ficheiro.
   ```
   src/lib.rs       (mod inexistente;)
   ```
   Resultado esperado: `Err(TraverseError::ModuleFileNotFound)`.

7. **`module-tree-syntax-error`**: ficheiro com erro de sintaxe
   Rust.
   ```
   src/lib.rs       (mod a; fn !bad syntax!)
   ```
   Resultado esperado: `Err(TraverseError::ParseFailed)`.

8. **`module-tree-cfg-duplicate`**: simulação do caso
   `#[cfg] + #[path]`.
   ```
   src/lib.rs:
     #[cfg(target_os = "linux")]
     #[path = "platform/linux.rs"]
     mod platform;

     #[cfg(target_os = "windows")]
     #[path = "platform/windows.rs"]
     mod platform;

   src/platform/linux.rs (vazio)
   src/platform/windows.rs (vazio)
   ```
   Resultado esperado: 2 nós (raiz + `platform`). O `platform`
   resolve para `linux.rs` (primeiro encontrado). Warning é
   emitido para o segundo (não testado neste teste, apenas
   ausência do segundo nó é verificada).

9. **Crate de Typst real (smoke test)**: rodar a função contra
   um membro real do `lab/typst-original/` (a definir qual). Não
   verificar conteúdo exacto da árvore; apenas verificar que:
   - Função retorna `Ok`.
   - `node_count` é > 10 (sanity check).
   - Nenhum nó tem `canonical_path` vazio ou inconsistente.

   Este teste pode ser marcado como `#[ignore]` por defeito para
   não exigir o `lab/typst-original/` em CI. Rodável manualmente.

---

## Estrutura das fixtures novas

```
tests/fixtures/
├── (fixtures existentes do Passo 1.1)
├── module-tree-flat/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── a.rs
│       ├── b.rs
│       └── c.rs
├── module-tree-nested/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── a.rs
│       └── a/
│           ├── b.rs
│           └── b/
│               └── c.rs
├── module-tree-with-mod-rs/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── a/
│           ├── mod.rs
│           └── b.rs
├── module-tree-inline/
│   ├── Cargo.toml
│   └── src/lib.rs
├── module-tree-with-path-attr/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── custom/
│           └── special.rs
├── module-tree-missing-file/
│   ├── Cargo.toml
│   └── src/lib.rs
├── module-tree-syntax-error/
│   ├── Cargo.toml
│   └── src/lib.rs
└── module-tree-cfg-duplicate/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        └── platform/
            ├── linux.rs
            └── windows.rs
```

Cada fixture precisa ser um crate Rust válido (sintacticamente).
A fixture `module-tree-syntax-error` é a única excepção: o seu
`lib.rs` é propositadamente inválido para testar o caminho de
erro. Esta fixture **não** deve ser referenciada por outros
testes (`cargo check` na pasta falharia).

---

## Critério de aceitação do prompt

- O ficheiro `03_infra/src/module_traverser.rs` existe e compila.
- A função `traverse_crate` tem a assinatura especificada.
- O enum `TraverseError` está definido como especificado.
- As 8 fixtures novas existem em `tests/fixtures/`.
- Os 8 testes principais passam (o teste 9 é opcional/ignored).
- `cargo clippy --all-targets` passa sem warnings novos.
- Nenhum `panic!`, `unwrap()` ou `expect()` em código de
  produção (excepto em testes).
- O módulo não exporta tipos de `syn` na sua API pública.
- O módulo `module_traverser` está exportado em
  `03_infra/src/lib.rs`.

---

## Limitações conhecidas e documentadas

(Já cobertas por ADRs e Estudo Prévio. Listadas aqui apenas para
referência local.)

1. `mod` dentro de macros não é detectado.
2. `extern crate` não é processado (Edição 2015 não suportada).
3. `#[cfg]` é completamente ignorado (ADR-0002).
4. `#[path]` dentro de módulos inline gera warning e é ignorado
   (ADR-0003).
5. Imports (`use` statements) não são extraídos neste passo
   (Passo 1.3).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-20 | Implementação do traversador de módulos e testes de integração | `03_infra/src/module_traverser.rs`, `03_infra/src/lib.rs`, `03_infra/tests/module_traverser_tests.rs`, `tests/fixtures/*` |

---

## Hash do prompt

A calcular após aprovação.
