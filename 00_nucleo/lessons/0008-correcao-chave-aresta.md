# Laudo de Execução — Prompt 0008 (Correção de ChaveAresta em lente_investiga)

**Camada**: L5 (laudo)
**Data**: 2026-05-27
**Prompt executado**: `00_nucleo/prompt/0008-correcao_chave_aresta.md`
**Origem do bug**: `lab/medicao-colisoes/remedicao/relatorio.md` §6 (Descoberta crítica)
**Pré-requisito**: estado pós-laudo 0006
**Estado**: `EXECUTADO` — 57 testes verdes (+ 2 ignored), pureza preservada,
salvaguarda contra regressão adicionada.

---

## O que o prompt pediu

Corrigir o `ChaveAresta` em `05_investiga/src/vizinhanca.rs` para usar
`(id_from, id_to, relation)` em vez de `(from, to, relation)`. Razão: a
remedição (§6) descobriu que a chave por paths colapsa arestas que apontam
para cópias distintas do mesmo path (caso clássico `Display+Debug`),
falsificando o veredito da E1 como `MesmoItem`.

**Não tocar** o critério categórico de "disjuntas/idênticas/inconclusivo" —
isso é decisão de design separada que fica para depois.

---

## O que foi alterado

| Arquivo | Mudança |
|---------|---------|
| `05_investiga/src/vizinhanca.rs` | `ChaveAresta` agora tem `id_from: usize, id_to: usize, relation: Relation`. `ChaveAresta::de(&Aresta)` passa os ids. **Tipo agora é `Copy`** (era só `Clone`) porque é 100% trivial. |
| `05_investiga/src/vizinhanca.rs` (testes) | Teste novo: `vizinhancas_de_copias_distintas_decidem_distintos` — salvaguarda contra regressão deste bug específico. |

**Não tocados** (escopo declarado do prompt):

- Critério categórico (`disjuntas/idênticas/inconclusivo`) — `compartilhadas
  == 0` etc. permanece.
- Tipos públicos do `lente_investiga` — interface igual.
- `fontes.rs`, `lib.rs` — não afetados.
- `lente_core`, `lente_infra` — não tocados.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo build` (workspace) | limpo |
| `cargo test -p lente_investiga` | **17/17** verdes (16 originais + 1 novo) |
| `cargo test` (workspace) | **57/57 verdes + 2 ignored** (lente_core 26, lente_infra 14+2 ignored, lente_investiga 17) |
| `cargo tree -p lente_core` | só o crate (pureza preservada) |
| `cargo tree -p lente_investiga` | só `lente_core` (pureza preservada) |

---

## Decisões tácitas

### D1 — `ChaveAresta` virou `Copy`

Antes: `Clone` (campos `String`, sem `Copy`). Agora: três `usize` + `Relation`
(que já é `Copy`). Pequena melhoria colateral — menos clones, menos
alocações, código mais limpo.

### D2 — Teste novo constrói literal `Aresta` (não usa o helper)

O helper `aresta(from, to, relation)` deriva `id_to` via hash determinístico
do path `to`. Para o cenário deste teste preciso que **dois nós distintos
recebam arestas com mesmo `to` mas `id_to` diferentes** — exatamente o que
o helper impede por construção. Solução: construir o `Aresta` por literal
no corpo do teste, com `id_to: 100` para a cópia A e `id_to: 101` para a
cópia B.

Mantenho o helper como está (ele continua útil para os outros testes onde
paths são únicos), só este teste novo escapa dele.

### D3 — Nome do teste segue sugestão do prompt

`vizinhancas_de_copias_distintas_decidem_distintos`. O nome explica o
cenário sem precisar ler o corpo — atende ao princípio de "teste como
documentação".

### D4 — Sem descobertas adicionais

A execução foi puramente mecânica — não emergiu nada que justifique nota
extra. O teste novo passa com a chave corrigida e **falharia** com a chave
antiga (verificável mentalmente: `ChaveAresta { from: "X", to: "X::fmt",
relation: Owns }` colidiria entre as duas arestas; com a chave nova,
`{ id_from: 42, id_to: 100, ... }` ≠ `{ id_from: 42, id_to: 101, ... }`).

---

## Por que esse teste é a salvaguarda certa

O bug descoberto pela remedição era estruturalmente:

> Os 16 testes do `lente_investiga` continuam passando porque nenhum deles
> exercita o caso de dois nós com mesmo path e ids distintos.

(Citação do prompt 0008 §Contexto.)

O teste novo **exatamente** exercita esse caso: mesmo `from` (path "X"),
mesmo `to` (path "X::fmt"), mesmo `relation` (Owns), **diferindo só por
`id_to`** (100 e 101). É o cenário mínimo que distingue a chave correta da
errada.

Com `ChaveAresta` antiga, as duas arestas ficavam idênticas, `compartilhadas
> 0`, vizinhanças "idênticas", veredito `MesmoItem`. Com a nova, viram
chaves distintas, vizinhanças disjuntas, veredito `Distintos`.

A propriedade que o teste protege é precisa e localizada — exatamente o
que uma salvaguarda contra regressão deve fazer.

---

## O que vem depois

Conforme o prompt declara, ficam para prompts futuros:

1. **Re-rodar a remedição** com a chave corrigida. Esperado: a E1 deixa de
   classificar `Display+Debug` como `MesmoItem`; a contagem real da E1
   passa a refletir o que ela consegue de fato.
2. **Considerar relaxar o critério** (`compartilhadas == 0` ainda exigido,
   ou substituir por "ambos com exclusivas"). Decisão dependente do que a
   remedição corrigida mostrar.
3. **Atualizar ou supersedar o ADR-0004** com os novos números.
4. **Construir `lente_resolve`** apenas depois de 1-3 acima.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Correção do bug §6 da remedição. `ChaveAresta` passa a usar `(id_from, id_to, relation)`. Teste novo `vizinhancas_de_copias_distintas_decidem_distintos` protege contra regressão. 57/57 verdes (+1 vs. estado anterior). | `05_investiga/src/vizinhanca.rs` |
