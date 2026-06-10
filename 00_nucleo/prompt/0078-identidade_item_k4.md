# Prompt: pareamento por identidade de item no `--comparar` (chave K4)

**Camada**: L1 — Núcleo (`lente_comparacao`: o censo de itens, a chave, as
quatro categorias) + L4 — Fiação (alimentar o pareamento de itens nos dois
lados) + L2 — CLI (vista de texto agregada; JSON completo)
**Criado em**: 2026-06-10
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0078-identidade_item_k4.md`)
**Número**: confirmar na Fase 1 o próximo número livre em `00_nucleo/prompt/`
(o 0078 está reservado para a tela lado a lado; se este tomar o 0078, a tela
desliza — a convenção dos 0075/0076/0077).
**Decisões de origem**: laudo 0076 — o pareamento por path tem sinal **zero**
no par typst (0 pareados); a identidade de item é a única forma de o
`--comparar` responder a pergunta de origem ("o que falta migrar"). Laudo 0077
(Arena) — a chave foi escolhida **com dado**: K4 = `(kind, trait_,
pai-tipo::nome)` dá 1474 pareáveis (3,5× a chave ingênua), ambiguidade de 380
itens (resíduo = boilerplate de macro do `comemo`), dominando K3 em
desambiguação a custo zero (o `trait_` já está no nó). Decisão do autor
(2026-06-10): **K4**.
**Pré-requisito**: 0076 (censo member-only; proveniência por lado); 0077 (as
definições operacionais do censo de itens e da chave, validadas contra o dado
real — este prompt as **promove** da Arena ao produto); 0074 (o contrato
honesto: sem-par declarado, nada inferido).
**Posição**: o passo de valor da trilha do comparar — o que dá sinal sob
reorganização. Depois dele: a tela lado a lado lê o JSON.
**Arquivos afetados (a confirmar na Fase 1)**: `01_core/comparacao/src/lib.rs`
(censo de itens + chave + categorias), `04_wiring/src/lib.rs` (alimentação nos
dois lados), `02_shell/cli/src/saida.rs` + catálogo (vista agregada + JSON),
testes dos três.

---

## Contexto

O `--comparar` hoje pareia **módulos por path** — e no par typst isso mede a
profundidade da reorganização (0 pareados), não a paridade de conteúdo. Este
prompt adiciona o nível de **item**: structs, enums, funções, traits etc.
pareados por uma chave **independente de localização**, para que "o tipo X
existe nos dois lados, mudou de lugar" vire dado.

A chave e o censo **não são desenho novo** — são as definições K4 da Arena
0077, medidas contra o par real. Este prompt as transcreve para o produto sem
alterá-las; alterá-las invalidaria a medição que justificou a escolha.

O contrato de honestidade do 0074 se estende: além de pareados e sem-par dos
dois lados, a saída declara os **ambíguos** (chaves com mais de um candidato)
como categoria própria — o produto não adivinha correspondência.

---

## Restrições estruturais

- **L1 — censo, chave e categorização são puros.** Entram os dois grafos (já
  filtrados) e as listas de paths-fantasma por lado; sai a estrutura de
  resultado. Só stdlib + `lente_core`. `cargo tree -p lente_core` inalterado.
- **Aditivo.** O nível de módulo (0076) não muda — texto e JSON ganham a
  seção de itens ao lado da existente. Nenhum campo atual é removido ou
  renomeado.
- **Os dois modos.** A chave é independente de path, então o pareamento de
  itens roda em crate×crate e em workspace igualmente. No modo crate×crate
  não há fantasmas a excluir (lista vazia).
- **Sem flag nova.** A seção de itens entra na saída padrão do `--comparar`.
  Se o tamanho do JSON se mostrar problema na rodada real, **registrar** no
  laudo (com o tamanho) — conserto é decisão posterior, não deste prompt.

---

## Fase 1 — Leitura primeiro (obrigatória)

1. O código da Arena (`lab/medicao-chave-item/src/main.rs`): as definições
   operacionais de censo e chave **como rodaram** — este prompt as promove;
   a transcrição tem que ser fiel (mesmos kinds, mesma regra de pai-tipo,
   mesma exclusão de representantes). Divergência entre a Arena e o produto
   tornaria a medição 0077 inválida como justificativa — se a leitura
   mostrar que algo da Arena não cabe no L1 como está, **parar e voltar**.
2. A forma atual do resultado do comparar no L1 (`lente_comparacao`) e da
   saída na L2, para a extensão ser aditiva.
3. Onde as listas de fantasmas por lado já transitam (a proveniência do
   0075), para o L4 alimentá-las ao censo sem buscar de novo.
4. Confirmar o próximo número livre de prompt.

---

## O que construir

### 1. O censo de itens (L1 — a definição da Arena, promovida)

Para cada lado, do grafo já filtrado (sysroot + não-membros, 0076):

- Itens = nós com `kind` ∈ {`fn`, `struct`, `enum`, `union`, `variant`,
  `const`, `static`, `trait`, `type`, `macro`} — excluindo `mod`, `crate`,
  `builtin`.
- Excluir nós cujo path está na lista de fantasmas do lado (representantes
  são stubs de referência, não definições). A contagem de excluídos por lado
  vai à proveniência.

### 2. A chave K4 (L1 — a definição da Arena, promovida)

Para cada item:

```
chave = (kind, trait_, qualificador)
```

- `trait_`: o campo do nó; vazio quando `None` (a convenção `"" = ausente`
  que o projeto já usa).
- `qualificador`: se o **pai por `Owns`** é um tipo (struct/enum/trait/union)
  → `NomeDoPai::nome` (ex.: `Counter::get`); senão (pai é módulo/crate, ou
  sem pai) → só `nome`. **Nunca** qualificar com o nome do módulo — isso
  reintroduziria a dependência de path que o 0076 zerou.
- Determinístico; sem normalização além da que o dado já tem (nomes de item
  não carregam hífen).

### 3. As quatro categorias (L1)

Agrupando os itens dos dois lados por chave:

| Categoria | Definição |
|---|---|
| **pareados** | chave com exatamente 1 item em cada lado |
| **ambíguos** | chave presente nos dois lados, com >1 item em pelo menos um |
| **sem-par antes** | chave só no lado antes |
| **sem-par depois** | chave só no lado depois |

Cada **pareado** carrega os dois paths (antes e depois) — o consumidor vê o
movimento sem o produto inferir nada ("`Counter` existia em
`typst_library::introspection::counter`, existe em `typst_core::dominio`").
Cada **ambíguo** carrega a chave e os itens candidatos de cada lado.
Ordenação estável em todas as listas (determinismo, disciplina 0045).

### 4. A vista de texto agrega; o JSON carrega tudo (L2)

Listas de milhares de itens não se leem cruas. A vista de texto mostra:

- as quatro contagens;
- a distribuição por `kind` de cada categoria;
- o sem-par **agregado por crate** (1º segmento do path): "typst_library:
  4112 itens sem-par", não os 4112;
- a contagem de representantes excluídos por lado (da proveniência).

O JSON carrega as listas completas (pareados com os dois paths; ambíguos com
candidatos; sem-par com paths) — é o insumo da tela e de agentes. Campos
aditivos, strings no catálogo (ADR-0002).

---

## O que NÃO muda

- O pareamento de **módulos por path** (0075/0076) — intocado, ao lado.
- Os filtros (sysroot, não-membros), a união, o cache, a extração resiliente.
- `--diff`, `--estrutura`, o modo global.
- **As definições da Arena 0077** — promovidas, não alteradas.

---

## Critérios de Verificação

```
Dado dois grafos forjados onde o antes tem struct Counter em a::x e o depois
tem struct Counter em b::y::z (paths diferentes, pai-módulo)
Quando o pareamento de itens roda
Então Counter é pareado, carregando os dois paths

Dado um método fn get com pai-tipo Counter num lado e fn get com pai-tipo
Frame no outro
Quando o pareamento roda
Então não pareiam (qualificadores Counter::get e Frame::get diferem) e cada
um é sem-par do seu lado

Dado fn fmt com trait_ Display e fn fmt com trait_ Debug, ambos sob o mesmo
pai-tipo, num lado; e só o de Display no outro
Quando o pareamento roda
Então o de Display pareia e o de Debug é sem-par (o trait_ separa)

Dado uma chave com 2 itens no antes e 1 no depois
Quando o pareamento roda
Então a chave é ambígua, com os 3 candidatos declarados (nenhum pareado)

Dado um item cujo path está na lista de fantasmas do lado
Quando o censo roda
Então ele não entra (e a contagem de excluídos o registra)

Dado os mesmos dois grafos
Quando o pareamento roda duas vezes
Então a mesma saída (determinístico)

Dado o workspace da lente como --antes E como --depois (#[ignore], fork real)
Quando --comparar roda
Então sem-par de itens é vazio dos dois lados e os ambíguos são espelhados
(mesmas chaves, mesmos candidatos nos dois lados)

Dado o JSON do comparar pós-mudança
Quando desserializado
Então os campos de módulo (0076) estão presentes e a seção de itens é aditiva
```

Casos puros (sem fork): a chave (pai-tipo qualifica; pai-módulo não; trait_
separa; sem pai), as quatro categorias, a exclusão de fantasmas, o
determinismo. E2E `#[ignore]`: lente vs lente (identidade); retrocompat
crate×crate (seção de módulos idêntica à do 0076). A rodada typst entra no
laudo.

---

## Resultado esperado

- O `--comparar` emite, além do nível de módulo, o nível de item com as
  quatro categorias, chave K4, texto agregado e JSON completo.
- **Laudo** em `00_nucleo/lessons/`:
  - A transcrição fiel das definições da Arena (e qualquer adaptação que o
    L1 exigiu, com justificativa).
  - **A rodada typst** — o portão de verdade deste prompt: as quatro
    contagens do produto têm que **reproduzir a Arena 0077** (pareáveis
    1474, ambíguos 380 itens, sem-par 10128/1183, com a mesma exclusão de
    431/0 representantes). Divergência = bug ou definição infiel; investigar
    antes de aceitar.
  - O sem-par agregado por crate dos dois lados (a primeira vista real de
    "o que falta migrar" por área).
  - O tamanho do JSON da rodada typst (o dado para a decisão futura sobre
    paginação/flag, se precisar).
  - Contagem da suíte (era 305 verdes + 34 ignored no 0076).

---

## O que NÃO entra

- **Filtro de boilerplate de macro** (`__ComemoCall::*` e família) — o
  resíduo de ambiguidade do 0077; refinamento próprio, decidido depois com a
  saída real na mão.
- **Similaridade estrutural / assinaturas** — a trilha pesada continua fora.
- **Resolvedor de colisão / typst-macros** — a lacuna do lado antes segue
  declarada (os itens reais do crate ausentes), não consertada.
- **A tela lado a lado** — lê este JSON, no número seguinte.
- **Inferência de "renomeado"** (item que mudou de nome) — fora; a chave é
  exata, e o que ela não casa é sem-par declarado.

---

## Observação metodológica

A sequência completa desta trilha é o método do projeto de ponta a ponta:
o caso real quebrou o desenho (0075), o filtro devolveu a honestidade ao
número (0076), a Arena mediu as opções antes da escolha (0077), e este
prompt promove a definição **medida** — não uma inventada — ao produto, com
o portão de reproduzir a Arena como critério de aceitação. O custo de três
passos antes do "passo de valor" é o que garante que o primeiro número de
"o que falta migrar" nasça citável em vez de contaminado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-10 | Proposta: nível de item no `--comparar` — censo e chave K4 `(kind, trait_, pai-tipo::nome)` promovidos da Arena 0077 (decisão do autor com o dado: 1474 pareáveis, ambiguidade 380); quatro categorias declaradas (pareados com os dois paths, ambíguos com candidatos, sem-par dos dois lados); fantasmas excluídos do censo com contagem; texto agregado por crate, JSON completo; aditivo aos dois modos; laudo deve reproduzir os números da Arena na rodada typst. | a confirmar na Fase 1 |
