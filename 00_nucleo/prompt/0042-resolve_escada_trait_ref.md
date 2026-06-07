# Prompt: `lente_resolve` — escada de nomeação `trait` → `trait_ref` → contador (ADR-0006)

**Camada**: L1 — Núcleo (pureza absoluta)
**Criado em**: 2026-06-05
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0042-resolve-escada-trait-ref.md`)
**Decisões de origem**: laudo 0041 (a regra de nomeação por `trait` **colide**
quando as cópias compartilham o mesmo trait — impls genéricos de `From`;
categoria `DistintosPosRegraColide` detectada na Arena); decisão do autor nesta
conversa: corrigir com uma **escada** `trait` → `trait_ref` → contador.
**Pré-requisito**: `No.trait_ref` já existe e é preenchido (cascata do descritor,
laudos 0012/0013). A extração padrão da lente traz `trait_ref` (o laudo 0041 leu
os valores reais: `From<&str>`, `From<String>`, etc.).
**Arquivos afetados**: `06_resolve/src/lib.rs`, `00_nucleo/adr/0006-*.md`, testes
do `lente_resolve`.

---

## Contexto

A regra atual (pós-0015, ADR-0006) nomeia cada nó colidente `Distintos` por
`no.trait_`: `ErroRaio::fmt` com `trait_ = Some("Display")` vira
`ErroRaio::<Display>::fmt`; nó sem `trait_` cai no contador `#N` por ordem de id.

O laudo 0041 (Arena) achou um buraco: quando **todas** as cópias colidentes têm
o **mesmo** `trait_`, os nomes novos **colidem entre si**. No próprio repo da
lente:

- `lente_core::…::Path::from`: 2 cópias, ambas `trait_ = "From"` → viram
  `Path::<From>::from` ×2. O `trait_ref` real distingue: `From<&str>`,
  `From<String>`.
- `lente_wiring::ErroLente::from`: 4 cópias, todas `trait_ = "From"` → viram
  `ErroLente::<From>::from` ×4. `trait_ref` real: `From<ErroFork>`,
  `From<ErroAdaptador>`, `From<ErroResolve>`, `From<ErroRaio>`.

Isso **viola o invariante "paths únicos após resolução"** (laudo 0010), em
silêncio: os testes unitários do `lente_resolve` usam o caso `Display + Debug`
(que o `trait_` já distingue), então o caso `From<T>` nunca foi exercitado em
teste. A Arena, rodando a resolução sobre os grafos reais, o expôs.

A correção: uma **escada** de identificadores, do mais curto ao mais específico,
parando no primeiro que torna os nomes do conjunto únicos — `trait_` →
`trait_ref` → contador. O `No.trait_ref` já existe (cascata do descritor); não
precisa de campo novo.

---

## Restrições estruturais

- **L1 — pureza absoluta.** Zero I/O, zero deps externas, só stdlib.
  `cargo tree -p lente_resolve` continua só `lente_core`.
- **Mudança localizada.** Só a lógica de nomeação muda. Não mudam: a
  redistribuição de arestas por `id_from`/`id_to` (laudo 0010, determinística),
  o `MesmoItem`, o `NaoDeterminado`, os erros (`ColisaoInexistente`, etc.).
- **`Path` aceita `<>` aninhado** (laudo 0010 D2 — sem validação). `From<&str>`
  embrulhado em `<…>` é aceito; confirmar.

---

## O que mudar

### A regra de nomeação (ADR-0006 → escada)

Para `Veredito::Distintos`, ao renomear o conjunto de nós que colidem num path,
escolher para cada nó o **degrau mais curto da escada que deixa os nomes do
conjunto únicos**:

1. **`trait_`** (degrau atual): nó com `trait_ = Some(t)` → `<t>` antes do último
   segmento. Nó sem `trait_` (`None`) → vai direto ao **contador** (degrau 3),
   como hoje (métodos inerentes, macros — Limite 6).
2. **`trait_ref`**: se dois ou mais nós ficam com o **mesmo** nome no degrau 1
   (mesmo `trait_`), reescrever **esses** por `<trait_ref>` (a referência com
   argumentos: `From<&str>`). Ex.: `Path::from` (`trait_ref = Some("From<&str>")`)
   → `Path::<From<&str>>::from`.
3. **Contador `#N`** (piso): se ainda colidem no degrau 2 (mesmo `trait_ref`, ou
   `trait_ref = None` num nó cujo `trait_` colidiu), reescrever **esses** pelo
   contador `#N` por ordem de id (laudo 0010). É o piso que **sempre** garante
   unicidade (id é único no grafo).

O resultado: **todo path resolvido é único** (o invariante volta a valer). As
cópias que o `trait_` já distingue (`Display + Debug`) **mantêm** `<trait>` — sem
regressão. As cópias de mesmo trait (`From<&str>` vs `From<String>`) ganham
`<trait_ref>`. O contador é a garantia final.

### Formato

`<trait_ref>` inserido antes do último segmento, **mesmo mecanismo** do `<trait>`
(laudo 0010 D9, `rsplit_once("::")`). Só muda o texto inserido.

### Atualizar a ADR-0006

Documentar a escada (`trait` → `trait_ref` → contador) na ADR-0006. Emendar a
ADR-0006 existente ou supersedê-la com uma ADR nova — escolha do gerador,
registrar no laudo (o projeto já emendou/supersedeu ADRs antes, p.ex. laudo
0008).

---

## O que NÃO muda

- Redistribuição de arestas por `id_from`/`id_to` — determinística, igual.
- Nós **sem** `trait_` → contador `#N` — igual ao laudo 0010/0015.
- O caso que o `trait_` **já** distingue (`Display + Debug`) → `<trait>` — igual,
  sem regressão.
- `MesmoItem` (unificação, dedup) — igual.
- `NaoDeterminado` → `ColisaoNaoResolvida` — igual.
- Os erros (`ColisaoInexistente`, etc.) — iguais.

---

## Critérios de Verificação

```
Dado duas cópias ErroRaio::fmt, trait_ Some("Display") e Some("Debug")
(distintos), Distintos
Quando aplicar
Então viram ErroRaio::<Display>::fmt e ErroRaio::<Debug>::fmt (degrau 1, sem
regressão)

Dado duas cópias Path::from, ambas trait_ Some("From"), trait_ref distintos
Some("From<&str>") e Some("From<String>"), Distintos
Quando aplicar
Então viram Path::<From<&str>>::from e Path::<From<String>>::from (degrau 2)
E os dois paths são únicos

Dado quatro cópias ErroLente::from, todas trait_ Some("From"), trait_ref
distintos (From<ErroFork>, From<ErroAdaptador>, From<ErroResolve>, From<ErroRaio>)
Quando aplicar
Então viram quatro paths distintos por <trait_ref>, todos únicos

Dado duas cópias de mesmo trait_ E trait_ref None (sem referência)
Quando aplicar
Então caem no contador #1/#2 (degrau 3)

Dado o caso patológico: duas cópias com trait_ E trait_ref idênticos
Quando aplicar
Então caem no contador #1/#2 (o piso garante unicidade)

Dado duas cópias SEM trait_ (None) — métodos inerentes
Quando aplicar
Então contador #1/#2 (não-regressão do laudo 0010)

Dado o grafo de saída de qualquer caso de sucesso
Então paths únicos, ids únicos, integridade referencial das arestas

Dado aplicar duas vezes ao mesmo grafo
Então mesmo resultado (trait_, trait_ref, id são estáveis no nó)
```

Casos a cobrir: `Display+Debug` (degrau 1, não-regressão); `From<T>` 2 cópias e 4
cópias (degrau 2); `trait_ref` ausente com `trait_` colidindo (degrau 3); mesmo
`trait_ref` (degrau 3, patológico); sem `trait_` (contador, não-regressão); todos
os 9+ testes anteriores do `lente_resolve`.

---

## Resultado esperado

- Regra de nomeação em escada: `trait_` se distingue, `trait_ref` se o `trait_`
  colide, contador como piso. Todo path resolvido único.
- ADR-0006 atualizada (emenda ou nova ADR — registrar a escolha).
- Testes: não-regressão do `Display+Debug` e dos casos sem trait; novos para
  `From<T>` (2 e 4 cópias), `trait_ref` ausente, e o patológico.
- **Pureza**: `cargo tree -p lente_resolve` só `lente_core`.
- **Laudo** em `00_nucleo/lessons/0042-…`:
  - A regra nova (a escada, com o formato de cada degrau).
  - A escolha emenda-vs-nova-ADR para a 0006.
  - **Confirmação** de que `Path::from` e `ErroLente::from` agora dão paths
    únicos (o caso real do 0041).
  - Se a extração padrão traz `trait_ref` preenchido (deve trazer — o 0041 leu os
    valores); se algum nó relevante vier com `trait_ref = None`, registrar (cai no
    contador).
  - **Não-regressão coordenada**: a correção fecha uma violação latente do
    invariante "paths únicos" que os testes unitários não pegavam (usavam
    `Display+Debug`). Verificar se algum teste de integração/E2E (que roda a
    resolução sobre grafos reais) muda de comportamento. Reconciliar a contagem
    da suíte (era 213 verdes + 22 ignored no laudo 0041).

---

## Cuidados

- **Violação latente fechada**: hoje a resolução de produção produz paths
  colididos para cópias de mesmo trait (`Path::from`, `ErroLente::from`), sem que
  os testes unitários peguem (usam `Display+Debug`). Esta correção fecha isso;
  confirmar `paths únicos` no caso `From<T>` com teste novo, e checar se algum
  teste de integração muda.
- **`trait_ref` aninhado no path**: `Path::<From<&str>>::from` aninha `<>`; o
  `Path` aceita (laudo 0010 D2) — confirmar que segue aceitando.
- **`trait_ref` ausente**: se um nó de trait-colidindo tem `trait_ref = None`,
  cai no contador (degrau 3) — não inventar nome.
- **Determinismo**: `trait_`, `trait_ref` e `id` são estáveis no nó; a escada é
  determinística (aplicar 2× dá o mesmo). A instabilidade residual é só o `id` do
  petgraph entre extrações (briefing §7), pré-existente — não atribuir a esta
  mudança.
- **Escopo geral, não só o diff**: esta correção melhora a resolução do **lente
  inteiro** — qualquer feature que resolva colisões de impl genérico (não só a
  trilha local). Por isso é mudança de produto, não de Arena.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Regra de nomeação do `lente_resolve` passa a escada `trait_` → `trait_ref` → contador (ADR-0006 atualizada). Fecha a colisão de nomes em impls de mesmo trait com argumentos genéricos distintos (`Path::from` 2×, `ErroLente::from` 4× — achado do laudo 0041), que violava em silêncio o invariante "paths únicos" (testes unitários usavam só `Display+Debug`). `trait_` distinguindo mantém `<trait>` (sem regressão); mesmo `trait_` escala para `<trait_ref>`; contador é o piso. Usa `No.trait_ref` (já existe, cascata do descritor); pureza L1 preservada. | `06_resolve/src/lib.rs`, `00_nucleo/adr/0006-*.md`, testes do `lente_resolve` |
