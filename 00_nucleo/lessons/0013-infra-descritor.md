# Laudo de Execução — Prompt 0013 (Descritor Semântico no lente_infra)

**Camada**: L5 (laudo)
**Data**: 2026-05-28
**Prompt executado**: `00_nucleo/prompt/0013-lente_infra_descritor.md`
**Depende de**: laudo 0012 (lente_core com campos do descritor). Fork 0.27.0
(commit `a928eba8`).
**Estado**: `EXECUTADO` — desserialização real dos campos do descritor; 74
testes verdes + 2 ignored; E2E confirma `trait_` por nó; pureza preservada.

---

## O que o prompt pediu

Substituir os defaults placeholder (laudo 0012) pela desserialização **real**:
o `lente_infra` lê os campos do descritor do JSON do fork 0.27.0 e preenche o
`No` com valores reais (`trait_`, `trait_ref`, `cfg`, `macro_kind`,
`is_non_exhaustive`, `modificadores` dos booleanos, `crate_name`).

---

## Verificação prévia: o que o fork 0.27.0 REALMENTE emite

Antes de escrever o DTO, inspecionei o JSON real (o prompt pediu: "verificar
o JSON real antes de assumir"). Achados, com **divergências do prompt**:

| Campo | Prompt assumiu | Fork 0.27.0 (real) |
|-------|----------------|--------------------|
| `trait` | string, `rename` | ✓ string, campo `"trait"` |
| `trait_ref` | string | ✓ string |
| `is_const`/`is_async`/`is_unsafe` | bool, só quando true | ✓ confirmado |
| `is_non_exhaustive` | bool | ✓ |
| `cfg` | `Option<String>` (texto) | ✗ **estruturado**: `[{"Flag":"unix"}]` |
| `crate` por nó | existe (marca stdlib) | ✗ **NÃO existe** |
| `macro_kind` | `Option<String>` | não observado (macro_rules não gerou nó com ele) |

Teste de verificação: rodei o fork contra um crate-amostra com `const fn`,
`async fn`, `unsafe fn`, `#[non_exhaustive]`, `#[cfg(unix)]`, `macro_rules!`.
Resultado: `is_const`/`is_async`/`is_unsafe`/`is_non_exhaustive`/`cfg` apareceram;
`crate` e `macro_kind` não.

---

## O que foi alterado

### `03_infra/src/dto.rs`

`NoDTO` ganhou 8 campos novos, todos `#[serde(default)]`:

- `is_const`, `is_async`, `is_unsafe`, `is_non_exhaustive`: bool.
- `trait_` com `#[serde(rename = "trait")]` (palavra reservada).
- `trait_ref`: `Option<String>`.
- `cfg`: **`Option<serde_json::Value>`** (não String — o fork manda estrutura).
- `macro_kind`: `Option<String>`.

**Não** adicionei campo `crate` por nó — o fork não o emite.

### `03_infra/src/traducao.rs`

- `modificadores`: construído dos **booleanos** do DTO (`is_const` etc.) —
  NÃO da string `kind`. (A armadilha das duas fontes, fixada no laudo 0012.)
- `kind`: continua via `TryFrom<&str>`, que despe os modificadores e dá o
  tipo base (`"const fn"` → `Kind::Fn`).
- `cfg`: `no_dto.cfg.as_ref().map(|v| v.to_string())` — serializa a estrutura
  para texto (`[{"Flag":"unix"}]`), a forma que o `lente_core` modela.
- `trait_`, `trait_ref`, `macro_kind`, `is_non_exhaustive`: copiados direto.
- `crate_name`: continua vindo do **grafo (topo)** — o fork não dá por nó.

### Testes

- Helper `no_dto` atualizado (campos novos em default).
- 4 testes novos: `descritor_trait_e_propagado`, `modificadores_vem_dos_booleanos_nao_da_string`, `no_sem_descritor_fica_em_default`, `cfg_estruturado_do_fork_vira_texto`.
- E2E `e2e_extrai_grafo_de_lente_core_com_colisao_de_path` estendido: verifica que as duas cópias de `ErroRaio::fmt` têm `trait_` **Display** e **Debug** distintos.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test` (workspace) | **74 verdes + 2 ignored** (core 30, infra **18**+2, investiga 17, resolve 9) |
| `cargo test -p lente_infra -- --ignored` | **2/2** — E2E confirma `trait_` Display+Debug em `ErroRaio::fmt` (fork 0.27.0) |
| `cargo tree -p lente_core` | só o crate — pureza preservada |

---

## Descoberta central: a D4 está resolvida na raiz

O E2E confirma: extraindo `lente_core` com o fork 0.27.0, as duas cópias de
`lente_core::domain::raio::ErroRaio::fmt` trazem `trait_` **distinto e
associado ao id correto** (id 36 → "Display", id 47 → "Debug"), direto do
JSON. Sem adivinhação, sem ordem, sem leitura de fontes.

Isso é exatamente a "matéria-prima" que a investigação 0011 concluiu que
**precisava vir do fork** (a visibility cobria só 13% e era indireta). O fork
0.27.0 entregou. Consequência para a cascata a jusante (registrada também no
prompt 0013 §"Nota sobre enriquecimento"):

- **A E2 (parser textual de fontes) e o enriquecimento por flag (ADR-0005
  Ajuste 3) tornam-se provavelmente obsoletos para o caso de trait.** O trait
  vem de graça no JSON, com id correto. O `lente_investiga`/`lente_resolve`
  podem usar o `trait_` por nó diretamente.
- A decisão de aposentar a E2 fica para o próximo prompt (lente_investiga),
  quando a integração confirmar que o trait-por-nó cobre os casos.

---

## Decisões tácitas

### D1 — `crate` por nó não existe; premissa do laudo 0012 D1 não se confirmou

O laudo 0012 D1 justificou incluir `crate_name` no `No` afirmando "o fork
0.27.0 emite o crate por nó". **A verificação do JSON real refutou isso** — o
fork não emite `crate` por nó (nem com `--sysroot`). Consequências:

- Não adicionei campo `crate` ao `NoDTO`.
- `No.crate_name` é preenchido com o crate-raiz do **grafo** (topo) — todos os
  nós ganham o mesmo valor. Para nós de stdlib, fica o crate-alvo (não "core").
- O campo `crate_name` no `No` **permanece** (não removi do lente_core), mas
  seu propósito original (marca de stdlib por nó) não é atendível por ora. A
  marca de stdlib continua computável pelo **prefixo do path** (ADR-0002 D3),
  como antes — não foi substituída.
- O critério do prompt "nó com crate=core → crate_name=core" **não é
  satisfazível** com o fork atual. Não implementado (impossível); registrado.

Se for desejável `crate_name` por nó de verdade, é mais uma rodada no fork
(emitir `crate` por nó) — análogo às anteriores.

### D2 — `cfg` é estruturado; serializo para texto

O fork manda `cfg` como árvore (`[{"Flag":"unix"}]`), não string. O
`lente_core` modela `cfg: Option<String>` (laudo 0012, "texto não
interpretado"). Ponte: DTO aceita `Option<serde_json::Value>`; a tradução faz
`v.to_string()`, gerando o JSON compacto como texto. Não perde informação;
quando a lente for processar cfg de verdade, parseia esse texto ou muda o
modelo. Testado em `cfg_estruturado_do_fork_vira_texto`.

### D3 — Todos os campos do descritor com `#[serde(default)]`

O fork emite os campos só quando aplicam (bool só quando true; Option ausente
quando não há). `#[serde(default)]` faz ausência virar `false`/`None`, não
erro. **Contraste deliberado com `id`** (laudo 0006), que NÃO tem default —
sua ausência distingue fork novo de antigo e deve falhar. Os campos do
descritor são opcionais por natureza; sua ausência é normal.

### D4 — `macro_kind` mantido, não exercitado

O `macro_rules!` do crate-teste não gerou um nó com `macro_kind` (o fork pode
não modelar `macro_rules` declarativas, ou usar outro mecanismo). Mantive o
campo no DTO com default `None`; não consegui um caso real que o populasse.
Se aparecer (macros procedurais?), o campo já está pronto. Registrado como
não-verificado.

### D5 — `crate_name` capturado uma vez antes do loop

Para evitar N clones de `dto.crate_name` (um por nó), capturo
`let crate_name = dto.crate_name.clone()` antes do loop e clono a referência
local. Mecânico, já estava assim desde o ajuste do 0012.

---

## Sinalização para o próximo prompt (lente_investiga)

1. **O `trait_` por nó resolve a D4 sem adivinhação.** A evidência
   `ImplDeTraitsDiferentes` pode agora carregar o trait do nó **com o id
   correto** — basta o `lente_investiga` ler `no.trait_` de cada cópia
   colidente, em vez de parsear fontes.
2. **Avaliar aposentar a E2.** Se o trait-por-nó cobre os casos, a Estratégia
   2 (parser textual) e todo o módulo `fontes.rs` podem ser removidos. Medir
   antes de remover (pode haver casos sem `trait_` onde a E2 ainda ajude — ex.:
   colisões que não são de impl-de-trait).
3. **O `lente_resolve` já aceita `ImplDeTraitsDiferentes`** (laudo 0010) — a
   nomeação por trait fica exata quando a evidência vier do `trait_` por nó.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | Desserialização real do descritor (fork 0.27.0). Modificadores dos booleanos; trait via rename; cfg estruturado→texto; campos opcionais com serde default. Descoberta: fork não emite `crate` por nó (refuta laudo 0012 D1); `trait_` por nó resolve a D4 na raiz (E2E confirma Display+Debug). 74 testes verdes + 2 ignored. | `03_infra/src/dto.rs`, `03_infra/src/traducao.rs`, `03_infra/src/lib.rs` |
