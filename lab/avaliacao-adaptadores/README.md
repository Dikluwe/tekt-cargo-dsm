# Avaliação independente de adaptadores

Executor Rust isolado para os experimentos **TypeScript, Python, C, C++, Go e
Zig**. Cenários locais declarativos verificam disponibilidade, contrato JSON,
topologia esperada, diagnósticos, repetibilidade e duração. Não altera adaptadores,
harness, manifests ou crates principais; não depende do pacote de análise em
desenvolvimento. Reutiliza diretamente `lente_infra::desserializar_grafo` e
`lente_core::calcular_raio`, por dependências de caminho.

## Executar

Da raiz do repositório, com dependências Rust já presentes:

```sh
cargo build --locked --offline --manifest-path lab/avaliacao-adaptadores/Cargo.toml
lab/avaliacao-adaptadores/target/debug/avaliacao-adaptadores
```

Cada cenário executável tem **3 repetições** no arquivo fornecido. Para mudar:

```sh
lab/avaliacao-adaptadores/target/debug/avaliacao-adaptadores --repetitions 5
lab/avaliacao-adaptadores/target/debug/avaliacao-adaptadores \
  --suite lab/avaliacao-adaptadores/cenarios.json --repo . --repetitions 2
```

O executor imprime o caminho de `relatorio.md`. A mesma pasta contém
`relatorio.json`, cópia da suíte, logs stdout/stderr por subprocesso e todas as
saídas dos adaptadores. Execuções usam diretórios exclusivos em `runs/run-*`;
amostras usam `cenário/sample-NNNN/output/`. Nenhuma saída anterior é reutilizada.
Fixtures permanecem no mesmo caminho entre repetições para permitir comparação
byte a byte do grafo, inclusive posições absolutas. Logs ficam fora do diretório
de saída do adaptador C, que exige diretório vazio.

**Não há instalação ou download automático.** Requisitos são sondados e
registrados; um executável/arquivo ausente produz `ferramenta_ausente`, sem tentar
compilar ou instalar nada. Cargo aqui é só o preparo explícito do avaliador e usa
`--offline`; o processo de avaliação nunca chama Cargo/npm/pip/gerenciadores de
pacotes. Disponibilize previamente, conforme os READMEs dos adaptadores:

| Adaptador | Requisitos da suíte |
|---|---|
| TypeScript | Node, TypeScript local 5.9.3, `extrair.cjs` |
| Python | Python 3.10+ e `adaptador.py` |
| C | Binário já preparado em `target/debug/lente-adaptador-c-experimental`, libclang 18 |
| C++ | Python, fontes do adaptador, libclang 18 |
| Go | Go instalado e binário já preparado em `target/debug/lente-adaptador-go` |
| Zig | Python, Zig 0.15.2 e helper já preparado em `bin/parser` |

A suíte aponta libclang para `/usr/lib/llvm-18/lib/libclang.so`. Se a instalação
for diferente, revise uma cópia do JSON e passe `--suite`; não há descoberta
silenciosa de bibliotecas alternativas. `probe_libclang.py` consulta a versão da
biblioteca explícita com ctypes, sem instalar ou compilar. Binários, scripts,
contratos e arquivos das fixtures declarados em requisitos têm SHA-256 registrado.
O relatório também identifica o binário do avaliador e a suíte por hash.

## Status e uso em CI

| Status do cenário | Significado |
|---|---|
| `aprovado` | Requisitos, códigos de retorno, grafo, metadados, topologia e repetições atenderam ao cenário |
| `reprovado` | Divergência, timeout, artefato inválido/ausente após extração, erro operacional ou falha de preparação |
| `ferramenta_ausente` | Executável ou arquivo requerido não está disponível |
| `nao_suportado` | Capacidade explicitamente fora do contrato, versão fora do requisito ou plataforma sem mecanismo de encerramento implementado |

Exit code **0**: todos os cenários/comparações aprovados; **1**: alguma reprovação
ou erro de invocação; **2**: sem reprovações, mas avaliação incompleta por ausência
ou não suporte. O JSON distingue os motivos. Comparações sem duas avaliações
aprovadas são marcadas `nao_suportado`, com motivo; não se fabricam resultados.

A suíte entregue inclui `zig-target-evaluation`, deliberadamente não suportado:
o adaptador não avalia target/optimize ou ramos comptime. Assim, **uma execução
esperada da suíte completa retorna 2**, com os demais cenários aprovados. Para um
job CI que exija exit 0, mantenha uma suíte somente com capacidades requeridas e
suportadas; não converta indiscriminadamente exit 2 em sucesso.

Códigos dos adaptadores são preservados em `samples[].process.code`. Cada cenário
declara sua lista `expected.exit_codes`; parcial pode ser uma aprovação do teste:

| Adaptador | Parcial observado pelo contrato |
|---|---|
| TypeScript | exit 0, `/coverage = partial` |
| Python | exit 1, `/execution/status = partial`; cobertura semântica sempre parcial |
| C | exit 2, `/status = partial`; TU inválida produz grafo vazio |
| C++ | exit 1, `/execution/status = partial`; TU inválida preserva seu nó |
| Go | exit 2, `/loading = partial`; pode incluir endpoint não carregado |
| Zig | exit 0, `/completeness/syntax = partial`; semântica não avaliada |

Python não distingue uma dependência externa instalada de uma ausente: o cenário
`python-missing` espera `external_dependency`, grafo sem destino externo e exit 0.
Isso testa a política declarada; **não** comprova detecção de falta de instalação.

## Etapas e medições

1. **Requisitos:** verifica arquivos e executa somente sondagens declaradas em
   argv. Registra versões, hashes, código, duração e logs; fora das amostras.
2. **Preparação:** materializa arquivos declarativos, como compilation database
   com paths absolutos. Executa comandos de preparação apenas se presentes no
   cenário (a suíte entregue não precisa de comandos de preparo). Medida uma vez.
3. **Comando do adaptador:** duração de parede do processo até seu término, com
   coleta de status e encerramento do grupo. Inclui startup, leitura, extração,
   validação interna e escrita que o adaptador fizer. **Não** chamamos esse tempo
   de parsing puro ou estimamos subetapas que a interface não permite medir.
4. **Validação posterior:** leitura dos artefatos, desserializador real, hash de
   associação quando fornecido, contagens, nós/arestas exatos e metadados esperados.
5. **Análise posterior:** `calcular_raio` para cada nó, profundidades jusante e nós
   isolados. Custo O(V × (V+E)) aproximado para consultas de todos os nós; não é
   medição de uma consulta interativa ou do pacote de análise paralelo.

Relatório legível: mediana, MAD, primeira amostra e mediana das subsequentes;
tabela adicional para preparação/validação/análise. JSON: **todas** as amostras,
mesmo falhas, além de mediana/MAD/mínimo/máximo/n. Estatísticas de desempenho usam
somente amostras aprovadas; nenhuma falha vira tempo zero. Resultado ausente é
null, nunca duração inventada. MAD é mediana dos desvios absolutos da mediana;
com uma amostra, MAD zero não demonstra estabilidade.

Primeira significa **primeira medida daquele cenário**, não cache frio. O
avaliador não limpa cache do sistema operacional nem caches globais. Go recebe
GOCACHE local ao cenário, compartilhado entre suas amostras. Não há warmup
oculto. Requisitos, hashing e casos anteriores podem aquecer filesystem/cache.
As execuções são sequenciais; concorrência externa continua afetando as medições.
A sondagem do término ocorre a cada 5 ms: o overhead/resolução do executor afeta
comandos muito curtos. O custo de renderização do relatório não é atribuído à
extração nem medido como fase interna do adaptador.

## Cenários e limites de comparação

São 36 cenários executáveis (seis por adaptador), mais o caso Zig não suportado:

- **chain:** três elementos a → b → c; exatamente duas arestas, sem a → c;
  o núcleo deve encontrar profundidade 2. Repetições verificam os bytes do grafo.
- **isolated:** um elemento e nenhuma aresta.
- **missing:** dependência ausente e diagnóstico específico do contrato.
- **partial:** construção não suportada/erro que preserva o resultado permitido.
- **config-a/config-b:** duas configurações legítimas, com comparação explícita
  exigindo grafos diferentes.

Configurações usadas: aliases `paths` em TypeScript; raízes explícitas com
implementações locais distintas em Python; define `EXTRA` em C/C++; build tag
`feature` em Go; mapa global de módulos em Zig. Nenhum build/teste/programa das
fixtures é executado. C/C++ recebem compilation databases geradas, sem executar
os compiladores descritos nelas.

Não se compara automaticamente linguagens. Cada cenário declara **granularity,
relations e universe**. Comparações com algum desses campos diferente são
recusadas antes de executar qualquer comando. Só existem comparações entre
configurações do mesmo adaptador na suíte entregue:

| Adaptador | Unidade | Relação/alcance |
|---|---|---|
| TypeScript | Arquivo | Import/reexport estático local |
| Python | Módulo | Import sintático resolvido nas raízes |
| C | Arquivo | Include ativo de uma TU, inclusive headers externos |
| C++ | Arquivo | Referência semântica observada entre arquivos locais |
| Go | Pacote | Import direto no universo configurado |
| Zig | Arquivo | Candidato sintático, sem prova de ativação |

Embora várias fixtures gerem 3 nós e 2 arestas, isso **não torna as relações
semanticamente equivalentes**. Não ordene provedores por número de arestas ou
pelos tempos desta tabela. O rótulo `uses/import` do JSON é insuficiente para
estabelecer comparabilidade. Todas as comparações usam correspondência explícita
por `node.name`, que precisa ser único nas fixtures; comparam nomes, pares e
profundidades. Não há algoritmo de isomorfismo geral ou fusão de linguagens.

A validação crua permite paths repetidos conforme o contrato atual. A análise
posterior os recusa por requerer a forma resolvida; isso é reportado como falha
da etapa analítica, não como JSON inválido. Nomes repetidos tornam a
correspondência desta suíte ambígua. Comparação estrutural não comprova
completude semântica, cobertura de projetos reais ou equivalência comportamental.

## Formato declarativo e extensão

`cenarios.json` é a especificação executável, com `version: 1`, `scenarios` e
`comparisons`. Campos desconhecidos são rejeitados. Exemplo abreviado de cenário
(os comandos são vetores, nunca strings de shell):

```json
{
  "id": "exemplo",
  "adapter": "python",
  "granularity": "module",
  "relations": "static-import",
  "universe": "declared-python-roots",
  "requirements": [{"name": "Python", "command": {
    "argv": ["python3", "--version"], "cwd": "{repo}", "timeout_ms": 5000
  }}],
  "preparation": {"files": {}, "commands": []},
  "command": {
    "argv": ["python3", "-I", "-B", "{repo}/lab/adaptador-python/adaptador.py",
      "--root", "{lab}/fixtures/python/isolated",
      "--graph", "{out}/grafo.json", "--metadata", "{out}/metadados.json"],
    "cwd": "{repo}", "timeout_ms": 20000
  },
  "graph": "{out}/grafo.json",
  "metadata": "{out}/metadados.json",
  "expected": {
    "exit_codes": [0], "nodes": 1, "edges": 0, "names": ["isolated"],
    "pairs": [], "edge_fields": {"relation": "uses", "uses_kind": "import"},
    "metadata": {"/execution/status": "complete"}, "depths": []
  },
  "repetitions": 3, "deterministic": true
}
```

Tokens: `{repo}` raiz do checkout; `{lab}` pasta do avaliador; `{run}` execução;
`{case}` preparação compartilhada pelo cenário; `{out}` saída exclusiva da
amostra. Substituição textual ocorre uma vez por argumento/string JSON, sem
expansão de variáveis, globs, pipes, redirecionamentos ou `$(...)`. Paths em
`preparation.files` devem ser relativos e não podem conter `..`. Valores string
são gravados como texto; objetos/arrays como JSON após substituir tokens.

Expectativas `names` e `pairs` são conjuntos exatos ordenáveis (não apenas
mínimos). `metadata` usa JSON Pointer e igualdade de valor. `diagnostics_min`
e `diagnostic_contains` exigem diagnósticos preservados; `edge_fields` exige
campos das arestas. `depths` contém `[origem, destino, distância]`.
`deterministic` exige bytes idênticos do **grafo** entre amostras aprovadas;
metadados são preservados integralmente, mas não comparados byte a byte.

Requisitos podem declarar arquivos, comando de versão e `version_contains`.
Versão incompatível vira `nao_suportado`; falha de execução da sondagem vira
`reprovado`. Capability ausente pode ser declarada com `unsupported: "motivo"`,
sem comando nem expectativas. Para acrescentar comparações, declare `left`,
`right`, `expect: same|different` e `reason`, com as três dimensões compatíveis.

Cenários são instruções **confiáveis e revisáveis**: argv explícito não torna
um executável arbitrário seguro. O executor não interpreta shell e recusa nomes
usuais de shells, mas não é sandbox nem audita o que um script aprovado faz.
Revise comandos novos, principalmente preparação; os entregues apenas leem
fixtures e escrevem resultados locais. O ambiente é herdado com overrides
registrados; a suíte limpa preloads Node e paths Python/headers e bloqueia buscas
Go. Não é isolamento hermético do sistema operacional.

## Timeout e plataforma

Validado em Linux. Em Unix, cada subprocesso recebe um grupo próprio. No timeout,
o executor envia SIGKILL ao grupo, encerra/recolhe o filho direto e continua os
outros cenários. Também encerra descendentes remanescentes após término normal.
Logs são arquivos, evitando bloqueio por buffer de pipe. Não use comandos que
pretendam deixar serviços em background.

Processos que deliberadamente escapam do grupo (setsid/novo grupo), processos
remotos e código no kernel não são contidos por essa técnica. Sistemas sem
implementação do mecanismo Unix são reportados `nao_suportado` **antes** de
iniciar subprocessos; não há promessa de suporte Windows/Job Objects. Interrupção
externa do avaliador não equivale ao seu timeout controlado. Logs não têm limite
de tamanho e fixtures devem ser pequenas e locais.

## Testes do executor

```sh
cargo test --locked --offline --manifest-path lab/avaliacao-adaptadores/Cargo.toml
cargo clippy --locked --offline --manifest-path lab/avaliacao-adaptadores/Cargo.toml \
  --all-targets -- -D warnings
cargo fmt --manifest-path lab/avaliacao-adaptadores/Cargo.toml --check
```

Os testes inline incluem: argv literal, recusa de shell, timeout com descendente,
código não zero esperado, ferramenta ausente, versão não suportada, cwd inválido,
preparo que falha, JSON/enums/endpoints/hash inválidos, separação forma crua/resolvida,
comparação incompatível, diretórios concorrentes exclusivos, repetibilidade que
falha sem perder amostras, mediana/MAD e separação primeira/subsequentes.
Os processos auxiliares são modos de um teste Rust próprio, sem instalar ferramentas.
