# Prompt: mapear o refactor V3+V12 antes de mover (investigação + plano, SEM código)

**Camada**: transversal (investigação) — no `tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0054-mapear_refactor_v3_v12.md`)
**Pré-requisito**: 0053 (V3 = 8 e V12 = 5 confirmados).
**Objetivo**: mapear a **forma real** do acoplamento CLI↔`lente_wiring` e dos enums
do V12 — onde cada símbolo é definido, se o `lente_wiring` **define** ou
**re-exporta**, quem depende — e produzir um **plano em estágios** para o refactor.
**SEM mudança de código** — só leitura e o plano. (Os estágios vêm depois, cada um
aprovado e executado à parte.)

---

## Contexto

O 0053 revelou o **V3 = 8** (a CLI `02_shell/cli`, L2, importa o `lente_wiring`,
L4) e o acoplamento com o **V12** (os enums no L4). O caminho acordado: **descer o
vocabulário do L4 para o L1** e **subir o ponto de entrada para o L4**. Mas é
refactor grande, atravessa crates e mexe no binário. **Antes de mover, mapear.**
Medir antes de afirmar.

Uma hipótese minha a **confirmar na fonte**: o `lente_wiring` **re-exporta** tipos
que na verdade nascem no L1 (ex.: `ResultadoDiff`, `TocadoComRaio` estavam no
`01_core/domain` no 0047), e a CLI os importa via essa fachada. Se for assim, parte
do V3 some só re-apontando a CLI para o L1 — sem mover nada.

E uma ressalva a verificar: nem todo o V12 desce. O **`ErroLente`** é erro
**agregado** — junta variantes do L3 (`Fork`, `Adaptador`). Se descer ao L1, o L1
passaria a referenciar o L3 (outra violação). Então o `ErroLente` provavelmente
**fica** no L4 (legítimo — erro agregado mora na composição), e a CLI deixa de
precisar dele pela relocação do ponto de entrada, não por mudar de camada.

---

## O que investigar (sem mudar nada)

1. **Cada símbolo que a CLI importa do `lente_wiring`** (os sítios do V3 do 0053 —
   `ErroLente`, `FonteGrafo`, `AlvoBusca`, `Escopo`, `ModoUses`, `ResultadoDiff`,
   `TocadoComRaio`, `RaioCombinado`, `Fantasma`, `EstruturaModulos`, `ItemRanking`,
   `Ciclo`, `DependenciaModulo`, e o que mais houver nos 8 sítios):
   - **Onde é DEFINIDO** (qual crate, qual camada)?
   - O `lente_wiring` **define** ou **re-exporta** (`pub use`)?
   - **Classificar**:
     - **(i) L1-origem re-exportado** → re-apontar a CLI para o L1 (Estágio 1, seguro).
     - **(ii) L4-nativo PURO** (não referencia outra camada) → mover para o L1 (Estágio 2).
     - **(iii) L4-nativo cross-layer** (ex.: `ErroLente`, que agrega L3) → **NÃO move**
       (fica L4 legítimo); a CLI deixa de precisar dele via Estágio 3.
2. **As funções de orquestração que a CLI chama** (`calcular_raio_de_alvo`,
   `analisar_diff`, `montar_grafo_workspace`, e quais mais): listar — são o que
   exige a relocação do ponto de entrada (Estágio 3).
3. **Dependentes do vocabulário L4-nativo**: quem mais usa
   `FonteGrafo`/`AlvoBusca`/`Escopo`/`ModoUses`/`ErroLente` além da CLI (as próprias
   funções do `lente_wiring` nos parâmetros, outros crates) — mover muda os imports
   deles, então mapear.
4. **`ErroLente` dissecado**: cada variante × a camada que ela referencia (quais
   trazem tipos do L3?) — confirma se pode ou não descer ao L1 (provável que não).
5. **Casa proposta no L1** para o vocabulário que move: onde? (Um módulo de contrato/
   entrada no `01_core/core`? Distribuir aos crates L1 relevantes? Um lugar novo?)
   **Propor com trade-offs** — é decisão de significado, sua.

---

## O plano a entregar (em estágios, com o delta esperado de V3/V12)

- **Estágio 1** — re-apontar a CLI dos tipos **L1-origem** (re-exportados pelo
  `lente_wiring`) para o L1 direto. Mecânico, preserva comportamento (mesmo tipo,
  outro caminho de import). Reduz o V3.
- **Estágio 2** — mover o vocabulário **L4-nativo puro** para o L1 (a casa
  escolhida). Resolve o V12 desses + os imports da CLI deles. Toca o `lente_wiring`
  (que usa esses tipos nos parâmetros) e os dependentes.
- **Estágio 3** — **relocar o ponto de entrada**: o `main` que chama a orquestração
  e trata o `ErroLente` sobe para o L4; a CLI vira **apresentação pura** (args,
  formatadores) que só importa L1. Resolve a parte de função do V3 + o import do
  `ErroLente`. **Decisão de significado a propor**: o binário "lente" passa a viver
  no `04_wiring` (L4 com o `[[bin]]`), ou num crate L4 novo? Trade-offs.
- **O que FICA**: o `ErroLente` (e qualquer L4-nativo cross-layer) permanece L4 —
  legítimo; o V12 dele se **declara intencional** (ou aceita como warning). Dizer
  quantos dos 5 do V12 movem e quantos ficam.

---

## O que NÃO fazer

- Mover nada, mudar import, mexer no binário — **esta passada é só o mapa**.
- Assumir o re-export sem **confirmar na fonte**.
- Assumir que **todo** o V12 move — o `ErroLente` provavelmente fica.
- Decidir a casa do vocabulário ou a forma do ponto de entrada por conta própria —
  **propor** opções com trade-offs; a escolha é sua.

---

## Critérios de Verificação

```
Dado cada símbolo que a CLI importa do lente_wiring
Então está classificado: L1-origem re-exportado / L4-nativo puro / L4-nativo cross-layer
(com onde é definido e se o lente_wiring define ou re-exporta)

Dado as funções de orquestração que a CLI chama
Então estão listadas

Dado o vocabulário L4-nativo
Então seus dependentes (além da CLI) estão mapeados

Dado o ErroLente
Então suas variantes × camadas estão dissecadas (pode descer ao L1, ou não?)

Dado o plano
Então tem 3 estágios com o delta de V3/V12 por estágio, a casa proposta no L1, a
forma proposta do ponto de entrada, e quantos V12 movem vs ficam

Dado o código
Então NADA mudou — só leitura
```

---

## Resultado esperado

- A **tabela de símbolos** (onde definido · define/re-exporta · classificação).
- A **lista de funções de orquestração** chamadas pela CLI.
- O **mapa de dependentes** do vocabulário L4-nativo.
- O **`ErroLente` dissecado** (variantes × camadas).
- A **casa proposta no L1** (com trade-offs) + a **forma proposta do ponto de
  entrada L4** (com trade-offs).
- O **plano em 3 estágios** com o delta de V3/V12 por estágio e o veredito do V12
  (quantos movem, quantos ficam).
- **Laudo** em `00_nucleo/lessons/0054-…`.

---

## Cuidados

- **O re-export é hipótese minha** — confirmar na fonte (o `lente_wiring` define ou
  re-exporta cada símbolo?). Se eu estiver errado, o plano muda.
- **O `ErroLente` (e cross-layer) provavelmente não desce ao L1** — não forçar; se
  forçar, vira violação L1→L3. É o caso onde o V12 é legítimo.
- **Decisões de significado são suas** (casa do vocabulário, forma do ponto de
  entrada) — o prompt as propõe, você decide antes dos estágios.
- **Sem código** — é o mapa antes do movimento; os estágios vêm depois, cada um
  preservando comportamento, suíte verde, lint re-checado.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Mapa do refactor V3+V12 antes de mover (decisão do 0053: descer o vocabulário do L4 ao L1, subir o ponto de entrada ao L4). Investigação **sem código**: classificar cada símbolo que a CLI importa do `lente_wiring` (L1-origem re-exportado → re-apontar; L4-nativo puro → mover ao L1; L4-nativo cross-layer como `ErroLente` → fica L4 legítimo), listar as funções de orquestração chamadas, mapear dependentes do vocabulário, dissecar o `ErroLente` (variantes × camadas — provável que não desça por agregar L3). Confirmar na fonte a hipótese do re-export (`lente_wiring` define vs `pub use`). Entregar plano em 3 estágios (1: re-apontar L1-origem; 2: mover vocabulário puro L4→L1 na casa escolhida; 3: relocar o ponto de entrada — binário sobe ao L4, CLI vira apresentação pura) com delta de V3/V12 por estágio e veredito do V12 (quantos movem vs ficam). Decisões de significado propostas com trade-offs (casa do vocabulário no L1; forma do ponto de entrada L4) para você decidir antes dos estágios. Nenhum código tocado. | `00_nucleo/lessons/0054-...` |
