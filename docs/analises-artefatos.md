# Snapshots, regras e DSM pela CLI/MCP

As entregas 22–24 agora têm uma entrada comum, complementar aos
[provedores de linguagens](adaptadores.md). A integração chama executáveis
opcionais por processo, sem importar crates de `lab/` em produção. Não instala
dependências automaticamente. O executor atual exige Unix e `/bin/kill`.

## Compilação

Na raiz do repositório, com as dependências Cargo já disponíveis:

```sh
cargo build --offline --locked -p lente_app -p lente_mcp
cargo build --offline --locked --manifest-path lab/impacto-snapshots/cli/Cargo.toml
cargo build --offline --locked --manifest-path lab/regras-arquiteturais/Cargo.toml
cargo build --offline --locked --manifest-path lab/dsm-explicavel/Cargo.toml
python3 03_infra/fixtures/analises/test_integracao.py
```

Cada workspace experimental gera seu próprio `target/debug/`. A biblioteca
original de snapshots continua pura; somente a nova CLI faz leitura e JSON.

## Entrada comum

Salve uma configuração JSON e execute:

```sh
./target/debug/lente --analise-artefatos /caminho/absoluto/config.json
```

A opção é exclusiva: não combine com `--ranking`, `--provedor`, `--text` ou
outros modos. Todos os caminhos **dentro** da configuração devem ser absolutos.
O arquivo de configuração em si pode ser relativo. Use executáveis confiáveis:
o processo é isolado em diretório/grupo e tem prazo, mas não é uma sandbox de
permissões. `timeout_ms` deve estar entre 1 e 3600000.

```json
{
  "executavel": "/caminho/do/repositorio/lab/impacto-snapshots/cli/target/debug/lente-impacto-snapshots-cli",
  "timeout_ms": 30000,
  "consulta": {
    "modo": "snapshots",
    "entrada": "/caminho/absoluto/snapshots.json"
  }
}
```

No MCP, chame `analisar_artefatos` com esse mesmo objeto como `arguments`
(não com o caminho da configuração). A resposta textual é o mesmo JSON da CLI:

```json
{"analise": {}, "execucao": {"modo": "snapshots", "codigo_saida": 0, "stderr": ""}}
```

`analise` contém o resultado integral da ferramenta, sem apagar diagnósticos ou
promover cobertura parcial. Para regras, a CLI preserva 0=aprovado, 1=violação,
2=inconclusivo. Os três são resultados válidos (`isError=false` no MCP).
Falhas de arquivo, configuração, execução ou protocolo retornam CLI 3/MCP
`isError=true`. Erros de sintaxe dos argumentos da própria CLI seguem o código
2 do parser; não são um relatório inconclusivo (não há JSON de análise).

## 1. Comparação de snapshots

O arquivo de entrada tem `format_version: 1`, `antes`, `depois`,
`correspondencias` e `mudancas`. Cada snapshot contém `versao`, `grafo` inline
(formato `crate/nodes/edges` da lente), `granularidade`, `natureza`,
`completude` (`completa`, `parcial`, `desconhecida`) e `diagnosticos`.
Completude omitida é desconhecida, nunca completa.

Exemplo de correspondências e mudanças, junto dos dois snapshots:

```json
{
  "correspondencias": [
    {"identidade": "servico", "antes": 10, "depois": 80},
    {"identidade": "consumidor", "antes": 20, "depois": 90}
  ],
  "mudancas": [
    {"tipo": "pareada", "identidade": "servico", "alteracao_declarada": true, "renomeacao_declarada": false},
    {"tipo": "remocao", "antes": 30},
    {"tipo": "adicao", "depois": 100}
  ]
}
```

IDs são locais a cada versão; igualdade numérica não declara identidade.
O chamador fornece correspondências e mudanças explicitamente. Uma remoção
consulta o grafo anterior; ausência de aresta em grafo parcial não prova que
o consumidor deixou de depender. Consulte o contrato em
`lab/impacto-snapshots/cli/CONTRATO.md`. Há uma entrada completa de remoção em
`lab/impacto-snapshots/cli/exemplo.json`, utilizável como `consulta.entrada`.

## 2. Regras arquiteturais

Use o executável `lab/regras-arquiteturais/target/debug/regras-arquiteturais`.
Substitua a consulta por:

```json
{
  "modo": "regras",
  "baseline": "/repositorio/lab/regras-arquiteturais/examples/baseline.json",
  "atual": "/repositorio/lab/regras-arquiteturais/examples/current.json",
  "regras": "/repositorio/lab/regras-arquiteturais/examples/rules.json",
  "data": "2026-09-26"
}
```

São os snapshots específicos do avaliador de regras, **não** o JSON inline da
comparação acima. Seus caminhos internos seguem o contrato da ferramenta
(relativos ao arquivo que os declara). Ela valida hashes, contexto, identidades,
cobertura e exceções. A data de avaliação é explícita; não é inferida do relógio.
Os exemplos `allowed-rules.json`, `rules.json` e `partial.json` exercitam os
códigos 0, 1 e 2 respectivamente.

## 3. DSM explicável e consulta de caminho

Use `lab/dsm-explicavel/target/debug/dsm-explicavel` e consulta
`{"modo":"dsm","entrada":"/caminho/entrada-dsm.json"}`.
A entrada original da DSM contém `format_version:1`, `metadata` e `graph`;
ver `lab/dsm-explicavel/README.md`. O resultado `analise` é o pacote preparado
original com matriz, evidências, exclusões, metadados e grafo-fonte.

Para abrir a interface, salve **somente o objeto `analise`**, sem o envelope,
como `bundle.json` e execute o servidor existente:

```sh
python3 lab/dsm-explicavel/serve.py /caminho/bundle.json \
  --binary /caminho/do/repositorio/lab/dsm-explicavel/target/debug/dsm-explicavel \
  --port 8765
```

Abra `http://127.0.0.1:8765`. CLI/MCP não iniciam servidor persistente.
Para consultar um caminho sem interface, use o mesmo executável e:

```json
{"modo":"caminho-dsm","pacote":"/caminho/bundle.json","consumidor":11,"alterado":31}
```

A direção é consumidor → elemento alterado. `no_path` significa que nenhum
caminho foi encontrado no grafo fornecido, não prova ausência no programa real.

## Limites desta integração

Não há ainda captura automática antes/depois de Git, geração automática de
identidades entre versões nem conversão universal dos formatos de snapshot.
A cobertura semântica continua sendo a declarada por cada provedor. A DSM
continua experimental. A integração é de arquivos e processos explícitos;
não significa que os experimentos foram promovidos ao núcleo de produção.
