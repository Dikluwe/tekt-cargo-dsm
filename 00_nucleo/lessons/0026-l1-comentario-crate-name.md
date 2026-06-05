# Laudo de Execução — Prompt 0026 (Comentário do `No.crate_name`)

**Camada**: L5 (laudo)
**Data**: 2026-06-02
**Prompt executado**: `00_nucleo/prompt/0026-l1-comentario-crate-name.md`
**Estado**: `EXECUTADO` — só doc-comment; zero mudança de comportamento;
127 verdes + 13 ignored (idêntico ao laudo 0025).

---

## O que mudou

Um campo, um comentário.

**Antes** (`01_core/src/entities/grafo.rs:199`):

```rust
/// Crate de origem do nó (distingue nós do crate-alvo de nós de stdlib).
pub crate_name: String,
```

**Depois**:

```rust
/// Crate-raiz do **grafo**, copiado para cada nó pelo L3.
///
/// O fork `cargo-modules` 0.27.0 **não** emite `crate` por nó (laudo
/// 0013 D1); o L3 (`lente_infra::traducao`) preenche este campo com o
/// `Grafo.crate_name` para todos os nós — inclusive os de sysroot
/// (`core::*`, `alloc::*`, `std::*`).
///
/// **Não distingue stdlib** do crate-alvo: o valor é igual para todos
/// os nós do mesmo grafo. A marca de stdlib é por **prefixo do path**
/// (ADR-0002 D3), aplicada no `lente_filtro` (laudo 0025).
pub crate_name: String,
```

Três fatos cobertos: (1) o que **é** (crate-raiz copiado), (2) por que é
assim (fork não emite por nó), (3) o que **não** faz (não distingue
stdlib) + onde a distinção real vive.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **127 verdes + 13 ignored** — bate com o laudo 0025 |
| Mudança de comportamento | nenhuma — só texto de doc-comment |
| Outros arquivos | nenhum |

---

## Por que este laudo é curto

É correção de **documentação enganosa**, não de código. A coisa correta
de fazer é declarar o que foi corrigido e qual erro futuro fica evitado;
não vale enrolar.

O erro que ficaria evitado: outro agente (ou eu, num turno futuro)
lendo só o tipo e o comentário, sem ler o laudo 0013 nem o comentário
do `traducao.rs:59-62`, repetiria a premissa do prompt 0025 v1 ("dá
para distinguir stdlib por `crate_name`"). O custo desse erro já foi
pago uma vez (Fase 1 suspensa do 0025). Comentário honesto fecha a
porta.

---

## O que NÃO mudou

- Tipo `No`, lógica, assinaturas — nada.
- O campo `crate_name` continua existindo, populado como antes.
- Pureza do L1.
- Pendência de "o campo deve existir mesmo?" — fora do escopo;
  declarada explicitamente no prompt.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-02 | Doc-comment do `No.crate_name` reescrito para descrever a realidade (crate-raiz copiado pelo L3; não distingue stdlib; ponteiro para ADR-0002 D3 / laudo 0013 D1). Zero mudança de comportamento; 127 verdes + 13 ignored estáveis. | `01_core/src/entities/grafo.rs`, `00_nucleo/lessons/0026-l1-comentario-crate-name.md` |
