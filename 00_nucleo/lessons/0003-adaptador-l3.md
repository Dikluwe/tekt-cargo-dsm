# Laudo de Execução — Prompt 0003 (Adaptador da Fonte / L3)

**Camada**: L5 (laudo)
**Data**: 2026-05-27
**Prompt executado**: `00_nucleo/prompt/0003-adaptador_l3.md`
**Spec de origem**: `00_nucleo/specs/forma-organizada.md` (com 5 limites)
**ADRs aplicáveis**: 0001 (fonte), 0002 (modelagem), **0003 (workspace Cargo)**
**Depende de**: laudo 0001 (tipos), laudo 0002 (cálculo do raio); fork do
`cargo-modules` instalado e no PATH (ADR-0001)
**Estado**: `EXECUTADO` (compila, testes verdes, pureza L1 preservada, E2E ok)

---

## O que o prompt pediu

Primeiro componente fora de `lente_core`: invocar o fork do `cargo-modules`,
deserializar o JSON, validar enums **na borda**, validar invariantes da spec,
materializar em `lente_core::Grafo`. Materializa o ADR-0003 criando o
workspace Cargo e o crate `lente_infra`.

---

## O que foi gerado

| Arquivo | Propósito |
|---------|-----------|
| `Cargo.toml` (raiz) | Workspace (`members = ["01_core", "03_infra"]`, `exclude = [".../crate-amostra"]`). |
| `03_infra/Cargo.toml` | Crate `lente_infra`. Deps: `lente_core` (path), `serde` (derive), `serde_json`. |
| `03_infra/src/lib.rs` | API pública: `extrair_grafo(&std::path::Path)`; enum `ErroAdaptador` (10 variantes, `Display`+`Error`). |
| `03_infra/src/invocacao.rs` | Subprocesso. Lê `[package].name` do `Cargo.toml`. Roda `cargo modules export-json --sysroot --compact --package <nome>`. |
| `03_infra/src/dto.rs` | Structs-espelho `serde::Deserialize` (campos string para `kind`/`visibility`/`relation`). |
| `03_infra/src/traducao.rs` | DTO → `lente_core::Grafo`. Conversão via `TryFrom<&str>`. Valida invariantes 1 e 2. |
| `03_infra/tests/fixtures/crate-amostra/` | Crate-fixture isolado (`[workspace]` vazio) para teste E2E. |

**Verificação**:
- `cargo build` (workspace): limpo.
- `cargo test` (workspace): **35 testes**: `lente_core` 22/22 + `lente_infra` 13/13 (+ 1 `#[ignore]` E2E).
- `cargo test -p lente_infra -- --ignored`: E2E **1/1** (extrai grafo do fixture, valida estrutura).
- `cargo tree -p lente_core`: **só `lente_core`** — pureza L1 preservada.

---

## Decisões tácitas

### D1 — Divisão interna em 4 módulos por responsabilidade

`invocacao` (subprocesso e descoberta de pacote), `dto` (struct-espelho serde),
`traducao` (DTO → tipos do `lente_core` + validação de invariantes), `lib`
(API pública + enum de erro). Traducao é **pura** — testável sem subprocesso,
o que sustenta a maioria dos testes unitários.

### D2 — `ErroAdaptador` com 10 variantes específicas

Em vez de um único `String` genérico, cada modo de falha tem variante própria.
Variantes adicionadas além do prompt original:

- `CargoTomlAusente(String)` — descoberta de pacote falhou (D6).
- `CargoTomlSemPackage(String)` — caminho aponta para workspace puro, não
  para um crate.

Cada uma com mensagem `Display` específica para diagnóstico. Implementa
`core::error::Error`.

### D3 — `--package <nome>` é **obrigatório** sempre (descoberto pelo E2E)

O prompt previa `cargo modules export-json --sysroot --compact`. O primeiro
E2E descobriu que, em workspace, o `cargo modules` exige `--package <nome>`
para desambiguar. Como o projeto-lente é workspace (ADR-0003), o adaptador
precisa **sempre** passar `--package`. Solução: ler o `name` do `[package]`
do `Cargo.toml` local antes de invocar.

Implementação: parser de TOML linha-a-linha (procura `[package]` depois `name
= "..."`). Não adiciona dep `toml` — preferência pelo mínimo.

Trade-off: parser limitado ao subset comum (não cobre TOML "exótico" com
escapes ou multi-line strings em `name`). Suficiente para o uso real;
substituir por crate `toml` se aparecer caso que quebre.

### D4 — Renome `crate` → `crate_name` via serde

O campo JSON `"crate"` colide com palavra reservada em Rust. Serde resolve
com `#[serde(rename = "crate")]` no DTO. Coerente com a renomeação que o
`lente_core::Grafo` faz no L0 (laudo 0001, D5).

### D5 — Validação na borda: usa `TryFrom<&str>` do `lente_core`

Cada `kind`/`visibility`/`relation` do DTO (string) passa por
`TryFrom<&str>` ao virar `Kind`/`Visibility`/`Relation`. O erro
`ValorDesconhecido` do `lente_core` é embrulhado em
`ErroAdaptador::ValorDesconhecido`. Coerente com ADR-0002 D1: validação
acontece exatamente uma vez, no L3.

### D6 — `descobrir_pacote()` lê Cargo.toml em vez de chamar `cargo metadata`

Alternativa rejeitada: usar `cargo metadata --no-deps --format-version 1`
(JSON nativo, sem TOML). Razão da rejeição: adiciona um segundo subprocesso
por extração; o parser linha-a-linha é mais leve. Custo aceito: lida só com
o subset comum de TOML.

### D7 — Fixture interno com `[workspace]` vazio

Tentativa inicial: usar `lente_core` como fixture (sugestão do prompt).
**Bloqueou** por descoberta importante (ver "Descobertas").

Tentativa 2: criar fixture em `03_infra/tests/fixtures/crate-amostra/` com
`exclude` no Cargo.toml raiz. **Bloqueou** porque o fixture está dentro de
um membro do workspace, e `exclude` da raiz não o alcança.

Solução: adicionar `[workspace]` (vazio) ao `Cargo.toml` do fixture. Isso o
declara como projeto independente, fora do workspace pai. Combinada com
`exclude` na raiz (defesa em profundidade — duas barreiras).

### D8 — Teste E2E com `#[ignore]`

O E2E invoca o fork como subprocesso real — depende do binário instalado
(ADR-0001) e leva alguns segundos. Marcado `#[ignore]` para não pesar no
ciclo de TDD rápido (`cargo test` sem flags); rodado com `cargo test --
--ignored` quando a integração de fato precisa validar.

Não é "teste pulado": é "teste de integração que requer ambiente". Rust não
tem `skip` condicional limpo; `#[ignore]` é o caminho idiomático.

### D9 — Pureza separada por crate

A pureza de `lente_core` agora se verifica com `cargo tree -p lente_core`
(em vez de `cargo tree` global). Esperado e desejado: o workspace agrega,
mas cada crate é auditável separadamente. ADR-0003 D1 nomeia isso: "a
pureza de cada estrato fica verificável separadamente".

### D10 — `01_core/Cargo.lock` removido

Ao virar workspace, o lockfile migra para a raiz (`Cargo.lock` na raiz). O
de `01_core/` é removido para evitar duas fontes. Hoje há `Cargo.lock` na
raiz, gerado pelo primeiro `cargo build` do workspace, que entra no commit
(decisão do `.gitignore` — projeto-aplicação).

### D11 — Sysroot sempre ligado

`--sysroot` é flag fixa do comando, não opcional. Política da lente
(ADR-0001, Limite 1 da spec). Coerente com o prompt.

---

## Descobertas (durante a execução)

### Descoberta 1 — `cargo modules` em workspace exige `--package <nome>`

O prompt e o ADR-0001 invocavam `cargo modules export-json --sysroot
--compact` sem `--package`. Funciona em crate isolado. **Em workspace
falha** com `Multiple packages present in workspace, please explicitly
select one via --package flag`. Como o projeto agora é workspace (ADR-0003),
o adaptador precisa **sempre** descobrir o pacote e passar `--package`.
Resolvido na D3.

### Descoberta 2 — `lente_core` viola o invariante 1 da spec

Quando o E2E foi tentado contra `01_core/` (o próprio `lente_core`), a
tradução falhou com:

```
PathDuplicado(Path("lente_core::domain::raio::ErroRaio::fmt"))
```

Causa: `ErroRaio` tem `impl fmt::Display` (escrito no código) **e**
`#[derive(Debug)]` (que gera implicitamente um `impl fmt::Debug`). Os dois
declaram um método chamado `fmt`. O `cargo-modules` emite ambos no JSON com
**o mesmo `path` qualificado** (sem incluir o trait no caminho).

**Conflito com a spec**: o invariante 1 declara que `path` é único entre os
nós. O fork viola.

**O adaptador está correto**: o prompt pede explicitamente "não corrigir
silenciosamente" — então a tradução rejeita o JSON. Mas isso significa que
a spec, como está escrita hoje, **não cobre a realidade da fonte**.

**Candidatos a resolver** (não escolhidos aqui — decisão para o autor da
spec):

- **Spec relaxa**: adicionar Limite 6 ("paths podem colidir entre métodos
  de impls diferentes para o mesmo tipo"); identidade passa a ser tupla
  (`path` + algum discriminador).
- **Spec mantém, fork muda**: pedir ao fork que inclua o trait no path
  (ex.: `ErroRaio::<Display>::fmt` vs `ErroRaio::<Debug>::fmt`). Provável
  caminho da Nota de Evolução já registrada na spec.
- **Adaptador sanea**: consolidar duplicatas (perder informação) ou
  numerar (`...::fmt`, `...::fmt#2`). Adia o problema, suja os dados.

Esta descoberta é **a contribuição mais importante** deste prompt para a
spec: ela teve que estar errada ou incompleta para que essa colisão ficasse
visível. Próximo passo natural: discutir em ADR-0004 ou atualizar a spec
com Limite 6.

Como contorno operacional, o E2E desta versão usa um fixture interno
(crate-amostra) que **não** tem essa colisão (porque não tem `impl Display`
+ `derive Debug` no mesmo enum). Permite que a integração seja exercitada
fim-a-fim sem depender da resolução acima.

### Descoberta 3 — `exclude` no Cargo.toml raiz não alcança fixtures dentro de membros

`Cargo.toml` raiz com `exclude = ["03_infra/tests/fixtures/crate-amostra"]`
**não** isolou o fixture, porque o cargo sobe a árvore procurando workspace
e encontra o pai mesmo com `exclude`. Solução conjuntiva: adicionar
`[workspace]` vazio ao `Cargo.toml` do fixture. Documentado também por uma
mensagem útil do próprio cargo. Marcador `exclude` permanece como defesa
adicional.

---

## Critérios de Verificação atendidos

| Critério | Status | Teste |
|----------|--------|-------|
| Crate Rust válido → `Ok(Grafo)` com nó-raiz e nós esperados | ✓ | `e2e_extrai_grafo_de_fixture` |
| Grafo respeita invariantes 1 e 2 | ✓ | `path_duplicado_e_invariante_violado`, `aresta_orfa_no_*` |
| Enums kind/visibility/relation: variantes válidas | ✓ | `*_falha_na_borda` (verifica rejeição), `traduz_grafo_com_arestas_validas` (verifica aceitação) |
| Caminho não é crate Rust → erro diagnóstico | ✓ | `diretorio_inexistente_da_cargo_toml_ausente`, `workspace_puro_sem_package_devolve_erro_claro` |
| JSON malformado → erro | ✓ | `json_invalido_resulta_em_erro_diagnosticavel` |
| Display cobre toda variante | ✓ | `erro_implementa_display_para_cada_variante` |
| Não-regressão de `lente_core` | ✓ | 22/22 testes verdes |
| Pureza L1 preservada | ✓ | `cargo tree -p lente_core` mostra só o crate |

Critério explicitamente não-coberto neste momento:

- **Conversão de `lente_core` como fixture** (sugestão do prompt): não cobre,
  pelos motivos da Descoberta 2. Substituído por fixture interno
  (`crate-amostra`).

---

## O que o prompt explicitamente não pediu (não fiz)

- **Não implementei cache**. Prompt declara que cache é responsabilidade
  futura.
- **Não filtrei stdlib**. É componente L1 separado, futuro.
- **Não corrigi silenciosamente nenhum JSON**. Path duplicado / aresta órfã
  resultam em `Err`. Honesto.

---

## Próximos passos (referência, não compromisso)

1. **Decidir sobre a Descoberta 2** — discussão arquitetural pendente.
   Sugiro um ADR-0004 ou uma atualização da spec com Limite 6, antes de
   tentar usar `lente_core` (ou qualquer crate com derives + impls do mesmo
   trait) como fonte real.
2. **Filtro de stdlib** (L1) — Limite 2 da spec, há tempos previsto.
3. **Refinar `descobrir_pacote`** — substituir parser linha-a-linha por
   crate `toml` se aparecer Cargo.toml exótico que ele não cobre. Adiar até
   sintoma concreto.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Execução inicial do prompt 0003. Workspace Cargo materializado. Adaptador L3 (`lente_infra`) com invocação, DTO, tradução, 10 variantes de erro. 35 testes verdes (22 lente_core + 13 lente_infra + 1 E2E `--ignored`). Pureza L1 preservada. | `Cargo.toml` (raiz), `03_infra/*`, `01_core/Cargo.lock` (removido) |
