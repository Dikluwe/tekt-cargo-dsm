# Prompt: Invocador do Fork (`lente_infra::fork`)

**Camada**: L3 — Infraestrutura (I/O permitido: processo externo)
**Criado em**: 2026-05-28
**Estado**: `PROPOSTO`
**Decisões de origem**: decisão do autor (CLI aceita `--pacote` E `--grafo`,
exige L3 novo que invoque o fork); ADR-0001 (fork como fonte); spec
`forma-organizada.md` Limite 1 (sysroot sempre ligado).
**Primeiro de três prompts da composição** (L3 invocador → L4 wiring → L2 CLI).
**Pré-requisito**: fork 0.27.0 instalado no PATH (`cargo install --git ...`).
**Arquivos afetados**: `03_infra/src/fork.rs` (novo), `03_infra/Cargo.toml`,
`03_infra/src/lib.rs` (reexport).

---

## Contexto

A CLI da lente aceita dois modos de entrada (decisão do autor): `--grafo
arquivo.json` (lê JSON pronto, já desserializado pelo `lente_infra` existente)
ou `--pacote NOME` (chama o fork e captura o JSON). O segundo modo exige um
componente que **invoque o fork como subprocess externo** — leitura de I/O,
portanto L3.

Hoje o `lente_infra` desserializa JSON via `serde`, mas não sabe **executar
o fork**. Este prompt cria essa peça: um invocador encapsulado que esconde o
detalhe de subprocess.

---

## Restrições estruturais

- **L3 — admite I/O.** Processo externo é I/O legítimo em L3.
- **Não toca o `lente_core`.** Continua puro.
- **Não toca o `lente_infra::traducao`.** Esta peça é independente — produz
  uma string JSON, que o `traducao` existente desserializa.
- **Limite 1 da spec**: `--sysroot` sempre ligado. Não é configurável pelo
  chamador (é política do projeto, não escolha do usuário).
- **Aditivo.** Os campos pré-existentes do crate `lente_infra` continuam
  inalterados.

---

## O que construir

### Módulo `03_infra/src/fork.rs`

Função pública única:

```rust
/// Invoca `cargo modules export-json --sysroot --compact --package <pacote>`
/// como subprocess no diretório atual, captura o stdout, e devolve o JSON
/// como String.
///
/// Erro se: subprocess falha (fork não instalado, pacote inexistente, etc.),
/// stdout não é UTF-8 válido, ou status de saída != 0.
pub fn invocar_fork(pacote: &str) -> Result<String, ErroFork>
```

Implementação:
- Usar `std::process::Command::new("cargo")` com args `["modules",
  "export-json", "--sysroot", "--compact", "--package", pacote]`.
- Capturar stdout (texto JSON) e stderr (mensagens de erro do fork).
- Se o status de saída do subprocess for ≠ 0, ler stderr e retornar erro
  contendo essa mensagem (ajuda a debugar — ex.: "pacote X não encontrado no
  workspace").
- Se o stdout não for UTF-8 válido, erro próprio.
- Se o `Command::spawn` ou `wait_with_output` falhar (fork ausente do PATH,
  permissão), erro próprio.

### Tipo de erro `ErroFork`

Enum com variantes mínimas cobrindo os modos de falha:

```rust
pub enum ErroFork {
    /// Falha ao iniciar o subprocess (cargo não no PATH, permissão, etc.)
    FalhaSubprocess(std::io::Error),
    /// Fork retornou status de erro; mensagem extraída do stderr.
    StatusErro { codigo: Option<i32>, stderr: String },
    /// Stdout do fork não é UTF-8 válido.
    StdoutInvalido(std::string::FromUtf8Error),
}
```

`impl Display` para cada variante (texto humano-legível para o L2 traduzir
depois). Não derive `serde` — `ErroFork` é interno do projeto, não serializado.

### Cargo.toml

Adicionar nada — `std::process` é stdlib. Mantém `lente_infra` com apenas
`serde`/`serde_json` como deps externas.

### Reexport

`03_infra/src/lib.rs` reexporta `pub mod fork;` (ou `pub use fork::{invocar_fork, ErroFork};`
— escolha do gerador).

---

## Critérios de Verificação

```
Dado um pacote VÁLIDO existente no workspace atual (use o próprio lente_core)
Quando invocar_fork("lente_core") é chamado
Então retorna Ok(String) com JSON parseável (pode validar com serde_json::from_str)

Dado um pacote INEXISTENTE
Quando invocar_fork("pacote_que_nao_existe") é chamado
Então retorna Err(ErroFork::StatusErro { stderr: <mensagem do fork> })

Dado o invocador rodado no diretório raiz do projeto-lente
Quando invocar_fork("lente_core") é chamado
Então o JSON resultante contém pelo menos um nó com path "ErroRaio" — sanidade
de que o fork rodou e capturou o crate certo
```

Casos a cobrir:
- Sucesso com pacote válido (lente_core, lente_infra — pacotes do próprio workspace
  servem como fixture viva).
- Pacote inexistente — erro com mensagem útil.
- (Não testar "cargo ausente do PATH" — exigiria mocar ambiente; aceitar como
  caminho não-coberto por teste, mas registrado no Display do erro.)

**Importante para testes**: como o invocador roda `cargo modules` de verdade
no `cwd`, os testes precisam ser rodados a partir do diretório raiz do
projeto-lente. Se o gerador preferir, pode tornar o `cwd` configurável via
parâmetro (`invocar_fork_em(pacote, dir)`) para facilitar testes — decisão
do gerador, registrar no laudo.

---

## Resultado esperado

- Módulo `fork.rs` em `03_infra/src/` com `invocar_fork` e `ErroFork`.
- Reexport em `lib.rs`.
- Testes inline que rodam o fork de verdade (sanidade) e cobrem erro de
  pacote inexistente.
- Workspace verde (não-regressão dos testes existentes).
- **Pureza**: `cargo tree -p lente_core` continua só o crate.
- **Laudo**: decisões sobre cwd configurável vs fixo, qualquer particularidade
  da invocação do subprocess, e sinalização para o L4 (próximo prompt: usa
  `invocar_fork` quando o modo é `--pacote`, e usa `lente_infra::traducao`
  diretamente quando é `--grafo`).

---

## O que NÃO entra (cascata)

- **L4 wiring**: compor o pipeline (extrair → resolver → calcular_raio).
  Próximo prompt.
- **L2 CLI**: parsing de argumentos, formatação. Terceiro prompt.
- **Versionamento do fork**: o fork 0.27.0 é assumido instalado. Detectar
  versão do fork e avisar incompatibilidade fica para depois (não-necessário
  agora; o usuário instala a versão certa conforme ADR-0001).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-28 | Invocador do fork em L3: encapsula `cargo modules export-json --sysroot --compact --package X` como subprocess, devolve JSON ou ErroFork. Primeiro dos três prompts da composição. | 03_infra/src/fork.rs (novo), 03_infra/src/lib.rs |
