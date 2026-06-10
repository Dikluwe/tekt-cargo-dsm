# Laudo de Execução — Prompt 0073 (raio na tela — clicar um módulo e ver o que ele alcança)

**Camada**: L1 (`lente_estrutura`: raio por módulo) + L4 (preencher) + L2 (interação
na vista HTML). O raio por item e os pipelines existentes **não mudam**.
**Data**: 2026-06-10
**Prompt executado**: `00_nucleo/prompt/0073-raio_na_tela.md`
**Estado**: `EXECUTADO` — clicar a diagonal de um módulo na DSM pinta e lista
montante/jusante **transitivos** (semântica exata); validado em egui e tekt-linter.
Suíte **291 passed · 31 ignored** (número exato — disciplina pós-0068); V1/V2=0,
V12=1; sem deps novas.

---

## A resposta em uma sentença

A segunda pergunta da proposta (§2 — alcance da propagação) chega ao humano na mesma
tela da forma: clicado um módulo, a vista mostra **quem depende dele** (montante) e
**do que ele depende** (jusante), transitivos e **exatos** (não a estimativa barata
do grafo agregado).

---

## A semântica — o produto (decidida antes do código)

**Definição 2 (exata)**: alcançabilidade no grafo de **itens** (BFS sobre `Uses`),
projetada a módulos. **Não** o fecho do grafo agregado, que **superestima**: `a∈A→b∈B`,
`b'∈B→c∈C` sem caminho de item `a⇝c` daria `A⇝C` no agregado — a exata não inclui C.
O **teste-contrato** (`raio_exato_nao_superestima_pela_agregacao`) trava isso: se
alguém trocar pela agregada, ele grita.

**Convenção de nomes** — alinhada ao [`Raio`] por item existente (`raio.rs:58`), **não**
ao parêntese do prompt (que veio trocado): **`montante` = quem depende deste (quem
sente** — alcançabilidade reversa); **`jusante` = do que este depende** (direta).
Consistência com `raio_do_alvo` valeu mais que a glosa do prompt; registrado.

---

## Fase 1 — medições (a exata é viável)

1. **Peças reusadas**: `raios_por_modulo` usa o **mesmo** `mapa_modulo_contenedor`
   (item→módulo) da agregação 0031 — não recria a conta.
2. **Custo da exata** no egui (111 módulos, `seu-codigo`): **negligível** — o total
   `--html` foi **8,14 s**, dominado pelo fork (~8 s); o BFS é in-memory, M·(itens+
   arestas) linear. **Sem fallback agregado** — a exata entra sem ressalva.
3. **Tamanho / JSON**: os raios como **índices na `ordem`** (não paths) custam
   **27 KB** no egui (total 97 KB vs 76 KB). **Decisão: embutir só no HTML**, fora do
   `--estrutura --json` — mantém o contrato `--json` byte-estável para os consumidores
   existentes (proto-dsm, agente, testes), e o raio por módulo é concern da tela (o
   agente já tem `raio_do_alvo` por item). Razão registrada.
4. **Fold × raio**: clicar um grupo dobrado = **união** dos raios dos membros — só
   união de conjuntos no JS (apresentação, coerente com a soma de pesos do 0071),
   não cálculo novo.

---

## Fase 2 — construção

- **L1** (`lente_estrutura`): `raios_por_modulo(grafo) -> Vec<RaioModulo>` (BFS
  iterativo à mão, precedente do Kahn 0035; `montante`/`jusante` transitivos, próprio
  módulo excluído, determinístico). `EstruturaModulos.raios` novo. Teste-contrato +
  casos.
- **L4** (`analisar_estrutura`): calcula os raios sobre o **mesmo grafo**
  (mesmo escopo/modo) e anexa — clicar não muda o universo da matriz.
- **L2**: a vista converte os raios para índices na `ordem` (`raios_para_indices`) e
  embute. No template: clicar a **diagonal** de um módulo pinta a linha/coluna —
  **montante** (vermelho, quem depende) e **jusante** (verde, do que depende) — e abre
  um **painel** com contagens, paths e a **semântica declarada** (catálogo:
  `DSM_RAIO_SEMANTICA`) + o limite §3 (estar no raio ≠ vai quebrar). Esc / re-clique /
  ✕ limpam. Grupo dobrado → união, painel marca "(grupo)". **JS só pinta e lista** —
  o cálculo é do L1 (padrão 0029).

---

## Fase 3 — uso real (dois projetos externos)

| Clique | Leitura |
|---|---|
| **egui::id** | montante **81** · jusante **2** (`id_salt`, `viewport`). Hub de base: mexer aqui está no raio de 81 módulos, mas ele quase nada arrasta junto — **alto risco estrutural**, a 1ª pergunta e a 2ª na mesma tela. |
| **egui::ui** (maior peso do 0071) | jusante grande (orquestra meio egui); o raio confirma o que o peso sugeria. |
| **crystalline_lint::entities::parsed_file** | montante **18** — os 4 parsers + contracts + rules + project_index. O hub da camada de entidades do linter; tocá-lo ripple-a 18 módulos. |

A resposta "o que isso alcança" **ajuda a decidir**: o módulo de maior montante é o de
maior risco de mudança; o de maior jusante é o que mais depende do resto. **O que
faltou** (fila): **profundidade por nível** (hoje o raio é um conjunto plano, sem "a
quantos saltos"); **drill por item** dentro do módulo; **diff na tela**.

---

## Verificação

| Item | Resultado |
|------|-----------|
| Teste-contrato (exata não superestima) | passa — `jusante(A) = {B}`, sem C |
| `cargo build --workspace` | passa |
| Suíte normal | **291 passed / 0 failed** |
| Ignorados | **31** (exato: 28 base + mcp-e2e + html + html-completo; 0073 não somou ignorado — raio entrou no E2E `--html` existente) |
| `--text`/`--json` | inalterados (raio é só-HTML; `--json` byte-estável) |
| `crystalline-lint .` | **V1=0, V2=0**; V12=1 (`ErroLente`) — preservado |
| `cargo tree` estrutura/core | **sem deps novas** (BFS à mão, stdlib) |
| Vistas geradas | egui 97 KB · tekt-linter — autocontidas, raio clicável |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Raio na tela (fila do 0071; 2ª pergunta da proposta §2 na mesma vista): clicar a diagonal de um módulo na DSM pinta montante (quem depende — vermelho) e jusante (do que depende — verde) **transitivos** e lista no painel. **Semântica EXATA** (def. 2): `raios_por_modulo` (L1) faz BFS no grafo de **itens** projetado a módulos — não o fecho agregado, que superestima (teste-contrato `a∈A→b∈B, b'∈B→c∈C` sem `a⇝c` → jusante(A) não inclui C). Nomes alinhados ao `Raio` por item (montante=reverso/quem-sente, jusante=direto), **não** ao parêntese trocado do prompt (registrado). Reusa `mapa_modulo_contenedor` (0031). **Fase 1**: exata viável — custo negligível (egui 8,14 s, BFS in-memory; fork domina), sem fallback agregado; raios como **índices na ordem** (27 KB egui) embutidos **só no HTML** (— `--json` byte-estável p/ consumidores; raio é concern da tela, o agente tem `raio_do_alvo`); fold = união (apresentação). **L4**: `analisar_estrutura` anexa os raios (mesmo escopo/modo). **L2**: `raios_para_indices` + template (clique → pinta linha/coluna + painel com contagens/paths/semântica `DSM_RAIO_SEMANTICA`/§3; Esc/✕/re-clique limpam; grupo = união). JS só pinta/lista (padrão 0029). **Fase 3** (2 externos): egui::id (montante 81, jusante 2 — hub de base, alto risco), parsed_file do linter (montante 18). Fila: profundidade por nível, drill por item, diff na tela. Suíte **291 passed / 31 ignored** (nº exato — disciplina 0068); V1=0, V2=0, V12=1; sem deps novas; `--text`/`--json` inalterados. | `01_core/estrutura/src/lib.rs` (RaioModulo + raios_por_modulo + teste), `04_wiring/src/lib.rs` (anexa raios), `02_shell/cli/src/{saida.rs,dsm_template.html}`, `02_shell/catalogo/src/lib.rs` (JSON_RAIOS/RAIO_SEMANTICA, DSM_RAIO_SEMANTICA), `04_wiring/app/src/main.rs` (E2E), `00_nucleo/prompts/estrutura.md` (snapshot), `00_nucleo/lessons/0073-raio_na_tela.md` |
