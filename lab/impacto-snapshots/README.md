# Impacto estrutural de dois snapshots

Biblioteca experimental pura em workspace Cargo independente. Recebe grafos,
correspondências e mudanças já conhecidas; não lê Git, arquivos ou configuração,
não extrai código e não executa processos. Dependências normais: **somente
`lente_core`**. Exemplo executável em memória separado da biblioteca.

Contrato normativo: [CONTRATO.md](CONTRATO.md). Implementação e testes inline:
[src/lib.rs](src/lib.rs). Não altera núcleo, adaptadores, workspace principal,
CLI, MCP ou renderizadores.

## Resultados anteriores consultados

- [Comparação por identidade explícita — tarefa 17](../../00_nucleo/lessons/2026-09-26-identidades-opacas-comparacao-01a0e01d45e4.md):
  identidade opaca, pares 1:1, preservação de resíduos, sem inferir pelo path.
  A API trabalha sobre estruturas de módulos; este experimento precisa comparar
  consumidores de alvos nos grafos originais, inclusive elementos removidos.
- [Mapeamento por inventário — tarefa 18](../../00_nucleo/lessons/2026-09-26-mapeamento-inventario-01a0e01d-6243.md):
  cobertura explícita, validação integral e remoções ainda fora do escopo.
  O laudo registrou uma falha concorrente em wiring/cache; esta biblioteca não
  usa esse fluxo e não depende da aprovação da suíte principal para seu cálculo.
- Contratos de [raio](../../00_nucleo/prompts/raio.md),
  [comparação](../../00_nucleo/prompts/comparacao.md),
  [mapeamento](../../00_nucleo/prompts/mapeamento.md) e
  [mapa declarativo](../../00_nucleo/prompts/infra-mapa-correspondencia.md).

## Executar e testar

Requer Rust ≥1.91. Da raiz do repositório, sem downloads:

```sh
cargo run --offline --locked --manifest-path lab/impacto-snapshots/Cargo.toml \
  --example comparar
cargo test --offline --locked --manifest-path lab/impacto-snapshots/Cargo.toml
cargo clippy --offline --locked --manifest-path lab/impacto-snapshots/Cargo.toml \
  --all-targets --no-deps -- -D warnings
cargo fmt --manifest-path lab/impacto-snapshots/Cargo.toml -- --check
cargo tree --offline --locked --manifest-path lab/impacto-snapshots/Cargo.toml \
  --edges normal
```

O [exemplo](examples/comparar.rs) constrói dois grafos em memória. Demonstra:
remoção com consumidor antigo, adição com consumidor novo, renomeação declarada
com relações preservadas e reutilização do ID local 10 por elementos diferentes.
Apenas imprime dados; não faz checkout ou leitura de revisões. Não é uma nova CLI
do produto nem um leitor de snapshots serializados.

## API

```rust,ignore
let resultado = comparar_impacto(
    &snapshot_antes,
    &snapshot_depois,
    &[
        Correspondencia {
            identidade: "servico-estavel".into(),
            antes: 20,
            depois: 200,
        },
        // Incluir os consumidores/dependências que têm correspondência conhecida.
    ],
    &[
        Mudanca::Remocao { antes: 10 },
        Mudanca::Adicao { depois: 10 }, // mesmo número não significa mesmo elemento
        Mudanca::Pareada {
            identidade: "servico-estavel".into(),
            alteracao_declarada: false,
            renomeacao_declarada: true,
        },
    ],
)?;
```

O exemplo completo compilável está em `examples/comparar.rs`. `Snapshot` contém
`Grafo`, versão declarada, completude, granularidade, natureza e diagnósticos.
Os dois snapshots precisam declarar granularidade/natureza compatíveis. Os
rótulos de versão são preservados; não são revisions ou hashes verificados.
Mesmo rótulo em ambos os snapshots é permitido: `Lado` continua distinguindo-os.

Correspondências referem-se a IDs **locais** de cada lado; a string `identidade`
é a chave de comparação entre versões. Não há normalização de caixa/Unicode,
prefixos ou paths; a chave vazia é válida, conforme a tarefa 17. Não há fallback
por nome, path, ID igual ou semelhança. Splits/merges N:M não são suportados.

O componente valida todo o mapa e os grafos antes do cálculo, incluindo entradas
não tocadas. Correspondências duplicadas, identidades/endpoints reutilizados,
IDs ausentes/duplicados e endpoints textuais incoerentes geram `Vec<Erro>`
ordenado. Nenhum resultado parcial esconde mapa inválido. Mudanças duplicadas
idênticas são idempotentes; declarações conflitantes sobre o mesmo par são erro.

Uma remoção/adição não pode usar um endpoint que esteja pareado. Para alterações
de um par, use `Pareada`; renomeação e alteração são flags independentes, pelo
menos uma verdadeira. Não se descobre automaticamente mudança nem renomeação.
Nós sem mapa permanecem em `sem_par_antes`/`sem_par_depois`, inclusive isolados.

## Semântica do impacto

| Mudança | Raio anterior | Raio posterior |
|---|---|---|
| Remoção | calculado no grafo anterior | `None` |
| Adição | `None` | calculado no grafo posterior |
| Pareada | calculado no anterior | calculado no posterior |

`None` significa não aplicável, nunca "raio vazio confirmado". Cada cálculo
reutiliza `calcular_raio`; os grafos nunca são unidos para essa finalidade.
`ImpactoSnapshot` preserva alvo, classificação, graus diretos, consumidores
(montante), dependências (jusante) e menores profundidades. Cada elemento possui
lado, versão, completude, ID local, path original e identidade opcional.

Paths podem colidir na forma crua: o experimento cria uma projeção um-para-um
local, com paths temporários únicos por ID, e a entrega ao algoritmo existente.
Isso não pareia IDs entre versões nem altera o grafo original. `Owns` não propaga
impacto; contexto de contenção não é retornado como dependência.

`consumidores` divide as observações em:

- `somente_antes` e `somente_depois`: consumidores observados em apenas um raio,
  com contraparte quando declarada e explicação do que se sabe do outro lado.
- `ambos`: consumidores com identidade explicitamente correspondente e alcance
  em ambos os lados; profundidades anteriores/posteriores são preservadas.
- `comparacao_bilateral`: falso para adição/remoção, verdadeiro para pares.

**Essas categorias descrevem o impacto, não a existência dos nós nos snapshots.**
Um consumidor pode existir nos dois grafos e ser alcançado em apenas um. Sem mapa,
dois nós de ID/path igual continuam sendo dois resíduos diferentes.

Cada unilateral recebe `EstadoOutroLado`:

| Estado | Significado |
|---|---|
| SemCorrespondencia | Não há identidade para comparar; não se infere desaparecimento. |
| AlvoNaoComparavel | Consumidor pareado, mas adição/remoção não tem alvo nos dois lados. |
| NaoAlcancadoNoGrafoCompleto | Contraparte existe; não foi alcançada no grafo completo fornecido. |
| NaoObservadoComCoberturaLimitada | Snapshot parcial/desconhecido: não observação não prova ausência. |

`Completa` significa somente completude declarada da extração no escopo informado,
não completude semântica. Evidências positivas em snapshots parciais continuam
preservadas; diagnósticos originais também. A ausência de consumidores em um
snapshot parcial não vira prova de segurança ou irrelevância da mudança.

## Renomeação e relações observadas

Para pares, `topologia_incidente` compara relações `Uses` **diretas incidentes**
ao alvo, em ambos os sentidos, por identidade dos endpoints e subtipo. Paralelas
idênticas são deduplicadas; import/reference/ausente continuam distintos. Retorna
relações comuns com originais dos dois lados e relações observadas apenas de
cada lado. A renomeação declarada fica em `mudanca`, independentemente desse delta.

`MesmasRelacoesObservadas` não afirma equivalência global ou comportamental.
`DiferencasNasRelacoesObservadas` indica mudança nos registros comparáveis.
`ComparacaoLimitada` sinaliza snapshot incompleto ou endpoints sem par; as listas
observadas continuam disponíveis, mas não atestam adição/remoção semântica.

Mudanças transitivas podem alterar consumidores sem mudar relações incidentes;
por isso o componente entrega ambos os resultados. Alteração de subtipo pode
mudar relações incidentes sem mudar o conjunto alcançado. Nenhum desses casos é
tratado como prova de quebra ou de preservação funcional.

## Verificação e limites restantes

Em 2026-09-26, Linux/Rust 1.92: **22 testes passaram**, Clippy sem avisos,
formatação conferida e exemplo executado. `cargo tree --edges normal` mostra
apenas o experimento e `lente_core`: nenhuma dependência L3 ou externa.

Testes cobrem os nove casos solicitados, além de paths colidentes, consumidores
transitivos, mudança de profundidade, subtipos, ciclos/paralelas/Owns, declaração
de renomeação junto com alteração, identidade opaca vazia, mapa inválido fora das
mudanças, erros determinísticos, conjunto vazio de mudanças e caminho que só
existiria numa união indevida dos snapshots. Testes inline, dados em memória.

Não há inferência de changeset, normalização de caminhos, leitura Git, integração
com o inventário, cache persistente, otimização multi-alvo ou verificação das
alegações do chamador. O raio é recalculado por alvo e lado. A granularidade e as
limitações do extrator continuam valendo: import não vira chamada nem prova de
uso em runtime. Este experimento não integra leitores ou interfaces do produto.
