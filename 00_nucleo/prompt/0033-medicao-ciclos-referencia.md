# Prompt: Medir os ciclos do egui contando só `reference` (Limite 4)

**Camada**: Arena (`lab/`) — medição descartável, como `lab/medicao-ciclos-egui`
(laudo 0032). **Não** entrega solução, entrega um **número**.
**Criado em**: 2026-06-03
**Estado**: `PROPOSTO`
**Decisões de origem**: o fork passou a emitir `uses_kind` por aresta `uses`
(`reference` vs `import`) — prompt `prompt-fork-subtipos-uses.md`, executado;
laudo 0031 (egui: 1 SCC de **85 módulos**, ≈76% do crate); laudo 0032 (a
ponte-raiz/reexport **não** é a causa: 85→84 ao remover a raiz). A pergunta que
abriu toda a expedição ao fork e ficou aberta: o SCC de 85 é **acoplamento de
tipo genuíno** ou está **inflado por import** (Limite 4)? Agora é respondível.
Decisão do autor: medir (Arena) antes de qualquer mudança de produto.
**Pré-requisito**: o fork atualizado **instalado** e o egui **re-extraído** com
`uses_kind`; `lente_estrutura` (`agregar_por_modulo`, `detectar_ciclos`, laudo
0031); a Arena do laudo 0032 como molde.
**Posição**: fecha a pergunta que motivou as rodadas de fork. Medição pura; se o
produto passa a contar só `reference` (por padrão ou por opção) é decisão de
**depois**, tomada pelo número.
**Arquivos afetados**: `lab/medicao-ciclos-referencia/` (programa + dumps +
relatório) e `00_nucleo/lessons/0033-…`. **Nenhum crate de produção.**

---

## Pergunta única

Recomputando os ciclos de módulo do egui contando **só** as arestas `reference`
(uso direto de tipo em assinatura/campo), o SCC de 85 encolhe?

- Se **encolher muito**, os imports (Limite 4) inflavam o ciclo; o acoplamento
  "real" do egui é o resíduo.
- Se **encolher pouco**, o acoplamento de tipo é genuíno e o egui é mesmo
  fortemente entrelaçado.

Em qualquer caso, a vista de ciclos passa a ser confiável — porque finalmente se
sabe **qual fração** do ciclo é referência real e qual é import.

---

## Contexto

O fork agora rotula cada aresta `uses`: `reference` (item→tipo, de
`walk_and_push_type` — uso genuíno) vs `import` (módulo→item, do laço de escopo
de `process_module` — declaração `use` atribuída ao módulo, o Limite 4). O laudo
0032 já descartou o reexport/raiz como motor do ciclo; o suspeito que sobra é o
**import**.

A medição: reconstruir o grafo de itens do egui a partir do `export-json` do fork
(que agora tem `uses_kind`), e computar os ciclos de módulo em **duas** versões —
todas as arestas `uses` (controle/sanidade) e **só** `reference`. Comparar.

**Importante (de onde vem o JSON)**: o `export-json` cru do **fork** carrega o
`uses_kind`. O pipeline `--estrutura` da **lente** agrega e **não** lê o
`uses_kind` hoje (o `desserializar_grafo` foi escrito antes do campo existir),
então a saída `--estrutura` da lente **não serve** — ela já perdeu o subtipo.
Esta medição consome o `export-json` **do fork, rodado direto** (`cargo modules
export-json`), não a saída da lente.

---

## Restrições estruturais

- **Arena, descartável, em `lab/`.** `[workspace]` vazio próprio (padrão
  `medicao-egui`/`medicao-ciclos-egui`). **Sem mudança de produto, sem flag.**
- **Reusar o algoritmo do produto.** A medição roda
  `lente_estrutura::agregar_por_modulo` + `detectar_ciclos` — exatamente as
  funções do laudo 0031. O run "todas as arestas `uses`" tem que **reproduzir o
  SCC de 85** do laudo 0031: é o **portão de sanidade**. Se não reproduzir, a
  reconstrução está errada; parar e corrigir antes de confiar no número
  `reference`.
- **Filtrar pelo `uses_kind` na hora de montar o grafo** (no parse do JSON),
  **não** estender a `Aresta`/`desserializar_grafo` da lente. Para o caso
  `reference`, incluir só as arestas `uses` com `uses_kind == "reference"` (+
  **todas** as `owns`, que `agregar_por_modulo` precisa para achar o módulo
  contenedor). Para o caso de sanidade, incluir todas as `uses`. (Estender a
  `Aresta` para carregar `uses_kind` é mudança de **produto**, para depois, se
  justificada — não é a medição.)
- **Não tocar o fork, os crates de produção, nem a spec.**

---

## Fase 1 — Re-extrair, confirmar o campo, e o portão de sanidade

1. Garantir o fork atualizado instalado. Rodar o `export-json` do **fork direto**
   no egui (com `--sysroot`, como a lente extrai) e **salvar** o JSON em
   `lab/medicao-ciclos-referencia/dados/export-egui.json`. Idem `lente_core`
   (controle) em `export-lente-core.json`.
2. **Confirmar** que as arestas `uses` trazem `uses_kind` (`"reference"` /
   `"import"`; e checar se há também `"reexport"` — o fork pode ter feito a parte
   opcional). Anotar quais valores aparecem.
3. **Portão de sanidade**: reconstruir o grafo de itens com **todas** as `uses` +
   `owns`, rodar `agregar_por_modulo` + `detectar_ciclos`, e confirmar que
   reproduz o SCC de **85 módulos** do laudo 0031 (no escopo completo; os ciclos
   são invariantes ao escopo — laudo 0031 —, então não é preciso filtrar
   stdlib). Se não bater, parar.
4. **Controle** `lente_core`: 0 ciclos nas duas versões.

**Reportar**: os valores de `uses_kind` observados, a sanidade (85 reproduzido),
e a contagem de arestas `reference` vs `import` no egui (já informa o tamanho
potencial do efeito).

---

## Fase 2 — Medir

- Reconstruir o grafo de itens com **só** as arestas `uses` de
  `uses_kind == "reference"` (+ todas as `owns`); rodar `agregar_por_modulo` +
  `detectar_ciclos`.
- **Reportar**:
  - maior SCC com **todas** as `uses` (85, sanidade) vs **só `reference`** (a
    resposta); número de SCCs em cada;
  - quantos módulos **saem** do SCC grande ao contar só `reference`;
  - (se barato) se o resíduo **fragmenta** em vários SCCs e **quais** módulos
    formam o(s) ciclo(s) de referência — é o acoplamento de tipo "real" do egui;
  - (se o fork tiver `reexport`) opcional: um terceiro corte
    `reference` + `import` sem `reexport`, só para registro.
- Escrever `relatorio.md` na Arena + registro em `00_nucleo/lessons/`.

---

## Critérios de Verificação

```
Dado o export-json do fork atualizado para o egui
Então as arestas uses trazem uses_kind (reference/import)

Dado o grafo reconstruído com TODAS as uses + owns
Quando agregado e os ciclos computados
Então reproduz o SCC de 85 módulos do laudo 0031 (portão de sanidade)

Dado o grafo reconstruído com SÓ reference + owns
Quando agregado e os ciclos computados
Então o relatório reporta o maior SCC e o número de SCCs resultantes

Dado a diferença todas-uses vs só-reference
Então o relatório reporta quantos módulos saem do SCC grande

Dado o lente_core (0 ciclos) como controle
Então continua 0 nas duas versões (o método não inventa nem some ciclo por engano)
```

(Não há suíte de produção — é Arena. A verificação é o portão de sanidade bater
85 e o relatório de achados existir.)

---

## Resultado esperado

- O número: maior SCC com todas as `uses` (85) vs só `reference`.
- A conclusão escrita: se `reference` ≪ 85, o import (Limite 4) inflava o ciclo,
  e o acoplamento de tipo "real" do egui é o resíduo (com os módulos nomeados);
  se `reference` ≈ 85, o acoplamento é genuíno. **Esta é a resposta que a
  expedição ao fork inteira existiu para obter.**
- Material para a próxima decisão (de **produto**): a `lente_estrutura` deve
  passar a contar só `reference` — por padrão, ou por opção (como o escopo do
  laudo 0030)? Decidido **pelo número**, não agora.

---

## O que NÃO entra

- **Mudar a `lente_estrutura`** para um modo `reference`-only / mudar o default:
  decidido **depois**, pelo número. Não é esta medição.
- **Estender a `Aresta`/`desserializar_grafo`** para carregar `uses_kind`: é a
  mudança de produto do modo escolhido, para depois — não a medição (que filtra
  no parse).
- **A trilha local (posições)**: separada.
- **DSM visual, filtro de folhas (Limite 3), remoção da E2**: outras trilhas.

---

## Observação metodológica

Medir antes de mudar o produto — a disciplina do projeto, a mesma do laudo 0032
(que rejeitou a hipótese da raiz). O fork deu o instrumento (`uses_kind`); esta
medição o lê. Reusa o **mesmo** `agregar_por_modulo` + `detectar_ciclos` do
produto (sanidade = reproduzir 85) e muda **só a entrada** (arestas
`reference`). E é honesta sobre o alcance: isola a contribuição do import (Limite
4) ao ciclo; se o fork também rotulou `reexport`, esse é um corte a mais, de
registro. O número decide se vale um modo `reference`-only no produto — em vez de
apostar.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-03 | Medição em Arena: recomputa os ciclos de módulo do egui contando só arestas `reference` (via `uses_kind` do fork), contra o SCC de 85 do laudo 0031 (com todas as `uses`, portão de sanidade). Reconstrói o grafo de itens do `export-json` do fork direto (que carrega `uses_kind`; a saída `--estrutura` da lente não serve, pois não lê o campo), filtra no parse, reusa `agregar_por_modulo`/`detectar_ciclos` do produto. Controle no `lente_core`. Responde se o SCC de 85 é acoplamento de tipo genuíno ou inflado por import (Limite 4). Sem mudança de produto, fork ou spec. | `lab/medicao-ciclos-referencia/{Cargo.toml,src/main.rs,dados/*.json,relatorio.md}`, `00_nucleo/lessons/0033-medicao-ciclos-referencia.md` |
