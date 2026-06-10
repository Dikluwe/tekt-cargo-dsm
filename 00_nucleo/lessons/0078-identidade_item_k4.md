# Laudo de Execução — Prompt 0078 (pareamento por identidade de item — chave K4)

**Camada**: L1 (`lente_comparacao`: censo de itens + chave K4 + 4 categorias) + L4
(alimentar nos dois lados) + L2 (vista agregada + JSON completo).
**Data**: 2026-06-10
**Prompt executado**: `00_nucleo/prompt/0078-identidade_item_k4.md` (era
`prompt-identidade-item-k4.md`; renumerado 0078 — a tela lado a lado vira 0079).
**Estado**: `EXECUTADO` — o `--comparar` ganhou o nível de **item** (chave K4); a
rodada typst **reproduz a Arena 0077** e dá a primeira vista real de "o que falta
migrar". Suíte **311 passed / 34 ignored** (exato); linter V1=0, V2=0, V12=1.

---

## A resposta em uma sentença

O `--comparar` passou a parear **itens** (struct/enum/fn/trait…) por uma chave
**independente de path** (K4 = `kind, trait_, pai-tipo::nome`, medida na Arena 0077) —
e no par typst isso transformou "0 pareados por path" em **1474 itens pareados**,
mostrando o movimento real (`Arg` foi de `typst_syntax::ast` para
`typst_core::entities::ast::expr`) e os **8041 itens de `typst_library` que faltam
migrar**.

---

## Fase 1 — fidelidade à Arena (o critério)

As definições de censo e chave foram **transcritas fiéis** do
`lab/medicao-chave-item/src/main.rs` (0077) para o L1 — mesmos kinds de item
(exclui `mod`/`crate`/`builtin`), mesma regra de **pai-tipo** via `Owns` (qualifica
só quando o pai é struct/enum/trait/union — nunca por módulo, que reintroduziria o
path que o 0076 zerou), mesma exclusão de representantes de fantasma. Alterar a
definição invalidaria a medição que justificou a escolha; não foi alterada.

---

## O que mudou

- **L1 `lente_comparacao`**: `comparar_itens(grafo_a, fant_a, grafo_b, fant_b) ->
  ComparacaoItens` (pura) — censo por chave K4, agrupa nos dois lados, categoriza em
  **pareados** (1:1, com os dois paths) / **ambíguos** (>1, com candidatos
  declarados — o produto **não adivinha**) / **sem-par** dos dois lados. Determinístico
  (BTreeMap/paths ordenados). 6 testes-contrato (pai-tipo qualifica, pai-módulo não,
  `trait_` separa, ambíguo, fantasma excluído, determinismo).
- **L4 `comparar`**: `extrair_lado` guarda o grafo filtrado; `comparar` monta os
  conjuntos de fantasma da proveniência e chama `comparar_itens`. Roda igual em
  crate×crate (chave independente de path) e workspace.
- **L2**: texto **agregado** (4 contagens; pareados por kind; **sem-par por crate** —
  "o que falta migrar por área"); JSON **completo** (as listas, insumo da tela e do
  agente). Aditivo — o nível de módulo (0076) intocado.

---

## A rodada typst — o portão de verdade (reproduz a Arena 0077)

`lente --comparar --antes lab/typst-original --depois .` (seu-codigo; cache morno):

| Categoria (K4) | Produto | Arena 0077 | Bate? |
|---|---|---|---|
| **pareados 1:1** | **1474** | 1474 | ✅ exato |
| **ambíguos** | **107 chaves / 380 itens** | 107 / 380 | ✅ exato |
| sem-par antes | **10910 itens** (10128 chaves) | 10128 chaves | ✅ por chave |
| sem-par depois | **1203 itens** (1183 chaves) | 1183 chaves | ✅ por chave |

**Investigação da única divergência (exigida pelo prompt)**: a Arena contou sem-par
por **chave**; o produto conta por **item** (uma chave sem-par com N candidatos = N
itens não pareados — o número honesto de "o que falta"). Conferido:
`{chaves distintas do sem-par do produto} = {10128, 1183} = Arena`. Logo censo e chave
**reproduzem a Arena exatamente** (pareados e ambíguos batem ao item; sem-par bate por
chave); a diferença é só a **unidade de reporte** (item, mais útil que chave). **Não é
bug nem definição infiel.**

### A primeira vista de "o que falta migrar" (sem-par por crate)

| Lado antes (vanilla, sem par no cristalino) | Lado depois (cristalino, novo) |
|---|---|
| **typst_library: 8041** · typst_layout 720 · typst_pdf 500 · typst_html 359 · typst_syntax 267 · typst_utils 192 | typst_core 979 · typst_infra 202 · typst_shell 22 |

`typst_library` é a maior superfície de migração (8041 itens). E o **movimento real**
aparece nos pareados — ex.: `enum Arg`: `typst_syntax::ast::Arg` →
`typst_core::entities::ast::expr::Arg`. O consumidor vê o item mudar de lugar sem o
produto inferir nada.

### Tamanho do JSON (dado para a decisão futura)

O JSON da rodada typst é **~1,67 MB** (as listas completas de ~14k itens). Registrado
para a decisão de paginação/flag, **se** a tela ou o agente sentirem — conserto é
prompt próprio, não deste (sem flag nova aqui).

---

## Verificação

| Item | Resultado |
|------|-----------|
| Portão vs Arena 0077 | **reproduz** (pareados/ambíguos exatos; sem-par exato por chave) |
| `cargo test --workspace` | **311 passed / 0 failed** (305 + 6 item) |
| Ignorados | **34** (exato — sem novo ignorado; item entrou no E2E workspace existente) |
| E2E workspace lente-vs-lente | item sem-par **0** dos dois lados (paridade total) ✓ |
| Retrocompat crate×crate / módulos (0076) | intocado |
| `crystalline-lint .` | **V1=0, V2=0**; V12=1 (`ErroLente`) |
| Rodada typst | símbolo criado e removido; typst repo limpo |

---

## Trilhas adiadas (registradas)

- **Filtro de boilerplate de macro** (`__ComemoCall::*`) — o resíduo de ambiguidade do
  0077; refinamento próprio, com a saída real na mão.
- **Paginação/flag do JSON de item** (1,67 MB) — se o tamanho doer.
- **Resolvedor de colisão / typst-macros** — itens reais do crate ausentes no antes
  (declarado, não consertado).
- **A tela lado a lado** (0079) — lê este JSON; o movimento por item é o que ela pinta.
- **Inferência de "renomeado"** (item que mudou de nome) — fora; a chave é exata.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Nível de item no `--comparar` — censo e chave **K4** `(kind, trait_, pai-tipo::nome)` promovidos **fiéis** da Arena 0077 (decisão do autor com dado). L1 `comparar_itens` (pura): 4 categorias — pareados (1:1, dois paths) / ambíguos (>1, candidatos, sem adivinhar) / sem-par dos dois lados; pai-tipo via `Owns` (nunca por módulo); fantasmas excluídos. L4: `extrair_lado` guarda o grafo filtrado, `comparar` alimenta `comparar_itens` com os conjuntos de fantasma. L2: texto agregado (4 contagens, pareados por kind, sem-par por crate) + JSON completo (insumo da tela). Aditivo (módulos 0076 intocados). **Rodada typst reproduz a Arena 0077**: pareados **1474** e ambíguos **107 chaves/380 itens** exatos; sem-par antes 10910 itens (10128 chaves) / depois 1203 (1183) — a divergência é unidade de reporte (item vs chave: chaves do produto = Arena), não bug. Primeira vista de "o que falta migrar": typst_library 8041 itens sem-par; movimento real visível (`enum Arg`: typst_syntax::ast → typst_core::entities::ast::expr). JSON ~1,67 MB (registrado p/ paginação futura). Suíte 311 / 34 ignored (exato); V1=0, V2=0, V12=1; E2E lente-vs-lente item sem-par 0. Trilhas: filtro de boilerplate de macro, paginação do JSON, tela lado a lado (0079). | `01_core/comparacao/src/lib.rs` (ItemPareado/Ambiguo/SemPar, ComparacaoItens, comparar_itens + 6 testes; Comparacao.itens; comparar_estruturas ganha itens), `04_wiring/src/lib.rs` (LadoExtraido.grafo, comparar alimenta comparar_itens, re-exports), `02_shell/cli/src/saida.rs` + `02_shell/catalogo/src/lib.rs` (vista item + JSON), `04_wiring/app/src/main.rs` (E2E item), `00_nucleo/prompts/{comparacao,wiring,cli-saida}.md` (snapshots), `00_nucleo/prompt/0078-identidade_item_k4.md` (renumerado), `00_nucleo/lessons/0078-identidade_item_k4.md` |
