# Prompt L0: Leitores de `crystalline.toml` e SARIF (L₃)

**Camada**: L₃ (Infraestrutura)
**Ficheiros alvo**:
  - `03_infra/src/crystalline_config_reader.rs` (novo)
  - `03_infra/src/sarif_reader.rs` (novo)
**Passo do roadmap**: 2.3 — Integração com `crystalline.toml`
**Status**: IMPLEMENTADO

---

## Decisões de design prévias

- **Passo 2.3, escopo**: DSM detecta direção topológica por conta
  própria (regra em L₁); para outras regras, lê SARIF do
  `crystalline-lint`. Uma fonte de verdade.
- **Atribuição por `crate_name`**: a tabela `crate_name → Layer`
  é derivada cruzando `crate_root` (do `cargo_metadata`) com
  `[layers]` (do toml). Esta derivação acontece aqui em L₃.

---

## Decisões locais (assumidas neste prompt)

1. **Dois leitores independentes**: `crystalline_config_reader`
   (lê `[layers]` do toml) e `sarif_reader` (lê output do
   linter). Funções separadas, ficheiros separados.

2. **Ambos produzem tipos de L₁ ou tipos próprios de L₃**: o
   config reader produz `LayerConfig` (de L₁). O sarif reader
   produz `Vec<SarifFinding>` (tipo próprio de L₃, pois é dado de
   wire específico do SARIF).

3. **Tolerância a campos extra**: o `crystalline.toml` tem muitas
   seções (`[project]`, `[languages]`, `[rules]`, etc). O leitor
   só extrai `[layers]`, ignorando o resto sem erro.

4. **`serde` já disponível em L₃** (do `json_serializer`). Adicionar
   `toml` crate para parsing TOML.

---

## Parte 1: `crystalline_config_reader`

### Contexto

Lê a seção `[layers]` de um `crystalline.toml` e cruza com os
`crate_root`s do workspace (já disponíveis no `Workspace` de L₁,
campo `crate_root` de cada `WorkspaceMember`) para construir o
mapa `crate_name → Layer`.

Exemplo de cruzamento:
- `[layers]` diz `L1 = "01_core"`.
- `WorkspaceMember { name: "crystalline-dsm-core", crate_root:
  ".../01_core" }`.
- Logo: `crystalline_dsm_core → L1`.

### Função pública

```rust
pub fn read_layer_config(
    toml_path: &Path,
    workspace: &Workspace,
) -> Result<LayerConfig, ConfigReadError>;
```

### Comportamento

1. Ler o ficheiro `toml_path`. Se não existir:
   `Err(ConfigReadError::FileNotFound)`.

2. Parsear como TOML. Se inválido:
   `Err(ConfigReadError::ParseFailed)`.

3. Extrair a tabela `[layers]`. Se ausente:
   `Err(ConfigReadError::NoLayersSection)`.

4. Para cada entrada `chave = "diretório"` em `[layers]`:
   - Parsear a chave para `Layer` via `Layer::from_config_key`.
   - Se chave inválida: ignorar (com possível warning) ou
     `Err`. **Decisão**: ignorar entradas não reconhecidas, para
     tolerância a chaves futuras.
   - Guardar mapa `diretório → Layer`.

5. Para cada `WorkspaceMember`:
   - Obter o último componente do `crate_root` (o nome do
     diretório, ex: `01_core`).
   - Procurar esse diretório no mapa `diretório → Layer`.
   - Se encontrado: registrar `crate_name → Layer`.
   - Normalizar `crate_name` para o formato usado no grafo
     (underscores: `crystalline_dsm_core`). O `WorkspaceMember.name`
     usa hífens; converter `-` para `_`.
   - Se não encontrado: o crate não está mapeado a nenhuma
     camada (não adicionar ao mapa). Não é erro (pode ser um
     crate fora da topologia).

6. Construir `LayerConfig::new(crate_to_layer)`.

### Casamento de diretório

O cruzamento compara o **último componente** do `crate_root` com
o valor em `[layers]`. Exemplo:

- `crate_root = "/abs/path/01_core"` → último componente
  `"01_core"`.
- `[layers]` tem `L1 = "01_core"` → match → `L1`.

Casos especiais:
- Se `[layers]` usar caminhos com subdiretórios (ex:
  `"crates/core"`), o casamento por último componente falha.
  Para o MVP, assumir que `[layers]` usa nomes de diretório de
  topo simples (como no exemplo do typst-crystalline). Documentar
  a limitação.

### Tipo de erro

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConfigReadError {
    #[error("crystalline.toml não encontrado: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Falha ao ler crystalline.toml: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("Falha ao parsear TOML: {source}")]
    ParseFailed {
        #[from]
        source: toml::de::Error,
    },

    #[error("Seção [layers] ausente no crystalline.toml")]
    NoLayersSection,
}
```

---

## Parte 2: `sarif_reader`

### Contexto

O `crystalline-lint` produz SARIF 2.1.0 com
`crystalline-lint --format sarif`. O DSM lê esse SARIF para
saber quais ficheiros têm violações detectadas pelo linter, e
cruza com o grafo para destacar os nós correspondentes.

Estrutura SARIF relevante (subset):

```json
{
  "version": "2.1.0",
  "runs": [
    {
      "tool": { "driver": { "name": "crystalline-lint" } },
      "results": [
        {
          "ruleId": "V9",
          "level": "error",
          "message": { "text": "..." },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": { "uri": "01_core/src/x.rs" },
                "region": { "startLine": 12 }
              }
            }
          ]
        }
      ]
    }
  ]
}
```

### Tipo de dado (DTO de L₃)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SarifFinding {
    /// Identificador da regra (ex: "V9").
    pub rule_id: String,

    /// Severidade ("error", "warning", "note", "none").
    pub level: String,

    /// Mensagem descritiva.
    pub message: String,

    /// Caminho do ficheiro afetado (uri do artifactLocation),
    /// como string, relativo à raiz do projeto.
    pub file_uri: String,

    /// Linha inicial, se presente.
    pub start_line: Option<u32>,
}
```

### Função pública

```rust
pub fn read_sarif(sarif_path: &Path)
    -> Result<Vec<SarifFinding>, SarifReadError>;
```

### Comportamento

1. Ler o ficheiro. Se não existe: `Err(FileNotFound)`.

2. Parsear como JSON (via `serde_json`).

3. Validar `version == "2.1.0"`. Se diferente:
   `Err(UnsupportedVersion)`.

4. Para cada `run` em `runs`:
   - Para cada `result` em `results`:
     - Extrair `ruleId` (se ausente, usar string vazia ou pular).
     - Extrair `level` (default "warning" se ausente).
     - Extrair `message.text`.
     - Extrair primeira `location`:
       `physicalLocation.artifactLocation.uri` e
       `physicalLocation.region.startLine` (opcional).
     - Construir `SarifFinding`.

5. Retornar `Vec<SarifFinding>` na ordem em que aparecem.

### Estruturas internas de parsing

Usar structs DTO com `serde` derive para o parsing,
representando apenas o subset necessário do SARIF:

```rust
#[derive(Deserialize)]
struct SarifDoc {
    version: String,
    runs: Vec<SarifRun>,
}

#[derive(Deserialize)]
struct SarifRun {
    #[serde(default)]
    results: Vec<SarifResult>,
}

#[derive(Deserialize)]
struct SarifResult {
    #[serde(rename = "ruleId", default)]
    rule_id: String,
    #[serde(default = "default_level")]
    level: String,
    message: SarifMessage,
    #[serde(default)]
    locations: Vec<SarifLocation>,
}

// ... e assim por diante para message, location,
// physicalLocation, artifactLocation, region.
```

Usar `#[serde(default)]` liberalmente para tolerância a campos
ausentes (SARIF tem muitos campos opcionais).

### Tipo de erro

```rust
#[derive(Debug, thiserror::Error)]
pub enum SarifReadError {
    #[error("Ficheiro SARIF não encontrado: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Falha de I/O ao ler SARIF: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    #[error("Falha ao parsear SARIF JSON: {source}")]
    ParseFailed {
        #[from]
        source: serde_json::Error,
    },

    #[error("Versão SARIF não suportada: {version} (esperado 2.1.0)")]
    UnsupportedVersion { version: String },
}
```

---

## Dependências externas

`03_infra/Cargo.toml`:
- `serde` e `serde_json`: já presentes.
- `toml = "0.8"`: **nova**, para parsing do `crystalline.toml`.

---

## Testes esperados

### `crystalline_config_reader` (testes inline)

1. **TOML válido com [layers]**: ficheiro de teste com
   `[layers]` mapeando L1-L4. Workspace com crates nos
   diretórios correspondentes. Verificar mapa correto.

2. **Ficheiro inexistente**: `Err(FileNotFound)`.

3. **TOML malformado**: `Err(ParseFailed)`.

4. **Sem seção [layers]**: TOML válido mas sem `[layers]`.
   `Err(NoLayersSection)`.

5. **Crate fora da topologia**: workspace com um crate cujo
   diretório não está em `[layers]`. Esse crate não aparece no
   `LayerConfig`, sem erro.

6. **Normalização de nome**: crate `crystalline-dsm-core`
   (hífen) é mapeado como `crystalline_dsm_core` (underscore) no
   `LayerConfig`.

7. **Chave de camada desconhecida ignorada**: `[layers]` com
   uma entrada `L5 = "05_extra"`. A entrada é ignorada sem
   erro.

8. **Seções extra ignoradas**: TOML com `[project]`,
   `[languages]`, `[rules]` além de `[layers]`. Apenas
   `[layers]` é usado; resto ignorado sem erro.

### `sarif_reader` (testes inline)

9. **SARIF válido com 1 finding**: parseia, retorna 1
   `SarifFinding` com campos corretos.

10. **SARIF com múltiplos findings**: vários `results`,
    retorna todos na ordem.

11. **SARIF com múltiplos runs**: findings de todos os runs
    são agregados.

12. **Ficheiro inexistente**: `Err(FileNotFound)`.

13. **JSON malformado**: `Err(ParseFailed)`.

14. **Versão não suportada**: `version: "3.0.0"` →
    `Err(UnsupportedVersion)`.

15. **Finding sem region**: `startLine` fica `None`.

16. **Finding sem ruleId**: `rule_id` fica string vazia (ou o
    default escolhido).

17. **Results vazio**: SARIF válido com `results: []` retorna
    `Vec` vazio.

---

## Critério de aceitação do prompt

- `03_infra/src/crystalline_config_reader.rs` existe e compila.
- `03_infra/src/sarif_reader.rs` existe e compila.
- `read_layer_config` e `read_sarif` com as assinaturas
  especificadas.
- `SarifFinding` conforme especificado.
- Os 17 testes passam.
- `cargo clippy --all-targets` sem warnings.
- `toml` adicionado ao `Cargo.toml` de `03_infra`.
- Módulos exportados em `03_infra/src/lib.rs`.
- L₁ permanece inalterado (apenas consome `LayerConfig`, que L₃
  constrói).

---

## Próximos passos (fora deste prompt)

1. **Renderizador HTML estendido**: consome
   `Vec<LayerViolation>` (do detector L₁) e `Vec<SarifFinding>`
   (deste leitor) para destacar células/nós. Prompt separado.

2. **L₄**: orquestrar. Ler config + SARIF (se fornecido via
   flags `--config` e `--sarif`), chamar detector, passar tudo
   ao renderizador.

---

## Limitações conhecidas

1. **`[layers]` com subdiretórios**: o casamento por último
   componente do path assume diretórios de topo simples. Paths
   como `"crates/core"` não casam. Documentado.

2. **SARIF: apenas primeira location de cada result**: se um
   result tem múltiplas locations, só a primeira é considerada.
   Suficiente para o caso do `crystalline-lint`.

3. **Cruzamento SARIF → nó do grafo é feito a jusante**: este
   leitor só extrai os findings com `file_uri`. O mapeamento
   `file_uri → GraphNodeId` (via `source_file`/`canonical_path`)
   é responsabilidade do renderizador ou de L₄, não deste leitor.

---

## Hash do prompt

A calcular após aprovação.
