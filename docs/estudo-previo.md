# Estudo Prévio: Referências Técnicas e DSM

**Passo**: 0.1  
**Objetivo**: Investigação sobre travessia de módulos Rust, tratamento de compilação condicional, metadados de workspaces e modelos de dados para matrizes de dependência (DSM).

---

## 1. Resolução de Declarações `mod` no `cargo-modules`

O `cargo-modules` analisa a árvore de módulos de um crate Rust a partir do ponto de entrada principal (`main.rs` ou `lib.rs`) fazendo parsing da Árvore de Sintaxe Abstrata (AST) via crate `syn`.

### Mecânica de Resolução de Módulos
*   **Busca por Declarações**: O parser identifica itens da AST correspondentes a declarações do tipo `mod nome_do_modulo;` (sem corpo `{}`, que indicam arquivos externos).
*   **Travessia de Filesystem**: Para cada declaração `mod foo;` encontrada em um módulo pai localizado em `pasta_pai/`, a ferramenta tenta resolver o arquivo físico seguindo a convenção oficial do compilador Rust:
    1.  Procura por um arquivo na mesma raiz com o nome do módulo: `pasta_pai/foo.rs`.
    2.  Procura por um arquivo de módulo interno dentro de uma subpasta correspondente: `pasta_pai/foo/mod.rs`.
*   **Recursividade**: A ferramenta repete o parsing do AST e a busca recursiva para cada módulo mapeado, construindo a árvore de nós interna do crate de cima para baixo.

---

## 2. Tratamento de `#[cfg(...)]` pelo `cargo-modules`

A compilação condicional em Rust (`#[cfg(...)]` ou `#[cfg_attr(...)]`) dita se um módulo ou item existirá ou não em tempo de compilação. O `cargo-modules` lida com isso de maneira análoga ao próprio compilador:

*   **Configuração Ativa**: A análise é contextual e depende das flags fornecidas pelo usuário em tempo de execução da CLI (como `--all-features`, `--no-default-features`, `--features <FEATURES>` ou `--cfg-test`).
*   **Avaliação do AST**: Ao analisar o AST via `syn`, a ferramenta lê os atributos `#[cfg]` colocados acima das declarações `mod`. 
*   **Filtragem de Branches**:
    *   Se a expressão condicional do `#[cfg]` for avaliada como **verdadeira** sob o contexto de features/testes declarados para a execução, o módulo é processado recursivamente.
    *   Se a expressão for **falsa**, a declaração `mod` é ignorada e o arquivo físico correspondente não é lido. A árvore de dependências resultante refletirá apenas os módulos que fariam parte daquele binário específico compilado.
*   **Implicação para o `crystalline-dsm`**: O MVP deve documentar e escolher uma estratégia. A recomendação do Tekt é permitir passar parâmetros de features para o parser para refletir uma configuração de build real de forma precisa, espelhando a mesma restrição do compilador Rust.

---

## 3. Uso do `cargo_metadata` para Resolução de Workspaces

A crate `cargo_metadata` expõe a estrutura física de múltiplos crates dentro de um workspace Cargo.

### Fluxo de Mapeamento
1.  **Execução do Comando**: Executa-se o `cargo metadata` em formato de comando e parseia-se o resultado para a struct `Metadata`:
    ```rust
    use cargo_metadata::MetadataCommand;
    
    let metadata = MetadataCommand::new()
        .exec()
        .expect("Falha ao rodar cargo metadata");
    ```
2.  **Identificação de Membros**: O campo `workspace_members` na struct `Metadata` fornece um vetor de `PackageId` (`Vec<PackageId>`) contendo apenas as crates que fazem parte do workspace (excluindo as dependências de terceiros baixadas do crates.io).
3.  **Localização dos Caminhos**: A struct `Metadata` funciona como um índice para os pacotes mapeados. Cada pacote possui o caminho absoluto do seu respectivo `Cargo.toml` (`manifest_path`):
    ```rust
    for package_id in &metadata.workspace_members {
        let package = &metadata[package_id];
        let cargo_toml_path = &package.manifest_path; // Ex: /caminho/para/01_core/Cargo.toml
        let crate_root_dir = cargo_toml_path.parent().unwrap(); // Ex: /caminho/para/01_core
        
        println!("Crate: {} no caminho: {}", package.name, crate_root_dir);
    }
    ```
4.  **Pontos de Entrada**: O `cargo_metadata` também indica os *targets* do pacote (lib, binários, testes), facilitando achar o ponto de entrada correto (`src/lib.rs` ou `src/main.rs`) para alimentar o nosso parser.

---

## 4. Análise Comparativa de Implementações de DSM

A representação de dependências em uma matriz DSM varia dependendo do nível de detalhamento que a ferramenta quer apresentar. Três ferramentas industriais se destacam:

### A. Lattix LDM (Comercial)
*   **Foco**: Gestão arquitetural geral e análise estrutural de sistemas de software complexos.
*   **Célula da Matriz (Valor/Peso)**:
    *   **Contagem de Elementos (Peso)**: A célula exibe um valor numérico inteiro que representa a força do acoplamento agregado (ex: a quantidade de dependências de folha entre um subsistema A e B).
    *   **Métricas Complexas**: Permite configurar a célula para exibir a força em percentual baseado no tamanho dos subsistemas, ou lidar com dependências diretas vs. indiretas (caminho mais curto).

### B. NDepend (.NET - Comercial)
*   **Foco**: Análise estática profunda para a plataforma .NET.
*   **Célula da Matriz (Valor/Peso)**:
    *   **Contagem de Membros**: Exibe o número de acoplamentos estruturais discretos entre as classes/módulos (ex: número exato de chamadas de métodos, acesso a campos ou referências a tipos).
    *   **Codificação por Cor**: Utiliza cores nas células (azul, verde, preto) indicando a direção do acoplamento (se a linha usa a coluna, a coluna usa a linha, ou se há acoplamento bidirecional/ciclo).

### C. Structure101 (Comercial)
*   **Foco**: Visualização de fluxo de dependências, controle de ciclos e arquiteturas baseadas em hierarquias.
*   **Célula da Matriz (Valor/Peso)**:
    *   **Contagem de Referências**: O valor numérico representa o volume de referências discretas item-a-item (chamadas, dependências de tipo, imports). A força numérica ajuda a detectar "hotspots" de complexidade e violações conceituais de camadas de maneira visual rápida.

### Resumo de Tipos de Células
1.  **Binário**: Apenas sinaliza a existência ou ausência (ex: "X" ou "1"). Útil para detecção básica de ciclos estruturais puros.
2.  **Por Contagem de Referências (Imports)**: Conta o número físico de declarações de uso (ex: número de declarações `use` de um módulo para outro). É a mais adequada para o MVP do `crystalline-dsm`.
3.  **Por Acoplamento Semântico**: Avalia o número real de chamadas a métodos e tipos na AST compilada (requer resolução semântica). Fora do escopo do MVP.
