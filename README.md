# `crystalline-dsm`

Uma ferramenta de linha de comando (CLI) em Rust para extração e visualização de dependências de código por meio de **Dependency Structure Matrix (DSM)**, projetada com conformidade à Arquitetura Cristalina (Tekt v1.4).

---

## 📐 Arquitetura do Projeto

Este projeto utiliza o framework de desenvolvimento **Tekt v1.4**, dividindo suas responsabilidades em um Cargo Workspace multi-crate para impor garantias estruturais ao nível do compilador:

```
                  04_wiring (Fiação - Crate Binária CLI)
                 /    |    \
                /     |     \
               /      |      \
    02_shell (Casca)  |   03_infra (Infraestrutura)
     (Interface UI)   |   (syn parser, HTML renderer)
                \     |     /
                 \    |    /
                  \   |   /
                  01_core (Núcleo - Algoritmos Puros)
                      |
                  00_nucleo (Semente - Prompts e ADRs)
```

### Divisão de Responsabilidades

| Diretório / Crate | Camada | Função Principal |
|---|---|---|
| [`00_nucleo/`](file:///home/dikluwe/Documentos/Antigravity/tekt-cargo-dsm/00_nucleo/) | `L₀` (Semente) | Contém prompts estruturados e ADRs. Nenhum código é permitido. |
| [`01_core/`](file:///home/dikluwe/Documentos/Antigravity/tekt-cargo-dsm/01_core/) | `L₁` (Núcleo) | Algoritmos de grafos determinísticos, ciclos (Tarjan), particionamento de DSM. Zero I/O. |
| [`02_shell/`](file:///home/dikluwe/Documentos/Antigravity/tekt-cargo-dsm/02_shell/) | `L₂` (Casca) | Formatação de console, logs, mensagens de erro e UI da CLI. |
| [`03_infra/`](file:///home/dikluwe/Documentos/Antigravity/tekt-cargo-dsm/03_infra/) | `L₃` (Infra) | Extração física de AST com `syn`, mapeamento de workspaces com `cargo_metadata`, escrita de HTML e leitura de `crystalline.toml`. |
| [`04_wiring/`](file:///home/dikluwe/Documentos/Antigravity/tekt-cargo-dsm/04_wiring/) | `L₄` (Fiação) | Ponto de entrada (`main.rs`), parser de argumentos (`clap`) e orquestração/composição de fluxo do sistema. |

---

## ⚖️ Documentos Arquiteturais e Referências

As decisões fundamentais e planos de desenvolvimento estão registrados em:
*   **ADR de Criação**: [ADR-0001 (Criação de Ferramenta DSM)](file:///home/dikluwe/Documentos/Antigravity/tekt-cargo-dsm/00_nucleo/adr/crystalline-dsm-adr-0001-criacao-ferramenta.md)
*   **Plano de Execução**: [Roadmap de Execução](file:///home/dikluwe/Documentos/Antigravity/tekt-cargo-dsm/00_nucleo/crystalline-dsm-roadmap.md)
*   **Análise de Viabilidade Tekt**: [Análise de Viabilidade Tekt](file:///home/dikluwe/.gemini/antigravity/brain/dc719a98-7e15-4f04-b81d-7bd7e6e46828/artifacts/analise_tekt_no_dsm.md) (com o trade-off de dependências do `syn` e estratégias de validação por fixtures).

---

## 🛠️ Como Iniciar o Desenvolvimento

Este workspace Cargo está configurado para compilar os subprojetos individualmente. Para verificar a compilação do esqueleto do projeto:

```bash
cargo check
```
