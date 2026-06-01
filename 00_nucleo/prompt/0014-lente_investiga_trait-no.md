# Prompt: `lente_investiga` usa trait-por-nó; E2 em quarentena

**Camada**: L1 — Núcleo (pureza absoluta)
**Criado em**: 2026-05-28
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo 0013 (D4 resolvida na raiz — trait_ vem por nó
com id correto); ADR-0005 (papel da E2); decisão do autor (E2 em quarentena de
remoção por incerteza genuína sobre crates não medidos, não por custo afundado).
**Pré-requisito**: lente_core pós-0012, lente_infra pós-0013 (No.trait_
preenchido com valor real do JSON).
**Terceiro da cascata a jusante.**
**Arquivos afetados**: `05_investiga/src/lib.rs`, `05_investiga/src/fontes.rs`
(quarentena), testes; laudo em `00_nucleo/lessons/`.

---

## Contexto

A investigação 0011 concluiu que a ligação trait↔id confiável tinha que vir
do fork. O fork 0.27.0 passou a emitir `trait` por nó, e o laudo 0013
confirmou: o `No.trait_` chega com o id correto associado (id 36→Display,
id 47→Debug no ErroRaio::fmt). **A D4 está resolvida na raiz** — não há mais
adivinhação de qual id é qual trait.

Consequência: a **E2** (`fontes.rs`, o parser textual de fontes que extraía o
trait quando o JSON não o tinha) **perdeu sua função principal**. O trait vem
de graça no nó, com id correto, sem ler fontes.

Mas a E2 **não é removida agora** — fica em **quarentena de remoção**. Razão
(decisão do autor): a medição de generalização contra crates de outras origens
ainda não aconteceu (pendente desde a 3ª medição). É possível, embora não
demonstrado, que existam casos em crates não medidos onde a E2 decida algo que
a E1 + trait-por-nó não cobrem. A quarentena é por **incerteza genuína**, não
por apego ao trabalho investido.

---

## Restrições estruturais

- **L1 — pureza absoluta.** Zero I/O, zero deps externas, só stdlib.
  `cargo tree -p lente_investiga` continua só `lente_core`.
- **Não tocar o lente_core.** A `Evidencia` e o `Veredito` ficam como estão
  (decisão do autor: não reformar a cascata por "ficar mais limpo" quando ela
  já funciona). A variante `ImplDeTraitsDiferentes` permanece no lente_core —
  não é removida.
- **A E1 permanece o coração.** A E1 (vizinhança) é o que decide se as cópias
  são distintas — isso o trait-por-nó NÃO substitui (o trait nomeia, a E1
  decide a distinção). A E1 não muda.

---

## O que mudar

### 1. A nomeação passa a usar `No.trait_`

Hoje, quando a E1 decide `Distintos` com evidência topológica
(`VizinhancaDisjunta`), o trait não viajava na evidência — o `lente_resolve`
adivinhava (D4). Agora o trait está no `No` (`no.trait_`).

A mudança aqui é mínima e localizada: o `lente_investiga` continua produzindo
`VizinhancaDisjunta` (a E1 decide distinção). O trait para nomeação **não
precisa entrar na evidência** — o `lente_resolve` (próximo prompt) vai ler
`no.trait_` direto do nó que está renomeando. Então:

- O `lente_investiga` **não muda** o que produz na E1 (continua
  `VizinhancaDisjunta`).
- O `lente_investiga` **deixa de tentar a E2** para obter o trait (a E2 sai do
  caminho — ver item 2).
- A correlação trait↔id, que era o problema da D4, agora é trivial: cada nó
  tem seu `trait_`. O `lente_resolve` usa isso. O `lente_investiga` não
  precisa carregar o trait na evidência.

Se a cascata atual já chama a E2 como fallback quando a E1 não decide, essa
chamada é removida (item 2). A E1 que não decide agora simplesmente resulta em
`NaoDeterminado` (os macros do Limite 6) — sem tentar a E2.

### 2. A E2 (`fontes.rs`) sai do caminho — quarentena de remoção

- Remover a chamada à E2 do fluxo da cascata em `lib.rs`. A cascata passa a
  ser: E1 decide; se decide, `Distintos`/`MesmoItem`; se não decide,
  `NaoDeterminado`. Sem etapa E2.
- **Manter `fontes.rs` no repo** — não apagar o arquivo nem seus testes. Ele
  fica compilável e testado, mas **fora do caminho** (ninguém o chama no fluxo
  principal).
- **Comentário de quarentena no topo de `fontes.rs`**: um bloco curto
  explicando que o módulo está fora do caminho, desde quando, por quê, e
  apontando para o laudo. Sugestão:

```rust
//! QUARENTENA DE REMOÇÃO (desde 2026-05-28, laudo 0014).
//!
//! Este módulo (E2 — parser textual de fontes) extraía o trait de impls
//! lendo o código-fonte, porque o JSON do fork não trazia o trait. O fork
//! 0.27.0 passou a emitir `trait` por nó (laudo 0013), tornando a E2
//! desnecessária para seu propósito original.
//!
//! Mantido fora do caminho da cascata, não removido, por INCERTEZA: a
//! medição de generalização contra crates de outras origens ainda não foi
//! feita. Condição de saída da quarentena:
//!   - REMOVER se a medição confirmar que E1 + trait-por-nó cobrem tudo.
//!   - RELIGAR ao caminho se a medição revelar casos que só a E2 decide.
//! Ver laudo 0014 em 00_nucleo/lessons/ para o registro completo.
```

- Os testes de `fontes.rs` continuam rodando (o módulo ainda compila e é
  testado — só não é usado no fluxo). Isso mantém a E2 reconstruível e
  verificável enquanto na quarentena.

### 3. Não reformar a Evidencia nem a cascata

Decisão explícita do autor: a cascata funciona (a D4 está resolvida pela
chegada do trait por nó). Não remover `ImplDeTraitsDiferentes` do lente_core,
não reorganizar a `Evidencia`, não "simplificar" o que já funciona. A única
mudança é tirar a E2 do caminho e deixar o trait vir do nó.

---

## Critérios de Verificação

```
Dado dois nós colidentes que a E1 decide distintos (VizinhancaDisjunta)
Quando investigar
Então produz Veredito::Distintos { VizinhancaDisjunta } — como antes

Dado dois nós colidentes que a E1 NÃO decide (macros, Limite 6)
Quando investigar
Então produz Veredito::NaoDeterminado — SEM tentar a E2
(antes: tentava a E2; agora a E2 está fora do caminho)

Dado o módulo fontes.rs
Quando compilar e testar o crate
Então fontes.rs compila e seus testes passam (quarentena, não remoção)
E fontes.rs NÃO é referenciado pelo fluxo principal de lib.rs

Dado o comentário de quarentena no topo de fontes.rs
Então explica o estado, a razão, a condição de saída, e aponta para o laudo
```

Casos a cobrir:
- E1 decide distintos → `VizinhancaDisjunta` (não-regressão dos 16/17 testes
  existentes).
- E1 não decide → `NaoDeterminado` direto, sem E2.
- `fontes.rs` ainda compila e testa, mas está fora do caminho.
- Não-regressão: os testes do lente_investiga ajustados (os que verificavam a
  cascata chamando a E2 mudam — a E2 não é mais chamada).

---

## Resultado esperado

- `lib.rs`: cascata sem a etapa E2 (E1 decide ou `NaoDeterminado`).
- `fontes.rs`: no repo, compilável, testado, fora do caminho, com comentário
  de quarentena apontando para o laudo.
- Testes ajustados (os que dependiam da E2 no caminho) e não-regressão do
  resto.
- **Pureza**: `cargo tree -p lente_investiga` só `lente_core`.
- **Laudo 0014** em `00_nucleo/lessons/` com:
  - O que mudou (E2 fora do caminho, trait vem do nó).
  - **A justificativa completa da quarentena** e a **condição de saída**
    (remover se medição confirmar cobertura; religar se revelar casos).
  - Sinalização para o lente_resolve (próximo): ler `no.trait_` para nomeação
    por trait com precisão, eliminando a adivinhação da D4.
  - A observação de que a D4 está resolvida na raiz ponta a ponta.

---

## O que NÃO entra (cascata a jusante)

- **lente_resolve**: ler `no.trait_` para nomear por trait com precisão (a
  D4 morre aqui de vez). Próximo e último prompt da cascata do descritor.
- **Medição de generalização**: rodar contra crates de outras origens para
  decidir o destino final da E2 (remover ou religar). Experimento futuro —
  é a condição de saída da quarentena.
- **Filtro de stdlib**: usar prefixo do path (ADR-0002 D3 mantida, já que o
  fork não emite crate por nó — laudo 0013 D1). Componente futuro.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | lente_investiga deixa de chamar a E2 (trait vem por nó, laudo 0013). E2/fontes.rs em quarentena de remoção (fora do caminho, mantido no repo, condição de saída = medição de generalização). E1 e Evidencia inalteradas. | 05_investiga/src/lib.rs, 05_investiga/src/fontes.rs |
