# Prompt: `lente_resolve` nomeia por trait via `no.trait_` (D4 encerrada)

**Camada**: L1 — Núcleo (pureza absoluta)
**Criado em**: 2026-05-28
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo 0010 (lente_resolve original, D4); laudo 0014
(E2 em quarentena, trait por nó); ADR-0006 (nomeação por trait é padrão, flag
aposentada).
**Pré-requisito**: lente_core pós-0012 (No.trait_), lente_infra pós-0013
(No.trait_ preenchido), lente_investiga pós-0014.
**Quarto e último da cascata a jusante do descritor.**
**Arquivos afetados**: `06_resolve/src/lib.rs`, testes.

---

## Contexto

O `lente_resolve` (laudo 0010) nomeia identidades distintas. Para o caso comum
(`VizinhancaDisjunta`), usava contador (`#1`/`#2`). A nomeação por trait só
acontecia com `ImplDeTraitsDiferentes`, e tinha a **D4**: a evidência carregava
os traits mas não dizia qual id era qual, então a atribuição era adivinhada
(menor id = primeiro trait) — podia ficar trocada.

Agora (laudos 0012/0013/0014) o `trait_` vem **por nó**, com o id correto. O
`lente_resolve` pode ler `no.trait_` direto do nó que está renomeando, sem
adivinhar. A D4 encerra-se aqui.

Conforme ADR-0006: a nomeação por trait passa a ser **padrão** (não
enriquecimento opcional sob flag — a flag foi aposentada porque ligava a E2,
hoje em quarentena, para obter o trait que agora vem de graça).

---

## Restrições estruturais

- **L1 — pureza absoluta.** Zero I/O, zero deps externas, só stdlib.
  `cargo tree -p lente_resolve` continua só `lente_core`.
- **Mudança localizada.** Só a lógica de nomeação muda. A redistribuição de
  arestas por id (laudo 0010, determinística) não muda. Os outros caminhos
  (`MesmoItem`, `NaoDeterminado`, erros) não mudam.

---

## O que mudar

### A regra de nomeação (ADR-0006)

Para `Veredito::Distintos`, ao renomear cada nó colidente:

- **Se o nó tem `trait_` (`Some(t)`)**: nomeia por trait — `<t>` inserido
  antes do último segmento do path. Ex.: nó `ErroRaio::fmt` com `trait_ =
  Some("Display")` vira `ErroRaio::<Display>::fmt`. O trait vem do **próprio
  nó** que está sendo renomeado, então o id↔trait é exato (D4 resolvida).
- **Se o nó não tem `trait_` (`None`)**: cai no **contador** `#1`/`#2` por
  ordem de id (laudo 0010), o piso. Nós sem trait são os inerentes (dois
  métodos de impls inerentes com mesmo nome) e os macros (Limite 6).

A regra é única (sem flag, sem modo): cada nó é nomeado por seu `trait_` se
tem, por contador se não tem. Dois nós colidentes podem misturar — um com
trait, outro sem? Na prática raro, mas a regra trata nó a nó: cada um pega seu
próprio nome pela presença ou ausência do seu `trait_`.

### O que sai

- A dependência da evidência `ImplDeTraitsDiferentes` para obter o trait. O
  trait agora vem do nó, não da evidência. A variante `ImplDeTraitsDiferentes`
  **permanece no lente_core** (decisão de não reformar — laudo 0014), mas o
  `lente_resolve` não precisa mais extrair o trait dela; lê do nó.
  - Se o `lente_resolve` recebe `ImplDeTraitsDiferentes`, pode ignorar os
    traits da evidência e usar `no.trait_` (a fonte confiável). Ou tratar
    ambos os casos (`VizinhancaDisjunta` e `ImplDeTraitsDiferentes`) pela
    mesma regra: lê `no.trait_`. Decisão do gerador, registrar — o importante
    é que o trait usado venha do **nó**, não da evidência (id correto).
- A lógica de adivinhação da D4 (menor id = t0) **é removida**. Não há mais
  atribuição por ordem — o trait está no nó certo.

### O que NÃO muda

- Contador por ordem de id (para nós sem trait) — igual ao laudo 0010.
- Redistribuição de arestas por `id_from`/`id_to` — determinística, igual.
- `MesmoItem` (unificação, dedup de arestas) — igual.
- `NaoDeterminado` → `ColisaoNaoResolvida` — igual.
- Os erros (`ColisaoInexistente`, etc.) — iguais.

---

## Critérios de Verificação

```
Dado dois nós colidentes ErroRaio::fmt, um com trait_=Some("Display") (id 36),
outro com trait_=Some("Debug") (id 47), veredito Distintos/VizinhancaDisjunta
Quando aplicar
Então o nó id 36 vira ErroRaio::<Display>::fmt E o id 47 vira ErroRaio::<Debug>::fmt
E a atribuição é EXATA (não adivinhada) — o trait veio do próprio nó

Dado dois nós colidentes SEM trait_ (None — métodos inerentes), Distintos
Quando aplicar
Então caem no contador: X#1 (menor id), X#2 (maior id)

Dado um nó com trait_=Some e outro sem (None) na mesma colisão
Quando aplicar
Então o com trait vira <Trait>, o sem trait vira #N — cada um por sua regra

Dado os casos já cobertos pelo laudo 0010 (MesmoItem, NaoDeterminado,
ColisaoInexistente, 3+ cópias, determinismo, redistribuição por id)
Então continuam passando — não-regressão
```

Casos a cobrir:
- Trait por nó, atribuição exata (o teste que mata a D4 — verificar que
  Display vai no id certo, não adivinhado).
- Contador para nós sem trait (não-regressão do laudo 0010).
- Mistura (um com, um sem trait) na mesma colisão.
- 3+ cópias: se têm trait, cada um pelo seu; se não, contador #1/#2/#3.
- Não-regressão de todos os 9 testes do laudo 0010.
- Determinismo: aplicar 2× dá o mesmo (o trait do nó é estável).

---

## Resultado esperado

- Regra de nomeação única: `no.trait_` se presente, contador se ausente.
- Adivinhação da D4 removida.
- Testes ajustados e novos (o teste que confirma atribuição exata de trait).
- **Pureza**: `cargo tree -p lente_resolve` só `lente_core`.
- **Laudo** em `00_nucleo/lessons/`:
  - A regra nova (trait por nó padrão, contador piso).
  - **Confirmação de que a D4 está encerrada** — atribuição exata, ponta a
    ponta (fork emite trait por nó → infra preenche → resolve nomeia com id
    correto).
  - Como tratou `ImplDeTraitsDiferentes` (ignorar traits da evidência, usar
    no.trait_).
  - Estado da cascata do descritor: completa (core, infra, investiga, resolve
    todos feitos).

---

## O encerramento da cascata e da D4

Este é o último prompt da cascata do descritor. Ao terminá-lo:

- A cascata está completa: `lente_core` (forma), `lente_infra` (consome),
  `lente_investiga` (E1 decide, E2 em quarentena), `lente_resolve` (nomeia
  por trait).
- A **D4 está encerrada ponta a ponta**: o trait que o `lente_resolve` usa
  para nomear vem do nó, com id correto, originado no fork. A adivinhação que
  motivou a investigação 0011, a rodada no fork, e toda a sequência, não
  existe mais.

### O que fica para depois (fora da cascata)

- **Medição de generalização** contra crates de outras origens — decide o
  destino da E2 (remover da quarentena ou religar) e valida a resolução de
  colisões além do typst.
- **Filtro de stdlib** — usar prefixo do path (ADR-0002 D3 mantida; o fork não
  emite crate por nó, laudo 0013 D1).
- **Cálculo do raio por id** — revisar o raio.rs para operar por id em vez de
  path (dívida latente: paths colididos podiam confundir o BFS).
- **L2 (mostrar)** — apresentar o resultado ao usuário.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | Nomeação por trait via no.trait_ (padrão, ADR-0006), contador como piso para nós sem trait. Adivinhação da D4 removida — atribuição exata pelo trait do nó. ImplDeTraitsDiferentes não mais usada como fonte do trait. Último da cascata do descritor; D4 encerrada ponta a ponta. | 06_resolve/src/lib.rs |
