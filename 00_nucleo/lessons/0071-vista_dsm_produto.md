# Laudo de Execução — Prompt 0071 (vista DSM de produção — `--estrutura --html`)

**Camada**: L1 (peso na agregação) + L2 (a vista HTML + montagem + flag) + L4
(dispatch). O cálculo (ordem, blocos, raio) **não muda**.
**Data**: 2026-06-10
**Prompt executado**: `00_nucleo/prompt/0071-vista_dsm_produto.md`
**Estado**: `EXECUTADO` — `lente --pacote <X> --estrutura --html` gera HTML
autocontido (matriz na ordem topológica + peso + blocos + fold por prefixo);
validado em **egui** (N=111) e **tekt-linter** (N=49), externos à lente. Suíte 289 +
ignorados verdes; V1/V2=0, V12=1.

---

## A resposta em uma sentença

A lente ganhou a **interface do humano** (Momento A): a DSM de estrutura como um
arquivo HTML autocontido que mostra a forma do projeto — o triângulo de
dependências, o quadrado do ciclo na diagonal, e agora a **densidade** (peso de
acoplamento) que o 0036 pedia — funcionando em qualquer projeto Rust.

---

## Fase 1 — leitura (decisões verificadas)

1. **Onde o peso morria** (Achado 1 do 0036): `agregar_por_modulo`,
   `lente_estrutura/src/lib.rs:158` — `if chaves.insert((from_mod, to_mod, Uses))`:
   a **1ª** aresta-de-item de um par módulo→módulo cria a aresta-de-módulo; as
   demais somem no dedup. O **peso = essa contagem**. Verificado contra o egui real:
   `egui::ui → egui::response` peso **80**, `context → viewport` 49 — os pares de
   maior acoplamento fazem sentido (são exatamente os esperados).
2. **Arena como referência**: `lab/proto-dsm/index.html` (350 linhas) — SVG, matriz
   na ordem, blocos emoldurados, tooltip. Sobe ao produto: o desenho. Muda: **dado
   embutido** (em vez de `fetch`), **peso** (cor por intensidade), **fold por
   prefixo**, cabeçalho honesto. A Arena fica intocada (referência histórica).
3. **Nível workspace — DEFERIDO com desenho** (conforme o prompt: "não forçar se
   exige peça nova"). `montar_grafo_workspace` (0045) e `ordenar_dsm`/
   `agregar_por_modulo` (genéricos) existem; mas uma DSM **por crate** precisa de uma
   **peça nova** — um `agregar_por_crate` / `mapa_crate_contenedor` (subir a cadeia
   `Owns` até o crate, não até o módulo). `agregar_por_modulo` sobre o grafo de
   workspace daria uma DSM **módulo-a-módulo de todos os crates** (reusa as peças, mas
   não é "por crate" e fica enorme). Como o nível módulo-por-pacote já entrega valor,
   **não expandi o escopo**. Desenho registrado para um prompt futuro.
4. **Tamanho do autocontido**: egui (N=111, 864 deps) → **76 KB**; lente_core → 13 KB.
   Viável sem minificação (o dado JSON domina; template ~9 KB).

---

## Fase 2 — construção

### L1 — o peso (mudança pequena, pura)

- `pesos_modulo_a_modulo(grafo) -> HashMap<(usize,usize), usize>`
  (`lente_estrutura`): conta as arestas-de-item `Uses` por par de módulo, mesma
  política do agregador (sem módulo contenedor / intra-módulo não contam). **Não
  toca `Aresta`** (o tipo L1 de entidade fica intacto) — o peso vive no tipo de
  **saída**.
- `DependenciaModulo` ganha `pub peso: usize`; `analisar_estrutura` (L4) o preenche
  do mapa. JSON ganha `{de, para, peso}` — **aditivo** (consumidores antigos
  ignoram). Puro: `cargo tree -p lente_estrutura`/`lente_core` **sem deps novas**.

### L2 — a vista (`02_shell/cli/src/dsm_template.html` + `formatar_estrutura_html`)

Template **autocontido** embutido por `include_str!`; o dado vai inline em
`const DADOS` (injeção única do MESMO JSON do `--json` + `pacote`/`limite`). A tela
é **vista** (padrão 0029): o JS só desenha e **dobra/desdobra rótulos** — nenhum
cálculo de estrutura (ordem/ciclos/peso vêm prontos do L1). Tem:

- grade N×N na `ordem`; **peso → intensidade** da célula (rampa azul log) + número
  no hover (densidade ≠ presença — fecha o Achado 1);
- **blocos de ciclo emoldurados** na diagonal;
- **fold por prefixo** (`crate::sub::*`): clicar um rótulo de grupo dobra/desdobra;
  célula de grupo = soma dos pesos dos membros (agregação de apresentação, não de
  estrutura); "dobrar/desdobrar tudo";
- **cabeçalho honesto**: pacote, escopo, modo de uses, contagens, e a declaração de
  limite (§3, do catálogo): forma **estática e estrutural**, não comportamental.

### L4 — dispatch

`--html` (requer `--estrutura`, conflita com `--text`) + `--saida` (default
`lente-estrutura.html`). Grava o arquivo e imprime o caminho (`stdout` leva a
mensagem, não o HTML). Strings no catálogo (ADR-0002).

---

## Fase 3 — uso real (dois projetos externos à lente)

### egui — o caso difícil conhecido (`/home/.../GitHub/egui`)

`lente --pacote egui --estrutura --html` (8,7s): **111 módulos, 864 deps, 1 ciclo de
85 módulos** (o SCC famoso do 0036). Leitura:

- **A forma "como um todo" funciona**: o bloco de ciclo de 85 salta (a moldura
  laranja domina a diagonal — egui tem um núcleo fortemente conexo enorme).
- **O peso distingue acoplamento**: `ui → response` (80), `context → viewport` (49),
  `memory → id` (35) — as arestas grossas saltam das finas; a densidade era
  invisível na vista só-presença do 0036.
- **O fold torna N=111 navegável**: dobrado por prefixo, egui cai para ~dezenas de
  grupos (`egui::widgets::*`, `egui::containers::*`); desdobra-se o que interessa.
  Sem fold, 111 rótulos de 5px são ilegíveis (Achado 3 do 0036, fechado).

### tekt-linter — segundo externo (`crystalline-lint`)

49 módulos, 128 deps, 1 ciclo pequeno (2). Leitura: arquitetura **limpa** (quase sem
ciclo); o acoplamento dominante são os **parsers → `entities::parsed_file`**
(`ts_parser` 33, `rs_parser` 31, `zig/py` 26) — o peso revela na hora que o
`parsed_file` é o hub compartilhado dos 4 parsers. Camadas (`infra` → `entities`)
visíveis no triângulo.

### O que faltou — a fila das próximas projeções

- **Escopo**: as vistas saíram em `completo` (sysroot incluído — `core::fmt` etc.
  aparecem no egui). Para o humano, `--filtrar-stdlib` (escopo `seu-codigo`) é mais
  limpo — mesmo sinal que o MCP deu no 0070. (Já suportado: a flag é ortogonal; só
  não foi o default.)
- **Raio na tela** e **diff na tela** — as outras projeções (§5: uma de cada vez);
  agora informadas pelo uso: o humano, vendo a DSM, vai querer clicar um módulo e ver
  seu raio. Próxima trilha provável.
- **Nível workspace** (deferido na Fase 1) — para "o projeto inteiro" de um workspace
  multi-crate; precisa do `agregar_por_crate`.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo build --workspace` | passa |
| Suíte normal | **289 passed / 0 failed** (287 + peso L1 + montagem HTML) |
| E2E `#[ignore]` `--html` | passa (grava HTML do `lente_core` com grade + peso) |
| `--json`/`--text` sem `--html` | idênticos (só `peso` novo, aditivo, no JSON) |
| `crystalline-lint .` | **V1=0, V2=0**; V12=1 (`ErroLente`, intencional) — preservado |
| `cargo tree` estrutura/core | **sem deps novas** (peso é contagem stdlib) |
| Vistas geradas | egui 76 KB · tekt-linter · lente_core 13 KB — autocontidas (0 cargas externas) |
| Arena `proto-dsm` | intocada |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Vista DSM de produção (Momento A — a interface do humano; o MCP 0070 é a do agente): `lente --pacote <X> --estrutura --html` gera HTML **autocontido** (SVG+JS+dado embutidos, sem rede/CDN) com a matriz na `ordem` topológica (0035), **peso** de acoplamento (Achado 1 do 0036 fechado — `pesos_modulo_a_modulo` conta as arestas-de-item por par módulo→módulo que `agregar_por_modulo:158` descartava; `DependenciaModulo.peso` aditivo no JSON), blocos de ciclo emoldurados e **fold por prefixo** (Achado 3). L1: peso puro (sem tocar `Aresta`, sem deps novas). L2: `dsm_template.html` (`include_str!`) + `formatar_estrutura_html` (injeta o MESMO JSON do `--json` + pacote/limite); a tela é vista — JS só desenha e dobra (cálculo no L1). Cabeçalho declara pacote/escopo/uses e o limite §3 (do catálogo). L4: flag `--html`/`--saida`, grava e imprime o caminho. **Fase 1**: nível workspace **deferido com desenho** (precisa de `agregar_por_crate` — peça nova; não expandi o escopo). **Fase 3** (2 externos): egui (N=111, ciclo de 85, peso `ui→response`=80 — fold torna navegável) e tekt-linter (N=49, parsers→`parsed_file` peso 33). Fila: escopo `seu-codigo` por default, raio/diff na tela, nível workspace. Aditivo: cálculo/tipos L1/fork/CLI intocados; V1=0, V2=0, V12=1; suíte 289 + E2E novo. Arena `proto-dsm` intocada. | `01_core/estrutura/src/lib.rs` (peso), `04_wiring/src/lib.rs` (preenche peso), `02_shell/cli/src/{saida.rs,args.rs,dsm_template.html}`, `02_shell/catalogo/src/lib.rs` (JSON_PESO/PACOTE/LIMITE, HELP_HTML/SAIDA, DSM_*), `04_wiring/app/src/main.rs` (dispatch + E2E), `00_nucleo/prompts/{estrutura,cli-args,cli-saida}.md` (snapshots), `00_nucleo/lessons/0071-vista_dsm_produto.md` |
