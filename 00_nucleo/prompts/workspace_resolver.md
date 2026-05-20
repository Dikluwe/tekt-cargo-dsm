# Prompt L0: Entidade `Workspace` (L₁)

**Camada**: L₁ (Núcleo)
**Ficheiro alvo**: `01_core/src/entities/workspace.rs`
**Passo do roadmap**: 1.1 — Resolução de workspace
**Status**: IMPLEMENTADO

---

## Decisões de design assumidas

Estas decisões foram tomadas no início do Passo 1.1. Estão registadas
aqui para revisão. Caso alguma seja rejeitada, este prompt precisa ser
atualizado antes da implementação.

1. **Representação de caminhos**: usar `std::path::PathBuf` em L₁.
   Alternativa considerada e rejeitada para o MVP: `String` opaca.
   Motivo da escolha: `PathBuf` é stdlib, não introduz dependência
   externa, e é o tipo natural que L₃ vai produzir. Custo aceitável de
   admitir um conceito de I/O em L₁ é menor que o custo de tradução
   manual em toda fronteira.

2. **Identidade dos membros**: usar `name: String` como identificador
   lógico. Não usar `cargo_metadata::PackageId` (tipo de terceiros).

3. **Erros**: L₁ não modela erros de leitura (isso é L₃). L₁ define
   apenas as structs de dados; falhas de construção (se houverem) são
   modeladas em L₃.

---

## Contexto

O `crystalline-dsm` precisa representar a estrutura de um workspace
Cargo de forma agnóstica à ferramenta de leitura (`cargo_metadata`).
Esta entidade é o produto canónico da leitura física feita em L₃ e o
input para todas as análises subsequentes (extracção de módulos,
construção do grafo, detecção de ciclos).

A separação é o que protege o núcleo contra mudanças de versão do
`cargo_metadata` e contra acoplamento a tipos de terceiros.

---

## Definição das structs

### `WorkspaceMember`

Representa um único crate dentro de um workspace.

```rust
pub struct WorkspaceMember {
    /// Nome lógico do crate, conforme declarado em `Cargo.toml`.
    /// Exemplo: "crystalline-dsm-core".
    pub name: String,

    /// Caminho absoluto do diretório raiz do crate (onde está o `Cargo.toml`).
    /// Exemplo: "/home/user/projeto/01_core".
    pub crate_root: PathBuf,

    /// Caminho absoluto do ficheiro de entrada do crate.
    /// Para libs: ".../src/lib.rs".
    /// Para binários: ".../src/main.rs".
    /// Para crates que têm ambos, ver `entry_kind`.
    pub entry_point: PathBuf,

    /// Tipo do ponto de entrada: biblioteca, binário, ou ambos.
    pub entry_kind: EntryKind,
}

pub enum EntryKind {
    /// Apenas `lib.rs` (crate biblioteca).
    Library,

    /// Apenas `main.rs` (crate binário).
    Binary,

    /// Tem ambos. Neste caso, `entry_point` aponta para `lib.rs`
    /// (preferência para análise estrutural).
    LibraryAndBinary { main_path: PathBuf },
}
```

### `Workspace`

Representa o workspace completo.

```rust
pub struct Workspace {
    /// Caminho absoluto da raiz do workspace (onde está o `Cargo.toml`
    /// do workspace).
    pub root: PathBuf,

    /// Lista de crates membros do workspace.
    /// Ordem: preservada da ordem retornada pela leitura física
    /// (geralmente ordem alfabética por `name`, mas L₁ não impõe).
    pub members: Vec<WorkspaceMember>,
}
```

---

## Invariantes (verificadas pelo construtor em L₃)

L₁ não valida invariantes (assume que L₃ entrega dados válidos). As
seguintes propriedades devem ser garantidas por quem constrói as
instâncias:

1. **Caminhos absolutos**: `root`, `crate_root` e `entry_point` são
   sempre absolutos. L₁ não tenta resolver caminhos relativos.

2. **Unicidade de nomes**: `members` não contém dois `WorkspaceMember`
   com o mesmo `name`. Cargo já garante isto a nível de workspace.

3. **Existência física**: os caminhos referenciados existem no
   filesystem no momento da construção. L₁ não verifica novamente; se
   o ficheiro for removido depois, a struct fica obsoleta mas não
   inválida do ponto de vista de L₁.

4. **Coerência de `entry_kind`**: se `entry_kind` é `Library`,
   `entry_point` aponta para `lib.rs`. Se `Binary`, para `main.rs`.
   Se `LibraryAndBinary`, `entry_point` aponta para `lib.rs` e
   `main_path` para `main.rs`.

---

## Operações em L₁

Esta entidade é primariamente um contentor de dados. As operações em
L₁ são apenas inspecção:

```rust
impl Workspace {
    /// Procura um membro pelo nome. Retorna `None` se não encontrado.
    pub fn find_member(&self, name: &str) -> Option<&WorkspaceMember>;

    /// Quantidade total de membros.
    pub fn member_count(&self) -> usize;

    /// Itera sobre os membros que são bibliotecas (Library ou
    /// LibraryAndBinary).
    pub fn libraries(&self) -> impl Iterator<Item = &WorkspaceMember>;

    /// Itera sobre os membros que são binários (Binary ou
    /// LibraryAndBinary).
    pub fn binaries(&self) -> impl Iterator<Item = &WorkspaceMember>;
}
```

Sem operações que mutam estado. Sem operações que fazem I/O.

---

## Derives obrigatórios

- `Debug` — todas as structs e enums.
- `Clone` — todas as structs e enums.
- `PartialEq` e `Eq` — para uso em testes e comparação em fixtures.
- `Hash` — apenas em `WorkspaceMember` (não em `Workspace`), para
  permitir uso em `HashSet` / `HashMap` em análises futuras.

`Serialize` e `Deserialize` (via `serde`) ficam **fora do MVP** desta
entidade. Quando o JSON canónico for definido (Passo 1.4 do roadmap),
um prompt separado decide se serde entra na lista de
`l1_allowed_external` ou se a serialização é feita por adaptador em
L₃.

---

## Dependências externas

Nenhuma. Apenas `std`.

Não usar:
- `cargo_metadata` (proibido em L₁).
- `serde` (adiado para Passo 1.4).
- `thiserror` (não há erros nesta entidade).

---

## Testes esperados

Localização: `01_core/src/entities/workspace.rs` (testes inline com
`#[cfg(test)]`).

Cobertura mínima:

1. **Construção literal de `Workspace` com 0, 1 e 3 membros**:
   verifica que as structs se constroem sem erro e os campos são
   acessíveis.

2. **`find_member`**: retorna `Some` para nome existente, `None` para
   nome inexistente.

3. **`member_count`**: retorna o número correto em workspaces de
   tamanhos 0, 1, 3.

4. **`libraries` e `binaries`**: filtram corretamente um workspace
   misto com 1 lib, 1 binário, 1 com ambos.

5. **`PartialEq`**: dois `Workspace` construídos com os mesmos dados
   são iguais; com dados diferentes, diferentes.

Sem testes de I/O, sem fixtures de filesystem nesta entidade.

---

## Critério de aceitação do prompt

Esta especificação é considerada implementada quando:

- O ficheiro `01_core/src/entities/workspace.rs` existe e compila.
- Todas as structs e enums acima estão definidas exactamente como
  especificado.
- Todos os métodos listados em "Operações em L₁" existem e têm a
  assinatura especificada.
- Os 5 grupos de testes acima passam.
- `cargo clippy -p crystalline-dsm-core` passa sem warnings.
- Nenhuma importação de `cargo_metadata`, `serde` ou `thiserror`
  aparece neste ficheiro.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-20 | Implementação da entidade Workspace e testes unitários | `01_core/src/entities/workspace.rs`, `01_core/src/entities/mod.rs`, `01_core/src/lib.rs` |

---

## Hash do prompt

A calcular após aprovação. (Padrão Tekt: hash do conteúdo deste
ficheiro registado em `prompt-history.md` no momento da
implementação.)
