# Laudo de Execução — Prompt 0012 (Descritor Semântico no lente_core)

**Camada**: L5 (laudo)
**Data**: 2026-05-28
**Prompt executado**: `00_nucleo/prompt/0012-lente_core_descritor.md`
**Decisões de origem**: descritor do fork 0.27.0; decisões do autor (formas
simples; remodelar Kind separando tipo base de modificadores).
**Estado**: `EXECUTADO` — `lente_core` remodelado (30 testes verdes), cascata
a jusante ajustada para compilar, workspace 70 verdes + 2 ignored, pureza
preservada.

---

## O que o prompt pediu

Mudança **só de modelagem** no `lente_core`:

1. **Campos novos no `No`** (formas simples): `modificadores`, `crate_name`,
   `trait_`, `trait_ref`, `cfg`, `macro_kind`, `is_non_exhaustive`.
2. **Remodelar o `Kind`**: separar tipo base dos modificadores
   (`const`/`async`/`unsafe`), que viravam variantes achatadas
   (`ConstFn`, etc.). Novo tipo `Modificadores` (três bool).
3. `TryFrom<&str>` do `Kind` passa a despir modificadores e parsear só o
   tipo base.

---

## O que foi alterado

### No `lente_core` (o foco do prompt)

| Item | Mudança |
|------|---------|
| `enum Kind` | De 17 variantes para **13 tipos base**. Removidas: `ConstFn`, `AsyncFn`, `UnsafeFn`, `UnsafeTrait`. |
| `struct Modificadores` (NOVO) | `is_const`, `is_async`, `is_unsafe` (bool), `#[derive(Default)]`. |
| `struct No` | +7 campos: `modificadores`, `crate_name`, `trait_`, `trait_ref`, `cfg`, `macro_kind`, `is_non_exhaustive`. |
| `impl TryFrom<&str> for Kind` | Parseia pelo **último token** da string. |
| Testes do `lente_core` | Ajustados + novos (ver abaixo). 30 verdes. |

### A jusante (mínimo mecânico para compilar — NÃO a lógica real)

O prompt declara "só toca o lente_core", mas a mudança no struct público `No`
quebra a compilação de todo consumidor. O próprio prompt antecipa isso
("não-regressão coordenada... afeta os testes de vários crates"). Ajustes
mecânicos feitos (preencher campos novos com defaults/None):

| Crate | Ponto ajustado | Natureza |
|-------|----------------|----------|
| `lente_core` (raio.rs) | helper de teste `no()` | só teste |
| `lente_investiga` | helper de teste `no()` | só teste |
| `lente_resolve` | helper de teste `no()` | só teste |
| `lente_infra` (traducao.rs) | **construção de `No`** na tradução | lógica (defaults), ver nota |
| `remedicao` (lab/Arena) | `no_para_lente` | experimento |

**Nota crucial sobre o `lente_infra`**: a construção de `No` na tradução
preenche os campos novos com **defaults** (`Modificadores::default()`,
`crate_name` do grafo, demais `None`/`false`). Isso **não é** a
desserialização real — é só o mínimo para compilar. Desserializar os campos
novos do JSON 0.27 e construir `Modificadores` a partir dos booleanos é o
**próximo prompt da cascata** (0013). Sinalizado em comentário no código e
aqui.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test -p lente_core` | **30/30** (26 anteriores + 4 novos) |
| `cargo test` (workspace) | **70 verdes + 2 ignored** (core 30, infra 14+2, investiga 17, resolve 9) |
| `cargo tree -p lente_core` | só o crate — **pureza preservada** |
| `remedicao` (Arena) recompila | sim (após ajuste do helper) |

---

## Decisões tácitas

### D1 — `crate_name` ENTRA no `No` (revertendo a D1 do laudo 0006)

O laudo 0006 (D1) recusou `crate_name` em `No` por considerá-lo redundante
com `Grafo.crate_name`. O prompt 0012 explicitamente o inclui — porque o
fork 0.27.0 emite o crate **por nó** (cada nó pode ser de um crate diferente:
o crate-alvo ou stdlib). Isso torna `crate_name` por-nó informação genuína,
não redundante: substitui a "marca de stdlib computada pelo prefixo do path"
(ADR-0002 D3) por um campo direto. Segui o prompt.

### D2 — `TryFrom<&str>` do `Kind` parseia pelo último token

A string vem como `"const async unsafe fn"`. Pegar o **último token**
(`s.rsplit(' ').next()`) resolve elegantemente a ambiguidade central:

- `"const"` (sozinho) → último token `const` → `Kind::Const` (o item constante).
- `"const fn"` → último token `fn` → `Kind::Fn` (modificador const é descartado).
- `"unsafe trait"` → último token `trait` → `Kind::Trait`.

Sem o último-token, distinguir `const`-item de `const`-modificador exigiria
lógica especial. Testado em `kind_const_sozinho_e_o_tipo_const_nao_modificador`.

Os **modificadores não saem do `TryFrom`** — eles vêm dos booleanos do fork
(`is_const` etc.), responsabilidade do `lente_infra` (próximo prompt). O
`Kind` só dá o tipo base.

### D3 — Erro do `Kind` reporta a string inteira

`Kind::try_from("frobnicate")` → `ValorDesconhecido { texto: "frobnicate" }`.
Reporto `s` inteiro (não só o último token) para diagnóstico mais útil. O
teste antigo usava `"extern fn"` esperando erro — mas agora `"extern fn"` →
último token `fn` → `Kind::Fn` (despe `extern` como se fosse modificador).
Troquei o teste para `"frobnicate"` (último token genuinamente desconhecido).

Consequência colateral: qualquer string cujo último token seja um tipo base
conhecido é aceita, mesmo com prefixos não-modificadores (`extern fn`,
`pub fn`...). Tolerável — o fork não emite esses, e o objetivo é o tipo base.

### D4 — Campos `trait_`/`cfg`/etc. como formas simples

`Option<String>` para `trait_`, `trait_ref`, `cfg`, `macro_kind`; `bool`
para `is_non_exhaustive`. Conforme decisão do autor no prompt: estruturar
(separar nome de args do trait, tipo recursivo do cfg) fica para quando
houver uso concreto. Hoje só carrega; o único com uso iminente é
`trait_`/`trait_ref` (resolve a D4 do laudo 0010 no `lente_investiga`,
prompt posterior).

### D5 — Helper `no_de` nos testes do `lente_core`

Em vez de repetir os 12 campos em cada literal de teste, criei
`no_de(id, path, name, kind)` que preenche o descritor com defaults. Os
testes `grafo_construido_*` e `grafo_minimo_*` passaram a usá-lo. Reduz o
churn e o ruído.

---

## O que a cascata a jusante vai precisar (sinalização para o próximo prompt)

**Prompt 0013 (lente_infra)** — o mais imediato:

1. **DTO**: adicionar ao `NoDTO` os campos do JSON 0.27: `crate` (já tem como
   `crate_name`? não — o `crate` do topo é do grafo; o fork agora emite crate
   por nó, campo a confirmar no JSON real), `trait`, `trait_ref`,
   `is_const`/`is_async`/`is_unsafe`, `cfg`, `macro_kind`, `is_non_exhaustive`.
2. **Construir `Modificadores`** a partir dos três booleanos do DTO.
3. **Preencher o `No`** com os campos reais (não os defaults que pus agora).
4. **Atenção à fonte única dos modificadores**: usar os **booleanos**, não a
   string `kind` (que ainda traz os modificadores embutidos por
   retrocompatibilidade). O `Kind::try_from` já ignora os modificadores da
   string; os booleanos do JSON são a verdade.
5. **Pré-requisito**: instalar o fork 0.27.0 (`cargo install --git ...
   --force`). Os E2E ignored do `lente_infra` ainda usam o fork instalado;
   com 0.27.0 o JSON tem os campos novos (serde ignora desconhecidos hoje,
   então não quebra antes do 0013, mas o 0013 passa a exigi-los).

**Prompts posteriores**:
- `lente_investiga`: usar `trait_`/`trait_ref` por nó para nomear por trait
  com id correto (resolve a D4 do laudo 0010 — a investigação 0011 concluiu
  que essa info tem de vir do fork, e agora vem).
- `lente_resolve`: nomear por trait com a precisão que o trait-por-nó permite.

---

## Não-regressão e pureza

- Os 4 crates do workspace continuam verdes (70 testes + 2 ignored).
- Nenhum teste removido para fazer passar; os testes do `Kind` que assumiam
  variantes achatadas (`ConstFn`) foram **reescritos** para o novo modelo
  (tipo base + despir modificadores), não deletados.
- `cargo tree -p lente_core` mostra só o crate — os campos novos são tipos
  puros (String, Option, bool, struct de bools), sem dependência externa.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | Descritor semântico no lente_core: 7 campos novos no `No` (formas simples), `Kind` remodelado para 13 tipos base, `Modificadores` novo tipo, `TryFrom` por último token. Cascata a jusante ajustada ao mínimo para compilar (defaults; desserialização real é o prompt 0013). 70 testes verdes + 2 ignored; pureza preservada. | `01_core/src/entities/grafo.rs`, `01_core/src/domain/raio.rs`, `03_infra/src/traducao.rs`, `05_investiga/src/lib.rs`, `06_resolve/src/lib.rs`, `lab/.../remedicao/src/main.rs` |
