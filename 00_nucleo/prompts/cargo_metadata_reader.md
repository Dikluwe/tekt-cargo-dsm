# Prompt L0: Leitor de `cargo_metadata` (L₃)

**Camada**: L₃ (Infraestrutura)
**Ficheiro alvo**: `03_infra/src/cargo_metadata_reader.rs`
**Passo do roadmap**: 1.1 — Resolução de workspace
**Status**: IMPLEMENTADO

---

## Decisões de design assumidas

1. **Tratamento de erros**: usar `Result<T, CargoMetadataError>` com
   enum de erro próprio via `thiserror`. Sem `panic!`, sem `unwrap()`
   em código de produção. Em código de teste, `expect` com mensagem
   descritiva é aceitável.

2. **Dependência da entidade**: este módulo importa
   `crystalline_dsm_core::entities::workspace::{Workspace, WorkspaceMember, EntryKind}`.
   L₃ depende de L₁ (regra cristalina padrão).

3. **Fixture compartilhada**: testes referenciam `tests/fixtures/` na
   raiz do workspace via `CARGO_MANIFEST_DIR` resolvido para a raiz do
   workspace (não da crate L₃).

4. **Escopo do MVP**: apenas leitura de workspace. Não cobre análise
   de módulos individuais (isso é Passo 1.2). Não cobre dependências
   externas (crates.io). Apenas membros do workspace.

---

## Contexto

Este módulo é a ponte entre o ecossistema Cargo (representado pela
crate de terceiros `cargo_metadata`) e o modelo de dados puro de L₁
(`Workspace`, `WorkspaceMember`).

A função principal recebe um caminho de filesystem e retorna uma
instância de `Workspace` totalmente construída, com todos os
caminhos absolutos resolvidos e os pontos de entrada de cada crate
identificados.

A presença deste módulo isola completamente o resto do sistema da
API do `cargo_metadata`. Se a versão da crate mudar, apenas este
ficheiro precisa ser ajustado.

---

## Função pública principal

```rust
pub fn read_workspace(workspace_path: &Path) -> Result<Workspace, CargoMetadataError>;
```

### Comportamento

1. Recebe um caminho que aponta para um diretório contendo um
   `Cargo.toml` de workspace, ou para o próprio `Cargo.toml`.
2. Executa `cargo metadata` via `MetadataCommand` da crate
   `cargo_metadata`.
3. Para cada `workspace_member` retornado:
   a. Extrai o nome (`package.name`).
   b. Resolve o diretório raiz (parent do `manifest_path`).
   c. Identifica os targets do tipo `lib` e/ou `bin`.
   d. Determina o `EntryKind` e o `entry_point`:
      - Se há lib e bin: `LibraryAndBinary { main_path }`,
        `entry_point` aponta para o `lib.rs`.
      - Se há apenas lib: `Library`, `entry_point` aponta para `lib.rs`.
      - Se há apenas bin: `Binary`, `entry_point` aponta para `main.rs`.
4. Constrói e retorna `Workspace` com todos os membros.

### Resolução de pontos de entrada

O `cargo_metadata::Target` expõe `src_path` que aponta directamente
para o ficheiro de entrada (ex: `/abs/path/src/lib.rs`). Não tentar
resolver manualmente seguindo a convenção `src/lib.rs`; usar sempre
o que `cargo_metadata` retorna.

Para crates com múltiplos binários (`[[bin]]` repetidos), considerar
apenas o primeiro binário no MVP. Documentar limitação. Casos
complexos (binários múltiplos) ficam para versão posterior.

---

## Tipo de erro

```rust
#[derive(Debug, thiserror::Error)]
pub enum CargoMetadataError {
    #[error("Caminho inválido ou inacessível: {path}")]
    InvalidPath { path: PathBuf },

    #[error("Falha ao executar 'cargo metadata': {source}")]
    MetadataExecutionFailed {
        #[from]
        source: cargo_metadata::Error,
    },

    #[error("Workspace member '{name}' não tem nem lib nem binário")]
    NoEntryPoint { name: String },

    #[error("Workspace não contém nenhum membro")]
    EmptyWorkspace,
}
```

Notas:

- `InvalidPath` é retornado antes de chamar `cargo_metadata`, quando o
  caminho não existe ou não é um diretório válido.
- `MetadataExecutionFailed` envolve qualquer erro vindo da crate
  `cargo_metadata` (ficheiro malformado, comando ausente, etc).
- `NoEntryPoint` cobre o caso patológico de um crate sem targets.
- `EmptyWorkspace` é retornado se o workspace existe mas não tem
  membros (provavelmente erro de configuração do utilizador).

---

## Dependências externas

Declaradas em `03_infra/Cargo.toml`:

- `cargo_metadata` — versão estável recente (verificar antes da
  implementação qual a versão actual).
- `thiserror` — para definição do enum de erro.

Dependência interna:

- `crystalline-dsm-core` (L₁) — para os tipos `Workspace`,
  `WorkspaceMember`, `EntryKind`.

---

## Função auxiliar interna

```rust
fn classify_targets(package: &cargo_metadata::Package)
    -> Result<(EntryKind, PathBuf), CargoMetadataError>;
```

Responsável por examinar `package.targets` e produzir o par
`(EntryKind, entry_point)`. Função privada do módulo, mas suficientemente
isolada para ser testada via testes unitários inline com pacotes
construídos manualmente (se viável) ou via testes de integração com
fixtures (mais provável).

Retorna `NoEntryPoint { name }` se o package não tem nem lib nem
binário.

---

## Testes esperados

### Testes unitários (`#[cfg(test)]` no próprio ficheiro)

Limitados a casos que podem ser construídos sem filesystem real.
Provavelmente poucos, dado que `MetadataCommand` precisa de um
Cargo.toml real para funcionar.

### Testes de integração (`03_infra/tests/cargo_metadata_reader_tests.rs`)

Usando as fixtures em `tests/fixtures/` (raiz do workspace):

1. **`empty-workspace`**: workspace válido sem membros.
   Resultado esperado: `Err(CargoMetadataError::EmptyWorkspace)`.

2. **`single-lib-crate`**: workspace com 1 crate biblioteca.
   Resultado esperado: `Ok(Workspace)` com 1 membro, `entry_kind = Library`.

3. **`single-bin-crate`**: workspace com 1 crate binário.
   Resultado esperado: `Ok(Workspace)` com 1 membro, `entry_kind = Binary`.

4. **`lib-and-bin-crate`**: workspace com 1 crate que tem ambos.
   Resultado esperado: `Ok(Workspace)` com 1 membro,
   `entry_kind = LibraryAndBinary`.

5. **`multi-crate-workspace`**: workspace com 3 crates (A, B, C) com
   dependência linear A→B→C. Cada um é biblioteca.
   Resultado esperado: `Ok(Workspace)` com 3 membros na ordem do
   `cargo metadata`, todos com `entry_kind = Library`.

6. **`invalid-path`**: caminho que não existe.
   Resultado esperado: `Err(CargoMetadataError::InvalidPath { .. })`.

7. **`not-a-workspace`**: caminho válido mas sem `Cargo.toml` de
   workspace.
   Resultado esperado: `Err(CargoMetadataError::MetadataExecutionFailed { .. })`.

---

## Estrutura das fixtures

Criar em `tests/fixtures/` na raiz do workspace. Cada fixture é um
mini-workspace Cargo real que compila.

```
tests/fixtures/
├── empty-workspace/
│   └── Cargo.toml          # [workspace] members = []
├── single-lib-crate/
│   ├── Cargo.toml          # [workspace] members = ["a"]
│   └── a/
│       ├── Cargo.toml
│       └── src/lib.rs      # pub fn hello() {}
├── single-bin-crate/
│   ├── Cargo.toml
│   └── a/
│       ├── Cargo.toml
│       └── src/main.rs     # fn main() {}
├── lib-and-bin-crate/
│   ├── Cargo.toml
│   └── a/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── main.rs
├── multi-crate-workspace/
│   ├── Cargo.toml          # members = ["a", "b", "c"]
│   ├── a/
│   │   ├── Cargo.toml      # depends on b
│   │   └── src/lib.rs
│   ├── b/
│   │   ├── Cargo.toml      # depends on c
│   │   └── src/lib.rs
│   └── c/
│       ├── Cargo.toml
│       └── src/lib.rs
└── not-a-workspace/
    └── README.md           # diretório sem Cargo.toml
```

`invalid-path` não é uma fixture (caminho inexistente, construído no
teste).

---

## Critério de aceitação do prompt

Esta especificação é considerada implementada quando:

- O ficheiro `03_infra/src/cargo_metadata_reader.rs` existe e compila.
- A função `read_workspace` tem a assinatura especificada.
- O enum `CargoMetadataError` está definido exactamente como
  especificado.
- As 6 fixtures listadas existem em `tests/fixtures/` e compilam
  individualmente com `cargo check`.
- Os 7 testes de integração passam.
- `cargo clippy -p crystalline-dsm-infra` passa sem warnings.
- Nenhum `panic!`, `unwrap()` ou `expect()` aparece no código de
  produção (apenas em testes).
- O módulo não exporta nenhum tipo de `cargo_metadata` na sua API
  pública (isolamento de fronteira).

---

## Limitações conhecidas e documentadas

1. Workspaces com binários múltiplos no mesmo crate: apenas o primeiro
   é considerado. Limitação a remover em versão posterior.

2. Crates com `path` apontando para fora do workspace declarado: não
   coberto. `cargo_metadata` reporta apenas `workspace_members`
   declarados explicitamente.

3. Workspaces virtuais vs não-virtuais: ambos devem funcionar (o
   `cargo_metadata` abstrai a diferença), mas só workspaces virtuais
   estão nas fixtures. Workspace não-virtual (raiz é também um crate)
   pode ser adicionado em fixture futura.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-20 | Implementação do leitor cargo_metadata e testes de integração | `03_infra/src/cargo_metadata_reader.rs`, `03_infra/src/lib.rs`, `03_infra/tests/cargo_metadata_reader_tests.rs`, `tests/fixtures/*` |

---

## Hash do prompt

A calcular após aprovação.
