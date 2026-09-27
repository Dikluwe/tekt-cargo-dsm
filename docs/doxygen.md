# Enriquecimento opcional Doxygen

A CLI/MCP coordena ferramentas documentais externas por processos explícitos.
Não substitui provedores C/C++, não executa Doxygen numa análise e não transforma
relações documentais em Uses/chamadas. L1 e seus algoritmos permanecem inalterados.
Decisão: [ADR-0009](../00_nucleo/adr/0009-enriquecimento-doxygen.md).

## Preparação e exemplo completo

Requisitos: Linux/Unix, Rust >=1.91, Python 3 e dependências já disponíveis.
Não há instalação, compilação ou download automático durante uma consulta.
Na raiz `/repos/Antigravity/tekt-cargo-dsm`:

```sh
cargo build --offline --locked -p lente_app -p lente_mcp
cargo build --offline --locked --manifest-path lab/doxygen-importador/Cargo.toml
cargo build --offline --locked --manifest-path lab/doxygen-correspondencia/Cargo.toml
cargo build --offline --locked --manifest-path lab/doxygen-prioridade/Cargo.toml
cargo build --offline --locked --manifest-path lab/doxygen-explorador/Cargo.toml
cargo build --offline --locked --manifest-path lab/dsm-explicavel/Cargo.toml
python3 lab/doxygen-integracao/test_integracao.py --output-dir /tmp/lente-doxygen-exemplo
```

Escolha um diretório novo a cada execução. O harness falha se faltar binário;
executa os quatro programas, projeta catálogo/impacto sobre o grafo real recebido,
confere hashes e equivalência CLI/MCP, testa falhas e repete para determinismo.
A fixture é sintética e controlada. `catalog-evidence.json` contém **evidência
auxiliar explicitamente fornecida pelo autor**: não é saída de um provedor C++.
A projeção padrão mantém os nomes opacos como null; o harness comprova isso antes
de fornecer o catálogo auxiliar da fixture. Não se enfraquece o matching.

Para gerar XML real como etapa explícita de teste (nunca de análise):

```sh
doxygen --version
python3 lab/doxygen-integracao/test_integracao.py --real-doxygen \
  --output-dir /tmp/lente-doxygen-real
```

Só Doxygen **1.9.8** é aceito. O teste usa `fixtures/real.hpp` e Doxyfile local,
sem configuração do projeto-alvo. Registra hash da configuração em `results.json`;
o importador não infere esse hash, mantendo seu campo null. A fixture real usa
`XML_NS_MEMB_FILE_SCOPE=NO` para evitar definições de membros repetidas entre
namespace/file. Outros XML podem repetir definições, situação recusada com erro.

## Operações pela CLI e MCP

A integração amplia **`analisar_artefatos`**, sem nova ferramenta MCP. CLI:

```sh
./target/debug/lente --analise-artefatos /caminho/absoluto/config.json
```

Configuração JSON estrita, caminhos internos absolutos:

```json
{
  "executavel": "/repos/Antigravity/tekt-cargo-dsm/lab/doxygen-importador/target/debug/lente-doxygen-importador",
  "timeout_ms": 30000,
  "consulta": {
    "modo": "doxygen-importar",
    "xml_dir": "/repos/Antigravity/tekt-cargo-dsm/lab/doxygen-integracao/fixtures/xml"
  }
}
```

| modo | Campos da consulta | Executável |
|---|---|---|
| doxygen-importar | xml_dir | lente-doxygen-importador |
| doxygen-associar | inventory, catalog | lente-doxygen-correspondencia |
| doxygen-prioridade | inventory, catalog, matches, impact | lente-doxygen-prioridade |
| doxygen-explorar | inventory, catalog, matches | doxygen-explorador |
| doxygen-grafo | grafo; opcionais politica_nomes, coverage, alvos_metricas | **omitir executavel** |

Os quatro primeiros executáveis estão em `lab/doxygen-<componente>/target/debug/`.
O explorador é chamado com saída finita JSON: **não abre porta pelo MCP**.
Cada chamada de processo recebe timeout (1..3600000 ms), diretório privado e
grupo encerrado ao terminar, argumentos tipados sem shell. Executáveis são
confiáveis e explicitamente preparados, não uma sandbox contra programas hostis.

No MCP, o mesmo objeto é `arguments` de `tools/call`:

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"analisar_artefatos","arguments":{"timeout_ms":30000,"consulta":{"modo":"doxygen-grafo","grafo":"/repos/Antigravity/tekt-cargo-dsm/lab/doxygen-integracao/fixtures/graph.json","coverage":"partial"}}}}
```

Resposta comum: `{"analise":{...},"execucao":{"modo":"...","codigo_saida":0,"stderr":""}}`.
`codigo_saida` é o original do helper: importação parcial mantém 2. CLI retorna
0 e MCP `isError=false` para esse resultado válido. Falhas: CLI 3/MCP true;
`provedor_indisponivel`, `timeout`, `saida_invalida`, `falha_execucao` distinguem
transporte. Falhas documentais conhecidas conservam contexto `schema`/`digest`
em stderr/status, sem apagar a causa. Argumentos inválidos do parser CLI são 2.

Para encadear chamadas, salve **somente `analise`** como arquivo novo. Os bytes
salvos definem os hashes a usar nas próximas etapas: não reutilize digests de
outra serialização, inclusive se só mudar whitespace. Exemplo de extração de
uma resposta previamente salva (arquivo de destino deve ser novo):

```sh
python3 - /tmp/resposta-importacao.json /tmp/inventario-novo.json <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f:
    result = json.load(f)['analise']
with open(sys.argv[2], 'x', encoding='utf-8') as f:
    json.dump(result, f, ensure_ascii=False, indent=2)
    f.write('\n')
PY
```

Ferramentas diretas também aceitam `--output`: escrevem temporário irmão e
publicam por hard link exclusivo. Não sobrescrevem entradas/resultados. Há
atomicidade do nome no filesystem local com hard links; sem fsync do diretório,
não se promete durabilidade após queda de energia. Stdout da CLI principal é
transporte, não publicação atômica em arquivo pelo shell.

## Catálogo e métricas vinculados ao grafo

`doxygen-grafo` lê os bytes uma vez, usa o desserializador da Lente e calcula
`graph_digest` SHA-256 desses bytes, sem resserializar. Retorna
`lente.doxygen.graph_projection` v1 com `catalog` e `impact` v1, que compartilham
esse hash. `node_id` é original; localização vem somente de `position.file` e
`start_line`; assinatura continua null. Nenhuma fonte é aberta.

`politica_nomes=opaque` é padrão: **path não vira nome qualificado**. `rust-path`
é opt-in para grafo Rust cujo provedor obedece à spec de path canônico; não é
uma convenção C/C++. Evidência insuficiente gera diagnóstico e unmatched. Um
catálogo auxiliar explícito continua aceito, sem afirmar que provedores atuais
já o exportam. A DSM verifica digest e conjunto exato de node_ids.

Métricas reutilizam `calcular_raio`: montante de todas as Uses, consumidores
**distintos**, alvo excluído mesmo em ciclos, Owns ignorado. Não há reimplementação
de alcance/ranking. Paths colidentes impedem a precondição: métricas null e
cobertura parcial. `alvos_metricas` pode selecionar IDs únicos existentes;
demais métricas são null, distintas de zero, com diagnóstico. Custo atual: um
raio por alvo; não há cache nem limite de CPU para essa projeção em memória.

`coverage` default unknown é declaração do chamador, nunca resultado do parse.
Mesmo complete descreve só o conjunto de entrada declarado, não a semântica do
programa. Ausência de dependência em grafo parcial não prova ausência no sistema.

## Schemas e migração

- Inventário novo: `lente.doxygen.inventory` **v2**, `relations` resolvidas e
  `pending_relations` obrigatório (`[]` quando vazio).
- Pendência: `{from,to,kind,reason}`; reason é missing_from/missing_to/missing_both,
  conferido contra os símbolos. Pendências exigem coverage partial, não entram
  em matching, impacto ou navegação como relações resolvidas.
- Catálogo, matches e impacto: v1. Relatório `lente.doxygen.priority_report`: **v2**,
  conservando `pending_relations`, relações resolvidas, coberturas e diagnósticos.
- Consumidores aceitam inventário **v1 resolvido**. V1 com endpoints ausentes
  continua erro; não muda de interpretação silenciosamente.

Migração explícita v1→v2: preserve símbolos e diagnósticos; particione relations
pela existência dos endpoints; mova as não resolvidas para pending_relations com
reason; se houver pendências, marque partial; mude version para 2. Grave arquivo
novo e regenere matches/prioridade/painel: os digests antigos deixam de valer.
Não crie símbolos fictícios. IDs duplicados e estruturas malformadas são erros.
Diagnósticos podem citar IDs ausentes: são contexto de perdas, não alvos navegáveis.

## Abrir documentação na DSM

Após o primeiro comando do harness, execute explicitamente:

```sh
python3 lab/dsm-explicavel/serve.py /tmp/lente-doxygen-exemplo/synthetic/bundle.json \
  --binary /repos/Antigravity/tekt-cargo-dsm/lab/dsm-explicavel/target/debug/dsm-explicavel \
  --graph /repos/Antigravity/tekt-cargo-dsm/lab/doxygen-integracao/fixtures/graph.json \
  --inventory /tmp/lente-doxygen-exemplo/synthetic/inventory.json \
  --catalog /tmp/lente-doxygen-exemplo/synthetic/catalog.json \
  --matches /tmp/lente-doxygen-exemplo/synthetic/matches.json \
  --explorer /repos/Antigravity/tekt-cargo-dsm/lab/doxygen-explorador/target/debug/doxygen-explorador \
  --port 8876
```

Abra `http://127.0.0.1:8876`, encerre com Ctrl-C. Sem essas cinco opções extras,
a DSM continua funcionando sem documentação. Com elas, qualquer vínculo inválido
impede abrir o servidor. O preparador DSM real confere a matriz contra o grafo;
o explorador real valida inventário/catálogo/matches congelados. Nenhuma célula,
peso ou alcance recebe relações Doxygen. O painel mostra confirmado, candidatos,
evidências, diagnósticos, cobertura e pendências relevantes por node_id.
Tudo é texto via textContent; não busca URLs nem cria rotas de arquivos.

Limites: SHA-256 comprova integridade, não autoria, coerência semântica ou versão
de fonte. Entrada e executáveis confiáveis; arquivos devem permanecer estáveis
durante a leitura. Assinaturas do importador são argsstring, não assinatura C++
completa. Perfil XML limitado a 1.9.8, sem XSD completo. Nenhuma promessa de escala
ou equivalência entre extração estrutural e documentação. Ver o laudo para os
testes realmente executados e as pendências globais preexistentes.

Verificação executada e lista de arquivos:
[laudo de integração](../00_nucleo/lessons/2026-09-26-integracao-doxygen.md).
