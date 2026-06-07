# Laudo de Execução — Prompt 0061 (verificar a semântica de `[excluded_files]`)

**Camada**: verificação (fonte do `tekt-linter` + prova empírica)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0061-verificar_excluded_files.md`
**Estado**: `EXECUTADO` — medição (sem mudar repo). **Veredito: `[excluded_files]`
suspende TODOS os checks (V1–V14), inclusive pureza (V4/V13)** — fonte e prova
concordam. `vizinhanca.rs` é **puro**.

---

## A resposta em uma sentença

`[excluded_files]` não é "exclusão de linhagem" — é **exclusão total**: o arquivo é
filtrado no *walk*, antes de ser parseado, então **nenhuma** checagem (V1–V14) o vê;
um arquivo de lógica L1 excluído (como o `vizinhanca.rs`) fica **sem guarda de
pureza**, mesmo sendo puro hoje.

---

## 1. A fonte do linter — ponto de aplicação: **walk-level**

`03_infra/walker.rs::files()` filtra os arquivos **antes** de qualquer parse:

```rust
.filter_entry(move |e| !is_ignored(e.path(), &root, &excluded_dirs, &excluded_files))
```

`is_ignored` consulta `excluded_files` por **path relativo exato** (linhas 47–51) →
`true` → o arquivo **não entra** no iterador. E em `04_wiring/main.rs:161`:

```rust
let (source_files, source_errors) = collect_walker_results(walker.files());
… run_pipeline(&source_files, …)   // TODAS as checagens rodam só sobre source_files
```

**Logo: arquivo em `[excluded_files]` nunca vira `ParsedFile` → nenhuma das V1–V14
o examina.** Não é por-check; é exclusão no walk, total. (Contraste: `--checks`
desliga checks; `[excluded_files]` desliga **arquivos**, para todos os checks.)

---

## 2. A prova empírica — concorda com a fonte

Numa **cópia descartável** (`git worktree`, removida ao fim), injetei a **mesma**
violação em dois arquivos:

- `01_core/investiga/src/vizinhanca.rs` — **está** em `[excluded_files]`.
- `01_core/investiga/src/lib.rs` — **não** está (é unidade nucleada).

Injeção: `static mut CONTADOR…` (gatilho **V13**) + `std::fs::read_to_string(…)`
(gatilho **V4**). Resultado (`crystalline-lint --checks v4,v13`):

| Arquivo | Excluído? | V13 | V4 |
|---|---|---|---|
| `investiga/src/lib.rs` | não | **dispara** (l.284) | **dispara** (l.286) |
| `investiga/src/vizinhanca.rs` | **sim** | **silêncio** | **silêncio** |

A injeção **é detectável** (provado pelo `lib.rs` disparar); o silêncio no
`vizinhanca.rs` é, portanto, **da exclusão**, não da pureza. **Fonte e prova
concordam: `[excluded_files]` suspende V4/V13** (e tudo o mais). Repo real
intocado — a injeção viveu só no worktree.

---

## 3. O `vizinhanca.rs` — veredito: **puro**

Leitura de `01_core/investiga/src/vizinhanca.rs` (Estratégia 1 da investigação —
compara conjuntos de arestas):

- Imports: só `std::collections::HashSet` + `lente_core` (Aresta/Relation/Veredito/
  Evidencia) + `crate::ArestasNo`.
- **Sem** `std::fs`/`std::io`/`std::net` (zero I/O).
- **Sem** `static mut` / estado global mutável (só locais e o `HashSet`).
- Lógica pura: interseção/diferença de conjuntos de `ChaveAresta` → veredito.

**É puro hoje.** O risco não é o presente — é o **futuro**: por estar excluído, uma
regressão de pureza nele (alguém adiciona I/O/estado depois) **passaria batida**.

---

## O que isto destrava (a decisão, informada — NÃO tomada aqui)

A regra do 0060 ("interno sem interface `pub` → `[excluded_files]`") **perde a
guarda de pureza** dos internos. Nuance importante medida de passagem: **V4 e V13
são checks de L1** ("…em L1" na mensagem). Consequência para escalar:

- **Internos do L1** (como `vizinhanca.rs`): excluir **perde** a guarda de pureza —
  é onde a decisão pesa. Opções (para depois): (a) cabeçalho leve + prompt do crate
  (mantém o arquivo **checado**, snapshot vazio ok); (b) ajustar o linter para
  excluir só linhagem; (c) aceitar e documentar (puro hoje, coberto por revisão/teste).
- **Internos do L3+** (`lente_infra`, muitos arquivos): V4/V13 **nem se aplicam**
  (L3 faz I/O legítimo) — excluir **não perde guarda**. Então, para o L3, a regra do
  0060 é segura como está.

A decisão dos internos do L1 fica para um prompt à parte, agora **informada** por:
`[excluded_files]` é total, e o `vizinhanca.rs` é puro.

---

## Verificação

| Item | Resultado |
|------|-----------|
| Fonte (`walker.rs`/`main.rs`) | `[excluded_files]` aplicado no walk → suspende **todas** V1–V14 |
| Prova empírica (worktree descartável) | V4/V13 disparam no não-excluído, silêncio no excluído — **concorda** |
| `vizinhanca.rs` | **puro** (sem I/O, sem estado mutável global) |
| Repo real | **intocado** (injeção em worktree, removido); `git status` limpo das fontes |
| Config/código | **não alterados** (só medição) |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Verificação (sem mudança) do escopo de `[excluded_files]`. **Fonte**: aplicado em `walker.rs::files()` via `.filter_entry`/`is_ignored` (path relativo exato) **antes** do parse; `main.rs` roda todas as checagens só sobre `source_files`, então arquivo excluído **nunca** é examinado → suspende **TODAS** as V1–V14 (não só linhagem). **Prova empírica** (em `git worktree` descartável, removido): injetada a mesma violação V13 (`static mut`) + V4 (`std::fs::read_to_string`) no `vizinhanca.rs` (excluído) e no `investiga/lib.rs` (não-excluído) — disparam no não-excluído (l.284/286), silêncio no excluído → **concorda** com a fonte. **`vizinhanca.rs`**: lido, **puro** (só `HashSet`+`lente_core`; sem `std::fs/io/net`; sem `static mut`). Implicação (decisão adiada): excluir interno do **L1** perde a guarda de pureza (V4/V13 são checks L1); para o **L3+** não perde (I/O legítimo, V4 não se aplica). Repo real intocado. | (verificação — nenhum arquivo do repo alterado; `00_nucleo/lessons/0061-verificar_excluded_files.md`) |
