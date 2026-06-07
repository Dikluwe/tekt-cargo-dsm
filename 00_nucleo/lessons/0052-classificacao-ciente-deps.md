# Laudo de Execução — Prompt 0052 (classificação de import ciente de dependências)

**Camada**: L5 (laudo)
**Data**: 2026-06-06
**Prompt executado**: `00_nucleo/0052-linter_classificacao_ciente_deps.md` (no clone do `tekt-linter`)
**Depende de**: laudo 0051 (premissa confirmada — V3 cego a cross-crate).
**Estado**: `EXECUTADO` — conserto a montante materializado, regras L1 intactas,
473 testes verdes (4 falhas `blanket_impl` **pré-existentes**, alheias), linter
passa em si mesmo (0 violações), cascata provada em fixture multi-crate.

---

## Veredito (em uma linha)

O buraco do V3 foi fechado **a montante**, no `resolve_layer`/`resolve_subdir`
(L3), com um registro membro→camada ciente de dependências. **Nenhuma regra L1
(V3, V14, V9) mudou.** O V3 volta a enxergar direção entre crates, o falso
positivo do `Kind` some, e o externo real continua pego — confirmado por TDD (6
casos + V9) e por fixture end-to-end.

---

## O conserto (onde ficou na arquitetura)

### 1. `CrateRegistry` (L3, novo) — `03_infra/crate_registry.rs`

Lê o workspace do projeto-alvo (I/O em `Cargo.toml`):

- **Membros**: nome do pacote (normalizado `-`→`_`), diretório, camada (via
  `walker::resolve_file_layer` sobre o diretório — **a mesma lógica `[layers]`** que
  dá camada aos ficheiros, garantindo consistência), e deps declaradas
  (`[dependencies]` **+** `[dev-dependencies]`).
- Enumeração: `[workspace].members` (com glob `crates/*`) ou `[package]` único.
- `member_layer(name)` — lookup first-party. `owner_of(file)` — membro dono (prefixo
  de diretório mais longo vence) → **contexto per-crate** de deps.
- `empty()`/`Default` — registro vazio ⇒ **classificação idêntica ao legado**.

### 2. Classificação ciente — `resolve_layer`→`classify_import` (L3)

Ordem (per-crate; `owner` = crate dono do ficheiro do import):

1. `crate::`/`super::` → `module_layer(seg[1])` (inalterado).
2. `std`/`core`/`alloc` → `Unknown` (V14 isenta; V4 cuida de I/O).
3. 1º seg == **nome do próprio crate** → intra-crate (cobre `crystalline_lint::…`,
   que no linter só ocorre em L4).
4. 1º seg ∈ **outro membro** → camada do membro → **V3 enxerga a direção**.
5. 1º seg ∈ **deps externas declaradas** → `Unknown` → V14 aplica no L1.
6. sem `owner` → `Unknown` (legado preservado).
7. owner presente e seg não é membro/dep/stdlib → **item local**: **não emite
   `Import`** → o falso positivo do `Kind` some **sem tocar o V14**.

`resolve_subdir` (V9): para membro first-party resolvido a L1, subdir = `seg[1]`,
**só** com ≥3 segmentos (`crate::sub::Item`) — um import de 2 segmentos usa a API
da raiz, não uma porta. `crate::`/`super::` preservado bit-a-bit.

### 3. Injeção (L4) — `04_wiring/main.rs`

`CrateRegistry::from_root(&cli.path, &config)` construído uma vez e passado aos três
`RustParser::new`. Zero lógica de negócio em L4.

---

## TDD — os 6 casos (+ V9), com prova de que mordem

Os testes foram escritos a partir dos Critérios de Verificação **antes** da
implementação. Para provar que não são sombra da implementação, injetei
temporariamente o classificador **legado** e confirmei que os casos cross-crate
**falham** sob ele:

| Caso | Esperado | Sob legado (bite) | Final |
|---|---|---|---|
| 1 — L3→L4 `use membro` | target L4 (V3 dispara) | **FALHOU** | ✓ |
| 2 — L1→L1 `use membro` | target L1 (V3/V14 calados) | **FALHOU** | ✓ |
| 3 — `use serde` declarado | Unknown (V14 dispara) | passou (já certo) | ✓ |
| 4 — `use EnumLocal::*` | não emitir (V14 calado) | **FALHOU** | ✓ |
| 5 — `use std::…` | Unknown isento | passou (já certo) | ✓ |
| 6 — `use crate::shell` | L2 (V3 dispara — controle 0051) | passou (já certo) | ✓ |
| V9 — `use membro_L1::internal::X` | subdir "internal" | (via `resolve_subdir`) | ✓ |
| regressão — registro vazio | `EnumLocal::*` → Unknown (não Skip) | — | ✓ |

`cargo test --lib`: **473 passed; 4 failed**. As 4 são `blanket_impl_*`,
**idênticas no master commitado** (verificado por `git stash`) — não introduzidas
por este conserto.

---

## Auto-validação (o critério primário do linter)

`crystalline-lint .` no próprio linter: **✓ No violations found** (exit 0). O linter
é crate único multi-camada; o caminho cross-crate fica dormente nele (todo import
é `crate::`/`super::`, `std`, ou dep externa declarada → classificação idêntica à
de antes). `--fix-hashes` aplicado após nuclear o prompt
`00_nucleo/prompts/crate-registry.md` (paridade dupla atualizada).

---

## Cascata — fixture multi-crate descartável (a capacidade provada)

Workspace cargo de teste (membros `corec`=L1, `shellc`=L2, `wiringc`=L4), saída
literal do linter consertado:

```
error: Inversão de gravidade: L1 não pode importar de L4 ('wiringc::Thing') [V3]
error: Inversão de gravidade: L1 não pode importar de L2 ('crate::shell::X') [V3]
error: Dependência externa não autorizada em L1: 'serde' não está em [l1_allowed_external] [V14]
```

`use LocalEnum::*` (o `Kind`) **não** gerou V14. Deltas vs 0051:

| Caso | 0051 (antes) | Consertado |
|---|---|---|
| cross-crate L1→L4 | V3 **silencioso** (o buraco) | **V3 dispara** |
| `Kind` (`use EnumLocal::*`) | V14 **falso positivo** | **V14 silencioso** |
| externo real (`serde`) | V14 dispara | V14 dispara (sem regressão) |
| controle (`crate::shell`) | V3 dispara | V3 dispara (sem regressão) |

---

## O que falta (passo à parte, no `tekt-cargo-dsm`)

A confirmação no projeto real (decisão à parte, modifica outro repo):

1. Instalar o linter consertado.
2. Remover os seis `lente_*` do `[l1_allowed_external]` (a whitelist do 0050).
3. Esperado: **V14 = 0** (os `lente_*` resolvem para L1, o `Kind` some) e **V3 = 0
   mas agora significativo** (capaz de pegar cross-crate). Confirma que a whitelist
   do 0050 ficou **desnecessária** — fecha o débito que o 0050 abriu de propósito.

---

## Garantias do protocolo

- **Regras L1 (V3, V14, V9) NÃO mudaram** — todo o conserto é em L3 (resolução) + L4
  (injeção). `git diff` toca `03_infra/{crate_registry.rs(novo),rs_parser.rs,mod.rs}`
  e `04_wiring/main.rs`; nada em `01_core/rules/`.
- **Sem regressão**: registro vazio ⇒ legado bit-a-bit; `crate::`/`super::`/`std`
  preservados; linter passa em si mesmo.
- **Disciplina do linter**: prompt nucleado em `00_nucleo/prompts/`, linhagem nos
  ficheiros novos/mudados, TDD com bite-proof, suíte verde (exceto débito
  pré-existente alheio).
