# Laudo de Execução — Prompt 0017 (lente_infra::fork — invocador do fork)

**Camada**: L5 (laudo)
**Data**: 2026-06-01
**Prompt executado**: `00_nucleo/prompt/0017-l3_invocador_fork.md`
**Decisões de origem**: decisão do autor (CLI aceita `--pacote` e `--grafo`);
ADR-0001 (fork como fonte); spec Limite 1 (sysroot sempre).
**Primeiro de três prompts da composição.**
**Estado**: `EXECUTADO` — módulo `fork.rs` criado, 77 testes verdes + 4
ignored; pureza preservada.

---

## O que o prompt pediu

Um invocador encapsulado do fork: `invocar_fork(pacote: &str) -> Result<String,
ErroFork>` que executa `cargo modules export-json --sysroot --compact --package
<pacote>` como subprocess e devolve o JSON cru. Independente da
desserialização (a `traducao` existente cuida disso). É a peça que o L4 (wiring,
próximo prompt) chamará quando a CLI vier com `--pacote NOME`.

---

## O que foi gerado

| Arquivo | Conteúdo |
|---------|----------|
| `03_infra/src/fork.rs` (novo) | `invocar_fork(pacote)` + `enum ErroFork { FalhaSubprocess, StatusErro, StdoutInvalido }` com `Display`/`Error`. 3 testes (2 ignored, 1 unitário de Display). |
| `03_infra/src/lib.rs` (edit) | `pub mod fork;` adicionado. |

**Não tocados**: `Cargo.toml` (std-only — `std::process`); `invocacao.rs`,
`traducao.rs`, `dto.rs` (aditivo, não mexe no que existe); `lente_core`.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test` (sem ignored) | **77 verdes** (core 30, infra **19**, investiga 17, resolve 11) |
| `cargo test -p lente_infra -- --ignored` | **4/4** (2 anteriores + 2 do fork: invoca lente_core, pacote inexistente) |
| `cargo tree -p lente_core` | só o crate — pureza preservada |
| `cargo tree -p lente_infra` | sem dependência externa nova (só serde + serde_json + lente_core) |

---

## Decisões tácitas

### D1 — Módulo novo `fork.rs`, separado do `invocacao.rs` existente

Já havia `invocacao.rs` (laudo 0003) que invoca o mesmo comando. Não modifiquei
— criei um módulo paralelo. Razões:

- **Interfaces diferentes**: `invocacao::invocar(diretorio: &Path)` recebe um
  diretório, lê o `Cargo.toml`, descobre o nome do pacote. `fork::invocar_fork(pacote: &str)`
  recebe o nome direto, roda no cwd.
- **Erros diferentes**: `invocacao` usa `ErroAdaptador` (acoplado à pipeline
  inteira); `fork` usa `ErroFork` próprio (limpo, independente).
- **Usos diferentes**: `invocacao` serve a `extrair_grafo(caminho_crate)` —
  uso "passe o diretório de um crate qualquer". `fork` servirá ao wiring
  CLI `--pacote NOME` — uso "estou no workspace e quero o JSON deste pacote".

Os dois coexistem. Quando o L4 nascer, ele escolhe qual usar conforme o
modo da CLI. Mais limpo que tentar generalizar uma única função para os dois
casos.

### D2 — `cwd` não-configurável

A função roda `Command::new("cargo")` sem `current_dir` — herda o cwd do
processo chamador. Manter assim (em vez de aceitar `cwd: &Path`):
- Mantém a interface simples (1 parâmetro).
- O L4 (chamador) pode `std::env::set_current_dir` se precisar mudar antes de
  invocar — responsabilidade dele.
- Os testes ignored rodam no cwd do `cargo test`, que é a raiz do workspace
  — funciona naturalmente.

Se aparecer caso onde o invocador precisa de cwd próprio sem afetar o
processo todo, parametrizar é mudança localizada (adicionar um overload ou
flag). Adiada por YAGNI.

### D3 — Testes ponta-a-ponta com `#[ignore]`

Os dois testes que invocam o fork real (`invoca_fork_no_lente_core_devolve_json_valido`,
`pacote_inexistente_retorna_status_erro_com_mensagem`) são `#[ignore]`.
Precedente firme: desde o laudo 0003, todo teste que dispara o fork
(~3s/execução; requer binário instalado) é ignored. Rodam com `cargo test --
--ignored` quando o ambiente está configurado.

O teste `erro_implementa_display_para_cada_variante` **não** é ignored —
não invoca subprocess, é unit puro de `Display`.

### D4 — `ErroFork` não deriva `PartialEq`/`Eq`/`Clone`

Diferente de outros enums de erro do projeto (que derivam tudo).
`std::io::Error` e `std::string::FromUtf8Error` não implementam `PartialEq`
nem `Clone`, então as variantes `FalhaSubprocess` e `StdoutInvalido` impedem
o derive. Trade-off: nos testes, o pattern matching usa `match` em vez de
`assert_eq!` — pequena verbosidade pelo realismo (carregar a causa original
do erro vale mais que matcher fácil).

### D5 — `pub mod fork`, não reexport plano

O prompt deu opção: `pub mod fork;` ou `pub use fork::{...}`. Escolhi
`pub mod fork` — preserva o namespace `lente_infra::fork::invocar_fork`,
`lente_infra::fork::ErroFork`. Vantagens:
- Discoverability: quem importa `lente_infra::fork::*` vê o módulo como
  entidade coerente.
- Não polui o namespace raiz de `lente_infra` (que já tem `extrair_grafo`,
  `ErroAdaptador`, etc.).

### D6 — Sem mudança no `Cargo.toml`

`std::process::Command` é stdlib. `std::string::FromUtf8Error` idem. Não
precisei adicionar nenhuma dependência. `lente_infra` continua com apenas
`lente_core` + `serde` + `serde_json`.

---

## Sinalização para o L4 wiring (próximo prompt)

O L4 vai compor o pipeline conforme o modo da CLI:

- **Modo `--pacote NOME`**: chama `fork::invocar_fork(NOME)` para obter o
  JSON, depois passa para algo que desserializa (`traducao::traduzir` direto
  sobre um `GrafoDTO` parseado por `serde_json::from_str`). Cuidado: a
  `traducao` hoje é `pub(crate)`; ou o L4 vive dentro do `lente_infra` (como
  função pública), ou a `traducao` precisa ser exposta (refator localizado).
- **Modo `--grafo arquivo.json`**: pula o `fork`, lê o arquivo do disco,
  desserializa direto.

Os dois modos depois caminham igual: `Grafo` → `lente_resolve` → `raio.rs`.

Atenção também à pendência declarada no laudo 0016: como o wiring vai
compor `resolve → raio`, ele define se a garantia "raio só vê grafo
resolvido" existe. Se o L4 chamar sempre resolve antes de raio, a dívida
"raio-por-id" continua latente sem dor.

---

## O que NÃO entra (cascata)

- **L4 wiring**: compor extrair → resolver → raio. Próximo prompt.
- **L2 CLI**: parser de argumentos + apresentação. Terceiro prompt.
- **Versionamento do fork**: detectar versão e avisar incompatibilidade — não
  precisa agora (ADR-0001 já estipula a versão).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | Invocador L3 do fork: `invocar_fork(pacote)` + `ErroFork`. Aditivo, std-only, cwd herdado, testes E2E ignored. Primeiro dos três prompts da composição (fork → wiring → CLI). 77 verdes + 4 ignored. Pureza preservada. | `03_infra/src/fork.rs` (novo), `03_infra/src/lib.rs` |
