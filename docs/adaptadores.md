# Provedores opcionais na CLI e no MCP

A seleção é explícita. O uso sem `--provedor`, `--provedor rust`, `--pacote`,
`--grafo`, diff e comparação conserva o fluxo histórico. Os seis adaptadores da
Arena são **ferramentas externas experimentais**, não dependências Cargo de
produção. A integração não torna suas análises semanticamente equivalentes.

## Preparação explícita

Linux/Unix, Rust >=1.91 para compilar a Lente e ferramentas Rust. A integração
usa `/bin/kill` para encerrar grupos de processos. Os comandos abaixo são
preparação manual, nunca executada automaticamente numa análise:

```sh
cargo build --offline --locked -p lente_app -p lente_mcp
cargo build --offline --locked --manifest-path lab/pacote-analise/Cargo.toml
cargo build --offline --locked --manifest-path lab/adaptador-c/Cargo.toml
cargo build --offline --locked --manifest-path lab/adaptador-go/Cargo.toml
```

`--offline` exige dependências já disponíveis. O usuário precisa preparar o
ambiente se faltarem crates; a análise não chama Cargo ou gestores de pacotes.

| Ferramenta | Preparação e dependência |
|---|---|
| Python | Python >=3.10; helper `lab/adaptador-python/adaptador.py`, somente stdlib |
| TypeScript/JavaScript | Node >=20; `cd lab/adaptador-typescript && npm ci --ignore-scripts`; TypeScript 5.9.3 fixado no lockfile do helper |
| Zig | Zig 0.15.2 e Python >=3.10; `cd lab/adaptador-zig && python3 preparar.py`; prepara apenas `bin/parser` próprio |
| C | libclang 18 disponível ao linker/carregador, depois build acima; não chama o compilador registrado na base |
| C++ | Python e libclang previamente instalado; caminho da biblioteca em `libclang` |
| Go | Go compatível com go.mod/go.work, dependências previamente presentes; helper fixa `GOTOOLCHAIN=local` e bloqueia busca remota |
| Pacote verificável | Binário `lente-pacote-analise` acima, implementa create/verify; Linux/filesystem local com renameat2 |

Consulte também os READMEs de cada `lab/adaptador-*` e de `lab/pacote-analise`.
Helpers e executáveis são código confiável instalado/configurado pelo usuário;
a infraestrutura não é uma sandbox para executáveis hostis.

## Configuração e CLI

`--config-provedor` aponta para JSON estrito. Campos desconhecidos são erros.
Caminhos **dentro** do JSON devem ser absolutos; não dependem do cwd da CLI/MCP
nem da pasta do JSON. O caminho da própria configuração pode ser relativo.

Exemplo Python (`/repo` representa o caminho real deste checkout):

```json
{
  "executavel": "/usr/bin/python3",
  "helper": "/repo/lab/adaptador-python/adaptador.py",
  "verificador": "/repo/lab/pacote-analise/target/debug/lente-pacote-analise",
  "timeout_ms": 30000,
  "parametros": {"roots": ["/projeto/src"]}
}
```

```sh
lente --provedor python --config-provedor python.json --ranking --top 10
lente --provedor python --config-provedor python.json --alvo python::pacote::util
lente --provedor python --config-provedor python.json --alvo-id 2
lente --provedor python --config-provedor python.json --estrutura
```

A análise é JSON, sempre com as chaves `analise` e `extracao`. A primeira reutiliza
os formatadores existentes. `extracao.verificacao.summary` contém os fatos
interpretados **pelo verificador**, com ponteiros de origem, inclusive provedor,
versão, granularidade, configuração, execução, cobertura de escopo e cobertura
semântica. `null` continua desconhecido. `metadados_originais` conserva todos os
campos, incluindo capacidades, ocorrências, posições, limites e diagnósticos.
C também conserva o manifesto de variantes em `manifesto_legado`.

Não se executa a resolução de colisões Rust: paths duplicados são recusados.
`--filtrar-stdlib`, `--so-referencia`, fontes legadas, diff, comparação e HTML/texto
são rejeitados neste fluxo antes da extração. `--completo` é redundante: o escopo
analítico é o grafo integral fornecido pelo adaptador, já sujeito à política de
externos dele. `--verbose` também é redundante: o JSON inclui detalhes completos.

Parâmetros específicos (`parametros`):

| Provedor | Obrigatórios | Opcionais |
|---|---|---|
| python | `roots`: lista não vazia de diretórios de busca | — |
| typescript | `project`: tsconfig.json, `name`: nome do projeto | — |
| zig | `root`: arquivo raiz, `modules`: JSON do mapa explícito, `name` | — |
| c | `database`: arquivo chamado compile_commands.json, `root`, `variante`: índice inteiro não negativo | — |
| cpp | `database`, `root`, `libclang` | `configuration`: hash SHA-256 escolhido; se omitido, o helper só aceita uma configuração |
| go | `project`, `goos`, `goarch`, `tests`: include/exclude, `work`: auto/off/caminho absoluto | `tags`: lista CSV, `patterns`: lista, `module_cache`: diretório existente |
| pacote-analise | `diretorio`: pacote existente | `manifest_sha256`: âncora obtida por canal independente |

`executavel` é o interpretador para Python/C++/Zig/TypeScript, ou o binário para
C/Go. `helper` é obrigatório nos quatro primeiros e proibido nos demais.
Para `pacote-analise`, `executavel` e `helper` ficam ausentes ou null; somente
`verificador`, `timeout_ms` e parâmetros são necessários.

A escolha da variante C é obrigatória mesmo com uma única observação. O índice
é local à base: consultar o manifesto do adaptador identifica TU/configuração;
inserir/remover comandos pode mudar índices. A Lente analisa **somente essa
observação**, preservando o manifesto inteiro, sem união silenciosa. A versão
atual do helper ainda processa as demais observações antes da seleção; o timeout
inclui esse custo. A validação semântica de bases, tsconfig, mapa Zig e opções de
linguagem continua no próprio helper, sem reimplementar seus resolvedores.

## Pacote, privacidade e resultado parcial

Cada extração nova passa por `lente-pacote-analise create` e `verify`; a Lente não
mantém uma segunda implementação de manifesto, hashes ou normalização de
metadados. Graph e metadata legados ficam preservados no pacote temporário, e
só o grafo verificado entra nos cálculos. O pacote temporário é removido ao final.
Para consumir pacote já publicado use `--provedor pacote-analise`.

Por padrão exige-se vínculo por digest. C e Go ainda não declaram graph_sha256:
para usá-los, configure `permitir_metadados_sem_vinculo: true`. O relatório mantém
`source_binding: unknown`, nunca afirma que existe vínculo provado. Não adicionamos
um hash aos metadados originais para disfarçar essa ausência.

Metadados com ambiente exigem `incluir_ambiente: true` na criação, como no Go e
quando presente no C. É um opt-in explícito para conservar/expor esses valores;
não há redação silenciosa. Revise metadados antes de compartilhar. O consumo de
pacote existente conserva a decisão registrada pelo produtor, sem reescrevê-lo.

Resultados parciais são sucesso de transporte (CLI exit 0, MCP isError=false),
com seus estados e motivos integrais, mesmo quando o helper retorna código de
parcialidade. Estado de execução failed/error é falha, não resultado completo.
O campo `codigo_saida_adaptador` preserva o código original. Códigos aceitos:
Python/C++ 0 ou 1 (parcial), C/Go 0 ou 2, TS/Zig 0. Qualquer código precisa de
arquivos válidos novos, não de saídas anteriores.

Falhas têm prefixos distintos: `parametros`, `provedor_indisponivel`,
`falha_execucao`, `timeout`, `saida_invalida`. Na CLI, erro de parâmetros retorna
2, os demais 1; no MCP são isError=true. A ausência de dependência carregada
internamente pelo helper pode aparecer como falha_execucao com seu stderr;
a infraestrutura não interpreta arbitrariamente mensagens como sucesso parcial.

## MCP

As três ferramentas anteriores permanecem e aceitam `provedor: "rust"` opcional.
A ferramenta aditiva `analisar_provedor` recebe a configuração como objeto:

```json
{
  "jsonrpc": "2.0", "id": 1, "method": "tools/call",
  "params": {
    "name": "analisar_provedor",
    "arguments": {
      "provedor": "python",
      "configuracao": {
        "executavel": "/usr/bin/python3",
        "helper": "/repo/lab/adaptador-python/adaptador.py",
        "verificador": "/repo/lab/pacote-analise/target/debug/lente-pacote-analise",
        "timeout_ms": 30000,
        "parametros": {"roots": ["/projeto/src"]}
      },
      "consulta": {"modo": "ranking", "top": 10}
    }
  }
}
```

Consultas adicionais: `{"modo":"raio","alvo":"python::pacote::util"}`,
`{"modo":"raio-id","alvo_id":2}`, `{"modo":"estrutura"}`. Chaves alheias ao
modo são rejeitadas antes da extração. O texto JSON retornado é o mesmo envelope
da CLI, não uma versão resumida que apague cobertura.

## Processo e concorrência

Invocação com `Command` e vetor de argumentos, nunca shell. `timeout_ms` (1 a
3.600.000) cobre extração, create e verify juntos, com relógio monotônico. Em Unix
cada comando recebe grupo próprio; fim normal, falha e timeout encerram o grupo
e recolhem o líder. Teste verifica também um descendente. Ferramentas que chamem
setsid ou escapem deliberadamente do grupo estão fora dessa garantia.

Cada chamada usa diretório novo modo 0700, criado exclusivamente, com captura
stdout/stderr em arquivos (sem bloqueio de pipes) e limpeza por RAII. Não há
cache de extração. O Go pode usar caches internos e gravar dados auxiliares como
previsto em seu contrato. Não há limite de bytes capturados nesta versão.

Pacotes existentes e fontes devem permanecer estáveis durante a chamada, conforme
o contrato do verificador; a integração não é um snapshot transacional do
filesystem. Hashes verificam integridade, não autenticam o produtor. A âncora
manifest_sha256 é opcional e precisa vir de canal confiável.

## Testes reproduzíveis

Com as ferramentas já preparadas, da raiz:

```sh
cargo test --offline --locked --workspace
python3 03_infra/fixtures/provedores/gerar_configuracoes.py /tmp/lente-fixtures-novas
export LENTE_TEST_CONFIG_DIR=/tmp/lente-fixtures-novas
export LENTE_TEST_CLI="$(pwd)/target/debug/lente"
cargo test --offline --locked -p lente_mcp real_ -- --ignored --nocapture
```

O gerador só escreve configurações e bases absolutas; não executa ferramentas.
A saída deve ser nova. Ajuste libclang/GOOS/GOARCH conforme seu ambiente. Se faltam
ferramentas, execute individualmente os testes `real_python`, `real_typescript`,
`real_zig`, `real_c`, `real_cpp`, `real_go`; registre os demais como **não
executados**, jamais aprovados. `real_parcial` requer Python/verificador e
`real_raio_estrutura` compara também as consultas de raio e estrutura;
`real_pacote` usa o verificador real para testar pacote válido e adulterado.

Os testes reais são ignorados na suíte padrão por exigirem ferramentas opcionais;
esta entrega os executou explicitamente. Dois outros testes ignorados na infra
são auxiliares internos do teste de timeout: não são provedores ausentes.

## Matriz efetivamente verificada (Linux, 2026-09-26)

| Provedor | Relação / granularidade | Fixture integrada | CLI=MCP | Cobertura declarada observada |
|---|---|---|---|---|
| Rust/JSON legado | Uses / itens Rust | Suíte existente | Interfaces anteriores preservadas | Consultar resultado da suíte no laudo |
| Python | Imports sintáticos / módulo | tests/fixtures/projeto e erro sintático local | Passou | Execução complete ou partial; semântica partial |
| TypeScript/JS | Imports/reexports estáticos / arquivo | exemplo/tsconfig.json | Passou | Complete dentro do escopo; semântica desconhecida |
| Zig | Candidatos sintáticos / arquivo | fixtures/basico | Passou | Sintaxe partial; semântica not-evaluated |
| C | Includes diretos observados / arquivo por TU/configuração | main.c com variantes e TU missing.c no manifesto | Passou | Estado da TU separado do manifesto parcial |
| C++ | Referências observadas libclang / arquivo | tests/fixtures | Passou | Execução complete; semântica partial |
| Go | Imports diretos / pacote | fixtures/chain | Passou | Loading complete; semântica not-claimed |
| Pacote de análise | Transporte verificável, não extrator | fixtures/graph.json + metadata.json; adulteração posterior | Passou | Metadados originais; adulterado recusado |

Sem ferramentas ausentes entre os seis casos executados neste ambiente. Essa
matriz verifica integração e preservação de contratos, não completude semântica
dos extratores. Windows, instalações diferentes e projetos grandes não foram
validados.

## Enriquecimento documental

[Doxygen opcional](doxygen.md) consome XML existente e associa documentação a um
catálogo explícito, preservando ambiguidades e pendências. Não é um substituto
para provedores C/C++, nem transforma referências documentais em Uses. Produção
continua sem dependências Cargo de lab; configuração de executáveis é explícita.
