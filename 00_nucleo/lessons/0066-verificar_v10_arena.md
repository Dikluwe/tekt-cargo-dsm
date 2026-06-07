# Laudo de Execução — Prompt 0066 (verificar o V10 após a exclusão da Arena)

**Camada**: verificação (fonte do `tekt-linter` + prova empírica)
**Data**: 2026-06-07
**Prompt executado**: `00_nucleo/prompt/0066-verificar_v10_arena.md`
**Estado**: `EXECUTADO` — medição (sem mudar repo). **Veredito: o `V10 = 0` do 0065 é
"V10 sem alvo"** — tirar a Arena do `[layers]` **desligou** o V10. Fonte e prova
concordam. A config mínima de recuperação existe (lab em `[layers]` **e**
`[excluded]`).

---

## A resposta em uma sentença

O V10 acha a quarentena pelo **layer do membro no registry** (derivado do
`[layers]`); ao tirar a Arena do `[layers]` (0065), nenhum import resolve para
`Layer::Lab` e o V10 fica **sem alvo** — mas, no projeto real, a Arena é um
**workspace separado** (nunca um membro do registry), então o V10 já era vazio; a
proteção de fato é a **fronteira de workspace**.

---

## 1. A fonte do V10

`01_core/rules/quarantine_leak.rs::check`:

```rust
if matches!(file.layer(), Layer::Lab | Layer::L0 | Layer::Unknown) { return [] }
file.imports().filter(|i| i.target_layer == Layer::Lab).map(|i| Violation{V10,...})
```

- **Identifica a quarentena** pelo `import.target_layer == Layer::Lab`.
- **`target_layer`** é setado pelo `classify_import` (rs_parser): para import
  cross-crate, via o **`CrateRegistry`** — `member_layer(nome)`, e o layer do membro
  vem de `resolve_file_layer(dir)`, que usa o **`[layers]`** (`crate_registry.rs:24`).
- **Interação com `[excluded]`**: o registry é construído de `[workspace].members` +
  `[layers]` (`from_root`), **não** do walk. Então `[excluded]` (poda do walk) **não**
  afeta o registry. Mas tirar a Arena do **`[layers]`** faz `resolve_file_layer(lab/…)`
  → `Unknown` → nenhum import resolve para `Lab` → **V10 sem alvo**.

---

## 2. A prova empírica (worktrees descartáveis, removidos)

Mesma injeção (produção `lente_filtro`, L1, importando o crate da Arena
`proto_impacto_diff`), sob três configs:

| Config | `[layers]` lab? | `[excluded]` lab? | V10 | V1 nos arquivos lab |
|---|---|---|---|---|
| **(a)** atual (0065) | não | sim | **silêncio** | 0 |
| **(b)** controle | **sim** | não | **DISPARA** (fatal) | (n/a) |
| **(c)** recuperação | **sim** | **sim** | **DISPARA** (fatal) | **0** |

A injeção é um **leak válido** (dispara em (b)/(c)); o silêncio em (a) é da
**ausência de alvo**, não da ausência de vazamento. **Fonte e prova concordam.**
Repo real **intocado** (tudo em worktree, removido).

> Nota metodológica: para o V10 ter alvo, precisei **adicionar a Arena aos
> `[workspace].members`** do projeto (o registry só conhece membros). Sem isso, o
> V10 não dispara nem em (b) — porque o registry não enxerga a Arena.

---

## 3. A ressalva crucial — no projeto real, a Arena é workspace separado

O `lab/` tem **`[workspace]` próprio** (manifesto Tekt; confirmado). Logo os crates
da Arena **não são membros** do workspace principal → **nunca entram no
`CrateRegistry`** → o V10 **nunca teve alvo** para eles, **mesmo antes do 0065**
(quando a Arena estava no `[layers]`). Nas provas (b)/(c), o V10 só disparou porque
**eu adicionei** a Arena aos membros principais.

Então a proteção real contra "produção importa Arena" **não é o V10** — é a
**fronteira de workspace**: para importar um crate da Arena, seria preciso uma
`path`-dep cruzando workspaces **e** adicioná-lo aos membros — um ato deliberado e
visível no `Cargo.toml`, não um `use` silencioso.

---

## O que isto destrava (decisão adiada, informada)

Dois caminhos, ambos válidos, **a decidir num prompt à parte**:

1. **Aceitar como está** — o V10 fica vazio para a Arena (sem alvo), mas a
   fronteira de workspace é a guarda real. O `V10 = 0` do 0065 é honesto **se** se
   entende que a defesa é estrutural, não pelo linter. Simples, zero config.
2. **Recuperar o V10 (defesa em profundidade)** — config mínima **provada (c)**:
   manter a Arena no **`[layers]`** (para o V10 ter o mapeamento de layer) **e** no
   **`[excluded]`** (para podar do walk → V1 = 0). Pega o caso futuro de alguém
   adicionar um crate da Arena aos membros principais. Custo: a Arena fica em dois
   blocos do config (documentado). **Recomendação fraca**: (2) — barato e fecha a
   defesa em profundidade; o 0065 tirou do `[layers]` a mais (bastava o `[excluded]`).

Conforme o prompt, **não decidi nem mudei o repo** — a aplicação fica para depois.

---

## Verificação

| Item | Resultado |
|------|-----------|
| Fonte (`quarantine_leak.rs`/`crate_registry.rs`/`rs_parser.rs`) | V10 acha lab pelo layer do membro (registry, via `[layers]`); `[excluded]` não afeta o registry |
| Prova (a)/(b)/(c) | (a) silêncio · (b) dispara · (c) dispara + V1=0 — **concorda** com a fonte |
| Ressalva | Arena é workspace separado → nunca membro do registry → V10 sem alvo mesmo pré-0065 |
| Config mínima de recuperação | **lab em `[layers]` E `[excluded]`** (provada em (c)) |
| Repo real | **intocado** (worktrees removidos; `git status` limpo das fontes) |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Verificação (sem mudança) do **V10 (QuarantineLeak)** após o 0065 ter tirado a Arena do `[layers]`. **Fonte**: o V10 dispara quando um import de produção tem `target_layer == Layer::Lab`; esse layer vem do `CrateRegistry` (`member_layer`), cujo layer de membro = `resolve_file_layer(dir)` via **`[layers]`**; o registry é de `[workspace].members`+`[layers]`, não do walk (logo `[excluded]` não o afeta, mas a remoção do `[layers]` sim). **Prova** (worktrees descartáveis, removidos; mesma injeção produção→Arena): **(a)** config atual (lab em `[excluded]`, fora do `[layers]`) → **V10 silêncio**; **(b)** controle (lab no `[layers]`) → **V10 dispara** (fatal); **(c)** lab em `[layers]` **E** `[excluded]` → **V10 dispara E V1=0**. A injeção é leak válido (dispara em b/c); o silêncio em (a) é **falta de alvo**. **Veredito**: o `V10=0` do 0065 é "V10 sem alvo" — a remoção do `[layers]` desligou o V10. **Ressalva**: no projeto real a Arena é **workspace separado** → seus crates **nunca** são membros do registry → o V10 já era vazio mesmo pré-0065 (nas provas só disparou porque adicionei a Arena aos membros); a proteção real é a **fronteira de workspace**. **Config mínima de recuperação** (defesa em profundidade): lab em `[layers]` + `[excluded]` (provada em (c): V10 ativo, V1=0). Decisão adiada (não mexi no repo). | (verificação — nenhum arquivo do repo alterado; `00_nucleo/lessons/0066-verificar_v10_arena.md`) |
