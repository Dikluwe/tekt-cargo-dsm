# ADR-0005: Validação empírica do ADR-0004 e ajustes pós-medição

**Status**: `PROPOSTO`
**Data**: 2026-05-27
**Relação**: complementa o ADR-0004 (não o supersede). O ADR-0004 permanece
como a hipótese arquitetural; este ADR registra a validação empírica e os
ajustes que a medição forçou.

---

## Contexto

O ADR-0004 decidiu resolver colisões de path com uma cascata de duas
estratégias (vizinhança → fontes), em dois crates L1 (`lente_investiga` e
`lente_resolve`). A decisão foi tomada **sem medição prévia**, com o risco
explicitamente declarado no próprio ADR-0004 ("Decisão tomada sem medição
prévia") e a promessa de que seria revisável conforme os dados.

A medição aconteceu, em três rodadas, registradas em
`lab/medicao-colisoes/`:

1. **1ª medição** (fork antigo, sem identidade-por-nó): a Estratégia 1 era
   estruturalmente inaplicável (o JSON referenciava arestas por path, sem
   distinguir cópias colidentes). Cobertura: 14,3%, toda pela E2.
2. **2ª medição** (fork novo com `id`, mas `ChaveAresta` ainda comparando por
   path — bug): E1 aparente saltou para 30,2%, mas 98% dos vereditos era
   `MesmoItem` por falsa coincidência de chave. Resultado contaminado.
3. **3ª medição** (fork novo + `ChaveAresta` corrigido para `(id_from,
   id_to, relation)`, laudo 0008): E1 sobe a **97,4%**, 100% dos vereditos
   como `Distintos/VizinhancaDisjunta`.

Este ADR registra o que a medição validou e o que ela forçou a ajustar.

---

## O que a medição validou

### A cascata do ADR-0004 funciona — pela E1

O Cenário A previsto pelo ADR-0004 ("E1 resolve a maioria; E2 fica como
fallback raro") **confirmou-se com folga**: a Estratégia 1 (vizinhança por
id) decide 97,4% das 384 colisões medidas nos 17 crates do typst. Dezesseis
dos dezessete crates ficam 100% decididos pela E1, incluindo o
`typst_library` (316 colisões, antes 217 indecididas, agora 0).

A decisão arquitetural fundamental do ADR-0004 — resolver colisões por
investigação de vizinhança, em vez de aceitar como limite ou mascarar — está
validada empiricamente.

### A identidade-por-nó foi a peça que faltava

A diferença entre 14,3% (1ª medição) e 97,4% (3ª medição) é atribuível à
identidade-por-nó no fork (commit 5fbcdfe8) mais a correção do `ChaveAresta`
(laudo 0008). A E1 só pôde funcionar quando as arestas passaram a referenciar
nós por id, e quando o `lente_investiga` passou a comparar arestas por id em
vez de por path.

---

## O que a medição forçou a ajustar

### Ajuste 1 — O papel da E2 muda: de decisão para nomeação

O ADR-0004 desenhou a E2 (parser textual de fontes) como **fallback de
decisão** — a segunda estratégia da cascata, que decide quando a E1 não
decide.

A medição mostrou que, com a chave corrigida, a E1 decide praticamente tudo
(97,4%), e a E2 decide **zero** casos adicionais na 3ª medição. Os casos que
a E2 decidia na 1ª medição (`From<Abs>+From<Em>`, `Add+Add<f64>`, etc.) agora
são decididos pela E1, porque cada `impl` produz métodos com id distinto e
vizinhanças disjuntas no fork novo.

**A E2 deixa de ser fallback de decisão.** Seu novo papel é **enriquecimento
opcional de nomeação**: quando o `lente_resolve` precisa nomear duas
identidades distintas e quer um nome legível (o trait), a E2 pode ser
acionada para tentar descobrir o trait do código-fonte. Mas isso é opcional,
não parte do caminho de decisão.

A E2 **não é removida** — pode ainda decidir casos em crates de outras
origens (não medidos), e o custo de mantê-la é baixo. Mas seu papel no
desenho é menor do que o ADR-0004 antecipava.

### Ajuste 2 — A convenção de nomeação: contador por padrão, trait opcional

O ADR-0004 §3 propunha nomear identidades distintas como
`Tipo::<Trait>::método` (ex.: `ErroRaio::<Display>::fmt`), assumindo que a
evidência traria o trait.

A medição mostrou que a evidência dominante é **topológica**
(`VizinhancaDisjunta`), não o trait — 100% dos vereditos da E1 são
`VizinhancaDisjunta`, que diz "as vizinhanças são disjuntas" mas **não diz
qual trait**. O caso `Display+Debug`, que motivou a convenção original, é
raro (9 de 384); o padrão dominante é traits genéricos com type parameters
diferentes, e mesmo esses são decididos pela E1 topologicamente, sem reportar
o trait.

**Nova convenção de nomeação:**

- **Padrão**: contador por ordem de id. O nó com menor id vira
  `Tipo::método#1`, o próximo `Tipo::método#2`, etc. Determinístico e estável
  (mesma ordem de id → mesma numeração), o que preserva a comparabilidade
  entre versões que a proposta (§6) exige.
- **Enriquecimento opcional**: quando o enriquecimento por E2 está ligado
  (ver Ajuste 3) e a E2 consegue descobrir o trait, o nome usa o trait
  (`Tipo::<Display>::método`). Quando a E2 está ligada mas não acha o trait,
  cai no contador.

O contador é o piso garantido; o trait é o upgrade quando disponível.

### Ajuste 3 — O enriquecimento mora no `lente_infra`, ligado por flag

A decisão de acionar a E2 para enriquecer a nomeação fica no `lente_infra`
(L3), porque é ele quem tem acesso ao disco (lê os arquivos `.rs`) e quem
orquestra a sequência investiga → resolve.

- **Default**: enriquecimento desligado. O `lente_infra` não lê fontes, a
  nomeação usa contador. Rápido, sem I/O extra.
- **Ligado** (por flag/parâmetro): o `lente_infra` lê as fontes dos arquivos
  envolvidos nas colisões e aciona a E2 do `lente_investiga` para descobrir
  traits, passando o resultado ao `lente_resolve` para nomeação enriquecida.

O custo (ler fontes) fica onde a escolha está: quem quer nomes legíveis liga
o enriquecimento e paga; quem só quer o raio funcionando usa o default.

### Ajuste 4 — `MesmoItem` praticamente não ocorre em dados reais

Achado da medição: `MesmoItem` deu **0 em 384 colisões**. No JSON do fork
novo, cada cópia de um nó tem suas próprias arestas indexadas por id, então
vizinhança genuinamente idêntica praticamente não existe — mesmo para coisas
que conceitualmente poderiam ser "o mesmo item" (reexports, aliases), cada
cópia tem arestas `Owns`/`Uses` próprias.

Implicação para o `lente_resolve`: ele vai lidar quase sempre com
`Distintos`, raramente (ou nunca, nesta amostra) com `MesmoItem`. O caminho
`MesmoItem` (unificar nós) deve existir por completude, mas não é o caminho
comum. O caminho comum é `Distintos` (separar identidades).

Ressalva: este achado é contra os 17 crates do typst, pobres em reexport.
Crates com muito reexport (`hyper`, `tokio`) poderiam revelar `MesmoItem`.
Não medido. O `lente_resolve` mantém o caminho `MesmoItem` por segurança.

---

## Ajuste 5 — Limite 6 da spec: colisões em código gerado por macro

Os 10 casos `NaoDeterminado` restantes na 3ª medição (2,6%) são todos
`typst_macros::util::kw::<nome>` — código gerado por macro, em que o nó
colidente é um **módulo**, não um tipo, e a vizinhança tem 1 aresta
compartilhada (o `Owns` do módulo-pai).

Estes casos não são decididos:

- pela E1, porque o critério exige `compartilhadas == 0` e há 1 compartilhada;
- pela E2, porque não há `impl <Trait> for kw` no fonte (é macro-gerado).

**Decisão**: aceitar como **Limite 6 da spec** (a ser registrado em
`forma-organizada.md`). O critério rígido da E1 é mantido (não relaxado),
porque relaxá-lo para capturar estes 10 casos introduziria risco em cenários
não medidos (reexports onde `compartilhadas > 0` é sinal genuíno). Colisões
em código gerado por macro que produz nomes colidentes no mesmo módulo são
declaradas como caso fora do alcance da resolução automática, com diagnóstico
claro ao usuário.

A alternativa (relaxar o critério para fechar 100% na amostra) foi rejeitada:
ganho de 2,6% não justifica o risco de decisão de design sobre cenário não
medido. Coerente com o princípio do projeto de não tomar decisões de design
sobre dados que não existem.

---

## Consequências

**Positivas**:
- O `lente_resolve` tem justificativa empírica: 97,4% dos casos têm veredito
  acionável. Vale construí-lo.
- A nomeação por contador é simples, determinística e estável — não depende
  de descobrir traits, que raramente estão disponíveis.
- O enriquecimento opcional dá legibilidade a quem quer, sem onerar quem não
  quer.
- O critério rígido mantido evita decisão de design sobre dado ausente.

**Negativas**:
- A nomeação por contador (`método#1`, `método#2`) é menos legível que por
  trait. Para quem não liga o enriquecimento, os nomes são opacos sobre o
  que distingue cada cópia.
- A E2, que custou trabalho de implementação (laudo 0004), tem papel muito
  menor do que o desenho original previa. Não é desperdício (ela validou que
  o caminho de fontes existe, e pode servir crates de outras origens), mas é
  menos central.
- Os 10 casos de macro ficam sem resolução (Limite 6).

**Neutras**:
- A medição foi feita só contra typst. Generalização contra crates de outras
  origens fica como trabalho futuro (não bloqueia o `lente_resolve`).

---

## Prompts Afetados

| Prompt / artefato | Como este ADR o molda |
|-------------------|------------------------|
| Prompt do `lente_resolve` (futuro) | Nomeação por contador (ordem de id) como padrão; caminho `Distintos` é o comum, `MesmoItem` por completude; recebe trait opcional para enriquecimento. |
| Prompt de modificação do `lente_infra` (futuro) | Adiciona flag de enriquecimento; quando ligada, lê fontes e aciona a E2 para descobrir traits antes de chamar o `lente_resolve`. |
| `forma-organizada.md` | Recebe o Limite 6 (colisões em código gerado por macro, fora do alcance da resolução automática). |
| ADR-0004 | Permanece como está (a hipótese). Este ADR é o complemento empírico. |

---

## Referências

- ADR-0004 — a hipótese arquitetural que este ADR valida e ajusta
- `lab/medicao-colisoes/relatorio.md` — 1ª medição
- `lab/medicao-colisoes/remedicao/relatorio.md` — 2ª e 3ª medições (consolidado)
- Laudo 0006 — identidade-por-nó no lente_core e lente_infra
- Laudo 0008 — correção do ChaveAresta
- `LESSONS.md` lições L2 (oráculo de medição) e L3 (lab como bancada) —
  o método de validar arquitetura por medição empírica que este ADR exemplifica
