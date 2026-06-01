# Laudo de Execução — Prompt 0015 (lente_resolve nomeia por trait via no.trait_)

**Camada**: L5 (laudo)
**Data**: 2026-05-28
**Prompt executado**: `00_nucleo/prompt/0015-lente_resolve_trait_no.md`
**Decisões de origem**: laudo 0010 (D4), laudo 0014 (trait por nó), ADR-0006
(nomeação por trait padrão, flag aposentada).
**Quarto e ÚLTIMO da cascata do descritor.**
**Estado**: `EXECUTADO` — nomeação por trait via nó; D4 encerrada ponta a
ponta; 76 testes verdes + 2 ignored; pureza preservada.

---

## O que o prompt pediu

A nomeação por trait passa a ser **padrão** (ADR-0006): cada nó colidente
ganha nome pelo seu `trait_` quando o tem, ou pelo contador `#N` quando não.
O trait vem do **próprio nó** (`no.trait_`), com id correto — encerrando a
adivinhação da D4 (laudo 0010). A evidência deixa de ser fonte do trait.

---

## O que foi alterado (`06_resolve/src/lib.rs`)

- `aplicar_distintos`: deixou de receber `&Evidencia`. Agora lê `no.trait_`
  de cada nó colidente e nomeia **nó a nó**:
  - `trait_ = Some(t)` → `path_com_trait(path, t)` (ex.: `M::T::<Display>::fmt`).
  - `trait_ = None` → contador `path#N` por ordem de id (piso, laudo 0010).
- `aplicar`: o braço `Distintos { .. }` ignora a evidência (a distinção já foi
  decidida pela E1; o nome vem do nó).
- A lógica de adivinhação da D4 (menor id = primeiro trait da tupla) **foi
  removida**.
- Import de `Evidencia` movido do top-level para o módulo de testes (fora dos
  testes não é mais usado — evita warning).

**Não mudou**: contador por ordem de id; redistribuição de arestas por
`id_from`/`id_to`; `MesmoItem` (unificação + dedup); `NaoDeterminado` →
`ColisaoNaoResolvida`; os erros.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test -p lente_resolve` | **11/11** (9 do laudo 0010 + 2 novos) |
| `cargo test` (workspace) | **76 verdes + 2 ignored** (core 30, infra 18+2, investiga 17, resolve 11) |
| `cargo tree -p lente_resolve` | só `lente_core` — pureza preservada |

Testes novos:
- `distintos_trait_atribuicao_exata_mata_d4` — **o teste que encerra a D4**.
- `distintos_mistura_trait_e_contador` — um nó com trait, outro sem.
- (`distintos_com_trait_nomeia_por_trait` reescrito para usar trait no nó.)

---

## A D4 está encerrada — prova no teste

O teste `distintos_trait_atribuicao_exata_mata_d4` constrói o caso adversário:
o nó de **menor id** (36) carrega `"Debug"`, o de **maior id** (47) carrega
`"Display"` — ordem **inversa** à que qualquer adivinhação por ordem usaria.

- Adivinhação antiga (laudo 0010): menor id = primeiro trait → erraria.
- Regra nova (lê `no.trait_`): id 36 → `<Debug>`, id 47 → `<Display>`. **Exato.**

Cada nó pega exatamente o seu trait porque o trait está no nó, não numa tupla
de evidência sem dono. A atribuição não depende mais de ordem.

### Encadeamento ponta a ponta

1. **0011**: investigação concluiu que a ligação trait↔id tinha de vir do fork.
2. **fork 0.27.0**: emite `trait` por nó, id correto.
3. **0013**: `lente_infra` preenche `No.trait_` com o valor real.
4. **0014**: `lente_investiga` para de adivinhar (E1 decide distinção; E2 em
   quarentena).
5. **0015** (este): `lente_resolve` nomeia lendo `no.trait_`. D4 morta.

---

## Decisões tácitas

### D1 — Evidência ignorada para nomeação; `ImplDeTraitsDiferentes` permanece

O `lente_resolve` trata `VizinhancaDisjunta` e `ImplDeTraitsDiferentes` pela
**mesma regra**: lê `no.trait_`, ignora o conteúdo da evidência. A variante
`ImplDeTraitsDiferentes` **permanece no lente_core** (decisão de não reformar
a cascata, laudo 0014) — só não é mais usada como fonte do trait. Se um dia
o `lente_investiga` voltar a produzi-la, o `lente_resolve` a aceita
(casa `Distintos { .. }`) e nomeia pelo nó do mesmo jeito.

### D2 — Nomeação nó a nó, não por colisão

A regra é aplicada individualmente a cada cópia: cada uma consulta o seu
`trait_`. Isso trata naturalmente o caso misto (um nó com trait, outro sem) —
testado em `distintos_mistura_trait_e_contador`. O contador usa o índice na
ordem de id de **todas** as cópias; no caso misto, o nó sem trait pode receber
`#2` sem que exista `#1` (o `#1` foi "pulado" porque aquele nó usou `<Trait>`).
Aceitável: os paths resultantes continuam únicos, que é o invariante que
importa.

### D3 — Contador continua o piso

Nós sem `trait_` (métodos inerentes colididos; macros do Limite 6) caem no
contador `#N`, exatamente como no laudo 0010. Sem trait, não há nome melhor a
dar — o contador é o piso honesto (ADR-0006).

### D4 — Limite teórico não tratado: dois nós com mesmo trait

Se o fork emitisse duas cópias com o **mesmo** `trait_` (ex.: dois
`<Display>`), os paths colidiriam de novo. Em Rust isso não acontece (não há
dois `impl Display for T`), então não tratei. Caso patológico, registrado.

---

## A cascata do descritor está COMPLETA

| Componente | Papel | Estado |
|------------|-------|--------|
| `lente_core` (0012) | Forma: campos do descritor, Kind base + Modificadores | ✓ |
| `lente_infra` (0013) | Consome: desserializa o descritor do JSON 0.27.0 | ✓ |
| `lente_investiga` (0014) | E1 decide distinção; E2 em quarentena | ✓ |
| `lente_resolve` (0015) | Nomeia por trait via nó; D4 encerrada | ✓ |

A D4 — que motivou a investigação 0011, a rodada no fork (0.27.0), e toda esta
sequência de 4 prompts — **não existe mais**. O trait que nomeia vem do nó,
com id correto, originado no fork.

---

## O que fica para depois (fora da cascata)

- **Medição de generalização** contra crates de outras origens — decide o
  destino da E2 (remover da quarentena ou religar) e valida a resolução além
  do typst.
- **Filtro de stdlib** — prefixo do path (ADR-0002 D3 mantida; o fork não emite
  crate por nó, laudo 0013 D1).
- **Cálculo do raio por id** — `raio.rs` ainda percorre por path; com colisões
  resolvidas (paths únicos após `lente_resolve`) isso fica seguro, mas operar
  por id seria mais robusto. Dívida latente registrada.
- **L2 (mostrar)** — apresentar o raio ao usuário (a visualização, primeiro
  passo da proposta que ainda não começou).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | Nomeação por trait via `no.trait_` (ADR-0006, padrão), contador como piso para nós sem trait. Adivinhação da D4 removida — atribuição exata. `ImplDeTraitsDiferentes` permanece no lente_core mas não é mais fonte do trait. Último da cascata do descritor; D4 encerrada ponta a ponta. 76 testes verdes + 2 ignored; pureza preservada. | `06_resolve/src/lib.rs` |
