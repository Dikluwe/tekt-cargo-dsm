# Laudo de Execução — Prompt 0034 (Modo de `uses` no `--estrutura`)

**Camada**: L5 (laudo)
**Data**: 2026-06-04
**Prompt executado**: `00_nucleo/prompt/0034-modo-uses-estrutura.md`
**Estado**: `EXECUTADO` — o achado **medido** do laudo 0033 (85→42 SCC no
egui) virou capacidade do produto. `Aresta` ganha `uses_kind:
Option<UsesKind>`; `lente_filtro` ganha `filtrar_so_referencia`;
`analisar_estrutura` ganha parâmetro `modo_uses` com diagnóstico claro
para fork antigo; CLI `--so-referencia` (default `Todas`). **A saída
declara o modo** ao lado do escopo. 194 verdes (+18) + 20 ignored (+1
E2E); subprocessos do cargo continuam dois únicos (0023).

---

## A pergunta que motivou e a resposta entregue

> O acoplamento "real" entre módulos do `egui` é o de **85 módulos**
> (todas as `Uses`) ou o de **42 módulos** (só `Reference`)?

O laudo 0033 deu a medida em Arena. Este prompt **entrega a vista** ao
usuário, com a escolha rotulada:

```
$ lente --pacote egui --estrutura --text
Estrutura de módulos (escopo: completo, uses: todas) — 111 módulos, 1 ciclos:
  SCC de 85 módulos

$ lente --pacote egui --estrutura --text --so-referencia
Estrutura de módulos (escopo: completo, uses: so-referencia) — 111 módulos, 1 ciclos:
  SCC de 42 módulos
```

Os números do laudo 0031 (85) e do laudo 0033 (42) são **reproduzidos
pelo produto**, agora sem programa de Arena. Ancorado também por teste
E2E (`e2e_estrutura_egui_so_referencia_reproduz_42`) que afirma o `42`
literal contra o egui v0.34.3.

---

## Estrutura entregue

```
01_core/src/entities/grafo.rs
  + pub enum UsesKind { Reference, Import }   (com TryFrom; mapeia "reference"/"import")
  ~ Aresta { …, uses_kind: Option<UsesKind> }   (None p/ Owns e JSON antigo)

03_infra/src/dto.rs
  ~ ArestaDTO { …, uses_kind: Option<String> }   (#[serde(default)] — opcional)

03_infra/src/traducao.rs
  ~ ler uses_kind do DTO; mapear "reference"/"import"; valor desconhecido → Import;
    Owns sempre None.

07_filtro/src/lib.rs
  + pub fn filtrar_so_referencia(&Grafo) -> Grafo
      mantém Owns; mantém Uses cujo uses_kind == Some(Reference);
      descarta Import e None.

04_wiring/src/lib.rs
  + pub enum ModoUses { Todas, SoReferencia }   (Default = Todas)
  + ErroLente::ForkSemUsesKind   (diagnóstico do fork antigo)
  ~ analisar_estrutura(fonte, escopo, modo_uses) -> Result<EstruturaModulos, ErroLente>
      Detecta fork antigo (total uses > 0 ∧ todas com kind None) → ErroLente::ForkSemUsesKind.
      SoReferencia → filtrar_so_referencia antes de agregar.

02_shell/catalogo/src/lib.rs
  + HELP_SO_REFERENCIA, ERRO_FORK_SEM_USES_KIND
  + JSON_MODO_USES / MODO_USES_TODAS / MODO_USES_SO_REFERENCIA
  ~ ESTRUTURA_CABECALHO ganha {modo_uses}

02_shell/cli/src/args.rs
  + #[arg(long="so-referencia")] pub so_referencia: bool

02_shell/cli/src/saida.rs
  ~ formatar_estrutura(estrut, escopo, modo_uses, modo)
  ~ JSON: campo "modo_uses" no topo
  ~ Texto: cabeçalho "(escopo: …, uses: …)"

02_shell/cli/src/erro.rs
  ~ traduz ErroLente::ForkSemUsesKind via ERRO_FORK_SEM_USES_KIND

02_shell/cli/src/main.rs
  + escolher_modo_uses(cli) → ModoUses
  ~ run_estrutura propaga modo_uses
```

---

## Decisões tácitas

### D1 — `UsesKind` em `lente_core` (não em `lente_infra` ou módulo separado)

`UsesKind` é parte do **tipo de dados do grafo**, não da desserialização.
Vive em `01_core/src/entities/grafo.rs` ao lado de `Relation`. O
`TryFrom<&str>` espelha o de `Relation`: lista fechada, valor
desconhecido erra. Adicionar uma terceira variante (`Reexport`, se o
fork distinguir um dia) é mudança localizada.

### D2 — `Aresta.uses_kind: Option<UsesKind>` (não enum aninhado)

Alternativa rejeitada: `Relation::Uses(UsesKind)` com `Relation::Owns`
sem dado. Razão: rippling em **toda** a base que match'a `Relation`.
`Option<UsesKind>` é aditivo — só código que se importa precisa ler.
Custo: a invariante "`uses_kind = Some` ⇔ `relation = Uses`" vira
implícita, não imposta pelo tipo. A `traducao` e o `filtrar_so_referencia`
explicitam a regra (assertions de teste cobrem).

### D3 — Valor desconhecido no DTO mapeia para `Import`, não erro

Política do prompt: "se o fork um dia distinguir `reexport`, trata-se
então". Hoje o fork funde `reexport` em `import` (laudo 0033 D6). Para
não quebrar com forks levemente mais novos que adicionem uma variante
sem coordenação, valor desconhecido → `Import` (conservador). Teste
`uses_kind_desconhecido_e_fundido_em_import` ancora.

### D4 — `None` no `Uses` é descartado pelo filtro, mas detectado pela fiação

`filtrar_so_referencia` é puro: descarta tudo que não é `Reference`,
inclusive `None`. **Não** emite warning, não inspeciona o grafo todo.
A **fiação** (`analisar_estrutura`) detecta o caso pathológico de
"todas as `Uses` vêm sem `uses_kind`" e retorna
`ErroLente::ForkSemUsesKind`. Coerente com o laudo 0024 (diagnóstico
do diretório inexistente em vez de erro silencioso).

### D5 — Default `ModoUses::Todas`, espelhando o escopo (laudo 0030)

Mesmo padrão do prompt 0030: **a lente mostra tudo por default**; o
usuário **opta** pelo recorte. A vista `Todas` preserva o que o laudo
0031 estabeleceu como base (85 no egui). `--so-referencia` é
*adicional*, opt-in, com **a saída declarando o modo** para nunca
haver dúvida do que está sendo visto.

Argumento que **não** prevaleceu (registrado para revisão futura):
"o default poderia ser `SoReferencia`, porque é a vista acionável para
refatoração". Contra: viola a consistência com o escopo (default
mostra tudo); embute uma escolha semântica no produto que o usuário
talvez queira inverter; e o laudo 0031 ficaria órfão. Manter `Todas`
default deixa o caminho aberto para futura inversão sem regressão.

### D6 — Saída declara o modo (texto E JSON)

JSON: `"modo_uses": "todas"|"so-referencia"` no topo, ao lado de
`"escopo"`. Texto: cabeçalho `(escopo: …, uses: …)`. Princípio do
laudo 0030 estendido: a lente **diz qual pergunta está respondendo**.
Quem comparar dois rankings (ou aqui, duas estruturas) com modos
diferentes vê a diferença explícita.

### D7 — Flag `--so-referencia` ortogonal (sem `conflicts_with`)

A flag tem **efeito** só com `--estrutura`, mas a CLI **não conflita**.
Razão: o usuário pode escrever `--so-referencia` por hábito e pedir
um `--ranking` numa segunda invocação; conflitar geraria erro
confuso. Outro modo (`--alvo`, `--ranking`) **ignora** a flag em
silêncio. Coerente com `--filtrar-stdlib` do laudo 0030, que segue o
mesmo padrão (ortogonal aos modos).

### D8 — Diagnóstico de fork antigo só dispara em modo `SoReferencia`

A detecção do fork antigo é feita **apenas** quando o usuário opta por
`SoReferencia`. Em `Todas`, o grafo é o mesmo do laudo 0031 — não há
sentido em alertar sobre `uses_kind` ausente, porque o produto não vai
usá-lo. Mesma economia que o laudo 0023 D5 fez sobre `NotFound`: erro
**no momento em que o caso é relevante**, não antes.

### D9 — Agregado (`agregar_por_modulo`) **perde** o subtipo intencionalmente

Uma aresta módulo→módulo agregada deriva de N arestas-de-item,
possivelmente de subtipos diferentes (algumas Reference, algumas
Import). O agregado **não preserva** o subtipo — é colapso natural da
abstração. Documentado como comentário em `09_estrutura/src/lib.rs:96`.
Não é regressão; é a propriedade que **`filtrar_so_referencia` rode
antes** da agregação, na ordem certa do pipeline.

### D10 — Não estender o filtro ao `raio`/`ranking`

Decisão explícita do prompt: ficar só no `--estrutura`. O `raio` e o
`ranking` operam por **path** e dependem do grafo todo para
classificação/montante; o filtro só-referência aqui mudaria semântica
sem medição própria. Trilha local separada.

---

## Verificação

### Suíte (sem ignored)

| Crate | Verdes | Δ vs 0033 |
|-------|--------|-----------|
| **lente_core** | **32** | **+2** (UsesKind try_from / desconhecido) |
| **lente_infra** | **35** | **+5** (uses_kind reference/import/ausente/desconhecido/owns intacto) |
| **lente_filtro** | **15** | **+5** (filtrar_so_referencia: preserva Reference, descarta Import/None; preserva owns/ids; idempotente) |
| **lente_wiring** | **18** | **+4** (estrutura_todas_uses, estrutura_so_referencia, fork_antigo_da_erro, fork_antigo_todas_funciona) |
| **lente_cli** | **35** | **+2** (json/texto da estrutura declarando modo_uses) |
| Outros | inalterados | 0 |
| **Total** | **194** | **+18** |

### Ignored (todos verdes)

| | Δ |
|---|---|
| lente_infra | 8 (inalterado) |
| lente_filtro (tests/) | 3 (inalterado) |
| **lente_wiring** | **6** (+1: `e2e_estrutura_egui_so_referencia_reproduz_42`) |
| lente_cli | 3 (inalterado) |
| **Total** | **20** (+1) |

Rodados todos: **17/17 verdes** (incluindo 4 E2Es contra `lente_core`/`egui`
real).

### Output real ancorado contra os laudos

| | Default `Todas` | `--so-referencia` |
|---|---|---|
| Módulos | 111 | 111 (idem) |
| Deps módulo→módulo | **864** | **386** (-55%) |
| Maior SCC | **85** | **42** |
| Reproduz | laudo 0031 ✓ | laudo 0033 ✓ |

JSON da CLI declara `"escopo": "completo", "modo_uses": "todas"` ou
`"so-referencia"` no topo, ao lado dos demais campos. Texto traz o
modo no cabeçalho.

### Subprocessos do cargo (invariante 0023)

```
$ grep -rn 'Command::new("cargo")' --include "*.rs"
03_infra/src/fork.rs:117      # cargo modules
03_infra/src/metadata.rs:170  # cargo metadata
```

Continuam dois únicos. O prompt 0034 não introduziu subprocesso.

### Compatibilidade com fork antigo (defesa em profundidade)

Teste `estrutura_todas_uses_com_fork_antigo_funciona` confirma:
- JSON antigo (sem `uses_kind`) + `Todas` → funciona normal (não-regressão).
- JSON antigo + `SoReferencia` → `ErroLente::ForkSemUsesKind` (diagnóstico).

O `desserializar_grafo` ignora o campo `uses_kind` em arestas `Owns`
mesmo se um fork hipotético o emitir (teste
`owns_nunca_carrega_uses_kind_mesmo_se_dto_emitisse`). Defesa em
profundidade vista do produto.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0034 |
|-----------|-----------------|
| Achado do laudo 0033 (85→42) | **Promovido a recurso** — `lente --pacote X --estrutura --so-referencia` |
| Limite 4 da spec (import como ponte) | **Endereçado** — agora separável pelo subtipo, não inferido |
| Diagnóstico de fork antigo em modo SoReferencia | **Coberto** com `ErroLente::ForkSemUsesKind` |
| Saída declara o que está mostrando | **Estendido** ao `modo_uses` (laudo 0030 cobria escopo) |
| Filtro só-referência no `raio`/`ranking` | **Aberta deliberadamente** — trilha local, requer medição própria |
| DSM visual | **Aberta** — consome o JSON do `--estrutura` (com novo campo) |
| `reexport` como variante distinta | **Aberta** — fork não distingue (laudo 0033 D6); se um dia distinguir, adicionar variante a `UsesKind` |
| Filtro de "folhas comportamentais" (Limite 3) | **Aberta** — trilha separada |

---

## O que NÃO mudou

- **`raio`/`ranking`** (modos `--alvo`/`--alvo-id`/`--ranking`): zero
  toques na lógica; ignoram `uses_kind`.
- **`lente_estrutura`** (L1): zero toques. O filtro novo é aplicado
  **antes** dele pela fiação — o módulo continua agnóstico.
- **Fork** (`cargo-modules`): zero toques. Usa o `uses_kind` que o
  fork já emite (commit `b44aa96`).
- **Spec, ADRs**: zero toques. O Limite 4 já estava declarado;
  agora ele é **endereçável**, não removido.
- **Subprocessos do cargo** (invariante 0023): dois únicos.
- **Default do `--estrutura`** sem flag: igual ao laudo 0031 (85
  módulos no egui).

---

## Observação metodológica

**Promoção de Arena a recurso, no molde do laudo 0030**. O ciclo
"medir antes de fazer" (laudos 0021/0027/0029/0030/0032/0033) entrega
aqui o melhor caso: a Arena mediu o efeito (-43 módulos), e o produto
incorporou a opção **com a forma exata** que o laudo 0033 sugeriu —
flag ortogonal, default conservador, saída declara o modo, diagnóstico
para o caso de borda.

A invariância do achado é importante de notar: o programa de Arena
do laudo 0033 e o produto que sai daqui **usam exatamente o mesmo
algoritmo** (`agregar_por_modulo` + `detectar_ciclos` do laudo 0031).
A única diferença foi onde o filtro `só-referência` mora: na Arena, no
parsing; no produto, depois da desserialização, antes do agregado.
**Mesma pergunta, mesma resposta**, agora endereçável por qualquer
usuário.

Coerente com o princípio "dados primeiro, conclusão por quem decide":
a lente entrega **as duas respostas** (85 e 42), **rotuladas**, e o
usuário decide qual está perguntando.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Modo de `uses` no `--estrutura`: `Aresta` ganha `uses_kind: Option<UsesKind>` (lido do JSON do fork no L3, aditivo); `lente_filtro::filtrar_so_referencia` (mantém Owns + Uses Reference, descarta Import/None); `ModoUses {Todas, SoReferencia}` na fiação aplicado em `analisar_estrutura` antes do `agregar_por_modulo` (que fica intocado); flag CLI `--so-referencia` (default Todas); saída declara `modo_uses` ao lado do escopo; `ErroLente::ForkSemUsesKind` com diagnóstico claro para fork antigo. Promove o achado do laudo 0033 (85→42) a capacidade, no molde do laudo 0030. Reproduzido contra o egui real: 85 com `--estrutura`, 42 com `--estrutura --so-referencia`. 194 verdes + 20 ignored; pureza do L1 mantida; dois subprocessos do cargo (0023). | `01_core/src/entities/grafo.rs`, `03_infra/src/{dto.rs,traducao.rs}`, `07_filtro/src/lib.rs`, `04_wiring/src/lib.rs`, `02_shell/catalogo/src/lib.rs`, `02_shell/cli/src/{args,saida,main,erro}.rs`, `00_nucleo/lessons/0034-modo-uses-estrutura.md` |
