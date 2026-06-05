# Prompt: Reconciliar a spec `forma-organizada.md` com o sistema construído

**Camada**: L0 — Semente (especificação)
**Criado em**: 2026-06-02
**Estado**: `PROPOSTO`
**Decisões de origem**: auditoria do estado do projeto — a spec
`00_nucleo/specs/forma-organizada.md` (última revisão 2026-05-27) **divergiu do
sistema**. Ela afirma identidade por `path` (Invariante 1) e descreve nós com 4
campos, mas o sistema usa identidade por `id` (laudo 0006), tem ~12 campos por
nó (descritor semântico, laudos 0012/0013), e tem uma camada inteira de
resolução de colisões (ADR-0004/0005) que a spec não menciona. O Limite 6
(`patch-spec-limite-6.md`) foi escrito mas **nunca aplicado**.
**Pré-requisito**: a spec atual; os ADRs de modelagem e resolução (0002; 0004;
0005; e o que introduziu o `id` — confirmar na Fase 1); os laudos 0006 (id),
0012/0013 (descritor); o `patch-spec-limite-6.md`; o código
(`01_core/src/entities/grafo.rs`).
**Posição**: maior débito de documentação do projeto. A spec se autodeclara "o
contrato central"; hoje ela mente em pontos centrais e induz quem a lê ao
modelo errado (a mesma classe de erro do comentário do `crate_name`, corrigido
no laudo 0026 — mas aqui no documento que governa o resto).
**Arquivos afetados**: `00_nucleo/specs/forma-organizada.md`; retirar
`00_nucleo/specs/patch-spec-limite-6.md` depois de aplicado. **Nenhum código.**

---

## Contexto

A spec é documentação, não código — este prompt **não muda comportamento**.
Faz a spec dizer a verdade sobre o sistema que ela governa.

O que divergiu, em quatro pontos:

1. **Identidade.** A spec (Invariante 1; tabela do nó) diz que `path` é único.
   A forma **crua** (que o adaptador L3 produz) tem identidade por `id`, e
   paths **colidem** (97 colisões no egui). A unicidade de path só vale **depois
   da resolução**.
2. **Camada de resolução ausente.** Existe `lente_investiga` + `lente_resolve`
   (ADR-0004/0005) que resolve colisões e **restaura** a unicidade de path. A
   spec não a menciona. O `calcular_raio` (que é por path) consome a forma
   **resolvida**, não a crua.
3. **Campos do nó.** A spec lista 4 (`path`, `name`, `kind`, `visibility`). O
   código tem ~12: os 4 mais `id`, `modificadores`, `crate_name`, `trait_`,
   `trait_ref`, `cfg`, `macro_kind`, `is_non_exhaustive` — o descritor semântico
   dos laudos 0012/0013. Além disso, `kind` no código é só o **tipo base**
   (enum `Kind`), com os modificadores (`const`/`async`/`unsafe`) separados em
   `Modificadores` — diferente da lista de `kind` da spec, que mistura
   `const fn`/`async fn`/`unsafe trait`.
4. **Limite 6.** Descoberto (colisões geradas por macro), patch escrito, não
   aplicado.

---

## Restrições estruturais

- **Só documentação.** Nenhuma mudança de código, tipo ou teste.
- **Preservar a estrutura e o estilo da spec** (seções, tom, "limites medidos
  contra dado real", os Critérios de Verificação).
- **Não apagar a verdade que ainda vale.** A unicidade de path **não** some —
  ela é **relocada**: deixa de ser invariante da forma crua e passa a ser
  propriedade da forma **resolvida**. A spec deve descrever as duas formas (crua
  e resolvida) ou deixar explícito qual forma cada consumidor vê.
- **Fidelidade à fonte** continua sendo o princípio da forma crua — só que a
  fonte hoje traz mais campos.

---

## Fase 1 — Reunir o estado real (leitura)

1. **Código**: `grafo.rs` — os campos reais de `No`, o enum `Kind`, o struct
   `Modificadores`, o `id`. A descrição honesta do `crate_name` (já corrigida
   no laudo 0026 — reusar esse texto).
2. **ADRs**: o de modelagem (0002, inclui a D3 do prefixo de stdlib); os de
   resolução (0004, 0005); e **qual** registrou a identidade por `id`
   (confirmar — provavelmente o laudo 0006 `id_no_core_infra`, e o ADR
   correspondente, se houver).
3. **Laudos**: 0006 (id), 0012/0013 (descritor semântico).
4. **O patch**: `patch-spec-limite-6.md` — o texto do Limite 6 e a adição à
   nota de evolução.

**Reportar no laudo**: a lista exata dos deltas aplicados e as fontes (ADR/laudo)
de cada um.

---

## Fase 2 — Reescrever a spec

- **Identidade (Invariante 1 + tabela do nó)**: a forma crua tem identidade por
  `id`; `path` **pode** repetir. Citar a origem (laudo 0006). Adicionar a
  distinção forma-crua vs forma-resolvida.
- **Camada de resolução**: nova subseção descrevendo que colisões de path são
  resolvidas (ADR-0004/0005) e que a forma que o `calcular_raio` consome é a
  **resolvida** (paths únicos de novo). Mencionar `investiga`/`resolve` como os
  componentes L1 que fazem isso.
- **Estrutura do nó** (exemplo JSON + tabela): atualizar para os campos reais.
  Documentar `kind` como tipo base + `Modificadores` separados. Descrever
  `crate_name` conforme o laudo 0026 (crate-raiz copiado; **não** distingue
  stdlib). Marcar quais campos vêm do descritor (laudos 0012/0013).
- **Limite 6**: aplicar o texto do patch, no estilo dos Limites 1–5, e a adição
  à nota de evolução.
- **Resultado Esperado**: os três derivados estão **construídos** — `tipo de
  dados` (`lente_core`), `adaptador` (`lente_infra`), `filtro de stdlib`
  (`lente_filtro`, laudo 0025). Atualizar de "derivam depois" para "feitos",
  com ponteiros. (Opcional: nota de que `resolve`/`raio`/`ranking`/`wiring`
  também existem, fora do escopo desta forma.)
- **Histórico de Revisões**: nova linha datada com o motivo (reconciliação).
- **Retirar** `patch-spec-limite-6.md` (aplicado).

---

## Critérios de Verificação

```
Dado a spec reconciliada
Então o Invariante 1 reflete identidade por id (path pode colidir na forma crua)
  e a forma resolvida (paths únicos) está descrita como o que o raio consome

Dado a tabela/JSON do nó na spec
Então lista os ~12 campos reais (id + descritor), com kind como base + Modificadores
  e crate_name descrito como no laudo 0026

Dado a seção de limites
Então o Limite 6 está presente (colisões geradas por macro), no estilo dos 1–5

Dado o repositório
Então patch-spec-limite-6.md foi retirado (aplicado)

Dado a spec e o grafo.rs
Então não há contradição entre o que a spec descreve e os campos/identidade do código

Dado o workspace
Então nenhuma mudança de comportamento (só documentação): suíte verde, contagem igual
```

---

## Resultado esperado

- A spec descreve o sistema real: identidade por `id`, forma crua vs resolvida,
  os campos do descritor, a camada de resolução, o Limite 6.
- O patch do Limite 6 deixa de ser arquivo solto.
- Zero mudança de comportamento.
- **Laudo** listando os deltas aplicados e suas fontes.

---

## O que NÃO entra

- **Mudar código, tipos ou testes**: nada — é documentação.
- **Remover o campo `No.crate_name`**: questão separada (ripple largo, ganho
  pequeno; débito do comentário já fechado no 0026).
- **Filtro de folhas (Limite 3)** e **remoção da E2**: fora do escopo.
- **Reescrever os ADRs**: a spec aponta para eles; não os reescreve.

---

## Observação metodológica

O contrato central divergir do sistema é a mesma falha do comentário do
`crate_name` que custou a Fase 1 do 0025 — documentação que mente. O projeto
trata specs e ADRs como documentos vivos (a própria spec tem Histórico de
Revisões), mas esta não foi mantida desde 2026-05-27, enquanto o sistema andou
muito. Reconciliar devolve a honestidade ao documento que os outros componentes
miram — e deixa rastreável (via Histórico) o quanto o desenho evoluiu a partir
do dado real.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-02 | Reconcilia a spec da forma com o sistema: identidade por `id` (forma crua) vs unicidade de path (forma resolvida); camada de resolução (ADR-0004/0005) descrita; campos do descritor semântico (laudos 0012/0013); `crate_name` conforme laudo 0026; Limite 6 aplicado (do patch); "Resultado Esperado" marcado como construído. Sem mudança de comportamento. | `00_nucleo/specs/forma-organizada.md`, `00_nucleo/specs/patch-spec-limite-6.md` (retirado), `00_nucleo/lessons/0028-reconciliar-spec-forma.md` |
