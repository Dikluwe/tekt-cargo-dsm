> Integração atual: consulte [o guia comum](../../docs/doxygen.md) e a revisão
> ADR-0009 no CONTRATO.md. Inventários novos são v2, com pendências separadas.
> Relatos e contagens abaixo descrevem a entrega isolada anterior; a validação
> integrada está no laudo da rodada.

# Correspondência documental Doxygen (experimental)

Associa símbolos de um catálogo estrutural explícito a um inventário documental,
com decisão conservadora e evidências verificáveis. Não extrai documentação nem
altera grafos, provedores, CLI/MCP de produção ou outros experimentos.

O [CONTRATO.md](CONTRATO.md), registrado antes da implementação, define a política
completa, os erros e o significado dos resíduos. Fontes usam cabeçalhos de linhagem
apontando para ele. Este diretório não existia antes da tarefa.

## Compilar, executar e testar

Da raiz do repositório:

```sh
cargo build --offline --locked --manifest-path lab/doxygen-correspondencia/Cargo.toml
cargo test --offline --locked --manifest-path lab/doxygen-correspondencia/Cargo.toml
python3 lab/doxygen-correspondencia/verificar_cli.py

lab/doxygen-correspondencia/target/debug/lente-doxygen-correspondencia \
  --inventory lab/doxygen-correspondencia/fixtures/inventory.json \
  --catalog lab/doxygen-correspondencia/fixtures/catalog.json \
  --output lab/doxygen-correspondencia/target/resultado.json
```

Escolha um caminho de saída ainda inexistente. O programa recusa sobrescrever
arquivos e não cria diretórios automaticamente. Sucesso retorna 0; erro retorna
1 com mensagem stderr. `--help` mostra a interface e retorna 0. Caminhos relativos
de argumentos são relativos ao cwd; paths dentro dos símbolos são comparados
literalmente, sem leitura de arquivos apontados por eles.

Requer Rust compatível com edition 2024 e Python 3 para o teste da CLI. Os crates
serde/serde_json/sha2 devem já estar disponíveis localmente. Todos os comandos
Cargo acima são offline; não há instalação, download ou chamada a Doxygen,
compilador do projeto analisado ou fontes. Workspace e lockfile são próprios.
Não há dependências Cargo de produção, outros experimentos ou outras tarefas.

## Exemplo entregue

`fixtures/inventory.json` e `fixtures/catalog.json` são independentes e pequenos.
`fixtures/matches.json` foi produzido pelo binário real, não preenchido à mão.
O graph_digest do catálogo é um valor sintético de fixture; não há grafo a verificar.

| Nó | Resultado | Motivo |
|---|---|---|
| 10 | matched → loc | Nome qualificado e localização exatos; assinatura ausente. |
| 20 | matched → sig | Nome e assinatura exatos; localização ausente. |
| 30 | matched → over-int | Sobrecarga int coincide; double diverge e é recusada. |
| 40 | matched → namespace | other::over não é demo::over. |
| 50 | ambiguous | amb-a e amb-b satisfazem a política; nenhum é escolhido. |
| 60 | unmatched | Localização igual, mas assinaturas declaradas divergem. |
| 70 | unmatched | Só nome qualificado; weak fica disponível para inspeção. |
| 80 | unmatched | ./src/demo.cpp difere de src/demo.cpp; assinatura ausente. |
| 90 | unmatched | Nenhum nome qualificado coincide. |
| 100 | unmatched | Nome qualificado não informado. |

São quatro matched, um ambiguous e cinco unmatched. `document_id` só existe como
valor não null nos matched. Todos os demais nós, inclusive ambiguous, aparecem
em unmatched_node_ids; documentos sem associação automática aparecem em
unmatched_document_ids, incluindo candidatos ambíguos.

`evidence` contém fatos comuns aos candidatos publicados. Cada comparação também
publica `candidate_evaluation`: a mensagem é texto JSON com IDs, nomes, assinaturas,
localizações originais, fatos e suficiência. Isso evita que evidências de dois
documentos diferentes sejam combinadas como se provassem um único candidato.
Diagnósticos de entrada e cobertura/produtor permanecem contextualizados na saída.

Conflitos globais são conservadores: um documento suficiente reivindicado por
mais de um nó torna todos os envolvidos ambiguous, mesmo que algum tenha apenas
um candidato. IDs duplicados são erro de entrada, mesmo para registros idênticos.

## Verificação efetivamente executada

- **17 testes Rust passaram**: duas regras de associação, sobrecargas, namespaces,
  múltiplos candidatos, veto de assinatura, nomes insuficientes, paths/assinaturas
  sem normalização, localização diferente sob regra por assinatura, duplicados,
  conflitos globais (inclusive contra nó ambíguo), evidências por candidato,
  reordenação, bytes/digests, opcionais ausentes, schemas/versões/localização/
  endpoints inválidos e assinaturas brancas.
- **5 testes Python da CLI real passaram**: formato e fixture esperada, SHA-256
  conferido independentemente por hashlib (incluindo whitespace/CRLF adicionais),
  proteção de saída/entrada existentes, erros sem relatório, IDs duplicados e
  validação de argumentos. Os arquivos temporários ficam dentro do target local.
- `cargo build --offline --locked` passou. `cargo tree --offline --locked
  --depth 1` confirmou somente serde, serde_json e sha2 como dependências diretas.
- Nenhuma suíte do workspace principal ou ferramenta Doxygen foi executada.

## Limites

Matching não significa equivalência semântica ou comportamental. Nem nome simples,
kind, ID, texto documental ou semelhança textual oferece identidade persistente.
Localização exata é somente um dado declarado; não prova que os arquivos estejam
na mesma revisão. O catálogo é fornecido explicitamente: não afirmamos que os
provedores atuais o gerem. O graph_digest é validado em formato, não confrontado
com um grafo ausente desta interface. SHA-256 vincula arquivos lidos, não autentica
produtores. Execução bem-sucedida não promove coverage a complete.

Assinaturas e nomes são opacos. Não fazemos parsing C++, aliases, resolução de
symlinks, canonicalização de path, normalização Unicode ou remoção de espaços.
Duas assinaturas declaradas diferentes sempre vetam associação, inclusive uma
vazia. Podem existir falsos negativos; são preferíveis a associações inventadas.
A política 1:1 pode recusar documentação legitimamente compartilhada por nós.
Não resolve atribuições globais nem ambiguidades por heurística.

Os arquivos são lidos em memória uma vez cada, sem snapshot transacional conjunto
entre eles. Não há limite de tamanho dedicado. Uma falha de escrita do filesystem
pode deixar saída incompleta; o processo reporta erro e não a apresenta como sucesso.
Consumidores devem aceitar somente uma execução bem-sucedida e JSON completo.

## Arquivos desta entrega

- `CONTRATO.md`, `README.md`: contrato, decisões e instruções.
- `Cargo.toml`, `Cargo.lock`, `.gitignore`: workspace isolado e dependências fixadas.
- `src/lib.rs`: modelos JSON, validação, matching, hashes e testes inline.
- `src/main.rs`: interface de arquivos, argumentos e criação exclusiva de saída.
- `fixtures/inventory.json`, `fixtures/catalog.json`, `fixtures/matches.json`.
- `verificar_cli.py`: testes locais da interface executável.

Não foi realizada integração em produção. A comunicação com as outras tarefas
fica restrita aos três schemas JSON v1 do contrato comum.
