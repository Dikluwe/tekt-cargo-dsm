# Laudo de Execução — Prompt 0072 (escopo `seu-codigo` como default das superfícies de consumo)

**Camada**: L2 (default da vista HTML + rótulos) + L4 (default da boca MCP). O
cálculo e o filtro (0025) **não mudam** — muda **qual lado é o default** em duas
superfícies.
**Data**: 2026-06-10
**Prompt executado**: `00_nucleo/prompt/0072-escopo_padrao_consumo.md`
**Estado**: `EXECUTADO` — vista `--html` e MCP `raio_do_alvo`/`ranking` agora
default `seu-codigo`; `completo` a uma flag/parâmetro, sempre declarado. CLI
`--text`/`--json` inalterada. `impacto_do_diff` **deferido com desenho**. Suíte 290
+ ignorados verdes; V1/V2=0, V12=1.

---

## A resposta em uma sentença

Dois consumidores independentes (humano via HTML, agente via MCP) mediram o mesmo
incômodo no primeiro uso (sysroot afogando o sinal — 0070/0071); este prompt inverte
o **default** dessas duas superfícies para `seu-codigo`, sem remover nada: o
`completo` fica a uma flag de distância e a saída sempre declara o escopo ativo.

---

## Fase 1 — leitura (eixos e tubulação)

1. **Os dois eixos, não confundir**: `--filtrar-stdlib` é o **escopo**
   (presença → `SeuCodigo`; `escolher_escopo` deriva antes do dispatch);
   `--so-referencia` é o **modo de uses** (eixo separado — **não tocado**). A
   inversão precisou de **flag nova** para o caminho de volta na vista:
   **`--completo`** (registrada no catálogo: `HELP_COMPLETO`).
2. **Escopo no diff — tubulação NÃO existe → DEFERIDO com desenho.**
   `analisar_diff(raiz)` não tem parâmetro de escopo e `montar_grafo_workspace` não
   passa pelo filtro. Threading exigiria aplicar `filtrar_stdlib` (desenhado para
   **um crate**, 0025) ao **grafo de workspace** (multi-crate, 0045) — território
   não-verificado do Limite 2 (que preserva impls do *crate-alvo*; num workspace há
   vários). É **peça nova**. Conforme o prompt (mesmo padrão do workspace no 0071),
   **deferido**. **Desenho**: `analisar_diff(raiz, escopo)` → após
   `montar_grafo_workspace`, se `SeuCodigo` então `filtrar_stdlib(&grafo)` antes de
   `mapear_diff`/raio; CLI `--diff` passa `Completo` (inalterado), MCP passa
   `SeuCodigo`; **a validar**: Limite 2 sobre grafo multi-crate. A descrição do
   `impacto_do_diff` declara honestamente que hoje é `completo` (ver laudo 0072).
3. **Testes reancorados** (lista da Fase 1): MCP `escopo_default_e_invalido`
   (default → `SeuCodigo`); app E2E `--html` (default → `seu-codigo`, sem sysroot);
   cli `html_estrutura_*` (dica de escopo). Todos **anotados** com a procedência.

---

## Fase 2 — a inversão (por superfície)

| Superfície | Antes | Agora | Caminho de volta |
|---|---|---|---|
| `--estrutura --html` (humano) | `completo` | **`seu-codigo`** | `--completo` |
| MCP `raio_do_alvo`, `ranking` | `completo` | **`seu-codigo`** | `escopo:"completo"` |
| MCP `impacto_do_diff` | `completo` | `completo` (deferido) | — |
| CLI `--text`/`--json` | `completo` | **inalterado** | — |

- **Vista HTML** (`run_estrutura`): `if cli.html { if cli.completo { Completo } else
  { SeuCodigo } }` — só a vista inverte; `--text`/`--json` seguem o escopo recebido.
  O cabeçalho ganha a **dica** (`DSM_ESCOPO_DICA`, catálogo): "escopo: seu-codigo
  (sysroot/stdlib ocultos — use --completo …)".
- **Boca MCP** (`parse_escopo`): `None | "seu-codigo" → SeuCodigo`; `"completo" →
  Completo`. As **descrições** de `raio_do_alvo`/`ranking` declaram "seu-codigo
  (DEFAULT …) | completo" — a honestidade do recorte no contrato do agente (padrão
  0070). `impacto_do_diff` declara que é `completo` (deferido).
- **A saída sempre declara o escopo** (0030 inalterado): JSON `escopo`, cabeçalho da
  vista, payload MCP.

---

## A revisita do 0030 — registrada (procedência)

O 0030 fez o escopo **escolha do usuário com saída rotulada**. Este prompt **não
revoga** isso: a flag existe nos dois sentidos, a saída segue rotulada, e o caminho
de volta é **testado byte-a-byte** (E2E `--completo` traz `core::fmt` de volta). O
que muda é **o lado que vem sem pedir**, em duas superfícies de **consumo**,
justificado por **dois consumidores independentes em uso real**: 0070 (jusante do
diff afogado em sysroot no JSON do agente) e 0071 (matriz do egui poluída por
`core::fmt`). A disciplina é decidir por uso — o uso falou duas vezes. **Esta linha é
a procedência da mudança**; se um uso futuro mostrar que o default novo esconde algo,
o caminho de volta é uma flag, porque nada foi removido.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `--html` sem flag | **seu-codigo**: egui 109 mód (sysroot 0), dica no cabeçalho |
| `--html --completo` | **completo**: egui 111 mód (sysroot 2) — caminho de volta |
| MCP sem `escopo` | `raio_do_alvo`/`ranking` → seu-codigo (default reancorado) |
| MCP `tools/list` | descrições declaram o default e como pedir completo |
| `impacto_do_diff` | deferido — descrição declara `completo`; desenho no laudo |
| CLI `--text`/`--json` | **inalterado** (escopo recebido; contrato/contagens intactos) |
| Suíte normal | **290 passed / 0 failed** (289 + dica seu-codigo) |
| E2E `#[ignore]` | `--html` seu-codigo + `--html --completo` (volta) + MCP — verdes |
| `crystalline-lint .` | **V1=0, V2=0**; V12=1 (`ErroLente`) — preservado |
| Deps | nenhuma nova |

---

## O que resta (fila)

- **`impacto_do_diff` com escopo** — o desenho acima; o passo de risco é validar o
  `filtrar_stdlib` no grafo de workspace (Limite 2 multi-crate). Prompt próprio.
- **Magnitude honesta**: na DSM de *estrutura*, o sysroot some em poucos módulos
  (egui: 2 de 111 — o stdlib é mais usado a nível de item que de módulo). O ganho
  maior do recorte está no **raio por nó** e no **jusante do diff** (onde o 0070
  mediu dezenas de paths de sysroot) — por isso `raio_do_alvo` é quem mais lucra.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Escopo `seu-codigo` como **default das superfícies de consumo** (procedência: 0030 ⊕ evidência 0070/0071 — dois consumidores independentes mediram sysroot afogando o sinal). **Vista `--html`**: default invertido para `seu-codigo` (`run_estrutura`: `if cli.html`); flag nova **`--completo`** restaura (caminho de volta testado byte-a-byte: traz `core::fmt`); cabeçalho ganha dica (`DSM_ESCOPO_DICA`). **MCP `raio_do_alvo`/`ranking`**: `parse_escopo` default → `SeuCodigo` (`None|"seu-codigo"`); descrições declaram o default e como pedir `completo`. **`impacto_do_diff` DEFERIDO** com desenho: `analisar_diff` não tem tubulação de escopo (`montar_grafo_workspace` não filtra) e aplicar `filtrar_stdlib` (single-crate, 0025) ao grafo de workspace (0045) é território não-verificado do Limite 2 → peça nova; descrição declara `completo` por ora. **CLI `--text`/`--json` inalterada** (contrato/contagens ancoradas). 0030 revisitado com registro — a escolha do usuário permanece; só o default de consumo muda; saída sempre declara o escopo. Testes reancorados e anotados (MCP escopo default, app E2E `--html` seu-codigo + `--completo`, cli dica). Sem deps/motor/filtro novos; modo de uses fora do escopo. Suíte 290 + ignorados verdes; V1=0, V2=0, V12=1. | `02_shell/cli/src/{args.rs,saida.rs,dsm_template.html}`, `02_shell/catalogo/src/lib.rs` (HELP_COMPLETO, DSM_ESCOPO_DICA, JSON_ESCOPO_DICA), `04_wiring/app/src/main.rs` (override + E2E), `04_wiring/mcp/src/main.rs` (parse_escopo + descrições + teste), `00_nucleo/prompts/cli-args.md` (snapshot), `00_nucleo/lessons/0072-escopo_padrao_consumo.md` |
