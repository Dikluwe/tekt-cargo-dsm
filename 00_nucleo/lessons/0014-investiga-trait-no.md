# Laudo de Execução — Prompt 0014 (lente_investiga: E2 em quarentena)

**Camada**: L5 (laudo)
**Data**: 2026-05-28
**Prompt executado**: `00_nucleo/prompt/0014-lente_investiga_trait-no.md`
**Depende de**: laudo 0013 (No.trait_ por nó). Decisão do autor (quarentena
por incerteza, não por custo afundado).
**Estado**: `EXECUTADO` — E2 fora do caminho, em quarentena; 74 testes verdes
+ 2 ignored; pureza preservada.

---

## O que o prompt pediu

Como o fork 0.27.0 entrega `trait` por nó (laudo 0013), a **Estratégia 2**
(parser textual de fontes) perdeu seu propósito. Tirar a E2 do caminho da
cascata, **mantendo `fontes.rs` no repo em quarentena de remoção** (compilável,
testado, fora do fluxo), com comentário explicando estado e condição de saída.
Não tocar o `lente_core` (Evidencia/Veredito permanecem). A E1 não muda.

---

## O que foi alterado

### `05_investiga/src/lib.rs`

- `investigar` deixou de ter a etapa E2. Agora: pré-condição (paths iguais) →
  **E1 decide** (`Distintos`/`MesmoItem`) ou **`NaoDeterminado`**. Sem fallback
  de fontes.
- O parâmetro `fontes` foi **mantido na assinatura** (compatibilidade — o
  `remedicao` e qualquer chamador continuam compilando), mas é ignorado no
  corpo (`let _ = fontes;` com comentário). Decisão do autor: não reformar a
  cascata que funciona.
- `extrair_tipo_e_metodo` **movida para `fontes.rs`** (era cola exclusiva da
  E2; não fazia sentido ficar no caminho principal sem uso).
- `mod fontes` anotado com `#[allow(dead_code)]` — nada do fluxo o referencia,
  então sem o allow haveria warnings de código não-usado durante a quarentena.

### `05_investiga/src/fontes.rs`

- **Comentário de quarentena no topo** (estado, razão, condição de saída,
  ponteiro para este laudo), conforme o modelo do prompt.
- `extrair_tipo_e_metodo` recebida de `lib.rs`.
- Nova `investigar_por_fontes(par, fontes) -> Veredito` — encapsula a E2
  ponta-a-ponta (extrair tipo/método + analisar). Mantém a E2 reconstruível e
  testável **sem** passar pelo `investigar`.

### Testes

- `lib.rs::tests`: os dois testes que exercitavam a E2 via `investigar` foram
  substituídos. `vizinhanca_ambigua_sem_fontes_e_nao_determinado` ajustado (o
  diagnóstico agora anuncia "quarentena", não "sem fontes"). Novo
  `e2_fora_do_caminho_vizinhanca_ambigua_com_fontes_da_nao_determinado`:
  confirma que, mesmo passando fontes que a E2 resolveria, `investigar` dá
  `NaoDeterminado` (a E2 não é chamada).
- `fontes.rs::tests`: os testes unitários da E2 permanecem; adicionado
  `investigar_por_fontes_decide_ponta_a_ponta` (a E2 inteira ainda decide
  Display+Debug, exercitada fora do `investigar`).

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test` (workspace) | **74 verdes + 2 ignored** (core 30, infra 18+2, investiga 17, resolve 9) |
| `cargo tree -p lente_investiga` | só `lente_core` — pureza preservada |
| `fontes.rs` compila e seus testes passam | sim (quarentena, não remoção) |
| `fontes.rs` referenciado pelo fluxo principal? | não (`investigar` não o chama) |

---

## A quarentena: justificativa e condição de saída

**Por que não remover a E2 agora**: por **incerteza genuína**, não por apego
ao trabalho investido (laudo 0004). A medição de generalização contra crates
de **outras origens** (fora do typst) ainda não aconteceu. É possível, embora
não demonstrado, que existam colisões em crates não medidos onde:

- A E1 (vizinhança) seja inconclusiva, E
- não haja `trait_` por nó (colisões que não são de impl-de-trait), mas
- a E2 (parser de fontes) consiga decidir.

Enquanto essa possibilidade não for descartada por medição, a E2 fica
disponível — fora do caminho, mas reconstruível.

**Condição de saída da quarentena** (registrada no topo de `fontes.rs`):

- **REMOVER** `fontes.rs` (e a função `investigar_por_fontes`) se a medição de
  generalização confirmar que **E1 + trait-por-nó cobrem tudo**.
- **RELIGAR** a E2 ao caminho da cascata se a medição revelar casos que **só a
  E2 decide**.

O gatilho é o experimento de generalização (pendente desde a 3ª medição) —
não uma data nem um palpite.

---

## A D4 está resolvida na raiz, ponta a ponta

Encadeando os laudos:

1. **0011** (investigação): concluiu que a ligação trait↔id confiável tinha
   de vir do fork (visibility cobria só 13%, era indireta).
2. **0012/0013**: o fork 0.27.0 emite `trait` por nó; o `lente_infra` preenche
   `No.trait_` com o valor real e id correto (E2E confirma Display/Debug).
3. **0014** (este): o `lente_investiga` deixa de adivinhar/parsear — a E1
   decide a distinção (topológica), e o trait para nomeação está no `No`.

Não há mais adivinhação de qual id é qual trait. A E1 decide **se** as cópias
são distintas; o `trait_` no nó diz **com que nome** cada uma fica. As duas
informações são independentes e ambas confiáveis.

---

## Decisões tácitas

### D1 — Parâmetro `fontes` mantido, ignorado

Em vez de remover `fontes` da assinatura de `investigar` (o que quebraria
chamadores como o `remedicao` da Arena), mantive-o e o ignoro com `let _ =
fontes;`. Coerente com a decisão do autor de "não reformar a cascata que
funciona". Quando a quarentena terminar (remoção da E2), aí sim a assinatura
pode ser simplificada — junto.

### D2 — `extrair_tipo_e_metodo` movida para `fontes.rs`

Era usada só pela E2. Mantê-la em `lib.rs` deixaria código morto no caminho
principal (warning) ou exigiria `#[allow]` individual. Movendo-a para
`fontes.rs` (sob o `#[allow(dead_code)]` do módulo), toda a maquinaria da E2
fica num lugar só — facilita tanto a remoção quanto o religamento futuros.

### D3 — `investigar_por_fontes` criada para manter a E2 testável

Sem ela, os testes de integração da E2 teriam de passar por `investigar` (que
não a chama mais) — impossível. `investigar_por_fontes` encapsula a E2
ponta-a-ponta, permitindo que `fontes.rs::tests` continue verificando que a
E2 decide corretamente. É a forma de "manter testado na quarentena" que o
prompt pede.

### D4 — `#[allow(dead_code)]` no módulo, não em cada item

Anotei `mod fontes` inteiro com `#[allow(dead_code)]`, em vez de espalhar
allows por função. Mais limpo e expressa a intenção ("este módulo todo está
fora do caminho") num lugar só, com comentário.

---

## Sinalização para o próximo prompt (lente_resolve)

- O `lente_resolve` já aceita `Evidencia::ImplDeTraitsDiferentes` (laudo 0010),
  mas o caminho comum recebe `VizinhancaDisjunta` (sem trait na evidência).
- **Agora o `lente_resolve` pode ler `no.trait_` direto do nó que está
  renomeando** — para os casos `VizinhancaDisjunta`, em vez de contador
  (`#1`/`#2`), pode usar `<Display>`/`<Debug>` quando o `trait_` estiver
  presente, com o id **correto** (sem a adivinhação da D4 do laudo 0010).
- Isso mata a D4 de vez: a nomeação por trait passa a ser exata, alimentada
  pelo `trait_` por nó. É o próximo e último prompt da cascata do descritor.

---

## O que NÃO entra (cascata a jusante)

- **lente_resolve**: ler `no.trait_` para nomear por trait com precisão.
  Próximo prompt.
- **Medição de generalização**: condição de saída da quarentena. Experimento
  futuro.
- **Filtro de stdlib**: prefixo do path (ADR-0002 D3 mantida — o fork não emite
  crate por nó, laudo 0013 D1). Componente futuro.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | E2 fora do caminho da cascata (trait vem por nó, laudo 0013). `fontes.rs` em quarentena de remoção: comentário de estado/condição-de-saída, `extrair_tipo_e_metodo` movida, `investigar_por_fontes` p/ testabilidade, `#[allow(dead_code)]` no módulo. E1 e Evidencia inalteradas. `fontes` mantido na assinatura, ignorado. 74 testes verdes + 2 ignored; pureza preservada. | `05_investiga/src/lib.rs`, `05_investiga/src/fontes.rs` |
