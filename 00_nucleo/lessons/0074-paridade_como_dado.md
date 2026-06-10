# Laudo de Execução — Prompt 0074 (paridade como dado — comparar duas estruturas)

**Camada**: L1 (crate novo `lente_comparacao`: pareamento + deltas) + L4 (pipeline
`comparar`) + L2 (saída texto + JSON). Os pipelines de extração **não mudam**.
**Data**: 2026-06-10
**Prompt executado**: `00_nucleo/prompt/0074-paridade_como_dado.md`
**Estado**: `EXECUTADO` — `lente --comparar --antes <raiz> --depois <raiz>` reporta
a paridade entre duas formas (pareados, sem-par dos dois lados, deltas de
arestas/peso/ciclos) como texto + JSON. Validado com egui 0.33→0.34 via worktree.
Suíte **298 passed · 33 ignored** (exato); V1/V2=0, **V12=1**; `lente_comparacao` puro.

---

## A resposta em uma sentença

Refatorar cria a pergunta "a forma nova cobre a antiga?" — o `--comparar` responde
**como dado**: o que parou (pareados), o que só existe de um lado (sem-par,
declarado honestamente), e como dependências, pesos e ciclos mudaram entre os pares.

---

## Fase 1 — decisões

1. **Onde mora**: **crate novo `01_core/comparacao`** (`lente_comparacao`) —
   consistente com os 6 crates-de-concern do L1; consome duas `EstruturaModulos`
   (depende de `lente_estrutura` + `lente_core`), puro.
2. **Normalização**: descarta o **1º segmento** do path (nome do crate) —
   `velho::nucleo::raio` ⇒ `nucleo::raio`, pareia apesar do crate renomeado. O
   módulo-raiz (1 segmento) ⇒ `""` (os dois raízes pareiam). Dentro de um lado a
   normalização é **injetiva** (mesmo crate), sem colisão.
3. **CLI**: estilo flag (consistente com `--diff`/`--estrutura`): `--comparar
   --antes <dir> --depois <dir>`. Os rótulos vêm do `crate_name` extraído (não
   precisam de input). Default `seu-codigo` (procedência 0072); `--completo` restaura.
4. **Worktree**: receita validada (Fase 3) — **documentação, não código**:
   `git worktree add /tmp/lado <branch|tag>` reduz "branch X vs Y" a duas raízes.

---

## Fase 2 — construção

- **L1** (`lente_comparacao`): `comparar_estruturas(antes, depois, nome_a, nome_b)
  -> Comparacao` (pareados, sem_par dos dois lados, arestas comuns/só-antes/
  só-depois com peso, `ResumoCiclos` de cada lado). **`Lado` (Antes/Depois) mora
  aqui** (domínio da paridade) — ver "V12" abaixo. **5 testes**, incluindo o
  **teste-contrato** `movido_e_sem_par_dos_dois_lados`: `a::b`→`c::b` normaliza
  diferente ⇒ sem-par dos dois lados, **não** adivinhado.
- **L4** (`comparar`): extrai cada lado por **diretório** (`lente_infra::
  extrair_grafo`, fork dir-aware) com os **mesmos** parâmetros, resolve colisões,
  aplica escopo, e chama o L1. Fatorada `estrutura_de_grafo` (compartilhada com
  `analisar_estrutura`). Erro `ErroComparar { lado, erro }` identifica **qual lado**
  falhou.
- **L2** (`formatar_comparacao`): texto (cabeçalho com lados/escopo/modo + o
  **limite do pareamento** escrito na saída; resumo; sem-par; arestas que
  sumiram/apareceram; maiores deltas de peso; ciclos lado a lado) + `--json` (o
  contrato da tela lado a lado e do agente). **Sem nota única.**
- **E2E** `#[ignore]`: `lente_core` vs **si mesmo** (paridade total: só-antes 0,
  só-depois 0) e vs **cópia com módulo extra** (o extra aparece sem-par no depois).

### V12 — um enum a mais no fio, corrigido

A 1ª versão declarou `pub enum Lado` no **L4** (wiring) → **V12 disparou** ("enum
no fio"), subindo para V12=2. Como `Lado` (Antes/Depois) é conceito do **domínio da
paridade**, movi-o para `lente_comparacao` (L1); o fio só re-exporta. **V12 voltou a
1** (só o `ErroLente`, intencional). Achado registrado: structs em L4 não disparam
V12, enums sim — o erro de composição (`ErroComparar`, struct) fica; o vocabulário
(`Lado`, enum) desce.

---

## Fase 3 — uso real

**Receita worktree validada** e exercitada: `git worktree add /tmp/egui-033 0.33.0`
+ `/tmp/egui-034 0.34.0` → `lente --comparar --antes …/crates/egui --depois …`.

**egui 0.33.0 → 0.34.0** (escopo `seu-codigo`):

| Métrica | Leitura |
|---|---|
| pareados | **105** · só-antes **0** · só-depois **2** (`input_state::wheel_state`, `widget_style` — módulos novos; nada removido/renomeado a nível de módulo) |
| arestas | comuns **798** · sumiram **7** · **apareceram 67** — o acoplamento **densificou** (ex.: sumiu `ui → util::id_type_map`; surgiram `atomics::atom_ext`, `containers::menu`) |
| ciclos (maior SCC) | **87 → 89** — o emaranhado **cresceu**, não encolheu: a evolução de egui adensou o núcleo fortemente conexo, não o desfez |

A comparação respondeu "o que mudou na evolução" de forma direta. **O que o texto
não deixou ver** (fila da tela lado a lado): a *forma* da mudança — onde no triângulo
da DSM as 67 arestas novas caíram, se concentradas num módulo ou espalhadas. O JSON
já carrega o necessário (paths + pesos dos dois lados) para a tela pintar isso.

*(O "par real de refatoração do autor" — Fase 3 item 2 — fica disponível pela mesma
receita; a validação de uso veio do egui real + do E2E da cópia adulterada.)*

---

## Verificação

| Item | Resultado |
|------|-----------|
| Teste-contrato (movido = sem-par 2 lados) | passa |
| `cargo build --workspace` | passa |
| Suíte normal | **298 passed / 0 failed** (291 + 5 L1 + 2 cli comparacao) |
| Ignorados | **33** (exato: 31 + 2 E2E comparar) |
| E2E comparar | vs si mesmo (paridade total) + vs cópia (sem-par) — verdes |
| `--estrutura`/`--diff`/raio/MCP | intocados (aditivo) |
| `crystalline-lint .` | **V1=0, V2=0**; **V12=1** (`ErroLente` — `Lado` movido ao L1) |
| `cargo tree -p lente_comparacao` | **puro** (só `lente_core` + `lente_estrutura`) |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Paridade como dado (requisito do autor: projeto vs refatoração; lados = pastas/branches/repos, **reduzidos a duas raízes** via receita `git worktree` documentada). Crate novo **L1 `lente_comparacao`**: `comparar_estruturas` — pareamento por **path normalizado na raiz do crate** (1º segmento descartado: crate renomeado pareia; módulo movido `a::b`→`c::b` = **sem-par dos dois lados**, teste-contrato; sem heurística, sem nota única) + deltas (arestas comuns com peso antes/depois, só-antes/só-depois, `ResumoCiclos` lado a lado). `Lado` (Antes/Depois) também no L1 (domínio da paridade) — **corrigido um V12** que surgira ao declará-lo no fio (enum em L4 dispara V12; movido ao L1, V12 voltou a 1). **L4 `comparar`**: extrai cada lado por diretório (`lente_infra::extrair_grafo`, fork dir-aware) com parâmetros **forçados iguais** (default `seu-codigo`, 0072; `--completo` restaura), resolve colisões, fatorou `estrutura_de_grafo` (compartilhada com `analisar_estrutura`); `ErroComparar{lado,erro}` identifica o lado. **L2** texto (cabeçalho + limite do pareamento na saída) + `--json` (contrato da tela lado a lado). CLI `--comparar/--antes/--depois` (flag-style). E2E: `lente_core` vs si mesmo (paridade total) + vs cópia com módulo extra (sem-par). **Fase 3**: receita worktree validada; egui 0.33→0.34 — 105 pareados, +2 módulos, +67/−7 arestas (densificou), maior SCC 87→89 (emaranhado cresceu). Aditivo; `--estrutura`/`--diff`/raio/MCP intocados. Suíte **298 passed / 33 ignored** (nº exato, disciplina 0068); V1=0, V2=0, V12=1; `lente_comparacao` puro. Fila: tela lado a lado, detecção de movido por similaridade. | `01_core/comparacao/{Cargo.toml,src/lib.rs}` (novo), `Cargo.toml` (member), `04_wiring/{Cargo.toml,src/lib.rs}` (comparar + estrutura_de_grafo + re-export), `02_shell/cli/{Cargo.toml,src/saida.rs,src/args.rs}`, `02_shell/catalogo/src/lib.rs`, `04_wiring/app/src/main.rs` (dispatch + E2E), `00_nucleo/prompts/{comparacao,cli-args,cli-saida,wiring}.md` (snapshots), `00_nucleo/lessons/0074-paridade_como_dado.md` |
