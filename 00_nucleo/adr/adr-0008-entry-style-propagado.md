# ⚖️ ADR-0008: Propagação de "entry-style" para o Resolvedor de Módulos

**Status**: `ACEITO`
**Data**: 2026-05-20
**Projecto**: `crystalline-dsm`
**Passos afectados**: 1.2 (travessia de módulos) — revisão de código
  `IMPLEMENTADO`.

---

## Contexto

Após aplicar a ADR-0007 (extensão do `EntryKind`) e re-executar o
smoke test contra `lab/typst-original/`, a Fase 2 (travessia)
falhou para o membro `typst-tests`:

```
Falhas em traversal:
  Crate: typst-tests
    Tipo: ModuleFileNotFound
    Módulo procurado: args
    Declarado em: /.../typst-original/tests/src/tests.rs
    Caminhos tentados:
      - /.../typst-original/tests/src/tests/args.rs
      - /.../typst-original/tests/src/tests/args/mod.rs
```

O ficheiro real está em `tests/src/args.rs` — irmão de
`tests.rs`, no mesmo directório. `tests.rs` é o entry point do
target `[[test]]` do crate (declarado em `Cargo.toml`).

### Causa raiz

Em `03_infra/src/module_traverser.rs`, a função `resolve_module_path`
decide se um ficheiro é "entry-style" (resolução `mod x;` no
mesmo directório) puramente pelo **nome**:

```rust
let is_mod_or_entry = file_name == "lib.rs"
    || file_name == "main.rs"
    || file_name == "mod.rs";
```

Cargo, porém, aplica convenção entry-style a **qualquer ficheiro
declarado como entry point de target** (`[[lib]]`, `[[bin]]`,
`[[test]]`, `[[bench]]`, `[[example]]`), independentemente do
nome. Um `tests.rs` que é entry de `[[test]]` resolve `mod args;`
para o mesmo directório, igual a `lib.rs`.

A informação de quais paths são entry points de target já está
no `WorkspaceMember` (dentro de cada variante de `EntryKind`,
após a ADR-0007). O resolvedor apenas não a usa.

---

## Alternativas consideradas

### Alternativa A — Hardcoder mais nomes

Estender a lista para incluir nomes comuns: `tests.rs`,
`benches.rs`, `examples.rs`, etc.

**Prós:**
- Mudança mínima de uma linha.
- Resolve o caso imediato do Typst.

**Contras:**
- Frágil: não cobre nomes arbitrários permitidos por Cargo
  (`path = "foo.rs"` em qualquer `[[test]]`).
- Ignora a fonte da verdade (o `Cargo.toml`).
- Cada novo workspace exótico pode requerer nova entrada.

### Alternativa B — Propagar a flag a partir do `WorkspaceMember`

O `traverse_crate` já recebe o `WorkspaceMember` e já sabe qual
é o path de entrada (via `primary_entry()`). Marcar esse ficheiro
como "entry-style" e propagar a noção descendentemente segundo
regras consistentes:

- Ficheiro raiz da travessia: entry-style por construção.
- Ficheiro descendente: entry-style se for `mod.rs` (convenção
  universal de Cargo); ficheiro normal caso contrário.

A heurística por nome (`lib.rs`/`main.rs`) deixa de existir; o
nome do ficheiro raiz torna-se irrelevante.

**Prós:**
- Usa a fonte da verdade (o `Cargo.toml`, via `cargo_metadata`).
- Cobre todos os tipos de target presentes e futuros sem
  modificação do resolvedor.
- Mantém `mod.rs` por nome — convenção sintáctica que existe
  *dentro* de qualquer crate, não vinculada a target.
- Custo localizado: assinatura interna de `resolve_module_path`
  e `traverse_file`.

**Contras:**
- Pequena mudança na assinatura de funções privadas de L₃.
- Requer actualização dos testes existentes do `module_traverser`
  para garantir comportamento idêntico em entry points clássicos
  (`lib.rs`/`main.rs`).

### Alternativa C — Re-detectar via `cargo_metadata` no resolvedor

Cada chamada de `resolve_module_path` consulta novamente
`cargo_metadata` para saber se o `parent_file` é entry de target.

**Prós:**
- Nenhuma mudança em estruturas de dados.

**Contras:**
- Acoplamento errado: L₃ resolvedor passa a depender directamente
  do reader; perde a separação de fases.
- Custo de I/O por chamada.
- Redundante: a informação já flui pelo `WorkspaceMember`.

---

## Decisão

**Alternativa B: propagar a flag a partir do `WorkspaceMember`.**

### Mecânica

1. `traverse_crate` permanece a entrada pública e passa a marcar
   a 1ª chamada de `traverse_file` com `is_entry = true`.

2. `traverse_file` recebe e propaga `is_entry: bool` para
   `traverse_items` e daí para `resolve_module_path`.

3. `resolve_module_path` deixa de ler `file_name` para decidir
   entry-style. Em vez disso, usa o `is_entry` recebido.

4. Para módulos filhos, o `is_entry` do próximo `traverse_file`
   é determinado pelo nome do **ficheiro resolvido**: `mod.rs`
   → `true`; qualquer outro → `false`.

### Pseudocódigo essencial

```rust
fn traverse_crate(member: &WorkspaceMember) -> Result<ModuleTree, _> {
    let entry = member.entry_kind.primary_entry()...;
    let mut tree = ModuleTree::new(member.name, entry.clone());
    traverse_file(&mut tree, root, &entry, /* is_entry = */ true, ...)?;
    Ok(tree)
}

fn traverse_file(..., file_path: &Path, is_entry: bool, ...) {
    // parsear, percorrer items
    traverse_items(..., file_path, is_entry, ...)?;
}

fn traverse_items(..., parent_is_entry: bool, ...) {
    for item in items {
        if let Item::Mod(m) = item {
            // ... módulos inline igual ...
            // módulos externos:
            let (resolved, ...) = resolve_module_path(
                file_path, parent_is_entry, &m.ident, &m.attrs)?;
            let child_is_entry = is_mod_rs(&resolved); // só mod.rs por nome
            traverse_file(..., &resolved, child_is_entry, ...)?;
        }
    }
}

fn resolve_module_path(parent_file, parent_is_entry: bool, ...) {
    // ...
    let search_dir = if parent_is_entry {
        parent_dir.to_path_buf()
    } else {
        parent_dir.join(stem)
    };
    // resto igual
}
```

A heurística antiga (`file_name == "lib.rs" || ... || "mod.rs"`)
é substituída por: `parent_is_entry` (passado) **ou** `mod.rs`
(detectado por nome do ficheiro filho ao recursar).

---

## Justificação

1. **Coerência com Cargo**: a fonte da verdade para "isto é um
   entry point" é o `Cargo.toml`. Já obtemos essa informação no
   L₃ reader. Usá-la no L₃ resolvedor fecha o ciclo correctamente.

2. **Robustez**: cobre `tests.rs`, `fuzz.rs`, `wrapper.rs`,
   `path = "foo/bar.rs"` em `[[test]]` etc., sem hardcoding.

3. **Custo mínimo**: três funções privadas de um único módulo
   ganham um parâmetro `bool`. API pública (`traverse_crate`)
   inalterada.

4. **Backwards-compatible para crates clássicos**: `lib.rs` e
   `main.rs` continuam a funcionar — agora porque vêm via
   `primary_entry()` como ficheiro raiz da travessia, não porque
   o nome bate na lista hardcoded.

5. **`mod.rs` continua especial por nome**: é correcto. `mod.rs`
   é uma convenção sintáctica de Cargo *dentro* de um crate,
   aplicável a qualquer sub-pasta, independente de target. Não
   está atrelada a nenhum entry point declarado em `Cargo.toml`.

---

## Consequências

### ✅ Positivas

- O smoke test contra Typst deve passar com 21/21 crates
  traversados (vs 20/21 actual).
- Workspaces com `tests/`, `fuzz/`, `examples/` ganham suporte
  automático sem casos especiais.
- A regra do resolvedor torna-se mais clara: "o ficheiro raiz
  da travessia é entry; descendentes só são entry se forem
  `mod.rs`".

### ❌ Negativas

- Mudança em código `IMPLEMENTADO` (`module_traverser.rs`),
  ainda que localizada.
- Testes existentes de `module_traverser_tests` precisam
  re-validar o comportamento. Como nenhuma fixture tem entry
  point com nome não-canónico, os testes actuais não cobririam
  a mudança — convém adicionar pelo menos um caso novo.

### ⚙️ Acções decorrentes

1. Criar prompt `module_traverser-revisao.md` com o desenho
   detalhado (próximo passo após esta ADR).
2. Implementar a mudança em `03_infra/src/module_traverser.rs`.
3. Adicionar fixture/teste cobrindo entry-style com nome
   não-canónico (ex: `tests-only-entry-custom-name/`).
4. Re-executar o smoke test contra Typst. Esperado: 21/21 OK.
5. Actualizar status de `module_traverser.md` para
   `IMPLEMENTADO (revisado)` com nota desta ADR.

---

## Critérios de reavaliação

Esta ADR deve ser reaberta se:

1. Surgir um caso onde Cargo trata um ficheiro como entry-style
   sem ele ser entry point de target (ex: futura convenção de
   workspace que não usa `[[test]]`).
2. A propagação da flag tornar-se incómoda para evoluções
   futuras (ex: se quisermos paralelizar a travessia e a flag
   atrapalhar o threading).
3. Aparecer necessidade de tratar entry points de **múltiplos**
   targets do mesmo crate simultaneamente (hoje só usamos
   `primary_entry()` — 1º elemento). Nesse caso, esta ADR
   continua válida mas a estratégia de quais paths visitar tem
   que evoluir em paralelo.

---

## Referências

- ADR-0007 — Extensão do `EntryKind` (motivou esta sequência).
- Cargo Book — Cargo Targets:
  https://doc.rust-lang.org/cargo/reference/cargo-targets.html
- Cargo Book — Path Attribute on modules:
  https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute
- Smoke test contra Typst real (incidente que motivou esta ADR).
- `00_nucleo/prompts/smoke-test-diagnostico.md` — revisão que
  tornou a falha investigável.
