# ⚖️ ADR-0004: Granularidade do Nó do Grafo

**Status**: `PROPOSTO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm`
**Passo do roadmap relacionado**: 1.4 — Construção do grafo

---

## Contexto

O `crystalline-dsm` constrói um grafo de dependências para gerar a
DSM. Cada nó do grafo precisa representar "algo" do código fonte.
A escolha de o que esse "algo" é tem consequências significativas:

- Afecta a interface da DSM (o que aparece nas linhas e colunas).
- Afecta a estabilidade do grafo entre versões (renomear arquivo é
  diferente de renomear módulo).
- Afecta a serialização para JSON canónico.
- Afecta a integração com `crystalline.toml` (regras são escritas
  contra que tipo de identificador?).

Esta decisão precisa ser tomada antes da implementação do Passo
1.4 (Construção do grafo), e idealmente antes do Passo 1.2
(Travessia de módulos), porque o tipo de nó produzido pela
travessia depende disto.

---

## Alternativas consideradas

### Alternativa A — Nó é o ficheiro físico

Cada `.rs` no projecto é um nó. Identificador: caminho relativo à
raiz do workspace (ex: `01_core/src/entities/workspace.rs`).

**Prós:**
- Mapeamento directo para o filesystem.
- Útil para integração com ferramentas externas que operam em
  ficheiros (linters, formatters, git).
- Fácil de mostrar visualmente (cada linha da DSM é um ficheiro).

**Contras:**
- Identificador instável: renomear ou mover ficheiro muda o nó.
- Não captura módulos inline (`mod foo { ... }`) que não têm
  ficheiro próprio.
- Identificador acoplado à organização física, que pode mudar
  durante refactor.

### Alternativa B — Nó é o módulo lógico

Cada módulo Rust é um nó. Identificador: caminho lógico canónico
(ex: `crystalline_dsm_core::entities::workspace`).

**Prós:**
- Identificador estável: mover ficheiro não muda o nó.
- Captura naturalmente módulos inline.
- Alinhado com o sistema de tipos do Rust (`use` statements
  referem-se a caminhos lógicos, não a ficheiros).
- Identificador legível para humanos (linha da DSM é
  `crate::a::b`, não `src/a/b.rs`).

**Contras:**
- Mapeamento para ficheiro físico requer estrutura adicional (o
  nó precisa carregar referência ao ficheiro para navegação).
- Em casos raros, dois módulos com mesmo nome lógico podem existir
  em crates diferentes (precisa qualificar com nome do crate).

### Alternativa C — Nó composto (ficheiro + módulo)

Cada nó carrega ambos os identificadores. A DSM pode mostrar um ou
outro conforme configuração.

**Prós:**
- Flexibilidade máxima.
- Suporta diferentes utilizadores (devs vs arquitectos).

**Contras:**
- Modelo de dados mais complexo.
- Não resolve a questão de "qual é o ID canónico para
  serialização e regras"?

### Alternativa D — Nó é o item (função, struct, etc)

Granularidade fina: cada item de código (função pública, struct,
trait) é um nó.

**Prós:**
- Análise mais precisa de acoplamento.
- Permite detectar dependências cirúrgicas (módulo A só depende
  da função `foo` de B, não do módulo todo).

**Contras:**
- Grafo explode em tamanho (10-100x maior).
- DSM fica ilegível em projectos médios.
- Implementação muito mais complexa (resolução de nomes, análise
  de tipos).
- **Fora do escopo declarado do MVP** (ADR-0001).

---

## Decisão

**Alternativa B: nó é o módulo lógico.**

Cada nó do grafo do `crystalline-dsm` representa um módulo Rust,
identificado pelo caminho lógico canónico qualificado com o nome
do crate.

### Formato do identificador

```
<crate_name>::<module_path>
```

Exemplos:
- `crystalline_dsm_core::entities::workspace`
- `crystalline_dsm_infra::cargo_metadata_reader`
- `typst_eval::compiler` (módulo raiz do crate `typst_eval`)
- `typst_eval` (próprio crate, ponto de entrada `lib.rs`)

Para módulos inline:
- `crystalline_dsm_core::entities::workspace::tests`
  (o `mod tests` inline aparece como nó separado)

### Informação adicional carregada pelo nó

Embora o identificador canónico seja o caminho lógico, o nó
carrega também:

```rust
pub struct ModuleNode {
    /// Identificador canónico. Ex: "crate_x::a::b".
    pub canonical_path: String,

    /// Nome do crate. Ex: "crate_x".
    pub crate_name: String,

    /// Caminho relativo do crate (caminho lógico sem o prefixo do crate).
    /// Ex: "a::b". Vazio para o módulo raiz do crate.
    pub module_path: Vec<String>,

    /// Ficheiro físico que contém este módulo. Para módulos inline,
    /// é o ficheiro do módulo pai.
    pub source_file: PathBuf,

    /// `true` se é módulo inline, `false` se tem ficheiro próprio.
    pub is_inline: bool,
}
```

### Critério de unicidade

Dois nós são iguais se e somente se têm o mesmo `canonical_path`.
Outros campos são informação derivada.

---

## Justificação

1. **Estabilidade**: Identificadores baseados em caminho lógico
   não mudam com refactor de organização de ficheiros (mover
   `a.rs` para `a/mod.rs` não muda o módulo).

2. **Alinhamento com o ecossistema**: Imports Rust (`use a::b::c`)
   referem-se a caminhos lógicos. Regras de camada em
   `crystalline.toml` são naturalmente expressas em termos
   lógicos.

3. **Suporte a módulos inline**: A Alternativa A não consegue
   representá-los sem inventar identificadores artificiais. A
   Alternativa B representa-os naturalmente como sub-módulos.

4. **Trade-off aceitável**: A informação de ficheiro físico não é
   perdida; é carregada como campo adicional no nó, disponível para
   visualização e navegação.

5. **Compatibilidade com Alternativa D futura**: Se mais tarde
   adicionarmos granularidade fina (itens), o caminho lógico é o
   prefixo natural (`crate::a::b::funcao_x`). Adoptar Alternativa B
   agora não bloqueia evolução futura.

---

## Consequências

### ✅ Positivas

- Identificadores estáveis para versionamento da DSM em CI.
- Compatibilidade natural com `crystalline.toml` (regras de camada
  expressas em caminhos lógicos).
- Suporte limpo para módulos inline.
- Modelo de dados claro: um único conceito de "nó", com metadata
  adicional.

### ❌ Negativas

- Cada nó carrega mais informação que apenas um string (PathBuf
  adicional, vector de strings).
- Renderização da DSM precisa decidir como mostrar o
  identificador (curto ou completo?). Decisão de UI, fora desta
  ADR.

### ⚙️ Acções decorrentes

- A struct `ModuleNode` em L₁ deve seguir exactamente o formato
  desta ADR. Será objecto de Prompt L0 do Passo 1.2.
- A serialização JSON do nó (Passo 1.4) deve usar `canonical_path`
  como chave primária; outros campos como propriedades.
- Documentar em `crystalline.toml`: regras de camada referenciam
  módulos pelo `canonical_path`, com suporte a glob
  (`crystalline_dsm_core::entities::*`).

---

## Referências

- ADR-0001 — Criação da ferramenta (define escopo do MVP).
- ADR-0002 — Tratamento de `#[cfg]`.
- ADR-0003 — Tratamento de `#[path]`.
- The Rust Reference — Paths:
  https://doc.rust-lang.org/reference/paths.html
