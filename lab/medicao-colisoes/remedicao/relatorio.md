# Remedição de Colisões de Path — Relatório Consolidado (3 medições)

**Tipo**: Experimento de Arena (`lab/`)
**Prompts**: 0005 (1ª medição), 0007 (2ª medição), 0009 (3ª medição — re-execução pós-correção)
**Comparação base**: `lab/medicao-colisoes/relatorio.md` (1ª medição, fork antigo sem id)
**Data**: 2026-05-27
**Escopo**: 17 crates do typst v0.14.2

---

## TL;DR

| | 1ª medição (fork antigo) | 2ª medição (fork novo, bug) | **3ª medição (chave corrigida)** |
|---|---:|---:|---:|
| **Total decidido** | 14.3% | 36.2% | **97.4%** |
| E1 (vizinhança) | 0 (inaplicável) | 30.2% (contaminado) | **97.4%** |
| E2 (fontes) | 14.3% | 6.0% | **0** |
| NaoDeterminado | 85.7% | 63.8% | **2.6%** |

A pergunta-pivot — *"a identidade-por-nó, com a chave corrigida, faz a E1
resolver a maioria das colisões?"* — tem resposta empírica clara: **sim,
quase tudo.** 97.4% decidido pela E1 sozinha, 100% dos vereditos da E1
são `Distintos/VizinhancaDisjunta`, todos os 31 casos `::fmt` (Display+Debug)
hoje classificados corretamente.

Os 10 restantes (2.6%) são todos `typst_macros::util::kw::<nome>` — código
gerado por macro com vizinhança em sobreposição parcial.

---

## Seção 1 — Comparação tripla (agregados)

| Métrica | 1ª (fork antigo) | 2ª (fork novo, bug) | **3ª (corrigida)** | Δ 2ª→3ª |
|---------|---:|---:|---:|---:|
| Total de colisões | 384 | 384 | **384** | (idem) |
| Decididas por E1 | 0 | 116 (30.2%) | **374 (97.4%)** | **+258** |
| Decididas por E2 | 55 (14.3%) | 23 (6.0%) | **0** | −23 |
| NaoDeterminado | 329 (85.7%) | 245 (63.8%) | **10 (2.6%)** | **−235** |
| **Total decidido** | 55 (14.3%) | 139 (36.2%) | **374 (97.4%)** | **+235 (61.2 pp)** |

A correção do `ChaveAresta` (laudo 0008 — incluir `id_from`/`id_to`) sozinha
fez a cobertura saltar 61pp. Não é refinamento incremental — é uma mudança
qualitativa do que a E1 consegue.

---

## Seção 2 — As três medições, em uma frase cada

1. **1ª medição** (prompt 0005): fork antigo, JSON referenciando arestas
   por path; a E1 não tinha como separar vizinhança de cópias colidentes,
   ficou estruturalmente inaplicável. Cobertura 14.3% (toda E2).
2. **2ª medição** (prompt 0007): fork novo com `id`/`id_from`/`id_to`, mas
   `ChaveAresta` ainda comparando por path (bug do laudo 0008 §6) — E1
   aparente saltou para 30.2%, mas 98% dos vereditos era `MesmoItem` por
   falsa-coincidência de chave. **Falsa decisão.**
3. **3ª medição** (este prompt): `ChaveAresta` corrigida para
   `(id_from, id_to, relation)` (laudo 0008). E1 sobe a 97.4%, 100% como
   `Distintos/VizinhancaDisjunta`.

---

## Seção 3 — Resultados por crate (3ª medição)

| Crate | Colisões | E1 | E2 | NaoDet |
|-------|---:|---:|---:|---:|
| typst (lib) | 10 | 10 | 0 | 0 |
| typst (cli) | 0 | 0 | 0 | 0 |
| typst_bundle | 0 | 0 | 0 | 0 |
| typst_eval | 0 | 0 | 0 | 0 |
| typst_html | 7 | 7 | 0 | 0 |
| typst_ide | 2 | 2 | 0 | 0 |
| typst_kit | 1 | 1 | 0 | 0 |
| typst_layout | 4 | 4 | 0 | 0 |
| **typst_library** | **316** | **316** | 0 | **0** |
| typst_macros | 16 | 6 | 0 | **10** |
| typst_pdf | 0 | 0 | 0 | 0 |
| typst_realize | 0 | 0 | 0 | 0 |
| typst_render | 0 | 0 | 0 | 0 |
| typst_svg | 8 | 8 | 0 | 0 |
| typst_syntax | 6 | 6 | 0 | 0 |
| typst_timing | 0 | 0 | 0 | 0 |
| typst_utils | 14 | 14 | 0 | 0 |
| **Total** | **384** | **374 (97.4%)** | **0** | **10 (2.6%)** |

**16 dos 17 crates** ficam **100% decididos pela E1** (incluindo
`typst_library` com suas 316 colisões — antes 217 NaoDet, agora 0). Único
remanescente: `typst_macros` com 10 colisões pendentes.

---

## Seção 4 — Distribuição dos vereditos E1

Distribuição dos 374 vereditos E1 da 3ª medição:

- `Distintos/VizinhancaDisjunta`: **374 (100%)**
- `MesmoItem`: **0**

Comparado com a 2ª medição (com bug):

- 2ª: `MesmoItem` 114 (98.3%) / `Distintos` 2 (1.7%)
- 3ª: `MesmoItem` 0 / `Distintos` 374 (100%)

A inversão total confirma o diagnóstico do laudo 0008: na 2ª medição, a
chave por path **colapsava arestas de cópias distintas**, produzindo a
ilusão de "vizinhança idêntica" (veredito `MesmoItem`). Com a chave por
id, as arestas viram realmente distintas, e os casos `Display+Debug` (que
nunca foram "mesmo item") aparecem corretamente como `Distintos`.

Observação adicional: os **31 casos terminados em `::fmt`** que a 2ª medição
apontou como suspeitos de falsa classificação (Display+Debug rotulados como
MesmoItem) agora aparecem **todos os 31 como `Distintos/VizinhancaDisjunta`**.
A correção fez exatamente o que devia.

Sobre `MesmoItem == 0`: o critério `MesmoItem` (`exclusivas_a == 0 AND
exclusivas_b == 0 AND compartilhadas > 0`) não dispara em nenhum dos 384
casos. Isso é compatível com o fato de que **toda colisão real** vem com
arestas separadas por id no JSON novo — vizinhança nunca é genuinamente
idêntica em dados que vêm do fork, mesmo para coisas que conceitualmente
poderiam ser "o mesmo item" (reexports, alias). Cada cópia tem suas
próprias arestas `Owns`/`Uses` indexadas por id.

---

## Seção 5 — Os 10 NaoDet restantes (todos do `typst_macros`)

Todos os 10 são do mesmo padrão: `typst_macros::util::kw::<nome>` (n=2 cada),
onde `<nome>` ∈ {`span`, `name`, `constructor`, `title`, `ext`, `contextual`,
`keywords`, `scope`, `cast`, `parent`}.

Vizinhança idêntica entre eles: **exclusivas_a=18, exclusivas_b=1,
compartilhadas=1**. Mesma anatomia em todos.

Por que a E1 não decide: o critério exige `compartilhadas == 0`; aqui há 1
compartilhada (provavelmente `Owns` do módulo-pai `util::kw`). Por que a E2
não decide: `kw` é **módulo**, não tipo — não existe `impl <Trait> for kw`
em parte alguma. O parser textual procura `impl X for kw` e acha zero (é o
caso normal de E2 falhar contra macro-gerado).

Para resolver os 10, dois caminhos:

- **Relaxar o critério da E1** (seção 6) — automaticamente decidiria os 10
  como `Distintos`.
- **Aceitar como Limite** — código gerado por macros que produz nomes
  colidentes no mesmo módulo é caso raro mas reconhecidamente fora do
  alcance.

---

## Seção 6 — Estimativa do critério relaxado (atualizada)

A 2ª medição estimou que relaxar para "ambos com exclusivas, independente
de compartilhadas" levaria E1 de 30% para ~75%. **Aquela estimativa estava
inflada pelo bug** (114 falsos `MesmoItem` já contavam como "decididos").

Re-estimativa **com a chave corrigida**:

| NaoDet com `exc_a > 0` E `exc_b > 0` | 10 (todos os 10) |
| Outros NaoDet (um lado zero exclusivas) | 0 |

Logo, sob critério relaxado **a partir do estado atual**, todos os 10
NaoDet passariam a `Distintos/VizinhancaDisjunta`. **E1 chegaria a 100%**.

Isso significa que, na prática, relaxar o critério é **suficiente para
decidir tudo nos 17 crates do typst**. Mas é decisão sobre o que faz
sentido conceitualmente, não só estatisticamente — `compartilhadas > 0`
ainda pode ser sinal genuíno em casos não-medidos (reexports, aliases,
contextos onde duas cópias compartilham um caminho de uso). A decisão
fica para depois.

---

## Seção 7 — Re-avaliação das hipóteses (final)

### H1 — E1 decide a maioria

→ **Confirmada com folga.** Esperado 50-90%; medido **97.4%**. A
identidade-por-nó funciona melhor do que a hipótese previa.

### H2 — E2 ainda decide alguns que E1 não decide

→ **Refutada nesta medição.** E2 = 0. Todos os casos que a E2 decidia na
1ª medição (`From<Abs>+From<Em>`, `Add+Add<f64>`, etc.) agora são decididos
pela E1 — porque cada `impl Trait<X> for T` produz `fn add` com id distinto
e vizinhanças disjuntas no fork novo.

Caveat: H2 foi refutada **contra os 17 crates do typst**. Não está provado
que E2 nunca decida nada em outros crates. Pode haver cenários (reexports
que compartilham vizinhança) onde a E2 ainda seja útil.

### H3 — NaoDet cai drasticamente

→ **Confirmada.** 85.7% → 2.6%. Queda de 83.1pp. Difícil descrever isso
como algo que não seja "drástico".

---

## Seção 8 — Avaliação contra os três cenários do ADR-0004

> **Cenário A**: E1 resolve a maioria; E2 fica como fallback raro.

→ **Confirmado.** E1 resolve 97.4%; E2 só seria invocada nos 2.6% NaoDet
(e nesses casos atuais, a E2 também não decide — são macros).

> **Cenário B**: maioria exige E2; cascata vira otimização menor.

→ **Refutado.** E2 não decidiu nada na 3ª medição.

> **Cenário C**: E2 raramente decide; arquitetura não cumpre função.

→ **Não realizado.** A função se cumpre pela E1, não pela E2.

**O ADR-0004 se sustenta com folga**, pelo lado da E1. O papel da E2 sob
o novo regime é menor do que o ADR antecipava — vale uma nota futura no
ADR ("E2 fica como fallback teórico; pode-se removê-la se a evidência
contra crates de outras origens confirmar o padrão"), mas a arquitetura
fundamental (cascata vizinhança → fontes) está validada.

---

## Seção 9 — Sugestões para a continuidade (não prescritivas)

1. **Construir o `lente_resolve`** — agora há evidência concreta de que vale
   a pena: 97.4% dos casos têm veredito acionável. A convenção de nomeação
   `Tipo::<Trait>::método` declarada no ADR-0004 §3 pode ser implementada;
   `lente_investiga` já fornece os `traits` quando decide via E2 (mesmo que
   E2 esteja inativa nesta amostra, a estrutura existe).

   Para os 374 casos `Distintos/VizinhancaDisjunta` (E1), a evidência é
   topológica — `VizinhancaDisjunta { exclusivas_a, exclusivas_b }`. O
   `lente_resolve` precisa de uma convenção de nomeação para esses casos
   (não pode inventar trait do nada). Sugestão para discussão: usar
   contador (`Tipo::método#1`, `Tipo::método#2`) ou primeiro vizinho
   distintivo. Decisão de design separada.

2. **Decidir sobre relaxar o critério** (seção 6) — empiricamente fecharia
   100% nesta amostra. Mas o critério atual está conceitualmente bem
   fundado ("zero compartilhadas, ambos com exclusivas é o padrão claro"),
   e os 10 NaoDet hoje são reconhecidamente macros (Limite). Talvez não
   relaxar mas aceitar como Limite seja melhor caminho.

3. **A E2 mantém ou removida?** Conservador: manter. Sensato porque (a)
   custo de manutenção é baixo, (b) pode decidir cenários não-medidos, (c)
   a remoção é trivial se a evidência adicional confirmar irrelevância.

4. **ADR-0004**: sustenta-se. Vale uma nota apontando para este relatório
   ("E1 cobre 97.4% nos 17 crates do typst; E2 inativa contra este
   workspace"), mas a estrutura fica.

5. **Medir contra crates de outras origens** — para validar generalização.
   Esta medição é típica do "Cenário B" do prompt 0005 (crates grandes e
   complexos); falta a categoria 1 (pequenos idiomáticos externos) e 3
   (bibliotecas de produção). Particularmente interessante: crates que usam
   muito reexport (`hyper`, `tokio`) podem revelar casos onde
   `MesmoItem` finalmente apareceria.

---

## Seção 10 — Limites declarados desta medição

- **17 crates, todos do typst.** Mesma limitação das medições anteriores.
  Generalização contra crates de outras origens ainda não testada.
- **Primeiro par só** por colisão (consistência). Colisões com 3+ cópias
  têm pares adicionais não investigados — mas como os primeiros pares dão
  100% `Distintos`, é razoável supor que os outros também (mesmas razões
  estruturais).
- **Critério relaxado projetado, não implementado.** O número "100% se
  relaxar" é contagem direta sobre os diagnósticos atuais.
- **JSONs reutilizados da 2ª medição** (mesmo fork `5fbcdfe8`); só a
  análise rodou de novo. Verificado: comportamento determinístico do
  `lente_investiga` corrigido.

---

## Seção 11 — Artefatos do experimento

- `lab/medicao-colisoes/remedicao/Cargo.toml`, `src/main.rs` — programa
  inalterado.
- `lab/medicao-colisoes/remedicao/json/*.json` — 17 JSONs do fork novo
  (gerados na 2ª medição, reutilizados aqui).
- `lab/medicao-colisoes/remedicao/analise.json` — resultado da 3ª medição
  (atual).
- `lab/medicao-colisoes/remedicao/analise-com-bug.json` — resultado da 2ª
  medição (preservado para comparação direta).
- `lab/medicao-colisoes/remedicao/relatorio.md` — este documento
  (consolidado das três medições).

---

## Histórico

| Data | Motivo |
|------|--------|
| 2026-05-27 (1ª) | Primeira medição com fork antigo. E1 inaplicável; cobertura 14.3% toda via E2. |
| 2026-05-27 (2ª) | Remedição com fork novo. E1 nominal saltou a 30.2% mas contaminada por bug em `ChaveAresta`. Descoberta da §6 motivou laudo 0008. |
| 2026-05-27 (3ª) | Re-execução pós-laudo 0008 (chave corrigida). E1 sobe a 97.4%; ADR-0004 confirmado. Os 10 NaoDet restantes são todos `typst_macros::util::kw::*` (código gerado). Estimativa: relaxar critério levaria a 100% nesta amostra. |
