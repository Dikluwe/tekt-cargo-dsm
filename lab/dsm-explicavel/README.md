# DSM explicável — laboratório independente

Vista somente leitura sobre um snapshot já extraído. Uma célula abre as relações
que sustentam seu peso; cada relação pode iniciar uma consulta de caminho entre
itens. Não integra o `--html`, CLI ou MCP principais e não extrai/importa/executa
o projeto analisado.

## Executar

Requer Python 3.10+ e a toolchain Rust do repositório. As dependências Rust precisam
estar disponíveis localmente; nenhum comando instala ferramentas automaticamente.
Execute a partir deste diretório:

```sh
cargo build --offline
./target/debug/dsm-explicavel prepare fixtures/partial.json > artifacts/partial.bundle.json
python3 serve.py artifacts/partial.bundle.json --port 8765
```

Abra **http://127.0.0.1:8765**. O servidor escuta somente loopback. Para encerrar,
use Ctrl+C no terminal. `--binary` permite apontar para outro build do mesmo
preparador. Os arquivos CSS/JS são locais, sem CDN, fontes externas ou telemetria.
O HTML exige esse servidor; abrir `web/index.html` como `file://` não basta.

Para preparar outro snapshot, forneça o envelope de entrada descrito abaixo.
Não misture configurações: cada pacote e sessão representam uma configuração.
A consulta também pode ser executada sem navegador:

```sh
./target/debug/dsm-explicavel query artifacts/partial.bundle.json 11 31
```

Resultado esperado: consumidor `app::ui::render <&>` → `app::service::load` →
item alterado `app::storage::save`, dois saltos. Inverter as pontas não encontra
caminho nessa fixture.

## APIs conferidas e reutilizadas (tarefas 14 e 20)

Foram lidos os contratos `00_nucleo/prompts/raio.md` e
`00_nucleo/prompts/estrutura.md` e suas implementações reais:

- `lente_estrutura::agregar_por_modulo_detalhado(&Grafo) -> AgregacaoDetalhada`:
  fornece células, pesos, índices de evidência, classificação de cada registro e
  itens sem contenedor. Detecta ciclos/múltiplos pais Owns; não fornece outro
  Grafo nem posições de referência. O preparador somente traduz essa saída para
  o contrato local e projeta as células em arestas Uses agregadas.
- `lente_core::domain::raio::explicar_caminho_impacto(grafo, alterado, consumidor)`:
  fornece um único caminho mínimo Uses, em direção consumidor → alterado, ou
  ausência de caminho/erro. O preparador **não replica BFS, raio ou agregação**.
- `lente_infra::desserializar_grafo`: traduz o JSON atual. A borda local valida
  unicidade de IDs/paths, integridade dos endpoints e limite de IDs inteiros
  exatos em JavaScript. A consulta exige a forma resolvida.

Caminhos são consultados **no snapshot original de itens**. A projeção em módulos
pode criar caminhos sem cadeia correspondente entre itens; por isso não usamos
alcance do grafo agregado como explicação de impacto. Todos os nós originais são
consultáveis, inclusive órfãos: a falha de contenção é independente de um caminho
Uses entre itens. Não há promessa de completude ou quebra comportamental.

## Contrato de entrada do preparador

JSON UTF-8, versão local 1, independente da versão do grafo principal:

```json
{
  "format_version": 1,
  "metadata": {
    "provider": "nome e versão do provedor",
    "language": "Rust",
    "granularity": "itens → módulos",
    "configuration": "debug / features=demo",
    "execution": "partial",
    "semantic_coverage": "limites de reconhecimento semântico",
    "diagnostics": ["um arquivo não foi analisado"]
  },
  "graph": {"crate": "exemplo", "nodes": [], "edges": []}
}
```

`graph` segue o JSON atual aceito pelo desserializador da Lente, incluindo os
subtipos `uses_kind`. Não alteramos esse formato. `metadata` é opcional: campos
textuais ausentes ou de outro tipo tornam-se “não informado”; execução inválida
ou ausente torna-se `unknown`. `execution` aceita `complete`, `partial`, `failed`,
`unknown`. Execução completa **não** implica cobertura semântica completa.
Configuração deve ser uma descrição textual explícita, não um conjunto de
configurações mescladas. Versão incompatível é rejeitada.

## Contrato local da vista: pacote preparado

`prepare` produz JSON com `format_version: 1`. É esse pacote que `serve.py`
recebe, sem recalcular a agregação. Não edite seus índices ou células manualmente.

| Campo | Conteúdo e invariantes |
|---|---|
| `metadata` | Proveniência/configuração, execução e cobertura semântica separadas, diagnósticos. |
| `aggregated_graph.nodes` | Módulos/crates do snapshot, `{id,path}`, ordenados por path/ID. |
| `aggregated_graph.edges` | Projeção das células `{from,to,relation:"uses"}`; Owns não são dependências da DSM. |
| `cells` | `{from,to,weight,evidence_indices}`; IDs de módulos e índices em `source_graph.edges`. |
| `weight_semantics` | Literal `snapshot_uses_records`: número de registros Uses, inclusive paralelas e todos os subtipos. |
| `relations` | Uma entrada por aresta, com `snapshot_index`, IDs/paths de origem/destino, `relation`, `subtype`, `reference_position:null` e `destination`. |
| `excluded_items` | `{id,path,problems}` para todo nó sem contenção segura. |
| `items` | `{id,path}` do grafo original; opções para consultar caminhos. |
| `limitations` | Explicações sempre disponíveis na vista. |
| `source_graph` | Snapshot original íntegro, usado pela consulta; omitido da resposta ao navegador. |

`destination.kind` é `aggregated` (IDs `from/to`), `internal` (`module`) ou
`excluded` (`reason`). Exclusão pode ser `owns_context` ou `invalid_containment`,
com `source_problems/target_problems`. Problemas são dados tipados por `code`:
`sem_modulo`, `no_ausente` + `id`, `multiplos_pais` + `item/pais`, `ciclo_owns` +
`membros`. A apresentação traduz esses códigos sem usar strings Debug de enums.

IDs e índices só valem no snapshot. Não medem locais de uso nem identidade
persistente. As APIs atuais não fornecem posição da referência: `reference_position`
é sempre null. `No.position`, quando presente na entrada, não é exibida como
local de uso nem copiada para esse campo. Falta de posição não impede investigar.

O servidor congela os bytes do pacote ao iniciar. `GET /api/data` entrega os dados
preparados sem `source_graph`. `POST /api/path`, objeto `{consumer: ID, changed: ID}`,
invoca o binário local com argumentos separados e envia o snapshot congelado por
stdin. Não usa shell. Retorno: `status:found`, `direction:consumer_to_changed`,
`hops`, `path` ou `status:no_path`. Endpoint inválido produz diagnóstico HTTP 400,
diferente de ausência de caminho. Mesmo endpoint retorna zero saltos, identificado
como consulta reflexiva. Nada é editado no grafo ou no código analisado.

## Interação, segurança e limites

- Linhas são consumidores; colunas são dependências. Uma célula vazia não comprova
  ausência no sistema, especialmente quando a análise é parcial.
- Tab entra na matriz uma vez; setas movem o foco, Home/End percorrem a linha,
  Enter/Espaço selecionam e Tab sai para evidências/controles. Foco e seleção são
  visíveis. Formulário e seções usam controles nativos com nomes acessíveis.
- Evidências são criadas no DOM só após seleção, em lotes de 20. Exclusões são
  abertas sob demanda e também paginadas. Selecionar outra célula limpa os detalhes.
- O estado parcial/desconhecido permanece no aviso fixado durante a rolagem.
  Limitações são visíveis no painel de cobertura e junto das evidências/caminhos.
- Todo texto do snapshot e dos metadados entra por `textContent` ou propriedades
  textuais DOM; nunca é interpolado em HTML. A página estática não embute JSON.
  CSP restringe recursos à própria origem e proíbe scripts inline/objetos.
- Os dados preparados são recebidos inteiros em memória. Esta versão pagina a
  **apresentação**, não o transporte. Matriz O(módulos²), sem virtualização:
  experimento para grafos pequenos/moderados, não uma solução de escala provada.
- Cada consulta chama a API Rust novamente; não há cache. Não há edição, links
  para executar fonte nem adoção por interfaces principais.

## Fixtures e testes

- `partial.json`: cadeia ui→service→storage, ciclo service↔cache, célula de peso 3,
  referências paralelas/import, relação interna, órfão com relação não agregável,
  posições de declaração sem posições de referência, estado parcial, Unicode e
  caracteres que parecem HTML nos paths/metadados.
- `empty.json`: nenhum nó e metadados ausentes; completude desconhecida.
- `pagination.json`: mesma base com 45 evidências na célula ui→service.

```sh
cargo test --offline
cargo build --offline
python3 -m unittest discover -s tests -v
```

Quatro testes Rust verificam preparação, evidências, versões/integridade,
metadados ausentes, vazio, determinismo, direção/ciclo/reflexividade e ausência
de caminho. Sete testes Python exercitam o servidor local real, contrato,
caracteres, consulta, erros, snapshot congelado e recursos locais.

Inspeção de navegador efetivamente realizada e roteiro reproduzível em
[tests/VERIFICACAO-UI.md](tests/VERIFICACAO-UI.md). Captura da vista em
[artifacts/dsm-explicavel.png](artifacts/dsm-explicavel.png).

## Painel documental opcional

A integração posterior está descrita em [CONTRATO-DOXYGEN.md](CONTRATO-DOXYGEN.md)
e no [guia operacional](../../docs/doxygen.md). As opções --graph, --inventory,
--catalog, --matches e --explorer habilitam o painel após validação dos vínculos.
Sem elas, comportamento e API anteriores permanecem. Nenhum servidor é iniciado
implicitamente pelo MCP.
