# Adaptador Zig experimental — grafo sintático

Implementação independente, inteiramente em `lab/adaptador-zig/`.
**Granularidade: arquivo/módulo, não função. Arestas representam candidatos
sintáticos; nenhuma dependência é declarada certamente ativa.** O adaptador não
faz resolução semântica completa e não altera os tipos ou o workspace principal.

## Versão e escolha de parser

Versão suportada e verificada no ambiente: **Zig 0.15.2**. O preparo recusa
outras versões e o helper possui uma verificação de versão em compile-time.
Python >=3.10, somente biblioteca padrão, faz a orquestração.

Antes da implementação foram consultados a documentação oficial versionada e
os fontes da stdlib instalada. APIs confirmadas e usadas:

- `std.zig.Ast.parse(..., .zig)` e diagnósticos sintáticos;
- `builtinCallParams`, `nodeTag`, `nodeMainToken`, `firstToken`, `lastToken`;
- `std.zig.string_literal.parseAlloc` para strings com escapes;
- tokens AST específicos para strings multiline.

Estas APIs são fixadas a 0.15.2; não se promete estabilidade entre versões.
Não se pressupõe uma interface estável de exportação de grafo pelo compilador.
A solução compila um pequeno helper próprio que usa o parser da stdlib.
Não há regex como mecanismo de parsing ou resolução.

Referências oficiais:

- [Referência Zig 0.15.2](https://ziglang.org/documentation/0.15.2/).
- [Semântica de @import](https://ziglang.org/documentation/0.15.2/#import).
- [Fronteira @cImport](https://ziglang.org/documentation/0.15.2/#cImport).
- [Ast.zig na versão 0.15.2](https://github.com/ziglang/zig/blob/0.15.2/lib/std/zig/Ast.zig).
- [Biblioteca padrão versionada](https://ziglang.org/documentation/0.15.2/std/).

## Preparação explícita

Com Zig 0.15.2 já instalado e acessível no PATH, a partir desta pasta:

```sh
zig version
python3 preparar.py
```

O comando compila **somente `parser.zig` deste adaptador**, produzindo
`bin/parser`. Não instala Zig, não baixa dependências e não executa `build.zig`.
Os diretórios de cache necessários ao compilador durante esse preparo ficam
em uma pasta temporária local e são removidos ao final. Isso não é cache de
extração: cada análise parseia novamente os arquivos. O binário do helper
precisa ser recompilado explicitamente quando seu código mudar.

A análise executa somente esse helper conhecido; nunca compila/executa o código
analisado, geradores, build scripts ou C. `preparar.py` e o executável do helper
foram testados em Linux x86_64; outras plataformas não foram validadas.

## Uso

```sh
python3 extrair.py \
  --root fixtures/basico/root.zig \
  --modules fixtures/basico/modules.json \
  --out saida/minha-analise \
  --name exemplo
```

O diretório de saída não pode conter resultados anteriores. Para repetir, use
outro diretório. O programa recusa sobrescrita. Produz:

- `grafo.json`: `{crate, nodes, edges}`, aceito pela entrada JSON atual da Lente.
- `metadados.json`: versão, configuração, capacidades, limites, completude,
  ocorrências de imports, diagnósticos e hash SHA-256 dos bytes do grafo.

O nome do grafo recebe o prefixo `zig_syntactic_`, tornando sua natureza visível
também nos paths. Cada segmento de path é codificado para impedir colisões com
`::`. IDs são ordenados por caminho canônico, determinísticos na mesma árvore;
não são identificadores persistentes entre revisões. Posições de fonte são
absolutas, com linhas/colunas 1-based; colunas contam bytes UTF-8.

A fixture básica gera **6 nós e 6 arestas**, com completude sintática parcial.
Possui imports relativos/nomeados/repetidos, std/builtin, ausente/gerado,
ramos condicionais, comptime e C. Seu `build.zig` e um arquivo importado contêm
`@compileError`, demonstrando que são fontes parseadas, não código executado.

## Raiz e mapa de módulos explícitos

`--root` identifica o arquivo raiz. `--modules` é obrigatório; use um arquivo
com `{}` quando não houver módulos nomeados. Exemplo:

```json
{
  "util": "modulos/util.zig",
  "options": { "path": "generated/options.zig", "generated": true }
}
```

Caminhos do mapa são relativos **ao arquivo JSON do mapa**; caminhos absolutos
também são aceitos no mapa. `generated: true` permite distinguir um módulo
gerado ausente de uma fonte comum ausente. Não há inferência automática de
que um arquivo deveria ter sido gerado. Módulos do mapa só são percorridos
quando alcançados a partir da raiz. Nomes std/builtin/root são reservados.

Política de resolução do experimento:

1. `std` e `builtin`: registrar como externos e excluir nós/arestas.
2. `root`: referenciar a raiz explicitamente fornecida.
3. Nome presente no mapa: usar exatamente o caminho fornecido.
4. Literal relativo terminado em `.zig`: resolver a partir do arquivo importador.
5. `.zon`: registrar fronteira de dados; não parsear como fonte Zig.
6. Outros nomes, caminhos absolutos em imports ou módulos não mapeados:
   diagnóstico, sem inventar destino. Use o mapa para destinos explícitos.

Arquivos existentes são canonicalizados, incluindo symlinks. Referências
repetidas e ciclos não causam percurso infinito. Arquivos externos à pasta da
raiz podem entrar pelo mapa ou por imports relativos. O adaptador **não valida
os limites de pacote que o compilador pode impor**.

Imports configurados pelo build precisam ser fornecidos explicitamente. O mapa
é global: não reproduz tabelas de dependências diferentes por módulo, target,
optimize, features ou múltiplas instanciações de um mesmo arquivo. Não se lê
nem executa `build.zig` automaticamente; escolhê-lo explicitamente como raiz
somente o parseia. Dependências de `build.zig.zon` não são descobertas.

## Incerteza e completude

Todos os imports têm `activation: "unknown"`, inclusive declarações no topo:
Zig pode analisar declarações sob demanda. Imports literais resolvidos são
candidatos e aparecem como arestas mesmo em ramos que poderiam estar inativos.
Nenhum `if`, `switch`, loop, teste, função ou bloco comptime é executado.

O helper procura nós AST de `if`, `switch`, `while`, `for`, `comptime`, funções,
testes, `and`, `or`, `orelse` e `catch` cujos spans contêm a ocorrência. Esses
contextos recebem diagnóstico `activation-uncertain`. Trata-se de uma
sobreaproximação sintática: até imports em uma condição podem receber o mesmo
contexto. Não é uma árvore de controle nem prova de ativação. Expressões
calculadas como `@import(nome)` ou `@import("a" ++ ".zig")` não são avaliadas
nem produzem arestas inferidas. Parênteses ao redor do literal também são
tratados conservadoramente como expressão não literal nesta versão.

Metadados declaram:

```json
{
  "edge_semantics": "syntactic-candidate-not-proven-active",
  "completeness": {
    "syntax": "partial",
    "semantics": "not-evaluated"
  }
}
```

`syntax` pode ser `complete-within-declared-scope`, `partial` ou `error`.
Exclusões std/builtin estão no escopo declarado; outras incertezas, fronteiras
não analisadas ou falhas tornam o resultado parcial. `semantics` permanece
**sempre** `not-evaluated`. Completude sintática não significa completude
semântica, nem que as relações estejam ativas para uma configuração.

Erro de parsing/leitura da raiz não publica grafo e grava metadados de erro.
Erro de dependência preserva os arquivos analisáveis, retira arestas para o
destino não analisado e registra diagnóstico. Erros de argumentos/mapa podem
ocorrer antes de qualquer metadado. CLI retorna 1 para erro fatal; resultados
parciais retornam 0, exigindo leitura de `completeness` pelo consumidor.

## Construções reconhecidas

| Construção | Resultado |
|---|---|
| `@import("arquivo.zig")` | Import relativo candidato, se arquivo existir e for parseável |
| `@import("nome")` presente no mapa | Import nomeado candidato |
| Literal comum com escapes Zig | Decodificado pela stdlib, inclusive `\xNN`/Unicode |
| String literal multiline | Conteúdo obtido dos tokens AST; caracteres preservados |
| Import repetido | Uma aresta por par; ocorrências individuais preservadas |
| Import sob if/switch/loop/comptime/função/teste/short-circuit | Candidato com ativação desconhecida e contexto sintático |
| `@import("root")` | Candidato para a raiz explícita, inclusive ciclos |
| `@import("std")`, `@import("builtin")` | Diagnóstico de exclusão externa; sem travessia da stdlib |
| Nome não mapeado | `unmapped-module`; sem nó sintético |
| Arquivo ausente | `missing-source`; sem nó sintético |
| Módulo marcado generated e ausente | `generated-module-missing`; nunca executa gerador |
| Argumento não literal | `nonliteral-import`; sem avaliação comptime |
| `@cImport(...)` | `c-import-external`; bloco Zig registrado, sem C, headers ou clang |
| `@embedFile(...)`, import de `.zon` | Fronteira de dados; sem aresta de módulo |
| Comentários/strings contendo texto `@import(...)` | Ignorados pelo parser AST |

Chamadas Zig dentro de um bloco `@cImport` continuam sendo apenas sintaxe Zig;
isso não implica que o bloco seja avaliado nem que o C resultante seja conhecido.
A análise não faz typecheck, coleta de símbolos, chamadas de função ou execução
de comptime. O parser processa no máximo 16 MiB por arquivo. A identificação de
contextos compara ocorrências com nós da AST e pode ser custosa em arquivos
muito grandes; não há promessa de desempenho para projetos grandes.

## Compatibilidade com a Lente

Cada arquivo é um nó `kind: "mod"`; `visibility: "priv"` preenche o contrato
sem inferir exports. Arestas são `relation: "uses"`, `uses_kind: "import"`,
do importador para o destino. Não há nós de função, chamadas nem `owns`
artificiais. `from`/`to` coincidem exatamente com o path do nó indicado pelo ID.

O formato atual não contém um campo de certeza por aresta. **O JSON deve ser
interpretado junto com os metadados. A Lente atual não carrega automaticamente
o arquivo separado**, portanto os raios calculados são sobreaproximações
sintáticas, não confirmação de dependências ativas. Exemplo com binário Lente
já existente:

```sh
lente --grafo saida/minha-analise/grafo.json \
  --alvo zig_syntactic_exemplo::local.zig
```

Não foi feita integração CLI/MCP. Não presuma que o modo Rust `--diff` tenha
cobertura Zig. `--so-referencia` remove essas arestas porque são imports.

## Testes reproduzíveis e resultados

Após preparar o helper:

```sh
python3 testes.py
python3 verificar_compat.py
zig fmt --check parser.zig
```

O teste de compatibilidade usa um workspace Cargo próprio em `compat/`, com
dependências por caminho para bibliotecas existentes. Exige Rust >=1.91 e as
dependências Rust disponíveis localmente; usa `cargo --offline`. Se precisar,
prepare explicitamente o ambiente, separadamente da análise:

```sh
cargo fetch --manifest-path compat/Cargo.toml
```

Resultados em 2026-09-26, Zig 0.15.2, Linux x86_64:

- **14 testes aprovados**: relativo/nomeado/endpoints, std/builtin, repetição,
  ausente/gerado, condição/comptime, C, comentários/strings, escapes/multiline e
  não literais, determinismo/hash/sem cache, root/ciclo, erros de parsing,
  embedFile/ZON, CLI de erro/sobrescrita, versão do helper e nomes reservados.
- **Harness real aprovado**: `lente_infra::desserializar_grafo` aceitou o JSON;
  três nós `mod`, três arestas `uses/import`, impacto transitivo de profundidade
  2 e um ciclo calculados pelos algoritmos atuais, sem modificar seus tipos.
- A fixture básica foi executada e gerou seis nós/seis arestas com os
  diagnósticos esperados e ativação não avaliada.

Nenhum arquivo do workspace principal ou de outros adaptadores foi alterado
para este trabalho. Binários, temporários e resultados ficam ignorados por Git
apenas nesta pasta. O contrato local está em `CONTRATO.md`.
