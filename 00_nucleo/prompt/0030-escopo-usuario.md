# Prompt: Escopo como escolha do usuário (`--filtrar-stdlib`) com a saída declarando o escopo

**Camada**: L4 (fiação) + L2 (CLI + catálogo)
**Criado em**: 2026-06-03
**Estado**: `PROPOSTO`
**Decisões de origem**:
- Laudo 0029, Achado 2 — o mesmo nó (`Path`) é classificado como **Base** no
  ranking (que filtra stdlib) e **Intermediário** no raio por nó (que não
  filtra). Não é bug; é **decisão omitida**: os dois modos respondem a perguntas
  diferentes sem dizer qual.
- Decisão do autor: **não limitar as respostas; dar a escolha ao usuário.**
  Default **completo** (com stdlib), com **opção de filtrar**. Aplicado aos dois
  modos, com o **mesmo default**, para que parem de divergir em silêncio.
**Pré-requisito**: laudos 0027 (`rankear_pacote`, `obter_grafo_resolvido`,
`calcular_raio_de_alvo`), 0025 (`lente_filtro::filtrar_stdlib`), 0029 (o
Achado 2 e os dumps).
**Posição**: conserta a incoerência semântica que o protótipo de UI revelou.
Pré-requisito de qualquer UI (ela mostra o escopo e oferece a troca).
**Arquivos afetados (a confirmar na Fase 1)**: `04_wiring/src/lib.rs`;
`02_shell/cli/src/{args,saida,main}.rs`; `02_shell/catalogo/src/lib.rs`; testes.
`lente_filtro` (L1) **não muda** — só passa a ser aplicado condicionalmente.

---

## Contexto

O conserto tem **duas metades**, e a segunda é a que resolve a confusão:

1. **Escopo vira parâmetro nos dois modos.** Hoje o ranking filtra **sempre** e
   o raio por nó **nunca** filtra. Os dois passam a receber um `escopo`. Default
   `completo` (não filtra); `--filtrar-stdlib` muda para `seu-codigo` (aplica
   `filtrar_stdlib`, que já existe).
2. **A saída declara o escopo.** Cada saída diz em que escopo foi calculada
   (`escopo: completo` ou `escopo: seu-codigo`). **Sem isso, a flag dá a escolha
   mas a divergência continua**: quem navega de um ranking para um raio precisa
   ver que o escopo é o mesmo (ou que mudou), em vez de ver "Base" virar
   "Intermediário" sem motivo aparente.

Um fato preciso que estreita o que muda (e que o executor deve confirmar): para
um nó do **seu código**, o **impacto** (o `montante`, "quem quebra se eu mexer
aqui") é o **mesmo** nos dois escopos — a stdlib não depende do seu código,
então filtrá-la não altera o montante. O escopo muda só duas coisas: a
**classificação** (Base vs Intermediário, que depende do que o nó *usa*, e o que
ele usa inclui ou não a stdlib) e **quais nós aparecem no ranking**. A resposta
central da lente não muda com o escopo; o escopo mexe na classificação e em quem
povoa o ranking.

**Consequência do default completo (intencional, não regressão)**: o ranking
sem flag volta a trazer sysroot no topo (a situação do laudo 0021);
`--filtrar-stdlib` recupera o ranking do laudo 0027 (Vec2/Color32/… sem
sysroot). É a escolha do autor; o executor deve tratar a saída com sysroot como
**esperada** no default, não como bug.

---

## Restrições estruturais

- **`lente_filtro` (L1) não muda.** Só passa a ser chamado condicionalmente.
- **Aplicação do escopo centralizada**, não duplicada — um único ponto decide
  filtrar-ou-não (um helper compartilhado pelos dois pipelines).
- **Os dois modos aceitam `escopo`** e aplicam **igual** (mesmo default).
- **A saída declara o escopo** em **JSON e texto**, nos **dois** modos.
- **Enum forte**, não `bool` solto nem string, para o escopo (preferência do
  projeto). A flag CLI mapeia para o enum.
- **Aditivo onde dá**, mas as assinaturas da fiação ganham `escopo` — a CLI é a
  única que as chama, então é mudança **contida**, não quebra de API pública. O
  **default do ranking muda** (de filtrado para completo); declarar isso no
  laudo.
- **Não toca o fork, os tipos `Grafo`/`No`, nem a E2** (quarentena).

---

## Fase 1 — Leitura e confirmação

1. **Fiação**: `calcular_raio_de_alvo`, `rankear_pacote`, `obter_grafo_resolvido`
   — onde o filtro entra (hoje só no ranking) e onde o alvo é resolvido (id→path).
2. **Saída**: `saida.rs` — `formatar_json` e `formatar_texto` dos dois modos
   (onde declarar o `escopo`).
3. **CLI**: `args.rs` (clap) e o roteamento de modo em `main.rs` (a flag é
   **ortogonal** a `--ranking`/`--alvo`/`--alvo-id`).
4. **Catálogo**: rótulos a acrescentar (chave JSON `escopo`, valores, texto do
   cabeçalho).
5. **Confirmar o fato**: que `filtrar_stdlib` remove só nós de sysroot e que
   esses não são montante de nós do alvo → o montante de um nó do alvo é o mesmo
   nos dois escopos. E o caso de borda: com `--filtrar-stdlib`, um alvo que é um
   nó de stdlib é **removido** → "alvo não encontrado" (consistente: você pediu
   para filtrar a stdlib **e** consultou um nó dela).

**Reportar no laudo**: a confirmação do montante invariante ao escopo, o ponto
escolhido para o helper, e o lugar onde o `escopo` é declarado na saída.

---

## Fase 2 — Conserto

### Enum de escopo

```rust
pub enum Escopo { Completo, SeuCodigo }   // nomes a confirmar
```
Mora na fiação (`04_wiring`), e a CLI usa `lente_wiring::Escopo` (padrão do
re-export do laudo 0027, D2) — **ou** em `lente_core` se a Fase 1 julgar melhor.

### Helper de escopo (ponto único)

```rust
fn obter_grafo(fonte: FonteGrafo, escopo: Escopo) -> Result<Grafo, ErroLente> {
    let g = obter_grafo_resolvido(fonte)?;
    Ok(match escopo {
        Escopo::SeuCodigo => lente_filtro::filtrar_stdlib(&g),
        Escopo::Completo  => g,
    })
}
```
Os **dois** pipelines passam a chamar `obter_grafo(fonte, escopo)`.

### Pipelines com `escopo`

- `calcular_raio_de_alvo(fonte, alvo, escopo)`: `obter_grafo(fonte, escopo)` →
  resolver alvo (id→path) → `calcular_raio`. (Com `Completo`, comportamento
  atual preservado; o filtro só entra com `SeuCodigo`.)
- `rankear_pacote(fonte, n, escopo)`: `obter_grafo(fonte, escopo)` → `rankear`.
  (Com `Completo`, **não** filtra — default novo; `SeuCodigo` recupera o 0027.)

Aplicar o filtro à forma resolvida; tudo depois (resolução do alvo,
`calcular_raio`, ranking) opera sobre o grafo já no escopo escolhido.

### CLI

- Flag `--filtrar-stdlib` (booleana; **ausente = `Completo`**, presente =
  `SeuCodigo`). Ortogonal a `--ranking`/`--alvo`/`--alvo-id`. (Nome a confirmar;
  alternativa: `--escopo <completo|seu-codigo>` se preferir explícito.)
- Mapeia para `Escopo` e passa à fiação nos dois modos.

### Saída declara o escopo

- **Raio JSON**: novo campo `escopo` (`"completo"`/`"seu-codigo"`).
- **Ranking JSON**: `escopo` no topo (o ranking inteiro é de um escopo).
- **Texto** (dois modos): o escopo no cabeçalho (ex.: `Ranking de impacto
  (escopo: completo) — top 10:`; e no raio, uma linha de escopo).
- Rótulos no **catálogo** (chave, valores, texto).

---

## Critérios de Verificação

```
Dado o raio por nó sem flag
Quando calculado
Então escopo = completo; classificação considera uses para stdlib; a saída declara "completo"

Dado o raio por nó com --filtrar-stdlib
Quando calculado
Então stdlib filtrada; o montante (transitivos) é IGUAL ao do modo completo;
  a classificação pode mudar; a saída declara "seu-codigo"

Dado o mesmo nó (ex.: Path) nos dois escopos
Então transitivos iguais; classificação difere (Intermediário no completo, Base no seu-codigo);
  e CADA saída declara seu escopo (Achado 2 resolvido: a divergência fica explicada, não silenciosa)

Dado o ranking sem flag
Quando calculado
Então escopo = completo; sysroot aparece no topo (esperado); a saída declara "completo"

Dado o ranking com --filtrar-stdlib
Quando calculado
Então o ranking do laudo 0027 (sem sysroot, Vec2 no topo); a saída declara "seu-codigo"

Dado os dois modos
Então têm o MESMO default (completo) — navegar ranking→raio no mesmo escopo dá classificação consistente

Dado --filtrar-stdlib com um alvo que é nó de stdlib
Então "alvo não encontrado" (consistente — foi filtrado)
```

Casos a cobrir:

- **Unidade/integração**: o helper `obter_grafo` filtra só em `SeuCodigo`;
  montante invariante ao escopo para um nó do alvo; a saída (JSON e texto)
  contém o `escopo` em ambos os modos.
- **Não-regressão**: o raio por nó no default **não muda de comportamento**
  (só ganha o rótulo). Os testes de ranking do laudo 0027 que assumiam filtro
  **mudam** para passar `Escopo::SeuCodigo` (ex.: `e2e_ranking_…_nao_traz_sysroot`);
  **adicionar** um teste do default completo (sysroot presente no ranking).
- **E2E `#[ignore]`**: ranking do egui nos dois escopos (completo com sysroot;
  seu-codigo sem) — ancorar contra o laudo 0027.

---

## Resultado esperado

- Escopo é escolha do usuário nos dois modos; default completo; `--filtrar-stdlib`
  filtra. A `lente_filtro` (L1) intacta, aplicada por um ponto único.
- **A saída declara o escopo** (JSON e texto, dois modos) — o Achado 2 deixa de
  ser divergência silenciosa e vira diferença **rotulada**.
- **Laudo** registrando: a confirmação do montante invariante ao escopo, a
  mudança de default do ranking, e a saída com `escopo` nos dois modos.

---

## O que NÃO entra

- **Mudar `filtrar_stdlib` (L1)**: nada — só é chamado condicionalmente.
- **Enriquecer o JSON do raio (profundidade/arestas — Achado 1 do laudo 0029)**:
  prompt próprio, decidido pelo que a UI pedir.
- **Nuclear a UI**: separado. (Opcional, fora do escopo obrigatório:
  re-capturar os dumps do `lab/proto-ui` já trará o campo `escopo`, e o
  protótipo pode mostrar um selo de escopo + botão de troca — mas isso é Arena,
  não exigido aqui.)
- **Filtro de folhas (Limite 3)** e **remoção da E2**: outras trilhas.

---

## Observação metodológica

O Achado 2 foi uma **decisão omitida** que só apareceu ao desenhar contra dado
real (o ganho lateral do protótipo, laudo 0029). O conserto **não é escolher uma
das respostas** — é tornar a escolha explícita **e rotular a saída**, para que as
duas respostas fiquem legíveis em vez de contraditórias. É o princípio "dados
primeiro, conclusão por quem decide" do projeto, estendido do autor para o
**usuário final**: a lente não decide qual pergunta você faz; ela responde a que
você pediu, e diz qual foi.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-03 | Escopo (`Completo`/`SeuCodigo`) como parâmetro dos dois modos via helper único na fiação; flag CLI `--filtrar-stdlib` (default completo); saída (JSON e texto) declara o `escopo` nos dois modos — conserta o Achado 2 do laudo 0029 (classificação divergente) tornando a diferença rotulada em vez de silenciosa. Default do ranking muda de filtrado para completo. `lente_filtro` intacta. | `04_wiring/src/lib.rs`, `02_shell/cli/src/{args,saida,main}.rs`, `02_shell/catalogo/src/lib.rs`, `00_nucleo/lessons/0030-escopo-usuario.md` |
