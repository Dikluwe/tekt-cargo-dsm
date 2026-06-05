# Prompt: Modo ranking — top-N por impacto (consumidor do `lente_filtro`)

**Camada**: L1 (cálculo puro) + L4 (fiação) + L2 (CLI + catálogo)
**Criado em**: 2026-06-02
**Estado**: `PROPOSTO`
**Decisões de origem**:
- Laudo 0021, Bloco C/D — sysroot domina os rankings; o modo ranking é o
  **consumidor** que motivou o filtro de stdlib (prompt 0025, já feito).
- A Arena `lab/medicao-egui/src/main.rs` **já prototipou** o ranking: para cada
  path, `calcular_raio`, junta `(path, montante.len())`, ordena decrescente,
  corta no top-N. Este prompt promove esse protótipo a componente de verdade.
**Pré-requisito**: prompt 0025 (`lente_filtro::filtrar_stdlib`); `lente_resolve`
(torna paths únicos); `lente_core::domain::raio::calcular_raio`; o wiring
existente (`calcular_raio_de_alvo`, que já faz extrair+resolver).
**Posição**: pendência 2 do laudo 0021, fechada. Fecha também a verificação do
Limite 2 no egui que ficou aberta no laudo 0025 (ver Fase 1).
**Arquivos afetados (a confirmar na Fase 1)**: cálculo L1 (crate novo
`08_ranking/lente_ranking` **ou** função em `lente_core::domain` — decidir na
Fase 1); `04_wiring/src/lib.rs`; `02_shell/cli`; `02_shell/catalogo`;
`Cargo.toml` raiz (se crate novo); testes.

---

## Contexto

A lente hoje responde por nó ("o que quebra se eu mexer no nó N?"). O ranking
responde "onde estão os nós mais impactantes deste crate?": para cada nó,
quantos dependem dele (montante), ordenado, top-N. É o uso que precisa do
filtro — sem ele, o top-N vem cheio de sysroot (laudo 0021: 7/10 do egui).

Dois fatos que tornam o desenho direto e seguro:

1. **A resolução torna os paths únicos.** O `lente_resolve` renomeia nós
   colidentes (`path#1`/`path#2` ou `Tipo::<Trait>::metodo`). Depois dela, cada
   nó tem path único, então o `calcular_raio` (que é por path) não conflaciona.
   Rankear sobre o **grafo resolvido** é correto — a pendência "raio-por-id"
   **continua latente**, o ranking não a ativa.
2. **A Arena já provou que roda** no egui (a medição 0021 saiu daí). O custo é
   dominado pela extração do fork, não pelo laço do ranking.

O fluxo é: extrair → resolver → **filtrar (0025)** → rankear → apresentar.

---

## Restrições estruturais

- **Reusar extrair+resolver do wiring existente.** O `calcular_raio_de_alvo`
  já faz desserializar + detectar/resolver colisões. **Não duplicar** — fatorar
  um "obter grafo resolvido a partir da fonte" compartilhado, se ainda não
  houver, e usar nos dois (raio-de-alvo e ranking).
- **Filtrar antes de rankear.** `filtrar_stdlib` entra entre a resolução e o
  ranking. É aqui que o filtro do 0025 ganha consumidor.
- **Cálculo puro, sem pré-otimização.** Reusar `calcular_raio` por nó (a
  abordagem provada da Arena), mesmo que ele reconstrua os índices a cada
  chamada. A Arena mostrou que o custo é dominado pela extração. **Não** expor
  índice nem otimizar antes de medir necessidade (princípio "não estruturar
  antes do uso pedir"). Se um dia medir-se lento, otimiza-se então.
- **Apresentação só no L2** (catálogo), por ADR-0002 do Tekt. O L1 devolve
  dados (lista ordenada), não texto formatado.
- **Modo per-nó intacto.** `--alvo-id`/`--path` continuam como estão
  (não-regressão).
- **Não toca o fork, os tipos `Grafo`/`No`, o `raio`, nem a E2** (quarentena).

---

## Fase 1 — Leitura e verificação (obrigatória)

1. **Ler**: o laço de ranking da Arena (`lab/medicao-egui/src/main.rs`);
   `calcular_raio` (e confirmar que a resolução deixa paths únicos, fundamento
   do ranking por path); o wiring (`calcular_raio_de_alvo` — o trecho
   extrair+resolver a reusar); a estrutura do CLI (clap) e do catálogo.

2. **Fechar a verificação do Limite 2 no egui (aberta no laudo 0025).** O 0025
   verificou a sobreposição zero ("path em prefixo sysroot ∧ `trait_`/`trait_ref`
   preenchido") só no `lente_core`. O egui é o crate difícil (mais impls,
   incluindo cfg-conditional do laudo 0021, e o caso inverso "impl de trait do
   alvo **para** um tipo de stdlib", que o `lente_core` quase não tem). Rodar
   `filtrar_stdlib` sobre o grafo resolvido do egui e verificar:
   - a sobreposição "primeiro segmento do path ∈ sysroot ∧ `trait_`/`trait_ref`
     preenchido" — se **zero**, prefixo puro está sólido também no egui;
   - se aparecer um caso (um impl do alvo cujo path cai sob `core`/`alloc`/`std`),
     **relatar**: é o caso que justificaria a cláusula `trait_`/`trait_ref`
     (a "opção C" do 0025), que deixa de ser especulação. Decidir então se entra
     neste prompt ou em um próprio.

3. **Confirmar** que o custo do ranking é dominado pela extração (evidência da
   Arena) — para justificar não pré-otimizar.

**Reportar no laudo**: o resultado da verificação do egui (o número da
sobreposição), a decisão sobre a cláusula C, e onde o cálculo do ranking vai
morar (crate novo vs `lente_core::domain`).

---

## Fase 2 — Conserto

### L1 — cálculo puro

`rankear(grafo: &Grafo, n: usize) -> Vec<ItemRanking>` (nomes a confirmar),
puro:
- Para cada nó do grafo, `calcular_raio(grafo, &no.path)`; chave de ordenação =
  `montante.len()` (quantos dependem dele).
- Ordena **decrescente por impacto**; desempate **determinístico** por path
  ascendente (testes precisam de ordem estável).
- Corta no top-`n`.
- `ItemRanking` carrega ao menos: path, impacto (`montante.len()`), e a
  `Classificacao` do `Raio` (Base/Intermediário/Folha/Isolado) — o ranking
  naturalmente sobe os `Base`.

Onde mora: crate novo `08_ranking/lente_ranking` (dep só `lente_core`, padrão
de `investiga`/`resolve`/`filtro`) **ou** função em `lente_core::domain`.
Decidir na Fase 1; se crate novo, somar aos `members`.

### L4 — fiação

Função pública nova (ex.: `rankear_pacote(fonte, n) -> Result<Vec<ItemRanking>, ErroLente>`):
- Reusa o "obter grafo resolvido a partir da fonte" (fatorado do
  `calcular_raio_de_alvo`).
- Aplica `lente_filtro::filtrar_stdlib`.
- Chama o `rankear` do L1.
- Variante de `ErroLente` nova só se necessário.

### L2 — CLI + catálogo

- **CLI** (clap): flag de modo ranking (ex.: `--ranking`, com `--top N`
  opcional, default 10). Conflita com `--alvo-id`/`--path` (um rankeia o crate,
  o outro consulta um nó) — expressar o conflito no clap, com mensagem clara.
- **Catálogo**: template de saída do ranking (lista: posição, path, impacto,
  classificação). A formatação é exclusiva do L2.

---

## Critérios de Verificação

```
Dado um grafo sintético com impactos conhecidos
Quando rankear(grafo, n)
Então a ordem é decrescente por impacto, desempate por path, cortada em n

Dado um grafo com nós de sysroot e nós do alvo
Quando o pipeline de ranking roda (com filtrar_stdlib)
Então nenhum nó de sysroot aparece no ranking (o filtro removeu-os)

Dado empates de impacto
Quando rankear roda
Então a ordem é determinística (path ascendente como desempate)

Dado o modo per-nó (--alvo-id / --path)
Quando usado
Então funciona como antes (não-regressão)

Dado --ranking junto com --alvo-id
Quando a CLI parseia
Então erro de conflito claro (clap)

Dado o egui resolvido e filtrado (E2E #[ignore])
Quando rankeado
Então o top-N não tem sysroot, e um nó-base conhecido do egui aparece
  (ex.: o tipo que a Arena destacou)
```

Casos a cobrir:

- **Unidade, puros** (grafos à mão): ordem, desempate, corte em n, classificação
  no item, grafo vazio, n maior que o número de nós.
- **E2E `#[ignore]`** sobre o egui real: top-N sem sysroot; nó-base conhecido
  presente; comparar com o top-10 que a Arena registrou no laudo 0021 (ancoragem
  histórica, banda não número exato — o fork pode variar).
- **Não-regressão**: suíte verde; modo per-nó inalterado; dois subprocessos do
  cargo, cada um único (invariante 0023).

---

## Resultado esperado

- Modo ranking funcional ponta-a-ponta: `lente --pacote X --ranking` devolve o
  top-N por impacto, **sem sysroot** (o filtro do 0025 em uso).
- Cálculo puro no L1, fiação no L4 (reusando extrair+resolver), apresentação no
  L2.
- Verificação do Limite 2 no egui fechada (a pendência do laudo 0025).
- Arena promovida a componente; pode então ser aposentada ou mantida como
  registro de medição (convenção lessons/ + lab/, candidato a LESSON do laudo
  0021).
- **Laudo** registrando: onde o cálculo ficou, o resultado da verificação do
  egui, a ancoragem contra o top-10 da Arena, e os números observados.

---

## O que NÃO entra

- **`calcular_raio` por id (pendência "raio-por-id")**: não. A resolução deixa
  paths únicos; o ranking por path é correto. A pendência segue latente.
- **Pré-otimização do ranking** (índice único): não — a Arena provou que roda;
  custo dominado pela extração.
- **Filtro de "folhas comportamentais" (Limite 3)**: outra pendência.
- **Remoção da E2**: quarentena.
- **Mudança no fork ou nos tipos `Grafo`/`No`.**

---

## Observação metodológica

Este prompt promove um protótipo de **Arena** (`lab/medicao-egui`) a
componente de produto — instância da convenção "experimentos de Arena também
ganham entrada em lessons/" (candidato a LESSON do laudo 0021). E dá ao filtro
do 0025 o seu consumidor: só agora o filtro é exercido ponta-a-ponta, e por
isso a verificação do Limite 2 que faltava (no egui, o crate difícil) é feita
**aqui**, na Fase 1 — verificar contra o dado real no momento em que o dado
real passa a ser usado, em vez de antes (quando seria especulação) ou nunca.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-02 | Modo ranking (top-N por impacto): cálculo puro no L1 (`rankear`), fiação no L4 (extrair+resolver reusados, `filtrar_stdlib` aplicado antes), CLI `--ranking`/`--top`, template no catálogo. Promove o protótipo da Arena. Fecha a verificação do Limite 2 no egui (pendência do laudo 0025). | `08_ranking/lente_ranking/*` (ou `lente_core::domain`), `04_wiring/src/lib.rs`, `02_shell/cli`, `02_shell/catalogo`, `Cargo.toml` raiz, `00_nucleo/lessons/0027-ranking-top-n.md` |
