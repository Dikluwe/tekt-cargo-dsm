# Conformidade de grafos da Lente

Harness independente para **arquivos JSON já extraídos**. Não invoca Cargo como
extrator, não executa código dos projetos analisados e não depende dos adaptadores
Python ou TypeScript. O compilador Rust/Cargo é necessário somente para construir
o harness. Requer Rust >= 1.91.

Possui workspace e `Cargo.lock` próprios. As dependências de caminho apontam para
os crates atuais da Lente: infraestrutura (desserialização), núcleo (raio),
estrutura (ciclos) e ranking. Nenhum desses algoritmos foi copiado. Mudanças nesses
crates feitas por trabalhos paralelos podem alterar o resultado ou exigir uma
recompilação; o harness não congela suas APIs numa cópia.

## Construir e testar

Da raiz do repositório:

```sh
cargo test --offline --locked --manifest-path lab/conformidade-grafos/Cargo.toml
cargo build --offline --locked --manifest-path lab/conformidade-grafos/Cargo.toml
```

`--offline` pressupõe que as dependências Rust do lock já estejam disponíveis.
Se não estiverem, faça a preparação explícita com `cargo fetch --locked
--manifest-path lab/conformidade-grafos/Cargo.toml` antes. O binário nunca baixa
nem instala dependências. Os artefatos ficam no `target/` deste workspace isolado
(salvo override explícito de `CARGO_TARGET_DIR` pelo usuário).

## Validar uma saída de adaptador

```sh
lab/conformidade-grafos/target/debug/conformidade-grafos /caminho/grafo.json
```

Pode fornecer vários arquivos. Todos são examinados; um erro não esconde o
relatório dos seguintes:

```sh
lab/conformidade-grafos/target/debug/conformidade-grafos \
  lab/conformidade-grafos/fixtures/cadeia.json \
  lab/conformidade-grafos/fixtures/ciclo.json \
  lab/conformidade-grafos/fixtures/isolado.json
```

Para nomes começando com hífen, use `--` antes dos arquivos. Caminhos com espaços
devem ser protegidos pelas aspas do shell. `--help` mostra a sintaxe.

O modo padrão chama **`lente_infra::desserializar_grafo`** e verifica a forma crua
aceita pelo adaptador existente: IDs únicos, endpoints existentes, concordância
do texto `from`/`to` com os IDs, e tradução dos enums. Os arquivos devem usar o
formato atual `crate`/`nodes`/`edges` descrito na spec central; o harness não inventa
conversão automática de formatos de outros provedores nem valida envelopes novos.

**Paths repetidos são permitidos na forma crua.** O relatório informa quantos paths
colidem. Isso não representa falha nem aprovação da forma resolvida.

As tolerâncias do adaptador são herdadas: campos opcionais podem ser omitidos,
campos desconhecidos podem ser ignorados, `kind` composto é normalizado e, na
implementação consultada, `uses_kind` desconhecido em `uses` vira `Import`; em
`owns` o subtipo é descartado. Portanto, este harness não é um validador de JSON
Schema estrito nem comprova que todo campo foi reconhecido. Os testes de enum
inválido cobrem `kind`, `visibility` e `relation`. Nenhuma tolerância foi corrigida
ou escondida por um parser paralelo aqui.

## Executar os cálculos

```sh
lab/conformidade-grafos/target/debug/conformidade-grafos --analitico \
  lab/conformidade-grafos/fixtures/cadeia.json
```

Esse modo exige **paths únicos** e chama os algoritmos locais sobre o grafo tal
como fornecido, sem filtragem, agregação ou resolução automática de colisões.
Relata, por ID: graus de vizinhança `Uses`, classificação, profundidades mínimas
para cada nó alcançável em ambas as direções, profundidades máximas e contenção
`Owns`. Também apresenta SCCs retornados pelo núcleo (ciclos com >= 2 nós) e as
pontuações/classificações do ranking completo. Não interpreta ausência de SCC
como prova de ausência de autoarestas. Arestas paralelas permanecem no
multiconjunto; grau e alcançabilidade contam vizinhos, não duplicatas.

Paths únicos são apenas a precondição deste ensaio. Não certificam aplicação da
cascata de resolução, validade de toda hierarquia `Owns` ou cobertura semântica.
O custo cresce com os grafos e com os cálculos por nó. Não há promessa geral de
latência baseada nestas fixtures pequenas.

## Comparar topologias com nomes e IDs diferentes

```sh
lab/conformidade-grafos/target/debug/conformidade-grafos \
  --mapa lab/conformidade-grafos/fixtures/mapa-cadeia.json \
  lab/conformidade-grafos/fixtures/cadeia.json \
  lab/conformidade-grafos/fixtures/cadeia-renomeada.json
```

O mapa é uma lista de pares de IDs, sempre **primeiro arquivo → segundo arquivo**:

```json
[[1, 90], [2, 70], [3, 10]]
```

Deve ser total e bijetivo: todos os nós dos dois grafos aparecem exatamente uma
vez. IDs inexistentes, repetições, cobertura incompleta e mapas malformados falham.
Não há inferência por ordem, rótulo, nome de arquivo ou semelhança de texto.
`--mapa` implica análise e exige exatamente dois arquivos.

A comparação normaliza pelos IDs do primeiro grafo e exige igualdade de:

- multiconjunto de arestas dirigidas, incluindo `Owns`/`Uses`, subtipo e duplicatas;
- graus, classificações, destinos alcançáveis **com suas profundidades** e contenção;
- conjuntos de nós dos ciclos e pontuações/classificações de ranking.

Desempates lexicais do ranking podem mudar com os nomes; por isso a comparação
usa a pontuação por nó correspondente, não sua posição na lista. Rótulos, nomes,
visibilidade, tipos de item e posições de código não são certificados como
semanticamente equivalentes. Mesmas contagens globais, sozinhas, não bastam.

## Fixtures e resultados esperados

| Arquivo | Resultado |
|---|---|
| `cadeia.json` | a → b → c; impacto de c tem b a 1 salto e a a 2 |
| `cadeia-renomeada.json` + `mapa-cadeia.json` | Mesma cadeia, IDs/nomes/ordens diferentes |
| `ciclo.json` | Ciclo a → b → c → a; dois outros nós no raio de cada alvo |
| `isolado.json` | Um nó, zero arestas, profundidade zero |
| `paralelas.json` | Duas relações a → b; um vizinho direto |
| `paths-repetidos.json` | Aceito na forma crua, recusado por `--analitico` |
| `id-duplicado.json` | Falha de ID duplicado |
| `endpoint-ausente.json` | Falha de referência a ID inexistente |
| `endpoint-texto-incompativel.json` | Falha de divergência texto/ID |
| `enum-invalido.json` | Falha de relation desconhecida |

As fixtures são grafos sintéticos do contrato atual; não alegam ter sido extraídas
de Rust, Python ou TypeScript. Os testes também verificam ambos os endpoints,
mapas inválidos, mudança de relação/multiplicidade e determinismo do relatório.

## Códigos de saída e integração em CI

- **0:** todas as verificações solicitadas passaram.
- **1:** falha de leitura, validação crua, precondição analítica ou comparação.
- **2:** uso inválido da linha de comando.

Para integrar um novo adaptador: produza seu arquivo no contrato vigente, execute
o harness no modo cru e depois execute ensaios analíticos com paths únicos. Para
comparar com uma referência, forneça a correspondência explícita dos nós. Mantenha
separados os testes que comprovam se seu extrator reconhece construções da linguagem.

**Aprovação estrutural não comprova completude semântica do extrator, equivalência
comportamental, granularidade adequada, estabilidade de IDs entre extrações nem
fidelidade das relações ao código original.** Esses pontos exigem contratos e
oráculos próprios do provedor. O harness não une grafos nem identifica linguagens.

## Propriedades geradas e oráculo independente

Além das fixtures, `src/propriedades.rs` (compilado **somente em testes**) executa
as nove propriedades abaixo. O harness de uso continua delegando todos os
cálculos às APIs públicas de produção; somente o **esperado dos testes** usa um
oráculo independente: matriz de distâncias com Floyd–Warshall, sem BFS, fila,
`calcular_raio` ou função de explicação. SCCs esperados vêm de alcançabilidade
mútua; pontuações esperadas são as cardinalidades do montante. A própria matriz
é conferida contra cadeia, atalho, ciclo e autoaresta com resultados conhecidos.

| Teste (`propriedades::…`) | Invariante / domínio |
|---|---|
| `permutacao` | Ordem dos nós/arestas não muda métricas, topologia, SCCs ou pontuações |
| `renomeacao_bijetiva` | IDs/paths novos preservam assinatura após mapa inverso; não compara desempates posicionais |
| `duplicacao` | Mesmo alcance, graus, profundidades, SCCs e pontuações; a multiplicidade de arestas muda |
| `isolado` | Raios e pontuações anteriores não mudam; não exige mesmas posições de ranking |
| `adicao_preserva_alcance` | Adicionar uma Uses não remove destinos de montante/jusante |
| `adicao_nao_aumenta_distancias` | Distâncias já existentes continuam definidas e não aumentam |
| `remocao_nao_cria_caminhos` | Após remover Uses, alcance novo é subconjunto do original |
| `inversao` | Inverter Uses troca montante/jusante, graus e profundidades; Owns é mantido |
| `owns_nao_propaga` | Acrescentar Owns mantém as métricas de Uses; contexto pai/filhos pode mudar |

Antes e depois de **cada** transformação, os resultados reais de raio, SCCs e
ranking são confrontados com o oráculo, além da comparação da propriedade.
As comparações normalizam por ID e usam mapas/conjuntos ordenados, nunca ordem
incidental de HashMap. Ranking é sempre comparado por pontuação/classificação
do nó correspondente, e não pela posição que o candidato ocupa.

### Corpus e limites do ensaio

- Gerador **v1**, SplitMix64 com operações wrapping em u64 e constantes fixas;
  teste com vetor conhecido protege a sequência do PRNG.
- **64 sementes decimais** (0–63) explicitamente registradas em `sementes.json`.
- De 0 a 8 nós de origem; no máximo 9 após a inserção de isolado. Quatro famílias:
  esparsa, densa, duas partições sem Uses entre elas, e ciclo com autoarestas e
  um nó isolado em Uses. Subtipos Import/Reference são variados.
- Paths/IDs únicos e endpoints coerentes. Owns forma floresta com no máximo um
  pai por filho; isso exclui a ambiguidade de qual pai escolher por ordem.
- Autoarestas contam como vizinho direto, mas o alvo não aparece no próprio
  alcance. SCCs unitários não são retornados, conforme o contrato vigente.
- Sem nós/arestas elegíveis, algumas transformações são identidade. Um teste
  de cobertura garante que cada propriedade também tem transformações efetivas,
  e que o corpus possui ampliação/redução de alcance e redução de distâncias.
- Não há testes baseados em tempo. Esses grafos pequenos não são benchmark nem
  evidência sobre desempenho/stack em grafos grandes.

### Executar e reproduzir

Todos os testes, preservando os ensaios antigos:

```sh
cargo test --offline --locked --manifest-path lab/conformidade-grafos/Cargo.toml
```

Apenas uma propriedade, com uma semente exata:

```sh
LENTE_SEMENTE=42 cargo test --offline --locked \
  --manifest-path lab/conformidade-grafos/Cargo.toml \
  propriedades::renomeacao_bijetiva -- --exact --nocapture
```

`LENTE_SEMENTE` aceita um u64 decimal e substitui o corpus apenas nos nove testes
de propriedades. Para rodar todos eles sobre essa semente, use o filtro
`propriedades::` sem `--exact`; os três testes auxiliares também executarão.
O teste auxiliar de cobertura sempre verifica o corpus padrão inteiro.
Nenhuma variável de ambiente é modificada pelos testes.

Em falha, o relatório mostra **semente, propriedade, grafo original JSON,
grafo transformado JSON, mapa inverso e divergência esperado/obtido**, mais o
comando exato de repetição. Panics capturáveis das APIs de produção recebem o
mesmo contexto. É possível salvar os dois JSONs do relatório e usar a CLI
analítica já existente. Repita com a mesma versão do gerador e dos crates de
produção; uma semente sozinha não congela implementações futuras.

Resultados desta execução: [RESULTADOS-PROPRIEDADES.md](RESULTADOS-PROPRIEDADES.md).
Esses testes avaliam **algoritmos sobre grafos admissíveis**; não demonstram
completude semântica dos extratores de qualquer linguagem.
