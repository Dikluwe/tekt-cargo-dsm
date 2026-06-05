# Prompt: Corrigir o comentário do campo `No.crate_name` (`lente_core`)

**Camada**: L1 — Núcleo
**Criado em**: 2026-06-02
**Estado**: `PROPOSTO`
**Decisões de origem**: Fase 1 do prompt 0025 — o comentário do campo
`No.crate_name` diz *"Crate de origem do nó (distingue nós do crate-alvo de
nós de stdlib)"*, e isso é **falso** para o fork 0.27.0. O fork não emite
`crate` por nó; o L3 popula o campo com o **crate-raiz do grafo**, igual para
todos os nós (inclusive os de `core`/`alloc`/`std`). Esse comentário induziu o
desenho errado do 0025 (premissa "marcar por `crate_name` resolve o Limite 2").
O fato correto já está registrado no laudo 0013 D1 e no comentário de
`traducao.rs:59-62`.
**Pré-requisito**: nenhum além de `lente_core`.
**Posição**: higiene. Corrige documentação enganosa que já causou um erro.
Independente do ranking.
**Arquivos afetados**: `01_core/src/entities/grafo.rs` (só o doc-comment do
campo). Possivelmente nenhum teste.

---

## Contexto

O campo existe e é populado, mas **não** faz o que o comentário promete. Quem
lê o comentário (humano ou agente) conclui que dá para distinguir stdlib por
`crate_name` — e não dá. O 0025 caiu nisso. Corrigir o texto é barato e impede
o próximo tropeço.

Não se está removendo o campo nem mudando o tipo — só fazendo o comentário
dizer a verdade. (Se o campo deve ou não existir é outra questão, fora deste
escopo.)

---

## Restrições estruturais

- **Só o comentário.** Nenhuma mudança de tipo, lógica ou assinatura.
- **`lente_core` continua puro.** Comentário não afeta build.
- **Não toca o filtro, o L3, nem nada além do doc-comment.**

---

## Fase 1 — Leitura

1. `01_core/src/entities/grafo.rs`: o campo `crate_name` do `No` e seu
   comentário atual.
2. `traducao.rs:59-62` e o laudo 0013 D1: a descrição **correta** de como o
   campo é populado (crate-raiz, por defaulting; o fork não dá `crate` por nó).
   Usar essa descrição como fonte para o texto novo.

---

## Fase 2 — Conserto

Reescrever o doc-comment do campo `No.crate_name` para descrever a realidade.
Deve dizer, em substância:

- É o **crate-raiz do grafo**, populado pelo L3.
- O fork 0.27.0 **não** emite `crate` por nó; o valor é o mesmo para todos os
  nós (ref.: laudo 0013 D1).
- **Não** distingue stdlib de alvo. A marca de stdlib é por **prefixo do path**
  (ADR-0002 D3; aplicada no `lente_filtro`).

(Texto exato a critério do gerador, desde que cubra esses três fatos.)

---

## Critérios de Verificação

```
Dado o campo No.crate_name
Quando se lê o doc-comment
Então ele descreve "crate-raiz, igual para todos os nós, não distingue stdlib"
  e aponta o ADR-0002 D3 / laudo 0013 D1

Dado o workspace
Quando compilado e testado
Então sem mudança de comportamento (só comentário): suíte verde, contagem igual
```

---

## Resultado esperado

- Comentário do `No.crate_name` honesto.
- Zero mudança de comportamento.
- **Laudo** curto registrando a correção e o porquê (evitar o erro do 0025).

---

## O que NÃO entra

- **Remover o campo `crate_name`**: questão separada, fora do escopo.
- **Mudar o `lente_filtro`** (já usa prefixo do path, certo): nada.
- **Tocar tipos, lógica, ou o L3.**

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-02 | Corrige o doc-comment de `No.crate_name`, que afirmava distinguir stdlib (falso para o fork 0.27.0) e induziu o desenho errado do prompt 0025. Sem mudança de comportamento. | `01_core/src/entities/grafo.rs`, `00_nucleo/lessons/0026-l1-comentario-crate-name.md` |
