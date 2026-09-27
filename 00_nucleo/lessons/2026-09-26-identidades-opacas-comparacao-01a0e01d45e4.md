# Identidade explícita para comparação estrutural — 2026-09-26

Contrato: `00_nucleo/prompts/comparacao.md`.
Implementação: `01_core/comparacao/src/lib.rs`.

## Mudança

API aditiva `comparar_estruturas_por_identidade`, com dois mapas em memória
`BTreeMap<Path, String>`. Paths endereçam módulos no próprio lado; strings opacas
pareiam os lados sem sintaxe Rust, normalização ou IDs locais de extração.
`ComparacaoIdentidades` separa pares, resíduos, deltas de dependências,
diagnósticos tipados e resumos de ciclos. Cada endpoint de uma aresta conserva
os paths originais dos dois lados. Pesos opcionais distinguem ausência de zero.

Só grupos 1:1 são pareados. Ausências, duplicidades unilaterais/bilaterais e
entradas excedentes são diagnosticadas. Todos os candidatos ambíguos permanecem
sem par. Arestas incidentes a resíduos não entram nos deltas. Não há heurística
nem alegação de equivalência comportamental.

## Verificação

- `cargo test --offline -p lente_comparacao --target-dir /tmp/lente-identidades-explicitas-target`:
  23 testes unitários passaram (17 anteriores e seis novos), mais um doctest.
- Casos novos cobrem mudança de path, identidades distintas em paths iguais ou
  semelhantes, separadores e Unicode (incluindo formas Unicode distintas),
  chave vazia, duplicidade em qualquer lado ou ambos, ausência, entrada excedente,
  preservação dos endpoints, pesos zero/alterados/adicionados/removidos e
  permutação da entrada com ambiguidades e ausências.
- Exemplo público em memória compilado e executado pelo rustdoc.
- `cargo tree --offline -p lente_comparacao --edges normal`: somente crates locais
  L1, sem dependências de terceiros.
- Conteúdo anterior de produção e testes comparado com cópia inicial da tarefa:
  preservado, exceto atualização de linhagem e comentário aditivo sobre a API.
- Snapshot e hashes atualizados com crystalline-lint em cópia temporária contendo
  somente o contrato e implementação autorizados; zero stale/drift remanescentes.
- `git diff --check` nos arquivos autorizados passou.

## Limites e integração

O chamador deve fornecer identidade adequada entre versões e entradas com
escopo/granularidade comparáveis. A API não verifica capacidades de extração.
Segue o contrato de EstruturaModulos: paths únicos e um peso agregado por par
dirigido. Strings vazias são chaves válidas; falta de chave significa ausência
no mapa. A API não promete persistência de IDs de extração.

Não houve alteração de tipos compartilhados, formatos serializados, adaptadores,
CLI, MCP ou harness. Adoção da API pertence a outra tarefa. APIs legadas mantêm
assinaturas, comportamento e testes, inclusive a colisão já documentada no modo
Normalizada. Alterações locais anteriores foram preservadas. A suíte executada
foi a do crate afetado; não foi repetida a suíte completa do workspace.
