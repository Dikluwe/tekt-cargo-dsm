# ADR-0006: Nomeação por trait é padrão; flag de enriquecimento aposentada (supersede ADR-0005 Ajuste 3)

**Status**: `PROPOSTO`
**Data**: 2026-05-28
**Relação**: supersede especificamente o **Ajuste 3 do ADR-0005** (flag de
enriquecimento no lente_infra para obter o trait via E2). O resto do ADR-0005
permanece válido.

---

## Contexto

O ADR-0005, Ajuste 3, decidiu que a nomeação por trait (`<Display>` em vez de
`#1`) seria um **enriquecimento opcional**, ligado por uma flag no
`lente_infra`. A razão era o custo: na época, obter o trait exigia a E2
(parser textual de fontes), que lê arquivos `.rs` — I/O caro. Fazer isso
opt-in protegia quem não queria pagar o custo.

Dois fatos mudaram a premissa:

1. O **fork 0.27.0** passou a emitir `trait` por nó (laudos 0012/0013). O
   trait chega no `No.trait_` com o id correto, **sem custo de I/O** — vem no
   JSON que já é lido de qualquer forma.

2. A **E2 foi posta em quarentena** (laudo 0014), justamente porque o trait
   por nó tornou seu propósito (extrair trait das fontes) desnecessário.

Consequência: a flag de enriquecimento ligava uma máquina (a E2) que agora
está fora do caminho, para obter uma informação (o trait) que agora vem de
graça. A flag perdeu o sentido — ela protegia contra um custo que não existe
mais.

---

## Decisão

A nomeação por trait passa a ser **padrão**, não enriquecimento opcional:

- Quando o nó colidente tem `trait_` preenchido (`Some`), a nomeação usa o
  trait: `Tipo::<Display>::fmt`, `Tipo::<Debug>::fmt`. Com o id correto (sem
  a adivinhação da D4 do laudo 0010), porque o trait vem associado ao nó.
- Quando o nó **não** tem `trait_` (`None` — nós que não são de impl-de-trait:
  métodos inerentes colididos, macros do Limite 6), a nomeação cai no
  **contador** (`#1`/`#2` por ordem de id, laudo 0010). O contador continua
  sendo o piso.

A **flag de enriquecimento é aposentada**. Não há mais "modo enriquecido
opcional" — a regra é única: trait quando o nó tem, contador quando não tem,
sempre, sem flag. O `lente_infra` não precisa de modo que liga leitura de
fontes (a E2 está em quarentena; o trait vem do JSON).

O Ajuste 3 do ADR-0005 fica marcado como **superado por este ADR**. O
ADR-0005 deve receber uma nota em seu Ajuste 3 apontando para cá.

---

## Consequências

**Positivas**:
- Nomes legíveis (`<Display>`) por padrão, sem ninguém precisar ligar nada —
  e confiáveis, porque o trait vem por nó com id correto.
- A D4 (laudo 0010 — trait podia ficar trocado) **morre de vez**: a nomeação
  por trait é exata, alimentada pelo `trait_` por nó.
- Simplificação: uma regra de nomeação em vez de duas (padrão + enriquecido).
  Sem flag, sem modo condicional.

**Negativas**:
- Nós sem `trait_` (inerentes, macros) ainda usam contador — nomes menos
  informativos. Mas é o piso honesto: sem trait, não há nome melhor a dar.

**Neutras**:
- A flag nunca chegou a ser implementada (o lente_infra do laudo 0013 preenche
  o trait, mas a flag de enriquecimento era prospectiva). Então aposentá-la é
  remover uma intenção, não código — não há refatoração.

---

## Ciclo de vida (lição M3 do LESSONS)

Terceira superação granular nesta linha do projeto (após a reversão da D1 do
laudo 0006, e uma proposta anterior sobre a D3 do ADR-0002 que foi descartada
por se basear em premissa que o laudo 0013 refutou — o fork não emite crate
por nó). Todas têm a mesma raiz: **o fork ganhou uma capacidade, e uma decisão
que existia por limitação da fonte tornou-se obsoleta**. Não é erro — é a
fonte melhorando e o que dependia dela reajustando.

Candidato a entrada no `LESSONS.md`: "quando a fonte de dados evolui, decisões
a jusante que existiam por limitação da fonte tornam-se superações granulares
— obsolescências a registrar, não erros". Instância concreta e recorrente da
M3 (superseded-by granular).

---

## Referências

- ADR-0005, Ajuste 3 — a decisão superada (flag de enriquecimento)
- Laudo 0010 — a D4 (adivinhação trait↔id) que este ADR encerra
- Laudos 0012/0013 — trait por nó no lente_core e lente_infra
- Laudo 0014 — E2 em quarentena
- `LESSONS.md` M3 — ciclo de vida de ADR (superseded-by granular)
