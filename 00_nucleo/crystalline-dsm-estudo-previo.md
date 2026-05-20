# Estudo Prévio: Referências Técnicas e DSM

**Passo**: 0.1

**Objetivo**: Investigação sobre travessia de módulos Rust, tratamento de compilação condicional, metadados de workspaces e modelos de dados para matrizes de dependência (DSM).

---

## 1. Resolução de Declarações `mod` no `cargo-modules`

O `cargo-modules` analisa a árvore de módulos de um crate Rust a partir do ponto de entrada principal (`main.rs` ou `lib.rs`) fazendo parsing da Árvore de Sintaxe Abstrata (AST) via crate `syn`.

### Mecânica de Resolução de Módulos

- **Busca por Declarações**: O parser identifica itens da AST correspondentes a declarações do tipo `mod nome_do_modulo;`.
- **Travessia de Diretórios**: Para cada declaração `mod foo;` em um módulo pai localizado em `pasta_pai/`, a ferramenta busca o arquivo físico seguindo a convenção oficial do compilador Rust:
  1. Procura por um arquivo: `pasta_pai/foo.rs`.
  2. Procura por uma subpasta com arquivo base: `pasta_pai/foo/mod.rs`.
- **Módulos Inline**: Declarações com corpo no próprio arquivo, como `mod foo { ... }`. Não exigem busca de arquivo externo, mas o parser precisa continuar a leitura dentro do bloco.
- **Caminhos Customizados**: O uso do atributo `#[path = "..."]` força a leitura de um arquivo em um local diferente do padrão.
- **Recursividade**: A ferramenta repete o parsing da AST e a busca para cada módulo mapeado, construindo a árvore de dependências de cima para baixo.

---

## 2. Tratamento de `#[cfg(...)]` pelo `cargo-modules`

A compilação condicional em Rust (`#[cfg(...)]` ou `#[cfg_attr(...)]`) define se um módulo, função ou item existirá em tempo de compilação. A condição pode depender de features ativas, sistema operacional alvo, arquitetura, ou outras flags. Por exemplo, um módulo marcado com `#[cfg(feature = "x")]` só é compilado quando a feature `x` está ativa no momento do build.

Isso importa para o `crystalline-dsm` porque o grafo de dependências de um crate pode ter formas diferentes dependendo de quais features estão ativas. A ferramenta precisa decidir como tratar isso na sua análise estática.

**Comportamento a ser verificado:** A forma exata como o `cargo-modules` lida com atributos `#[cfg]` precisa ser confirmada através da leitura do seu código fonte. Existem duas possibilidades para o design do `crystalline-dsm`:

1. A ferramenta avalia o contexto de compilação (features ativas) e ignora os módulos onde a condição é falsa.
2. A ferramenta ignora os atributos `#[cfg]` e mapeia todos os módulos encontrados no código fonte.

O projeto precisará escolher e documentar uma destas abordagens após a verificação no código fonte do `cargo-modules`.

---

## 3. Uso do `cargo_metadata` para Resolução de Workspaces

A crate `cargo_metadata` expõe a estrutura física de múltiplos crates dentro de um workspace Cargo.

### Fluxo de Mapeamento

1. **Execução do Comando**: Executa o comando `cargo metadata` e converte o resultado para a struct `Metadata`:

```rust
use cargo_metadata::MetadataCommand;

let metadata = MetadataCommand::new()
    .exec()
    .expect("Falha ao rodar cargo metadata");
```

2. **Identificação de Membros**: O campo `workspace_members` fornece uma lista com as crates que fazem parte do workspace, excluindo dependências externas baixadas da internet.

3. **Localização dos Caminhos**: A struct `Metadata` funciona como um índice. Cada pacote possui o caminho absoluto do seu respectivo `Cargo.toml`:

```rust
for package_id in &metadata.workspace_members {
    let package = &metadata[package_id];
    let cargo_toml_path = &package.manifest_path;
    let crate_root_dir = cargo_toml_path.parent()
        .expect("O manifest_path sempre tem um diretório pai");

    println!("Crate: {} no caminho: {}", package.name, crate_root_dir);
}
```

4. **Pontos de Entrada**: O comando também indica os alvos do pacote (lib, binários, testes) para ajudar a achar o ponto de entrada correto (`src/lib.rs` ou `src/main.rs`) que alimentará o parser.

---

## 4. Análise Comparativa de Implementações de DSM

A representação de dependências em uma matriz varia de acordo com o nível de detalhamento.

### A. Lattix LDM

- **Foco**: Gestão arquitetural e análise de sistemas complexos.
- **Célula da Matriz**: Exibe um número inteiro que representa a força do acoplamento agregado (exemplo: quantidade total de dependências entre dois subsistemas). Pode exibir os dados em formato de percentual.

### B. NDepend

- **Foco**: Análise estática para a plataforma .NET.
- **Célula da Matriz**: Exibe o número exato de acoplamentos entre classes. Utiliza um esquema de cores nas células para indicar a direção do acoplamento ou a existência de ciclos.

### C. Structure101

- **Foco**: Visualização de fluxo de dependências e controle de ciclos.
- **Célula da Matriz**: O valor numérico representa o volume de referências item-a-item (chamadas, imports). A recomendação para o MVP do `crystalline-dsm` é adotar este modelo (Contagem de Referências baseada em imports), por ser simples e determinístico.

---

## 5. Limitações Conhecidas da Abordagem Baseada em `syn`

Como a ferramenta utilizará a crate `syn` para ler o código fonte estático, o código não será compilado. Isso gera limitações técnicas para o MVP do `crystalline-dsm`:

1. **Código gerado por Macros**: O parser `syn` lê o texto antes da expansão de macros. Qualquer módulo declarado dentro de uma macro ou importações geradas por macros não serão detectadas.
2. **Atributo `#[path]`**: Caso o projeto decida não implementar a resolução do atributo `#[path = "..."]` para simplificar o desenvolvimento inicial, os arquivos que usam essa técnica não serão encontrados e a ferramenta emitirá um aviso na execução.
3. **Compatibilidade (Rust Edição 2024)**: O parser terá como alvo projetos modernos, utilizando uma versão recente do `syn` (família 2.x). O sistema será testado contra o compilador atual (versão 1.95+). O MVP não suportará projetos legados (como Edição 2015 que usa a sintaxe `extern crate`).
