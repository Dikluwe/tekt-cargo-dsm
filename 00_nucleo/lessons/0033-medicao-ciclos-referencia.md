# Laudo de Execução — Prompt 0033 (Ciclos do egui só com `reference`)

**Camada**: L5 (laudo)
**Data**: 2026-06-04
**Prompt executado**: `00_nucleo/prompt/0033-medicao-ciclos-referencia.md`
**Tipo**: Arena — medição descartável; **dá o número** que decide o próximo
prompt de produto.
**Estado**: `EXECUTADO` — fork reinstalado do clone local (com `uses_kind`);
re-extração de `egui` e `lente_core`; sanidade reproduz o 85 do laudo 0031;
**SCC cai de 85 → 42 ao contar só `reference`** (−43 módulos, ≈51%).
Suíte de produção intacta (176 verdes + 19 ignored, mesma do laudo
0031/0032); fork novo retrocompatível com o desserializador (E2E real do
`lente_core` verde após a re-instalação).

---

## A pergunta e a resposta

> Recomputando os ciclos de módulo do `egui` contando **só** as arestas
> `uses_kind == "reference"`, o SCC de 85 (laudo 0031) encolhe?

**SCC: 85 → 42.** Pouco mais da metade do SCC inflado por `import`
(Limite 4). Os 43 módulos que saíram são módulos que **importavam** algo
do anel mas **não usavam o tipo na própria API**. Sobram 42 num único SCC
remanescente: o **acoplamento de tipo real** do egui — irredutível sem
refatorar o crate.

A expedição ao fork (subtipos de `uses`, identidade-por-id, descritor
semântico) entrega agora a resposta que ficou em aberto desde o laudo
0031.

---

## Fase 1 — Pré-requisito (fork instalado) e sanidade

### O fork instalado estava velho

A primeira coisa que tentei foi capturar o JSON com o fork; saiu sem
`uses_kind`. Inspeção: o `cargo-modules` em `~/.cargo/bin` era a versão
0.27.0 instalada de origem-remota (commit `a928eba`); o clone local em
`/home/dikluwe/Documentos/Antigravity/cargo-modules` tinha já dois
commits novos:

```
ddcd3ca 2026-06-03 feat(export-json): posição no fonte (arquivo + faixa de linhas) por nó
b44aa96 2026-06-03 feat(export-json): subtipos de uses (reference/import) por aresta
```

Reinstalei do clone local: `cargo install --path … cargo-modules`. ~2
minutos. O binário em PATH passou a refletir os dois commits.

### Compatibilidade do produto com o fork novo (defesa em profundidade)

Os JSONs ganharam dois campos novos (`uses_kind` por aresta `uses`;
`position` por nó). O `desserializar_grafo` da lente é tolerante a
campos extras (`serde` default, sem `deny_unknown_fields`). Verificado
rodando `cargo test -p lente_infra e2e_extrai_grafo_de_lente_core…
--ignored` → **verde** após a re-instalação. Sem regressão.

### Re-extração + sanidade contra 85

Re-extraídos `dados/export-egui.json` (`cargo modules export-json
--sysroot --compact --lib --package egui` em `<egui>/crates/egui`,
2.9 MB) e `dados/export-lente-core.json` (controle, 72 KB).

Distribuição de `uses_kind` no egui:

| `uses_kind` | Arestas | % das `uses` |
|---|---|---|
| `reference` (uso de tipo direto) | 8 331 | 80% |
| `import` (declaração `use` num módulo) | 2 098 | 20% |
| `<ausente>` | 0 | 0 |
| **Total `uses`** | 10 429 | 100% |

(Não há `reexport` separado — o fork classifica reexports junto com
`import`.)

No `lente_core`: 178 reference + 10 import = 188.

**Portão de sanidade**: reconstruir o grafo de itens com todas as `uses`
+ todas as `owns`, agregar por módulo (`lente_estrutura::agregar_por_modulo`),
detectar ciclos (`detectar_ciclos`) — **deve dar 85 no egui e 0 no
lente_core**. Resultado:

```
egui:        SCC=85  módulos=111  deps=864    ✓ bate com laudo 0031
lente_core:  SCC= 0  módulos=  7  deps=  3    ✓ bate com laudo 0031
```

A reconstrução está validada. Confio nos números da Fase 2.

---

## Fase 2 — Medir `só reference`

Reconstruir o grafo de itens incluindo **só** arestas `uses` com
`uses_kind == "reference"` (+ todas as `owns`, necessárias para
`agregar_por_modulo` achar o módulo contenedor); agregar + detectar.

```
=== egui ===
[Todas uses]     itens: 3694  arestas: 13937  módulos: 111  deps: 864  SCC=85
[Só reference]   itens: 3694  arestas: 11839  módulos: 111  deps: 386  SCC=42

Delta: 43 módulos saíram do SCC.

=== lente_core (controle) ===
[Todas uses]     SCC=0
[Só reference]   SCC=0   ← método não inventa ciclo
```

### Tabela do delta

| Métrica | Todas | Só reference | Δ |
|---|---|---|---|
| Arestas no grafo | 13 937 | 11 839 | −2 098 (−15%) |
| Deps módulo→módulo | 864 | 386 | **−478 (−55%)** |
| Maior SCC | 85 | **42** | **−43 (−51%)** |
| Nº SCCs ≥ 2 | 1 | 1 | 0 |

A linha decisiva é o SCC: cai à metade. **Metade do tamanho do "ciclo
gigante" do egui era artefato de imports.**

### 43 módulos que saíram (parcial — ver relatório)

```
egui::atomics, egui::atomics::atom_ext, egui::atomics::atom_layout,
egui::containers, egui::containers::close_tag, egui::containers::combo_box,
egui::containers::menu, egui::containers::modal, egui::containers::panel,
egui::containers::resize, egui::containers::scene,
egui::containers::scroll_area, egui::containers::sides,
egui::containers::tooltip, egui::containers::window, egui::debug_text,
egui::drag_and_drop, egui::gui_zoom, egui::introspection,
egui::load::texture_loader, … (+23)
```

Padrão qualitativo: módulos "agregadores" (`atomics`, `containers`) e
vários **containers individuais** (`menu`, `modal`, `panel`, `window`,
`scroll_area`). Faz sentido: containers tipicamente fazem
`use crate::*;` no topo, mas só **alguns** dos tipos importados aparecem
na assinatura pública dos seus widgets.

### 42 módulos que permaneceram — o acoplamento real

Primeiros 10:

```
egui                              egui::atomics::sized_atom_kind
egui::animation_manager           egui::containers::area
egui::atomics::atom               egui::containers::collapsing_header
egui::atomics::atom_kind          egui::containers::frame
egui::atomics::atoms              … (+32)
egui::atomics::sized_atom
```

São módulos que de fato **mencionam tipos uns dos outros** em
assinaturas/campos. A estrutura "core" do egui (`Context`, `Ui`,
`Response`, `Style`, `Memory`, `Layout`) provavelmente está aí, com
muitos widgets/containers passando essas tipos em sua API pública —
coerente com o estilo de **UI imediata**.

---

## Decisão que o número permite

O laudo **não** decide o produto. Mas o número torna duas escolhas
claras:

| Caminho | Estado |
|---------|--------|
| Adicionar opção `--modo-uses` (ou similar) ao `--estrutura`, com valores `todas`/`so-referencia` | **Justifica** — o ganho de utilidade é alto (SCC à metade), e o padrão "escolha do usuário" do laudo 0030 cobre isso bem. |
| Default novo `so-referencia` | **Tendência forte, mas pode esperar** — preserva o comportamento do laudo 0031 como default; usuário ativa o filtro quando quiser foco em acoplamento de tipo. Decisão do autor. |
| Estender `Aresta` para carregar `uses_kind: Option<UsesKind>` | **Necessário** para o filtro entrar; é o pré-requisito de produto. |
| Mudar `desserializar_grafo` no L3 para ler o campo | **Necessário** junto com o anterior. |
| Manter `--modo-uses` fora do produto, deixar a vista atual como única | **Desperdiça o instrumento** que o fork já entrega. |

O próximo prompt natural — pelo padrão dos laudos 0030 (escopo) e 0027
(ranking) — é "tornar `modo_uses` parâmetro do `--estrutura`, default
`todas`, opção `--so-referencia` (ou enum explícito)". Não é este
prompt.

---

## Honestidade sobre o alcance

- A medição mede **só** o efeito de `uses_kind`. Outras inflações
  possíveis (acoplamento via traits, via genéricos) não são separáveis
  com o que o fork rotula hoje.
- O fork **não distingue** `reexport` de `import` — o
  `prompt-fork-subtipos-uses.md` (referenciado no prompt 0033) parece ter
  parado em dois valores. Se um dia `reexport` virar valor distinto, um
  terceiro corte fica registrado.
- O **escopo** (`SeuCodigo` vs `Completo`) não foi exercitado aqui —
  laudo 0031 mostrou que ciclos são invariantes ao escopo. Vale aqui pela
  mesma razão.

---

## Verificação

| Item | Resultado |
|------|-----------|
| Fork instalado emite `uses_kind` | **Sim** após reinstalação do clone local |
| Compatibilidade com o produto (E2E real `lente_core`) | **Verde** após reinstalação |
| Portão de sanidade (SCC=85 com todas as `uses`) | **OK** no egui |
| Controle `lente_core` (0 nas duas versões) | **OK** |
| Suíte de produção (workspace) | **176 verdes + 19 ignored** — mesma do laudo 0031/0032 |
| Subprocessos do cargo (invariante 0023) | dois únicos, intocados |
| `Cargo.toml` raiz | intocado |
| `members` do workspace | `lab/medicao-ciclos-referencia` invisível (Arena com `[workspace]` vazio próprio) |

---

## Decisões tácitas

### D1 — Reusar `agregar_por_modulo` + `detectar_ciclos` do produto

Mesma decisão do laudo 0032: a confiança no resultado depende de usar
**a função do produto** e mudar **só a entrada**. Sanidade = reproduzir
o 85. Se a sanidade não batesse, a reconstrução estaria errada — e
nenhum número seria confiável. Bateu.

### D2 — Filtrar no parse, não estender `Aresta`

A política do prompt: **não** mudar o produto na medição. O
`uses_kind` é lido por uma struct **local** ao programa de medição
(`EdgeJSON`), filtrado na hora de virar `Aresta` do `lente_core`. A
`Aresta` do produto continua sem o campo. Quando/se um prompt de
produto entrar, aí sim estende.

### D3 — Reinstalar o fork do clone local

O pré-requisito do prompt era "fork atualizado **instalado**". O clone
tinha os commits; o binário não. Reinstalar é operação local e
reversível (basta `cargo install --git https://… cargo-modules` para
voltar ao remoto). Custo: ~2 min de compilação. Documentei o commit do
clone (`ddcd3ca`) para que o achado seja rastreável.

### D4 — Confirmar compatibilidade do desserializador (defesa em profundidade)

Após reinstalar o fork, rodei um E2E real do produto antes de confiar
nas medições. Veio verde. Confirma que o `serde` da lente é tolerante
a campos extras (`uses_kind`, `position`) — sem `deny_unknown_fields`.
Isso é informação útil para o prompt de produto subsequente: o
desserializador **não** precisa mudar para ignorar; precisa mudar (de
modo aditivo) para **ler**.

### D5 — Escopo Completo (não SeuCodigo) na medição

Por simplicidade do dump. O laudo 0031 estabeleceu invariância dos
ciclos ao escopo (stdlib é sorvedouro). Vale aqui pelo mesmo argumento
estrutural: filtrar stdlib não muda quem sai/quem permanece no SCC
intra-alvo.

### D6 — Não há `reexport` no `uses_kind`

Observado empiricamente: só `import` e `reference`. O prompt 0033
contemplava "se o fork tiver `reexport`, um terceiro corte". Não tem;
registrado.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0033 |
|-----------|-----------------|
| Pergunta do laudo 0031 (SCC de 85 é genuíno ou inflado?) | **Respondida**: 42 genuíno + 43 inflação por import. |
| Hipótese do laudo 0032 (raiz como ponte) | Já rejeitada; reconfirmada com o número mais fino. |
| Adicionar `modo_uses` ao produto (estrutura) | **Aberta com justificativa** (próximo prompt natural). |
| Estender `Aresta` para `uses_kind` | **Aberta como pré-requisito** do anterior. |
| DSM visual | **Aberta** — a vista de 42 módulos é tratável; com matriz fica acessível. |
| Filtro de "folhas comportamentais" (Limite 3) | **Aberta** — trilha separada. |

---

## O que NÃO mudou

- **Crates de produção**: zero toques.
- **`Cargo.toml` raiz**: intocado.
- **`lente_estrutura`** (L1): usado pela Arena, não modificado.
- **Spec, ADRs**: zero toques.
- **`Aresta` / `desserializar_grafo`**: zero toques (extensão fica para
  prompt de produto).
- **Suíte de testes**: 176 verdes + 19 ignored — mesma do laudo 0031/0032.
- **Subprocessos do cargo** (invariante 0023): dois únicos.

---

## Observação metodológica

"Medir antes de mudar o produto" — terceira aplicação seguida (laudos
0021, 0029, 0030, 0032 todos no padrão). Aqui a medição **confirma uma
parte** da intuição (o import inflava o SCC) e **refuta outra** (o
acoplamento "real" é menor do que o todas-as-uses sugeria, mas existe e
é substancial). Sem isso, qualquer escolha entre "ignorar Limite 4" e
"focar em arquitetura" seria aposta.

O ganho da expedição ao fork inteira (identidade-por-id, descritor
semântico, subtipos de `uses`, posição-no-fonte) materializa-se nesta
medição: o `uses_kind` em particular é o que **separa Limite 4 da
arquitetura**, fazendo a vista global passar de "85 módulos opacos" a
"42 módulos nomeados, irredutíveis sem refatoração".

---

## Arquivos

- `lab/medicao-ciclos-referencia/Cargo.toml` — Arena isolada.
- `lab/medicao-ciclos-referencia/src/main.rs` — programa de medição.
- `lab/medicao-ciclos-referencia/dados/export-egui.json` — fork direto, 2.9 MB.
- `lab/medicao-ciclos-referencia/dados/export-lente-core.json` — controle.
- `lab/medicao-ciclos-referencia/relatorio.md` — relatório bruto (padrão
  Arena: conteúdo denso em `lab/`, registro em `lessons/`).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Medição em Arena: o SCC de 85 módulos do egui (laudo 0031) é **51% inflação por `import`** (Limite 4) e **49% acoplamento de tipo real**. Recomputado com `uses_kind == "reference"`: SCC cai de 85 para 42, com 43 módulos saindo. Distribuição no egui: 80% das `uses` são `reference`, 20% `import` (sem `reexport` distinto). Portão de sanidade (85) bateu; controle `lente_core` (0) bateu. Pré-requisito: fork reinstalado do clone local (commit `ddcd3ca`); compatibilidade com produto confirmada por E2E real. Material para o próximo prompt de produto (estender `Aresta` com `uses_kind` + opção `--modo-uses`/`--so-referencia` no `--estrutura`). Sem mudança em produto, spec ou fork. | `lab/medicao-ciclos-referencia/{Cargo.toml,src/main.rs,dados/*.json,relatorio.md}`, `00_nucleo/lessons/0033-medicao-ciclos-referencia.md` |
