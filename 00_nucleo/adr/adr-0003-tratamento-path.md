# ⚖️ ADR-0003: Tratamento de `#[path = "..."]` no MVP

**Status**: `PROPOSTO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm`
**Passo do roadmap relacionado**: 1.2 — Travessia de módulos por crate

---

## Contexto

Por defeito, quando o compilador Rust encontra uma declaração
`mod foo;` num ficheiro `src/lib.rs`, ele procura o módulo em
`src/foo.rs` ou `src/foo/mod.rs`. O atributo `#[path = "..."]`
permite especificar um caminho diferente:

```rust
#[path = "platform/linux.rs"]
mod platform;
```

Neste exemplo, o módulo `platform` é lido de
`src/platform/linux.rs`, não dos locais padrão.

O atributo é usado tipicamente em conjunto com `#[cfg]` para
trocar implementações entre plataformas:

```rust
#[cfg(target_os = "linux")]
#[path = "os/linux.rs"]
mod os;

#[cfg(target_os = "windows")]
#[path = "os/windows.rs"]
mod os;
```

Também aparece em testes para partilha de fixtures e em código
gerado por scripts de build.

O `crystalline-dsm` precisa decidir se resolve esses caminhos
customizados ou se opera apenas com a convenção padrão.

---

## Alternativas consideradas

### Alternativa A — Implementar resolução completa de `#[path]`

Quando o parser encontra `#[path = "x"]` em cima de `mod foo;`,
lê o ficheiro indicado em `x` em vez do local padrão. Caminhos
relativos são resolvidos a partir do diretório do ficheiro onde
a declaração aparece.

**Prós:**
- Cobre o caso real sem falhas silenciosas.
- Grafo correcto para projectos que usam `#[path]`.
- Complexidade de implementação moderada (estimativa: 30-50
  linhas de código além da resolução padrão).

**Contras:**
- Mais código a manter.
- Casos sutis a tratar (caminhos com `..`, caminhos absolutos,
  caminhos que apontam para fora do crate).

### Alternativa B — Não implementar, emitir warning

Quando o parser encontra `#[path]`, emite um aviso explícito
("módulo `foo` usa #[path], não suportado neste MVP, ignorado")
e segue. O módulo não entra no grafo.

**Prós:**
- Implementação simples (detectar atributo, registar warning).
- Falhas explícitas, não silenciosas.

**Contras:**
- Ferramenta incompleta para projectos que dependem de `#[path]`.
- Grafo incorrecto, mesmo com warning.

### Alternativa C — Implementar parcial (apenas caminhos relativos simples)

Cobrir `#[path = "foo.rs"]` e `#[path = "subdir/foo.rs"]`. Não
cobrir caminhos com `..`, paths absolutos, ou outros casos
complexos. Para casos não cobertos, emitir warning.

**Prós:**
- Cobre maioria dos casos reais.
- Complexidade menor que Alternativa A.

**Contras:**
- Fronteira de "suportado vs não suportado" pouco clara.
- Pode gerar bugs sutis em casos limítrofes.

---

## Decisão

**Alternativa A: implementar resolução completa de `#[path]` no MVP.**

O parser de travessia de módulos do `crystalline-dsm` lê o atributo
`#[path]` via `syn` quando presente, e resolve o caminho indicado
para localizar o ficheiro do módulo.

### Regras de resolução

1. O caminho em `#[path = "x"]` é tratado como relativo ao diretório
   onde está o ficheiro que contém a declaração `mod`.

   Excepção: declarações `mod` dentro de blocos `mod nome { ... }`
   inline têm regras diferentes no compilador Rust real. Para o
   MVP, **não suportar** `#[path]` dentro de módulos inline.
   Emitir warning se encontrar e seguir sem incluir o módulo.

2. Se o caminho for absoluto (raro mas legal), usar como está.

3. Caminhos com `..` são permitidos e resolvidos normalmente via
   `std::path` (com `canonicalize` para normalizar).

4. Se o ficheiro indicado não existir, retornar erro
   `ModuleFileNotFound { declaration_at, expected_path }` (a ser
   definido no enum de erros do parser, fora do escopo desta ADR).

### Casos especiais

- `#[path]` em conjunto com `#[cfg]`: como a ADR-0002 decidiu
  ignorar `#[cfg]`, o `#[path]` é sempre aplicado. Isso pode causar
  conflito: dois `mod foo;` com `#[path]` diferentes (um para
  Linux, outro para Windows) seriam ambos lidos. Decisão: **o
  primeiro encontrado vence**, os seguintes geram warning de "módulo
  já registado, ignorando".

- `#[path]` em testes (`#[cfg(test)]` + `#[path]`): mesma regra. O
  ficheiro é incluído no grafo, marcado como módulo normal.

---

## Justificação da escolha

1. **Falhas silenciosas são piores que código a mais**: A
   Alternativa B emite warning mas o grafo fica errado. Para uma
   ferramenta de análise arquitectural, um grafo errado é mais
   prejudicial que código adicional.

2. **Custo de implementação razoável**: A resolução real é
   essencialmente "ler atributo, juntar com diretório pai,
   verificar existência". Não é trabalho complexo.

3. **Caso de uso justifica**: O Typst (caso de uso principal) não
   usa `#[path]` actualmente, mas qualquer crate cross-platform usa.
   Se a intenção é ferramenta generalista, suporte é necessário.

4. **A Alternativa C tem o pior dos dois mundos**: complexidade
   média com cobertura incompleta. Sem ganho claro sobre A.

---

## Consequências

### ✅ Positivas

- Parser cobre o conjunto realista de projectos Rust.
- Sem falhas silenciosas em projectos cross-platform.
- Comportamento alinhado com o compilador Rust real (na medida do
  que o MVP cobre).

### ❌ Negativas

- Código de travessia de módulos fica ~30-50 linhas maior.
- Mais casos para testar nas fixtures (precisamos de uma fixture
  específica para `#[path]`).
- Limitação documentada: `#[path]` dentro de módulos inline não
  é suportado no MVP.

### ⚙️ Acções decorrentes

- Criar fixture `tests/fixtures/path-attribute-crate/` para testes
  de integração desta funcionalidade. Conteúdo mínimo:
  - `Cargo.toml` declarando `[[lib]]`.
  - `src/lib.rs` com `#[path = "custom/special.rs"] mod x;`.
  - `src/custom/special.rs` com módulo trivial.
- Adicionar caso de teste ao Passo 1.2: travessia de módulos com
  `#[path]` produz o módulo no grafo na posição esperada.
- Documentar limitação de `#[path]` dentro de módulos inline em
  `docs/limitacoes.md`.

---

## Referências

- The Rust Reference — Module path attribute:
  https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute
- ADR-0002 — Tratamento de `#[cfg]` (decisão complementar).
- Estudo Prévio (Passo 0.1) — Seção 1 sobre resolução de módulos.
