# Laudo de Execução — Prompt 0002 (Cálculo do Raio de Impacto)

**Camada**: L5 (laudo)
**Data**: 2026-05-27
**Prompt executado**: `00_nucleo/prompt/0002-calculo_raio.md`
**Spec de origem**: `00_nucleo/specs/forma-organizada.md`
**ADRs aplicáveis**: 0002 (modelagem do grafo)
**Depende de**: laudo 0001 (tipo de dados da forma organizada)
**Estado**: `EXECUTADO` (compila, testes verdes, pureza L1 preservada)

---

## O que o prompt pediu

Sobre o `Grafo` da forma organizada, calcular o raio de impacto estrutural de
um nó-alvo:

- **Hierarquia de risco** (base vs. folha) sobre arestas `Uses`.
- **Alcance da propagação** (montante e jusante) com profundidade,
  transitivamente sobre `Uses`.
- **Distinguir `Uses` (consequência) de `Owns` (contexto hierárquico)**.
- Travessia segura com ciclos; alvo inexistente sem panic.
- Pureza L1: só stdlib, indexação à mão, sem `petgraph`.
- `kind` e `visibility` na interface, mas **não usados no cálculo** desta versão.

---

## O que foi gerado

| Arquivo | Propósito |
|---------|-----------|
| `01_core/src/domain/mod.rs` | Declara `pub mod raio`. |
| `01_core/src/domain/raio.rs` | Tipos `Raio`, `Classificacao`, `ErroRaio`; função `calcular_raio`; testes inline. |
| `01_core/src/lib.rs` (edit) | Adiciona `pub mod domain` ao lado de `entities`. |

**Verificação**:
- `cargo build`: limpo (0 warnings).
- `cargo test`: **22/22** (11 herdados + 11 novos do raio).
- `cargo tree`: `lente_core v0.0.0` sozinho — **pureza L1 preservada**.

---

## Decisões tácitas (registro para futura revisão)

### D1 — Estrutura única `Raio` agregando tudo

Em vez de separar em `HierarquiaDeRisco` + `Alcance` + `ContextoOwns`, um único
`Raio` com campos coesos: `classificacao`, contagens de grau (`uses_entrada`,
`uses_saida`), mapas de profundidade (`montante`, `jusante`), contexto Owns
(`owns_pai`, `owns_filhos`). Razão: o consumidor (futuro UI/CLI) lê tudo junto;
separar gera fricção sem ganho.

### D2 — Classificação por extremos puros, sem thresholds

`Classificacao` é enum estrita: `Isolado | Folha | Base | Intermediario`,
definida por zero/não-zero nas duas direções de `Uses`:

- `(0, 0)` → `Isolado`
- `(0, _)` → `Folha`  (ninguém depende dele)
- `(_, 0)` → `Base`   (ele não depende de ninguém, mas dependem dele)
- `(_, _)` → `Intermediario`

Sem corte arbitrário do tipo "≥ 5 entradas é base". Nuances ("muitos" vs.
"poucos") se leem nas próprias contagens. Mais honesto que inventar limiar.

### D3 — Alvo inexistente: `Err(ErroRaio::AlvoInexistente)`, não vazio

Erro é semanticamente distinto de "raio vazio" (que é resultado legítimo: um
nó isolado tem raio vazio). Enum `ErroRaio` (extensível para erros futuros)
implementa `Display` + `core::error::Error`. Nada de panic, nada de default.

### D4 — Travessia BFS com `HashSet<Path>` de visitados

BFS termina em ciclos por construção. Vantagem extra: BFS entrega
naturalmente o **caminho mais curto** (verificado pelo teste
`caminho_mais_curto_e_o_reportado`). Se houver dois caminhos para o mesmo nó,
a profundidade reportada é a do mais curto. Coerente com a noção de "raio".

### D5 — Indexação interna `Indices` (struct privada)

Construída uma vez por chamada de `calcular_raio` a partir das arestas do
`Grafo`. Quatro mapas: `uses_entrada`, `uses_saida`, `owns_pai`,
`owns_filhos`. Não exposta; usuários da biblioteca só veem `calcular_raio`.

### D6 — Clonagem de `Path` nos índices

Em vez de `&Path` com lifetimes, índices guardam `Path` clonados. Custo
aceitável para grafos medianos (dezenas de milhares de nós). Otimização
adiada se virar gargalo — proposta §10 ("uma coisa de cada vez"). A
indexação por `Path` (newtype `String`) é hashable porque derivamos `Hash`
em `Path` no L0.

### D7 — Owns como contexto direto (não transitivo)

`owns_pai: Option<Path>` (um pai por nó na árvore de contenção); `owns_filhos:
Vec<Path>` ordenado por path (determinismo de teste). **Não há cálculo de
ancestrais ou descendentes transitivos por Owns** — coerente com o prompt
("Owns serve para localizar/contextualizar"). Se algum dia for útil mostrar o
caminho hierárquico inteiro, é evolução, não esta versão.

### D8 — Alvo não entra em `montante`/`jusante`, mesmo em ciclos

A pergunta "o que depende de X" exclui X. Profundidade do alvo é implícita
(0); seria ruído nos mapas. Em grafos cíclicos isso significa que se A `uses`
B e B `uses` A, o `montante` de A contém apenas {B: 1}, não {A: 0, B: 1}.
Verificado por `ciclo_termina_e_inclui_alcance`.

### D9 — `kind` e `visibility` ignorados no cálculo

Recebidos via `Grafo` mas não consultados. Reservados conforme prompt: a
visibilidade poderá servir como teto de alcance ("um item `priv` não propaga
fora do seu módulo"), e `kind` para diferenciar tipos de nó na classificação.
Refinar isso depois **não muda a interface** (`Raio`/`calcular_raio`).

### D10 — Localização: `01_core/src/domain/`

O prompt diz `01_core/src/domain/raio.rs` (decisão pré-existente). `domain`
separa "regras" (cálculo) de `entities` (dados). Inspiração Clean Arch; não
contestada aqui.

### D11 — Métodos `profundidade_maxima_*` no `Raio`

`Raio::profundidade_maxima_montante()` e `_jusante()` — conveniências sobre
os mapas. Não custam interface (são consultas read-only). Úteis para a
hierarquia de risco em uma linha.

---

## Honestidade sobre Limite 4 (incorporada, não escondida)

O cabeçalho do arquivo declara explicitamente que o `montante` reflete o
piso de granularidade descrito pelo Limite 4 da spec: arestas `Uses` vindas
de `import` saem do **módulo**, não do **item** que importa. Quem ler o
`montante` deve estar ciente de que um path de módulo no resultado pode
significar "o módulo importa X, mas não sei qual item dentro dele usa X".

O cálculo **não tenta corrigir** isso — não pode, é limite da fonte
(`cargo-modules`). Quando esse piso virar dor concreta, a Nota de Evolução
da spec aponta o caminho: subtipos de `uses` no fork.

---

## Critérios de Verificação atendidos

| Critério (do prompt) | Status | Teste |
|----------------------|--------|-------|
| B→A, C→B; raio de A: B(1), C(2) | ✓ | `montante_inclui_direto_e_indireto_com_profundidade` |
| Folha tem montante vazio e é classificada como Folha | ✓ | `folha_tem_montante_vazio` |
| Muitos dependem, ele não depende → Base | ✓ | `base_e_classificado_quando_muitos_dependem_e_ele_nao_depende` |
| Owns sem Uses: consequência vazia, contexto (`owns_pai`) presente | ✓ | `owns_nao_propaga_consequencia_e_aparece_como_contexto` |
| Ciclo termina; vizinho do ciclo aparece no alcance | ✓ | `ciclo_termina_e_inclui_alcance` |
| Alvo inexistente → erro (não panic) | ✓ | `alvo_inexistente_retorna_erro` |
| Profundidade N em cadeia longa | ✓ | `cadeia_longa_reporta_profundidade_correta` |
| Grafo de um nó só → raio vazio | ✓ | `grafo_de_um_no_so_da_raio_vazio` |
| Nó isolado em grafo com outros → Isolado | ✓ | `no_isolado_em_grafo_com_outros_e_isolado` |
| BFS reporta caminho mais curto | ✓ | `caminho_mais_curto_e_o_reportado` |
| Owns filhos ordenados (determinismo) | ✓ | `owns_filhos_aparecem_ordenados` |
| Sem dependência externa | ✓ | `cargo tree` mostra só `lente_core` |

---

## O que o prompt explicitamente não pediu (não fiz)

- **Não filtrei stdlib.** Limite 2 da spec: filtro é componente L1 separado,
  prompt futuro.
- **Não usei `kind`/`visibility` no cálculo.** Reservados (D9).
- **Não calculei "raio de comportamento"**. Limite 3 da spec: lente é
  estrutural por desenho.
- **Não usei `petgraph` nem nenhuma biblioteca de grafos.** ADR-0002,
  Decisão 2: indexação à mão sobre stdlib.
- **Não exportei a struct `Indices`** — encapsulada.

---

## Próximos componentes (referência, não compromisso)

- **Filtro de stdlib** (L1) — recebe a forma completa, esconde `std::*` /
  `core::*` / `alloc::*` respeitando a fronteira do Limite 2 (preservar impls
  do crate-alvo).
- **Adaptador da fonte** (L3) — invoca o fork do `cargo-modules`, desserializa
  o JSON usando o tipo da forma organizada, validando os enums na borda.
- **Refinamento do cálculo** — usar `visibility` como teto de alcance e
  `kind` para classificação mais rica. **Sem mudar interface** (`Raio`
  permanece o mesmo).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Execução inicial do prompt 0002. Cálculo do raio: classificação + BFS sobre Uses, contexto Owns ao lado. 22 testes verdes; pureza L1 preservada. | `01_core/src/domain/mod.rs`, `01_core/src/domain/raio.rs`, `01_core/src/lib.rs` |
