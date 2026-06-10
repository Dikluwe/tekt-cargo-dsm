# Laudo de Execução — Prompt 0069 (reconciliar sobreviventes; inerte vs fora-do-oráculo)

**Camada**: verificação e documentação — alvo: o repo do **linter** (`tekt-linter`,
clone canônico com o conserto do 0052). Sem código novo no projeto.
**Data**: 2026-06-09
**Prompt executado**: `00_nucleo/prompt/0069-reconciliar_sobreviventes_oraculo.md`
**Estado**: `EXECUTADO` — 43 sobreviventes reconciliados **um-a-um** contra a lista crua
da ferramenta; **0 mudam veredito** (nenhuma fixture nova); selo "completo para
vereditos" **estendido** ao código do cego-#2 (0060). Self-lint do linter = 0; suíte
verde (550 passed / 0 failed).

---

## Correspondência de numeração (registrada, conforme a nota do prompt)

| Projeto (`tekt-cargo-dsm`) | Linter (`tekt-linter`, `00_nucleo/`) |
|---|---|
| **0069** `reconciliar_sobreviventes_oraculo` | **0056** `reconciliar_sobreviventes` (`IMPLEMENTADO`, 2026-06-08) |

O linter já fechou esta reconciliação como seu **0056** contra o código **pré-0057**
(rs_parser com **178** mutantes → 38 sobreviventes). Depois disso o linter evoluiu
(0057–0062: caminho lint→veredito, cego-#2, exclusão de `#[cfg(test)]`, release
0.2.0). Este laudo (projeto 0069) **re-roda a reconciliação no HEAD atual** (`ec0fc9a`,
pós-0062) — onde o `rs_parser.rs` cresceu para **209** mutantes / **43** sobreviventes —
e estende a classificação ao código novo do **cego-#2 (0060)**. Ponteiro de volta:
linter `00_nucleo/0056-reconciliar_sobreviventes.md`.

---

## A lista crua (fonte: `mutants.out/missed.txt`, run de hoje, HEAD atual)

`cargo mutants` (run de repo inteiro, jun 9) — subconjunto `03_infra/rs_parser.rs`:

```
153 caught + 43 missed + 13 unviable = 209   (= cargo mutants --list)
```

Totais conferem. **Não** confiei na tabela do 0056 — reli `missed.txt` linha a linha.
A soma das três naturezas abaixo = **43, exata, sem resíduo**.

---

## Reconciliação um-a-um — 43 = 8 inerte + 31 fora-do-oráculo + 4 equiv-input-válido + 0 veredito

### Muda veredito — **0** (nenhuma fixture nova)

O motor (V1–V14) e a classificação ciente de deps já tinham 0 (0054); o 0055 fechou os
de extração que afetavam veredito. Os 5 sobreviventes **novos** (código do 0060) **não**
mudam veredito — provado abaixo.

### Inerte — **8** (saída que nenhuma regra lê, ou código morto sob a grammar)

| Linha:função | Mutante | Prova de inércia (verificada por grep, HEAD atual) |
|---|---|---|
| 214/216/217/218/219 `parse_layer_tag` | delete arms L0/L2/L3/L4/Lab | produz `PromptHeader.layer`; o `grep .layer` em `01_core/rules/` só acha o accessor `fn layer()` do **file** (path-derivado por `resolve_file_layer`) — nenhuma regra lê o `@layer` **parseado** |
| 422 `collect_imports` | `&&`→`||` | decide só `ImportKind` (`Named`/`Direct`); o `grep ImportKind` em `rules/` só acha **construção** em fixtures — nenhuma regra **ramifica** por `import.kind` |
| 967/969 `collect_type_param_names` | delete arms `type_identifier`/`constrained_type_parameter` | código morto sob `tree-sitter-rust = "0.23"` (pinada no `Cargo.toml`); a grammar 0.23 só emite `type_parameter` — arms de compat de grammars antigas |

### Fora-do-oráculo — **31** (muda só a posição; o harness afirma IDs+contagem, não posição)

| Linha:função | nº | O que muda na saída |
|---|---|---|
| 1109–1127 `find_first_error_pos` | 19 | linha:coluna de um erro de sintaxe. Gated por `root.has_error()` — **não decide** se o `PARSE`/V0 é emitido, só a posição na mensagem |
| 994/1003/1014 `extract_declarations` | 5 | `line` da declaração (V12) |
| 416/435 `collect_imports` (`+`→`*`) | 2 | `line` do import reportado |
| 765/778 `collect_tokens` (`+`→`*`) | 2 | `line`/`column` do token (V4) |
| 311 `collect_path_refs` (`+`→`*`) **[novo, 0060]** | 1 | `line` do path-ref (`row + 1` → `row * 1` = `row`) — só a posição; o path/chave/classificação não muda |
| 399 `scan_token_tree` (`+`→`*`, `+`→`-`) **[novo, 0060]** | 2 | `line` do path-ref de macro (mesma aritmética de `row + 1`) |

### Equivalente no domínio de input válido — **4** (novo: o guard de varredura de macro do 0060)

`scan_token_tree` L397 — o guard `prev_is_colon` que evita emitir segmentos
**intermediários** de um caminho dentro de um `token_tree` (atributo/macro):

| Linha | Mutante | Efeito | Por que é equivalente para input válido |
|---|---|---|---|
| 397 | `-`→`/` (`i-1`→`i`) | `prev_is_colon` vira sempre `false` | emite também segmentos intermediários |
| 397 | `>`→`<` (`i>0`→`i<0`) | idem (usize: `i<0` nunca) | idem |
| 397 | `>`→`==` (`i>0`→`i==0`) | idem (i=0 → `child(MAX)`=None) | idem |
| 397 | `>`→`>=` (`i>0`→`i>=0`) | quase-idêntico | idem |

**Prova de equivalência (estrutural, não por "ninguém lê"):** `try_emit_path_ref`
chaveia por `first_segment(path)` do token **isolado** e deduplica via `seen`; um
segmento intermediário só viraria aresta se ele próprio resolvesse a um crate. Em Rust
**válido, só o segmento-cabeça de um caminho é um crate** — os intermediários são
módulos/tipos → `classify_import` devolve `LocalItem` → **não emitido**. Logo estes
mutantes diferem do correto **apenas** em sequências `id :: id :: id` cujo `id` do meio
seja um crate — **input que caminhos Rust válidos nunca produzem**. Não muda veredito
para nenhum ficheiro real. (Construir uma fixture que os mate exigiria input inválido →
fora do contrato do corpo; por isso ficam aqui, não em "muda veredito".)

**8 + 31 + 4 + 0 = 43.** ✓

---

## O selo, estendido ao cego-#2 (0060)

O 0056 selou "completo para vereditos de lint de Rust" para `{regras + classificação +
extração}` pré-0057; o 0057 estendeu ao caminho inteiro lint→veredito. **Este laudo
estende ao código do path-ref-fora-do-`use` (0060):** seus 7 sobreviventes novos são
**3 de posição** (out-of-oracle, mesma natureza de antes) + **4 equivalentes no input
válido** (guard de macro, prova estrutural) + **0 que mudam veredito**. O cego-#2
entrou **sem abrir buraco de veredito** — nenhuma fixture nova foi necessária.

A fronteira permanece declarada, não construída: **o oráculo de posição** (linha:coluna
de violações e de erros `PARSE`/V0) mataria os 31 fora-do-oráculo — é contrato de outra
natureza (frágil, acoplado à grammar), **trilha à parte, a decidir, sem decidir aqui**.

---

## Erros de premissa do prompt 0055 (registro causal, do linter-0056)

Reconfirmados (já corrigidos in loco no linter): os números de linha do prompt 0055
vinham da versão **pré-0052** (lidos do master público, não do clone canônico); e a
suposição de que `collect_type_param_names` alimentava as type-sigs de V6/V12 estava
errada — alimenta o **V11** (blanket impls), por isso as fixtures genéricas do V12 não
mataram nada. O que absorveu o erro foi a disciplina "matar ou provar equivalente, da
fonte, um a um" — lição: prompts que raciocinam de fonte não-canônica precisam da
verificação contra o clone canônico embutida na execução.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `missed.txt` relido (não a tabela) | 153 caught + 43 missed + 13 unviable = 209 (= `--list`) — totais conferem |
| 43 classificados, soma exata | 8 inerte + 31 fora-do-oráculo + 4 equiv-input-válido + 0 veredito = 43 |
| Cada inerte com prova | grep anexo (layer/ImportKind não-lidos; grammar 0.23 pinada) |
| Cada fora-do-oráculo | comportamento (posição) registrado com linha e função |
| Muda-veredito | **0** → nenhuma fixture nova → **0 código** |
| Self-lint do linter | **0** |
| Suíte do linter | **550 passed / 0 failed** |
| Nada mascarado | confirmado (sem whitelist, sem exclusão nova) |

---

## O que resta (fora de escopo deste prompt, do próprio prompt)

Em ordem, prompts seguintes no linter: contador de `Layer::Unknown` em alvo real;
oráculo diferencial contra a computação de dependências da própria lente; corpus de
projetos reais. À parte: decisão de merge com o `master` público; e a decisão sobre o
**oráculo de posição/V0-`PARSE`**, que este laudo só **declara**.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-09 | Reconciliação dos sobreviventes do `rs_parser.rs` do linter no **HEAD atual** (`ec0fc9a`, pós-0062) — espelho do linter-0056 (que fechou 38 no código pré-0057). Re-rodada a mutação (lista crua `missed.txt`, não a tabela): rs_parser **153 caught + 43 missed + 13 unviable = 209** (= `--list`; totais conferem — o ficheiro cresceu 178→209 com o cego-#2 do 0060). 43 classificados um-a-um, soma exata: **8 inerte** (`parse_layer_tag` ×5 — `@layer` parseado não-lido; `collect_imports` L422 `&&→||` — `ImportKind` não-ramificado; `collect_type_param_names` ×2 — morto sob `tree-sitter-rust 0.23`; provas por grep no HEAD), **31 fora-do-oráculo** (posição: `find_first_error_pos` 19, `extract_declarations` 5, `collect_imports` line 2, `collect_tokens` 2, e os **novos** do 0060 `collect_path_refs` L311 1 + `scan_token_tree` L399 2), **4 equivalentes no domínio de input válido** (novo: `scan_token_tree` L397 — o guard `prev_is_colon` de varredura de macro; prova **estrutural**: `try_emit_path_ref` chaveia por `first_segment`+dedup, e em Rust válido só o segmento-cabeça é crate → diferem só em `id::id::id` com crate no meio, input impossível), **0 que mudam veredito** → **nenhuma fixture nova / 0 código**. Selo "completo para vereditos" **estendido ao cego-#2 (0060)**: entrou sem abrir buraco de veredito. Oráculo de posição/V0-`PARSE` declarado como trilha à parte (não construído). Erros de premissa do 0055 reconfirmados (linhas pré-0052; `collect_type_param_names`→V11, não V6/V12). Self-lint do linter = 0; suíte 550/0; nada mascarado. Correspondência: projeto 0069 ↔ linter 0056. | (verificação — nenhum código alterado; `00_nucleo/lessons/0069-reconciliar_sobreviventes_oraculo.md`; ponteiro p/ linter `00_nucleo/0056-reconciliar_sobreviventes.md`) |
