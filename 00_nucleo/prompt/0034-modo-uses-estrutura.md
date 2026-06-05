# Prompt: Modo de `uses` no `--estrutura` (`todas` vs `--so-referencia`)

**Camada**: L1 (`lente_core` + `lente_filtro`) + L4 (fiação) + L2 (CLI + catálogo)
**Criado em**: 2026-06-04
**Estado**: `PROPOSTO`
**Decisões de origem**:
- Laudo 0033 — o SCC de 85 módulos do egui é **51% inflação por `import`**
  (Limite 4) e **49% acoplamento de tipo real** (42 módulos). Contar só
  `reference` derruba o SCC de 85 para 42. A medição justificou tornar isso uma
  **capacidade** do produto (hoje só existe num programa de Arena).
- Laudo 0030 (escopo) — o **molde**: escolha do usuário, default mostra tudo,
  opção filtra, **a saída declara o modo**. Aqui a escolha é quais arestas `uses`
  contam: `todas` (a vista do laudo 0031) ou só `reference` (o acoplamento de
  tipo genuíno).
- O fork emite `uses_kind` por aresta `uses` (`reference`/`import`; **sem**
  `reexport` distinto — laudo 0033). O `desserializar_grafo` **tolera** o campo
  hoje (serde sem `deny_unknown_fields`, confirmado pelo E2E do laudo 0033 D4);
  esta mudança o faz **ler** o campo, de modo aditivo.
**Pré-requisito**: o fork atualizado **instalado** (emite `uses_kind`); laudos
0031 (`lente_estrutura`), 0033 (o número), 0030 (o padrão de escopo, espelhado
aqui).
**Posição**: promove o achado medido (0033) a recurso — `lente --pacote X
--estrutura --so-referencia` em qualquer projeto, sem reconstruir um programa de
medição. Fecha o arco da trilha global.
**Arquivos afetados (a confirmar na Fase 1)**: `01_core/src/entities/grafo.rs`;
`03_infra/src/traducao.rs` (desserialização); `07_filtro/src/lib.rs`;
`04_wiring/src/lib.rs`; `02_shell/cli/src/*`; `02_shell/catalogo/src/lib.rs`;
testes. `lente_estrutura` (L1) **não muda**.

---

## Contexto

O laudo 0033 mediu, em Arena, que metade do "ciclo gigante" do egui era artefato
de `import` (Limite 4) e metade é acoplamento de tipo real. O filtro que produziu
isso (contar só `reference`) deve virar uma escolha do produto, no **mesmo molde
do escopo** (laudo 0030): o usuário escolhe, o default mostra tudo, a opção
filtra, e **a saída declara qual modo** — para que ninguém confunda os 42 com os
85 sem saber qual está vendo.

O pré-requisito (que o laudo 0033 nomeou): a `Aresta` precisa **carregar** o
`uses_kind`, lido do JSON do fork. É mudança **aditiva** — o desserializador já
tolera o campo (não quebra com ele); agora passa a lê-lo. JSONs antigos (sem o
campo) → `uses_kind` ausente.

**Estrutura espelha o escopo do 0030**: assim como o filtro de stdlib é uma
transformação de grafo aplicada na fiação **antes** do pipeline, o filtro
"só-referência" é uma transformação de grafo aplicada **antes** do
`agregar_por_modulo`. Resultado: o `agregar_por_modulo`/`detectar_ciclos` do laudo
0031 ficam **intocados** — recebem um grafo já filtrado e agregam como sempre.

---

## Restrições estruturais

- **Aditivo e retrocompatível.** Ler `uses_kind` não quebra nada; JSON sem o
  campo → `None`; testes existentes passam.
- **O `lente_estrutura` (L1) não muda.** O filtro novo é transformação de grafo
  (como `filtrar_stdlib`), aplicada na fiação antes do `agregar_por_modulo`.
- **Enum forte** (`ModoUses`), não `bool`/string nas assinaturas internas
  (preferência do projeto); a flag CLI mapeia para o enum.
- **A saída declara o modo** (texto e JSON), como o 0030 declara o escopo. O
  escopo (0030) continua declarado também.
- **`raio` e `ranking` não são afetados** — não leem `uses_kind`; acrescentar o
  campo opcional na `Aresta` não muda o comportamento deles.
- **Default `Todas`** (espelha o 0030: mostra tudo, opção filtra). A flag muda
  para `SoReferencia`. (Inverter o default é decisão do autor — ver Observação.)
- **Não tocar o fork** (pronto), a **spec**, nem a **E2**. **Não** estender o
  filtro de referência ao `raio`/`ranking` (decisão separada, da trilha local,
  e dependente de medição própria).

---

## Fase 1 — Leitura e confirmação

1. **Ler**: `Aresta` (`grafo.rs`); a desserialização de arestas
   (`traducao.rs`); `filtrar_stdlib` (`07_filtro`) como molde do filtro novo;
   `analisar_estrutura` + o modo `--estrutura` na CLI (laudo 0031); o catálogo; e
   a fiação do escopo (laudo 0030) como espelho — como `Escopo` flui e como a
   saída o declara.
2. **Confirmar** que o JSON de aresta do fork traz `uses_kind`
   (`"reference"`/`"import"`), e que o desserializador o tolera hoje (laudo 0033
   D4). Ler passa a ser aditivo.
3. **Confirmar o caso `None`**: fork antigo / JSON antigo → `uses_kind` ausente.
   Decidir o comportamento de `--so-referencia` quando ausente (ver §5 — não
   silenciar).

**Reportar**: o ponto de leitura na desserialização, onde o filtro entra na
fiação, e o tratamento do caso `None`.

---

## Fase 2 — Conserto

### 1. `lente_core` (`grafo.rs`)

```rust
pub enum UsesKind { Reference, Import }   // sem Reexport (o fork funde em Import)

// em Aresta:
pub uses_kind: Option<UsesKind>,   // Some para arestas uses; None para owns e p/ JSON antigo
```

### 2. `lente_infra` (`traducao.rs`)

A desserialização de aresta passa a **ler** `uses_kind`:
`"reference"` → `Reference`; `"import"` → `Import`; ausente → `None`; qualquer
outro valor presente → `Import` (funde, como o fork faz hoje; se o fork um dia
distinguir `reexport`, trata-se então). Aditivo — nada mais muda.

### 3. `lente_filtro` (`07_filtro`)

```rust
pub fn filtrar_so_referencia(grafo: &Grafo) -> Grafo
```
Mantém **todas** as arestas `owns` (o `agregar_por_modulo` precisa delas para
achar o módulo contenedor) e **só** as `uses` com `uses_kind == Some(Reference)`;
descarta as `uses` com `Import`. Mesma forma do `filtrar_stdlib` (grafo→grafo).
(Tratamento de `None`: ver §5.)

### 4. `lente_wiring`

```rust
pub enum ModoUses { Todas, SoReferencia }   // nome a confirmar; mora aqui ou em core
```
`analisar_estrutura(fonte, escopo, modo_uses)`: `obter_grafo(fonte, escopo)` →
**se `SoReferencia`**, `filtrar_so_referencia(&g)` → `agregar_por_modulo` →
`detectar_ciclos`. Reusa o `obter_grafo` do laudo 0030 (escopo). Re-exporta
`ModoUses` para a CLI (padrão dos laudos 0027/0030).

### 5. CLI (`02_shell/cli`) + catálogo

- Flag `--so-referencia` (booleana; **ausente = `Todas`**, presente =
  `SoReferencia`). Mapeia para `ModoUses`. Faz sentido **com `--estrutura`**.
  (Nome a confirmar; alternativa `--modo-uses <todas|so-referencia>`.)
- **A saída declara o modo** (texto e JSON), ao lado do escopo:
  - Texto: cabeçalho do `--estrutura` inclui o modo (ex.: `Estrutura de módulos
    (escopo: completo, uses: so-referencia) — N módulos, C ciclos:`).
  - JSON: campo `modo_uses` (`"todas"`/`"so-referencia"`) no topo.
- **Caso `None` (fork antigo)**: se `--so-referencia` foi pedido **e** as arestas
  `uses` vêm sem `uses_kind` (o fork instalado é antigo — situação que o laudo
  0033 D3 encontrou na prática), **não silenciar** produzindo `Todas`
  disfarçado. Emitir um **diagnóstico claro**: o fork instalado não emite
  `uses_kind`; atualize-o. (No espírito do diagnóstico de diretório-inexistente
  do laudo 0024 — erro claro em vez de comportamento errado mudo.)
- Rótulos no **catálogo**.

---

## Critérios de Verificação

```
Dado --estrutura sem flag (default)
Então modo = todas; reproduz a vista do laudo 0031 (egui: SCC de 85);
  a saída declara "uses: todas"

Dado --estrutura --so-referencia
Então reproduz o número do laudo 0033 (egui: SCC de 42);
  a saída declara "uses: so-referencia"

Dado um grafo desserializado do fork atual
Então cada aresta uses carrega uses_kind (Reference/Import); arestas owns têm None

Dado filtrar_so_referencia
Então mantém todas as owns + as uses Reference; descarta as uses Import

Dado raio e ranking
Então comportamento inalterado (ignoram uses_kind) — não-regressão

Dado --so-referencia com um fork antigo (uses sem uses_kind)
Então diagnóstico claro (atualize o fork), não Todas silencioso

Dado escopo (--filtrar-stdlib) junto com o modo
Então o escopo continua funcionando e declarado; saída declara escopo E modo_uses

Dado o egui (E2E #[ignore])
Então --estrutura → SCC 85; --estrutura --so-referencia → SCC 42 (ancorado em 0031/0033)
```

Casos a cobrir:

- **Unidade**: desserialização popula `uses_kind` (reference/import/ausente→None);
  `filtrar_so_referencia` (mantém owns + reference, descarta import); o modo flui
  pela fiação; a saída (texto e JSON) contém `modo_uses`.
- **Não-regressão**: `raio`/`ranking` idênticos; `lente_estrutura` intocado;
  escopo (0030) intacto; testes existentes verdes.
- **E2E `#[ignore]`**: egui nos dois modos (85 / 42), ancorado nos laudos.
- **Diagnóstico**: `--so-referencia` sem `uses_kind` → erro claro.

---

## Resultado esperado

- `--estrutura --so-referencia` vira capacidade: roda em qualquer projeto e
  mostra os ciclos de acoplamento de tipo genuíno (vs todas-as-uses), com o modo
  **declarado** na saída.
- A medição do laudo 0033 deixa de ser Arena de uma vez e vira recurso.
- `raio`/`ranking` intocados; `lente_estrutura` intocado; escopo (0030) intacto.
- **Laudo** registrando: o ponto de leitura do `uses_kind`, a reprodução dos
  números (85 / 42) nos dois modos, e o diagnóstico do fork antigo.

---

## O que NÃO entra

- **Estender o filtro de referência ao `raio`/`ranking`**: decisão separada, da
  trilha local, e dependente de medição própria. Aqui é só o `--estrutura`.
- **DSM visual**: consome isto depois.
- **Inverter o default para `SoReferencia`**: decisão do autor (ver Observação);
  o prompt entrega default `Todas`, espelhando o 0030.
- **Tocar o fork** (pronto), a **spec**, a **E2**, o **filtro de folhas**.
- **`reexport` como terceiro valor**: o fork funde em `import`; se um dia
  distinguir, estende-se então.

---

## Observação metodológica

Isto promove um achado **medido** (laudo 0033) a capacidade, no molde exato do
escopo (laudo 0030): escolha do usuário, default mostra tudo, opção filtra, a
saída declara o modo. É o "dados primeiro, conclusão por quem decide" estendido
ao usuário final — a lente não decide se você quer a vista inflada (85) ou a
genuína (42); entrega as duas e **diz qual**. A medição ganhou o recurso;
construí-lo antes do número teria sido especulação.

Sobre o default: entrego `Todas` por consistência com a sua decisão do laudo 0030
(não limitar; opção liga o foco). Se você preferir que o `--estrutura` abra já no
acoplamento real (`SoReferencia` por padrão, opção `--todas` para o completo), é
inverter o default — mesma estrutura, decisão sua, como foi lá. (Argumento a
favor de inverter, se quiser pesar: a vista de 42 é a acionável para refatoração,
que foi o seu uso declarado; a de 85 carrega ruído de import. Argumento a favor
de manter: consistência com o escopo, onde o default mostra tudo.)

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Modo de `uses` no `--estrutura`. `Aresta` ganha `uses_kind: Option<UsesKind>` (lido do JSON do fork no L3, aditivo); `lente_filtro::filtrar_so_referencia` (mantém owns + uses Reference, descarta Import); `ModoUses {Todas, SoReferencia}` na fiação, aplicado em `analisar_estrutura` antes do `agregar_por_modulo` (que fica intocado, como `lente_estrutura` todo); flag `--so-referencia` (default Todas); a saída declara `modo_uses` ao lado do escopo. Diagnóstico claro se `--so-referencia` com fork antigo (sem `uses_kind`). `raio`/`ranking` inalterados; escopo (0030) intacto. Promove o achado do laudo 0033 (85→42) a capacidade, no molde do laudo 0030. | `01_core/src/entities/grafo.rs`, `03_infra/src/traducao.rs`, `07_filtro/src/lib.rs`, `04_wiring/src/lib.rs`, `02_shell/cli/src/*`, `02_shell/catalogo/src/lib.rs`, `00_nucleo/lessons/0034-modo-uses-estrutura.md` |
