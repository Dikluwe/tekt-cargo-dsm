# Regras arquiteturais sobre snapshots

Biblioteca pura e executor local, em workspace independente. Consome grafos **já
extraídos** e políticas explícitas. Não infere camadas por diretórios, não lê
`crystalline.toml`, não duplica verificações de linhagem/snapshots/camadas do
crystalline-lint e não integra CLI/MCP ou CI.

O contrato normativo é [CONTRATO.md](CONTRATO.md). Foram conferidos os artefatos
atuais das entregas **13** (`lab/pacote-analise/`) e **17** (comparação por identidade
opaca em `lente_comparacao`). A leitura de grafo usa o desserializador atual;
testemunhas usam `explicar_caminho_impacto`; SCCs usam `detectar_ciclos`.
Não se copiam esses algoritmos. Singleton com self-loop é tratado localmente,
pois a API de SCC existente retorna apenas componentes com dois ou mais nós.

## Executar

Da raiz do checkout, com Rust e dependências locais já disponíveis:

```sh
cargo build --locked --offline --manifest-path lab/regras-arquiteturais/Cargo.toml
lab/regras-arquiteturais/target/debug/regras-arquiteturais \
  --baseline lab/regras-arquiteturais/examples/baseline.json \
  --current lab/regras-arquiteturais/examples/current.json \
  --rules lab/regras-arquiteturais/examples/rules.json \
  --as-of 2026-09-26 --format text
```

Esse exemplo retorna **1**: há uma dependência transitiva nova A → B → C e um
SCC novo B ↔ C. A dependência direta antiga A → B está coberta por uma exceção
até 2026-12-31. Nomes/IDs locais diferem entre baseline e atual, mas a
correspondência foi declarada explicitamente. O nó D permanece isolado.

O formato padrão é JSON; `--format text` produz texto. Saída em stdout, erros
em stderr. Não há execução de adaptadores, subprocessos ou downloads. Para guardar
resultados, redirecione stdout para um arquivo escolhido; o executor não grava
sobre nenhum input. `--as-of` é obrigatório: não há relógio implícito na avaliação.

| Exit | Resultado |
|---|---|
| **0** | Aprovado para o enforcement e escopo declarados, incluindo exceções explícitas |
| **1** | Violação arquitetural exigível não excetuada |
| **2** | Inconclusivo: qualidade, capacidade, contexto ou seleção insuficiente |
| **3** | Erro de entrada, versão/schema, identidade, exceção, digest, I/O ou serialização |

Inconclusivo tem precedência global sobre violação; violações conclusivas de
outras regras permanecem visíveis no relatório. Exceções não removem essa
precedência. Uma aprovação arquitetural **não comprova correção funcional**.

Outros exemplos, usando os mesmos argumentos baseline/current/data:

- `allowed-rules.json`: nenhuma aresta direta A → D; retorna 0.
- `baseline-rules.json`: exige apenas regressões (`new_only`); a dependência
  antiga A → B fica `preexisting`, não é escondida; retorna 0.
- `--current examples/partial.json`: metadados da origem declaram parcial;
  retorna 2 mesmo que a declaração local tente dizer complete.
- `invalid-exception-rules.json`: exceção sem justificativa; retorna 3.

Use os paths completos sob `lab/regras-arquiteturais/` nos comandos acima.

## Formato de snapshot v1

Veja `examples/baseline.json` e `current.json`. `graph` e `source_metadata` são
relativos ao arquivo descritor (ou absolutos), nunca ao diretório do executável.

```json
{
  "version": 1,
  "graph": "graph.json",
  "graph_sha256": "SHA-256 DOS BYTES EXATOS DO ARQUIVO",
  "context": {
    "project": "meu-projeto",
    "provider": "meu-provedor",
    "provider_version": "1",
    "granularity": "file",
    "nature": "static-import",
    "configuration": "configuracao-declarada-v1",
    "scope": "arquivos-locais-selecionados"
  },
  "quality": {
    "execution": "complete",
    "coverage": "complete",
    "capabilities": ["direct_dependencies", "reachability", "cyclic_components"],
    "diagnostics": []
  },
  "identities": [
    {"node_id": 10, "identity": "componente:entrada"},
    {"node_id": 20, "identity": "componente:base"}
  ]
}
```

Todos os nós precisam de identidade canônica não vazia, única dentro do snapshot.
Dois nós com o mesmo path cru podem receber identidades diferentes por ID.
Identidade repetida, ID inexistente, mapa incompleto e endpoints inconsistentes
são erros. Igualdade de identidade entre os **dois mapas fornecidos** declara o
pareamento; ausência de uma chave em um lado representa elemento sem par nessa
amostra. Não existe fallback por nome, path, prefixo Rust ou ID numérico local.

As sete dimensões de contexto devem coincidir e ser conhecidas; caso contrário,
o resultado é inconclusivo. `granularity` admite item/file/module/package.
Comparar configurações, naturezas ou provedores distintos exige outro contrato;
um mapeamento de nomes não torna includes C equivalentes a referências C++ ou
imports de pacotes Go. `configuration` é uma identidade declarada, não um hash
calculado automaticamente dos arquivos de build.

`quality` é uma **declaração do produtor/chamador** para essa natureza e escopo.
Não invente complete ou capacidades ausentes para conseguir aprovação. Valores
permitidos de execução/cobertura: complete, partial, unknown; omissão significa
unknown. Omissão de capacidade significa que ela não está disponível. Regras de
alcançabilidade exigem direct_dependencies e reachability; ciclos exigem também
cyclic_components. Uma regra direta pode continuar conclusiva se faltar somente
a capacidade de ciclos.

### Metadados e confiança

O digest obrigatório vincula a declaração aos bytes do grafo; não autentica o
produtor nem comprova a verdade das capacidades. O leitor não certifica o extrator.
Preserve a declaração do produtor e seus diagnósticos, em vez de inventar uma
classificação por linguagem ou granularidade.

`source_metadata` opcional referencia os metadados originais do adaptador, que
são preservados no JSON de saída. Um `graph_sha256` nativo, quando presente, deve
coincidir. Estados negativos em campos conhecidos sempre bloqueiam aprovação;
versão/schema/provedor desconhecido também. Não se promove a estado completo
uma origem parcial porque o processo retornou 0 ou o JSON foi aceito.

A leitura conservadora segue os schemas conferidos na entrega 13: TS/Zig
schema_version=1, Python/C++ metadata_version=1, Go contract_version=1; C não
possui versão de schema no contrato atual. Estados como partial, error, unknown,
not-evaluated e not-claimed impedem conclusão, incluindo cobertura semântica.
Por isso alguns adaptadores atuais continuarão **inconclusivos** mesmo com
carregamento completo. Não se remove essa limitação nem se implementa uma
inferência nova de capacidade nesta etapa.

Sem `source_metadata`, o descritor local é a declaração explícita de qualidade;
não há busca automática de sidecars. Quem o produz deve conservar qualquer
parcialidade da origem. Se usar um pacote da entrega 13, verifique-o com sua
ferramenta própria antes; este executor pode ler seus arquivos graph/metadata,
mas não substitui `verify`, a âncora do manifesto ou a política de privacidade.
O relatório conserva os inputs e pode conter paths/diagnósticos/metadados: revise
seu conteúdo antes de distribuí-lo.

## Regras v1

```json
{
  "version": 1,
  "enforcement": "new_only",
  "rules": [
    {
      "id": "interface-nao-importa-storage",
      "policy": {
        "kind": "forbid_direct",
        "from": {"prefixes": ["interface:"]},
        "to": {"ids": ["storage:db"]}
      }
    },
    {
      "id": "interface-nao-alcanca-db",
      "policy": {
        "kind": "forbid_reachability",
        "from": {"prefixes": ["interface:"]},
        "to": {"ids": ["storage:db"]}
      }
    },
    {
      "id": "sem-novos-ciclos-de-storage",
      "policy": {
        "kind": "no_new_cycles",
        "scope": {"prefixes": ["storage:"]}
      }
    }
  ],
  "exceptions": []
}
```

`enforcement=all` cobra proibições diretas/transitivas antigas e novas;
`new_only` cobra apenas as novas e mantém antigas explicitamente na saída.
`no_new_cycles` sempre cobra só regressões de componentes cíclicos. A mesma
versão atual das regras é aplicada aos dois snapshots; não há histórico de regras
nem alegação de que uma regra já existia na época da baseline.

IDs de regras são autorais, obrigatórios, únicos e não recalculados. Mudar o
significado de uma regra pode merecer um novo ID por decisão do autor.

Seletores são união de `ids` exatos e `prefixes` literais sobre **identidades
canônicas**, não paths. Prefixo usa starts_with, diferencia caixa e formas
Unicode, não contém glob/regex e não interpreta `/`, `.`, `::` ou diretórios.
`"foo*"` significa prefixo literal com asterisco. Cada termo precisa encontrar
algo no atual; um termo incorreto deixa a regra inconclusiva mesmo se outros
termos encontrarem nós. Seletor vazio ou termo repetido é erro de entrada.

### Direta, transitiva e testemunhas

Somente relações `Uses` participam; todas as naturezas/subtipos presentes devem
ser compatíveis com a natureza declarada. `Owns` não propaga dependências.
Arestas paralelas se reduzem a um par dirigido; peso não altera uma proibição.

- Direta exige a aresta origem → destino, inclusive self-loop.
- Alcançabilidade proíbe caminhos de **um ou mais saltos**, incluindo diretos.
  Caminhos podem atravessar elementos fora dos conjuntos dos seletores.
- Mesma ponta exige percurso fechado positivo; o caminho reflexivo de zero
  saltos não é violação.
- Testemunhas apontam **consumidor → dependência**, com endpoints incluídos.
  Para pontas distintas, a API existente escolhe a cadeia mínima, com desempate
  lexicográfico pelas identidades canônicas da projeção. Reporta hops e distingue
  direct/transitive. Nenhuma cadeia é prova de quebra funcional.

Uma violação direta/transitiva é antiga se o mesmo par satisfazia a mesma
proibição na baseline. Alterar a cadeia intermediária não cria nova violação
transitiva se a alcançabilidade desse par já existia.

### O que conta como ciclo novo

O objeto avaliado é o **componente fortemente conectado cíclico**, não a contagem
de ciclos nem a enumeração de ciclos simples. Inclui SCCs com ≥2 nós e self-loops.
O scope seleciona SCCs que intersectam o conjunto: seus membros externos ao
scope não são cortados, pois isso poderia esconder um ciclo que cruza a fronteira.

Um componente atual é antigo somente se:

1. Todos os seus membros pertenciam a **um único SCC cíclico** da baseline.
2. Todas as suas arestas internas já existiam na baseline.

Caso contrário, é regressão. Isso detecta troca de componentes com a mesma
contagem, expansão/fusão de SCCs e nova aresta interna em SCC com mesmos membros.
Remover arestas, encolher ou dividir um SCC sem acrescentar arestas cíclicas não
é regressão. Mudanças só no subtipo Uses não são avaliadas por estas regras.

A saída registra membros, arestas internas, `added_internal_edges` e uma
testemunha fechada. Se houver aresta interna nova, a testemunha inclui a primeira
nova aresta em ordem lexicográfica e um caminho mínimo de retorno. Assim, não
apresenta somente um ciclo antigo como evidência da regressão. Não enumera todos
os ciclos simples nem afirma estabilidade comportamental de um SCC inalterado.

## Exceções

```json
{
  "id": "migracao-db-2026",
  "rule_id": "interface-nao-importa-storage",
  "scope": {
    "kind": "pairs",
    "from": {"ids": ["interface:legacy"]},
    "to": {"ids": ["storage:db"]}
  },
  "justification": "Migração aprovada no ADR local 42, etapa de remoção pendente.",
  "until": "2026-12-31"
}
```

Para ciclos: `scope={"kind":"component","members":{"ids":["A","B"]}}`.
Todos os membros do SCC precisam estar nesse escopo; uma exceção só para A não
silencia um ciclo A ↔ B. Exceções de pares cobrem endpoints, não exigem que
intermediários de uma cadeia pertençam ao escopo. Não permitem exceção global
implícita: regra e escopo são obrigatórios.

Justificativa vazia/branca, regra inexistente, tipo de escopo incompatível, ID
repetido ou data inválida são erro 3. `until` é opcional e inclusivo;
`as_of == until` ainda é válido. Exceções válidas coincidentes são todas listadas.
As expiradas ficam `expired`; válidas sem aplicação, `unused`; aplicadas, `applied`.
Regras cobertas recebem `excepted`, **separado de `approved`**. Dívidas antigas
sob new_only ficam `preexisting`. Exceções não corrigem dado parcial ou capacidade
insuficiente e não removem os achados do relatório.

## Identificadores, biblioteca e limites

`src/lib.rs` expõe `evaluate(&Snapshot, &Snapshot, &Rules, Date) -> Result<Report>`;
não lê arquivos, relógio, ambiente nem serializa. Modelos são tipos próprios;
`src/files.rs` faz DTO/serde, hashes, leitura e apresentação; `src/main.rs` trata
argumentos/códigos de saída. Nenhum tipo compartilhado foi alterado.

A projeção interna remapeia cada nó por ID local para uma identidade única;
não agrega nós, não une grafos e não é publicada como grafo extraído. Paths/IDs
originais permanecem rastreáveis em `origins.baseline/current`. A API da entrega
17 foi conferida: sua comparação ponderada de estruturas agregadas não resolve
estas políticas; reutilizamos o contrato de identidades opacas, sem matching
legado por normalização Rust.

IDs de violação usam versão + regra + tipo + par, ou membros/arestas de SCC,
com strings prefixadas pelo comprimento em bytes UTF-8. Não há concatenação
ambígua, DefaultHasher, timestamp ou ID local. O contexto de projeto delimita
seu namespace. IDs de proibições transitivas não dependem da testemunha.
Uma redução de SCC pode alterar seu ID e ainda ser classificada preexistente:
ID reproduzível de conteúdo não é uma linhagem de SCCs.

A avaliação pura é determinística sob permutação de nós, arestas, mapas e regras;
arestas paralelas não mudam o resultado. JSON completo é byte a byte estável com
os mesmos arquivos, paths de invocação e as_of; sua proveniência preserva os
inputs, então arquivos reformatados podem mudar hashes e o relatório completo.

Custo: projeção e SCCs sobre grafos inteiros; consulta de caminho por par
selecionado. Seletores grandes podem gerar produto cartesiano e várias BFS.
Não há cache, limite automático de grafo, orçamento de tempo ou benchmark nesta
entrega. A declaração de qualidade não é autenticada; um autor pode mentir.
Nenhuma aprovação substitui revisão do escopo, dos metadados ou do código.

## Testes

```sh
cargo test --locked --offline --manifest-path lab/regras-arquiteturais/Cargo.toml
cargo clippy --locked --offline --manifest-path lab/regras-arquiteturais/Cargo.toml \
  --all-targets -- -D warnings
cargo fmt --manifest-path lab/regras-arquiteturais/Cargo.toml --check
```

Cobertura: direta permitida/proibida, transitiva sem aresta direta, antigo/novo,
troca de SCCs com contagem igual, fusão/expansão/encolhimento/divisão, nova aresta
interna, self-loop, renomeação explícita, Owns, paths crus repetidos, exceções
válidas/expiradas/sem justificativa, seletores vazios de resultados, parcialidade,
capacidade/contexto insuficientes, qualidade nativa desconhecida e determinismo.
