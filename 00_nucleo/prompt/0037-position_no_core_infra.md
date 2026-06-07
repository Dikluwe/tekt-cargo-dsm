# Prompt: Consumir `position` no `No` (`lente_core` + `lente_infra`)

**Camada**: L1 (adição puramente aditiva ao `lente_core`) + L3 (modificação do
`lente_infra` para consumir o campo novo do JSON)
**Criado em**: 2026-06-04
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0037-position_no_core_infra.md`)
**Decisões de origem**: 5ª rodada do fork (cada nó com fonte traz `position`);
briefing da trilha local (Projeto Lente); decisões do autor nesta conversa:
(1) escopo desta mudança = **só consumir `position` no `No`**, sem mapeamento
diff→nós, sem cálculo de raio, sem modo de CLI; (2) quando o modo de diff for
construído, o formato de entrada será um `git diff` completo; (3) a função do
diff morará num **modo da CLI**, com casca de agente (MCP) numa etapa posterior.
**Análogo a**: prompt 0006 (consumir `id`/`id_from`/`id_to` em core+infra) e
prompt 0012/0013 (campos do descritor em forma simples). O padrão é o mesmo:
campo aditivo no `lente_core`, desserialização no `lente_infra`.
**Pré-requisito**: o fork atualizado (5ª rodada, com `position`) instalado em
PATH **para a verificação contra dado real** (teste opcional `#[ignore]`). Os
testes unitários obrigatórios usam JSON inline e **não** dependem de fork ao vivo.
**Arquivos afetados**: `01_core/src/entities/grafo.rs` (novo tipo `Posicao`,
campo novo em `No`), testes do `lente_core`; `03_infra/src/dto.rs` (novo
`PositionDto`, campo no `NodeDto`), `03_infra/src/traducao.rs` (propagação),
testes do `lente_infra`; helpers de teste a jusante que constroem `No` (ver
§Não-regressão coordenada).

---

## Contexto

A 5ª rodada do fork adicionou `position` por nó. No `export-json`, cada nó com
fonte de arquivo traz:

```json
"position": { "file": "<caminho absoluto>", "start_line": <u32>, "end_line": <u32> }
```

- Linhas contadas a partir de 1 (1-based).
- Itens sem fonte (tipos embutidos) **não** trazem `position`.
- Itens gerados por macro trazem a posição do **call-site**.

Hoje o desserializador (em `03_infra/src/traducao.rs`) **tolera** o campo (serde
sem `deny_unknown_fields`), mas **não o lê** — o `No` da lente não tem posição.
Esta mudança faz o `No` **carregar** a posição que vem do JSON. É o pré-requisito
de qualquer mapeamento diff→nós (que é prompt futuro, não este).

Esta é a primeira mudança da **trilha local** (mostrar o que uma mudança toca
antes de um agente executar o comando). O cálculo do raio já existe; falta o `No`
saber sua posição no fonte para, depois, casar um diff aos nós. Este prompt
entrega só o `No` com `position` — **sem função visível ainda**.

Diferença importante em relação ao `id` (prompt 0006): o `id` é obrigatório e sua
ausência é erro (distingue JSON novo de antigo). A `position` é **opcional por
natureza** — alguns nós legitimamente não a têm (tipos embutidos). Logo, ausência
de `position` num nó **não** é erro; vira `None`. O diagnóstico "atualize o fork"
para quando `position` está ausente em tudo é responsabilidade do modo de CLI
futuro, não deste prompt — aqui não há consumidor de `position` que justifique
diagnóstico.

---

## Restrições estruturais

- **`lente_core` permanece puro.** O tipo `Posicao` usa só stdlib (`String`,
  `u32`). Sem `serde`, sem dependência externa. `cargo tree -p lente_core`
  continua mostrando só o crate.
- **`position` é opcional.** O campo em `No` é `Option<Posicao>`. Ausência é
  legítima (tipos embutidos) e vira `None` — **não** falha. Isto contrasta com o
  `id` do prompt 0006, e é deliberado.
- **Caminho armazenado como vem.** A `position.file` é **absoluta** (como o fork
  a emite). Este prompt armazena a string **verbatim**, sem transformar.
  Relativizar para casar com um diff é trabalho do mapeamento diff→nós (prompt
  futuro), não deste.
- **`lente_infra` ganha campo no DTO, não muda assinatura.** A função pública
  mantém a assinatura; muda só o conteúdo do `Grafo` retornado (que agora tem
  `position` preenchida quando o JSON a traz).
- **Serde tolerante mantido.** Sem `deny_unknown_fields`. Um JSON sem `position`
  (fork antigo) desserializa sem erro, com todos os `No.position == None`.
- **Gravidade Tekt.** `lente_infra` depende de `lente_core`; nunca o contrário.

---

## O que mudar no `lente_core` (`01_core/src/entities/grafo.rs`)

### Tipo novo `Posicao`

```rust
/// Posição de um nó no código-fonte, como o fork a emite.
/// Ausente para itens sem fonte (tipos embutidos). Para itens gerados
/// por macro, é a posição do call-site. Linhas contadas a partir de 1.
#[derive(Debug, Clone, PartialEq, Eq)]   // derive ao menos o que `No` deriva
pub struct Posicao {
    /// Caminho do arquivo. Absoluto, como o fork o produz. Não transformado aqui.
    pub file: String,
    /// Primeira linha do item (1-based).
    pub start_line: u32,
    /// Última linha do item (1-based).
    pub end_line: u32,
}
```

Nome sugerido `Posicao`, em português como `No`/`Aresta`/`Raio`. Se o autor
preferir `Position` (espelhando o nome do campo JSON, como `UsesKind` espelha
`uses_kind`), o gerador renomeia e registra no laudo. É decisão do autor.

Sobre os derives: `Posicao` precisa derivar ao menos o que `No` deriva
(provavelmente `Debug, Clone, PartialEq`; se `No` deriva `Eq`/`Hash`, `Posicao`
também pode — `String` e `u32` suportam todos). O gerador confere e ajusta.

### Campo novo em `No`

Adicionar:

```rust
/// Posição no fonte. `None` quando o item não tem fonte (tipo embutido)
/// ou quando o JSON não traz `position` (fork antigo).
pub position: Option<Posicao>,
```

Adição puramente aditiva — nenhum campo existente muda.

### Testes do `lente_core`

- Construir um `No` com `position: Some(Posicao { file, start_line, end_line })`
  e conferir que o campo é acessível e carrega os valores.
- Construir um `No` com `position: None` e conferir que é válido.
- Se útil, conferir que `Posicao` carrega os três campos.

---

## O que mudar no `lente_infra`

### DTO (`03_infra/src/dto.rs`)

Adicionar o DTO da posição e o campo no `NodeDto`:

```rust
#[derive(Deserialize)]
struct PositionDto {
    file: String,
    start_line: u32,
    end_line: u32,
}

// No NodeDto, campo novo:
position: Option<PositionDto>,   // campo ausente no JSON → None
```

- `Option<PositionDto>`: o serde trata campo ausente como `None` em tipos
  `Option`. Um nó sem `position` (tipo embutido) e um JSON antigo (sem `position`
  em lugar nenhum) **desserializam sem erro**, virando `None`. Se ao rodar
  aparecer erro de "campo faltando `position`", adicionar `#[serde(default)]` ao
  campo, por garantia. O gerador confirma o comportamento observado no laudo.
- **Não** adicionar `deny_unknown_fields` (mantém a tolerância do invariante).

### Tradução (`03_infra/src/traducao.rs`)

Propagar o DTO para o `No`:

```rust
position: dto.position.map(|p| Posicao {
    file: p.file,
    start_line: p.start_line,
    end_line: p.end_line,
}),
```

Mecânico — passa o campo, sem transformar (caminho absoluto fica como veio).

### Testes do `lente_infra`

Preferir JSON inline (pequeno, explícito, independente do fork ao vivo):

- **Nó com `position`** → `No.position == Some(Posicao { file, start_line,
  end_line })`, com as linhas 1-based preservadas e o `file` verbatim (absoluto).
- **Nó sem `position`** (campo ausente no JSON) → `No.position == None`, sem erro.
- **JSON sem `position` em nenhum nó** (simula fork antigo) → desserializa sem
  erro, todos os `No.position == None`. Prova a tolerância aditiva.
- **Não-regressão**: os testes existentes do `lente_infra` continuam passando.

Se reaproveitar uma fixture existente em vez de JSON inline, garanta que ela traz
`position` em ao menos um nó.

### Teste contra dado real (opcional, recomendado — requer fork atualizado em PATH)

Um teste `#[ignore]` que roda o fork num crate pequeno (por exemplo o próprio
`lente_core`), desserializa, e confirma que **ao menos um `No`** traz `position`
com `file` não-vazio e `start_line <= end_line`. Fecha o ciclo "medir antes de
afirmar" contra a saída real do fork. Se o fork em PATH for antigo, este teste
falha (ou é ignorado) — sinal direto de que o fork precisa ser atualizado (o
laudo 0033 tropeçou exatamente em fork velho em `~/.cargo/bin`).

---

## Não-regressão coordenada

Adicionar um campo público ao `No` quebra a compilação de todo construtor de
`No`. Preencher o campo novo com `None` (mecânico) nos helpers de teste a jusante
(lição do laudo 0012):

| Crate | Ponto a ajustar | Natureza |
|-------|-----------------|----------|
| `lente_core` (`raio.rs`) | helper de teste que constrói `No` | só teste |
| `lente_investiga` | helper de teste que constrói `No` | só teste |
| `lente_resolve` | helper de teste que constrói `No` | só teste |
| `lente_infra` (`traducao.rs`) | construção real de `No` | **lê do DTO** (não default) |
| `lab/` (Arena, se constrói `No`) | construtor do experimento | experimento |

A construção em `traducao.rs` é a única que **lê** `position` do dado; todas as
outras recebem `None`.

A contagem atual da suíte (cerca de 206 verdes + 21 ignored, conforme o briefing)
deve permanecer verde ou crescer com os testes novos. Reportar o número exato no
laudo. Nenhum teste removido ou desabilitado para a mudança passar.

---

## Critérios de Verificação

```
Dado o tipo No
Quando construído com position = Some(Posicao { file, start_line, end_line })
Então o campo é acessível e carrega os três valores

Dado o tipo No
Quando construído com position = None
Então é válido (ausência é legítima)

Dado um JSON do fork com um nó que traz "position": { file, start_line, end_line }
Quando desserializado pelo lente_infra
Então o No correspondente tem position = Some(Posicao { ... }), com as linhas
1-based preservadas e o file verbatim (absoluto)

Dado um JSON com um nó sem o campo "position"
Quando desserializado
Então o No correspondente tem position = None, sem erro

Dado um JSON sem "position" em nenhum nó (fork antigo)
Quando desserializado
Então todos os No têm position = None, sem erro (tolerância aditiva)

Dado cargo tree -p lente_core
Então mostra só o crate (pureza L1 preservada)
```

---

## Resultado esperado

- `Posicao` novo em `lente_core` (stdlib só).
- `No.position: Option<Posicao>`, aditivo.
- `lente_infra` desserializa `position` (Option) e propaga no `traducao.rs`,
  verbatim.
- Ausência de `position` vira `None`, sem erro (contraste deliberado com o `id`).
- Testes novos verdes (core + infra) + não-regressão coordenada nos helpers a
  jusante.
- Suíte inteira verde (≈206 + os novos), contagem reportada, nada
  removido/desabilitado.
- **Laudo de execução** em `00_nucleo/lessons/`: o que mudou, o nome adotado para
  o tipo (`Posicao`/`Position`), o comportamento confirmado do serde para `Option`
  ausente, a contagem reconciliada, e — se rodado — o resultado do teste contra
  dado real.

---

## O que NÃO entra neste prompt (trilha local, etapas seguintes)

- **Mapeamento diff→nós.** Dado um `git diff` (formato escolhido pelo autor),
  achar os nós cujo span de fonte intersecta as linhas alteradas. Prompt futuro.
- **Regra de intersecção linha↔span.** Definir quando um nó é "tocado" (a faixa
  alterada intersecta `[start_line, end_line]`?) e o que fazer com spans aninhados
  (um módulo contém os itens dentro dele). Prompt futuro, junto do mapeamento.
- **Relativizar o caminho.** `position.file` é absoluto; um `git diff` traz
  caminhos relativos à raiz do crate. Casá-los é trabalho do mapeamento, não deste
  prompt.
- **Cálculo do raio sobre os nós tocados.** O `calcular_raio` já existe; ligá-lo
  ao conjunto de nós tocados é parte do modo de CLI. Prompt futuro.
- **Modo da CLI (`lente --diff …`).** A interface que recebe o `git diff` e mostra
  o impacto. Prompt futuro.
- **Diagnóstico "atualize o fork".** Quando `position` está ausente em tudo e o
  modo local é pedido. Vive no modo de CLI, que ainda não existe; sem consumidor,
  não há o que diagnosticar aqui.
- **Casca de agente (MCP).** Etapa posterior (a Ponte 2 da trilha local).

---

## Cuidados

- **Caminho absoluto armazenado como vem.** Não relativizar, não normalizar aqui.
  Relativizar é do mapeamento.
- **Determinismo.** `position` é determinística. A instabilidade residual entre
  extrações é só o `id` do petgraph (pré-existente, documentada no schema do fork)
  — não atribuir ao trabalho novo.
- **Macro call-site.** Itens gerados por macro trazem a posição do call-site. Para
  este prompt (só armazenar), não há distinção a fazer: o objeto `position` do
  JSON é só `file`/`start_line`/`end_line`. A lente armazena o que vem.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-04 | Consumir `position` no `No`: tipo `Posicao` novo no lente_core (stdlib só); `No.position: Option<Posicao>` aditivo; lente_infra desserializa `position` (Option, ausência → None) e propaga verbatim. Primeira mudança da trilha local; pré-requisito do mapeamento diff→nós. | 01_core/src/entities/grafo.rs, 03_infra/src/dto.rs, 03_infra/src/traducao.rs |
