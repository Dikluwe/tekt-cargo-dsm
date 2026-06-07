# Laudo de Execução — Prompt 0049 (rodar o `crystalline-lint` sobre o projeto — verificação)

**Camada**: transversal (verificação arquitetural)
**Data**: 2026-06-06
**Prompt executado**: `00_nucleo/prompt/0049-tekt_linter.md`
**Estado**: `EXECUTADO` — `crystalline-lint` rodado; `crystalline.toml` criado;
**todas** as violações levantadas, contadas e classificadas. **Verificação, não
conserto**: nenhum cabeçalho reescrito, nenhum diretório renomeado, nenhum código
de produção tocado. Único artefato criado: `crystalline.toml` (+ este laudo).

---

## A resposta em uma sentença

Os invariantes arquiteturais **reais** (direção de import V3, I/O no L1 V4, import
do `lab` V10, vazamento de porta V9, estado mutável no L1 V13, contratos V11)
estão **todos limpos (0 violações)**; o ruído é convenção (V1 — formato de
cabeçalho, 40) e estrutura (V8 — os seis crates L1 vs o modelo de-um-diretório do
linter, 8 fatal); os dois "quase-reais" são um **falso positivo** (V14, `use
Kind::*`) e uma observação discutível (V12 — enums no L4).

---

## Versão e instalação

- **`crystalline-lint v0.1.0`** (sem flag `--version`; obtido via
  `cargo install --list`).
- Instalado de um **clone local**: `/home/dikluwe/Documentos/Antigravity/tekt-linter`
  (não via `--git`). Vantagem: pude **ler a fonte** do linter para resolver o
  crux e o abort (abaixo) com certeza, não por tentativa.

---

## O `crystalline.toml` criado e o CRUX dos seis crates L1

**Determinado pela fonte** (`03_infra/config.rs:44`):
`pub layers: HashMap<String, String>` — o `[layers]` mapeia **um diretório por
camada**. **A lista NÃO é aceita.** Logo só `01_core` é mapeável como L1; os
crates `05_investiga`, `06_resolve`, `07_filtro`, `08_ranking`, `09_estrutura`
ficam **fora de qualquer camada** → **V8 (AlienFile, fatal)**. É o **achado
estrutural** que o prompt previu: o modelo de-um-diretório-por-camada do linter
vs a estrutura multi-crate-L1 do projeto. **Não reestruturei** (decisão pendente).

Config (raiz): `[layers]` L0–L4 + lab (um dir cada); `[excluded]`
target/.git/.cargo; `[module_layers]`/`[l1_ports]` = `entities`+`domain` (os
módulos do `01_core`); `[l1_allowed_external] rust = []` (L1 puro — V14 deveria
confirmar). Todos os campos do config têm `#[serde(default)]`, então o parcial
parseia.

---

## O abort que o prompt não previu: o linter **para** sem `00_nucleo/prompts`

`crystalline-lint .` (default, todos os checks) **aborta de cara**:

```
crystalline-lint: prompt scan error: NucleoUnreadable { path: "./00_nucleo/prompts", … }
```

O linter exige `00_nucleo/prompts` (**plural**); o projeto usa `00_nucleo/prompt`
(**singular**). Não é uma violação por-arquivo — é um **abort fatal** antes de
qualquer relatório. **Determinado pela fonte** (`04_wiring/main.rs:75`): o scan de
prompts roda **só quando V7 está ligado**. Logo, rodar **sem v5/v6/v7** passa pelo
abort e produz o relatório estrutural/camadas. Foi o que fiz:

```
crystalline-lint . --checks v1,v2,v3,v4,v8,v9,v10,v11,v12,v13,v14
```

(Nenhum diretório renomeado, nenhum symlink — só a flag `--checks`.) Os checks
dependentes de prompt foram caracterizados à parte (abaixo).

---

## Relatório completo de violações (por ID, com contagem)

| ID | Nível | Qtde | O quê |
|----|-------|------|-------|
| **V1** | error | **40** | Arquivo Cristalino sem `@prompt` no formato do linter |
| **V8** | fatal | **8** | Arquivo fora da topologia (crates 05–09) |
| **V12** | warning | **5** | Declaração (enum) no L4 (`04_wiring/src/lib.rs`) |
| **V14** | error | **1** | "externo no L1" — `Kind` (`01_core/.../grafo.rs:183`) |
| V2,V3,V4,V9,V10,V11,V13 | — | **0** | (nada) |
| V5, V6 | warning | **0** | rodados isolados: `✓ No violations found` |
| V7 | — | **abort** | exige `00_nucleo/prompts` (plural) — não roda |

**Total no relatório estrutural: 54** (41 error + 8 fatal + 5 warning).

### V8 (8 fatal) — os crates L1 fora de `01_core`

`05_investiga/{fontes,lib,vizinhanca}.rs`, `06_resolve/lib.rs`,
`07_filtro/{lib.rs, tests/e2e_lente_core.rs}`, `08_ranking/lib.rs`,
`09_estrutura/lib.rs`. Exatamente os 05–09. **Estrutural** (ver crux).

### V1 (40 error) — formato de cabeçalho

Por área: `03_infra` 11 (3 são fixtures), `01_core` 9, `lab` 6, `02_shell` 5,
`05_investiga` 3, `07_filtro` 2, `04/06/08/09` 1 cada. Os arquivos do projeto
**têm** linhagem — no formato `//! Lineage: prompt 00_nucleo/prompt/<nome>.md` —
mas o linter espera `//! Crystalline Lineage` + `@prompt 00_nucleo/prompts/…`.
**Desalinhamento de convenção**, não ausência de linhagem.

### V12 (5 warning) — enums no L4

`FonteGrafo`, `Escopo`, `ModoUses`, `AlvoBusca`, `ErroLente` — declarados em
`04_wiring/src/lib.rs`. V12 proíbe `enum_item` no L4 (L4 "não cria tipos").

### V14 (1 error) — **FALSO POSITIVO**

`01_core/src/entities/grafo.rs:183` é `use Kind::*;` — **glob de um enum local**
(o próprio `Kind`, definido no mesmo arquivo). O heurístico do V14 leu `Kind` como
um pacote externo. **Não é dependência externa.** O L1 é puro (confirmado por
`cargo tree -p lente_core` = só o crate). Não adicionei `Kind` ao
`l1_allowed_external` (seria errado — não é externo).

---

## Avaliação por categoria

| Categoria | Violações | Veredito |
|---|---|---|
| **Real (arquitetural)** | **nenhuma genuína** | V3/V4/V9/V10/V11/V13 = 0; V14 é falso positivo; V12 é discutível (API do fio) |
| **Estrutural** | V8 (8) | o crux: 6 crates L1 vs modelo de-um-diretório. Decisão pendente |
| **Formato/cabeçalho** | V1 (40), V7 (abort), V5/V6 (0) | convenção: `//! Lineage:` vs `@prompt`; `prompt` vs `prompts` |
| **Configuração** | — | o config mapeou o que dá; o V8 **não** é ajustável por config (sem lista) |

**Achados reais (V3/V4/V9/V10/V11/V13/V14):** o único que disparou é o **V14, e é
falso positivo**. Os invariantes que o projeto afirma — L1 puro (sem I/O, sem
externo, sem estado mutável), direção de dependência, sem import do `lab`,
disciplina de portas — **passam todos**. É a confirmação que importava.

---

## O caminho dos prompts (`prompt` vs `prompts`) e seu efeito

- **V7**: aborta o linter inteiro (hard) — exige `00_nucleo/prompts`.
- **V1**: contribui (o `@prompt` esperado referenciaria `00_nucleo/prompts/…`).
- **V5/V6**: 0 — nossos cabeçalhos não têm `@prompt-hash`, então drift/stale não
  têm o que comparar (não é "limpo de verdade"; é "não aplicável ao nosso formato").

---

## Plano proposto (para decisão — **não executado**)

1. **V8 / estrutura (o crux)** — decidir entre: (a) **aceitar** o V8 como
   divergência conhecida (o projeto é multi-crate-L1 por design; não reestruturar);
   (b) pedir ao `tekt-linter` suporte a **lista** em `[layers]` (a correção mora no
   linter, não no projeto); (c) um wrapper que rode o linter por-crate-L1. **Não**
   mover os crates 05–09 sem decisão.
2. **V1 + V7 / convenção de cabeçalho e caminho** — se for adotar o linter como
   gate, é uma **migração de convenção** grande: cabeçalhos `//! Lineage:` →
   `//! Crystalline Lineage / @prompt`, e `00_nucleo/prompt/` → `prompts/`. É
   reescrita em massa + rename de diretório — **decisão consciente**, fora desta
   passada. Enquanto isso, rodar com `--checks` sem v5/v6/v7.
3. **V12 / enums no L4** — decidir se `ErroLente` (erro agregado), `FonteGrafo`,
   `Escopo`, `ModoUses`, `AlvoBusca` são **API legítima do fio** (provável — são a
   fronteira pública do `lente_wiring`) ou se migram para L2/L3. É warning; o
   projeto pode declará-los intencionais (ou configurar a exceção).
4. **V14 / falso positivo** — reportar **upstream** ao `tekt-linter`: `use
   LocalEnum::*;` é lido como import externo. Nenhuma mudança no projeto.

---

## Critérios de Verificação — atendidos

- Versão registrada (v0.1.0, clone local). ✓
- `crystalline.toml` criado, mapeando a estrutura real; tratamento dos 05–09
  registrado (V8, sem suporte a lista — confirmado na fonte). ✓
- Relatório de violações por ID e arquivo, com contagem. ✓
- Cada grupo classificado (real / config / formato / estrutural). ✓
- **Nada** reescrito/renomeado/mexido — só o `crystalline.toml`. ✓

---

## Estado do projeto

| Item | Resultado |
|------|-----------|
| Código de produção | **intocado** — suíte segue 273 verdes + 28 ignored (0048) |
| Artefato novo | `crystalline.toml` (config do linter) |
| Invariantes reais (V3/V4/V9/V10/V11/V13) | **limpos (0)** |
| Achado estrutural | V8 (8) — 6 crates L1 vs modelo do linter |
| Ruído de convenção | V1 (40), V7 (abort), V5/V6 (n/a) |
| Falso positivo | V14 (`use Kind::*`) — reportar upstream |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-06 | Verificação arquitetural com o `crystalline-lint v0.1.0` (clone local). Criado `crystalline.toml` mapeando a estrutura real. **Crux confirmado na fonte** (`config.rs`: `layers: HashMap<String,String>`): `[layers]` não aceita lista → os 6 crates L1 viram só `01_core` mapeado; `05`–`09` disparam **V8 (8 fatal)** — achado estrutural, não reestruturado. **Abort descoberto** (`main.rs:75`): o scan de prompts (V7) aborta sem `00_nucleo/prompts` (plural; projeto usa `prompt` singular) — contornado rodando `--checks` sem v5/v6/v7 (sem renomear/symlink). Relatório: V1=40 (formato de cabeçalho `//! Lineage:` vs `@prompt`), V8=8 (estrutural), V12=5 (enums no L4 — discutível, API do fio), V14=1 (**falso positivo**: `use Kind::*` lido como externo). **V2/V3/V4/V9/V10/V11/V13 = 0** — os invariantes reais (L1 puro, direção de import, sem `lab`, portas, sem estado mutável) passam todos. V5/V6=0 (n/a — sem `@prompt-hash`). V7 não roda (abort). Plano proposto p/ decisão: aceitar V8 ou pedir lista ao linter; migração de convenção V1/V7 (grande, fora desta passada); V12 declarar API do fio; V14 reportar upstream. **Verificação, não conserto**: nenhum código/cabeçalho/diretório tocado — só o `crystalline.toml`. | `crystalline.toml`, `00_nucleo/lessons/0049-tekt_linter.md` |
