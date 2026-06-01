# Prompt: Adaptador da Fonte (L3)

**Camada**: L3 — Infraestrutura
**Criado em**: 2026-05-27
**Estado**: `PROPOSTO`
**Decisões de origem**: ADR-0001 (fonte = fork do cargo-modules),
ADR-0002 (modelagem do JSON), ADR-0003 (workspace Cargo), spec
`forma-organizada.md` (com 5 limites)
**Depende de**: `lente_core` (tipos `Grafo`, `No`, `Aresta`, `Path`,
`Relation`, `Visibility`, `Kind`, `ValorDesconhecido`), fork do cargo-modules
instalado e no PATH
**Arquivos a gerar**: `Cargo.toml` (raiz, workspace), `03_infra/Cargo.toml`,
`03_infra/src/lib.rs`, e os módulos do adaptador (estrutura interna a decidir)

---

## Contexto

Este é o primeiro componente fora de `lente_core`. Ele fecha a entrada da
pipeline da lente: invoca o fork do `cargo-modules` (binário externo) sobre um
crate Rust, captura o JSON que ele produz, e o materializa nos tipos puros de
`lente_core` (`Grafo` e companhia), validando os enums na borda.

Sem este componente, a lente só roda contra grafos construídos à mão nos
testes. Com ele, ela passa a operar sobre crates Rust reais.

L3 é a camada onde o serde finalmente entra no projeto, e onde o ADR-0003
(workspace Cargo) se materializa: este componente cria o crate `lente_infra`,
membro novo do workspace.

---

## Restrições Estruturais

- **Camada L3 — admite dependências externas e I/O.** Pode usar `serde`,
  `serde_json`, executar subprocessos (`std::process::Command`), ler stdout.
- **Pureza de `lente_core` deve permanecer.** Este componente importa
  `lente_core` mas não modifica nada lá. `cargo tree -p lente_core` continua
  mostrando só o crate.
- **Gravidade Tekt**: `lente_infra` depende de `lente_core`. Nenhuma outra
  direção é permitida — `lente_core` nunca importa `lente_infra`.
- **Subprocesso, não biblioteca** (decisão registrada): o adaptador invoca o
  binário do fork via `std::process::Command`, não importa o `cargo-modules`
  como crate. Isso mantém a fronteira honesta e evita arrastar o rust-analyzer
  como dependência de compilação.
- **Sem cache** (decisão registrada): este componente é puro invoca-e-devolve.
  Toda invocação re-roda o fork. Cache é responsabilidade de componente
  futuro, não deste.
- **Validação na borda** (ADR-0002, Decisão 1): a desserialização valida os
  enums. Texto desconhecido em `kind`, `visibility` ou `relation` retorna erro
  — não panic, não valor default. O `lente_core` já tem `TryFrom<&str>` para
  isso; o adaptador o usa.
- **Sysroot sempre ligado**: a invocação canônica é `cargo modules export-json
  --sysroot --compact`. O `--sysroot` é política da lente (ADR-0001, Limite 1
  da spec) — sempre presente, não opcional aqui.

---

## Instrução

### Estrutura do workspace (materialização do ADR-0003)

Criar o `Cargo.toml` da raiz do projeto como workspace, declarando os membros:

```toml
[workspace]
resolver = "2"
members = ["01_core", "03_infra"]
```

Manter o `01_core/Cargo.toml` como está (não modificar `lente_core`).

Criar `03_infra/Cargo.toml` como crate `lente_infra`, com:

- `edition = "2024"`, `rust-version = "1.91"` (alinhado a `lente_core` e ao
  fork).
- Dependência interna: `lente_core = { path = "../01_core" }`.
- Dependências externas mínimas necessárias: `serde` (com derive),
  `serde_json`. Se precisar localizar binário no PATH ou propagar `cargo
  metadata`, adicionar o mínimo necessário e justificar no laudo.

Criar `03_infra/src/lib.rs` e os módulos internos. A divisão interna é decisão
do gerador (sugestão: um módulo para invocação do subprocesso, um para
desserialização do JSON, um para tradução para os tipos de `lente_core`).
Documentar no laudo a divisão escolhida e por quê.

### Funcionalidade

O componente expõe uma operação principal: dado um caminho para um crate Rust
(ou um diretório que contém um crate), invocar o fork e retornar um `Grafo` (do
`lente_core`).

Assinatura sugerida (ajustável conforme o gerador julgar idiomático):

```rust
pub fn extrair_grafo(caminho_crate: &Path) -> Result<lente_core::Grafo, ErroAdaptador>
```

Onde `ErroAdaptador` é um enum próprio do `lente_infra` cobrindo os modos de
falha (ver abaixo).

### Invocação do subprocesso

- Comando: `cargo modules export-json --sysroot --compact`, executado no
  diretório do crate-alvo.
- O binário deve estar instalado e no PATH (`cargo install --git
  https://github.com/Dikluwe/cargo-modules cargo-modules`). Se não estiver, é
  modo de falha (ver erros).
- Capturar stdout (o JSON) e stderr (mensagens de erro, se houver).
- O exit code não-zero é falha; reportar com a mensagem do stderr para o erro
  ser diagnosticável.

### Desserialização e validação na borda

- Definir structs internas ao `lente_infra` com `#[derive(Deserialize)]` que
  espelhem a forma do JSON do fork. Estas são struct-espelho, não os tipos do
  `lente_core` — servem só para `serde_json` parsear. Os campos (string para
  kind/visibility/relation) ficam como `String` aqui.
- Após parsear, converter para os tipos do `lente_core` usando o `TryFrom<&str>`
  que já existe — cada `kind`, `visibility`, `relation` valida na borda. Se
  qualquer um falhar, o erro vira modo de falha do adaptador (ver erros).
- Atenção ao campo `crate` do JSON, que vira `crate_name` no `lente_core`
  (laudo 0001, D5).

### Garantia dos invariantes da spec

Após construir o `Grafo`, verificar (ou ao menos assegurar pela construção) os
invariantes da spec (`forma-organizada.md`):

- Identidade por `path` (cada path em `nodes` é único).
- Integridade referencial (todo `from`/`to` de aresta referencia um `path`
  existente em `nodes`).
- Valores fechados respeitados (já garantido pelas conversões via enum).

Se algum invariante falhar (fork produziu JSON com aresta órfã ou path
duplicado), é modo de falha — não corrigir silenciosamente.

### Modos de falha (`ErroAdaptador`)

Enum cobrindo, no mínimo:

- Binário não encontrado / falha ao executar o subprocesso.
- Subprocesso terminou com exit code não-zero (incluir stderr na mensagem).
- JSON inválido (falha de parsing).
- Valor desconhecido em enum (kind, visibility, relation) — embrulhar o
  `ValorDesconhecido` do `lente_core`.
- Invariante violado (path duplicado ou aresta órfã).

Cada variante deve permitir mensagem diagnóstica clara — quem usa a lente vai
ver o erro, então ele precisa indicar o que deu errado.

---

## Critérios de Verificação

```
Dado um crate Rust válido (pode ser um fixture pequeno criado no teste)
Quando extrair_grafo é chamado com o caminho dele
Então retorna Ok(Grafo) com pelo menos o nó-raiz e os nós esperados

Dado o mesmo crate
Quando o Grafo é inspecionado
Então todo from/to de aresta referencia um path existente em nodes
E todo path em nodes é único
E todos os kind/visibility/relation são variantes válidas dos enums

Dado um caminho que não é um crate Rust válido
Quando extrair_grafo é chamado
Então retorna Err(ErroAdaptador) com mensagem diagnóstica
(não panic, não Grafo vazio)

Dado um cenário onde o binário cargo-modules não está no PATH
Quando extrair_grafo é chamado
Então retorna Err(ErroAdaptador::BinarioNaoEncontrado) ou equivalente
```

Casos a cobrir nos testes:

- Caminho inexistente / sem Cargo.toml.
- JSON malformado (simulado com um mock de stdout, se viável, ou aceito como
  "modo de falha coberto por inspeção de código" se simular for desproporcional).
- Conversão bem-sucedida de um crate-fixture conhecido (criar um pequeno crate
  em `tests/` se necessário, ou usar o próprio `lente_core` como crate-fixture).

**Nota sobre testes**: os testes que invocam o subprocesso real precisam do
`cargo-modules` instalado no ambiente de teste. Isso é aceitável (o ADR-0001
declara o fork como pré-requisito). Se algum teste precisar pular quando o
binário não estiver disponível, fazer isso explicitamente e documentar.

---

## Resultado Esperado

- `Cargo.toml` da raiz (workspace, membros).
- `03_infra/Cargo.toml` (`lente_infra`).
- `03_infra/src/lib.rs` e módulos internos.
- A função pública de extração, retornando `Result<Grafo, ErroAdaptador>`.
- Testes inline cobrindo os critérios.
- **Verificação de não-regressão**: `cargo test` na raiz roda os testes dos
  dois crates; `lente_core` continua passando seus 22 testes. `cargo tree -p
  lente_core` continua mostrando só o crate (pureza de L1 preservada).
- **Laudo de execução** em `00_nucleo/lessons/`: o que o prompt pediu, o que
  foi gerado, decisões tácitas (divisão interna do `lente_infra`, forma exata
  da assinatura, escolhas de erro, como o teste de fixture foi montado).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Criação inicial. Primeiro componente fora de lente_core. Materializa ADR-0003 (workspace) e fecha a entrada da pipeline da lente. | Cargo.toml (raiz), 03_infra/* |
