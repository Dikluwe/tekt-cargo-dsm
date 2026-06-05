# Laudo de Execução — Prompt 0022 (Invocador cobre bin+lib)

**Camada**: L5 (laudo)
**Data**: 2026-06-01
**Prompt executado**: `00_nucleo/prompt/0022-l3-invocador-bin-lib.md`
**Decisões de origem**: laudo 0021 Bloco A — `egui_demo_app` (único pacote
bin+lib do egui) falhou porque o invocador não passava `--lib`/`--bin`;
pendência 4 da transferência.
**Estado**: `EXECUTADO` — detecção de alvo (`--lib` / `--bin <nome>`)
adicionada à `invocacao`; bin+lib agora roda; subprocess único preservado
(laudo 0018); pureza do L1 intacta; 114 verdes + 7 ignored (todos passam).

---

## Fase 1 — Leitura e verificação contra o binário real

### Chamadores das duas portas (`grep -rn`)

| Chamador | Porta | Modo |
|----------|-------|------|
| `04_wiring/src/lib.rs:102` | `fork::invocar_fork(&p)` | `FonteGrafo::Pacote` (cwd herdado) |
| `lab/medicao-egui/src/main.rs:164` | `lente_infra::extrair_grafo(&crate_path)` | via `invocacao::invocar` (diretório explícito) |
| `03_infra/src/lib.rs:115` | `invocacao::invocar(caminho_crate)` | dentro de `extrair_grafo` |

Subprocess único pré-existente: `03_infra/src/fork.rs:81` —
`Command::new("cargo")`.

### Flags reais do fork (`cargo modules export-json --help`)

Confirmadas: `--lib` (sem argumento) e `--bin <BIN>` (nome). O `-p, --package`
já estava em uso. Demais flags da política da lente (`--sysroot`,
`--compact`) já fixas no invocador.

### Mensagens de erro reais (rodadas contra fixtures em `/tmp`)

| Cenário | Exit | Stderr |
|---------|------|--------|
| bin+lib **sem** flag | 1 | `Error: Multiple targets present in package, please explicitly select one via --lib or --bin flag.` + lista `- name (--lib)` / `- name (--bin name)` |
| só-bin com `--lib` | 1 | `Error: No library target found.` |
| só-bin (1 alvo) sem flag | 0 | JSON ok (cargo resolve sozinho) |
| bin+lib com `--lib` | 0 | JSON ok |
| multi-bin sem lib, sem flag | 1 | `Error: Multiple targets present in package…` |

**Achado decisivo da Fase 1**: passar `--lib` cegamente **regride** os pacotes
só-bin (cenário 2). Não basta acrescentar a flag — é preciso detectar antes.
A leitura do `--help` sozinha não revelaria isso; a verificação contra o
binário real, sim. (Coerente com o princípio do prompt: as suposições do
componente foram pagas por laudos passados — 0012/0013.)

---

## Fase 2 — Conserto

### Estrutura final (deltas ao desenho do laudo 0018)

```
03_infra/src/fork.rs
  + pub(crate) enum AlvoFork { Lib, Bin(String) }
  ~ invocar_em(pacote, current_dir, alvo: Option<&AlvoFork>)   (+1 parâmetro)
      adiciona --lib ou --bin <nome> antes de --package
  = invocar_fork(pacote)  → invocar_em(pacote, None, None)     (compat total)

03_infra/src/invocacao.rs
  + parse_alvos_do_toml(&str) → { pacote, tem_lib_explicito, bins_explicitos }
    (uma passada; descobrir_pacote agora delega a ele)
  + listar_bins_dir(&Path)    → Vec<String>  (src/bin/<x>.rs e src/bin/<x>/main.rs)
  + detectar_alvo(&Path)      → Result<AlvoFork, ErroAdaptador>
  ~ invocar(diretorio)        chama detectar_alvo e passa Some(&alvo) ao fork

03_infra/src/lib.rs
  + ErroAdaptador::AlvosAmbiguos { bins: Vec<String> }
    (com Display que lista os binários encontrados)
```

`invocar_em` continua sendo o **único** `Command::new("cargo")` do crate
(laudo 0018 preservado).

### Regra de escolha do alvo (implementação)

| Condição | Alvo |
|----------|------|
| `[lib]` no Cargo.toml **ou** `src/lib.rs` existe | `AlvoFork::Lib` |
| sem lib, 1 binário (de `[[bin]]` ∪ `src/main.rs` ∪ `src/bin/…`) | `AlvoFork::Bin(nome)` |
| sem lib, 0 ou ≥2 binários | `ErroAdaptador::AlvosAmbiguos { bins }` |

A escolha "tem lib → lib" cobre bin+lib (o caso que motivou o prompt) sem
caso especial: a lente analisa estrutura de biblioteca, então quando há os
dois, lib ganha.

### Verificação grep do subprocess único

```
$ grep -rn 'Command::new("cargo")' --include "*.rs"
03_infra/src/fork.rs:92:/// **Esta é a única função do crate que roda `Command::new("cargo")`** —
03_infra/src/fork.rs:100:    let mut cmd = Command::new("cargo");
```

Uma única ocorrência real (linha 100). A linha 92 é doc-comment.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` (sem ignored) | **114 verdes** (era 108; +6 dos novos de `detectar_alvo`) |
| `cargo test -p lente_infra -- --ignored` | **5/5** verdes (4 pré-existentes + E2E novo `e2e_bin_mais_lib_passa_a_funcionar`) |
| `cargo build -p lente_core` | só o crate — pureza preservada (L1 inalterado) |
| Testes pré-existentes alterados | **um**: `erro_implementa_display_para_cada_variante` ganhou as 2 variantes novas de `AlvosAmbiguos` (lista vazia e com bins) |
| Chamadores externos tocados | **zero** (`invocar_fork(pacote)` e `extrair_grafo(diretorio)` mantêm assinaturas) |

### Cobertura dos testes novos

| Teste (em `03_infra/src/invocacao.rs`) | Caso |
|------|------|
| `detecta_lib_em_pacote_bin_mais_lib` | bin+lib → `Lib` (o caso que motivou o prompt) |
| `detecta_lib_so_pelo_layout_src_lib_rs` | sem `[lib]`, `src/lib.rs` existe → `Lib` |
| `detecta_bin_unico_pelo_src_main_rs` | sem `[[bin]]`, `src/main.rs` existe → `Bin(nome-do-pacote)` |
| `detecta_bin_unico_explicito_em_cargo_toml` | só `[[bin]]` com nome próprio → `Bin(nome)` |
| `varios_bins_sem_lib_devolve_erro_listando_nomes` | 2 `[[bin]]`, sem lib → `AlvosAmbiguos { bins: [a, b] }`, Display lista ambos |
| `detecta_bins_em_src_bin_subdir` | `src/bin/cli.rs` → `Bin("cli")` |
| `e2e_bin_mais_lib_passa_a_funcionar` `#[ignore]` | fixture bin+lib real → `invocar` produz JSON com `"crate":"binlib_e2e"` |

---

## Decisões tácitas

### D1 — Detecção via Cargo.toml + layout (não `cargo metadata`)

Duas opções legítimas: (a) parser do `Cargo.toml` + checar arquivos em
`src/`; (b) `cargo metadata --no-deps` e ler `targets[].kind`. Escolhi (a).

**Razão**: (b) introduziria um segundo `Command::new("cargo")` no crate, o
que o prompt explicitamente proíbe ("não se cria um segundo ponto de
invocação"). Mantém o invariante do laudo 0018 — subprocess único.

**Custo aceito**: (a) não cobre 100% das esquisitices da auto-descoberta do
Cargo (`autobins = false`, paths customizados em `[[bin]]`, etc.). Para o
escopo da lente — "rodar o fork com a flag certa, ou diagnosticar sem
adivinhar" — a heurística simples basta. Se um dia houver crate exótico que
escape da heurística, a saída é o `AlvosAmbiguos` (diagnóstico claro), não
uma falha obscura.

### D2 — `AlvoFork` é `pub(crate)`, não público

O enum vive em `fork.rs` (perto da primitiva que aplica a flag), mas com
visibilidade de crate. Razão: nenhum chamador externo precisa hoje, e
adicionar tipo público é difícil de retirar depois. Quando alguém precisar
(ex.: o L4 wiring querer escolher alvo explicitamente), promove-se.
"Aditivo" do prompt foi cumprido pelas assinaturas públicas
(`invocar_fork`, `extrair_grafo`) — todas inalteradas.

### D3 — Erro novo em vez de propagar `ErroFork::StatusErro`

O fork já dá mensagem clara para multi-bin sem lib ("Multiple targets
present in package…" + lista). Poderia-se deixar essa mensagem propagar via
`SubprocessoFalhou`. Decidi adicionar `ErroAdaptador::AlvosAmbiguos { bins }`
porque:
- A condição é decidida **pelo nosso código**, não pelo fork — o erro deve
  falar a linguagem do invocador.
- Chamadores podem casar a variante (`match`) sem grep no texto do stderr.
- Diagnóstico do `Display` é específico da lente ("a lente analisa
  estrutura de biblioteca") — informação que o fork não tem.

### D4 — Auto-descoberta tratada com heurística simples

O parser detecta `[lib]` e `[[bin]]` explícitos, e o `listar_bins_dir`
cobre `src/bin/<x>.rs` e `src/bin/<x>/main.rs`. Não inspeciona
`autobins`/`autolib` em `[package]` (raramente usado, e quando usado é
override conservador). Resultado: a heurística pode **superestimar** o
número de bins em crates esquisitos — saída é `AlvosAmbiguos`, que é
diagnóstico claro, não falha obscura. Coerente com o "não-adivinhar" do
prompt.

### D5 — `descobrir_pacote` refatorado, não removido

`descobrir_pacote` agora delega a `parse_alvos_do_toml` (uma única passada
em vez de duas). Mantido como função independente porque seus testes
pré-existentes (`descobre_pacote_de_cargo_toml_simples`,
`workspace_puro_sem_package_devolve_erro_claro`) batem direto nele.
Não-regressão dos testes preservada sem mexer neles.

---

## Não-regressão registrada

| Crate | Antes | Depois |
|-------|-------|--------|
| lente_core | 30 / 0 ignored | 30 / 0 |
| lente_infra (não-ignored) | 21 | **27** (+6 detectar_alvo) |
| lente_infra (ignored) | 4/4 | **5/5** (+ E2E novo bin+lib) |
| lente_investiga | 17 | 17 |
| lente_resolve | 11 | 11 |
| lente_wiring | 6 / 1 ignored | 6 / 1 |
| lente_catalogo | 7 / 0 | 7 / 0 |
| lente_cli | 16 / 1 ignored | 16 / 1 |

**Total**: 114 verdes + 7 ignored (todos os ignored passam quando rodados,
incluindo o E2E real do bin+lib).

---

## Pendências reforçadas / cobertas

| Pendência | Estado pós-0022 |
|-----------|-----------------|
| Bin+lib via `extrair_grafo` (pendência 4 do laudo 0021) | **Coberta**. |
| Bin+lib via `fork::invocar_fork(pacote)` (caminho do wiring `--pacote`) | **Não coberta deliberadamente** — o wiring sem diretório não tem como detectar; falha continua com a mensagem clara do fork. Conserto futuro: ou o L2 passa o diretório, ou o `invocar_fork` ganha uma sobrecarga `invocar_fork_em(pacote, dir)`. Fora do escopo deste prompt (não regride). |
| Filtro de stdlib | **Não coberta** — prompt próprio (pendência 2 do laudo 0021). |
| Caso "vários bins sem lib" | **Diagnosticado, não resolvido** — `AlvosAmbiguos` com lista; o usuário decide manualmente qual bin analisar. Coerente com o tratamento de Limite 6 da spec. |

---

## O que NÃO mudou (declarado)

- `lente_core` (L1): zero toques — `cargo build -p lente_core` sem novas
  dependências.
- `lente_investiga` / `fontes.rs` / E2: em quarentena, intocados (laudo
  0014).
- `cargo-modules` (fork): nenhuma mudança — usamos flags que ele já tem.
- Assinaturas públicas (`invocar_fork`, `extrair_grafo`,
  `desserializar_grafo`): inalteradas.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | Invocador detecta o alvo (`--lib`/`--bin <nome>`) pelo `Cargo.toml` + layout `src/`; pacotes bin+lib passam a funcionar; subprocess único preservado; nova variante `ErroAdaptador::AlvosAmbiguos` para o caso de borda "vários bins sem lib". | `03_infra/src/fork.rs`, `03_infra/src/invocacao.rs`, `03_infra/src/lib.rs`, `00_nucleo/lessons/0022-l3-invocador-bin-lib.md` |
