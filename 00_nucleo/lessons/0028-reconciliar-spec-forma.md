# Laudo de Execução — Prompt 0028 (Reconciliar a spec da forma)

**Camada**: L5 (laudo)
**Data**: 2026-06-03
**Prompt executado**: `00_nucleo/prompt/0028-reconciliar-spec-forma.md`
**Estado**: `EXECUTADO` — só documentação; zero código tocado; 143 verdes +
15 ignored (idêntico ao laudo 0027). A spec da forma volta a descrever o
sistema real; o `patch-spec-limite-6.md` deixou de existir como arquivo
solto.

---

## Fase 1 — Estado real reunido

| Fonte | O que confirmei / extraí |
|-------|--------------------------|
| `01_core/src/entities/grafo.rs:188-219` | `No` real tem **12 campos**: `id`, `path`, `name`, `kind`, `modificadores`, `visibility`, `crate_name`, `trait_`, `trait_ref`, `cfg`, `macro_kind`, `is_non_exhaustive`. Doc-comment do `crate_name` reescrito no laudo 0026 — texto reusado. |
| `01_core/src/entities/grafo.rs:127-186` | `Kind` é só tipo base (13 valores); `Modificadores { is_const, is_async, is_unsafe }` separado. O `TryFrom<&str>` despe modificadores de strings como `"const async fn"` e mantém `Fn`. |
| `01_core/src/entities/grafo.rs:221-230` | `Aresta` tem `id_from`/`id_to` canônicos + `from`/`to` legíveis. |
| ADR-0002 D3 | Marca de stdlib **pelo prefixo do path** — preservada (vindicada pelo dado do laudo 0025). |
| ADR-0004 | Resolução por cascata: `lente_investiga` (veredito) + `lente_resolve` (aplicar). Convenção `Tipo::<Trait>::metodo`. |
| ADR-0005 | Validação contra typst: 97,4% das 384 colisões resolvidas pela E1. E2 vira nomeação, não decisão. Limite 6 documentado (Ajuste 5). |
| Laudo 0006 | Identidade por `id`; troca de invariante "path único" por "id único". `lente_core` passa a ser processável após a mudança. |
| Laudo 0013 D1 | Fork 0.27.0 **não** emite `crate` por nó. `No.crate_name` populado com o crate-raiz; igual para todos os nós. |
| Laudo 0025 (Fase 1) | Verificação contra `lente_core`: sobreposição "path em prefixo sysroot ∧ trait/trait_ref preenchido" = 0; D3 vindicada. |
| Laudo 0026 | Doc-comment do `No.crate_name` corrigido — texto reusado verbatim na spec. |
| Laudo 0027 (Fase 1) | Mesma verificação no `egui` (3694 nós): sobreposição = 0. Cláusula híbrida arquivada. |
| `patch-spec-limite-6.md` | Texto do Limite 6 + adição à nota de evolução. Aplicado e removido. |

---

## Deltas aplicados na spec

### 1. Metadado + validações empíricas atualizadas

| Antes | Depois |
|-------|--------|
| Estado `PROPOSTO` | Estado `CONSTRUÍDO` (os três derivados existem) |
| Validação só com `typst_syntax` e crate-amostra | Adicionadas medições: `lente_core` (108/278), `egui` (3694/13937), typst (97,4% colisões resolvidas) |
| Sem ponteiros para ADRs aplicáveis | Lista: ADR-0002/0004/0005/0006 |

**Fonte**: leitura dos laudos 0021/0025/0027 e dos ADRs.

### 2. Nova seção "Duas formas: crua e resolvida"

Introduz a distinção que a versão original (2026-05-27) não tinha. A spec
agora diz explicitamente:

- **Forma crua** — saída do `lente_infra`; identidade por `id`; `path`
  pode colidir.
- **Forma resolvida** — saída da cascata `investiga → resolve`; `path`
  único de novo.
- Unicidade de path "não some; muda de lugar".

**Fontes**: laudo 0006 (id) + ADRs 0004/0005.

### 3. Estrutura do nó reescrita: JSON-exemplo + tabela com ~12 campos

| Mudança | Fonte |
|---------|-------|
| JSON-exemplo passou de 4 campos para todos os 12 do descritor + `id` | grafo.rs |
| Tabela do nó separa: campos da forma crua + descritor semântico | laudos 0012/0013 |
| Lista de `kind` enxuta (13 tipos base; sem `const fn`/`async fn`/`unsafe fn`) com nota sobre `Modificadores` | grafo.rs:127-186 |
| Subseção "Campo de Cortesia: `crate_name`" diz **não distingue stdlib** | laudo 0026 (verbatim) |

### 4. Tabela da aresta reescrita: `id_from`/`id_to` canônicos

| Mudança | Fonte |
|---------|-------|
| `id_from`/`id_to` apresentados como referência canônica | laudo 0006 |
| `from`/`to` rebaixados a "texto legível pareado" | grafo.rs:223-230 |

### 5. Invariantes reorganizados

| Antes | Depois |
|-------|--------|
| 5 invariantes; `path` único era #1 | 6 invariantes; **#1 = id único** (comum) + **#6 = path único (apenas forma resolvida)** |
| (sem subdivisão) | "Comuns às duas formas" vs "adicional da forma resolvida" |

**Fontes**: laudo 0006 + ADR-0004.

### 6. Nova subseção "Camada de Resolução"

Descreve `lente_investiga` (E1=vizinhança, E2=código-fonte; veredito) e
`lente_resolve` (aplicar; convenção `Tipo::<Trait>::metodo`). Cita o
97,4%/2,6% do ADR-0005 e conecta os 2,6% ao Limite 6.

**Fontes**: ADR-0004, ADR-0005, ADR-0006.

### 7. Limite 2 — verificação empírica adicionada

A versão original alertava do risco do filtro ingênuo. Adicionei o **achado
verificado** dos laudos 0025/0027: sobreposição zero no `lente_core` (108 nós)
e no `egui` (3694 nós); filtro por prefixo **seguro por construção** neste
fork. Medições do `lente_core` (15,7% sysroot, 35,3% das arestas removidas)
e `egui` (1,6% sysroot, domina ranking pelo montante) ancoradas.

**Fontes**: laudos 0025 e 0027.

### 8. Limite 6 aplicado

Texto do patch transcrito (com ajuste leve de estilo para casar com 1–5).
Magnitude 10/384 (2,6%) em `typst_macros` preservada.

**Fonte**: `patch-spec-limite-6.md` + ADR-0005 Ajuste 5.

### 9. Nota de Evolução ampliada com o parágrafo sobre Limite 6

Sub-seção nova ("O Limite 6 também é família das ambiguidades")
conecta o caminho-de-evolução (fork identificar nós macro-gerados) ao
mesmo padrão da identidade-por-`id`.

**Fonte**: parágrafo opcional do patch.

### 10. Critérios de Verificação — duas seções

| Antes | Depois |
|-------|--------|
| Critérios assumiam unicidade de path no JSON | Critérios **crua** (id único, listas fechadas, etc.) + critérios **resolvida** (path único após cascata; Limite 6 mantém ids colidentes) |
| Casos de borda básicos | Adicionados: colisão verdadeira (`MesmoItem` une ids) e colisão não resolvível (Limite 6) |

### 11. Resultado Esperado — feito vs derivar

| Antes | Depois |
|-------|--------|
| "Dela derivam, em momentos posteriores: tipo / adaptador / filtro" | "Os três derivados estão **construídos**" com ponteiros para crates e prompts |
| Sem menção a componentes adjacentes | Lista `investiga`/`resolve`/`raio`/`ranking`/`wiring`/`cli`/`catalogo` como "fora desta forma" |

### 12. Histórico — nova linha datada com motivo

Linha adicionada referenciando todos os deltas e o `patch-spec-limite-6.md`
retirado.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` | **143 verdes + 15 ignored** — idêntico ao laudo 0027 |
| Mudança de código/tipo/teste | **zero** — só documentação |
| `patch-spec-limite-6.md` | **retirado** via `git rm` |
| Subprocessos do cargo (invariante 0023) | dois únicos, intocados |
| Pureza do L1 | intacta |
| Estrutura/estilo da spec | preservados (mesmas seções, mesmo tom, casos de borda preservados, "Nota de Evolução" ampliada não substituída) |

---

## Decisões tácitas

### D1 — `crate_name` documentado como "campo de cortesia"

Em vez de listar `crate_name` na tabela do JSON do nó (o que sugeriria que o
fork emite o campo, premissa falsa), fiz subseção própria
("Campo de Cortesia: `crate_name`"). Isso:

- Distingue **campo do tipo Rust** (sim, existe) de **campo do JSON** (não,
  o fork não emite).
- Reusa o texto preciso do laudo 0026 (verbatim na essência).
- Deixa rastreável que o nome `crate_name` no código é resíduo da
  premissa original do ADR-0002 (D3 explicava por que **não** armazenar,
  então o nome foi mantido por motivo histórico).

O prompt explicitamente excluiu "remover o campo `crate_name`" do escopo —
respeitado.

### D2 — "kind" sem modificadores: tabela + nota, não duas listas

A versão original tinha **uma** lista de `kind` que misturava modificadores
(`const fn`, `unsafe trait`, etc.). O código tem dois conceitos (`Kind` e
`Modificadores`). Optei por:

- Tabela de campos do nó lista `kind` (tipo base) **e** os booleanos
  separadamente.
- Lista fechada de `kind` enxuta (13 valores).
- Bloco-de-aviso explicando a separação e por que o `TryFrom` despe
  modificadores (laudo 0013).

Alternativa rejeitada: duas listas paralelas. Mais barulho, mesma informação.

### D3 — Camada de Resolução é seção própria, não anexo do Limite 6

Coloquei "Camada de Resolução" **antes** dos Limites (logo após Invariantes),
porque é estrutural — descreve **como** a forma resolvida nasce da crua, e
condiciona como ler o resto da spec. Os Limites 4/5/6 referenciam-na.

O Limite 6 fica como item de lista de Limites (consistente com 1–5).

### D4 — "Resultado Esperado" marcado como CONSTRUÍDO, com ponteiros

Em vez de reescrever a seção, mantive o título original ("Resultado
Esperado") e mudei só o que segue:

- Antes: "Dela derivam, em momentos posteriores: tipo / adaptador / filtro
  (cada um nasce de prompt próprio)."
- Depois: "Os três derivados estão **construídos**:" + bullets com nome do
  crate e prompt(s) que os criaram/ampliaram.
- Bloco adicional listando os 6 componentes que existem **fora desta
  forma** (`investiga`/`resolve`/`raio`/`ranking`/`wiring`/`cli`).

O `Estado` no topo da spec virou `CONSTRUÍDO` em consequência.

### D5 — Patch removido via `git rm`, não `rm`

Para que o git veja a deleção explícita (a remoção do `patch-spec-limite-6.md`
é parte do que este prompt entrega). Trabalho de stage; commit fica para o
usuário.

### D6 — Nenhuma mudança nos ADRs ou nos prompts pré-existentes

O prompt 0028 diz "Reescrever os ADRs: a spec aponta para eles; não os
reescreve." Cumprido. Os ADRs já refletem a realidade nos pontos relevantes
(0002 D3 prefixo do path, 0004/0005 resolução); a divergência era **só na
spec**. Esse foi o reparo cirúrgico.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0028 |
|-----------|-----------------|
| Spec "contrato central" divergiu do sistema | **Coberta** — reconciliada com ponteiros para ADRs/laudos. |
| `patch-spec-limite-6.md` solto | **Coberto** — aplicado e removido. |
| Comentário enganoso do `crate_name` (laudo 0026) | **Já estava coberto**; spec agora reflete o texto. |
| Remover o campo `No.crate_name` | **Aberta deliberadamente** — fora do escopo; ripple largo, ganho pequeno. |
| Filtro de "folhas comportamentais" (Limite 3) | **Aberta** — outra pendência. |
| Remoção da E2 (quarentena) | **Aberta**; spec menciona E2 com seu novo papel (enriquecimento de nomeação) sem assumir remoção. |

---

## Por que este laudo é detalhado

Diferente do laudo 0026 (correção de uma linha de comentário, laudo curto),
o 0028 mexe no **documento de contrato central** — a referência que outros
agentes e laudos miram. Detalhar os deltas e suas fontes deixa **rastreável**
o que mudou, **por quê**, e **de onde** veio cada afirmação atualizada. Se
algum delta for contestado no futuro, a fonte está aqui.

O texto da spec em si segue enxuto: este laudo carrega o detalhe
histórico para que a spec não precise.

---

## O que NÃO mudou (declaração explícita)

- **Código, tipos, testes**: zero toques.
- **ADRs**: zero toques.
- **Prompts anteriores**: zero toques.
- **Outros laudos**: zero toques.
- **Estrutura geral da spec** (seções, tom, Critérios de Verificação,
  casos de borda): preservada — só o conteúdo foi atualizado dentro das
  mesmas seções (+ duas subseções novas).
- **Pureza do L1**: intacta.
- **Subprocessos do cargo** (invariante 0023): dois únicos.
- **Suíte de testes**: 143 verdes + 15 ignored.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-03 | Reconcilia `forma-organizada.md` com o sistema construído: 12 deltas listados acima (identidade por `id`, forma crua vs resolvida, camada de resolução, descritor semântico, `crate_name` como campo de cortesia, Limite 6 do patch, Resultado Esperado como construído). `patch-spec-limite-6.md` retirado. Zero mudança de comportamento. | `00_nucleo/specs/forma-organizada.md`, `00_nucleo/specs/patch-spec-limite-6.md` (retirado), `00_nucleo/lessons/0028-reconciliar-spec-forma.md` |
