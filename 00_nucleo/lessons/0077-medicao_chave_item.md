# Laudo de Execução — Prompt 0077 (Arena: discriminância das chaves de identidade de item)

**Camada**: Arena (`lab/medicao-chave-item/`) — medição descartável. **Mede, não
decide.** Zero toques no produto.
**Data**: 2026-06-10
**Prompt executado**: `00_nucleo/prompt/0077-medicao_chave_item.md` (era
`prompt-medicao-chave-item.md`; renumerado 0077 — a tela lado a lado vira 0078).
**Estado**: `EXECUTADO` — programa de Arena mediu K1–K4 sobre o par typst
member-only; portão de sanidade **bate com o 0076**; determinismo confirmado. O
produto não foi tocado (linter segue V12=1; suíte intacta).

---

## A pergunta (sem responder — é dado para o autor)

No nível de **item**, quanta discriminância cada chave candidata tem no par typst —
quantos itens pareariam 1:1, quantos caem em ambíguo, e onde as colisões moram?

---

## Portão de sanidade — bate com o 0076

| Lado | nós pós-filtro | fantasmas | third-party removido |
|---|---|---|---|
| typst-original (antes) | 13392 | **448** | **434** |
| typst-crystalline (depois) | 3026 | **0** | **40** |

0076 esperava fantasmas **448 / 0** e third-party **434 / 40** — **confere**. O censo
da medição é o mesmo da produção; os números transferem. Montagem dos dois grafos
(cache morno): **12,10 s**. **Determinismo**: duas rodadas idênticas (fora a linha de
tempo).

---

## Censo de itens

Kinds de definição (`fn/struct/enum/union/variant/const/static/trait/type/macro`),
excluindo `mod`/`crate` (medido no 0076 — 0 pareados por path) e `builtin`, e os
**representantes de fantasma**:

- typst-original: **12590** itens · representantes de fantasma excluídos: **431**.
- typst-crystalline: **2851** itens · representantes de fantasma excluídos: **0**.

**Lacuna conhecida do lado antes (declarada, não consertada)**: os 448 fantasmas do
vanilla são, na maioria, representantes de `typst_macros::*` — os **itens reais** desse
crate estão **ausentes** porque o `typst-macros` falhou na extração no 0075 (colisão
irresolúvel). O censo de itens do antes está, portanto, sem o conteúdo real de um
crate-membro. Trilha do resolvedor de colisão, à parte.

(Distribuição completa por kind: `lab/medicao-chave-item/relatorio.md`.)

---

## A síntese K1–K4 (o produto da medição)

| Chave | definição | censo a/d | **pareáveis 1:1** | ambíguas (chaves/itens) | sem-par a/d |
|---|---|---|---|---|---|
| **K1** | `(kind, nome)` | 12590/2851 | **415** | 232 / 6991 | 3540/704 |
| **K2** | K1 sem folhas de impl-de-trait | 5979/1583 | **414** | 185 / 1668 | 3443/669 |
| **K3** | `(kind, pai-tipo::nome)` | 12590/2851 | **1456** | 120 / 441 | 9899/1171 |
| **K4** | K3 + `trait_` | 12590/2851 | **1474** | 107 / 380 | 10128/1183 |

**Onde a ambiguidade mora** (top colisões, antes×depois):

- **K1**: dominada pelas folhas de impl-de-trait — `fn|fmt` **796×259**, `fn|clone`
  **734×246**, `fn|hash` **608×152** (a família `Display`/`Debug`/`Clone`/`Hash` do
  laudo 0021). São 6991 itens ambíguos, quase todos aqui.
- **K2** remove essas folhas: censo cai pela metade (12590→5979), ambiguidade
  despenca (185 chaves / 1668 itens) — confirma que fmt/clone/hash eram o grosso do
  ruído. Pareáveis quase iguais a K1 (414 vs 415).
- **K3** qualifica por **pai-tipo** (`Counter::get`): pareáveis **3,5× K1** (1456),
  ambiguidade a 120 chaves / 441 itens. O resíduo é sobretudo **boilerplate de macro**
  — `fn|__ComemoCall::clone/eq/hash` (gerado pelo `comemo`), não item real do usuário.
  Custo: sem-par antes sobe a 9899 (qualificar torna mais chaves únicas).
- **K4** (+`trait_`): marginal sobre K3 (1474 vs 1456 pareáveis; ambiguidade 107/380).
  O trait acrescenta pouco depois da qualificação por pai-tipo.

---

## O que a medição NÃO decide

A **escolha da chave** — fica com o autor, com estes números. As hipóteses do prompt
estão ambas no dado: "K1 basta" é falsa (415 pareáveis, 6991 ambíguos); "só K4
discrimina" também (K3 já dá o salto, K4 acrescenta marginal). O dado aponta um
trade-off (K3 maximiza pareáveis ao custo de sem-par; K2 limpa o censo sem qualificar),
mas a decisão é de produto, não da Arena.

---

## Verificação

| Item | Resultado |
|------|-----------|
| Portão de sanidade vs 0076 | **bate** (448/0 fantasmas; 434/40 third-party) |
| Determinismo | duas rodadas idênticas (fora o tempo) |
| Produto tocado | **nenhum** — Arena em `lab/`; `crystalline-lint .` segue V12=1; suíte intacta |
| Artefatos | `lab/medicao-chave-item/{Cargo.toml,src/main.rs,relatorio.md}` |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Arena de medição (descartável, consome a lente como lib; zero toques no produto) da discriminância de 4 chaves de identidade de item no par typst member-only. Censo idêntico ao da produção (`montar_grafo_workspace`→`filtrar_stdlib`→`filtrar_nao_membros`); raízes por arg de CLI; representantes de fantasma excluídos do censo de itens (431/0). **Portão de sanidade bate com o 0076** (fantasmas 448/0, third-party 434/40). Itens: 12590 (antes) / 2851 (depois). **Síntese**: K1 (kind,nome) 415 pareáveis, 6991 ambíguos (dominado por fmt/clone/hash); K2 (sem folhas de impl-trait) censo 5979, ambiguidade 1668; K3 (pai-tipo::nome) **1456 pareáveis** (3,5×K1), ambiguidade 441 (resíduo = boilerplate `__ComemoCall`); K4 (+trait) 1474, marginal. Lacuna declarada: typst-macros ausente (falha 0075) → itens reais desse crate faltam no antes. Tempo 12,10 s (cache morno); determinismo confirmado. **Sem conclusão** — a escolha da chave fica com o autor. Numeração: este 0077; tela lado a lado → 0078. | `lab/medicao-chave-item/{Cargo.toml,src/main.rs,relatorio.md}`, `00_nucleo/prompt/0077-medicao_chave_item.md` (renumerado), `00_nucleo/lessons/0077-medicao_chave_item.md` |
