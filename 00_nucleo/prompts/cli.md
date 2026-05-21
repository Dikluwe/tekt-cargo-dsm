# Prompt: Esqueleto de CLI e Fundação (Passo 0.3)

**Camada**: L₄ (Fiação) e L₂ (Casca)  
**Criado em**: 2026-05-20  
**Status**: IMPLEMENTADO (revisado)
**Revisão**: `cli_output_flags.md` (pipeline real substituiu o mock; flags `--output` default `./graph.json` e `--emit-trees`; flag `--format` removida; `shell::format_summary` + `format_error` adicionados em L₂).
**Arquivos gerados**:
*   `04_wiring/src/main.rs` (Fiação - CLI Clap e Orquestração mockada)
*   `02_shell/src/lib.rs` (Casca - Formatador de mensagens e interface de saída)
*   `04_wiring/tests/integration_tests.rs` (Teste de Integração de CLI)
*   `04_wiring/tests/fixtures/empty-workspace/Cargo.toml` (Fixture de teste)
*   `04_wiring/tests/fixtures/empty-workspace/src/lib.rs` (Fixture de teste)

---

## Contexto

Este prompt estabelece o esqueleto inicial de linha de comando (`crystalline-dsm-cli`) do projeto. O objetivo é configurar o parser de argumentos, a interface de exibição amigável de console e garantir que a estrutura física do workspace esteja pronta para receber as integrações de análise de grafos e do parser estático de arquivos nos passos subsequentes.

---

## Restrições Estruturais

*   **Fiação (`04_wiring`)**:
    *   Deve conter a definição da CLI usando a crate `clap` (estilo derive).
    *   Não deve conter lógica de negócio direta de validação ou geração física de relatórios.
    *   Deve invocar a Casca (`L₂`) para formatação e saída de texto no terminal.
    *   Deve fazer a criação de arquivo simulado de saída (neste Passo 0.3) para passar nos testes.
*   **Casca (`02_shell`)**:
    *   Deve conter apenas funções de formatação e estruturação de mensagens amigáveis direcionadas ao usuário.
    *   Não deve realizar I/O de disco.
    *   Deve ser acoplada apenas a `L₁` (Núcleo) ou `L₀`.
*   **Linhagem**: Todos os arquivos gerados contendo código Rust devem conter o header de linhagem Cristalina apontando para este prompt:
    ```rust
    /**
     * Crystalline Lineage
     * @prompt 00_nucleo/prompts/cli.md
     * @layer L<n>
     * @updated 2026-05-20
     */
    ```

---

## Instrução

1.  **Crate de Fiação (`04_wiring`)**:
    *   Configure `clap` para aceitar:
        *   Um argumento posicional obrigatório: `workspace_path` (caminho para a raiz do workspace Cargo a ser analisado).
        *   Flag `--output` / `-o`: Caminho para salvar a saída do arquivo (padrão: `./dsm.html`).
        *   Flag `--format` / `-f`: Formato de saída, limitando as opções aceitas para `html` ou `json` (padrão: `html`).
    *   Ao executar, o programa principal deve:
        1.  Chamar o formatador da Casca (`02_shell`) para imprimir na saída padrão a mensagem confirmando o início da análise.
        2.  Como mock deste passo, criar um arquivo vazio (ou com conteúdo texto mockado) no caminho fornecido pela flag `--output` para validar o fluxo de I/O nos testes de integração.
        3.  Retornar `exit code 0` em caso de sucesso.

2.  **Crate de Casca (`02_shell`)**:
    *   Crie uma struct ou funções puras que recebam dados de análise e retornem mensagens formatadas.
    *   Exemplo: `format_start_analysis(path: &str) -> String` que retorna `"Analisando o workspace em: [path]..."`.

3.  **Fixture de Teste (`tests/fixtures/empty-workspace/`)**:
    *   Crie um workspace Cargo mockado mínimo contendo um arquivo `Cargo.toml` e um `src/lib.rs` vazio.

4.  **Testes de Integração (`tests/integration_tests.rs`)**:
    *   Use a crate `assert_cmd` ou chame o binário do `crystalline-dsm-cli` via comando Rust (`std::process::Command`).
    *   Crie um teste que chame o binário passando a fixture `empty-workspace`.
    *   Verifique se o binário retorna sucesso (código `0`), se a mensagem correta é impressa na saída e se o arquivo `./dsm.html` (ou outro especificado via `--output`) é de fato criado no disco.

---

## Critérios de Verificação

```
Dado um binário compilado crystalline-dsm-cli
Quando executado com as flags '--version'
Então ele deve imprimir com sucesso a versão atual do CLI

Dado a fixture de controle 'tests/fixtures/empty-workspace/'
Quando a CLI é executada apontando para a fixture e sem flags adicionais
Então ela deve imprimir a confirmação da análise, retornar status 0 e criar o arquivo './dsm.html' no diretório de trabalho

Dado a fixture de controle e as flags '-o output.json -f json'
Quando a CLI é executada
Então ela deve retornar status 0 e criar o arquivo 'output.json' no local configurado
```

---

## Resultado Esperado

*   O comando `cargo run --bin crystalline-dsm-cli` funciona.
*   Os testes em `tests/integration_tests.rs` rodam e passam com sucesso via `cargo test`.
*   A estrutura física está pronta e protegida contra violações de importação acidentais de camadas.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-20 | Criação inicial e conclusão do Passo 0.3 | `02_shell/src/lib.rs`, `04_wiring/src/main.rs`, `04_wiring/tests/integration_tests.rs`, `04_wiring/tests/fixtures/empty-workspace/*` |
