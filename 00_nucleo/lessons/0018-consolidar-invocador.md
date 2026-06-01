# Laudo de Execução — Prompt 0018 (Consolidar invocacao.rs + fork.rs)

**Camada**: L5 (laudo)
**Data**: 2026-06-01
**Prompt executado**: `00_nucleo/prompt/0018-consolidar-invocador.md`
**Decisões de origem**: laudo 0017 D1 — duplicação de subprocess.
**Estado**: `EXECUTADO` — primitiva única de subprocess no crate; 77 verdes
+ 4 ignored; não-regressão total; pureza preservada.

---

## O que o prompt pediu

Eliminar a duplicação: `invocacao.rs` (antigo, recebe diretório) e `fork.rs`
(novo, recebe nome do pacote) ambos rodavam `Command::new("cargo")
modules export-json...`. Consolidar para que **exista um único subprocess
no crate**, antes do L4 wiring nascer sobre o desenho duplo.

---

## Fase 1 — Leitura

| Item | Achado |
|------|--------|
| `invocacao::invocar(&Path) -> Result<String, ErroAdaptador>` | Lê `Cargo.toml`, extrai `[package].name` via parser TOML linha-a-linha, executa `cargo modules ...` com `current_dir(diretorio)`. |
| `fork::invocar_fork(&str) -> Result<String, ErroFork>` | Recebe o nome direto, executa sem `current_dir`. |
| Chamadores | `invocacao::invocar` é usado **só** por `extrair_grafo` (`lib.rs:116`); `fork::invocar_fork` ainda sem chamadores externos (será do L4). |
| Diferença crítica | `current_dir(diretorio)` — `invocacao` muda o cwd só do subprocess; `fork` herda o cwd do processo. |
| Duplicação | Dois `Command::new("cargo")` (`invocacao.rs:58`, `fork.rs:70`). |

A diferença `current_dir` decidiu o desenho da consolidação.

---

## Fase 2 — Consolidação aplicada

### Estrutura final

```
fork::invocar_em(pacote, current_dir: Option<&Path>)   ← primitiva única
                ↑                                       (pub(crate))
        ┌───────┴───────────────────────┐
        │                               │
fork::invocar_fork(pacote)      invocacao::invocar(diretorio)
        (pública)                  (pub(crate); lê Cargo.toml,
                                    delega ao invocar_em)
```

- **`fork::invocar_em`** é a única função do crate com `Command::new`.
- **`fork::invocar_fork(pacote)`** delega para `invocar_em(pacote, None)` —
  interface pública simples (laudo 0017 D2 preservada).
- **`invocacao::invocar(diretorio)`** descobre o pacote, depois chama
  `fork::invocar_em(&pacote, Some(diretorio))` e mapeia `ErroFork` para
  `ErroAdaptador`.

### Verificação grep

```
$ grep -rn 'Command::new("cargo"' --include='*.rs' 03_infra/
03_infra/src/fork.rs:74:    /// única função do crate que roda `Command::new("cargo")`...  (doc-comment)
03_infra/src/fork.rs:81:    let mut cmd = Command::new("cargo");
```

**Uma única ocorrência real** (linha 81). A linha 74 é só menção no doc-comment.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test` (sem ignored) | **77 verdes** — mesma contagem de antes |
| `cargo test -p lente_infra -- --ignored` | **4/4** verdes (E2E de `extrair_grafo` atravessam o novo caminho) |
| `cargo tree -p lente_core` | só o crate — pureza preservada |
| Testes pré-existentes alterados | **zero** — refatoração invisível |
| Chamadores externos tocados | **zero** |

---

## Decisões tácitas

### D1 — `invocar_fork` + `invocar_em` (pública simples + pub(crate) parametrizada)

Em vez de mudar a interface pública `invocar_fork(pacote)` para aceitar `cwd`
(o que violaria a D2 do laudo 0017), criei uma função interna `invocar_em`
que aceita `current_dir: Option<&Path>`. A pública delega à interna com
`None`. Vantagens:
- Interface pública continua mínima (1 parâmetro).
- `invocacao` tem o que precisa via `pub(crate)` sem expor complexidade.
- Documento explícito no doc-comment: `invocar_em` é a primitiva única.

### D2 — Mapeamento de erro local, sem `impl From<ErroFork>`

Optei por uma função `mapear_erro_fork(ErroFork) -> ErroAdaptador` em
`invocacao.rs` em vez de `impl From<ErroFork> for ErroAdaptador`. Razões:
- `From` impl introduz conversão **implícita** via `?` — esconde o ramo.
  Função explícita força ver o mapeamento.
- A correspondência não é 1:1 trivial: `FalhaSubprocess` se ramifica em
  `BinarioNaoEncontrado` (quando `ErrorKind::NotFound`) ou
  `FalhaSubprocesso(String)` para os outros. Explicitar isso ajuda manutenção.
- Acoplamento `ErroAdaptador → ErroFork` é assimétrico: apenas o
  `invocacao` precisa converter; manter local evita expor essa conversão
  como API.

### D3 — Tipos de erro preservados (não-regressão estrita)

Não toquei `ErroAdaptador` nem `ErroFork`. O mapeamento traduz uma
`ErroFork::FalhaSubprocess(NotFound)` para `ErroAdaptador::BinarioNaoEncontrado`
(que existia desde o laudo 0006), uma `StatusErro` para `SubprocessoFalhou`,
etc. Todos os testes do `lente_infra` que casam contra essas variantes
continuam passando sem alteração.

### D4 — Cabeçalho de linhagem do `invocacao.rs` atualizado

Adicionei linha de "consolidado por prompt 0018" no lineage do `invocacao.rs`.
O `fork.rs` mantém seu lineage do 0017 (não foi reformado, só ganhou a
função interna `invocar_em`).

---

## Não-regressão registrada

| Crate | Antes | Depois |
|-------|-------|--------|
| lente_core | 30 | 30 |
| lente_infra (não-ignored) | 19 | 19 |
| lente_infra (ignored) | 4/4 | 4/4 (todos passam — `e2e_extrai_grafo_de_fixture` e `e2e_extrai_grafo_de_lente_core_com_colisao_de_path` atravessam o novo caminho `invocacao → fork::invocar_em`) |
| lente_investiga | 17 | 17 |
| lente_resolve | 11 | 11 |

Zero testes alterados. Refatoração 100% interna.

---

## Sinalização para o L4 wiring (próximo prompt)

Agora há dois pontos de entrada claros, com responsabilidades distintas:

- **Modo `--pacote NOME`** (L4 vai chamar): `fork::invocar_fork(NOME)` —
  pega o JSON cru, depois L4 desserializa via `serde_json::from_str` num
  `GrafoDTO` e chama `traducao::traduzir` (ou função equivalente exposta —
  ver pendência abaixo).
- **Modo `--grafo arquivo.json`** (L4 vai chamar): lê o arquivo do disco
  (sem invocar fork), desserializa, traduz.

Pendência herdada do laudo 0017: a `traducao::traduzir` ainda é `pub(crate)`.
Se o L4 vive **dentro** do `lente_infra` (como função pública adicional),
isso não é problema; se vive **fora** (outro crate), precisa expor a
tradução. Decisão do prompt do L4.

A pendência do laudo 0016 também permanece: o L4 define se `resolve → raio`
é garantia ou se a dívida raio-por-id continua latente.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | Consolidação: `fork::invocar_em` vira primitiva única (subprocess); `fork::invocar_fork` e `invocacao::invocar` delegam a ela. Mapeamento `ErroFork → ErroAdaptador` local em `invocacao.rs`. Verificação grep: um único `Command::new("cargo")` no crate. 77 verdes + 4 ignored, zero testes alterados, pureza preservada. | `03_infra/src/fork.rs`, `03_infra/src/invocacao.rs` |
