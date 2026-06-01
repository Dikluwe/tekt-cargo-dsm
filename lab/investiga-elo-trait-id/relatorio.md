# Investigação do Elo trait↔id

**Tipo**: Experimento de Arena (`lab/`) — faseado
**Prompt**: `00_nucleo/prompt/0011-investiga_elo_trait-id.md`
**Origem**: laudo 0010 (lente_resolve) D4 — a evidência
`ImplDeTraitsDiferentes` carrega os traits mas não diz qual id é qual.
**Data**: 2026-05-28
**Fonte de dados**: JSONs em `lab/medicao-colisoes/remedicao/json/` (fork
5fbcdfe8); fontes do typst em `lab/typst-original/`.

---

## Pergunta

Existe informação no JSON do fork, disponível ao `lente_investiga`, que ligue
um **trait específico** a um **id específico**, sem depender de ordem (frágil)?

- Se sim e confiável → correção mora no `lente_investiga`.
- Se não → correção mora no fork (emitir trait junto do nó).

---

## Fase 1 — Inspeção manual

### Casos inspecionados

Foco nos `::fmt` (Display+Debug é o caso clássico, gabarito inequívoco:
Display é sempre impl manual, Debug é quase sempre `#[derive]`).

| Caso | id A (vis) | id B (vis) | vizinhança difere? | gabarito (fonte) | sinal que correlaciona |
|------|-----------|-----------|--------------------|------------------|------------------------|
| `typst::args::Output::fmt` | 304 (priv) | 512 (pub) | **não** (idêntica) | Debug=`#[derive]` (l.537), Display=manual (l.563) | **visibility**: priv↔derivado, pub↔manual |
| `typst::args::DepsFormat::fmt` | 309 (priv) | 521 (pub) | não | `#[derive(Debug)]` + `impl Display` manual | visibility (mesmo padrão) |
| `typst::args::Feature::fmt` | 312 (priv) | 537 (pub) | não | idem | visibility |
| `typst::args::Input::fmt` | 300 (priv) | 510 (pub) | não | idem | visibility |
| `typst::args::Target::fmt` | 310 (priv) | 525 (pub) | não | idem | visibility |
| `typst::world::WorldCreationError::fmt` | 621 (priv) | 624 (pub) | não | idem | visibility |
| `typst_library::diag::FileError::from` | priv | priv (3 cópias) | — | 3× `impl From<...>` **manuais** | **NENHUM** (vis toda igual) |

### O que o gabarito humano usou

Para os `::fmt`: o sinal que distingue as cópias no JSON é a **`visibility`** —
uma cópia é `priv`, a outra `pub`, com vizinhança idêntica. Lendo o fonte, a
correlação é clara e consistente nos 6 casos inspecionados:

- **`pub` ↔ impl manual** (o `impl Display for X { fn fmt }` escrito no código).
- **`priv` ↔ impl derivado** (`#[derive(Debug)]` — o `fmt` é sintetizado pela
  macro; o rust-analyzer marca o item gerado como privado).

Ou seja, a visibility **não distingue o trait diretamente** — distingue
**manual vs. derivado**. No caso Display+Debug isso funciona como proxy
(Display é sempre manual, Debug é quase sempre derivado), mas é uma
coincidência do padrão, não uma ligação trait↔id.

### O teste de generalização (decisivo)

Para os casos `From<X>+From<Y>` (o padrão dominante, e os operadores
aritméticos overloaded), **ambos os impls são manuais** — não há derive.
Resultado: as cópias têm a **mesma visibility** (todas `priv`, ou todas
`pub`). O sinal **desaparece**.

Quantificação sobre as 385 colisões dos 17 crates:

| | vis DIFERE (sinal presente) | vis IGUAL (sinal ausente) |
|---|---:|---:|
| Casos `::fmt` | 20 | 11 |
| Casos não-`::fmt` | 31 | 323 |
| **Total** | **51 (13.2%)** | **334 (86.8%)** |

A visibility distingue as cópias em apenas **13.2%** das colisões. Mesmo
entre os `::fmt`, falha em 11 de 31 (provavelmente tipos onde Debug é manual
ou Display vem de derive externo). Para 86.8% das colisões, não há diferença
de visibility — o sinal é inútil.

### Conclusão da Fase 1: **(b)/(c)**

Não há sinal confiável e geral no JSON que ligue id↔trait:

- O único candidato (visibility) cobre só 13.2% dos casos e é **indireto**
  (distingue manual-vs-derivado, não o trait).
- Mesmo onde funciona, não dá o **nome** do trait — só diz "esta cópia é a
  manual". Para nomear `<Display>`/`<Debug>`, ainda é preciso ler o fonte
  (E2) e correlacionar; a visibility só ajudaria no mapeamento dentro do
  subconjunto manual-vs-derivado.
- Para o padrão dominante (`From<X>+From<Y>`, operadores — todos manuais), a
  visibility é idêntica entre as cópias e não distingue nada.

O único sinal alternativo seria a **ordem dos ids** (id menor ↔ primeiro impl
no texto), que é exatamente o "frágil, não garantido pelo fork" que o prompt
pede para evitar. (Observação: nos `::fmt`, o `priv`/derivado teve id menor
nos 6 casos — mas isso reflete a ordem de processamento do rust-analyzer, não
uma garantia.)

---

## Fase 2 — Não executada

A Fase 1 concluiu (b)/(c). Conforme o prompt ("Só executar se a Fase 1
encontrar um sinal promissor"), a Fase 2 (automação + taxa de acerto) **não
foi executada** — não há sinal promissor que generalize para valer a medição
de taxa de acerto. O sinal visibility já é conhecido por cobrir só 13.2%; medir
sua "taxa de acerto" no subconjunto onde aparece não muda a conclusão de que
ele não serve para o caso geral.

---

## Recomendação: a correção mora no fork

Com base nos dados, a ligação trait↔id confiável **não existe no JSON atual**
e deve ser produzida na fonte. Justificativa:

1. **O fork foi quem colapsou a informação.** Os dois métodos `fmt` (de
   `Display` e `Debug`) têm path idêntico porque o fork não inclui o trait no
   path/nó. Restaurar o trait por nó é análogo ao que já se fez com a
   identidade-por-nó (commit 5fbcdfe8) — uma rodada no fork que adiciona um
   campo, retrocompatível.

2. **Nenhum sinal local resolve o caso geral.** A visibility cobre 13.2% e é
   proxy de manual-vs-derivado. A ordem de id é frágil. Construir a correção
   no `lente_investiga` em cima desses sinais seria resolver um oitavo dos
   casos com heurística não-garantida — trabalho de baixo retorno e risco de
   erro silencioso.

3. **O impacto da imprecisão é pequeno e contido.** A nomeação por trait é
   enriquecimento **opcional** (ADR-0005 Ajuste 3, desligado por padrão); o
   caminho comum usa contador (`#1`/`#2`), que é determinístico e correto. A
   D4 do laudo 0010 só morde quem liga o enriquecimento E tem caso
   Display+Debug — subconjunto pequeno. Não é urgente.

### Forma sugerida da correção no fork (para decisão do autor)

Emitir, em cada nó que seja método de impl-de-trait, o **nome do trait**
(ex.: campo `trait_impl: Option<String>` no nó). Com isso:

- O `lente_investiga` não precisa de heurística — lê o trait direto do nó.
- A nomeação no `lente_resolve` por trait passa a ser exata (id↔trait vem do
  fork), eliminando a imprecisão da D4.
- Resolve não só Display+Debug mas todos os padrões (From, operadores), porque
  o trait viria anotado em cada cópia.

Custo: mais uma rodada no fork, mais um campo retrocompatível. Análogo à
identidade-por-nó. Decisão do autor sobre prioridade — a imprecisão atual é
tolerável no curto prazo (enriquecimento é opcional).

---

## Limites declarados

- Gabarito construído por leitura manual de 6 casos `::fmt` + inspeção de
  visibilidade agregada nos 385. Não li o fonte de todos os 385 (o sinal
  agregado de visibility já decide a questão).
- Só typst. Crates de outras origens poderiam ter padrões de visibility
  diferentes, mas é improvável que mudem a conclusão (a visibility marcar
  derivado como priv é comportamento do rust-analyzer, não do typst).
- A hipótese "priv↔derivado, pub↔manual" foi confirmada nos 6 casos lidos;
  não exaustivamente provada. Mas como a conclusão é "o sinal não generaliza"
  (independente de qual lado é qual), a hipótese exata não altera a recomendação.

---

## Artefatos

- `lab/investiga-elo-trait-id/relatorio.md` — este documento.
- Dados-fonte: JSONs em `lab/medicao-colisoes/remedicao/json/` (não
  duplicados aqui).
