# Prompt L0: `LayerConfig` + `detect_layer_violations` (L₁)

**Camada**: L₁ (Núcleo)
**Ficheiros alvo**:
  - `01_core/src/entities/layer_config.rs` (novo)
  - `01_core/src/rules/layer_violation_detector.rs` (novo)
**Passo do roadmap**: 2.3 — Integração com `crystalline.toml`
**Status**: IMPLEMENTADO

---

## Decisões de design prévias

- **Passo 2.3, decisão de escopo**: o `crystalline-dsm` detecta
  apenas **violações de direção topológica entre camadas** por
  conta própria. Outras regras (V9 PubLeak, V11 DanglingContract,
  etc) vêm do SARIF do `crystalline-lint`, não são reimplementadas
  aqui. Razão: uma fonte de verdade. O linter é o juiz; o DSM é o
  visualizador.

- **Atribuição de nó a camada**: por `crate_name`, via tabela
  `crate_name → camada` derivada do cruzamento entre `crate_root`
  (do `cargo_metadata`) e a seção `[layers]` do `crystalline.toml`.
  A derivação dessa tabela acontece em L₃ (leitor); L₁ recebe a
  tabela pronta dentro do `LayerConfig`.

---

## Decisões locais (assumidas neste prompt)

1. **`LayerConfig` é entidade pura de L₁**: não lê ficheiro,
   não conhece TOML. Recebe os dados já parseados (de L₃).

2. **Topologia cristalina embutida como regra**: as regras de
   direção (L1 só importa L1; L2 importa L1,L2; L3 importa L1,L3;
   L4 importa tudo) são conhecimento de domínio, codificadas em
   L₁.

3. **Nós sem camada não geram violação**: nós externos (crates.io,
   stdlib) não têm camada. Arestas de/para eles são ignoradas na
   detecção de violação topológica.

4. **`lab` é caso especial**: a topologia cristalina diz que
   produção (L1-L4) não pode importar de `lab`. Modelado.

---

## Contexto

O `crystalline.toml` mapeia camadas a diretórios:

```toml
[layers]
L0  = "00_nucleo"
L1  = "01_core"
L2  = "02_shell"
L3  = "03_infra"
L4  = "04_wiring"
lab = "lab"
```

A topologia cristalina define quais camadas podem depender de
quais:

| Camada | Pode importar de |
|--------|------------------|
| L1 | L1 |
| L2 | L1, L2 |
| L3 | L1, L3 |
| L4 | L1, L2, L3, L4 |
| lab | qualquer |

E uma regra adicional: nenhuma camada de produção (L1-L4) pode
importar de `lab`.

Este prompt define:
- A entidade `LayerConfig` que carrega o mapa `crate_name → Layer`.
- A regra `detect_layer_violations` que, dado um grafo + um
  `LayerConfig`, retorna as arestas que violam a direção
  topológica.

---

## Entidade `Layer`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    L0,
    L1,
    L2,
    L3,
    L4,
    Lab,
}

impl Layer {
    /// Retorna `true` se um nó nesta camada pode depender de um
    /// nó na camada `target`, segundo a topologia cristalina.
    pub fn can_depend_on(self, target: Layer) -> bool {
        use Layer::*;
        match self {
            // L0 é prompts/ADRs; não tem código que importa.
            // Tratado como não-restritivo aqui (não deveria
            // aparecer no grafo de imports Rust).
            L0 => true,
            L1 => matches!(target, L1),
            L2 => matches!(target, L1 | L2),
            L3 => matches!(target, L1 | L3),
            L4 => matches!(target, L1 | L2 | L3 | L4),
            // lab pode importar qualquer coisa.
            Lab => true,
        }
    }

    /// Parse a partir do nome usado no `[layers]` do toml.
    /// "L0".."L4" e "lab" (case-insensitive para "lab").
    pub fn from_config_key(key: &str) -> Option<Layer> {
        match key {
            "L0" => Some(Layer::L0),
            "L1" => Some(Layer::L1),
            "L2" => Some(Layer::L2),
            "L3" => Some(Layer::L3),
            "L4" => Some(Layer::L4),
            "lab" | "Lab" | "LAB" => Some(Layer::Lab),
            _ => None,
        }
    }

    /// Nome canónico para exibição.
    pub fn as_str(self) -> &'static str {
        match self {
            Layer::L0 => "L0",
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
            Layer::L4 => "L4",
            Layer::Lab => "lab",
        }
    }
}
```

Nota sobre a regra `lab`: a tabela `can_depend_on` diz que `Lab`
pode depender de tudo. A regra "produção não pode importar lab" é
o **inverso**: qualquer camada L1-L4 importando de `Lab` é
violação. Isso é capturado porque `can_depend_on` para L1-L4 não
inclui `Lab` no conjunto permitido. Portanto, L2→Lab retorna
`false` (violação), enquanto Lab→L2 retorna `true` (permitido).
Coerente com a topologia.

---

## Entidade `LayerConfig`

```rust
pub struct LayerConfig {
    /// Mapa de crate_name (formato canónico, ex:
    /// "crystalline_dsm_core") para a camada.
    /// Construído em L₃ cruzando crate_root com [layers].
    crate_to_layer: HashMap<String, Layer>,
}

impl LayerConfig {
    /// Constrói a partir de um mapa pré-computado.
    pub fn new(crate_to_layer: HashMap<String, Layer>) -> Self;

    /// Retorna a camada de um crate, se conhecida.
    /// `None` para crates não mapeados (ex: externos, ou crates
    /// fora da topologia cristalina).
    pub fn layer_of_crate(&self, crate_name: &str) -> Option<Layer>;

    /// Quantidade de crates mapeados.
    pub fn len(&self) -> usize;

    /// `true` se vazio.
    pub fn is_empty(&self) -> bool;
}
```

Notas:
- O `crate_name` aqui é o formato que aparece no `canonical_path`
  dos nós do grafo. Atenção: pode ser o nome com underscores
  (`crystalline_dsm_core`) ou hífens (`crystalline-dsm-core`)
  dependendo de como o grafo guarda. O leitor L₃ deve normalizar
  para o mesmo formato que o grafo usa. Documentar a convenção.

---

## Entidade `LayerViolation`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerViolation {
    /// Nó de origem (o que importa).
    pub from_node: GraphNodeId,

    /// Nó de destino (o que é importado).
    pub to_node: GraphNodeId,

    /// Camada do nó de origem.
    pub from_layer: Layer,

    /// Camada do nó de destino.
    pub to_layer: Layer,
}
```

---

## Regra `detect_layer_violations`

Ficheiro: `01_core/src/rules/layer_violation_detector.rs`.

```rust
pub fn detect_layer_violations(
    graph: &DependencyGraph,
    config: &LayerConfig,
) -> Vec<LayerViolation>;
```

### Comportamento

Para cada aresta `(from, to)` no grafo:

1. Obter `from_crate` e `to_crate` (os `crate_name` dos nós).
   - Se qualquer um for externo (sem `crate_name`), pular (não há
     camada para externos).

2. Obter `from_layer = config.layer_of_crate(from_crate)` e
   `to_layer = config.layer_of_crate(to_crate)`.
   - Se qualquer um for `None` (crate não mapeado), pular.

3. Se `!from_layer.can_depend_on(to_layer)`: é violação.
   Adicionar `LayerViolation` ao resultado.

4. Deduplicação: se múltiplas arestas entre o mesmo par
   `(from, to)` violam, registrar apenas uma `LayerViolation`
   (a violação é da relação, não de cada import individual).

### Ordem do resultado

Determinística: ordenar por `(from_canonical_path,
to_canonical_path)` lexicográfico.

---

## Função auxiliar de consumo

```rust
impl LayerViolation {
    /// Descrição textual da violação, para tooltips/relatórios.
    /// Ex: "L1 → L3 (forbidden)".
    pub fn describe(&self) -> String {
        format!("{} → {} (forbidden)",
                self.from_layer.as_str(),
                self.to_layer.as_str())
    }
}
```

---

## Derives obrigatórios

- `Debug`, `Clone`, `PartialEq`, `Eq` em todas as structs.
- `Hash` em `Layer` (usado como chave/valor em mapas).
- `LayerConfig` NÃO precisa de `Hash`.

Sem `Serialize`/`Deserialize` (ADR-0009: L₁ não conhece serde).

---

## Dependências externas

`01_core/Cargo.toml`: nenhuma nova. Apenas `std` e `petgraph`
(já presentes).

---

## Testes esperados

### `layer_config.rs` (testes inline)

1. **`Layer::can_depend_on` — L1**: L1→L1 `true`; L1→L2, L1→L3,
   L1→L4, L1→Lab todos `false`.

2. **`can_depend_on` — L2**: L2→L1 `true`, L2→L2 `true`; L2→L3,
   L2→L4, L2→Lab `false`.

3. **`can_depend_on` — L3**: L3→L1 `true`, L3→L3 `true`; L3→L2,
   L3→L4, L3→Lab `false`.

4. **`can_depend_on` — L4**: L4→L1, L4→L2, L4→L3, L4→L4 todos
   `true`; L4→Lab `false`.

5. **`can_depend_on` — Lab**: Lab→qualquer `true`.

6. **`from_config_key`**: "L0".."L4" e "lab" parseiam; "L5",
   "xyz" retornam `None`.

7. **`LayerConfig::layer_of_crate`**: crate mapeado retorna
   `Some(layer)`; crate não mapeado retorna `None`.

### `layer_violation_detector.rs` (testes inline)

8. **Sem violações (DAG limpo)**: L2→L1, L3→L1, L4→L3. Nenhuma
   viola. Resultado vazio.

9. **Violação L1→L3**: criar grafo onde um nó de crate-L1
   importa de crate-L3. Detectar 1 violação com `from_layer ==
   L1`, `to_layer == L3`.

10. **Violação L2→L3**: análogo.

11. **Violação produção→lab**: nó L2 importa de crate-lab.
    Detectar violação (`to_layer == Lab`).

12. **lab→produção é permitido**: nó lab importa de crate-L1.
    Sem violação.

13. **Externos ignorados**: nó L1 importa de `serde` (externo).
    Sem violação (externo não tem camada).

14. **Crate não mapeado ignorado**: nó de crate não presente no
    `LayerConfig`. Aresta envolvendo-o é pulada.

15. **Deduplicação**: múltiplas arestas L1→L3 entre o mesmo par
    de nós geram 1 só `LayerViolation`.

16. **Ordem determinística**: resultado ordenado por
    `(from_path, to_path)`.

17. **`LayerViolation::describe`**: formato "L1 → L3
    (forbidden)".

---

## Critério de aceitação do prompt

- `01_core/src/entities/layer_config.rs` existe e compila.
- `01_core/src/rules/layer_violation_detector.rs` existe e
  compila.
- `Layer`, `LayerConfig`, `LayerViolation` conforme especificado.
- `detect_layer_violations` com a assinatura especificada.
- Os 17 testes passam.
- `cargo clippy -p crystalline-dsm-core` sem warnings.
- Módulos exportados em `entities/mod.rs` e `rules/mod.rs`.
- Nenhum tipo `petgraph` exposto na API pública.

---

## Próximos passos (fora deste prompt)

1. **L₃ — leitor de `crystalline.toml`**: parseia `[layers]`,
   cruza com `crate_root` do `cargo_metadata`, constrói o
   `LayerConfig`. Prompt separado.

2. **L₃ — leitor de SARIF**: parseia o output SARIF do
   `crystalline-lint`, mapeia `results` a nós do grafo via
   `canonical_path`/`source_file`. Prompt separado.

3. **L₃ — renderizador HTML estendido**: destaca violações de
   camada (deste detector) e violações do SARIF em cores
   distintas. Prompt separado.

---

## Limitações conhecidas

1. **Granularidade de crate, não de módulo**: a atribuição de
   camada é por `crate_name`. Se um único crate contiver módulos
   de camadas diferentes (anti-padrão na arquitetura cristalina,
   mas possível), a detecção não distingue. A arquitetura
   cristalina assume um crate por camada (ou camadas mapeadas a
   diretórios de topo), então esta limitação raramente importa
   na prática.

2. **L0 tratado como não-restritivo**: L0 é prompts/ADRs, sem
   código Rust que importa. Se por acaso aparecer um nó L0 no
   grafo, ele não gera violações. Assumido como caso que não
   ocorre.

---

## Hash do prompt

A calcular após aprovação.
