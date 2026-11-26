# Prompt de Nucleação: `lente_wiring` — a fiação (L4)
Hash do Código: 247e8138

**Camada**: L4 — Fiação (composição). Importa L1/L2/L3 (é o topo, compõe tudo).
**Unidade**: `04_wiring/src/lib.rs` (crate `lente_wiring`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0019-l4-wiring.md` (+ 0027/
0030/0031/0034/0045/0047 — os pipelines).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **composição** dos pipelines da lente, ponta a ponta: extrai/recebe o grafo,
resolve colisões, aplica o escopo, e responde cada modo (per-nó / ranking /
estrutura / diff / grafo de workspace). **Não** formata nem lê argumentos (isso é
L2); **não** define vocabulário de pedido (desceu ao L1 no Estágio 2) — só **fia**
as camadas abaixo.

## Comportamento e invariantes

- ADR-0007: obtenção unitária e extração Rust por membro/diretório atravessam a fronteira de provedores; analise oferece preparação injetável e consultas sobre o mesmo grafo. Canonicalização delegada ao L3. ErroLente::Provedor preserva contexto e causa.

- `montar_grafo_workspace` etiqueta cada `GrafoCrate` com `grafo.crate_name`
  e devolve `crates_por_pacote: BTreeMap<String, String>` (pacote Cargo → crate
  canônico) dos grafos incluídos. O pacote permanece na seleção Cargo e em
  `FalhaCrate.crate_name` (nome histórico do campo de diagnóstico).
- A comparação reutiliza esse mapa no filtro `seu-codigo`, sem enumerar membros
  novamente. O censo conta sucessos + falhas, incluindo os pacotes que falharam.
- **Limitação**: sem extração/resolução válida, o pacote fica em `falhas`, fora
  do mapa e da união. Não se presume identidade normalizando seu nome. Referências
  a esse crate podem sair no filtro `seu-codigo` e não são reconhecidas como
  fantasmas de membro; os diagnósticos declaram a análise parcial.

- `FalhaCrate` é reexportado de L1 (preserva o caminho público `lente_wiring::FalhaCrate`). `analisar_diff` propaga integralmente `GrafoWorkspace.falhas` até `ResultadoDiff`, junto aos raios, censo e fantasmas disponíveis.
- A finalização da montagem exige ao menos um grafo extraído/resolvido, independentemente da quantidade de nós. Zero sucessos (inclusive zero membros) retorna `ErroLente::NenhumCrateAnalisado(Vec<FalhaCrate>)`, cujo Display preserva nomes e motivos. A regra vale para todos os consumidores da montagem do workspace.
- Testes sem fork: todos extraem → completa; um falha → parcial preservando raio e motivo; todos falham → erro com todos os motivos.

- **Pipelines** (`pub fn`):
  - `calcular_raio_de_alvo(fonte, alvo, escopo) -> Result<Raio, ErroLente>`.
  - `rankear_pacote(fonte, n, escopo) -> Result<Vec<ItemRanking>, _>`.
  - `analisar_estrutura(fonte, escopo, modo_uses) -> Result<EstruturaModulos, _>`.
  - `analisar_estrutura_com_evidencias` fornece a mesma estrutura mais a
    agregação detalhada do L1 para a vista HTML. Ambas vêm da mesma extração,
    após aplicar o escopo e o modo de uses.
  - `analisar_estrutura_indexada` usa o mesmo cálculo sobre um grafo de
    arquivos já resolvido, projetado a partir de um índice de referências.
  - `montar_grafo_workspace(raiz) -> Result<GrafoWorkspace, _>` (enumera→extrai
    cacheado→resolve por crate→une). Compartilha uma `CacheExtracao` por rodada;
    contexto Rust continua consultado por membro. Bypass do L3 é respeitado. Antes de cada extração, consulta
    `versao_toolchain(&membro.dir)` e passa a versão à chave do cache, sem
    reutilizar a versão da raiz ou de outro membro. Falhas de descoberta
    continuam propagadas como `ErroLente::Workspace`. A comparação aplica
    esse mesmo pipeline independentemente a cada lado que seja workspace;
    lados que sejam crates mantêm a extração direta, sem cache.
  - `analisar_diff(raiz) -> Result<ResultadoDiff, _>` (canonicaliza a raiz; ler_diff
    + grafo de workspace + mapear_diff + raio por tocado).
- **`GrafoWorkspace`** (`pub struct`: `grafo`, `fantasmas`, `falhas`,
  `crates_por_pacote`).
- **`ErroLente`** (`pub enum`) — o **erro agregado da composição** (ver abaixo).
- **Re-exports**: o vocabulário L1 (`Escopo`/`ModoUses`/`FonteGrafo`/`AlvoBusca` de
  `consulta`; `ResultadoDiff`/`TocadoComRaio`/`RaioCombinado`/`combinar_raios`;
  `Fantasma`; `Ciclo`/`OrdemDsm`/`DependenciaModulo`/`EstruturaModulos`;
  `ItemRanking`) — para os consumidores do fio.

## `ErroLente` — residência no L4 por desígnio (V12 = 1 intencional)

`ErroLente` **agrega** os erros das camadas internas via `From`: `Fork`/
`Adaptador`/`Workspace`/`Diff` (do **L3**) e `Resolucao`/`Raio` (do L1), mais
`IdInexistente`/`ForkSemUsesKind`. **É um erro de composição** — só na fiação, onde
L1 e L3 se encontram, faz sentido juntá-los. **Não desce ao L1** (faria o L1
referenciar o L3). Por isso o **V12 = 1 sobre o `ErroLente` é intencional e
aceito** — um `enum` no L4 que é legítimo (não é "L4 criando tipo de domínio";
é o tipo-soma dos erros que a composição propaga com `?`).

`Error::source()` devolve a falha encapsulada em `Fork`, `Adaptador`,
`Resolucao`, `Raio`, `Workspace` e `Diff`. `IdInexistente`,
`ForkSemUsesKind` e `NenhumCrateAnalisado` não têm uma causa encadeada
individual. `Display` preserva as mensagens e o resumo das falhas por crate.

## Restrições (L4)

- O **topo**: importa L1/L2/L3 (depende de tudo abaixo). Não há camada acima.
  V3 = 0 por construção.

## Critérios de Verificação

```
Dado um JSON com colisão Quando calcular_raio_de_alvo Então o raio do alvo resolvido
Dado a raiz do repo Quando montar_grafo_workspace Então o grafo unificado + fantasmas
Dado um ErroFork Quando ? num pipeline Então ErroLente::Fork (agregação via From)
Dada cada variante que encapsula falha Quando consultar Error::source Então obter a causa com seu tipo original
Dada uma variante sem erro encapsulado Quando consultar Error::source Então obter None
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"calcular_raio_de_alvo","params":["FonteGrafo","AlvoBusca","Escopo"],"return_type":"Result<Raio, ErroLente>"},{"name":"montar_grafo_workspace","params":["&std::path::Path"],"return_type":"Result<GrafoWorkspace, ErroLente>"},{"name":"analisar_diff","params":["&std::path::Path"],"return_type":"Result<ResultadoDiff, ErroLente>"},{"name":"rankear_pacote","params":["FonteGrafo","usize","Escopo"],"return_type":"Result<Vec<ItemRanking>, ErroLente>"},{"name":"analisar_estrutura","params":["FonteGrafo","Escopo","ModoUses"],"return_type":"Result<EstruturaModulos, ErroLente>"},{"name":"analisar_estrutura_com_evidencias","params":["FonteGrafo","Escopo","ModoUses"],"return_type":"Result<(EstruturaModulos, AgregacaoDetalhada), ErroLente>"},{"name":"analisar_estrutura_indexada","params":["&Grafo"],"return_type":"EstruturaModulos"},{"name":"comparar","params":["&std::path::Path","&std::path::Path","Escopo","ModoUses","Option<&MapaCorrespondencia>"],"return_type":"Result<Comparacao, ErroComparar>"}],"types":[{"name":"ErroLente","kind":"enum","members":["Fork","Adaptador","Resolucao","Raio","IdInexistente","ForkSemUsesKind","Workspace","Diff","NenhumCrateAnalisado"]},{"name":"GrafoWorkspace","kind":"struct","members":["grafo","fantasmas","falhas","crates_por_pacote"]},{"name":"ErroComparar","kind":"struct","members":["lado","erro"]}],"reexports":["lente_infra::analises","lente_core::domain::consulta::{AlvoBusca, Escopo, FonteGrafo, ModoUses}","lente_core::domain::resultado_diff::{\n    CompletudeDiff, FalhaCrate, RaioCombinado, ResultadoDiff, TocadoComRaio, combinar_raios,\n}","lente_core::domain::uniao::Fantasma","lente_estrutura::{Ciclo, DependenciaModulo, EstruturaModulos, OrdemDsm}","lente_ranking::ItemRanking","lente_comparacao::{\n    AlegacaoDeclarada, ArestaComparada, Comparacao, ComparacaoItens, CorrespondenciaAplicada,\n    ItemAmbiguo, ItemPareado, ItemSemPar, Lado, RelacaoDeclarada, ResultadoMapa, ResumoCiclos,\n}"]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-11-26 | Expor as causas dos erros encapsulados por `ErroLente`. | `04_wiring/src/lib.rs` |
| 2026-09-26 | Distinguir pacote Cargo de crate canônico da extração; contrato de identidade e limites de membros com falha. | `04_wiring/src/lib.rs` |
| 2026-09-26 | Toolchain consultado por membro em cada raiz; teste de propagação até o cache. | `04_wiring/src/lib.rs` |
| 2026-08-26 | P0079: `comparar` recebe mapa declarativo opcional já lido pelo L3 e o injeta no pareamento puro; ausência preserva o pipeline anterior. | `04_wiring/src/lib.rs` |
| 2026-06-07 | Nucleação (migração de convenção, prompt 0065) da fiação. `ErroLente` documentado como erro de composição L4 (V12=1 intencional). Código inalterado. | `04_wiring/src/lib.rs` |

| 2026-09-26 | Correção da perda de falhas no diff: completude explícita, resultados parciais consultáveis e erro se nenhum crate for analisado. | Unidade deste contrato |

## Integração aditiva de provedores (2026-09-26)

Expõe módulo provedores: composição aditiva de ExtratorGrafo com análise de grafos externos, sem resolução/filtro Rust. Contrato próprio em wiring-provedores.md. Fluxos legados mantidos.

Instalação, protocolo e matriz verificada: `docs/adaptadores.md`. ADR-0008.
