# Laudo de Execução — Prompt 0001 (Tipo de Dados da Forma Organizada)

**Camada**: L5 (laudo)
**Data**: 2026-05-27
**Prompt executado**: `00_nucleo/prompt/0001-dados_grafo.md`
**Spec de origem**: `00_nucleo/specs/forma-organizada.md`
**ADRs aplicáveis**: 0001 (fonte do grafo), 0002 (modelagem do grafo)
**Estado**: `EXECUTADO` (compila, testes verdes, pureza L1 verificada)

---

## O que o prompt pediu

Gerar o **primeiro componente de código** do projeto-lente: os tipos Rust que
materializam a forma organizada. Apenas estrutura — sem cálculo de raio, sem
desserialização, sem I/O. Pureza L1: só stdlib.

Componentes a gerar:

- Enums `Relation` (2 var.), `Visibility` (5 var., incluindo `PubIn(caminho)`),
  `Kind` (17 var. da lista fechada da spec).
- Structs `No`, `Aresta`, `Grafo`.
- Conversão texto→enum (ex.: `impl TryFrom<&str>`), retornando erro para texto
  fora da lista fechada — sem panic, sem default.
- Testes cobrindo critérios (cada enum, casos de borda, grafo mínimo).

---

## O que foi gerado

| Arquivo | Propósito |
|---------|-----------|
| `01_core/Cargo.toml` | Package `lente_core`, edition 2024, **zero dependências**. |
| `01_core/src/lib.rs` | Crate root. `#![forbid(unsafe_code)]`, `pub mod entities`. |
| `01_core/src/entities/mod.rs` | Reexporta `pub mod grafo`. |
| `01_core/src/entities/grafo.rs` | Enums, structs, conversões, testes inline. |

**Verificação**:
- `cargo build`: compila limpo (0 warnings).
- `cargo test`: 11/11 testes passam.
- `cargo tree`: `lente_core v0.0.0` sozinho — sem dependência externa
  (pureza L1 confirmada).

---

## Decisões tácitas (registro para futura revisão)

O prompt deixou pontos abertos. As escolhas e por quê:

### D1 — Crate isolado em `01_core/`, package `lente_core`

O prompt menciona `01_core/entities/grafo.rs` mas o repositório não tinha
nenhum `Cargo.toml`. Para o critério "compila apenas com stdlib" ser
verificável, foi criado um crate Rust isolado dentro de `01_core/`.

- Nome do package: `lente_core` (snake_case obrigatório). Não usar `core` para
  não confundir com `core` da stdlib.
- Edition 2024 (rustc 1.92 instalado já a suporta).
- `rust-version = "1.91"` alinhado ao do fork (ADR-0001).

**Pendência arquitetural**: se cada estrato (L1, L2, L3, L4) virar um crate
separado, faz sentido um workspace Cargo na raiz que os agrupe. Não foi feito
ainda — primeiro crate só, sem antecipar estrutura de múltiplos.

### D2 — `Path` como newtype `Path(String)`

O prompt aceita `String` ou newtype. Escolhido **newtype** para que assinaturas
do cálculo futuro (`fn dependentes_de(p: &Path) -> ...`) não aceitem qualquer
string. Tem `From<&str>`, `From<String>`, `as_str()`, `Display`. Custo:
verbosidade pequena na construção (`Path::from("...")`).

### D3 — `Grafo` com `Vec<No>` + `Vec<Aresta>`

O prompt prefere a forma fiel ao JSON salvo justificativa. Adotada. Sem
indexação aqui — ADR-0002, Decisão 2: indexação pertence ao cálculo, não à
entrada.

### D4 — Conversão texto→enum via `impl TryFrom<&str>`

Idiomático em Rust moderno, integra com `?` naturalmente. Tipo de erro
`ValorDesconhecido { tipo: &'static str, texto: String }` único para os três
enums — implementa `Display` e `core::error::Error`. Texto desconhecido
**retorna erro**, nunca panic nem default, atendendo o critério do prompt e a
consequência da Decisão 1 do ADR-0002 (validação na borda L3).

### D5 — Campo `crate` do JSON → campo Rust `crate_name`

`crate` é palavra reservada em Rust e não pode ser nome de campo sem
`r#crate`. Renomeado para `crate_name`. Mapeamento JSON↔struct é
responsabilidade do L3 (adaptador) e não causa atrito porque o L3 já fará
desserialização explícita.

### D6 — Visibilidade `pub(in <caminho>)`

Variante `Visibility::PubIn(String)` carrega o caminho sem o envelope
`pub(in ` ... `)`. Parser usa `strip_prefix("pub(in ")` + `strip_suffix(")")` +
`trim`. Caminho vazio (`"pub(in )"`) é erro — não há `pub(in )` válido em Rust.

### D7 — Layout de testes inline (não `grafo.test.rs`)

O prompt aceitou "layout equivalente". Convenção idiomática Rust é
`#[cfg(test)] mod tests` no fim do mesmo arquivo. Adotada — testes ficam
junto ao código, compilam só em modo teste, não exigem nome de arquivo
não-padrão.

### D8 — Laudo em `00_nucleo/lessons/0001-dados-grafo.md`

O prompt cita "L5" sem caminho fixo. Adotado `00_nucleo/lessons/` alinhado às
pastas existentes em `00_nucleo/` (`adr/`, `prompt/`). Mover é fácil se a
estrutura Tekt definir outro caminho canônico.

---

## Discrepâncias de referência observadas (cosméticas)

- **Resolvida durante a execução**: a spec passou a viver em
  `00_nucleo/specs/forma-organizada.md` (caminho que o prompt e o ADR-0002 já
  citavam). Cabeçalhos de linhagem dos arquivos gerados (`lib.rs`,
  `entities/grafo.rs`) e a referência neste laudo apontam para o caminho novo.
- **Pendente**: o CLAUDE.md referencia `00_nucleo/MANIFESTO.md`; o arquivo
  presente é `00_nucleo/MANIFESTO.pt.md`. Não bloqueia este componente.
  Decisão de canônico fica com o autor do CLAUDE.md / manifesto (renomear o
  arquivo ou atualizar a referência).

---

## O que o prompt explicitamente não pediu (não fiz)

- **Não implementei desserialização de JSON.** É L3, prompt futuro.
- **Não implementei índices** (`HashMap<Path, ...>` de vizinhos). É L1-cálculo,
  prompt futuro (ADR-0002, Decisão 2).
- **Não computei marca de stdlib.** ADR-0002, Decisão 3: filtro L1 separado.
- **Não criei o `MANIFESTO.md`, workspace na raiz, ou outras pastas L2/L3/L4.**
  Trabalho de outros prompts/decisões; não é deste.

---

## Critérios de Verificação atendidos

Conferência item-a-item dos critérios do prompt:

| Critério | Status | Teste |
|----------|--------|-------|
| Texto "uses"/"owns" → `Relation::Uses`/`Owns` | ✓ | `relation_owns_e_uses_traduzem` |
| Texto desconhecido em qualquer enum → erro (não panic, não default) | ✓ | `relation_desconhecida_retorna_erro`, `kind_desconhecido_retorna_erro`, `visibility_desconhecida_retorna_erro` |
| Todos os 17 valores de `kind` traduzem corretamente | ✓ | `kind_cobre_lista_fechada_inteira` |
| Visibilidades canônicas + `pub(in <path>)` preservando caminho | ✓ | `visibility_textos_canonicos_traduzem`, `visibility_pub_in_preserva_caminho` |
| Grafo construído preserva nós e arestas | ✓ | `grafo_construido_preserva_nos_e_arestas` |
| Grafo mínimo (só raiz, zero arestas) é válido | ✓ | `grafo_minimo_so_raiz_e_valido` |
| Sem `use` de crate externo, sem I/O, compila só com stdlib | ✓ | `cargo tree` mostra apenas `lente_core` |

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-05-27 | Execução inicial do prompt 0001. Componente compila, testes verdes, pureza L1 verificada. | `01_core/Cargo.toml`, `01_core/src/lib.rs`, `01_core/src/entities/mod.rs`, `01_core/src/entities/grafo.rs` |
