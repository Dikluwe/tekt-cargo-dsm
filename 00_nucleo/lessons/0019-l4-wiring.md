# Laudo de Execução — Prompt 0019 (L4 wiring: composição do pipeline)

**Camada**: L5 (laudo)
**Data**: 2026-06-01
**Prompt executado**: `00_nucleo/prompt/0019-l4-wiring.md`
**Estado**: `EXECUTADO` — pipeline composto pela primeira vez; 85 testes
verdes + 5 ignored; cascata do descritor confirmada ponta-a-ponta.

---

## O que o prompt pediu

Compor o pipeline pela primeira vez. Duas partes:

- **Parte 1**: expor `lente_infra::desserializar_grafo` como fachada limpa
  para JSON → `Grafo` (sem o L4 precisar de DTO/serde).
- **Parte 2**: novo crate `lente_wiring` em `04_wiring/` com
  `calcular_raio_de_alvo(fonte, alvo) -> Result<Raio, ErroLente>`,
  `FonteGrafo`, `AlvoBusca`, `ErroLente`. Só composição, sem CLI.

---

## Parte 1 — `lente_infra::desserializar_grafo`

`pub fn desserializar_grafo(json: &str) -> Result<Grafo, ErroAdaptador>`:
encapsula `serde_json::from_str` (erro → `ErroAdaptador::JsonInvalido`) e
delega à `traducao::traduzir` (que continua `pub(crate)`). A `extrair_grafo`
existente foi refatorada para delegar à nova (sem duplicar).

Dois testes novos:
- `desserializar_grafo_valido_devolve_grafo` (JSON sintético mínimo).
- `desserializar_grafo_invalido_retorna_erro_de_json` (caminho de erro).

Não-regressão: os 17 testes pré-existentes do `lente_infra` continuam
passando sem alteração. Total: **21 verdes + 4 ignored**.

---

## Parte 2 — Crate `lente_wiring`

### Estrutura

```
04_wiring/Cargo.toml  — deps por path: lente_core, lente_infra,
                        lente_investiga, lente_resolve. Zero deps externas.
04_wiring/src/lib.rs  — função pública, tipos, ErroLente, helpers, testes.
```

Adicionado a `members` do workspace.

### API pública

```rust
pub fn calcular_raio_de_alvo(fonte: FonteGrafo, alvo: AlvoBusca)
    -> Result<Raio, ErroLente>

pub enum FonteGrafo { Json(String), Pacote(String) }
pub enum AlvoBusca  { PorPath(Path), PorId(usize) }
pub enum ErroLente  { Fork, Adaptador, Resolucao, Raio, IdInexistente }
```

`ErroLente` com `impl From<...>` para cada variante interna (uso natural
com `?`), `Display`, e `std::error::Error`.

### Fluxo do pipeline (passos correspondendo ao prompt)

1. `FonteGrafo::Json(s)` → `s`; `FonteGrafo::Pacote(p)` →
   `lente_infra::fork::invocar_fork(&p)`. Erro vira `ErroLente::Fork`.
2. `lente_infra::desserializar_grafo(&json)` → `Grafo` ou
   `ErroLente::Adaptador`.
3. `detectar_colisoes(&grafo)` — helper interno: agrupa nós por path,
   devolve os paths com 2+ nós.
4. Para cada `path_colidente`: `resolver_uma_colisao(grafo, path)` —
   investiga o primeiro par (ordem de id), aplica o veredito. Grafo evolui.
5. `AlvoBusca::PorPath(p)` → usa `p`. `AlvoBusca::PorId(id)` → procura o
   nó com aquele id **no grafo já resolvido** e usa seu path (o path pode
   ter sido renomeado pelo resolve — comportamento correto).
6. `calcular_raio(&grafo, &path_alvo)` → `Raio` ou `ErroLente::Raio`.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test` (sem ignored) | **85 verdes** (core 30, infra 21, investiga 17, resolve 11, **wiring 6**) |
| `cargo test -- --ignored` | **5/5** (4 do infra + **1 do wiring** novo) |
| `cargo tree -p lente_core` | só o crate — pureza preservada |
| `cargo tree -p lente_wiring` | 4 deps internas por path, **zero externas** |

### A verificação crucial — passou

O teste `verificacao_crucial_colisao_some_e_traits_aparecem` faz a inspeção
prometida pelo prompt:

```
ANTES do pipeline: dois nós colidem em "t::T::fmt"
DEPOIS do pipeline: "t::T::fmt" não existe;
                    "t::T::<Display>::fmt" e "t::T::<Debug>::fmt" existem
```

Este é o **primeiro ponto** onde se prova que a cascata inteira do descritor
(laudos 0012/0013/0014/0015) funciona **quando composta**, não só isolada:

- `lente_infra` desserializa `trait_` por nó.
- `lente_investiga` decide `Distintos/VizinhancaDisjunta` (E1 sozinha, E2
  em quarentena).
- `lente_resolve` lê `no.trait_` e nomeia `<Display>` / `<Debug>` no id
  correto, encerrando a D4 ponta-a-ponta.

E o teste `pipeline_completo_renomeia_colisao_por_trait_do_no` confirma a
ponta complementar: `AlvoBusca::PorId(20)` (id do nó com `trait_=Display`)
devolve um `Raio` cujo `alvo` é `"t::T::<Display>::fmt"` — o pipeline
resolveu o id para o **novo path**, automaticamente.

---

## Decisões tácitas

### D1 — Detecção de colisões uma vez, no grafo de entrada

`detectar_colisoes` roda **uma única vez** no grafo recém-desserializado.
Razão: cada chamada de `lente_resolve::aplicar` só toca o path da colisão
que está resolvendo. Os outros paths colidentes permanecem colidindo no
grafo intermediário até serem processados — então a lista capturada no
início continua válida durante toda a iteração.

Alternativa rejeitada: re-detectar a cada passo. Mais robusto a mudanças
futuras no `aplicar`, mas custo extra sem benefício atual.

### D2 — `From` impls para `ErroLente` (em vez de mapeamento explícito)

O laudo 0018 D2 preferiu mapeamento explícito sobre `From` impl
(`mapear_erro_fork` em vez de `impl From<ErroFork> for ErroAdaptador`). Aqui
em L4 fiz o **oposto**: cada erro interno tem `impl From<...> for ErroLente`,
para `?` funcionar naturalmente em toda a função `calcular_raio_de_alvo`.

Razão da inversão: o L4 é **composição** — propaga muitos erros através de
uma sequência. Mapeamento explícito em cada `?` viraria ruído visual sem
ganho. No L3 (laudo 0018) a conversão era **localizada** em um ponto e
**ramificada** (`FalhaSubprocess` se ramifica em `BinarioNaoEncontrado` ou
`FalhaSubprocesso(String)` conforme `ErrorKind::NotFound`) — explicitar
fazia sentido. Aqui as conversões são 1:1 e múltiplas; `From` é apropriado.

Princípio implícito: **explícito quando o mapeamento se ramifica;
implícito (From) quando é 1:1 e a função propaga muitos erros**.

### D3 — Par de cópias por ordem crescente de id

Para cada colisão, escolho os dois ids menores (após `sort_unstable`).
Consistência com a 3ª medição (laudo 0009), que usou "primeiro par" também.
Caso de 3+ cópias: o `aplicar` renomeia **todas** as cópias do path numa só
chamada (laudo 0010), então só preciso investigar **um** par para obter o
veredito que dispara a renomeação geral.

### D4 — `fontes: None` no `investigar` (E2 em quarentena)

A E2 está em quarentena (laudo 0014). O L4 sempre passa `None` — coerente
com o estado atual. Quando/se a quarentena for resolvida (remoção ou
religação), o L4 muda só este parâmetro.

### D5 — `AlvoBusca::PorId` resolve o path **atual** do nó

Após o pipeline, o nó com id X pode ter path **renomeado** (ex.: id 20
era `t::T::fmt`, virou `t::T::<Display>::fmt`). O `PorId(20)` retorna o
path **atual** — o que o usuário provavelmente quer (apontou para o item,
não para o path original). Casos onde `MesmoItem` removeu o id (unificação)
retornam `ErroLente::IdInexistente(id)` — comportamento honesto.

### D6 — Crate `lente_wiring`, diretório `04_wiring/`

Nome do package: `lente_wiring` (snake_case, padrão `lente_*`). Diretório
preenche o "buraco" `04_` na numeração que estava livre.

### D7 — `From<&Aresta>` via clone

`construir_vizinhanca` clona arestas para os `ArestasNo`. `Aresta` deriva
`Clone` (laudo 0001), e a `Vizinhanca` do `lente_investiga` aceita
`Vec<Aresta>` por valor. Performance não é crítica em L4; clarity vence.

---

## A pendência do laudo 0016 está resolvida (parcialmente)

O laudo 0016 (verificação do raio) registrou que a "garantia resolve→raio"
não existia, porque não havia composição. **Agora existe**: o
`calcular_raio_de_alvo` sempre executa as resoluções **antes** do
`calcular_raio`. Quem usar o L4 sempre vê grafo resolvido; o raio operar por
path é seguro neste caminho.

A dívida raio-por-id permanece **latente, sem dor ativa**: se alguém um dia
chamar `lente_core::domain::raio::calcular_raio` diretamente, sem passar
pelo L4, ainda pode receber grafo não-resolvido. Aceitável — a recomendação
deste laudo é manter como está. Se um segundo caminho de composição surgir
e for inseguro, então mudar.

---

## Sinalização para o L2 CLI (próximo prompt)

O L2 vai:
- Parsear argumentos (`--grafo arquivo.json | --pacote NOME`,
  `--alvo PATH | --id N`, `--out FORMATO`).
- Construir `FonteGrafo` e `AlvoBusca` conforme os args.
- Chamar `lente_wiring::calcular_raio_de_alvo(fonte, alvo)`.
- Formatar `Raio` (ou `ErroLente`) para stdout/stderr.
- O `main()` mora aqui.

Decisões abertas para o L2 (e não este prompt):
- Modo de saída: humano (texto), JSON, ambos?
- Mostrar montante/jusante completos ou só sumário?
- Códigos de saída por categoria de erro?

---

## Não tocado

- `lente_core`, `lente_investiga`, `lente_resolve` — não modificados.
- `lente_infra` recebeu **apenas a adição** de `desserializar_grafo`
  (Parte 1); `extrair_grafo` foi refatorada para delegar, sem mudança de
  contrato externo.
- Crate `remedicao` (Arena): não precisou de ajuste — não usa o L4.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | L4 wiring: primeira composição do sistema. Parte 1: `lente_infra::desserializar_grafo` exposta como fachada. Parte 2: crate `lente_wiring` (04_wiring/) com `calcular_raio_de_alvo`, `FonteGrafo`, `AlvoBusca`, `ErroLente`. Verificação crucial: `t::T::fmt` (colisão) some, `<Display>`/`<Debug>` aparecem — cascata do descritor funciona ponta-a-ponta quando composta. Pendência do laudo 0016 resolvida no caminho L4. 85 testes + 5 ignored; pureza preservada. | `Cargo.toml` (raiz), `03_infra/src/lib.rs`, `04_wiring/Cargo.toml` (novo), `04_wiring/src/lib.rs` (novo) |
