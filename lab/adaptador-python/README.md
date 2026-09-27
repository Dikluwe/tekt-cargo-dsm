# Adaptador Python experimental — dependências entre módulos

Experimento isolado da Arena, implementado apenas com a biblioteca padrão do
Python (3.10+). Lê arquivos com `tokenize.open`, analisa com `ast.parse` e visita
`import` e `from ... import`. **Não importa, compila para execução nem executa o
projeto analisado.** Não instala dependências, consulta `sys.path`, chama Python
no projeto ou usa `importlib` para resolver seus imports. Não habilita cache.

Este documento é o contrato e a origem de linhagem dos arquivos do experimento.
Fixtures são dados de entrada; inclusive código que falharia se fosse executado.
Não há integração com o workspace Cargo principal, CLI, MCP ou outros adaptadores.

## Uso

A partir da raiz do repositório:

```sh
mkdir -p lab/adaptador-python/out
python3 -I -B lab/adaptador-python/adaptador.py \
  --root lab/adaptador-python/tests/fixtures/projeto \
  --graph lab/adaptador-python/out/grafo.json \
  --metadata lab/adaptador-python/out/metadados.json
```

`--root` é obrigatório e repetível. Cada raiz é um **diretório de busca**:
`src/pacote/__init__.py` corresponde ao módulo `pacote` se a raiz for `src`.
Para analisar um pacote, passe seu diretório **pai**, não o próprio pacote.
Raízes não recebem precedência entre si: dois candidatos ao mesmo nome geram
ambiguidade, não uma escolha baseada na ordem dos argumentos. Raízes iguais são
deduplicadas e ordenadas. Não há descoberta via ambiente, diretório corrente ou
`sys.path`. Caminhos relativos fornecidos pelo usuário são resolvidos normalmente.

Todos os `.py` sob as raízes são considerados, exceto `.git` e `__pycache__`.
Declare raízes estreitas para excluir ambientes virtuais e código de terceiros.
Links simbólicos são ignorados com diagnóstico, sem atravessar seu destino.
Diretórios intermediários de pacotes precisam ter `__init__.py` regular.
`__init__.py` na própria raiz é diagnosticado e ignorado: falta o nome do pacote.

Saídas são arquivos distintos. Diretórios de saída precisam existir. Códigos de
saída: **0** para leitura/análise sintática concluída no escopo suportado; **1**
para execução parcial com JSON e diagnósticos; **2** para erro de argumentos,
raiz ou escrita. Código 0 **não significa cobertura semântica completa**.

## Resolução e significado das arestas

- `import pacote.modulo`: resolve o módulo e seus inicializadores de pacote
  dentro das raízes declaradas. Gera dependências para os prefixos existentes.
- `from .vizinho import funcao` e `from ..helpers import X`: resolve a base
  relativa ao pacote do arquivo, respeitando os limites da raiz. Imports que
  sobem além do pacote são diagnosticados.
- `from pacote import Valor`: depende de `pacote`. Não cria `pacote.Valor` só
  porque `Valor` aparece na instrução; pode ser um símbolo.
- `from pacote import modulo`: acrescenta dependência ao submódulo se houver
  arquivo/pacote candidato e não houver binding estático conflitante no pacote.
  Binding, `__getattr__`, wildcard ou erro de leitura/análise na base impedem a
  decisão precisa e geram `ambiguous_symbol_or_module`. A dependência da base
  continua válida. Bindings condicionais são tratados conservadoramente.
- Nomes de módulo duplicados (inclusive arquivo versus pacote) geram
  `ambiguous_module`. Esses módulos e descendentes não entram como nós;
  referências geram `ambiguous_import`, sem aresta para um candidato arbitrário.
- Imports externos, incluindo biblioteca padrão, geram `external_dependency`.
  **Não há nós nem arestas externos**, instalação, busca no ambiente ou distinção
  entre pacote ausente e instalado. Nomes de topo locais com destino inexistente
  geram `unresolved_local_import`.
- Imports em funções, condições, `try` e `TYPE_CHECKING` são visitados sem avaliar
  condições. A aresta significa **import estático encontrado**, não execução
  comprovada, chamada ou referência a tipo. Arestas repetidas são deduplicadas;
  ciclos e imports de si próprio são mantidos, sem execução ou recursão no grafo.
- Um arquivo com erro sintático continua como nó conhecido, sem suas arestas de
  saída. Outros arquivos continuam sendo analisados. A execução fica parcial.

## Contrato JSON e metadados separados

`grafo.json` preserva o formato hoje aceito por
`lente_infra::desserializar_grafo`: `crate`, `nodes`, `edges`. Cada nó tem `id`,
`path`, `name`, `kind="mod"`, `visibility="pub"` e posição do arquivo.
Cada aresta tem `id_from`, `from`, `id_to`, `to`, `relation="uses"` e
`uses_kind="import"`. Não emite `owns`, `kind="crate"`, traits ou modificadores
Rust fictícios. Os defaults opcionais do leitor Rust permanecem sob seu contrato.

O campo obrigatório `crate="python"` é somente um **rótulo de compatibilidade**;
não declara um crate Rust. `visibility="pub"` também é um valor de compatibilidade
para módulos, sem inferir controle de acesso Python. O nome qualificado Python
`pacote.modulo` é preservado em `name`; `path="python::pacote::modulo"` adapta os
separadores para o formato atual. Não use esse prefixo para compor projetos
independentes sem um namespace de análise adicional.

IDs são inteiros sequenciais após ordenar os nomes qualificados. São únicos
**somente dentro de uma extração**, determinísticos para os mesmos arquivos,
raízes e versão do adaptador. Adicionar/remover módulos pode renumerá-los; não
há promessa de identidade persistente. Endpoints sempre correspondem ao ID e
path do nó. Posições são intervalos de arquivo, não de símbolos.

`metadados.json` contém versão própria (`metadata_version=1`), provedor,
linguagem, granularidade de módulo, raízes canônicas, alcance dos IDs, política
externa, capacidades, limites, contadores e diagnósticos ordenados. Inclui
SHA-256 dos bytes UTF-8 exatos do grafo para associar os dois arquivos. Não inclui
horário ou campos aleatórios. O formato atual da Lente **não consome esses
metadados**; aceitar o JSON não torna essa análise equivalente à extração Rust.

`execution.status` distingue arquivos lidos/analisados de arquivos ignorados,
erros e candidatos ambíguos. `semantic_coverage` é sempre `partial`, mesmo quando
`execution.status` é `complete`. Capacidades: imports entre módulos parcialmente
resolvidos; sem chamadas ou referências a tipos; posições de arquivo.

## Limites

Imports dinâmicos não são resolvidos. Chamadas literais a `__import__` e
atributos chamados `import_module` recebem diagnóstico; aliases e outras formas
indiretas podem não ser detectados. Não há promessa de inventário completo.
Namespace packages sem `__init__.py` não são suportados: arquivos são ignorados
com diagnóstico, nunca silenciosamente assumidos como pacotes regulares.
Mudanças de `sys.path`, `__path__`, import hooks, extensões nativas, bytecode,
reexports dinâmicos e atributos atribuídos em runtime não são avaliados.
A ausência de binding estático não prova ausência de binding em runtime.
`from ... import *` registra dependência da base e diagnóstico, sem expandir
`__all__`. A gramática suportada é a do interpretador que executa o adaptador.

## Testes e verificação contra o leitor real

```sh
python3 -B -m unittest discover -s lab/adaptador-python/tests -v
cargo run --offline --quiet \
  --manifest-path lab/adaptador-python/contract-check/Cargo.toml \
  --target-dir lab/adaptador-python/contract-check/target \
  -- lab/adaptador-python/out/grafo.json
```

O pequeno checker Cargo é um workspace **isolado** e só chama o leitor real do
L3 para verificar o JSON e seus endpoints. Não é adaptador de produção. Build,
lockfile e saídas ficam nesta pasta e são ignorados pelo Git. As fixtures cobrem
imports absolutos/relativos, pacote, símbolo, externo, ciclo e duplicidade de
raízes; testes criam ainda erro sintático, conflito símbolo/submódulo e código
com efeito observável que nunca deve executar. Determinismo também é verificado
em processos com `PYTHONHASHSEED` distintos e `PYTHONPATH` irrelevante ao escopo.
