# Integração dos provedores opcionais por processo

Data: 2026-09-26. Tarefa: integracao-provedores-01a0e01d-6243.

## Auditoria e escopo

Lidos AGENTS.md, spec do grafo, ADRs 0001/0002/0007, contratos @prompt de
extração, wiring, CLI, catálogo e MCP, contratos e implementações dos seis
adaptadores e pacote da tarefa 13. Diretórios não foram considerados prova:
foram conferidos argumentos, códigos de saída, metadados e executadas fixtures
reais com seus helpers/binários já preparados.

A fronteira L3 ExtratorGrafo/SolicitacaoExtracao já existia. O contrato
wiring-analise.md descrevia uma preparação L4 ainda ausente do checkout; não foi
anunciada como implementada. O protótipo 03_infra/typescript também não tinha o
fluxo pronto. A integração usa o helper funcional lab/adaptador-typescript como
ferramenta opcional, preservando o protótipo.

Alterações preexistentes foram conservadas. Esta tarefa não alterou algoritmos
L1, extratores de lab, tradutor, cache Rust ou HTML/formatadores da DSM. Formatou
somente unidades tocadas de infraestrutura, wiring, catálogo e argumentos.
Dependências adicionadas ao wiring: lente_cli, serde e serde_json já disponíveis;
MCP passa a importar o catálogo. Nenhuma dependência Cargo aponta para lab.

## Entrega

- ADR-0008 e contratos próprios de infra-provedores/wiring-provedores/fixtures.
- ExtratorFerramenta implementa a fronteira; envelope L1 continua Json como
  transporte, contexto obrigatório conserva identidade real e metadados.
- Configuração estrita por provedor, caminhos absolutos, disponibilidade de
  executáveis/helpers e incompatibilidades verificadas antes dos subprocessos.
- Execução Unix sem shell, grupo por comando, timeout total e limpeza do grupo
  no encerramento normal/erro/timeout; diretório privado exclusivo por chamada.
- create/verify delegados ao executável real da tarefa 13. Sem reimplementar
  manifesto, hashes ou normalização. C/Go sem digest exigem opt-in; ambiente
  exige opt-in na criação. Preservação de metadata e manifestação C original.
- CLI --provedor/--config-provedor e MCP analisar_provedor com mesmo envelope;
  raio, ranking e estrutura sobre o grafo, sem resolver colisões/filtrar Rust.
- Rust/JSON legados e seus argumentos permanecem; opções externas incompatíveis
  são recusadas em vez de ignoradas.
- Documentação/instalação/exemplos/matriz: docs/adaptadores.md; gerador de configs
  em 03_infra/fixtures/provedores/gerar_configuracoes.py.

## Resultados executados

Linux, ferramentas já instaladas; Cargo offline/locked; sem instalações/downloads.
CARGO_TARGET_DIR=/tmp/lente-integracao-target; RUSTUP_AUTO_INSTALL=0.

| Verificação | Resultado |
|---|---|
| Infraestrutura controlada | 5 aprovados; 2 helpers internos ignorados no runner padrão, executados pelo teste de timeout |
| Seis provedores reais, equivalência CLI/MCP | 6 aprovados: Python, TS, Zig, C, C++, Go |
| Parcial Python, preserva dois nós e diagnósticos | Aprovado |
| Pacote real válido/equivalência e depois adulterado | Aprovado; adulterado recusado antes do cálculo |
| Raio e estrutura reais pela CLI e MCP | Aprovado |
| Total dos testes reais opt-in | 9 aprovados, nenhum ignorado por ferramenta ausente |
| Binário MCP por stdio, Python/ranking | JSON equivalente à CLI; stdout somente protocolo |
| Workspace integral | 394 aprovados, 1 falhou, 45 ignorados até a interrupção pelo Cargo |
| Workspace exceto a falha conhecida, incluindo doctests restantes | 396 aprovados, 45 ignorados, 1 filtrado |
| Clippy dos crates integrados | Sem novos avisos nos módulos adicionados; avisos anteriores em metadata.rs, diff.rs e rotulo_pacote do app |
| Linhagem V5/V6/V7, unidades tocadas | Sem violações em cópia isolada; snapshots/hashes atualizados |
| rustfmt e git diff --check | Conferidos nas unidades tocadas |

A suíte integral **não está verde**. Falha:
`tests::analisar_diff_propaga_falhas_do_cache_e_rejeita_falha_total`, wiring,
asserção `Completa` versus `Parcial` após invalidar cache. É a mesma falha
reproduzida e registrada antes desta integração no laudo
2026-09-26-mapeamento-inventario-01a0e01d-6243.md. Não se atribui causa definitiva
nem se altera o comportamento de cache para mascarar o resultado.

Os 45 ignorados da suíte final incluem testes preexistentes dependentes de
ferramentas/fixtures, os 9 testes opt-in de integração (executados separadamente)
e os 2 helpers de processo. O comando com --skip serve para concluir a cobertura
restante, não para declarar aprovação integral.

Comandos principais:

```sh
export RUSTUP_AUTO_INSTALL=0
export CARGO_TARGET_DIR=/tmp/lente-integracao-target
cargo build --offline --locked -p lente_app -p lente_mcp
cargo test --offline --locked -p lente_infra provedores::tests
cargo test --offline --locked --workspace
cargo test --offline --locked --workspace -- --skip tests::analisar_diff_propaga_falhas_do_cache_e_rejeita_falha_total
python3 03_infra/fixtures/provedores/gerar_configuracoes.py /tmp/lente-fixtures-novas
export LENTE_TEST_CONFIG_DIR=/tmp/lente-fixtures-novas
export LENTE_TEST_CLI=/tmp/lente-integracao-target/debug/lente
cargo test --offline --locked -p lente_mcp real_ -- --ignored --nocapture
cargo clippy --offline --locked -p lente_infra -p lente_wiring -p lente_app -p lente_mcp --all-targets --no-deps
```

## Limites declarados

A matriz verifica integração, não completude semântica. C permanece uma
observação TU/configuração selecionada; C++ semântica parcial; Go pacote/import;
Zig candidato sintático; Python/TS imports estáticos. Exclusões, capacidades e
diagnósticos não são apagados. Estado complete não se converte em semântica
completa; fatos desconhecidos ficam null no relatório do verificador.

Sem cache externo, diff multilinguagem, comparação multilinguagem ou HTML novo.
Pacote exige ferramenta opcional confiável e filesystem estável; hashes não
atestam autoria. Processos que escapem deliberadamente do grupo, Windows,
limites de tamanho de captura e projetos grandes não foram validados. A limpeza
recolhe o líder; Linux pode manter descendente zombie até o reaper, sem execução.

O atualizador automático de hashes do linter não reescreve o cabeçalho Python;
os hashes recíprocos do gerador foram calculados segundo a mesma regra e
verificados com V5/V6/V7. Nenhum contrato de outras tarefas foi reescrito pelo
linter global: atualização ocorreu em cópia isolada das unidades desta entrega,
com verificação de ausência de mudanças concorrentes antes de copiar de volta.
